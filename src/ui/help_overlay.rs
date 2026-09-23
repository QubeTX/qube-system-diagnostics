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
    let [center] = Layout::horizontal([Constraint::Length(64)])
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
        .title_style(
            Style::default()
                .fg(COLOR_ACCENT)
                .add_modifier(Modifier::BOLD),
        );

    let lines = vec![
        help_line("1-9 / Tab", "Sections / next section"),
        help_line("q / Ctrl+C", "Quit to shell"),
        help_line("Esc", "Close panel / clear filter / quit"),
        help_line("m", "Choose User or Technician"),
        help_line("?", "Help"),
        help_line("j/k / ↑↓", "Select rows; scroll inspector"),
        help_line("PgUp/PgDn", "Page through all rows"),
        help_line("Home / End", "First / last row"),
        help_line("/", "Filter the complete inventory"),
        help_line("Enter", "Open / close row inspector"),
        help_line("Space", "Freeze view / resume latest"),
        help_line("F", "Findings and evidence"),
        help_line("c / M", "Process sort: CPU / memory"),
        help_line("n / p / s", "Name / PID / reverse sort"),
        help_line("f", "Toggle temperature unit"),
        help_line("r", "Retry providers and discovery"),
        Line::from(""),
        Line::from(" Charts: dots are gaps; underscores are zero."),
        Line::from(" Collection continues while the view is paused."),
        Line::from(" Preferences: settings.json / tui namespace."),
        Line::from(" Press ? or Esc to close."),
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
