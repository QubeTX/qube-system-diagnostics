use serde::Serialize;

use crate::collectors::disk_health::DiskHealthData;
use crate::collectors::drivers::{DriverData, DriverScanStatus};
use crate::collectors::network_diag::NetworkDiagData;
use crate::collectors::{DiagnosticWarning, SystemSnapshot, WarningSeverity};
use crate::error::{AppError, Result};
use crate::observation::Observation;

/// One-shot exports share the cancellable provider boundary. The finite wait
/// gathers a second fast/activity baseline without waiting forever for a probe.
pub async fn collect_snapshot() -> SystemSnapshot {
    let monitor = crate::monitor::Monitor::start(crate::monitor::Profile::Full);
    let mut snapshot = SystemSnapshot::default();
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(27);
    loop {
        monitor.drain(&mut snapshot);
        let warm = snapshot
            .samples
            .get("fast")
            .is_some_and(|sample| sample.sequence >= 2)
            && snapshot
                .samples
                .get("activity")
                .is_some_and(|sample| sample.sequence >= 2 || !sample.observation.is_available());
        if (snapshot.samples.len() == crate::monitor::Lane::ALL.len() && warm)
            || std::time::Instant::now() >= deadline
        {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(25)).await;
    }
    snapshot
}
#[derive(Debug, Clone)]
pub struct DiagnosticReport {
    pub samples: std::collections::BTreeMap<String, crate::collectors::sampling::SampleMeta>,
    pub findings: Vec<crate::findings::Finding>,
    pub disk_activity: crate::collectors::disk_activity::DiskActivity,
    pub schema_version: u32,
    pub product: &'static str,
    pub product_version: &'static str,
    pub target_os: &'static str,
    pub target_arch: &'static str,
    pub privacy: PrivacyMetadata,
    pub system: crate::collectors::system_info::SystemInfoData,
    pub cpu: crate::collectors::cpu::CpuData,
    pub memory: crate::collectors::memory::MemoryData,
    pub disk: crate::collectors::disk::DiskData,
    pub disk_health: DiskHealthData,
    pub displays: crate::collectors::display::DisplayData,
    pub gpu: crate::collectors::gpu::GpuData,
    pub network: crate::collectors::network::NetworkData,
    pub network_diagnostics: NetworkDiagData,
    pub companion: crate::companion::State,
    pub storage_probe: crate::storage_probe::State,
    pub processes: crate::collectors::processes::ProcessData,
    pub thermals: crate::collectors::thermals::ThermalData,
    pub drivers: DriverData,
    pub capabilities: Vec<CapabilityRecord>,
    pub warnings: Vec<DiagnosticWarning>,
}

#[derive(Debug, Clone, Serialize)]
pub struct PrivacyMetadata {
    pub sensitive_values_included: bool,
    pub redacted_fields: Vec<&'static str>,
}

#[derive(Debug, Clone, Serialize)]
pub struct CapabilityRecord {
    pub id: &'static str,
    #[serde(flatten)]
    pub observation: Observation,
}

impl DiagnosticReport {
    pub async fn collect(include_sensitive: bool) -> Self {
        let mut snapshot = collect_snapshot().await;
        let attention = snapshot
            .drivers
            .attention_devices()
            .map(|device| device.name.as_str())
            .collect::<Vec<_>>();
        if !attention.is_empty() {
            snapshot.warnings.push(DiagnosticWarning {
                source: "Drivers".into(),
                message: format!(
                    "{} device(s) need attention: {}",
                    attention.len(),
                    attention.join(", ")
                ),
                severity: WarningSeverity::Warning,
            });
        }

        Self::from_snapshot(&snapshot, include_sensitive)
    }

    /// Build the same versioned, redaction-aware report from an already-live
    /// collector snapshot. The desktop engine uses this for an explicit export
    /// request so it does not start a competing collector or pause the GUI
    /// renderer while gathering duplicate data.
    pub fn from_snapshot(snapshot: &SystemSnapshot, include_sensitive: bool) -> Self {
        let capabilities = capabilities_for(snapshot);
        let mut report = Self {
            samples: snapshot.samples.clone(),
            findings: crate::findings::for_snapshot(snapshot),
            disk_activity: snapshot.disk_activity.clone(),
            schema_version: 1,
            product: "SD-300",
            product_version: env!("CARGO_PKG_VERSION"),
            target_os: std::env::consts::OS,
            target_arch: std::env::consts::ARCH,
            privacy: PrivacyMetadata {
                sensitive_values_included: include_sensitive,
                redacted_fields: if include_sensitive {
                    Vec::new()
                } else {
                    vec![
                        "system.hostname",
                        "disk_health.drives[].serial",
                        "network.interfaces[].mac_address",
                        "network.interfaces[].ip_addresses",
                        "network_diagnostics.*_addr",
                    ]
                },
            },
            system: snapshot.system.clone(),
            cpu: snapshot.cpu.clone(),
            memory: snapshot.memory.clone(),
            disk: snapshot.disk.clone(),
            disk_health: snapshot.disk_health.clone(),
            displays: snapshot.displays.clone(),
            gpu: snapshot.gpu.clone(),
            network: snapshot.network.clone(),
            network_diagnostics: snapshot.network_diag.clone(),
            companion: snapshot.companion.clone(),
            storage_probe: snapshot.storage_probe.clone(),
            processes: snapshot.processes.clone(),
            thermals: snapshot.thermals.clone(),
            drivers: snapshot.drivers.clone(),
            capabilities,
            warnings: snapshot.warnings.clone(),
        };
        if !include_sensitive {
            report.redact();
        }
        report
    }

    fn redact(&mut self) {
        self.storage_probe = self.storage_probe.redacted();
        for adapter in &mut self.gpu.adapters {
            adapter.device_id = "[redacted]".into();
        }
        for sensor in &mut self.thermals.sensors {
            sensor.device_id = sensor.device_id.as_ref().map(|_| "[redacted]".into());
        }
        for fan in &mut self.thermals.fans {
            fan.device_id = fan.device_id.as_ref().map(|_| "[redacted]".into());
        }
        for device in &mut self.disk_activity.devices {
            device.identity = "[redacted]".into();
        }
        self.system.hostname = "[redacted]".into();
        for drive in &mut self.disk_health.drives {
            drive.serial = drive.serial.as_ref().map(|_| "[redacted]".into());
        }
        for interface in &mut self.network.interfaces {
            interface.mac_address = "[redacted]".into();
            for address in &mut interface.ip_addresses {
                *address = "[redacted]".into();
            }
        }
        self.network_diagnostics.gateway.target = "[redacted]".into();
        self.network_diagnostics.dns.resolved_ip = self
            .network_diagnostics
            .dns
            .resolved_ip
            .as_ref()
            .map(|_| "[redacted]".into());
        for connection in self
            .network_diagnostics
            .active_connections
            .iter_mut()
            .chain(self.network_diagnostics.listening_ports.iter_mut())
        {
            connection.local_addr = "[redacted]".into();
            connection.remote_addr = "[redacted]".into();
        }
    }
}

impl Serialize for DiagnosticReport {
    fn serialize<S: serde::Serializer>(
        &self,
        serializer: S,
    ) -> std::result::Result<S::Ok, S::Error> {
        self.as_schema(1).serialize(serializer)
    }
}

impl DiagnosticReport {
    pub fn as_schema(&self, version: u8) -> serde_json::Value {
        use serde_json::json;
        let mut value = json!({
            "schema_version": version, "product": self.product, "product_version": self.product_version,
            "target_os": self.target_os, "target_arch": self.target_arch, "privacy": self.privacy,
            "system": self.system, "cpu": self.cpu, "memory": self.memory, "disk": self.disk,
            "disk_health": self.disk_health, "displays": self.displays, "gpu": self.gpu,
            "network": self.network, "network_diagnostics": self.network_diagnostics,
            "processes": self.processes, "thermals": self.thermals, "drivers": self.drivers,
            "capabilities": self.capabilities, "warnings": self.warnings,
        });
        if version == 1 {
            return schema_one_projection(value);
        }
        value["privileged_storage"] = json!(self.storage_probe);
        value["samples"] = json!(self.samples);
        value["findings"] = json!(self.findings);
        value["disk_activity"] = json!(self.disk_activity);
        value["companion_results"] = self
            .companion
            .export(self.privacy.sensitive_values_included);
        if !self.privacy.sensitive_values_included {
            value["privacy"]["redacted_fields"]
                .as_array_mut()
                .unwrap()
                .push(json!("disk_activity.devices[].identity"));
            value["privacy"]["redacted_fields"]
                .as_array_mut()
                .unwrap()
                .extend([
                    json!("privileged_storage.identifiers_and_provider_detail"),
                    json!("gpu.adapters[].device_id"),
                    json!("thermals.sensors[].device_id"),
                    json!("thermals.fans[].device_id"),
                ]);
        }
        value["processes"]["cpu_normalization"] =
            json!("percent of one logical processor; 100% equals one fully used logical processor");
        let valid = |topic: &str| {
            self.samples
                .get(topic)
                .is_some_and(|s| s.observation.is_available() && !s.is_stale())
        };
        if !valid("fast") {
            value["cpu"]["total_usage"] = serde_json::Value::Null;
            value["cpu"]["per_core_usage"] = serde_json::Value::Null;
        }
        if self.memory.total_bytes == 0 || !valid("fast") {
            for key in ["total_bytes", "used_bytes", "available_bytes"] {
                value["memory"][key] = serde_json::Value::Null;
            }
        }
        if !self.network.sample.observation.is_available() || !valid("fast") {
            value["network"]["total_download_rate"] = serde_json::Value::Null;
            value["network"]["total_upload_rate"] = serde_json::Value::Null;
        }
        if let Some(interfaces) = value["network"]["interfaces"].as_array_mut() {
            for (interface, original) in interfaces.iter_mut().zip(&self.network.interfaces) {
                if !original.rate_status.is_available() || !valid("fast") {
                    interface["download_rate"] = serde_json::Value::Null;
                    interface["upload_rate"] = serde_json::Value::Null;
                }
                if !original.address_status.is_available() {
                    interface["ip_addresses"] = serde_json::Value::Null;
                }
            }
        }
        let primary = self.gpu.primary();
        if let Some(services) = value["drivers"]["services"].as_array_mut() {
            for (service, original) in services.iter_mut().zip(&self.drivers.services) {
                if !original.observation.is_available() || !valid("drivers") {
                    service["is_running"] = serde_json::Value::Null;
                    service["state"] = serde_json::Value::Null;
                }
            }
        }
        value["gpu"]["utilization_percent"] = json!(primary.and_then(|a| a.utilization_percent));
        value["gpu"]["memory_used_mb"] = json!(primary.and_then(|a| a.memory_used_mb));
        value["gpu"]["memory_total_mb"] = json!(primary.and_then(|a| a.dedicated_memory_mb));
        if let Some(processes) = value["processes"]["list"].as_array_mut() {
            for (process, original) in processes.iter_mut().zip(&self.processes.list) {
                if !original.cpu_observation.is_available() || !valid("fast") {
                    process["cpu_percent"] = serde_json::Value::Null;
                }
                if !original.memory_observation.is_available() {
                    process["memory_bytes"] = serde_json::Value::Null;
                    process["memory_percent"] = serde_json::Value::Null;
                }
            }
        }
        value
    }
}

/// Frozen v2.0.6/schema-1 keys. Additive collector fields are deliberately
/// excluded here, so internal evolution cannot silently change old exports.
fn schema_one_projection(mut value: serde_json::Value) -> serde_json::Value {
    static CONTRACT: std::sync::LazyLock<serde_json::Value> = std::sync::LazyLock::new(|| {
        serde_json::from_str(include_str!("report_schema1.json"))
            .expect("checked-in schema-1 contract")
    });
    let retain = |value: &mut serde_json::Value, keys: &serde_json::Value| {
        if let Some(object) = value.as_object_mut() {
            object.retain(|key, _| {
                keys.as_array()
                    .is_some_and(|keys| keys.iter().any(|k| k.as_str() == Some(key)))
            });
        }
    };
    for category in ["object_keys", "optional_object_keys"] {
        for (pointer, keys) in CONTRACT[category].as_object().unwrap() {
            if let Some(value) = value.pointer_mut(pointer) {
                retain(value, keys);
            }
        }
    }
    for (pointer, keys) in CONTRACT["array_item_keys"].as_object().unwrap() {
        if let Some(items) = value.pointer_mut(pointer).and_then(|v| v.as_array_mut()) {
            for item in items {
                retain(item, keys);
            }
        }
    }
    value
}

pub fn capabilities_for(snapshot: &SystemSnapshot) -> Vec<CapabilityRecord> {
    let available_or = |condition: bool, source: &str, detail: &str| {
        if condition {
            Observation::available(source)
        } else {
            Observation::unavailable(source, detail)
        }
    };

    vec![
        capability(
            "system.identity",
            available_or(
                !snapshot.system.os_name.is_empty(),
                "sysinfo",
                "Operating-system identity was empty",
            ),
        ),
        capability("system.hardware", snapshot.system.hardware_status.clone()),
        capability(
            "cpu.usage",
            available_or(
                !snapshot.cpu.per_core_usage.is_empty(),
                "sysinfo",
                "No logical processors were returned",
            ),
        ),
        capability(
            "cpu.hybrid_topology",
            Observation::unsupported(
                "platform CPU topology",
                "Performance-core versus efficiency-core classification is not implemented; core and thread totals remain authoritative",
            ),
        ),
        capability(
            "memory.aggregate",
            available_or(
                snapshot.memory.total_bytes > 0,
                "sysinfo",
                "Total memory was zero",
            ),
        ),
        capability("memory.modules", snapshot.memory.module_status.clone()),
        capability("gpu.inventory", snapshot.gpu.inventory_status.clone()),
        capability("gpu.telemetry", snapshot.gpu.telemetry_status.clone()),
        capability(
            "display.inventory",
            snapshot.displays.inventory_status.clone(),
        ),
        capability(
            "display.brightness",
            snapshot.displays.brightness_status.clone(),
        ),
        capability(
            "disk.inventory",
            available_or(
                !snapshot.disk_health.drives.is_empty(),
                "platform disk providers",
                "No physical drives were returned",
            ),
        ),
        capability("disk.health", snapshot.disk_health.health_status.clone()),
        capability(
            "disk.reliability",
            snapshot.disk_health.reliability_status.clone(),
        ),
        capability(
            "thermal.temperature",
            snapshot.thermals.temperature_status.clone(),
        ),
        capability(
            "thermal.cpu_temperature",
            snapshot.thermals.cpu_temperature_status.clone(),
        ),
        capability(
            "thermal.gpu_temperature",
            snapshot.thermals.gpu_temperature_status.clone(),
        ),
        capability("thermal.fans", snapshot.thermals.fan_status.clone()),
        capability("battery", snapshot.thermals.battery_status.clone()),
        capability(
            "battery.full_charged_capacity",
            available_or(
                snapshot
                    .thermals
                    .battery
                    .as_ref()
                    .and_then(|battery| battery.full_charged_capacity_mwh)
                    .is_some(),
                "BatteryFullChargedCapacity",
                "The provider returned no full-charge capacity",
            ),
        ),
        capability(
            "battery.cycle_count",
            available_or(
                snapshot
                    .thermals
                    .battery
                    .as_ref()
                    .and_then(|battery| battery.cycle_count)
                    .is_some(),
                "BatteryCycleCount",
                "The provider returned no cycle count",
            ),
        ),
        capability(
            "network.interfaces",
            available_or(
                !snapshot.network.interfaces.is_empty(),
                "sysinfo",
                "No network interfaces were returned",
            ),
        ),
        capability(
            "network.adapters",
            snapshot.network.adapter_status.clone(),
        ),
        capability(
            "network.connectivity",
            available_or(
                !snapshot.network_diag.internet.target.is_empty(),
                "platform ICMP, TCP fallback and DNS",
                "Connectivity checks did not run",
            ),
        ),
        capability(
            "processes",
            available_or(
                snapshot.processes.total_count > 0,
                "sysinfo",
                "No processes were returned",
            ),
        ),
        capability(
            "drivers",
            match &snapshot.drivers.scan_status {
                DriverScanStatus::Success => Observation::available("platform device provider"),
                DriverScanStatus::ScanFailed(message) => {
                    Observation::error("platform device provider", message)
                }
                DriverScanStatus::NotScanned | DriverScanStatus::Scanning => {
                    Observation::unavailable(
                        "platform device provider",
                        "The driver scan did not complete",
                    )
                }
            },
        ),
    ]
}

fn capability(id: &'static str, observation: Observation) -> CapabilityRecord {
    CapabilityRecord { id, observation }
}

pub fn print_snapshot(report: &DiagnosticReport, json: bool, schema_version: u8) -> Result<()> {
    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&report.as_schema(schema_version)).map_err(|error| {
                AppError::platform(format!("JSON serialization failed: {error}"))
            })?
        );
    } else {
        println!("SD-300 {} diagnostic snapshot", report.product_version);
        println!(
            "{} {} on {} ({})",
            report.system.os_name,
            report.system.os_version,
            report.system.cpu_model,
            report.target_arch
        );
        println!(
            "CPU: {} cores / {} threads, {:.1}% | Memory: {:.1}%",
            report.cpu.core_count,
            report.cpu.thread_count,
            report.cpu.total_usage,
            report.memory.usage_percent()
        );
        println!(
            "GPU adapters: {} | Physical drives: {} | Driver scan: {:?}",
            report.gpu.adapters.len(),
            report.disk_health.drives.len(),
            report.drivers.scan_status
        );
    }
    Ok(())
}

pub fn print_capabilities(report: &DiagnosticReport, json: bool, schema_version: u8) -> Result<()> {
    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&if schema_version == 1 { serde_json::json!(report.capabilities) } else { serde_json::json!({"schema_version":2,"product":report.product,"product_version":report.product_version,"capabilities":report.capabilities,"samples":report.samples,"findings":report.findings}) }).map_err(|error| {
                AppError::platform(format!("JSON serialization failed: {error}"))
            })?
        );
    } else {
        for capability in &report.capabilities {
            println!(
                "{:<24} {:?} ({}){}",
                capability.id,
                capability.observation.status,
                capability.observation.source,
                capability
                    .observation
                    .detail
                    .as_deref()
                    .map(|detail| format!(": {detail}"))
                    .unwrap_or_default()
            );
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn service_query_failure_is_null_in_schema_two_and_legacy_keys_stay_frozen() {
        use crate::collectors::drivers::ServiceInfo;
        let mut snapshot = SystemSnapshot::default();
        let mut sample = crate::collectors::sampling::SampleMeta::default();
        sample.record(
            std::time::Duration::from_secs(300),
            std::time::Duration::from_secs(300),
            Observation::available("fixture"),
        );
        snapshot.samples.insert("drivers".into(), sample);
        snapshot.drivers.services = vec![
            ServiceInfo {
                name: "denied".into(),
                display_name: "Denied".into(),
                is_running: false,
                state: String::new(),
                observation: Observation::permission_denied("fixture", "Denied"),
            },
            ServiceInfo {
                name: "idle".into(),
                display_name: "Idle".into(),
                is_running: false,
                state: "inactive/dead".into(),
                observation: Observation::available("fixture"),
            },
        ];
        let report = DiagnosticReport::from_snapshot(&snapshot, false);
        let rich = report.as_schema(2);
        assert!(rich["drivers"]["services"][0]["is_running"].is_null());
        assert_eq!(
            rich["drivers"]["services"][0]["observation"]["status"],
            "permission_denied"
        );
        assert_eq!(rich["drivers"]["services"][1]["is_running"], false);
        assert_eq!(rich["drivers"]["services"][1]["state"], "inactive/dead");
        let legacy = report.as_schema(1);
        assert_eq!(legacy["drivers"]["services"][0]["is_running"], false);
        assert!(legacy["drivers"]["services"][0]
            .get("observation")
            .is_none());
        assert!(legacy["drivers"].get("observations").is_none());
        assert!(report
            .findings
            .iter()
            .any(|f| f.id == "observation:device-inventory"
                && f.kind == crate::findings::FindingKind::IncompleteObservation));
    }

    #[test]
    fn schema_two_preserves_missing_measurements_while_default_remains_schema_one() {
        let mut snapshot = SystemSnapshot::default();
        snapshot
            .processes
            .list
            .push(crate::collectors::processes::ProcessInfo {
                pid: 42,
                start_time_unix_ms: Some(123),
                ..Default::default()
            });
        let report = DiagnosticReport::from_snapshot(&snapshot, false);
        let old = serde_json::to_value(&report).unwrap();
        assert_eq!(old["schema_version"], 1);
        assert_eq!(old["cpu"]["total_usage"], 0.0);
        assert!(old.get("samples").is_none());
        assert!(old["processes"]["list"][0]
            .get("start_time_unix_ms")
            .is_none());
        assert!(old["network"].get("sample").is_none());
        let new = report.as_schema(2);
        assert_eq!(new["schema_version"], 2);
        assert!(new["cpu"]["total_usage"].is_null());
        assert!(new["network"]["total_download_rate"].is_null());
        assert!(new["gpu"]["utilization_percent"].is_null());
        assert!(new["processes"]["list"][0]["memory_bytes"].is_null());
        assert!(new["processes"]["list"][0]["cpu_percent"].is_null());
        assert_eq!(new["processes"]["list"][0]["start_time_unix_ms"], 123);
    }

    #[test]
    fn schema_two_keeps_measured_zero_and_does_not_fill_missing_gpu_fields() {
        let mut snapshot = SystemSnapshot::default();
        let mut sample = crate::collectors::sampling::SampleMeta::default();
        sample.record(
            std::time::Duration::from_secs(1),
            std::time::Duration::from_secs(1),
            Observation::available("fixture"),
        );
        snapshot.samples.insert("fast".into(), sample);
        snapshot
            .processes
            .list
            .push(crate::collectors::processes::ProcessInfo {
                cpu_observation: Observation::available("fixture"),
                memory_observation: Observation::permission_denied("fixture", "Access denied"),
                ..Default::default()
            });
        let value = DiagnosticReport::from_snapshot(&snapshot, false).as_schema(2);
        assert_eq!(value["cpu"]["total_usage"], 0.0);
        assert_eq!(value["processes"]["list"][0]["cpu_percent"], 0.0);
        assert!(value["processes"]["list"][0]["memory_bytes"].is_null());
        assert_eq!(
            value["processes"]["list"][0]["memory_observation"]["status"],
            "permission_denied"
        );
    }

    #[test]
    fn companion_privacy_projection_is_applied_by_both_export_versions() {
        let mut snapshot = SystemSnapshot::default();
        snapshot.companion.sequence = 1;
        snapshot.companion.result = Some(
            crate::companion::parse_results(
                crate::companion::Action::Deep,
                crate::companion::VERIFIED_VERSION,
                include_bytes!("companion-fixtures/nd300-4.0.1-partial.json"),
                Some(2),
                None,
            )
            .unwrap(),
        );
        let report = DiagnosticReport::from_snapshot(&snapshot, false);
        assert!(report.as_schema(1).get("companion_results").is_none());
        let v2 = report.as_schema(2);
        assert_eq!(
            v2["companion_results"]["result"]["checks"][1]["state"],
            "incomplete"
        );
        assert!(!v2.to_string().contains("private-host"));
        assert!(!v2.to_string().contains("192.0.2.1"));
        assert!(DiagnosticReport::from_snapshot(&snapshot, true)
            .as_schema(2)
            .to_string()
            .contains("private-host"));
    }

    #[test]
    fn explicit_storage_export_redacts_identity_and_keeps_schema_one_unchanged() {
        let mut snapshot = SystemSnapshot::default();
        let mut drive = crate::collectors::disk_health::empty_drive(
            "private-device".into(),
            "Fixture model".into(),
            crate::collectors::disk_health::MediaType::Unknown,
        );
        drive.serial = Some("private-serial".into());
        snapshot.storage_probe.notice = "private-helper-path".into();
        snapshot.storage_probe.result = Some(crate::storage_probe::ProbeResult {
            captured_unix_ms: 500,
            drive,
            observation: Observation::error("smartctl", "private-provider-detail"),
            elevated: true,
        });
        let report = DiagnosticReport::from_snapshot(&snapshot, false);
        assert!(report.as_schema(1).get("privileged_storage").is_none());
        let value = report.as_schema(2);
        assert!(!value.to_string().contains("private-"));
        assert_eq!(
            value["privileged_storage"]["result"]["captured_unix_ms"],
            500
        );
        assert_eq!(
            value["privileged_storage"]["result"]["drive"]["temperature_celsius"],
            serde_json::Value::Null
        );
        assert_eq!(value["findings"][0]["kind"], "incomplete_observation");
    }

    #[test]
    fn default_report_redacts_stable_identifiers() {
        let mut snapshot = SystemSnapshot::default();
        snapshot.system.hostname = "private-fixture-host".into();
        snapshot
            .network
            .interfaces
            .push(crate::collectors::network::InterfaceInfo {
                name: "fixture".into(),
                ip_addresses: vec!["192.0.2.1".into()],
                mac_address: "00:11:22:33:44:55".into(),
                received_bytes: 0,
                transmitted_bytes: 0,
                download_rate: 0,
                upload_rate: 0,
                is_up: true,
                operational_state: "up".into(),
                rate_status: Observation::available("fixture"),
                address_status: Observation::available("fixture"),
                included_in_total: true,
            });
        snapshot
            .disk_health
            .drives
            .push(crate::collectors::disk_health::DriveHealth {
                device_id: "fixture".into(),
                model: "fixture".into(),
                serial: Some("private-serial".into()),
                firmware: None,
                media_type: Default::default(),
                health_status: Default::default(),
                temperature_celsius: None,
                power_on_hours: None,
                wear_percent: None,
                read_errors_total: None,
                write_errors_total: None,
                io_stats: None,
                health_source: "fixture".into(),
            });
        let report = DiagnosticReport::from_snapshot(&snapshot, false);
        assert_eq!(report.network.interfaces.len(), 1);
        assert_eq!(report.disk_health.drives.len(), 1);
        assert_eq!(report.system.hostname, "[redacted]");
        assert!(report
            .network
            .interfaces
            .iter()
            .all(|interface| interface.mac_address == "[redacted]"));
        assert!(report
            .disk_health
            .drives
            .iter()
            .all(|drive| drive.serial.is_none() || drive.serial.as_deref() == Some("[redacted]")));
    }

    #[test]
    fn report_schema_serializes_observation_states() {
        let capability = capability(
            "fixture",
            Observation::unavailable("fixture provider", "no rows"),
        );
        let json = serde_json::to_value(capability).unwrap();
        assert_eq!(json["status"], "unavailable");
        assert_eq!(json["source"], "fixture provider");
    }
}
