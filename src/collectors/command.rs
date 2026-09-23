//! Bounded collector execution without reader threads or inherited-pipe EOF waits.
use std::ffi::OsStr;
use std::io::{Read, Seek, SeekFrom, Write};
use std::process::{Child, Command, Output, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommandTimeout {
    Quick,
    Normal,
    Slow,
    Custom(Duration),
}
impl CommandTimeout {
    pub fn duration(self) -> Duration {
        match self {
            Self::Quick => Duration::from_millis(750),
            Self::Normal => Duration::from_secs(2),
            Self::Slow => Duration::from_millis(7_500),
            Self::Custom(duration) => duration,
        }
    }
}
pub const MAX_OUTPUT_BYTES: u64 = 8 * 1024 * 1024;

// Every helper has explicit redirected standard handles. Detaching avoids a
// headless console host; suspension still lets us own the entire job before
// any helper code runs. Do not combine DETACHED_PROCESS with CREATE_NO_WINDOW.
#[cfg(windows)]
const HELPER_CREATION_FLAGS: u32 =
    winapi::um::winbase::DETACHED_PROCESS | winapi::um::winbase::CREATE_SUSPENDED;

#[derive(Debug, thiserror::Error)]
pub enum CommandError {
    #[error("provider executable was not found")]
    NotFound,
    #[error("provider access was denied")]
    PermissionDenied,
    #[error("provider exceeded its deadline")]
    Timeout,
    #[error("provider was cancelled")]
    Cancelled,
    #[error("provider output exceeded the capture limit")]
    OutputLimit,
    #[error("provider response did not match the bounded worker protocol")]
    Protocol,
    #[error("provider exited unsuccessfully ({0})")]
    Exit(std::process::ExitStatus),
    #[error("provider returned text with an invalid UTF-8 encoding")]
    Encoding,
    #[error("provider process failed: {0}")]
    Io(#[from] std::io::Error),
}
fn classify(error: std::io::Error) -> CommandError {
    match error.kind() {
        std::io::ErrorKind::NotFound => CommandError::NotFound,
        std::io::ErrorKind::PermissionDenied => CommandError::PermissionDenied,
        _ => CommandError::Io(error),
    }
}

pub fn run_checked<P, I, S>(
    program: P,
    args: I,
    timeout: CommandTimeout,
    cancelled: &AtomicBool,
) -> Result<Output, CommandError>
where
    P: AsRef<OsStr>,
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    let deadline = Instant::now() + timeout.duration();
    if cancelled.load(Ordering::Relaxed) {
        return Err(CommandError::Cancelled);
    }
    let mut stdout = tempfile::tempfile()?;
    let mut stderr = tempfile::tempfile()?;
    let mut command = Command::new(program);
    command
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::from(stdout.try_clone()?))
        .stderr(Stdio::from(stderr.try_clone()?));
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        command.process_group(0).env("LC_ALL", "C").env("LANG", "C");
    }
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        // Assign the job before resuming so even fast descendants are owned.
        command.creation_flags(HELPER_CREATION_FLAGS);
    }
    let mut child = command.spawn().map_err(classify)?;
    let owned = match OwnedProcess::new(&child) {
        Ok(owned) => owned,
        Err(error) => {
            let _ = child.kill();
            let _ = child.wait();
            return Err(error.into());
        }
    };
    let result = (|| loop {
        if cancelled.load(Ordering::Relaxed) {
            return Err(CommandError::Cancelled);
        }
        if Instant::now() >= deadline {
            return Err(CommandError::Timeout);
        }
        if stdout
            .metadata()?
            .len()
            .saturating_add(stderr.metadata()?.len())
            > MAX_OUTPUT_BYTES
        {
            return Err(CommandError::OutputLimit);
        }
        if let Some(status) = child.try_wait()? {
            owned.terminate();
            let read = |file: &mut std::fs::File| -> Result<Vec<u8>, CommandError> {
                file.seek(SeekFrom::Start(0))?;
                let mut bytes = Vec::new();
                file.take(MAX_OUTPUT_BYTES + 1).read_to_end(&mut bytes)?;
                if bytes.len() as u64 > MAX_OUTPUT_BYTES {
                    return Err(CommandError::OutputLimit);
                }
                Ok(bytes)
            };
            let out = read(&mut stdout)?;
            let err = read(&mut stderr)?;
            if out.len().saturating_add(err.len()) as u64 > MAX_OUTPUT_BYTES {
                return Err(CommandError::OutputLimit);
            }
            return Ok(Output {
                status,
                stdout: out,
                stderr: err,
            });
        }
        std::thread::sleep(Duration::from_millis(10));
    })();
    owned.terminate();
    let _ = child.wait();
    result
}

/// Private in-memory capture for user-requested diagnostics. No result payload is
/// written to disk; there are no reader threads or waits for inherited-pipe EOF.
#[derive(Debug)]
pub struct MemoryCapture {
    pub status: Option<std::process::ExitStatus>,
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
    pub failure: Option<CommandError>,
}

#[cfg(unix)]
fn ready_bytes<T: std::os::fd::AsRawFd>(pipe: &T) -> std::io::Result<usize> {
    let mut available: libc::c_int = 0;
    // This handle has exactly one reader, so available bytes cannot be consumed
    // between this query and read. Never issue a read when no bytes are ready.
    if unsafe { libc::ioctl(pipe.as_raw_fd(), libc::FIONREAD, &mut available) } < 0 {
        return Err(std::io::Error::last_os_error());
    }
    Ok(available.max(0) as usize)
}
#[cfg(windows)]
fn ready_bytes<T: std::os::windows::io::AsRawHandle>(pipe: &T) -> std::io::Result<usize> {
    use std::ptr::null_mut;
    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn PeekNamedPipe(
            handle: *mut std::ffi::c_void,
            buffer: *mut std::ffi::c_void,
            size: u32,
            read: *mut u32,
            available: *mut u32,
            left: *mut u32,
        ) -> i32;
    }
    let mut available = 0;
    // Sole owner: no concurrent synchronous operation can hold this pipe handle.
    if unsafe {
        PeekNamedPipe(
            pipe.as_raw_handle(),
            null_mut(),
            0,
            null_mut(),
            &mut available,
            null_mut(),
        )
    } == 0
    {
        let error = std::io::Error::last_os_error();
        if matches!(error.raw_os_error(), Some(109 | 232)) {
            return Ok(0);
        }
        return Err(error);
    }
    Ok(available as usize)
}

pub fn run_memory<P, I, S>(
    program: P,
    args: I,
    timeout: CommandTimeout,
    cancelled: &AtomicBool,
) -> Result<MemoryCapture, CommandError>
where
    P: AsRef<OsStr>,
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    let mut command = Command::new(program);
    command.args(args);
    run_memory_command(&mut command, timeout, cancelled)
}

/// Caller may set a fixed working directory/environment, while ownership,
/// pipes, deadlines and cancellation remain mandatory here.
pub fn run_memory_command(
    command: &mut Command,
    timeout: CommandTimeout,
    cancelled: &AtomicBool,
) -> Result<MemoryCapture, CommandError> {
    if cancelled.load(Ordering::Acquire) {
        return Err(CommandError::Cancelled);
    }
    command
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        command.process_group(0).env("LC_ALL", "C").env("LANG", "C");
    }
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(HELPER_CREATION_FLAGS);
    }
    let mut child = command.spawn().map_err(classify)?;
    let owned = match OwnedProcess::new(&child) {
        Ok(owned) => owned,
        Err(error) => {
            let _ = child.kill();
            let _ = child.wait();
            return Err(error.into());
        }
    };
    let mut stdout = child.stdout.take().expect("piped stdout");
    let mut stderr = child.stderr.take().expect("piped stderr");
    let mut capture = MemoryCapture {
        status: None,
        stdout: Vec::new(),
        stderr: Vec::new(),
        failure: None,
    };
    let deadline = Instant::now() + timeout.duration();
    let mut buffer = [0u8; 16 * 1024];
    let mut drain = || -> Result<(), CommandError> {
        loop {
            // Bound work per cycle so a noisy producer cannot starve cancellation.
            for _ in 0..4 {
                let out = ready_bytes(&stdout)?.min(buffer.len());
                let err = ready_bytes(&stderr)?.min(buffer.len());
                if out == 0 && err == 0 {
                    break;
                }
                if capture
                    .stdout
                    .len()
                    .saturating_add(capture.stderr.len())
                    .saturating_add(out)
                    .saturating_add(err)
                    > MAX_OUTPUT_BYTES as usize
                {
                    return Err(CommandError::OutputLimit);
                }
                if out > 0 {
                    let n = stdout.read(&mut buffer[..out])?;
                    capture.stdout.extend_from_slice(&buffer[..n]);
                }
                if err > 0 {
                    let n = stderr.read(&mut buffer[..err])?;
                    capture.stderr.extend_from_slice(&buffer[..n]);
                }
            }
            if cancelled.load(Ordering::Acquire) {
                return Err(CommandError::Cancelled);
            }
            if Instant::now() >= deadline {
                return Err(CommandError::Timeout);
            }
            if let Some(status) = child.try_wait()? {
                capture.status = Some(status);
                owned.terminate();
                // Collect the finite bytes already queued, never wait for EOF.
                for _ in 0..(MAX_OUTPUT_BYTES as usize / buffer.len() + 1) {
                    let out = ready_bytes(&stdout)?.min(buffer.len());
                    let err = ready_bytes(&stderr)?.min(buffer.len());
                    if out == 0 && err == 0 {
                        return Ok(());
                    }
                    if capture.stdout.len() + capture.stderr.len() + out + err
                        > MAX_OUTPUT_BYTES as usize
                    {
                        return Err(CommandError::OutputLimit);
                    }
                    if out > 0 {
                        let n = stdout.read(&mut buffer[..out])?;
                        capture.stdout.extend_from_slice(&buffer[..n]);
                    }
                    if err > 0 {
                        let n = stderr.read(&mut buffer[..err])?;
                        capture.stderr.extend_from_slice(&buffer[..n]);
                    }
                }
                return Err(CommandError::OutputLimit);
            }
            std::thread::sleep(Duration::from_millis(10));
        }
    };
    capture.failure = drain().err();
    owned.terminate();
    let _ = child.wait();
    Ok(capture)
}

/// One reusable owned process, with one bounded request outstanding. Requests are
/// smaller than a pipe's minimum capacity; length-framed responses stay in memory.
/// Only already-readable pipe bytes are consumed, never waiting for inherited EOF.
pub struct WorkerProcess {
    child: Child,
    owned: OwnedProcess,
    stdout: std::process::ChildStdout,
    stderr: std::process::ChildStderr,
}
impl WorkerProcess {
    pub fn spawn(program: &OsStr, topic: &str) -> Result<Self, CommandError> {
        Self::spawn_configured(program, || vec!["collect-server".into(), topic.into()])
    }
    pub fn process_id(&self) -> u32 {
        self.child.id()
    }
    fn spawn_configured(
        program: &OsStr,
        args: impl FnOnce() -> Vec<std::ffi::OsString>,
    ) -> Result<Self, CommandError> {
        let mut command = Command::new(program);
        command
            .args(args())
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        #[cfg(unix)]
        {
            use std::os::unix::process::CommandExt;
            command.process_group(0).env("LC_ALL", "C").env("LANG", "C");
        }
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            command.creation_flags(HELPER_CREATION_FLAGS);
        }
        let mut child = command.spawn().map_err(classify)?;
        let owned = match OwnedProcess::new(&child) {
            Ok(owned) => owned,
            Err(error) => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(error.into());
            }
        };
        let stdout = child.stdout.take().expect("piped worker stdout");
        let stderr = child.stderr.take().expect("piped worker stderr");
        Ok(Self {
            child,
            owned,
            stdout,
            stderr,
        })
    }
    pub fn request(
        &mut self,
        reset: bool,
        timeout: Duration,
        cancelled: &AtomicBool,
    ) -> Result<Vec<u8>, CommandError> {
        let result = self.read_response(reset, timeout, cancelled);
        if result.is_err() {
            self.owned.terminate();
            let _ = self.child.wait();
        }
        result
    }
    fn read_response(
        &mut self,
        reset: bool,
        timeout: Duration,
        cancelled: &AtomicBool,
    ) -> Result<Vec<u8>, CommandError> {
        let deadline = Instant::now() + timeout;
        if cancelled.load(Ordering::Relaxed) {
            return Err(CommandError::Cancelled);
        }
        // Exactly one two-byte message can be pending. No reader/replacement threads.
        self.child
            .stdin
            .as_mut()
            .ok_or_else(|| std::io::Error::other("worker input closed"))?
            .write_all(if reset { b"r\n" } else { b"s\n" })?;
        let mut frame = WorkerFrame::default();
        let mut errors = Vec::new();
        let mut error_bytes = 0usize;
        let mut buffer = [0u8; 16 * 1024];
        loop {
            // Bounded work per iteration leaves cancellation/deadline observable
            // even when native code writes continuously or inherits output pipes.
            let mut progressed = false;
            for _ in 0..4 {
                let out = ready_bytes(&self.stdout)?.min(buffer.len());
                let err = ready_bytes(&self.stderr)?.min(buffer.len());
                if out == 0 && err == 0 {
                    break;
                }
                progressed = true;
                if out > 0 {
                    let n = self.stdout.read(&mut buffer[..out])?;
                    frame.feed(&buffer[..n])?;
                }
                if err > 0 {
                    let n = self.stderr.read(&mut buffer[..err])?;
                    error_bytes = error_bytes.saturating_add(n);
                    let keep = n.min(2048usize.saturating_sub(errors.len()));
                    errors.extend_from_slice(&buffer[..keep]);
                }
                if frame.payload.len().saturating_add(error_bytes) > MAX_OUTPUT_BYTES as usize {
                    return Err(CommandError::OutputLimit);
                }
            }
            if cancelled.load(Ordering::Acquire) {
                return Err(CommandError::Cancelled);
            }
            if Instant::now() >= deadline {
                return Err(CommandError::Timeout);
            }
            if frame.complete() {
                return Ok(frame.payload);
            }
            if let Some(status) = self.child.try_wait()? {
                // A reaped child may leave queued bytes or a descendant holding
                // stdout. Drain only queued data, then fail; never wait for EOF.
                if ready_bytes(&self.stdout)? > 0 || ready_bytes(&self.stderr)? > 0 {
                    continue;
                }
                return Err(std::io::Error::other(format!(
                    "collector worker exited before a complete response ({status}): {}",
                    String::from_utf8_lossy(&errors)
                ))
                .into());
            }
            if !progressed {
                std::thread::sleep(Duration::from_millis(10));
            }
        }
    }
}
impl Drop for WorkerProcess {
    fn drop(&mut self) {
        self.owned.terminate();
        let _ = self.child.wait();
    }
}

#[derive(Default)]
struct WorkerFrame {
    header: [u8; 8],
    header_len: usize,
    expected: Option<usize>,
    payload: Vec<u8>,
}
impl WorkerFrame {
    fn feed(&mut self, mut bytes: &[u8]) -> Result<(), CommandError> {
        if self.header_len < self.header.len() {
            let count = bytes.len().min(self.header.len() - self.header_len);
            self.header[self.header_len..self.header_len + count].copy_from_slice(&bytes[..count]);
            self.header_len += count;
            bytes = &bytes[count..];
            if self.header_len < self.header.len() {
                return Ok(());
            }
            if &self.header[..4] != b"SD4\0" {
                return Err(CommandError::Protocol);
            }
            let length = u32::from_le_bytes(self.header[4..].try_into().unwrap()) as usize;
            if length == 0 {
                return Err(CommandError::Protocol);
            }
            if length > MAX_OUTPUT_BYTES as usize {
                return Err(CommandError::OutputLimit);
            }
            self.expected = Some(length);
            self.payload.reserve_exact(length);
        }
        let expected = self.expected.ok_or(CommandError::Protocol)?;
        if self.payload.len().saturating_add(bytes.len()) > expected {
            return Err(CommandError::Protocol);
        }
        self.payload.extend_from_slice(bytes);
        Ok(())
    }
    fn complete(&self) -> bool {
        self.expected == Some(self.payload.len())
    }
}

/// Preserve execution failures separately from the provider's exit status.
pub fn run_output<P, I, S>(
    program: P,
    args: I,
    timeout: CommandTimeout,
) -> Result<Output, CommandError>
where
    P: AsRef<OsStr>,
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    run_checked(program, args, timeout, &AtomicBool::new(false))
}
pub fn run_status<P, I, S>(
    program: P,
    args: I,
    timeout: CommandTimeout,
) -> Result<bool, CommandError>
where
    P: AsRef<OsStr>,
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    run_output(program, args, timeout).map(|output| output.status.success())
}
pub fn run_stdout<P, I, S>(
    program: P,
    args: I,
    timeout: CommandTimeout,
) -> Result<String, CommandError>
where
    P: AsRef<OsStr>,
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    let output = run_output(program, args, timeout)?;
    decode_stdout(output)
}

fn decode_stdout(output: Output) -> Result<String, CommandError> {
    if !output.status.success() {
        return Err(CommandError::Exit(output.status));
    }
    String::from_utf8(output.stdout).map_err(|_| CommandError::Encoding)
}

#[cfg(unix)]
struct OwnedProcess(std::cell::Cell<Option<u32>>);
#[cfg(unix)]
impl OwnedProcess {
    fn new(child: &Child) -> std::io::Result<Self> {
        Ok(Self(std::cell::Cell::new(Some(child.id()))))
    }
    fn terminate(&self) {
        if let Some(pid) = self.0.take() {
            unsafe {
                libc::kill(-(pid as i32), libc::SIGKILL);
            }
        }
    }
}
#[cfg(unix)]
impl Drop for OwnedProcess {
    fn drop(&mut self) {
        self.terminate();
    }
}

#[cfg(windows)]
struct OwnedProcess(winapi::um::winnt::HANDLE);
/// The child is still CREATE_SUSPENDED, so only its initial thread exists.
/// Capture that process alone rather than enumerating every system thread for
/// every helper invocation. PSS descriptors belong to this calling process.
#[cfg(windows)]
fn suspended_thread_id(child: &Child) -> std::io::Result<u32> {
    use std::{mem::size_of, os::windows::io::AsRawHandle, ptr};
    use windows_sys::Win32::System::Diagnostics::ProcessSnapshotting::*;
    struct Snapshot(HPSS);
    impl Drop for Snapshot {
        fn drop(&mut self) {
            unsafe {
                PssFreeSnapshot(
                    winapi::um::processthreadsapi::GetCurrentProcess().cast(),
                    self.0,
                );
            }
        }
    }
    struct Marker(HPSSWALK);
    impl Drop for Marker {
        fn drop(&mut self) {
            unsafe {
                PssWalkMarkerFree(self.0);
            }
        }
    }
    let check = |code: u32| {
        if code == 0 {
            Ok(())
        } else {
            Err(std::io::Error::from_raw_os_error(code as i32))
        }
    };
    unsafe {
        let mut snapshot = ptr::null_mut();
        check(PssCaptureSnapshot(
            child.as_raw_handle(),
            PSS_CAPTURE_THREADS,
            0,
            &mut snapshot,
        ))?;
        let snapshot = Snapshot(snapshot);
        let mut marker = ptr::null_mut();
        check(PssWalkMarkerCreate(ptr::null(), &mut marker))?;
        let marker = Marker(marker);
        let mut entry = PSS_THREAD_ENTRY::default();
        check(PssWalkSnapshot(
            snapshot.0,
            PSS_WALK_THREADS,
            marker.0,
            (&mut entry as *mut PSS_THREAD_ENTRY).cast(),
            size_of::<PSS_THREAD_ENTRY>() as u32,
        ))?;
        if entry.ProcessId != child.id() || entry.ThreadId == 0 {
            return Err(std::io::Error::other(
                "owned child thread identity did not match",
            ));
        }
        Ok(entry.ThreadId)
    }
}
#[cfg(windows)]
impl OwnedProcess {
    fn new(child: &Child) -> std::io::Result<Self> {
        use std::{
            mem::{size_of, zeroed},
            os::windows::io::AsRawHandle,
            ptr,
        };
        use winapi::um::{handleapi::*, jobapi2::*, processthreadsapi::*, tlhelp32::*, winnt::*};
        unsafe {
            let job = CreateJobObjectW(ptr::null_mut(), ptr::null());
            if job.is_null() {
                return Err(std::io::Error::last_os_error());
            }
            let owned = Self(job);
            let mut limits: JOBOBJECT_EXTENDED_LIMIT_INFORMATION = zeroed();
            limits.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
            if SetInformationJobObject(
                job,
                JobObjectExtendedLimitInformation,
                &mut limits as *mut _ as *mut _,
                size_of::<JOBOBJECT_EXTENDED_LIMIT_INFORMATION>() as u32,
            ) == 0
                || AssignProcessToJobObject(job, child.as_raw_handle() as HANDLE) == 0
            {
                return Err(std::io::Error::last_os_error());
            }
            if let Ok(id) = suspended_thread_id(child) {
                let thread = OpenThread(THREAD_SUSPEND_RESUME, 0, id);
                if !thread.is_null() {
                    let resumed = ResumeThread(thread) != u32::MAX;
                    CloseHandle(thread);
                    if resumed {
                        return Ok(owned);
                    }
                }
            }
            // Compatibility fallback for restricted hosts. The job already
            // owns the child; any failure below closes it without running it.
            let snapshot = CreateToolhelp32Snapshot(TH32CS_SNAPTHREAD, 0);
            if snapshot == INVALID_HANDLE_VALUE {
                return Err(std::io::Error::last_os_error());
            }
            let mut entry: THREADENTRY32 = zeroed();
            entry.dwSize = size_of::<THREADENTRY32>() as u32;
            let mut found = false;
            let mut valid = Thread32First(snapshot, &mut entry);
            while valid != 0 {
                if entry.th32OwnerProcessID == child.id() {
                    let thread = OpenThread(THREAD_SUSPEND_RESUME, 0, entry.th32ThreadID);
                    if !thread.is_null() {
                        found = ResumeThread(thread) != u32::MAX;
                        CloseHandle(thread);
                    }
                    break;
                }
                valid = Thread32Next(snapshot, &mut entry);
            }
            CloseHandle(snapshot);
            if !found {
                return Err(std::io::Error::other(
                    "could not resume owned collector process",
                ));
            }
            Ok(owned)
        }
    }
    fn terminate(&self) {
        unsafe {
            winapi::um::jobapi2::TerminateJobObject(self.0, 1);
        }
    }
}
#[cfg(windows)]
impl Drop for OwnedProcess {
    fn drop(&mut self) {
        self.terminate();
        unsafe {
            winapi::um::handleapi::CloseHandle(self.0);
        }
    }
}

#[cfg(test)]
pub(crate) static TEST_PROCESS_GUARD: std::sync::Mutex<()> = std::sync::Mutex::new(());

#[cfg(test)]
pub(crate) fn test_fixture(name: &str) -> (std::path::PathBuf, Vec<String>) {
    (
        std::env::current_exe().expect("test executable"),
        vec![
            "--exact".into(),
            format!("collectors::command::tests::{name}"),
            "--ignored".into(),
            "--nocapture".into(),
        ],
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn text_commands_distinguish_missing_invalid_and_unsuccessful_output() {
        assert!(matches!(
            run_stdout(
                "sd300-fixture-executable-does-not-exist-7ca491",
                ["--version"],
                CommandTimeout::Quick
            ),
            Err(CommandError::NotFound)
        ));
        #[cfg(unix)]
        use std::os::unix::process::ExitStatusExt;
        #[cfg(windows)]
        use std::os::windows::process::ExitStatusExt;
        let output = |status, stdout| Output {
            status: std::process::ExitStatus::from_raw(status),
            stdout,
            stderr: b"not part of the text result".to_vec(),
        };
        assert!(matches!(
            decode_stdout(output(0, vec![0xff])),
            Err(CommandError::Encoding)
        ));
        assert!(matches!(
            decode_stdout(output(256, b"misleading success text".to_vec())),
            Err(CommandError::Exit(_))
        ));
        assert_eq!(
            decode_stdout(output(0, b"valid".to_vec())).unwrap(),
            "valid"
        );
    }

    #[test]
    fn worker_framing_handles_fragmentation_and_rejects_invalid_lengths() {
        let bytes = [b"SD4\0".as_slice(), &3u32.to_le_bytes(), b"abc"].concat();
        for split in 0..bytes.len() {
            let mut frame = WorkerFrame::default();
            frame.feed(&bytes[..split]).unwrap();
            assert!(!frame.complete());
            frame.feed(&bytes[split..]).unwrap();
            assert!(frame.complete());
            assert_eq!(frame.payload, b"abc");
            assert!(matches!(
                frame.feed(b"trailing"),
                Err(CommandError::Protocol)
            ));
        }
        for length in [0, MAX_OUTPUT_BYTES as u32 + 1, u32::MAX] {
            let mut frame = WorkerFrame::default();
            assert!(frame
                .feed(&[b"SD4\0".as_slice(), &length.to_le_bytes()].concat())
                .is_err());
            assert_eq!(
                frame.payload.capacity(),
                0,
                "invalid length must not allocate"
            );
        }
        assert!(matches!(
            WorkerFrame::default().feed(b"bad frame"),
            Err(CommandError::Protocol)
        ));
    }

    #[test]
    fn native_worker_pipes_bound_large_partial_noisy_and_inherited_output() {
        let _guard = TEST_PROCESS_GUARD.lock().unwrap_or_else(|p| p.into_inner());
        let temp = tempfile::tempdir().unwrap();
        let helper = temp.path().join(if cfg!(windows) {
            "pipe-fixture.exe"
        } else {
            "pipe-fixture"
        });
        let fixture = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/collector_pipe.rs");
        let output = run_checked(
            "rustc",
            [
                OsStr::new("--edition=2021"),
                OsStr::new("-O"),
                fixture.as_os_str(),
                OsStr::new("-o"),
                helper.as_os_str(),
            ],
            CommandTimeout::Custom(Duration::from_secs(30)),
            &AtomicBool::new(false),
        )
        .unwrap();
        assert!(output.status.success(), "{output:?}");
        let mut worker = WorkerProcess::spawn(helper.as_os_str(), "large").unwrap();
        let pid = worker.process_id();
        for _ in 0..3 {
            let bytes = worker
                .request(false, Duration::from_secs(5), &AtomicBool::new(false))
                .unwrap();
            assert_eq!(bytes.len(), 1024 * 1024);
            assert!(bytes.iter().all(|&b| b == b'a'));
            assert_eq!(worker.process_id(), pid);
        }
        drop(worker);
        for topic in ["invalid", "oversized", "noisy", "inherited", "timeout"] {
            let mut worker = WorkerProcess::spawn(helper.as_os_str(), topic).unwrap();
            let start = Instant::now();
            let timeout = if topic == "timeout" {
                Duration::from_millis(150)
            } else {
                Duration::from_secs(5)
            };
            let error = worker
                .request(false, timeout, &AtomicBool::new(false))
                .unwrap_err();
            match topic {
                "invalid" => assert!(
                    matches!(error, CommandError::Protocol),
                    "{topic}: {error:?}; elapsed {:?}",
                    start.elapsed()
                ),
                "oversized" | "noisy" => assert!(
                    matches!(error, CommandError::OutputLimit),
                    "{topic}: {error:?}; elapsed {:?}",
                    start.elapsed()
                ),
                "inherited" => assert!(
                    matches!(error, CommandError::Io(_)),
                    "{topic}: {error:?}; elapsed {:?}",
                    start.elapsed()
                ),
                "timeout" => assert!(
                    matches!(error, CommandError::Timeout),
                    "{topic}: {error:?}; elapsed {:?}",
                    start.elapsed()
                ),
                _ => unreachable!(),
            }
            assert!(
                worker.child.try_wait().unwrap().is_some(),
                "failed requests must reap their owner"
            );
            assert!(start.elapsed() < Duration::from_secs(5), "{topic}: {error}");
        }
    }
    #[cfg(windows)]
    #[test]
    #[ignore = "child-only console and redirected handle probe"]
    fn fixture_no_console() {
        let mut processes = [0u32; 4];
        assert_eq!(
            unsafe { winapi::um::wincon::GetConsoleProcessList(processes.as_mut_ptr(), 4) },
            0,
            "background helpers must not allocate a console host"
        );
        std::io::stdout()
            .write_all(b"redirected stdout works")
            .unwrap();
        std::io::stderr()
            .write_all(b"redirected stderr works")
            .unwrap();
    }
    #[cfg(windows)]
    #[test]
    fn detached_helpers_keep_redirected_handles_without_console_allocation() {
        let _guard = TEST_PROCESS_GUARD.lock().unwrap_or_else(|p| p.into_inner());
        let (program, args) = test_fixture("fixture_no_console");
        let output = run_checked(
            &program,
            &args,
            CommandTimeout::Slow,
            &AtomicBool::new(false),
        )
        .unwrap();
        assert!(output.status.success(), "{output:?}");
        assert!(String::from_utf8_lossy(&output.stdout).contains("redirected stdout works"));
        assert!(String::from_utf8_lossy(&output.stderr).contains("redirected stderr works"));
        let output =
            run_memory(program, args, CommandTimeout::Slow, &AtomicBool::new(false)).unwrap();
        assert!(output.failure.is_none(), "{:?}", output.failure);
        assert!(output.status.is_some_and(|s| s.success()));
        assert!(String::from_utf8_lossy(&output.stdout).contains("redirected stdout works"));
        assert!(String::from_utf8_lossy(&output.stderr).contains("redirected stderr works"));
    }
    #[cfg(windows)]
    #[test]
    fn suspended_child_snapshot_finds_its_thread_without_system_enumeration() {
        use std::os::windows::process::CommandExt;
        let _guard = TEST_PROCESS_GUARD.lock().unwrap_or_else(|p| p.into_inner());
        let (program, args) = test_fixture("fixture_hung");
        let mut child = Command::new(program)
            .args(args)
            .creation_flags(HELPER_CREATION_FLAGS)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .unwrap();
        let result = suspended_thread_id(&child);
        child.kill().unwrap();
        child.wait().unwrap();
        assert!(result.is_ok(), "{result:?}");
    }
    #[test]
    #[ignore = "child-only deterministic producer"]
    fn fixture_one_mebibyte() {
        std::io::stdout().write_all(&vec![0; 1024 * 1024]).unwrap();
    }
    #[test]
    #[ignore = "child-only deterministic producer"]
    fn fixture_excess_output() {
        std::io::stdout().write_all(&vec![0; 10_000_000]).unwrap();
    }
    #[test]
    #[ignore = "child-only deterministic hung probe"]
    fn fixture_hung() {
        std::thread::sleep(Duration::from_secs(30));
    }
    #[test]
    #[ignore = "child-only partial output probe"]
    fn fixture_partial_hung() {
        std::io::stdout().write_all(br#"{"ok":true}"#).unwrap();
        std::io::stdout().flush().unwrap();
        std::thread::sleep(Duration::from_secs(30));
    }
    #[test]
    fn memory_capture_bounds_output_and_retains_completed_output() {
        let _guard = TEST_PROCESS_GUARD.lock().unwrap_or_else(|p| p.into_inner());
        let (program, args) = test_fixture("fixture_partial_hung");
        let result = run_memory(
            program,
            args,
            CommandTimeout::Custom(Duration::from_secs(2)),
            &AtomicBool::new(false),
        )
        .unwrap();
        assert!(matches!(result.failure, Some(CommandError::Timeout)));
        assert!(String::from_utf8_lossy(&result.stdout).contains(r#"{"ok":true}"#));
        let (program, args) = test_fixture("fixture_excess_output");
        let result =
            run_memory(program, args, CommandTimeout::Slow, &AtomicBool::new(false)).unwrap();
        assert!(
            matches!(result.failure, Some(CommandError::OutputLimit)),
            "{:?}",
            result.failure
        );
        assert!(result.stdout.len() + result.stderr.len() <= MAX_OUTPUT_BYTES as usize);
    }
    #[cfg(unix)]
    #[test]
    fn memory_capture_never_waits_for_descendant_eof() {
        let _guard = TEST_PROCESS_GUARD.lock().unwrap_or_else(|p| p.into_inner());
        let start = Instant::now();
        let result = run_memory(
            "sh",
            ["-c", "sleep 30 & printf ok"],
            CommandTimeout::Normal,
            &AtomicBool::new(false),
        )
        .unwrap();
        assert!(result.failure.is_none());
        assert_eq!(result.stdout, b"ok");
        assert!(start.elapsed() < Duration::from_secs(1));
    }
    #[test]
    fn missing_provider_and_cancellation_are_distinct() {
        let _guard = TEST_PROCESS_GUARD.lock().unwrap_or_else(|p| p.into_inner());
        assert!(matches!(
            run_checked(
                "sd300-provider-does-not-exist-9471",
                [""],
                CommandTimeout::Quick,
                &AtomicBool::new(false)
            ),
            Err(CommandError::NotFound)
        ));
        assert!(matches!(
            run_checked(
                "sd300-provider-does-not-exist-9471",
                [""],
                CommandTimeout::Quick,
                &AtomicBool::new(true)
            ),
            Err(CommandError::Cancelled)
        ));
    }
    #[test]
    fn output_is_bounded() {
        let _guard = TEST_PROCESS_GUARD.lock().unwrap_or_else(|p| p.into_inner());
        let (program, args) = test_fixture("fixture_excess_output");
        let result = run_checked(program, args, CommandTimeout::Slow, &AtomicBool::new(false));
        assert!(
            matches!(result, Err(CommandError::OutputLimit)),
            "{result:?}"
        );
    }
    #[cfg(unix)]
    #[test]
    fn inherited_output_does_not_wait_for_descendant_eof() {
        let _guard = TEST_PROCESS_GUARD.lock().unwrap_or_else(|p| p.into_inner());
        let start = Instant::now();
        let result = run_checked(
            "sh",
            ["-c", "sleep 30 & printf ok"],
            CommandTimeout::Normal,
            &AtomicBool::new(false),
        )
        .unwrap();
        assert_eq!(result.stdout, b"ok");
        assert!(start.elapsed() < Duration::from_secs(1));
    }
    #[test]
    fn persistent_worker_timeout_and_cancellation_do_not_wait_for_stdin_or_native_return() {
        let _guard = TEST_PROCESS_GUARD.lock().unwrap_or_else(|p| p.into_inner());
        #[cfg(windows)]
        let program = OsStr::new("powershell");
        #[cfg(unix)]
        let program = OsStr::new("sh");
        let mut worker = WorkerProcess::spawn_configured(program, || {
            #[cfg(windows)]
            let args = [
                "-NoProfile",
                "-Command",
                "[Console]::ReadLine() | Out-Null; Start-Sleep -Seconds 30",
            ];
            #[cfg(unix)]
            let args = ["-c", "read value; sleep 30"];
            args.into_iter().map(Into::into).collect()
        })
        .unwrap();
        let started = Instant::now();
        assert!(matches!(
            worker.request(false, Duration::from_millis(75), &AtomicBool::new(false)),
            Err(CommandError::Timeout)
        ));
        drop(worker);
        assert!(started.elapsed() < Duration::from_secs(2));
    }
}
