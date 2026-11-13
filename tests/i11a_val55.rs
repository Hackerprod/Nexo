use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};

const EMPTY_ENV: &str = r#"{"schema_version":1,"mock_read_records":[],"mock_write_i64":[],"mock_request":[],"mock_reply":[]}"#;

struct CliCase {
    directory: PathBuf,
    source: PathBuf,
}

impl CliCase {
    fn new(name: &str, source: &str) -> Self {
        let directory =
            std::env::temp_dir().join(format!("nexo-i11a-val55-{}-{name}", std::process::id()));
        let _ = fs::remove_dir_all(&directory);
        fs::create_dir_all(&directory).unwrap();
        let source_path = directory.join("program.n");
        fs::write(&source_path, source).unwrap();
        Self {
            directory,
            source: source_path,
        }
    }

    fn environment(&self, json: &str) -> PathBuf {
        let path = self.directory.join("environment.json");
        fs::write(&path, json).unwrap();
        path
    }

    fn run_with(&self, mock_path: &Path) -> (i32, String, String) {
        let output = Command::new(env!("CARGO_BIN_EXE_nexo"))
            .arg("run")
            .arg(&self.source)
            .arg("--mock-env")
            .arg(mock_path)
            .output()
            .unwrap();
        (
            output.status.code().unwrap_or(-1),
            String::from_utf8(output.stdout).unwrap(),
            String::from_utf8(output.stderr).unwrap(),
        )
    }
}

impl Drop for CliCase {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.directory);
    }
}

fn env_with(queue: &str, body: &str) -> String {
    let queues = [
        ("mock_read_records", "[]"),
        ("mock_write_i64", "[]"),
        ("mock_request", "[]"),
        ("mock_reply", "[]"),
    ];
    let fields = queues
        .iter()
        .map(|(key, empty)| format!("\"{key}\":{}", if *key == queue { body } else { empty }))
        .collect::<Vec<_>>()
        .join(",");
    format!("{{\"schema_version\":1,{fields}}}")
}

#[test]
fn mock_read_preserves_record_order_and_contents_under_indexing() {
    let case = CliCase::new(
        "record-order",
        "fn Main()->Result<(),I64>{let a=MockReadRecords(MockOpen(1))?;Print(Len(a));Print(a[0]);Print(a[1]);Print(a[2]);return Ok(());}",
    );
    let environment = case.environment(&env_with(
        "mock_read_records",
        r#"[{"ok":[{"ok":"7"},{"ok":"8"},{"err":"9"}]}]"#,
    ));
    let (code, stdout, _) = case.run_with(&environment);
    assert_eq!(code, 0);
    assert_eq!(stdout, "3\nOk(7)\nOk(8)\nErr(9)\n");
}

#[test]
fn mock_reply_events_preserve_negative_status_sign_on_both_results() {
    let case = CliCase::new("reply-status", "fn Main(){MockReply(-404);MockReply(-7);}");
    let environment = case.environment(&env_with("mock_reply", r#"[{"ok":null},{"err":"3"}]"#));
    let (code, stdout, stderr) = case.run_with(&environment);
    assert_eq!(code, 0);
    assert!(stdout.is_empty());
    assert!(stderr.contains("mock-io:reply:-404:ok\n"), "{stderr}");
    assert!(stderr.contains("mock-io:reply:-7:err:3\n"), "{stderr}");
}

#[test]
fn mock_environment_integer_strings_reject_plus_sign() {
    let case = CliCase::new("integer-sign", "fn Main(){MockRequest();}");
    for text in ["+5", "+0"] {
        let environment = case.environment(&env_with(
            "mock_request",
            &format!(r#"[{{"ok":"{text}"}}]"#),
        ));
        let (code, stdout, stderr) = case.run_with(&environment);
        assert_eq!(code, 2, "{text}");
        assert!(stdout.is_empty(), "{text}");
        assert_eq!(stderr, "nexo-driver:invalid_mock_schema\n", "{text}");
    }
}

#[test]
fn mock_json_boolean_payloads_are_schema_errors() {
    let case = CliCase::new("boolean-literal", "fn Main(){MockWriteI64(1);}");
    for literal in ["false", "true"] {
        let environment = case.environment(&env_with(
            "mock_write_i64",
            &format!(r#"[{{"ok":{literal}}}]"#),
        ));
        let (code, stdout, stderr) = case.run_with(&environment);
        assert_eq!(code, 2, "{literal}");
        assert!(stdout.is_empty(), "{literal}");
        assert_eq!(stderr, "nexo-driver:invalid_mock_schema\n", "{literal}");
    }
}

#[test]
fn mock_environment_open_failures_report_unreadable_for_all_file_shapes() {
    let case = CliCase::new("unreadable", "fn Main(){}");
    let missing = case.directory.join("missing.json");
    let (code, stdout, stderr) = case.run_with(&missing);
    assert_eq!(code, 2);
    assert!(stdout.is_empty());
    assert_eq!(stderr, "nexo-driver:mock_env_unreadable\n");

    let directory = case.directory.join("directory.json");
    fs::create_dir(&directory).unwrap();
    let (code, stdout, stderr) = case.run_with(&directory);
    assert_eq!(code, 2);
    assert!(stdout.is_empty());
    assert_eq!(stderr, "nexo-driver:mock_env_unreadable\n");

    #[cfg(windows)]
    {
        use std::os::windows::fs::OpenOptionsExt;

        let blocked = case.environment(EMPTY_ENV);
        let _exclusive = std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .share_mode(0)
            .open(blocked)
            .unwrap();
        let (code, stdout, stderr) = case.run_with(&case.directory.join("environment.json"));
        assert_eq!(code, 2);
        assert!(stdout.is_empty());
        assert_eq!(stderr, "nexo-driver:mock_env_unreadable\n");
    }
}

#[test]
fn cli_size_limit_precedes_invalid_utf8_for_mock_environment() {
    let case = CliCase::new("size-before-utf8", "fn Main(){}");
    let path = case.directory.join("oversized.json");
    let mut bytes = vec![0xff];
    bytes.resize(1_048_577, b' ');
    fs::write(&path, bytes).unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_nexo"))
        .arg("run")
        .arg(&case.source)
        .arg("--mock-env")
        .arg(&path)
        .arg("--format")
        .arg("json")
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(2));
    assert!(output.stderr.is_empty());
    assert_eq!(
        String::from_utf8(output.stdout).unwrap(),
        concat!(
            "{\"schema_version\":1,\"profile_id\":\"f0-0002-b-v2\",",
            "\"outcome\":{\"kind\":\"driver_error\",\"reason\":\"mock_env_too_large\"},",
            "\"stdout\":[],\"events\":[],\"observations\":[],",
            "\"observation_bytes\":0,\"reserved_observation_bytes\":0,\"diagnostics\":[]}\n"
        )
    );
}
