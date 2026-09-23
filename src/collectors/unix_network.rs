//! Fresh Unix counter reads with explicit errors. Address/link enumeration and
//! byte counters keep separate availability; cached counters never become idle.
#[cfg(unix)]
use super::network::InterfaceInfo;
use crate::observation::Observation;
#[cfg(unix)]
use std::{
    collections::BTreeMap,
    ffi::CStr,
    net::{Ipv4Addr, Ipv6Addr},
    ptr,
};
#[cfg(any(target_os = "linux", test))]
use std::{collections::HashMap, io::Read, path::Path};

#[cfg(target_os = "macos")]
pub const SOURCE: &str = "macOS IFMIB_IFDATA 64-bit octets / monotonic elapsed time";
#[cfg(not(target_os = "macos"))]
pub const SOURCE: &str = "Linux procfs 64-bit interface octets / monotonic elapsed time";
#[cfg(unix)]
pub const AGGREGATION: &str = "Non-loopback interfaces; traffic traversing a bridge, VPN or other layers may appear more than once";
#[cfg(any(target_os = "linux", test))]
const MAX_BYTES: usize = 8 * 1024 * 1024;
const MAX_INTERFACES: usize = 16_384;

fn failure(source: &str, error: std::io::Error) -> Observation {
    match error.kind() {
        std::io::ErrorKind::PermissionDenied => {
            Observation::permission_denied(source, error.to_string())
        }
        std::io::ErrorKind::NotFound => Observation::unavailable(source, error.to_string()),
        _ => Observation::error(source, error.to_string()),
    }
}

#[cfg(unix)]
#[derive(Default)]
struct Link {
    index: u32,
    flags: u32,
    mac: String,
    addresses: Vec<String>,
}

#[cfg(unix)]
fn links() -> Result<BTreeMap<String, Link>, Observation> {
    const SOURCE: &str = "getifaddrs interface identity and addresses";
    let mut head = ptr::null_mut();
    if unsafe { libc::getifaddrs(&mut head) } != 0 {
        return Err(failure(SOURCE, std::io::Error::last_os_error()));
    }
    struct Owned(*mut libc::ifaddrs);
    impl Drop for Owned {
        fn drop(&mut self) {
            unsafe {
                libc::freeifaddrs(self.0);
            }
        }
    }
    let _owned = Owned(head);
    let mut current = head;
    let mut rows = BTreeMap::<String, Link>::new();
    let mut count = 0;
    while !current.is_null() {
        count += 1;
        if count > 65_536 || rows.len() > MAX_INTERFACES {
            return Err(Observation::error(
                SOURCE,
                "Interface/address inventory exceeds its bound",
            ));
        }
        let row = unsafe { &*current };
        current = row.ifa_next;
        if row.ifa_name.is_null() {
            return Err(Observation::error(SOURCE, "Missing interface name"));
        }
        let name = unsafe { CStr::from_ptr(row.ifa_name) }
            .to_str()
            .map_err(|_| Observation::error(SOURCE, "Interface name is not UTF-8"))?;
        let link = rows.entry(name.into()).or_default();
        link.flags = row.ifa_flags;
        if row.ifa_addr.is_null() {
            continue;
        }
        let family = unsafe { (*row.ifa_addr).sa_family } as i32;
        let address = match family {
            libc::AF_INET => {
                let addr = unsafe { ptr::read_unaligned(row.ifa_addr.cast::<libc::sockaddr_in>()) };
                Some(Ipv4Addr::from(addr.sin_addr.s_addr.to_ne_bytes()).to_string())
            }
            libc::AF_INET6 => {
                let addr =
                    unsafe { ptr::read_unaligned(row.ifa_addr.cast::<libc::sockaddr_in6>()) };
                let ip = Ipv6Addr::from(addr.sin6_addr.s6_addr);
                Some(if addr.sin6_scope_id == 0 {
                    ip.to_string()
                } else {
                    format!("{ip}%{}", addr.sin6_scope_id)
                })
            }
            #[cfg(target_os = "linux")]
            libc::AF_PACKET => {
                let addr = unsafe { ptr::read_unaligned(row.ifa_addr.cast::<libc::sockaddr_ll>()) };
                link.index = u32::try_from(addr.sll_ifindex).unwrap_or(0);
                if addr.sll_halen as usize > addr.sll_addr.len() {
                    return Err(Observation::error(SOURCE, "Invalid link address length"));
                }
                link.mac = format_mac(&addr.sll_addr[..addr.sll_halen as usize]);
                None
            }
            #[cfg(target_os = "macos")]
            libc::AF_LINK => {
                let addr = row.ifa_addr.cast::<libc::sockaddr_dl>();
                let offset = std::mem::offset_of!(libc::sockaddr_dl, sdl_data);
                let length = unsafe { ptr::addr_of!((*addr).sdl_len).read() } as usize;
                if length < offset {
                    return Err(Observation::error(SOURCE, "Truncated link address"));
                }
                link.index =
                    u32::from(unsafe { ptr::addr_of!((*addr).sdl_index).read_unaligned() });
                let start = offset + unsafe { ptr::addr_of!((*addr).sdl_nlen).read() } as usize;
                let end = start + unsafe { ptr::addr_of!((*addr).sdl_alen).read() } as usize;
                if end > length {
                    return Err(Observation::error(SOURCE, "Invalid link address bounds"));
                }
                let bytes = unsafe {
                    std::slice::from_raw_parts(row.ifa_addr.cast::<u8>().add(start), end - start)
                };
                link.mac = format_mac(bytes);
                None
            }
            _ => None,
        };
        if let Some(address) = address {
            link.addresses.push(address);
        }
    }
    for (name, link) in &mut rows {
        if link.index == 0 {
            let name = std::ffi::CString::new(name.as_str()).unwrap();
            link.index = unsafe { libc::if_nametoindex(name.as_ptr()) };
        }
        link.addresses.sort();
        link.addresses.dedup();
    }
    Ok(rows)
}

#[cfg(unix)]
fn format_mac(bytes: &[u8]) -> String {
    bytes
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect::<Vec<_>>()
        .join(":")
}

#[cfg(unix)]
pub fn collect() -> Result<Vec<(String, InterfaceInfo)>, Observation> {
    let links = links()?;
    #[cfg(target_os = "linux")]
    let counters =
        read_text(Path::new("/proc/net/dev"), MAX_BYTES).and_then(|text| parse_proc(&text));
    let mut rows = Vec::with_capacity(links.len());
    for (name, link) in links {
        #[cfg(target_os = "linux")]
        let counters = counters.as_ref().map_err(Clone::clone).and_then(|rows| {
            rows.get(&name).copied().ok_or_else(|| {
                Observation::unavailable(SOURCE, "Interface disappeared during counter capture")
            })
        });
        #[cfg(target_os = "macos")]
        let counters = mac_counters(link.index, &name);
        let (received_bytes, transmitted_bytes, counter_status) = match counters {
            Ok((received, transmitted)) if link.index != 0 => {
                (received, transmitted, Observation::available(SOURCE))
            }
            Ok(_) => (
                0,
                0,
                Observation::unavailable(SOURCE, "Interface identity changed during capture"),
            ),
            Err(error) => (0, 0, error),
        };
        let enabled = link.flags & libc::IFF_UP as u32 != 0;
        let is_up = enabled && link.flags & libc::IFF_RUNNING as u32 != 0;
        let mut state = if enabled {
            "administratively up"
        } else {
            "administratively down"
        }
        .to_string();
        #[cfg(target_os = "linux")]
        {
            if let Ok(value) = read_text(
                &Path::new("/sys/class/net").join(&name).join("operstate"),
                64,
            ) {
                state = value.trim().into();
            }
        }
        #[cfg(target_os = "macos")]
        if enabled && link.flags & libc::IFF_RUNNING as u32 != 0 {
            state.push_str("; resources running");
        }
        let identity = format!("{}:{}:{}", link.index, name, link.mac);
        #[cfg(target_os = "linux")]
        let identity = {
            let mut identity = identity;
            use std::os::unix::fs::MetadataExt;
            if let Ok(metadata) = std::fs::metadata(Path::new("/sys/class/net").join(&name)) {
                identity.push_str(&format!(":{}", metadata.ino()));
            }
            identity
        };
        rows.push((
            identity,
            InterfaceInfo {
                name,
                counter_status,
                address_status: Observation::available("getifaddrs"),
                ip_addresses: link.addresses,
                mac_address: link.mac,
                received_bytes,
                transmitted_bytes,
                is_up,
                operational_state: state,
                included_in_total: link.flags & libc::IFF_LOOPBACK as u32 == 0,
                ..Default::default()
            },
        ));
    }
    Ok(rows)
}

#[cfg(any(target_os = "linux", test))]
fn read_text(path: &Path, limit: usize) -> Result<String, Observation> {
    let mut bytes = Vec::new();
    std::fs::File::open(path)
        .map_err(|error| failure(SOURCE, error))?
        .take(limit as u64 + 1)
        .read_to_end(&mut bytes)
        .map_err(|error| failure(SOURCE, error))?;
    if bytes.len() > limit {
        return Err(Observation::error(
            SOURCE,
            "Kernel response exceeds the supported bound",
        ));
    }
    String::from_utf8(bytes).map_err(|_| Observation::error(SOURCE, "Kernel response is not UTF-8"))
}

#[cfg(any(target_os = "linux", test))]
fn parse_proc(text: &str) -> Result<HashMap<String, (u64, u64)>, Observation> {
    let invalid = || Observation::error(SOURCE, "Malformed Linux interface counter table");
    if text.len() > MAX_BYTES {
        return Err(invalid());
    }
    let mut lines = text.lines();
    if !lines
        .next()
        .is_some_and(|s| s.contains("Receive") && s.contains("Transmit"))
        || !lines.next().is_some_and(|s| s.contains("bytes"))
    {
        return Err(invalid());
    }
    let mut rows = HashMap::new();
    for line in lines.filter(|s| !s.trim().is_empty()) {
        let (name, fields) = line.rsplit_once(':').ok_or_else(invalid)?;
        let values = fields
            .split_whitespace()
            .map(str::parse::<u64>)
            .collect::<Result<Vec<_>, _>>()
            .map_err(|_| invalid())?;
        let name = name.trim();
        if values.len() != 16
            || name.is_empty()
            || name.len() > 255
            || rows.len() >= MAX_INTERFACES
            || rows
                .insert(name.to_string(), (values[0], values[8]))
                .is_some()
        {
            return Err(invalid());
        }
    }
    Ok(rows)
}

#[cfg(target_os = "macos")]
fn mac_counters(index: u32, name: &str) -> Result<(u64, u64), Observation> {
    let mut mib = [
        libc::CTL_NET,
        libc::PF_LINK,
        libc::NETLINK_GENERIC,
        libc::IFMIB_IFDATA,
        index as i32,
        libc::IFDATA_GENERAL,
    ];
    let mut data: libc::ifmibdata = unsafe { std::mem::zeroed() };
    let mut length = std::mem::size_of_val(&data);
    if unsafe {
        libc::sysctl(
            mib.as_mut_ptr(),
            mib.len() as u32,
            (&mut data as *mut libc::ifmibdata).cast(),
            &mut length,
            ptr::null_mut(),
            0,
        )
    } != 0
    {
        return Err(failure(SOURCE, std::io::Error::last_os_error()));
    }
    if length != std::mem::size_of_val(&data) {
        return Err(Observation::error(
            SOURCE,
            "Unexpected 64-bit interface counter structure size",
        ));
    }
    let returned = data
        .ifmd_name
        .iter()
        .take_while(|c| **c != 0)
        .map(|c| *c as u8)
        .collect::<Vec<_>>();
    if returned != name.as_bytes() {
        return Err(Observation::unavailable(
            SOURCE,
            "Interface identity changed during counter capture",
        ));
    }
    Ok((data.ifmd_data.ifi_ibytes, data.ifmd_data.ifi_obytes))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn linux_table_keeps_64_bit_bytes_and_rejects_partial_or_duplicate_rows() {
        let header = "Inter-| Receive | Transmit\n face |bytes packets errs drop fifo frame compressed multicast|bytes packets errs drop fifo colls carrier compressed\n";
        let row = " eth0: 5000000000 1 0 0 0 0 0 0 6000000000 1 0 0 0 0 0 0\n";
        assert_eq!(
            parse_proc(&format!("{header}{row}")).unwrap()["eth0"],
            (5_000_000_000, 6_000_000_000)
        );
        assert!(parse_proc(&format!("{header}{row}{row}")).is_err());
        assert!(parse_proc(&format!("{header}eth0: 12 34")).is_err());
        assert!(parse_proc("not a counter table").is_err());
    }
    #[test]
    fn inaccessible_or_excessive_reads_have_explicit_status() {
        use crate::observation::ObservationStatus;
        let directory = tempfile::tempdir().unwrap();
        assert_eq!(
            read_text(&directory.path().join("absent"), 64)
                .unwrap_err()
                .status,
            ObservationStatus::Unavailable
        );
        let path = directory.path().join("large");
        std::fs::write(&path, [b'a'; 65]).unwrap();
        assert_eq!(
            read_text(&path, 64).unwrap_err().status,
            ObservationStatus::Error
        );
        assert_eq!(
            failure(SOURCE, std::io::ErrorKind::PermissionDenied.into()).status,
            ObservationStatus::PermissionDenied
        );
    }
    #[cfg(unix)]
    #[test]
    fn native_interface_inventory_keeps_loopback_and_readable_64_bit_counters() {
        let rows = collect().unwrap();
        assert!(!rows.is_empty());
        let loopback = rows
            .iter()
            .find(|(_, row)| row.ip_addresses.iter().any(|a| a == "127.0.0.1"))
            .expect("native loopback row");
        assert!(
            loopback.1.counter_status.is_available(),
            "{:?}",
            loopback.1.counter_status
        );
        assert!(!loopback.1.included_in_total);
    }
    #[cfg(unix)]
    #[test]
    fn native_loopback_byte_counters_cover_a_known_local_payload() {
        use std::{net::UdpSocket, time::Duration};
        let receiver = UdpSocket::bind("127.0.0.1:0").unwrap();
        receiver
            .set_read_timeout(Some(Duration::from_secs(2)))
            .unwrap();
        let sender = UdpSocket::bind("127.0.0.1:0").unwrap();
        let count = || {
            collect()
                .unwrap()
                .into_iter()
                .find(|(_, r)| r.ip_addresses.iter().any(|a| a == "127.0.0.1"))
                .map(|(id, r)| {
                    assert!(r.counter_status.is_available(), "{:?}", r.counter_status);
                    (id, r.received_bytes, r.transmitted_bytes)
                })
                .unwrap()
        };
        let before = count();
        let payload = [0u8; 1024];
        let mut received = [0u8; 1024];
        for _ in 0..32 {
            sender
                .send_to(&payload, receiver.local_addr().unwrap())
                .unwrap();
            assert_eq!(receiver.recv_from(&mut received).unwrap().0, payload.len());
        }
        let after = count();
        assert_eq!(before.0, after.0);
        // Counters include protocol overhead and unrelated local traffic; the
        // transmitted payload is a lower bound, not an exact wire-byte oracle.
        assert!(after.1 - before.1 >= 32 * 1024);
        assert!(after.2 - before.2 >= 32 * 1024);
    }
}
