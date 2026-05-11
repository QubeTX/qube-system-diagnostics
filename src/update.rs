use std::process::{Command, Stdio};

use crate::collectors::command::{run_output, CommandTimeout};
use crate::error::Result;

pub const RELEASES_URL: &str =
    "https://api.github.com/repos/QubeTX/qube-system-diagnostics/releases/latest";
const CRATE_NAME: &str = "tr300-tui";
const MANUAL_INSTALL_URL: &str = "https://github.com/QubeTX/qube-system-diagnostics#installation";

#[cfg(not(windows))]
const SHELL_INSTALLER: &str =
    "https://github.com/QubeTX/qube-system-diagnostics/releases/latest/download/tr300-tui-installer.sh";

#[cfg(windows)]
const PS_INSTALLER: &str =
    "https://github.com/QubeTX/qube-system-diagnostics/releases/latest/download/tr300-tui-installer.ps1";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum UpdateStrategy {
    Cargo,
    InstallerCurl,
    InstallerWget,
    InstallerPowerShell,
    InstallerPwsh,
}

impl UpdateStrategy {
    fn label(self) -> &'static str {
        match self {
            Self::Cargo => "cargo install",
            Self::InstallerCurl => "curl shell installer",
            Self::InstallerWget => "wget shell installer",
            Self::InstallerPowerShell => "PowerShell installer",
            Self::InstallerPwsh => "pwsh installer",
        }
    }

    #[cfg(test)]
    fn json_method(self) -> &'static str {
        if matches!(self, Self::Cargo) {
            "cargo"
        } else {
            "installer"
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TargetOs {
    Unix,
    Windows,
}

#[derive(Debug)]
enum StrategyError {
    Preflight(String),
    Runtime(String),
}

#[derive(Debug)]
struct AttemptRecord {
    strategy: UpdateStrategy,
    skipped: bool,
    message: String,
}

#[derive(Debug)]
struct UpdateFailure {
    attempts: Vec<AttemptRecord>,
}

pub fn run() -> Result<i32> {
    println!();
    println!("  * Checking for updates...");

    let latest = match fetch_latest_version() {
        Ok(version) => version,
        Err(message) => {
            eprintln!("  Update check failed: {}", message);
            return Ok(2);
        }
    };
    let current = env!("CARGO_PKG_VERSION").to_string();

    if !is_newer(&current, &latest) {
        println!("  Already on the latest version (v{})", current);
        return Ok(0);
    }

    println!("  Update available: v{} -> v{}", current, latest);
    let strategies = build_strategy_list();
    if let Some(strategy) = strategies.first() {
        println!("  Updating via {}...", strategy.label());
    }
    println!();

    match execute_update(&strategies) {
        Ok(strategy) => {
            println!();
            println!("  Updated to v{} via {}", latest, strategy.label());
            Ok(0)
        }
        Err(failure) => {
            println!();
            eprintln!("  Update failed. Strategies attempted:");
            for attempt in &failure.attempts {
                let kind = if attempt.skipped { "skipped" } else { "failed" };
                eprintln!(
                    "    - {} {}: {}",
                    attempt.strategy.label(),
                    kind,
                    attempt.message
                );
            }
            eprintln!("  To update manually, see: {}", MANUAL_INSTALL_URL);
            Ok(2)
        }
    }
}

fn fetch_latest_version() -> std::result::Result<String, String> {
    let json = fetch_latest_release_json(current_target_os())?;
    let body: serde_json::Value =
        serde_json::from_str(&json).map_err(|e| format!("failed to parse response: {}", e))?;

    let tag = body["tag_name"]
        .as_str()
        .ok_or_else(|| "missing tag_name in release response".to_string())?;

    Ok(tag.strip_prefix('v').unwrap_or(tag).to_string())
}

fn fetch_latest_release_json(os: TargetOs) -> std::result::Result<String, String> {
    let mut attempts = Vec::new();
    let user_agent = format!("User-Agent: sd300/{}", env!("CARGO_PKG_VERSION"));
    let accept = "Accept: application/vnd.github+json".to_string();

    match os {
        TargetOs::Unix => {
            if let Some(stdout) = run_fetch_command(
                "curl",
                vec![
                    "--proto".into(),
                    "=https".into(),
                    "--tlsv1.2".into(),
                    "-fLsS".into(),
                    "-H".into(),
                    user_agent.clone(),
                    "-H".into(),
                    accept.clone(),
                    RELEASES_URL.into(),
                ],
                &mut attempts,
            ) {
                return Ok(stdout);
            }
            if let Some(stdout) = run_fetch_command(
                "wget",
                vec![
                    "-qO-".into(),
                    format!("--header={}", user_agent),
                    format!("--header={}", accept),
                    RELEASES_URL.into(),
                ],
                &mut attempts,
            ) {
                return Ok(stdout);
            }
        }
        TargetOs::Windows => {
            let script = format!(
                "$ProgressPreference='SilentlyContinue'; \
                 Invoke-RestMethod -Headers @{{'User-Agent'='sd300/{version}'; 'Accept'='application/vnd.github+json'}} \
                 -Uri '{url}' | ConvertTo-Json -Compress",
                version = env!("CARGO_PKG_VERSION"),
                url = RELEASES_URL
            );
            for program in ["powershell.exe", "pwsh.exe"] {
                if let Some(stdout) = run_fetch_command(
                    program,
                    vec![
                        "-NoProfile".into(),
                        "-NonInteractive".into(),
                        "-ExecutionPolicy".into(),
                        "Bypass".into(),
                        "-Command".into(),
                        script.clone(),
                    ],
                    &mut attempts,
                ) {
                    return Ok(stdout);
                }
            }
        }
    }

    Err(format!(
        "no supported release-check command succeeded ({})",
        attempts.join("; ")
    ))
}

fn run_fetch_command(
    program: &str,
    args: Vec<String>,
    attempts: &mut Vec<String>,
) -> Option<String> {
    let Some(output) = run_output(
        program,
        args,
        CommandTimeout::Custom(std::time::Duration::from_secs(15)),
    ) else {
        attempts.push(format!("{} unavailable or timed out", program));
        return None;
    };

    if output.status.success() {
        return Some(String::from_utf8_lossy(&output.stdout).to_string());
    }

    let stderr = String::from_utf8_lossy(&output.stderr);
    let message = stderr.lines().next().unwrap_or("non-zero exit");
    attempts.push(format!("{} failed: {}", program, message));
    None
}

fn is_newer(current: &str, latest: &str) -> bool {
    let parse =
        |value: &str| -> Vec<u64> { value.split('.').filter_map(|s| s.parse().ok()).collect() };
    let current = parse(current);
    let latest = parse(latest);
    let len = current.len().max(latest.len());

    for idx in 0..len {
        let c = current.get(idx).copied().unwrap_or(0);
        let l = latest.get(idx).copied().unwrap_or(0);
        if l > c {
            return true;
        }
        if l < c {
            return false;
        }
    }

    false
}

fn order_strategies(cargo_invokable: bool, os: TargetOs) -> Vec<UpdateStrategy> {
    let mut strategies = Vec::new();
    if cargo_invokable {
        strategies.push(UpdateStrategy::Cargo);
    }
    match os {
        TargetOs::Unix => {
            strategies.push(UpdateStrategy::InstallerCurl);
            strategies.push(UpdateStrategy::InstallerWget);
        }
        TargetOs::Windows => {
            strategies.push(UpdateStrategy::InstallerPowerShell);
            strategies.push(UpdateStrategy::InstallerPwsh);
        }
    }
    strategies
}

fn current_target_os() -> TargetOs {
    if cfg!(windows) {
        TargetOs::Windows
    } else {
        TargetOs::Unix
    }
}

fn tool_exists(tool: &str) -> bool {
    Command::new(tool)
        .arg("--version")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
}

fn build_strategy_list() -> Vec<UpdateStrategy> {
    order_strategies(tool_exists("cargo"), current_target_os())
}

fn execute_update(
    strategies: &[UpdateStrategy],
) -> std::result::Result<UpdateStrategy, UpdateFailure> {
    let mut attempts = Vec::new();
    for &strategy in strategies {
        match try_strategy(strategy) {
            Ok(()) => return Ok(strategy),
            Err(StrategyError::Preflight(message)) => attempts.push(AttemptRecord {
                strategy,
                skipped: true,
                message,
            }),
            Err(StrategyError::Runtime(message)) => attempts.push(AttemptRecord {
                strategy,
                skipped: false,
                message,
            }),
        }
    }
    Err(UpdateFailure { attempts })
}

fn try_strategy(strategy: UpdateStrategy) -> std::result::Result<(), StrategyError> {
    match strategy {
        UpdateStrategy::Cargo => run_command_status("cargo", &["install", CRATE_NAME, "--force"]),
        UpdateStrategy::InstallerCurl => try_installer_curl(),
        UpdateStrategy::InstallerWget => try_installer_wget(),
        UpdateStrategy::InstallerPowerShell => try_installer_powershell("powershell"),
        UpdateStrategy::InstallerPwsh => try_installer_powershell("pwsh"),
    }
}

fn run_command_status(program: &str, args: &[&str]) -> std::result::Result<(), StrategyError> {
    match Command::new(program).args(args).status() {
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            Err(StrategyError::Preflight(format!("{} not on PATH", program)))
        }
        Err(e) => Err(StrategyError::Preflight(format!(
            "failed to spawn {}: {}",
            program, e
        ))),
        Ok(status) if status.success() => Ok(()),
        Ok(status) => Err(StrategyError::Runtime(format!(
            "{} exited with code {}",
            program,
            status.code().unwrap_or(-1)
        ))),
    }
}

#[cfg(unix)]
fn try_installer_curl() -> std::result::Result<(), StrategyError> {
    if !tool_exists("curl") {
        return Err(StrategyError::Preflight("curl not on PATH".into()));
    }
    let script = format!(
        "set -e; set -o pipefail; curl --proto '=https' --tlsv1.2 -fLsS {} | sh",
        SHELL_INSTALLER
    );
    run_command_status("sh", &["-c", &script])
}

#[cfg(not(unix))]
fn try_installer_curl() -> std::result::Result<(), StrategyError> {
    Err(StrategyError::Preflight(
        "curl installer is Unix-only".into(),
    ))
}

#[cfg(unix)]
fn try_installer_wget() -> std::result::Result<(), StrategyError> {
    if !tool_exists("wget") {
        return Err(StrategyError::Preflight("wget not on PATH".into()));
    }
    let script = format!(
        "set -e; set -o pipefail; wget -qO- {} | sh",
        SHELL_INSTALLER
    );
    run_command_status("sh", &["-c", &script])
}

#[cfg(not(unix))]
fn try_installer_wget() -> std::result::Result<(), StrategyError> {
    Err(StrategyError::Preflight(
        "wget installer is Unix-only".into(),
    ))
}

#[cfg(windows)]
fn try_installer_powershell(program: &str) -> std::result::Result<(), StrategyError> {
    let script = format!("$ErrorActionPreference='Stop'; irm {} | iex", PS_INSTALLER);
    run_command_status(
        program,
        &[
            "-NoProfile",
            "-NonInteractive",
            "-ExecutionPolicy",
            "Bypass",
            "-Command",
            &script,
        ],
    )
}

#[cfg(not(windows))]
fn try_installer_powershell(_program: &str) -> std::result::Result<(), StrategyError> {
    Err(StrategyError::Preflight(
        "PowerShell installer is Windows-only".into(),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unix_with_cargo_orders_cargo_then_installers() {
        assert_eq!(
            order_strategies(true, TargetOs::Unix),
            vec![
                UpdateStrategy::Cargo,
                UpdateStrategy::InstallerCurl,
                UpdateStrategy::InstallerWget,
            ]
        );
    }

    #[test]
    fn unix_without_cargo_prunes_cargo() {
        assert_eq!(
            order_strategies(false, TargetOs::Unix),
            vec![UpdateStrategy::InstallerCurl, UpdateStrategy::InstallerWget]
        );
    }

    #[test]
    fn windows_without_cargo_uses_powershell_then_pwsh() {
        assert_eq!(
            order_strategies(false, TargetOs::Windows),
            vec![
                UpdateStrategy::InstallerPowerShell,
                UpdateStrategy::InstallerPwsh,
            ]
        );
    }

    #[test]
    fn compares_semver_segments() {
        assert!(is_newer("1.3.0", "1.3.1"));
        assert!(is_newer("1.3", "1.3.1"));
        assert!(!is_newer("1.3.1", "1.3.0"));
        assert!(!is_newer("1.3.1", "1.3.1"));
    }

    #[test]
    fn json_method_preserves_legacy_taxonomy() {
        assert_eq!(UpdateStrategy::Cargo.json_method(), "cargo");
        assert_eq!(UpdateStrategy::InstallerCurl.json_method(), "installer");
        assert_eq!(
            UpdateStrategy::InstallerPowerShell.json_method(),
            "installer"
        );
    }

    #[test]
    fn cargo_strategy_uses_publish_package_not_binary_name() {
        assert_eq!(CRATE_NAME, "tr300-tui");
        assert_ne!(CRATE_NAME, "sd300");
    }

    #[cfg(not(windows))]
    #[test]
    fn shell_installer_uses_publish_package_asset_name() {
        assert!(SHELL_INSTALLER.ends_with("/tr300-tui-installer.sh"));
        assert!(!SHELL_INSTALLER.ends_with("/sd300-installer.sh"));
    }

    #[cfg(windows)]
    #[test]
    fn powershell_installer_uses_publish_package_asset_name() {
        assert!(PS_INSTALLER.ends_with("/tr300-tui-installer.ps1"));
        assert!(!PS_INSTALLER.ends_with("/sd300-installer.ps1"));
    }
}
