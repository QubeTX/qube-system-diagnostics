use ratatui::layout::{Alignment, Constraint, Flex, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, Clear, Paragraph};
use ratatui::Frame;

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
        .title(" Help — Keybindings ")
        .title_alignment(Alignment::Center)
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Color::Cyan));

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
            Style::default().fg(Color::Yellow),
        )),
        help_line("j / k", "Scroll up/down"),
        help_line("c", "Sort by CPU"),
        help_line("n", "Sort by name"),
        help_line("p", "Sort by PID"),
        Line::from(""),
        Line::from(Span::styled(
            " Connections (Section 6, Tech Mode)",
            Style::default().fg(Color::Yellow),
        )),
        help_line("j / k", "Scroll connections"),
        Line::from(""),
        help_line("f", "Toggle \u{00B0}C / \u{00B0}F"),
        help_line("r", "Refresh drivers (Section 9)"),
        Line::from(""),
        Line::from(Span::styled(
            "  Press ? or Esc to close",
            Style::default().fg(Color::DarkGray),
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
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(desc.to_string(), Style::default().fg(Color::White)),
    ])
}
