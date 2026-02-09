use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

use crate::app::App;
use crate::collectors::drivers::{DeviceStatus, DriverScanStatus, ServiceInfo};
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

    let outer = content_block("Device Health");
    let inner = outer.inner(area);
    frame.render_widget(outer, area);

    let mut lines = vec![Line::from("")];

    // Scan status
    match &drivers.scan_status {
        DriverScanStatus::Scanning => {
            lines.push(Line::from(Span::styled(
                "  \u{27F3} Scanning devices...",
                Style::default().fg(COLOR_ACCENT),
            )));
            lines.push(Line::from(""));
        }
        DriverScanStatus::ScanFailed(ref msg) => {
            lines.push(Line::from(Span::styled(
                format!("  \u{26A0} {}", msg),
                Style::default().fg(COLOR_WARN),
            )));
            lines.push(Line::from(""));
            lines.push(Line::from(Span::styled(
                "  Device information requires administrator privileges or may be temporarily unavailable.",
                Style::default().fg(COLOR_MUTED),
            )));
            lines.push(Line::from(Span::styled(
                "  Try running SD-300 as Administrator.",
                Style::default().fg(COLOR_MUTED),
            )));
            let panel = Paragraph::new(lines);
            frame.render_widget(panel, inner);
            return;
        }
        DriverScanStatus::NotScanned => {
            lines.push(Line::from(Span::styled(
                "  Waiting for scan...",
                Style::default().fg(COLOR_MUTED),
            )));
            lines.push(Line::from(""));
        }
        DriverScanStatus::Success => {}
    }

    // Categories
    render_user_category(&mut lines, "NETWORK", &drivers.network);
    render_user_category(&mut lines, "BLUETOOTH", &drivers.bluetooth);
    render_user_category(&mut lines, "AUDIO", &drivers.audio);
    render_user_category(&mut lines, "KEYBOARD & MOUSE", &drivers.input);
    render_user_category(&mut lines, "DISPLAY", &drivers.display);
    render_user_category(&mut lines, "STORAGE", &drivers.storage);
    render_user_category(&mut lines, "USB", &drivers.usb);

    let panel = Paragraph::new(lines);
    frame.render_widget(panel, inner);
}

fn render_user_category(lines: &mut Vec<Line<'_>>, title: &str, devices: &[crate::collectors::drivers::DeviceInfo]) {
    if devices.is_empty() {
        return;
    }
    lines.push(section_header(title));
    for dev in devices {
        let status = match &dev.status {
            DeviceStatus::Ok => HealthStatus::Good,
            DeviceStatus::Disabled => HealthStatus::Warning,
            DeviceStatus::Error(_) => HealthStatus::Critical,
            DeviceStatus::NotFound => HealthStatus::Unknown,
            DeviceStatus::Unknown => HealthStatus::Unknown,
        };
        lines.push(status_line(&status, &dev.name, dev.status.user_description()));
    }
    lines.push(Line::from(""));
}

fn render_tech(frame: &mut Frame, app: &App, area: Rect) {
    let drivers = &app.snapshot.drivers;
    let os_name = &app.snapshot.system.os_name;

    let outer = content_block(&format!("Drivers & Devices \u{2014} {}    (press r to refresh)", os_name));
    let inner = outer.inner(area);
    frame.render_widget(outer, area);

    let mut lines = Vec::new();

    // Scan status
    match &drivers.scan_status {
        DriverScanStatus::Scanning => {
            lines.push(Line::from(Span::styled(
                "  \u{27F3} Scanning devices...",
                Style::default().fg(COLOR_ACCENT),
            )));
            lines.push(Line::from(""));
        }
        DriverScanStatus::ScanFailed(msg) => {
            lines.push(Line::from(Span::styled(
                format!("  \u{26A0} {}", msg),
                Style::default().fg(COLOR_WARN),
            )));
            lines.push(Line::from(""));
            lines.push(Line::from(Span::styled(
                "  Device information requires administrator privileges or may be temporarily unavailable.",
                Style::default().fg(COLOR_MUTED),
            )));
            lines.push(Line::from(Span::styled(
                "  Try running SD-300 as Administrator.",
                Style::default().fg(COLOR_MUTED),
            )));
            let panel = Paragraph::new(lines);
            frame.render_widget(panel, inner);
            return;
        }
        DriverScanStatus::NotScanned => {
            lines.push(Line::from(Span::styled(
                "  Scan status: Not scanned yet",
                Style::default().fg(COLOR_MUTED),
            )));
            lines.push(Line::from(""));
        }
        DriverScanStatus::Success => {}
    }

    // Network Adapters table
    render_tech_category(&mut lines, "NETWORK ADAPTERS", &drivers.network, true);

    let net_services: Vec<&ServiceInfo> = drivers.services.iter()
        .filter(|s| ["Dhcp", "Dnscache", "WlanSvc", "NlaSvc", "NetworkManager", "wpa_supplicant"].contains(&s.name.as_str()))
        .collect();
    if !net_services.is_empty() {
        lines.push(render_service_line("Services", &net_services));
    }
    lines.push(Line::from(""));

    // Display
    render_tech_category(&mut lines, "DISPLAY", &drivers.display, true);
    let display_services: Vec<&ServiceInfo> = drivers.services.iter()
        .filter(|s| ["DisplayEnhancementService"].contains(&s.name.as_str()))
        .collect();
    if !display_services.is_empty() {
        lines.push(render_service_line("Services", &display_services));
    }
    lines.push(Line::from(""));

    // Storage
    render_tech_category(&mut lines, "STORAGE", &drivers.storage, true);
    let storage_services: Vec<&ServiceInfo> = drivers.services.iter()
        .filter(|s| ["StorSvc", "VSS"].contains(&s.name.as_str()))
        .collect();
    if !storage_services.is_empty() {
        lines.push(render_service_line("Services", &storage_services));
    }
    lines.push(Line::from(""));

    // Bluetooth
    render_tech_category(&mut lines, "BLUETOOTH", &drivers.bluetooth, false);
    let bt_services: Vec<&ServiceInfo> = drivers.services.iter()
        .filter(|s| ["bthserv", "BthAvctpSvc", "bluetooth", "com.apple.blued"].contains(&s.name.as_str()))
        .collect();
    if !bt_services.is_empty() {
        lines.push(render_service_line("Services", &bt_services));
    }
    lines.push(Line::from(""));

    // Audio
    render_tech_category(&mut lines, "AUDIO", &drivers.audio, true);
    let audio_services: Vec<&ServiceInfo> = drivers.services.iter()
        .filter(|s| ["Audiosrv", "AudioEndpointBuilder", "pipewire", "pulseaudio", "com.apple.audio.coreaudiod"].contains(&s.name.as_str()))
        .collect();
    if !audio_services.is_empty() {
        lines.push(render_service_line("Services", &audio_services));
    }
    lines.push(Line::from(""));

    // Input
    render_tech_category(&mut lines, "INPUT DEVICES", &drivers.input, false);
    let input_services: Vec<&ServiceInfo> = drivers.services.iter()
        .filter(|s| ["hidserv"].contains(&s.name.as_str()))
        .collect();
    if !input_services.is_empty() {
        lines.push(render_service_line("Services", &input_services));
    }
    lines.push(Line::from(""));

    // USB
    render_tech_category(&mut lines, "USB", &drivers.usb, false);
    let usb_services: Vec<&ServiceInfo> = drivers.services.iter()
        .filter(|s| ["USBHUB3"].contains(&s.name.as_str()))
        .collect();
    if !usb_services.is_empty() {
        lines.push(render_service_line("Services", &usb_services));
    }
    lines.push(Line::from(""));

    // System
    if !drivers.system.is_empty() {
        render_tech_category(&mut lines, "SYSTEM", &drivers.system, false);
        lines.push(Line::from(""));
    }

    // Other (limit to 20 with "+ N more")
    if !drivers.other.is_empty() {
        lines.push(section_header("OTHER DEVICES"));
        lines.push(Line::from(Span::styled(
            format!("  {:<30} {:<16} {:>8}", "DEVICE", "DRIVER VER", "STATUS"),
            Style::default().fg(COLOR_DIM),
        )));

        let show_count = drivers.other.len().min(20);
        for dev in drivers.other.iter().take(show_count) {
            let status_color = match &dev.status {
                DeviceStatus::Ok => COLOR_GOOD,
                DeviceStatus::Error(_) => COLOR_CRIT,
                _ => COLOR_DIM,
            };
            lines.push(Line::from(vec![
                Span::styled(
                    format!("  {:<30} {:<16} ", truncate_str(&dev.name, 30), truncate_str(&dev.driver_version, 16)),
                    Style::default().fg(COLOR_TEXT),
                ),
                Span::styled(format!("{} {}", dev.status.icon(), dev.status), Style::default().fg(status_color)),
            ]));
        }
        if drivers.other.len() > 20 {
            lines.push(Line::from(Span::styled(
                format!("  + {} more devices", drivers.other.len() - 20),
                Style::default().fg(COLOR_DIM),
            )));
        }
        lines.push(Line::from(""));
    }

    let panel = Paragraph::new(lines);
    frame.render_widget(panel, inner);
}

fn render_tech_category(lines: &mut Vec<Line<'_>>, title: &str, devices: &[crate::collectors::drivers::DeviceInfo], show_date: bool) {
    if devices.is_empty() {
        return;
    }
    lines.push(section_header(title));
    if show_date {
        lines.push(Line::from(Span::styled(
            format!("  {:<30} {:<16} {:>8} {}", "DEVICE", "DRIVER VER", "STATUS", "DATE"),
            Style::default().fg(COLOR_DIM),
        )));
    } else {
        lines.push(Line::from(Span::styled(
            format!("  {:<30} {:<16} {:>8}", "DEVICE", "DRIVER VER", "STATUS"),
            Style::default().fg(COLOR_DIM),
        )));
    }

    for dev in devices {
        let status_color = match &dev.status {
            DeviceStatus::Ok => COLOR_GOOD,
            DeviceStatus::Disabled => COLOR_WARN,
            DeviceStatus::Error(_) => COLOR_CRIT,
            _ => COLOR_DIM,
        };

        if show_date {
            lines.push(Line::from(vec![
                Span::styled(
                    format!("  {:<30} {:<16} ", truncate_str(&dev.name, 30), truncate_str(&dev.driver_version, 16)),
                    Style::default().fg(COLOR_TEXT),
                ),
                Span::styled(format!("{} {:<6}", dev.status.icon(), dev.status), Style::default().fg(status_color)),
                Span::styled(format!(" {}", dev.driver_date), Style::default().fg(COLOR_DIM)),
            ]));
        } else {
            lines.push(Line::from(vec![
                Span::styled(
                    format!("  {:<30} {:<16} ", truncate_str(&dev.name, 30), truncate_str(&dev.driver_version, 16)),
                    Style::default().fg(COLOR_TEXT),
                ),
                Span::styled(format!("{} {}", dev.status.icon(), dev.status), Style::default().fg(status_color)),
            ]));
        }
    }
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
