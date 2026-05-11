use crate::collectors::command::{run_output, run_status, CommandTimeout};
use crate::collectors::drivers::{
    DeviceCategory, DeviceInfo, DeviceStatus, DriverData, ServiceInfo,
};
use serde_json::Value;

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
    if let Some(output) = run_output(
        "networksetup",
        ["-listallhardwareports"],
        CommandTimeout::Normal,
    ) {
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
    if let Some(output) = run_output(
        "system_profiler",
        ["SPBluetoothDataType", "-json"],
        CommandTimeout::Slow,
    ) {
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
    if let Some(output) = run_output(
        "system_profiler",
        ["SPAudioDataType", "-json"],
        CommandTimeout::Slow,
    ) {
        for name in system_profiler_names(&output.stdout, "SPAudioDataType") {
            if name != "Audio" && name != "Devices" && name != "Properties" {
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

fn system_profiler_names(stdout: &[u8], top_level_key: &str) -> Vec<String> {
    let Ok(value) = serde_json::from_slice::<Value>(stdout) else {
        return Vec::new();
    };

    let Some(items) = value.get(top_level_key).and_then(Value::as_array) else {
        return Vec::new();
    };

    let mut names = Vec::new();
    for item in items {
        collect_profiler_names(item, &mut names);
    }
    names.sort();
    names.dedup();
    names
}

fn collect_profiler_names(value: &Value, names: &mut Vec<String>) {
    match value {
        Value::Array(items) => {
            for item in items {
                collect_profiler_names(item, names);
            }
        }
        Value::Object(map) => {
            if let Some(name) = map.get("_name").and_then(Value::as_str) {
                let trimmed = name.trim();
                if !trimmed.is_empty() {
                    names.push(trimmed.to_string());
                }
            }
            for item in map.values() {
                collect_profiler_names(item, names);
            }
        }
        _ => {}
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
        let is_running =
            run_status("launchctl", ["list", name], CommandTimeout::Quick).unwrap_or(false);

        data.services.push(ServiceInfo {
            name: name.to_string(),
            display_name: display.to_string(),
            is_running,
        });
    }
}

#[cfg(test)]
mod tests {
    use super::system_profiler_names;

    #[test]
    fn parses_audio_system_profiler_json_fixture() {
        let fixture = br#"
        {
          "SPAudioDataType": [
            {
              "_name": "Apple Inc. Speakers",
              "coreaudio_device_output": [
                { "_name": "MacBook Pro Speakers" }
              ]
            },
            {
              "_name": "External USB Audio",
              "coreaudio_device_input": [
                { "_name": "USB Microphone" }
              ]
            }
          ]
        }
        "#;

        let names = system_profiler_names(fixture, "SPAudioDataType");

        assert!(names.contains(&"Apple Inc. Speakers".to_string()));
        assert!(names.contains(&"MacBook Pro Speakers".to_string()));
        assert!(names.contains(&"External USB Audio".to_string()));
        assert!(names.contains(&"USB Microphone".to_string()));
    }
}
