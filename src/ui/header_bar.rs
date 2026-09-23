use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

use crate::app::App;
use crate::types::DiagnosticMode;
use crate::ui::common::*;

pub fn render(frame: &mut Frame, app: &App, area: Rect) {
    let mode = app.mode.unwrap_or(DiagnosticMode::User);

    // Clock
    let now = chrono_free_time();

    // Mode badge
    let (mode_label, mode_fg, mode_bg) = match mode {
        DiagnosticMode::User => ("User Mode", Color::Rgb(20, 20, 20), COLOR_GOOD),
        DiagnosticMode::Technician => ("Tech Mode", Color::Rgb(20, 20, 20), COLOR_WARN),
    };

    // Right side: mode badge + clock
    let right_text = format!(" {} ", mode_label);
    let clock_text = format!("  {}", now);
    let right_len = right_text.len() + clock_text.len();
    let pad_len = (area.width as usize).saturating_sub(26 + right_len);

    let title_line = Line::from(vec![
        Span::styled(
            " SD-300 System Diagnostic",
            Style::default()
                .fg(COLOR_ACCENT)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(" ".repeat(pad_len)),
        Span::styled(
            right_text,
            Style::default()
                .fg(mode_fg)
                .bg(mode_bg)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(clock_text, Style::default().fg(COLOR_MUTED)),
    ]);

    let lane = match app.current_section {
        crate::types::Section::Disk
        | crate::types::Section::Gpu
        | crate::types::Section::Thermals => "slow",
        crate::types::Section::Network => "diagnostics",
        crate::types::Section::Drivers => "drivers",
        _ => "fast",
    };
    let (status, color) = if let Some(sample) = app.snapshot.samples.get(lane) {
        if !sample.observation.is_available() {
            (
                format!(
                    " {} · {} · r retry",
                    lane,
                    sample
                        .observation
                        .detail
                        .as_deref()
                        .unwrap_or("Unavailable")
                ),
                COLOR_WARN,
            )
        } else if sample.is_stale() {
            (
                format!(
                    " {} · stale · last capture {}s ago · r retry",
                    lane,
                    sample.age_ms().unwrap_or(0) / 1000
                ),
                COLOR_WARN,
            )
        } else {
            (
                format!(
                    " {} · live · sample {} · {}s since capture",
                    lane,
                    sample.sequence,
                    sample.age_ms().unwrap_or(0) / 1000
                ),
                COLOR_MUTED,
            )
        }
    } else {
        (format!(" {} · waiting for first sample", lane), COLOR_MUTED)
    };
    let separator_line = Line::from(Span::styled(status, Style::default().fg(color)));

    let paragraph = Paragraph::new(vec![title_line, separator_line]);
    frame.render_widget(paragraph, area);
}

/// Get current time as HH:MM:SS (without chrono dependency)
fn chrono_free_time() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let secs_of_day = secs % 86400;
    let h = secs_of_day / 3600;
    let m = (secs_of_day % 3600) / 60;
    let s = secs_of_day % 60;
    format!("{:02}:{:02}:{:02}", h, m, s)
}
