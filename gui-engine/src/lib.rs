use std::ffi::{c_char, c_void};
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::path::{Path, PathBuf};
use std::ptr;
use std::sync::atomic::{AtomicBool, AtomicU8, Ordering};
use std::sync::{mpsc, Arc, Condvar, Mutex};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use sd_300::collectors::disk_health::DiskHealthStatus;
use sd_300::collectors::{self, DiagnosticWarning, SystemSnapshot, WarningSeverity};
use sd_300::types::ProcessSortKey;
use serde::Serialize;
use serde_json::json;

pub const ABI_VERSION: u32 = 1;
pub const SCHEMA_VERSION: u32 = 1;

pub const STATUS_OK: i32 = 0;
pub const STATUS_UNCHANGED: i32 = 1;
pub const STATUS_BUFFER_TOO_SMALL: i32 = 2;
pub const STATUS_ALREADY_RUNNING: i32 = 3;
pub const STATUS_INVALID_ARGUMENT: i32 = -1;
pub const STATUS_INVALID_TOPIC: i32 = -2;
pub const STATUS_NOT_RUNNING: i32 = -3;
pub const STATUS_INTERNAL_ERROR: i32 = -8;
pub const STATUS_PANIC: i32 = -9;

const TOPIC_COUNT: usize = 9;
const PROFILE_FOREGROUND: u8 = 0;
const PROFILE_HIDDEN: u8 = 1;
const PROFILE_OVERVIEW: u8 = 2;
const PROFILE_PROCESSES: u8 = 3;
const PROCESS_SORT_CPU: u8 = 0;
const PROCESS_SORT_MEMORY: u8 = 1;
const PROCESS_SORT_PID: u8 = 2;
const PROCESS_SORT_NAME: u8 = 3;
const PROCESS_SUMMARY_ROWS: usize = 16;
const PROCESS_NAME_BYTES: usize = 96;
const PROCESS_STATUS_BYTES: usize = 32;
const EXPORT_NONE: u8 = 0;
const EXPORT_SNAPSHOT: u8 = 1;
const EXPORT_CAPABILITIES: u8 = 2;

#[repr(u32)]
#[derive(Clone, Copy)]
pub enum Topic {
    Static = 0,
    Fast = 1,
    Medium = 2,
    Slow = 3,
    Diagnostics = 4,
    Health = 5,
    Drivers = 6,
    Warnings = 7,
    Capabilities = 8,
}

impl Topic {
    fn from_raw(value: u32) -> Option<Self> {
        match value {
            0 => Some(Self::Static),
            1 => Some(Self::Fast),
            2 => Some(Self::Medium),
            3 => Some(Self::Slow),
            4 => Some(Self::Diagnostics),
            5 => Some(Self::Health),
            6 => Some(Self::Drivers),
            7 => Some(Self::Warnings),
            8 => Some(Self::Capabilities),
            _ => None,
        }
    }

    fn name(self) -> &'static str {
        match self {
            Self::Static => "static",
            Self::Fast => "fast",
            Self::Medium => "medium",
            Self::Slow => "slow",
            Self::Diagnostics => "diagnostics",
            Self::Health => "health",
            Self::Drivers => "drivers",
            Self::Warnings => "warnings",
            Self::Capabilities => "capabilities",
        }
    }

    fn provenance(self) -> &'static str {
        match self {
            Self::Static => "SD-300 system, display, memory, and network collectors",
            Self::Fast => "SD-300 sysinfo-backed live collectors",
            Self::Medium => "SD-300 platform connection collector",
            Self::Slow => "SD-300 disk, GPU, and thermal collectors",
            Self::Diagnostics => "SD-300 gateway, DNS, and internet collectors",
            Self::Health => "SD-300 platform disk-health collector",
            Self::Drivers => "SD-300 platform device provider",
            Self::Warnings => "SD-300 warning deduplication pipeline",
            Self::Capabilities => "SD-300 capability and provenance matrix",
        }
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct FastSummary {
    pub sequence: u64,
    pub captured_unix_ms: u64,
    pub cpu_percent: f32,
    pub memory_percent: f32,
    pub memory_used_bytes: u64,
    pub memory_total_bytes: u64,
    pub logical_processors: u32,
    pub warning_count: u32,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct TraySummary {
    pub sequence: u64,
    pub gpu_percent: f32,
    pub storage_free_percent: f32,
    pub gpu_available: u32,
    pub storage_available: u32,
    pub disk_health: u32,
    pub reserved: u32,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct ProcessRowSummary {
    pub pid: u32,
    pub cpu_percent: f32,
    pub memory_bytes: u64,
    pub memory_percent: f64,
    pub name_len: u32,
    pub friendly_name_len: u32,
    pub status_len: u32,
    pub reserved: u32,
    pub name: [u8; PROCESS_NAME_BYTES],
    pub friendly_name: [u8; PROCESS_NAME_BYTES],
    pub status: [u8; PROCESS_STATUS_BYTES],
}

impl Default for ProcessRowSummary {
    fn default() -> Self {
        Self {
            pid: 0,
            cpu_percent: 0.0,
            memory_bytes: 0,
            memory_percent: 0.0,
            name_len: 0,
            friendly_name_len: 0,
            status_len: 0,
            reserved: 0,
            name: [0; PROCESS_NAME_BYTES],
            friendly_name: [0; PROCESS_NAME_BYTES],
            status: [0; PROCESS_STATUS_BYTES],
        }
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct ProcessSummary {
    pub sequence: u64,
    pub captured_unix_ms: u64,
    pub total_count: u32,
    pub total_threads: u32,
    pub row_count: u32,
    pub reserved: u32,
    pub rows: [ProcessRowSummary; PROCESS_SUMMARY_ROWS],
}

impl Default for ProcessSummary {
    fn default() -> Self {
        Self {
            sequence: 0,
            captured_unix_ms: 0,
            total_count: 0,
            total_threads: 0,
            row_count: 0,
            reserved: 0,
            rows: [ProcessRowSummary::default(); PROCESS_SUMMARY_ROWS],
        }
    }
}

#[derive(Default)]
struct LatestTopic {
    sequence: u64,
    json: Vec<u8>,
}

struct Shared {
    stop: AtomicBool,
    running: AtomicBool,
    driver_request: AtomicBool,
    export_request: AtomicU8,
    profile: AtomicU8,
    process_sort: AtomicU8,
    wake_lock: Mutex<()>,
    wake: Condvar,
    topics: Mutex<[LatestTopic; TOPIC_COUNT]>,
    summary: Mutex<FastSummary>,
    tray_summary: Mutex<TraySummary>,
    process_summary: Mutex<ProcessSummary>,
    last_error: Mutex<Vec<u8>>,
    export_status: Mutex<Vec<u8>>,
}

impl Default for Shared {
    fn default() -> Self {
        Self {
            stop: AtomicBool::new(false),
            running: AtomicBool::new(false),
            driver_request: AtomicBool::new(false),
            export_request: AtomicU8::new(EXPORT_NONE),
            profile: AtomicU8::new(PROFILE_FOREGROUND),
            process_sort: AtomicU8::new(PROCESS_SORT_CPU),
            wake_lock: Mutex::new(()),
            wake: Condvar::new(),
            topics: Mutex::new(std::array::from_fn(|_| LatestTopic::default())),
            summary: Mutex::new(FastSummary::default()),
            tray_summary: Mutex::new(TraySummary::default()),
            process_summary: Mutex::new(ProcessSummary::default()),
            last_error: Mutex::new(Vec::new()),
            export_status: Mutex::new(br#"{"state":"idle"}"#.to_vec()),
        }
    }
}

struct Engine {
    shared: Arc<Shared>,
    worker: Mutex<Option<JoinHandle<()>>>,
}

impl Engine {
    fn new() -> Self {
        Self {
            shared: Arc::new(Shared::default()),
            worker: Mutex::new(None),
        }
    }

    fn start(&self) -> i32 {
        let mut worker = match self.worker.lock() {
            Ok(worker) => worker,
            Err(_) => return STATUS_INTERNAL_ERROR,
        };
        if worker.is_some() || self.shared.running.swap(true, Ordering::AcqRel) {
            return STATUS_ALREADY_RUNNING;
        }
        self.shared.stop.store(false, Ordering::Release);
        let shared = Arc::clone(&self.shared);
        *worker = Some(thread::spawn(move || worker_main(shared)));
        STATUS_OK
    }

    fn stop(&self) -> i32 {
        self.shared.stop.store(true, Ordering::Release);
        self.shared.wake.notify_all();
        let handle = match self.worker.lock() {
            Ok(mut worker) => worker.take(),
            Err(_) => return STATUS_INTERNAL_ERROR,
        };
        if let Some(handle) = handle {
            if handle.join().is_err() {
                self.shared.running.store(false, Ordering::Release);
                set_error(&self.shared, "collector worker panicked");
                return STATUS_PANIC;
            }
            STATUS_OK
        } else {
            STATUS_NOT_RUNNING
        }
    }
}

impl Drop for Engine {
    fn drop(&mut self) {
        self.shared.stop.store(true, Ordering::Release);
        self.shared.wake.notify_all();
        if let Ok(worker) = self.worker.get_mut() {
            if let Some(handle) = worker.take() {
                let _ = handle.join();
            }
        }
    }
}

#[derive(Serialize)]
struct StaticProjection<'a> {
    system: &'a collectors::system_info::SystemInfoData,
    displays: &'a collectors::display::DisplayData,
    memory_modules: &'a [collectors::memory::MemoryModule],
    memory_module_status: &'a sd_300::observation::Observation,
    network_adapters: &'a [collectors::network::NetworkAdapterInfo],
    network_adapter_status: &'a sd_300::observation::Observation,
}

#[derive(Serialize)]
struct FastProjection<'a> {
    cpu: &'a collectors::cpu::CpuData,
    memory: &'a collectors::memory::MemoryData,
    network: &'a collectors::network::NetworkData,
    processes: &'a collectors::processes::ProcessData,
}

#[derive(Serialize)]
struct SlowProjection<'a> {
    disk: &'a collectors::disk::DiskData,
    gpu: &'a collectors::gpu::GpuData,
    thermals: &'a collectors::thermals::ThermalData,
}

#[derive(Serialize)]
struct MediumProjection<'a> {
    active_connections: &'a [collectors::network_diag::ConnectionInfo],
    listening_ports: &'a [collectors::network_diag::ConnectionInfo],
}

#[derive(Serialize)]
struct TopicEnvelope<'a, T: ?Sized> {
    schema_version: u32,
    product_version: &'static str,
    target: &'static str,
    topic: &'static str,
    sequence: u64,
    captured_unix_ms: u64,
    freshness_ms: u64,
    availability: &'static str,
    provenance: &'static str,
    warnings: &'a [DiagnosticWarning],
    data: &'a T,
}

fn target_label() -> &'static str {
    match (std::env::consts::ARCH, std::env::consts::OS) {
        ("x86_64", "windows") => "x86_64-windows",
        ("x86_64", "macos") => "x86_64-macos",
        ("aarch64", "macos") => "aarch64-macos",
        ("x86_64", "linux") => "x86_64-linux",
        ("aarch64", "linux") => "aarch64-linux",
        _ => "unsupported-target",
    }
}

fn unix_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
        .try_into()
        .unwrap_or(u64::MAX)
}

fn publish<T: Serialize>(shared: &Shared, topic: Topic, data: &T, warnings: &[DiagnosticWarning]) {
    let mut topics = match shared.topics.lock() {
        Ok(topics) => topics,
        Err(_) => return,
    };
    let state = &mut topics[topic as usize];
    state.sequence = state.sequence.saturating_add(1);
    let captured_unix_ms = unix_ms();
    let envelope = TopicEnvelope {
        schema_version: SCHEMA_VERSION,
        product_version: env!("CARGO_PKG_VERSION"),
        target: target_label(),
        topic: topic.name(),
        sequence: state.sequence,
        captured_unix_ms,
        freshness_ms: 0,
        availability: "available",
        provenance: topic.provenance(),
        warnings,
        data,
    };
    match serde_json::to_vec(&envelope) {
        Ok(json) => state.json = json,
        Err(error) => set_error(
            shared,
            &format!("{} serialization failed: {error}", topic.name()),
        ),
    }
}

fn publish_fast(shared: &Shared, snapshot: &SystemSnapshot) {
    publish(
        shared,
        Topic::Fast,
        &FastProjection {
            cpu: &snapshot.cpu,
            memory: &snapshot.memory,
            network: &snapshot.network,
            processes: &snapshot.processes,
        },
        &snapshot.warnings,
    );
    update_fast_summary(shared, snapshot);
}

fn update_fast_summary(shared: &Shared, snapshot: &SystemSnapshot) {
    if let Ok(mut summary) = shared.summary.lock() {
        let sequence = summary.sequence.saturating_add(1);
        *summary = FastSummary {
            sequence,
            captured_unix_ms: unix_ms(),
            cpu_percent: snapshot.cpu.total_usage,
            memory_percent: snapshot.memory.usage_percent() as f32,
            memory_used_bytes: snapshot.memory.used_bytes,
            memory_total_bytes: snapshot.memory.total_bytes,
            logical_processors: snapshot.cpu.thread_count.try_into().unwrap_or(u32::MAX),
            warning_count: snapshot.warnings.len().try_into().unwrap_or(u32::MAX),
        };
    }
}

fn copy_summary_text<const N: usize>(destination: &mut [u8; N], text: &str) -> u32 {
    let mut length = text.len().min(N);
    while length > 0 && !text.is_char_boundary(length) {
        length -= 1;
    }
    destination[..length].copy_from_slice(&text.as_bytes()[..length]);
    length.try_into().unwrap_or(u32::MAX)
}

fn update_process_summary(shared: &Shared, snapshot: &SystemSnapshot) {
    let Ok(mut summary) = shared.process_summary.lock() else {
        return;
    };
    let sequence = summary.sequence.saturating_add(1);
    let mut next = ProcessSummary {
        sequence,
        captured_unix_ms: unix_ms(),
        total_count: snapshot
            .processes
            .total_count
            .try_into()
            .unwrap_or(u32::MAX),
        total_threads: snapshot
            .processes
            .total_threads
            .try_into()
            .unwrap_or(u32::MAX),
        row_count: snapshot
            .processes
            .list
            .len()
            .min(PROCESS_SUMMARY_ROWS)
            .try_into()
            .unwrap_or(PROCESS_SUMMARY_ROWS as u32),
        ..ProcessSummary::default()
    };
    for (destination, source) in next.rows.iter_mut().zip(&snapshot.processes.list) {
        destination.pid = source.pid;
        destination.cpu_percent = source.cpu_percent;
        destination.memory_bytes = source.memory_bytes;
        destination.memory_percent = source.memory_percent;
        destination.name_len = copy_summary_text(&mut destination.name, &source.name);
        destination.friendly_name_len =
            copy_summary_text(&mut destination.friendly_name, &source.friendly_name);
        destination.status_len = copy_summary_text(&mut destination.status, &source.status);
    }
    *summary = next;
}

fn update_tray_summary(shared: &Shared, snapshot: &SystemSnapshot) {
    let total_bytes = snapshot
        .disk
        .partitions
        .iter()
        .filter(|partition| !partition.is_removable)
        .map(|partition| partition.total_bytes)
        .sum::<u64>();
    let free_bytes = snapshot
        .disk
        .partitions
        .iter()
        .filter(|partition| !partition.is_removable)
        .map(|partition| partition.available_bytes)
        .sum::<u64>();
    let disk_health = snapshot
        .disk_health
        .drives
        .iter()
        .map(|drive| match drive.health_status {
            DiskHealthStatus::Healthy => 1,
            DiskHealthStatus::Warning => 2,
            DiskHealthStatus::Critical => 3,
            DiskHealthStatus::Unknown => 0,
        })
        .max()
        .unwrap_or(0);
    if let Ok(mut summary) = shared.tray_summary.lock() {
        let sequence = summary.sequence.saturating_add(1);
        *summary = TraySummary {
            sequence,
            gpu_percent: snapshot.gpu.utilization_percent,
            storage_free_percent: if total_bytes == 0 {
                0.0
            } else {
                (free_bytes as f64 / total_bytes as f64 * 100.0) as f32
            },
            gpu_available: u32::from(snapshot.gpu.telemetry_available),
            storage_available: u32::from(total_bytes > 0),
            disk_health,
            reserved: 0,
        };
    }
}

fn worker_main(shared: Arc<Shared>) {
    let result = catch_unwind(AssertUnwindSafe(|| collect_loop(&shared)));
    if result.is_err() {
        set_error(&shared, "collector worker panicked");
    }
    shared.running.store(false, Ordering::Release);
}

fn process_sort_from_raw(value: u32) -> Option<ProcessSortKey> {
    match value {
        value if value == u32::from(PROCESS_SORT_CPU) => Some(ProcessSortKey::Cpu),
        value if value == u32::from(PROCESS_SORT_MEMORY) => Some(ProcessSortKey::Memory),
        value if value == u32::from(PROCESS_SORT_PID) => Some(ProcessSortKey::Pid),
        value if value == u32::from(PROCESS_SORT_NAME) => Some(ProcessSortKey::Name),
        _ => None,
    }
}

fn selected_process_sort(shared: &Shared) -> ProcessSortKey {
    process_sort_from_raw(u32::from(shared.process_sort.load(Ordering::Acquire)))
        .unwrap_or(ProcessSortKey::Cpu)
}

fn collect_loop(shared: &Shared) {
    let mut snapshot = SystemSnapshot::default();

    // Static identity and display inventory are collected once on the engine
    // worker even for the lightweight Overview profile. They are part of the
    // existing TUI contract, do not participate in the one-second loop, and
    // therefore add parity without adding permanent renderer/collector load.
    snapshot.refresh_static();
    publish(
        shared,
        Topic::Static,
        &StaticProjection {
            system: &snapshot.system,
            displays: &snapshot.displays,
            memory_modules: &snapshot.memory.modules,
            memory_module_status: &snapshot.memory.module_status,
            network_adapters: &snapshot.network.adapters,
            network_adapter_status: &snapshot.network.adapter_status,
        },
        &snapshot.warnings,
    );
    let (health_tx, health_rx) = mpsc::channel();
    let mut health_running = false;

    // The first native surface only displays CPU and memory. Start with the
    // smallest truthful collection set and do not run command-backed connection,
    // GPU, diagnostic, health, or driver probes until the UI selects a profile
    // that needs them.
    if matches!(
        shared.profile.load(Ordering::Acquire),
        PROFILE_OVERVIEW | PROFILE_HIDDEN
    ) {
        snapshot.refresh_overview();
        thread::sleep(Duration::from_millis(250));
        snapshot.refresh_overview();
        update_fast_summary(shared, &snapshot);
        publish(
            shared,
            Topic::Warnings,
            &snapshot.warnings,
            &snapshot.warnings,
        );
        let capabilities = sd_300::report::capabilities_for(&snapshot);
        publish(
            shared,
            Topic::Capabilities,
            &capabilities,
            &snapshot.warnings,
        );
        let mut active_profile = shared.profile.load(Ordering::Acquire);
        let mut next_overview = Instant::now() + Duration::from_secs(1);
        let mut next_hidden_slow = Instant::now();
        let mut next_hidden_health = Instant::now();
        while !shared.stop.load(Ordering::Acquire) {
            service_export_request(shared, &snapshot);
            let profile = shared.profile.load(Ordering::Acquire);
            if !matches!(profile, PROFILE_OVERVIEW | PROFILE_HIDDEN) {
                break;
            }
            if profile != active_profile {
                active_profile = profile;
                next_overview = Instant::now();
            }
            let now = Instant::now();
            if now >= next_overview {
                snapshot.refresh_overview();
                update_fast_summary(shared, &snapshot);
                next_overview = now + Duration::from_secs(1);
            }
            if profile == PROFILE_HIDDEN && now >= next_hidden_slow {
                snapshot.refresh_slow();
                publish(
                    shared,
                    Topic::Slow,
                    &SlowProjection {
                        disk: &snapshot.disk,
                        gpu: &snapshot.gpu,
                        thermals: &snapshot.thermals,
                    },
                    &snapshot.warnings,
                );
                update_tray_summary(shared, &snapshot);
                next_hidden_slow = now + Duration::from_secs(30);
            }
            if profile == PROFILE_HIDDEN && now >= next_hidden_health && !health_running {
                let sender = health_tx.clone();
                thread::spawn(move || {
                    let _ = sender.send(collectors::disk_health::collect());
                });
                health_running = true;
                next_hidden_health = now + Duration::from_secs(60);
            }
            if let Ok((health, warnings)) = health_rx.try_recv() {
                health_running = false;
                snapshot.disk_health = health;
                snapshot
                    .warnings
                    .retain(|warning| warning.source != "Disk Health");
                snapshot.warnings.extend(warnings);
                publish(
                    shared,
                    Topic::Health,
                    &snapshot.disk_health,
                    &snapshot.warnings,
                );
                update_tray_summary(shared, &snapshot);
            }
            wait_for_wake(
                shared,
                next_overview.saturating_duration_since(Instant::now()),
            );
        }
        if shared.stop.load(Ordering::Acquire) {
            return;
        }
    }

    if shared.profile.load(Ordering::Acquire) == PROFILE_PROCESSES {
        snapshot.refresh_processes_gui(selected_process_sort(shared));
        update_process_summary(shared, &snapshot);
    } else {
        snapshot.refresh_fast_gui_summary();
    }
    thread::sleep(Duration::from_millis(250));
    if shared.profile.load(Ordering::Acquire) == PROFILE_PROCESSES {
        snapshot.refresh_processes_gui(selected_process_sort(shared));
        update_process_summary(shared, &snapshot);
    } else {
        snapshot.refresh_fast_gui_summary();
    }
    publish_fast(shared, &snapshot);
    publish(
        shared,
        Topic::Warnings,
        &snapshot.warnings,
        &snapshot.warnings,
    );
    let capabilities = sd_300::report::capabilities_for(&snapshot);
    publish(
        shared,
        Topic::Capabilities,
        &capabilities,
        &snapshot.warnings,
    );

    let (driver_tx, driver_rx) = mpsc::channel();
    let (diag_tx, diag_rx) = mpsc::channel();
    let mut driver_running = false;
    let mut diag_running = false;
    let mut next_fast = Instant::now() + Duration::from_secs(1);
    let mut next_medium = Instant::now();
    let mut next_slow = Instant::now();
    let mut next_diag = Instant::now();
    let mut next_health = Instant::now();
    let mut active_profile = shared.profile.load(Ordering::Acquire);
    let mut active_process_sort = shared.process_sort.load(Ordering::Acquire);
    shared.driver_request.store(true, Ordering::Release);

    while !shared.stop.load(Ordering::Acquire) {
        service_export_request(shared, &snapshot);
        let now = Instant::now();
        let profile = shared.profile.load(Ordering::Acquire);
        if profile != active_profile {
            active_profile = profile;
            next_fast = now;
        }
        let process_sort = shared.process_sort.load(Ordering::Acquire);
        if process_sort != active_process_sort {
            active_process_sort = process_sort;
            if profile == PROFILE_PROCESSES {
                next_fast = now;
            }
        }
        if profile == PROFILE_OVERVIEW {
            if now >= next_fast {
                snapshot.refresh_overview();
                update_fast_summary(shared, &snapshot);
                next_fast = now + Duration::from_secs(1);
            }
            thread::sleep(Duration::from_millis(50));
            continue;
        }
        let hidden = profile == PROFILE_HIDDEN;
        let process_view = profile == PROFILE_PROCESSES;
        let full_detail = profile == PROFILE_FOREGROUND;
        let mut capability_state_changed = false;

        if now >= next_fast {
            if hidden {
                // Tray/hidden mode still samples its visible CPU and memory
                // summary every second. It deliberately skips the process
                // inventory: no hidden UI consumes that table, and reopening
                // the window forces a full foreground refresh.
                snapshot.refresh_overview();
                update_fast_summary(shared, &snapshot);
            } else if process_view {
                snapshot.refresh_processes_gui(selected_process_sort(shared));
                update_process_summary(shared, &snapshot);
                publish_fast(shared, &snapshot);
            } else {
                snapshot.refresh_fast_gui_summary();
                publish_fast(shared, &snapshot);
            }
            next_fast = now + Duration::from_secs(1);
        }
        if full_detail && now >= next_medium {
            snapshot.refresh_connections();
            publish(
                shared,
                Topic::Medium,
                &MediumProjection {
                    active_connections: &snapshot.network_diag.active_connections,
                    listening_ports: &snapshot.network_diag.listening_ports,
                },
                &snapshot.warnings,
            );
            next_medium = now + Duration::from_secs(3);
        }
        if full_detail && now >= next_slow {
            snapshot.refresh_slow();
            publish(
                shared,
                Topic::Slow,
                &SlowProjection {
                    disk: &snapshot.disk,
                    gpu: &snapshot.gpu,
                    thermals: &snapshot.thermals,
                },
                &snapshot.warnings,
            );
            update_tray_summary(shared, &snapshot);
            capability_state_changed = true;
            next_slow = now + Duration::from_secs(5);
        }
        if full_detail && now >= next_diag && !diag_running {
            let sender = diag_tx.clone();
            thread::spawn(move || {
                let _ = sender.send(collectors::network_diag::collect_connectivity());
            });
            diag_running = true;
            next_diag = now + Duration::from_secs(15);
        }
        if full_detail && now >= next_health && !health_running {
            let sender = health_tx.clone();
            thread::spawn(move || {
                let _ = sender.send(collectors::disk_health::collect());
            });
            health_running = true;
            next_health = now + Duration::from_secs(60);
        }
        if full_detail && shared.driver_request.swap(false, Ordering::AcqRel) && !driver_running {
            let sender = driver_tx.clone();
            thread::spawn(move || {
                let _ = sender.send(collectors::drivers::collect());
            });
            driver_running = true;
        }

        if let Ok(drivers) = driver_rx.try_recv() {
            driver_running = false;
            snapshot.drivers = drivers;
            snapshot
                .warnings
                .retain(|warning| warning.source != "Drivers");
            if let collectors::drivers::DriverScanStatus::ScanFailed(message) =
                &snapshot.drivers.scan_status
            {
                snapshot.warnings.push(DiagnosticWarning {
                    source: "Drivers".into(),
                    message: message.clone(),
                    severity: WarningSeverity::Warning,
                });
            }
            publish(
                shared,
                Topic::Drivers,
                &snapshot.drivers,
                &snapshot.warnings,
            );
            capability_state_changed = true;
        }
        if let Ok((diagnostics, warnings)) = diag_rx.try_recv() {
            diag_running = false;
            snapshot.network_diag.gateway = diagnostics.gateway;
            snapshot.network_diag.dns = diagnostics.dns;
            snapshot.network_diag.internet = diagnostics.internet;
            snapshot
                .warnings
                .retain(|warning| warning.source != "Network");
            snapshot.warnings.extend(warnings);
            publish(
                shared,
                Topic::Diagnostics,
                &snapshot.network_diag,
                &snapshot.warnings,
            );
            capability_state_changed = true;
        }
        if let Ok((health, warnings)) = health_rx.try_recv() {
            health_running = false;
            snapshot.disk_health = health;
            snapshot
                .warnings
                .retain(|warning| warning.source != "Disk Health");
            snapshot.warnings.extend(warnings);
            publish(
                shared,
                Topic::Health,
                &snapshot.disk_health,
                &snapshot.warnings,
            );
            update_tray_summary(shared, &snapshot);
            capability_state_changed = true;
        }

        if capability_state_changed {
            publish(
                shared,
                Topic::Warnings,
                &snapshot.warnings,
                &snapshot.warnings,
            );
            let capabilities = sd_300::report::capabilities_for(&snapshot);
            publish(
                shared,
                Topic::Capabilities,
                &capabilities,
                &snapshot.warnings,
            );
        }
        if process_view && !driver_running && !diag_running && !health_running {
            // The Processes profile has no command-backed background jobs to
            // poll. Profile/sort/export/stop setters all signal this condvar,
            // so sleep directly until the next one-second sample instead of
            // waking the collector worker twenty times per second.
            wait_for_wake(
                shared,
                next_fast.saturating_duration_since(Instant::now()),
            );
        } else {
            // Detailed pages can have driver, connectivity, or disk-health
            // workers completing on channels that do not own the wake handle.
            thread::sleep(Duration::from_millis(50));
        }
    }
}

fn service_export_request(shared: &Shared, snapshot: &SystemSnapshot) {
    let kind = shared.export_request.swap(EXPORT_NONE, Ordering::AcqRel);
    if kind == EXPORT_NONE {
        return;
    }
    let result = write_export(snapshot, kind);
    let status = match result {
        Ok(path) => json!({
            "state": "complete",
            "kind": if kind == EXPORT_SNAPSHOT { "redacted_snapshot" } else { "capabilities" },
            "path": path.to_string_lossy(),
        }),
        Err(error) => {
            set_error(shared, &error);
            json!({
                "state": "error",
                "kind": if kind == EXPORT_SNAPSHOT { "redacted_snapshot" } else { "capabilities" },
                "error": error,
            })
        }
    };
    if let (Ok(bytes), Ok(mut destination)) =
        (serde_json::to_vec(&status), shared.export_status.lock())
    {
        *destination = bytes;
    }
}

fn write_export(snapshot: &SystemSnapshot, kind: u8) -> Result<PathBuf, String> {
    let directory = sd_300::settings::reports_dir()?;
    ensure_export_directory(&directory)?;
    let captured = unix_ms();
    let stem = if kind == EXPORT_SNAPSHOT {
        "sd300-redacted-snapshot"
    } else if kind == EXPORT_CAPABILITIES {
        "sd300-capabilities"
    } else {
        return Err("unknown export kind".into());
    };
    let report = sd_300::report::DiagnosticReport::from_snapshot(snapshot, false);
    let bytes = if kind == EXPORT_SNAPSHOT {
        serde_json::to_vec_pretty(&report)
    } else {
        serde_json::to_vec_pretty(&json!({
            "schema_version": report.schema_version,
            "product": report.product,
            "product_version": report.product_version,
            "target_os": report.target_os,
            "target_arch": report.target_arch,
            "capabilities": report.capabilities,
            "warnings": report.warnings,
        }))
    }
    .map_err(|error| format!("could not serialize the requested export: {error}"))?;

    for suffix in 0..100u8 {
        let file_name = if suffix == 0 {
            format!("{stem}-{captured}.json")
        } else {
            format!("{stem}-{captured}-{suffix}.json")
        };
        let destination = directory.join(file_name);
        if destination.exists() {
            continue;
        }
        write_export_atomically(&destination, &bytes)?;
        return Ok(destination);
    }
    Err("could not allocate a unique report filename".into())
}

fn ensure_export_directory(directory: &Path) -> Result<(), String> {
    match fs::symlink_metadata(directory) {
        Ok(metadata) if metadata.file_type().is_symlink() || !metadata.is_dir() => {
            return Err(format!(
                "report destination {} is not an owned directory and was preserved",
                directory.display()
            ));
        }
        Ok(_) => return restrict_export_directory(directory),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => {
            return Err(format!(
                "could not inspect report destination {}: {error}",
                directory.display()
            ));
        }
    }
    fs::create_dir_all(directory).map_err(|error| {
        format!(
            "could not create report destination {}: {error}",
            directory.display()
        )
    })?;
    restrict_export_directory(directory)
}

#[cfg(unix)]
fn restrict_export_directory(directory: &Path) -> Result<(), String> {
    use std::os::unix::fs::{MetadataExt, PermissionsExt};

    let effective_uid = unsafe { libc::geteuid() };
    for path in directory.parent().into_iter().chain([directory]) {
        let metadata = fs::symlink_metadata(path)
            .map_err(|error| format!("could not inspect {}: {error}", path.display()))?;
        if !metadata.is_dir()
            || metadata.file_type().is_symlink()
            || metadata.uid() != effective_uid
        {
            return Err(format!(
                "report destination component {} is not a same-user directory and was preserved",
                path.display()
            ));
        }
        fs::set_permissions(path, fs::Permissions::from_mode(0o700))
            .map_err(|error| format!("could not restrict {}: {error}", path.display()))?;
    }
    Ok(())
}

#[cfg(not(unix))]
fn restrict_export_directory(_directory: &Path) -> Result<(), String> {
    Ok(())
}

fn write_export_atomically(destination: &Path, bytes: &[u8]) -> Result<(), String> {
    let temporary = destination.with_extension("json.tmp");
    let mut options = OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut file = options.open(&temporary).map_err(|error| {
        format!(
            "could not create temporary report {}: {error}",
            temporary.display()
        )
    })?;
    let result = (|| {
        file.write_all(bytes)?;
        file.write_all(b"\n")?;
        file.sync_all()?;
        drop(file);
        fs::rename(&temporary, destination)
    })();
    if let Err(error) = result {
        let _ = fs::remove_file(&temporary);
        return Err(format!(
            "could not commit report {}: {error}",
            destination.display()
        ));
    }
    Ok(())
}

fn wait_for_wake(shared: &Shared, timeout: Duration) {
    let Ok(guard) = shared.wake_lock.lock() else {
        thread::sleep(timeout.min(Duration::from_millis(250)));
        return;
    };
    match shared.wake.wait_timeout(guard, timeout) {
        Ok((_guard, _result)) => {}
        Err(_) => thread::sleep(timeout.min(Duration::from_millis(250))),
    }
}

fn set_error(shared: &Shared, message: &str) {
    if let Ok(mut error) = shared.last_error.lock() {
        error.clear();
        error.extend_from_slice(message.as_bytes());
        if error.len() > 4096 {
            error.truncate(4096);
        }
    }
}

unsafe fn engine_from_handle<'a>(handle: *mut c_void) -> Option<&'a Engine> {
    if handle.is_null() {
        None
    } else {
        Some(unsafe { &*(handle.cast::<Engine>()) })
    }
}

fn copy_to_caller(bytes: &[u8], buffer: *mut u8, capacity: usize, required: *mut usize) -> i32 {
    if required.is_null() {
        return STATUS_INVALID_ARGUMENT;
    }
    let needed = bytes.len().saturating_add(1);
    unsafe { *required = needed };
    if buffer.is_null() || capacity < needed {
        return STATUS_BUFFER_TOO_SMALL;
    }
    unsafe {
        ptr::copy_nonoverlapping(bytes.as_ptr(), buffer, bytes.len());
        *buffer.add(bytes.len()) = 0;
    }
    STATUS_OK
}

#[no_mangle]
pub extern "C" fn sd300_engine_abi_version() -> u32 {
    ABI_VERSION
}

#[no_mangle]
pub extern "C" fn sd300_engine_schema_version() -> u32 {
    SCHEMA_VERSION
}

#[no_mangle]
pub extern "C" fn sd300_engine_metadata(
    buffer: *mut u8,
    capacity: usize,
    required: *mut usize,
) -> i32 {
    let result = catch_unwind(AssertUnwindSafe(|| {
        let metadata = json!({
            "abi_version": ABI_VERSION,
            "schema_version": SCHEMA_VERSION,
            "product": "SD-300",
            "product_version": env!("CARGO_PKG_VERSION"),
            "target_os": std::env::consts::OS,
            "target_arch": std::env::consts::ARCH,
        });
        match serde_json::to_vec(&metadata) {
            Ok(bytes) => copy_to_caller(&bytes, buffer, capacity, required),
            Err(_) => STATUS_INTERNAL_ERROR,
        }
    }));
    result.unwrap_or(STATUS_PANIC)
}

#[no_mangle]
pub extern "C" fn sd300_engine_read_settings(
    buffer: *mut u8,
    capacity: usize,
    required: *mut usize,
) -> i32 {
    catch_unwind(AssertUnwindSafe(|| match sd_300::settings::read_json() {
        Ok(bytes) => copy_to_caller(&bytes, buffer, capacity, required),
        Err(_) => STATUS_INTERNAL_ERROR,
    }))
    .unwrap_or(STATUS_PANIC)
}

#[no_mangle]
pub extern "C" fn sd300_engine_write_settings(buffer: *const u8, length: usize) -> i32 {
    if buffer.is_null() || length == 0 || length > 256 * 1024 {
        return STATUS_INVALID_ARGUMENT;
    }
    catch_unwind(AssertUnwindSafe(|| {
        let bytes = unsafe { std::slice::from_raw_parts(buffer, length) };
        match sd_300::settings::write_json(bytes) {
            Ok(()) => STATUS_OK,
            Err(_) => STATUS_INTERNAL_ERROR,
        }
    }))
    .unwrap_or(STATUS_PANIC)
}

#[no_mangle]
pub extern "C" fn sd300_engine_set_launch_at_login(enabled: u32, start_hidden: u32) -> i32 {
    if enabled > 1 || start_hidden > 1 {
        return STATUS_INVALID_ARGUMENT;
    }
    catch_unwind(AssertUnwindSafe(
        || match sd_300::settings::set_launch_at_login(enabled == 1, start_hidden == 1) {
            Ok(()) => STATUS_OK,
            Err(_) => STATUS_INTERNAL_ERROR,
        },
    ))
    .unwrap_or(STATUS_PANIC)
}

#[no_mangle]
pub extern "C" fn sd300_engine_create(out_handle: *mut *mut c_void) -> i32 {
    if out_handle.is_null() {
        return STATUS_INVALID_ARGUMENT;
    }
    catch_unwind(AssertUnwindSafe(|| {
        let engine = Box::new(Engine::new());
        unsafe { *out_handle = Box::into_raw(engine).cast::<c_void>() };
        STATUS_OK
    }))
    .unwrap_or(STATUS_PANIC)
}

#[no_mangle]
pub extern "C" fn sd300_engine_start(handle: *mut c_void) -> i32 {
    catch_unwind(AssertUnwindSafe(|| unsafe {
        engine_from_handle(handle)
            .map(Engine::start)
            .unwrap_or(STATUS_INVALID_ARGUMENT)
    }))
    .unwrap_or(STATUS_PANIC)
}

#[no_mangle]
pub extern "C" fn sd300_engine_stop(handle: *mut c_void) -> i32 {
    catch_unwind(AssertUnwindSafe(|| unsafe {
        engine_from_handle(handle)
            .map(Engine::stop)
            .unwrap_or(STATUS_INVALID_ARGUMENT)
    }))
    .unwrap_or(STATUS_PANIC)
}

#[no_mangle]
pub extern "C" fn sd300_engine_destroy(handle: *mut c_void) -> i32 {
    if handle.is_null() {
        return STATUS_INVALID_ARGUMENT;
    }
    catch_unwind(AssertUnwindSafe(|| {
        unsafe { drop(Box::from_raw(handle.cast::<Engine>())) };
        STATUS_OK
    }))
    .unwrap_or(STATUS_PANIC)
}

#[no_mangle]
pub extern "C" fn sd300_engine_set_profile(handle: *mut c_void, profile: u32) -> i32 {
    catch_unwind(AssertUnwindSafe(|| unsafe {
        let Some(engine) = engine_from_handle(handle) else {
            return STATUS_INVALID_ARGUMENT;
        };
        let profile = match profile {
            0 => PROFILE_FOREGROUND,
            1 => PROFILE_HIDDEN,
            2 => PROFILE_OVERVIEW,
            3 => PROFILE_PROCESSES,
            _ => return STATUS_INVALID_ARGUMENT,
        };
        engine.shared.profile.store(profile, Ordering::Release);
        engine.shared.wake.notify_all();
        STATUS_OK
    }))
    .unwrap_or(STATUS_PANIC)
}

#[no_mangle]
pub extern "C" fn sd300_engine_set_process_sort(handle: *mut c_void, sort: u32) -> i32 {
    catch_unwind(AssertUnwindSafe(|| unsafe {
        let Some(engine) = engine_from_handle(handle) else {
            return STATUS_INVALID_ARGUMENT;
        };
        if process_sort_from_raw(sort).is_none() {
            return STATUS_INVALID_ARGUMENT;
        }
        let sort = u8::try_from(sort).unwrap_or(PROCESS_SORT_CPU);
        engine.shared.process_sort.store(sort, Ordering::Release);
        engine.shared.wake.notify_all();
        STATUS_OK
    }))
    .unwrap_or(STATUS_PANIC)
}

#[no_mangle]
pub extern "C" fn sd300_engine_request_driver_scan(handle: *mut c_void) -> i32 {
    catch_unwind(AssertUnwindSafe(|| unsafe {
        let Some(engine) = engine_from_handle(handle) else {
            return STATUS_INVALID_ARGUMENT;
        };
        engine.shared.driver_request.store(true, Ordering::Release);
        engine.shared.wake.notify_all();
        STATUS_OK
    }))
    .unwrap_or(STATUS_PANIC)
}

#[no_mangle]
pub extern "C" fn sd300_engine_request_export(handle: *mut c_void, kind: u32) -> i32 {
    catch_unwind(AssertUnwindSafe(|| unsafe {
        let Some(engine) = engine_from_handle(handle) else {
            return STATUS_INVALID_ARGUMENT;
        };
        let kind = match kind {
            1 => EXPORT_SNAPSHOT,
            2 => EXPORT_CAPABILITIES,
            _ => return STATUS_INVALID_ARGUMENT,
        };
        if !engine.shared.running.load(Ordering::Acquire) {
            return STATUS_NOT_RUNNING;
        }
        if engine
            .shared
            .export_request
            .compare_exchange(EXPORT_NONE, kind, Ordering::AcqRel, Ordering::Acquire)
            .is_err()
        {
            return STATUS_ALREADY_RUNNING;
        }
        if let (Ok(bytes), Ok(mut status)) = (
            serde_json::to_vec(&json!({ "state": "pending" })),
            engine.shared.export_status.lock(),
        ) {
            *status = bytes;
        }
        engine.shared.wake.notify_all();
        STATUS_OK
    }))
    .unwrap_or(STATUS_PANIC)
}

#[no_mangle]
pub extern "C" fn sd300_engine_read_export_status(
    handle: *mut c_void,
    buffer: *mut u8,
    capacity: usize,
    required: *mut usize,
) -> i32 {
    catch_unwind(AssertUnwindSafe(|| unsafe {
        let Some(engine) = engine_from_handle(handle) else {
            return STATUS_INVALID_ARGUMENT;
        };
        let Ok(status) = engine.shared.export_status.lock() else {
            return STATUS_INTERNAL_ERROR;
        };
        copy_to_caller(&status, buffer, capacity, required)
    }))
    .unwrap_or(STATUS_PANIC)
}

#[no_mangle]
pub extern "C" fn sd300_engine_topic_sequence(
    handle: *mut c_void,
    topic: u32,
    out_sequence: *mut u64,
) -> i32 {
    catch_unwind(AssertUnwindSafe(|| unsafe {
        if out_sequence.is_null() {
            return STATUS_INVALID_ARGUMENT;
        }
        let Some(engine) = engine_from_handle(handle) else {
            return STATUS_INVALID_ARGUMENT;
        };
        let Some(topic) = Topic::from_raw(topic) else {
            return STATUS_INVALID_TOPIC;
        };
        let Ok(topics) = engine.shared.topics.lock() else {
            return STATUS_INTERNAL_ERROR;
        };
        *out_sequence = topics[topic as usize].sequence;
        STATUS_OK
    }))
    .unwrap_or(STATUS_PANIC)
}

#[no_mangle]
pub extern "C" fn sd300_engine_read_topic(
    handle: *mut c_void,
    topic: u32,
    after_sequence: u64,
    buffer: *mut u8,
    capacity: usize,
    required: *mut usize,
    out_sequence: *mut u64,
) -> i32 {
    catch_unwind(AssertUnwindSafe(|| unsafe {
        if out_sequence.is_null() {
            return STATUS_INVALID_ARGUMENT;
        }
        let Some(engine) = engine_from_handle(handle) else {
            return STATUS_INVALID_ARGUMENT;
        };
        let Some(topic) = Topic::from_raw(topic) else {
            return STATUS_INVALID_TOPIC;
        };
        let Ok(topics) = engine.shared.topics.lock() else {
            return STATUS_INTERNAL_ERROR;
        };
        let state = &topics[topic as usize];
        *out_sequence = state.sequence;
        if state.sequence == 0 || state.sequence == after_sequence {
            if !required.is_null() {
                *required = 0;
            }
            return STATUS_UNCHANGED;
        }
        copy_to_caller(&state.json, buffer, capacity, required)
    }))
    .unwrap_or(STATUS_PANIC)
}

#[no_mangle]
pub extern "C" fn sd300_engine_read_fast_summary(
    handle: *mut c_void,
    out_summary: *mut FastSummary,
) -> i32 {
    catch_unwind(AssertUnwindSafe(|| unsafe {
        if out_summary.is_null() {
            return STATUS_INVALID_ARGUMENT;
        }
        let Some(engine) = engine_from_handle(handle) else {
            return STATUS_INVALID_ARGUMENT;
        };
        let Ok(summary) = engine.shared.summary.lock() else {
            return STATUS_INTERNAL_ERROR;
        };
        *out_summary = *summary;
        if summary.sequence == 0 {
            STATUS_UNCHANGED
        } else {
            STATUS_OK
        }
    }))
    .unwrap_or(STATUS_PANIC)
}

#[no_mangle]
pub extern "C" fn sd300_engine_read_tray_summary(
    handle: *mut c_void,
    out_summary: *mut TraySummary,
) -> i32 {
    catch_unwind(AssertUnwindSafe(|| unsafe {
        if out_summary.is_null() {
            return STATUS_INVALID_ARGUMENT;
        }
        let Some(engine) = engine_from_handle(handle) else {
            return STATUS_INVALID_ARGUMENT;
        };
        let Ok(summary) = engine.shared.tray_summary.lock() else {
            return STATUS_INTERNAL_ERROR;
        };
        *out_summary = *summary;
        if summary.sequence == 0 {
            STATUS_UNCHANGED
        } else {
            STATUS_OK
        }
    }))
    .unwrap_or(STATUS_PANIC)
}

#[no_mangle]
pub extern "C" fn sd300_engine_read_process_summary(
    handle: *mut c_void,
    after_sequence: u64,
    out_summary: *mut ProcessSummary,
) -> i32 {
    catch_unwind(AssertUnwindSafe(|| unsafe {
        if out_summary.is_null() {
            return STATUS_INVALID_ARGUMENT;
        }
        let Some(engine) = engine_from_handle(handle) else {
            return STATUS_INVALID_ARGUMENT;
        };
        let Ok(summary) = engine.shared.process_summary.lock() else {
            return STATUS_INTERNAL_ERROR;
        };
        if summary.sequence == 0 || summary.sequence == after_sequence {
            return STATUS_UNCHANGED;
        }
        *out_summary = *summary;
        STATUS_OK
    }))
    .unwrap_or(STATUS_PANIC)
}

#[no_mangle]
pub extern "C" fn sd300_engine_last_error(
    handle: *mut c_void,
    buffer: *mut c_char,
    capacity: usize,
    required: *mut usize,
) -> i32 {
    catch_unwind(AssertUnwindSafe(|| unsafe {
        let Some(engine) = engine_from_handle(handle) else {
            return STATUS_INVALID_ARGUMENT;
        };
        let Ok(error) = engine.shared.last_error.lock() else {
            return STATUS_INTERNAL_ERROR;
        };
        copy_to_caller(&error, buffer.cast::<u8>(), capacity, required)
    }))
    .unwrap_or(STATUS_PANIC)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn metadata_uses_caller_owned_buffers() {
        let mut required = 0usize;
        assert_eq!(
            sd300_engine_metadata(ptr::null_mut(), 0, &mut required),
            STATUS_BUFFER_TOO_SMALL
        );
        assert!(required > 1);
        let mut buffer = vec![0u8; required];
        assert_eq!(
            sd300_engine_metadata(buffer.as_mut_ptr(), buffer.len(), &mut required),
            STATUS_OK
        );
        let metadata: serde_json::Value =
            serde_json::from_slice(&buffer[..required - 1]).expect("valid metadata JSON");
        assert_eq!(metadata["abi_version"], ABI_VERSION);
        assert_eq!(metadata["schema_version"], SCHEMA_VERSION);
        assert_eq!(metadata["product_version"], "3.0.0");
    }

    #[test]
    fn settings_write_rejects_invalid_caller_input() {
        assert_eq!(
            sd300_engine_write_settings(ptr::null(), 0),
            STATUS_INVALID_ARGUMENT
        );
        let invalid = b"{not-json";
        assert_eq!(
            sd300_engine_write_settings(invalid.as_ptr(), invalid.len()),
            STATUS_INTERNAL_ERROR
        );
    }

    #[test]
    fn invalid_topic_is_rejected_without_touching_the_buffer() {
        let engine = Engine::new();
        let handle = (&engine as *const Engine).cast_mut().cast::<c_void>();
        let mut sequence = 7;
        let mut required = 7;
        assert_eq!(
            sd300_engine_read_topic(
                handle,
                99,
                0,
                ptr::null_mut(),
                0,
                &mut required,
                &mut sequence,
            ),
            STATUS_INVALID_TOPIC
        );
        assert_eq!(sequence, 7);
        assert_eq!(required, 7);
    }

    #[test]
    fn typed_topic_envelope_preserves_the_versioned_json_contract() {
        let shared = Shared::default();
        let warnings = [DiagnosticWarning {
            source: "Test".into(),
            message: "bounded warning".into(),
            severity: WarningSeverity::Warning,
        }];
        let data = ["first", "second"];

        publish(&shared, Topic::Warnings, &data, &warnings);

        let topics = shared.topics.lock().expect("topic lock");
        let state = &topics[Topic::Warnings as usize];
        let envelope: serde_json::Value =
            serde_json::from_slice(&state.json).expect("valid topic JSON");
        assert_eq!(envelope["schema_version"], SCHEMA_VERSION);
        assert_eq!(envelope["product_version"], "3.0.0");
        assert_eq!(envelope["target"], target_label());
        assert_eq!(envelope["topic"], "warnings");
        assert_eq!(envelope["sequence"], 1);
        assert_eq!(envelope["freshness_ms"], 0);
        assert_eq!(envelope["availability"], "available");
        assert_eq!(envelope["warnings"][0]["source"], "Test");
        assert_eq!(envelope["data"], serde_json::json!(["first", "second"]));
    }

    #[test]
    fn fast_summary_layout_is_fixed_for_the_zig_boundary() {
        assert_eq!(std::mem::size_of::<FastSummary>(), 48);
        assert_eq!(std::mem::align_of::<FastSummary>(), 8);
        assert_eq!(std::mem::size_of::<TraySummary>(), 32);
        assert_eq!(std::mem::align_of::<TraySummary>(), 8);
        assert_eq!(std::mem::size_of::<ProcessRowSummary>(), 264);
        assert_eq!(std::mem::align_of::<ProcessRowSummary>(), 8);
        assert_eq!(std::mem::size_of::<ProcessSummary>(), 4256);
        assert_eq!(std::mem::align_of::<ProcessSummary>(), 8);
    }

    #[test]
    fn process_summary_is_latest_only_and_caller_owned() {
        let engine = Engine::new();
        let handle = (&engine as *const Engine).cast_mut().cast::<c_void>();
        let mut source = ProcessSummary {
            sequence: 11,
            captured_unix_ms: 22,
            total_count: 3,
            total_threads: 4,
            row_count: 1,
            ..ProcessSummary::default()
        };
        source.rows[0].pid = 42;
        source.rows[0].name_len = copy_summary_text(&mut source.rows[0].name, "native.exe");
        *engine
            .shared
            .process_summary
            .lock()
            .expect("process summary lock") = source;

        let mut destination = ProcessSummary::default();
        assert_eq!(
            sd300_engine_read_process_summary(handle, 0, &mut destination),
            STATUS_OK
        );
        assert_eq!(destination.sequence, 11);
        assert_eq!(destination.rows[0].pid, 42);
        assert_eq!(
            &destination.rows[0].name[..destination.rows[0].name_len as usize],
            b"native.exe"
        );
        assert_eq!(
            sd300_engine_read_process_summary(handle, 11, &mut destination),
            STATUS_UNCHANGED
        );
    }

    #[test]
    fn supported_profiles_are_accepted_and_unknown_profiles_fail_closed() {
        let engine = Engine::new();
        let handle = (&engine as *const Engine).cast_mut().cast::<c_void>();

        assert_eq!(sd300_engine_set_profile(handle, 2), STATUS_OK);
        assert_eq!(
            engine.shared.profile.load(Ordering::Acquire),
            PROFILE_OVERVIEW
        );
        assert_eq!(sd300_engine_set_profile(handle, 3), STATUS_OK);
        assert_eq!(
            engine.shared.profile.load(Ordering::Acquire),
            PROFILE_PROCESSES
        );
        assert_eq!(
            sd300_engine_set_profile(handle, u32::MAX),
            STATUS_INVALID_ARGUMENT
        );
        assert_eq!(
            engine.shared.profile.load(Ordering::Acquire),
            PROFILE_PROCESSES
        );
    }

    #[test]
    fn process_sort_setter_maps_all_keys_and_rejects_unknown_values() {
        let engine = Engine::new();
        let handle = (&engine as *const Engine).cast_mut().cast::<c_void>();
        let cases = [
            (0, ProcessSortKey::Cpu),
            (1, ProcessSortKey::Memory),
            (2, ProcessSortKey::Pid),
            (3, ProcessSortKey::Name),
        ];

        for (raw, expected) in cases {
            assert_eq!(sd300_engine_set_process_sort(handle, raw), STATUS_OK);
            assert_eq!(selected_process_sort(&engine.shared), expected);
        }
        assert_eq!(
            sd300_engine_set_process_sort(handle, u32::MAX),
            STATUS_INVALID_ARGUMENT
        );
        assert_eq!(selected_process_sort(&engine.shared), ProcessSortKey::Name);
        assert_eq!(
            sd300_engine_set_process_sort(ptr::null_mut(), 0),
            STATUS_INVALID_ARGUMENT
        );
    }

    #[test]
    fn export_status_is_caller_owned_and_requests_fail_closed() {
        let engine = Engine::new();
        let handle = (&engine as *const Engine).cast_mut().cast::<c_void>();
        let mut required = 0;
        assert_eq!(
            sd300_engine_read_export_status(handle, ptr::null_mut(), 0, &mut required),
            STATUS_BUFFER_TOO_SMALL
        );
        let mut buffer = vec![0; required];
        assert_eq!(
            sd300_engine_read_export_status(
                handle,
                buffer.as_mut_ptr(),
                buffer.len(),
                &mut required
            ),
            STATUS_OK
        );
        let status: serde_json::Value =
            serde_json::from_slice(&buffer[..required - 1]).expect("valid export status");
        assert_eq!(status["state"], "idle");
        assert_eq!(
            sd300_engine_request_export(handle, u32::MAX),
            STATUS_INVALID_ARGUMENT
        );
        assert_eq!(sd300_engine_request_export(handle, 1), STATUS_NOT_RUNNING);
    }

    #[cfg(unix)]
    #[test]
    fn export_directory_and_report_are_private_to_the_current_user() {
        use std::os::unix::fs::PermissionsExt;

        let temporary = tempfile::tempdir().expect("temporary root");
        let application = temporary.path().join("sd300");
        let reports = application.join("reports");

        fs::create_dir(&application).expect("application directory");
        fs::set_permissions(&application, fs::Permissions::from_mode(0o755))
            .expect("relax application directory before the test");
        ensure_export_directory(&reports).expect("private reports directory");

        assert_eq!(
            fs::metadata(&application)
                .expect("application metadata")
                .permissions()
                .mode()
                & 0o777,
            0o700
        );
        assert_eq!(
            fs::metadata(&reports)
                .expect("reports metadata")
                .permissions()
                .mode()
                & 0o777,
            0o700
        );

        let report = reports.join("snapshot.json");
        write_export_atomically(&report, br#"{"redacted":true}"#).expect("private atomic report");
        assert_eq!(
            fs::metadata(report)
                .expect("report metadata")
                .permissions()
                .mode()
                & 0o777,
            0o600
        );
    }
}
