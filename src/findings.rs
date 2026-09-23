//! One interpretation layer for both frontends and exported reports.
use crate::collectors::{disk_health::DiskHealthStatus, SystemSnapshot};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FindingKind {
    ResourcePressure,
    ConnectivityObservation,
    HardwareFault,
    IncompleteObservation,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Finding {
    pub id: String,
    pub kind: FindingKind,
    pub severity: String,
    pub title: String,
    pub evidence: String,
    pub next_step: String,
    pub source: String,
}

pub fn for_snapshot(snapshot: &SystemSnapshot) -> Vec<Finding> {
    let mut findings = Vec::new();
    let now = crate::collectors::sampling::unix_ms();
    if snapshot.processes.observation.source != "not_collected"
        && !snapshot.processes.observation.is_available()
    {
        findings.push(Finding {
            id: "process-inventory".into(), kind: FindingKind::IncompleteObservation,
            severity: "info".into(), title: "Process inventory is incomplete".into(),
            evidence: snapshot.processes.observation.detail.clone().unwrap_or_else(|| "The process provider returned no readable inventory".into()),
            next_step: "Inspect process provider availability and retry; aggregate monitoring remains available".into(),
            source: snapshot.processes.observation.source.clone(),
        });
    }
    let fast_valid = snapshot
        .samples
        .get("fast")
        .is_some_and(|s| s.observation.is_available() && !s.is_stale());
    if fast_valid && snapshot.cpu.total_usage >= 90.0 {
        findings.push(Finding { id: "cpu-pressure".into(), kind: FindingKind::ResourcePressure,
            severity: "warning".into(), title: "High processor demand".into(),
            evidence: format!("Aggregate CPU utilization is {:.1}%; utilization alone does not establish a hardware fault", snapshot.cpu.total_usage),
            next_step: "Inspect Processes and look for sustained demand before changing applications".into(), source: "fast".into() });
    }
    if fast_valid && snapshot.memory.total_bytes > 0 && snapshot.memory.usage_percent() >= 90.0 {
        findings.push(Finding {
            id: "memory-pressure".into(),
            kind: FindingKind::ResourcePressure,
            severity: "warning".into(),
            title: "Little memory headroom".into(),
            evidence: format!(
                "{:.1}% of physical memory is used",
                snapshot.memory.usage_percent()
            ),
            next_step: "Sort Processes by memory and inspect the largest consumers".into(),
            source: "fast".into(),
        });
    }
    for drive in &snapshot.disk_health.drives {
        if matches!(
            drive.health_status,
            DiskHealthStatus::Warning | DiskHealthStatus::Critical
        ) {
            findings.push(Finding { id: format!("disk-health:{}", drive.device_id), kind: FindingKind::HardwareFault,
                severity: if drive.health_status == DiskHealthStatus::Critical { "critical" } else { "warning" }.into(),
                title: format!("Storage provider reports {}", drive.health_status.user_label()),
                evidence: format!("{}: {:?} from {}; {}", drive.device_id, drive.health_status, drive.health_source,
                    capture_context(snapshot.samples.get("health"), now)),
                next_step: "Verify backups and inspect the drive's detailed health report; no repair has been attempted".into(), source: "health".into() });
        }
    }
    for (source, meta) in &snapshot.samples {
        if !meta.observation.is_available() || meta.is_stale() {
            findings.push(Finding {
                id: format!("observation:{source}"),
                kind: FindingKind::IncompleteObservation,
                severity: "info".into(),
                title: format!("{source} observation is incomplete"),
                evidence: format!(
                    "{}; {}",
                    meta.observation
                        .detail
                        .as_deref()
                        .unwrap_or("The latest check is incomplete"),
                    capture_context(Some(meta), now)
                ),
                next_step:
                    "Inspect the provider state and retry; other monitoring remains available"
                        .into(),
                source: source.clone(),
            });
        }
    }
    if snapshot.network.sample.observation.source != "not_collected"
        && !snapshot.network.sample.observation.is_available()
    {
        findings.push(Finding {
            id: "observation:network-rates".into(),
            kind: FindingKind::IncompleteObservation,
            severity: "info".into(),
            title: "Network rate observation is incomplete".into(),
            evidence: format!(
                "{}; {}",
                snapshot
                    .network
                    .sample
                    .observation
                    .detail
                    .as_deref()
                    .unwrap_or("No available interface aggregate"),
                capture_context(Some(&snapshot.network.sample), now)
            ),
            next_step:
                "Inspect Network for individual interfaces, aggregate scope and provider details"
                    .into(),
            source: "network".into(),
        });
    }
    let driver_limits: Vec<_> = snapshot
        .drivers
        .observations
        .iter()
        .chain(
            snapshot
                .drivers
                .services
                .iter()
                .map(|s| &s.observation)
                .filter(|o| {
                    matches!(
                        o.status,
                        crate::observation::ObservationStatus::PermissionDenied
                            | crate::observation::ObservationStatus::Error
                            | crate::observation::ObservationStatus::Contradictory
                    )
                }),
        )
        .filter(|o| !o.is_available())
        .take(16)
        .map(|o| {
            format!(
                "{}: {:?}; {}",
                o.source,
                o.status,
                o.detail.as_deref().unwrap_or("No provider detail")
            )
        })
        .collect();
    if !driver_limits.is_empty() {
        findings.push(Finding {
            id: "observation:device-inventory".into(), kind: FindingKind::IncompleteObservation,
            severity: "info".into(), title: "Device inventory has observation limits".into(),
            evidence: format!("{}; {}", driver_limits.join(" | "), capture_context(snapshot.samples.get("drivers"), now)),
            next_step: "Inspect Drivers for per-provider and service states; discovery alone does not establish hardware health".into(),
            source: "drivers".into(),
        });
    }
    if let Some(result) = &snapshot.storage_probe.result {
        let fault = matches!(
            result.drive.health_status,
            DiskHealthStatus::Warning | DiskHealthStatus::Critical
        );
        if fault || !result.observation.is_available() {
            findings.push(Finding {
                id: "explicit-storage-read".into(),
                kind: if fault { FindingKind::HardwareFault } else { FindingKind::IncompleteObservation },
                severity: if fault { "warning" } else { "info" }.into(),
                title: if fault { "Explicit storage read reported a hardware warning" } else { "Explicit storage read was incomplete" }.into(),
                evidence: format!("Captured {} ms UTC; health {:?}; observation {:?}", result.captured_unix_ms, result.drive.health_status, result.observation.status),
                next_step: if fault { "Verify backups and inspect the separately captured storage result before further testing" } else { "Inspect the storage result; ordinary monitoring remains available" }.into(),
                source: "explicit storage probe".into(),
            });
        }
    }
    if let Some(result) = &snapshot.companion.result {
        for check in &result.checks {
            use crate::companion::CheckState;
            if !matches!(
                check.state,
                CheckState::Warning | CheckState::Fault | CheckState::Incomplete
            ) {
                continue;
            }
            findings.push(Finding {
                id: format!("companion:{}", check.category),
                kind: if check.state == CheckState::Incomplete { FindingKind::IncompleteObservation } else { FindingKind::ConnectivityObservation },
                severity: if check.state == CheckState::Fault { "warning" } else { "info" }.into(),
                title: format!("ND-300 {}: {}", check.category, check.state.explanation()),
                evidence: format!("Explicit {:?} action captured at {} ms UTC; check state {:?}", result.action, result.captured_unix_ms, check.state),
                next_step: "Inspect Network companion for the measured evidence; no repair has been attempted".into(),
                source: "companion".into(),
            });
        }
    }
    findings.sort_by_key(|finding| match finding.kind {
        FindingKind::HardwareFault => 0,
        FindingKind::ResourcePressure | FindingKind::ConnectivityObservation => 1,
        FindingKind::IncompleteObservation => 2,
    });
    findings
}

fn capture_context(meta: Option<&crate::collectors::sampling::SampleMeta>, now: u64) -> String {
    let Some(meta) = meta.filter(|meta| meta.sequence > 0 && meta.captured_unix_ms > 0) else {
        return "No successful capture timestamp is available".into();
    };
    let age = now.checked_sub(meta.captured_unix_ms);
    let age_label = match age {
        Some(age) if age > meta.expected_interval_ms.saturating_mul(3).max(3_000) => {
            format!("{} ms old, stale", age)
        }
        Some(age) => format!("{} ms old", age),
        None => "age unavailable after a clock change".into(),
    };
    format!(
        "Last capture {} ms UTC ({age_label}); latest provider state {:?}",
        meta.captured_unix_ms, meta.observation.status
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn failed_health_refresh_keeps_fault_evidence_and_identifies_its_capture() {
        let mut snapshot = SystemSnapshot::default();
        snapshot
            .disk_health
            .drives
            .push(crate::collectors::disk_health::DriveHealth {
                device_id: "fixture-drive".into(),
                model: "fixture".into(),
                serial: None,
                firmware: None,
                media_type: Default::default(),
                health_status: DiskHealthStatus::Critical,
                temperature_celsius: None,
                power_on_hours: None,
                wear_percent: None,
                read_errors_total: None,
                write_errors_total: None,
                io_stats: None,
                health_source: "fixture SMART".into(),
            });
        let meta = crate::collectors::sampling::SampleMeta {
            sequence: 3,
            captured_unix_ms: 1000,
            interval_ms: 60_000,
            expected_interval_ms: 60_000,
            observation: crate::observation::Observation::error("health", "Read timed out"),
        };
        assert!(capture_context(Some(&meta), 182_000).contains("181000 ms old, stale"));
        assert!(capture_context(Some(&meta), 500).contains("age unavailable after a clock change"));
        assert!(capture_context(None, 1000).starts_with("No successful capture"));
        snapshot.samples.insert("health".into(), meta);
        let findings = for_snapshot(&snapshot);
        let fault = findings
            .iter()
            .find(|f| f.kind == FindingKind::HardwareFault)
            .unwrap();
        assert!(fault.evidence.contains("Last capture 1000 ms UTC"));
        assert!(fault.evidence.contains("latest provider state Error"));
        let incomplete = findings
            .iter()
            .find(|f| f.kind == FindingKind::IncompleteObservation)
            .unwrap();
        assert!(incomplete.evidence.contains("Read timed out"));
        assert!(incomplete.evidence.contains("Last capture 1000 ms UTC"));
    }
    #[test]
    fn high_cpu_is_resource_pressure_not_a_hardware_diagnosis() {
        let mut snapshot = SystemSnapshot::default();
        snapshot.cpu.total_usage = 100.0;
        let mut meta = crate::collectors::sampling::SampleMeta::default();
        meta.record(
            std::time::Duration::from_secs(1),
            std::time::Duration::from_secs(1),
            crate::observation::Observation::available("fixture"),
        );
        snapshot.samples.insert("fast".into(), meta);
        assert!(for_snapshot(&snapshot)
            .iter()
            .any(|f| f.kind == FindingKind::ResourcePressure));
        assert!(!for_snapshot(&snapshot)
            .iter()
            .any(|f| f.kind == FindingKind::HardwareFault));
    }
}
