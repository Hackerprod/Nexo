use super::{
    compile_scalar,
    model::{Op, Terminator},
    work::{WorkEvent, WorkOrigin, WorkPhase, WorkPlanner},
};
use crate::interp::{Outcome, run_scalar};

type InstructionPredicate = fn(&Op) -> bool;
type WorkInstructionCase<'a> = (&'a str, &'a str, InstructionPredicate);
type AdministrativeInstructionCase<'a> = (&'a str, InstructionPredicate);

#[test]
fn empty_function_charges_entry_then_final_unit() {
    let program = compile_scalar("empty_i10b.n", "fn Main(){}").unwrap();
    let function = program.functions().function("Main").unwrap();
    let unit = function
        .block
        .instructions
        .iter()
        .find(|instruction| matches!(instruction.op, Op::ConstUnit))
        .unwrap();
    assert_eq!(unit.charges.last().unwrap().ticks, 1);
    assert_eq!(
        (
            unit.charges.last().unwrap().origin.start,
            unit.charges.last().unwrap().origin.end
        ),
        (10, 11)
    );
    let execution = run_scalar(&program, Default::default());
    assert_eq!(execution.outcome, Outcome::Unit);
    assert_eq!(execution.work_consumed, 2);
}

#[test]
fn runtime_index_diagnostics_use_selector_and_full_context_for_dg_cases() {
    let cases = [
        ("fn Main(){let a=[[0]];Print(a[0][2]);}", 32, 35, 28, 35),
        ("fn Main(){var a=[[0]];a[0][2]=5;}", 26, 29, 22, 29),
        (
            "fn F(x:edit I64){}fn Main(){var a=[[0]];F(edit a[0][2]);}",
            51,
            54,
            42,
            54,
        ),
        (
            "fn F(x:edit I64){}fn Main(){var a=[[0]];F(x:edit a[0][2]);}",
            53,
            56,
            42,
            56,
        ),
    ];
    for (source, start, end, context_start, context_end) in cases {
        let program = compile_scalar("dg_i10b.n", source).unwrap();
        let result = run_scalar(&program, Default::default());
        let diagnostic = &result.diagnostics[0];
        assert_eq!(diagnostic.code, "R0102");
        assert_eq!(diagnostic.operation.as_str(), "value.index");
        assert_eq!(
            diagnostic.primary_span.as_ref().map(|s| (s.start, s.end)),
            Some((start, end))
        );
        assert_eq!(diagnostic.related_spans.len(), 1);
        assert_eq!(
            (
                diagnostic.related_spans[0].span.start,
                diagnostic.related_spans[0].span.end
            ),
            (context_start, context_end)
        );
    }
    let variants = [
        ("fn Main(){let a=[[0]];Print(a[2][0]);}", 29, 32, 28, 35),
        ("fn Main(){var a=[[0]];a[2][0]=5;}", 23, 26, 22, 29),
        (
            "fn F(x:edit I64){}fn Main(){var a=[[0]];F(edit a[2][0]);}",
            48,
            51,
            42,
            54,
        ),
        ("fn Main(){Print([1][1]);}", 19, 22, 16, 22),
        ("fn Main(){var a=[1];a[3]=1;}", 21, 24, 20, 24),
        ("fn Main(){var a=[0];a[ 3 ]=1;}", 21, 26, 20, 26),
    ];
    for (source, start, end, context_start, context_end) in variants {
        let program = compile_scalar("index_variant_i10b.n", source).unwrap();
        let diagnostic = &run_scalar(&program, Default::default()).diagnostics[0];
        assert_eq!(
            diagnostic.primary_span.as_ref().map(|s| (s.start, s.end)),
            Some((start, end))
        );
        assert_eq!(
            (
                diagnostic.related_spans[0].span.start,
                diagnostic.related_spans[0].span.end
            ),
            (context_start, context_end)
        );
    }
    let borrowed = "fn F(x:view I64){}fn Main(){var a=[[0]];F(x:a[0][2]);}";
    let result = run_scalar(
        &compile_scalar("dg_label_i10b.n", borrowed).unwrap(),
        Default::default(),
    );
    assert_eq!(result.diagnostics.len(), 1, "{result:#?}");
    let diagnostic = &result.diagnostics[0];
    assert_eq!(diagnostic.code, "R0102");
    assert_eq!(
        diagnostic
            .primary_span
            .as_ref()
            .map(|span| (span.start, span.end)),
        Some((
            borrowed.find("[2]").unwrap() as u64,
            borrowed.find("[2]").unwrap() as u64 + 3
        ))
    );
    assert_eq!(diagnostic.related_spans.len(), 1);
    assert_eq!(
        (
            diagnostic.related_spans[0].span.start,
            diagnostic.related_spans[0].span.end
        ),
        (
            borrowed.find("x:a[0][2]").unwrap() as u64,
            borrowed.find("x:a[0][2]").unwrap() as u64 + "x:a[0][2]".len() as u64,
        )
    );
}

#[test]
fn equality_stops_at_the_first_different_payload_pair() {
    let source = "fn Main(){AssertEq([1,2],[9,2]);}";
    let program = compile_scalar("equality_work_i10b.n", source).unwrap();
    let result = run_scalar(&program, Default::default());
    assert_eq!(result.outcome, Outcome::SimulatorFault);
    assert_eq!(result.diagnostics[0].code, "R0201");
    assert_eq!(result.work_consumed, 21);
}

#[test]
fn published_work_table_totals_and_deltas_are_preserved() {
    let baseline = "fn Main(){var a=[3,4];var v=7;}";
    let base = run_scalar(
        &compile_scalar("ledger_i10b.n", baseline).unwrap(),
        Default::default(),
    )
    .work_consumed;
    let delta_rows = [
        (baseline.replace(";}", ";var w=v;}"), 3),
        (baseline.replace(";}", ";var b=[v,v];}"), 7),
        (baseline.replace(";}", ";a[1]=v;}"), 6),
        (baseline.replace(";}", ";if v<3 {}else{}}"), 8),
        (baseline.replace(";}", ";Drop(a);}"), 5),
        (baseline.replace(";}", ";var w=a[1]+v;}"), 8),
        (baseline.replace(";}", ";var w=a[1];}"), 6),
        (baseline.replace(";}", ";a[a[0]-3]=1;}"), 12),
        (baseline.replace(";}", ";AssertEq(a[0],3);}"), 13),
        (baseline.replace(";}", ";AssertEq(a[0],a[0]);}"), 15),
        (baseline.replace(";}", ";Print(a[0]+0);}"), 14),
    ];
    for (source, expected_delta) in delta_rows {
        let actual = run_scalar(
            &compile_scalar("ledger_i10b.n", &source).unwrap(),
            Default::default(),
        )
        .work_consumed;
        assert_eq!(actual - base, expected_delta, "{source}");
    }

    let function_source = "fn F(n:view I64)->I64{return n;}fn Main(){var a=[3,4];var v=7;F(v);}";
    let function_baseline = "fn F(n:view I64)->I64{return n;}fn Main(){var a=[3,4];var v=7;}";
    let function_total = run_scalar(
        &compile_scalar("ledger_i10b.n", function_source).unwrap(),
        Default::default(),
    )
    .work_consumed;
    let function_base = run_scalar(
        &compile_scalar("ledger_i10b.n", function_baseline).unwrap(),
        Default::default(),
    )
    .work_consumed;
    assert_eq!(function_total - function_base, 8);

    let nested = "fn Main(){var c=[[7]];}";
    let nested_source = "fn Main(){var c=[[7]];Print(c[0][0]);}";
    let nested_delta = run_scalar(
        &compile_scalar("ledger_i10b.n", nested_source).unwrap(),
        Default::default(),
    )
    .work_consumed
        - run_scalar(
            &compile_scalar("ledger_i10b.n", nested).unwrap(),
            Default::default(),
        )
        .work_consumed;
    assert_eq!(nested_delta, 13);

    let totals = [
        ("fn Main(){AssertEq(1<2,true);}", 19),
        ("fn Main(){return;}", 3),
        ("fn F(){return;}fn Main(){F();}", 8),
        ("fn Main(){let r=MockOpen(7);}", 9),
        ("fn Main(){let a=[1,2];Len(a);}", 15),
        ("fn Main(){let r=MockOpen(7);Inspect(r);}", 15),
        ("fn Main(){let r=TryAdd(1,2);}", 12),
        ("fn Main(){Print(1);}", 10),
    ];
    for (source, expected) in totals {
        let actual = run_scalar(
            &compile_scalar("ledger_i10b.n", source).unwrap(),
            Default::default(),
        )
        .work_consumed;
        assert_eq!(actual, expected, "{source}");
    }
}

#[test]
fn static_work_catalog_has_one_ordered_origin_rule_per_event() {
    let events = [
        (
            WorkEvent::Entry,
            WorkOrigin::FunctionToken,
            WorkPhase::Entry,
            0,
        ),
        (
            WorkEvent::FinalUnit,
            WorkOrigin::ClosingBrace,
            WorkPhase::Exit,
            16,
        ),
        (
            WorkEvent::WhileCondition,
            WorkOrigin::Condition,
            WorkPhase::BeforeChildren,
            1,
        ),
        (
            WorkEvent::InoutPrefix,
            WorkOrigin::EditPrefix,
            WorkPhase::BeforeChildren,
            2,
        ),
        (
            WorkEvent::ReadSelector,
            WorkOrigin::Selector,
            WorkPhase::PreOrder,
            3,
        ),
        (
            WorkEvent::ReadName,
            WorkOrigin::PlaceRoot,
            WorkPhase::PreOrder,
            4,
        ),
        (
            WorkEvent::PlaceSelector,
            WorkOrigin::Selector,
            WorkPhase::PreOrder,
            5,
        ),
        (
            WorkEvent::ReturnStatement,
            WorkOrigin::ReturnToken,
            WorkPhase::BeforeChildren,
            6,
        ),
        (
            WorkEvent::ValueStatement,
            WorkOrigin::ValueMarkerOrOperand,
            WorkPhase::BeforeChildren,
            7,
        ),
        (
            WorkEvent::ExpressionStatement,
            WorkOrigin::Expression,
            WorkPhase::AfterOperand,
            8,
        ),
        (
            WorkEvent::EmptyValue,
            WorkOrigin::ValueStatement,
            WorkPhase::BeforeChildren,
            9,
        ),
        (
            WorkEvent::IfStatement,
            WorkOrigin::IfOrWhileToken,
            WorkPhase::BeforeChildren,
            10,
        ),
        (
            WorkEvent::Block,
            WorkOrigin::OpeningBrace,
            WorkPhase::BeforeChildren,
            11,
        ),
        (
            WorkEvent::Assignment,
            WorkOrigin::FullPlace,
            WorkPhase::PreOrder,
            12,
        ),
        (
            WorkEvent::Binding,
            WorkOrigin::BindingToken,
            WorkPhase::BeforeChildren,
            13,
        ),
        (
            WorkEvent::IfExpression,
            WorkOrigin::BranchOperand,
            WorkPhase::PreOrder,
            14,
        ),
        (
            WorkEvent::IndexExpression,
            WorkOrigin::IndexAccess,
            WorkPhase::PreOrder,
            15,
        ),
    ];
    let mut orders = Vec::new();
    for (event, origin, phase, order) in events {
        assert_eq!(WorkPlanner::units(event), 1);
        assert_eq!(WorkPlanner::origin(event), origin);
        assert_eq!(WorkPlanner::phase(event), phase);
        assert_eq!(WorkPlanner::order(event), order);
        orders.push(order);
    }
    orders.sort_unstable();
    orders.dedup();
    assert_eq!(orders.len(), events.len());
}

#[test]
fn omitted_work_rows_and_recorded_else_if_gap_are_fixed() {
    let rows = [
        ("fn E(x:edit I64){}fn Main(){var a=[1];E(edit a[0]);}", 16),
        ("fn E(x:edit I64){}fn Main(){var x=1;E(edit x);}", 11),
        (
            "fn Main(){let y=(if true then {value:1;}else{value:2;});}",
            9,
        ),
        ("fn Main(){let y=(if true then 1 else 2);}", 9),
        (
            "fn F()->I64{return if true then 1 else 2;}fn Main(){let y=F();}",
            13,
        ),
        ("fn Main(){var x=0;if x<0{}else if x<2{}else{}}", 20),
    ];
    for (source, expected) in rows {
        let actual = run_scalar(
            &compile_scalar("work_rows_i10b.n", source).unwrap(),
            Default::default(),
        );
        assert_eq!(actual.outcome, Outcome::Unit, "{source}");
        assert_eq!(actual.work_consumed, expected, "{source}");
    }

    for source in [
        "fn Main(){let x=[1]==[1,2];}",
        "fn Main(){let x=[1,2]==[1];}",
    ] {
        let actual = run_scalar(
            &compile_scalar("array_len_eq_i10b.n", source).unwrap(),
            Default::default(),
        );
        assert_eq!(actual.outcome, Outcome::Unit, "{source}");
        assert_eq!(actual.work_consumed, 17, "{source}");
    }
    for source in [
        "fn Main(){AssertEq([1],[1,2]);}",
        "fn Main(){AssertEq([1,2],[1]);}",
    ] {
        let actual = run_scalar(
            &compile_scalar("array_len_assert_i10b.n", source).unwrap(),
            Default::default(),
        );
        assert_eq!(actual.diagnostics[0].code, "R0201", "{source}");
        assert_eq!(actual.work_consumed, 17, "{source}");
    }
}

#[test]
fn return_unit_budget_origin_is_the_return_keyword_for_all_terminator_forms() {
    let cases = [
        ("fn F(){return;}fn Main(){while true{F();}}", (7, 13)),
        ("fn F(){return;}fn Main(){{}while true{F();}}", (7, 13)),
        ("fn F(){\nreturn\n}\nfn Main(){while true{F();}}", (8, 14)),
    ];
    for (source, expected) in cases {
        let execution = run_scalar(
            &compile_scalar("return_origin_i10b.n", source).unwrap(),
            Default::default(),
        );
        assert_eq!(execution.diagnostics[0].code, "R0001", "{source}");
        assert_eq!(
            execution.diagnostics[0]
                .primary_span
                .as_ref()
                .map(|span| (span.start, span.end)),
            Some(expected),
            "{source}"
        );
        assert_eq!(execution.work_consumed, 1_000_000, "{source}");
    }
}

#[test]
fn boundary_mutation_witnesses_cover_condition_statement_and_index_preorder_origins() {
    let condition_source = "fn Main(){{}{}while true{}}";
    let condition = run_scalar(
        &compile_scalar("condition_origin_i10b.n", condition_source).unwrap(),
        Default::default(),
    );
    assert_eq!(condition.diagnostics[0].code, "R0001");
    assert_eq!(
        condition.diagnostics[0]
            .primary_span
            .as_ref()
            .map(|span| (span.start, span.end)),
        Some((20, 24))
    );

    let prefix = "fn Main(){var i=0;while i<71427{i=i+1;}";
    for (body, expected) in [("if true{}", (57, 59)), ("while false{}", (57, 62))] {
        let source = format!("{prefix}{}{}{}", "{}".repeat(9), body, "}");
        let execution = run_scalar(
            &compile_scalar("if_while_origin_i10b.n", &source).unwrap(),
            Default::default(),
        );
        assert_eq!(execution.diagnostics[0].code, "R0001", "{source}");
        assert_eq!(
            execution.diagnostics[0]
                .primary_span
                .as_ref()
                .map(|span| (span.start, span.end)),
            Some(expected),
            "{source}"
        );
    }

    let prefix = "fn Main(){var i=0;while i<71426{i=i+1;}";
    for (padding, expected) in [(13, (83, 90)), (12, (81, 85)), (11, (79, 80))] {
        let source = format!(
            "{prefix}{}var c=[[7]];Print(c[0][0]);}}",
            "{}".repeat(padding)
        );
        let execution = run_scalar(
            &compile_scalar("index_order_origin_i10b.n", &source).unwrap(),
            Default::default(),
        );
        assert_eq!(execution.diagnostics[0].code, "R0001", "{source}");
        assert_eq!(
            execution.diagnostics[0]
                .primary_span
                .as_ref()
                .map(|span| (span.start, span.end)),
            Some(expected),
            "{source}"
        );
    }

    let assign = run_scalar(
        &compile_scalar(
            "assign_place_origin_i10b.n",
            "fn Main(){var a=[1];while true{a[0]=1;}}",
        )
        .unwrap(),
        Default::default(),
    );
    assert_eq!(assign.diagnostics[0].code, "R0001");
    assert_eq!(
        assign.diagnostics[0]
            .primary_span
            .as_ref()
            .map(|span| (span.start, span.end)),
        Some((31, 35))
    );
}

#[test]
fn drop_work_and_result_tag_equality_stop_at_the_first_pair() {
    for (source, expected) in [
        ("fn Main(){var a=[1];}", 7),
        ("fn Main(){Drop([1]);}", 11),
        ("fn Main(){var a=[1];Drop(a);}", 12),
    ] {
        assert_eq!(
            run_scalar(
                &compile_scalar("drop_work_i10b.n", source).unwrap(),
                Default::default()
            )
            .work_consumed,
            expected,
            "{source}"
        );
    }
    let source = "fn Main()->Result<(),I64>{let a:Result<I64,I64>=Ok(1);let b:Result<I64,I64>=Err(1);AssertEq(a,b);return Ok(());}";
    let result = run_scalar(
        &compile_scalar("result_tag_eq_i10b.n", source).unwrap(),
        Default::default(),
    );
    assert_eq!(result.diagnostics[0].code, "R0201");
    assert_eq!(result.work_consumed, 23);
}

#[test]
fn drop_binding_machine_charge_is_observable_on_a_verified_ir_route() {
    let source = "fn Main(){var a=[1];}";
    let mut program = compile_scalar("drop_binding_i10b.n", source).unwrap();
    let mut forged = program.functions().function("Main").unwrap().clone();
    let final_unit = std::iter::once(&mut forged.block)
        .chain(&mut forged.blocks)
        .flat_map(|block| &mut block.instructions)
        .find(|instruction| matches!(instruction.op, Op::ConstUnit))
        .unwrap();
    final_unit.op = Op::DropBinding(super::model::LocalId(0));
    final_unit.charges.clear();
    forged.content_hash = super::model::content_hash(&forged);
    assert_eq!(super::verify::verify_function(&forged), Ok(()));
    let mut donor = program.clone();
    donor
        .functions
        .replace_for_development("Main", forged)
        .unwrap();
    program.substitute_for_development("Main", &donor).unwrap();
    let execution = run_scalar(&program, Default::default());
    assert_eq!(execution.outcome, Outcome::Unit);
    assert_eq!(execution.work_consumed, 7);
}

#[test]
fn lowering_does_not_find_statement_origins_by_token_start() {
    let source = include_str!("lower.rs");
    assert!(!source.contains("tokens.iter().find(|token| token.span.start == span.start)"));
}

#[test]
fn work_limit_rejects_the_first_loop_rhs_charge_and_preserves_boundary_cases() {
    let loop_source = "fn Main(){var i=0;while true{i=i+1;}}";
    let program = compile_scalar("loop_boundary_i10b.n", loop_source).unwrap();
    let rejected = run_scalar(&program, Default::default());
    assert_eq!(rejected.diagnostics[0].code, "R0001");
    assert_eq!(
        rejected.diagnostics[0]
            .primary_span
            .as_ref()
            .map(|s| (s.start, s.end)),
        Some((31, 32))
    );
    assert_eq!(rejected.work_consumed, 1_000_000);

    let prefix = "fn Main(){var i=0;while i<71427{i=i+1;}";
    let drop_source = format!("{prefix}var a=[1];Drop(a);}}");
    let drop_result = run_scalar(
        &compile_scalar("drop_boundary_i10b.n", &drop_source).unwrap(),
        Default::default(),
    );
    assert_eq!(drop_result.diagnostics[0].code, "R0001");
    assert_eq!(
        drop_result.diagnostics[0]
            .primary_span
            .as_ref()
            .map(|s| (s.start, s.end)),
        Some((49, 56))
    );
    assert_eq!(
        drop_result.diagnostics[0].data.get("observed"),
        Some(&crate::diagnostic::DataValue::UInt(1_000_001))
    );

    let suffix = "var q=1;var w=q;{}{}{}}";
    let final_source = format!("{prefix}{suffix}");
    let final_result = run_scalar(
        &compile_scalar("final_boundary_i10b.n", &final_source).unwrap(),
        Default::default(),
    );
    assert_eq!(final_result.diagnostics[0].code, "R0001");
    assert_eq!(
        final_result.diagnostics[0]
            .primary_span
            .as_ref()
            .map(|s| (s.start, s.end)),
        Some((61, 62))
    );
    assert_eq!(
        (
            final_result.diagnostics[0].related_spans[0].span.start,
            final_result.diagnostics[0].related_spans[0].span.end
        ),
        (9, 62)
    );
}

#[test]
fn nested_try_charges_use_the_complete_operands_and_boundary_pair() {
    fn source(k: u64, blocks: usize) -> String {
        format!(
            "fn A(n:view I64)->Result<I64,I64>{{return Ok(n);}}fn B(n:view I64)->Result<I64,I64>{{return Ok(n);}}fn Main()->Result<(),I64>{{var i=0;while i<{k}{{i=i+1;}}{}let x=B(A(1)?)?;return Ok(());}}",
            "{}".repeat(blocks)
        )
    }
    let outer = source(71_425, 36);
    let result = run_scalar(
        &compile_scalar("nested_try_i10b.n", &outer).unwrap(),
        Default::default(),
    );
    assert_eq!(result.diagnostics[0].code, "R0001");
    assert_eq!(
        result.diagnostics[0]
            .primary_span
            .as_ref()
            .map(|s| (s.start, s.end)),
        Some((229, 238))
    );
    let inner = source(71_425, 33);
    let result = run_scalar(
        &compile_scalar("nested_try_i10b.n", &inner).unwrap(),
        Default::default(),
    );
    assert_eq!(
        result.diagnostics[0]
            .primary_span
            .as_ref()
            .map(|s| (s.start, s.end)),
        Some((225, 230))
    );
    let safe = source(71_422, 47);
    let result = run_scalar(
        &compile_scalar("nested_try_i10b.n", &safe).unwrap(),
        Default::default(),
    );
    assert_eq!(result.work_consumed, 1_000_000);
    assert_eq!(result.outcome, Outcome::Ok);
    let denied = source(71_422, 48);
    let result = run_scalar(
        &compile_scalar("nested_try_i10b.n", &denied).unwrap(),
        Default::default(),
    );
    assert_eq!(result.diagnostics[0].code, "R0001");
    assert_eq!(
        result.diagnostics[0]
            .primary_span
            .as_ref()
            .map(|s| (s.start, s.end)),
        Some((270, 276))
    );
}

#[test]
fn verifier_rejects_forged_index_site_metadata_after_rehash() {
    use super::model::IndexSite;
    let source = "fn Main(){Print([1][1]);}";
    let valid = compile_scalar("index_site_i10b.n", source).unwrap();
    let original = valid.functions().function("Main").unwrap().clone();
    assert_eq!(super::verify::verify_function(&original), Ok(()));
    let mutate = |edit: fn(&mut IndexSite)| {
        let mut forged = original.clone();
        let instruction = forged
            .block
            .instructions
            .iter_mut()
            .find(|instruction| matches!(instruction.op, Op::IndexValue { .. }))
            .unwrap();
        if let Op::IndexValue { site, .. } = &mut instruction.op {
            edit(site);
        }
        forged.content_hash = super::model::content_hash(&forged);
        assert_eq!(super::verify::verify_function(&forged), Err("index site"));
    };
    mutate(|site| site.selectors.clear());
    mutate(|site| site.context.file = "other.n".into());
    mutate(|site| site.selectors[0].end = site.selectors[0].start);
    mutate(|site| site.selectors[0].start = site.context.end);
}

#[test]
fn verifier_isolates_selector_file_utf8_containment_and_overlap_invariants() {
    use super::model::IndexSite;
    let source = "fn Main(){var a=[[1]];Print(a[0][0]);}";
    let valid = compile_scalar("site_invariants_i10b.n", source).unwrap();
    let original = valid.functions().function("Main").unwrap().clone();
    let mutate = |edit: fn(&mut IndexSite)| {
        let mut forged = original.clone();
        let instruction = std::iter::once(&mut forged.block)
            .chain(&mut forged.blocks)
            .flat_map(|block| &mut block.instructions)
            .find(|instruction| match &instruction.op {
                Op::IndexValue { site, .. }
                | Op::ReadIndex { site, .. }
                | Op::BorrowIndex { site, .. }
                | Op::CheckIndex { site, .. } => site.selectors.len() >= 2,
                _ => false,
            })
            .unwrap();
        if let Op::IndexValue { site, .. }
        | Op::ReadIndex { site, .. }
        | Op::BorrowIndex { site, .. }
        | Op::CheckIndex { site, .. } = &mut instruction.op
        {
            edit(site);
        }
        forged.content_hash = super::model::content_hash(&forged);
        assert_eq!(super::verify::verify_function(&forged), Err("index site"));
    };
    mutate(|site| site.selectors[0].file = "different.n".into());
    mutate(|site| site.selectors[0].start = site.context.start - 1);
    mutate(|site| {
        let first_end = site.selectors[0].end;
        site.selectors[1].start = first_end - 1;
        site.selectors[1].end = first_end + 2;
    });

    let valid = compile_scalar("site_utf8_i10b.n", "fn Main(){var a=[0];Print(a[0]);}").unwrap();
    let mut forged = valid.functions().function("Main").unwrap().clone();
    let original_len = forged.source_text.len() as u64;
    forged.source_text = format!("{}é", forged.source_text).into();
    let inside = original_len + 1;
    let instruction = std::iter::once(&mut forged.block)
        .chain(&mut forged.blocks)
        .flat_map(|block| &mut block.instructions)
        .find(|instruction| {
            matches!(
                instruction.op,
                Op::IndexValue { .. }
                    | Op::ReadIndex { .. }
                    | Op::BorrowIndex { .. }
                    | Op::CheckIndex { .. }
            )
        })
        .unwrap();
    if let Op::IndexValue { site, .. }
    | Op::ReadIndex { site, .. }
    | Op::BorrowIndex { site, .. }
    | Op::CheckIndex { site, .. } = &mut instruction.op
    {
        site.context.end = original_len + 2;
        site.selectors[0].start = inside;
        site.selectors[0].end = original_len + 2;
    }
    forged.content_hash = super::model::content_hash(&forged);
    assert_eq!(super::verify::verify_function(&forged), Err("index site"));
}

#[test]
fn verifier_rejects_selector_ending_after_its_context_only() {
    let mut forged = compile_scalar(
        "site_contain_hi_i10b.n",
        "fn Main(){var a=[1];Print(a[0]);}",
    )
    .unwrap()
    .functions()
    .function("Main")
    .unwrap()
    .clone();
    let instruction = std::iter::once(&mut forged.block)
        .chain(&mut forged.blocks)
        .flat_map(|block| &mut block.instructions)
        .find(|instruction| {
            matches!(
                instruction.op,
                Op::IndexValue { .. }
                    | Op::ReadIndex { .. }
                    | Op::BorrowIndex { .. }
                    | Op::CheckIndex { .. }
            )
        })
        .unwrap();
    match &mut instruction.op {
        Op::IndexValue { site, .. }
        | Op::ReadIndex { site, .. }
        | Op::BorrowIndex { site, .. }
        | Op::CheckIndex { site, .. } => {
            site.selectors[0].end = site.context.end + 1;
        }
        _ => unreachable!(),
    }
    forged.content_hash = super::model::content_hash(&forged);
    assert_eq!(super::verify::verify_function(&forged), Err("index site"));
}

#[test]
fn verifier_rejects_administrative_charges_on_a_multiblock_route() {
    let mut forged = compile_scalar(
        "admin_multiblock_i10b.n",
        "fn Main(){if true{{let a=1;}}else{{let b=2;}}}",
    )
    .unwrap()
    .functions()
    .function("Main")
    .unwrap()
    .clone();
    let instruction = std::iter::once(&mut forged.block)
        .chain(&mut forged.blocks)
        .flat_map(|block| &mut block.instructions)
        .find(|instruction| matches!(instruction.op, Op::DropLocal(_) | Op::DropBinding(_)))
        .unwrap();
    instruction.charges.push(super::model::WorkCharge {
        ticks: 1,
        origin: instruction.span.clone(),
    });
    forged.content_hash = super::model::content_hash(&forged);
    assert_eq!(
        super::verify::verify_function(&forged),
        Err("work charge rule")
    );
}

#[test]
fn finite_loop_matches_the_published_source_ledger() {
    let source = "fn Main(){var i=0;while i<3{i=i+1;}}";
    let program = compile_scalar("loop_i10b.n", source).unwrap();
    let execution = run_scalar(&program, Default::default());
    assert_eq!(execution.outcome, Outcome::Unit);
    assert_eq!(execution.work_consumed, 56);
    let function = program.functions().function("Main").unwrap();
    let preheader = std::iter::once(&function.block).chain(&function.blocks).find(|block| matches!(block.terminator, Terminator::Jump { target, .. } if target.0 > block.id.0)).unwrap();
    assert_eq!(
        preheader
            .terminator
            .terminator_charges()
            .iter()
            .map(|charge| charge.ticks)
            .sum::<u64>(),
        1
    );
}

#[test]
fn indexed_assignment_delta_and_result_error_include_semantic_charges() {
    let with_write = compile_scalar("indexed_i10b.n", "fn Main(){var a=[0,0];a[1]=5;}").unwrap();
    let without_write = compile_scalar("indexed_i10b.n", "fn Main(){var a=[0,0];}").unwrap();
    assert_eq!(
        run_scalar(&with_write, Default::default()).work_consumed
            - run_scalar(&without_write, Default::default()).work_consumed,
        6
    );

    let source = "fn Inc(n:view I64)->Result<I64,I64>{return Err(9);}fn Main()->Result<(),I64>{let x=Inc(2)?;return Ok(());}";
    let program = compile_scalar("result_i10b.n", source).unwrap();
    let main = program.functions().function("Main").unwrap();
    let try_origin = source.find("Inc(2)?").unwrap() as u64;
    assert!(
        std::iter::once(&main.block)
            .chain(&main.blocks)
            .flat_map(|block| &block.instructions)
            .flat_map(|instruction| &instruction.charges)
            .any(|charge| (charge.origin.start, charge.origin.end) == (try_origin, try_origin + 7))
    );
    let execution = run_scalar(&program, Default::default());
    assert_eq!(execution.outcome, Outcome::Err(9));
    assert_eq!(execution.work_consumed, 15);

    let success = "fn Inc(n:view I64)->Result<I64,I64>{return Ok(n+1);}fn Main()->Result<(),I64>{let x=Inc(2)?;return Ok(());}";
    let program = compile_scalar("result_ok_i10b.n", success).unwrap();
    assert_eq!(run_scalar(&program, Default::default()).work_consumed, 25);
}

#[test]
fn runtime_profiles_change_only_the_named_quota() {
    let program = compile_scalar("profile_i10b.n", "fn Main(){}").unwrap();
    let execution = crate::interp::run_with_limits(
        &program,
        Default::default(),
        crate::interp::ReducedExecutionProfile::Cells64,
    );
    assert_eq!(execution.outcome, Outcome::Unit);
    assert_eq!(execution.work_consumed, 2);
}

#[test]
fn final_unit_cell_denial_points_at_closing_brace_and_body_context() {
    let values = std::iter::repeat_n("0", 63).collect::<Vec<_>>().join(",");
    let source = format!("fn Main(){{let a=[{values}];}}");
    let program = compile_scalar("final_cells_i10b.n", &source).unwrap();
    let execution = crate::interp::run_with_limits(
        &program,
        Default::default(),
        crate::interp::ReducedExecutionProfile::Cells64,
    );
    let diagnostic = &execution.diagnostics[0];
    assert_eq!(diagnostic.code, "R0002");
    assert_eq!(
        diagnostic.data.get("runtime_budget_kind"),
        Some(&crate::diagnostic::DataValue::String(
            "live_value_cells".into()
        ))
    );
    assert_eq!(
        (
            diagnostic.primary_span.as_ref().unwrap().start,
            diagnostic.primary_span.as_ref().unwrap().end
        ),
        (144, 145)
    );
    assert_eq!(
        (
            diagnostic.related_spans[0].span.start,
            diagnostic.related_spans[0].span.end
        ),
        (9, 145)
    );
}

#[test]
fn reduced_frame_and_observation_profiles_report_exact_quota() {
    let mut functions = String::new();
    for index in 0..9 {
        functions.push_str(&format!("fn F{index}(){{F{}();}}", index + 1));
    }
    functions.push_str("fn F9(){}fn Main(){F0();}");
    let program = compile_scalar("frames_i10b.n", &functions).unwrap();
    let execution = crate::interp::run_with_limits(
        &program,
        Default::default(),
        crate::interp::ReducedExecutionProfile::Frames8,
    );
    assert_eq!(execution.diagnostics[0].code, "R0002");
    assert_eq!(
        execution.diagnostics[0].profile_id,
        "f0-0002-b-test-frames-8-v2"
    );
    assert_eq!(
        execution.diagnostics[0].data.get("runtime_budget_kind"),
        Some(&crate::diagnostic::DataValue::String("call_frames".into()))
    );

    let prints = "Print(0);".repeat(33);
    let source = format!("fn Main(){{{prints}}}");
    let program = compile_scalar("observations_i10b.n", &source).unwrap();
    let execution = crate::interp::run_with_limits(
        &program,
        Default::default(),
        crate::interp::ReducedExecutionProfile::Observations64,
    );
    assert_eq!(execution.diagnostics[0].code, "R0002");
    assert_eq!(
        execution.diagnostics[0].profile_id,
        "f0-0002-b-test-observations-64-v2"
    );
    assert_eq!(
        execution.diagnostics[0].data.get("runtime_budget_kind"),
        Some(&crate::diagnostic::DataValue::String(
            "observation_bytes".into()
        ))
    );
}

#[test]
fn final_unit_work_family_reaches_the_exact_boundary() {
    let prefix = "fn Main(){var i=0;while i<71427{i=i+1;";
    for (padding, expected) in [(7, 999_999), (8, 1_000_000)] {
        let source = format!("{prefix}{}{}{}", "}", "{}".repeat(padding), "}");
        let program = compile_scalar("final_work_i10b.n", &source).unwrap();
        assert_eq!(
            run_scalar(&program, Default::default()).work_consumed,
            expected
        );
    }
    let source = format!("{prefix}{}{}{}", "}", "{}".repeat(9), "}");
    let program = compile_scalar("final_work_i10b.n", &source).unwrap();
    let execution = run_scalar(&program, Default::default());
    assert_eq!(execution.work_consumed, 1_000_000);
    let diagnostic = &execution.diagnostics[0];
    assert_eq!(diagnostic.code, "R0001");
    assert_eq!(
        diagnostic.data.get("observed"),
        Some(&crate::diagnostic::DataValue::UInt(1_000_001))
    );
    assert_eq!(
        (
            diagnostic.primary_span.as_ref().unwrap().start,
            diagnostic.primary_span.as_ref().unwrap().end
        ),
        (57, 58)
    );
    assert_eq!(
        (
            diagnostic.related_spans[0].span.start,
            diagnostic.related_spans[0].span.end
        ),
        (9, 58)
    );
}

#[test]
fn verifier_checks_work_charge_metadata_independently() {
    let mut bad_span = compile_scalar("verify_charge_i10b.n", "fn Main(){let n=1+2;}")
        .unwrap()
        .functions()
        .function("Main")
        .unwrap()
        .clone();
    let constant = bad_span
        .block
        .instructions
        .iter_mut()
        .find(|instruction| matches!(instruction.op, Op::ConstI64(_)))
        .unwrap();
    constant.charges[0].origin.end = bad_span.source_text.len() as u64 + 1;
    bad_span.content_hash = super::model::content_hash(&bad_span);
    assert_eq!(
        super::verify::verify_function(&bad_span),
        Err("work charge origin")
    );

    let mut zero_ticks = compile_scalar("verify_charge_i10b.n", "fn Main(){let n=1;}")
        .unwrap()
        .functions()
        .function("Main")
        .unwrap()
        .clone();
    zero_ticks.block.instructions[0].charges[0].ticks = 0;
    zero_ticks.content_hash = super::model::content_hash(&zero_ticks);
    assert_eq!(
        super::verify::verify_function(&zero_ticks),
        Err("work charge origin")
    );

    let mut wrong_file = compile_scalar("verify_charge_i10b.n", "fn Main(){let n=1;}")
        .unwrap()
        .functions()
        .function("Main")
        .unwrap()
        .clone();
    wrong_file.block.instructions[0].charges[0].origin.file = "other.n".into();
    wrong_file.content_hash = super::model::content_hash(&wrong_file);
    assert_eq!(
        super::verify::verify_function(&wrong_file),
        Err("work charge origin")
    );

    let mut missing_binary = compile_scalar("verify_charge_i10b.n", "fn Main(){let n=1+2;}")
        .unwrap()
        .functions()
        .function("Main")
        .unwrap()
        .clone();
    missing_binary
        .block
        .instructions
        .iter_mut()
        .find(|instruction| matches!(instruction.op, Op::Binary { .. }))
        .unwrap()
        .charges
        .clear();
    missing_binary.content_hash = super::model::content_hash(&missing_binary);
    assert_eq!(super::verify::verify_function(&missing_binary), Ok(()));

    let source = "fn Inc(n:view I64)->Result<I64,I64>{return Err(9);}fn Main()->Result<(),I64>{let x=Inc(2)?;return Ok(());}";
    let mut missing_question = compile_scalar("verify_charge_i10b.n", source)
        .unwrap()
        .functions()
        .function("Main")
        .unwrap()
        .clone();
    missing_question
        .block
        .instructions
        .iter_mut()
        .find(|instruction| matches!(instruction.op, Op::BranchResult { .. }))
        .unwrap()
        .charges
        .clear();
    missing_question.content_hash = super::model::content_hash(&missing_question);
    assert_eq!(super::verify::verify_function(&missing_question), Ok(()));

    let mut administrative = compile_scalar(
        "verify_charge_i10b.n",
        "fn F(n:view I64){}fn Main(){let a=1;F(a);}",
    )
    .unwrap()
    .functions()
    .function("Main")
    .unwrap()
    .clone();
    let end_loan = administrative
        .block
        .instructions
        .iter_mut()
        .find(|instruction| matches!(instruction.op, Op::EndLoan(_)))
        .unwrap();
    end_loan.charges.push(super::model::WorkCharge {
        ticks: 1,
        origin: end_loan.span.clone(),
    });
    administrative.content_hash = super::model::content_hash(&administrative);
    assert_eq!(
        super::verify::verify_function(&administrative),
        Err("work charge rule")
    );

    let source = "// é\nfn Main(){let n=1;}";
    let mut split_character = compile_scalar("verify_charge_i10b.n", source)
        .unwrap()
        .functions()
        .function("Main")
        .unwrap()
        .clone();
    let inside = (source.find('é').unwrap() + 1) as u64;
    split_character.block.instructions[0].charges[0]
        .origin
        .start = inside;
    split_character.block.instructions[0].charges[0].origin.end = inside + 1;
    split_character.content_hash = super::model::content_hash(&split_character);
    assert_eq!(
        super::verify::verify_function(&split_character),
        Err("work charge origin")
    );
}

#[test]
fn content_hash_tracks_work_charge_ticks_and_source_origin() {
    let original = compile_scalar("hash_charge_i10b.n", "fn Main(){let n=1;}")
        .unwrap()
        .functions()
        .function("Main")
        .unwrap()
        .clone();
    let hash = original.content_hash;

    let mut changed_ticks = original.clone();
    changed_ticks.block.instructions[0].charges[0].ticks += 1;
    assert_ne!(super::model::content_hash(&changed_ticks), hash);

    let mut changed_origin = original;
    changed_origin.block.instructions[0].charges[0].origin.end += 1;
    assert_ne!(super::model::content_hash(&changed_origin), hash);
}

#[test]
fn content_hash_tracks_index_site_context_and_selector_spans() {
    let original = compile_scalar(
        "hash_index_site_i10b.n",
        "fn Main(){var a=[1];Print(a[0]);}",
    )
    .unwrap()
    .functions()
    .function("Main")
    .unwrap()
    .clone();
    let hash = original.content_hash;
    for change_context in [true, false] {
        let mut changed = original.clone();
        let instruction = std::iter::once(&mut changed.block)
            .chain(&mut changed.blocks)
            .flat_map(|block| &mut block.instructions)
            .find(|instruction| {
                matches!(
                    instruction.op,
                    Op::IndexValue { .. }
                        | Op::ReadIndex { .. }
                        | Op::BorrowIndex { .. }
                        | Op::CheckIndex { .. }
                )
            })
            .unwrap();
        match &mut instruction.op {
            Op::IndexValue { site, .. }
            | Op::ReadIndex { site, .. }
            | Op::BorrowIndex { site, .. }
            | Op::CheckIndex { site, .. } => {
                if change_context {
                    site.context.start += 1;
                } else {
                    site.selectors[0].start = site.context.start;
                    site.selectors[0].end = site.context.start + 1;
                }
            }
            _ => unreachable!(),
        }
        changed.content_hash = super::model::content_hash(&changed);
        assert_ne!(changed.content_hash, hash);
    }
}

#[test]
fn content_hash_tracks_terminator_work_charge_fields() {
    let original = compile_scalar("term_hash_i10b.n", "fn Main(){var i=0;while i<1{i=i+1;}}")
        .unwrap()
        .functions()
        .function("Main")
        .unwrap()
        .clone();
    let return_block = original
        .blocks
        .iter()
        .position(|block| matches!(block.terminator, Terminator::Return { .. }))
        .unwrap();
    let mut baseline = original.clone();
    let file = baseline.source_file.clone();
    if let Terminator::Return { charges, .. } = &mut baseline.blocks[return_block].terminator {
        charges.push(super::model::WorkCharge {
            ticks: 1,
            origin: crate::diagnostic::Span {
                file,
                start: 0,
                end: 1,
            },
        });
    }
    let hash = super::model::content_hash(&baseline);
    for field in 0..4 {
        let mut changed = baseline.clone();
        let charge = match &mut changed.blocks[return_block].terminator {
            Terminator::Return { charges, .. } => charges.last_mut().unwrap(),
            _ => panic!("expected return terminator"),
        };
        match field {
            0 => charge.ticks += 1,
            1 => charge.origin.file.push('x'),
            2 => charge.origin.start += 1,
            _ => charge.origin.end += 1,
        }
        assert_ne!(super::model::content_hash(&changed), hash);
    }
}

#[test]
fn verifier_checks_work_charge_metadata_on_cfg_terminators() {
    let source = "fn Main(){var i=0;while i<3{i=i+1;}}";
    let original = compile_scalar("term_verify_i10b.n", source)
        .unwrap()
        .functions()
        .function("Main")
        .unwrap()
        .clone();
    for block_index in 0..original.blocks.len() {
        let mut forged = original.clone();
        let file = forged.source_file.clone();
        match &mut forged.blocks[block_index].terminator {
            Terminator::Jump { charges, .. }
            | Terminator::BranchBool { charges, .. }
            | Terminator::Return { charges, .. } => {
                charges.push(super::model::WorkCharge {
                    ticks: 0,
                    origin: crate::diagnostic::Span {
                        file,
                        start: 0,
                        end: 0,
                    },
                });
            }
        }
        forged.content_hash = super::model::content_hash(&forged);
        assert_eq!(
            super::verify::verify_function(&forged),
            Err("work charge origin")
        );
    }
    let mut single = compile_scalar("return_verify_i10b.n", "fn Main(){}")
        .unwrap()
        .functions()
        .function("Main")
        .unwrap()
        .clone();
    let file = single.source_file.clone();
    if let Terminator::Return { charges, .. } = &mut single.block.terminator {
        charges.push(super::model::WorkCharge {
            ticks: 1,
            origin: crate::diagnostic::Span {
                file,
                start: 0,
                end: 0,
            },
        });
    }
    single.content_hash = super::model::content_hash(&single);
    assert_eq!(
        super::verify::verify_function(&single),
        Err("work charge origin")
    );
}

#[test]
fn zero_length_charge_origins_are_rejected_on_instructions_and_terminators() {
    fn invalid(mut function: super::model::Function, target: &str) {
        let file = function.source_file.clone();
        let charge = super::model::WorkCharge {
            ticks: 1,
            origin: crate::diagnostic::Span {
                file,
                start: 0,
                end: 0,
            },
        };
        match target {
            "instruction" => function.block.instructions[0].charges.push(charge),
            "jump" => {
                let block = std::iter::once(&mut function.block)
                    .chain(&mut function.blocks)
                    .find(|b| matches!(b.terminator, Terminator::Jump { .. }))
                    .unwrap();
                if let Terminator::Jump { charges, .. } = &mut block.terminator {
                    charges.push(charge);
                }
            }
            "branch" => {
                let block = std::iter::once(&mut function.block)
                    .chain(&mut function.blocks)
                    .find(|b| matches!(b.terminator, Terminator::BranchBool { .. }))
                    .unwrap();
                if let Terminator::BranchBool { charges, .. } = &mut block.terminator {
                    charges.push(charge);
                }
            }
            "return" => {
                let block = std::iter::once(&mut function.block)
                    .chain(&mut function.blocks)
                    .find(|b| matches!(b.terminator, Terminator::Return { .. }))
                    .unwrap();
                if let Terminator::Return { charges, .. } = &mut block.terminator {
                    charges.push(charge);
                }
            }
            _ => unreachable!(),
        }
        function.content_hash = super::model::content_hash(&function);
        assert_eq!(
            super::verify::verify_function(&function),
            Err("work charge origin")
        );
    }
    invalid(
        compile_scalar("m02_i10b.n", "fn Main(){let n=1;}")
            .unwrap()
            .functions()
            .function("Main")
            .unwrap()
            .clone(),
        "instruction",
    );
    let cfg = compile_scalar("m02_i10b.n", "fn Main(){if true {}else{}}")
        .unwrap()
        .functions()
        .function("Main")
        .unwrap()
        .clone();
    invalid(cfg.clone(), "jump");
    invalid(cfg.clone(), "branch");
    invalid(
        compile_scalar("m02_i10b.n", "fn Main(){}")
            .unwrap()
            .functions()
            .function("Main")
            .unwrap()
            .clone(),
        "return",
    );
}

#[test]
fn jump_and_branch_charge_origins_are_independently_verified() {
    let original = compile_scalar("m06_i10b.n", "fn Main(){var i=0;while i<3{i=i+1;}}")
        .unwrap()
        .functions()
        .function("Main")
        .unwrap()
        .clone();
    for target in ["jump", "branch"] {
        for malformed in 0..4 {
            let mut forged = original.clone();
            let file = forged.source_file.clone();
            let source_len = forged.source_text.len() as u64;
            let block = std::iter::once(&mut forged.block)
                .chain(&mut forged.blocks)
                .find(|block| {
                    if target == "jump" {
                        matches!(block.terminator, Terminator::Jump { .. })
                    } else {
                        matches!(block.terminator, Terminator::BranchBool { .. })
                    }
                })
                .unwrap();
            let charges = match &mut block.terminator {
                Terminator::Jump { charges, .. } if target == "jump" => charges,
                Terminator::BranchBool { charges, .. } if target == "branch" => charges,
                _ => unreachable!(),
            };
            let mut charge = super::model::WorkCharge {
                ticks: 1,
                origin: crate::diagnostic::Span {
                    file,
                    start: 0,
                    end: 1,
                },
            };
            match malformed {
                0 => charge.ticks = 0,
                1 => charge.origin.file = "other.n".into(),
                2 => charge.origin.end = charge.origin.start,
                _ => charge.origin.end = source_len + 1,
            }
            charges.push(charge);
            forged.content_hash = super::model::content_hash(&forged);
            assert_eq!(
                super::verify::verify_function(&forged),
                Err("work charge origin"),
                "{target}, malformed={malformed}"
            );
        }
    }
}

#[test]
fn return_charge_origins_are_checked_in_single_and_multiblock_functions() {
    for source in ["fn Main(){}", "fn Main(){if true {}else{}}"] {
        let mut forged = compile_scalar("m07_i10b.n", source)
            .unwrap()
            .functions()
            .function("Main")
            .unwrap()
            .clone();
        let file = forged.source_file.clone();
        let block = std::iter::once(&mut forged.block)
            .chain(&mut forged.blocks)
            .find(|block| matches!(block.terminator, Terminator::Return { .. }))
            .unwrap();
        if let Terminator::Return { charges, .. } = &mut block.terminator {
            charges.push(super::model::WorkCharge {
                ticks: 0,
                origin: crate::diagnostic::Span {
                    file,
                    start: 0,
                    end: 1,
                },
            });
        }
        forged.content_hash = super::model::content_hash(&forged);
        assert_eq!(
            super::verify::verify_function(&forged),
            Err("work charge origin"),
            "{source}"
        );
    }
}

#[test]
fn verifier_allows_prefixed_charge_ownership_after_rehash() {
    let cases: &[WorkInstructionCase<'_>] = &[
        ("fn Main(){let n=1;}", "Main", |op| {
            matches!(op, Op::ConstI64(_))
        }),
        ("fn Main(){if true {let n=1;}else{}}", "Main", |op| {
            matches!(op, Op::ConstBool(_))
        }),
        ("fn Main(){}", "Main", |op| matches!(op, Op::ConstUnit)),
        ("fn Main(){let a=1;let b=a;}", "Main", |op| {
            matches!(op, Op::CopyValue(_))
        }),
        ("fn Main(){let n=1+2;}", "Main", |op| {
            matches!(op, Op::Binary { .. })
        }),
        ("fn Main(){let a=[1];}", "Main", |op| {
            matches!(op, Op::BuildArray(_))
        }),
        (
            "fn Make()->Result<I64,I64>{return Ok(1);}fn Main(){}",
            "Make",
            |op| matches!(op, Op::BuildResult { .. }),
        ),
        (
            "fn Inc(n:view I64)->Result<I64,I64>{return Err(9);}fn Prop()->Result<(),I64>{let x=Inc(2)?;return Ok(());}fn Main(){}",
            "Prop",
            |op| matches!(op, Op::BranchResult { .. }),
        ),
        ("fn F(){}fn Main(){F();}", "Main", |op| {
            matches!(op, Op::CallDirect { .. })
        }),
        ("fn Main(){Print(1);}", "Main", |op| {
            matches!(op, Op::CallIntrinsic { .. })
        }),
    ];

    for (source, function_name, matches_op) in cases {
        let mut function = compile_scalar("verify_charge_i10b.n", source)
            .unwrap()
            .functions()
            .function(function_name)
            .unwrap()
            .clone();
        let instruction = std::iter::once(&mut function.block)
            .chain(&mut function.blocks)
            .flat_map(|block| &mut block.instructions)
            .find(|instruction| matches_op(&instruction.op))
            .unwrap_or_else(|| panic!("missing target instruction in {source}"));
        instruction.charges.clear();
        function.content_hash = super::model::content_hash(&function);
        assert_eq!(
            super::verify::verify_function(&function),
            Ok(()),
            "source {source}"
        );
    }
}

#[test]
fn verifier_rejects_charges_on_each_administrative_instruction() {
    let cases: &[AdministrativeInstructionCase<'_>] = &[
        ("fn F(n:view I64){}fn Main(){let n=1;F(n);}", |op| {
            matches!(op, Op::EndLoan(_))
        }),
        ("fn Main(){Print(1);}", |op| matches!(op, Op::DropValue(_))),
        ("fn Main(){{let r=MockOpen(1);}}", |op| {
            matches!(op, Op::DropLocal(_))
        }),
    ];

    for (source, matches_op) in cases {
        let mut function = compile_scalar("verify_charge_i10b.n", source)
            .unwrap()
            .functions()
            .function("Main")
            .unwrap()
            .clone();
        let instruction = std::iter::once(&mut function.block)
            .chain(&mut function.blocks)
            .flat_map(|block| &mut block.instructions)
            .find(|instruction| matches_op(&instruction.op))
            .unwrap_or_else(|| panic!("missing target instruction in {source}"));
        instruction.charges.push(super::model::WorkCharge {
            ticks: 1,
            origin: instruction.span.clone(),
        });
        function.content_hash = super::model::content_hash(&function);
        assert_eq!(
            super::verify::verify_function(&function),
            Err("work charge rule"),
            "source {source}"
        );
    }
}

trait TerminatorCharges {
    fn terminator_charges(&self) -> &[super::model::WorkCharge];
}
impl TerminatorCharges for Terminator {
    fn terminator_charges(&self) -> &[super::model::WorkCharge] {
        match self {
            Terminator::Return { charges, .. }
            | Terminator::Jump { charges, .. }
            | Terminator::BranchBool { charges, .. } => charges,
        }
    }
}
