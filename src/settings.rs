//! Versioned desktop-companion settings.
//!
//! The terminal UI deliberately does not read this document: its chooser,
//! units, sorting, and session defaults remain exactly as they were before the
//! GUI existed. The `shared` namespace is reserved for future settings that
//! are explicitly introduced for both frontends.

use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

pub const SETTINGS_SCHEMA_VERSION: u32 = 1;
const SETTINGS_FILE: &str = "settings.json";
const MAX_SETTINGS_BYTES: u64 = 256 * 1024;

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
#[serde(default)]
pub struct SharedSettings {}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum AudienceMode {
    #[default]
    User,
    Technician,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum TemperatureUnit {
    #[default]
    Celsius,
    Fahrenheit,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum ChartDensity {
    Compact,
    #[default]
    Balanced,
    Comfortable,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default)]
pub struct GuiSettings {
    pub audience_mode: AudienceMode,
    pub temperature_unit: TemperatureUnit,
    pub tray_enabled: bool,
    pub launch_at_login: bool,
    pub reduced_motion: bool,
    pub chart_density: ChartDensity,
    pub last_section: u8,
}

impl Default for GuiSettings {
    fn default() -> Self {
        Self {
            audience_mode: AudienceMode::User,
            temperature_unit: TemperatureUnit::Celsius,
            tray_enabled: false,
            launch_at_login: false,
            reduced_motion: true,
            chart_density: ChartDensity::Balanced,
            last_section: 0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default)]
pub struct SettingsDocument {
    pub schema_version: u32,
    pub shared: SharedSettings,
    pub gui: GuiSettings,
}

impl Default for SettingsDocument {
    fn default() -> Self {
        Self {
            schema_version: SETTINGS_SCHEMA_VERSION,
            shared: SharedSettings::default(),
            gui: GuiSettings::default(),
        }
    }
}

pub fn read_json() -> Result<Vec<u8>, String> {
    let document = load_from_path(&settings_path()?)?;
    serde_json::to_vec(&document).map_err(|error| format!("could not serialize settings: {error}"))
}

pub fn write_json(bytes: &[u8]) -> Result<(), String> {
    if bytes.is_empty() || bytes.len() as u64 > MAX_SETTINGS_BYTES {
        return Err("settings input was empty or exceeded the 256 KiB limit".into());
    }
    let document: SettingsDocument = serde_json::from_slice(bytes)
        .map_err(|error| format!("settings JSON was invalid: {error}"))?;
    validate(&document)?;
    save_to_path(&settings_path()?, &document)
}

pub fn settings_path() -> Result<PathBuf, String> {
    #[cfg(windows)]
    {
        return std::env::var_os("APPDATA")
            .map(PathBuf::from)
            .map(|root| root.join("SD-300").join(SETTINGS_FILE))
            .ok_or_else(|| "APPDATA is unavailable for the current user".into());
    }
    #[cfg(target_os = "macos")]
    {
        return std::env::var_os("HOME")
            .map(PathBuf::from)
            .map(|home| {
                home.join("Library")
                    .join("Application Support")
                    .join("SD-300")
                    .join(SETTINGS_FILE)
            })
            .ok_or_else(|| "HOME is unavailable for the current user".into());
    }
    #[cfg(target_os = "linux")]
    {
        let root = std::env::var_os("XDG_CONFIG_HOME")
            .map(PathBuf::from)
            .or_else(|| {
                std::env::var_os("HOME")
                    .map(PathBuf::from)
                    .map(|home| home.join(".config"))
            })
            .ok_or_else(|| "neither XDG_CONFIG_HOME nor HOME is available".to_string())?;
        return Ok(root.join("sd300").join(SETTINGS_FILE));
    }
    #[allow(unreachable_code)]
    Err("settings are supported only on Windows, macOS, and Linux".into())
}

/// User-requested reports intentionally live beside, but not inside, the set
/// of files owned by uninstall. `remove_owned_gui_state` only removes the
/// exact settings/cache names above, so this directory and its exports survive
/// product removal as promised by the lifecycle contract.
pub fn reports_dir() -> Result<PathBuf, String> {
    settings_path()?
        .parent()
        .map(|parent| parent.join("reports"))
        .ok_or_else(|| "settings path had no parent directory".into())
}

pub fn set_launch_at_login(enabled: bool, start_hidden: bool) -> Result<(), String> {
    let executable = std::env::current_exe()
        .map_err(|error| format!("could not resolve the GUI executable: {error}"))?;
    set_launch_at_login_for(&executable, enabled, start_hidden)
}

/// Remove only per-user state whose ownership can be proven from its exact
/// filename or integration marker. Unknown files (including user exports) are
/// deliberately left in place and therefore keep the parent directory alive.
pub fn remove_owned_gui_state() -> Result<(), String> {
    set_launch_at_login(false, false)?;
    remove_settings_files_at(&settings_path()?)
}

fn remove_settings_files_at(path: &Path) -> Result<(), String> {
    let Some(parent) = path.parent() else {
        return Err("settings path had no parent directory".into());
    };
    let entries = match fs::read_dir(parent) {
        Ok(entries) => entries,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(error) => return Err(format!("could not inspect {}: {error}", parent.display())),
    };

    for entry in entries {
        let entry = entry.map_err(|error| {
            format!(
                "could not inspect an entry under {}: {error}",
                parent.display()
            )
        })?;
        let name = entry.file_name();
        let name = name.to_string_lossy();
        let owned = name == SETTINGS_FILE
            || (name.starts_with("settings.corrupt-") && name.ends_with(".json"))
            || (name.starts_with("settings.unsupported-") && name.ends_with(".json"))
            || (name.starts_with(".settings-") && name.ends_with(".tmp"));
        if !owned {
            continue;
        }

        let metadata = fs::symlink_metadata(entry.path()).map_err(|error| {
            format!(
                "could not inspect owned GUI state {}: {error}",
                entry.path().display()
            )
        })?;
        if !metadata.file_type().is_file() {
            return Err(format!(
                "owned GUI state path {} was not a regular file; it was preserved",
                entry.path().display()
            ));
        }
        fs::remove_file(entry.path()).map_err(|error| {
            format!(
                "could not remove owned GUI state {}: {error}",
                entry.path().display()
            )
        })?;
    }

    match fs::remove_dir(parent) {
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
            "could not remove the empty GUI settings directory {}: {error}",
            parent.display()
        )),
    }
}

#[cfg(windows)]
fn set_launch_at_login_for(
    executable: &Path,
    enabled: bool,
    start_hidden: bool,
) -> Result<(), String> {
    use winreg::enums::{HKEY_CURRENT_USER, KEY_READ, KEY_WRITE};
    use winreg::RegKey;

    const RUN_KEY: &str = r"Software\Microsoft\Windows\CurrentVersion\Run";
    const VALUE_NAME: &str = "SD-300";
    let root = RegKey::predef(HKEY_CURRENT_USER);
    let (run, _) = root
        .create_subkey_with_flags(RUN_KEY, KEY_READ | KEY_WRITE)
        .map_err(|error| format!("could not open the per-user startup key: {error}"))?;
    if !enabled {
        match run.get_value::<String, _>(VALUE_NAME) {
            Ok(existing) if windows_startup_command_is_owned(&existing, executable) => run
                .delete_value(VALUE_NAME)
                .map_err(|error| format!("could not remove SD-300 launch-at-login: {error}"))?,
            Ok(_) => return Err(
                "the SD-300 startup value exists but does not identify the GUI; it was preserved"
                    .into(),
            ),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
            Err(error) => return Err(format!("could not inspect launch-at-login: {error}")),
        }
        return Ok(());
    }
    let mut command = format!("\"{}\" --startup", executable.display());
    if start_hidden {
        command.push_str(" --hidden");
    }
    match run.get_value::<String, _>(VALUE_NAME) {
        Ok(existing) if existing == command => return Ok(()),
        Ok(existing) if windows_startup_command_is_owned(&existing, executable) => {}
        Ok(_) => {
            return Err(
                "the SD-300 startup value already exists but is not owned by this GUI; it was preserved"
                    .into(),
            )
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => return Err(format!("could not inspect launch-at-login: {error}")),
    }
    run.set_value(VALUE_NAME, &command)
        .map_err(|error| format!("could not enable launch-at-login: {error}"))?;
    let verified: String = run
        .get_value(VALUE_NAME)
        .map_err(|error| format!("could not verify launch-at-login: {error}"))?;
    if verified != command {
        return Err("the launch-at-login value did not verify byte-for-byte".into());
    }
    Ok(())
}

#[cfg(windows)]
fn windows_startup_command_is_owned(command: &str, executable: &Path) -> bool {
    let Some(remainder) = command.strip_prefix('"') else {
        return false;
    };
    let Some((command_path, arguments)) = remainder.split_once('"') else {
        return false;
    };
    let normalized_command_path = command_path.replace('/', "\\");
    let normalized_executable = executable.to_string_lossy().replace('/', "\\");
    if !normalized_command_path.eq_ignore_ascii_case(&normalized_executable) {
        return false;
    }
    matches!(arguments, " --startup" | " --startup --hidden")
}

#[cfg(target_os = "macos")]
fn set_launch_at_login_for(
    executable: &Path,
    enabled: bool,
    start_hidden: bool,
) -> Result<(), String> {
    let home = std::env::var_os("HOME")
        .map(PathBuf::from)
        .ok_or_else(|| "HOME is unavailable for launch-at-login".to_string())?;
    let directory = home.join("Library").join("LaunchAgents");
    let path = directory.join("dev.qubetx.sd300.plist");
    const MARKER: &str = "<!-- SD-300 managed launch-at-login -->";
    if !enabled {
        return remove_owned_text_file(&path, MARKER);
    }
    fs::create_dir_all(&directory)
        .map_err(|error| format!("could not create {}: {error}", directory.display()))?;
    let bundle = executable.ancestors().find(|candidate| {
        candidate
            .extension()
            .is_some_and(|extension| extension.eq_ignore_ascii_case("app"))
    });
    let mut arguments = Vec::new();
    if let Some(bundle) = bundle {
        arguments.push("/usr/bin/open".to_string());
        if start_hidden {
            arguments.push("-gj".to_string());
        }
        arguments.push(bundle.to_string_lossy().into_owned());
        arguments.push("--args".to_string());
        arguments.push("--startup".to_string());
        if start_hidden {
            arguments.push("--hidden".to_string());
        }
    } else {
        arguments.push(executable.to_string_lossy().into_owned());
        arguments.push("--startup".to_string());
    }
    let argument_xml = arguments
        .iter()
        .map(|argument| format!("    <string>{}</string>", xml_escape(argument)))
        .collect::<Vec<_>>()
        .join("\n");
    let contents = format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n{MARKER}\n<!DOCTYPE plist PUBLIC \"-//Apple//DTD PLIST 1.0//EN\" \"http://www.apple.com/DTDs/PropertyList-1.0.dtd\">\n<plist version=\"1.0\">\n<dict>\n  <key>Label</key><string>dev.qubetx.sd300</string>\n  <key>ProgramArguments</key>\n  <array>\n{argument_xml}\n  </array>\n  <key>RunAtLoad</key><true/>\n</dict>\n</plist>\n"
    );
    write_owned_text_file(&path, contents.as_bytes(), MARKER)
}

#[cfg(target_os = "linux")]
fn set_launch_at_login_for(
    executable: &Path,
    enabled: bool,
    _start_hidden: bool,
) -> Result<(), String> {
    let config_root = std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|| {
            std::env::var_os("HOME")
                .map(PathBuf::from)
                .map(|home| home.join(".config"))
        })
        .ok_or_else(|| "neither XDG_CONFIG_HOME nor HOME is available".to_string())?;
    let directory = config_root.join("autostart");
    let path = directory.join("sd300.desktop");
    const MARKER: &str = "# SD-300 managed launch-at-login";
    if !enabled {
        return remove_owned_text_file(&path, MARKER);
    }
    fs::create_dir_all(&directory)
        .map_err(|error| format!("could not create {}: {error}", directory.display()))?;
    let launcher = linux_gui_launcher(executable);
    let executable = desktop_exec_escape(&launcher.to_string_lossy());
    let contents = format!(
        "[Desktop Entry]\n{MARKER}\nType=Application\nName=SD-300\nComment=Open the SD-300 native system monitor\nExec=\"{executable}\" --startup\nTerminal=false\nX-GNOME-Autostart-enabled=true\n"
    );
    write_owned_text_file(&path, contents.as_bytes(), MARKER)
}

#[cfg(target_os = "linux")]
fn linux_gui_launcher(executable: &Path) -> PathBuf {
    let managed_launcher = executable
        .parent()
        .filter(|parent| parent.file_name().is_some_and(|name| name == "libexec"))
        .and_then(Path::parent)
        .map(|root| root.join("bin").join("sd300-gui"));
    managed_launcher
        .filter(|candidate| candidate.is_file())
        .unwrap_or_else(|| executable.to_path_buf())
}

#[cfg(not(any(windows, target_os = "macos", target_os = "linux")))]
fn set_launch_at_login_for(
    _executable: &Path,
    _enabled: bool,
    _start_hidden: bool,
) -> Result<(), String> {
    Err("launch-at-login is supported only on Windows, macOS, and Linux".into())
}

#[cfg(any(target_os = "macos", target_os = "linux"))]
fn remove_owned_text_file(path: &Path, marker: &str) -> Result<(), String> {
    match fs::symlink_metadata(path) {
        Ok(metadata) if !metadata.file_type().is_file() || metadata.file_type().is_symlink() => {
            return Err(format!(
                "{} exists but is not a regular SD-300-owned file; it was preserved",
                path.display()
            ));
        }
        Ok(_) => {}
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(error) => return Err(format!("could not inspect {}: {error}", path.display())),
    }
    let existing = match fs::read_to_string(path) {
        Ok(existing) => existing,
        Err(error) => return Err(format!("could not inspect {}: {error}", path.display())),
    };
    if !existing.contains(marker) {
        return Err(format!(
            "{} exists but is not SD-300-owned; it was preserved",
            path.display()
        ));
    }
    fs::remove_file(path).map_err(|error| format!("could not remove {}: {error}", path.display()))
}

#[cfg(any(target_os = "macos", target_os = "linux"))]
fn write_owned_text_file(path: &Path, bytes: &[u8], marker: &str) -> Result<(), String> {
    let parent = path
        .parent()
        .ok_or_else(|| "launch-at-login path had no parent".to_string())?;
    match fs::symlink_metadata(path) {
        Ok(metadata) => {
            if !metadata.file_type().is_file() || metadata.file_type().is_symlink() {
                return Err(format!(
                    "{} exists but is not a regular SD-300-owned file; it was preserved",
                    path.display()
                ));
            }
            let existing = fs::read(path)
                .map_err(|error| format!("could not inspect {}: {error}", path.display()))?;
            if existing == bytes {
                return Ok(());
            }
            if !String::from_utf8_lossy(&existing).contains(marker) {
                return Err(format!(
                    "{} exists but is not SD-300-owned; it was preserved",
                    path.display()
                ));
            }
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => return Err(format!("could not inspect {}: {error}", path.display())),
    }
    let temp = parent.join(format!(".sd300-startup-{}.tmp", std::process::id()));
    fs::write(&temp, bytes).map_err(|error| format!("could not stage launch-at-login: {error}"))?;
    if let Err(error) = fs::rename(&temp, path) {
        let _ = fs::remove_file(&temp);
        return Err(format!(
            "could not install launch-at-login atomically: {error}"
        ));
    }
    Ok(())
}

#[cfg(target_os = "macos")]
fn xml_escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

#[cfg(target_os = "linux")]
fn desktop_exec_escape(value: &str) -> String {
    value.replace('\\', "\\\\").replace('"', "\\\"")
}

fn validate(document: &SettingsDocument) -> Result<(), String> {
    if document.schema_version != SETTINGS_SCHEMA_VERSION {
        return Err(format!(
            "unsupported settings schema {}; expected {}",
            document.schema_version, SETTINGS_SCHEMA_VERSION
        ));
    }
    if document.gui.last_section > 8 {
        return Err("GUI last_section must be between 0 and 8".into());
    }
    Ok(())
}

fn load_from_path(path: &Path) -> Result<SettingsDocument, String> {
    let metadata = match fs::metadata(path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Ok(SettingsDocument::default())
        }
        Err(error) => return Err(format!("could not inspect {}: {error}", path.display())),
    };
    if metadata.len() > MAX_SETTINGS_BYTES {
        preserve_corrupt(path)?;
        return Ok(SettingsDocument::default());
    }
    let bytes =
        fs::read(path).map_err(|error| format!("could not read {}: {error}", path.display()))?;
    match serde_json::from_slice::<SettingsDocument>(&bytes) {
        Ok(document) if document.schema_version != SETTINGS_SCHEMA_VERSION => {
            preserve_settings(path, "unsupported")?;
            Ok(SettingsDocument::default())
        }
        Ok(document) => match validate(&document) {
            Ok(()) => Ok(document),
            Err(_) => {
                preserve_corrupt(path)?;
                Ok(SettingsDocument::default())
            }
        },
        Err(_) => {
            preserve_corrupt(path)?;
            Ok(SettingsDocument::default())
        }
    }
}

fn preserve_corrupt(path: &Path) -> Result<PathBuf, String> {
    preserve_settings(path, "corrupt")
}

fn preserve_settings(path: &Path, reason: &str) -> Result<PathBuf, String> {
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();
    let preserved = path.with_file_name(format!("settings.{reason}-{stamp}.json"));
    fs::rename(path, &preserved).map_err(|error| {
        format!(
            "settings were {reason} but could not be preserved as {}: {error}",
            preserved.display()
        )
    })?;
    Ok(preserved)
}

fn save_to_path(path: &Path, document: &SettingsDocument) -> Result<(), String> {
    validate(document)?;
    let parent = path
        .parent()
        .ok_or_else(|| "settings path had no parent directory".to_string())?;
    fs::create_dir_all(parent)
        .map_err(|error| format!("could not create {}: {error}", parent.display()))?;
    restrict_directory(parent)?;

    let bytes = serde_json::to_vec_pretty(document)
        .map_err(|error| format!("could not serialize settings: {error}"))?;
    let temp = parent.join(format!(
        ".settings-{}-{}.tmp",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos()
    ));
    let mut options = OpenOptions::new();
    options.write(true).create_new(true);
    restrict_file_options(&mut options);
    let mut file = options
        .open(&temp)
        .map_err(|error| format!("could not create settings staging file: {error}"))?;
    let write_result = (|| -> std::io::Result<()> {
        file.write_all(&bytes)?;
        file.write_all(b"\n")?;
        file.sync_all()
    })();
    if let Err(error) = write_result {
        let _ = fs::remove_file(&temp);
        return Err(format!("could not write settings staging file: {error}"));
    }
    drop(file);
    if let Err(error) = fs::rename(&temp, path) {
        let _ = fs::remove_file(&temp);
        return Err(format!("could not atomically replace settings: {error}"));
    }
    sync_parent(parent)?;
    Ok(())
}

#[cfg(unix)]
fn restrict_directory(path: &Path) -> Result<(), String> {
    use std::os::unix::fs::PermissionsExt;
    fs::set_permissions(path, fs::Permissions::from_mode(0o700))
        .map_err(|error| format!("could not restrict {}: {error}", path.display()))
}

#[cfg(not(unix))]
fn restrict_directory(_path: &Path) -> Result<(), String> {
    Ok(())
}

#[cfg(unix)]
fn restrict_file_options(options: &mut OpenOptions) {
    use std::os::unix::fs::OpenOptionsExt;
    options.mode(0o600);
}

#[cfg(not(unix))]
fn restrict_file_options(_options: &mut OpenOptions) {}

#[cfg(unix)]
fn sync_parent(path: &Path) -> Result<(), String> {
    fs::File::open(path)
        .and_then(|directory| directory.sync_all())
        .map_err(|error| format!("could not sync settings directory: {error}"))
}

#[cfg(not(unix))]
fn sync_parent(_path: &Path) -> Result<(), String> {
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_keep_terminal_state_out_of_the_gui_document() {
        let json = serde_json::to_value(SettingsDocument::default()).expect("serialize defaults");
        assert_eq!(json["schema_version"], SETTINGS_SCHEMA_VERSION);
        assert_eq!(json["shared"], serde_json::json!({}));
        assert_eq!(json["gui"]["audience_mode"], "user");
        assert_eq!(json["gui"]["tray_enabled"], false);
        assert!(json.get("tui").is_none());
    }

    #[test]
    fn writes_atomically_and_preserves_corrupt_input() {
        let temp = tempfile::tempdir().expect("tempdir");
        let path = temp.path().join("SD-300").join(SETTINGS_FILE);
        let mut document = SettingsDocument::default();
        document.gui.audience_mode = AudienceMode::Technician;
        document.gui.last_section = 6;
        save_to_path(&path, &document).expect("write settings");
        assert_eq!(load_from_path(&path).expect("read settings"), document);

        fs::write(&path, b"{not-json").expect("corrupt settings");
        assert_eq!(
            load_from_path(&path).expect("recover corrupt settings"),
            SettingsDocument::default()
        );
        let preserved = fs::read_dir(path.parent().expect("parent"))
            .expect("list parent")
            .filter_map(Result::ok)
            .any(|entry| {
                entry
                    .file_name()
                    .to_string_lossy()
                    .starts_with("settings.corrupt-")
            });
        assert!(preserved);
    }

    #[test]
    fn rejects_unknown_schema_and_out_of_range_section() {
        let mut document = SettingsDocument {
            schema_version: 2,
            ..SettingsDocument::default()
        };
        assert!(validate(&document).is_err());
        document.schema_version = SETTINGS_SCHEMA_VERSION;
        document.gui.last_section = 9;
        assert!(validate(&document).is_err());
    }

    #[test]
    fn load_preserves_newer_schema_before_returning_safe_defaults() {
        let temp = tempfile::tempdir().expect("tempdir");
        let path = temp.path().join("SD-300").join(SETTINGS_FILE);
        fs::create_dir_all(path.parent().expect("parent")).expect("settings directory");
        let mut document = SettingsDocument {
            schema_version: SETTINGS_SCHEMA_VERSION + 1,
            ..SettingsDocument::default()
        };
        document.gui.last_section = 4;
        fs::write(
            &path,
            serde_json::to_vec_pretty(&document).expect("serialize newer settings"),
        )
        .expect("write newer settings");

        assert_eq!(
            load_from_path(&path).expect("preserve newer settings"),
            SettingsDocument::default()
        );
        assert!(!path.exists());
        let preserved = fs::read_dir(path.parent().expect("parent"))
            .expect("list parent")
            .filter_map(Result::ok)
            .any(|entry| {
                entry
                    .file_name()
                    .to_string_lossy()
                    .starts_with("settings.unsupported-")
            });
        assert!(preserved);
    }

    #[cfg(windows)]
    #[test]
    fn windows_startup_ownership_requires_the_exact_gui_path_and_arguments() {
        let executable = Path::new(r"C:\Program Files\SD-300\sd300-gui.exe");
        assert!(windows_startup_command_is_owned(
            r#""C:\Program Files\SD-300\sd300-gui.exe" --startup"#,
            executable
        ));
        assert!(windows_startup_command_is_owned(
            r#""c:/program files/sd-300/sd300-gui.exe" --startup --hidden"#,
            executable
        ));
        assert!(!windows_startup_command_is_owned(
            r#""C:\Other\sd300-gui.exe" --startup"#,
            executable
        ));
        assert!(!windows_startup_command_is_owned(
            r#""C:\Program Files\SD-300\sd300-gui.exe" --startup --unsafe"#,
            executable
        ));
    }

    #[cfg(any(target_os = "macos", target_os = "linux"))]
    #[test]
    fn startup_file_replacement_preserves_ambiguous_existing_content() {
        let temp = tempfile::tempdir().expect("tempdir");
        let path = temp.path().join("startup-entry");
        fs::write(&path, b"user-owned").expect("write ambiguous entry");
        let error = write_owned_text_file(&path, b"# SD-300 managed\nnew", "# SD-300 managed")
            .expect_err("ambiguous entry must be preserved");
        assert!(error.contains("preserved"));
        assert_eq!(
            fs::read(&path).expect("read preserved entry"),
            b"user-owned"
        );

        fs::write(&path, b"# SD-300 managed\nold").expect("write owned entry");
        write_owned_text_file(&path, b"# SD-300 managed\nnew", "# SD-300 managed")
            .expect("replace owned entry");
        assert_eq!(
            fs::read(&path).expect("read replaced entry"),
            b"# SD-300 managed\nnew"
        );
    }

    #[cfg(unix)]
    #[test]
    fn startup_file_removal_preserves_symlinks_even_when_target_has_marker() {
        use std::os::unix::fs::symlink;

        let temp = tempfile::tempdir().expect("tempdir");
        let target = temp.path().join("owned-looking-target");
        let path = temp.path().join("startup-entry");
        fs::write(&target, b"# SD-300 managed\nuser target").expect("write target");
        symlink(&target, &path).expect("create startup symlink");

        let error = remove_owned_text_file(&path, "# SD-300 managed")
            .expect_err("ambiguous symlink must be preserved");

        assert!(error.contains("preserved"));
        assert!(fs::symlink_metadata(&path)
            .expect("symlink remains")
            .file_type()
            .is_symlink());
        assert!(target.is_file());
    }

    #[test]
    fn cleanup_removes_only_owned_settings_files_and_preserves_exports() {
        let temp = tempfile::tempdir().expect("tempdir");
        let directory = temp.path().join("SD-300");
        fs::create_dir_all(&directory).expect("settings directory");
        fs::write(directory.join(SETTINGS_FILE), b"{}").expect("settings");
        fs::write(directory.join("settings.corrupt-123.json"), b"broken")
            .expect("preserved corrupt settings");
        fs::write(directory.join("settings.unsupported-124.json"), b"newer")
            .expect("preserved newer settings");
        fs::write(directory.join(".settings-1-2.tmp"), b"staged").expect("staging file");
        fs::write(directory.join("exported-report.json"), b"user export").expect("export");

        remove_settings_files_at(&directory.join(SETTINGS_FILE)).expect("cleanup settings");

        assert!(!directory.join(SETTINGS_FILE).exists());
        assert!(!directory.join("settings.corrupt-123.json").exists());
        assert!(!directory.join("settings.unsupported-124.json").exists());
        assert!(!directory.join(".settings-1-2.tmp").exists());
        assert!(directory.join("exported-report.json").is_file());
        assert!(directory.is_dir());
    }

    #[test]
    fn cleanup_removes_an_empty_owned_settings_directory() {
        let temp = tempfile::tempdir().expect("tempdir");
        let directory = temp.path().join("SD-300");
        fs::create_dir_all(&directory).expect("settings directory");
        fs::write(directory.join(SETTINGS_FILE), b"{}").expect("settings");

        remove_settings_files_at(&directory.join(SETTINGS_FILE)).expect("cleanup settings");

        assert!(!directory.exists());
    }
}
