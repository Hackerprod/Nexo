use nexo::diagnostic::{DataValue, Diagnostic, Operation, RelatedRole, Span, Stage};
use nexo::interp::{MockEnvironment, Outcome, run_scalar};
use nexo::ir::{CompileError, ReducedProfile, compile_scalar_with_profile};

const FILE: &str = "<memory>";

fn rejected(source: &str) -> Box<Diagnostic> {
    match compile_scalar_with_profile(FILE, source, ReducedProfile::JNormal, false) {
        Err(CompileError::Rejected(mut errors)) if errors.len() == 1 => {
            let error = errors.pop().unwrap();
            error.validate(FILE, source).unwrap();
            error
        }
        other => panic!("expected one source diagnostic: {other:?}"),
    }
}

fn accepted(source: &str) {
    let program = compile_scalar_with_profile(FILE, source, ReducedProfile::JNormal, true)
        .unwrap_or_else(|error| panic!("valid match rejected: {error:?}"));
    let result = run_scalar(&program, MockEnvironment::default());
    assert_eq!(result.outcome, Outcome::Unit);
    assert!(result.diagnostics.is_empty());
}

fn span(start: u64, end: u64) -> Span {
    Span {
        file: FILE.into(),
        start,
        end,
    }
}

fn pattern_error(
    source: &str,
    code: &str,
    subcause: Option<&str>,
    start: u64,
    end: u64,
    data: &[(&str, DataValue)],
) -> Box<Diagnostic> {
    let error = rejected(source);
    assert_eq!(error.code, code);
    assert_eq!(error.subcause.as_deref(), subcause);
    assert_eq!(error.stage, Stage::Types);
    assert_eq!(error.operation, Operation::TypesCheck);
    assert_eq!(error.primary_span, Some(span(start, end)));
    assert_eq!(
        error
            .data
            .iter()
            .map(|(key, value)| (key.as_str(), value))
            .collect::<Vec<_>>(),
        data.iter()
            .map(|(key, value)| (*key, value))
            .collect::<Vec<_>>()
    );
    assert!(error.cause_chain.is_empty());
    assert!(error.feature_id.is_none());
    error
}

#[test]
fn match_option_missing_none_is_m01() {
    let source = "fn Main(){let o:Option<I64>=Some(1);match o{Some(n)=>{}}}";
    let error = pattern_error(
        source,
        "NX-E1304",
        Some("non_exhaustive"),
        36,
        41,
        &[("missing_pattern", DataValue::String("None".into()))],
    );
    assert!(error.related_spans.is_empty());
    accepted("fn Main(){let o:Option<I64>=Some(1);match o{Some(n)=>{},None=>{}}}");
}

#[test]
fn match_redundant_wildcard_is_m02() {
    let source = "fn Main(){match true{true=>{},false=>{},_=>{}}}";
    let error = pattern_error(source, "NX-E1304", Some("unreachable_arm"), 40, 41, &[]);
    assert_eq!(
        error
            .related_spans
            .iter()
            .map(|related| (related.role, related.span.clone()))
            .collect::<Vec<_>>(),
        vec![
            (RelatedRole::Context, span(21, 25)),
            (RelatedRole::Context, span(30, 35))
        ]
    );
    accepted("fn Main(){match true{true=>{},false=>{}}}");
}

#[test]
fn match_expression_branch_join_is_m03() {
    let source = "fn Main(){let x=match true{true=>{value:1;},false=>{value:false;}};}";
    let error = pattern_error(
        source,
        "NX-E1303",
        None,
        58,
        63,
        &[
            ("expected_type", DataValue::String("I64".into())),
            ("found_type", DataValue::String("Bool".into())),
        ],
    );
    assert_eq!(
        error
            .related_spans
            .iter()
            .map(|related| (related.role, related.span.clone()))
            .collect::<Vec<_>>(),
        vec![
            (RelatedRole::ExpectedType, span(40, 41)),
            (RelatedRole::FoundType, span(58, 63))
        ]
    );
    accepted("fn Main(){let x=match true{true=>{value:1;},false=>{value:2;}};AssertEq(x,1);}");
}

#[test]
fn match_constructor_arity_is_m04() {
    let source = "fn Main(){let o:Option<I64>=Some(1);match o{Some(a,b)=>{},None=>{}}}";
    let error = pattern_error(
        source,
        "NX-E1325",
        Some("constructor_arity"),
        44,
        53,
        &[
            ("constructor", DataValue::String("prelude:Some".into())),
            ("expected_arity", DataValue::UInt(1)),
            ("found_arity", DataValue::UInt(2)),
        ],
    );
    assert!(error.related_spans.is_empty());
    accepted("fn Main(){let o:Option<I64>=Some(1);match o{Some(a)=>{},None=>{}}}");
}

#[test]
fn match_constructor_nominal_identity_is_m05() {
    let source = "fn Main(){let o:Option<I64>=Some(1);match o{Ok(n)=>{},_=>{}}}";
    let error = pattern_error(
        source,
        "NX-E1325",
        Some("constructor_type"),
        44,
        49,
        &[
            ("constructor", DataValue::String("prelude:Ok".into())),
            ("scrutinee_type", DataValue::String("Option<I64>".into())),
        ],
    );
    assert!(error.related_spans.is_empty());
    accepted("fn Main(){let o:Option<I64>=Some(1);match o{Some(n)=>{},_=>{}}}");
}

#[test]
fn match_duplicate_binding_precedes_arity_m07() {
    let source = "fn Main(){let o:Option<I64>=Some(1);match o{Some(x,x)=>{},None=>{}}}";
    let error = rejected(source);
    assert_eq!(error.code, "NX-E0104");
    assert_eq!(error.subcause, None);
    assert_eq!(error.stage, Stage::Names);
    assert_eq!(error.operation, Operation::NamesDeclare);
    assert_eq!(error.primary_span, Some(span(51, 52)));
    assert_eq!(error.data.len(), 1);
    assert_eq!(
        error.data.get("declaration_kind"),
        Some(&DataValue::String("binding".into()))
    );
    assert_eq!(
        error
            .related_spans
            .iter()
            .map(|related| (related.role, related.span.clone()))
            .collect::<Vec<_>>(),
        vec![(RelatedRole::PreviousDeclaration, span(49, 50))]
    );
    assert!(error.cause_chain.is_empty());
    accepted("fn Main(){let o:Option<I64>=Some(1);match o{Some(x)=>{},None=>{}}}");
}

#[test]
fn match_unknown_constructor_is_m08() {
    let source = "fn Main(){let o:Option<I64>=Some(1);match o{Missing(x)=>{},None=>{}}}";
    let error = rejected(source);
    assert_eq!(error.code, "NX-E0101");
    assert_eq!(error.subcause, None);
    assert_eq!(error.stage, Stage::Names);
    assert_eq!(error.operation, Operation::NamesResolve);
    assert_eq!(error.primary_span, Some(span(44, 51)));
    assert_eq!(error.data.len(), 1);
    assert_eq!(
        error.data.get("use_kind"),
        Some(&DataValue::String("call_target".into()))
    );
    assert!(error.related_spans.is_empty());
    assert!(error.cause_chain.is_empty());
    accepted("fn Main(){let o:Option<I64>=Some(1);match o{Some(x)=>{},None=>{}}}");
}

#[test]
fn match_nested_residual_is_m10() {
    let source = "fn Main(){let o:Option<Result<I64,Bool>>=None;match o{None=>{},Some(Ok(_))=>{},Some(Err(true))=>{}}}";
    let error = pattern_error(
        source,
        "NX-E1304",
        Some("non_exhaustive"),
        46,
        51,
        &[(
            "missing_pattern",
            DataValue::String("Some(Err(false))".into()),
        )],
    );
    assert!(error.related_spans.is_empty());
    accepted(
        "fn Main(){let o:Option<Result<I64,Bool>>=None;match o{None=>{},Some(Ok(_))=>{},Some(Err(true))=>{},Some(Err(false))=>{}}}",
    );
}

#[test]
fn match_integer_residual_is_m11() {
    let source = "fn Main(){match 0{0=>{},1=>{}}}";
    let error = pattern_error(
        source,
        "NX-E1304",
        Some("non_exhaustive"),
        10,
        15,
        &[("missing_pattern", DataValue::String("-1".into()))],
    );
    assert!(error.related_spans.is_empty());
    accepted("fn Main(){match 0{0=>{},1=>{},_=>{}}}");
}

#[test]
fn none_pattern_is_a_read_binding_under_result() {
    accepted("fn Main(){let r:Result<I64,I64>=Ok(1);match r{None=>{AssertEq(None,r);}}}");
    let error = rejected("fn Main(){let r:Result<I64,I64>=Ok(1);match r{None=>{Drop(None);}}}");
    assert_eq!(
        (error.code.as_str(), error.subcause.as_deref()),
        ("NX-E1105", Some("permission_escalation"))
    );
}

#[test]
fn match_read_alias_preserves_resource_owner() {
    let source = "fn Main(){let o:Option<MockResource>=Some(MockOpen(1));match o{Some(n)=>{Inspect(n);},None=>{}}}";
    let program = compile_scalar_with_profile(FILE, source, ReducedProfile::JNormal, true).unwrap();
    let result = run_scalar(&program, MockEnvironment::default());
    assert_eq!(result.outcome, Outcome::Unit);
    assert_eq!(
        result.observations.events(),
        ["open:0:1", "inspect:0:1", "close:0:1"]
    );
}
