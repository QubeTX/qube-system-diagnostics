use super::DiagnosticWarning;
use crate::observation::Observation;
use serde::Serialize;

#[derive(Debug, Clone, Default, Serialize, serde::Deserialize)]
pub struct DiskHealthData {
    pub drives: Vec<DriveHealth>,
    pub health_status: Observation,
    pub reliability_status: Observation,
}

#[derive(Debug, Clone, Serialize, serde::Deserialize)]
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

#[derive(Debug, Clone, Default, Serialize, serde::Deserialize)]
pub struct DiskIoStats {
    pub read_bytes_per_sec: u64,
    pub write_bytes_per_sec: u64,
    pub queue_depth: f64,
    pub avg_read_latency_ms: f64,
    pub avg_write_latency_ms: f64,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, serde::Deserialize)]
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

#[derive(Debug, Clone, PartialEq, Default, Serialize, serde::Deserialize)]
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
    let (mut data, warnings) = collect_platform();
    for drive in &mut data.drives {
        if let Some(observation) = collect_smart(drive) {
            if observation.is_available() || !data.reliability_status.is_available() {
                data.reliability_status = observation;
            }
        }
    }
    if data.drives.iter().any(|drive| {
        drive.health_source == "smartctl JSON" && drive.health_status != DiskHealthStatus::Unknown
    }) {
        data.health_status = Observation::available("smartctl JSON and platform storage health");
    }
    (data, warnings)
}

fn collect_platform() -> (DiskHealthData, Vec<DiagnosticWarning>) {
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
struct WmiPhysicalDisk {
    #[serde(rename = "DeviceId")]
    device_id: Option<String>,
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
                let Some(drive) = data.drives.iter_mut().find(|drive| physical_drive_number(&drive.device_id) == index && index.is_some()) else {
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
                let Some(drive) = data.drives.iter_mut().find(|drive| physical_drive_number(&drive.device_id) == Some(index)) else {
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

#[cfg(any(windows, test))]
fn physical_drive_number(id: &str) -> Option<usize> {
    id.to_ascii_lowercase()
        .strip_prefix(r"\\.\physicaldrive")?
        .parse()
        .ok()
}

#[cfg(any(not(windows), test))]
fn empty_drive(device_id: String, model: String, media_type: MediaType) -> DriveHealth {
    DriveHealth {
        device_id,
        model,
        media_type,
        serial: None,
        firmware: None,
        health_status: DiskHealthStatus::Unknown,
        temperature_celsius: None,
        power_on_hours: None,
        wear_percent: None,
        read_errors_total: None,
        write_errors_total: None,
        io_stats: None,
        health_source: "inventory only".into(),
    }
}

#[cfg(target_os = "linux")]
fn collect_linux() -> (DiskHealthData, Vec<DiagnosticWarning>) {
    let mut data = DiskHealthData::default();
    if let Ok(entries) = std::fs::read_dir("/sys/block") {
        for entry in entries.flatten() {
            let path = entry.path();
            if !path.join("device").exists() {
                continue;
            }
            let name = entry.file_name().to_string_lossy().into_owned();
            let read = |suffix| {
                std::fs::read_to_string(path.join(suffix))
                    .ok()
                    .map(|s| s.trim().to_owned())
            };
            let media = if name.starts_with("nvme") {
                MediaType::NVMe
            } else {
                match read("queue/rotational").as_deref() {
                    Some("1") => MediaType::Hdd,
                    Some("0") => MediaType::Ssd,
                    _ => MediaType::Unknown,
                }
            };
            let mut drive = empty_drive(
                format!("/dev/{name}"),
                read("device/model").unwrap_or_else(|| name.clone()),
                media,
            );
            drive.serial = read("device/serial");
            drive.firmware = read("device/firmware_rev");
            data.drives.push(drive);
        }
    }
    data.health_status = Observation::unavailable(
        "sysfs inventory",
        "Detailed health needs a supported readable SMART provider",
    );
    data.reliability_status = Observation::unavailable(
        "smartctl",
        "Install smartmontools explicitly to enable supported read-only storage health",
    );
    (data, Vec::new())
}

#[cfg(target_os = "macos")]
fn collect_macos() -> (DiskHealthData, Vec<DiagnosticWarning>) {
    let mut data = DiskHealthData::default();
    let list =
        match super::macos::command_plist("/usr/sbin/diskutil", &["list", "-plist", "physical"]) {
            Ok(list) => list,
            Err(error) => {
                data.health_status = Observation::error("diskutil", error);
                return (data, Vec::new());
            }
        };
    for id in super::macos::physical_disks(&list) {
        if let Ok(info) =
            super::macos::command_plist("/usr/sbin/diskutil", &["info", "-plist", &id])
        {
            if let Some(drive) = parse_diskutil_info(&id, &info) {
                data.drives.push(drive);
            }
        }
    }
    data.health_status = if data
        .drives
        .iter()
        .any(|d| d.health_status != DiskHealthStatus::Unknown)
    {
        Observation::available("diskutil SMARTStatus")
    } else {
        Observation::unavailable("diskutil SMARTStatus", "The storage driver exposes no SMART status; a supported smartctl helper may add detail")
    };
    data.reliability_status = Observation::unavailable(
        "smartctl",
        "Optional detailed reliability counters require a supported readable device",
    );
    (data, Vec::new())
}

#[cfg(any(target_os = "macos", test))]
fn parse_diskutil_info(id: &str, info: &plist::Value) -> Option<DriveHealth> {
    if !super::macos::valid_disk_id(id) {
        return None;
    }
    let dict = info.as_dictionary()?;
    let string = |key| dict.get(key).and_then(plist::Value::as_string);
    let media = if string("BusProtocol") == Some("PCI-Express")
        && string("DeviceModel").is_some_and(|s| s.contains("NVMe"))
    {
        MediaType::NVMe
    } else {
        match dict.get("SolidState").and_then(plist::Value::as_boolean) {
            Some(true) => MediaType::Ssd,
            Some(false) => MediaType::Hdd,
            None => MediaType::Unknown,
        }
    };
    let mut drive = empty_drive(
        format!("/dev/{id}"),
        string("MediaName")
            .or_else(|| string("DeviceModel"))
            .unwrap_or(id)
            .into(),
        media,
    );
    drive.health_status = match string("SMARTStatus") {
        Some("Verified") => DiskHealthStatus::Healthy,
        Some("Failing") => DiskHealthStatus::Critical,
        _ => DiskHealthStatus::Unknown,
    };
    drive.health_source = "diskutil SMARTStatus".into();
    Some(drive)
}

fn collect_smart(drive: &mut DriveHealth) -> Option<Observation> {
    use super::command::{run_checked, CommandError, CommandTimeout};
    let device = drive.device_id.clone();
    #[cfg(windows)]
    let device = physical_drive_number(&device)
        .map(|i| format!("/dev/pd{i}"))
        .unwrap_or(device);
    let executable = crate::optional_tools::detect("smartctl")?;
    match run_checked(
        executable,
        ["--json", "--all", "--nocheck=standby,3", &device],
        CommandTimeout::Slow,
        &std::sync::atomic::AtomicBool::new(false),
    ) {
        Ok(output) => Some(apply_smart_json(
            drive,
            &output.stdout,
            output.status.code().unwrap_or(255),
        )),
        Err(CommandError::NotFound) => None,
        Err(CommandError::PermissionDenied) => Some(Observation::permission_denied(
            "smartctl",
            "This read-only probe requires device access permission",
        )),
        Err(error) => Some(Observation::error("smartctl", error.to_string())),
    }
}

/// smartctl exits are a bitmask, not a success boolean. Preserve measured
/// attributes from partial responses without turning command failures into a
/// hardware failure (or declaring a device healthy from exit 0 alone).
fn apply_smart_json(drive: &mut DriveHealth, bytes: &[u8], exit: i32) -> Observation {
    let Ok(value) = serde_json::from_slice::<serde_json::Value>(bytes) else {
        return Observation::error("smartctl JSON", "Malformed or missing structured output");
    };
    let reported = value
        .pointer("/smartctl/exit_status")
        .and_then(|v| v.as_i64());
    if reported != Some(i64::from(exit)) || !(0..=255).contains(&exit) {
        return Observation::error(
            "smartctl JSON",
            "Exit-status metadata does not match the helper outcome",
        );
    }
    let has = |path: &str| value.pointer(path).and_then(|v| v.as_u64());
    let text = |path: &str| {
        value
            .pointer(path)
            .and_then(|v| v.as_str())
            .map(str::to_owned)
    };
    let passed = value
        .pointer("/smart_status/passed")
        .and_then(|v| v.as_bool());
    if passed == Some(false) || exit & 0x18 != 0 {
        drive.health_status = DiskHealthStatus::Critical;
        drive.health_source = "smartctl JSON".into();
    } else if exit & 0xe0 != 0 {
        drive.health_status = DiskHealthStatus::Warning;
        drive.health_source = "smartctl JSON".into();
    } else if passed == Some(true) && exit & 7 == 0 {
        drive.health_status = DiskHealthStatus::Healthy;
        drive.health_source = "smartctl JSON".into();
    }
    drive.serial = text("/serial_number").or(drive.serial.take());
    drive.firmware = text("/firmware_version").or(drive.firmware.take());
    drive.temperature_celsius = value
        .pointer("/temperature/current")
        .and_then(|v| v.as_f64())
        .filter(|t| t.is_finite() && *t >= -273.15)
        .or(drive.temperature_celsius);
    drive.power_on_hours = has("/power_on_time/hours").or(drive.power_on_hours);
    drive.wear_percent = has("/nvme_smart_health_information_log/percentage_used")
        .map(|v| v.min(255) as u8)
        .or(drive.wear_percent);
    // NVMe media_errors is combined; do not falsely attribute it to reads or writes.
    drive.read_errors_total =
        has("/scsi_error_counter_log/read/total_uncorrected_errors").or(drive.read_errors_total);
    drive.write_errors_total =
        has("/scsi_error_counter_log/write/total_uncorrected_errors").or(drive.write_errors_total);
    if exit & 7 != 0 {
        let denied = value
            .pointer("/smartctl/messages")
            .and_then(|v| v.as_array())
            .is_some_and(|messages| {
                messages
                    .iter()
                    .filter_map(|v| v.get("string").and_then(|v| v.as_str()))
                    .any(|s| {
                        s.to_ascii_lowercase().contains("permission denied")
                            || s.to_ascii_lowercase().contains("access is denied")
                    })
            });
        if denied {
            Observation::permission_denied(
                "smartctl JSON",
                "Device access was denied; unprivileged inventory remains available",
            )
        } else {
            Observation::unavailable("smartctl JSON", "The read was incomplete, unsupported, or the device is sleeping; available fields are retained")
        }
    } else if passed.is_some()
        || drive.temperature_celsius.is_some()
        || drive.power_on_hours.is_some()
    {
        Observation::available("smartctl JSON")
    } else {
        Observation::unavailable(
            "smartctl JSON",
            "No supported health or reliability fields were returned",
        )
    }
}

#[cfg(test)]
mod storage_tests {
    use super::*;
    #[test]
    fn device_numbers_do_not_depend_on_provider_order() {
        assert_eq!(physical_drive_number(r"\\.\PHYSICALDRIVE12"), Some(12));
        assert_eq!(physical_drive_number("12"), None);
    }
    #[test]
    fn smart_exit_bits_preserve_partial_data_and_distinguish_faults() {
        let mut drive = empty_drive("fixture".into(), "fixture".into(), MediaType::Unknown);
        let result = apply_smart_json(&mut drive, br#"{"smartctl":{"exit_status":4},"temperature":{"current":41},"smart_status":{"passed":true}}"#, 4);
        assert!(!result.is_available());
        assert_eq!(drive.temperature_celsius, Some(41.0));
        assert_eq!(drive.health_status, DiskHealthStatus::Unknown);
        apply_smart_json(
            &mut drive,
            br#"{"smartctl":{"exit_status":8},"smart_status":{"passed":false}}"#,
            8,
        );
        assert_eq!(drive.health_status, DiskHealthStatus::Critical);
        let bad = apply_smart_json(&mut drive, br#"{"smartctl":{"exit_status":0}}"#, 1);
        assert_eq!(bad.status, crate::observation::ObservationStatus::Error);
    }
    #[test]
    fn diskutil_structured_health_does_not_guess_missing_media() {
        let info = super::super::macos::parse_plist(br#"<plist version="1.0"><dict><key>MediaName</key><string>External Disk</string><key>SMARTStatus</key><string>Verified</string></dict></plist>"#).unwrap();
        let drive = parse_diskutil_info("disk4", &info).unwrap();
        assert_eq!(drive.device_id, "/dev/disk4");
        assert_eq!(drive.health_status, DiskHealthStatus::Healthy);
        assert_eq!(drive.media_type, MediaType::Unknown);
    }
}
