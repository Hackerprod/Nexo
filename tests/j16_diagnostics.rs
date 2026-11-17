use nexo::diagnostic::{DataValue, Operation, Span, Stage};
use nexo::ir::{CompileError, ReducedProfile, compile_scalar_with_profile};
use std::{
    fs,
    process::Command,
    sync::atomic::{AtomicUsize, Ordering},
};

static NEXT: AtomicUsize = AtomicUsize::new(0);

const FILE: &str = "j16-diagnostics.n";

fn compile(source: &str) -> Result<(), CompileError> {
    let profile = ReducedProfile::from_id("f0-0003-b-v1").unwrap();
    compile_scalar_with_profile(FILE, source, profile, true).map(|_| ())
}

fn rejected(source: &str) -> Vec<nexo::diagnostic::Diagnostic> {
    let Err(CompileError::Rejected(errors)) = compile(source) else {
        panic!("expected a source rejection: {source}");
    };
    for error in &errors {
        error.validate(FILE, source).unwrap();
    }
    errors.into_iter().map(|error| *error).collect()
}

#[test]
fn self_prefixed_identifier_is_accepted_by_check_and_run() {
    let source = "fn Main(){let selfless=1;let y=selfless;}";
    compile(source).unwrap();
    let id = NEXT.fetch_add(1, Ordering::Relaxed);
    let file =
        std::env::temp_dir().join(format!("nexo-j16-selfless-{}-{id}.n", std::process::id()));
    fs::write(&file, source).unwrap();
    for command in ["check", "run"] {
        let output = Command::new(env!("CARGO_BIN_EXE_nexo"))
            .args([
                command,
                file.to_str().unwrap(),
                "--profile",
                "f0-0003-b-v1",
                "--format",
                "json",
            ])
            .output()
            .unwrap();
        let stdout = String::from_utf8(output.stdout).unwrap();
        assert_eq!(output.status.code(), Some(0), "{command}: {stdout}");
        assert!(stdout.contains("\"diagnostics\":[]"), "{command}: {stdout}");
    }
    fs::remove_file(file).unwrap();
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

#[test]
fn self_in_expression_positions_is_deferred_at_name_resolution() {
    let cases = [
        ("call_target", "fn Main(){let o=Some(1);let x=self(o);}", 30),
        ("index_target", "fn Main(){let x=self[0];}", 16),
        ("argument", "fn Main(){Print(self);}", 16),
        ("operand", "fn Main(){let x=1+self;}", 18),
        ("closure", "fn Main(){let f=fn [](){let x=self;};}", 30),
    ];
    for (context, source, start) in cases {
        let errors = rejected(source);
        assert_eq!(errors.len(), 1, "{context}: {errors:?}");
        let error = &errors[0];
        assert_eq!(error.code, "NX-E0006", "{context}");
        assert_eq!(error.subcause, None, "{context}");
        assert_eq!(error.stage, Stage::Names, "{context}");
        assert_eq!(error.operation, Operation::NamesResolve, "{context}");
        assert_eq!(error.feature_id.as_deref(), Some("methods"), "{context}");
        assert_eq!(
            error
                .primary_span
                .as_ref()
                .map(|span| (span.start, span.end)),
            Some((start, start + 4)),
            "{context}"
        );
        assert_eq!(error.data.len(), 1, "{context}");
        assert_eq!(
            error.data.get("pending_scope"),
            Some(&DataValue::String("later_adr".into())),
            "{context}"
        );
        assert!(error.related_spans.is_empty(), "{context}");
        assert!(error.cause_chain.is_empty(), "{context}");
        assert!(error.repair.is_empty(), "{context}");
        assert_eq!(error.unexpected_token, None, "{context}");
        assert!(error.expected_tokens.is_empty(), "{context}");
        assert_eq!(error.expected_tokens_kind, None, "{context}");
        assert_eq!(error.parser_context, None, "{context}");
        assert_eq!(error.recognized_prefix, None, "{context}");

        let id = NEXT.fetch_add(1, Ordering::Relaxed);
        let file = std::env::temp_dir().join(format!(
            "nexo-j16-self-{context}-{}-{id}.n",
            std::process::id()
        ));
        fs::write(&file, source).unwrap();
        for command in ["check", "run"] {
            let output = Command::new(env!("CARGO_BIN_EXE_nexo"))
                .args([
                    command,
                    file.to_str().unwrap(),
                    "--profile",
                    "f0-0003-b-v1",
                    "--format",
                    "json",
                ])
                .output()
                .unwrap();
            let stdout = String::from_utf8(output.stdout).unwrap();
            assert_eq!(
                output.status.code(),
                Some(1),
                "{context}/{command}: {stdout}"
            );
            assert!(
                stdout.contains("\"code\":\"NX-E0006\""),
                "{context}/{command}: {stdout}"
            );
            assert!(
                stdout.contains("\"stage\":\"names\""),
                "{context}/{command}: {stdout}"
            );
            assert!(
                stdout.contains("\"operation\":\"names.resolve\""),
                "{context}/{command}: {stdout}"
            );
            assert!(
                stdout.contains("\"feature_id\":\"methods\""),
                "{context}/{command}: {stdout}"
            );
            assert!(
                stdout.contains(&format!("\"start\":{start},\"end\":{}", start + 4)),
                "{context}/{command}: {stdout}"
            );
            assert!(
                stdout.contains("\"pending_scope\":\"later_adr\""),
                "{context}/{command}: {stdout}"
            );
        }
        fs::remove_file(file).unwrap();
    }
}

#[test]
fn match_scrutinee_read_ends_at_the_last_reachable_edge() {
    compile("fn Main(){var i=0;match i{0=>{},_=>{}}i=1;}").unwrap();

    let source = "fn Main(){var i=0;match i{0=>{i=1;},_=>{}}}";
    let errors = rejected(source);
    assert_eq!(errors.len(), 1);
    let error = &errors[0];
    assert_eq!(error.code, "NX-E1105");
    assert_eq!(error.subcause.as_deref(), Some("active_borrow_conflict"));
    assert_eq!(error.stage, Stage::Ownership);
    assert_eq!(error.operation, Operation::OwnershipAccess);
    assert_eq!(
        error.primary_span.as_ref().map(|s| (s.start, s.end)),
        Some((30, 31))
    );
    assert_eq!(error.related_spans.len(), 2);
    assert_eq!(error.cause_chain.len(), 1);
    assert_eq!(error.data.len(), 5);
    assert_eq!(
        error.data.get("active_permission"),
        Some(&DataValue::String("read".into()))
    );
    assert_eq!(
        error.data.get("requested_access"),
        Some(&DataValue::String("write".into()))
    );
}

fn option_bindings(levels: usize) -> String {
    let mut source = String::from("let a0=1;");
    for index in 1..=levels {
        source.push_str(&format!("let a{index}=Some(a{});", index - 1));
    }
    source
}

fn array_bindings(levels: usize) -> String {
    let mut source = String::from("let a0=0;");
    for index in 1..=levels {
        source.push_str(&format!("let a{index}=[a{}];", index - 1));
    }
    source
}

fn depth_diagnostic(error: &nexo::diagnostic::Diagnostic, kind: &str, span: (u64, u64)) {
    assert_eq!(error.code, "NX-E0003");
    assert_eq!(error.subcause, None);
    assert_eq!(error.stage, Stage::Types);
    assert_eq!(error.operation, Operation::AnalysisLimit);
    assert_eq!(
        error.primary_span.as_ref().map(|s| (s.start, s.end)),
        Some(span)
    );
    assert!(error.related_spans.is_empty());
    assert!(error.cause_chain.is_empty());
    assert_eq!(error.data.len(), 5);
    assert_eq!(
        error.data.get("budget_kind"),
        Some(&DataValue::String(kind.into()))
    );
    assert_eq!(
        error.data.get("budget_scope"),
        Some(&DataValue::String("compilation".into()))
    );
    assert_eq!(
        error.data.get("incomplete_analysis"),
        Some(&DataValue::String("types".into()))
    );
    assert_eq!(error.data.get("limit"), Some(&DataValue::UInt(128)));
    assert_eq!(error.data.get("observed"), Some(&DataValue::UInt(129)));
}

#[test]
fn value_depth_is_one_causal_diagnostic_across_generators() {
    let valid = format!(
        "gen fn G()->I64{{{}yield 1;}}fn Main(){{let g1=G();let g2=G();}}",
        option_bindings(127)
    );
    compile(&valid).unwrap();

    let invalid = format!(
        "gen fn G()->I64{{{}yield 1;}}gen fn H()->I64{{{}yield 1;}}fn Main(){{}}",
        option_bindings(129),
        option_bindings(129)
    );
    let errors = rejected(&invalid);
    assert_eq!(errors.len(), 1);
    depth_diagnostic(&errors[0], "value_type_depth", (2376, 2386));
}

#[test]
fn array_depth_reports_each_invalid_function() {
    let source = format!(
        "fn F(){{{}}}fn G(){{{}}}fn Main(){{}}",
        array_bindings(129),
        array_bindings(129)
    );
    let errors = rejected(&source);
    assert_eq!(errors.len(), 2);
    depth_diagnostic(&errors[0], "array_type_depth", (1855, 1861));
    depth_diagnostic(&errors[1], "array_type_depth", (3718, 3724));
}

#[test]
fn unresolved_payload_and_independent_match_residual_are_both_reported() {
    compile("fn Main(){let o:Option<Result<I64,I64>>=if true then{value:None;}else{value:Some(Err(1));};match o{None if true=>{},Some(Ok(p))=>{},_=>{}}}").unwrap();

    let source = "fn Main(){match if true then{value:None;}else{value:Some(Err(1));}{None if true=>{},Some(Ok(p))=>{}}}";
    let errors = rejected(source);
    assert_eq!(errors.len(), 4);
    assert_eq!(errors[0].code, "NX-E1304");
    assert_eq!(errors[0].subcause.as_deref(), Some("non_exhaustive"));
    assert_eq!(
        errors[0].primary_span.as_ref().map(|s| (s.start, s.end)),
        Some((10, 15))
    );
    assert_eq!(errors[0].data.len(), 1);
    assert_eq!(
        errors[0].data.get("missing_pattern"),
        Some(&DataValue::String("None".into()))
    );
    for (error, (start, end, component)) in errors[1..].iter().zip([
        (35, 39, "$.some.ok"),
        (52, 64, "$.some.ok"),
        (57, 63, "$.ok"),
    ]) {
        assert_eq!(error.code, "NX-E0113");
        assert_eq!(error.subcause, None);
        assert_eq!(error.stage, Stage::Types);
        assert_eq!(error.operation, Operation::TypesInfer);
        assert_eq!(
            error.primary_span.as_ref().map(|s| (s.start, s.end)),
            Some((start, end))
        );
        assert_eq!(error.data.len(), 1);
        assert_eq!(
            error.data.get("unresolved_components"),
            Some(&DataValue::Array(vec![DataValue::String(component.into())]))
        );
    }
}
