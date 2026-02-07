use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Paragraph, Sparkline, Block, Borders};
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
            Constraint::Length(7),  // Status
            Constraint::Length(9),  // Sparkline
            Constraint::Min(6),    // Top consumers
        ])
        .split(area);

    // Header
    let header = Paragraph::new(Line::from(vec![
        Span::styled("  PROCESSOR", Style::default().fg(COLOR_HEADER).add_modifier(Modifier::BOLD)),
    ]));
    frame.render_widget(header, chunks[0]);

    // Status
    let cpu = &app.snapshot.cpu;
    let status = HealthStatus::from_percent(cpu.total_usage as f64);
    let unit = app.temp_unit;
    let temp_desc = app.snapshot.thermals.cpu_temp
        .map(|t| format!("{} ({}) \u{2014} {}", plain_language_temp(t), format_temp(t, unit),
            if t > 70.0 { "This is expected when busy" } else { "Comfortable" }
        ))
        .unwrap_or_else(|| "Sensor data unavailable".into());

    let freq = cpu.per_core_frequency.first().unwrap_or(&0);
    let speed_desc = if *freq > 3000 { "Running at full speed" }
        else if *freq > 2000 { "Running at normal speed" }
        else { "Slowed down" };

    let status_lines = vec![
        separator(area.width as usize),
        status_line(&status, "Status", &format!("{}", plain_language_cpu(cpu.total_usage))),
        health_gauge_line_simple("How busy", cpu.total_usage as f64, 20),
        Line::from(vec![
            Span::styled("  Temperature    ", Style::default().fg(Color::White)),
            Span::styled(temp_desc, Style::default().fg(COLOR_DIM)),
        ]),
        Line::from(vec![
            Span::styled("  Speed          ", Style::default().fg(Color::White)),
            Span::styled(speed_desc.to_string(), Style::default().fg(COLOR_DIM)),
        ]),
    ];
    let status_panel = Paragraph::new(status_lines);
    frame.render_widget(status_panel, chunks[1]);

    // Sparkline
    let spark_data = app.cpu_history.as_u64_vec();
    let sparkline_block = Block::default()
        .title("  OVER TIME (last 60 seconds)")
        .title_style(Style::default().fg(COLOR_HEADER))
        .borders(Borders::NONE);
    let sparkline = Sparkline::default()
        .block(sparkline_block)
        .data(&spark_data)
        .max(100)
        .style(Style::default().fg(COLOR_INFO));
    frame.render_widget(sparkline, chunks[2]);

    // Top consumers
    let mut consumer_lines = vec![
        separator(area.width as usize),
        section_header("WHAT'S KEEPING THE PROCESSOR BUSY"),
        Line::from(""),
    ];

    for proc in app.snapshot.processes.list.iter().take(5) {
        if proc.cpu_percent > 0.1 {
            consumer_lines.push(Line::from(vec![
                Span::styled(format!("  {:<24}", proc.friendly_name), Style::default().fg(Color::White)),
                Span::styled(gauge_bar(proc.cpu_percent as f64, 10), Style::default().fg(COLOR_INFO)),
            ]));
        }
    }

    let consumer_panel = Paragraph::new(consumer_lines);
    frame.render_widget(consumer_panel, chunks[3]);
}

fn render_tech(frame: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),   // Header + summary
            Constraint::Min(8),      // Per-core + history
            Constraint::Length(8),   // Process table
        ])
        .split(area);

    let cpu = &app.snapshot.cpu;
    let sys = &app.snapshot.system;
    let freq = cpu.per_core_frequency.first().unwrap_or(&0);
    let unit = app.temp_unit;
    let temp_str = app.snapshot.thermals.cpu_temp
        .map(|t| format_temp(t, unit))
        .unwrap_or_else(|| "N/A".into());

    // Header
    let header_lines = vec![
        Line::from(vec![
            Span::styled(
                format!("  CPU \u{2014} {} ({} threads / {} cores) \u{2014} {}",
                    cpu.cpu_model, cpu.thread_count, cpu.core_count, sys.architecture),
                Style::default().fg(COLOR_HEADER).add_modifier(Modifier::BOLD),
            ),
        ]),
        separator(area.width as usize),
        Line::from(vec![
            Span::styled("  Total Load  ", Style::default().fg(COLOR_DIM)),
            Span::styled(gauge_bar(cpu.total_usage as f64, 20), Style::default().fg(status_color(&HealthStatus::from_percent(cpu.total_usage as f64)))),
            Span::styled(format!("    Freq {} MHz", freq), Style::default().fg(Color::White)),
            Span::styled(format!("    Temp {}", temp_str), Style::default().fg(Color::White)),
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
    let mut core_lines = vec![
        section_header("PER-CORE UTILIZATION"),
        Line::from(""),
    ];

    for (i, usage) in cpu.per_core_usage.iter().enumerate() {
        let freq_val = cpu.per_core_frequency.get(i).unwrap_or(&0);
        core_lines.push(Line::from(vec![
            Span::styled(format!("  Core {:>2} ", i), Style::default().fg(COLOR_DIM)),
            Span::styled(gauge_bar(*usage as f64, 16), Style::default().fg(status_color(&HealthStatus::from_percent(*usage as f64)))),
            Span::styled(format!("  {} MHz", freq_val), Style::default().fg(Color::White)),
        ]));

        if core_lines.len() >= core_area.height as usize {
            break;
        }
    }

    let core_panel = Paragraph::new(core_lines);
    frame.render_widget(core_panel, core_chunks[0]);

    // Load history sparkline
    let spark_data = app.cpu_history.as_u64_vec();
    let sparkline_block = Block::default()
        .title(" LOAD HISTORY (60s) ")
        .title_style(Style::default().fg(COLOR_HEADER))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(COLOR_DIM));
    let sparkline = Sparkline::default()
        .block(sparkline_block)
        .data(&spark_data)
        .max(100)
        .style(Style::default().fg(COLOR_ACCENT));
    frame.render_widget(sparkline, core_chunks[1]);

    // Process table
    let mut proc_lines = vec![
        separator(area.width as usize),
        Line::from(Span::styled(
            format!("  {:<28} {:>6} {:>8} {:>10}", "TOP CPU CONSUMERS", "PID", "CPU%", "MEMORY"),
            Style::default().fg(COLOR_DIM),
        )),
    ];

    for proc in app.snapshot.processes.list.iter().take(5) {
        proc_lines.push(Line::from(Span::styled(
            format!("  {:<28} {:>6} {:>7.1}% {:>10}",
                truncate_str(&proc.name, 28), proc.pid, proc.cpu_percent, format_bytes(proc.memory_bytes)),
            Style::default().fg(Color::White),
        )));
    }

    let proc_panel = Paragraph::new(proc_lines);
    frame.render_widget(proc_panel, chunks[2]);
}

fn health_gauge_line_simple<'a>(label: &str, percent: f64, bar_width: usize) -> Line<'a> {
    let status = HealthStatus::from_percent(percent);
    let color = status_color(&status);
    let filled = ((percent / 100.0) * bar_width as f64).round() as usize;
    let empty = bar_width.saturating_sub(filled);

    Line::from(vec![
        Span::styled(format!("  {:<16}", label), Style::default().fg(Color::White)),
        Span::styled(
            format!("[{}{}] {:.0}% \u{2014} {}", "\u{2588}".repeat(filled), "\u{2591}".repeat(empty), percent, plain_language_cpu(percent as f32)),
            Style::default().fg(color),
        ),
    ])
}

fn truncate_str(s: &str, max: usize) -> String {
    if max < 3 { return s.chars().take(max).collect(); }
    if s.chars().count() <= max { s.to_string() } else {
        let truncated: String = s.chars().take(max - 2).collect();
        format!("{}..", truncated)
    }
}
