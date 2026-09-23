use serde::Serialize;

use crate::observation::Observation;

#[cfg(not(target_os = "macos"))]
use super::command::{run_output, CommandTimeout};

#[derive(Debug, Clone, Default, Serialize, serde::Deserialize)]
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

#[derive(Debug, Clone, Default, Serialize, serde::Deserialize)]
pub struct GpuAdapter {
    #[serde(default)]
    pub device_id: String,
    #[serde(default)]
    pub pci_address: Option<String>,
    #[serde(default)]
    pub shared_memory_mb: Option<u64>,
    #[serde(default)]
    pub dedicated_system_memory_mb: Option<u64>,
    #[serde(default)]
    pub unified_memory: Option<bool>,
    #[serde(default)]
    pub recommended_working_set_mb: Option<u64>,
    #[serde(default)]
    pub fields: std::collections::BTreeMap<String, Observation>,
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
    pub fn utilization(&self) -> Option<f32> {
        self.primary().and_then(|a| a.utilization_percent)
    }
    pub fn primary(&self) -> Option<&GpuAdapter> {
        self.adapters
            .iter()
            .find(|a| a.utilization_percent.is_some())
            .or_else(|| self.adapters.iter().find(|a| a.telemetry_available))
            .or_else(|| self.adapters.first())
    }
    pub fn memory_percent(&self) -> f64 {
        if self.memory_total_mb == 0 {
            return 0.0;
        }
        (self.memory_used_mb as f64 / self.memory_total_mb as f64) * 100.0
    }

    fn from_adapters(mut adapters: Vec<GpuAdapter>, inventory_status: Observation) -> Self {
        for adapter in &mut adapters {
            adapter.telemetry_available = adapter.utilization_percent.is_some()
                || adapter.memory_used_mb.is_some()
                || adapter.temperature_celsius.is_some();
            for (field, present) in [
                ("utilization_percent", adapter.utilization_percent.is_some()),
                ("memory_used_mb", adapter.memory_used_mb.is_some()),
                ("dedicated_memory_mb", adapter.dedicated_memory_mb.is_some()),
                ("temperature_celsius", adapter.temperature_celsius.is_some()),
            ] {
                adapter.fields.entry(field.into()).or_insert_with(|| {
                    if present {
                        Observation::available(&adapter.source)
                    } else {
                        Observation::unavailable(
                            &adapter.source,
                            "This provider does not expose a trustworthy reading for this field",
                        )
                    }
                });
            }
        }
        let primary = adapters
            .iter()
            .find(|adapter| adapter.utilization_percent.is_some())
            .or_else(|| adapters.iter().find(|adapter| adapter.telemetry_available))
            .or_else(|| adapters.first());
        let telemetry_available = adapters.iter().any(|adapter| adapter.telemetry_available);
        let telemetry_status = if telemetry_available {
            Observation::available("per-adapter field providers")
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

    #[cfg(target_os = "linux")]
    {
        let (mut adapters, status) =
            super::gpu_linux::collect(std::path::Path::new("/sys/class/drm"));
        merge_telemetry(&mut adapters, collect_nvidia());
        GpuData::from_adapters(adapters, status)
    }
    #[cfg(target_os = "macos")]
    {
        let (adapters, status) = super::apple_inventory::gpus();
        GpuData::from_adapters(adapters, status)
    }
    #[cfg(not(any(windows, target_os = "linux", target_os = "macos")))]
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
    #[serde(rename = "PNPDeviceID")]
    pnp_device_id: Option<String>,
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

    let (mut adapters, mut inventory_status) = match COMLibrary::new()
        .and_then(WMIConnection::new)
        .and_then(|connection| {
            connection.raw_query::<WmiVideoController>(
                "SELECT Name, PNPDeviceID, DriverVersion, Status, AdapterRAM, CurrentHorizontalResolution, CurrentVerticalResolution, CurrentRefreshRate FROM Win32_VideoController",
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
                        pci_address: row.pnp_device_id.as_deref().and_then(super::gpu_windows::pnp_pci_address),
                        device_id: row.pnp_device_id.unwrap_or_default(),
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
                        ..Default::default()
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

    if let Ok(mut dxgi) = super::gpu_windows::inventory() {
        if !dxgi.is_empty() {
            for device in &mut dxgi {
                if let Some(previous) = adapters
                    .iter()
                    .find(|a| a.pci_address.is_some() && a.pci_address == device.pci_address)
                {
                    device.driver_version = previous.driver_version.clone();
                    device.current_resolution = previous.current_resolution.clone();
                    device.refresh_rate_hz = previous.refresh_rate_hz;
                    device.status = previous.status.clone();
                }
            }
            adapters = dxgi;
            inventory_status =
                Observation::available("DXGI adapter identity and memory categories");
        }
    }
    merge_telemetry(&mut adapters, collect_nvidia());
    super::gpu_windows::add_engine_utilization(&mut adapters);

    GpuData::from_adapters(adapters, inventory_status)
}

#[cfg(not(target_os = "macos"))]
fn collect_nvidia() -> Vec<GpuAdapter> {
    let Some(output) = run_output(
        "nvidia-smi",
        [
            "--query-gpu=name,utilization.gpu,memory.used,memory.total,temperature.gpu,driver_version,uuid,pci.bus_id",
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

#[cfg(any(not(target_os = "macos"), test))]
fn parse_nvidia_csv(csv: &str) -> Vec<GpuAdapter> {
    csv.lines()
        .filter_map(|line| {
            let parts = line.split(',').map(str::trim).collect::<Vec<_>>();
            if parts.len() != 8 || parts[0].is_empty() {
                return None;
            }
            Some(GpuAdapter {
                device_id: format!("nvidia:{}", parts[6]),
                pci_address: normalize_pci(parts[7]),
                name: parts[0].to_string(),
                utilization_percent: parts[1]
                    .parse::<f32>()
                    .ok()
                    .filter(|n| n.is_finite() && (0.0..=100.0).contains(n)),
                memory_used_mb: parts[2].parse().ok(),
                dedicated_memory_mb: parts[3].parse().ok(),
                temperature_celsius: parts[4]
                    .parse::<f64>()
                    .ok()
                    .filter(|n| n.is_finite() && (-50.0..=200.0).contains(n)),
                driver_version: clean_string(Some(parts[5].to_string())),
                status: None,
                current_resolution: None,
                refresh_rate_hz: None,
                telemetry_available: true,
                source: "nvidia-smi".into(),
                ..Default::default()
            })
        })
        .collect()
}

#[cfg(any(not(target_os = "macos"), test))]
fn clean_string(value: Option<String>) -> Option<String> {
    value.and_then(|value| {
        let value = value.trim().to_string();
        (!value.is_empty() && !value.eq_ignore_ascii_case("n/a")).then_some(value)
    })
}

#[cfg(any(not(target_os = "macos"), test))]
pub(super) fn normalize_pci(value: &str) -> Option<String> {
    let (domain, tail) = value.split_once(':')?;
    let (bus, tail) = tail.split_once(':')?;
    let (device, function) = tail.split_once('.')?;
    let domain = u32::from_str_radix(domain, 16).ok()?;
    let bus = u8::from_str_radix(bus, 16).ok()?;
    let device = u8::from_str_radix(device, 16).ok()?;
    let function = u8::from_str_radix(function, 16).ok()?;
    if domain > 0xffff || device > 31 || function > 7 {
        return None;
    }
    Some(format!("{domain:04x}:{bus:02x}:{device:02x}.{function}"))
}

#[cfg(any(not(target_os = "macos"), test))]
fn merge_telemetry(adapters: &mut Vec<GpuAdapter>, telemetry: Vec<GpuAdapter>) {
    for row in telemetry {
        // PCI location, never a display name, joins independent providers.
        let matches = adapters
            .iter()
            .enumerate()
            .filter(|(_, a)| a.pci_address.is_some() && a.pci_address == row.pci_address)
            .map(|(index, _)| index)
            .collect::<Vec<_>>();
        if matches.len() == 1 {
            let target = &mut adapters[matches[0]];
            target.utilization_percent = row.utilization_percent.or(target.utilization_percent);
            target.memory_used_mb = row.memory_used_mb.or(target.memory_used_mb);
            target.temperature_celsius = row.temperature_celsius.or(target.temperature_celsius);
            target.dedicated_memory_mb = target.dedicated_memory_mb.or(row.dedicated_memory_mb);
            target.driver_version = row.driver_version.or(target.driver_version.take());
            for (key, present) in [
                ("utilization_percent", row.utilization_percent.is_some()),
                ("memory_used_mb", row.memory_used_mb.is_some()),
                ("temperature_celsius", row.temperature_celsius.is_some()),
            ] {
                if present {
                    target
                        .fields
                        .insert(key.into(), Observation::available(&row.source));
                }
            }
            target.source = format!("{} + {} (PCI identity match)", target.source, row.source);
        } else {
            // Preserve telemetry with its own stable identity if correlation
            // is unavailable; never attach it to an arbitrary identical GPU.
            adapters.push(row);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_every_nvidia_adapter_and_preserves_unavailable_fields() {
        let rows = parse_nvidia_csv(
            "NVIDIA RTX A, 12, 100, 8192, 52, 610.74, GPU-A, 00000000:01:00.0\nNVIDIA RTX B, N/A, 0, 4096, N/A, 610.74, GPU-B, 00000000:02:00.0\n",
        );
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].utilization_percent, Some(12.0));
        assert_eq!(rows[1].utilization_percent, None);
        assert_eq!(rows[1].temperature_celsius, None);
    }

    #[test]
    fn identical_names_do_not_correlate_different_adapters() {
        let mut rows = vec![
            GpuAdapter {
                name: "Same model".into(),
                pci_address: Some("0000:01:00.0".into()),
                ..Default::default()
            },
            GpuAdapter {
                name: "Same model".into(),
                pci_address: Some("0000:02:00.0".into()),
                ..Default::default()
            },
        ];
        merge_telemetry(
            &mut rows,
            vec![GpuAdapter {
                name: "Same model".into(),
                pci_address: Some("0000:02:00.0".into()),
                temperature_celsius: Some(55.0),
                ..Default::default()
            }],
        );
        assert_eq!(rows[0].temperature_celsius, None);
        assert_eq!(rows[1].temperature_celsius, Some(55.0));
        let data = GpuData::from_adapters(rows, Observation::available("fixture"));
        assert_eq!(data.utilization(), None);
        assert!(!data.adapters[1].fields["utilization_percent"].is_available());
    }
}
