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

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::cli::{MigrateArgs, MsiCargoAction};

const APP_NAME: &str = "sd300";
const CARGO_PACKAGE_NAME: &str = "tr300-tui";
const MSI_CARGO_JOURNAL_SCHEMA: u32 = 1;

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
    manifests_replaced: usize,
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

#[derive(Debug, Clone, Serialize, Deserialize)]
struct MsiCargoJournal {
    schema_version: u32,
    cargo_home: PathBuf,
    user_profile: PathBuf,
    binary: MsiCargoStagedFile,
    receipt: Option<MsiCargoStagedFile>,
    manifests: Vec<MsiCargoManifestFile>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct MsiCargoStagedFile {
    target: PathBuf,
    backup: PathBuf,
    original_sha256: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct MsiCargoManifestFile {
    target: PathBuf,
    backup: PathBuf,
    replacement: PathBuf,
    original_sha256: String,
    updated_sha256: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum MsiCargoCheckpoint {
    JournalWritten,
    ManifestReplaced(usize),
    ReceiptStaged,
    BinaryStaged,
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
    if let Some(action) = args.msi_cargo_action {
        let result = if !args.cargo_copy || args.other_edition || args.dry_run {
            failure(
                "cargo_copy",
                args.msi_cargo_journal.clone(),
                "MSI Cargo transaction actions require --cargo-copy and cannot be combined with --other-edition or --dry-run",
            )
        } else {
            match action {
                MsiCargoAction::Prepare => {
                    let results = clean_cargo_pair(args);
                    let success = results.iter().all(|result| {
                        matches!(
                            result.status,
                            CleanupStatus::Removed
                                | CleanupStatus::WouldRemove
                                | CleanupStatus::Absent
                        )
                    });
                    if !args.quiet {
                        print_cleanup_results(&results);
                    }
                    return if success { 0 } else { 2 };
                }
                MsiCargoAction::Rollback => rollback_msi_cargo_transaction(args),
                MsiCargoAction::Commit => commit_msi_cargo_transaction(args),
            }
        };
        if !args.quiet {
            print_cleanup_results(std::slice::from_ref(&result));
        }
        return if matches!(
            result.status,
            CleanupStatus::Removed | CleanupStatus::Absent
        ) {
            0
        } else {
            2
        };
    }

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
        print_cleanup_results(&results);
    }
    if success {
        0
    } else {
        2
    }
}

fn print_cleanup_results(results: &[CleanupResult]) {
    for result in results {
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
    let binary_exists = match fs::symlink_metadata(&binary) {
        Ok(metadata) if metadata.file_type().is_file() => true,
        Ok(_) => {
            return vec![preserved(
                "cargo_copy",
                Some(binary),
                "Cargo-path binary is a symlink or special file",
            )];
        }
        Err(error) if error.kind() == io::ErrorKind::NotFound => false,
        Err(error) => {
            return vec![preserved(
                "cargo_copy",
                Some(binary),
                &format!("Cargo-path binary metadata could not be inspected: {error}"),
            )];
        }
    };
    let receipt_exists = match receipt.as_deref() {
        Some(path) => match fs::symlink_metadata(path) {
            Ok(metadata) if metadata.file_type().is_file() => true,
            Ok(_) => {
                return vec![preserved(
                    "managed_receipt",
                    Some(path.to_path_buf()),
                    "managed receipt is a symlink or special file",
                )];
            }
            Err(error) if error.kind() == io::ErrorKind::NotFound => false,
            Err(error) => {
                return vec![preserved(
                    "managed_receipt",
                    Some(path.to_path_buf()),
                    &format!("managed receipt metadata could not be inspected: {error}"),
                )];
            }
        },
        None => false,
    };

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

    let manifest_paths = [
        cargo_home.join(".crates2.json"),
        cargo_home.join(".crates.toml"),
    ];
    let mut manifest_edits = Vec::new();
    for (index, manifest_path) in manifest_paths.iter().enumerate() {
        let edit = match fs::symlink_metadata(manifest_path) {
            Ok(metadata) if metadata.file_type().is_file() => {
                let manifest = match fs::read(manifest_path) {
                    Ok(manifest) => manifest,
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
                let parsed = if index == 0 {
                    cargo_manifest_edit(manifest_path, manifest)
                } else {
                    cargo_legacy_manifest_edit(manifest_path, manifest)
                };
                match parsed {
                    Ok(edit) => edit,
                    Err(error) => {
                        return vec![preserved(
                            "cargo_manifest_entry",
                            Some(manifest_path.clone()),
                            &format!("Cargo ownership metadata is ambiguous: {error}"),
                        )];
                    }
                }
            }
            Ok(_) => {
                return vec![preserved(
                    "cargo_manifest_entry",
                    Some(manifest_path.clone()),
                    "Cargo ownership metadata is a symlink or special file",
                )];
            }
            Err(error) if error.kind() == io::ErrorKind::NotFound => None,
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
        if let Some(edit) = edit {
            manifest_edits.push(edit);
        }
    }

    if manifest_edits.len() == 2 && manifest_edits[0].entry_key != manifest_edits[1].entry_key {
        return vec![preserved(
            "cargo_manifest_entry",
            Some(manifest_paths[1].clone()),
            &format!(
                "Cargo ownership metadata conflicts between {} ({}) and {} ({})",
                manifest_paths[0].display(),
                manifest_edits[0].entry_key,
                manifest_paths[1].display(),
                manifest_edits[1].entry_key
            ),
        )];
    }

    let legacy_manifest_proven = manifest_edits
        .iter()
        .any(|edit| edit.path == manifest_paths[1]);
    if !receipt_exists && !legacy_manifest_proven && !manifest_edits.is_empty() {
        return vec![preserved(
            "cargo_manifest_entry",
            Some(manifest_paths[1].clone()),
            "Cargo's authoritative .crates.toml does not prove ownership; refusing a v2-only takeover",
        )];
    }

    if !binary_exists && !receipt_exists {
        if manifest_edits.is_empty() {
            return vec![absent(
                "cargo_copy",
                Some(binary),
                "no Cargo-path copy, managed receipt, or exact Cargo ownership entry exists",
            )];
        }
        return vec![preserved(
            "cargo_copy",
            Some(binary),
            "Cargo records exact SD-300 ownership but the owned binary is missing; refusing an incomplete takeover",
        )];
    }

    if !receipt_exists && manifest_edits.is_empty() {
        return vec![preserved(
            "cargo_copy",
            Some(binary),
            &format!(
                "receipt-less Cargo-path binary is not owned by {CARGO_PACKAGE_NAME} in {} or {}",
                manifest_paths[0].display(),
                manifest_paths[1].display()
            ),
        )];
    }

    if args.dry_run {
        let mut results = vec![would_remove("cargo_copy", binary)];
        if let Some(receipt) = receipt.filter(|path| path.is_file()) {
            results.push(would_remove("managed_receipt", receipt));
        }
        for edit in &manifest_edits {
            results.push(would_remove_manifest_entry(edit));
        }
        return results;
    }

    let receipt_to_remove = receipt.as_deref().filter(|path| path.is_file());
    let outcome = match if args.msi_cargo_action == Some(MsiCargoAction::Prepare) {
        prepare_msi_cargo_transaction(
            args,
            &cargo_home,
            &binary,
            receipt_to_remove,
            &manifest_edits,
        )
    } else {
        commit_cargo_removal(&binary, receipt_to_remove, &manifest_edits)
    } {
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
    for edit in &manifest_edits {
        results.push(removed_manifest_entry(edit));
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
    let proven_version = crate::update::cargo_manifest_version(manifest)?;

    let mut json: serde_json::Value = serde_json::from_slice(&original)
        .map_err(|error| format!("Cargo's .crates2.json is invalid: {error}"))?;
    let installs = json
        .get_mut("installs")
        .and_then(serde_json::Value::as_object_mut)
        .ok_or_else(|| "Cargo's .crates2.json has no installs object".to_string())?;
    let binary_name = if cfg!(windows) { "sd300.exe" } else { "sd300" };
    let prefix = format!("{CARGO_PACKAGE_NAME} ");
    let foreign_owners = installs
        .iter()
        .filter_map(|(key, value)| {
            let owns_binary = value
                .get("bins")
                .and_then(serde_json::Value::as_array)
                .is_some_and(|bins| bins.iter().any(|bin| bin.as_str() == Some(binary_name)));
            (owns_binary && !key.starts_with(&prefix)).then(|| key.clone())
        })
        .collect::<Vec<_>>();
    if !foreign_owners.is_empty() {
        return Err(format!(
            "Cargo's .crates2.json also records foreign ownership of {binary_name}: {}",
            foreign_owners.join(", ")
        ));
    }
    if proven_version.is_none() {
        return Ok(None);
    }
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

fn cargo_legacy_manifest_edit(
    path: &Path,
    original: Vec<u8>,
) -> std::result::Result<Option<CargoManifestEdit>, String> {
    std::str::from_utf8(&original)
        .map_err(|error| format!("Cargo's .crates.toml is not UTF-8 TOML: {error}"))?;

    let mut offset = 0usize;
    let mut in_v1 = false;
    let mut saw_v1 = false;
    let mut matching = Vec::new();
    let mut foreign_owners = Vec::new();
    while offset < original.len() {
        let line_start = offset;
        let line_end = next_line_offset(&original, offset);
        let mut content_end = line_end;
        if content_end > line_start && original[content_end - 1] == b'\n' {
            content_end -= 1;
        }
        if content_end > line_start && original[content_end - 1] == b'\r' {
            content_end -= 1;
        }
        let mut trimmed = &original[line_start..content_end];
        if line_start == 0 {
            trimmed = trimmed.strip_prefix(b"\xef\xbb\xbf").unwrap_or(trimmed);
        }
        trimmed = trim_ascii_whitespace(trimmed);
        if trimmed.is_empty() || trimmed.starts_with(b"#") {
            offset = line_end;
            continue;
        }
        if trimmed.starts_with(b"[") {
            if trimmed == b"[v1]" {
                if saw_v1 {
                    return Err("Cargo's .crates.toml contains multiple [v1] tables".into());
                }
                saw_v1 = true;
                in_v1 = true;
            } else {
                if !trimmed.ends_with(b"]") {
                    return Err("Cargo's .crates.toml contains a malformed table header".into());
                }
                in_v1 = false;
            }
            offset = line_end;
            continue;
        }
        if !in_v1 {
            offset = line_end;
            continue;
        }

        let entry = parse_legacy_cargo_entry(&original, line_start)?;
        let entry_end = entry.end;
        if entry.bins.iter().any(|bin| bin == cargo_binary_name()) {
            if entry.key.starts_with(&format!("{CARGO_PACKAGE_NAME} ")) {
                if entry.bins.as_slice() != [cargo_binary_name()] {
                    return Err(format!(
                        "Cargo's .crates.toml entry {} owns additional binaries and cannot be removed as one SD-300 target",
                        entry.key
                    ));
                }
                matching.push(entry);
            } else {
                foreign_owners.push(entry.key.clone());
            }
        }
        offset = entry_end;
    }

    if !foreign_owners.is_empty() {
        return Err(format!(
            "Cargo's .crates.toml also records foreign ownership of {}: {}",
            cargo_binary_name(),
            foreign_owners.join(", ")
        ));
    }

    let [entry] = matching.as_slice() else {
        if matching.is_empty() {
            return Ok(None);
        }
        return Err(format!(
            "Cargo's .crates.toml records multiple {CARGO_PACKAGE_NAME} installations owning {}",
            cargo_binary_name()
        ));
    };
    let mut updated = Vec::with_capacity(original.len() - (entry.end - entry.start));
    updated.extend_from_slice(&original[..entry.start]);
    updated.extend_from_slice(&original[entry.end..]);
    Ok(Some(CargoManifestEdit {
        path: path.to_path_buf(),
        original,
        updated,
        entry_key: entry.key.clone(),
    }))
}

pub(crate) fn cargo_legacy_manifest_version(
    manifest: &str,
) -> std::result::Result<Option<String>, String> {
    cargo_legacy_manifest_package_id(manifest)?
        .map(|entry_key| {
            entry_key
                .strip_prefix(&format!("{CARGO_PACKAGE_NAME} "))
                .and_then(|remainder| remainder.split_once(" (").map(|(version, _)| version))
                .filter(|version| !version.is_empty())
                .map(str::to_string)
                .ok_or_else(|| {
                    "Cargo's .crates.toml ownership entry has no exact package version".to_string()
                })
        })
        .transpose()
}

pub(crate) fn cargo_legacy_manifest_package_id(
    manifest: &str,
) -> std::result::Result<Option<String>, String> {
    cargo_legacy_manifest_edit(Path::new(".crates.toml"), manifest.as_bytes().to_vec())
        .map(|edit| edit.map(|edit| edit.entry_key))
}

#[derive(Debug)]
struct LegacyCargoEntry {
    start: usize,
    end: usize,
    key: String,
    bins: Vec<String>,
}

fn parse_legacy_cargo_entry(
    manifest: &[u8],
    line_start: usize,
) -> std::result::Result<LegacyCargoEntry, String> {
    let mut cursor = line_start;
    while manifest
        .get(cursor)
        .is_some_and(|byte| matches!(byte, b' ' | b'\t'))
    {
        cursor += 1;
    }
    let key = parse_toml_basic_string(manifest, &mut cursor)?;
    skip_toml_space(manifest, &mut cursor);
    if manifest.get(cursor) != Some(&b'=') {
        return Err("Cargo's .crates.toml [v1] entry is missing '='".into());
    }
    cursor += 1;
    skip_toml_space_and_comments(manifest, &mut cursor);
    if manifest.get(cursor) != Some(&b'[') {
        return Err("Cargo's .crates.toml [v1] entry does not contain a binary array".into());
    }
    cursor += 1;
    let mut bins = Vec::new();
    loop {
        skip_toml_space_and_comments(manifest, &mut cursor);
        if manifest.get(cursor) == Some(&b']') {
            cursor += 1;
            break;
        }
        bins.push(parse_toml_basic_string(manifest, &mut cursor)?);
        skip_toml_space_and_comments(manifest, &mut cursor);
        match manifest.get(cursor) {
            Some(b',') => cursor += 1,
            Some(b']') => {
                cursor += 1;
                break;
            }
            _ => {
                return Err("Cargo's .crates.toml binary array has an unexpected value".into());
            }
        }
    }
    skip_toml_space(manifest, &mut cursor);
    if manifest.get(cursor) == Some(&b'#') {
        while manifest.get(cursor).is_some_and(|byte| *byte != b'\n') {
            cursor += 1;
        }
    }
    match manifest.get(cursor) {
        Some(b'\r') if manifest.get(cursor + 1) == Some(&b'\n') => cursor += 2,
        Some(b'\n') => cursor += 1,
        None => {}
        _ => {
            return Err("Cargo's .crates.toml [v1] entry has trailing data".into());
        }
    }
    Ok(LegacyCargoEntry {
        start: line_start,
        end: cursor,
        key,
        bins,
    })
}

fn parse_toml_basic_string(
    bytes: &[u8],
    cursor: &mut usize,
) -> std::result::Result<String, String> {
    if bytes.get(*cursor) != Some(&b'"') {
        return Err("Cargo's .crates.toml [v1] keys and binaries must be quoted".into());
    }
    *cursor += 1;
    let mut result = String::new();
    while let Some(&byte) = bytes.get(*cursor) {
        *cursor += 1;
        match byte {
            b'"' => return Ok(result),
            b'\\' => {
                let escaped = *bytes.get(*cursor).ok_or_else(|| {
                    "Cargo's .crates.toml contains an incomplete string escape".to_string()
                })?;
                *cursor += 1;
                result.push(match escaped {
                    b'"' => '"',
                    b'\\' => '\\',
                    b'b' => '\u{0008}',
                    b't' => '\t',
                    b'n' => '\n',
                    b'f' => '\u{000c}',
                    b'r' => '\r',
                    _ => {
                        return Err(
                            "Cargo's .crates.toml contains an unsupported string escape".into()
                        );
                    }
                });
            }
            b'\n' | b'\r' | 0..=0x1f => {
                return Err("Cargo's .crates.toml contains an invalid basic string".into());
            }
            _ if byte.is_ascii() => result.push(char::from(byte)),
            _ => {
                let start = *cursor - 1;
                let remainder = std::str::from_utf8(&bytes[start..])
                    .map_err(|error| format!("Cargo's .crates.toml is not UTF-8: {error}"))?;
                let character = remainder.chars().next().ok_or_else(|| {
                    "Cargo's .crates.toml contains an incomplete UTF-8 string".to_string()
                })?;
                *cursor = start + character.len_utf8();
                result.push(character);
            }
        }
    }
    Err("Cargo's .crates.toml contains an unterminated string".into())
}

fn skip_toml_space(bytes: &[u8], cursor: &mut usize) {
    while bytes
        .get(*cursor)
        .is_some_and(|byte| matches!(byte, b' ' | b'\t'))
    {
        *cursor += 1;
    }
}

fn skip_toml_space_and_comments(bytes: &[u8], cursor: &mut usize) {
    loop {
        while bytes
            .get(*cursor)
            .is_some_and(|byte| matches!(byte, b' ' | b'\t' | b'\r' | b'\n'))
        {
            *cursor += 1;
        }
        if bytes.get(*cursor) != Some(&b'#') {
            break;
        }
        while bytes.get(*cursor).is_some_and(|byte| *byte != b'\n') {
            *cursor += 1;
        }
    }
}

fn next_line_offset(bytes: &[u8], start: usize) -> usize {
    bytes[start..]
        .iter()
        .position(|byte| *byte == b'\n')
        .map_or(bytes.len(), |position| start + position + 1)
}

fn trim_ascii_whitespace(mut bytes: &[u8]) -> &[u8] {
    while bytes.first().is_some_and(u8::is_ascii_whitespace) {
        bytes = &bytes[1..];
    }
    while bytes.last().is_some_and(u8::is_ascii_whitespace) {
        bytes = &bytes[..bytes.len() - 1];
    }
    bytes
}

fn cargo_binary_name() -> &'static str {
    if cfg!(windows) {
        "sd300.exe"
    } else {
        "sd300"
    }
}

fn prepare_msi_cargo_transaction(
    args: &MigrateArgs,
    cargo_home: &Path,
    binary: &Path,
    receipt: Option<&Path>,
    manifests: &[CargoManifestEdit],
) -> std::result::Result<CargoRemovalOutcome, String> {
    let (explicit_cargo_home, user_profile, journal_path) = msi_cargo_paths(args)?;
    if !same_path(cargo_home, &explicit_cargo_home) {
        return Err("the proven Cargo home does not match MSI CustomActionData".into());
    }
    recover_stale_msi_cargo_transaction(args)?;

    let binary_backup = unused_adjacent_path(binary, "msi-binary-backup")?;
    let receipt_entry = receipt
        .map(|target| {
            Ok::<MsiCargoStagedFile, String>(MsiCargoStagedFile {
                target: target.to_path_buf(),
                backup: unused_adjacent_path(target, "msi-receipt-backup")?,
                original_sha256: sha256_file(target)?,
            })
        })
        .transpose()?;
    let mut manifest_entries = Vec::with_capacity(manifests.len());
    for edit in manifests {
        let metadata = fs::metadata(&edit.path).map_err(|error| {
            format!(
                "could not inspect {} before MSI transaction preparation: {error}",
                edit.path.display()
            )
        })?;
        let attributes = PreservedFileAttributes::from_metadata(&metadata);
        let replacement = write_adjacent_file(
            &edit.path,
            "msi-manifest-replacement",
            &edit.updated,
            Some(&attributes),
        )
        .map_err(|error| {
            format!(
                "could not prepare the MSI replacement for {}: {error}",
                edit.path.display()
            )
        })?;
        manifest_entries.push(MsiCargoManifestFile {
            target: edit.path.clone(),
            backup: unused_adjacent_path(&edit.path, "msi-manifest-backup")?,
            replacement,
            original_sha256: sha256_bytes(&edit.original),
            updated_sha256: sha256_bytes(&edit.updated),
        });
    }
    let journal = MsiCargoJournal {
        schema_version: MSI_CARGO_JOURNAL_SCHEMA,
        cargo_home: explicit_cargo_home,
        user_profile,
        binary: MsiCargoStagedFile {
            target: binary.to_path_buf(),
            backup: binary_backup,
            original_sha256: sha256_file(binary)?,
        },
        receipt: receipt_entry,
        manifests: manifest_entries,
    };
    validate_msi_cargo_journal(args, &journal)?;
    write_new_msi_journal(&journal_path, &journal)?;

    let result = apply_msi_cargo_transaction(&journal, |_| Ok(()));
    if let Err(cause) = result {
        let rollback = rollback_msi_cargo_transaction_inner(args);
        return match rollback {
            Ok(_) => Err(format!("{cause}; MSI Cargo rollback succeeded")),
            Err(rollback_error) => Err(format!(
                "{cause}; MSI Cargo rollback failed: {rollback_error}; recovery material remains at {}",
                journal_path.display()
            )),
        };
    }
    Ok(CargoRemovalOutcome::default())
}

fn apply_msi_cargo_transaction<F>(
    journal: &MsiCargoJournal,
    mut checkpoint: F,
) -> std::result::Result<(), String>
where
    F: FnMut(MsiCargoCheckpoint) -> io::Result<()>,
{
    checkpoint(MsiCargoCheckpoint::JournalWritten)
        .map_err(|error| format!("journal checkpoint failed: {error}"))?;
    for (index, manifest) in journal.manifests.iter().enumerate() {
        require_regular_hash(
            &manifest.target,
            &manifest.original_sha256,
            "Cargo manifest changed after ownership proof",
        )?;
        fs::rename(&manifest.target, &manifest.backup).map_err(|error| {
            format!(
                "could not stage original Cargo manifest {}: {error}",
                manifest.target.display()
            )
        })?;
        if let Err(error) = fs::rename(&manifest.replacement, &manifest.target) {
            let restore = fs::rename(&manifest.backup, &manifest.target);
            return Err(match restore {
                Ok(()) => format!(
                    "could not install updated Cargo manifest {}: {error}; original restored",
                    manifest.target.display()
                ),
                Err(restore_error) => format!(
                    "could not install updated Cargo manifest {}: {error}; immediate restore also failed: {restore_error}",
                    manifest.target.display()
                ),
            });
        }
        checkpoint(MsiCargoCheckpoint::ManifestReplaced(index)).map_err(|error| {
            format!(
                "manifest {} checkpoint failed: {error}",
                manifest.target.display()
            )
        })?;
    }
    if let Some(receipt) = &journal.receipt {
        require_regular_hash(
            &receipt.target,
            &receipt.original_sha256,
            "managed receipt changed after ownership proof",
        )?;
        fs::rename(&receipt.target, &receipt.backup).map_err(|error| {
            format!(
                "could not stage managed receipt {}: {error}",
                receipt.target.display()
            )
        })?;
        checkpoint(MsiCargoCheckpoint::ReceiptStaged)
            .map_err(|error| format!("receipt checkpoint failed: {error}"))?;
    }
    require_regular_hash(
        &journal.binary.target,
        &journal.binary.original_sha256,
        "Cargo binary changed after ownership proof",
    )?;
    fs::rename(&journal.binary.target, &journal.binary.backup).map_err(|error| {
        format!(
            "could not stage Cargo binary {}: {error}",
            journal.binary.target.display()
        )
    })?;
    checkpoint(MsiCargoCheckpoint::BinaryStaged)
        .map_err(|error| format!("binary checkpoint failed: {error}"))?;
    Ok(())
}

fn rollback_msi_cargo_transaction(args: &MigrateArgs) -> CleanupResult {
    match rollback_msi_cargo_transaction_inner(args) {
        Ok(true) => removed(
            "cargo_msi_transaction",
            args.msi_cargo_journal.clone().unwrap_or_default(),
        ),
        Ok(false) => absent(
            "cargo_msi_transaction",
            args.msi_cargo_journal.clone(),
            "no prepared MSI Cargo transaction exists",
        ),
        Err(error) => failure(
            "cargo_msi_transaction",
            args.msi_cargo_journal.clone(),
            &error,
        ),
    }
}

fn rollback_msi_cargo_transaction_inner(args: &MigrateArgs) -> std::result::Result<bool, String> {
    let (_, _, journal_path) = msi_cargo_paths(args)?;
    let marker_path = msi_commit_marker_path(&journal_path)?;
    let Some((journal_bytes, journal)) = read_msi_journal(args)? else {
        if marker_path.exists() {
            remove_regular_file(&marker_path)?;
        }
        return Ok(false);
    };
    if commit_marker_matches(&marker_path, &journal_bytes)? {
        cleanup_committed_msi_transaction(&journal_path, &marker_path, &journal)?;
        return Ok(false);
    }
    preflight_msi_rollback(&journal)?;

    restore_staged_file(&journal.binary)?;
    if let Some(receipt) = &journal.receipt {
        restore_staged_file(receipt)?;
    }
    for manifest in journal.manifests.iter().rev() {
        restore_manifest_file(manifest)?;
    }
    for manifest in &journal.manifests {
        remove_regular_file_if_exists(&manifest.replacement)?;
    }
    remove_regular_file(&journal_path)?;
    remove_regular_file_if_exists(&marker_path)?;
    Ok(true)
}

fn commit_msi_cargo_transaction(args: &MigrateArgs) -> CleanupResult {
    match commit_msi_cargo_transaction_inner(args) {
        Ok(true) => removed(
            "cargo_msi_transaction",
            args.msi_cargo_journal.clone().unwrap_or_default(),
        ),
        Ok(false) => absent(
            "cargo_msi_transaction",
            args.msi_cargo_journal.clone(),
            "no prepared MSI Cargo transaction exists",
        ),
        Err(error) => failure(
            "cargo_msi_transaction",
            args.msi_cargo_journal.clone(),
            &error,
        ),
    }
}

fn commit_msi_cargo_transaction_inner(args: &MigrateArgs) -> std::result::Result<bool, String> {
    let (_, _, journal_path) = msi_cargo_paths(args)?;
    let marker_path = msi_commit_marker_path(&journal_path)?;
    let Some((journal_bytes, journal)) = read_msi_journal(args)? else {
        if marker_path.exists() {
            remove_regular_file(&marker_path)?;
        }
        return Ok(false);
    };
    if commit_marker_matches(&marker_path, &journal_bytes)? {
        cleanup_committed_msi_transaction(&journal_path, &marker_path, &journal)?;
        return Ok(true);
    }
    preflight_msi_commit(&journal, false)?;
    write_commit_marker(&marker_path, &journal_bytes)?;
    cleanup_committed_msi_transaction(&journal_path, &marker_path, &journal)?;
    Ok(true)
}

fn recover_stale_msi_cargo_transaction(args: &MigrateArgs) -> std::result::Result<(), String> {
    let (_, _, journal_path) = msi_cargo_paths(args)?;
    let marker_path = msi_commit_marker_path(&journal_path)?;
    if !journal_path.exists() {
        if marker_path.exists() {
            remove_regular_file(&marker_path)?;
        }
        return Ok(());
    }
    let Some((journal_bytes, journal)) = read_msi_journal(args)? else {
        return Ok(());
    };
    if commit_marker_matches(&marker_path, &journal_bytes)? {
        cleanup_committed_msi_transaction(&journal_path, &marker_path, &journal)
    } else {
        rollback_msi_cargo_transaction_inner(args).map(|_| ())
    }
}

fn preflight_msi_rollback(journal: &MsiCargoJournal) -> std::result::Result<(), String> {
    preflight_staged_rollback(&journal.binary, "Cargo binary")?;
    if let Some(receipt) = &journal.receipt {
        preflight_staged_rollback(receipt, "managed receipt")?;
    }
    for manifest in &journal.manifests {
        match regular_file_hash(&manifest.backup)? {
            Some(hash) if hash == manifest.original_sha256 => {
                match regular_file_hash(&manifest.target)? {
                    Some(target_hash) if target_hash == manifest.updated_sha256 => {}
                    None => {}
                    Some(_) => {
                        return Err(format!(
                            "refusing to overwrite concurrently changed Cargo metadata at {}",
                            manifest.target.display()
                        ));
                    }
                }
            }
            None => match regular_file_hash(&manifest.target)? {
                Some(hash) if hash == manifest.original_sha256 => {}
                _ => {
                    return Err(format!(
                        "Cargo metadata recovery material is incomplete for {}",
                        manifest.target.display()
                    ));
                }
            },
            Some(_) => {
                return Err(format!(
                    "Cargo metadata backup was modified at {}",
                    manifest.backup.display()
                ));
            }
        }
    }
    Ok(())
}

fn preflight_staged_rollback(
    file: &MsiCargoStagedFile,
    label: &str,
) -> std::result::Result<(), String> {
    match regular_file_hash(&file.backup)? {
        Some(hash) if hash == file.original_sha256 => {
            if regular_file_hash(&file.target)?.is_some() {
                return Err(format!(
                    "refusing to overwrite a concurrent {label} replacement at {}",
                    file.target.display()
                ));
            }
        }
        None => match regular_file_hash(&file.target)? {
            Some(hash) if hash == file.original_sha256 => {}
            _ => {
                return Err(format!(
                    "{label} recovery material is incomplete for {}",
                    file.target.display()
                ));
            }
        },
        Some(_) => {
            return Err(format!(
                "{label} backup was modified at {}",
                file.backup.display()
            ));
        }
    }
    Ok(())
}

fn preflight_msi_commit(
    journal: &MsiCargoJournal,
    allow_missing_backups: bool,
) -> std::result::Result<(), String> {
    preflight_staged_commit(&journal.binary, "Cargo binary", allow_missing_backups)?;
    if let Some(receipt) = &journal.receipt {
        preflight_staged_commit(receipt, "managed receipt", allow_missing_backups)?;
    }
    for manifest in &journal.manifests {
        require_regular_hash(
            &manifest.target,
            &manifest.updated_sha256,
            "Cargo metadata changed before MSI commit",
        )?;
        match regular_file_hash(&manifest.backup)? {
            Some(hash) if hash == manifest.original_sha256 => {}
            None if allow_missing_backups => {}
            None => {
                return Err(format!(
                    "Cargo metadata backup is missing before commit: {}",
                    manifest.backup.display()
                ));
            }
            Some(_) => {
                return Err(format!(
                    "Cargo metadata backup changed before commit: {}",
                    manifest.backup.display()
                ));
            }
        }
    }
    Ok(())
}

fn preflight_staged_commit(
    file: &MsiCargoStagedFile,
    label: &str,
    allow_missing_backup: bool,
) -> std::result::Result<(), String> {
    if regular_file_hash(&file.target)?.is_some() {
        return Err(format!(
            "{label} reappeared before MSI commit at {}",
            file.target.display()
        ));
    }
    match regular_file_hash(&file.backup)? {
        Some(hash) if hash == file.original_sha256 => Ok(()),
        None if allow_missing_backup => Ok(()),
        None => Err(format!(
            "{label} backup is missing before commit: {}",
            file.backup.display()
        )),
        Some(_) => Err(format!(
            "{label} backup changed before commit: {}",
            file.backup.display()
        )),
    }
}

fn restore_staged_file(file: &MsiCargoStagedFile) -> std::result::Result<(), String> {
    if file.backup.exists() {
        fs::rename(&file.backup, &file.target).map_err(|error| {
            format!(
                "could not restore {} from {}: {error}",
                file.target.display(),
                file.backup.display()
            )
        })?;
    }
    Ok(())
}

fn restore_manifest_file(file: &MsiCargoManifestFile) -> std::result::Result<(), String> {
    if !file.backup.exists() {
        return Ok(());
    }
    if file.target.exists() {
        fs::remove_file(&file.target).map_err(|error| {
            format!(
                "could not remove updated Cargo metadata {} during rollback: {error}",
                file.target.display()
            )
        })?;
    }
    fs::rename(&file.backup, &file.target).map_err(|error| {
        format!(
            "could not restore original Cargo metadata {}: {error}",
            file.target.display()
        )
    })
}

fn cleanup_committed_msi_transaction(
    journal_path: &Path,
    marker_path: &Path,
    journal: &MsiCargoJournal,
) -> std::result::Result<(), String> {
    preflight_msi_commit(journal, true)?;
    let mut residue = Vec::new();
    for path in journal
        .manifests
        .iter()
        .flat_map(|manifest| [&manifest.backup, &manifest.replacement])
        .chain(std::iter::once(&journal.binary.backup))
        .chain(journal.receipt.iter().map(|receipt| &receipt.backup))
    {
        if let Err(error) = remove_regular_file_if_exists(path) {
            residue.push(format!("{}: {error}", path.display()));
        }
    }
    if !residue.is_empty() {
        return Err(format!(
            "committed MSI Cargo recovery residue remains: {}",
            residue.join("; ")
        ));
    }
    remove_regular_file(journal_path)?;
    remove_regular_file(marker_path)?;
    Ok(())
}

fn msi_cargo_paths(args: &MigrateArgs) -> std::result::Result<(PathBuf, PathBuf, PathBuf), String> {
    let explicit_cargo = args
        .cargo_home
        .as_ref()
        .filter(|path| !path.as_os_str().is_empty());
    let profile = args
        .user_profile
        .as_ref()
        .filter(|path| !path.as_os_str().is_empty())
        .ok_or_else(|| {
            "MSI CustomActionData did not include an invoking user profile".to_string()
        })?;
    let cargo_home = explicit_cargo
        .cloned()
        .unwrap_or_else(|| profile.join(".cargo"));
    let journal = args
        .msi_cargo_journal
        .as_ref()
        .filter(|path| !path.as_os_str().is_empty())
        .ok_or_else(|| {
            "MSI CustomActionData did not include a Cargo recovery journal".to_string()
        })?;
    if !cargo_home.is_absolute() || !profile.is_absolute() || !journal.is_absolute() {
        return Err("MSI Cargo transaction paths must be absolute".into());
    }
    Ok((cargo_home, profile.clone(), journal.clone()))
}

fn validate_msi_cargo_journal(
    args: &MigrateArgs,
    journal: &MsiCargoJournal,
) -> std::result::Result<(), String> {
    let (cargo_home, profile, journal_path) = msi_cargo_paths(args)?;
    if journal.schema_version != MSI_CARGO_JOURNAL_SCHEMA {
        return Err(format!(
            "unsupported MSI Cargo journal schema {}",
            journal.schema_version
        ));
    }
    if !same_path(&journal.cargo_home, &cargo_home) || !same_path(&journal.user_profile, &profile) {
        return Err("MSI Cargo journal identity does not match CustomActionData".into());
    }
    let expected_binary = cargo_home.join("bin").join(cargo_binary_name());
    if !same_path(&journal.binary.target, &expected_binary) {
        return Err("MSI Cargo journal names an unexpected binary target".into());
    }
    let expected_receipt = if cfg!(windows) {
        profile
            .join("AppData")
            .join("Local")
            .join(APP_NAME)
            .join("sd300-receipt.json")
    } else {
        profile
            .join(".config")
            .join(APP_NAME)
            .join("sd300-receipt.json")
    };
    if journal
        .receipt
        .as_ref()
        .is_some_and(|receipt| !same_path(&receipt.target, &expected_receipt))
    {
        return Err("MSI Cargo journal names an unexpected managed receipt".into());
    }
    if journal.manifests.len() > 2 {
        return Err("MSI Cargo journal contains too many manifest records".into());
    }
    let allowed_manifests = [
        cargo_home.join(".crates2.json"),
        cargo_home.join(".crates.toml"),
    ];
    for manifest in &journal.manifests {
        if !allowed_manifests
            .iter()
            .any(|allowed| same_path(&manifest.target, allowed))
        {
            return Err("MSI Cargo journal names an unexpected manifest target".into());
        }
    }
    let mut all_paths = vec![
        journal_path,
        journal.binary.target.clone(),
        journal.binary.backup.clone(),
    ];
    if let Some(receipt) = &journal.receipt {
        all_paths.extend([receipt.target.clone(), receipt.backup.clone()]);
    }
    for manifest in &journal.manifests {
        all_paths.extend([
            manifest.target.clone(),
            manifest.backup.clone(),
            manifest.replacement.clone(),
        ]);
    }
    for (index, path) in all_paths.iter().enumerate() {
        if all_paths[..index]
            .iter()
            .any(|other| same_path(path, other))
        {
            return Err(format!(
                "MSI Cargo journal reuses a transaction path: {}",
                path.display()
            ));
        }
    }
    for (target, recovery) in std::iter::once((&journal.binary.target, &journal.binary.backup))
        .chain(
            journal
                .receipt
                .iter()
                .map(|receipt| (&receipt.target, &receipt.backup)),
        )
        .chain(journal.manifests.iter().flat_map(|manifest| {
            [
                (&manifest.target, &manifest.backup),
                (&manifest.target, &manifest.replacement),
            ]
        }))
    {
        if recovery.parent() != target.parent()
            || !recovery
                .file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name.contains(".sd300-migrate-"))
        {
            return Err(format!(
                "MSI Cargo recovery path is not an owned adjacent file: {}",
                recovery.display()
            ));
        }
    }
    Ok(())
}

fn read_msi_journal(
    args: &MigrateArgs,
) -> std::result::Result<Option<(Vec<u8>, MsiCargoJournal)>, String> {
    let (_, _, path) = msi_cargo_paths(args)?;
    match fs::symlink_metadata(&path) {
        Ok(metadata) if metadata.file_type().is_file() => {}
        Ok(_) => return Err("MSI Cargo recovery journal is a symlink or special file".into()),
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(error) => {
            return Err(format!(
                "could not inspect MSI Cargo recovery journal {}: {error}",
                path.display()
            ));
        }
    }
    let bytes = fs::read(&path).map_err(|error| {
        format!(
            "could not read MSI Cargo recovery journal {}: {error}",
            path.display()
        )
    })?;
    let journal: MsiCargoJournal = serde_json::from_slice(&bytes)
        .map_err(|error| format!("MSI Cargo recovery journal is invalid: {error}"))?;
    validate_msi_cargo_journal(args, &journal)?;
    Ok(Some((bytes, journal)))
}

fn write_new_msi_journal(
    path: &Path,
    journal: &MsiCargoJournal,
) -> std::result::Result<(), String> {
    let parent = path
        .parent()
        .ok_or_else(|| "MSI Cargo journal has no parent directory".to_string())?;
    fs::create_dir_all(parent).map_err(|error| {
        format!(
            "could not create MSI Cargo journal directory {}: {error}",
            parent.display()
        )
    })?;
    let parent_metadata = fs::symlink_metadata(parent).map_err(|error| {
        format!(
            "could not inspect MSI Cargo journal directory {}: {error}",
            parent.display()
        )
    })?;
    if !parent_metadata.file_type().is_dir() {
        return Err("MSI Cargo journal parent is a symlink or special file".into());
    }
    let bytes = serde_json::to_vec(journal)
        .map_err(|error| format!("could not serialize MSI Cargo recovery journal: {error}"))?;
    let mut options = OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut file = options.open(path).map_err(|error| {
        format!(
            "could not reserve MSI Cargo recovery journal {}: {error}",
            path.display()
        )
    })?;
    if let Err(error) = file.write_all(&bytes).and_then(|_| file.sync_all()) {
        drop(file);
        let _ = fs::remove_file(path);
        return Err(format!(
            "could not persist MSI Cargo recovery journal {}: {error}",
            path.display()
        ));
    }
    Ok(())
}

fn msi_commit_marker_path(journal: &Path) -> std::result::Result<PathBuf, String> {
    let name = journal
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| "MSI Cargo journal filename is not valid Unicode".to_string())?;
    Ok(journal.with_file_name(format!("{name}.commit")))
}

fn write_commit_marker(path: &Path, journal_bytes: &[u8]) -> std::result::Result<(), String> {
    let expected = format!("{}\n", sha256_bytes(journal_bytes));
    match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.file_type().is_file() => {
            let current = fs::read_to_string(path)
                .map_err(|error| format!("could not read MSI Cargo commit marker: {error}"))?;
            if current == expected {
                return Ok(());
            }
            return Err("MSI Cargo commit marker does not match its journal".into());
        }
        Ok(_) => return Err("MSI Cargo commit marker is a symlink or special file".into()),
        Err(error) if error.kind() == io::ErrorKind::NotFound => {}
        Err(error) => {
            return Err(format!(
                "could not inspect MSI Cargo commit marker: {error}"
            ))
        }
    }
    let mut options = OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut file = options
        .open(path)
        .map_err(|error| format!("could not create MSI Cargo commit marker: {error}"))?;
    file.write_all(expected.as_bytes())
        .and_then(|_| file.sync_all())
        .map_err(|error| format!("could not persist MSI Cargo commit marker: {error}"))
}

fn commit_marker_matches(path: &Path, journal_bytes: &[u8]) -> std::result::Result<bool, String> {
    match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.file_type().is_file() => {
            let contents = fs::read_to_string(path)
                .map_err(|error| format!("could not read MSI Cargo commit marker: {error}"))?;
            let expected = format!("{}\n", sha256_bytes(journal_bytes));
            if contents != expected {
                return Err("MSI Cargo commit marker does not match its journal".into());
            }
            Ok(true)
        }
        Ok(_) => Err("MSI Cargo commit marker is a symlink or special file".into()),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(false),
        Err(error) => Err(format!(
            "could not inspect MSI Cargo commit marker: {error}"
        )),
    }
}

fn unused_adjacent_path(target: &Path, role: &str) -> std::result::Result<PathBuf, String> {
    for _ in 0..64 {
        let path = unique_adjacent_path(target, role);
        match fs::symlink_metadata(&path) {
            Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(path),
            Ok(_) => continue,
            Err(error) => {
                return Err(format!(
                    "could not inspect prospective recovery path {}: {error}",
                    path.display()
                ));
            }
        }
    }
    Err("could not reserve an unused adjacent MSI recovery path".into())
}

fn require_regular_hash(
    path: &Path,
    expected: &str,
    context: &str,
) -> std::result::Result<(), String> {
    match regular_file_hash(path)? {
        Some(actual) if actual == expected => Ok(()),
        Some(_) => Err(format!("{context}: {}", path.display())),
        None => Err(format!("{context}; file is missing: {}", path.display())),
    }
}

fn regular_file_hash(path: &Path) -> std::result::Result<Option<String>, String> {
    match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.file_type().is_file() => sha256_file(path).map(Some),
        Ok(_) => Err(format!("{} is a symlink or special file", path.display())),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(format!("could not inspect {}: {error}", path.display())),
    }
}

fn sha256_file(path: &Path) -> std::result::Result<String, String> {
    fs::read(path)
        .map(|bytes| sha256_bytes(&bytes))
        .map_err(|error| format!("could not hash {}: {error}", path.display()))
}

fn sha256_bytes(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn remove_regular_file(path: &Path) -> std::result::Result<(), String> {
    match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.file_type().is_file() => fs::remove_file(path)
            .map_err(|error| format!("could not remove {}: {error}", path.display())),
        Ok(_) => Err(format!(
            "refusing to remove a symlink or special file at {}",
            path.display()
        )),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(format!("could not inspect {}: {error}", path.display())),
    }
}

fn remove_regular_file_if_exists(path: &Path) -> std::result::Result<(), String> {
    remove_regular_file(path)
}

fn commit_cargo_removal(
    binary: &Path,
    receipt: Option<&Path>,
    manifests: &[CargoManifestEdit],
) -> std::result::Result<CargoRemovalOutcome, String> {
    commit_cargo_removal_with_checkpoint(binary, receipt, manifests, |_| Ok(()))
}

fn commit_cargo_removal_with_checkpoint<F>(
    binary: &Path,
    receipt: Option<&Path>,
    manifests: &[CargoManifestEdit],
    mut checkpoint: F,
) -> std::result::Result<CargoRemovalOutcome, String>
where
    F: FnMut(MigrationCheckpoint) -> io::Result<()>,
{
    let binary_sha256 = regular_file_hash(binary)?.ok_or_else(|| {
        format!(
            "Cargo binary disappeared before staging: {}",
            binary.display()
        )
    })?;
    let receipt_sha256 = receipt
        .map(|path| {
            regular_file_hash(path)?.ok_or_else(|| {
                format!(
                    "managed receipt disappeared before staging: {}",
                    path.display()
                )
            })
        })
        .transpose()?;
    let mut prepared_manifests = Vec::with_capacity(manifests.len());
    for edit in manifests {
        match prepare_manifest_edit(edit) {
            Ok(prepared) => prepared_manifests.push(prepared),
            Err(cause) => {
                let residue = prepared_manifests
                    .iter()
                    .flat_map(|prepared: &PreparedManifestEdit| {
                        [&prepared.backup, &prepared.replacement]
                    })
                    .filter_map(|path| match fs::remove_file(path) {
                        Ok(()) => None,
                        Err(error) if error.kind() == io::ErrorKind::NotFound => None,
                        Err(_) => Some(path.display().to_string()),
                    })
                    .collect::<Vec<_>>();
                if residue.is_empty() {
                    return Err(cause);
                }
                return Err(format!(
                    "{cause}; uncommitted recovery material was preserved at: {}",
                    residue.join(", ")
                ));
            }
        }
    }
    let mut state = CargoRemovalState::default();

    let operation = (|| -> std::result::Result<(), String> {
        for (edit, prepared) in manifests.iter().zip(&prepared_manifests) {
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
            state.manifests_replaced += 1;
            checkpoint(MigrationCheckpoint::ManifestReplaced)
                .map_err(|error| format!("metadata checkpoint failed: {error}"))?;
        }

        if let Some(receipt_path) = receipt {
            require_regular_hash(
                receipt_path,
                receipt_sha256
                    .as_deref()
                    .expect("a receipt hash exists when a receipt path exists"),
                "managed receipt changed after ownership proof",
            )?;
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

        require_regular_hash(
            binary,
            &binary_sha256,
            "Cargo binary changed after ownership proof",
        )?;
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
        let rollback_errors =
            rollback_cargo_removal(binary, receipt, manifests, &prepared_manifests, &state);
        if rollback_errors.is_empty() {
            cleanup_prepared_transaction(&state, &prepared_manifests);
            return Err(format!("{cause}; rollback succeeded"));
        }

        let recovery_paths = transaction_artifacts(&state, &prepared_manifests)
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

    let cleanup_residue = cleanup_prepared_transaction(&state, &prepared_manifests);
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
    manifests: &[CargoManifestEdit],
    prepared_manifests: &[PreparedManifestEdit],
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

    for (edit, prepared) in manifests
        .iter()
        .zip(prepared_manifests)
        .take(state.manifests_replaced)
        .rev()
    {
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
    prepared_manifests: &[PreparedManifestEdit],
) -> Vec<PathBuf> {
    let mut paths = Vec::new();
    if let Some(path) = &state.binary_staged {
        paths.push(path.clone());
    }
    if let Some(path) = &state.receipt_staged {
        paths.push(path.clone());
    }
    for prepared in prepared_manifests {
        paths.push(prepared.backup.clone());
        paths.push(prepared.replacement.clone());
    }
    paths
}

fn cleanup_prepared_transaction(
    state: &CargoRemovalState,
    prepared_manifests: &[PreparedManifestEdit],
) -> Vec<PathBuf> {
    transaction_artifacts(state, prepared_manifests)
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
        .filter(|path| !path.as_os_str().is_empty())
        .or_else(|| {
            args.user_profile
                .as_ref()
                .filter(|path| !path.as_os_str().is_empty())
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
        if let Some(profile) = args
            .user_profile
            .as_ref()
            .filter(|path| !path.as_os_str().is_empty())
        {
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
            .filter(|path| !path.as_os_str().is_empty())
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

    fn legacy_cargo_manifest(entry_version: &str) -> Vec<u8> {
        format!(
            concat!(
                "# Cargo install ownership\r\n",
                "[v1]\r\n",
                "\"another-tool 9.1.0 (registry+https://example.invalid/index)\" = [\r\n",
                "    \"another-tool\",\r\n",
                "]\r\n",
                "\"tr300-tui {} (registry+https://example.invalid/index)\" = [\"{}\"]\r\n",
                "\"tr300-tui 0.1.0 (path+file:///foreign)\" = [\"not-sd300\"]\r\n",
                "# retained footer\r\n"
            ),
            entry_version,
            cargo_binary_name()
        )
        .into_bytes()
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
        std::fs::write(
            cargo_home.join(".crates.toml"),
            legacy_cargo_manifest("1.4.3"),
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
    fn legacy_manifest_edit_preserves_unrelated_bytes_and_crlf() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join(".crates.toml");
        let original = legacy_cargo_manifest("2.0.6");
        let edit = cargo_legacy_manifest_edit(&path, original.clone())
            .unwrap()
            .expect("the exact legacy Cargo entry should be proven");
        let removed = format!(
            "\"tr300-tui 2.0.6 (registry+https://example.invalid/index)\" = [\"{}\"]\r\n",
            cargo_binary_name()
        );
        let expected = String::from_utf8(original)
            .unwrap()
            .replace(&removed, "")
            .into_bytes();

        assert_eq!(edit.updated, expected);
        assert!(edit.updated.windows(2).any(|bytes| bytes == b"\r\n"));
        assert!(String::from_utf8(edit.updated)
            .unwrap()
            .contains("# retained footer\r\n"));
    }

    #[test]
    fn legacy_only_cleanup_commits_binary_and_exact_manifest_edit() {
        let temp = tempfile::tempdir().unwrap();
        let cargo_home = temp.path().join("cargo");
        let binary = cargo_home.join("bin").join(cargo_binary_name());
        let manifest_path = cargo_home.join(".crates.toml");
        fs::create_dir_all(binary.parent().unwrap()).unwrap();
        fs::write(&binary, b"owned executable").unwrap();
        let original = legacy_cargo_manifest("2.0.6");
        let expected = cargo_legacy_manifest_edit(&manifest_path, original.clone())
            .unwrap()
            .unwrap()
            .updated;
        fs::write(&manifest_path, original).unwrap();

        let results = clean_cargo_pair(&MigrateArgs {
            cargo_copy: true,
            strict: true,
            cargo_home: Some(cargo_home),
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
        assert_eq!(fs::read(manifest_path).unwrap(), expected);
    }

    #[cfg(unix)]
    #[test]
    fn legacy_manifest_permissions_are_preserved() {
        use std::os::unix::fs::PermissionsExt;

        let temp = tempfile::tempdir().unwrap();
        let cargo_home = temp.path().join("cargo");
        let binary = cargo_home.join("bin").join(cargo_binary_name());
        let manifest_path = cargo_home.join(".crates.toml");
        fs::create_dir_all(binary.parent().unwrap()).unwrap();
        fs::write(&binary, b"owned executable").unwrap();
        fs::write(&manifest_path, legacy_cargo_manifest("2.0.6")).unwrap();
        fs::set_permissions(&manifest_path, fs::Permissions::from_mode(0o640)).unwrap();

        let results = clean_cargo_pair(&MigrateArgs {
            cargo_copy: true,
            strict: true,
            cargo_home: Some(cargo_home),
            user_profile: Some(temp.path().to_path_buf()),
            ..MigrateArgs::default()
        });

        assert!(results
            .iter()
            .all(|result| result.status == CleanupStatus::Removed));
        assert_eq!(
            fs::metadata(manifest_path).unwrap().permissions().mode() & 0o777,
            0o640
        );
    }

    #[test]
    fn legacy_entry_proves_real_host_when_crates2_has_only_unrelated_installs() {
        let temp = tempfile::tempdir().unwrap();
        let cargo_home = temp.path().join("cargo");
        let binary = cargo_home.join("bin").join(cargo_binary_name());
        let json_path = cargo_home.join(".crates2.json");
        let toml_path = cargo_home.join(".crates.toml");
        fs::create_dir_all(binary.parent().unwrap()).unwrap();
        fs::write(&binary, b"real v2.0.6 executable").unwrap();
        let unrelated_json = serde_json::to_vec_pretty(&serde_json::json!({
            "v1": 1,
            "installs": {
                "cargo-audit 0.22.2 (registry+https://github.com/rust-lang/crates.io-index)": {
                    "bins": [if cfg!(windows) { "cargo-audit.exe" } else { "cargo-audit" }]
                },
                "tr300 3.14.3 (registry+https://github.com/rust-lang/crates.io-index)": {
                    "bins": [if cfg!(windows) { "tr300.exe" } else { "tr300" }]
                }
            }
        }))
        .unwrap();
        let legacy = legacy_cargo_manifest("2.0.6");
        let expected_legacy = cargo_legacy_manifest_edit(&toml_path, legacy.clone())
            .unwrap()
            .unwrap()
            .updated;
        fs::write(&json_path, &unrelated_json).unwrap();
        fs::write(&toml_path, legacy).unwrap();

        let results = clean_cargo_pair(&MigrateArgs {
            cargo_copy: true,
            strict: true,
            cargo_home: Some(cargo_home),
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
        assert_eq!(fs::read(json_path).unwrap(), unrelated_json);
        assert_eq!(fs::read(toml_path).unwrap(), expected_legacy);
    }

    #[test]
    fn receipt_less_v2_only_ownership_fails_closed() {
        let temp = tempfile::tempdir().unwrap();
        let cargo_home = temp.path().join("cargo");
        let binary = cargo_home.join("bin").join(cargo_binary_name());
        let json_path = cargo_home.join(".crates2.json");
        fs::create_dir_all(binary.parent().unwrap()).unwrap();
        fs::write(&binary, b"inconsistent executable").unwrap();
        let json = cargo_manifest_with_other_install("2.0.6");
        fs::write(&json_path, &json).unwrap();

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
        assert_eq!(fs::read(binary).unwrap(), b"inconsistent executable");
        assert_eq!(fs::read(json_path).unwrap(), json);
    }

    #[test]
    fn exact_stale_manifest_with_missing_binary_fails_closed() {
        let temp = tempfile::tempdir().unwrap();
        let cargo_home = temp.path().join("cargo");
        let toml_path = cargo_home.join(".crates.toml");
        fs::create_dir_all(&cargo_home).unwrap();
        let toml = legacy_cargo_manifest("2.0.6");
        fs::write(&toml_path, &toml).unwrap();

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
        assert_eq!(fs::read(toml_path).unwrap(), toml);
    }

    #[test]
    fn both_cargo_manifests_are_removed_in_one_transaction() {
        let temp = tempfile::tempdir().unwrap();
        let cargo_home = temp.path().join("cargo");
        let binary = cargo_home.join("bin").join(cargo_binary_name());
        let json_path = cargo_home.join(".crates2.json");
        let toml_path = cargo_home.join(".crates.toml");
        fs::create_dir_all(binary.parent().unwrap()).unwrap();
        fs::write(&binary, b"owned executable").unwrap();
        fs::write(&json_path, cargo_manifest_with_other_install("2.0.6")).unwrap();
        fs::write(&toml_path, legacy_cargo_manifest("2.0.6")).unwrap();

        let results = clean_cargo_pair(&MigrateArgs {
            cargo_copy: true,
            strict: true,
            cargo_home: Some(cargo_home),
            user_profile: Some(temp.path().to_path_buf()),
            ..MigrateArgs::default()
        });

        assert_eq!(
            results
                .iter()
                .filter(|result| result.target == "cargo_manifest_entry")
                .count(),
            2,
            "unexpected cleanup results: {results:?}"
        );
        assert!(!binary.exists());
        assert!(!String::from_utf8(fs::read(json_path).unwrap())
            .unwrap()
            .contains("tr300-tui 2.0.6"));
        assert!(!String::from_utf8(fs::read(toml_path).unwrap())
            .unwrap()
            .contains("tr300-tui 2.0.6"));
    }

    #[test]
    fn conflicting_manifests_fail_closed_without_mutation() {
        let temp = tempfile::tempdir().unwrap();
        let cargo_home = temp.path().join("cargo");
        let binary = cargo_home.join("bin").join(cargo_binary_name());
        let json_path = cargo_home.join(".crates2.json");
        let toml_path = cargo_home.join(".crates.toml");
        fs::create_dir_all(binary.parent().unwrap()).unwrap();
        fs::write(&binary, b"owned executable").unwrap();
        let json = cargo_manifest_with_other_install("2.0.6");
        let toml = legacy_cargo_manifest("2.0.5");
        fs::write(&json_path, &json).unwrap();
        fs::write(&toml_path, &toml).unwrap();

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
        assert_eq!(fs::read(binary).unwrap(), b"owned executable");
        assert_eq!(fs::read(json_path).unwrap(), json);
        assert_eq!(fs::read(toml_path).unwrap(), toml);
    }

    #[test]
    fn ambiguous_legacy_manifest_fails_closed_without_mutation() {
        let temp = tempfile::tempdir().unwrap();
        let cargo_home = temp.path().join("cargo");
        let binary = cargo_home.join("bin").join(cargo_binary_name());
        let toml_path = cargo_home.join(".crates.toml");
        fs::create_dir_all(binary.parent().unwrap()).unwrap();
        fs::write(&binary, b"owned executable").unwrap();
        let mut toml = legacy_cargo_manifest("2.0.6");
        toml.extend_from_slice(
            format!(
                "\"tr300-tui 2.0.5 (registry+https://example.invalid/index)\" = [\"{}\"]\r\n",
                cargo_binary_name()
            )
            .as_bytes(),
        );
        fs::write(&toml_path, &toml).unwrap();

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
        assert_eq!(fs::read(binary).unwrap(), b"owned executable");
        assert_eq!(fs::read(toml_path).unwrap(), toml);
    }

    #[test]
    fn foreign_sd300_owner_in_either_manifest_fails_closed() {
        let temp = tempfile::tempdir().unwrap();
        let cargo_home = temp.path().join("cargo");
        let binary = cargo_home.join("bin").join(cargo_binary_name());
        let json_path = cargo_home.join(".crates2.json");
        let toml_path = cargo_home.join(".crates.toml");
        fs::create_dir_all(binary.parent().unwrap()).unwrap();
        fs::write(&binary, b"ambiguous executable").unwrap();
        let json = serde_json::to_vec(&serde_json::json!({
            "installs": {
                "foreign-tool 1.0.0 (registry+https://example.invalid/index)": {
                    "bins": [cargo_binary_name()]
                }
            }
        }))
        .unwrap();
        let toml = legacy_cargo_manifest("2.0.6");
        fs::write(&json_path, &json).unwrap();
        fs::write(&toml_path, &toml).unwrap();

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
        assert_eq!(fs::read(binary).unwrap(), b"ambiguous executable");
        assert_eq!(fs::read(json_path).unwrap(), json);
        assert_eq!(fs::read(toml_path).unwrap(), toml);
    }

    #[test]
    fn multi_binary_package_entry_is_never_deleted() {
        let temp = tempfile::tempdir().unwrap();
        let json_path = temp.path().join(".crates2.json");
        let toml_path = temp.path().join(".crates.toml");
        let json = serde_json::to_vec(&serde_json::json!({
            "installs": {
                "tr300-tui 2.0.6 (registry+https://example.invalid/index)": {
                    "bins": [cargo_binary_name(), "another-owned-binary"]
                }
            }
        }))
        .unwrap();
        let toml = format!(
            "[v1]\n\"tr300-tui 2.0.6 (registry+https://example.invalid/index)\" = [\"{}\", \"another-owned-binary\"]\n",
            cargo_binary_name()
        )
        .into_bytes();

        assert!(cargo_manifest_edit(&json_path, json).is_err());
        assert!(cargo_legacy_manifest_edit(&toml_path, toml).is_err());
    }

    #[test]
    fn committed_cargo_cleanup_removes_binary_and_manifest_ownership_together() {
        let temp = tempfile::tempdir().unwrap();
        let cargo_home = temp.path().join("cargo");
        let binary = cargo_home.join("bin").join(cargo_binary_name());
        let manifest_path = cargo_home.join(".crates2.json");
        let legacy_manifest_path = cargo_home.join(".crates.toml");
        fs::create_dir_all(binary.parent().unwrap()).unwrap();
        fs::write(&binary, b"owned executable").unwrap();
        fs::write(&manifest_path, cargo_manifest_with_other_install("2.0.6")).unwrap();
        fs::write(&legacy_manifest_path, legacy_cargo_manifest("2.0.6")).unwrap();

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
        assert!(!String::from_utf8(fs::read(legacy_manifest_path).unwrap())
            .unwrap()
            .contains("tr300-tui 2.0.6"));
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
        let legacy_manifest_path = cargo_home.join(".crates.toml");
        fs::create_dir_all(binary.parent().unwrap()).unwrap();
        let binary_contents = b"original executable bytes";
        let receipt_contents = br#"{"provider":{"source":"cargo-dist"}}"#;
        let manifest_contents = cargo_manifest_with_other_install("2.0.6");
        let legacy_manifest_contents = legacy_cargo_manifest("2.0.6");
        fs::write(&binary, binary_contents).unwrap();
        fs::write(&receipt, receipt_contents).unwrap();
        fs::write(&manifest_path, &manifest_contents).unwrap();
        fs::write(&legacy_manifest_path, &legacy_manifest_contents).unwrap();
        let edit = cargo_manifest_edit(&manifest_path, manifest_contents.clone())
            .unwrap()
            .unwrap();
        let legacy_edit =
            cargo_legacy_manifest_edit(&legacy_manifest_path, legacy_manifest_contents.clone())
                .unwrap()
                .unwrap();

        let result = commit_cargo_removal_with_checkpoint(
            &binary,
            Some(&receipt),
            &[edit, legacy_edit],
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
        assert_eq!(
            fs::read(&legacy_manifest_path).unwrap(),
            legacy_manifest_contents
        );
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

    fn msi_transaction_args(
        profile: &Path,
        cargo_home: &Path,
        journal: &Path,
        action: MsiCargoAction,
    ) -> MigrateArgs {
        MigrateArgs {
            cargo_copy: true,
            strict: true,
            cargo_home: Some(cargo_home.to_path_buf()),
            user_profile: Some(profile.to_path_buf()),
            msi_cargo_action: Some(action),
            msi_cargo_journal: Some(journal.to_path_buf()),
            ..MigrateArgs::default()
        }
    }

    fn create_msi_transaction_fixture(
        profile: &Path,
    ) -> (
        PathBuf,
        PathBuf,
        PathBuf,
        PathBuf,
        PathBuf,
        Vec<u8>,
        Vec<u8>,
    ) {
        let cargo_home = profile.join(".cargo");
        let binary = cargo_home.join("bin").join(cargo_binary_name());
        let v2_manifest = cargo_home.join(".crates2.json");
        let legacy_manifest = cargo_home.join(".crates.toml");
        let receipt = resolve_receipt_path(&MigrateArgs {
            user_profile: Some(profile.to_path_buf()),
            ..MigrateArgs::default()
        })
        .expect("the explicit profile determines the managed receipt path");

        fs::create_dir_all(binary.parent().unwrap()).unwrap();
        fs::create_dir_all(receipt.parent().unwrap()).unwrap();
        let v2_bytes = cargo_manifest_with_other_install("2.0.6");
        let legacy_bytes = legacy_cargo_manifest("2.0.6");
        fs::write(&binary, b"owned v2 executable").unwrap();
        fs::write(&v2_manifest, &v2_bytes).unwrap();
        fs::write(&legacy_manifest, &legacy_bytes).unwrap();
        fs::write(
            &receipt,
            serde_json::json!({
                "provider": { "source": "cargo-dist" },
                "source": { "app_name": APP_NAME },
                "install_prefix": cargo_home,
            })
            .to_string(),
        )
        .unwrap();

        (
            cargo_home,
            binary,
            receipt,
            v2_manifest,
            legacy_manifest,
            v2_bytes,
            legacy_bytes,
        )
    }

    #[test]
    fn msi_rollback_restores_binary_receipt_and_both_manifests_exactly() {
        let temp = tempfile::tempdir().unwrap();
        let profile = temp.path().join("profile");
        let journal = profile.join("transactions").join("cargo-journal.json");
        let (cargo_home, binary, receipt, v2_manifest, legacy_manifest, v2_bytes, legacy_bytes) =
            create_msi_transaction_fixture(&profile);
        let receipt_bytes = fs::read(&receipt).unwrap();

        let prepare =
            msi_transaction_args(&profile, &cargo_home, &journal, MsiCargoAction::Prepare);
        let results = clean_cargo_pair(&prepare);
        assert!(
            results
                .iter()
                .all(|result| result.status == CleanupStatus::Removed),
            "unexpected prepare results: {results:?}"
        );
        assert!(!binary.exists());
        assert!(!receipt.exists());
        assert!(journal.exists());
        assert!(!String::from_utf8(fs::read(&v2_manifest).unwrap())
            .unwrap()
            .contains("tr300-tui 2.0.6"));
        assert!(!String::from_utf8(fs::read(&legacy_manifest).unwrap())
            .unwrap()
            .contains("tr300-tui 2.0.6"));

        let rollback =
            msi_transaction_args(&profile, &cargo_home, &journal, MsiCargoAction::Rollback);
        assert!(rollback_msi_cargo_transaction_inner(&rollback).unwrap());
        assert_eq!(fs::read(&binary).unwrap(), b"owned v2 executable");
        assert_eq!(fs::read(&receipt).unwrap(), receipt_bytes);
        assert_eq!(fs::read(&v2_manifest).unwrap(), v2_bytes);
        assert_eq!(fs::read(&legacy_manifest).unwrap(), legacy_bytes);
        assert!(!journal.exists());
        assert!(!msi_commit_marker_path(&journal).unwrap().exists());
        assert!(fs::read_dir(&cargo_home).unwrap().all(|entry| {
            !entry
                .unwrap()
                .file_name()
                .to_string_lossy()
                .contains("sd300-migrate")
        }));
    }

    #[test]
    fn msi_commit_keeps_cargo_retired_and_clears_recovery_material() {
        let temp = tempfile::tempdir().unwrap();
        let profile = temp.path().join("profile");
        let journal = profile.join("transactions").join("cargo-journal.json");
        let (cargo_home, binary, receipt, v2_manifest, legacy_manifest, _, _) =
            create_msi_transaction_fixture(&profile);

        let prepare =
            msi_transaction_args(&profile, &cargo_home, &journal, MsiCargoAction::Prepare);
        let results = clean_cargo_pair(&prepare);
        assert!(
            results
                .iter()
                .all(|result| result.status == CleanupStatus::Removed),
            "unexpected prepare results: {results:?}"
        );

        let commit = msi_transaction_args(&profile, &cargo_home, &journal, MsiCargoAction::Commit);
        assert!(commit_msi_cargo_transaction_inner(&commit).unwrap());
        assert!(!binary.exists());
        assert!(!receipt.exists());
        assert!(!String::from_utf8(fs::read(v2_manifest).unwrap())
            .unwrap()
            .contains("tr300-tui 2.0.6"));
        assert!(!String::from_utf8(fs::read(legacy_manifest).unwrap())
            .unwrap()
            .contains("tr300-tui 2.0.6"));
        assert!(!journal.exists());
        assert!(!msi_commit_marker_path(&journal).unwrap().exists());
        assert!(!rollback_msi_cargo_transaction_inner(&commit).unwrap());
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

        let error = commit_cargo_removal(&binary, Some(&receipt), std::slice::from_ref(&edit))
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
    fn receipt_change_before_staging_aborts_and_rolls_back_manifests() {
        let temp = tempfile::tempdir().unwrap();
        let binary = temp.path().join(cargo_binary_name());
        let receipt = temp.path().join("receipt.json");
        let manifest_path = temp.path().join(".crates2.json");
        let manifest = cargo_manifest_with_other_install("2.0.6");
        fs::write(&binary, b"owned binary").unwrap();
        fs::write(&receipt, b"owned receipt").unwrap();
        fs::write(&manifest_path, &manifest).unwrap();
        let edit = cargo_manifest_edit(&manifest_path, manifest.clone())
            .unwrap()
            .unwrap();

        let error =
            commit_cargo_removal_with_checkpoint(&binary, Some(&receipt), &[edit], |checkpoint| {
                if checkpoint == MigrationCheckpoint::ManifestReplaced {
                    fs::write(&receipt, b"concurrent receipt")?;
                }
                Ok(())
            })
            .expect_err("a concurrently changed receipt must abort staging");

        assert!(error.contains("managed receipt changed"), "{error}");
        assert_eq!(fs::read(&binary).unwrap(), b"owned binary");
        assert_eq!(fs::read(&receipt).unwrap(), b"concurrent receipt");
        assert_eq!(fs::read(&manifest_path).unwrap(), manifest);
    }

    #[test]
    fn binary_change_before_staging_aborts_and_rolls_back_receipt_and_manifests() {
        let temp = tempfile::tempdir().unwrap();
        let binary = temp.path().join(cargo_binary_name());
        let receipt = temp.path().join("receipt.json");
        let manifest_path = temp.path().join(".crates2.json");
        let manifest = cargo_manifest_with_other_install("2.0.6");
        fs::write(&binary, b"owned binary").unwrap();
        fs::write(&receipt, b"owned receipt").unwrap();
        fs::write(&manifest_path, &manifest).unwrap();
        let edit = cargo_manifest_edit(&manifest_path, manifest.clone())
            .unwrap()
            .unwrap();

        let error =
            commit_cargo_removal_with_checkpoint(&binary, Some(&receipt), &[edit], |checkpoint| {
                if checkpoint == MigrationCheckpoint::ReceiptStaged {
                    fs::write(&binary, b"concurrent binary")?;
                }
                Ok(())
            })
            .expect_err("a concurrently changed binary must abort staging");

        assert!(error.contains("Cargo binary changed"), "{error}");
        assert_eq!(fs::read(&binary).unwrap(), b"concurrent binary");
        assert_eq!(fs::read(&receipt).unwrap(), b"owned receipt");
        assert_eq!(fs::read(&manifest_path).unwrap(), manifest);
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

    #[cfg(unix)]
    #[test]
    fn symlinked_receipt_is_never_followed_or_removed() {
        use std::os::unix::fs::symlink;

        let temp = tempfile::tempdir().unwrap();
        let profile = temp.path();
        let cargo_home = profile.join(".cargo");
        let binary = cargo_home.join("bin").join(cargo_binary_name());
        fs::create_dir_all(binary.parent().unwrap()).unwrap();
        fs::write(&binary, b"owned").unwrap();
        fs::write(
            cargo_home.join(".crates.toml"),
            legacy_cargo_manifest("2.0.6"),
        )
        .unwrap();
        let receipt = resolve_receipt_path(&MigrateArgs {
            user_profile: Some(profile.to_path_buf()),
            ..MigrateArgs::default()
        })
        .unwrap();
        fs::create_dir_all(receipt.parent().unwrap()).unwrap();
        let target = temp.path().join("foreign-receipt.json");
        fs::write(&target, b"foreign bytes").unwrap();
        symlink(&target, &receipt).unwrap();

        let results = clean_cargo_pair(&MigrateArgs {
            cargo_copy: true,
            strict: true,
            cargo_home: Some(cargo_home),
            user_profile: Some(profile.to_path_buf()),
            ..MigrateArgs::default()
        });

        assert!(results
            .iter()
            .any(|result| result.status == CleanupStatus::Preserved));
        assert_eq!(fs::read(binary).unwrap(), b"owned");
        assert_eq!(fs::read(target).unwrap(), b"foreign bytes");
        assert!(fs::symlink_metadata(receipt)
            .unwrap()
            .file_type()
            .is_symlink());
    }
}
