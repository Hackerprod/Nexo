use std::fs;
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

fn check(source: &str) -> String {
    static NEXT: AtomicU64 = AtomicU64::new(0);
    let path = std::env::temp_dir().join(format!(
        "nexo-j06-names-{}-{}.n",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    fs::write(&path, source).unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_nexo"))
        .args([
            "check",
            path.to_str().unwrap(),
            "--profile",
            "f0-0003-b-v1",
            "--format",
            "json",
        ])
        .output()
        .unwrap();
    fs::remove_file(path).unwrap();
    assert_eq!(output.status.code(), Some(1));
    String::from_utf8(output.stdout).unwrap()
}

#[test]
fn exhaustive_capture_list_rejects_one_free_binding() {
    let output = check("fn Main(){let x=1;let f=fn []()->I64{return x;};}");
    assert_eq!(output.matches("\"code\":").count(), 1, "{output}");
    assert!(output.contains("\"code\":\"NX-E1101\""), "{output}");
    assert!(output.contains("\"stage\":\"names\""), "{output}");
    assert!(
        output.contains("\"operation\":\"names.resolve\""),
        "{output}"
    );
    assert!(output.contains("\"start\":44,\"end\":45"), "{output}");
    assert!(output.contains("\"role\":\"declaration\""), "{output}");
    assert!(output.contains("\"start\":14,\"end\":15"), "{output}");
    assert!(output.contains("\"data\":{}"), "{output}");
    assert!(output.contains("\"cause_chain\":[]"), "{output}");
    assert!(output.contains("\"observations\":[]"), "{output}");
}

#[test]
fn repeated_capture_reports_second_name_and_first_declaration() {
    let output = check("fn Main(){let x=1;let f=fn [copy x,copy x](){};}");
    assert_eq!(output.matches("\"code\":").count(), 1, "{output}");
    assert!(output.contains("\"code\":\"NX-E1102\""), "{output}");
    assert!(output.contains("\"stage\":\"names\""), "{output}");
    assert!(
        output.contains("\"operation\":\"names.resolve\""),
        "{output}"
    );
    assert!(output.contains("\"start\":40,\"end\":41"), "{output}");
    assert!(
        output.contains("\"role\":\"previous_declaration\""),
        "{output}"
    );
    assert!(output.contains("\"start\":33,\"end\":34"), "{output}");
    assert!(output.contains("\"data\":{}"), "{output}");
    assert!(output.contains("\"cause_chain\":[]"), "{output}");
    assert!(output.contains("\"observations\":[]"), "{output}");
}

#[test]
fn unresolved_closure_parameter_reports_its_declaration() {
    let output = check("fn Main(){let f=fn [](x:view){return x;};}");
    assert_eq!(output.matches("\"code\":").count(), 1, "{output}");
    assert!(output.contains("\"code\":\"NX-E1111\""), "{output}");
    assert!(output.contains("\"stage\":\"types\""), "{output}");
    assert!(output.contains("\"operation\":\"types.check\""), "{output}");
    assert!(output.contains("\"start\":22,\"end\":23"), "{output}");
    assert!(output.contains("\"role\":\"declaration\""), "{output}");
    assert!(output.contains("\"data\":{}"), "{output}");
    assert!(output.contains("\"cause_chain\":[]"), "{output}");
    assert!(output.contains("\"observations\":[]"), "{output}");
}

#[test]
fn resource_capture_cannot_copy() {
    let output = check("fn Main(){let r=MockOpen(1);let f=fn [copy r](){};}");
    assert_eq!(output.matches("\"code\":").count(), 1, "{output}");
    assert!(output.contains("\"code\":\"NX-E0201\""), "{output}");
    assert!(output.contains("\"stage\":\"ownership\""), "{output}");
    assert!(output.contains("\"operation\":\"value.copy\""), "{output}");
    assert!(output.contains("\"start\":43,\"end\":44"), "{output}");
    assert!(output.contains("\"role\":\"declaration\""), "{output}");
    assert!(output.contains("\"start\":14,\"end\":15"), "{output}");
    assert!(output.contains("\"data\":{}"), "{output}");
    assert!(output.contains("\"cause_chain\":[]"), "{output}");
    assert!(output.contains("\"observations\":[]"), "{output}");
}

#[test]
fn copied_scalar_capture_constructs_and_cleans_without_calling_body() {
    let source = "fn Main(){let x=1;let f=fn [copy x](){};}";
    let program = nexo::ir::compile_for_internal_harness("j06-positive.n", source, true).unwrap();
    let execution = nexo::interp::run_scalar(&program, Default::default());
    assert_eq!(execution.outcome, nexo::interp::Outcome::Unit);
    assert!(execution.diagnostics.is_empty());
    assert!(execution.observations.observations().is_empty());
}

#[test]
fn moved_resource_capture_closes_once_without_calling_body() {
    let source = "fn Main(){let r=MockOpen(1);let f=fn [move r](){};}";
    let program = nexo::ir::compile_for_internal_harness("j06-resource.n", source, true).unwrap();
    let execution = nexo::interp::run_scalar(&program, Default::default());
    assert_eq!(execution.outcome, nexo::interp::Outcome::Unit);
    assert!(execution.diagnostics.is_empty());
    assert_eq!(
        execution
            .observations
            .events()
            .into_iter()
            .filter(|event| event.starts_with("close:"))
            .count(),
        1
    );
}

#[test]
fn objective_14_executes_through_library_harness() {
    let source = include_str!("../corpus/objetivo/14_closure_capture.n");
    let execution =
        nexo::ir::run_for_internal_harness("objective-14.n", source, Default::default()).unwrap();
    assert_eq!(execution.outcome, nexo::interp::Outcome::Unit);
    assert!(execution.diagnostics.is_empty());
    assert!(execution.observations.observations().is_empty());
}

#[test]
fn local_call_constrains_unannotated_closure_parameter() {
    let source = "fn Main(){let f=fn [](x:view){return x;};AssertEq(f(3),3);}";
    let program = nexo::ir::compile_for_internal_harness("j06-infer.n", source, true).unwrap();
    let execution = nexo::interp::run_scalar(&program, Default::default());
    assert_eq!(execution.outcome, nexo::interp::Outcome::Unit);
    assert!(execution.diagnostics.is_empty());
    assert!(execution.observations.observations().is_empty());
}

#[test]
fn copied_array_capture_keeps_independent_value() {
    let source =
        "fn Main(){var a=[1];let f=fn [copy a]()->I64{return a[0];};a[0]=2;AssertEq(f(),1);}";
    let program = nexo::ir::compile_for_internal_harness("j06-array.n", source, true).unwrap();
    let execution = nexo::interp::run_scalar(&program, Default::default());
    assert_eq!(execution.outcome, nexo::interp::Outcome::Unit);
    assert!(execution.diagnostics.is_empty());
}

#[test]
fn copied_mutable_capture_changes_only_its_environment_field() {
    let source = "fn Main(){var x=1;var f=fn [copy x](){x=2;};edit f();AssertEq(x,1);}";
    let program = nexo::ir::compile_for_internal_harness("copy-mutable.n", source, true).unwrap();
    let execution = nexo::interp::run_scalar(&program, Default::default());
    assert_eq!(execution.outcome, nexo::interp::Outcome::Unit);
    assert!(
        execution.diagnostics.is_empty(),
        "{:?}",
        execution.diagnostics
    );
    assert!(execution.observations.observations().is_empty());
}
