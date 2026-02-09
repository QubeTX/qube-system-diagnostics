use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Paragraph, Sparkline};
use ratatui::Frame;

use crate::app::App;
use crate::collectors::network_diag::ConnectionState;
use crate::types::{DiagnosticMode, HealthStatus};
use crate::ui::common::*;

pub fn render(frame: &mut Frame, app: &App, area: Rect, mode: DiagnosticMode) {
    match mode {
        DiagnosticMode::User => render_user(frame, app, area),
        DiagnosticMode::Technician => render_tech(frame, app, area),
    }
}

fn render_user(frame: &mut Frame, app: &App, area: Rect) {
    let outer = content_block("Network");
    let inner = outer.inner(area);
    frame.render_widget(outer, area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(11),
            Constraint::Min(6),
        ])
        .split(inner);

    let net = &app.snapshot.network;
    let diag = &app.snapshot.network_diag;
    let connected = !net.interfaces.is_empty();

    let conn_type = net.interfaces.iter().find_map(|i| {
        let name = i.name.to_lowercase();
        if name.contains("wlan") || name.contains("wi-fi") || name.contains("wifi") || name.contains("wlp") {
            Some("Wi-Fi")
        } else if name.contains("eth") || name.contains("enp") || name.contains("ethernet") {
            Some("Wired connection")
        } else {
            None
        }
    }).unwrap_or(if connected { "Connected" } else { "Disconnected" });

    let speed_desc = plain_language_speed(net.total_download_rate);
    let status = if connected { HealthStatus::Good } else { HealthStatus::Warning };

    let mut lines = vec![
        Line::from(""),
        status_line(&status, "Connection", conn_type),
        Line::from(vec![
            Span::styled("  Speed          ", Style::default().fg(COLOR_TEXT)),
            Span::styled(
                format!("Downloading at {}  \u{2022}  Uploading at {}",
                    format_throughput(net.total_download_rate),
                    format_throughput(net.total_upload_rate)),
                Style::default().fg(COLOR_DIM),
            ),
        ]),
        Line::from(vec![
            Span::styled("  Quality        ", Style::default().fg(COLOR_TEXT)),
            Span::styled(speed_desc.to_string(), Style::default().fg(COLOR_DIM)),
        ]),
        Line::from(""),
    ];

    // Connectivity diagnostics
    let gw_status = if diag.gateway.reachable { HealthStatus::Good } else { HealthStatus::Critical };
    let gw_desc = if diag.gateway.reachable {
        format!("Reachable ({})", diag.gateway.latency_ms.map(|l| format!("{:.0}ms", l)).unwrap_or_else(|| "N/A".into()))
    } else {
        diag.gateway.error.clone().unwrap_or_else(|| "Unreachable".into())
    };
    lines.push(status_line(&gw_status, "Gateway", &gw_desc));

    let dns_status = if diag.dns.resolved { HealthStatus::Good } else { HealthStatus::Critical };
    let dns_desc = if diag.dns.resolved {
        format!("Working ({})", diag.dns.resolution_ms.map(|l| format!("{:.0}ms", l)).unwrap_or_else(|| "N/A".into()))
    } else {
        diag.dns.error.clone().unwrap_or_else(|| "Failed".into())
    };
    lines.push(status_line(&dns_status, "DNS", &dns_desc));

    let inet_status = if diag.internet.reachable { HealthStatus::Good } else { HealthStatus::Critical };
    let inet_desc = if diag.internet.reachable {
        format!("Online ({})", diag.internet.latency_ms.map(|l| format!("{:.0}ms", l)).unwrap_or_else(|| "N/A".into()))
    } else {
        "Offline".into()
    };
    lines.push(status_line(&inet_status, "Internet", &inet_desc));

    let status_panel = Paragraph::new(lines);
    frame.render_widget(status_panel, chunks[0]);

    // Sparkline
    let spark_data = app.net_down_history.as_u64_vec();
    let max_val = spark_data.iter().copied().max().unwrap_or(1).max(1);
    let sparkline = Sparkline::default()
        .block(sub_block("Download Speed (last 60 seconds)"))
        .data(&spark_data)
        .max(max_val)
        .bar_set(sparkline_bar_set())
        .style(Style::default().fg(SPARK_NET_DOWN));
    frame.render_widget(sparkline, chunks[1]);
}

fn render_tech(frame: &mut Frame, app: &App, area: Rect) {
    let net = &app.snapshot.network;
    let diag = &app.snapshot.network_diag;
    let has_connections = !diag.active_connections.is_empty();

    let outer = content_block(&format!("Network \u{2014} {} interfaces", net.interfaces.len()));
    let inner = outer.inner(area);
    frame.render_widget(outer, area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(2),  // Header + throughput
            Constraint::Length(3),  // Connectivity status
            Constraint::Min(6),    // Interfaces + connections
            Constraint::Length(7), // Sparklines
        ])
        .split(inner);

    // Header
    let header_lines = vec![
        Line::from(vec![
            Span::styled(
                format!("  \u{2193} {}  \u{2191} {}",
                    format_throughput(net.total_download_rate),
                    format_throughput(net.total_upload_rate)),
                Style::default().fg(COLOR_TEXT),
            ),
        ]),
        Line::from(Span::styled(
            format!("  {:<20} {:<18} {:<18} {:>12} {:>12}", "INTERFACE", "MAC", "IP", "RX/s", "TX/s"),
            Style::default().fg(COLOR_DIM),
        )),
    ];
    let header_panel = Paragraph::new(header_lines);
    frame.render_widget(header_panel, chunks[0]);

    // Connectivity status row
    let gw_color = if diag.gateway.reachable { COLOR_GOOD } else { COLOR_CRIT };
    let dns_color = if diag.dns.resolved { COLOR_GOOD } else { COLOR_CRIT };
    let inet_color = if diag.internet.reachable { COLOR_GOOD } else { COLOR_CRIT };

    let gw_text = format!("GW {}",
        diag.gateway.latency_ms.map(|l| format!("{:.0}ms", l)).unwrap_or_else(|| "N/A".into()));
    let dns_text = format!("DNS {}",
        diag.dns.resolution_ms.map(|l| format!("{:.0}ms", l)).unwrap_or_else(|| "N/A".into()));
    let inet_text = if diag.internet.reachable { "INET OK" } else { "INET DOWN" };

    let conn_status_lines = vec![
        Line::from(vec![
            Span::styled("  ", Style::default()),
            Span::styled(gw_text, Style::default().fg(gw_color)),
            Span::styled("  ", Style::default()),
            Span::styled(dns_text, Style::default().fg(dns_color)),
            Span::styled("  ", Style::default()),
            Span::styled(inet_text.to_string(), Style::default().fg(inet_color)),
        ]),
        Line::from(""),
    ];
    let conn_status_panel = Paragraph::new(conn_status_lines);
    frame.render_widget(conn_status_panel, chunks[1]);

    // Interfaces + connections
    let mid_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints(if has_connections {
            vec![
                Constraint::Length(net.interfaces.len() as u16 + 1),
                Constraint::Min(4),
            ]
        } else {
            vec![Constraint::Min(1)]
        })
        .split(chunks[2]);

    // Interface table
    let mut iface_lines = Vec::new();
    for iface in &net.interfaces {
        let ip = iface.ip_addresses.first().map(|s| s.as_str()).unwrap_or("N/A");
        iface_lines.push(Line::from(Span::styled(
            format!("  {:<20} {:<18} {:<18} {:>12} {:>12}",
                truncate_str(&iface.name, 20),
                truncate_str(&iface.mac_address, 18),
                truncate_str(ip, 18),
                format_throughput(iface.download_rate),
                format_throughput(iface.upload_rate)),
            Style::default().fg(COLOR_TEXT),
        )));
    }
    let iface_panel = Paragraph::new(iface_lines);
    frame.render_widget(iface_panel, mid_chunks[0]);

    // Active connections table
    if has_connections && mid_chunks.len() > 1 {
        let conn_block = sub_block(&format!("Active Connections ({})  j/k to scroll", diag.active_connections.len()));
        let conn_inner = conn_block.inner(mid_chunks[1]);
        frame.render_widget(conn_block, mid_chunks[1]);

        let mut conn_lines = Vec::new();
        conn_lines.push(Line::from(Span::styled(
            format!("  {:<6} {:<22} {:<22} {:<14} {:>6}", "PROTO", "LOCAL", "REMOTE", "STATE", "PID"),
            Style::default().fg(COLOR_DIM),
        )));

        let established: Vec<_> = diag.active_connections.iter()
            .filter(|c| c.state == ConnectionState::Established)
            .collect();
        let listening: Vec<_> = diag.active_connections.iter()
            .filter(|c| c.state == ConnectionState::Listening)
            .collect();

        let visible_height = conn_inner.height.saturating_sub(2) as usize;
        let scroll = app.connection_scroll;
        let total_connections = established.len() + listening.len();

        for conn in established.iter().chain(listening.iter()).skip(scroll).take(visible_height) {
            let state_color = match conn.state {
                ConnectionState::Established => COLOR_GOOD,
                ConnectionState::Listening => COLOR_INFO,
                ConnectionState::TimeWait | ConnectionState::CloseWait => COLOR_WARN,
                _ => COLOR_DIM,
            };

            let local = format!("{}:{}", truncate_str(&conn.local_addr, 16), conn.local_port);
            let remote = if conn.remote_addr == "*" || conn.remote_addr == "0.0.0.0" {
                "*".to_string()
            } else {
                format!("{}:{}", truncate_str(&conn.remote_addr, 16), conn.remote_port)
            };
            let pid_str = conn.pid.map(|p| format!("{}", p)).unwrap_or_default();

            conn_lines.push(Line::from(vec![
                Span::styled(
                    format!("  {:<6} {:<22} {:<22} ", conn.protocol, truncate_str(&local, 22), truncate_str(&remote, 22)),
                    Style::default().fg(COLOR_TEXT),
                ),
                Span::styled(format!("{:<14}", conn.state), Style::default().fg(state_color)),
                Span::styled(format!("{:>6}", pid_str), Style::default().fg(COLOR_DIM)),
            ]));
        }

        // Scroll indicator
        let end = (scroll + visible_height).min(total_connections);
        if total_connections > 0 {
            conn_lines.push(Line::from(Span::styled(
                format!("  Showing {}-{} of {}", scroll + 1, end, total_connections),
                Style::default().fg(COLOR_DIM),
            )));
        }

        let conn_panel = Paragraph::new(conn_lines);
        frame.render_widget(conn_panel, conn_inner);
    }

    // Download/Upload sparklines
    let spark_area = chunks[3];
    let spark_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(spark_area);

    let down_data = app.net_down_history.as_u64_vec();
    let down_max = down_data.iter().copied().max().unwrap_or(1).max(1);
    let down_sparkline = Sparkline::default()
        .block(sub_block("Download"))
        .data(&down_data)
        .max(down_max)
        .bar_set(sparkline_bar_set())
        .style(Style::default().fg(SPARK_NET_DOWN));
    frame.render_widget(down_sparkline, spark_chunks[0]);

    let up_data = app.net_up_history.as_u64_vec();
    let up_max = up_data.iter().copied().max().unwrap_or(1).max(1);
    let up_sparkline = Sparkline::default()
        .block(sub_block("Upload"))
        .data(&up_data)
        .max(up_max)
        .bar_set(sparkline_bar_set())
        .style(Style::default().fg(SPARK_NET_UP));
    frame.render_widget(up_sparkline, spark_chunks[1]);
}
