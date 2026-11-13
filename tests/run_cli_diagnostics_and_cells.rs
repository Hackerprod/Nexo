use std::{
    fs,
    path::PathBuf,
    process::{Command, Output},
    sync::atomic::{AtomicUsize, Ordering},
};

use nexo::interp::{MockEnvironment, Outcome, ReducedExecutionProfile, run_with_limits};

const EMPTY_ENV: &str = r#"{"schema_version":1,"mock_read_records":[],"mock_write_i64":[],"mock_request":[],"mock_reply":[]}"#;
const CELLS64: &str = "f0-0002-b-test-cells-64-v2";
static NEXT: AtomicUsize = AtomicUsize::new(0);

struct TestDirectory(PathBuf);

impl TestDirectory {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "nexo-i11a-val57-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        let _ = fs::remove_dir_all(&path);
        fs::create_dir_all(&path).unwrap();
        Self(path)
    }

    fn file(&self, name: &str, contents: &str) -> String {
        let path = self.0.join(name);
        fs::write(&path, contents).unwrap();
        path.to_str().unwrap().to_owned()
    }
}

impl Drop for TestDirectory {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn nexo(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_nexo"))
        .args(args)
        .output()
        .unwrap()
}

fn driver_reason(output: &Output) -> Option<String> {
    let stdout = String::from_utf8_lossy(&output.stdout);
    let marker = "\"reason\":\"";
    let start = stdout.find(marker)? + marker.len();
    Some(stdout[start..].split('"').next()?.to_owned())
}

fn environment_with_version(version: &str) -> String {
    EMPTY_ENV.replace(
        "\"schema_version\":1",
        &format!("\"schema_version\":{version}"),
    )
}

#[test]
fn malformed_and_noncanonical_json_numbers_keep_their_rejection_phase() {
    let directory = TestDirectory::new();
    let source = directory.file("program.n", "fn Main(){}");

    for version in [
        "1.", "1e", "1E", "1e+", "1e-", "1.e3", "-", "01", ".5", "+1",
    ] {
        let environment = directory.file("environment.json", &environment_with_version(version));
        let output = nexo(&[
            "run",
            &source,
            "--mock-env",
            &environment,
            "--format",
            "json",
        ]);
        assert_eq!(output.status.code(), Some(2), "{version}");
        assert_eq!(
            driver_reason(&output).as_deref(),
            Some("invalid_mock_json"),
            "{version}"
        );
    }

    for version in [
        "1.0", "1e0", "1E0", "1e+0", "1E+0", "1e-0", "-0", "10", "1.5", "0",
    ] {
        let environment = directory.file("environment.json", &environment_with_version(version));
        let output = nexo(&[
            "run",
            &source,
            "--mock-env",
            &environment,
            "--format",
            "json",
        ]);
        assert_eq!(output.status.code(), Some(2), "{version}");
        assert_eq!(
            driver_reason(&output).as_deref(),
            Some("invalid_mock_schema"),
            "{version}"
        );
    }
}

fn cells_program(array_items: usize) -> String {
    let values = vec!["0"; array_items].join(",");
    format!(
        "fn Main()->Result<(),I64>{{let array=[{values}];let rows=MockReadRecords(MockOpen(1))?;return Ok(());}}"
    )
}

fn empty_read_environment(records: usize) -> String {
    let values = vec![r#"{"ok":"1"}"#; records].join(",");
    format!(
        r#"{{"schema_version":1,"mock_read_records":[{{"ok":[{values}]}}],"mock_write_i64":[],"mock_request":[],"mock_reply":[]}}"#
    )
}

#[test]
fn cells64_faults_exit_one_in_both_formats_and_empty_read_uses_two_cells() {
    let directory = TestDirectory::new();
    for (items, faults) in [(59usize, false), (60, false), (61, true), (62, true)] {
        let source = directory.file("program.n", &cells_program(items));
        let environment = directory.file("environment.json", &empty_read_environment(0));
        let json = nexo(&[
            "run",
            &source,
            "--mock-env",
            &environment,
            "--profile",
            CELLS64,
            "--format",
            "json",
        ]);
        let text = nexo(&[
            "run",
            &source,
            "--mock-env",
            &environment,
            "--profile",
            CELLS64,
        ]);
        let json_stdout = String::from_utf8_lossy(&json.stdout);
        let text_stderr = String::from_utf8_lossy(&text.stderr);
        if faults {
            assert_eq!(json.status.code(), Some(1), "items={items} {json_stdout}");
            assert!(
                json_stdout.contains(r#""outcome":{"kind":"simulator_fault"}"#),
                "items={items} {json_stdout}"
            );
            assert!(json_stdout.contains(r#""code":"R0002""#), "{json_stdout}");
            assert!(
                json_stdout.contains(&format!(r#""limit":64,"observed":{}"#, 4 + items)),
                "items={items} {json_stdout}"
            );
            assert_eq!(text.status.code(), Some(1), "items={items} {text_stderr}");
            assert!(text_stderr.contains("R0002"), "{text_stderr}");
        } else {
            assert_eq!(json.status.code(), Some(0), "items={items} {json_stdout}");
            assert!(json_stdout.contains(r#""outcome":{"kind":"ok"}"#));
            assert_eq!(text.status.code(), Some(0), "items={items} {text_stderr}");
        }
    }
}

#[test]
fn repeated_options_and_option_names_used_as_values_are_usage_errors() {
    let directory = TestDirectory::new();
    let source = directory.file("program.n", "fn Main(){}");
    let environment = directory.file("environment.json", EMPTY_ENV);
    let cases = [
        vec!["run", &source, "--format", "json", "--format", "json"],
        vec!["run", &source, "--format", "json", "--format", "text"],
        vec![
            "run",
            &source,
            "--profile",
            "f0-0002-v2",
            "--profile",
            "f0-0002-v2",
        ],
        vec!["check", &source, "--format", "text", "--format", "text"],
        vec!["run", &source, "--format", "--mock-env", &environment],
        vec!["run", &source, "--mock-env", "--format", "json"],
        vec!["run", &source, "--mock-env", "--format"],
        vec!["run", &source, "--profile", "--format", "json"],
        vec!["run", &source, "--profile", "--format"],
    ];
    for args in cases {
        let output = nexo(&args);
        assert_eq!(output.status.code(), Some(2), "{args:?}");
        assert_eq!(output.stdout, b"", "{args:?}");
        assert_eq!(
            output.stderr, b"nexo-driver:invalid_arguments\n",
            "{args:?}"
        );
    }
}

fn assert_invalid_arguments(args: &[&str]) {
    let output = nexo(args);
    assert_eq!(output.status.code(), Some(2), "{args:?}");
    assert_eq!(output.stdout, b"", "{args:?}");
    assert_eq!(
        output.stderr, b"nexo-driver:invalid_arguments\n",
        "{args:?}"
    );
}

#[test]
fn run_option_surface_matrix_has_exact_usage_errors_and_accepts_supported_siblings() {
    let directory = TestDirectory::new();
    let source = directory.file("program.n", "fn Main(){}");
    let environment = directory.file("environment.json", EMPTY_ENV);

    for format in ["text", "json"] {
        let mut json_mode = vec!["--format", "json"];
        if format == "text" {
            json_mode.clear();
        }

        let mut run = vec!["run", source.as_str()];
        run.extend_from_slice(&json_mode);
        let run_output = nexo(&run);
        assert_eq!(run_output.status.code(), Some(0), "{format}");
        if format == "json" {
            assert_eq!(
                run_output.stdout,
                b"{\"schema_version\":1,\"profile_id\":\"f0-0002-b-v2\",\"outcome\":{\"kind\":\"unit\"},\"stdout\":[],\"events\":[],\"observations\":[],\"observation_bytes\":0,\"reserved_observation_bytes\":0,\"diagnostics\":[]}\n"
            );
            assert_eq!(run_output.stderr, b"");
        } else {
            assert_eq!(run_output.stdout, b"");
            assert_eq!(run_output.stderr, b"nexo-result:()\n");
        }

        let mut profiled_run = vec!["run", source.as_str(), "--profile", "f0-0002-b-v2"];
        profiled_run.extend_from_slice(&json_mode);
        let profile_output = nexo(&profiled_run);
        assert_eq!(profile_output.status.code(), Some(0), "{format}");

        let mut mock_run = vec!["run", source.as_str(), "--mock-env", environment.as_str()];
        mock_run.extend_from_slice(&json_mode);
        let mock_output = nexo(&mock_run);
        assert_eq!(mock_output.status.code(), Some(0), "{format}");

        for option in ["--format", "--profile", "--mock-env"] {
            let invoke_case = |suffix: &[&str]| {
                let mut args = vec!["run", source.as_str()];
                args.extend_from_slice(&json_mode);
                args.extend_from_slice(suffix);
                assert_invalid_arguments(&args);
            };
            match option {
                "--format" => {
                    let value = format;
                    let other = if format == "text" { "json" } else { "text" };
                    invoke_case(&[option, value, option, value]);
                    invoke_case(&[option, value, option, other]);
                    invoke_case(&[option]);
                    invoke_case(&[option, "--profile", "f0-0002-v2"]);
                    invoke_case(&["--unknown"]);
                    invoke_case(&[option, "xml"]);
                    assert_invalid_arguments(&["build", source.as_str(), option, value]);
                }
                "--profile" => {
                    invoke_case(&[option, "f0-0002-b-v2", option, "f0-0002-b-v2"]);
                    invoke_case(&[option, "f0-0002-b-v2", option, "f0-0002-b-test-cells-64-v2"]);
                    invoke_case(&[option]);
                    invoke_case(&[option, "--format", "json"]);
                    invoke_case(&["--unknown"]);
                    assert_invalid_arguments(&["build", source.as_str(), option, "f0-0002-b-v2"]);
                }
                "--mock-env" => {
                    invoke_case(&[option, environment.as_str(), option, environment.as_str()]);
                    invoke_case(&[option, environment.as_str(), option, "missing.json"]);
                    invoke_case(&[option]);
                    invoke_case(&[option, "--format", "json"]);
                    invoke_case(&["--unknown"]);
                    let mut check_args = vec!["check", source.as_str()];
                    check_args.extend_from_slice(&json_mode);
                    check_args.extend_from_slice(&[option, environment.as_str()]);
                    assert_invalid_arguments(&check_args);
                }
                _ => unreachable!(),
            }
        }

        let mut check = Command::new(env!("CARGO_BIN_EXE_nexo"));
        check
            .arg("check")
            .arg(&source)
            .arg("--profile")
            .arg("f0-0002-b-v2");
        if format == "json" {
            check.args(["--format", "json"]);
        } else {
            check.args(["--format", "text"]);
        }
        let output = check.output().unwrap();
        assert_eq!(output.status.code(), Some(0));
        if format == "json" {
            assert_eq!(output.stderr, b"");
            assert_eq!(
                output.stdout,
                b"{\"schema_version\":1,\"profile_id\":\"f0-0002-b-v2\",\"outcome\":{\"kind\":\"checked\"},\"stdout\":[],\"events\":[],\"observations\":[],\"observation_bytes\":0,\"reserved_observation_bytes\":0,\"diagnostics\":[]}\n"
            );
        } else {
            assert_eq!(output.stdout, b"nexo-check:ok\n");
            assert_eq!(output.stderr, b"");
        }
    }
}

#[test]
fn result_diagnostic_and_fault_classes_have_exact_outputs_in_both_formats() {
    let directory = TestDirectory::new();

    for (source, outcome_json, text_result, status) in [
        (
            "fn Main()->Result<(),I64>{return Ok(());}",
            r#"{"kind":"ok"}"#,
            "nexo-result:Ok(())\n",
            0,
        ),
        (
            "fn Main()->Result<(),I64>{return Err(-7);}",
            r#"{"kind":"err","error":"-7"}"#,
            "nexo-result:Err(-7)\n",
            1,
        ),
    ] {
        let path = directory.file("result.n", source);
        let json = nexo(&["run", &path, "--format", "json"]);
        assert_eq!(json.status.code(), Some(status), "{source}");
        assert_eq!(json.stderr, b"", "{source}");
        assert_eq!(
            String::from_utf8(json.stdout).unwrap(),
            format!(
                r#"{{"schema_version":1,"profile_id":"f0-0002-b-v2","outcome":{outcome_json},"stdout":[],"events":[],"observations":[],"observation_bytes":0,"reserved_observation_bytes":0,"diagnostics":[]}}"#
            ) + "\n",
            "{source}"
        );
        let text = nexo(&["run", &path]);
        assert_eq!(text.status.code(), Some(status), "{source}");
        assert_eq!(text.stdout, b"", "{source}");
        assert_eq!(text.stderr, text_result.as_bytes(), "{source}");
    }

    let overflow_source = "fn Main(){Print(9223372036854775807+1);}";
    let overflow_path = directory.file("sum.n", overflow_source);
    let overflow_start = overflow_source.find("9223372036854775807+1").unwrap();
    let overflow_diagnostic = runtime_diagnostic_json(RuntimeDiagnostic {
        profile: "f0-0002-b-v2",
        code: "R0101",
        operation: "value.add",
        message: "Desbordamiento de suma i64.",
        file: &overflow_path,
        start: overflow_start,
        end: overflow_start + "9223372036854775807+1".len(),
        related: "[]",
        data: "{}",
    });
    let overflow_json = nexo(&["run", &overflow_path, "--format", "json"]);
    assert_eq!(overflow_json.status.code(), Some(1));
    assert_eq!(
        String::from_utf8(overflow_json.stdout).unwrap(),
        execution_json(
            "f0-0002-b-v2",
            r#"{"kind":"simulator_fault"}"#,
            &[],
            0,
            std::slice::from_ref(&overflow_diagnostic)
        ) + "\n"
    );
    assert_eq!(overflow_json.stderr, b"");
    let overflow_text = nexo(&["run", &overflow_path]);
    assert_eq!(overflow_text.status.code(), Some(1));
    assert_eq!(overflow_text.stdout, b"");
    assert_eq!(
        String::from_utf8(overflow_text.stderr).unwrap(),
        format!(
            "R0101:1:{}: Desbordamiento de suma i64.\n  namespace: F0-0002-sim\n  profile: f0-0002-b-v2\n  stage: runtime\n  operation: value.add\n  at: {{\"end\":{},\"file\":{},\"start\":{}}}\n",
            overflow_start + 1,
            overflow_start + "9223372036854775807+1".len(),
            json_string(&overflow_path),
            overflow_start,
        )
    );

    let mut work_source = String::from("fn Main(){var i=0;while i<71427{i=i+1;");
    work_source.push('}');
    work_source.push_str(&"{}".repeat(9));
    work_source.push('}');
    let work_path = directory.file("work.n", &work_source);
    let work_start = work_source.len() - 1;
    let work_diagnostic = runtime_diagnostic_json(RuntimeDiagnostic {
        profile: "f0-0002-b-v2",
        code: "R0001",
        operation: "runtime.limit",
        message: "Fallo del simulador.",
        file: &work_path,
        start: work_start,
        end: work_source.len(),
        related: &format!(
            r#"[{{"role":"context","span":{{"file":{},"start":9,"end":{}}}}}]"#,
            json_string(&work_path),
            work_source.len()
        ),
        data: r#"{"limit":1000000,"observed":1000001,"work_consumed":1000000}"#,
    });
    let work_json = nexo(&["run", &work_path, "--format", "json"]);
    assert_eq!(work_json.status.code(), Some(1));
    assert_eq!(
        String::from_utf8(work_json.stdout).unwrap(),
        execution_json(
            "f0-0002-b-v2",
            r#"{"kind":"simulator_fault"}"#,
            &[],
            0,
            &[work_diagnostic]
        ) + "\n"
    );
    let work_text = nexo(&["run", &work_path]);
    assert_eq!(work_text.status.code(), Some(1));
    assert_eq!(work_text.stdout, b"");
    assert_eq!(
        String::from_utf8(work_text.stderr).unwrap(),
        format!(
            "R0001:1:{}: Fallo del simulador.\n  namespace: F0-0002-sim\n  profile: f0-0002-b-v2\n  stage: runtime\n  operation: runtime.limit\n  at: {{\"end\":{},\"file\":{},\"start\":{}}}\n  related: {{\"role\":\"context\",\"span\":{{\"end\":{},\"file\":{},\"start\":9}}}}\n  data: {{\"limit\":1000000,\"observed\":1000001,\"work_consumed\":1000000}}\n",
            work_start + 1,
            work_source.len(),
            json_string(&work_path),
            work_start,
            work_source.len(),
            json_string(&work_path)
        )
    );

    let cell_source = cells_program(61);
    let cell_path = directory.file("cells.n", &cell_source);
    let environment = directory.file("environment.json", &empty_read_environment(0));
    let call_start = cell_source.find("MockReadRecords").unwrap();
    let cell_diagnostic = runtime_diagnostic_json(RuntimeDiagnostic {
        profile: CELLS64,
        code: "R0002",
        operation: "runtime.limit",
        message: "Fallo del simulador.",
        file: &cell_path,
        start: call_start,
        end: call_start + "MockReadRecords(MockOpen(1))".len(),
        related: "[]",
        data: r#"{"limit":64,"observed":65,"runtime_budget_kind":"live_value_cells"}"#,
    });
    let cell_events = vec!["open:0:1".to_owned(), "close:0:1".to_owned()];
    let cell_json = nexo(&[
        "run",
        &cell_path,
        "--mock-env",
        &environment,
        "--profile",
        CELLS64,
        "--format",
        "json",
    ]);
    assert_eq!(cell_json.status.code(), Some(1));
    assert_eq!(
        String::from_utf8(cell_json.stdout).unwrap(),
        execution_json(
            CELLS64,
            r#"{"kind":"simulator_fault"}"#,
            &cell_events,
            47,
            &[cell_diagnostic]
        ) + "\n"
    );
    let cell_text = nexo(&[
        "run",
        &cell_path,
        "--mock-env",
        &environment,
        "--profile",
        CELLS64,
    ]);
    assert_eq!(cell_text.status.code(), Some(1));
    assert_eq!(cell_text.stdout, b"");
    assert_eq!(
        String::from_utf8(cell_text.stderr).unwrap(),
        format!(
            "mock-resource:open:0:1\nmock-resource:close:0:1\nR0002:1:{}: Fallo del simulador.\n  namespace: F0-0002-sim\n  profile: {CELLS64}\n  stage: runtime\n  operation: runtime.limit\n  at: {{\"end\":{},\"file\":{},\"start\":{}}}\n  data: {{\"limit\":64,\"observed\":65,\"runtime_budget_kind\":\"live_value_cells\"}}\n",
            call_start + 1,
            call_start + "MockReadRecords(MockOpen(1))".len(),
            json_string(&cell_path),
            call_start
        )
    );
}

#[test]
fn mock_environment_and_compilation_diagnostics_match_exact_contract_outputs() {
    let directory = TestDirectory::new();
    let valid_source = directory.file("program.n", "fn Main(){}");
    let bad_json = directory.file("invalid-json.json", "{");
    let bad_schema = directory.file("invalid-schema.json", "{}");
    let unreadable = directory.0.join("missing-environment.json");
    let large_path = directory.0.join("large-environment.json");
    let mut oversized = EMPTY_ENV.to_owned();
    let padding = " ".repeat(1_048_577 - oversized.len());
    oversized.push_str(&padding);
    fs::write(&large_path, oversized).unwrap();

    for (path, reason) in [
        (bad_json, "invalid_mock_json"),
        (bad_schema, "invalid_mock_schema"),
        (
            unreadable.to_string_lossy().into_owned(),
            "mock_env_unreadable",
        ),
        (
            large_path.to_string_lossy().into_owned(),
            "mock_env_too_large",
        ),
    ] {
        for format in ["text", "json"] {
            let mut args = vec!["run", valid_source.as_str(), "--mock-env", path.as_str()];
            if format == "json" {
                args.extend(["--format", "json"]);
            }
            let output = nexo(&args);
            assert_eq!(output.status.code(), Some(2), "{reason} {format}");
            if format == "json" {
                let expected = execution_json(
                    "f0-0002-b-v2",
                    &format!(
                        r#"{{"kind":"driver_error","reason":{}}}"#,
                        json_string(reason)
                    ),
                    &[],
                    0,
                    &[],
                ) + "\n";
                assert_eq!(
                    String::from_utf8(output.stdout).unwrap(),
                    expected,
                    "{reason}"
                );
                assert_eq!(output.stderr, b"", "{reason}");
            } else {
                assert_eq!(output.stdout, b"", "{reason}");
                assert_eq!(
                    output.stderr,
                    format!("nexo-driver:{reason}\n").as_bytes(),
                    "{reason}"
                );
            }
        }
    }

    // A2 §13.6 and §13.8, lines 2048–2078: the diagnostic fields and spans
    // come from the decimal-literal example. A2 l. 2160 permits translated
    // message text, so the comparison deliberately masks only that field.
    let decimal_source = "fn Main(){let x=0.5;}";
    let decimal_path = directory.file("decimal.nx", decimal_source);
    let decimal_file = json_string(&decimal_path);
    let decimal_diagnostic = format!(
        r#"{{"schema_version":1,"diagnostic_namespace":"Nexo-proposed-v1","profile_id":"f0-0002-b-v2","code":"NX-E0006","subcause":null,"rule":"NX-E0006","explanation_id":"NX-E0006","severity":"error","stage":"parse","message":"<message>","operation":"syntax.parse","primary_span":{{"file":{decimal_file},"start":16,"end":19}},"related_spans":[],"cause_chain":[],"feature_id":"float_literal","unexpected_token":null,"expected_tokens":[],"expected_tokens_kind":null,"parser_context":null,"recognized_prefix":null,"repair":[],"data":{{"pending_scope":"later_adr"}}}}"#
    );
    let json = nexo(&["run", &decimal_path, "--format", "json"]);
    assert_eq!(json.status.code(), Some(1));
    assert_eq!(json.stderr, b"");
    let expected_json = format!(
        r#"{{"schema_version":1,"profile_id":"f0-0002-b-v2","outcome":{{"kind":"rejected"}},"stdout":[],"events":[],"observations":[],"observation_bytes":0,"reserved_observation_bytes":0,"diagnostics":[{decimal_diagnostic}]}}"#
    ) + "\n";
    assert_eq!(
        without_json_message(&String::from_utf8(json.stdout).unwrap()),
        without_json_message(&expected_json)
    );
    let text = nexo(&["run", &decimal_path]);
    assert_eq!(text.status.code(), Some(1));
    assert_eq!(text.stdout, b"");
    assert_eq!(
        without_text_message(&String::from_utf8(text.stderr).unwrap()),
        format!(
            "NX-E0006:1:17: <message>\n  namespace: Nexo-proposed-v1\n  profile: f0-0002-b-v2\n  stage: parse\n  operation: syntax.parse\n  at: {{\"end\":19,\"file\":{decimal_file},\"start\":16}}\n  feature_id: float_literal\n  data: {{\"pending_scope\":\"later_adr\"}}\n"
        )
    );
}

fn cells_read_source(array_items: usize) -> String {
    let items = vec!["0"; array_items].join(",");
    format!(
        "fn Main()->Result<(),I64>{{let array:Array<I64>=[{items}];let rows=MockReadRecords(MockOpen(1))?;return Ok(());}}"
    )
}

fn cells_read_environment(count: usize, pattern: &str) -> String {
    let records = (0..count)
        .map(|index| match pattern {
            "ok" => r#"{"ok":"1"}"#.to_owned(),
            "err" => r#"{"err":"-1"}"#.to_owned(),
            "mixed" if index % 2 == 0 => r#"{"ok":"1"}"#.to_owned(),
            "mixed" => r#"{"err":"-1"}"#.to_owned(),
            _ => unreachable!(),
        })
        .collect::<Vec<_>>()
        .join(",");
    format!(
        r#"{{"schema_version":1,"mock_read_records":[{{"ok":[{records}]}}],"mock_write_i64":[],"mock_request":[],"mock_reply":[]}}"#
    )
}

fn read_error_environment() -> &'static [u8] {
    br#"{"schema_version":1,"mock_read_records":[{"err":"7"}],"mock_write_i64":[],"mock_request":[],"mock_reply":[]}"#
}

fn event_bytes<I, S>(events: I) -> u64
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    events
        .into_iter()
        .map(|event| {
            let event = event.as_ref();
            let prefix = if event.starts_with("open:") || event.starts_with("close:") {
                "mock-resource:"
            } else {
                "mock-io:"
            };
            (prefix.len() + event.len() + 1) as u64
        })
        .sum()
}

fn json_string(value: &str) -> String {
    format!("\"{}\"", value.replace('\\', "\\\\").replace('"', "\\\""))
}

fn without_json_message(text: &str) -> String {
    let marker = "\"message\":";
    let start = text.find(marker).unwrap() + marker.len();
    let end = text[start..].find(",\"operation\":").unwrap() + start;
    format!(
        "{}\"message\":\"<message>\"{}",
        &text[..start - marker.len()],
        &text[end..]
    )
}

fn without_text_message(text: &str) -> String {
    let line_end = text.find('\n').unwrap();
    let message_start = text.find(": ").unwrap() + 2;
    format!("{}<message>{}", &text[..message_start], &text[line_end..])
}

struct RuntimeDiagnostic<'a> {
    profile: &'a str,
    code: &'a str,
    operation: &'a str,
    message: &'a str,
    file: &'a str,
    start: usize,
    end: usize,
    related: &'a str,
    data: &'a str,
}

fn runtime_diagnostic_json(diagnostic: RuntimeDiagnostic<'_>) -> String {
    format!(
        r#"{{"schema_version":1,"diagnostic_namespace":"F0-0002-sim","profile_id":{},"code":"{}","subcause":null,"rule":"{}","explanation_id":null,"severity":"error","stage":"runtime","message":{},"operation":{},"primary_span":{{"file":{},"start":{},"end":{}}},"related_spans":{},"cause_chain":[],"feature_id":null,"unexpected_token":null,"expected_tokens":[],"expected_tokens_kind":null,"parser_context":null,"recognized_prefix":null,"repair":[],"data":{}}}"#,
        json_string(diagnostic.profile),
        diagnostic.code,
        diagnostic.code,
        json_string(diagnostic.message),
        json_string(diagnostic.operation),
        json_string(diagnostic.file),
        diagnostic.start,
        diagnostic.end,
        diagnostic.related,
        diagnostic.data
    )
}

fn execution_json(
    profile: &str,
    outcome: &str,
    events: &[String],
    bytes: u64,
    diagnostics: &[String],
) -> String {
    let events_json = events
        .iter()
        .map(|event| json_string(event))
        .collect::<Vec<_>>()
        .join(",");
    let observations_json = events
        .iter()
        .map(|event| format!(r#"{{"channel":"event","text":{}}}"#, json_string(event)))
        .collect::<Vec<_>>()
        .join(",");
    format!(
        r#"{{"schema_version":1,"profile_id":{},"outcome":{outcome},"stdout":[],"events":[{events_json}],"observations":[{observations_json}],"observation_bytes":{bytes},"reserved_observation_bytes":0,"diagnostics":[{}]}}"#,
        json_string(profile),
        diagnostics.join(",")
    )
}

#[test]
fn cells64_read_reservations_cover_zero_one_two_and_large_ok_err_record_sets() {
    for count in [0usize, 1, 2, 30] {
        for pattern in ["ok", "err", "mixed"] {
            let environment = cells_read_environment(count, pattern);
            let parsed = MockEnvironment::parse(environment.as_bytes()).unwrap();
            let boundary_array_items = 60 - 2 * count;
            for (extra_array_item, should_fault) in [(0usize, false), (1, true)] {
                let array_items = boundary_array_items + extra_array_item;
                let source = cells_read_source(array_items);
                let program = nexo::ir::compile_scalar_with_profile(
                    "cells-read-matrix.n",
                    &source,
                    nexo::ir::ReducedProfile::Cells64,
                    true,
                )
                .unwrap();
                let execution =
                    run_with_limits(&program, parsed.clone(), ReducedExecutionProfile::Cells64);
                let label = format!("count={count} pattern={pattern} array={array_items}");
                if should_fault {
                    assert_eq!(execution.outcome, Outcome::SimulatorFault, "{label}");
                    assert_eq!(execution.diagnostics.len(), 1, "{label}");
                    let diagnostic = &execution.diagnostics[0];
                    assert_eq!(diagnostic.code, "R0002", "{label}");
                    assert_eq!(diagnostic.subcause, None, "{label}");
                    assert_eq!(
                        diagnostic.data,
                        [
                            ("limit".to_owned(), nexo::diagnostic::DataValue::UInt(64)),
                            ("observed".to_owned(), nexo::diagnostic::DataValue::UInt(65)),
                            (
                                "runtime_budget_kind".to_owned(),
                                nexo::diagnostic::DataValue::String("live_value_cells".into())
                            ),
                        ]
                        .into(),
                        "{label}"
                    );
                    let start = source.find("MockReadRecords").unwrap();
                    let diagnostic_json = diagnostic
                        .to_json_view(
                            "cells-read-matrix.n",
                            nexo::diagnostic::SourceView {
                                bytes_len: source.len(),
                                text: Some(&source),
                            },
                        )
                        .unwrap();
                    let expected = format!(
                        r#"{{"schema_version":1,"diagnostic_namespace":"F0-0002-sim","profile_id":"{CELLS64}","code":"R0002","subcause":null,"rule":"R0002","explanation_id":null,"severity":"error","stage":"runtime","message":"Fallo del simulador.","operation":"runtime.limit","primary_span":{{"file":"cells-read-matrix.n","start":{start},"end":{}}},"related_spans":[],"cause_chain":[],"feature_id":null,"unexpected_token":null,"expected_tokens":[],"expected_tokens_kind":null,"parser_context":null,"recognized_prefix":null,"repair":[],"data":{{"limit":64,"observed":65,"runtime_budget_kind":"live_value_cells"}}}}"#,
                        start + "MockReadRecords(MockOpen(1))".len()
                    );
                    assert_eq!(diagnostic_json, expected, "{label}");
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
                    let expected_events = vec![
                        "open:0:1".to_owned(),
                        format!("read_records:0:ok:{count}"),
                        "close:0:1".to_owned(),
                    ];
                    assert_eq!(execution.observations.events(), expected_events, "{label}");
                    assert_eq!(
                        execution.observations.bytes(),
                        event_bytes(expected_events.iter().map(String::as_str)),
                        "{label}"
                    );
                    assert_eq!(execution.observations.reserved_bytes(), 0, "{label}");
                }
            }
        }
    }

    for (array_items, should_fault) in [(60usize, false), (61, true)] {
        let source = cells_read_source(array_items);
        let program = nexo::ir::compile_scalar_with_profile(
            "cells-read-matrix.n",
            &source,
            nexo::ir::ReducedProfile::Cells64,
            true,
        )
        .unwrap();
        let execution = run_with_limits(
            &program,
            MockEnvironment::parse(read_error_environment()).unwrap(),
            ReducedExecutionProfile::Cells64,
        );
        if should_fault {
            assert_eq!(execution.outcome, Outcome::SimulatorFault);
            assert_eq!(execution.diagnostics[0].code, "R0002");
            assert_eq!(
                execution.diagnostics[0].data["observed"],
                nexo::diagnostic::DataValue::UInt(65)
            );
            let expected_events = ["open:0:1", "close:0:1"];
            assert_eq!(execution.observations.events(), expected_events);
            assert_eq!(execution.observations.bytes(), 47);
        } else {
            assert_eq!(execution.outcome, Outcome::Err(7));
            assert!(execution.diagnostics.is_empty());
            let expected_events = ["open:0:1", "read_records:0:err:7", "close:0:1"];
            assert_eq!(execution.observations.events(), expected_events);
            assert_eq!(execution.observations.bytes(), event_bytes(expected_events));
        }
        assert_eq!(execution.observations.reserved_bytes(), 0);
    }
}

#[test]
fn cells64_write_and_reply_literal_arguments_keep_the_current_frontier() {
    // A2 §11.6 ll. 1063–1064 fixes the returned two-cell Result, but not
    // whether each evaluated literal argument remains live during reservation.
    for operation in ["write", "reply"] {
        for (items, expected_fault) in [(61usize, false), (62, true)] {
            let values = vec!["0"; items].join(",");
            let source = if operation == "write" {
                format!("fn Main(){{let array:Array<I64>=[{values}];MockWriteI64(25);}}")
            } else {
                format!("fn Main(){{let array:Array<I64>=[{values}];MockReply(200);}}")
            };
            let program = nexo::ir::compile_scalar_with_profile(
                "literal-liveness.n",
                &source,
                nexo::ir::ReducedProfile::Cells64,
                true,
            )
            .unwrap();
            let environment = if operation == "write" {
                br#"{"schema_version":1,"mock_read_records":[],"mock_write_i64":[{"ok":null}],"mock_request":[],"mock_reply":[]}"#.as_slice()
            } else {
                br#"{"schema_version":1,"mock_read_records":[],"mock_write_i64":[],"mock_request":[],"mock_reply":[{"ok":null}]}"#
            };
            let execution = run_with_limits(
                &program,
                MockEnvironment::parse(environment).unwrap(),
                ReducedExecutionProfile::Cells64,
            );
            if expected_fault {
                assert_eq!(execution.outcome, Outcome::SimulatorFault, "{operation}");
                assert_eq!(execution.diagnostics.len(), 1, "{operation}");
                assert_eq!(execution.diagnostics[0].code, "R0002", "{operation}");
                assert_eq!(
                    execution.diagnostics[0].data["observed"],
                    nexo::diagnostic::DataValue::UInt(65),
                    "{operation}"
                );
                assert!(execution.observations.events().is_empty(), "{operation}");
                assert!(execution.observations.stdout().is_empty(), "{operation}");
                assert_eq!(execution.observations.bytes(), 0, "{operation}");
            } else {
                assert_eq!(execution.outcome, Outcome::Unit, "{operation}");
                assert!(execution.diagnostics.is_empty(), "{operation}");
                assert_eq!(
                    execution.observations.events(),
                    [if operation == "write" {
                        "write_i64:25:ok"
                    } else {
                        "reply:200:ok"
                    }],
                    "{operation}"
                );
                assert_eq!(
                    execution.observations.stdout(),
                    if operation == "write" {
                        vec!["25"]
                    } else {
                        vec![]
                    },
                    "{operation}"
                );
                assert_eq!(
                    execution.observations.bytes(),
                    if operation == "write" { 27 } else { 21 },
                    "{operation}"
                );
            }
            assert_eq!(execution.observations.reserved_bytes(), 0, "{operation}");
        }
    }
}
