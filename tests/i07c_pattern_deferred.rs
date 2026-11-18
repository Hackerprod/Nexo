//! F0-0002 §§4.1.1–4.1.2.3: closed graph types reach live IR values.
use nexo::diagnostic::{DataValue, Operation, RelatedRole, Stage};
use nexo::interp::{MockEnvironment, Outcome, run_scalar};
use nexo::ir::{CompileError, ReducedProfile, compile_scalar_with_profile};

const FILE: &str = "pattern_deferred.n";

fn source(body: &str) -> String {
    format!("fn Main(){{{body}}}")
}

fn accepted(body: &str) {
    let source = source(body);
    compile_scalar_with_profile(FILE, &source, ReducedProfile::JNormal, false)
        .unwrap_or_else(|error| panic!("check rejected {source}: {error:?}"));
    let program = compile_scalar_with_profile(FILE, &source, ReducedProfile::JNormal, true)
        .unwrap_or_else(|error| panic!("run rejected {source}: {error:?}"));
    let execution = run_scalar(&program, MockEnvironment::default());
    assert_eq!(execution.outcome, Outcome::Unit, "{source}");
    assert!(execution.diagnostics.is_empty(), "{source}");
    assert!(execution.observations.events().is_empty(), "{source}");
}

fn compiles(body: &str) {
    let source = source(body);
    for run_entry in [false, true] {
        compile_scalar_with_profile(FILE, &source, ReducedProfile::JNormal, run_entry)
            .unwrap_or_else(|error| panic!("compilation rejected {source}: {error:?}"));
    }
}

#[allow(clippy::too_many_arguments)] // Every diagnostic field is checked independently.
fn rejected(
    body: &str,
    code: &str,
    subcause: Option<&str>,
    operation: Operation,
    primary: (u64, u64),
    token: &str,
    types: Option<(&str, &str)>,
    related: &[(RelatedRole, u64, u64)],
) {
    let source = source(body);
    for run_entry in [false, true] {
        let mut errors =
            match compile_scalar_with_profile(FILE, &source, ReducedProfile::JNormal, run_entry) {
                Err(CompileError::Rejected(errors)) => errors,
                other => panic!("expected source rejection for {source}: {other:?}"),
            };
        assert_eq!(errors.len(), 1, "{source}: {errors:?}");
        let mut error = errors.pop().unwrap();
        error.publish(FILE, &source).unwrap();
        assert_eq!(error.code, code);
        assert_eq!(error.subcause.as_deref(), subcause);
        assert_eq!(error.stage, Stage::Types);
        assert_eq!(error.operation, operation);
        assert_eq!(
            error.rule,
            subcause.map_or_else(|| code.to_owned(), |sub| format!("{code}/{sub}"))
        );
        assert_eq!(error.explanation_id.as_deref(), Some(error.rule.as_str()));
        let span = error.primary_span.as_ref().unwrap();
        assert_eq!((span.start, span.end), primary);
        assert_eq!(&source[primary.0 as usize..primary.1 as usize], token);
        assert_eq!(error.data.len(), types.map_or(0, |_| 2));
        assert_eq!(
            error.data.get("expected_type"),
            types
                .map(|(expected, _)| DataValue::String(expected.into()))
                .as_ref()
        );
        assert_eq!(
            error.data.get("found_type"),
            types
                .map(|(_, found)| DataValue::String(found.into()))
                .as_ref()
        );
        let actual: Vec<_> = error
            .related_spans
            .iter()
            .map(|item| (item.role, item.span.start, item.span.end))
            .collect();
        assert_eq!(actual, related);
        assert!(error.cause_chain.is_empty());
        assert!(error.repair.is_empty());
        assert_eq!(error.feature_id, None);
        assert_eq!(error.unexpected_token, None);
        assert!(error.expected_tokens.is_empty());
        assert_eq!(error.expected_tokens_kind, None);
        assert_eq!(error.parser_context, None);
        assert_eq!(error.recognized_prefix, None);
    }
}

#[test]
fn pattern_local_exports_closed_array_element() {
    accepted("match Some([]){Some(v)=>{let a:Array<I64>=v;},None=>{}}");
    accepted("match Some([1]){Some(v)=>{let a:Array<I64>=v;},None=>{}}");
    accepted(
        "match Some([]){Some(v) if false=>{let a:Array<I64>=v;},Some(w)=>{let b:Array<I64>=w;},None=>{}}",
    );
    accepted("let x=Some([]);match x{Some(v)=>{let a:Array<I64>=v;},None=>{}}");
    rejected(
        "match Some([]){Some(v)=>{},None=>{}}",
        "NX-E0103",
        None,
        Operation::TypesInfer,
        (21, 23),
        "[]",
        None,
        &[],
    );
}

#[test]
fn self_read_assignment_exports_nominal_result() {
    for body in [
        "var r=[];r=[(r==r)==true];r=[true];",
        "var r=[];r=[(r==r)==true];r=[false];",
        "var r=[];r=[(r==r)==true];",
        "var r=[true];r=[(r==r)==true];",
        "var r=[];r=[r==r];r=[true];",
        "var r=[];var w=[];r=[w==w];w=[true];",
        "let p=true;let aar=[[1]];var r=[];let b=(if p==(r==r) then 1 else 2);let c=(r==aar);",
    ] {
        accepted(body);
    }
    // The empty-array index is a runtime fault; the source is type-correct.
    compiles("let ar=[1];var r=[];let a=(if (r==r)==true then 1 else 2);r[0]=r[0];r=ar;");
    rejected(
        "var r=[];r=[r];",
        "NX-E0103",
        None,
        Operation::TypesInfer,
        (16, 18),
        "[]",
        None,
        &[],
    );
    rejected(
        "var r=[];r=[(r==r)==true];r=[1];",
        "NX-E0102",
        Some("type_mismatch"),
        Operation::TypesCheck,
        (39, 40),
        "1",
        Some(("Bool", "I64")),
        &[
            (RelatedRole::ExpectedType, 21, 35),
            (RelatedRole::FoundType, 39, 40),
        ],
    );
    rejected(
        "var r=[true];r=[1];",
        "NX-E0102",
        Some("type_mismatch"),
        Operation::TypesCheck,
        (26, 27),
        "1",
        Some(("Bool", "I64")),
        &[
            (RelatedRole::ExpectedType, 16, 22),
            (RelatedRole::FoundType, 26, 27),
        ],
    );
}

#[test]
fn deferred_pattern_and_self_read_variations_do_not_panic() {
    for value in ["1", "true", "[1]", "[true]"] {
        for shape in ["Some([])", "Some([1])", "Some([true])"] {
            let body = format!(
                "match {shape}{{Some(v)=>{{let a:Array<I64>=v;let b={value};}},None=>{{}}}}"
            );
            let source = source(&body);
            for run_entry in [false, true] {
                let result =
                    compile_scalar_with_profile(FILE, &source, ReducedProfile::JNormal, run_entry);
                assert!(
                    !matches!(result, Err(CompileError::InvalidIr(_))),
                    "{source}"
                );
            }
        }
    }
}

const DV: &str = "(if true then {return;} else {return;})";

fn open_source(body: &str) -> String {
    source(&body.replace("DV", DV))
}

fn open_rejected(body: &str) -> nexo::diagnostic::Diagnostic {
    let src = open_source(body);
    for run_entry in [false, true] {
        let Err(CompileError::Rejected(mut errors)) =
            compile_scalar_with_profile(FILE, &src, ReducedProfile::JNormal, run_entry)
        else {
            panic!("expected rejection: {src}");
        };
        assert_eq!(errors.len(), 1, "{src}: {errors:?}");
        errors[0].publish(FILE, &src).unwrap();
    }
    let Err(CompileError::Rejected(mut errors)) =
        compile_scalar_with_profile(FILE, &src, ReducedProfile::JNormal, false)
    else {
        unreachable!()
    };
    *errors.remove(0)
}

#[test]
fn open_pattern_shape_accepts_constructor_orders_and_shared_payloads() {
    for body in [
        "let l0=DV;match l0{Some(x)=>{},_=>{}}",
        "let l0=DV;match l0{Some(x)=>{},None=>{}}",
        "let l0=DV;match l0{None=>{},Some(x)=>{}}",
        "let l0=DV;match l0{Ok(x)=>{},_=>{}}",
        "let l0=DV;match l0{Some(Some(x))=>{},_=>{}}",
        "let l0=DV;match l0{Some(Ok(x))=>{},_=>{}}",
        "var l0=DV;match l0{Some(x)=>{},_=>{}}",
        "let l0=DV;let w=l0;match w{Some(x)=>{},_=>{}}",
        "let l0=DV;match (l0){Some(x)=>{},_=>{}}",
        "let l0=DV;match l0{Some(x)=>{let k:Bool=x;},_=>{}}",
        "let l0=DV;match l0{Some(x)=>{let k:I64=x;},_=>{}}",
        "let l0=DV;match l0{Some(x)=>{let k=x;},_=>{}}",
        "let l0=DV;match l0{Some(x)=>{},_=>{}}let b:Option<I64>=l0;",
    ] {
        let src = open_source(body);
        for run_entry in [false, true] {
            let program =
                compile_scalar_with_profile(FILE, &src, ReducedProfile::JNormal, run_entry)
                    .unwrap_or_else(|error| panic!("{src}: {error:?}"));
            if run_entry {
                let result = run_scalar(&program, MockEnvironment::default());
                assert_eq!(result.outcome, Outcome::Unit, "{src}");
                assert!(result.diagnostics.is_empty(), "{src}");
                assert!(result.observations.stdout().is_empty(), "{src}");
                assert!(result.observations.events().is_empty(), "{src}");
            }
        }
    }
}

#[test]
fn open_pattern_reports_family_at_pattern_and_coverage_at_match() {
    for (body, code, cause, primary, token, data) in [
        (
            "let l0=DV;match l0{Some(x)=>{},Ok(y)=>{},_=>{}}",
            "NX-E1325",
            "constructor_shape",
            (78, 83),
            "Ok(y)",
            vec![
                ("constructor", "prelude:Ok"),
                ("scrutinee_constructor", "Option"),
            ],
        ),
        (
            "let l0=DV;match l0{Some(Some(x))=>{},Some(Ok(y))=>{},_=>{}}",
            "NX-E1325",
            "constructor_shape",
            (89, 94),
            "Ok(y)",
            vec![
                ("constructor", "prelude:Ok"),
                ("scrutinee_constructor", "Option"),
            ],
        ),
        (
            "let l0=DV;match l0{Some(x)=>{}}",
            "NX-E1304",
            "non_exhaustive",
            (57, 62),
            "match",
            vec![("missing_pattern", "None")],
        ),
    ] {
        let error = open_rejected(body);
        let src = open_source(body);
        assert_eq!(
            (error.code.as_str(), error.subcause.as_deref()),
            (code, Some(cause))
        );
        assert_eq!(
            (error.stage, error.operation),
            (Stage::Types, Operation::TypesCheck)
        );
        assert_eq!(
            (
                error.primary_span.as_ref().unwrap().start,
                error.primary_span.as_ref().unwrap().end
            ),
            primary
        );
        assert_eq!(&src[primary.0 as usize..primary.1 as usize], token);
        assert_eq!(error.data.len(), data.len());
        for (key, value) in data {
            assert_eq!(error.data.get(key), Some(&DataValue::String(value.into())));
        }
        assert!(error.related_spans.is_empty());
        assert!(error.cause_chain.is_empty());
        assert!(error.repair.is_empty());
        assert_eq!(error.rule, format!("{code}/{cause}"));
        assert_eq!(error.explanation_id.as_deref(), Some(error.rule.as_str()));
        assert!(error.feature_id.is_none());
        assert!(error.unexpected_token.is_none());
        assert!(error.expected_tokens.is_empty());
        assert!(error.expected_tokens_kind.is_none());
        assert!(error.parser_context.is_none());
        assert!(error.recognized_prefix.is_none());
    }
    let none = open_rejected("let l0=DV;match l0{None=>{},_=>{}}");
    assert_eq!(
        (none.code.as_str(), none.subcause.as_deref()),
        ("NX-E1304", Some("unreachable_arm"))
    );
    assert_eq!(
        (
            none.primary_span.as_ref().unwrap().start,
            none.primary_span.as_ref().unwrap().end
        ),
        (75, 76)
    );
    assert_eq!(
        none.related_spans
            .iter()
            .map(|r| (r.role, r.span.start, r.span.end))
            .collect::<Vec<_>>(),
        vec![(RelatedRole::Context, 66, 70)]
    );
    assert!(none.data.is_empty());
    assert!(none.cause_chain.is_empty());
    assert!(none.repair.is_empty());
    assert_eq!(none.rule, "NX-E1304/unreachable_arm");
    assert_eq!(none.explanation_id.as_deref(), Some(none.rule.as_str()));
    assert!(none.feature_id.is_none());
    assert!(none.unexpected_token.is_none());
    assert!(none.expected_tokens.is_empty());
    assert!(none.expected_tokens_kind.is_none());
    assert!(none.parser_context.is_none());
    assert!(none.recognized_prefix.is_none());
}

#[test]
fn unreachable_match_arms_still_check_resource_ownership() {
    for (body, code, cause, primary, relations) in [
        (
            "let l0=DV;match l0{_=>{let r=MockOpen(1);let d=r;}}",
            "NX-E0201",
            None,
            (94, 95),
            vec![(RelatedRole::Declaration, 74, 75)],
        ),
        (
            "let l0=DV;match l0{_=>{let r=MockOpen(1);let o=move r;Inspect(r);}}",
            "NX-E0202",
            Some("definitely_consumed"),
            (109, 110),
            vec![
                (RelatedRole::Declaration, 74, 75),
                (RelatedRole::Consumption, 94, 100),
            ],
        ),
    ] {
        let src = open_source(body);
        let error = open_rejected(body);
        assert_eq!(
            (error.code.as_str(), error.subcause.as_deref()),
            (code, cause)
        );
        assert_eq!(error.stage, Stage::Ownership);
        assert_eq!(
            error.operation,
            if code == "NX-E0201" {
                Operation::ValueCopy
            } else {
                Operation::OwnershipUse
            }
        );
        assert_eq!(
            (
                error.primary_span.as_ref().unwrap().start,
                error.primary_span.as_ref().unwrap().end
            ),
            primary
        );
        assert_eq!(&src[primary.0 as usize..primary.1 as usize], "r");
        assert_eq!(
            error
                .related_spans
                .iter()
                .map(|r| (r.role, r.span.start, r.span.end))
                .collect::<Vec<_>>(),
            relations
        );
        assert!(error.data.is_empty());
        let rule = cause.map_or_else(|| code.to_owned(), |sub| format!("{code}/{sub}"));
        assert_eq!(error.rule, rule);
        assert_eq!(error.explanation_id.as_deref(), Some(error.rule.as_str()));
        assert!(error.feature_id.is_none());
        assert!(error.unexpected_token.is_none());
        assert!(error.expected_tokens.is_empty());
        assert!(error.expected_tokens_kind.is_none());
        assert!(error.parser_context.is_none());
        assert!(error.recognized_prefix.is_none());
        assert!(error.repair.is_empty());
        if code == "NX-E0201" {
            assert!(error.cause_chain.is_empty());
        } else {
            assert_eq!(error.cause_chain.len(), 1);
            let cause = &error.cause_chain[0];
            assert_eq!(cause.kind, nexo::diagnostic::CauseKind::Consume);
            assert_eq!(
                cause.operation,
                nexo::diagnostic::CauseOperation::ValueConsume
            );
            assert_eq!(
                cause.span.as_ref().map(|s| (s.start, s.end)),
                Some((94, 100))
            );
            assert_eq!(
                cause.entity.as_ref().map(|e| (e.kind, e.id.as_str())),
                Some((nexo::diagnostic::EntityKind::Binding, "binding:74"))
            );
            assert_eq!(cause.evidence, nexo::diagnostic::Evidence::Verified);
        }
    }
    let some = open_rejected("let l0=DV;match l0{Some(x)=>{let r=MockOpen(1);let d=r;},_=>{}}");
    assert_eq!(some.code, "NX-E0201");
    accepted(
        "match (if true then {return;} else {return;}){Some(x)=>{let r=MockOpen(1);let d=r;},_=>{}}",
    );
    for body in [
        "let l0=DV;match l0{_=>{let r=MockOpen(1);let d=move r;}}",
        "let l0=DV;match l0{_=>{let r=MockOpen(1);let o=move r;}}",
    ] {
        let src = open_source(body);
        for run_entry in [false, true] {
            compile_scalar_with_profile(FILE, &src, ReducedProfile::JNormal, run_entry).unwrap();
        }
    }
    let corpus = include_str!("../corpus/fail/unreachable_resource_copy.n");
    for run_entry in [false, true] {
        assert!(matches!(
            compile_scalar_with_profile(FILE, corpus, ReducedProfile::JNormal, run_entry),
            Err(CompileError::Rejected(_))
        ));
    }
}
