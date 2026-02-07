use crate::collectors::drivers::{DeviceCategory, DeviceInfo, DeviceStatus, DriverData, ServiceInfo};
use std::process::Command;

pub fn collect() -> DriverData {
    let mut data = DriverData::default();

    collect_network_devices(&mut data);
    collect_bluetooth_devices(&mut data);
    collect_audio_devices(&mut data);
    collect_input_devices(&mut data);
    collect_services(&mut data);

    data
}

fn collect_network_devices(data: &mut DriverData) {
    // Use networksetup to list network services
    if let Ok(output) = Command::new("networksetup")
        .args(["-listallhardwareports"])
        .output()
    {
        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut current_name = String::new();

        for line in stdout.lines() {
            if let Some(rest) = line.strip_prefix("Hardware Port: ") {
                current_name = rest.trim().to_string();
            }
            if line.starts_with("Device: ") && !current_name.is_empty() {
                let status = if current_name.to_lowercase().contains("wi-fi")
                    || current_name.to_lowercase().contains("ethernet")
                {
                    DeviceStatus::Ok
                } else {
                    DeviceStatus::Unknown
                };

                data.network.push(DeviceInfo {
                    name: current_name.clone(),
                    driver_version: String::new(),
                    driver_date: String::new(),
                    status,
                    category: DeviceCategory::Network,
                    extra: String::new(),
                });
                current_name.clear();
            }
        }
    }

    if data.network.is_empty() {
        data.network.push(DeviceInfo {
            name: "Network".into(),
            driver_version: String::new(),
            driver_date: String::new(),
            status: DeviceStatus::Unknown,
            category: DeviceCategory::Network,
            extra: String::new(),
        });
    }
}

fn collect_bluetooth_devices(data: &mut DriverData) {
    // Check if Bluetooth is available via system_profiler
    if let Ok(output) = Command::new("system_profiler")
        .args(["SPBluetoothDataType", "-json"])
        .output()
    {
        let stdout = String::from_utf8_lossy(&output.stdout);
        if stdout.contains("controller_properties") {
            data.bluetooth.push(DeviceInfo {
                name: "Bluetooth Controller".into(),
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
    // Use system_profiler for audio
    if let Ok(output) = Command::new("system_profiler")
        .args(["SPAudioDataType"])
        .output()
    {
        let stdout = String::from_utf8_lossy(&output.stdout);
        for line in stdout.lines() {
            let trimmed = line.trim();
            // Look for device names (lines ending with ':' at a certain indent level)
            if trimmed.ends_with(':') && !trimmed.starts_with("Audio") && trimmed.len() > 1 {
                let name = trimmed.trim_end_matches(':').to_string();
                if !name.is_empty() && name != "Devices" && name != "Properties" {
                    data.audio.push(DeviceInfo {
                        name,
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
            status: DeviceStatus::Unknown,
            category: DeviceCategory::Audio,
            extra: String::new(),
        });
    }
}

fn collect_input_devices(data: &mut DriverData) {
    // Basic input device detection
    data.input.push(DeviceInfo {
        name: "Keyboard".into(),
        driver_version: String::new(),
        driver_date: String::new(),
        status: DeviceStatus::Ok,
        category: DeviceCategory::Input,
        extra: String::new(),
    });
    data.input.push(DeviceInfo {
        name: "Trackpad".into(),
        driver_version: String::new(),
        driver_date: String::new(),
        status: DeviceStatus::Ok,
        category: DeviceCategory::Input,
        extra: String::new(),
    });
}

fn collect_services(data: &mut DriverData) {
    // Check key macOS daemons
    let daemons = [
        ("com.apple.blued", "Bluetooth Daemon"),
        ("com.apple.audio.coreaudiod", "Core Audio"),
    ];

    for (name, display) in &daemons {
        let is_running = Command::new("launchctl")
            .args(["list", name])
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false);

        data.services.push(ServiceInfo {
            name: name.to_string(),
            display_name: display.to_string(),
            is_running,
        });
    }
}
