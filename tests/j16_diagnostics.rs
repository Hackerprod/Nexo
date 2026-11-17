use nexo::diagnostic::{DataValue, Operation, Span, Stage};
use nexo::ir::{CompileError, ReducedProfile, compile_scalar_with_profile};

const FILE: &str = "j16-diagnostics.n";

fn compile(source: &str) -> Result<(), CompileError> {
    let profile = ReducedProfile::from_id("f0-0003-b-v1").unwrap();
    compile_scalar_with_profile(FILE, source, profile, true).map(|_| ())
}

#[test]
fn self_expression_is_deferred_at_name_resolution() {
    compile("fn Main(){let x=0;}").unwrap();

    let source = "fn Main(){let x=self;}";
    let error = match compile(source) {
        Err(CompileError::Rejected(mut diagnostics)) if diagnostics.len() == 1 => {
            diagnostics.pop().unwrap()
        }
        other => panic!("expected one methods diagnostic: {other:?}"),
    };
    error.validate(FILE, source).unwrap();
    assert_eq!(error.code, "NX-E0006");
    assert_eq!(error.subcause, None);
    assert_eq!(error.stage, Stage::Names);
    assert_eq!(error.operation, Operation::NamesResolve);
    assert_eq!(error.feature_id.as_deref(), Some("methods"));
    assert_eq!(
        error.primary_span,
        Some(Span {
            file: FILE.into(),
            start: 16,
            end: 20,
        })
    );
    assert_eq!(error.data.len(), 1);
    assert_eq!(
        error.data.get("pending_scope"),
        Some(&DataValue::String("later_adr".into()))
    );
    assert!(error.related_spans.is_empty());
    assert!(error.cause_chain.is_empty());
    assert!(error.repair.is_empty());
    assert_eq!(error.unexpected_token, None);
    assert!(error.expected_tokens.is_empty());
    assert_eq!(error.expected_tokens_kind, None);
    assert_eq!(error.parser_context, None);
    assert_eq!(error.recognized_prefix, None);
}
