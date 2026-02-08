use std::net::ToSocketAddrs;
use std::process::Command;
use std::time::Instant;

use super::DiagnosticWarning;

#[derive(Debug, Clone, Default)]
pub struct NetworkDiagData {
    pub gateway: ConnectivityResult,
    pub dns: DnsResult,
    pub internet: ConnectivityResult,
    pub active_connections: Vec<ConnectionInfo>,
    pub listening_ports: Vec<ConnectionInfo>,
}

#[derive(Debug, Clone, Default)]
pub struct ConnectivityResult {
    pub reachable: bool,
    pub latency_ms: Option<f64>,
    pub target: String,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct DnsResult {
    pub resolved: bool,
    pub resolution_ms: Option<f64>,
    pub domain: String,
    pub resolved_ip: Option<String>,
    pub error: Option<String>,
}

#[derive(Debug, Clone)]
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

#[derive(Debug, Clone, PartialEq)]
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

#[derive(Debug, Clone, PartialEq)]
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
    let mut data = NetworkDiagData::default();

    // Gateway detection + ping
    let gateway_ip = detect_gateway();
    if let Some(ref gw) = gateway_ip {
        data.gateway = ping_host(gw);
        data.gateway.target = gw.clone();
    } else {
        data.gateway = ConnectivityResult {
            reachable: false,
            latency_ms: None,
            target: "N/A".into(),
            error: Some("Could not detect default gateway".into()),
        };
    }

    // DNS test
    data.dns = test_dns("www.google.com");

    // Internet ping
    data.internet = ping_host("8.8.8.8");
    data.internet.target = "8.8.8.8".into();

    (data, warnings)
}

/// Refresh only active connections (fast, every 3s)
pub fn refresh_connections(data: &mut NetworkDiagData) {
    let connections = collect_connections();
    data.listening_ports = connections.iter()
        .filter(|c| c.state == ConnectionState::Listening)
        .cloned()
        .collect();
    data.active_connections = connections;
}

// --- Gateway detection ---

fn detect_gateway() -> Option<String> {
    #[cfg(windows)]
    {
        detect_gateway_windows()
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
        None
    }
}

#[cfg(windows)]
fn detect_gateway_windows() -> Option<String> {
    let output = Command::new("route")
        .args(["print", "0.0.0.0"])
        .output()
        .ok()?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    // Parse "route print 0.0.0.0" output: find lines with 0.0.0.0
    for line in stdout.lines() {
        let parts: Vec<&str> = line.split_whitespace().collect();
        // Format: Network Destination, Netmask, Gateway, Interface, Metric
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
fn detect_gateway_linux() -> Option<String> {
    let output = Command::new("ip")
        .args(["route", "show", "default"])
        .output()
        .ok()?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    // "default via 192.168.1.1 dev eth0"
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
fn detect_gateway_macos() -> Option<String> {
    let output = Command::new("route")
        .args(["-n", "get", "default"])
        .output()
        .ok()?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    for line in stdout.lines() {
        let trimmed = line.trim();
        if let Some(rest) = trimmed.strip_prefix("gateway:") {
            return Some(rest.trim().to_string());
        }
    }
    None
}

// --- Ping ---

fn ping_host(host: &str) -> ConnectivityResult {
    let start = Instant::now();

    #[cfg(windows)]
    let result = Command::new("ping")
        .args(["-n", "1", "-w", "3000", host])
        .output();

    #[cfg(not(windows))]
    let result = Command::new("ping")
        .args(["-c", "1", "-W", "3", host])
        .output();

    match result {
        Ok(output) => {
            let elapsed = start.elapsed().as_secs_f64() * 1000.0;
            ConnectivityResult {
                reachable: output.status.success(),
                latency_ms: if output.status.success() { Some(elapsed) } else { None },
                target: host.into(),
                error: if !output.status.success() {
                    Some("Host unreachable".into())
                } else {
                    None
                },
            }
        }
        Err(e) => ConnectivityResult {
            reachable: false,
            latency_ms: None,
            target: host.into(),
            error: Some(format!("Ping failed: {}", e)),
        },
    }
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
                resolved: true,
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

fn collect_connections() -> Vec<ConnectionInfo> {
    #[cfg(windows)]
    {
        collect_connections_windows()
    }
    #[cfg(target_os = "linux")]
    {
        collect_connections_linux()
    }
    #[cfg(target_os = "macos")]
    {
        collect_connections_macos()
    }
    #[cfg(not(any(windows, target_os = "linux", target_os = "macos")))]
    {
        Vec::new()
    }
}

#[cfg(windows)]
fn collect_connections_windows() -> Vec<ConnectionInfo> {
    let mut connections = Vec::new();

    let output = match Command::new("netstat")
        .args(["-ano"])
        .output()
    {
        Ok(o) => o,
        Err(_) => return connections,
    };

    let stdout = String::from_utf8_lossy(&output.stdout);
    for line in stdout.lines().skip(4) {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() < 4 {
            continue;
        }

        let proto_str = parts[0];
        let protocol = match proto_str {
            "TCP" => Protocol::Tcp,
            "UDP" => Protocol::Udp,
            _ => continue,
        };

        let (local_addr, local_port) = parse_addr_port(parts[1]);
        let (remote_addr, remote_port) = if protocol == Protocol::Tcp && parts.len() >= 5 {
            parse_addr_port(parts[2])
        } else {
            ("*".into(), 0)
        };

        let state = if protocol == Protocol::Tcp && parts.len() >= 5 {
            parse_state(parts[3])
        } else if protocol == Protocol::Udp {
            ConnectionState::Unknown("".into())
        } else {
            continue;
        };

        let pid = if protocol == Protocol::Tcp && parts.len() >= 5 {
            parts[4].parse().ok()
        } else if protocol == Protocol::Udp && parts.len() >= 4 {
            parts.last().and_then(|p| p.parse().ok())
        } else {
            None
        };

        connections.push(ConnectionInfo {
            protocol,
            local_addr,
            local_port,
            remote_addr,
            remote_port,
            state,
            pid,
            process_name: None,
        });
    }

    connections
}

#[cfg(target_os = "linux")]
fn collect_connections_linux() -> Vec<ConnectionInfo> {
    let mut connections = Vec::new();

    let output = match Command::new("ss")
        .args(["-tunap"])
        .output()
    {
        Ok(o) => o,
        Err(_) => return connections,
    };

    let stdout = String::from_utf8_lossy(&output.stdout);
    for line in stdout.lines().skip(1) {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() < 5 {
            continue;
        }

        let protocol = match parts[0] {
            "tcp" => Protocol::Tcp,
            "udp" => Protocol::Udp,
            _ => continue,
        };

        let state = parse_state(parts[1]);
        let (local_addr, local_port) = parse_addr_port_unix(parts[4]);
        let (remote_addr, remote_port) = if parts.len() > 5 {
            parse_addr_port_unix(parts[5])
        } else {
            ("*".into(), 0)
        };

        // Try to extract PID from the users: column
        let pid = parts.iter().find_map(|p| {
            if p.contains("pid=") {
                p.split("pid=").nth(1)?.split(|c: char| !c.is_ascii_digit()).next()?.parse().ok()
            } else {
                None
            }
        });

        connections.push(ConnectionInfo {
            protocol,
            local_addr,
            local_port,
            remote_addr,
            remote_port,
            state,
            pid,
            process_name: None,
        });
    }

    connections
}

#[cfg(target_os = "macos")]
fn collect_connections_macos() -> Vec<ConnectionInfo> {
    let mut connections = Vec::new();

    let output = match Command::new("netstat")
        .args(["-anp", "tcp"])
        .output()
    {
        Ok(o) => o,
        Err(_) => return connections,
    };

    let stdout = String::from_utf8_lossy(&output.stdout);
    for line in stdout.lines().skip(2) {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() < 6 {
            continue;
        }

        let protocol = match parts[0] {
            "tcp4" | "tcp6" | "tcp46" => Protocol::Tcp,
            _ => continue,
        };

        let (local_addr, local_port) = parse_addr_port_unix(parts[3]);
        let (remote_addr, remote_port) = parse_addr_port_unix(parts[4]);
        let state = parse_state(parts[5]);

        connections.push(ConnectionInfo {
            protocol,
            local_addr,
            local_port,
            remote_addr,
            remote_port,
            state,
            pid: None,
            process_name: None,
        });
    }

    connections
}

// --- Address parsing helpers ---

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

#[cfg(any(target_os = "linux", target_os = "macos"))]
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
