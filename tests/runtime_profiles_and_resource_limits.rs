use std::{
    fs,
    path::{Path, PathBuf},
    process::{Command, Output},
    sync::atomic::{AtomicUsize, Ordering},
};

const FRAMES8: &str = "f0-0002-b-test-frames-8-v2";
const CELLS64: &str = "f0-0002-b-test-cells-64-v2";
const OBSERVATIONS64: &str = "f0-0002-b-test-observations-64-v2";
const WORK_LIMIT: u64 = 1_000_000;
static NEXT: AtomicUsize = AtomicUsize::new(0);

struct TestDirectory(PathBuf);

impl TestDirectory {
    fn new() -> Self {
        let path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("target")
            .join("test-fixtures")
            .join(format!(
                "i11a-profiles-{}-{}",
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

fn json_string(text: &str) -> String {
    let mut out = String::from("\"");
    for character in text.chars() {
        match character {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if c < ' ' => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

fn diagnostic_has_profile(output: &str, profile: &str) -> bool {
    output
        .find(r#""diagnostics":["#)
        .is_some_and(|start| output[start..].contains(&format!(r#""profile_id":"{profile}""#)))
}

fn assert_runtime_diagnostic_envelope(output: &str, profile: &str, code: &str) {
    for field in [
        r#""diagnostic_namespace":"F0-0002-sim""#,
        r#""subcause":null,"#,
        r#""severity":"error""#,
        r#""stage":"runtime""#,
        r#""operation":"runtime.limit""#,
        r#""cause_chain":[],"#,
    ] {
        assert!(
            output.contains(field),
            "{profile} {code} missing {field}: {output}"
        );
    }
    assert!(
        output.contains(&format!(r#""code":"{code}""#)),
        "{profile} {output}"
    );
    assert!(
        diagnostic_has_profile(output, profile),
        "{profile} {output}"
    );
}

fn run_json(directory: &TestDirectory, source: &str, profile: &str) -> Output {
    let path = directory.file("program.n", source);
    nexo(&["run", &path, "--profile", profile, "--format", "json"])
}

fn source_runtime_fault_json(
    profile: &str,
    events: &[&str],
    bytes: u64,
    reserved: u64,
    diagnostic: &str,
) -> String {
    let events_json = events
        .iter()
        .map(|event| json_string(event))
        .collect::<Vec<_>>()
        .join(",");
    let observations = events
        .iter()
        .map(|event| format!(r#"{{"channel":"event","text":{}}}"#, json_string(event)))
        .collect::<Vec<_>>()
        .join(",");
    format!(
        r#"{{"schema_version":1,"profile_id":{},"outcome":{{"kind":"simulator_fault"}},"stdout":[],"events":[{}],"observations":[{}],"observation_bytes":{},"reserved_observation_bytes":{},"diagnostics":[{}]}}"#,
        json_string(profile),
        events_json,
        observations,
        bytes,
        reserved,
        diagnostic
    ) + "\n"
}

fn runtime_work_diagnostic(profile: &str, file: &str, start: usize, end: usize) -> String {
    format!(
        r#"{{"schema_version":1,"diagnostic_namespace":"F0-0002-sim","profile_id":{},"code":"R0001","subcause":null,"rule":"R0001","explanation_id":null,"severity":"error","stage":"runtime","message":"Fallo del simulador.","operation":"runtime.limit","primary_span":{{"file":{},"start":{},"end":{}}},"related_spans":[],"cause_chain":[],"feature_id":null,"unexpected_token":null,"expected_tokens":[],"expected_tokens_kind":null,"parser_context":null,"recognized_prefix":null,"repair":[],"data":{{"limit":1000000,"observed":1000001,"work_consumed":1000000}}}}"#,
        json_string(profile),
        json_string(file),
        start,
        end
    )
}

fn runtime_work_diagnostic_text(profile: &str, file: &str, start: usize, end: usize) -> String {
    format!(
        "R0001:1:{}: Fallo del simulador.\n  namespace: F0-0002-sim\n  profile: {profile}\n  stage: runtime\n  operation: runtime.limit\n  at: {{\"end\":{end},\"file\":{},\"start\":{start}}}\n  data: {{\"limit\":1000000,\"observed\":1000001,\"work_consumed\":1000000}}\n",
        start + 1,
        json_string(file)
    )
}

#[test]
fn reduced_frames_profile_accepts_seven_and_eight_and_rejects_nine_frames() {
    let directory = TestDirectory::new();
    for (functions, rejected) in [(6usize, false), (7, false), (8, true)] {
        let mut source = String::new();
        for index in 0..functions {
            if index + 1 < functions {
                source.push_str(&format!("fn F{index}(){{F{}();}}", index + 1));
            } else {
                source.push_str(&format!("fn F{index}(){{}}"));
            }
        }
        source.push_str("fn Main(){F0();}");
        let output = run_json(&directory, &source, FRAMES8);
        let stdout = String::from_utf8(output.stdout).unwrap();
        if rejected {
            assert_eq!(output.status.code(), Some(1), "{stdout}");
            assert!(stdout.contains(r#""outcome":{"kind":"simulator_fault"}"#));
            assert!(stdout.contains(r#""code":"R0002""#), "{stdout}");
            assert!(
                stdout.contains(
                    r#""data":{"limit":8,"observed":9,"runtime_budget_kind":"call_frames"}"#
                ),
                "{stdout}"
            );
            assert!(diagnostic_has_profile(&stdout, FRAMES8), "{stdout}");
        } else {
            assert_eq!(
                output.status.code(),
                Some(0),
                "functions={functions} {stdout}"
            );
            assert!(stdout.contains(r#""outcome":{"kind":"unit"}"#));
        }
    }
}

#[test]
fn reduced_observation_profile_checks_all_sides_of_the_limit_with_and_without_a_resource() {
    let directory = TestDirectory::new();

    for (body, emitted, rejected) in [
        (
            format!("{}Print(10);", "Print(0);".repeat(30)),
            63u64,
            false,
        ),
        ("Print(0);".repeat(32), 64, false),
        (format!("{}Print(10);", "Print(0);".repeat(31)), 65, true),
    ] {
        let source = format!("fn Main(){{{body}}}");
        let output = run_json(&directory, &source, OBSERVATIONS64);
        let stdout = String::from_utf8(output.stdout).unwrap();
        if rejected {
            assert_eq!(output.status.code(), Some(1), "{stdout}");
            assert!(stdout.contains(r#""code":"R0002""#), "{stdout}");
            assert!(
                stdout.contains(
                    r#""data":{"limit":64,"observed":65,"runtime_budget_kind":"observation_bytes"}"#
                ),
                "{stdout}"
            );
            assert!(diagnostic_has_profile(&stdout, OBSERVATIONS64), "{stdout}");
        } else {
            assert_eq!(output.status.code(), Some(0), "{stdout}");
            assert!(stdout.contains(&format!(r#""observation_bytes":{emitted},"#)));
        }
    }

    for (body, committed, rejected) in [
        ("Print(0);".repeat(8), 63u64, false),
        (format!("{}Print(10);", "Print(0);".repeat(7)), 64, false),
        ("Print(0);".repeat(9), 65, true),
    ] {
        let source = format!("fn Main(){{let resource=MockOpen(7);{body}}}");
        let output = run_json(&directory, &source, OBSERVATIONS64);
        let stdout = String::from_utf8(output.stdout).unwrap();
        if rejected {
            assert_eq!(output.status.code(), Some(1), "{stdout}");
            assert!(stdout.contains(r#""code":"R0002""#), "{stdout}");
            assert!(stdout.contains(r#""events":["open:0:7","close:0:7"]"#));
            assert!(stdout.contains(r#""observation_bytes":63,"reserved_observation_bytes":0,"#));
            assert!(stdout.contains(r#""observed":65,"runtime_budget_kind":"observation_bytes""#));
        } else {
            assert_eq!(output.status.code(), Some(0), "{stdout}");
            assert!(stdout.contains(&format!(r#""observation_bytes":{committed},"#)));
            assert!(stdout.contains(r#""reserved_observation_bytes":0,"diagnostics":[]"#));
        }
    }
}

#[test]
fn selected_profile_is_preserved_in_runtime_and_static_diagnostics() {
    let directory = TestDirectory::new();
    let runtime_source = "fn Main(){var i=0;while i<71428{i=i+1;}}";
    let runtime_profiles = [
        "f0-0002-b-v2",
        "f0-0002-b-test-types-32-v2",
        "f0-0002-b-test-flow-32-v2",
        "f0-0002-b-test-flow-global-32-v2",
        "f0-0002-b-test-effects-32-v2",
        FRAMES8,
        CELLS64,
        OBSERVATIONS64,
    ];
    for profile in runtime_profiles {
        let output = run_json(&directory, runtime_source, profile);
        let stdout = String::from_utf8(output.stdout).unwrap();
        assert_eq!(output.status.code(), Some(1), "{profile} {stdout}");
        assert!(stdout.contains(r#""code":"R0001""#), "{profile} {stdout}");
        assert!(
            diagnostic_has_profile(&stdout, profile),
            "{profile} {stdout}"
        );
    }

    let rejected = directory.file("type-error.n", "fn Main(){MockWriteI64(true);}");
    for profile in [
        "f0-0002-b-v2",
        "f0-0002-b-test-types-32-v2",
        "f0-0002-b-test-flow-32-v2",
        "f0-0002-b-test-flow-global-32-v2",
        "f0-0002-b-test-effects-32-v2",
        "f0-0002-b-test-ir-32-v2",
        FRAMES8,
        CELLS64,
        OBSERVATIONS64,
    ] {
        let output = nexo(&["run", &rejected, "--profile", profile, "--format", "json"]);
        let stdout = String::from_utf8(output.stdout).unwrap();
        assert_eq!(output.status.code(), Some(1), "{profile} {stdout}");
        assert!(
            stdout.contains(r#""code":"NX-E0102""#),
            "{profile} {stdout}"
        );
        assert!(
            diagnostic_has_profile(&stdout, profile),
            "{profile} {stdout}"
        );
    }
}

#[test]
fn profile_catalog_matches_official_b_cli_identifiers() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let catalog = fs::read_to_string(root.join("docs/adr/F0-CODIGOS.md")).unwrap();
    for profile in [
        "f0-0002-b-v2",
        "f0-0002-b-test-types-32-v2",
        "f0-0002-b-test-flow-32-v2",
        "f0-0002-b-test-flow-global-32-v2",
        "f0-0002-b-test-effects-32-v2",
        "f0-0002-b-test-ir-32-v2",
        FRAMES8,
        CELLS64,
        OBSERVATIONS64,
    ] {
        assert!(
            catalog.contains(&format!("`{profile}`")),
            "catalog misses {profile}"
        );
    }
    let d15 = fs::read_to_string(root.join("docs/adr/F0-D15-sintaxis-oficial.md")).unwrap();
    assert!(
        d15.contains("solo se admiten los identificadores B publicados en §6.1"),
        "D15 CLI profile rule must refer to its B identifiers"
    );
}

#[test]
fn profile_selection_matrix_covers_defaults_all_profiles_and_unknown_ids() {
    let directory = TestDirectory::new();
    let source = directory.file("empty.n", "fn Main(){}");
    let profiles = [
        "f0-0002-b-v2",
        "f0-0002-b-test-types-32-v2",
        "f0-0002-b-test-flow-32-v2",
        "f0-0002-b-test-flow-global-32-v2",
        "f0-0002-b-test-effects-32-v2",
        "f0-0002-b-test-ir-32-v2",
        FRAMES8,
        CELLS64,
        OBSERVATIONS64,
    ];

    for command in ["check", "run"] {
        for format in ["text", "json"] {
            for selected in std::iter::once(None).chain(profiles.iter().copied().map(Some)) {
                let profile = selected.unwrap_or("f0-0002-b-v2");
                let mut args = vec![command, source.as_str()];
                if let Some(profile) = selected {
                    args.extend(["--profile", profile]);
                }
                if format == "json" {
                    args.extend(["--format", "json"]);
                }
                let output = nexo(&args);
                assert_eq!(output.status.code(), Some(0), "{args:?}");
                if format == "json" {
                    assert!(
                        String::from_utf8_lossy(&output.stdout)
                            .contains(&format!(r#""profile_id":"{profile}""#)),
                        "{args:?}"
                    );
                    assert_eq!(output.stderr, b"");
                } else if command == "check" {
                    assert_eq!(output.stdout, b"nexo-check:ok\n");
                    assert_eq!(output.stderr, b"");
                } else {
                    assert_eq!(output.stdout, b"");
                    assert_eq!(output.stderr, b"nexo-result:()\n");
                }
            }

            let unknown = nexo(&[command, &source, "--profile", "bogus", "--format", "json"]);
            assert_eq!(unknown.status.code(), Some(2));
            assert_eq!(
                unknown.stdout,
                b"{\"schema_version\":1,\"profile_id\":\"bogus\",\"outcome\":{\"kind\":\"driver_error\",\"reason\":\"unknown_profile\"},\"stdout\":[],\"events\":[],\"observations\":[],\"observation_bytes\":0,\"reserved_observation_bytes\":0,\"diagnostics\":[]}\n"
            );
            assert_eq!(unknown.stderr, b"");

            let unknown_text = nexo(&[command, &source, "--profile", "bogus"]);
            assert_eq!(unknown_text.status.code(), Some(2));
            assert_eq!(unknown_text.stdout, b"");
            assert_eq!(unknown_text.stderr, b"nexo-driver:unknown_profile\n");
        }
    }

    for format in ["text", "json"] {
        for args in [
            vec![
                "run",
                &source,
                "--profile",
                profiles[0],
                "--profile",
                profiles[0],
            ],
            vec![
                "run",
                &source,
                "--profile",
                profiles[0],
                "--profile",
                profiles[1],
            ],
        ] {
            let mut args = args;
            if format == "json" {
                args.extend(["--format", "json"]);
            }
            let output = nexo(&args);
            assert_eq!(output.status.code(), Some(2), "{args:?}");
            assert_eq!(output.stdout, b"");
            assert_eq!(output.stderr, b"nexo-driver:invalid_arguments\n");
        }
    }

    // A2 §13.10 publishes each explain option once; repetitions are invalid.
    let explain_format = nexo(&[
        "explain", "NX-E0006", "--format", "json", "--format", "text",
    ]);
    assert_eq!(explain_format.status.code(), Some(2));
    assert!(
        String::from_utf8_lossy(&explain_format.stdout).contains(r#""reason":"invalid_arguments""#)
    );
    assert_eq!(explain_format.stderr, b"");
    let explain_subcause = nexo(&[
        "explain",
        "NX-E0006",
        "--subcause",
        "first",
        "--subcause",
        "second",
    ]);
    assert_eq!(explain_subcause.status.code(), Some(2));
}

#[test]
fn observations_reservation_uses_complete_proposal_and_preserves_prior_lines() {
    let directory = TestDirectory::new();
    let exact_source = format!(
        "fn Main(){{{}Print(10);let resource=MockOpen(7);}}",
        "Print(0);".repeat(7)
    );
    let exact = run_json(&directory, &exact_source, OBSERVATIONS64);
    let exact_json = String::from_utf8(exact.stdout).unwrap();
    assert_eq!(exact.status.code(), Some(0), "{exact_json}");
    assert!(exact_json.contains(r#""events":["open:0:7","close:0:7"]"#));
    assert!(exact_json.contains(r#""observation_bytes":64,"reserved_observation_bytes":0"#));

    for (count, expected, events, bytes) in [(20usize, 87u64, "[]", 40u64), (21, 89, "[]", 42)] {
        let source = format!(
            "fn Main(){{{}let resource=MockOpen(7);}}",
            "Print(0);".repeat(count)
        );
        let output = run_json(&directory, &source, OBSERVATIONS64);
        let stdout = String::from_utf8(output.stdout).unwrap();
        assert_eq!(output.status.code(), Some(1), "{stdout}");
        assert!(stdout.contains(r#""code":"R0002""#), "{stdout}");
        assert!(
            stdout.contains(&format!(
                r#""data":{{"limit":64,"observed":{expected},"runtime_budget_kind":"observation_bytes"}}"#
            )),
            "{stdout}"
        );
        assert!(
            stdout.contains(&format!(r#""events":{events}"#)),
            "{stdout}"
        );
        assert!(
            stdout.contains(&format!(
                r#""observation_bytes":{bytes},"reserved_observation_bytes":0,"#
            )),
            "{stdout}"
        );
    }
}

#[test]
fn normal_profile_work_frames_cells_and_observations_reach_all_three_boundary_points() {
    // A2 §11.5 ll. 963–971 and §11.7 ll. 1187–1205 define the source families
    // and expected L-1/L/L+1 results independently of the quota implementation.
    let directory = TestDirectory::new();
    let normal = "f0-0002-b-v2";

    for (blocks, rejected) in [(7usize, false), (8, false), (9, true)] {
        let independent_work = 999_992 + blocks as u64;
        assert_eq!(
            independent_work,
            [999_999, 1_000_000, 1_000_001][blocks - 7]
        );
        let source = format!(
            "fn Main(){{var i=0;while i<71427{{i=i+1;}}{}{}",
            "{}".repeat(blocks),
            "}"
        );
        let path = directory.file("normal-work.n", &source);
        let json = nexo(&["run", &path, "--profile", normal, "--format", "json"]);
        let stdout = String::from_utf8(json.stdout).unwrap();
        if rejected {
            assert_eq!(json.status.code(), Some(1), "{stdout}");
            assert!(stdout.contains(r#""code":"R0001""#), "{stdout}");
            assert!(stdout.contains(r#""profile_id":"f0-0002-b-v2""#));
            assert!(
                stdout.contains(r#""limit":1000000,"observed":1000001,"work_consumed":1000000"#)
            );
        } else {
            assert_eq!(json.status.code(), Some(0), "{stdout}");
            assert!(stdout.contains(r#""outcome":{"kind":"unit"}"#));
        }
        let text = nexo(&["run", &path, "--profile", normal]);
        if rejected {
            assert_eq!(text.status.code(), Some(1));
            assert_eq!(text.stdout, b"");
            assert!(
                String::from_utf8(text.stderr)
                    .unwrap()
                    .starts_with("R0001:")
            );
        } else {
            assert_eq!(text.status.code(), Some(0));
            assert_eq!(text.stdout, b"");
            assert_eq!(text.stderr, b"nexo-result:()\n");
        }
    }

    for (depth, rejected) in [(255usize, false), (256, false), (257, true)] {
        let functions = depth - 1;
        let mut source = String::new();
        for index in 0..functions {
            if index + 1 < functions {
                source.push_str(&format!("fn F{index}(){{F{}();}}", index + 1));
            } else {
                source.push_str(&format!("fn F{index}(){{}}"));
            }
        }
        source.push_str("fn Main(){F0();}");
        let path = directory.file("normal-frames.n", &source);
        let json = nexo(&["run", &path, "--profile", normal, "--format", "json"]);
        let stdout = String::from_utf8(json.stdout).unwrap();
        if rejected {
            assert_eq!(json.status.code(), Some(1), "{stdout}");
            assert!(stdout.contains(r#""code":"R0002""#), "{stdout}");
            assert!(stdout.contains(
                r#""data":{"limit":256,"observed":257,"runtime_budget_kind":"call_frames"}"#
            ));
        } else {
            assert_eq!(json.status.code(), Some(0), "depth={depth} {stdout}");
        }
        let text = nexo(&["run", &path, "--profile", normal]);
        assert_eq!(
            text.status.code(),
            Some(if rejected { 1 } else { 0 }),
            "depth={depth}"
        );
    }

    for profile in [(CELLS64, 64usize)] {
        let (profile_id, limit) = profile;
        for (items, rejected) in [(limit - 2, false), (limit - 1, false), (limit, true)] {
            let literals = std::iter::repeat_n("0", items)
                .collect::<Vec<_>>()
                .join(",");
            let source = format!("fn Main(){{let values=[{literals}];Drop(values);}}");
            let path = directory.file("cell-boundary.n", &source);
            let json = nexo(&["run", &path, "--profile", profile_id, "--format", "json"]);
            let stdout = String::from_utf8(json.stdout).unwrap();
            if rejected {
                assert_eq!(
                    json.status.code(),
                    Some(1),
                    "{profile_id} items={items} {stdout}"
                );
                assert!(stdout.contains(r#""code":"R0002""#), "{stdout}");
                assert!(stdout.contains(&format!(
                    r#""data":{{"limit":{limit},"observed":{},"runtime_budget_kind":"live_value_cells"}}"#,
                    limit + 1
                )));
            } else {
                assert_eq!(
                    json.status.code(),
                    Some(0),
                    "{profile_id} items={items} {stdout}"
                );
                assert!(stdout.contains(r#""outcome":{"kind":"unit"}"#));
            }
            let text = nexo(&["run", &path, "--profile", profile_id]);
            assert_eq!(
                text.status.code(),
                Some(if rejected { 1 } else { 0 }),
                "{profile_id} items={items}"
            );
        }
    }

    for (prints, include_ten, bytes, rejected) in [
        (32_766usize, true, 65_535u64, false),
        (32_768, false, 65_536, false),
        (32_767, true, 65_537, true),
    ] {
        let mut source = format!("fn Main(){{var i=0;while i<{prints}{{Print(0);i=i+1;}}");
        if include_ten {
            source.push_str("Print(10);");
        }
        source.push('}');
        let path = directory.file("normal-observations.n", &source);
        let json = nexo(&["run", &path, "--profile", normal, "--format", "json"]);
        let stdout = String::from_utf8(json.stdout).unwrap();
        if rejected {
            assert_eq!(json.status.code(), Some(1), "{stdout}");
            assert!(stdout.contains(r#""code":"R0002""#), "{stdout}");
            assert!(stdout.contains(r#""data":{"limit":65536,"observed":65537,"runtime_budget_kind":"observation_bytes"}"#));
            assert!(
                stdout.contains(r#""observation_bytes":65534,"reserved_observation_bytes":0,"#)
            );
        } else {
            assert_eq!(json.status.code(), Some(0), "{stdout}");
            assert!(stdout.contains(&format!(r#""observation_bytes":{bytes},"#)));
        }
        let text = nexo(&["run", &path, "--profile", normal]);
        assert_eq!(text.status.code(), Some(if rejected { 1 } else { 0 }));
        if rejected {
            assert_eq!(text.stdout.len(), 65_534);
            assert!(
                String::from_utf8(text.stderr)
                    .unwrap()
                    .starts_with("R0002:")
            );
        } else {
            assert_eq!(text.stdout.len() as u64, bytes);
            assert_eq!(text.stderr, b"nexo-result:()\n");
        }
    }
}

fn assert_large_observation_boundaries_for_profile(directory: &TestDirectory, profile: &str) {
    const LIMIT: u64 = 65_536;
    for (record_count, shape, expected_total, rejected) in [
        (9_350usize, "minus_one", LIMIT - 1, false),
        (9_351, "exact", LIMIT, false),
        (9_350, "plus_one", LIMIT + 1, true),
    ] {
        let mut records = Vec::with_capacity(record_count);
        let mut rendered = Vec::with_capacity(record_count);
        for index in 0..record_count {
            let (record, text) = match (shape, index) {
                ("minus_one", 0) => (r#"{"ok":"1000000"}"#, "Ok(1000000)".to_owned()),
                ("plus_one", 0) => (r#"{"ok":"10000000"}"#, "Ok(10000000)".to_owned()),
                ("plus_one", 1) => (r#"{"err":"0"}"#, "Err(0)".to_owned()),
                _ => (r#"{"ok":"1"}"#, "Ok(1)".to_owned()),
            };
            records.push(record);
            rendered.push(text);
        }
        let records = records.join(",");
        let expected_line = format!("[{}]", rendered.join(", "));
        let environment = format!(
            r#"{{"schema_version":1,"mock_read_records":[{{"ok":[{records}]}}],"mock_write_i64":[],"mock_request":[],"mock_reply":[]}}"#
        );
        let env_path = directory.file("large-observation-environment.json", &environment);
        let source = "fn Main()->Result<(),I64>{let resource=MockOpen(1);let rows=MockReadRecords(resource)?;Print(rows);Drop(resource);return Ok(());}";
        let path = directory.file("large-observation.n", source);
        let read_event = format!("read_records:0:ok:{record_count}");
        let event_bytes = 23 + 8 + read_event.len() as u64 + 1 + 24;
        let line_bytes = match shape {
            "minus_one" => (7 * record_count + 7) as u64,
            "exact" => (7 * record_count + 1) as u64,
            "plus_one" => (7 * record_count + 9) as u64,
            _ => unreachable!(),
        };
        assert_eq!(event_bytes + line_bytes, expected_total);

        let json = nexo(&[
            "run",
            &path,
            "--mock-env",
            &env_path,
            "--profile",
            profile,
            "--format",
            "json",
        ]);
        let stdout = String::from_utf8(json.stdout).unwrap();
        if rejected {
            let call = "Print(rows)";
            let start = source.find(call).unwrap();
            assert_eq!(json.status.code(), Some(1), "{profile} {stdout}");
            assert!(stdout.contains(r#""code":"R0002""#), "{profile} {stdout}");
            assert_runtime_diagnostic_envelope(&stdout, profile, "R0002");
            assert!(stdout.contains(&format!(
                r#""primary_span":{{"file":{},"start":{start},"end":{}}}"#,
                json_string(&path),
                start + call.len()
            )));
            assert!(stdout.contains(&format!(
                r#""data":{{"limit":{LIMIT},"observed":{},"runtime_budget_kind":"observation_bytes"}}"#,
                LIMIT + 1
            )));
            assert!(stdout.contains(&format!(
                r#""events":["open:0:1","{read_event}","close:0:1"]"#
            )));
            assert!(stdout.contains(&format!(
                r#""observation_bytes":{event_bytes},"reserved_observation_bytes":0,"#
            )));
            assert!(
                diagnostic_has_profile(&stdout, profile),
                "{profile} {stdout}"
            );
        } else {
            assert_eq!(json.status.code(), Some(0), "{profile} {stdout}");
            assert!(stdout.contains(&format!(r#""observation_bytes":{expected_total},"#)));
            assert!(stdout.contains(&format!(
                r#""stdout":[{}],"events"#,
                json_string(&expected_line)
            )));
            assert!(stdout.contains(&format!(
                r#""events":["open:0:1","{read_event}","close:0:1"]"#
            )));
            assert!(stdout.contains(r#""reserved_observation_bytes":0,"diagnostics":[]"#));
        }

        let text = nexo(&["run", &path, "--mock-env", &env_path, "--profile", profile]);
        assert_eq!(
            text.status.code(),
            Some(if rejected { 1 } else { 0 }),
            "{profile}"
        );
        if rejected {
            assert!(text.stdout.is_empty(), "{profile}");
            let stderr = String::from_utf8(text.stderr).unwrap();
            assert!(
                stderr.contains(&format!("mock-io:{read_event}\n")),
                "{profile}"
            );
            assert!(stderr.contains("mock-resource:open:0:1\n"), "{profile}");
            assert!(stderr.contains("mock-resource:close:0:1\n"), "{profile}");
            assert!(stderr.contains("R0002:"), "{profile}");
            assert!(stderr.contains(&format!("profile: {profile}")), "{profile}");
            assert!(stderr.contains(&format!(
                "\"limit\":{LIMIT},\"observed\":{},\"runtime_budget_kind\":\"observation_bytes\"",
                LIMIT + 1
            )));
        } else {
            assert_eq!(
                text.stdout,
                format!("{expected_line}\n").as_bytes(),
                "{profile}"
            );
            let stderr = String::from_utf8(text.stderr).unwrap();
            assert!(
                stderr.contains(&format!("mock-io:{read_event}\n")),
                "{profile}"
            );
            assert!(stderr.ends_with("nexo-result:Ok(())\n"), "{profile}");
        }
    }
}

#[test]
fn runtime_profile_table_drives_work_frames_cells_and_observation_boundaries() {
    // Independent A2 table: the five analysis-only profiles inherit normal
    // execution limits; only frames-8, cells-64 and observations-64 reduce one.
    let profiles = [
        ("f0-0002-b-v2", 256u64, 65_536u64, 65_536u64),
        ("f0-0002-b-test-types-32-v2", 256, 65_536, 65_536),
        ("f0-0002-b-test-flow-32-v2", 256, 65_536, 65_536),
        ("f0-0002-b-test-flow-global-32-v2", 256, 65_536, 65_536),
        ("f0-0002-b-test-effects-32-v2", 256, 65_536, 65_536),
        (FRAMES8, 8, 65_536, 65_536),
        (CELLS64, 256, 64, 65_536),
        (OBSERVATIONS64, 256, 65_536, 64),
    ];
    let directory = TestDirectory::new();

    for (profile, frames, cells, observations) in profiles {
        for (blocks, work, rejected) in [
            (7usize, 999_999u64, false),
            (8, 1_000_000, false),
            (9, 1_000_001, true),
        ] {
            assert_eq!(999_992 + blocks as u64, work);
            let mut source = String::from("fn Main(){var i=0;while i<71427{i=i+1;}");
            source.push_str(&"{}".repeat(blocks));
            source.push('}');
            let path = directory.file("table-work.n", &source);
            let json = nexo(&["run", &path, "--profile", profile, "--format", "json"]);
            let stdout = String::from_utf8(json.stdout).unwrap();
            if rejected {
                assert_eq!(json.status.code(), Some(1), "{profile} {stdout}");
                assert!(stdout.contains(r#""code":"R0001""#), "{profile} {stdout}");
                assert_runtime_diagnostic_envelope(&stdout, profile, "R0001");
                assert!(
                    diagnostic_has_profile(&stdout, profile),
                    "{profile} {stdout}"
                );
                assert!(
                    stdout
                        .contains(r#""limit":1000000,"observed":1000001,"work_consumed":1000000"#)
                );
                let span = source.len() - 1;
                assert!(stdout.contains(&format!(
                    r#""primary_span":{{"file":{},"start":{span},"end":{}}}"#,
                    json_string(&path),
                    source.len()
                )));
                assert!(stdout.contains(&format!(
                    r#""related_spans":[{{"role":"context","span":{{"file":{},"start":9,"end":{}}}}}]"#,
                    json_string(&path),
                    source.len()
                )), "{stdout}");
            } else {
                assert_eq!(json.status.code(), Some(0), "{profile} {stdout}");
                assert!(stdout.contains(&format!(r#""profile_id":"{profile}""#)));
                assert!(stdout.contains(r#""outcome":{"kind":"unit"}"#));
            }
            let text = nexo(&["run", &path, "--profile", profile]);
            assert_eq!(
                text.status.code(),
                Some(if rejected { 1 } else { 0 }),
                "{profile}"
            );
            if rejected {
                assert_eq!(text.stdout, b"");
                let stderr = String::from_utf8(text.stderr).unwrap();
                assert!(stderr.starts_with("R0001:"), "{profile} {stderr}");
                assert!(stderr.contains(&format!("profile: {profile}")), "{profile}");
                assert!(
                    stderr.contains(
                        "\"limit\":1000000,\"observed\":1000001,\"work_consumed\":1000000"
                    )
                );
            } else {
                assert_eq!(text.stdout, b"");
                assert_eq!(text.stderr, b"nexo-result:()\n");
            }
        }

        for (target_depth, rejected) in [(frames - 1, false), (frames, false), (frames + 1, true)] {
            let (source, call) = if profile == CELLS64 {
                // The reduced cells-64 profile cannot hold 255 recursive scalar
                // parameters; a finite source call chain reaches frames without
                // adding live value cells.
                let functions = target_depth as usize - 1;
                let mut source = String::new();
                for index in 0..functions {
                    if index + 1 < functions {
                        source.push_str(&format!("fn F{index}(){{F{}();}}", index + 1));
                    } else {
                        source.push_str(&format!("fn F{index}(){{}}"));
                    }
                }
                source.push_str("fn Main(){F0();}");
                let rejected_call = format!("F{}()", functions - 1);
                (source, rejected_call)
            } else {
                // Main plus (n + 1) Step entries gives n + 2 frames.
                let recursion = target_depth - 2;
                (
                    format!(
                        "fn Step(n:view I64){{if 0<n{{Step(n-1);}}}}fn Main(){{Step({recursion});}}"
                    ),
                    "Step(n-1)".to_owned(),
                )
            };
            let path = directory.file("table-frames.n", &source);
            let json = nexo(&["run", &path, "--profile", profile, "--format", "json"]);
            let stdout = String::from_utf8(json.stdout).unwrap();
            if rejected {
                let start = source.find(&call).unwrap();
                assert_eq!(json.status.code(), Some(1), "{profile} {stdout}");
                assert!(stdout.contains(r#""code":"R0002""#), "{profile} {stdout}");
                assert_runtime_diagnostic_envelope(&stdout, profile, "R0002");
                assert!(
                    stdout.contains(&format!(
                        r#""primary_span":{{"file":{},"start":{start},"end":{}}}"#,
                        json_string(&path),
                        start + call.len()
                    )),
                    "{profile} {stdout}"
                );
                assert!(stdout.contains(&format!(
                    r#""data":{{"limit":{frames},"observed":{},"runtime_budget_kind":"call_frames"}}"#,
                    frames + 1
                )));
                assert!(
                    diagnostic_has_profile(&stdout, profile),
                    "{profile} {stdout}"
                );
            } else {
                assert_eq!(
                    json.status.code(),
                    Some(0),
                    "{profile} depth={target_depth} {stdout}"
                );
                assert!(stdout.contains(&format!(r#""profile_id":"{profile}""#)));
            }
            let text = nexo(&["run", &path, "--profile", profile]);
            assert_eq!(
                text.status.code(),
                Some(if rejected { 1 } else { 0 }),
                "{profile}"
            );
            if rejected {
                assert_eq!(text.stdout, b"");
                let stderr = String::from_utf8(text.stderr).unwrap();
                assert!(stderr.contains("R0002:"), "{profile} {stderr}");
                assert!(stderr.contains(&format!("profile: {profile}")), "{profile}");
                assert!(stderr.contains(&format!(
                    "\"limit\":{frames},\"observed\":{},\"runtime_budget_kind\":\"call_frames\"",
                    frames + 1
                )));
            } else {
                assert_eq!(text.stdout, b"");
                assert_eq!(text.stderr, b"nexo-result:()\n");
            }
        }

        let base_records = ((cells - 4) / 2) as usize;
        for (record_count, scalar_before_read, rejected) in [
            (base_records, false, false),
            (base_records, true, false),
            (base_records + 1, false, true),
        ] {
            let records = std::iter::repeat_n(r#"{"ok":"1"}"#, record_count)
                .collect::<Vec<_>>()
                .join(",");
            let environment = format!(
                r#"{{"schema_version":1,"mock_read_records":[{{"ok":[{records}]}}],"mock_write_i64":[],"mock_request":[],"mock_reply":[]}}"#
            );
            let env_path = directory.file("table-cells.json", &environment);
            let scalar = if scalar_before_read {
                "let spare=0;"
            } else {
                ""
            };
            let source = format!(
                "fn Main(){{let resource=MockOpen(1);{scalar}let rows=MockReadRecords(resource);Drop(rows);Drop(resource);}}"
            );
            let path = directory.file("table-cells.n", &source);
            let json = nexo(&[
                "run",
                &path,
                "--mock-env",
                &env_path,
                "--profile",
                profile,
                "--format",
                "json",
            ]);
            let stdout = String::from_utf8(json.stdout).unwrap();
            let observation_preempts_cell = profile == OBSERVATIONS64 && !rejected;
            if observation_preempts_cell {
                assert_eq!(json.status.code(), Some(1), "{profile} {stdout}");
                assert!(stdout.contains(r#""code":"R0002""#), "{stdout}");
                assert_runtime_diagnostic_envelope(&stdout, profile, "R0002");
                assert!(stdout.contains(
                    r#""data":{"limit":64,"observed":79,"runtime_budget_kind":"observation_bytes"}"#
                ));
                assert!(stdout.contains(r#""events":["open:0:1","close:0:1"]"#));
                assert!(
                    stdout.contains(r#""observation_bytes":47,"reserved_observation_bytes":0,"#)
                );
                assert!(
                    diagnostic_has_profile(&stdout, profile),
                    "{profile} {stdout}"
                );
            } else if rejected {
                let call = "MockReadRecords(resource)";
                let start = source.find(call).unwrap();
                assert_eq!(json.status.code(), Some(1), "{profile} {stdout}");
                assert!(stdout.contains(r#""code":"R0002""#), "{stdout}");
                assert_runtime_diagnostic_envelope(&stdout, profile, "R0002");
                assert!(stdout.contains(&format!(
                    r#""primary_span":{{"file":{},"start":{start},"end":{}}}"#,
                    json_string(&path),
                    start + call.len()
                )));
                assert!(stdout.contains(&format!(
                    r#""data":{{"limit":{cells},"observed":{},"runtime_budget_kind":"live_value_cells"}}"#,
                    cells + 1
                )));
                assert!(stdout.contains(r#""events":["open:0:1","close:0:1"]"#));
                assert!(
                    stdout.contains(r#""observation_bytes":47,"reserved_observation_bytes":0,"#)
                );
                assert!(
                    diagnostic_has_profile(&stdout, profile),
                    "{profile} {stdout}"
                );
            } else {
                assert_eq!(json.status.code(), Some(0), "{profile} {stdout}");
                assert!(stdout.contains(&format!(r#""profile_id":"{profile}""#)));
                assert!(stdout.contains(r#""outcome":{"kind":"unit"}"#));
                assert!(stdout.contains(r#""reserved_observation_bytes":0,"diagnostics":[]"#));
            }
            let text = nexo(&["run", &path, "--mock-env", &env_path, "--profile", profile]);
            assert_eq!(
                text.status.code(),
                Some(if rejected || observation_preempts_cell {
                    1
                } else {
                    0
                }),
                "{profile}"
            );
            assert_eq!(text.stdout, b"", "{profile}");
            let stderr = String::from_utf8(text.stderr).unwrap();
            if observation_preempts_cell || rejected {
                assert!(stderr.contains("mock-resource:open:0:1\n"), "{profile}");
                assert!(stderr.contains("mock-resource:close:0:1\n"), "{profile}");
                assert!(stderr.contains("R0002:"), "{profile}");
                if observation_preempts_cell {
                    assert!(stderr.contains(
                        "\"limit\":64,\"observed\":79,\"runtime_budget_kind\":\"observation_bytes\""
                    ));
                } else {
                    assert!(stderr.contains(&format!("\"limit\":{cells},\"observed\":{},\"runtime_budget_kind\":\"live_value_cells\"", cells + 1)));
                }
            } else {
                let read_event = format!("mock-io:read_records:0:ok:{record_count}\n");
                assert!(stderr.contains(&read_event), "{profile}");
                assert!(stderr.ends_with("nexo-result:()\n"), "{profile}");
            }
        }

        if profile != CELLS64 && profile != OBSERVATIONS64 {
            assert_large_observation_boundaries_for_profile(&directory, profile);
            continue;
        }

        for (prints, print_ten, expected_bytes, rejected) in [
            (
                ((observations - 4) / 2) as usize,
                true,
                observations - 1,
                false,
            ),
            ((observations / 2) as usize, false, observations, false),
            (
                ((observations - 2) / 2) as usize,
                true,
                observations + 1,
                true,
            ),
        ] {
            let mut source = format!("fn Main(){{var i=0;while i<{prints}{{Print(0);i=i+1;}}");
            if print_ten {
                source.push_str("Print(10);");
            }
            source.push('}');
            let path = directory.file("table-observations.n", &source);
            let json = nexo(&["run", &path, "--profile", profile, "--format", "json"]);
            let stdout = String::from_utf8(json.stdout).unwrap();
            if rejected {
                let call = "Print(10)";
                let start = source.find(call).unwrap();
                assert_eq!(json.status.code(), Some(1), "{profile} {stdout}");
                assert!(stdout.contains(r#""code":"R0002""#), "{profile} {stdout}");
                assert_runtime_diagnostic_envelope(&stdout, profile, "R0002");
                assert!(stdout.contains(&format!(
                    r#""primary_span":{{"file":{},"start":{start},"end":{}}}"#,
                    json_string(&path),
                    start + call.len()
                )));
                assert!(stdout.contains(&format!(
                    r#""data":{{"limit":{observations},"observed":{},"runtime_budget_kind":"observation_bytes"}}"#,
                    observations + 1
                )));
                assert!(stdout.contains(&format!(
                    r#""observation_bytes":{},"reserved_observation_bytes":0,"#,
                    observations - 2
                )));
                assert!(
                    diagnostic_has_profile(&stdout, profile),
                    "{profile} {stdout}"
                );
            } else {
                assert_eq!(json.status.code(), Some(0), "{profile} {stdout}");
                assert!(stdout.contains(&format!(r#""observation_bytes":{expected_bytes},"#)));
                assert!(stdout.contains(r#""reserved_observation_bytes":0,"diagnostics":[]"#));
            }
            let text = nexo(&["run", &path, "--profile", profile]);
            assert_eq!(
                text.status.code(),
                Some(if rejected { 1 } else { 0 }),
                "{profile}"
            );
            let mut expected_text = "0\n".repeat(prints);
            if print_ten && !rejected {
                expected_text.push_str("10\n");
            }
            assert_eq!(text.stdout, expected_text.as_bytes(), "{profile}");
            if rejected {
                let stderr = String::from_utf8(text.stderr).unwrap();
                assert!(stderr.starts_with("R0002:"), "{profile}");
                assert!(stderr.contains(&format!("profile: {profile}")), "{profile}");
                assert!(stderr.contains(&format!(
                    "\"limit\":{observations},\"observed\":{},\"runtime_budget_kind\":\"observation_bytes\"",
                    observations + 1
                )));
            } else {
                assert_eq!(text.stderr, b"nexo-result:()\n");
            }
        }
    }
    eprintln!(
        "runtime_profile_boundary_matrix profiles=8 resources=4 points=3 cases=96 observation64_cells_preempted=2"
    );
}

#[test]
fn drop_destroys_resource_before_materializing_unit_at_the_cell_boundary() {
    // A2 §11.6 l. 1065–1073: charge Drop's work first, destroy the resource,
    // then materialize its unit result without holding both values at once.
    let directory = TestDirectory::new();
    let values = vec!["0"; 62].join(",");
    let source =
        format!("fn Main(){{let array=[{values}];let resource=MockOpen(1);Drop(resource);}}");
    let output = run_json(&directory, &source, CELLS64);
    assert_eq!(output.status.code(), Some(0));
    assert_eq!(
        String::from_utf8(output.stdout).unwrap(),
        format!(
            "{{\"schema_version\":1,\"profile_id\":{},\"outcome\":{{\"kind\":\"unit\"}},\"stdout\":[],\"events\":[\"open:0:1\",\"close:0:1\"],\"observations\":[{{\"channel\":\"event\",\"text\":\"open:0:1\"}},{{\"channel\":\"event\",\"text\":\"close:0:1\"}}],\"observation_bytes\":47,\"reserved_observation_bytes\":0,\"diagnostics\":[]}}\n",
            json_string(CELLS64)
        )
    );
}

#[test]
fn ir_32_analysis_profile_reports_its_own_limit_before_runtime_execution() {
    let directory = TestDirectory::new();
    let profile = "f0-0002-b-test-ir-32-v2";
    for (resource, source) in [
        ("work", "fn Main(){var i=0;while i<71427{i=i+1;}}"),
        (
            "frames",
            "fn Step(n:view I64){if 0<n{Step(n-1);}}fn Main(){Step(254);}",
        ),
        (
            "cells",
            "fn Main(){let resource=MockOpen(1);let spare=0;let rows=MockReadRecords(resource);Drop(rows);Drop(resource);}",
        ),
        (
            "observations",
            "fn Main(){var i=0;while i<32768{Print(0);i=i+1;}}",
        ),
    ] {
        let path = directory.file(&format!("ir-{resource}-preempted.n"), source);
        let output = nexo(&["run", &path, "--profile", profile, "--format", "json"]);
        let stdout = String::from_utf8(output.stdout).unwrap();
        assert_eq!(output.status.code(), Some(1), "{resource} {stdout}");
        assert!(
            stdout.contains(r#""code":"NX-E0003""#),
            "{resource} {stdout}"
        );
        assert!(
            diagnostic_has_profile(&stdout, profile),
            "{resource} {stdout}"
        );
        assert!(stdout.contains(r#""budget_kind":"ir_items","budget_scope":"compilation""#));
        assert!(
            stdout.contains(r#""incomplete_analysis":"lower","limit":32,"observed":33"#),
            "{resource} {stdout}"
        );
        assert!(!stdout.contains(r#""code":"R0001""#), "{resource} {stdout}");
    }

    let runtime_source = "fn Main(){while true{}}";
    let runtime_path = directory.file("ir-runtime.n", runtime_source);
    let runtime = nexo(&[
        "run",
        &runtime_path,
        "--profile",
        profile,
        "--format",
        "json",
    ]);
    let runtime_stdout = String::from_utf8(runtime.stdout).unwrap();
    assert_eq!(runtime.status.code(), Some(1), "{runtime_stdout}");
    assert!(
        runtime_stdout.contains(r#""code":"R0001""#),
        "{runtime_stdout}"
    );
    assert!(
        diagnostic_has_profile(&runtime_stdout, profile),
        "{runtime_stdout}"
    );
    assert!(
        runtime_stdout.contains(r#""limit":1000000,"observed":1000001,"work_consumed":1000000"#)
    );
    eprintln!(
        "ir_32_profile=analysis_limit_32 work_frames_cells_observations=NX-E0003 tiny_loop_reaches=R0001"
    );
}

#[test]
fn reduced_types32_profile_reports_the_exact_source_budget_boundary() {
    let declarations = (0..64)
        .map(|index| format!("let value{index}:I64=0;"))
        .collect::<String>();
    let source = format!("fn Main(){{{declarations}}}");
    let result = nexo::ir::compile_scalar_with_profile(
        "types32-boundary.n",
        &source,
        nexo::ir::ReducedProfile::Types32,
        false,
    );
    let nexo::ir::CompileError::Rejected(errors) = result.unwrap_err() else {
        panic!("64 annotated bindings must exceed the reduced type-solver profile");
    };
    let diagnostic = errors
        .iter()
        .find(|diagnostic| diagnostic.code == "NX-E0003")
        .expect("type_solver_steps diagnostic");
    assert_eq!(diagnostic.profile_id, "f0-0002-b-test-types-32-v2");
    assert_eq!(diagnostic.stage.as_str(), "types");
    assert_eq!(
        diagnostic.data.get("budget_kind"),
        Some(&nexo::diagnostic::DataValue::String(
            "type_solver_steps".into()
        ))
    );
    assert_eq!(
        diagnostic.data.get("limit"),
        Some(&nexo::diagnostic::DataValue::UInt(32))
    );
    assert_eq!(
        diagnostic.data.get("observed"),
        Some(&nexo::diagnostic::DataValue::UInt(33))
    );
    diagnostic.validate("types32-boundary.n", &source).unwrap();
}

#[test]
#[ignore = "normal-profile ir_items source witness; execute through scripts/i11a_campanas.py"]
fn normal_ir_item_limit_is_profiled_and_reports_the_declared_budget() {
    let declarations = (0..65_537)
        .map(|index| format!("let x{index}=0;"))
        .collect::<String>();
    let source = format!("fn Main(){{{declarations}}}");
    assert!(source.len() < 1_048_576);
    let nexo::ir::CompileError::Rejected(errors) =
        nexo::ir::compile_scalar("normal-ir-boundary.n", &source).unwrap_err()
    else {
        panic!("the normal IR-item profile must reject the generated declarations");
    };
    let diagnostic = errors
        .iter()
        .find(|diagnostic| diagnostic.code == "NX-E0003")
        .expect("IR quota diagnostic");
    assert_eq!(diagnostic.profile_id, "f0-0002-b-v2");
    assert_eq!(diagnostic.stage.as_str(), "lower");
    assert_eq!(
        diagnostic.data.get("budget_kind"),
        Some(&nexo::diagnostic::DataValue::String("ir_items".into()))
    );
    assert_eq!(
        diagnostic.data.get("budget_scope"),
        Some(&nexo::diagnostic::DataValue::String("compilation".into()))
    );
    assert_eq!(
        diagnostic.data.get("limit"),
        Some(&nexo::diagnostic::DataValue::UInt(262_144))
    );
    assert_eq!(
        diagnostic.data.get("observed"),
        Some(&nexo::diagnostic::DataValue::UInt(262_145))
    );
    assert_eq!(
        diagnostic.data.get("incomplete_analysis"),
        Some(&nexo::diagnostic::DataValue::String("lower".into()))
    );
    diagnostic
        .validate("normal-ir-boundary.n", &source)
        .unwrap();
    eprintln!("normal_ir_items_source_witness=NX-E0003 limit=262144 observed=262145");
}

#[test]
fn normal_type_solver_limit_is_reached_by_repeated_deep_array_comparisons() {
    // A2 §11.8 point 1 charges every logical type-node pair; depth 100 gives
    // 101 pairs per AssertEq. Repetition is fixed from that published cost.
    let value = format!("{}1{}", "[".repeat(100), "]".repeat(100));
    let mut source = format!("fn Main(){{let left={value};let right={value};");
    source.push_str(&"AssertEq(left,right);".repeat(11_000));
    source.push('}');
    assert!(source.len() < 1_048_576);
    let nexo::ir::CompileError::Rejected(errors) =
        nexo::ir::compile_scalar("normal-type-budget.n", &source).unwrap_err()
    else {
        panic!("the repeated array comparisons exceed the normal solver quota");
    };
    let diagnostic = errors
        .iter()
        .find(|diagnostic| diagnostic.code == "NX-E0003")
        .expect("normal type-solver diagnostic");
    assert_eq!(diagnostic.profile_id, "f0-0002-b-v2");
    assert_eq!(diagnostic.stage.as_str(), "types");
    assert_eq!(
        diagnostic.data.get("budget_kind"),
        Some(&nexo::diagnostic::DataValue::String(
            "type_solver_steps".into()
        ))
    );
    assert_eq!(
        diagnostic.data.get("budget_scope"),
        Some(&nexo::diagnostic::DataValue::String("compilation".into()))
    );
    assert_eq!(
        diagnostic.data.get("limit"),
        Some(&nexo::diagnostic::DataValue::UInt(1_000_000))
    );
    assert_eq!(
        diagnostic.data.get("observed"),
        Some(&nexo::diagnostic::DataValue::UInt(1_000_001))
    );
    assert_eq!(
        diagnostic.data.get("incomplete_analysis"),
        Some(&nexo::diagnostic::DataValue::String("types".into()))
    );
    diagnostic
        .validate("normal-type-budget.n", &source)
        .unwrap();
}

#[test]
fn mock_intrinsics_declare_io_and_read_records_also_declares_alloc() {
    let cases = [
        (
            "MockOpen",
            "fn F() with no_io{let r=MockOpen(1);} ",
            "fn F(){let r=MockOpen(1);} ",
            "intrinsic.call",
            "MockOpen",
        ),
        (
            "MockBlockingOpen",
            "fn F() with no_io{let r=MockBlockingOpen(1);} ",
            "fn F(){let r=MockBlockingOpen(1);} ",
            "intrinsic.call",
            "MockBlockingOpen",
        ),
        (
            "Inspect",
            "fn F(r:view MockResource) with no_io{Inspect(r);} ",
            "fn F(r:view MockResource){Inspect(r);} ",
            "intrinsic.call",
            "Inspect",
        ),
        (
            "MockReadRecords",
            "fn F(r:view MockResource) with no_io{let rows=MockReadRecords(r);} ",
            "fn F(r:view MockResource){let rows=MockReadRecords(r);} ",
            "intrinsic.call",
            "MockReadRecords",
        ),
        (
            "Drop",
            "fn F(r:move MockResource) with no_io{Drop(r);} ",
            "fn F(r:move MockResource){Drop(r);} ",
            "value.drop",
            "Drop",
        ),
        (
            "MockWriteI64",
            "fn F() with no_io{MockWriteI64(1);} ",
            "fn F(){MockWriteI64(1);} ",
            "intrinsic.call",
            "MockWriteI64",
        ),
        (
            "MockRequest",
            "fn F() with no_io{let result=MockRequest();} ",
            "fn F(){let result=MockRequest();} ",
            "intrinsic.call",
            "MockRequest",
        ),
        (
            "MockReply",
            "fn F() with no_io{MockReply(200);} ",
            "fn F(){MockReply(200);} ",
            "intrinsic.call",
            "MockReply",
        ),
    ];
    for (intrinsic, negative, positive, expected_operation, expected_primary) in cases {
        let negative = format!("{negative}fn Main(){{}}");
        let rejected = nexo::ir::compile_scalar("intrinsic-effects.n", &negative).unwrap_err();
        let nexo::ir::CompileError::Rejected(errors) = rejected else {
            panic!("{intrinsic} must reject no_io")
        };
        assert_eq!(errors.len(), 1, "{intrinsic}: {errors:?}");
        assert_eq!(errors[0].code, "NX-E0415", "{intrinsic}: {errors:?}");
        assert_eq!(errors[0].subcause, None, "{intrinsic}");
        assert_eq!(errors[0].stage.as_str(), "effects", "{intrinsic}");
        assert_eq!(errors[0].profile_id, "f0-0002-b-v2", "{intrinsic}");
        assert_eq!(errors[0].rule, "NX-E0415", "{intrinsic}");
        assert_eq!(errors[0].explanation_id.as_deref(), Some("NX-E0415"));
        assert_eq!(
            errors[0].message, "La función F infringe no_io.",
            "{intrinsic}"
        );
        assert_eq!(
            errors[0].operation.as_str(),
            expected_operation,
            "{intrinsic}"
        );
        let start = negative.find(expected_primary).unwrap();
        let contract_start = negative.find("no_io").unwrap();
        let error = &errors[0];
        assert_eq!(
            error
                .primary_span
                .as_ref()
                .map(|span| (span.start as usize, span.end as usize)),
            Some((start, start + expected_primary.len())),
            "{intrinsic}"
        );
        assert_eq!(
            error.data,
            [
                (
                    "contract_origin".into(),
                    nexo::diagnostic::DataValue::Object(
                        [
                            (
                                "end".into(),
                                nexo::diagnostic::DataValue::UInt((contract_start + 5) as u64),
                            ),
                            (
                                "file".into(),
                                nexo::diagnostic::DataValue::String("intrinsic-effects.n".into()),
                            ),
                            (
                                "start".into(),
                                nexo::diagnostic::DataValue::UInt(contract_start as u64),
                            ),
                        ]
                        .into(),
                    ),
                ),
                (
                    "effect_evidence".into(),
                    nexo::diagnostic::DataValue::String("known_effect".into()),
                ),
                (
                    "effect_model".into(),
                    nexo::diagnostic::DataValue::String("f0-0002-sim-v1".into()),
                ),
                (
                    "required_effect".into(),
                    nexo::diagnostic::DataValue::String("io".into()),
                ),
            ]
            .into(),
            "{intrinsic}"
        );
        assert_eq!(error.related_spans.len(), 1, "{intrinsic}");
        assert_eq!(
            error.related_spans[0].role,
            nexo::diagnostic::RelatedRole::Contract,
            "{intrinsic}"
        );
        assert_eq!(error.related_spans[0].span.file, "intrinsic-effects.n");
        assert_eq!(
            (
                error.related_spans[0].span.start,
                error.related_spans[0].span.end
            ),
            (contract_start as u64, (contract_start + 5) as u64),
            "{intrinsic}"
        );
        assert_eq!(
            error.cause_chain.len(),
            if intrinsic == "Drop" { 2 } else { 1 },
            "{intrinsic}: {:?}",
            error.cause_chain
        );
        for cause in &error.cause_chain {
            assert_eq!(
                cause.evidence,
                nexo::diagnostic::Evidence::Verified,
                "{intrinsic}"
            );
        }
        if intrinsic != "Drop" {
            assert_eq!(
                error.cause_chain[0].operation.as_str(),
                expected_operation,
                "{intrinsic}"
            );
            assert_eq!(
                error.cause_chain[0]
                    .span
                    .as_ref()
                    .map(|span| (span.start as usize, span.end as usize)),
                Some((start, start + expected_primary.len())),
                "{intrinsic}"
            );
        }
        error.validate("intrinsic-effects.n", &negative).unwrap();

        let positive = format!("{positive}fn Main(){{}}");
        nexo::ir::compile_scalar("intrinsic-effects.n", &positive)
            .unwrap_or_else(|error| panic!("{intrinsic} with io: {error:?}"));
    }

    let no_alloc =
        "fn F(r:view MockResource) with no_alloc{let rows=MockReadRecords(r);} fn Main(){}";
    let nexo::ir::CompileError::Rejected(errors) =
        nexo::ir::compile_scalar("read-alloc-effect.n", no_alloc).unwrap_err()
    else {
        panic!("MockReadRecords must reject no_alloc")
    };
    assert_eq!(errors[0].code, "NX-E0412");
    assert_eq!(
        errors[0].data.get("required_effect"),
        Some(&nexo::diagnostic::DataValue::String("alloc".into()))
    );

    // Exercise the public check command for each row as well as the compiler API.
    let directory = TestDirectory::new();
    for (intrinsic, negative, positive, _, _) in cases {
        let negative_path = directory.file("negative.n", &format!("{negative}fn Main(){{}}"));
        let rejected = nexo(&["check", &negative_path, "--format", "json"]);
        assert_eq!(rejected.status.code(), Some(1), "{intrinsic}");
        let stdout = String::from_utf8(rejected.stdout).unwrap();
        assert!(
            stdout.contains(r#""code":"NX-E0415""#),
            "{intrinsic} {stdout}"
        );
        assert!(
            stdout.contains(r#""required_effect":"io""#),
            "{intrinsic} {stdout}"
        );

        let positive_path = directory.file("positive.n", &format!("{positive}fn Main(){{}}"));
        let accepted = nexo(&["check", &positive_path, "--format", "json"]);
        assert_eq!(accepted.status.code(), Some(0), "{intrinsic}");
        assert!(
            String::from_utf8(accepted.stdout)
                .unwrap()
                .contains(r#""outcome":{"kind":"checked"}"#)
        );
    }
}

#[test]
fn indexed_result_element_copy_uses_each_cell_at_the_work_frontier() {
    let directory = TestDirectory::new();
    let environment = directory.file(
        "environment.json",
        r#"{"schema_version":1,"mock_read_records":[{"ok":[{"ok":"5"},{"ok":"6"}]}],"mock_write_i64":[],"mock_request":[],"mock_reply":[]}"#,
    );
    for (blocks, rejected) in [(5usize, false), (6, true)] {
        let source = format!(
            "fn Main()->Result<(),I64>{{var fl=0;while fl<71425{{fl=fl+1;}}{}let x1=MockReadRecords(MockOpen(1))?;let value=x1[0]?;return Ok(());}}",
            "{}".repeat(blocks)
        );
        let program = directory.file("index-result.n", &source);
        let output = nexo(&[
            "run",
            &program,
            "--mock-env",
            &environment,
            "--format",
            "json",
        ]);
        let stdout = String::from_utf8(output.stdout).unwrap();
        if rejected {
            assert_eq!(output.status.code(), Some(1), "{stdout}");
            assert!(stdout.contains(r#""code":"R0001""#), "{stdout}");
            assert!(stdout.contains(&format!(
                r#""limit":{WORK_LIMIT},"observed":1000001,"work_consumed":1000000"#
            )));
        } else {
            assert_eq!(output.status.code(), Some(0), "{stdout}");
            assert!(stdout.contains(r#""outcome":{"kind":"ok"}"#), "{stdout}");
        }
    }
}

#[test]
fn entry_tick_exhaustion_closes_every_temporary_resource_primitive() {
    // A2 §11.2 l. 903 requires cleanup and close-reservation conversion to survive
    // work exhaustion; §11.6 l. 1068 and §12.4 l. 1594 repeat the uninterruptible
    // close rule. §11.5 l. 970 fixes the 23+24-byte resource-event accounting.
    let directory = TestDirectory::new();
    let environment = directory.file(
        "environment.json",
        r#"{"schema_version":1,"mock_read_records":[{"ok":[]}],"mock_write_i64":[],"mock_request":[],"mock_reply":[]}"#,
    );
    let cases = [
        (
            "MockReadRecords",
            "let result=MockReadRecords(MockOpen(1))?;",
            "MockReadRecords(MockOpen(1))",
        ),
        ("Inspect", "Inspect(MockOpen(1));", "Inspect(MockOpen(1))"),
        ("Drop", "Drop(MockOpen(1));", "Drop(MockOpen(1))"),
        (
            "Drop(MockBlockingOpen)",
            "Drop(MockBlockingOpen(1));",
            "Drop(MockBlockingOpen(1))",
        ),
    ];
    let primitive_cases = cases.len();
    let candidate_sources = primitive_cases * 5;
    let mut matching_entry_faults = 0;

    for (primitive, statement, call) in cases {
        let mut entry_faults = 0;
        for blocks in 12..=16 {
            let source = format!(
                "fn Main()->Result<(),I64>{{var fl=0;while fl<71426{{fl=fl+1;}}{}{statement}return Ok(());}}",
                "{}".repeat(blocks)
            );
            let path = directory.file("entry-boundary.n", &source);
            let output = nexo(&["run", &path, "--mock-env", &environment, "--format", "json"]);
            let stdout = String::from_utf8(output.stdout).unwrap();
            let call_start = source.find(call).unwrap();
            let call_end = call_start + call.len();
            let primary = format!(
                r#""primary_span":{{"file":{},"start":{},"end":{}}}"#,
                json_string(&path),
                call_start,
                call_end
            );
            if stdout.contains(r#""code":"R0001""#) && stdout.contains(&primary) {
                entry_faults += 1;
                assert_eq!(output.status.code(), Some(1), "{primitive} {stdout}");
                assert!(
                    stdout.contains(r#""events":["open:0:1","close:0:1"]"#),
                    "{primitive} blocks={blocks}: {stdout}"
                );
                assert!(
                    stdout.contains(r#""observation_bytes":47,"reserved_observation_bytes":0,"#),
                    "{primitive} blocks={blocks}: {stdout}"
                );
                assert!(
                    stdout.contains(
                        r#""data":{"limit":1000000,"observed":1000001,"work_consumed":1000000}"#
                    ),
                    "{primitive} blocks={blocks}: {stdout}"
                );

                let diagnostic =
                    runtime_work_diagnostic("f0-0002-b-v2", &path, call_start, call_end);
                let expected = source_runtime_fault_json(
                    "f0-0002-b-v2",
                    &["open:0:1", "close:0:1"],
                    47,
                    0,
                    &diagnostic,
                );
                assert_eq!(stdout, expected, "{primitive} blocks={blocks}");

                let text_output = nexo(&["run", &path, "--mock-env", &environment]);
                assert_eq!(text_output.status.code(), Some(1), "{primitive}");
                assert_eq!(text_output.stdout, b"", "{primitive}");
                assert_eq!(
                    String::from_utf8(text_output.stderr).unwrap(),
                    format!(
                        "mock-resource:open:0:1\nmock-resource:close:0:1\n{}",
                        runtime_work_diagnostic_text("f0-0002-b-v2", &path, call_start, call_end)
                    ),
                    "{primitive}"
                );
            }
        }
        assert!(
            entry_faults > 0,
            "no entry-boundary case generated for {primitive}"
        );
        matching_entry_faults += entry_faults;
    }
    eprintln!(
        "entry_tick_temp_matrix primitives={primitive_cases} candidates={candidate_sources} matched_faults={matching_entry_faults} minimum_per_primitive=1"
    );
}

#[test]
fn compile_time_diagnostics_and_unknown_profile_json_keep_selected_profile_ids() {
    let directory = TestDirectory::new();
    let source = directory.file("type-error.n", "fn Main(){MockWriteI64(true);}");
    for profile in [
        "f0-0002-b-v2",
        "f0-0002-b-test-types-32-v2",
        "f0-0002-b-test-flow-32-v2",
        "f0-0002-b-test-flow-global-32-v2",
        "f0-0002-b-test-effects-32-v2",
        "f0-0002-b-test-ir-32-v2",
        FRAMES8,
        CELLS64,
        OBSERVATIONS64,
    ] {
        let output = nexo(&["run", &source, "--profile", profile, "--format", "json"]);
        let stdout = String::from_utf8(output.stdout).unwrap();
        assert_eq!(output.status.code(), Some(1), "{profile} {stdout}");
        assert!(stdout.contains(r#""code":"NX-E0102""#), "{stdout}");
        assert!(
            diagnostic_has_profile(&stdout, profile),
            "{profile} {stdout}"
        );
    }

    let syntax_rejected = directory.file("syntax-error.n", "fn Main(){let value=;}");
    for profile in [
        "f0-0002-b-v2",
        "f0-0002-b-test-types-32-v2",
        "f0-0002-b-test-flow-32-v2",
        "f0-0002-b-test-flow-global-32-v2",
        "f0-0002-b-test-effects-32-v2",
        "f0-0002-b-test-ir-32-v2",
        FRAMES8,
        CELLS64,
        OBSERVATIONS64,
    ] {
        let output = nexo(&[
            "run",
            &syntax_rejected,
            "--profile",
            profile,
            "--format",
            "json",
        ]);
        let stdout = String::from_utf8(output.stdout).unwrap();
        assert_eq!(output.status.code(), Some(1), "{profile} {stdout}");
        assert!(
            stdout.contains(r#""code":"NX-E0002""#),
            "{profile} {stdout}"
        );
        assert!(
            diagnostic_has_profile(&stdout, profile),
            "{profile} {stdout}"
        );
    }

    let valid = directory.file("empty.n", "fn Main(){}");
    for command in ["run", "check"] {
        let unknown = nexo(&[command, &valid, "--profile", "bogus", "--format", "json"]);
        assert_eq!(unknown.status.code(), Some(2));
        assert_eq!(
            unknown.stdout,
            b"{\"schema_version\":1,\"profile_id\":\"bogus\",\"outcome\":{\"kind\":\"driver_error\",\"reason\":\"unknown_profile\"},\"stdout\":[],\"events\":[],\"observations\":[],\"observation_bytes\":0,\"reserved_observation_bytes\":0,\"diagnostics\":[]}\n"
        );
        assert_eq!(unknown.stderr, b"");
    }
}
