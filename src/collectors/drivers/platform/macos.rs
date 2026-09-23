use crate::collectors::command::{run_output, run_stdout, CommandError, CommandTimeout};
use crate::collectors::drivers::{
    DeviceCategory, DeviceInfo, DeviceStatus, DriverData, ServiceInfo,
};
use crate::observation::Observation;
use serde_json::Value;

pub fn collect() -> DriverData {
    let mut data = DriverData::default();
    match run_stdout(
        "/usr/sbin/networksetup",
        ["-listallhardwareports"],
        CommandTimeout::Normal,
    ) {
        Ok(text) => {
            data.network = network_devices(&text);
            data.observations
                .push(Observation::available("networksetup hardware ports"));
        }
        Err(error) => data
            .observations
            .push(error.observation("networksetup hardware ports")),
    }
    for (kind, category) in [
        ("SPBluetoothDataType", DeviceCategory::Bluetooth),
        ("SPAudioDataType", DeviceCategory::Audio),
    ] {
        let result = run_stdout(
            "/usr/sbin/system_profiler",
            [kind, "-json"],
            CommandTimeout::Slow,
        )
        .and_then(|text| profiler_devices(text.as_bytes(), kind, category.clone()));
        match result {
            Ok(devices) => {
                if category == DeviceCategory::Bluetooth {
                    data.bluetooth = devices;
                } else {
                    data.audio = devices;
                }
                data.observations.push(Observation::available(kind));
            }
            Err(error) => data.observations.push(error.observation(kind)),
        }
    }
    let result = run_output(
        "/usr/sbin/ioreg",
        ["-a", "-r", "-c", "IOHIDDevice"],
        CommandTimeout::Normal,
    )
    .and_then(|output| {
        if !output.status.success() {
            return Err(CommandError::Exit(output.status));
        }
        hid_devices(&output.stdout)
    });
    match result {
        Ok(devices) => {
            data.input = devices;
            data.observations
                .push(Observation::available("IOHIDDevice registry inventory"));
        }
        Err(error) => data
            .observations
            .push(error.observation("IOHIDDevice registry inventory")),
    }
    let jobs = run_stdout("/bin/launchctl", ["list"], CommandTimeout::Normal);
    for (name, display) in [
        ("com.apple.bluetoothd", "Bluetooth Daemon"),
        ("com.apple.blued", "Legacy Bluetooth Daemon"),
        ("com.apple.audio.coreaudiod", "Core Audio"),
    ] {
        let (running, state, observation) = match &jobs {
            Ok(text) => launchd_state(text, name),
            Err(error) => (
                false,
                String::new(),
                error.observation("launchctl list (current bootstrap domain)"),
            ),
        };
        data.services.push(ServiceInfo {
            name: name.into(),
            display_name: display.into(),
            is_running: running,
            state,
            observation,
        });
    }
    data.finish_discovery();
    data
}

fn present(name: String, category: DeviceCategory, extra: String) -> DeviceInfo {
    DeviceInfo {
        name,
        category,
        extra,
        driver_version: String::new(),
        driver_date: String::new(),
        status: DeviceStatus::Unknown,
    }
}

fn network_devices(text: &str) -> Vec<DeviceInfo> {
    let mut name = None;
    let mut devices = Vec::new();
    for line in text.lines() {
        if let Some(port) = line.strip_prefix("Hardware Port: ") {
            name = Some(port.trim());
        }
        if let Some(interface) = line.strip_prefix("Device: ") {
            if let Some(port) = name.take().filter(|p| !p.is_empty()) {
                devices.push(present(
                    port.into(),
                    DeviceCategory::Network,
                    format!(
                        "Interface {}; hardware-port inventory only; health not reported",
                        interface.trim()
                    ),
                ));
            }
        }
    }
    devices
}

fn profiler_devices(
    bytes: &[u8],
    key: &str,
    category: DeviceCategory,
) -> Result<Vec<DeviceInfo>, CommandError> {
    let value: Value = serde_json::from_slice(bytes).map_err(|_| CommandError::Protocol)?;
    let items = value
        .get(key)
        .and_then(Value::as_array)
        .ok_or(CommandError::Protocol)?;
    let mut devices = Vec::new();
    fn visit(
        value: &Value,
        category: &DeviceCategory,
        devices: &mut Vec<DeviceInfo>,
        depth: usize,
    ) {
        if depth > 24 {
            return;
        }
        match value {
            Value::Array(items) => {
                for item in items {
                    visit(item, category, devices, depth + 1);
                }
            }
            Value::Object(map) => {
                // Only controller objects count as Bluetooth inventory. Paired
                // device names and audio grouping labels are not health evidence.
                if *category == DeviceCategory::Bluetooth {
                    if let Some(controller) =
                        map.get("controller_properties").and_then(Value::as_object)
                    {
                        let name = controller
                            .get("controller_name")
                            .and_then(Value::as_str)
                            .filter(|s| !s.is_empty())
                            .unwrap_or("Bluetooth Controller");
                        devices.push(present(
                            name.into(),
                            category.clone(),
                            "system_profiler controller inventory; health not reported".into(),
                        ));
                    }
                } else if let Some(name) = map
                    .get("_name")
                    .and_then(Value::as_str)
                    .filter(|s| !s.trim().is_empty())
                {
                    if map.keys().any(|k| k.starts_with("coreaudio_device_")) {
                        devices.push(present(
                            name.trim().into(),
                            category.clone(),
                            "system_profiler audio inventory; health not reported".into(),
                        ));
                    }
                }
                for item in map.values() {
                    visit(item, category, devices, depth + 1);
                }
            }
            _ => {}
        }
    }
    for item in items {
        visit(item, &category, &mut devices, 0);
    }
    Ok(devices)
}

fn hid_devices(bytes: &[u8]) -> Result<Vec<DeviceInfo>, CommandError> {
    let value = plist::Value::from_reader(std::io::Cursor::new(bytes))
        .map_err(|_| CommandError::Protocol)?;
    let roots = value.as_array().ok_or(CommandError::Protocol)?;
    let mut devices = Vec::new();
    // -r restricts roots to matching IOHIDDevice instances. Child registry
    // dictionaries describe subordinate objects and must not become extra devices.
    for root in roots {
        let dict = root.as_dictionary().ok_or(CommandError::Protocol)?;
        let name = dict
            .get("Product")
            .and_then(plist::Value::as_string)
            .or_else(|| {
                dict.get("IORegistryEntryName")
                    .and_then(plist::Value::as_string)
            })
            .filter(|s| !s.trim().is_empty())
            .unwrap_or("Unnamed HID device");
        let page = dict
            .get("PrimaryUsagePage")
            .and_then(plist::Value::as_unsigned_integer);
        let usage = dict
            .get("PrimaryUsage")
            .and_then(plist::Value::as_unsigned_integer);
        let pairs = dict
            .get("DeviceUsagePairs")
            .and_then(plist::Value::as_array)
            .map(Vec::len);
        devices.push(present(name.into(), DeviceCategory::Input,
            format!("IOHIDDevice inventory; primary usage page {page:?}, usage {usage:?}, usage pairs {pairs:?}; health not reported")));
    }
    Ok(devices)
}

fn launchd_state(text: &str, label: &str) -> (bool, String, Observation) {
    let source = "launchctl list (current bootstrap domain)";
    for line in text.lines() {
        let parts: Vec<_> = line.split_whitespace().collect();
        if parts.len() != 3 || parts[2] != label {
            continue;
        }
        let valid_pid = parts[0].parse::<u32>().ok().is_some_and(|pid| pid > 0);
        if (!valid_pid && parts[0] != "-") || parts[1].parse::<i32>().is_err() {
            return (
                false,
                String::new(),
                Observation::error(source, "Job entry has an invalid PID or exit status"),
            );
        }
        return (
            valid_pid,
            if valid_pid {
                "running"
            } else {
                "loaded; not running (may be on demand)"
            }
            .into(),
            Observation::available(source),
        );
    }
    (false, String::new(), Observation::unavailable(source, "This job was not listed in the current bootstrap domain; this does not establish whether its system-domain service is running"))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    #[cfg(target_os = "macos")]
    fn native_discovery_parses_the_host_registry_without_invented_input_devices() {
        let _guard = crate::collectors::command::TEST_PROCESS_GUARD
            .lock()
            .unwrap_or_else(|p| p.into_inner());
        let data = collect();
        let hid = data
            .observations
            .iter()
            .find(|o| o.source == "IOHIDDevice registry inventory")
            .unwrap();
        assert!(hid.is_available(), "Native HID query: {hid:?}");
        assert!(data
            .input
            .iter()
            .all(|d| d.extra.starts_with("IOHIDDevice inventory;")
                && d.status == DeviceStatus::Unknown));
        assert_eq!(data.attention_devices().count(), 0);
    }
    #[test]
    fn inventory_presence_never_invents_working_hardware() {
        let network = network_devices("Hardware Port: Wi-Fi\nDevice: en0\n");
        assert_eq!(network.len(), 1);
        assert_eq!(network[0].status, DeviceStatus::Unknown);
        assert!(profiler_devices(
            br#"{"SPBluetoothDataType":[]}"#,
            "SPBluetoothDataType",
            DeviceCategory::Bluetooth
        )
        .unwrap()
        .is_empty());
        assert!(profiler_devices(
            b"not json",
            "SPBluetoothDataType",
            DeviceCategory::Bluetooth
        )
        .is_err());
        let audio = profiler_devices(br#"{"SPAudioDataType":[{"_name":"Devices","_items":[{"_name":"USB Audio","coreaudio_device_transport":"USB"}]}]}"#, "SPAudioDataType", DeviceCategory::Audio).unwrap();
        assert_eq!(audio.len(), 1);
        assert_eq!(audio[0].name, "USB Audio");
        assert_eq!(audio[0].status, DeviceStatus::Unknown);
    }
    #[test]
    fn hid_inventory_is_real_structured_data_and_preserves_identical_devices() {
        assert!(hid_devices(b"<plist version=\"1.0\"><array/></plist>")
            .unwrap()
            .is_empty());
        let devices = hid_devices(b"<plist version=\"1.0\"><array><dict><key>Product</key><string>Keyboard</string><key>IORegistryEntryChildren</key><array><dict><key>Product</key><string>Child</string></dict></array></dict><dict><key>Product</key><string>Keyboard</string></dict></array></plist>").unwrap();
        assert_eq!(devices.len(), 2);
        assert!(devices
            .iter()
            .all(|d| d.name == "Keyboard" && d.status == DeviceStatus::Unknown));
        assert!(hid_devices(b"broken").is_err());
    }
    #[test]
    fn loaded_job_is_not_a_running_process_and_missing_domain_is_unknown() {
        let text = "PID Status Label\n- 0 on.demand\n35 -9 running\n";
        assert!(!launchd_state(text, "on.demand").0);
        assert!(launchd_state(text, "on.demand").2.is_available());
        assert!(launchd_state(text, "running").0);
        assert!(!launchd_state(text, "absent").2.is_available());
        assert!(!launchd_state("? 0 broken", "broken").2.is_available());
    }
}
