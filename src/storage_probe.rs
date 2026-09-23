//! Explicit, single-device SMART reads. Authentication and device access run in
//! owned CLI processes; neither frontend elevates its monitoring session.
use crate::collectors::{command, disk_health, sampling};
use crate::observation::Observation;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::io::{Read, Write};
use std::net::{Ipv4Addr, SocketAddr, TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex,
};
use std::time::{Duration, Instant};

const MAX_MESSAGE: usize = 64 * 1024;
const MAX_HELPER: u64 = 16 * 1024 * 1024;
const AUTH_BUDGET: Duration = Duration::from_secs(60);
const READ_BUDGET: Duration = Duration::from_secs(12);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Prepared {
    product_version: String,
    device: String,
    helper: PathBuf,
    sha256: String,
    expected_serial: Option<String>,
}
impl Prepared {
    pub fn notice(&self) -> String {
        format!("Read storage health for {} using {} (SHA-256 {}). The operating system will request administrator authorization for one isolated probe. Run only smartctl --json --all --nocheck=standby,3 on this device, with a twelve-second read limit. No self-test, repair, service or disk setting change is requested. Authentication expires after one minute. Results remain in memory until exported. Cancel keeps ordinary monitoring available.", self.device, self.helper.display(), self.sha256)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProbeResult {
    pub captured_unix_ms: u64,
    pub drive: disk_health::DriveHealth,
    pub observation: Observation,
    pub elevated: bool,
}
#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct State {
    pub sequence: u64,
    pub running: bool,
    pub awaiting_consent: bool,
    pub notice: String,
    pub message: String,
    pub result: Option<ProbeResult>,
}
impl State {
    pub fn lines(&self) -> Vec<String> {
        let mut lines = vec![self.message.clone()];
        if let Some(result) = &self.result {
            let d = &result.drive;
            lines.push(format!(
                "Captured {} ms UTC · {} · {:?}",
                result.captured_unix_ms, d.device_id, result.observation.status
            ));
            lines.push(format!(
                "Health: {} · {}",
                d.health_status.user_label(),
                d.health_source
            ));
            lines.push(format!(
                "Temperature: {} · Powered-on hours: {} · Wear: {}",
                d.temperature_celsius
                    .map(|v| format!("{v:.1} °C"))
                    .unwrap_or("Unavailable".into()),
                d.power_on_hours
                    .map(|v| v.to_string())
                    .unwrap_or("Unavailable".into()),
                d.wear_percent
                    .map(|v| format!("{v}%"))
                    .unwrap_or("Unavailable".into())
            ));
            lines.push(format!(
                "Read errors: {} · Write errors: {}",
                d.read_errors_total
                    .map(|v| v.to_string())
                    .unwrap_or("Unavailable".into()),
                d.write_errors_total
                    .map(|v| v.to_string())
                    .unwrap_or("Unavailable".into())
            ));
            if let Some(detail) = &result.observation.detail {
                lines.push(detail.clone());
            }
        }
        lines
    }

    pub fn redacted(&self) -> Self {
        let mut value = self.clone();
        value.notice.clear();
        value.message = "Explicit storage probe; identifiers omitted".into();
        if let Some(result) = &mut value.result {
            result.drive.device_id = "[redacted device]".into();
            result.drive.serial = result.drive.serial.as_ref().map(|_| "[redacted]".into());
            // Errors can contain paths or device identifiers from the OS.
            result.observation.detail = result
                .observation
                .detail
                .as_ref()
                .map(|_| "Provider detail omitted from redacted export".into());
        }
        value
    }
}

#[cfg(any(windows, test))]
fn windows_device(value: &str) -> Option<String> {
    let lower = value.to_ascii_lowercase();
    let number = lower.strip_prefix(r"\\.\physicaldrive")?;
    (!number.is_empty() && number.len() <= 8 && number.bytes().all(|b| b.is_ascii_digit()))
        .then(|| format!("/dev/pd{number}"))
}
#[cfg(any(target_os = "macos", test))]
fn macos_device(value: &str) -> bool {
    value
        .strip_prefix("/dev/disk")
        .is_some_and(|n| !n.is_empty() && n.len() <= 8 && n.bytes().all(|b| b.is_ascii_digit()))
}
#[cfg(any(target_os = "linux", test))]
fn linux_device(value: &str) -> bool {
    let Some(name) = value.strip_prefix("/dev/") else {
        return false;
    };
    let digits = |n: &str| !n.is_empty() && n.len() <= 8 && n.bytes().all(|b| b.is_ascii_digit());
    for prefix in ["sd", "hd", "vd", "xvd"] {
        if name.strip_prefix(prefix).is_some_and(|n| {
            !n.is_empty() && n.len() <= 4 && n.bytes().all(|b| b.is_ascii_lowercase())
        }) {
            return true;
        }
    }
    if name.strip_prefix("mmcblk").is_some_and(digits) {
        return true;
    }
    name.strip_prefix("nvme")
        .and_then(|n| n.split_once('n'))
        .is_some_and(|(c, n)| digits(c) && digits(n))
}
fn device_argument(value: &str) -> Result<String, String> {
    #[cfg(windows)]
    if let Some(device) = windows_device(value) {
        return Ok(device);
    }
    #[cfg(target_os = "macos")]
    if macos_device(value) {
        return Ok(value.into());
    }
    #[cfg(target_os = "linux")]
    if linux_device(value) {
        return Ok(value.into());
    }
    Err("Select a supported whole physical device from the current storage inventory".into())
}

fn helper_bytes(path: &Path) -> Result<Vec<u8>, String> {
    if !path.is_absolute() || path.to_string_lossy().chars().any(char::is_control) {
        return Err("The SMART helper must have an absolute local path".into());
    }
    #[cfg(windows)]
    if path.to_string_lossy().starts_with(r"\\") && !path.to_string_lossy().starts_with(r"\\?\C:\")
    {
        return Err("Network helper paths cannot be elevated".into());
    }
    let mut file = std::fs::File::open(path).map_err(|e| e.to_string())?;
    let metadata = file.metadata().map_err(|e| e.to_string())?;
    if !metadata.is_file() || metadata.len() == 0 || metadata.len() > MAX_HELPER {
        return Err("The SMART helper is not a bounded regular executable".into());
    }
    let mut bytes = Vec::new();
    (&mut file)
        .take(MAX_HELPER + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    if bytes.len() as u64 > MAX_HELPER {
        return Err("SMART helper size changed during verification".into());
    }
    Ok(bytes)
}

/// Called only in an owned, ten-second preparation subprocess.
pub fn prepare_worker(device: &str) -> Result<Prepared, String> {
    device_argument(device)?;
    #[cfg(target_os = "linux")]
    if !Path::new("/usr/bin/pkexec").is_file() {
        return Err("A PolicyKit installation and graphical authentication agent are required for a bounded privileged read on Linux; ordinary monitoring remains available".into());
    }
    let helper = crate::optional_tools::detect("smartctl")
        .ok_or("smartctl is unavailable; install or select the optional SMART helper first")?;
    crate::smart_setup::verify(&helper, &AtomicBool::new(false))?;
    let sha256 = format!("{:x}", Sha256::digest(helper_bytes(&helper)?));
    Ok(Prepared {
        product_version: env!("CARGO_PKG_VERSION").into(),
        device: device.into(),
        helper,
        sha256,
        expected_serial: None,
    })
}

fn send<T: Serialize>(stream: &mut TcpStream, value: &T) -> Result<(), String> {
    let bytes = serde_json::to_vec(value).map_err(|e| e.to_string())?;
    if bytes.len() > MAX_MESSAGE {
        return Err("Storage probe message exceeded its bound".into());
    }
    stream
        .write_all(&(bytes.len() as u32).to_le_bytes())
        .and_then(|_| stream.write_all(&bytes))
        .map_err(|e| e.to_string())
}
fn read_exact_until(
    stream: &mut TcpStream,
    mut bytes: &mut [u8],
    deadline: Instant,
) -> Result<(), String> {
    while !bytes.is_empty() {
        if Instant::now() >= deadline {
            return Err("Storage handshake exceeded its deadline".into());
        }
        match stream.read(bytes) {
            Ok(0) => return Err("Storage channel closed".into()),
            Ok(n) => bytes = &mut bytes[n..],
            Err(e)
                if matches!(
                    e.kind(),
                    std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut
                ) => {}
            Err(e) => return Err(e.to_string()),
        }
    }
    Ok(())
}
fn receive<T: serde::de::DeserializeOwned>(stream: &mut TcpStream) -> Result<T, String> {
    let deadline = Instant::now() + Duration::from_millis(500);
    let mut len = [0u8; 4];
    read_exact_until(stream, &mut len, deadline)?;
    let len = u32::from_le_bytes(len) as usize;
    if len > MAX_MESSAGE {
        return Err("Storage probe message exceeded its bound".into());
    }
    let mut bytes = vec![0; len];
    read_exact_until(stream, &mut bytes, deadline)?;
    serde_json::from_slice(&bytes).map_err(|_| "Malformed storage probe response".into())
}
fn nonce_valid(nonce: &str) -> bool {
    nonce.len() == 64 && nonce.bytes().all(|b| b.is_ascii_hexdigit())
}

fn elevated() -> bool {
    #[cfg(windows)]
    {
        #[link(name = "shell32")]
        unsafe extern "system" {
            fn IsUserAnAdmin() -> i32;
        }
        unsafe { IsUserAnAdmin() != 0 }
    }
    #[cfg(unix)]
    {
        unsafe { libc::geteuid() == 0 }
    }
}

/// Invoked by the OS authorization broker. The callback carries the confirmed
/// request in memory; there is no arbitrary command or writable report path.
pub fn read_worker(port: u16, nonce: &str) -> Result<(), String> {
    if !nonce_valid(nonce) || port == 0 || !elevated() {
        return Err("Invalid or unprivileged storage worker invocation".into());
    }
    let mut stream = TcpStream::connect_timeout(
        &SocketAddr::from((Ipv4Addr::LOCALHOST, port)),
        Duration::from_secs(1),
    )
    .map_err(|e| e.to_string())?;
    stream
        .set_read_timeout(Some(Duration::from_secs(1)))
        .map_err(|e| e.to_string())?;
    stream
        .set_write_timeout(Some(Duration::from_secs(1)))
        .map_err(|e| e.to_string())?;
    send(&mut stream, &nonce)?;
    let request: Prepared = receive(&mut stream)?;
    let cancel = Arc::new(AtomicBool::new(false));
    let done = Arc::new(AtomicBool::new(false));
    let watchdog_cancel = cancel.clone();
    let watchdog_done = done.clone();
    // This thread lives only in the isolated CLI worker, never in the engine
    // library. Normal process exit removes it; shutdown never joins its sleep.
    std::thread::spawn(move || {
        std::thread::sleep(Duration::from_secs(20));
        watchdog_cancel.store(true, Ordering::Release);
        std::thread::sleep(Duration::from_secs(2));
        if !watchdog_done.load(Ordering::Acquire) {
            std::process::exit(124);
        }
    });
    let result = read_prepared(&mut stream, request, &cancel);
    let sent = send(&mut stream, &result);
    done.store(true, Ordering::Release);
    sent
}

fn read_prepared(
    stream: &mut TcpStream,
    request: Prepared,
    cancel: &AtomicBool,
) -> Result<ProbeResult, String> {
    if request.product_version != env!("CARGO_PKG_VERSION") {
        return Err("The storage worker version differs from the prepared operation; retry after the update finishes".into());
    }
    let device = device_argument(&request.device)?;
    let bytes = helper_bytes(&request.helper)?;
    if format!("{:x}", Sha256::digest(&bytes)) != request.sha256 {
        return Err(
            "The SMART helper changed after confirmation; no device read was started".into(),
        );
    }
    // Execute verified bytes from a private elevated directory. Replacing the
    // original user-owned path after hashing cannot replace this invocation.
    let staging = tempfile::tempdir().map_err(|e| e.to_string())?;
    let helper = staging
        .path()
        .join(crate::optional_tools::executable_name("smartctl"));
    std::fs::write(&helper, bytes).map_err(|e| e.to_string())?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&helper, std::fs::Permissions::from_mode(0o700))
            .map_err(|e| e.to_string())?;
    }
    let finished = AtomicBool::new(false);
    let mut disconnect = stream.try_clone().map_err(|e| e.to_string())?;
    disconnect
        .set_read_timeout(Some(Duration::from_millis(50)))
        .map_err(|e| e.to_string())?;
    let result = std::thread::scope(|scope| {
        scope.spawn(|| {
            let mut byte = [0];
            while !finished.load(Ordering::Acquire) {
                match disconnect.read(&mut byte) {
                    Err(e)
                        if matches!(
                            e.kind(),
                            std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut
                        ) => {}
                    _ => {
                        cancel.store(true, Ordering::Release);
                        break;
                    }
                }
            }
        });
        let result = command::run_memory(
            &helper,
            ["--json", "--all", "--nocheck=standby,3", &device],
            command::CommandTimeout::Custom(READ_BUDGET),
            cancel,
        );
        finished.store(true, Ordering::Release);
        result
    });
    let mut drive = disk_health::empty_drive(
        request.device,
        "Selected physical device".into(),
        disk_health::MediaType::Unknown,
    );
    drive.serial = request.expected_serial;
    let observation = match result {
        Ok(output) => match output.failure {
            Some(error) => Observation::error("explicit smartctl read", error.to_string()),
            None => disk_health::apply_smart_json(
                &mut drive,
                &output.stdout,
                output.status.and_then(|s| s.code()).unwrap_or(255),
            ),
        },
        Err(error) => Observation::error("explicit smartctl read", error.to_string()),
    };
    Ok(ProbeResult {
        captured_unix_ms: sampling::unix_ms(),
        drive,
        observation,
        elevated: true,
    })
}

#[cfg(any(target_os = "macos", test))]
fn apple_script(cli: &str, port: u16, nonce: &str) -> String {
    let literal = cli.replace('\\', "\\\\").replace('"', "\\\"");
    format!("do shell script ((quoted form of \"{literal}\") & \" storage-probe-read {port} {nonce}\") with administrator privileges")
}
fn broker_command(cli: &Path, port: u16, nonce: &str) -> Result<Command, String> {
    let mut command = Command::new(cli);
    command.args(["storage-probe-elevate", &port.to_string(), nonce]);
    Ok(command)
}

#[cfg(unix)]
fn unix_authorization_command(cli: &Path, port: u16, nonce: &str) -> Result<Command, String> {
    #[cfg(target_os = "linux")]
    {
        if !Path::new("/usr/bin/pkexec").is_file() {
            return Err("A graphical PolicyKit authentication agent is required for a bounded privileged read; ordinary monitoring remains available".into());
        }
        let mut c = Command::new("/usr/bin/pkexec");
        c.arg("--disable-internal-agent").arg(cli).args([
            "storage-probe-read",
            &port.to_string(),
            nonce,
        ]);
        Ok(c)
    }
    #[cfg(target_os = "macos")]
    {
        let mut c = Command::new("/usr/bin/osascript");
        c.args([
            "-e",
            &apple_script(cli.to_str().ok_or("CLI path is not UTF-8")?, port, nonce),
        ]);
        Ok(c)
    }
}

/// Only this short-lived unelevated broker may wait in an OS authorization call.
pub fn elevate_worker(port: u16, nonce: &str) -> Result<(), String> {
    if !nonce_valid(nonce) || port == 0 {
        return Err("Invalid storage broker invocation".into());
    }
    #[cfg(windows)]
    {
        use winapi::um::{
            handleapi::CloseHandle, processthreadsapi::TerminateProcess, shellapi::*,
            synchapi::WaitForSingleObject,
        };
        let _com = wmi::COMLibrary::new().map_err(|e| e.to_string())?;
        let executable = std::env::current_exe().map_err(|e| e.to_string())?;
        use std::os::windows::ffi::OsStrExt;
        let path: Vec<_> = executable
            .as_os_str()
            .encode_wide()
            .chain(Some(0))
            .collect();
        let verb: Vec<_> = "runas".encode_utf16().chain(Some(0)).collect();
        let args: Vec<_> = format!("storage-probe-read {port} {nonce}")
            .encode_utf16()
            .chain(Some(0))
            .collect();
        let mut info: SHELLEXECUTEINFOW = unsafe { std::mem::zeroed() };
        info.cbSize = std::mem::size_of_val(&info) as u32;
        info.fMask = SEE_MASK_NOCLOSEPROCESS | SEE_MASK_NOASYNC | SEE_MASK_FLAG_NO_UI;
        info.lpVerb = verb.as_ptr();
        info.lpFile = path.as_ptr();
        info.lpParameters = args.as_ptr();
        info.nShow = 0;
        if unsafe { ShellExecuteExW(&mut info) } == 0 {
            return Err(format!(
                "Storage authorization was declined or unavailable: {}",
                std::io::Error::last_os_error()
            ));
        }
        if info.hProcess.is_null() {
            return Err("Storage authorization returned no owned worker".into());
        }
        let wait = unsafe { WaitForSingleObject(info.hProcess, 20_000) };
        if wait != 0 {
            unsafe {
                TerminateProcess(info.hProcess, 1);
            }
        }
        unsafe {
            CloseHandle(info.hProcess);
        }
        if wait != 0 {
            return Err("The isolated storage worker exceeded its deadline".into());
        }
        Ok(())
    }
    #[cfg(unix)]
    {
        let cli = std::env::current_exe().map_err(|e| e.to_string())?;
        let status = unix_authorization_command(&cli, port, nonce)?
            .status()
            .map_err(|e| e.to_string())?;
        if status.success() {
            Ok(())
        } else {
            Err("Storage authorization was declined or unavailable".into())
        }
    }
}

fn run_confirmed(request: &Prepared, cancel: &AtomicBool) -> Result<ProbeResult, String> {
    if cancel.load(Ordering::Acquire) {
        return Err("Storage read cancelled".into());
    }
    let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).map_err(|e| e.to_string())?;
    listener.set_nonblocking(true).map_err(|e| e.to_string())?;
    let port = listener.local_addr().map_err(|e| e.to_string())?.port();
    let mut random = [0u8; 32];
    getrandom::fill(&mut random).map_err(|e| e.to_string())?;
    let nonce: String = random.iter().map(|b| format!("{b:02x}")).collect();
    let cli =
        crate::collectors::probe::executable().ok_or("The matching SD-300 CLI is unavailable")?;
    let mut command = broker_command(&cli, port, &nonce)?;
    let stop_broker = AtomicBool::new(false);
    std::thread::scope(|scope| {
        let broker = scope.spawn(|| {
            command::run_memory_command(
                &mut command,
                command::CommandTimeout::Custom(AUTH_BUDGET),
                &stop_broker,
            )
        });
        let deadline = Instant::now() + AUTH_BUDGET;
        let result = (|| {
            loop {
                if cancel.load(Ordering::Acquire) {
                    return Err("Storage read cancelled; the bounded worker will stop".into());
                }
                if Instant::now() >= deadline {
                    return Err("Storage authentication or read exceeded its deadline".into());
                }
                match listener.accept() {
                    Ok((mut stream, peer)) => {
                        if !peer.ip().is_loopback() {
                            continue;
                        }
                        // Accepted sockets inherit nonblocking mode on Windows/BSD,
                        // unlike Linux. Normalize before timed framing and reads.
                        stream.set_nonblocking(false).map_err(|e| e.to_string())?;
                        stream
                            .set_read_timeout(Some(Duration::from_millis(100)))
                            .map_err(|e| e.to_string())?;
                        stream
                            .set_write_timeout(Some(Duration::from_millis(100)))
                            .map_err(|e| e.to_string())?;
                        if receive::<String>(&mut stream).ok().as_deref() != Some(&nonce) {
                            continue;
                        }
                        send(&mut stream, request)?;
                        // Receive one bounded framed response incrementally so cancellation
                        // and the deadline remain responsive even if a worker stalls.
                        let mut bytes = Vec::new();
                        loop {
                            if cancel.load(Ordering::Acquire) {
                                return Err("Storage read cancelled; no result was imported".into());
                            }
                            if Instant::now() >= deadline {
                                return Err("Storage read exceeded its deadline".into());
                            }
                            let mut chunk = [0u8; 4096];
                            match stream.read(&mut chunk) {
                                Ok(0) => {
                                    return Err(
                                        "The privileged worker closed without a complete result"
                                            .into(),
                                    )
                                }
                                Ok(n) => bytes.extend_from_slice(&chunk[..n]),
                                Err(e)
                                    if matches!(
                                        e.kind(),
                                        std::io::ErrorKind::WouldBlock
                                            | std::io::ErrorKind::TimedOut
                                    ) =>
                                {
                                    continue
                                }
                                Err(e) => return Err(e.to_string()),
                            }
                            if bytes.len() > MAX_MESSAGE + 4 {
                                return Err("Storage result exceeded its bound".into());
                            }
                            if bytes.len() >= 4 {
                                let len =
                                    u32::from_le_bytes(bytes[..4].try_into().unwrap()) as usize;
                                if len > MAX_MESSAGE {
                                    return Err("Storage result exceeded its bound".into());
                                }
                                if bytes.len() >= len + 4 {
                                    let result: ProbeResult =
                                        serde_json::from_slice::<Result<ProbeResult, String>>(
                                            &bytes[4..len + 4],
                                        )
                                        .map_err(|_| "Malformed privileged storage result")??;
                                    if result.drive.device_id != request.device || !result.elevated
                                    {
                                        return Err("Storage response identity did not match the confirmed read".into());
                                    }
                                    return Ok(result);
                                }
                            }
                        }
                    }
                    Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {}
                    Err(e) => return Err(e.to_string()),
                }
                if broker.is_finished() {
                    return Err("Storage authorization was declined, unavailable, or the worker could not start; ordinary monitoring remains available".into());
                }
                std::thread::sleep(Duration::from_millis(10));
            }
        })();
        stop_broker.store(true, Ordering::Release);
        let output = broker.join().ok().and_then(Result::ok);
        if result
            .as_ref()
            .err()
            .is_some_and(|e| e.starts_with("Storage authorization was"))
        {
            if let Some(error) = output
                .and_then(|o| serde_json::from_slice::<Result<(), String>>(&o.stdout).ok())
                .and_then(Result::err)
            {
                return Err(error);
            }
        }
        result
    })
}

enum Completion {
    Prepared(Prepared),
    Read(Box<ProbeResult>),
}
#[derive(Default)]
pub struct Controller {
    pub state: State,
    prepared: Option<Prepared>,
    cancel: Arc<AtomicBool>,
    complete: Arc<Mutex<Option<Result<Completion, String>>>>,
    worker: Option<std::thread::JoinHandle<()>>,
}
impl Controller {
    pub fn prepare(&mut self, drive: &disk_health::DriveHealth) -> bool {
        self.poll();
        if self.worker.is_some() {
            return false;
        }
        self.prepared = None;
        self.state.awaiting_consent = false;
        self.state.notice.clear();
        let device = drive.device_id.clone();
        let serial = drive.serial.clone();
        self.spawn(
            move |cancel| {
                device_argument(&device)?;
                let cli = crate::collectors::probe::executable()
                    .ok_or("The matching SD-300 CLI is unavailable")?;
                let output = command::run_memory(
                    cli,
                    ["storage-probe-prepare", &device],
                    command::CommandTimeout::Custom(Duration::from_secs(10)),
                    cancel,
                )
                .map_err(|e| e.to_string())?;
                let bytes = crate::optional_tools::successful("Storage probe preparation", output)?;
                let mut prepared: Prepared =
                    serde_json::from_slice::<Result<Prepared, String>>(&bytes)
                        .map_err(|_| "Malformed storage preparation")??;
                if prepared.product_version != env!("CARGO_PKG_VERSION") { return Err("The installed CLI and monitoring engine versions differ; complete the product update before a privileged read".into()); }
                prepared.expected_serial = serial;
                Ok(Completion::Prepared(prepared))
            },
            "Verifying the optional SMART helper before requesting permission",
        )
    }
    pub fn confirm(&mut self, consent: bool) -> bool {
        self.poll();
        if self.worker.is_some() {
            return false;
        }
        self.state.awaiting_consent = false;
        let Some(request) = self.prepared.take() else {
            return false;
        };
        if !consent {
            self.state.message =
                "Privileged read declined; ordinary monitoring remains available".into();
            self.state.sequence += 1;
            return false;
        }
        self.spawn(
            move |cancel| {
                run_confirmed(&request, cancel)
                    .map(Box::new)
                    .map(Completion::Read)
            },
            "Waiting for operating-system authorization; cancellation is available",
        )
    }
    fn spawn(
        &mut self,
        job: impl FnOnce(&AtomicBool) -> Result<Completion, String> + Send + 'static,
        message: &str,
    ) -> bool {
        self.cancel.store(false, Ordering::Release);
        self.state.running = true;
        self.state.message = message.into();
        self.state.sequence += 1;
        let cancel = self.cancel.clone();
        let complete = self.complete.clone();
        match std::thread::Builder::new()
            .name("sd300-storage-action".into())
            .spawn(move || {
                let result =
                    std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| job(&cancel)))
                        .unwrap_or_else(|_| Err("Storage action failed unexpectedly".into()));
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
                self.state.message = "Could not start storage action".into();
                false
            }
        }
    }
    pub fn cancel(&mut self) {
        if self.state.awaiting_consent {
            self.state.message =
                "Privileged read declined; ordinary monitoring remains available".into();
        } else if self.state.running {
            self.state.message = "Cancelling the bounded storage action".into();
        }
        self.cancel.store(true, Ordering::Release);
        self.prepared = None;
        self.state.awaiting_consent = false;
        self.state.notice.clear();
        self.state.sequence += 1;
    }
    pub fn poll(&mut self) -> bool {
        if !self.worker.as_ref().is_some_and(|w| w.is_finished()) {
            return false;
        }
        let _ = self.worker.take().unwrap().join();
        self.state.running = false;
        self.state.sequence += 1;
        match self.complete.lock().ok().and_then(|mut slot| slot.take()) {
            Some(Ok(Completion::Prepared(request))) if !self.cancel.load(Ordering::Acquire) => {
                self.state.notice = request.notice();
                self.state.awaiting_consent = true;
                self.prepared = Some(request);
                self.state.message =
                    "Review the exact operation before allowing the privileged read".into();
            }
            Some(Ok(Completion::Read(result))) if !self.cancel.load(Ordering::Acquire) => {
                self.state.message =
                    "Storage read finished; captured results are separate from periodic monitoring"
                        .into();
                self.state.result = Some(*result);
            }
            Some(Err(message)) => self.state.message = message,
            _ => self.state.message = "Storage action cancelled".into(),
        }
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
    fn sockets() -> (TcpStream, TcpStream) {
        let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).unwrap();
        let client = TcpStream::connect(listener.local_addr().unwrap()).unwrap();
        let (server, _) = listener.accept().unwrap();
        server
            .set_read_timeout(Some(Duration::from_millis(20)))
            .unwrap();
        server
            .set_write_timeout(Some(Duration::from_millis(100)))
            .unwrap();
        (server, client)
    }
    fn fake_device(number: &str) -> String {
        #[cfg(windows)]
        {
            format!(r"\\.\PHYSICALDRIVE{number}")
        }
        #[cfg(target_os = "macos")]
        {
            format!("/dev/disk{number}")
        }
        #[cfg(target_os = "linux")]
        {
            format!("/dev/nvme{number}n1")
        }
    }
    #[test]
    fn changed_helper_is_rejected_before_execution_and_results_are_redacted() {
        let temp = tempfile::tempdir().unwrap();
        let helper = temp.path().join("smartctl");
        std::fs::write(&helper, b"changed executable").unwrap();
        let request = Prepared {
            product_version: env!("CARGO_PKG_VERSION").into(),
            device: fake_device("99999999"),
            helper,
            sha256: "0".repeat(64),
            expected_serial: None,
        };
        let (mut server, _peer) = sockets();
        assert!(read_prepared(&mut server, request, &AtomicBool::new(false))
            .unwrap_err()
            .contains("changed after confirmation"));
        let mut state = State {
            notice: "private/path".into(),
            message: "private detail".into(),
            ..Default::default()
        };
        let mut drive = disk_health::empty_drive(
            "private-id".into(),
            "Model".into(),
            disk_health::MediaType::Unknown,
        );
        drive.serial = Some("private-serial".into());
        state.result = Some(ProbeResult {
            captured_unix_ms: 100,
            drive,
            observation: Observation::error("smartctl", "private probe detail"),
            elevated: true,
        });
        let encoded = serde_json::to_string(&state.redacted()).unwrap();
        assert!(!encoded.contains("private"));
        assert_eq!(
            state.result.unwrap().drive.serial.as_deref(),
            Some("private-serial")
        );
    }
    #[test]
    fn framed_messages_reject_excess_output_and_have_an_absolute_deadline() {
        let (mut server, mut client) = sockets();
        client
            .write_all(&((MAX_MESSAGE + 1) as u32).to_le_bytes())
            .unwrap();
        assert!(receive::<String>(&mut server)
            .unwrap_err()
            .contains("bound"));
        let (mut server, _client) = sockets();
        let at = Instant::now();
        assert!(
            read_exact_until(&mut server, &mut [0; 4], at + Duration::from_millis(25)).is_err()
        );
        assert!(at.elapsed() < Duration::from_secs(1));
        assert!(read_worker(0, &"a".repeat(64)).is_err());
    }
    #[test]
    fn native_read_fixture_imports_faults_and_disconnect_cancels_the_owned_child() {
        let _guard = command::TEST_PROCESS_GUARD
            .lock()
            .unwrap_or_else(|p| p.into_inner());
        let temp = tempfile::tempdir().unwrap();
        let helper = temp
            .path()
            .join(crate::optional_tools::executable_name("smartctl"));
        let mut compiler = Command::new("rustc");
        compiler
            .arg("--edition=2021")
            .arg("-O")
            .arg(Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/smartctl_probe.rs"))
            .arg("-o")
            .arg(&helper);
        let output = command::run_memory_command(
            &mut compiler,
            command::CommandTimeout::Custom(Duration::from_secs(30)),
            &AtomicBool::new(false),
        )
        .unwrap();
        assert!(
            output.failure.is_none() && output.status.is_some_and(|s| s.success()),
            "fixture compiler: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        let request = Prepared {
            product_version: env!("CARGO_PKG_VERSION").into(),
            device: fake_device("99999999"),
            sha256: format!("{:x}", Sha256::digest(helper_bytes(&helper).unwrap())),
            helper,
            expected_serial: Some("FIXTURE-ALPHA".into()),
        };
        let (mut server, peer) = sockets();
        let result = read_prepared(&mut server, request.clone(), &AtomicBool::new(false)).unwrap();
        assert_eq!(
            result.drive.health_status,
            disk_health::DiskHealthStatus::Critical
        );
        assert_eq!(result.drive.temperature_celsius, Some(41.0));
        assert!(result.observation.is_available());
        drop(peer);
        let (mut server, peer) = sockets();
        let closer = std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(100));
            drop(peer);
        });
        let mut hung = request;
        hung.device = fake_device("99999998");
        let started = Instant::now();
        let result = read_prepared(&mut server, hung, &AtomicBool::new(false)).unwrap();
        closer.join().unwrap();
        assert!(started.elapsed() < Duration::from_secs(2));
        assert!(!result.observation.is_available());
        assert!(result
            .observation
            .detail
            .as_deref()
            .unwrap()
            .contains("cancelled"));
    }

    #[test]
    fn device_allowlists_exclude_options_partitions_and_shell_syntax() {
        assert_eq!(
            windows_device(r"\\.\PHYSICALDRIVE12").as_deref(),
            Some("/dev/pd12")
        );
        assert!(macos_device("/dev/disk4"));
        for good in ["/dev/sda", "/dev/vda", "/dev/nvme0n1", "/dev/mmcblk0"] {
            assert!(linux_device(good));
        }
        for bad in [
            "",
            "--test=long",
            "/dev/sda1",
            "/dev/../sda",
            "/dev/nvme0n1p1",
            "/dev/sda;id",
            "/dev/sda\n",
        ] {
            assert!(!linux_device(bad));
        }
        assert!(!macos_device("/dev/disk4s1"));
        assert_eq!(windows_device(r"\\server\disk"), None);
    }
    #[test]
    fn consent_is_bound_to_a_prepared_request_and_never_inferred() {
        let mut controller = Controller::default();
        assert!(!controller.confirm(true));
        controller.prepared = Some(Prepared {
            product_version: env!("CARGO_PKG_VERSION").into(),
            device: "fixture".into(),
            helper: PathBuf::from("fixture"),
            sha256: "fixture".into(),
            expected_serial: None,
        });
        assert!(!controller.confirm(false));
        assert!(!controller.state.running);
        assert!(controller.prepared.is_none());
        assert!(!controller.confirm(true));
    }
    #[test]
    fn applescript_quotes_the_executable_as_data() {
        let script = apple_script(
            "/Applications/A \"quoted\" $app/sd300",
            1234,
            &"a".repeat(64),
        );
        assert!(script.contains("quoted form of"));
        assert!(script.contains("\\\"quoted\\\""));
        assert!(script.ends_with("with administrator privileges"));
        assert!(!nonce_valid("a;id"));
    }
}
