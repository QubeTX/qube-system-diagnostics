use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Paragraph, Sparkline};
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
    let outer = content_block("Processor");
    let inner = outer.inner(area);
    frame.render_widget(outer, area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(6),  // Status
            Constraint::Length(8),  // Sparkline
            Constraint::Min(5),    // Top consumers
        ])
        .split(inner);

    // Status
    let cpu = &app.snapshot.cpu;
    let status = HealthStatus::from_percent(cpu.total_usage as f64);
    let unit = app.temp_unit;
    let temp_desc = app.snapshot.thermals.cpu_temp
        .map(|t| format!("{} ({}) \u{2014} {}", plain_language_temp(t), format_temp(t, unit),
            if t > TEMP_CPU_WARN { "This is expected when busy" } else { "Comfortable" }
        ))
        .unwrap_or_else(|| "Sensor data unavailable".into());

    let freq = cpu.per_core_frequency.first().unwrap_or(&0);
    let speed_desc = if *freq > 3000 { "Running at full speed" }
        else if *freq > 2000 { "Running at normal speed" }
        else { "Slowed down" };

    let status_lines = vec![
        Line::from(""),
        status_line(&status, "Status", plain_language_cpu(cpu.total_usage)),
        health_gauge_line_simple("How busy", cpu.total_usage as f64, 20),
        Line::from(vec![
            Span::styled("  Temperature    ", Style::default().fg(COLOR_TEXT)),
            Span::styled(temp_desc, Style::default().fg(COLOR_DIM)),
        ]),
        Line::from(vec![
            Span::styled("  Speed          ", Style::default().fg(COLOR_TEXT)),
            Span::styled(speed_desc.to_string(), Style::default().fg(COLOR_DIM)),
        ]),
    ];
    let status_panel = Paragraph::new(status_lines);
    frame.render_widget(status_panel, chunks[0]);

    // Sparkline
    let spark_data = app.cpu_history.as_u64_vec();
    let sparkline_block = sub_block("Activity \u{2014} Last 60 Seconds");
    let sparkline = Sparkline::default()
        .block(sparkline_block)
        .data(&spark_data)
        .max(100)
        .bar_set(sparkline_bar_set())
        .style(Style::default().fg(SPARK_CPU));
    frame.render_widget(sparkline, chunks[1]);

    // Top consumers
    let consumer_block = sub_block("What's Keeping It Busy");
    let consumer_inner = consumer_block.inner(chunks[2]);
    frame.render_widget(consumer_block, chunks[2]);

    let mut consumer_lines = Vec::new();
    for proc in app.snapshot.processes.list.iter().take(5) {
        if proc.cpu_percent > 0.1 {
            consumer_lines.push(Line::from(vec![
                Span::styled(format!("  {:<24}", proc.friendly_name), Style::default().fg(COLOR_TEXT)),
                Span::styled(gauge_bar(proc.cpu_percent as f64, 20), Style::default().fg(COLOR_INFO)),
            ]));
        }
    }

    let consumer_panel = Paragraph::new(consumer_lines);
    frame.render_widget(consumer_panel, consumer_inner);
}

fn render_tech(frame: &mut Frame, app: &App, area: Rect) {
    let cpu = &app.snapshot.cpu;
    let sys = &app.snapshot.system;
    let outer = content_block(&format!("CPU \u{2014} {}", cpu.cpu_model));
    let inner = outer.inner(area);
    frame.render_widget(outer, area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(2),   // Summary
            Constraint::Min(8),      // Per-core + history
            Constraint::Length(8),   // Process table
        ])
        .split(inner);

    let freq = cpu.per_core_frequency.first().unwrap_or(&0);
    let unit = app.temp_unit;
    let temp_str = app.snapshot.thermals.cpu_temp
        .map(|t| format_temp(t, unit))
        .unwrap_or_else(|| "N/A".into());

    // Summary
    let header_lines = vec![
        Line::from(vec![
            Span::styled(
                format!("  {} threads / {} cores \u{2014} {}",
                    cpu.thread_count, cpu.core_count, sys.architecture),
                Style::default().fg(COLOR_MUTED),
            ),
        ]),
        Line::from(vec![
            Span::styled("  Total Load  ", Style::default().fg(COLOR_DIM)),
            Span::styled(gauge_bar(cpu.total_usage as f64, 20), Style::default().fg(status_color(&HealthStatus::from_percent(cpu.total_usage as f64)))),
            Span::styled(format!("    Freq {} MHz", freq), Style::default().fg(COLOR_TEXT)),
            Span::styled(format!("    Temp {}", temp_str), Style::default().fg(COLOR_TEXT)),
        ]),
    ];
    let header_panel = Paragraph::new(header_lines);
    frame.render_widget(header_panel, chunks[0]);

    // Per-core utilization + sparkline
    let core_area = chunks[1];
    let core_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(50),
            Constraint::Percentage(50),
        ])
        .split(core_area);

    // Per-core bars
    let per_core_block = sub_block("Per-Core Utilization");
    let per_core_inner = per_core_block.inner(core_chunks[0]);
    frame.render_widget(per_core_block, core_chunks[0]);

    let mut core_lines = Vec::new();
    for (i, usage) in cpu.per_core_usage.iter().enumerate() {
        let freq_val = cpu.per_core_frequency.get(i).unwrap_or(&0);
        core_lines.push(Line::from(vec![
            Span::styled(format!("  Core {:>2} ", i), Style::default().fg(COLOR_DIM)),
            Span::styled(gauge_bar(*usage as f64, 16), Style::default().fg(status_color(&HealthStatus::from_percent(*usage as f64)))),
            Span::styled(format!("  {} MHz", freq_val), Style::default().fg(COLOR_TEXT)),
        ]));

        if core_lines.len() >= per_core_inner.height as usize {
            break;
        }
    }

    let core_panel = Paragraph::new(core_lines);
    frame.render_widget(core_panel, per_core_inner);

    // Load history sparkline
    let spark_data = app.cpu_history.as_u64_vec();
    let sparkline = Sparkline::default()
        .block(sub_block("Load History (60s)"))
        .data(&spark_data)
        .max(100)
        .bar_set(sparkline_bar_set())
        .style(Style::default().fg(SPARK_CPU));
    frame.render_widget(sparkline, core_chunks[1]);

    // Process table
    let proc_block = sub_block("Top CPU Consumers");
    let proc_inner = proc_block.inner(chunks[2]);
    frame.render_widget(proc_block, chunks[2]);

    let mut proc_lines = vec![
        Line::from(Span::styled(
            format!("  {:<28} {:>6} {:>8} {:>10}", "PROCESS", "PID", "CPU%", "MEMORY"),
            Style::default().fg(COLOR_DIM),
        )),
    ];

    for proc in app.snapshot.processes.list.iter().take(5) {
        proc_lines.push(Line::from(Span::styled(
            format!("  {:<28} {:>6} {:>7.1}% {:>10}",
                truncate_str(&proc.name, 28), proc.pid, proc.cpu_percent, format_bytes(proc.memory_bytes)),
            Style::default().fg(COLOR_TEXT),
        )));
    }

    let proc_panel = Paragraph::new(proc_lines);
    frame.render_widget(proc_panel, proc_inner);
}
