use crate::collectors::drivers::{DeviceCategory, DeviceInfo, DeviceStatus, DriverData, ServiceInfo};
use std::collections::HashMap;
use wmi::{COMLibrary, WMIConnection};
use serde::Deserialize;

#[derive(Deserialize, Debug)]
#[serde(rename = "Win32_PnPSignedDriver")]
#[serde(rename_all = "PascalCase")]
struct PnpDriver {
    device_name: Option<String>,
    driver_version: Option<String>,
    driver_date: Option<String>,
    device_class: Option<String>,
    status: Option<String>,
    #[allow(dead_code)]
    is_signed: Option<bool>,
}

#[derive(Deserialize, Debug)]
#[serde(rename = "Win32_Service")]
#[serde(rename_all = "PascalCase")]
struct Win32Service {
    name: Option<String>,
    display_name: Option<String>,
    state: Option<String>,
}

pub fn collect() -> DriverData {
    let mut data = DriverData::default();

    // Try WMI connection
    let com = match COMLibrary::new() {
        Ok(c) => c,
        Err(_) => return data,
    };

    let wmi = match WMIConnection::new(com) {
        Ok(w) => w,
        Err(_) => return data,
    };

    // Query signed drivers
    if let Ok(drivers) = wmi.raw_query::<PnpDriver>("SELECT DeviceName, DriverVersion, DriverDate, DeviceClass, Status FROM Win32_PnPSignedDriver") {
        for drv in drivers {
            let name = drv.device_name.unwrap_or_default();
            if name.is_empty() {
                continue;
            }

            let device_class = drv.device_class.as_deref().unwrap_or("");
            let category = match device_class {
                "Net" | "NetClient" | "NetService" | "NetTrans" => Some(DeviceCategory::Network),
                "Bluetooth" => Some(DeviceCategory::Bluetooth),
                "AudioEndpoint" | "Media" | "MEDIA" => Some(DeviceCategory::Audio),
                "Keyboard" | "Mouse" | "HIDClass" => Some(DeviceCategory::Input),
                _ => None,
            };

            if let Some(cat) = category {
                let status_str = drv.status.as_deref().unwrap_or("Unknown");
                let status = match status_str {
                    "OK" => DeviceStatus::Ok,
                    "Degraded" | "Error" => DeviceStatus::Error(status_str.to_string()),
                    _ => DeviceStatus::Unknown,
                };

                let info = DeviceInfo {
                    name,
                    driver_version: drv.driver_version.unwrap_or_default(),
                    driver_date: format_wmi_date(drv.driver_date.as_deref().unwrap_or("")),
                    status,
                    category: cat.clone(),
                    extra: String::new(),
                };

                match cat {
                    DeviceCategory::Network => data.network.push(info),
                    DeviceCategory::Bluetooth => data.bluetooth.push(info),
                    DeviceCategory::Audio => data.audio.push(info),
                    DeviceCategory::Input => data.input.push(info),
                }
            }
        }
    }

    // Query critical services
    let service_names = [
        "Dhcp", "Dnscache", "WlanSvc", "NlaSvc",     // Network
        "bthserv", "BthAvctpSvc",                      // Bluetooth
        "Audiosrv", "AudioEndpointBuilder",            // Audio
        "hidserv",                                      // Input
    ];

    if let Ok(services) = wmi.raw_query::<Win32Service>(
        "SELECT Name, DisplayName, State FROM Win32_Service"
    ) {
        let svc_map: HashMap<String, Win32Service> = services
            .into_iter()
            .filter_map(|s| {
                let name = s.name.clone()?;
                Some((name, s))
            })
            .collect();

        for svc_name in &service_names {
            if let Some(svc) = svc_map.get(*svc_name) {
                data.services.push(ServiceInfo {
                    name: svc_name.to_string(),
                    display_name: svc.display_name.clone().unwrap_or_default(),
                    is_running: svc.state.as_deref() == Some("Running"),
                });
            }
        }
    }

    data
}

/// Format WMI date string (yyyyMMddHHmmss.ffffff+zzz) to yyyy-MM-dd
fn format_wmi_date(date: &str) -> String {
    if let (Some(year), Some(month), Some(day)) = (date.get(0..4), date.get(4..6), date.get(6..8)) {
        format!("{}-{}-{}", year, month, day)
    } else {
        date.to_string()
    }
}
