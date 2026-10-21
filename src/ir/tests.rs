use super::{CompileError, compile_scalar};
use crate::frontend::parse;
use crate::interp::{Outcome, run_scalar};

fn execute(source: &str) -> crate::interp::Execution {
    let program = compile_scalar("fixture.nx", source).unwrap();
    let result = run_scalar(&program);
    for d in &result.diagnostics {
        d.validate("fixture.nx", source).unwrap();
    }
    result
}
#[test]
fn integer_limits_execute_with_literal_expected_output() {
    let source = "fn main(){assert_eq(-9223372036854775808+1,-9223372036854775807);assert_eq(9223372036854775806+1,9223372036854775807);assert_eq(true,true);print(25);}";
    let execution = execute(source);
    assert_eq!(execution.outcome, Outcome::Unit);
    assert_eq!(execution.observations.stdout(), vec!["25"]);
    assert!(execution.observations.events().is_empty());
    assert_eq!(execution.observations.bytes(), 3);
    assert_eq!(execution.observations.reserved_bytes(), 0);
    assert!(execution.diagnostics.is_empty());
}
#[test]
fn published_integer_limits_corpus_runs_through_verified_ir() {
    let source = include_str!("../../corpus/pass/integer_limits.nx");
    let e = execute(source);
    assert_eq!(e.outcome, Outcome::Unit);
    assert_eq!(e.observations.stdout(), vec!["25"]);
    assert_eq!(e.observations.bytes(), 3);
    assert!(e.diagnostics.is_empty());
}
#[test]
fn signed_literal_outside_i64_is_a_types_diagnostic() {
    let source = "fn main(){let x=9223372036854775808;}";
    let CompileError::Rejected(errors) = compile_scalar("fixture.nx", source).unwrap_err() else {
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
        Some((16, 35))
    );
    assert!(d.data.is_empty());
    d.validate("fixture.nx", source).unwrap();
    assert_eq!(
        execute("fn main(){let x=9223372036854775807;}").outcome,
        Outcome::Unit
    );
}
#[test]
fn trailing_module_garbage_is_rejected_before_lowering() {
    let source = "fn main(){} garbage";
    let CompileError::Rejected(errors) = compile_scalar("fixture.nx", source).unwrap_err() else {
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
    d.validate("fixture.nx", source).unwrap();
    assert_eq!(execute("fn main(){}").outcome, Outcome::Unit);
}
#[test]
fn adr_gap_unmigrated_corpus_comments_and_crlf_keep_worktree_byte_offsets() {
    for (source, code, start, end) in [
        (
            include_str!("../../corpus/fail/overflow_literal.nx"),
            "NX-E0107",
            42,
            61,
        ),
        (
            include_str!("../../corpus/fail/trailing_garbage.nx"),
            "NX-E0002",
            31,
            38,
        ),
    ] {
        let CompileError::Rejected(errors) = compile_scalar("fixture.nx", source).unwrap_err()
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
        d.validate("fixture.nx", source).unwrap();
    }
}
#[test]
fn subtraction_overflow_has_published_runtime_fields_and_no_output() {
    let source = "fn main(){print(-9223372036854775808-1);}";
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
        Some((16, 38))
    );
    assert!(d.data.is_empty());
    let p = execute("fn main(){print(-9223372036854775808-0);}");
    assert_eq!(p.outcome, Outcome::Unit);
    assert_eq!(p.observations.stdout(), vec!["-9223372036854775808"]);
    assert_eq!(p.observations.bytes(), 21);
}
#[test]
fn addition_overflow_and_assert_failure_keep_prior_output_in_order() {
    let plus = "fn main(){print(7);print(9223372036854775807+1);}";
    let e = execute(plus);
    assert_eq!(e.outcome, Outcome::SimulatorFault);
    assert_eq!(e.observations.stdout(), vec!["7"]);
    assert_eq!(e.observations.bytes(), 2);
    assert_eq!(e.diagnostics[0].code, "R0101");
    assert_eq!(e.diagnostics[0].stage.as_str(), "runtime");
    assert_eq!(e.diagnostics[0].operation.as_str(), "value.add");
    assert!(e.diagnostics[0].data.is_empty());
    let assertion = execute("fn main(){print(7);assert_eq(1,2);print(9);}");
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
    let e = execute("fn main(){let x=3;drop(x);print(true);fail();print(9);}");
    assert_eq!(e.outcome, Outcome::SimulatorFault);
    assert_eq!(e.observations.stdout(), vec!["true"]);
    assert!(e.observations.events().is_empty());
    assert_eq!(e.diagnostics.len(), 1);
    assert_eq!(
        (&*e.diagnostics[0].code, e.diagnostics[0].operation.as_str()),
        ("R0301", "intrinsic.fail")
    );
    assert!(e.diagnostics[0].data.is_empty());
    let p = execute("fn main(){drop(3);print(false);}");
    assert_eq!(p.outcome, Outcome::Unit);
    assert_eq!(p.observations.stdout(), vec!["false"]);
}
#[test]
fn observation_quota_checks_complete_next_line_and_preserves_prior_lines() {
    let mut source = String::from("fn main(){");
    for _ in 0..32_769 {
        source.push_str("print(0);")
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
    let a = "fn main(){\nlet value=2\nlet a=value -1\nlet b=value - 1\nassert_eq(a,b)\n}";
    let b = a.replacen("value -1", "value - 1", 1);
    let ast_a = parse::parse("fixture.nx", a).unwrap();
    let ast_b = parse::parse("fixture.nx", &b).unwrap();
    assert_eq!(
        crate::frontend::print::print_canonical(&ast_a),
        crate::frontend::print::print_canonical(&ast_b)
    );
    let pa = compile_scalar("fixture.nx", a).unwrap();
    let pb = compile_scalar("fixture.nx", &b).unwrap();
    assert_eq!(
        pa.functions().function("main").unwrap().content_hash,
        pb.functions().function("main").unwrap().content_hash
    );
    for source in [a, b.as_str()] {
        let e = execute(source);
        assert_eq!(e.outcome, Outcome::Unit);
        assert!(e.observations.observations().is_empty());
    }
}
#[test]
fn unicode_crlf_and_final_eof_binding_variants_execute_equally() {
    let base = "// á🙂\nfn main(){\nlet missing=1;\nlet x=missing;\n}";
    let crlf = base.replace('\n', "\r\n");
    let final_lf = format!("{base}\n");
    let mut canonical = Vec::new();
    for source in [base, crlf.as_str(), final_lf.as_str()] {
        let ast = parse::parse("fixture.nx", source).unwrap();
        canonical.push(crate::frontend::print::print_canonical(&ast));
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
        ("S16", "fn main(){let b=0<1;}"),
        ("S18", "fn main(){let b=1==1;}"),
        ("S22", "fn main(){let match_value=1;}"),
        ("S23", "fn main(){let x=1;let y=2;}"),
        ("S27", "fn main(){let x=1+2;}"),
        ("S28", "fn main(){let x=1==2;}"),
        ("S31", "fn main(){let x=1;}"),
        ("S32", "fn main(){let x=1<2;}"),
        ("S33", "fn main(){let x=1<2;}"),
        ("S34", "fn main(){let x=1<2;}"),
        ("F03", "fn main(){let x=0;}"),
        ("F04", "fn main(){let n=1;let x=0+n;}"),
        ("F05", "fn main(){let x=(1);}"),
        ("F06", "fn main(){}"),
        ("F08", "fn main(){let f=();}"),
        ("F16", "fn main(){}"),
        ("F17", "fn main(){}"),
        ("F18", "fn main(){}"),
        ("F19", "fn main(){}"),
        ("F21", "fn main(){}"),
        ("F22", "fn main(){let x=1;}"),
        ("F24", "fn main(){var x=1;x=x+1;}"),
        ("F27", "fn g(){} fn main(){}"),
        ("F28", "fn f(){} fn main(){}"),
        ("F32", "fn main(){{}}"),
        ("F33", "fn main(){let x=0;}"),
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
    let e = execute("fn main(){var x=1;{let x=7;assert_eq(x,7);}x=x+2;assert_eq(x,3);print(x);}");
    assert_eq!(e.outcome, Outcome::Unit);
    assert_eq!(e.observations.stdout(), vec!["3"]);
}
#[test]
fn inner_assignment_targets_inner_binding_without_replacing_outer_value() {
    let e = execute("fn main(){var x=1;{var x=7;x=8;assert_eq(x,8);}assert_eq(x,1);}");
    assert_eq!(e.outcome, Outcome::Unit);
    assert!(e.observations.observations().is_empty());
}
#[test]
fn scalar_relations_keep_strict_less_and_boolean_equality() {
    let e = execute(
        "fn main(){assert_eq(1<2,true);assert_eq(2<2,false);assert_eq(3<2,false);assert_eq(true==false,false);}",
    );
    assert_eq!(e.outcome, Outcome::Unit);
    assert!(e.observations.observations().is_empty());
}
#[test]
fn scalar_work_tally_counts_expression_visits_and_intrinsic_entry() {
    assert_eq!(execute("fn main(){print(1);}").work_consumed, 10);
    assert_eq!(execute("fn main(){assert_eq(1<2,true);}").work_consumed, 19);
}
#[test]
fn repeated_complete_assignment_releases_replaced_cells() {
    let mut source = String::from("fn main(){var x=0;");
    for _ in 0..66_000 {
        source.push_str("x=x+1;")
    }
    source.push_str("assert_eq(x,66000);}");
    let e = execute(&source);
    assert_eq!(e.outcome, Outcome::Unit);
    assert!(e.diagnostics.is_empty());
    assert!(e.observations.observations().is_empty());
}
#[test]
fn function_identity_and_hash_ignore_unrelated_order_and_track_body() {
    let one = compile_scalar("fixture.nx", "fn main(){print(1);}").unwrap();
    assert_eq!(
        one.functions().function("main").unwrap().content_hash,
        0xdd16_3509_de47_393f
    );
    let a = compile_scalar("fixture.nx", "fn helper(){print(1);} fn main(){print(2);}").unwrap();
    assert_eq!(a.functions().function("helper").unwrap().id.0, "helper");
    assert_eq!(a.functions().function("main").unwrap().id.0, "main");
    let b = compile_scalar(
        "fixture.nx",
        "fn main(){ print(2); } fn helper(){print(1);}",
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
    let changed =
        compile_scalar("fixture.nx", "fn helper(){print(1);} fn main(){print(3);}").unwrap();
    assert_ne!(
        a.functions().function("main").unwrap().content_hash,
        changed.functions().function("main").unwrap().content_hash
    );
    assert_eq!(
        a.functions().function("helper").unwrap().content_hash,
        changed.functions().function("helper").unwrap().content_hash
    );
    let add = compile_scalar("fixture.nx", "fn main(){print(1+2);}").unwrap();
    let sub = compile_scalar("fixture.nx", "fn main(){print(1-2);}").unwrap();
    assert_ne!(
        add.functions().function("main").unwrap().content_hash,
        sub.functions().function("main").unwrap().content_hash
    );
}
#[test]
fn function_ids_keep_names_while_table_indices_follow_canonical_order() {
    for source in [
        "fn zeta(){} fn main(){} fn alpha(){}",
        "fn main(){} fn alpha(){} fn zeta(){}",
    ] {
        let program = compile_scalar("fixture.nx", source).unwrap();
        let functions = program.functions().functions();
        assert_eq!(
            functions
                .iter()
                .map(|f| (f.name.as_str(), f.table_index, f.id.0.as_str()))
                .collect::<Vec<_>>(),
            vec![
                ("alpha", 0, "alpha"),
                ("main", 1, "main"),
                ("zeta", 2, "zeta")
            ]
        );
    }
}
#[test]
fn verified_function_table_substitution_changes_development_execution() {
    let mut a = compile_scalar("fixture.nx", "fn main(){print(1);}").unwrap();
    let b = compile_scalar("fixture.nx", "fn main(){print(2);}").unwrap();
    assert_eq!(run_scalar(&a).observations.stdout(), vec!["1"]);
    a.substitute_for_development("main", &b).unwrap();
    assert_eq!(run_scalar(&a).observations.stdout(), vec!["2"]);
}
#[test]
fn function_identity_survives_other_functions_and_substitution_checks_signature() {
    let mut receiver = compile_scalar("fixture.nx", "fn helper(){} fn main(){print(1);}").unwrap();
    let donor = compile_scalar(
        "fixture.nx",
        "fn aaa(){} fn helper(){} fn main(){print(2);}",
    )
    .unwrap();
    let original = receiver.functions().function("main").unwrap();
    let edited = donor.functions().function("main").unwrap();
    assert_eq!(original.id, edited.id);
    assert_ne!(original.table_index, edited.table_index);
    assert_ne!(original.content_hash, edited.content_hash);
    receiver.substitute_for_development("main", &donor).unwrap();
    assert_eq!(run_scalar(&receiver).observations.stdout(), vec!["2"]);

    let reordered = compile_scalar("fixture.nx", "fn main(){print(3);} fn zzz(){}").unwrap();
    assert_eq!(
        reordered.functions().function("main").unwrap().id,
        edited.id
    );
    receiver
        .substitute_for_development("main", &reordered)
        .unwrap();
    assert_eq!(run_scalar(&receiver).observations.stdout(), vec!["3"]);

    let mut receiver = compile_scalar("fixture.nx", "fn helper(){} fn main(){}").unwrap();
    let changed = compile_scalar("fixture.nx", "fn helper()->i64{return 1;} fn main(){}").unwrap();
    assert_eq!(
        receiver.substitute_for_development("helper", &changed),
        Err("signature mismatch")
    );
}
#[test]
fn function_table_refuses_forged_development_identity() {
    let mut program = compile_scalar("fixture.nx", "fn main(){}").unwrap();
    let mut replacement = program.functions().function("main").unwrap().clone();
    replacement.id.0 = "other".into();
    assert_eq!(
        program
            .functions
            .replace_for_development("main", replacement),
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
                    let source = format!("fn main(){{print({expr});}}");
                    let result = execute(&source);
                    assert_eq!(result.outcome, Outcome::Unit, "{source}");
                }
                let source = format!("fn main(){{{}print(1);{}}}", "{".repeat(90), "}".repeat(90));
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
                "fn main(){{print({}1{});}}",
                "(".repeat(depth),
                ")".repeat(depth)
            ),
            1 => format!(
                "fn main(){{{}print(1);{}}}",
                "{".repeat(depth),
                "}".repeat(depth)
            ),
            2 => format!("fn main(){{print({}1);}}", "1+".repeat(depth)),
            3 => format!(
                "fn main(){{print(1+{}1{});}}",
                "(1+".repeat(depth),
                ")".repeat(depth)
            ),
            _ => {
                let mut expr = "(1<2)".to_string();
                for _ in 0..depth {
                    expr = format!("({expr}==(1<2))");
                }
                format!("fn main(){{print({expr});}}")
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
                        match compile_scalar("fixture.nx", &s) {
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
fn unsupported_arrays_and_user_calls_never_execute_partially() {
    assert!(matches!(
        compile_scalar("fixture.nx", "fn main(){print(1);let a=[2];}"),
        Err(CompileError::OutsideScalarSlice("arrays and indexing"))
    ));
    assert!(matches!(
        compile_scalar("fixture.nx", "fn f(){} fn main(){f();}"),
        Err(CompileError::OutsideScalarSlice("user call"))
    ));
    assert!(matches!(
        compile_scalar("fixture.nx", "fn main(){let x=1;x=2;}"),
        Err(CompileError::OutsideScalarSlice(
            "immutable write awaits ownership checking"
        ))
    ));
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
        let mut source = format!("fn main(){{var x={start};");
        for index in 0..3 {
            seed ^= seed << 13;
            seed ^= seed >> 7;
            seed ^= seed << 17;
            let add = seed & 1 == 0;
            let operand = ((seed >> 8) % 1_000) as i64;
            steps.push((add, operand));
            source.push_str(&format!("x=x{}{operand};", if add { "+" } else { "-" }));
            if index == 1 {
                source.push_str("{let x=true;assert_eq(x,true);}")
            }
        }
        let expected = reference_linear(start, &steps);
        source.push_str(&format!("assert_eq(x,{expected});print(x);}}"));
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
    let alphabet = b"fn main(){let x=1;print(x);}/%!?\n\r[]<>=abc019 ";
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
        match compile_scalar("fuzz.nx", &source) {
            Ok(program) => {
                let e = run_scalar(&program);
                for d in &e.diagnostics {
                    d.validate("fuzz.nx", &source).unwrap()
                }
            }
            Err(CompileError::Rejected(errors)) => {
                for d in errors {
                    d.validate("fuzz.nx", &source).unwrap()
                }
            }
            Err(CompileError::OutsideScalarSlice(_)) => {}
            Err(CompileError::InvalidIr(e)) => panic!("invalid generated IR: {e} for {source:?}"),
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
                    "fn main(){{print({}1{});}}",
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
    use std::time::Instant;
    let patterns = [
        "x=x+1;",
        "x=x-1;",
        "print(x);",
        "assert_eq(x,x);",
        "assert_eq(true,true);",
        "{let y=1;}",
        "{var y=1;y=2;}",
        "{let y=true;}",
        "x=1;",
        "x=(x+1);",
        "x=1+2-3;",
        "assert_eq(1<2,true);",
        "print(());",
        "print(false);",
        "{}",
    ];
    for (family, pattern) in patterns.iter().enumerate() {
        let mut measurements = Vec::new();
        for size in [262_144usize, 1_048_576] {
            let mut source = String::from("fn main(){var x=0;");
            while source.len() + pattern.len() + 3 <= size {
                source.push_str(pattern)
            }
            source.push('}');
            source.push_str("//");
            source.push_str(&"a".repeat(size - source.len()));
            assert_eq!(source.len(), size);
            let ast = parse::parse("scale.nx", &source).unwrap();
            let before = Instant::now();
            let program = super::lower::compile_ast(&ast, &source).unwrap();
            let elapsed = before.elapsed();
            assert_eq!(program.functions().functions().len(), 1);
            measurements.push(elapsed.as_secs_f64());
        }
        let ratio = measurements[1] / measurements[0];
        eprintln!(
            "family {:02} {:<24} {:.6}s {:.6}s ratio {:.3}",
            family + 1,
            pattern,
            measurements[0],
            measurements[1],
            ratio
        );
        assert!(ratio < 8.0, "family {family} grew superlinearly: {ratio}");
    }
}
