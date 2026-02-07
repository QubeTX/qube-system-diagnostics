use sysinfo::System;

#[derive(Debug, Clone, Default)]
pub struct MemoryData {
    pub used_bytes: u64,
    pub total_bytes: u64,
    pub available_bytes: u64,
    pub swap_used_bytes: u64,
    pub swap_total_bytes: u64,
}

impl MemoryData {
    pub fn usage_percent(&self) -> f64 {
        if self.total_bytes == 0 {
            return 0.0;
        }
        (self.used_bytes as f64 / self.total_bytes as f64) * 100.0
    }

    pub fn swap_percent(&self) -> f64 {
        if self.swap_total_bytes == 0 {
            return 0.0;
        }
        (self.swap_used_bytes as f64 / self.swap_total_bytes as f64) * 100.0
    }
}

pub fn collect(sys: &System) -> MemoryData {
    MemoryData {
        used_bytes: sys.used_memory(),
        total_bytes: sys.total_memory(),
        available_bytes: sys.available_memory(),
        swap_used_bytes: sys.used_swap(),
        swap_total_bytes: sys.total_swap(),
    }
}
