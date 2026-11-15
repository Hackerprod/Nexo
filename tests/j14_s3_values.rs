use std::{
    fs,
    process::Command,
    sync::atomic::{AtomicUsize, Ordering},
};

static SOURCE_ID: AtomicUsize = AtomicUsize::new(0);

fn run(source: &str) -> (i32, String) {
    run_profile(source, "f0-0003-b-v1")
}

fn run_profile(source: &str, profile: &str) -> (i32, String) {
    let id = SOURCE_ID.fetch_add(1, Ordering::Relaxed);
    let path = std::env::temp_dir().join(format!("j14-s3-{}-{id}.n", std::process::id()));
    fs::write(&path, source).unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_nexo"))
        .args([
            "run",
            path.to_str().unwrap(),
            "--profile",
            profile,
            "--format",
            "json",
        ])
        .output()
        .unwrap();
    fs::remove_file(path).unwrap();
    (
        output.status.code().expect("runtime worker must not crash"),
        String::from_utf8(output.stdout).unwrap(),
    )
}

fn expected_unit(printed: Option<&str>, events: &[&str]) -> String {
    let stdout = printed.map_or_else(|| "[]".to_owned(), |text| format!("[\"{text}\"]"));
    let event_list = format!(
        "[{}]",
        events
            .iter()
            .map(|event| format!("\"{event}\""))
            .collect::<Vec<_>>()
            .join(",")
    );
    let mut observations = Vec::new();
    let mut bytes = 0;
    if let Some(text) = printed {
        observations.push(format!("{{\"channel\":\"stdout\",\"text\":\"{text}\"}}"));
        bytes += text.len() + 1;
    }
    for event in events {
        observations.push(format!("{{\"channel\":\"event\",\"text\":\"{event}\"}}"));
        bytes += "mock-resource:".len() + event.len() + 1;
    }
    format!(
        "{{\"schema_version\":1,\"profile_id\":\"f0-0003-b-v1\",\"outcome\":{{\"kind\":\"unit\"}},\"stdout\":{stdout},\"events\":{event_list},\"observations\":[{}],\"observation_bytes\":{bytes},\"reserved_observation_bytes\":0,\"diagnostics\":[]}}",
        observations.join(",")
    )
}

fn assert_unit(source: &str, printed: Option<&str>, events: &[&str]) {
    let (status, report) = run(source);
    assert_eq!(status, 0, "{report}");
    assert_eq!(report.trim_end(), expected_unit(printed, events));
}

fn array_source(depth: usize, body: &str) -> String {
    let mut source = String::from("fn Main(){let a0=[1];");
    for index in 1..depth {
        source.push_str(&format!("let a{index}=[a{}];", index - 1));
    }
    source.push_str(body);
    source.push('}');
    source
}

#[test]
fn nested_array_print_copy_equal_and_dispose_on_runtime_worker() {
    for depth in [115, 120, 127] {
        let body = format!(
            "Print(a{});let b=a{};AssertEq(b,a{});Drop(b);",
            depth - 1,
            depth - 1,
            depth - 1
        );
        let printed = format!("{}1{}", "[".repeat(depth), "]".repeat(depth));
        assert_unit(&array_source(depth, &body), Some(&printed), &[]);
    }
}

#[test]
fn array_operations_each_fit_the_runtime_worker_stack() {
    for depth in [115, 120, 127] {
        let name = format!("a{}", depth - 1);
        let printed = format!("{}1{}", "[".repeat(depth), "]".repeat(depth));
        assert_unit(
            &array_source(depth, &format!("Print({name});")),
            Some(&printed),
            &[],
        );
        assert_unit(
            &array_source(depth, &format!("let b={name};Drop(b);")),
            None,
            &[],
        );
        assert_unit(
            &array_source(depth, &format!("AssertEq({name},{name});")),
            None,
            &[],
        );
        assert_unit(&array_source(depth, &format!("Drop({name});")), None, &[]);
    }
}

#[test]
fn deep_array_print_uses_explicit_value_work() {
    let depth = 127;
    let printed = format!("{}1{}", "[".repeat(depth), "]".repeat(depth));
    assert_unit(&array_source(depth, "Print(a126);"), Some(&printed), &[]);
}

#[test]
fn deep_array_copy_uses_explicit_value_work() {
    assert_unit(&array_source(127, "let b=a126;Drop(b);"), None, &[]);
}

#[test]
fn deep_array_equality_uses_explicit_value_work() {
    assert_unit(&array_source(127, "AssertEq(a126,a126);"), None, &[]);
}

#[test]
fn deep_array_disposal_uses_explicit_value_work() {
    assert_unit(&array_source(127, "Drop(a126);"), None, &[]);
}

#[test]
fn deep_array_equality_finds_a_different_leaf() {
    let mut source = String::from("fn Main(){let a0=[1];let b0=[2];");
    for index in 1..127 {
        source.push_str(&format!(
            "let a{index}=[a{}];let b{index}=[b{}];",
            index - 1,
            index - 1
        ));
    }
    source.push_str("AssertEq(a126,b126);}");
    let (status, report) = run(&source);
    assert_eq!(status, 1, "{report}");
    assert!(report.contains("\"kind\":\"simulator_fault\""), "{report}");
    assert!(report.contains("\"code\":\"R0201\""), "{report}");
    assert!(
        report.contains("\"operation\":\"intrinsic.assert_eq\""),
        "{report}"
    );
    assert!(report.contains("\"observations\":[]"), "{report}");
    assert!(
        report.contains("\"reserved_observation_bytes\":0"),
        "{report}"
    );
}

#[test]
fn nested_owned_option_releases_the_resource_once() {
    for depth in [1, 2, 8, 127] {
        let mut source = String::from("fn Main(){let a0=Some(MockOpen(1));");
        for index in 1..depth {
            source.push_str(&format!("let a{index}=Some(move a{});", index - 1));
        }
        source.push('}');
        assert_unit(&source, None, &["open:0:1", "close:0:1"]);
    }
}

#[test]
fn direct_recursion_uses_255_and_256_frames_then_reports_257() {
    for (argument, expected_exit) in [(253, 0), (254, 0), (255, 1)] {
        let source = format!("fn F(n:view I64){{if 0<n{{F(n-1);}}}}fn Main(){{F({argument});}}");
        let (status, report) = run(&source);
        assert_eq!(status, expected_exit, "{argument}: {report}");
        if argument == 255 {
            assert!(report.contains("\"code\":\"R0002\""), "{report}");
            assert!(
                report.contains("\"runtime_budget_kind\":\"call_frames\""),
                "{report}"
            );
            assert!(
                report.contains("\"limit\":256,\"observed\":257"),
                "{report}"
            );
            assert!(report.contains("\"observations\":[]"), "{report}");
            assert!(
                report.contains("\"reserved_observation_bytes\":0"),
                "{report}"
            );
        } else {
            assert_eq!(report.trim_end(), expected_unit(None, &[]));
        }
    }
}

#[test]
fn nested_result_closure_and_generator_values_close_resources_once() {
    for depth in [0, 1, 2, 8] {
        let mut result = String::from("fn Main(){let r0=MockOpen(1);");
        for index in 1..=depth {
            let ty = format!(
                "{}MockResource{}",
                "Result<".repeat(index),
                ",I64>".repeat(index)
            );
            result.push_str(&format!("let r{index}:{ty}=Ok(move r{});", index - 1));
        }
        result.push_str(&format!("Drop(r{depth});}}"));
        assert_unit(&result, None, &["open:0:1", "close:0:1"]);

        let mut closure = String::from("fn Main(){let r=MockOpen(1);let f0=fn [move r](){};");
        for index in 1..=depth {
            closure.push_str(&format!("let f{index}=fn [move f{}](){{}};", index - 1));
        }
        closure.push_str(&format!("Drop(f{depth});}}"));
        assert_unit(&closure, None, &["open:0:1", "close:0:1"]);

        let count = depth.max(1);
        let mut generator = String::new();
        for index in 1..count {
            generator.push_str(&format!(
                "gen fn G{index}()->I64{{var g=G{}();let a=Iterator.Next(edit g);yield 1;}}",
                index + 1
            ));
        }
        generator.push_str(&format!("gen fn G{count}()->I64{{let r=MockOpen(1);yield 1;Drop(r);}}fn Main(){{var g=G1();let a=Iterator.Next(edit g);AssertEq(a,Some(1));Drop(g);}}"));
        assert_unit(&generator, None, &["open:0:1", "close:0:1"]);
    }
}

#[test]
fn nested_union_format_and_multiple_captures_keep_text_and_close_order() {
    assert_unit(
        "fn Main(){let a:Option<Result<I64,I64>>=Some(Ok(1));Print(a);}",
        Some("Some(Ok(1))"),
        &[],
    );
    assert_unit(
        "fn Main(){let r1=MockOpen(1);let r2=MockOpen(2);let o=Some(move r1);let f=fn [move o,move r2](){};Drop(f);}",
        None,
        &["open:0:1", "open:1:2", "close:1:2", "close:0:1"],
    );
}

#[test]
fn nested_option_cells_include_payload_and_final_unit() {
    for (levels, status) in [(63, 0), (64, 1)] {
        let mut source = String::from("fn Main(){let a0=1;");
        for index in 1..levels {
            source.push_str(&format!("let a{index}=Some(move a{});", index - 1));
        }
        source.push('}');
        let (actual, report) = run_profile(&source, "f0-0003-b-test-cells-64-v1");
        assert_eq!(actual, status, "{levels}: {report}");
        if levels == 64 {
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
fn dropping_nested_values_releases_cells_for_the_next_value() {
    let mut source = String::from("fn Main(){");
    for name in ["a", "b"] {
        source.push_str(&format!("let {name}0=1;"));
        for index in 1..40 {
            source.push_str(&format!(
                "let {name}{index}=Some(move {name}{});",
                index - 1
            ));
        }
        source.push_str(&format!("Drop({name}39);"));
    }
    source.push('}');
    let (status, report) = run_profile(&source, "f0-0003-b-test-cells-64-v1");
    assert_eq!(status, 0, "{report}");
    assert!(
        report.contains("\"outcome\":{\"kind\":\"unit\"}"),
        "{report}"
    );
    assert!(report.contains("\"diagnostics\":[]"), "{report}");
    assert!(
        report.contains("\"reserved_observation_bytes\":0"),
        "{report}"
    );
}

#[test]
fn closure_call_continuation_returns_to_assertion() {
    assert_unit(
        "fn Main(){let f=fn []()->I64{return 1;};AssertEq(f(),1);}",
        None,
        &[],
    );
}

#[test]
fn generator_next_continuation_returns_to_assertion() {
    assert_unit(
        "gen fn G()->I64{yield 1;}fn Main(){var g=G();let a=Iterator.Next(edit g);AssertEq(a,Some(1));}",
        None,
        &[],
    );
}
