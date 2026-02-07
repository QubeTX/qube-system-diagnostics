use clap::Parser;

/// SD-300 System Diagnostic — Real-time interactive system diagnostics TUI
#[derive(Parser, Debug)]
#[command(name = "sd300")]
#[command(author, version, about = "SD-300 System Diagnostic — QubeTX Developer Tools")]
#[command(long_about = "SD-300 is a live, interactive terminal user interface for real-time \n\
    system diagnostics and monitoring. Part of the QubeTX 300 Series \n\
    alongside TR-300 (Machine Report) and ND-300 (Network Diagnostic).\n\n\
    Run without flags to choose a diagnostic mode interactively, \n\
    or use --user / --tech to launch directly into a mode.")]
pub struct Cli {
    /// Launch directly into User Mode (plain language diagnostics)
    #[arg(long)]
    pub user: bool,

    /// Launch directly into Technician Mode (raw data and advanced metrics)
    #[arg(long)]
    pub tech: bool,
}
