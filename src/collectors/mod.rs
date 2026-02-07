pub mod cpu;
pub mod disk;
pub mod drivers;
pub mod gpu;
pub mod memory;
pub mod network;
pub mod platform;
pub mod processes;
pub mod system_info;
pub mod thermals;

use sysinfo::System;

/// Aggregate of all system data, refreshed on tick intervals
pub struct SystemSnapshot {
    pub system: system_info::SystemInfoData,
    pub cpu: cpu::CpuData,
    pub memory: memory::MemoryData,
    pub disk: disk::DiskData,
    pub gpu: gpu::GpuData,
    pub network: network::NetworkData,
    pub processes: processes::ProcessData,
    pub thermals: thermals::ThermalData,
    pub drivers: drivers::DriverData,
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
            gpu: gpu::GpuData::default(),
            network: network::NetworkData::default(),
            processes: processes::ProcessData::default(),
            thermals: thermals::ThermalData::default(),
            drivers: drivers::DriverData::default(),
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
        self.thermals = thermals::collect(&self.sys);
    }

    /// Refresh drivers (manual or every 30s)
    pub fn refresh_drivers(&mut self) {
        self.drivers = drivers::collect();
    }
}
