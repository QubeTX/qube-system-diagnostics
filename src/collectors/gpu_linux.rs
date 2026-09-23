use super::{
    gpu::{normalize_pci, GpuAdapter},
    linux_inventory::text,
};
use crate::observation::Observation;
use std::{collections::HashSet, fs, path::Path};

pub fn collect(root: &Path) -> (Vec<GpuAdapter>, Observation) {
    let Ok(entries) = fs::read_dir(root) else {
        return (
            vec![],
            Observation::unavailable(
                "DRM sysfs",
                "Graphics devices are not exposed by this kernel/session",
            ),
        );
    };
    let mut paths = entries
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| {
            p.file_name().and_then(|v| v.to_str()).is_some_and(|name| {
                name.strip_prefix("card").is_some_and(|tail| {
                    !tail.is_empty() && tail.bytes().all(|b| b.is_ascii_digit())
                })
            })
        })
        .collect::<Vec<_>>();
    paths.sort();
    let mut seen = HashSet::new();
    let mut adapters = Vec::new();
    for path in paths {
        let Ok(device) = fs::canonicalize(path.join("device")) else {
            continue;
        };
        if !seen.insert(device.clone()) {
            continue;
        }
        let pci = device
            .file_name()
            .and_then(|n| n.to_str())
            .and_then(normalize_pci);
        let vendor = text(device.join("vendor"));
        let vendor_name = match vendor.as_deref() {
            Some("0x1002") => "AMD",
            Some("0x8086") => "Intel",
            Some("0x10de") => "NVIDIA",
            Some("0x1af4") => "Virtio",
            _ => "Graphics adapter",
        };
        let name = text(device.join("product_name")).unwrap_or_else(|| {
            format!(
                "{vendor_name} {}",
                text(device.join("device")).unwrap_or_default()
            )
        });
        let driver = fs::canonicalize(device.join("driver"))
            .ok()
            .and_then(|p| p.file_name().map(|s| s.to_string_lossy().into_owned()));
        let number = |key: &str| text(device.join(key)).and_then(|s| s.parse::<u64>().ok());
        let temp = fs::read_dir(device.join("hwmon"))
            .ok()
            .into_iter()
            .flatten()
            .filter_map(Result::ok)
            .find_map(|e| {
                text(e.path().join("temp1_input"))
                    .and_then(|t| t.parse::<f64>().ok())
                    .map(|t| t / 1000.0)
                    .filter(|v| v.is_finite() && (-50.0..=200.0).contains(v))
            });
        adapters.push(GpuAdapter {
            device_id: format!("drm:{}", device.display()),
            pci_address: pci,
            name,
            status: driver.map(|driver| format!("Kernel driver: {driver}")),
            utilization_percent: number("gpu_busy_percent")
                .filter(|v| *v <= 100)
                .map(|v| v as f32),
            dedicated_memory_mb: number("mem_info_vram_total").map(|v| v / 1_048_576),
            memory_used_mb: number("mem_info_vram_used").map(|v| v / 1_048_576),
            temperature_celsius: temp,
            source: "DRM/PCI sysfs; driver VRAM counters and hwmon millidegrees".into(),
            ..Default::default()
        });
    }
    let status = if adapters.is_empty() {
        Observation::unavailable(
            "DRM/PCI sysfs",
            "No readable DRM card devices; containers may hide host GPUs",
        )
    } else {
        Observation::available("DRM/PCI sysfs")
    };
    (adapters, status)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn hwmon_and_vram_units_remain_independent_of_missing_utilization() {
        let root = std::env::temp_dir().join(format!("sd300-gpu-sysfs-{}", std::process::id()));
        let device = root.join("card0/device");
        fs::create_dir_all(device.join("hwmon/hwmon0")).unwrap();
        fs::write(device.join("vendor"), "0x1002").unwrap();
        fs::write(device.join("mem_info_vram_total"), "8589934592").unwrap();
        fs::write(device.join("hwmon/hwmon0/temp1_input"), "53500").unwrap();
        let (rows, status) = collect(&root);
        assert!(status.is_available());
        assert_eq!(rows[0].dedicated_memory_mb, Some(8192));
        assert_eq!(rows[0].temperature_celsius, Some(53.5));
        assert_eq!(rows[0].utilization_percent, None);
        fs::remove_dir_all(root).unwrap();
    }
}
