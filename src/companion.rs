//! Optional, explicit ND-300/SpeedQX actions. Never scheduled by a collector.
use crate::collectors::command::{self, CommandError, CommandTimeout};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::path::{Path, PathBuf};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex,
};
use std::time::{Duration, Instant};

pub const VERIFIED_VERSION: &str = "4.0.1";
pub const MLAB_NOTICE: &str = "M-Lab publishes measurement results and your IP address. Enable M-Lab only if you consent to that publication.";
pub const SPEED_NOTICE: &str = "SpeedQX measures sustained application throughput on this device and path. Quick: up to 90 seconds / 5 GB synthetic payload. Deep: up to 300 seconds / 20 GB. Protocol overhead and in-flight overshoot are additional; this is not a line-speed guarantee.";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Action {
    Standard,
    Deep,
    SpeedQuick,
    SpeedDeep,
}
impl Action {
    pub fn label(self) -> &'static str {
        match self {
            Self::Standard => "standard diagnostic",
            Self::Deep => "deep diagnostic",
            Self::SpeedQuick => "SpeedQX Quick",
            Self::SpeedDeep => "SpeedQX Deep",
        }
    }
    pub fn is_speed(self) -> bool {
        matches!(self, Self::SpeedQuick | Self::SpeedDeep)
    }
    pub fn budget_seconds(self) -> u64 {
        match self {
            Self::Standard => 90,
            Self::Deep => 240,
            Self::SpeedQuick => 90,
            Self::SpeedDeep => 300,
        }
    }
    pub fn budget_bytes(self) -> Option<u64> {
        match self {
            Self::SpeedQuick => Some(5_000_000_000),
            Self::SpeedDeep => Some(20_000_000_000),
            _ => None,
        }
    }
    pub fn arguments(self, mlab_consent: bool) -> Vec<String> {
        let mut args = vec!["--json".into()];
        match self {
            Self::Standard => args.push("--fast".into()),
            Self::Deep => args.extend(["--tech".into(), "--fast".into()]),
            Self::SpeedQuick | Self::SpeedDeep => {
                if self == Self::SpeedDeep {
                    args.push("--deep".into());
                }
                args.extend([
                    "--max-bytes".into(),
                    self.budget_bytes().unwrap().to_string(),
                ]);
                if mlab_consent {
                    args.push("--accept-mlab".into());
                }
            }
        }
        args
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Failure {
    Missing,
    UnsupportedVersion,
    PermissionDenied,
    Timeout,
    Cancelled,
    OutputLimit,
    MalformedOutput,
    ProcessFailed,
    Internal,
}
impl Failure {
    pub fn explanation(self) -> &'static str {
        match self {
        Self::Missing => "ND-300 or SpeedQX is not installed. Monitoring remains available; optional setup requires consent.",
        Self::UnsupportedVersion => "This executable is not the verified public ND-300 4.0.1 interface. Keep its existing installation; choose a supported companion explicitly.",
        Self::PermissionDenied => "The operating system denied access to the companion executable.",
        Self::Timeout => "The companion exceeded its deadline. Unfinished checks do not prove a network fault.",
        Self::Cancelled => "The diagnostic was cancelled. Previously completed results remain available.",
        Self::OutputLimit => "The companion exceeded the bounded result size.",
        Self::MalformedOutput => "The companion did not return the supported JSON structure.",
        Self::ProcessFailed => "The companion exited without a valid diagnostic outcome.",
        Self::Internal => "The companion worker failed. Monitoring continues; retry is available.",
    }
    }
}
impl From<CommandError> for Failure {
    fn from(value: CommandError) -> Self {
        match value {
            CommandError::NotFound => Self::Missing,
            CommandError::PermissionDenied => Self::PermissionDenied,
            CommandError::Timeout => Self::Timeout,
            CommandError::Cancelled => Self::Cancelled,
            CommandError::OutputLimit => Self::OutputLimit,
            CommandError::Io(_) => Self::ProcessFailed,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CheckState {
    Ok,
    Warning,
    Fault,
    Skipped,
    Incomplete,
}
impl CheckState {
    pub fn explanation(self) -> &'static str {
        match self {
            Self::Ok => "Check completed successfully",
            Self::Warning => "Check reported a warning",
            Self::Fault => "Check reported a failure",
            Self::Skipped => "Check was not run",
            Self::Incomplete => "Check did not finish; no confirmed fault",
        }
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Check {
    pub category: String,
    pub state: CheckState,
    pub summary: String,
    pub details: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpeedDirection {
    pub sustained_mbps: Option<f64>,
    pub ceiling_mbps: Option<f64>,
    pub measured_ms: Option<f64>,
    pub measured_bytes: Option<u64>,
    pub qualification: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpeedSummary {
    pub methodology_version: String,
    pub download: SpeedDirection,
    pub upload: SpeedDirection,
    pub http_idle_rtt_ms: Option<f64>,
    pub jitter_p95_minus_p50_ms: Option<f64>,
    pub bytes_transferred: Option<u64>,
    pub elapsed_ms: Option<f64>,
    pub stop_reason: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResultData {
    pub action: Action,
    pub version: String,
    pub captured_unix_ms: u64,
    pub exit_code: Option<i32>,
    pub incomplete: bool,
    pub failure: Option<Failure>,
    pub checks: Vec<Check>,
    pub speed: Option<SpeedSummary>,
    pub detail_lines: Vec<String>,
    /// Session-only raw evidence. Export must make an explicit privacy choice.
    #[serde(skip)]
    pub raw: Value,
}
impl ResultData {
    pub fn export(&self, include_sensitive: bool) -> Value {
        let mut value = serde_json::to_value(self).unwrap_or(Value::Null);
        if include_sensitive {
            value["raw"] = self.raw.clone();
        } else {
            // Never try to regex-redact arbitrary imported text. Export only
            // validated categories, states and numeric measurement semantics.
            value["detail_lines"] = json!([]);
            for row in value["checks"].as_array_mut().into_iter().flatten() {
                let state: CheckState =
                    serde_json::from_value(row["state"].clone()).unwrap_or(CheckState::Incomplete);
                row["summary"] = json!(state.explanation());
                row["details"] = Value::Null;
            }
        }
        value
    }
}
fn text(value: &Value, max: usize) -> String {
    value
        .as_str()
        .unwrap_or("")
        .chars()
        .filter(|c| !c.is_control() || *c == '\n')
        .take(max)
        .collect()
}
fn numeric(value: &Value) -> Option<f64> {
    value.as_f64().filter(|v| v.is_finite() && *v >= 0.0)
}
fn known(value: &Value, allowed: &[&str]) -> String {
    value
        .as_str()
        .filter(|s| allowed.contains(s))
        .unwrap_or("unknown")
        .into()
}
fn direction(value: &Value) -> SpeedDirection {
    SpeedDirection {
        sustained_mbps: numeric(&value["sustained_mbps"]),
        ceiling_mbps: numeric(&value["ceiling_mbps"]),
        measured_ms: numeric(&value["measured_ms"]),
        measured_bytes: value["measured_bytes"].as_u64(),
        qualification: known(
            &value["qualification"],
            &["measured", "unavailable", "insufficient", "provisional"],
        ),
    }
}

fn flatten_detail(prefix: &str, value: &Value, depth: usize, lines: &mut Vec<String>) {
    if lines.len() >= 255 {
        return;
    }
    if depth > 5 {
        lines.push(format!(
            "{prefix}: nested detail retained in sensitive export"
        ));
        return;
    }
    match value {
        Value::Object(map) => {
            for (key, value) in map.iter().take(32) {
                flatten_detail(
                    &format!(
                        "{prefix}.{}",
                        key.chars()
                            .filter(|c| !c.is_control())
                            .take(80)
                            .collect::<String>()
                    ),
                    value,
                    depth + 1,
                    lines,
                );
            }
        }
        Value::Array(rows) => {
            for (index, value) in rows.iter().take(16).enumerate() {
                flatten_detail(&format!("{prefix}[{index}]"), value, depth + 1, lines);
            }
            if rows.len() > 16 && lines.len() < 255 {
                lines.push(format!(
                    "{prefix}: {} entries; first 16 shown, full evidence retained in session",
                    rows.len()
                ));
            }
        }
        _ => {
            let value = if value.is_string() {
                text(value, 512)
            } else {
                value.to_string()
            };
            lines.push(format!("{prefix}: {value}"));
        }
    }
}

pub fn parse_results(
    action: Action,
    version: &str,
    bytes: &[u8],
    exit: Option<i32>,
    failure: Option<Failure>,
) -> Result<ResultData, Failure> {
    if version != VERIFIED_VERSION {
        return Err(Failure::UnsupportedVersion);
    }
    if bytes.len() as u64 > command::MAX_OUTPUT_BYTES {
        return Err(Failure::OutputLimit);
    }
    let raw: Value =
        serde_json::from_slice(bytes).map_err(|_| failure.unwrap_or(Failure::MalformedOutput))?;
    if raw["interrupted"] == true && raw["timestamp"].is_null() {
        return Err(Failure::Cancelled);
    }
    if !matches!(exit, Some(0..=2 | 130) | None) && failure.is_none() {
        return Err(Failure::ProcessFailed);
    }
    let mut result = ResultData {
        action,
        version: VERIFIED_VERSION.into(),
        captured_unix_ms: crate::collectors::sampling::unix_ms(),
        exit_code: exit,
        incomplete: raw["timed_out"] == true || failure.is_some() || exit == Some(130),
        failure: if exit == Some(130) {
            Some(Failure::Cancelled)
        } else {
            failure
        },
        checks: Vec::new(),
        speed: None,
        detail_lines: Vec::new(),
        raw: Value::Null,
    };
    if action.is_speed() {
        let measurement = &raw["measurement"];
        if raw["methodology_version"] != "5.0"
            || !measurement["download"].is_object()
            || !measurement["upload"].is_object()
        {
            return Err(Failure::MalformedOutput);
        }
        result.speed = Some(SpeedSummary {
            methodology_version: "5.0".into(),
            download: direction(&measurement["download"]),
            upload: direction(&measurement["upload"]),
            http_idle_rtt_ms: numeric(&raw["ping_ms"]),
            jitter_p95_minus_p50_ms: numeric(&raw["jitter_ms"]),
            bytes_transferred: measurement["bytes_transferred"].as_u64(),
            elapsed_ms: numeric(&measurement["elapsed_ms"]),
            stop_reason: known(
                &measurement["stop_reason"],
                &[
                    "complete",
                    "cancelled",
                    "time-limit",
                    "byte-limit",
                    "network-change",
                ],
            ),
        });
    } else {
        if !raw["timestamp"].is_string() {
            return Err(Failure::MalformedOutput);
        }
        for category in [
            "adapters",
            "interfaces",
            "gateway",
            "dns",
            "public_ip",
            "latency",
            "speed",
            "ports",
        ] {
            let row = &raw[category];
            if row.is_null() {
                result.incomplete = true;
                continue;
            }
            let state = if row["timed_out"] == true {
                result.incomplete = true;
                CheckState::Incomplete
            } else {
                match row["status"].as_str() {
                    Some("Ok") => CheckState::Ok,
                    Some("Warn") => CheckState::Warning,
                    Some("Fail") => CheckState::Fault,
                    Some("Skip") => CheckState::Skipped,
                    _ => return Err(Failure::MalformedOutput),
                }
            };
            result.checks.push(Check {
                category: category.into(),
                state,
                summary: text(&row["summary"], 1024),
                details: row["details"].as_str().map(|_| text(&row["details"], 4096)),
            });
        }
        if result.checks.is_empty() {
            return Err(Failure::MalformedOutput);
        }
    }
    let mut detail = |prefix: &str, value: &Value| {
        if !value.is_null() {
            flatten_detail(prefix, value, 0, &mut result.detail_lines);
        }
    };
    for key in [
        "interface_details",
        "adapter_details",
        "gateway_details",
        "dns_details",
        "public_ip_details",
        "latency_details",
        "port_details",
    ] {
        detail(key, &raw[key]);
    }
    for key in [
        "arp_table",
        "routing_table",
        "active_connections",
        "listening_ports",
        "dhcp_info",
        "protocol_stats",
        "adapter_hw_stats",
        "proxy_config",
        "vpn_info",
        "firewall_info",
        "dns_cache",
        "ipv6_info",
        "mtu_info",
        "connection_states",
        "bufferbloat",
        "reverse_dns",
        "tls_inspection",
        "traffic_counters",
        "route_path",
        "packet_loss",
        "nat_analysis",
        "wifi",
        "dns_benchmark",
        "captive_portal",
        "clock_sync",
        "path_mtu",
        "arp_health",
    ] {
        detail(&format!("technician.{key}"), &raw["technician"][key]);
    }
    if action.is_speed() {
        for key in ["measurement", "http_latency", "providers", "warnings"] {
            detail(key, &raw[key]);
        }
    }
    result.raw = raw;
    Ok(result)
}

pub fn verify_executable(path: &Path, speed: bool, cancel: &AtomicBool) -> Result<String, Failure> {
    let output = command::run_memory(path, ["--version"], CommandTimeout::Slow, cancel)
        .map_err(Failure::from)?;
    if let Some(failure) = output.failure {
        return Err(failure.into());
    }
    if !output.status.is_some_and(|s| s.success()) {
        return Err(Failure::ProcessFailed);
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut tokens = stdout.split_whitespace();
    let product = tokens.next().unwrap_or("").to_ascii_lowercase();
    let version = tokens.next().unwrap_or("");
    if product != if speed { "speedqx" } else { "nd300" }
        || version != VERIFIED_VERSION
        || tokens.next().is_some()
    {
        return Err(Failure::UnsupportedVersion);
    }
    Ok(version.into())
}

/// Resolve PATH explicitly, avoiding implicit current-directory search on Windows.
pub fn detect(speed: bool) -> Result<PathBuf, Failure> {
    crate::optional_tools::detect(if speed { "speedqx" } else { "nd300" }).ok_or(Failure::Missing)
}
pub fn run(action: Action, mlab_consent: bool, cancel: &AtomicBool) -> Result<ResultData, Failure> {
    let executable = detect(action.is_speed())?;
    let version = verify_executable(&executable, action.is_speed(), cancel)?;
    // Allow ND-300's own deadline to produce its partial JSON before terminating.
    // This grace period does not increase the SpeedQX measurement budget.
    let timeout = Duration::from_secs(action.budget_seconds() + 15);
    let output = command::run_memory(
        &executable,
        action.arguments(mlab_consent),
        CommandTimeout::Custom(timeout),
        cancel,
    )
    .map_err(Failure::from)?;
    parse_results(
        action,
        &version,
        &output.stdout,
        output.status.and_then(|s| s.code()),
        output.failure.map(Into::into),
    )
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct State {
    pub sequence: u64,
    pub running: bool,
    pub action: Option<Action>,
    pub elapsed_ms: u64,
    pub failure: Option<Failure>,
    pub result: Option<ResultData>,
}
fn reading(value: Option<f64>, units: &str) -> String {
    value.map_or_else(|| "unavailable".into(), |v| format!("{v:.1} {units}"))
}

impl State {
    pub fn export(&self, sensitive: bool) -> Value {
        if self.sequence == 0 {
            return Value::Null;
        }
        let mut value = serde_json::to_value(self).unwrap_or(Value::Null);
        if let Some(result) = &self.result {
            value["result"] = result.export(sensitive);
        }
        value
    }
    pub fn lines(&self) -> Vec<String> {
        let mut lines = vec![
            "Optional ND-300 4.0.1 diagnostics; results stay in this session until exported."
                .into(),
        ];
        if self.running {
            lines.push(format!(
                "Running {} · {} seconds elapsed · X cancels",
                self.action.map_or("diagnostic", Action::label),
                self.elapsed_ms / 1000
            ));
        }
        if let Some(failure) = self.failure {
            lines.push(failure.explanation().into());
        }
        if let Some(result) = &self.result {
            lines.push(format!(
                "Captured at {} ms UTC · ND-300 {} · exit {:?}{}",
                result.captured_unix_ms,
                result.version,
                result.exit_code,
                if result.incomplete { " · PARTIAL" } else { "" }
            ));
            for check in &result.checks {
                lines.push(format!(
                    "{} — {}: {}",
                    check.category,
                    check.state.explanation(),
                    check.summary
                ));
            }
            if let Some(speed) = &result.speed {
                lines.push(format!(
                    "Sustained download: {} · upload: {}",
                    reading(speed.download.sustained_mbps, "Mbps"),
                    reading(speed.upload.sustained_mbps, "Mbps")
                ));
                lines.push(format!(
                    "HTTP idle RTT: {} · PDV jitter: {} · methodology {}",
                    reading(speed.http_idle_rtt_ms, "ms"),
                    reading(speed.jitter_p95_minus_p50_ms, "ms"),
                    speed.methodology_version
                ));
                lines.push(format!(
                    "Download {} · upload {} · {}",
                    speed.download.qualification, speed.upload.qualification, SPEED_NOTICE
                ));
            }
        }
        lines
    }
}

#[derive(Default)]
pub struct Controller {
    pub state: State,
    cancel: Arc<AtomicBool>,
    complete: Arc<Mutex<Option<Result<ResultData, Failure>>>>,
    worker: Option<std::thread::JoinHandle<()>>,
    started: Option<Instant>,
}
impl Controller {
    pub fn start(&mut self, action: Action, mlab_consent: bool) -> bool {
        self.start_with(action, move |cancel| run(action, mlab_consent, cancel))
    }
    fn start_with<F>(&mut self, action: Action, run: F) -> bool
    where
        F: FnOnce(&AtomicBool) -> Result<ResultData, Failure> + Send + 'static,
    {
        self.poll();
        if self.worker.is_some() {
            return false;
        }
        self.cancel.store(false, Ordering::Release);
        self.state.running = true;
        self.state.action = Some(action);
        self.state.failure = None;
        self.state.elapsed_ms = 0;
        self.state.sequence += 1;
        self.started = Some(Instant::now());
        let cancel = self.cancel.clone();
        let complete = self.complete.clone();
        let worker = std::thread::Builder::new()
            .name("sd300-companion".into())
            .spawn(move || {
                let result =
                    std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| run(&cancel)))
                        .unwrap_or(Err(Failure::Internal));
                if let Ok(mut slot) = complete.lock() {
                    *slot = Some(result);
                }
            });
        match worker {
            Ok(worker) => self.worker = Some(worker),
            Err(_) => {
                self.state.running = false;
                self.state.failure = Some(Failure::Internal);
                return false;
            }
        }
        true
    }
    pub fn cancel(&self) {
        self.cancel.store(true, Ordering::Release);
    }
    pub fn poll(&mut self) -> bool {
        if !self.state.running {
            return false;
        }
        let elapsed = self.started.map_or(0, |s| s.elapsed().as_millis() as u64);
        let changed = elapsed / 1000 != self.state.elapsed_ms / 1000;
        self.state.elapsed_ms = elapsed;
        if self.worker.as_ref().is_some_and(|w| w.is_finished()) {
            let _ = self.worker.take().unwrap().join();
            self.state.running = false;
            match self
                .complete
                .lock()
                .ok()
                .and_then(|mut slot| slot.take())
                .unwrap_or(Err(Failure::Internal))
            {
                Ok(result) => {
                    self.state.failure = result.failure;
                    self.state.result = Some(result);
                }
                Err(failure) => self.state.failure = Some(failure),
            }
            self.state.sequence += 1;
            return true;
        }
        if changed {
            self.state.sequence += 1;
        }
        changed
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
    const PARTIAL: &[u8] = include_bytes!("companion-fixtures/nd300-4.0.1-partial.json");
    #[test]
    fn worker_panic_cancel_and_retry_preserve_one_active_request() {
        let mut controller = Controller::default();
        assert!(controller.start_with(Action::Standard, |_| panic!("injected provider panic")));
        let deadline = Instant::now() + Duration::from_secs(2);
        while controller.state.running && Instant::now() < deadline {
            controller.poll();
            std::thread::sleep(Duration::from_millis(1));
        }
        assert!(!controller.state.running);
        assert_eq!(controller.state.failure, Some(Failure::Internal));
        assert!(controller.start_with(Action::Standard, |cancel| {
            while !cancel.load(Ordering::Acquire) {
                std::thread::sleep(Duration::from_millis(1));
            }
            Err(Failure::Cancelled)
        }));
        assert!(!controller.start_with(Action::Deep, |_| Err(Failure::Internal)));
        controller.cancel();
        let started = Instant::now();
        drop(controller);
        assert!(started.elapsed() < Duration::from_secs(1));
    }

    #[test]
    fn valid_warning_failure_exits_are_results_and_timeouts_are_incomplete() {
        for exit in [0, 1, 2, 130] {
            let result =
                parse_results(Action::Deep, VERIFIED_VERSION, PARTIAL, Some(exit), None).unwrap();
            assert!(result.incomplete);
            assert_eq!(result.checks[0].state, CheckState::Ok);
            assert_eq!(result.checks[1].state, CheckState::Incomplete);
            assert_eq!(result.checks[2].state, CheckState::Warning);
        }
    }
    #[test]
    fn redaction_uses_only_validated_fields_never_imported_details() {
        let result =
            parse_results(Action::Standard, VERIFIED_VERSION, PARTIAL, Some(2), None).unwrap();
        let redacted = result.export(false).to_string();
        for secret in [
            "192.0.2.1",
            "private-host",
            "private-adapter",
            "198.51.100.2",
        ] {
            assert!(!redacted.contains(secret));
        }
        assert!(result.export(true).to_string().contains("private-host"));
    }
    #[test]
    fn diagnostics_cannot_trigger_speed_or_repairs_and_mlab_requires_consent() {
        assert_eq!(Action::Standard.arguments(true), ["--json", "--fast"]);
        assert_eq!(Action::Deep.arguments(true), ["--json", "--tech", "--fast"]);
        assert!(!Action::SpeedQuick
            .arguments(false)
            .contains(&"--accept-mlab".into()));
        assert!(Action::SpeedDeep
            .arguments(true)
            .contains(&"--accept-mlab".into()));
        assert_eq!(Action::SpeedQuick.budget_bytes(), Some(5_000_000_000));
    }
    #[test]
    fn invalid_interrupted_missing_and_unsupported_outputs_are_distinct() {
        assert_eq!(
            parse_results(Action::Standard, "5.0.0", PARTIAL, Some(0), None).unwrap_err(),
            Failure::UnsupportedVersion
        );
        assert_eq!(
            parse_results(Action::Standard, VERIFIED_VERSION, b"{}", Some(0), None).unwrap_err(),
            Failure::MalformedOutput
        );
        assert_eq!(
            parse_results(
                Action::Standard,
                VERIFIED_VERSION,
                br#"{"error":"interrupted","interrupted":true}"#,
                Some(130),
                None
            )
            .unwrap_err(),
            Failure::Cancelled
        );
        assert_eq!(
            parse_results(
                Action::Standard,
                VERIFIED_VERSION,
                b"",
                None,
                Some(Failure::Timeout)
            )
            .unwrap_err(),
            Failure::Timeout
        );
        let result = parse_results(
            Action::Standard,
            VERIFIED_VERSION,
            PARTIAL,
            None,
            Some(Failure::Cancelled),
        )
        .unwrap();
        assert!(result.incomplete);
        assert_eq!(result.failure, Some(Failure::Cancelled));
    }
    #[test]
    fn speed_uses_nullable_methodology_measurements_not_legacy_zero_aliases() {
        let bytes = include_bytes!("companion-fixtures/speedqx-4.0.1-partial.json");
        let result =
            parse_results(Action::SpeedQuick, VERIFIED_VERSION, bytes, Some(0), None).unwrap();
        let speed = result.speed.unwrap();
        assert_eq!(speed.download.sustained_mbps, Some(91.5));
        assert_eq!(speed.upload.sustained_mbps, None);
        assert_eq!(speed.http_idle_rtt_ms, Some(21.0));
    }
}
