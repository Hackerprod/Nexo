use super::{CompileError, compile_scalar};

#[test]
fn while_condition_effects_reach_body_and_false_exit() {
    let invalid = [
        "fn f(b:bool){var r=mock_open(1);while drop(r)==(){inspect(r);r=mock_open(2);}}fn main(){}",
    ];
    for source in invalid {
        let result = compile_scalar("condition_effects.nx", source);
        assert!(
            matches!(&result, Err(CompileError::Rejected(ds)) if ds.iter().any(|d| d.code == "NX-E0202" && d.subcause.as_deref() == Some("definitely_consumed"))),
            "{source}: {result:?}"
        );
    }
    let valid = "fn f(b:bool){var r=mock_open(1);drop(r);while if b{r=mock_open(2);value:true;}else{r=mock_open(3);value:false;}{inspect(r);}}fn main(){}";
    let result = compile_scalar("condition_effects.nx", valid);
    assert!(result.is_ok(), "{result:?}");
}

#[test]
fn boolean_condition_keeps_its_true_and_false_ownership_exits_separate() {
    let source = "fn f(b:bool){var r=mock_open(1);while if b{drop(r);value:true;}else{value:false;}{r=mock_open(2);}inspect(r);}fn main(){}";
    let result = compile_scalar("boolean_condition_exits.nx", source);
    assert!(
        matches!(result, Err(CompileError::Rejected(ref ds)) if ds.iter().any(|d| d.code == "NX-E0202" && d.subcause.as_deref() == Some("maybe_consumed"))),
        "{result:?}"
    );
}

#[test]
fn condition_arm_consumption_has_exact_joined_exit_diagnostic() {
    use crate::diagnostic::{CauseKind, CauseOperation, Operation, RelatedRole, Stage};
    let source = "fn f(b:bool){var r=mock_open(1);while if b{drop(r);value:true;}else{value:false;}{r=mock_open(2);}inspect(r);}fn main(){}";
    let Err(CompileError::Rejected(ds)) = compile_scalar("joined_condition.nx", source) else {
        panic!("expected joined ownership error");
    };
    let at = source.find("inspect(r)").unwrap() as u64 + 8;
    let d = ds
        .iter()
        .find(|d| d.primary_span.as_ref().is_some_and(|s| s.start == at))
        .unwrap();
    assert_eq!(d.code, "NX-E0202");
    assert_eq!(d.subcause.as_deref(), Some("maybe_consumed"));
    assert_eq!(d.stage, Stage::Ownership);
    assert_eq!(d.operation, Operation::OwnershipUse);
    assert_eq!(d.primary_span.as_ref().unwrap().end, at + 1);
    assert!(d.data.is_empty());
    let drop_at = source.find("drop(r)").unwrap() as u64;
    assert_eq!(
        d.related_spans
            .iter()
            .map(|r| (r.role, r.span.start, r.span.end))
            .collect::<Vec<_>>(),
        [
            (
                RelatedRole::Declaration,
                source.find("var r").unwrap() as u64 + 4,
                source.find("var r").unwrap() as u64 + 5
            ),
            (RelatedRole::Consumption, drop_at, drop_at + 7)
        ]
    );
    assert_eq!(d.cause_chain.len(), 1);
    assert_eq!(d.cause_chain[0].kind, CauseKind::Drop);
    assert_eq!(d.cause_chain[0].operation, CauseOperation::ValueDrop);
    d.validate("joined_condition.nx", source).unwrap();
}

#[test]
fn condition_value_forms_share_joined_ownership_state() {
    for condition in [
        "if b{drop(r);value:true;}else{value:false;}",
        "if b{drop(r);value:b;}else{value:b;}",
        "(if b{drop(r);value:0;}else{value:1;})<1",
        "if b{value:if b{drop(r);value:true;}else{value:false;};}else{value:false;}",
    ] {
        let source = format!(
            "fn f(b:bool){{var r=mock_open(1);while {condition}{{r=mock_open(2);}}inspect(r);}}fn main(){{}}"
        );
        let result = compile_scalar("joined_value_forms.nx", &source);
        assert!(
            matches!(result, Err(CompileError::Rejected(ref ds)) if ds.iter().any(|d| d.code == "NX-E0202" && d.subcause.as_deref() == Some("maybe_consumed") && d.primary_span.as_ref().is_some_and(|s| s.start == source.find("inspect(r)").unwrap() as u64 + 8))),
            "{source}: {result:?}"
        );
    }
}

#[test]
fn condition_effects_match_single_iteration_unrolled_twins() {
    for (condition, body, use_after) in [
        (
            "if b{drop(r);value:true;}else{value:false;}",
            "r=mock_open(2);",
            "inspect(r);",
        ),
        (
            "if b{value:true;}else{drop(r);value:false;}",
            "",
            "drop(r);",
        ),
    ] {
        let loop_source = format!(
            "fn f(b:bool){{var r=mock_open(1);while {condition}{{{body}}}{use_after}}}fn main(){{}}"
        );
        let twin_source = format!(
            "fn f(b:bool){{var r=mock_open(1);if {condition}{{{body}}}{use_after}}}fn main(){{}}"
        );
        for source in [loop_source, twin_source] {
            let result = compile_scalar("condition_twin.nx", &source);
            assert!(
                matches!(result, Err(CompileError::Rejected(ref ds)) if ds.iter().any(|d| d.code == "NX-E0202" && d.subcause.as_deref() == Some("maybe_consumed"))),
                "{source}: {result:?}"
            );
        }
    }
}

#[test]
fn false_condition_arm_consumption_reaches_next_header() {
    use crate::diagnostic::{CauseOperation, Operation, RelatedRole, Stage};
    let source = "fn f(b:bool){var r=mock_open(1);while if b{value:true;}else{drop(r);value:false;}{}}fn main(){}";
    let Err(CompileError::Rejected(ds)) = compile_scalar("false_arm_header.nx", source) else {
        panic!("expected ownership error");
    };
    let at = source.find("drop(r)").unwrap() as u64 + 5;
    let d = ds
        .iter()
        .find(|d| d.primary_span.as_ref().is_some_and(|s| s.start == at))
        .unwrap();
    assert_eq!(d.code, "NX-E0202");
    assert_eq!(d.subcause.as_deref(), Some("maybe_consumed"));
    assert_eq!(d.stage, Stage::Ownership);
    assert_eq!(d.operation, Operation::OwnershipUse);
    assert_eq!(d.primary_span.as_ref().unwrap().end, at + 1);
    assert!(d.data.is_empty());
    let declaration = source.find("var r").unwrap() as u64 + 4;
    let consumption = source.find("drop(r)").unwrap() as u64;
    assert!(
        d.related_spans
            .iter()
            .any(|r| r.role == RelatedRole::Declaration
                && r.span.start == declaration
                && r.span.end == declaration + 1)
    );
    assert!(
        d.related_spans
            .iter()
            .any(|r| r.role == RelatedRole::Consumption
                && r.span.start == consumption
                && r.span.end == consumption + 7)
    );
    assert_eq!(
        d.cause_chain.last().unwrap().operation,
        CauseOperation::ValueDrop
    );
    assert!(
        d.cause_chain
            .iter()
            .any(|cause| cause.operation == CauseOperation::FlowBackedge)
    );
    d.validate("false_arm_header.nx", source).unwrap();
}

#[test]
fn condition_arm_effects_cover_values_branches_and_use_positions() {
    use crate::diagnostic::{CauseOperation, Evidence, Operation, RelatedRole, Stage};
    for form in 0..4 {
        for then_effect in [true, false] {
            for action in ["drop(r);", "r=mock_open(2);", "inspect(r);"] {
                for position in ["body", "exit", "header"] {
                    let initial_drop = if action.starts_with("r=") {
                        "drop(r);"
                    } else {
                        ""
                    };
                    let (then_arm, else_arm) = if then_effect {
                        (action, "")
                    } else {
                        ("", action)
                    };
                    let condition = match form {
                        0 => format!("if b{{{then_arm}value:true;}}else{{{else_arm}value:false;}}"),
                        1 => format!("if b{{{then_arm}value:b;}}else{{{else_arm}value:b;}}"),
                        2 => format!("(if b{{{then_arm}value:0;}}else{{{else_arm}value:1;}})<1"),
                        _ => format!(
                            "if b{{value:if b{{{then_arm}value:true;}}else{{{else_arm}value:false;}};}}else{{value:false;}}"
                        ),
                    };
                    let (body, after) = match position {
                        "body" => ("inspect(r);r=mock_open(3);", ""),
                        "exit" => ("r=mock_open(3);", "inspect(r);"),
                        _ => ("", ""),
                    };
                    let source = format!(
                        "fn f(b:bool){{var r=mock_open(1);{initial_drop}while {condition}{{{body}}}{after}}}fn main(){{}}"
                    );
                    let result = compile_scalar("condition_matrix.nx", &source);
                    let invalid =
                        action == "drop(r);" || (action.starts_with("r=") && position != "header");
                    let twin = source.replacen("while ", "if ", 1);
                    let twin_result = compile_scalar("condition_twin_matrix.nx", &twin);
                    let twin_invalid = invalid && position != "header";
                    assert_eq!(
                        matches!(twin_result, Err(CompileError::Rejected(ref ds)) if ds.iter().any(|d| d.code == "NX-E0202" && d.subcause.as_deref() == Some("maybe_consumed"))),
                        twin_invalid,
                        "{twin}: {twin_result:?}"
                    );
                    if !invalid {
                        assert!(
                            result.is_ok(),
                            "{form} {then_effect} {action} {position}: {source}: {result:?}"
                        );
                        continue;
                    }
                    let Err(CompileError::Rejected(ds)) = result else {
                        panic!("expected ownership error: {source}");
                    };
                    let target = if position == "header" {
                        source.rfind("drop(r)").unwrap() as u64 + 5
                    } else {
                        source.rfind("inspect(r)").unwrap() as u64 + 8
                    };
                    let d = ds
                        .iter()
                        .find(|d| d.primary_span.as_ref().is_some_and(|s| s.start == target))
                        .unwrap_or_else(|| panic!("missing target {target}: {source}: {ds:?}"));
                    assert_eq!(d.code, "NX-E0202", "{source}");
                    assert_eq!(d.subcause.as_deref(), Some("maybe_consumed"), "{source}");
                    assert_eq!(d.stage, Stage::Ownership, "{source}");
                    assert_eq!(d.operation, Operation::OwnershipUse, "{source}");
                    assert_eq!(d.primary_span.as_ref().unwrap().end, target + 1, "{source}");
                    assert!(d.data.is_empty(), "{source}: {d:?}");
                    let declaration = source.find("var r").unwrap() as u64 + 4;
                    let consumed = source.find("drop(r)").unwrap() as u64;
                    assert!(
                        d.related_spans
                            .iter()
                            .any(|r| r.role == RelatedRole::Declaration
                                && r.span.start == declaration
                                && r.span.end == declaration + 1),
                        "{source}: {d:?}"
                    );
                    assert!(
                        d.related_spans
                            .iter()
                            .any(|r| r.role == RelatedRole::Consumption
                                && r.span.start == consumed
                                && r.span.end == consumed + 7),
                        "{source}: {d:?}"
                    );
                    let cause = d.cause_chain.last().unwrap();
                    assert_eq!(cause.operation, CauseOperation::ValueDrop, "{source}");
                    assert_eq!(cause.evidence, Evidence::Verified, "{source}");
                    assert_eq!(
                        cause.entity.as_ref().unwrap().id,
                        format!("binding:{declaration}"),
                        "{source}"
                    );
                    d.validate("condition_matrix.nx", &source).unwrap();
                }
            }
        }
    }
}

#[test]
fn condition_arm_outcomes_keep_consumption_and_reinitialization_on_their_edges() {
    let valid = [
        "fn f(b:bool){var r=mock_open(1);while if b{value:true;}else{value:false;}{inspect(r);}}fn main(){}",
        "fn f(b:bool){var r=mock_open(1);while if b{value:false;}else{value:true;}{inspect(r);}}fn main(){}",
        "fn f(b:bool){while if b{value:true;}else{value:true;}{}}fn main(){}",
        "fn f(b:bool){while if b{value:false;}else{value:false;}{}}fn main(){}",
    ];
    for source in valid {
        let result = compile_scalar("condition_arm_matrix.nx", source);
        assert!(result.is_ok(), "{source}: {result:?}");
    }
    let invalid = [
        (
            "fn f(b:bool){var r=mock_open(1);while if b{drop(r);value:true;}else{value:false;}{inspect(r);r=mock_open(2);}}fn main(){}",
            "maybe_consumed",
        ),
        (
            "fn f(b:bool){var r=mock_open(1);while if b{drop(r);value:false;}else{value:true;}{inspect(r);}inspect(r);}fn main(){}",
            "maybe_consumed",
        ),
        (
            "fn f(b:bool){var r=mock_open(1);drop(r);while if b{r=mock_open(2);value:true;}else{value:false;}{inspect(r);}inspect(r);}fn main(){}",
            "maybe_consumed",
        ),
        (
            "fn f(b:bool){var r=mock_open(1);while if b{drop(r);value:true;}else{value:false;}{r=mock_open(2);}inspect(r);}fn main(){}",
            "maybe_consumed",
        ),
        (
            "fn f(b:bool){var r=mock_open(1);while if b{r=mock_open(2);value:true;}else{drop(r);value:false;}{inspect(r);}}fn main(){}",
            "maybe_consumed",
        ),
    ];
    for (source, subcause) in invalid {
        let result = compile_scalar("condition_arm_matrix.nx", source);
        assert!(
            matches!(&result, Err(CompileError::Rejected(ds)) if ds.iter().any(|d| d.code == "NX-E0202" && d.subcause.as_deref() == Some(subcause))),
            "{source}: {result:?}"
        );
    }
}

#[test]
fn forward_and_backedge_errors_accumulate_at_independent_uses() {
    let source = "fn f(b:bool){let r=mock_open(1);let s=mock_open(2);drop(s);inspect(s);while b{drop(r);}}fn main(){}";
    let Err(CompileError::Rejected(ds)) = compile_scalar("independent_flow.nx", source) else {
        panic!("expected two ownership errors");
    };
    assert_eq!(
        ds.iter().filter(|d| d.code == "NX-E0202").count(),
        2,
        "{ds:?}"
    );
    assert!(
        ds[0].primary_span.as_ref().unwrap().start < ds[1].primary_span.as_ref().unwrap().start
    );

    let source = "fn take(consume x:MockResource){}fn f(b:bool){let r=mock_open(1);while b{take(consume r);inspect(r);}}fn main(){}";
    let Err(CompileError::Rejected(ds)) = compile_scalar("independent_flow.nx", source) else {
        panic!("expected two ownership errors");
    };
    assert_eq!(
        ds.iter().filter(|d| d.code == "NX-E0202").count(),
        2,
        "{ds:?}"
    );
}

#[test]
fn loop_join_distinguishes_maybe_from_definite_consumption() {
    for source in [
        "fn f(b:bool){var r=mock_open(1);drop(r);while b{inspect(r);r=mock_open(2);}}fn main(){}",
        "fn f(b:bool){var r=mock_open(1);drop(r);while b{r=mock_open(2);}inspect(r);}fn main(){}",
    ] {
        let Err(CompileError::Rejected(ds)) = compile_scalar("join_consumption.nx", source) else {
            panic!("expected ownership error: {source}");
        };
        assert!(
            ds.iter()
                .any(|d| d.code == "NX-E0202" && d.subcause.as_deref() == Some("maybe_consumed")),
            "{source}: {ds:?}"
        );
    }
}

#[test]
fn loop_exit_lists_all_reaching_consumptions_and_backedge() {
    use crate::diagnostic::RelatedRole;
    let source = "fn f(b:bool){var r=mock_open(1);drop(r);while b{r=mock_open(2);drop(r);}inspect(r);}fn main(){}";
    let Err(CompileError::Rejected(ds)) = compile_scalar("all_loop_sites.nx", source) else {
        panic!("expected ownership error");
    };
    let d = ds
        .iter()
        .find(|d| {
            d.primary_span
                .as_ref()
                .is_some_and(|s| s.start == source.find("inspect(r)").unwrap() as u64 + 8)
        })
        .unwrap();
    let consumptions: Vec<_> = d
        .related_spans
        .iter()
        .filter(|s| s.role == RelatedRole::Consumption)
        .collect();
    assert_eq!(consumptions.len(), 2, "{d:?}");
    assert!(
        d.related_spans
            .iter()
            .any(|s| s.role == RelatedRole::Backedge),
        "{d:?}"
    );
}

#[test]
fn exhausted_flow_budget_keeps_earlier_independent_diagnostics() {
    let mut source = String::from(
        "fn a(b:bool){let q=mock_open(1);while b{drop(q);}}fn aa(){let z=mock_open(1);drop(z);inspect(z);}fn f(b:bool){",
    );
    for _ in 0..40 {
        source.push_str("while b{} ");
    }
    source.push_str("}fn main(){}");
    let Err(CompileError::Rejected(ds)) = compile_scalar("quota_preserve.nx", &source) else {
        panic!("expected earlier errors and flow limit");
    };
    assert_eq!(
        ds.iter().filter(|d| d.code == "NX-E0202").count(),
        2,
        "{ds:?}"
    );
    assert_eq!(
        ds.iter()
            .filter(|d| d.code == "NX-E0003"
                && d.data.get("budget_kind")
                    == Some(&crate::diagnostic::DataValue::String(
                        "ownership_flow_steps".into()
                    )))
            .count(),
        1,
        "{ds:?}"
    );
    assert!(
        ds.windows(2)
            .all(|pair| pair[0].primary_span.as_ref().unwrap().start
                < pair[1].primary_span.as_ref().unwrap().start)
    );
}

#[test]
fn adr_gap_ownership_flow_quota_scope() {
    let mut source = String::from(
        "fn f(b:bool){var s=mock_open(1);drop(s);inspect(s);var t=mock_open(1);while b{drop(t);}",
    );
    for i in 0..100 {
        source.push_str(&format!(
            "var r{i}=mock_open(1);drop(r{i});while b{{r{i}=mock_open(2);}}"
        ));
    }
    source.push_str("}fn g(){let z=mock_open(1);drop(z);inspect(z);}fn main(){}");
    let Err(CompileError::Rejected(ds)) = compile_scalar("quota_preserve.nx", &source) else {
        panic!("expected ownership errors and quota");
    };
    let observed: Vec<_> = ds.iter().map(|d| d.code.as_str()).collect();
    assert_eq!(
        observed,
        ["NX-E0202", "NX-E0202", "NX-E0003", "NX-E0202"],
        "{ds:?}"
    );
    assert_eq!(
        ds[0].primary_span.as_ref().unwrap().start,
        source.find("inspect(s)").unwrap() as u64 + 8
    );
    assert_eq!(
        ds[3].primary_span.as_ref().unwrap().start,
        source.find("inspect(z)").unwrap() as u64 + 8
    );
}

#[test]
#[ignore = "explicit declarations-first 820 KiB diagnostic-pass quota regression"]
fn ownership_quota_large_source_keeps_converged_loop_diagnostic() {
    let prefix = "fn big(b:bool){var s=mock_open(1);drop(s);while b{inspect(s);s=mock_open(2);}";
    let mut source = prefix.to_owned();
    for index in 0..13_000 {
        source.push_str(&format!("var r{index}=mock_open(1);drop(r{index});"));
    }
    for index in 0..13_000 {
        source.push_str(&format!("while b{{r{index}=mock_open(2);}}"));
    }
    source.push_str("}fn main(){}");
    let short = format!("{prefix}}}fn main(){{}}");
    let expected = match compile_scalar("quota_large.nx", &short) {
        Err(CompileError::Rejected(errors)) => errors.into_iter().map(|d| *d).collect::<Vec<_>>(),
        other => panic!("expected an ownership diagnostic: {other:?}"),
    };
    assert_eq!(expected.len(), 1);
    assert_eq!(expected[0].subcause.as_deref(), Some("maybe_consumed"));
    let actual = match compile_scalar("quota_large.nx", &source) {
        Err(CompileError::Rejected(errors)) => errors.into_iter().map(|d| *d).collect::<Vec<_>>(),
        other => panic!("expected a quota diagnostic: {other:?}"),
    };
    assert_eq!(actual.len(), 2, "{actual:?}");
    assert_eq!(actual[0], expected[0]);
    assert_eq!(actual[1].code, "NX-E0003");
    assert_eq!(
        (
            actual[1].primary_span.as_ref().unwrap().start,
            actual[1].primary_span.as_ref().unwrap().end
        ),
        (819469, 819498)
    );
}

#[test]
fn shared_node_join_keeps_conditional_subcause_at_source_use() {
    use crate::diagnostic::{CauseKind, CauseOperation, Operation, RelatedRole, Stage};
    let source = "fn f(b:bool){var r0=mock_open(0);drop(r0);while b{while if b{value:true;}else{if if b{value:true;}else{r0=mock_open(7);value:false;}{}value:false;}{}} inspect(r0);} fn main(){}";
    let Err(CompileError::Rejected(errors)) = compile_scalar("shared_conditional.nx", source)
    else {
        panic!("expected ownership diagnostic");
    };
    assert_eq!(errors.len(), 1, "{errors:?}");
    let d = &errors[0];
    assert_eq!(d.code, "NX-E0202");
    assert_eq!(d.subcause.as_deref(), Some("maybe_consumed"));
    assert_eq!(d.stage, Stage::Ownership);
    assert_eq!(d.operation, Operation::OwnershipUse);
    assert_eq!(
        (
            d.primary_span.as_ref().unwrap().start,
            d.primary_span.as_ref().unwrap().end
        ),
        (159, 161)
    );
    assert_eq!(
        d.related_spans
            .iter()
            .map(|r| (r.role, r.span.start, r.span.end))
            .collect::<Vec<_>>(),
        [
            (RelatedRole::Declaration, 17, 19),
            (RelatedRole::Consumption, 33, 41)
        ]
    );
    assert_eq!(d.cause_chain.len(), 1);
    assert_eq!(d.cause_chain[0].kind, CauseKind::Drop);
    assert_eq!(d.cause_chain[0].operation, CauseOperation::ValueDrop);
    assert_eq!(
        (
            d.cause_chain[0].span.as_ref().unwrap().start,
            d.cause_chain[0].span.as_ref().unwrap().end
        ),
        (33, 41)
    );
    assert_eq!(d.cause_chain[0].entity.as_ref().unwrap().id, "binding:17");
    assert!(d.data.is_empty());
    d.validate("shared_conditional.nx", source).unwrap();
}

#[test]
#[ignore = "explicit 100000-program shared-node nested-condition sparse/dense comparison"]
fn shared_node_nested_condition_matches_dense_reference() {
    for seed in 0..100_000 {
        let source = format!(
            "fn f(b:bool){{var r0=mock_open(0);drop(r0);while b{{while if b{{value:true;}}else{{if if b{{value:true;}}else{{r0=mock_open(7);value:false;}}{{}}value:false;}}{{if b{{r0=mock_open({});}}else{{}}}}if b{{r0=mock_open({});}}else{{}}}}inspect(r0);}}fn main(){{}}",
            seed % 97,
            seed % 89
        );
        match compile_scalar("sparse_dense_oracle.nx", &source) {
            Err(CompileError::Rejected(errors)) if errors.iter().all(|d| d.code == "NX-E0202") => {
                for diagnostic in errors {
                    diagnostic
                        .validate("sparse_dense_oracle.nx", &source)
                        .unwrap();
                }
            }
            other => panic!("seed {seed}: {other:?}"),
        }
    }
}

#[test]
fn shared_node_shortest_path_preserves_source_cause() {
    use crate::diagnostic::CauseOperation;
    let source = "fn f(b:bool){var r2=mock_open(0);while if b{if drop(r2)==(){while if b{value:true;}else{value:false;}{}}else{}value:true;}else{value:false;}{drop(r2);}}fn main(){}";
    let Err(CompileError::Rejected(errors)) = compile_scalar("shared_witness.nx", source) else {
        panic!("expected ownership error");
    };
    let use_at = source.rfind("drop(r2)").unwrap() as u64 + 5;
    let d = errors
        .iter()
        .find(|d| d.primary_span.as_ref().is_some_and(|s| s.start == use_at))
        .unwrap();
    assert_eq!(d.primary_span.as_ref().unwrap().end, use_at + 2);
    assert_eq!(
        d.cause_chain.last().unwrap().operation,
        CauseOperation::ValueDrop
    );
    assert_eq!(
        d.cause_chain.last().unwrap().span.as_ref().unwrap().start,
        source.find("drop(r2)").unwrap() as u64
    );
    d.validate("shared_witness.nx", source).unwrap();
}

#[test]
fn flow_budget_resets_for_each_function() {
    let mut source = String::new();
    for n in 0..20 {
        source.push_str(&format!("fn f{n}(b:bool){{while b{{}}}}"));
    }
    source.push_str("fn main(){}");
    assert!(compile_scalar("quota_preserve.nx", &source).is_ok());
}

#[test]
fn shortest_backedge_witness_uses_fewer_cfg_edges() {
    use crate::diagnostic::CauseOperation;
    let source = "fn f(b:bool){let r=mock_open(1);while b{inspect(r);if b{drop(r);if b{}else{}if b{}else{}}else{drop(r);}}}fn main(){}";
    let Err(CompileError::Rejected(ds)) = compile_scalar("short_witness.nx", source) else {
        panic!("expected ownership error");
    };
    let d = ds
        .iter()
        .find(|d| {
            d.primary_span
                .as_ref()
                .is_some_and(|s| s.start == source.find("inspect(r)").unwrap() as u64 + 8)
        })
        .unwrap();
    assert_eq!(d.cause_chain[0].operation, CauseOperation::FlowBackedge);
    assert_eq!(
        d.cause_chain.last().unwrap().operation,
        CauseOperation::ValueDrop
    );
    assert_eq!(
        d.cause_chain.last().unwrap().span.as_ref().unwrap().start,
        source.rfind("drop(r)").unwrap() as u64
    );
}

#[test]
fn loop_loan_origin_is_checked_after_a_prior_iteration_consumes_it() {
    let source = "fn g(inout x:MockResource){}fn f(b:bool){var r=mock_open(1);while b{g(inout r);drop(r);}}fn main(){}";
    let Err(CompileError::Rejected(ds)) = compile_scalar("loan_after_backedge.nx", source) else {
        panic!("expected consumed loan origin");
    };
    assert!(
        ds.iter().any(|d| d.code == "NX-E0202"
            && d.primary_span
                .as_ref()
                .is_some_and(|s| s.start == source.find("inout r").unwrap() as u64)),
        "{ds:?}"
    );
}

#[test]
fn unreachable_loop_backedge_witness_marks_its_closing_brace() {
    use crate::diagnostic::RelatedRole;
    let source = "fn f(b:bool){return;let r=mock_open(1);while b{drop(r);}}fn main(){}";
    let Err(CompileError::Rejected(ds)) = compile_scalar("dead_closing.nx", source) else {
        panic!("expected dead loop error");
    };
    let d = ds.iter().find(|d| d.code == "NX-E0202").unwrap();
    let closing = source.find("drop(r);").unwrap() as u64 + "drop(r);".len() as u64;
    assert!(
        d.related_spans
            .iter()
            .any(|r| r.role == RelatedRole::Backedge
                && r.span.start == closing
                && r.span.end == closing + 1),
        "{d:?}"
    );
}

#[test]
fn loop_reinitialization_and_read_borrows_reach_a_stable_state() {
    for source in [
        "fn main(){var r=mock_open(1);var n=0;while n<1{drop(r);r=mock_open(2);n=n+1;}inspect(r);}",
        "fn f(b:bool){let r=mock_open(1);while b{inspect(r);}} fn main(){}",
        "fn f(b:bool){while b{let r=mock_open(1);drop(r);}} fn main(){}",
        "fn main(){var r=mock_open(1);var n=0;while n<2{if n==0{drop(r);r=mock_open(2);}else{inspect(r);}n=n+1;}drop(r);}",
        "fn f(b:bool){let r=mock_open(1);while b{drop(r);return;}} fn main(){}",
        "fn f(inout x:i64,b:bool){while b{x=x+1;return;}} fn main(){}",
        "fn f(b:bool){let r=mock_open(1);while b{while b{inspect(r);return;}}} fn main(){}",
    ] {
        let result = compile_scalar("loop_valid.nx", source);
        assert!(result.is_ok(), "{source}: {result:?}");
    }
}

#[test]
fn conditional_consumption_and_loop_exit_keep_backedge_witnesses() {
    for source in [
        "fn f(b:bool){let r=mock_open(1);while b{if b{drop(r);}}} fn main(){}",
        "fn f(b:bool){let r=mock_open(1);while b{if b{drop(r);}else{drop(r);}}} fn main(){}",
        "fn f(b:bool){let r=mock_open(1);while b{drop(r);}inspect(r);} fn main(){}",
        "fn f(b:bool){let r=mock_open(1);while drop(r)==(){}} fn main(){}",
    ] {
        let result = compile_scalar("loop_invalid.nx", source);
        assert!(
            matches!(&result, Err(CompileError::Rejected(ds)) if ds.iter().any(|d| d.code == "NX-E0202" && d.related_spans.iter().any(|r| r.role == crate::diagnostic::RelatedRole::Backedge))),
            "{source}: {result:?}"
        );
    }
}

#[test]
fn independent_backedge_consumptions_keep_two_diagnostics() {
    let source =
        "fn f(b:bool){let r=mock_open(1);let s=mock_open(2);while b{drop(r);drop(s);}} fn main(){}";
    let result = compile_scalar("two_loops.nx", source);
    let Err(CompileError::Rejected(errors)) = result else {
        panic!("{result:?}")
    };
    assert_eq!(
        errors
            .iter()
            .map(|d| (d.code.as_str(), d.subcause.as_deref()))
            .collect::<Vec<_>>(),
        [
            ("NX-E0202", Some("maybe_consumed")),
            ("NX-E0202", Some("maybe_consumed")),
        ]
    );
    assert!(
        errors[0].primary_span.as_ref().unwrap().start
            < errors[1].primary_span.as_ref().unwrap().start
    );
    for d in errors {
        assert_eq!(
            d.cause_chain[0].operation,
            crate::diagnostic::CauseOperation::FlowBackedge
        );
        d.validate("two_loops.nx", source).unwrap();
    }
}

#[test]
fn independent_functions_keep_each_backedge_diagnostic() {
    let source = "fn f(b:bool){let r=mock_open(1);while b{drop(r);}}fn g(b:bool){let s=mock_open(2);while b{drop(s);}}fn main(){}";
    let ast = crate::frontend::parse::parse("two_functions.nx", source).unwrap();
    let canonical = crate::frontend::print::print_canonical(&ast);
    let Err(CompileError::Rejected(errors)) = compile_scalar("two_functions.nx", &canonical) else {
        panic!("expected independent ownership diagnostics for {canonical}")
    };
    assert_eq!(errors.len(), 2);
    assert!(
        errors
            .iter()
            .all(|d| d.code == "NX-E0202" && d.subcause.as_deref() == Some("maybe_consumed"))
    );
    assert!(
        errors[0].primary_span.as_ref().unwrap().start
            < errors[1].primary_span.as_ref().unwrap().start
    );
    for d in errors {
        d.validate("two_functions.nx", &canonical).unwrap();
    }
}

#[test]
fn nested_loop_witness_runs_from_use_through_outer_and_inner_backedges() {
    let source =
        "fn f(b:bool){let r=mock_open(1);while inspect(r)==(){while b{drop(r);}}} fn main(){}";
    let result = compile_scalar("nested_witness.nx", source);
    let Err(CompileError::Rejected(errors)) = result else {
        panic!("{result:?}")
    };
    let read = source.find("inspect(r)").unwrap() as u64 + 8;
    let d = errors
        .iter()
        .find(|d| d.primary_span.as_ref().is_some_and(|s| s.start == read))
        .unwrap();
    assert_eq!(
        d.cause_chain
            .iter()
            .map(|c| c.operation)
            .collect::<Vec<_>>(),
        [
            crate::diagnostic::CauseOperation::FlowBackedge,
            crate::diagnostic::CauseOperation::FlowBackedge,
            crate::diagnostic::CauseOperation::ValueDrop,
        ]
    );
    let backedges: Vec<_> = d
        .cause_chain
        .iter()
        .take(2)
        .map(|c| c.span.as_ref().unwrap().start)
        .collect();
    assert!(backedges[0] > backedges[1]);
    d.validate("nested_witness.nx", source).unwrap();
}

#[test]
fn body_local_conditional_drop_is_cleaned_per_iteration() {
    let source = "fn f(b:bool){while b{let r=mock_open(1);if b{drop(r);}}} fn main(){}";
    let result = compile_scalar("loop_local.nx", source);
    assert!(result.is_ok(), "{result:?}");
}

#[test]
fn consumed_parameter_across_backedge_keeps_parameter_identity() {
    use crate::diagnostic::{CauseKind, CauseOperation, EntityKind, RelatedRole};
    let source = "fn f(consume r:MockResource,b:bool){while b{drop(r);}}fn main(){}";
    let ast = crate::frontend::parse::parse("parameter_loop.nx", source).unwrap();
    let canonical = crate::frontend::print::print_canonical(&ast);
    let declaration = canonical.find("consume r:").unwrap() as u64 + "consume ".len() as u64;
    let use_at = canonical.find("drop(r)").unwrap() as u64 + "drop(".len() as u64;
    let Err(CompileError::Rejected(errors)) = compile_scalar("parameter_loop.nx", &canonical)
    else {
        panic!("expected ownership diagnostic for {canonical}")
    };
    let d = errors
        .iter()
        .find(|d| d.primary_span.as_ref().is_some_and(|s| s.start == use_at))
        .unwrap();
    assert_eq!(d.code, "NX-E0202");
    assert_eq!(d.subcause.as_deref(), Some("maybe_consumed"));
    assert_eq!(
        d.related_spans
            .iter()
            .find(|r| r.role == RelatedRole::Declaration)
            .unwrap()
            .span
            .start,
        declaration
    );
    assert_eq!(d.cause_chain[0].kind, CauseKind::Backedge);
    assert_eq!(d.cause_chain[0].operation, CauseOperation::FlowBackedge);
    assert_eq!(
        d.cause_chain[0].entity.as_ref().unwrap().kind,
        EntityKind::Parameter
    );
    assert_eq!(
        d.cause_chain[0].entity.as_ref().unwrap().id,
        format!("parameter:{declaration}")
    );
    d.validate("parameter_loop.nx", &canonical).unwrap();
}

#[test]
fn transferred_binding_across_backedge_keeps_consume_cause() {
    use crate::diagnostic::{CauseKind, CauseOperation};
    let source = "fn take(consume r:MockResource){}fn f(b:bool){let r=mock_open(1);while b{take(consume r);}}fn main(){}";
    let ast = crate::frontend::parse::parse("transfer_loop.nx", source).unwrap();
    let canonical = crate::frontend::print::print_canonical(&ast);
    let Err(CompileError::Rejected(errors)) = compile_scalar("transfer_loop.nx", &canonical) else {
        panic!("expected ownership diagnostic for {canonical}")
    };
    let d = errors.iter().find(|d| d.code == "NX-E0202").unwrap();
    assert_eq!(d.cause_chain[0].kind, CauseKind::Backedge);
    assert_eq!(d.cause_chain.last().unwrap().kind, CauseKind::Consume);
    assert_eq!(
        d.cause_chain.last().unwrap().operation,
        CauseOperation::ValueConsume
    );
    d.validate("transfer_loop.nx", &canonical).unwrap();
}

#[test]
fn unreachable_loop_still_checks_later_iteration() {
    let source = "fn f(b:bool){return;let r=mock_open(1);while b{drop(r);}} fn main(){}";
    let result = compile_scalar("dead_loop.nx", source);
    let Err(CompileError::Rejected(ds)) = result else {
        panic!("{result:?}")
    };
    let d = ds.iter().find(|d| d.code == "NX-E0202").unwrap();
    assert_eq!(d.subcause.as_deref(), Some("maybe_consumed"));
    assert_eq!(
        d.cause_chain[0].operation,
        crate::diagnostic::CauseOperation::FlowBackedge
    );
    d.validate("dead_loop.nx", source).unwrap();
}

#[test]
fn live_and_unreachable_loop_use_the_same_ownership_rules() {
    for body in [
        "let r=mock_open(1);while b{drop(r);}",
        "var r=mock_open(1);while b{drop(r);r=mock_open(2);}",
        "let r=mock_open(1);while b{inspect(r);}",
        "let r=mock_open(1);while b{if b{drop(r);}}",
    ] {
        let compile = |prefix: &str| {
            compile_scalar(
                "mode_loop.nx",
                &format!("fn f(b:bool){{{prefix}{body}}}fn main(){{}}"),
            )
        };
        let live = compile("");
        let dead = compile("return;");
        let signature = |result: Result<super::Program, CompileError>| match result {
            Ok(_) => None,
            Err(CompileError::Rejected(ds)) => Some(
                ds.iter()
                    .map(|d| (d.code.clone(), d.subcause.clone()))
                    .collect::<Vec<_>>(),
            ),
            Err(other) => panic!("unexpected {other:?}"),
        };
        assert_eq!(signature(live), signature(dead), "{body}");
    }
}

#[test]
fn repeated_loop_analysis_has_linear_step_growth() {
    use std::sync::atomic::Ordering::Relaxed;
    let mut samples = Vec::new();
    for target in [32 * 1024, 128 * 1024] {
        let mut source = String::from("fn main(){");
        while source.len() + "while false{}".len() < target {
            source.push_str("while false{}");
        }
        source.push('}');
        super::ownership_flow::FLOW_WORK.store(0, Relaxed);
        compile_scalar("flow_short_scale.nx", &source).unwrap();
        samples.push(super::ownership_flow::FLOW_WORK.load(Relaxed));
    }
    let ratio = samples[1] as f64 / samples[0] as f64;
    assert!((3.5..=4.6).contains(&ratio), "{samples:?}: {ratio}");
}

#[test]
fn consumed_prefix_then_loop_suffix_converges_before_each_exit() {
    use std::sync::atomic::Ordering::Relaxed;
    let mut samples = Vec::new();
    for count in [700, 1100, 3000, 10000] {
        let mut source = String::from("fn f(b:bool){");
        for i in 0..count {
            source.push_str(&format!("var r{i}=mock_open(1);drop(r{i});"));
        }
        for i in 0..count {
            source.push_str(&format!("while b{{r{i}=mock_open(2);}}"));
        }
        source.push_str("}fn main(){}");
        super::ownership_flow::PREFIX_FLOW_WORK.store(0, Relaxed);
        let result = compile_scalar("prefix_scale.nx", &source);
        assert!(
            result.is_ok(),
            "{count} / {} bytes: {result:?}",
            source.len()
        );
        samples.push(super::ownership_flow::PREFIX_FLOW_WORK.load(Relaxed));
    }
    for ((small, large), (a, b)) in
        samples
            .windows(2)
            .map(|w| (w[0], w[1]))
            .zip([(700, 1100), (1100, 3000), (3000, 10000)])
    {
        let ratio = large as f64 / small as f64;
        assert!(ratio <= b as f64 / a as f64 * 1.15, "{samples:?}: {ratio}");
    }
}

#[test]
fn ownership_trie_addresses_resource_after_65536_locals() {
    let mut source = String::from("fn f(b:bool){");
    for i in 0..65_537 {
        source.push_str(&format!("let x{i}=0;"));
    }
    source.push_str("var r=mock_open(1);drop(r);while b{r=mock_open(2);}}fn main(){}");
    assert!(source.len() < 1_048_576, "{}", source.len());
    let result = compile_scalar("large_local_index.nx", &source);
    assert!(result.is_ok(), "{} bytes: {result:?}", source.len());
}

#[test]
#[ignore = "explicit 256 KiB to 1 MiB consumed-prefix loop-suffix scale measurement"]
fn consumed_prefix_loop_suffix_scales_across_source_budget() {
    use std::sync::atomic::Ordering::Relaxed;
    let mut samples = Vec::new();
    for (count, target) in [(2700, 256 * 1024), (10800, 1024 * 1024)] {
        let mut source = String::from("fn f(b:bool){");
        for i in 0..count {
            source.push_str(&format!("var r{i}=mock_open(1);drop(r{i});"));
        }
        for i in 0..count {
            source.push_str(&format!("while b{{r{i}=mock_open(2);}}"));
        }
        let ending = "}fn main(){}";
        assert!(source.len() + ending.len() <= target);
        source.push_str(&" ".repeat(target - source.len() - ending.len()));
        source.push_str(ending);
        super::ownership_flow::PREFIX_FLOW_WORK.store(0, Relaxed);
        let result = compile_scalar("prefix_scale.nx", &source);
        assert!(
            result.is_ok(),
            "{count} / {} bytes: {result:?}",
            source.len()
        );
        samples.push(super::ownership_flow::PREFIX_FLOW_WORK.load(Relaxed));
    }
    let ratio = samples[1] as f64 / samples[0] as f64;
    assert!((3.5..=4.6).contains(&ratio), "{samples:?}: {ratio}");
}

#[test]
fn consumed_bindings_before_many_small_loops_keep_sparse_state_linear() {
    use std::sync::atomic::Ordering::Relaxed;
    let mut samples = Vec::new();
    for target in [32 * 1024, 128 * 1024] {
        let mut source = String::from("fn main(){");
        for i in 0.. {
            let item = format!("let r{i}=mock_open(1);drop(r{i});while false{{}}");
            if source.len() + item.len() + 1 >= target {
                break;
            }
            source.push_str(&item);
        }
        source.push('}');
        super::ownership_flow::SPARSE_FLOW_WORK.store(0, Relaxed);
        super::ownership_flow::FLOW_STATE_ALLOCS.store(0, Relaxed);
        super::ownership_flow::FLOW_STATE_BYTES.store(0, Relaxed);
        compile_scalar("sparse_scale.nx", &source).unwrap();
        samples.push([
            super::ownership_flow::SPARSE_FLOW_WORK.load(Relaxed),
            super::ownership_flow::FLOW_STATE_ALLOCS.load(Relaxed),
            super::ownership_flow::FLOW_STATE_BYTES.load(Relaxed),
        ]);
    }
    for (small, large) in samples[0].into_iter().zip(samples[1]) {
        let ratio = large as f64 / small as f64;
        assert!((3.5..=4.6).contains(&ratio), "{samples:?}: {ratio}");
    }
}

fn sparse_scale_source(family: usize, target: usize) -> String {
    if family == 10 {
        let mut source = String::new();
        for function in 0..100 {
            source.push_str(&format!("fn f{function}(b:bool){{"));
            let function_start = source.len();
            for i in 0.. {
                let item =
                    format!("var r{i}=mock_open(1);while b{{drop(r{i});r{i}=mock_open(2);}}");
                if source.len() - function_start + item.len() + 1 >= target / 100 - 64 {
                    break;
                }
                source.push_str(&item);
            }
            source.push('}');
        }
        source.push_str("fn main(){}");
        return source;
    }
    if family == 11 || family == 12 || family == 13 {
        let mut source = String::from("fn f(b:bool){");
        let mut body = String::new();
        let mut after = String::new();
        for i in 0.. {
            let declaration = if family == 13 {
                format!("var r{i}=mock_open(1);drop(r{i});")
            } else {
                format!("var r{i}=mock_open(1);")
            };
            let action = match family {
                11 => format!("drop(r{i});r{i}=mock_open(2);"),
                12 => format!("if b{{drop(r{i});r{i}=mock_open(2);}}else{{inspect(r{i});}}"),
                _ => String::new(),
            };
            let end = if family == 13 {
                format!("r{i}=mock_open(2);inspect(r{i});")
            } else {
                String::new()
            };
            if source.len()
                + body.len()
                + after.len()
                + declaration.len()
                + action.len()
                + end.len()
                + 32
                >= target
            {
                break;
            }
            source.push_str(&declaration);
            body.push_str(&action);
            after.push_str(&end);
        }
        source.push_str("while b{");
        source.push_str(&body);
        source.push('}');
        source.push_str(&after);
        source.push_str("}fn main(){}");
        return source;
    }
    let mut source = if family == 8 {
        String::from("fn g(inout x:MockResource){}fn f(b:bool){")
    } else {
        String::from("fn f(b:bool){")
    };
    let mut returned = false;
    for i in 0.. {
        if family == 9 && !returned && source.len() >= target / 2 {
            source.push_str("return;");
            returned = true;
        }
        let item = match family {
            0 => format!("let r{i}=mock_open(1);drop(r{i});while false{{}}"),
            1 => format!("var r{i}=mock_open(1);while b{{if b{{drop(r{i});r{i}=mock_open(2);}}}}"),
            2 => format!("let r{i}=mock_open(1);while b{{while b{{inspect(r{i});}}}}"),
            3 => format!(
                "var r{i}=mock_open(1);drop(r{i});r{i}=mock_open(2);while b{{inspect(r{i});}}inspect(r{i});"
            ),
            4 => format!("var r{i}=mock_open(1);while b{{drop(r{i});r{i}=mock_open(2);}}"),
            5 => format!(
                "var r{i}=mock_open(1);while b{{if b{{drop(r{i});r{i}=mock_open(2);}}else{{inspect(r{i});}}}}"
            ),
            6 => format!("var r{i}=mock_open(1);if b{{drop(r{i});}}else{{}}r{i}=mock_open(2);"),
            7 => format!(
                "var r{i}=mock_open(1);while if b{{inspect(r{i});value:false;}}else{{value:false;}}{{}}"
            ),
            8 => format!("var r{i}=mock_open(1);while b{{g(inout r{i});}}"),
            9 => format!("var r{i}=mock_open(1);while b{{drop(r{i});r{i}=mock_open(2);}}"),
            _ => unreachable!(),
        };
        if source.len() + item.len() + 13 >= target {
            break;
        }
        source.push_str(&item);
    }
    source.push_str("}fn main(){}");
    source
}

#[test]
#[ignore = "explicit adversarial 256-KiB to 1-MiB sparse-state measurement"]
fn sparse_ownership_scales_across_fourteen_adversarial_families() {
    use std::sync::atomic::Ordering::Relaxed;
    for family in 0..14 {
        if let Ok(only) = std::env::var("NEXO_I07B_SCALE_FAMILY")
            && only.parse::<usize>().ok() != Some(family)
        {
            continue;
        }
        let mut samples = Vec::new();
        for target in [256 * 1024, 1024 * 1024] {
            let source = sparse_scale_source(family, target);
            super::ownership_flow::SPARSE_FLOW_WORK.store(0, Relaxed);
            super::ownership_flow::FLOW_STATE_ALLOCS.store(0, Relaxed);
            super::ownership_flow::FLOW_STATE_BYTES.store(0, Relaxed);
            let start = std::time::Instant::now();
            compile_scalar("sparse_scale.nx", &source)
                .unwrap_or_else(|e| panic!("family {family}, {} bytes: {e:?}", source.len()));
            let elapsed = start.elapsed().as_nanos() as u64;
            samples.push((
                source.len(),
                [
                    super::ownership_flow::SPARSE_FLOW_WORK.load(Relaxed),
                    super::ownership_flow::FLOW_STATE_ALLOCS.load(Relaxed),
                    super::ownership_flow::FLOW_STATE_BYTES.load(Relaxed),
                    elapsed,
                ],
            ));
        }
        let input_ratio = samples[1].0 as f64 / samples[0].0 as f64;
        for metric in 0..4 {
            let small = samples[0].1[metric];
            let large = samples[1].1[metric];
            if small == 0 {
                assert_eq!(large, 0, "family {family}, metric {metric}");
                continue;
            }
            let ratio = large as f64 / small as f64;
            eprintln!(
                "sparse family {family} metric {metric}: {small} -> {large}, ratio {ratio:.3}, input {input_ratio:.3}"
            );
            assert!(
                ratio <= if metric == 3 { 8.0 } else { 4.6 },
                "family {family}, metric {metric}: {ratio:.3}"
            );
        }
    }
}

#[test]
fn nested_loops_compile_on_small_calling_stacks() {
    for stack in [128 * 1024, 256 * 1024, 512 * 1024] {
        let handle = std::thread::Builder::new()
            .stack_size(stack)
            .spawn(|| {
                for depth in [32, 64, 96, 120] {
                    let source = format!(
                        "fn main(){{{}{}{} }}",
                        "while false{".repeat(depth),
                        "print(1);",
                        "}".repeat(depth)
                    );
                    let result = compile_scalar("nested_stack.nx", &source);
                    assert!(result.is_ok(), "depth {depth}: {result:?}");
                }
            })
            .unwrap();
        handle.join().unwrap();
    }
}

// A small source-AST interpreter, independent of the CFG and its transfer code.
// The two bits describe paths that still own r and paths that have consumed it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Paths {
    owns: bool,
    consumed: bool,
}

impl Paths {
    fn join(self, other: Self) -> Self {
        Self {
            owns: self.owns || other.owns,
            consumed: self.consumed || other.consumed,
        }
    }
}

#[derive(Clone)]
enum SourceStmt {
    Drop,
    Consume,
    Inspect,
    Loan,
    Reset,
    If(Vec<SourceStmt>, Vec<SourceStmt>),
    While(bool, Vec<SourceStmt>),
    Return,
}

fn source_flow(
    statements: &[SourceStmt],
    mut state: Paths,
    errors: &mut Vec<bool>,
) -> Option<Paths> {
    let mut alive = true;
    for stmt in statements {
        match stmt {
            SourceStmt::Drop | SourceStmt::Consume => {
                if state.consumed {
                    errors.push(state.owns);
                }
                state = Paths {
                    owns: false,
                    consumed: true,
                };
            }
            SourceStmt::Inspect | SourceStmt::Loan => {
                if state.consumed {
                    errors.push(state.owns);
                }
            }
            SourceStmt::Reset => {
                state = Paths {
                    owns: true,
                    consumed: false,
                }
            }
            SourceStmt::If(yes, no) => {
                let mut ignored = Vec::new();
                let left = source_flow(yes, state, &mut ignored);
                let right = source_flow(no, state, &mut ignored);
                match (left, right) {
                    (Some(a), Some(b)) => state = a.join(b),
                    (Some(a), None) | (None, Some(a)) => state = a,
                    (None, None) => alive = false,
                };
                if !ignored.is_empty() {
                    // The same branches are evaluated again below once for
                    // diagnostics, after any enclosing loop has converged.
                    errors.extend(ignored);
                }
            }
            SourceStmt::While(read_condition, body) => {
                let entry = state;
                let mut head = entry;
                for _ in 0..4 {
                    let mut ignored = Vec::new();
                    let body_exit = source_flow(body, head, &mut ignored);
                    let next = body_exit.map_or(entry, |body_exit| entry.join(body_exit));
                    if next == head {
                        break;
                    }
                    head = next;
                }
                if *read_condition && head.consumed {
                    errors.push(head.owns);
                }
                let mut body_errors = Vec::new();
                source_flow(body, head, &mut body_errors);
                errors.extend(body_errors);
                state = head;
            }
            SourceStmt::Return => alive = false,
        }
    }
    alive.then_some(state)
}

fn print_source(statements: &[SourceStmt], out: &mut String) {
    for stmt in statements {
        match stmt {
            SourceStmt::Drop => out.push_str("drop(r);"),
            SourceStmt::Consume => out.push_str("take(consume r);"),
            SourceStmt::Inspect => out.push_str("inspect(r);"),
            SourceStmt::Loan => out.push_str("g(inout r);"),
            SourceStmt::Reset => out.push_str("r=mock_open(2);"),
            SourceStmt::If(yes, no) => {
                out.push_str("if b{");
                print_source(yes, out);
                out.push_str("}else{");
                print_source(no, out);
                out.push('}');
            }
            SourceStmt::While(read_condition, body) => {
                out.push_str(if *read_condition {
                    "while inspect(r)==(){"
                } else {
                    "while b{"
                });
                print_source(body, out);
                out.push('}');
            }
            SourceStmt::Return => out.push_str("return;"),
        }
    }
}

fn next(state: &mut u64) -> u64 {
    *state = state
        .wrapping_mul(6364136223846793005)
        .wrapping_add(1442695040888963407);
    *state >> 24
}

fn generated_tree(state: &mut u64, depth: usize, count: usize) -> Vec<SourceStmt> {
    (0..count)
        .map(|at| {
            if at + 1 == count && next(state).is_multiple_of(8) {
                return SourceStmt::Return;
            }
            match next(state) % if depth == 0 { 5 } else { 8 } {
                0 => SourceStmt::Drop,
                1 => SourceStmt::Inspect,
                2 => SourceStmt::Reset,
                3 => SourceStmt::Consume,
                4 => SourceStmt::Loan,
                5 => {
                    let yes_count = 1 + next(state) as usize % 2;
                    let yes = generated_tree(state, depth - 1, yes_count);
                    let no_count = 1 + next(state) as usize % 2;
                    let no = generated_tree(state, depth - 1, no_count);
                    SourceStmt::If(yes, no)
                }
                _ => {
                    let read = next(state).is_multiple_of(2);
                    let body_count = 1 + next(state) as usize % 2;
                    SourceStmt::While(read, generated_tree(state, depth - 1, body_count))
                }
            }
        })
        .collect()
}

fn generated_ownership_oracle(samples: u64, file: &str) {
    for seed in 0..samples {
        let mut state = seed ^ 0x7c10_05a1_71cf_0001;
        let body = generated_tree(&mut state, 2 + (seed as usize % 2), 2 + (seed as usize % 3));
        let mut expected = Vec::new();
        source_flow(
            &body,
            Paths {
                owns: true,
                consumed: false,
            },
            &mut expected,
        );
        let mut source = String::from(
            "fn take(consume x:MockResource){}fn g(inout x:MockResource){}fn f(b:bool){var r=mock_open(1);",
        );
        print_source(&body, &mut source);
        source.push_str("}fn main(){}");
        let ast = crate::frontend::parse::parse(file, &source).unwrap();
        let canonical = crate::frontend::print::print_canonical(&ast);
        let result = compile_scalar(file, &canonical);
        match (expected.is_empty(), result) {
            (true, Ok(_)) => {}
            (false, Err(CompileError::Rejected(errors)))
                if errors.iter().any(|d| d.code == "NX-E0202") =>
            {
                for d in errors {
                    d.validate(file, &canonical).unwrap();
                }
            }
            (valid, other) => panic!("seed {seed}, model valid {valid}: {canonical}: {other:?}"),
        }
    }
}

#[test]
fn generated_nested_loop_ownership_matches_source_ast_model() {
    generated_ownership_oracle(3_000, "ownership_oracle.nx");
}

#[test]
fn generated_sparse_ownership_matches_dense_reference_in_regular_tests() {
    generated_ownership_oracle(3_000, "sparse_dense_oracle.nx");
}

#[test]
#[ignore = "explicit 200000-program source-AST ownership campaign"]
fn extensive_nested_loop_ownership_matches_source_ast_model() {
    generated_ownership_oracle(200_000, "ownership_oracle.nx");
}

#[test]
#[ignore = "explicit 100000-program sparse versus dense ownership comparison"]
fn generated_sparse_ownership_matches_dense_reference_exactly() {
    generated_ownership_oracle(100_000, "sparse_dense_oracle.nx");
}

#[test]
#[ignore = "explicit 15-family 256-KiB to 1-MiB CFG scale measurement"]
fn loop_fixpoint_scales_by_steps_and_allocation_traffic() {
    use std::sync::atomic::Ordering::Relaxed;
    let patterns = [
        "while false{}",
        "while false{while false{}}",
        "while false{if true{}else{}}",
        "while false{let r=mock_open(1);drop(r);}",
        "while false{let r=mock_open(1);inspect(r);}",
        "while false{while false{while false{while false{}}}}",
        "while (if false{value:true;}else{value:false;}){}",
        "{let r=mock_open(1);while false{inspect(r);}}",
        "{var r=mock_open(1);while false{drop(r);r=mock_open(2);}}",
        "{var r=mock_open(1);while false{if true{drop(r);r=mock_open(2);}else{inspect(r);}}}",
        "{var r=mock_open(1);while inspect(r)==(){drop(r);r=mock_open(2);}}",
        "while false{if true{while false{}}else{}}",
        "{let r=mock_open(1);while false{if true{inspect(r);}else{inspect(r);}}}",
        "while false{var x=0;while x<1{x=x+1;}}",
        "while false{let r=mock_open(1);if true{drop(r);}else{drop(r);}}",
    ];
    for (family, pattern) in patterns.into_iter().enumerate() {
        if let Ok(only) = std::env::var("NEXO_I07B_SCALE_FAMILY")
            && only.parse::<usize>().ok() != Some(family)
        {
            continue;
        }
        let mut measures = Vec::new();
        for target in [256 * 1024, 1024 * 1024] {
            let mut source = String::from("fn main(){");
            while source.len() + pattern.len() < target {
                source.push_str(pattern);
            }
            source.push('}');
            crate::test_alloc::reset("cfg_scale_counter.nx");
            super::CFG_WORK.store(0, Relaxed);
            super::ownership_flow::FLOW_WORK.store(0, Relaxed);
            compile_scalar("cfg_scale_counter.nx", &source)
                .unwrap_or_else(|e| panic!("family {family} at {target}: {e:?}"));
            let all_steps = super::CFG_WORK.load(Relaxed);
            let flow_steps = super::ownership_flow::FLOW_WORK.load(Relaxed);
            let traffic = crate::test_alloc::snapshot("cfg_scale_counter.nx");
            measures.push([
                all_steps,
                flow_steps,
                all_steps - flow_steps,
                traffic.0,
                traffic.1,
            ]);
        }
        for (metric, (small, large)) in measures[0].into_iter().zip(measures[1]).enumerate() {
            if small == 0 {
                continue;
            }
            let ratio = large as f64 / small as f64;
            eprintln!("family {family} metric {metric}: {small} -> {large} = {ratio:.3}");
            assert!(
                (3.5..=4.6).contains(&ratio),
                "family {family}, metric {metric}: {ratio:.3}"
            );
        }
    }
}
