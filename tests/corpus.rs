use std::{fs, path::Path};

use nexo::{
    diagnostic::{DataValue, Diagnostic},
    interp::{Execution, Outcome},
    ir::{CompileError, compile_scalar},
};

const FILE: &str = "corpus.n";

fn compile(source: &str) -> Result<nexo::ir::model::Program, CompileError> {
    compile_scalar(FILE, source)
}

fn byte_span(source: &str, needle: &str, occurrence: usize) -> (u64, u64) {
    let start = source
        .match_indices(needle)
        .nth(occurrence)
        .unwrap_or_else(|| panic!("missing occurrence {occurrence} of {needle:?}"))
        .0 as u64;
    (start, start + needle.len() as u64)
}
fn first_byte(span: (u64, u64)) -> (u64, u64) {
    (span.0, span.0 + 1)
}

fn run(source: &str) -> Execution {
    nexo::interp::run_scalar(&compile(source).expect("must statically check"))
}

#[allow(clippy::vec_box)] // Mirrors the pipeline's diagnostic collection.
fn rejected(source: &str) -> Vec<Box<Diagnostic>> {
    match compile(source) {
        Err(CompileError::Rejected(errors)) => errors,
        other => panic!("expected source rejection: {other:?}"),
    }
}

#[test]
fn corpus_has_only_official_source_extension() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("corpus");
    for (group, expected) in [("pass", 5), ("fail", 10)] {
        let mut count = 0;
        for entry in fs::read_dir(root.join(group)).unwrap() {
            let path = entry.unwrap().path();
            assert!(
                path.is_file(),
                "unexpected corpus entry: {}",
                path.display()
            );
            assert_ne!(
                path.extension(),
                Some(std::ffi::OsStr::new(concat!("n", "x"))),
                "obsolete corpus source: {}",
                path.display()
            );
            assert_eq!(
                path.extension(),
                Some(std::ffi::OsStr::new("n")),
                "unexpected corpus source extension: {}",
                path.display()
            );
            count += 1;
        }
        assert_eq!(count, expected, "corpus/{group}");
    }
}

#[test]
fn positive_corpus() {
    for entry in fs::read_dir(Path::new(env!("CARGO_MANIFEST_DIR")).join("corpus/pass")).unwrap() {
        let path = entry.unwrap().path();
        let source = fs::read_to_string(&path).unwrap();
        let program = compile_scalar(path.to_str().unwrap(), &source)
            .unwrap_or_else(|e| panic!("{}: {e:?}", path.display()));
        let result = nexo::interp::run_scalar(&program);
        assert_eq!(
            result.outcome,
            Outcome::Unit,
            "{}: {result:?}",
            path.display()
        );
        assert!(
            result.diagnostics.is_empty(),
            "{}: {result:?}",
            path.display()
        );
        let (stdout, events): (&[&str], &[&str]) = match path.file_name().unwrap().to_str().unwrap()
        {
            "integer_limits.n" => (&["25"], &[]),
            "nested_copy.n" => (&["[[1, 9], [7, 8]]"], &[]),
            "resource_transfer.n" => (
                &[],
                &[
                    "open:0:10",
                    "inspect:0:10",
                    "open:1:20",
                    "inspect:1:20",
                    "close:1:20",
                    "close:0:10",
                ],
            ),
            "return_cleanup.n" => (&[], &["open:0:1", "open:1:2", "close:1:2", "close:0:1"]),
            "shadow_scope.n" => (&[], &["open:0:7", "inspect:0:7", "close:0:7"]),
            other => panic!("unclassified pass corpus fixture: {other}"),
        };
        assert_eq!(result.observations.stdout(), stdout, "{}", path.display());
        assert_eq!(result.observations.events(), events, "{}", path.display());
    }
}

#[test]
fn negative_corpus_exact_codes() {
    for entry in fs::read_dir(Path::new(env!("CARGO_MANIFEST_DIR")).join("corpus/fail")).unwrap() {
        let path = entry.unwrap().path();
        let source = fs::read_to_string(&path).unwrap();
        let expected = source
            .lines()
            .next()
            .unwrap()
            .strip_prefix("// expect: ")
            .unwrap();
        let errors = match compile_scalar(path.to_str().unwrap(), &source) {
            Err(CompileError::Rejected(errors)) => errors,
            other => panic!("{}: {other:?}", path.display()),
        };
        assert_eq!(errors.len(), 1, "{}: {errors:?}", path.display());
        let error = &errors[0];
        let actual = match error.code.as_str() {
            "NX-E0006" => format!("{}/{}", error.code, error.feature_id.as_deref().unwrap()),
            _ if error.subcause.is_some() => {
                format!("{}/{}", error.code, error.subcause.as_deref().unwrap())
            }
            _ => error.code.clone(),
        };
        assert_eq!(actual, expected, "{}: {error:?}", path.display());
        let name = path.file_name().unwrap().to_str().unwrap();
        let span = error.primary_span.as_ref().unwrap();
        let observed = (span.start, span.end);
        let crlf = source.contains("\r\n");
        match name {
            "escape_scope.n" => assert_eq!(observed, if crlf { (66, 67) } else { (65, 66) }),
            "immutable_write.n" => assert_eq!(observed, if crlf { (52, 57) } else { (51, 56) }),
            "overflow_literal.n" => assert_eq!(observed, if crlf { (45, 64) } else { (44, 63) }),
            "unreachable_resource_copy.n" => {
                assert_eq!(observed, if crlf { (78, 79) } else { (77, 78) })
            }
            "mixed_array.n" => assert_eq!(observed, byte_span(&source, "true", 0)),
            "nested_resource.n" => assert_eq!(observed, byte_span(&source, "[MockOpen(1)]", 0)),
            "resource_copy.n" => assert_eq!(observed, first_byte(byte_span(&source, "r;", 0))),
            "trailing_garbage.n" => assert_eq!(observed, byte_span(&source, "ignored", 0)),
            "use_after_drop.n" => assert_eq!(observed, first_byte(byte_span(&source, "r);", 1))),
            "use_after_move.n" => assert_eq!(observed, first_byte(byte_span(&source, "r);", 0))),
            other => panic!("unclassified fail corpus fixture: {other}"),
        }
        error.validate(path.to_str().unwrap(), &source).unwrap();
    }
}

#[test]
fn nested_copy_output() {
    let result = run(include_str!("../corpus/pass/nested_copy.n"));
    assert_eq!(result.observations.stdout(), ["[[1, 9], [7, 8]]"]);
    assert!(result.observations.events().is_empty());
}

#[test]
fn transfer_has_one_owner_and_exact_cleanup() {
    let result = run(include_str!("../corpus/pass/resource_transfer.n"));
    assert_eq!(
        result.observations.events(),
        [
            "open:0:10",
            "inspect:0:10",
            "open:1:20",
            "inspect:1:20",
            "close:1:20",
            "close:0:10"
        ]
    );
}

#[test]
fn early_return_unwinds_scopes_in_reverse_order() {
    let result = run(include_str!("../corpus/pass/return_cleanup.n"));
    assert_eq!(
        result.observations.events(),
        ["open:0:1", "open:1:2", "close:1:2", "close:0:1"]
    );
    assert!(result.observations.stdout().is_empty());
}

#[test]
fn recoverable_failure_and_partial_initialization() {
    let result = run(
        "fn Main(){let a=MockOpen(1);{let b=MockOpen(2);let c=MockOpen(9223372036854775807+1);}}",
    );
    assert_eq!(result.diagnostics[0].code, "R0101");
    assert_eq!(
        result.observations.events(),
        ["open:0:1", "open:1:2", "close:1:2", "close:0:1"]
    );
}

#[test]
fn explicit_close_and_failure_do_not_double_close() {
    let result = run("fn Main(){let a=MockOpen(1);let b=MockOpen(2);Drop(a);Fail();}");
    assert_eq!(result.diagnostics[0].code, "R0301");
    assert_eq!(
        result.observations.events(),
        ["open:0:1", "open:1:2", "close:0:1", "close:1:2"]
    );
}

#[test]
fn indexing_and_overflow_are_checked_in_every_build() {
    for (expr, code) in [
        ("[1][-1]", "R0102"),
        ("[1][1]", "R0102"),
        ("9223372036854775807+1", "R0101"),
        ("-9223372036854775808+-1", "R0101"),
    ] {
        let result = run(&format!("fn Main(){{let r=MockOpen(1);Print({expr});}}"));
        assert_eq!(result.diagnostics[0].code, code);
        assert_eq!(result.observations.events(), ["open:0:1", "close:0:1"]);
    }
}

#[test]
fn assignment_bounds_preserve_cleanup() {
    let result = run("fn Main(){let r=MockOpen(1);var a=[[1]];a[0][1]=2;}");
    assert_eq!(result.diagnostics[0].code, "R0102");
    assert_eq!(result.observations.events(), ["open:0:1", "close:0:1"]);
}

#[test]
fn statements_after_return_still_checked() {
    let errors = rejected("fn Main(){return;let x=unknown;}");
    assert_eq!(errors[0].code, "NX-E0101");
}

#[test]
fn malformed_inputs_return_diagnostics() {
    for source in [
        "",
        "fn",
        "fn Main(",
        "fn Main(){",
        "fn Main(){let a=",
        "fn Main(){let a=-",
        "fn Main(){let a=[1,",
        "fn Main(){let a=1;} garbage",
        "fn Main(){Print(1.5);}",
    ] {
        assert!(
            matches!(compile(source), Err(CompileError::Rejected(_))),
            "{source:?}"
        );
    }
}

#[test]
fn diagnostic_unicode_comment_location() {
    let source = "// á🙂\nfn Main(){\n let a = missing;\n}";
    let errors = rejected(source);
    let rendered = errors[0].render_text(FILE, source).unwrap();
    assert!(rendered.starts_with("NX-E0101:3:10:"), "{rendered}");
}

#[test]
fn nesting_limit_is_diagnostic() {
    let source = format!(
        "fn Main(){{let x={}1{};}}",
        "[".repeat(200),
        "]".repeat(200)
    );
    let errors = rejected(&source);
    assert_eq!(errors[0].code, "NX-E0003");
}

#[test]
fn composed_ast_depth_is_bounded_before_recursive_passes() {
    let mut expr = "1".to_owned();
    for _ in 0..110 {
        expr = format!("({expr}+{})", vec!["0"; 128].join("+"));
    }
    let errors = rejected(&format!("fn Main(){{Print({expr});}}"));
    assert_eq!(errors[0].code, "NX-E0003");
}

#[test]
fn inferred_nesting_across_bindings_is_bounded() {
    let mut source = "fn Main(){let a0=1;".to_owned();
    for i in 1..=130 {
        source.push_str(&format!("let a{i}=[a{}];", i - 1));
    }
    source.push('}');
    let errors = rejected(&source);
    assert_eq!(errors[0].code, "NX-E0003");
    assert_eq!(errors[0].stage.as_str(), "types");
    assert_eq!(
        errors[0].data.get("budget_kind"),
        Some(&DataValue::String("array_type_depth".into()))
    );
    assert_eq!(errors[0].data.get("limit"), Some(&DataValue::UInt(128)));
    assert_eq!(errors[0].data.get("observed"), Some(&DataValue::UInt(129)));
    errors[0].validate(FILE, &source).unwrap();
}

#[test]
fn composed_expression_height_uses_parser_budget_instead_of_lowering_depth() {
    let compound = "(if true then 1 else 1)";
    let source =
        |count: usize, term: &str| format!("fn Main(){{let x={};}}", vec![term; count].join("+"));
    let accepted = source(127, compound);
    assert_eq!(accepted.len(), 3065);
    compile(&accepted).unwrap();
    let rejected_source = source(128, compound);
    assert_eq!(rejected_source.len(), 3089);
    let errors = rejected(&rejected_source);
    assert_eq!(errors.len(), 1);
    let error = &errors[0];
    assert_eq!(error.code, "NX-E0003");
    assert_eq!(error.stage.as_str(), "parse");
    assert_eq!(error.operation.as_str(), "analysis.limit");
    assert_eq!(
        error.data.get("budget_kind"),
        Some(&DataValue::String("expression_tree_depth".into()))
    );
    assert_eq!(error.data.get("limit"), Some(&DataValue::UInt(128)));
    assert_eq!(error.data.get("observed"), Some(&DataValue::UInt(129)));
    assert_eq!(
        error
            .primary_span
            .as_ref()
            .map(|span| (span.start, span.end)),
        Some((16, 3087))
    );
    error.validate(FILE, &rejected_source).unwrap();
    compile(&source(128, "1")).unwrap();
    let atomic = source(129, "1");
    let errors = rejected(&atomic);
    assert_eq!(errors[0].code, "NX-E0003");
    assert_eq!(
        errors[0].data.get("budget_kind"),
        Some(&DataValue::String("expression_tree_depth".into()))
    );
}

#[test]
fn invalid_assignment_root_and_deferred_selector_reject_before_lowering() {
    compile("fn Main(){var a=[1];a[0]=2;}").unwrap();

    let invalid_root = "fn Main(){(1)[0]=2;}";
    let errors = rejected(invalid_root);
    assert_eq!(errors.len(), 1);
    assert_eq!(errors[0].code, "NX-E0005");
    assert_eq!(errors[0].stage.as_str(), "parse");
    assert_eq!(
        errors[0].unexpected_token.as_ref().unwrap().text.as_deref(),
        Some("=")
    );
    errors[0].validate(FILE, invalid_root).unwrap();

    let selector = "fn Main(){var a=[1];a.foo=2;}";
    let errors = rejected(selector);
    assert_eq!(errors.len(), 1);
    assert_eq!(errors[0].code, "NX-E0006");
    assert_eq!(errors[0].feature_id.as_deref(), Some("methods"));
    errors[0].validate(FILE, selector).unwrap();
}

#[test]
fn pending_capabilities_have_exact_diagnostic_fields_and_valid_inverse() {
    for (source, scope, span) in [
        ("fn Main()->Result<(),I64>{return Ok(());}", "I08", (11, 25)),
        ("fn Main() with no_io{}", "I09", (10, 20)),
        ("fn Main(){MockRequest();}", "I11a", (10, 21)),
        ("fn Main(){MockReadRecords(MockOpen(1));}", "I11a", (10, 25)),
        ("fn Main(){MockWriteI64(1);}", "I11a", (10, 22)),
        ("fn Main(){MockReply(1);}", "I11a", (10, 19)),
        ("fn Main(){MockUnknown();}", "I09", (10, 21)),
    ] {
        let errors = rejected(source);
        assert_eq!(errors.len(), 1, "{source}");
        let error = &errors[0];
        assert_eq!(error.code, "NX-E0006", "{source}");
        assert_eq!(error.subcause, None);
        assert_eq!(error.rule, "NX-E0006");
        assert_eq!(error.explanation_id.as_deref(), Some("NX-E0006"));
        assert_eq!(error.stage.as_str(), "types");
        assert_eq!(error.operation.as_str(), "types.check");
        assert_eq!(error.feature_id.as_deref(), Some("implementation_limit"));
        assert_eq!(
            error.message,
            format!("Capacidad pendiente de implementación en {scope}.")
        );
        assert_eq!(
            error.primary_span.as_ref().map(|s| (s.start, s.end)),
            Some(span)
        );
        assert!(error.related_spans.is_empty());
        assert!(error.cause_chain.is_empty());
        assert!(error.repair.is_empty());
        assert!(error.expected_tokens.is_empty());
        assert!(error.unexpected_token.is_none());
        assert!(error.parser_context.is_none());
        assert!(error.recognized_prefix.is_none());
        assert_eq!(
            error.data,
            std::collections::BTreeMap::from([(
                "pending_scope".into(),
                DataValue::String(scope.into())
            )])
        );
        error.validate(FILE, source).unwrap();
        compile("fn Main(){}").unwrap();
    }
}

#[test]
fn pending_construction_precedes_later_type_error_in_compiler() {
    let source = "fn Main()->Result<(),I64>{let x=true+1;return Ok(());}";
    let errors = rejected(source);
    assert_eq!(errors.len(), 1);
    let error = &errors[0];
    assert_eq!(error.code, "NX-E0006");
    assert_eq!(error.feature_id.as_deref(), Some("implementation_limit"));
    assert_eq!(
        error.data.get("pending_scope"),
        Some(&DataValue::String("I08".into()))
    );
    assert_eq!(
        error.primary_span.as_ref().map(|s| (s.start, s.end)),
        Some((11, 25))
    );
    error.validate(FILE, source).unwrap();
    compile("fn Main(){let x=1+1;}").unwrap();
}

#[test]
fn pending_result_annotation_uses_type_span_even_after_user_function_named_result() {
    let source =
        "fn Result()->I64{return 1;}fn Main(){let x=Result();let y:Result<I64,I64>=Ok(1);}";
    let errors = rejected(source);
    assert_eq!(errors.len(), 1);
    let error = &errors[0];
    assert_eq!(error.code, "NX-E0006");
    assert_eq!(error.feature_id.as_deref(), Some("implementation_limit"));
    let type_start = source.find("Result<I64,I64>").unwrap() as u64;
    assert_eq!(
        error.primary_span.as_ref().map(|s| (s.start, s.end)),
        Some((type_start, type_start + "Result<I64,I64>".len() as u64))
    );
    error.validate(FILE, source).unwrap();
}

#[test]
fn pending_nested_result_parameter_uses_inner_written_type_span() {
    let source = "fn F(xs:view Array<Result<I64,I64>>){} fn Main(){}";
    let errors = rejected(source);
    assert_eq!(errors.len(), 1);
    let error = &errors[0];
    assert_eq!(error.code, "NX-E0006");
    assert_eq!(error.feature_id.as_deref(), Some("implementation_limit"));
    let type_start = source.find("Result<I64,I64>").unwrap() as u64;
    assert_eq!(
        error.primary_span.as_ref().map(|s| (s.start, s.end)),
        Some((type_start, type_start + "Result<I64,I64>".len() as u64))
    );
    error.validate(FILE, source).unwrap();
}

#[test]
fn pending_i08_expression_forms_reject_at_their_own_tokens() {
    for (source, span) in [
        ("fn Main(){let x=Ok(1);}", (16, 18)),
        ("fn Main(){let x=Err(1);}", (16, 19)),
        ("fn Main(){let x=1?;}", (17, 18)),
        ("fn Main(){let x=TryAdd(1,2);}", (16, 22)),
    ] {
        let errors = rejected(source);
        assert_eq!(errors.len(), 1, "{source}");
        let error = &errors[0];
        assert_eq!(error.code, "NX-E0006", "{source}");
        assert_eq!(error.feature_id.as_deref(), Some("implementation_limit"));
        assert_eq!(
            error.data.get("pending_scope"),
            Some(&DataValue::String("I08".into()))
        );
        assert_eq!(
            error.primary_span.as_ref().map(|s| (s.start, s.end)),
            Some(span),
            "{source}"
        );
        error.validate(FILE, source).unwrap();
    }
    compile("fn Main(){let x=1;}").unwrap();
}

#[test]
fn assignment_checks_negative_bounds_after_rhs() {
    for index in [-1, 1] {
        let result = run(&format!(
            "fn Main(){{let r=MockOpen(1);var a=[0];a[{index}]=9223372036854775807+1;}}"
        ));
        assert_eq!(result.diagnostics[0].code, "R0101");
        assert_eq!(result.observations.events(), ["open:0:1", "close:0:1"]);
    }
    let result = run("fn Main(){var a=[[0]];a[-1][9223372036854775807+1]=0;}");
    assert_eq!(result.diagnostics[0].code, "R0101");
}

#[test]
fn diagnostic_render_rejects_non_boundary_utf8_offsets() {
    let source = "// 🙂\nfn Main(){let a=missing;}";
    let errors = rejected(source);
    let diagnostic = &errors[0];
    assert!(
        diagnostic
            .render_text(FILE, source)
            .unwrap()
            .starts_with("NX-E0101:2:")
    );
    let mut malformed = (**diagnostic).clone();
    malformed.primary_span.as_mut().unwrap().start = 4; // inside the four-byte emoji
    assert!(malformed.validate(FILE, source).is_err());
    assert!(malformed.render_text(FILE, source).is_err());
}

#[test]
fn work_budget_exhaustion_closes_resources() {
    let mut source = "fn Main(){let r=MockOpen(7);let a0=[1];".to_owned();
    for i in 1..25 {
        source.push_str(&format!("let a{i}=[a{},a{}];", i - 1, i - 1));
    }
    source.push('}');
    let result = run(&source);
    assert_eq!(result.diagnostics[0].code, "R0002");
    assert_eq!(
        result.diagnostics[0].data.get("runtime_budget_kind"),
        Some(&DataValue::String("live_value_cells".into()))
    );
    assert_eq!(result.observations.events(), ["open:0:7", "close:0:7"]);
}

#[test]
fn runtime_work_limit_still_closes_resources() {
    let result = run("fn Main(){let r=MockOpen(7);while true{}}");
    assert_eq!(result.diagnostics[0].code, "R0001");
    assert_eq!(result.observations.events(), ["open:0:7", "close:0:7"]);
}

#[test]
fn truncated_and_mutated_inputs_never_panic() {
    let source = "fn Main(){let r=MockOpen(1);var a=[[1,2]];let b=a;a[0][1]=3;AssertEq(b,[[1,2]]);{let s=move r;Inspect(s);}Print(a);}";
    for n in 0..=source.len() {
        let _ = compile(&source[..n]);
    }
    for n in 0..source.len() {
        for replacement in ["", "🙂", "{", "-", "]", "return", "//", "0"] {
            let candidate = format!("{}{}{}", &source[..n], replacement, &source[n + 1..]);
            if let Err(CompileError::Rejected(errors)) = compile(&candidate) {
                for error in errors {
                    error.validate(FILE, &candidate).unwrap();
                }
            }
        }
    }
}
