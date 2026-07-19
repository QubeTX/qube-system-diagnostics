use super::DiagnosticWarning;
use crate::observation::Observation;
use serde::Serialize;

#[derive(Debug, Clone, Default, Serialize)]
pub struct DiskHealthData {
    pub drives: Vec<DriveHealth>,
    pub health_status: Observation,
    pub reliability_status: Observation,
}

#[derive(Debug, Clone, Serialize)]
pub struct DriveHealth {
    pub device_id: String,
    pub model: String,
    pub serial: Option<String>,
    pub firmware: Option<String>,
    pub media_type: MediaType,
    pub health_status: DiskHealthStatus,
    pub temperature_celsius: Option<f64>,
    pub power_on_hours: Option<u64>,
    pub wear_percent: Option<u8>,
    pub read_errors_total: Option<u64>,
    pub write_errors_total: Option<u64>,
    pub io_stats: Option<DiskIoStats>,
    pub health_source: String,
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct DiskIoStats {
    pub read_bytes_per_sec: u64,
    pub write_bytes_per_sec: u64,
    pub queue_depth: f64,
    pub avg_read_latency_ms: f64,
    pub avg_write_latency_ms: f64,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum MediaType {
    Ssd,
    Hdd,
    #[serde(rename = "nvme")]
    NVMe,
    #[default]
    Unknown,
}

impl std::fmt::Display for MediaType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Ssd => write!(f, "SSD"),
            Self::Hdd => write!(f, "HDD"),
            Self::NVMe => write!(f, "NVMe"),
            Self::Unknown => write!(f, "Unknown"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Default, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DiskHealthStatus {
    Healthy,
    Warning,
    Critical,
    #[default]
    Unknown,
}

impl DiskHealthStatus {
    pub fn user_label(&self) -> &'static str {
        match self {
            Self::Healthy => "Good",
            Self::Warning => "Degrading - Back up data",
            Self::Critical => "FAILING - Back up immediately!",
            Self::Unknown => "Unknown",
        }
    }
}

pub fn collect() -> (DiskHealthData, Vec<DiagnosticWarning>) {
    #[cfg(windows)]
    {
        collect_windows()
    }
    #[cfg(target_os = "linux")]
    {
        collect_linux()
    }
    #[cfg(target_os = "macos")]
    {
        collect_macos()
    }
    #[cfg(not(any(windows, target_os = "linux", target_os = "macos")))]
    {
        (DiskHealthData::default(), Vec::new())
    }
}

// --- Windows implementation ---

#[cfg(windows)]
use serde::Deserialize;

#[cfg(windows)]
#[derive(Deserialize, Debug)]
#[serde(rename = "Win32_DiskDrive")]
#[serde(rename_all = "PascalCase")]
struct WmiDiskDrive {
    #[serde(rename = "DeviceID")]
    device_id: Option<String>,
    model: Option<String>,
    serial_number: Option<String>,
    firmware_revision: Option<String>,
    media_type: Option<String>,
    status: Option<String>,
}

#[cfg(windows)]
#[derive(Deserialize, Debug)]
#[serde(rename_all = "PascalCase")]
struct WmiFailurePrediction {
    predict_failure: Option<bool>,
    instance_name: Option<String>,
}

#[cfg(windows)]
#[derive(Deserialize, Debug)]
#[serde(rename_all = "PascalCase")]
struct WmiDiskPerf {
    name: Option<String>,
    disk_read_bytes_per_sec: Option<u64>,
    disk_write_bytes_per_sec: Option<u64>,
    current_disk_queue_length: Option<u32>,
    avg_disk_sec_per_read: Option<u32>,
    avg_disk_sec_per_write: Option<u32>,
}

#[cfg(windows)]
#[derive(Deserialize, Debug)]
#[serde(rename_all = "PascalCase")]
struct WmiPhysicalDisk {
    #[serde(rename = "DeviceId")]
    device_id: Option<String>,
    friendly_name: Option<String>,
    health_status: Option<u16>,
    media_type: Option<u16>,
    bus_type: Option<u16>,
    firmware_version: Option<String>,
}

#[cfg(windows)]
#[derive(Deserialize, Debug)]
#[serde(rename_all = "PascalCase")]
struct WmiStorageReliabilityCounter {
    #[serde(rename = "DeviceId")]
    device_id: Option<String>,
    temperature: Option<u64>,
    power_on_hours: Option<u64>,
    wear: Option<u8>,
    read_errors_total: Option<u64>,
    write_errors_total: Option<u64>,
}

#[cfg(windows)]
fn collect_windows() -> (DiskHealthData, Vec<DiagnosticWarning>) {
    use wmi::{COMLibrary, WMIConnection};

    let mut data = DiskHealthData::default();
    let warnings = Vec::new();
    data.health_status = Observation::unavailable(
        "Windows storage providers",
        "No authoritative physical-disk health provider has returned data",
    );
    data.reliability_status = Observation::unavailable(
        "MSFT_StorageReliabilityCounter",
        "The reliability provider has not returned data",
    );

    let com = match COMLibrary::new() {
        Ok(c) => c,
        Err(_) => return (data, warnings),
    };

    let wmi = match WMIConnection::new(com) {
        Ok(w) => w,
        Err(_) => return (data, warnings),
    };

    // Query physical drives
    if let Ok(drives) = wmi.raw_query::<WmiDiskDrive>(
        "SELECT DeviceID, Model, SerialNumber, FirmwareRevision, MediaType, Status FROM Win32_DiskDrive"
    ) {
        for drive in drives {
            let model = drive.model.unwrap_or_default();
            let media_type_str = drive.media_type.as_deref().unwrap_or("");
            let status_str = drive.status.as_deref().unwrap_or("Unknown");

            let media_type = if model.to_lowercase().contains("nvme") {
                MediaType::NVMe
            } else if media_type_str.contains("Fixed hard disk") || media_type_str.contains("External hard disk") {
                // Could be SSD or HDD — check model name for hints
                if model.to_lowercase().contains("ssd") || model.to_lowercase().contains("solid") {
                    MediaType::Ssd
                } else {
                    MediaType::Unknown
                }
            } else {
                MediaType::Unknown
            };

            let health_status = match status_str {
                "OK" => DiskHealthStatus::Unknown,
                "Degraded" => DiskHealthStatus::Warning,
                "Pred Fail" | "Error" => DiskHealthStatus::Critical,
                _ => DiskHealthStatus::Unknown,
            };

            data.drives.push(DriveHealth {
                device_id: drive.device_id.unwrap_or_default(),
                model,
                serial: drive.serial_number,
                firmware: drive.firmware_revision,
                media_type,
                health_status,
                temperature_celsius: None,
                power_on_hours: None,
                wear_percent: None,
                read_errors_total: None,
                write_errors_total: None,
                io_stats: None,
                health_source: "Win32_DiskDrive inventory status".into(),
            });
        }
    }

    let (storage_health, reliability_status) = collect_windows_storage_details(&mut data);
    data.health_status = storage_health;
    data.reliability_status = reliability_status;

    // Try SMART failure prediction from root\WMI (requires admin)
    if let Ok(com2) = COMLibrary::new() {
        if let Ok(wmi_root) = WMIConnection::with_namespace_path("root\\WMI", com2) {
            if let Ok(predictions) = wmi_root.raw_query::<WmiFailurePrediction>(
                "SELECT PredictFailure, InstanceName FROM MSStorageDriver_FailurePredictStatus",
            ) {
                let had_predictions = !predictions.is_empty();
                for pred in predictions {
                    if pred.predict_failure == Some(true) {
                        // Find matching drive and upgrade to Critical
                        if let Some(ref instance) = pred.instance_name {
                            for drive in &mut data.drives {
                                if instance.contains(&drive.device_id)
                                    && drive.health_status != DiskHealthStatus::Critical
                                {
                                    drive.health_status = DiskHealthStatus::Critical;
                                }
                            }
                        }
                    }
                }
                if had_predictions && !data.health_status.is_available() {
                    data.health_status =
                        Observation::available("MSStorageDriver_FailurePredictStatus");
                }
            }
        }
    }

    // I/O performance from root\cimv2
    if let Ok(com3) = COMLibrary::new() {
        if let Ok(wmi3) = WMIConnection::new(com3) {
            if let Ok(perfs) = wmi3.raw_query::<WmiDiskPerf>(
                "SELECT Name, DiskReadBytesPerSec, DiskWriteBytesPerSec, CurrentDiskQueueLength, AvgDiskSecPerRead, AvgDiskSecPerWrite FROM Win32_PerfFormattedData_PerfDisk_PhysicalDisk"
            ) {
                for perf in perfs {
                    let name = perf.name.as_deref().unwrap_or("");
                    // Name format: "0 C:" or "1 D:" — match by disk index
                    if name == "_Total" {
                        continue;
                    }
                    let disk_index = name.split_whitespace().next()
                        .and_then(|s| s.parse::<usize>().ok());

                    if let Some(idx) = disk_index {
                        if let Some(drive) = data.drives.get_mut(idx) {
                            drive.io_stats = Some(DiskIoStats {
                                read_bytes_per_sec: perf.disk_read_bytes_per_sec.unwrap_or(0),
                                write_bytes_per_sec: perf.disk_write_bytes_per_sec.unwrap_or(0),
                                queue_depth: perf.current_disk_queue_length.unwrap_or(0) as f64,
                                avg_read_latency_ms: perf.avg_disk_sec_per_read.unwrap_or(0) as f64,
                                avg_write_latency_ms: perf.avg_disk_sec_per_write.unwrap_or(0) as f64,
                            });
                        }
                    }
                }
            }
        }
    }

    (data, warnings)
}

#[cfg(windows)]
fn collect_windows_storage_details(data: &mut DiskHealthData) -> (Observation, Observation) {
    use wmi::{COMLibrary, WMIConnection};

    let com = match COMLibrary::new() {
        Ok(com) => com,
        Err(error) => {
            let observation = windows_storage_error("Windows Storage WMI", &error.to_string());
            return (observation.clone(), observation);
        }
    };
    let connection =
        match WMIConnection::with_namespace_path("root\\Microsoft\\Windows\\Storage", com) {
            Ok(connection) => connection,
            Err(error) => {
                let observation = windows_storage_error("Windows Storage WMI", &error.to_string());
                return (observation.clone(), observation);
            }
        };

    let health_status = match connection.raw_query::<WmiPhysicalDisk>(
        "SELECT DeviceId, FriendlyName, HealthStatus, MediaType, BusType, FirmwareVersion FROM MSFT_PhysicalDisk",
    ) {
        Ok(rows) if rows.is_empty() => Observation::unavailable(
            "MSFT_PhysicalDisk",
            "The provider returned no physical disks",
        ),
        Ok(rows) => {
            let mut usable_health = false;
            for row in rows {
                let index = row
                    .device_id
                    .as_deref()
                    .and_then(|value| value.parse::<usize>().ok());
                let matched_index = index.filter(|index| *index < data.drives.len()).or_else(|| {
                    row.friendly_name.as_deref().and_then(|name| {
                        data.drives
                            .iter()
                            .position(|drive| drive.model.eq_ignore_ascii_case(name))
                    })
                });
                let Some(drive) = matched_index.and_then(|index| data.drives.get_mut(index)) else {
                    continue;
                };

                if let Some(status) = row.health_status {
                    usable_health = true;
                    drive.health_status = match status {
                        0 => DiskHealthStatus::Healthy,
                        1 => DiskHealthStatus::Warning,
                        2 => DiskHealthStatus::Critical,
                        _ => DiskHealthStatus::Unknown,
                    };
                    drive.health_source = "MSFT_PhysicalDisk.HealthStatus".into();
                }
                if row.bus_type == Some(17) {
                    drive.media_type = MediaType::NVMe;
                } else {
                    drive.media_type = match row.media_type {
                        Some(3) => MediaType::Hdd,
                        Some(4) => MediaType::Ssd,
                        _ => drive.media_type.clone(),
                    };
                }
                if let Some(firmware) = row.firmware_version.filter(|value| !value.trim().is_empty())
                {
                    drive.firmware = Some(firmware.trim().to_string());
                }
            }
            if usable_health {
                Observation::available("MSFT_PhysicalDisk.HealthStatus")
            } else {
                Observation::unavailable(
                    "MSFT_PhysicalDisk.HealthStatus",
                    "Physical disks were listed without health values",
                )
            }
        }
        Err(error) => windows_storage_error("MSFT_PhysicalDisk", &error.to_string()),
    };

    let reliability_status = match connection.raw_query::<WmiStorageReliabilityCounter>(
        "SELECT DeviceId, Temperature, PowerOnHours, Wear, ReadErrorsTotal, WriteErrorsTotal FROM MSFT_StorageReliabilityCounter",
    ) {
        Ok(rows) if rows.is_empty() => Observation::unavailable(
            "MSFT_StorageReliabilityCounter",
            "The provider returned no reliability counters for these drives",
        ),
        Ok(rows) => {
            let mut usable = false;
            for row in rows {
                let Some(index) = row
                    .device_id
                    .as_deref()
                    .and_then(|value| value.parse::<usize>().ok())
                else {
                    continue;
                };
                let Some(drive) = data.drives.get_mut(index) else {
                    continue;
                };
                drive.temperature_celsius = row.temperature.map(|value| value as f64);
                drive.power_on_hours = row.power_on_hours;
                drive.wear_percent = row.wear;
                drive.read_errors_total = row.read_errors_total;
                drive.write_errors_total = row.write_errors_total;
                usable |= drive.temperature_celsius.is_some()
                    || drive.power_on_hours.is_some()
                    || drive.wear_percent.is_some()
                    || drive.read_errors_total.is_some()
                    || drive.write_errors_total.is_some();
            }
            if usable {
                Observation::available("MSFT_StorageReliabilityCounter")
            } else {
                Observation::unavailable(
                    "MSFT_StorageReliabilityCounter",
                    "Reliability rows contained no usable counters",
                )
            }
        }
        Err(error) => windows_storage_error(
            "MSFT_StorageReliabilityCounter",
            &error.to_string(),
        ),
    };

    (health_status, reliability_status)
}

#[cfg(windows)]
fn windows_storage_error(source: &str, error: &str) -> Observation {
    let normalized = error.to_ascii_lowercase();
    if normalized.contains("access denied") || normalized.contains("0x80041003") {
        Observation::permission_denied(source, error)
    } else if normalized.contains("0x8004100c") || normalized.contains("0x80041010") {
        Observation::unsupported(source, error)
    } else {
        Observation::error(source, error)
    }
}

// --- Linux implementation ---

#[cfg(target_os = "linux")]
fn collect_linux() -> (DiskHealthData, Vec<DiagnosticWarning>) {
    use super::command::{run_output, CommandTimeout};
    use std::fs;

    let mut data = DiskHealthData::default();
    let warnings = Vec::new();

    // Read block devices from /sys/block/
    if let Ok(entries) = fs::read_dir("/sys/block") {
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            // Skip loop, ram, and dm devices
            if name.starts_with("loop") || name.starts_with("ram") || name.starts_with("dm-") {
                continue;
            }

            let model_path = format!("/sys/block/{}/device/model", name);
            let model = fs::read_to_string(&model_path)
                .unwrap_or_default()
                .trim()
                .to_string();

            let rotational_path = format!("/sys/block/{}/queue/rotational", name);
            let is_rotational = fs::read_to_string(&rotational_path)
                .unwrap_or_default()
                .trim()
                == "1";

            let media_type = if model.to_lowercase().contains("nvme") || name.starts_with("nvme") {
                MediaType::NVMe
            } else if is_rotational {
                MediaType::Hdd
            } else {
                MediaType::Ssd
            };

            // Try smartctl for health
            let health_status = if let Some(output) = run_output(
                "smartctl",
                ["-H", &format!("/dev/{}", name)],
                CommandTimeout::Normal,
            ) {
                let stdout = String::from_utf8_lossy(&output.stdout);
                if stdout.contains("PASSED") || stdout.contains("OK") {
                    DiskHealthStatus::Healthy
                } else if stdout.contains("FAILED") {
                    DiskHealthStatus::Critical
                } else {
                    DiskHealthStatus::Unknown
                }
            } else {
                DiskHealthStatus::Unknown
            };

            data.drives.push(DriveHealth {
                device_id: format!("/dev/{}", name),
                model,
                serial: None,
                firmware: None,
                media_type,
                health_status,
                temperature_celsius: None,
                power_on_hours: None,
                wear_percent: None,
                read_errors_total: None,
                write_errors_total: None,
                io_stats: None,
                health_source: "smartctl".into(),
            });
        }
    }

    data.health_status = if data
        .drives
        .iter()
        .any(|drive| drive.health_status != DiskHealthStatus::Unknown)
    {
        Observation::available("smartctl -H")
    } else {
        Observation::unavailable(
            "smartctl -H",
            "No supported drive returned an authoritative health result",
        )
    };
    data.reliability_status = Observation::unavailable(
        "smartctl",
        "Detailed reliability counters are not collected by this implementation",
    );
    (data, warnings)
}

// --- macOS implementation ---

#[cfg(target_os = "macos")]
fn collect_macos() -> (DiskHealthData, Vec<DiagnosticWarning>) {
    use super::command::{run_output, CommandTimeout};

    let mut data = DiskHealthData::default();
    let warnings = Vec::new();

    // Use diskutil to list drives
    if let Some(output) = run_output("diskutil", ["list", "-plist"], CommandTimeout::Normal) {
        let stdout = String::from_utf8_lossy(&output.stdout);
        // Basic parsing — look for physical drives
        // macOS diskutil output varies, use simple info
        if let Some(info_output) = run_output("diskutil", ["info", "disk0"], CommandTimeout::Normal)
        {
            let info = String::from_utf8_lossy(&info_output.stdout);
            let (model, media_type) = parse_diskutil_info(&info);

            data.drives.push(DriveHealth {
                device_id: "disk0".into(),
                model,
                serial: None,
                firmware: None,
                media_type,
                health_status: DiskHealthStatus::Unknown,
                temperature_celsius: None,
                power_on_hours: None,
                wear_percent: None,
                read_errors_total: None,
                write_errors_total: None,
                io_stats: None,
                health_source: "diskutil inventory only".into(),
            });
        }

        // Suppress unused variable warning
        let _ = stdout;
    }

    data.health_status = Observation::unavailable(
        "diskutil inventory",
        "The current macOS collector does not parse an authoritative health result",
    );
    data.reliability_status = Observation::unsupported(
        "diskutil inventory",
        "Native NVMe reliability telemetry is not implemented yet",
    );
    (data, warnings)
}

#[cfg(target_os = "macos")]
fn parse_diskutil_info(info: &str) -> (String, MediaType) {
    let mut model = String::new();
    let mut media_type = MediaType::Unknown;

    for line in info.lines() {
        let trimmed = line.trim();
        if let Some(rest) = trimmed.strip_prefix("Device / Media Name:") {
            model = rest.trim().to_string();
        }
        if let Some(rest) = trimmed.strip_prefix("Solid State:") {
            if rest.trim() == "Yes" {
                media_type = MediaType::Ssd;
            } else {
                media_type = MediaType::Hdd;
            }
        }
    }

    if model.to_lowercase().contains("nvme") {
        media_type = MediaType::NVMe;
    }

    (model, media_type)
}

#[cfg(all(test, target_os = "macos"))]
mod macos_tests {
    use super::{parse_diskutil_info, MediaType};

    #[test]
    fn parses_diskutil_info_fixture() {
        let fixture = r#"
            Device Identifier:         disk0
            Device / Media Name:       APPLE SSD AP1024N
            Solid State:               Yes
        "#;

        let (model, media_type) = parse_diskutil_info(fixture);

        assert_eq!(model, "APPLE SSD AP1024N");
        assert_eq!(media_type, MediaType::Ssd);
    }

    #[test]
    fn diskutil_nvme_model_overrides_solid_state_label() {
        let fixture = r#"
            Device / Media Name:       Example NVMe Media
            Solid State:               Yes
        "#;

        let (_, media_type) = parse_diskutil_info(fixture);

        assert_eq!(media_type, MediaType::NVMe);
    }
}
