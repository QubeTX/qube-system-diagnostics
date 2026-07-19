use serde::Serialize;
use sysinfo::System;

use crate::observation::Observation;

#[derive(Debug, Clone, Default, Serialize)]
pub struct SystemInfoData {
    pub os_name: String,
    pub os_version: String,
    pub hostname: String,
    pub cpu_model: String,
    pub cpu_cores: usize,
    pub cpu_threads: usize,
    pub total_memory_bytes: u64,
    pub architecture: String,
    pub uptime_seconds: u64,
    pub kernel_version: String,
    pub manufacturer: Option<String>,
    pub model: Option<String>,
    pub bios_version: Option<String>,
    pub bios_release_date: Option<String>,
    pub hypervisor_present: Option<bool>,
    pub hardware_status: Observation,
}

pub fn collect(sys: &System) -> SystemInfoData {
    let cpu_model = sys
        .cpus()
        .first()
        .map(|c| c.brand().to_string())
        .unwrap_or_else(|| "Unknown".into());

    let cpu_cores = System::physical_core_count().unwrap_or(0);
    let cpu_threads = sys.cpus().len();

    let mut data = SystemInfoData {
        os_name: System::name().unwrap_or_else(|| "Unknown".into()),
        os_version: System::os_version().unwrap_or_else(|| "Unknown".into()),
        hostname: System::host_name().unwrap_or_else(|| "Unknown".into()),
        cpu_model,
        cpu_cores,
        cpu_threads,
        total_memory_bytes: sys.total_memory(),
        architecture: std::env::consts::ARCH.to_string(),
        uptime_seconds: System::uptime(),
        kernel_version: System::kernel_version().unwrap_or_else(|| "Unknown".into()),
        manufacturer: None,
        model: None,
        bios_version: None,
        bios_release_date: None,
        hypervisor_present: None,
        hardware_status: Observation::default(),
    };

    #[cfg(windows)]
    refresh_windows_hardware(&mut data);

    #[cfg(not(windows))]
    {
        data.hardware_status = Observation::unsupported(
            "platform hardware identity",
            "Detailed manufacturer, model, and firmware collection is not implemented",
        );
    }

    data
}

#[cfg(windows)]
#[derive(serde::Deserialize, Debug)]
#[serde(rename = "Win32_ComputerSystem")]
#[serde(rename_all = "PascalCase")]
struct WmiComputerSystem {
    manufacturer: Option<String>,
    model: Option<String>,
    hypervisor_present: Option<bool>,
}

#[cfg(windows)]
#[derive(serde::Deserialize, Debug)]
#[serde(rename = "Win32_BIOS")]
#[serde(rename_all = "PascalCase")]
struct WmiBios {
    #[serde(rename = "SMBIOSBIOSVersion")]
    smbios_bios_version: Option<String>,
    release_date: Option<String>,
}

#[cfg(windows)]
fn refresh_windows_hardware(data: &mut SystemInfoData) {
    use wmi::{COMLibrary, WMIConnection};

    let result = COMLibrary::new().and_then(WMIConnection::new);
    let connection = match result {
        Ok(connection) => connection,
        Err(error) => {
            data.hardware_status = Observation::error(
                "Win32_ComputerSystem/Win32_BIOS",
                format!("WMI connection failed: {error}"),
            );
            return;
        }
    };

    let computer = connection
        .raw_query::<WmiComputerSystem>(
            "SELECT Manufacturer, Model, HypervisorPresent FROM Win32_ComputerSystem",
        )
        .ok()
        .and_then(|rows| rows.into_iter().next());
    let bios = connection
        .raw_query::<WmiBios>("SELECT SMBIOSBIOSVersion, ReleaseDate FROM Win32_BIOS")
        .ok()
        .and_then(|rows| rows.into_iter().next());

    if let Some(computer) = computer {
        data.manufacturer = clean_string(computer.manufacturer);
        data.model = clean_string(computer.model);
        data.hypervisor_present = computer.hypervisor_present;
    }
    if let Some(bios) = bios {
        data.bios_version = clean_string(bios.smbios_bios_version);
        data.bios_release_date = clean_string(bios.release_date);
    }
    data.hardware_status = if data.model.is_some() || data.bios_version.is_some() {
        Observation::available("Win32_ComputerSystem + Win32_BIOS")
    } else {
        Observation::unavailable(
            "Win32_ComputerSystem + Win32_BIOS",
            "The providers returned no usable manufacturer, model, or firmware values",
        )
    };
}

#[cfg(windows)]
fn clean_string(value: Option<String>) -> Option<String> {
    value.and_then(|value| {
        let value = value.trim().to_string();
        (!value.is_empty()).then_some(value)
    })
}
