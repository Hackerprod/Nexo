use std::{
    path::{Path, PathBuf},
    process::{Command, Output},
    sync::atomic::{AtomicUsize, Ordering},
};

static NEXT: AtomicUsize = AtomicUsize::new(0);

fn source_file(bytes: &[u8]) -> PathBuf {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("target")
        .join("test-fixtures")
        .join("cli");
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join(format!(
        "source-{}-{}.n",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    std::fs::write(&path, bytes).unwrap();
    path
}
fn invoke(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_nexo"))
        .args(args)
        .output()
        .unwrap()
}
fn checked(command: &str, bytes: &[u8]) -> Output {
    let path = source_file(bytes);
    let result = invoke(&[command, path.to_str().unwrap()]);
    std::fs::remove_file(path).unwrap();
    result
}
fn text(output: &[u8]) -> &str {
    std::str::from_utf8(output).unwrap()
}

#[test]
fn cli_check_and_run_use_official_b_pipeline() {
    let check = checked("check", b"fn F(){}");
    assert_eq!(check.status.code(), Some(0));
    assert_eq!(check.stdout, b"nexo-check:ok\n");
    assert!(check.stderr.is_empty());

    let missing_entry = checked("run", b"fn F(){}");
    assert_eq!(missing_entry.status.code(), Some(1));
    assert!(missing_entry.stdout.is_empty());
    assert!(text(&missing_entry.stderr).starts_with("NX-E0112:"));
    assert!(text(&missing_entry.stderr).contains("subcause: missing_main"));

    let run = checked("run", b"fn Main(){Print(1+2);}");
    assert_eq!(run.status.code(), Some(0));
    assert_eq!(run.stdout, b"3\n");
    assert_eq!(run.stderr, b"nexo-result:()\n");
}

#[test]
fn cli_check_and_run_ignore_source_extension() {
    let official = source_file(b"fn Main(){Print(7);}");
    let without_extension = official.with_extension("");
    std::fs::copy(&official, &without_extension).unwrap();

    for command in ["check", "run"] {
        let from_official = invoke(&[command, official.to_str().unwrap()]);
        let from_without_extension = invoke(&[command, without_extension.to_str().unwrap()]);
        assert_eq!(from_official.status.code(), Some(0), "{command}");
        assert_eq!(
            from_official.status.code(),
            from_without_extension.status.code()
        );
        assert_eq!(from_official.stdout, from_without_extension.stdout);
        assert_eq!(from_official.stderr, from_without_extension.stderr);
    }

    std::fs::remove_file(official).unwrap();
    std::fs::remove_file(without_extension).unwrap();
}

#[test]
fn cli_function_assignment_is_a_source_diagnostic() {
    for source in [
        "fn M(){M=();}",
        "fn Main(){Main=();}",
        "fn F(){} fn Main(){F=();}",
    ] {
        for command in ["check", "run"] {
            let output = checked(command, source.as_bytes());
            assert_eq!(output.status.code(), Some(1), "{command}: {source}");
            assert!(output.stdout.is_empty());
            let stderr = text(&output.stderr);
            assert!(stderr.starts_with("NX-E0006:"), "{stderr}");
            assert!(stderr.contains("feature_id: function_value"), "{stderr}");
            assert!(!stderr.contains("internal_error"), "{stderr}");
        }
    }
}

#[test]
fn cli_isolated_edit_uses_parse_diagnostic() {
    for source in [
        "fn Main(){var r=1;var b=edit r;}",
        "fn Main(){var r=1;r=edit r;}",
        "fn Main(){var r=1;[edit r];}",
        "fn Main(){var r=1;edit r;}",
    ] {
        for command in ["check", "run"] {
            let output = checked(command, source.as_bytes());
            assert_eq!(output.status.code(), Some(1), "{command}: {source}");
            assert!(output.stdout.is_empty());
            let stderr = text(&output.stderr);
            assert!(stderr.starts_with("NX-E0002:"), "{stderr}");
            assert!(
                stderr.contains("\"parser_context\":\"inout_place\""),
                "{stderr}"
            );
            assert!(!stderr.contains("internal_error"), "{stderr}");
        }
    }
}

#[test]
fn cli_intrinsic_arguments_reject_conflicting_later_access() {
    for source in [
        "fn Main(){var i=1;AssertEq(i,if true then {i=2;value:1;} else 1);}",
        "fn Main(){var a=[1];AssertEq(a[0],if true then {a[0]=2;value:1;} else 1);}",
        "fn Inc(x:edit I64){x=x+1;} fn Main(){var i=1;AssertEq(i,if true then {Inc(edit i);value:1;} else 1);}",
        "fn Inc(x:edit I64){x=x+1;} fn Main(){var a=[1];AssertEq(a[0],if true then {Inc(edit a[0]);value:1;} else 1);}",
        "fn Main(){var i=1;AssertEq(a:i,b:if true then {i=2;value:1;} else 1);}",
    ] {
        for command in ["check", "run"] {
            let output = checked(command, source.as_bytes());
            assert_eq!(output.status.code(), Some(1), "{command}: {source}");
            assert!(output.stdout.is_empty());
            let stderr = text(&output.stderr);
            assert!(stderr.starts_with("NX-E1105:"), "{stderr}");
            assert!(stderr.contains("active_borrow_conflict"), "{stderr}");
            assert!(!stderr.contains("internal_error"), "{stderr}");
        }
    }
}

#[test]
fn cli_index_consuming_its_owner_is_a_source_diagnostic() {
    for source in [
        "fn Main(){var b=[1,2];var c=b[if true then {let t=move b;value:0;} else 0];}",
        "fn Main(){var b=[1,2];Print(b[if false then {Drop(b);value:0;} else 0]);}",
        "fn F(p:move Array<I64>)->I64{return 0;} fn Main(){var b=[1,2];Print(b[F(move b)]);}",
        "fn Main(){var b=[1,2];AssertEq(b[if false then {let t=move b;value:0;} else 0],1);}",
        "fn Main(){var b=[1,2];var x=b[Len(move b)];}",
    ] {
        for command in ["check", "run"] {
            let output = checked(command, source.as_bytes());
            assert_eq!(
                output.status.code(),
                Some(1),
                "{command}: {source}: {output:?}"
            );
            assert!(
                text(&output.stderr).starts_with("NX-E0202:"),
                "{source}: {output:?}"
            );
            assert!(!text(&output.stderr).contains("internal_error"));
        }
    }
}

#[test]
fn cli_array_equality_releases_loans_before_write() {
    let source = b"fn Main(){var a=[1];Print(a==[1]);a[0]=2;Print(a[0]);}";
    for command in ["check", "run"] {
        let output = checked(command, source);
        assert_eq!(output.status.code(), Some(0), "{command}: {output:?}");
        if command == "check" {
            assert_eq!(output.stdout, b"nexo-check:ok\n");
            assert!(output.stderr.is_empty());
        } else {
            assert_eq!(output.stdout, b"true\n2\n");
            assert_eq!(output.stderr, b"nexo-result:()\n");
        }
    }
}

#[test]
fn cli_rejects_options_before_reading_any_source() {
    let missing = "target/test-fixtures/no-such-source.n";
    for args in [
        vec!["check", missing, "--profile", "x"],
        vec!["run", "--profile=x", missing],
        vec!["check", "--format", missing],
        vec!["run", missing, "--mock-env=x"],
        vec!["check", missing, "--profile=x", "--profile=y"],
        vec!["check", missing, "--profile"],
        vec!["check", "--profile"],
        vec!["run", "--format=text"],
        vec!["check", "--mock-env"],
        vec!["run", "--mock-env", missing],
        vec!["check", missing, "--format=text"],
        vec!["build", missing],
        vec!["check"],
        vec!["check", missing, "extra"],
    ] {
        let output = invoke(&args);
        assert_eq!(output.status.code(), Some(2), "{args:?}");
        assert!(output.stdout.is_empty(), "{args:?}");
        assert_eq!(
            output.stderr, b"nexo-driver:invalid_arguments\n",
            "{args:?}"
        );
    }
    let empty = invoke(&[]);
    assert_eq!(empty.status.code(), Some(2));
    assert_eq!(empty.stderr, b"nexo-driver:invalid_arguments\n");
}

#[test]
fn cli_reports_unreadable_and_invalid_utf8_before_parsing() {
    let missing = invoke(&["check", "target/test-fixtures/no-such-source.n"]);
    assert_eq!(missing.status.code(), Some(2));
    assert!(missing.stdout.is_empty());
    assert!(text(&missing.stderr).starts_with("NX-E0004:0:0:"));
    assert!(text(&missing.stderr).contains("subcause: source_unreadable"));
    assert!(text(&missing.stderr).contains("input_path"));

    let directory = env!("CARGO_MANIFEST_DIR");
    let unreadable = invoke(&["check", directory]);
    assert_eq!(unreadable.status.code(), Some(2));
    assert!(unreadable.stdout.is_empty());
    assert!(text(&unreadable.stderr).starts_with("NX-E0004:0:0:"));
    assert!(text(&unreadable.stderr).contains("subcause: source_unreadable"));

    for bytes in [
        b"a\xffb".as_slice(),
        b"a\xf0\x9f".as_slice(),
        b"a\xf0\x9f\x99".as_slice(),
        b"a\xe2".as_slice(),
        b"a\xf0\x9fb".as_slice(),
        b"a\xf0\x9f\x99b".as_slice(),
        b"a\xe2b".as_slice(),
        b"\xff".as_slice(),
        b"x\xc0\x80x".as_slice(),
        b"x\xe0\x80\x80x".as_slice(),
        b"x\xed\xa0\x80x".as_slice(),
        b"x\xf4\x90\x80\x80x".as_slice(),
        b"x\xf0\x80\x80\x80x".as_slice(),
        b"x\xf5\x80\x80\x80x".as_slice(),
    ] {
        let result = checked("check", bytes);
        assert_eq!(result.status.code(), Some(2), "{bytes:?}");
        assert!(result.stdout.is_empty());
        let rendered = text(&result.stderr);
        assert!(rendered.starts_with("NX-E0004:0:0:"), "{rendered}");
        assert!(rendered.contains("subcause: invalid_utf8"), "{rendered}");
        assert!(rendered.contains("invalid_bytes_hex"), "{rendered}");
        assert!(!rendered.contains("stage: parse"), "{rendered}");
    }
}

#[test]
fn cli_source_byte_boundary_precedes_utf8_decoding() {
    let mut exact = b"fn Main(){}//".to_vec();
    exact.resize(nexo::diagnostic::SOURCE_LIMIT, b' ');
    let result = checked("check", &exact);
    assert_eq!(result.status.code(), Some(0), "{}", text(&result.stderr));
    assert_eq!(result.stdout, b"nexo-check:ok\n");

    exact.push(0xff);
    let over = checked("check", &exact);
    assert_eq!(over.status.code(), Some(1));
    assert!(over.stdout.is_empty());
    let rendered = text(&over.stderr);
    assert!(rendered.starts_with("NX-E0003:0:0:"), "{rendered}");
    assert!(rendered.contains("source_bytes"), "{rendered}");
    assert!(rendered.contains("\"limit\":1048576"), "{rendered}");
    assert!(rendered.contains("\"observed\":1048577"), "{rendered}");

    for prefix in [
        b"\xe2".as_slice(),
        b"\xf0\x9f".as_slice(),
        b"\xf0\x9f\x99".as_slice(),
    ] {
        let mut bytes = b"fn Main(){}//".to_vec();
        bytes.resize(nexo::diagnostic::SOURCE_LIMIT - prefix.len(), b' ');
        bytes.extend_from_slice(prefix);
        let at_limit = checked("check", &bytes);
        assert_eq!(at_limit.status.code(), Some(2));
        assert!(text(&at_limit.stderr).contains("invalid_utf8"));
        bytes.push(b'x');
        let after_limit = checked("check", &bytes);
        assert_eq!(after_limit.status.code(), Some(1));
        assert!(text(&after_limit.stderr).contains("source_bytes"));
    }

    let mut boundary = b"fn Main(){}//".to_vec();
    boundary.resize(nexo::diagnostic::SOURCE_LIMIT - 1, b' ');
    boundary.push(0xff);
    let at_limit = checked("check", &boundary);
    assert_eq!(at_limit.status.code(), Some(2));
    assert!(text(&at_limit.stderr).contains("invalid_utf8"));
    boundary.push(b'x');
    let beyond_limit = checked("check", &boundary);
    assert_eq!(beyond_limit.status.code(), Some(1));
    assert!(text(&beyond_limit.stderr).contains("source_bytes"));
}

#[test]
fn cli_preserves_corpus_semantics_and_events() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("corpus");
    let transfer = root.join("pass/resource_transfer.n");
    let output = invoke(&["run", transfer.to_str().unwrap()]);
    assert_eq!(output.status.code(), Some(0));
    assert!(output.stdout.is_empty());
    assert_eq!(
        output.stderr,
        b"mock-resource:open:0:10\nmock-resource:inspect:0:10\nmock-resource:open:1:20\nmock-resource:inspect:1:20\nmock-resource:close:1:20\nmock-resource:close:0:10\nnexo-result:()\n"
    );

    for command in ["check", "run"] {
        let failure = root.join("fail/use_after_move.n");
        let output = invoke(&[command, failure.to_str().unwrap()]);
        assert_eq!(output.status.code(), Some(1));
        assert!(output.stdout.is_empty());
        let rendered = text(&output.stderr);
        assert!(rendered.starts_with("NX-E0202:"), "{rendered}");
        assert!(
            rendered.contains("subcause: definitely_consumed"),
            "{rendered}"
        );
    }
}

#[test]
fn cli_pending_capabilities_report_scope_and_source_site() {
    for (source, scope, first_line) in [
        (
            "fn Main()->Result<(),I64>{return Ok(());}",
            "I08",
            "NX-E0006:1:12: Capacidad pendiente de implementación en I08.\n",
        ),
        (
            "fn Main() with no_io{}",
            "I09",
            "NX-E0006:1:11: Capacidad pendiente de implementación en I09.\n",
        ),
        (
            "fn Main(){MockRequest();}",
            "I11a",
            "NX-E0006:1:11: Capacidad pendiente de implementación en I11a.\n",
        ),
        (
            "fn Main(){MockUnknown();}",
            "I09",
            "NX-E0006:1:11: Capacidad pendiente de implementación en I09.\n",
        ),
        (
            "fn Main(){let x=TryAdd(1,2);}",
            "I08",
            "NX-E0006:1:17: Capacidad pendiente de implementación en I08.\n",
        ),
    ] {
        let output = checked("run", source.as_bytes());
        assert_eq!(output.status.code(), Some(1), "{source}");
        assert!(output.stdout.is_empty());
        let rendered = text(&output.stderr);
        assert!(rendered.starts_with(first_line), "{source}: {rendered}");
        assert!(
            rendered.contains("feature_id: implementation_limit"),
            "{rendered}"
        );
        assert!(
            rendered.contains(&format!("\"pending_scope\":\"{scope}\"")),
            "{rendered}"
        );
    }
}
