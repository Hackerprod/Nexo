use nexo::diagnostic::{DataValue, Diagnostic, Operation, RelatedRole, Span, Stage};
use nexo::interp::{MockEnvironment, Outcome, run_scalar};
use nexo::ir::{CompileError, ReducedProfile, compile_scalar_with_profile};

const FILE: &str = "<memory>";

fn diagnostic(source: &str) -> Box<Diagnostic> {
    match compile_scalar_with_profile(FILE, source, ReducedProfile::JNormal, false) {
        Err(CompileError::Rejected(mut errors)) if errors.len() == 1 => {
            let error = errors.pop().unwrap();
            error.validate(FILE, source).unwrap();
            error
        }
        _ => panic!("expected one source diagnostic: {source}"),
    }
}

fn accepted(source: &str) {
    assert!(
        compile_scalar_with_profile(FILE, source, ReducedProfile::JNormal, true).is_ok(),
        "{source}"
    );
}

fn span(start: u64, end: u64) -> Span {
    Span {
        file: FILE.into(),
        start,
        end,
    }
}

#[test]
fn option_none_without_context_is_o01() {
    let source = "fn Main(){let o=None;}";
    let error = diagnostic(source);
    assert_eq!(error.code, "NX-E0113");
    assert_eq!(error.subcause, None);
    assert_eq!(error.stage, Stage::Types);
    assert_eq!(error.operation, Operation::TypesInfer);
    assert_eq!(error.primary_span, Some(span(16, 20)));
    assert_eq!(
        error.data.get("unresolved_components"),
        Some(&DataValue::Array(vec![DataValue::String("$.some".into())]))
    );
    assert_eq!(error.data.len(), 1);
    assert!(error.related_spans.is_empty());
    assert!(error.cause_chain.is_empty());
    accepted("fn Main(){let o:Option<I64>=None;}");
}

#[test]
fn option_question_mark_requires_result_o02() {
    let source = "fn Main()->Result<(),I64>{let o:Option<I64>=Some(1);let x=o?;return Ok(());}";
    let error = diagnostic(source);
    assert_eq!(error.code, "NX-E0102");
    assert_eq!(error.subcause.as_deref(), Some("type_mismatch"));
    assert_eq!(error.stage, Stage::Types);
    assert_eq!(error.operation, Operation::TypesCheck);
    assert_eq!(error.primary_span, Some(span(59, 60)));
    assert_eq!(
        error.data.get("expected_type"),
        Some(&DataValue::String("domain:result".into()))
    );
    assert_eq!(
        error.data.get("found_type"),
        Some(&DataValue::String("Option<I64>".into()))
    );
    assert_eq!(error.data.len(), 2);
    assert_eq!(error.related_spans.len(), 1);
    assert_eq!(error.related_spans[0].role, RelatedRole::FoundType);
    assert_eq!(error.related_spans[0].span, span(58, 59));
    assert!(error.cause_chain.is_empty());
    accepted(
        "fn Main()->Result<(),I64>{let o:Option<I64>=Some(1);let x=TryAdd(1,2)?;return Ok(());}",
    );
}

#[test]
fn none_call_and_explicit_some_type_keep_their_published_boundaries() {
    let source = "fn Main(){let x:Option<I64>=None();}";
    let error = diagnostic(source);
    assert_eq!(error.code, "NX-E0111");
    assert_eq!(error.subcause.as_deref(), Some("non_callable_target"));
    assert_eq!(error.primary_span, Some(span(28, 32)));
    assert_eq!(error.stage, Stage::Arguments);
    assert_eq!(error.operation, Operation::CallBind);
    assert!(error.data.is_empty());
    assert!(error.related_spans.is_empty());
    accepted("fn Main(){let x:Option<I64>=None;}");

    let source = "fn Main(){let x=Some<I64>(1);}";
    let error = diagnostic(source);
    assert_eq!(error.code, "NX-E0006");
    assert_eq!(error.feature_id.as_deref(), Some("traits_generics"));
    assert_eq!(error.primary_span, Some(span(20, 21)));
    assert_eq!(
        error.data.get("pending_scope"),
        Some(&DataValue::String("later_adr".into()))
    );
    accepted("fn Main(){let x=Some(1);}");
}

#[test]
fn option_resource_payload_is_closed_once_and_none_remains_noncopyable() {
    let source = "fn Main(){let o:Option<MockResource>=Some(MockOpen(1));}";
    let program = compile_scalar_with_profile(FILE, source, ReducedProfile::JNormal, true)
        .unwrap_or_else(|_| panic!("Option resource construction must compile"));
    let execution = run_scalar(&program, MockEnvironment::default());
    assert_eq!(execution.outcome, Outcome::Unit);
    assert_eq!(execution.observations.events(), ["open:0:1", "close:0:1"]);
    let source = "fn Main(){let o:Option<MockResource>=None;let q=o;}";
    let error = diagnostic(source);
    assert_eq!(error.code, "NX-E0201");
    assert_eq!(error.stage, Stage::Ownership);
    assert_eq!(error.operation, Operation::ValueCopy);
    assert_eq!(error.primary_span, Some(span(48, 49)));
}
