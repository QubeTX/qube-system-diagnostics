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
    let outer = content_block("Memory");
    let inner = outer.inner(area);
    frame.render_widget(outer, area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(6),
            Constraint::Length(8),
            Constraint::Min(5),
        ])
        .split(inner);

    let mem = &app.snapshot.memory;
    let pct = mem.usage_percent();
    let status = HealthStatus::from_percent(pct);
    let used_gb = mem.used_bytes as f64 / (1024.0 * 1024.0 * 1024.0);
    let total_gb = mem.total_bytes as f64 / (1024.0 * 1024.0 * 1024.0);

    let mut status_lines = vec![
        Line::from(""),
        Line::from(vec![
            Span::styled(format!("  {} ", status.icon()), Style::default().fg(status_color(&status))),
            Span::styled(
                format!("Using {:.0} of {:.0} GB ({:.0}%)", used_gb, total_gb, pct),
                Style::default().fg(COLOR_TEXT),
            ),
        ]),
        gauge_line("Usage", pct, 20),
        Line::from(vec![
            Span::styled("  Status          ", Style::default().fg(COLOR_TEXT)),
            Span::styled(plain_language_percent(pct, "memory"), Style::default().fg(COLOR_DIM)),
        ]),
    ];

    if mem.swap_used_bytes > 0 {
        status_lines.push(Line::from(vec![
            Span::styled("  Swap            ", Style::default().fg(COLOR_TEXT)),
            Span::styled("Your computer is using extra temporary storage", Style::default().fg(COLOR_DIM)),
        ]));
    }

    let status_panel = Paragraph::new(status_lines);
    frame.render_widget(status_panel, chunks[0]);

    // Sparkline
    let spark_data = app.mem_history.as_u64_vec();
    let sparkline = Sparkline::default()
        .block(sub_block("Usage \u{2014} Last 60 Seconds"))
        .data(&spark_data)
        .max(100)
        .style(Style::default().fg(SPARK_MEMORY));
    frame.render_widget(sparkline, chunks[1]);

    // Top consumers
    let consumer_block = sub_block("What's Using the Most Memory");
    let consumer_inner = consumer_block.inner(chunks[2]);
    frame.render_widget(consumer_block, chunks[2]);

    let mut consumer_lines = Vec::new();
    let mut mem_sorted: Vec<_> = app.snapshot.processes.list.clone();
    mem_sorted.sort_by(|a, b| b.memory_bytes.cmp(&a.memory_bytes));

    for proc in mem_sorted.iter().take(5) {
        consumer_lines.push(Line::from(vec![
            Span::styled(format!("  {:<24}", proc.friendly_name), Style::default().fg(COLOR_TEXT)),
            Span::styled(format_bytes(proc.memory_bytes), Style::default().fg(COLOR_INFO)),
            Span::styled(
                format!("  ({:.1}%)", proc.memory_percent),
                Style::default().fg(COLOR_DIM),
            ),
        ]));
    }

    let consumer_panel = Paragraph::new(consumer_lines);
    frame.render_widget(consumer_panel, consumer_inner);
}

fn render_tech(frame: &mut Frame, app: &App, area: Rect) {
    let mem = &app.snapshot.memory;
    let outer = content_block(&format!("Memory \u{2014} {} / {} ({:.1}%)",
        format_bytes_gib(mem.used_bytes), format_bytes_gib(mem.total_bytes), mem.usage_percent()));
    let inner = outer.inner(area);
    frame.render_widget(outer, area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(2),
            Constraint::Length(6),
            Constraint::Min(6),
        ])
        .split(inner);

    // Gauge row
    let gauge_lines = vec![
        Line::from(vec![
            Span::styled("  RAM  ", Style::default().fg(COLOR_DIM)),
            Span::styled(gauge_bar(mem.usage_percent(), 20), Style::default().fg(status_color(&HealthStatus::from_percent(mem.usage_percent())))),
            Span::raw("    "),
            Span::styled("Swap ", Style::default().fg(COLOR_DIM)),
            Span::styled(gauge_bar(mem.swap_percent(), 20), Style::default().fg(status_color(&HealthStatus::from_percent(mem.swap_percent())))),
        ]),
    ];
    let gauge_panel = Paragraph::new(gauge_lines);
    frame.render_widget(gauge_panel, chunks[0]);

    // Sparklines side by side
    let spark_area = chunks[1];
    let spark_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(spark_area);

    let mem_spark = app.mem_history.as_u64_vec();
    let mem_sparkline = Sparkline::default()
        .block(sub_block("RAM Usage"))
        .data(&mem_spark)
        .max(100)
        .style(Style::default().fg(SPARK_MEMORY));
    frame.render_widget(mem_sparkline, spark_chunks[0]);

    let swap_spark = app.swap_history.as_u64_vec();
    let swap_sparkline = Sparkline::default()
        .block(sub_block("Swap Usage"))
        .data(&swap_spark)
        .max(100)
        .style(Style::default().fg(COLOR_WARN));
    frame.render_widget(swap_sparkline, spark_chunks[1]);

    // Process table
    let proc_block = sub_block("Top Memory Consumers");
    let proc_inner = proc_block.inner(chunks[2]);
    frame.render_widget(proc_block, chunks[2]);

    let mut proc_lines = vec![
        Line::from(Span::styled(
            format!("  {:<28} {:>6} {:>8} {:>10} {:>10}", "PROCESS", "PID", "MEM%", "RSS", "CPU%"),
            Style::default().fg(COLOR_DIM),
        )),
    ];

    let mut mem_sorted: Vec<_> = app.snapshot.processes.list.clone();
    mem_sorted.sort_by(|a, b| b.memory_bytes.cmp(&a.memory_bytes));

    for proc in mem_sorted.iter().take(8) {
        proc_lines.push(Line::from(Span::styled(
            format!("  {:<28} {:>6} {:>7.1}% {:>10} {:>9.1}%",
                truncate_str(&proc.name, 28), proc.pid, proc.memory_percent, format_bytes(proc.memory_bytes), proc.cpu_percent),
            Style::default().fg(COLOR_TEXT),
        )));
    }

    let proc_panel = Paragraph::new(proc_lines);
    frame.render_widget(proc_panel, proc_inner);
}
