use serde::Serialize;

use crate::observation::Observation;

#[derive(Debug, Clone, Default, Serialize)]
pub struct DisplayData {
    pub displays: Vec<DisplayInfo>,
    pub inventory_status: Observation,
    pub brightness_status: Observation,
}

#[derive(Debug, Clone, Serialize)]
pub struct DisplayInfo {
    pub label: String,
    pub active: Option<bool>,
    pub connection: String,
    pub brightness_percent: Option<u8>,
    pub physical_width_cm: Option<u16>,
    pub physical_height_cm: Option<u16>,
    pub source: String,
}

pub fn collect() -> DisplayData {
    #[cfg(windows)]
    {
        collect_windows()
    }

    #[cfg(not(windows))]
    {
        DisplayData {
            displays: Vec::new(),
            inventory_status: Observation::unsupported(
                "platform display provider",
                "Display inventory is not implemented on this platform",
            ),
            brightness_status: Observation::unsupported(
                "platform display provider",
                "Display brightness is not implemented on this platform",
            ),
        }
    }
}

#[cfg(windows)]
#[derive(serde::Deserialize, Debug)]
#[serde(rename_all = "PascalCase")]
struct WmiMonitorConnectionParams {
    instance_name: Option<String>,
    video_output_technology: Option<u32>,
}

#[cfg(windows)]
#[derive(serde::Deserialize, Debug)]
#[serde(rename_all = "PascalCase")]
struct WmiMonitorBasicDisplayParams {
    instance_name: Option<String>,
    active: Option<bool>,
    max_horizontal_image_size: Option<u16>,
    max_vertical_image_size: Option<u16>,
}

#[cfg(windows)]
#[derive(serde::Deserialize, Debug)]
#[serde(rename_all = "PascalCase")]
struct WmiMonitorBrightness {
    instance_name: Option<String>,
    active: Option<bool>,
    current_brightness: Option<u8>,
}

#[cfg(windows)]
fn collect_windows() -> DisplayData {
    use std::collections::HashMap;
    use wmi::{COMLibrary, WMIConnection};

    let com = match COMLibrary::new() {
        Ok(com) => com,
        Err(error) => {
            let observation = Observation::error(
                "root\\WMI monitor classes",
                format!("COM initialization failed: {error}"),
            );
            return DisplayData {
                displays: Vec::new(),
                inventory_status: observation.clone(),
                brightness_status: observation,
            };
        }
    };
    let connection = match WMIConnection::with_namespace_path("root\\WMI", com) {
        Ok(connection) => connection,
        Err(error) => {
            let observation = Observation::error(
                "root\\WMI monitor classes",
                format!("WMI connection failed: {error}"),
            );
            return DisplayData {
                displays: Vec::new(),
                inventory_status: observation.clone(),
                brightness_status: observation,
            };
        }
    };

    let basics = connection
        .raw_query::<WmiMonitorBasicDisplayParams>(
            "SELECT InstanceName, Active, MaxHorizontalImageSize, MaxVerticalImageSize FROM WmiMonitorBasicDisplayParams",
        )
        .unwrap_or_default()
        .into_iter()
        .filter_map(|row| {
            let key = row.instance_name.as_deref().map(monitor_key)?;
            Some((key, row))
        })
        .collect::<HashMap<_, _>>();
    let (brightness_rows, brightness_error) = match connection.raw_query::<WmiMonitorBrightness>(
        "SELECT InstanceName, Active, CurrentBrightness FROM WmiMonitorBrightness",
    ) {
        Ok(rows) => (rows, None),
        Err(error) => (Vec::new(), Some(format!("WMI query failed: {error}"))),
    };
    let brightness = brightness_rows
        .iter()
        .filter_map(|row| {
            row.instance_name
                .as_deref()
                .map(|name| (monitor_key(name), row.current_brightness))
        })
        .collect::<HashMap<_, _>>();
    let connections = match connection.raw_query::<WmiMonitorConnectionParams>(
        "SELECT InstanceName, VideoOutputTechnology FROM WmiMonitorConnectionParams",
    ) {
        Ok(rows) => rows,
        Err(error) => {
            return DisplayData {
                displays: Vec::new(),
                inventory_status: Observation::error(
                    "WmiMonitorConnectionParams",
                    format!("WMI query failed: {error}"),
                ),
                brightness_status: brightness_observation(
                    &brightness_rows,
                    brightness_error.as_deref(),
                ),
            };
        }
    };

    let displays = connections
        .into_iter()
        .enumerate()
        .filter_map(|(index, row)| {
            let name = row.instance_name?;
            let key = monitor_key(&name);
            let basic = basics.get(&key);
            Some(DisplayInfo {
                label: format!("Display {}", index + 1),
                active: basic.and_then(|row| row.active),
                connection: connection_name(row.video_output_technology),
                brightness_percent: brightness.get(&key).copied().flatten(),
                physical_width_cm: basic.and_then(|row| row.max_horizontal_image_size),
                physical_height_cm: basic.and_then(|row| row.max_vertical_image_size),
                source: "WmiMonitorConnectionParams + WmiMonitorBasicDisplayParams".into(),
            })
        })
        .collect::<Vec<_>>();

    let inventory_status = if displays.is_empty() {
        Observation::unavailable(
            "WmiMonitorConnectionParams",
            "The provider returned no connected monitor rows",
        )
    } else {
        Observation::available("WmiMonitorConnectionParams")
    };
    DisplayData {
        displays,
        inventory_status,
        brightness_status: brightness_observation(&brightness_rows, brightness_error.as_deref()),
    }
}

#[cfg(windows)]
fn brightness_observation(rows: &[WmiMonitorBrightness], query_error: Option<&str>) -> Observation {
    if let Some(error) = query_error {
        Observation::error("WmiMonitorBrightness", error)
    } else if rows
        .iter()
        .any(|row| row.active != Some(false) && row.current_brightness.is_some())
    {
        Observation::available("WmiMonitorBrightness")
    } else {
        Observation::unavailable(
            "WmiMonitorBrightness",
            "No active display exposed software brightness",
        )
    }
}

#[cfg(windows)]
fn monitor_key(instance_name: &str) -> String {
    let trimmed = instance_name.trim();
    let base = trimmed
        .rsplit_once('_')
        .filter(|(_, suffix)| suffix.chars().all(|character| character.is_ascii_digit()))
        .map(|(base, _)| base)
        .unwrap_or(trimmed);
    base.to_ascii_uppercase()
}

#[cfg(windows)]
fn connection_name(value: Option<u32>) -> String {
    match value {
        Some(1) => "VGA",
        Some(5) => "DVI",
        Some(6) => "HDMI",
        Some(8) => "LVDS",
        Some(10) => "DisplayPort",
        Some(11) => "Embedded DisplayPort",
        Some(15) => "Miracast",
        Some(16) => "Internal",
        Some(value) => return format!("Technology {value}"),
        None => "Unknown",
    }
    .into()
}

#[cfg(all(test, windows))]
mod tests {
    use super::*;

    #[test]
    fn maps_windows_display_connection_codes() {
        assert_eq!(connection_name(Some(6)), "HDMI");
        assert_eq!(connection_name(Some(10)), "DisplayPort");
        assert_eq!(connection_name(Some(11)), "Embedded DisplayPort");
    }

    #[test]
    fn monitor_key_removes_provider_instance_suffix() {
        assert_eq!(monitor_key("DISPLAY\\ABC\\1_0"), "DISPLAY\\ABC\\1");
    }

    #[test]
    fn brightness_query_errors_remain_errors() {
        let observation = brightness_observation(&[], Some("WMI query failed: provider error"));

        assert_eq!(
            observation.status,
            crate::observation::ObservationStatus::Error
        );
        assert_eq!(
            observation.detail.as_deref(),
            Some("WMI query failed: provider error")
        );
    }
}
