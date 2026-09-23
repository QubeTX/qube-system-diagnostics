use super::DiagnosticWarning;
use crate::observation::{Observation, ObservationStatus};
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
            if observation.status == ObservationStatus::Contradictory {
                data.health_status = observation.clone();
            }
            if observation.is_available() || !data.reliability_status.is_available() {
                data.reliability_status = observation;
            }
        }
    }
    if data.health_status.status != ObservationStatus::Contradictory
        && data.drives.iter().any(|drive| {
            drive.health_source.contains("smartctl JSON")
                && drive.health_status != DiskHealthStatus::Unknown
        })
    {
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
    #[serde(rename = "PNPDeviceID")]
    pnp_device_id: Option<String>,
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
    #[serde(rename = "__Path")]
    path: String,
    serial_number: Option<String>,
    health_status: Option<u16>,
    media_type: Option<u16>,
    bus_type: Option<u16>,
    firmware_version: Option<String>,
}

#[cfg(windows)]
#[derive(Deserialize, Debug)]
#[serde(rename = "MSFT_StorageReliabilityCounter", rename_all = "PascalCase")]
struct WmiStorageReliabilityCounter {
    temperature: Option<u64>,
    power_on_hours: Option<u64>,
    wear: Option<u8>,
    read_errors_total: Option<u64>,
    write_errors_total: Option<u64>,
}

#[cfg(windows)]
#[derive(Deserialize)]
#[serde(rename = "MSFT_PhysicalDiskToStorageReliabilityCounter")]
struct PhysicalDiskReliabilityAssociation {}

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

    let mut pnp_ids = Vec::new();
    // Query physical drives
    if let Ok(drives) = wmi.raw_query::<WmiDiskDrive>(
        "SELECT DeviceID, PNPDeviceID, Model, SerialNumber, FirmwareRevision, MediaType, Status FROM Win32_DiskDrive"
    ) {
        for drive in drives {
            pnp_ids.push(drive.pnp_device_id.unwrap_or_default());
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
                for pred in predictions {
                    let Some((index, predicted)) = pred
                        .instance_name
                        .as_deref()
                        .and_then(|name| matching_prediction(name, &pnp_ids))
                        .zip(pred.predict_failure)
                    else {
                        continue;
                    };
                    let status = if predicted {
                        DiskHealthStatus::Critical
                    } else {
                        DiskHealthStatus::Healthy
                    };
                    let conflict = merge_health(
                        &mut data.drives[index],
                        status,
                        "MSStorageDriver_FailurePredictStatus",
                    );
                    if conflict {
                        data.health_status = Observation::contradictory(
                            "Windows storage providers",
                            "Providers disagree on disk health; the reported fault is retained",
                        );
                    } else if data.health_status.status != ObservationStatus::Contradictory {
                        data.health_status = Observation::available(
                            "Windows storage providers; uniquely matched device identity",
                        );
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

    let rows = match connection.raw_query::<WmiPhysicalDisk>(
        "SELECT __Path, SerialNumber, HealthStatus, MediaType, BusType, FirmwareVersion FROM MSFT_PhysicalDisk",
    ) {
        Ok(rows) => rows,
        Err(error) => {
            let status = windows_storage_error("MSFT_PhysicalDisk", &error.to_string());
            return (status.clone(), status);
        }
    };
    let serials: Vec<_> = rows
        .iter()
        .map(|row| row.serial_number.as_deref())
        .collect();
    let mut health_status = Observation::unavailable(
        "MSFT_PhysicalDisk.HealthStatus",
        "No health value could be uniquely matched to the disk inventory",
    );
    let mut reliability_status = Observation::unavailable(
        "MSFT_StorageReliabilityCounter",
        "No associated reliability fields were returned for an identified disk",
    );
    for row in &rows {
        let Some(index) = matching_serial(&data.drives, row.serial_number.as_deref(), &serials)
        else {
            continue;
        };
        let drive = &mut data.drives[index];
        let status = match row.health_status {
            Some(0) => DiskHealthStatus::Healthy,
            Some(1) => DiskHealthStatus::Warning,
            Some(2) => DiskHealthStatus::Critical,
            _ => DiskHealthStatus::Unknown,
        };
        if status != DiskHealthStatus::Unknown {
            if merge_health(drive, status, "MSFT_PhysicalDisk.HealthStatus") {
                health_status = Observation::contradictory(
                    "Windows storage providers",
                    "Providers disagree on disk health; the reported fault is retained",
                );
            } else if health_status.status != ObservationStatus::Contradictory {
                health_status = Observation::available(
                    "MSFT_PhysicalDisk.HealthStatus; unique serial identity",
                );
            }
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
        if let Some(firmware) = row
            .firmware_version
            .as_deref()
            .filter(|v| !v.trim().is_empty())
        {
            drive.firmware = Some(firmware.trim().to_owned());
        }
        // DeviceId may name a subsystem disk or an OS disk number. Only the
        // documented physical-disk association establishes which one it is.
        match connection
            .associators::<WmiStorageReliabilityCounter, PhysicalDiskReliabilityAssociation>(
                &row.path,
            ) {
            Ok(counters) if counters.len() == 1 => {
                let counter = &counters[0];
                drive.temperature_celsius = counter.temperature.map(|v| v as f64);
                drive.power_on_hours = counter.power_on_hours;
                drive.wear_percent = counter.wear;
                drive.read_errors_total = counter.read_errors_total;
                drive.write_errors_total = counter.write_errors_total;
                if drive.temperature_celsius.is_some()
                    || drive.power_on_hours.is_some()
                    || drive.wear_percent.is_some()
                    || drive.read_errors_total.is_some()
                    || drive.write_errors_total.is_some()
                {
                    reliability_status =
                        Observation::available("MSFT_PhysicalDiskToStorageReliabilityCounter");
                }
            }
            Ok(_) => {}
            Err(error) if !reliability_status.is_available() => {
                reliability_status = windows_storage_error(
                    "MSFT_PhysicalDiskToStorageReliabilityCounter",
                    &error.to_string(),
                );
            }
            Err(_) => {}
        }
    }

    (health_status, reliability_status)
}

#[cfg(any(windows, test))]
fn matching_serial(
    drives: &[DriveHealth],
    serial: Option<&str>,
    provider_serials: &[Option<&str>],
) -> Option<usize> {
    let serial = serial?.trim();
    if serial.is_empty()
        || provider_serials
            .iter()
            .filter(|s| s.is_some_and(|s| s.trim() == serial))
            .count()
            != 1
    {
        return None;
    }
    let mut matches = drives
        .iter()
        .enumerate()
        .filter(|(_, d)| d.serial.as_deref().is_some_and(|s| s.trim() == serial));
    let index = matches.next()?.0;
    matches.next().is_none().then_some(index)
}

#[cfg(any(windows, test))]
fn matching_prediction(instance: &str, pnp_ids: &[String]) -> Option<usize> {
    // Storage WMI uses the device instance path, optionally followed by an
    // instance counter. Never search for a PhysicalDrive substring or match an
    // empty identifier. More than one candidate is an incomplete observation.
    let instance = instance.to_ascii_lowercase();
    let mut matches = pnp_ids.iter().enumerate().filter(|(_, id)| {
        if id.is_empty() {
            return false;
        }
        let id = id.to_ascii_lowercase();
        instance == id
            || instance
                .strip_prefix(&id)
                .and_then(|s| s.strip_prefix('_'))
                .is_some_and(|suffix| {
                    !suffix.is_empty() && suffix.bytes().all(|b| b.is_ascii_digit())
                })
    });
    let index = matches.next()?.0;
    matches.next().is_none().then_some(index)
}

/// Merge fresh provider observations without allowing a healthy response to
/// erase another provider's fault. Unknown never overwrites measured health.
fn merge_health(drive: &mut DriveHealth, incoming: DiskHealthStatus, source: &str) -> bool {
    let rank = |s: &DiskHealthStatus| match s {
        DiskHealthStatus::Unknown => 0,
        DiskHealthStatus::Healthy => 1,
        DiskHealthStatus::Warning => 2,
        DiskHealthStatus::Critical => 3,
    };
    if incoming == DiskHealthStatus::Unknown {
        return false;
    }
    let conflict = (drive.health_status == DiskHealthStatus::Healthy && rank(&incoming) >= 2)
        || (incoming == DiskHealthStatus::Healthy && rank(&drive.health_status) >= 2);
    if conflict {
        drive.health_source = format!(
            "{}; {source} (conflicting health observations)",
            drive.health_source
        );
    } else if rank(&incoming) >= rank(&drive.health_status) {
        drive.health_source = source.into();
    }
    if rank(&incoming) > rank(&drive.health_status) {
        drive.health_status = incoming;
    }
    conflict
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
    if let (Some(expected), Some(actual)) = (
        drive.serial.as_deref().filter(|s| !s.trim().is_empty()),
        text("/serial_number"),
    ) {
        if expected.trim() != actual.trim() {
            return Observation::contradictory("smartctl JSON", "The device identity changed between inventory and probe; readings were not attached to the previous device");
        }
    }
    let status = if passed == Some(false) || exit & 0x18 != 0 {
        DiskHealthStatus::Critical
    } else if exit & 0xe0 != 0 {
        DiskHealthStatus::Warning
    } else if passed == Some(true) && exit & 7 == 0 {
        DiskHealthStatus::Healthy
    } else {
        DiskHealthStatus::Unknown
    };
    let conflict = merge_health(drive, status, "smartctl JSON");
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
    if conflict {
        return Observation::contradictory("smartctl JSON and platform storage health", "Providers disagree on disk health; the reported fault and available measurements are retained");
    }
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
    fn provider_serials_must_be_unique_in_both_inventories() {
        let mut drives = vec![
            empty_drive("disk9".into(), "Identical model".into(), MediaType::Ssd),
            empty_drive("disk0".into(), "Identical model".into(), MediaType::Ssd),
        ];
        drives[0].serial = Some(" A ".into());
        drives[1].serial = Some("B".into());
        assert_eq!(
            matching_serial(&drives, Some("B"), &[Some("B"), Some("A")]),
            Some(1)
        );
        assert_eq!(
            matching_serial(&drives, Some("A"), &[Some("A"), Some("A")]),
            None
        );
        assert_eq!(matching_serial(&drives, Some(" "), &[Some(" ")]), None);
        assert_eq!(matching_serial(&drives, None, &[None]), None);
        drives[1].serial = Some("A".into());
        assert_eq!(matching_serial(&drives, Some("A"), &[Some("A")]), None);
    }

    #[test]
    fn failure_prediction_uses_unique_pnp_identity_and_counter_boundaries() {
        let ids = vec![
            String::new(),
            r"SCSI\DISK&VEN_FIXTURE\4&ABC".into(),
            r"SCSI\DISK&VEN_FIXTURE\4&ABCD".into(),
        ];
        assert_eq!(
            matching_prediction(r"scsi\disk&ven_fixture\4&abc_0", &ids),
            Some(1)
        );
        assert_eq!(
            matching_prediction(r"SCSI\DISK&VEN_FIXTURE\4&ABCD_12", &ids),
            Some(2)
        );
        assert_eq!(matching_prediction(r"\\.\PHYSICALDRIVE0", &ids), None);
        assert_eq!(
            matching_prediction(r"SCSI\DISK&VEN_FIXTURE\4&ABC_other", &ids),
            None
        );
        assert_eq!(matching_prediction("", &ids), None);
        assert_eq!(
            matching_prediction("disk_0", &["disk".into(), "disk".into()]),
            None
        );
        assert_eq!(
            matching_prediction("disk_0", &["disk".into(), "disk_0".into()]),
            None
        );
    }

    #[test]
    fn healthy_smart_does_not_erase_another_providers_fault() {
        let mut drive = empty_drive("fixture".into(), "fixture".into(), MediaType::Unknown);
        merge_health(&mut drive, DiskHealthStatus::Critical, "platform provider");
        let result = apply_smart_json(&mut drive, br#"{"smartctl":{"exit_status":0},"temperature":{"current":41},"smart_status":{"passed":true}}"#, 0);
        assert_eq!(result.status, ObservationStatus::Contradictory);
        assert_eq!(drive.health_status, DiskHealthStatus::Critical);
        assert_eq!(drive.temperature_celsius, Some(41.0));
        assert!(drive.health_source.contains("platform provider"));
        assert!(drive.health_source.contains("smartctl JSON"));
        assert!(!merge_health(
            &mut drive,
            DiskHealthStatus::Unknown,
            "unavailable provider"
        ));
        assert_eq!(drive.health_status, DiskHealthStatus::Critical);
    }

    #[test]
    fn replaced_device_smart_result_is_not_attached_to_old_identity() {
        let mut drive = empty_drive("fixture".into(), "fixture".into(), MediaType::Unknown);
        drive.serial = Some("old device".into());
        let result = apply_smart_json(&mut drive, br#"{"smartctl":{"exit_status":8},"serial_number":"replacement","temperature":{"current":41},"smart_status":{"passed":false}}"#, 8);
        assert_eq!(result.status, ObservationStatus::Contradictory);
        assert_eq!(drive.serial.as_deref(), Some("old device"));
        assert_eq!(drive.health_status, DiskHealthStatus::Unknown);
        assert_eq!(drive.temperature_celsius, None);
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
