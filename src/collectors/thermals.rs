use sysinfo::{Components, System};

use super::{DiagnosticWarning, WarningSeverity};

#[derive(Debug, Clone, Default)]
pub struct ThermalData {
    pub cpu_temp: Option<f64>,
    pub gpu_temp: Option<f64>,
    pub sensors: Vec<SensorInfo>,
    pub fans: Vec<FanInfo>,
    pub battery: Option<BatteryInfo>,
    pub power_source: PowerSource,
}

#[derive(Debug, Clone)]
pub struct SensorInfo {
    pub label: String,
    pub temperature: f64,
    pub critical: Option<f64>,
}

#[derive(Debug, Clone)]
pub struct FanInfo {
    pub label: String,
    pub rpm: u64,
}

#[derive(Debug, Clone)]
pub struct BatteryInfo {
    pub percent: f64,
    pub is_charging: bool,
    pub time_remaining: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub enum PowerSource {
    #[default]
    Unknown,
    Ac,
    Battery,
}

pub fn collect(_sys: &System) -> (ThermalData, Vec<DiagnosticWarning>) {
    let components = Components::new_with_refreshed_list();
    let mut warnings = Vec::new();

    let mut cpu_temp: Option<f64> = None;
    let mut gpu_temp: Option<f64> = None;
    let mut sensors = Vec::new();

    for component in &components {
        let label = component.label().to_string();
        let temp = component.temperature() as f64;
        let critical = component.critical().map(|t| t as f64);

        // Identify CPU and GPU temps
        let label_lower = label.to_lowercase();
        if (label_lower.contains("cpu") || label_lower.contains("tctl") || label_lower.contains("coretemp") || label_lower.contains("package"))
            && cpu_temp.is_none_or(|current| temp > current)
        {
            cpu_temp = Some(temp);
        }
        if (label_lower.contains("gpu") || label_lower.contains("nvidia") || label_lower.contains("radeon"))
            && gpu_temp.is_none_or(|current| temp > current)
        {
            gpu_temp = Some(temp);
        }

        sensors.push(SensorInfo {
            label,
            temperature: temp,
            critical,
        });
    }

    let mut fans = Vec::new();

    // WMI fallback on Windows when sysinfo returns empty
    #[cfg(windows)]
    {
        if sensors.is_empty() {
            let (wmi_sensors, wmi_fans, wmi_warnings) = collect_wmi_thermals();
            warnings.extend(wmi_warnings);
            sensors = wmi_sensors;
            fans = wmi_fans;

            // Extract CPU temp from WMI sensors
            for sensor in &sensors {
                let label_lower = sensor.label.to_lowercase();
                if (label_lower.contains("thermal zone") || label_lower.contains("cpu") || label_lower.contains("acpi"))
                    && cpu_temp.is_none_or(|current| sensor.temperature > current)
                {
                    cpu_temp = Some(sensor.temperature);
                }
            }

            if sensors.is_empty() {
                warnings.push(DiagnosticWarning {
                    source: "Thermals".into(),
                    message: "No temperature sensors detected. Try running as Administrator.".into(),
                    severity: WarningSeverity::Warning,
                });
            }
        }
    }

    #[cfg(not(windows))]
    {
        if sensors.is_empty() {
            warnings.push(DiagnosticWarning {
                source: "Thermals".into(),
                message: "No temperature sensors detected".into(),
                severity: WarningSeverity::Info,
            });
        }
    }

    // Battery info — platform specific
    let battery = collect_battery();
    let power_source = if let Some(ref bat) = battery {
        if bat.is_charging {
            PowerSource::Ac
        } else {
            PowerSource::Battery
        }
    } else {
        PowerSource::Unknown
    };

    let data = ThermalData {
        cpu_temp,
        gpu_temp,
        sensors,
        fans,
        battery,
        power_source,
    };

    (data, warnings)
}

// --- WMI fallback for Windows ---

#[cfg(windows)]
use serde::Deserialize;

#[cfg(windows)]
#[derive(Deserialize, Debug)]
#[serde(rename_all = "PascalCase")]
struct WmiThermalZone {
    instance_name: Option<String>,
    current_temperature: Option<u32>,
    critical_trip_point: Option<u32>,
}

#[cfg(windows)]
#[derive(Deserialize, Debug)]
#[serde(rename = "Win32_Fan")]
#[serde(rename_all = "PascalCase")]
struct WmiFan {
    name: Option<String>,
    desired_speed: Option<u64>,
}

#[cfg(windows)]
fn collect_wmi_thermals() -> (Vec<SensorInfo>, Vec<FanInfo>, Vec<DiagnosticWarning>) {
    use wmi::{COMLibrary, WMIConnection};

    let mut sensors = Vec::new();
    let mut fans = Vec::new();
    let mut warnings = Vec::new();

    let com = match COMLibrary::new() {
        Ok(c) => c,
        Err(e) => {
            warnings.push(DiagnosticWarning {
                source: "Thermals".into(),
                message: format!("COM init failed: {} — run as Administrator", e),
                severity: WarningSeverity::Warning,
            });
            return (sensors, fans, warnings);
        }
    };

    // Query thermal zones from root\WMI namespace
    match WMIConnection::with_namespace_path("root\\WMI", com) {
        Ok(wmi_conn) => {
            match wmi_conn.raw_query::<WmiThermalZone>(
                "SELECT InstanceName, CurrentTemperature, CriticalTripPoint FROM MSAcpi_ThermalZoneTemperature"
            ) {
                Ok(zones) => {
                    for zone in zones {
                        if let Some(raw_temp) = zone.current_temperature {
                            // Convert from tenths-of-Kelvin to Celsius
                            let celsius = (raw_temp as f64 / 10.0) - 273.15;
                            // Sanity check: 0-150C range
                            if (0.0..=150.0).contains(&celsius) {
                                let label = zone.instance_name
                                    .unwrap_or_else(|| "Thermal Zone".into());
                                let critical = zone.critical_trip_point.map(|c| {
                                    (c as f64 / 10.0) - 273.15
                                });
                                sensors.push(SensorInfo {
                                    label,
                                    temperature: celsius,
                                    critical,
                                });
                            }
                        }
                    }
                }
                Err(e) => {
                    warnings.push(DiagnosticWarning {
                        source: "Thermals".into(),
                        message: format!("WMI thermal query failed: {} — run as Administrator", e),
                        severity: WarningSeverity::Warning,
                    });
                }
            }
        }
        Err(e) => {
            warnings.push(DiagnosticWarning {
                source: "Thermals".into(),
                message: format!("WMI namespace root\\WMI unavailable: {}", e),
                severity: WarningSeverity::Warning,
            });
        }
    }

    // Query fans from root\cimv2 (needs a separate COM init)
    if let Ok(com2) = wmi::COMLibrary::new() {
        if let Ok(wmi_conn) = WMIConnection::new(com2) {
            if let Ok(wmi_fans) = wmi_conn.raw_query::<WmiFan>(
                "SELECT Name, DesiredSpeed FROM Win32_Fan"
            ) {
                for fan in wmi_fans {
                    fans.push(FanInfo {
                        label: fan.name.unwrap_or_else(|| "Fan".into()),
                        rpm: fan.desired_speed.unwrap_or(0),
                    });
                }
            }
        }
    }

    (sensors, fans, warnings)
}

// --- Battery collection ---

fn collect_battery() -> Option<BatteryInfo> {
    #[cfg(windows)]
    {
        collect_battery_windows()
    }
    #[cfg(not(windows))]
    {
        None
    }
}

#[cfg(windows)]
fn collect_battery_windows() -> Option<BatteryInfo> {
    let output = std::process::Command::new("powershell")
        .args([
            "-NoProfile",
            "-Command",
            "(Get-WmiObject Win32_Battery | Select-Object EstimatedChargeRemaining, BatteryStatus | ConvertTo-Json)",
        ])
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stdout = stdout.trim();
    if stdout.is_empty() || stdout == "null" {
        return None;
    }

    let percent: f64 = extract_json_number(stdout, "EstimatedChargeRemaining")?;
    let status: f64 = extract_json_number(stdout, "BatteryStatus")?;

    Some(BatteryInfo {
        percent,
        is_charging: status as u32 == 2,
        time_remaining: None,
    })
}

#[cfg(windows)]
fn extract_json_number(json: &str, key: &str) -> Option<f64> {
    let pattern = format!("\"{}\"", key);
    let idx = json.find(&pattern)?;
    let rest = &json[idx + pattern.len()..];
    let rest = rest.trim_start().strip_prefix(':')?;
    let rest = rest.trim_start();
    let end = rest.find(|c: char| !c.is_ascii_digit() && c != '.').unwrap_or(rest.len());
    rest[..end].parse().ok()
}
