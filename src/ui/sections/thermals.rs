use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Sparkline};
use ratatui::Frame;

use crate::app::App;
use crate::collectors::thermals::PowerSource;
use crate::types::{DiagnosticMode, HealthStatus};
use crate::ui::common::*;

pub fn render(frame: &mut Frame, app: &App, area: Rect, mode: DiagnosticMode) {
    match mode {
        DiagnosticMode::User => render_user(frame, app, area),
        DiagnosticMode::Technician => render_tech(frame, app, area),
    }
}

fn render_user(frame: &mut Frame, app: &App, area: Rect) {
    let thermal = &app.snapshot.thermals;
    let unit = app.temp_unit;

    let mut lines = vec![
        Line::from(Span::styled(
            "  THERMALS & POWER",
            Style::default().fg(COLOR_HEADER).add_modifier(Modifier::BOLD),
        )),
        separator(area.width as usize),
        Line::from(Span::styled(
            "  Press f to toggle \u{00B0}C / \u{00B0}F",
            Style::default().fg(COLOR_DIM),
        )),
        Line::from(""),
    ];

    // CPU temperature
    if let Some(temp) = thermal.cpu_temp {
        let desc = format!("{} ({})", plain_language_temp(temp), format_temp(temp, unit));
        let status = if temp < 70.0 { HealthStatus::Good }
            else if temp < 85.0 { HealthStatus::Warning }
            else { HealthStatus::Critical };
        lines.push(status_line(&status, "Processor", &desc));
    } else {
        lines.push(status_line(&HealthStatus::Unknown, "Processor", "Temperature data unavailable"));
    }

    // GPU temperature
    if let Some(temp) = thermal.gpu_temp {
        let desc = format!("{} ({})", plain_language_temp(temp), format_temp(temp, unit));
        let status = if temp < 75.0 { HealthStatus::Good }
            else if temp < 90.0 { HealthStatus::Warning }
            else { HealthStatus::Critical };
        lines.push(status_line(&status, "Graphics", &desc));
    }

    // Fans
    if thermal.fans.is_empty() {
        lines.push(Line::from(vec![
            Span::styled("  Fans            ", Style::default().fg(Color::White)),
            Span::styled("Quiet / not detected", Style::default().fg(COLOR_DIM)),
        ]));
    } else {
        for fan in &thermal.fans {
            let desc = if fan.rpm == 0 { "Off" } else { "Running" };
            lines.push(Line::from(vec![
                Span::styled(format!("  {:<16}", fan.label), Style::default().fg(Color::White)),
                Span::styled(desc.to_string(), Style::default().fg(COLOR_DIM)),
            ]));
        }
    }

    // Battery
    if let Some(ref bat) = thermal.battery {
        let charge_str = if bat.is_charging { "Charging" } else { "On battery" };
        lines.push(Line::from(""));
        lines.push(Line::from(vec![
            Span::styled("  Battery         ", Style::default().fg(Color::White)),
            Span::styled(
                format!("{:.0}%, {}", bat.percent, charge_str),
                Style::default().fg(Color::White),
            ),
        ]));
        lines.push(gauge_line("Battery", bat.percent, 20));
    }

    // Power source
    let power_desc = match thermal.power_source {
        PowerSource::Ac => "Plugged in",
        PowerSource::Battery => "On battery",
        PowerSource::Unknown => "Unknown",
    };
    lines.push(Line::from(vec![
        Span::styled("  Power           ", Style::default().fg(Color::White)),
        Span::styled(power_desc.to_string(), Style::default().fg(COLOR_DIM)),
    ]));

    let panel = Paragraph::new(lines);
    frame.render_widget(panel, area);
}

fn render_tech(frame: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(4),
            Constraint::Min(6),
            Constraint::Length(8),
        ])
        .split(area);

    let thermal = &app.snapshot.thermals;
    let unit = app.temp_unit;

    // Header
    let cpu_temp_str = thermal.cpu_temp.map(|t| format_temp(t, unit)).unwrap_or("N/A".into());
    let header_lines = vec![
        Line::from(Span::styled(
            format!("  THERMALS & POWER \u{2014} CPU: {}", cpu_temp_str),
            Style::default().fg(COLOR_HEADER).add_modifier(Modifier::BOLD),
        )),
        separator(area.width as usize),
        Line::from(Span::styled(
            "  Press f to toggle \u{00B0}C / \u{00B0}F",
            Style::default().fg(COLOR_DIM),
        )),
        Line::from(Span::styled(
            format!("  {:<24} {:>10} {:>10}", "SENSOR", "TEMP", "CRITICAL"),
            Style::default().fg(COLOR_DIM),
        )),
    ];
    let header_panel = Paragraph::new(header_lines);
    frame.render_widget(header_panel, chunks[0]);

    // Sensor table
    let mut sensor_lines = Vec::new();
    for sensor in &thermal.sensors {
        let crit_str = sensor.critical
            .map(|c| format_temp(c, unit))
            .unwrap_or_else(|| "N/A".into());

        let color = if sensor.temperature > sensor.critical.unwrap_or(100.0) {
            Color::Red
        } else if sensor.temperature > 80.0 {
            Color::Yellow
        } else {
            Color::White
        };

        sensor_lines.push(Line::from(Span::styled(
            format!("  {:<24} {:>10} {:>10}",
                truncate_str(&sensor.label, 24), format_temp(sensor.temperature, unit), crit_str),
            Style::default().fg(color),
        )));
    }

    if sensor_lines.is_empty() {
        sensor_lines.push(Line::from(Span::styled(
            "  No temperature sensors detected",
            Style::default().fg(COLOR_DIM),
        )));
    }

    // Fan info
    sensor_lines.push(Line::from(""));
    if thermal.fans.is_empty() {
        sensor_lines.push(Line::from(Span::styled(
            "  Fans: No data available",
            Style::default().fg(COLOR_DIM),
        )));
    } else {
        for fan in &thermal.fans {
            sensor_lines.push(Line::from(Span::styled(
                format!("  Fan: {} \u{2014} {} RPM", fan.label, fan.rpm),
                Style::default().fg(Color::White),
            )));
        }
    }

    // Battery details
    if let Some(ref bat) = thermal.battery {
        sensor_lines.push(Line::from(""));
        sensor_lines.push(Line::from(vec![
            Span::styled("  Battery  ", Style::default().fg(COLOR_DIM)),
            Span::styled(
                format!("{:.1}%  {}",
                    bat.percent,
                    if bat.is_charging { "AC (charging)" } else { "Discharging" }),
                Style::default().fg(Color::White),
            ),
        ]));
    }

    let sensor_panel = Paragraph::new(sensor_lines);
    frame.render_widget(sensor_panel, chunks[1]);

    // Temperature history sparkline
    let spark_data = app.temp_history.as_u64_vec();
    let sparkline = Sparkline::default()
        .block(Block::default()
            .title(" CPU Temperature (60s) ")
            .title_style(Style::default().fg(COLOR_HEADER))
            .borders(Borders::ALL)
            .border_style(Style::default().fg(COLOR_DIM)))
        .data(&spark_data)
        .max(100)
        .style(Style::default().fg(Color::Red));
    frame.render_widget(sparkline, chunks[2]);
}

fn truncate_str(s: &str, max: usize) -> String {
    if max < 3 { return s.chars().take(max).collect(); }
    if s.chars().count() <= max { s.to_string() } else {
        let truncated: String = s.chars().take(max - 2).collect();
        format!("{}..", truncated)
    }
}
