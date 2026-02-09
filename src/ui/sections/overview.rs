use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::Style;
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
    // Outer content block
    let outer = content_block("System Health");
    let inner = outer.inner(area);
    frame.render_widget(outer, area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(8),  // System Health gauges
            Constraint::Length(8),  // Top Resources
            Constraint::Min(3),    // Your Computer
        ])
        .split(inner);

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
        if t < TEMP_CPU_WARN { HealthStatus::Good }
        else if t < TEMP_CPU_CRIT { HealthStatus::Warning }
        else { HealthStatus::Critical }
    }).unwrap_or(HealthStatus::Unknown);

    let driver_issues = app.snapshot.drivers.network.iter()
        .chain(app.snapshot.drivers.bluetooth.iter())
        .chain(app.snapshot.drivers.audio.iter())
        .chain(app.snapshot.drivers.input.iter())
        .chain(app.snapshot.drivers.display.iter())
        .chain(app.snapshot.drivers.storage.iter())
        .chain(app.snapshot.drivers.usb.iter())
        .chain(app.snapshot.drivers.system.iter())
        .chain(app.snapshot.drivers.other.iter())
        .filter(|d| d.status != crate::collectors::drivers::DeviceStatus::Ok)
        .count();
    let driver_status = if driver_issues == 0 { HealthStatus::Good } else { HealthStatus::Warning };

    let bar_width = 20;
    let mut health_lines = vec![
        Line::from(""),
    ];

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
    frame.render_widget(health_panel, chunks[0]);

    // Top Resource Consumers
    let resource_block = sub_block("Top Resource Consumers");
    let resource_inner = resource_block.inner(chunks[1]);
    frame.render_widget(resource_block, chunks[1]);

    let mut resource_lines = vec![Line::from("")];
    for proc in app.snapshot.processes.list.iter().take(5) {
        let bar = gauge_bar(proc.cpu_percent as f64, 20);
        resource_lines.push(Line::from(vec![
            Span::styled(format!("  {:<24}", proc.friendly_name), Style::default().fg(COLOR_TEXT)),
            Span::styled(bar, Style::default().fg(COLOR_INFO)),
        ]));
    }

    let resource_panel = Paragraph::new(resource_lines);
    frame.render_widget(resource_panel, resource_inner);

    // Your Computer
    let computer_block = sub_block("Your Computer");
    let computer_inner = computer_block.inner(chunks[2]);
    frame.render_widget(computer_block, chunks[2]);

    let sys = &app.snapshot.system;
    let total_mem_gb = sys.total_memory_bytes as f64 / (1024.0 * 1024.0 * 1024.0);
    let disk_total = app.snapshot.disk.partitions.first().map(|d| format_bytes(d.total_bytes)).unwrap_or_else(|| "Unknown".into());

    let computer_lines = vec![
        Line::from(Span::styled(
            format!(
                "  {} {}  \u{2022}  {}  \u{2022}  Up {}",
                sys.os_name, sys.os_version, sys.hostname, format_uptime(sys.uptime_seconds)
            ),
            Style::default().fg(COLOR_TEXT),
        )),
        Line::from(Span::styled(
            format!(
                "  {}  \u{2022}  {:.0} GB RAM  \u{2022}  {}",
                sys.cpu_model, total_mem_gb, disk_total
            ),
            Style::default().fg(COLOR_MUTED),
        )),
    ];

    let computer_panel = Paragraph::new(computer_lines);
    frame.render_widget(computer_panel, computer_inner);
}

fn render_tech(frame: &mut Frame, app: &App, area: Rect) {
    let outer = content_block("System Overview \u{2014} Technician");
    let inner = outer.inner(area);
    frame.render_widget(outer, area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(6),  // System identity
            Constraint::Length(3),  // Gauges
            Constraint::Length(4),  // Disk + Network
            Constraint::Min(6),    // Processes + Summary
        ])
        .split(inner);

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

    let freq = app.snapshot.cpu.per_core_frequency.first().unwrap_or(&0);

    // Dynamic column width: left_label(10) + left_value(val_w) + right_label(8) + right_value(~20)
    // Reserve 10 (label) + 8 (right label) + 20 (right value) = 38 chars for non-value columns
    let val_w = (inner.width as usize).saturating_sub(38).max(20).min(60);

    let os_val = truncate_str(&format!("{} {} ({})", sys.os_name, sys.os_version, sys.kernel_version), val_w);
    let cpu_val = truncate_str(&format!("{} ({}) @ {} MHz", sys.cpu_model, sys.cpu_threads, freq), val_w);
    let gpu_val = truncate_str(gpu_name, val_w);
    let mem_val = truncate_str(&format!("{} / {} ({:.1}%)", mem_used_gib, mem_total_gib, app.snapshot.memory.usage_percent()), val_w);

    let identity_lines = vec![
        Line::from(vec![
            Span::styled("  OS      ", Style::default().fg(COLOR_DIM)),
            Span::styled(format!("{:<width$}", os_val, width = val_w), Style::default().fg(COLOR_TEXT)),
            Span::styled("Uptime  ", Style::default().fg(COLOR_DIM)),
            Span::styled(format_uptime(sys.uptime_seconds), Style::default().fg(COLOR_TEXT)),
        ]),
        Line::from(vec![
            Span::styled("  Host    ", Style::default().fg(COLOR_DIM)),
            Span::styled(format!("{:<width$}", truncate_str(&sys.hostname, val_w), width = val_w), Style::default().fg(COLOR_TEXT)),
        ]),
        Line::from(vec![
            Span::styled("  CPU     ", Style::default().fg(COLOR_DIM)),
            Span::styled(format!("{:<width$}", cpu_val, width = val_w), Style::default().fg(COLOR_TEXT)),
            Span::styled("Arch    ", Style::default().fg(COLOR_DIM)),
            Span::styled(&sys.architecture, Style::default().fg(COLOR_TEXT)),
        ]),
        Line::from(vec![
            Span::styled("  GPU     ", Style::default().fg(COLOR_DIM)),
            Span::styled(format!("{:<width$}", gpu_val, width = val_w), Style::default().fg(COLOR_TEXT)),
            Span::styled("Driver  ", Style::default().fg(COLOR_DIM)),
            Span::styled(gpu_driver.to_string(), Style::default().fg(COLOR_TEXT)),
        ]),
        Line::from(vec![
            Span::styled("  Memory  ", Style::default().fg(COLOR_DIM)),
            Span::styled(format!("{:<width$}", mem_val, width = val_w), Style::default().fg(COLOR_TEXT)),
            Span::styled("Swap    ", Style::default().fg(COLOR_DIM)),
            Span::styled(format!("{} / {}", swap_used_gib, swap_total_gib), Style::default().fg(COLOR_TEXT)),
        ]),
    ];
    let identity_panel = Paragraph::new(identity_lines);
    frame.render_widget(identity_panel, chunks[0]);

    // Gauges row
    let gauge_lines = vec![
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
    frame.render_widget(gauge_panel, chunks[1]);

    // Disk + Network summary
    let mut disk_net_lines = Vec::new();

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
                Style::default().fg(COLOR_TEXT),
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
                Style::default().fg(COLOR_TEXT),
            ),
        ]));
    }

    let disk_net_panel = Paragraph::new(disk_net_lines);
    frame.render_widget(disk_net_panel, chunks[2]);

    // Top processes + summary
    let proc_block = sub_block("Top Processes");
    let proc_inner = proc_block.inner(chunks[3]);
    frame.render_widget(proc_block, chunks[3]);

    let mut proc_lines = vec![
        Line::from(vec![
            Span::styled(
                format!("  {:<28} {:>6} {:>8} {:>8} {:>10}", "PROCESS", "PID", "CPU%", "MEM%", "MEM"),
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
            Style::default().fg(COLOR_TEXT),
        )));
    }

    // Driver + Thermal summary
    proc_lines.push(Line::from(""));
    let driver_ok = app.snapshot.drivers.network.iter()
        .chain(app.snapshot.drivers.bluetooth.iter())
        .chain(app.snapshot.drivers.audio.iter())
        .chain(app.snapshot.drivers.input.iter())
        .chain(app.snapshot.drivers.display.iter())
        .chain(app.snapshot.drivers.storage.iter())
        .chain(app.snapshot.drivers.usb.iter())
        .chain(app.snapshot.drivers.system.iter())
        .chain(app.snapshot.drivers.other.iter())
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
        Span::styled(temp_str, Style::default().fg(COLOR_TEXT)),
    ]));

    let proc_panel = Paragraph::new(proc_lines);
    frame.render_widget(proc_panel, proc_inner);
}
