use std::{path::PathBuf, process::Command};

fn corpus(path: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("corpus")
        .join(path)
}

#[test]
fn cli_separates_check_from_run_and_preserves_error_codes() {
    let exe = env!("CARGO_BIN_EXE_nexo");
    let check = Command::new(exe)
        .arg("check")
        .arg(corpus("pass/resource_transfer.nx"))
        .output()
        .unwrap();
    assert!(check.status.success());
    assert!(
        check.stderr.is_empty(),
        "check must not acquire simulated resources"
    );
    let run = Command::new(exe)
        .arg("run")
        .arg(corpus("pass/nested_copy.nx"))
        .output()
        .unwrap();
    assert!(run.status.success());
    assert_eq!(String::from_utf8(run.stdout).unwrap(), "[[1, 9], [7, 8]]\n");
    for command in ["check", "run"] {
        let bad = Command::new(exe)
            .arg(command)
            .arg(corpus("fail/use_after_move.nx"))
            .output()
            .unwrap();
        assert_eq!(bad.status.code(), Some(1));
        assert!(String::from_utf8(bad.stderr).unwrap().starts_with("E0202:"));
        assert!(bad.stdout.is_empty());
    }
}

#[test]
fn cli_invalid_invocations_do_not_execute() {
    let exe = env!("CARGO_BIN_EXE_nexo");
    let empty = Command::new(exe).output().unwrap();
    assert_eq!(empty.status.code(), Some(2));
    let unsupported = Command::new(exe)
        .arg("build")
        .arg(corpus("pass/resource_transfer.nx"))
        .output()
        .unwrap();
    assert_eq!(unsupported.status.code(), Some(2));
    let missing = Command::new(exe)
        .arg("run")
        .arg(corpus("missing-source.nx"))
        .output()
        .unwrap();
    assert_eq!(missing.status.code(), Some(2));
    assert!(
        String::from_utf8(missing.stderr)
            .unwrap()
            .starts_with("E0004:")
    );
}

#[test]
fn cli_size_limit_precedes_utf8_decoding_of_truncated_input() {
    let file = std::env::temp_dir().join(format!("nexo-cli-limit-{}.nx", std::process::id()));
    let mut source = b"fn main(){}//".to_vec();
    source.resize(1_048_576, b' ');
    source.extend_from_slice("🙂".as_bytes());
    std::fs::write(&file, &source).unwrap();
    let result = Command::new(env!("CARGO_BIN_EXE_nexo"))
        .arg("check")
        .arg(&file)
        .output()
        .unwrap();
    std::fs::remove_file(&file).unwrap();
    assert_eq!(result.status.code(), Some(1));
    assert!(
        String::from_utf8(result.stderr)
            .unwrap()
            .starts_with("E0003:")
    );
}
