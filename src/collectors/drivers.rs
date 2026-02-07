pub mod platform;

/// Driver/device health data
#[derive(Debug, Clone, Default)]
pub struct DriverData {
    pub network: Vec<DeviceInfo>,
    pub bluetooth: Vec<DeviceInfo>,
    pub audio: Vec<DeviceInfo>,
    pub input: Vec<DeviceInfo>,
    pub services: Vec<ServiceInfo>,
}

#[derive(Debug, Clone)]
pub struct DeviceInfo {
    pub name: String,
    pub driver_version: String,
    pub driver_date: String,
    pub status: DeviceStatus,
    pub category: DeviceCategory,
    pub extra: String,
}

#[derive(Debug, Clone, PartialEq)]
pub enum DeviceStatus {
    Ok,
    Disabled,
    Error(String),
    NotFound,
    Unknown,
}

impl std::fmt::Display for DeviceStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Ok => write!(f, "OK"),
            Self::Disabled => write!(f, "Disabled"),
            Self::Error(msg) => write!(f, "Error: {}", msg),
            Self::NotFound => write!(f, "Not Found"),
            Self::Unknown => write!(f, "Unknown"),
        }
    }
}

impl DeviceStatus {
    pub fn icon(&self) -> &'static str {
        match self {
            Self::Ok => "\u{2713}",      // ✓
            Self::Disabled => "\u{2014}", // —
            Self::Error(_) => "\u{2717}", // ✗
            Self::NotFound => "?",
            Self::Unknown => "?",
        }
    }

    pub fn user_description(&self) -> &'static str {
        match self {
            Self::Ok => "Working",
            Self::Disabled => "Turned off",
            Self::Error(_) => "Not working properly",
            Self::NotFound => "Not detected",
            Self::Unknown => "Unknown status",
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum DeviceCategory {
    Network,
    Bluetooth,
    Audio,
    Input,
}

#[derive(Debug, Clone)]
pub struct ServiceInfo {
    pub name: String,
    pub display_name: String,
    pub is_running: bool,
}

pub fn collect() -> DriverData {
    platform::collect_drivers()
}
