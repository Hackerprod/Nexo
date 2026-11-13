//! F0-0007: Result constructors retain monomorphic component constraints.
use nexo::{
    diagnostic::{DataValue, Diagnostic, Operation, Stage},
    interp::{self, Outcome},
    ir::{self, CompileError},
};

const FILE: &str = "f0007.n";

#[allow(clippy::vec_box)] // Matches the diagnostic API returned by compilation.
fn rejected(source: &str, executable: bool) -> Vec<Box<Diagnostic>> {
    let result = if executable {
        ir::compile_scalar(FILE, source)
    } else {
        ir::check_scalar(FILE, source)
    };
    match result {
        Err(CompileError::Rejected(errors)) => errors,
        other => panic!("expected rejection for {source}: {other:?}"),
    }
}

fn accepted(source: &str) {
    ir::check_scalar(FILE, source).expect(source);
    let program = ir::compile_scalar(FILE, source).expect(source);
    let execution = interp::run_scalar(&program, Default::default());
    assert_eq!(execution.outcome, Outcome::Unit, "{source}");
    assert!(execution.observations.events().is_empty(), "{source}");
}

fn marked_source(marked: &str) -> (String, (u64, u64)) {
    let start = marked.find('⟦').expect("opening marker");
    let end = marked.find('⟧').expect("closing marker");
    let source = marked.replace(['⟦', '⟧'], "");
    (source, (start as u64, (end - '⟦'.len_utf8()) as u64))
}

#[test]
fn result_constructor_posterior_context_accepts_adr_valid_programs() {
    // Literal V1–V12 from F0-0007 §4; each run returns unit with no events.
    for source in [
        "fn Main(){let r=Ok(1);let s:Result<I64,Bool>=r;}",
        "fn Main(){let r=Err(7);let s:Result<Bool,I64>=r;}",
        "fn F(r:view Result<I64,Bool>){} fn Main(){let r=Ok(1);F(r);}",
        "fn F()->Result<I64,I64>{let x=Ok(1);return x;} fn Main(){let y=F();}",
        "fn Main(){var r=Ok(1);r=Err(true);let s:Result<I64,Bool>=r;}",
        "fn Main(){let r:Result<I64,I64>=Ok(1);AssertEq(Ok(1),r);}",
        "fn Main(){let r:Result<I64,I64>=Ok(1);AssertEq(r,Ok(1));}",
        "fn Main(){let r=TryAdd(1,2);AssertEq(Ok(3),r);AssertEq(r,Ok(3));}",
        "fn Main(){let r=Ok(1);let s=r;let t:Result<I64,Bool>=s;}",
        "fn Main(){let r=Ok(Ok(1));let s:Result<Result<I64,Bool>,I64>=r;}",
        "fn Main(){let a=[];let b:Array<I64>=a;AssertEq(a,[]);}",
        "fn Main(){let r=Ok(1);let a:Array<Result<I64,Bool>>=[r];}",
    ] {
        accepted(source);
    }
}

#[test]
fn result_constructor_posterior_context_reports_adr_negative_diagnostics() {
    // Literal N1–N10 from F0-0007 §4. U/A carry only unresolved_components/{};
    // M carries the full Result types from the table, not component types.
    type Case<'a> = (
        &'a str,
        &'a str,
        Option<&'a str>,
        Option<(&'a str, &'a str)>,
        &'a [&'a str],
    );
    let cases: [Case<'_>; 10] = [
        (
            "fn Main(){let r=⟦Ok(1)⟧;}",
            "NX-E0113",
            None,
            None,
            &["$.err"],
        ),
        (
            "fn Main(){let r=⟦Err(7)⟧;}",
            "NX-E0113",
            None,
            None,
            &["$.ok"],
        ),
        (
            "fn Main(){let r=Err(1);let s:Result<Bool,Bool>=⟦r⟧;let t:Result<I64,I64>=r;}",
            "NX-E0102",
            Some("type_mismatch"),
            Some(("Result<Bool,Bool>", "Result<I64,I64>")),
            &[],
        ),
        (
            "fn Main(){let r=Ok(1);let s:Result<I64,Bool>=r;let t:Result<I64,I64>=⟦r⟧;}",
            "NX-E0102",
            Some("type_mismatch"),
            Some(("Result<I64,I64>", "Result<I64,Bool>")),
            &[],
        ),
        (
            "fn F(r:view Result<Bool,I64>){} fn Main(){let r=Ok(1);let s:Result<I64,I64>=r;F(⟦r⟧);}",
            "NX-E0102",
            Some("type_mismatch"),
            Some(("Result<Bool,I64>", "Result<I64,I64>")),
            &[],
        ),
        (
            "fn F()->Result<Bool,I64>{let x=Ok(1);let s:Result<I64,I64>=x;return ⟦x⟧;} fn Main(){}",
            "NX-E0102",
            Some("type_mismatch"),
            Some(("Result<Bool,I64>", "Result<I64,I64>")),
            &[],
        ),
        (
            "fn Main(){let r:Result<I64,I64>=Ok(1);let s:Result<I64,Bool>=Ok(1);AssertEq(r,⟦s⟧);}",
            "NX-E0102",
            Some("type_mismatch"),
            Some(("Result<I64,I64>", "Result<I64,Bool>")),
            &[],
        ),
        (
            "fn Main(){let r=⟦Ok(1)⟧;Print(r);}",
            "NX-E0113",
            None,
            None,
            &["$.err"],
        ),
        ("fn Main(){let a=⟦[]⟧;}", "NX-E0103", None, None, &[]),
        ("fn Main(){let r=Ok(⟦[]⟧);}", "NX-E0103", None, None, &[]),
    ];
    for (marked, code, subcause, types, unresolved) in cases {
        let (source, span) = marked_source(marked);
        for executable in [false, true] {
            let errors = rejected(&source, executable);
            assert_eq!(errors.len(), 1, "{source}: {errors:?}");
            let d = &errors[0];
            assert_eq!(
                (d.code.as_str(), d.subcause.as_deref()),
                (code, subcause),
                "{source}"
            );
            assert_eq!(d.stage, Stage::Types, "{source}");
            assert_eq!(
                d.operation,
                if types.is_some() {
                    Operation::TypesCheck
                } else {
                    Operation::TypesInfer
                },
                "{source}"
            );
            let primary = d.primary_span.as_ref().unwrap();
            assert_eq!((primary.start, primary.end), span, "{source}");
            let expected_data = if let Some((expected, found)) = types {
                [
                    (
                        "expected_type".to_owned(),
                        DataValue::String(expected.into()),
                    ),
                    ("found_type".to_owned(), DataValue::String(found.into())),
                ]
                .into()
            } else if code == "NX-E0113" {
                [(
                    "unresolved_components".to_owned(),
                    DataValue::Array(
                        unresolved
                            .iter()
                            .map(|s| DataValue::String((*s).into()))
                            .collect(),
                    ),
                )]
                .into()
            } else {
                Default::default()
            };
            assert_eq!(d.data, expected_data, "{source}");
        }
    }
}

#[test]
fn result_constructor_conflicts_preserve_later_compatible_constraints() {
    // The rejected equality in N3 must not fix T=Bool or poison the later t.
    accepted("fn Main(){let r=Err(1);let t:Result<I64,I64>=r;}");
    accepted("fn Main(){let r=Ok(1);let s:Result<I64,Bool>=r;let t:Result<I64,Bool>=r;}");
    accepted("fn Main(){let r=Ok(1);let s=r;let t:Result<I64,Bool>=s;}");
}

#[test]
fn result_constructor_negative_repairs_and_control_contexts() {
    // Positives paired with N1–N10 in F0-0007 §4.
    for source in [
        "fn Main(){let r:Result<I64,I64>=Ok(1);}",
        "fn Main(){let r:Result<Bool,I64>=Err(7);}",
        "fn Main(){let r=Err(1);let t:Result<I64,I64>=r;}",
        "fn Main(){let r=Ok(1);let s:Result<I64,Bool>=r;let t:Result<I64,Bool>=r;}",
        "fn F(r:view Result<I64,I64>){} fn Main(){let r=Ok(1);let s:Result<I64,I64>=r;F(r);}",
        "fn F()->Result<I64,I64>{let x=Ok(1);let s:Result<I64,I64>=x;return x;} fn Main(){}",
        "fn Main(){let r:Result<I64,I64>=Ok(1);let s:Result<I64,I64>=Ok(1);AssertEq(r,s);}",
        "fn Main(){let r:Result<I64,Bool>=Ok(1);Print(r);}",
        "fn Main(){let a:Array<I64>=[];}",
        "fn Main(){let r:Result<Array<I64>,Bool>=Ok([]);}",
        "fn Main(){if true {let r=Ok(1);let s:Result<I64,Bool>=r;}while false {let r=Err(true);let s:Result<I64,Bool>=r;}}",
    ] {
        accepted(source);
    }
}

#[test]
fn result_constructor_nested_unresolved_paths_follow_preorder() {
    let source = "fn Main(){let r=Ok(Ok(1));}";
    let errors = rejected(source, false);
    assert_eq!(errors.len(), 2);
    assert_eq!(errors[0].code, "NX-E0113");
    assert_eq!(
        errors[0].primary_span.as_ref().map(|s| (s.start, s.end)),
        Some((16, 25))
    );
    assert_eq!(
        errors[0].data.get("unresolved_components"),
        Some(&DataValue::Array(vec![
            DataValue::String("$.ok.err".into()),
            DataValue::String("$.err".into()),
        ]))
    );
    assert_eq!(errors[1].code, "NX-E0113");
    assert_eq!(
        errors[1].primary_span.as_ref().map(|s| (s.start, s.end)),
        Some((19, 24))
    );
    assert_eq!(
        errors[1].data.get("unresolved_components"),
        Some(&DataValue::Array(vec![DataValue::String("$.err".into()),]))
    );
}

#[test]
fn result_constructor_posterior_context_keeps_resource_event_order() {
    let source =
        "fn Main(){let r=Ok(MockOpen(1));let s:Result<MockResource,I64>=move r;Drop(move s);}";
    ir::check_scalar(FILE, source).expect(source);
    let program = ir::compile_scalar(FILE, source).expect(source);
    let execution = interp::run_scalar(&program, Default::default());
    assert_eq!(execution.outcome, Outcome::Unit);
    assert_eq!(execution.observations.events(), ["open:0:1", "close:0:1"]);
}

#[test]
fn result_array_shape_conflict_waits_for_both_complete_types() {
    let (source, span) = marked_source(
        "fn Main(){let r=Ok(1);let a=[];AssertEq(r,⟦a⟧);let x:Result<I64,Bool>=r;let y:Array<I64>=a;}",
    );
    let errors = rejected(&source, false);
    assert_eq!(errors.len(), 1, "{errors:?}");
    let error = &errors[0];
    assert_eq!(
        (error.code.as_str(), error.subcause.as_deref()),
        ("NX-E0102", Some("type_mismatch"))
    );
    assert_eq!(
        error.primary_span.as_ref().map(|s| (s.start, s.end)),
        Some(span)
    );
    assert_eq!(
        error.data.get("expected_type"),
        Some(&DataValue::String("Result<I64,Bool>".into()))
    );
    assert_eq!(
        error.data.get("found_type"),
        Some(&DataValue::String("Array<I64>".into()))
    );
}

#[test]
fn result_constructor_payload_conflict_rejects_whole_equality_atomically() {
    let (source, span) = marked_source(
        "fn Main(){let a=Ok(1);let b=Ok(true);AssertEq(a,⟦b⟧);let x:Result<I64,Bool>=a;let y:Result<Bool,Bool>=b;}",
    );
    let errors = rejected(&source, false);
    assert_eq!(errors.len(), 1, "{errors:?}");
    let error = &errors[0];
    assert_eq!(
        (error.code.as_str(), error.subcause.as_deref()),
        ("NX-E0102", Some("type_mismatch"))
    );
    assert_eq!(
        error.primary_span.as_ref().map(|s| (s.start, s.end)),
        Some(span)
    );
    assert_eq!(
        error.data.get("expected_type"),
        Some(&DataValue::String("Result<I64,Bool>".into()))
    );
    assert_eq!(
        error.data.get("found_type"),
        Some(&DataValue::String("Result<Bool,Bool>".into()))
    );
    let unresolved = rejected(
        "fn Main(){let a=Ok(1);let b=Ok(true);AssertEq(a,b);}",
        false,
    );
    assert_eq!(unresolved.len(), 2);
    assert!(unresolved.iter().all(|error| error.code == "NX-E0113"));
}

#[test]
fn nested_result_conflict_is_atomic_and_later_context_resolves_it() {
    let source = "fn Main(){let r=Ok(Ok(1));let t:Result<Bool,Bool>=r;let u:Result<Result<I64,I64>,Bool>=r;}";
    for executable in [false, true] {
        let errors = rejected(source, executable);
        assert_eq!(errors.len(), 1, "{errors:?}");
        let d = &errors[0];
        assert_eq!(
            (d.code.as_str(), d.subcause.as_deref()),
            ("NX-E0102", Some("type_mismatch"))
        );
        assert_eq!(
            (d.stage, d.operation),
            (Stage::Types, Operation::TypesCheck)
        );
        assert_eq!(
            d.primary_span.as_ref().map(|s| (s.start, s.end)),
            Some((50, 51))
        );
        assert_eq!(
            d.data.get("expected_type"),
            Some(&DataValue::String("Result<Bool,Bool>".into()))
        );
        assert_eq!(
            d.data.get("found_type"),
            Some(&DataValue::String("Result<Result<I64,I64>,Bool>".into()))
        );
    }
}

#[test]
fn nested_result_conflict_keeps_both_constructor_obligations() {
    let source = "fn Main(){let r=Ok(Ok(1));let t:Result<Bool,Bool>=r;}";
    for executable in [false, true] {
        let errors = rejected(source, executable);
        assert_eq!(errors.len(), 2, "{errors:?}");
        for (d, span, paths) in [
            (&errors[0], (16, 25), vec!["$.ok.err", "$.err"]),
            (&errors[1], (19, 24), vec!["$.err"]),
        ] {
            assert_eq!((d.code.as_str(), d.subcause.as_deref()), ("NX-E0113", None));
            assert_eq!(
                (d.stage, d.operation),
                (Stage::Types, Operation::TypesInfer)
            );
            assert_eq!(
                d.primary_span.as_ref().map(|s| (s.start, s.end)),
                Some(span)
            );
            assert_eq!(
                d.data.get("unresolved_components"),
                Some(&DataValue::Array(
                    paths
                        .into_iter()
                        .map(|p| DataValue::String(p.into()))
                        .collect()
                ))
            );
        }
    }
}

#[test]
fn nested_result_alias_shape_conflict_never_reaches_invalid_ir() {
    // A2 §4.1.2.2 omits an unserializable mismatch and leaves each
    // constructor's unresolved components to its own §13.17 diagnosis.
    let source = "fn Main(){let a=Ok(1);let r=Ok(a);let s:Result<I64,Bool>=r;}";
    for executable in [false, true] {
        let errors = rejected(source, executable);
        assert_eq!(errors.len(), 2, "{errors:?}");
        for (d, span, paths) in [
            (&errors[0], (16, 21), vec!["$.err"]),
            (&errors[1], (28, 33), vec!["$.ok.err", "$.err"]),
        ] {
            assert_eq!((d.code.as_str(), d.subcause.as_deref()), ("NX-E0113", None));
            assert_eq!(
                (d.stage, d.operation),
                (Stage::Types, Operation::TypesInfer)
            );
            assert_eq!(
                d.primary_span.as_ref().map(|s| (s.start, s.end)),
                Some(span)
            );
            assert_eq!(
                d.data.get("unresolved_components"),
                Some(&DataValue::Array(
                    paths
                        .into_iter()
                        .map(|p| DataValue::String(p.into()))
                        .collect()
                ))
            );
        }
    }
    for source in [
        "fn Main(){let a=Err(true);let r=Err(a);let s:Result<I64,I64>=r;}",
        "fn Main(){let a=Ok(1);let b=a;let r=Ok(Ok(b));let s:Result<I64,Bool>=r;}",
        "fn F(p:view Result<Bool,Bool>){} fn Main(){let a=Ok(1);let r=Ok(a);F(r);}",
        "fn Main(){let a=Ok(1);let r=Err(a);let q:Array<I64>=[r];}",
    ] {
        for executable in [false, true] {
            let errors = rejected(source, executable);
            assert!(
                errors.iter().any(|d| d.code == "NX-E0113"),
                "{source}: {errors:?}"
            );
        }
    }
}

#[test]
fn try_constrains_result_error_and_never_does_not_constrain_payload() {
    let try_source = "fn Main()->Result<(),I64>{let x=Ok(1)?;return Ok(());}";
    ir::check_scalar(FILE, try_source).expect(try_source);
    let program = ir::compile_scalar(FILE, try_source).expect(try_source);
    assert_eq!(
        interp::run_scalar(&program, Default::default()).outcome,
        Outcome::Ok
    );
    accepted(
        "fn Main(){let r=Ok((if true then {return;} else {return;}));let s:Result<I64,Bool>=r;}",
    );
    accepted(
        "fn Main(){let r=Err((if true then {return;} else {return;}));let s:Result<I64,Bool>=r;}",
    );
}

#[test]
fn try_projects_pending_result_bindings_through_the_shared_type_graph() {
    accepted("fn F()->Result<(),I64>{let r=Ok(1);let x=r?;return Ok(());} fn Main(){}");
    for source in [
        "fn Main()->Result<(),I64>{let r=Ok(1);let x=r?;return Ok(());}",
        "fn Main()->Result<(),I64>{let r=Ok(1);let x=r?;let s:Result<I64,I64>=r;return Ok(());}",
        "fn Main()->Result<(),I64>{let r=Ok(1);let a=r;let x=a?;return Ok(());}",
        "fn Main()->Result<(),I64>{let r=Ok(1);let a=r;let x=a?;let s:Result<I64,I64>=r;return Ok(());}",
        "fn Main()->Result<(),I64>{let r=Ok(1);let s:Result<I64,I64>=r;let x=r?;return Ok(());}",
        "fn Main()->Result<(),I64>{let r=Err(1);let x:I64=r?;return Ok(());}",
        "fn Main()->Result<(),I64>{let r=Ok(1);if true {let x=r?;}while false {let y=r?;}return Ok(());}",
        "fn Main()->Result<(),I64>{let r=TryAdd(1,2);let x=r?;return Ok(());}",
    ] {
        ir::check_scalar(FILE, source).expect(source);
        let program = ir::compile_scalar(FILE, source).expect(source);
        let execution = interp::run_scalar(&program, Default::default());
        let expected = if source.contains("let r=Err(1)") {
            Outcome::Err(1)
        } else {
            Outcome::Ok
        };
        assert_eq!(execution.outcome, expected, "{source}");
        assert!(execution.observations.events().is_empty(), "{source}");
    }
}

#[test]
fn nested_err_with_unresolved_array_has_one_causal_inference_diagnostic() {
    let source = "fn Main(){let r=Err(Ok([]));}";
    for executable in [false, true] {
        let errors = rejected(source, executable);
        assert_eq!(errors.len(), 1, "{errors:?}");
        let d = &errors[0];
        assert_eq!((d.code.as_str(), d.subcause.as_deref()), ("NX-E0103", None));
        assert_eq!(
            (d.stage, d.operation),
            (Stage::Types, Operation::TypesInfer)
        );
        assert_eq!(
            d.primary_span.as_ref().map(|s| (s.start, s.end)),
            Some((23, 25))
        );
        assert!(d.data.is_empty());
    }
}

#[test]
fn result_var_reassignment_preserves_each_constructor_obligation() {
    // A2 §§4.1.2.2, 13.17 and F0-0007 R3/R5: failed composed equality
    // contributes no partial component binding. The four written constructors
    // retain their independent unresolved paths in source preorder.
    for (source, last_end) in [
        (
            "fn Main(){let a=Ok(1);var r=Ok(a);r=Ok(1);r=Err(true);}",
            53,
        ),
        ("fn Main(){let a=Ok(1);var r=Ok(a);r=Ok(1);r=Err(1);}", 50),
    ] {
        let expected = [
            ((16, 21), vec!["$.err"]),
            ((28, 33), vec!["$.ok.err"]),
            ((36, 41), vec!["$.err"]),
            ((44, last_end), vec!["$.ok.err"]),
        ];
        for executable in [false, true] {
            let errors = rejected(source, executable);
            assert_eq!(errors.len(), expected.len(), "{source}: {errors:?}");
            for (error, (span, paths)) in errors.iter().zip(expected.iter()) {
                assert_eq!(
                    (error.code.as_str(), error.subcause.as_deref()),
                    ("NX-E0113", None)
                );
                assert_eq!(
                    (error.stage, error.operation),
                    (Stage::Types, Operation::TypesInfer)
                );
                assert_eq!(
                    error.primary_span.as_ref().map(|s| (s.start, s.end)),
                    Some(*span)
                );
                assert_eq!(
                    error.data.get("unresolved_components"),
                    Some(&DataValue::Array(
                        paths
                            .iter()
                            .map(|path| DataValue::String((*path).into()))
                            .collect()
                    ))
                );
            }
        }
    }
}

#[test]
fn structural_var_reassignments_keep_compatible_bindings() {
    for source in [
        "fn Main(){var r=Ok(1);r=Err(true);let s:Result<I64,Bool>=r;}",
        "fn Main(){var r:Result<I64,Bool>=Ok(1);r=Err(true);r=Ok(2);}",
        "fn Main(){var a:Array<Result<I64,Bool>>=[Ok(1)];a=[Err(true)];}",
        "fn Main(){var a:Array<Array<Result<I64,Bool>>>=[[Ok(1)]];a=[[Err(true)]];}",
        "fn Main(){var r=Ok(1);let alias=r;r=Err(true);let s:Result<I64,Bool>=alias;}",
        "fn Main(){var r:Result<I64,Bool>=Ok(1);if true {r=Err(true);}while false {r=Ok(2);}}",
        "fn Main()->Result<(),I64>{var r=Ok(1);let x=r?;r=Err(2);return Ok(());}",
    ] {
        if source.contains("fn Main()->Result") {
            ir::check_scalar(FILE, source).expect(source);
            ir::compile_scalar(FILE, source).expect(source);
        } else {
            accepted(source);
        }
    }
}

#[test]
fn incompatible_structural_var_reassignments_report_literal_type_mismatches() {
    // A2 §4.1.2.2 uses NX-E0102 once both sides have serializable evidence.
    for (source, span, expected, found) in [
        (
            "fn Main(){var r:Result<I64,Bool>=Ok(1);r=Err(1);}",
            (45, 46),
            "Bool",
            "I64",
        ),
        (
            "fn Main(){var a:Array<Result<I64,Bool>>=[Ok(1)];a=[Err(1)];}",
            (55, 56),
            "Bool",
            "I64",
        ),
        (
            "fn Main(){var a:Array<Array<Result<I64,Bool>>>=[[Ok(1)]];a=[[Err(1)]];}",
            (65, 66),
            "Bool",
            "I64",
        ),
        (
            "fn Main(){var r:Result<I64,Bool>=Ok(1);if true {r=Err(1);}}",
            (54, 55),
            "Bool",
            "I64",
        ),
        (
            "fn Main(){var r:Result<I64,Bool>=Ok(1);while false {r=Err(1);}}",
            (58, 59),
            "Bool",
            "I64",
        ),
        (
            "fn Main()->Result<(),I64>{var r=Ok(1);let x=r?;r=Err(true);return Ok(());}",
            (53, 57),
            "I64",
            "Bool",
        ),
    ] {
        for executable in [false, true] {
            let errors = rejected(source, executable);
            assert_eq!(errors.len(), 1, "{source}: {errors:?}");
            let d = &errors[0];
            assert_eq!(
                (d.code.as_str(), d.subcause.as_deref()),
                ("NX-E0102", Some("type_mismatch"))
            );
            assert_eq!(
                (d.stage, d.operation),
                (Stage::Types, Operation::TypesCheck)
            );
            assert_eq!(
                d.primary_span.as_ref().map(|s| (s.start, s.end)),
                Some(span)
            );
            assert_eq!(d.data.len(), 2);
            assert_eq!(
                d.data.get("expected_type"),
                Some(&DataValue::String(expected.into()))
            );
            assert_eq!(
                d.data.get("found_type"),
                Some(&DataValue::String(found.into()))
            );
        }
    }
}

#[test]
fn try_rejects_a_concrete_error_component_different_from_the_function() {
    for (source, span) in [
        (
            "fn Main()->Result<(),Bool>{let r:Result<I64,I64>=Ok(1);let x=r?;return Ok(());}",
            (62, 63),
        ),
        (
            "fn Main()->Result<(),Bool>{let r:Result<I64,I64>=Err(1);let x=r?;return Ok(());}",
            (63, 64),
        ),
    ] {
        for executable in [false, true] {
            let errors = rejected(source, executable);
            assert_eq!(errors.len(), 1, "{source}: {errors:?}");
            let d = &errors[0];
            assert_eq!(
                (d.code.as_str(), d.subcause.as_deref()),
                ("NX-E0102", Some("type_mismatch"))
            );
            assert_eq!(
                (d.stage, d.operation),
                (Stage::Types, Operation::TypesCheck)
            );
            assert_eq!(
                d.primary_span.as_ref().map(|s| (s.start, s.end)),
                Some(span)
            );
            assert_eq!(
                d.data.get("expected_type"),
                Some(&DataValue::String("Bool".into()))
            );
            assert_eq!(
                d.data.get("found_type"),
                Some(&DataValue::String("I64".into()))
            );
        }
    }
}

#[test]
fn array_of_pending_result_reports_only_result_constructor() {
    let source = "fn Main(){let a=[Ok(1)];}";
    for executable in [false, true] {
        let errors = rejected(source, executable);
        assert_eq!(errors.len(), 1, "{errors:?}");
        let d = &errors[0];
        assert_eq!((d.code.as_str(), d.subcause.as_deref()), ("NX-E0113", None));
        assert_eq!(
            (d.stage, d.operation),
            (Stage::Types, Operation::TypesInfer)
        );
        assert_eq!(
            d.primary_span.as_ref().map(|s| (s.start, s.end)),
            Some((17, 22))
        );
        assert_eq!(
            d.data.get("unresolved_components"),
            Some(&DataValue::Array(vec![DataValue::String("$.err".into())]))
        );
    }
    accepted("fn Main(){let a:Array<Result<I64,Bool>>=[Ok(1)];}");
}

#[test]
fn recursive_result_equality_preserves_finite_constructor_obligations() {
    // A2 §§4.1.2.2, 13.17: the unsatisfiable equality contributes no
    // constraint; each constructor retains its own finite component paths.
    let source = "fn Main(){var r=Ok(1);r=Ok(r);}";
    for executable in [false, true] {
        let errors = rejected(source, executable);
        assert_eq!(errors.len(), 2, "{errors:?}");
        for (error, span, paths) in [
            (&errors[0], (16, 21), vec!["$.err"]),
            (&errors[1], (24, 29), vec!["$.ok.err", "$.err"]),
        ] {
            assert_eq!(
                (error.code.as_str(), error.subcause.as_deref()),
                ("NX-E0113", None)
            );
            assert_eq!(
                (error.stage, error.operation),
                (Stage::Types, Operation::TypesInfer)
            );
            assert_eq!(
                error.primary_span.as_ref().map(|s| (s.start, s.end)),
                Some(span)
            );
            assert_eq!(
                error.data.get("unresolved_components"),
                Some(&DataValue::Array(
                    paths
                        .into_iter()
                        .map(|p| DataValue::String(p.into()))
                        .collect()
                ))
            );
        }
    }
}

#[test]
fn recursive_result_aliases_and_array_paths_remain_finite() {
    // A2 §§4.1.2.2, 13.17: every failed equality leaves only the
    // constructors' independent obligations, including mixed Array paths.
    type ExpectedPath<'a> = ((u64, u64), &'a [&'a str]);
    let cases: &[(&str, &[ExpectedPath<'_>])] = &[
        (
            "fn Main(){var r=Err(1);r=Err(r);}",
            &[((16, 22), &["$.ok"]), ((25, 31), &["$.ok", "$.err.ok"])],
        ),
        (
            "fn Main(){var a=Ok(1);let b=Ok(a);a=b;}",
            &[((16, 21), &["$.err"]), ((28, 33), &["$.ok.err", "$.err"])],
        ),
        (
            "fn Main(){var a=Ok(1);let b=Ok(a);let c=Ok(b);a=c;}",
            &[
                ((16, 21), &["$.err"]),
                ((28, 33), &["$.ok.err", "$.err"]),
                ((40, 45), &["$.ok.ok.err", "$.ok.err", "$.err"]),
            ],
        ),
        (
            "fn Main(){var r=Ok(1);r=Ok([r]);}",
            &[
                ((16, 21), &["$.err"]),
                ((24, 31), &["$.ok.element.err", "$.err"]),
            ],
        ),
    ];
    for (source, expected) in cases {
        for executable in [false, true] {
            let errors = rejected(source, executable);
            assert_eq!(errors.len(), expected.len(), "{source}: {errors:?}");
            for (error, (span, paths)) in errors.iter().zip(*expected) {
                assert_eq!(
                    (error.code.as_str(), error.subcause.as_deref()),
                    ("NX-E0113", None),
                    "{source}"
                );
                assert_eq!(
                    (error.stage, error.operation),
                    (Stage::Types, Operation::TypesInfer),
                    "{source}"
                );
                assert_eq!(
                    error.primary_span.as_ref().map(|s| (s.start, s.end)),
                    Some(*span),
                    "{source}"
                );
                assert_eq!(
                    error.data.get("unresolved_components"),
                    Some(&DataValue::Array(
                        paths
                            .iter()
                            .map(|p| DataValue::String((*p).into()))
                            .collect()
                    )),
                    "{source}"
                );
            }
        }
    }
    accepted("fn Main(){var r:Result<I64,Bool>=Ok(1);r=Ok(2);}");
}
