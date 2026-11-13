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
fn parenthesized_indexed_places_run_with_expected_output() {
    let cases: [(&[u8], &[u8]); 6] = [
        (b"fn Main(){var a=[1]; (a[0]) = 2; Print(a);}", b"[2]\n"),
        (b"fn Main(){var a=[[1,2]]; (a[0])[1]=5;}", b""),
        (b"fn Main(){var a=[1,2]; ((a[0])) = 7;}", b""),
        (b"fn Main(){var a=[[1]]; (a[0][0]) = 2;}", b""),
        (b"fn Main(){var a=[1,2]; ((a)[0]) = 2;}", b""),
        (
            b"fn F(x: edit I64){x=3;} fn Main(){var a=[1]; F(edit (a[0]));}",
            b"",
        ),
    ];
    for (source, expected_stdout) in cases {
        let check = checked("check", source);
        assert_eq!(check.status.code(), Some(0), "{}", text(&check.stderr));
        assert_eq!(check.stdout, b"nexo-check:ok\n");
        assert!(check.stderr.is_empty());

        let run = checked("run", source);
        assert_eq!(run.status.code(), Some(0), "{}", text(&run.stderr));
        assert_eq!(run.stdout, expected_stdout);
        assert_eq!(run.stderr, b"nexo-result:()\n");
    }
}

#[test]
fn assert_eq_result_constructor_uses_first_operand_context() {
    for source in [
        b"fn Main(){let r:Result<I64,I64>=Ok(1); AssertEq(r,Ok(1));}".as_slice(),
        b"fn Main(){let r=TryAdd(1,2); AssertEq(r,Ok(3));}".as_slice(),
    ] {
        let check = checked("check", source);
        assert_eq!(check.status.code(), Some(0), "{}", text(&check.stderr));
        assert_eq!(check.stdout, b"nexo-check:ok\n");
        assert!(check.stderr.is_empty());

        let run = checked("run", source);
        assert_eq!(run.status.code(), Some(0), "{}", text(&run.stderr));
        assert!(run.stdout.is_empty());
        assert_eq!(run.stderr, b"nexo-result:()\n");
    }
}

#[test]
fn hypothetical_unresolved_index_write_has_no_internal_error() {
    // F0-0002 §4.1.2.3: without an Array constructor, the selector's
    // conditional I64 context is inactive in hypothetical code.
    let source = b"fn Main(){var q=(if true then {return;}else{return;});q[true]=1;}";
    for command in ["check", "run"] {
        let output = checked(command, source);
        assert_eq!(
            output.status.code(),
            Some(0),
            "{command}: {}",
            text(&output.stderr)
        );
        assert!(!text(&output.stderr).contains("internal_error"));
    }
    let non_array = b"fn Main(){var q=(if true then {return;}else{return;});let a=q[q<q];}";
    for command in ["check", "run"] {
        let output = checked(command, non_array);
        assert_eq!(
            output.status.code(),
            Some(1),
            "{command}: {}",
            text(&output.stderr)
        );
        assert!(text(&output.stderr).contains("subcause: not_indexable"));
        assert!(!text(&output.stderr).contains("internal_error"));
    }
}

#[test]
fn deferred_array_branch_conflict_rejects_before_driver_lowering() {
    // F0-0002 §4.1.2.4–5: the deferred element context reaches both
    // branches and reports their incompatible values before IR lowering.
    let source = b"fn Main(){var a1=[];var aar=[[1]];let a=[a1,[117],(if true then {value:aar;} else {value:false;})];}";
    for command in ["check", "run"] {
        let output = checked(command, source);
        let stderr = text(&output.stderr);
        assert_eq!(output.status.code(), Some(1), "{command}: {stderr}");
        assert_eq!(
            stderr.matches("NX-E0102:").count(),
            2,
            "{command}: {stderr}"
        );
        assert!(!stderr.contains("internal_error"), "{command}: {stderr}");
    }
}

#[test]
fn adr_gap_distributed_branch_consumes_outer_add_domain() {
    // F0-0002 §4.1.2.4–5 distributes I64 but does not identify whether
    // the enclosing + domain must also diagnose the already failed branch.
    let source = b"fn Main(){let p=true;let ar=[1];let x=1;let a=(if p then ar else x)+true;}";
    for command in ["check", "run"] {
        let output = checked(command, source);
        let stderr = text(&output.stderr);
        assert_eq!(output.status.code(), Some(1), "{command}: {stderr}");
        assert_eq!(
            stderr.matches("NX-E0102:").count(),
            1,
            "{command}: {stderr}"
        );
        assert!(stderr.contains("at: {\"end\":59,"), "{command}: {stderr}");
        assert!(stderr.contains("\"start\":57}"), "{command}: {stderr}");
        assert!(stderr.contains("\"expected_type\":\"I64\",\"found_type\":\"Array<I64>\""));
    }
}

#[test]
fn adr_gap_deferred_less_than_bool_uses_operator_primary() {
    // F0-0002 §4.1.2.4–5 leaves this operator versus operand attribution open.
    let source = b"fn Main(){var q=(if true then {return;}else{return;});let b=q<true;}";
    for command in ["check", "run"] {
        let output = checked(command, source);
        let stderr = text(&output.stderr);
        assert_eq!(output.status.code(), Some(1), "{command}: {stderr}");
        assert_eq!(
            stderr.matches("NX-E0102:").count(),
            1,
            "{command}: {stderr}"
        );
        assert!(stderr.contains("at: {\"end\":62,"), "{command}: {stderr}");
        assert!(stderr.contains("\"start\":61}"), "{command}: {stderr}");
        assert!(stderr.contains("\"expected_type\":\"I64\",\"found_type\":\"Bool\""));
    }
}

#[test]
fn adr_gap_direct_never_comparison_preserves_operator_primary() {
    // F0-0002 §§4.1.2.4–5 fix the I64 domain; attribution for an
    // intrinsically divergent left operand and Bool right operand is open.
    let source = b"fn Main(){let x=(if true then {return;} else {return;})<true;}";
    assert_eq!(&source[55..56], b"<");
    for command in ["check", "run"] {
        let output = checked(command, source);
        let stderr = text(&output.stderr);
        assert_eq!(output.status.code(), Some(1), "{command}: {stderr}");
        assert_eq!(
            stderr.matches("NX-E0102:").count(),
            1,
            "{command}: {stderr}"
        );
        assert!(stderr.contains("at: {\"end\":56,"), "{command}: {stderr}");
        assert!(stderr.contains("\"start\":55}"), "{command}: {stderr}");
        assert!(stderr.contains("stage: types"), "{command}: {stderr}");
        assert!(
            stderr.contains("operation: types.check"),
            "{command}: {stderr}"
        );
        assert!(
            stderr.contains("subcause: type_mismatch"),
            "{command}: {stderr}"
        );
        assert_eq!(stderr.matches("related:").count(), 1, "{command}: {stderr}");
        assert!(
            stderr.contains("\"role\":\"found_type\""),
            "{command}: {stderr}"
        );
        let related = stderr
            .lines()
            .find(|line| line.trim_start().starts_with("related: "))
            .unwrap();
        assert!(related.contains("\"end\":60,"), "{command}: {stderr}");
        assert!(related.contains("\"start\":56}}"), "{command}: {stderr}");
        assert!(stderr.contains("\"expected_type\":\"I64\",\"found_type\":\"Bool\""));
    }
    let valid = b"fn Main(){let x=(if true then {return;} else {return;})<1;}";
    for command in ["check", "run"] {
        let output = checked(command, valid);
        assert_eq!(
            output.status.code(),
            Some(0),
            "{command}: {}",
            text(&output.stderr)
        );
    }
}

#[test]
fn adr_gap_deferred_resource_array_uses_copyable_domain() {
    // F0-0002 §4.1.2.3–5 fixes the domain but leaves its late attribution open.
    let source = b"fn Main(){var q=(if true then {return;}else{return;});let a=[q];q=MockOpen(1);}";
    for command in ["check", "run"] {
        let output = checked(command, source);
        let stderr = text(&output.stderr);
        assert_eq!(output.status.code(), Some(1), "{command}: {stderr}");
        assert_eq!(
            stderr.matches("NX-E0102:").count(),
            1,
            "{command}: {stderr}"
        );
        assert!(stderr.contains("at: {\"end\":77,"), "{command}: {stderr}");
        assert!(stderr.contains("\"start\":66}"), "{command}: {stderr}");
        assert!(stderr.contains(
            "\"expected_type\":\"domain:copyable_data\",\"found_type\":\"MockResource\""
        ));
    }
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
fn cli_nested_function_values_reject_in_check_and_run() {
    for source in [
        "fn Main(){var v0=0;v0=(if false then Main else v0);v0=0;}",
        "fn Main(){var v=[1];v[Main]=1;}",
        "fn F(){} fn Main(){var v=0;v=(if true then F else v);}",
    ] {
        for command in ["check", "run"] {
            let output = checked(command, source.as_bytes());
            assert_eq!(output.status.code(), Some(1), "{command}: {source}");
            assert!(output.stdout.is_empty());
            let stderr = text(&output.stderr);
            assert!(stderr.starts_with("NX-E0006:"), "{source}: {stderr}");
            assert!(
                stderr.contains("feature_id: function_value"),
                "{source}: {stderr}"
            );
            assert!(!stderr.contains("internal_error"), "{source}: {stderr}");
        }
    }
}

#[test]
fn cli_type_conflicts_with_divergent_arrays_exit_as_source_errors() {
    let dv = "(if true then {return;}else{return;})";
    for source in [
        "fn Main(){let a=([1]==[0,true]);}".to_owned(),
        format!("fn Main(){{var v={dv};v=[false,{dv}];v=[4];}}"),
        format!("fn Main(){{var v=[false,{dv}];v=[4];}}"),
        format!("fn Main(){{var a=0;a=[0,{dv}];}}"),
        format!("fn Main(){{var v={dv};v=[4];v=[v[0],[v]];}}"),
    ] {
        for command in ["check", "run"] {
            let output = checked(command, source.as_bytes());
            assert_eq!(
                output.status.code(),
                Some(1),
                "{command}: {source}: {output:?}"
            );
            let stderr = text(&output.stderr);
            assert!(
                stderr.starts_with("NX-E01"),
                "{command}: {source}: {stderr}"
            );
            assert!(
                !stderr.contains("internal_error"),
                "{command}: {source}: {stderr}"
            );
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
    let unknown = invoke(&["check", missing, "--profile", "x"]);
    assert_eq!(unknown.status.code(), Some(2));
    assert!(unknown.stdout.is_empty());
    assert_eq!(unknown.stderr, b"nexo-driver:unknown_profile\n");
    let empty = invoke(&[]);
    assert_eq!(empty.status.code(), Some(2));
    assert_eq!(empty.stderr, b"nexo-driver:invalid_arguments\n");
}

#[test]
fn cli_unknown_profile_precedes_source_read_and_json_format_is_structured() {
    let missing = "target/test-fixtures/no-such-source.n";
    let unknown = invoke(&["check", missing, "--profile", "x"]);
    assert_eq!(unknown.status.code(), Some(2));
    assert!(unknown.stdout.is_empty());
    assert_eq!(unknown.stderr, b"nexo-driver:unknown_profile\n");

    let path = source_file(b"fn Main(){}");
    let checked = invoke(&["check", path.to_str().unwrap(), "--format", "json"]);
    std::fs::remove_file(path).unwrap();
    assert_eq!(checked.status.code(), Some(0));
    assert_eq!(checked.stderr, b"");
    assert!(text(&checked.stdout).contains(r#""outcome":{"kind":"checked"}"#));
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
fn cli_mock_primitive_is_supported_after_i11a() {
    let source = "fn Main(){MockRequest();}";
    let output = checked("run", source.as_bytes());
    assert_eq!(
        output.status.code(),
        Some(0),
        "{source}: {:?}",
        output.stderr
    );
    assert!(output.stdout.is_empty());
    let rendered = text(&output.stderr);
    assert_eq!(rendered, "mock-io:request:err:-9000\nnexo-result:()\n");
}

#[test]
fn cli_result_outcomes_keep_err_separate_from_runtime_faults() {
    let success = checked("run", b"fn Main()->Result<(),I64>{return Ok(());}");
    assert_eq!(success.status.code(), Some(0));
    assert!(success.stdout.is_empty());
    assert_eq!(text(&success.stderr), "nexo-result:Ok(())\n");

    let error = checked(
        "run",
        b"fn Main()->Result<(),I64>{let x=TryAdd(9223372036854775807,1)?;return Ok(());}",
    );
    assert_eq!(error.status.code(), Some(1));
    assert!(error.stdout.is_empty());
    assert_eq!(text(&error.stderr), "nexo-result:Err(30)\n");
}

#[test]
fn cli_json_preserves_err_types_channels_and_exit_status() {
    let source = b"fn Main()->Result<(),I64>{MockWriteI64(25)?;return Ok(());} ";
    let env = source_file(br#"{"schema_version":1,"mock_read_records":[],"mock_write_i64":[{"err":"40"}],"mock_request":[],"mock_reply":[]}"#);
    let src = source_file(source);
    let output = invoke(&[
        "run",
        "--format",
        "json",
        src.to_str().unwrap(),
        "--mock-env",
        env.to_str().unwrap(),
    ]);
    assert_eq!(output.status.code(), Some(1));
    assert!(output.stderr.is_empty());
    assert_eq!(
        text(&output.stdout),
        concat!(
            r#"{"schema_version":1,"profile_id":"f0-0002-b-v2","outcome":{"kind":"err","error":"40"},"stdout":[],"events":["write_i64:25:err:40"],"observations":[{"channel":"event","text":"write_i64:25:err:40"}],"observation_bytes":28,"reserved_observation_bytes":0,"diagnostics":[]}"#,
            "\n"
        )
    );
    std::fs::remove_file(src).unwrap();
    std::fs::remove_file(env).unwrap();

    let source = b"fn Main()->Result<(),I64>{MockWriteI64(25)?;return Ok(());} ";
    let env = source_file(br#"{"schema_version":1,"mock_read_records":[],"mock_write_i64":[{"ok":null}],"mock_request":[],"mock_reply":[]}"#);
    let src = source_file(source);
    let output = invoke(&[
        "run",
        "--format",
        "json",
        src.to_str().unwrap(),
        "--mock-env",
        env.to_str().unwrap(),
    ]);
    assert_eq!(output.status.code(), Some(0));
    assert_eq!(
        text(&output.stdout),
        concat!(
            r#"{"schema_version":1,"profile_id":"f0-0002-b-v2","outcome":{"kind":"ok"},"stdout":["25"],"events":["write_i64:25:ok"],"observations":[{"channel":"event","text":"write_i64:25:ok"},{"channel":"stdout","text":"25"}],"observation_bytes":27,"reserved_observation_bytes":0,"diagnostics":[]}"#,
            "\n"
        )
    );
    std::fs::remove_file(src).unwrap();
    std::fs::remove_file(env).unwrap();
}

#[test]
fn cli_mock_read_temporary_has_exact_ok_and_err_json() {
    // A2 §12.4, «Esquema del entorno», l. 1501, «Eventos exactos», ll. 1544–1555.
    let source = b"fn Main()->Result<(),I64>{let a=MockReadRecords(MockOpen(1))?;return Ok(());} ";
    let src = source_file(source);
    let check = invoke(&["check", src.to_str().unwrap()]);
    assert_eq!(check.status.code(), Some(0));
    assert_eq!(check.stdout, b"nexo-check:ok\n");
    assert!(check.stderr.is_empty());

    let values = [
        (
            br#"{"schema_version":1,"mock_read_records":[{"ok":[]}],"mock_write_i64":[],"mock_request":[],"mock_reply":[]}"#.as_slice(),
            0,
            r#"{"schema_version":1,"profile_id":"f0-0002-b-v2","outcome":{"kind":"ok"},"stdout":[],"events":["open:0:1","read_records:0:ok:0","close:0:1"],"observations":[{"channel":"event","text":"open:0:1"},{"channel":"event","text":"read_records:0:ok:0"},{"channel":"event","text":"close:0:1"}],"observation_bytes":75,"reserved_observation_bytes":0,"diagnostics":[]}"#,
        ),
        (
            br#"{"schema_version":1,"mock_read_records":[{"err":"10"}],"mock_write_i64":[],"mock_request":[],"mock_reply":[]}"#,
            1,
            r#"{"schema_version":1,"profile_id":"f0-0002-b-v2","outcome":{"kind":"err","error":"10"},"stdout":[],"events":["open:0:1","read_records:0:err:10","close:0:1"],"observations":[{"channel":"event","text":"open:0:1"},{"channel":"event","text":"read_records:0:err:10"},{"channel":"event","text":"close:0:1"}],"observation_bytes":77,"reserved_observation_bytes":0,"diagnostics":[]}"#,
        ),
    ];
    for (environment, code, expected) in values {
        let env = source_file(environment);
        let output = invoke(&[
            "run",
            "--format",
            "json",
            src.to_str().unwrap(),
            "--mock-env",
            env.to_str().unwrap(),
        ]);
        assert_eq!(output.status.code(), Some(code));
        assert!(output.stderr.is_empty());
        assert_eq!(text(&output.stdout), format!("{expected}\n"));
        std::fs::remove_file(env).unwrap();
    }
    std::fs::remove_file(src).unwrap();
}

#[test]
fn cli_mock_environment_option_is_run_only_and_reports_exact_schema_reason() {
    let src = source_file(b"fn Main(){}");
    let env = source_file(br#"{"schema_version":1,"mock_read_records":[],"mock_write_i64":[],"mock_request":[],"mock_reply":[]}"#);
    let output = invoke(&[
        "check",
        src.to_str().unwrap(),
        "--mock-env",
        env.to_str().unwrap(),
    ]);
    assert_eq!(output.status.code(), Some(2));
    assert!(output.stdout.is_empty());
    assert_eq!(output.stderr, b"nexo-driver:invalid_arguments\n");

    let invalids: [&[u8]; 4] = [
        br#"{"schema_version":1,"mock_read_records":[],"mock_write_i64":[],"mock_request":[{"ok":"01"}],"mock_reply":[]}"#,
        br#"{"schema_version":1,"mock_read_records":[],"mock_write_i64":[],"mock_request":[{"ok":"-01"}],"mock_reply":[]}"#,
        br#"{"schema_version":1.0,"mock_read_records":[],"mock_write_i64":[],"mock_request":[],"mock_reply":[]}"#,
        br#"{"schema_version":1,"mock_read_records":[],"mock_write_i64":[{"ok":true}],"mock_request":[],"mock_reply":[]}"#,
    ];
    for invalid in invalids {
        std::fs::write(&env, invalid).unwrap();
        let output = invoke(&[
            "run",
            "--format",
            "json",
            src.to_str().unwrap(),
            "--mock-env",
            env.to_str().unwrap(),
        ]);
        assert_eq!(output.status.code(), Some(2));
        assert_eq!(
            text(&output.stdout),
            concat!(
                r#"{"schema_version":1,"profile_id":"f0-0002-b-v2","outcome":{"kind":"driver_error","reason":"invalid_mock_schema"},"stdout":[],"events":[],"observations":[],"observation_bytes":0,"reserved_observation_bytes":0,"diagnostics":[]}"#,
                "\n"
            )
        );
    }
    std::fs::remove_file(src).unwrap();
    std::fs::remove_file(env).unwrap();
}

#[test]
fn cli_rejects_repeated_mock_environment_option() {
    // A2 §12.4, «Invocación y selección de perfil», ll. 1488–1497.
    let src = source_file(b"fn Main(){}");
    let env = source_file(
        br#"{"schema_version":1,"mock_read_records":[],"mock_write_i64":[],"mock_request":[],"mock_reply":[]}"#,
    );
    let output = invoke(&[
        "run",
        src.to_str().unwrap(),
        "--mock-env",
        env.to_str().unwrap(),
        "--mock-env",
        env.to_str().unwrap(),
    ]);
    assert_eq!(output.status.code(), Some(2));
    assert!(output.stdout.is_empty());
    assert_eq!(output.stderr, b"nexo-driver:invalid_arguments\n");
    std::fs::remove_file(src).unwrap();
    std::fs::remove_file(env).unwrap();
}

#[test]
fn cli_reports_mock_environment_size_limit_reason() {
    let src = source_file(b"fn Main(){}");
    let env = source_file(b"{}");
    let mut oversized = br#"{"schema_version":1,"mock_read_records":[],"mock_write_i64":[],"mock_request":[],"mock_reply":[]}"#.to_vec();
    oversized.resize(1_048_577, b' ');
    std::fs::write(&env, oversized).unwrap();
    let output = invoke(&[
        "run",
        "--format",
        "json",
        src.to_str().unwrap(),
        "--mock-env",
        env.to_str().unwrap(),
    ]);
    assert_eq!(output.status.code(), Some(2));
    assert!(text(&output.stdout).contains(r#""reason":"mock_env_too_large""#));
    std::fs::remove_file(src).unwrap();
    std::fs::remove_file(env).unwrap();
}

#[test]
fn cli_request_requires_io_effect_and_reply_effect_name_is_specific() {
    for (source, expected) in [
        (
            "fn Main() with no_io {let x=MockRequest();}",
            "prelude:mock_request",
        ),
        (
            "fn F() with no_io {MockReply(200);} fn Main(){F();}",
            "prelude:mock_reply",
        ),
    ] {
        let nexo::ir::CompileError::Rejected(errors) =
            nexo::ir::compile_scalar("effects.n", source).unwrap_err()
        else {
            panic!("expected effect diagnostic")
        };
        assert!(
            errors.iter().any(|diagnostic| {
                diagnostic.code == "NX-E0415"
                    && diagnostic
                        .cause_chain
                        .first()
                        .and_then(|cause| cause.entity.as_ref())
                        .is_some_and(|entity| entity.id == expected)
            }),
            "{source}: {errors:?}"
        );
    }
}

#[test]
fn cli_mock_write_requires_io_effect() {
    // D15 §2.3, l. 97: MockWriteI64 declara el efecto io.
    let source = "fn Main() with no_io{MockWriteI64(1);}";
    let nexo::ir::CompileError::Rejected(errors) =
        nexo::ir::compile_scalar("write-effect.n", source).unwrap_err()
    else {
        panic!("expected effect diagnostic")
    };
    assert_eq!(errors.len(), 1, "{errors:?}");
    let error = &errors[0];
    assert_eq!(error.code, "NX-E0415");
    assert_eq!(error.subcause, None);
    assert_eq!(error.stage.as_str(), "effects");
    assert_eq!(error.operation.as_str(), "intrinsic.call");
    assert_eq!(error.message, "La función Main infringe no_io.");
    assert_eq!(
        error
            .primary_span
            .as_ref()
            .map(|span| (span.start, span.end)),
        Some((21, 33))
    );
    assert_eq!(
        error.data,
        [
            (
                "contract_origin".into(),
                nexo::diagnostic::DataValue::Object(
                    [
                        ("end".into(), nexo::diagnostic::DataValue::UInt(20)),
                        (
                            "file".into(),
                            nexo::diagnostic::DataValue::String("write-effect.n".into()),
                        ),
                        ("start".into(), nexo::diagnostic::DataValue::UInt(15)),
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
        .into()
    );
    assert!(error.unexpected_token.is_none());
    assert!(error.expected_tokens.is_empty());
    assert!(error.feature_id.is_none());
    assert_eq!(error.related_spans.len(), 1);
    assert_eq!(
        error.related_spans[0].role,
        nexo::diagnostic::RelatedRole::Contract
    );
    assert_eq!(error.related_spans[0].span.file, "write-effect.n");
    assert_eq!(
        (
            error.related_spans[0].span.start,
            error.related_spans[0].span.end
        ),
        (15, 20)
    );
    assert_eq!(error.cause_chain.len(), 1);
    let cause = &error.cause_chain[0];
    assert_eq!(cause.operation.as_str(), "intrinsic.call");
    assert_eq!(cause.evidence, nexo::diagnostic::Evidence::Verified);
    assert_eq!(
        cause.span.as_ref().map(|span| (span.start, span.end)),
        Some((21, 33))
    );
    let entity = cause.entity.as_ref().unwrap();
    assert_eq!(entity.kind, nexo::diagnostic::EntityKind::Intrinsic);
    assert_eq!(entity.id, "prelude:mock_write_i64");
    error.validate("write-effect.n", source).unwrap();
}
