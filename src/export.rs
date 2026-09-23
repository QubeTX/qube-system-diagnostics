//! Explicit session exports. Both frontends use the same redaction and private writer.
use crate::{collectors::SystemSnapshot, report::DiagnosticReport};
use serde_json::json;
use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
    thread::{self, JoinHandle},
};

#[derive(Default, Clone, Copy)]
pub enum Kind {
    #[default]
    Snapshot,
    Capabilities,
}
impl Kind {
    pub fn label(self) -> &'static str {
        match self {
            Self::Snapshot => "redacted_snapshot",
            Self::Capabilities => "capabilities",
        }
    }
    fn stem(self) -> &'static str {
        match self {
            Self::Snapshot => "sd300-redacted-snapshot",
            Self::Capabilities => "sd300-capabilities",
        }
    }
}
#[derive(Default)]
pub struct Controller {
    worker: Option<JoinHandle<Result<PathBuf, String>>>,
    pub kind: Kind,
    notify: bool,
    pub result: Option<Result<PathBuf, String>>,
}
impl Controller {
    pub fn running(&self) -> bool {
        self.worker.is_some()
    }
    pub fn start(&mut self, snapshot: &SystemSnapshot, kind: Kind) -> bool {
        if self.running() {
            return false;
        }
        let report = DiagnosticReport::from_snapshot(snapshot, false);
        self.kind = kind;
        self.result = None;
        match thread::Builder::new()
            .name("sd300-export".into())
            .spawn(move || write_report(&report, kind, &crate::settings::reports_dir()?))
        {
            Ok(worker) => self.worker = Some(worker),
            Err(error) => {
                self.result = Some(Err(format!("Could not start export: {error}")));
                self.notify = true;
            }
        }
        true
    }
    pub fn poll(&mut self) -> bool {
        if self.worker.as_ref().is_some_and(|w| w.is_finished()) {
            self.result = Some(
                self.worker
                    .take()
                    .unwrap()
                    .join()
                    .unwrap_or_else(|_| Err("The export worker failed".into())),
            );
            return true;
        }
        // A creation failure is also a completed request.
        std::mem::take(&mut self.notify)
    }
    pub fn message(&self) -> String {
        if self.running() {
            return "Saving a redacted report; monitoring continues…".into();
        }
        match &self.result {
            Some(Ok(path)) => format!("Saved {}\n{}", self.kind.label(), path.display()),
            Some(Err(error)) => format!("Export failed: {error}"),
            None => "E saves a redacted snapshot. C saves capabilities and findings.".into(),
        }
    }
}
impl Drop for Controller {
    fn drop(&mut self) {
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}
fn write_report(
    report: &DiagnosticReport,
    kind: Kind,
    directory: &Path,
) -> Result<PathBuf, String> {
    ensure_export_directory(directory)?;
    let bytes = match kind {
        Kind::Snapshot => serde_json::to_vec_pretty(&report.as_schema(2)),
        Kind::Capabilities => serde_json::to_vec_pretty(&json!({
            "schema_version":2, "samples":report.samples, "findings":report.findings,
            "product":report.product, "product_version":report.product_version,
            "target_os":report.target_os, "target_arch":report.target_arch,
            "capabilities":report.capabilities, "warnings":report.warnings,
        })),
    }
    .map_err(|e| format!("Could not serialize the report: {e}"))?;
    let captured = crate::collectors::sampling::unix_ms();
    for suffix in 0..100u8 {
        let destination = directory.join(format!("{}-{captured}-{suffix}.json", kind.stem()));
        match write_export_atomically(&destination, &bytes) {
            Ok(()) => return Ok(destination),
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(e) => return Err(format!("Could not save report: {e}")),
        }
    }
    Err("Could not allocate a unique report filename".into())
}
fn ensure_export_directory(directory: &Path) -> Result<(), String> {
    match fs::symlink_metadata(directory) {
        Ok(metadata) if metadata.file_type().is_symlink() || !metadata.is_dir() => {
            return Err(format!(
                "report destination {} is not an owned directory and was preserved",
                directory.display()
            ));
        }
        Ok(_) => return restrict_export_directory(directory),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => {
            return Err(format!(
                "could not inspect report destination {}: {error}",
                directory.display()
            ));
        }
    }
    fs::create_dir_all(directory).map_err(|error| {
        format!(
            "could not create report destination {}: {error}",
            directory.display()
        )
    })?;
    restrict_export_directory(directory)
}

#[cfg(unix)]
fn restrict_export_directory(directory: &Path) -> Result<(), String> {
    use std::os::unix::fs::{MetadataExt, PermissionsExt};

    let effective_uid = unsafe { libc::geteuid() };
    for path in directory.parent().into_iter().chain([directory]) {
        let metadata = fs::symlink_metadata(path)
            .map_err(|error| format!("could not inspect {}: {error}", path.display()))?;
        if !metadata.is_dir()
            || metadata.file_type().is_symlink()
            || metadata.uid() != effective_uid
        {
            return Err(format!(
                "report destination component {} is not a same-user directory and was preserved",
                path.display()
            ));
        }
        fs::set_permissions(path, fs::Permissions::from_mode(0o700))
            .map_err(|error| format!("could not restrict {}: {error}", path.display()))?;
    }
    Ok(())
}

#[cfg(not(unix))]
fn restrict_export_directory(_directory: &Path) -> Result<(), String> {
    Ok(())
}

fn write_export_atomically(destination: &Path, bytes: &[u8]) -> std::io::Result<()> {
    let directory = destination
        .parent()
        .ok_or_else(|| std::io::Error::other("Report directory missing"))?;
    let mut file = tempfile::NamedTempFile::new_in(directory)?;
    file.write_all(bytes)?;
    file.write_all(b"\n")?;
    file.as_file().sync_all()?;
    // Never replace an existing export, including a symlink introduced during creation.
    file.persist_noclobber(destination).map_err(|e| e.error)?;
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[cfg(unix)]
    #[test]
    fn export_directory_and_report_are_private_to_the_current_user() {
        use std::os::unix::fs::PermissionsExt;

        let temporary = tempfile::tempdir().expect("temporary root");
        let application = temporary.path().join("sd300");
        let reports = application.join("reports");

        fs::create_dir(&application).expect("application directory");
        fs::set_permissions(&application, fs::Permissions::from_mode(0o755))
            .expect("relax application directory before the test");
        ensure_export_directory(&reports).expect("private reports directory");

        assert_eq!(
            fs::metadata(&application)
                .expect("application metadata")
                .permissions()
                .mode()
                & 0o777,
            0o700
        );
        assert_eq!(
            fs::metadata(&reports)
                .expect("reports metadata")
                .permissions()
                .mode()
                & 0o777,
            0o700
        );

        let report = reports.join("snapshot.json");
        write_export_atomically(&report, br#"{"redacted":true}"#).expect("private atomic report");
        assert_eq!(
            fs::metadata(report)
                .expect("report metadata")
                .permissions()
                .mode()
                & 0o777,
            0o600
        );
    }

    #[test]
    fn exports_preserve_session_results_and_never_overwrite_files() {
        let temporary = tempfile::tempdir().unwrap();
        let reports = temporary.path().join("sd300/reports");
        let mut snapshot = SystemSnapshot::default();
        snapshot.system.hostname = "private-fixture-host".into();
        snapshot.companion.sequence = 1;
        snapshot.companion.result = Some(
            crate::companion::parse_results(
                crate::companion::Action::Standard,
                "4.0.1",
                include_bytes!("companion-fixtures/nd300-4.0.1-partial.json"),
                Some(2),
                None,
            )
            .unwrap(),
        );
        let report = DiagnosticReport::from_snapshot(&snapshot, false);
        let path = write_report(&report, Kind::Snapshot, &reports).unwrap();
        let bytes = fs::read(&path).unwrap();
        let value: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(value["schema_version"], 2);
        assert!(value["companion_results"].is_object());
        assert!(!String::from_utf8_lossy(&bytes).contains("private-fixture-host"));
        assert!(write_export_atomically(&path, b"replacement").is_err());
        assert_eq!(fs::read(&path).unwrap(), bytes);
        let capabilities = write_report(&report, Kind::Capabilities, &reports).unwrap();
        assert_ne!(path, capabilities);
    }
}
