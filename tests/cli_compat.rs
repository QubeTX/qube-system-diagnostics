use std::collections::BTreeSet;
use std::process::{Command, Output};
use std::sync::{Mutex, OnceLock};

use clap::Parser;
use sd_300::cli::{ActionArgs, Cli, Command as CliCommand, ReportArgs, UpdateActionArgs};
use serde_json::Value;

const V2_LONG_HELP: &str = include_str!("fixtures/v2.0.6/help.stdout");
const V2_SHORT_HELP: &str = include_str!("fixtures/v2.0.6/short-help.stdout");
const V2_VERSION: &str = include_str!("fixtures/v2.0.6/version.stdout");
const V2_REPORT_CONTRACT: &str = include_str!("fixtures/v2.0.6/report-contract.json");
const V2_CAPABILITY_IDS: &str = include_str!("fixtures/v2.0.6/capability-ids.json");

fn sd300() -> Command {
    Command::new(env!("CARGO_BIN_EXE_sd300"))
}

fn run(args: &[&str]) -> Output {
    sd300()
        .args(args)
        .output()
        .unwrap_or_else(|error| panic!("sd300 {args:?} should run: {error}"))
}

fn normalize_cli_text(bytes: &[u8]) -> String {
    let text = String::from_utf8(bytes.to_vec())
        .expect("CLI output should be UTF-8")
        .replace("\r\n", "\n")
        .replace("sd300.exe", "sd300");
    let had_trailing_newline = text.ends_with('\n');
    let mut normalized = text
        .lines()
        .map(str::trim_end)
        .collect::<Vec<_>>()
        .join("\n");
    if had_trailing_newline {
        normalized.push('\n');
    }
    normalized
}

fn strip_additive_gui_help(text: &str) -> String {
    let mut normalized = text
        .lines()
        .filter(|line| {
            let trimmed = line.trim_start();
            !trimmed.starts_with("gui ") && !trimmed.starts_with("sd300 gui ")
        })
        .collect::<Vec<_>>()
        .join("\n");
    normalized.push('\n');
    normalized
}

fn parse_single_json(stdout: &[u8]) -> Value {
    let mut values = serde_json::Deserializer::from_slice(stdout).into_iter::<Value>();
    let value = values
        .next()
        .expect("stdout should contain one JSON value")
        .expect("stdout should contain valid JSON");
    assert!(
        values.next().is_none(),
        "stdout must contain exactly one JSON value"
    );
    value
}

fn sorted_keys(value: &Value) -> Vec<String> {
    let mut keys = value
        .as_object()
        .expect("contract path should resolve to an object")
        .keys()
        .cloned()
        .collect::<Vec<_>>();
    keys.sort();
    keys
}

fn expected_keys(value: &Value) -> Vec<String> {
    value
        .as_array()
        .expect("fixture key list should be an array")
        .iter()
        .map(|key| {
            key.as_str()
                .expect("fixture keys should be strings")
                .to_owned()
        })
        .collect()
}

fn assert_observation(value: &Value) {
    let keys = sorted_keys(value).into_iter().collect::<BTreeSet<_>>();
    let required = ["source".to_owned(), "status".to_owned()]
        .into_iter()
        .collect::<BTreeSet<_>>();
    let allowed = [
        "detail".to_owned(),
        "id".to_owned(),
        "source".to_owned(),
        "status".to_owned(),
    ]
    .into_iter()
    .collect::<BTreeSet<_>>();
    assert!(
        required.is_subset(&keys),
        "observation missing required keys"
    );
    assert!(keys.is_subset(&allowed), "observation added unknown keys");
}

fn collector_command_lock() -> std::sync::MutexGuard<'static, ()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
        .lock()
        .expect("collector command lock should not be poisoned")
}

#[test]
fn long_help_is_the_v2_0_6_golden_plus_only_the_additive_gui_lines() {
    let output = run(&["--help"]);
    assert_eq!(output.status.code(), Some(0));
    assert!(output.stderr.is_empty());

    let help = normalize_cli_text(&output.stdout);
    assert!(
        help.contains("  gui "),
        "the additive GUI command is missing"
    );
    assert_eq!(strip_additive_gui_help(&help), V2_LONG_HELP);
}

#[test]
fn short_help_is_the_v2_0_6_golden_plus_only_the_additive_gui_command() {
    let output = run(&["-h"]);
    assert_eq!(output.status.code(), Some(0));
    assert!(output.stderr.is_empty());

    let help = normalize_cli_text(&output.stdout);
    assert!(
        help.contains("  gui "),
        "the additive GUI command is missing"
    );
    assert_eq!(strip_additive_gui_help(&help), V2_SHORT_HELP);
}

#[test]
fn version_keeps_the_v2_binary_identity_and_one_line_shape() {
    for flag in ["--version", "-V"] {
        let output = run(&[flag]);
        assert_eq!(output.status.code(), Some(0));
        assert!(output.stderr.is_empty());

        let version = normalize_cli_text(&output.stdout);
        assert_eq!(version, format!("sd300 {}\n", env!("CARGO_PKG_VERSION")));
        assert_eq!(
            version.replace(env!("CARGO_PKG_VERSION"), "2.0.6"),
            V2_VERSION
        );
    }
}

#[test]
fn deterministic_parse_errors_match_v2_0_6_stdout_stderr_and_exit_codes() {
    let cases = [
        (
            &["definitely-not-command"][..],
            include_str!("fixtures/v2.0.6/errors/unknown-subcommand.stderr"),
        ),
        (
            &["--user", "--tech"][..],
            include_str!("fixtures/v2.0.6/errors/mode-conflict.stderr"),
        ),
        (
            &["snapshot", "--include-sensitive"][..],
            include_str!("fixtures/v2.0.6/errors/sensitive-requires-json.stderr"),
        ),
        (
            &["update", "--tech"][..],
            include_str!("fixtures/v2.0.6/errors/subcommand-mode-conflict.stderr"),
        ),
    ];

    for (args, expected_stderr) in cases {
        let output = run(args);
        assert_eq!(
            output.status.code(),
            Some(2),
            "unexpected exit for {args:?}"
        );
        assert!(
            output.stdout.is_empty(),
            "stdout was not empty for {args:?}"
        );
        assert_eq!(
            normalize_cli_text(&output.stderr),
            expected_stderr,
            "stderr drifted for {args:?}"
        );
    }
}

#[test]
fn v2_command_and_legacy_flag_parser_contract_is_unchanged() {
    let bare = Cli::try_parse_from(["sd300"]).expect("bare TUI launch should parse");
    assert!(!bare.user && !bare.tech && !bare.update && bare.command.is_none());

    let user = Cli::try_parse_from(["sd300", "--user"]).expect("--user should parse");
    assert!(user.user && !user.tech && user.command.is_none());

    let tech = Cli::try_parse_from(["sd300", "--tech"]).expect("--tech should parse");
    assert!(tech.tech && !tech.user && tech.command.is_none());

    let legacy = Cli::try_parse_from(["sd300", "--update"]).expect("--update should parse");
    assert!(legacy.update && legacy.command.is_none());

    let update =
        Cli::try_parse_from(["sd300", "update", "--json"]).expect("update --json should parse");
    assert_eq!(
        update.command,
        Some(CliCommand::Update(UpdateActionArgs {
            json: true,
            relaunch_gui: false,
        }))
    );

    let install =
        Cli::try_parse_from(["sd300", "install", "--json"]).expect("install --json should parse");
    assert_eq!(
        install.command,
        Some(CliCommand::Install(ActionArgs { json: true }))
    );

    let uninstall = Cli::try_parse_from(["sd300", "uninstall", "--json"])
        .expect("uninstall --json should parse");
    assert_eq!(
        uninstall.command,
        Some(CliCommand::Uninstall(ActionArgs { json: true }))
    );

    let snapshot =
        Cli::try_parse_from(["sd300", "snapshot", "--json"]).expect("snapshot --json should parse");
    assert_eq!(
        snapshot.command,
        Some(CliCommand::Snapshot(ReportArgs {
            json: true,
            include_sensitive: false,
        }))
    );

    let capabilities = Cli::try_parse_from(["sd300", "capabilities", "--json"])
        .expect("capabilities --json should parse");
    assert_eq!(
        capabilities.command,
        Some(CliCommand::Capabilities(ReportArgs {
            json: true,
            include_sensitive: false,
        }))
    );

    let gui = Cli::try_parse_from(["sd300", "gui"]).expect("additive GUI command should parse");
    assert_eq!(gui.command, Some(CliCommand::Gui));
}

#[test]
fn snapshot_json_preserves_the_v2_0_6_schema_and_default_redaction() {
    let _guard = collector_command_lock();
    let output = run(&["snapshot", "--json"]);
    assert_eq!(output.status.code(), Some(0));
    assert!(output.stderr.is_empty());
    let report = parse_single_json(&output.stdout);
    let contract: Value =
        serde_json::from_str(V2_REPORT_CONTRACT).expect("report contract fixture should parse");

    for (pointer, keys) in contract["object_keys"]
        .as_object()
        .expect("object_keys should be an object")
    {
        let actual = report
            .pointer(pointer)
            .unwrap_or_else(|| panic!("snapshot is missing v2 path {pointer}"));
        assert_eq!(
            sorted_keys(actual),
            expected_keys(keys),
            "keys at {pointer}"
        );
    }

    for (pointer, keys) in contract["optional_object_keys"]
        .as_object()
        .expect("optional_object_keys should be an object")
    {
        if let Some(actual) = report.pointer(pointer).filter(|value| !value.is_null()) {
            assert_eq!(
                sorted_keys(actual),
                expected_keys(keys),
                "keys at {pointer}"
            );
        }
    }

    for (pointer, keys) in contract["array_item_keys"]
        .as_object()
        .expect("array_item_keys should be an object")
    {
        let values = report
            .pointer(pointer)
            .unwrap_or_else(|| panic!("snapshot is missing v2 array {pointer}"))
            .as_array()
            .unwrap_or_else(|| panic!("snapshot path {pointer} should be an array"));
        for value in values {
            assert_eq!(
                sorted_keys(value),
                expected_keys(keys),
                "item keys at {pointer}"
            );
        }
    }

    for pointer in contract["observation_pointers"]
        .as_array()
        .expect("observation_pointers should be an array")
    {
        let pointer = pointer
            .as_str()
            .expect("observation pointer should be text");
        assert_observation(
            report
                .pointer(pointer)
                .unwrap_or_else(|| panic!("snapshot is missing observation {pointer}")),
        );
    }

    assert_eq!(report["schema_version"], 1);
    assert_eq!(report["product"], "SD-300");
    assert_eq!(report["privacy"]["sensitive_values_included"], false);
    assert_eq!(report["system"]["hostname"], "[redacted]");
    assert_eq!(
        report["privacy"]["redacted_fields"],
        contract["redacted_fields"]
    );

    for drive in report["disk_health"]["drives"]
        .as_array()
        .expect("drives should be an array")
    {
        assert!(drive["serial"].is_null() || drive["serial"] == "[redacted]");
    }
    for interface in report["network"]["interfaces"]
        .as_array()
        .expect("interfaces should be an array")
    {
        assert_eq!(interface["mac_address"], "[redacted]");
        assert!(interface["ip_addresses"]
            .as_array()
            .expect("IP addresses should be an array")
            .iter()
            .all(|address| address == "[redacted]"));
    }
}

#[test]
fn capabilities_json_preserves_v2_0_6_order_shape_and_single_value_stdout() {
    let _guard = collector_command_lock();
    let output = run(&["capabilities", "--json"]);
    assert_eq!(output.status.code(), Some(0));
    assert!(output.stderr.is_empty());
    let capabilities = parse_single_json(&output.stdout);
    let capabilities = capabilities
        .as_array()
        .expect("capabilities JSON should be an array");
    let expected_ids: Vec<String> =
        serde_json::from_str(V2_CAPABILITY_IDS).expect("capability ID fixture should parse");
    let actual_ids = capabilities
        .iter()
        .map(|capability| {
            assert_observation(capability);
            capability["id"]
                .as_str()
                .expect("capability ID should be text")
                .to_owned()
        })
        .collect::<Vec<_>>();
    assert_eq!(actual_ids, expected_ids);
}
