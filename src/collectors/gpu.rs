use serde::Serialize;

use crate::observation::Observation;

use super::command::{run_output, CommandTimeout};

#[derive(Debug, Clone, Default, Serialize)]
pub struct GpuData {
    pub available: bool,
    pub telemetry_available: bool,
    pub name: String,
    pub utilization_percent: f32,
    pub memory_used_mb: u64,
    pub memory_total_mb: u64,
    pub temperature: Option<f64>,
    pub driver_version: String,
    pub adapters: Vec<GpuAdapter>,
    pub inventory_status: Observation,
    pub telemetry_status: Observation,
}

#[derive(Debug, Clone, Serialize)]
pub struct GpuAdapter {
    pub name: String,
    pub driver_version: Option<String>,
    pub status: Option<String>,
    pub dedicated_memory_mb: Option<u64>,
    pub utilization_percent: Option<f32>,
    pub memory_used_mb: Option<u64>,
    pub temperature_celsius: Option<f64>,
    pub current_resolution: Option<String>,
    pub refresh_rate_hz: Option<u32>,
    pub telemetry_available: bool,
    pub source: String,
}

impl GpuData {
    pub fn memory_percent(&self) -> f64 {
        if self.memory_total_mb == 0 {
            return 0.0;
        }
        (self.memory_used_mb as f64 / self.memory_total_mb as f64) * 100.0
    }

    fn from_adapters(adapters: Vec<GpuAdapter>, inventory_status: Observation) -> Self {
        let primary = adapters
            .iter()
            .find(|adapter| adapter.telemetry_available)
            .or_else(|| adapters.first());
        let telemetry_available = adapters.iter().any(|adapter| adapter.telemetry_available);
        let telemetry_status = if telemetry_available {
            Observation::available("vendor telemetry")
        } else if adapters.is_empty() {
            Observation::unavailable("GPU inventory", "No graphics adapters were detected")
        } else {
            Observation::unavailable(
                "GPU inventory",
                "Adapters were detected, but no utilization or temperature provider was available",
            )
        };

        Self {
            available: !adapters.is_empty(),
            telemetry_available,
            name: primary.map(|item| item.name.clone()).unwrap_or_default(),
            utilization_percent: primary
                .and_then(|item| item.utilization_percent)
                .unwrap_or_default(),
            memory_used_mb: primary
                .and_then(|item| item.memory_used_mb)
                .unwrap_or_default(),
            memory_total_mb: primary
                .and_then(|item| item.dedicated_memory_mb)
                .unwrap_or_default(),
            temperature: primary.and_then(|item| item.temperature_celsius),
            driver_version: primary
                .and_then(|item| item.driver_version.clone())
                .unwrap_or_default(),
            adapters,
            inventory_status,
            telemetry_status,
        }
    }
}

pub fn collect() -> GpuData {
    #[cfg(windows)]
    {
        collect_windows()
    }

    #[cfg(not(windows))]
    {
        let adapters = collect_nvidia();
        let status = if adapters.is_empty() {
            Observation::unsupported(
                "platform GPU inventory",
                "Only NVIDIA telemetry is implemented on this platform",
            )
        } else {
            Observation::available("nvidia-smi")
        };
        GpuData::from_adapters(adapters, status)
    }
}

#[cfg(windows)]
#[derive(serde::Deserialize, Debug)]
#[serde(rename = "Win32_VideoController")]
#[serde(rename_all = "PascalCase")]
struct WmiVideoController {
    name: Option<String>,
    driver_version: Option<String>,
    status: Option<String>,
    adapter_ram: Option<u64>,
    current_horizontal_resolution: Option<u32>,
    current_vertical_resolution: Option<u32>,
    current_refresh_rate: Option<u32>,
}

#[cfg(windows)]
fn collect_windows() -> GpuData {
    use wmi::{COMLibrary, WMIConnection};

    let (mut adapters, inventory_status) = match COMLibrary::new()
        .and_then(WMIConnection::new)
        .and_then(|connection| {
            connection.raw_query::<WmiVideoController>(
                "SELECT Name, DriverVersion, Status, AdapterRAM, CurrentHorizontalResolution, CurrentVerticalResolution, CurrentRefreshRate FROM Win32_VideoController",
            )
        }) {
        Ok(rows) => {
            let adapters = rows
                .into_iter()
                .filter_map(|row| {
                    let name = row.name?.trim().to_string();
                    if name.is_empty() {
                        return None;
                    }
                    let current_resolution = row
                        .current_horizontal_resolution
                        .zip(row.current_vertical_resolution)
                        .map(|(width, height)| format!("{width}x{height}"));
                    Some(GpuAdapter {
                        name,
                        driver_version: clean_string(row.driver_version),
                        status: clean_string(row.status),
                        dedicated_memory_mb: row.adapter_ram.map(|bytes| bytes / 1024 / 1024),
                        utilization_percent: None,
                        memory_used_mb: None,
                        temperature_celsius: None,
                        current_resolution,
                        refresh_rate_hz: row.current_refresh_rate,
                        telemetry_available: false,
                        source: "Win32_VideoController".into(),
                    })
                })
                .collect::<Vec<_>>();
            let status = if adapters.is_empty() {
                Observation::unavailable(
                    "Win32_VideoController",
                    "The provider returned no graphics adapters",
                )
            } else {
                Observation::available("Win32_VideoController")
            };
            (adapters, status)
        }
        Err(error) => (
            Vec::new(),
            Observation::error("Win32_VideoController", format!("WMI query failed: {error}")),
        ),
    };

    for telemetry in collect_nvidia() {
        if let Some(adapter) = adapters
            .iter_mut()
            .find(|adapter| gpu_names_match(&adapter.name, &telemetry.name))
        {
            adapter.utilization_percent = telemetry.utilization_percent;
            adapter.memory_used_mb = telemetry.memory_used_mb;
            adapter.dedicated_memory_mb = telemetry.dedicated_memory_mb;
            adapter.temperature_celsius = telemetry.temperature_celsius;
            adapter.driver_version = telemetry.driver_version;
            adapter.telemetry_available = true;
            adapter.source = "Win32_VideoController + nvidia-smi".into();
        } else {
            adapters.push(telemetry);
        }
    }

    GpuData::from_adapters(adapters, inventory_status)
}

fn collect_nvidia() -> Vec<GpuAdapter> {
    let Some(output) = run_output(
        "nvidia-smi",
        [
            "--query-gpu=name,utilization.gpu,memory.used,memory.total,temperature.gpu,driver_version",
            "--format=csv,noheader,nounits",
        ],
        CommandTimeout::Normal,
    ) else {
        return Vec::new();
    };

    if !output.status.success() {
        return Vec::new();
    }

    parse_nvidia_csv(&String::from_utf8_lossy(&output.stdout))
}

fn parse_nvidia_csv(csv: &str) -> Vec<GpuAdapter> {
    csv.lines()
        .filter_map(|line| {
            let parts = line.split(',').map(str::trim).collect::<Vec<_>>();
            if parts.len() != 6 || parts[0].is_empty() {
                return None;
            }
            Some(GpuAdapter {
                name: parts[0].to_string(),
                utilization_percent: parts[1].parse().ok(),
                memory_used_mb: parts[2].parse().ok(),
                dedicated_memory_mb: parts[3].parse().ok(),
                temperature_celsius: parts[4].parse().ok(),
                driver_version: clean_string(Some(parts[5].to_string())),
                status: None,
                current_resolution: None,
                refresh_rate_hz: None,
                telemetry_available: true,
                source: "nvidia-smi".into(),
            })
        })
        .collect()
}

fn clean_string(value: Option<String>) -> Option<String> {
    value.and_then(|value| {
        let value = value.trim().to_string();
        (!value.is_empty() && !value.eq_ignore_ascii_case("n/a")).then_some(value)
    })
}

#[cfg(windows)]
fn gpu_names_match(left: &str, right: &str) -> bool {
    fn normalize(value: &str) -> String {
        value
            .to_ascii_lowercase()
            .replace("nvidia", "")
            .replace("geforce", "")
            .chars()
            .filter(|character| character.is_ascii_alphanumeric())
            .collect()
    }

    let left = normalize(left);
    let right = normalize(right);
    !left.is_empty() && (left.contains(&right) || right.contains(&left))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_every_nvidia_adapter_and_preserves_unavailable_fields() {
        let rows = parse_nvidia_csv(
            "NVIDIA RTX A, 12, 100, 8192, 52, 610.74\nNVIDIA RTX B, N/A, 0, 4096, N/A, 610.74\n",
        );
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].utilization_percent, Some(12.0));
        assert_eq!(rows[1].utilization_percent, None);
        assert_eq!(rows[1].temperature_celsius, None);
    }

    #[cfg(windows)]
    #[test]
    fn matches_wmi_and_nvidia_names_without_vendor_noise() {
        assert!(gpu_names_match(
            "NVIDIA GeForce RTX 4070 Laptop GPU",
            "NVIDIA GeForce RTX 4070 Laptop GPU"
        ));
        assert!(!gpu_names_match("Intel(R) Arc(TM) Graphics", "RTX 4070"));
    }
}
