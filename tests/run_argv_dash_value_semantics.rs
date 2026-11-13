use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
    sync::atomic::{AtomicUsize, Ordering},
};

static NEXT: AtomicUsize = AtomicUsize::new(0);

fn with_source(test: impl FnOnce(&Path, &Path)) {
    let directory = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("target")
        .join("test-fixtures")
        .join(format!(
            "run-option-values-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
    let _ = fs::remove_dir_all(&directory);
    fs::create_dir_all(&directory).unwrap();
    let source = directory.join("p.n");
    fs::write(&source, "fn Main(){}").unwrap();
    test(&directory, &source);
    fs::remove_dir_all(directory).unwrap();
}

fn run(directory: &Path, source: &Path, options: &[&str]) -> (Option<i32>, String, String) {
    let output = Command::new(env!("CARGO_BIN_EXE_nexo"))
        .current_dir(directory)
        .arg("run")
        .arg(source)
        .args(options)
        .output()
        .unwrap();
    (
        output.status.code(),
        String::from_utf8(output.stdout).unwrap(),
        String::from_utf8(output.stderr).unwrap(),
    )
}

#[test]
fn single_dash_mock_environment_values_are_paths() {
    with_source(|directory, source| {
        for value in ["-x", "-"] {
            assert_eq!(
                run(directory, source, &["--mock-env", value]),
                (
                    Some(2),
                    String::new(),
                    "nexo-driver:mock_env_unreadable\n".into()
                ),
                "mock environment value {value:?}"
            );
        }
        assert_eq!(
            run(
                directory,
                source,
                &["--mock-env", "-", "--format", "json"]
            ),
            (
                Some(2),
                "{\"schema_version\":1,\"profile_id\":\"f0-0002-b-v2\",\"outcome\":{\"kind\":\"driver_error\",\"reason\":\"mock_env_unreadable\"},\"stdout\":[],\"events\":[],\"observations\":[],\"observation_bytes\":0,\"reserved_observation_bytes\":0,\"diagnostics\":[]}\n".into(),
                String::new()
            )
        );
    });
}

#[test]
fn double_dash_mock_environment_value_is_invalid_arguments() {
    with_source(|directory, source| {
        assert_eq!(
            run(directory, source, &["--mock-env", "--x"]),
            (
                Some(2),
                String::new(),
                "nexo-driver:invalid_arguments\n".into()
            )
        );
    });
}

#[test]
fn embedded_double_dash_mock_environment_value_is_a_path() {
    with_source(|directory, source| {
        assert_eq!(
            run(
                directory,
                source,
                &["--mock-env", "missing--environment.json"]
            ),
            (
                Some(2),
                String::new(),
                "nexo-driver:mock_env_unreadable\n".into()
            )
        );
    });
}

#[test]
fn single_dash_profile_value_reaches_profile_selection() {
    with_source(|directory, source| {
        assert_eq!(
            run(directory, source, &["--profile", "-x"]),
            (
                Some(2),
                String::new(),
                "nexo-driver:unknown_profile\n".into()
            )
        );
    });
}

#[test]
fn embedded_double_dash_profile_value_reaches_profile_selection() {
    with_source(|directory, source| {
        assert_eq!(
            run(directory, source, &["--profile", "profile--unknown"]),
            (
                Some(2),
                String::new(),
                "nexo-driver:unknown_profile\n".into()
            )
        );
    });
}

#[test]
fn single_dash_format_value_is_invalid_arguments() {
    with_source(|directory, source| {
        assert_eq!(
            run(directory, source, &["--format", "-x"]),
            (
                Some(2),
                String::new(),
                "nexo-driver:invalid_arguments\n".into()
            )
        );
    });
}

#[test]
fn embedded_double_dash_format_value_is_invalid_arguments() {
    with_source(|directory, source| {
        assert_eq!(
            run(directory, source, &["--format", "text--json"]),
            (
                Some(2),
                String::new(),
                "nexo-driver:invalid_arguments\n".into()
            )
        );
    });
}
