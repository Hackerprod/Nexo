mod common;
use nexo::interp::{MockEnvironment, Outcome};
use nexo::ir::CompileError;
use std::process::Command;

fn run(source: &str) -> nexo::interp::Execution {
    common::run_j("<memory>", source, MockEnvironment::default()).unwrap()
}

#[test]
fn match_place_loop_exits_release_selection_loan() {
    for control in ["break", "continue"] {
        let source = format!(
            "fn Main(){{let mb=false;var i=0;while i<1{{i=i+1;match mb{{true=>{{{control};}},false=>{{}}}}}}}}"
        );
        let result = run(&source);
        assert_eq!(result.outcome, Outcome::Unit, "{source}");
    }
}

#[test]
fn match_place_break_in_closure_releases_loan_before_local() {
    let source = "gen fn G()->I64{yield 1;}fn Main(){let f=fn [](){for x in G(){let b=true;match b{true=>{break;},false=>{}}}};f();}";
    let result = run(source);
    assert_eq!(result.outcome, Outcome::Unit);
}

#[test]
fn match_place_yield_resumes_with_held_selection() {
    let source = "gen fn G(b:view Bool)->I64{match b{true=>{yield 1;},false=>{yield 0;}}}fn Main(){for x in G(true){Print(x);}}";
    let result = run(source);
    assert_eq!(result.outcome, Outcome::Unit);
    assert_eq!(result.observations.stdout(), ["1"]);
}

#[test]
fn match_place_return_and_propagation_release_selection_loan() {
    for source in [
        "fn Main(){let b=true;match b{true=>{return;},false=>{}}}",
        "fn F()->Result<(),I64>{let b=true;match b{true=>{let x=TryAdd(1,2)?;},false=>{}}return Ok(());}fn Main(){let x=F();}",
    ] {
        assert!(
            common::compile_j("<memory>", source, true).is_ok(),
            "{source}"
        );
    }
}

#[test]
fn match_edit_loan_is_incompatible_with_yield() {
    let source = "gen fn G()->I64{var o=Some(1);match edit o{Some(edit n)=>{yield 1;n=2;},None=>{}}}fn Main(){}";
    let CompileError::Rejected(errors) = common::compile_j("<memory>", source, true).unwrap_err()
    else {
        panic!("expected suspend safety diagnostic")
    };
    assert!(
        errors.iter().any(|error| error.code == "NX-E1204"),
        "{errors:?}"
    );
}

#[test]
fn match_read_resource_survives_yield_without_double_close() {
    let source = "gen fn G()->I64{let o=Some(MockOpen(1));match o{Some(r)=>{yield 1;Inspect(r);},None=>{}}}fn Main(){for x in G(){Print(x);}}";
    let result = run(source);
    assert_eq!(result.outcome, Outcome::Unit);
    assert_eq!(
        result.observations.events(),
        ["open:0:1", "inspect:0:1", "close:0:1"]
    );
    let stopped = "gen fn G()->I64{let o=Some(MockOpen(1));match o{Some(r)=>{yield 1;Inspect(r);},None=>{}}}fn Main(){var g=G();let n=Iterator.Next(edit g);Drop(g);}";
    let result = run(stopped);
    assert_eq!(result.outcome, Outcome::Unit);
    assert_eq!(result.observations.events(), ["open:0:1", "close:0:1"]);
    let failed = "gen fn G()->I64{let o=Some(MockOpen(1));match o{Some(r)=>{yield 1;Fail();Inspect(r);},None=>{}}}fn Main(){for x in G(){}}";
    let result = run(failed);
    assert_eq!(result.outcome, Outcome::SimulatorFault);
    assert_eq!(result.observations.events(), ["open:0:1", "close:0:1"]);
}

#[test]
fn for_exit_joins_only_reachable_edges() {
    let prefix = "gen fn G()->I64{yield 1;yield 2;}";
    let returning = format!(
        "{prefix}fn Main(){{let r=MockOpen(1);for x in G(){{Drop(r);return;}}Inspect(r);}}"
    );
    assert!(common::compile_j("<memory>", &returning, true).is_ok());
    let breaking =
        format!("{prefix}fn Main(){{let r=MockOpen(1);for x in G(){{Drop(r);break;}}Inspect(r);}}");
    let CompileError::Rejected(errors) =
        common::compile_j("<memory>", &breaking, true).unwrap_err()
    else {
        panic!("expected ownership diagnostic")
    };
    assert!(
        errors
            .iter()
            .any(|e| e.code == "NX-E0202" && e.subcause.as_deref() == Some("maybe_consumed")),
        "{errors:?}"
    );
}

#[test]
fn while_break_and_continue_close_the_lexical_loop() {
    for source in [
        "fn Main(){while true{break;}}",
        "fn Main(){while false{continue;}}",
    ] {
        let result = run(source);
        assert_eq!(result.outcome, Outcome::Unit);
        assert!(result.diagnostics.is_empty());
    }
    for (source, subcause, span) in [
        ("fn Main(){break;}", "break_outside_loop", (10, 15)),
        ("fn Main(){continue;}", "continue_outside_loop", (10, 18)),
    ] {
        let CompileError::Rejected(mut errors) =
            common::compile_j("<memory>", source, true).unwrap_err()
        else {
            panic!("wrong error");
        };
        assert_eq!(errors.len(), 1);
        let error = errors.remove(0);
        assert_eq!(error.code, "NX-E1326");
        assert_eq!(error.subcause.as_deref(), Some(subcause));
        let primary = error.primary_span.unwrap();
        assert_eq!((primary.start, primary.end), span);
    }
}

#[test]
fn for_moves_items_and_completes_generator() {
    let result = run(
        "gen fn G()->I64{yield 1;yield 2;}fn Main(){var n=0;for x in G(){n=n+x;}AssertEq(n,3);}",
    );
    assert_eq!(result.outcome, Outcome::Unit);
    assert!(result.diagnostics.is_empty());
}

#[test]
fn for_requires_owned_generator_and_rejects_edit_binding() {
    let cases = [
        (
            "gen fn G()->I64{yield 1;}fn Main(){let g=G();for x in g{Print(x);}}",
            "NX-E0201",
            None,
            (54, 55),
            "gen fn G()->I64{yield 1;}fn Main(){let g=G();for x in move g{Print(x);}}",
        ),
        (
            "gen fn G()->I64{yield 1;}fn Main(){for edit x in G(){}}",
            "NX-E0006",
            None,
            (39, 43),
            "gen fn G()->I64{yield 1;}fn Main(){for x in G(){}}",
        ),
        (
            "fn Main(){for x in 1{}}",
            "NX-E0102",
            Some("type_mismatch"),
            (19, 20),
            "gen fn G()->I64{yield 1;}fn Main(){for x in G(){}}",
        ),
    ];
    for (negative, code, subcause, span, positive) in cases {
        let CompileError::Rejected(mut errors) =
            common::compile_j("<memory>", negative, true).unwrap_err()
        else {
            panic!("wrong error: {negative}");
        };
        assert_eq!(errors.len(), 1, "{negative}");
        let error = errors.remove(0);
        assert_eq!(error.code, code, "{negative}");
        assert_eq!(error.subcause.as_deref(), subcause, "{negative}");
        let primary = error.primary_span.unwrap();
        assert_eq!((primary.start, primary.end), span, "{negative}");
        assert_eq!(run(positive).outcome, Outcome::Unit);
    }
}

#[test]
fn for_implicit_next_carries_suspend_effect() {
    let negative = "gen fn G()->I64{yield 1;}fn Main() with no_suspend{for x in G(){}}";
    let CompileError::Rejected(mut errors) =
        common::compile_j("<memory>", negative, true).unwrap_err()
    else {
        panic!("wrong error");
    };
    assert_eq!(errors.len(), 1);
    let error = errors.remove(0);
    assert_eq!(error.code, "NX-E0414");
    let primary = error.primary_span.unwrap();
    assert_eq!((primary.start, primary.end), (51, 54));
    assert_eq!(
        run("gen fn G()->I64{}fn Main() with no_suspend{for x in G(){}}").outcome,
        Outcome::Unit
    );
}

#[test]
fn goal_generator_sum_runs_through_public_profile() {
    let source = include_str!("../corpus/objetivo/15_generator_sum.n");
    let result = run(source);
    assert_eq!(result.outcome, Outcome::Unit);
    assert!(result.diagnostics.is_empty());
    assert!(result.observations.observations().is_empty());
}

#[test]
fn ex04_success_events_match_adr_section_11() {
    let source = include_str!("../corpus/objetivo/11_ex04_generador.n");
    let environment = MockEnvironment::parse(br#"{"schema_version":1,"mock_read_records":[],"mock_write_i64":[{"ok":null},{"ok":null},{"ok":null}],"mock_request":[{"ok":"3"}],"mock_reply":[]}"#).unwrap();
    let result = common::run_j("<memory>", source, environment).unwrap();
    assert_eq!(result.outcome, Outcome::Ok);
    assert_eq!(result.observations.stdout(), ["0", "1", "4"]);
    assert_eq!(
        result.observations.events(),
        [
            "request:ok:3",
            "open:0:4",
            "inspect:0:4",
            "write_i64:0:ok",
            "inspect:0:4",
            "write_i64:1:ok",
            "inspect:0:4",
            "write_i64:4:ok",
            "close:0:4",
        ]
    );
    assert_eq!(result.observations.reserved_bytes(), 0);
}

#[test]
fn for_closes_each_owned_item_before_the_next_advance() {
    let prefix = "gen fn G()->MockResource{yield MockOpen(1);yield MockOpen(2);}";
    let stop = run(&format!(
        "{prefix}fn Main(){{for r in G(){{Inspect(r);break;}}}}"
    ));
    assert_eq!(stop.outcome, Outcome::Unit);
    assert_eq!(
        stop.observations.events(),
        ["open:0:1", "inspect:0:1", "close:0:1"]
    );
    let all = run(&format!(
        "{prefix}fn Main(){{for r in G(){{Inspect(r);continue;}}}}"
    ));
    assert_eq!(all.outcome, Outcome::Unit);
    assert_eq!(
        all.observations.events(),
        [
            "open:0:1",
            "inspect:0:1",
            "close:0:1",
            "open:1:2",
            "inspect:1:2",
            "close:1:2",
        ]
    );
}

#[test]
fn nested_loop_control_targets_the_nearest_loop() {
    let result = run(
        "gen fn G()->I64{yield 1;yield 2;}fn Main(){var n=0;while n<2{for x in G(){n=n+x;break;}n=n+1;}AssertEq(n,2);}",
    );
    assert_eq!(result.outcome, Outcome::Unit);
}

#[test]
fn ex04_error_and_early_exit_paths_close_the_generator() {
    let source = include_str!("../corpus/objetivo/11_ex04_generador.n");
    type Ex04Case = (
        &'static [u8],
        Outcome,
        &'static [&'static str],
        &'static [&'static str],
    );
    let cases: [Ex04Case; 4] = [
        (br#"{"schema_version":1,"mock_read_records":[],"mock_write_i64":[],"mock_request":[{"ok":"0"}],"mock_reply":[]}"#,Outcome::Ok,&[],&["request:ok:0","open:0:4","close:0:4"]),
        (br#"{"schema_version":1,"mock_read_records":[],"mock_write_i64":[],"mock_request":[{"err":"10"}],"mock_reply":[]}"#,Outcome::Err(10),&[],&["request:err:10"]),
        (br#"{"schema_version":1,"mock_read_records":[],"mock_write_i64":[],"mock_request":[{"ok":"1001"}],"mock_reply":[]}"#,Outcome::Err(41),&[],&["request:ok:1001"]),
        (br#"{"schema_version":1,"mock_read_records":[],"mock_write_i64":[{"ok":null},{"err":"40"},{"ok":null}],"mock_request":[{"ok":"3"}],"mock_reply":[]}"#,Outcome::Err(40),&["0"],&["request:ok:3","open:0:4","inspect:0:4","write_i64:0:ok","inspect:0:4","write_i64:1:err:40","close:0:4"]),
    ];
    for (json, outcome, stdout, events) in cases {
        let environment = MockEnvironment::parse(json).unwrap();
        let result = common::run_j("<memory>", source, environment).unwrap();
        assert_eq!(result.outcome, outcome);
        assert_eq!(result.observations.stdout(), stdout);
        assert_eq!(result.observations.events(), events);
        assert_eq!(result.observations.reserved_bytes(), 0);
    }
    let break_source = source.replace("MockWriteI64(valor)?;", "MockWriteI64(valor)?;break;");
    let environment = MockEnvironment::parse(br#"{"schema_version":1,"mock_read_records":[],"mock_write_i64":[{"ok":null},{"ok":null},{"ok":null}],"mock_request":[{"ok":"3"}],"mock_reply":[]}"#).unwrap();
    let result = common::run_j("<memory>", &break_source, environment).unwrap();
    assert_eq!(result.outcome, Outcome::Ok);
    assert_eq!(result.observations.stdout(), ["0"]);
    assert_eq!(
        result.observations.events(),
        [
            "request:ok:3",
            "open:0:4",
            "inspect:0:4",
            "write_i64:0:ok",
            "close:0:4"
        ]
    );
}

#[test]
fn nested_generator_for_and_empty_closure_remain_suspend_safe() {
    let source = "gen fn H()->I64{yield 1;}gen fn G()->I64{let f=fn [](){};for x in H(){yield x;f();}}fn Main(){for x in G(){AssertEq(x,1);}}";
    let result = run(source);
    assert_eq!(result.outcome, Outcome::Unit);
    assert!(result.diagnostics.is_empty());
}

#[test]
fn return_from_for_discards_suspended_iterator_once() {
    let source = "gen fn G(r:move MockResource)->I64{yield 1;yield 2;}fn Main(){for x in G(MockOpen(5)){return;}}";
    let result = run(source);
    assert_eq!(result.outcome, Outcome::Unit);
    assert_eq!(result.observations.events(), ["open:0:5", "close:0:5"]);
}

#[test]
fn loop_control_does_not_cross_an_inner_closure() {
    let source = "fn Main(){while true{let f=fn [](){break;};break;}}";
    let CompileError::Rejected(mut errors) =
        common::compile_j("<memory>", source, true).unwrap_err()
    else {
        panic!("wrong error");
    };
    assert_eq!(errors.len(), 1);
    let error = errors.remove(0);
    assert_eq!(error.code, "NX-E1326");
    assert_eq!(error.subcause.as_deref(), Some("break_outside_loop"));
}

#[test]
fn goal_programs_execute_through_public_cli() {
    for name in ["11_ex04_generador.n", "15_generator_sum.n"] {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("corpus/objetivo")
            .join(name);
        for command in ["check", "run"] {
            let mut invocation = Command::new(env!("CARGO_BIN_EXE_nexo"));
            invocation.args([
                command,
                path.to_str().unwrap(),
                "--profile",
                "f0-0003-b-v1",
                "--format",
                "json",
            ]);
            let environment = path.with_extension("env.json");
            if command == "run" && environment.exists() {
                invocation.args(["--mock-env", environment.to_str().unwrap()]);
            }
            let output = invocation.output().unwrap();
            assert_eq!(output.status.code(), Some(0), "{name} {command}");
            let text = String::from_utf8(output.stdout).unwrap();
            assert!(text.contains("\"diagnostics\":[]"), "{text}");
        }
    }
}

#[test]
fn for_backedge_rejects_repeated_consumption_of_an_outer_owner() {
    let source =
        "gen fn G()->I64{yield 1;yield 2;}fn Main(){let r=MockOpen(5);for x in G(){Drop(move r);}}";
    let result = common::compile_j("<memory>", source, true);
    let CompileError::Rejected(errors) = result.unwrap_err() else {
        panic!("wrong error");
    };
    assert_eq!(errors.len(), 1);
    assert_eq!(errors[0].code, "NX-E0202");
    let positive =
        "gen fn G()->I64{yield 1;yield 2;}fn Main(){for x in G(){let r=MockOpen(x);Drop(r);}}";
    assert_eq!(run(positive).outcome, Outcome::Unit);
}
