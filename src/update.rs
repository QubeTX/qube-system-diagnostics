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
    let outcome = perform_update(&installation, &release, json);
    match outcome {
        Ok(()) => emit(
            json,
            LifecycleResult {
                action: "update",
                success: true,
                current_version: VERSION,
                target_version: Some(&release.version),
                install_channel: Some(installation.channel),
                strategy: Some(strategy),
                message: format!(
                    "Updated to {} without changing the {} channel",
                    release.version,
                    installation.channel.label()
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
    if !json {
        println!(
            "Installing SD-300 {} through the preferred {} channel...",
            release.version,
            channel.label()
        );
    }
    let outcome = execute_managed_wrapper(channel, &release, json).and_then(|()| {
        let binary = managed_receipt_binary().ok_or_else(|| {
            "The managed installer finished without a matching receipt and binary".to_string()
        })?;
        verify_version(&binary, &release.version)
    });
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
    match execute_uninstall(&installation, json) {
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
) -> std::result::Result<(), String> {
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
        ),
        InstallChannel::PowerShellInstaller | InstallChannel::ShellInstaller => {
            execute_managed_wrapper(installation.channel, release, quiet_stdout)
        }
        InstallChannel::MsiGlobal | InstallChannel::MsiCorporate => {
            execute_windows_msi(installation.channel, release, quiet_stdout)
        }
        InstallChannel::ExeGlobal | InstallChannel::ExeCorporate => {
            execute_windows_exe(installation.channel, release, quiet_stdout)
        }
        InstallChannel::MacPkg => execute_macos_pkg(release, quiet_stdout),
    }
}

fn perform_update(
    installation: &Installation,
    release: &Release,
    quiet_stdout: bool,
) -> std::result::Result<(), String> {
    #[cfg(windows)]
    {
        if installation.channel.global_worker_id().is_some() {
            return with_elevated_windows_live_image_handoff(installation, release);
        }
        with_windows_live_image_handoff(|| {
            execute_update(installation, release, quiet_stdout)?;
            verify_version(&installation.binary_path, &release.version)
        })
    }

    #[cfg(not(windows))]
    {
        execute_update(installation, release, quiet_stdout)?;
        verify_version(&installation.binary_path, &release.version)
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
fn execute_windows_msi(
    channel: InstallChannel,
    release: &Release,
    quiet_stdout: bool,
) -> std::result::Result<(), String> {
    let asset = channel
        .update_asset()
        .ok_or_else(|| "MSI asset is unavailable".to_string())?;
    let staged = stage_verified(release, asset)?;
    run_status(
        Command::new("msiexec.exe").args([
            "/i",
            &staged.path.to_string_lossy(),
            "/passive",
            "/norestart",
        ]),
        quiet_stdout,
    )
}

#[cfg(not(windows))]
fn execute_windows_msi(
    _channel: InstallChannel,
    _release: &Release,
    _quiet_stdout: bool,
) -> std::result::Result<(), String> {
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
        Command::new(&staged.path).args(["/VERYSILENT", "/SUPPRESSMSGBOXES", "/NORESTART"]),
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
fn with_elevated_windows_live_image_handoff(
    installation: &Installation,
    release: &Release,
) -> std::result::Result<(), String> {
    let handoff = WindowsLiveImageHandoff::plan()?;
    let channel = installation
        .channel
        .global_worker_id()
        .ok_or_else(|| "Refused a non-Global elevated update request".to_string())?;
    let exit_code =
        launch_elevated_windows_update_worker(channel, &release.version, &handoff.backup)?;
    if exit_code != 0 {
        return Err(format!(
            "The elevated same-channel update worker exited with code {exit_code}. It retained or restored the old executable; verify `sd300 --version` before retrying."
        ));
    }
    verify_version(&installation.binary_path, &release.version)
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
    let outcome = execute_update(&installation, &release, true)
        .and_then(|()| verify_version(&handoff.original, version));
    match handoff.finish(outcome) {
        Ok(()) => 0,
        Err(message) => {
            eprintln!("SD-300 update worker failed safely: {message}");
            2
        }
    }
}

#[cfg(not(windows))]
pub fn run_windows_update_worker(_channel: &str, _version: &str, _backup: &Path) -> i32 {
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

#[cfg(windows)]
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

fn verify_version(path: &Path, expected: &str) -> std::result::Result<(), String> {
    if !path.is_file() {
        return Err(format!("Installed binary is missing: {}", path.display()));
    }
    let output = run_output(
        &path.to_string_lossy(),
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
    Ok("Cargo-owned SD-300 was removed; the Rust toolchain was preserved".into())
}

#[cfg(windows)]
fn uninstall_managed(installation: &Installation) -> std::result::Result<String, String> {
    schedule_windows_file_cleanup(installation, false)?;
    Ok("Managed SD-300 removal was scheduled for immediately after this process exits".into())
}

#[cfg(not(windows))]
fn uninstall_managed(installation: &Installation) -> std::result::Result<String, String> {
    std::fs::remove_file(&installation.binary_path)
        .map_err(|error| format!("Could not remove managed binary: {error}"))?;
    if let Some(receipt) = receipt_path() {
        let _ = std::fs::remove_file(receipt);
    }
    Ok("Managed SD-300 binary and receipt were removed".into())
}

#[cfg(windows)]
fn schedule_windows_file_cleanup(
    installation: &Installation,
    cargo_owned: bool,
) -> std::result::Result<(), String> {
    let receipt = receipt_path();
    let mut commands = format!(
        "Wait-Process -Id {} -ErrorAction SilentlyContinue; ",
        std::process::id()
    );
    if cargo_owned {
        commands.push_str("& cargo uninstall tr300-tui *> $null; ");
    }
    commands.push_str(&format!(
        "Remove-Item -LiteralPath '{}' -Force -ErrorAction SilentlyContinue; ",
        powershell_escape(&installation.binary_path.to_string_lossy())
    ));
    if let Some(receipt) = receipt {
        commands.push_str(&format!(
            "Remove-Item -LiteralPath '{}' -Force -ErrorAction SilentlyContinue; ",
            powershell_escape(&receipt.to_string_lossy())
        ));
    }
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

#[cfg(windows)]
fn uninstall_windows_native(
    installation: &Installation,
    quiet_stdout: bool,
) -> std::result::Result<String, String> {
    let registrations = native_registrations();
    let registration = registrations
        .iter()
        .find(|registration| registration.channel == installation.channel)
        .ok_or_else(|| "The proven native registration disappeared before uninstall".to_string())?;
    match installation.channel {
        InstallChannel::MsiGlobal | InstallChannel::MsiCorporate => {
            run_status(
                Command::new("msiexec.exe").args([
                    "/x",
                    &registration.key_name,
                    "/passive",
                    "/norestart",
                ]),
                quiet_stdout,
            )?;
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
                Command::new(executable).args(["/VERYSILENT", "/SUPPRESSMSGBOXES", "/NORESTART"]),
                quiet_stdout,
            )?;
        }
        _ => return Err("The detected channel is not a Windows native installer".into()),
    }
    Ok(format!(
        "{} uninstall completed",
        installation.channel.label()
    ))
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
    let script = format!(
        "rm -f /usr/local/bin/sd300 '/Library/Application Support/SD300/install-receipt.json'; pkgutil --forget {MAC_PKG_ID} >/dev/null"
    );
    run_status(
        Command::new("sudo").args(["sh", "-c", &script]),
        quiet_stdout,
    )?;
    Ok("macOS PKG payload and receipt were removed".into())
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
        assert_eq!(
            windows_quote_command_arg(r"C:\Program Files\sd300\backup.exe"),
            r#""C:\Program Files\sd300\backup.exe""#
        );
    }
}
