use std::{fs, process::Command};

use nexo::{prelude::PENDING_INTRINSICS, union::UnionKind};

fn check(source: &str, profile: &str) -> (i32, String) {
    let path = std::env::temp_dir().join(format!(
        "nexo-j01-{}-{}.n",
        std::process::id(),
        source.len()
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
fn j01_pending_capabilities_are_scoped_without_changing_a2_rejection() {
    for (source, scope, stage, feature) in [
        (
            "fn Main(){let x:Option<I64>=None;}",
            "J02",
            "types",
            "option",
        ),
        (
            "fn Main(){match true{true=>{},false=>{}}}",
            "J03",
            "types",
            "match",
        ),
        ("fn Main(){let f=fn [](){};}", "J06", "types", "closure"),
        ("gen fn G(){}fn Main(){}", "J10", "types", "generator"),
        (
            "fn Main(){for x in [1] {}}",
            "J13",
            "parse",
            "for_iteration",
        ),
    ] {
        let (status, output) = check(source, "f0-0003-b-v1");
        assert_eq!(status, 1, "{output}");
        assert!(output.contains("\"code\":\"NX-E0006\""), "{output}");
        assert!(
            output.contains("\"feature_id\":\"implementation_limit\""),
            "{output}"
        );
        assert!(
            output.contains(&format!("\"pending_scope\":\"{scope}\"")),
            "{output}"
        );
        assert!(
            output.contains(&format!("\"stage\":\"{stage}\"")),
            "{output}"
        );
        let (old_status, old_output) = check(source, "f0-0002-b-v2");
        assert_eq!(old_status, 1, "{old_output}");
        assert!(
            old_output.contains(&format!("\"feature_id\":\"{feature}\"")),
            "{old_output}"
        );
    }
}

#[test]
fn j01_pending_prelude_entries_have_j_scopes() {
    assert_eq!(
        PENDING_INTRINSICS.map(|entry| entry.identity),
        ["prelude:Some", "prelude:None", "prelude:Iterator.Next"]
    );
    assert_eq!(UnionKind::Option.variant_tag("Some").unwrap().0, 1);
    assert_eq!(UnionKind::Option.variant_tag("None").unwrap().0, 0);
    for (source, scope) in [
        ("fn Main(){let x=Some(1);}", "J02"),
        ("fn Main(){Iterator.Next(1);}", "J11"),
    ] {
        let (status, output) = check(source, "f0-0003-b-v1");
        assert_eq!(status, 1, "{output}");
        assert!(
            output.contains(&format!("\"pending_scope\":\"{scope}\"")),
            "{output}"
        );
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
