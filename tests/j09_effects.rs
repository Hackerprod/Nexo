mod common;
use nexo::ir::CompileError;

#[test]
fn closure_map_requires_call_operation() {
    let source = "fn Main(){let f=fn []() with {drop(no_io)}{};}";
    let Err(CompileError::Rejected(errors)) = common::compile_j("w09.n", source, true) else {
        panic!("W09 must reject a map without call");
    };
    assert_eq!(errors.len(), 1);
    assert_eq!(errors[0].code, "NX-E0417");
    assert_eq!(
        errors[0].subcause.as_deref(),
        Some("missing_body_operation")
    );
    assert_eq!(
        errors[0].primary_span.as_ref().map(|s| (s.start, s.end)),
        Some((29, 42))
    );
    errors[0].validate("w09.n", source).unwrap();
}

#[test]
fn written_closure_operation_bounds_use_capture_witnesses() {
    let cases = [
        (
            "fn Main(){let a=[1];let f=fn [copy a]() with {create(no_alloc),call(unrestricted)}{};}",
            "NX-E0412",
            "closure.create",
            (26, 28),
            (53, 61),
            "alloc",
            1,
        ),
        (
            "fn Main(){let a=[1];let f=fn [move a]() with {copy(no_alloc),call(unrestricted)}{};}",
            "NX-E0412",
            "value.copy",
            (26, 28),
            (51, 59),
            "alloc",
            1,
        ),
        (
            "fn Main(){let r=MockBlockingOpen(1);let f=fn [move r]() with {drop(no_block),call(unrestricted)}{};}",
            "NX-E0413",
            "value.drop",
            (42, 44),
            (67, 75),
            "block",
            2,
        ),
        (
            "fn Main(){let r=MockOpen(1);let f=fn [move r]() with {drop(no_io),call(unrestricted)}{};}",
            "NX-E0415",
            "value.drop",
            (34, 36),
            (59, 64),
            "io",
            2,
        ),
    ];
    for (source, code, operation, primary, contract, effect, causes) in cases {
        let Err(CompileError::Rejected(errors)) = common::compile_j("effects.n", source, true)
        else {
            panic!("the operation bound must reject: {source}");
        };
        assert_eq!(errors.len(), 1, "{source}: {errors:?}");
        let error = &errors[0];
        assert_eq!(error.code, code);
        assert_eq!(error.operation.as_str(), operation);
        assert_eq!(
            error.primary_span.as_ref().map(|s| (s.start, s.end)),
            Some(primary)
        );
        assert_eq!(error.related_spans.len(), 1);
        assert_eq!(
            (
                error.related_spans[0].span.start,
                error.related_spans[0].span.end
            ),
            contract
        );
        assert_eq!(error.subcause, None);
        assert_eq!(error.feature_id, None);
        assert_eq!(error.cause_chain.len(), causes);
        assert_eq!(
            error.cause_chain[0].entity.as_ref().map(|e| e.id.as_str()),
            Some("binding:14")
        );
        use nexo::diagnostic::DataValue;
        assert_eq!(
            error.data,
            [
                (
                    "effect_model".into(),
                    DataValue::String("f0-0003-sim-v1".into())
                ),
                (
                    "effect_evidence".into(),
                    DataValue::String("known_effect".into())
                ),
                ("required_effect".into(), DataValue::String(effect.into())),
                (
                    "contract_origin".into(),
                    DataValue::Object(
                        [
                            ("file".into(), DataValue::String("effects.n".into())),
                            ("start".into(), DataValue::UInt(contract.0)),
                            ("end".into(), DataValue::UInt(contract.1)),
                        ]
                        .into()
                    )
                ),
            ]
            .into()
        );
        error.validate("effects.n", source).unwrap();
    }
}

#[test]
fn latent_body_is_separate_from_creation_and_drop() {
    let latent = "fn F() with no_io{let f=fn [](){Print(1);};}fn Main(){F();}";
    let program = common::compile_j("latent.n", latent, true).unwrap();
    let execution = nexo::interp::run_scalar(&program, Default::default());
    assert_eq!(execution.outcome, nexo::interp::Outcome::Unit);
    assert!(execution.diagnostics.is_empty());
    assert!(execution.observations.observations().is_empty());

    let invoked = "fn F() with no_io{let f=fn [](){Print(1);};f();}fn Main(){}";
    let Err(CompileError::Rejected(errors)) = common::compile_j("invoked.n", invoked, false) else {
        panic!("invocation makes the body effect visible");
    };
    assert_eq!(errors.len(), 1);
    assert_eq!(errors[0].code, "NX-E0415");
    errors[0].validate("invoked.n", invoked).unwrap();
}

#[test]
fn owned_capture_destructor_is_part_of_enclosing_function() {
    let source = "fn F(r:move MockResource) with no_io{let f=fn [move r](){};}fn Main(){}";
    let Err(CompileError::Rejected(errors)) = common::compile_j("drop-effect.n", source, false)
    else {
        panic!("the closure destructor performs IO");
    };
    assert_eq!(errors.len(), 1);
    assert_eq!(errors[0].code, "NX-E0415");
    errors[0].validate("drop-effect.n", source).unwrap();
}

#[test]
fn unknown_call_effect_stays_latent_until_invocation() {
    let latent = "fn Main(){let f=fn []() with {call(no_io)}{MockUnknown();};}";
    let Err(CompileError::Rejected(errors)) = common::compile_j("unknown-bound.n", latent, false)
    else {
        panic!("unknown call effect cannot satisfy no_io");
    };
    assert_eq!(errors.len(), 1);
    assert_eq!(errors[0].code, "NX-E0415");
    assert_eq!(
        errors[0].data.get("effect_evidence"),
        Some(&nexo::diagnostic::DataValue::String(
            "unknown_effect".into()
        ))
    );
    errors[0].validate("unknown-bound.n", latent).unwrap();

    let unbounded = "fn F() with no_io{let f=fn [](){MockUnknown();};}fn Main(){}";
    common::compile_j("unknown-latent.n", unbounded, false).unwrap();
}

#[test]
fn function_bound_sees_invoked_closure_body() {
    let source = "fn F() with no_io{let f=fn [](){Print(1);};f();}fn Main(){}";
    let Err(CompileError::Rejected(errors)) =
        common::compile_j("closure-call-effect.n", source, false)
    else {
        panic!("no_io must include the called body");
    };
    assert_eq!(errors.len(), 1);
    assert_eq!(errors[0].code, "NX-E0415");
    assert_eq!(errors[0].operation.as_str(), "closure.call");
    assert!(
        errors[0]
            .cause_chain
            .iter()
            .any(|cause| cause.operation.as_str() == "closure.call")
    );
    errors[0].validate("closure-call-effect.n", source).unwrap();
}

#[test]
fn operation_bounds_accept_the_adr_repairs() {
    let sources = [
        "fn Main(){let a=[1];let f=fn [move a]() with {create(no_alloc),call(unrestricted)}{};}",
        "fn Main(){let a=1;let f=fn [move a]() with {copy(no_alloc),call(unrestricted)}{};}",
        "fn Main(){let r=MockOpen(1);let f=fn [move r]() with {drop(no_block),call(unrestricted)}{};}",
        "fn Main(){let r=1;let f=fn [move r]() with {drop(no_io),call(unrestricted)}{};}",
    ];
    for source in sources {
        common::compile_j("repair.n", source, true).unwrap();
    }
}

#[test]
fn recursive_function_closure_cycle_propagates_effect() {
    let source = "fn F() with no_io{let f=fn [](){Print(1);F();};f();}fn Main(){}";
    let Err(CompileError::Rejected(errors)) = common::compile_j("cycle.n", source, false) else {
        panic!("cycle must propagate the closure body effect");
    };
    assert_eq!(errors.len(), 1);
    assert_eq!(errors[0].code, "NX-E0415");
    errors[0].validate("cycle.n", source).unwrap();
}

#[test]
fn nested_owned_closure_drop_is_visible_in_outer_contract() {
    let source = "fn Main(){let r=MockOpen(1);let inner=fn [move r](){};let outer=fn [move inner]() with {drop(no_io),call(unrestricted)}{};}";
    let Err(CompileError::Rejected(errors)) = common::compile_j("nested-drop.n", source, true)
    else {
        panic!("the outer environment owns the inner resource");
    };
    assert_eq!(errors.len(), 1);
    assert_eq!(errors[0].code, "NX-E0415");
    errors[0].validate("nested-drop.n", source).unwrap();
}
