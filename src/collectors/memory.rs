use serde::Serialize;
use sysinfo::System;

use crate::observation::Observation;

#[derive(Debug, Clone, Default, Serialize)]
pub struct MemoryData {
    pub used_bytes: u64,
    pub total_bytes: u64,
    pub available_bytes: u64,
    pub swap_used_bytes: u64,
    pub swap_total_bytes: u64,
    pub modules: Vec<MemoryModule>,
    pub module_status: Observation,
}

#[derive(Debug, Clone, Serialize)]
pub struct MemoryModule {
    pub capacity_bytes: u64,
    pub configured_speed_mt_s: Option<u32>,
    pub rated_speed_mt_s: Option<u32>,
    pub manufacturer: Option<String>,
    pub part_number: Option<String>,
    pub locator: Option<String>,
    pub memory_type: Option<String>,
}

impl MemoryData {
    pub fn usage_percent(&self) -> f64 {
        if self.total_bytes == 0 {
            return 0.0;
        }
        (self.used_bytes as f64 / self.total_bytes as f64) * 100.0
    }

    pub fn swap_percent(&self) -> f64 {
        if self.swap_total_bytes == 0 {
            return 0.0;
        }
        (self.swap_used_bytes as f64 / self.swap_total_bytes as f64) * 100.0
    }
}

pub fn collect(sys: &System) -> MemoryData {
    MemoryData {
        used_bytes: sys.used_memory(),
        total_bytes: sys.total_memory(),
        available_bytes: sys.available_memory(),
        swap_used_bytes: sys.used_swap(),
        swap_total_bytes: sys.total_swap(),
        modules: Vec::new(),
        module_status: Observation::default(),
    }
}

pub fn refresh_hardware(data: &mut MemoryData) {
    #[cfg(windows)]
    {
        let (modules, status) = collect_windows_modules();
        data.modules = modules;
        data.module_status = status;
    }

    #[cfg(not(windows))]
    {
        data.module_status = Observation::unsupported(
            "platform",
            "Physical memory module inventory is not implemented on this platform",
        );
    }
}

#[cfg(windows)]
#[derive(serde::Deserialize, Debug)]
#[serde(rename = "Win32_PhysicalMemory")]
#[serde(rename_all = "PascalCase")]
struct WmiPhysicalMemory {
    capacity: Option<u64>,
    configured_clock_speed: Option<u32>,
    speed: Option<u32>,
    manufacturer: Option<String>,
    part_number: Option<String>,
    device_locator: Option<String>,
    #[serde(rename = "SMBIOSMemoryType")]
    smbios_memory_type: Option<u16>,
}

#[cfg(windows)]
fn collect_windows_modules() -> (Vec<MemoryModule>, Observation) {
    use wmi::{COMLibrary, WMIConnection};

    let com = match COMLibrary::new() {
        Ok(com) => com,
        Err(error) => {
            return (
                Vec::new(),
                Observation::error(
                    "Win32_PhysicalMemory",
                    format!("COM initialization failed: {error}"),
                ),
            )
        }
    };
    let connection = match WMIConnection::new(com) {
        Ok(connection) => connection,
        Err(error) => {
            return (
                Vec::new(),
                Observation::error(
                    "Win32_PhysicalMemory",
                    format!("WMI connection failed: {error}"),
                ),
            )
        }
    };
    let rows = match connection.raw_query::<WmiPhysicalMemory>(
        "SELECT Capacity, ConfiguredClockSpeed, Speed, Manufacturer, PartNumber, DeviceLocator, SMBIOSMemoryType FROM Win32_PhysicalMemory",
    ) {
        Ok(rows) => rows,
        Err(error) => {
            return (
                Vec::new(),
                Observation::error("Win32_PhysicalMemory", format!("WMI query failed: {error}")),
            )
        }
    };

    let modules = rows
        .into_iter()
        .filter_map(|row| {
            let capacity_bytes = row.capacity?;
            Some(MemoryModule {
                capacity_bytes,
                configured_speed_mt_s: row.configured_clock_speed,
                rated_speed_mt_s: row.speed,
                manufacturer: clean_wmi_string(row.manufacturer),
                part_number: clean_wmi_string(row.part_number),
                locator: clean_wmi_string(row.device_locator),
                memory_type: row.smbios_memory_type.map(smbios_memory_type),
            })
        })
        .collect::<Vec<_>>();

    let status = if modules.is_empty() {
        Observation::unavailable(
            "Win32_PhysicalMemory",
            "The provider returned no populated memory modules",
        )
    } else {
        Observation::available("Win32_PhysicalMemory")
    };
    (modules, status)
}

#[cfg(windows)]
fn clean_wmi_string(value: Option<String>) -> Option<String> {
    value.and_then(|value| {
        let trimmed = value.trim().to_string();
        (!trimmed.is_empty()).then_some(trimmed)
    })
}

#[cfg(windows)]
fn smbios_memory_type(value: u16) -> String {
    match value {
        20 => "DDR".into(),
        21 => "DDR2".into(),
        24 => "DDR3".into(),
        26 => "DDR4".into(),
        34 => "DDR5".into(),
        _ => format!("SMBIOS {value}"),
    }
}

#[cfg(all(test, windows))]
mod tests {
    use super::*;

    #[test]
    fn deserializes_exact_smbios_memory_type_property() {
        let row: WmiPhysicalMemory =
            serde_json::from_value(serde_json::json!({ "SMBIOSMemoryType": 34 })).unwrap();

        assert_eq!(row.smbios_memory_type, Some(34));
    }
}
