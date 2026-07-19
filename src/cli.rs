use clap::{Args, Parser, Subcommand};

#[derive(Subcommand, Debug, Clone, PartialEq, Eq)]
pub enum Command {
    /// Check for updates and install the latest release.
    Update(ActionArgs),
    /// Install the latest release through the preferred managed CLI channel.
    Install(ActionArgs),
    /// Remove SD-300 through its proven installation owner.
    Uninstall(ActionArgs),
    /// Collect one redacted noninteractive diagnostic snapshot.
    Snapshot(ReportArgs),
    /// Show which diagnostic capabilities are available on this machine.
    Capabilities(ReportArgs),
    /// Installer-only cleanup used to make a fresh native install authoritative.
    #[command(hide = true)]
    MigrateCleanup(MigrateArgs),
    /// Delete a renamed Windows live-image backup after a verified update.
    #[command(hide = true)]
    UpdateCleanup(UpdateCleanupArgs),
    /// Perform an elevated, pinned, same-channel Global Windows update.
    #[command(hide = true)]
    UpdateWorker(UpdateWorkerArgs),
    /// Perform an elevated, pinned Global-to-managed Windows takeover.
    #[command(hide = true)]
    InstallWorker(InstallWorkerArgs),
    /// Perform an elevated, proven-owner Global Windows uninstall.
    #[command(hide = true)]
    UninstallWorker(UninstallWorkerArgs),
}

#[derive(Args, Debug, Clone, Default, PartialEq, Eq)]
pub struct ActionArgs {
    /// Emit exactly one machine-readable JSON result object.
    #[arg(long)]
    pub json: bool,
}

#[derive(Args, Debug, Clone, PartialEq, Eq)]
pub struct ReportArgs {
    /// Emit machine-readable JSON instead of a compact text summary.
    #[arg(long)]
    pub json: bool,

    /// Include host, network-address, MAC-address, and drive-serial values.
    #[arg(long, requires = "json")]
    pub include_sensitive: bool,
}

#[derive(Args, Debug, Clone, Default, PartialEq, Eq)]
pub struct MigrateArgs {
    /// Remove an allowlisted SD-300 copy from the invoking user's Cargo home.
    #[arg(long = "cargo-copy", hide = true)]
    pub cargo_copy: bool,

    /// Remove an unregistered executable left in the other Windows edition path.
    #[arg(long = "other-edition", hide = true)]
    pub other_edition: bool,

    /// Require every requested cleanup target to converge.
    #[arg(long, hide = true)]
    pub strict: bool,

    /// Suppress human-readable output.
    #[arg(long, hide = true)]
    pub quiet: bool,

    /// Resolve targets without deleting them.
    #[arg(long = "dry-run", hide = true)]
    pub dry_run: bool,

    /// Cargo home belonging to the user who launched the installer.
    #[arg(long = "cargo-home", value_name = "PATH", hide = true)]
    pub cargo_home: Option<std::path::PathBuf>,

    /// Profile belonging to the user who launched the installer.
    #[arg(long = "user-profile", value_name = "PATH", hide = true)]
    pub user_profile: Option<std::path::PathBuf>,
}

#[derive(Args, Debug, Clone, PartialEq, Eq)]
pub struct UpdateCleanupArgs {
    /// Exact private sibling created by the Windows live-image handoff.
    #[arg(long = "update-backup", value_name = "PATH", hide = true)]
    pub update_backup: std::path::PathBuf,
}

#[derive(Args, Debug, Clone, PartialEq, Eq)]
pub struct UpdateWorkerArgs {
    /// Exact Global Windows channel selected by the unelevated parent.
    #[arg(long = "update-channel", value_name = "CHANNEL", hide = true)]
    pub update_channel: String,

    /// Exact immutable release version selected by the unelevated parent.
    #[arg(long = "update-version", value_name = "VERSION", hide = true)]
    pub update_version: String,

    /// Exact private sibling reserved by the unelevated parent.
    #[arg(long = "update-backup", value_name = "PATH", hide = true)]
    pub update_backup: std::path::PathBuf,
}

#[derive(Args, Debug, Clone, PartialEq, Eq)]
pub struct InstallWorkerArgs {
    /// Exact Global Windows channel selected by the unelevated parent.
    #[arg(long = "install-channel", value_name = "CHANNEL", hide = true)]
    pub install_channel: String,

    /// Exact immutable release version selected by the unelevated parent.
    #[arg(long = "install-version", value_name = "VERSION", hide = true)]
    pub install_version: String,

    /// Exact private sibling reserved by the unelevated parent.
    #[arg(long = "install-backup", value_name = "PATH", hide = true)]
    pub install_backup: std::path::PathBuf,
}

#[derive(Args, Debug, Clone, PartialEq, Eq)]
pub struct UninstallWorkerArgs {
    /// Exact Global Windows channel selected by the unelevated parent.
    #[arg(long = "uninstall-channel", value_name = "CHANNEL", hide = true)]
    pub uninstall_channel: String,

    /// Exact private sibling reserved for the running installed image.
    #[arg(long = "uninstall-backup", value_name = "PATH", hide = true)]
    pub uninstall_backup: std::path::PathBuf,
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
    sd300 update to check for and install the latest release. Use
    sd300 snapshot --json for a redacted, noninteractive diagnostic report."
)]
#[command(after_long_help = "\
EXAMPLES:
  sd300          Choose a diagnostic mode interactively
  sd300 --user   Launch directly into User Mode
  sd300 --tech   Launch directly into Technician Mode
  sd300 update   Check for updates and install
  sd300 install  Install latest through the managed CLI channel
  sd300 uninstall Remove the proven installed channel
  sd300 snapshot --json       Redacted diagnostic snapshot
  sd300 capabilities --json   Capability and availability states
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
        assert_eq!(cli.command, Some(Command::Update(ActionArgs::default())));
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
        assert!(help.contains("sd300 snapshot --json"));
    }

    #[test]
    fn parses_redacted_snapshot_and_capabilities_actions() {
        let snapshot = Cli::try_parse_from(["sd300", "snapshot", "--json"])
            .expect("snapshot action should parse");
        assert_eq!(
            snapshot.command,
            Some(Command::Snapshot(ReportArgs {
                json: true,
                include_sensitive: false,
            }))
        );

        let capabilities = Cli::try_parse_from(["sd300", "capabilities", "--json"])
            .expect("capabilities action should parse");
        assert!(matches!(
            capabilities.command,
            Some(Command::Capabilities(ReportArgs { json: true, .. }))
        ));
    }

    #[test]
    fn parses_lifecycle_json_actions() {
        let update = Cli::try_parse_from(["sd300", "update", "--json"])
            .expect("update JSON action should parse");
        assert_eq!(
            update.command,
            Some(Command::Update(ActionArgs { json: true }))
        );

        let install =
            Cli::try_parse_from(["sd300", "install"]).expect("install action should parse");
        assert_eq!(
            install.command,
            Some(Command::Install(ActionArgs::default()))
        );

        let uninstall =
            Cli::try_parse_from(["sd300", "uninstall"]).expect("uninstall action should parse");
        assert_eq!(
            uninstall.command,
            Some(Command::Uninstall(ActionArgs::default()))
        );
    }

    #[test]
    fn parses_hidden_installer_cleanup() {
        let cleanup = Cli::try_parse_from([
            "sd300",
            "migrate-cleanup",
            "--cargo-copy",
            "--strict",
            "--quiet",
        ])
        .expect("installer cleanup should parse");
        assert!(matches!(
            cleanup.command,
            Some(Command::MigrateCleanup(MigrateArgs {
                cargo_copy: true,
                strict: true,
                quiet: true,
                ..
            }))
        ));
    }

    #[test]
    fn parses_hidden_windows_update_actions() {
        let worker = Cli::try_parse_from([
            "sd300",
            "update-worker",
            "--update-channel",
            "msi-global",
            "--update-version",
            "2.0.0",
            "--update-backup",
            r"C:\Program Files\sd300\bin\.sd300-update-backup-12-34.exe",
        ])
        .expect("hidden update worker should parse");
        assert!(matches!(worker.command, Some(Command::UpdateWorker(_))));

        let install_worker = Cli::try_parse_from([
            "sd300",
            "install-worker",
            "--install-channel",
            "exe-global",
            "--install-version",
            "2.0.0",
            "--install-backup",
            r"C:\Program Files\sd300\bin\.sd300-update-backup-12-34.exe",
        ])
        .expect("hidden install worker should parse");
        assert!(matches!(
            install_worker.command,
            Some(Command::InstallWorker(_))
        ));

        let cleanup = Cli::try_parse_from([
            "sd300",
            "update-cleanup",
            "--update-backup",
            r"C:\Users\example\.cargo\bin\.sd300-update-backup-12-34.exe",
        ])
        .expect("hidden update cleanup should parse");
        assert!(matches!(cleanup.command, Some(Command::UpdateCleanup(_))));

        let uninstall_worker = Cli::try_parse_from([
            "sd300",
            "uninstall-worker",
            "--uninstall-channel",
            "msi-global",
            "--uninstall-backup",
            r"C:\Program Files\sd300\bin\.sd300-uninstall-backup-12-34.exe",
        ])
        .expect("hidden uninstall worker should parse");
        assert!(matches!(
            uninstall_worker.command,
            Some(Command::UninstallWorker(_))
        ));
    }

    #[test]
    fn sensitive_snapshot_requires_json() {
        let error = Cli::try_parse_from(["sd300", "snapshot", "--include-sensitive"])
            .expect_err("sensitive values should require explicit JSON output");
        assert_eq!(
            error.kind(),
            clap::error::ErrorKind::MissingRequiredArgument
        );
    }
}
