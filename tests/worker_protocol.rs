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
