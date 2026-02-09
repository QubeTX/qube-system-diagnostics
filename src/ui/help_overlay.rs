use ratatui::layout::{Alignment, Constraint, Flex, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, Clear, Paragraph};
use ratatui::Frame;

use crate::ui::common::*;

pub fn render(frame: &mut Frame, area: Rect) {
    let [center_y] = Layout::vertical([Constraint::Length(23)])
        .flex(Flex::Center)
        .areas(area);
    let [center] = Layout::horizontal([Constraint::Length(50)])
        .flex(Flex::Center)
        .areas(center_y);

    // Clear the area behind the overlay
    frame.render_widget(Clear, center);

    let block = Block::default()
        .title(" Help \u{2014} Keybindings ")
        .title_alignment(Alignment::Center)
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(COLOR_BORDER))
        .title_style(Style::default().fg(COLOR_ACCENT).add_modifier(Modifier::BOLD));

    let lines = vec![
        Line::from(""),
        help_line("1-9", "Switch to section"),
        help_line("q / Esc", "Quit"),
        help_line("Ctrl+C", "Quit to shell"),
        help_line("m", "Mode selection screen"),
        help_line("?", "Toggle this help"),
        Line::from(""),
        Line::from(Span::styled(
            " Process Table (Section 7, Tech Mode)",
            Style::default().fg(COLOR_WARN),
        )),
        help_line("j / k", "Scroll up/down"),
        help_line("c", "Sort by CPU"),
        help_line("n", "Sort by name"),
        help_line("p", "Sort by PID"),
        Line::from(""),
        Line::from(Span::styled(
            " Connections (Section 6, Tech Mode)",
            Style::default().fg(COLOR_WARN),
        )),
        help_line("j / k", "Scroll connections"),
        Line::from(""),
        help_line("f", "Toggle \u{00B0}C / \u{00B0}F"),
        help_line("r", "Refresh drivers (Section 9)"),
        Line::from(""),
        Line::from(Span::styled(
            "  Press ? or Esc to close",
            Style::default().fg(COLOR_MUTED),
        )),
    ];

    let paragraph = Paragraph::new(lines).block(block);
    frame.render_widget(paragraph, center);
}

fn help_line(key: &str, desc: &str) -> Line<'static> {
    Line::from(vec![
        Span::styled(
            format!("  {:>10}  ", key),
            Style::default()
                .fg(COLOR_ACCENT)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(desc.to_string(), Style::default().fg(COLOR_TEXT)),
    ])
}
