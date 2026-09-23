//! Prepared terminal presentation. Inventory work never happens inside a draw call.
use crate::{
    app::App,
    observation::Observation,
    types::{DiagnosticMode, ProcessSortKey, Section},
    ui::common::{format_bytes, format_temp, format_throughput},
};
use serde_json::{json, Value};

#[derive(Default)]
pub struct Presentation {
    pub rows: Vec<ViewRow>,
    pub selected: usize,
    pub selected_id: Option<String>,
    pub summary: Vec<String>,
    pub inspector: Vec<String>,
    pub plot: Vec<Option<f64>>,
    pub plot_title: String,
    pub plot_max: Option<f64>,
    pub secondary_plot: Vec<Option<f64>>,
    pub secondary_title: String,
    pub secondary_max: Option<f64>,
    pub total: usize,
    pub status: String,
    pub selection_lost: bool,
}
pub struct ViewRow {
    pub id: String,
    pub name: String,
    pub value: String,
    pub state: String,
    pub target: Target,
}
#[derive(Clone, Copy)]
pub enum Target {
    Process(usize),
    Cpu(usize),
    Module(usize),
    Partition(usize),
    Drive(usize),
    Activity(usize),
    Gpu(usize),
    Interface(usize),
    NetworkTotal,
    Connection(usize),
    Sensor(usize),
    Fan(usize),
    Battery,
    Device(usize),
    Service(usize),
    DriverObservation(usize),
    Finding(usize),
    System,
    Displays,
    Memory,
    Diagnostics,
}

fn observation(value: &Observation) -> String {
    if value.is_available() {
        value.source.clone()
    } else {
        format!(
            "{}: {}",
            match value.status {
                crate::observation::ObservationStatus::PermissionDenied =>
                    "Permission denied / Administrator may be required",
                crate::observation::ObservationStatus::Unsupported => "Unsupported",
                crate::observation::ObservationStatus::Error => "Provider error",
                crate::observation::ObservationStatus::Contradictory => "Contradictory",
                _ => "Unavailable",
            },
            value.detail.as_deref().unwrap_or(&value.source)
        )
    }
}
fn percentage(value: Option<f64>) -> String {
    value
        .map(|v| format!("{v:.1}%"))
        .unwrap_or_else(|| "Unavailable".into())
}
fn rate(value: Option<f64>) -> String {
    value
        .map(|v| format_throughput(v.max(0.0) as u64))
        .unwrap_or_else(|| "Warming up / unavailable".into())
}
fn fresh(app: &App, lane: &str) -> bool {
    app.snapshot.samples.get(lane).is_some_and(|s| {
        s.observation.is_available()
            && s.sequence > 0
            && app.presentation_time_ms.saturating_sub(s.captured_unix_ms)
                <= s.expected_interval_ms.saturating_mul(3).max(3000)
    })
}
impl Presentation {
    pub fn prepare(app: &App) -> Self {
        let s = &app.snapshot;
        let fast = fresh(app, "fast");
        let slow = fresh(app, "slow");
        let cpu = percentage(fast.then_some(s.cpu.total_usage as f64));
        let mem = percentage((fast && s.memory.total_bytes > 0).then(|| s.memory.usage_percent()));
        let net = rate(
            (fast && s.network.sample.observation.is_available())
                .then_some(s.network.total_download_rate as f64),
        );
        let mut v = Self {
            selected_id: app.view.selected_id.clone(),
            selected: app.view.selected,
            ..Default::default()
        };
        let mut add = |id: String, name: String, value: String, state: String, target| {
            v.rows.push(ViewRow {
                id,
                name,
                value,
                state,
                target,
            })
        };
        let lane = match app.current_section {
            Section::Overview => {
                v.summary = vec![
                    format!("Processor {cpu}   Memory {mem}   Download {net}"),
                    format!(
                        "Graphics {}   CPU temperature {}",
                        percentage(slow.then(|| s.gpu.utilization()).flatten().map(f64::from)),
                        s.thermals
                            .cpu_temp
                            .map(|t| format_temp(t, app.temp_unit))
                            .unwrap_or_else(|| observation(&s.thermals.cpu_temperature_status))
                    ),
                ];
                for (i, f) in app.findings.iter().enumerate() {
                    add(
                        f.id.clone(),
                        f.title.clone(),
                        f.severity.clone(),
                        f.evidence.clone(),
                        Target::Finding(i),
                    );
                }
                if app.findings.is_empty() {
                    add(
                        "findings:empty".into(),
                        "No current findings".into(),
                        "Observing".into(),
                        "This does not certify hardware health".into(),
                        Target::System,
                    );
                }
                add(
                    "system".into(),
                    "Your computer".into(),
                    s.system.os_name.clone(),
                    s.cpu.cpu_model.clone(),
                    Target::System,
                );
                add(
                    "displays".into(),
                    "Displays".into(),
                    observation(&s.displays.inventory_status),
                    observation(&s.displays.brightness_status),
                    Target::Displays,
                );
                "fast"
            }
            Section::Cpu => {
                v.summary = vec![
                    format!(
                        "Processor {cpu}   {} physical cores / {} logical processors",
                        s.cpu.core_count, s.cpu.thread_count
                    ),
                    s.cpu.cpu_model.clone(),
                ];
                for (i, usage) in s.cpu.per_core_usage.iter().enumerate() {
                    add(
                        format!("cpu:{i}"),
                        format!("Logical processor {}", i + 1),
                        percentage(fast.then_some(*usage as f64)),
                        s.cpu
                            .per_core_frequency
                            .get(i)
                            .filter(|f| **f > 0)
                            .map(|f| format!("{f} MHz"))
                            .unwrap_or_else(|| "Frequency unavailable".into()),
                        Target::Cpu(i),
                    );
                }
                "fast"
            }
            Section::Memory => {
                v.summary = vec![
                    format!(
                        "Memory {mem}   {} used / {} total",
                        format_bytes(s.memory.used_bytes),
                        format_bytes(s.memory.total_bytes)
                    ),
                    format!(
                        "Swap {} / {}   Modules: {}",
                        format_bytes(s.memory.swap_used_bytes),
                        format_bytes(s.memory.swap_total_bytes),
                        observation(&s.memory.module_status)
                    ),
                ];
                add(
                    "memory".into(),
                    "System memory".into(),
                    mem,
                    "Used / available / swap".into(),
                    Target::Memory,
                );
                for (i, m) in s.memory.modules.iter().enumerate() {
                    add(
                        format!("module:{}", m.locator.as_deref().unwrap_or(&i.to_string())),
                        m.locator
                            .clone()
                            .unwrap_or_else(|| format!("Module {}", i + 1)),
                        format_bytes(m.capacity_bytes),
                        m.configured_speed_mt_s
                            .map(|v| format!("{v} MT/s"))
                            .unwrap_or_else(|| "Speed unavailable".into()),
                        Target::Module(i),
                    );
                }
                "fast"
            }
            Section::Disk => {
                let totals = fresh(app, "activity")
                    .then(|| s.disk_activity.totals())
                    .flatten();
                v.summary = vec![
                    format!(
                        "Read {}   Write {}",
                        rate(totals.map(|v| v.0)),
                        rate(totals.map(|v| v.1))
                    ),
                    format!(
                        "Health: {} · A: optional read for selected drive",
                        observation(&s.disk_health.health_status)
                    ),
                ];
                for (i, d) in s.disk.partitions.iter().enumerate() {
                    add(
                        format!("volume:{}:{}", d.mount_point, d.name),
                        d.mount_point.clone(),
                        format!("{:.1}% used", d.usage_percent()),
                        format!(
                            "{} free / {} · {}",
                            format_bytes(d.available_bytes),
                            format_bytes(d.total_bytes),
                            d.filesystem
                        ),
                        Target::Partition(i),
                    );
                }
                for (i, d) in s.disk_health.drives.iter().enumerate() {
                    add(
                        format!("drive:{}", d.device_id),
                        d.model.clone(),
                        d.health_status.user_label().into(),
                        d.device_id.clone(),
                        Target::Drive(i),
                    );
                }
                for (i, d) in s.disk_activity.devices.iter().enumerate() {
                    add(
                        format!("io:{}", d.identity),
                        format!("Activity {}", d.device_id),
                        rate(d.read_bytes_per_sec),
                        format!("Write {}", rate(d.write_bytes_per_sec)),
                        Target::Activity(i),
                    );
                }
                "activity"
            }
            Section::Gpu => {
                v.summary = vec![
                    format!(
                        "Graphics inventory: {}",
                        observation(&s.gpu.inventory_status)
                    ),
                    format!("Live fields: {}", observation(&s.gpu.telemetry_status)),
                ];
                for (i, g) in s.gpu.adapters.iter().enumerate() {
                    add(
                        if g.device_id.is_empty() {
                            format!("unidentified-gpu:{i}")
                        } else {
                            format!("gpu:{}", g.device_id)
                        },
                        g.name.clone(),
                        percentage(
                            slow.then_some(g.utilization_percent)
                                .flatten()
                                .map(f64::from),
                        ),
                        format!(
                            "{} · {}",
                            g.temperature_celsius
                                .map(|t| format_temp(t, app.temp_unit))
                                .unwrap_or_else(|| "Temperature unavailable".into()),
                            if g.unified_memory == Some(true) {
                                "Unified system memory".into()
                            } else {
                                g.dedicated_memory_mb
                                    .map(|m| format!("{m} MiB dedicated"))
                                    .unwrap_or_else(|| "Dedicated capacity unavailable".into())
                            }
                        ),
                        Target::Gpu(i),
                    );
                }
                "slow"
            }
            Section::Network => {
                v.summary = vec![
                    format!(
                        "Download {net}   Upload {}",
                        rate(
                            (fast && s.network.sample.observation.is_available())
                                .then_some(s.network.total_upload_rate as f64)
                        )
                    ),
                    format!(
                        "{} interfaces · {} sockets{} · Enter inspects evidence",
                        s.network.interfaces.len(),
                        s.network_diag.active_connections.len(),
                        if s.network_diag.connections_observation.is_available() {
                            ""
                        } else {
                            " (incomplete)"
                        }
                    ),
                ];
                if let Some(detail) = &s.network_diag.connections_observation.detail {
                    v.summary.push(detail.clone());
                }
                if let Some(detail) = &s.network.sample.observation.detail {
                    v.summary.push(detail.clone());
                }
                v.summary.push(s.network.aggregation.clone());
                add(
                    "network-total".into(),
                    "Aggregate scope".into(),
                    observation(&s.network.sample.observation),
                    s.network.aggregation.clone(),
                    Target::NetworkTotal,
                );
                add(
                    "diagnostics".into(),
                    "Connectivity evidence".into(),
                    if fresh(app, "diagnostics") {
                        "Captured"
                    } else {
                        "Pending / stale"
                    }
                    .into(),
                    "ICMP failure alone does not establish an outage".into(),
                    Target::Diagnostics,
                );
                for (i, n) in s.network.interfaces.iter().enumerate() {
                    add(
                        format!("net:{}:{}", n.name, n.mac_address),
                        n.name.clone(),
                        rate(
                            (fast && n.rate_status.is_available())
                                .then_some(n.download_rate as f64),
                        ),
                        format!(
                            "{} · upload {}",
                            n.operational_state,
                            rate(
                                (fast && n.rate_status.is_available())
                                    .then_some(n.upload_rate as f64)
                            )
                        ),
                        Target::Interface(i),
                    );
                }
                for (i, c) in s.network_diag.active_connections.iter().enumerate() {
                    add(
                        format!(
                            "{}:{}:{}:{}:{}:{:?}",
                            c.protocol,
                            c.local_addr,
                            c.local_port,
                            c.remote_addr,
                            c.remote_port,
                            c.pid
                        ),
                        c.process_name
                            .clone()
                            .unwrap_or_else(|| c.protocol.to_string()),
                        format!("{}:{}", c.remote_addr, c.remote_port),
                        c.state.to_string(),
                        Target::Connection(i),
                    );
                }
                "fast"
            }
            Section::Processes => {
                v.summary = vec![
                    format!(
                        "{} processes · {} matching · CPU 100% = one logical processor",
                        s.processes.total_count,
                        app.view.rows.len()
                    ),
                    format!(
                        "Sort: {:?} {} · c CPU / M memory / n name / p PID / s reverse",
                        app.process_sort,
                        if app.sort_reversed {
                            "reversed"
                        } else if matches!(
                            app.process_sort,
                            ProcessSortKey::Cpu | ProcessSortKey::Memory
                        ) {
                            "descending"
                        } else {
                            "ascending"
                        }
                    ),
                ];
                let mut indices: Vec<_> = (0..s.processes.list.len()).collect();
                indices.sort_unstable_by(|&a, &b| {
                    let (a, b) = (&s.processes.list[a], &s.processes.list[b]);
                    let order = match app.process_sort {
                        ProcessSortKey::Cpu => b
                            .cpu_observation
                            .is_available()
                            .cmp(&a.cpu_observation.is_available())
                            .then_with(|| b.cpu_percent.total_cmp(&a.cpu_percent)),
                        ProcessSortKey::Memory => b
                            .memory_observation
                            .is_available()
                            .cmp(&a.memory_observation.is_available())
                            .then_with(|| b.memory_bytes.cmp(&a.memory_bytes)),
                        ProcessSortKey::Name => a.name.cmp(&b.name),
                        ProcessSortKey::Pid => a.pid.cmp(&b.pid),
                    }
                    .then_with(|| a.pid.cmp(&b.pid));
                    if app.sort_reversed {
                        order.reverse()
                    } else {
                        order
                    }
                });
                for i in indices {
                    let p = &s.processes.list[i];
                    add(
                        format!("process:{}:{:?}", p.pid, p.start_time_unix_ms),
                        if app.mode == Some(DiagnosticMode::User) {
                            p.friendly_name.clone()
                        } else {
                            p.name.clone()
                        },
                        format!(
                            "{} CPU",
                            percentage(
                                (fast && p.cpu_observation.is_available())
                                    .then_some(p.cpu_percent as f64)
                            )
                        ),
                        format!(
                            "{} · PID {}",
                            if p.memory_observation.is_available() {
                                format_bytes(p.memory_bytes)
                            } else {
                                "Memory unavailable".into()
                            },
                            p.pid
                        ),
                        Target::Process(i),
                    );
                }
                "fast"
            }
            Section::Thermals => {
                v.summary = vec![
                    format!(
                        "CPU {}",
                        s.thermals
                            .cpu_temp
                            .map(|t| format_temp(t, app.temp_unit))
                            .unwrap_or_else(|| observation(&s.thermals.cpu_temperature_status))
                    ),
                    format!(
                        "Graphics {} · Fans {}",
                        s.thermals
                            .gpu_temp
                            .map(|t| format_temp(t, app.temp_unit))
                            .unwrap_or_else(|| observation(&s.thermals.gpu_temperature_status)),
                        observation(&s.thermals.fan_status)
                    ),
                ];
                for (i, t) in s.thermals.sensors.iter().enumerate() {
                    add(
                        t.device_id
                            .clone()
                            .unwrap_or_else(|| format!("unidentified-sensor:{i}")),
                        t.label.clone(),
                        format_temp(t.temperature, app.temp_unit),
                        t.source.clone(),
                        Target::Sensor(i),
                    );
                }
                for (i, f) in s.thermals.fans.iter().enumerate() {
                    add(
                        f.device_id
                            .clone()
                            .unwrap_or_else(|| format!("unidentified-fan:{i}")),
                        f.label.clone(),
                        format!("{} RPM", f.rpm),
                        f.source.clone(),
                        Target::Fan(i),
                    );
                }
                if let Some(b) = &s.thermals.battery {
                    add(
                        "battery".into(),
                        "Battery".into(),
                        format!("{:.0}%", b.percent),
                        if b.is_charging {
                            "Charging"
                        } else if b.is_on_ac {
                            "AC power"
                        } else {
                            "Battery power"
                        }
                        .into(),
                        Target::Battery,
                    );
                }
                "slow"
            }
            Section::Drivers => {
                v.summary = vec![
                    format!(
                        "Devices: {:?} · {} need attention",
                        s.drivers.scan_status,
                        s.drivers.attention_devices().count()
                    ),
                    "r retries discovery. Read-only inspection; no driver changes.".into(),
                ];
                for (i, d) in s.drivers.devices().enumerate() {
                    add(
                        format!("device:{:?}:{}:{i}", d.category, d.name),
                        d.name.clone(),
                        d.status.user_description().into(),
                        d.category.label().into(),
                        Target::Device(i),
                    );
                }
                for (i, d) in s.drivers.services.iter().enumerate() {
                    add(
                        format!("service:{}", d.name),
                        d.display_name.clone(),
                        d.display_state().into(),
                        d.name.clone(),
                        Target::Service(i),
                    );
                }
                for (i, observation) in s.drivers.observations.iter().enumerate() {
                    add(
                        format!("driver-provider:{}", observation.source),
                        observation.source.clone(),
                        format!("{:?}", observation.status),
                        observation
                            .detail
                            .clone()
                            .unwrap_or_else(|| "Inventory query completed".into()),
                        Target::DriverObservation(i),
                    );
                }
                "drivers"
            }
        };
        v.total = v.rows.len();
        if !app.filter.is_empty() {
            let filter = app.filter.to_lowercase();
            v.rows.retain(|r| {
                r.name.to_lowercase().contains(&filter)
                    || r.value.to_lowercase().contains(&filter)
                    || r.state.to_lowercase().contains(&filter)
            });
        }
        if app.current_section == Section::Processes {
            v.summary[0] = format!(
                "{} processes · {} matching · CPU 100% = one logical processor",
                s.processes.total_count,
                v.rows.len()
            );
        }
        v.selection_lost = v
            .selected_id
            .as_ref()
            .is_some_and(|id| id.starts_with("process:") && !v.rows.iter().any(|r| &r.id == id));
        v.selected = v
            .selected_id
            .as_ref()
            .and_then(|id| v.rows.iter().position(|r| &r.id == id))
            .unwrap_or(v.selected.min(v.rows.len().saturating_sub(1)));
        if !v.selection_lost {
            v.selected_id = v.rows.get(v.selected).map(|r| r.id.clone());
        }
        if v.selection_lost {
            v.inspector.push("The selected process ended or is no longer visible. Select another row to inspect it.".into());
        } else if let Some(row) = v.rows.get(v.selected) {
            v.inspector.push(row.name.clone());
            v.inspector.push(format!("{} · {}", row.value, row.state));
            v.inspector.push(String::new());
            if app.mode == Some(DiagnosticMode::Technician) {
                let data = match row.target {
                    Target::Process(i) => {
                        let process = &s.processes.list[i];
                        let mut value = json!(process);
                        if !fast || !process.cpu_observation.is_available() {
                            value["cpu_percent"] = serde_json::Value::Null;
                        }
                        if !fast || !process.memory_observation.is_available() {
                            value["memory_bytes"] = serde_json::Value::Null;
                            value["memory_percent"] = serde_json::Value::Null;
                        }
                        value
                    }
                    Target::Cpu(i) => {
                        json!({"logical_processor":i,"usage_percent":if fast {Some(s.cpu.per_core_usage[i])}else{None},"frequency_mhz":s.cpu.per_core_frequency.get(i)})
                    }
                    Target::Module(i) => json!(s.memory.modules[i]),
                    Target::Partition(i) => json!(s.disk.partitions[i]),
                    Target::Drive(i) => json!(s.disk_health.drives[i]),
                    Target::Activity(i) => json!(s.disk_activity.devices[i]),
                    Target::Gpu(i) => json!(s.gpu.adapters[i]),
                    Target::Interface(i) => {
                        let interface = &s.network.interfaces[i];
                        let mut value = json!({"interface":interface,"adapter_capabilities":s.network.adapters});
                        if !fast || !interface.counter_status.is_available() {
                            value["interface"]["received_bytes"] = serde_json::Value::Null;
                            value["interface"]["transmitted_bytes"] = serde_json::Value::Null;
                        }
                        if !fast || !interface.rate_status.is_available() {
                            value["interface"]["download_rate"] = serde_json::Value::Null;
                            value["interface"]["upload_rate"] = serde_json::Value::Null;
                        }
                        value
                    }
                    Target::NetworkTotal => {
                        json!({"scope":s.network.aggregation,"sample":s.network.sample})
                    }
                    Target::Connection(i) => json!(s.network_diag.active_connections[i]),
                    Target::Sensor(i) => json!(s.thermals.sensors[i]),
                    Target::Fan(i) => json!(s.thermals.fans[i]),
                    Target::Battery => json!(s.thermals.battery),
                    Target::Device(i) => json!(s.drivers.devices().nth(i)),
                    Target::Service(i) => json!(s.drivers.services[i]),
                    Target::DriverObservation(i) => json!(s.drivers.observations[i]),
                    Target::Finding(i) => json!(app.findings[i]),
                    Target::System => json!(s.system),
                    Target::Displays => json!(s.displays),
                    Target::Memory => json!(s.memory),
                    Target::Diagnostics => json!(s.network_diag),
                };
                flatten(&mut v.inspector, "", &data, 0);
            } else {
                v.inspector.push(match row.target {
                    Target::Process(_)=>"CPU describes processor demand. Memory describes the space used by this application. Missing readings may require additional permissions.",
                    Target::Cpu(_)=>"A busy logical processor shows work being performed. Sustained demand can explain slowness; usage alone is not evidence of hardware failure.",
                    Target::Gpu(_)=>"Temperature, load and memory are separate readings. A missing field does not mean the graphics adapter is idle. Shared memory belongs to the system.",
                    Target::Drive(_)=>"Health comes from the drive or operating-system provider. Inspect reported warnings and verify backups before further testing.",
                    Target::Diagnostics=>"These lightweight checks provide limited connectivity evidence. ICMP can be filtered even when internet access works. DNS duration is separate from network round-trip time.",
                    Target::NetworkTotal=>"The scope above explains which interface rates contribute to the total. Inspect individual interfaces for virtual or tunnel traffic; this is not a bandwidth test.",
                    Target::Sensor(_)|Target::Fan(_)=>"This reading belongs to a specific provider channel. Temperature and fan speed availability depend on the hardware and its driver.",
                    Target::Device(_)=>"Detection establishes that the operating system lists a device. Unknown health is not a fault; inspect the discovery details below.",
                    Target::Service(_)=>"Optional services can be absent or run only when needed. An unreadable service state is not a measured stopped service.",
                    Target::DriverObservation(_)=>"This describes what a discovery provider could observe. Other available readings remain useful when one provider is limited.",
                    _=>"Press m and choose Technician for units, provider identifiers and detailed observations."
                }.into());
                match row.target {
                    Target::Device(i) => {
                        if let Some(device) = s.drivers.devices().nth(i) {
                            v.inspector.push(device.extra.clone());
                        }
                    }
                    Target::Service(i) => {
                        if let Some(detail) = &s.drivers.services[i].observation.detail {
                            v.inspector.push(detail.clone());
                        }
                    }
                    _ => {}
                }
            }
            if let Target::Finding(i) = row.target {
                let f = &app.findings[i];
                v.inspector.push(format!("Evidence: {}", f.evidence));
                v.inspector.push(format!("Next step: {}", f.next_step));
            }
        } else {
            v.inspector.push(
                if app.filter.is_empty() {
                    "No rows are available yet. Inspect the provider state and press r to retry."
                } else {
                    "No rows match. Press / to edit the filter or Esc to clear it."
                }
                .into(),
            );
        }
        v.status = s
            .samples
            .get(lane)
            .map(|m| {
                format!(
                    "{} · sample {} · {} ms interval · {} ms old",
                    observation(&m.observation),
                    m.sequence,
                    m.interval_ms,
                    app.presentation_time_ms.saturating_sub(m.captured_unix_ms)
                )
            })
            .unwrap_or_else(|| format!("{lane} · awaiting first capture"));
        let (history, title, step, max) = match app.current_section {
            Section::Memory => (&app.mem_history, "Memory %", 1000, Some(100.0)),
            Section::Network => (&app.net_down_history, "Download bytes/s", 1000, None),
            Section::Gpu => (&app.gpu_history, "Graphics load %", 5000, Some(100.0)),
            Section::Disk => (
                &app.disk_read_history,
                "Physical disk read bytes/s",
                1000,
                None,
            ),
            Section::Thermals => (&app.temp_history, "CPU temperature °C", 5000, None),
            _ => (&app.cpu_history, "Processor %", 1000, Some(100.0)),
        };
        v.plot = history.timeline(app.presentation_time_ms, step, 120);
        v.plot_title = format!("{title} · {}s/column · gaps are unobserved", step / 1000);
        v.plot_max = max;
        let (second, title, step, max) = match app.current_section {
            Section::Memory => (&app.swap_history, "Swap %", 1000, Some(100.0)),
            Section::Network => (&app.net_up_history, "Upload bytes/s", 1000, None),
            Section::Disk => (
                &app.disk_write_history,
                "Physical disk write bytes/s",
                1000,
                None,
            ),
            Section::Thermals => (&app.gpu_temp_history, "GPU temperature °C", 5000, None),
            Section::Cpu => (
                app.per_core_history
                    .get(v.selected)
                    .unwrap_or(&app.cpu_history),
                "Selected logical processor %",
                1000,
                Some(100.0),
            ),
            _ => (&app.mem_history, "Memory %", 1000, Some(100.0)),
        };
        v.secondary_plot = second.timeline(app.presentation_time_ms, step, 120);
        v.secondary_title = format!("{title} · {}s/column", step / 1000);
        v.secondary_max = max;
        if !fresh(app, lane) && s.samples.contains_key(lane) {
            v.status = format!("STALE / INCOMPLETE · {}", v.status);
        }

        v
    }
}
fn flatten(lines: &mut Vec<String>, prefix: &str, value: &Value, depth: usize) {
    if lines.len() >= 256 || depth > 7 {
        return;
    }
    match value {
        Value::Object(map) => {
            for (k, v) in map {
                let key = if prefix.is_empty() {
                    k.replace('_', " ")
                } else {
                    format!("{prefix} / {}", k.replace('_', " "))
                };
                flatten(lines, &key, v, depth + 1);
            }
        }
        Value::Array(items) => {
            for (i, v) in items.iter().take(32).enumerate() {
                flatten(lines, &format!("{prefix} {}", i + 1), v, depth + 1);
            }
        }
        _ => lines.push(format!(
            "{prefix}: {}",
            match value {
                Value::Null => "Unavailable".into(),
                Value::String(s) => s.chars().take(512).collect(),
                _ => value.to_string(),
            }
        )),
    }
}
