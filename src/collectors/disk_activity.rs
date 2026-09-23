//! Physical-device activity, independent of SMART/reliability polling.
use crate::observation::Observation;
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    time::{Duration, Instant},
};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DiskActivity {
    pub devices: Vec<DeviceActivity>,
    pub observation: Observation,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceActivity {
    pub device_id: String,
    pub identity: String,
    pub read_bytes_per_sec: Option<f64>,
    pub write_bytes_per_sec: Option<f64>,
    pub read_latency_ms: Option<f64>,
    pub write_latency_ms: Option<f64>,
    pub queue_depth: Option<u64>,
    pub observation: Observation,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CounterFrame {
    pub devices: Vec<DeviceCounters>,
    pub observation: Observation,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DeviceCounters {
    pub device_id: String,
    pub identity: String,
    pub read_bytes: u64,
    pub write_bytes: u64,
    pub reads: Option<u64>,
    pub writes: Option<u64>,
    pub read_time_ns: Option<u64>,
    pub write_time_ns: Option<u64>,
    pub queue_depth: Option<u64>,
}

#[derive(Default)]
pub struct DiskSampler {
    previous: HashMap<String, (DeviceCounters, Instant)>,
}

impl DiskSampler {
    pub fn sample(&mut self, frame: CounterFrame, now: Instant) -> DiskActivity {
        self.previous
            .retain(|id, _| frame.devices.iter().any(|d| &d.identity == id));
        let devices = frame
            .devices
            .into_iter()
            .map(|current| {
                let old = self
                    .previous
                    .insert(current.identity.clone(), (current.clone(), now));
                let valid = old.as_ref().and_then(|(old, at)| {
                    let elapsed = now.checked_duration_since(*at)?;
                    if elapsed.is_zero() || elapsed > Duration::from_secs(10) {
                        return None;
                    }
                    Some((
                        current.read_bytes.checked_sub(old.read_bytes)? as f64
                            / elapsed.as_secs_f64(),
                        current.write_bytes.checked_sub(old.write_bytes)? as f64
                            / elapsed.as_secs_f64(),
                    ))
                });
                let latency = |read: bool| -> Option<f64> {
                    valid?;
                    let (old, _) = old.as_ref()?;
                    let (time, count, old_time, old_count) = if read {
                        (
                            current.read_time_ns?,
                            current.reads?,
                            old.read_time_ns?,
                            old.reads?,
                        )
                    } else {
                        (
                            current.write_time_ns?,
                            current.writes?,
                            old.write_time_ns?,
                            old.writes?,
                        )
                    };
                    let operations = count.checked_sub(old_count)?;
                    (operations > 0)
                        .then(|| {
                            time.checked_sub(old_time)
                                .map(|time| time as f64 / operations as f64 / 1_000_000.0)
                        })
                        .flatten()
                };
                DeviceActivity {
                    device_id: current.device_id.clone(),
                    identity: current.identity.clone(),
                    read_bytes_per_sec: valid.map(|v| v.0),
                    write_bytes_per_sec: valid.map(|v| v.1),
                    read_latency_ms: latency(true),
                    write_latency_ms: latency(false),
                    queue_depth: current.queue_depth,
                    observation: if valid.is_some() {
                        frame.observation.clone()
                    } else {
                        Observation::unavailable(
                            frame.observation.source.clone(),
                            "Warming up after discovery, counter reset, replacement, or resume",
                        )
                    },
                }
            })
            .collect();
        DiskActivity {
            devices,
            observation: frame.observation,
        }
    }
}

impl DiskActivity {
    pub fn totals(&self) -> Option<(f64, f64)> {
        if self.devices.is_empty() || !self.observation.is_available() {
            return None;
        }
        self.devices.iter().try_fold((0.0, 0.0), |sum, d| {
            Some((
                sum.0 + d.read_bytes_per_sec?,
                sum.1 + d.write_bytes_per_sec?,
            ))
        })
    }
    pub fn apply_legacy_io(&self, health: &mut super::disk_health::DiskHealthData) {
        for drive in &mut health.drives {
            drive.io_stats = self
                .devices
                .iter()
                .find(|d| d.device_id == drive.device_id)
                .and_then(|d| {
                    Some(super::disk_health::DiskIoStats {
                        read_bytes_per_sec: d.read_bytes_per_sec? as u64,
                        write_bytes_per_sec: d.write_bytes_per_sec? as u64,
                        queue_depth: d.queue_depth.unwrap_or(0) as f64,
                        avg_read_latency_ms: d.read_latency_ms.unwrap_or(0.0),
                        avg_write_latency_ms: d.write_latency_ms.unwrap_or(0.0),
                    })
                });
        }
    }
}

pub fn collect() -> CounterFrame {
    #[cfg(windows)]
    {
        windows::collect()
    }
    #[cfg(target_os = "linux")]
    {
        collect_linux(std::path::Path::new("/sys/block"))
    }
    #[cfg(target_os = "macos")]
    {
        super::macos::disk_counters()
    }
    #[cfg(not(any(windows, target_os = "linux", target_os = "macos")))]
    {
        CounterFrame {
            observation: Observation::unsupported(
                "disk activity",
                "No counter provider for this platform",
            ),
            ..Default::default()
        }
    }
}

#[cfg(any(target_os = "linux", test))]
fn parse_linux_stat(device_id: String, identity: String, stat: &str) -> Option<DeviceCounters> {
    let fields = stat
        .split_whitespace()
        .map(str::parse::<u64>)
        .collect::<Result<Vec<_>, _>>()
        .ok()?;
    if fields.len() < 11 {
        return None;
    }
    Some(DeviceCounters {
        device_id,
        identity,
        read_bytes: fields[2].checked_mul(512)?,
        write_bytes: fields[6].checked_mul(512)?,
        reads: Some(fields[0]),
        writes: Some(fields[4]),
        read_time_ns: fields[3].checked_mul(1_000_000),
        write_time_ns: fields[7].checked_mul(1_000_000),
        queue_depth: Some(fields[8]),
    })
}

#[cfg(any(target_os = "linux", test))]
pub fn collect_linux(root: &std::path::Path) -> CounterFrame {
    let mut frame = CounterFrame {
        observation: Observation::available(
            "Linux physical block-device counters; 512-byte sectors, nanosecond normalization",
        ),
        ..Default::default()
    };
    let Ok(entries) = std::fs::read_dir(root) else {
        frame.observation =
            Observation::unavailable("Linux sysfs", "Block-device counters are not readable");
        return frame;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().into_owned();
        // Aggregate leaf physical devices only. Never add dm/md logical layers,
        // partitions, or memory-backed devices to their backing devices again.
        if name.starts_with("loop")
            || name.starts_with("ram")
            || name.starts_with("zram")
            || path.join("partition").exists()
            || !path.join("device").exists()
            || std::fs::read_dir(path.join("slaves"))
                .is_ok_and(|mut entries| entries.next().is_some())
        {
            continue;
        }
        let identity = ["diskseq", "device/wwid", "device/serial"]
            .iter()
            .filter_map(|p| std::fs::read_to_string(path.join(p)).ok())
            .collect::<Vec<_>>()
            .join(":");
        if let Ok(stat) = std::fs::read_to_string(path.join("stat")) {
            if let Some(counters) =
                parse_linux_stat(format!("/dev/{name}"), format!("{name}:{identity}"), &stat)
            {
                frame.devices.push(counters);
            }
        }
    }
    if frame.devices.is_empty() {
        frame.observation = Observation::unavailable(
            "Linux sysfs",
            "No readable physical-device counters (container or permissions may restrict sysfs)",
        );
    }
    frame
}

#[cfg(windows)]
mod windows {
    use super::*;
    use winapi::um::{
        fileapi::{CreateFileW, QueryDosDeviceW, OPEN_EXISTING},
        handleapi::{CloseHandle, INVALID_HANDLE_VALUE},
        ioapiset::DeviceIoControl,
        winioctl::{DISK_PERFORMANCE, IOCTL_DISK_PERFORMANCE},
        winnt::{FILE_SHARE_READ, FILE_SHARE_WRITE},
    };
    pub fn collect() -> CounterFrame {
        let mut names = vec![0u16; 65536];
        let count =
            unsafe { QueryDosDeviceW(std::ptr::null(), names.as_mut_ptr(), names.len() as u32) };
        let mut frame = CounterFrame {
            observation: Observation::available(
                "IOCTL_DISK_PERFORMANCE; byte counters and 100 ns service times",
            ),
            ..Default::default()
        };
        if count == 0 {
            frame.observation = Observation::error(
                "QueryDosDevice",
                std::io::Error::last_os_error().to_string(),
            );
            return frame;
        }
        let mut denied = false;
        for name in names[..count as usize]
            .split(|n| *n == 0)
            .filter(|n| !n.is_empty())
        {
            let name = String::from_utf16_lossy(name);
            let Some(index) = name
                .strip_prefix("PhysicalDrive")
                .and_then(|i| i.parse::<u32>().ok())
            else {
                continue;
            };
            let device_id = format!(r"\\.\PhysicalDrive{index}");
            let path: Vec<_> = device_id.encode_utf16().chain(Some(0)).collect();
            let handle = unsafe {
                CreateFileW(
                    path.as_ptr(),
                    0,
                    FILE_SHARE_READ | FILE_SHARE_WRITE,
                    std::ptr::null_mut(),
                    OPEN_EXISTING,
                    0,
                    std::ptr::null_mut(),
                )
            };
            if handle == INVALID_HANDLE_VALUE {
                denied |= std::io::Error::last_os_error().raw_os_error() == Some(5);
                continue;
            }
            let mut perf: DISK_PERFORMANCE = unsafe { std::mem::zeroed() };
            let mut returned = 0;
            let success = unsafe {
                DeviceIoControl(
                    handle,
                    IOCTL_DISK_PERFORMANCE,
                    std::ptr::null_mut(),
                    0,
                    (&mut perf as *mut DISK_PERFORMANCE).cast(),
                    std::mem::size_of_val(&perf) as u32,
                    &mut returned,
                    std::ptr::null_mut(),
                )
            };
            let error = std::io::Error::last_os_error();
            let identity = device_identity(handle, &name);
            unsafe {
                CloseHandle(handle);
            }
            if success == 0 {
                denied |= error.raw_os_error() == Some(5);
                continue;
            }
            if returned < std::mem::size_of_val(&perf) as u32 {
                continue;
            }
            let read = unsafe { *perf.BytesRead.QuadPart() };
            let write = unsafe { *perf.BytesWritten.QuadPart() };
            if read < 0 || write < 0 {
                continue;
            }
            // The kernel storage-manager device number is independent of
            // provider enumeration order. A counter rollback resets its sample.
            let manager = String::from_utf16_lossy(&perf.StorageManagerName);
            frame.devices.push(DeviceCounters {
                identity: format!(
                    "{device_id}:{manager}:{}:{identity}",
                    perf.StorageDeviceNumber
                ),
                device_id,
                read_bytes: read as u64,
                write_bytes: write as u64,
                reads: Some(perf.ReadCount as u64),
                writes: Some(perf.WriteCount as u64),
                read_time_ns: u64::try_from(unsafe { *perf.ReadTime.QuadPart() })
                    .ok()
                    .and_then(|v| v.checked_mul(100)),
                write_time_ns: u64::try_from(unsafe { *perf.WriteTime.QuadPart() })
                    .ok()
                    .and_then(|v| v.checked_mul(100)),
                queue_depth: Some(perf.QueueDepth as u64),
            });
        }
        if frame.devices.is_empty() {
            frame.observation = if denied {
                Observation::permission_denied(
                    "IOCTL_DISK_PERFORMANCE",
                    "Read-only disk counters require additional permission on this system",
                )
            } else {
                Observation::unavailable(
                    "IOCTL_DISK_PERFORMANCE",
                    "The device driver did not expose disk performance counters",
                )
            };
        }
        frame
    }

    fn device_identity(handle: winapi::um::winnt::HANDLE, name: &str) -> String {
        use sha2::{Digest, Sha256};
        use winapi::um::winioctl::{
            PropertyStandardQuery, StorageDeviceProperty, IOCTL_STORAGE_QUERY_PROPERTY,
            STORAGE_PROPERTY_QUERY,
        };
        let mut query = STORAGE_PROPERTY_QUERY {
            PropertyId: StorageDeviceProperty,
            QueryType: PropertyStandardQuery,
            AdditionalParameters: [0],
        };
        let mut descriptor = [0u8; 4096];
        let mut returned = 0;
        let ok = unsafe {
            DeviceIoControl(
                handle,
                IOCTL_STORAGE_QUERY_PROPERTY,
                (&mut query as *mut STORAGE_PROPERTY_QUERY).cast(),
                std::mem::size_of_val(&query) as u32,
                descriptor.as_mut_ptr().cast(),
                descriptor.len() as u32,
                &mut returned,
                std::ptr::null_mut(),
            )
        };
        let mut identity = Vec::new();
        // STORAGE_DEVICE_DESCRIPTOR.SerialNumberOffset is the DWORD at byte
        // 24. All descriptor string offsets are relative to this bounded buffer.
        if ok != 0 && returned >= 36 && returned as usize <= descriptor.len() {
            let offset = u32::from_le_bytes(descriptor[24..28].try_into().unwrap()) as usize;
            if offset >= 36 && offset < returned as usize {
                identity.extend(
                    descriptor[offset..returned as usize]
                        .iter()
                        .copied()
                        .take_while(|b| *b != 0),
                );
            }
        }
        let name: Vec<_> = name.encode_utf16().chain(Some(0)).collect();
        let mut target = [0u16; 1024];
        let count =
            unsafe { QueryDosDeviceW(name.as_ptr(), target.as_mut_ptr(), target.len() as u32) }
                as usize;
        if count <= target.len() {
            for character in &target[..count] {
                identity.extend(character.to_le_bytes());
            }
        }
        format!("{:x}", Sha256::digest(identity))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn frame(identity: &str, bytes: u64, ops: u64, time_ns: u64) -> CounterFrame {
        CounterFrame {
            devices: vec![DeviceCounters {
                device_id: "disk".into(),
                identity: identity.into(),
                read_bytes: bytes,
                reads: Some(ops),
                read_time_ns: Some(time_ns),
                ..Default::default()
            }],
            observation: Observation::available("fixture"),
        }
    }
    #[test]
    fn irregular_windows_units_reset_replacement_and_resume() {
        let mut sampler = DiskSampler::default();
        let now = Instant::now();
        assert_eq!(
            sampler.sample(frame("a", 100, 10, 100_000), now).totals(),
            None
        );
        let data = sampler.sample(
            frame("a", 600, 12, 5_100_000),
            now + Duration::from_millis(250),
        );
        assert_eq!(data.totals(), Some((2000.0, 0.0)));
        assert_eq!(data.devices[0].read_latency_ms, Some(2.5));
        assert_eq!(
            sampler
                .sample(frame("a", 1, 1, 1), now + Duration::from_secs(1))
                .totals(),
            None
        );
        assert_eq!(
            sampler
                .sample(frame("b", 1000, 50, 500), now + Duration::from_secs(2))
                .totals(),
            None
        );
        assert_eq!(
            sampler
                .sample(frame("b", 2000, 60, 600), now + Duration::from_secs(30))
                .totals(),
            None
        );
    }
    #[test]
    fn linux_units_are_sector_bytes_and_milliseconds() {
        let counters = parse_linux_stat(
            "sda".into(),
            "fixture".into(),
            "10 0 100 25 20 0 200 80 2 4 6",
        )
        .unwrap();
        assert_eq!(counters.read_bytes, 51_200);
        assert_eq!(counters.write_bytes, 102_400);
        assert_eq!(counters.read_time_ns, Some(25_000_000));
        assert_eq!(counters.queue_depth, Some(2));
        assert!(parse_linux_stat("sda".into(), "fixture".into(), "malformed").is_none());
    }
    #[test]
    fn linux_aggregation_excludes_stacked_devices() {
        let root = tempfile::tempdir().unwrap();
        for name in ["sda", "dm-0", "loop0"] {
            let path = root.path().join(name);
            std::fs::create_dir_all(path.join("device")).unwrap();
            std::fs::create_dir_all(path.join("slaves")).unwrap();
            std::fs::write(path.join("stat"), "1 0 100 2 1 0 100 2 0 0 0").unwrap();
        }
        std::fs::write(root.path().join("dm-0/slaves/sda"), "").unwrap();
        let frame = collect_linux(root.path());
        assert_eq!(frame.devices.len(), 1);
        assert_eq!(frame.devices[0].device_id, "/dev/sda");
    }
}
