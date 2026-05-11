pub mod command;
pub mod cpu;
pub mod disk;
pub mod disk_health;
pub mod drivers;
pub mod gpu;
pub mod memory;
pub mod network;
pub mod network_diag;
pub mod platform;
pub mod processes;
pub mod system_info;
pub mod thermals;

use sysinfo::{Components, Disks, Networks, ProcessesToUpdate, System};

#[derive(Debug, Clone)]
pub struct DiagnosticWarning {
    pub source: String,
    pub message: String,
    pub severity: WarningSeverity,
}

#[cfg(test)]
mod command_tests {
    use std::time::{Duration, Instant};

    use super::command::{run_output, CommandTimeout};

    #[test]
    fn command_helper_returns_successful_output() {
        #[cfg(unix)]
        let output = run_output("sh", ["-c", "printf ok"], CommandTimeout::Normal)
            .expect("command should produce output");

        #[cfg(windows)]
        let output = run_output("cmd", ["/C", "echo ok"], CommandTimeout::Normal)
            .expect("command should produce output");

        assert!(output.status.success());
        assert!(String::from_utf8_lossy(&output.stdout).contains("ok"));
    }

    #[test]
    fn command_helper_times_out_and_kills_child() {
        let started = Instant::now();

        #[cfg(unix)]
        let output = run_output(
            "sh",
            ["-c", "sleep 2; printf late"],
            CommandTimeout::Custom(Duration::from_millis(75)),
        );

        #[cfg(windows)]
        let output = run_output(
            "powershell",
            [
                "-NoProfile",
                "-Command",
                "Start-Sleep -Seconds 2; Write-Output late",
            ],
            CommandTimeout::Custom(Duration::from_millis(75)),
        );

        assert!(output.is_none());
        assert!(started.elapsed() < Duration::from_secs(1));
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum WarningSeverity {
    Info,
    Warning,
    Error,
}

/// Aggregate of all system data, refreshed on tick intervals
pub struct SystemSnapshot {
    pub system: system_info::SystemInfoData,
    pub cpu: cpu::CpuData,
    pub memory: memory::MemoryData,
    pub disk: disk::DiskData,
    pub disk_health: disk_health::DiskHealthData,
    pub gpu: gpu::GpuData,
    pub network: network::NetworkData,
    pub network_diag: network_diag::NetworkDiagData,
    pub processes: processes::ProcessData,
    pub thermals: thermals::ThermalData,
    pub drivers: drivers::DriverData,
    pub warnings: Vec<DiagnosticWarning>,
    /// Internal sysinfo handle
    sys: System,
    networks: Networks,
    disks: Disks,
    components: Components,
}

impl Default for SystemSnapshot {
    fn default() -> Self {
        Self {
            system: system_info::SystemInfoData::default(),
            cpu: cpu::CpuData::default(),
            memory: memory::MemoryData::default(),
            disk: disk::DiskData::default(),
            disk_health: disk_health::DiskHealthData::default(),
            gpu: gpu::GpuData::default(),
            network: network::NetworkData::default(),
            network_diag: network_diag::NetworkDiagData::default(),
            processes: processes::ProcessData::default(),
            thermals: thermals::ThermalData::default(),
            drivers: drivers::DriverData::default(),
            warnings: Vec::new(),
            sys: System::new_all(),
            networks: Networks::new_with_refreshed_list(),
            disks: Disks::new_with_refreshed_list(),
            components: Components::new_with_refreshed_list(),
        }
    }
}

impl SystemSnapshot {
    /// Refresh static info (once at startup)
    pub fn refresh_static(&mut self) {
        self.system = system_info::collect(&self.sys);
    }

    /// Refresh fast metrics (every 1s): CPU, memory, network, processes
    pub fn refresh_fast(&mut self) {
        self.sys.refresh_cpu_all();
        self.sys.refresh_memory();
        self.sys.refresh_processes(ProcessesToUpdate::All, true);

        self.cpu = cpu::collect(&self.sys);
        self.memory = memory::collect(&self.sys);
        self.network = network::collect(&mut self.networks);
        self.processes = processes::collect(&self.sys);
    }

    /// Refresh slow metrics (every 5s): disk, GPU, thermals
    pub fn refresh_slow(&mut self) {
        self.disk = disk::collect(&mut self.disks);
        self.gpu = gpu::collect();

        let (thermal_data, thermal_warnings) = thermals::collect(&mut self.components);
        self.thermals = thermal_data;
        self.warnings.retain(|w| w.source != "Thermals");
        self.warnings.extend(thermal_warnings);
    }

    /// Refresh drivers (manual or every 30s)
    pub fn refresh_drivers(&mut self) {
        self.drivers = drivers::collect();
        self.warnings.retain(|w| w.source != "Drivers");
        if let drivers::DriverScanStatus::ScanFailed(ref msg) = self.drivers.scan_status {
            self.warnings.push(DiagnosticWarning {
                source: "Drivers".into(),
                message: msg.clone(),
                severity: WarningSeverity::Warning,
            });
        }
    }

    /// Refresh active connections (every 3s)
    pub fn refresh_connections(&mut self) {
        network_diag::refresh_connections(&mut self.network_diag);
    }

    /// Refresh connectivity checks (every 15s) — call from spawn_blocking
    pub fn refresh_network_diag(&mut self) {
        let (diag_data, diag_warnings) = network_diag::collect_connectivity();
        self.network_diag.gateway = diag_data.gateway;
        self.network_diag.dns = diag_data.dns;
        self.network_diag.internet = diag_data.internet;
        self.warnings.retain(|w| w.source != "Network");
        self.warnings.extend(diag_warnings);
    }

    /// Refresh disk health (every 60s)
    pub fn refresh_disk_health(&mut self) {
        let (health_data, health_warnings) = disk_health::collect();
        self.disk_health = health_data;
        self.warnings.retain(|w| w.source != "Disk Health");
        self.warnings.extend(health_warnings);
    }
}
