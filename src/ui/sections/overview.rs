use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
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
            Constraint::Length(2),  // Header
            Constraint::Length(10), // System Health
            Constraint::Length(7),  // Top Resources
            Constraint::Min(4),    // Your Computer
        ])
        .split(area);

    // Header
    let header = Paragraph::new(Line::from(vec![
        Span::styled(
            "  SD-300 SYSTEM DIAGNOSTIC",
            Style::default().fg(COLOR_ACCENT).add_modifier(Modifier::BOLD),
        ),
        Span::styled(" \u{2014} User Mode", Style::default().fg(COLOR_DIM)),
    ]));
    frame.render_widget(header, chunks[0]);

    // System Health panel
    let cpu_pct = app.snapshot.cpu.total_usage;
    let mem_pct = app.snapshot.memory.usage_percent();
    let disk_pct = app.snapshot.disk.partitions.first().map(|d| d.usage_percent()).unwrap_or(0.0);
    let gpu_pct = app.snapshot.gpu.utilization_percent;

    let cpu_status = HealthStatus::from_percent(cpu_pct as f64);
    let mem_status = HealthStatus::from_percent(mem_pct);
    let disk_status = HealthStatus::from_percent(disk_pct);
    let gpu_status = HealthStatus::from_percent(gpu_pct as f64);

    let net_connected = !app.snapshot.network.interfaces.is_empty();
    let net_status = if net_connected { HealthStatus::Good } else { HealthStatus::Warning };
    let temp_status = app.snapshot.thermals.cpu_temp.map(|t| {
        if t < 70.0 { HealthStatus::Good }
        else if t < 85.0 { HealthStatus::Warning }
        else { HealthStatus::Critical }
    }).unwrap_or(HealthStatus::Unknown);

    let driver_issues = app.snapshot.drivers.network.iter()
        .chain(app.snapshot.drivers.bluetooth.iter())
        .chain(app.snapshot.drivers.audio.iter())
        .chain(app.snapshot.drivers.input.iter())
        .filter(|d| d.status != crate::collectors::drivers::DeviceStatus::Ok)
        .count();
    let driver_status = if driver_issues == 0 { HealthStatus::Good } else { HealthStatus::Warning };

    let mut health_lines = vec![
        section_header("SYSTEM HEALTH"),
        separator(area.width as usize),
        Line::from(""),
    ];

    let bar_width = 10;
    health_lines.push(health_gauge_line("Processor", &cpu_status, plain_language_cpu(cpu_pct), cpu_pct as f64, bar_width));
    health_lines.push(health_gauge_line("Memory", &mem_status, &plain_language_percent(mem_pct, "memory"), mem_pct, bar_width));
    health_lines.push(health_gauge_line("Storage", &disk_status, &plain_language_percent(disk_pct, "storage"), disk_pct, bar_width));

    if app.snapshot.gpu.available {
        health_lines.push(health_gauge_line("Graphics", &gpu_status, plain_language_cpu(gpu_pct), gpu_pct as f64, bar_width));
    } else {
        health_lines.push(status_line(&HealthStatus::Good, "Graphics", "Integrated (no detailed data)"));
    }

    let net_desc = if net_connected { "Connected" } else { "Disconnected" };
    health_lines.push(status_line(&net_status, "Network", net_desc));

    let temp_desc = app.snapshot.thermals.cpu_temp
        .map(|t| format!("{} ({})", plain_language_temp(t), format_temp(t, app.temp_unit)))
        .unwrap_or_else(|| "Sensor data unavailable".into());
    health_lines.push(status_line(&temp_status, "Temperature", &temp_desc));

    let driver_desc = if driver_issues == 0 {
        "All devices working".to_string()
    } else {
        format!("{} device(s) need attention", driver_issues)
    };
    health_lines.push(status_line(&driver_status, "Drivers", &driver_desc));

    let health_panel = Paragraph::new(health_lines);
    frame.render_widget(health_panel, chunks[1]);

    // Top Resource Consumers
    let mut resource_lines = vec![
        separator(area.width as usize),
        section_header("WHAT'S USING THE MOST RESOURCES"),
        Line::from(""),
    ];

    for proc in app.snapshot.processes.list.iter().take(3) {
        let bar = gauge_bar(proc.cpu_percent as f64, 6);
        resource_lines.push(Line::from(vec![
            Span::styled(format!("  {:<24}", proc.friendly_name), Style::default().fg(Color::White)),
            Span::styled(bar, Style::default().fg(COLOR_INFO)),
        ]));
    }

    let resource_panel = Paragraph::new(resource_lines);
    frame.render_widget(resource_panel, chunks[2]);

    // Your Computer
    let sys = &app.snapshot.system;
    let total_mem_gb = sys.total_memory_bytes as f64 / (1024.0 * 1024.0 * 1024.0);
    let disk_total = app.snapshot.disk.partitions.first().map(|d| format_bytes(d.total_bytes)).unwrap_or_else(|| "Unknown".into());

    let computer_lines = vec![
        separator(area.width as usize),
        section_header("YOUR COMPUTER"),
        Line::from(""),
        Line::from(Span::styled(
            format!(
                "  {} {}  \u{2022}  {}  \u{2022}  Up {}",
                sys.os_name, sys.os_version, sys.hostname, format_uptime(sys.uptime_seconds)
            ),
            Style::default().fg(Color::White),
        )),
        Line::from(Span::styled(
            format!(
                "  {}  \u{2022}  {:.0} GB RAM  \u{2022}  {}",
                sys.cpu_model, total_mem_gb, disk_total
            ),
            Style::default().fg(COLOR_DIM),
        )),
    ];

    let computer_panel = Paragraph::new(computer_lines);
    frame.render_widget(computer_panel, chunks[3]);
}

fn render_tech(frame: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(2),  // Header
            Constraint::Length(7),  // System identity
            Constraint::Length(3),  // Gauges
            Constraint::Length(4),  // Disk + Network
            Constraint::Min(6),    // Processes + Summary
        ])
        .split(area);

    // Header with timestamp
    let now = chrono_free_time();
    let header = Paragraph::new(Line::from(vec![
        Span::styled(
            "  SD-300 SYSTEM DIAGNOSTIC",
            Style::default().fg(COLOR_ACCENT).add_modifier(Modifier::BOLD),
        ),
        Span::styled(" \u{2014} Technician Mode", Style::default().fg(COLOR_DIM)),
        Span::raw(format!("{:>width$}", now, width = area.width as usize - 46)),
    ]));
    frame.render_widget(header, chunks[0]);

    // System identity
    let sys = &app.snapshot.system;
    let gpu_name = if app.snapshot.gpu.available {
        &app.snapshot.gpu.name
    } else {
        "N/A"
    };
    let gpu_driver = if app.snapshot.gpu.available {
        &app.snapshot.gpu.driver_version
    } else {
        ""
    };
    let mem_used_gib = format_bytes_gib(app.snapshot.memory.used_bytes);
    let mem_total_gib = format_bytes_gib(app.snapshot.memory.total_bytes);
    let swap_used_gib = format_bytes_gib(app.snapshot.memory.swap_used_bytes);
    let swap_total_gib = format_bytes_gib(app.snapshot.memory.swap_total_bytes);

    let identity_lines = vec![
        separator(area.width as usize),
        Line::from(vec![
            Span::styled("  OS     ", Style::default().fg(COLOR_DIM)),
            Span::styled(format!("{} {} ({})", sys.os_name, sys.os_version, sys.kernel_version), Style::default().fg(Color::White)),
            Span::styled(format!("{}Uptime  ", " ".repeat(4)), Style::default().fg(COLOR_DIM)),
            Span::styled(format_uptime(sys.uptime_seconds), Style::default().fg(Color::White)),
        ]),
        Line::from(vec![
            Span::styled("  Host   ", Style::default().fg(COLOR_DIM)),
            Span::styled(&sys.hostname, Style::default().fg(Color::White)),
        ]),
        Line::from(vec![
            Span::styled("  CPU    ", Style::default().fg(COLOR_DIM)),
            Span::styled(
                format!("{} ({}) @ {} MHz", sys.cpu_model, sys.cpu_threads, app.snapshot.cpu.per_core_frequency.first().unwrap_or(&0)),
                Style::default().fg(Color::White),
            ),
            Span::styled(format!("{}Arch    ", " ".repeat(4)), Style::default().fg(COLOR_DIM)),
            Span::styled(&sys.architecture, Style::default().fg(Color::White)),
        ]),
        Line::from(vec![
            Span::styled("  GPU    ", Style::default().fg(COLOR_DIM)),
            Span::styled(format!("{}", gpu_name), Style::default().fg(Color::White)),
            Span::styled(format!("{}Driver  ", " ".repeat(4)), Style::default().fg(COLOR_DIM)),
            Span::styled(gpu_driver.to_string(), Style::default().fg(Color::White)),
        ]),
        Line::from(vec![
            Span::styled("  Memory ", Style::default().fg(COLOR_DIM)),
            Span::styled(
                format!("{} / {} ({:.1}%)", mem_used_gib, mem_total_gib, app.snapshot.memory.usage_percent()),
                Style::default().fg(Color::White),
            ),
            Span::styled(format!("{}Swap    ", " ".repeat(4)), Style::default().fg(COLOR_DIM)),
            Span::styled(format!("{} / {}", swap_used_gib, swap_total_gib), Style::default().fg(Color::White)),
        ]),
    ];
    let identity_panel = Paragraph::new(identity_lines);
    frame.render_widget(identity_panel, chunks[1]);

    // Gauges row
    let gauge_lines = vec![
        separator(area.width as usize),
        Line::from(vec![
            Span::styled("  CPU ", Style::default().fg(COLOR_DIM)),
            Span::styled(
                gauge_bar(app.snapshot.cpu.total_usage as f64, 20),
                Style::default().fg(status_color(&HealthStatus::from_percent(app.snapshot.cpu.total_usage as f64))),
            ),
            Span::raw("   "),
            Span::styled("MEM ", Style::default().fg(COLOR_DIM)),
            Span::styled(
                gauge_bar(app.snapshot.memory.usage_percent(), 20),
                Style::default().fg(status_color(&HealthStatus::from_percent(app.snapshot.memory.usage_percent()))),
            ),
        ]),
        Line::from(vec![
            Span::styled("  GPU ", Style::default().fg(COLOR_DIM)),
            Span::styled(
                gauge_bar(app.snapshot.gpu.utilization_percent as f64, 20),
                Style::default().fg(status_color(&HealthStatus::from_percent(app.snapshot.gpu.utilization_percent as f64))),
            ),
            Span::raw("   "),
            Span::styled("SWP ", Style::default().fg(COLOR_DIM)),
            Span::styled(
                gauge_bar(app.snapshot.memory.swap_percent(), 20),
                Style::default().fg(status_color(&HealthStatus::from_percent(app.snapshot.memory.swap_percent()))),
            ),
        ]),
    ];
    let gauge_panel = Paragraph::new(gauge_lines);
    frame.render_widget(gauge_panel, chunks[2]);

    // Disk + Network summary
    let mut disk_net_lines = vec![separator(area.width as usize)];

    for part in app.snapshot.disk.partitions.iter().take(3) {
        disk_net_lines.push(Line::from(vec![
            Span::styled(
                format!("  DSK {}  ", part.mount_point),
                Style::default().fg(COLOR_DIM),
            ),
            Span::styled(
                gauge_bar(part.usage_percent(), 20),
                Style::default().fg(status_color(&HealthStatus::from_percent(part.usage_percent()))),
            ),
            Span::styled(
                format!("  {}/{}", format_bytes(part.used_bytes), format_bytes(part.total_bytes)),
                Style::default().fg(Color::White),
            ),
        ]));
    }

    if !app.snapshot.network.interfaces.is_empty() {
        disk_net_lines.push(Line::from(vec![
            Span::styled("  NET    ", Style::default().fg(COLOR_DIM)),
            Span::styled(
                format!("\u{2193} {}  \u{2191} {}",
                    format_throughput(app.snapshot.network.total_download_rate),
                    format_throughput(app.snapshot.network.total_upload_rate)
                ),
                Style::default().fg(Color::White),
            ),
        ]));
    }

    let disk_net_panel = Paragraph::new(disk_net_lines);
    frame.render_widget(disk_net_panel, chunks[3]);

    // Top processes + summary
    let mut proc_lines = vec![
        separator(area.width as usize),
        Line::from(vec![
            Span::styled(
                format!("  {:<28} {:>6} {:>8} {:>8} {:>10}", "TOP PROCESSES", "PID", "CPU%", "MEM%", "MEM"),
                Style::default().fg(COLOR_DIM),
            ),
        ]),
    ];

    for proc in app.snapshot.processes.list.iter().take(5) {
        proc_lines.push(Line::from(Span::styled(
            format!(
                "  {:<28} {:>6} {:>7.1}% {:>7.1}% {:>10}",
                truncate_str(&proc.name, 28),
                proc.pid,
                proc.cpu_percent,
                proc.memory_percent,
                format_bytes(proc.memory_bytes)
            ),
            Style::default().fg(Color::White),
        )));
    }

    // Driver + Thermal summary
    proc_lines.push(Line::from(""));
    let driver_ok = app.snapshot.drivers.network.iter()
        .chain(app.snapshot.drivers.bluetooth.iter())
        .chain(app.snapshot.drivers.audio.iter())
        .chain(app.snapshot.drivers.input.iter())
        .all(|d| d.status == crate::collectors::drivers::DeviceStatus::Ok);

    let temp_str = app.snapshot.thermals.cpu_temp
        .map(|t| format!("CPU: {}", format_temp(t, app.temp_unit)))
        .unwrap_or_else(|| "CPU: N/A".into());

    proc_lines.push(Line::from(vec![
        Span::styled("  DRVRS  ", Style::default().fg(COLOR_DIM)),
        Span::styled(
            if driver_ok { "All OK" } else { "Issues found" },
            Style::default().fg(if driver_ok { COLOR_GOOD } else { COLOR_WARN }),
        ),
        Span::styled("    TEMPS  ", Style::default().fg(COLOR_DIM)),
        Span::styled(temp_str, Style::default().fg(Color::White)),
    ]));

    let proc_panel = Paragraph::new(proc_lines);
    frame.render_widget(proc_panel, chunks[4]);
}

fn health_gauge_line<'a>(label: &str, status: &HealthStatus, description: &str, percent: f64, bar_width: usize) -> Line<'a> {
    let color = status_color(status);
    let filled = ((percent / 100.0) * bar_width as f64).round() as usize;
    let empty = bar_width.saturating_sub(filled);

    Line::from(vec![
        Span::styled(format!("  {} ", status.icon()), Style::default().fg(color)),
        Span::styled(format!("{:<16}", label), Style::default().fg(Color::White)),
        Span::styled(format!("{:<28}", description), Style::default().fg(COLOR_DIM)),
        Span::styled(
            format!("[{}{}] {:.0}%", "\u{2588}".repeat(filled), "\u{2591}".repeat(empty), percent),
            Style::default().fg(color),
        ),
    ])
}

/// Get current time as HH:MM:SS (without chrono dependency)
fn chrono_free_time() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    // Rough local time — not timezone-aware without chrono
    let secs_of_day = secs % 86400;
    let h = secs_of_day / 3600;
    let m = (secs_of_day % 3600) / 60;
    let s = secs_of_day % 60;
    format!("{:02}:{:02}:{:02} UTC", h, m, s)
}

fn truncate_str(s: &str, max: usize) -> String {
    if max < 3 {
        return s.chars().take(max).collect();
    }
    if s.chars().count() <= max {
        s.to_string()
    } else {
        let truncated: String = s.chars().take(max - 2).collect();
        format!("{}..", truncated)
    }
}
