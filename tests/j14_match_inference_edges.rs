use std::{
    fs,
    process::Command,
    sync::atomic::{AtomicUsize, Ordering},
};

static SOURCE_ID: AtomicUsize = AtomicUsize::new(0);

fn report(command: &str, source: &str) -> (i32, String) {
    let id = SOURCE_ID.fetch_add(1, Ordering::Relaxed);
    let path = std::env::temp_dir().join(format!("nexo-j14-match-{}-{id}.n", std::process::id()));
    fs::write(&path, source).unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_nexo"))
        .args([
            command,
            path.to_str().unwrap(),
            "--profile",
            "f0-0003-b-v1",
            "--format",
            "json",
        ])
        .output()
        .unwrap();
    fs::remove_file(path).unwrap();
    (
        output.status.code().expect("Nexo must not crash"),
        String::from_utf8(output.stdout).unwrap(),
    )
}

fn accepted(source: &str) {
    for command in ["check", "run"] {
        let (status, output) = report(command, source);
        assert_eq!(status, 0, "{command}: {source}: {output}");
        assert!(output.contains("\"diagnostics\":[]"), "{output}");
    }
}

#[test]
fn guard_constraint_resolves_none_payload_after_pattern_binding() {
    accepted("fn Main(){match None{Some(x) if x==1=>{},_=>{}}}");
    let source = "fn Main(){match None{Some(x) if true=>{},_=>{}}}";
    for command in ["check", "run"] {
        let (status, output) = report(command, source);
        assert_eq!(status, 1, "{output}");
        assert_eq!(
            output.matches("\"code\":\"NX-E0113\"").count(),
            1,
            "{output}"
        );
        assert!(output.contains("\"subcause\":null"), "{output}");
        assert!(output.contains("\"start\":16,\"end\":20"), "{output}");
        assert!(
            output.contains("\"unresolved_components\":[\"$.some\"]"),
            "{output}"
        );
        assert!(
            !output.contains("NX-E1304") && !output.contains("internal_error"),
            "{output}"
        );
    }
}

#[test]
fn partial_option_result_coverage_preserves_open_payload_and_residual() {
    let head = "fn Main(){match if true then{value:None;}else{value:Some(Err(1));}";
    let rows = [
        ("{None=>{},Some(Ok(p))=>{},_=>{}}", None),
        ("{None=>{},Some(Ok(p))=>{}}", Some("Some(Err(0))")),
        ("{None if true=>{},Some(Ok(p))=>{},_=>{}}", None),
        ("{None if true=>{},Some(Ok(p))=>{}}", Some("None")),
        ("{None=>{},_=>{}}", None),
        ("{None if true=>{},_=>{}}", None),
        ("{Some(Ok(p))=>{},_=>{}}", None),
    ];
    for (arms, missing) in rows {
        let source = format!("{head}{arms}}}");
        for command in ["check", "run"] {
            let (status, output) = report(command, &source);
            assert_eq!(status, 1, "{source}: {output}");
            assert_eq!(
                output.matches("\"code\":\"NX-E0113\"").count(),
                3,
                "{source}: {output}"
            );
            for (start, end, path) in [
                (35, 39, "$.some.ok"),
                (52, 64, "$.some.ok"),
                (57, 63, "$.ok"),
            ] {
                assert!(
                    output.contains(&format!("\"start\":{start},\"end\":{end}"))
                        && output.contains(&format!("\"unresolved_components\":[\"{path}\"]")),
                    "{source}: {output}"
                );
            }
            assert_eq!(
                output.matches("\"code\":\"NX-E1304\"").count(),
                usize::from(missing.is_some()),
                "{source}: {output}"
            );
            if let Some(witness) = missing {
                assert!(
                    output.contains("\"subcause\":\"non_exhaustive\""),
                    "{output}"
                );
                assert!(output.contains("\"start\":10,\"end\":15"), "{output}");
                assert!(
                    output.contains(&format!("\"missing_pattern\":\"{witness}\"")),
                    "{output}"
                );
            }
            assert!(
                !output.contains("unreachable_arm") && !output.contains("internal_error"),
                "{output}"
            );
        }
        let complete_arms = if missing.is_some() {
            arms.replacen("=>{}}", "=>{},_=>{}}", 1)
        } else {
            arms.to_owned()
        };
        let positive = format!(
            "fn Main(){{let o:Option<Result<I64,I64>>=if true then{{value:None;}}else{{value:Some(Err(1));}};match o{complete_arms}}}"
        );
        accepted(&positive);
    }
    if_join_retains_each_unresolved_constructor_obligation();
}

#[test]
fn arm_body_annotation_does_not_constrain_scrutinee_payload() {
    let source = "fn Main(){match if true then{value:None;}else{value:Some(Err(1));}{None if true=>{let v:Option<Result<I64,I64>>=None;},Some(Ok(p))=>{},_=>{}}}";
    for command in ["check", "run"] {
        let (status, output) = report(command, source);
        assert_eq!(status, 1, "{output}");
        assert_eq!(
            output.matches("\"code\":\"NX-E0113\"").count(),
            3,
            "{output}"
        );
        assert!(
            !output.contains("NX-E1304") && !output.contains("internal_error"),
            "{output}"
        );
    }
    accepted(
        "fn Main(){let o:Option<Result<I64,I64>>=if true then{value:None;}else{value:Some(Err(1));};match o{None if true=>{let v:Option<Result<I64,I64>>=None;},Some(Ok(p))=>{},_=>{}}}",
    );
}

fn if_join_retains_each_unresolved_constructor_obligation() {
    let source = "fn Main(){let x=if true then{value:None;}else{value:Some(Err(1));};}";
    for command in ["check", "run"] {
        let (status, output) = report(command, source);
        assert_eq!(status, 1, "{output}");
        assert_eq!(
            output.matches("\"code\":\"NX-E0113\"").count(),
            3,
            "{output}"
        );
        for (start, end, path) in [
            (35, 39, "$.some.ok"),
            (52, 64, "$.some.ok"),
            (57, 63, "$.ok"),
        ] {
            assert!(
                output.contains(&format!("\"start\":{start},\"end\":{end}")),
                "{output}"
            );
            assert!(
                output.contains(&format!("\"unresolved_components\":[\"{path}\"]")),
                "{output}"
            );
        }
        assert!(!output.contains("internal_error"), "{output}");
    }
    accepted(
        "fn Main(){let x:Option<Result<I64,I64>>=if true then{value:None;}else{value:Some(Err(1));};}",
    );
}

fn return_and_try_edges_close_owner_once_with_pending_left_operand() {
    for source in [
        "fn F()->I64{let r=MockOpen(7);return 1+(if false then{value:2;}else{return 3;});}fn Main(){AssertEq(F(),3);}",
        "fn F()->Result<I64,I64>{let r=MockOpen(7);let x=1+(if false then{value:2;}else{value:Err(3)?;});return Ok(x);}fn Main(){let v=F();Drop(v);}",
    ] {
        let (check_status, check_output) = report("check", source);
        assert_eq!(check_status, 0, "{check_output}");
        let (run_status, run_output) = report("run", source);
        assert_eq!(run_status, 0, "{run_output}");
        assert!(
            run_output.contains("\"events\":[\"open:0:7\",\"close:0:7\"]"),
            "{run_output}"
        );
        assert!(
            run_output.contains("\"reserved_observation_bytes\":0"),
            "{run_output}"
        );
        assert!(run_output.contains("\"diagnostics\":[]"), "{run_output}");
    }
}

#[test]
fn pending_operand_survives_selection_and_cleans_on_loop_exit() {
    for source in [
        "fn Main(){let x=1+(match (match true{true=>{value:true;},false=>{value:false;}}){true=>{value:1;},false=>{value:2;}});}",
        "fn Main(){var i=0;while i<2{let x=3+(if i==0 then{value:1;}else{break;});Print(x);i=i+1;}}",
        "fn Main(){var i=0;while i<2{let x=3+(if i==0 then{value:1;}else{i=i+1;continue;});Print(x);i=i+1;}}",
        "fn Main(){var i=0;while i<2{let x=3+(match i{0=>{value:1;},_=>{break;}});Print(x);i=i+1;}}",
        "fn Main(){let a=[3,(match true{true=>{value:1;},false=>{value:2;}})];}",
        "fn F(r:move MockResource,x:view I64){Drop(r);}fn Main(){F(MockOpen(1),match true{true=>{value:1;},false=>{value:2;}});}",
    ] {
        accepted(source);
    }
    return_and_try_edges_close_owner_once_with_pending_left_operand();
}
