use crate::{app::App, types::Section, ui::common::*};
use ratatui::{
    layout::{Constraint, Layout, Rect},
    style::{Modifier, Style},
    widgets::Paragraph,
    Frame,
};
pub fn render(frame: &mut Frame, app: &App, area: Rect) {
    let panes = Layout::horizontal([Constraint::Ratio(1, 9); 9]).split(area);
    for (i, s) in Section::ALL.iter().enumerate() {
        let style = if *s == app.current_section {
            Style::default()
                .fg(COLOR_ACCENT)
                .bg(COLOR_HIGHLIGHT_BG)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(COLOR_MUTED)
        };
        let label = if area.width < 100 {
            [
                "Home", "CPU", "Memory", "Disk", "GPU", "Net", "Apps", "Temps", "Devices",
            ][i]
        } else {
            s.label()
        };
        frame.render_widget(
            Paragraph::new(format!("{} {}", s.number(), label)).style(style),
            panes[i],
        );
    }
}
