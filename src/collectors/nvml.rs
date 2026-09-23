//! Optional, read-only NVML bindings. Loaded only inside the isolated slow worker;
//! nvidia-smi remains the fallback when this driver interface cannot initialize.
use super::gpu::{normalize_pci, GpuAdapter};
use crate::observation::Observation;
use std::{
    cell::RefCell,
    ffi::{c_char, c_void, CStr},
    time::{Duration, Instant},
};

type Device = *mut c_void;
type Status = i32;
type Init = unsafe extern "C" fn() -> Status;
type Count = unsafe extern "C" fn(*mut u32) -> Status;
type ByIndex = unsafe extern "C" fn(u32, *mut Device) -> Status;
type DeviceText = unsafe extern "C" fn(Device, *mut c_char, u32) -> Status;
type Driver = unsafe extern "C" fn(*mut c_char, u32) -> Status;
type Pci = unsafe extern "C" fn(Device, *mut PciInfo) -> Status;
type Memory = unsafe extern "C" fn(Device, *mut MemoryInfo) -> Status;
type Util = unsafe extern "C" fn(Device, *mut Utilization) -> Status;
type Temp = unsafe extern "C" fn(Device, u32, *mut u32) -> Status;

#[repr(C)]
#[derive(Default)]
struct PciInfo {
    legacy_bus_id: [c_char; 16],
    domain: u32,
    bus: u32,
    device: u32,
    device_id: u32,
    subsystem_id: u32,
    bus_id: [c_char; 32],
}
#[repr(C)]
#[derive(Default)]
struct MemoryInfo {
    version: u32,
    total: u64,
    reserved: u64,
    free: u64,
    used: u64,
}
#[repr(C)]
#[derive(Default)]
struct Utilization {
    gpu: u32,
    memory: u32,
}
const _: () = assert!(std::mem::size_of::<PciInfo>() == 68 && std::mem::align_of::<PciInfo>() == 4);
const _: () =
    assert!(std::mem::size_of::<MemoryInfo>() == 40 && std::mem::align_of::<MemoryInfo>() == 8);
const _: () = assert!(std::mem::size_of::<Utilization>() == 8);

struct Library(*mut c_void);
impl Library {
    fn open() -> Option<Self> {
        #[cfg(windows)]
        let handle = unsafe {
            use windows_sys::Win32::System::LibraryLoader::*;
            let name: Vec<u16> = "nvml.dll\0".encode_utf16().collect();
            // Never search the working directory or an application-controlled path.
            LoadLibraryExW(
                name.as_ptr(),
                std::ptr::null_mut(),
                LOAD_LIBRARY_SEARCH_SYSTEM32,
            )
        };
        #[cfg(target_os = "linux")]
        let handle = unsafe {
            libc::dlopen(
                c"libnvidia-ml.so.1".as_ptr(),
                libc::RTLD_LOCAL | libc::RTLD_NOW,
            )
        };
        (!handle.is_null()).then(|| Self(handle))
    }
    /// T must be the documented C function signature for this exact NVML symbol.
    unsafe fn symbol<T: Copy>(&self, name: &CStr) -> Option<T> {
        #[cfg(windows)]
        let pointer = unsafe {
            windows_sys::Win32::System::LibraryLoader::GetProcAddress(self.0, name.as_ptr().cast())
        }
        .map(|function| function as *mut c_void)?;
        #[cfg(target_os = "linux")]
        let pointer = unsafe { libc::dlsym(self.0, name.as_ptr()) };
        if pointer.is_null() || std::mem::size_of::<T>() != std::mem::size_of_val(&pointer) {
            return None;
        }
        Some(unsafe { std::mem::transmute_copy(&pointer) })
    }
}
impl Drop for Library {
    fn drop(&mut self) {
        #[cfg(windows)]
        unsafe {
            windows_sys::Win32::Foundation::FreeLibrary(self.0);
        }
        #[cfg(target_os = "linux")]
        unsafe {
            libc::dlclose(self.0);
        }
    }
}
struct Api {
    count: Count,
    by_index: ByIndex,
    uuid: DeviceText,
    name: Option<DeviceText>,
    pci: Pci,
    memory: Option<Memory>,
    utilization: Option<Util>,
    temperature: Option<Temp>,
    driver: Option<Driver>,
}
struct Nvml {
    api: Api,
    shutdown: Init,
    _library: Library,
}
impl Nvml {
    fn open() -> Option<Self> {
        let library = Library::open()?;
        unsafe {
            let init: Init = library.symbol(c"nvmlInit_v2")?;
            // Resolve required symbols before init so every successful init has
            // exactly one matching shutdown, including early setup failure.
            let shutdown = library.symbol(c"nvmlShutdown")?;
            let count = library.symbol(c"nvmlDeviceGetCount_v2")?;
            let by_index = library.symbol(c"nvmlDeviceGetHandleByIndex_v2")?;
            let uuid = library.symbol(c"nvmlDeviceGetUUID")?;
            let pci = library.symbol(c"nvmlDeviceGetPciInfo_v3")?;
            // v1 includes reserved memory in used; require v2 to preserve the
            // allocated-memory meaning of nvidia-smi. Older drivers use that fallback.
            let memory = Some(library.symbol(c"nvmlDeviceGetMemoryInfo_v2")?);
            if init() != 0 {
                return None;
            }
            Some(Self {
                shutdown,
                api: Api {
                    count,
                    by_index,
                    uuid,
                    pci,
                    name: library.symbol(c"nvmlDeviceGetName"),
                    memory,
                    utilization: library.symbol(c"nvmlDeviceGetUtilizationRates"),
                    temperature: library.symbol(c"nvmlDeviceGetTemperature"),
                    driver: library.symbol(c"nvmlSystemGetDriverVersion"),
                },
                _library: library,
            })
        }
    }
}
impl Api {
    fn collect(&self) -> Option<Vec<GpuAdapter>> {
        let mut count = 0;
        if unsafe { (self.count)(&mut count) } != 0 || count > 256 {
            return None;
        }
        let driver = self.driver.and_then(|f| text(|p, n| unsafe { f(p, n) }));
        let mut adapters = Vec::with_capacity(count as usize);
        for index in 0..count {
            let mut device = std::ptr::null_mut();
            if unsafe { (self.by_index)(index, &mut device) } != 0 || device.is_null() {
                return None;
            }
            let uuid = text(|p, n| unsafe { (self.uuid)(device, p, n) })?;
            let mut pci = PciInfo::default();
            let address = (unsafe { (self.pci)(device, &mut pci) } == 0)
                .then(|| bounded_text(&pci.bus_id))
                .flatten()
                .and_then(|id| normalize_pci(&id));
            let name = self
                .name
                .and_then(|f| text(|p, n| unsafe { f(device, p, n) }))
                .unwrap_or_else(|| "NVIDIA GPU".into());
            let mut memory = MemoryInfo {
                version: (2 << 24) | std::mem::size_of::<MemoryInfo>() as u32,
                ..Default::default()
            };
            let memory_status = self.memory.map_or(3, |f| unsafe { f(device, &mut memory) });
            let mut utilization = Utilization::default();
            let util_status = self
                .utilization
                .map_or(3, |f| unsafe { f(device, &mut utilization) });
            let mut temperature = 0;
            let temp_status = self
                .temperature
                .map_or(3, |f| unsafe { f(device, 0, &mut temperature) });
            let memory_valid = memory_status == 0
                && memory.total > 0
                && memory.total != u64::MAX
                && memory.used != u64::MAX
                && memory.used <= memory.total;
            let util_valid = util_status == 0 && utilization.gpu <= 100;
            let temp_valid = temp_status == 0 && temperature <= 200;
            let mut row = GpuAdapter {
                device_id: format!("nvidia:{uuid}"),
                pci_address: address,
                name,
                driver_version: driver.clone(),
                dedicated_memory_mb: memory_valid.then_some(memory.total / 1_048_576),
                memory_used_mb: memory_valid.then_some(memory.used / 1_048_576),
                utilization_percent: util_valid.then_some(utilization.gpu as f32),
                temperature_celsius: temp_valid.then_some(temperature as f64),
                source: "NVIDIA NVML (read-only driver API)".into(),
                ..Default::default()
            };
            for (field, status, valid, semantics) in [
                (
                    "utilization_percent",
                    util_status,
                    util_valid,
                    "NVIDIA driver sampling window; percentage of time executing GPU work",
                ),
                (
                    "memory_used_mb",
                    memory_status,
                    memory_valid,
                    "NVML v2 allocated device-memory bytes, excluding reserved memory, converted to MiB",
                ),
                (
                    "dedicated_memory_mb",
                    memory_status,
                    memory_valid,
                    "NVML usable physical device memory in MiB",
                ),
                (
                    "temperature_celsius",
                    temp_status,
                    temp_valid,
                    "NVIDIA GPU temperature in degrees Celsius",
                ),
            ] {
                row.fields
                    .insert(field.into(), field_observation(status, valid, semantics));
            }
            row.telemetry_available = memory_valid || util_valid || temp_valid;
            adapters.push(row);
        }
        Some(adapters)
    }
}
impl Drop for Nvml {
    fn drop(&mut self) {
        unsafe {
            (self.shutdown)();
        }
    }
}
fn bounded_text(bytes: &[c_char]) -> Option<String> {
    let length = bytes.iter().position(|b| *b == 0)?;
    let bytes: Vec<u8> = bytes[..length].iter().map(|b| *b as u8).collect();
    let text = std::str::from_utf8(&bytes).ok()?.trim();
    (!text.is_empty() && !text.chars().any(char::is_control)).then(|| text.to_owned())
}
fn text(call: impl FnOnce(*mut c_char, u32) -> Status) -> Option<String> {
    let mut value = [0 as c_char; 256];
    (call(value.as_mut_ptr(), value.len() as u32) == 0)
        .then(|| bounded_text(&value))
        .flatten()
}
fn field_observation(status: Status, valid: bool, semantics: &str) -> Observation {
    match status {
        0 if valid => Observation::available(format!("NVML: {semantics}")),
        0 => Observation::contradictory(
            "NVML",
            "Driver returned an out-of-range or unavailable sentinel value",
        ),
        3 => Observation::unsupported(
            "NVML",
            "This driver/device does not support this read-only query",
        ),
        4 => Observation::permission_denied("NVML", "The driver denied this telemetry query"),
        code => Observation::error(
            "NVML",
            format!("Driver query failed with NVML status {code}"),
        ),
    }
}
#[derive(Default)]
struct Cache {
    api: Option<Nvml>,
    retry: Option<Instant>,
    failures: u32,
    generation: u64,
}
thread_local! {static CACHE:RefCell<Cache>=RefCell::new(Cache::default());}
pub(super) fn collect() -> Option<Vec<GpuAdapter>> {
    CACHE.with(|cache| {
        let mut cache = cache.borrow_mut();
        let generation = super::provider_cache::generation();
        if cache.generation != generation {
            *cache = Cache {
                generation,
                ..Default::default()
            };
        }
        let now = Instant::now();
        if cache.retry.is_some_and(|at| now < at) {
            return None;
        }
        if cache.api.is_none() {
            cache.api = Nvml::open();
        }
        if let Some(rows) = cache.api.as_ref().and_then(|api| api.api.collect()) {
            cache.failures = 0;
            cache.retry = None;
            return Some(rows);
        }
        cache.api = None;
        cache.failures = cache.failures.saturating_add(1);
        cache.retry =
            Some(now + Duration::from_secs((5 * 2u64.pow(cache.failures.min(6))).min(300)));
        None
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    unsafe extern "C" fn count(out: *mut u32) -> Status {
        unsafe {
            *out = 2;
        }
        0
    }
    unsafe extern "C" fn by_index(index: u32, out: *mut Device) -> Status {
        unsafe {
            *out = (index + 1) as Device;
        }
        0
    }
    unsafe extern "C" fn uuid(device: Device, out: *mut c_char, length: u32) -> Status {
        let value = if device as usize == 1 {
            b"GPU-one\0"
        } else {
            b"GPU-two\0"
        };
        assert!(length as usize >= value.len());
        unsafe {
            std::ptr::copy_nonoverlapping(value.as_ptr().cast(), out, value.len());
        }
        0
    }
    unsafe extern "C" fn pci(device: Device, out: *mut PciInfo) -> Status {
        let value = if device as usize == 1 {
            b"00000000:01:00.0\0"
        } else {
            b"00000000:02:00.0\0"
        };
        unsafe {
            std::ptr::copy_nonoverlapping(
                value.as_ptr().cast(),
                (*out).bus_id.as_mut_ptr(),
                value.len(),
            );
        }
        0
    }
    unsafe extern "C" fn memory(device: Device, out: *mut MemoryInfo) -> Status {
        if device as usize == 1 {
            return 4;
        }
        unsafe {
            assert_eq!((*out).version, (2 << 24) | 40);
            (*out).total = 8 * 1024 * 1_048_576;
            (*out).used = 100 * 1_048_576;
            (*out).reserved = 200 * 1_048_576;
            (*out).free = (*out).total - (*out).used - (*out).reserved;
        }
        0
    }
    unsafe extern "C" fn utilization(device: Device, out: *mut Utilization) -> Status {
        if device as usize == 1 {
            return 3;
        }
        unsafe {
            (*out).gpu = 0;
        }
        0
    }
    unsafe extern "C" fn temperature(device: Device, _sensor: u32, out: *mut u32) -> Status {
        if device as usize == 2 {
            return 3;
        }
        unsafe {
            *out = 55;
        }
        0
    }
    #[test]
    fn independent_device_identity_partial_fields_and_allocated_memory_survive_driver_queries() {
        let api = Api {
            count,
            by_index,
            uuid,
            name: None,
            pci,
            memory: Some(memory),
            utilization: Some(utilization),
            temperature: Some(temperature),
            driver: None,
        };
        let rows = api.collect().unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].name, rows[1].name);
        assert_ne!(rows[0].device_id, rows[1].device_id);
        assert_eq!(rows[0].pci_address.as_deref(), Some("0000:01:00.0"));
        assert_eq!(rows[0].temperature_celsius, Some(55.0));
        assert_eq!(rows[0].utilization_percent, None);
        assert_eq!(rows[0].memory_used_mb, None);
        assert_eq!(
            rows[0].fields["memory_used_mb"].status,
            crate::observation::ObservationStatus::PermissionDenied
        );
        assert_eq!(rows[1].utilization_percent, Some(0.0));
        assert_eq!(rows[1].temperature_celsius, None);
        assert_eq!(rows[1].memory_used_mb, Some(100));
        assert_eq!(rows[1].dedicated_memory_mb, Some(8192));
    }
    #[test]
    fn driver_fields_preserve_failure_categories_and_strings_are_bounded() {
        use crate::observation::ObservationStatus::*;
        assert_eq!(field_observation(3, false, "").status, Unsupported);
        assert_eq!(field_observation(4, false, "").status, PermissionDenied);
        assert_eq!(field_observation(15, false, "").status, Error);
        assert_eq!(field_observation(0, false, "").status, Contradictory);
        assert!(field_observation(0, true, "fixture").is_available());
        assert!(bounded_text(&[b'x' as c_char; 16]).is_none());
        assert_eq!(bounded_text(&[b'x' as c_char, 0]), Some("x".into()));
        assert!(text(|_, _| 3).is_none());
    }
}
