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
    fn IOServiceMatching(name: *const i8) -> CfRef;
    fn IOServiceGetMatchingServices(port: u32, matching: CfRef, iterator: *mut u32) -> i32;
    fn IOIteratorNext(iterator: u32) -> u32;
    fn IOObjectRelease(object: u32) -> i32;
    fn IORegistryEntryGetRegistryEntryID(entry: u32, id: *mut u64) -> i32;
    fn IORegistryEntryGetChildIterator(entry: u32, plane: *const i8, iterator: *mut u32) -> i32;
    fn IORegistryEntryCreateCFProperties(
        entry: u32,
        properties: *mut CfRef,
        allocator: CfRef,
        options: u32,
    ) -> i32;
    fn IOPSCopyPowerSourcesInfo() -> CfRef;
    fn IOPSCopyPowerSourcesList(info: CfRef) -> CfRef;
    fn IOPSGetPowerSourceDescription(info: CfRef, source: CfRef) -> CfRef;
}
#[link(name = "CoreFoundation", kind = "framework")]
unsafe extern "C" {
    fn CFRelease(value: CfRef);
    fn CFStringGetCString(value: CfRef, buffer: *mut i8, size: isize, encoding: u32) -> u8;
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

#[link(name = "Metal", kind = "framework")]
unsafe extern "C" {
    fn MTLCopyAllDevices() -> CfRef;
}
#[link(name = "objc")]
unsafe extern "C" {
    fn objc_msgSend();
    fn sel_registerName(name: *const i8) -> *const c_void;
    fn objc_autoreleasePoolPush() -> *mut c_void;
    fn objc_autoreleasePoolPop(pool: *mut c_void);
}
struct Pool(*mut c_void);
impl Drop for Pool {
    fn drop(&mut self) {
        unsafe { objc_autoreleasePoolPop(self.0) }
    }
}

pub fn gpus() -> (Vec<super::gpu::GpuAdapter>, Observation) {
    let _pool = Pool(unsafe { objc_autoreleasePoolPush() });
    let Some(devices) = Owned::new(unsafe { MTLCopyAllDevices() }) else {
        return (
            vec![],
            Observation::unavailable("Metal", "Metal returned no device array"),
        );
    };
    // These Objective-C selectors return scalars/pointers, never structs. Use
    // their precise C ABI signatures on both Intel and Apple Silicon.
    let get_u64: unsafe extern "C" fn(CfRef, *const c_void) -> u64 =
        unsafe { std::mem::transmute(objc_msgSend as unsafe extern "C" fn()) };
    let get_bool: unsafe extern "C" fn(CfRef, *const c_void) -> u8 =
        unsafe { std::mem::transmute(objc_msgSend as unsafe extern "C" fn()) };
    let get_ptr: unsafe extern "C" fn(CfRef, *const c_void) -> CfRef =
        unsafe { std::mem::transmute(objc_msgSend as unsafe extern "C" fn()) };
    let mut rows = Vec::new();
    for index in 0..unsafe { CFArrayGetCount(devices.0) }.clamp(0, 64) {
        let device = unsafe { CFArrayGetValueAtIndex(devices.0, index) };
        if device.is_null() {
            continue;
        }
        let registry = unsafe { get_u64(device, sel_registerName(c"registryID".as_ptr())) };
        let unified =
            unsafe { get_bool(device, sel_registerName(c"hasUnifiedMemory".as_ptr())) } != 0;
        let recommended = unsafe {
            get_u64(
                device,
                sel_registerName(c"recommendedMaxWorkingSetSize".as_ptr()),
            )
        };
        let name = unsafe { get_ptr(device, sel_registerName(c"name".as_ptr())) };
        let mut buffer = [0i8; 512];
        let name = if !name.is_null()
            && unsafe {
                CFStringGetCString(name, buffer.as_mut_ptr(), buffer.len() as isize, 0x08000100)
            } != 0
        {
            unsafe { std::ffi::CStr::from_ptr(buffer.as_ptr()) }
                .to_string_lossy()
                .into_owned()
        } else {
            "Metal graphics device".into()
        };
        rows.push(super::gpu::GpuAdapter {
            device_id: format!("iokit:{registry:016x}"), name, unified_memory: Some(unified),
            recommended_working_set_mb: (recommended > 0).then_some(recommended / 1_048_576),
            // Neither recommendedMaxWorkingSetSize nor currentAllocatedSize
            // is total VRAM/system-wide usage. Preserve absent metrics.
            source: "Metal registry ID and memory architecture; public Metal inventory has no system-wide utilization/temperature".into(), ..Default::default()
        });
    }
    let status = if rows.is_empty() {
        Observation::unavailable("Metal", "No Metal devices available in this session")
    } else {
        Observation::available("MTLCopyAllDevices")
    };
    (rows, status)
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

struct IoObject(u32);
impl Drop for IoObject {
    fn drop(&mut self) {
        if self.0 != 0 {
            unsafe {
                IOObjectRelease(self.0);
            }
        }
    }
}
fn property_list(value: CfRef) -> Option<plist::Value> {
    if value.is_null() {
        return None;
    }
    let data = Owned::new(unsafe {
        CFPropertyListCreateData(ptr::null(), value, 100, 0, ptr::null_mut())
    })?;
    let length = unsafe { CFDataGetLength(data.0) };
    let bytes = unsafe { CFDataGetBytePtr(data.0) };
    if !(1..=1_048_576).contains(&length) || bytes.is_null() {
        return None;
    }
    super::macos::parse_plist(unsafe { std::slice::from_raw_parts(bytes, length as usize) }).ok()
}
fn registry_node(entry: u32, depth: usize, remaining: &mut usize) -> Option<plist::Value> {
    if depth > 8 || *remaining == 0 {
        return None;
    }
    *remaining -= 1;
    let mut properties = ptr::null();
    if unsafe { IORegistryEntryCreateCFProperties(entry, &mut properties, ptr::null(), 0) } != 0 {
        return None;
    }
    let owned = Owned::new(properties)?;
    let mut value = property_list(owned.0)?;
    let row = value.as_dictionary_mut()?;
    let mut id = 0u64;
    if unsafe { IORegistryEntryGetRegistryEntryID(entry, &mut id) } == 0 {
        row.insert("IORegistryEntryID".into(), id.into());
    }
    // The whole medium identifies the physical driver. Do not walk logical
    // partitions/containers below it and count their backing activity twice.
    if row.get("Whole").and_then(plist::Value::as_boolean) == Some(true) {
        return Some(value);
    }
    let mut children = 0;
    if unsafe { IORegistryEntryGetChildIterator(entry, c"IOService".as_ptr(), &mut children) } == 0
        && children != 0
    {
        let iterator = IoObject(children);
        let mut rows = Vec::new();
        while *remaining > 0 {
            let child = IoObject(unsafe { IOIteratorNext(iterator.0) });
            if child.0 == 0 {
                break;
            }
            if let Some(value) = registry_node(child.0, depth + 1, remaining) {
                rows.push(value);
            }
        }
        row.insert("IORegistryEntryChildren".into(), plist::Value::Array(rows));
    }
    Some(value)
}
pub fn storage_registry() -> Result<plist::Value, String> {
    let matching = unsafe { IOServiceMatching(c"IOBlockStorageDriver".as_ptr()) };
    if matching.is_null() {
        return Err("IOKit could not create a storage matching dictionary".into());
    }
    let mut iterator = 0;
    // Matching consumes one CF reference on both success and failure.
    let status = unsafe { IOServiceGetMatchingServices(0, matching, &mut iterator) };
    if status != 0 {
        return Err(format!("IOKit storage enumeration failed ({status})"));
    }
    let iterator = IoObject(iterator);
    let mut nodes = Vec::new();
    let mut remaining = 1024;
    while iterator.0 != 0 && remaining > 0 {
        let entry = IoObject(unsafe { IOIteratorNext(iterator.0) });
        if entry.0 == 0 {
            break;
        }
        if let Some(value) = registry_node(entry.0, 0, &mut remaining) {
            nodes.push(value);
        }
    }
    if remaining == 0 {
        return Err("IOKit storage enumeration exceeded its node budget".into());
    }
    Ok(plist::Value::Array(nodes))
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
