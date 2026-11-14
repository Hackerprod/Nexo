use nexo::diagnostic::{CauseOperation, DataValue, RelatedRole};
use nexo::ir::CompileError;

fn rejected(source: &str) -> nexo::diagnostic::Diagnostic {
    let CompileError::Rejected(mut errors) =
        nexo::ir::compile_for_internal_harness("<memory>", source, true).unwrap_err()
    else {
        panic!("compiler failed without a source diagnostic: {source}")
    };
    assert_eq!(errors.len(), 1, "{source}: {errors:?}");
    *errors.remove(0)
}

#[test]
fn for_edit_keeps_permanent_feature_exclusion() {
    let error = rejected("gen fn G()->I64{yield 1;}fn Main(){for edit x in G(){}}");
    assert_eq!(error.code, "NX-E0006");
    assert_eq!(error.feature_id.as_deref(), Some("for_iteration"));
    assert_eq!(
        error.data.get("pending_scope"),
        Some(&DataValue::String("later_adr".into()))
    );
    let span = error.primary_span.unwrap();
    assert_eq!((span.start, span.end), (39, 43));
    assert!(
        nexo::ir::compile_for_internal_harness(
            "<memory>",
            "gen fn G()->I64{yield 1;}fn Main(){for x in G(){}}",
            true
        )
        .is_ok()
    );
}

#[test]
fn inferred_generator_result_reports_return_value() {
    let error = rejected("gen fn G(){return 1;}fn Main(){}");
    assert_eq!(error.code, "NX-E1324");
    assert!(error.related_spans.is_empty());
    let span = error.primary_span.unwrap();
    assert_eq!((span.start, span.end), (11, 17));
    assert!(
        nexo::ir::compile_for_internal_harness("<memory>", "gen fn G(){return;}fn Main(){}", true)
            .is_ok()
    );
}

#[test]
fn for_iterable_domain_reports_concrete_option() {
    let error = rejected("fn Main(){for x in Some(1){}}");
    assert_eq!(error.code, "NX-E0102");
    assert_eq!(error.subcause.as_deref(), Some("type_mismatch"));
    assert_eq!(
        error.data.get("expected_type"),
        Some(&DataValue::String("domain:iterable".into()))
    );
    assert_eq!(
        error.data.get("found_type"),
        Some(&DataValue::String("Option<I64>".into()))
    );
    assert!(
        nexo::ir::compile_for_internal_harness(
            "<memory>",
            "gen fn G()->I64{yield 1;}fn Main(){for x in G(){}}",
            true
        )
        .is_ok()
    );
    for (iterable, code) in [
        ("1", "NX-E0102"),
        ("true", "NX-E0102"),
        ("()", "NX-E0102"),
        ("None", "NX-E0113"),
        ("Ok(1)", "NX-E0113"),
    ] {
        let source = format!("fn Main(){{for x in {iterable}{{}}}}");
        assert_eq!(rejected(&source).code, code, "{source}");
    }
}

#[test]
fn generator_result_relation_covers_arrow_and_type() {
    let error = rejected("gen fn G()->I64{return 1;}fn Main(){}");
    assert_eq!(error.code, "NX-E1324");
    assert!(
        error
            .related_spans
            .iter()
            .any(|related| related.role == RelatedRole::ExpectedType
                && (related.span.start, related.span.end) == (10, 15))
    );
    assert!(
        nexo::ir::compile_for_internal_harness(
            "<memory>",
            "gen fn G()->I64{return;}fn Main(){}",
            true
        )
        .is_ok()
    );
}

#[test]
fn yield_safety_names_the_carrier_binding() {
    let error =
        rejected("gen fn G()->I64{var x=0;var f=fn [edit x](){x=1;};yield 1;edit f();}fn Main(){}");
    assert_eq!(error.code, "NX-E1204");
    assert!(
        error
            .related_spans
            .iter()
            .any(|related| related.role == RelatedRole::Declaration
                && (related.span.start, related.span.end) == (28, 29))
    );
    let cause = error
        .cause_chain
        .iter()
        .find(|cause| cause.operation == CauseOperation::SuspendSafe)
        .unwrap();
    assert_eq!(cause.entity.as_ref().unwrap().id, "binding:28");
    assert!(nexo::ir::compile_for_internal_harness("<memory>", "gen fn G()->I64{var x=0;var f=fn [edit x](){x=1;};edit f();Drop(f);yield 1;}fn Main(){}", true).is_ok());
}

#[test]
fn consuming_closure_call_has_schema_valid_effect_cause() {
    let error = rejected(
        "fn F(p0:move MockResource) with no_io{let f=fn [move p0](){};move f();}fn Main(){}",
    );
    assert_eq!(error.code, "NX-E0415");
    assert!(
        error
            .validate(
                "<memory>",
                "fn F(p0:move MockResource) with no_io{let f=fn [move p0](){};move f();}fn Main(){}"
            )
            .is_ok()
    );
    assert!(
        nexo::ir::compile_for_internal_harness(
            "<memory>",
            "fn F(p0:move MockResource){let f=fn [move p0](){};move f();}fn Main(){}",
            true
        )
        .is_ok()
    );
}
