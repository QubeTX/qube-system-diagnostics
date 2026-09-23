use clap::Parser;
use sd_300::{
    app::App,
    cli::{Cli, Command},
    error::Result,
    types::DiagnosticMode,
};

fn main() -> Result<()> {
    let cli = Cli::parse();
    if let Some(Command::StorageProbePrepare { device }) = &cli.command {
        let result = sd_300::storage_probe::prepare_worker(device);
        println!("{}", serde_json::to_string(&result).unwrap());
        return Ok(());
    }
    if let Some(Command::StorageProbeRead { port, nonce }) = &cli.command {
        return sd_300::storage_probe::read_worker(*port, nonce)
            .map_err(sd_300::error::AppError::platform);
    }
    if let Some(Command::StorageProbeElevate { port, nonce }) = &cli.command {
        let result = sd_300::storage_probe::elevate_worker(*port, nonce);
        println!("{}", serde_json::to_string(&result).unwrap());
        return Ok(());
    }
    if let Some(Command::CollectWorker { topic }) = &cli.command {
        return sd_300::collectors::probe::print_worker(*topic);
    }
    if let Some(Command::CollectServer { topic, response }) = &cli.command {
        return sd_300::collectors::probe::serve(*topic, response);
    }
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(run(cli))
}

async fn run(cli: Cli) -> Result<()> {
    // Enable UTF-8 output on Windows
    #[cfg(windows)]
    {
        enable_utf8_console();
    }

    if cli.update {
        let exit_code = sd_300::update::run(false)?;
        std::process::exit(exit_code);
    }

    if let Some(command) = cli.command {
        match command {
            Command::StorageProbePrepare { .. }
            | Command::StorageProbeRead { .. }
            | Command::StorageProbeElevate { .. } => {
                unreachable!("private probe handled before runtime startup")
            }
            Command::CollectServer { topic, response } => {
                sd_300::collectors::probe::serve(topic, &response)?;
                return Ok(());
            }
            Command::CollectWorker { topic } => {
                sd_300::collectors::probe::print_worker(topic)?;
                return Ok(());
            }
            Command::Update(args) => {
                let exit_code = sd_300::update::run_with_relaunch(args.json, args.relaunch_gui)?;
                std::process::exit(exit_code);
            }
            Command::Install(args) => {
                let exit_code = sd_300::update::install(args.json)?;
                std::process::exit(exit_code);
            }
            Command::Uninstall(args) => {
                let exit_code = sd_300::update::uninstall(args.json)?;
                std::process::exit(exit_code);
            }
            Command::Snapshot(args) => {
                let report =
                    sd_300::report::DiagnosticReport::collect(args.include_sensitive).await;
                sd_300::report::print_snapshot(&report, args.json, args.schema_version)?;
                return Ok(());
            }
            Command::Capabilities(args) => {
                let report =
                    sd_300::report::DiagnosticReport::collect(args.include_sensitive).await;
                sd_300::report::print_capabilities(&report, args.json, args.schema_version)?;
                return Ok(());
            }
            Command::Gui => {
                std::process::exit(sd_300::gui::launch());
            }
            Command::Tools(args) => {
                std::process::exit(sd_300::optional_tools::run_cli(&args));
            }
            Command::MigrateCleanup(args) => {
                let exit_code = sd_300::migrate::run(&args);
                std::process::exit(exit_code);
            }
            Command::CleanupGuiState(args) => {
                let exit_code = sd_300::update::cleanup_owned_gui_state(args.quiet);
                std::process::exit(exit_code);
            }
            Command::StopGui(args) => {
                let exit_code = sd_300::update::stop_gui(args.quiet);
                std::process::exit(exit_code);
            }
            Command::UpdateCleanup(args) => {
                let exit_code = sd_300::update::cleanup_windows_update_backup(&args.update_backup);
                std::process::exit(exit_code);
            }
            Command::UpdateWorker(args) => {
                let exit_code = sd_300::update::run_windows_update_worker(
                    &args.update_channel,
                    &args.update_version,
                    &args.update_backup,
                );
                std::process::exit(exit_code);
            }
            Command::InstallWorker(args) => {
                let exit_code = sd_300::update::run_windows_install_worker(
                    &args.install_channel,
                    &args.install_version,
                    &args.install_backup,
                );
                std::process::exit(exit_code);
            }
            Command::UninstallWorker(args) => {
                let exit_code = sd_300::update::run_windows_uninstall_worker(
                    &args.uninstall_channel,
                    &args.uninstall_backup,
                );
                std::process::exit(exit_code);
            }
        }
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
    if initial_mode.is_none() {
        app.cargo_gui_completion_notice = sd_300::update::cargo_gui_completion_needed();
    }
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
