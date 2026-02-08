use super::{DiagnosticWarning, WarningSeverity};

#[derive(Debug, Clone, Default)]
pub struct DiskHealthData {
    pub drives: Vec<DriveHealth>,
}

#[derive(Debug, Clone)]
pub struct DriveHealth {
    pub device_id: String,
    pub model: String,
    pub serial: Option<String>,
    pub firmware: Option<String>,
    pub media_type: MediaType,
    pub health_status: DiskHealthStatus,
    pub temperature_celsius: Option<f64>,
    pub power_on_hours: Option<u64>,
    pub io_stats: Option<DiskIoStats>,
}

#[derive(Debug, Clone, Default)]
pub struct DiskIoStats {
    pub read_bytes_per_sec: u64,
    pub write_bytes_per_sec: u64,
    pub queue_depth: f64,
    pub avg_read_latency_ms: f64,
    pub avg_write_latency_ms: f64,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub enum MediaType {
    Ssd,
    Hdd,
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

#[derive(Debug, Clone, PartialEq, Default)]
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
fn collect_windows() -> (DiskHealthData, Vec<DiagnosticWarning>) {
    use wmi::{COMLibrary, WMIConnection};

    let mut data = DiskHealthData::default();
    let mut warnings = Vec::new();

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
                "OK" => DiskHealthStatus::Healthy,
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
                io_stats: None,
            });
        }
    }

    // Try SMART failure prediction from root\WMI (requires admin)
    if let Ok(com2) = COMLibrary::new() {
        match WMIConnection::with_namespace_path("root\\WMI", com2) {
            Ok(wmi_root) => {
                if let Ok(predictions) = wmi_root.raw_query::<WmiFailurePrediction>(
                    "SELECT PredictFailure, InstanceName FROM MSStorageDriver_FailurePredictStatus"
                ) {
                    for pred in predictions {
                        if pred.predict_failure == Some(true) {
                            // Find matching drive and upgrade to Critical
                            if let Some(ref instance) = pred.instance_name {
                                for drive in &mut data.drives {
                                    if instance.contains(&drive.device_id) && drive.health_status != DiskHealthStatus::Critical {
                                        drive.health_status = DiskHealthStatus::Critical;
                                    }
                                }
                            }
                        }
                    }
                }
            }
            Err(_) => {
                warnings.push(DiagnosticWarning {
                    source: "Disk Health".into(),
                    message: "SMART data requires Administrator privileges".into(),
                    severity: WarningSeverity::Info,
                });
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

// --- Linux implementation ---

#[cfg(target_os = "linux")]
fn collect_linux() -> (DiskHealthData, Vec<DiagnosticWarning>) {
    use std::fs;
    use std::process::Command;

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
                .trim() == "1";

            let media_type = if model.to_lowercase().contains("nvme") || name.starts_with("nvme") {
                MediaType::NVMe
            } else if is_rotational {
                MediaType::Hdd
            } else {
                MediaType::Ssd
            };

            // Try smartctl for health
            let health_status = if let Ok(output) = Command::new("smartctl")
                .args(["-H", &format!("/dev/{}", name)])
                .output()
            {
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
                io_stats: None,
            });
        }
    }

    (data, warnings)
}

// --- macOS implementation ---

#[cfg(target_os = "macos")]
fn collect_macos() -> (DiskHealthData, Vec<DiagnosticWarning>) {
    use std::process::Command;

    let mut data = DiskHealthData::default();
    let warnings = Vec::new();

    // Use diskutil to list drives
    if let Ok(output) = Command::new("diskutil")
        .args(["list", "-plist"])
        .output()
    {
        let stdout = String::from_utf8_lossy(&output.stdout);
        // Basic parsing — look for physical drives
        // macOS diskutil output varies, use simple info
        if let Ok(info_output) = Command::new("diskutil")
            .args(["info", "disk0"])
            .output()
        {
            let info = String::from_utf8_lossy(&info_output.stdout);
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

            data.drives.push(DriveHealth {
                device_id: "disk0".into(),
                model,
                serial: None,
                firmware: None,
                media_type,
                health_status: DiskHealthStatus::Unknown,
                temperature_celsius: None,
                power_on_hours: None,
                io_stats: None,
            });
        }

        // Suppress unused variable warning
        let _ = stdout;
    }

    (data, warnings)
}
