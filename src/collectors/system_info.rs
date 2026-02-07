use sysinfo::System;

#[derive(Debug, Clone, Default)]
pub struct SystemInfoData {
    pub os_name: String,
    pub os_version: String,
    pub hostname: String,
    pub cpu_model: String,
    pub cpu_cores: usize,
    pub cpu_threads: usize,
    pub total_memory_bytes: u64,
    pub architecture: String,
    pub uptime_seconds: u64,
    pub kernel_version: String,
}

pub fn collect(sys: &System) -> SystemInfoData {
    let cpu_model = sys
        .cpus()
        .first()
        .map(|c| c.brand().to_string())
        .unwrap_or_else(|| "Unknown".into());

    let cpu_cores = sys.physical_core_count().unwrap_or(0);
    let cpu_threads = sys.cpus().len();

    SystemInfoData {
        os_name: System::name().unwrap_or_else(|| "Unknown".into()),
        os_version: System::os_version().unwrap_or_else(|| "Unknown".into()),
        hostname: System::host_name().unwrap_or_else(|| "Unknown".into()),
        cpu_model,
        cpu_cores,
        cpu_threads,
        total_memory_bytes: sys.total_memory(),
        architecture: std::env::consts::ARCH.to_string(),
        uptime_seconds: System::uptime(),
        kernel_version: System::kernel_version().unwrap_or_else(|| "Unknown".into()),
    }
}
