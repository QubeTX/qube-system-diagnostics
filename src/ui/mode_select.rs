use ratatui::layout::{Alignment, Constraint, Flex, Layout};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, Paragraph};
use ratatui::Frame;

use crate::ui::common::*;

pub fn render(frame: &mut Frame) {
    let area = frame.area();

    // Center the selection box
    let [center_y] = Layout::vertical([Constraint::Length(18)])
        .flex(Flex::Center)
        .areas(area);
    let [center] = Layout::horizontal([Constraint::Length(60)])
        .flex(Flex::Center)
        .areas(center_y);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(COLOR_BORDER))
        .title_alignment(Alignment::Center);

    let lines = vec![
        Line::from(""),
        Line::from(Span::styled(
            "SD-300 SYSTEM DIAGNOSTIC",
            Style::default()
                .fg(COLOR_ACCENT)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(Span::styled(
            "QubeTX Developer Tools",
            Style::default().fg(COLOR_DIM),
        )),
        Line::from(""),
        Line::from(Span::styled(
            "Select a diagnostic mode:",
            Style::default().fg(COLOR_TEXT),
        )),
        Line::from(""),
        Line::from(vec![
            Span::styled("  [1]  ", Style::default().fg(COLOR_GOOD).bold()),
            Span::styled("User Mode", Style::default().fg(COLOR_TEXT).bold()),
        ]),
        Line::from(Span::styled(
            "       Plain language system health overview.",
            Style::default().fg(COLOR_DIM),
        )),
        Line::from(Span::styled(
            "       Designed for everyday users.",
            Style::default().fg(COLOR_DIM),
        )),
        Line::from(""),
        Line::from(vec![
            Span::styled("  [2]  ", Style::default().fg(COLOR_WARN).bold()),
            Span::styled("Technician Mode", Style::default().fg(COLOR_TEXT).bold()),
        ]),
        Line::from(Span::styled(
            "       Advanced metrics and raw system data.",
            Style::default().fg(COLOR_DIM),
        )),
        Line::from(Span::styled(
            "       Designed for IT professionals.",
            Style::default().fg(COLOR_DIM),
        )),
        Line::from(""),
        Line::from(Span::styled(
            "Press 1 or 2 to continue.  q to quit.",
            Style::default().fg(COLOR_MUTED),
        )),
    ];

    let paragraph = Paragraph::new(lines)
        .block(block)
        .alignment(Alignment::Center);

    frame.render_widget(paragraph, center);
}
