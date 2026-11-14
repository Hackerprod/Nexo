mod common;
use nexo::interp::{MockEnvironment, Outcome};
use std::{fs, process::Command};

fn cli(source: &str, profile: &str, command: &str) -> (i32, String) {
    let path = std::env::temp_dir().join(format!(
        "nexo-j14-{}-{}-{}.n",
        std::process::id(),
        command,
        source.len()
    ));
    fs::write(&path, source).unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_nexo"))
        .args([
            command,
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
fn published_new_operation_work_totals() {
    let cases = [
        (
            "minimum",
            "gen fn G()->I64{}fn Main(){let g=G();Drop(g);}",
            12,
        ),
        ("T01", "fn Main(){let x:Option<I64>=Some(1);}", 9),
        ("T02", "fn Main(){let x:Option<I64>=None;}", 5),
        ("T03", "fn Main(){let f=fn [](){};f();Drop(f);}", 15),
        (
            "T04",
            "gen fn G()->I64{}fn Main(){var g=G();let n=Iterator.Next(edit g);Drop(n);Drop(g);}",
            24,
        ),
        (
            "T05",
            "gen fn G()->I64{yield 1;}fn Main(){var g=G();let n=Iterator.Next(edit g);Drop(n);Drop(g);}",
            27,
        ),
        ("T06", "gen fn G()->I64{}fn Main(){for x in G(){}}", 12),
        (
            "T07",
            "gen fn G()->I64{yield 1;}fn Main(){for x in G(){}}",
            20,
        ),
        (
            "T08",
            "gen fn G()->I64{yield 1;}fn Main(){for x in G(){break;}}",
            16,
        ),
        (
            "T09",
            "gen fn G()->I64{yield 1;}fn Main(){for x in G(){continue;}}",
            21,
        ),
        (
            "T10",
            "gen fn G(n:view I64)->I64{}fn Main(){let g=G(1);Drop(g);}",
            15,
        ),
        (
            "T11",
            "fn Main(){let x=1;let f=fn [copy x](){};Drop(f);}",
            14,
        ),
        (
            "T12",
            "fn Main(){let x=1;let f=fn [view x](){};Drop(f);}",
            14,
        ),
        ("T13", "fn Main(){let f=fn [](){};move f();}", 11),
        (
            "T14",
            "gen fn G()->I64{}fn Main(){var g=G();let a=Iterator.Next(edit g);Drop(a);let b=Iterator.Next(edit g);Drop(b);Drop(g);}",
            36,
        ),
    ];
    let mut mismatches = Vec::new();
    for (name, source, expected) in cases {
        let (check_exit, checked) = cli(source, "f0-0003-b-v1", "check");
        assert_eq!(check_exit, 0, "{name}: {checked}");
        assert!(checked.contains("\"diagnostics\":[]"), "{name}: {checked}");
        let (run_exit, report) = cli(source, "f0-0003-b-v1", "run");
        assert_eq!(run_exit, 0, "{name}: {report}");
        assert!(report.contains("\"diagnostics\":[]"), "{name}: {report}");
        assert!(report.contains("\"observations\":[]"), "{name}: {report}");
        let execution = common::run_j(name, source, MockEnvironment::default()).unwrap();
        assert_eq!(execution.outcome, Outcome::Unit, "{name}");
        assert!(
            execution.diagnostics.is_empty(),
            "{name}: {:?}",
            execution.diagnostics
        );
        if execution.work_consumed != expected {
            mismatches.push((name, expected, execution.work_consumed));
        }
    }
    assert!(mismatches.is_empty(), "{mismatches:?}");
}

#[test]
fn published_frame_depths_count_main_closure_advance_and_calls() {
    let cases = [
        (
            7,
            "fn F1(){F2();}fn F2(){F3();}fn F3(){F4();}fn F4(){}gen fn G()->I64{F1();}fn Main(){let g=G();let f=fn [move g](){var h=move g;let x=Iterator.Next(edit h);};move f();}",
            0,
        ),
        (
            8,
            "fn F1(){F2();}fn F2(){F3();}fn F3(){F4();}fn F4(){F5();}fn F5(){}gen fn G()->I64{F1();}fn Main(){let g=G();let f=fn [move g](){var h=move g;let x=Iterator.Next(edit h);};move f();}",
            0,
        ),
        (
            9,
            "fn F1(){F2();}fn F2(){F3();}fn F3(){F4();}fn F4(){F5();}fn F5(){F6();}fn F6(){}gen fn G()->I64{F1();}fn Main(){let g=G();let f=fn [move g](){var h=move g;let x=Iterator.Next(edit h);};move f();}",
            1,
        ),
    ];
    for (depth, source, expected_exit) in cases {
        let (check_exit, checked) = cli(source, "f0-0003-b-test-frames-8-v1", "check");
        assert_eq!(check_exit, 0, "{depth}: {checked}");
        assert!(checked.contains("\"diagnostics\":[]"), "{checked}");
        let (exit, report) = cli(source, "f0-0003-b-test-frames-8-v1", "run");
        assert_eq!(exit, expected_exit, "{depth}: {report}");
        assert!(report.contains("\"observations\":[]"), "{report}");
        assert!(
            report.contains("\"reserved_observation_bytes\":0"),
            "{report}"
        );
        if depth == 9 {
            assert!(report.contains("\"code\":\"R0002\""), "{report}");
            assert!(
                report.contains("\"runtime_budget_kind\":\"call_frames\""),
                "{report}"
            );
            assert!(report.contains("\"limit\":8,\"observed\":9"), "{report}");
            assert!(report.contains("\"start\":64,\"end\":68"), "{report}");
        } else {
            assert!(report.contains("\"diagnostics\":[]"), "{report}");
        }
    }
}

#[test]
fn cold_generator_headers_enforce_the_cells_64_boundary() {
    for (count, exit) in [(63, 0), (64, 0), (65, 1)] {
        let mut source = String::from("gen fn G()->I64{}fn Main(){");
        for i in 0..count {
            source.push_str(&format!("let g{i}=G();"));
        }
        for i in 0..count {
            source.push_str(&format!("Drop(g{i});"));
        }
        source.push('}');
        let (status, report) = cli(&source, "f0-0003-b-test-cells-64-v1", "run");
        assert_eq!(status, exit, "{count}: {report}");
        if count == 65 {
            assert!(report.contains("\"code\":\"R0002\""), "{report}");
            assert!(
                report.contains("\"runtime_budget_kind\":\"live_value_cells\""),
                "{report}"
            );
            assert!(report.contains("\"limit\":64,\"observed\":65"), "{report}");
        } else {
            assert!(report.contains("\"diagnostics\":[]"), "{report}");
        }
        assert!(
            report.contains("\"reserved_observation_bytes\":0"),
            "{report}"
        );
    }
}

#[test]
fn permanent_closure_array_for_edit_and_recursive_state_exclusions_remain_public() {
    for (source, feature, stage) in [
        ("fn Main(){let f=fn [](){};let a=[f];}", "closure", "types"),
        (
            "gen fn G()->I64{}fn Main(){for edit x in G(){}}",
            "for_iteration",
            "parse",
        ),
        (
            "gen fn G()->I64{let g=G();}fn Main(){}",
            "generator",
            "types",
        ),
    ] {
        for command in ["check", "run"] {
            let (exit, report) = cli(source, "f0-0003-b-v1", command);
            assert_eq!(exit, 1, "{source}: {report}");
            assert!(report.contains("\"code\":\"NX-E0006\""), "{report}");
            assert!(
                report.contains(&format!("\"feature_id\":\"{feature}\"")),
                "{report}"
            );
            assert!(
                report.contains("\"pending_scope\":\"later_adr\""),
                "{report}"
            );
            assert!(
                report.contains(&format!("\"stage\":\"{stage}\"")),
                "{report}"
            );
        }
    }
}

#[test]
fn retained_generator_resource_keeps_its_close_reservation() {
    let source = "gen fn G(r:move MockResource)->I64{}fn Main(){let g=G(MockOpen(4));Drop(g);}";
    let (exit, report) = cli(source, "f0-0003-b-test-observations-64-v1", "run");
    assert_eq!(exit, 0, "{report}");
    assert!(
        report.contains("\"events\":[\"open:0:4\",\"close:0:4\"]"),
        "{report}"
    );
    assert!(report.contains("\"observation_bytes\":47"), "{report}");
    assert!(
        report.contains("\"reserved_observation_bytes\":0"),
        "{report}"
    );

    let source = "gen fn G(r:move MockResource)->I64{}fn Main(){let a=G(MockOpen(4));let b=G(MockOpen(5));Drop(a);Drop(b);}";
    let (exit, report) = cli(source, "f0-0003-b-test-observations-64-v1", "run");
    assert_eq!(exit, 1, "{report}");
    assert!(
        report.contains("\"runtime_budget_kind\":\"observation_bytes\""),
        "{report}"
    );
    assert!(
        report.contains("\"events\":[\"open:0:4\",\"close:0:4\"]"),
        "{report}"
    );
    assert!(
        report.contains("\"reserved_observation_bytes\":0"),
        "{report}"
    );
}

#[test]
fn next_output_tag_coexists_with_suspended_generator_headers() {
    for (count, exit) in [(63, 0), (64, 1)] {
        let mut source = String::from("gen fn G()->I64{}fn Main(){");
        for i in 0..count {
            source.push_str(&format!("var g{i}=G();"));
        }
        source.push_str("let n=Iterator.Next(edit g0);");
        for i in 0..count {
            source.push_str(&format!("Drop(g{i});"));
        }
        source.push('}');
        let (status, report) = cli(&source, "f0-0003-b-test-cells-64-v1", "run");
        assert_eq!(status, exit, "{count}: {report}");
        if count == 64 {
            assert!(
                report.contains("\"runtime_budget_kind\":\"live_value_cells\""),
                "{report}"
            );
            assert!(report.contains("\"limit\":64,\"observed\":65"), "{report}");
        } else {
            assert!(report.contains("\"diagnostics\":[]"), "{report}");
        }
        assert!(
            report.contains("\"reserved_observation_bytes\":0"),
            "{report}"
        );
    }
}

#[test]
fn match_charges_only_pattern_nodes_reached_in_preorder() {
    let first = "fn Main(){let o:Option<I64>=Some(1);match o{Some(_)=>{},None=>{}}}";
    let second = "fn Main(){let o:Option<I64>=Some(1);match o{None=>{},Some(_)=>{}}}";
    let one = common::run_j("match-first.n", first, MockEnvironment::default()).unwrap();
    let two = common::run_j("match-second.n", second, MockEnvironment::default()).unwrap();
    assert_eq!(one.outcome, Outcome::Unit);
    assert_eq!(two.outcome, Outcome::Unit);
    assert_eq!(two.work_consumed, one.work_consumed + 1);
}

#[test]
fn cold_empty_slots_do_not_release_phantom_cells_on_drop() {
    for (extras, exit) in [(1, 0), (2, 1)] {
        let mut source = String::from("gen fn G()->I64{let x=1;}fn Main(){");
        for i in 0..64 {
            source.push_str(&format!("let g{i}=G();"));
        }
        source.push_str("Drop(g0);");
        for i in 0..extras {
            source.push_str(&format!("let extra{i}=G();"));
        }
        for i in 0..extras {
            source.push_str(&format!("Drop(extra{i});"));
        }
        source.push('}');
        let (status, report) = cli(&source, "f0-0003-b-test-cells-64-v1", "run");
        assert_eq!(status, exit, "{extras}: {report}");
        if extras == 2 {
            assert!(
                report.contains("\"runtime_budget_kind\":\"live_value_cells\""),
                "{report}"
            );
            assert!(report.contains("\"limit\":64,\"observed\":65"), "{report}");
        } else {
            assert!(report.contains("\"diagnostics\":[]"), "{report}");
        }
    }
}
