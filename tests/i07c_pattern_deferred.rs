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
