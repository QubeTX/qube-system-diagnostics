use serde::Serialize;
use sysinfo::Components;

use super::{DiagnosticWarning, WarningSeverity};
use crate::observation::Observation;

#[derive(Debug, Clone, Default, Serialize)]
pub struct ThermalData {
    pub cpu_temp: Option<f64>,
    pub gpu_temp: Option<f64>,
    pub sensors: Vec<SensorInfo>,
    pub fans: Vec<FanInfo>,
    pub battery: Option<BatteryInfo>,
    pub power_source: PowerSource,
    pub temperature_status: Observation,
    pub fan_status: Observation,
    pub battery_status: Observation,
}

#[derive(Debug, Clone, Serialize)]
pub struct SensorInfo {
    pub label: String,
    pub temperature: f64,
    pub critical: Option<f64>,
}

#[derive(Debug, Clone, Serialize)]
pub struct FanInfo {
    pub label: String,
    pub rpm: u64,
}

#[derive(Debug, Clone, Serialize)]
pub struct BatteryInfo {
    pub percent: f64,
    pub is_charging: bool,
    pub is_on_ac: bool,
    pub time_remaining: Option<String>,
    pub full_charged_capacity_mwh: Option<u64>,
    pub design_voltage_mv: Option<u64>,
    pub cycle_count: Option<u32>,
    pub provider_status: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PowerSource {
    #[default]
    Unknown,
    Ac,
    Battery,
}

pub fn collect(components: &mut Components) -> (ThermalData, Vec<DiagnosticWarning>) {
    components.refresh(true);
    let mut warnings = Vec::new();

    let mut cpu_temp: Option<f64> = None;
    let mut gpu_temp: Option<f64> = None;
    let mut sensors = Vec::new();

    for component in components.iter() {
        let label = component.label().to_string();
        let Some(temp) = component.temperature().map(|value| value as f64) else {
            continue;
        };
        let critical = component.critical().map(|t| t as f64);

        // Identify CPU and GPU temps
        let label_lower = label.to_lowercase();
        if (label_lower.contains("cpu")
            || label_lower.contains("tctl")
            || label_lower.contains("coretemp")
            || label_lower.contains("package"))
            && cpu_temp.is_none_or(|current| temp > current)
        {
            cpu_temp = Some(temp);
        }
        if (label_lower.contains("gpu")
            || label_lower.contains("nvidia")
            || label_lower.contains("radeon"))
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

    #[cfg_attr(not(windows), allow(unused_mut))]
    let mut temperature_status = if sensors.is_empty() {
        Observation::unavailable("sysinfo components", "No temperature values were returned")
    } else {
        Observation::available("sysinfo components")
    };

    #[cfg(windows)]
    let (fans, fan_status) = {
        let (wmi_sensors, wmi_fans, wmi_temperature_status, wmi_fan_status, wmi_warnings) =
            collect_wmi_thermals();
        warnings.extend(wmi_warnings);
        if sensors.is_empty() {
            sensors = wmi_sensors;
            temperature_status = wmi_temperature_status;
            for sensor in &sensors {
                let label_lower = sensor.label.to_lowercase();
                if (label_lower.contains("thermal zone")
                    || label_lower.contains("cpu")
                    || label_lower.contains("acpi"))
                    && cpu_temp.is_none_or(|current| sensor.temperature > current)
                {
                    cpu_temp = Some(sensor.temperature);
                }
            }
        }
        (wmi_fans, wmi_fan_status)
    };

    #[cfg(not(windows))]
    let (fans, fan_status) = (
        Vec::new(),
        Observation::unsupported(
            "platform fan provider",
            "Fan speed collection is not implemented on this platform",
        ),
    );

    if !temperature_status.is_available() {
        warnings.push(DiagnosticWarning {
            source: "Thermals".into(),
            message: "Temperature telemetry is unavailable on this hardware/provider combination"
                .into(),
            severity: WarningSeverity::Info,
        });
    }

    let (battery, battery_status) = collect_battery();
    let power_source = if let Some(ref bat) = battery {
        if bat.is_on_ac {
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
        temperature_status,
        fan_status,
        battery_status,
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
    status: Option<String>,
    availability: Option<u16>,
}

#[cfg(windows)]
fn collect_wmi_thermals() -> (
    Vec<SensorInfo>,
    Vec<FanInfo>,
    Observation,
    Observation,
    Vec<DiagnosticWarning>,
) {
    use wmi::{COMLibrary, WMIConnection};

    let mut sensors = Vec::new();
    let mut warnings = Vec::new();

    let com = match COMLibrary::new() {
        Ok(c) => c,
        Err(e) => {
            let status = wmi_error_observation("Windows thermal providers", &e.to_string());
            return (sensors, Vec::new(), status.clone(), status, warnings);
        }
    };

    let temperature_status = match WMIConnection::with_namespace_path("root\\WMI", com) {
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
                    if sensors.is_empty() {
                        Observation::unavailable(
                            "MSAcpi_ThermalZoneTemperature",
                            "The firmware provider returned no usable temperature rows",
                        )
                    } else {
                        Observation::available("MSAcpi_ThermalZoneTemperature")
                    }
                }
                Err(e) => {
                    wmi_error_observation("MSAcpi_ThermalZoneTemperature", &e.to_string())
                }
            }
        }
        Err(e) => wmi_error_observation("root\\WMI", &e.to_string()),
    };

    let fan_status = if let Ok(com2) = wmi::COMLibrary::new() {
        if let Ok(wmi_conn) = WMIConnection::new(com2) {
            match wmi_conn.raw_query::<WmiFan>(
                "SELECT Name, DesiredSpeed, Status, Availability FROM Win32_Fan",
            ) {
                Ok(rows) if rows.is_empty() => {
                    Observation::unavailable("Win32_Fan", "The provider returned no fan devices")
                }
                Ok(rows) => {
                    let names = rows
                        .iter()
                        .filter_map(|row| row.name.as_deref())
                        .collect::<Vec<_>>()
                        .join(", ");
                    let provider_details = rows
                        .iter()
                        .map(|row| {
                            format!(
                                "status={}, availability={}, desired_speed={}",
                                row.status.as_deref().unwrap_or("unknown"),
                                row.availability
                                    .map(|value| value.to_string())
                                    .unwrap_or_else(|| "unknown".into()),
                                row.desired_speed
                                    .map(|value| value.to_string())
                                    .unwrap_or_else(|| "unavailable".into())
                            )
                        })
                        .collect::<Vec<_>>()
                        .join("; ");
                    Observation::unavailable(
                        "Win32_Fan",
                        format!(
                            "Fan devices ({names}) were listed, but Win32_Fan exposes desired speed rather than verified actual RPM ({provider_details})"
                        ),
                    )
                }
                Err(error) => wmi_error_observation("Win32_Fan", &error.to_string()),
            }
        } else {
            Observation::error("Win32_Fan", "Could not connect to root\\cimv2")
        }
    } else {
        Observation::error("Win32_Fan", "COM initialization failed")
    };

    if !fan_status.is_available()
        && !matches!(
            fan_status.status,
            crate::observation::ObservationStatus::Unavailable
        )
    {
        warnings.push(DiagnosticWarning {
            source: "Thermals".into(),
            message: "Windows fan telemetry provider failed".into(),
            severity: WarningSeverity::Info,
        });
    }

    (
        sensors,
        Vec::new(),
        temperature_status,
        fan_status,
        warnings,
    )
}

// --- Battery collection ---

fn collect_battery() -> (Option<BatteryInfo>, Observation) {
    #[cfg(windows)]
    {
        collect_battery_windows()
    }
    #[cfg(not(windows))]
    {
        (
            None,
            Observation::unsupported(
                "platform battery provider",
                "Battery collection is not implemented on this platform",
            ),
        )
    }
}

#[cfg(windows)]
#[derive(Deserialize, Debug)]
#[serde(rename = "Win32_Battery")]
#[serde(rename_all = "PascalCase")]
struct WmiBattery {
    estimated_charge_remaining: Option<u16>,
    battery_status: Option<u16>,
    estimated_run_time: Option<u32>,
    design_voltage: Option<u64>,
    status: Option<String>,
}

#[cfg(windows)]
#[derive(Deserialize, Debug)]
#[serde(rename_all = "PascalCase")]
struct WmiFullChargedCapacity {
    full_charged_capacity: Option<u64>,
}

#[cfg(windows)]
#[derive(Deserialize, Debug)]
#[serde(rename_all = "PascalCase")]
struct WmiBatteryCycleCount {
    cycle_count: Option<u32>,
}

#[cfg(windows)]
fn collect_battery_windows() -> (Option<BatteryInfo>, Observation) {
    use wmi::{COMLibrary, WMIConnection};

    let com = match COMLibrary::new() {
        Ok(com) => com,
        Err(error) => {
            return (
                None,
                wmi_error_observation("Win32_Battery", &error.to_string()),
            )
        }
    };
    let connection = match WMIConnection::new(com) {
        Ok(connection) => connection,
        Err(error) => {
            return (
                None,
                wmi_error_observation("Win32_Battery", &error.to_string()),
            )
        }
    };
    let rows = match connection.raw_query::<WmiBattery>(
        "SELECT EstimatedChargeRemaining, BatteryStatus, EstimatedRunTime, DesignVoltage, Status FROM Win32_Battery",
    ) {
        Ok(rows) => rows,
        Err(error) => {
            return (
                None,
                wmi_error_observation("Win32_Battery", &error.to_string()),
            )
        }
    };
    let Some(row) = rows.into_iter().next() else {
        return (
            None,
            Observation::unavailable(
                "Win32_Battery",
                "The provider returned no battery rows; this may be a desktop system",
            ),
        );
    };
    let Some(percent) = row.estimated_charge_remaining else {
        return (
            None,
            Observation::unavailable(
                "Win32_Battery",
                "A battery was listed without charge percentage",
            ),
        );
    };

    let status_code = row.battery_status.unwrap_or_default();
    let is_charging = matches!(status_code, 6..=9);
    let is_on_ac = matches!(status_code, 2 | 3 | 6..=9 | 11);
    let time_remaining = row
        .estimated_run_time
        .filter(|minutes| *minutes != 71_582_788)
        .map(|minutes| format!("{minutes} minutes"));
    let (full_charged_capacity_mwh, cycle_count) = collect_battery_details_windows();

    (
        Some(BatteryInfo {
            percent: f64::from(percent.min(100)),
            is_charging,
            is_on_ac,
            time_remaining,
            full_charged_capacity_mwh,
            design_voltage_mv: row.design_voltage,
            cycle_count,
            provider_status: row.status,
        }),
        Observation::available("Win32_Battery + root\\WMI battery classes"),
    )
}

#[cfg(windows)]
fn collect_battery_details_windows() -> (Option<u64>, Option<u32>) {
    use wmi::{COMLibrary, WMIConnection};

    let Ok(com) = COMLibrary::new() else {
        return (None, None);
    };
    let Ok(connection) = WMIConnection::with_namespace_path("root\\WMI", com) else {
        return (None, None);
    };
    let capacity = connection
        .raw_query::<WmiFullChargedCapacity>(
            "SELECT FullChargedCapacity FROM BatteryFullChargedCapacity",
        )
        .ok()
        .and_then(|rows| rows.into_iter().find_map(|row| row.full_charged_capacity));
    let cycles = connection
        .raw_query::<WmiBatteryCycleCount>("SELECT CycleCount FROM BatteryCycleCount")
        .ok()
        .and_then(|rows| rows.into_iter().find_map(|row| row.cycle_count));
    (capacity, cycles)
}

#[cfg(windows)]
fn wmi_error_observation(source: &str, error: &str) -> Observation {
    let normalized = error.to_ascii_lowercase();
    if normalized.contains("access denied") || normalized.contains("0x80041003") {
        Observation::permission_denied(source, error)
    } else if normalized.contains("0x8004100c") || normalized.contains("0x80041010") {
        Observation::unsupported(source, error)
    } else {
        Observation::error(source, error)
    }
}
