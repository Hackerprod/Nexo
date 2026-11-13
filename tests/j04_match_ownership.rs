use nexo::diagnostic::{
    CauseKind, CauseOperation, DataValue, Diagnostic, EntityKind, Evidence, Operation, RelatedRole,
    Span, Stage,
};
use nexo::interp::{MockEnvironment, Outcome, run_scalar};
use nexo::ir::{CompileError, ReducedProfile, compile_scalar_with_profile};
use std::collections::BTreeMap;

const FILE: &str = "<memory>";

fn compile(source: &str) -> nexo::ir::Program {
    compile_scalar_with_profile(FILE, source, ReducedProfile::JNormal, true)
        .unwrap_or_else(|error| panic!("valid program rejected: {error:?}"))
}

fn rejected(source: &str) -> Box<Diagnostic> {
    match compile_scalar_with_profile(FILE, source, ReducedProfile::JNormal, false) {
        Err(CompileError::Rejected(mut errors)) if errors.len() == 1 => {
            let error = errors.pop().unwrap();
            error.validate(FILE, source).unwrap();
            error
        }
        other => panic!("expected one diagnostic: {other:?}"),
    }
}

fn span(start: u64, end: u64) -> Span {
    Span {
        file: FILE.into(),
        start,
        end,
    }
}

fn entity(kind: &str, id: &str) -> DataValue {
    DataValue::Object(BTreeMap::from([
        ("kind".into(), DataValue::String(kind.into())),
        ("id".into(), DataValue::String(id.into())),
    ]))
}

fn unit_with_events(source: &str, expected: &[&str]) {
    let result = run_scalar(&compile(source), MockEnvironment::default());
    assert_eq!(result.outcome, Outcome::Unit, "{source}");
    assert!(result.diagnostics.is_empty(), "{source}");
    assert_eq!(result.observations.events(), expected, "{source}");
}

#[test]
fn m06_pattern_edit_requires_exclusive_selection() {
    let negative = "fn Main(){var o:Option<I64>=Some(1);match o{Some(edit n)=>{},None=>{}}}";
    let error = rejected(negative);
    assert_eq!(error.code, "NX-E1105");
    assert_eq!(error.subcause.as_deref(), Some("permission_escalation"));
    assert_eq!(error.stage, Stage::Ownership);
    assert_eq!(error.operation, Operation::OwnershipAccess);
    assert_eq!(error.primary_span, Some(span(49, 55)));
    assert_eq!(
        error.data,
        BTreeMap::from([
            ("origin".into(), entity("binding", "binding:14")),
            ("permission_source".into(), entity("loan", "loan:42:0")),
            ("active_permission".into(), DataValue::String("read".into())),
            (
                "requested_access".into(),
                DataValue::String("borrow_inout".into())
            ),
        ])
    );
    assert_eq!(
        error
            .related_spans
            .iter()
            .map(|related| (related.role, related.span.clone()))
            .collect::<Vec<_>>(),
        vec![
            (RelatedRole::Declaration, span(14, 15)),
            (RelatedRole::Origin, span(42, 43))
        ]
    );
    assert_eq!(error.cause_chain.len(), 1);
    let cause = &error.cause_chain[0];
    assert_eq!(
        (
            cause.kind,
            cause.operation,
            cause.span.clone(),
            cause.evidence
        ),
        (
            CauseKind::Borrow,
            CauseOperation::BorrowRead,
            Some(span(42, 43)),
            Evidence::Verified
        )
    );
    assert_eq!(
        cause
            .entity
            .as_ref()
            .map(|entity| (entity.kind, entity.id.as_str())),
        Some((EntityKind::Loan, "loan:42:0"))
    );
    unit_with_events(
        "fn Main(){var o:Option<I64>=Some(1);match edit o{Some(edit n)=>{n=2;},None=>{}}match o{Some(n)=>{AssertEq(n,2);},None=>{}}}",
        &[],
    );
}

#[test]
fn m09_join_reports_maybe_consumed() {
    let negative = "fn F(b:view Bool){let r=MockOpen(1);match b{true=>{Drop(r);},false=>{}}Inspect(r);}fn Main(){}";
    let error = rejected(negative);
    assert_eq!(error.code, "NX-E0202");
    assert_eq!(error.subcause.as_deref(), Some("maybe_consumed"));
    assert_eq!(error.stage, Stage::Ownership);
    assert_eq!(error.primary_span, Some(span(79, 80)));
    assert!(error.data.is_empty());
    assert_eq!(
        error
            .related_spans
            .iter()
            .map(|related| (related.role, related.span.clone()))
            .collect::<Vec<_>>(),
        vec![
            (RelatedRole::Declaration, span(22, 23)),
            (RelatedRole::Consumption, span(51, 58))
        ]
    );
    assert_eq!(error.cause_chain.len(), 1);
    let cause = &error.cause_chain[0];
    assert_eq!(
        (
            cause.kind,
            cause.operation,
            cause.span.clone(),
            cause.evidence
        ),
        (
            CauseKind::Drop,
            CauseOperation::ValueDrop,
            Some(span(51, 58)),
            Evidence::Verified
        )
    );
    assert_eq!(
        cause
            .entity
            .as_ref()
            .map(|entity| (entity.kind, entity.id.as_str())),
        Some((EntityKind::Binding, "binding:22"))
    );
    unit_with_events(
        "fn F(b:view Bool){let r=MockOpen(1);match b{true=>{Inspect(r);},false=>{}}Inspect(r);}fn Main(){}",
        &[],
    );
}

#[test]
fn owner_extraction_and_cleanup_follow_selected_arm() {
    unit_with_events(
        "fn Main(){let o=Some(MockOpen(1));match move o{Some(move r)=>{Inspect(r);},None=>{}}}",
        &["open:0:1", "inspect:0:1", "close:0:1"],
    );
    unit_with_events(
        "fn Main(){let o:Option<MockResource>=None;match move o{Some(move r)=>{Inspect(r);},None=>{}}}",
        &[],
    );
    unit_with_events(
        "fn Main(){let o=Some(MockOpen(1));match move o{Some(_)=>{},None=>{}}}",
        &["open:0:1", "close:0:1"],
    );
}

#[test]
fn edit_place_and_payload_index_write_through() {
    unit_with_events(
        "fn Main(){var a=[Some(1)];match edit a[0]{Some(edit n)=>{n=2;},None=>{}}match a[0]{Some(n)=>{AssertEq(n,2);},None=>{}}}",
        &[],
    );
    unit_with_events(
        "fn Main(){var o=Some([1]);match edit o{Some(edit a)=>{a[0]=2;},None=>{}}match o{Some(a)=>{AssertEq(a[0],2);},None=>{}}}",
        &[],
    );
    unit_with_events(
        "fn Id()->I64{MockWriteI64(1);return 0;}fn Main(){var a=[Some(1)];match edit a[Id()]{Some(edit n)=>{n=2;},None=>{}}}",
        &["write_i64:1:err:-9000"],
    );
    unit_with_events(
        "fn Id()->I64{MockWriteI64(1);return 0;}fn Main(){let a=[Some(1)];match a[Id()]{Some(n)=>{AssertEq(n,1);},None=>{}}}",
        &["write_i64:1:err:-9000"],
    );
}

#[test]
fn owner_and_edit_modes_cannot_be_escalated() {
    let read_move =
        rejected("fn Main(){let o=Some(MockOpen(1));match o{Some(move r)=>{},None=>{}}}");
    assert_eq!(
        (read_move.code.as_str(), read_move.subcause.as_deref()),
        ("NX-E1105", Some("permission_escalation"))
    );
    let edit_move =
        rejected("fn Main(){var o=Some(MockOpen(1));match edit o{Some(move r)=>{},None=>{}}}");
    assert_eq!(
        (edit_move.code.as_str(), edit_move.subcause.as_deref()),
        ("NX-E1105", Some("permission_escalation"))
    );
    unit_with_events(
        "fn Main(){let o=Some(MockOpen(1));match o{Some(r)=>{Inspect(r);},None=>{}}}",
        &["open:0:1", "inspect:0:1", "close:0:1"],
    );
}

#[test]
fn return_and_propagation_clean_extracted_owner_once() {
    unit_with_events(
        "fn F()->Result<(),I64>{let o=Some(MockOpen(1));match move o{Some(move r)=>{return Ok(());},None=>{}}return Ok(());}fn Main(){let x=F();Drop(x);}",
        &["open:0:1", "close:0:1"],
    );
    unit_with_events(
        "fn F()->Result<(),I64>{let o=Some(MockOpen(1));match move o{Some(move r)=>{let x:Result<(),I64>=Err(1);x?;},None=>{}}return Ok(());}fn Main(){let x=F();Drop(x);}",
        &["open:0:1", "close:0:1"],
    );
}

#[test]
fn returning_arm_does_not_consume_the_continuing_branch() {
    unit_with_events(
        "fn F(b:view Bool){let r=MockOpen(1);match b{true=>{Drop(r);return;},false=>{}}Inspect(r);}fn Main(){F(true);F(false);}",
        &[
            "open:0:1",
            "close:0:1",
            "open:1:1",
            "inspect:1:1",
            "close:1:1",
        ],
    );
}

#[test]
fn consumption_in_both_arms_is_definite_and_reinitialization_repairs_it() {
    let negative = "fn F(b:view Bool){let r=MockOpen(1);match b{true=>{Drop(r);},false=>{Drop(r);}}Inspect(r);}fn Main(){}";
    let error = rejected(negative);
    assert_eq!(
        (error.code.as_str(), error.subcause.as_deref()),
        ("NX-E0202", Some("definitely_consumed"))
    );
    assert_eq!(error.primary_span, Some(span(87, 88)));
    assert!(error.data.is_empty());
    unit_with_events(
        "fn F(b:view Bool){var r=MockOpen(1);match b{true=>{Drop(r);r=MockOpen(2);},false=>{Drop(r);r=MockOpen(3);}}Inspect(r);}fn Main(){F(true);F(false);}",
        &[
            "open:0:1",
            "close:0:1",
            "open:1:2",
            "inspect:1:2",
            "close:1:2",
            "open:2:1",
            "close:2:1",
            "open:3:3",
            "inspect:3:3",
            "close:3:3",
        ],
    );
}

#[test]
fn owner_selection_consumes_the_original_binding() {
    let negative = "fn Main(){let o=Some(MockOpen(1));match move o{Some(move r)=>{Inspect(r);},None=>{}}Drop(o);}";
    let error = rejected(negative);
    assert_eq!(
        (error.code.as_str(), error.subcause.as_deref()),
        ("NX-E0202", Some("definitely_consumed"))
    );
    assert_eq!(error.primary_span, Some(span(89, 90)));
    assert!(error.data.is_empty());
    assert_eq!(
        error
            .related_spans
            .iter()
            .map(|related| (related.role, related.span.clone()))
            .collect::<Vec<_>>(),
        vec![
            (RelatedRole::Declaration, span(14, 15)),
            (RelatedRole::Consumption, span(40, 44))
        ]
    );
    unit_with_events(
        "fn Main(){let o=Some(MockOpen(1));match move o{Some(move r)=>{Inspect(r);},None=>{}}}",
        &["open:0:1", "inspect:0:1", "close:0:1"],
    );
}

#[test]
fn option_objective_programs_have_the_published_unit_result() {
    for file in ["12_option_match.n", "13_option_none.n"] {
        let path = format!("corpus/objetivo/{file}");
        let source = std::fs::read_to_string(&path).unwrap();
        unit_with_events(&source, &[]);
    }
}
