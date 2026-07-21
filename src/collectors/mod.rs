pub mod command;
pub mod cpu;
pub mod disk;
pub mod disk_health;
pub mod display;
pub mod drivers;
pub mod gpu;
pub mod memory;
pub mod network;
pub mod network_diag;
pub mod platform;
pub mod processes;
pub mod system_info;
pub mod thermals;

use serde::Serialize;
#[cfg(not(target_os = "windows"))]
use sysinfo::ProcessRefreshKind;
use sysinfo::{Components, Disks, Networks, ProcessesToUpdate, System};

#[derive(Debug, Clone, Serialize)]
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

    #[test]
    fn command_helper_drains_output_larger_than_a_pipe_buffer() {
        const OUTPUT_SIZE: usize = 1024 * 1024;

        #[cfg(unix)]
        let output = run_output(
            "sh",
            ["-c", "head -c 1048576 /dev/zero"],
            CommandTimeout::Normal,
        )
        .expect("large-output command should complete");

        #[cfg(windows)]
        let output = run_output(
            "powershell",
            [
                "-NoProfile",
                "-Command",
                "[Console]::OpenStandardOutput().Write((New-Object byte[] 1048576), 0, 1048576)",
            ],
            CommandTimeout::Normal,
        )
        .expect("large-output command should complete");

        assert!(output.status.success());
        assert_eq!(output.stdout.len(), OUTPUT_SIZE);
    }

    #[cfg(windows)]
    #[test]
    fn windows_console_probe() {
        use winapi::um::wincon::GetConsoleWindow;

        println!("SD300_CONSOLE_HANDLE={}", unsafe {
            GetConsoleWindow() as usize
        });
    }

    #[cfg(windows)]
    #[test]
    fn command_helper_does_not_create_a_windows_console() {
        let executable = std::env::current_exe().expect("test executable path");
        let output = run_output(
            executable.as_os_str(),
            [
                "--exact",
                "collectors::command_tests::windows_console_probe",
                "--nocapture",
            ],
            CommandTimeout::Normal,
        )
        .expect("console probe should run");
        let stdout = String::from_utf8_lossy(&output.stdout);

        assert!(output.status.success(), "console probe failed: {stdout}");
        assert!(
            stdout.contains("SD300_CONSOLE_HANDLE=0"),
            "collector child unexpectedly owned a console: {stdout}"
        );
    }
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
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
    pub displays: display::DisplayData,
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
    #[cfg(target_os = "windows")]
    gui_process_sampler: processes::GuiProcessSampler,
}

impl Default for SystemSnapshot {
    fn default() -> Self {
        Self {
            system: system_info::SystemInfoData::default(),
            cpu: cpu::CpuData::default(),
            memory: memory::MemoryData::default(),
            disk: disk::DiskData::default(),
            disk_health: disk_health::DiskHealthData::default(),
            displays: display::DisplayData::default(),
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
            #[cfg(target_os = "windows")]
            gui_process_sampler: processes::GuiProcessSampler::default(),
        }
    }
}

impl SystemSnapshot {
    /// Refresh static info (once at startup)
    pub fn refresh_static(&mut self) {
        self.system = system_info::collect(&self.sys);
        memory::refresh_hardware(&mut self.memory);
        self.displays = display::collect();
        network::refresh_hardware(&mut self.network);
    }

    /// Refresh fast metrics (every 1s): CPU, memory, network, processes
    pub fn refresh_fast(&mut self) {
        self.sys.refresh_cpu_all();
        self.sys.refresh_memory();
        self.sys.refresh_processes(ProcessesToUpdate::All, true);

        self.cpu = cpu::collect(&self.sys);
        let modules = std::mem::take(&mut self.memory.modules);
        let module_status = self.memory.module_status.clone();
        self.memory = memory::collect(&self.sys);
        self.memory.modules = modules;
        self.memory.module_status = module_status;
        let adapters = std::mem::take(&mut self.network.adapters);
        let adapter_status = self.network.adapter_status.clone();
        self.network = network::collect(&mut self.networks);
        self.network.adapters = adapters;
        self.network.adapter_status = adapter_status;
        self.processes = processes::collect(&self.sys);
    }

    /// Refresh the same fast values consumed by the native GUI without asking
    /// sysinfo to poll process fields that are absent from the GUI projection.
    ///
    /// The established TUI keeps calling `refresh_fast` above. This additive
    /// path preserves its output and cadence while avoiding per-process disk
    /// and executable refresh work in the separate GUI engine process.
    pub fn refresh_fast_gui(&mut self) {
        self.refresh_fast_gui_summary();
        self.refresh_processes_gui(crate::types::ProcessSortKey::Cpu);
    }

    /// Refresh fast aggregate values used by detailed GUI pages that do not
    /// display a process table. Process enumeration is subscription-driven so
    /// CPU, disk, network, and thermal views do not pay for invisible rows.
    pub fn refresh_fast_gui_summary(&mut self) {
        self.sys.refresh_cpu_all();
        self.sys.refresh_memory();

        self.cpu = cpu::collect(&self.sys);
        let modules = std::mem::take(&mut self.memory.modules);
        let module_status = self.memory.module_status.clone();
        self.memory = memory::collect(&self.sys);
        self.memory.modules = modules;
        self.memory.module_status = module_status;
        let adapters = std::mem::take(&mut self.network.adapters);
        let adapter_status = self.network.adapter_status.clone();
        self.network = network::collect(&mut self.networks);
        self.network.adapters = adapters;
        self.network.adapter_status = adapter_status;
    }

    /// Refresh the one-second process projection only while its GUI page is
    /// subscribed. The platform sampler supplies both ranked process rows and
    /// total CPU load from the same system-time sample; memory is refreshed so
    /// the persistent header/tray never freezes while Processes is selected.
    /// Unrelated network and command-backed collectors stay dormant as before.
    pub fn refresh_processes_gui(&mut self, sort: crate::types::ProcessSortKey) {
        #[cfg(target_os = "windows")]
        {
            self.sys.refresh_memory();
            memory::refresh_usage(&mut self.memory, &self.sys);
            self.processes = self
                .gui_process_sampler
                .collect(self.memory.total_bytes, 16, sort);
            self.cpu.total_usage = self.gui_process_sampler.total_cpu_percent();
        }
        #[cfg(not(target_os = "windows"))]
        {
            self.sys.refresh_cpu_usage();
            self.sys.refresh_memory();
            self.cpu.total_usage = self.sys.global_cpu_usage();
            memory::refresh_usage(&mut self.memory, &self.sys);
            self.sys.refresh_processes_specifics(
                ProcessesToUpdate::All,
                true,
                ProcessRefreshKind::nothing().with_cpu().with_memory(),
            );
            self.processes = processes::collect_limited(&self.sys, 16, sort);
        }
    }

    /// Refresh only the CPU and memory values used by the native Overview.
    ///
    /// This is intentionally additive: the TUI continues to call `refresh_fast`
    /// with its existing CPU, memory, network, and process behavior. The GUI uses
    /// this narrower path until a visible page subscribes to the other collectors.
    pub fn refresh_overview(&mut self) {
        // The Overview consumes aggregate utilization, not per-core frequencies.
        // Avoid the broader refresh used by the TUI so the GUI's dedicated
        // collector thread does not perform work that its projection discards.
        self.sys.refresh_cpu_usage();
        self.sys.refresh_memory();

        self.cpu = cpu::collect(&self.sys);
        let modules = std::mem::take(&mut self.memory.modules);
        let module_status = self.memory.module_status.clone();
        self.memory = memory::collect(&self.sys);
        self.memory.modules = modules;
        self.memory.module_status = module_status;
    }

    /// Refresh slow metrics (every 5s): disk, GPU, thermals
    pub fn refresh_slow(&mut self) {
        self.disk = disk::collect(&mut self.disks);
        self.gpu = gpu::collect();

        let (thermal_data, thermal_warnings) = thermals::collect(&mut self.components, &self.gpu);
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
