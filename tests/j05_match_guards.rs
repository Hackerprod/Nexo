use nexo::diagnostic::{
    CauseKind, CauseOperation, DataValue, Diagnostic, EntityKind, Evidence, Operation, Span, Stage,
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
            let Err(CompileError::Rejected(run_errors)) =
                compile_scalar_with_profile(FILE, source, ReducedProfile::JNormal, true)
            else {
                panic!("run disagreed with check: {source}");
            };
            assert_eq!(run_errors.len(), 1, "{source}");
            assert_eq!(
                error.to_json(FILE, source).unwrap(),
                run_errors[0].to_json(FILE, source).unwrap(),
                "{source}"
            );
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

fn unit_events(source: &str, expected: &[&str]) {
    let execution = run_scalar(&compile(source), MockEnvironment::default());
    assert_eq!(execution.outcome, Outcome::Unit, "{source}");
    assert!(execution.diagnostics.is_empty(), "{source}");
    assert_eq!(execution.observations.events(), expected, "{source}");
}

fn rejected_sites(source: &str, expected: &[(&str, Option<&str>, u64)]) {
    let mut checked = None;
    for execute in [false, true] {
        let Err(CompileError::Rejected(errors)) =
            compile_scalar_with_profile(FILE, source, ReducedProfile::JNormal, execute)
        else {
            panic!("expected source diagnostics: {source}");
        };
        let actual: Vec<_> = errors
            .iter()
            .map(|d| {
                d.validate(FILE, source).unwrap();
                d.to_json(FILE, source).unwrap();
                (
                    d.code.clone(),
                    d.subcause.clone(),
                    d.primary_span.as_ref().unwrap().start,
                )
            })
            .collect();
        let expected: Vec<_> = expected
            .iter()
            .map(|(code, cause, start)| (code.to_string(), cause.map(str::to_string), *start))
            .collect();
        assert_eq!(actual, expected, "{source}");
        if let Some(previous) = &checked {
            assert_eq!(previous, &actual, "check/run mismatch: {source}");
        }
        checked = Some(actual);
    }
}

#[test]
fn nested_match_releases_enclosing_temporary_on_unreachable_edge() {
    unit_events(
        "fn Main(){let o:Bool=true;match o{true=>{match true{true=>{},false=>{}}},false=>{}}}",
        &[],
    );
    unit_events(
        "fn Main(){let o:Option<I64>=Some(1);match o{Some(x) if x==1=>{match true{true=>{},false=>{}}},_=>{}}}",
        &[],
    );
    unit_events(
        "fn Main(){let o:Option<I64>=Some(4);match o{Some(x) if match x==4{true=>{value:true;},false=>{value:false;}}=>{Print(1);},_=>{Print(2);}}}",
        &[],
    );
    rejected_sites(
        "fn Main(){let o:Bool=true;match o{true=>{match true{true=>{},false=>{}}},false=>{let x:I64=true;}}}",
        &[("NX-E0102", Some("type_mismatch"), 91)],
    );
}

#[test]
fn independent_name_and_type_errors_survive_invalid_guards() {
    rejected_sites(
        "fn Main(){match true{true if zz=>{},_ if yy=>{}}}",
        &[("NX-E0101", None, 29), ("NX-E0101", None, 41)],
    );
    unit_events(
        "fn Main(){let zz=true;let yy=true;match true{true if zz=>{},_ if yy=>{},_=>{}}}",
        &[],
    );
    rejected_sites(
        "fn Main(){match true{true if 1=>{},_=>{let x:I64=true;}}}",
        &[
            ("NX-E0102", Some("type_mismatch"), 29),
            ("NX-E0102", Some("type_mismatch"), 49),
        ],
    );
    unit_events(
        "fn Main(){match true{true if true=>{},_=>{let x:I64=1;}}}",
        &[],
    );
}

#[test]
fn invalid_guard_does_not_hide_independent_ownership_errors() {
    rejected_sites(
        "fn P(x:edit I64)->Bool{return true;}fn Main(){var x=0;let r=MockOpen(1);match true{true if P(edit x)=>{Drop(r);Inspect(r);},_=>{}}}",
        &[
            ("NX-E1305", Some("guard_mutation"), 93),
            ("NX-E0202", Some("definitely_consumed"), 119),
        ],
    );
    unit_events(
        "fn P(x:view I64)->Bool{return true;}fn Main(){let x=0;let r=MockOpen(1);match true{true if P(x)=>{Inspect(r);},_=>{}}}",
        &["open:0:1", "inspect:0:1", "close:0:1"],
    );
}

#[test]
fn guard_error_preserves_derived_coverage_and_none_inference() {
    rejected_sites(
        "fn Main(){match true{true if 1=>{}}}",
        &[
            ("NX-E1304", Some("non_exhaustive"), 10),
            ("NX-E0102", Some("type_mismatch"), 29),
        ],
    );
    unit_events("fn Main(){match true{true=>{},false=>{}}}", &[]);
    unit_events("fn Main(){match None{Some(x) if x==1=>{},_=>{}}}", &[]);
    rejected_sites(
        "fn Main(){match None{Some(x) if true=>{},_=>{}}}",
        &[("NX-E0113", None, 16)],
    );
}

fn guard_error(
    source: &str,
    cause: &str,
    stage: Stage,
    operation: Operation,
    primary: Span,
) -> Box<Diagnostic> {
    let error = rejected(source);
    assert_eq!(error.code, "NX-E1305");
    assert_eq!(error.subcause.as_deref(), Some(cause));
    assert_eq!(error.stage, stage);
    assert_eq!(error.operation, operation);
    assert_eq!(error.primary_span, Some(primary));
    assert!(error.related_spans.is_empty());
    assert_eq!(error.feature_id, None);
    assert_eq!(error.unexpected_token, None);
    assert!(error.expected_tokens.is_empty());
    assert!(error.repair.is_empty());
    error
}

#[test]
fn q01_unknown_effect_is_rejected_after_ownership() {
    let negative = "fn Main(){match true{true if MockUnknown()==()=>{},_=>{}}}";
    let error = guard_error(
        negative,
        "guard_effect",
        Stage::Effects,
        Operation::EffectsCheck,
        span(29, 40),
    );
    assert_eq!(
        error.data,
        BTreeMap::from([
            (
                "effect_model".into(),
                DataValue::String("f0-0003-sim-v1".into())
            ),
            (
                "effect_evidence".into(),
                DataValue::String("unknown_effect".into())
            ),
            ("required_effect".into(), DataValue::String("alloc".into())),
        ])
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
            CauseKind::Call,
            CauseOperation::IntrinsicCall,
            Some(span(29, 40)),
            Evidence::Unknown
        )
    );
    assert_eq!(
        cause
            .entity
            .as_ref()
            .map(|entity| (entity.kind, entity.id.as_str())),
        Some((EntityKind::Intrinsic, "prelude:mock_unknown"))
    );
    unit_events("fn Main(){match true{true if true=>{},_=>{}}}", &[]);
}

#[test]
fn q02_edit_intent_is_rejected_without_mutating_root() {
    let negative = "fn P(x:edit I64)->Bool{return true;}fn Main(){var x=0;match true{true if P(edit x)=>{},_=>{}}}";
    let error = guard_error(
        negative,
        "guard_mutation",
        Stage::Ownership,
        Operation::OwnershipCheck,
        span(75, 79),
    );
    assert!(error.data.is_empty());
    assert!(error.cause_chain.is_empty());
    unit_events(
        "fn P(x:edit I64)->Bool{return true;}fn Main(){var x=0;match true{true if true=>{},_=>{}}}",
        &[],
    );
}

#[test]
fn q03_move_intent_is_rejected_without_consuming_root() {
    let negative = "fn P(x:move I64)->Bool{return true;}fn Main(){let x=0;match true{true if P(move x)=>{},_=>{}}}";
    let error = guard_error(
        negative,
        "guard_transfer",
        Stage::Ownership,
        Operation::OwnershipCheck,
        span(75, 79),
    );
    assert!(error.data.is_empty());
    assert!(error.cause_chain.is_empty());
    unit_events(
        "fn P(x:move I64)->Bool{return true;}fn Main(){let x=0;match true{true if true=>{},_=>{}}}",
        &[],
    );
}

#[test]
fn q04_resource_move_in_guard_precedes_later_use_error() {
    let negative = "fn Take(r:move MockResource)->Bool{Drop(r);return true;}fn Main(){let r=MockOpen(1);match true{true if Take(move r)=>{},_=>{}}Inspect(r);}";
    let error = guard_error(
        negative,
        "guard_transfer",
        Stage::Ownership,
        Operation::OwnershipCheck,
        span(108, 112),
    );
    assert!(error.data.is_empty());
    assert!(error.cause_chain.is_empty());
    unit_events(
        "fn Take(r:view MockResource)->Bool{return true;}fn Main(){let r=MockOpen(1);match true{true if Take(r)=>{},_=>{}}Inspect(r);}",
        &["open:0:1", "inspect:0:1", "close:0:1"],
    );
}

#[test]
fn q06_try_cannot_exit_guard() {
    let negative =
        "fn Main()->Result<(),I64>{match true{true if TryAdd(1,2)?==3=>{},_=>{}}return Ok(());}";
    let error = guard_error(
        negative,
        "guard_control",
        Stage::Ownership,
        Operation::OwnershipCheck,
        span(56, 57),
    );
    assert!(error.data.is_empty());
    assert!(error.cause_chain.is_empty());
    compile(
        "fn Main()->Result<(),I64>{match true{true if TryAdd(1,2)==Ok(3)=>{},_=>{}}return Ok(());}",
    );
}

#[test]
fn q07_return_inside_if_expression_cannot_exit_guard() {
    let negative =
        "fn Main(){match true{true if if true then {return;}else{value:true;}=>{},_=>{}}}";
    let error = guard_error(
        negative,
        "guard_control",
        Stage::Ownership,
        Operation::OwnershipCheck,
        span(43, 49),
    );
    assert!(error.data.is_empty());
    assert!(error.cause_chain.is_empty());
    unit_events(
        "fn Main(){match true{true if if true then {value:true;}else{value:true;}=>{},_=>{}}}",
        &[],
    );
}

#[test]
fn guard_requires_bool_before_ownership_and_effect_checks() {
    let negative = "fn Main(){match true{true if 1=>{},_=>{}}}";
    let error = rejected(negative);
    assert_eq!(
        (error.code.as_str(), error.subcause.as_deref()),
        ("NX-E0102", Some("type_mismatch"))
    );
    assert_eq!(error.stage, Stage::Types);
    assert_eq!(error.operation, Operation::TypesCheck);
    assert_eq!(error.primary_span, Some(span(29, 30)));
    assert_eq!(
        error.data,
        BTreeMap::from([
            ("expected_type".into(), DataValue::String("Bool".into())),
            ("found_type".into(), DataValue::String("I64".into())),
        ])
    );
    assert_eq!(error.related_spans.len(), 2);
    assert_eq!(error.related_spans[0].role.as_str(), "expected_type");
    assert_eq!(error.related_spans[0].span, span(26, 28));
    assert_eq!(error.related_spans[1].role.as_str(), "found_type");
    assert_eq!(error.related_spans[1].span, span(29, 30));
    assert!(error.cause_chain.is_empty());
    unit_events("fn Main(){match true{true if true=>{},_=>{}}}", &[]);
}

#[test]
fn break_continue_and_yield_inside_guard_use_control_diagnostic() {
    for token in ["break", "continue", "yield 1"] {
        let source = format!(
            "fn Main(){{match true{{true if if true then {{{token};}}else{{value:true;}}=>{{}},_=>{{}}}}}}"
        );
        let error = rejected(&source);
        let start = source.find(token).unwrap() as u64;
        assert_eq!(
            (error.code.as_str(), error.subcause.as_deref()),
            ("NX-E1305", Some("guard_control"))
        );
        assert_eq!(error.stage, Stage::Ownership);
        assert_eq!(error.operation, Operation::OwnershipCheck);
        assert_eq!(
            error.primary_span,
            Some(span(
                start,
                start + token.split_whitespace().next().unwrap().len() as u64
            ))
        );
        assert!(error.data.is_empty());
        assert!(error.related_spans.is_empty());
        assert!(error.cause_chain.is_empty());
    }
}

#[test]
fn independent_guard_sites_keep_their_own_diagnostics() {
    let source = "fn Main(){match true{true if if true then {let x=1;Drop(x);return;}else{value:true;}=>{},_=>{}}}";
    let Err(CompileError::Rejected(errors)) =
        compile_scalar_with_profile(FILE, source, ReducedProfile::JNormal, false)
    else {
        panic!("independent guard violations were accepted");
    };
    assert_eq!(errors.len(), 2, "{errors:?}");
    assert_eq!(
        errors
            .iter()
            .map(|error| error.subcause.as_deref())
            .collect::<Vec<_>>(),
        vec![Some("guard_transfer"), Some("guard_control")]
    );
    for error in errors {
        error.validate(FILE, source).unwrap();
    }
}

#[test]
fn independent_guard_effect_sites_each_report_one_diagnostic() {
    let source = "fn Main(){match true{true if MockUnknown()==MockUnknown()=>{},_=>{}}}";
    let Err(CompileError::Rejected(errors)) =
        compile_scalar_with_profile(FILE, source, ReducedProfile::JNormal, false)
    else {
        panic!("independent effects were accepted");
    };
    assert_eq!(errors.len(), 2, "{errors:?}");
    assert!(
        errors
            .iter()
            .all(|error| error.code == "NX-E1305"
                && error.subcause.as_deref() == Some("guard_effect"))
    );
    assert_eq!(
        errors
            .iter()
            .map(|error| error.primary_span.as_ref().unwrap().start)
            .collect::<Vec<_>>(),
        vec![29, 44]
    );
    for error in errors {
        error.validate(FILE, source).unwrap();
    }
}

#[test]
fn guarded_arm_does_not_prove_coverage_and_unmatched_guard_is_not_evaluated() {
    let negative = "fn Main(){match true{true if true=>{},false=>{}}}";
    let error = rejected(negative);
    assert_eq!(
        (error.code.as_str(), error.subcause.as_deref()),
        ("NX-E1304", Some("non_exhaustive"))
    );
    assert_eq!(
        error.data.get("missing_pattern"),
        Some(&DataValue::String("true".into()))
    );
    unit_events(
        "fn Main(){match true{false if 9223372036854775807+1==0=>{},true=>{},_=>{}}}",
        &[],
    );
}

#[test]
fn false_guard_releases_provisional_binding_before_owner_extraction() {
    unit_events(
        "fn Main(){let o=Some(MockOpen(1));match move o{Some(move r) if false=>{Inspect(r);},Some(move r)=>{Inspect(r);},None=>{}}}",
        &["open:0:1", "inspect:0:1", "close:0:1"],
    );
    unit_events(
        "fn Main(){let o=Some(MockOpen(1));match move o{Some(move r) if true=>{Inspect(r);},Some(move r)=>{Inspect(r);},None=>{}}}",
        &["open:0:1", "inspect:0:1", "close:0:1"],
    );
    unit_events(
        "fn P(r:view MockResource)->Bool{return true;}fn Main(){let o=Some(MockOpen(1));match move o{Some(move r) if P(r)=>{Inspect(r);},Some(move r)=>{},None=>{}}}",
        &["open:0:1", "inspect:0:1", "close:0:1"],
    );
}

#[test]
fn transitive_guard_effect_uses_callee_summary_and_call_site() {
    let source = "fn P()->Bool{Print(1);return true;}fn Main(){match true{true if P()=>{},_=>{}}}";
    let error = rejected(source);
    assert_eq!(
        (error.code.as_str(), error.subcause.as_deref()),
        ("NX-E1305", Some("guard_effect"))
    );
    assert_eq!(error.stage, Stage::Effects);
    assert_eq!(error.operation, Operation::EffectsCheck);
    assert_eq!(error.primary_span, Some(span(64, 65)));
    assert_eq!(
        error.data.get("required_effect"),
        Some(&DataValue::String("io".into()))
    );
    assert_eq!(
        error.data.get("effect_evidence"),
        Some(&DataValue::String("known_effect".into()))
    );
    assert_eq!(error.cause_chain.len(), 2);
    assert_eq!(error.cause_chain[0].operation, CauseOperation::FunctionCall);
    assert_eq!(
        error.cause_chain[1].operation,
        CauseOperation::IntrinsicCall
    );
    unit_events(
        "fn P()->Bool{return true;}fn Main(){match true{true if P()=>{},_=>{}}}",
        &[],
    );
}

#[test]
fn guard_fallthrough_and_body_states_join_after_selection() {
    let source = "fn F(b:view Bool){let r=MockOpen(1);match b{true if b=>{Drop(r);},true=>{},false=>{}}Inspect(r);}fn Main(){}";
    let error = rejected(source);
    assert_eq!(
        (error.code.as_str(), error.subcause.as_deref()),
        ("NX-E0202", Some("maybe_consumed"))
    );
    unit_events(
        "fn F(b:view Bool){let r=MockOpen(1);match b{true if b=>{Inspect(r);},true=>{},false=>{}}Inspect(r);}fn Main(){F(true);F(false);}",
        &[
            "open:0:1",
            "inspect:0:1",
            "inspect:0:1",
            "close:0:1",
            "open:1:1",
            "inspect:1:1",
            "close:1:1",
        ],
    );
}

#[test]
fn guarded_match_expression_uses_the_selected_body_value() {
    unit_events(
        "fn Main(){let x=match true{true if false=>{value:1;},true=>{value:2;},false=>{value:3;}};AssertEq(x,2);}",
        &[],
    );
}

#[test]
fn false_guard_releases_provisional_edit_binding_before_selected_edit() {
    unit_events(
        "fn Main(){var o=Some(1);match edit o{Some(edit n) if false=>{n=1;},Some(edit n)=>{n=2;},None=>{}}match o{Some(n)=>{AssertEq(n,2);},None=>{}}}",
        &[],
    );
}

#[test]
fn scrutinee_is_evaluated_once_before_guard_and_body() {
    unit_events(
        "fn P(r:view MockResource)->Bool{return true;}fn Main(){match Some(MockOpen(1)){Some(move r) if P(r)=>{Inspect(r);},_=>{}}}",
        &["open:0:1", "inspect:0:1", "close:0:1"],
    );
}

#[test]
fn guard_runtime_fault_cleans_unextracted_owner_once() {
    let source = "fn Main(){match Some(MockOpen(1)){Some(move r) if 9223372036854775807+1==0=>{Inspect(r);},Some(move r)=>{Inspect(r);},_=>{}}}";
    let execution = run_scalar(&compile(source), MockEnvironment::default());
    assert_eq!(execution.outcome, Outcome::SimulatorFault);
    assert_eq!(execution.diagnostics.len(), 1);
    assert_eq!(execution.diagnostics[0].code, "R0101");
    assert_eq!(execution.observations.events(), ["open:0:1", "close:0:1"]);
}
