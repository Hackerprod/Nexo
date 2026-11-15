use std::{
    fs,
    process::Command,
    sync::atomic::{AtomicUsize, Ordering},
};

static SOURCE_ID: AtomicUsize = AtomicUsize::new(0);

fn run(command: &str, source: &str) -> (i32, String) {
    let id = SOURCE_ID.fetch_add(1, Ordering::Relaxed);
    let path = std::env::temp_dir().join(format!("nexo-j14-publish-{}-{id}.n", std::process::id()));
    fs::write(&path, source).unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_nexo"))
        .args([
            command,
            path.to_str().unwrap(),
            "--profile",
            "f0-0003-b-v1",
            "--format",
            "json",
        ])
        .output()
        .unwrap();
    fs::remove_file(path).unwrap();
    (
        output.status.code().unwrap(),
        String::from_utf8(output.stdout).unwrap(),
    )
}

#[test]
fn all_diagnostic_emitters_use_canonical_publication() {
    for (source, code) in [
        ("fn Main(){let x=;}", "NX-E0002"),
        ("fn Main(){Foo();}", "NX-E0101"),
        ("fn Main(){let x:I64=true;}", "NX-E0102"),
        (
            "fn Main(){let r=MockOpen(1);Drop(move r);Inspect(r);}",
            "NX-E0202",
        ),
        ("fn Main() with no_io{Print(1);}", "NX-E0415"),
        (
            "gen fn G()->I64{var x=0;var f=fn [edit x](){x=1;};let o=Some(move f);yield 1;}fn Main(){}",
            "NX-E1204",
        ),
    ] {
        for command in ["check", "run"] {
            let (status, output) = run(command, source);
            assert_eq!(status, 1, "{command}: {output}");
            assert!(output.contains(&format!("\"code\":\"{code}\"")), "{output}");
            assert!(
                !output.contains("\"diagnostics\":[null") && !output.contains("internal_error"),
                "{output}"
            );
        }
    }
    let (status, output) = run("run", "fn Main(){Fail();}");
    assert_eq!(status, 1, "{output}");
    assert!(output.contains("\"code\":\"R0301\""), "{output}");
    assert!(!output.contains("\"diagnostics\":[null"), "{output}");

    let id = SOURCE_ID.fetch_add(1, Ordering::Relaxed);
    let absent =
        std::env::temp_dir().join(format!("nexo-j14-absent-{}-{id}.n", std::process::id()));
    let input = Command::new(env!("CARGO_BIN_EXE_nexo"))
        .args([
            "check",
            absent.to_str().unwrap(),
            "--profile",
            "f0-0003-b-v1",
            "--format",
            "json",
        ])
        .output()
        .unwrap();
    let output = String::from_utf8(input.stdout).unwrap();
    assert_eq!(input.status.code(), Some(2), "{output}");
    assert!(output.contains("\"code\":\"NX-E0004\""), "{output}");
    assert!(!output.contains("\"diagnostics\":[null"), "{output}");

    for args in [
        vec!["explain", "NX-E0113", "--format", "json"],
        vec!["check", "--format", "json"],
    ] {
        let output = Command::new(env!("CARGO_BIN_EXE_nexo"))
            .args(args)
            .output()
            .unwrap();
        let text = String::from_utf8(output.stdout).unwrap();
        assert!(!text.contains("\"diagnostics\":[null"), "{text}");
    }
}
