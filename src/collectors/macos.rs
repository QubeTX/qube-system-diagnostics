//! Structured macOS inventory and IOKit registry decoding. Parsers are also
//! compiled by fixtures on other platforms; probing is macOS-only.
use super::disk_activity::{CounterFrame, DeviceCounters};
use crate::observation::Observation;
use plist::Value;

pub fn parse_plist(bytes: &[u8]) -> Result<Value, String> {
    Value::from_reader(std::io::Cursor::new(bytes)).map_err(|error| error.to_string())
}

#[cfg(target_os = "macos")]
pub fn command_plist(program: &str, args: &[&str]) -> Result<Value, String> {
    let output = super::command::run_checked(
        program,
        args,
        super::command::CommandTimeout::Slow,
        &std::sync::atomic::AtomicBool::new(false),
    )
    .map_err(|e| e.to_string())?;
    if !output.status.success() {
        return Err(format!("{program} exited {}", output.status));
    }
    parse_plist(&output.stdout)
}

pub fn physical_disks(list: &Value) -> Vec<String> {
    list.as_dictionary()
        .and_then(|v| v.get("AllDisksAndPartitions"))
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|v| v.as_dictionary())
        .filter_map(|d| d.get("DeviceIdentifier").and_then(Value::as_string))
        .filter(|id| valid_disk_id(id))
        .map(str::to_owned)
        .collect()
}

pub fn valid_disk_id(id: &str) -> bool {
    id.strip_prefix("disk")
        .is_some_and(|tail| !tail.is_empty() && tail.bytes().all(|b| b.is_ascii_digit()))
}

fn whole_disk(value: &Value) -> Option<&str> {
    if let Some(dict) = value.as_dictionary() {
        if dict.get("Whole").and_then(Value::as_boolean) == Some(true) {
            if let Some(name) = dict
                .get("BSD Name")
                .and_then(Value::as_string)
                .filter(|id| valid_disk_id(id))
            {
                return Some(name);
            }
        }
        if let Some(children) = dict
            .get("IORegistryEntryChildren")
            .and_then(Value::as_array)
        {
            return children.iter().find_map(whole_disk);
        }
    }
    None
}

pub fn parse_disk_counters(registry: &Value) -> CounterFrame {
    let mut devices = Vec::new();
    fn walk(value: &Value, devices: &mut Vec<DeviceCounters>) {
        if let Some(values) = value.as_array() {
            for value in values {
                walk(value, devices);
            }
        }
        if let Some(dict) = value.as_dictionary() {
            if let Some(stats) = dict.get("Statistics").and_then(Value::as_dictionary) {
                if let Some(name) = whole_disk(value) {
                    let number = |key| stats.get(key).and_then(Value::as_unsigned_integer);
                    if let (Some(read_bytes), Some(write_bytes)) =
                        (number("Bytes (Read)"), number("Bytes (Write)"))
                    {
                        let registry_id = dict
                            .get("IORegistryEntryID")
                            .and_then(Value::as_unsigned_integer)
                            .unwrap_or(0);
                        devices.push(DeviceCounters {
                            device_id: format!("/dev/{name}"),
                            identity: format!("iokit:{registry_id}:{name}"),
                            read_bytes,
                            write_bytes,
                            reads: number("Operations (Read)"),
                            writes: number("Operations (Write)"),
                            read_time_ns: number("Total Time (Read)"),
                            write_time_ns: number("Total Time (Write)"),
                            queue_depth: None,
                        });
                        return; // Do not sum partition/backing layers again.
                    }
                }
            }
            if let Some(children) = dict.get("IORegistryEntryChildren") {
                walk(children, devices);
            }
        }
    }
    walk(registry, &mut devices);
    let observation = if devices.is_empty() {
        Observation::unavailable(
            "IOBlockStorageDriver.Statistics",
            "This storage driver does not publish readable whole-device counters",
        )
    } else {
        Observation::available("IOBlockStorageDriver.Statistics; bytes and nanoseconds")
    };
    CounterFrame {
        devices,
        observation,
    }
}

#[cfg(target_os = "macos")]
pub fn disk_counters() -> CounterFrame {
    match command_plist(
        "/usr/sbin/ioreg",
        &["-a", "-r", "-c", "IOBlockStorageDriver"],
    ) {
        Ok(value) => parse_disk_counters(&value),
        Err(error) => CounterFrame {
            observation: Observation::error("IOKit registry", error),
            ..Default::default()
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn physical_disk_inventory_is_not_limited_to_disk_zero() {
        let value = parse_plist(br#"<?xml version="1.0"?><plist version="1.0"><dict><key>AllDisksAndPartitions</key><array><dict><key>DeviceIdentifier</key><string>disk0</string></dict><dict><key>DeviceIdentifier</key><string>disk4</string></dict><dict><key>DeviceIdentifier</key><string>disk4s1</string></dict></array></dict></plist>"#).unwrap();
        assert_eq!(physical_disks(&value), ["disk0", "disk4"]);
    }
    #[test]
    fn registry_counts_whole_disks_and_preserves_optional_counters() {
        let value = parse_plist(br#"<?xml version="1.0"?><plist version="1.0"><array><dict><key>IORegistryEntryID</key><integer>55</integer><key>Statistics</key><dict><key>Bytes (Read)</key><integer>1024</integer><key>Bytes (Write)</key><integer>2048</integer><key>Total Time (Read)</key><integer>9000000</integer></dict><key>IORegistryEntryChildren</key><array><dict><key>Whole</key><true/><key>BSD Name</key><string>disk4</string></dict></array></dict></array></plist>"#).unwrap();
        let frame = parse_disk_counters(&value);
        assert_eq!(frame.devices.len(), 1);
        assert_eq!(frame.devices[0].device_id, "/dev/disk4");
        assert_eq!(frame.devices[0].read_time_ns, Some(9_000_000));
        assert_eq!(frame.devices[0].writes, None);
    }
}
