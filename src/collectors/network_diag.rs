use std::net::ToSocketAddrs;
use std::time::Instant;

use serde::Serialize;

use super::command::CommandError;
#[cfg(any(test, not(windows)))]
use super::command::CommandTimeout;
#[cfg(not(windows))]
use super::command::{run_output, run_stdout};
use super::DiagnosticWarning;
use crate::observation::Observation;

#[derive(Debug, Clone, Default, Serialize, serde::Deserialize)]
pub struct NetworkDiagData {
    pub gateway: ConnectivityResult,
    pub dns: DnsResult,
    pub internet: ConnectivityResult,
    pub active_connections: Vec<ConnectionInfo>,
    pub listening_ports: Vec<ConnectionInfo>,
    #[serde(default)]
    pub connections_observation: Observation,
}

#[derive(Debug, Clone, Default, Serialize, serde::Deserialize)]
pub struct ConnectivityResult {
    pub reachable: bool,
    pub latency_ms: Option<f64>,
    pub target: String,
    pub error: Option<String>,
    #[serde(default)]
    pub source: String,
    #[serde(default)]
    pub latency_resolution_ms: Option<f64>,
}

#[derive(Debug, Clone, Default, Serialize, serde::Deserialize)]
pub struct DnsResult {
    pub resolved: bool,
    pub resolution_ms: Option<f64>,
    pub domain: String,
    pub resolved_ip: Option<String>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, serde::Deserialize)]
pub struct ConnectionInfo {
    pub protocol: Protocol,
    pub local_addr: String,
    pub local_port: u16,
    pub remote_addr: String,
    pub remote_port: u16,
    pub state: ConnectionState,
    pub pid: Option<u32>,
    pub process_name: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Protocol {
    Tcp,
    Udp,
}

impl std::fmt::Display for Protocol {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Tcp => write!(f, "TCP"),
            Self::Udp => write!(f, "UDP"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConnectionState {
    Established,
    Listening,
    TimeWait,
    CloseWait,
    SynSent,
    SynReceived,
    FinWait1,
    FinWait2,
    LastAck,
    Closing,
    Unknown(String),
}

impl std::fmt::Display for ConnectionState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Established => write!(f, "ESTABLISHED"),
            Self::Listening => write!(f, "LISTENING"),
            Self::TimeWait => write!(f, "TIME_WAIT"),
            Self::CloseWait => write!(f, "CLOSE_WAIT"),
            Self::SynSent => write!(f, "SYN_SENT"),
            Self::SynReceived => write!(f, "SYN_RECV"),
            Self::FinWait1 => write!(f, "FIN_WAIT_1"),
            Self::FinWait2 => write!(f, "FIN_WAIT_2"),
            Self::LastAck => write!(f, "LAST_ACK"),
            Self::Closing => write!(f, "CLOSING"),
            Self::Unknown(s) => write!(f, "{}", s),
        }
    }
}

#[cfg(any(not(windows), test))]
fn parse_state(s: &str) -> ConnectionState {
    match s.trim() {
        "ESTABLISHED" => ConnectionState::Established,
        "LISTENING" | "LISTEN" => ConnectionState::Listening,
        "TIME_WAIT" => ConnectionState::TimeWait,
        "CLOSE_WAIT" => ConnectionState::CloseWait,
        "SYN_SENT" => ConnectionState::SynSent,
        "SYN_RECEIVED" | "SYN_RECV" => ConnectionState::SynReceived,
        "FIN_WAIT_1" => ConnectionState::FinWait1,
        "FIN_WAIT_2" => ConnectionState::FinWait2,
        "LAST_ACK" => ConnectionState::LastAck,
        "CLOSING" => ConnectionState::Closing,
        other => ConnectionState::Unknown(other.to_string()),
    }
}

/// Collect connectivity diagnostics (ping/DNS) - call from spawn_blocking
pub fn collect_connectivity() -> (NetworkDiagData, Vec<DiagnosticWarning>) {
    let warnings = Vec::new();
    // Gateway detection + ping
    let gateway = match detect_gateway() {
        Ok(Some(gw)) => ping_host(&gw),
        result => ConnectivityResult {
            reachable: false,
            latency_ms: None,
            target: "N/A".into(),
            error: Some(match result {
                Err(error) => format!("Default gateway query: {error}"),
                _ => "No default gateway was reported by the routing provider".into(),
            }),
            source: "OS route query".into(),
            latency_resolution_ms: None,
        },
    };

    let mut data = NetworkDiagData {
        gateway,
        dns: test_dns("www.google.com"),
        internet: ping_host("1.1.1.1"),
        ..Default::default()
    };
    // ICMP is commonly filtered independently of ordinary internet access.
    // A successful TCP connection proves reachability, not an ICMP RTT.
    if !data.internet.reachable {
        let address = std::net::SocketAddr::from(([1, 1, 1, 1], 443));
        if std::net::TcpStream::connect_timeout(&address, std::time::Duration::from_secs(2)).is_ok()
        {
            data.internet.reachable = true;
            data.internet.latency_ms = None;
            data.internet.target = "1.1.1.1:443 (TCP fallback)".into();
            data.internet.error = None;
            data.internet.source = "TCP connect_timeout; reachability only".into();
            data.internet.latency_resolution_ms = None;
        } else {
            data.internet.error = Some("No ICMP or TCP response from the probe target; filtering or target failure may also cause this".into());
        }
    }

    (data, warnings)
}

/// Refresh only active connections (fast, every 3s)
pub fn refresh_connections(data: &mut NetworkDiagData) {
    let (connections, observation) = collect_connections();
    data.connections_observation = observation;
    data.listening_ports = connections
        .iter()
        .filter(|c| c.state == ConnectionState::Listening)
        .cloned()
        .collect();
    data.active_connections = connections;
}

// --- Gateway detection ---

fn detect_gateway() -> Result<Option<String>, CommandError> {
    #[cfg(windows)]
    {
        super::windows_connectivity::gateway()
    }
    #[cfg(target_os = "linux")]
    {
        detect_gateway_linux()
    }
    #[cfg(target_os = "macos")]
    {
        detect_gateway_macos()
    }
    #[cfg(not(any(windows, target_os = "linux", target_os = "macos")))]
    {
        Ok(None)
    }
}

#[cfg(test)]
fn parse_windows_gateway(stdout: &str) -> Option<String> {
    for line in stdout.lines() {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() >= 4 && parts[0] == "0.0.0.0" && parts[1] == "0.0.0.0" {
            let gw = parts[2];
            if gw != "0.0.0.0" {
                return Some(gw.to_string());
            }
        }
    }
    None
}

#[cfg(target_os = "linux")]
fn detect_gateway_linux() -> Result<Option<String>, CommandError> {
    let stdout = run_stdout("ip", ["route", "show", "default"], CommandTimeout::Normal)?;
    Ok(parse_linux_default_gateway(&stdout))
}

#[cfg_attr(not(target_os = "linux"), allow(dead_code))]
fn parse_linux_default_gateway(stdout: &str) -> Option<String> {
    for line in stdout.lines() {
        if line.starts_with("default") {
            if let Some(idx) = line.find("via ") {
                let rest = &line[idx + 4..];
                return rest.split_whitespace().next().map(|s| s.to_string());
            }
        }
    }
    None
}

#[cfg(target_os = "macos")]
fn detect_gateway_macos() -> Result<Option<String>, CommandError> {
    let stdout = run_stdout("route", ["-n", "get", "default"], CommandTimeout::Normal)?;
    Ok(parse_macos_gateway(&stdout))
}

#[cfg_attr(not(target_os = "macos"), allow(dead_code))]
fn parse_macos_gateway(stdout: &str) -> Option<String> {
    for line in stdout.lines() {
        let trimmed = line.trim();
        if let Some(rest) = trimmed.strip_prefix("gateway:") {
            return Some(rest.trim().to_string());
        }
    }
    None
}

// --- Ping ---

#[cfg(windows)]
fn ping_host(host: &str) -> ConnectivityResult {
    super::windows_connectivity::ping(host)
}

#[cfg(not(windows))]
fn ping_host(host: &str) -> ConnectivityResult {
    let result = run_output(
        "ping",
        ping_args(std::env::consts::OS, host),
        CommandTimeout::Slow,
    );

    match result {
        Ok(output) => {
            let rtt = parse_ping_rtt(&String::from_utf8_lossy(&output.stdout));
            ConnectivityResult {
                reachable: output.status.success(),
                latency_ms: if output.status.success() { rtt } else { None },
                target: host.into(),
                source: format!("{} ping ICMP reply", std::env::consts::OS),
                latency_resolution_ms: None,
                error: if !output.status.success() {
                    Some("No ICMP reply; the target or network may filter ICMP".into())
                } else if rtt.is_none() {
                    Some("ICMP replied; exact round-trip time was unavailable or below the provider resolution".into())
                } else {
                    None
                },
            }
        }
        Err(error) => ConnectivityResult {
            reachable: false,
            latency_ms: None,
            target: host.into(),
            error: Some(format!("ICMP provider: {error}")),
            source: format!("{} ping ICMP reply", std::env::consts::OS),
            latency_resolution_ms: None,
        },
    }
}

#[cfg(any(test, not(windows)))]
fn ping_args<'a>(os: &str, host: &'a str) -> Vec<&'a str> {
    match os {
        "windows" => vec!["-n", "1", "-w", "3000", host],
        "macos" => vec!["-n", "-c", "1", "-W", "3000", host],
        _ => vec!["-n", "-c", "1", "-W", "3", host],
    }
}

/// Read reply RTT, never subprocess duration or summary extrema. A reported
/// upper bound (time<1ms) is not an exact measurement and remains unavailable.
#[cfg(any(test, not(windows)))]
fn parse_ping_rtt(output: &str) -> Option<f64> {
    output.lines().find_map(|line| {
        let lower = line.to_lowercase();
        ["time=", "temps=", "zeit=", "tiempo=", "tempo="]
            .iter()
            .find_map(|marker| {
                let start = lower.find(marker)? + marker.len();
                let rest = lower[start..].trim_start();
                let end = rest
                    .find(|c: char| !c.is_ascii_digit() && c != '.' && c != ',')
                    .unwrap_or(rest.len());
                if !rest[end..].trim_start().starts_with("ms") {
                    return None;
                }
                rest[..end]
                    .replace(',', ".")
                    .parse::<f64>()
                    .ok()
                    .filter(|value| value.is_finite() && *value >= 0.0)
            })
    })
}

// --- DNS test ---

fn test_dns(domain: &str) -> DnsResult {
    let start = Instant::now();
    let lookup = format!("{}:80", domain);

    match lookup.to_socket_addrs() {
        Ok(mut addrs) => {
            let elapsed = start.elapsed().as_secs_f64() * 1000.0;
            let ip = addrs.next().map(|a| a.ip().to_string());
            DnsResult {
                resolved: ip.is_some(),
                resolution_ms: Some(elapsed),
                domain: domain.into(),
                resolved_ip: ip,
                error: None,
            }
        }
        Err(e) => {
            let elapsed = start.elapsed().as_secs_f64() * 1000.0;
            DnsResult {
                resolved: false,
                resolution_ms: Some(elapsed),
                domain: domain.into(),
                resolved_ip: None,
                error: Some(format!("{}", e)),
            }
        }
    }
}

// --- Connection tracking ---

fn collect_connections() -> (Vec<ConnectionInfo>, Observation) {
    #[cfg(windows)]
    {
        super::windows_connections::collect()
    }
    #[cfg(target_os = "linux")]
    {
        let result = command_connections("ss", &["-Htunap"], parse_linux_connections);
        if result.1.status == crate::observation::ObservationStatus::Unavailable {
            super::linux_connections::collect(std::path::Path::new("/proc/net"))
        } else {
            result
        }
    }
    #[cfg(target_os = "macos")]
    {
        command_connections("netstat", &["-an"], parse_macos_connections)
    }
    #[cfg(not(any(windows, target_os = "linux", target_os = "macos")))]
    {
        (
            Vec::new(),
            Observation::unsupported(
                "sockets",
                "Endpoint inventory is not supported on this platform",
            ),
        )
    }
}

#[cfg(any(not(windows), test))]
fn command_connections(
    program: &str,
    args: &[&str],
    parse: fn(&str) -> Vec<ConnectionInfo>,
) -> (Vec<ConnectionInfo>, Observation) {
    use super::command::{run_checked, CommandError};
    let result = run_checked(
        program,
        args,
        CommandTimeout::Normal,
        &std::sync::atomic::AtomicBool::new(false),
    );
    match result {
        Ok(output) if output.status.success() => (
            parse(&String::from_utf8_lossy(&output.stdout)),
            Observation::available(program),
        ),
        Ok(output) => (
            Vec::new(),
            Observation::error(
                program,
                format!("Endpoint provider exited with {}", output.status),
            ),
        ),
        Err(CommandError::PermissionDenied) => (
            Vec::new(),
            Observation::permission_denied(
                program,
                "Endpoint enumeration was denied by the operating system",
            ),
        ),
        Err(CommandError::NotFound) => (
            Vec::new(),
            Observation::unavailable(
                program,
                format!("The {program} endpoint provider is not installed"),
            ),
        ),
        Err(error) => (Vec::new(), Observation::error(program, error.to_string())),
    }
}

#[cfg(any(target_os = "linux", test))]
fn parse_linux_connections(stdout: &str) -> Vec<ConnectionInfo> {
    stdout
        .lines()
        .filter_map(|line| {
            let parts: Vec<_> = line.split_whitespace().collect();
            if parts.len() < 6 {
                return None;
            }
            let protocol = match parts[0] {
                "tcp" => Protocol::Tcp,
                "udp" => Protocol::Udp,
                _ => return None,
            };
            let state = match parts[1] {
                "ESTAB" => ConnectionState::Established,
                "SYN-RECV" => ConnectionState::SynReceived,
                "SYN-SENT" => ConnectionState::SynSent,
                "FIN-WAIT-1" => ConnectionState::FinWait1,
                "FIN-WAIT-2" => ConnectionState::FinWait2,
                "CLOSE-WAIT" => ConnectionState::CloseWait,
                "TIME-WAIT" => ConnectionState::TimeWait,
                "LAST-ACK" => ConnectionState::LastAck,
                other => parse_state(other),
            };
            let (local_addr, local_port) = parse_addr_port_unix(parts[4]);
            let (remote_addr, remote_port) = parse_addr_port_unix(parts[5]);
            let pid = parts.iter().find_map(|p| {
                p.split("pid=")
                    .nth(1)?
                    .split(|c: char| !c.is_ascii_digit())
                    .next()?
                    .parse()
                    .ok()
            });
            Some(ConnectionInfo {
                protocol,
                local_addr,
                local_port,
                remote_addr,
                remote_port,
                state,
                pid,
                process_name: None,
            })
        })
        .collect()
}

#[cfg(any(target_os = "macos", test))]
fn parse_macos_connections(stdout: &str) -> Vec<ConnectionInfo> {
    stdout
        .lines()
        .filter_map(|line| {
            let parts: Vec<_> = line.split_whitespace().collect();
            if parts.len() < 5 {
                return None;
            }
            let protocol = match parts[0] {
                "tcp4" | "tcp6" | "tcp46" => Protocol::Tcp,
                "udp4" | "udp6" | "udp46" => Protocol::Udp,
                _ => return None,
            };
            let state = if protocol == Protocol::Tcp {
                parse_state(parts.get(5)?)
            } else {
                ConnectionState::Unknown("connectionless".into())
            };
            let (local_addr, local_port) = parse_addr_port_unix(parts[3]);
            let (remote_addr, remote_port) = parse_addr_port_unix(parts[4]);
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
        })
        .collect()
}

// --- Address parsing helpers ---

#[cfg(any(not(windows), test))]
fn parse_addr_port(addr_str: &str) -> (String, u16) {
    // Windows format: "192.168.1.1:443" or "[::1]:443"
    if let Some(bracket_end) = addr_str.rfind(']') {
        // IPv6 in brackets — bounds-check both ends
        let addr = if addr_str.starts_with('[') {
            &addr_str[1..bracket_end]
        } else {
            &addr_str[..bracket_end]
        };
        let port_str = addr_str.get(bracket_end + 2..).unwrap_or("0");
        let port = port_str.parse().unwrap_or(0);
        (addr.to_string(), port)
    } else if let Some(colon_pos) = addr_str.rfind(':') {
        let addr = &addr_str[..colon_pos];
        let port_str = &addr_str[colon_pos + 1..];
        let port = port_str.parse().unwrap_or(0);
        (addr.to_string(), port)
    } else {
        (addr_str.to_string(), 0)
    }
}

#[cfg(any(not(windows), test))]
fn parse_addr_port_unix(addr_str: &str) -> (String, u16) {
    // Unix format: "192.168.1.1:443" or ":::443" or "[::]:443" or "*:*"
    if addr_str == "*:*" || addr_str == "*.*" {
        return ("*".into(), 0);
    }

    if let Some(bracket_end) = addr_str.rfind(']') {
        let addr = if addr_str.starts_with('[') {
            &addr_str[1..bracket_end]
        } else {
            &addr_str[..bracket_end]
        };
        let port_str = addr_str.get(bracket_end + 2..).unwrap_or("0");
        let port = port_str.parse().unwrap_or(0);
        (addr.to_string(), port)
    } else if let Some(dot_pos) = addr_str.rfind('.') {
        // macOS uses dots: "*.80" or "192.168.1.1.443"
        let port_str = &addr_str[dot_pos + 1..];
        if let Ok(port) = port_str.parse::<u16>() {
            let addr = &addr_str[..dot_pos];
            (addr.to_string(), port)
        } else {
            parse_addr_port(addr_str)
        }
    } else {
        parse_addr_port(addr_str)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn linux_socket_states_and_inaccessible_owners_are_preserved() {
        let rows = parse_linux_connections("tcp ESTAB 0 0 127.0.0.1:42000 127.0.0.1:443 users:((\"test\",pid=42,fd=3))\ntcp TIME-WAIT 0 0 [::1]:42001 [::1]:443\nudp UNCONN 0 0 0.0.0.0:5353 0.0.0.0:*\n");
        assert_eq!(rows.len(), 3);
        assert_eq!(rows[0].state, ConnectionState::Established);
        assert_eq!(rows[0].pid, Some(42));
        assert_eq!(rows[1].state, ConnectionState::TimeWait);
        assert_eq!(rows[1].pid, None);
        assert_eq!(rows[2].protocol, Protocol::Udp);
        assert_eq!(rows[2].local_port, 5353);
    }
    #[test]
    fn macos_inventory_includes_udp_ipv6_and_wildcard_endpoints() {
        let rows = parse_macos_connections("Active Internet connections\nProto Recv-Q Send-Q Local Address Foreign Address (state)\ntcp4 0 0 127.0.0.1.51000 127.0.0.1.443 ESTABLISHED\nudp6 0 0 fe80::1%en0.5353 *.*\ntcp46 0 0 *.8080 *.* LISTEN\n/var/run/unix-socket ignored\n");
        assert_eq!(rows.len(), 3);
        assert_eq!(rows[0].state, ConnectionState::Established);
        assert_eq!(rows[1].protocol, Protocol::Udp);
        assert_eq!(rows[1].local_addr, "fe80::1%en0");
        assert_eq!(rows[1].local_port, 5353);
        assert_eq!(rows[1].pid, None);
        assert_eq!(rows[2].state, ConnectionState::Listening);
    }
    #[test]
    fn missing_endpoint_provider_is_not_a_measured_empty_inventory() {
        let (rows, observation) = command_connections(
            "sd300-no-socket-provider-fixture-9471",
            &[],
            parse_linux_connections,
        );
        assert!(rows.is_empty());
        assert_eq!(
            observation.status,
            crate::observation::ObservationStatus::Unavailable
        );
        assert!(observation.detail.unwrap().contains("not installed"));
    }

    #[test]
    fn reply_rtt_is_not_process_runtime_or_summary() {
        assert_eq!(
            parse_ping_rtt("64 bytes from 1.1.1.1: icmp_seq=1 ttl=58 time=12.34 ms"),
            Some(12.34)
        );
        assert_eq!(
            parse_ping_rtt("Reply from 1.1.1.1: bytes=32 time=14ms TTL=58"),
            Some(14.0)
        );
        assert_eq!(
            parse_ping_rtt("Antwort: Bytes=32 Zeit=1,25ms TTL=58"),
            Some(1.25)
        );
        assert_eq!(parse_ping_rtt("Reply: bytes=32 time<1ms TTL=58"), None);
        assert_eq!(parse_ping_rtt("Minimum = 0ms, Maximum = 10ms"), None);
        assert_eq!(parse_ping_rtt("time=NaN ms"), None);
    }

    #[test]
    fn ping_timeout_units_match_each_operating_system() {
        assert_eq!(
            ping_args("macos", "host"),
            ["-n", "-c", "1", "-W", "3000", "host"]
        );
        assert_eq!(
            ping_args("linux", "host"),
            ["-n", "-c", "1", "-W", "3", "host"]
        );
        assert_eq!(
            ping_args("windows", "host"),
            ["-n", "1", "-w", "3000", "host"]
        );
    }

    #[test]
    fn parses_linux_default_gateway_fixture() {
        let fixture =
            "default via 192.168.86.1 dev wlan0 proto dhcp src 192.168.86.25 metric 600\n";
        assert_eq!(
            parse_linux_default_gateway(fixture),
            Some("192.168.86.1".into())
        );
    }

    #[test]
    fn parses_macos_gateway_fixture() {
        let fixture = "   route to: default\n   destination: default\n       gateway: 10.0.0.1\n";
        assert_eq!(parse_macos_gateway(fixture), Some("10.0.0.1".into()));
    }

    #[test]
    fn parses_windows_route_fixture() {
        let fixture = "\
Network Destination        Netmask          Gateway       Interface  Metric
          0.0.0.0          0.0.0.0       172.16.0.1     172.16.0.50     25
";
        assert_eq!(parse_windows_gateway(fixture), Some("172.16.0.1".into()));
    }

    #[test]
    fn parses_windows_ipv6_socket_with_port() {
        assert_eq!(parse_addr_port("[fe80::1]:443"), ("fe80::1".into(), 443));
    }

    #[test]
    fn parses_macos_dotted_socket_with_port() {
        assert_eq!(
            parse_addr_port_unix("192.168.1.20.5353"),
            ("192.168.1.20".into(), 5353)
        );
    }
}
