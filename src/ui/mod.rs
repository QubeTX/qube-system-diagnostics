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
        mode_select::render(frame, app.cargo_gui_completion_notice);
        return;
    }

    // Main layout: header + content + bottom bar
    let area = frame.area();
    let chunks = ratatui::layout::Layout::default()
        .direction(ratatui::layout::Direction::Vertical)
        .constraints([
            ratatui::layout::Constraint::Length(2), // Header bar
            ratatui::layout::Constraint::Min(1),    // Content area
            ratatui::layout::Constraint::Length(1), // Bottom bar
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
    use crate::ui::common::{COLOR_MUTED, COLOR_WARN};
    use ratatui::layout::{Alignment, Constraint, Flex, Layout};
    use ratatui::style::Style;
    use ratatui::text::{Line, Span};
    use ratatui::widgets::Paragraph;

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{DiagnosticMode, Section};
    use ratatui::{backend::TestBackend, Terminal};

    #[test]
    fn renders_every_populated_section_in_both_modes_at_minimum_size() {
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut app = App::new(Some(DiagnosticMode::User));
        app.snapshot.refresh_static();
        app.snapshot.refresh_fast();
        app.snapshot.refresh_slow();
        app.snapshot.refresh_connections();

        for mode in [DiagnosticMode::User, DiagnosticMode::Technician] {
            app.mode = Some(mode);
            for section in Section::ALL {
                app.current_section = section;
                terminal.draw(|frame| render(frame, &app)).unwrap();
                let buffer = terminal.backend().buffer();
                assert!(
                    buffer.content().iter().any(|cell| cell.symbol() != " "),
                    "{mode:?} {section:?} rendered a blank frame"
                );
            }
        }
    }

    #[test]
    fn renders_small_terminal_guard() {
        let backend = TestBackend::new(60, 18);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut app = App::new(Some(DiagnosticMode::Technician));
        app.too_small = true;
        terminal.draw(|frame| render(frame, &app)).unwrap();
        let rendered = terminal
            .backend()
            .buffer()
            .content()
            .iter()
            .map(|cell| cell.symbol())
            .collect::<String>();
        assert!(rendered.contains("Terminal too small"));
    }

    #[test]
    fn thermal_views_keep_gpu_data_when_dell_cpu_access_is_permission_gated() {
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut app = App::new(Some(DiagnosticMode::User));
        app.snapshot.thermals.gpu_temp = Some(53.0);
        app.snapshot.thermals.temperature_status =
            crate::observation::Observation::available("nvidia-smi");
        app.snapshot.thermals.gpu_temperature_status =
            crate::observation::Observation::available("nvidia-smi");
        app.snapshot.thermals.cpu_temperature_status =
            crate::observation::Observation::permission_denied(
                "Dell AWCC",
                "Administrator required",
            );
        app.snapshot.thermals.fan_status = crate::observation::Observation::permission_denied(
            "Dell AWCC",
            "Administrator required",
        );

        for section in [Section::Overview, Section::Thermals] {
            app.current_section = section;
            terminal.draw(|frame| render(frame, &app)).unwrap();
            let rendered = terminal
                .backend()
                .buffer()
                .content()
                .iter()
                .map(|cell| cell.symbol())
                .collect::<String>();
            assert!(rendered.contains("Graphics"));
            assert!(rendered.contains("Administrator"));
            assert!(!rendered.contains("Thermals not supported"));
        }
    }

    #[test]
    fn cargo_completion_notice_is_confined_to_the_intermediate_chooser_state() {
        let mut plain_terminal = Terminal::new(TestBackend::new(80, 24)).unwrap();
        let plain = App::new(None);
        plain_terminal.draw(|frame| render(frame, &plain)).unwrap();
        let plain_text = plain_terminal
            .backend()
            .buffer()
            .content()
            .iter()
            .map(|cell| cell.symbol())
            .collect::<String>();
        assert!(!plain_text.contains("Desktop app pending"));

        let mut pending_terminal = Terminal::new(TestBackend::new(80, 24)).unwrap();
        let mut pending = App::new(None);
        pending.cargo_gui_completion_notice = true;
        pending_terminal
            .draw(|frame| render(frame, &pending))
            .unwrap();
        let pending_text = pending_terminal
            .backend()
            .buffer()
            .content()
            .iter()
            .map(|cell| cell.symbol())
            .collect::<String>();
        assert!(pending_text.contains("Desktop app pending"));

        pending.mode = Some(DiagnosticMode::User);
        pending_terminal
            .draw(|frame| render(frame, &pending))
            .unwrap();
        let user_text = pending_terminal
            .backend()
            .buffer()
            .content()
            .iter()
            .map(|cell| cell.symbol())
            .collect::<String>();
        assert!(!user_text.contains("Desktop app pending"));
    }
}
