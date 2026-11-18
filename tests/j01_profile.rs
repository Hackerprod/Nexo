mod common;
use std::sync::atomic::{AtomicU64, Ordering};
use std::{fs, process::Command};

use nexo::{prelude::PENDING_INTRINSICS, union::UnionKind};

fn check(source: &str, profile: &str) -> (i32, String) {
    static NEXT: AtomicU64 = AtomicU64::new(0);
    let path = std::env::temp_dir().join(format!(
        "nexo-j01-{}-{}-{}.n",
        std::process::id(),
        source.len(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    fs::write(&path, source).unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_nexo"))
        .args([
            "check",
            path.to_str().unwrap(),
            "--profile",
            profile,
            "--format",
            "json",
        ])
        .output()
        .unwrap();
    fs::remove_file(&path).unwrap();
    (
        output.status.code().unwrap(),
        String::from_utf8(output.stdout).unwrap(),
    )
}

#[test]
fn j01_profile_accepts_a2_and_its_eight_reduced_selectors() {
    let profiles = [
        "f0-0003-b-v1",
        "f0-0003-b-test-types-32-v1",
        "f0-0003-b-test-flow-32-v1",
        "f0-0003-b-test-flow-global-32-v1",
        "f0-0003-b-test-effects-32-v1",
        "f0-0003-b-test-ir-32-v1",
        "f0-0003-b-test-frames-8-v1",
        "f0-0003-b-test-cells-64-v1",
        "f0-0003-b-test-observations-64-v1",
    ];
    for profile in profiles {
        let (status, output) = check("fn Main(){let x=1;AssertEq(x,1);}", profile);
        assert_eq!(status, 0, "{profile}: {output}");
        assert!(output.contains(&format!("\"profile_id\":\"{profile}\"")));
        assert!(output.contains("\"kind\":\"checked\""));
    }
}

#[test]
fn j01_types_reduced_profile_stops_at_the_declared_frontier() {
    let source = format!("fn Main(){{var a=1;{}}}", "a=a+1;".repeat(11));
    let (normal_status, _) = check(&source, "f0-0003-b-v1");
    assert_eq!(normal_status, 0);
    let (status, output) = check(&source, "f0-0003-b-test-types-32-v1");
    assert_eq!(status, 1, "{output}");
    assert!(
        output.contains("\"budget_kind\":\"type_solver_steps\""),
        "{output}"
    );
    assert!(output.contains("\"limit\":32,\"observed\":33"), "{output}");
}

#[test]
fn j01_effect_diagnostic_names_the_new_model_without_changing_a2() {
    let source = "fn Main() with no_io{MockWriteI64(1);}";
    for (profile, model) in [
        ("f0-0003-b-v1", "f0-0003-sim-v1"),
        ("f0-0002-b-v2", "f0-0002-sim-v1"),
    ] {
        let (status, output) = check(source, profile);
        assert_eq!(status, 1, "{output}");
        assert!(
            output.contains(&format!("\"effect_model\":\"{model}\"")),
            "{output}"
        );
    }
}

#[test]
fn completed_capabilities_are_public_without_changing_a2_rejection() {
    for (source, feature) in [
        ("fn Main(){let f=fn [](){};}", "closure"),
        ("gen fn G(){}fn Main(){}", "generator"),
    ] {
        let (status, output) = check(source, "f0-0003-b-v1");
        assert_eq!(status, 0, "{output}");
        assert!(output.contains("\"diagnostics\":[]"), "{output}");
        let (old_status, old_output) = check(source, "f0-0002-b-v2");
        assert_eq!(old_status, 1, "{old_output}");
        assert!(
            old_output.contains(&format!("\"feature_id\":\"{feature}\"")),
            "{old_output}"
        );
    }
}

#[test]
fn array_iteration_keeps_its_permanent_feature_boundary() {
    let (status, output) = check("fn Main(){for x in [1] {}}", "f0-0003-b-v1");
    assert_eq!(status, 1);
    assert!(
        output.contains("\"feature_id\":\"for_iteration\""),
        "{output}"
    );
    assert!(
        output.contains("\"pending_scope\":\"later_adr\""),
        "{output}"
    );
    assert!(output.contains("\"stage\":\"types\""), "{output}");
}

#[test]
fn generator_and_iteration_analysis_diagnostics_are_public() {
    for (source, code, stage) in [
        (
            "gen fn G(r:view MockResource)->I64{yield 1;}fn Main(){}",
            "NX-E1110",
            "ownership",
        ),
        (
            "gen fn G()->I64{yield 1;}fn Main() with no_suspend{var g=G();Iterator.Next(edit g);}",
            "NX-E0414",
            "effects",
        ),
        ("fn Main(){for x in 1{}}", "NX-E0102", "types"),
        ("fn Main(){break;}", "NX-E1326", "types"),
    ] {
        let (status, output) = check(source, "f0-0003-b-v1");
        assert_eq!(status, 1, "{source}: {output}");
        assert!(output.contains(&format!("\"code\":\"{code}\"")), "{output}");
        assert!(
            output.contains(&format!("\"stage\":\"{stage}\"")),
            "{output}"
        );
        assert!(!output.contains("\"pending_scope\":\"J14\""), "{output}");
    }
}

#[test]
fn objective_14_executes_through_the_public_profile() {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/corpus/objetivo/14_closure_capture.n"
    );
    let source = fs::read_to_string(path).unwrap();
    let execution = common::run_j(path, &source, Default::default()).unwrap();
    assert_eq!(execution.outcome, nexo::interp::Outcome::Unit);
    assert!(execution.diagnostics.is_empty());
    assert!(execution.observations.observations().is_empty());
    for command in ["check", "run"] {
        let output = Command::new(env!("CARGO_BIN_EXE_nexo"))
            .args([
                command,
                path,
                "--profile",
                "f0-0003-b-v1",
                "--format",
                "json",
            ])
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(0));
        let text = String::from_utf8(output.stdout).unwrap();
        assert!(text.contains("\"diagnostics\":[]"), "{text}");
        assert!(text.contains("\"observations\":[]"), "{text}");
    }
}

#[test]
fn closure_analysis_rejections_are_public() {
    for (source, code) in [
        (
            "fn Main(){let x=1;let f=fn []()->I64{return x;};}",
            "NX-E1101",
        ),
        (
            "fn Main(){var x=1;let f=fn [view x](){};x=2;Drop(f);}",
            "NX-E1105",
        ),
        (
            "fn Main(){var x=1;var f=fn [edit x](){x=2;};f();}",
            "NX-E1107",
        ),
        (
            "fn Main(){let a=[1];let f=fn [copy a]() with {create(no_alloc),call(unrestricted)}{};}",
            "NX-E0412",
        ),
    ] {
        let (status, output) = check(source, "f0-0003-b-v1");
        assert_eq!(status, 1, "{output}");
        assert_eq!(output.matches("\"code\":").count(), 1, "{output}");
        assert!(output.contains(&format!("\"code\":\"{code}\"")), "{output}");
        assert!(!output.contains("\"pending_scope\":\"J14\""), "{output}");
    }
}

#[test]
fn j05_match_guards_are_available_only_in_the_j_profile() {
    let source = "fn Main(){match true{true if true=>{},_=>{}}}";
    let (status, output) = check(source, "f0-0003-b-v1");
    assert_eq!(status, 0, "{output}");
    let (old_status, old_output) = check(source, "f0-0002-b-v2");
    assert_eq!(old_status, 1, "{old_output}");
    assert!(
        old_output.contains("\"feature_id\":\"match\""),
        "{old_output}"
    );
}

#[test]
fn j04_match_modes_are_available_only_in_the_j_profile() {
    let source = "fn Main(){let o=true;match move o{true=>{},false=>{}}}";
    let (new_status, new_output) = check(source, "f0-0003-b-v1");
    assert_eq!(new_status, 0, "{new_output}");
    let (old_status, old_output) = check(source, "f0-0002-b-v2");
    assert_eq!(old_status, 1, "{old_output}");
    assert!(
        old_output.contains("\"feature_id\":\"match\""),
        "{old_output}"
    );
}

#[test]
fn j01_pending_prelude_entries_have_j_scopes() {
    assert_eq!(
        PENDING_INTRINSICS.map(|entry| entry.identity),
        ["prelude:Some", "prelude:None", "prelude:Iterator.Next"]
    );
    assert_eq!(UnionKind::Option.variant_tag("Some").unwrap().0, 1);
    assert_eq!(UnionKind::Option.variant_tag("None").unwrap().0, 0);
    let (status, output) = check("fn Main(){Iterator.Next(1);}", "f0-0003-b-v1");
    assert_eq!(status, 1, "{output}");
    assert!(
        output.contains("\"subcause\":\"argument_mode_mismatch\""),
        "{output}"
    );
}

#[test]
fn j02_option_boundary_accepts_both_nominal_variants() {
    for source in [
        "fn Main(){let x:Option<I64>=None;}",
        "fn Main(){let x=Some(1);}",
    ] {
        let (status, output) = check(source, "f0-0003-b-v1");
        assert_eq!(status, 0, "{source}: {output}");
        let (old_status, old_output) = check(source, "f0-0002-b-v2");
        assert_eq!(old_status, 1, "{source}: {old_output}");
    }
}

#[test]
fn j01_malformed_option_and_some_keep_syntax_priority() {
    for source in ["fn Main(){let x:Option<I64=1;}", "fn Main(){Some(1,);}"] {
        let (status, output) = check(source, "f0-0003-b-v1");
        assert_eq!(status, 1, "{output}");
        assert!(!output.contains("\"pending_scope\":\"J02\""), "{output}");
    }
}

#[test]
fn j01_later_syntax_error_precedes_pending_option_constructor() {
    let source = "fn Main(){let x=Some(1);let y:Result=Ok(1);}";
    let (status, output) = check(source, "f0-0003-b-v1");
    assert_eq!(status, 1, "{output}");
    assert!(output.contains("\"code\":\"NX-E0002\""), "{output}");
    assert!(!output.contains("\"pending_scope\":\"J02\""), "{output}");
}

#[test]
fn j01_explain_describes_each_constructor_and_loop_subcause_separately() {
    for (code, first, second, first_text, second_text) in [
        (
            "NX-E1325",
            "constructor_arity",
            "constructor_type",
            "aridad incorrecta",
            "otra unión nominal",
        ),
        (
            "NX-E1326",
            "break_outside_loop",
            "continue_outside_loop",
            "break requiere",
            "continue requiere",
        ),
    ] {
        let first = nexo::diagnostic::explain(code, Some(first)).unwrap();
        let second = nexo::diagnostic::explain(code, Some(second)).unwrap();
        assert_eq!(first.len(), 1);
        assert_eq!(second.len(), 1);
        assert!(first[0].description.contains(first_text));
        assert!(second[0].description.contains(second_text));
        assert_ne!(first[0].description, second[0].description);
    }
}

#[test]
fn j01_explain_describes_partial_constructor_shapes() {
    for code in ["NX-E1325", "NX-E0102"] {
        let rows = nexo::diagnostic::explain(code, Some("constructor_shape")).unwrap();
        assert_eq!(rows.len(), 1);
        assert!(rows[0].description.contains("forma"));
        assert!(rows[0].negative_source.contains("match l0"));
        assert!(rows[0].positive_source.contains("match l0"));
    }
}
