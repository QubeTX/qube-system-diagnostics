use serde::Serialize;

use crate::collectors::disk_health::DiskHealthData;
use crate::collectors::drivers::{DriverData, DriverScanStatus};
use crate::collectors::network_diag::NetworkDiagData;
use crate::collectors::{DiagnosticWarning, SystemSnapshot, WarningSeverity};
use crate::error::{AppError, Result};
use crate::observation::Observation;

#[derive(Debug, Clone, Serialize)]
pub struct DiagnosticReport {
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
        let mut snapshot = SystemSnapshot::default();
        snapshot.refresh_static();
        snapshot.refresh_fast();
        tokio::time::sleep(std::time::Duration::from_millis(250)).await;
        snapshot.refresh_fast();
        snapshot.refresh_slow();
        snapshot.refresh_connections();

        let (drivers, connectivity, disk_health) = tokio::join!(
            tokio::task::spawn_blocking(crate::collectors::drivers::collect),
            tokio::task::spawn_blocking(crate::collectors::network_diag::collect_connectivity),
            tokio::task::spawn_blocking(crate::collectors::disk_health::collect),
        );

        match drivers {
            Ok(data) => snapshot.drivers = data,
            Err(error) => snapshot.warnings.push(DiagnosticWarning {
                source: "Drivers".into(),
                message: format!("Driver collector task failed: {error}"),
                severity: WarningSeverity::Error,
            }),
        }
        match connectivity {
            Ok((data, warnings)) => {
                snapshot.network_diag.gateway = data.gateway;
                snapshot.network_diag.dns = data.dns;
                snapshot.network_diag.internet = data.internet;
                snapshot.warnings.extend(warnings);
            }
            Err(error) => snapshot.warnings.push(DiagnosticWarning {
                source: "Network".into(),
                message: format!("Connectivity collector task failed: {error}"),
                severity: WarningSeverity::Error,
            }),
        }
        match disk_health {
            Ok((data, warnings)) => {
                snapshot.disk_health = data;
                snapshot.warnings.extend(warnings);
            }
            Err(error) => snapshot.warnings.push(DiagnosticWarning {
                source: "Disk Health".into(),
                message: format!("Disk-health collector task failed: {error}"),
                severity: WarningSeverity::Error,
            }),
        }

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

        let capabilities = capabilities_for(&snapshot);
        let mut report = Self {
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
            system: snapshot.system,
            cpu: snapshot.cpu,
            memory: snapshot.memory,
            disk: snapshot.disk,
            disk_health: snapshot.disk_health,
            displays: snapshot.displays,
            gpu: snapshot.gpu,
            network: snapshot.network,
            network_diagnostics: snapshot.network_diag,
            processes: snapshot.processes,
            thermals: snapshot.thermals,
            drivers: snapshot.drivers,
            capabilities,
            warnings: snapshot.warnings,
        };
        if !include_sensitive {
            report.redact();
        }
        report
    }

    fn redact(&mut self) {
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

fn capabilities_for(snapshot: &SystemSnapshot) -> Vec<CapabilityRecord> {
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
                "ping and DNS",
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

pub fn print_snapshot(report: &DiagnosticReport, json: bool) -> Result<()> {
    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(report).map_err(|error| AppError::platform(format!(
                "JSON serialization failed: {error}"
            )))?
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

pub fn print_capabilities(report: &DiagnosticReport, json: bool) -> Result<()> {
    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&report.capabilities).map_err(|error| {
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

    #[tokio::test]
    async fn default_report_redacts_stable_identifiers() {
        let report = DiagnosticReport::collect(false).await;
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
