use sysinfo::System;

#[derive(Debug, Clone, Default)]
pub struct CpuData {
    pub total_usage: f32,
    pub per_core_usage: Vec<f32>,
    pub per_core_frequency: Vec<u64>,
    pub cpu_model: String,
    pub core_count: usize,
    pub thread_count: usize,
}

pub fn collect(sys: &System) -> CpuData {
    let cpus = sys.cpus();
    let total_usage = sys.global_cpu_usage();

    let per_core_usage: Vec<f32> = cpus.iter().map(|c| c.cpu_usage()).collect();
    let per_core_frequency: Vec<u64> = cpus.iter().map(|c| c.frequency()).collect();

    let cpu_model = cpus
        .first()
        .map(|c| c.brand().to_string())
        .unwrap_or_default();

    CpuData {
        total_usage,
        per_core_usage,
        per_core_frequency,
        cpu_model,
        core_count: sys.physical_core_count().unwrap_or(0),
        thread_count: cpus.len(),
    }
}
