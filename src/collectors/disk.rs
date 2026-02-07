use sysinfo::{Disks, System};

#[derive(Debug, Clone, Default)]
pub struct DiskData {
    pub partitions: Vec<PartitionInfo>,
}

#[derive(Debug, Clone)]
pub struct PartitionInfo {
    pub name: String,
    pub mount_point: String,
    pub filesystem: String,
    pub total_bytes: u64,
    pub used_bytes: u64,
    pub available_bytes: u64,
    pub is_removable: bool,
    pub disk_type: DiskType,
}

#[derive(Debug, Clone, PartialEq)]
pub enum DiskType {
    Ssd,
    Hdd,
    Unknown,
}

impl std::fmt::Display for DiskType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DiskType::Ssd => write!(f, "SSD"),
            DiskType::Hdd => write!(f, "HDD"),
            DiskType::Unknown => write!(f, "Unknown"),
        }
    }
}

impl PartitionInfo {
    pub fn usage_percent(&self) -> f64 {
        if self.total_bytes == 0 {
            return 0.0;
        }
        (self.used_bytes as f64 / self.total_bytes as f64) * 100.0
    }
}

pub fn collect(_sys: &System) -> DiskData {
    let disks = Disks::new_with_refreshed_list();

    let partitions = disks
        .iter()
        .map(|d| {
            let total = d.total_space();
            let available = d.available_space();
            let used = total.saturating_sub(available);

            let disk_type = match d.kind() {
                sysinfo::DiskKind::SSD => DiskType::Ssd,
                sysinfo::DiskKind::HDD => DiskType::Hdd,
                _ => DiskType::Unknown,
            };

            PartitionInfo {
                name: d.name().to_string_lossy().to_string(),
                mount_point: d.mount_point().to_string_lossy().to_string(),
                filesystem: d.file_system().to_string_lossy().to_string(),
                total_bytes: total,
                used_bytes: used,
                available_bytes: available,
                is_removable: d.is_removable(),
                disk_type,
            }
        })
        .collect();

    DiskData { partitions }
}
