use super::{CompileError, compile_scalar};
use crate::frontend::parse;
use crate::interp::{Outcome, run_scalar};

#[test]
fn nested_array_equality_reports_the_outermost_live_loan() {
    let source = "fn Main(){var a=[1];let x=a == (if a == (if true then {a[0]=2;value:[1];} else [1]) then a else a);}";
    let Err(CompileError::Rejected(errors)) = compile_scalar("loan-order.n", source) else {
        panic!("the nested write must conflict with the outer read loan");
    };
    let conflict = errors
        .iter()
        .find(|error| error.code == "NX-E1105")
        .unwrap();
    assert_eq!(conflict.subcause.as_deref(), Some("active_borrow_conflict"));
    // Inserting the nested equality loan at position zero would make first()
    // report its later origin instead of the already live outer loan.
    let origin = conflict
        .related_spans
        .iter()
        .find(|span| span.role.as_str() == "origin")
        .unwrap();
    assert_eq!((origin.span.start, origin.span.end), (26, 27));
    assert_eq!(
        conflict.cause_chain[0]
            .span
            .as_ref()
            .map(|s| (s.start, s.end)),
        Some((26, 27))
    );
    conflict.validate("loan-order.n", source).unwrap();
}

#[test]
fn isolated_edit_never_reaches_lowering() {
    for body in [
        "var b=edit r;",
        "r=edit r;",
        "[edit r];",
        "edit r;",
        "Print(1+edit r);",
    ] {
        let source = format!("fn Main(){{var r=1;{body}}}");
        let Err(CompileError::Rejected(errors)) = compile_scalar("edit.n", &source) else {
            panic!("source must be rejected before lowering: {source}");
        };
        assert_eq!(errors[0].code, "NX-E0002", "{source}");
        assert_eq!(errors[0].parser_context.as_deref(), Some("inout_place"));
        errors[0].validate("edit.n", &source).unwrap();
    }
    compile_scalar("edit.n", "fn F(x:edit I64){} fn Main(){var r=1;F(edit r);}").unwrap();
}

#[test]
fn forged_exclusive_expression_hits_only_the_ir_guard() {
    use crate::frontend::parse::{Expr, Mode, Statement};
    let source = "fn Main(){var r=1;let b=move r;}";
    let mut ast = parse::parse("guard.n", source).unwrap();
    let Statement::Let {
        value: Expr::Unary { mode, .. },
        ..
    } = &mut ast.functions[0].body.statements[1]
    else {
        panic!("expected transfer")
    };
    *mode = Mode::Inout;
    assert!(matches!(
        super::lower::compile_ast(&ast, source),
        Err(CompileError::InvalidIr(
            "exclusive loans or result propagation"
        ))
    ));
    let rejected = compile_scalar("guard.n", "fn Main(){var r=1;let b=edit r;}");
    assert!(matches!(rejected, Err(CompileError::Rejected(_))));
}

#[test]
#[ignore = "directed prefix and expression-context campaign"]
fn access_prefix_contexts_never_escape_as_invalid_ir() {
    let contexts = [
        "var b={p};",
        "r={p};",
        "[{p}];",
        "{p};",
        "var b=1+{p};",
        "var b=({p})+1;",
        "var b=[1][{p}];",
        "var b=({p}).field;",
        "return {p};",
        "if {p} {}",
        "F({p});",
        "F(1+{p});",
        "{{{p};}}",
        "var b=if true then {p} else 1;",
        "var b=({p})?;",
        "F(x: {p});",
    ];
    let prefixes = ["edit r", "move r", "view r"];
    for case in 0..20_160 {
        let context = contexts[(case / 3) % contexts.len()];
        let prefix = prefixes[case % prefixes.len()];
        let fragment = context.replace("{p}", prefix);
        let separator = ["", " ", "\n", "// gap\n"][case / 48 % 4];
        let source = format!(
            "fn F(x:edit I64){{}} fn G()->I64{{var r=1;let pad={};{separator}{fragment}return pad;}} fn Main(){{Print(G());}}",
            case % 997
        );
        match compile_scalar("access.n", &source) {
            Ok(program) => {
                let execution = run_scalar(&program);
                assert!(
                    !matches!(execution.outcome, Outcome::InternalError(_)),
                    "{source}"
                );
                for error in execution.diagnostics {
                    error.validate("access.n", &source).unwrap();
                }
            }
            Err(CompileError::Rejected(errors)) => {
                for error in errors {
                    error.validate("access.n", &source).unwrap();
                }
            }
            Err(CompileError::InvalidIr(reason)) => panic!("{source}: {reason}"),
            Err(CompileError::Dependent) => panic!("dependent IR: {source}"),
        }
    }
}

#[test]
#[ignore = "Unicode source and lowering campaign"]
fn unicode_sources_never_reach_internal_failure() {
    let templates = [
        "fn Main(){var r=1;edit r;}",
        "fn Main(){let a=[1,2];Print(a[0]);}",
        "fn F(x:edit I64){} fn Main(){var r=1;F(edit r);}",
        "fn Main(){let x=if true then 1 else 2;Print(x);}",
        "fn Main(){let r=MockOpen(1);let s=move r;Drop(s);}",
    ];
    let insertions = [
        "é", "中", "🙂", "\u{2028}", "\u{0085}", "\u{000c}", "//語\n",
    ];
    let mut state = 0x6e65_786f_6564_6974_u64;
    for case in 0..100_000 {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        let base = templates[case % templates.len()];
        let at = (state as usize) % (base.len() + 1);
        let insertion = insertions[(state >> 32) as usize % insertions.len()];
        let mut source = String::new();
        source.push_str(&base[..at]);
        source.push_str(insertion);
        source.push_str(&base[at..]);
        match compile_scalar("unicode.n", &source) {
            Ok(program) => {
                let execution = run_scalar(&program);
                assert!(
                    !matches!(execution.outcome, Outcome::InternalError(_)),
                    "{source}"
                );
                for error in execution.diagnostics {
                    error.validate("unicode.n", &source).unwrap();
                }
            }
            Err(CompileError::Rejected(errors)) => {
                for error in errors {
                    error.validate("unicode.n", &source).unwrap();
                }
            }
            Err(CompileError::InvalidIr(reason)) => panic!("{source}: {reason}"),
            Err(CompileError::Dependent) => panic!("dependent IR: {source}"),
        }
    }
}

#[test]
fn function_assignment_rejects_before_lowering() {
    for source in [
        "fn M(){M=();}",
        "fn Main(){Main=();}",
        "fn F(){} fn Main(){F=();}",
    ] {
        let Err(CompileError::Rejected(errors)) = compile_scalar("fixture.n", source) else {
            panic!("expected published diagnostic for {source}")
        };
        assert_eq!(errors[0].code, "NX-E0006");
        assert_eq!(errors[0].feature_id.as_deref(), Some("function_value"));
        errors[0].validate("fixture.n", source).unwrap();
    }
}

#[test]
#[ignore = "directed rejected-place campaign"]
fn function_and_other_nonlocal_assignment_fuzz_has_no_internal_error() {
    let targets = ["F", "(F)", "((F))", "F[0]", "Main", "(Main)"];
    let values = ["()", "0", "true", "[0]", "if true then 1 else 2"];
    for n in 0..20_000 {
        let assignment = format!(
            "{}={};",
            targets[n % targets.len()],
            values[(n / 7) % values.len()]
        );
        let body = match (n / 31) % 5 {
            0 => assignment,
            1 => format!("{{{assignment}}}"),
            2 => format!("if true {{{assignment}}}"),
            3 => format!("while false {{{assignment}}}"),
            _ => format!("let x=0;{assignment}"),
        };
        let source = format!("fn F(){{}} fn Main(){{{body}}}");
        match compile_scalar("nonlocal-place.n", &source) {
            Err(CompileError::Rejected(errors)) => {
                assert!(!errors.is_empty(), "{source}");
                for error in errors {
                    error.validate("nonlocal-place.n", &source).unwrap();
                }
            }
            Err(CompileError::InvalidIr(error)) => panic!("{source}: {error}"),
            Err(CompileError::Dependent) => panic!("{source}: dependent IR"),
            Ok(_) => panic!("accepted assignment to function: {source}"),
        }
    }
    for declaration in [
        "type T=I64; fn Main(){T=();}",
        "const C:I64=comptime 1; fn Main(){C=();}",
        "mod m; fn Main(){m=();}",
        "use m; fn Main(){m=();}",
    ] {
        assert!(
            matches!(
                compile_scalar("nonlocal-place.n", declaration),
                Err(CompileError::Rejected(_))
            ),
            "{declaration}"
        );
    }
}

fn execute(source: &str) -> crate::interp::Execution {
    let program = compile_scalar("fixture.n", source).unwrap();
    let result = run_scalar(&program);
    for d in &result.diagnostics {
        d.validate("fixture.n", source).unwrap();
    }
    result
}

#[test]
fn indexed_place_rechecks_owner_after_index_evaluation() {
    let prefix = "fn F(p:move Array<I64>)->I64{return 0;} fn Main(){var b=[1,2];";
    for (body, subcause) in [
        (
            "var c=b[if true then {let t=move b;value:0;} else 0];",
            "maybe_consumed",
        ),
        (
            "Print(b[if false then {Drop(b);value:0;} else 0]);",
            "maybe_consumed",
        ),
        ("Print(b[F(move b)]);", "definitely_consumed"),
        (
            "AssertEq(b[if false then {let t=move b;value:0;} else 0],1);",
            "maybe_consumed",
        ),
        ("var x=b[Len(move b)];", "definitely_consumed"),
        ("b[F(move b)]=3;", "definitely_consumed"),
    ] {
        let source = format!("{prefix}{body}}}");
        let Err(CompileError::Rejected(errors)) = compile_scalar("index-owner.n", &source) else {
            panic!("expected source ownership diagnostic: {source}");
        };
        let error = errors
            .iter()
            .find(|error| error.code == "NX-E0202")
            .unwrap_or_else(|| panic!("{source}: {errors:?}"));
        assert_eq!(error.subcause.as_deref(), Some(subcause), "{source}");
        let root = source.rfind("b[").unwrap() as u64;
        assert_eq!(
            error
                .primary_span
                .as_ref()
                .map(|span| (span.start, span.end)),
            Some((root, root + 1)),
            "{source}"
        );
        for error in errors {
            error.validate("index-owner.n", &source).unwrap();
        }
    }
}

#[test]
fn array_equality_releases_operand_loans_before_later_access() {
    for source in [
        "fn Main(){var a=[1];Print(a==[1]);a[0]=2;Print(a[0]);}",
        "fn Main(){var a=[1];Print(a==[1]);let b=move a;Print(b[0]);}",
        "fn B(x:edit Array<I64>){x[0]=2;} fn Main(){var a=[1];Print(a==[1]);B(edit a);Print(a[0]);}",
        "fn Main(){var a=[1];var b=[1];Print(a==b);a[0]=2;b[0]=3;}",
        "fn Main(){var a=[1];var b=[1];Print(a==b);let c=move b;Print(c[0]);}",
        "fn B(x:edit Array<I64>){x[0]=2;} fn Main(){var a=[1];var b=[1];Print(a==b);B(edit b);}",
        "fn Main(){var a=[1];Print((a==[1])==true);a[0]=2;}",
        "fn Main(){var a=[1];if a==[1] {a[0]=2;} Print(a[0]);}",
        "fn Main(){var a=[1];while a==[1] {a[0]=2;} Print(a[0]);}",
    ] {
        super::check_scalar("equality-loan.n", source)
            .unwrap_or_else(|error| panic!("check {source}: {error:?}"));
        let program = compile_scalar("equality-loan.n", source)
            .unwrap_or_else(|error| panic!("{source}: {error:?}"));
        let result = run_scalar(&program);
        assert_eq!(result.outcome, Outcome::Unit, "{source}: {result:?}");
        assert!(result.diagnostics.is_empty(), "{source}: {result:?}");
    }
}

#[test]
fn indexed_place_access_matrix_checks_root_after_every_index() {
    let uses = [
        "Print({p});",
        "AssertEq({p},1);",
        "let x={p};",
        "Read({p});",
        "Inc(edit {p});",
        "{p}=3;",
    ];
    for (binding, place) in [
        ("var b=[1,2];", "b[{index}]"),
        ("var b=[[1,2]];", "b[0][{index}]"),
    ] {
        for effect in ["let t=move b;", "Drop(b);"] {
            let index = format!("if true then {{{effect}value:0;}} else 0");
            let selected = place.replace("{index}", &index);
            for use_site in uses {
                let body = use_site.replace("{p}", &selected);
                let source = format!(
                    "fn Read(x:view I64){{}} fn Inc(x:edit I64){{x=x+1;}} fn Main(){{{binding}{body}}}"
                );
                for result in [
                    super::check_scalar("index-matrix.n", &source),
                    compile_scalar("index-matrix.n", &source),
                ] {
                    let Err(CompileError::Rejected(errors)) = result else {
                        panic!("expected source rejection: {source}: {result:?}");
                    };
                    assert!(
                        errors.iter().any(|error| error.code == "NX-E0202"),
                        "{source}: {errors:?}"
                    );
                    for error in errors {
                        error.validate("index-matrix.n", &source).unwrap();
                    }
                }
            }
        }
    }
}

#[test]
#[ignore = "structured indexed-place ownership campaign"]
fn indexed_place_effects_never_escape_as_invalid_ir() {
    let cases_per_seed = std::env::var("NEXO_INDEX_EFFECT_CASES")
        .ok()
        .and_then(|value| value.parse::<usize>().ok())
        .unwrap_or(100_000);
    let uses = [
        "Print({p});",
        "AssertEq({p},1);",
        "let x={p};",
        "Read({p});",
        "Inc(edit {p});",
        "{p}=3;",
        "Print({p}==1);",
        "let x=[{p},1];",
    ];
    let effects = [
        "if true then {let t=move b;value:0;} else 0",
        "if false then {Drop(b);value:0;} else 0",
        "Len(move b)",
        "if true then {{write}value:0;} else 0",
        "if false then {Touch(edit b);value:0;} else 0",
        "if true then {Print({read});value:0;} else 0",
    ];
    let mut accepted_compilations = 0usize;
    let mut rejected_compilations = 0usize;
    for seed in 0..4_usize {
        let mut state = 0x57a1_4b31_0d0e_0001_u64 ^ (seed as u64 * 0x10001);
        for case in 0..cases_per_seed {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            let nested = (state & 1) != 0;
            let (binding, ty, write, read, place) = if nested {
                (
                    "var b=[[1,2],[3,4]];",
                    "Array<Array<I64>>",
                    "b[0][0]=4;",
                    "b[0][0]",
                    "b[0][{index}]",
                )
            } else {
                (
                    "var b=[1,2];",
                    "Array<I64>",
                    "b[0]=4;",
                    "b[0]",
                    "b[{index}]",
                )
            };
            let effect = effects[(state >> 8) as usize % effects.len()]
                .replace("{write}", write)
                .replace("{read}", read);
            let selected = place.replace("{index}", &effect);
            let body = uses[(case + seed) % uses.len()].replace("{p}", &selected);
            let source = format!(
                "fn Read(x:view I64){{}} fn Inc(x:edit I64){{x=x+1;}} fn Touch(x:edit {ty}){{}} fn Main(){{{binding}let pad={};{body}}}",
                case % 997
            );
            for result in [
                super::check_scalar("index-fuzz.n", &source),
                compile_scalar("index-fuzz.n", &source),
            ] {
                match result {
                    Ok(program) => {
                        accepted_compilations += 1;
                        let execution = run_scalar(&program);
                        assert!(
                            !matches!(execution.outcome, Outcome::InternalError(_)),
                            "{source}: {execution:?}"
                        );
                        for error in execution.diagnostics {
                            error.validate("index-fuzz.n", &source).unwrap();
                        }
                    }
                    Err(CompileError::Rejected(errors)) => {
                        rejected_compilations += 1;
                        assert!(!errors.is_empty(), "{source}");
                        for error in errors {
                            error.validate("index-fuzz.n", &source).unwrap();
                        }
                    }
                    Err(other) => panic!("{source}: {other:?}"),
                }
            }
        }
    }
    assert_eq!(
        accepted_compilations + rejected_compilations,
        8 * cases_per_seed
    );
    println!(
        "indexed sources: {}; check/compile calls: {}; executions: {}",
        4 * cases_per_seed,
        8 * cases_per_seed,
        accepted_compilations
    );
}

fn byte_span(source: &str, needle: &str, occurrence: usize) -> (u64, u64) {
    let start = source
        .match_indices(needle)
        .nth(occurrence)
        .unwrap_or_else(|| panic!("missing occurrence {occurrence} of {needle:?}"))
        .0 as u64;
    (start, start + needle.len() as u64)
}
#[test]
fn integer_limits_execute_with_literal_expected_output() {
    let source = "fn Main() {\n    AssertEq(-9223372036854775808 + 1, -9223372036854775807);\n    AssertEq(9223372036854775806 + 1, 9223372036854775807);\n    AssertEq(true, true);\n    Print(25);\n}\n";
    let execution = execute(source);
    assert_eq!(execution.outcome, Outcome::Unit);
    assert_eq!(execution.observations.stdout(), vec!["25"]);
    assert!(execution.observations.events().is_empty());
    assert_eq!(execution.observations.bytes(), 3);
    assert_eq!(execution.observations.reserved_bytes(), 0);
    assert!(execution.diagnostics.is_empty());
}
#[test]
fn integer_limits_fixture_runs_through_verified_ir() {
    // Exercise the corpus program's observations through verified IR.
    let source = "fn Main() {\n    AssertEq(-9223372036854775808 + 1, -9223372036854775807);\n    AssertEq(9223372036854775806 + 1, 9223372036854775807);\n    AssertEq(true, true);\n    Print(25);\n}\n";
    let e = execute(source);
    assert_eq!(e.outcome, Outcome::Unit);
    assert_eq!(e.observations.stdout(), vec!["25"]);
    assert_eq!(e.observations.bytes(), 3);
    assert!(e.diagnostics.is_empty());
}
#[test]
fn signed_literal_outside_i64_is_a_types_diagnostic() {
    let source = "fn Main() {\n    let x = 9223372036854775808;\n}\n";
    let CompileError::Rejected(errors) = compile_scalar("fixture.n", source).unwrap_err() else {
        panic!("expected rejection")
    };
    assert_eq!(errors.len(), 1);
    let d = &errors[0];
    assert_eq!(
        (
            &*d.code,
            d.subcause.as_deref(),
            d.stage.as_str(),
            d.operation.as_str()
        ),
        ("NX-E0107", None, "types", "value.literal")
    );
    assert_eq!(
        d.primary_span.as_ref().map(|s| (s.start, s.end)),
        Some(byte_span(source, "9223372036854775808", 0))
    );
    assert!(d.data.is_empty());
    d.validate("fixture.n", source).unwrap();
    assert_eq!(
        execute("fn Main() {\n    let x = 9223372036854775807;\n}\n").outcome,
        Outcome::Unit
    );
}
#[test]
fn trailing_module_garbage_is_rejected_before_lowering() {
    let source = "fn Main(){} garbage";
    let CompileError::Rejected(errors) = compile_scalar("fixture.n", source).unwrap_err() else {
        panic!("expected rejection")
    };
    let d = &errors[0];
    assert_eq!(
        (&*d.code, d.stage.as_str(), d.operation.as_str()),
        ("NX-E0002", "parse", "syntax.parse")
    );
    assert_eq!(
        d.primary_span.as_ref().map(|s| (s.start, s.end)),
        Some((12, 19))
    );
    assert_eq!(
        d.unexpected_token.as_ref().and_then(|t| t.text.as_deref()),
        Some("garbage")
    );
    d.validate("fixture.n", source).unwrap();
    assert_eq!(execute("fn Main() {}\n").outcome, Outcome::Unit);
}
#[test]
fn b_corpus_comments_and_crlf_keep_original_byte_offsets() {
    for (fixture, code, needle, published_lf, published_crlf) in [
        (
            include_str!("../../corpus/fail/overflow_literal.n"),
            "NX-E0107",
            "9223372036854775808",
            Some((44, 63)),
            Some((45, 64)),
        ),
        (
            include_str!("../../corpus/fail/trailing_garbage.n"),
            "NX-E0002",
            "ignored",
            None,
            None,
        ),
    ] {
        let lf = fixture.replace("\r\n", "\n");
        for (source, published) in [
            (lf.clone(), published_lf),
            (lf.replace('\n', "\r\n"), published_crlf),
        ] {
            let (start, end) = byte_span(&source, needle, 0);
            if let Some(expected) = published {
                assert_eq!((start, end), expected);
            }
            let CompileError::Rejected(errors) = compile_scalar("fixture.n", &source).unwrap_err()
            else {
                panic!("expected rejection")
            };
            assert_eq!(errors.len(), 1);
            let d = &errors[0];
            assert_eq!(d.code, code);
            assert_eq!(
                d.primary_span.as_ref().map(|s| (s.start, s.end)),
                Some((start, end))
            );
            assert!(d.data.is_empty());
            d.validate("fixture.n", &source).unwrap();
        }
    }
}
#[test]
fn subtraction_overflow_has_published_runtime_fields_and_no_output() {
    let source = "fn Main() {\n    Print(-9223372036854775808 - 1);\n}\n";
    let e = execute(source);
    assert_eq!(e.outcome, Outcome::SimulatorFault);
    assert!(e.observations.observations().is_empty());
    assert_eq!(e.observations.bytes(), 0);
    assert_eq!(e.diagnostics.len(), 1);
    let d = &e.diagnostics[0];
    assert_eq!(
        (
            &*d.code,
            d.subcause.as_deref(),
            d.diagnostic_namespace.as_str(),
            d.stage.as_str(),
            d.operation.as_str()
        ),
        ("R0103", None, "F0-0002-sim", "runtime", "value.sub")
    );
    assert_eq!(
        d.primary_span.as_ref().map(|s| (s.start, s.end)),
        Some(byte_span(source, "-9223372036854775808 - 1", 0))
    );
    assert!(d.data.is_empty());
    let p = execute("fn Main() {\n    Print(-9223372036854775808 - 0);\n}\n");
    assert_eq!(p.outcome, Outcome::Unit);
    assert_eq!(p.observations.stdout(), vec!["-9223372036854775808"]);
    assert_eq!(p.observations.bytes(), 21);
}
#[test]
fn addition_overflow_and_assert_failure_keep_prior_output_in_order() {
    let plus = "fn Main() {\n    Print(7);\n    Print(9223372036854775807 + 1);\n}\n";
    let e = execute(plus);
    assert_eq!(e.outcome, Outcome::SimulatorFault);
    assert_eq!(e.observations.stdout(), vec!["7"]);
    assert_eq!(e.observations.bytes(), 2);
    assert_eq!(e.diagnostics[0].code, "R0101");
    assert_eq!(e.diagnostics[0].stage.as_str(), "runtime");
    assert_eq!(e.diagnostics[0].operation.as_str(), "value.add");
    assert!(e.diagnostics[0].data.is_empty());
    let assertion = execute("fn Main() {\n    Print(7);\n    AssertEq(1, 2);\n    Print(9);\n}\n");
    assert_eq!(assertion.outcome, Outcome::SimulatorFault);
    assert_eq!(assertion.observations.stdout(), vec!["7"]);
    assert_eq!(assertion.diagnostics[0].code, "R0201");
    assert_eq!(
        assertion.diagnostics[0].operation.as_str(),
        "intrinsic.assert_eq"
    );
}
#[test]
fn scalar_drop_has_no_event_and_fail_faults_after_prior_output() {
    let e = execute(
        "fn Main() {\n    let x = 3;\n    Drop(x);\n    Print(true);\n    Fail();\n    Print(9);\n}\n",
    );
    assert_eq!(e.outcome, Outcome::SimulatorFault);
    assert_eq!(e.observations.stdout(), vec!["true"]);
    assert!(e.observations.events().is_empty());
    assert_eq!(e.diagnostics.len(), 1);
    assert_eq!(
        (&*e.diagnostics[0].code, e.diagnostics[0].operation.as_str()),
        ("R0301", "intrinsic.fail")
    );
    assert!(e.diagnostics[0].data.is_empty());
    let p = execute("fn Main() {\n    Drop(3);\n    Print(false);\n}\n");
    assert_eq!(p.outcome, Outcome::Unit);
    assert_eq!(p.observations.stdout(), vec!["false"]);
}
#[test]
fn observation_quota_checks_complete_next_line_and_preserves_prior_lines() {
    let mut source = String::from("fn Main(){");
    for _ in 0..32_769 {
        source.push_str("Print(0);")
    }
    source.push('}');
    let e = execute(&source);
    assert_eq!(e.outcome, Outcome::SimulatorFault);
    assert_eq!(e.observations.stdout().len(), 32_768);
    assert_eq!(e.observations.bytes(), 65_536);
    let d = &e.diagnostics[0];
    assert_eq!(d.code, "R0002");
    assert_eq!(d.operation.as_str(), "runtime.limit");
    assert_eq!(
        d.data.get("runtime_budget_kind"),
        Some(&crate::diagnostic::DataValue::String(
            "observation_bytes".into()
        ))
    );
    assert_eq!(
        d.data.get("limit"),
        Some(&crate::diagnostic::DataValue::UInt(65_536))
    );
    assert_eq!(
        d.data.get("observed"),
        Some(&crate::diagnostic::DataValue::UInt(65_538))
    );
}
#[test]
fn value_minus_one_spellings_share_ast_and_result() {
    let a = "fn Main() {\n    let value = 2;\n    let a = value - 1;\n    let b = value - 1;\n    AssertEq(a, b);\n}\n";
    let b = a.replacen("value -1", "value - 1", 1);
    let ast_a = parse::parse("fixture.n", a).unwrap();
    let ast_b = parse::parse("fixture.n", &b).unwrap();
    assert_eq!(
        crate::frontend::print::print_canonical(&ast_a).unwrap(),
        crate::frontend::print::print_canonical(&ast_b).unwrap()
    );
    let pa = compile_scalar("fixture.n", a).unwrap();
    let pb = compile_scalar("fixture.n", &b).unwrap();
    assert_eq!(
        pa.functions().function("Main").unwrap().content_hash,
        pb.functions().function("Main").unwrap().content_hash
    );
    for source in [a, b.as_str()] {
        let e = execute(source);
        assert_eq!(e.outcome, Outcome::Unit);
        assert!(e.observations.observations().is_empty());
    }
}
#[test]
fn unicode_crlf_and_final_eof_binding_variants_execute_equally() {
    let base = "fn Main() {\n    let missing = 1;\n    let x = missing;\n}\n";
    let crlf = base.replace('\n', "\r\n");
    let final_lf = format!("{base}\n");
    let mut canonical = Vec::new();
    for source in [base, crlf.as_str(), final_lf.as_str()] {
        let ast = parse::parse("fixture.n", source).unwrap();
        canonical.push(crate::frontend::print::print_canonical(&ast).unwrap());
        let e = execute(source);
        assert_eq!(e.outcome, Outcome::Unit);
        assert!(e.diagnostics.is_empty());
        assert!(e.observations.observations().is_empty());
    }
    assert_eq!(canonical[0], canonical[1]);
    assert_eq!(canonical[0], canonical[2]);
}
#[test]
fn scalar_positive_repairs_of_published_scope_rows_execute_without_observations() {
    let cases = [
        ("S16", "fn Main() {\n    let b = 0 < 1;\n}\n"),
        ("S18", "fn Main() {\n    let b = 1 == 1;\n}\n"),
        ("S22", "fn Main() {\n    let matchValue = 1;\n}\n"),
        ("S23", "fn Main() {\n    let x = 1;\n    let y = 2;\n}\n"),
        ("S27", "fn Main() {\n    let x = 1 + 2;\n}\n"),
        ("S28", "fn Main() {\n    let x = 1 == 2;\n}\n"),
        ("S31", "fn Main() {\n    let x = 1;\n}\n"),
        ("S32", "fn Main() {\n    let x = 1 < 2;\n}\n"),
        ("S33", "fn Main() {\n    let x = 1 < 2;\n}\n"),
        ("S34", "fn Main() {\n    let x = 1 < 2;\n}\n"),
        ("F03", "fn Main() {\n    let x = 0;\n}\n"),
        (
            "F04",
            "fn Main() {\n    let n = 1;\n    let x = 0 + n;\n}\n",
        ),
        ("F05", "fn Main() {\n    let x = (1);\n}\n"),
        ("F06", "fn Main() {}\n"),
        ("F08", "fn Main() {\n    let f = ();\n}\n"),
        ("F16", "fn Main() {}\n"),
        ("F17", "fn Main() {}\n"),
        ("F18", "fn Main() {}\n"),
        ("F19", "fn Main() {}\n"),
        ("F21", "fn Main() {}\n"),
        ("F22", "fn Main() {\n    let x = 1;\n}\n"),
        ("F24", "fn Main() {\n    var x = 1;\n    x = x + 1;\n}\n"),
        ("F27", "fn G() {}\nfn Main() {}\n"),
        ("F28", "fn F() {}\nfn Main() {}\n"),
        ("F32", "fn Main() {\n    {}\n}\n"),
        ("F33", "fn Main() {\n    let x = 0;\n}\n"),
    ];
    for (row, source) in cases {
        let e = execute(source);
        assert_eq!(e.outcome, Outcome::Unit, "{row}");
        assert!(e.diagnostics.is_empty(), "{row}");
        assert!(e.observations.observations().is_empty(), "{row}");
        assert_eq!(e.observations.bytes(), 0, "{row}");
        assert_eq!(e.observations.reserved_bytes(), 0, "{row}");
    }
}
#[test]
fn shadowed_binding_and_complete_reassignment_use_current_slot() {
    let e = execute(
        "fn Main() {\n    var x = 1;\n    {\n        let x = 7;\n        AssertEq(x, 7);\n    }\n    x = x + 2;\n    AssertEq(x, 3);\n    Print(x);\n}\n",
    );
    assert_eq!(e.outcome, Outcome::Unit);
    assert_eq!(e.observations.stdout(), vec!["3"]);
}
#[test]
fn inner_assignment_targets_inner_binding_without_replacing_outer_value() {
    let e = execute(
        "fn Main() {\n    var x = 1;\n    {\n        var x = 7;\n        x = 8;\n        AssertEq(x, 8);\n    }\n    AssertEq(x, 1);\n}\n",
    );
    assert_eq!(e.outcome, Outcome::Unit);
    assert!(e.observations.observations().is_empty());
}
#[test]
fn scalar_relations_keep_strict_less_and_boolean_equality() {
    let e = execute(
        "fn Main() {\n    AssertEq(1 < 2, true);\n    AssertEq(2 < 2, false);\n    AssertEq(3 < 2, false);\n    AssertEq(true == false, false);\n}\n",
    );
    assert_eq!(e.outcome, Outcome::Unit);
    assert!(e.observations.observations().is_empty());
}
#[test]
fn scalar_work_tally_counts_expression_visits_and_intrinsic_entry() {
    assert_eq!(execute("fn Main() {\n    Print(1);\n}\n").work_consumed, 10);
    assert_eq!(
        execute("fn Main() {\n    AssertEq(1 < 2, true);\n}\n").work_consumed,
        19
    );
}
#[test]
fn repeated_complete_assignment_releases_replaced_cells() {
    let mut source = String::from("fn Main(){var x=0;");
    for _ in 0..66_000 {
        source.push_str("x=x+1;")
    }
    source.push_str("AssertEq(x,66000);}");
    let e = execute(&source);
    assert_eq!(e.outcome, Outcome::Unit);
    assert!(e.diagnostics.is_empty());
    assert!(e.observations.observations().is_empty());
}
#[test]
fn function_identity_and_hash_ignore_unrelated_order_and_track_body() {
    let one = compile_scalar("fixture.n", "fn Main() {\n    Print(1);\n}\n").unwrap();
    assert_eq!(
        one.functions().function("Main").unwrap().content_hash,
        0x2429_2d7d_adf8_6fdf
    );
    let a = compile_scalar(
        "fixture.n",
        "fn Helper() {\n    Print(1);\n}\nfn Main() {\n    Print(2);\n}\n",
    )
    .unwrap();
    assert_eq!(a.functions().function("Helper").unwrap().id.0, "Helper");
    assert_eq!(a.functions().function("Main").unwrap().id.0, "Main");
    let b = compile_scalar(
        "fixture.n",
        "fn Main() {\n    Print(2);\n}\nfn Helper() {\n    Print(1);\n}\n",
    )
    .unwrap();
    assert_eq!(
        a.functions()
            .functions()
            .iter()
            .map(|f| (&f.name, &f.id, f.content_hash))
            .collect::<Vec<_>>(),
        b.functions()
            .functions()
            .iter()
            .map(|f| (&f.name, &f.id, f.content_hash))
            .collect::<Vec<_>>()
    );
    let changed = compile_scalar(
        "fixture.n",
        "fn Helper() {\n    Print(1);\n}\nfn Main() {\n    Print(3);\n}\n",
    )
    .unwrap();
    assert_ne!(
        a.functions().function("Main").unwrap().content_hash,
        changed.functions().function("Main").unwrap().content_hash
    );
    assert_eq!(
        a.functions().function("Helper").unwrap().content_hash,
        changed.functions().function("Helper").unwrap().content_hash
    );
    let add = compile_scalar("fixture.n", "fn Main() {\n    Print(1 + 2);\n}\n").unwrap();
    let sub = compile_scalar("fixture.n", "fn Main() {\n    Print(1 - 2);\n}\n").unwrap();
    assert_ne!(
        add.functions().function("Main").unwrap().content_hash,
        sub.functions().function("Main").unwrap().content_hash
    );
}
#[test]
fn function_ids_keep_names_while_table_indices_follow_canonical_order() {
    for source in [
        "fn Zeta() {}\nfn Main() {}\nfn Alpha() {}\n",
        "fn Main() {}\nfn Alpha() {}\nfn Zeta() {}\n",
    ] {
        let program = compile_scalar("fixture.n", source).unwrap();
        let functions = program.functions().functions();
        assert_eq!(
            functions
                .iter()
                .map(|f| (f.name.as_str(), f.table_index, f.id.0.as_str()))
                .collect::<Vec<_>>(),
            vec![
                ("Alpha", 0, "Alpha"),
                ("Main", 1, "Main"),
                ("Zeta", 2, "Zeta")
            ]
        );
    }
}
#[test]
fn verified_function_table_substitution_changes_development_execution() {
    let mut a = compile_scalar("fixture.n", "fn Main() {\n    Print(1);\n}\n").unwrap();
    let b = compile_scalar("fixture.n", "fn Main() {\n    Print(2);\n}\n").unwrap();
    assert_eq!(run_scalar(&a).observations.stdout(), vec!["1"]);
    a.substitute_for_development("Main", &b).unwrap();
    assert_eq!(run_scalar(&a).observations.stdout(), vec!["2"]);
}
#[test]
fn function_identity_survives_other_functions_and_substitution_checks_signature() {
    let mut receiver = compile_scalar(
        "fixture.n",
        "fn Helper() {}\nfn Main() {\n    Print(1);\n}\n",
    )
    .unwrap();
    let donor = compile_scalar(
        "fixture.n",
        "fn Aaa() {}\nfn Helper() {}\nfn Main() {\n    Print(2);\n}\n",
    )
    .unwrap();
    let original = receiver.functions().function("Main").unwrap();
    let edited = donor.functions().function("Main").unwrap();
    assert_eq!(original.id, edited.id);
    assert_ne!(original.table_index, edited.table_index);
    assert_ne!(original.content_hash, edited.content_hash);
    receiver.substitute_for_development("Main", &donor).unwrap();
    assert_eq!(run_scalar(&receiver).observations.stdout(), vec!["2"]);

    let reordered =
        compile_scalar("fixture.n", "fn Main() {\n    Print(3);\n}\nfn Zzz() {}\n").unwrap();
    assert_eq!(
        reordered.functions().function("Main").unwrap().id,
        edited.id
    );
    receiver
        .substitute_for_development("Main", &reordered)
        .unwrap();
    assert_eq!(run_scalar(&receiver).observations.stdout(), vec!["3"]);

    let mut receiver = compile_scalar("fixture.n", "fn Helper() {}\nfn Main() {}\n").unwrap();
    let changed = compile_scalar(
        "fixture.n",
        "fn Helper() -> I64 {\n    return 1;\n}\nfn Main() {}\n",
    )
    .unwrap();
    assert_eq!(
        receiver.substitute_for_development("Helper", &changed),
        Err("signature mismatch")
    );
}
#[test]
fn function_table_refuses_forged_development_identity() {
    let mut program = compile_scalar("fixture.n", "fn Main() {}\n").unwrap();
    let mut replacement = program.functions().function("Main").unwrap().clone();
    replacement.id.0 = "other".into();
    assert_eq!(
        program
            .functions
            .replace_for_development("Main", replacement),
        Err("identity mismatch")
    );
}

#[test]
fn scalar_compilation_stays_within_small_calling_stacks_at_admitted_depths() {
    for stack in [256 * 1024, 512 * 1024] {
        std::thread::Builder::new()
            .stack_size(stack)
            .spawn(|| {
                let mut comparison = "(1<2)".to_string();
                for _ in 1..110 {
                    comparison = format!("({comparison}==(1<2))");
                }
                let expressions = [
                    format!("{}1{}", "(".repeat(110), ")".repeat(110)),
                    std::iter::repeat_n("1", 110).collect::<Vec<_>>().join("+"),
                    format!("1+{}1{}", "(1+".repeat(64), ")".repeat(64)),
                    comparison,
                ];
                for expr in expressions {
                    let source = format!("fn Main(){{Print({expr});}}");
                    let result = execute(&source);
                    assert_eq!(result.outcome, Outcome::Unit, "{source}");
                }
                let source = format!("fn Main(){{{}Print(1);{}}}", "{".repeat(90), "}".repeat(90));
                assert_eq!(execute(&source).outcome, Outcome::Unit);
            })
            .unwrap()
            .join()
            .unwrap();
    }
}
#[test]
fn deepest_admitted_scalar_families_compile_on_small_caller_stacks() {
    fn source(family: usize, depth: usize) -> String {
        match family {
            0 => format!(
                "fn Main(){{Print({}1{});}}",
                "(".repeat(depth),
                ")".repeat(depth)
            ),
            1 => format!(
                "fn Main(){{{}Print(1);{}}}",
                "{".repeat(depth),
                "}".repeat(depth)
            ),
            2 => format!("fn Main(){{Print({}1);}}", "1+".repeat(depth)),
            3 => format!(
                "fn Main(){{Print(1+{}1{});}}",
                "(1+".repeat(depth),
                ")".repeat(depth)
            ),
            _ => {
                let mut expr = "(1<2)".to_string();
                for _ in 0..depth {
                    expr = format!("({expr}==(1<2))");
                }
                format!("fn Main(){{Print({expr});}}")
            }
        }
    }
    for stack in [256 * 1024, 512 * 1024] {
        std::thread::Builder::new()
            .stack_size(stack)
            .spawn(move || {
                let mut endings = Vec::new();
                for family in 0..5 {
                    let mut last = 0;
                    for depth in 1..=140 {
                        let s = source(family, depth);
                        match compile_scalar("fixture.n", &s) {
                            Ok(program) => {
                                assert_eq!(run_scalar(&program).outcome, Outcome::Unit);
                                last = depth;
                            }
                            Err(CompileError::Rejected(errors)) => {
                                assert_eq!(
                                    errors[0].code, "NX-E0003",
                                    "family {family} depth {depth}"
                                );
                                endings.push(errors[0].data.get("budget_kind").cloned());
                                break;
                            }
                            Err(other) => panic!("family {family} depth {depth}: {other:?}"),
                        }
                    }
                    assert!(last >= 50, "family {family} max {last}");
                }
                assert_eq!(endings[0], endings[1]);
            })
            .unwrap()
            .join()
            .unwrap();
    }
}
#[test]
fn arrays_and_user_calls_execute_only_after_verification() {
    let arrays = compile_scalar(
        "fixture.n",
        "fn Main() {\n    Print(1);\n    let a = [2];\n}\n",
    )
    .unwrap();
    assert_eq!(run_scalar(&arrays).observations.stdout(), &["1"]);
    let called = compile_scalar("fixture.n", "fn F() {}\nfn Main() {\n    F();\n}\n").unwrap();
    assert_eq!(run_scalar(&called).outcome, Outcome::Unit);
    assert!(matches!(
        compile_scalar("fixture.n", "fn Main() {\n    let x = 1;\n    x = 2;\n}\n"),
        Err(CompileError::Rejected(_))
    ));
}
#[test]
fn nested_copy_rebinds_physical_values_without_aliasing() {
    // Keep the corpus program's copy and output oracle here.
    let source = "fn Main() {\n    var original = [[1, 2], [3, 4]];\n    var copy = original;\n    copy[0][1] = 9;\n    AssertEq(original[0][1], 2);\n    AssertEq(copy[0][1], 9);\n    var third = copy;\n    third[1] = [7, 8];\n    AssertEq(copy[1], [3, 4]);\n    original = third;\n    third[0][0] = 42;\n    AssertEq(original[0][0], 1);\n    Print(original);\n}\n";
    let e = execute(source);
    assert_eq!(e.outcome, Outcome::Unit);
    assert_eq!(e.observations.stdout(), vec!["[[1, 9], [7, 8]]"]);
    assert_eq!(e.observations.bytes(), 17);
    assert!(e.diagnostics.is_empty());
}
#[test]
fn immutable_index_root_has_ownership_diagnostic_and_declaration_span() {
    let source = "fn Main() {\n    let value = [[1]];\n    value[0][0] = 2;\n}\n";
    let CompileError::Rejected(errors) = compile_scalar("fixture.n", source).unwrap_err() else {
        panic!("ownership rejection")
    };
    assert_eq!(errors.len(), 1);
    let d = &errors[0];
    assert_eq!(
        (
            d.code.as_str(),
            d.subcause.as_deref(),
            d.stage.as_str(),
            d.operation.as_str()
        ),
        ("NX-E0105", None, "ownership", "place.write")
    );
    assert_eq!(
        d.primary_span.as_ref().map(|s| (s.start, s.end)),
        Some(byte_span(source, "value", 1))
    );
    assert_eq!(d.related_spans.len(), 1);
    assert_eq!(
        (
            d.related_spans[0].role.as_str(),
            d.related_spans[0].span.start,
            d.related_spans[0].span.end
        ),
        {
            let (start, end) = byte_span(source, "value", 0);
            ("declaration", start, end)
        }
    );
    assert!(d.data.is_empty());
    d.validate("fixture.n", source).unwrap();
    let e = execute(
        "fn Main() {\n    var value = [[1]];\n    value[0][0] = 2;\n    Print(value);\n}\n",
    );
    assert_eq!(e.observations.stdout(), vec!["[[2]]"]);
}
#[test]
fn independent_immutable_writes_report_each_site_in_source_order() {
    for (source, primary, declaration) in [
        (
            "fn Main() {\n    let a = [1];\n    let b = [2];\n    a[0] = 1;\n    b[0] = 2;\n}\n",
            vec![("a[0]", 0), ("b[0]", 0)],
            vec![("let a =", 0), ("let b =", 0)],
        ),
        (
            "fn Helper() {\n    let q = [1];\n    q[0] = 2;\n}\nfn Main() {\n    let p = [1];\n    p[0] = 2;\n}\n",
            vec![("q[0]", 0), ("p[0]", 0)],
            vec![("let q =", 0), ("let p =", 0)],
        ),
    ] {
        let CompileError::Rejected(errors) = compile_scalar("fixture.n", source).unwrap_err()
        else {
            panic!("ownership rejection")
        };
        assert_eq!(errors.len(), 2);
        for (index, error) in errors.iter().enumerate() {
            assert_eq!(
                (
                    error.code.as_str(),
                    error.stage.as_str(),
                    error.operation.as_str()
                ),
                ("NX-E0105", "ownership", "place.write")
            );
            assert_eq!(error.primary_span.as_ref().map(|s| (s.start, s.end)), {
                let start = byte_span(source, primary[index].0, primary[index].1).0;
                Some((start, start + 1))
            });
            assert_eq!(
                error
                    .related_spans
                    .iter()
                    .map(|s| (s.span.start, s.span.end))
                    .collect::<Vec<_>>(),
                {
                    let start = byte_span(source, declaration[index].0, declaration[index].1).0 + 4;
                    vec![(start, start + 1)]
                }
            );
            assert!(error.data.is_empty());
            error.validate("fixture.n", source).unwrap();
        }
    }
}

#[test]
fn immutable_write_does_not_emit_dependent_cascades() {
    let source = "fn Main() {\n    let a = [1];\n    a[0] = 2;\n    a[0] = 3;\n    let b = [4];\n    b[0] = 5;\n}\n";
    let CompileError::Rejected(errors) = compile_scalar("fixture.n", source).unwrap_err() else {
        panic!("ownership rejection")
    };
    assert_eq!(errors.len(), 3);
    assert_eq!(
        errors
            .iter()
            .map(|d| d.primary_span.as_ref().unwrap().start)
            .collect::<Vec<_>>(),
        vec![
            byte_span(source, "a[0]", 0).0,
            byte_span(source, "a[0]", 1).0,
            byte_span(source, "b[0]", 0).0,
        ]
    );
    for error in errors {
        assert_eq!(error.code, "NX-E0105");
        error.validate("fixture.n", source).unwrap();
    }
}
#[test]
fn ownership_diagnostics_sort_by_source_span_across_canonical_function_order() {
    let source = "fn Z() {\n    let a = [1];\n    a[0] = 2;\n}\nfn Main() {\n    let b = [1];\n    b[0] = 2;\n}\n";
    let CompileError::Rejected(errors) = compile_scalar("fixture.n", source).unwrap_err() else {
        panic!("ownership rejection")
    };
    assert_eq!(
        errors
            .iter()
            .map(|d| d.primary_span.as_ref().unwrap().start)
            .collect::<Vec<_>>(),
        vec![
            byte_span(source, "a[0]", 0).0,
            byte_span(source, "b[0]", 0).0
        ]
    );
}
#[test]
fn ownership_errors_in_nonunit_function_precede_lowering_of_later_functions() {
    let source = "fn Z() -> I64 {\n    let a = [1];\n    a[0] = 2;\n}\nfn Main() {\n    let b = [1];\n    b[0] = 2;\n}\n";
    let CompileError::Rejected(errors) = compile_scalar("fixture.n", source).unwrap_err() else {
        panic!("ownership rejection")
    };
    assert_eq!(
        errors.iter().map(|d| d.code.as_str()).collect::<Vec<_>>(),
        ["NX-E0105", "NX-E1317", "NX-E0105"]
    );
}

#[test]
fn missing_return_at_normal_exit_uses_published_s11_and_l06_spans() {
    // F0-0002 §14.1 S11 and §14.8 L06: the closing brace is the primary span.
    for source in [
        "fn F() -> I64 {}\nfn Main() {}\n",
        "fn F() -> I64 {\n    1;\n}\nfn Main() {}\n",
    ] {
        let CompileError::Rejected(errors) = compile_scalar("fixture.n", source).unwrap_err()
        else {
            panic!("expected missing return")
        };
        assert_eq!(errors.len(), 1, "{source}");
        let d = &errors[0];
        assert_eq!((d.code.as_str(), d.subcause.as_deref()), ("NX-E1317", None));
        assert_eq!(
            (d.stage.as_str(), d.operation.as_str()),
            ("ownership", "flow.return")
        );
        assert_eq!(
            d.primary_span.as_ref().map(|s| (s.start, s.end)),
            Some(byte_span(source, "}", 0))
        );
        assert_eq!(d.related_spans.len(), 1);
        assert_eq!(d.related_spans[0].role.as_str(), "expected_type");
        assert_eq!(
            (d.related_spans[0].span.start, d.related_spans[0].span.end),
            byte_span(source, "I64", 0)
        );
        assert!(d.data.is_empty());
        assert!(d.cause_chain.is_empty());
        assert!(d.unexpected_token.is_none());
        d.validate("fixture.n", source).unwrap();
    }
    compile_scalar(
        "fixture.n",
        "fn F() -> I64 {\n    return 1;\n}\nfn Main() {}\n",
    )
    .unwrap();
    compile_scalar(
        "fixture.n",
        "fn F() -> I64 {\n    return 1;\n}\nfn Main() {}\n",
    )
    .unwrap();
}

#[test]
fn cfg_statement_if_selects_one_branch() {
    let source = "fn Main() {\n    var x = 0;\n    if true {\n        x = 1;\n    } else {\n        x = 2;\n    }\n    AssertEq(x, 1);\n}\n";
    let program = compile_scalar("if.n", source).unwrap();
    let execution = crate::interp::run_scalar(&program);
    assert!(
        matches!(execution.outcome, crate::interp::Outcome::Unit),
        "{execution:?}"
    );
}

#[test]
fn cfg_while_rechecks_condition_and_updates_local() {
    let source = "fn Main() {\n    var x = 0;\n    while x < 3 {\n        x = x + 1;\n    }\n    AssertEq(x, 3);\n}\n";
    let program = compile_scalar("while.n", source).unwrap();
    let execution = crate::interp::run_scalar(&program);
    assert!(
        matches!(execution.outcome, crate::interp::Outcome::Unit),
        "{execution:?}"
    );
}

#[test]
fn cfg_expression_if_delivers_explicit_unit() {
    // F0-0002 §14.8 L05.
    let source = "fn Main() {\n    let x = if true then () else ();\n    AssertEq(x, ());\n}\n";
    let program = compile_scalar("value.n", source).unwrap();
    let execution = crate::interp::run_scalar(&program);
    assert!(
        matches!(execution.outcome, crate::interp::Outcome::Unit),
        "{execution:?}"
    );
}

#[test]
fn cfg_join_reports_maybe_consumed_resource() {
    // F0-0002 §14.2 P08.
    let source = "fn F(b: view Bool) {\n    let r = MockOpen(1);\n    if b {\n        Drop(r);\n    }\n    Inspect(r);\n}\nfn Main() {}\n";
    let CompileError::Rejected(errors) = compile_scalar("p08.n", source).unwrap_err() else {
        panic!("expected maybe consumed resource")
    };
    assert_eq!(errors.len(), 1);
    let d = &errors[0];
    assert_eq!(
        (d.code.as_str(), d.subcause.as_deref()),
        ("NX-E0202", Some("maybe_consumed"))
    );
    assert_eq!(
        (d.stage.as_str(), d.operation.as_str()),
        ("ownership", "ownership.use")
    );
    assert_eq!(d.primary_span.as_ref().map(|s| (s.start, s.end)), {
        let start = byte_span(source, "Inspect(r)", 0).0 + "Inspect(".len() as u64;
        Some((start, start + 1))
    });
    assert_eq!(
        d.related_spans
            .iter()
            .map(|r| (r.role.as_str(), r.span.start, r.span.end))
            .collect::<Vec<_>>(),
        {
            let declaration = byte_span(source, "let r =", 0).0 + 4;
            let consumption = byte_span(source, "Drop(r)", 0);
            [
                ("declaration", declaration, declaration + 1),
                ("consumption", consumption.0, consumption.1),
            ]
        }
    );
    assert_eq!(d.cause_chain.len(), 1);
    assert_eq!(
        d.cause_chain[0].span.as_ref().map(|s| (s.start, s.end)),
        Some(byte_span(source, "Drop(r)", 0))
    );
    assert_eq!(
        d.cause_chain[0].entity.as_ref().map(|e| e.id.as_str()),
        Some(format!("binding:{}", byte_span(source, "let r =", 0).0 + 4).as_str())
    );
    assert!(d.data.is_empty());
    d.validate("p08.n", source).unwrap();
}

#[test]
fn cfg_returning_branch_does_not_poison_join() {
    let source = "fn F(b: view Bool) {\n    let r = MockOpen(1);\n    if b {\n        Drop(r);\n        return;\n    }\n    Inspect(r);\n}\nfn Main() {\n    F(true);\n    F(false);\n}\n";
    let program = compile_scalar("join.n", source).unwrap();
    let execution = crate::interp::run_scalar(&program);
    assert!(
        matches!(execution.outcome, crate::interp::Outcome::Unit),
        "{execution:?}"
    );
}

#[test]
fn cfg_first_value_arm_continues_when_else_returns() {
    let source = "fn F(b: view Bool) -> I64 {\n    return if b then 2 else {\n        return 3;\n    };\n}\nfn Main() {\n    AssertEq(F(true), 2);\n    AssertEq(F(false), 3);\n}\n";
    let program = compile_scalar("first_arm.n", source).unwrap();
    let execution = crate::interp::run_scalar(&program);
    assert!(
        matches!(execution.outcome, crate::interp::Outcome::Unit),
        "{execution:?}"
    );
}

#[test]
fn cfg_returning_loop_body_excludes_consumption_from_false_exit() {
    let source = "fn F(b: view Bool) {\n    let r = MockOpen(1);\n    while b {\n        Drop(r);\n        return;\n    }\n    Inspect(r);\n}\nfn Main() {\n    F(false);\n    F(true);\n}\n";
    let program = compile_scalar("loop_return_owner.n", source).unwrap();
    let execution = crate::interp::run_scalar(&program);
    assert!(
        matches!(execution.outcome, crate::interp::Outcome::Unit),
        "{execution:?}"
    );
    assert_eq!(
        execution.observations.events(),
        [
            "open:0:1",
            "inspect:0:1",
            "close:0:1",
            "open:1:1",
            "close:1:1"
        ]
    );
}

#[test]
fn cfg_else_consumption_joins_as_maybe_consumed() {
    // Inverse of P08: the false continuation consumes the binding.
    let source = "fn F(b: view Bool) {\n    let r = MockOpen(1);\n    if b {} else {\n        Drop(r);\n    }\n    Inspect(r);\n}\nfn Main() {}\n";
    let CompileError::Rejected(errors) = compile_scalar("else_consumed.n", source).unwrap_err()
    else {
        panic!("expected maybe consumed binding")
    };
    assert_eq!(errors.len(), 1);
    assert_eq!(errors[0].code, "NX-E0202");
    assert_eq!(errors[0].subcause.as_deref(), Some("maybe_consumed"));
}

#[test]
fn cfg_only_continuing_arm_determines_consumed_state_in_both_orders() {
    // F0-0002 §6.2: returning arms do not contribute to an availability union.
    for source in [
        "fn F(b: view Bool) {\n    let r = MockOpen(1);\n    if b {\n        Drop(r);\n    } else {\n        return;\n    }\n    Inspect(r);\n}\nfn Main() {}\n",
        "fn F(b: view Bool) {\n    let r = MockOpen(1);\n    if b {\n        return;\n    } else {\n        Drop(r);\n    }\n    Inspect(r);\n}\nfn Main() {}\n",
    ] {
        let CompileError::Rejected(errors) =
            compile_scalar("single_owner_arm.n", source).unwrap_err()
        else {
            panic!("expected consumed owner")
        };
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].code, "NX-E0202");
        assert_eq!(errors[0].subcause.as_deref(), Some("definitely_consumed"));
    }
}

#[test]
fn cfg_availability_merge_distinguishes_all_consumed_and_reinitialized() {
    // F0-0002 §6.2: the union contains only continuing paths.
    let all_consumed = "fn F(b: view Bool) {\n    let r = MockOpen(1);\n    if b {\n        Drop(r);\n    } else {\n        Drop(r);\n    }\n    Inspect(r);\n}\nfn Main() {}\n";
    let CompileError::Rejected(errors) =
        compile_scalar("all_consumed.n", all_consumed).unwrap_err()
    else {
        panic!("expected consumed binding")
    };
    assert_eq!(errors.len(), 1);
    assert_eq!(errors[0].code, "NX-E0202");
    assert_eq!(errors[0].subcause.as_deref(), Some("definitely_consumed"));

    let restored = "fn F(b: view Bool) {\n    var r = MockOpen(1);\n    if b {\n        Drop(r);\n        r = MockOpen(2);\n    } else {\n        Drop(r);\n        r = MockOpen(3);\n    }\n    Inspect(r);\n}\nfn Main() {\n    F(true);\n    F(false);\n}\n";
    let program = compile_scalar("restored.n", restored).unwrap();
    let execution = crate::interp::run_scalar(&program);
    assert!(
        matches!(execution.outcome, crate::interp::Outcome::Unit),
        "{execution:?}"
    );
}

#[test]
fn cfg_published_branch_negative_rows_keep_their_rule() {
    // F0-0002 §14.3 B01–B06 and B09 (B08 is a parser terminator case).
    let rows = [
        (
            "B01",
            "fn F(b:view Bool)->I64{return if b then {value:1;};} fn Main(){}",
            "NX-E1302",
            Some("missing_else"),
            (30, 32),
        ),
        (
            "B02",
            "fn F()->I64{return if true then {1;}else{value:2;};} fn Main(){}",
            "NX-E1302",
            Some("missing_branch_value"),
            (35, 36),
        ),
        (
            "B03",
            "fn F(){let n=if true then {value:1;}else{value:true;};} fn Main(){}",
            "NX-E1303",
            None,
            (47, 51),
        ),
        (
            "B04",
            "fn Main(){value:1;}",
            "NX-E1302",
            Some("invalid_value_target"),
            (10, 16),
        ),
        (
            "B05",
            "fn F()->I64{return if true then {value:}else{value:2;};} fn Main(){}",
            "NX-E1302",
            Some("missing_value_operand"),
            (33, 39),
        ),
        (
            "B06",
            "fn F()->I64{return if true then {value:1;2;}else{value:2;};} fn Main(){}",
            "NX-E1302",
            Some("non_final_branch_value"),
            (33, 39),
        ),
        (
            "B09",
            "fn F()->I64{return if true then {value:1;}else{value:true;};} fn Main(){}",
            "NX-E0102",
            Some("type_mismatch"),
            (53, 57),
        ),
    ];
    for (row, source, code, subcause, primary) in rows {
        let CompileError::Rejected(errors) = compile_scalar("branch.n", source).unwrap_err() else {
            panic!("{row}: expected rejection")
        };
        assert_eq!(errors.len(), 1, "{row}: {errors:?}");
        assert_eq!(
            (errors[0].code.as_str(), errors[0].subcause.as_deref()),
            (code, subcause),
            "{row}: {errors:?}"
        );
        assert_eq!(
            errors[0].primary_span.as_ref().map(|s| (s.start, s.end)),
            Some(primary),
            "{row}: {errors:?}"
        );
        errors[0].validate("branch.n", source).unwrap();
    }
}

#[test]
fn cfg_b08_missing_value_terminator_keeps_parser_fields() {
    // F0-0002 §14.3 B08: the closing brace is the offending token.
    let source = "fn F()->I64{return if true then {value:1 }else{value:2;};} fn Main(){}";
    let CompileError::Rejected(errors) = compile_scalar("b08.n", source).unwrap_err() else {
        panic!("expected missing terminator")
    };
    assert_eq!(errors.len(), 1);
    let d = &errors[0];
    assert_eq!(d.code, "NX-E0005");
    assert_eq!(
        d.primary_span.as_ref().map(|s| (s.start, s.end)),
        Some((41, 42))
    );
    assert_eq!(
        d.unexpected_token.as_ref().and_then(|t| t.text.as_deref()),
        Some("}")
    );
    assert_eq!(d.expected_tokens, [";"]);
    assert_eq!(
        d.expected_tokens_kind,
        Some(crate::diagnostic::ExpectedTokensKind::CanonicalInsertion)
    );
    d.validate("b08.n", source).unwrap();
}

#[test]
fn cfg_published_branch_repairs_execute() {
    // F0-0002 §14.3 B01–B06, B08–B09 repairs; results are semantic observations.
    for source in [
        "fn F(b: view Bool) -> I64 {\n    return if b then 1 else 2;\n}\nfn Main() {\n    AssertEq(F(true), 1);\n    AssertEq(F(false), 2);\n}\n",
        "fn F() -> I64 {\n    return if true then 1 else 2;\n}\nfn Main() {\n    AssertEq(F(), 1);\n}\n",
        "fn F() {\n    let n = if true then 1 else 2;\n    AssertEq(n, 1);\n}\nfn Main() {\n    F();\n}\n",
        "fn Main() {\n    let value = 1;\n    AssertEq(value, 1);\n}\n",
    ] {
        let program = compile_scalar("branch_repair.n", source).unwrap();
        let execution = crate::interp::run_scalar(&program);
        assert!(
            matches!(execution.outcome, crate::interp::Outcome::Unit),
            "{source}: {execution:?}"
        );
    }
}

#[test]
fn cfg_hash_tracks_branch_contents_and_development_substitution() {
    let original = "fn F(b: view Bool) -> I64 {\n    return if b then 1 else 2;\n}\nfn Main() {\n    Print(F(true));\n}\n";
    let spaced = "fn F(b: view Bool) -> I64 {\n    return if b then 1 else 2;\n}\nfn Main() {\n    Print(F(true));\n}\n";
    let changed = "fn F(b: view Bool) -> I64 {\n    return if b then 3 else 2;\n}\nfn Main() {\n    Print(F(true));\n}\n";
    let mut program = compile_scalar("cfg_hash.n", original).unwrap();
    let same = compile_scalar("cfg_hash.n", spaced).unwrap();
    let donor = compile_scalar("cfg_hash.n", changed).unwrap();
    let hash = program.functions.function("F").unwrap().content_hash;
    assert_eq!(hash, same.functions.function("F").unwrap().content_hash);
    assert_ne!(hash, donor.functions.function("F").unwrap().content_hash);
    assert_eq!(
        crate::interp::run_scalar(&program).observations.stdout(),
        ["1"]
    );
    program
        .functions
        .replace_for_development("F", donor.functions.function("F").unwrap().clone())
        .unwrap();
    assert_eq!(
        crate::interp::run_scalar(&program).observations.stdout(),
        ["3"]
    );
}

#[test]
fn cfg_loop_ownership_fixpoint_reports_p09() {
    // F0-0002 §6.3 and §17.2 P09 require a fixed point on the backedge.
    let source = "fn F(b: view Bool) {\n    let r = MockOpen(1);\n    while b {\n        Drop(r);\n    }\n}\nfn Main() {}\n";
    let result = compile_scalar("loop.n", source);
    let Err(CompileError::Rejected(errors)) = result else {
        panic!("{result:?}")
    };
    assert_eq!(errors.len(), 1);
    let d = &errors[0];
    assert_eq!(d.code, "NX-E0202");
    assert_eq!(d.subcause.as_deref(), Some("maybe_consumed"));
    assert_eq!(d.primary_span.as_ref().map(|s| (s.start, s.end)), {
        let start = byte_span(source, "Drop(r)", 0).0 + "Drop(".len() as u64;
        Some((start, start + 1))
    });
    assert_eq!(d.data.len(), 0);
    assert_eq!(
        d.related_spans
            .iter()
            .map(|r| (r.role, r.span.start, r.span.end))
            .collect::<Vec<_>>(),
        [
            {
                let start = byte_span(source, "let r =", 0).0 + 4;
                (
                    crate::diagnostic::RelatedRole::Declaration,
                    start,
                    start + 1,
                )
            },
            {
                let (start, end) = byte_span(source, "Drop(r)", 0);
                (crate::diagnostic::RelatedRole::Consumption, start, end)
            },
            {
                let start = byte_span(source, "}", 0).0;
                (crate::diagnostic::RelatedRole::Backedge, start, start + 1)
            },
        ]
    );
    assert_eq!(
        d.cause_chain[0].operation,
        crate::diagnostic::CauseOperation::FlowBackedge
    );
    assert_eq!(
        d.cause_chain[0].kind,
        crate::diagnostic::CauseKind::Backedge
    );
    assert_eq!(
        d.cause_chain[1].operation,
        crate::diagnostic::CauseOperation::ValueDrop
    );
    assert_eq!(d.cause_chain[1].kind, crate::diagnostic::CauseKind::Drop);
    assert_eq!(
        d.cause_chain
            .iter()
            .map(|c| c.span.as_ref().map(|s| (s.start, s.end)))
            .collect::<Vec<_>>(),
        [
            Some(byte_span(source, "}", 0)),
            Some(byte_span(source, "Drop(r)", 0)),
        ]
    );
    let binding = format!("binding:{}", byte_span(source, "let r =", 0).0 + 4);
    assert_eq!(
        d.cause_chain
            .iter()
            .map(|c| c.entity.as_ref().unwrap().id.as_str())
            .collect::<Vec<_>>(),
        [binding.as_str(), binding.as_str()]
    );
}

#[test]
fn cfg_missing_return_considers_every_normal_path() {
    // F0-0002 §6.2, §6.3, §17.1 and S11.
    for source in [
        "fn F(b: view Bool) -> I64 {\n    if b {\n        return 1;\n    }\n}\nfn Main() {}\n",
        "fn F(b: view Bool) -> I64 {\n    while b {\n        return 1;\n    }\n}\nfn Main() {}\n",
    ] {
        let CompileError::Rejected(errors) = compile_scalar("return.n", source).unwrap_err() else {
            panic!("expected missing return: {source}")
        };
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].code, "NX-E1317");
    }
    let source = "fn F(b: view Bool) -> I64 {\n    if b {\n        return 1;\n    } else {\n        return 2;\n    }\n}\nfn Main() {\n    AssertEq(F(true), 1);\n    AssertEq(F(false), 2);\n}\n";
    let program = compile_scalar("return.n", source).unwrap();
    let execution = crate::interp::run_scalar(&program);
    assert!(
        matches!(execution.outcome, crate::interp::Outcome::Unit),
        "{execution:?}"
    );
}

#[test]
fn cfg_expression_if_excludes_returning_continuations() {
    let source = "fn F(b: view Bool) -> I64 {\n    return if b then {\n        return 3;\n    } else 2;\n}\nfn Main() {\n    AssertEq(F(true), 3);\n    AssertEq(F(false), 2);\n}\n";
    let program = compile_scalar("value_return.n", source).unwrap();
    let execution = crate::interp::run_scalar(&program);
    assert!(
        matches!(execution.outcome, crate::interp::Outcome::Unit),
        "{execution:?}"
    );
    let source = "fn F(b: view Bool) -> I64 {\n    return if b then {\n        return 3;\n    } else {\n        return 2;\n    };\n}\nfn Main() {\n    AssertEq(F(true), 3);\n    AssertEq(F(false), 2);\n}\n";
    let program = compile_scalar("value_return.n", source).unwrap();
    let execution = crate::interp::run_scalar(&program);
    assert!(
        matches!(execution.outcome, crate::interp::Outcome::Unit),
        "{execution:?}"
    );
}

#[test]
fn cfg_else_if_expression_selects_each_value() {
    let source = "fn F(x: view I64) -> I64 {\n    return if x < 0 then 1 else (if x < 2 then 2 else 3);\n}\nfn Main() {\n    AssertEq(F(0 - 1), 1);\n    AssertEq(F(1), 2);\n    AssertEq(F(3), 3);\n}\n";
    let program = compile_scalar("chain.n", source).unwrap();
    let execution = crate::interp::run_scalar(&program);
    assert!(
        matches!(execution.outcome, crate::interp::Outcome::Unit),
        "{execution:?}"
    );
}

#[test]
fn cfg_else_if_statement_selects_each_arm() {
    let source = "fn F(x: view I64) -> I64 {\n    var y = 0;\n    if x < 0 {\n        y = 1;\n    } else if x < 2 {\n        y = 2;\n    } else {\n        y = 3;\n    }\n    return y;\n}\nfn Main() {\n    AssertEq(F(0 - 1), 1);\n    AssertEq(F(1), 2);\n    AssertEq(F(3), 3);\n}\n";
    let program = compile_scalar("chain.n", source).unwrap();
    let execution = crate::interp::run_scalar(&program);
    assert!(
        matches!(execution.outcome, crate::interp::Outcome::Unit),
        "{execution:?}"
    );
}

#[test]
fn cfg_nested_while_recreates_and_cleans_body_local_each_iteration() {
    // F0-0002 §6.3 and §6.4: each body entry owns a fresh resource instance.
    let source = "fn Main() {\n    var x = 0;\n    while x < 2 {\n        var y = 0;\n        while y < 2 {\n            let r = MockOpen(x + x + y);\n            Inspect(r);\n            y = y + 1;\n        }\n        x = x + 1;\n    }\n}\n";
    let program = compile_scalar("nested.n", source).unwrap();
    let execution = crate::interp::run_scalar(&program);
    assert!(
        matches!(execution.outcome, crate::interp::Outcome::Unit),
        "{execution:?}"
    );
    assert_eq!(
        execution.observations.events(),
        [
            "open:0:0",
            "inspect:0:0",
            "close:0:0",
            "open:1:1",
            "inspect:1:1",
            "close:1:1",
            "open:2:2",
            "inspect:2:2",
            "close:2:2",
            "open:3:3",
            "inspect:3:3",
            "close:3:3",
        ]
    );
}

#[test]
fn cfg_infinite_loop_uses_work_quota_and_cleans_resource() {
    let program = compile_scalar(
        "quota.n",
        "fn Main() {\n    let r = MockOpen(1);\n    while true {}\n}\n",
    )
    .unwrap();
    let execution = crate::interp::run_scalar(&program);
    assert!(
        matches!(execution.outcome, crate::interp::Outcome::SimulatorFault),
        "{execution:?}"
    );
    assert_eq!(execution.diagnostics[0].code, "R0001");
    assert_eq!(execution.observations.events(), ["open:0:1", "close:0:1"]);
}

#[test]
fn cfg_l02_repaired_value_line_break_executes() {
    // F0-0002 §14.8 L02: grouping begins before the LF.
    let source = "fn F() -> I64 {\n    return if true then (1) else 2;\n}\nfn Main() {\n    AssertEq(F(), 1);\n}\n";
    let program = compile_scalar("l02.n", source).unwrap();
    let execution = crate::interp::run_scalar(&program);
    assert!(
        matches!(execution.outcome, crate::interp::Outcome::Unit),
        "{execution:?}"
    );
}

#[test]
fn cfg_deferred_forms_have_executable_repairs() {
    // F0-0002 §14.6 F14, F25, F26.
    for (source, feature) in [
        ("fn Main(){match true{true=>{},false=>{}}}", "match"),
        ("fn Main(){for n in [1]{}}", "for_iteration"),
        ("fn Main(){while false{break;}}", "loop_control"),
    ] {
        let CompileError::Rejected(errors) = compile_scalar("feature.n", source).unwrap_err()
        else {
            panic!("expected deferred form")
        };
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].code, "NX-E0006");
        assert_eq!(errors[0].feature_id.as_deref(), Some(feature));
    }
    for source in [
        "fn Main() {\n    if true {} else {}\n}\n",
        "fn Main() {\n    while false {}\n}\n",
    ] {
        let program = compile_scalar("feature.n", source).unwrap();
        let execution = crate::interp::run_scalar(&program);
        assert!(
            matches!(execution.outcome, crate::interp::Outcome::Unit),
            "{execution:?}"
        );
    }
}

#[test]
fn cfg_else_if_all_returning_paths_need_no_tail_return() {
    let source = "fn F(x: view I64) -> I64 {\n    if x < 0 {\n        return 1;\n    } else if x < 2 {\n        return 2;\n    } else {\n        return 3;\n    }\n}\nfn Main() {\n    AssertEq(F(0 - 1), 1);\n    AssertEq(F(1), 2);\n    AssertEq(F(3), 3);\n}\n";
    let program = compile_scalar("chain_return.n", source).unwrap();
    let execution = crate::interp::run_scalar(&program);
    assert!(
        matches!(execution.outcome, crate::interp::Outcome::Unit),
        "{execution:?}"
    );
}

#[test]
fn cfg_nested_expression_if_all_returning_paths_need_no_value_slot() {
    let source = "fn F(x: view I64) -> I64 {\n    return if x < 0 then {\n        return 1;\n    } else (if x < 2 then {\n        return 2;\n    } else {\n        return 3;\n    });\n}\nfn Main() {\n    AssertEq(F(0 - 1), 1);\n    AssertEq(F(1), 2);\n    AssertEq(F(3), 3);\n}\n";
    let program = compile_scalar("chain_return.n", source).unwrap();
    let execution = crate::interp::run_scalar(&program);
    assert!(
        matches!(execution.outcome, crate::interp::Outcome::Unit),
        "{execution:?}"
    );
}

#[test]
fn cfg_while_body_with_all_returning_arms_preserves_false_exit() {
    let source = "fn F(b: view Bool) -> I64 {\n    while b {\n        if true {\n            return 1;\n        } else {\n            return 2;\n        }\n    }\n    return 3;\n}\nfn Main() {\n    AssertEq(F(true), 1);\n    AssertEq(F(false), 3);\n}\n";
    let program = compile_scalar("loop_return.n", source).unwrap();
    let execution = crate::interp::run_scalar(&program);
    assert!(
        matches!(execution.outcome, crate::interp::Outcome::Unit),
        "{execution:?}"
    );
}

#[test]
fn cfg_unreachable_tail_still_checks_ownership_without_rejoining() {
    // F0-0002 §6.3: later statements use a separate hypothetical state.
    let source = "fn F() -> I64 {\n    if true {\n        return 1;\n    } else {\n        return 2;\n    }\n    let r = MockOpen(1);\n    Drop(r);\n    Inspect(r);\n}\nfn Main() {}\n";
    let CompileError::Rejected(errors) = compile_scalar("dead.n", source).unwrap_err() else {
        panic!("expected dead-code ownership error")
    };
    assert_eq!(errors.len(), 1);
    assert_eq!(
        (errors[0].code.as_str(), errors[0].subcause.as_deref()),
        ("NX-E0202", Some("definitely_consumed"))
    );
}

#[test]
fn cfg_value_union_transfers_resource_and_cleans_unselected_owner() {
    let source = "fn F(b: view Bool) -> MockResource {\n    let a = MockOpen(1);\n    let c = MockOpen(2);\n    return if b then move a else move c;\n}\nfn Main() {\n    let x = F(true);\n    Inspect(x);\n}\n";
    let program = compile_scalar("resource_union.n", source).unwrap();
    let execution = crate::interp::run_scalar(&program);
    assert!(
        matches!(execution.outcome, crate::interp::Outcome::Unit),
        "{execution:?}"
    );
    assert_eq!(
        execution.observations.events(),
        [
            "open:0:1",
            "open:1:2",
            "close:1:2",
            "inspect:0:1",
            "close:0:1",
        ]
    );
}

#[test]
fn cfg_branch_call_keeps_loan_permissions_and_writeback() {
    let source = "fn Bump(x: edit I64) {\n    x = x + 1;\n}\nfn Main() {\n    var a = 0;\n    if true {\n        Bump(edit a);\n    } else {}\n    AssertEq(a, 1);\n}\n";
    let program = compile_scalar("branch_loan.n", source).unwrap();
    let execution = crate::interp::run_scalar(&program);
    assert!(
        matches!(execution.outcome, crate::interp::Outcome::Unit),
        "{execution:?}"
    );
}

#[test]
fn cfg_return_from_loop_cleans_other_live_owners_before_result_delivery() {
    // F0-0002 §6.4: the result remains owned while callee locals close.
    let source = "fn F() -> MockResource {\n    let r = MockOpen(1);\n    while true {\n        let s = MockOpen(2);\n        return move s;\n    }\n    return move r;\n}\nfn Main() {\n    let x = F();\n    Inspect(x);\n}\n";
    let program = compile_scalar("loop_return_resource.n", source).unwrap();
    let execution = crate::interp::run_scalar(&program);
    assert!(
        matches!(execution.outcome, crate::interp::Outcome::Unit),
        "{execution:?}"
    );
    assert_eq!(
        execution.observations.events(),
        [
            "open:0:1",
            "open:1:2",
            "close:0:1",
            "inspect:1:2",
            "close:1:2",
        ]
    );
}

#[test]
fn cfg_loop_drops_array_cells_each_iteration() {
    let source = "fn Main() {\n    var x = 0;\n    while x < 1000 {\n        let a = [1, 2, 3, 4, 5, 6, 7, 8];\n        AssertEq(Len(a), 8);\n        x = x + 1;\n    }\n}\n";
    let program = compile_scalar("loop_cells.n", source).unwrap();
    let execution = crate::interp::run_scalar(&program);
    assert!(
        matches!(execution.outcome, crate::interp::Outcome::Unit),
        "{execution:?}"
    );
}

fn zeros(count: usize) -> String {
    std::iter::repeat_n("0", count)
        .collect::<Vec<_>>()
        .join(",")
}

#[test]
fn drop_binding_releases_cells_before_unit_and_next_array() {
    let first = zeros(32_768);
    let e = execute(&format!("fn Main(){{let a=[{first}];Drop(a);}}"));
    assert_eq!(e.outcome, Outcome::Unit);
    assert!(e.diagnostics.is_empty());
    let e = execute(&format!(
        "fn Main(){{let a=[{}];Drop(a);let b=[{}];Print(Len(b));}}",
        zeros(30_000),
        zeros(40_000)
    ));
    assert_eq!(e.outcome, Outcome::Unit);
    assert_eq!(e.observations.stdout(), vec!["40000"]);
}

#[test]
fn drop_releases_one_copy_without_invalidating_another_owner() {
    let e = execute(
        "fn Main() {\n    let a = [1, 2];\n    let b = a;\n    Drop(a);\n    AssertEq(b, [1, 2]);\n    Print(Len(b));\n}\n",
    );
    assert_eq!(e.outcome, Outcome::Unit);
    assert_eq!(e.observations.stdout(), vec!["2"]);
    let e = execute(
        "fn Main() {\n    {\n        let a = [1, 2];\n        Drop(a);\n    }\n    Print(3);\n}\n",
    );
    assert_eq!(e.outcome, Outcome::Unit);
    assert_eq!(e.observations.stdout(), vec!["3"]);
}
#[test]
fn live_cell_boundary_is_exact_with_and_without_explicit_drop() {
    let data = zeros(65_535);
    let dropped = execute(&format!("fn Main(){{let a=[{data}];Drop(a);}}"));
    assert_eq!(dropped.outcome, Outcome::Unit);
    let retained = execute(&format!("fn Main(){{let a=[{data}];}}"));
    assert_eq!(retained.outcome, Outcome::SimulatorFault);
    assert_eq!(retained.diagnostics[0].code, "R0002");
    assert_eq!(
        retained.diagnostics[0].data.get("observed"),
        Some(&crate::diagnostic::DataValue::UInt(65_537))
    );
}
#[test]
fn dropped_binding_result_unit_is_counted_before_implicit_return() {
    let e = execute(&format!(
        "fn Main(){{let a=[{}];let b=[0];let u=Drop(b);let c=0;}}",
        zeros(65_533)
    ));
    assert_eq!(e.outcome, Outcome::SimulatorFault);
    assert_eq!(e.diagnostics[0].code, "R0002");
    assert_eq!(
        e.diagnostics[0].data.get("observed"),
        Some(&crate::diagnostic::DataValue::UInt(65_537))
    );
}

#[test]
fn indexed_array_reads_borrow_selected_subtree_and_owner_context_copies() {
    let large = zeros(40_000);
    for tail in [
        "Print(Len(a[0]));",
        "AssertEq(a[0],a[0]);",
        "AssertEq(a[0]==a[0],true);",
    ] {
        let e = execute(&format!("fn Main(){{let a=[[{large}],[1]];{tail}}}"));
        assert_eq!(e.outcome, Outcome::Unit, "{tail}");
        assert!(e.diagnostics.is_empty());
        if tail.starts_with("print") {
            assert_eq!(e.observations.stdout(), vec!["40000"]);
        }
    }
    let e = execute(&format!("fn Main(){{let a=[[{large}],[1]];let b=a[0];}}"));
    assert_eq!(e.outcome, Outcome::SimulatorFault);
    assert_eq!(e.diagnostics[0].code, "R0002");
    assert_eq!(
        e.diagnostics[0].data.get("observed"),
        Some(&crate::diagnostic::DataValue::UInt(80_005))
    );
}
#[test]
fn borrowed_index_reports_invalid_path_before_reading_selected_value() {
    let e = execute("fn Main() {\n    let a = [[1]];\n    Print(Len(a[2]));\n}\n");
    assert_eq!(e.outcome, Outcome::SimulatorFault);
    assert_eq!(e.diagnostics[0].code, "R0102");
    assert_eq!(e.diagnostics[0].operation.as_str(), "value.index");
    let mut p = compile_scalar(
        "fixture.n",
        "fn Main() {\n    let a = [[1]];\n    Print(Len(a[2]));\n}\n",
    )
    .unwrap();
    let f = &mut p.functions.entries[0];
    f.block
        .instructions
        .retain(|i| !matches!(i.op, super::model::Op::CheckIndex { .. }));
    for (index, ins) in f.block.instructions.iter_mut().enumerate() {
        ins.origin.0 = index;
    }
    f.content_hash = super::model::content_hash(f);
    super::verify::verify_program(&p).unwrap();
    let e = run_scalar(&p);
    assert_eq!(e.outcome, Outcome::SimulatorFault);
    assert_eq!(e.diagnostics[0].code, "R0102");
}
#[test]
fn indexed_assignment_evaluates_rhs_before_checking_bounds() {
    let overflow =
        execute("fn Main() {\n    var a = [0];\n    a[-1] = 9223372036854775807 + 1;\n}\n");
    assert_eq!(overflow.outcome, Outcome::SimulatorFault);
    assert_eq!(overflow.diagnostics[0].code, "R0101");
    assert!(overflow.observations.stdout().is_empty());
    let invalid = execute("fn Main() {\n    var a = [0];\n    a[-1] = 2;\n}\n");
    assert_eq!(invalid.outcome, Outcome::SimulatorFault);
    assert_eq!(invalid.diagnostics[0].code, "R0102");
    assert_eq!(invalid.diagnostics[0].operation.as_str(), "value.index");
    let valid = execute("fn Main() {\n    var a = [0];\n    a[0] = 2;\n    Print(a[0]);\n}\n");
    assert_eq!(valid.outcome, Outcome::Unit);
    assert_eq!(valid.observations.stdout(), vec!["2"]);
    let nested =
        execute("fn Main() {\n    var a = [[0]];\n    a[-1][0] = 9223372036854775807 + 1;\n}\n");
    assert_eq!(nested.diagnostics[0].code, "R0101");
    for source in [
        "fn Main() {\n    let a: Array<I64> = [];\n    Print(a[0]);\n}\n",
        "fn Main() {\n    let a = [1];\n    Print(a[9223372036854775807]);\n}\n",
        "fn Main() {\n    Print([1][-1]);\n}\n",
    ] {
        let e = execute(source);
        assert_eq!(e.outcome, Outcome::SimulatorFault, "{source}");
        assert_eq!(
            (
                e.diagnostics[0].code.as_str(),
                e.diagnostics[0].operation.as_str()
            ),
            ("R0102", "value.index"),
            "{source}"
        );
        assert!(e.diagnostics[0].data.is_empty());
        assert!(e.observations.stdout().is_empty());
    }
}
#[test]
fn nested_read_checks_first_index_before_evaluating_later_index() {
    let invalid =
        execute("fn Main() {\n    let a = [[1]];\n    Print(a[-1][9223372036854775807 + 1]);\n}\n");
    assert_eq!(invalid.outcome, Outcome::SimulatorFault);
    assert_eq!(invalid.diagnostics[0].code, "R0102");
    let valid = execute("fn Main() {\n    let a = [[1]];\n    Print(a[0][0]);\n}\n");
    assert_eq!(valid.outcome, Outcome::Unit);
    assert_eq!(valid.observations.stdout(), vec!["1"]);
}
#[test]
fn array_literals_and_indices_change_function_hash() {
    let hash = |s| {
        compile_scalar("h.n", s)
            .unwrap()
            .functions()
            .function("Main")
            .unwrap()
            .content_hash
    };
    assert_ne!(
        hash("fn Main() {\n    let a = [1];\n    Print(a[0]);\n}\n"),
        hash("fn Main() {\n    let a = [2];\n    Print(a[0]);\n}\n")
    );
    assert_ne!(
        hash("fn Main() {\n    let a = [1, 2];\n    Print(a[0]);\n}\n"),
        hash("fn Main() {\n    let a = [1, 2];\n    Print(a[1]);\n}\n")
    );
    assert_eq!(
        hash("fn Main() {\n    let a = [1, 2];\n    Print(a[0]);\n}\n"),
        hash("fn Main() {\n    let a = [1, 2];\n    Print(a[0]);\n}\n")
    );
}
#[test]
fn binary_instruction_hash_matches_fixed_canonical_fnv_encoding() {
    // Independently encoded: Unit result, zero locals, six instructions
    // (1, 2, Add, Print, Drop, implicit Unit), empty cleanup, return temp 4.
    let p = compile_scalar("h.n", "fn Main() {\n    Print(1 + 2);\n}\n").unwrap();
    assert_eq!(
        p.functions().function("Main").unwrap().content_hash,
        0x20c7_71da_7f5d_f960
    );
}
#[test]
fn ordinary_resource_parameter_borrows_across_frame_and_closes_once() {
    let e = execute(
        "fn Leer(r: view MockResource) {\n    Inspect(r);\n    Inspect(r);\n}\nfn Main() {\n    let r = MockOpen(1);\n    Leer(r);\n    Inspect(r);\n}\n",
    );
    assert_eq!(e.outcome, Outcome::Unit);
    assert_eq!(
        e.observations.events(),
        [
            "open:0:1",
            "inspect:0:1",
            "inspect:0:1",
            "inspect:0:1",
            "close:0:1"
        ]
    );
    assert_eq!(e.observations.reserved_bytes(), 0);
}
#[test]
fn consumed_resource_parameter_closes_at_callee_exit() {
    let e = execute(
        "fn Take(r: move MockResource) {\n    Inspect(r);\n}\nfn Main() {\n    let r = MockOpen(4);\n    Take(move r);\n}\n",
    );
    assert_eq!(e.outcome, Outcome::Unit);
    assert_eq!(
        e.observations.events(),
        ["open:0:4", "inspect:0:4", "close:0:4"]
    );
}
#[test]
fn resource_result_is_transferred_out_of_callee_before_cleanup() {
    let e = execute(
        "fn Make() -> MockResource {\n    return MockOpen(3);\n}\nfn Main() {\n    let r = Make();\n    Inspect(r);\n}\n",
    );
    assert_eq!(e.outcome, Outcome::Unit);
    assert_eq!(
        e.observations.events(),
        ["open:0:3", "inspect:0:3", "close:0:3"]
    );
}
#[test]
fn later_argument_fault_cleans_earlier_resource_without_entering_callee() {
    let e = execute(
        "fn F(r: move MockResource, n: view I64) {\n    Print(n);\n}\nfn Main() {\n    F(MockOpen(1), 9223372036854775807 + 1);\n}\n",
    );
    assert_eq!(e.outcome, Outcome::SimulatorFault);
    assert_eq!(e.diagnostics[0].code, "R0101");
    assert_eq!(e.observations.events(), ["open:0:1", "close:0:1"]);
    assert!(e.observations.stdout().is_empty());
}
#[test]
fn named_arguments_evaluate_in_written_order() {
    let e = execute(
        "fn Mark(n: view I64) -> I64 {\n    Print(n);\n    return n;\n}\nfn F(a: view I64, b: view I64) {}\nfn Main() {\n    F(b: Mark(2), a: Mark(1));\n}\n",
    );
    assert_eq!(e.outcome, Outcome::Unit);
    assert_eq!(e.observations.stdout(), ["2", "1"]);
}
#[test]
fn resource_copy_is_rejected_and_explicit_transfer_runs() {
    let negative = "fn Main() {\n    let r = MockOpen(1);\n    let s = r;\n}\n";
    let Err(CompileError::Rejected(errors)) = compile_scalar("p.n", negative) else {
        panic!("expected ownership diagnostic")
    };
    assert_eq!(errors.len(), 1);
    let d = &errors[0];
    assert_eq!(
        (
            &*d.code,
            d.subcause.as_deref(),
            d.stage.as_str(),
            d.operation.as_str()
        ),
        ("NX-E0201", None, "ownership", "value.copy")
    );
    assert_eq!(d.primary_span.as_ref().map(|s| (s.start, s.end)), {
        let start = byte_span(negative, "let s = r", 0).0 + "let s = ".len() as u64;
        Some((start, start + 1))
    });
    d.validate("p.n", negative).unwrap();
    let e =
        execute("fn Main() {\n    let r = MockOpen(1);\n    let s = move r;\n    Inspect(s);\n}\n");
    assert_eq!(
        e.observations.events(),
        ["open:0:1", "inspect:0:1", "close:0:1"]
    );
}
#[test]
fn use_after_consume_or_drop_reports_consumption_site() {
    for (source, primary, consumption) in [
        (
            "fn Main() {\n    let r = MockOpen(1);\n    let s = move r;\n    Inspect(r);\n}\n",
            ("Inspect(r)", "r"),
            "move r",
        ),
        (
            "fn Main() {\n    let x = 1;\n    Drop(x);\n    Print(x);\n}\n",
            ("Print(x)", "x"),
            "Drop(x)",
        ),
    ] {
        let Err(CompileError::Rejected(errors)) = compile_scalar("p.n", source) else {
            panic!("expected ownership diagnostic")
        };
        assert_eq!(errors.len(), 1);
        let d = &errors[0];
        assert_eq!(
            (
                &*d.code,
                d.subcause.as_deref(),
                d.stage.as_str(),
                d.operation.as_str()
            ),
            (
                "NX-E0202",
                Some("definitely_consumed"),
                "ownership",
                "ownership.use"
            )
        );
        assert_eq!(d.primary_span.as_ref().map(|s| (s.start, s.end)), {
            let start =
                byte_span(source, primary.0, 0).0 + (primary.0.len() - primary.1.len() - 1) as u64;
            Some((start, start + 1))
        });
        assert!(
            d.related_spans
                .iter()
                .any(|r| r.role == crate::diagnostic::RelatedRole::Consumption
                    && (r.span.start, r.span.end) == byte_span(source, consumption, 0))
        );
        d.validate("p.n", source).unwrap();
    }
}
#[test]
fn read_parameter_cannot_be_consumed() {
    let source =
        "fn F(x: view MockResource) -> MockResource {\n    return move x;\n}\nfn Main() {}\n";
    let Err(CompileError::Rejected(errors)) = compile_scalar("p.n", source) else {
        panic!("expected permission diagnostic")
    };
    assert_eq!(errors.len(), 1);
    let d = &errors[0];
    assert_eq!(
        (
            &*d.code,
            d.subcause.as_deref(),
            d.stage.as_str(),
            d.operation.as_str()
        ),
        (
            "NX-E1105",
            Some("permission_escalation"),
            "ownership",
            "ownership.access"
        )
    );
    d.validate("p.n", source).unwrap();
    let e = execute(
        "fn F(x: move MockResource) -> MockResource {\n    return move x;\n}\nfn Main() {\n    let r = F(MockOpen(1));\n    Inspect(r);\n}\n",
    );
    assert_eq!(
        e.observations.events(),
        ["open:0:1", "inspect:0:1", "close:0:1"]
    );
}
#[test]
fn adr_gap_call_frame_limit_primary_uses_rejected_call_span() {
    let source = "fn Down() {\n    Down();\n}\nfn Main() {\n    Down();\n}\n";
    let e = execute(source);
    assert_eq!(e.outcome, Outcome::SimulatorFault);
    let d = &e.diagnostics[0];
    assert_eq!(d.code, "R0002");
    assert_eq!(
        d.data.get("runtime_budget_kind"),
        Some(&crate::diagnostic::DataValue::String("call_frames".into()))
    );
    assert_eq!(
        d.data.get("limit"),
        Some(&crate::diagnostic::DataValue::UInt(256))
    );
    assert_eq!(
        d.data.get("observed"),
        Some(&crate::diagnostic::DataValue::UInt(257))
    );
    assert_eq!(
        d.primary_span.as_ref().map(|s| (s.start, s.end)),
        Some(byte_span(source, "Down()", 1))
    );
    d.validate("fixture.n", source).unwrap();
}
#[test]
fn unreachable_tail_is_checked_without_emitting_its_instructions() {
    let source = "fn Main() {\n    return;\n    let a = [1];\n    a[0] = 2;\n}\n";
    let Err(CompileError::Rejected(errors)) = compile_scalar("dead.n", source) else {
        panic!("expected ownership diagnostic")
    };
    assert_eq!(errors.len(), 1);
    assert_eq!(errors[0].code, "NX-E0105");
    errors[0].validate("dead.n", source).unwrap();
    let e = execute("fn Main() {\n    return;\n    Print(9);\n}\n");
    assert_eq!(e.outcome, Outcome::Unit);
    assert!(e.observations.stdout().is_empty());
}
#[test]
fn shadowed_resources_close_in_reverse_scope_order() {
    let e = execute(
        "fn Main() {\n    let r = MockOpen(1);\n    {\n        let r = MockOpen(2);\n        Inspect(r);\n    }\n    Inspect(r);\n}\n",
    );
    assert_eq!(e.outcome, Outcome::Unit);
    assert_eq!(
        e.observations.events(),
        [
            "open:0:1",
            "open:1:2",
            "inspect:1:2",
            "close:1:2",
            "inspect:0:1",
            "close:0:1"
        ]
    );
}
#[test]
fn dropped_var_can_be_reinitialized_without_closing_twice() {
    let e = execute(
        "fn Main() {\n    var r = MockOpen(1);\n    Drop(r);\n    r = MockOpen(2);\n    Inspect(r);\n}\n",
    );
    assert_eq!(e.outcome, Outcome::Unit);
    assert_eq!(
        e.observations.events(),
        [
            "open:0:1",
            "close:0:1",
            "open:1:2",
            "inspect:1:2",
            "close:1:2"
        ]
    );
}
#[test]
fn ordinary_array_parameter_returns_deep_copy() {
    let e = execute(
        "fn Copia(a: view Array<I64>) -> Array<I64> {\n    return a;\n}\nfn Main() {\n    var a = [1];\n    let b = Copia(a);\n    a[0] = 2;\n    AssertEq(b, [1]);\n}\n",
    );
    assert_eq!(e.outcome, Outcome::Unit);
    assert!(e.observations.observations().is_empty());
}
#[test]
fn call_target_named_value_continues_across_lf() {
    let source = "fn Value(x: view I64) -> I64 {\n    return x;\n}\nfn Main() {\n    let x = 7;\n    let y = Value(x);\n    AssertEq(y, 7);\n}\n";
    let ast = parse::parse("fixture.n", source).unwrap();
    let crate::frontend::parse::Statement::Let {
        value: crate::frontend::parse::Expr::Call { span, .. },
        ..
    } = &ast
        .functions
        .iter()
        .find(|f| f.name == "Main")
        .unwrap()
        .body
        .statements[1]
    else {
        panic!("L03 call")
    };
    assert_eq!((span.start, span.end), byte_span(source, "Value(x)", 0));
    let e = execute(source);
    assert_eq!(e.outcome, Outcome::Unit);
    assert!(e.observations.observations().is_empty());
}
#[test]
fn explicit_return_parenthesis_continues_across_lf() {
    let source = "fn F() -> I64 {\n    return (1);\n}\nfn Main() {\n    AssertEq(F(), 1);\n}\n";
    let e = execute(source);
    assert_eq!(e.outcome, Outcome::Unit);
    assert!(e.observations.observations().is_empty());
}
#[test]
fn drop_destroys_complete_binding_or_temporary_and_rejects_projection() {
    let e = execute(
        "fn Main() {\n    let a = [1];\n    Drop(a);\n    Drop([1][0]);\n    Drop(MockOpen(9));\n}\n",
    );
    assert_eq!(e.outcome, Outcome::Unit);
    assert_eq!(e.observations.events(), ["open:0:9", "close:0:9"]);
    let source = "fn Main() {\n    let a = [1];\n    Drop(a[0]);\n}\n";
    let Err(CompileError::Rejected(errors)) = compile_scalar("drop.n", source) else {
        panic!("expected partial transfer rejection")
    };
    assert_eq!(errors.len(), 1);
    assert_eq!(
        (
            &*errors[0].code,
            errors[0].feature_id.as_deref(),
            errors[0].stage.as_str()
        ),
        ("NX-E0006", Some("partial_transfer"), "arguments")
    );
    errors[0].validate("drop.n", source).unwrap();
}
#[test]
fn caller_hash_tracks_referenced_name_and_signature_but_not_unrelated_body() {
    let hash = |source: &str| {
        compile_scalar("hash.n", source)
            .unwrap()
            .functions()
            .function("Main")
            .unwrap()
            .content_hash
    };
    let base = hash("fn F(x: view I64) -> I64 {\n    return x;\n}\nfn Main() {\n    F(1);\n}\n");
    assert_eq!(
        base,
        hash("fn F(x: view I64) -> I64 {\n    return x + 1;\n}\nfn Main() {\n    F(1);\n}\n")
    );
    assert_ne!(
        base,
        hash("fn G(x: view I64) -> I64 {\n    return x;\n}\nfn Main() {\n    G(1);\n}\n")
    );
    assert_ne!(
        base,
        hash("fn F(y: view I64) -> I64 {\n    return y;\n}\nfn Main() {\n    F(1);\n}\n")
    );
    assert_ne!(
        base,
        hash("fn F(x: view I64) -> Bool {\n    return true;\n}\nfn Main() {\n    F(1);\n}\n")
    );
    assert_ne!(
        base,
        hash("fn F(x: move I64) -> I64 {\n    return x;\n}\nfn Main() {\n    F(1);\n}\n")
    );
}
#[test]
fn parameter_name_and_argument_slot_are_encoded_in_canonical_hash() {
    let a = compile_scalar(
        "hash.n",
        "fn F(x: view I64) -> I64 {\n    return 1;\n}\nfn Main() {\n    F(2);\n}\n",
    )
    .unwrap();
    let b = compile_scalar(
        "hash.n",
        "fn F(y: view I64) -> I64 {\n    return 1;\n}\nfn Main() {\n    F(2);\n}\n",
    )
    .unwrap();
    assert_ne!(
        a.functions().function("F").unwrap().content_hash,
        b.functions().function("F").unwrap().content_hash
    );
    let p = compile_scalar(
        "hash.n",
        "fn F(a: view I64, b: view I64) {}\nfn Main() {\n    F(a: 1, b: 2);\n}\n",
    )
    .unwrap();
    let mut f = p.functions().function("Main").unwrap().clone();
    let original = super::model::content_hash(&f);
    let call = f
        .block
        .instructions
        .iter_mut()
        .find(|i| matches!(i.op, super::model::Op::CallDirect { .. }))
        .unwrap();
    if let super::model::Op::CallDirect { args, .. } = &mut call.op {
        args.swap(0, 1);
    }
    assert_ne!(original, super::model::content_hash(&f));
    let mut f = p.functions().function("Main").unwrap().clone();
    let call = f
        .block
        .instructions
        .iter_mut()
        .find(|i| matches!(i.op, super::model::Op::CallDirect { .. }))
        .unwrap();
    if let super::model::Op::CallDirect { args, .. } = &mut call.op {
        args[0].0 = 9;
    }
    assert_ne!(original, super::model::content_hash(&f));
}
#[test]
fn canonical_hash_distinguishes_transfer_from_copy_on_same_slot() {
    let p = compile_scalar(
        "hash.n",
        "fn Main() {\n    let x = 1;\n    let y = move x;\n}\n",
    )
    .unwrap();
    let mut f = p.functions().function("Main").unwrap().clone();
    let original = super::model::content_hash(&f);
    let ins = f
        .block
        .instructions
        .iter_mut()
        .find(|i| matches!(i.op, super::model::Op::TransferValue(_)))
        .unwrap();
    ins.op = super::model::Op::CopyValue(super::model::LocalId(0));
    assert_ne!(original, super::model::content_hash(&f));
}
#[test]
fn direct_call_hash_has_fixed_canonical_vector() {
    // F0-D15 does not publish a hash vector. Pin the current semantic IR
    // encoding for a direct call to the B-spelled function F.
    let p = compile_scalar(
        "hash.n",
        "fn F(x: view I64) {}\nfn Main() {\n    F(1);\n}\n",
    )
    .unwrap();
    assert_eq!(
        p.functions().function("Main").unwrap().content_hash,
        0x1dee_b608_6d9e_e0aa
    );
}
#[test]
fn failed_development_substitution_restores_previous_function() {
    let mut receiver =
        compile_scalar("r.n", "fn Extra() {}\nfn Main() {\n    Print(1);\n}\n").unwrap();
    let donor = compile_scalar("d.n", "fn Helper() {}\nfn Main() {\n    Helper();\n}\n").unwrap();
    let before = receiver.functions().function("Main").unwrap().content_hash;
    assert_eq!(
        receiver.substitute_for_development("Main", &donor),
        Err("unknown callee")
    );
    assert_eq!(
        receiver.functions().function("Main").unwrap().content_hash,
        before
    );
    let e = run_scalar(&receiver);
    assert_eq!(e.observations.stdout(), ["1"]);
}
#[test]
fn independent_ownership_errors_accumulate_in_source_order() {
    let source = "fn Main() {\n    let a = MockOpen(1);\n    let b = MockOpen(2);\n    let x = a;\n    let y = b;\n}\n";
    let Err(CompileError::Rejected(errors)) = compile_scalar("multi.n", source) else {
        panic!("expected two copies")
    };
    assert_eq!(errors.len(), 2);
    assert_eq!(
        errors.iter().map(|d| d.code.as_str()).collect::<Vec<_>>(),
        ["NX-E0201", "NX-E0201"]
    );
    assert_eq!(
        errors
            .iter()
            .map(|d| d.primary_span.as_ref().unwrap().start)
            .collect::<Vec<_>>(),
        [
            byte_span(source, "let x = a", 0).0 + 8,
            byte_span(source, "let y = b", 0).0 + 8,
        ]
    );
    for d in errors {
        d.validate("multi.n", source).unwrap();
    }
}
#[test]
fn user_function_named_drop_has_ordinary_read_semantics() {
    let e = execute(
        "fn Drop(x: view I64) {}\nfn Main() {\n    let a = [1];\n    Drop(a[0]);\n    AssertEq(a[0], 1);\n}\n",
    );
    assert_eq!(e.outcome, Outcome::Unit);
    assert!(e.observations.observations().is_empty());
}
#[test]
fn consumed_temporary_borrowed_by_read_parameter_closes_after_call() {
    let e = execute(
        "fn Read(r: view MockResource) {\n    Inspect(r);\n}\nfn Main() {\n    let r = MockOpen(5);\n    Read(move r);\n}\n",
    );
    assert_eq!(e.outcome, Outcome::Unit);
    assert_eq!(
        e.observations.events(),
        ["open:0:5", "inspect:0:5", "close:0:5"]
    );
    let source = "fn Read(r: view MockResource) {}\nfn Main() {\n    let r = MockOpen(5);\n    Read(move r);\n    Inspect(r);\n}\n";
    let Err(CompileError::Rejected(errors)) = compile_scalar("consume.n", source) else {
        panic!("expected consumed use")
    };
    assert_eq!(errors.len(), 1);
    assert_eq!(
        (&*errors[0].code, errors[0].subcause.as_deref()),
        ("NX-E0202", Some("definitely_consumed"))
    );
    errors[0].validate("consume.n", source).unwrap();
}
#[test]
fn simulator_fault_cleans_callee_then_caller_resources() {
    let e = execute(
        "fn Failer(r: move MockResource) {\n    let s = MockOpen(2);\n    Fail();\n}\nfn Main() {\n    let r = MockOpen(1);\n    Failer(move r);\n}\n",
    );
    assert_eq!(e.outcome, Outcome::SimulatorFault);
    assert_eq!(e.diagnostics[0].code, "R0301");
    assert_eq!(
        e.observations.events(),
        ["open:0:1", "open:1:2", "close:1:2", "close:0:1"]
    );
}
#[test]
fn blocking_resource_has_same_event_shapes_and_one_close() {
    let e = execute("fn Main() {\n    let r = MockBlockingOpen(7);\n    Inspect(r);\n}\n");
    assert_eq!(e.outcome, Outcome::Unit);
    assert_eq!(
        e.observations.events(),
        ["open:0:7", "inspect:0:7", "close:0:7"]
    );
}
#[test]
fn frame_limit_unwinds_shared_resource_once() {
    let source = "fn F(r: view MockResource) {\n    F(r);\n}\nfn Main() {\n    let r = MockOpen(1);\n    F(r);\n}\n";
    let e = execute(source);
    assert_eq!(e.outcome, Outcome::SimulatorFault);
    assert_eq!(e.diagnostics[0].code, "R0002");
    assert_eq!(
        e.diagnostics[0].data.get("runtime_budget_kind"),
        Some(&crate::diagnostic::DataValue::String("call_frames".into()))
    );
    assert_eq!(e.observations.events(), ["open:0:1", "close:0:1"]);
}
#[test]
fn sequential_calls_release_frames_before_the_next_entry() {
    let source = format!("fn F(){{}} fn Main(){{{}}}", "F();".repeat(300));
    let e = execute(&source);
    assert_eq!(e.outcome, Outcome::Unit);
    assert!(e.diagnostics.is_empty());
    assert!(e.observations.observations().is_empty());
}
#[test]
fn frame_limit_denies_acquisition_in_frame_257() {
    let e = execute("fn F() {\n    let r = MockOpen(1);\n    F();\n}\nfn Main() {\n    F();\n}\n");
    assert_eq!(e.outcome, Outcome::SimulatorFault);
    assert_eq!(e.diagnostics[0].code, "R0002");
    assert_eq!(
        e.observations
            .events()
            .iter()
            .filter(|event| event.starts_with("open:"))
            .count(),
        255
    );
    assert_eq!(
        e.observations
            .events()
            .iter()
            .filter(|event| event.starts_with("close:"))
            .count(),
        255
    );
    assert_eq!(e.observations.events().first().copied(), Some("open:0:1"));
    assert_eq!(e.observations.events().last().copied(), Some("close:0:1"));
}
#[test]
fn complete_resource_reassignment_closes_old_before_scope_exit() {
    let e = execute(
        "fn Main() {\n    var r = MockOpen(1);\n    r = MockOpen(2);\n    Inspect(r);\n}\n",
    );
    assert_eq!(e.outcome, Outcome::Unit);
    assert_eq!(
        e.observations.events(),
        [
            "open:0:1",
            "open:1:2",
            "close:0:1",
            "inspect:1:2",
            "close:1:2"
        ]
    );
}
#[test]
fn selected_array_copy_is_independent_of_mutable_root() {
    let e = execute(
        "fn Main() {\n    var a = [[1]];\n    let b = a[0];\n    a[0][0] = 2;\n    AssertEq(b, [1]);\n}\n",
    );
    assert_eq!(e.outcome, Outcome::Unit);
    assert!(e.diagnostics.is_empty());
}
#[test]
fn repeated_drop_reports_consumed_origin_without_second_close() {
    let source = "fn Main() {\n    let r = MockOpen(1);\n    Drop(r);\n    Drop(r);\n}\n";
    let Err(CompileError::Rejected(errors)) = compile_scalar("drop.n", source) else {
        panic!("expected consumed use")
    };
    assert_eq!(errors.len(), 1);
    assert_eq!(
        (&*errors[0].code, errors[0].subcause.as_deref()),
        ("NX-E0202", Some("definitely_consumed"))
    );
    errors[0].validate("drop.n", source).unwrap();
}
#[test]
fn read_parameter_rejects_write_and_drop_without_execution() {
    for (source, access) in [
        (
            "fn F(x: view I64) {\n    x = 2;\n}\nfn Main() {}\n",
            "write",
        ),
        (
            "fn F(x: view I64) {\n    Drop(x);\n}\nfn Main() {}\n",
            "drop",
        ),
    ] {
        let Err(CompileError::Rejected(errors)) = compile_scalar("permission.n", source) else {
            panic!("expected {access} rejection")
        };
        assert_eq!(errors.len(), 1);
        assert_eq!(
            (&*errors[0].code, errors[0].subcause.as_deref()),
            ("NX-E1105", Some("permission_escalation"))
        );
        assert_eq!(
            errors[0].data.get("requested_access"),
            Some(&crate::diagnostic::DataValue::String(access.into()))
        );
        errors[0].validate("permission.n", source).unwrap();
    }
}
#[test]
fn consumed_nested_local_does_not_receive_second_cleanup() {
    let e = execute(
        "fn Take(r: move MockResource) {}\nfn Main() {\n    {\n        let r = MockOpen(1);\n        Take(move r);\n    }\n}\n",
    );
    assert_eq!(e.outcome, Outcome::Unit);
    assert_eq!(e.observations.events(), ["open:0:1", "close:0:1"]);
}
#[test]
fn nested_drop_and_reinitialization_use_one_liveness_record() {
    let e = execute(
        "fn Main() {\n    {\n        var r = MockOpen(1);\n        Drop(r);\n        r = MockOpen(2);\n    }\n}\n",
    );
    assert_eq!(e.outcome, Outcome::Unit);
    assert_eq!(
        e.observations.events(),
        ["open:0:1", "close:0:1", "open:1:2", "close:1:2"]
    );
}
#[test]
fn i05_source_rows_have_isolated_rejections_and_executable_repairs() {
    let rows: &[(&str, &str, &str, Option<&str>, &str)] = &[
        (
            "S01",
            "fn F(x: view I64) {}\nfn Main() {\n    F();\n}\n",
            "NX-E0111",
            Some("missing_argument"),
            "fn F(x: view I64) {}\nfn Main() {\n    F(1);\n}\n",
        ),
        (
            "S02",
            "fn F(x: view I64) {}\nfn Main() {\n    F(1, 2);\n}\n",
            "NX-E0111",
            Some("excess_positional_argument"),
            "fn F(x: view I64) {}\nfn Main() {\n    F(1);\n}\n",
        ),
        (
            "S03",
            "fn F(x: view I64) {}\nfn Main() {\n    F(y: 1);\n}\n",
            "NX-E0111",
            Some("unknown_argument_label"),
            "fn F(x: view I64) {}\nfn Main() {\n    F(x: 1);\n}\n",
        ),
        (
            "S04",
            "fn F(x: view I64) {}\nfn Main() {\n    F(x: 1, x: 2);\n}\n",
            "NX-E0111",
            Some("duplicate_argument"),
            "fn F(x: view I64) {}\nfn Main() {\n    F(x: 1);\n}\n",
        ),
        (
            "S05",
            "fn F(x: view I64, y: view I64) {}\nfn Main() {\n    F(x: 1, 2);\n}\n",
            "NX-E0111",
            Some("positional_after_named"),
            "fn F(x: view I64, y: view I64) {}\nfn Main() {\n    F(x: 1, y: 2);\n}\n",
        ),
        (
            "S07",
            "fn Main() {\n    let f = 1;\n    f();\n}\n",
            "NX-E0111",
            Some("non_callable_target"),
            "fn F() {}\nfn Main() {\n    F();\n}\n",
        ),
        (
            "S12",
            "fn F() -> I64 {\n    return true;\n}\nfn Main() {}\n",
            "NX-E0102",
            Some("type_mismatch"),
            "fn F() -> I64 {\n    return 1;\n}\nfn Main() {\n    AssertEq(F(), 1);\n}\n",
        ),
        (
            "S13",
            "fn F() {}\nfn F() {}\nfn Main() {}\n",
            "NX-E0104",
            None,
            "fn F() {}\nfn G() {}\nfn Main() {}\n",
        ),
        (
            "S14",
            "fn F(x: view I64, x: view I64) {}\nfn Main() {}\n",
            "NX-E0104",
            None,
            "fn F(x: view I64, y: view I64) {}\nfn Main() {\n    F(1, 2);\n}\n",
        ),
        (
            "S15",
            "fn Main() {\n    ausente();\n}\n",
            "NX-E0101",
            None,
            "fn Ausente() {}\nfn Main() {\n    Ausente();\n}\n",
        ),
        (
            "S19",
            "fn Main() {\n    let r = MockOpen(1);\n    let b = r == r;\n}\n",
            "NX-E0102",
            Some("type_mismatch"),
            "fn Main() {\n    let r = MockOpen(1);\n    Inspect(r);\n}\n",
        ),
        (
            "S24",
            "fn F(x: view I64) {\n    let x = 1;\n}\nfn Main() {}\n",
            "NX-E0104",
            None,
            "fn F(x: view I64) {\n    {\n        let x = 1;\n    }\n}\nfn Main() {\n    F(1);\n}\n",
        ),
        (
            "S26",
            "fn Main() {\n    let x = 1;\n    Inspect(x);\n}\n",
            "NX-E0102",
            Some("resource_required"),
            "fn Main() {\n    let x = MockOpen(1);\n    Inspect(x);\n}\n",
        ),
        (
            "S29",
            "fn Main() {\n    let a = [1];\n    Drop(a[0]);\n}\n",
            "NX-E0006",
            None,
            "fn Main() {\n    let a = [1];\n    Drop(a);\n}\n",
        ),
        (
            "B07",
            "fn F()->I64{return 1 } fn Main(){}",
            "NX-E0005",
            None,
            "fn F() -> I64 {\n    return 1;\n}\nfn Main() {\n    AssertEq(F(), 1);\n}\n",
        ),
    ];
    for (id, negative, code, subcause, positive) in rows {
        let Err(CompileError::Rejected(errors)) = compile_scalar("rows.n", negative) else {
            panic!("{id} negative accepted")
        };
        assert_eq!(errors.len(), 1, "{id}");
        assert_eq!(
            (&*errors[0].code, errors[0].subcause.as_deref()),
            (*code, *subcause),
            "{id}"
        );
        errors[0].validate("rows.n", negative).unwrap();
        let run = execute(positive);
        assert_eq!(run.outcome, Outcome::Unit, "{id}");
        assert!(run.diagnostics.is_empty(), "{id}");
    }
}
#[test]
fn i05_source_rows_keep_published_primary_byte_spans_and_stages() {
    // Marked operands and stages are transcribed from F0-0002 §§14.1–14.3, 14.9.
    let rows = [
        (
            "S01",
            "fn F(x:view I64){} fn Main(){⟦F⟧();}",
            "arguments",
            "call.bind",
        ),
        (
            "S02",
            "fn F(x:view I64){} fn Main(){F(1,⟦2⟧);}",
            "arguments",
            "call.bind",
        ),
        (
            "S03",
            "fn F(x:view I64){} fn Main(){F(⟦y⟧:1);}",
            "arguments",
            "call.bind",
        ),
        (
            "S04",
            "fn F(x:view I64){} fn Main(){F(x:1,⟦x⟧:2);}",
            "arguments",
            "call.bind",
        ),
        (
            "S05",
            "fn F(x:view I64,y:view I64){} fn Main(){F(x:1,⟦2⟧);}",
            "arguments",
            "call.bind",
        ),
        ("S07", "fn Main(){let f=1;⟦f⟧();}", "arguments", "call.bind"),
        (
            "S12",
            "fn F()->I64{return ⟦true⟧;} fn Main(){}",
            "types",
            "types.check",
        ),
        (
            "S13",
            "fn F(){} fn ⟦F⟧(){} fn Main(){}",
            "names",
            "names.declare",
        ),
        (
            "S14",
            "fn F(x:view I64,⟦x⟧:view I64){} fn Main(){}",
            "names",
            "names.declare",
        ),
        ("S15", "fn Main(){⟦Ausente⟧();}", "names", "names.resolve"),
        (
            "S19",
            "fn Main(){let r=MockOpen(1);let b=r⟦==⟧r;}",
            "types",
            "types.check",
        ),
        (
            "S24",
            "fn F(x:view I64){let ⟦x⟧=1;} fn Main(){}",
            "names",
            "names.declare",
        ),
        (
            "S26",
            "fn Main(){let x=1;Inspect(⟦x⟧);}",
            "types",
            "types.check",
        ),
        (
            "S29",
            "fn Main(){let a=[1];Drop(⟦a[0]⟧);}",
            "arguments",
            "call.bind",
        ),
        (
            "B07",
            "fn F()->I64{return 1 ⟦}⟧ fn Main(){}",
            "parse",
            "syntax.parse",
        ),
    ];
    // Primary byte intervals and rule identities are literal F0-D15 §8 rows.
    let published = [
        (29, 30, "NX-E0111", Some("missing_argument")),
        (33, 34, "NX-E0111", Some("excess_positional_argument")),
        (31, 32, "NX-E0111", Some("unknown_argument_label")),
        (35, 36, "NX-E0111", Some("duplicate_argument")),
        (46, 47, "NX-E0111", Some("positional_after_named")),
        (18, 19, "NX-E0111", Some("non_callable_target")),
        (19, 23, "NX-E0102", Some("type_mismatch")),
        (12, 13, "NX-E0104", None),
        (16, 17, "NX-E0104", None),
        (10, 17, "NX-E0101", None),
        (35, 37, "NX-E0102", Some("type_mismatch")),
        (21, 22, "NX-E0104", None),
        (26, 27, "NX-E0102", Some("resource_required")),
        (25, 29, "NX-E0006", None),
        (21, 22, "NX-E0005", None),
    ];
    for ((id, marked, stage, operation), (start, end, code, subcause)) in
        rows.into_iter().zip(published)
    {
        let (before, rest) = marked.split_once('⟦').unwrap();
        let (primary, after) = rest.split_once('⟧').unwrap();
        let source = format!("{before}{primary}{after}");
        assert_eq!(
            (before.len() as u64, (before.len() + primary.len()) as u64),
            (start, end),
            "{id}"
        );
        let Err(CompileError::Rejected(errors)) = compile_scalar("rows.n", &source) else {
            panic!("{id} accepted")
        };
        assert_eq!(errors.len(), 1, "{id}");
        let d = &errors[0];
        assert_eq!(
            d.primary_span.as_ref().map(|s| (s.start, s.end)),
            Some((start, end)),
            "{id}"
        );
        assert_eq!(
            (d.code.as_str(), d.subcause.as_deref()),
            (code, subcause),
            "{id}"
        );
        assert_eq!(
            (d.stage.as_str(), d.operation.as_str()),
            (stage, operation),
            "{id}"
        );
        d.validate("rows.n", &source).unwrap();
    }
}
#[test]
fn i05_feature_exclusions_have_executable_repairs() {
    // F2-F01 is accepted in B: move is the official transfer mode.
    let transfer = "fn Main(){let r=MockOpen(1);let s=move r;}";
    let result = execute(transfer);
    assert_eq!(result.outcome, Outcome::Unit);
    assert!(result.diagnostics.is_empty());
    let rows: &[(&str, &str, &str, &str)] = &[
        (
            "F02",
            "fn F(){return 1;} fn Main(){}",
            "inferred_non_unit_result",
            "fn F() -> I64 {\n    return 1;\n}\nfn Main() {\n    AssertEq(F(), 1);\n}\n",
        ),
        (
            "F07",
            "fn F(x:view I64=1){} fn Main(){F(1);}",
            "default_argument",
            "fn F(x: view I64) {}\nfn Main() {\n    F(1);\n}\n",
        ),
        (
            "F09",
            "fn F(){} fn Main(){let g=F;}",
            "function_value",
            "fn F() {}\nfn Main() {\n    F();\n}\n",
        ),
        (
            "F10",
            "fn F(){} fn Main(){move F();}",
            "receiver_mode",
            "fn F() {}\nfn Main() {\n    F();\n}\n",
        ),
        (
            "F11",
            "fn Main(){let a=[1];let x=move a[0];}",
            "partial_transfer",
            "fn Main() {\n    let a = [1];\n    let x = a[0];\n    AssertEq(x, 1);\n}\n",
        ),
        (
            "F12",
            "fn Main(){let a=[MockOpen(1)];}",
            "resource_array",
            "fn Main() {\n    let a = MockOpen(1);\n    Inspect(a);\n}\n",
        ),
        (
            "F20",
            "fn F(x:view I64)->I64 from(x){return x;} fn Main(){}",
            "provenance_clauses",
            "fn F(x: view I64) -> I64 {\n    return x;\n}\nfn Main() {\n    AssertEq(F(1), 1);\n}\n",
        ),
        (
            "F29",
            "fn F(x:view U32){} fn Main(){}",
            "unsupported_type",
            "fn F(x: view I64) {}\nfn Main() {\n    F(1);\n}\n",
        ),
        (
            "F30",
            "fn F(x:view String){} fn Main(){}",
            "unsupported_type",
            "fn F(x: view Array<I64>) {}\nfn Main() {\n    F([1]);\n}\n",
        ),
        (
            "F31",
            "fn F(x:view app.Error){} fn Main(){}",
            "unsupported_type",
            "fn F(x: view I64) {}\nfn Main() {\n    F(1);\n}\n",
        ),
    ];
    for (id, negative, feature, positive) in rows {
        let Err(CompileError::Rejected(errors)) = compile_scalar("features.n", negative) else {
            panic!("{id} accepted")
        };
        assert_eq!(errors.len(), 1, "{id}");
        assert_eq!(errors[0].code, "NX-E0006", "{id}");
        assert_eq!(errors[0].feature_id.as_deref(), Some(*feature), "{id}");
        errors[0].validate("features.n", negative).unwrap();
        let e = execute(positive);
        assert_eq!(e.outcome, Outcome::Unit, "{id}");
        assert!(e.diagnostics.is_empty(), "{id}");
    }
}
#[test]
fn hash_distinguishes_less_from_equal_with_same_operands() {
    let hash = |operator| {
        let source = format!("fn Main(){{Print(1{operator}2);}}");
        compile_scalar("h.n", &source)
            .unwrap()
            .functions()
            .function("Main")
            .unwrap()
            .content_hash
    };
    assert_ne!(hash("<"), hash("=="));
}
#[test]
fn function_hash_includes_local_mutability() {
    let hash = |source| {
        compile_scalar("h.n", source)
            .unwrap()
            .functions()
            .function("Main")
            .unwrap()
            .content_hash
    };
    assert_ne!(
        hash("fn Main() {\n    let x = 1;\n    Print(x);\n}\n"),
        hash("fn Main() {\n    var x = 1;\n    Print(x);\n}\n")
    );
}

#[test]
fn hash_includes_write_index_root_and_read_index_path() {
    let program = compile_scalar(
        "h.n",
        "fn Main() {\n    var a = [1, 2];\n    var b = [3, 4];\n    a[0] = 5;\n    let c = a[1];\n}\n",
    )
    .unwrap();
    let original = program.functions().function("Main").unwrap().clone();
    let baseline = super::model::content_hash(&original);
    let mut changed = original.clone();
    let write = changed
        .block
        .instructions
        .iter_mut()
        .find(|i| matches!(i.op, super::model::Op::WriteIndex { .. }))
        .unwrap();
    if let super::model::Op::WriteIndex { local, .. } = &mut write.op {
        *local = super::model::LocalId(1);
    }
    assert_ne!(super::model::content_hash(&changed), baseline);
    let mut changed = original;
    let read = changed
        .block
        .instructions
        .iter_mut()
        .find(|i| matches!(i.op, super::model::Op::ReadIndex { .. }))
        .unwrap();
    if let super::model::Op::ReadIndex { indexes, .. } = &mut read.op {
        indexes[0].0 += 1;
    }
    assert_ne!(super::model::content_hash(&changed), baseline);
}
#[test]
fn read_index_without_optional_early_probe_still_checks_bounds() {
    let mut p = compile_scalar(
        "h.n",
        "fn Main() {\n    let a = [0];\n    Print(a[2]);\n}\n",
    )
    .unwrap();
    let f = &mut p.functions.entries[0];
    f.block
        .instructions
        .retain(|i| !matches!(i.op, super::model::Op::CheckIndex { .. }));
    for (i, ins) in f.block.instructions.iter_mut().enumerate() {
        ins.origin = super::model::OriginId(i);
    }
    f.content_hash = super::model::content_hash(f);
    super::verify::verify_program(&p).unwrap();
    let e = run_scalar(&p);
    assert_eq!(e.outcome, Outcome::SimulatorFault);
    assert_eq!(
        (
            e.diagnostics[0].code.as_str(),
            e.diagnostics[0].operation.as_str()
        ),
        ("R0102", "value.index")
    );
}
#[test]
fn true_array_equality_and_nonempty_length_are_observed() {
    let e = execute(
        "fn Main() {\n    AssertEq([1, 2] == [1, 2], true);\n    AssertEq(Len([1, 2]), 2);\n}\n",
    );
    assert_eq!(e.outcome, Outcome::Unit);
    assert!(e.diagnostics.is_empty());
}
#[test]
fn annotated_empty_array_len_and_array_comparison_are_executable() {
    let source = "fn Main() {\n    let empty: Array<I64> = [];\n    AssertEq(Len(empty), 0);\n    let a = [1, 2];\n    AssertEq(a == [1, 2], true);\n    AssertEq(a < [1], false);\n}\n";
    let valid = source.replace("AssertEq(a < [1], false);", "AssertEq(Len(a), 2);");
    let e = execute(&valid);
    assert_eq!(e.outcome, Outcome::Unit);
    assert!(e.diagnostics.is_empty());
    assert!(matches!(
        compile_scalar("fixture.n", source),
        Err(CompileError::Rejected(_))
    ));
}
fn doubling_array_source(last: usize, tail: &str) -> String {
    let mut source = String::from("fn Main(){let a0=[0];");
    for k in 1..=last {
        source.push_str(&format!("let a{k}=[a{},a{}];", k - 1, k - 1));
    }
    source.push_str(tail);
    source.push('}');
    source
}
#[test]
fn ordinary_array_arguments_and_equality_borrow_without_live_cell_growth() {
    let source = doubling_array_source(
        13,
        "AssertEq(a13,a13);AssertEq(a13==a13,true);AssertEq(Len(a13),2);Print(a13);",
    );
    let e = execute(&source);
    assert_eq!(e.outcome, Outcome::Unit);
    assert!(e.diagnostics.is_empty());
    assert_eq!(e.observations.bytes(), 57_341);
}
#[test]
fn indexed_read_copies_selected_subtree_without_copying_whole_root() {
    let e = execute(&doubling_array_source(
        13,
        "let selected=a13[0];AssertEq(Len(selected),2);",
    ));
    assert_eq!(e.outcome, Outcome::Unit);
    assert!(e.diagnostics.is_empty());
}
#[test]
fn second_selected_subtree_copy_hits_live_cell_limit_without_partial_installation() {
    let e = execute(&doubling_array_source(
        13,
        "let first=a13[0];let second=a13[0];",
    ));
    assert_eq!(e.outcome, Outcome::SimulatorFault);
    let d = &e.diagnostics[0];
    assert_eq!(d.code, "R0002");
    assert_eq!(
        d.data.get("runtime_budget_kind"),
        Some(&crate::diagnostic::DataValue::String(
            "live_value_cells".into()
        ))
    );
    assert_eq!(
        d.data.get("observed"),
        Some(&crate::diagnostic::DataValue::UInt(73_709))
    );
}
#[test]
fn annotated_empty_array_can_replace_indexed_array_element() {
    let e = execute(
        "fn Main() {\n    var a = [[1]];\n    a[0] = [];\n    AssertEq(Len(a[0]), 0);\n}\n",
    );
    assert_eq!(e.outcome, Outcome::Unit);
}
#[test]
fn nested_replacement_updates_cached_cell_sizes_and_preserves_copies() {
    let e = execute(
        "fn Main() {\n    var a = [[[1]]];\n    a[0][0] = [2, 3];\n    let b = a;\n    a[0] = [[4]];\n    AssertEq(b, [[[2, 3]]]);\n    Print(a);\n}\n",
    );
    assert_eq!(e.outcome, Outcome::Unit);
    assert_eq!(e.observations.stdout(), vec!["[[[4]]]"]);
}
#[test]
fn live_value_cells_count_nested_headers_and_elements_before_copying() {
    let source = doubling_array_source(14, "");
    let e = execute(&source);
    assert_eq!(e.outcome, Outcome::SimulatorFault);
    assert_eq!(e.diagnostics.len(), 1);
    let d = &e.diagnostics[0];
    assert_eq!(
        (d.code.as_str(), d.stage.as_str(), d.operation.as_str()),
        ("R0002", "runtime", "runtime.limit")
    );
    assert_eq!(
        d.data.get("runtime_budget_kind"),
        Some(&crate::diagnostic::DataValue::String(
            "live_value_cells".into()
        ))
    );
    assert_eq!(
        d.data.get("limit"),
        Some(&crate::diagnostic::DataValue::UInt(65_536))
    );
    assert_eq!(
        d.data.get("observed"),
        Some(&crate::diagnostic::DataValue::UInt(73_710))
    );
    assert!(e.observations.stdout().is_empty());
}
#[test]
fn immutable_write_fixture_uses_current_source_byte_span() {
    // Keep the corpus program's ownership oracle here.
    let source = "// expect: E0105\nfn Main() { let value = [[1]]; value[0][0] = 2; }\n";
    let CompileError::Rejected(errors) = compile_scalar("immutable_write.n", source).unwrap_err()
    else {
        panic!("ownership rejection")
    };
    assert_eq!(errors.len(), 1);
    let d = &errors[0];
    assert_eq!(
        (
            d.code.as_str(),
            d.subcause.as_deref(),
            d.stage.as_str(),
            d.operation.as_str()
        ),
        ("NX-E0105", None, "ownership", "place.write")
    );
    assert_eq!(d.data.len(), 0);
    assert_eq!(d.related_spans.len(), 1);
    assert_eq!(d.related_spans[0].role.as_str(), "declaration");
    let start = source.find("value[0][0]").unwrap() as u64;
    assert_eq!(
        d.primary_span.as_ref().map(|s| (s.start, s.end)),
        Some((start, start + 5))
    );
    d.validate("immutable_write.n", source).unwrap();
}
#[test]
fn heterogeneous_array_rejection_has_homogeneous_positive() {
    let source = include_str!("../../corpus/fail/mixed_array.n");
    let CompileError::Rejected(errors) = compile_scalar("mixed_array.n", source).unwrap_err()
    else {
        panic!("types rejection")
    };
    assert_eq!(errors.len(), 1);
    assert_eq!(
        (
            errors[0].code.as_str(),
            errors[0].subcause.as_deref(),
            errors[0].stage.as_str()
        ),
        ("NX-E0102", Some("type_mismatch"), "types")
    );
    errors[0].validate("mixed_array.n", source).unwrap();
    let e = execute("fn Main() {\n    let a = [1, 2];\n    AssertEq(a, [1, 2]);\n}\n");
    assert_eq!(e.outcome, Outcome::Unit);
}
#[test]
fn array_fixtures_for_empty_array_comparison_index_and_len_have_complete_positives() {
    for source in [
        "fn Main() {\n    let xs: Array<I64> = [];\n    AssertEq(Len(xs), 0);\n}\n",
        "fn Main() {\n    let b = [1] == [2];\n    AssertEq(b, false);\n}\n",
        "fn Main() {\n    let x = [1][0];\n    AssertEq(x, 1);\n}\n",
        "fn Main() {\n    let a = [1];\n    Len(a);\n}\n",
    ] {
        let e = execute(source);
        assert_eq!(e.outcome, Outcome::Unit, "{source}");
        assert!(e.diagnostics.is_empty(), "{source}");
        assert!(e.observations.stdout().is_empty());
    }
}
#[test]
fn lexical_operator_repair_pairs_execute_for_lx01_through_lx05() {
    for symbol in ['/', '%', '!', '&', '|'] {
        let syntax_operator = matches!(symbol, '/' | '%');
        let negative = format!("fn Main(){{let x=1{symbol}2;}}");
        let CompileError::Rejected(errors) = compile_scalar("fixture.n", &negative).unwrap_err()
        else {
            panic!("operator rejection {symbol}")
        };
        assert_eq!(errors.len(), 1);
        let d = &errors[0];
        assert_eq!(
            (
                d.code.as_str(),
                d.subcause.as_deref(),
                d.stage.as_str(),
                d.operation.as_str()
            ),
            if syntax_operator {
                ("NX-E0002", None, "parse", "syntax.parse")
            } else {
                ("NX-E0001", None, "lex", "syntax.lex")
            }
        );
        assert_eq!(
            d.primary_span.as_ref().map(|s| (s.start, s.end)),
            Some((17, 18))
        );
        let token = d.unexpected_token.as_ref().unwrap();
        let spelling = symbol.to_string();
        assert_eq!(
            (
                token.kind.as_str(),
                token.text.as_deref(),
                token.span.start,
                token.span.end
            ),
            (
                if syntax_operator { "symbol" } else { "invalid" },
                Some(spelling.as_str()),
                17,
                18,
            )
        );
        assert_eq!(
            d.parser_context.as_deref(),
            Some(if syntax_operator {
                "unsupported_binary_operator"
            } else {
                "lexical_token"
            })
        );
        assert_eq!(
            d.expected_tokens,
            if syntax_operator {
                &["+", "-", "<", "=="][..]
            } else {
                &["valid_token"][..]
            }
        );
        assert_eq!(
            d.expected_tokens_kind.map(|x| x.as_str()),
            Some(if syntax_operator {
                "admissible_set"
            } else {
                "lexical_category"
            })
        );
        assert_eq!(
            d.recognized_prefix.as_ref().map(|s| (s.start, s.end)),
            Some((if syntax_operator { 10 } else { 0 }, 17))
        );
        assert!(d.data.is_empty());
        d.validate("fixture.n", &negative).unwrap();
        let e = execute("fn Main() {\n    let x = 1 + 2;\n}\n");
        assert_eq!(e.outcome, Outcome::Unit);
        assert!(e.diagnostics.is_empty());
        assert!(e.observations.observations().is_empty());
    }
    let value = execute("fn Main() {\n    let x = 1 + 2;\n    AssertEq(x, 3);\n}\n");
    assert_eq!(value.outcome, Outcome::Unit);
}
#[test]
fn adr_gap_runtime_index_primary_tracks_whole_index_operation() {
    let read_source = "fn Main() {\n    let a = [0];\n    Print(a[2]);\n}\n";
    let read = execute(read_source);
    assert_eq!(read.diagnostics[0].code, "R0102");
    assert_eq!(
        read.diagnostics[0]
            .primary_span
            .as_ref()
            .map(|s| (s.start, s.end)),
        Some(byte_span(read_source, "a[2]", 0))
    );
    let write_source = "fn Main() {\n    var a = [0];\n    a[2] = 1;\n}\n";
    let write = execute(write_source);
    assert_eq!(write.diagnostics[0].code, "R0102");
    assert_eq!(
        write.diagnostics[0]
            .primary_span
            .as_ref()
            .map(|s| (s.start, s.end)),
        Some(byte_span(write_source, "a[2] = 1;", 0))
    );
}
#[test]
fn generated_array_copies_and_writes_match_independent_matrix_model() {
    let mut seed = 0x8c6f_3419_u64;
    for _ in 0..8_000 {
        let mut draw = || {
            seed = seed
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            ((seed >> 32) % 17) as i64 - 8
        };
        let original = [[draw(), draw()], [draw(), draw()]];
        let mut copy = original;
        let row = (draw().unsigned_abs() % 2) as usize;
        let col = (draw().unsigned_abs() % 2) as usize;
        let replacement = draw();
        copy[row][col] = replacement;
        let source = format!(
            "fn Main(){{var a=[[{},{}],[{},{}]];var b=a;b[{row}][{col}]={replacement};Print(a);Print(b);}}",
            original[0][0], original[0][1], original[1][0], original[1][1]
        );
        let expected =
            |m: [[i64; 2]; 2]| format!("[[{}, {}], [{}, {}]]", m[0][0], m[0][1], m[1][0], m[1][1]);
        let e = execute(&source);
        assert_eq!(e.outcome, Outcome::Unit, "{source}");
        assert_eq!(
            e.observations.stdout(),
            vec![expected(original), expected(copy)]
        );
    }
}
#[test]
fn array_sources_with_mutated_syntax_never_panic_and_validate_rejections() {
    let templates = [
        "fn Main() {\n    var a = [[1, 2], [3, 4]];\n    a[0][1] = 9;\n    Print(a[0][1]);\n}\n",
        "fn Main() {\n    let a: Array<Array<I64>> = [[1], [2]];\n    Print(Len(a));\n}\n",
        "fn Main() {\n    let a = [true, false];\n    AssertEq(a, [true, false]);\n}\n",
        "fn Main() {\n    var a = [0];\n    a[-1] = 9223372036854775807 + 1;\n}\n",
    ];
    for k in 0..20_000usize {
        let base = templates[k % templates.len()];
        let mut bytes = base.as_bytes().to_vec();
        let pos = (k.wrapping_mul(7919) + k / 7) % bytes.len();
        bytes[pos] = b"[](){};,+-=<>0123456789x"[k % 23];
        let source = String::from_utf8(bytes).unwrap();
        match compile_scalar("fuzz.n", &source) {
            Ok(program) => {
                let e = run_scalar(&program);
                assert!(!matches!(e.outcome, Outcome::InternalError(_)), "{source}");
                for d in e.diagnostics {
                    d.validate("fuzz.n", &source).unwrap();
                }
            }
            Err(CompileError::Rejected(ds)) => {
                for d in ds {
                    d.validate("fuzz.n", &source).unwrap();
                }
            }
            Err(CompileError::InvalidIr(e)) => panic!("invalid IR from source {source}: {e}"),
            Err(CompileError::Dependent) => panic!("escaped dependent error: {source}"),
        }
    }
}
#[test]
fn nested_arrays_compile_and_run_from_small_calling_stacks() {
    for stack in [128usize, 256, 512] {
        std::thread::Builder::new()
            .stack_size(stack * 1024)
            .spawn(move || {
                for depth in [1usize, 32, 64, 96, 120, 128, 129] {
                    let source = format!(
                        "fn Main(){{let a={}1{};Print(Len(a));}}",
                        "[".repeat(depth),
                        "]".repeat(depth)
                    );
                    match compile_scalar("stack.n", &source) {
                        Ok(p) => {
                            let e = run_scalar(&p);
                            assert_eq!(e.outcome, Outcome::Unit, "depth {depth}");
                            assert_eq!(e.observations.stdout(), vec!["1"]);
                        }
                        Err(CompileError::Rejected(ds)) if depth > 120 => {
                            for d in ds {
                                d.validate("stack.n", &source).unwrap();
                            }
                        }
                        Err(CompileError::Rejected(ds)) => panic!("admitted depth {depth}: {ds:?}"),
                        Err(CompileError::InvalidIr(e)) => panic!("depth {depth}: {e}"),
                        Err(CompileError::Dependent) => {
                            panic!("depth {depth}: escaped dependent error")
                        }
                    }
                }
            })
            .unwrap()
            .join()
            .unwrap();
    }
}
#[test]
#[ignore = "explicit array lowering scale measurement"]
fn array_ir_lowering_scales_across_ten_source_families() {
    let _campaign = crate::test_alloc::campaign_guard();
    let patterns = [
        "a[0][0]=1;",
        "a[1]=[3,4];",
        "AssertEq(a[0],[1,2]);",
        "AssertEq(a,a);",
        "AssertEq(Len(a),2);",
        "Print(a[0][1]);",
        "[1,2,3];",
        "a[0][1]=a[1][0];",
        "{let b=a;}",
        "Print(Len(a));",
    ];
    for (family, pattern) in patterns.iter().enumerate() {
        let mut traffic = Vec::new();
        for size in [262_144usize, 1_048_576] {
            let mut source = String::from("fn Main(){var a=[[1,2],[3,4]];");
            while source.len() + pattern.len() + 3 <= size {
                source.push_str(pattern);
            }
            source.push('}');
            source.push_str("//");
            source.push_str(&"a".repeat(size - source.len()));
            let ast = parse::parse("dead_scale_counter.n", &source).unwrap();
            crate::test_alloc::reset("dead_scale_counter.n");
            let p = super::lower::compile_ast(&ast, &source).unwrap();
            assert_eq!(p.functions().functions().len(), 1);
            traffic.push(crate::test_alloc::snapshot("dead_scale_counter.n"));
        }
        eprintln!("array family {:02} {} {traffic:?}", family + 1, pattern);
        for (small, large) in [(traffic[0].0, traffic[1].0), (traffic[0].1, traffic[1].1)] {
            assert!(
                large <= small.saturating_mul(8),
                "array family {family}: {traffic:?}"
            );
        }
    }
}

/// Independent decimal model: it does not call the IR evaluator, reuse IR types,
/// or derive an expected result from the program's output.
fn reference_linear(start: i64, steps: &[(bool, i64)]) -> i64 {
    let mut value = i128::from(start);
    for &(add, operand) in steps {
        if add {
            value += i128::from(operand)
        } else {
            value -= i128::from(operand)
        }
    }
    i64::try_from(value).unwrap()
}
#[test]
fn generated_scalar_programs_match_independent_decimal_model() {
    let mut seed = 0x513C_2DA4_8872_151Bu64;
    for _case in 0..8_000 {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        let start = (seed % 20_001) as i64 - 10_000;
        let mut steps = Vec::new();
        let mut source = format!("fn Main(){{var x={start};");
        for index in 0..3 {
            seed ^= seed << 13;
            seed ^= seed >> 7;
            seed ^= seed << 17;
            let add = seed & 1 == 0;
            let operand = ((seed >> 8) % 1_000) as i64;
            steps.push((add, operand));
            source.push_str(&format!("x=x{}{operand};", if add { "+" } else { "-" }));
            if index == 1 {
                source.push_str("{let x=true;AssertEq(x,true);}")
            }
        }
        let expected = reference_linear(start, &steps);
        source.push_str(&format!("AssertEq(x,{expected});Print(x);}}"));
        let e = execute(&source);
        assert_eq!(e.outcome, Outcome::Unit, "{source}");
        assert_eq!(
            e.observations.stdout(),
            vec![expected.to_string()],
            "{source}"
        );
        assert!(e.observations.events().is_empty());
        assert!(e.diagnostics.is_empty());
        assert_eq!(
            e.observations.bytes(),
            expected.to_string().len() as u64 + 1
        );
    }
}
#[test]
fn malformed_source_campaign_never_panics_and_all_diagnostics_validate() {
    let alphabet = b"fn Main(){let x=1;Print(x);}/%!?\n\r[]<>=abc019 ";
    let mut seed = 0xA73D_8013_017B_459Eu64;
    for _case in 0..20_000 {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        let len = (seed % 60) as usize;
        let mut source = String::with_capacity(len);
        for _ in 0..len {
            seed ^= seed << 13;
            seed ^= seed >> 7;
            seed ^= seed << 17;
            source.push(alphabet[(seed as usize) % alphabet.len()] as char);
        }
        match compile_scalar("fuzz.n", &source) {
            Ok(program) => {
                let e = run_scalar(&program);
                for d in &e.diagnostics {
                    d.validate("fuzz.n", &source).unwrap()
                }
            }
            Err(CompileError::Rejected(errors)) => {
                for d in errors {
                    d.validate("fuzz.n", &source).unwrap()
                }
            }
            Err(CompileError::InvalidIr(e)) => panic!("invalid generated IR: {e} for {source:?}"),
            Err(CompileError::Dependent) => panic!("escaped dependent error: {source:?}"),
        }
    }
}
#[test]
fn scalar_lowering_and_machine_run_on_a_512_kib_caller_stack() {
    std::thread::Builder::new()
        .stack_size(512 * 1024)
        .spawn(|| {
            for depth in [1, 16, 48, 80] {
                let source = format!(
                    "fn Main(){{Print({}1{});}}",
                    "(".repeat(depth),
                    ")".repeat(depth)
                );
                let e = execute(&source);
                assert_eq!(e.outcome, Outcome::Unit);
                assert_eq!(e.observations.stdout(), vec!["1"]);
            }
        })
        .unwrap()
        .join()
        .unwrap();
}
#[test]
#[ignore = "explicit scaling measurement; run separately with --ignored --nocapture"]
fn scalar_ir_lowering_scales_across_fifteen_source_families() {
    let _campaign = crate::test_alloc::campaign_guard();
    let patterns = [
        "x=x+1;",
        "x=x-1;",
        "Print(x);",
        "AssertEq(x,x);",
        "AssertEq(true,true);",
        "{let y=1;}",
        "{var y=1;y=2;}",
        "{let y=true;}",
        "x=1;",
        "x=(x+1);",
        "x=1+2-3;",
        "AssertEq(1<2,true);",
        "Print(());",
        "Print(false);",
        "{}",
    ];
    for (family, pattern) in patterns.iter().enumerate() {
        let mut measurements = Vec::new();
        for size in [262_144usize, 1_048_576] {
            let mut source = String::from("fn Main(){var x=0;");
            while source.len() + pattern.len() + 3 <= size {
                source.push_str(pattern)
            }
            source.push('}');
            source.push_str("//");
            source.push_str(&"a".repeat(size - source.len()));
            assert_eq!(source.len(), size);
            let ast = parse::parse("dead_scale_counter.n", &source).unwrap();
            crate::test_alloc::reset("dead_scale_counter.n");
            let program = super::lower::compile_ast(&ast, &source).unwrap();
            assert_eq!(program.functions().functions().len(), 1);
            measurements.push(crate::test_alloc::snapshot("dead_scale_counter.n"));
        }
        eprintln!("family {:02} {:<24} {measurements:?}", family + 1, pattern);
        for (small, large) in [
            (measurements[0].0, measurements[1].0),
            (measurements[0].1, measurements[1].1),
        ] {
            assert!(
                large <= small.saturating_mul(8),
                "family {family}: {measurements:?}"
            );
        }
    }
}
