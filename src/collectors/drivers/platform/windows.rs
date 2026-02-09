use crate::collectors::drivers::{
    DeviceCategory, DeviceInfo, DeviceStatus, DriverData, DriverScanStatus, ServiceInfo,
};
use windows::Win32::Devices::DeviceAndDriverInstallation::*;
use windows::Win32::System::Registry::*;
use windows::Win32::System::Services::*;
use windows::core::{PCWSTR, PWSTR};

pub fn collect() -> DriverData {
    let mut data = DriverData::default();

    if !enumerate_devices(&mut data) {
        return data;
    }

    query_services(&mut data);
    data
}

/// Enumerate all present PnP devices via Setup API.
/// Returns false if enumeration completely failed (sets ScanFailed).
fn enumerate_devices(data: &mut DriverData) -> bool {
    let dev_info = unsafe {
        SetupDiGetClassDevsW(
            None,
            PCWSTR::null(),
            None,
            DIGCF_ALLCLASSES | DIGCF_PRESENT,
        )
    };

    let dev_info = match dev_info {
        Ok(h) => h,
        Err(_) => {
            data.scan_status = DriverScanStatus::ScanFailed(
                "Device enumeration unavailable. Try running as Administrator.".to_string(),
            );
            return false;
        }
    };

    let mut index: u32 = 0;
    loop {
        let mut dev_info_data = SP_DEVINFO_DATA {
            cbSize: std::mem::size_of::<SP_DEVINFO_DATA>() as u32,
            ..Default::default()
        };

        let ok = unsafe { SetupDiEnumDeviceInfo(dev_info, index, &mut dev_info_data) };
        if ok.is_err() {
            break; // No more devices
        }
        index += 1;

        // Read device class
        let class_str = get_device_string_property(dev_info, &dev_info_data, SPDRP_CLASS);
        let category = match class_str.as_str() {
            "Net" | "NetClient" | "NetService" | "NetTrans" => DeviceCategory::Network,
            "Bluetooth" => DeviceCategory::Bluetooth,
            "AudioEndpoint" | "Media" | "MEDIA" => DeviceCategory::Audio,
            "Keyboard" | "Mouse" | "HIDClass" => DeviceCategory::Input,
            "Display" | "Monitor" => DeviceCategory::Display,
            "DiskDrive" | "SCSIAdapter" | "HDC" | "Volume" | "CDROM" => DeviceCategory::Storage,
            "USB" | "USBDevice" => DeviceCategory::Usb,
            "System" | "Processor" | "Computer" | "Firmware" | "Battery" => DeviceCategory::System,
            "" => continue,
            _ => DeviceCategory::Other,
        };

        // Read device name (friendly name, fallback to description)
        let name = {
            let friendly = get_device_string_property(dev_info, &dev_info_data, SPDRP_FRIENDLYNAME);
            if friendly.is_empty() {
                get_device_string_property(dev_info, &dev_info_data, SPDRP_DEVICEDESC)
            } else {
                friendly
            }
        };
        if name.is_empty() {
            continue;
        }

        // Read device status via Configuration Manager
        let status = get_device_status(&dev_info_data);

        // Read driver version and date from registry
        let (driver_version, driver_date) = get_driver_registry_info(dev_info, &dev_info_data);

        let info = DeviceInfo {
            name,
            driver_version,
            driver_date,
            status,
            category: category.clone(),
            extra: String::new(),
        };

        match category {
            DeviceCategory::Network => data.network.push(info),
            DeviceCategory::Bluetooth => data.bluetooth.push(info),
            DeviceCategory::Audio => data.audio.push(info),
            DeviceCategory::Input => data.input.push(info),
            DeviceCategory::Display => data.display.push(info),
            DeviceCategory::Storage => data.storage.push(info),
            DeviceCategory::Usb => data.usb.push(info),
            DeviceCategory::System => data.system.push(info),
            DeviceCategory::Other => data.other.push(info),
        }
    }

    unsafe {
        let _ = SetupDiDestroyDeviceInfoList(dev_info);
    }

    data.scan_status = DriverScanStatus::Success;
    true
}

/// Read a string property from a device via SetupDiGetDeviceRegistryPropertyW.
fn get_device_string_property(
    dev_info: HDEVINFO,
    dev_info_data: &SP_DEVINFO_DATA,
    property: SETUP_DI_REGISTRY_PROPERTY,
) -> String {
    let mut buf: Vec<u8> = vec![0u8; 512];
    let mut required_size: u32 = 0;
    let mut reg_type: u32 = 0;

    let result = unsafe {
        SetupDiGetDeviceRegistryPropertyW(
            dev_info,
            dev_info_data,
            property,
            Some(&mut reg_type),
            Some(&mut buf),
            Some(&mut required_size),
        )
    };

    if result.is_err() {
        // Try again with larger buffer if needed
        if required_size > buf.len() as u32 {
            buf.resize(required_size as usize, 0);
            let result = unsafe {
                SetupDiGetDeviceRegistryPropertyW(
                    dev_info,
                    dev_info_data,
                    property,
                    Some(&mut reg_type),
                    Some(&mut buf),
                    Some(&mut required_size),
                )
            };
            if result.is_err() {
                return String::new();
            }
        } else {
            return String::new();
        }
    }

    // Convert UTF-16LE bytes to String
    if required_size >= 2 {
        let wide: Vec<u16> = buf[..required_size as usize]
            .chunks_exact(2)
            .map(|chunk| u16::from_le_bytes([chunk[0], chunk[1]]))
            .collect();
        // Trim trailing null
        let len = wide.iter().position(|&c| c == 0).unwrap_or(wide.len());
        String::from_utf16_lossy(&wide[..len])
    } else {
        String::new()
    }
}

/// Get device status via CM_Get_DevNode_Status.
fn get_device_status(dev_info_data: &SP_DEVINFO_DATA) -> DeviceStatus {
    let mut status_flags = CM_DEVNODE_STATUS_FLAGS(0);
    let mut problem_number = CM_PROB(0);

    let cr = unsafe {
        CM_Get_DevNode_Status(&mut status_flags, &mut problem_number, dev_info_data.DevInst, 0)
    };

    if cr != CONFIGRET(0) {
        return DeviceStatus::Unknown;
    }

    let dn_has_problem = CM_DEVNODE_STATUS_FLAGS(0x00000400);
    if (status_flags.0 & dn_has_problem.0) == 0 {
        DeviceStatus::Ok
    } else if problem_number.0 == 22 {
        // CM_PROB_DISABLED = 22
        DeviceStatus::Disabled
    } else {
        DeviceStatus::Error(format!("Problem code {}", problem_number.0))
    }
}

/// Read DriverVersion and DriverDate from the device's driver registry key.
fn get_driver_registry_info(
    dev_info: HDEVINFO,
    dev_info_data: &SP_DEVINFO_DATA,
) -> (String, String) {
    // Get the driver registry key path from SPDRP_DRIVER
    let driver_key = get_device_string_property(dev_info, dev_info_data, SPDRP_DRIVER);
    if driver_key.is_empty() {
        return (String::new(), String::new());
    }

    // Open the registry key: HKLM\SYSTEM\CurrentControlSet\Control\Class\{driver_key}
    let key_path = format!(
        "SYSTEM\\CurrentControlSet\\Control\\Class\\{}",
        driver_key
    );
    let wide_path: Vec<u16> = key_path.encode_utf16().chain(std::iter::once(0)).collect();

    let mut hkey = HKEY::default();
    let result = unsafe {
        RegOpenKeyExW(
            HKEY_LOCAL_MACHINE,
            PCWSTR(wide_path.as_ptr()),
            Some(0),
            KEY_READ,
            &mut hkey,
        )
    };

    if result.is_err() {
        return (String::new(), String::new());
    }

    let version = read_reg_string(hkey, "DriverVersion");
    let date = read_reg_string(hkey, "DriverDate");

    unsafe {
        let _ = RegCloseKey(hkey);
    }

    (version, format_driver_date(&date))
}

/// Read a REG_SZ value from a registry key.
fn read_reg_string(hkey: HKEY, value_name: &str) -> String {
    let wide_name: Vec<u16> = value_name.encode_utf16().chain(std::iter::once(0)).collect();
    let mut buf: Vec<u8> = vec![0u8; 512];
    let mut buf_size = buf.len() as u32;
    let mut reg_type: REG_VALUE_TYPE = REG_VALUE_TYPE(0);

    let result = unsafe {
        RegQueryValueExW(
            hkey,
            PCWSTR(wide_name.as_ptr()),
            None,
            Some(&mut reg_type),
            Some(buf.as_mut_ptr()),
            Some(&mut buf_size),
        )
    };

    if result.is_err() || buf_size < 2 {
        return String::new();
    }

    let wide: Vec<u16> = buf[..buf_size as usize]
        .chunks_exact(2)
        .map(|chunk| u16::from_le_bytes([chunk[0], chunk[1]]))
        .collect();
    let len = wide.iter().position(|&c| c == 0).unwrap_or(wide.len());
    String::from_utf16_lossy(&wide[..len])
}

/// Format driver date from registry format (e.g. "6-21-2006" or "m-d-yyyy") to "YYYY-MM-DD".
fn format_driver_date(date: &str) -> String {
    if date.is_empty() {
        return String::new();
    }
    // Registry format is typically "m-d-yyyy"
    let parts: Vec<&str> = date.split('-').collect();
    if parts.len() == 3 {
        if let (Ok(month), Ok(day), Ok(year)) = (
            parts[0].parse::<u32>(),
            parts[1].parse::<u32>(),
            parts[2].parse::<u32>(),
        ) {
            return format!("{:04}-{:02}-{:02}", year, month, day);
        }
    }
    date.to_string()
}

/// Query critical Windows services via the Service Control Manager.
fn query_services(data: &mut DriverData) {
    let scm = unsafe { OpenSCManagerW(PCWSTR::null(), PCWSTR::null(), SC_MANAGER_CONNECT) };
    let scm = match scm {
        Ok(h) => h,
        Err(_) => return, // Skip services silently
    };

    let service_names = [
        "Dhcp",
        "Dnscache",
        "WlanSvc",
        "NlaSvc",
        "bthserv",
        "BthAvctpSvc",
        "Audiosrv",
        "AudioEndpointBuilder",
        "hidserv",
        "StorSvc",
        "VSS",
        "USBHUB3",
        "DisplayEnhancementService",
    ];

    for svc_name in &service_names {
        let wide_name: Vec<u16> = svc_name.encode_utf16().chain(std::iter::once(0)).collect();

        let svc = unsafe {
            OpenServiceW(scm, PCWSTR(wide_name.as_ptr()), SERVICE_QUERY_STATUS)
        };
        let svc = match svc {
            Ok(h) => h,
            Err(_) => continue,
        };

        let mut status = SERVICE_STATUS::default();
        let query_ok = unsafe { QueryServiceStatus(svc, &mut status) };

        if query_ok.is_ok() {
            let display_name = get_service_display_name(scm, &wide_name);
            data.services.push(ServiceInfo {
                name: svc_name.to_string(),
                display_name,
                is_running: status.dwCurrentState == SERVICE_RUNNING,
            });
        }

        unsafe {
            let _ = CloseServiceHandle(svc);
        }
    }

    unsafe {
        let _ = CloseServiceHandle(scm);
    }
}

/// Get the display name of a service from SCM.
fn get_service_display_name(scm: SC_HANDLE, wide_name: &[u16]) -> String {
    let mut buf: Vec<u16> = vec![0u16; 256];
    let mut buf_size = buf.len() as u32;

    let result = unsafe {
        GetServiceDisplayNameW(
            scm,
            PCWSTR(wide_name.as_ptr()),
            Some(PWSTR(buf.as_mut_ptr())),
            &mut buf_size,
        )
    };

    if result.is_ok() {
        let len = buf.iter().position(|&c| c == 0).unwrap_or(buf_size as usize);
        String::from_utf16_lossy(&buf[..len])
    } else {
        String::new()
    }
}
