use super::gpu::GpuAdapter;
use crate::observation::Observation;
use std::collections::{HashMap, HashSet};
use windows::{
    core::Interface,
    Win32::{
        Foundation::LUID,
        Graphics::Dxgi::{
            CreateDXGIFactory1, IDXGIAdapter4, IDXGIFactory1, DXGI_ADAPTER_FLAG_SOFTWARE,
        },
    },
};
use windows_sys::Wdk::Graphics::Direct3D::*;

fn luid_key(high: u32, low: u32) -> String {
    format!("luid:{high:08x}:{low:08x}")
}
struct AdapterHandle(u32);
impl Drop for AdapterHandle {
    fn drop(&mut self) {
        unsafe {
            D3DKMTCloseAdapter(&D3DKMT_CLOSEADAPTER { hAdapter: self.0 });
        }
    }
}

fn pci_address(luid: LUID) -> Option<String> {
    let mut open = D3DKMT_OPENADAPTERFROMLUID {
        AdapterLuid: windows_sys::Win32::Foundation::LUID {
            LowPart: luid.LowPart,
            HighPart: luid.HighPart,
        },
        ..Default::default()
    };
    if unsafe { D3DKMTOpenAdapterFromLuid(&mut open) } < 0 {
        return None;
    }
    let handle = AdapterHandle(open.hAdapter);
    let mut address = D3DKMT_ADAPTERADDRESS::default();
    let mut query = D3DKMT_QUERYADAPTERINFO {
        hAdapter: handle.0,
        Type: KMTQAITYPE_ADAPTERADDRESS,
        pPrivateDriverData: (&mut address as *mut D3DKMT_ADAPTERADDRESS).cast(),
        PrivateDriverDataSize: std::mem::size_of_val(&address) as u32,
    };
    if unsafe { D3DKMTQueryAdapterInfo(&mut query) } < 0
        || address.BusNumber > 255
        || address.DeviceNumber > 31
        || address.FunctionNumber > 7
    {
        return None;
    }
    // D3DKMT exposes bus/device/function; it has no PCI segment field. Only
    // segment-zero NVIDIA records can correlate with this Windows address.
    Some(format!(
        "0000:{:02x}:{:02x}.{}",
        address.BusNumber, address.DeviceNumber, address.FunctionNumber
    ))
}

pub fn inventory() -> Result<Vec<GpuAdapter>, String> {
    let factory: IDXGIFactory1 = unsafe { CreateDXGIFactory1() }.map_err(|e| e.to_string())?;
    let mut rows = Vec::new();
    for index in 0..64 {
        let Ok(adapter) = (unsafe { factory.EnumAdapters1(index) }) else {
            break;
        };
        let desc = unsafe { adapter.GetDesc1() }.map_err(|e| e.to_string())?;
        if desc.Flags & DXGI_ADAPTER_FLAG_SOFTWARE.0 as u32 != 0 {
            continue;
        }
        let desc3 = adapter
            .cast::<IDXGIAdapter4>()
            .ok()
            .and_then(|a| unsafe { a.GetDesc3() }.ok());
        let end = desc
            .Description
            .iter()
            .position(|c| *c == 0)
            .unwrap_or(desc.Description.len());
        rows.push(GpuAdapter {
            device_id: luid_key(desc.AdapterLuid.HighPart as u32, desc.AdapterLuid.LowPart),
            pci_address: pci_address(desc.AdapterLuid),
            name: String::from_utf16_lossy(&desc.Description[..end]),
            dedicated_memory_mb: Some(
                desc3
                    .as_ref()
                    .map_or(desc.DedicatedVideoMemory, |d| d.DedicatedVideoMemory)
                    as u64
                    / 1_048_576,
            ),
            dedicated_system_memory_mb: Some(
                desc3
                    .as_ref()
                    .map_or(desc.DedicatedSystemMemory, |d| d.DedicatedSystemMemory)
                    as u64
                    / 1_048_576,
            ),
            shared_memory_mb: Some(
                desc3
                    .as_ref()
                    .map_or(desc.SharedSystemMemory, |d| d.SharedSystemMemory)
                    as u64
                    / 1_048_576,
            ),
            source: "DXGI; dedicated video/system memory and shared-system limit are distinct"
                .into(),
            ..Default::default()
        });
    }
    Ok(rows)
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "PascalCase")]
struct EngineRow {
    name: String,
    utilization_percentage: u64,
}

fn aggregate_engines(rows: &[EngineRow]) -> HashMap<String, f32> {
    let mut engines = HashMap::<(String, String, String), u64>::new();
    let mut seen = HashSet::new();
    for row in rows {
        if row.utilization_percentage > 100 || !seen.insert(&row.name) {
            continue;
        }
        let parts = row.name.split('_').collect::<Vec<_>>();
        let find = |key| parts.iter().position(|part| *part == key);
        let (Some(luid), Some(physical), Some(engine)) = (find("luid"), find("phys"), find("eng"))
        else {
            continue;
        };
        let hex = |index: usize| {
            parts
                .get(index)
                .and_then(|v| v.strip_prefix("0x"))
                .and_then(|v| u32::from_str_radix(v, 16).ok())
        };
        let (Some(high), Some(low), Some(physical), Some(engine)) = (
            hex(luid + 1),
            hex(luid + 2),
            parts.get(physical + 1),
            parts.get(engine + 1),
        ) else {
            continue;
        };
        let value = engines
            .entry((
                luid_key(high, low),
                physical.to_string(),
                engine.to_string(),
            ))
            .or_default();
        *value = value.saturating_add(row.utilization_percentage);
    }
    let mut result = HashMap::<String, f32>::new();
    for ((luid, _, _), value) in engines {
        let current = result.entry(luid).or_default();
        *current = current.max(value.min(100) as f32);
    }
    result
}

pub fn pnp_pci_address(id: &str) -> Option<String> {
    use windows::{core::PCWSTR, Win32::Devices::DeviceAndDriverInstallation::*};
    if !id.to_ascii_uppercase().starts_with("PCI\\") {
        return None;
    }
    struct Set(HDEVINFO);
    impl Drop for Set {
        fn drop(&mut self) {
            let _ = unsafe { SetupDiDestroyDeviceInfoList(self.0) };
        }
    }
    let set = Set(unsafe { SetupDiCreateDeviceInfoList(None, None) }.ok()?);
    let mut device = SP_DEVINFO_DATA {
        cbSize: std::mem::size_of::<SP_DEVINFO_DATA>() as u32,
        ..Default::default()
    };
    let id = id.encode_utf16().chain(Some(0)).collect::<Vec<_>>();
    unsafe { SetupDiOpenDeviceInfoW(set.0, PCWSTR(id.as_ptr()), None, 0, Some(&mut device)) }
        .ok()?;
    let read = |property| {
        let mut bytes = [0u8; 4];
        let mut size = 0;
        unsafe {
            SetupDiGetDeviceRegistryPropertyW(
                set.0,
                &device,
                property,
                None,
                Some(&mut bytes),
                Some(&mut size),
            )
        }
        .ok()?;
        (size == 4).then(|| u32::from_le_bytes(bytes))
    };
    let bus = read(SPDRP_BUSNUMBER)?;
    let address = read(SPDRP_ADDRESS)?;
    let device = address >> 16;
    let function = address & 0xffff;
    if bus > 255 || device > 31 || function > 7 {
        return None;
    }
    Some(format!("0000:{bus:02x}:{device:02x}.{function}"))
}

pub fn add_engine_utilization(adapters: &mut [GpuAdapter]) {
    let result = wmi::COMLibrary::new().and_then(wmi::WMIConnection::new).and_then(|c| c.raw_query::<EngineRow>("SELECT Name, UtilizationPercentage FROM Win32_PerfFormattedData_GPUPerformanceCounters_GPUEngine"));
    match result {
        Ok(rows) => {
            let values = aggregate_engines(&rows);
            for adapter in adapters {
                if let Some(value) = values.get(&adapter.device_id) {
                    adapter.utilization_percent = Some(*value);
                    adapter.fields.insert("utilization_percent".into(), Observation::available("WDDM GPU engine counters: sum processes per physical engine, then busiest engine; rounded percent capped at 100"));
                }
            }
        }
        Err(error) => {
            for adapter in adapters {
                if adapter.utilization_percent.is_none() {
                    adapter.fields.insert(
                        "utilization_percent".into(),
                        Observation::unavailable(
                            "WDDM GPU engine counters",
                            format!("Driver/provider did not expose utilization: {error}"),
                        ),
                    );
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn utilization_sums_processes_but_not_independent_engines_or_adapters() {
        let row = |pid, luid, engine, value| EngineRow {
            name: format!("pid_{pid}_luid_0x00000000_0x{luid:08x}_phys_0_eng_{engine}_engtype_3D"),
            utilization_percentage: value,
        };
        let values = aggregate_engines(&[
            row(1, 10, 0, 30),
            row(2, 10, 0, 40),
            row(3, 10, 1, 60),
            row(1, 20, 0, 12),
        ]);
        assert_eq!(values[&luid_key(0, 10)], 70.0);
        assert_eq!(values[&luid_key(0, 20)], 12.0);
    }
}
