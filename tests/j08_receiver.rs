use nexo::ir::{CompileError, compile_for_internal_harness};

#[test]
fn named_function_receiver_is_a_names_limit() {
    for source in [
        "fn Foo(){}fn Main(){edit Foo();}",
        "fn Foo(){}fn Main(){move Foo();}",
    ] {
        let Err(CompileError::Rejected(errors)) =
            compile_for_internal_harness("receiver.n", source, true)
        else {
            panic!("ordinary functions have no receiver: {source}");
        };
        assert_eq!(errors.len(), 1);
        let error = &errors[0];
        assert_eq!(error.code, "NX-E0006");
        assert_eq!(error.feature_id.as_deref(), Some("receiver_mode"));
        assert_eq!(error.stage.as_str(), "names");
        assert_eq!(error.operation.as_str(), "names.resolve");
        error.validate("receiver.n", source).unwrap();
    }
}

#[test]
fn read_mode_cannot_call_mutating_or_consuming_closure() {
    for (source, required, primary) in [
        (
            "fn Main(){var x=1;var f=fn [edit x](){x=2;};f();}",
            "inout",
            (44, 45),
        ),
        (
            "fn Main(){let r=MockOpen(1);let f=fn [move r](){Drop(r);};f();}",
            "consume",
            (58, 59),
        ),
    ] {
        let Err(CompileError::Rejected(errors)) =
            compile_for_internal_harness("receiver-mode.n", source, true)
        else {
            panic!("receiver mode must reject: {source}");
        };
        assert_eq!(errors.len(), 1, "{source}: {errors:?}");
        let error = &errors[0];
        assert_eq!(error.code, "NX-E1107", "{source}");
        assert_eq!(
            error.primary_span.as_ref().map(|s| (s.start, s.end)),
            Some(primary)
        );
        assert_eq!(
            error.data.get("required_mode"),
            Some(&nexo::diagnostic::DataValue::String(required.into()))
        );
        assert_eq!(
            error.data.get("requested_mode"),
            Some(&nexo::diagnostic::DataValue::String("read".into()))
        );
        error.validate("receiver-mode.n", source).unwrap();
    }
}

#[test]
fn mutable_receiver_updates_captured_origin() {
    let source = "fn Main(){var x=1;var f=fn [edit x](){x=2;};edit f();Drop(f);AssertEq(x,2);}";
    let program = compile_for_internal_harness("c05-positive.n", source, true).unwrap();
    let execution = nexo::interp::run_scalar(&program, Default::default());
    assert_eq!(execution.outcome, nexo::interp::Outcome::Unit);
    assert!(
        execution.diagnostics.is_empty(),
        "{:?}",
        execution.diagnostics
    );
}

#[test]
fn consuming_receiver_closes_owned_field_once() {
    let source = "fn Main(){let r=MockOpen(1);let f=fn [move r](){Drop(r);};move f();}";
    let program = compile_for_internal_harness("c09-positive.n", source, true).unwrap();
    let execution = nexo::interp::run_scalar(&program, Default::default());
    assert_eq!(execution.outcome, nexo::interp::Outcome::Unit);
    assert!(
        execution.diagnostics.is_empty(),
        "{:?}",
        execution.diagnostics
    );
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
fn consumed_closure_cannot_be_called_again() {
    let source = "fn Main(){let f=fn [](){};move f();f();}";
    let Err(CompileError::Rejected(errors)) = compile_for_internal_harness("c08.n", source, true)
    else {
        panic!("the second call must see the consumed receiver");
    };
    assert_eq!(errors.len(), 1);
    assert_eq!(errors[0].code, "NX-E0202");
    assert_eq!(errors[0].subcause.as_deref(), Some("definitely_consumed"));
    assert_eq!(
        errors[0].primary_span.as_ref().map(|s| (s.start, s.end)),
        Some((35, 36))
    );
    errors[0].validate("c08.n", source).unwrap();
}

#[test]
fn owned_resource_can_still_use_read_receiver_twice() {
    let source = "fn Main(){let r=MockOpen(1);let f=fn [move r](){Inspect(r);};f();f();}";
    let program = compile_for_internal_harness("resource-call.n", source, true).unwrap();
    let execution = nexo::interp::run_scalar(&program, Default::default());
    assert_eq!(execution.outcome, nexo::interp::Outcome::Unit);
    assert!(
        execution.diagnostics.is_empty(),
        "{:?}",
        execution.diagnostics
    );
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
fn stronger_explicit_modes_are_allowed_and_edit_needs_var() {
    let valid = "fn Main(){var f=fn [](){};edit f();move f();}";
    let program = compile_for_internal_harness("strong-receiver.n", valid, true).unwrap();
    let execution = nexo::interp::run_scalar(&program, Default::default());
    assert_eq!(execution.outcome, nexo::interp::Outcome::Unit);
    assert!(execution.diagnostics.is_empty());

    let invalid = "fn Main(){var x=1;let f=fn [edit x](){x=2;};edit f();}";
    let Err(CompileError::Rejected(errors)) =
        compile_for_internal_harness("immutable-receiver.n", invalid, true)
    else {
        panic!("edit requires a mutable receiver binding");
    };
    assert_eq!(errors.len(), 1);
    assert_eq!(errors[0].code, "NX-E0105");
    errors[0].validate("immutable-receiver.n", invalid).unwrap();
}

#[test]
fn closure_edit_argument_returns_to_callers_place() {
    let source = "fn Main(){var x=1;let f=fn [](v:edit I64){v=2;};f(edit x);AssertEq(x,2);}";
    let program = compile_for_internal_harness("edit-argument.n", source, true).unwrap();
    let execution = nexo::interp::run_scalar(&program, Default::default());
    assert_eq!(execution.outcome, nexo::interp::Outcome::Unit);
    assert!(
        execution.diagnostics.is_empty(),
        "{:?}",
        execution.diagnostics
    );
}

#[test]
fn prefixed_temporary_receiver_requires_a_place() {
    for source in [
        "fn Main(){edit (fn [](){})();}",
        "fn Main(){move (fn [](){})();}",
    ] {
        let Err(CompileError::Rejected(errors)) =
            compile_for_internal_harness("temporary.n", source, true)
        else {
            panic!("temporary cannot be a prefixed receiver: {source}");
        };
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].code, "NX-E0002");
        errors[0].validate("temporary.n", source).unwrap();
    }
}
