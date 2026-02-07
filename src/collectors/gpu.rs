/// GPU data — gracefully degrades when no GPU info is available
#[derive(Debug, Clone, Default)]
pub struct GpuData {
    pub available: bool,
    pub name: String,
    pub utilization_percent: f32,
    pub memory_used_mb: u64,
    pub memory_total_mb: u64,
    pub temperature: Option<f64>,
    pub driver_version: String,
}

impl GpuData {
    pub fn memory_percent(&self) -> f64 {
        if self.memory_total_mb == 0 {
            return 0.0;
        }
        (self.memory_used_mb as f64 / self.memory_total_mb as f64) * 100.0
    }
}

pub fn collect() -> GpuData {
    // Try NVIDIA via command line as a basic approach
    // In a full implementation, would use nvml-wrapper
    collect_nvidia().unwrap_or_default()
}

fn collect_nvidia() -> Option<GpuData> {
    // Try to run nvidia-smi for basic GPU info
    let output = std::process::Command::new("nvidia-smi")
        .args([
            "--query-gpu=name,utilization.gpu,memory.used,memory.total,temperature.gpu,driver_version",
            "--format=csv,noheader,nounits",
        ])
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let line = stdout.lines().next()?;
    let parts: Vec<&str> = line.split(", ").collect();

    if parts.len() < 6 {
        return None;
    }

    Some(GpuData {
        available: true,
        name: parts[0].trim().to_string(),
        utilization_percent: parts[1].trim().parse().unwrap_or(0.0),
        memory_used_mb: parts[2].trim().parse().unwrap_or(0),
        memory_total_mb: parts[3].trim().parse().unwrap_or(0),
        temperature: parts[4].trim().parse().ok(),
        driver_version: parts[5].trim().to_string(),
    })
}
