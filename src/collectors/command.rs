//! Bounded collector execution without reader threads or inherited-pipe EOF waits.
use std::ffi::OsStr;
use std::io::{Read, Seek, SeekFrom};
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
struct OwnedProcess(u32);
#[cfg(unix)]
impl OwnedProcess {
    fn new(child: &Child) -> std::io::Result<Self> {
        Ok(Self(child.id()))
    }
    fn terminate(&self) {
        unsafe {
            libc::kill(-(self.0 as i32), libc::SIGKILL);
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
mod tests {
    use super::*;
    #[test]
    fn missing_provider_and_cancellation_are_distinct() {
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
        #[cfg(unix)]
        let result = run_checked(
            "sh",
            ["-c", "head -c 10000000 /dev/zero"],
            CommandTimeout::Slow,
            &AtomicBool::new(false),
        );
        #[cfg(windows)]
        let result = run_checked(
            "powershell",
            [
                "-NoProfile",
                "-Command",
                "[Console]::OpenStandardOutput().Write((New-Object byte[] 10000000),0,10000000)",
            ],
            CommandTimeout::Slow,
            &AtomicBool::new(false),
        );
        assert!(
            matches!(result, Err(CommandError::OutputLimit)),
            "{result:?}"
        );
    }
    #[cfg(unix)]
    #[test]
    fn inherited_output_does_not_wait_for_descendant_eof() {
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
}
