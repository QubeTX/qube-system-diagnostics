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
    let mut lines = vec![Line::from(""), Line::from(gpu.name.clone())];
    if let Some(util) = gpu.utilization() {
        lines.push(Line::from(format!("  Graphics utilization: {util:.1}%")));
        lines.push(gauge_line("GPU", util as f64, 20));
    } else {
        lines.push(status_line(
            &HealthStatus::Unknown,
            "Utilization",
            "Not exposed by the available provider",
        ));
    }
    if let Some(adapter) = gpu.primary() {
        if adapter.unified_memory == Some(true) {
            lines.push(Line::from("  This GPU shares memory with the processor."));
        }
        if let Some(capacity) = adapter.dedicated_memory_mb {
            lines.push(Line::from(format!(
                "  Reported GPU memory capacity: {capacity} MiB"
            )));
        }
        if let Some(used) = adapter.memory_used_mb {
            lines.push(Line::from(format!(
                "  Reported GPU memory used: {used} MiB"
            )));
        }
        if let Some(limit) = adapter.recommended_working_set_mb {
            lines.push(Line::from(format!(
                "  Recommended allocation budget: {limit} MiB (not VRAM capacity)"
            )));
        }
        if let Some(temp) = adapter.temperature_celsius {
            lines.push(Line::from(format!(
                "  Temperature: {}",
                format_temp(temp, app.temp_unit)
            )));
        } else {
            lines.push(Line::from("  Temperature: not reported"));
        }
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
            .map(|value| format!("{value} MiB"))
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
        lines.push(Line::from(format!("    ID: {}", adapter.device_id)));
        if adapter.unified_memory == Some(true) {
            lines.push(Line::from(
                "    Unified memory: shared with CPU; no dedicated VRAM value",
            ));
        }
        if let Some(shared) = adapter.shared_memory_mb {
            lines.push(Line::from(format!(
                "    Shared-system limit: {shared} MiB (not currently used)"
            )));
        }
        if let Some(dedicated) = adapter.dedicated_system_memory_mb {
            lines.push(Line::from(format!(
                "    Dedicated system memory: {dedicated} MiB"
            )));
        }
        if let Some(recommended) = adapter.recommended_working_set_mb {
            lines.push(Line::from(format!(
                "    Recommended working set: {recommended} MiB (not capacity)"
            )));
        }
        if let Some(field) = adapter.fields.get("utilization_percent") {
            lines.push(Line::from(format!(
                "    Utilization: {:?}; {}",
                field.status, field.source
            )));
        }
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
