//! Read-only Linux ABI inventory. File roots are injectable for deterministic
//! provider fixtures on every build host; no commands or privilege changes.
use super::{
    display::{DisplayData, DisplayInfo},
    network::NetworkAdapterInfo,
    system_info::SystemInfoData,
    thermals::BatteryInfo,
};
use crate::observation::Observation;
use std::{
    fs,
    io::Read,
    path::{Path, PathBuf},
};

pub fn text(path: impl AsRef<Path>) -> Option<String> {
    let mut bytes = Vec::new();
    fs::File::open(path)
        .ok()?
        .take(16_385)
        .read_to_end(&mut bytes)
        .ok()?;
    if bytes.len() > 16_384 {
        return None;
    }
    let value = String::from_utf8(bytes)
        .ok()?
        .trim_matches(['\0', '\n', '\r', ' ', '\t'])
        .to_string();
    (!value.is_empty()).then_some(value)
}
fn number(path: impl AsRef<Path>) -> Option<u64> {
    text(path)?.parse().ok()
}
fn entries(root: &Path) -> Result<Vec<PathBuf>, Observation> {
    let mut paths = fs::read_dir(root)
        .map_err(|error| match error.kind() {
            std::io::ErrorKind::PermissionDenied => {
                Observation::permission_denied("Linux sysfs", error.to_string())
            }
            std::io::ErrorKind::NotFound => {
                Observation::unavailable("Linux sysfs", "The kernel interface is absent")
            }
            _ => Observation::error("Linux sysfs", error.to_string()),
        })?
        .filter_map(Result::ok)
        .map(|e| e.path())
        .collect::<Vec<_>>();
    paths.sort();
    Ok(paths)
}

pub fn battery(root: &Path) -> (Option<BatteryInfo>, Observation) {
    let paths = match entries(root) {
        Ok(paths) => paths,
        Err(error) => return (None, error),
    };
    let external = paths
        .iter()
        .filter(|p| text(p.join("type")).is_some_and(|v| v != "Battery"))
        .filter_map(|p| number(p.join("online")))
        .collect::<Vec<_>>();
    for path in &paths {
        if text(path.join("type")).as_deref() != Some("Battery")
            || number(path.join("present")) == Some(0)
        {
            continue;
        }
        let Some(percent) = number(path.join("capacity")).filter(|n| *n <= 100) else {
            continue;
        };
        let status = text(path.join("status"));
        if !matches!(
            status.as_deref(),
            Some("Charging" | "Discharging" | "Full" | "Not charging")
        ) {
            continue;
        }
        let on_ac = if external.contains(&1) {
            Some(true)
        } else if !external.is_empty() {
            Some(false)
        } else {
            match status.as_deref() {
                Some("Charging") => Some(true),
                Some("Discharging") => Some(false),
                _ => None,
            }
        };
        // Avoid converting an unknown power state into a measured false. Other
        // devices remain eligible if this battery cannot provide a usable row.
        let Some(on_ac) = on_ac else {
            continue;
        };
        let name = path.file_name().unwrap_or_default().to_string_lossy();
        let time_remaining = if status.as_deref() == Some("Charging") {
            number(path.join("time_to_full_now"))
        } else if !on_ac {
            number(path.join("time_to_empty_now"))
        } else {
            None
        }
        .filter(|seconds| *seconds > 0)
        .map(|seconds| format!("{} min (provider estimate)", seconds / 60));
        return (
            Some(BatteryInfo {
                percent: percent as f64,
                is_charging: status.as_deref() == Some("Charging"),
                is_on_ac: on_ac,
                time_remaining,
                // ENERGY is micro-watt-hours; CHARGE is micro-amp-hours and must
                // never be relabelled as energy when only that interface exists.
                full_charged_capacity_mwh: number(path.join("energy_full")).map(|v| v / 1000),
                design_voltage_mv: number(path.join("voltage_min_design")).map(|v| v / 1000),
                cycle_count: number(path.join("cycle_count")).and_then(|v| v.try_into().ok()),
                provider_status: Some(format!(
                    "{name}: {}; selected battery, not a multi-battery aggregate",
                    status.as_deref().unwrap_or("Unknown")
                )),
            }),
            Observation::available(format!("power_supply/{name}")),
        );
    }
    (None, Observation::unavailable("power_supply", "No present battery with a charge percentage and known power state; desktops commonly have none"))
}

pub fn displays(root: &Path, backlight_root: &Path) -> DisplayData {
    let paths = match entries(root) {
        Ok(paths) => paths,
        Err(error) => {
            return DisplayData {
                inventory_status: error,
                ..Default::default()
            }
        }
    };
    let backlights = entries(backlight_root)
        .unwrap_or_default()
        .into_iter()
        .filter_map(|p| {
            let max = number(p.join("max_brightness"))?;
            let current =
                number(p.join("actual_brightness")).or_else(|| number(p.join("brightness")))?;
            (max > 0 && current <= max)
                .then(|| ((current as f64 * 100.0 / max as f64).round() as u8, p))
        })
        .collect::<Vec<_>>();
    let mut rows = Vec::new();
    for path in paths {
        if text(path.join("status")).as_deref() != Some("connected") {
            continue;
        }
        let name = path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();
        let connection = name
            .split_once('-')
            .map_or("Unknown", |(_, connector)| connector)
            .to_string();
        let edid = fs::read(path.join("edid")).ok();
        let dimensions = edid.as_deref().and_then(edid_dimensions);
        // Global ACPI backlight nodes are not reliable connector identities.
        // Match only a backlight physically below this connector's device.
        let canonical = fs::canonicalize(&path).ok();
        let brightness = backlights
            .iter()
            .find(|(_, p)| {
                canonical.as_ref().is_some_and(|connector| {
                    fs::canonicalize(p).is_ok_and(|b| b.starts_with(connector))
                })
            })
            .map(|(value, _)| *value);
        rows.push(DisplayInfo {
            label: name,
            active: text(path.join("enabled")).map(|v| v == "enabled"),
            connection,
            brightness_percent: brightness,
            physical_width_cm: dimensions.map(|v| v.0),
            physical_height_cm: dimensions.map(|v| v.1),
            source: "DRM connector sysfs / EDID".into(),
        });
    }
    let brightness_status = if rows.iter().any(|r| r.brightness_percent.is_some()) {
        Observation::available("connector-associated backlight sysfs")
    } else {
        Observation::unavailable("backlight sysfs", "No readable backlight could be matched to a connector; external monitor control is not attempted")
    };
    let inventory_status = if rows.is_empty() {
        Observation::unavailable(
            "DRM connector sysfs",
            "No connected displays; this can be a headless session",
        )
    } else {
        Observation::available("DRM connector sysfs")
    };
    DisplayData {
        displays: rows,
        inventory_status,
        brightness_status,
    }
}

fn edid_dimensions(bytes: &[u8]) -> Option<(u16, u16)> {
    if bytes.len() < 128
        || bytes[..8] != [0, 255, 255, 255, 255, 255, 255, 0]
        || bytes[..128].iter().fold(0u8, |sum, b| sum.wrapping_add(*b)) != 0
        || bytes[21] == 0
        || bytes[22] == 0
    {
        return None;
    }
    Some((bytes[21] as u16, bytes[22] as u16))
}

pub fn adapters(root: &Path) -> (Vec<NetworkAdapterInfo>, Observation) {
    let paths = match entries(root) {
        Ok(paths) => paths,
        Err(error) => return (vec![], error),
    };
    let rows = paths
        .into_iter()
        .filter(|p| p.join("device").exists())
        .map(|path| NetworkAdapterInfo {
            name: path
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .into_owned(),
            description: fs::canonicalize(path.join("device/driver"))
                .ok()
                .and_then(|p| p.file_name().map(|v| v.to_string_lossy().into_owned())),
            status: text(path.join("operstate")),
            media_connection_state: None,
            link_speed_bps: number(path.join("speed"))
                .filter(|v| *v > 0)
                .and_then(|v| v.checked_mul(1_000_000)),
            hardware_interface: Some(true),
        })
        .collect::<Vec<_>>();
    let status = if rows.is_empty() {
        Observation::unavailable("network sysfs", "No hardware-backed network adapters")
    } else {
        Observation::available("network sysfs; reported link speed in bits/s")
    };
    (rows, status)
}

pub fn hardware(data: &mut SystemInfoData, dmi: &Path, device_tree: &Path) {
    data.manufacturer = text(dmi.join("sys_vendor"));
    data.model = text(dmi.join("product_name")).or_else(|| text(device_tree.join("model")));
    data.bios_version = text(dmi.join("bios_version"));
    data.bios_release_date = text(dmi.join("bios_date"));
    data.hardware_status =
        if data.model.is_some() || data.manufacturer.is_some() || data.bios_version.is_some() {
            Observation::available("DMI sysfs / device tree")
        } else {
            Observation::unavailable(
                "DMI sysfs / device tree",
                "No readable hardware identity; containers may hide host firmware",
            )
        };
}

#[cfg(test)]
mod tests {
    use super::*;
    fn fixture(name: &str) -> PathBuf {
        let path = std::env::temp_dir().join(format!("sd300-linux-{name}-{}", std::process::id()));
        fs::create_dir_all(&path).unwrap();
        path
    }
    fn write(root: &Path, name: &str, value: &str) {
        let path = root.join(name);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, value).unwrap();
    }
    #[test]
    fn power_supply_keeps_charge_and_energy_units_distinct() {
        let root = fixture("power");
        for (name, value) in [
            ("type", "Battery"),
            ("capacity", "75"),
            ("status", "Discharging"),
            ("charge_full", "5000000"),
            ("voltage_min_design", "12000000"),
        ] {
            write(&root, &format!("BAT0/{name}"), value);
        }
        let first = battery(&root).0.unwrap();
        assert_eq!(first.percent, 75.0);
        assert_eq!(first.full_charged_capacity_mwh, None);
        assert_eq!(first.design_voltage_mv, Some(12000));
        write(&root, "BAT0/energy_full", "60000000");
        assert_eq!(
            battery(&root).0.unwrap().full_charged_capacity_mwh,
            Some(60000)
        );
        write(&root, "BAT0/capacity", "101");
        assert!(battery(&root).0.is_none());
        fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn missing_edid_and_speed_are_not_fabricated() {
        let root = fixture("display-net");
        write(&root, "card0-DP-1/status", "connected");
        write(&root, "card0-DP-1/enabled", "disabled");
        let data = displays(&root, &root.join("backlight"));
        assert_eq!(data.displays.len(), 1);
        assert_eq!(data.displays[0].active, Some(false));
        assert_eq!(data.displays[0].physical_width_cm, None);
        fs::create_dir_all(root.join("eth0/device")).unwrap();
        write(&root, "eth0/speed", "-1");
        assert_eq!(adapters(&root).0[0].link_speed_bps, None);
        write(&root, "eth0/speed", "2500");
        assert_eq!(adapters(&root).0[0].link_speed_bps, Some(2_500_000_000));
        fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn arm_identity_uses_device_tree_without_inventing_firmware() {
        let root = fixture("arm");
        write(&root, "tree/model", "Fixture ARM board\0");
        let mut data = SystemInfoData::default();
        hardware(&mut data, &root.join("dmi"), &root.join("tree"));
        assert_eq!(data.model.as_deref(), Some("Fixture ARM board"));
        assert!(data.bios_version.is_none());
        fs::remove_dir_all(root).unwrap();
    }
}
