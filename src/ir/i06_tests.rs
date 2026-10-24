use super::{CompileError, check_scalar, compile_scalar};
use crate::interp::{Outcome, run_scalar};

#[test]
fn inout_reborrow_returns_modified_value() {
    let source = "fn Inc(x: edit I64) {\n    x = x + 1;\n}\nfn Dos(x: edit I64) {\n    Inc(edit x);\n    Inc(edit x);\n}\nfn Main() {\n    var x = 0;\n    Dos(edit x);\n    AssertEq(x, 2);\n}\n";
    let result = run_scalar(&compile_scalar("i06.n", source).unwrap());
    assert_eq!(result.outcome, Outcome::Unit);
    assert!(result.diagnostics.is_empty());
}

#[test]
fn published_conflicts_have_exact_identity_data() {
    use crate::diagnostic::{DataValue, EntityKind, RelatedRole};
    use std::collections::BTreeMap;
    let entity = |kind: EntityKind, id: &str| {
        DataValue::Object(BTreeMap::from([
            ("kind".into(), DataValue::String(kind.as_str().into())),
            ("id".into(), DataValue::String(id.into())),
        ]))
    };
    let cases = [
        (
            "fn F(x:edit I64,y:edit I64){} fn Main(){var x=1;F(edit x,edit x);}",
            "overlapping_mutable_borrows",
            "binding:44",
            "loan:50:0",
            62,
            63,
            50,
            56,
        ),
        (
            "fn F(x:view I64,y:edit I64){} fn Main(){var x=1;F(x,edit x);}",
            "active_borrow_conflict",
            "binding:44",
            "loan:50:0",
            57,
            58,
            50,
            51,
        ),
        (
            "fn F(x:edit I64,y:edit I64){} fn Main(){var a=[1,2];F(edit a[0],edit a[1]);}",
            "overlapping_mutable_borrows",
            "binding:44",
            "loan:54:0",
            69,
            73,
            54,
            63,
        ),
    ];
    for (source, subcause, binding, loan, start, end, origin_start, origin_end) in cases {
        let CompileError::Rejected(errors) = compile_scalar("i06.n", source).unwrap_err() else {
            panic!("missing source diagnostic: {source}")
        };
        assert_eq!(errors.len(), 1, "{source}");
        let d = &errors[0];
        d.validate("i06.n", source).unwrap();
        assert_eq!(d.code, "NX-E1105");
        assert_eq!(d.subcause.as_deref(), Some(subcause));
        assert_eq!(
            d.primary_span.as_ref().map(|s| (s.start, s.end)),
            Some((start, end))
        );
        assert_eq!(d.related_spans.len(), 2);
        assert_eq!(d.related_spans[1].role, RelatedRole::Origin);
        assert_eq!(
            (d.related_spans[1].span.start, d.related_spans[1].span.end),
            (origin_start, origin_end)
        );
        assert_eq!(d.cause_chain.len(), 1);
        assert_eq!(d.cause_chain[0].entity.as_ref().unwrap().id, loan);
        assert_eq!(
            d.data,
            BTreeMap::from([
                ("origin".into(), entity(EntityKind::Binding, binding)),
                ("binding".into(), entity(EntityKind::Binding, binding)),
                ("loan".into(), entity(EntityKind::Loan, loan)),
                (
                    "active_permission".into(),
                    DataValue::String(
                        if subcause == "active_borrow_conflict" {
                            "read"
                        } else {
                            "inout"
                        }
                        .into()
                    )
                ),
                (
                    "requested_access".into(),
                    DataValue::String("borrow_inout".into())
                ),
            ])
        );
    }
}

#[test]
fn reading_during_argument_loans_reports_source_conflict() {
    use crate::diagnostic::{CauseKind, CauseOperation, DataValue, Operation, Stage};
    let cases = [
        (
            "fn F(x: edit I64, y: view I64) {}\nfn Main() {\n    var x = 1;\n    F(edit x, x + 0);\n}\n",
            "x + 0",
            "x",
            "read",
        ),
        (
            "fn F(x: edit I64, y: view I64) {}\nfn Main() {\n    var a = [1];\n    F(edit a[0], a[0] + 0);\n}\n",
            "a[0] + 0",
            "a[0]",
            "read",
        ),
        (
            "fn F(x: view I64, y: edit I64) {}\nfn Main() {\n    var a = [1];\n    F(a[0], edit a[0]);\n}\n",
            "edit a[0]",
            "a[0]",
            "borrow_inout",
        ),
        (
            "fn F(x: edit I64, y: view I64) {}\nfn Main() {\n    var x = 0;\n    var a = [1];\n    F(edit x, a[x] + 0);\n}\n",
            "a[x] + 0",
            "x",
            "read",
        ),
        (
            "fn F(x: edit I64, y: view Bool) {}\nfn Main() {\n    var x = 0;\n    F(edit x, x < 1);\n}\n",
            "x < 1",
            "x",
            "read",
        ),
    ];
    for (source, suffix, place, access) in cases {
        let CompileError::Rejected(errors) = compile_scalar("conflict.n", source).unwrap_err()
        else {
            panic!("missing source conflict: {source}");
        };
        assert_eq!(errors.len(), 1, "{source}");
        let d = &errors[0];
        d.validate("conflict.n", source).unwrap();
        assert_eq!(d.code, "NX-E1105", "{source}");
        assert_eq!(
            d.subcause.as_deref(),
            Some("active_borrow_conflict"),
            "{source}"
        );
        assert_eq!(d.stage, Stage::Ownership);
        assert_eq!(d.operation, Operation::OwnershipAccess);
        let start = source.rfind(suffix).unwrap() + suffix.find(place).unwrap();
        assert_eq!(
            d.primary_span.as_ref().map(|s| (s.start, s.end)),
            Some((start as u64, (start + place.len()) as u64)),
            "{source}"
        );
        assert_eq!(d.data["requested_access"], DataValue::String(access.into()));
        assert_eq!(d.cause_chain[0].kind, CauseKind::Borrow);
        assert_eq!(
            d.cause_chain[0].operation,
            if access == "read" {
                CauseOperation::BorrowInout
            } else {
                CauseOperation::BorrowRead
            }
        );
    }
}

#[test]
fn indexed_conflict_primary_covers_complete_place_across_line_endings() {
    // ADR F0-0002 §14.2 P07 marks `⟦a[1]⟧`, not only its root.
    for prefix in ["", "// café\n", "// café\r\n"] {
        let source = format!(
            "{prefix}fn F(x:edit I64,y:edit I64){{}}\nfn Main(){{var a=[1,2];F(edit a[0],edit a[1]);}}"
        );
        let CompileError::Rejected(errors) = compile_scalar("p07.n", &source).unwrap_err() else {
            panic!("missing P07 conflict")
        };
        assert_eq!(errors.len(), 1);
        let d = &errors[0];
        d.validate("p07.n", &source).unwrap();
        assert_eq!(d.code, "NX-E1105");
        assert_eq!(d.subcause.as_deref(), Some("overlapping_mutable_borrows"));
        let start = source.rfind("a[1]").unwrap() as u64;
        assert_eq!(
            d.primary_span.as_ref().map(|s| (s.start, s.end)),
            Some((start, start + 4))
        );
        let first = source.find("edit a[0]").unwrap() as u64;
        assert_eq!(
            d.cause_chain[0].span.as_ref().map(|s| (s.start, s.end)),
            Some((first, first + "edit a[0]".len() as u64))
        );
    }
}

#[test]
fn named_loan_identity_starts_at_argument_label() {
    use crate::diagnostic::DataValue;
    let source = "fn F(x: edit I64, y: edit I64) {}\nfn Main() {\n    var a = [1, 2];\n    F(x: edit a[0], y: edit a[1]);\n}\n";
    let CompileError::Rejected(errors) = compile_scalar("label.n", source).unwrap_err() else {
        panic!("missing named loan conflict")
    };
    assert_eq!(errors.len(), 1);
    let d = &errors[0];
    d.validate("label.n", source).unwrap();
    let first = source.find("x: edit a[0]").unwrap() as u64;
    let second = source.find("y: edit a[1]").unwrap() as u64;
    let place = source.rfind("a[1]").unwrap() as u64;
    assert_eq!(
        d.primary_span.as_ref().map(|s| (s.start, s.end)),
        Some((place, place + 4))
    );
    assert_eq!(
        d.cause_chain[0].span.as_ref().map(|s| (s.start, s.end)),
        Some((first, first + "x: edit a[0]".len() as u64))
    );
    let DataValue::Object(loan) = &d.data["loan"] else {
        panic!("loan data")
    };
    assert_eq!(loan["id"], DataValue::String(format!("loan:{first}:0")));
    assert_eq!(d.related_spans[1].span.start, first);
    assert!(second > first);
}

#[test]
fn named_exclusive_loan_keeps_full_argument_origin_in_verified_ir() {
    use super::model::Op;
    let source = "fn F(x: edit I64) {}\nfn Main() {\n    var a = [1];\n    F(x: edit a[0]);\n}\n";
    let program = compile_scalar("named.n", source).unwrap();
    let function = program.functions().function("Main").unwrap();
    let first = source.find("x: edit a[0]").unwrap() as u64;
    let loan = function
        .block
        .instructions
        .iter()
        .find_map(|ins| {
            if let Op::BeginLoan {
                loan, origin_span, ..
            } = &ins.op
            {
                Some((loan.0, origin_span.start, origin_span.end))
            } else {
                None
            }
        })
        .unwrap();
    assert_eq!(
        loan,
        (first as usize, first, first + "x: edit a[0]".len() as u64)
    );
    assert_eq!(run_scalar(&program).outcome, Outcome::Unit);
}

#[test]
fn two_read_loans_of_the_same_root_remain_compatible() {
    let source = "fn F(x: view Array<I64>, y: view Array<I64>) {}\nfn Main() {\n    var a = [1];\n    F(a, a);\n}\n";
    let result = run_scalar(&compile_scalar("reads.n", source).unwrap());
    assert_eq!(result.outcome, Outcome::Unit);
    assert!(result.diagnostics.is_empty());
}

#[test]
fn indexed_assignment_rechecks_owner_consumed_by_rhs() {
    let source = "fn Take(x: move Array<I64>) -> I64 {\n    return 1;\n}\nfn Main() {\n    var a = [0];\n    a[0] = Take(move a);\n}\n";
    let CompileError::Rejected(errors) = compile_scalar("consume.n", source).unwrap_err() else {
        panic!("missing unavailable destination")
    };
    assert_eq!(errors.len(), 1);
    let d = &errors[0];
    d.validate("consume.n", source).unwrap();
    assert_eq!(d.code, "NX-E0202");
    assert_eq!(d.subcause.as_deref(), Some("definitely_consumed"));
    let destination = source.rfind("a[0] =").unwrap() as u64;
    assert_eq!(
        d.primary_span.as_ref().map(|s| (s.start, s.end)),
        Some((destination, destination + 1))
    );
}

#[test]
fn valid_pairs_and_permissions() {
    let valid = [
        "fn F(x: edit I64) {}\nfn Main() {\n    var x = 1;\n    F(edit x);\n}\n",
        "fn F(x: edit I64) {}\nfn Main() {\n    var x = 1;\n    var y = 2;\n    F(edit y);\n}\n",
        "fn F(x: view I64, y: edit I64) {}\nfn Main() {\n    var x = 1;\n    F(x + 0, edit x);\n}\n",
        "fn F(x: edit I64, y: edit I64) {}\nfn Main() {\n    var a = [1, 2];\n    var b = [3, 4];\n    F(edit a[0], edit b[1]);\n}\n",
        "fn F(x: view I64, y: view I64) {}\nfn Main() {\n    var x = 1;\n    F(x, x);\n}\n",
        "fn F(x: view I64, y: view I64) {}\nfn Main() {\n    var x = 1;\n    F(x, x + 0);\n}\n",
        "fn F(x: edit MockResource) {\n    x = MockOpen(2);\n}\nfn Main() {\n    var r = MockOpen(1);\n    F(edit r);\n    Inspect(r);\n}\n",
        "fn Copy(a: view Array<I64>) -> Array<I64> {\n    return a;\n}\nfn Main() {\n    var a = [1];\n    let b = Copy(a);\n    a[0] = 2;\n    AssertEq(b, [1]);\n}\n",
    ];
    for source in valid {
        let result = run_scalar(
            &compile_scalar("i06.n", source).unwrap_or_else(|e| panic!("{source}: {e:?}")),
        );
        assert_eq!(
            result.outcome,
            Outcome::Unit,
            "{source}: {:?}",
            result.diagnostics
        );
    }
    for (source, subcause) in [
        (
            "fn F(x: view MockResource) {\n    Drop(x);\n}\nfn Main() {}\n",
            "permission_escalation",
        ),
        (
            "fn F(x: edit MockResource) {\n    Drop(x);\n}\nfn Main() {}\n",
            "permission_escalation",
        ),
        (
            "fn F(x: edit MockResource) -> MockResource {\n    return move x;\n}\nfn Main() {}\n",
            "permission_escalation",
        ),
        (
            "fn F(s: view MockResource, r: move MockResource) {}\nfn Main() {\n    let r = MockOpen(1);\n    F(r, move r);\n}\n",
            "active_borrow_conflict",
        ),
        (
            "fn F(r: view MockResource, n: view ()) {}\nfn Main() {\n    let r = MockOpen(1);\n    F(r, Drop(r));\n}\n",
            "active_borrow_conflict",
        ),
    ] {
        let CompileError::Rejected(errors) = compile_scalar("i06.n", source).unwrap_err() else {
            panic!("missing diagnostic: {source}")
        };
        assert_eq!(errors[0].subcause.as_deref(), Some(subcause), "{source}");
        for e in errors {
            e.validate("i06.n", source)
                .unwrap_or_else(|reason| panic!("{source}: {reason:?}: {e:?}"));
        }
    }
}

#[test]
fn indexed_destination_rechecks_after_rhs_call_modifies_root() {
    let source = "fn Resize(a: edit Array<I64>) -> I64 {\n    a = [7];\n    return 9;\n}\nfn Main() {\n    var a = [1, 2];\n    a[1] = Resize(edit a);\n}\n";
    let result = run_scalar(&compile_scalar("i06.n", source).unwrap());
    assert_eq!(result.outcome, Outcome::SimulatorFault);
    assert_eq!(result.diagnostics[0].code, "R0102");
    let source = "fn Resize(a: edit Array<I64>) -> I64 {\n    a = [7, 8, 9];\n    return 4;\n}\nfn Main() {\n    var a = [1];\n    a[1] = Resize(edit a);\n    AssertEq(a, [7, 4, 9]);\n}\n";
    let result = run_scalar(&compile_scalar("i06.n", source).unwrap());
    assert_eq!(result.outcome, Outcome::Unit, "{:?}", result.diagnostics);
}

#[test]
fn syntax_mode_and_immutable_place_pairs() {
    use crate::diagnostic::{Operation, Stage};
    let cases = [
        (
            "fn F(x: edit I64) {}\nfn Main() {\n    var x = 1;\n    F(x);\n}\n",
            "NX-E0111",
            Some("argument_mode_mismatch"),
            Stage::Arguments,
            Operation::CallBind,
        ),
        (
            "fn F(x: edit I64) {}\nfn Main() {\n    let x = 1;\n    F(edit x);\n}\n",
            "NX-E0105",
            None,
            Stage::Ownership,
            Operation::PlaceWrite,
        ),
        (
            "fn F(x:edit I64){} fn Main(){F(edit 1);}",
            "NX-E0002",
            None,
            Stage::Parse,
            Operation::SyntaxParse,
        ),
    ];
    for (source, code, subcause, stage, operation) in cases {
        let start = match code {
            "NX-E0111" => source.rfind("F(x)").unwrap() + 2,
            "NX-E0105" => source.rfind("edit x").unwrap() + "edit ".len(),
            "NX-E0002" => source.rfind('1').unwrap(),
            _ => unreachable!(),
        } as u64;
        let CompileError::Rejected(errors) = compile_scalar("i06.n", source).unwrap_err() else {
            panic!("missing {code}: {source}")
        };
        assert_eq!(errors.len(), 1, "{source}");
        let d = &errors[0];
        d.validate("i06.n", source).unwrap();
        assert_eq!(d.code, code);
        assert_eq!(d.subcause.as_deref(), subcause);
        assert_eq!(d.stage, stage);
        assert_eq!(d.operation, operation);
        assert_eq!(
            d.primary_span.as_ref().map(|s| (s.start, s.end)),
            Some((start, start + 1))
        );
    }
}

#[test]
fn verifier_rejects_forged_loan_permissions_and_lifecycle() {
    use super::model::{self, LoanId, LoanPermission, Op};
    use super::verify::verify_function;
    let p = compile_scalar(
        "i06.n",
        "fn F(x: edit I64) {}\nfn Main() {\n    var x = 1;\n    F(edit x);\n}\n",
    )
    .unwrap();
    let base = p.functions().function("Main").unwrap().clone();
    let verify = |mut f: model::Function| {
        f.content_hash = model::content_hash(&f);
        verify_function(&f)
    };
    let loan_pos = base
        .block
        .instructions
        .iter()
        .position(|i| matches!(i.op, Op::BeginLoan { .. }))
        .unwrap();
    let call_pos = base
        .block
        .instructions
        .iter()
        .position(|i| matches!(i.op, Op::CallDirect { .. }))
        .unwrap();
    let end_pos = base
        .block
        .instructions
        .iter()
        .position(|i| matches!(i.op, Op::EndLoan(_)))
        .unwrap();

    let mut f = base.clone();
    if let Op::BeginLoan { permission, .. } = &mut f.block.instructions[loan_pos].op {
        *permission = LoanPermission::Read;
    }
    assert_eq!(verify(f), Err("edit argument permission"));

    let mut f = base.clone();
    if let Op::BeginLoan { parent, root, .. } = &mut f.block.instructions[loan_pos].op {
        *parent = Some(*root);
    }
    assert_eq!(verify(f), Err("loan parent"));

    let mut f = base.clone();
    if let Op::BeginLoan { loan, .. } = &mut f.block.instructions[loan_pos].op {
        *loan = LoanId(1000);
    }
    assert_eq!(verify(f), Err("loan identity"));

    let mut f = base.clone();
    f.block.instructions.remove(end_pos);
    for (position, ins) in f.block.instructions.iter_mut().enumerate() {
        ins.origin.0 = position;
    }
    assert_eq!(verify(f), Err("loan end missing"));

    let mut f = base.clone();
    if let Op::EndLoan(loan) = &mut f.block.instructions[end_pos].op {
        *loan = LoanId(999);
    }
    assert_eq!(verify(f), Err("unmatched loan end"));

    let mut f = base.clone();
    if let Op::CallDirect { args, .. } = &mut f.block.instructions[call_pos].op {
        args.clear();
    }
    assert_eq!(verify(f), Err("call arity"));

    let mut f = base.clone();
    let root = match &f.block.instructions[loan_pos].op {
        Op::BeginLoan { root, .. } => *root,
        _ => unreachable!(),
    };
    f.block.instructions[call_pos].op = Op::TransferValue(root);
    assert_eq!(verify(f), Err("transfer while borrowed"));

    let mut f = base.clone();
    let borrowed = f.block.instructions[loan_pos].output.unwrap();
    f.block.instructions[call_pos].op = Op::WritePlace {
        local: root,
        value: borrowed,
    };
    f.block.instructions[call_pos].access = model::Access::Write;
    f.block.instructions[call_pos].output = None;
    f.block.instructions[call_pos].ty = None;
    assert_eq!(verify(f), Err("write while borrowed"));

    let mut f = base;
    let model::Terminator::Return { span, .. } = &f.block.terminator else {
        panic!("return terminator")
    };
    f.block.terminator = model::Terminator::Return {
        value: Some(borrowed),
        span: span.clone(),
    };
    assert_eq!(verify(f), Err("borrowed return"));
}

#[test]
fn independent_argument_errors_are_collected_without_dependent_cascades() {
    let source = "fn F(a: move MockResource, b: move MockResource) {}\nfn Main() {\n    let r = MockOpen(1);\n    let s = MockOpen(2);\n    F(r, s);\n}\n";
    let CompileError::Rejected(errors) = compile_scalar("i06.n", source).unwrap_err() else {
        panic!("missing copy diagnostics")
    };
    assert_eq!(errors.len(), 2);
    assert!(errors.iter().all(|e| e.code == "NX-E0201"));
    assert!(
        errors[0].primary_span.as_ref().unwrap().start
            < errors[1].primary_span.as_ref().unwrap().start
    );
    for e in errors {
        e.validate("i06.n", source).unwrap();
    }

    let source = "fn F(a: move MockResource, b: move MockResource) {}\nfn Main() {\n    let r = MockOpen(1);\n    let s = r;\n    F(s, s);\n}\n";
    let CompileError::Rejected(errors) = compile_scalar("i06.n", source).unwrap_err() else {
        panic!("missing original copy diagnostic")
    };
    assert_eq!(errors.len(), 1);
    assert_eq!(errors[0].code, "NX-E0201");
}

#[test]
fn loan_hash_ignores_source_offsets_and_tracks_permission_and_callee_signature() {
    let a = compile_scalar(
        "i06.n",
        "fn F(x: edit I64) {\n    x = x + 1;\n}\nfn Main() {\n    var x = 1;\n    F(edit x);\n}\n",
    )
    .unwrap();
    let b = compile_scalar(
        "i06.n",
        "fn F(x: edit I64) {\n    x = x + 1;\n}\nfn Main() {\n    var x = 1;\n    F(edit x);\n}\n",
    )
    .unwrap();
    assert_eq!(
        a.functions().function("Main").unwrap().content_hash,
        b.functions().function("Main").unwrap().content_hash
    );
    let c = compile_scalar(
        "i06.n",
        "fn F(x: edit I64) {\n    x = x + 2;\n}\nfn Main() {\n    var x = 1;\n    F(edit x);\n}\n",
    )
    .unwrap();
    assert_eq!(
        a.functions().function("Main").unwrap().content_hash,
        c.functions().function("Main").unwrap().content_hash
    );
    assert_ne!(
        a.functions().function("F").unwrap().content_hash,
        c.functions().function("F").unwrap().content_hash
    );
    let d = compile_scalar(
        "i06.n",
        "fn F(y: edit I64) {\n    y = y + 1;\n}\nfn Main() {\n    var x = 1;\n    F(edit x);\n}\n",
    )
    .unwrap();
    assert_ne!(
        a.functions().function("Main").unwrap().content_hash,
        d.functions().function("Main").unwrap().content_hash
    );
}

#[test]
fn development_substitution_executes_new_inout_body() {
    let mut base = compile_scalar(
        "i06.n",
        "fn Inc(x: edit I64) {\n    x = x + 1;\n}\nfn Main() {\n    var x = 0;\n    Inc(edit x);\n    AssertEq(x, 2);\n}\n",
    )
    .unwrap();
    let donor = compile_scalar(
        "i06.n",
        "fn Inc(x: edit I64) {\n    x = x + 2;\n}\nfn Main() {\n    var x = 0;\n    Inc(edit x);\n    AssertEq(x, 2);\n}\n",
    )
    .unwrap();
    assert_eq!(run_scalar(&base).outcome, Outcome::SimulatorFault);
    base.substitute_for_development("Inc", &donor).unwrap();
    assert_eq!(run_scalar(&base).outcome, Outcome::Unit);
}

#[test]
fn inout_resource_replacement_preserves_single_owner_and_cleanup_order() {
    let source = "fn Replace(r: edit MockResource) {\n    r = MockOpen(2);\n    Inspect(r);\n}\nfn Main() {\n    var r = MockOpen(1);\n    Replace(edit r);\n    Inspect(r);\n}\n";
    let result = run_scalar(&compile_scalar("i06.n", source).unwrap());
    assert_eq!(result.outcome, Outcome::Unit);
    assert_eq!(
        result.observations.events(),
        [
            "open:0:1",
            "open:1:2",
            "close:0:1",
            "inspect:1:2",
            "inspect:1:2",
            "close:1:2",
        ]
    );
}

#[test]
fn inout_owner_is_restored_on_fault_and_frame_limit() {
    let source = "fn Change(r: edit MockResource) {\n    r = MockOpen(2);\n    Fail();\n}\nfn Main() {\n    var r = MockOpen(1);\n    Change(edit r);\n}\n";
    let result = run_scalar(&compile_scalar("i06.n", source).unwrap());
    assert_eq!(result.outcome, Outcome::SimulatorFault);
    assert_eq!(result.diagnostics[0].code, "R0301");
    assert_eq!(
        result.observations.events(),
        ["open:0:1", "open:1:2", "close:0:1", "close:1:2"]
    );

    let source = "fn Recur(r: edit MockResource) {\n    Recur(edit r);\n}\nfn Main() {\n    var r = MockOpen(1);\n    Recur(edit r);\n}\n";
    let result = run_scalar(&compile_scalar("i06.n", source).unwrap());
    assert_eq!(result.outcome, Outcome::SimulatorFault);
    assert_eq!(result.diagnostics[0].code, "R0002");
    assert_eq!(result.observations.events(), ["open:0:1", "close:0:1"]);
}

#[test]
fn generated_inout_and_copy_programs_match_independent_integer_model() {
    let prefix = "fn Inc(x:edit I64){x=x+1;} fn Twice(x:edit I64){Inc(edit x);Inc(edit x);} fn Retinc(x:edit I64)->I64{x=x+1;return x;} fn Copy(a:view Array<I64>)->Array<I64>{return a;} fn Main(){var x=0;var a=[0,0];";
    let mut seed = 0x5129_1234u64;
    for case in 0..8_000 {
        let mut source = String::from(prefix);
        let (mut x, mut a) = (0i64, [0i64, 0i64]);
        for step in 0..8 {
            seed = seed
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            match (seed >> 33) % 6 {
                0 => {
                    source.push_str("Inc(edit x);");
                    x += 1;
                }
                1 => {
                    source.push_str("Inc(edit a[0]);");
                    a[0] += 1;
                }
                2 => {
                    source.push_str("Inc(edit a[1]);");
                    a[1] += 1;
                }
                3 => {
                    source.push_str("Twice(edit x);");
                    x += 2;
                }
                4 => {
                    source.push_str("a[0]=Retinc(edit a[1]);");
                    a[1] += 1;
                    a[0] = a[1];
                }
                _ => {
                    source.push_str(&format!(
                        "let b{step}=Copy(a);a[0]=a[0]+1;AssertEq(b{step},[{},{}]);",
                        a[0], a[1]
                    ));
                    a[0] += 1;
                }
            }
        }
        source.push_str(&format!(
            "AssertEq(x,{x});AssertEq(a,[{},{}]);}}",
            a[0], a[1]
        ));
        let compiled = compile_scalar("model.n", &source)
            .unwrap_or_else(|e| panic!("case {case}: {e:?}: {source}"));
        let result = run_scalar(&compiled);
        assert_eq!(
            result.outcome,
            Outcome::Unit,
            "case {case}: {source}: {:?}",
            result.diagnostics
        );
    }
}

#[test]
fn semantic_loan_fuzz_never_constructs_invalid_ir() {
    let programs = [
        "fn G(x: edit I64) {}\nfn Main() {\n    var x = 1;\n    G(edit x);\n}\n",
        "fn G(x: edit I64) {}\nfn Main() {\n    let x = 1;\n    G(edit x);\n}\n",
        "fn G(x: edit I64, y: edit I64) {}\nfn Main() {\n    var x = 1;\n    G(edit x, edit x);\n}\n",
        "fn G(x: view I64, y: edit I64) {}\nfn Main() {\n    var x = 1;\n    G(x, edit x);\n}\n",
        "fn G(x: view I64, y: edit I64) {}\nfn Main() {\n    var x = 1;\n    G(x + 0, edit x);\n}\n",
        "fn G(x: edit I64) {}\nfn Main() {\n    var a = [1];\n    G(edit a[0]);\n}\n",
        "fn G(x: edit I64) {}\nfn Main() {\n    var a = [1];\n    G(edit a[2]);\n}\n",
        "fn G(x:edit I64){} fn Main(){G(edit 1);}",
        "fn G(s: view MockResource, r: move MockResource) {}\nfn Main() {\n    let r = MockOpen(1);\n    G(r, move r);\n}\n",
        "fn G(r: edit MockResource) {\n    r = MockOpen(2);\n}\nfn Main() {\n    var r = MockOpen(1);\n    G(edit r);\n    Inspect(r);\n}\n",
        "fn G(x: edit I64) {\n    Drop(x);\n}\nfn Main() {}\n",
        "fn G(x: edit I64) -> I64 {\n    return x;\n}\nfn Main() {\n    var x = 1;\n    AssertEq(G(edit x), 1);\n}\n",
    ];
    for i in 0..30_000 {
        let source = programs[(i * 7919) % programs.len()].replace("=1", &format!("={}", i % 31));
        match compile_scalar("fuzz.n", &source) {
            Ok(program) => {
                let result = run_scalar(&program);
                for d in result.diagnostics {
                    d.validate("fuzz.n", &source).unwrap();
                }
                assert_ne!(result.outcome, Outcome::InternalError("invalid program"));
            }
            Err(CompileError::Rejected(errors)) => {
                for d in errors {
                    d.validate("fuzz.n", &source)
                        .unwrap_or_else(|reason| panic!("{source}: {reason:?}: {d:?}"));
                }
            }
            Err(CompileError::InvalidIr(reason)) => {
                panic!("source produced invalid IR: {reason}: {source}")
            }
            Err(CompileError::Dependent) => panic!("taint escaped: {source}"),
        }
    }
}

#[test]
fn loan_source_fuzz_validates_diagnostics_and_never_panics() {
    let arguments = [
        "edit x",
        "x",
        "edit 1",
        "edit a[0]",
        "edit a[1]",
        "edit a[missing]",
        "move x",
        "edit a[1+2]",
        "edit a[9223372036854775807+1]",
        "edit (x)",
    ];
    let mut seed = 0x1046_92f3u64;
    for case in 0..20_000 {
        seed = seed
            .wrapping_mul(2862933555777941757)
            .wrapping_add(3037000493);
        let argument = arguments[((seed >> 32) as usize) % arguments.len()];
        let separator = if case % 2 == 0 { " " } else { "\n" };
        let source = format!(
            "fn G(z:edit I64){{z=z+1;}}{separator}fn Main(){{var x={};var a=[{},{}];G({argument});}}",
            case % 97,
            case % 31,
            case % 19
        );
        match compile_scalar("fuzz.n", &source) {
            Ok(program) => {
                let result = run_scalar(&program);
                assert!(
                    !matches!(result.outcome, Outcome::InternalError(_)),
                    "{source}"
                );
                for d in result.diagnostics {
                    d.validate("fuzz.n", &source).unwrap();
                }
            }
            Err(CompileError::Rejected(errors)) => {
                for d in errors {
                    d.validate("fuzz.n", &source).unwrap();
                }
            }
            Err(other) => panic!("unexpected compiler outcome {other:?}: {source}"),
        }
    }
}

#[test]
fn loan_compiler_and_interpreter_work_from_small_caller_stacks() {
    for stack in [128 * 1024, 256 * 1024, 512 * 1024] {
        std::thread::Builder::new().stack_size(stack).spawn(|| {
            let source = "fn Inc(x: edit I64) {\n    x = x + 1;\n}\nfn Twice(x: edit I64) {\n    Inc(edit x);\n    Inc(edit x);\n}\nfn Main() {\n    var a = [0, 0];\n    Twice(edit a[0]);\n    AssertEq(a, [2, 0]);\n}\n";
            let program = compile_scalar("stack.n", source).unwrap();
            let result = run_scalar(&program);
            assert_eq!(result.outcome, Outcome::Unit);
        }).unwrap().join().unwrap();
    }
}

#[test]
#[ignore = "explicit 256 KiB to 1 MiB compilation measurement"]
fn dense_loan_and_call_sources_scale_across_twelve_families() {
    let _campaign = crate::test_alloc::campaign_guard();
    use std::sync::atomic::Ordering;
    fn source(family: usize, target: usize) -> String {
        let mut s = String::from(
            "fn Inc(x:edit I64){x=x+1;} fn Twice(x:edit I64){Inc(edit x);Inc(edit x);} fn Read(x:view I64){} fn Pair(x:edit I64,y:edit I64){x=x+1;y=y+1;} fn Retinc(x:edit I64)->I64{x=x+1;return x;} fn Copy(a:view Array<I64>)->Array<I64>{return a;} fn Main(){var x=0;var y=0;var a=[0,0];",
        );
        let mut n = 0usize;
        loop {
            let fragment = match family {
                0 => "Inc(edit x);".to_owned(),
                1 => "Twice(edit x);".to_owned(),
                2 => "Read(x);".to_owned(),
                3 => "Read(x+0);".to_owned(),
                4 => "Inc(edit a[0]);".to_owned(),
                5 => "Inc(edit a[1]);".to_owned(),
                6 => "a[0]=Retinc(edit a[1]);".to_owned(),
                7 => "Read(a[0]);".to_owned(),
                8 => "Copy(a);".to_owned(),
                9 => "Pair(edit x,edit y);".to_owned(),
                10 => format!("let z{n}=x;"),
                11 => format!("var z{n}=0;Inc(edit z{n});"),
                _ => unreachable!(),
            };
            if s.len() + fragment.len() + 1 >= target {
                break;
            }
            s.push_str(&fragment);
            n += 1;
        }
        s.extend(std::iter::repeat_n(' ', target - s.len() - 1));
        s.push('}');
        assert_eq!(s.len(), target);
        s
    }
    for family in 0..12 {
        let mut samples = Vec::new();
        for bytes in [256 * 1024, 1024 * 1024] {
            let src = source(family, bytes);
            super::lower::LOAN_WORK.store(0, Ordering::Relaxed);
            crate::test_alloc::reset("dead_scale_counter.n");
            let program = compile_scalar("dead_scale_counter.n", &src)
                .unwrap_or_else(|e| panic!("family {family}: {e:?}"));
            let work = super::lower::LOAN_WORK.load(Ordering::Relaxed);
            assert!(program.functions().function("Main").is_some());
            samples.push((crate::test_alloc::snapshot("dead_scale_counter.n"), work));
        }
        let work_ratio = samples[1].1 as f64 / samples[0].1.max(1) as f64;
        eprintln!("I06 scale family {family}: {samples:?}, work {work_ratio:.3}");
        for (small, large) in [
            (samples[0].0.0, samples[1].0.0),
            (samples[0].0.1, samples[1].0.1),
        ] {
            assert!(
                large <= small.saturating_mul(8),
                "family {family}: {samples:?}"
            );
        }
        assert!(work_ratio < 5.0, "family {family}: {work_ratio}");
    }
}

#[test]
fn verifier_rejects_unavailable_and_misplaced_loans() {
    use super::model::{self, Op};
    use super::verify::verify_function;
    let p = compile_scalar(
        "i06.n",
        "fn F(x: edit I64) {}\nfn Main() {\n    var x = 1;\n    F(edit x);\n}\n",
    )
    .unwrap();
    let base = p.functions().function("Main").unwrap().clone();
    let verify = |mut f: model::Function| {
        f.content_hash = model::content_hash(&f);
        verify_function(&f)
    };
    let loan_pos = base
        .block
        .instructions
        .iter()
        .position(|i| matches!(i.op, Op::BeginLoan { .. }))
        .unwrap();
    let call_pos = base
        .block
        .instructions
        .iter()
        .position(|i| matches!(i.op, Op::CallDirect { .. }))
        .unwrap();
    let end_pos = base
        .block
        .instructions
        .iter()
        .position(|i| matches!(i.op, Op::EndLoan(_)))
        .unwrap();
    let root = match base.block.instructions[loan_pos].op {
        Op::BeginLoan { root, .. } => root,
        _ => unreachable!(),
    };
    let loan_temp = base.block.instructions[loan_pos].output.unwrap();

    let mut f = base.clone();
    let write = f
        .block
        .instructions
        .iter_mut()
        .find(|i| matches!(i.op, Op::WritePlace { .. }))
        .unwrap();
    let Op::WritePlace { value, .. } = write.op else {
        unreachable!()
    };
    write.op = Op::DropValue(value);
    write.access = model::Access::Own;
    assert_eq!(verify(f), Err("loan unavailable root"));

    let mut f = base.clone();
    f.locals[root.0].mutable = false;
    assert_eq!(verify(f), Err("exclusive loan permission"));

    let mut f = base.clone();
    if let Op::BeginLoan {
        loan, origin_span, ..
    } = &mut f.block.instructions[loan_pos].op
    {
        origin_span.start += 1;
        loan.0 = origin_span.start as usize;
    }
    if let Op::EndLoan(loan) = &mut f.block.instructions[end_pos].op {
        loan.0 += 1;
    }
    assert_eq!(verify(f), Err("loan identity"));

    let mut f = base.clone();
    if let Op::BeginLoan { origin_span, .. } = &mut f.block.instructions[loan_pos].op {
        origin_span.end -= 1;
    }
    assert_eq!(verify(f), Err("loan identity"));

    let mut f = base.clone();
    f.block.instructions.swap(end_pos, end_pos + 1);
    for (position, ins) in f.block.instructions.iter_mut().enumerate() {
        ins.origin.0 = position;
    }
    assert_eq!(verify(f), Err("loan end missing"));

    let mut f = base.clone();
    f.block.instructions.truncate(call_pos + 1);
    let model::Terminator::Return { span, .. } = &f.block.terminator else {
        panic!("return terminator")
    };
    let call_output = f.block.instructions[call_pos].output.unwrap();
    f.block.terminator = model::Terminator::Return {
        value: Some(call_output),
        span: span.clone(),
    };
    assert_eq!(verify(f), Err("loan end missing"));

    let mut f = base.clone();
    f.block.instructions[call_pos].op = Op::DropValue(loan_temp);
    f.block.instructions[call_pos].output = None;
    f.block.instructions[call_pos].ty = None;
    assert_eq!(verify(f), Err("unmatched loan end"));

    let mut f = base;
    f.block.instructions[call_pos].op = Op::CopyValue(root);
    f.block.instructions[call_pos].access = model::Access::Read;
    assert_eq!(verify(f), Err("read during exclusive loan"));
}

#[test]
fn verifier_rejects_forged_overlapping_roots() {
    use super::model::{self, LoanPermission, Op};
    let p = compile_scalar(
        "i06.n",
        "fn F(x: edit I64, y: edit I64) {}\nfn Main() {\n    var x = 1;\n    var y = 2;\n    F(edit x, edit y);\n}\n",
    )
    .unwrap();
    let base = p.functions().function("Main").unwrap().clone();
    let loan_positions = base
        .block
        .instructions
        .iter()
        .enumerate()
        .filter_map(|(i, ins)| matches!(ins.op, Op::BeginLoan { .. }).then_some(i))
        .collect::<Vec<_>>();
    assert_eq!(loan_positions.len(), 2);
    let first_root = match base.block.instructions[loan_positions[0]].op {
        Op::BeginLoan { root, .. } => root,
        _ => unreachable!(),
    };
    let mut f = base.clone();
    if let Op::BeginLoan { root, .. } = &mut f.block.instructions[loan_positions[1]].op {
        *root = first_root;
    }
    f.content_hash = model::content_hash(&f);
    assert_eq!(super::verify::verify_function(&f), Err("overlapping loan"));
    let mut f = base.clone();
    let first_loan = match &f.block.instructions[loan_positions[0]].op {
        Op::BeginLoan { loan, .. } => *loan,
        _ => unreachable!(),
    };
    if let Op::BeginLoan {
        loan, origin_span, ..
    } = &mut f.block.instructions[loan_positions[1]].op
    {
        *loan = first_loan;
        origin_span.start = first_loan.0 as u64;
    }
    f.content_hash = model::content_hash(&f);
    assert_eq!(super::verify::verify_function(&f), Err("loan identity"));
    let mut f = base;
    if let Op::BeginLoan {
        root, permission, ..
    } = &mut f.block.instructions[loan_positions[1]].op
    {
        *root = first_root;
        *permission = LoanPermission::Read;
    }
    f.content_hash = model::content_hash(&f);
    assert_eq!(
        super::verify::verify_function(&f),
        Err("read during exclusive loan")
    );
}

#[test]
fn verifier_requires_reverse_loan_release_and_parameter_permissions() {
    use super::model::{self, Op};
    use super::verify::verify_function;
    let p = compile_scalar(
        "loans.n",
        "fn F(x: edit I64, y: edit I64) {}\nfn Main() {\n    var x = 1;\n    var y = 2;\n    F(edit x, edit y);\n}\n",
    )
    .unwrap();
    let base = p.functions().function("Main").unwrap().clone();
    let ends = base
        .block
        .instructions
        .iter()
        .enumerate()
        .filter_map(|(i, ins)| matches!(ins.op, Op::EndLoan(_)).then_some(i))
        .collect::<Vec<_>>();
    assert_eq!(ends.len(), 2);
    let mut forged = base;
    let (left, right) = forged.block.instructions.split_at_mut(ends[1]);
    std::mem::swap(&mut left[ends[0]].op, &mut right[0].op);
    forged.content_hash = model::content_hash(&forged);
    assert_eq!(verify_function(&forged), Err("unmatched loan end"));

    let p = compile_scalar(
        "loans.n",
        "fn F(x: edit I64) {\n    x = x + 1;\n}\nfn Main() {}\n",
    )
    .unwrap();
    let base = p.functions().function("F").unwrap().clone();
    let mut forged = base.clone();
    forged.locals[0].mutable = false;
    forged.content_hash = model::content_hash(&forged);
    assert_eq!(verify_function(&forged), Err("parameter layout"));

    let mut forged = base;
    let read = forged
        .block
        .instructions
        .iter_mut()
        .find(|ins| matches!(ins.op, Op::CopyValue(_)))
        .unwrap();
    read.op = Op::TransferValue(model::LocalId(0));
    read.access = model::Access::Own;
    forged.content_hash = model::content_hash(&forged);
    assert_eq!(verify_function(&forged), Err("transfer read parameter"));
}

#[test]
fn loan_permission_is_part_of_function_content_hash() {
    use super::model::{self, LoanPermission, Op};
    let p = compile_scalar(
        "hash.n",
        "fn F(x: edit I64) {}\nfn Main() {\n    var x = 1;\n    F(edit x);\n}\n",
    )
    .unwrap();
    let base = p.functions().function("Main").unwrap().clone();
    let mut changed = base.clone();
    let loan = changed
        .block
        .instructions
        .iter_mut()
        .find(|ins| matches!(ins.op, Op::BeginLoan { .. }))
        .unwrap();
    let Op::BeginLoan { permission, .. } = &mut loan.op else {
        unreachable!()
    };
    *permission = LoanPermission::Read;
    assert_ne!(model::content_hash(&changed), base.content_hash);
}

#[test]
fn indexed_inout_does_not_add_live_cells() {
    let values = std::iter::repeat_n("0", 32_767)
        .collect::<Vec<_>>()
        .join(",");
    let source =
        format!("fn Take(x:edit Array<I64>){{}} fn Main(){{var a=[[{values}]];Take(edit a[0]);}}");
    let result = run_scalar(&compile_scalar("cells.n", &source).unwrap());
    assert_eq!(result.outcome, Outcome::Unit, "{:?}", result.diagnostics);
    assert!(result.diagnostics.is_empty());
}

#[test]
fn large_indexed_inout_uses_existing_owner_cells() {
    let values = std::iter::repeat_n("0", 24_575)
        .collect::<Vec<_>>()
        .join(",");
    for argument in ["edit big[0]", "big[0]"] {
        let source = format!(
            "fn F(x:edit Array<I64>){{}} fn G(x:view Array<I64>){{}} fn Main(){{var x=[{values}];var y=[{values}];var big=[move x,move y];{}({argument});}}",
            if argument.starts_with("edit") {
                "F"
            } else {
                "G"
            }
        );
        let result = run_scalar(&compile_scalar("big.n", &source).unwrap());
        assert_eq!(
            result.outcome,
            Outcome::Unit,
            "{argument}: {:?}",
            result.diagnostics
        );
        assert!(result.diagnostics.is_empty());
    }
}

#[test]
fn indexed_inout_replacement_restores_nested_value_and_owner() {
    let source = "fn Replace(x: edit Array<I64>) {\n    x = [7, 8];\n}\nfn Main() {\n    var a = [[1], [2]];\n    Replace(edit a[0]);\n    AssertEq(a, [[7, 8], [2]]);\n}\n";
    let result = run_scalar(&compile_scalar("nested.n", source).unwrap());
    assert_eq!(result.outcome, Outcome::Unit, "{:?}", result.diagnostics);
    assert!(result.diagnostics.is_empty());
}

#[test]
fn indexed_inout_owner_survives_each_callee_runtime_fault() {
    for (body, code) in [
        ("let y=9223372036854775807+1;", "R0101"),
        ("let y=0-9223372036854775807-2;", "R0103"),
        ("let y=x[9];", "R0102"),
        ("AssertEq(1,2);", "R0201"),
        ("Fail();", "R0301"),
    ] {
        let source =
            format!("fn F(x:edit Array<I64>){{{body}}} fn Main(){{var a=[[1]];F(edit a[0]);}}");
        let program = compile_scalar("fault.n", &source).unwrap();
        let result = run_scalar(&program);
        assert_eq!(
            result.outcome,
            Outcome::SimulatorFault,
            "{body}: {:?}",
            result.diagnostics
        );
        assert_eq!(result.diagnostics.len(), 1);
        assert_eq!(result.diagnostics[0].code, code);
        result.diagnostics[0].validate("fault.n", &source).unwrap();
    }
}

#[test]
fn indexed_inout_replacement_updates_root_cell_count() {
    let values = std::iter::repeat_n("0", 32_764)
        .collect::<Vec<_>>()
        .join(",");
    let source = format!(
        "fn Grow(x:edit Array<I64>){{x=[0,0];}} fn Main(){{var a=[[0],[{values}]];Grow(edit a[0]);let b=a;}}"
    );
    let result = run_scalar(&compile_scalar("cells.n", &source).unwrap());
    assert_eq!(result.outcome, Outcome::SimulatorFault);
    assert_eq!(result.diagnostics[0].code, "R0002");
    assert_eq!(
        result.diagnostics[0].data["observed"],
        crate::diagnostic::DataValue::UInt(65_538)
    );
    result.diagnostics[0].validate("cells.n", &source).unwrap();
}

#[test]
fn indexed_write_runs_the_disposal_path_for_replaced_value() {
    let source = "fn Main() {\n    var a = [1];\n    a[0] = 2;\n}\n";
    let result = run_scalar(&compile_scalar("dispose.n", source).unwrap());
    assert_eq!(result.outcome, Outcome::Unit);
    assert_eq!(result.disposals, 2);
}

#[test]
fn ordinary_parameter_cannot_grant_an_exclusive_reborrow() {
    let source = "fn Mutate(x: edit MockResource) {}\nfn Read(r: view MockResource) {\n    Mutate(edit r);\n}\nfn Main() {}\n";
    let CompileError::Rejected(errors) = compile_scalar("permission.n", source).unwrap_err() else {
        panic!("missing permission diagnostic")
    };
    assert_eq!(errors.len(), 1);
    let d = &errors[0];
    assert_eq!(d.code, "NX-E1105");
    assert_eq!(d.subcause.as_deref(), Some("permission_escalation"));
    assert_eq!(
        d.data["requested_access"],
        crate::diagnostic::DataValue::String("borrow_inout".into())
    );
    d.validate("permission.n", source).unwrap();
}

#[test]
fn conflict_uses_earliest_read_loan_and_releases_nested_loans_only() {
    let source = "fn F(a: view I64, b: view I64, c: edit I64) {}\nfn Main() {\n    var x = 1;\n    F(x, x, edit x);\n}\n";
    let CompileError::Rejected(errors) = compile_scalar("loans.n", source).unwrap_err() else {
        panic!("missing conflict")
    };
    assert_eq!(errors.len(), 1);
    let first = source.find("F(x, x,").unwrap() as u64 + 2;
    assert_eq!(
        errors[0].data["loan"],
        crate::diagnostic::DataValue::Object(std::collections::BTreeMap::from([
            (
                "kind".into(),
                crate::diagnostic::DataValue::String("loan".into())
            ),
            (
                "id".into(),
                crate::diagnostic::DataValue::String(format!("loan:{first}:0"))
            ),
        ]))
    );
    errors[0].validate("loans.n", source).unwrap();

    let source = "fn Read(x: view I64) -> I64 {\n    return x;\n}\nfn Write(x: edit I64) -> I64 {\n    x = 2;\n    return x;\n}\nfn F(a: view I64, b: view I64, c: view I64) {}\nfn Main() {\n    var x = 1;\n    F(x, Read(x), Write(edit x));\n}\n";
    let CompileError::Rejected(errors) = compile_scalar("loans.n", source).unwrap_err() else {
        panic!("outer loan was lost")
    };
    assert_eq!(
        errors[0].subcause.as_deref(),
        Some("active_borrow_conflict")
    );
    errors[0].validate("loans.n", source).unwrap();

    let source =
        "fn F(a: edit I64, b: view I64) {}\nfn Main() {\n    var x = 1;\n    F(edit x, x);\n}\n";
    let CompileError::Rejected(errors) = compile_scalar("loans.n", source).unwrap_err() else {
        panic!("exclusive loan allowed read")
    };
    assert_eq!(
        errors[0].subcause.as_deref(),
        Some("active_borrow_conflict")
    );
    errors[0].validate("loans.n", source).unwrap();
}

#[test]
fn adr_gap_reborrow_parent_uses_formal_declaration_without_caller_argument() {
    use crate::diagnostic::{CauseKind, CauseOperation, EntityKind};
    let source = "fn G(a: view I64, b: edit I64) {}\nfn F(x: edit I64) {\n    G(x, edit x);\n}\nfn Main() {}\n";
    let CompileError::Rejected(errors) = compile_scalar("reborrow.n", source).unwrap_err() else {
        panic!("missing conflict")
    };
    assert_eq!(errors.len(), 1);
    let d = &errors[0];
    d.validate("reborrow.n", source).unwrap();
    assert_eq!(d.subcause.as_deref(), Some("active_borrow_conflict"));
    assert_eq!(d.cause_chain.len(), 2);
    assert_eq!(d.cause_chain[0].kind, CauseKind::Borrow);
    assert_eq!(d.cause_chain[1].kind, CauseKind::Reborrow);
    assert_eq!(d.cause_chain[1].operation, CauseOperation::BorrowReborrow);
    assert_eq!(
        d.cause_chain[1].entity.as_ref().unwrap().kind,
        EntityKind::Parameter
    );
    let parameter = source.find("x: edit I64").unwrap();
    assert_eq!(
        d.cause_chain[1].entity.as_ref().unwrap().id,
        format!("parameter:{parameter}")
    );
}

#[test]
fn same_call_independent_name_and_type_sites_are_reported() {
    let names = "fn F(x: view I64, y: view I64) {}\nfn Main() {\n    F(nada, nada2);\n}\n";
    let errors = crate::check::check_proposed("cardinality.n", names, true).unwrap_err();
    assert_eq!(errors.len(), 2, "{errors:?}");
    for e in errors {
        e.validate("cardinality.n", names).unwrap();
    }

    let types = "fn F(x: view I64, y: view I64) {}\nfn Main() {\n    F(true, false);\n}\n";
    let errors = crate::check::check_proposed("cardinality.n", types, true).unwrap_err();
    assert_eq!(errors.len(), 2, "{errors:?}");
    for e in errors {
        e.validate("cardinality.n", types).unwrap();
    }

    let tainted = "fn F(x: view I64, y: view I64) {}\nfn Main() {\n    let x: I64 = true;\n    F(x, false);\n}\n";
    let errors = crate::check::check_proposed("cardinality.n", tainted, true).unwrap_err();
    assert_eq!(errors.len(), 2, "{errors:?}");
    for e in errors {
        e.validate("cardinality.n", tainted).unwrap();
    }
}
#[test]
fn sequential_intrinsic_arguments_hold_place_loans() {
    use crate::diagnostic::{Operation, Stage};

    let cases = [
        (
            "fn Main(){var i=1;AssertEq(i,if true then {i=2;value:1;} else 1);}",
            "i=2",
            "active_borrow_conflict",
            "write",
        ),
        (
            "fn Main(){var a=[1];AssertEq(a[0],if true then {a[0]=2;value:1;} else 1);}",
            "a[0]=2",
            "active_borrow_conflict",
            "write",
        ),
        (
            "fn Inc(x:edit I64){x=x+1;} fn Main(){var i=1;AssertEq(i,if true then {Inc(edit i);value:1;} else 1);}",
            "edit i",
            "active_borrow_conflict",
            "borrow_inout",
        ),
        (
            "fn Inc(x:edit I64){x=x+1;} fn Main(){var a=[1];AssertEq(a[0],if true then {Inc(edit a[0]);value:1;} else 1);}",
            "edit a[0]",
            "active_borrow_conflict",
            "borrow_inout",
        ),
        (
            "fn Main(){var i=1;AssertEq(a:i,b:if true then {i=2;value:1;} else 1);}",
            "i=2",
            "active_borrow_conflict",
            "write",
        ),
    ];
    for (source, site, subcause, requested) in cases {
        for result in [
            check_scalar("call_loan.n", source),
            compile_scalar("call_loan.n", source),
        ] {
            let Err(CompileError::Rejected(errors)) = result else {
                panic!("expected source diagnostic: {source}: {result:?}");
            };
            assert_eq!(errors.len(), 1, "{source}: {errors:#?}");
            let diagnostic = &errors[0];
            diagnostic.validate("call_loan.n", source).unwrap();
            assert_eq!(diagnostic.code, "NX-E1105", "{source}");
            assert_eq!(diagnostic.subcause.as_deref(), Some(subcause), "{source}");
            assert_eq!(diagnostic.stage, Stage::Ownership);
            assert_eq!(diagnostic.operation, Operation::OwnershipAccess);
            assert_eq!(
                diagnostic.data["active_permission"],
                crate::diagnostic::DataValue::String("read".into())
            );
            assert_eq!(
                diagnostic.data["requested_access"],
                crate::diagnostic::DataValue::String(requested.into())
            );
            let start = source.rfind(site).unwrap();
            let expected_start = if requested == "write" {
                start as u64
            } else {
                (start + "edit ".len()) as u64
            };
            let expected_end = expected_start
                + if requested == "borrow_inout" {
                    (site.len() - "edit ".len()) as u64
                } else {
                    1
                };
            assert_eq!(
                diagnostic.primary_span.as_ref().map(|s| (s.start, s.end)),
                Some((expected_start, expected_end)),
                "{source}"
            );
            assert_eq!(diagnostic.data.len(), 5);
        }
    }
    for source in [
        "fn Main(){var i=1;AssertEq(i,if true then {var j=2;j=3;value:1;} else 1);}",
        "fn Main(){var a=[1];AssertEq(a[0],if true then {var b=[2];b[0]=3;value:1;} else 1);}",
        "fn Main(){var i=1;AssertEq(i,i);}",
    ] {
        let result = compile_scalar("call_loan_valid.n", source);
        assert!(result.is_ok(), "{source}: {result:?}");
    }
}

#[test]
#[ignore = "structured sequential-operand ownership campaign"]
fn sequential_operands_never_escape_as_invalid_ir() {
    let per_seed = std::env::var("NEXO_OPERAND_FUZZ_CASES")
        .ok()
        .and_then(|s| s.parse::<usize>().ok())
        .unwrap_or(100_000);
    let mut checked = 0usize;
    let mut parsed = 0usize;
    let mut executed = 0usize;
    for seed in [0x1729_48b3_u64, 0x562c_1d89, 0x91e4_37ab, 0xe4b6_0c75] {
        let mut state = seed;
        for _ in 0..per_seed {
            state = state
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            let n = 1 + (state >> 32) % 47;
            let case = (state >> 17) % 54;
            let body = match case {
                0 => format!("AssertEq(i,if true then {{i={n};value:1;}} else 1);"),
                1 => format!("AssertEq(a[0],if true then {{a[0]={n};value:1;}} else 1);"),
                2 => "AssertEq(i,if true then {Inc(edit i);value:1;} else 1);".into(),
                3 => format!("Pair(i,if true then {{i={n};value:1;}} else 1);"),
                4 => format!("Pair(a[0],if false then {{a[0]={n};value:1;}} else 1);"),
                5 => format!("AssertEq(i+0,if true then {{i={n};value:1;}} else 1);"),
                6 => format!("AssertEq(i,if true then {{var j=2;j={n};value:1;}} else 1);"),
                7 => "AssertEq(i,if true then {Print(i);value:1;} else 1);".into(),
                8 => "AssertEq(i,if true then {Drop(i);value:1;} else 1);".into(),
                9 => format!("AssertEq(a,if true then {{a[0]={n};value:[1];}} else [1]);"),
                10 => format!("AssertEq(a[0],if true then {{a[0]={n};value:1;}} else 1);"),
                11 => format!("AssertEq(a:i,b:if true then {{i={n};value:1;}} else 1);"),
                12 => format!("AssertEq(b:i,a:if true then {{i={n};value:1;}} else 1);"),
                13 => format!("let x=i+(if true then {{i={n};value:1;}} else 1);"),
                14 => format!("let x=[i,if true then {{i={n};value:1;}} else 1];"),
                15 => format!("let x=a[if true then {{a[0]={n};value:0;}} else 0];"),
                16 => format!("a[0]=if true then {{a[0]={n};value:1;}} else 1;"),
                17 => "Print(if true then {Inc(edit i);value:i;} else i);".into(),
                18 => format!("Len(if true then {{a[0]={n};value:a;}} else a);"),
                19 => "while i<2 {AssertEq(i,if true then {i=2;value:1;} else 1);i=i+1;}".into(),
                20 => "AssertEq(i,if true then {return;} else 1);".into(),
                21 => "while i<2 {AssertEq(i,if true then {break;} else 1);}".into(),
                22 => "AssertEq(i,if true then {Print(\"text\");value:1;} else 1);".into(),
                23 => format!("TryAdd(a:i,b:if true then {{i={n};value:1;}} else 1);"),
                24 => "Print(i);".into(),
                25 => "Len(a);".into(),
                26 => "var r=MockOpen(1);Inspect(r);".into(),
                27 => "var r=MockBlockingOpen(1);Inspect(r);".into(),
                28 => "Fail();".into(),
                29 => "MockReadRecords(MockOpen(1));".into(),
                30 => "MockWriteI64(i);".into(),
                31 => "MockRequest();".into(),
                32 => "MockReply(1);".into(),
                33 => "let x:Result<I64,I64>=Ok(i);".into(),
                34 => "Print(a[if true then {let t=move a;value:0;} else 0]);".into(),
                35 => "Print(a[if false then {Drop(a);value:0;} else 0]);".into(),
                36 => "Print(a[Take(move a)]);".into(),
                37 => "Print(a[Len(move a)]);".into(),
                38 => "Print(m[if true then {let t=move m;value:0;} else 0][0]);".into(),
                39 => "Print(a[if true then {a=[2];value:0;} else 0]);".into(),
                40 => "Print(a[if true then {Inc(edit i);value:0;} else 0]);".into(),
                41 => "Print(a[if true then {EditArray(edit a);value:0;} else 0]);".into(),
                42 => format!("a[if true then {{let t=move a;value:0;}} else 0]={n};"),
                43 => "AssertEq(a[if true then {let t=move a;value:0;} else 0],1);".into(),
                44 => "Pair(a[if true then {Drop(a);value:0;} else 0],1);".into(),
                45 => "Print(m[0][if true then {let t=move m;value:0;} else 0]);".into(),
                46 => format!("Print(a==[1]);a[0]={n};"),
                47 => "Print(a==[1]);let t=move a;Print(t[0]);".into(),
                48 => "Print(a==[1]);EditArray(edit a);".into(),
                49 => format!("Print((a==[1])==true);a[0]={n};"),
                50 => format!("if a==[1] {{a[0]={n};}}"),
                51 => format!("while a==[1] {{a[0]={n};break;}}"),
                52 => "let x=i+(if true then {Drop(i);value:1;} else 1);".into(),
                _ => "let x=[i,if true then {let t=move i;value:1;} else 1];".into(),
            };
            let source = format!(
                "fn Inc(x:edit I64){{x=x+1;}} fn Pair(x:view I64,y:view I64){{}} fn Take(p:move Array<I64>)->I64{{return 0;}} fn EditArray(p:edit Array<I64>){{p[0]=2;}} fn Main(){{var i=1;var a=[1];var m=[[1]];{body}}}"
            );
            parsed += usize::from(crate::frontend::parse::parse("operand_fuzz.n", &source).is_ok());
            for result in [
                check_scalar("operand_fuzz.n", &source),
                compile_scalar("operand_fuzz.n", &source),
            ] {
                match result {
                    Ok(program) => {
                        executed += 1;
                        let execution = run_scalar(&program);
                        assert!(
                            !matches!(execution.outcome, Outcome::InternalError(_)),
                            "{source}: {:?}",
                            execution.outcome
                        );
                    }
                    Err(CompileError::Rejected(errors)) => {
                        for diagnostic in errors {
                            diagnostic.validate("operand_fuzz.n", &source).unwrap();
                        }
                    }
                    Err(other) => panic!("{source}: {other:?}"),
                }
            }
            checked += 1;
        }
    }
    assert!(parsed > checked * 3 / 4, "parsed {parsed}/{checked}");
    if per_seed == 100_000 {
        assert_eq!((checked, parsed, executed), (400_000, 377_677, 370_000));
    }
    println!(
        "operand sources: {checked}; parser accepted: {parsed}; check/run invocations: {}; executed: {executed}; invalid IR: 0",
        checked * 2
    );
}
