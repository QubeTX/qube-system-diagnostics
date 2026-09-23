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
        command.creation_flags(0x0800_0000 | 0x0000_0004);
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
    if cancelled.load(Ordering::Acquire) {
        return Err(CommandError::Cancelled);
    }
    let mut command = Command::new(program);
    command
        .args(args)
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
        command.creation_flags(0x0800_0000 | 0x0000_0004);
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
/// smaller than a pipe's minimum capacity; responses use atomic files, never a
/// draining thread or an inherited stdout pipe.
pub struct WorkerProcess {
    child: Child,
    owned: OwnedProcess,
    directory: tempfile::TempDir,
    stdout: std::fs::File,
    stderr: std::fs::File,
}
impl WorkerProcess {
    pub fn spawn(program: &OsStr, topic: &str) -> Result<Self, CommandError> {
        Self::spawn_configured(program, |response| {
            vec![
                "collect-server".into(),
                topic.into(),
                "--response".into(),
                response.as_os_str().to_owned(),
            ]
        })
    }
    pub fn process_id(&self) -> u32 {
        self.child.id()
    }
    fn spawn_configured(
        program: &OsStr,
        args: impl FnOnce(&std::path::Path) -> Vec<std::ffi::OsString>,
    ) -> Result<Self, CommandError> {
        let directory = tempfile::tempdir()?;
        let stdout = tempfile::tempfile()?;
        let stderr = tempfile::tempfile()?;
        let mut command = Command::new(program);
        command
            .args(args(&directory.path().join("response.json")))
            .stdin(Stdio::piped())
            .stdout(stdout.try_clone()?)
            .stderr(stderr.try_clone()?);
        #[cfg(unix)]
        {
            use std::os::unix::process::CommandExt;
            command.process_group(0).env("LC_ALL", "C").env("LANG", "C");
        }
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            command.creation_flags(0x0800_0000 | 0x0000_0004);
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
        Ok(Self {
            child,
            owned,
            directory,
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
        let response = self.directory.path().join("response.json");
        loop {
            if cancelled.load(Ordering::Relaxed) {
                return Err(CommandError::Cancelled);
            }
            if Instant::now() >= deadline {
                return Err(CommandError::Timeout);
            }
            if self
                .stdout
                .metadata()?
                .len()
                .saturating_add(self.stderr.metadata()?.len())
                > MAX_OUTPUT_BYTES
            {
                return Err(CommandError::OutputLimit);
            }
            match std::fs::File::open(&response) {
                Ok(file) => {
                    if file.metadata()?.len() > MAX_OUTPUT_BYTES {
                        return Err(CommandError::OutputLimit);
                    }
                    let mut bytes = Vec::new();
                    file.take(MAX_OUTPUT_BYTES + 1).read_to_end(&mut bytes)?;
                    if bytes.len() as u64 > MAX_OUTPUT_BYTES {
                        return Err(CommandError::OutputLimit);
                    }
                    std::fs::remove_file(&response)?;
                    return Ok(bytes);
                }
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
                Err(e) => return Err(e.into()),
            }
            if let Some(status) = self.child.try_wait()? {
                self.stderr.seek(SeekFrom::Start(0))?;
                let mut error = String::new();
                Read::by_ref(&mut self.stderr)
                    .take(2048)
                    .read_to_string(&mut error)?;
                return Err(std::io::Error::other(format!(
                    "collector worker exited with {status}: {error}"
                ))
                .into());
            }
            std::thread::sleep(Duration::from_millis(10));
        }
    }
}
impl Drop for WorkerProcess {
    fn drop(&mut self) {
        self.owned.terminate();
        let _ = self.child.wait();
    }
}

/// Compatibility adapter while individual providers migrate to explicit errors.
pub fn run_output<P, I, S>(program: P, args: I, timeout: CommandTimeout) -> Option<Output>
where
    P: AsRef<OsStr>,
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    run_checked(program, args, timeout, &AtomicBool::new(false)).ok()
}
pub fn run_status<P, I, S>(program: P, args: I, timeout: CommandTimeout) -> Option<bool>
where
    P: AsRef<OsStr>,
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    run_output(program, args, timeout).map(|output| output.status.success())
}
pub fn run_stdout<P, I, S>(program: P, args: I, timeout: CommandTimeout) -> Option<String>
where
    P: AsRef<OsStr>,
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    let output = run_output(program, args, timeout)?;
    output
        .status
        .success()
        .then(|| String::from_utf8_lossy(&output.stdout).into_owned())
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
        let mut worker = WorkerProcess::spawn_configured(program, |_| {
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
