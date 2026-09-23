use sd_300::collectors::command::{CommandError, WorkerProcess};
use std::{
    sync::atomic::AtomicBool,
    time::{Duration, Instant},
};
#[test]
fn collector_session_reuses_one_process_then_cancels_cleanly() {
    let mut worker = WorkerProcess::spawn(
        std::ffi::OsStr::new(env!("CARGO_BIN_EXE_sd300")),
        "activity",
    )
    .unwrap();
    let pid = worker.process_id();
    for (index, reset) in [false, false, true].into_iter().enumerate() {
        let output = worker
            .request(reset, Duration::from_secs(10), &AtomicBool::new(false))
            .unwrap();
        let value: serde_json::Value = serde_json::from_slice(&output).unwrap();
        assert_eq!(value["version"], env!("CARGO_PKG_VERSION"));
        assert_eq!(value["data"]["topic"], "activity");
        assert!(value["captured_unix_ms"].as_u64().unwrap() > 0);
        assert!(value["interval_ms"].is_u64());
        if index == 0 || reset {
            assert_eq!(value["interval_ms"], 0);
            for device in value["data"]["data"]["devices"].as_array().unwrap() {
                assert!(device["read_bytes_per_sec"].is_null());
            }
        }
        assert_eq!(worker.process_id(), pid);
    }
    assert!(matches!(
        worker.request(false, Duration::from_secs(1), &AtomicBool::new(true)),
        Err(CommandError::Cancelled)
    ));
    let started = Instant::now();
    drop(worker);
    assert!(started.elapsed() < Duration::from_secs(2));
}

#[test]
fn native_endpoint_inventory_matches_owned_tcp_and_udp_sockets() {
    use std::net::{TcpListener, TcpStream, UdpSocket};
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    let client = TcpStream::connect(listener.local_addr().unwrap()).unwrap();
    let (_accepted, _) = listener.accept().unwrap();
    let client_port = client.local_addr().unwrap().port();
    let udp = UdpSocket::bind("127.0.0.1:0").unwrap();
    let udp_port = udp.local_addr().unwrap().port();
    let ipv6 = TcpListener::bind("[::1]:0").ok();
    let mut worker = WorkerProcess::spawn(
        std::ffi::OsStr::new(env!("CARGO_BIN_EXE_sd300")),
        "connections",
    )
    .unwrap();
    let bytes = worker
        .request(false, Duration::from_secs(10), &AtomicBool::new(false))
        .unwrap();
    let result: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    let data = &result["data"]["data"];
    assert_eq!(
        data["connections_observation"]["status"], "available",
        "{}",
        data["connections_observation"]
    );
    let rows = data["active_connections"].as_array().unwrap();
    let matches = |protocol: &str, address: &str, port: u16, state: Option<&str>| {
        rows.iter().any(|row| {
            row["protocol"] == protocol
                && row["local_addr"] == address
                && row["local_port"] == port
                && state.is_none_or(|state| row["state"] == state)
        })
    };
    assert!(
        matches("tcp", "127.0.0.1", port, Some("listening")),
        "owned TCP listener absent"
    );
    assert!(
        matches("tcp", "127.0.0.1", client_port, Some("established")),
        "owned TCP client absent"
    );
    assert!(
        matches("udp", "127.0.0.1", udp_port, None),
        "owned UDP endpoint absent"
    );
    if let Some(listener) = &ipv6 {
        assert!(
            matches(
                "tcp",
                "::1",
                listener.local_addr().unwrap().port(),
                Some("listening")
            ),
            "owned IPv6 listener absent"
        );
    }
    #[cfg(windows)]
    assert!(rows
        .iter()
        .any(|r| r["local_port"] == port && r["pid"] == std::process::id()));
    #[cfg(target_os = "linux")]
    {
        // A minimal host must still expose endpoints without iproute2. Do not
        // install a provider merely to make this availability test pass.
        let mut command = std::process::Command::new(env!("CARGO_BIN_EXE_sd300"));
        command
            .args(["collect-worker", "connections"])
            .env("PATH", "");
        let output = sd_300::collectors::command::run_memory_command(
            &mut command,
            sd_300::collectors::command::CommandTimeout::Custom(Duration::from_secs(10)),
            &AtomicBool::new(false),
        )
        .unwrap();
        assert!(output.failure.is_none());
        let value: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
        let data = &value["data"]["data"];
        assert_eq!(data["connections_observation"]["status"], "available");
        assert!(data["connections_observation"]["source"]
            .as_str()
            .unwrap()
            .contains("procfs"));
        assert!(data["active_connections"]
            .as_array()
            .unwrap()
            .iter()
            .any(|row| row["protocol"] == "tcp"
                && row["local_port"] == port
                && row["pid"].is_null()));
    }
}
