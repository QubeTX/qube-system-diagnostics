use clap::Parser;
use sd_300::{app::App, cli::Cli, error::Result, types::DiagnosticMode};

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    // Enable UTF-8 output on Windows
    #[cfg(windows)]
    {
        enable_utf8_console();
    }

    // Determine initial mode from CLI flags
    let initial_mode = if cli.user {
        Some(DiagnosticMode::User)
    } else if cli.tech {
        Some(DiagnosticMode::Technician)
    } else {
        None
    };

    // Install panic hook to restore terminal
    let original_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |panic_info| {
        ratatui::restore();
        original_hook(panic_info);
    }));

    // Run the app
    let mut terminal = ratatui::init();
    let mut app = App::new(initial_mode);
    let result = app.run(&mut terminal).await;

    // Restore terminal
    ratatui::restore();

    result
}

/// Enable UTF-8 console output on Windows
#[cfg(windows)]
fn enable_utf8_console() {
    use std::io::IsTerminal;
    if std::io::stdout().is_terminal() {
        unsafe {
            winapi::um::wincon::SetConsoleOutputCP(65001);
        }
    }
}
