use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
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
    let outer = content_block(&format!("Running Apps \u{2014} {} total", app.snapshot.processes.total_count));
    let inner = outer.inner(area);
    frame.render_widget(outer, area);

    let mut lines = vec![Line::from("")];

    for proc in app.snapshot.processes.list.iter().take(15) {
        let (dot_color, descriptor) = if proc.cpu_percent > 20.0 {
            (COLOR_CRIT, "Using a lot of processor")
        } else if proc.memory_percent > 5.0 {
            (COLOR_WARN, "Using a lot of memory")
        } else if proc.cpu_percent > 5.0 {
            (COLOR_WARN, "Using some processor")
        } else {
            (COLOR_GOOD, "Running quietly")
        };

        lines.push(Line::from(vec![
            Span::styled("  \u{2022} ", Style::default().fg(dot_color)),
            Span::styled(format!("{:<22}", proc.friendly_name), Style::default().fg(COLOR_TEXT)),
            Span::styled(descriptor.to_string(), Style::default().fg(COLOR_DIM)),
        ]));
    }

    let panel = Paragraph::new(lines);
    frame.render_widget(panel, inner);
}

fn render_tech(frame: &mut Frame, app: &App, area: Rect) {
    let outer = content_block(&format!("Processes \u{2014} {} total", app.snapshot.processes.total_count));
    let inner = outer.inner(area);
    frame.render_widget(outer, area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(6),
        ])
        .split(inner);

    // Header with sort indicator
    let sort_indicator = match app.process_sort {
        ProcessSortKey::Cpu => "CPU%",
        ProcessSortKey::Memory => "MEM%",
        ProcessSortKey::Pid => "PID",
        ProcessSortKey::Name => "Name",
    };

    let header_lines = vec![
        Line::from(Span::styled(
            format!("  Sorted by {}    Sort: [c]pu  [m]emory  [p]id  [n]ame    Scroll: j/k or arrows", sort_indicator),
            Style::default().fg(COLOR_MUTED),
        )),
        Line::from(""),
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

    // Visible rows (reserve 1 line for scroll indicator)
    let visible_height = chunks[1].height.saturating_sub(1) as usize;
    let scroll = app.process_scroll.min(sorted_procs.len().saturating_sub(visible_height));
    let total = sorted_procs.len();

    let mut proc_lines = Vec::new();
    for proc in sorted_procs.iter().skip(scroll).take(visible_height) {
        let style = if proc.cpu_percent > 50.0 {
            Style::default().fg(COLOR_CRIT)
        } else if proc.cpu_percent > 20.0 {
            Style::default().fg(COLOR_WARN)
        } else {
            Style::default().fg(COLOR_TEXT)
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

    // Scroll indicator
    let end = (scroll + visible_height).min(total);
    proc_lines.push(Line::from(Span::styled(
        format!("  Showing {}-{} of {}", scroll + 1, end, total),
        Style::default().fg(COLOR_DIM),
    )));

    let proc_panel = Paragraph::new(proc_lines);
    frame.render_widget(proc_panel, chunks[1]);
}
