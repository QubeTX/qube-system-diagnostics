//! IPv4 probes stay inside the bounded diagnostics worker. Native IP status and
//! millisecond RTT are distinct from process launch time and internet reachability.
use super::{command::CommandError, network_diag::ConnectivityResult};
use std::{net::Ipv4Addr, ptr};
use windows_sys::Win32::{
    Foundation::{GetLastError, HANDLE, INVALID_HANDLE_VALUE},
    NetworkManagement::IpHelper::*,
};

const SOURCE: &str = "Windows IcmpSendEcho; RTT resolution 1 ms";
const PAYLOAD: &[u8] = b"SD300 diagnostic";

struct Icmp(HANDLE);
impl Drop for Icmp {
    fn drop(&mut self) {
        unsafe {
            IcmpCloseHandle(self.0);
        }
    }
}

pub(super) fn gateway() -> Result<Option<String>, CommandError> {
    let mut route = MIB_IPFORWARDROW::default();
    let status = unsafe { GetBestRoute(u32::from_ne_bytes([1, 1, 1, 1]), 0, &mut route) };
    if status != 0 {
        return Err(CommandError::Io(std::io::Error::from_raw_os_error(
            status as i32,
        )));
    }
    // A directly connected route has no router to probe. Never ping 0.0.0.0.
    Ok((route.dwForwardNextHop != 0)
        .then(|| Ipv4Addr::from(route.dwForwardNextHop.to_ne_bytes()).to_string()))
}

pub(super) fn ping(host: &str) -> ConnectivityResult {
    let target = match host.parse::<Ipv4Addr>() {
        Ok(ip) => u32::from_ne_bytes(ip.octets()),
        Err(_) => {
            return failure(
                host,
                "This lightweight ICMP provider requires an IPv4 address".into(),
            )
        }
    };
    let handle = unsafe { IcmpCreateFile() };
    if handle == INVALID_HANDLE_VALUE || handle.is_null() {
        return failure(
            host,
            format!("IcmpCreateFile failed ({})", unsafe { GetLastError() }),
        );
    }
    let owned = Icmp(handle);
    // Both documented native/32-bit reply layouts have the same first three
    // u32 fields. We never read returned data/option pointers. This aligned
    // buffer exceeds either reply header plus payload and 8 error-message bytes.
    let mut reply = [0u64; 32];
    let count = unsafe {
        IcmpSendEcho(
            owned.0,
            target,
            PAYLOAD.as_ptr().cast(),
            PAYLOAD.len() as u16,
            ptr::null(),
            reply.as_mut_ptr().cast(),
            std::mem::size_of_val(&reply) as u32,
            3000,
        )
    };
    if count == 0 {
        let status = unsafe { GetLastError() };
        return failure(host, format!("ICMP returned no successful reply ({status}); filtering or target failure is possible"));
    }
    let words = unsafe { std::slice::from_raw_parts(reply.as_ptr().cast::<u32>(), 3) };
    project_reply(host, target, words[0], words[1], words[2])
}

fn failure(host: &str, error: String) -> ConnectivityResult {
    ConnectivityResult {
        target: host.into(),
        error: Some(error),
        source: SOURCE.into(),
        latency_resolution_ms: Some(1.0),
        ..Default::default()
    }
}

fn project_reply(
    host: &str,
    target: u32,
    address: u32,
    status: u32,
    rtt: u32,
) -> ConnectivityResult {
    if status != IP_SUCCESS {
        return failure(
            host,
            format!("ICMP reply status {status}; this is not a successful echo reply"),
        );
    }
    if address != target {
        return failure(
            host,
            "ICMP reply address differs from the requested target".into(),
        );
    }
    ConnectivityResult {
        reachable: true,
        latency_ms: (rtt > 0).then_some(rtt as f64),
        target: host.into(),
        source: SOURCE.into(),
        latency_resolution_ms: Some(1.0),
        error: (rtt == 0).then(|| {
            "ICMP replied below the provider's 1 ms resolution; exact RTT is unavailable".into()
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn reply_status_address_and_resolution_are_separate() {
        let target = u32::from_ne_bytes([192, 0, 2, 9]);
        let good = project_reply("192.0.2.9", target, target, IP_SUCCESS, 27);
        assert_eq!(good.latency_ms, Some(27.0));
        assert!(good.reachable);
        let fast = project_reply("192.0.2.9", target, target, IP_SUCCESS, 0);
        assert!(fast.reachable);
        assert_eq!(fast.latency_ms, None);
        for status in [
            IP_REQ_TIMED_OUT,
            IP_DEST_HOST_UNREACHABLE,
            IP_TTL_EXPIRED_TRANSIT,
        ] {
            let result = project_reply("192.0.2.9", target, target, status, 4);
            assert!(!result.reachable);
            assert_eq!(result.latency_ms, None);
        }
        assert!(!project_reply("192.0.2.9", target, target + 1, IP_SUCCESS, 4).reachable);
    }
    #[test]
    fn native_loopback_echo_uses_ip_status_without_a_helper_process() {
        let before = std::time::Instant::now();
        let result = ping("127.0.0.1");
        assert!(
            result.reachable,
            "Native loopback probe: {:?}",
            result.error
        );
        assert_eq!(result.latency_resolution_ms, Some(1.0));
        assert!(before.elapsed() < std::time::Duration::from_secs(4));
        assert!(std::mem::size_of::<ICMP_ECHO_REPLY>() + PAYLOAD.len() + 8 < 256);
        assert!(std::mem::size_of::<ICMP_ECHO_REPLY32>() + PAYLOAD.len() + 8 < 256);
    }
}
