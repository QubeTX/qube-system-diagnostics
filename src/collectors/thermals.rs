use sysinfo::{Components, System};

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

pub fn collect(_sys: &System) -> ThermalData {
    let components = Components::new_with_refreshed_list();

    let mut cpu_temp: Option<f64> = None;
    let mut gpu_temp: Option<f64> = None;
    let mut sensors = Vec::new();

    for component in &components {
        let label = component.label().to_string();
        let temp = component.temperature() as f64;
        let critical = component.critical().map(|t| t as f64);

        // Identify CPU and GPU temps
        let label_lower = label.to_lowercase();
        if label_lower.contains("cpu") || label_lower.contains("tctl") || label_lower.contains("coretemp") || label_lower.contains("package") {
            if cpu_temp.map_or(true, |current| temp > current) {
                cpu_temp = Some(temp);
            }
        }
        if label_lower.contains("gpu") || label_lower.contains("nvidia") || label_lower.contains("radeon") {
            if gpu_temp.map_or(true, |current| temp > current) {
                gpu_temp = Some(temp);
            }
        }

        sensors.push(SensorInfo {
            label,
            temperature: temp,
            critical,
        });
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

    ThermalData {
        cpu_temp,
        gpu_temp,
        sensors,
        fans: Vec::new(), // Fan data often requires platform-specific APIs
        battery,
        power_source,
    }
}

fn collect_battery() -> Option<BatteryInfo> {
    // Basic battery detection — platform specific
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
    // Try to get battery info via WMI or powerprofile
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

    // Simple parsing — look for key values
    let percent: f64 = extract_json_number(&stdout, "EstimatedChargeRemaining")?;
    let status: f64 = extract_json_number(&stdout, "BatteryStatus")?;

    Some(BatteryInfo {
        percent,
        is_charging: status as u32 == 2, // 2 = AC power / charging
        time_remaining: None,
    })
}

#[cfg(windows)]
fn extract_json_number(json: &str, key: &str) -> Option<f64> {
    let pattern = format!("\"{}\"", key);
    let idx = json.find(&pattern)?;
    let rest = &json[idx + pattern.len()..];
    // Skip whitespace and colon
    let rest = rest.trim_start().strip_prefix(':')?;
    let rest = rest.trim_start();
    // Parse number
    let end = rest.find(|c: char| !c.is_ascii_digit() && c != '.').unwrap_or(rest.len());
    rest[..end].parse().ok()
}
