pub mod cpu;
pub mod disk;
pub mod drivers;
pub mod gpu;
pub mod memory;
pub mod network;
pub mod overview;
pub mod processes;
pub mod thermals;

use ratatui::layout::Rect;
use ratatui::Frame;

use crate::app::App;
use crate::types::{DiagnosticMode, Section};

/// Dispatch rendering to the active section
pub fn render(frame: &mut Frame, app: &App, area: Rect) {
    let mode = app.mode.unwrap_or(DiagnosticMode::User);

    match app.current_section {
        Section::Overview => overview::render(frame, app, area, mode),
        Section::Cpu => cpu::render(frame, app, area, mode),
        Section::Memory => memory::render(frame, app, area, mode),
        Section::Disk => disk::render(frame, app, area, mode),
        Section::Gpu => gpu::render(frame, app, area, mode),
        Section::Network => network::render(frame, app, area, mode),
        Section::Processes => processes::render(frame, app, area, mode),
        Section::Thermals => thermals::render(frame, app, area, mode),
        Section::Drivers => drivers::render(frame, app, area, mode),
    }
}
