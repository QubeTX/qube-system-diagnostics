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

use sysinfo::System;

#[derive(Debug, Clone)]
pub struct DiagnosticWarning {
    pub source: String,
    pub message: String,
    pub severity: WarningSeverity,
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
        self.sys.refresh_all();

        self.cpu = cpu::collect(&self.sys);
        self.memory = memory::collect(&self.sys);
        let prev_network = std::mem::take(&mut self.network);
        self.network = network::collect(&self.sys, &prev_network);
        self.processes = processes::collect(&self.sys);
    }

    /// Refresh slow metrics (every 5s): disk, GPU, thermals
    pub fn refresh_slow(&mut self) {
        self.disk = disk::collect(&self.sys);
        self.gpu = gpu::collect();

        let (thermal_data, thermal_warnings) = thermals::collect(&self.sys);
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
