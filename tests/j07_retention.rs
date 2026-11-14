mod common;
use nexo::ir::CompileError;

#[test]
fn retained_view_blocks_write_until_carrier_drops() {
    let source = "fn Main(){var x=1;let f=fn [view x](){};x=2;Drop(f);}";
    let Err(CompileError::Rejected(errors)) = common::compile_j("c04.n", source, true) else {
        panic!("C04 must reject the write");
    };
    assert_eq!(errors.len(), 1);
    let error = &errors[0];
    assert_eq!(error.code, "NX-E1105");
    assert_eq!(error.subcause.as_deref(), Some("active_borrow_conflict"));
    assert_eq!(
        error.primary_span.as_ref().map(|s| (s.start, s.end)),
        Some((40, 41))
    );
    error.validate("c04.n", source).unwrap();
}

#[test]
fn borrowed_closure_cannot_escape_its_origin_scope() {
    let source = "fn Main(){let f=if true then {let x=1;value:fn [view x]()->I64{return x;};}else{return;};Drop(f);}";
    let Err(CompileError::Rejected(errors)) = common::compile_j("c06.n", source, true) else {
        panic!("C06 must reject the escaping closure");
    };
    assert_eq!(errors.len(), 1);
    let error = &errors[0];
    assert_eq!(error.code, "NX-E1104");
    assert_eq!(
        error.primary_span.as_ref().map(|s| (s.start, s.end)),
        Some((44, 73))
    );
    error.validate("c06.n", source).unwrap();
}

#[test]
fn dropping_reader_releases_the_source() {
    let source = "fn Main(){var x=1;let f=fn [view x](){};Drop(f);x=2;}";
    let program = common::compile_j("c04-positive.n", source, true).unwrap();
    let execution = nexo::interp::run_scalar(&program, Default::default());
    assert_eq!(execution.outcome, nexo::interp::Outcome::Unit);
    assert!(
        execution.diagnostics.is_empty(),
        "{:?}",
        execution.diagnostics
    );
    assert!(execution.observations.observations().is_empty());
}

#[test]
fn retained_reader_can_call_and_copy_keeps_second_loan() {
    let valid = "fn Main(){var x=1;let f=fn [view x]()->I64{return x;};AssertEq(f(),1);let g=f;Drop(f);Drop(g);x=2;}";
    let program = common::compile_j("readers.n", valid, true).unwrap();
    let execution = nexo::interp::run_scalar(&program, Default::default());
    assert_eq!(execution.outcome, nexo::interp::Outcome::Unit);
    assert!(
        execution.diagnostics.is_empty(),
        "{:?}",
        execution.diagnostics
    );

    let invalid = "fn Main(){var x=1;let f=fn [view x](){};let g=f;Drop(f);x=2;Drop(g);}";
    let Err(CompileError::Rejected(errors)) =
        common::compile_j("readers-negative.n", invalid, true)
    else {
        panic!("the remaining reader must block the write");
    };
    assert_eq!(errors.len(), 1);
    assert_eq!(errors[0].code, "NX-E1105");
    errors[0].validate("readers-negative.n", invalid).unwrap();
}

#[test]
fn exclusive_capture_releases_on_drop_and_cannot_copy() {
    let valid = "fn Main(){var x=1;let f=fn [edit x](){x=2;};Drop(f);x=3;}";
    let program = common::compile_j("edit-positive.n", valid, true).unwrap();
    let execution = nexo::interp::run_scalar(&program, Default::default());
    assert_eq!(execution.outcome, nexo::interp::Outcome::Unit);
    assert!(execution.diagnostics.is_empty());

    let invalid = "fn Main(){var x=1;let f=fn [edit x](){x=2;};let g=f;}";
    let Err(CompileError::Rejected(errors)) = common::compile_j("edit-copy.n", invalid, true)
    else {
        panic!("exclusive closure cannot copy");
    };
    assert_eq!(errors.len(), 1);
    assert_eq!(errors[0].code, "NX-E0201");
    errors[0].validate("edit-copy.n", invalid).unwrap();
}

#[test]
fn option_carrier_keeps_and_releases_reader() {
    let valid = "fn Main(){var x=1;let f=fn [view x](){};let o=Some(f);Drop(f);Drop(o);x=2;}";
    let program = common::compile_j("option-reader.n", valid, true).unwrap();
    let execution = nexo::interp::run_scalar(&program, Default::default());
    assert_eq!(execution.outcome, nexo::interp::Outcome::Unit);
    assert!(
        execution.diagnostics.is_empty(),
        "{:?}",
        execution.diagnostics
    );

    let invalid = "fn Main(){var x=1;let f=fn [view x](){};let o=Some(f);Drop(f);x=2;Drop(o);}";
    let Err(CompileError::Rejected(errors)) =
        common::compile_j("option-reader-negative.n", invalid, true)
    else {
        panic!("Option still carries the reader");
    };
    assert_eq!(errors.len(), 1);
    assert_eq!(errors[0].code, "NX-E1105");
    errors[0]
        .validate("option-reader-negative.n", invalid)
        .unwrap();
}

#[test]
fn inferred_reader_and_resource_reader_preserve_origins() {
    let inferred = "fn Main(){var x=1;let f=fn ()->I64{return x;};x=2;Drop(f);}";
    let Err(CompileError::Rejected(errors)) =
        common::compile_j("inferred-reader.n", inferred, true)
    else {
        panic!("inferred view must retain its loan");
    };
    assert_eq!(errors.len(), 1);
    assert_eq!(errors[0].code, "NX-E1105");
    errors[0].validate("inferred-reader.n", inferred).unwrap();

    let resource = "fn Main(){let r=MockOpen(1);let f=fn [view r](){Inspect(r);};let g=f;Drop(f);Drop(g);Drop(r);}";
    let program = common::compile_j("resource-reader.n", resource, true).unwrap();
    let execution = nexo::interp::run_scalar(&program, Default::default());
    assert_eq!(execution.outcome, nexo::interp::Outcome::Unit);
    assert_eq!(
        execution
            .observations
            .events()
            .iter()
            .filter(|e| e.starts_with("close:"))
            .count(),
        1
    );
}

#[test]
fn optional_borrow_cannot_escape_inner_binding() {
    let source = "fn Main(){let o=if true then {let x=1;value:Some(fn [view x]()->I64{return x;});}else{return;};Drop(o);}";
    let Err(CompileError::Rejected(errors)) = common::compile_j("optional-escape.n", source, true)
    else {
        panic!("Option cannot hide the escaping loan");
    };
    assert_eq!(errors.len(), 1);
    assert_eq!(errors[0].code, "NX-E1104");
    errors[0].validate("optional-escape.n", source).unwrap();
}

#[test]
fn retained_edit_freezes_direct_access_until_drop() {
    let invalid = "fn Main(){var x=1;let f=fn [edit x](){x=2;};x=3;}";
    let Err(CompileError::Rejected(errors)) = common::compile_j("edit-freeze.n", invalid, true)
    else {
        panic!("direct write must wait for the retained edit loan");
    };
    assert_eq!(errors.len(), 1);
    assert_eq!(errors[0].code, "NX-E1105");
    errors[0].validate("edit-freeze.n", invalid).unwrap();

    let valid = "fn Main(){var x=1;let f=fn [edit x](){x=2;};Drop(f);x=3;}";
    common::compile_j("edit-unfreeze.n", valid, true).unwrap();
}

#[test]
fn nested_edit_capture_reborrows_parent_permission() {
    let source = "fn Main(){var x=1;var f=fn [edit x](){var g=fn [edit x](){x=2;};edit g();};edit f();Drop(f);AssertEq(x,2);}";
    let program = common::compile_j("nested-edit.n", source, true).unwrap();
    let execution = nexo::interp::run_scalar(&program, Default::default());
    assert_eq!(execution.outcome, nexo::interp::Outcome::Unit);
    assert!(
        execution.diagnostics.is_empty(),
        "{:?}",
        execution.diagnostics
    );
}
