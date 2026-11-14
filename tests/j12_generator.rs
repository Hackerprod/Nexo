use nexo::interp::{MockEnvironment, Outcome};
use nexo::ir::CompileError;

fn diagnostic(source: &str) -> nexo::diagnostic::Diagnostic {
    match nexo::ir::compile_for_internal_harness("<memory>", source, true).unwrap_err() {
        CompileError::Rejected(mut diagnostics) => {
            assert_eq!(diagnostics.len(), 1, "{source}");
            *diagnostics.remove(0)
        }
        other => panic!("unexpected compiler failure: {other:?}"),
    }
}

#[test]
fn generator_contracts_require_resume_and_reject_call() {
    let cases = [
        (
            "gen fn G()->I64 with {create(no_io)}{}fn Main(){}",
            "missing_body_operation",
            (21, 36),
            "gen fn G()->I64 with {create(no_io),resume(unrestricted)}{}fn Main(){}",
        ),
        (
            "gen fn G() with {call(unrestricted)}{}fn Main(){}",
            "invalid_operation",
            (17, 21),
            "gen fn G() with {resume(unrestricted)}{}fn Main(){}",
        ),
    ];
    for (negative, subcause, span, positive) in cases {
        let error = diagnostic(negative);
        assert_eq!(error.code, "NX-E0417");
        assert_eq!(error.subcause.as_deref(), Some(subcause));
        let primary = error.primary_span.unwrap();
        assert_eq!((primary.start, primary.end), span);
        let result =
            nexo::ir::run_for_internal_harness("<memory>", positive, MockEnvironment::default())
                .unwrap();
        assert_eq!(result.outcome, Outcome::Unit);
    }
}

#[test]
fn yield_and_next_contribute_suspend_to_written_contracts() {
    let cases = [
        (
            "gen fn G()->I64 with no_suspend{yield 1;}fn Main(){}",
            (32, 37),
            "gen fn G()->I64 with no_suspend{}fn Main(){}",
        ),
        (
            "gen fn G()->I64{yield 1;}fn Main() with no_suspend{var g=G();let x=Iterator.Next(edit g);}",
            (76, 80),
            "gen fn G()->I64{yield 1;}fn Main(){var g=G();let x=Iterator.Next(edit g);}",
        ),
        (
            "gen fn G()->I64{yield 1;}fn Main(){let g=G();let f=fn [move g]() with no_suspend{var h=move g;let n=Iterator.Next(edit h);};}",
            (109, 113),
            "gen fn G()->I64{}fn Main(){let g=G();let f=fn [move g]() with no_suspend{var h=move g;let n=Iterator.Next(edit h);};}",
        ),
    ];
    for (negative, span, positive) in cases {
        let error = diagnostic(negative);
        assert_eq!(error.code, "NX-E0414", "{negative}");
        let primary = error.primary_span.unwrap();
        assert_eq!((primary.start, primary.end), span, "{negative}");
        let result =
            nexo::ir::run_for_internal_harness("<memory>", positive, MockEnvironment::default())
                .unwrap();
        assert_eq!(result.outcome, Outcome::Unit);
    }
}

#[test]
fn retained_edit_capture_must_end_before_yield() {
    let negative =
        "gen fn G()->I64{var x=0;var f=fn [edit x](){x=1;};yield 1;edit f();}fn Main(){}";
    let error = diagnostic(negative);
    assert_eq!(error.code, "NX-E1204");
    assert_eq!(
        (
            error.primary_span.as_ref().unwrap().start,
            error.primary_span.as_ref().unwrap().end
        ),
        (50, 55)
    );
    assert_eq!(
        error.data.get("property_evidence"),
        Some(&nexo::diagnostic::DataValue::String("incompatible".into()))
    );
    assert_eq!(
        error.data.get("suspension_kind"),
        Some(&nexo::diagnostic::DataValue::String("yield".into()))
    );
    let positive =
        "gen fn G()->I64{var x=0;var f=fn [edit x](){x=1;};edit f();Drop(f);yield 1;}fn Main(){}";
    let result =
        nexo::ir::run_for_internal_harness("<memory>", positive, MockEnvironment::default())
            .unwrap();
    assert_eq!(result.outcome, Outcome::Unit);
}

#[test]
fn suspend_safety_follows_wrapped_closure_permissions() {
    let negative = "gen fn G()->I64{var x=0;let o=Some(fn [edit x](){x=1;});yield 1;}fn Main(){}";
    let error = diagnostic(negative);
    assert_eq!(error.code, "NX-E1204");
    let carrier = negative.find("o=Some").unwrap() as u64;
    assert!(error.related_spans.iter().any(|related| {
        related.role == nexo::diagnostic::RelatedRole::Declaration
            && (related.span.start, related.span.end) == (carrier, carrier + 1)
    }));
    assert_eq!(
        error.cause_chain[0].entity.as_ref().unwrap().id,
        format!("binding:{carrier}")
    );
    assert_eq!(
        error.data.get("property_evidence"),
        Some(&nexo::diagnostic::DataValue::String("incompatible".into()))
    );
    let positive =
        "gen fn G()->I64{var x=0;let o=Some(fn [edit x](){x=1;});Drop(o);yield 1;}fn Main(){}";
    assert_eq!(
        nexo::ir::run_for_internal_harness("<memory>", positive, MockEnvironment::default())
            .unwrap()
            .outcome,
        Outcome::Unit
    );
    let read = "gen fn G()->I64{let x=1;let f=fn [view x]()->I64{return x;};yield 1;AssertEq(f(),1);}fn Main(){var g=G();AssertEq(Iterator.Next(edit g),Some(1));}";
    assert_eq!(
        nexo::ir::run_for_internal_harness("<memory>", read, MockEnvironment::default())
            .unwrap()
            .outcome,
        Outcome::Unit
    );
}

#[test]
fn generator_drop_contract_includes_retained_blocking_resource() {
    let negative = "gen fn G(r:move MockBlockingResource)->I64 with {resume(unrestricted),drop(no_block)}{}fn Main(){}";
    let error = diagnostic(negative);
    assert_eq!(error.code, "NX-E0413");
    assert_eq!(error.operation, nexo::diagnostic::Operation::ValueDrop);
    assert_eq!(
        error.data.get("required_effect"),
        Some(&nexo::diagnostic::DataValue::String("block".into()))
    );
    let positive = "gen fn G(r:move MockResource)->I64 with {resume(unrestricted),drop(no_block)}{}fn Main(){}";
    assert_eq!(
        nexo::ir::run_for_internal_harness("<memory>", positive, MockEnvironment::default())
            .unwrap()
            .outcome,
        Outcome::Unit
    );
}

#[test]
fn implicit_generator_cleanup_propagates_blocking_drop_to_owner() {
    let negative = "gen fn G(r:move MockBlockingResource){}fn F(r:move MockBlockingResource) with no_block{let g=G(move r);}fn Main(){}";
    let error = diagnostic(negative);
    assert_eq!(error.code, "NX-E0413");
    let positive = "gen fn G(r:move MockResource){}fn F(r:move MockResource) with no_block{let g=G(move r);}fn Main(){}";
    assert_eq!(
        nexo::ir::run_for_internal_harness("<memory>", positive, MockEnvironment::default())
            .unwrap()
            .outcome,
        Outcome::Unit
    );
}

#[test]
fn generator_drop_effect_transits_through_an_owning_closure() {
    let negative = "gen fn G(r:move MockBlockingResource)->I64{yield 1;}fn Main() with no_block{let g=G(MockBlockingOpen(1));let f=fn [move g](){};Drop(f);}";
    let result = nexo::ir::compile_for_internal_harness("<memory>", negative, true);
    let nexo::ir::CompileError::Rejected(errors) = result.unwrap_err() else {
        panic!("wrong error");
    };
    assert!(
        errors.iter().any(|error| error.code == "NX-E0413"),
        "{errors:?}"
    );
    let positive = negative.replace(" with no_block", "");
    assert_eq!(
        nexo::ir::run_for_internal_harness("<memory>", &positive, MockEnvironment::default())
            .unwrap()
            .outcome,
        Outcome::Unit
    );
}
