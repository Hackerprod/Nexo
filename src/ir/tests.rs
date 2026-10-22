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
        let source = source.replace("\r\n", "\n").replace('\n', "\r\n");
        let source = source.as_str();
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
        0x2429_2d7d_adf8_6fdf
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
fn arrays_and_user_calls_execute_only_after_verification() {
    let arrays = compile_scalar("fixture.nx", "fn main(){print(1);let a=[2];}").unwrap();
    assert_eq!(run_scalar(&arrays).observations.stdout(), &["1"]);
    let called = compile_scalar("fixture.nx", "fn f(){} fn main(){f();}").unwrap();
    assert_eq!(run_scalar(&called).outcome, Outcome::Unit);
    assert!(matches!(
        compile_scalar("fixture.nx", "fn main(){let x=1;x=2;}"),
        Err(CompileError::Rejected(_))
    ));
}
#[test]
fn nested_copy_rebinds_physical_values_without_aliasing() {
    let source = include_str!("../../corpus/pass/nested_copy.nx");
    let e = execute(source);
    assert_eq!(e.outcome, Outcome::Unit);
    assert_eq!(e.observations.stdout(), vec!["[[1, 9], [7, 8]]"]);
    assert_eq!(e.observations.bytes(), 17);
    assert!(e.diagnostics.is_empty());
}
#[test]
fn immutable_index_root_has_ownership_diagnostic_and_declaration_span() {
    let source = "fn main(){let value=[[1]];value[0][0]=2;}";
    let CompileError::Rejected(errors) = compile_scalar("fixture.nx", source).unwrap_err() else {
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
        Some((26, 31))
    );
    assert_eq!(d.related_spans.len(), 1);
    assert_eq!(
        (
            d.related_spans[0].role.as_str(),
            d.related_spans[0].span.start,
            d.related_spans[0].span.end
        ),
        ("declaration", 14, 19)
    );
    assert!(d.data.is_empty());
    d.validate("fixture.nx", source).unwrap();
    let e = execute("fn main(){var value=[[1]];value[0][0]=2;print(value);}");
    assert_eq!(e.observations.stdout(), vec!["[[2]]"]);
}
#[test]
fn independent_immutable_writes_report_each_site_in_source_order() {
    for (source, primary, declaration) in [
        (
            "fn main(){let a=[1];let b=[2];a[0]=1;b[0]=2;}",
            vec![(30, 31), (37, 38)],
            vec![(14, 15), (24, 25)],
        ),
        (
            "fn helper(){let q=[1];q[0]=2;} fn main(){let p=[1];p[0]=2;}",
            vec![(22, 23), (51, 52)],
            vec![(16, 17), (45, 46)],
        ),
    ] {
        let CompileError::Rejected(errors) = compile_scalar("fixture.nx", source).unwrap_err()
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
            assert_eq!(
                error.primary_span.as_ref().map(|s| (s.start, s.end)),
                Some(primary[index])
            );
            assert_eq!(
                error
                    .related_spans
                    .iter()
                    .map(|s| (s.span.start, s.span.end))
                    .collect::<Vec<_>>(),
                vec![declaration[index]]
            );
            assert!(error.data.is_empty());
            error.validate("fixture.nx", source).unwrap();
        }
    }
}

#[test]
fn immutable_write_does_not_emit_dependent_cascades() {
    let source = "fn main(){let a=[1];a[0]=2;a[0]=3;let b=[4];b[0]=5;}";
    let CompileError::Rejected(errors) = compile_scalar("fixture.nx", source).unwrap_err() else {
        panic!("ownership rejection")
    };
    assert_eq!(errors.len(), 3);
    assert_eq!(
        errors
            .iter()
            .map(|d| d.primary_span.as_ref().unwrap().start)
            .collect::<Vec<_>>(),
        vec![20, 27, 44]
    );
    for error in errors {
        assert_eq!(error.code, "NX-E0105");
        error.validate("fixture.nx", source).unwrap();
    }
}
#[test]
fn ownership_diagnostics_sort_by_source_span_across_canonical_function_order() {
    let source = "fn z(){let a=[1];a[0]=2;} fn main(){let b=[1];b[0]=2;}";
    let CompileError::Rejected(errors) = compile_scalar("fixture.nx", source).unwrap_err() else {
        panic!("ownership rejection")
    };
    assert_eq!(
        errors
            .iter()
            .map(|d| d.primary_span.as_ref().unwrap().start)
            .collect::<Vec<_>>(),
        vec![17, 46]
    );
}
#[test]
fn ownership_errors_in_nonunit_function_precede_lowering_of_later_functions() {
    let source = "fn z()->i64{let a=[1];a[0]=2;} fn main(){let b=[1];b[0]=2;}";
    let CompileError::Rejected(errors) = compile_scalar("fixture.nx", source).unwrap_err() else {
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
    for (source, primary) in [
        ("fn f()->i64 { } fn main(){}", (14, 15)),
        ("fn f()->i64{\n1\n}\nfn main(){}", (15, 16)),
    ] {
        let CompileError::Rejected(errors) = compile_scalar("fixture.nx", source).unwrap_err()
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
            Some(primary)
        );
        assert_eq!(d.related_spans.len(), 1);
        assert_eq!(d.related_spans[0].role.as_str(), "expected_type");
        assert_eq!(
            (d.related_spans[0].span.start, d.related_spans[0].span.end),
            (8, 11)
        );
        assert!(d.data.is_empty());
        assert!(d.cause_chain.is_empty());
        assert!(d.unexpected_token.is_none());
        d.validate("fixture.nx", source).unwrap();
    }
    compile_scalar("fixture.nx", "fn f()->i64{return 1;} fn main(){}").unwrap();
    compile_scalar("fixture.nx", "fn f()->i64{\nreturn 1\n}\nfn main(){}").unwrap();
}

#[test]
fn cfg_statement_if_selects_one_branch() {
    let source = "fn main(){var x=0;if true{x=1;}else{x=2;}assert_eq(x,1);}";
    let program = compile_scalar("if.nx", source).unwrap();
    let execution = crate::interp::run_scalar(&program);
    assert!(
        matches!(execution.outcome, crate::interp::Outcome::Unit),
        "{execution:?}"
    );
}

#[test]
fn cfg_while_rechecks_condition_and_updates_local() {
    let source = "fn main(){var x=0;while x<3{x=x+1;}assert_eq(x,3);}";
    let program = compile_scalar("while.nx", source).unwrap();
    let execution = crate::interp::run_scalar(&program);
    assert!(
        matches!(execution.outcome, crate::interp::Outcome::Unit),
        "{execution:?}"
    );
}

#[test]
fn cfg_expression_if_delivers_explicit_unit() {
    // F0-0002 §14.8 L05.
    let source = "fn main(){\nlet x=if true{value: ();}else{value: ();};\nassert_eq(x,());\n}";
    let program = compile_scalar("value.nx", source).unwrap();
    let execution = crate::interp::run_scalar(&program);
    assert!(
        matches!(execution.outcome, crate::interp::Outcome::Unit),
        "{execution:?}"
    );
}

#[test]
fn cfg_join_reports_maybe_consumed_resource() {
    // F0-0002 §14.2 P08.
    let source = "fn f(b:bool){let r=mock_open(1);if b{drop(r);}inspect(r);} fn main(){}";
    let CompileError::Rejected(errors) = compile_scalar("p08.nx", source).unwrap_err() else {
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
    assert_eq!(
        d.primary_span.as_ref().map(|s| (s.start, s.end)),
        Some((54, 55))
    );
    assert_eq!(
        d.related_spans
            .iter()
            .map(|r| (r.role.as_str(), r.span.start, r.span.end))
            .collect::<Vec<_>>(),
        [("declaration", 17, 18), ("consumption", 37, 44)]
    );
    assert_eq!(d.cause_chain.len(), 1);
    assert_eq!(
        d.cause_chain[0].span.as_ref().map(|s| (s.start, s.end)),
        Some((37, 44))
    );
    assert_eq!(
        d.cause_chain[0].entity.as_ref().map(|e| e.id.as_str()),
        Some("binding:17")
    );
    assert!(d.data.is_empty());
    d.validate("p08.nx", source).unwrap();
}

#[test]
fn cfg_returning_branch_does_not_poison_join() {
    let source = "fn f(b:bool){let r=mock_open(1);if b{drop(r);return;}inspect(r);} fn main(){f(true);f(false);}";
    let program = compile_scalar("join.nx", source).unwrap();
    let execution = crate::interp::run_scalar(&program);
    assert!(
        matches!(execution.outcome, crate::interp::Outcome::Unit),
        "{execution:?}"
    );
}

#[test]
fn cfg_first_value_arm_continues_when_else_returns() {
    let source = "fn f(b:bool)->i64{return if b{value:2;}else{return 3;};} fn main(){assert_eq(f(true),2);assert_eq(f(false),3);}";
    let program = compile_scalar("first_arm.nx", source).unwrap();
    let execution = crate::interp::run_scalar(&program);
    assert!(
        matches!(execution.outcome, crate::interp::Outcome::Unit),
        "{execution:?}"
    );
}

#[test]
fn cfg_returning_loop_body_excludes_consumption_from_false_exit() {
    let source = "fn f(b:bool){let r=mock_open(1);while b{drop(r);return;}inspect(r);} fn main(){f(false);f(true);}";
    let program = compile_scalar("loop_return_owner.nx", source).unwrap();
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
    let source = "fn f(b:bool){let r=mock_open(1);if b{}else{drop(r);}inspect(r);} fn main(){}";
    let CompileError::Rejected(errors) = compile_scalar("else_consumed.nx", source).unwrap_err()
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
        "fn f(b:bool){let r=mock_open(1);if b{drop(r);}else{return;}inspect(r);} fn main(){}",
        "fn f(b:bool){let r=mock_open(1);if b{return;}else{drop(r);}inspect(r);} fn main(){}",
    ] {
        let CompileError::Rejected(errors) =
            compile_scalar("single_owner_arm.nx", source).unwrap_err()
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
    let all_consumed =
        "fn f(b:bool){let r=mock_open(1);if b{drop(r);}else{drop(r);}inspect(r);} fn main(){}";
    let CompileError::Rejected(errors) =
        compile_scalar("all_consumed.nx", all_consumed).unwrap_err()
    else {
        panic!("expected consumed binding")
    };
    assert_eq!(errors.len(), 1);
    assert_eq!(errors[0].code, "NX-E0202");
    assert_eq!(errors[0].subcause.as_deref(), Some("definitely_consumed"));

    let restored = "fn f(b:bool){var r=mock_open(1);if b{drop(r);r=mock_open(2);}else{drop(r);r=mock_open(3);}inspect(r);} fn main(){f(true);f(false);}";
    let program = compile_scalar("restored.nx", restored).unwrap();
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
            "fn f(b:bool)->i64{return if b{value:1;};} fn main(){}",
            "NX-E1302",
            Some("missing_else"),
            (25, 27),
        ),
        (
            "B02",
            "fn f()->i64{return if true{1;}else{value:2;};} fn main(){}",
            "NX-E1302",
            Some("missing_branch_value"),
            (29, 30),
        ),
        (
            "B03",
            "fn f(){let n=if true{value:1;}else{value:true;};} fn main(){}",
            "NX-E1303",
            None,
            (41, 45),
        ),
        (
            "B04",
            "fn main(){value:1;}",
            "NX-E1302",
            Some("invalid_value_target"),
            (10, 16),
        ),
        (
            "B05",
            "fn f()->i64{return if true{value:}else{value:2;};} fn main(){}",
            "NX-E1302",
            Some("missing_value_operand"),
            (27, 33),
        ),
        (
            "B06",
            "fn f()->i64{return if true{value:1;2;}else{value:2;};} fn main(){}",
            "NX-E1302",
            Some("non_final_branch_value"),
            (27, 33),
        ),
        (
            "B09",
            "fn f()->i64{return if true{value:1;}else{value:true;};} fn main(){}",
            "NX-E0102",
            Some("type_mismatch"),
            (47, 51),
        ),
    ];
    for (row, source, code, subcause, primary) in rows {
        let CompileError::Rejected(errors) = compile_scalar("branch.nx", source).unwrap_err()
        else {
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
        errors[0].validate("branch.nx", source).unwrap();
    }
}

#[test]
fn cfg_b08_missing_value_terminator_keeps_parser_fields() {
    // F0-0002 §14.3 B08: the closing brace is the offending token.
    let source = "fn f()->i64{return if true{value:1 }else{value:2;};} fn main(){}";
    let CompileError::Rejected(errors) = compile_scalar("b08.nx", source).unwrap_err() else {
        panic!("expected missing terminator")
    };
    assert_eq!(errors.len(), 1);
    let d = &errors[0];
    assert_eq!(d.code, "NX-E0005");
    assert_eq!(
        d.primary_span.as_ref().map(|s| (s.start, s.end)),
        Some((35, 36))
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
    d.validate("b08.nx", source).unwrap();
}

#[test]
fn cfg_published_branch_repairs_execute() {
    // F0-0002 §14.3 B01–B06, B08–B09 repairs; results are semantic observations.
    for source in [
        "fn f(b:bool)->i64{return if b{value:1;}else{value:2;};} fn main(){assert_eq(f(true),1);assert_eq(f(false),2);}",
        "fn f()->i64{return if true{value:1;}else{value:2;};} fn main(){assert_eq(f(),1);}",
        "fn f(){let n=if true{value:1;}else{value:2;};assert_eq(n,1);} fn main(){f();}",
        "fn main(){let value=1;assert_eq(value,1);}",
    ] {
        let program = compile_scalar("branch_repair.nx", source).unwrap();
        let execution = crate::interp::run_scalar(&program);
        assert!(
            matches!(execution.outcome, crate::interp::Outcome::Unit),
            "{source}: {execution:?}"
        );
    }
}

#[test]
fn cfg_hash_tracks_branch_contents_and_development_substitution() {
    let original =
        "fn f(b:bool)->i64{return if b{value:1;}else{value:2;};} fn main(){print(f(true));}";
    let spaced = "fn f(b:bool)->i64 { return if b { value:1; } else { value:2; }; } fn main() { print(f(true)); }";
    let changed =
        "fn f(b:bool)->i64{return if b{value:3;}else{value:2;};} fn main(){print(f(true));}";
    let mut program = compile_scalar("cfg_hash.nx", original).unwrap();
    let same = compile_scalar("cfg_hash.nx", spaced).unwrap();
    let donor = compile_scalar("cfg_hash.nx", changed).unwrap();
    let hash = program.functions.function("f").unwrap().content_hash;
    assert_eq!(hash, same.functions.function("f").unwrap().content_hash);
    assert_ne!(hash, donor.functions.function("f").unwrap().content_hash);
    assert_eq!(
        crate::interp::run_scalar(&program).observations.stdout(),
        ["1"]
    );
    program
        .functions
        .replace_for_development("f", donor.functions.function("f").unwrap().clone())
        .unwrap();
    assert_eq!(
        crate::interp::run_scalar(&program).observations.stdout(),
        ["3"]
    );
}

#[test]
fn cfg_loop_ownership_fixpoint_has_nominal_boundary() {
    // F0-0002 §6.3 and §17.2 P09 require a fixed point on the backedge.
    let source = "fn f(b:bool){let r=mock_open(1);while b{drop(r);}} fn main(){}";
    assert!(matches!(
        compile_scalar("loop.nx", source),
        Err(CompileError::OutsideScalarSlice("ownership fixpoint"))
    ));
}

#[test]
fn cfg_missing_return_considers_every_normal_path() {
    // F0-0002 §6.2, §6.3, §17.1 and S11.
    for source in [
        "fn f(b:bool)->i64{if b{return 1;}} fn main(){}",
        "fn f(b:bool)->i64{while b{return 1;}} fn main(){}",
    ] {
        let CompileError::Rejected(errors) = compile_scalar("return.nx", source).unwrap_err()
        else {
            panic!("expected missing return: {source}")
        };
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].code, "NX-E1317");
    }
    let source = "fn f(b:bool)->i64{if b{return 1;}else{return 2;}} fn main(){assert_eq(f(true),1);assert_eq(f(false),2);}";
    let program = compile_scalar("return.nx", source).unwrap();
    let execution = crate::interp::run_scalar(&program);
    assert!(
        matches!(execution.outcome, crate::interp::Outcome::Unit),
        "{execution:?}"
    );
}

#[test]
fn cfg_expression_if_excludes_returning_continuations() {
    let source = "fn f(b:bool)->i64{return if b{return 3;}else{value:2;};} fn main(){assert_eq(f(true),3);assert_eq(f(false),2);}";
    let program = compile_scalar("value_return.nx", source).unwrap();
    let execution = crate::interp::run_scalar(&program);
    assert!(
        matches!(execution.outcome, crate::interp::Outcome::Unit),
        "{execution:?}"
    );
    let source = "fn f(b:bool)->i64{return if b{return 3;}else{return 2;};} fn main(){assert_eq(f(true),3);assert_eq(f(false),2);}";
    let program = compile_scalar("value_return.nx", source).unwrap();
    let execution = crate::interp::run_scalar(&program);
    assert!(
        matches!(execution.outcome, crate::interp::Outcome::Unit),
        "{execution:?}"
    );
}

#[test]
fn cfg_else_if_expression_selects_each_value() {
    let source = "fn f(x:i64)->i64{return if x<0{value:1;}else if x<2{value:2;}else{value:3;};} fn main(){assert_eq(f(0-1),1);assert_eq(f(1),2);assert_eq(f(3),3);}";
    let program = compile_scalar("chain.nx", source).unwrap();
    let execution = crate::interp::run_scalar(&program);
    assert!(
        matches!(execution.outcome, crate::interp::Outcome::Unit),
        "{execution:?}"
    );
}

#[test]
fn cfg_else_if_statement_selects_each_arm() {
    let source = "fn f(x:i64)->i64{var y=0;if x<0{y=1;}else if x<2{y=2;}else{y=3;}return y;} fn main(){assert_eq(f(0-1),1);assert_eq(f(1),2);assert_eq(f(3),3);}";
    let program = compile_scalar("chain.nx", source).unwrap();
    let execution = crate::interp::run_scalar(&program);
    assert!(
        matches!(execution.outcome, crate::interp::Outcome::Unit),
        "{execution:?}"
    );
}

#[test]
fn cfg_nested_while_recreates_and_cleans_body_local_each_iteration() {
    // F0-0002 §6.3 and §6.4: each body entry owns a fresh resource instance.
    let source = "fn main(){var x=0;while x<2{var y=0;while y<2{let r=mock_open(x+x+y);inspect(r);y=y+1;}x=x+1;}}";
    let program = compile_scalar("nested.nx", source).unwrap();
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
    let program = compile_scalar("quota.nx", "fn main(){let r=mock_open(1);while true{}}").unwrap();
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
    let source = "fn f()->i64{\nreturn if true{\nvalue: (\n1);\n}else{value:2;};\n}\nfn main(){assert_eq(f(),1);}";
    let program = compile_scalar("l02.nx", source).unwrap();
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
        ("fn main(){match true{true=>{},false=>{}}}", "match"),
        ("fn main(){for n in [1]{}}", "for_iteration"),
        ("fn main(){while false{break;}}", "loop_control"),
    ] {
        let CompileError::Rejected(errors) = compile_scalar("feature.nx", source).unwrap_err()
        else {
            panic!("expected deferred form")
        };
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].code, "NX-E0006");
        assert_eq!(errors[0].feature_id.as_deref(), Some(feature));
    }
    for source in ["fn main(){if true{}else{}}", "fn main(){while false{}}"] {
        let program = compile_scalar("feature.nx", source).unwrap();
        let execution = crate::interp::run_scalar(&program);
        assert!(
            matches!(execution.outcome, crate::interp::Outcome::Unit),
            "{execution:?}"
        );
    }
}

#[test]
fn cfg_else_if_all_returning_paths_need_no_tail_return() {
    let source = "fn f(x:i64)->i64{if x<0{return 1;}else if x<2{return 2;}else{return 3;}} fn main(){assert_eq(f(0-1),1);assert_eq(f(1),2);assert_eq(f(3),3);}";
    let program = compile_scalar("chain_return.nx", source).unwrap();
    let execution = crate::interp::run_scalar(&program);
    assert!(
        matches!(execution.outcome, crate::interp::Outcome::Unit),
        "{execution:?}"
    );
}

#[test]
fn cfg_nested_expression_if_all_returning_paths_need_no_value_slot() {
    let source = "fn f(x:i64)->i64{return if x<0{return 1;}else if x<2{return 2;}else{return 3;};} fn main(){assert_eq(f(0-1),1);assert_eq(f(1),2);assert_eq(f(3),3);}";
    let program = compile_scalar("chain_return.nx", source).unwrap();
    let execution = crate::interp::run_scalar(&program);
    assert!(
        matches!(execution.outcome, crate::interp::Outcome::Unit),
        "{execution:?}"
    );
}

#[test]
fn cfg_while_body_with_all_returning_arms_preserves_false_exit() {
    let source = "fn f(b:bool)->i64{while b{if true{return 1;}else{return 2;}}return 3;} fn main(){assert_eq(f(true),1);assert_eq(f(false),3);}";
    let program = compile_scalar("loop_return.nx", source).unwrap();
    let execution = crate::interp::run_scalar(&program);
    assert!(
        matches!(execution.outcome, crate::interp::Outcome::Unit),
        "{execution:?}"
    );
}

#[test]
fn cfg_unreachable_tail_still_checks_ownership_without_rejoining() {
    // F0-0002 §6.3: later statements use a separate hypothetical state.
    let source = "fn f()->i64{if true{return 1;}else{return 2;}let r=mock_open(1);drop(r);inspect(r);} fn main(){}";
    let CompileError::Rejected(errors) = compile_scalar("dead.nx", source).unwrap_err() else {
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
    let source = "fn f(b:bool)->MockResource{let a=mock_open(1);let c=mock_open(2);return if b{value:consume a;}else{value:consume c;};} fn main(){let x=f(true);inspect(x);}";
    let program = compile_scalar("resource_union.nx", source).unwrap();
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
    let source = "fn bump(inout x:i64){x=x+1;} fn main(){var a=0;if true{bump(inout a);}else{}assert_eq(a,1);}";
    let program = compile_scalar("branch_loan.nx", source).unwrap();
    let execution = crate::interp::run_scalar(&program);
    assert!(
        matches!(execution.outcome, crate::interp::Outcome::Unit),
        "{execution:?}"
    );
}

#[test]
fn cfg_return_from_loop_cleans_other_live_owners_before_result_delivery() {
    // F0-0002 §6.4: the result remains owned while callee locals close.
    let source = "fn f()->MockResource{let r=mock_open(1);while true{let s=mock_open(2);return consume s;}return consume r;} fn main(){let x=f();inspect(x);}";
    let program = compile_scalar("loop_return_resource.nx", source).unwrap();
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
    let source =
        "fn main(){var x=0;while x<1000{let a=[1,2,3,4,5,6,7,8];assert_eq(len(a),8);x=x+1;}}";
    let program = compile_scalar("loop_cells.nx", source).unwrap();
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
    let e = execute(&format!("fn main(){{let a=[{first}];drop(a);}}"));
    assert_eq!(e.outcome, Outcome::Unit);
    assert!(e.diagnostics.is_empty());
    let e = execute(&format!(
        "fn main(){{let a=[{}];drop(a);let b=[{}];print(len(b));}}",
        zeros(30_000),
        zeros(40_000)
    ));
    assert_eq!(e.outcome, Outcome::Unit);
    assert_eq!(e.observations.stdout(), vec!["40000"]);
}

#[test]
fn drop_releases_one_copy_without_invalidating_another_owner() {
    let e = execute("fn main(){let a=[1,2];let b=a;drop(a);assert_eq(b,[1,2]);print(len(b));}");
    assert_eq!(e.outcome, Outcome::Unit);
    assert_eq!(e.observations.stdout(), vec!["2"]);
    let e = execute("fn main(){{let a=[1,2];drop(a);}print(3);}");
    assert_eq!(e.outcome, Outcome::Unit);
    assert_eq!(e.observations.stdout(), vec!["3"]);
}
#[test]
fn live_cell_boundary_is_exact_with_and_without_explicit_drop() {
    let data = zeros(65_535);
    let dropped = execute(&format!("fn main(){{let a=[{data}];drop(a);}}"));
    assert_eq!(dropped.outcome, Outcome::Unit);
    let retained = execute(&format!("fn main(){{let a=[{data}];}}"));
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
        "fn main(){{let a=[{}];let b=[0];let u=drop(b);let c=0;}}",
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
        "print(len(a[0]));",
        "assert_eq(a[0],a[0]);",
        "assert_eq(a[0]==a[0],true);",
    ] {
        let e = execute(&format!("fn main(){{let a=[[{large}],[1]];{tail}}}"));
        assert_eq!(e.outcome, Outcome::Unit, "{tail}");
        assert!(e.diagnostics.is_empty());
        if tail.starts_with("print") {
            assert_eq!(e.observations.stdout(), vec!["40000"]);
        }
    }
    let e = execute(&format!("fn main(){{let a=[[{large}],[1]];let b=a[0];}}"));
    assert_eq!(e.outcome, Outcome::SimulatorFault);
    assert_eq!(e.diagnostics[0].code, "R0002");
    assert_eq!(
        e.diagnostics[0].data.get("observed"),
        Some(&crate::diagnostic::DataValue::UInt(80_005))
    );
}
#[test]
fn borrowed_index_reports_invalid_path_before_reading_selected_value() {
    let e = execute("fn main(){let a=[[1]];print(len(a[2]));}");
    assert_eq!(e.outcome, Outcome::SimulatorFault);
    assert_eq!(e.diagnostics[0].code, "R0102");
    assert_eq!(e.diagnostics[0].operation.as_str(), "value.index");
    let mut p = compile_scalar("fixture.nx", "fn main(){let a=[[1]];print(len(a[2]));}").unwrap();
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
    let overflow = execute("fn main(){var a=[0];a[-1]=9223372036854775807+1;}");
    assert_eq!(overflow.outcome, Outcome::SimulatorFault);
    assert_eq!(overflow.diagnostics[0].code, "R0101");
    assert!(overflow.observations.stdout().is_empty());
    let invalid = execute("fn main(){var a=[0];a[-1]=2;}");
    assert_eq!(invalid.outcome, Outcome::SimulatorFault);
    assert_eq!(invalid.diagnostics[0].code, "R0102");
    assert_eq!(invalid.diagnostics[0].operation.as_str(), "value.index");
    let valid = execute("fn main(){var a=[0];a[0]=2;print(a[0]);}");
    assert_eq!(valid.outcome, Outcome::Unit);
    assert_eq!(valid.observations.stdout(), vec!["2"]);
    let nested = execute("fn main(){var a=[[0]];a[-1][0]=9223372036854775807+1;}");
    assert_eq!(nested.diagnostics[0].code, "R0101");
    for source in [
        "fn main(){let a:Array<i64>=[];print(a[0]);}",
        "fn main(){let a=[1];print(a[9223372036854775807]);}",
        "fn main(){print([1][-1]);}",
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
    let invalid = execute("fn main(){let a=[[1]];print(a[-1][9223372036854775807+1]);}");
    assert_eq!(invalid.outcome, Outcome::SimulatorFault);
    assert_eq!(invalid.diagnostics[0].code, "R0102");
    let valid = execute("fn main(){let a=[[1]];print(a[0][0]);}");
    assert_eq!(valid.outcome, Outcome::Unit);
    assert_eq!(valid.observations.stdout(), vec!["1"]);
}
#[test]
fn array_literals_and_indices_change_function_hash() {
    let hash = |s| {
        compile_scalar("h.nx", s)
            .unwrap()
            .functions()
            .function("main")
            .unwrap()
            .content_hash
    };
    assert_ne!(
        hash("fn main(){let a=[1];print(a[0]);}"),
        hash("fn main(){let a=[2];print(a[0]);}")
    );
    assert_ne!(
        hash("fn main(){let a=[1,2];print(a[0]);}"),
        hash("fn main(){let a=[1,2];print(a[1]);}")
    );
    assert_eq!(
        hash("fn main(){let a=[1,2];print(a[0]);}"),
        hash("fn main(){  let a = [1, 2]; print(a[0]); }")
    );
}
#[test]
fn binary_instruction_hash_matches_fixed_canonical_fnv_encoding() {
    // Independently encoded: Unit result, zero locals, six instructions
    // (1, 2, Add, Print, Drop, implicit Unit), empty cleanup, return temp 4.
    let p = compile_scalar("h.nx", "fn main(){print(1+2);}").unwrap();
    assert_eq!(
        p.functions().function("main").unwrap().content_hash,
        0x20c7_71da_7f5d_f960
    );
}
#[test]
fn ordinary_resource_parameter_borrows_across_frame_and_closes_once() {
    let e = execute(
        "fn leer(r:MockResource){inspect(r);inspect(r);} fn main(){let r=mock_open(1);leer(r);inspect(r);}",
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
        "fn take(consume r:MockResource){inspect(r);} fn main(){let r=mock_open(4);take(consume r);}",
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
        "fn make()->MockResource{return mock_open(3);} fn main(){let r=make();inspect(r);}",
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
        "fn f(consume r:MockResource,n:i64){print(n);} fn main(){f(mock_open(1),9223372036854775807+1);}",
    );
    assert_eq!(e.outcome, Outcome::SimulatorFault);
    assert_eq!(e.diagnostics[0].code, "R0101");
    assert_eq!(e.observations.events(), ["open:0:1", "close:0:1"]);
    assert!(e.observations.stdout().is_empty());
}
#[test]
fn named_arguments_evaluate_in_written_order() {
    let e = execute(
        "fn mark(n:i64)->i64{print(n);return n;} fn f(a:i64,b:i64){} fn main(){f(b:mark(2),a:mark(1));}",
    );
    assert_eq!(e.outcome, Outcome::Unit);
    assert_eq!(e.observations.stdout(), ["2", "1"]);
}
#[test]
fn resource_copy_is_rejected_and_explicit_transfer_runs() {
    let negative = "fn main(){let r=mock_open(1);let s=r;}";
    let Err(CompileError::Rejected(errors)) = compile_scalar("p.nx", negative) else {
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
    assert_eq!(
        d.primary_span.as_ref().map(|s| (s.start, s.end)),
        Some((35, 36))
    );
    d.validate("p.nx", negative).unwrap();
    let e = execute("fn main(){let r=mock_open(1);let s=consume r;inspect(s);}");
    assert_eq!(
        e.observations.events(),
        ["open:0:1", "inspect:0:1", "close:0:1"]
    );
}
#[test]
fn use_after_consume_or_drop_reports_consumption_site() {
    for (source, primary, consumption) in [
        (
            "fn main(){let r=mock_open(1);let s=consume r;inspect(r);}",
            (53, 54),
            (35, 44),
        ),
        ("fn main(){let x=1;drop(x);print(x);}", (32, 33), (18, 25)),
    ] {
        let Err(CompileError::Rejected(errors)) = compile_scalar("p.nx", source) else {
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
        assert_eq!(
            d.primary_span.as_ref().map(|s| (s.start, s.end)),
            Some(primary)
        );
        assert!(
            d.related_spans
                .iter()
                .any(|r| r.role == crate::diagnostic::RelatedRole::Consumption
                    && (r.span.start, r.span.end) == consumption)
        );
        d.validate("p.nx", source).unwrap();
    }
}
#[test]
fn read_parameter_cannot_be_consumed() {
    let source = "fn f(x:MockResource)->MockResource{return consume x;} fn main(){}";
    let Err(CompileError::Rejected(errors)) = compile_scalar("p.nx", source) else {
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
    d.validate("p.nx", source).unwrap();
    let e = execute(
        "fn f(consume x:MockResource)->MockResource{return consume x;} fn main(){let r=f(mock_open(1));inspect(r);}",
    );
    assert_eq!(
        e.observations.events(),
        ["open:0:1", "inspect:0:1", "close:0:1"]
    );
}
#[test]
fn adr_gap_call_frame_limit_primary_uses_rejected_call_span() {
    let source = "fn down(){down();} fn main(){down();}";
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
        Some((10, 16))
    );
    d.validate("fixture.nx", source).unwrap();
}
#[test]
fn unreachable_tail_is_checked_without_emitting_its_instructions() {
    let source = "fn main(){return;let a=[1];a[0]=2;}";
    let Err(CompileError::Rejected(errors)) = compile_scalar("dead.nx", source) else {
        panic!("expected ownership diagnostic")
    };
    assert_eq!(errors.len(), 1);
    assert_eq!(errors[0].code, "NX-E0105");
    errors[0].validate("dead.nx", source).unwrap();
    let e = execute("fn main(){return;print(9);}");
    assert_eq!(e.outcome, Outcome::Unit);
    assert!(e.observations.stdout().is_empty());
}
#[test]
fn shadowed_resources_close_in_reverse_scope_order() {
    let e = execute("fn main(){let r=mock_open(1);{let r=mock_open(2);inspect(r);}inspect(r);}");
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
    let e = execute("fn main(){var r=mock_open(1);drop(r);r=mock_open(2);inspect(r);}");
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
        "fn copia(a:Array<i64>)->Array<i64>{return a;} fn main(){var a=[1];let b=copia(a);a[0]=2;assert_eq(b,[1]);}",
    );
    assert_eq!(e.outcome, Outcome::Unit);
    assert!(e.observations.observations().is_empty());
}
#[test]
fn call_target_named_value_continues_across_lf() {
    let source =
        "fn value(x:i64)->i64{return x;}\nfn main(){let x=7;let y=value\n(x);assert_eq(y,7);}";
    let ast = parse::parse("fixture.nx", source).unwrap();
    let crate::frontend::parse::Statement::Let {
        value: crate::frontend::parse::Expr::Call { span, .. },
        ..
    } = &ast
        .functions
        .iter()
        .find(|f| f.name == "main")
        .unwrap()
        .body
        .statements[1]
    else {
        panic!("L03 call")
    };
    assert_eq!((span.start, span.end), (56, 65));
    let e = execute(source);
    assert_eq!(e.outcome, Outcome::Unit);
    assert!(e.observations.observations().is_empty());
}
#[test]
fn explicit_return_parenthesis_continues_across_lf() {
    let source = "fn f()->i64{return (\n1);} fn main(){assert_eq(f(),1);}";
    let e = execute(source);
    assert_eq!(e.outcome, Outcome::Unit);
    assert!(e.observations.observations().is_empty());
}
#[test]
fn drop_destroys_complete_binding_or_temporary_and_rejects_projection() {
    let e = execute("fn main(){let a=[1];drop(a);drop([1][0]);drop(mock_open(9));}");
    assert_eq!(e.outcome, Outcome::Unit);
    assert_eq!(e.observations.events(), ["open:0:9", "close:0:9"]);
    let source = "fn main(){let a=[1];drop(a[0]);}";
    let Err(CompileError::Rejected(errors)) = compile_scalar("drop.nx", source) else {
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
    errors[0].validate("drop.nx", source).unwrap();
}
#[test]
fn caller_hash_tracks_referenced_name_and_signature_but_not_unrelated_body() {
    let hash = |source: &str| {
        compile_scalar("hash.nx", source)
            .unwrap()
            .functions()
            .function("main")
            .unwrap()
            .content_hash
    };
    let base = hash("fn f(x:i64)->i64{return x;} fn main(){f(1);}");
    assert_eq!(base, hash("fn f(x:i64)->i64{return x+1;} fn main(){f(1);}"));
    assert_ne!(base, hash("fn g(x:i64)->i64{return x;} fn main(){g(1);}"));
    assert_ne!(base, hash("fn f(y:i64)->i64{return y;} fn main(){f(1);}"));
    assert_ne!(
        base,
        hash("fn f(x:i64)->bool{return true;} fn main(){f(1);}")
    );
    assert_ne!(
        base,
        hash("fn f(consume x:i64)->i64{return x;} fn main(){f(1);}")
    );
}
#[test]
fn parameter_name_and_argument_slot_are_encoded_in_canonical_hash() {
    let a = compile_scalar("hash.nx", "fn f(x:i64)->i64{return 1;} fn main(){f(2);}").unwrap();
    let b = compile_scalar("hash.nx", "fn f(y:i64)->i64{return 1;} fn main(){f(2);}").unwrap();
    assert_ne!(
        a.functions().function("f").unwrap().content_hash,
        b.functions().function("f").unwrap().content_hash
    );
    let p = compile_scalar("hash.nx", "fn f(a:i64,b:i64){} fn main(){f(a:1,b:2);}").unwrap();
    let mut f = p.functions().function("main").unwrap().clone();
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
    let mut f = p.functions().function("main").unwrap().clone();
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
    let p = compile_scalar("hash.nx", "fn main(){let x=1;let y=consume x;}").unwrap();
    let mut f = p.functions().function("main").unwrap().clone();
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
    let p = compile_scalar("hash.nx", "fn f(x:i64){} fn main(){f(1);}").unwrap();
    assert_eq!(
        p.functions().function("main").unwrap().content_hash,
        0xdde0_95c7_2e5f_e0ca
    );
}
#[test]
fn failed_development_substitution_restores_previous_function() {
    let mut receiver = compile_scalar("r.nx", "fn extra(){} fn main(){print(1);}").unwrap();
    let donor = compile_scalar("d.nx", "fn helper(){} fn main(){helper();}").unwrap();
    let before = receiver.functions().function("main").unwrap().content_hash;
    assert_eq!(
        receiver.substitute_for_development("main", &donor),
        Err("unknown callee")
    );
    assert_eq!(
        receiver.functions().function("main").unwrap().content_hash,
        before
    );
    let e = run_scalar(&receiver);
    assert_eq!(e.observations.stdout(), ["1"]);
}
#[test]
fn independent_ownership_errors_accumulate_in_source_order() {
    let source = "fn main(){let a=mock_open(1);let b=mock_open(2);let x=a;let y=b;}";
    let Err(CompileError::Rejected(errors)) = compile_scalar("multi.nx", source) else {
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
        [54, 62]
    );
    for d in errors {
        d.validate("multi.nx", source).unwrap();
    }
}
#[test]
fn user_function_named_drop_has_ordinary_read_semantics() {
    let e = execute("fn drop(x:i64){} fn main(){let a=[1];drop(a[0]);assert_eq(a[0],1);}");
    assert_eq!(e.outcome, Outcome::Unit);
    assert!(e.observations.observations().is_empty());
}
#[test]
fn consumed_temporary_borrowed_by_read_parameter_closes_after_call() {
    let e = execute(
        "fn read(r:MockResource){inspect(r);} fn main(){let r=mock_open(5);read(consume r);}",
    );
    assert_eq!(e.outcome, Outcome::Unit);
    assert_eq!(
        e.observations.events(),
        ["open:0:5", "inspect:0:5", "close:0:5"]
    );
    let source =
        "fn read(r:MockResource){} fn main(){let r=mock_open(5);read(consume r);inspect(r);}";
    let Err(CompileError::Rejected(errors)) = compile_scalar("consume.nx", source) else {
        panic!("expected consumed use")
    };
    assert_eq!(errors.len(), 1);
    assert_eq!(
        (&*errors[0].code, errors[0].subcause.as_deref()),
        ("NX-E0202", Some("definitely_consumed"))
    );
    errors[0].validate("consume.nx", source).unwrap();
}
#[test]
fn simulator_fault_cleans_callee_then_caller_resources() {
    let e = execute(
        "fn failer(consume r:MockResource){let s=mock_open(2);fail();} fn main(){let r=mock_open(1);failer(consume r);}",
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
    let e = execute("fn main(){let r=mock_blocking_open(7);inspect(r);}");
    assert_eq!(e.outcome, Outcome::Unit);
    assert_eq!(
        e.observations.events(),
        ["open:0:7", "inspect:0:7", "close:0:7"]
    );
}
#[test]
fn frame_limit_unwinds_shared_resource_once() {
    let source = "fn f(r:MockResource){f(r);} fn main(){let r=mock_open(1);f(r);}";
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
    let source = format!("fn f(){{}} fn main(){{{}}}", "f();".repeat(300));
    let e = execute(&source);
    assert_eq!(e.outcome, Outcome::Unit);
    assert!(e.diagnostics.is_empty());
    assert!(e.observations.observations().is_empty());
}
#[test]
fn frame_limit_denies_acquisition_in_frame_257() {
    let e = execute("fn f(){let r=mock_open(1);f();} fn main(){f();}");
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
    let e = execute("fn main(){var r=mock_open(1);r=mock_open(2);inspect(r);}");
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
    let e = execute("fn main(){var a=[[1]];let b=a[0];a[0][0]=2;assert_eq(b,[1]);}");
    assert_eq!(e.outcome, Outcome::Unit);
    assert!(e.diagnostics.is_empty());
}
#[test]
fn repeated_drop_reports_consumed_origin_without_second_close() {
    let source = "fn main(){let r=mock_open(1);drop(r);drop(r);}";
    let Err(CompileError::Rejected(errors)) = compile_scalar("drop.nx", source) else {
        panic!("expected consumed use")
    };
    assert_eq!(errors.len(), 1);
    assert_eq!(
        (&*errors[0].code, errors[0].subcause.as_deref()),
        ("NX-E0202", Some("definitely_consumed"))
    );
    errors[0].validate("drop.nx", source).unwrap();
}
#[test]
fn read_parameter_rejects_write_and_drop_without_execution() {
    for (source, access) in [
        ("fn f(x:i64){x=2;} fn main(){}", "write"),
        ("fn f(x:i64){drop(x);} fn main(){}", "drop"),
    ] {
        let Err(CompileError::Rejected(errors)) = compile_scalar("permission.nx", source) else {
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
        errors[0].validate("permission.nx", source).unwrap();
    }
}
#[test]
fn consumed_nested_local_does_not_receive_second_cleanup() {
    let e = execute(
        "fn take(consume r:MockResource){} fn main(){{let r=mock_open(1);take(consume r);}}",
    );
    assert_eq!(e.outcome, Outcome::Unit);
    assert_eq!(e.observations.events(), ["open:0:1", "close:0:1"]);
}
#[test]
fn nested_drop_and_reinitialization_use_one_liveness_record() {
    let e = execute("fn main(){{var r=mock_open(1);drop(r);r=mock_open(2);}}");
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
            "fn f(x:i64){} fn main(){f();}",
            "NX-E0111",
            Some("missing_argument"),
            "fn f(x:i64){} fn main(){f(1);}",
        ),
        (
            "S02",
            "fn f(x:i64){} fn main(){f(1,2);}",
            "NX-E0111",
            Some("excess_positional_argument"),
            "fn f(x:i64){} fn main(){f(1);}",
        ),
        (
            "S03",
            "fn f(x:i64){} fn main(){f(y:1);}",
            "NX-E0111",
            Some("unknown_argument_label"),
            "fn f(x:i64){} fn main(){f(x:1);}",
        ),
        (
            "S04",
            "fn f(x:i64){} fn main(){f(x:1,x:2);}",
            "NX-E0111",
            Some("duplicate_argument"),
            "fn f(x:i64){} fn main(){f(x:1);}",
        ),
        (
            "S05",
            "fn f(x:i64,y:i64){} fn main(){f(x:1,2);}",
            "NX-E0111",
            Some("positional_after_named"),
            "fn f(x:i64,y:i64){} fn main(){f(x:1,y:2);}",
        ),
        (
            "S07",
            "fn main(){let f=1;f();}",
            "NX-E0111",
            Some("non_callable_target"),
            "fn f(){} fn main(){f();}",
        ),
        (
            "S12",
            "fn f()->i64{return true;} fn main(){}",
            "NX-E0102",
            Some("type_mismatch"),
            "fn f()->i64{return 1;} fn main(){assert_eq(f(),1);}",
        ),
        (
            "S13",
            "fn f(){} fn f(){} fn main(){}",
            "NX-E0104",
            None,
            "fn f(){} fn g(){} fn main(){}",
        ),
        (
            "S14",
            "fn f(x:i64,x:i64){} fn main(){}",
            "NX-E0104",
            None,
            "fn f(x:i64,y:i64){} fn main(){f(1,2);}",
        ),
        (
            "S15",
            "fn main(){ausente();}",
            "NX-E0101",
            None,
            "fn ausente(){} fn main(){ausente();}",
        ),
        (
            "S19",
            "fn main(){let r=mock_open(1);let b=r==r;}",
            "NX-E0102",
            Some("type_mismatch"),
            "fn main(){let r=mock_open(1);inspect(r);}",
        ),
        (
            "S24",
            "fn f(x:i64){let x=1;} fn main(){}",
            "NX-E0104",
            None,
            "fn f(x:i64){{let x=1;}} fn main(){f(1);}",
        ),
        (
            "S26",
            "fn main(){let x=1;inspect(x);}",
            "NX-E0102",
            Some("resource_required"),
            "fn main(){let x=mock_open(1);inspect(x);}",
        ),
        (
            "S29",
            "fn main(){let a=[1];drop(a[0]);}",
            "NX-E0006",
            None,
            "fn main(){let a=[1];drop(a);}",
        ),
        (
            "B07",
            "fn f()->i64{return 1 } fn main(){}",
            "NX-E0005",
            None,
            "fn f()->i64{return 1;} fn main(){assert_eq(f(),1);}",
        ),
    ];
    for (id, negative, code, subcause, positive) in rows {
        let Err(CompileError::Rejected(errors)) = compile_scalar("rows.nx", negative) else {
            panic!("{id} negative accepted")
        };
        assert_eq!(errors.len(), 1, "{id}");
        assert_eq!(
            (&*errors[0].code, errors[0].subcause.as_deref()),
            (*code, *subcause),
            "{id}"
        );
        errors[0].validate("rows.nx", negative).unwrap();
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
            "fn f(x:i64){} fn main(){⟦f⟧();}",
            "arguments",
            "call.bind",
        ),
        (
            "S02",
            "fn f(x:i64){} fn main(){f(1,⟦2⟧);}",
            "arguments",
            "call.bind",
        ),
        (
            "S03",
            "fn f(x:i64){} fn main(){f(⟦y⟧:1);}",
            "arguments",
            "call.bind",
        ),
        (
            "S04",
            "fn f(x:i64){} fn main(){f(x:1,⟦x⟧:2);}",
            "arguments",
            "call.bind",
        ),
        (
            "S05",
            "fn f(x:i64,y:i64){} fn main(){f(x:1,⟦2⟧);}",
            "arguments",
            "call.bind",
        ),
        ("S07", "fn main(){let f=1;⟦f⟧();}", "arguments", "call.bind"),
        (
            "S12",
            "fn f()->i64{return ⟦true⟧;} fn main(){}",
            "types",
            "types.check",
        ),
        (
            "S13",
            "fn f(){} fn ⟦f⟧(){} fn main(){}",
            "names",
            "names.declare",
        ),
        (
            "S14",
            "fn f(x:i64,⟦x⟧:i64){} fn main(){}",
            "names",
            "names.declare",
        ),
        ("S15", "fn main(){⟦ausente⟧();}", "names", "names.resolve"),
        (
            "S19",
            "fn main(){let r=mock_open(1);let b=r⟦==⟧r;}",
            "types",
            "types.check",
        ),
        (
            "S24",
            "fn f(x:i64){let ⟦x⟧=1;} fn main(){}",
            "names",
            "names.declare",
        ),
        (
            "S26",
            "fn main(){let x=1;inspect(⟦x⟧);}",
            "types",
            "types.check",
        ),
        (
            "S29",
            "fn main(){let a=[1];drop(⟦a[0]⟧);}",
            "arguments",
            "call.bind",
        ),
        (
            "B07",
            "fn f()->i64{return 1 ⟦}⟧ fn main(){}",
            "parse",
            "syntax.parse",
        ),
    ];
    for (id, marked, stage, operation) in rows {
        let (before, rest) = marked.split_once('⟦').unwrap();
        let (primary, after) = rest.split_once('⟧').unwrap();
        let source = format!("{before}{primary}{after}");
        let Err(CompileError::Rejected(errors)) = compile_scalar("rows.nx", &source) else {
            panic!("{id} accepted")
        };
        assert_eq!(errors.len(), 1, "{id}");
        let d = &errors[0];
        assert_eq!(
            d.primary_span.as_ref().map(|s| (s.start, s.end)),
            Some((before.len() as u64, (before.len() + primary.len()) as u64)),
            "{id}"
        );
        assert_eq!(
            (d.stage.as_str(), d.operation.as_str()),
            (stage, operation),
            "{id}"
        );
        d.validate("rows.nx", &source).unwrap();
    }
}
#[test]
fn i05_feature_exclusions_have_executable_repairs() {
    let rows: &[(&str, &str, &str, &str)] = &[
        (
            "F01",
            "fn main(){let r=mock_open(1);let s=move r;}",
            "legacy_move",
            "fn main(){let r=mock_open(1);let s=consume r;}",
        ),
        (
            "F02",
            "fn f(){return 1;} fn main(){}",
            "inferred_non_unit_result",
            "fn f()->i64{return 1;} fn main(){assert_eq(f(),1);}",
        ),
        (
            "F07",
            "fn f(x:i64=1){} fn main(){f(1);}",
            "default_argument",
            "fn f(x:i64){} fn main(){f(1);}",
        ),
        (
            "F09",
            "fn f(){} fn main(){let g=f;}",
            "function_value",
            "fn f(){} fn main(){f();}",
        ),
        (
            "F10",
            "fn f(){} fn main(){consume f();}",
            "receiver_mode",
            "fn f(){} fn main(){f();}",
        ),
        (
            "F11",
            "fn main(){let a=[1];let x=consume a[0];}",
            "partial_transfer",
            "fn main(){let a=[1];let x=a[0];assert_eq(x,1);}",
        ),
        (
            "F12",
            "fn main(){let a=[mock_open(1)];}",
            "resource_array",
            "fn main(){let a=mock_open(1);inspect(a);}",
        ),
        (
            "F20",
            "fn f(x:i64)->i64 from(x){return x;} fn main(){}",
            "provenance_clauses",
            "fn f(x:i64)->i64{return x;} fn main(){assert_eq(f(1),1);}",
        ),
        (
            "F29",
            "fn f(x:u32){} fn main(){}",
            "unsupported_type",
            "fn f(x:i64){} fn main(){f(1);}",
        ),
        (
            "F30",
            "fn f(x:String){} fn main(){}",
            "unsupported_type",
            "fn f(x:Array<i64>){} fn main(){f([1]);}",
        ),
        (
            "F31",
            "fn f(x:app.Error){} fn main(){}",
            "unsupported_type",
            "fn f(x:i64){} fn main(){f(1);}",
        ),
    ];
    for (id, negative, feature, positive) in rows {
        let Err(CompileError::Rejected(errors)) = compile_scalar("features.nx", negative) else {
            panic!("{id} accepted")
        };
        assert_eq!(errors.len(), 1, "{id}");
        assert_eq!(errors[0].code, "NX-E0006", "{id}");
        assert_eq!(errors[0].feature_id.as_deref(), Some(*feature), "{id}");
        errors[0].validate("features.nx", negative).unwrap();
        let e = execute(positive);
        assert_eq!(e.outcome, Outcome::Unit, "{id}");
        assert!(e.diagnostics.is_empty(), "{id}");
    }
}
#[test]
fn hash_distinguishes_less_from_equal_with_same_operands() {
    let hash = |operator| {
        let source = format!("fn main(){{print(1{operator}2);}}");
        compile_scalar("h.nx", &source)
            .unwrap()
            .functions()
            .function("main")
            .unwrap()
            .content_hash
    };
    assert_ne!(hash("<"), hash("=="));
}
#[test]
fn function_hash_includes_local_mutability() {
    let hash = |source| {
        compile_scalar("h.nx", source)
            .unwrap()
            .functions()
            .function("main")
            .unwrap()
            .content_hash
    };
    assert_ne!(
        hash("fn main(){let x=1;print(x);}"),
        hash("fn main(){var x=1;print(x);}")
    );
}

#[test]
fn hash_includes_write_index_root_and_read_index_path() {
    let program = compile_scalar(
        "h.nx",
        "fn main(){var a=[1,2];var b=[3,4];a[0]=5;let c=a[1];}",
    )
    .unwrap();
    let original = program.functions().function("main").unwrap().clone();
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
    let mut p = compile_scalar("h.nx", "fn main(){let a=[0];print(a[2]);}").unwrap();
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
    let e = execute("fn main(){assert_eq([1,2]==[1,2],true);assert_eq(len([1,2]),2);}");
    assert_eq!(e.outcome, Outcome::Unit);
    assert!(e.diagnostics.is_empty());
}
#[test]
fn annotated_empty_array_len_and_array_comparison_are_executable() {
    let source = "fn main(){let empty:Array<i64>=[];assert_eq(len(empty),0);let a=[1,2];assert_eq(a==[1,2],true);assert_eq(a<[1],false);}";
    let valid = source.replace("assert_eq(a<[1],false);", "assert_eq(len(a),2);");
    let e = execute(&valid);
    assert_eq!(e.outcome, Outcome::Unit);
    assert!(e.diagnostics.is_empty());
    assert!(matches!(
        compile_scalar("fixture.nx", source),
        Err(CompileError::Rejected(_))
    ));
}
fn doubling_array_source(last: usize, tail: &str) -> String {
    let mut source = String::from("fn main(){let a0=[0];");
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
        "assert_eq(a13,a13);assert_eq(a13==a13,true);assert_eq(len(a13),2);print(a13);",
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
        "let selected=a13[0];assert_eq(len(selected),2);",
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
    let e = execute("fn main(){var a=[[1]];a[0]=[];assert_eq(len(a[0]),0);}");
    assert_eq!(e.outcome, Outcome::Unit);
}
#[test]
fn nested_replacement_updates_cached_cell_sizes_and_preserves_copies() {
    let e = execute(
        "fn main(){var a=[[[1]]];a[0][0]=[2,3];let b=a;a[0]=[[4]];assert_eq(b,[[[2,3]]]);print(a);}",
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
fn corpus_immutable_write_uses_current_worktree_byte_span() {
    let source = include_str!("../../corpus/fail/immutable_write.nx");
    let CompileError::Rejected(errors) = compile_scalar("immutable_write.nx", source).unwrap_err()
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
    d.validate("immutable_write.nx", source).unwrap();
}
#[test]
fn heterogeneous_array_rejection_has_homogeneous_positive() {
    let source = include_str!("../../corpus/fail/mixed_array.nx");
    let CompileError::Rejected(errors) = compile_scalar("mixed_array.nx", source).unwrap_err()
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
    errors[0].validate("mixed_array.nx", source).unwrap();
    let e = execute("fn main(){let a=[1,2];assert_eq(a,[1,2]);}");
    assert_eq!(e.outcome, Outcome::Unit);
}
#[test]
fn array_fixtures_for_empty_array_comparison_index_and_len_have_complete_positives() {
    for source in [
        "fn main(){let xs:Array<i64>=[];assert_eq(len(xs),0);}",
        "fn main(){let b=[1]==[2];assert_eq(b,false);}",
        "fn main(){let x=[1][0];assert_eq(x,1);}",
        "fn main(){let a=[1];len(a);}",
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
        let negative = format!("fn main(){{let x=1{symbol}2;}}");
        let CompileError::Rejected(errors) = compile_scalar("fixture.nx", &negative).unwrap_err()
        else {
            panic!("lexical rejection {symbol}")
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
            ("NX-E0001", None, "lex", "syntax.lex")
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
            ("invalid", Some(spelling.as_str()), 17, 18)
        );
        assert_eq!(d.parser_context.as_deref(), Some("lexical_token"));
        assert_eq!(d.expected_tokens, ["valid_token"]);
        assert_eq!(
            d.expected_tokens_kind.map(|x| x.as_str()),
            Some("lexical_category")
        );
        assert_eq!(
            d.recognized_prefix.as_ref().map(|s| (s.start, s.end)),
            Some((0, 17))
        );
        assert!(d.data.is_empty());
        d.validate("fixture.nx", &negative).unwrap();
        let e = execute("fn main(){let x=1+2;}");
        assert_eq!(e.outcome, Outcome::Unit);
        assert!(e.diagnostics.is_empty());
        assert!(e.observations.observations().is_empty());
    }
    let value = execute("fn main(){let x=1+2;assert_eq(x,3);}");
    assert_eq!(value.outcome, Outcome::Unit);
}
#[test]
fn adr_gap_runtime_index_primary_tracks_whole_index_operation() {
    let read = execute("fn main(){let a=[0];print(a[2]);}");
    assert_eq!(read.diagnostics[0].code, "R0102");
    assert_eq!(
        read.diagnostics[0]
            .primary_span
            .as_ref()
            .map(|s| (s.start, s.end)),
        Some((26, 30))
    );
    let write = execute("fn main(){var a=[0];a[2]=1;}");
    assert_eq!(write.diagnostics[0].code, "R0102");
    assert_eq!(
        write.diagnostics[0]
            .primary_span
            .as_ref()
            .map(|s| (s.start, s.end)),
        Some((20, 27))
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
            "fn main(){{var a=[[{},{}],[{},{}]];var b=a;b[{row}][{col}]={replacement};print(a);print(b);}}",
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
        "fn main(){var a=[[1,2],[3,4]];a[0][1]=9;print(a[0][1]);}",
        "fn main(){let a:Array<Array<i64>>=[[1],[2]];print(len(a));}",
        "fn main(){let a=[true,false];assert_eq(a,[true,false]);}",
        "fn main(){var a=[0];a[-1]=9223372036854775807+1;}",
    ];
    for k in 0..20_000usize {
        let base = templates[k % templates.len()];
        let mut bytes = base.as_bytes().to_vec();
        let pos = (k.wrapping_mul(7919) + k / 7) % bytes.len();
        bytes[pos] = b"[](){};,+-=<>0123456789x"[k % 23];
        let source = String::from_utf8(bytes).unwrap();
        match compile_scalar("fuzz.nx", &source) {
            Ok(program) => {
                let e = run_scalar(&program);
                assert!(!matches!(e.outcome, Outcome::InternalError(_)), "{source}");
                for d in e.diagnostics {
                    d.validate("fuzz.nx", &source).unwrap();
                }
            }
            Err(CompileError::Rejected(ds)) => {
                for d in ds {
                    d.validate("fuzz.nx", &source).unwrap();
                }
            }
            Err(CompileError::OutsideScalarSlice(_)) => {}
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
                        "fn main(){{let a={}1{};print(len(a));}}",
                        "[".repeat(depth),
                        "]".repeat(depth)
                    );
                    match compile_scalar("stack.nx", &source) {
                        Ok(p) => {
                            let e = run_scalar(&p);
                            assert_eq!(e.outcome, Outcome::Unit, "depth {depth}");
                            assert_eq!(e.observations.stdout(), vec!["1"]);
                        }
                        Err(CompileError::Rejected(ds)) if depth > 120 => {
                            for d in ds {
                                d.validate("stack.nx", &source).unwrap();
                            }
                        }
                        Err(CompileError::OutsideScalarSlice(_)) if depth > 120 => {}
                        Err(CompileError::Rejected(ds)) => panic!("admitted depth {depth}: {ds:?}"),
                        Err(CompileError::OutsideScalarSlice(reason)) => {
                            panic!("admitted depth {depth}: {reason}")
                        }
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
    let patterns = [
        "a[0][0]=1;",
        "a[1]=[3,4];",
        "assert_eq(a[0],[1,2]);",
        "assert_eq(a,a);",
        "assert_eq(len(a),2);",
        "print(a[0][1]);",
        "[1,2,3];",
        "a[0][1]=a[1][0];",
        "{let b=a;}",
        "print(len(a));",
    ];
    for (family, pattern) in patterns.iter().enumerate() {
        let mut traffic = Vec::new();
        for size in [262_144usize, 1_048_576] {
            let mut source = String::from("fn main(){var a=[[1,2],[3,4]];");
            while source.len() + pattern.len() + 3 <= size {
                source.push_str(pattern);
            }
            source.push('}');
            source.push_str("//");
            source.push_str(&"a".repeat(size - source.len()));
            let ast = parse::parse("dead_scale_counter.nx", &source).unwrap();
            crate::test_alloc::reset("dead_scale_counter.nx");
            let p = super::lower::compile_ast(&ast, &source).unwrap();
            assert_eq!(p.functions().functions().len(), 1);
            traffic.push(crate::test_alloc::snapshot("dead_scale_counter.nx"));
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
            let ast = parse::parse("dead_scale_counter.nx", &source).unwrap();
            crate::test_alloc::reset("dead_scale_counter.nx");
            let program = super::lower::compile_ast(&ast, &source).unwrap();
            assert_eq!(program.functions().functions().len(), 1);
            measurements.push(crate::test_alloc::snapshot("dead_scale_counter.nx"));
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
