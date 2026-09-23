#[cfg(target_os = "windows")]
mod windows;

#[cfg(any(test, target_os = "macos"))]
#[cfg_attr(not(target_os = "macos"), allow(dead_code))]
mod macos;

#[cfg(any(test, target_os = "linux"))]
#[cfg_attr(not(target_os = "linux"), allow(dead_code))]
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
