//! Documented IP Helper endpoint tables. Native calls remain in the owned worker.
use super::network_diag::{ConnectionInfo, ConnectionState, Protocol};
use crate::observation::Observation;
use std::{
    ffi::c_void,
    mem::{offset_of, size_of},
    net::{Ipv4Addr, Ipv6Addr},
};
use windows_sys::Win32::NetworkManagement::IpHelper::*;

const SOURCE: &str = "Windows IP Helper IPv4/IPv6 TCP/UDP owner-PID tables";

fn table<R: Copy>(
    offset: usize,
    mut query: impl FnMut(*mut c_void, &mut u32) -> u32,
) -> std::io::Result<Vec<R>> {
    let mut length = 0;
    let mut buffer = Vec::<u64>::new();
    for _ in 0..4 {
        let code = query(
            if buffer.is_empty() {
                std::ptr::null_mut()
            } else {
                buffer.as_mut_ptr().cast()
            },
            &mut length,
        );
        match code {
            0 => {
                let length = (length as usize).min(buffer.len() * size_of::<u64>());
                if length < size_of::<u32>() || length < offset {
                    return Err(std::io::Error::other("Truncated endpoint table"));
                }
                let bytes = buffer.as_ptr().cast::<u8>();
                let count = unsafe { bytes.cast::<u32>().read_unaligned() } as usize;
                if count > (length - offset) / size_of::<R>() {
                    return Err(std::io::Error::other("Invalid endpoint table count"));
                }
                // Only the four POD SDK row types below instantiate R. Reading
                // unaligned accounts for table padding without assuming layout.
                return Ok((0..count)
                    .map(|i| unsafe {
                        bytes
                            .add(offset + i * size_of::<R>())
                            .cast::<R>()
                            .read_unaligned()
                    })
                    .collect());
            }
            122 if (4..=super::command::MAX_OUTPUT_BYTES as u32).contains(&length) => {
                buffer.resize((length as usize).div_ceil(size_of::<u64>()), 0);
            }
            _ => return Err(std::io::Error::from_raw_os_error(code as i32)),
        }
    }
    Err(std::io::Error::other(
        "Endpoint table changed repeatedly while collecting",
    ))
}
fn port(raw: u32) -> u16 {
    u16::from_be(raw as u16)
}
fn ipv4(raw: u32) -> String {
    Ipv4Addr::from(raw.to_ne_bytes()).to_string()
}
fn ipv6(raw: [u8; 16], scope: u32) -> String {
    let address = Ipv6Addr::from(raw);
    if scope == 0 {
        address.to_string()
    } else {
        format!("{address}%{}", u32::from_be(scope))
    }
}
fn state(raw: u32) -> ConnectionState {
    match raw {
        2 => ConnectionState::Listening,
        3 => ConnectionState::SynSent,
        4 => ConnectionState::SynReceived,
        5 => ConnectionState::Established,
        6 => ConnectionState::FinWait1,
        7 => ConnectionState::FinWait2,
        8 => ConnectionState::CloseWait,
        9 => ConnectionState::Closing,
        10 => ConnectionState::LastAck,
        11 => ConnectionState::TimeWait,
        1 => ConnectionState::Unknown("CLOSED".into()),
        12 => ConnectionState::Unknown("DELETE_TCB".into()),
        other => ConnectionState::Unknown(format!("OS state {other}")),
    }
}
fn tcp(
    local: String,
    local_port: u32,
    remote: String,
    remote_port: u32,
    code: u32,
    pid: u32,
) -> ConnectionInfo {
    let state = state(code);
    let listening = state == ConnectionState::Listening;
    ConnectionInfo {
        protocol: Protocol::Tcp,
        local_addr: local,
        local_port: port(local_port),
        remote_addr: if listening { "*".into() } else { remote },
        remote_port: if listening { 0 } else { port(remote_port) },
        state,
        pid: (pid != 0).then_some(pid),
        process_name: None,
    }
}
fn udp(local: String, local_port: u32, pid: u32) -> ConnectionInfo {
    ConnectionInfo {
        protocol: Protocol::Udp,
        local_addr: local,
        local_port: port(local_port),
        remote_addr: "*".into(),
        remote_port: 0,
        state: ConnectionState::Unknown("connectionless".into()),
        pid: (pid != 0).then_some(pid),
        process_name: None,
    }
}

pub(super) fn collect() -> (Vec<ConnectionInfo>, Observation) {
    let mut connections = Vec::new();
    let mut errors = Vec::new();
    macro_rules! append {
        ($label:literal, $table:ty, $row:ty, $query:expr, $map:expr) => {
            match table::<$row>(offset_of!($table, table), $query) {
                Ok(rows) => connections.extend(rows.into_iter().map($map)),
                Err(error) => errors.push(format!("{}: {error}", $label)),
            }
        };
    }
    append!(
        "TCP IPv4",
        MIB_TCPTABLE_OWNER_PID,
        MIB_TCPROW_OWNER_PID,
        |p, n| unsafe { GetExtendedTcpTable(p, n, 0, 2, TCP_TABLE_OWNER_PID_ALL, 0) },
        |r: MIB_TCPROW_OWNER_PID| tcp(
            ipv4(r.dwLocalAddr),
            r.dwLocalPort,
            ipv4(r.dwRemoteAddr),
            r.dwRemotePort,
            r.dwState,
            r.dwOwningPid
        )
    );
    append!(
        "TCP IPv6",
        MIB_TCP6TABLE_OWNER_PID,
        MIB_TCP6ROW_OWNER_PID,
        |p, n| unsafe { GetExtendedTcpTable(p, n, 0, 23, TCP_TABLE_OWNER_PID_ALL, 0) },
        |r: MIB_TCP6ROW_OWNER_PID| tcp(
            ipv6(r.ucLocalAddr, r.dwLocalScopeId),
            r.dwLocalPort,
            ipv6(r.ucRemoteAddr, r.dwRemoteScopeId),
            r.dwRemotePort,
            r.dwState,
            r.dwOwningPid
        )
    );
    append!(
        "UDP IPv4",
        MIB_UDPTABLE_OWNER_PID,
        MIB_UDPROW_OWNER_PID,
        |p, n| unsafe { GetExtendedUdpTable(p, n, 0, 2, UDP_TABLE_OWNER_PID, 0) },
        |r: MIB_UDPROW_OWNER_PID| udp(ipv4(r.dwLocalAddr), r.dwLocalPort, r.dwOwningPid)
    );
    append!(
        "UDP IPv6",
        MIB_UDP6TABLE_OWNER_PID,
        MIB_UDP6ROW_OWNER_PID,
        |p, n| unsafe { GetExtendedUdpTable(p, n, 0, 23, UDP_TABLE_OWNER_PID, 0) },
        |r: MIB_UDP6ROW_OWNER_PID| udp(
            ipv6(r.ucLocalAddr, r.dwLocalScopeId),
            r.dwLocalPort,
            r.dwOwningPid
        )
    );
    let observation = if errors.is_empty() {
        Observation::available(SOURCE)
    } else {
        Observation::error(
            SOURCE,
            format!("Incomplete endpoint inventory: {}", errors.join("; ")),
        )
    };
    (connections, observation)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn network_order_and_undefined_listener_remote_are_explicit() {
        assert_eq!(port(u16::to_be(443) as u32), 443);
        assert_eq!(ipv4(u32::from_ne_bytes([127, 0, 0, 1])), "127.0.0.1");
        assert_eq!(ipv6(Ipv6Addr::LOCALHOST.octets(), u32::to_be(7)), "::1%7");
        let row = tcp(
            "127.0.0.1".into(),
            u16::to_be(4444) as u32,
            "undefined".into(),
            u32::MAX,
            2,
            0,
        );
        assert_eq!(row.local_port, 4444);
        assert_eq!((row.remote_addr.as_str(), row.remote_port), ("*", 0));
        assert_eq!(row.pid, None);
    }
    #[test]
    fn table_resize_is_bounded_and_rejects_invalid_counts() {
        let mut calls = 0;
        let result = table::<MIB_TCPROW_OWNER_PID>(4, |p, n| {
            calls += 1;
            if p.is_null() {
                *n = 8;
                return 122;
            }
            unsafe {
                p.cast::<u32>().write(1000);
            }
            0
        });
        assert!(result.is_err());
        assert_eq!(calls, 2);
        let mut calls = 0;
        assert!(table::<MIB_TCPROW_OWNER_PID>(4, |_, n| {
            calls += 1;
            *n = 64;
            122
        })
        .is_err());
        assert_eq!(calls, 4);
    }
}
