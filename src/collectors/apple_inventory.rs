//! Native read-only CoreGraphics/IOKit inventory, run inside the bounded probe
//! process. Every Create/Copy reference is released before returning.
use super::{
    display::{DisplayData, DisplayInfo},
    system_info::SystemInfoData,
    thermals::BatteryInfo,
};
use crate::observation::Observation;
use std::{ffi::c_void, ptr};
type CfRef = *const c_void;
struct Owned(CfRef);
impl Owned {
    fn new(value: CfRef) -> Option<Self> {
        (!value.is_null()).then_some(Self(value))
    }
}
impl Drop for Owned {
    fn drop(&mut self) {
        unsafe { CFRelease(self.0) }
    }
}

#[repr(C)]
struct Size {
    width: f64,
    height: f64,
}
#[link(name = "CoreGraphics", kind = "framework")]
unsafe extern "C" {
    fn CGGetActiveDisplayList(max: u32, displays: *mut u32, count: *mut u32) -> i32;
    fn CGDisplayScreenSize(display: u32) -> Size;
    fn CGDisplayIsBuiltin(display: u32) -> u32;
}
#[link(name = "IOKit", kind = "framework")]
unsafe extern "C" {
    fn IOPSCopyPowerSourcesInfo() -> CfRef;
    fn IOPSCopyPowerSourcesList(info: CfRef) -> CfRef;
    fn IOPSGetPowerSourceDescription(info: CfRef, source: CfRef) -> CfRef;
}
#[link(name = "CoreFoundation", kind = "framework")]
unsafe extern "C" {
    fn CFRelease(value: CfRef);
    fn CFArrayGetCount(array: CfRef) -> isize;
    fn CFArrayGetValueAtIndex(array: CfRef, index: isize) -> CfRef;
    fn CFPropertyListCreateData(
        allocator: CfRef,
        value: CfRef,
        format: isize,
        options: usize,
        error: *mut CfRef,
    ) -> CfRef;
    fn CFDataGetLength(data: CfRef) -> isize;
    fn CFDataGetBytePtr(data: CfRef) -> *const u8;
}

pub fn displays() -> DisplayData {
    let mut count = 0;
    // Cap the allocation and perform the query in one native call so hotplug
    // cannot invalidate a separately queried allocation size.
    let mut ids = [0u32; 64];
    let result = unsafe { CGGetActiveDisplayList(ids.len() as u32, ids.as_mut_ptr(), &mut count) };
    if result != 0 || count as usize > ids.len() {
        return DisplayData {
            inventory_status: Observation::error(
                "CoreGraphics",
                format!("Active display enumeration failed ({result})"),
            ),
            ..Default::default()
        };
    }
    let displays = ids[..count as usize].iter().map(|id| {
        let size = unsafe { CGDisplayScreenSize(*id) };
        let builtin = unsafe { CGDisplayIsBuiltin(*id) } != 0;
        let dimension = |mm: f64| (mm.is_finite() && mm > 0.0 && mm < 655_350.0).then(|| (mm / 10.0).round() as u16);
        DisplayInfo { label: format!("{} display {id}", if builtin { "Built-in" } else { "External" }), active: Some(true),
            connection: if builtin { "Internal" } else { "External (connector type not exposed by CoreGraphics)" }.into(),
            brightness_percent: None, physical_width_cm: dimension(size.width), physical_height_cm: dimension(size.height), source: "CoreGraphics active display inventory; physical dimensions in mm converted to cm".into() }
    }).collect::<Vec<_>>();
    let inventory_status = if displays.is_empty() {
        Observation::unavailable(
            "CoreGraphics",
            "No active displays; a headless session can have none",
        )
    } else {
        Observation::available("CGGetActiveDisplayList")
    };
    DisplayData { displays, inventory_status, brightness_status: Observation::unsupported("CoreGraphics", "Public CoreGraphics display enumeration does not expose brightness; no private control API is used") }
}

fn power_sources() -> Option<Vec<plist::Value>> {
    let info = Owned::new(unsafe { IOPSCopyPowerSourcesInfo() })?;
    let list = Owned::new(unsafe { IOPSCopyPowerSourcesList(info.0) })?;
    let count = unsafe { CFArrayGetCount(list.0) };
    let mut rows = Vec::new();
    for index in 0..count.clamp(0, 64) {
        let source = unsafe { CFArrayGetValueAtIndex(list.0, index) };
        let description = unsafe { IOPSGetPowerSourceDescription(info.0, source) };
        if description.is_null() {
            continue;
        }
        // Serialize only the API-owned property-list dictionary while its
        // parent snapshot is alive. XML format is kCFPropertyListXMLFormat_v1_0.
        let Some(data) = Owned::new(unsafe {
            CFPropertyListCreateData(ptr::null(), description, 100, 0, ptr::null_mut())
        }) else {
            continue;
        };
        let length = unsafe { CFDataGetLength(data.0) };
        let bytes = unsafe { CFDataGetBytePtr(data.0) };
        if !(1..=1_048_576).contains(&length) || bytes.is_null() {
            continue;
        }
        if let Ok(value) =
            super::macos::parse_plist(unsafe { std::slice::from_raw_parts(bytes, length as usize) })
        {
            rows.push(value);
        }
    }
    Some(rows)
}

pub fn battery() -> (Option<BatteryInfo>, Observation) {
    let Some(sources) = power_sources() else {
        return (
            None,
            Observation::error(
                "IOKit power sources",
                "Could not obtain a power-source snapshot",
            ),
        );
    };
    for source in sources {
        if let Some(battery) = super::macos::parse_battery(&source) {
            return (
                Some(battery),
                Observation::available("IOKit IOPowerSources"),
            );
        }
    }
    (None, Observation::unavailable("IOKit power sources", "No present internal battery with readable capacity and state; desktops commonly have none"))
}

pub fn hardware(data: &mut SystemInfoData) {
    match super::macos::command_plist(
        "/usr/sbin/ioreg",
        &["-a", "-r", "-c", "IOPlatformExpertDevice"],
    ) {
        Ok(value) => {
            let row = value
                .as_array()
                .and_then(|rows| rows.first())
                .and_then(plist::Value::as_dictionary);
            let get = |key: &str| {
                row.and_then(|row| row.get(key))
                    .and_then(|v| {
                        v.as_string().map(str::to_string).or_else(|| {
                            v.as_data()
                                .and_then(|bytes| std::str::from_utf8(bytes).ok())
                                .map(|s| s.trim_matches('\0').to_string())
                        })
                    })
                    .filter(|s| !s.is_empty())
            };
            data.manufacturer = get("manufacturer");
            data.model = get("model");
            data.bios_version = get("boot-rom-version");
            data.hardware_status = if data.model.is_some() {
                Observation::available("IOPlatformExpertDevice")
            } else {
                Observation::unavailable(
                    "IOPlatformExpertDevice",
                    "Hardware identity properties were not exposed",
                )
            };
        }
        Err(error) => data.hardware_status = Observation::error("IOPlatformExpertDevice", error),
    }
}
