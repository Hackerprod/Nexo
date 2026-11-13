use std::{
    fs,
    path::{Path, PathBuf},
    process::{Command, Output},
    sync::atomic::{AtomicUsize, Ordering},
};

use nexo::interp::{MockEnvironment, Outcome, ReducedExecutionProfile, run_with_limits};

const EMPTY_ENV: &str = r#"{"schema_version":1,"mock_read_records":[],"mock_write_i64":[],"mock_request":[],"mock_reply":[]}"#;
static NEXT_DIRECTORY: AtomicUsize = AtomicUsize::new(0);

struct CliCase {
    directory: PathBuf,
    source: PathBuf,
}

impl CliCase {
    fn new(name: &str, source: &str) -> Self {
        let unique = NEXT_DIRECTORY.fetch_add(1, Ordering::Relaxed);
        let directory = std::env::temp_dir().join(format!(
            "nexo-i11a-val56-{}-{name}-{unique}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&directory);
        fs::create_dir_all(&directory).unwrap();
        let source_path = directory.join("program.n");
        fs::write(&source_path, source).unwrap();
        Self {
            directory,
            source: source_path,
        }
    }

    fn environment(&self, bytes: &[u8]) -> PathBuf {
        let path = self.directory.join("environment.json");
        fs::write(&path, bytes).unwrap();
        path
    }

    fn invoke(&self, mock: &Path, json: bool) -> Output {
        let mut command = Command::new(env!("CARGO_BIN_EXE_nexo"));
        command
            .arg("run")
            .arg(&self.source)
            .arg("--mock-env")
            .arg(mock);
        if json {
            command.args(["--format", "json"]);
        }
        command.output().unwrap()
    }
}

impl Drop for CliCase {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.directory);
    }
}

fn reason(case: &CliCase, bytes: &[u8]) -> Option<String> {
    let path = case.environment(bytes);
    let output = case.invoke(&path, true);
    let stdout = String::from_utf8(output.stdout).unwrap();
    if output.status.code() == Some(2) {
        let marker = "\"reason\":\"";
        let start = stdout
            .find(marker)
            .unwrap_or_else(|| panic!("driver error is not JSON: {stdout:?}"))
            + marker.len();
        Some(stdout[start..].split('"').next().unwrap().to_owned())
    } else {
        assert_eq!(output.status.code(), Some(0), "{stdout}");
        None
    }
}

fn environment_with(slot: &str, body: &str) -> String {
    let fields = [
        ("mock_read_records", "[]"),
        ("mock_write_i64", "[]"),
        ("mock_request", "[]"),
        ("mock_reply", "[]"),
    ]
    .into_iter()
    .map(|(key, empty)| format!("\"{key}\":{}", if key == slot { body } else { empty }))
    .collect::<Vec<_>>()
    .join(",");
    format!("{{\"schema_version\":1,{fields}}}")
}

#[test]
fn one_trailing_byte_after_a_complete_document_is_invalid_json() {
    let case = CliCase::new("trailing-byte", "fn Main(){}");
    for suffix in ["}", "]", ",", "x", "0", "\"", "1", ":"] {
        let input = format!("{EMPTY_ENV}{suffix}");
        assert_eq!(
            reason(&case, input.as_bytes()).as_deref(),
            Some("invalid_mock_json"),
            "suffix={suffix:?}"
        );
    }
    assert_eq!(reason(&case, format!("{EMPTY_ENV}\n").as_bytes()), None);
}

#[test]
fn truncation_inside_a_json_string_is_invalid_json_not_schema() {
    let case = CliCase::new("truncated-string", "fn Main(){}");
    for input in [
        r#"{"schema_version":1,"mock_read_records":[],"mock_write_i64":[],"mock_request":[{"ok":"12"#,
        r#"{"schema_version":1,"mock_read_records":[],"mock_write_i64":[],"mock_request":[],"mock_re"#,
        r#"{"schema_version":1,"mock_read_records":[],"mock_write_i64":[],"mock_request":[{"ok":"1\"#,
        r#"{"sch"#,
        r#"""#,
    ] {
        assert_eq!(
            reason(&case, input.as_bytes()).as_deref(),
            Some("invalid_mock_json"),
            "input={input:?}"
        );
    }
}

#[test]
fn result_objects_reject_every_key_other_than_ok_and_err() {
    let case = CliCase::new("result-key", "fn Main(){}");
    for key in ["foo", "OK", "Err", "error", "okay", "o", "", "ok "] {
        for slot in ["mock_write_i64", "mock_request", "mock_reply"] {
            let body = format!(r#"[{{"{key}":"5"}}]"#);
            assert_eq!(
                reason(&case, environment_with(slot, &body).as_bytes()).as_deref(),
                Some("invalid_mock_schema"),
                "slot={slot} key={key:?}"
            );
        }
        let records = format!(r#"[{{"ok":[{{"{key}":"5"}}]}}]"#);
        assert_eq!(
            reason(
                &case,
                environment_with("mock_read_records", &records).as_bytes()
            )
            .as_deref(),
            Some("invalid_mock_schema"),
            "record key={key:?}"
        );
    }
}

#[test]
fn only_json_whitespace_bytes_separate_tokens() {
    let case = CliCase::new("json-whitespace", "fn Main(){}");
    for separator in [
        "\u{0c}", "\u{0b}", "\u{a0}", "\u{2028}", "\u{feff}", "\u{1f}",
    ] {
        let between_members = EMPTY_ENV.replacen(
            "{\"schema_version\":1",
            &format!("{{{separator}\"schema_version\":1"),
            1,
        );
        assert_eq!(
            reason(&case, between_members.as_bytes()).as_deref(),
            Some("invalid_mock_json"),
            "separator={separator:?} between members"
        );
        let inside_array = EMPTY_ENV.replacen("[]", &format!("[{separator}]"), 1);
        assert_eq!(
            reason(&case, inside_array.as_bytes()).as_deref(),
            Some("invalid_mock_json"),
            "separator={separator:?} inside array"
        );
    }
}

#[test]
fn raw_del_in_json_strings_is_valid_json_and_reaches_schema_validation() {
    let case = CliCase::new("raw-del", "fn Main(){}");
    let key = EMPTY_ENV.replacen(
        "{\"schema_version\"",
        "{\"a\u{7f}b\":1,\"schema_version\"",
        1,
    );
    assert_eq!(
        reason(&case, key.as_bytes()).as_deref(),
        Some("invalid_mock_schema")
    );
    let value = environment_with("mock_request", "[{\"ok\":\"5\u{7f}\"}]");
    assert_eq!(
        reason(&case, value.as_bytes()).as_deref(),
        Some("invalid_mock_schema")
    );
}

#[test]
fn trailing_commas_in_objects_and_arrays_are_invalid_json() {
    let case = CliCase::new("trailing-comma", "fn Main(){}");
    for input in [
        environment_with("mock_write_i64", r#"[{"ok":null},]"#),
        environment_with("mock_write_i64", r#"[{"ok":null,}]"#),
        environment_with("mock_read_records", r#"[{"ok":[{"ok":"1"},]}]"#),
        environment_with("mock_request", "[,]"),
        format!("{},}}", &EMPTY_ENV[..EMPTY_ENV.len() - 1]),
        format!("{} ,\n}}", &EMPTY_ENV[..EMPTY_ENV.len() - 1]),
    ] {
        assert_eq!(
            reason(&case, input.as_bytes()).as_deref(),
            Some("invalid_mock_json"),
            "input={input}"
        );
    }
}

#[test]
fn read_err_and_mixed_record_reservations_cover_cells_64_boundaries() {
    let source = "fn Main()->Result<(),I64>{let rows=MockReadRecords(MockOpen(1))?;return Ok(());}";
    let program = nexo::ir::compile_scalar_with_profile(
        "read-record-boundary.n",
        source,
        nexo::ir::ReducedProfile::Cells64,
        true,
    )
    .unwrap();

    for count in 28..=34 {
        for mixed in [false, true] {
            let records = (0..count)
                .map(|index| {
                    if mixed && index % 2 == 0 {
                        r#"{"ok":"1"}"#
                    } else {
                        r#"{"err":"1"}"#
                    }
                })
                .collect::<Vec<_>>()
                .join(",");
            let json = format!(
                r#"{{"schema_version":1,"mock_read_records":[{{"ok":[{records}]}}],"mock_write_i64":[],"mock_request":[],"mock_reply":[]}}"#
            );
            let execution = run_with_limits(
                &program,
                MockEnvironment::parse(json.as_bytes()).unwrap(),
                ReducedExecutionProfile::Cells64,
            );
            let label = format!("count={count} mixed={mixed}");
            if count >= 31 {
                assert_eq!(execution.outcome, Outcome::SimulatorFault, "{label}");
                assert_eq!(execution.diagnostics.len(), 1, "{label}");
                let diagnostic = &execution.diagnostics[0];
                assert_eq!(diagnostic.code, "R0002", "{label}");
                assert_eq!(
                    diagnostic.data["runtime_budget_kind"],
                    nexo::diagnostic::DataValue::String("live_value_cells".into()),
                    "{label}"
                );
                assert_eq!(
                    diagnostic.data["limit"],
                    nexo::diagnostic::DataValue::UInt(64),
                    "{label}"
                );
                assert_eq!(
                    diagnostic.data["observed"],
                    nexo::diagnostic::DataValue::UInt((3 + 2 * count) as u64),
                    "{label}"
                );
                let diagnostic_json = diagnostic
                    .to_json_view(
                        "read-record-boundary.n",
                        nexo::diagnostic::SourceView {
                            bytes_len: source.len(),
                            text: Some(source),
                        },
                    )
                    .unwrap();
                let expected_diagnostic_json = format!(
                    r#"{{"schema_version":1,"diagnostic_namespace":"F0-0002-sim","profile_id":"f0-0002-b-test-cells-64-v2","code":"R0002","subcause":null,"rule":"R0002","explanation_id":null,"severity":"error","stage":"runtime","message":"Fallo del simulador.","operation":"runtime.limit","primary_span":{{"file":"read-record-boundary.n","start":35,"end":63}},"related_spans":[],"cause_chain":[],"feature_id":null,"unexpected_token":null,"expected_tokens":[],"expected_tokens_kind":null,"parser_context":null,"recognized_prefix":null,"repair":[],"data":{{"limit":64,"observed":{},"runtime_budget_kind":"live_value_cells"}}}}"#,
                    3 + 2 * count
                );
                assert_eq!(diagnostic_json, expected_diagnostic_json, "{label}");
                assert_eq!(
                    execution.observations.events(),
                    ["open:0:1", "close:0:1"],
                    "{label}"
                );
                assert_eq!(execution.observations.bytes(), 47, "{label}");
                assert_eq!(execution.observations.reserved_bytes(), 0, "{label}");
            } else {
                assert_eq!(execution.outcome, Outcome::Ok, "{label}");
                assert!(execution.diagnostics.is_empty(), "{label}");
                assert_eq!(
                    execution.observations.events(),
                    [
                        "open:0:1".to_owned(),
                        format!("read_records:0:ok:{count}"),
                        "close:0:1".to_owned()
                    ],
                    "{label}"
                );
                let expected_bytes = [
                    ("mock-resource:", "open:0:1".to_owned()),
                    ("mock-io:", format!("read_records:0:ok:{count}")),
                    ("mock-resource:", "close:0:1".to_owned()),
                ]
                .into_iter()
                .map(|(prefix, text)| (prefix.len() + text.len() + 1) as u64)
                .sum::<u64>();
                assert_eq!(execution.observations.bytes(), expected_bytes, "{label}");
                assert_eq!(execution.observations.reserved_bytes(), 0, "{label}");
            }
        }
    }
}

const UNREADABLE_JSON: &str = concat!(
    "{\"schema_version\":1,\"profile_id\":\"f0-0002-b-v2\",",
    "\"outcome\":{\"kind\":\"driver_error\",\"reason\":\"mock_env_unreadable\"},",
    "\"stdout\":[],\"events\":[],\"observations\":[],",
    "\"observation_bytes\":0,\"reserved_observation_bytes\":0,\"diagnostics\":[]}\n"
);

#[test]
fn unreadable_environment_is_exact_json_and_text_for_missing_and_directory() {
    let case = CliCase::new("unreadable-formats", "fn Main(){}\n");
    let directory = case.directory.join("environment-directory");
    fs::create_dir(&directory).unwrap();
    for path in [case.directory.join("missing.json"), directory] {
        let json = case.invoke(&path, true);
        assert_eq!(json.status.code(), Some(2), "{path:?}");
        assert_eq!(
            String::from_utf8(json.stdout).unwrap(),
            UNREADABLE_JSON,
            "{path:?}"
        );
        assert!(json.stderr.is_empty(), "{path:?}");

        let text = case.invoke(&path, false);
        assert_eq!(text.status.code(), Some(2), "{path:?}");
        assert_eq!(text.stdout, b"", "{path:?}");
        assert_eq!(
            text.stderr, b"nexo-driver:mock_env_unreadable\n",
            "{path:?}"
        );
    }
}

#[cfg(windows)]
#[test]
fn read_failure_after_open_is_reported_as_unreadable() {
    let case = CliCase::new("locked-read", "fn Main(){}\n");
    let environment = case.environment(EMPTY_ENV.as_bytes());
    let held = fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open(&environment)
        .unwrap();
    held.lock().unwrap();

    let output = case.invoke(&environment, true);
    drop(held);
    assert_eq!(output.status.code(), Some(2));
    assert_eq!(String::from_utf8(output.stdout).unwrap(), UNREADABLE_JSON);
    assert!(output.stderr.is_empty());
}

#[test]
fn json_format_reports_invalid_json_reason_for_syntax_frontiers() {
    let case = CliCase::new("json-reason", "fn Main(){}\n");
    for input in [
        format!("{EMPTY_ENV}x"),
        environment_with("mock_request", "[,]"),
        r#"{"schema_version":1,"mock_read_records":[],"mock_write_i64":[],"mock_request":[{"ok":"unterminated"#
            .to_owned(),
    ] {
        let output = case.invoke(&case.environment(input.as_bytes()), true);
        assert_eq!(output.status.code(), Some(2), "{input:?}");
        let stdout = String::from_utf8(output.stdout).unwrap();
        assert!(stdout.contains("\"reason\":\"invalid_mock_json\""), "{stdout}");
        assert!(output.stderr.is_empty(), "{input:?}");
    }
}
