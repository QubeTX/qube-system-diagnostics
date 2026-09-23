//! Narrow subprocess boundary for probes that cannot reliably cancel a native
//! call. Workers accept an enumerated read-only topic, never a command string.
use super::*;
use serde::{Deserialize, Serialize};
use std::{
    path::PathBuf,
    sync::atomic::AtomicBool,
    time::{Duration, Instant},
};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "topic", content = "data", rename_all = "snake_case")]
pub enum ProbeData {
    Activity(disk_activity::DiskActivity),
    Static {
        system: system_info::SystemInfoData,
        memory: memory::MemoryData,
        displays: display::DisplayData,
        network: network::NetworkData,
    },
    Slow {
        disk: disk::DiskData,
        gpu: gpu::GpuData,
        thermals: thermals::ThermalData,
        warnings: Vec<DiagnosticWarning>,
    },
    Connections(network_diag::NetworkDiagData),
    Diagnostics(network_diag::NetworkDiagData, Vec<DiagnosticWarning>),
    Health(disk_health::DiskHealthData, Vec<DiagnosticWarning>),
    Drivers(drivers::DriverData),
}

#[derive(Serialize, Deserialize)]
struct Envelope {
    version: String,
    captured_unix_ms: u64,
    interval_ms: u64,
    data: ProbeData,
}

#[derive(Default)]
struct WorkerSampler {
    disk: disk_activity::DiskSampler,
    previous: Option<(Instant, u64)>,
}
impl WorkerSampler {
    fn finish(&mut self, data: ProbeData, now: Instant, wall: u64) -> Envelope {
        let interval = self
            .previous
            .replace((now, wall))
            .map(|(previous, _)| now.saturating_duration_since(previous))
            .unwrap_or_default();
        Envelope {
            version: env!("CARGO_PKG_VERSION").into(),
            captured_unix_ms: wall,
            interval_ms: interval.as_millis().try_into().unwrap_or(u64::MAX),
            data,
        }
    }
    fn activity(
        &mut self,
        frame: disk_activity::CounterFrame,
        now: Instant,
        wall: u64,
    ) -> Envelope {
        // Some OS monotonic clocks exclude suspended time. Use wall time only
        // to invalidate a baseline, never as the rate denominator.
        if self
            .previous
            .is_some_and(|(_, last)| wall < last || wall.saturating_sub(last) > 10_000)
        {
            self.disk = Default::default();
        }
        let data = self.disk.sample(frame, now);
        self.finish(ProbeData::Activity(data), now, wall)
    }
    fn collect(&mut self, topic: crate::cli::CollectorTopic) -> Envelope {
        if matches!(topic, crate::cli::CollectorTopic::Activity) {
            let frame = disk_activity::collect();
            return self.activity(frame, Instant::now(), sampling::unix_ms());
        }
        let data = collect_local(topic);
        self.finish(data, Instant::now(), sampling::unix_ms())
    }
}

fn collect_local(topic: crate::cli::CollectorTopic) -> ProbeData {
    use crate::cli::CollectorTopic;
    match topic {
        CollectorTopic::Activity => unreachable!("activity uses the worker's capture baseline"),
        CollectorTopic::Static => {
            let mut snapshot = SystemSnapshot::default();
            snapshot.refresh_static();
            ProbeData::Static {
                system: snapshot.system,
                memory: snapshot.memory,
                displays: snapshot.displays,
                network: snapshot.network,
            }
        }
        CollectorTopic::Slow => {
            let mut snapshot = SystemSnapshot::default();
            snapshot.refresh_slow();
            ProbeData::Slow {
                disk: snapshot.disk,
                gpu: snapshot.gpu,
                thermals: snapshot.thermals,
                warnings: snapshot.warnings,
            }
        }
        CollectorTopic::Connections => {
            let mut data = network_diag::NetworkDiagData::default();
            network_diag::refresh_connections(&mut data);
            ProbeData::Connections(data)
        }
        CollectorTopic::Diagnostics => {
            let (data, warnings) = network_diag::collect_connectivity();
            ProbeData::Diagnostics(data, warnings)
        }
        CollectorTopic::Health => {
            let (data, warnings) = disk_health::collect();
            ProbeData::Health(data, warnings)
        }
        CollectorTopic::Drivers => ProbeData::Drivers(drivers::collect()),
    }
}

pub fn print_worker(topic: crate::cli::CollectorTopic) -> crate::error::Result<()> {
    serde_json::to_writer(
        std::io::stdout().lock(),
        &WorkerSampler::default().collect(topic),
    )
    .map_err(|error| crate::error::AppError::platform(error.to_string()))
}

/// Requests are a single byte plus newline, with no paths or commands supplied
/// through the protocol. Parent ownership and timeout cover every native call.
pub fn serve(topic: crate::cli::CollectorTopic) -> crate::error::Result<()> {
    use std::io::{BufRead, Read, Write};
    let mut input = std::io::stdin().lock();
    let mut output = std::io::stdout().lock();
    let mut sampler = WorkerSampler::default();
    loop {
        let mut request = String::new();
        if input.by_ref().take(3).read_line(&mut request)? == 0 {
            return Ok(());
        }
        match request.as_str() {
            "r\n" => {
                super::provider_cache::invalidate();
                sampler = WorkerSampler::default();
            }
            "s\n" => {}
            _ => {
                return Err(crate::error::AppError::platform(
                    "Invalid collector request",
                ))
            }
        }
        let bytes = serde_json::to_vec(&sampler.collect(topic))
            .map_err(|e| crate::error::AppError::platform(e.to_string()))?;
        if bytes.len() as u64 > command::MAX_OUTPUT_BYTES {
            return Err(crate::error::AppError::platform(
                "Collector response exceeds limit",
            ));
        }
        output.write_all(b"SD4\0")?;
        output.write_all(&(bytes.len() as u32).to_le_bytes())?;
        output.write_all(&bytes)?;
        output.flush()?;
    }
}

#[derive(Default)]
pub struct Session {
    worker: Option<command::WorkerProcess>,
}
impl Session {
    pub fn collect(
        &mut self,
        topic: crate::cli::CollectorTopic,
        cancelled: &AtomicBool,
        reset: bool,
    ) -> Result<(ProbeData, u64, Duration), String> {
        let name = topic_name(topic);
        if self.worker.is_none() {
            let executable =
                executable().ok_or("The matching SD-300 CLI collector companion is missing")?;
            self.worker = Some(
                command::WorkerProcess::spawn(executable.as_os_str(), name)
                    .map_err(|e| e.to_string())?,
            );
        }
        let result = self
            .worker
            .as_mut()
            .unwrap()
            .request(reset, Duration::from_secs(25), cancelled)
            .map_err(|e| format!("{name}: {e}"))
            .and_then(|bytes| decode(topic, &bytes));
        // Only sub-minute lanes benefit from retaining a native process. The
        // parent already keeps inventory/health results through their cadence;
        // leave no idle address space or OS helper handles behind between them.
        if result.is_err()
            || matches!(
                topic,
                crate::cli::CollectorTopic::Static
                    | crate::cli::CollectorTopic::Health
                    | crate::cli::CollectorTopic::Drivers
            )
        {
            self.worker.take();
        }
        result
    }
}
fn topic_name(topic: crate::cli::CollectorTopic) -> &'static str {
    match topic {
        crate::cli::CollectorTopic::Activity => "activity",
        crate::cli::CollectorTopic::Static => "static",
        crate::cli::CollectorTopic::Slow => "slow",
        crate::cli::CollectorTopic::Connections => "connections",
        crate::cli::CollectorTopic::Diagnostics => "diagnostics",
        crate::cli::CollectorTopic::Health => "health",
        crate::cli::CollectorTopic::Drivers => "drivers",
    }
}

pub fn executable() -> Option<PathBuf> {
    let current = std::env::current_exe().ok()?;
    let name = if cfg!(windows) { "sd300.exe" } else { "sd300" };
    if current.file_name().is_some_and(|file| file == name) {
        return Some(current);
    }
    crate::gui::locate_update_cli()
}

fn decode(
    topic: crate::cli::CollectorTopic,
    bytes: &[u8],
) -> Result<(ProbeData, u64, Duration), String> {
    let name = topic_name(topic);
    let envelope: Envelope =
        serde_json::from_slice(bytes).map_err(|e| format!("Invalid {name} worker result: {e}"))?;
    if envelope.version != env!("CARGO_PKG_VERSION") {
        return Err("Collector companion version does not match this monitor".into());
    }
    let correct = matches!(
        (topic, &envelope.data),
        (crate::cli::CollectorTopic::Activity, ProbeData::Activity(_))
            | (crate::cli::CollectorTopic::Static, ProbeData::Static { .. })
            | (crate::cli::CollectorTopic::Slow, ProbeData::Slow { .. })
            | (
                crate::cli::CollectorTopic::Connections,
                ProbeData::Connections(_)
            )
            | (
                crate::cli::CollectorTopic::Diagnostics,
                ProbeData::Diagnostics(..)
            )
            | (crate::cli::CollectorTopic::Health, ProbeData::Health(..))
            | (crate::cli::CollectorTopic::Drivers, ProbeData::Drivers(_))
    );
    if !correct {
        return Err("Collector companion returned the wrong topic".into());
    }
    Ok((
        envelope.data,
        envelope.captured_unix_ms,
        Duration::from_millis(envelope.interval_ms),
    ))
}

impl ProbeData {
    pub fn apply(self, snapshot: &mut SystemSnapshot) {
        let merge =
            |snapshot: &mut SystemSnapshot, source: &str, warnings: Vec<DiagnosticWarning>| {
                snapshot.warnings.retain(|w| w.source != source);
                snapshot.warnings.extend(warnings);
            };
        match self {
            Self::Activity(data) => snapshot.disk_activity = data,
            Self::Static {
                system,
                memory,
                displays,
                network,
            } => {
                snapshot.system = system;
                snapshot.memory.modules = memory.modules;
                snapshot.memory.module_status = memory.module_status;
                snapshot.displays = displays;
                snapshot.network.adapters = network.adapters;
                snapshot.network.adapter_status = network.adapter_status;
            }
            Self::Slow {
                disk,
                gpu,
                thermals,
                warnings,
            } => {
                snapshot.disk = disk;
                snapshot.gpu = gpu;
                snapshot.thermals = thermals;
                merge(snapshot, "Thermals", warnings);
            }
            Self::Connections(data) => {
                snapshot.network_diag.active_connections = data.active_connections;
                snapshot.network_diag.listening_ports = data.listening_ports;
                snapshot.network_diag.connections_observation = data.connections_observation;
            }
            Self::Diagnostics(data, warnings) => {
                snapshot.network_diag.gateway = data.gateway;
                snapshot.network_diag.dns = data.dns;
                snapshot.network_diag.internet = data.internet;
                merge(snapshot, "Network", warnings);
            }
            Self::Health(data, warnings) => {
                snapshot.disk_health = data;
                merge(snapshot, "Disk Health", warnings);
            }
            Self::Drivers(data) => {
                snapshot.drivers = data;
                snapshot.warnings.retain(|w| w.source != "Drivers");
                if let drivers::DriverScanStatus::ScanFailed(message) =
                    &snapshot.drivers.scan_status
                {
                    snapshot.warnings.push(DiagnosticWarning {
                        source: "Drivers".into(),
                        message: message.clone(),
                        severity: WarningSeverity::Warning,
                    });
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{cli::CollectorTopic, observation::Observation};

    fn frame(bytes: u64) -> disk_activity::CounterFrame {
        disk_activity::CounterFrame {
            devices: vec![disk_activity::DeviceCounters {
                device_id: "disk-test".into(),
                identity: "stable-test-id".into(),
                read_bytes: bytes,
                write_bytes: bytes,
                ..Default::default()
            }],
            observation: Observation::available("fixture"),
        }
    }
    fn rate(envelope: &Envelope) -> Option<f64> {
        let ProbeData::Activity(data) = &envelope.data else {
            panic!("activity expected")
        };
        data.devices[0].read_bytes_per_sec
    }
    #[test]
    fn disk_capture_interval_survives_serialization_and_delayed_delivery() {
        let start = Instant::now();
        let mut worker = WorkerSampler::default();
        let first = worker.activity(frame(100), start, 1000);
        assert_eq!(rate(&first), None);
        let second = worker.activity(frame(2600), start + Duration::from_millis(2500), 3500);
        assert_eq!(rate(&second), Some(1000.0));
        let bytes = serde_json::to_vec(&second).unwrap();
        // Decoding/arrival has no clock argument and cannot change the rate.
        for _ in 0..3 {
            let (data, captured, interval) = decode(CollectorTopic::Activity, &bytes).unwrap();
            assert_eq!(captured, 3500);
            assert_eq!(interval, Duration::from_millis(2500));
            let ProbeData::Activity(data) = data else {
                panic!("activity expected")
            };
            assert_eq!(data.devices[0].read_bytes_per_sec, Some(1000.0));
        }
    }
    #[test]
    fn suspend_excluding_monotonic_clock_and_new_worker_reset_disk_baselines() {
        let start = Instant::now();
        let mut worker = WorkerSampler::default();
        worker.activity(frame(100), start, 1000);
        let resumed = worker.activity(frame(5000), start + Duration::from_secs(1), 61000);
        assert_eq!(rate(&resumed), None);
        let next = worker.activity(frame(6000), start + Duration::from_secs(2), 62000);
        assert_eq!(rate(&next), Some(1000.0));
        let restarted =
            WorkerSampler::default().activity(frame(7000), start + Duration::from_secs(3), 63000);
        assert_eq!(rate(&restarted), None);
        assert_eq!(restarted.interval_ms, 0);
    }
}
