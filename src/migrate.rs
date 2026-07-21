//! Bounded cross-method cleanup invoked only by native installers.

use std::fs::{self, OpenOptions, Permissions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

#[cfg(unix)]
use std::os::fd::AsRawFd;
#[cfg(unix)]
use std::os::unix::fs::MetadataExt;

use serde::Serialize;

use crate::cli::MigrateArgs;

const APP_NAME: &str = "sd300";
const CARGO_PACKAGE_NAME: &str = "tr300-tui";

static MIGRATION_FILE_COUNTER: AtomicU64 = AtomicU64::new(0);

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

#[derive(Debug, Clone)]
struct CargoManifestEdit {
    path: PathBuf,
    original: Vec<u8>,
    updated: Vec<u8>,
    entry_key: String,
}

#[derive(Debug)]
struct PreparedManifestEdit {
    backup: PathBuf,
    replacement: PathBuf,
    attributes: PreservedFileAttributes,
}

#[derive(Debug, Clone)]
struct PreservedFileAttributes {
    permissions: Permissions,
    #[cfg(unix)]
    uid: u32,
    #[cfg(unix)]
    gid: u32,
}

#[derive(Debug, Default)]
struct CargoRemovalState {
    binary_staged: Option<PathBuf>,
    receipt_staged: Option<PathBuf>,
    manifest_replaced: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum MigrationCheckpoint {
    ManifestReplaced,
    ReceiptStaged,
    BinaryStaged,
}

#[derive(Debug, Default)]
struct CargoRemovalOutcome {
    cleanup_residue: Vec<PathBuf>,
}

impl PreservedFileAttributes {
    fn from_metadata(metadata: &fs::Metadata) -> Self {
        Self {
            permissions: metadata.permissions(),
            #[cfg(unix)]
            uid: metadata.uid(),
            #[cfg(unix)]
            gid: metadata.gid(),
        }
    }

    fn apply_to(&self, file: &fs::File) -> io::Result<()> {
        #[cfg(unix)]
        {
            let metadata = file.metadata()?;
            if metadata.uid() != self.uid || metadata.gid() != self.gid {
                // SAFETY: `file` remains open for the call, so its raw descriptor is valid.
                let result = unsafe { libc::fchown(file.as_raw_fd(), self.uid, self.gid) };
                if result != 0 {
                    return Err(io::Error::last_os_error());
                }
            }
        }
        file.set_permissions(self.permissions.clone())
    }
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

    let manifest_path = cargo_home.join(".crates2.json");
    let manifest_edit = match fs::read(&manifest_path) {
        Ok(manifest) => match cargo_manifest_edit(&manifest_path, manifest) {
            Ok(edit) => edit,
            Err(error) => {
                return vec![preserved(
                    "cargo_manifest_entry",
                    Some(manifest_path),
                    &format!("Cargo ownership metadata is ambiguous: {error}"),
                )];
            }
        },
        Err(error) if error.kind() == io::ErrorKind::NotFound && receipt_exists => None,
        Err(error) => {
            return vec![preserved(
                "cargo_copy",
                Some(binary),
                &format!(
                    "Cargo-path ownership could not be reconciled because {} could not be read: {error}",
                    manifest_path.display()
                ),
            )];
        }
    };

    if !receipt_exists && manifest_edit.is_none() {
        return vec![preserved(
            "cargo_copy",
            Some(binary),
            &format!(
                "receipt-less Cargo-path binary is not owned by {CARGO_PACKAGE_NAME} in {}",
                manifest_path.display()
            ),
        )];
    }

    if args.dry_run {
        let mut results = vec![would_remove("cargo_copy", binary)];
        if let Some(receipt) = receipt.filter(|path| path.is_file()) {
            results.push(would_remove("managed_receipt", receipt));
        }
        if let Some(edit) = manifest_edit {
            results.push(would_remove_manifest_entry(&edit));
        }
        return results;
    }

    let receipt_to_remove = receipt.as_deref().filter(|path| path.is_file());
    let outcome = match commit_cargo_removal(&binary, receipt_to_remove, manifest_edit.as_ref()) {
        Ok(outcome) => outcome,
        Err(error) => {
            return vec![failure(
                "cargo_copy",
                Some(binary),
                &format!("could not commit Cargo ownership transfer: {error}"),
            )];
        }
    };

    let mut results = vec![removed("cargo_copy", binary)];
    if let Some(receipt_path) = receipt.filter(|path| !path.exists()) {
        if receipt_exists {
            results.push(removed("managed_receipt", receipt_path));
        }
    }
    if let Some(edit) = manifest_edit {
        results.push(removed_manifest_entry(&edit));
    }
    if !outcome.cleanup_residue.is_empty() {
        let residue = outcome
            .cleanup_residue
            .iter()
            .map(|path| path.display().to_string())
            .collect::<Vec<_>>()
            .join(", ");
        if let Some(result) = results.first_mut() {
            result.detail = format!(
                "removed exact owned target; committed rollback material could not be deleted: {residue}"
            );
        }
    }
    results
}

fn cargo_manifest_edit(
    path: &Path,
    original: Vec<u8>,
) -> std::result::Result<Option<CargoManifestEdit>, String> {
    let manifest = std::str::from_utf8(&original)
        .map_err(|error| format!("Cargo's .crates2.json is not UTF-8 JSON: {error}"))?;
    if crate::update::cargo_manifest_version(manifest)?.is_none() {
        return Ok(None);
    }

    let mut json: serde_json::Value = serde_json::from_slice(&original)
        .map_err(|error| format!("Cargo's .crates2.json is invalid: {error}"))?;
    let installs = json
        .get_mut("installs")
        .and_then(serde_json::Value::as_object_mut)
        .ok_or_else(|| "Cargo's .crates2.json has no installs object".to_string())?;
    let binary_name = if cfg!(windows) { "sd300.exe" } else { "sd300" };
    let prefix = format!("{CARGO_PACKAGE_NAME} ");
    let matching_keys = installs
        .iter()
        .filter_map(|(key, value)| {
            let owns_binary = key.starts_with(&prefix)
                && value
                    .get("bins")
                    .and_then(serde_json::Value::as_array)
                    .is_some_and(|bins| bins.iter().any(|bin| bin.as_str() == Some(binary_name)));
            owns_binary.then(|| key.clone())
        })
        .collect::<Vec<_>>();
    let [entry_key] = matching_keys.as_slice() else {
        return Err(format!(
            "the proven {CARGO_PACKAGE_NAME} ownership entry could not be isolated exactly"
        ));
    };
    installs
        .remove(entry_key)
        .expect("the exact manifest entry was collected from this object");

    let mut updated = serde_json::to_vec(&json)
        .map_err(|error| format!("could not serialize updated Cargo metadata: {error}"))?;
    if original.ends_with(b"\r\n") {
        updated.extend_from_slice(b"\r\n");
    } else if original.ends_with(b"\n") {
        updated.push(b'\n');
    }

    Ok(Some(CargoManifestEdit {
        path: path.to_path_buf(),
        original,
        updated,
        entry_key: entry_key.clone(),
    }))
}

fn commit_cargo_removal(
    binary: &Path,
    receipt: Option<&Path>,
    manifest: Option<&CargoManifestEdit>,
) -> std::result::Result<CargoRemovalOutcome, String> {
    commit_cargo_removal_with_checkpoint(binary, receipt, manifest, |_| Ok(()))
}

fn commit_cargo_removal_with_checkpoint<F>(
    binary: &Path,
    receipt: Option<&Path>,
    manifest: Option<&CargoManifestEdit>,
    mut checkpoint: F,
) -> std::result::Result<CargoRemovalOutcome, String>
where
    F: FnMut(MigrationCheckpoint) -> io::Result<()>,
{
    let prepared_manifest = manifest.map(prepare_manifest_edit).transpose()?;
    let mut state = CargoRemovalState::default();

    let operation = (|| -> std::result::Result<(), String> {
        if let (Some(edit), Some(prepared)) = (manifest, prepared_manifest.as_ref()) {
            let current = fs::read(&edit.path).map_err(|error| {
                format!(
                    "could not re-read {} before the atomic metadata write: {error}",
                    edit.path.display()
                )
            })?;
            if current != edit.original {
                return Err(format!(
                    "{} changed after ownership was proven; no stale metadata write was applied",
                    edit.path.display()
                ));
            }
            fs::rename(&prepared.replacement, &edit.path).map_err(|error| {
                format!(
                    "could not atomically replace {}: {error}",
                    edit.path.display()
                )
            })?;
            state.manifest_replaced = true;
            checkpoint(MigrationCheckpoint::ManifestReplaced)
                .map_err(|error| format!("metadata checkpoint failed: {error}"))?;
        }

        if let Some(receipt_path) = receipt {
            let staged = unique_adjacent_path(receipt_path, "receipt-backup");
            fs::rename(receipt_path, &staged).map_err(|error| {
                format!(
                    "could not stage managed receipt {}: {error}",
                    receipt_path.display()
                )
            })?;
            state.receipt_staged = Some(staged);
            checkpoint(MigrationCheckpoint::ReceiptStaged)
                .map_err(|error| format!("receipt checkpoint failed: {error}"))?;
        }

        let staged = unique_adjacent_path(binary, "binary-backup");
        fs::rename(binary, &staged).map_err(|error| {
            format!("could not stage Cargo binary {}: {error}", binary.display())
        })?;
        state.binary_staged = Some(staged);
        checkpoint(MigrationCheckpoint::BinaryStaged)
            .map_err(|error| format!("binary checkpoint failed: {error}"))?;
        Ok(())
    })();

    if let Err(cause) = operation {
        let rollback_errors = rollback_cargo_removal(
            binary,
            receipt,
            manifest,
            prepared_manifest.as_ref(),
            &state,
        );
        if rollback_errors.is_empty() {
            cleanup_prepared_transaction(&state, prepared_manifest.as_ref());
            return Err(format!("{cause}; rollback succeeded"));
        }

        let recovery_paths = transaction_artifacts(&state, prepared_manifest.as_ref())
            .into_iter()
            .filter(|path| path.exists())
            .map(|path| path.display().to_string())
            .collect::<Vec<_>>()
            .join(", ");
        return Err(format!(
            "{cause}; rollback failed: {}; recovery material was preserved at: {recovery_paths}",
            rollback_errors.join("; ")
        ));
    }

    let cleanup_residue = cleanup_prepared_transaction(&state, prepared_manifest.as_ref());
    Ok(CargoRemovalOutcome { cleanup_residue })
}

fn prepare_manifest_edit(
    edit: &CargoManifestEdit,
) -> std::result::Result<PreparedManifestEdit, String> {
    let current = fs::read(&edit.path).map_err(|error| {
        format!(
            "could not read {} while preparing its backup: {error}",
            edit.path.display()
        )
    })?;
    if current != edit.original {
        return Err(format!(
            "{} changed after ownership was proven; no mutation was attempted",
            edit.path.display()
        ));
    }
    let metadata = fs::metadata(&edit.path).map_err(|error| {
        format!(
            "could not read file attributes for {}: {error}",
            edit.path.display()
        )
    })?;
    let attributes = PreservedFileAttributes::from_metadata(&metadata);
    let backup = write_adjacent_file(&edit.path, "manifest-backup", &edit.original, None).map_err(
        |error| {
            format!(
                "could not create an ownership metadata backup beside {}: {error}",
                edit.path.display()
            )
        },
    )?;
    let replacement = match write_adjacent_file(
        &edit.path,
        "manifest-replacement",
        &edit.updated,
        Some(&attributes),
    ) {
        Ok(replacement) => replacement,
        Err(error) => {
            let _ = fs::remove_file(&backup);
            return Err(format!(
                "could not prepare an atomic ownership metadata write beside {}: {error}",
                edit.path.display()
            ));
        }
    };
    Ok(PreparedManifestEdit {
        backup,
        replacement,
        attributes,
    })
}

fn rollback_cargo_removal(
    binary: &Path,
    receipt: Option<&Path>,
    manifest: Option<&CargoManifestEdit>,
    prepared_manifest: Option<&PreparedManifestEdit>,
    state: &CargoRemovalState,
) -> Vec<String> {
    let mut errors = Vec::new();

    if let Some(staged) = state.binary_staged.as_deref() {
        if binary.exists() {
            errors.push(format!(
                "refusing to overwrite a replacement binary at {}",
                binary.display()
            ));
        } else if let Err(error) = fs::rename(staged, binary) {
            errors.push(format!(
                "could not restore binary {}: {error}",
                binary.display()
            ));
        }
    }

    if let (Some(receipt_path), Some(staged)) = (receipt, state.receipt_staged.as_deref()) {
        if receipt_path.exists() {
            errors.push(format!(
                "refusing to overwrite a replacement receipt at {}",
                receipt_path.display()
            ));
        } else if let Err(error) = fs::rename(staged, receipt_path) {
            errors.push(format!(
                "could not restore receipt {}: {error}",
                receipt_path.display()
            ));
        }
    }

    if state.manifest_replaced {
        if let (Some(edit), Some(prepared)) = (manifest, prepared_manifest) {
            match fs::read(&edit.path) {
                Ok(current) if current == edit.original => {}
                Ok(current) if current == edit.updated => {
                    if let Err(error) =
                        restore_file_atomically(&prepared.backup, &edit.path, &prepared.attributes)
                    {
                        errors.push(format!(
                            "could not restore Cargo metadata {}: {error}",
                            edit.path.display()
                        ));
                    }
                }
                Ok(_) => errors.push(format!(
                    "refusing to overwrite concurrently changed Cargo metadata at {}",
                    edit.path.display()
                )),
                Err(error) => errors.push(format!(
                    "could not inspect Cargo metadata {} during rollback: {error}",
                    edit.path.display()
                )),
            }
        }
    }

    errors
}

fn restore_file_atomically(
    source: &Path,
    target: &Path,
    attributes: &PreservedFileAttributes,
) -> io::Result<()> {
    let contents = fs::read(source)?;
    let replacement = write_adjacent_file(target, "rollback", &contents, Some(attributes))?;
    if let Err(error) = fs::rename(&replacement, target) {
        let _ = fs::remove_file(replacement);
        return Err(error);
    }
    Ok(())
}

fn write_adjacent_file(
    target: &Path,
    role: &str,
    contents: &[u8],
    attributes: Option<&PreservedFileAttributes>,
) -> io::Result<PathBuf> {
    for _ in 0..64 {
        let path = unique_adjacent_path(target, role);
        let mut file = match OpenOptions::new().write(true).create_new(true).open(&path) {
            Ok(file) => file,
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => continue,
            Err(error) => return Err(error),
        };
        let result = (|| {
            file.write_all(contents)?;
            file.sync_all()?;
            if let Some(attributes) = attributes {
                attributes.apply_to(&file)?;
            }
            file.sync_all()?;
            drop(file);
            Ok(())
        })();
        if let Err(error) = result {
            let _ = fs::remove_file(&path);
            return Err(error);
        }
        return Ok(path);
    }
    Err(io::Error::new(
        io::ErrorKind::AlreadyExists,
        "could not reserve a unique adjacent migration file",
    ))
}

fn unique_adjacent_path(target: &Path, role: &str) -> PathBuf {
    let sequence = MIGRATION_FILE_COUNTER.fetch_add(1, Ordering::Relaxed);
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let file_name = target
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("sd300");
    target.with_file_name(format!(
        ".{file_name}.sd300-migrate-{}-{timestamp}-{sequence}.{role}",
        std::process::id()
    ))
}

fn transaction_artifacts(
    state: &CargoRemovalState,
    prepared_manifest: Option<&PreparedManifestEdit>,
) -> Vec<PathBuf> {
    let mut paths = Vec::new();
    if let Some(path) = &state.binary_staged {
        paths.push(path.clone());
    }
    if let Some(path) = &state.receipt_staged {
        paths.push(path.clone());
    }
    if let Some(prepared) = prepared_manifest {
        paths.push(prepared.backup.clone());
        paths.push(prepared.replacement.clone());
    }
    paths
}

fn cleanup_prepared_transaction(
    state: &CargoRemovalState,
    prepared_manifest: Option<&PreparedManifestEdit>,
) -> Vec<PathBuf> {
    transaction_artifacts(state, prepared_manifest)
        .into_iter()
        .filter_map(|path| match fs::remove_file(&path) {
            Ok(()) => None,
            Err(error) if error.kind() == io::ErrorKind::NotFound => None,
            Err(_) => Some(path),
        })
        .collect()
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

fn would_remove_manifest_entry(edit: &CargoManifestEdit) -> CleanupResult {
    CleanupResult {
        target: "cargo_manifest_entry",
        status: CleanupStatus::WouldRemove,
        path: Some(edit.path.clone()),
        detail: format!(
            "exact Cargo ownership entry would be removed: {}",
            edit.entry_key
        ),
    }
}

fn removed_manifest_entry(edit: &CargoManifestEdit) -> CleanupResult {
    CleanupResult {
        target: "cargo_manifest_entry",
        status: CleanupStatus::Removed,
        path: Some(edit.path.clone()),
        detail: format!("removed exact Cargo ownership entry: {}", edit.entry_key),
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

    fn cargo_binary_name() -> &'static str {
        if cfg!(windows) {
            "sd300.exe"
        } else {
            "sd300"
        }
    }

    fn cargo_manifest_with_other_install(entry_version: &str) -> Vec<u8> {
        let binary_name = cargo_binary_name();
        let mut installs = serde_json::Map::new();
        installs.insert(
            "another-tool 9.1.0 (registry+https://example.invalid/index)".into(),
            serde_json::json!({"bins": ["another-tool"], "features": ["safe"]}),
        );
        installs.insert(
            format!("tr300-tui {entry_version} (registry+https://example.invalid/index)"),
            serde_json::json!({"bins": [binary_name], "features": ["default"]}),
        );
        installs.insert(
            "tr300-tui 0.1.0 (path+file:///foreign)".into(),
            serde_json::json!({"bins": ["not-sd300"]}),
        );
        let mut manifest = serde_json::to_vec_pretty(&serde_json::json!({
            "v1": 1,
            "installs": installs,
        }))
        .unwrap();
        manifest.push(b'\n');
        manifest
    }

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
    fn dry_run_preserves_proven_cargo_binary() {
        let temp = tempfile::tempdir().unwrap();
        let cargo_home = temp.path().join("cargo");
        let binary = cargo_home
            .join("bin")
            .join(if cfg!(windows) { "sd300.exe" } else { "sd300" });
        std::fs::create_dir_all(binary.parent().unwrap()).unwrap();
        std::fs::write(&binary, b"owned").unwrap();
        let binary_name = if cfg!(windows) { "sd300.exe" } else { "sd300" };
        std::fs::write(
            cargo_home.join(".crates2.json"),
            serde_json::json!({
                "installs": {
                    "tr300-tui 1.4.3 (registry+https://example.invalid/index)": {
                        "bins": [binary_name]
                    }
                }
            })
            .to_string(),
        )
        .unwrap();
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

    #[test]
    fn manifest_edit_removes_only_the_exact_owned_entry() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join(".crates2.json");
        let original = cargo_manifest_with_other_install("2.0.6");
        let edit = cargo_manifest_edit(&path, original.clone())
            .unwrap()
            .expect("the exact Cargo entry should be proven");

        assert!(edit.entry_key.starts_with("tr300-tui 2.0.6 (registry+"));
        assert_eq!(edit.original, original);
        assert!(edit.updated.ends_with(b"\n"));

        let updated: serde_json::Value = serde_json::from_slice(&edit.updated).unwrap();
        let installs = updated["installs"].as_object().unwrap();
        assert!(!installs.contains_key(&edit.entry_key));
        assert!(
            installs.contains_key("another-tool 9.1.0 (registry+https://example.invalid/index)")
        );
        assert!(installs.contains_key("tr300-tui 0.1.0 (path+file:///foreign)"));
        assert_eq!(updated["v1"], 1);
        assert_eq!(
            updated["installs"]["another-tool 9.1.0 (registry+https://example.invalid/index)"]
                ["features"],
            serde_json::json!(["safe"])
        );
    }

    #[test]
    fn committed_cargo_cleanup_removes_binary_and_manifest_ownership_together() {
        let temp = tempfile::tempdir().unwrap();
        let cargo_home = temp.path().join("cargo");
        let binary = cargo_home.join("bin").join(cargo_binary_name());
        let manifest_path = cargo_home.join(".crates2.json");
        fs::create_dir_all(binary.parent().unwrap()).unwrap();
        fs::write(&binary, b"owned executable").unwrap();
        fs::write(&manifest_path, cargo_manifest_with_other_install("2.0.6")).unwrap();

        let results = clean_cargo_pair(&MigrateArgs {
            cargo_copy: true,
            strict: true,
            cargo_home: Some(cargo_home.clone()),
            user_profile: Some(temp.path().to_path_buf()),
            ..MigrateArgs::default()
        });

        assert!(
            results
                .iter()
                .all(|result| result.status == CleanupStatus::Removed),
            "unexpected cleanup results: {results:?}"
        );
        assert!(!binary.exists());
        let manifest: serde_json::Value =
            serde_json::from_slice(&fs::read(&manifest_path).unwrap()).unwrap();
        let installs = manifest["installs"].as_object().unwrap();
        assert_eq!(installs.len(), 2);
        assert!(
            installs.contains_key("another-tool 9.1.0 (registry+https://example.invalid/index)")
        );
        assert!(installs.contains_key("tr300-tui 0.1.0 (path+file:///foreign)"));
        assert!(fs::read_dir(&cargo_home)
            .unwrap()
            .chain(fs::read_dir(cargo_home.join("bin")).unwrap())
            .all(|entry| !entry
                .unwrap()
                .file_name()
                .to_string_lossy()
                .contains("sd300-migrate")));
    }

    #[test]
    fn transaction_failure_restores_manifest_receipt_and_binary_exactly() {
        let temp = tempfile::tempdir().unwrap();
        let cargo_home = temp.path().join("cargo");
        let binary = cargo_home.join("bin").join(cargo_binary_name());
        let receipt = temp.path().join("sd300-receipt.json");
        let manifest_path = cargo_home.join(".crates2.json");
        fs::create_dir_all(binary.parent().unwrap()).unwrap();
        let binary_contents = b"original executable bytes";
        let receipt_contents = br#"{"provider":{"source":"cargo-dist"}}"#;
        let manifest_contents = cargo_manifest_with_other_install("2.0.6");
        fs::write(&binary, binary_contents).unwrap();
        fs::write(&receipt, receipt_contents).unwrap();
        fs::write(&manifest_path, &manifest_contents).unwrap();
        let edit = cargo_manifest_edit(&manifest_path, manifest_contents.clone())
            .unwrap()
            .unwrap();

        let result = commit_cargo_removal_with_checkpoint(
            &binary,
            Some(&receipt),
            Some(&edit),
            |checkpoint| {
                if checkpoint == MigrationCheckpoint::BinaryStaged {
                    Err(io::Error::other("injected post-stage failure"))
                } else {
                    Ok(())
                }
            },
        );

        let error = result.expect_err("the injected failure must abort the transfer");
        assert!(error.contains("rollback succeeded"), "{error}");
        assert_eq!(fs::read(&binary).unwrap(), binary_contents);
        assert_eq!(fs::read(&receipt).unwrap(), receipt_contents);
        assert_eq!(fs::read(&manifest_path).unwrap(), manifest_contents);
        assert!(fs::read_dir(binary.parent().unwrap())
            .unwrap()
            .all(|entry| {
                !entry
                    .unwrap()
                    .file_name()
                    .to_string_lossy()
                    .contains("sd300-migrate")
            }));
        assert!(fs::read_dir(temp.path()).unwrap().all(|entry| {
            !entry
                .unwrap()
                .file_name()
                .to_string_lossy()
                .contains("sd300-migrate")
        }));
        assert!(fs::read_dir(&cargo_home).unwrap().all(|entry| {
            !entry
                .unwrap()
                .file_name()
                .to_string_lossy()
                .contains("sd300-migrate")
        }));
    }

    #[test]
    fn manifest_change_after_proof_aborts_before_binary_or_receipt_mutation() {
        let temp = tempfile::tempdir().unwrap();
        let cargo_home = temp.path().join("cargo");
        let binary = cargo_home.join("bin").join(cargo_binary_name());
        let receipt = temp.path().join("receipt.json");
        let manifest_path = cargo_home.join(".crates2.json");
        fs::create_dir_all(binary.parent().unwrap()).unwrap();
        fs::write(&binary, b"owned executable").unwrap();
        fs::write(&receipt, b"owned receipt").unwrap();
        let original = cargo_manifest_with_other_install("2.0.6");
        fs::write(&manifest_path, &original).unwrap();
        let edit = cargo_manifest_edit(&manifest_path, original)
            .unwrap()
            .unwrap();
        let concurrent = cargo_manifest_with_other_install("2.0.7");
        fs::write(&manifest_path, &concurrent).unwrap();

        let error = commit_cargo_removal(&binary, Some(&receipt), Some(&edit))
            .expect_err("concurrent metadata changes must fail closed");

        assert!(
            error.contains("changed after ownership was proven"),
            "{error}"
        );
        assert_eq!(fs::read(&binary).unwrap(), b"owned executable");
        assert_eq!(fs::read(&receipt).unwrap(), b"owned receipt");
        assert_eq!(fs::read(&manifest_path).unwrap(), concurrent);
    }

    #[test]
    fn receipt_less_unproven_cargo_binary_is_preserved() {
        let temp = tempfile::tempdir().unwrap();
        let cargo_home = temp.path().join("cargo");
        let binary = cargo_home
            .join("bin")
            .join(if cfg!(windows) { "sd300.exe" } else { "sd300" });
        std::fs::create_dir_all(binary.parent().unwrap()).unwrap();
        std::fs::write(&binary, b"foreign").unwrap();

        let results = clean_cargo_pair(&MigrateArgs {
            cargo_copy: true,
            strict: true,
            cargo_home: Some(cargo_home),
            user_profile: Some(temp.path().to_path_buf()),
            ..MigrateArgs::default()
        });

        assert!(results
            .iter()
            .any(|result| result.status == CleanupStatus::Preserved));
        assert_eq!(std::fs::read(binary).unwrap(), b"foreign");
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
