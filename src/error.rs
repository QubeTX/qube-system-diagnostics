use thiserror::Error;

/// Result type alias using AppError
pub type Result<T> = std::result::Result<T, AppError>;

/// Application-level errors for SD-300
#[derive(Error, Debug)]
pub enum AppError {
    /// Failed to retrieve system information
    #[error("Failed to retrieve system information: {message}")]
    SystemInfo { message: String },

    /// Platform-specific operation failed
    #[error("Platform operation failed: {message}")]
    Platform { message: String },

    /// I/O operation failed
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// Terminal/display error
    #[error("Display error: {message}")]
    Display { message: String },

    /// WMI error (Windows only)
    #[cfg(windows)]
    #[error("WMI query failed: {0}")]
    Wmi(#[from] wmi::WMIError),
}

impl AppError {
    pub fn system_info(message: impl Into<String>) -> Self {
        Self::SystemInfo {
            message: message.into(),
        }
    }

    pub fn platform(message: impl Into<String>) -> Self {
        Self::Platform {
            message: message.into(),
        }
    }

    pub fn display(message: impl Into<String>) -> Self {
        Self::Display {
            message: message.into(),
        }
    }
}
