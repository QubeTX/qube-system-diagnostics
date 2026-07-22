use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::Duration;
#[cfg(any(windows, unix))]
use std::time::Instant;

use serde::Deserialize;

use crate::collectors::command::{run_output, CommandTimeout};

#[cfg(windows)]
use std::os::windows::process::CommandExt;

#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x0800_0000;
#[cfg(windows)]
const EVENT_MODIFY_STATE: u32 = 0x0002;

#[cfg(windows)]
use winapi::{
    shared::minwindef::FALSE,
    um::{
        handleapi::CloseHandle,
        processthreadsapi::ProcessIdToSessionId,
        synchapi::{OpenEventW, SetEvent},
    },
};

/// Open the separately installed native monitor without changing the bare
/// `sd300` TUI path. Lifecycle commands own installation and repair; this
/// command only discovers a proven product-relative/platform location and
/// starts it.
pub fn launch() -> i32 {
    let Some(candidate) = locate() else {
        eprintln!(
            "SD-300 desktop monitor is not installed or is incomplete. Run `sd300 update` to repair the current installation, or `sd300 install` for a managed install."
        );
        return 2;
    };

    match spawn(&candidate) {
        Ok(()) => 0,
        Err(error) => {
            eprintln!(
                "Could not open the SD-300 desktop monitor at '{}': {error}. Run `sd300 update` to repair the GUI companion.",
                candidate.display()
            );
            1
        }
    }
}

#[derive(Debug, Deserialize)]
struct SelfTestResult {
    success: bool,
    product: String,
    product_version: String,
    abi_version: u32,
    engine_schema_version: u32,
}

/// Prove that the installed companion is more than a path-shaped placeholder.
/// The app's headless self-test loads the adjacent Rust engine, rejects
/// ABI/schema/product/target mismatches, starts collection, and waits for a
/// live CPU/memory sample without opening a window or creating a tray item.
pub fn verify_installed(expected_version: &str) -> std::result::Result<(), String> {
    let installed = locate().ok_or_else(|| "the GUI companion is missing".to_string())?;
    let executable = self_test_executable(&installed);
    let output = run_output(
        executable.as_os_str(),
        ["--self-test", "--json"],
        CommandTimeout::Custom(Duration::from_secs(15)),
    )
    .ok_or_else(|| {
        format!(
            "the GUI companion self-test did not finish safely at {}",
            executable.display()
        )
    })?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!(
            "the GUI companion self-test failed at {}{}",
            executable.display(),
            if stderr.trim().is_empty() {
                String::new()
            } else {
                format!(": {}", stderr.trim())
            }
        ));
    }
    let line = String::from_utf8(output.stdout)
        .map_err(|_| "the GUI companion self-test returned non-UTF-8 output".to_string())?;
    let result: SelfTestResult = serde_json::from_str(line.trim())
        .map_err(|error| format!("the GUI companion self-test JSON was invalid: {error}"))?;
    if !result.success
        || result.product != "SD-300"
        || result.product_version != expected_version
        || result.abi_version != 1
        || result.engine_schema_version != 1
    {
        return Err(format!(
            "the GUI companion reported an incompatible identity (product={}, version={}, ABI={}, schema={})",
            result.product,
            result.product_version,
            result.abi_version,
            result.engine_schema_version
        ));
    }
    Ok(())
}

fn self_test_executable(installed: &Path) -> PathBuf {
    #[cfg(target_os = "macos")]
    if installed
        .extension()
        .is_some_and(|extension| extension == "app")
    {
        return installed.join("Contents").join("MacOS").join("sd300-gui");
    }
    installed.to_path_buf()
}

/// Ask every GUI in the current Windows logon session to exit through its
/// application message loop, then prove the process is gone before lifecycle
/// code mutates the installed image.
#[cfg(windows)]
pub fn request_exit() -> std::result::Result<(), String> {
    use std::os::windows::ffi::OsStrExt;

    let running_before = running_gui_processes();
    let mut name: Vec<u16> = std::ffi::OsStr::new("Local\\SD300.Gui.Quit.v1")
        .encode_wide()
        .collect();
    name.push(0);
    let handle = unsafe { OpenEventW(EVENT_MODIFY_STATE, FALSE, name.as_ptr()) };
    if handle.is_null() {
        return if running_before == 0 {
            Ok(())
        } else {
            Err(format!(
                "{running_before} SD-300 GUI process(es) are running without the authenticated lifecycle endpoint"
            ))
        };
    }
    let signaled = unsafe { SetEvent(handle) } != 0;
    unsafe {
        CloseHandle(handle);
    }
    if !signaled {
        return Err("the SD-300 GUI lifecycle endpoint could not be signaled".into());
    }

    let deadline = Instant::now() + Duration::from_secs(10);
    while Instant::now() < deadline {
        if running_gui_processes() == 0 {
            return Ok(());
        }
        std::thread::sleep(Duration::from_millis(100));
    }
    Err(format!(
        "{} SD-300 GUI process(es) did not exit within 10 seconds",
        running_gui_processes()
    ))
}

#[cfg(not(windows))]
pub fn request_exit() -> std::result::Result<(), String> {
    use std::io::Write;
    use std::os::unix::{
        fs::{FileTypeExt, MetadataExt},
        net::UnixStream,
    };

    let running_before = running_gui_processes();
    let endpoint = unix_lifecycle_endpoint();
    if !endpoint.exists() {
        return if running_before == 0 {
            Ok(())
        } else {
            Err(format!(
                "{running_before} SD-300 GUI process(es) are running without the authenticated lifecycle endpoint"
            ))
        };
    }
    let parent = endpoint
        .parent()
        .ok_or_else(|| "the GUI lifecycle endpoint has no parent directory".to_string())?;
    let parent_metadata = std::fs::symlink_metadata(parent)
        .map_err(|error| format!("the GUI lifecycle directory is unreadable: {error}"))?;
    let socket_metadata = std::fs::symlink_metadata(&endpoint)
        .map_err(|error| format!("the GUI lifecycle endpoint is unreadable: {error}"))?;
    let effective_uid = unsafe { libc::geteuid() };
    if !parent_metadata.is_dir()
        || parent_metadata.uid() != effective_uid
        || parent_metadata.mode() & 0o077 != 0
        || !socket_metadata.file_type().is_socket()
        || socket_metadata.uid() != effective_uid
    {
        return Err(format!(
            "the GUI lifecycle endpoint at {} is not a private same-user Unix socket",
            endpoint.display()
        ));
    }

    // A crash or SIGKILL cannot run the GUI's normal socket cleanup. Once
    // process discovery proves that no same-user GUI exists, this validated
    // same-user socket is stale rather than an active lifecycle authority.
    // Remove only that exact endpoint (and its empty private directory) so an
    // otherwise safe update/uninstall cannot be blocked forever by residue.
    if running_before == 0 && running_gui_processes() == 0 {
        std::fs::remove_file(&endpoint).map_err(|error| {
            format!(
                "the stale GUI lifecycle endpoint at {} could not be removed safely: {error}",
                endpoint.display()
            )
        })?;
        match std::fs::remove_dir(parent) {
            Ok(()) => {}
            Err(error)
                if matches!(
                    error.kind(),
                    std::io::ErrorKind::NotFound | std::io::ErrorKind::DirectoryNotEmpty
                ) => {}
            Err(error) => {
                return Err(format!(
                    "the empty GUI lifecycle directory at {} could not be removed: {error}",
                    parent.display()
                ));
            }
        }
        return Ok(());
    }

    let mut stream = UnixStream::connect(&endpoint).map_err(|error| {
        format!(
            "the GUI lifecycle endpoint at {} could not be reached: {error}",
            endpoint.display()
        )
    })?;
    stream
        .set_write_timeout(Some(Duration::from_secs(2)))
        .map_err(|error| format!("the GUI lifecycle timeout could not be set: {error}"))?;
    stream
        .write_all(b"quit\n")
        .map_err(|error| format!("the GUI lifecycle request could not be sent: {error}"))?;

    let deadline = Instant::now() + Duration::from_secs(10);
    while Instant::now() < deadline {
        if running_gui_processes() == 0 {
            return Ok(());
        }
        std::thread::sleep(Duration::from_millis(100));
    }
    Err(format!(
        "{} SD-300 GUI process(es) did not exit within 10 seconds",
        running_gui_processes()
    ))
}

#[cfg(unix)]
fn unix_lifecycle_endpoint() -> PathBuf {
    unix_lifecycle_endpoint_from(
        std::env::var_os("XDG_RUNTIME_DIR")
            .as_deref()
            .map(Path::new),
        unsafe { libc::geteuid() },
        cfg!(target_os = "linux"),
    )
}

#[cfg(unix)]
fn unix_lifecycle_endpoint_from(
    xdg_runtime: Option<&Path>,
    effective_uid: u32,
    linux: bool,
) -> PathBuf {
    if linux {
        if let Some(runtime) = xdg_runtime.filter(|path| path.is_absolute()) {
            return runtime.join("sd300").join("gui.sock");
        }
    }
    PathBuf::from(format!("/tmp/sd300-{effective_uid}/gui.sock"))
}

#[cfg(unix)]
fn running_gui_processes() -> usize {
    let mut system = sysinfo::System::new();
    system.refresh_processes(sysinfo::ProcessesToUpdate::All, true);
    let current_user = sysinfo::get_current_pid()
        .ok()
        .and_then(|pid| system.process(pid))
        .and_then(sysinfo::Process::user_id);
    system
        .processes()
        .values()
        .filter(|process| {
            process.name() == "sd300-gui"
                && current_user.is_some()
                && process.user_id() == current_user
        })
        .count()
}

#[cfg(windows)]
fn running_gui_processes() -> usize {
    let mut system = sysinfo::System::new();
    system.refresh_processes(sysinfo::ProcessesToUpdate::All, true);
    let Some(current_session) = windows_session_id(std::process::id()) else {
        // Failing closed here prevents lifecycle mutation when Windows cannot
        // prove which per-logon `Local\` endpoint owns the visible process.
        return usize::MAX;
    };
    system
        .processes()
        .values()
        .filter(|process| {
            process
                .name()
                .to_string_lossy()
                .eq_ignore_ascii_case("sd300-gui.exe")
                && windows_session_id(process.pid().as_u32()) == Some(current_session)
        })
        .count()
}

#[cfg(windows)]
fn windows_session_id(process_id: u32) -> Option<u32> {
    let mut session_id = 0;
    let ok = unsafe { ProcessIdToSessionId(process_id, &mut session_id) } != 0;
    ok.then_some(session_id)
}

fn locate() -> Option<PathBuf> {
    let current = std::env::current_exe().ok();
    locate_from(
        current.as_deref(),
        home_dir().as_deref(),
        xdg_data_home().as_deref(),
    )
}

/// Cheap presence check for the exceptional two-step Cargo migration notice.
///
/// Lifecycle commands still use [`verify_installed`] before committing a
/// repair or update. A normal TUI launch must not start the GUI engine or pay
/// for a self-test merely to decide whether to show the completion hint.
pub(crate) fn companion_path_present() -> bool {
    locate().is_some()
}

fn locate_from(
    _current_exe: Option<&Path>,
    _home: Option<&Path>,
    _xdg_data: Option<&Path>,
) -> Option<PathBuf> {
    let mut candidates = Vec::new();
    #[cfg(any(windows, target_os = "linux"))]
    if let Some(current) = _current_exe {
        if let Some(bin) = current.parent() {
            #[cfg(any(windows, target_os = "linux"))]
            if let Some(root) = bin.parent() {
                #[cfg(windows)]
                candidates.push(root.join("app").join("sd300-gui.exe"));
                #[cfg(target_os = "linux")]
                candidates.push(root.join("app").join("sd300-gui"));
            }
            #[cfg(windows)]
            candidates.push(bin.join("sd300-gui.exe"));
            #[cfg(target_os = "linux")]
            candidates.push(bin.join("sd300-gui"));
        }
    }

    #[cfg(windows)]
    if let Some(local_app_data) = std::env::var_os("LOCALAPPDATA") {
        candidates.push(
            PathBuf::from(local_app_data)
                .join("Programs")
                .join("SD-300")
                .join("app")
                .join("sd300-gui.exe"),
        );
    }

    #[cfg(target_os = "macos")]
    {
        if let Some(home) = _home {
            candidates.push(home.join("Applications").join("SD-300.app"));
        }
        candidates.push(PathBuf::from("/Applications/SD-300.app"));
    }

    #[cfg(target_os = "linux")]
    {
        let data_root = _xdg_data
            .map(Path::to_path_buf)
            .or_else(|| _home.map(|path| path.join(".local").join("share")));
        if let Some(data_root) = data_root {
            candidates.push(data_root.join("sd300").join("bin").join("sd300-gui"));
        }
    }

    candidates
        .into_iter()
        .find(|path| path.is_file() || is_macos_app(path))
}

#[cfg(target_os = "macos")]
fn is_macos_app(path: &Path) -> bool {
    path.is_dir() && path.join("Contents/MacOS/sd300-gui").is_file()
}

#[cfg(not(target_os = "macos"))]
fn is_macos_app(_path: &Path) -> bool {
    false
}

fn spawn(candidate: &Path) -> std::io::Result<()> {
    #[cfg(target_os = "macos")]
    let mut command = {
        let mut command = Command::new("open");
        command.arg(candidate);
        command
    };

    #[cfg(not(target_os = "macos"))]
    let mut command = Command::new(candidate);

    command
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    #[cfg(windows)]
    command.creation_flags(CREATE_NO_WINDOW);
    command.spawn().map(|_| ())
}

fn home_dir() -> Option<PathBuf> {
    #[cfg(windows)]
    let value = std::env::var_os("USERPROFILE");
    #[cfg(not(windows))]
    let value = std::env::var_os("HOME");
    value.map(PathBuf::from)
}

fn xdg_data_home() -> Option<PathBuf> {
    std::env::var_os("XDG_DATA_HOME").map(PathBuf::from)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn absent_companion_is_not_mistaken_for_the_tui_binary() {
        let missing = PathBuf::from("definitely-not-an-sd300-install/bin/sd300");
        assert!(locate_from(Some(&missing), None, None).is_none());
    }

    #[test]
    fn non_bundle_self_test_uses_the_discovered_executable() {
        let executable = PathBuf::from("somewhere/sd300-gui");
        assert_eq!(self_test_executable(&executable), executable);
    }

    #[cfg(unix)]
    #[test]
    fn unix_lifecycle_endpoint_uses_private_runtime_location() {
        let runtime = Path::new("/run/user/1000");
        assert_eq!(
            unix_lifecycle_endpoint_from(Some(runtime), 1000, true),
            runtime.join("sd300/gui.sock")
        );
        assert_eq!(
            unix_lifecycle_endpoint_from(Some(runtime), 501, false),
            PathBuf::from("/tmp/sd300-501/gui.sock")
        );
        assert_eq!(
            unix_lifecycle_endpoint_from(Some(Path::new("relative")), 1000, true),
            PathBuf::from("/tmp/sd300-1000/gui.sock")
        );
    }
}
