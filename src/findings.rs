//! One interpretation layer for both frontends and exported reports.
use crate::collectors::{disk_health::DiskHealthStatus, SystemSnapshot};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FindingKind {
    ResourcePressure,
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
                evidence: format!("{}: {:?} from {}", drive.device_id, drive.health_status, drive.health_source),
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
                evidence: meta.observation.detail.clone().unwrap_or_else(|| {
                    format!("Last capture is {} ms old", meta.age_ms().unwrap_or(0))
                }),
                next_step:
                    "Inspect the provider state and retry; other monitoring remains available"
                        .into(),
                source: source.clone(),
            });
        }
    }
    findings.sort_by_key(|finding| match finding.kind {
        FindingKind::HardwareFault => 0,
        FindingKind::ResourcePressure => 1,
        FindingKind::IncompleteObservation => 2,
    });
    findings
}

#[cfg(test)]
mod tests {
    use super::*;
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
