use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Sparkline};
use ratatui::Frame;

use crate::app::App;
use crate::types::{DiagnosticMode, HealthStatus};
use crate::ui::common::*;

pub fn render(frame: &mut Frame, app: &App, area: Rect, mode: DiagnosticMode) {
    match mode {
        DiagnosticMode::User => render_user(frame, app, area),
        DiagnosticMode::Technician => render_tech(frame, app, area),
    }
}

fn render_user(frame: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(2),
            Constraint::Length(8),
            Constraint::Min(6),
        ])
        .split(area);

    let header = Paragraph::new(Line::from(Span::styled(
        "  NETWORK",
        Style::default().fg(COLOR_HEADER).add_modifier(Modifier::BOLD),
    )));
    frame.render_widget(header, chunks[0]);

    let net = &app.snapshot.network;
    let connected = !net.interfaces.is_empty();

    // Guess connection type from interface names
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
        separator(area.width as usize),
        status_line(&status, "Connection", conn_type),
        Line::from(vec![
            Span::styled("  Speed          ", Style::default().fg(Color::White)),
            Span::styled(
                format!("Downloading at {}  \u{2022}  Uploading at {}",
                    format_throughput(net.total_download_rate),
                    format_throughput(net.total_upload_rate)),
                Style::default().fg(COLOR_DIM),
            ),
        ]),
        Line::from(vec![
            Span::styled("  Quality        ", Style::default().fg(Color::White)),
            Span::styled(format!("{}", speed_desc), Style::default().fg(COLOR_DIM)),
        ]),
    ];

    // Show interface count
    lines.push(Line::from(vec![
        Span::styled("  Adapters       ", Style::default().fg(Color::White)),
        Span::styled(format!("{} network adapters detected", net.interfaces.len()), Style::default().fg(COLOR_DIM)),
    ]));

    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        "  Press 9 for driver details",
        Style::default().fg(COLOR_DIM),
    )));

    let status_panel = Paragraph::new(lines);
    frame.render_widget(status_panel, chunks[1]);

    // Sparkline
    let spark_data = app.net_down_history.as_u64_vec();
    let max_val = spark_data.iter().copied().max().unwrap_or(1).max(1);
    let sparkline = Sparkline::default()
        .block(Block::default()
            .title("  DOWNLOAD SPEED (last 60 seconds)")
            .title_style(Style::default().fg(COLOR_HEADER))
            .borders(Borders::NONE))
        .data(&spark_data)
        .max(max_val)
        .style(Style::default().fg(Color::Blue));
    frame.render_widget(sparkline, chunks[2]);
}

fn render_tech(frame: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(6),
            Constraint::Length(8),
        ])
        .split(area);

    let net = &app.snapshot.network;

    // Header
    let header_lines = vec![
        Line::from(Span::styled(
            format!("  NETWORK \u{2014} {} interfaces  \u{2193} {}  \u{2191} {}",
                net.interfaces.len(),
                format_throughput(net.total_download_rate),
                format_throughput(net.total_upload_rate)),
            Style::default().fg(COLOR_HEADER).add_modifier(Modifier::BOLD),
        )),
        separator(area.width as usize),
        Line::from(Span::styled(
            format!("  {:<20} {:<18} {:<18} {:>12} {:>12}", "INTERFACE", "MAC", "IP", "RX/s", "TX/s"),
            Style::default().fg(COLOR_DIM),
        )),
    ];
    let header_panel = Paragraph::new(header_lines);
    frame.render_widget(header_panel, chunks[0]);

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
            Style::default().fg(Color::White),
        )));
    }

    let iface_panel = Paragraph::new(iface_lines);
    frame.render_widget(iface_panel, chunks[1]);

    // Download/Upload sparklines
    let spark_area = chunks[2];
    let spark_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(spark_area);

    let down_data = app.net_down_history.as_u64_vec();
    let down_max = down_data.iter().copied().max().unwrap_or(1).max(1);
    let down_sparkline = Sparkline::default()
        .block(Block::default()
            .title(" Download ")
            .title_style(Style::default().fg(COLOR_HEADER))
            .borders(Borders::ALL)
            .border_style(Style::default().fg(COLOR_DIM)))
        .data(&down_data)
        .max(down_max)
        .style(Style::default().fg(Color::Blue));
    frame.render_widget(down_sparkline, spark_chunks[0]);

    let up_data = app.net_up_history.as_u64_vec();
    let up_max = up_data.iter().copied().max().unwrap_or(1).max(1);
    let up_sparkline = Sparkline::default()
        .block(Block::default()
            .title(" Upload ")
            .title_style(Style::default().fg(COLOR_HEADER))
            .borders(Borders::ALL)
            .border_style(Style::default().fg(COLOR_DIM)))
        .data(&up_data)
        .max(up_max)
        .style(Style::default().fg(Color::Magenta));
    frame.render_widget(up_sparkline, spark_chunks[1]);
}

fn truncate_str(s: &str, max: usize) -> String {
    if max < 3 { return s.chars().take(max).collect(); }
    if s.chars().count() <= max { s.to_string() } else {
        let truncated: String = s.chars().take(max - 2).collect();
        format!("{}..", truncated)
    }
}
