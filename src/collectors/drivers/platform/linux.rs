use crate::collectors::command::{run_stdout, CommandTimeout};
use crate::collectors::drivers::{
    DeviceCategory, DeviceInfo, DeviceStatus, DriverData, ServiceInfo,
};
use crate::observation::Observation;
use std::{fs, io::Read, path::Path};

pub fn collect() -> DriverData {
    let mut data = device_inventory(Path::new("/sys"), Path::new("/proc"));
    for (name, display, user) in [
        ("NetworkManager.service", "Network Manager", false),
        ("wpa_supplicant.service", "WPA Supplicant", false),
        ("bluetooth.service", "Bluetooth (BlueZ)", false),
        ("pipewire.service", "PipeWire", true),
        ("pulseaudio.service", "PulseAudio", true),
    ] {
        let scope = if user { "--user" } else { "--system" };
        let source = format!("systemctl show ({scope})");
        let result = run_stdout(
            "systemctl",
            [
                scope,
                "show",
                "--no-pager",
                "--property=LoadState,ActiveState,SubState,MainPID",
                name,
            ],
            CommandTimeout::Quick,
        );
        let (running, state, observation) = match result {
            Ok(text) => systemd_state(&text, &source),
            Err(error) => (false, String::new(), error.observation(&source)),
        };
        data.services.push(ServiceInfo {
            name: name.into(),
            display_name: display.into(),
            is_running: running,
            state,
            observation,
        });
    }
    data
}

fn io_observation(source: &str, error: std::io::Error) -> Observation {
    match error.kind() {
        std::io::ErrorKind::PermissionDenied => {
            Observation::permission_denied(source, "Inventory read was denied")
        }
        std::io::ErrorKind::NotFound => {
            Observation::unavailable(source, "This kernel inventory interface is not present")
        }
        _ => Observation::error(source, format!("Inventory read failed: {error}")),
    }
}

fn read_text(path: &Path) -> std::io::Result<String> {
    let mut text = String::new();
    fs::File::open(path)?
        .take(1_048_577)
        .read_to_string(&mut text)?;
    if text.len() > 1_048_576 {
        return Err(std::io::Error::other("Inventory exceeds its text limit"));
    }
    Ok(text)
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

fn device_inventory(sys: &Path, proc: &Path) -> DriverData {
    let mut data = DriverData::default();
    for (relative, category) in [
        ("class/net", DeviceCategory::Network),
        ("class/bluetooth", DeviceCategory::Bluetooth),
    ] {
        let source = format!("sysfs {relative}");
        let mut observation = Observation::available(&source);
        match fs::read_dir(sys.join(relative)) {
            Ok(entries) => {
                for entry in entries {
                    let entry = match entry {
                        Ok(entry) => entry,
                        Err(error) => {
                            observation = io_observation(&source, error);
                            continue;
                        }
                    };
                    let name = entry.file_name().to_string_lossy().into_owned();
                    if category == DeviceCategory::Network && name == "lo" {
                        continue;
                    }
                    let mut device = present(
                        name,
                        category.clone(),
                        "Kernel inventory; hardware health not reported".into(),
                    );
                    if category == DeviceCategory::Network {
                        // IF_OPER_DOWN can mean an unplugged cable. It does not
                        // establish administrative disablement or driver failure.
                        device.extra = match read_text(&entry.path().join("operstate")) {
                            Ok(state) => format!("Operational link state: {}; hardware health not reported", state.trim()),
                            Err(error) => format!("Operational link state unavailable: {error}; hardware health not reported"),
                        };
                        if let Ok(driver) = fs::read_link(entry.path().join("device/driver")) {
                            if let Some(driver) = driver.file_name() {
                                device.extra.push_str(&format!(
                                    "; driver module {} (version not reported)",
                                    driver.to_string_lossy()
                                ));
                            }
                        }
                        data.network.push(device);
                    } else {
                        data.bluetooth.push(device);
                    }
                }
            }
            Err(error) => observation = io_observation(&source, error),
        }
        data.observations.push(observation);
    }
    for (relative, category) in [
        ("asound/cards", DeviceCategory::Audio),
        ("bus/input/devices", DeviceCategory::Input),
    ] {
        let source = format!("procfs {relative}");
        match read_text(&proc.join(relative)) {
            Ok(text) => {
                if category == DeviceCategory::Audio {
                    data.audio = audio_devices(&text);
                } else {
                    data.input = input_devices(&text);
                }
                data.observations.push(Observation::available(source));
            }
            Err(error) => data.observations.push(io_observation(&source, error)),
        }
    }
    data.finish_discovery();
    data
}

fn audio_devices(text: &str) -> Vec<DeviceInfo> {
    text.lines()
        .filter_map(|line| {
            let line = line.trim();
            if !line.starts_with(|c: char| c.is_ascii_digit()) {
                return None;
            }
            let (_, description) = line.split_once(':')?;
            Some(present(
                description.trim().into(),
                DeviceCategory::Audio,
                "ALSA card inventory; hardware health not reported".into(),
            ))
        })
        .collect()
}

fn input_devices(text: &str) -> Vec<DeviceInfo> {
    text.split("\n\n")
        .filter_map(|block| {
            let name = block.lines().find_map(|line| {
                line.strip_prefix("N: Name=\"")
                    .and_then(|name| name.strip_suffix('"'))
            })?;
            let handlers = block
                .lines()
                .find_map(|line| line.strip_prefix("H: Handlers="))
                .unwrap_or("not reported");
            Some(present(
                name.into(),
                DeviceCategory::Input,
                format!(
                    "Kernel input inventory; handlers {handlers}; hardware health not reported"
                ),
            ))
        })
        .collect()
}

fn systemd_state(text: &str, source: &str) -> (bool, String, Observation) {
    let property = |key: &str| {
        text.lines().find_map(|line| {
            line.split_once('=')
                .filter(|(name, _)| *name == key)
                .map(|(_, value)| value)
        })
    };
    let Some(load) = property("LoadState") else {
        return (
            false,
            String::new(),
            Observation::error(source, "Service query omitted LoadState"),
        );
    };
    if load == "not-found" {
        return (
            false,
            String::new(),
            Observation::unavailable(
                source,
                "The unit is not installed in this service-manager scope",
            ),
        );
    }
    if load != "loaded" {
        return (
            false,
            String::new(),
            Observation::unavailable(source, format!("Service configuration state: {load}")),
        );
    }
    let Some((active, sub, pid)) = property("ActiveState")
        .zip(property("SubState"))
        .zip(property("MainPID").and_then(|p| p.parse::<u32>().ok()))
        .map(|((a, s), p)| (a, s, p))
    else {
        return (
            false,
            String::new(),
            Observation::error(
                source,
                "Service query omitted a valid ActiveState, SubState or MainPID",
            ),
        );
    };
    if active.is_empty() || sub.is_empty() {
        return (
            false,
            String::new(),
            Observation::error(source, "Service state was empty"),
        );
    }
    (
        pid > 0,
        format!("{active}/{sub}"),
        Observation::available(source),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    #[cfg(target_os = "linux")]
    fn native_kernel_inventory_reports_provider_availability() {
        let data = device_inventory(Path::new("/sys"), Path::new("/proc"));
        let net = data
            .observations
            .iter()
            .find(|o| o.source == "sysfs class/net")
            .unwrap();
        assert!(net.is_available(), "Native network inventory: {net:?}");
        assert_eq!(data.attention_devices().count(), 0);
        assert!(data.devices().all(|d| d.status == DeviceStatus::Unknown));
    }
    #[test]
    fn missing_inventory_and_link_down_never_invent_failed_devices() {
        let temp = tempfile::tempdir().unwrap();
        let mut data = device_inventory(temp.path(), temp.path());
        assert_eq!(data.devices().count(), 0);
        assert_eq!(data.attention_devices().count(), 0);
        assert!(data.observations.iter().all(|o| !o.is_available()));
        fs::create_dir_all(temp.path().join("class/net/eth0")).unwrap();
        fs::write(temp.path().join("class/net/eth0/operstate"), "down\n").unwrap();
        data = device_inventory(temp.path(), temp.path());
        assert_eq!(data.network[0].status, DeviceStatus::Unknown);
        assert!(data.network[0].extra.contains("down"));
        assert_eq!(data.attention_devices().count(), 0);
    }
    #[test]
    fn inventories_preserve_real_devices_without_name_guesses() {
        assert!(audio_devices("--- no soundcards ---").is_empty());
        assert_eq!(
            audio_devices(" 0 [USB]: USB-Audio - Interface\n   detail").len(),
            1
        );
        let input = input_devices("N: Name=\"Vendor 123\"\nH: Handlers=kbd event0\n\nN: Name=\"Vendor 123\"\nH: Handlers=mouse0 event1\n");
        assert_eq!(input.len(), 2);
        assert!(input.iter().all(|d| d.status == DeviceStatus::Unknown));
    }
    #[test]
    fn service_availability_is_distinct_from_inactive_failed_and_oneshot() {
        for (active, sub, pid) in [
            ("active", "running", 45),
            ("inactive", "dead", 0),
            ("active", "exited", 0),
            ("failed", "failed", 0),
        ] {
            let (running, state, observation) = systemd_state(
                &format!("LoadState=loaded\nActiveState={active}\nSubState={sub}\nMainPID={pid}\n"),
                "fixture",
            );
            assert_eq!(running, pid > 0);
            assert_eq!(state, format!("{active}/{sub}"));
            assert!(observation.is_available());
        }
        assert!(!systemd_state("LoadState=not-found\n", "fixture")
            .2
            .is_available());
        assert!(!systemd_state("not properties", "fixture").2.is_available());
        assert!(!systemd_state(
            "LoadState=loaded\nActiveState=active\nSubState=running\nMainPID=no\n",
            "fixture"
        )
        .2
        .is_available());
        assert_eq!(
            io_observation(
                "fixture",
                std::io::Error::from(std::io::ErrorKind::PermissionDenied)
            )
            .status,
            crate::observation::ObservationStatus::PermissionDenied
        );
    }
}
