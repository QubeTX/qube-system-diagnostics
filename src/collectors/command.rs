use std::ffi::OsStr;
use std::io::Read;
use std::process::{Command, Output, Stdio};
use std::thread;
use std::time::{Duration, Instant};

#[cfg(windows)]
use std::os::windows::process::CommandExt;

// Collector commands are non-interactive probes whose output is always captured. A
// GUI-subsystem parent has no console to inherit, so Windows would otherwise create
// a visible console window for helpers such as ping, netstat, route, or nvidia-smi.
// Keep this scoped to the collector helper; installer/UAC launches use their own
// explicit Command paths in update.rs.
#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommandTimeout {
    Quick,
    Normal,
    Slow,
    Custom(Duration),
}

impl CommandTimeout {
    fn duration(self) -> Duration {
        match self {
            Self::Quick => Duration::from_millis(750),
            Self::Normal => Duration::from_millis(2_000),
            Self::Slow => Duration::from_millis(7_500),
            Self::Custom(duration) => duration,
        }
    }
}

pub fn run_output<P, I, S>(program: P, args: I, timeout: CommandTimeout) -> Option<Output>
where
    P: AsRef<OsStr>,
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    let mut command = Command::new(program);
    command
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    #[cfg(windows)]
    command.creation_flags(CREATE_NO_WINDOW);

    let mut child = command.spawn().ok()?;
    let mut stdout = child.stdout.take()?;
    let mut stderr = child.stderr.take()?;
    let stdout_reader = thread::spawn(move || {
        let mut bytes = Vec::new();
        let _ = stdout.read_to_end(&mut bytes);
        bytes
    });
    let stderr_reader = thread::spawn(move || {
        let mut bytes = Vec::new();
        let _ = stderr.read_to_end(&mut bytes);
        bytes
    });

    let deadline = Instant::now() + timeout.duration();
    loop {
        match child.try_wait() {
            Ok(Some(status)) => {
                return Some(Output {
                    status,
                    stdout: stdout_reader.join().ok()?,
                    stderr: stderr_reader.join().ok()?,
                });
            }
            Ok(None) => {
                if Instant::now() >= deadline {
                    let _ = child.kill();
                    let _ = child.wait();
                    return None;
                }
                thread::sleep(Duration::from_millis(10));
            }
            Err(_) => {
                let _ = child.kill();
                let _ = child.wait();
                return None;
            }
        }
    }
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
    if output.status.success() {
        Some(String::from_utf8_lossy(&output.stdout).to_string())
    } else {
        None
    }
}
