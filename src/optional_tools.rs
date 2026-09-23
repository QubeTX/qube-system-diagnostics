//! Explicit setup of independently owned optional tools. No collector installs software.
use crate::collectors::command::{self, CommandTimeout};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex,
};
use std::time::Duration;

pub const NETWORK_NOTICE: &str = "Install official ND-300 4.0.1 and SpeedQX for optional network checks. Download one checksum-pinned release archive from QubeTX on GitHub, verify both executables, then place them in an independent per-user ND-300 directory. Existing installations are preserved. No administrator access, PATH changes, network repairs, diagnostics or bandwidth tests are requested. ND-300 remains installed when SD-300 is removed.";

pub fn run_cli(args: &crate::cli::OptionalToolArgs) -> i32 {
    let (name, notice, destination) = match args.tool {
        crate::cli::OptionalTool::Nd300 => ("nd300", NETWORK_NOTICE, network_directory()),
        crate::cli::OptionalTool::Smartctl => (
            "smartctl",
            crate::smart_setup::notice(),
            crate::smart_setup::directory(),
        ),
    };
    let result = if args.install {
        match args.tool {
            crate::cli::OptionalTool::Nd300 => {
                install_network(args.accept, &AtomicBool::new(false))
            }
            crate::cli::OptionalTool::Smartctl => {
                crate::smart_setup::install(args.accept, &AtomicBool::new(false))
            }
        }
    } else {
        Ok(format!("{notice}\nStandalone destination (when applicable): {}\nExisting executable: {}\nTo accept: sd300 tools {name} --install --accept",
            destination.map(|p| p.display().to_string()).unwrap_or_else(|e| e),
            detect(name).map(|p| p.display().to_string()).unwrap_or_else(|| "not found".into())))
    };
    let success = result.is_ok();
    let message = match result {
        Ok(message) | Err(message) => message,
    };
    if args.json {
        println!(
            "{}",
            serde_json::json!({"tool":name, "success":success, "installed":success && args.install, "message":message})
        );
    } else {
        println!("{message}");
    }
    if success {
        0
    } else {
        1
    }
}

#[derive(Debug, Clone, Copy)]
struct Archive {
    name: &'static str,
    sha256: &'static str,
}
fn archive_for(os: &str, arch: &str, musl: bool) -> Option<Archive> {
    let (name, sha256) = match (os, arch, musl) {
        ("windows", "x86_64", _) => (
            "nd300-x86_64-pc-windows-msvc.zip",
            "2b188f060a81687ca120444e88c4e63744d02841b2a17d18956e2c345c967603",
        ),
        ("macos", "x86_64", _) => (
            "nd300-x86_64-apple-darwin.tar.xz",
            "ec15ef716ac62220183fb33e1c61fb8067e792d1dc3447a855f9b76dbfcf75e5",
        ),
        ("macos", "aarch64", _) => (
            "nd300-aarch64-apple-darwin.tar.xz",
            "145086fc5d8a48a740c4122daf1deadcdd240995a4d724de2c1e3d436e250374",
        ),
        ("linux", "x86_64", false) => (
            "nd300-x86_64-unknown-linux-gnu.tar.xz",
            "95a082bdcf6fe27ce6540b4b55e14054133414e8a9df7b5e5c42b53c1bb067f1",
        ),
        ("linux", "aarch64", false) => (
            "nd300-aarch64-unknown-linux-gnu.tar.xz",
            "1f579931aaaa9e62862872cfadda27285cd9779bcb3f0e123d8dc7f6cc3007ed",
        ),
        ("linux", "x86_64", true) => (
            "nd300-x86_64-unknown-linux-musl.tar.xz",
            "236c0d67415db0f6982728f2263b96f68a94060dda99f96cdb6ff453ab6a8ace",
        ),
        _ => return None,
    };
    Some(Archive { name, sha256 })
}

fn home() -> Option<PathBuf> {
    std::env::var_os(if cfg!(windows) { "USERPROFILE" } else { "HOME" })
        .map(PathBuf::from)
        .filter(|p| p.is_absolute())
}
pub fn network_directory() -> Result<PathBuf, String> {
    #[cfg(windows)]
    let root = std::env::var_os("LOCALAPPDATA").map(PathBuf::from);
    #[cfg(target_os = "macos")]
    let root = home().map(|p| p.join("Library/Application Support"));
    #[cfg(not(any(windows, target_os = "macos")))]
    let root = std::env::var_os("XDG_DATA_HOME")
        .map(PathBuf::from)
        .or_else(|| home().map(|p| p.join(".local/share")));
    root.filter(|p| p.is_absolute())
        .map(|p| p.join("nd300/standalone-4.0.1"))
        .ok_or_else(|| "The per-user application directory is unavailable".into())
}

pub fn executable_name(name: &str) -> String {
    format!("{name}{}", std::env::consts::EXE_SUFFIX)
}

/// Explicit absolute locations only: neither Windows' implicit current-directory
/// search nor a relative PATH entry can choose a provider.
pub fn candidates(name: &str) -> Vec<PathBuf> {
    let mut dirs: Vec<PathBuf> =
        std::env::split_paths(&std::env::var_os("PATH").unwrap_or_default())
            .filter(|p| p.is_absolute())
            .collect();
    if let Some(cargo) = std::env::var_os("CARGO_HOME")
        .map(PathBuf::from)
        .filter(|p| p.is_absolute())
    {
        dirs.push(cargo.join("bin"));
    }
    if let Some(home) = home() {
        dirs.extend([home.join(".cargo/bin"), home.join(".local/bin")]);
    }
    if let Ok(path) = network_directory() {
        dirs.push(path);
    }
    if let Ok(path) = crate::smart_setup::directory() {
        dirs.push(path);
    }
    #[cfg(windows)]
    for key in ["ProgramFiles", "ProgramFiles(x86)", "LOCALAPPDATA"] {
        if let Some(root) = std::env::var_os(key)
            .map(PathBuf::from)
            .filter(|p| p.is_absolute())
        {
            for suffix in [
                "nd300/bin",
                "nd300",
                "ND-300/bin",
                "smartmontools/bin",
                "Programs/nd300/bin",
            ] {
                dirs.push(root.join(suffix));
            }
        }
    }
    #[cfg(unix)]
    dirs.extend(
        [
            "/usr/local/bin",
            "/usr/local/sbin",
            "/usr/bin",
            "/usr/sbin",
            "/opt/homebrew/bin",
            "/opt/homebrew/sbin",
        ]
        .map(PathBuf::from),
    );
    let name = executable_name(name);
    let mut paths = Vec::new();
    for dir in dirs {
        let path = dir.join(&name);
        if !paths.contains(&path) {
            paths.push(path);
        }
    }
    paths
}

pub fn detect(name: &str) -> Option<PathBuf> {
    let settings = crate::settings::shared_preferences();
    let choice = match name {
        "nd300" => settings.nd300_path,
        "speedqx" => settings
            .nd300_path
            .and_then(|p| p.parent().map(|p| p.join(executable_name(name)))),
        "smartctl" => settings.smartctl_path,
        _ => None,
    };
    // An explicit choice must never silently fall back to a different binary.
    choice.or_else(|| candidates(name).into_iter().find(|p| p.is_file()))
}

fn existing_network_owner() -> Result<Option<String>, String> {
    for name in ["nd300", "speedqx"] {
        if let Some(path) = detect(name) {
            return Ok(Some(path.display().to_string()));
        }
    }
    // Receipts survive an incomplete/unavailable executable. Do not repair or
    // overwrite such an installation behind its owner's back.
    let mut receipts = Vec::new();
    if let Some(home) = home() {
        receipts.extend([
            home.join(".config/nd300/nd300-receipt.json"),
            home.join(".cargo/.crates2.json"),
            home.join(".cargo/.crates.toml"),
        ]);
    }
    for key in ["LOCALAPPDATA", "XDG_CONFIG_HOME"] {
        if let Some(root) = std::env::var_os(key)
            .map(PathBuf::from)
            .filter(|p| p.is_absolute())
        {
            receipts.push(root.join("nd300/nd300-receipt.json"));
        }
    }
    if let Some(root) = std::env::var_os("CARGO_HOME")
        .map(PathBuf::from)
        .filter(|p| p.is_absolute())
    {
        receipts.extend([root.join(".crates2.json"), root.join(".crates.toml")]);
    }
    for path in receipts {
        match fs::metadata(&path) {
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => continue,
            Err(_) => {
                return Err("Could not inspect optional-tool ownership; no files changed".into())
            }
            Ok(meta) if meta.len() > 8 * 1024 * 1024 => {
                return Err(
                    "An install receipt exceeds the inspection limit; no files changed".into(),
                )
            }
            Ok(_) => {}
        }
        let bytes = fs::read(&path).map_err(|_| "Could not read optional-tool ownership")?;
        if path.file_name().is_some_and(|p| p == "nd300-receipt.json")
            || String::from_utf8_lossy(&bytes).contains("nd300 ")
        {
            return Ok(Some(path.display().to_string()));
        }
    }
    #[cfg(windows)]
    {
        use winreg::{enums::*, RegKey};
        for (hive, view) in [
            (HKEY_CURRENT_USER, KEY_WOW64_64KEY),
            (HKEY_LOCAL_MACHINE, KEY_WOW64_64KEY),
            (HKEY_LOCAL_MACHINE, KEY_WOW64_32KEY),
        ] {
            let root = match RegKey::predef(hive).open_subkey_with_flags(
                "Software\\Microsoft\\Windows\\CurrentVersion\\Uninstall",
                KEY_READ | view,
            ) {
                Ok(root) => root,
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => continue,
                Err(_) => {
                    return Err(
                        "Could not inspect registered ND-300 owners; no files changed".into(),
                    )
                }
            };
            for key in root.enum_keys() {
                let key = key.map_err(|_| "Could not enumerate installation owners")?;
                let record = root
                    .open_subkey(&key)
                    .map_err(|_| "Could not inspect an installation owner")?;
                let display: String = record.get_value("DisplayName").unwrap_or_default();
                if display.to_ascii_lowercase().starts_with("nd300")
                    || display.to_ascii_lowercase().starts_with("nd-300")
                {
                    return Ok(Some(display));
                }
            }
        }
    }
    Ok(None)
}

pub(crate) fn system_tool(name: &str) -> Result<PathBuf, String> {
    #[cfg(windows)]
    let paths = std::env::var_os("SystemRoot")
        .map(PathBuf::from)
        .filter(|p| p.is_absolute())
        .map(|p| vec![p.join("System32").join(executable_name(name))])
        .unwrap_or_default();
    #[cfg(unix)]
    let paths = vec![
        PathBuf::from("/usr/bin").join(name),
        PathBuf::from("/bin").join(name),
    ];
    paths.into_iter().find(|p| p.is_file()).ok_or_else(|| format!("{name} is required for optional setup; install it through your operating system and retry"))
}

pub(crate) fn successful(stage: &str, output: command::MemoryCapture) -> Result<Vec<u8>, String> {
    if let Some(error) = output.failure {
        return Err(format!("{stage}: {error}"));
    }
    if !output.status.is_some_and(|s| s.success()) {
        return Err(format!(
            "{stage} failed (exit {:?}); no diagnostic or repair was started",
            output.status.and_then(|s| s.code())
        ));
    }
    Ok(output.stdout)
}

fn verify_digest(bytes: &[u8], expected: &str) -> Result<(), String> {
    if format!("{:x}", Sha256::digest(bytes)) != expected {
        return Err("Release checksum mismatch; nothing was installed".into());
    }
    Ok(())
}

pub(crate) fn download_verified(
    url: &str,
    sha256: &str,
    cancel: &AtomicBool,
) -> Result<Vec<u8>, String> {
    let bytes = successful(
        "Official release download",
        command::run_memory(
            system_tool("curl")?,
            [
                "--disable",
                "--proto",
                "=https",
                "--proto-redir",
                "=https",
                "--tlsv1.2",
                "--fail",
                "--location",
                "--silent",
                "--show-error",
                "--max-time",
                "120",
                "--max-filesize",
                "8388608",
                "--",
                url,
            ],
            CommandTimeout::Custom(Duration::from_secs(125)),
            cancel,
        )
        .map_err(|e| e.to_string())?,
    )?;
    verify_digest(&bytes, sha256)?;
    Ok(bytes)
}

/// Consent is checked before discovery, network activity or filesystem changes.
pub fn install_network(consent: bool, cancel: &AtomicBool) -> Result<String, String> {
    if !consent {
        return Err("Installation declined; monitoring remains available".into());
    }
    if cancel.load(Ordering::Acquire) {
        return Err("Installation cancelled".into());
    }
    if let Some(owner) = existing_network_owner()? {
        return Err(format!("An ND-300 installation or receipt already exists ({owner}). Use that installation's own update or repair process; SD-300 has preserved it."));
    }
    let archive = archive_for(
        std::env::consts::OS,
        std::env::consts::ARCH,
        cfg!(target_env = "musl"),
    )
    .ok_or("No verified ND-300 archive is available for this platform")?;
    let destination = network_directory()?;
    if destination.exists() {
        return Err("The ND-300 destination already exists; no files changed".into());
    }
    let url = format!(
        "https://github.com/QubeTX/qube-network-diagnostics/releases/download/v4.0.1/{}",
        archive.name
    );
    let bytes = download_verified(&url, archive.sha256, cancel)?;
    let staging = tempfile::tempdir().map_err(|e| e.to_string())?;
    let archive_path = staging.path().join(archive.name);
    fs::write(&archive_path, bytes).map_err(|e| e.to_string())?;
    let extracted = staging.path().join("extracted");
    fs::create_dir(&extracted).map_err(|e| e.to_string())?;
    let mut extract = std::process::Command::new(system_tool("tar")?);
    extract.env_remove("TAR_OPTIONS").args([
        std::ffi::OsStr::new("-xf"),
        archive_path.as_os_str(),
        std::ffi::OsStr::new("-C"),
        extracted.as_os_str(),
    ]);
    successful(
        "Verified archive extraction",
        command::run_memory_command(
            &mut extract,
            CommandTimeout::Custom(Duration::from_secs(20)),
            cancel,
        )
        .map_err(|e| e.to_string())?,
    )?;
    // cargo-dist ZIPs are flat; tar archives have one product directory.
    let archive_root = archive
        .name
        .trim_end_matches(".tar.xz")
        .trim_end_matches(".zip");
    let source = if extracted.join(executable_name("nd300")).is_file() {
        extracted
    } else {
        extracted.join(archive_root)
    };
    for (name, speed) in [("nd300", false), ("speedqx", true)] {
        crate::companion::verify_executable(&source.join(executable_name(name)), speed, cancel)
            .map_err(|e| e.explanation().to_string())?;
    }
    if cancel.load(Ordering::Acquire) {
        return Err("Installation cancelled before activation".into());
    }
    if existing_network_owner()?.is_some() {
        return Err("An ND-300 owner appeared during setup; no files changed".into());
    }
    install_verified_files(&source, &destination, archive.sha256)?;
    for (name, speed) in [("nd300", false), ("speedqx", true)] {
        crate::companion::verify_executable(
            &destination.join(executable_name(name)),
            speed,
            cancel,
        )
        .map_err(|e| {
            format!(
                "Files installed but final verification failed: {}",
                e.explanation()
            )
        })?;
    }
    // This selection is deliberately shared. A failed settings write leaves a
    // complete, independently usable ND-300 installation, never partial files.
    crate::settings::select_network_companion(destination.join(executable_name("nd300")))?;
    Ok(format!("Verified ND-300 4.0.1 and SpeedQX installed at {}. They remain independent of SD-300. Choose a diagnostic explicitly to run it.", destination.display()))
}

fn install_verified_files(source: &Path, destination: &Path, digest: &str) -> Result<(), String> {
    fs::create_dir_all(destination.parent().ok_or("Missing installation parent")?)
        .map_err(|e| e.to_string())?;
    // Atomic reservation: never replace an existing directory, even if empty.
    fs::create_dir(destination)
        .map_err(|e| format!("Could not reserve ND-300 destination: {e}"))?;
    let mut created = Vec::new();
    let result = (|| {
        for name in [
            executable_name("nd300"),
            executable_name("speedqx"),
            "LICENSE".into(),
            "README.md".into(),
        ] {
            let from = source.join(&name);
            let to = destination.join(&name);
            let mut file = fs::File::create_new(&to).map_err(|e| e.to_string())?;
            created.push(to.clone());
            let mut input = fs::File::open(&from).map_err(|e| e.to_string())?;
            std::io::copy(&mut input, &mut file).map_err(|e| e.to_string())?;
            file.sync_all().map_err(|e| e.to_string())?;
            fs::set_permissions(
                &to,
                fs::metadata(&from)
                    .map_err(|e| e.to_string())?
                    .permissions(),
            )
            .map_err(|e| e.to_string())?;
        }
        let receipt = destination.join("standalone-receipt.json");
        let mut file = fs::File::create_new(&receipt).map_err(|e| e.to_string())?;
        created.push(receipt);
        file.write_all(serde_json::json!({"product":"ND-300", "version":"4.0.1", "distribution":"official-release-archive", "archive_sha256":digest, "lifecycle":"independent", "removal":"Remove this standalone directory when no ND-300 process is running; SD-300 uninstall preserves it"}).to_string().as_bytes()).map_err(|e| e.to_string())?;
        file.sync_all().map_err(|e| e.to_string())
    })();
    if result.is_err() {
        // Only exact files created by this attempt. Never recursive cleanup.
        for path in created.iter().rev() {
            let _ = fs::remove_file(path);
        }
        let _ = fs::remove_dir(destination);
    }
    result
}

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct State {
    pub sequence: u64,
    pub running: bool,
    pub message: String,
    pub succeeded: bool,
    pub tool: String,
}
#[derive(Default)]
pub struct Controller {
    pub state: State,
    cancel: Arc<AtomicBool>,
    complete: Arc<Mutex<Option<Result<String, String>>>>,
    worker: Option<std::thread::JoinHandle<()>>,
}
impl Controller {
    pub fn start_network(&mut self, consent: bool) -> bool {
        self.start_tool(crate::cli::OptionalTool::Nd300, consent)
    }
    pub fn start_smart(&mut self, consent: bool) -> bool {
        self.start_tool(crate::cli::OptionalTool::Smartctl, consent)
    }
    fn start_tool(&mut self, tool: crate::cli::OptionalTool, consent: bool) -> bool {
        self.poll();
        if self.worker.is_some() {
            return false;
        }
        if !consent {
            self.state.message = "Installation declined; monitoring remains available".into();
            self.state.sequence += 1;
            return false;
        }
        self.cancel.store(false, Ordering::Release);
        self.state.running = true;
        self.state.succeeded = false;
        self.state.tool = match tool {
            crate::cli::OptionalTool::Nd300 => "nd300",
            crate::cli::OptionalTool::Smartctl => "smartctl",
        }
        .into();
        self.state.message = "Checking existing ownership, downloading and verifying the official release. Cancel is available before activation.".into();
        self.state.sequence += 1;
        let cancel = self.cancel.clone();
        let complete = self.complete.clone();
        match std::thread::Builder::new()
            .name("sd300-optional-setup".into())
            .spawn(move || {
                let result = std::panic::catch_unwind(|| match tool {
                    crate::cli::OptionalTool::Nd300 => install_network(consent, &cancel),
                    crate::cli::OptionalTool::Smartctl => {
                        crate::smart_setup::install(consent, &cancel)
                    }
                })
                .unwrap_or_else(|_| Err("Optional setup worker failed".into()));
                if let Ok(mut slot) = complete.lock() {
                    *slot = Some(result);
                }
            }) {
            Ok(worker) => {
                self.worker = Some(worker);
                true
            }
            Err(_) => {
                self.state.running = false;
                self.state.message = "Could not start the optional setup worker".into();
                false
            }
        }
    }
    pub fn cancel(&self) {
        self.cancel.store(true, Ordering::Release);
    }
    pub fn poll(&mut self) -> bool {
        if !self.worker.as_ref().is_some_and(|w| w.is_finished()) {
            return false;
        }
        let _ = self.worker.take().unwrap().join();
        self.state.running = false;
        self.state.message = match self.complete.lock().ok().and_then(|mut s| s.take()) {
            Some(Ok(message)) => {
                self.state.succeeded = true;
                message
            }
            Some(Err(message)) => {
                self.state.succeeded = false;
                message
            }
            None => "Optional setup failed without a result".into(),
        };
        self.state.sequence += 1;
        true
    }
}
impl Drop for Controller {
    fn drop(&mut self) {
        self.cancel();
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn refusal_and_cancellation_do_not_start_setup() {
        assert!(install_network(false, &AtomicBool::new(false))
            .unwrap_err()
            .contains("declined"));
        assert!(install_network(true, &AtomicBool::new(true))
            .unwrap_err()
            .contains("cancelled"));
        let mut controller = Controller::default();
        assert!(!controller.start_network(false));
        assert!(!controller.state.running);
        assert!(controller.worker.is_none());
    }
    #[test]
    fn all_six_archives_are_pinned_and_unknown_platforms_rejected() {
        for (os, arch, musl) in [
            ("windows", "x86_64", false),
            ("macos", "x86_64", false),
            ("macos", "aarch64", false),
            ("linux", "x86_64", false),
            ("linux", "aarch64", false),
            ("linux", "x86_64", true),
        ] {
            let archive = archive_for(os, arch, musl).unwrap();
            assert_eq!(archive.sha256.len(), 64);
            assert!(verify_digest(b"wrong release", archive.sha256).is_err());
        }
        assert!(archive_for("windows", "aarch64", false).is_none());
        assert!(archive_for("linux", "aarch64", true).is_none());
    }
    #[test]
    fn installation_reserves_destination_rolls_back_only_owned_files_and_retains_license() {
        let temp = tempfile::tempdir().unwrap();
        let source = temp.path().join("source");
        fs::create_dir(&source).unwrap();
        let destination = temp.path().join("nd300");
        fs::write(source.join(executable_name("nd300")), b"fixture").unwrap();
        assert!(install_verified_files(&source, &destination, "fixture").is_err());
        assert!(!destination.exists());
        for name in [
            executable_name("speedqx"),
            "LICENSE".into(),
            "README.md".into(),
        ] {
            fs::write(source.join(name), b"fixture").unwrap();
        }
        install_verified_files(&source, &destination, "fixture").unwrap();
        fs::write(destination.join("user-export.json"), b"preserve").unwrap();
        assert!(install_verified_files(&source, &destination, "other").is_err());
        assert_eq!(
            fs::read(destination.join("user-export.json")).unwrap(),
            b"preserve"
        );
        assert!(destination.join("LICENSE").is_file());
        assert!(destination.join("standalone-receipt.json").is_file());
    }
}
