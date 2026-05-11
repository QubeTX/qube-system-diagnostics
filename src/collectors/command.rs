use std::ffi::OsStr;
use std::process::{Command, Output, Stdio};
use std::thread;
use std::time::{Duration, Instant};

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

pub fn run_output<I, S>(program: &str, args: I, timeout: CommandTimeout) -> Option<Output>
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    let mut child = Command::new(program)
        .args(args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .ok()?;

    let deadline = Instant::now() + timeout.duration();
    loop {
        match child.try_wait() {
            Ok(Some(_)) => return child.wait_with_output().ok(),
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

pub fn run_status<I, S>(program: &str, args: I, timeout: CommandTimeout) -> Option<bool>
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    run_output(program, args, timeout).map(|output| output.status.success())
}

pub fn run_stdout<I, S>(program: &str, args: I, timeout: CommandTimeout) -> Option<String>
where
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
