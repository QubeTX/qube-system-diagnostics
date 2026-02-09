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
    let title = if mode == DiagnosticMode::User { "Graphics" } else { "GPU" };
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
            "  Your computer may have an integrated graphics chip that works fine",
            Style::default().fg(COLOR_MUTED),
        )));
        lines.push(Line::from(Span::styled(
            "  but doesn't provide detailed monitoring data.",
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
    let status = HealthStatus::from_percent(util as f64);

    let util_desc = if util < 25.0 { "Not very busy" }
        else if util < 50.0 { "Moderately busy" }
        else if util < 75.0 { "Busy" }
        else { "Very busy" };

    let mem_pct = gpu.memory_percent();
    let mem_desc = if mem_pct < 50.0 { "Mostly free" }
        else if mem_pct < 80.0 { "Moderate use" }
        else { "Nearly full" };

    let temp_desc = gpu.temperature
        .map(|t| format!("{} ({})", plain_language_temp(t), format_temp(t, app.temp_unit)))
        .unwrap_or_else(|| "Unknown".into());

    let simple_name = simplify_gpu_name(&gpu.name);

    let lines = vec![
        Line::from(""),
        status_line(&status, "Card", &simple_name),
        Line::from(vec![
            Span::styled("  Utilization    ", Style::default().fg(COLOR_TEXT)),
            Span::styled(format!("{} ({:.0}%)", util_desc, util), Style::default().fg(COLOR_DIM)),
        ]),
        gauge_line("GPU", util as f64, 20),
        Line::from(vec![
            Span::styled("  Memory         ", Style::default().fg(COLOR_TEXT)),
            Span::styled(format!("Graphics memory: {}", mem_desc), Style::default().fg(COLOR_DIM)),
        ]),
        Line::from(vec![
            Span::styled("  Temperature    ", Style::default().fg(COLOR_TEXT)),
            Span::styled(temp_desc, Style::default().fg(COLOR_DIM)),
        ]),
    ];

    let panel = Paragraph::new(lines);
    frame.render_widget(panel, inner);
}

fn render_tech(frame: &mut Frame, app: &App, area: Rect) {
    let gpu = &app.snapshot.gpu;
    let outer = content_block(&format!("GPU \u{2014} {}", gpu.name));
    let inner = outer.inner(area);
    frame.render_widget(outer, area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(6),
            Constraint::Min(6),
        ])
        .split(inner);

    let temp_str = gpu.temperature
        .map(|t| format_temp(t, app.temp_unit))
        .unwrap_or_else(|| "N/A".into());

    let lines = vec![
        Line::from(vec![
            Span::styled("  Driver     ", Style::default().fg(COLOR_DIM)),
            Span::styled(&gpu.driver_version, Style::default().fg(COLOR_TEXT)),
            Span::styled("    Temp  ", Style::default().fg(COLOR_DIM)),
            Span::styled(&temp_str, Style::default().fg(COLOR_TEXT)),
        ]),
        Line::from(vec![
            Span::styled("  VRAM       ", Style::default().fg(COLOR_DIM)),
            Span::styled(
                format!("{} / {} MB ({:.1}%)", gpu.memory_used_mb, gpu.memory_total_mb, gpu.memory_percent()),
                Style::default().fg(COLOR_TEXT),
            ),
        ]),
        Line::from(vec![
            Span::styled("  GPU Util   ", Style::default().fg(COLOR_DIM)),
            Span::styled(gauge_bar(gpu.utilization_percent as f64, 20), Style::default().fg(status_color(&HealthStatus::from_percent(gpu.utilization_percent as f64)))),
        ]),
        Line::from(vec![
            Span::styled("  VRAM Util  ", Style::default().fg(COLOR_DIM)),
            Span::styled(gauge_bar(gpu.memory_percent(), 20), Style::default().fg(status_color(&HealthStatus::from_percent(gpu.memory_percent())))),
        ]),
    ];

    let info_panel = Paragraph::new(lines);
    frame.render_widget(info_panel, chunks[0]);

    // GPU history sparkline
    let spark_data = app.gpu_history.as_u64_vec();
    let sparkline = Sparkline::default()
        .block(sub_block("GPU Utilization (60s)"))
        .data(&spark_data)
        .max(100)
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
