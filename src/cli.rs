use clap::{Parser, Subcommand};

#[derive(Subcommand, Debug, Clone, Copy, PartialEq, Eq)]
pub enum Command {
    /// Check for updates and install the latest release.
    Update,
}

/// SD-300 System Diagnostic — Real-time interactive system diagnostics TUI
#[derive(Parser, Debug)]
#[command(name = "sd300")]
#[command(
    author,
    version,
    about = "SD-300 System Diagnostic — QubeTX Developer Tools"
)]
#[command(args_conflicts_with_subcommands = true)]
#[command(
    long_about = "SD-300 is a live, interactive terminal user interface for real-time \n\
    system diagnostics and monitoring. Part of the QubeTX 300 Series \n\
    alongside TR-300 (Machine Report) and ND-300 (Network Diagnostic).\n\n\
    Run without flags to choose a diagnostic mode interactively, \n\
    use --user / --tech to launch directly into a mode, or run \n\
    sd300 update to check for and install the latest release."
)]
#[command(after_long_help = "\
EXAMPLES:
  sd300          Choose a diagnostic mode interactively
  sd300 --user   Launch directly into User Mode
  sd300 --tech   Launch directly into Technician Mode
  sd300 update   Check for updates and install
  sd300 --update Same as 'sd300 update' (legacy flag form)

KEYBINDINGS:
  1-9          Switch to section
  q / Esc      Quit
  Ctrl+C       Quit to shell
  m            Return to mode selection
  ?            Help overlay
  f            Toggle temperature unit (C/F)
  j / k        Scroll (Processes, Connections, Drivers, Disk)
  c / M / p / n  Sort processes by CPU / Memory / PID / Name
  r            Refresh drivers (Drivers section)

SECTIONS:
  1 Overview    System health dashboard / identity and gauges
  2 CPU         Load, sparkline, per-core utilization
  3 Memory      Usage, swap, top consumers
  4 Disk        Drive health, partitions, SMART data
  5 GPU         Utilization, VRAM, temperature
  6 Network     Connectivity, interfaces, active connections
  7 Processes   Running apps / sortable process table
  8 Thermals    Temperature, fans, battery, power
  9 Drivers     Device health, driver versions, services")]
pub struct Cli {
    /// Launch directly into User Mode (plain language diagnostics)
    #[arg(long, conflicts_with_all = ["tech", "update"])]
    pub user: bool,

    /// Launch directly into Technician Mode (raw data and advanced metrics)
    #[arg(long, conflicts_with_all = ["user", "update"])]
    pub tech: bool,

    /// Check for updates and install the latest version (legacy flag form of `sd300 update`)
    #[arg(long, conflicts_with_all = ["user", "tech"])]
    pub update: bool,

    /// Action subcommand. If present, takes precedence over legacy action flags.
    #[command(subcommand)]
    pub command: Option<Command>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::{CommandFactory, Parser};

    #[test]
    fn parses_positional_update_action() {
        let cli = Cli::try_parse_from(["sd300", "update"]).expect("update action should parse");
        assert_eq!(cli.command, Some(Command::Update));
        assert!(!cli.update);
    }

    #[test]
    fn parses_legacy_update_flag() {
        let cli = Cli::try_parse_from(["sd300", "--update"]).expect("--update should parse");
        assert!(cli.update);
        assert_eq!(cli.command, None);
    }

    #[test]
    fn rejects_update_with_mode_flags() {
        let err = Cli::try_parse_from(["sd300", "update", "--tech"]).unwrap_err();
        assert!(matches!(
            err.kind(),
            clap::error::ErrorKind::ArgumentConflict | clap::error::ErrorKind::UnknownArgument
        ));
    }

    #[test]
    fn help_includes_update_forms() {
        let help = Cli::command().render_long_help().to_string();
        assert!(help.contains("sd300 update"));
        assert!(help.contains("--update"));
    }
}
