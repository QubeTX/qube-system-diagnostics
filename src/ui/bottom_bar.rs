use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

use crate::app::App;
use crate::types::Section;
use crate::ui::common::*;

pub fn render(frame: &mut Frame, app: &App, area: Rect) {
    let mut spans = Vec::new();

    for (i, section) in Section::ALL.iter().enumerate() {
        let is_active = app.current_section == *section;

        let label = format!(" {} {} ", section.number(), section.label());

        let style = if is_active {
            Style::default()
                .fg(COLOR_ACCENT)
                .bg(COLOR_HIGHLIGHT_BG)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(COLOR_MUTED)
        };

        spans.push(Span::styled(label, style));

        // Separator between tabs (not after last)
        if i < Section::ALL.len() - 1 {
            spans.push(Span::styled("\u{2502}", Style::default().fg(COLOR_BORDER)));
        }
    }

    // Right-align help hint
    let tabs_len: usize = spans.iter().map(|s| s.content.len()).sum();
    let remaining = (area.width as usize).saturating_sub(tabs_len + 8);
    if remaining > 0 {
        spans.push(Span::raw(" ".repeat(remaining)));
    }
    spans.push(Span::styled(
        " ? Help ",
        Style::default().fg(COLOR_DIM),
    ));

    let line = Line::from(spans);
    let paragraph = Paragraph::new(line);
    frame.render_widget(paragraph, area);
}
