//! Opt-in hosted qualification. Uses preauthorized runner privileges and a
//! native synthetic helper, never a real disk or an interactive auth prompt.
use sd_300::collectors::command::{run_memory_command, CommandTimeout};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::io::{Read, Write};
use std::net::{Ipv4Addr, TcpListener};
use std::process::Command;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use std::time::{Duration, Instant};

#[test]
#[ignore = "runs a synthetic privileged worker only on explicitly opted-in native runners"]
fn privileged_worker_uses_confirmed_bytes_and_returns_a_bounded_structured_result() {
    let temp = tempfile::Builder::new()
        .prefix("storage probe qualification ")
        .tempdir()
        .unwrap();
    // Root on a Unix runner must be able to traverse the isolated user temp.
    let helper = temp
        .path()
        .join(sd_300::optional_tools::executable_name("smartctl"));
    let mut compiler = Command::new("rustc");
    compiler
        .args(["--edition=2021", "-O"])
        .arg(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("tests/fixtures/smartctl_probe.rs"),
        )
        .arg("-o")
        .arg(&helper);
    let output = run_memory_command(
        &mut compiler,
        CommandTimeout::Custom(Duration::from_secs(30)),
        &AtomicBool::new(false),
    )
    .unwrap();
    assert!(
        output.failure.is_none() && output.status.is_some_and(|s| s.success()),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let hash = format!("{:x}", Sha256::digest(std::fs::read(&helper).unwrap()));
    let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).unwrap();
    listener.set_nonblocking(true).unwrap();
    let port = listener.local_addr().unwrap().port().to_string();
    let mut random = [0u8; 32];
    getrandom::fill(&mut random).unwrap();
    let nonce: String = random.iter().map(|b| format!("{b:02x}")).collect();
    let cli = env!("CARGO_BIN_EXE_sd300");
    #[cfg(windows)]
    let mut command = Command::new(cli);
    #[cfg(unix)]
    let mut command = {
        // Keep the directly owned parent unelevated even when sudo changes the
        // worker UID. The wrapper contains no interpolated path or user data.
        let mut c = Command::new("/bin/sh");
        c.args(["-c", "\"$@\"; result=$?; exit \"$result\"", "sd300-test"]);
        if unsafe { libc::geteuid() } != 0 {
            c.args(["/usr/bin/sudo", "-n"]);
        }
        c.arg(cli);
        c
    };
    command.args(["storage-probe-read", &port, &nonce]);
    let cancel = Arc::new(AtomicBool::new(false));
    let runner_cancel = cancel.clone();
    let runner = std::thread::spawn(move || {
        run_memory_command(
            &mut command,
            CommandTimeout::Custom(Duration::from_secs(30)),
            &runner_cancel,
        )
    });
    let deadline = Instant::now() + Duration::from_secs(10);
    let mut stream = loop {
        match listener.accept() {
            Ok((stream, _)) => break Some(stream),
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {}
            Err(e) => panic!("callback accept: {e}"),
        }
        if runner.is_finished() || Instant::now() >= deadline {
            break None;
        }
        std::thread::sleep(Duration::from_millis(10));
    };
    if stream.is_none() {
        cancel.store(true, Ordering::Release);
        let result = runner.join().unwrap().unwrap();
        panic!(
            "Privileged worker did not connect: exit={:?}, failure={:?}, stderr={}",
            result.status,
            result.failure,
            String::from_utf8_lossy(&result.stderr)
        );
    }
    let mut stream = stream.take().unwrap();
    // accept inherits nonblocking mode on Windows/macOS but not Linux.
    stream.set_nonblocking(false).unwrap();
    stream
        .set_read_timeout(Some(Duration::from_secs(5)))
        .unwrap();
    stream
        .set_write_timeout(Some(Duration::from_secs(1)))
        .unwrap();
    let mut read_frame = || {
        let mut len = [0; 4];
        stream.read_exact(&mut len).unwrap();
        let len = u32::from_le_bytes(len) as usize;
        assert!(len <= 65536);
        let mut bytes = vec![0; len];
        stream.read_exact(&mut bytes).unwrap();
        serde_json::from_slice::<Value>(&bytes).unwrap()
    };
    assert_eq!(read_frame(), nonce);
    #[cfg(windows)]
    let device = r"\\.\PHYSICALDRIVE99999999";
    #[cfg(target_os = "macos")]
    let device = "/dev/disk99999999";
    #[cfg(target_os = "linux")]
    let device = "/dev/nvme99999999n1";
    let request = serde_json::to_vec(
        &json!({"product_version":env!("CARGO_PKG_VERSION"),"device":device,"helper":helper,"sha256":hash,"expected_serial":"FIXTURE-ALPHA"}),
    )
    .unwrap();
    stream
        .write_all(&(request.len() as u32).to_le_bytes())
        .unwrap();
    stream.write_all(&request).unwrap();
    let mut len = [0; 4];
    stream.read_exact(&mut len).unwrap();
    let len = u32::from_le_bytes(len) as usize;
    assert!(len <= 65536);
    let mut bytes = vec![0; len];
    stream.read_exact(&mut bytes).unwrap();
    let result: Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(result["Ok"]["elevated"], true, "{result}");
    assert_eq!(result["Ok"]["drive"]["device_id"], device);
    assert_eq!(result["Ok"]["drive"]["health_status"], "critical");
    assert_eq!(result["Ok"]["drive"]["temperature_celsius"], 41.0);
    drop(stream);
    let output = runner.join().unwrap().unwrap();
    assert!(
        output.failure.is_none() && output.status.is_some_and(|s| s.success()),
        "worker exit={:?}, failure={:?}, stderr={}",
        output.status,
        output.failure,
        String::from_utf8_lossy(&output.stderr)
    );
    println!("Qualified privileged structured storage worker on {}-{} using synthetic helper bytes; no device was opened", std::env::consts::OS, std::env::consts::ARCH);
}
