//! Session-local sampling. Each lane has one worker and one replaceable result;
//! neither a slow consumer nor a slow provider can create a work queue.
use crate::{
    cli::CollectorTopic,
    collectors::{self, probe::ProbeData, sampling::SampleMeta, SystemSnapshot},
    observation::Observation,
    types::ProcessSortKey,
};
use std::{
    panic::{catch_unwind, AssertUnwindSafe},
    sync::{
        atomic::{AtomicBool, AtomicU8, Ordering},
        Arc, Mutex,
    },
    thread::{self, JoinHandle},
    time::{Duration, Instant},
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum Profile {
    Full,
    Overview,
    Summary,
    Processes,
    Hidden,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(usize)]
pub enum Lane {
    Fast,
    Static,
    Slow,
    Connections,
    Diagnostics,
    Health,
    Drivers,
    Activity,
}

impl Lane {
    pub const ALL: [Self; 8] = [
        Self::Fast,
        Self::Static,
        Self::Slow,
        Self::Connections,
        Self::Diagnostics,
        Self::Health,
        Self::Drivers,
        Self::Activity,
    ];
    pub fn name(self) -> &'static str {
        match self {
            Self::Fast => "fast",
            Self::Static => "static",
            Self::Slow => "slow",
            Self::Connections => "connections",
            Self::Diagnostics => "diagnostics",
            Self::Health => "health",
            Self::Drivers => "drivers",
            Self::Activity => "activity",
        }
    }
    pub fn cadence(self) -> Duration {
        Duration::from_secs(match self {
            Self::Fast | Self::Activity => 1,
            Self::Static => 300,
            Self::Slow => 5,
            Self::Connections => 3,
            Self::Diagnostics => 15,
            Self::Health => 60,
            Self::Drivers => 300,
        })
    }
    fn topic(self) -> CollectorTopic {
        match self {
            Self::Static => CollectorTopic::Static,
            Self::Slow => CollectorTopic::Slow,
            Self::Connections => CollectorTopic::Connections,
            Self::Diagnostics => CollectorTopic::Diagnostics,
            Self::Health => CollectorTopic::Health,
            Self::Drivers => CollectorTopic::Drivers,
            Self::Activity => CollectorTopic::Activity,
            Self::Fast => unreachable!("fast samples stay in process"),
        }
    }
}

struct FastData {
    cpu: collectors::cpu::CpuData,
    memory: collectors::memory::MemoryData,
    network: Option<collectors::network::NetworkData>,
    processes: Option<collectors::processes::ProcessData>,
}
enum Data {
    Activity(collectors::disk_activity::DiskActivity),
    Fast(Box<FastData>),
    Probe(Box<ProbeData>),
}
struct Update {
    result: Result<Data, String>,
    captured: u64,
    interval: Duration,
    expected: Duration,
    sequence: u64,
}
struct Shared {
    stop: AtomicBool,
    profile: AtomicU8,
    sort: AtomicU8,
    retry: [AtomicBool; 8],
    slots: [Mutex<Option<Update>>; 8],
}

pub struct Monitor {
    shared: Arc<Shared>,
    workers: Vec<JoinHandle<()>>,
}

impl Monitor {
    pub fn start(profile: Profile) -> Self {
        let shared = Arc::new(Shared {
            stop: AtomicBool::new(false),
            profile: AtomicU8::new(profile as u8),
            sort: AtomicU8::new(0),
            retry: std::array::from_fn(|_| AtomicBool::new(false)),
            slots: std::array::from_fn(|_| Mutex::new(None)),
        });
        let workers = Lane::ALL
            .into_iter()
            .map(|lane| {
                let shared = Arc::clone(&shared);
                thread::spawn(move || run_lane(shared, lane))
            })
            .collect();
        Self { shared, workers }
    }
    pub fn set_profile(&self, profile: Profile) {
        if self.shared.profile.swap(profile as u8, Ordering::AcqRel) != profile as u8 {
            self.retry(Lane::Fast);
            self.wake();
        }
    }
    pub fn set_sort(&self, sort: ProcessSortKey) {
        let value = match sort {
            ProcessSortKey::Cpu => 0,
            ProcessSortKey::Memory => 1,
            ProcessSortKey::Name => 2,
            ProcessSortKey::Pid => 3,
        };
        if self.shared.sort.swap(value, Ordering::AcqRel) != value {
            self.retry(Lane::Fast);
        }
    }
    pub fn retry(&self, lane: Lane) {
        // CPU providers require a real sampling interval. Input repeats must
        // never turn their cached values into additional counter samples.
        if lane == Lane::Fast {
            return;
        }
        self.shared.retry[lane as usize].store(true, Ordering::Release);
        self.workers[lane as usize].thread().unpark();
    }
    fn wake(&self) {
        for worker in &self.workers {
            worker.thread().unpark();
        }
    }
    /// Drain at most one result per provider. A failure retains the last actual
    /// sample and its capture time while marking its availability explicitly.
    pub fn drain(&self, snapshot: &mut SystemSnapshot) -> Vec<Lane> {
        let mut changed = Vec::with_capacity(7);
        for lane in Lane::ALL {
            let update = self.shared.slots[lane as usize]
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .take();
            let Some(update) = update else { continue };
            match update.result {
                Ok(data) => {
                    match data {
                        Data::Activity(data) => {
                            snapshot.disk_activity = data;
                            snapshot
                                .disk_activity
                                .apply_legacy_io(&mut snapshot.disk_health);
                        }
                        Data::Fast(data) => {
                            snapshot.cpu = data.cpu;
                            snapshot.memory.total_bytes = data.memory.total_bytes;
                            snapshot.memory.used_bytes = data.memory.used_bytes;
                            snapshot.memory.available_bytes = data.memory.available_bytes;
                            snapshot.memory.swap_total_bytes = data.memory.swap_total_bytes;
                            snapshot.memory.swap_used_bytes = data.memory.swap_used_bytes;
                            if let Some(mut network) = data.network {
                                network.adapters = std::mem::take(&mut snapshot.network.adapters);
                                network.adapter_status = snapshot.network.adapter_status.clone();
                                snapshot.network = network;
                            }
                            if let Some(processes) = data.processes {
                                snapshot.processes = processes;
                            }
                        }
                        Data::Probe(data) => {
                            data.apply(snapshot);
                            snapshot
                                .disk_activity
                                .apply_legacy_io(&mut snapshot.disk_health);
                        }
                    }
                    snapshot.samples.insert(
                        lane.name().into(),
                        SampleMeta {
                            sequence: update.sequence,
                            captured_unix_ms: update.captured,
                            interval_ms: update.interval.as_millis() as u64,
                            expected_interval_ms: update.expected.as_millis() as u64,
                            observation: if lane == Lane::Fast && update.sequence == 1 {
                                Observation::unavailable(
                                    lane.name(),
                                    "Waiting for a second CPU counter sample",
                                )
                            } else {
                                Observation::available(lane.name())
                            },
                        },
                    );
                }
                Err(error) => {
                    let meta = snapshot.samples.entry(lane.name().into()).or_default();
                    meta.expected_interval_ms = update.expected.as_millis() as u64;
                    meta.observation = Observation::error(lane.name(), error);
                }
            }
            changed.push(lane);
        }
        changed
    }
}

impl Drop for Monitor {
    fn drop(&mut self) {
        self.shared.stop.store(true, Ordering::Release);
        self.wake();
        for worker in self.workers.drain(..) {
            let _ = worker.join();
        }
    }
}

fn run_lane(shared: Arc<Shared>, lane: Lane) {
    let mut snapshot = SystemSnapshot::default();
    let mut activity = collectors::disk_activity::DiskSampler::default();
    let mut next = Instant::now();
    let mut previous = None;
    let mut failures = 0u32;
    let mut sequence = 0u64;
    while !shared.stop.load(Ordering::Acquire) {
        let profile = shared.profile.load(Ordering::Acquire);
        let enabled = matches!(lane, Lane::Fast | Lane::Static)
            || profile == Profile::Full as u8
            || profile == Profile::Summary as u8
            || (profile == Profile::Hidden as u8 && matches!(lane, Lane::Slow | Lane::Health));
        let requested = shared.retry[lane as usize].swap(false, Ordering::AcqRel);
        let now = Instant::now();
        if !enabled || (!requested && now < next) {
            thread::park_timeout(if enabled {
                next.saturating_duration_since(now)
            } else {
                Duration::from_secs(60)
            });
            continue;
        }
        let expected = if lane == Lane::Slow && profile == Profile::Hidden as u8 {
            Duration::from_secs(30)
        } else {
            lane.cadence()
        };
        let result = catch_unwind(AssertUnwindSafe(|| {
            if lane != Lane::Fast {
                return collectors::probe::collect(lane.topic(), &shared.stop).map(
                    |(data, captured)| {
                        let data = match data {
                            ProbeData::Activity(frame) => {
                                Data::Activity(activity.sample(frame, Instant::now()))
                            }
                            data => Data::Probe(Box::new(data)),
                        };
                        (data, captured)
                    },
                );
            }
            let full = profile == Profile::Full as u8;
            let summary = profile == Profile::Summary as u8;
            let processes = profile == Profile::Processes as u8;
            if full {
                snapshot.refresh_fast();
            } else if processes {
                let sort = match shared.sort.load(Ordering::Acquire) {
                    1 => ProcessSortKey::Memory,
                    2 => ProcessSortKey::Name,
                    3 => ProcessSortKey::Pid,
                    _ => ProcessSortKey::Cpu,
                };
                snapshot.refresh_processes_gui(sort);
            } else if summary {
                snapshot.refresh_fast_gui_summary();
            } else {
                snapshot.refresh_overview();
            }
            Ok((
                Data::Fast(Box::new(FastData {
                    cpu: snapshot.cpu.clone(),
                    memory: snapshot.memory.clone(),
                    network: (full || summary).then(|| snapshot.network.clone()),
                    processes: (full || processes).then(|| snapshot.processes.clone()),
                })),
                collectors::sampling::unix_ms(),
            ))
        }))
        .unwrap_or_else(|_| {
            Err(format!(
                "{} collector panicked; retry is available",
                lane.name()
            ))
        });
        let captured = result.as_ref().map(|(_, captured)| *captured).unwrap_or(0);
        if result.is_ok() {
            failures = 0;
            sequence = sequence.saturating_add(1);
        } else {
            failures = failures.saturating_add(1);
        }
        let interval = previous
            .map(|last| now.saturating_duration_since(last))
            .unwrap_or_default();
        previous = Some(now);
        *shared.slots[lane as usize]
            .lock()
            .unwrap_or_else(|e| e.into_inner()) = Some(Update {
            result: result.map(|(data, _)| data),
            captured,
            interval,
            expected,
            sequence,
        });
        let delay = if failures == 0 {
            expected
        } else {
            Duration::from_secs(5 * 2u64.pow(failures.min(5)))
        };
        // Skip overdue work. Sampling never catches up with a burst after a stall.
        next = (now + delay).max(Instant::now());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn session_without_workers() -> Monitor {
        Monitor {
            shared: Arc::new(Shared {
                stop: AtomicBool::new(false),
                profile: AtomicU8::new(0),
                sort: AtomicU8::new(0),
                retry: std::array::from_fn(|_| AtomicBool::new(false)),
                slots: std::array::from_fn(|_| Mutex::new(None)),
            }),
            workers: Vec::new(),
        }
    }
    fn update(result: Result<Data, String>, sequence: u64) -> Update {
        Update {
            result,
            captured: 1000 * sequence,
            interval: Duration::from_secs(1),
            expected: Duration::from_secs(1),
            sequence,
        }
    }
    fn fast(cpu: f32) -> Data {
        Data::Fast(Box::new(FastData {
            cpu: collectors::cpu::CpuData {
                total_usage: cpu,
                ..Default::default()
            },
            memory: Default::default(),
            network: None,
            processes: None,
        }))
    }
    #[test]
    fn latest_only_delivery_preserves_actual_capture_on_failure_and_recovers() {
        let monitor = session_without_workers();
        let mut snapshot = SystemSnapshot::default();
        let slot = &monitor.shared.slots[Lane::Fast as usize];
        *slot.lock().unwrap() = Some(update(Ok(fast(10.0)), 1));
        *slot.lock().unwrap() = Some(update(Ok(fast(25.0)), 2));
        assert_eq!(monitor.drain(&mut snapshot), [Lane::Fast]);
        assert_eq!(snapshot.cpu.total_usage, 25.0);
        assert_eq!(snapshot.samples["fast"].captured_unix_ms, 2000);
        assert!(monitor.drain(&mut snapshot).is_empty());
        *slot.lock().unwrap() = Some(update(Err("permission denied".into()), 2));
        monitor.drain(&mut snapshot);
        assert_eq!(snapshot.cpu.total_usage, 25.0);
        assert_eq!(snapshot.samples["fast"].captured_unix_ms, 2000);
        assert!(!snapshot.samples["fast"].observation.is_available());
        *slot.lock().unwrap() = Some(update(Ok(fast(30.0)), 3));
        monitor.drain(&mut snapshot);
        assert!(snapshot.samples["fast"].observation.is_available());
        assert_eq!(snapshot.samples["fast"].captured_unix_ms, 3000);
    }
}
