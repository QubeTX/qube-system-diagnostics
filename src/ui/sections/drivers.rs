use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

use crate::app::App;
use crate::collectors::drivers::{DeviceStatus, ServiceInfo};
use crate::types::{DiagnosticMode, HealthStatus};
use crate::ui::common::*;

pub fn render(frame: &mut Frame, app: &App, area: Rect, mode: DiagnosticMode) {
    match mode {
        DiagnosticMode::User => render_user(frame, app, area),
        DiagnosticMode::Technician => render_tech(frame, app, area),
    }
}

fn render_user(frame: &mut Frame, app: &App, area: Rect) {
    let drivers = &app.snapshot.drivers;

    let mut lines = vec![
        Line::from(Span::styled(
            "  DEVICE HEALTH",
            Style::default().fg(COLOR_HEADER).add_modifier(Modifier::BOLD),
        )),
        separator(area.width as usize),
        Line::from(""),
    ];

    // Network
    lines.push(section_header("NETWORK"));
    for dev in &drivers.network {
        let status = match &dev.status {
            DeviceStatus::Ok => HealthStatus::Good,
            DeviceStatus::Disabled => HealthStatus::Warning,
            DeviceStatus::Error(_) => HealthStatus::Critical,
            _ => HealthStatus::Unknown,
        };
        lines.push(status_line(&status, &dev.name, dev.status.user_description()));
    }
    if drivers.network.is_empty() {
        lines.push(Line::from(Span::styled("  No network adapters detected", Style::default().fg(COLOR_DIM))));
    }

    lines.push(Line::from(""));

    // Bluetooth
    lines.push(section_header("BLUETOOTH"));
    for dev in &drivers.bluetooth {
        let status = match &dev.status {
            DeviceStatus::Ok => HealthStatus::Good,
            DeviceStatus::NotFound => HealthStatus::Unknown,
            DeviceStatus::Error(_) => HealthStatus::Critical,
            _ => HealthStatus::Warning,
        };
        lines.push(status_line(&status, &dev.name, dev.status.user_description()));
    }

    lines.push(Line::from(""));

    // Audio
    lines.push(section_header("AUDIO"));
    for dev in &drivers.audio {
        let status = match &dev.status {
            DeviceStatus::Ok => HealthStatus::Good,
            DeviceStatus::NotFound => HealthStatus::Unknown,
            DeviceStatus::Error(_) => HealthStatus::Critical,
            _ => HealthStatus::Warning,
        };
        lines.push(status_line(&status, &dev.name, dev.status.user_description()));
    }

    lines.push(Line::from(""));

    // Input
    lines.push(section_header("KEYBOARD & MOUSE"));
    for dev in &drivers.input {
        let status = match &dev.status {
            DeviceStatus::Ok => HealthStatus::Good,
            DeviceStatus::Error(_) => HealthStatus::Critical,
            _ => HealthStatus::Unknown,
        };
        lines.push(status_line(&status, &dev.name, dev.status.user_description()));
    }

    let panel = Paragraph::new(lines);
    frame.render_widget(panel, area);
}

fn render_tech(frame: &mut Frame, app: &App, area: Rect) {
    let drivers = &app.snapshot.drivers;
    let os_name = &app.snapshot.system.os_name;

    let mut lines = vec![
        Line::from(Span::styled(
            format!("  DRIVERS & DEVICES \u{2014} {}    (press r to refresh)", os_name),
            Style::default().fg(COLOR_HEADER).add_modifier(Modifier::BOLD),
        )),
        separator(area.width as usize),
    ];

    // Network Adapters table
    lines.push(section_header("NETWORK ADAPTERS"));
    lines.push(Line::from(Span::styled(
        format!("  {:<30} {:<16} {:>8} {}", "DEVICE", "DRIVER VER", "STATUS", "DATE"),
        Style::default().fg(COLOR_DIM),
    )));

    for dev in &drivers.network {
        let status_icon = dev.status.icon();
        let status_color = match &dev.status {
            DeviceStatus::Ok => COLOR_GOOD,
            DeviceStatus::Disabled => COLOR_WARN,
            DeviceStatus::Error(_) => COLOR_CRIT,
            _ => COLOR_DIM,
        };

        lines.push(Line::from(vec![
            Span::styled(
                format!("  {:<30} {:<16} ", truncate_str(&dev.name, 30), truncate_str(&dev.driver_version, 16)),
                Style::default().fg(Color::White),
            ),
            Span::styled(format!("{} {:<6}", status_icon, dev.status), Style::default().fg(status_color)),
            Span::styled(format!(" {}", dev.driver_date), Style::default().fg(COLOR_DIM)),
        ]));
    }

    // Network services
    let net_services: Vec<&ServiceInfo> = drivers.services.iter()
        .filter(|s| ["Dhcp", "Dnscache", "WlanSvc", "NlaSvc", "NetworkManager", "wpa_supplicant"].contains(&s.name.as_str()))
        .collect();
    if !net_services.is_empty() {
        lines.push(render_service_line("Services", &net_services));
    }

    lines.push(Line::from(""));

    // Bluetooth
    lines.push(section_header("BLUETOOTH"));
    lines.push(Line::from(Span::styled(
        format!("  {:<30} {:<16} {:>8}", "DEVICE", "DRIVER VER", "STATUS"),
        Style::default().fg(COLOR_DIM),
    )));
    for dev in &drivers.bluetooth {
        let status_color = match &dev.status {
            DeviceStatus::Ok => COLOR_GOOD,
            DeviceStatus::NotFound => COLOR_DIM,
            _ => COLOR_WARN,
        };
        lines.push(Line::from(vec![
            Span::styled(
                format!("  {:<30} {:<16} ", truncate_str(&dev.name, 30), truncate_str(&dev.driver_version, 16)),
                Style::default().fg(Color::White),
            ),
            Span::styled(format!("{} {}", dev.status.icon(), dev.status), Style::default().fg(status_color)),
        ]));
    }

    let bt_services: Vec<&ServiceInfo> = drivers.services.iter()
        .filter(|s| ["bthserv", "BthAvctpSvc", "bluetooth", "com.apple.blued"].contains(&s.name.as_str()))
        .collect();
    if !bt_services.is_empty() {
        lines.push(render_service_line("Services", &bt_services));
    }

    lines.push(Line::from(""));

    // Audio
    lines.push(section_header("AUDIO"));
    lines.push(Line::from(Span::styled(
        format!("  {:<30} {:<16} {:>8} {}", "DEVICE", "DRIVER VER", "STATUS", "DATE"),
        Style::default().fg(COLOR_DIM),
    )));
    for dev in &drivers.audio {
        let status_color = match &dev.status {
            DeviceStatus::Ok => COLOR_GOOD,
            _ => COLOR_WARN,
        };
        lines.push(Line::from(vec![
            Span::styled(
                format!("  {:<30} {:<16} ", truncate_str(&dev.name, 30), truncate_str(&dev.driver_version, 16)),
                Style::default().fg(Color::White),
            ),
            Span::styled(format!("{} {:<6}", dev.status.icon(), dev.status), Style::default().fg(status_color)),
            Span::styled(format!(" {}", dev.driver_date), Style::default().fg(COLOR_DIM)),
        ]));
    }

    let audio_services: Vec<&ServiceInfo> = drivers.services.iter()
        .filter(|s| ["Audiosrv", "AudioEndpointBuilder", "pipewire", "pulseaudio", "com.apple.audio.coreaudiod"].contains(&s.name.as_str()))
        .collect();
    if !audio_services.is_empty() {
        lines.push(render_service_line("Services", &audio_services));
    }

    lines.push(Line::from(""));

    // Input
    lines.push(section_header("INPUT DEVICES"));
    lines.push(Line::from(Span::styled(
        format!("  {:<30} {:<16} {:>8}", "DEVICE", "DRIVER VER", "STATUS"),
        Style::default().fg(COLOR_DIM),
    )));
    for dev in &drivers.input {
        let status_color = match &dev.status {
            DeviceStatus::Ok => COLOR_GOOD,
            _ => COLOR_WARN,
        };
        lines.push(Line::from(vec![
            Span::styled(
                format!("  {:<30} {:<16} ", truncate_str(&dev.name, 30), truncate_str(&dev.driver_version, 16)),
                Style::default().fg(Color::White),
            ),
            Span::styled(format!("{} {}", dev.status.icon(), dev.status), Style::default().fg(status_color)),
        ]));
    }

    let input_services: Vec<&ServiceInfo> = drivers.services.iter()
        .filter(|s| ["hidserv"].contains(&s.name.as_str()))
        .collect();
    if !input_services.is_empty() {
        lines.push(render_service_line("Services", &input_services));
    }

    let panel = Paragraph::new(lines);
    frame.render_widget(panel, area);
}

fn render_service_line<'a>(label: &str, services: &[&ServiceInfo]) -> Line<'a> {
    let mut spans = vec![
        Span::styled(format!("  {}: ", label), Style::default().fg(COLOR_DIM)),
    ];

    for (i, svc) in services.iter().enumerate() {
        let icon = if svc.is_running { "\u{2713}" } else { "\u{2717}" };
        let color = if svc.is_running { COLOR_GOOD } else { COLOR_CRIT };
        let name = if svc.display_name.is_empty() {
            &svc.name
        } else {
            &svc.display_name
        };

        spans.push(Span::styled(
            format!("{} {}", name, icon),
            Style::default().fg(color),
        ));

        if i < services.len() - 1 {
            spans.push(Span::styled("  ", Style::default()));
        }
    }

    Line::from(spans)
}

fn truncate_str(s: &str, max: usize) -> String {
    if max < 3 { return s.chars().take(max).collect(); }
    if s.chars().count() <= max { s.to_string() } else {
        let truncated: String = s.chars().take(max - 2).collect();
        format!("{}..", truncated)
    }
}
