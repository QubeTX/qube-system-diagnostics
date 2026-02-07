use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

use crate::app::App;
use crate::types::{DiagnosticMode, ProcessSortKey};
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
            "  RUNNING APPS",
            Style::default().fg(COLOR_HEADER).add_modifier(Modifier::BOLD),
        )),
        separator(area.width as usize),
        Line::from(""),
        Line::from(Span::styled(
            format!("  {} things running on your computer", app.snapshot.processes.total_count),
            Style::default().fg(Color::White),
        )),
        Line::from(""),
    ];

    for proc in app.snapshot.processes.list.iter().take(10) {
        let descriptor = if proc.cpu_percent > 20.0 {
            "Using a lot of processor"
        } else if proc.memory_percent > 5.0 {
            "Using a lot of memory"
        } else if proc.cpu_percent > 5.0 {
            "Using some processor"
        } else {
            "Running quietly"
        };

        lines.push(Line::from(vec![
            Span::styled(format!("  {:<24}", proc.friendly_name), Style::default().fg(Color::White)),
            Span::styled(descriptor.to_string(), Style::default().fg(COLOR_DIM)),
        ]));
    }

    let panel = Paragraph::new(lines);
    frame.render_widget(panel, area);
}

fn render_tech(frame: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(4),
            Constraint::Min(6),
        ])
        .split(area);

    // Header with sort indicator
    let sort_indicator = match app.process_sort {
        ProcessSortKey::Cpu => "CPU%",
        ProcessSortKey::Memory => "MEM%",
        ProcessSortKey::Pid => "PID",
        ProcessSortKey::Name => "Name",
    };

    let header_lines = vec![
        Line::from(Span::styled(
            format!("  PROCESSES \u{2014} {} processes  (sorted by {})",
                app.snapshot.processes.total_count, sort_indicator),
            Style::default().fg(COLOR_HEADER).add_modifier(Modifier::BOLD),
        )),
        separator(area.width as usize),
        Line::from(Span::styled(
            "  Sort: [c]pu  [m]emory  [p]id  [n]ame    Scroll: j/k or arrows",
            Style::default().fg(COLOR_DIM),
        )),
        Line::from(Span::styled(
            format!("  {:<28} {:>6} {:>8} {:>8} {:>10} {:>8}", "NAME", "PID", "CPU%", "MEM%", "MEMORY", "STATUS"),
            Style::default().fg(COLOR_DIM).add_modifier(Modifier::BOLD),
        )),
    ];
    let header_panel = Paragraph::new(header_lines);
    frame.render_widget(header_panel, chunks[0]);

    // Sort processes
    let mut sorted_procs = app.snapshot.processes.list.clone();
    match app.process_sort {
        ProcessSortKey::Cpu => sorted_procs.sort_by(|a, b| b.cpu_percent.partial_cmp(&a.cpu_percent).unwrap_or(std::cmp::Ordering::Equal)),
        ProcessSortKey::Memory => sorted_procs.sort_by(|a, b| b.memory_bytes.cmp(&a.memory_bytes)),
        ProcessSortKey::Pid => sorted_procs.sort_by(|a, b| a.pid.cmp(&b.pid)),
        ProcessSortKey::Name => sorted_procs.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase())),
    }

    // Visible rows
    let visible_height = chunks[1].height as usize;
    let scroll = app.process_scroll.min(sorted_procs.len().saturating_sub(visible_height));

    let mut proc_lines = Vec::new();
    for (i, proc) in sorted_procs.iter().skip(scroll).take(visible_height).enumerate() {
        let _row_idx = i; // Available for future highlighting

        let style = if proc.cpu_percent > 50.0 {
            Style::default().fg(Color::Red)
        } else if proc.cpu_percent > 20.0 {
            Style::default().fg(Color::Yellow)
        } else {
            Style::default().fg(Color::White)
        };

        proc_lines.push(Line::from(Span::styled(
            format!("  {:<28} {:>6} {:>7.1}% {:>7.1}% {:>10} {:>8}",
                truncate_str(&proc.name, 28),
                proc.pid,
                proc.cpu_percent,
                proc.memory_percent,
                format_bytes(proc.memory_bytes),
                truncate_str(&proc.status, 8)),
            style,
        )));
    }

    let proc_panel = Paragraph::new(proc_lines);
    frame.render_widget(proc_panel, chunks[1]);
}

fn truncate_str(s: &str, max: usize) -> String {
    if max < 3 { return s.chars().take(max).collect(); }
    if s.chars().count() <= max { s.to_string() } else {
        let truncated: String = s.chars().take(max - 2).collect();
        format!("{}..", truncated)
    }
}
