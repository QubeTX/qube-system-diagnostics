use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Sparkline};
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
            Constraint::Length(2),
            Constraint::Length(7),
            Constraint::Length(9),
            Constraint::Min(6),
        ])
        .split(area);

    let header = Paragraph::new(Line::from(Span::styled(
        "  MEMORY",
        Style::default().fg(COLOR_HEADER).add_modifier(Modifier::BOLD),
    )));
    frame.render_widget(header, chunks[0]);

    let mem = &app.snapshot.memory;
    let pct = mem.usage_percent();
    let status = HealthStatus::from_percent(pct);
    let used_gb = mem.used_bytes as f64 / (1024.0 * 1024.0 * 1024.0);
    let total_gb = mem.total_bytes as f64 / (1024.0 * 1024.0 * 1024.0);

    let mut status_lines = vec![
        separator(area.width as usize),
        Line::from(vec![
            Span::styled(format!("  {} ", status.icon()), Style::default().fg(status_color(&status))),
            Span::styled(
                format!("Using {:.0} of {:.0} GB ({:.0}%)", used_gb, total_gb, pct),
                Style::default().fg(Color::White),
            ),
        ]),
        gauge_line("Usage", pct, 20),
        Line::from(vec![
            Span::styled("  Status          ", Style::default().fg(Color::White)),
            Span::styled(plain_language_percent(pct, "memory"), Style::default().fg(COLOR_DIM)),
        ]),
    ];

    if mem.swap_used_bytes > 0 {
        status_lines.push(Line::from(vec![
            Span::styled("  Swap            ", Style::default().fg(Color::White)),
            Span::styled("Your computer is using extra temporary storage", Style::default().fg(COLOR_DIM)),
        ]));
    }

    let status_panel = Paragraph::new(status_lines);
    frame.render_widget(status_panel, chunks[1]);

    // Sparkline
    let spark_data = app.mem_history.as_u64_vec();
    let sparkline_block = Block::default()
        .title("  MEMORY USAGE (last 60 seconds)")
        .title_style(Style::default().fg(COLOR_HEADER))
        .borders(Borders::NONE);
    let sparkline = Sparkline::default()
        .block(sparkline_block)
        .data(&spark_data)
        .max(100)
        .style(Style::default().fg(Color::Magenta));
    frame.render_widget(sparkline, chunks[2]);

    // Top consumers
    let mut consumer_lines = vec![
        separator(area.width as usize),
        section_header("WHAT'S USING THE MOST MEMORY"),
        Line::from(""),
    ];

    let mut mem_sorted: Vec<_> = app.snapshot.processes.list.clone();
    mem_sorted.sort_by(|a, b| b.memory_bytes.cmp(&a.memory_bytes));

    for proc in mem_sorted.iter().take(5) {
        consumer_lines.push(Line::from(vec![
            Span::styled(format!("  {:<24}", proc.friendly_name), Style::default().fg(Color::White)),
            Span::styled(format_bytes(proc.memory_bytes), Style::default().fg(COLOR_INFO)),
            Span::styled(
                format!("  ({:.1}%)", proc.memory_percent),
                Style::default().fg(COLOR_DIM),
            ),
        ]));
    }

    let consumer_panel = Paragraph::new(consumer_lines);
    frame.render_widget(consumer_panel, chunks[3]);
}

fn render_tech(frame: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Length(6),
            Constraint::Min(8),
        ])
        .split(area);

    let mem = &app.snapshot.memory;

    // Header
    let header_lines = vec![
        Line::from(Span::styled(
            format!("  MEMORY \u{2014} {} / {} ({:.1}%)",
                format_bytes_gib(mem.used_bytes), format_bytes_gib(mem.total_bytes), mem.usage_percent()),
            Style::default().fg(COLOR_HEADER).add_modifier(Modifier::BOLD),
        )),
        separator(area.width as usize),
        Line::from(vec![
            Span::styled("  RAM  ", Style::default().fg(COLOR_DIM)),
            Span::styled(gauge_bar(mem.usage_percent(), 20), Style::default().fg(status_color(&HealthStatus::from_percent(mem.usage_percent())))),
            Span::raw("    "),
            Span::styled("Swap ", Style::default().fg(COLOR_DIM)),
            Span::styled(gauge_bar(mem.swap_percent(), 20), Style::default().fg(status_color(&HealthStatus::from_percent(mem.swap_percent())))),
        ]),
    ];
    let header_panel = Paragraph::new(header_lines);
    frame.render_widget(header_panel, chunks[0]);

    // Sparklines side by side
    let spark_area = chunks[1];
    let spark_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(spark_area);

    let mem_spark = app.mem_history.as_u64_vec();
    let mem_sparkline = Sparkline::default()
        .block(Block::default()
            .title(" RAM Usage ")
            .title_style(Style::default().fg(COLOR_HEADER))
            .borders(Borders::ALL)
            .border_style(Style::default().fg(COLOR_DIM)))
        .data(&mem_spark)
        .max(100)
        .style(Style::default().fg(Color::Magenta));
    frame.render_widget(mem_sparkline, spark_chunks[0]);

    let swap_spark = app.swap_history.as_u64_vec();
    let swap_sparkline = Sparkline::default()
        .block(Block::default()
            .title(" Swap Usage ")
            .title_style(Style::default().fg(COLOR_HEADER))
            .borders(Borders::ALL)
            .border_style(Style::default().fg(COLOR_DIM)))
        .data(&swap_spark)
        .max(100)
        .style(Style::default().fg(Color::Yellow));
    frame.render_widget(swap_sparkline, spark_chunks[1]);

    // Process table
    let mut proc_lines = vec![
        separator(area.width as usize),
        Line::from(Span::styled(
            format!("  {:<28} {:>6} {:>8} {:>10} {:>10}", "TOP MEMORY CONSUMERS", "PID", "MEM%", "RSS", "CPU%"),
            Style::default().fg(COLOR_DIM),
        )),
    ];

    let mut mem_sorted: Vec<_> = app.snapshot.processes.list.clone();
    mem_sorted.sort_by(|a, b| b.memory_bytes.cmp(&a.memory_bytes));

    for proc in mem_sorted.iter().take(8) {
        proc_lines.push(Line::from(Span::styled(
            format!("  {:<28} {:>6} {:>7.1}% {:>10} {:>9.1}%",
                truncate_str(&proc.name, 28), proc.pid, proc.memory_percent, format_bytes(proc.memory_bytes), proc.cpu_percent),
            Style::default().fg(Color::White),
        )));
    }

    let proc_panel = Paragraph::new(proc_lines);
    frame.render_widget(proc_panel, chunks[2]);
}

fn truncate_str(s: &str, max: usize) -> String {
    if max < 3 { return s.chars().take(max).collect(); }
    if s.chars().count() <= max { s.to_string() } else {
        let truncated: String = s.chars().take(max - 2).collect();
        format!("{}..", truncated)
    }
}
