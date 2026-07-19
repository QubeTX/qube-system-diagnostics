use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Paragraph, Sparkline};
use ratatui::Frame;

use crate::app::App;
use crate::types::{DiagnosticMode, HealthStatus};
use crate::ui::common::*;

pub fn render(frame: &mut Frame, app: &App, area: Rect, mode: DiagnosticMode) {
    if !app.snapshot.gpu.available {
        render_unavailable(frame, area, mode);
        return;
    }

    match mode {
        DiagnosticMode::User => render_user(frame, app, area),
        DiagnosticMode::Technician => render_tech(frame, app, area),
    }
}

fn render_unavailable(frame: &mut Frame, area: Rect, mode: DiagnosticMode) {
    let title = if mode == DiagnosticMode::User {
        "Graphics"
    } else {
        "GPU"
    };
    let outer = content_block(title);
    let inner = outer.inner(area);
    frame.render_widget(outer, area);

    let mut lines = vec![Line::from(""), Line::from("")];

    if mode == DiagnosticMode::User {
        lines.push(Line::from(Span::styled(
            "  Detailed graphics card data is not available.",
            Style::default().fg(COLOR_MUTED),
        )));
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            "  No graphics adapter inventory provider returned usable data.",
            Style::default().fg(COLOR_MUTED),
        )));
    } else {
        lines.push(Line::from(Span::styled(
            "  GPU telemetry not available (no supported GPU detected or driver not installed)",
            Style::default().fg(COLOR_MUTED),
        )));
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            "  Install NVIDIA drivers for NVIDIA GPU metrics via nvidia-smi",
            Style::default().fg(COLOR_MUTED),
        )));
    }

    let panel = Paragraph::new(lines);
    frame.render_widget(panel, inner);
}

fn render_user(frame: &mut Frame, app: &App, area: Rect) {
    let outer = content_block("Graphics");
    let inner = outer.inner(area);
    frame.render_widget(outer, area);

    let gpu = &app.snapshot.gpu;
    let util = gpu.utilization_percent;
    let status = if gpu.telemetry_available {
        HealthStatus::from_percent(util as f64)
    } else {
        HealthStatus::Unknown
    };

    let util_desc = if util < 25.0 {
        "Not very busy"
    } else if util < 50.0 {
        "Moderately busy"
    } else if util < 75.0 {
        "Busy"
    } else {
        "Very busy"
    };

    let mem_pct = gpu.memory_percent();
    let mem_desc = if mem_pct < 50.0 {
        "Mostly free"
    } else if mem_pct < 80.0 {
        "Moderate use"
    } else {
        "Nearly full"
    };

    let temp_desc = gpu
        .temperature
        .map(|t| {
            format!(
                "{} ({})",
                plain_language_temp(t),
                format_temp(t, app.temp_unit)
            )
        })
        .unwrap_or_else(|| "Unknown".into());

    let simple_name = simplify_gpu_name(&gpu.name);

    let mut lines = vec![Line::from(""), status_line(&status, "Card", &simple_name)];

    if gpu.telemetry_available {
        lines.extend([
            Line::from(vec![
                Span::styled("  Utilization    ", Style::default().fg(COLOR_TEXT)),
                Span::styled(
                    format!("{} ({:.0}%)", util_desc, util),
                    Style::default().fg(COLOR_DIM),
                ),
            ]),
            gauge_line("GPU", util as f64, 20),
            Line::from(vec![
                Span::styled("  Memory         ", Style::default().fg(COLOR_TEXT)),
                Span::styled(
                    format!("Graphics memory: {}", mem_desc),
                    Style::default().fg(COLOR_DIM),
                ),
            ]),
            Line::from(vec![
                Span::styled("  Temperature    ", Style::default().fg(COLOR_TEXT)),
                Span::styled(temp_desc, Style::default().fg(COLOR_DIM)),
            ]),
        ]);
    } else {
        lines.push(status_line(
            &HealthStatus::Unknown,
            "Telemetry",
            "Utilization and temperature unavailable",
        ));
    }

    if gpu.adapters.len() > 1 {
        lines.push(Line::from(Span::styled(
            format!("  {} graphics adapters detected", gpu.adapters.len()),
            Style::default().fg(COLOR_DIM),
        )));
    }
    if !app.snapshot.displays.displays.is_empty() {
        lines.push(Line::from(Span::styled(
            format!(
                "  {} connected displays",
                app.snapshot.displays.displays.len()
            ),
            Style::default().fg(COLOR_DIM),
        )));
    }

    let panel = Paragraph::new(lines);
    frame.render_widget(panel, inner);
}

fn render_tech(frame: &mut Frame, app: &App, area: Rect) {
    let gpu = &app.snapshot.gpu;
    let outer = content_block(&format!("GPU \u{2014} {} adapter(s)", gpu.adapters.len()));
    let inner = outer.inner(area);
    frame.render_widget(outer, area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(7), Constraint::Length(7)])
        .split(inner);

    let mut lines = vec![Line::from(Span::styled(
        format!(
            "  {:<32} {:<12} {:>7} {:>9} {:>7}",
            "ADAPTER", "DRIVER", "UTIL", "MEMORY", "TEMP"
        ),
        Style::default().fg(COLOR_DIM),
    ))];
    for adapter in &gpu.adapters {
        let utilization = adapter
            .utilization_percent
            .map(|value| format!("{value:.0}%"))
            .unwrap_or_else(|| "N/A".into());
        let memory = adapter
            .dedicated_memory_mb
            .map(|value| format!("{value} MB"))
            .unwrap_or_else(|| "N/A".into());
        let temperature = adapter
            .temperature_celsius
            .map(|value| format_temp(value, app.temp_unit))
            .unwrap_or_else(|| "N/A".into());
        lines.push(Line::from(Span::styled(
            format!(
                "  {:<32} {:<12} {:>7} {:>9} {:>7}",
                truncate_str(&adapter.name, 32),
                truncate_str(adapter.driver_version.as_deref().unwrap_or("N/A"), 12),
                utilization,
                memory,
                temperature
            ),
            Style::default().fg(COLOR_TEXT),
        )));
        let details = [
            adapter.status.as_deref(),
            adapter.current_resolution.as_deref(),
            adapter.source.as_str().into(),
        ]
        .into_iter()
        .flatten()
        .collect::<Vec<_>>()
        .join("  ");
        lines.push(Line::from(Span::styled(
            format!("    {details}"),
            Style::default().fg(COLOR_DIM),
        )));
    }
    if !app.snapshot.displays.displays.is_empty() {
        lines.push(Line::from(""));
        for display in &app.snapshot.displays.displays {
            lines.push(Line::from(Span::styled(
                format!(
                    "  {}: {}  brightness={}  size={}x{} cm",
                    display.label,
                    display.connection,
                    display
                        .brightness_percent
                        .map(|value| format!("{value}%"))
                        .unwrap_or_else(|| "N/A".into()),
                    display
                        .physical_width_cm
                        .map(|value| value.to_string())
                        .unwrap_or_else(|| "N/A".into()),
                    display
                        .physical_height_cm
                        .map(|value| value.to_string())
                        .unwrap_or_else(|| "N/A".into()),
                ),
                Style::default().fg(COLOR_DIM),
            )));
        }
    }

    let info_panel = Paragraph::new(lines);
    frame.render_widget(info_panel, chunks[0]);

    // GPU history sparkline
    let spark_data = app.gpu_history.as_u64_vec();
    let sparkline = Sparkline::default()
        .block(sub_block("GPU Utilization (60s)"))
        .data(&spark_data)
        .max(100)
        .bar_set(sparkline_bar_set())
        .style(Style::default().fg(SPARK_GPU));
    frame.render_widget(sparkline, chunks[1]);
}

fn simplify_gpu_name(name: &str) -> String {
    if name.to_lowercase().contains("nvidia") {
        "NVIDIA graphics card".to_string()
    } else if name.to_lowercase().contains("amd") || name.to_lowercase().contains("radeon") {
        "AMD graphics card".to_string()
    } else if name.to_lowercase().contains("intel") {
        "Intel integrated graphics".to_string()
    } else {
        "Graphics card".to_string()
    }
}
