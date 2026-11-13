use nexo::diagnostic::{DataValue, Operation, Span, Stage, explain};
use nexo::interp::{MockEnvironment, Outcome, run_scalar};
use nexo::ir::{CompileError, ReducedProfile, compile_scalar_with_profile};
use std::{fs, process::Command};

const FILE: &str = "<match-regression>";

fn rejected(source: &str) -> Box<nexo::diagnostic::Diagnostic> {
    for execute in [false, true] {
        match compile_scalar_with_profile(FILE, source, ReducedProfile::JNormal, execute) {
            Err(CompileError::Rejected(ref errors)) if errors.len() == 1 => {
                errors[0].validate(FILE, source).unwrap();
            }
            other => panic!("expected one diagnostic for {source}: {other:?}"),
        }
    }
    let Err(CompileError::Rejected(mut errors)) =
        compile_scalar_with_profile(FILE, source, ReducedProfile::JNormal, false)
    else {
        unreachable!()
    };
    errors.pop().unwrap()
}

fn runs(source: &str, stdout: &[&str], events: &[&str]) {
    compile_scalar_with_profile(FILE, source, ReducedProfile::JNormal, false).unwrap();
    let program = compile_scalar_with_profile(FILE, source, ReducedProfile::JNormal, true).unwrap();
    let result = run_scalar(&program, MockEnvironment::default());
    assert_eq!(result.outcome, Outcome::Unit, "{source}");
    assert!(result.diagnostics.is_empty());
    assert_eq!(result.observations.stdout(), stdout, "{source}");
    assert_eq!(result.observations.events(), events, "{source}");
}

#[test]
fn pattern_constructor_name_must_resolve_a_nominal_variant() {
    for name in ["MockOpen", "Print", "K"] {
        let prefix = if name == "K" {
            "fn K(x:view I64){}"
        } else {
            ""
        };
        let source = format!(
            "{prefix}fn Main(){{let o:Option<I64>=Some(1);match o{{{name}(n)=>{{}},_=>{{}}}}}}"
        );
        let error = rejected(&source);
        let start = source.find(&format!("{name}(n)")).unwrap() as u64;
        assert_eq!(
            (error.code.as_str(), error.subcause.as_deref()),
            ("NX-E0101", None)
        );
        assert_eq!(
            (error.stage, error.operation),
            (Stage::Names, Operation::NamesResolve)
        );
        assert_eq!(
            error.primary_span,
            Some(Span {
                file: FILE.into(),
                start,
                end: start + name.len() as u64
            })
        );
        assert_eq!(
            error.data.get("use_kind"),
            Some(&DataValue::String("call_target".into()))
        );
    }
    runs(
        "fn Main(){let o:Option<I64>=Some(1);match o{Some(n)=>{Print(n);},None=>{}}}",
        &["1"],
        &[],
    );
}

#[test]
fn final_payload_binding_is_proved_after_subdivided_sibling_variants() {
    let negative =
        "fn Main(){let x:Result<Bool,Bool>=Ok(true);match x{Ok(true)=>{},Ok(false)=>{}}}";
    let error = rejected(negative);
    assert_eq!(
        (error.code.as_str(), error.subcause.as_deref()),
        ("NX-E1304", Some("non_exhaustive"))
    );
    assert_eq!(
        error.data.get("missing_pattern"),
        Some(&DataValue::String("Err(false)".into()))
    );
    runs(
        "fn Main(){let x:Result<Bool,Bool>=Ok(true);match x{Ok(true)=>{},Ok(false)=>{},Err(e)=>{}}}",
        &[],
        &[],
    );
    runs(
        "fn Main(){let x:Result<Option<I64>,I64>=Ok(Some(1));match x{Ok(Some(_))=>{},Ok(None)=>{},Err(e)=>{}}}",
        &[],
        &[],
    );
    runs(
        "fn Mk()->Result<Option<MockResource>,I64>{return Ok(Some(MockOpen(8)));}fn Main(){match Mk(){Ok(Some(move r))=>{Inspect(r);},Ok(None)=>{},Err(e)=>{Print(e);}}}",
        &[],
        &["open:0:8", "inspect:0:8", "close:0:8"],
    );
}

#[test]
fn temporary_scrutinee_uses_its_resolved_nominal_type_and_plan() {
    let negative = "fn Main(){match Some(5){Some(true)=>{},_=>{}}}";
    let error = rejected(negative);
    assert_eq!(
        (error.code.as_str(), error.subcause.as_deref()),
        ("NX-E0102", Some("type_mismatch"))
    );
    runs(
        "fn Main(){match Some(5){Some(5)=>{Print(1);},_=>{Print(2);}}}",
        &["1"],
        &[],
    );
    runs("fn Main(){match Some(1){x=>{}}}", &[], &[]);
    runs(
        "fn Main(){match Some(Some(1)){Some(Some(m))=>{},_=>{}}}",
        &[],
        &[],
    );
    runs(
        "fn Main(){match [Some(1)][0]{Some(n)=>{},None=>{}}}",
        &[],
        &[],
    );
}

#[test]
fn unresolved_inline_scrutinee_literals_report_only_the_constructor_inference_error() {
    let cases = [
        (
            "fn Main(){match Ok(1){Ok(1)=>{},_=>{}}}",
            "fn Main(){let r:Result<I64,Bool>=Ok(1);match r{Ok(1)=>{},_=>{}}}",
        ),
        (
            "fn Main(){match None{Some(1)=>{},_=>{}}}",
            "fn Main(){let o:Option<I64>=None;match o{Some(1)=>{},_=>{}}}",
        ),
        (
            "fn Main(){match Err(true){Err(true)=>{},_=>{}}}",
            "fn Main(){let r:Result<I64,Bool>=Err(true);match r{Err(true)=>{},_=>{}}}",
        ),
    ];
    for (source, positive) in cases {
        let error = rejected(source);
        assert_eq!(
            (error.code.as_str(), error.subcause.as_deref()),
            ("NX-E0113", None)
        );
        assert_eq!(
            (error.stage, error.operation),
            (Stage::Types, Operation::TypesInfer)
        );
        assert!(
            matches!(error.data.get("unresolved_components"), Some(DataValue::Array(paths)) if !paths.is_empty())
        );
        runs(positive, &[], &[]);
    }
}

#[test]
fn unresolved_inline_scrutinee_cli_diagnostics_are_json_objects_in_check_and_run() {
    for (index, source) in [
        "fn Main(){match Ok(1){Ok(1)=>{},_=>{}}}",
        "fn Main(){match None{Some(1)=>{},_=>{}}}",
        "fn Main(){match Err(true){Err(true)=>{},_=>{}}}",
    ]
    .into_iter()
    .enumerate()
    {
        let file = std::env::temp_dir().join(format!(
            "nexo-inline-scrutinee-{}-{index}.n",
            std::process::id()
        ));
        fs::write(&file, source).unwrap();
        for command in ["check", "run"] {
            let result = Command::new(env!("CARGO_BIN_EXE_nexo"))
                .args([
                    command,
                    file.to_str().unwrap(),
                    "--format",
                    "json",
                    "--profile",
                    "f0-0003-b-v1",
                ])
                .output()
                .unwrap();
            let output = String::from_utf8(result.stdout).unwrap();
            assert_eq!(result.status.code(), Some(1), "{source}: {output}");
            assert!(
                output.contains("\"diagnostics\":[{\"schema_version\":"),
                "{source}: {output}"
            );
            assert!(!output.contains("},null]"), "{source}: {output}");
            assert_eq!(
                output.matches("\"code\":\"NX-E0113\"").count(),
                1,
                "{source}: {output}"
            );
            assert!(!output.contains("internal_error"), "{source}: {output}");
        }
        fs::remove_file(file).unwrap();
    }
}

#[test]
fn pattern_binding_shadows_a_function_inside_its_arm() {
    let negative = "fn K()->I64{return 5;}fn Main(){Print(K);}";
    let error = rejected(negative);
    assert_eq!(
        (error.code.as_str(), error.feature_id.as_deref()),
        ("NX-E0006", Some("function_value"))
    );
    runs(
        "fn K()->I64{return 5;}fn Main(){let o:Option<I64>=Some(1);match o{Some(K)=>{Print(K);},None=>{}}}",
        &["1"],
        &[],
    );
    runs(
        "fn K()->I64{return 5;}fn Main(){let o:Option<I64>=Some(1);match o{K=>{let n=K;Print(n);}}}",
        &["Some(1)"],
        &[],
    );
}

#[test]
fn match_explanations_carry_paired_b_sources() {
    for (code, subcause) in [
        ("NX-E1304", "non_exhaustive"),
        ("NX-E1304", "unreachable_arm"),
        ("NX-E1325", "constructor_arity"),
        ("NX-E1325", "constructor_type"),
        ("NX-E1326", "break_outside_loop"),
        ("NX-E1326", "continue_outside_loop"),
    ] {
        let rows = explain(code, Some(subcause)).unwrap();
        assert_eq!(rows.len(), 1);
        assert!(rows[0].negative_source.starts_with("fn Main()"));
        assert!(rows[0].positive_source.starts_with("fn Main()"));
    }
}
