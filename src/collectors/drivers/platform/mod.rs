#[cfg(target_os = "windows")]
mod windows;

#[cfg(target_os = "macos")]
mod macos;

#[cfg(target_os = "linux")]
mod linux;

use super::DriverData;

pub fn collect_drivers() -> DriverData {
    #[cfg(target_os = "windows")]
    {
        windows::collect()
    }

    #[cfg(target_os = "macos")]
    {
        macos::collect()
    }

    #[cfg(target_os = "linux")]
    {
        linux::collect()
    }

    #[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
    {
        DriverData::default()
    }
}
