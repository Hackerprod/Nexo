use std::{
    fs,
    path::{Path, PathBuf},
    process::{Command, Output},
    sync::atomic::{AtomicUsize, Ordering},
};

const NORMAL: &str = "f0-0002-b-v2";
const TYPES: &str = "f0-0002-b-test-types-32-v2";
const FLOW: &str = "f0-0002-b-test-flow-32-v2";
const GLOBAL: &str = "f0-0002-b-test-flow-global-32-v2";
const EFFECTS: &str = "f0-0002-b-test-effects-32-v2";
const IR: &str = "f0-0002-b-test-ir-32-v2";
const FRAMES: &str = "f0-0002-b-test-frames-8-v2";
const CELLS: &str = "f0-0002-b-test-cells-64-v2";
const OBSERVATIONS: &str = "f0-0002-b-test-observations-64-v2";

static NEXT: AtomicUsize = AtomicUsize::new(0);

struct QuotaExpectation {
    kind: &'static str,
    scope: &'static str,
    incomplete: &'static str,
}

struct QuotaCase {
    name: &'static str,
    source: String,
    profile: &'static str,
    expected: Option<QuotaExpectation>,
}

struct AnalysisFrontier {
    label: &'static str,
    profile: &'static str,
    kind: &'static str,
    scope: &'static str,
    incomplete: &'static str,
    below: usize,
    at: usize,
    above: usize,
    source_for: fn(usize) -> String,
}

struct TestDirectory(PathBuf);

impl TestDirectory {
    fn new() -> Self {
        let path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("target")
            .join("test-fixtures")
            .join(format!(
                "i11a-profile-selection-{}-{}",
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

fn output_text(bytes: Vec<u8>) -> String {
    String::from_utf8(bytes).unwrap()
}

fn json_quote(text: &str) -> String {
    let mut quoted = String::from("\"");
    for character in text.chars() {
        match character {
            '"' => quoted.push_str("\\\""),
            '\\' => quoted.push_str("\\\\"),
            '\n' => quoted.push_str("\\n"),
            '\r' => quoted.push_str("\\r"),
            '\t' => quoted.push_str("\\t"),
            character if character < ' ' => {
                quoted.push_str(&format!("\\u{:04x}", character as u32));
            }
            character => quoted.push(character),
        }
    }
    quoted.push('"');
    quoted
}

fn assert_analysis_diagnostic(
    output: &str,
    profile: &str,
    kind: &str,
    scope: &str,
    incomplete: &str,
) {
    for field in [
        r#""diagnostic_namespace":"Nexo-proposed-v1""#,
        r#""code":"NX-E0003""#,
        r#""subcause":null"#,
        r#""severity":"error""#,
        r#""operation":"analysis.limit""#,
        r#""related_spans":[]"#,
        r#""cause_chain":[]"#,
    ] {
        assert!(
            output.contains(field),
            "{profile} missing {field}: {output}"
        );
    }
    assert!(
        output.contains(&format!(r#""profile_id":"{profile}""#)),
        "{output}"
    );
    assert!(
        output.contains(&format!(r#""budget_kind":"{kind}""#)),
        "{output}"
    );
    assert!(
        output.contains(&format!(r#""budget_scope":"{scope}""#)),
        "{output}"
    );
    assert!(
        output.contains(&format!(r#""incomplete_analysis":"{incomplete}""#)),
        "{output}"
    );
    assert!(output.contains(r#""limit":32,"observed":33"#), "{output}");
    assert!(output.contains(r#""primary_span":{"file":"#), "{output}");
}

#[test]
fn analysis_quota_profiles_are_selected_by_their_cli_identifier() {
    // The expected profile rows and limits are transcribed from A2 §11.4–§11.5
    // and D15 §6.1, independently of ReducedProfile::from_id.
    let nested = format!(
        "fn F0(b:view Bool){{{}{}}}fn Main(){{}}",
        "while b{".repeat(8),
        "}".repeat(8)
    );
    let ifs = |count: usize| {
        (0..count)
            .map(|index| format!("fn F{index}(){{let x=(if true then 1 else 1);}}\n"))
            .collect::<String>()
            + "fn Main(){}\n"
    };
    let empties = |count: usize| {
        (0..count)
            .map(|index| format!("fn F{index}(){{}}"))
            .collect::<String>()
            + "fn Main(){}"
    };
    let types = |count: usize| format!("fn Main(){{var a=1;{}}}", "a=a+1;".repeat(count));
    let cases = vec![
        QuotaCase {
            name: "types-limit",
            source: types(11),
            profile: TYPES,
            expected: Some(QuotaExpectation {
                kind: "type_solver_steps",
                scope: "compilation",
                incomplete: "types",
            }),
        },
        QuotaCase {
            name: "types-below",
            source: types(10),
            profile: TYPES,
            expected: None,
        },
        QuotaCase {
            name: "types-normal-control",
            source: types(11),
            profile: NORMAL,
            expected: None,
        },
        QuotaCase {
            name: "types-frames-control",
            source: types(11),
            profile: FRAMES,
            expected: None,
        },
        QuotaCase {
            name: "types-cells-control",
            source: types(11),
            profile: CELLS,
            expected: None,
        },
        QuotaCase {
            name: "types-observations-control",
            source: types(11),
            profile: OBSERVATIONS,
            expected: None,
        },
        QuotaCase {
            name: "nested-flow",
            source: nested.clone(),
            profile: FLOW,
            expected: Some(QuotaExpectation {
                kind: "ownership_flow_steps",
                scope: "function",
                incomplete: "ownership",
            }),
        },
        QuotaCase {
            name: "nested-global",
            source: nested.clone(),
            profile: GLOBAL,
            expected: Some(QuotaExpectation {
                kind: "ownership_flow_steps",
                scope: "compilation",
                incomplete: "ownership",
            }),
        },
        QuotaCase {
            name: "local-flow-control",
            source: nested.clone(),
            profile: NORMAL,
            expected: None,
        },
        QuotaCase {
            name: "global-flow-control",
            source: ifs(1),
            profile: GLOBAL,
            expected: None,
        },
        QuotaCase {
            name: "effects-limit",
            source: empties(32),
            profile: EFFECTS,
            expected: Some(QuotaExpectation {
                kind: "effect_fixpoint_steps",
                scope: "compilation",
                incomplete: "effects",
            }),
        },
        QuotaCase {
            name: "effects-below",
            source: empties(31),
            profile: EFFECTS,
            expected: None,
        },
        QuotaCase {
            name: "effects-normal-control",
            source: empties(32),
            profile: NORMAL,
            expected: None,
        },
        QuotaCase {
            name: "ir-limit",
            source: ifs(1),
            profile: IR,
            expected: Some(QuotaExpectation {
                kind: "ir_items",
                scope: "compilation",
                incomplete: "lower",
            }),
        },
        QuotaCase {
            name: "ir-normal-control",
            source: ifs(1),
            profile: NORMAL,
            expected: None,
        },
    ];

    let directory = TestDirectory::new();
    for case in cases {
        let path = directory.file(&format!("{}.n", case.name), &case.source);
        for command in ["check", "run"] {
            let output = nexo(&[
                command,
                &path,
                "--profile",
                case.profile,
                "--format",
                "json",
            ]);
            let stdout = output_text(output.stdout);
            match &case.expected {
                None => {
                    assert_eq!(
                        output.status.code(),
                        Some(0),
                        "{} {}: {stdout}",
                        case.name,
                        case.profile
                    );
                    assert!(
                        stdout.contains(&format!(r#""profile_id":"{}""#, case.profile)),
                        "{stdout}"
                    );
                    assert_eq!(output.stderr, b"");
                }
                Some(expected) => {
                    assert_eq!(
                        output.status.code(),
                        Some(1),
                        "{} {}: {stdout}",
                        case.name,
                        case.profile
                    );
                    assert_analysis_diagnostic(
                        &stdout,
                        case.profile,
                        expected.kind,
                        expected.scope,
                        expected.incomplete,
                    );
                    assert_eq!(output.stderr, b"");
                }
            }
        }
    }

    // Flow-function and flow-global are distinct published scopes: the same
    // local witness rejects only in the former, while enough functions exhaust
    // the latter compilation-wide budget.
    let global_source = ifs(2);
    let global_path = directory.file("global-flow-limit.n", &global_source);
    let global = nexo(&[
        "check",
        &global_path,
        "--profile",
        GLOBAL,
        "--format",
        "json",
    ]);
    let global_stdout = output_text(global.stdout);
    assert_eq!(global.status.code(), Some(1), "{global_stdout}");
    assert_analysis_diagnostic(
        &global_stdout,
        GLOBAL,
        "ownership_flow_steps",
        "compilation",
        "ownership",
    );

    // Every listed ID is recognized by the CLI even when the source is under
    // the selected analysis budget. A bad identifier is rejected before read.
    let empty = directory.file("empty.n", "fn Main(){}");
    for profile in [
        NORMAL,
        TYPES,
        FLOW,
        GLOBAL,
        EFFECTS,
        IR,
        FRAMES,
        CELLS,
        OBSERVATIONS,
    ] {
        let checked = nexo(&["check", &empty, "--profile", profile, "--format", "json"]);
        assert_eq!(
            checked.status.code(),
            Some(0),
            "{profile}: {}",
            output_text(checked.stdout)
        );
    }
    let unknown = nexo(&[
        "check",
        "missing-source.n",
        "--profile",
        "unpublished",
        "--format",
        "json",
    ]);
    assert_eq!(unknown.status.code(), Some(2));
    assert_eq!(
        unknown.stdout,
        b"{\"schema_version\":1,\"profile_id\":\"unpublished\",\"outcome\":{\"kind\":\"driver_error\",\"reason\":\"unknown_profile\"},\"stdout\":[],\"events\":[],\"observations\":[],\"observation_bytes\":0,\"reserved_observation_bytes\":0,\"diagnostics\":[]}\n"
    );
    assert_eq!(unknown.stderr, b"");
}

#[test]
fn selected_profile_is_preserved_on_input_and_mock_environment_failures() {
    let directory = TestDirectory::new();
    let unreadable_source = directory.0.join("missing-source.n");
    let source = directory.file("empty.n", "fn Main(){}");
    let missing_environment = directory.0.join("missing-environment.json");
    let invalid_source = directory.0.join("invalid-utf8.n");
    fs::write(&invalid_source, [0xff]).unwrap();
    let oversized_source = directory.0.join("oversized-source.n");
    fs::write(&oversized_source, vec![b' '; 1_048_577]).unwrap();
    let static_error = directory.file("static-error.n", "fn Main(){MockWriteI64(true);}");
    let runtime_error = directory.file("runtime-error.n", "fn Main(){while true{}}");

    for profile in [
        NORMAL,
        TYPES,
        FLOW,
        GLOBAL,
        EFFECTS,
        IR,
        FRAMES,
        CELLS,
        OBSERVATIONS,
    ] {
        let input_text = nexo(&[
            "check",
            unreadable_source.to_str().unwrap(),
            "--profile",
            profile,
        ]);
        assert_eq!(input_text.status.code(), Some(2), "{profile}");
        assert!(input_text.stdout.is_empty());
        let stderr = output_text(input_text.stderr);
        assert!(stderr.starts_with("NX-E0004:0:0:"), "{profile}: {stderr}");
        assert!(
            stderr.contains("subcause: source_unreadable"),
            "{profile}: {stderr}"
        );
        assert!(
            stderr.contains(&format!("profile: {profile}\n")),
            "{profile}: {stderr}"
        );
        assert!(stderr.contains("input_path"), "{profile}: {stderr}");
        assert_eq!(
            stderr,
            format!(
                "NX-E0004:0:0: No se puede leer la fuente.\n  namespace: Nexo-proposed-v1\n  profile: {profile}\n  stage: input\n  operation: input.read\n  at: null\n  subcause: source_unreadable\n  data: {{\"input_path\":{}}}\n",
                json_quote(unreadable_source.to_str().unwrap())
            ),
            "{profile}"
        );

        let input_json = nexo(&[
            "run",
            unreadable_source.to_str().unwrap(),
            "--format",
            "json",
            "--profile",
            profile,
        ]);
        assert_eq!(input_json.status.code(), Some(2), "{profile}");
        assert_eq!(input_json.stderr, b"");
        let json = output_text(input_json.stdout);
        let diagnostic = format!(
            r#"{{"schema_version":1,"diagnostic_namespace":"Nexo-proposed-v1","profile_id":"{profile}","code":"NX-E0004","subcause":"source_unreadable","rule":"NX-E0004/source_unreadable","explanation_id":"NX-E0004/source_unreadable","severity":"error","stage":"input","message":"No se puede leer la fuente.","operation":"input.read","primary_span":null,"related_spans":[],"cause_chain":[],"feature_id":null,"unexpected_token":null,"expected_tokens":[],"expected_tokens_kind":null,"parser_context":null,"recognized_prefix":null,"repair":[],"data":{{"input_path":{}}}}}"#,
            json_quote(unreadable_source.to_str().unwrap())
        );
        assert_eq!(
            json,
            format!(
                r#"{{"schema_version":1,"profile_id":"{profile}","outcome":{{"kind":"input_error"}},"stdout":[],"events":[],"observations":[],"observation_bytes":0,"reserved_observation_bytes":0,"diagnostics":[{diagnostic}]}}"#
            ) + "\n",
            "{profile}"
        );

        for (path, expected_code, expected_subcause, expected_operation, status) in [
            (
                invalid_source.to_str().unwrap(),
                "NX-E0004",
                Some("invalid_utf8"),
                "input.decode",
                2,
            ),
            (
                oversized_source.to_str().unwrap(),
                "NX-E0003",
                None,
                "analysis.limit",
                1,
            ),
        ] {
            let input = nexo(&["check", path, "--profile", profile, "--format", "json"]);
            let input_stdout = output_text(input.stdout);
            assert_eq!(
                input.status.code(),
                Some(status),
                "{profile}: {input_stdout}"
            );
            assert!(
                input_stdout.contains(&format!("\"profile_id\":\"{profile}\"")),
                "{input_stdout}"
            );
            assert!(
                input_stdout.contains(&format!("\"code\":\"{expected_code}\"")),
                "{input_stdout}"
            );
            assert!(
                input_stdout.contains(&format!("\"operation\":\"{expected_operation}\"")),
                "{input_stdout}"
            );
            if let Some(subcause) = expected_subcause {
                assert!(
                    input_stdout.contains(&format!("\"subcause\":\"{subcause}\"")),
                    "{input_stdout}"
                );
            } else {
                assert!(
                    input_stdout.contains(
                        "\"budget_kind\":\"source_bytes\",\"budget_scope\":\"compilation\",\"incomplete_analysis\":\"input\",\"limit\":1048576,\"observed\":1048577"
                    ),
                    "{input_stdout}"
                );
            }
            let text = nexo(&["check", path, "--profile", profile]);
            assert_eq!(text.status.code(), Some(status), "{profile}");
            let text_stderr = output_text(text.stderr);
            assert!(
                text_stderr.contains(&format!("profile: {profile}\n")),
                "{text_stderr}"
            );
            assert!(text_stderr.contains(expected_code), "{text_stderr}");
        }

        let mock_text = nexo(&[
            "run",
            &source,
            "--mock-env",
            missing_environment.to_str().unwrap(),
            "--profile",
            profile,
        ]);
        assert_eq!(mock_text.status.code(), Some(2), "{profile}");
        assert_eq!(mock_text.stdout, b"");
        assert_eq!(mock_text.stderr, b"nexo-driver:mock_env_unreadable\n");

        let mock_json = nexo(&[
            "run",
            &source,
            "--mock-env",
            missing_environment.to_str().unwrap(),
            "--profile",
            profile,
            "--format",
            "json",
        ]);
        assert_eq!(mock_json.status.code(), Some(2), "{profile}");
        assert_eq!(mock_json.stderr, b"");
        assert_eq!(
            mock_json.stdout,
            format!(
                "{{\"schema_version\":1,\"profile_id\":\"{profile}\",\"outcome\":{{\"kind\":\"driver_error\",\"reason\":\"mock_env_unreadable\"}},\"stdout\":[],\"events\":[],\"observations\":[],\"observation_bytes\":0,\"reserved_observation_bytes\":0,\"diagnostics\":[]}}\n"
            )
            .as_bytes()
        );

        for command in ["check", "run"] {
            let static_json = nexo(&[
                command,
                &static_error,
                "--profile",
                profile,
                "--format",
                "json",
            ]);
            let static_stdout = output_text(static_json.stdout);
            assert_eq!(
                static_json.status.code(),
                Some(1),
                "{profile}: {static_stdout}"
            );
            assert!(
                static_stdout.contains(r#""code":"NX-E0102""#),
                "{static_stdout}"
            );
            assert!(
                static_stdout.contains(&format!(r#""profile_id":"{profile}""#)),
                "{static_stdout}"
            );
            assert!(static_json.stderr.is_empty(), "{static_stdout}");
        }

        let runtime_json = nexo(&[
            "run",
            &runtime_error,
            "--profile",
            profile,
            "--format",
            "json",
        ]);
        let runtime_stdout = output_text(runtime_json.stdout);
        assert_eq!(
            runtime_json.status.code(),
            Some(1),
            "{profile}: {runtime_stdout}"
        );
        for field in [
            r#""diagnostic_namespace":"F0-0002-sim""#,
            r#""code":"R0001""#,
            r#""stage":"runtime""#,
            r#""operation":"runtime.limit""#,
            r#""limit":1000000,"observed":1000001,"work_consumed":1000000"#,
        ] {
            assert!(
                runtime_stdout.contains(field),
                "{profile} missing {field}: {runtime_stdout}"
            );
        }
        assert!(
            runtime_stdout.contains(&format!(r#""profile_id":"{profile}""#)),
            "{runtime_stdout}"
        );
        let runtime_text = nexo(&["run", &runtime_error, "--profile", profile]);
        assert_eq!(runtime_text.status.code(), Some(1), "{profile}");
        assert!(runtime_text.stdout.is_empty());
        let runtime_stderr = output_text(runtime_text.stderr);
        assert!(
            runtime_stderr.starts_with("R0001:"),
            "{profile}: {runtime_stderr}"
        );
        assert!(
            runtime_stderr.contains(&format!("profile: {profile}\n")),
            "{runtime_stderr}"
        );
    }

    let unknown_text = nexo(&[
        "check",
        unreadable_source.to_str().unwrap(),
        "--profile",
        "unpublished",
    ]);
    assert_eq!(unknown_text.status.code(), Some(2));
    assert_eq!(unknown_text.stdout, b"");
    assert_eq!(unknown_text.stderr, b"nexo-driver:unknown_profile\n");

    for command in ["check", "run"] {
        let output = nexo(&[
            command,
            unreadable_source.to_str().unwrap(),
            "--profile",
            "unpublished",
            "--format",
            "json",
        ]);
        assert_eq!(output.status.code(), Some(2));
        let stdout = output_text(output.stdout);
        assert!(stdout.contains(r#""profile_id":"unpublished","outcome":{"kind":"driver_error","reason":"unknown_profile"}"#));
        assert_eq!(output.stderr, b"");
    }
}

#[test]
fn reduced_analysis_source_frontiers_are_transcribed_from_the_profile_table() {
    // A2 §11.4–§11.5 gives the source families and 31/32/33 analysis
    // boundaries; D15 §6.1 assigns their stable CLI identifiers.
    let directory = TestDirectory::new();
    let annotated = |count: usize| {
        let declarations = (0..count)
            .map(|index| format!("let value{index}:I64=0;"))
            .collect::<String>();
        format!("fn Main(){{{declarations}}}")
    };
    let initialized = |count: usize| {
        let declarations = (0..count)
            .map(|index| format!("let value{index}=0;"))
            .collect::<String>();
        format!("fn Main(){{{declarations}}}")
    };
    let empty_functions = |count: usize| {
        (0..count)
            .map(|index| format!("fn F{index}(){{}}"))
            .collect::<String>()
            + "fn Main(){}"
    };

    let frontiers = [
        AnalysisFrontier {
            label: "types-32",
            profile: TYPES,
            kind: "type_solver_steps",
            scope: "compilation",
            incomplete: "types",
            below: 15,
            at: 16,
            above: 17,
            source_for: annotated,
        },
        AnalysisFrontier {
            label: "flow-32",
            profile: FLOW,
            kind: "ownership_flow_steps",
            scope: "function",
            incomplete: "ownership",
            below: 6,
            at: 7,
            above: 8,
            source_for: initialized,
        },
        AnalysisFrontier {
            label: "flow-global-32",
            profile: GLOBAL,
            kind: "ownership_flow_steps",
            scope: "compilation",
            incomplete: "ownership",
            below: 6,
            at: 7,
            above: 8,
            source_for: initialized,
        },
        AnalysisFrontier {
            label: "ir-32",
            profile: IR,
            kind: "ir_items",
            scope: "compilation",
            incomplete: "lower",
            below: 2,
            at: 3,
            above: 4,
            source_for: initialized,
        },
    ];

    for frontier in frontiers {
        for (units, rejected) in [
            (frontier.below, false),
            (frontier.at, false),
            (frontier.above, true),
        ] {
            let source = (frontier.source_for)(units);
            let path = directory.file(&format!("{}-{units}.n", frontier.label), &source);
            let output = nexo(&[
                "check",
                &path,
                "--profile",
                frontier.profile,
                "--format",
                "json",
            ]);
            let stdout = output_text(output.stdout);
            if rejected {
                assert_eq!(
                    output.status.code(),
                    Some(1),
                    "{} {units}: {stdout}",
                    frontier.label
                );
                assert_analysis_diagnostic(
                    &stdout,
                    frontier.profile,
                    frontier.kind,
                    frontier.scope,
                    frontier.incomplete,
                );
                let text = nexo(&["check", &path, "--profile", frontier.profile]);
                assert_eq!(text.status.code(), Some(1), "{} {units}", frontier.label);
                assert!(text.stdout.is_empty());
                let stderr = output_text(text.stderr);
                assert!(
                    stderr.starts_with("NX-E0003:"),
                    "{}: {stderr}",
                    frontier.label
                );
                assert!(
                    stderr.contains(&format!("profile: {}\n", frontier.profile)),
                    "{stderr}"
                );
                assert!(
                    stderr.contains(&format!("\"budget_kind\":\"{}\"", frontier.kind)),
                    "{stderr}"
                );
                assert!(
                    stderr.contains(&format!("\"budget_scope\":\"{}\"", frontier.scope)),
                    "{stderr}"
                );
                assert!(stderr.contains("\"limit\":32,\"observed\":33"), "{stderr}");
            } else {
                assert_eq!(
                    output.status.code(),
                    Some(0),
                    "{} {units}: {stdout}",
                    frontier.label
                );
                assert!(
                    stdout.contains(r#""outcome":{"kind":"checked"}"#),
                    "{stdout}"
                );
                assert!(
                    stdout.contains(&format!(r#""profile_id":"{}""#, frontier.profile)),
                    "{stdout}"
                );
            }
            assert_eq!(output.stderr, b"");
        }
    }

    // Each empty function contributes one effect-analysis operation. Main is
    // part of the independently counted source total: 31, 32, or 33.
    for (function_count, rejected) in [(30usize, false), (31, false), (32, true)] {
        let source = empty_functions(function_count);
        let path = directory.file(&format!("effects-{function_count}.n"), &source);
        let output = nexo(&["check", &path, "--profile", EFFECTS, "--format", "json"]);
        let stdout = output_text(output.stdout);
        if rejected {
            assert_eq!(output.status.code(), Some(1), "{stdout}");
            assert_analysis_diagnostic(
                &stdout,
                EFFECTS,
                "effect_fixpoint_steps",
                "compilation",
                "effects",
            );
        } else {
            assert_eq!(output.status.code(), Some(0), "{stdout}");
            assert!(
                stdout.contains(r#""outcome":{"kind":"checked"}"#),
                "{stdout}"
            );
        }
    }
}
