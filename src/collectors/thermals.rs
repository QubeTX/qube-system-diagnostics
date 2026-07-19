use serde::Serialize;
use sysinfo::Components;

use super::{DiagnosticWarning, WarningSeverity};
use crate::collectors::gpu::GpuData;
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
    pub cpu_temperature_status: Observation,
    pub gpu_temperature_status: Observation,
    pub fan_status: Observation,
    pub battery_status: Observation,
}

#[derive(Debug, Clone, Serialize)]
pub struct SensorInfo {
    pub label: String,
    pub temperature: f64,
    pub critical: Option<f64>,
    pub kind: SensorKind,
    pub source: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct FanInfo {
    pub label: String,
    pub rpm: u64,
    pub source: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SensorKind {
    Cpu,
    Gpu,
    Other,
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

pub fn collect(
    components: &mut Components,
    gpu: &GpuData,
) -> (ThermalData, Vec<DiagnosticWarning>) {
    components.refresh(true);
    let mut warnings = Vec::new();

    let mut sensors = Vec::new();

    for component in components.iter() {
        let label = component.label().to_string();
        let Some(temp) = component.temperature().map(|value| value as f64) else {
            continue;
        };
        let critical = component.critical().map(|t| t as f64);

        sensors.push(SensorInfo {
            kind: classify_sensor(&label, ""),
            label,
            temperature: temp,
            critical,
            source: "sysinfo components".into(),
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
        } else {
            for sensor in wmi_sensors {
                push_unique_sensor(&mut sensors, sensor);
            }
            if !sensors.iter().any(|sensor| sensor.kind == SensorKind::Cpu)
                && matches!(
                    wmi_temperature_status.status,
                    crate::observation::ObservationStatus::PermissionDenied
                        | crate::observation::ObservationStatus::Error
                )
            {
                temperature_status = wmi_temperature_status;
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

    if let Some((temperature, label, source)) = gpu_temperature_sensor(gpu) {
        push_unique_sensor(
            &mut sensors,
            SensorInfo {
                label,
                temperature,
                critical: None,
                kind: SensorKind::Gpu,
                source,
            },
        );
    }

    let cpu_temp = hottest_temperature(&sensors, SensorKind::Cpu);
    let gpu_temp = hottest_temperature(&sensors, SensorKind::Gpu);
    let cpu_temperature_status = temperature_observation_for_kind(
        &sensors,
        SensorKind::Cpu,
        &temperature_status,
        "No provider returned an identifiable CPU temperature",
    );
    let gpu_temperature_status = temperature_observation_for_kind(
        &sensors,
        SensorKind::Gpu,
        &gpu.telemetry_status,
        "No provider returned an identifiable GPU temperature",
    );
    if !sensors.is_empty() {
        temperature_status = Observation::available(sensor_sources(&sensors));
    }

    if !cpu_temperature_status.is_available() {
        let message = if gpu_temperature_status.is_available() {
            "CPU temperature telemetry is unavailable through the current providers; GPU temperature telemetry remains available"
        } else {
            "CPU temperature telemetry is unavailable through the current providers; vendor or privileged monitor access may differ"
        };
        warnings.push(DiagnosticWarning {
            source: "Thermals".into(),
            message: message.into(),
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
        cpu_temperature_status,
        gpu_temperature_status,
        fan_status,
        battery_status,
    };

    (data, warnings)
}

fn classify_sensor(label: &str, identity: &str) -> SensorKind {
    let haystack = format!("{} {}", label, identity).to_ascii_lowercase();
    if haystack.contains("/intelcpu/")
        || haystack.contains("/amdcpu/")
        || haystack.contains("cpu")
        || haystack.contains("coretemp")
        || haystack.contains("package")
        || haystack.contains("tctl")
        || haystack.contains("tdie")
    {
        SensorKind::Cpu
    } else if haystack.contains("/gpu")
        || haystack.contains("gpu")
        || haystack.contains("nvidia")
        || haystack.contains("radeon")
    {
        SensorKind::Gpu
    } else {
        SensorKind::Other
    }
}

fn push_unique_sensor(sensors: &mut Vec<SensorInfo>, sensor: SensorInfo) {
    let duplicate = sensors.iter().any(|current| {
        current.kind == sensor.kind
            && current.label.eq_ignore_ascii_case(&sensor.label)
            && (current.temperature - sensor.temperature).abs() < 0.1
    });
    if !duplicate {
        sensors.push(sensor);
    }
}

fn hottest_temperature(sensors: &[SensorInfo], kind: SensorKind) -> Option<f64> {
    sensors
        .iter()
        .filter(|sensor| sensor.kind == kind)
        .map(|sensor| sensor.temperature)
        .reduce(f64::max)
}

fn sensor_sources(sensors: &[SensorInfo]) -> String {
    let mut sources = Vec::new();
    for sensor in sensors {
        if !sources.iter().any(|source| source == &sensor.source) {
            sources.push(sensor.source.clone());
        }
    }
    sources.join(" + ")
}

fn temperature_observation_for_kind(
    sensors: &[SensorInfo],
    kind: SensorKind,
    fallback: &Observation,
    unavailable_detail: &str,
) -> Observation {
    let matching = sensors
        .iter()
        .filter(|sensor| sensor.kind == kind)
        .cloned()
        .collect::<Vec<_>>();
    if matching.is_empty() {
        if matches!(
            fallback.status,
            crate::observation::ObservationStatus::PermissionDenied
                | crate::observation::ObservationStatus::Error
        ) {
            fallback.clone()
        } else {
            Observation::unavailable(fallback.source.clone(), unavailable_detail)
        }
    } else {
        Observation::available(sensor_sources(&matching))
    }
}

fn gpu_temperature_sensor(gpu: &GpuData) -> Option<(f64, String, String)> {
    let adapter = gpu
        .adapters
        .iter()
        .find(|adapter| adapter.temperature_celsius.is_some())?;
    let temperature = adapter.temperature_celsius?;
    (-50.0..=200.0).contains(&temperature).then(|| {
        (
            temperature,
            format!("GPU: {}", adapter.name),
            adapter.source.clone(),
        )
    })
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
#[derive(Deserialize, Debug)]
#[serde(rename = "Sensor")]
#[serde(rename_all = "PascalCase")]
struct WmiHardwareMonitorSensor {
    name: Option<String>,
    identifier: Option<String>,
    parent: Option<String>,
    sensor_type: Option<String>,
    value: Option<f32>,
}

#[cfg(windows)]
#[derive(Deserialize, Debug)]
#[serde(rename = "AWCCWmiMethodFunction")]
#[serde(rename_all = "PascalCase")]
struct AwccWmiMethodFunction {
    #[serde(rename = "__Path")]
    path: String,
    active: Option<bool>,
}

#[cfg(windows)]
#[derive(Serialize)]
struct AwccMethodInput {
    arg2: u32,
}

#[cfg(windows)]
#[derive(Deserialize)]
struct AwccMethodOutput {
    #[serde(rename = "ReturnValue")]
    return_value: bool,
    argr: u32,
}

#[cfg(windows)]
#[derive(Clone)]
struct WindowsThermalReadings {
    sensors: Vec<SensorInfo>,
    fans: Vec<FanInfo>,
    temperature_status: Observation,
    fan_status: Observation,
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

    let mut warnings = Vec::new();

    let bridge_readings = collect_hardware_monitor_bridge();
    let awcc_readings = collect_awcc_thermals();
    if let Some(mut readings) = bridge_readings.clone() {
        if let Some(awcc) = awcc_readings.as_ref() {
            if readings.sensors.is_empty() && !awcc.sensors.is_empty() {
                readings.sensors = awcc.sensors.clone();
                readings.temperature_status = awcc.temperature_status.clone();
            }
            if readings.fans.is_empty() && !awcc.fans.is_empty() {
                readings.fans = awcc.fans.clone();
                readings.fan_status = awcc.fan_status.clone();
            }
        }
        if !readings.sensors.is_empty() || !readings.fans.is_empty() {
            return (
                readings.sensors,
                readings.fans,
                readings.temperature_status,
                readings.fan_status,
                warnings,
            );
        }
    }
    if let Some(readings) = awcc_readings.as_ref() {
        if !readings.sensors.is_empty() || !readings.fans.is_empty() {
            return (
                readings.sensors.clone(),
                readings.fans.clone(),
                readings.temperature_status.clone(),
                readings.fan_status.clone(),
                warnings,
            );
        }
    }

    let com = match COMLibrary::new() {
        Ok(c) => c,
        Err(e) => {
            let status = wmi_error_observation("Windows thermal providers", &e.to_string());
            return (Vec::new(), Vec::new(), status.clone(), status, warnings);
        }
    };

    let mut sensors = Vec::new();
    let mut temperature_status = match WMIConnection::with_namespace_path("root\\WMI", com) {
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
                                    kind: classify_sensor(&label, ""),
                                    label,
                                    temperature: celsius,
                                    critical,
                                    source: "MSAcpi_ThermalZoneTemperature".into(),
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

    let mut fan_status = if let Ok(com2) = wmi::COMLibrary::new() {
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

    for readings in [bridge_readings.as_ref(), awcc_readings.as_ref()]
        .into_iter()
        .flatten()
    {
        if !temperature_status.is_available()
            && matches!(
                readings.temperature_status.status,
                crate::observation::ObservationStatus::PermissionDenied
                    | crate::observation::ObservationStatus::Error
            )
        {
            temperature_status = readings.temperature_status.clone();
        }
        if !fan_status.is_available()
            && matches!(
                readings.fan_status.status,
                crate::observation::ObservationStatus::PermissionDenied
                    | crate::observation::ObservationStatus::Error
            )
        {
            fan_status = readings.fan_status.clone();
        }
    }

    if matches!(
        fan_status.status,
        crate::observation::ObservationStatus::Error
            | crate::observation::ObservationStatus::Contradictory
    ) {
        warnings.push(DiagnosticWarning {
            source: "Thermals".into(),
            message: "Windows fan telemetry provider returned an error".into(),
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

#[cfg(windows)]
fn collect_hardware_monitor_bridge() -> Option<WindowsThermalReadings> {
    use wmi::{COMLibrary, WMIConnection};

    for (namespace, source) in [
        ("root\\LibreHardwareMonitor", "LibreHardwareMonitor WMI"),
        ("root\\OpenHardwareMonitor", "OpenHardwareMonitor WMI"),
    ] {
        let com = COMLibrary::new().ok()?;
        let connection = match WMIConnection::with_namespace_path(namespace, com) {
            Ok(connection) => connection,
            Err(error) if wmi_namespace_absent(&error.to_string()) => continue,
            Err(error) => {
                let status = wmi_error_observation(source, &error.to_string());
                return Some(WindowsThermalReadings {
                    sensors: Vec::new(),
                    fans: Vec::new(),
                    temperature_status: status.clone(),
                    fan_status: status,
                });
            }
        };
        let rows = match connection.raw_query::<WmiHardwareMonitorSensor>(
            "SELECT Name, Identifier, Parent, SensorType, Value FROM Sensor",
        ) {
            Ok(rows) => rows,
            Err(error) => {
                let status = wmi_error_observation(source, &error.to_string());
                return Some(WindowsThermalReadings {
                    sensors: Vec::new(),
                    fans: Vec::new(),
                    temperature_status: status.clone(),
                    fan_status: status,
                });
            }
        };

        let mut sensors = Vec::new();
        let mut fans = Vec::new();
        for row in rows {
            let Some(value) = row.value.map(f64::from).filter(|value| value.is_finite()) else {
                continue;
            };
            let name = row.name.unwrap_or_else(|| "Hardware sensor".into());
            let identity = format!(
                "{} {}",
                row.identifier.as_deref().unwrap_or_default(),
                row.parent.as_deref().unwrap_or_default()
            );
            match row
                .sensor_type
                .as_deref()
                .unwrap_or_default()
                .to_ascii_lowercase()
                .as_str()
            {
                "temperature" if (-50.0..=200.0).contains(&value) => {
                    sensors.push(SensorInfo {
                        kind: classify_sensor(&name, &identity),
                        label: name,
                        temperature: value,
                        critical: None,
                        source: source.into(),
                    });
                }
                "fan" if (0.0..=100_000.0).contains(&value) => fans.push(FanInfo {
                    label: name,
                    rpm: value.round() as u64,
                    source: source.into(),
                }),
                _ => {}
            }
        }

        let temperature_status = if sensors.is_empty() {
            Observation::unavailable(source, "The WMI bridge returned no temperature sensors")
        } else {
            Observation::available(source)
        };
        let fan_status = if fans.is_empty() {
            Observation::unavailable(source, "The WMI bridge returned no fan-speed sensors")
        } else {
            Observation::available(source)
        };
        return Some(WindowsThermalReadings {
            sensors,
            fans,
            temperature_status,
            fan_status,
        });
    }
    None
}

#[cfg(windows)]
fn collect_awcc_thermals() -> Option<WindowsThermalReadings> {
    use wmi::{COMLibrary, WMIConnection};

    const SOURCE: &str = "Dell AWCC Thermal_Information (read-only)";
    let com = COMLibrary::new().ok()?;
    let connection = match WMIConnection::with_namespace_path("root\\WMI", com) {
        Ok(connection) => connection,
        Err(error) => {
            let status = wmi_error_observation(SOURCE, &error.to_string());
            return Some(empty_windows_readings(status));
        }
    };
    let instances = match connection
        .raw_query::<AwccWmiMethodFunction>("SELECT __Path, Active FROM AWCCWmiMethodFunction")
    {
        Ok(instances) => instances,
        Err(error) if wmi_class_absent(&error.to_string()) => return None,
        Err(error) => {
            let status = wmi_error_observation(SOURCE, &error.to_string());
            let status = if matches!(
                status.status,
                crate::observation::ObservationStatus::PermissionDenied
            ) {
                Observation::permission_denied(
                    SOURCE,
                    "Dell's AWCC thermal sensor interface is present but requires Administrator access",
                )
            } else {
                status
            };
            return Some(empty_windows_readings(status));
        }
    };
    let instance = instances
        .iter()
        .find(|instance| instance.active == Some(true))
        .or_else(|| instances.first())?;

    let description = match awcc_thermal_call(&connection, &instance.path, 0x02) {
        Ok(value) => value,
        Err(error) => return Some(empty_windows_readings(Observation::error(SOURCE, error))),
    };
    let (fan_count, sensor_count) = decode_awcc_description(description);
    if fan_count > 16 || sensor_count > 32 {
        return Some(empty_windows_readings(Observation::contradictory(
            SOURCE,
            format!(
                "Firmware returned implausible sensor counts: {fan_count} fans, {sensor_count} temperatures"
            ),
        )));
    }

    let mut fan_ids = Vec::new();
    let mut sensor_ids = Vec::new();
    for index in 0..fan_count.saturating_add(sensor_count) {
        let argument = awcc_argument(0x03, index);
        let Ok(id) = awcc_thermal_call(&connection, &instance.path, argument) else {
            continue;
        };
        let id = (id & 0xff) as u8;
        if index < fan_count {
            fan_ids.push(id);
        } else {
            sensor_ids.push(id);
        }
    }

    let mut sensors = Vec::new();
    for id in sensor_ids {
        let Ok(raw) = awcc_thermal_call(&connection, &instance.path, awcc_argument(0x04, id))
        else {
            continue;
        };
        let temperature = f64::from(raw);
        if (-50.0..=200.0).contains(&temperature) {
            sensors.push(SensorInfo {
                label: format!("Dell thermal sensor 0x{id:02X}"),
                temperature,
                critical: None,
                kind: SensorKind::Other,
                source: SOURCE.into(),
            });
        }
    }

    let mut fans = Vec::new();
    for id in fan_ids {
        let Ok(raw) = awcc_thermal_call(&connection, &instance.path, awcc_argument(0x05, id))
        else {
            continue;
        };
        if raw <= 100_000 {
            fans.push(FanInfo {
                label: format!("Dell fan 0x{id:02X}"),
                rpm: u64::from(raw),
                source: SOURCE.into(),
            });
        }
    }

    let temperature_status = if sensors.is_empty() {
        Observation::unavailable(SOURCE, "The Dell interface returned no valid temperatures")
    } else {
        Observation::available(SOURCE)
    };
    let fan_status = if fans.is_empty() {
        Observation::unavailable(
            SOURCE,
            "The Dell interface returned no valid fan RPM values",
        )
    } else {
        Observation::available(SOURCE)
    };
    Some(WindowsThermalReadings {
        sensors,
        fans,
        temperature_status,
        fan_status,
    })
}

#[cfg(windows)]
fn empty_windows_readings(status: Observation) -> WindowsThermalReadings {
    WindowsThermalReadings {
        sensors: Vec::new(),
        fans: Vec::new(),
        temperature_status: status.clone(),
        fan_status: status,
    }
}

#[cfg(windows)]
fn awcc_thermal_call(
    connection: &wmi::WMIConnection,
    path: &str,
    argument: u32,
) -> Result<u32, String> {
    let output = connection
        .exec_instance_method::<AwccWmiMethodFunction, _, AwccMethodOutput>(
            "Thermal_Information",
            path,
            AwccMethodInput { arg2: argument },
        )
        .map_err(|error| error.to_string())?;
    if output.return_value {
        Ok(output.argr)
    } else {
        Err(format!(
            "Dell firmware rejected read operation 0x{:02X}",
            argument & 0xff
        ))
    }
}

#[cfg(windows)]
fn awcc_argument(operation: u8, value: u8) -> u32 {
    u32::from(operation) | (u32::from(value) << 8)
}

#[cfg(windows)]
fn decode_awcc_description(value: u32) -> (u8, u8) {
    ((value & 0xff) as u8, ((value >> 8) & 0xff) as u8)
}

#[cfg(windows)]
fn wmi_namespace_absent(error: &str) -> bool {
    error.to_ascii_lowercase().contains("0x8004100e")
}

#[cfg(windows)]
fn wmi_class_absent(error: &str) -> bool {
    let normalized = error.to_ascii_lowercase();
    normalized.contains("0x80041010") || normalized.contains("invalid class")
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

#[cfg(test)]
mod tests {
    use super::*;

    fn sensor(label: &str, kind: SensorKind, source: &str) -> SensorInfo {
        SensorInfo {
            label: label.into(),
            temperature: 55.0,
            critical: None,
            kind,
            source: source.into(),
        }
    }

    #[test]
    fn classifies_hardware_monitor_identifiers_without_guessing_generic_zones() {
        assert_eq!(
            classify_sensor("CPU Package", "/intelcpu/0/temperature/0"),
            SensorKind::Cpu
        );
        assert_eq!(
            classify_sensor("GPU Core", "/gpu-nvidia/0/temperature/0"),
            SensorKind::Gpu
        );
        assert_eq!(
            classify_sensor("Thermal Zone 0", "ACPI\\ThermalZone"),
            SensorKind::Other
        );
    }

    #[test]
    fn per_device_observation_preserves_permission_failure_until_data_exists() {
        let denied = Observation::permission_denied("vendor", "Administrator required");
        assert_eq!(
            temperature_observation_for_kind(&[], SensorKind::Cpu, &denied, "CPU unavailable"),
            denied
        );

        let observation = temperature_observation_for_kind(
            &[sensor("CPU Package", SensorKind::Cpu, "fixture")],
            SensorKind::Cpu,
            &denied,
            "CPU unavailable",
        );
        assert_eq!(observation, Observation::available("fixture"));
    }

    #[cfg(windows)]
    #[test]
    fn awcc_read_arguments_and_description_use_documented_byte_order() {
        assert_eq!(awcc_argument(0x04, 0xA0), 0xA004);
        assert_eq!(decode_awcc_description(0x0500_0203), (3, 2));
    }
}
