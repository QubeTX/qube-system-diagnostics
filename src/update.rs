use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use serde::Serialize;
use sha2::{Digest, Sha256};

use crate::collectors::command::{run_output, CommandTimeout};
use crate::error::Result;

pub const RELEASES_URL: &str =
    "https://api.github.com/repos/QubeTX/qube-system-diagnostics/releases/latest";
const RELEASES_PAGE: &str = "https://github.com/QubeTX/qube-system-diagnostics/releases";
const RELEASE_BASE: &str = "https://github.com/QubeTX/qube-system-diagnostics/releases/download";
const CRATE_NAME: &str = "tr300-tui";
const APP_NAME: &str = "sd300";
const VERSION: &str = env!("CARGO_PKG_VERSION");

const POWERSHELL_WRAPPER: &str = "sd300-cli-installer.ps1";
const SHELL_WRAPPER: &str = "sd300-cli-installer.sh";
const MSI_GLOBAL_ASSET: &str = "sd300-windows-x64-global.msi";
const MSI_CORPORATE_ASSET: &str = "sd300-windows-x64-corporate.msi";
const EXE_GLOBAL_ASSET: &str = "sd300-windows-x64-global.exe";
const EXE_CORPORATE_ASSET: &str = "sd300-windows-x64-corporate.exe";
const MAC_PKG_ASSET: &str = "sd300-macos-universal.pkg";
#[cfg_attr(not(target_os = "macos"), allow(dead_code))]
const MAC_PKG_ID: &str = "com.qubetx.sd300.pkg";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
#[cfg_attr(not(windows), allow(dead_code))]
enum InstallChannel {
    #[serde(rename = "powershell-installer")]
    PowerShellInstaller,
    ShellInstaller,
    Cargo,
    MsiGlobal,
    MsiCorporate,
    ExeGlobal,
    ExeCorporate,
    #[cfg_attr(not(target_os = "macos"), allow(dead_code))]
    MacPkg,
}

impl InstallChannel {
    fn label(self) -> &'static str {
        match self {
            Self::PowerShellInstaller => "managed PowerShell",
            Self::ShellInstaller => "managed shell",
            Self::Cargo => "Cargo",
            Self::MsiGlobal => "Global MSI",
            Self::MsiCorporate => "Corporate MSI",
            Self::ExeGlobal => "Global EXE",
            Self::ExeCorporate => "Corporate EXE",
            Self::MacPkg => "macOS PKG",
        }
    }

    fn update_asset(self) -> Option<&'static str> {
        match self {
            Self::PowerShellInstaller => Some(POWERSHELL_WRAPPER),
            Self::ShellInstaller => Some(SHELL_WRAPPER),
            Self::MsiGlobal => Some(MSI_GLOBAL_ASSET),
            Self::MsiCorporate => Some(MSI_CORPORATE_ASSET),
            Self::ExeGlobal => Some(EXE_GLOBAL_ASSET),
            Self::ExeCorporate => Some(EXE_CORPORATE_ASSET),
            Self::MacPkg => Some(MAC_PKG_ASSET),
            Self::Cargo => None,
        }
    }

    #[cfg(windows)]
    fn global_worker_id(self) -> Option<&'static str> {
        match self {
            Self::MsiGlobal => Some("msi-global"),
            Self::ExeGlobal => Some("exe-global"),
            _ => None,
        }
    }

    #[cfg(windows)]
    fn from_global_worker_id(value: &str) -> Option<Self> {
        match value {
            "msi-global" => Some(Self::MsiGlobal),
            "exe-global" => Some(Self::ExeGlobal),
            _ => None,
        }
    }
}

#[derive(Debug, Clone)]
struct Installation {
    channel: InstallChannel,
    binary_path: PathBuf,
}

#[derive(Debug, Clone)]
struct Release {
    tag: String,
    version: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(not(any(windows, test)), allow(dead_code))]
enum WindowsInstallerCompletion {
    Complete,
    RebootRequired(i32),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum UpdateExecution {
    Standard,
    WindowsMsiCommitted(WindowsInstallerCompletion),
}

impl UpdateExecution {
    #[cfg(any(windows, test))]
    fn windows_msi_committed(self) -> bool {
        matches!(self, Self::WindowsMsiCommitted(_))
    }

    fn reboot_required(self) -> bool {
        matches!(
            self,
            Self::WindowsMsiCommitted(WindowsInstallerCompletion::RebootRequired(_))
        )
    }

    fn success_suffix(self) -> &'static str {
        if self.reboot_required() {
            "; Windows Installer committed the update and requires a reboot"
        } else {
            ""
        }
    }

    #[cfg(any(windows, test))]
    fn post_commit_failure(self, message: String) -> String {
        match self {
            Self::WindowsMsiCommitted(completion) => format!(
                "{message}; Windows Installer already committed the product transaction, so the prior executable was not restored.{}",
                match completion {
                    WindowsInstallerCompletion::Complete => {
                        " Run the same update again to repair the composite installation."
                    }
                    _ => windows_pending_verification_reboot_message(completion),
                }
            ),
            Self::Standard => message,
        }
    }

    #[cfg(any(windows, test))]
    fn worker_exit_code(self) -> i32 {
        match self {
            Self::WindowsMsiCommitted(WindowsInstallerCompletion::RebootRequired(code)) => code,
            _ => 0,
        }
    }

    #[cfg(any(windows, test))]
    fn committed_worker_failure_exit_code(self) -> i32 {
        match self {
            Self::WindowsMsiCommitted(completion) => {
                windows_committed_worker_failure_exit_code(completion)
            }
            Self::Standard => 2,
        }
    }
}

#[cfg(windows)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum WindowsNativeUninstallExecution {
    Exe,
    MsiCommitted(WindowsInstallerCompletion),
}

#[cfg(windows)]
impl WindowsNativeUninstallExecution {
    fn windows_msi_committed(self) -> bool {
        matches!(self, Self::MsiCommitted(_))
    }

    fn reboot_required(self) -> bool {
        matches!(
            self,
            Self::MsiCommitted(WindowsInstallerCompletion::RebootRequired(_))
        )
    }

    fn worker_exit_code(self) -> i32 {
        match self {
            Self::MsiCommitted(WindowsInstallerCompletion::RebootRequired(code)) => code,
            _ => 0,
        }
    }

    fn committed_worker_failure_exit_code(self) -> i32 {
        match self {
            Self::MsiCommitted(completion) => {
                windows_committed_worker_failure_exit_code(completion)
            }
            Self::Exe => 2,
        }
    }
}

#[cfg(any(windows, test))]
const WINDOWS_WORKER_MSI_COMMITTED_NEEDS_REPAIR: i32 = 200;
#[cfg(any(windows, test))]
const WINDOWS_WORKER_MSI_1641_COMMITTED_NEEDS_REPAIR: i32 = 201;
#[cfg(any(windows, test))]
const WINDOWS_WORKER_MSI_3010_COMMITTED_NEEDS_REPAIR: i32 = 202;

#[cfg(any(windows, test))]
fn windows_committed_worker_failure_exit_code(completion: WindowsInstallerCompletion) -> i32 {
    match completion {
        WindowsInstallerCompletion::Complete => WINDOWS_WORKER_MSI_COMMITTED_NEEDS_REPAIR,
        WindowsInstallerCompletion::RebootRequired(1641) => {
            WINDOWS_WORKER_MSI_1641_COMMITTED_NEEDS_REPAIR
        }
        WindowsInstallerCompletion::RebootRequired(_) => {
            WINDOWS_WORKER_MSI_3010_COMMITTED_NEEDS_REPAIR
        }
    }
}

#[cfg(any(windows, test))]
fn windows_committed_worker_failure_completion(
    exit_code: u32,
) -> Option<WindowsInstallerCompletion> {
    match i32::try_from(exit_code).ok()? {
        WINDOWS_WORKER_MSI_COMMITTED_NEEDS_REPAIR => Some(WindowsInstallerCompletion::Complete),
        WINDOWS_WORKER_MSI_1641_COMMITTED_NEEDS_REPAIR => {
            Some(WindowsInstallerCompletion::RebootRequired(1641))
        }
        WINDOWS_WORKER_MSI_3010_COMMITTED_NEEDS_REPAIR => {
            Some(WindowsInstallerCompletion::RebootRequired(3010))
        }
        _ => None,
    }
}

#[cfg(any(windows, test))]
fn windows_pending_verification_reboot_message(
    completion: WindowsInstallerCompletion,
) -> &'static str {
    match completion {
        WindowsInstallerCompletion::Complete => "",
        WindowsInstallerCompletion::RebootRequired(1641) => {
            " Windows Installer reported that a reboot was initiated; verify the composite installation after Windows restarts."
        }
        WindowsInstallerCompletion::RebootRequired(_) => {
            " Windows Installer requires a reboot; restart Windows before verifying or repairing the composite installation."
        }
    }
}

#[derive(Debug, Serialize)]
struct LifecycleResult<'a> {
    action: &'a str,
    success: bool,
    current_version: &'a str,
    target_version: Option<&'a str>,
    install_channel: Option<InstallChannel>,
    strategy: Option<&'a str>,
    message: String,
}

/// The desktop app's in-app update path: the identical proven-owner update
/// transaction, followed by a GUI relaunch that the singleton Open route makes
/// idempotent. Relaunch requires both the explicit hidden coordinator flag and
/// a successful outcome, so installers and ordinary terminal updates never
/// launch the app.
pub fn run_with_relaunch(json: bool, relaunch_gui: bool) -> Result<i32> {
    let exit_code = run(json)?;
    if should_relaunch_gui(relaunch_gui, exit_code) {
        // Relaunch problems report on stderr only; the update's stdout
        // contract and exit code stay exactly what the transaction produced.
        let _ = crate::gui::launch();
    }
    Ok(exit_code)
}

fn should_relaunch_gui(requested: bool, exit_code: i32) -> bool {
    requested && exit_code == 0
}

pub fn run(json: bool) -> Result<i32> {
    let installation = match detect_installation() {
        Ok(installation) => installation,
        Err(message) => {
            return emit(
                json,
                LifecycleResult {
                    action: "update",
                    success: false,
                    current_version: VERSION,
                    target_version: None,
                    install_channel: None,
                    strategy: None,
                    message,
                },
                2,
            )
        }
    };
    let release = match fetch_latest_release() {
        Ok(release) => release,
        Err(message) => {
            return emit(
                json,
                LifecycleResult {
                    action: "update",
                    success: false,
                    current_version: VERSION,
                    target_version: None,
                    install_channel: Some(installation.channel),
                    strategy: None,
                    message,
                },
                2,
            )
        }
    };

    if !is_newer(VERSION, &release.version) {
        if release.version == VERSION {
            if let Err(reason) = crate::gui::verify_installed(VERSION) {
                return repair_current_gui(json, &installation, &release, reason);
            }
        }
        return emit(
            json,
            LifecycleResult {
                action: "update",
                success: true,
                current_version: VERSION,
                target_version: Some(&release.version),
                install_channel: Some(installation.channel),
                strategy: Some("current"),
                message: format!("SD-300 {VERSION} is already current"),
            },
            0,
        );
    }

    if !json {
        println!(
            "Updating SD-300 {} -> {} through the proven {} channel...",
            VERSION,
            release.version,
            installation.channel.label()
        );
    }
    let strategy = installation.channel.label();
    if let Err(message) = crate::gui::request_exit() {
        return emit(
            json,
            LifecycleResult {
                action: "update",
                success: false,
                current_version: VERSION,
                target_version: Some(&release.version),
                install_channel: Some(installation.channel),
                strategy: Some(strategy),
                message: format!(
                    "The installed GUI could not be stopped safely before update: {message}"
                ),
            },
            2,
        );
    }
    let outcome = perform_update(&installation, &release, json);
    match outcome {
        Ok(execution) => emit(
            json,
            LifecycleResult {
                action: "update",
                success: true,
                current_version: VERSION,
                target_version: Some(&release.version),
                install_channel: Some(installation.channel),
                strategy: Some(strategy),
                message: format!(
                    "Updated to {} without changing the {} channel{}",
                    release.version,
                    installation.channel.label(),
                    execution.success_suffix()
                ),
            },
            0,
        ),
        Err(message) => emit(
            json,
            LifecycleResult {
                action: "update",
                success: false,
                current_version: VERSION,
                target_version: Some(&release.version),
                install_channel: Some(installation.channel),
                strategy: Some(strategy),
                message,
            },
            2,
        ),
    }
}

fn repair_current_gui(
    json: bool,
    installation: &Installation,
    release: &Release,
    reason: String,
) -> Result<i32> {
    let (strategy, target_channel) = if installation.channel == InstallChannel::Cargo {
        (
            "cargo-managed-completion",
            if cfg!(windows) {
                InstallChannel::PowerShellInstaller
            } else {
                InstallChannel::ShellInstaller
            },
        )
    } else {
        ("same-version-repair", installation.channel)
    };
    if !json {
        println!(
            "Repairing the SD-300 {VERSION} GUI companion through the {} path ({reason})...",
            target_channel.label()
        );
    }
    if let Err(message) = crate::gui::request_exit() {
        return emit(
            json,
            LifecycleResult {
                action: "update",
                success: false,
                current_version: VERSION,
                target_version: Some(&release.version),
                install_channel: Some(installation.channel),
                strategy: Some(strategy),
                message: format!(
                    "The installed GUI could not be stopped safely before repair: {message}"
                ),
            },
            2,
        );
    }

    let repaired = if installation.channel == InstallChannel::Cargo {
        perform_managed_install(target_channel, release, json).map(|()| UpdateExecution::Standard)
    } else {
        perform_update(installation, release, json)
    };

    match repaired {
        Ok(execution) => emit(
            json,
            LifecycleResult {
                action: "update",
                success: true,
                current_version: VERSION,
                target_version: Some(&release.version),
                install_channel: Some(installation.channel),
                strategy: Some(strategy),
                message: if installation.channel == InstallChannel::Cargo {
                    format!(
                        "Completed the same-version managed CLI+GUI takeover from Cargo through the {} channel",
                        target_channel.label()
                    )
                } else {
                    format!(
                        "Repaired the missing or incompatible GUI companion without changing the {} channel{}",
                        installation.channel.label(),
                        execution.success_suffix()
                    )
                },
            },
            0,
        ),
        Err(message) => emit(
            json,
            LifecycleResult {
                action: "update",
                success: false,
                current_version: VERSION,
                target_version: Some(&release.version),
                install_channel: Some(installation.channel),
                strategy: Some(strategy),
                message,
            },
            2,
        ),
    }
}

pub fn install(json: bool) -> Result<i32> {
    let release = match fetch_latest_release() {
        Ok(release) => release,
        Err(message) => {
            return emit(
                json,
                LifecycleResult {
                    action: "install",
                    success: false,
                    current_version: VERSION,
                    target_version: None,
                    install_channel: None,
                    strategy: None,
                    message,
                },
                2,
            )
        }
    };
    let channel = if cfg!(windows) {
        InstallChannel::PowerShellInstaller
    } else {
        InstallChannel::ShellInstaller
    };
    if let Err(message) = crate::gui::request_exit() {
        return emit(
            json,
            LifecycleResult {
                action: "install",
                success: false,
                current_version: VERSION,
                target_version: Some(&release.version),
                install_channel: Some(channel),
                strategy: Some(channel.label()),
                message: format!(
                    "The installed GUI could not be stopped safely before installation: {message}"
                ),
            },
            2,
        );
    }
    if !json {
        println!(
            "Installing SD-300 {} through the preferred {} channel...",
            release.version,
            channel.label()
        );
    }
    let outcome = perform_managed_install(channel, &release, json);
    match outcome {
        Ok(()) => emit(
            json,
            LifecycleResult {
                action: "install",
                success: true,
                current_version: VERSION,
                target_version: Some(&release.version),
                install_channel: Some(channel),
                strategy: Some(channel.label()),
                message: "The latest deliberate managed install is now authoritative".into(),
            },
            0,
        ),
        Err(message) => emit(
            json,
            LifecycleResult {
                action: "install",
                success: false,
                current_version: VERSION,
                target_version: Some(&release.version),
                install_channel: Some(channel),
                strategy: Some(channel.label()),
                message,
            },
            2,
        ),
    }
}

pub fn uninstall(json: bool) -> Result<i32> {
    let installation = match detect_installation() {
        Ok(installation) => installation,
        Err(message) => {
            return emit(
                json,
                LifecycleResult {
                    action: "uninstall",
                    success: false,
                    current_version: VERSION,
                    target_version: None,
                    install_channel: None,
                    strategy: None,
                    message,
                },
                2,
            )
        }
    };
    let strategy = installation.channel.label();
    if let Err(message) = crate::gui::request_exit() {
        return emit(
            json,
            LifecycleResult {
                action: "uninstall",
                success: false,
                current_version: VERSION,
                target_version: None,
                install_channel: Some(installation.channel),
                strategy: Some(strategy),
                message: format!(
                    "The installed GUI could not be stopped safely before uninstall: {message}"
                ),
            },
            2,
        );
    }
    match execute_uninstall(&installation, json).and_then(|message| {
        let cleanup = remove_owned_windows_startup(&installation)
            .and_then(|()| crate::settings::remove_owned_gui_state())
            .and_then(|()| verify_windows_uninstall_completion(&installation));
        cleanup.map_err(|error| format!("{message}; post-uninstall cleanup failed: {error}"))?;
        Ok(format!(
            "{message}; owned GUI settings and launch-at-login integration were removed"
        ))
    }) {
        Ok(message) => emit(
            json,
            LifecycleResult {
                action: "uninstall",
                success: true,
                current_version: VERSION,
                target_version: None,
                install_channel: Some(installation.channel),
                strategy: Some(strategy),
                message,
            },
            0,
        ),
        Err(message) => emit(
            json,
            LifecycleResult {
                action: "uninstall",
                success: false,
                current_version: VERSION,
                target_version: None,
                install_channel: Some(installation.channel),
                strategy: Some(strategy),
                message,
            },
            2,
        ),
    }
}

#[cfg(not(windows))]
fn remove_owned_windows_startup(_installation: &Installation) -> std::result::Result<(), String> {
    Ok(())
}

#[cfg(not(windows))]
fn verify_windows_uninstall_completion(
    _installation: &Installation,
) -> std::result::Result<(), String> {
    Ok(())
}

pub fn cleanup_owned_gui_state(quiet: bool) -> i32 {
    let result =
        crate::gui::request_exit().and_then(|()| crate::settings::remove_owned_gui_state());
    match result {
        Ok(()) => {
            if !quiet {
                println!("Owned SD-300 GUI settings and startup integration were removed");
            }
            0
        }
        Err(message) => {
            eprintln!("SD-300 GUI state cleanup failed safely: {message}");
            2
        }
    }
}

pub fn stop_gui(quiet: bool) -> i32 {
    match crate::gui::request_exit() {
        Ok(()) => {
            if !quiet {
                println!("The SD-300 GUI is stopped");
            }
            0
        }
        Err(message) => {
            eprintln!("SD-300 GUI shutdown failed safely: {message}");
            2
        }
    }
}

fn emit(json: bool, result: LifecycleResult<'_>, exit_code: i32) -> Result<i32> {
    if json {
        println!("{}", serialize_lifecycle_result(&result, exit_code));
    } else if result.success {
        println!("{}", result.message);
    } else {
        eprintln!("SD-300 {} failed safely: {}", result.action, result.message);
    }
    Ok(exit_code)
}

fn serialize_lifecycle_result(result: &LifecycleResult<'_>, exit_code: i32) -> String {
    let mut payload = match serde_json::to_value(result) {
        Ok(payload) => payload,
        Err(error) => {
            return serde_json::json!({
                "action": result.action,
                "success": false,
                "message": format!("JSON serialization failed: {error}"),
                "recovery_url": RELEASES_PAGE,
                "requires_user_action": true,
            })
            .to_string();
        }
    };
    if let Some(object) = payload.as_object_mut() {
        object.insert(
            "recovery_url".to_string(),
            serde_json::Value::String(RELEASES_PAGE.to_string()),
        );
        object.insert(
            "requires_user_action".to_string(),
            serde_json::Value::Bool(exit_code != 0),
        );
    }
    payload.to_string()
}

fn execute_update(
    installation: &Installation,
    release: &Release,
    quiet_stdout: bool,
) -> std::result::Result<UpdateExecution, String> {
    match installation.channel {
        InstallChannel::Cargo => run_status(
            Command::new("cargo").args([
                "install",
                CRATE_NAME,
                "--version",
                &release.version,
                "--force",
                "--locked",
            ]),
            quiet_stdout,
        )
        .map(|()| UpdateExecution::Standard),
        InstallChannel::PowerShellInstaller | InstallChannel::ShellInstaller => {
            execute_managed_wrapper(installation.channel, release, quiet_stdout)
                .map(|()| UpdateExecution::Standard)
        }
        InstallChannel::MsiGlobal | InstallChannel::MsiCorporate => {
            execute_windows_msi(installation.channel, release, quiet_stdout)
                .map(UpdateExecution::WindowsMsiCommitted)
        }
        InstallChannel::ExeGlobal | InstallChannel::ExeCorporate => {
            execute_windows_exe(installation.channel, release, quiet_stdout)
                .map(|()| UpdateExecution::Standard)
        }
        InstallChannel::MacPkg => {
            execute_macos_pkg(release, quiet_stdout).map(|()| UpdateExecution::Standard)
        }
    }
}

fn perform_update(
    installation: &Installation,
    release: &Release,
    quiet_stdout: bool,
) -> std::result::Result<UpdateExecution, String> {
    #[cfg(windows)]
    {
        if installation.channel.global_worker_id().is_some() {
            return with_elevated_windows_live_image_handoff(installation, release);
        }
        cleanup_stale_windows_update_backups();
        let handoff = WindowsLiveImageHandoff::begin()?;
        let execution = match execute_update(installation, release, quiet_stdout) {
            Ok(execution) => execution,
            Err(message) => {
                return handoff
                    .finish(Err(message))
                    .map(|()| UpdateExecution::Standard)
            }
        };
        handoff.finish_execution(
            execution,
            verify_composite_install(&installation.binary_path, &release.version),
        )
    }

    #[cfg(not(windows))]
    {
        let execution = execute_update(installation, release, quiet_stdout)?;
        verify_composite_install(&installation.binary_path, &release.version)?;
        Ok(execution)
    }
}

fn verify_composite_install(binary: &Path, version: &str) -> std::result::Result<(), String> {
    verify_version(binary, version)?;
    crate::gui::verify_installed(version)
}

fn perform_managed_install(
    channel: InstallChannel,
    release: &Release,
    quiet_stdout: bool,
) -> std::result::Result<(), String> {
    let attempt = || {
        execute_managed_wrapper(channel, release, quiet_stdout)?;
        let binary = managed_receipt_binary().ok_or_else(|| {
            "The managed installer finished without a matching receipt and binary".to_string()
        })?;
        verify_composite_install(&binary, &release.version)
    };

    #[cfg(windows)]
    {
        let current = std::env::current_exe()
            .map_err(|error| format!("Could not resolve the running executable: {error}"))?;
        if expected_windows_binary(InstallChannel::MsiGlobal)
            .is_some_and(|expected| path_eq(&current, &expected))
        {
            if let Some(existing) = detect_windows_native_channel(&current)? {
                if existing.global_worker_id().is_some() {
                    return with_elevated_windows_managed_install_handoff(existing, release);
                }
            }
            return Err(
                "The running executable is in the Global install path, but no proven Global installer owns it. No mutation was attempted."
                    .into(),
            );
        }
        if expected_windows_binary(InstallChannel::MsiCorporate)
            .is_some_and(|expected| path_eq(&current, &expected))
        {
            return with_windows_managed_takeover_handoff(attempt);
        }
        if cargo_binary_path().is_some_and(|expected| path_eq(&current, &expected)) {
            return with_windows_live_image_handoff(attempt);
        }
        if managed_receipt_binary().is_some_and(|expected| path_eq(&current, &expected)) {
            return with_windows_managed_takeover_handoff(attempt);
        }
        attempt()
    }

    #[cfg(not(windows))]
    {
        attempt()
    }
}

fn execute_managed_wrapper(
    channel: InstallChannel,
    release: &Release,
    quiet_stdout: bool,
) -> std::result::Result<(), String> {
    let asset = channel
        .update_asset()
        .ok_or_else(|| "Managed installer asset is unavailable".to_string())?;
    let staged = stage_verified(release, asset)?;

    match channel {
        InstallChannel::PowerShellInstaller => {
            #[cfg(windows)]
            let program = windows_powershell_program()?;
            #[cfg(not(windows))]
            let program: std::ffi::OsString = if tool_exists("pwsh") {
                "pwsh".into()
            } else {
                return Err("PowerShell is not available".into());
            };
            run_status(
                // A pwsh 7 parent's PSModulePath shadows the in-box shell's
                // built-in modules (Get-FileHash fails to auto-load); every
                // PowerShell child rebuilds its own defaults when unset.
                Command::new(&program).env_remove("PSModulePath").args([
                    "-NoProfile",
                    "-NonInteractive",
                    "-ExecutionPolicy",
                    "Bypass",
                    "-File",
                    &staged.path.to_string_lossy(),
                ]),
                quiet_stdout,
            )
        }
        InstallChannel::ShellInstaller => {
            run_status(Command::new("sh").arg(&staged.path), quiet_stdout)
        }
        _ => Err("The selected channel is not a managed wrapper".into()),
    }
}

#[cfg(windows)]
fn windows_msi_repair_required(current_version: &str, target_version: &str) -> bool {
    current_version == target_version
}

#[cfg(windows)]
fn execute_windows_msi(
    channel: InstallChannel,
    release: &Release,
    quiet_stdout: bool,
) -> std::result::Result<WindowsInstallerCompletion, String> {
    let asset = channel
        .update_asset()
        .ok_or_else(|| "MSI asset is unavailable".to_string())?;
    let staged = stage_verified(release, asset)?;
    let msiexec = trusted_windows_system_executable(Path::new("msiexec.exe"))?;
    let mut command = Command::new(msiexec);
    command.args([
        "/i",
        &staged.path.to_string_lossy(),
        "/passive",
        "/norestart",
        "SD300GUIALREADYSTOPPED=1",
    ]);
    if windows_msi_repair_required(VERSION, &release.version) {
        // Repair properties apply only to the already-registered product.
        // Supplying them during a normal version transition targets the new
        // ProductCode as though it were already installed and prevents WiX's
        // MajorUpgrade transaction from running. The same-version companion
        // repair needs them so Windows Installer restores a missing engine or
        // GUI instead of treating `/i` as a no-op.
        command.args(["REINSTALL=ALL", "REINSTALLMODE=vomus"]);
    }
    run_windows_installer_status(&mut command, quiet_stdout)
}

#[cfg(not(windows))]
fn execute_windows_msi(
    _channel: InstallChannel,
    _release: &Release,
    _quiet_stdout: bool,
) -> std::result::Result<WindowsInstallerCompletion, String> {
    Err("MSI updates are Windows-only".into())
}

#[cfg(windows)]
fn execute_windows_exe(
    channel: InstallChannel,
    release: &Release,
    quiet_stdout: bool,
) -> std::result::Result<(), String> {
    let asset = channel
        .update_asset()
        .ok_or_else(|| "EXE asset is unavailable".to_string())?;
    let staged = stage_verified(release, asset)?;
    run_status(
        Command::new(&staged.path).args([
            "/VERYSILENT",
            "/SUPPRESSMSGBOXES",
            "/NORESTART",
            "/SD300GUIALREADYSTOPPED",
        ]),
        quiet_stdout,
    )
}

#[cfg(not(windows))]
fn execute_windows_exe(
    _channel: InstallChannel,
    _release: &Release,
    _quiet_stdout: bool,
) -> std::result::Result<(), String> {
    Err("EXE updates are Windows-only".into())
}

#[cfg(target_os = "macos")]
fn execute_macos_pkg(release: &Release, quiet_stdout: bool) -> std::result::Result<(), String> {
    let staged = stage_verified(release, MAC_PKG_ASSET)?;
    if std::env::var("GITHUB_ACTIONS").as_deref() == Ok("true")
        && std::env::var("SD300_CI_NONINTERACTIVE_PKG").as_deref() == Ok("1")
    {
        return run_status(
            Command::new("sudo").args([
                "installer",
                "-pkg",
                &staged.path.to_string_lossy(),
                "-target",
                "/",
            ]),
            quiet_stdout,
        );
    }
    run_status(
        Command::new("open").args(["-W", &staged.path.to_string_lossy()]),
        quiet_stdout,
    )
}

#[cfg(not(target_os = "macos"))]
fn execute_macos_pkg(_release: &Release, _quiet_stdout: bool) -> std::result::Result<(), String> {
    Err("PKG updates are macOS-only".into())
}

#[cfg(windows)]
fn with_windows_live_image_handoff<F>(attempt: F) -> std::result::Result<(), String>
where
    F: FnOnce() -> std::result::Result<(), String>,
{
    cleanup_stale_windows_update_backups();
    let handoff = WindowsLiveImageHandoff::begin()?;
    handoff.finish(attempt())
}

#[cfg(windows)]
fn with_windows_managed_takeover_handoff<F>(attempt: F) -> std::result::Result<(), String>
where
    F: FnOnce() -> std::result::Result<(), String>,
{
    cleanup_stale_windows_update_backups();
    let handoff = WindowsLiveImageHandoff::begin()?;
    handoff.finish_managed_takeover(attempt())
}

#[cfg(windows)]
fn with_elevated_windows_live_image_handoff(
    installation: &Installation,
    release: &Release,
) -> std::result::Result<UpdateExecution, String> {
    let handoff = WindowsLiveImageHandoff::plan()?;
    let channel = installation
        .channel
        .global_worker_id()
        .ok_or_else(|| "Refused a non-Global elevated update request".to_string())?;
    let exit_code =
        launch_elevated_windows_update_worker(channel, &release.version, &handoff.backup)?;
    if let Some(completion) = windows_committed_worker_failure_completion(exit_code) {
        return Err(format!(
            "Windows Installer committed the Global MSI transaction, but post-install verification or cleanup failed; the prior executable was not restored.{}",
            windows_pending_verification_reboot_message(completion)
        ));
    }
    let msi_completion = (installation.channel == InstallChannel::MsiGlobal)
        .then(|| {
            i32::try_from(exit_code)
                .ok()
                .and_then(windows_installer_completion)
        })
        .flatten();
    if exit_code != 0
        && !matches!(
            msi_completion,
            Some(WindowsInstallerCompletion::RebootRequired(_))
        )
    {
        return Err(format!(
            "The elevated same-channel update worker exited with code {exit_code}. It retained or restored the old executable; verify `sd300 --version` before retrying."
        ));
    }
    let execution = match installation.channel {
        InstallChannel::MsiGlobal => UpdateExecution::WindowsMsiCommitted(
            msi_completion.unwrap_or(WindowsInstallerCompletion::Complete),
        ),
        _ => UpdateExecution::Standard,
    };
    verify_composite_install(&installation.binary_path, &release.version)
        .map_err(|message| execution.post_commit_failure(message))?;
    Ok(execution)
}

#[cfg(windows)]
fn with_elevated_windows_managed_install_handoff(
    existing: InstallChannel,
    release: &Release,
) -> std::result::Result<(), String> {
    let handoff = WindowsLiveImageHandoff::plan()?;
    let channel = existing
        .global_worker_id()
        .ok_or_else(|| "Refused a non-Global elevated managed install request".to_string())?;
    let exit_code =
        launch_elevated_windows_install_worker(channel, &release.version, &handoff.backup)?;
    if exit_code != 0 {
        return Err(format!(
            "The elevated managed install worker exited with code {exit_code}. It retained or restored the Global executable; verify `sd300 --version` before retrying."
        ));
    }
    let binary = managed_receipt_binary().ok_or_else(|| {
        "The elevated managed installer finished without a matching receipt and binary".to_string()
    })?;
    verify_composite_install(&binary, &release.version)
}

#[cfg(windows)]
pub fn run_windows_update_worker(channel: &str, version: &str, backup: &Path) -> i32 {
    let Some(channel) = InstallChannel::from_global_worker_id(channel) else {
        return 2;
    };
    if !is_worker_release_version(version) {
        return 2;
    }
    let installation = match detect_installation() {
        Ok(installation) if installation.channel == channel => installation,
        _ => return 2,
    };
    cleanup_stale_windows_update_backups();
    let handoff = match WindowsLiveImageHandoff::begin_with_backup(backup) {
        Ok(handoff) => handoff,
        Err(message) => {
            eprintln!("SD-300 update worker refused the handoff: {message}");
            return 2;
        }
    };
    let release = Release {
        tag: format!("v{version}"),
        version: version.to_string(),
    };
    let execution = match execute_update(&installation, &release, true) {
        Ok(execution) => execution,
        Err(message) => {
            return match handoff.finish(Err(message)) {
                Ok(()) => 2,
                Err(message) => {
                    eprintln!("SD-300 update worker failed safely: {message}");
                    2
                }
            }
        }
    };
    let verification = verify_composite_install(&handoff.original, version);
    match handoff.finish_execution(execution, verification) {
        Ok(execution) => execution.worker_exit_code(),
        Err(message) => {
            eprintln!("SD-300 update worker failed safely: {message}");
            if execution.windows_msi_committed() {
                execution.committed_worker_failure_exit_code()
            } else {
                2
            }
        }
    }
}

#[cfg(windows)]
pub fn run_windows_install_worker(channel: &str, version: &str, backup: &Path) -> i32 {
    let Some(existing) = InstallChannel::from_global_worker_id(channel) else {
        return 2;
    };
    if !is_worker_release_version(version) {
        return 2;
    }
    let current = match std::env::current_exe() {
        Ok(current) => current,
        Err(_) => return 2,
    };
    match detect_windows_native_channel(&current) {
        Ok(Some(channel)) if channel == existing => {}
        _ => return 2,
    }
    cleanup_stale_windows_update_backups();
    let handoff = match WindowsLiveImageHandoff::begin_with_backup(backup) {
        Ok(handoff) => handoff,
        Err(message) => {
            eprintln!("SD-300 install worker refused the handoff: {message}");
            return 2;
        }
    };
    let release = Release {
        tag: format!("v{version}"),
        version: version.to_string(),
    };
    let outcome = execute_managed_wrapper(InstallChannel::PowerShellInstaller, &release, true)
        .and_then(|()| {
            let binary = managed_receipt_binary().ok_or_else(|| {
                "The managed installer finished without a matching receipt and binary".to_string()
            })?;
            verify_composite_install(&binary, version)
        });
    match handoff.finish_managed_takeover(outcome) {
        Ok(()) => 0,
        Err(message) => {
            eprintln!("SD-300 install worker failed safely: {message}");
            2
        }
    }
}

#[cfg(not(windows))]
pub fn run_windows_install_worker(_channel: &str, _version: &str, _backup: &Path) -> i32 {
    2
}

#[cfg(not(windows))]
pub fn run_windows_update_worker(_channel: &str, _version: &str, _backup: &Path) -> i32 {
    2
}

#[cfg(windows)]
/// The elevated worker's stdio is lost across the UAC boundary, so its
/// failure detail is relayed through a report file at a path both sides
/// derive from the parent-chosen backup location.
#[cfg(any(windows, test))]
fn worker_error_report_path(backup: &Path) -> PathBuf {
    backup.with_extension("uninstall-error.log")
}

#[cfg(windows)]
pub fn run_windows_uninstall_worker(channel: &str, backup: &Path) -> i32 {
    let report = |message: &str| {
        let _ = std::fs::write(worker_error_report_path(backup), message);
    };
    let Some(channel) = InstallChannel::from_global_worker_id(channel) else {
        report("the uninstall worker received an unknown channel");
        return 2;
    };
    let installation = match detect_installation() {
        Ok(installation) if installation.channel == channel => installation,
        Ok(_) => {
            report("the detected installation no longer matches the requested channel");
            return 2;
        }
        Err(message) => {
            report(&message);
            eprintln!("SD-300 uninstall worker preflight failed safely: {message}");
            return 2;
        }
    };
    let handoff = match WindowsUninstallImageHandoff::begin_with_backup(backup) {
        Ok(handoff) => handoff,
        Err(message) => {
            report(&message);
            eprintln!("SD-300 uninstall worker failed safely: {message}");
            return 2;
        }
    };
    let execution = match execute_windows_native_uninstaller(&installation, true) {
        Ok(execution) => execution,
        Err(message) => {
            report(&message);
            return match handoff.finish(Err(message)) {
                Ok(()) => 2,
                Err(message) => {
                    report(&message);
                    eprintln!("SD-300 uninstall worker failed safely: {message}");
                    2
                }
            };
        }
    };
    let verification = verify_windows_native_uninstalled_converged(&installation);
    match handoff.finish_execution(execution, verification) {
        Ok(execution) => execution.worker_exit_code(),
        Err(message) => {
            report(&message);
            eprintln!("SD-300 uninstall worker failed safely: {message}");
            if execution.windows_msi_committed() {
                execution.committed_worker_failure_exit_code()
            } else {
                2
            }
        }
    }
}

#[cfg(not(windows))]
pub fn run_windows_uninstall_worker(_channel: &str, _backup: &Path) -> i32 {
    2
}

fn is_worker_release_version(version: &str) -> bool {
    version.len() <= 64
        && version.split('.').count() == 3
        && version
            .split('.')
            .all(|part| !part.is_empty() && part.bytes().all(|byte| byte.is_ascii_digit()))
}

#[cfg(windows)]
fn launch_elevated_windows_update_worker(
    channel: &str,
    version: &str,
    backup: &Path,
) -> std::result::Result<u32, String> {
    use std::os::windows::ffi::OsStrExt;
    use std::ptr::{null, null_mut};
    use winapi::um::errhandlingapi::GetLastError;
    use winapi::um::handleapi::CloseHandle;
    use winapi::um::processthreadsapi::GetExitCodeProcess;
    use winapi::um::shellapi::{
        ShellExecuteExW, SEE_MASK_NOASYNC, SEE_MASK_NOCLOSEPROCESS, SHELLEXECUTEINFOW,
    };
    use winapi::um::synchapi::WaitForSingleObject;
    use winapi::um::winbase::{INFINITE, WAIT_OBJECT_0};
    use winapi::um::winuser::SW_HIDE;

    if InstallChannel::from_global_worker_id(channel).is_none()
        || !is_worker_release_version(version)
    {
        return Err("Refused an invalid elevated Global update request".into());
    }
    let current = std::env::current_exe()
        .map_err(|error| format!("Could not resolve the Global executable: {error}"))?;
    validate_windows_update_backup(&current, backup)?;
    let backup = backup.to_str().ok_or_else(|| {
        "The Global update path cannot be represented safely in the worker command line".to_string()
    })?;
    let parameters = format!(
        "update-worker --update-channel {channel} --update-version {version} --update-backup {}",
        windows_quote_command_arg(backup)
    );
    let verb: Vec<u16> = std::ffi::OsStr::new("runas")
        .encode_wide()
        .chain(Some(0))
        .collect();
    let file: Vec<u16> = current.as_os_str().encode_wide().chain(Some(0)).collect();
    let parameters: Vec<u16> = std::ffi::OsStr::new(&parameters)
        .encode_wide()
        .chain(Some(0))
        .collect();
    let mut info: SHELLEXECUTEINFOW = unsafe { std::mem::zeroed() };
    info.cbSize = std::mem::size_of::<SHELLEXECUTEINFOW>() as u32;
    info.fMask = SEE_MASK_NOCLOSEPROCESS | SEE_MASK_NOASYNC;
    info.hwnd = null_mut();
    info.lpVerb = verb.as_ptr();
    info.lpFile = file.as_ptr();
    info.lpParameters = parameters.as_ptr();
    info.lpDirectory = null();
    info.nShow = SW_HIDE;

    if unsafe { ShellExecuteExW(&mut info) } == 0 {
        let code = unsafe { GetLastError() };
        return if code == 1223 {
            Err("UAC was cancelled; the Global installation was not changed".into())
        } else {
            Err(format!(
                "Could not start the elevated same-channel update worker (Windows error {code}: {})",
                std::io::Error::from_raw_os_error(code as i32)
            ))
        };
    }
    if info.hProcess.is_null() {
        return Err("Windows returned no update-worker process handle".into());
    }
    let wait = unsafe { WaitForSingleObject(info.hProcess, INFINITE) };
    if wait != WAIT_OBJECT_0 {
        unsafe { CloseHandle(info.hProcess) };
        return Err(format!("Waiting for the update worker failed ({wait})"));
    }
    let mut exit_code = 0u32;
    let got_exit = unsafe { GetExitCodeProcess(info.hProcess, &mut exit_code) };
    unsafe { CloseHandle(info.hProcess) };
    if got_exit == 0 {
        return Err("Windows did not return the update-worker exit code".into());
    }
    Ok(exit_code)
}

#[cfg(windows)]
fn launch_elevated_windows_install_worker(
    channel: &str,
    version: &str,
    backup: &Path,
) -> std::result::Result<u32, String> {
    use std::os::windows::ffi::OsStrExt;
    use std::ptr::{null, null_mut};
    use winapi::um::errhandlingapi::GetLastError;
    use winapi::um::handleapi::CloseHandle;
    use winapi::um::processthreadsapi::GetExitCodeProcess;
    use winapi::um::shellapi::{
        ShellExecuteExW, SEE_MASK_NOASYNC, SEE_MASK_NOCLOSEPROCESS, SHELLEXECUTEINFOW,
    };
    use winapi::um::synchapi::WaitForSingleObject;
    use winapi::um::winbase::{INFINITE, WAIT_OBJECT_0};
    use winapi::um::winuser::SW_HIDE;

    if InstallChannel::from_global_worker_id(channel).is_none()
        || !is_worker_release_version(version)
    {
        return Err("Refused an invalid elevated Global managed install request".into());
    }
    let current = std::env::current_exe()
        .map_err(|error| format!("Could not resolve the Global executable: {error}"))?;
    validate_windows_update_backup(&current, backup)?;
    let backup = backup.to_str().ok_or_else(|| {
        "The Global install path cannot be represented safely in the worker command line"
            .to_string()
    })?;
    let parameters = format!(
        "install-worker --install-channel {channel} --install-version {version} --install-backup {}",
        windows_quote_command_arg(backup)
    );
    let verb: Vec<u16> = std::ffi::OsStr::new("runas")
        .encode_wide()
        .chain(Some(0))
        .collect();
    let file: Vec<u16> = current.as_os_str().encode_wide().chain(Some(0)).collect();
    let parameters: Vec<u16> = std::ffi::OsStr::new(&parameters)
        .encode_wide()
        .chain(Some(0))
        .collect();
    let mut info: SHELLEXECUTEINFOW = unsafe { std::mem::zeroed() };
    info.cbSize = std::mem::size_of::<SHELLEXECUTEINFOW>() as u32;
    info.fMask = SEE_MASK_NOCLOSEPROCESS | SEE_MASK_NOASYNC;
    info.hwnd = null_mut();
    info.lpVerb = verb.as_ptr();
    info.lpFile = file.as_ptr();
    info.lpParameters = parameters.as_ptr();
    info.lpDirectory = null();
    info.nShow = SW_HIDE;

    if unsafe { ShellExecuteExW(&mut info) } == 0 {
        let code = unsafe { GetLastError() };
        return if code == 1223 {
            Err("UAC was cancelled; the Global installation was not changed".into())
        } else {
            Err(format!(
                "Could not start the elevated managed install worker (Windows error {code}: {})",
                std::io::Error::from_raw_os_error(code as i32)
            ))
        };
    }
    if info.hProcess.is_null() {
        return Err("Windows returned no install-worker process handle".into());
    }
    let wait = unsafe { WaitForSingleObject(info.hProcess, INFINITE) };
    if wait != WAIT_OBJECT_0 {
        unsafe { CloseHandle(info.hProcess) };
        return Err(format!("Waiting for the install worker failed ({wait})"));
    }
    let mut exit_code = 0u32;
    let got_exit = unsafe { GetExitCodeProcess(info.hProcess, &mut exit_code) };
    unsafe { CloseHandle(info.hProcess) };
    if got_exit == 0 {
        return Err("Windows did not return the install-worker exit code".into());
    }
    Ok(exit_code)
}

#[cfg(windows)]
fn launch_elevated_windows_uninstall_worker(
    channel: &str,
    backup: &Path,
) -> std::result::Result<u32, String> {
    use std::os::windows::ffi::OsStrExt;
    use std::ptr::{null, null_mut};
    use winapi::um::errhandlingapi::GetLastError;
    use winapi::um::handleapi::CloseHandle;
    use winapi::um::processthreadsapi::GetExitCodeProcess;
    use winapi::um::shellapi::{
        ShellExecuteExW, SEE_MASK_NOASYNC, SEE_MASK_NOCLOSEPROCESS, SHELLEXECUTEINFOW,
    };
    use winapi::um::synchapi::WaitForSingleObject;
    use winapi::um::winbase::{INFINITE, WAIT_OBJECT_0};
    use winapi::um::winuser::SW_HIDE;

    if InstallChannel::from_global_worker_id(channel).is_none() {
        return Err("Refused an invalid elevated Global uninstall request".into());
    }
    let current = std::env::current_exe()
        .map_err(|error| format!("Could not resolve the Global executable: {error}"))?;
    validate_windows_uninstall_backup(&current, backup)?;
    let backup = backup.to_str().ok_or_else(|| {
        "The Global uninstall path cannot be represented safely in the worker command line"
            .to_string()
    })?;
    let parameters = format!(
        "uninstall-worker --uninstall-channel {channel} --uninstall-backup {}",
        windows_quote_command_arg(backup)
    );
    let verb: Vec<u16> = std::ffi::OsStr::new("runas")
        .encode_wide()
        .chain(Some(0))
        .collect();
    let file: Vec<u16> = current.as_os_str().encode_wide().chain(Some(0)).collect();
    let parameters: Vec<u16> = std::ffi::OsStr::new(&parameters)
        .encode_wide()
        .chain(Some(0))
        .collect();
    let mut info: SHELLEXECUTEINFOW = unsafe { std::mem::zeroed() };
    info.cbSize = std::mem::size_of::<SHELLEXECUTEINFOW>() as u32;
    info.fMask = SEE_MASK_NOCLOSEPROCESS | SEE_MASK_NOASYNC;
    info.hwnd = null_mut();
    info.lpVerb = verb.as_ptr();
    info.lpFile = file.as_ptr();
    info.lpParameters = parameters.as_ptr();
    info.lpDirectory = null();
    info.nShow = SW_HIDE;

    if unsafe { ShellExecuteExW(&mut info) } == 0 {
        let code = unsafe { GetLastError() };
        return if code == 1223 {
            Err("UAC was cancelled; the Global installation was not changed".into())
        } else {
            Err(format!(
                "Could not start the elevated Global uninstall worker (Windows error {code}: {})",
                std::io::Error::from_raw_os_error(code as i32)
            ))
        };
    }
    if info.hProcess.is_null() {
        return Err("Windows returned no uninstall-worker process handle".into());
    }
    let wait = unsafe { WaitForSingleObject(info.hProcess, INFINITE) };
    if wait != WAIT_OBJECT_0 {
        unsafe { CloseHandle(info.hProcess) };
        return Err(format!("Waiting for the uninstall worker failed ({wait})"));
    }
    let mut exit_code = 0u32;
    let got_exit = unsafe { GetExitCodeProcess(info.hProcess, &mut exit_code) };
    unsafe { CloseHandle(info.hProcess) };
    if got_exit == 0 {
        return Err("Windows did not return the uninstall-worker exit code".into());
    }
    Ok(exit_code)
}

#[cfg(any(windows, test))]
fn windows_quote_command_arg(value: &str) -> String {
    if !value.is_empty() && !value.chars().any(|ch| ch.is_whitespace() || ch == '"') {
        return value.to_string();
    }
    let mut quoted = String::from("\"");
    let mut backslashes = 0usize;
    for ch in value.chars() {
        if ch == '\\' {
            backslashes += 1;
        } else if ch == '"' {
            quoted.push_str(&"\\".repeat(backslashes * 2 + 1));
            quoted.push('"');
            backslashes = 0;
        } else {
            quoted.push_str(&"\\".repeat(backslashes));
            backslashes = 0;
            quoted.push(ch);
        }
    }
    quoted.push_str(&"\\".repeat(backslashes * 2));
    quoted.push('"');
    quoted
}

#[cfg(windows)]
fn validate_windows_update_backup(
    original: &Path,
    backup: &Path,
) -> std::result::Result<(), String> {
    let original_is_product = original
        .file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| name.eq_ignore_ascii_case("sd300.exe"));
    let same_parent = original
        .parent()
        .zip(backup.parent())
        .is_some_and(|(left, right)| path_eq(left, right));
    let valid_name = backup
        .file_name()
        .and_then(|name| name.to_str())
        .is_some_and(is_windows_update_backup_name);
    if !original.is_absolute()
        || !backup.is_absolute()
        || !original_is_product
        || !same_parent
        || !valid_name
        || std::fs::symlink_metadata(backup).is_ok()
    {
        return Err("Refused an invalid or pre-existing Windows update backup path".into());
    }
    Ok(())
}

#[cfg(windows)]
fn cleanup_stale_windows_update_backups() {
    let Ok(current) = std::env::current_exe() else {
        return;
    };
    if !current
        .file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| name.eq_ignore_ascii_case("sd300.exe"))
    {
        return;
    }
    let Some(parent) = current.parent() else {
        return;
    };
    let Ok(entries) = std::fs::read_dir(parent) else {
        return;
    };
    for entry in entries.flatten() {
        if entry
            .file_name()
            .to_str()
            .is_some_and(is_windows_update_backup_name)
        {
            let _ = std::fs::remove_file(entry.path());
        }
    }
}

#[cfg(windows)]
struct WindowsLiveImageHandoff {
    original: PathBuf,
    backup: PathBuf,
}

#[cfg(windows)]
impl WindowsLiveImageHandoff {
    fn plan() -> std::result::Result<Self, String> {
        let original = std::env::current_exe()
            .map_err(|error| format!("Could not resolve the running executable: {error}"))?;
        let parent = original
            .parent()
            .ok_or_else(|| "Running executable has no parent directory".to_string())?;
        let nonce = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        let backup = parent.join(format!(
            ".sd300-update-backup-{}-{nonce}.exe",
            std::process::id()
        ));
        validate_windows_update_backup(&original, &backup)?;
        Ok(Self { original, backup })
    }

    fn begin() -> std::result::Result<Self, String> {
        Self::plan()?.rename_live_image()
    }

    fn begin_with_backup(backup: &Path) -> std::result::Result<Self, String> {
        let original = std::env::current_exe()
            .map_err(|error| format!("Could not resolve the worker executable: {error}"))?;
        validate_windows_update_backup(&original, backup)?;
        Self {
            original,
            backup: backup.to_path_buf(),
        }
        .rename_live_image()
    }

    fn rename_live_image(self) -> std::result::Result<Self, String> {
        std::fs::rename(&self.original, &self.backup).map_err(|error| {
            format!(
                "Could not reserve the running executable for rollback ({} -> {}): {error}",
                self.original.display(),
                self.backup.display()
            )
        })?;
        Ok(self)
    }

    fn finish(self, result: std::result::Result<(), String>) -> std::result::Result<(), String> {
        match result {
            Ok(()) => {
                self.spawn_cleanup().map_err(|error| {
                    format!(
                        "The new version verified, but safe cleanup could not start for {}: {error}",
                        self.backup.display()
                    )
                })?;
                Ok(())
            }
            Err(message) => match self.rollback() {
                Ok(()) => Err(format!("{message}; the prior executable was restored")),
                Err(error) => Err(format!(
                    "{message}; rollback also failed ({error}); the prior executable remains at {}",
                    self.backup.display()
                )),
            },
        }
    }

    fn finish_execution(
        self,
        execution: UpdateExecution,
        verification: std::result::Result<(), String>,
    ) -> std::result::Result<UpdateExecution, String> {
        if execution.windows_msi_committed() {
            self.finish_committed_msi(verification)?;
        } else {
            self.finish(verification)?;
        }
        Ok(execution)
    }

    fn finish_committed_msi(
        self,
        verification: std::result::Result<(), String>,
    ) -> std::result::Result<(), String> {
        let cleanup = schedule_windows_update_takeover_cleanup(&self.backup);
        match (verification, cleanup) {
            (Ok(()), Ok(())) => Ok(()),
            (Err(message), Ok(())) => Err(format!(
                "{message}; Windows Installer committed the product transaction, so the prior executable was not restored"
            )),
            (Ok(()), Err(error)) => Err(format!(
                "The MSI transaction verified, but safe cleanup could not start for {}: {error}",
                self.backup.display()
            )),
            (Err(message), Err(error)) => Err(format!(
                "{message}; Windows Installer committed the product transaction, so the prior executable was not restored; cleanup also could not start for {}: {error}",
                self.backup.display()
            )),
        }
    }

    fn finish_managed_takeover(
        self,
        result: std::result::Result<(), String>,
    ) -> std::result::Result<(), String> {
        match result {
            Ok(()) => schedule_windows_update_takeover_cleanup(&self.backup).map_err(|error| {
                format!(
                    "The managed install verified, but final cleanup could not start for {}: {error}",
                    self.backup.display()
                )
            }),
            Err(message) => match self.rollback() {
                Ok(()) => Err(format!("{message}; the prior executable was restored")),
                Err(error) => Err(format!(
                    "{message}; rollback also failed ({error}); the prior executable remains at {}",
                    self.backup.display()
                )),
            },
        }
    }

    fn rollback(&self) -> std::io::Result<()> {
        if self.original.exists() {
            std::fs::remove_file(&self.original)?;
        }
        std::fs::rename(&self.backup, &self.original)
    }

    fn spawn_cleanup(&self) -> std::io::Result<()> {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        Command::new(&self.original)
            .arg("update-cleanup")
            .arg("--update-backup")
            .arg(&self.backup)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .creation_flags(CREATE_NO_WINDOW)
            .spawn()
            .map(|_| ())
    }
}

#[cfg(windows)]
pub fn cleanup_windows_update_backup(backup: &Path) -> i32 {
    let Ok(current) = std::env::current_exe() else {
        return 2;
    };
    let valid = current
        .file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| name.eq_ignore_ascii_case("sd300.exe"))
        && current
            .parent()
            .zip(backup.parent())
            .is_some_and(|(left, right)| path_eq(left, right))
        && backup
            .file_name()
            .and_then(|name| name.to_str())
            .is_some_and(is_windows_update_backup_name)
        && backup.is_absolute();
    if !valid {
        return 2;
    }
    for _ in 0..600 {
        match std::fs::remove_file(backup) {
            Ok(()) => return 0,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return 0,
            Err(_) => std::thread::sleep(std::time::Duration::from_millis(100)),
        }
    }
    2
}

#[cfg(not(windows))]
pub fn cleanup_windows_update_backup(_backup: &Path) -> i32 {
    2
}

#[cfg(any(windows, test))]
fn is_windows_update_backup_name(name: &str) -> bool {
    let Some(body) = name
        .strip_prefix(".sd300-update-backup-")
        .and_then(|name| name.strip_suffix(".exe"))
    else {
        return false;
    };
    let Some((pid, nonce)) = body.split_once('-') else {
        return false;
    };
    !pid.is_empty()
        && !nonce.is_empty()
        && pid.bytes().all(|byte| byte.is_ascii_digit())
        && nonce.bytes().all(|byte| byte.is_ascii_digit())
}

#[cfg(windows)]
struct WindowsUninstallImageHandoff {
    original: PathBuf,
    backup: PathBuf,
}

#[cfg(windows)]
impl WindowsUninstallImageHandoff {
    fn plan() -> std::result::Result<Self, String> {
        let original = std::env::current_exe()
            .map_err(|error| format!("Could not resolve the running executable: {error}"))?;
        let parent = original
            .parent()
            .ok_or_else(|| "Running executable has no parent directory".to_string())?;
        let nonce = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        let backup = parent.join(format!(
            ".sd300-uninstall-backup-{}-{nonce}.exe",
            std::process::id()
        ));
        validate_windows_uninstall_backup(&original, &backup)?;
        Ok(Self { original, backup })
    }

    fn begin() -> std::result::Result<Self, String> {
        Self::plan()?.rename_live_image()
    }

    fn begin_with_backup(backup: &Path) -> std::result::Result<Self, String> {
        let original = std::env::current_exe()
            .map_err(|error| format!("Could not resolve the uninstall worker: {error}"))?;
        validate_windows_uninstall_backup(&original, backup)?;
        Self {
            original,
            backup: backup.to_path_buf(),
        }
        .rename_live_image()
    }

    fn rename_live_image(self) -> std::result::Result<Self, String> {
        std::fs::rename(&self.original, &self.backup).map_err(|error| {
            format!(
                "Could not retire the running executable before uninstall ({} -> {}): {error}",
                self.original.display(),
                self.backup.display()
            )
        })?;
        Ok(self)
    }

    fn finish(self, result: std::result::Result<(), String>) -> std::result::Result<(), String> {
        match result {
            Ok(()) => schedule_windows_uninstall_cleanup(&self.backup).map_err(|error| {
                format!(
                    "The registered uninstall completed, but final cleanup could not start for {}: {error}",
                    self.backup.display()
                )
            }),
            Err(message) => match self.rollback() {
                Ok(()) => Err(format!("{message}; the installed executable was restored")),
                Err(error) => Err(format!(
                    "{message}; rollback also failed ({error}); the executable remains at {}",
                    self.backup.display()
                )),
            },
        }
    }

    fn finish_execution(
        self,
        execution: WindowsNativeUninstallExecution,
        verification: std::result::Result<(), String>,
    ) -> std::result::Result<WindowsNativeUninstallExecution, String> {
        if execution.windows_msi_committed() {
            self.finish_committed_msi(verification)?;
        } else {
            self.finish(verification)?;
        }
        Ok(execution)
    }

    fn finish_committed_msi(
        self,
        verification: std::result::Result<(), String>,
    ) -> std::result::Result<(), String> {
        let cleanup = schedule_windows_uninstall_cleanup(&self.backup);
        match (verification, cleanup) {
            (Ok(()), Ok(())) => Ok(()),
            (Err(message), Ok(())) => Err(format!(
                "{message}; Windows Installer committed the uninstall transaction, so the installed executable was not restored"
            )),
            (Ok(()), Err(error)) => Err(format!(
                "The MSI uninstall committed, but final cleanup could not start for {}: {error}",
                self.backup.display()
            )),
            (Err(message), Err(error)) => Err(format!(
                "{message}; Windows Installer committed the uninstall transaction, so the installed executable was not restored; final cleanup also could not start for {}: {error}",
                self.backup.display()
            )),
        }
    }

    fn rollback(&self) -> std::io::Result<()> {
        if self.original.exists() {
            std::fs::remove_file(&self.original)?;
        }
        std::fs::rename(&self.backup, &self.original)
    }
}

#[cfg(windows)]
fn validate_windows_uninstall_backup(
    original: &Path,
    backup: &Path,
) -> std::result::Result<(), String> {
    let original_is_product = original
        .file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| name.eq_ignore_ascii_case("sd300.exe"));
    let same_parent = original
        .parent()
        .zip(backup.parent())
        .is_some_and(|(left, right)| path_eq(left, right));
    let valid_name = backup
        .file_name()
        .and_then(|name| name.to_str())
        .is_some_and(is_windows_uninstall_backup_name);
    if original_is_product && same_parent && valid_name && backup.is_absolute() {
        Ok(())
    } else {
        Err("Refused an invalid Windows uninstall live-image path".into())
    }
}

#[cfg(any(windows, test))]
fn is_windows_uninstall_backup_name(name: &str) -> bool {
    let Some(body) = name
        .strip_prefix(".sd300-uninstall-backup-")
        .and_then(|name| name.strip_suffix(".exe"))
    else {
        return false;
    };
    let Some((pid, nonce)) = body.split_once('-') else {
        return false;
    };
    !pid.is_empty()
        && !nonce.is_empty()
        && pid.bytes().all(|byte| byte.is_ascii_digit())
        && nonce.bytes().all(|byte| byte.is_ascii_digit())
}

#[cfg(windows)]
fn schedule_windows_uninstall_cleanup(backup: &Path) -> std::result::Result<(), String> {
    use std::os::windows::process::CommandExt;
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    const SCRIPT: &str = "$target=$env:SD300_UNINSTALL_BACKUP; $bin=[IO.Path]::GetDirectoryName($target); $root=[IO.Path]::GetDirectoryName($bin); for($i=0;$i -lt 600;$i++){try{[IO.File]::Delete($target)}catch{}; if(-not [IO.File]::Exists($target)){try{[IO.Directory]::Delete($bin,$false)}catch{}; try{[IO.Directory]::Delete($root,$false)}catch{}; exit 0}; Start-Sleep -Milliseconds 100}; exit 1";
    let powershell =
        trusted_windows_system_executable(Path::new("WindowsPowerShell\\v1.0\\powershell.exe"))?;
    Command::new(powershell)
        .args([
            "-NoLogo",
            "-NoProfile",
            "-NonInteractive",
            "-WindowStyle",
            "Hidden",
            "-Command",
            SCRIPT,
        ])
        .env("SD300_UNINSTALL_BACKUP", backup.as_os_str())
        .creation_flags(CREATE_NO_WINDOW)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map(|_| ())
        .map_err(|error| format!("Could not schedule trusted uninstall cleanup: {error}"))
}

#[cfg(windows)]
fn schedule_windows_update_takeover_cleanup(backup: &Path) -> std::result::Result<(), String> {
    use std::os::windows::process::CommandExt;
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    const SCRIPT: &str = "$target=$env:SD300_UPDATE_BACKUP; $bin=[IO.Path]::GetDirectoryName($target); $root=[IO.Path]::GetDirectoryName($bin); for($i=0;$i -lt 600;$i++){try{[IO.File]::Delete($target)}catch{}; if(-not [IO.File]::Exists($target)){try{[IO.Directory]::Delete($bin,$false)}catch{}; try{[IO.Directory]::Delete($root,$false)}catch{}; exit 0}; Start-Sleep -Milliseconds 100}; exit 1";
    let powershell =
        trusted_windows_system_executable(Path::new("WindowsPowerShell\\v1.0\\powershell.exe"))?;
    Command::new(powershell)
        .args([
            "-NoLogo",
            "-NoProfile",
            "-NonInteractive",
            "-WindowStyle",
            "Hidden",
            "-Command",
            SCRIPT,
        ])
        .env("SD300_UPDATE_BACKUP", backup.as_os_str())
        .creation_flags(CREATE_NO_WINDOW)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map(|_| ())
        .map_err(|error| format!("Could not schedule trusted managed-install cleanup: {error}"))
}

#[cfg(windows)]
fn trusted_windows_system_executable(relative: &Path) -> std::result::Result<PathBuf, String> {
    use std::os::windows::ffi::OsStringExt;
    use winapi::um::sysinfoapi::GetSystemDirectoryW;

    let mut buffer = vec![0_u16; 32_768];
    let length = unsafe { GetSystemDirectoryW(buffer.as_mut_ptr(), buffer.len() as u32) };
    if length == 0 {
        return Err(format!(
            "Could not resolve the trusted Windows system directory: {}",
            std::io::Error::last_os_error()
        ));
    }
    let length = length as usize;
    if length >= buffer.len() {
        return Err("Windows system directory exceeded the trusted path buffer".into());
    }
    let system = PathBuf::from(std::ffi::OsString::from_wide(&buffer[..length]));
    let executable = system.join(relative);
    if !system.is_absolute() || !executable.is_file() {
        return Err(format!(
            "Trusted Windows system executable was not found at {}",
            executable.display()
        ));
    }
    Ok(executable)
}

struct StagedAsset {
    _directory: tempfile::TempDir,
    path: PathBuf,
}

fn stage_verified(release: &Release, asset: &str) -> std::result::Result<StagedAsset, String> {
    let directory = tempfile::Builder::new()
        .prefix("sd300-update-")
        .tempdir()
        .map_err(|error| format!("Could not create private staging: {error}"))?;
    let path = directory.path().join(asset);
    let checksum_path = directory.path().join(format!("{asset}.sha256"));
    if let Some(source) = ci_asset_directory()? {
        std::fs::copy(source.join(asset), &path)
            .map_err(|error| format!("Could not stage CI candidate asset {asset}: {error}"))?;
        std::fs::copy(source.join(format!("{asset}.sha256")), &checksum_path).map_err(|error| {
            format!("Could not stage CI candidate sidecar for {asset}: {error}")
        })?;
    } else {
        let base = format!("{RELEASE_BASE}/{}", release.tag);
        download(&format!("{base}/{asset}"), &path)?;
        download(&format!("{base}/{asset}.sha256"), &checksum_path)?;
    }
    verify_sha256(&path, &checksum_path)?;
    Ok(StagedAsset {
        _directory: directory,
        path,
    })
}

fn ci_asset_directory() -> std::result::Result<Option<PathBuf>, String> {
    let Some(value) = std::env::var_os("SD300_CI_RELEASE_ASSET_DIR") else {
        return Ok(None);
    };
    if std::env::var("GITHUB_ACTIONS").as_deref() != Ok("true") {
        return Err("SD300_CI_RELEASE_ASSET_DIR is restricted to GitHub Actions".into());
    }
    let path = PathBuf::from(value);
    if !path.is_absolute() || !path.is_dir() {
        return Err("The GitHub Actions candidate asset directory is invalid".into());
    }
    Ok(Some(path))
}

fn download(url: &str, destination: &Path) -> std::result::Result<(), String> {
    #[cfg(windows)]
    {
        let program = windows_powershell_program()
            .map_err(|_| "PowerShell is required to download release assets".to_string())?;
        let script = format!(
            "$ProgressPreference='SilentlyContinue'; $ErrorActionPreference='Stop'; Invoke-WebRequest -UseBasicParsing -Uri '{}' -OutFile '{}'",
            powershell_escape(url),
            powershell_escape(&destination.to_string_lossy())
        );
        run_status(
            // See execute_managed_wrapper: inherited pwsh module paths break
            // the in-box shell's auto-loading; children rebuild defaults.
            Command::new(program).env_remove("PSModulePath").args([
                "-NoProfile",
                "-NonInteractive",
                "-ExecutionPolicy",
                "Bypass",
                "-Command",
                &script,
            ]),
            true,
        )
    }

    #[cfg(not(windows))]
    {
        if tool_exists("curl") {
            return run_status(
                Command::new("curl").args([
                    "--proto",
                    "=https",
                    "--tlsv1.2",
                    "-fLsS",
                    url,
                    "-o",
                    &destination.to_string_lossy(),
                ]),
                true,
            );
        }
        if tool_exists("wget") {
            return run_status(
                Command::new("wget").args(["-q", "-O", &destination.to_string_lossy(), url]),
                true,
            );
        }
        Err("curl or wget is required to download release assets".into())
    }
}

#[cfg(any(windows, test))]
fn powershell_escape(value: &str) -> String {
    value.replace('\'', "''")
}

fn verify_sha256(path: &Path, checksum_path: &Path) -> std::result::Result<(), String> {
    let checksum = std::fs::read_to_string(checksum_path)
        .map_err(|error| format!("Could not read SHA-256 sidecar: {error}"))?;
    let expected = checksum
        .split_whitespace()
        .find(|field| {
            field.len() == 64 && field.chars().all(|character| character.is_ascii_hexdigit())
        })
        .ok_or_else(|| "SHA-256 sidecar did not contain one 64-character digest".to_string())?;
    let bytes = std::fs::read(path)
        .map_err(|error| format!("Could not read staged release asset: {error}"))?;
    let actual = format!("{:x}", Sha256::digest(bytes));
    if actual.eq_ignore_ascii_case(expected) {
        Ok(())
    } else {
        Err(format!(
            "SHA-256 mismatch for {}: expected {}, received {}",
            path.file_name()
                .and_then(|name| name.to_str())
                .unwrap_or("asset"),
            expected,
            actual
        ))
    }
}

fn run_status(command: &mut Command, quiet_stdout: bool) -> std::result::Result<(), String> {
    if quiet_stdout {
        command.stdout(Stdio::null());
    }
    let program = format!("{:?}", command.get_program());
    match command.status() {
        Ok(status) if status.success() => Ok(()),
        Ok(status) => Err(format!(
            "{} exited with code {}",
            program,
            status.code().unwrap_or(-1)
        )),
        Err(error) => Err(format!("Could not start {}: {}", program, error)),
    }
}

#[cfg(any(windows, test))]
fn windows_installer_completion(exit_code: i32) -> Option<WindowsInstallerCompletion> {
    match exit_code {
        0 => Some(WindowsInstallerCompletion::Complete),
        // ERROR_SUCCESS_REBOOT_INITIATED and ERROR_SUCCESS_REBOOT_REQUIRED
        1641 | 3010 => Some(WindowsInstallerCompletion::RebootRequired(exit_code)),
        _ => None,
    }
}

#[cfg(windows)]
fn run_windows_installer_status(
    command: &mut Command,
    quiet_stdout: bool,
) -> std::result::Result<WindowsInstallerCompletion, String> {
    if quiet_stdout {
        command.stdout(Stdio::null());
    }
    let program = format!("{:?}", command.get_program());
    match command.status() {
        Ok(status) => {
            let exit_code = status.code().unwrap_or(-1);
            windows_installer_completion(exit_code)
                .ok_or_else(|| format!("{program} exited with code {exit_code}"))
        }
        Err(error) => Err(format!("Could not start {program}: {error}")),
    }
}

fn verify_version(path: &Path, expected: &str) -> std::result::Result<(), String> {
    if !path.is_file() {
        return Err(format!("Installed binary is missing: {}", path.display()));
    }
    let output = run_output(
        path.as_os_str(),
        ["--version"],
        CommandTimeout::Custom(std::time::Duration::from_secs(15)),
    )
    .ok_or_else(|| format!("Installed binary did not run: {}", path.display()))?;
    let reported = String::from_utf8_lossy(&output.stdout);
    if output.status.success()
        && reported
            .lines()
            .next()
            .is_some_and(|line| line.trim() == format!("sd300 {expected}"))
    {
        Ok(())
    } else {
        Err(format!(
            "Installed binary did not report sd300 {expected}: {}",
            reported.lines().next().unwrap_or("no version output")
        ))
    }
}

fn fetch_latest_release() -> std::result::Result<Release, String> {
    if let Some(release) = ci_release_override()? {
        return Ok(release);
    }
    let json = fetch_latest_release_json()?;
    let body: serde_json::Value = serde_json::from_str(&json)
        .map_err(|error| format!("Release response was invalid JSON: {error}"))?;
    let tag = body["tag_name"]
        .as_str()
        .ok_or_else(|| "Latest release response had no tag_name".to_string())?
        .to_string();
    let version = tag.strip_prefix('v').unwrap_or(&tag).to_string();
    if version.is_empty()
        || version.split('.').any(|part| {
            part.is_empty() || !part.chars().all(|character| character.is_ascii_digit())
        })
    {
        return Err(format!(
            "Latest release tag is not a stable semantic version: {tag}"
        ));
    }
    Ok(Release { tag, version })
}

fn ci_release_override() -> std::result::Result<Option<Release>, String> {
    let Some(tag) = std::env::var_os("SD300_CI_RELEASE_TAG") else {
        return Ok(None);
    };
    if std::env::var("GITHUB_ACTIONS").as_deref() != Ok("true") {
        return Err("SD300_CI_RELEASE_TAG is restricted to GitHub Actions".into());
    }
    let tag = tag
        .into_string()
        .map_err(|_| "The GitHub Actions candidate tag is not UTF-8".to_string())?;
    let version = tag
        .strip_prefix('v')
        .ok_or_else(|| "The GitHub Actions candidate tag must start with v".to_string())?;
    if !is_worker_release_version(version) {
        return Err("The GitHub Actions candidate tag is not a stable semantic version".into());
    }
    let version = version.to_string();
    Ok(Some(Release { tag, version }))
}

fn fetch_latest_release_json() -> std::result::Result<String, String> {
    #[cfg(windows)]
    let candidates = ["powershell.exe", "pwsh.exe"];
    #[cfg(not(windows))]
    let candidates = ["curl", "wget"];

    let mut failures = Vec::new();
    for program in candidates {
        #[cfg(windows)]
        let args = vec![
            "-NoProfile".into(),
            "-NonInteractive".into(),
            "-ExecutionPolicy".into(),
            "Bypass".into(),
            "-Command".into(),
            format!(
                "$ProgressPreference='SilentlyContinue'; Invoke-RestMethod -Headers @{{'User-Agent'='sd300/{VERSION}';'Accept'='application/vnd.github+json'}} -Uri '{RELEASES_URL}' | ConvertTo-Json -Depth 20 -Compress"
            ),
        ];
        #[cfg(not(windows))]
        let args = if program == "curl" {
            vec![
                "--proto".into(),
                "=https".into(),
                "--tlsv1.2".into(),
                "-fLsS".into(),
                "-H".into(),
                format!("User-Agent: sd300/{VERSION}"),
                RELEASES_URL.into(),
            ]
        } else {
            vec![
                "-qO-".into(),
                format!("--header=User-Agent: sd300/{VERSION}"),
                RELEASES_URL.into(),
            ]
        };

        match run_output(
            program,
            args,
            CommandTimeout::Custom(std::time::Duration::from_secs(20)),
        ) {
            Some(output) if output.status.success() => {
                return Ok(String::from_utf8_lossy(&output.stdout).to_string())
            }
            Some(output) => failures.push(format!(
                "{program} exited {}",
                output.status.code().unwrap_or(-1)
            )),
            None => failures.push(format!("{program} unavailable or timed out")),
        }
    }
    Err(format!(
        "No release-check transport succeeded ({})",
        failures.join("; ")
    ))
}

fn is_newer(current: &str, latest: &str) -> bool {
    let parse = |value: &str| {
        value
            .split('.')
            .map(|part| part.parse::<u64>().unwrap_or_default())
            .collect::<Vec<_>>()
    };
    let current = parse(current);
    let latest = parse(latest);
    for index in 0..current.len().max(latest.len()) {
        let current = current.get(index).copied().unwrap_or_default();
        let latest = latest.get(index).copied().unwrap_or_default();
        if latest != current {
            return latest > current;
        }
    }
    false
}

fn tool_exists(tool: &str) -> bool {
    Command::new(tool)
        .arg("--version")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .is_ok_and(|status| status.success())
}

/// Stock Windows ships only Windows PowerShell 5.1, which rejects the
/// cross-platform `--version` probe with a parse error, so a spawn probe
/// reports the in-box shell as missing on any machine without PowerShell 7.
/// Resolve the trusted System32 image directly instead and fall back to a
/// PATH-resolved PowerShell 7 only when the in-box shell is absent.
#[cfg(windows)]
fn windows_powershell_program() -> std::result::Result<std::ffi::OsString, String> {
    if let Ok(powershell) =
        trusted_windows_system_executable(Path::new("WindowsPowerShell\\v1.0\\powershell.exe"))
    {
        return Ok(powershell.into_os_string());
    }
    if tool_exists("pwsh.exe") {
        return Ok(std::ffi::OsString::from("pwsh.exe"));
    }
    Err("PowerShell is not available".into())
}

fn detect_installation() -> std::result::Result<Installation, String> {
    let current = std::env::current_exe()
        .map_err(|error| format!("Could not resolve the running executable: {error}"))?;

    let managed_receipt = receipt_path();
    let receipt_present = managed_receipt
        .as_deref()
        .map(|receipt| regular_file_presence(receipt, "managed receipt"))
        .transpose()?
        .unwrap_or(false);
    let managed_current = managed_receipt_is_current(
        &current,
        receipt_present,
        managed_receipt_evidence_regular()?,
        VERSION,
    )?;
    let cargo_current = cargo_install_is_current(&current, VERSION)?;
    if managed_current && cargo_current {
        let receipt = managed_receipt
            .as_deref()
            .expect("a current managed receipt has a path");
        let manifest = cargo_legacy_manifest_path()
            .ok_or_else(|| "Cargo ownership has no resolvable manifest path".to_string())?;
        return match newer_ownership_record(receipt, &manifest)? {
            OverlapOwner::Managed => Ok(Installation {
                channel: if cfg!(windows) {
                    InstallChannel::PowerShellInstaller
                } else {
                    InstallChannel::ShellInstaller
                },
                binary_path: current,
            }),
            OverlapOwner::Cargo => Ok(Installation {
                channel: InstallChannel::Cargo,
                binary_path: current,
            }),
        };
    }
    if managed_current {
        return Ok(Installation {
            channel: if cfg!(windows) {
                InstallChannel::PowerShellInstaller
            } else {
                InstallChannel::ShellInstaller
            },
            binary_path: current,
        });
    }

    #[cfg(windows)]
    if let Some(channel) = detect_windows_native_channel(&current)? {
        return Ok(Installation {
            channel,
            binary_path: current,
        });
    }

    #[cfg(target_os = "macos")]
    if path_eq(&current, Path::new("/usr/local/bin/sd300")) && mac_pkg_receipt_matches() {
        return Ok(Installation {
            channel: InstallChannel::MacPkg,
            binary_path: current,
        });
    }

    if cargo_current {
        return Ok(Installation {
            channel: InstallChannel::Cargo,
            binary_path: current,
        });
    }

    Err(format!(
        "Install origin is unknown for {}. No mutation was attempted. Run the preferred fresh installer from the website to make the latest intent authoritative.",
        current.display()
    ))
}

/// Return true only for the deliberate intermediate state created by the
/// immutable v2 Cargo updater: the running v3 binary is still authoritatively
/// Cargo-owned and the separately installed desktop companion is absent.
///
/// Ambiguous or unknown ownership fails closed to `false`; ordinary TUI
/// launches must never turn ownership uncertainty into a new warning or a
/// lifecycle mutation.
pub fn cargo_gui_completion_needed() -> bool {
    cargo_gui_completion_needed_for(detect_installation(), crate::gui::companion_path_present())
}

fn cargo_gui_completion_needed_for(
    installation: std::result::Result<Installation, String>,
    companion_present: bool,
) -> bool {
    installation.is_ok_and(|installation| {
        installation.channel == InstallChannel::Cargo && !companion_present
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum OverlapOwner {
    Managed,
    Cargo,
}

fn newer_ownership_record(
    receipt: &Path,
    manifest: &Path,
) -> std::result::Result<OverlapOwner, String> {
    let managed_time = std::fs::metadata(receipt)
        .and_then(|metadata| metadata.modified())
        .map_err(|error| format!("Could not timestamp managed ownership evidence: {error}"))?;
    let cargo_time = std::fs::metadata(manifest)
        .and_then(|metadata| metadata.modified())
        .map_err(|error| format!("Could not timestamp Cargo ownership evidence: {error}"))?;
    ownership_from_times(managed_time, cargo_time).ok_or_else(|| {
        "Managed and Cargo ownership records have equal timestamps. No mutation was attempted. Run a fresh official installer to make the intended channel authoritative."
            .to_string()
    })
}

fn ownership_from_times(
    managed_time: std::time::SystemTime,
    cargo_time: std::time::SystemTime,
) -> Option<OverlapOwner> {
    use std::cmp::Ordering;
    match managed_time.cmp(&cargo_time) {
        Ordering::Greater => Some(OverlapOwner::Managed),
        Ordering::Less => Some(OverlapOwner::Cargo),
        Ordering::Equal => None,
    }
}

fn managed_receipt_is_current(
    current: &Path,
    receipt_present: bool,
    evidence: Option<(PathBuf, String)>,
    current_version: &str,
) -> std::result::Result<bool, String> {
    if !receipt_present {
        return Ok(false);
    }
    let (binary, version) = evidence.ok_or_else(|| {
        "A managed SD-300 receipt exists but does not prove an exact cargo-dist binary. No mutation was attempted.".to_string()
    })?;
    if !path_eq(&binary, current) || version != current_version {
        return Err(format!(
            "The managed receipt proves {} version {}, but the running binary is {} version {}. No mutation was attempted.",
            binary.display(),
            version,
            current.display(),
            current_version
        ));
    }
    Ok(true)
}

fn receipt_path() -> Option<PathBuf> {
    #[cfg(windows)]
    {
        std::env::var_os("XDG_CONFIG_HOME")
            .or_else(|| std::env::var_os("LOCALAPPDATA"))
            .map(PathBuf::from)
            .map(|root| root.join(APP_NAME).join("sd300-receipt.json"))
    }
    #[cfg(not(windows))]
    {
        std::env::var_os("XDG_CONFIG_HOME")
            .map(PathBuf::from)
            .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".config")))
            .map(|root| root.join(APP_NAME).join("sd300-receipt.json"))
    }
}

fn managed_receipt_binary() -> Option<PathBuf> {
    managed_receipt_evidence().map(|(binary, _)| binary)
}

/// The managed channel's receipt-recorded CLI executable, for callers that
/// need a proven absolute product location rather than a PATH lookup.
pub(crate) fn managed_cli_binary() -> Option<PathBuf> {
    managed_receipt_binary()
}

fn managed_receipt_evidence() -> Option<(PathBuf, String)> {
    managed_receipt_evidence_regular().ok().flatten()
}

fn managed_receipt_evidence_regular() -> std::result::Result<Option<(PathBuf, String)>, String> {
    let Some(receipt_path) = receipt_path() else {
        return Ok(None);
    };
    if !regular_file_presence(&receipt_path, "managed receipt")? {
        return Ok(None);
    }
    let receipt = std::fs::read_to_string(&receipt_path).map_err(|error| {
        format!(
            "Could not read managed receipt {}: {error}. No mutation was attempted.",
            receipt_path.display()
        )
    })?;
    let Some((prefix, version)) = managed_receipt_fields(&receipt) else {
        return Ok(None);
    };
    let binary = prefix
        .join("bin")
        .join(if cfg!(windows) { "sd300.exe" } else { "sd300" });
    if !regular_file_presence(&binary, "managed binary")? {
        return Ok(None);
    }
    Ok(Some((binary, version)))
}

fn regular_file_presence(path: &Path, label: &str) -> std::result::Result<bool, String> {
    match std::fs::symlink_metadata(path) {
        Ok(metadata) if metadata.file_type().is_file() => Ok(true),
        Ok(_) => Err(format!(
            "The {label} at {} is a symlink or special file. No mutation was attempted.",
            path.display()
        )),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(error) => Err(format!(
            "Could not inspect the {label} at {}: {error}. No mutation was attempted.",
            path.display()
        )),
    }
}

fn managed_receipt_fields(receipt: &str) -> Option<(PathBuf, String)> {
    // Windows PowerShell 5.1 writers (Set-Content -Encoding utf8) prefix a
    // UTF-8 BOM that serde_json rejects; the receipt is ours to read liberally.
    let receipt = receipt.trim_start_matches('\u{feff}');
    let json: serde_json::Value = serde_json::from_str(receipt).ok()?;
    if json
        .pointer("/provider/source")
        .and_then(serde_json::Value::as_str)
        != Some("cargo-dist")
        || json
            .pointer("/source/app_name")
            .and_then(serde_json::Value::as_str)
            != Some(APP_NAME)
    {
        return None;
    }
    let prefix = PathBuf::from(json.get("install_prefix")?.as_str()?);
    let version = json.get("version")?.as_str()?.to_string();
    Some((prefix, version))
}

fn cargo_home() -> Option<PathBuf> {
    std::env::var_os("CARGO_HOME")
        .map(PathBuf::from)
        .or_else(|| {
            std::env::var_os(if cfg!(windows) { "USERPROFILE" } else { "HOME" })
                .map(|home| PathBuf::from(home).join(".cargo"))
        })
}

fn cargo_binary_path() -> Option<PathBuf> {
    cargo_home().map(|root| {
        root.join("bin")
            .join(if cfg!(windows) { "sd300.exe" } else { "sd300" })
    })
}

fn cargo_manifest_path() -> Option<PathBuf> {
    cargo_home().map(|root| root.join(".crates2.json"))
}

fn cargo_legacy_manifest_path() -> Option<PathBuf> {
    cargo_home().map(|root| root.join(".crates.toml"))
}

fn read_cargo_ownership_file(path: &Path) -> std::result::Result<Option<String>, String> {
    // A UTF-8 BOM from a Windows PowerShell 5.1 writer would otherwise fail
    // JSON/TOML parsing; ownership evidence is read liberally.
    let metadata = match std::fs::symlink_metadata(path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => {
            return Err(format!(
                "Could not inspect Cargo ownership metadata {}: {error}. No mutation was attempted.",
                path.display()
            ));
        }
    };
    if !metadata.file_type().is_file() {
        return Err(format!(
            "Cargo ownership metadata {} is a symlink or special file. No mutation was attempted.",
            path.display()
        ));
    }
    std::fs::read_to_string(path)
        .map(|contents| Some(contents.trim_start_matches('\u{feff}').to_string()))
        .map_err(|error| {
            format!(
                "Could not read Cargo ownership metadata {}: {error}. No mutation was attempted.",
                path.display()
            )
        })
}

fn cargo_install_is_current(
    current: &Path,
    current_version: &str,
) -> std::result::Result<bool, String> {
    let Some(expected_binary) = cargo_binary_path() else {
        return Ok(false);
    };
    if !path_eq(current, &expected_binary) {
        return Ok(false);
    }
    let legacy_manifest_path =
        cargo_legacy_manifest_path().expect("cargo home exists when its binary path exists");
    let legacy_manifest = match read_cargo_ownership_file(&legacy_manifest_path)? {
        Some(manifest) => manifest,
        None => return Ok(false),
    };
    let manifest_path = cargo_manifest_path().expect("Cargo home exists for both manifests");
    let v2_manifest = read_cargo_ownership_file(&manifest_path)?;
    cargo_manifests_match_current(
        &legacy_manifest,
        v2_manifest.as_deref(),
        current_version,
        &legacy_manifest_path,
        &manifest_path,
    )
}

fn cargo_manifests_match_current(
    legacy_manifest: &str,
    v2_manifest: Option<&str>,
    current_version: &str,
    legacy_path: &Path,
    v2_path: &Path,
) -> std::result::Result<bool, String> {
    let Some(legacy_package_id) =
        crate::migrate::cargo_legacy_manifest_package_id(legacy_manifest)?
    else {
        return Ok(false);
    };
    let legacy_version = crate::migrate::cargo_legacy_manifest_version(legacy_manifest)?
        .expect("a proven legacy PackageId has a version");
    if legacy_version != current_version {
        return Err(format!(
            "Cargo records tr300-tui version {} in {}, but the running binary reports {}. No mutation was attempted.",
            legacy_version,
            legacy_path.display(),
            current_version
        ));
    }
    if let Some(manifest) = v2_manifest {
        if let Some(package_id) = cargo_manifest_package_id(manifest)? {
            if package_id != legacy_package_id {
                return Err(format!(
                    "Cargo ownership metadata conflicts between {} ({}) and {} ({}). No mutation was attempted.",
                    legacy_path.display(),
                    legacy_package_id,
                    v2_path.display(),
                    package_id
                ));
            }
        }
    }
    Ok(true)
}

#[cfg(test)]
fn cargo_manifest_matches_current(
    manifest: &str,
    current_version: &str,
) -> std::result::Result<bool, String> {
    let Some(version) = cargo_manifest_version(manifest)? else {
        return Ok(false);
    };
    if version != current_version {
        return Err(format!(
            "Cargo records tr300-tui version {}, but the running binary reports {}. No mutation was attempted.",
            version, current_version
        ));
    }
    Ok(true)
}

pub(crate) fn cargo_manifest_version(
    manifest: &str,
) -> std::result::Result<Option<String>, String> {
    cargo_manifest_package_id(manifest)?
        .map(|package_id| {
            let remainder = package_id
                .strip_prefix(&format!("{CRATE_NAME} "))
                .ok_or_else(|| {
                    "Cargo's .crates2.json ownership entry has no exact package name".to_string()
                })?;
            let version = remainder
                .split_once(" (")
                .map_or(remainder, |(version, _)| version);
            if version.is_empty() {
                return Err(
                    "Cargo's .crates2.json ownership entry has no exact package version".into(),
                );
            }
            Ok(version.to_string())
        })
        .transpose()
}

pub(crate) fn cargo_manifest_package_id(
    manifest: &str,
) -> std::result::Result<Option<String>, String> {
    let json: serde_json::Value = serde_json::from_str(manifest).map_err(|error| {
        format!("Cargo's .crates2.json is invalid: {error}. No mutation was attempted.")
    })?;
    let installs = json
        .get("installs")
        .and_then(serde_json::Value::as_object)
        .ok_or_else(|| {
            "Cargo's .crates2.json has no installs object. No mutation was attempted.".to_string()
        })?;
    let binary_name = if cfg!(windows) { "sd300.exe" } else { "sd300" };
    let prefix = format!("{CRATE_NAME} ");
    let mut matching = Vec::new();
    let mut foreign_owners = Vec::new();
    for (key, value) in installs {
        let target_package = key.starts_with(&prefix);
        let Some(bins_value) = value.get("bins") else {
            if target_package {
                return Err(format!(
                    "Cargo's .crates2.json entry {key} has no bins array. No mutation was attempted."
                ));
            }
            continue;
        };
        let Some(bins) = bins_value.as_array() else {
            if target_package {
                return Err(format!(
                    "Cargo's .crates2.json entry {key} has a malformed bins value. No mutation was attempted."
                ));
            }
            continue;
        };
        let Some(bin_names) = bins
            .iter()
            .map(serde_json::Value::as_str)
            .collect::<Option<Vec<_>>>()
        else {
            if target_package {
                return Err(format!(
                    "Cargo's .crates2.json entry {key} contains a non-string binary name. No mutation was attempted."
                ));
            }
            continue;
        };
        let owns_binary = bin_names.contains(&binary_name);
        if !owns_binary {
            continue;
        }
        if !target_package {
            foreign_owners.push(key.as_str());
            continue;
        }
        if bin_names.as_slice() != [binary_name] {
            return Err(format!(
                "Cargo's .crates2.json entry {key} owns additional binaries and cannot be treated as one SD-300 target. No mutation was attempted."
            ));
        }
        matching.push(key.clone());
    }
    if !foreign_owners.is_empty() {
        return Err(format!(
            "Cargo's .crates2.json records foreign ownership of {binary_name}: {}. No mutation was attempted.",
            foreign_owners.join(", ")
        ));
    }
    if matching.len() > 1 {
        return Err(
            "Cargo records multiple tr300-tui installations owning sd300. No mutation was attempted."
                .into(),
        );
    }
    Ok(matching.pop())
}

fn path_eq(left: &Path, right: &Path) -> bool {
    #[cfg(windows)]
    {
        left.to_string_lossy()
            .trim_end_matches(['\\', '/'])
            .eq_ignore_ascii_case(right.to_string_lossy().trim_end_matches(['\\', '/']))
    }
    #[cfg(not(windows))]
    {
        left == right
    }
}

#[cfg(windows)]
#[derive(Debug, Clone)]
struct NativeRegistration {
    channel: InstallChannel,
    key_name: String,
    uninstall_string: Option<String>,
    quiet_uninstall_string: Option<String>,
}

#[cfg(windows)]
fn detect_windows_native_channel(
    current: &Path,
) -> std::result::Result<Option<InstallChannel>, String> {
    let registrations = native_registrations();
    let candidates = registrations
        .iter()
        .filter(|registration| {
            expected_windows_binary(registration.channel)
                .is_some_and(|expected| path_eq(current, &expected))
        })
        .map(|registration| registration.channel)
        .collect::<Vec<_>>();
    let marker = read_install_source_marker();
    if let Some(marker) = marker {
        if candidates.contains(&marker) {
            return Ok(Some(marker));
        }
        return Err(format!(
            "Windows install marker names {}, but no matching native registration owns the running path",
            marker.label()
        ));
    }
    let mut unique = candidates;
    unique.sort_by_key(|channel| *channel as u8);
    unique.dedup();
    match unique.as_slice() {
        [] => Ok(None),
        [channel] => Ok(Some(*channel)),
        _ => Err(format!(
            "Multiple native installer channels claim the running SD-300 path: {}. No mutation was attempted.",
            unique
                .iter()
                .map(|channel| channel.label())
                .collect::<Vec<_>>()
                .join(", ")
        )),
    }
}

#[cfg(windows)]
fn expected_windows_binary(channel: InstallChannel) -> Option<PathBuf> {
    let root = match channel {
        InstallChannel::MsiGlobal | InstallChannel::ExeGlobal => {
            PathBuf::from(std::env::var_os("ProgramFiles")?)
        }
        InstallChannel::MsiCorporate | InstallChannel::ExeCorporate => {
            PathBuf::from(std::env::var_os("LOCALAPPDATA")?).join("Programs")
        }
        _ => return None,
    };
    Some(root.join("sd300").join("bin").join("sd300.exe"))
}

#[cfg(windows)]
fn read_install_source_marker() -> Option<InstallChannel> {
    use winreg::enums::HKEY_CURRENT_USER;
    use winreg::RegKey;

    let key = RegKey::predef(HKEY_CURRENT_USER)
        .open_subkey("Software\\SD300")
        .ok()?;
    let value: String = key.get_value("InstallSource").ok()?;
    match value.as_str() {
        "msi-global" => Some(InstallChannel::MsiGlobal),
        "msi-corporate" => Some(InstallChannel::MsiCorporate),
        "exe-global" => Some(InstallChannel::ExeGlobal),
        "exe-corporate" => Some(InstallChannel::ExeCorporate),
        _ => None,
    }
}

#[cfg(windows)]
fn native_registrations() -> Vec<NativeRegistration> {
    use winreg::enums::{
        HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE, KEY_READ, KEY_WOW64_32KEY, KEY_WOW64_64KEY,
    };
    use winreg::RegKey;

    let roots = [
        (RegKey::predef(HKEY_CURRENT_USER), KEY_READ),
        (
            RegKey::predef(HKEY_LOCAL_MACHINE),
            KEY_READ | KEY_WOW64_64KEY,
        ),
        (
            RegKey::predef(HKEY_LOCAL_MACHINE),
            KEY_READ | KEY_WOW64_32KEY,
        ),
    ];
    let mut registrations = Vec::new();
    for (root, flags) in roots {
        let Ok(uninstall) = root.open_subkey_with_flags(
            "SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\Uninstall",
            flags,
        ) else {
            continue;
        };
        for key_name in uninstall.enum_keys().filter_map(std::result::Result::ok) {
            let Ok(entry) = uninstall.open_subkey_with_flags(&key_name, KEY_READ) else {
                continue;
            };
            let display_name: String = entry.get_value("DisplayName").unwrap_or_default();
            let normalized = display_name.to_ascii_lowercase().replace(['-', ' '], "");
            if !normalized.contains("sd300") {
                continue;
            }
            let install_location: String = entry.get_value("InstallLocation").unwrap_or_default();
            let windows_installer: u32 = entry.get_value("WindowsInstaller").unwrap_or_default();
            let channel = classify_windows_registration(
                &key_name,
                &display_name,
                &install_location,
                windows_installer == 1,
            );
            let Some(channel) = channel else {
                continue;
            };
            registrations.push(NativeRegistration {
                channel,
                key_name,
                uninstall_string: entry.get_value("UninstallString").ok(),
                quiet_uninstall_string: entry.get_value("QuietUninstallString").ok(),
            });
        }
    }
    registrations.sort_by(|left, right| left.key_name.cmp(&right.key_name));
    registrations.dedup_by(|left, right| {
        left.channel == right.channel && left.key_name.eq_ignore_ascii_case(&right.key_name)
    });
    registrations
}

#[cfg(windows)]
fn classify_windows_registration(
    key_name: &str,
    display_name: &str,
    install_location: &str,
    windows_installer: bool,
) -> Option<InstallChannel> {
    const INNO_GLOBAL: &str = "DC74D35F-CBF4-425F-B11E-E9EA87C13CA9";
    const INNO_CORPORATE: &str = "ED209931-B5C0-43AE-89F6-83EE2C581653";
    if key_name.to_ascii_uppercase().contains(INNO_GLOBAL) {
        return Some(InstallChannel::ExeGlobal);
    }
    if key_name.to_ascii_uppercase().contains(INNO_CORPORATE) {
        return Some(InstallChannel::ExeCorporate);
    }
    if !windows_installer {
        return None;
    }
    let evidence = format!("{display_name} {install_location}").to_ascii_lowercase();
    if evidence.contains("corporate")
        || std::env::var_os("LOCALAPPDATA").is_some_and(|root| {
            install_location.to_ascii_lowercase().starts_with(
                &PathBuf::from(root)
                    .join("Programs")
                    .to_string_lossy()
                    .to_ascii_lowercase(),
            )
        })
    {
        Some(InstallChannel::MsiCorporate)
    } else {
        Some(InstallChannel::MsiGlobal)
    }
}

#[cfg(target_os = "macos")]
fn mac_pkg_receipt_matches() -> bool {
    let package_info = run_output(
        "pkgutil",
        ["--pkg-info", MAC_PKG_ID],
        CommandTimeout::Normal,
    );
    let file_info = run_output(
        "pkgutil",
        ["--file-info", "/usr/local/bin/sd300"],
        CommandTimeout::Normal,
    );
    package_info.is_some_and(|output| output.status.success())
        && file_info.is_some_and(|output| {
            output.status.success() && String::from_utf8_lossy(&output.stdout).contains(MAC_PKG_ID)
        })
}

fn execute_uninstall(
    installation: &Installation,
    quiet_stdout: bool,
) -> std::result::Result<String, String> {
    match installation.channel {
        InstallChannel::Cargo => uninstall_cargo(installation, quiet_stdout),
        InstallChannel::PowerShellInstaller | InstallChannel::ShellInstaller => {
            uninstall_managed(installation)
        }
        InstallChannel::MsiGlobal
        | InstallChannel::MsiCorporate
        | InstallChannel::ExeGlobal
        | InstallChannel::ExeCorporate => uninstall_windows_native(installation, quiet_stdout),
        InstallChannel::MacPkg => uninstall_macos_pkg(quiet_stdout),
    }
}

#[cfg(windows)]
fn uninstall_cargo(
    installation: &Installation,
    quiet_stdout: bool,
) -> std::result::Result<String, String> {
    uninstall_windows_files(installation, true, quiet_stdout)?;
    Ok(
        "Cargo-owned SD-300 was removed; final running-image cleanup was scheduled and the Rust toolchain was preserved"
            .into(),
    )
}

#[cfg(not(windows))]
fn uninstall_cargo(
    _installation: &Installation,
    quiet_stdout: bool,
) -> std::result::Result<String, String> {
    run_status(
        Command::new("cargo").args(["uninstall", CRATE_NAME]),
        quiet_stdout,
    )?;
    remove_empty_managed_receipt_directory()?;
    Ok("Cargo-owned SD-300 was removed; the Rust toolchain was preserved".into())
}

#[cfg(windows)]
fn uninstall_managed(installation: &Installation) -> std::result::Result<String, String> {
    uninstall_windows_files(installation, false, true)?;
    Ok(
        "Managed SD-300 CLI, GUI, integrations, and receipt were removed; final running-image cleanup was scheduled"
            .into(),
    )
}

#[cfg(not(windows))]
fn uninstall_managed(installation: &Installation) -> std::result::Result<String, String> {
    let gui = prove_unix_managed_gui_install()?;
    if let Some(gui) = &gui {
        let marker_removed_with_root = gui.owner_marker.starts_with(&gui.root);
        std::fs::remove_dir_all(&gui.root)
            .map_err(|error| format!("Could not remove the managed GUI payload: {error}"))?;
        // Linux deliberately keeps its ownership marker inside the managed
        // payload root. `remove_dir_all` already removed it; attempting a
        // second removal turns every successful Linux uninstall into a false
        // failure and strands the remaining CLI/integrations. macOS keeps the
        // marker outside the signed .app bundle and still removes it here.
        if !marker_removed_with_root {
            std::fs::remove_file(&gui.owner_marker).map_err(|error| {
                format!("Could not remove the managed GUI ownership marker: {error}")
            })?;
        }
        if let Some(desktop) = &gui.desktop_entry {
            std::fs::remove_file(desktop)
                .map_err(|error| format!("Could not remove the managed desktop entry: {error}"))?;
        }
    }
    std::fs::remove_file(&installation.binary_path)
        .map_err(|error| format!("Could not remove managed binary: {error}"))?;
    remove_managed_receipt()?;
    Ok(if gui.is_some() {
        "Managed SD-300 CLI, GUI, integrations, and receipt were removed".into()
    } else {
        "Managed SD-300 binary and receipt were removed".into()
    })
}

#[cfg(test)]
mod managed_gui_path_tests {
    use super::*;

    #[test]
    fn linux_owner_marker_is_removed_with_its_payload_root() {
        let root = PathBuf::from("/home/test/.local/share/sd300");
        let marker = root.join(".sd300-managed-owner.json");
        assert!(marker.starts_with(&root));
    }

    #[test]
    fn macos_owner_marker_remains_outside_the_signed_app_bundle() {
        let root = PathBuf::from("/Users/test/Applications/SD-300.app");
        let marker = PathBuf::from(
            "/Users/test/Library/Application Support/SD-300/managed-install-owner.json",
        );
        assert!(!marker.starts_with(&root));
    }
}

#[cfg(not(windows))]
#[derive(serde::Deserialize)]
struct UnixManagedGuiOwner {
    schema: u32,
    product: String,
    owner: String,
}

#[cfg(not(windows))]
struct UnixManagedGuiInstall {
    root: PathBuf,
    owner_marker: PathBuf,
    desktop_entry: Option<PathBuf>,
}

#[cfg(not(windows))]
fn prove_unix_managed_gui_install() -> std::result::Result<Option<UnixManagedGuiInstall>, String> {
    let home = std::env::var_os("HOME")
        .map(PathBuf::from)
        .ok_or_else(|| "HOME is unavailable for managed GUI cleanup".to_string())?;
    #[cfg(target_os = "macos")]
    let (root, owner_marker, desktop_entry) = (
        home.join("Applications").join("SD-300.app"),
        home.join("Library")
            .join("Application Support")
            .join("SD-300")
            .join("managed-install-owner.json"),
        None::<PathBuf>,
    );
    #[cfg(target_os = "linux")]
    let (root, owner_marker, desktop_entry) = {
        let data = std::env::var_os("XDG_DATA_HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|| home.join(".local").join("share"));
        (
            data.join("sd300"),
            data.join("sd300").join(".sd300-managed-owner.json"),
            Some(data.join("applications").join("sd300.desktop")),
        )
    };
    #[cfg(not(any(target_os = "macos", target_os = "linux")))]
    return Ok(None);

    if !root.exists() {
        if owner_marker.exists() {
            return Err(format!(
                "The managed GUI ownership marker exists without its payload at {}; it was preserved",
                owner_marker.display()
            ));
        }
        return Ok(None);
    }
    let metadata = std::fs::symlink_metadata(&root)
        .map_err(|error| format!("Could not inspect the managed GUI root: {error}"))?;
    if !metadata.is_dir() || metadata.file_type().is_symlink() {
        return Err(format!(
            "The managed GUI root is not an owned directory: {}",
            root.display()
        ));
    }
    let parent = root
        .parent()
        .ok_or_else(|| "The managed GUI root has no parent".to_string())?;
    let canonical_root = std::fs::canonicalize(&root)
        .map_err(|error| format!("Could not resolve the managed GUI root: {error}"))?;
    let canonical_parent = std::fs::canonicalize(parent)
        .map_err(|error| format!("Could not resolve the managed GUI parent: {error}"))?;
    if canonical_root.parent() != Some(canonical_parent.as_path()) {
        return Err(format!(
            "The managed GUI root escaped its exact owned parent: {}",
            canonical_root.display()
        ));
    }
    let owner: UnixManagedGuiOwner =
        serde_json::from_slice(&std::fs::read(&owner_marker).map_err(|error| {
            format!(
                "The managed GUI root has no readable ownership marker at {}: {error}",
                owner_marker.display()
            )
        })?)
        .map_err(|error| format!("The managed GUI ownership marker is invalid: {error}"))?;
    if owner.schema != 1 || owner.product != "SD-300" || owner.owner != "shell-installer" {
        return Err("The managed GUI ownership marker is ambiguous; it was preserved".into());
    }
    if let Some(desktop) = &desktop_entry {
        match std::fs::read_to_string(desktop) {
            Ok(contents) if contents.contains("# SD-300 managed desktop entry") => {}
            Ok(_) => {
                return Err(format!(
                    "The Linux desktop entry at {} is not SD-300-owned; it was preserved",
                    desktop.display()
                ))
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => {
                return Err(format!(
                    "Could not inspect the Linux desktop entry at {}: {error}",
                    desktop.display()
                ))
            }
        }
    }
    let desktop_entry = desktop_entry.filter(|desktop| desktop.exists());
    Ok(Some(UnixManagedGuiInstall {
        root,
        owner_marker,
        desktop_entry,
    }))
}

#[cfg(not(windows))]
fn remove_managed_receipt() -> std::result::Result<(), String> {
    let Some(receipt) = receipt_path() else {
        return Ok(());
    };
    match std::fs::remove_file(&receipt) {
        Ok(()) => {}
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => return Err(format!("Could not remove managed receipt: {error}")),
    }
    remove_empty_managed_receipt_directory()
}

fn remove_empty_managed_receipt_directory() -> std::result::Result<(), String> {
    let Some(parent) = receipt_path().and_then(|receipt| receipt.parent().map(Path::to_path_buf))
    else {
        return Ok(());
    };
    match std::fs::remove_dir(parent) {
        Ok(()) => Ok(()),
        Err(error)
            if matches!(
                error.kind(),
                std::io::ErrorKind::NotFound | std::io::ErrorKind::DirectoryNotEmpty
            ) =>
        {
            Ok(())
        }
        Err(error) => Err(format!(
            "Could not remove the empty managed receipt directory: {error}"
        )),
    }
}

#[cfg(windows)]
fn uninstall_windows_files(
    installation: &Installation,
    cargo_owned: bool,
    quiet_stdout: bool,
) -> std::result::Result<(), String> {
    let receipt = (!cargo_owned).then(receipt_path).flatten();
    let managed_gui = if cargo_owned {
        None
    } else {
        prove_windows_managed_gui_root()?
    };
    let shortcut = (!cargo_owned).then(windows_managed_gui_shortcut).flatten();
    let commands = windows_managed_cleanup_commands(
        None,
        &installation.binary_path,
        managed_gui.as_deref(),
        shortcut.as_deref(),
        receipt.as_deref(),
        cargo_owned,
    );
    let handoff = WindowsUninstallImageHandoff::begin()?;
    let powershell =
        trusted_windows_system_executable(Path::new("WindowsPowerShell\\v1.0\\powershell.exe"))?;
    let cleanup = run_status(
        // See execute_managed_wrapper: inherited pwsh module paths break the
        // in-box shell's auto-loading; children rebuild defaults when unset.
        Command::new(powershell).env_remove("PSModulePath").args([
            "-NoLogo",
            "-NoProfile",
            "-NonInteractive",
            "-ExecutionPolicy",
            "Bypass",
            "-WindowStyle",
            "Hidden",
            "-Command",
            &commands,
        ]),
        quiet_stdout,
    )
    .and_then(|()| {
        verify_windows_file_uninstalled(
            installation,
            managed_gui.as_deref(),
            shortcut.as_deref(),
            receipt.as_deref(),
            cargo_owned,
        )
    });
    handoff.finish(cleanup)
}

#[cfg(any(windows, test))]
fn windows_managed_cleanup_commands(
    process_id: Option<u32>,
    binary: &Path,
    managed_gui: Option<&Path>,
    shortcut: Option<&Path>,
    receipt: Option<&Path>,
    cargo_owned: bool,
) -> String {
    let mut commands = process_id.map_or_else(String::new, |process_id| {
        format!("Wait-Process -Id {process_id} -ErrorAction SilentlyContinue; ")
    });
    if cargo_owned {
        commands.push_str("& cargo uninstall tr300-tui *> $null; ");
    }
    commands.push_str(&format!(
        "Remove-Item -LiteralPath '{}' -Force -ErrorAction SilentlyContinue; ",
        powershell_escape(&binary.to_string_lossy())
    ));
    if !cargo_owned {
        // These fixed per-user integrations remain SD-300-owned even when an
        // interrupted earlier removal already deleted the GUI payload root.
        // Always scheduling them makes a retry complete rather than report
        // success while leaving Start/Search or Installed Apps residue.
        if let Some(shortcut) = shortcut {
            commands.push_str(&format!(
                "Remove-Item -LiteralPath '{}' -Force -ErrorAction SilentlyContinue; ",
                powershell_escape(&shortcut.to_string_lossy())
            ));
        }
        commands.push_str(
            "Remove-Item -LiteralPath 'Registry::HKEY_CURRENT_USER\\Software\\Microsoft\\Windows\\CurrentVersion\\Uninstall\\SD-300-Managed' -Recurse -Force -ErrorAction SilentlyContinue; ",
        );
        if let Some(bin) = binary.parent() {
            commands.push_str(&format!(
                "$sd300Bin='{}'; $sd300Environment=[Microsoft.Win32.Registry]::CurrentUser.OpenSubKey('Environment',$true); if($null -ne $sd300Environment){{ try{{ $sd300Path=$sd300Environment.GetValue('Path',$null,[Microsoft.Win32.RegistryValueOptions]::DoNotExpandEnvironmentNames); if($null -ne $sd300Path){{ $sd300Kind=$sd300Environment.GetValueKind('Path'); $sd300Parts=@([string]$sd300Path -split ';' | Where-Object {{ $_.Trim().TrimEnd('\\','/') -ine $sd300Bin.TrimEnd('\\','/') }}); $sd300Environment.SetValue('Path',($sd300Parts -join ';'),$sd300Kind) }} }} finally {{ $sd300Environment.Dispose() }} }}; ",
                powershell_escape(&bin.to_string_lossy())
            ));
        }
    }
    if let Some(root) = managed_gui {
        commands.push_str(&format!(
            "$sd300Root='{}'; for($i=0;$i -lt 50 -and (Test-Path -LiteralPath $sd300Root);$i++){{ Remove-Item -LiteralPath $sd300Root -Recurse -Force -ErrorAction SilentlyContinue; if(Test-Path -LiteralPath $sd300Root){{ Start-Sleep -Milliseconds 100 }} }}; ",
            powershell_escape(&root.to_string_lossy()),
        ));
    }
    if let Some(receipt) = receipt {
        commands.push_str(&format!(
            "Remove-Item -LiteralPath '{}' -Force -ErrorAction SilentlyContinue; ",
            powershell_escape(&receipt.to_string_lossy())
        ));
        if let Some(parent) = receipt.parent() {
            // The receipt is owned, but its parent may contain unrelated state. Remove the
            // directory only when empty; Win32 ERROR_DIR_NOT_EMPTY (145) is the expected
            // preservation outcome and every other I/O failure remains fatal.
            commands.push_str(&format!(
                "try{{[IO.Directory]::Delete('{}',$false)}}catch [IO.DirectoryNotFoundException]{{}}catch [IO.IOException]{{if(($_.Exception.HResult -band 0xFFFF) -ne 145){{throw}}}}; ",
                powershell_escape(&parent.to_string_lossy())
            ));
        }
    }
    // Windows PowerShell 5.1 mirrors the final statement's $? in the -Command
    // exit code, so a tolerated outcome (caught nonempty-parent exception or a
    // suppressed removal) in the last position would report a false failure.
    // Uncaught throws still abort before reaching this terminal marker.
    commands.push_str("exit 0");
    commands
}

#[cfg(windows)]
fn verify_windows_file_uninstalled(
    installation: &Installation,
    managed_gui: Option<&Path>,
    shortcut: Option<&Path>,
    receipt: Option<&Path>,
    cargo_owned: bool,
) -> std::result::Result<(), String> {
    let mut residual = Vec::new();
    for path in [
        Some(installation.binary_path.as_path()),
        managed_gui,
        shortcut,
        receipt,
    ]
    .into_iter()
    .flatten()
    {
        if std::fs::symlink_metadata(path).is_ok() {
            residual.push(path.display().to_string());
        }
    }

    if cargo_owned {
        if let Some(manifest) = cargo_manifest_path() {
            match read_cargo_ownership_file(&manifest)? {
                Some(contents) if cargo_manifest_version(&contents)?.is_some() => {
                    residual.push(format!("Cargo ownership in {}", manifest.display()));
                }
                Some(_) | None => {}
            }
        }
        if let Some(manifest) = cargo_legacy_manifest_path() {
            match read_cargo_ownership_file(&manifest)? {
                Some(contents)
                    if crate::migrate::cargo_legacy_manifest_version(&contents)?.is_some() =>
                {
                    residual.push(format!("Cargo ownership in {}", manifest.display()));
                }
                Some(_) | None => {}
            }
        }
    } else {
        if windows_managed_registration_exists()? {
            residual.push("the managed Installed Apps registration".into());
        }
        if let Some(bin) = installation.binary_path.parent() {
            if windows_path_registry_contains(InstallChannel::PowerShellInstaller, bin)? {
                residual.push("the managed per-user PATH entry".into());
            }
        }
    }

    if residual.is_empty() {
        Ok(())
    } else {
        Err(format!(
            "Managed uninstall returned before all owned state was removed: {}",
            residual.join(", ")
        ))
    }
}

#[cfg(windows)]
fn windows_managed_registration_exists() -> std::result::Result<bool, String> {
    use winreg::enums::{HKEY_CURRENT_USER, KEY_READ};
    use winreg::RegKey;

    match RegKey::predef(HKEY_CURRENT_USER).open_subkey_with_flags(
        r"Software\Microsoft\Windows\CurrentVersion\Uninstall\SD-300-Managed",
        KEY_READ,
    ) {
        Ok(_) => Ok(true),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(error) => Err(format!(
            "Could not verify the managed Installed Apps registration: {error}"
        )),
    }
}

#[cfg(any(windows, test))]
fn windows_path_value_contains(value: &str, expected: &Path) -> bool {
    let expected = expected
        .to_string_lossy()
        .trim()
        .trim_matches('"')
        .trim_end_matches(['\\', '/'])
        .replace('/', "\\");
    value.split(';').any(|entry| {
        entry
            .trim()
            .trim_matches('"')
            .trim_end_matches(['\\', '/'])
            .replace('/', "\\")
            .eq_ignore_ascii_case(&expected)
    })
}

#[cfg(windows)]
fn windows_path_registry_contains(
    channel: InstallChannel,
    expected: &Path,
) -> std::result::Result<bool, String> {
    use winreg::enums::{HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE, KEY_READ};
    use winreg::RegKey;

    let (root, key) = match channel {
        InstallChannel::MsiGlobal | InstallChannel::ExeGlobal => (
            RegKey::predef(HKEY_LOCAL_MACHINE),
            r"SYSTEM\CurrentControlSet\Control\Session Manager\Environment",
        ),
        InstallChannel::MsiCorporate
        | InstallChannel::ExeCorporate
        | InstallChannel::PowerShellInstaller => {
            (RegKey::predef(HKEY_CURRENT_USER), r"Environment")
        }
        _ => return Ok(false),
    };
    let environment = match root.open_subkey_with_flags(key, KEY_READ) {
        Ok(environment) => environment,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(false),
        Err(error) => return Err(format!("Could not inspect the Windows PATH owner: {error}")),
    };
    let value = match environment.get_value::<String, _>("Path") {
        Ok(value) => value,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(false),
        Err(error) => return Err(format!("Could not inspect the Windows PATH value: {error}")),
    };
    Ok(windows_path_value_contains(&value, expected))
}

#[cfg(windows)]
#[derive(serde::Deserialize)]
struct ManagedGuiOwner {
    schema: u32,
    product: String,
    owner: String,
}

#[cfg(windows)]
fn prove_windows_managed_gui_root() -> std::result::Result<Option<PathBuf>, String> {
    let Some(local_app_data) = std::env::var_os("LOCALAPPDATA").map(PathBuf::from) else {
        return Err("LOCALAPPDATA is unavailable for managed GUI cleanup".into());
    };
    let programs = local_app_data.join("Programs");
    let root = programs.join("SD-300");
    if !root.exists() {
        return Ok(None);
    }
    let root = std::fs::canonicalize(&root)
        .map_err(|error| format!("Could not resolve the managed GUI root: {error}"))?;
    let programs = std::fs::canonicalize(&programs)
        .map_err(|error| format!("Could not resolve the per-user Programs root: {error}"))?;
    if !root.starts_with(&programs) || root.parent() != Some(programs.as_path()) {
        return Err(format!(
            "Refused a managed GUI root outside the exact per-user Programs directory: {}",
            root.display()
        ));
    }
    let owner_path = root.join(".sd300-managed-owner.json");
    let bytes = std::fs::read(&owner_path).map_err(|error| {
        format!(
            "The managed GUI root exists without a readable ownership marker at {}: {error}",
            owner_path.display()
        )
    })?;
    let owner: ManagedGuiOwner = serde_json::from_slice(&bytes)
        .map_err(|error| format!("The managed GUI ownership marker is invalid: {error}"))?;
    if owner.schema != 1 || owner.product != "SD-300" || owner.owner != "powershell-installer" {
        return Err("The managed GUI root ownership marker is ambiguous; it was preserved".into());
    }
    Ok(Some(root))
}

#[cfg(windows)]
fn windows_managed_gui_shortcut() -> Option<PathBuf> {
    std::env::var_os("APPDATA").map(PathBuf::from).map(|root| {
        root.join("Microsoft")
            .join("Windows")
            .join("Start Menu")
            .join("Programs")
            .join("SD-300.lnk")
    })
}

#[cfg(windows)]
fn windows_gui_candidates(installation: &Installation) -> Vec<PathBuf> {
    let mut candidates = Vec::new();
    if let Some(root) = installation.binary_path.parent().and_then(Path::parent) {
        candidates.push(root.join("app").join("sd300-gui.exe"));
    }
    if let Some(local_app_data) = std::env::var_os("LOCALAPPDATA").map(PathBuf::from) {
        let managed = local_app_data
            .join("Programs")
            .join("SD-300")
            .join("app")
            .join("sd300-gui.exe");
        if !candidates
            .iter()
            .any(|candidate| path_eq(candidate, &managed))
        {
            candidates.push(managed);
        }
    }
    candidates
}

#[cfg(any(windows, test))]
fn windows_startup_command_target(command: &str) -> Option<PathBuf> {
    let remainder = command.strip_prefix('"')?;
    let (path, arguments) = remainder.split_once('"')?;
    matches!(arguments, " --startup" | " --startup --hidden").then(|| PathBuf::from(path))
}

#[cfg(windows)]
fn remove_owned_windows_startup(installation: &Installation) -> std::result::Result<(), String> {
    use winreg::enums::{HKEY_CURRENT_USER, KEY_READ, KEY_WRITE};
    use winreg::RegKey;

    const RUN_KEY: &str = r"Software\Microsoft\Windows\CurrentVersion\Run";
    const VALUE_NAME: &str = "SD-300";
    let run = match RegKey::predef(HKEY_CURRENT_USER)
        .open_subkey_with_flags(RUN_KEY, KEY_READ | KEY_WRITE)
    {
        Ok(run) => run,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(error) => return Err(format!("Could not inspect launch-at-login: {error}")),
    };
    let existing = match run.get_value::<String, _>(VALUE_NAME) {
        Ok(existing) => existing,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(error) => return Err(format!("Could not inspect launch-at-login: {error}")),
    };
    let Some(target) = windows_startup_command_target(&existing) else {
        return Err(
            "The SD-300 startup value is ambiguous; it was preserved and uninstall is incomplete"
                .into(),
        );
    };
    if !windows_gui_candidates(installation)
        .iter()
        .any(|candidate| path_eq(candidate, &target))
    {
        return Err(
            "The SD-300 startup value does not identify the proven GUI; it was preserved and uninstall is incomplete"
                .into(),
        );
    }
    run.delete_value(VALUE_NAME)
        .map_err(|error| format!("Could not remove SD-300 launch-at-login: {error}"))
}

#[cfg(windows)]
fn verify_windows_uninstall_completion(
    installation: &Installation,
) -> std::result::Result<(), String> {
    use winreg::enums::{HKEY_CURRENT_USER, KEY_READ};
    use winreg::RegKey;

    crate::gui::request_exit()
        .map_err(|message| format!("An SD-300 GUI process remained after uninstall: {message}"))?;
    match RegKey::predef(HKEY_CURRENT_USER)
        .open_subkey_with_flags(r"Software\Microsoft\Windows\CurrentVersion\Run", KEY_READ)
    {
        Ok(run) => match run.get_value::<String, _>("SD-300") {
            Ok(_) => return Err("The SD-300 launch-at-login value remained after uninstall".into()),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => {
                return Err(format!(
                    "Could not verify SD-300 launch-at-login removal: {error}"
                ))
            }
        },
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => {
            return Err(format!(
                "Could not inspect launch-at-login after uninstall: {error}"
            ))
        }
    }
    if matches!(
        installation.channel,
        InstallChannel::MsiGlobal
            | InstallChannel::MsiCorporate
            | InstallChannel::ExeGlobal
            | InstallChannel::ExeCorporate
    ) {
        verify_windows_native_uninstalled_converged(installation)?;
    }
    Ok(())
}

#[cfg(windows)]
fn uninstall_windows_native(
    installation: &Installation,
    quiet_stdout: bool,
) -> std::result::Result<String, String> {
    remove_empty_managed_receipt_directory()?;
    if installation.channel.global_worker_id().is_some() {
        let handoff = WindowsUninstallImageHandoff::plan()?;
        let channel = installation
            .channel
            .global_worker_id()
            .ok_or_else(|| "Refused a non-Global elevated uninstall request".to_string())?;
        let exit_code = launch_elevated_windows_uninstall_worker(channel, &handoff.backup)?;
        if let Some(completion) = windows_committed_worker_failure_completion(exit_code) {
            return Err(format!(
                "Windows Installer committed the Global MSI uninstall, but removal verification or final cleanup is pending; the installed executable was not restored.{}",
                windows_pending_verification_reboot_message(completion)
            ));
        }
        let msi_completion = (installation.channel == InstallChannel::MsiGlobal)
            .then(|| {
                i32::try_from(exit_code)
                    .ok()
                    .and_then(windows_installer_completion)
            })
            .flatten();
        if exit_code != 0
            && !matches!(
                msi_completion,
                Some(WindowsInstallerCompletion::RebootRequired(_))
            )
        {
            // The elevated worker's stdio is lost across the UAC boundary;
            // relay its failure detail from the shared report file.
            let report_path = worker_error_report_path(&handoff.backup);
            let detail = std::fs::read_to_string(&report_path)
                .map(|contents| format!("; worker: {}", contents.trim().replace('\n', " | ")))
                .unwrap_or_default();
            let _ = std::fs::remove_file(&report_path);
            return Err(format!(
                "The elevated {} uninstaller exited with code {exit_code}; verify the installed command before retrying{detail}",
                installation.channel.label()
            ));
        }
        let _ = std::fs::remove_file(worker_error_report_path(&handoff.backup));
        return Ok(format!(
            "{} uninstall completed; final running-image cleanup was scheduled{}",
            installation.channel.label(),
            if matches!(
                msi_completion,
                Some(WindowsInstallerCompletion::RebootRequired(_))
            ) {
                "; Windows Installer requires a reboot"
            } else {
                ""
            }
        ));
    }

    let handoff = WindowsUninstallImageHandoff::begin()?;
    let execution = match execute_windows_native_uninstaller(installation, quiet_stdout) {
        Ok(execution) => execution,
        Err(message) => {
            return handoff.finish(Err(message)).map(|()| {
                "Windows native uninstall failed before it returned an execution result".into()
            })
        }
    };
    let verification = verify_windows_native_uninstalled_converged(installation);
    handoff.finish_execution(execution, verification)?;
    Ok(format!(
        "{} uninstall completed; final running-image cleanup was scheduled{}",
        installation.channel.label(),
        if execution.reboot_required() {
            "; Windows Installer requires a reboot"
        } else {
            ""
        }
    ))
}

#[cfg(windows)]
fn execute_windows_native_uninstaller(
    installation: &Installation,
    quiet_stdout: bool,
) -> std::result::Result<WindowsNativeUninstallExecution, String> {
    let registrations = native_registrations();
    let registration = registrations
        .iter()
        .find(|registration| registration.channel == installation.channel)
        .ok_or_else(|| "The proven native registration disappeared before uninstall".to_string())?;
    match installation.channel {
        InstallChannel::MsiGlobal | InstallChannel::MsiCorporate => {
            let msiexec = trusted_windows_system_executable(Path::new("msiexec.exe"))?;
            let completion = run_windows_installer_status(
                Command::new(msiexec).args([
                    "/x",
                    &registration.key_name,
                    "/passive",
                    "/norestart",
                    "SD300GUIALREADYSTOPPED=1",
                    "SD300PRESERVEGUISTATE=1",
                ]),
                quiet_stdout,
            )?;
            Ok(WindowsNativeUninstallExecution::MsiCommitted(completion))
        }
        InstallChannel::ExeGlobal | InstallChannel::ExeCorporate => {
            let command_line = registration
                .quiet_uninstall_string
                .as_deref()
                .or(registration.uninstall_string.as_deref())
                .ok_or_else(|| "The EXE registration had no uninstall command".to_string())?;
            let executable = parse_quoted_executable(command_line)
                .ok_or_else(|| "The EXE uninstall command was ambiguous".to_string())?;
            let expected_root = installation
                .binary_path
                .parent()
                .and_then(Path::parent)
                .ok_or_else(|| "The installed EXE root was invalid".to_string())?;
            if !executable.starts_with(expected_root) {
                return Err(
                    "The registered EXE uninstaller points outside its install root".into(),
                );
            }
            let inno_log = std::env::temp_dir()
                .join(format!("sd300-exe-uninstall-{}.log", std::process::id()));
            let outcome = run_status(
                Command::new(&executable).args([
                    "/VERYSILENT",
                    "/SUPPRESSMSGBOXES",
                    "/NORESTART",
                    "/SD300GUIALREADYSTOPPED",
                    "/PRESERVEGUISTATE",
                    &format!("/LOG={}", inno_log.display()),
                ]),
                quiet_stdout,
            );
            if let Err(error) = outcome {
                let tail = std::fs::read_to_string(&inno_log)
                    .map(|contents| {
                        contents
                            .lines()
                            .rev()
                            .take(25)
                            .collect::<Vec<_>>()
                            .into_iter()
                            .rev()
                            .collect::<Vec<_>>()
                            .join(" | ")
                    })
                    .unwrap_or_else(|_| "uninstall log unavailable".to_string());
                let _ = std::fs::remove_file(&inno_log);
                return Err(format!("{error}; uninstall log tail: {tail}"));
            }
            let _ = std::fs::remove_file(&inno_log);
            reap_orphaned_inno_uninstaller(&executable);
            Ok(WindowsNativeUninstallExecution::Exe)
        }
        _ => Err("The detected channel is not a Windows native installer".into()),
    }
}

/// Inno uninstallers finish through a relaunched temp copy after the original
/// process returns, and that copy occasionally loses the race to remove the
/// original `unins*.exe` even though the registration, data file, and payloads
/// are gone. Wait briefly for the self-deletion; if a data-less orphan
/// remains inside the just-retired root, remove the proven-dead file and
/// reclaim the directory only when empty. Best-effort: the uninstall itself
/// already succeeded and was verified, so failures here are not fatal.
#[cfg(windows)]
fn reap_orphaned_inno_uninstaller(executable: &Path) {
    let data = executable.with_extension("dat");
    for _ in 0..50 {
        if !executable.is_file() {
            return;
        }
        if !data.is_file() {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(100));
    }
    if executable.is_file() && !data.is_file() {
        let _ = std::fs::remove_file(executable);
        if let Some(root) = executable.parent() {
            let _ = std::fs::remove_dir(root);
        }
    }
}

/// Inno uninstallers finish through a relaunched temp copy whose final
/// `usPostUninstall` work — including machine PATH removal — lands after the
/// original uninstaller process returns, so owned-state verification is
/// eventually consistent. Poll briefly before declaring residue; a clean
/// state returns immediately (MSI removal is synchronous and unaffected).
#[cfg(windows)]
fn verify_windows_native_uninstalled_converged(
    installation: &Installation,
) -> std::result::Result<(), String> {
    let mut last = verify_windows_native_uninstalled(installation);
    for _ in 0..150 {
        if last.is_ok() {
            return last;
        }
        std::thread::sleep(std::time::Duration::from_millis(100));
        last = verify_windows_native_uninstalled(installation);
    }
    last
}

#[cfg(windows)]
fn verify_windows_native_uninstalled(
    installation: &Installation,
) -> std::result::Result<(), String> {
    let mut residual = Vec::new();
    if native_registrations()
        .iter()
        .any(|candidate| candidate.channel == installation.channel)
    {
        residual.push("the proven Installed Apps registration".to_string());
    }
    let root = installation
        .binary_path
        .parent()
        .and_then(Path::parent)
        .ok_or_else(|| "The installed native root was invalid during verification".to_string())?;
    let owned_payloads = [
        installation.binary_path.clone(),
        root.join("app").join("sd300-gui.exe"),
        root.join("app").join("sd300_engine.dll"),
        root.join("app").join("assets").join("app-icon.png"),
        root.join("app").join("assets").join("app-icon.ico"),
        root.join("app").join("assets").join("tray-icon.ico"),
        root.join("app").join("assets").join("tray-icon-dark.ico"),
        root.join("app").join("assets").join("tray-icon-light.ico"),
        root.join("app").join("licenses").join("PRODUCT-LICENSE.md"),
        root.join("app")
            .join("licenses")
            .join("IBM-PLEX-OFL-1.1.txt"),
        root.join("app")
            .join("licenses")
            .join("NATIVE-SDK-APACHE-2.0.txt"),
    ];
    for path in owned_payloads {
        if std::fs::symlink_metadata(&path).is_ok() {
            residual.push(path.display().to_string());
        }
    }
    if let Some(shortcut) = windows_native_gui_shortcut(installation.channel) {
        if std::fs::symlink_metadata(&shortcut).is_ok() {
            residual.push(shortcut.display().to_string());
        }
    }
    if let Some(bin) = installation.binary_path.parent() {
        if windows_path_registry_contains(installation.channel, bin)? {
            residual.push("the native Windows PATH entry".into());
        }
    }
    residual.extend(windows_native_marker_residuals(installation.channel, root)?);
    if !residual.is_empty() {
        return Err(format!(
            "The native uninstaller returned before all owned state was removed: {}",
            residual.join(", ")
        ));
    }
    Ok(())
}

#[cfg(windows)]
fn windows_native_gui_shortcut(channel: InstallChannel) -> Option<PathBuf> {
    let programs = match channel {
        InstallChannel::MsiGlobal | InstallChannel::ExeGlobal => std::env::var_os("ProgramData")
            .map(PathBuf::from)
            .map(|root| {
                root.join("Microsoft")
                    .join("Windows")
                    .join("Start Menu")
                    .join("Programs")
            }),
        InstallChannel::MsiCorporate | InstallChannel::ExeCorporate => {
            std::env::var_os("APPDATA").map(PathBuf::from).map(|root| {
                root.join("Microsoft")
                    .join("Windows")
                    .join("Start Menu")
                    .join("Programs")
            })
        }
        _ => None,
    }?;
    Some(programs.join("SD-300").join("SD-300.lnk"))
}

#[cfg(windows)]
fn windows_native_marker_residuals(
    channel: InstallChannel,
    expected_root: &Path,
) -> std::result::Result<Vec<String>, String> {
    use winreg::enums::{HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE, KEY_READ};
    use winreg::RegKey;

    let mut residual = Vec::new();
    let channel_label = match channel {
        InstallChannel::MsiGlobal => "msi-global",
        InstallChannel::MsiCorporate => "msi-corporate",
        InstallChannel::ExeGlobal => "exe-global",
        InstallChannel::ExeCorporate => "exe-corporate",
        _ => return Ok(residual),
    };
    let user_key = match RegKey::predef(HKEY_CURRENT_USER)
        .open_subkey_with_flags(r"Software\SD300", KEY_READ)
    {
        Ok(key) => Some(key),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
        Err(error) => {
            return Err(format!(
                "Could not inspect current-user native install markers: {error}"
            ))
        }
    };
    if let Some(key) = user_key {
        match key.get_value::<String, _>("InstallSource") {
            Ok(value) if value.eq_ignore_ascii_case(channel_label) => {
                residual.push("the current-user install-source marker".into());
            }
            Ok(_) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => {
                return Err(format!(
                    "Could not verify the current-user install-source marker: {error}"
                ))
            }
        }
        let scoped_source = if matches!(
            channel,
            InstallChannel::MsiGlobal | InstallChannel::ExeGlobal
        ) {
            "InstallSourceGlobal"
        } else {
            "InstallSourceCorporate"
        };
        match key.get_value::<String, _>(scoped_source) {
            Ok(_) => residual.push(format!("the {scoped_source} marker")),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => {
                return Err(format!(
                    "Could not verify the {scoped_source} marker: {error}"
                ))
            }
        }
        let shortcut_marker = if matches!(
            channel,
            InstallChannel::MsiGlobal | InstallChannel::ExeGlobal
        ) {
            "GuiStartMenuShortcutGlobal"
        } else {
            "GuiStartMenuShortcut"
        };
        match key.get_raw_value(shortcut_marker) {
            Ok(_) => residual.push(format!("the {shortcut_marker} marker")),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => {
                return Err(format!(
                    "Could not verify the {shortcut_marker} marker: {error}"
                ))
            }
        }
    }
    let (root, value_name) = if matches!(
        channel,
        InstallChannel::MsiGlobal | InstallChannel::ExeGlobal
    ) {
        (
            RegKey::predef(HKEY_LOCAL_MACHINE),
            "NativeInstallRootGlobal",
        )
    } else {
        (
            RegKey::predef(HKEY_CURRENT_USER),
            "NativeInstallRootCorporate",
        )
    };
    let root_key = match root.open_subkey_with_flags(r"Software\SD300", KEY_READ) {
        Ok(key) => Some(key),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
        Err(error) => {
            return Err(format!(
                "Could not inspect the native install-root marker owner: {error}"
            ))
        }
    };
    if let Some(key) = root_key {
        match key.get_value::<String, _>(value_name) {
            Ok(value) => residual.push(if path_eq(Path::new(&value), expected_root) {
                format!("the {value_name} marker")
            } else {
                format!("the ambiguous {value_name} marker")
            }),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(format!("Could not verify the {value_name} marker: {error}")),
        }
    }
    Ok(residual)
}

#[cfg(not(windows))]
fn uninstall_windows_native(
    _installation: &Installation,
    _quiet_stdout: bool,
) -> std::result::Result<String, String> {
    Err("Windows native uninstall is Windows-only".into())
}

#[cfg(windows)]
fn parse_quoted_executable(command_line: &str) -> Option<PathBuf> {
    let value = command_line.trim();
    if let Some(rest) = value.strip_prefix('"') {
        return rest.find('"').map(|end| PathBuf::from(&rest[..end]));
    }
    value.split_whitespace().next().map(PathBuf::from)
}

#[cfg(target_os = "macos")]
fn uninstall_macos_pkg(quiet_stdout: bool) -> std::result::Result<String, String> {
    remove_empty_managed_receipt_directory()?;
    let script = format!(
        "rm -f /usr/local/bin/sd300 '/Library/Application Support/SD300/install-receipt.json'; rm -rf /Applications/SD-300.app; rmdir '/Library/Application Support/SD300' 2>/dev/null || true; pkgutil --forget {MAC_PKG_ID} >/dev/null"
    );
    run_status(
        Command::new("sudo").args(["sh", "-c", &script]),
        quiet_stdout,
    )?;
    Ok("macOS PKG CLI, application, and receipt were removed".into())
}

#[cfg(not(target_os = "macos"))]
fn uninstall_macos_pkg(_quiet_stdout: bool) -> std::result::Result<String, String> {
    Err("macOS PKG uninstall is macOS-only".into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn relaunches_gui_only_for_explicit_successful_coordinator_updates() {
        assert!(should_relaunch_gui(true, 0));
        assert!(!should_relaunch_gui(false, 0));
        assert!(!should_relaunch_gui(true, 1));
        assert!(!should_relaunch_gui(true, 2));
        assert!(!should_relaunch_gui(false, 1));
    }

    #[test]
    fn compares_semantic_version_segments() {
        assert!(is_newer("1.4.3", "2.0.0"));
        assert!(is_newer("2.0", "2.0.1"));
        assert!(!is_newer("2.0.0", "2.0.0"));
        assert!(!is_newer("2.1.0", "2.0.9"));
    }

    #[test]
    fn every_managed_or_native_channel_preserves_its_own_asset() {
        assert_eq!(
            InstallChannel::PowerShellInstaller.update_asset(),
            Some(POWERSHELL_WRAPPER)
        );
        assert_eq!(
            InstallChannel::ShellInstaller.update_asset(),
            Some(SHELL_WRAPPER)
        );
        assert_eq!(
            InstallChannel::MsiGlobal.update_asset(),
            Some(MSI_GLOBAL_ASSET)
        );
        assert_eq!(
            InstallChannel::MsiCorporate.update_asset(),
            Some(MSI_CORPORATE_ASSET)
        );
        assert_eq!(
            InstallChannel::ExeGlobal.update_asset(),
            Some(EXE_GLOBAL_ASSET)
        );
        assert_eq!(
            InstallChannel::ExeCorporate.update_asset(),
            Some(EXE_CORPORATE_ASSET)
        );
        assert_eq!(InstallChannel::MacPkg.update_asset(), Some(MAC_PKG_ASSET));
        assert_eq!(InstallChannel::Cargo.update_asset(), None);
    }

    #[test]
    fn cargo_completion_hint_fails_closed_and_only_names_the_missing_companion_state() {
        let installation = |channel| Installation {
            channel,
            binary_path: PathBuf::from("sd300"),
        };

        assert!(cargo_gui_completion_needed_for(
            Ok(installation(InstallChannel::Cargo)),
            false
        ));
        assert!(!cargo_gui_completion_needed_for(
            Ok(installation(InstallChannel::Cargo)),
            true
        ));
        assert!(!cargo_gui_completion_needed_for(
            Ok(installation(InstallChannel::ShellInstaller)),
            false
        ));
        assert!(!cargo_gui_completion_needed_for(
            Err("ambiguous ownership".into()),
            false
        ));
    }

    #[test]
    fn public_lifecycle_asset_names_are_versionless() {
        for name in [
            POWERSHELL_WRAPPER,
            SHELL_WRAPPER,
            MSI_GLOBAL_ASSET,
            MSI_CORPORATE_ASSET,
            EXE_GLOBAL_ASSET,
            EXE_CORPORATE_ASSET,
            MAC_PKG_ASSET,
        ] {
            assert!(!name.contains(VERSION));
            assert!(!name.contains("v2.0.0"));
        }
    }

    #[test]
    fn lifecycle_json_always_includes_recovery_context() {
        let result = LifecycleResult {
            action: "update",
            success: false,
            current_version: "2.0.0",
            target_version: None,
            install_channel: None,
            strategy: None,
            message: "ownership is unknown".into(),
        };
        let failed: serde_json::Value =
            serde_json::from_str(&serialize_lifecycle_result(&result, 2)).unwrap();
        assert_eq!(
            failed.get("recovery_url").and_then(|value| value.as_str()),
            Some(RELEASES_PAGE)
        );
        assert_eq!(
            failed
                .get("requires_user_action")
                .and_then(|value| value.as_bool()),
            Some(true)
        );

        let succeeded: serde_json::Value =
            serde_json::from_str(&serialize_lifecycle_result(&result, 0)).unwrap();
        assert_eq!(
            succeeded
                .get("requires_user_action")
                .and_then(|value| value.as_bool()),
            Some(false)
        );
    }

    #[test]
    fn lifecycle_json_preserves_the_v2_0_6_one_object_contract() {
        let fixture: serde_json::Value = serde_json::from_str(include_str!(
            "../tests/fixtures/v2.0.6/lifecycle-result.json"
        ))
        .unwrap();
        for (action, target_version) in [
            ("update", Some("3.0.0")),
            ("install", Some("3.0.0")),
            ("uninstall", None),
        ] {
            let result = LifecycleResult {
                action,
                success: false,
                current_version: "2.0.6",
                target_version,
                install_channel: Some(InstallChannel::PowerShellInstaller),
                strategy: Some("managed-wrapper"),
                message: "fixture failure".into(),
            };
            let serialized = serialize_lifecycle_result(&result, 2);
            assert!(
                !serialized.contains(['\r', '\n']),
                "{action} stdout payload must stay on one line"
            );

            let actual: serde_json::Value = serde_json::from_str(&serialized).unwrap();
            let mut expected = fixture.clone();
            expected["action"] = action.into();
            expected["target_version"] = target_version.into();
            assert_eq!(actual, expected, "{action} JSON contract drifted");
        }
    }

    #[test]
    fn checksum_verification_rejects_wrong_bytes() {
        let directory = tempfile::tempdir().unwrap();
        let asset = directory.path().join("asset");
        let sidecar = directory.path().join("asset.sha256");
        std::fs::write(&asset, b"actual").unwrap();
        std::fs::write(&sidecar, format!("{}  asset\n", "0".repeat(64))).unwrap();
        assert!(verify_sha256(&asset, &sidecar).is_err());
    }

    #[cfg(windows)]
    #[test]
    fn classifies_all_four_windows_native_registrations() {
        assert_eq!(
            classify_windows_registration(
                "{DC74D35F-CBF4-425F-B11E-E9EA87C13CA9}_is1",
                "SD-300 Global",
                r"C:\Program Files\sd300",
                false,
            ),
            Some(InstallChannel::ExeGlobal)
        );
        assert_eq!(
            classify_windows_registration(
                "{ED209931-B5C0-43AE-89F6-83EE2C581653}_is1",
                "SD-300 Corporate",
                r"C:\Users\u\AppData\Local\Programs\sd300",
                false,
            ),
            Some(InstallChannel::ExeCorporate)
        );
        assert_eq!(
            classify_windows_registration(
                "{PRODUCT}",
                "SD-300 Global",
                r"C:\Program Files\sd300",
                true,
            ),
            Some(InstallChannel::MsiGlobal)
        );
        assert_eq!(
            classify_windows_registration(
                "{PRODUCT}",
                "SD-300 Corporate",
                r"C:\Users\u\AppData\Local\Programs\sd300",
                true,
            ),
            Some(InstallChannel::MsiCorporate)
        );
    }

    #[test]
    fn managed_receipt_contradictions_fail_before_cargo_fallback() {
        let current = Path::new("/home/test/.cargo/bin/sd300");
        assert!(managed_receipt_is_current(current, false, None, "2.0.0").is_ok_and(|v| !v));

        let invalid = managed_receipt_is_current(current, true, None, "2.0.0")
            .expect_err("an invalid receipt must block fallback");
        assert!(invalid.contains("No mutation was attempted"));

        let stale = managed_receipt_is_current(
            current,
            true,
            Some((current.to_path_buf(), "1.4.3".into())),
            "2.0.0",
        )
        .expect_err("a stale receipt must block fallback");
        assert!(stale.contains("version 1.4.3"));

        assert!(managed_receipt_is_current(
            current,
            true,
            Some((current.to_path_buf(), "2.0.0".into())),
            "2.0.0",
        )
        .expect("matching receipt should be accepted"));
    }

    #[test]
    fn cargo_manifest_requires_exact_package_and_binary_ownership() {
        let binary = if cfg!(windows) { "sd300.exe" } else { "sd300" };
        let valid = format!(
            r#"{{"installs":{{"tr300-tui 2.0.0 (registry+https://example.invalid/index)":{{"bins":["{binary}"]}}}}}}"#
        );
        assert_eq!(
            cargo_manifest_version(&valid).unwrap().as_deref(),
            Some("2.0.0")
        );

        let wrong_package = format!(
            r#"{{"installs":{{"another-crate 2.0.0 (registry+https://example.invalid/index)":{{"bins":["{binary}"]}}}}}}"#
        );
        assert!(cargo_manifest_version(&wrong_package).is_err());

        let unrelated_package = r#"{"installs":{"another-crate 2.0.0 (registry+https://example.invalid/index)":{"bins":["another"]}}}"#;
        assert_eq!(cargo_manifest_version(unrelated_package).unwrap(), None);

        let wrong_binary = r#"{"installs":{"tr300-tui 2.0.0 (registry+https://example.invalid/index)":{"bins":["other"]}}}"#;
        assert_eq!(cargo_manifest_version(wrong_binary).unwrap(), None);
        assert!(cargo_manifest_version("not json").is_err());
        assert!(cargo_manifest_matches_current(&wrong_package, "2.0.0").is_err());
        assert!(!cargo_manifest_matches_current(unrelated_package, "2.0.0").unwrap());
        assert!(cargo_manifest_matches_current(&valid, "2.0.0").unwrap());
        assert!(cargo_manifest_matches_current(&valid, "1.9.9").is_err());

        let multi_binary = format!(
            r#"{{"installs":{{"tr300-tui 2.0.0 (registry+https://example.invalid/index)":{{"bins":["{binary}","other"]}}}}}}"#
        );
        assert!(cargo_manifest_version(&multi_binary).is_err());
        for malformed in [
            r#"{"installs":{"tr300-tui 2.0.0 (registry+https://example.invalid/index)":{}}}"#,
            r#"{"installs":{"tr300-tui 2.0.0 (registry+https://example.invalid/index)":{"bins":"sd300"}}}"#,
            r#"{"installs":{"tr300-tui 2.0.0 (registry+https://example.invalid/index)":{"bins":[7]}}}"#,
        ] {
            assert!(cargo_manifest_version(malformed).is_err(), "{malformed}");
        }
    }

    #[test]
    fn authoritative_legacy_manifest_drives_cargo_owner_detection() {
        let binary = if cfg!(windows) { "sd300.exe" } else { "sd300" };
        let legacy = format!(
            "[v1]\n\"tr300-tui 2.0.6 (registry+https://example.invalid/index)\" = [\"{binary}\"]\n"
        );
        let unrelated_v2 = r#"{"installs":{"cargo-audit 0.22.2 (registry+https://example.invalid/index)":{"bins":["cargo-audit"]}}}"#;
        assert!(cargo_manifests_match_current(
            &legacy,
            Some(unrelated_v2),
            "2.0.6",
            Path::new(".crates.toml"),
            Path::new(".crates2.json")
        )
        .unwrap());
        assert!(cargo_manifests_match_current(
            &legacy,
            None,
            "2.0.6",
            Path::new(".crates.toml"),
            Path::new(".crates2.json")
        )
        .unwrap());

        let conflicting_v2 = format!(
            r#"{{"installs":{{"tr300-tui 2.0.5 (registry+https://example.invalid/index)":{{"bins":["{binary}"]}}}}}}"#
        );
        assert!(cargo_manifests_match_current(
            &legacy,
            Some(&conflicting_v2),
            "2.0.6",
            Path::new(".crates.toml"),
            Path::new(".crates2.json")
        )
        .is_err());

        let same_version_different_source = format!(
            r#"{{"installs":{{"tr300-tui 2.0.6 (registry+https://different.invalid/index)":{{"bins":["{binary}"]}}}}}}"#
        );
        assert!(cargo_manifests_match_current(
            &legacy,
            Some(&same_version_different_source),
            "2.0.6",
            Path::new(".crates.toml"),
            Path::new(".crates2.json")
        )
        .is_err());
    }

    #[test]
    fn ownership_evidence_requires_regular_files() {
        let temp = tempfile::tempdir().unwrap();
        let special = temp.path().join("special");
        std::fs::create_dir(&special).unwrap();
        assert!(regular_file_presence(&special, "test evidence").is_err());
        assert!(!regular_file_presence(&temp.path().join("missing"), "test evidence").unwrap());
        let regular = temp.path().join("regular");
        std::fs::write(&regular, b"evidence").unwrap();
        assert!(regular_file_presence(&regular, "test evidence").unwrap());
    }

    #[test]
    fn managed_receipt_fields_reject_nested_lookalikes() {
        let exact = r#"{
            "provider":{"source":"cargo-dist"},
            "source":{"app_name":"sd300"},
            "install_prefix":"/managed",
            "version":"2.0.0"
        }"#;
        assert_eq!(
            managed_receipt_fields(exact),
            Some((PathBuf::from("/managed"), "2.0.0".into()))
        );

        let bom_prefixed = format!("\u{feff}{exact}");
        assert_eq!(
            managed_receipt_fields(&bom_prefixed),
            Some((PathBuf::from("/managed"), "2.0.0".into()))
        );

        let nested = r#"{
            "provider":{"source":"foreign"},
            "source":{"app_name":"foreign"},
            "install_prefix":"/managed",
            "version":"2.0.0",
            "lookalike":{"source":"cargo-dist","app_name":"sd300"}
        }"#;
        assert_eq!(managed_receipt_fields(nested), None);
    }

    #[test]
    fn newer_overlapping_ownership_record_wins_and_ties_refuse() {
        let base = std::time::UNIX_EPOCH;
        let earlier = base + std::time::Duration::from_secs(10);
        let later = base + std::time::Duration::from_secs(20);
        assert_eq!(
            ownership_from_times(later, earlier),
            Some(OverlapOwner::Managed)
        );
        assert_eq!(
            ownership_from_times(earlier, later),
            Some(OverlapOwner::Cargo)
        );
        assert_eq!(ownership_from_times(later, later), None);
    }

    #[test]
    fn windows_update_worker_inputs_are_tightly_bounded() {
        assert!(is_worker_release_version("2.0.0"));
        assert!(!is_worker_release_version("v2.0.0"));
        assert!(!is_worker_release_version("2.0.0-rc.1"));
        assert!(is_windows_update_backup_name(
            ".sd300-update-backup-12-345.exe"
        ));
        assert!(!is_windows_update_backup_name("sd300.exe"));
        assert!(is_windows_uninstall_backup_name(
            ".sd300-uninstall-backup-12-345.exe"
        ));
        assert!(!is_windows_uninstall_backup_name(
            ".sd300-uninstall-backup-owner-nonce.exe"
        ));
        assert_eq!(
            windows_quote_command_arg(r"C:\Program Files\sd300\backup.exe"),
            r#""C:\Program Files\sd300\backup.exe""#
        );
    }

    #[test]
    fn windows_installer_success_codes_preserve_committed_reboot_state() {
        assert_eq!(
            windows_installer_completion(0),
            Some(WindowsInstallerCompletion::Complete)
        );
        assert_eq!(
            windows_installer_completion(1641),
            Some(WindowsInstallerCompletion::RebootRequired(1641))
        );
        assert_eq!(
            windows_installer_completion(3010),
            Some(WindowsInstallerCompletion::RebootRequired(3010))
        );
        assert_eq!(windows_installer_completion(1603), None);
        assert_eq!(windows_installer_completion(-1), None);
        assert!(
            UpdateExecution::WindowsMsiCommitted(WindowsInstallerCompletion::Complete)
                .windows_msi_committed()
        );
        assert!(
            UpdateExecution::WindowsMsiCommitted(WindowsInstallerCompletion::RebootRequired(3010))
                .windows_msi_committed()
        );
        assert!(!UpdateExecution::Standard.windows_msi_committed());
        assert_eq!(
            UpdateExecution::WindowsMsiCommitted(WindowsInstallerCompletion::RebootRequired(1641))
                .worker_exit_code(),
            1641
        );
        assert_eq!(
            UpdateExecution::WindowsMsiCommitted(WindowsInstallerCompletion::RebootRequired(3010))
                .worker_exit_code(),
            3010
        );
        for completion in [
            WindowsInstallerCompletion::Complete,
            WindowsInstallerCompletion::RebootRequired(1641),
            WindowsInstallerCompletion::RebootRequired(3010),
        ] {
            let encoded = windows_committed_worker_failure_exit_code(completion);
            assert_eq!(
                windows_committed_worker_failure_completion(encoded as u32),
                Some(completion)
            );
            assert_eq!(
                UpdateExecution::WindowsMsiCommitted(completion)
                    .committed_worker_failure_exit_code(),
                encoded
            );
        }
        assert!(windows_pending_verification_reboot_message(
            WindowsInstallerCompletion::RebootRequired(3010)
        )
        .contains("requires a reboot"));
        let committed_failure =
            UpdateExecution::WindowsMsiCommitted(WindowsInstallerCompletion::RebootRequired(3010))
                .post_commit_failure("GUI verification failed".into());
        assert!(committed_failure.contains("already committed"));
        assert!(committed_failure.contains("prior executable was not restored"));
        assert!(committed_failure.contains("requires a reboot"));
        assert!(!committed_failure.contains("Run the same update again"));
        let committed_without_reboot =
            UpdateExecution::WindowsMsiCommitted(WindowsInstallerCompletion::Complete)
                .post_commit_failure("GUI verification failed".into());
        assert!(committed_without_reboot.contains("Run the same update again"));
        assert_eq!(
            UpdateExecution::Standard.post_commit_failure("ordinary failure".into()),
            "ordinary failure"
        );
        #[cfg(windows)]
        {
            let uninstall = WindowsNativeUninstallExecution::MsiCommitted(
                WindowsInstallerCompletion::RebootRequired(1641),
            );
            assert!(uninstall.windows_msi_committed());
            assert!(uninstall.reboot_required());
            assert_eq!(uninstall.worker_exit_code(), 1641);
        }
    }

    #[cfg(windows)]
    #[test]
    fn windows_powershell_program_resolves_the_trusted_in_box_shell() {
        let program = windows_powershell_program().expect("in-box PowerShell must resolve");
        let path = std::path::PathBuf::from(&program);
        assert!(path.is_absolute());
        assert!(path.ends_with("WindowsPowerShell\\v1.0\\powershell.exe"));
    }

    #[test]
    fn managed_windows_cleanup_removes_integrations_when_gui_root_is_already_missing() {
        let commands = windows_managed_cleanup_commands(
            Some(42),
            Path::new(r"C:\Users\test\bin\sd300.exe"),
            None,
            Some(Path::new(
                r"C:\Users\test\AppData\Roaming\Microsoft\Windows\Start Menu\Programs\SD-300.lnk",
            )),
            Some(Path::new(r"C:\Users\test\.sd300\install-receipt.json")),
            false,
        );
        assert!(commands.contains("SD-300.lnk"));
        assert!(commands.contains("Uninstall\\SD-300-Managed"));
        assert!(commands.contains("install-receipt.json"));
        assert!(commands
            .contains("[Microsoft.Win32.RegistryValueOptions]::DoNotExpandEnvironmentNames"));
        assert!(commands.contains("HResult -band 0xFFFF) -ne 145"));
        assert!(commands.ends_with("exit 0"));
        // Path::parent resolves the literal Windows receipt path only where `\`
        // is a separator; non-Windows hosts see one component and an empty parent.
        #[cfg(windows)]
        {
            assert!(commands.contains("[IO.Directory]::Delete('C:\\Users\\test\\.sd300',$false)"));
            assert!(!commands.contains("Remove-Item -LiteralPath 'C:\\Users\\test\\.sd300'"));
        }
        assert!(!commands.contains("$sd300Root="));
    }

    #[test]
    fn managed_windows_cleanup_retries_a_proven_gui_root() {
        let commands = windows_managed_cleanup_commands(
            Some(42),
            Path::new(r"C:\Users\test\bin\sd300.exe"),
            Some(Path::new(r"C:\Users\test\AppData\Local\Programs\SD-300")),
            None,
            None,
            false,
        );
        assert!(commands.contains("for($i=0;$i -lt 50"));
        assert!(commands.contains("Test-Path -LiteralPath $sd300Root"));
        assert!(commands.contains("Uninstall\\SD-300-Managed"));
    }

    #[test]
    fn synchronous_windows_cleanup_does_not_wait_for_the_calling_process() {
        let commands = windows_managed_cleanup_commands(
            None,
            Path::new(r"C:\Users\test\bin\sd300.exe"),
            None,
            None,
            None,
            true,
        );
        assert!(!commands.contains("Wait-Process"));
        assert!(commands.contains("cargo uninstall tr300-tui"));
    }

    #[test]
    fn windows_uninstall_path_and_startup_matching_are_exact() {
        assert!(windows_path_value_contains(
            r"C:\Windows;C:\Users\test\bin\;C:\Tools",
            Path::new(r"c:\users\test\bin")
        ));
        assert!(!windows_path_value_contains(
            r"C:\Users\test\binary",
            Path::new(r"C:\Users\test\bin")
        ));
        assert_eq!(
            windows_startup_command_target(
                r#""C:\Users\test\Programs\SD-300\app\sd300-gui.exe" --startup --hidden"#
            ),
            Some(PathBuf::from(
                r"C:\Users\test\Programs\SD-300\app\sd300-gui.exe"
            ))
        );
        assert!(windows_startup_command_target(
            r#""C:\Users\test\Programs\SD-300\app\sd300-gui.exe" --unexpected"#
        )
        .is_none());
    }

    #[cfg(windows)]
    #[test]
    fn msi_reinstall_properties_are_reserved_for_same_version_repairs() {
        assert!(windows_msi_repair_required("3.0.0", "3.0.0"));
        assert!(!windows_msi_repair_required("1.9.9", "3.0.0"));
    }
}
