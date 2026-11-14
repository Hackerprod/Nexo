use nexo::diagnostic::{Diagnostic, Operation, Stage};
use nexo::ir::{CompileError, compile_for_internal_harness};
use std::process::Command;

fn accepted(source: &str) {
    let program = compile_for_internal_harness("validation.n", source, true).unwrap();
    let result = nexo::interp::run_scalar(&program, Default::default());
    assert_eq!(result.outcome, nexo::interp::Outcome::Unit);
    assert!(result.diagnostics.is_empty(), "{:?}", result.diagnostics);
}

fn rejected(source: &str, code: &str, subcause: Option<&str>) -> Box<Diagnostic> {
    let result = compile_for_internal_harness("validation.n", source, true);
    let Err(CompileError::Rejected(errors)) = result else {
        panic!("expected source diagnostic for {source}: {result:?}");
    };
    assert_eq!(errors.len(), 1, "{source}");
    assert_eq!(errors[0].code, code, "{source}");
    assert_eq!(errors[0].subcause.as_deref(), subcause, "{source}");
    errors[0].validate("validation.n", source).unwrap();
    errors.into_iter().next().unwrap()
}

fn rejected_each(source: &str, code: &str, subcause: Option<&str>) {
    let Err(CompileError::Rejected(errors)) =
        compile_for_internal_harness("validation.n", source, true)
    else {
        panic!("expected source diagnostic for {source}");
    };
    assert!(!errors.is_empty(), "{source}");
    for error in errors {
        assert_eq!(error.code, code, "{source}");
        assert_eq!(error.subcause.as_deref(), subcause, "{source}");
        error.validate("validation.n", source).unwrap();
    }
}

fn rejected_contains(source: &str, code: &str, subcause: Option<&str>) {
    let Err(CompileError::Rejected(errors)) =
        compile_for_internal_harness("validation.n", source, true)
    else {
        panic!("expected source diagnostic for {source}");
    };
    assert!(
        errors
            .iter()
            .any(|error| { error.code == code && error.subcause.as_deref() == subcause }),
        "{source}: {errors:?}"
    );
    for error in errors {
        error.validate("validation.n", source).unwrap();
    }
}

#[test]
fn closure_return_infers_result_without_arrow() {
    accepted("fn Main(){let f=fn [](x:view I64){return x;};}");
    accepted("fn Main(){let f=fn [](){return 1;};}");
    accepted("fn Main(){let f=fn [](){return;};}");
    accepted("fn Main(){let f=fn [](x:view I64){if true{return x;}return 2;};}");
    rejected(
        "fn Main(){let f=fn [](){if true{return 1;}return;};}",
        "NX-E0102",
        Some("type_mismatch"),
    );
    accepted("fn Main(){let f=fn [](x:view I64){return x;};AssertEq(f(1),1);}");
    rejected(
        "fn Main(){let f=fn [](x:view){return x;};}",
        "NX-E1111",
        None,
    );
}

#[test]
fn copied_and_moved_fields_keep_source_mutability() {
    for mode in ["copy", "move"] {
        rejected(
            &format!("fn Main(){{let x=1;let f=fn [{mode} x](){{x=7;}};}}"),
            "NX-E0105",
            None,
        );
        accepted(&format!(
            "fn Main(){{var x=1;let f=fn [{mode} x](){{x=7;}};}}"
        ));
        rejected(
            &format!("fn Main(){{let a=[1];let f=fn [{mode} a](){{a[0]=7;}};}}"),
            "NX-E0105",
            None,
        );
        accepted(&format!(
            "fn Main(){{var a=[1];let f=fn [{mode} a](){{a[0]=7;}};}}"
        ));
        rejected(
            &format!(
                "fn Main(){{let o=Some(1);let f=fn [{mode} o](){{match edit o{{Some(edit n)=>{{n=2;}},None=>{{}}}}}};}}"
            ),
            "NX-E0105",
            None,
        );
        accepted(&format!(
            "fn Main(){{var o=Some(1);let f=fn [{mode} o](){{match edit o{{Some(edit n)=>{{n=2;}},None=>{{}}}}}};}}"
        ));
    }
}

#[test]
fn exclusive_capture_conflict_has_serializable_diagnostic() {
    for tail in [
        "let g=fn [view x](){};",
        "let g=fn (){let y=x;};",
        "let g=fn (){Print(x);};",
    ] {
        rejected(
            &format!("fn Main(){{var x=1;let f=fn [edit x](){{}};{tail}}}"),
            "NX-E1105",
            Some("active_borrow_conflict"),
        );
    }
    rejected(
        "fn Main(){var f2=fn [](){};let f=fn [edit f2](){};Drop(f2);}",
        "NX-E1105",
        Some("active_borrow_conflict"),
    );
    accepted("fn Main(){var x=1;let f=fn [view x](){};let g=fn [view x](){};}");
}

#[test]
fn compound_data_domain_excludes_nested_closures() {
    rejected(
        "fn Main(){let f=fn [](){};Print(Some(f));}",
        "NX-E0102",
        Some("type_mismatch"),
    );
    rejected_contains(
        "fn Main(){let f=fn [](){};AssertEq(Some(f),Some(f));}",
        "NX-E0102",
        Some("type_mismatch"),
    );
    rejected(
        "fn Main(){let f=fn [](){};Print(Some(Some(f)));}",
        "NX-E0102",
        Some("type_mismatch"),
    );
    rejected(
        "fn Main(){let f=fn [](){};let r=if true then{value:Ok(f);}else{value:Err(1);};Print(r);}",
        "NX-E0102",
        Some("type_mismatch"),
    );
    accepted("fn Main(){Print(Some(1));AssertEq(Some(1),Some(1));}");
    accepted("fn Main(){let r=if true then{value:Ok(1);}else{value:Err(2);};Print(r);}");
}

#[test]
fn cli_serializes_closure_rejections_without_null() {
    for (source, code) in [
        (
            "fn Main(){var x=1;let f=fn [edit x](){};let g=fn [view x](){};}",
            "NX-E1105",
        ),
        ("fn Main(){let f=fn [](x:view I64){return x;};}", "NX-E0006"),
    ] {
        let file = std::env::temp_dir().join(format!(
            "nexo-closure-diagnostic-{}-{}.n",
            std::process::id(),
            source.len()
        ));
        std::fs::write(&file, source).unwrap();
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
            let json = String::from_utf8(output.stdout).unwrap();
            assert_eq!(output.status.code(), Some(1), "{source}: {json}");
            assert!(json.contains(code), "{source}: {json}");
            assert!(!json.contains("\"diagnostics\":[null]"), "{source}: {json}");
            assert!(!json.contains("internal_error"), "{source}: {json}");
        }
        std::fs::remove_file(file).unwrap();
    }
}

#[test]
fn edit_capture_requires_mutable_source() {
    let error = rejected(
        "fn Main(){let x=1;let f=fn [edit x](){};}",
        "NX-E0105",
        None,
    );
    assert_eq!(
        error
            .primary_span
            .as_ref()
            .map(|span| (span.start, span.end)),
        Some((33, 34))
    );
    accepted("fn Main(){var x=1;let f=fn [edit x](){};Drop(f);}");
}

#[test]
fn nested_capture_cannot_escalate_view_permission() {
    rejected(
        "fn Main(){var x=1;let f=fn [view x](){let g=fn [edit x](){};};}",
        "NX-E1105",
        Some("permission_escalation"),
    );
    rejected(
        "fn F(p:view I64){let f=fn [move p](){};}fn Main(){}",
        "NX-E1105",
        Some("permission_escalation"),
    );
    rejected(
        "fn Main(){var x=1;let f=fn [view x](){let g=fn [move x](){};};}",
        "NX-E1105",
        Some("permission_escalation"),
    );
    rejected(
        "fn Main(){var x=1;let f=fn (){let g=fn [edit x](){};};}",
        "NX-E1105",
        Some("permission_escalation"),
    );
    accepted("fn Main(){var x=1;let f=fn [view x](){let g=fn [view x](){};};Drop(f);}");
}

#[test]
fn captured_closure_call_respects_field_permission() {
    accepted("fn Main(){var f=fn [](){};var g=fn [edit f](){f();};edit g();}");
    accepted("fn Main(){let f=fn [](){};let g=fn [move f](){move f();};Drop(g);}");
    accepted("fn Main(){var f=fn [](){};var g=fn [move f](){edit f();};edit g();}");
    rejected(
        "fn Main(){let f=fn [](){};let g=fn [edit f](){f();};}",
        "NX-E0105",
        None,
    );
    rejected_contains(
        "fn Main(){let f=fn [](){};let g=fn [view f](){move f();};g();}",
        "NX-E1105",
        Some("permission_escalation"),
    );
    accepted(
        "fn Main(){var x=1;var o=Some(fn [edit x](){x=2;});match edit o{Some(edit g)=>{edit g();},None=>{}}}",
    );
    accepted(
        "fn Main(){var x=Some(fn [](){});var g=fn [move x](){match edit x{Some(edit h)=>{edit h();},None=>{}}};edit g();}",
    );
}

#[test]
fn independent_view_captures_coexist() {
    accepted("fn Main(){var x=1;let f=fn [view x](){};let g=fn [view x](){};Drop(g);Drop(f);}");
    rejected(
        "fn Main(){var x=1;let f=fn [view x](){};x=2;Drop(f);}",
        "NX-E1105",
        Some("active_borrow_conflict"),
    );
}

#[test]
fn closure_values_cannot_be_printed_or_compared() {
    rejected_each(
        "fn Main(){let f=fn [](){};AssertEq(f,f);}",
        "NX-E0102",
        Some("type_mismatch"),
    );
    rejected(
        "fn Main(){let f=fn [](){};Print(f);}",
        "NX-E0102",
        Some("type_mismatch"),
    );
    rejected(
        "fn Main(){let f=fn [](){};let same=f==f;}",
        "NX-E0102",
        Some("type_mismatch"),
    );
    accepted("fn Main(){AssertEq(Some(1),Some(1));Print(Some(1));}");
}

#[test]
fn arrays_reject_transitive_closure_elements() {
    rejected(
        "fn Main(){let f=fn [](){};let a=[Some(f)];}",
        "NX-E0006",
        None,
    );
    accepted("fn Main(){let f=fn [](){};let a=Some(f);Drop(a);}");
}

#[test]
fn option_of_copyable_closure_can_be_captured_by_copy() {
    accepted("fn Main(){let o=Some(fn [](){});let g=fn [copy o](){};Drop(g);}");
    rejected(
        "fn Main(){let r=MockOpen(1);let o=Some(fn [move r](){});let g=fn [copy o](){};}",
        "NX-E0201",
        None,
    );
}

#[test]
fn ordinary_call_receiver_mode_keeps_valid_diagnostic() {
    let error = rejected("fn F(){}fn Main(){edit (F());}", "NX-E0006", None);
    assert_eq!(error.stage, Stage::Names);
    assert_eq!(error.operation, Operation::NamesResolve);
    assert_eq!(
        error
            .primary_span
            .as_ref()
            .map(|span| (span.start, span.end)),
        Some((18, 22))
    );
    accepted("fn F(){}fn Main(){F();}");
}

#[test]
fn empty_match_arm_pattern_has_serializable_syntax_diagnostic() {
    rejected("fn Main(){match 1{=>{}}}", "NX-E0002", None);
    accepted("fn Main(){match 1{_=>{}}}");
}
