//! Compatibility fallback for minimal Linux installations without iproute2/ss.
//! procfs exposes the current network namespace, but does not identify owning PIDs.
use super::network_diag::{ConnectionInfo, ConnectionState, Protocol};
use crate::observation::Observation;
use std::{
    io::Read,
    net::{Ipv4Addr, Ipv6Addr},
    path::Path,
};

pub(super) fn collect(root: &Path) -> (Vec<ConnectionInfo>, Observation) {
    let source =
        "Linux procfs endpoints (current network namespace; owner PIDs unavailable without ss)";
    let mut rows = Vec::new();
    let mut failures = Vec::new();
    let mut denied = false;
    for (name, protocol) in [
        ("tcp", Protocol::Tcp),
        ("tcp6", Protocol::Tcp),
        ("udp", Protocol::Udp),
        ("udp6", Protocol::Udp),
    ] {
        let read = || -> std::io::Result<String> {
            let file = std::fs::File::open(root.join(name))?;
            let mut text = String::new();
            file.take(super::command::MAX_OUTPUT_BYTES + 1)
                .read_to_string(&mut text)?;
            if text.len() as u64 > super::command::MAX_OUTPUT_BYTES {
                return Err(std::io::Error::other(
                    "Endpoint table exceeds capture limit",
                ));
            }
            Ok(text)
        };
        match read() {
            Ok(text) => {
                for line in text.lines().skip(1).filter(|line| !line.trim().is_empty()) {
                    if let Some(row) = parse(line, protocol.clone()) {
                        rows.push(row);
                    } else if !failures
                        .iter()
                        .any(|failure: &String| failure.starts_with(name))
                    {
                        failures.push(format!("{name}: malformed endpoint row"));
                    }
                }
            }
            Err(error) if name.ends_with('6') && error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => {
                denied |= error.kind() == std::io::ErrorKind::PermissionDenied;
                failures.push(format!("{name}: {error}"));
            }
        }
    }
    let observation = if failures.is_empty() {
        Observation::available(source)
    } else if denied {
        Observation::permission_denied(source, failures.join("; "))
    } else {
        Observation::error(source, failures.join("; "))
    };
    (rows, observation)
}
fn endpoint(value: &str) -> Option<(String, u16)> {
    let (address, port) = value.split_once(':')?;
    let port = u16::from_str_radix(port, 16).ok()?;
    if !address.is_ascii() {
        return None;
    }
    let address = match address.len() {
        8 => Ipv4Addr::from(u32::from_str_radix(address, 16).ok()?.to_ne_bytes()).to_string(),
        32 => {
            let mut bytes = [0; 16];
            for index in 0..4 {
                bytes[index * 4..index * 4 + 4].copy_from_slice(
                    &u32::from_str_radix(&address[index * 8..index * 8 + 8], 16)
                        .ok()?
                        .to_ne_bytes(),
                );
            }
            Ipv6Addr::from(bytes).to_string()
        }
        _ => return None,
    };
    Some((address, port))
}
fn parse(line: &str, protocol: Protocol) -> Option<ConnectionInfo> {
    let fields: Vec<_> = line.split_whitespace().collect();
    if fields.len() < 4 {
        return None;
    }
    let (local_addr, local_port) = endpoint(fields[1])?;
    let (remote_addr, remote_port) = endpoint(fields[2])?;
    let state = if protocol == Protocol::Udp {
        ConnectionState::Unknown("connectionless".into())
    } else {
        match u8::from_str_radix(fields[3], 16).ok()? {
            1 => ConnectionState::Established,
            2 => ConnectionState::SynSent,
            3 | 12 => ConnectionState::SynReceived,
            4 => ConnectionState::FinWait1,
            5 => ConnectionState::FinWait2,
            6 => ConnectionState::TimeWait,
            7 => ConnectionState::Unknown("CLOSED".into()),
            8 => ConnectionState::CloseWait,
            9 => ConnectionState::LastAck,
            10 => ConnectionState::Listening,
            11 => ConnectionState::Closing,
            value => ConnectionState::Unknown(format!("OS state {value}")),
        }
    };
    Some(ConnectionInfo {
        protocol,
        local_addr,
        local_port,
        remote_addr,
        remote_port,
        state,
        pid: None,
        process_name: None,
    })
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn procfs_units_ipv6_and_missing_owners_are_explicit() {
        let row = parse(
            "0: 0100007F:01BB 00000000:0000 0A 0 0 0 1000 0 1234",
            Protocol::Tcp,
        )
        .unwrap();
        assert_eq!(
            (row.local_addr.as_str(), row.local_port),
            ("127.0.0.1", 443)
        );
        assert_eq!(row.state, ConnectionState::Listening);
        assert!(row.pid.is_none());
        assert_eq!(
            endpoint("00000000000000000000000001000000:1F90"),
            Some(("::1".into(), 8080))
        );
        assert!(endpoint("wrong:01BB").is_none());
    }
    #[test]
    fn minimal_host_fallback_retains_partial_inventory_and_reports_unreadable_tables() {
        let temp = tempfile::tempdir().unwrap();
        std::fs::write(
            temp.path().join("tcp"),
            "header\n0: 0100007F:01BB 00000000:0000 0A\n",
        )
        .unwrap();
        std::fs::write(
            temp.path().join("udp"),
            "header\n0: 0100007F:14E9 00000000:0000 07\n",
        )
        .unwrap();
        let (rows, observation) = collect(temp.path());
        assert_eq!(rows.len(), 2);
        assert!(observation.is_available());
        std::fs::remove_file(temp.path().join("udp")).unwrap();
        let (rows, observation) = collect(temp.path());
        assert_eq!(rows.len(), 1);
        assert!(!observation.is_available());
    }
}
