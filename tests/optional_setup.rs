//! Opt-in native-runner qualification. Downloads only the pinned release;
//! it never runs diagnostics, repairs or a bandwidth test.
use serde_json::Value;
use std::path::Path;
use std::process::Command;

fn run(root: &Path, args: &[&str]) -> std::process::Output {
    let output = Command::new(env!("CARGO_BIN_EXE_sd300"))
        .args(args)
        .env("HOME", root)
        .env("USERPROFILE", root)
        .env("APPDATA", root.join("roaming"))
        .env("LOCALAPPDATA", root.join("local"))
        .env("XDG_DATA_HOME", root.join("data"))
        .env("XDG_CONFIG_HOME", root.join("config"))
        .env("CARGO_HOME", root.join("cargo"))
        .output()
        .expect("optional setup process");
    assert!(
        output.stderr.is_empty(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    output
}

#[test]
#[ignore = "downloads the official companion on a clean native qualification runner"]
fn official_archive_installs_independently_and_repeat_preserves_owner() {
    let temp = tempfile::tempdir().unwrap();
    let declined = run(temp.path(), &["tools", "nd300", "--install", "--json"]);
    assert_eq!(declined.status.code(), Some(1));
    assert_eq!(std::fs::read_dir(temp.path()).unwrap().count(), 0);
    let installed = run(
        temp.path(),
        &["tools", "nd300", "--install", "--accept", "--json"],
    );
    let payload: Value = serde_json::from_slice(&installed.stdout).unwrap();
    assert!(installed.status.success(), "setup payload: {payload}");
    assert_eq!(payload["installed"], true);
    #[cfg(windows)]
    let root = temp.path().join("local");
    #[cfg(target_os = "macos")]
    let root = temp.path().join("Library/Application Support");
    #[cfg(target_os = "linux")]
    let root = temp.path().join("data");
    let directory = root.join("nd300/standalone-4.0.1");
    let executable = directory.join(sd_300::optional_tools::executable_name("nd300"));
    let before = std::fs::read(&executable).unwrap();
    let repeated = run(
        temp.path(),
        &["tools", "nd300", "--install", "--accept", "--json"],
    );
    let repeat_payload: Value = serde_json::from_slice(&repeated.stdout).unwrap();
    assert_eq!(repeated.status.code(), Some(1), "{repeat_payload}");
    assert_eq!(std::fs::read(executable).unwrap(), before);
    assert!(directory.join("standalone-receipt.json").is_file());
    assert!(directory.join("LICENSE").is_file());
    println!("Qualified official ND-300 4.0.1 archive on {}-{}; refusal, installed version verification and owner preservation passed", std::env::consts::OS, std::env::consts::ARCH);
}

#[test]
#[ignore = "qualifies optional SMART setup on a clean native runner without reading devices"]
fn smart_helper_setup_verifies_version_and_preserves_repeat_ownership() {
    let temp = tempfile::Builder::new()
        .prefix("smart setup with spaces ")
        .tempdir()
        .unwrap();
    let declined = run(temp.path(), &["tools", "smartctl", "--install", "--json"]);
    assert_eq!(declined.status.code(), Some(1));
    assert_eq!(std::fs::read_dir(temp.path()).unwrap().count(), 0);
    let installed = run(
        temp.path(),
        &["tools", "smartctl", "--install", "--accept", "--json"],
    );
    let payload: Value = serde_json::from_slice(&installed.stdout).unwrap();
    #[cfg(all(target_os = "linux", target_env = "musl"))]
    if !installed.status.success() {
        // A native-only failure must disclose which package-manager condition
        // failed before a product fix is chosen. Never print the archive bytes.
        let mut diagnostic = Command::new("/sbin/apk");
        diagnostic
            .args([
                "--no-cache",
                "--quiet",
                "fetch",
                "--stdout",
                "smartmontools",
            ])
            .env("HOME", temp.path())
            .env("XDG_CONFIG_HOME", temp.path().join("config"));
        let output = sd_300::collectors::command::run_memory_command(
            &mut diagnostic,
            sd_300::collectors::command::CommandTimeout::Custom(std::time::Duration::from_secs(30)),
            &std::sync::atomic::AtomicBool::new(false),
        );
        match output {
            Ok(output) => println!(
                "Native Alpine fetch: exit={:?}, failure={:?}, stderr={}",
                output.status.and_then(|s| s.code()),
                output.failure,
                String::from_utf8_lossy(&output.stderr)
                    .chars()
                    .take(2048)
                    .collect::<String>()
            ),
            Err(error) => println!("Native Alpine fetch could not start: {error}"),
        }
    }
    assert!(installed.status.success(), "SMART setup payload: {payload}");
    assert_eq!(payload["installed"], true);
    assert!(payload["message"]
        .as_str()
        .unwrap()
        .contains("Verified smartctl 7."));
    let repeated = run(
        temp.path(),
        &["tools", "smartctl", "--install", "--accept", "--json"],
    );
    assert_eq!(
        repeated.status.code(),
        Some(1),
        "{}",
        String::from_utf8_lossy(&repeated.stdout)
    );
    println!(
        "Qualified SMART helper setup on {}-{} without reading a disk or starting a self-test",
        std::env::consts::OS,
        std::env::consts::ARCH
    );
}
