use crate::collectors::drivers::{DeviceCategory, DeviceInfo, DeviceStatus, DriverData, ServiceInfo};
use std::fs;
use std::process::Command;

pub fn collect() -> DriverData {
    let mut data = DriverData::default();

    // Network adapters
    collect_network_devices(&mut data);

    // Bluetooth
    collect_bluetooth_devices(&mut data);

    // Audio
    collect_audio_devices(&mut data);

    // Input devices
    collect_input_devices(&mut data);

    // Services
    collect_services(&mut data);

    data
}

fn collect_network_devices(data: &mut DriverData) {
    // Read network interfaces from /sys/class/net/
    if let Ok(entries) = fs::read_dir("/sys/class/net") {
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            if name == "lo" {
                continue; // Skip loopback
            }

            // Read driver info
            let driver_path = format!("/sys/class/net/{}/device/driver", name);
            let driver = fs::read_link(&driver_path)
                .ok()
                .and_then(|p| p.file_name().map(|f| f.to_string_lossy().to_string()))
                .unwrap_or_else(|| "unknown".into());

            // Read operstate
            let state_path = format!("/sys/class/net/{}/operstate", name);
            let operstate = fs::read_to_string(&state_path)
                .unwrap_or_default()
                .trim()
                .to_string();

            let status = match operstate.as_str() {
                "up" => DeviceStatus::Ok,
                "down" => DeviceStatus::Disabled,
                _ => DeviceStatus::Unknown,
            };

            data.network.push(DeviceInfo {
                name,
                driver_version: driver,
                driver_date: String::new(),
                status,
                category: DeviceCategory::Network,
                extra: operstate,
            });
        }
    }
}

fn collect_bluetooth_devices(data: &mut DriverData) {
    // Check /sys/class/bluetooth/
    if let Ok(entries) = fs::read_dir("/sys/class/bluetooth") {
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            data.bluetooth.push(DeviceInfo {
                name,
                driver_version: String::new(),
                driver_date: String::new(),
                status: DeviceStatus::Ok,
                category: DeviceCategory::Bluetooth,
                extra: String::new(),
            });
        }
    }

    if data.bluetooth.is_empty() {
        data.bluetooth.push(DeviceInfo {
            name: "Bluetooth".into(),
            driver_version: String::new(),
            driver_date: String::new(),
            status: DeviceStatus::NotFound,
            category: DeviceCategory::Bluetooth,
            extra: String::new(),
        });
    }
}

fn collect_audio_devices(data: &mut DriverData) {
    // Check /proc/asound/cards
    if let Ok(content) = fs::read_to_string("/proc/asound/cards") {
        for line in content.lines() {
            let line = line.trim();
            // Lines like " 0 [PCH            ]: HDA-Intel - HDA Intel PCH"
            if line.starts_with(|c: char| c.is_ascii_digit()) && line.contains(':') {
                if let Some(desc) = line.split(':').nth(1) {
                    data.audio.push(DeviceInfo {
                        name: desc.trim().to_string(),
                        driver_version: String::new(),
                        driver_date: String::new(),
                        status: DeviceStatus::Ok,
                        category: DeviceCategory::Audio,
                        extra: String::new(),
                    });
                }
            }
        }
    }

    if data.audio.is_empty() {
        data.audio.push(DeviceInfo {
            name: "Audio".into(),
            driver_version: String::new(),
            driver_date: String::new(),
            status: DeviceStatus::NotFound,
            category: DeviceCategory::Audio,
            extra: String::new(),
        });
    }
}

fn collect_input_devices(data: &mut DriverData) {
    // Check /proc/bus/input/devices
    if let Ok(content) = fs::read_to_string("/proc/bus/input/devices") {
        let mut current_name = String::new();
        for line in content.lines() {
            if let Some(rest) = line.strip_prefix("N: Name=\"") {
                current_name = rest.trim_end_matches('"').to_string();
            }
            if line.starts_with("H: Handlers=") && !current_name.is_empty() {
                let name_lower = current_name.to_lowercase();
                if name_lower.contains("keyboard") || name_lower.contains("mouse")
                    || name_lower.contains("touchpad") || name_lower.contains("trackpad")
                {
                    data.input.push(DeviceInfo {
                        name: current_name.clone(),
                        driver_version: String::new(),
                        driver_date: String::new(),
                        status: DeviceStatus::Ok,
                        category: DeviceCategory::Input,
                        extra: String::new(),
                    });
                }
                current_name.clear();
            }
        }
    }

    if data.input.is_empty() {
        data.input.push(DeviceInfo {
            name: "Input Devices".into(),
            driver_version: String::new(),
            driver_date: String::new(),
            status: DeviceStatus::Unknown,
            category: DeviceCategory::Input,
            extra: String::new(),
        });
    }
}

fn collect_services(data: &mut DriverData) {
    let services = [
        ("NetworkManager", "Network Manager"),
        ("wpa_supplicant", "WPA Supplicant"),
        ("bluetooth", "Bluetooth (BlueZ)"),
        ("pipewire", "PipeWire"),
        ("pulseaudio", "PulseAudio"),
    ];

    for (name, display) in &services {
        let is_running = Command::new("systemctl")
            .args(["is-active", "--quiet", name])
            .status()
            .map(|s| s.success())
            .unwrap_or(false);

        data.services.push(ServiceInfo {
            name: name.to_string(),
            display_name: display.to_string(),
            is_running,
        });
    }
}
