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

    fn post_commit_failure(self, message: String) -> String {
        if matches!(self, Self::WindowsMsiCommitted(_)) {
            format!(
                "{message}; Windows Installer already committed the product transaction, so the prior executable was not restored. Run the same update again to repair the composite installation"
            )
        } else {
            message
        }
    }

    #[cfg(any(windows, test))]
    fn worker_exit_code(self) -> i32 {
        match self {
            Self::WindowsMsiCommitted(WindowsInstallerCompletion::RebootRequired(code)) => code,
            _ => 0,
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
}

#[cfg(windows)]
const WINDOWS_WORKER_MSI_COMMITTED_NEEDS_REPAIR: i32 = 200;

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
    let outcome = perform_update(&installation, &release, json).and_then(|execution| {
        crate::gui::verify_installed(&release.version)
            .map_err(|message| execution.post_commit_failure(message))?;
        Ok(execution)
    });
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
    }
    .and_then(|execution| {
        crate::gui::verify_installed(&release.version)
            .map_err(|message| execution.post_commit_failure(message))?;
        Ok(execution)
    });

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
    let outcome = perform_managed_install(channel, &release, json)
        .and_then(|()| crate::gui::verify_installed(&release.version));
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
        crate::settings::remove_owned_gui_state()?;
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
            verify_version(&installation.binary_path, &release.version),
        )
    }

    #[cfg(not(windows))]
    {
        let execution = execute_update(installation, release, quiet_stdout)?;
        verify_version(&installation.binary_path, &release.version)?;
        Ok(execution)
    }
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
        verify_version(&binary, &release.version)
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
            let program = if tool_exists("powershell.exe") {
                "powershell.exe"
            } else if tool_exists("pwsh.exe") {
                "pwsh.exe"
            } else {
                return Err("PowerShell is not available".into());
            };
            run_status(
                Command::new(program).args([
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
    if exit_code == WINDOWS_WORKER_MSI_COMMITTED_NEEDS_REPAIR as u32 {
        return Err(
            "Windows Installer committed the Global MSI transaction, but post-install verification or cleanup failed; the prior executable was not restored. Run the same update again to repair the composite installation."
                .into(),
        );
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
    verify_version(&installation.binary_path, &release.version)
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
    verify_version(&binary, &release.version)
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
    let verification = verify_version(&handoff.original, version);
    match handoff.finish_execution(execution, verification) {
        Ok(execution) => execution.worker_exit_code(),
        Err(message) => {
            eprintln!("SD-300 update worker failed safely: {message}");
            if execution.windows_msi_committed() {
                WINDOWS_WORKER_MSI_COMMITTED_NEEDS_REPAIR
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
            verify_version(&binary, version)
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
pub fn run_windows_uninstall_worker(channel: &str, backup: &Path) -> i32 {
    let Some(channel) = InstallChannel::from_global_worker_id(channel) else {
        return 2;
    };
    let installation = match detect_installation() {
        Ok(installation) if installation.channel == channel => installation,
        Ok(_) => return 2,
        Err(message) => {
            eprintln!("SD-300 uninstall worker preflight failed safely: {message}");
            return 2;
        }
    };
    let handoff = match WindowsUninstallImageHandoff::begin_with_backup(backup) {
        Ok(handoff) => handoff,
        Err(message) => {
            eprintln!("SD-300 uninstall worker failed safely: {message}");
            return 2;
        }
    };
    let execution = match execute_windows_native_uninstaller(&installation, true) {
        Ok(execution) => execution,
        Err(message) => {
            return match handoff.finish(Err(message)) {
                Ok(()) => 2,
                Err(message) => {
                    eprintln!("SD-300 uninstall worker failed safely: {message}");
                    2
                }
            }
        }
    };
    let verification = verify_windows_native_uninstalled(&installation);
    match handoff.finish_execution(execution, verification) {
        Ok(execution) => execution.worker_exit_code(),
        Err(message) => {
            eprintln!("SD-300 uninstall worker failed safely: {message}");
            if execution.windows_msi_committed() {
                WINDOWS_WORKER_MSI_COMMITTED_NEEDS_REPAIR
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
        let program = if tool_exists("powershell.exe") {
            "powershell.exe"
        } else if tool_exists("pwsh.exe") {
            "pwsh.exe"
        } else {
            return Err("PowerShell is required to download release assets".into());
        };
        let script = format!(
            "$ProgressPreference='SilentlyContinue'; $ErrorActionPreference='Stop'; Invoke-WebRequest -UseBasicParsing -Uri '{}' -OutFile '{}'",
            powershell_escape(url),
            powershell_escape(&destination.to_string_lossy())
        );
        run_status(
            Command::new(program).args([
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

fn detect_installation() -> std::result::Result<Installation, String> {
    let current = std::env::current_exe()
        .map_err(|error| format!("Could not resolve the running executable: {error}"))?;

    let managed_receipt = receipt_path();
    let receipt_present = managed_receipt
        .as_ref()
        .is_some_and(|receipt| receipt.is_file());
    let managed_current = managed_receipt_is_current(
        &current,
        receipt_present,
        managed_receipt_evidence(),
        VERSION,
    )?;
    let cargo_current = cargo_install_is_current(&current, VERSION)?;
    if managed_current && cargo_current {
        let receipt = managed_receipt
            .as_deref()
            .expect("a current managed receipt has a path");
        let manifest = cargo_manifest_path()
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

fn managed_receipt_evidence() -> Option<(PathBuf, String)> {
    let receipt = std::fs::read_to_string(receipt_path()?).ok()?;
    let (prefix, version) = managed_receipt_fields(&receipt)?;
    let binary = prefix
        .join("bin")
        .join(if cfg!(windows) { "sd300.exe" } else { "sd300" });
    binary.is_file().then_some((binary, version))
}

fn managed_receipt_fields(receipt: &str) -> Option<(PathBuf, String)> {
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
    let manifest_path =
        cargo_manifest_path().expect("cargo home exists when its binary path exists");
    let manifest = match std::fs::read_to_string(&manifest_path) {
        Ok(manifest) => manifest,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(false),
        Err(error) => {
            return Err(format!(
                "The running path looks like Cargo, but {} could not be read: {}. No mutation was attempted.",
                manifest_path.display(),
                error
            ));
        }
    };
    cargo_manifest_matches_current(&manifest, current_version)
}

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
    let mut versions = installs.iter().filter_map(|(key, value)| {
        let remainder = key.strip_prefix(&prefix)?;
        let owns_binary = value
            .get("bins")
            .and_then(serde_json::Value::as_array)
            .is_some_and(|bins| bins.iter().any(|bin| bin.as_str() == Some(binary_name)));
        owns_binary
            .then(|| {
                remainder
                    .split_once(" (")
                    .map_or(remainder, |(version, _)| version)
            })
            .map(str::to_string)
    });
    let version = versions.next();
    if versions.next().is_some() {
        return Err(
            "Cargo records multiple tr300-tui installations owning sd300. No mutation was attempted."
                .into(),
        );
    }
    Ok(version)
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
    _quiet_stdout: bool,
) -> std::result::Result<String, String> {
    schedule_windows_file_cleanup(installation, true)?;
    Ok("Cargo uninstall was scheduled for immediately after this process exits".into())
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
    schedule_windows_file_cleanup(installation, false)?;
    Ok("Managed SD-300 removal was scheduled for immediately after this process exits".into())
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
fn schedule_windows_file_cleanup(
    installation: &Installation,
    cargo_owned: bool,
) -> std::result::Result<(), String> {
    let receipt = receipt_path();
    let managed_gui = if cargo_owned {
        None
    } else {
        prove_windows_managed_gui_root()?
    };
    let shortcut = (!cargo_owned).then(windows_managed_gui_shortcut).flatten();
    let commands = windows_managed_cleanup_commands(
        std::process::id(),
        &installation.binary_path,
        managed_gui.as_deref(),
        shortcut.as_deref(),
        receipt.as_deref(),
        cargo_owned,
    );
    Command::new("powershell.exe")
        .args([
            "-NoProfile",
            "-NonInteractive",
            "-ExecutionPolicy",
            "Bypass",
            "-WindowStyle",
            "Hidden",
            "-Command",
            &commands,
        ])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map(|_| ())
        .map_err(|error| format!("Could not schedule post-exit cleanup: {error}"))
}

#[cfg(any(windows, test))]
fn windows_managed_cleanup_commands(
    process_id: u32,
    binary: &Path,
    managed_gui: Option<&Path>,
    shortcut: Option<&Path>,
    receipt: Option<&Path>,
    cargo_owned: bool,
) -> String {
    let mut commands = format!("Wait-Process -Id {process_id} -ErrorAction SilentlyContinue; ");
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
            commands.push_str(&format!(
                "Remove-Item -LiteralPath '{}' -Force -ErrorAction SilentlyContinue; ",
                powershell_escape(&parent.to_string_lossy())
            ));
        }
    }
    commands
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
        if exit_code == WINDOWS_WORKER_MSI_COMMITTED_NEEDS_REPAIR as u32 {
            return Err(
                "Windows Installer committed the Global MSI uninstall, but removal verification or final cleanup failed; the installed executable was not restored. Retry from Installed Apps or the original MSI if SD-300 remains discoverable."
                .into(),
            );
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
                "The elevated {} uninstaller exited with code {exit_code}; verify the installed command before retrying",
                installation.channel.label()
            ));
        }
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
    let verification = verify_windows_native_uninstalled(installation);
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
            run_status(
                Command::new(executable).args([
                    "/VERYSILENT",
                    "/SUPPRESSMSGBOXES",
                    "/NORESTART",
                    "/SD300GUIALREADYSTOPPED",
                    "/PRESERVEGUISTATE",
                ]),
                quiet_stdout,
            )?;
            Ok(WindowsNativeUninstallExecution::Exe)
        }
        _ => Err("The detected channel is not a Windows native installer".into()),
    }
}

#[cfg(windows)]
fn verify_windows_native_uninstalled(
    installation: &Installation,
) -> std::result::Result<(), String> {
    if native_registrations()
        .iter()
        .any(|candidate| candidate.channel == installation.channel)
    {
        return Err("The proven native registration remained after its uninstaller exited".into());
    }
    Ok(())
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
        assert_eq!(cargo_manifest_version(&wrong_package).unwrap(), None);

        let wrong_binary = r#"{"installs":{"tr300-tui 2.0.0 (registry+https://example.invalid/index)":{"bins":["other"]}}}"#;
        assert_eq!(cargo_manifest_version(wrong_binary).unwrap(), None);
        assert!(cargo_manifest_version("not json").is_err());
        assert!(!cargo_manifest_matches_current(&wrong_package, "2.0.0").unwrap());
        assert!(cargo_manifest_matches_current(&valid, "2.0.0").unwrap());
        assert!(cargo_manifest_matches_current(&valid, "1.9.9").is_err());
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
        let committed_failure =
            UpdateExecution::WindowsMsiCommitted(WindowsInstallerCompletion::RebootRequired(3010))
                .post_commit_failure("GUI verification failed".into());
        assert!(committed_failure.contains("already committed"));
        assert!(committed_failure.contains("prior executable was not restored"));
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

    #[test]
    fn managed_windows_cleanup_removes_integrations_when_gui_root_is_already_missing() {
        let commands = windows_managed_cleanup_commands(
            42,
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
        assert!(!commands.contains("$sd300Root="));
    }

    #[test]
    fn managed_windows_cleanup_retries_a_proven_gui_root() {
        let commands = windows_managed_cleanup_commands(
            42,
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

    #[cfg(windows)]
    #[test]
    fn msi_reinstall_properties_are_reserved_for_same_version_repairs() {
        assert!(windows_msi_repair_required("3.0.0", "3.0.0"));
        assert!(!windows_msi_repair_required("1.9.9", "3.0.0"));
    }
}
