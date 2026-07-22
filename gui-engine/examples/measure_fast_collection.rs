use sd_300::collectors::SystemSnapshot;
use sd_300::types::ProcessSortKey;
use serde::Serialize;
use std::{
    thread,
    time::{Duration, Instant},
};
use sysinfo::{ProcessRefreshKind, ProcessesToUpdate, System};

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Report {
    schema_version: u32,
    profile: &'static str,
    samples: usize,
    interval_milliseconds: u64,
    average_milliseconds: f64,
    p95_milliseconds: f64,
    maximum_milliseconds: f64,
}

fn main() {
    let samples = std::env::args()
        .nth(1)
        .and_then(|value| value.parse::<usize>().ok())
        .unwrap_or(30)
        .max(1);
    let interval = Duration::from_secs(1);
    let requested_profile = std::env::args().nth(2).unwrap_or_else(|| "detailed".into());
    let profile = match requested_profile.as_str() {
        "overview" => "overview",
        "process-cpu" => "process-cpu",
        "process-memory" => "process-memory",
        "process-cpu-memory" => "process-cpu-memory",
        "process-page" => "process-page",
        _ => "detailed",
    };
    let mut snapshot = SystemSnapshot::default();
    let mut raw_system = System::new_all();

    refresh(profile, &mut snapshot, &mut raw_system);
    thread::sleep(Duration::from_millis(250));
    refresh(profile, &mut snapshot, &mut raw_system);

    let mut measurements = Vec::with_capacity(samples);
    for _ in 0..samples {
        let started = Instant::now();
        refresh(profile, &mut snapshot, &mut raw_system);
        let elapsed = started.elapsed();
        measurements.push(elapsed.as_secs_f64() * 1_000.0);
        thread::sleep(interval.saturating_sub(elapsed));
    }

    measurements.sort_by(f64::total_cmp);
    let sum: f64 = measurements.iter().sum();
    let p95_index = ((measurements.len() as f64 * 0.95).ceil() as usize)
        .saturating_sub(1)
        .min(measurements.len() - 1);
    let report = Report {
        schema_version: 1,
        profile,
        samples: measurements.len(),
        interval_milliseconds: interval.as_millis() as u64,
        average_milliseconds: sum / measurements.len() as f64,
        p95_milliseconds: measurements[p95_index],
        maximum_milliseconds: *measurements.last().expect("at least one sample"),
    };
    println!(
        "{}",
        serde_json::to_string_pretty(&report).expect("serialize report")
    );
}

fn refresh(profile: &str, snapshot: &mut SystemSnapshot, raw_system: &mut System) {
    match profile {
        "overview" => snapshot.refresh_overview(),
        "process-cpu" => {
            raw_system.refresh_processes_specifics(
                ProcessesToUpdate::All,
                true,
                ProcessRefreshKind::nothing().with_cpu(),
            );
        }
        "process-memory" => {
            raw_system.refresh_processes_specifics(
                ProcessesToUpdate::All,
                true,
                ProcessRefreshKind::nothing().with_memory(),
            );
        }
        "process-cpu-memory" => {
            raw_system.refresh_processes_specifics(
                ProcessesToUpdate::All,
                true,
                ProcessRefreshKind::nothing().with_cpu().with_memory(),
            );
        }
        "process-page" => snapshot.refresh_processes_gui(ProcessSortKey::Cpu),
        _ => snapshot.refresh_fast_gui(),
    }
}
