pub mod platform;

use serde::Serialize;

/// Driver/device health data
#[derive(Debug, Clone, Default, Serialize)]
pub struct DriverData {
    pub network: Vec<DeviceInfo>,
    pub bluetooth: Vec<DeviceInfo>,
    pub audio: Vec<DeviceInfo>,
    pub input: Vec<DeviceInfo>,
    pub display: Vec<DeviceInfo>,
    pub storage: Vec<DeviceInfo>,
    pub usb: Vec<DeviceInfo>,
    pub system: Vec<DeviceInfo>,
    pub other: Vec<DeviceInfo>,
    pub services: Vec<ServiceInfo>,
    pub scan_status: DriverScanStatus,
}

#[derive(Debug, Clone, Serialize)]
pub struct DeviceInfo {
    pub name: String,
    pub driver_version: String,
    pub driver_date: String,
    pub status: DeviceStatus,
    pub category: DeviceCategory,
    pub extra: String,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DeviceStatus {
    Ok,
    Degraded(String),
    Disabled,
    Error(String),
    NotFound,
    Unknown,
}

impl std::fmt::Display for DeviceStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Ok => write!(f, "OK"),
            Self::Degraded(msg) => write!(f, "Degraded: {}", msg),
            Self::Disabled => write!(f, "Disabled"),
            Self::Error(msg) => write!(f, "Error: {}", msg),
            Self::NotFound => write!(f, "Not Found"),
            Self::Unknown => write!(f, "Unknown"),
        }
    }
}

impl DeviceStatus {
    pub fn requires_attention(&self) -> bool {
        matches!(
            self,
            Self::Degraded(_) | Self::Disabled | Self::Error(_) | Self::NotFound
        )
    }

    pub fn icon(&self) -> &'static str {
        match self {
            Self::Ok => "\u{2713}", // checkmark
            Self::Degraded(_) => "\u{26A0}",
            Self::Disabled => "\u{2014}", // em dash
            Self::Error(_) => "\u{2717}", // x mark
            Self::NotFound => "?",
            Self::Unknown => "?",
        }
    }

    pub fn user_description(&self) -> &'static str {
        match self {
            Self::Ok => "Working",
            Self::Degraded(_) => "Working with reported problems",
            Self::Disabled => "Turned off",
            Self::Error(_) => "Not working properly",
            Self::NotFound => "Not detected",
            Self::Unknown => "Unknown status",
        }
    }
}

impl DriverData {
    pub fn devices(&self) -> impl Iterator<Item = &DeviceInfo> {
        self.network
            .iter()
            .chain(self.bluetooth.iter())
            .chain(self.audio.iter())
            .chain(self.input.iter())
            .chain(self.display.iter())
            .chain(self.storage.iter())
            .chain(self.usb.iter())
            .chain(self.system.iter())
            .chain(self.other.iter())
    }

    pub fn attention_devices(&self) -> impl Iterator<Item = &DeviceInfo> {
        self.devices()
            .filter(|device| device.status.requires_attention())
    }
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DeviceCategory {
    Network,
    Bluetooth,
    Audio,
    Input,
    Display,
    Storage,
    Usb,
    System,
    Other,
}

impl DeviceCategory {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Network => "Network",
            Self::Bluetooth => "Bluetooth",
            Self::Audio => "Audio",
            Self::Input => "Input",
            Self::Display => "Display",
            Self::Storage => "Storage",
            Self::Usb => "USB",
            Self::System => "System",
            Self::Other => "Other",
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DriverScanStatus {
    #[default]
    NotScanned,
    Scanning,
    Success,
    ScanFailed(String),
}

#[derive(Debug, Clone, Serialize)]
pub struct ServiceInfo {
    pub name: String,
    pub display_name: String,
    pub is_running: bool,
}

pub fn collect() -> DriverData {
    platform::collect_drivers()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn device(name: &str, status: DeviceStatus, category: DeviceCategory) -> DeviceInfo {
        DeviceInfo {
            name: name.into(),
            driver_version: String::new(),
            driver_date: String::new(),
            status,
            category,
            extra: String::new(),
        }
    }

    #[test]
    fn attention_iterator_includes_hidden_categories_and_ignores_unknown() {
        let mut data = DriverData::default();
        data.network
            .push(device("healthy", DeviceStatus::Ok, DeviceCategory::Network));
        data.system.push(device(
            "system issue",
            DeviceStatus::Disabled,
            DeviceCategory::System,
        ));
        data.other.push(device(
            "other issue",
            DeviceStatus::Degraded("fixture".into()),
            DeviceCategory::Other,
        ));
        data.other.push(device(
            "unknown capability",
            DeviceStatus::Unknown,
            DeviceCategory::Other,
        ));

        let names = data
            .attention_devices()
            .map(|device| device.name.as_str())
            .collect::<Vec<_>>();
        assert_eq!(names, vec!["system issue", "other issue"]);
    }
}
