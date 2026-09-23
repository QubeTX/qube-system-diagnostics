//! Diagnostic stage timings, not a process-resource or end-to-end latency gate.
//! Run a release build without other builds or visual observers. Output contains
//! only durations/counts, never process names, host identities or device details.
use std::{
    collections::BTreeMap,
    time::{Duration, Instant},
};

use ratatui::{backend::TestBackend, Terminal};
use sd_300::{
    app::App,
    collectors,
    observation::Observation,
    types::{DiagnosticMode, ProcessSortKey, Section},
};

#[derive(Default)]
struct Timings(BTreeMap<&'static str, Vec<f64>>);
impl Timings {
    fn measure<T>(&mut self, label: &'static str, collect: impl FnOnce() -> T) -> T {
        let start = Instant::now();
        let value = collect();
        self.0
            .entry(label)
            .or_default()
            .push(start.elapsed().as_secs_f64() * 1000.0);
        value
    }
    fn report(self) -> serde_json::Value {
        let stages = self.0.into_iter().map(|(label, mut values)| {
            values.sort_by(f64::total_cmp);
            let count = values.len();
            (label, serde_json::json!({"count":count,"mean_ms":values.iter().sum::<f64>() / count as f64,
                "p95_ms":values[((count * 95).div_ceil(100)).saturating_sub(1)],"max_ms":values[count-1]}))
        }).collect::<BTreeMap<_,_>>();
        serde_json::json!({"schema":1,"os":std::env::consts::OS,"arch":std::env::consts::ARCH,
            "build":if cfg!(debug_assertions) {"debug"}else{"release"},"stages":stages,
            "limitations":"Stage wall times, sequential diagnostic workload; TestBackend excludes terminal transport. Not process CPU or real input latency."})
    }
}

fn main() {
    let count = std::env::args()
        .nth(1)
        .map(|s| s.parse::<usize>().expect("integer samples"))
        .unwrap_or(30)
        .clamp(5, 120);
    let mut timings = Timings::default();
    let mut sys = sysinfo::System::new();
    let mut networks = sysinfo::Networks::new();
    let mut network = collectors::network::NetworkSampler::default();
    #[cfg(windows)]
    let mut processes = collectors::processes::GuiProcessSampler::default();
    let mut app = App::new(Some(DiagnosticMode::User));
    let mut terminal = Terminal::new(TestBackend::new(80, 24)).unwrap();
    for index in 0..count + 2 {
        let start = Instant::now();
        timings.measure("cpu.refresh_all", || sys.refresh_cpu_all());
        timings.measure("memory.refresh", || sys.refresh_memory());
        app.snapshot.cpu = timings.measure("cpu.project_and_topology", || {
            collectors::cpu::collect(&sys)
        });
        app.snapshot.memory =
            timings.measure("memory.project", || collectors::memory::collect(&sys));
        app.snapshot.network = timings.measure("network.refresh_and_project", || {
            network.collect(&mut networks)
        });
        #[cfg(windows)]
        {
            app.snapshot.processes = timings.measure("processes.refresh_and_project", || {
                processes.collect(sys.total_memory(), usize::MAX, ProcessSortKey::Cpu)
            });
        }
        #[cfg(not(windows))]
        {
            timings.measure("processes.refresh", || {
                sys.refresh_processes_specifics(
                    sysinfo::ProcessesToUpdate::All,
                    true,
                    sysinfo::ProcessRefreshKind::nothing()
                        .with_cpu()
                        .with_memory(),
                );
            });
            app.snapshot.processes = timings.measure("processes.project", || {
                collectors::processes::collect_limited(&sys, usize::MAX, ProcessSortKey::Cpu)
            });
        }
        app.presentation_time_ms = collectors::sampling::unix_ms();
        app.snapshot
            .samples
            .entry("fast".into())
            .or_default()
            .record(
                Duration::from_secs(1),
                Duration::from_secs(1),
                Observation::available("qualification"),
            );
        app.cpu_history.push_at(
            app.presentation_time_ms,
            Some(app.snapshot.cpu.total_usage as f64),
        );
        if index == 1 {
            // Exercise a fully populated bounded history, including steady-state
            // chart preparation rather than an empty startup graph.
            for i in 0..60 {
                app.cpu_history.push_at(
                    app.presentation_time_ms.saturating_sub((59 - i) * 1000),
                    Some(i as f64),
                );
            }
        }
        app.findings =
            timings.measure("findings", || sd_300::findings::for_snapshot(&app.snapshot));
        for (section, prepare, draw) in [
            (
                Section::Overview,
                "overview.prepare",
                "overview.test_backend_draw",
            ),
            (
                Section::Processes,
                "processes.prepare",
                "processes.test_backend_draw",
            ),
        ] {
            app.current_section = section;
            timings.measure(prepare, || app.prepare_view());
            timings.measure(draw, || {
                terminal
                    .draw(|frame| sd_300::ui::render(frame, &app))
                    .unwrap()
            });
        }
        if index < 2 {
            timings.0.clear();
        }
        std::thread::sleep(Duration::from_secs(1).saturating_sub(start.elapsed()));
    }
    println!(
        "{}",
        serde_json::to_string_pretty(&timings.report()).unwrap()
    );
}
