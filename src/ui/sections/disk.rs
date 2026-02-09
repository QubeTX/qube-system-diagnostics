use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

use crate::app::App;
use crate::collectors::disk::DiskType;
use crate::collectors::disk_health::DiskHealthStatus;
use crate::types::{DiagnosticMode, HealthStatus};
use crate::ui::common::*;

pub fn render(frame: &mut Frame, app: &App, area: Rect, mode: DiagnosticMode) {
    match mode {
        DiagnosticMode::User => render_user(frame, app, area),
        DiagnosticMode::Technician => render_tech(frame, app, area),
    }
}

fn render_user(frame: &mut Frame, app: &App, area: Rect) {
    let outer = content_block("Storage");
    let inner = outer.inner(area);
    frame.render_widget(outer, area);

    let mut lines = vec![Line::from("")];

    if app.snapshot.disk.partitions.is_empty() {
        lines.push(Line::from(Span::styled(
            "  No storage devices detected",
            Style::default().fg(COLOR_DIM),
        )));
    } else {
        for (i, part) in app.snapshot.disk.partitions.iter().enumerate() {
            let pct = part.usage_percent();
            let status = HealthStatus::from_percent(pct);

            let type_desc = match part.disk_type {
                DiskType::Ssd => "Fast solid-state drive",
                DiskType::Hdd => "Mechanical drive",
                DiskType::Unknown => "Storage drive",
            };

            let name = if part.mount_point.is_empty() {
                &part.name
            } else {
                &part.mount_point
            };

            lines.push(Line::from(vec![
                Span::styled(format!("  {} ", status.icon()), Style::default().fg(status_color(&status))),
                Span::styled(name.to_string(), Style::default().fg(COLOR_TEXT).add_modifier(Modifier::BOLD)),
            ]));
            lines.push(Line::from(vec![
                Span::styled("    Type     ", Style::default().fg(COLOR_DIM)),
                Span::styled(type_desc.to_string(), Style::default().fg(COLOR_TEXT)),
            ]));
            lines.push(Line::from(vec![
                Span::styled("    Space    ", Style::default().fg(COLOR_DIM)),
                Span::styled(
                    format!("Using {} of {} ({:.0}%)", format_bytes(part.used_bytes), format_bytes(part.total_bytes), pct),
                    Style::default().fg(COLOR_TEXT),
                ),
            ]));
            lines.push(gauge_line(&format!("    {}", name), pct, 20));

            // Real health from disk_health collector
            let drive_health = if app.snapshot.disk_health.drives.len() == 1 {
                app.snapshot.disk_health.drives.first()
            } else {
                app.snapshot.disk_health.drives.get(i)
                    .or_else(|| app.snapshot.disk_health.drives.first())
            };
            let (health_label, health_color) = match drive_health.map(|d| &d.health_status) {
                Some(DiskHealthStatus::Healthy) => ("Good", COLOR_GOOD),
                Some(DiskHealthStatus::Warning) => ("Degrading - Back up data", COLOR_WARN),
                Some(DiskHealthStatus::Critical) => ("FAILING - Back up immediately!", COLOR_CRIT),
                Some(DiskHealthStatus::Unknown) | None => ("Unknown", COLOR_DIM),
            };
            lines.push(Line::from(vec![
                Span::styled("    Health   ", Style::default().fg(COLOR_DIM)),
                Span::styled(health_label.to_string(), Style::default().fg(health_color)),
            ]));
            lines.push(Line::from(""));
        }
    }

    let panel = Paragraph::new(lines);
    frame.render_widget(panel, inner);
}

fn render_tech(frame: &mut Frame, app: &App, area: Rect) {
    let outer = content_block("Disk / Storage");
    let inner = outer.inner(area);
    frame.render_widget(outer, area);

    let mut lines = vec![
        Line::from(Span::styled(
            format!("  {:<20} {:<10} {:>12} {:>12} {:>8} {}", "MOUNT", "FS", "USED", "TOTAL", "USE%", "TYPE"),
            Style::default().fg(COLOR_DIM),
        )),
    ];

    for part in &app.snapshot.disk.partitions {
        let pct = part.usage_percent();
        let color = status_color(&HealthStatus::from_percent(pct));

        lines.push(Line::from(vec![
            Span::styled(
                format!("  {:<20} {:<10} {:>12} {:>12} ",
                    truncate_str(&part.mount_point, 20),
                    truncate_str(&part.filesystem, 10),
                    format_bytes_gib(part.used_bytes),
                    format_bytes_gib(part.total_bytes),
                ),
                Style::default().fg(COLOR_TEXT),
            ),
            Span::styled(format!("{:>7.1}%", pct), Style::default().fg(color)),
            Span::styled(format!(" {}", part.disk_type), Style::default().fg(COLOR_DIM)),
        ]));

        // Gauge bar
        lines.push(Line::from(vec![
            Span::styled("    ", Style::default()),
            Span::styled(gauge_bar(pct, 20), Style::default().fg(color)),
        ]));
    }

    // Physical drives table
    if !app.snapshot.disk_health.drives.is_empty() {
        lines.push(Line::from(""));
        lines.push(section_header("PHYSICAL DRIVES"));
        lines.push(Line::from(Span::styled(
            format!("  {:<30} {:<8} {:<10} {:>8} {:>12} {:>12}", "MODEL", "TYPE", "HEALTH", "TEMP", "RD/s", "WR/s"),
            Style::default().fg(COLOR_DIM),
        )));

        for drive in &app.snapshot.disk_health.drives {
            let health_color = match drive.health_status {
                DiskHealthStatus::Healthy => COLOR_GOOD,
                DiskHealthStatus::Warning => COLOR_WARN,
                DiskHealthStatus::Critical => COLOR_CRIT,
                DiskHealthStatus::Unknown => COLOR_DIM,
            };

            let temp_str = drive.temperature_celsius
                .map(|t| format!("{:.0}C", t))
                .unwrap_or_else(|| "N/A".into());

            let (rd_str, wr_str) = if let Some(ref io) = drive.io_stats {
                (format_throughput(io.read_bytes_per_sec), format_throughput(io.write_bytes_per_sec))
            } else {
                ("N/A".into(), "N/A".into())
            };

            lines.push(Line::from(vec![
                Span::styled(
                    format!("  {:<30} {:<8} ", truncate_str(&drive.model, 30), drive.media_type),
                    Style::default().fg(COLOR_TEXT),
                ),
                Span::styled(format!("{:<10}", drive.health_status.user_label()), Style::default().fg(health_color)),
                Span::styled(
                    format!("{:>8} {:>12} {:>12}", temp_str, rd_str, wr_str),
                    Style::default().fg(COLOR_DIM),
                ),
            ]));

            // Detail line
            if let Some(ref io) = drive.io_stats {
                let mut detail_parts = vec![
                    format!("Queue: {:.1}", io.queue_depth),
                    format!("RdLat: {:.1}ms", io.avg_read_latency_ms),
                    format!("WrLat: {:.1}ms", io.avg_write_latency_ms),
                ];
                if let Some(poh) = drive.power_on_hours {
                    detail_parts.push(format!("POH: {}", poh));
                }
                lines.push(Line::from(Span::styled(
                    format!("    {}", detail_parts.join("  ")),
                    Style::default().fg(COLOR_DIM),
                )));
            }
        }
    }

    // Show disk health warnings
    let disk_warnings: Vec<_> = app.snapshot.warnings.iter()
        .filter(|w| w.source == "Disk Health")
        .collect();
    if !disk_warnings.is_empty() {
        lines.push(Line::from(""));
        for warn in disk_warnings {
            lines.push(Line::from(Span::styled(
                format!("  \u{26A0} {}", warn.message),
                Style::default().fg(COLOR_WARN),
            )));
        }
    }

    let panel = Paragraph::new(lines);
    frame.render_widget(panel, inner);
}
