use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Paragraph, Tabs};
use ratatui::Frame;

use crate::app::App;
use crate::types::Section;
use crate::ui::common::*;

pub fn render(frame: &mut Frame, app: &App, area: Rect) {
    let titles = Section::ALL.iter().map(|section| {
        Line::from(Span::raw(format!(
            " {} {} ",
            section.number(),
            section.label()
        )))
    });

    let selected = Section::ALL
        .iter()
        .position(|section| *section == app.current_section)
        .unwrap_or(0);
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Min(10), Constraint::Length(8)])
        .split(area);

    let tabs = Tabs::new(titles)
        .select(selected)
        .style(Style::default().fg(COLOR_MUTED))
        .highlight_style(
            Style::default()
                .fg(COLOR_ACCENT)
                .bg(COLOR_HIGHLIGHT_BG)
                .add_modifier(Modifier::BOLD),
        )
        .divider(Span::styled("\u{2502}", Style::default().fg(COLOR_BORDER)))
        .padding("", "");

    frame.render_widget(tabs, chunks[0]);
    frame.render_widget(
        Paragraph::new(Line::from(Span::styled(
            "? Help",
            Style::default().fg(COLOR_DIM),
        )))
        .alignment(Alignment::Right),
        chunks[1],
    );
}
