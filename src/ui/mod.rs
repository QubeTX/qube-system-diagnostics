pub mod bottom_bar;
pub mod common;
pub mod header_bar;
pub mod help_overlay;
pub mod mode_select;
pub mod sections;

use ratatui::Frame;

use crate::app::App;

/// Root render dispatcher
pub fn render(frame: &mut Frame, app: &App) {
    // Terminal too small
    if app.too_small {
        render_too_small(frame);
        return;
    }

    // Mode selection screen
    if app.mode.is_none() {
        mode_select::render(frame);
        return;
    }

    // Main layout: header + content + bottom bar
    let area = frame.area();
    let chunks = ratatui::layout::Layout::default()
        .direction(ratatui::layout::Direction::Vertical)
        .constraints([
            ratatui::layout::Constraint::Length(2),  // Header bar
            ratatui::layout::Constraint::Min(1),     // Content area
            ratatui::layout::Constraint::Length(1),  // Bottom bar
        ])
        .split(area);

    // Render header bar
    header_bar::render(frame, app, chunks[0]);

    // Render active section
    sections::render(frame, app, chunks[1]);

    // Render bottom navigation bar
    bottom_bar::render(frame, app, chunks[2]);

    // Help overlay (on top of everything)
    if app.show_help {
        help_overlay::render(frame, area);
    }
}

fn render_too_small(frame: &mut Frame) {
    use ratatui::layout::{Alignment, Constraint, Flex, Layout};
    use ratatui::style::Style;
    use ratatui::text::{Line, Span};
    use ratatui::widgets::Paragraph;
    use crate::ui::common::{COLOR_WARN, COLOR_MUTED};

    let area = frame.area();
    let [center_y] = Layout::vertical([Constraint::Length(3)])
        .flex(Flex::Center)
        .areas(area);
    let [center] = Layout::horizontal([Constraint::Length(40)])
        .flex(Flex::Center)
        .areas(center_y);

    let text = vec![
        Line::from(Span::styled(
            "Terminal too small",
            Style::default().fg(COLOR_WARN),
        )),
        Line::from(""),
        Line::from(Span::styled(
            "Please resize to at least 80x24",
            Style::default().fg(COLOR_MUTED),
        )),
    ];

    let paragraph = Paragraph::new(text).alignment(Alignment::Center);
    frame.render_widget(paragraph, center);
}
