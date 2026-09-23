//! Optional SMART helper setup. Ordinary collection never calls this module.
use crate::collectors::command::{self, CommandTimeout};
use crate::optional_tools::{self, successful};
#[cfg(any(windows, target_os = "linux"))]
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

pub fn notice() -> &'static str {
    if cfg!(windows) {
        "Install smartctl for optional storage-health reads. Download the official smartmontools 7.5 installer, verify its pinned checksum, and extract only the 64-bit smartctl, drive database and documentation into a private staging directory without elevation. Copy the read-only helper into an independent per-user smartmontools directory. Existing installations are preserved. No background service, PATH change, self-test or disk setting change is requested. Privileged reads require a separate confirmation."
    } else if cfg!(target_os = "macos") {
        "Install smartmontools through the existing Homebrew installation using brew install --formula smartmontools, with automatic Homebrew updates and cleanup disabled. Homebrew retains ownership. No service is started and no self-test or disk setting change is requested. Privileged reads require a separate confirmation."
    } else {
        "Download smartmontools through the configured Debian/Ubuntu or Alpine package repository, verify package integrity, extract its smartctl binary and documentation into an independent per-user directory, and verify the executable. Package scripts are not run and the system package database is unchanged. Existing installations are preserved. No service, self-test or disk setting change is requested. Privileged reads require a separate confirmation. Other distributions can use their own smartmontools package."
    }
}

pub fn directory() -> Result<PathBuf, String> {
    let nd = optional_tools::network_directory()?;
    let root = nd
        .parent()
        .and_then(Path::parent)
        .ok_or("Application directory is missing")?;
    Ok(root.join("smartmontools/standalone"))
}

pub fn verify(path: &Path, cancel: &AtomicBool) -> Result<String, String> {
    let bytes = successful(
        "SMART helper verification",
        command::run_memory(path, ["--json", "--version"], CommandTimeout::Slow, cancel)
            .map_err(|e| e.to_string())?,
    )?;
    verify_version_json(&bytes)
}
fn verify_version_json(bytes: &[u8]) -> Result<String, String> {
    let value: serde_json::Value =
        serde_json::from_slice(bytes).map_err(|_| "The helper did not return smartctl JSON")?;
    let major = value
        .pointer("/smartctl/version/0")
        .and_then(|v| v.as_u64());
    let minor = value
        .pointer("/smartctl/version/1")
        .and_then(|v| v.as_u64());
    if value
        .pointer("/json_format_version/0")
        .and_then(|v| v.as_u64())
        != Some(1)
        || major != Some(7)
        || minor.is_none()
        || value
            .pointer("/smartctl/exit_status")
            .and_then(|v| v.as_u64())
            != Some(0)
    {
        return Err("The helper is not a supported smartctl 7.x JSON interface".into());
    }
    Ok(format!("7.{}", minor.unwrap()))
}

pub fn install(consent: bool, cancel: &AtomicBool) -> Result<String, String> {
    if !consent {
        return Err(
            "SMART helper installation declined; ordinary storage monitoring remains available"
                .into(),
        );
    }
    if cancel.load(Ordering::Acquire) {
        return Err("SMART helper setup cancelled".into());
    }
    if let Some(path) = optional_tools::detect("smartctl") {
        return Err(format!("An existing smartctl is selected or installed at {}. Its owner is preserved; use its own update or repair process.", path.display()));
    }
    let executable = install_platform(cancel)?;
    let version = verify(&executable, cancel).map_err(|e| {
        format!("SMART helper files were installed, but final verification is incomplete: {e}")
    })?;
    crate::settings::select_smartctl(executable.clone()).map_err(|e| format!("Verified smartctl is installed at {}, but the provider preference could not be saved: {e}", executable.display()))?;
    Ok(format!("Verified smartctl {version} at {}. Retry storage collection to use it. Device access may still require a separately confirmed privileged read.", executable.display()))
}

#[cfg(windows)]
fn install_platform(cancel: &AtomicBool) -> Result<PathBuf, String> {
    use winreg::{enums::*, RegKey};
    for view in [KEY_WOW64_64KEY, KEY_WOW64_32KEY] {
        match RegKey::predef(HKEY_LOCAL_MACHINE).open_subkey_with_flags(
            "Software\\Microsoft\\Windows\\CurrentVersion\\Uninstall\\smartmontools",
            KEY_READ | view,
        ) {
            Ok(_) => return Err(
                "A registered smartmontools installation exists; use its owner's repair operation"
                    .into(),
            ),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(_) => {
                return Err("Could not inspect smartmontools ownership; nothing changed".into())
            }
        }
    }
    let destination = directory()?;
    if destination.exists() {
        return Err(
            "The standalone smartmontools directory already exists; it was preserved".into(),
        );
    }
    let bytes = optional_tools::download_verified("https://github.com/smartmontools/smartmontools/releases/download/RELEASE_7_5/smartmontools-7.5.win32-setup.exe",
        "896337fcc253220614cf8cdbd5cf2321c5aa326a37a04160a672a281e6104c70", cancel)?;
    let staging = tempfile::tempdir().map_err(|e| e.to_string())?;
    let installer = staging.path().join("smartmontools-7.5.win32-setup.exe");
    fs::write(&installer, bytes).map_err(|e| e.to_string())?;
    let extracted = staging.path().join("extracted");
    // The reviewed NSIS source makes all registration/service/PATH actions
    // separate components. RunAsInvoker retains the caller's token; it grants
    // no privilege. /D is the final NSIS argument and intentionally unquoted.
    let mut command = Command::new(&installer);
    use std::os::windows::process::CommandExt;
    command
        .env("__COMPAT_LAYER", "RunAsInvoker")
        .args(["/S", "/SO", "x64,smartctl,drivedb,doc"])
        .raw_arg(format!("/D={}", extracted.display()));
    successful(
        "SMART helper file extraction",
        command::run_memory_command(
            &mut command,
            CommandTimeout::Custom(Duration::from_secs(45)),
            cancel,
        )
        .map_err(|e| e.to_string())?,
    )?;
    verify(&extracted.join("bin/smartctl.exe"), cancel)?;
    activate(
        &extracted,
        &destination,
        &[
            ("bin/smartctl.exe", "smartctl.exe"),
            ("bin/drivedb.h", "drivedb.h"),
            ("doc/COPYING.txt", "COPYING.txt"),
            ("doc/README.txt", "README.txt"),
        ],
        cancel,
    )?;
    Ok(destination.join("smartctl.exe"))
}

#[cfg(target_os = "macos")]
fn install_platform(cancel: &AtomicBool) -> Result<PathBuf, String> {
    let brew = ["/opt/homebrew/bin/brew", "/usr/local/bin/brew"].into_iter().map(PathBuf::from)
        .find(|p| p.is_file()).ok_or("Homebrew is unavailable. Install smartmontools through its official macOS distribution or an existing package manager, then retry discovery")?;
    let existing = command::run_memory(
        &brew,
        ["list", "--versions", "smartmontools"],
        CommandTimeout::Custom(Duration::from_secs(20)),
        cancel,
    )
    .map_err(|e| e.to_string())?;
    if let Some(error) = existing.failure {
        return Err(error.to_string());
    }
    if !existing.stdout.is_empty() {
        return Err(
            "Homebrew already owns smartmontools; use brew to repair that installation".into(),
        );
    }
    let mut command = Command::new(&brew);
    command
        .args(["install", "--formula", "smartmontools"])
        .env("HOMEBREW_NO_AUTO_UPDATE", "1")
        .env("HOMEBREW_NO_INSTALL_CLEANUP", "1")
        .env("HOMEBREW_NO_ANALYTICS", "1");
    successful(
        "Homebrew smartmontools installation",
        command::run_memory_command(
            &mut command,
            CommandTimeout::Custom(Duration::from_secs(300)),
            cancel,
        )
        .map_err(|e| e.to_string())?,
    )?;
    optional_tools::detect("smartctl").ok_or_else(|| "Homebrew completed but smartctl could not be found; inspect brew's installation before retrying".into())
}

#[cfg(target_os = "linux")]
fn install_platform(cancel: &AtomicBool) -> Result<PathBuf, String> {
    let destination = directory()?;
    if destination.exists() {
        return Err(
            "The standalone smartmontools directory already exists; it was preserved".into(),
        );
    }
    let staging = tempfile::tempdir().map_err(|e| e.to_string())?;
    let extracted = staging.path().join("extracted");
    fs::create_dir(&extracted).map_err(|e| e.to_string())?;
    if Path::new("/usr/bin/apt-get").is_file() && Path::new("/usr/bin/dpkg-deb").is_file() {
        let mut download = Command::new("/usr/bin/apt-get");
        download.current_dir(staging.path()).args([
            "-o",
            "APT::Get::AllowUnauthenticated=false",
            "download",
            "smartmontools",
        ]);
        successful(
            "Authenticated smartmontools package download",
            command::run_memory_command(
                &mut download,
                CommandTimeout::Custom(Duration::from_secs(120)),
                cancel,
            )
            .map_err(|e| e.to_string())?,
        )?;
        let packages = fs::read_dir(staging.path())
            .map_err(|e| e.to_string())?
            .filter_map(Result::ok)
            .map(|e| e.path())
            .filter(|p| p.extension().is_some_and(|s| s == "deb"))
            .collect::<Vec<_>>();
        if packages.len() != 1 {
            return Err(
                "The package manager did not return exactly one smartmontools package".into(),
            );
        }
        successful(
            "SMART package extraction",
            command::run_memory(
                "/usr/bin/dpkg-deb",
                [
                    std::ffi::OsStr::new("--extract"),
                    packages[0].as_os_str(),
                    extracted.as_os_str(),
                ],
                CommandTimeout::Custom(Duration::from_secs(20)),
                cancel,
            )
            .map_err(|e| e.to_string())?,
        )?;
    } else if Path::new("/sbin/apk").is_file() {
        // Clean Alpine hosts may have no cached indexes. Fetch signed indexes
        // in memory; never update the system cache or installed database.
        let bytes = successful(
            "Smartmontools package download",
            command::run_memory(
                "/sbin/apk",
                [
                    "--no-cache",
                    "--quiet",
                    "fetch",
                    "--stdout",
                    "smartmontools",
                ],
                CommandTimeout::Custom(Duration::from_secs(120)),
                cancel,
            )
            .map_err(|e| e.to_string())?,
        )?;
        let package = staging.path().join("smartmontools.apk");
        fs::write(&package, bytes).map_err(|e| e.to_string())?;
        successful(
            "Alpine package signature verification",
            command::run_memory(
                "/sbin/apk",
                [std::ffi::OsStr::new("verify"), package.as_os_str()],
                CommandTimeout::Slow,
                cancel,
            )
            .map_err(|e| e.to_string())?,
        )?;
        let mut extract = Command::new(optional_tools::system_tool("tar")?);
        extract
            .env_remove("TAR_OPTIONS")
            .arg("--ignore-zeros")
            .arg("-xf")
            .arg(package)
            .arg("-C")
            .arg(&extracted);
        successful(
            "SMART package extraction",
            command::run_memory_command(
                &mut extract,
                CommandTimeout::Custom(Duration::from_secs(20)),
                cancel,
            )
            .map_err(|e| e.to_string())?,
        )?;
    } else {
        return Err("Automatic helper setup supports Debian/Ubuntu and Alpine package repositories. Install smartmontools through this distribution's package manager, then retry discovery".into());
    }
    let relative = ["usr/sbin/smartctl", "usr/bin/smartctl"]
        .into_iter()
        .find(|p| extracted.join(p).is_file())
        .ok_or("The package has no supported smartctl executable")?;
    verify(&extracted.join(relative), cancel).map_err(|e| format!("Extracted smartctl is not usable: {e}. Its runtime libraries may be absent; install smartmontools through the distribution's package manager to resolve dependencies."))?;
    let mut files = vec![(relative, "smartctl")];
    for (from, to) in [
        ("usr/share/smartmontools/drivedb.h", "drivedb.h"),
        ("var/lib/smartmontools/drivedb/drivedb.h", "drivedb.h"),
        ("usr/share/doc/smartmontools/copyright", "COPYRIGHT"),
        ("usr/share/licenses/smartmontools/COPYING", "COPYING"),
    ] {
        if extracted.join(from).is_file() && !files.iter().any(|(_, name)| *name == to) {
            files.push((from, to));
        }
    }
    activate(&extracted, &destination, &files, cancel)?;
    Ok(destination.join("smartctl"))
}

#[cfg(any(windows, target_os = "linux"))]
fn activate(
    source: &Path,
    destination: &Path,
    files: &[(&str, &str)],
    cancel: &AtomicBool,
) -> Result<(), String> {
    if cancel.load(Ordering::Acquire) {
        return Err("SMART helper setup cancelled before activation".into());
    }
    fs::create_dir_all(destination.parent().ok_or("Missing helper directory")?)
        .map_err(|e| e.to_string())?;
    fs::create_dir(destination).map_err(|e| e.to_string())?;
    let mut created = Vec::new();
    let result = (|| {
        for (from, to) in files {
            let from = source.join(from);
            let to = destination.join(to);
            let mut output = fs::File::create_new(&to).map_err(|e| e.to_string())?;
            created.push(to.clone());
            let mut input = fs::File::open(&from).map_err(|e| e.to_string())?;
            std::io::copy(&mut input, &mut output).map_err(|e| e.to_string())?;
            output.sync_all().map_err(|e| e.to_string())?;
            fs::set_permissions(
                to,
                fs::metadata(from).map_err(|e| e.to_string())?.permissions(),
            )
            .map_err(|e| e.to_string())?;
        }
        let receipt = destination.join("standalone-receipt.json");
        let mut output = fs::File::create_new(&receipt).map_err(|e| e.to_string())?;
        created.push(receipt);
        use std::io::Write;
        output.write_all(br#"{"product":"smartmontools","lifecycle":"independent","purpose":"read-only SMART helper","source":"https://www.smartmontools.org/","license":"GPL-2.0-or-later","removal":"Remove this standalone directory when smartctl is not running; SD-300 uninstall preserves it"}"#).map_err(|e| e.to_string())?;
        output.sync_all().map_err(|e| e.to_string())
    })();
    if result.is_err() {
        for path in created.iter().rev() {
            let _ = fs::remove_file(path);
        }
        let _ = fs::remove_dir(destination);
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn validates_json_interface_without_running_a_device_probe() {
        assert_eq!(
            verify_version_json(
                br#"{"json_format_version":[1,0],"smartctl":{"version":[7,5],"exit_status":0}}"#
            )
            .unwrap(),
            "7.5"
        );
        for bad in [
            br#"{}"#.as_slice(),
            br#"{"json_format_version":[2,0],"smartctl":{"version":[7,5],"exit_status":0}}"#,
            br#"{"json_format_version":[1,0],"smartctl":{"version":[8,0],"exit_status":0}}"#,
        ] {
            assert!(verify_version_json(bad).is_err());
        }
        assert!(install(false, &AtomicBool::new(false))
            .unwrap_err()
            .contains("declined"));
        assert!(install(true, &AtomicBool::new(true))
            .unwrap_err()
            .contains("cancelled"));
    }
}
