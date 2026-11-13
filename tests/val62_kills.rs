use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
    sync::atomic::{AtomicUsize, Ordering},
};

static NEXT: AtomicUsize = AtomicUsize::new(0);

const ENV: &str = r#"{"schema_version":1,"mock_read_records":[],"mock_write_i64":[],"mock_request":[],"mock_reply":[]}"#;

fn fixture(test: impl FnOnce(&Path)) {
    let directory = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("target")
        .join("test-fixtures")
        .join(format!(
            "cli-argument-matrix-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
    let _ = fs::remove_dir_all(&directory);
    fs::create_dir_all(&directory).unwrap();
    fs::write(directory.join("p.n"), "fn Main(){}").unwrap();
    fs::write(directory.join("-p.n"), "fn Main(){}").unwrap();
    fs::write(directory.join("env.json"), ENV).unwrap();
    test(&directory);
    fs::remove_dir_all(directory).unwrap();
}

fn run(directory: &Path, args: &[&str]) -> (Option<i32>, String, String) {
    let output = Command::new(env!("CARGO_BIN_EXE_nexo"))
        .current_dir(directory)
        .args(args)
        .output()
        .unwrap();
    (
        output.status.code(),
        String::from_utf8(output.stdout).unwrap(),
        String::from_utf8(output.stderr).unwrap(),
    )
}

fn usage(reason: &str) -> (Option<i32>, String, String) {
    (Some(2), String::new(), format!("nexo-driver:{reason}\n"))
}

fn ok_text() -> (Option<i32>, String, String) {
    (Some(0), String::new(), "nexo-result:()\n".into())
}

#[test]
fn empty_mock_environment_value_is_a_path() {
    fixture(|directory| {
        assert_eq!(
            run(directory, &["run", "p.n", "--mock-env", ""]),
            usage("mock_env_unreadable")
        );
        assert_eq!(
            run(directory, &["run", "p.n", "--mock-env", "", "--format", "json"]),
            (
                Some(2),
                "{\"schema_version\":1,\"profile_id\":\"f0-0002-b-v2\",\"outcome\":{\"kind\":\"driver_error\",\"reason\":\"mock_env_unreadable\"},\"stdout\":[],\"events\":[],\"observations\":[],\"observation_bytes\":0,\"reserved_observation_bytes\":0,\"diagnostics\":[]}\n".into(),
                String::new()
            )
        );
    });
}

#[test]
fn empty_profile_value_reaches_profile_selection() {
    fixture(|directory| {
        assert_eq!(
            run(directory, &["run", "p.n", "--profile", ""]),
            usage("unknown_profile")
        );
    });
}

#[test]
fn empty_format_value_is_invalid_arguments() {
    fixture(|directory| {
        assert_eq!(
            run(directory, &["run", "p.n", "--format", ""]),
            usage("invalid_arguments")
        );
    });
}

#[test]
fn profile_identifiers_are_case_sensitive() {
    fixture(|directory| {
        assert_eq!(
            run(directory, &["run", "p.n", "--profile", "F0-0002-B-V2"]),
            usage("unknown_profile")
        );
        assert_eq!(
            run(directory, &["run", "p.n", "--profile", "f0-0002-b-v2"]),
            ok_text()
        );
    });
}

#[test]
fn single_dash_source_paths_are_sources() {
    fixture(|directory| {
        assert_eq!(run(directory, &["run", "-p.n"]), ok_text());
        assert_eq!(
            run(directory, &["check", "-p.n"]),
            (Some(0), "nexo-check:ok\n".into(), String::new())
        );
        let (code, stdout, stderr) = run(directory, &["run", "-missing.n"]);
        assert_eq!(code, Some(2));
        assert_eq!(stdout, "");
        assert!(stderr.starts_with("NX-E0004:0:0:"), "{stderr}");
    });
}

#[test]
fn bare_double_dash_is_not_a_source_path_or_end_of_options() {
    fixture(|directory| {
        assert_eq!(run(directory, &["run", "--"]), usage("invalid_arguments"));
        assert_eq!(
            run(directory, &["run", "p.n", "--"]),
            usage("invalid_arguments")
        );
        assert_eq!(
            run(directory, &["check", "--", "p.n"]),
            usage("invalid_arguments")
        );
    });
}

#[test]
fn only_exact_command_names_are_accepted() {
    fixture(|directory| {
        for command in ["runx", "checkx", "Run", "CHECK", "run ", "r", ""] {
            assert_eq!(
                run(directory, &[command, "p.n"]),
                usage("invalid_arguments"),
                "command {command:?}"
            );
        }
    });
}

#[test]
fn options_may_precede_or_follow_the_source() {
    fixture(|directory| {
        assert_eq!(
            run(directory, &["run", "--mock-env", "env.json", "p.n"]),
            ok_text()
        );
        assert_eq!(
            run(directory, &["run", "p.n", "--mock-env", "env.json"]),
            ok_text()
        );
        assert_eq!(
            run(
                directory,
                &[
                    "run",
                    "--profile",
                    "f0-0002-b-v2",
                    "--mock-env",
                    "env.json",
                    "p.n",
                    "--format",
                    "text"
                ]
            ),
            ok_text()
        );
        assert_eq!(
            run(
                directory,
                &["run", "--mock-env", "env.json", "--format", "json", "p.n"]
            )
            .0,
            Some(0)
        );
        assert_eq!(
            run(
                directory,
                &["run", "--format", "json", "--mock-env", "env.json", "p.n"]
            )
            .0,
            Some(0)
        );
        assert_eq!(
            run(
                directory,
                &[
                    "run",
                    "--format",
                    "text",
                    "--profile",
                    "f0-0002-b-v2",
                    "p.n"
                ]
            ),
            ok_text()
        );
        assert_eq!(
            run(
                directory,
                &["run", "p.n", "--format", "text", "--mock-env", "env.json"]
            ),
            ok_text()
        );
    });
}

#[test]
fn surrogate_pair_boundaries_decode_as_one_scalar_before_schema_validation() {
    fixture(|directory| {
        for (name, key, reason) in [
            ("hi_max.json", "\\udbff\\udfff", "invalid_mock_schema"),
            ("lo_min.json", "\\ud800\\udc00", "invalid_mock_schema"),
            (
                "hi_max_lo_min.json",
                "\\udbff\\udc00",
                "invalid_mock_schema",
            ),
            ("lone_hi_max.json", "\\udbff", "invalid_mock_json"),
            ("lone_lo_min.json", "\\udc00", "invalid_mock_json"),
            ("lone_lo_max.json", "\\udfff", "invalid_mock_json"),
        ] {
            let document = format!(
                "{{\"schema_version\":1,\"mock_read_records\":[],\"mock_write_i64\":[],\"mock_request\":[],\"mock_reply\":[],\"{key}\":[]}}"
            );
            fs::write(directory.join(name), document).unwrap();
            assert_eq!(
                run(directory, &["run", "p.n", "--mock-env", name]),
                usage(reason),
                "{name}"
            );
        }
    });
}
