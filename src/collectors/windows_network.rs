//! Native byte counters and unicast addresses, joined by locally unique identity.
use super::network::InterfaceInfo;
use crate::observation::Observation;
use std::{
    collections::HashMap,
    net::{Ipv4Addr, Ipv6Addr},
    ptr,
};
use windows_sys::Win32::{
    NetworkManagement::{IpHelper::*, Ndis::*},
    Networking::WinSock::*,
};

const MAX_ROWS: usize = 16_384;
pub(super) const SOURCE: &str = "GetIfTable2 normal interface octets / monotonic elapsed time";
pub(super) const AGGREGATION: &str =
    "Hardware interfaces only; virtual and tunnel rates remain per-interface";
struct Table(*mut std::ffi::c_void);
impl Drop for Table {
    fn drop(&mut self) {
        unsafe {
            FreeMibTable(self.0);
        }
    }
}
fn failure(source: &str, error: u32) -> Observation {
    if error == 5 {
        Observation::permission_denied(source, "Windows denied this read-only interface query")
    } else {
        Observation::error(source, format!("Windows interface query failed ({error})"))
    }
}
fn bounded_rows(count: u32, source: &str) -> Result<usize, Observation> {
    if count as usize > MAX_ROWS {
        Err(Observation::error(
            source,
            "Interface table exceeds the bounded inventory limit",
        ))
    } else {
        Ok(count as usize)
    }
}
fn addresses() -> Result<HashMap<u64, Vec<String>>, Observation> {
    let mut table = ptr::null_mut();
    let status = unsafe { GetUnicastIpAddressTable(AF_UNSPEC, &mut table) };
    if status != 0 {
        return Err(failure("GetUnicastIpAddressTable", status));
    }
    if table.is_null() {
        return Err(Observation::error(
            "GetUnicastIpAddressTable",
            "Windows returned a null table",
        ));
    }
    let _owned = Table(table.cast());
    let rows = unsafe {
        std::slice::from_raw_parts(
            (*table).Table.as_ptr(),
            bounded_rows((*table).NumEntries, "GetUnicastIpAddressTable")?,
        )
    };
    let mut result: HashMap<u64, Vec<String>> = HashMap::new();
    for row in rows {
        if let Some(address) = address(&row.Address) {
            result
                .entry(unsafe { row.InterfaceLuid.Value })
                .or_default()
                .push(address);
        }
    }
    for addresses in result.values_mut() {
        addresses.sort();
        addresses.dedup();
    }
    Ok(result)
}
fn address(address: &SOCKADDR_INET) -> Option<String> {
    // Read only the member selected by the family; IPv4 bytes are network order.
    unsafe {
        match address.si_family {
            AF_INET => {
                Some(Ipv4Addr::from(address.Ipv4.sin_addr.S_un.S_addr.to_ne_bytes()).to_string())
            }
            AF_INET6 => {
                let ip = Ipv6Addr::from(address.Ipv6.sin6_addr.u.Byte);
                let scope = address.Ipv6.Anonymous.sin6_scope_id;
                Some(if scope == 0 {
                    ip.to_string()
                } else {
                    format!("{ip}%{scope}")
                })
            }
            _ => None,
        }
    }
}
pub(super) fn collect() -> Result<Vec<(String, InterfaceInfo)>, Observation> {
    let mut table = ptr::null_mut();
    let status = unsafe { GetIfTable2(&mut table) };
    if status != 0 {
        return Err(failure("GetIfTable2", status));
    }
    if table.is_null() {
        return Err(Observation::error(
            "GetIfTable2",
            "Windows returned a null table",
        ));
    }
    let _owned = Table(table.cast());
    let rows = unsafe {
        std::slice::from_raw_parts(
            (*table).Table.as_ptr(),
            bounded_rows((*table).NumEntries, "GetIfTable2")?,
        )
    };
    let addresses = addresses();
    let observation = match &addresses {
        Ok(_) => Observation::available("GetUnicastIpAddressTable"),
        Err(observation) => observation.clone(),
    };
    Ok(rows
        .iter()
        .filter_map(|row| project(row, addresses.as_ref().ok(), &observation))
        .collect())
}
#[allow(non_upper_case_globals)] // SDK constants retain their documented names.
fn project(
    row: &MIB_IF_ROW2,
    addresses: Option<&HashMap<u64, Vec<String>>>,
    address_status: &Observation,
) -> Option<(String, InterfaceInfo)> {
    let flags = row.InterfaceAndOperStatusFlags._bitfield;
    // Filter-module rows duplicate their backing interface; loopback is local
    // traffic. Keep non-filter virtual/tunnel interfaces individually inspectable.
    if flags & 2 != 0 || row.Type == MIB_IF_TYPE_LOOPBACK {
        return None;
    }
    let guid = &row.InterfaceGuid;
    let luid = unsafe { row.InterfaceLuid.Value };
    let identity = format!(
        "{:08x}-{:04x}-{:04x}-{:02x?}:{luid}",
        guid.data1, guid.data2, guid.data3, guid.data4
    );
    let end = row
        .Alias
        .iter()
        .position(|&c| c == 0)
        .unwrap_or(row.Alias.len());
    let name = String::from_utf16_lossy(&row.Alias[..end]);
    let operational_state = match row.OperStatus {
        IfOperStatusUp => "up",
        IfOperStatusDown => "down",
        IfOperStatusTesting => "testing",
        IfOperStatusDormant => "dormant",
        IfOperStatusNotPresent => "not present",
        IfOperStatusLowerLayerDown => "lower layer down",
        _ => "unknown",
    };
    Some((
        identity,
        InterfaceInfo {
            rate_status: Observation::default(),
            address_status: address_status.clone(),
            included_in_total: flags & 1 != 0 && flags & 128 == 0,
            name,
            ip_addresses: addresses
                .and_then(|a| a.get(&luid))
                .cloned()
                .unwrap_or_default(),
            mac_address: row.PhysicalAddress
                [..(row.PhysicalAddressLength as usize).min(row.PhysicalAddress.len())]
                .iter()
                .map(|byte| format!("{byte:02x}"))
                .collect::<Vec<_>>()
                .join(":"),
            received_bytes: row.InOctets,
            transmitted_bytes: row.OutOctets,
            download_rate: 0,
            upload_rate: 0,
            is_up: row.OperStatus == IfOperStatusUp,
            operational_state: operational_state.into(),
        },
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn native_counter_snapshot_brackets_the_per_interface_api() {
        let before = collect().expect("Native interface inventory");
        let mut table = ptr::null_mut();
        assert_eq!(unsafe { GetIfTable2(&mut table) }, 0);
        assert!(!table.is_null());
        let _owned = Table(table.cast());
        let rows = unsafe {
            std::slice::from_raw_parts(
                (*table).Table.as_ptr(),
                bounded_rows((*table).NumEntries, "fixture").unwrap(),
            )
        };
        let mut direct = Vec::new();
        for row in rows {
            let mut entry = MIB_IF_ROW2 {
                InterfaceLuid: row.InterfaceLuid,
                ..Default::default()
            };
            if unsafe { GetIfEntry2(&mut entry) } == 0 {
                if let Some(info) = project(&entry, None, &Observation::default()) {
                    direct.push(info);
                }
            }
        }
        let after = collect().expect("Native interface inventory");
        let mut compared = 0;
        for (id, value) in direct {
            let Some((_, before)) = before.iter().find(|(key, _)| key == &id) else {
                continue;
            };
            let Some((_, after)) = after.iter().find(|(key, _)| key == &id) else {
                continue;
            };
            if before.received_bytes > after.received_bytes
                || before.transmitted_bytes > after.transmitted_bytes
            {
                continue;
            }
            // Counter calls bracket one another; no rate or percentage tolerance
            // is invented. A reset/hotplug invalidates this comparison.
            assert!((before.received_bytes..=after.received_bytes).contains(&value.received_bytes));
            assert!((before.transmitted_bytes..=after.transmitted_bytes)
                .contains(&value.transmitted_bytes));
            compared += 1;
        }
        assert!(
            compared > 0,
            "No stable non-loopback interfaces could be compared"
        );
    }
    #[test]
    fn identity_does_not_guess_from_names_or_partial_guids() {
        let mut first = MIB_IF_ROW2::default();
        first.InterfaceGuid.data1 = 1;
        first.Alias[0] = b'A' as u16;
        first.InterfaceAndOperStatusFlags._bitfield = 1;
        first.InOctets = 1234;
        let mut second = first;
        second.InterfaceGuid.data1 = 2;
        let obs = Observation::available("fixture");
        let a = project(&first, None, &obs).unwrap();
        let b = project(&second, None, &obs).unwrap();
        assert_ne!(a.0, b.0);
        assert_eq!(a.1.received_bytes, 1234);
        first.Alias[0] = b'B' as u16;
        assert_eq!(a.0, project(&first, None, &obs).unwrap().0);
        first.InterfaceAndOperStatusFlags._bitfield = 3;
        assert!(project(&first, None, &obs).is_none());
        first.InterfaceAndOperStatusFlags._bitfield = 0;
        assert!(!project(&first, None, &obs).unwrap().1.included_in_total);
        assert!(!project(&first, None, &Observation::error("IP", "fixture"))
            .unwrap()
            .1
            .address_status
            .is_available());
        assert!(bounded_rows(MAX_ROWS as u32 + 1, "fixture").is_err());
    }
    #[test]
    fn addresses_follow_native_network_order_and_ipv6_scope() {
        let mut row = SOCKADDR_INET::default();
        row.Ipv4.sin_family = AF_INET;
        row.Ipv4.sin_addr.S_un.S_addr = u32::from_ne_bytes([192, 0, 2, 1]);
        assert_eq!(address(&row).as_deref(), Some("192.0.2.1"));
        row.Ipv6.sin6_family = AF_INET6;
        row.Ipv6.sin6_addr.u.Byte = "fe80::1".parse::<Ipv6Addr>().unwrap().octets();
        row.Ipv6.Anonymous.sin6_scope_id = 7;
        assert_eq!(address(&row).as_deref(), Some("fe80::1%7"));
    }
}
