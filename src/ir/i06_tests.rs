use super::{CompileError, compile_scalar};
use crate::interp::{Outcome, run_scalar};

#[test]
fn inout_reborrow_returns_modified_value() {
    let source = "fn inc(inout x:i64){x=x+1;} fn dos(inout x:i64){inc(inout x);inc(inout x);} fn main(){var x=0;dos(inout x);assert_eq(x,2);}";
    let result = run_scalar(&compile_scalar("i06.nx", source).unwrap());
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
            "fn f(inout x:i64,inout y:i64){} fn main(){var x=1;f(inout x,inout x);}",
            "overlapping_mutable_borrows",
            "binding:46",
            "loan:52:0",
            66,
            67,
            52,
            59,
        ),
        (
            "fn f(x:i64,inout y:i64){} fn main(){var x=1;f(x,inout x);}",
            "active_borrow_conflict",
            "binding:40",
            "loan:46:0",
            54,
            55,
            46,
            47,
        ),
        (
            "fn f(inout x:i64,inout y:i64){} fn main(){var a=[1,2];f(inout a[0],inout a[1]);}",
            "overlapping_mutable_borrows",
            "binding:46",
            "loan:56:0",
            73,
            77,
            56,
            66,
        ),
    ];
    for (source, subcause, binding, loan, start, end, origin_start, origin_end) in cases {
        let CompileError::Rejected(errors) = compile_scalar("i06.nx", source).unwrap_err() else {
            panic!("missing source diagnostic: {source}")
        };
        assert_eq!(errors.len(), 1, "{source}");
        let d = &errors[0];
        d.validate("i06.nx", source).unwrap();
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
            "fn f(inout x:i64,y:i64){} fn main(){var x=1;f(inout x,x+0);}",
            "x+0",
            "x",
            "read",
        ),
        (
            "fn f(inout x:i64,y:i64){} fn main(){var a=[1];f(inout a[0],a[0]+0);}",
            "a[0]+0",
            "a[0]",
            "read",
        ),
        (
            "fn f(x:i64,inout y:i64){} fn main(){var a=[1];f(a[0],inout a[0]);}",
            "inout a[0]",
            "a[0]",
            "borrow_inout",
        ),
        (
            "fn f(inout x:i64,y:i64){} fn main(){var x=0;var a=[1];f(inout x,a[x]+0);}",
            "a[x]+0",
            "x",
            "read",
        ),
        (
            "fn f(inout x:i64,y:bool){} fn main(){var x=0;f(inout x,x<1);}",
            "x<1",
            "x",
            "read",
        ),
    ];
    for (source, suffix, place, access) in cases {
        let CompileError::Rejected(errors) = compile_scalar("conflict.nx", source).unwrap_err()
        else {
            panic!("missing source conflict: {source}");
        };
        assert_eq!(errors.len(), 1, "{source}");
        let d = &errors[0];
        d.validate("conflict.nx", source).unwrap();
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
            "{prefix}fn f(inout x:i64,inout y:i64){{}}\nfn main(){{var a=[1,2];f(inout a[0],inout a[1]);}}"
        );
        let CompileError::Rejected(errors) = compile_scalar("p07.nx", &source).unwrap_err() else {
            panic!("missing P07 conflict")
        };
        assert_eq!(errors.len(), 1);
        let d = &errors[0];
        d.validate("p07.nx", &source).unwrap();
        assert_eq!(d.code, "NX-E1105");
        assert_eq!(d.subcause.as_deref(), Some("overlapping_mutable_borrows"));
        let start = source.rfind("a[1]").unwrap() as u64;
        assert_eq!(
            d.primary_span.as_ref().map(|s| (s.start, s.end)),
            Some((start, start + 4))
        );
        let first = source.find("inout a[0]").unwrap() as u64;
        assert_eq!(
            d.cause_chain[0].span.as_ref().map(|s| (s.start, s.end)),
            Some((first, first + 10))
        );
    }
}

#[test]
fn named_loan_identity_starts_at_argument_label() {
    use crate::diagnostic::DataValue;
    let source =
        "fn f(inout x:i64,inout y:i64){} fn main(){var a=[1,2];f(x: inout a[0],y: inout a[1]);}";
    let CompileError::Rejected(errors) = compile_scalar("label.nx", source).unwrap_err() else {
        panic!("missing named loan conflict")
    };
    assert_eq!(errors.len(), 1);
    let d = &errors[0];
    d.validate("label.nx", source).unwrap();
    let first = source.find("x: inout a[0]").unwrap() as u64;
    let second = source.find("y: inout a[1]").unwrap() as u64;
    let place = source.rfind("a[1]").unwrap() as u64;
    assert_eq!(
        d.primary_span.as_ref().map(|s| (s.start, s.end)),
        Some((place, place + 4))
    );
    assert_eq!(
        d.cause_chain[0].span.as_ref().map(|s| (s.start, s.end)),
        Some((first, first + 13))
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
    let source = "fn f(inout x:i64){} fn main(){var a=[1];f(x: inout a[0]);}";
    let program = compile_scalar("named.nx", source).unwrap();
    let function = program.functions().function("main").unwrap();
    let first = source.find("x: inout a[0]").unwrap() as u64;
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
    assert_eq!(loan, (first as usize, first, first + 13));
    assert_eq!(run_scalar(&program).outcome, Outcome::Unit);
}

#[test]
fn two_read_loans_of_the_same_root_remain_compatible() {
    let source = "fn f(x:Array<i64>,y:Array<i64>){} fn main(){var a=[1];f(a,a);}";
    let result = run_scalar(&compile_scalar("reads.nx", source).unwrap());
    assert_eq!(result.outcome, Outcome::Unit);
    assert!(result.diagnostics.is_empty());
}

#[test]
fn indexed_assignment_rechecks_owner_consumed_by_rhs() {
    let source =
        "fn take(consume x:Array<i64>)->i64{return 1;} fn main(){var a=[0];a[0]=take(consume a);}";
    let CompileError::Rejected(errors) = compile_scalar("consume.nx", source).unwrap_err() else {
        panic!("missing unavailable destination")
    };
    assert_eq!(errors.len(), 1);
    let d = &errors[0];
    d.validate("consume.nx", source).unwrap();
    assert_eq!(d.code, "NX-E0202");
    assert_eq!(d.subcause.as_deref(), Some("definitely_consumed"));
    let destination = source.rfind("a[0]=").unwrap() as u64;
    assert_eq!(
        d.primary_span.as_ref().map(|s| (s.start, s.end)),
        Some((destination, destination + 1))
    );
}

#[test]
fn valid_pairs_and_permissions() {
    let valid = [
        "fn f(inout x:i64){} fn main(){var x=1;f(inout x);}",
        "fn f(inout x:i64){} fn main(){var x=1;var y=2;f(inout y);}",
        "fn f(x:i64,inout y:i64){} fn main(){var x=1;f(x+0,inout x);}",
        "fn f(inout x:i64,inout y:i64){} fn main(){var a=[1,2];var b=[3,4];f(inout a[0],inout b[1]);}",
        "fn f(x:i64,y:i64){} fn main(){var x=1;f(x,x);}",
        "fn f(x:i64,y:i64){} fn main(){var x=1;f(x,x+0);}",
        "fn f(inout x:MockResource){x=mock_open(2);} fn main(){var r=mock_open(1);f(inout r);inspect(r);}",
        "fn copy(a:Array<i64>)->Array<i64>{return a;} fn main(){var a=[1];let b=copy(a);a[0]=2;assert_eq(b,[1]);}",
    ];
    for source in valid {
        let result = run_scalar(
            &compile_scalar("i06.nx", source).unwrap_or_else(|e| panic!("{source}: {e:?}")),
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
            "fn f(x:MockResource){drop(x);} fn main(){}",
            "permission_escalation",
        ),
        (
            "fn f(inout x:MockResource){drop(x);} fn main(){}",
            "permission_escalation",
        ),
        (
            "fn f(inout x:MockResource)->MockResource{return consume x;} fn main(){}",
            "permission_escalation",
        ),
        (
            "fn f(s:MockResource,consume r:MockResource){} fn main(){let r=mock_open(1);f(r,consume r);}",
            "active_borrow_conflict",
        ),
        (
            "fn f(r:MockResource,n:()){} fn main(){let r=mock_open(1);f(r,drop(r));}",
            "active_borrow_conflict",
        ),
    ] {
        let CompileError::Rejected(errors) = compile_scalar("i06.nx", source).unwrap_err() else {
            panic!("missing diagnostic: {source}")
        };
        assert_eq!(errors[0].subcause.as_deref(), Some(subcause), "{source}");
        for e in errors {
            e.validate("i06.nx", source).unwrap();
        }
    }
}

#[test]
fn indexed_destination_rechecks_after_rhs_call_modifies_root() {
    let source = "fn resize(inout a:Array<i64>)->i64{a=[7];return 9;} fn main(){var a=[1,2];a[1]=resize(inout a);}";
    let result = run_scalar(&compile_scalar("i06.nx", source).unwrap());
    assert_eq!(result.outcome, Outcome::SimulatorFault);
    assert_eq!(result.diagnostics[0].code, "R0102");
    let source = "fn resize(inout a:Array<i64>)->i64{a=[7,8,9];return 4;} fn main(){var a=[1];a[1]=resize(inout a);assert_eq(a,[7,4,9]);}";
    let result = run_scalar(&compile_scalar("i06.nx", source).unwrap());
    assert_eq!(result.outcome, Outcome::Unit, "{:?}", result.diagnostics);
}

#[test]
fn syntax_mode_and_immutable_place_pairs() {
    use crate::diagnostic::{Operation, Stage};
    let cases = [
        (
            "fn f(inout x:i64){} fn main(){var x=1;f(x);}",
            "NX-E0111",
            Some("argument_mode_mismatch"),
            Stage::Arguments,
            Operation::CallBind,
            (40, 41),
        ),
        (
            "fn f(inout x:i64){} fn main(){let x=1;f(inout x);}",
            "NX-E0105",
            None,
            Stage::Ownership,
            Operation::PlaceWrite,
            (46, 47),
        ),
        (
            "fn f(inout x:i64){} fn main(){f(inout 1);}",
            "NX-E0002",
            None,
            Stage::Parse,
            Operation::SyntaxParse,
            (38, 39),
        ),
    ];
    for (source, code, subcause, stage, operation, primary) in cases {
        let CompileError::Rejected(errors) = compile_scalar("i06.nx", source).unwrap_err() else {
            panic!("missing {code}: {source}")
        };
        assert_eq!(errors.len(), 1, "{source}");
        let d = &errors[0];
        d.validate("i06.nx", source).unwrap();
        assert_eq!(d.code, code);
        assert_eq!(d.subcause.as_deref(), subcause);
        assert_eq!(d.stage, stage);
        assert_eq!(d.operation, operation);
        assert_eq!(
            d.primary_span.as_ref().map(|s| (s.start, s.end)),
            Some(primary)
        );
    }
}

#[test]
fn verifier_rejects_forged_loan_permissions_and_lifecycle() {
    use super::model::{self, LoanId, LoanPermission, Op};
    use super::verify::verify_function;
    let p = compile_scalar(
        "i06.nx",
        "fn f(inout x:i64){} fn main(){var x=1;f(inout x);}",
    )
    .unwrap();
    let base = p.functions().function("main").unwrap().clone();
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
    assert_eq!(verify(f), Err("inout argument permission"));

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
    let source = "fn f(consume a:MockResource,consume b:MockResource){} fn main(){let r=mock_open(1);let s=mock_open(2);f(r,s);}";
    let CompileError::Rejected(errors) = compile_scalar("i06.nx", source).unwrap_err() else {
        panic!("missing copy diagnostics")
    };
    assert_eq!(errors.len(), 2);
    assert!(errors.iter().all(|e| e.code == "NX-E0201"));
    assert!(
        errors[0].primary_span.as_ref().unwrap().start
            < errors[1].primary_span.as_ref().unwrap().start
    );
    for e in errors {
        e.validate("i06.nx", source).unwrap();
    }

    let source = "fn f(consume a:MockResource,consume b:MockResource){} fn main(){let r=mock_open(1);let s=r;f(s,s);}";
    let CompileError::Rejected(errors) = compile_scalar("i06.nx", source).unwrap_err() else {
        panic!("missing original copy diagnostic")
    };
    assert_eq!(errors.len(), 1);
    assert_eq!(errors[0].code, "NX-E0201");
}

#[test]
fn loan_hash_ignores_source_offsets_and_tracks_permission_and_callee_signature() {
    let a = compile_scalar(
        "i06.nx",
        "fn f(inout x:i64){x=x+1;} fn main(){var x=1;f(inout x);}",
    )
    .unwrap();
    let b = compile_scalar(
        "i06.nx",
        "fn f(inout x:i64){ x = x + 1; } fn main(){ var x = 1; f(inout x); }",
    )
    .unwrap();
    assert_eq!(
        a.functions().function("main").unwrap().content_hash,
        b.functions().function("main").unwrap().content_hash
    );
    let c = compile_scalar(
        "i06.nx",
        "fn f(inout x:i64){x=x+2;} fn main(){var x=1;f(inout x);}",
    )
    .unwrap();
    assert_eq!(
        a.functions().function("main").unwrap().content_hash,
        c.functions().function("main").unwrap().content_hash
    );
    assert_ne!(
        a.functions().function("f").unwrap().content_hash,
        c.functions().function("f").unwrap().content_hash
    );
    let d = compile_scalar(
        "i06.nx",
        "fn f(inout y:i64){y=y+1;} fn main(){var x=1;f(inout x);}",
    )
    .unwrap();
    assert_ne!(
        a.functions().function("main").unwrap().content_hash,
        d.functions().function("main").unwrap().content_hash
    );
}

#[test]
fn development_substitution_executes_new_inout_body() {
    let mut base = compile_scalar(
        "i06.nx",
        "fn inc(inout x:i64){x=x+1;} fn main(){var x=0;inc(inout x);assert_eq(x,2);}",
    )
    .unwrap();
    let donor = compile_scalar(
        "i06.nx",
        "fn inc(inout x:i64){x=x+2;} fn main(){var x=0;inc(inout x);assert_eq(x,2);}",
    )
    .unwrap();
    assert_eq!(run_scalar(&base).outcome, Outcome::SimulatorFault);
    base.substitute_for_development("inc", &donor).unwrap();
    assert_eq!(run_scalar(&base).outcome, Outcome::Unit);
}

#[test]
fn inout_resource_replacement_preserves_single_owner_and_cleanup_order() {
    let source = "fn replace(inout r:MockResource){r=mock_open(2);inspect(r);} fn main(){var r=mock_open(1);replace(inout r);inspect(r);}";
    let result = run_scalar(&compile_scalar("i06.nx", source).unwrap());
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
    let source = "fn change(inout r:MockResource){r=mock_open(2);fail();} fn main(){var r=mock_open(1);change(inout r);}";
    let result = run_scalar(&compile_scalar("i06.nx", source).unwrap());
    assert_eq!(result.outcome, Outcome::SimulatorFault);
    assert_eq!(result.diagnostics[0].code, "R0301");
    assert_eq!(
        result.observations.events(),
        ["open:0:1", "open:1:2", "close:0:1", "close:1:2"]
    );

    let source = "fn recur(inout r:MockResource){recur(inout r);} fn main(){var r=mock_open(1);recur(inout r);}";
    let result = run_scalar(&compile_scalar("i06.nx", source).unwrap());
    assert_eq!(result.outcome, Outcome::SimulatorFault);
    assert_eq!(result.diagnostics[0].code, "R0002");
    assert_eq!(result.observations.events(), ["open:0:1", "close:0:1"]);
}

#[test]
fn generated_inout_and_copy_programs_match_independent_integer_model() {
    let prefix = "fn inc(inout x:i64){x=x+1;} fn twice(inout x:i64){inc(inout x);inc(inout x);} fn retinc(inout x:i64)->i64{x=x+1;return x;} fn copy(a:Array<i64>)->Array<i64>{return a;} fn main(){var x=0;var a=[0,0];";
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
                    source.push_str("inc(inout x);");
                    x += 1;
                }
                1 => {
                    source.push_str("inc(inout a[0]);");
                    a[0] += 1;
                }
                2 => {
                    source.push_str("inc(inout a[1]);");
                    a[1] += 1;
                }
                3 => {
                    source.push_str("twice(inout x);");
                    x += 2;
                }
                4 => {
                    source.push_str("a[0]=retinc(inout a[1]);");
                    a[1] += 1;
                    a[0] = a[1];
                }
                _ => {
                    source.push_str(&format!(
                        "let b{step}=copy(a);a[0]=a[0]+1;assert_eq(b{step},[{},{}]);",
                        a[0], a[1]
                    ));
                    a[0] += 1;
                }
            }
        }
        source.push_str(&format!(
            "assert_eq(x,{x});assert_eq(a,[{},{}]);}}",
            a[0], a[1]
        ));
        let compiled = compile_scalar("model.nx", &source)
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
        "fn g(inout x:i64){} fn main(){var x=1;g(inout x);}",
        "fn g(inout x:i64){} fn main(){let x=1;g(inout x);}",
        "fn g(inout x:i64,inout y:i64){} fn main(){var x=1;g(inout x,inout x);}",
        "fn g(x:i64,inout y:i64){} fn main(){var x=1;g(x,inout x);}",
        "fn g(x:i64,inout y:i64){} fn main(){var x=1;g(x+0,inout x);}",
        "fn g(inout x:i64){} fn main(){var a=[1];g(inout a[0]);}",
        "fn g(inout x:i64){} fn main(){var a=[1];g(inout a[2]);}",
        "fn g(inout x:i64){} fn main(){g(inout 1);}",
        "fn g(s:MockResource,consume r:MockResource){} fn main(){let r=mock_open(1);g(r,consume r);}",
        "fn g(inout r:MockResource){r=mock_open(2);} fn main(){var r=mock_open(1);g(inout r);inspect(r);}",
        "fn g(inout x:i64){drop(x);} fn main(){}",
        "fn g(inout x:i64)->i64{return x;} fn main(){var x=1;assert_eq(g(inout x),1);}",
    ];
    for i in 0..30_000 {
        let source = programs[(i * 7919) % programs.len()].replace("=1", &format!("={}", i % 31));
        match compile_scalar("fuzz.nx", &source) {
            Ok(program) => {
                let result = run_scalar(&program);
                for d in result.diagnostics {
                    d.validate("fuzz.nx", &source).unwrap();
                }
                assert_ne!(result.outcome, Outcome::InternalError("invalid program"));
            }
            Err(CompileError::Rejected(errors)) => {
                for d in errors {
                    d.validate("fuzz.nx", &source).unwrap();
                }
            }
            Err(CompileError::InvalidIr(reason)) => {
                panic!("source produced invalid IR: {reason}: {source}")
            }
            Err(CompileError::OutsideScalarSlice(reason)) => {
                panic!("unexpected deferred source: {reason}: {source}")
            }
            Err(CompileError::Dependent) => panic!("taint escaped: {source}"),
        }
    }
}

#[test]
fn loan_source_fuzz_validates_diagnostics_and_never_panics() {
    let arguments = [
        "inout x",
        "x",
        "inout 1",
        "inout a[0]",
        "inout a[1]",
        "inout a[missing]",
        "consume x",
        "inout a[1+2]",
        "inout a[9223372036854775807+1]",
        "inout (x)",
    ];
    let mut seed = 0x1046_92f3u64;
    for case in 0..20_000 {
        seed = seed
            .wrapping_mul(2862933555777941757)
            .wrapping_add(3037000493);
        let argument = arguments[((seed >> 32) as usize) % arguments.len()];
        let separator = if case % 2 == 0 { " " } else { "\n" };
        let source = format!(
            "fn g(inout z:i64){{z=z+1;}}{separator}fn main(){{var x={};var a=[{},{}];g({argument});}}",
            case % 97,
            case % 31,
            case % 19
        );
        match compile_scalar("fuzz.nx", &source) {
            Ok(program) => {
                let result = run_scalar(&program);
                assert!(
                    !matches!(result.outcome, Outcome::InternalError(_)),
                    "{source}"
                );
                for d in result.diagnostics {
                    d.validate("fuzz.nx", &source).unwrap();
                }
            }
            Err(CompileError::Rejected(errors)) => {
                for d in errors {
                    d.validate("fuzz.nx", &source).unwrap();
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
            let source = "fn inc(inout x:i64){x=x+1;} fn twice(inout x:i64){inc(inout x);inc(inout x);} fn main(){var a=[0,0];twice(inout a[0]);assert_eq(a,[2,0]);}";
            let program = compile_scalar("stack.nx", source).unwrap();
            let result = run_scalar(&program);
            assert_eq!(result.outcome, Outcome::Unit);
        }).unwrap().join().unwrap();
    }
}

#[test]
#[ignore = "explicit 256 KiB to 1 MiB compilation measurement"]
fn dense_loan_and_call_sources_scale_across_twelve_families() {
    use std::sync::atomic::Ordering;
    fn source(family: usize, target: usize) -> String {
        let mut s = String::from(
            "fn inc(inout x:i64){x=x+1;} fn twice(inout x:i64){inc(inout x);inc(inout x);} fn read(x:i64){} fn pair(inout x:i64,inout y:i64){x=x+1;y=y+1;} fn retinc(inout x:i64)->i64{x=x+1;return x;} fn copy(a:Array<i64>)->Array<i64>{return a;} fn main(){var x=0;var y=0;var a=[0,0];",
        );
        let mut n = 0usize;
        loop {
            let fragment = match family {
                0 => "inc(inout x);".to_owned(),
                1 => "twice(inout x);".to_owned(),
                2 => "read(x);".to_owned(),
                3 => "read(x+0);".to_owned(),
                4 => "inc(inout a[0]);".to_owned(),
                5 => "inc(inout a[1]);".to_owned(),
                6 => "a[0]=retinc(inout a[1]);".to_owned(),
                7 => "read(a[0]);".to_owned(),
                8 => "copy(a);".to_owned(),
                9 => "pair(inout x,inout y);".to_owned(),
                10 => format!("let z{n}=x;"),
                11 => format!("var z{n}=0;inc(inout z{n});"),
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
            crate::test_alloc::reset("dead_scale_counter.nx");
            let program = compile_scalar("dead_scale_counter.nx", &src)
                .unwrap_or_else(|e| panic!("family {family}: {e:?}"));
            let work = super::lower::LOAN_WORK.load(Ordering::Relaxed);
            assert!(program.functions().function("main").is_some());
            samples.push((crate::test_alloc::snapshot("dead_scale_counter.nx"), work));
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
        "i06.nx",
        "fn f(inout x:i64){} fn main(){var x=1;f(inout x);}",
    )
    .unwrap();
    let base = p.functions().function("main").unwrap().clone();
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
        "i06.nx",
        "fn f(inout x:i64,inout y:i64){} fn main(){var x=1;var y=2;f(inout x,inout y);}",
    )
    .unwrap();
    let base = p.functions().function("main").unwrap().clone();
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
        "loans.nx",
        "fn f(inout x:i64,inout y:i64){} fn main(){var x=1;var y=2;f(inout x,inout y);}",
    )
    .unwrap();
    let base = p.functions().function("main").unwrap().clone();
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

    let p = compile_scalar("loans.nx", "fn f(inout x:i64){x=x+1;} fn main(){}").unwrap();
    let base = p.functions().function("f").unwrap().clone();
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
        "hash.nx",
        "fn f(inout x:i64){} fn main(){var x=1;f(inout x);}",
    )
    .unwrap();
    let base = p.functions().function("main").unwrap().clone();
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
    let source = format!(
        "fn take(inout x:Array<i64>){{}} fn main(){{var a=[[{values}]];take(inout a[0]);}}"
    );
    let result = run_scalar(&compile_scalar("cells.nx", &source).unwrap());
    assert_eq!(result.outcome, Outcome::Unit, "{:?}", result.diagnostics);
    assert!(result.diagnostics.is_empty());
}

#[test]
fn large_indexed_inout_uses_existing_owner_cells() {
    let values = std::iter::repeat_n("0", 24_575)
        .collect::<Vec<_>>()
        .join(",");
    for argument in ["inout big[0]", "big[0]"] {
        let source = format!(
            "fn f(inout x:Array<i64>){{}} fn g(x:Array<i64>){{}} fn main(){{var x=[{values}];var y=[{values}];var big=[consume x,consume y];{}({argument});}}",
            if argument.starts_with("inout") {
                "f"
            } else {
                "g"
            }
        );
        let result = run_scalar(&compile_scalar("big.nx", &source).unwrap());
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
    let source = "fn replace(inout x:Array<i64>){x=[7,8];} fn main(){var a=[[1],[2]];replace(inout a[0]);assert_eq(a,[[7,8],[2]]);}";
    let result = run_scalar(&compile_scalar("nested.nx", source).unwrap());
    assert_eq!(result.outcome, Outcome::Unit, "{:?}", result.diagnostics);
    assert!(result.diagnostics.is_empty());
}

#[test]
fn indexed_inout_owner_survives_each_callee_runtime_fault() {
    for (body, code) in [
        ("let y=9223372036854775807+1;", "R0101"),
        ("let y=0-9223372036854775807-2;", "R0103"),
        ("let y=x[9];", "R0102"),
        ("assert_eq(1,2);", "R0201"),
        ("fail();", "R0301"),
    ] {
        let source =
            format!("fn f(inout x:Array<i64>){{{body}}} fn main(){{var a=[[1]];f(inout a[0]);}}");
        let program = compile_scalar("fault.nx", &source).unwrap();
        let result = run_scalar(&program);
        assert_eq!(
            result.outcome,
            Outcome::SimulatorFault,
            "{body}: {:?}",
            result.diagnostics
        );
        assert_eq!(result.diagnostics.len(), 1);
        assert_eq!(result.diagnostics[0].code, code);
        result.diagnostics[0].validate("fault.nx", &source).unwrap();
    }
}

#[test]
fn indexed_inout_replacement_updates_root_cell_count() {
    let values = std::iter::repeat_n("0", 32_764)
        .collect::<Vec<_>>()
        .join(",");
    let source = format!(
        "fn grow(inout x:Array<i64>){{x=[0,0];}} fn main(){{var a=[[0],[{values}]];grow(inout a[0]);let b=a;}}"
    );
    let result = run_scalar(&compile_scalar("cells.nx", &source).unwrap());
    assert_eq!(result.outcome, Outcome::SimulatorFault);
    assert_eq!(result.diagnostics[0].code, "R0002");
    assert_eq!(
        result.diagnostics[0].data["observed"],
        crate::diagnostic::DataValue::UInt(65_538)
    );
    result.diagnostics[0].validate("cells.nx", &source).unwrap();
}

#[test]
fn indexed_write_runs_the_disposal_path_for_replaced_value() {
    let source = "fn main(){var a=[1];a[0]=2;}";
    let result = run_scalar(&compile_scalar("dispose.nx", source).unwrap());
    assert_eq!(result.outcome, Outcome::Unit);
    assert_eq!(result.disposals, 2);
}

#[test]
fn ordinary_parameter_cannot_grant_an_exclusive_reborrow() {
    let source =
        "fn mutate(inout x:MockResource){} fn read(r:MockResource){mutate(inout r);} fn main(){}";
    let CompileError::Rejected(errors) = compile_scalar("permission.nx", source).unwrap_err()
    else {
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
    d.validate("permission.nx", source).unwrap();
}

#[test]
fn conflict_uses_earliest_read_loan_and_releases_nested_loans_only() {
    let source = "fn f(a:i64,b:i64,inout c:i64){} fn main(){var x=1;f(x,x,inout x);}";
    let CompileError::Rejected(errors) = compile_scalar("loans.nx", source).unwrap_err() else {
        panic!("missing conflict")
    };
    assert_eq!(errors.len(), 1);
    let first = source.find("f(x,x,").unwrap() as u64 + 2;
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
    errors[0].validate("loans.nx", source).unwrap();

    let source = "fn read(x:i64)->i64{return x;} fn write(inout x:i64)->i64{x=2;return x;} fn f(a:i64,b:i64,c:i64){} fn main(){var x=1;f(x,read(x),write(inout x));}";
    let CompileError::Rejected(errors) = compile_scalar("loans.nx", source).unwrap_err() else {
        panic!("outer loan was lost")
    };
    assert_eq!(
        errors[0].subcause.as_deref(),
        Some("active_borrow_conflict")
    );
    errors[0].validate("loans.nx", source).unwrap();

    let source = "fn f(inout a:i64,b:i64){} fn main(){var x=1;f(inout x,x);}";
    let CompileError::Rejected(errors) = compile_scalar("loans.nx", source).unwrap_err() else {
        panic!("exclusive loan allowed read")
    };
    assert_eq!(
        errors[0].subcause.as_deref(),
        Some("active_borrow_conflict")
    );
    errors[0].validate("loans.nx", source).unwrap();
}

#[test]
fn adr_gap_reborrow_parent_uses_formal_declaration_without_caller_argument() {
    use crate::diagnostic::{CauseKind, CauseOperation, EntityKind};
    let source = "fn g(a:i64,inout b:i64){} fn f(inout x:i64){g(x,inout x);} fn main(){}";
    let CompileError::Rejected(errors) = compile_scalar("reborrow.nx", source).unwrap_err() else {
        panic!("missing conflict")
    };
    assert_eq!(errors.len(), 1);
    let d = &errors[0];
    d.validate("reborrow.nx", source).unwrap();
    assert_eq!(d.subcause.as_deref(), Some("active_borrow_conflict"));
    assert_eq!(d.cause_chain.len(), 2);
    assert_eq!(d.cause_chain[0].kind, CauseKind::Borrow);
    assert_eq!(d.cause_chain[1].kind, CauseKind::Reborrow);
    assert_eq!(d.cause_chain[1].operation, CauseOperation::BorrowReborrow);
    assert_eq!(
        d.cause_chain[1].entity.as_ref().unwrap().kind,
        EntityKind::Parameter
    );
    assert_eq!(d.cause_chain[1].entity.as_ref().unwrap().id, "parameter:37");
}

#[test]
fn same_call_independent_name_and_type_sites_are_reported() {
    let names = "fn f(x:i64,y:i64){} fn main(){f(nada,nada2);}";
    let errors = crate::check::check_proposed("cardinality.nx", names, true).unwrap_err();
    assert_eq!(errors.len(), 2, "{errors:?}");
    for e in errors {
        e.validate("cardinality.nx", names).unwrap();
    }

    let types = "fn f(x:i64,y:i64){} fn main(){f(true,false);}";
    let errors = crate::check::check_proposed("cardinality.nx", types, true).unwrap_err();
    assert_eq!(errors.len(), 2, "{errors:?}");
    for e in errors {
        e.validate("cardinality.nx", types).unwrap();
    }

    let tainted = "fn f(x:i64,y:i64){} fn main(){let x:i64=true; f(x,false);}";
    let errors = crate::check::check_proposed("cardinality.nx", tainted, true).unwrap_err();
    assert_eq!(errors.len(), 2, "{errors:?}");
    for e in errors {
        e.validate("cardinality.nx", tainted).unwrap();
    }
}
