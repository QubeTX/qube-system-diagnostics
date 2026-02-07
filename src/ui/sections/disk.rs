use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

use crate::app::App;
use crate::collectors::disk::DiskType;
use crate::types::{DiagnosticMode, HealthStatus};
use crate::ui::common::*;

pub fn render(frame: &mut Frame, app: &App, area: Rect, mode: DiagnosticMode) {
    match mode {
        DiagnosticMode::User => render_user(frame, app, area),
        DiagnosticMode::Technician => render_tech(frame, app, area),
    }
}

fn render_user(frame: &mut Frame, app: &App, area: Rect) {
    let mut lines = vec![
        Line::from(Span::styled(
            "  STORAGE",
            Style::default().fg(COLOR_HEADER).add_modifier(Modifier::BOLD),
        )),
        separator(area.width as usize),
        Line::from(""),
    ];

    if app.snapshot.disk.partitions.is_empty() {
        lines.push(Line::from(Span::styled(
            "  No storage devices detected",
            Style::default().fg(COLOR_DIM),
        )));
    } else {
        for part in &app.snapshot.disk.partitions {
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
                Span::styled(format!("{}", name), Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
            ]));
            lines.push(Line::from(vec![
                Span::styled("    Type     ", Style::default().fg(COLOR_DIM)),
                Span::styled(type_desc.to_string(), Style::default().fg(Color::White)),
            ]));
            lines.push(Line::from(vec![
                Span::styled("    Space    ", Style::default().fg(COLOR_DIM)),
                Span::styled(
                    format!("Using {} of {} ({:.0}%)", format_bytes(part.used_bytes), format_bytes(part.total_bytes), pct),
                    Style::default().fg(Color::White),
                ),
            ]));
            lines.push(gauge_line(&format!("    {}", name), pct, 20));
            lines.push(Line::from(vec![
                Span::styled("    Health   ", Style::default().fg(COLOR_DIM)),
                Span::styled("Good", Style::default().fg(COLOR_GOOD)),
            ]));
            lines.push(Line::from(""));
        }
    }

    let panel = Paragraph::new(lines);
    frame.render_widget(panel, area);
}

fn render_tech(frame: &mut Frame, app: &App, area: Rect) {
    let mut lines = vec![
        Line::from(Span::styled(
            "  DISK / STORAGE",
            Style::default().fg(COLOR_HEADER).add_modifier(Modifier::BOLD),
        )),
        separator(area.width as usize),
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
                Style::default().fg(Color::White),
            ),
            Span::styled(format!("{:>7.1}%", pct), Style::default().fg(color)),
            Span::styled(format!(" {}", part.disk_type), Style::default().fg(COLOR_DIM)),
        ]));

        // Gauge bar
        lines.push(Line::from(vec![
            Span::styled("    ", Style::default()),
            Span::styled(gauge_bar(pct, 30), Style::default().fg(color)),
        ]));
    }

    let panel = Paragraph::new(lines);
    frame.render_widget(panel, area);
}

fn truncate_str(s: &str, max: usize) -> String {
    if max < 3 { return s.chars().take(max).collect(); }
    if s.chars().count() <= max { s.to_string() } else {
        let truncated: String = s.chars().take(max - 2).collect();
        format!("{}..", truncated)
    }
}
