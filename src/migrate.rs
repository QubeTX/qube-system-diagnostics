//! Bounded cross-method cleanup invoked only by native installers.

use std::path::{Path, PathBuf};

use serde::Serialize;

use crate::cli::MigrateArgs;

const APP_NAME: &str = "sd300";

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
enum CleanupStatus {
    Removed,
    WouldRemove,
    Absent,
    Preserved,
    Failed,
}

#[derive(Debug, Clone, Serialize)]
struct CleanupResult {
    target: &'static str,
    status: CleanupStatus,
    path: Option<PathBuf>,
    detail: String,
}

pub fn run(args: &MigrateArgs) -> i32 {
    let cargo_copy = args.cargo_copy || !args.other_edition;
    let mut results = Vec::new();
    if cargo_copy {
        results.extend(clean_cargo_pair(args));
    }
    if args.other_edition {
        results.push(clean_other_edition(args));
    }

    let success = results.iter().all(|result| {
        matches!(
            result.status,
            CleanupStatus::Removed | CleanupStatus::WouldRemove | CleanupStatus::Absent
        ) || (!args.strict && result.status == CleanupStatus::Preserved)
    });

    if !args.quiet {
        for result in &results {
            println!(
                "{}: {:?}: {}{}",
                result.target,
                result.status,
                result.detail,
                result
                    .path
                    .as_ref()
                    .map(|path| format!(" ({})", path.display()))
                    .unwrap_or_default()
            );
        }
    }
    if success {
        0
    } else {
        2
    }
}

fn clean_cargo_pair(args: &MigrateArgs) -> Vec<CleanupResult> {
    let Some(cargo_home) = resolve_cargo_home(args) else {
        return vec![failure(
            "cargo_copy",
            None,
            "could not resolve the invoking user's Cargo home",
        )];
    };
    let binary = cargo_home
        .join("bin")
        .join(if cfg!(windows) { "sd300.exe" } else { "sd300" });
    let receipt = resolve_receipt_path(args);
    let binary_exists = binary.is_file();
    let receipt_exists = receipt.as_ref().is_some_and(|path| path.is_file());

    if !binary_exists && !receipt_exists {
        return vec![absent(
            "cargo_copy",
            Some(binary),
            "no Cargo-path copy or managed receipt exists",
        )];
    }

    if current_exe_matches(&binary) {
        return vec![preserved(
            "cargo_copy",
            Some(binary),
            "refusing to remove the running executable",
        )];
    }

    if receipt_exists {
        let receipt_path = receipt.as_ref().expect("receipt path was checked");
        if !receipt_exactly_matches(receipt_path, &cargo_home) {
            return vec![preserved(
                "managed_receipt",
                Some(receipt_path.clone()),
                "receipt does not exactly identify SD-300 cargo-dist ownership at this Cargo home",
            )];
        }
        if !binary_exists {
            return vec![preserved(
                "managed_receipt",
                Some(receipt_path.clone()),
                "receipt exists but its owned binary is missing",
            )];
        }
    }

    if args.dry_run {
        let mut results = vec![would_remove("cargo_copy", binary)];
        if let Some(receipt) = receipt.filter(|path| path.is_file()) {
            results.push(would_remove("managed_receipt", receipt));
        }
        return results;
    }

    let backup = binary.with_file_name(format!(
        ".sd300-migrate-{}-{}",
        std::process::id(),
        if cfg!(windows) { "sd300.exe" } else { "sd300" }
    ));
    if let Err(error) = std::fs::rename(&binary, &backup) {
        return vec![failure(
            "cargo_copy",
            Some(binary),
            &format!("could not stage bounded removal: {error}"),
        )];
    }

    if let Some(receipt_path) = receipt.as_ref().filter(|path| path.is_file()) {
        if let Err(error) = std::fs::remove_file(receipt_path) {
            let rollback = std::fs::rename(&backup, &binary);
            return vec![failure(
                "managed_receipt",
                Some(receipt_path.clone()),
                &format!(
                    "could not remove receipt: {error}; binary rollback {}",
                    if rollback.is_ok() {
                        "succeeded"
                    } else {
                        "failed"
                    }
                ),
            )];
        }
    }

    if let Err(error) = std::fs::remove_file(&backup) {
        return vec![failure(
            "cargo_copy",
            Some(backup),
            &format!("could not finish staged removal: {error}"),
        )];
    }

    let mut results = vec![removed("cargo_copy", binary)];
    if let Some(receipt_path) = receipt.filter(|path| !path.exists()) {
        if receipt_exists {
            results.push(removed("managed_receipt", receipt_path));
        }
    }
    results
}

fn receipt_exactly_matches(path: &Path, cargo_home: &Path) -> bool {
    let Ok(contents) = std::fs::read_to_string(path) else {
        return false;
    };
    let Ok(json) = serde_json::from_str::<serde_json::Value>(&contents) else {
        return false;
    };
    json.pointer("/provider/source")
        .and_then(serde_json::Value::as_str)
        == Some("cargo-dist")
        && json
            .pointer("/source/app_name")
            .and_then(serde_json::Value::as_str)
            == Some(APP_NAME)
        && json
            .get("install_prefix")
            .and_then(serde_json::Value::as_str)
            .is_some_and(|prefix| same_path(Path::new(prefix), cargo_home))
}

fn resolve_cargo_home(args: &MigrateArgs) -> Option<PathBuf> {
    args.cargo_home
        .clone()
        .or_else(|| {
            args.user_profile
                .as_ref()
                .map(|profile| profile.join(".cargo"))
        })
        .or_else(|| std::env::var_os("CARGO_HOME").map(PathBuf::from))
        .or_else(|| {
            std::env::var_os(if cfg!(windows) { "USERPROFILE" } else { "HOME" })
                .map(PathBuf::from)
                .map(|home| home.join(".cargo"))
        })
}

fn resolve_receipt_path(args: &MigrateArgs) -> Option<PathBuf> {
    #[cfg(windows)]
    {
        if let Some(profile) = &args.user_profile {
            return Some(
                profile
                    .join("AppData")
                    .join("Local")
                    .join(APP_NAME)
                    .join("sd300-receipt.json"),
            );
        }
        std::env::var_os("XDG_CONFIG_HOME")
            .or_else(|| std::env::var_os("LOCALAPPDATA"))
            .map(PathBuf::from)
            .map(|root| root.join(APP_NAME).join("sd300-receipt.json"))
    }
    #[cfg(not(windows))]
    {
        args.user_profile
            .clone()
            .map(|home| home.join(".config"))
            .or_else(|| std::env::var_os("XDG_CONFIG_HOME").map(PathBuf::from))
            .or_else(|| {
                std::env::var_os("HOME")
                    .map(PathBuf::from)
                    .map(|home| home.join(".config"))
            })
            .map(|root| root.join(APP_NAME).join("sd300-receipt.json"))
    }
}

#[cfg(windows)]
fn clean_other_edition(args: &MigrateArgs) -> CleanupResult {
    let Some(current) = std::env::current_exe().ok() else {
        return failure(
            "other_edition",
            None,
            "could not resolve the running executable",
        );
    };
    let global = std::env::var_os("ProgramFiles")
        .map(PathBuf::from)
        .map(|root| root.join(APP_NAME).join("bin").join("sd300.exe"));
    let corporate = args
        .user_profile
        .as_ref()
        .map(|profile| profile.join("AppData").join("Local"))
        .or_else(|| std::env::var_os("LOCALAPPDATA").map(PathBuf::from))
        .map(|root| {
            root.join("Programs")
                .join(APP_NAME)
                .join("bin")
                .join("sd300.exe")
        });

    let (other, other_is_corporate) = if global
        .as_ref()
        .is_some_and(|path| same_path(path, &current))
    {
        (corporate, true)
    } else if corporate
        .as_ref()
        .is_some_and(|path| same_path(path, &current))
    {
        (global, false)
    } else {
        return preserved(
            "other_edition",
            Some(current),
            "running executable is not in an exact Global or Corporate install path",
        );
    };
    let Some(other) = other else {
        return failure(
            "other_edition",
            None,
            "could not resolve the opposite edition path",
        );
    };

    if opposite_edition_registered(other_is_corporate) {
        return preserved(
            "other_edition",
            Some(other),
            "the opposite edition is registered; its owning uninstaller must run first",
        );
    }
    if !other.is_file() {
        return absent(
            "other_edition",
            Some(other),
            "no orphaned opposite-edition binary exists",
        );
    }
    if args.dry_run {
        return would_remove("other_edition", other);
    }
    match std::fs::remove_file(&other) {
        Ok(()) => removed("other_edition", other),
        Err(error) => failure(
            "other_edition",
            Some(other),
            &format!("could not remove the orphaned binary: {error}"),
        ),
    }
}

#[cfg(windows)]
fn opposite_edition_registered(corporate: bool) -> bool {
    use winreg::enums::{
        HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE, KEY_READ, KEY_WOW64_32KEY, KEY_WOW64_64KEY,
    };
    use winreg::RegKey;

    let expected_inno = if corporate {
        "ED209931-B5C0-43AE-89F6-83EE2C581653"
    } else {
        "DC74D35F-CBF4-425F-B11E-E9EA87C13CA9"
    };
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
    roots.into_iter().any(|(root, flags)| {
        let Ok(uninstall) = root.open_subkey_with_flags(
            "SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\Uninstall",
            flags,
        ) else {
            return false;
        };
        uninstall
            .enum_keys()
            .filter_map(std::result::Result::ok)
            .any(|key_name| {
                if key_name.to_ascii_uppercase().contains(expected_inno) {
                    return true;
                }
                let Ok(entry) = uninstall.open_subkey_with_flags(&key_name, KEY_READ) else {
                    return false;
                };
                let display_name: String = entry.get_value("DisplayName").unwrap_or_default();
                let windows_installer: u32 =
                    entry.get_value("WindowsInstaller").unwrap_or_default();
                windows_installer == 1
                    && display_name.to_ascii_lowercase().contains("sd-300")
                    && display_name.to_ascii_lowercase().contains("corporate") == corporate
            })
    })
}

#[cfg(not(windows))]
fn clean_other_edition(_args: &MigrateArgs) -> CleanupResult {
    absent(
        "other_edition",
        None,
        "Global and Corporate editions are Windows-only",
    )
}

fn current_exe_matches(candidate: &Path) -> bool {
    std::env::current_exe()
        .ok()
        .is_some_and(|current| same_path(&current, candidate))
}

fn same_path(left: &Path, right: &Path) -> bool {
    let left = left.canonicalize().unwrap_or_else(|_| left.to_path_buf());
    let right = right.canonicalize().unwrap_or_else(|_| right.to_path_buf());
    #[cfg(windows)]
    {
        left.to_string_lossy()
            .eq_ignore_ascii_case(&right.to_string_lossy())
    }
    #[cfg(not(windows))]
    {
        left == right
    }
}

fn removed(target: &'static str, path: PathBuf) -> CleanupResult {
    CleanupResult {
        target,
        status: CleanupStatus::Removed,
        path: Some(path),
        detail: "removed exact owned target".into(),
    }
}

fn would_remove(target: &'static str, path: PathBuf) -> CleanupResult {
    CleanupResult {
        target,
        status: CleanupStatus::WouldRemove,
        path: Some(path),
        detail: "exact target would be removed".into(),
    }
}

fn absent(target: &'static str, path: Option<PathBuf>, detail: &str) -> CleanupResult {
    CleanupResult {
        target,
        status: CleanupStatus::Absent,
        path,
        detail: detail.into(),
    }
}

fn preserved(target: &'static str, path: Option<PathBuf>, detail: &str) -> CleanupResult {
    CleanupResult {
        target,
        status: CleanupStatus::Preserved,
        path,
        detail: detail.into(),
    }
}

fn failure(target: &'static str, path: Option<PathBuf>, detail: &str) -> CleanupResult {
    CleanupResult {
        target,
        status: CleanupStatus::Failed,
        path,
        detail: detail.into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn receipt_requires_exact_provider_app_and_prefix() {
        let temp = tempfile::tempdir().unwrap();
        let receipt = temp.path().join("receipt.json");
        std::fs::write(
            &receipt,
            serde_json::json!({
                "provider": { "source": "cargo-dist" },
                "source": { "app_name": "sd300" },
                "install_prefix": temp.path(),
            })
            .to_string(),
        )
        .unwrap();
        assert!(receipt_exactly_matches(&receipt, temp.path()));
        std::fs::write(
            &receipt,
            serde_json::json!({
                "provider": { "source": "foreign" },
                "source": { "app_name": "foreign" },
                "install_prefix": temp.path(),
                "lookalike": { "source": "cargo-dist", "app_name": "sd300" },
            })
            .to_string(),
        )
        .unwrap();
        assert!(!receipt_exactly_matches(&receipt, temp.path()));
        std::fs::write(&receipt, r#"{"provider":{"source":"cargo"}}"#).unwrap();
        assert!(!receipt_exactly_matches(&receipt, temp.path()));
    }

    #[test]
    fn dry_run_preserves_allowlisted_binary() {
        let temp = tempfile::tempdir().unwrap();
        let cargo_home = temp.path().join("cargo");
        let binary = cargo_home
            .join("bin")
            .join(if cfg!(windows) { "sd300.exe" } else { "sd300" });
        std::fs::create_dir_all(binary.parent().unwrap()).unwrap();
        std::fs::write(&binary, b"owned").unwrap();
        let results = clean_cargo_pair(&MigrateArgs {
            cargo_copy: true,
            dry_run: true,
            cargo_home: Some(cargo_home),
            user_profile: Some(temp.path().to_path_buf()),
            ..MigrateArgs::default()
        });
        assert!(
            results
                .iter()
                .any(|result| result.status == CleanupStatus::WouldRemove),
            "unexpected cleanup results: {results:?}"
        );
        assert!(binary.exists());
    }

    #[cfg(not(windows))]
    #[test]
    fn explicit_user_profile_controls_unix_receipt_path() {
        let temp = tempfile::tempdir().unwrap();
        let profile = temp.path().join("explicit-user");
        let receipt = resolve_receipt_path(&MigrateArgs {
            user_profile: Some(profile.clone()),
            ..MigrateArgs::default()
        });
        assert_eq!(
            receipt,
            Some(
                profile
                    .join(".config")
                    .join(APP_NAME)
                    .join("sd300-receipt.json")
            )
        );
    }

    #[test]
    fn ambiguous_receipt_blocks_binary_mutation() {
        let temp = tempfile::tempdir().unwrap();
        let profile = temp.path();
        let cargo_home = profile.join(".cargo");
        let binary = cargo_home
            .join("bin")
            .join(if cfg!(windows) { "sd300.exe" } else { "sd300" });
        std::fs::create_dir_all(binary.parent().unwrap()).unwrap();
        std::fs::write(&binary, b"owned").unwrap();
        let receipt = resolve_receipt_path(&MigrateArgs {
            user_profile: Some(profile.to_path_buf()),
            ..MigrateArgs::default()
        })
        .unwrap();
        std::fs::create_dir_all(receipt.parent().unwrap()).unwrap();
        std::fs::write(&receipt, r#"{"provider":{"source":"foreign"}}"#).unwrap();
        let results = clean_cargo_pair(&MigrateArgs {
            cargo_copy: true,
            strict: true,
            user_profile: Some(profile.to_path_buf()),
            ..MigrateArgs::default()
        });
        assert!(results
            .iter()
            .any(|result| result.status == CleanupStatus::Preserved));
        assert_eq!(std::fs::read(binary).unwrap(), b"owned");
    }
}
