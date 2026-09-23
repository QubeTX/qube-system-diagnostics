use crate::{app::App, ui::common::*};
use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Clear, Paragraph, Widget, Wrap},
    Frame,
};

pub fn render(frame: &mut Frame, app: &App, area: Rect) {
    let parts = Layout::vertical([
        Constraint::Length(4),
        Constraint::Length(4),
        Constraint::Min(3),
        Constraint::Length(1),
    ])
    .split(area);
    frame.render_widget(
        Paragraph::new(
            app.view
                .summary
                .iter()
                .map(|s| Line::from(s.as_str()))
                .collect::<Vec<_>>(),
        )
        .style(Style::default().fg(COLOR_TEXT))
        .block(content_block(app.current_section.label())),
        parts[0],
    );
    let wide = area.width >= 120;
    let charts = Layout::horizontal(if wide {
        vec![Constraint::Percentage(50); 2]
    } else {
        vec![Constraint::Percentage(100)]
    })
    .split(parts[1]);
    frame.render_widget(
        TimePlot {
            values: &app.view.plot,
            max: app.view.plot_max,
            block: sub_block(&app.view.plot_title),
            ascii: app.preferences.ascii,
        },
        charts[0],
    );
    if wide {
        frame.render_widget(
            TimePlot {
                values: &app.view.secondary_plot,
                max: app.view.secondary_max,
                block: sub_block(&app.view.secondary_title),
                ascii: app.preferences.ascii,
            },
            charts[1],
        );
    }
    let panes = Layout::horizontal(if wide {
        vec![Constraint::Percentage(60), Constraint::Percentage(40)]
    } else {
        vec![Constraint::Percentage(100)]
    })
    .split(parts[2]);
    render_rows(frame, app, panes[0]);
    if wide {
        render_inspector(frame, app, panes[1]);
    }
    if app.show_inspector && !wide {
        frame.render_widget(Clear, parts[2]);
        render_inspector(frame, app, parts[2]);
    }
    let controls = if app.editing_filter {
        format!(" /{}█  Enter apply · Esc clear", app.filter)
    } else {
        format!(
            " / filter{}   Enter inspect   Space {}   F findings   ? help",
            if app.filter.is_empty() {
                String::new()
            } else {
                format!(": {}", app.filter)
            },
            if app.paused_at.is_some() {
                "resume"
            } else {
                "pause"
            }
        )
    };
    frame.render_widget(
        Paragraph::new(controls).style(Style::default().fg(COLOR_MUTED)),
        parts[3],
    );
}
fn render_rows(frame: &mut Frame, app: &App, area: Rect) {
    let block = content_block(&format!(
        "{} of {} · ↑↓ PgUp/PgDn",
        app.view.rows.len(),
        app.view.total
    ));
    let inner = block.inner(area);
    frame.render_widget(block, area);
    let height = inner.height as usize;
    let start = app.view.selected.saturating_sub(height.saturating_sub(1));
    if app.view.rows.is_empty() {
        frame.render_widget(
            Paragraph::new(if app.filter.is_empty() {
                "No data from this provider yet. r retries; other sections remain available."
            } else {
                "No matching rows. / edits the filter; Esc clears it."
            })
            .wrap(Wrap { trim: true })
            .style(Style::default().fg(COLOR_MUTED)),
            inner,
        );
        return;
    }
    for (offset, row) in app.view.rows.iter().skip(start).take(height).enumerate() {
        let rect = Rect::new(inner.x, inner.y + offset as u16, inner.width, 1);
        let widths = Layout::horizontal([
            Constraint::Percentage(39),
            Constraint::Percentage(25),
            Constraint::Percentage(36),
        ])
        .split(rect);
        let selected = !app.view.selection_lost && start + offset == app.view.selected;
        let style = if selected {
            Style::default()
                .fg(COLOR_ACCENT)
                .bg(COLOR_HIGHLIGHT_BG)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(COLOR_TEXT)
        };
        frame.render_widget(
            Paragraph::new(format!("{} {}", if selected { ">" } else { " " }, row.name))
                .style(style),
            widths[0],
        );
        frame.render_widget(Paragraph::new(row.value.as_str()).style(style), widths[1]);
        frame.render_widget(
            Paragraph::new(row.state.as_str()).style(if selected {
                style
            } else {
                Style::default().fg(COLOR_MUTED)
            }),
            widths[2],
        );
    }
}
fn render_inspector(frame: &mut Frame, app: &App, area: Rect) {
    let mut lines = app
        .view
        .inspector
        .iter()
        .map(|s| Line::from(s.as_str()))
        .collect::<Vec<_>>();
    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        app.view.status.as_str(),
        Style::default().fg(COLOR_MUTED),
    )));
    frame.render_widget(
        Paragraph::new(lines)
            .wrap(Wrap { trim: true })
            .scroll((app.inspector_scroll, 0))
            .style(Style::default().fg(COLOR_TEXT))
            .block(content_block(if app.show_inspector {
                "Inspector · j/k scroll · Enter/Esc close"
            } else {
                "Inspector · Enter to focus"
            })),
        area,
    );
}
/// One column per time bucket. Dots are missing observations; a measured zero
/// uses an underscore. No line connects across missing captures.
struct TimePlot<'a> {
    values: &'a [Option<f64>],
    max: Option<f64>,
    block: Block<'a>,
    ascii: bool,
}
impl Widget for TimePlot<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let inner = self.block.inner(area);
        self.block.render(area, buf);
        if inner.is_empty() {
            return;
        }
        let values = &self.values[self.values.len().saturating_sub(inner.width as usize)..];
        let max = self
            .max
            .unwrap_or_else(|| values.iter().flatten().copied().fold(1.0, f64::max))
            .max(0.001);
        let start = inner.x + inner.width.saturating_sub(values.len() as u16);
        for (i, v) in values.iter().enumerate() {
            let x = start + i as u16;
            let bottom = inner.bottom() - 1;
            match v {
                None => {
                    buf[(x, bottom)]
                        .set_symbol(if self.ascii { "." } else { "·" })
                        .set_fg(COLOR_DIM);
                }
                Some(v) if *v <= 0.0 => {
                    buf[(x, bottom)].set_symbol("_").set_fg(SPARK_CPU);
                }
                Some(v) => {
                    let bars =
                        ((v / max).clamp(0.0, 1.0) * f64::from(inner.height) * 8.0).ceil() as u16;
                    for j in 0..inner.height {
                        let level = bars.saturating_sub(j * 8).min(8);
                        if level == 0 {
                            break;
                        }
                        let symbol = if self.ascii {
                            "#"
                        } else {
                            [" ", "▁", "▂", "▃", "▄", "▅", "▆", "▇", "█"][level as usize]
                        };
                        buf[(x, bottom - j)].set_symbol(symbol).set_fg(SPARK_CPU);
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn gaps_and_measured_zero_are_visibly_distinct() {
        let area = Rect::new(0, 0, 5, 1);
        let mut b = Buffer::empty(area);
        TimePlot {
            values: &[None, Some(0.0), Some(1.0)],
            max: Some(1.0),
            block: Block::default(),
            ascii: true,
        }
        .render(area, &mut b);
        assert_eq!(b[(2, 0)].symbol(), ".");
        assert_eq!(b[(3, 0)].symbol(), "_");
        assert_eq!(b[(4, 0)].symbol(), "#");
    }
}
