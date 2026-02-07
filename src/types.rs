/// The diagnostic display mode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiagnosticMode {
    User,
    Technician,
}

/// The 9 navigable sections
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Section {
    Overview = 1,
    Cpu = 2,
    Memory = 3,
    Disk = 4,
    Gpu = 5,
    Network = 6,
    Processes = 7,
    Thermals = 8,
    Drivers = 9,
}

impl Section {
    pub fn from_number(n: u8) -> Option<Self> {
        match n {
            1 => Some(Self::Overview),
            2 => Some(Self::Cpu),
            3 => Some(Self::Memory),
            4 => Some(Self::Disk),
            5 => Some(Self::Gpu),
            6 => Some(Self::Network),
            7 => Some(Self::Processes),
            8 => Some(Self::Thermals),
            9 => Some(Self::Drivers),
            _ => None,
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            Self::Overview => "Overview",
            Self::Cpu => "CPU",
            Self::Memory => "Mem",
            Self::Disk => "Disk",
            Self::Gpu => "GPU",
            Self::Network => "Net",
            Self::Processes => "Procs",
            Self::Thermals => "Thermals",
            Self::Drivers => "Drivers",
        }
    }

    pub fn number(&self) -> u8 {
        *self as u8
    }

    pub const ALL: [Section; 9] = [
        Self::Overview,
        Self::Cpu,
        Self::Memory,
        Self::Disk,
        Self::Gpu,
        Self::Network,
        Self::Processes,
        Self::Thermals,
        Self::Drivers,
    ];
}

/// Health status for a subsystem
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HealthStatus {
    Good,
    Warning,
    Critical,
    Unknown,
}

impl HealthStatus {
    pub fn from_percent(pct: f64) -> Self {
        if pct < 75.0 {
            Self::Good
        } else if pct < 90.0 {
            Self::Warning
        } else {
            Self::Critical
        }
    }

    pub fn icon(&self) -> &'static str {
        match self {
            Self::Good => "\u{2713}",     // ✓
            Self::Warning => "\u{26A0}",  // ⚠
            Self::Critical => "\u{2717}", // ✗
            Self::Unknown => "?",
        }
    }
}

/// Sort order for the process table
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProcessSortKey {
    Cpu,
    Memory,
    Pid,
    Name,
}

/// Temperature display unit
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TempUnit {
    Celsius,
    Fahrenheit,
}

impl TempUnit {
    pub fn toggle(&self) -> Self {
        match self {
            Self::Celsius => Self::Fahrenheit,
            Self::Fahrenheit => Self::Celsius,
        }
    }

    pub fn convert(&self, celsius: f64) -> f64 {
        match self {
            Self::Celsius => celsius,
            Self::Fahrenheit => celsius * 9.0 / 5.0 + 32.0,
        }
    }

    pub fn suffix(&self) -> &'static str {
        match self {
            Self::Celsius => "\u{00B0}C",
            Self::Fahrenheit => "\u{00B0}F",
        }
    }
}
