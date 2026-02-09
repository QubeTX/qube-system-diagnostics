use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Paragraph, Sparkline};
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
    let outer = content_block("Thermals & Power");
    let inner = outer.inner(area);
    frame.render_widget(outer, area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(10),
            Constraint::Length(7),
        ])
        .split(inner);

    let thermal = &app.snapshot.thermals;
    let unit = app.temp_unit;

    let mut lines = vec![
        Line::from(Span::styled(
            "  Press f to toggle \u{00B0}C / \u{00B0}F",
            Style::default().fg(COLOR_MUTED),
        )),
        Line::from(""),
    ];

    // CPU temperature
    if let Some(temp) = thermal.cpu_temp {
        let desc = format!("{} ({})", plain_language_temp(temp), format_temp(temp, unit));
        let status = if temp < TEMP_CPU_WARN { HealthStatus::Good }
            else if temp < TEMP_CPU_CRIT { HealthStatus::Warning }
            else { HealthStatus::Critical };
        lines.push(status_line(&status, "Processor", &desc));
    } else {
        lines.push(status_line(&HealthStatus::Unknown, "Processor", "Temperature data unavailable"));
    }

    // GPU temperature
    if let Some(temp) = thermal.gpu_temp {
        let desc = format!("{} ({})", plain_language_temp(temp), format_temp(temp, unit));
        let status = if temp < TEMP_GPU_WARN { HealthStatus::Good }
            else if temp < TEMP_GPU_CRIT { HealthStatus::Warning }
            else { HealthStatus::Critical };
        lines.push(status_line(&status, "Graphics", &desc));
    }

    // Fans
    if thermal.fans.is_empty() {
        lines.push(Line::from(vec![
            Span::styled("  Fans            ", Style::default().fg(COLOR_TEXT)),
            Span::styled("Quiet / not detected", Style::default().fg(COLOR_DIM)),
        ]));
    } else {
        for fan in &thermal.fans {
            let desc = if fan.rpm == 0 {
                "Off".to_string()
            } else {
                format!("Running ({} RPM)", fan.rpm)
            };
            lines.push(Line::from(vec![
                Span::styled(format!("  {:<16}", fan.label), Style::default().fg(COLOR_TEXT)),
                Span::styled(desc, Style::default().fg(COLOR_DIM)),
            ]));
        }
    }

    // Battery
    if let Some(ref bat) = thermal.battery {
        let charge_str = if bat.is_charging { "Charging" } else { "On battery" };
        lines.push(Line::from(""));
        lines.push(Line::from(vec![
            Span::styled("  Battery         ", Style::default().fg(COLOR_TEXT)),
            Span::styled(
                format!("{:.0}%, {}", bat.percent, charge_str),
                Style::default().fg(COLOR_TEXT),
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
        Span::styled("  Power           ", Style::default().fg(COLOR_TEXT)),
        Span::styled(power_desc.to_string(), Style::default().fg(COLOR_DIM)),
    ]));

    let panel = Paragraph::new(lines);
    frame.render_widget(panel, chunks[0]);

    // Temperature history sparkline (now in user mode too)
    let spark_data = app.temp_history.as_u64_vec();
    let sparkline = Sparkline::default()
        .block(sub_block("CPU Temperature (60s)"))
        .data(&spark_data)
        .max(100)
        .style(Style::default().fg(SPARK_TEMP));
    frame.render_widget(sparkline, chunks[1]);
}

fn render_tech(frame: &mut Frame, app: &App, area: Rect) {
    let thermal = &app.snapshot.thermals;
    let unit = app.temp_unit;
    let cpu_temp_str = thermal.cpu_temp.map(|t| format_temp(t, unit)).unwrap_or("N/A".into());

    let outer = content_block(&format!("Thermals & Power \u{2014} CPU: {}", cpu_temp_str));
    let inner = outer.inner(area);
    frame.render_widget(outer, area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(6),
            Constraint::Length(7),
        ])
        .split(inner);

    // Header
    let header_lines = vec![
        Line::from(Span::styled(
            "  Press f to toggle \u{00B0}C / \u{00B0}F",
            Style::default().fg(COLOR_MUTED),
        )),
        Line::from(""),
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
            COLOR_CRIT
        } else if sensor.temperature > TEMP_CPU_WARN {
            COLOR_WARN
        } else {
            COLOR_TEXT
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
                Style::default().fg(COLOR_TEXT),
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
                Style::default().fg(COLOR_TEXT),
            ),
        ]));
    }

    let sensor_panel = Paragraph::new(sensor_lines);
    frame.render_widget(sensor_panel, chunks[1]);

    // Temperature history sparkline
    let spark_data = app.temp_history.as_u64_vec();
    let sparkline = Sparkline::default()
        .block(sub_block("CPU Temperature (60s)"))
        .data(&spark_data)
        .max(100)
        .style(Style::default().fg(SPARK_TEMP));
    frame.render_widget(sparkline, chunks[2]);
}
