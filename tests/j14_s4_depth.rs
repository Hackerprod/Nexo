use nexo::check::check_proposed;
use nexo::diagnostic::{DataValue, Operation, Stage};
use std::{
    fs,
    process::Command,
    sync::atomic::{AtomicUsize, Ordering},
};

static SOURCE_ID: AtomicUsize = AtomicUsize::new(0);

fn j_check(source: &str) -> (i32, String) {
    j_command("check", source)
}

fn j_command(command: &str, source: &str) -> (i32, String) {
    let id = SOURCE_ID.fetch_add(1, Ordering::Relaxed);
    let path = std::env::temp_dir().join(format!("j14-s4-depth-{}-{id}.n", std::process::id()));
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
        output.status.code().unwrap(),
        String::from_utf8(output.stdout).unwrap(),
    )
}

fn accepted_both(source: &str) {
    for command in ["check", "run"] {
        let (status, report) = j_command(command, source);
        assert_eq!(status, 0, "{command}: {report}");
        assert!(report.contains("\"diagnostics\":[]"), "{report}");
    }
}

fn rejected_both(source: &str, kind: &str, observed: u64, span: (u64, u64)) {
    for command in ["check", "run"] {
        let (status, report) = j_command(command, source);
        assert_eq!(status, 1, "{command}: {report}");
        for expected in [
            "\"code\":\"NX-E0003\"".to_owned(),
            "\"subcause\":null".to_owned(),
            "\"stage\":\"types\"".to_owned(),
            "\"operation\":\"analysis.limit\"".to_owned(),
            "\"budget_scope\":\"compilation\"".to_owned(),
            "\"incomplete_analysis\":\"types\"".to_owned(),
            "\"related_spans\":[]".to_owned(),
            "\"cause_chain\":[]".to_owned(),
            "\"feature_id\":null".to_owned(),
            format!("\"budget_kind\":\"{kind}\""),
            "\"limit\":128".to_owned(),
            format!("\"observed\":{observed}"),
            format!("\"start\":{}", span.0),
            format!("\"end\":{}", span.1),
        ] {
            assert!(report.contains(&expected), "{expected}: {report}");
        }
        assert_eq!(
            report.matches("\"code\":\"NX-E0003\"").count(),
            1,
            "{report}"
        );
        assert_eq!(report.matches("\"code\":").count(), 1, "{report}");
    }
}

fn options(levels: usize) -> String {
    format!("{}I64{}", "Option<".repeat(levels), ">".repeat(levels))
}

fn option_bindings(levels: usize) -> String {
    let mut source = String::from("let a0=1;");
    for index in 1..=levels {
        source.push_str(&format!("let a{index}=Some(a{});", index - 1));
    }
    source
}

fn generator_with_deep_temporary(retained_slot: bool) -> String {
    let make = format!(
        "fn Make()->{}{{{}return a127;}}",
        options(127),
        option_bindings(127)
    );
    let (binding, first) = if retained_slot {
        ("let x=Some(Make());", "move x")
    } else {
        ("", "Some(Make())")
    };
    format!(
        "{make}gen fn G()->I64{{{binding}AssertEq({first},(if true then{{yield 1;value:Some(Make());}}else{{value:Some(Make());}}));}}fn Main(){{var g=G();let a=Iterator.Next(edit g);}}"
    )
}

#[test]
fn dynamic_temporary_does_not_extend_generator_layout_but_owned_slot_does() {
    accepted_both(&generator_with_deep_temporary(false));
    let invalid = generator_with_deep_temporary(true);
    let start = invalid.rfind("G()").unwrap() as u64;
    rejected_both(&invalid, "value_type_depth", 129, (start, start + 3));
}

fn parameter_state(levels: usize, outer: bool, late: bool) -> String {
    let generator = format!(
        "gen fn G(x:move {})->I64{{yield 1;Drop(x);}}",
        options(levels)
    );
    let call = if outer {
        format!("Some(G(move a{levels}))")
    } else {
        format!("G(move a{levels})")
    };
    let main = format!("fn Main(){{{}let g={call};}}", option_bindings(levels));
    if late {
        format!("{main}{generator}")
    } else {
        format!("{generator}{main}")
    }
}

#[test]
fn generator_parameter_slot_and_late_declaration_charge_original_constructor() {
    for late in [false, true] {
        accepted_both(&parameter_state(127, false, late));
        let positive = parameter_state(126, true, late);
        // The positive changes the field by exactly one Option level.
        accepted_both(&positive);
        let invalid = parameter_state(127, true, late);
        let start = invalid.find("Some(G(move a127))").unwrap() as u64;
        rejected_both(&invalid, "value_type_depth", 129, (start, start + 18));
    }
}

fn local_state(before_yield: bool, outer: bool) -> String {
    let body = option_bindings(127);
    let tail = if before_yield {
        "Drop(a127);yield 1;"
    } else {
        "yield 1;Drop(a127);"
    };
    let call = if outer { "Some(G())" } else { "G()" };
    format!("gen fn G()->I64{{{body}{tail}}}fn Main(){{let g={call};}}")
}

#[test]
fn generator_local_slot_counts_after_drop_before_yield() {
    for before_yield in [false, true] {
        accepted_both(&local_state(before_yield, false));
        let invalid = local_state(before_yield, true);
        let start = invalid.find("Some(G())").unwrap() as u64;
        rejected_both(&invalid, "value_type_depth", 129, (start, start + 9));
    }
}

fn nested_generators(count: usize) -> String {
    let mut source = String::new();
    for index in 1..count {
        source.push_str(&format!(
            "gen fn G{index}()->I64{{var g=G{}();let a=Iterator.Next(edit g);yield 1;}}",
            index + 1
        ));
    }
    source.push_str(&format!("gen fn G{count}()->I64{{yield 1;}}fn Main(){{var g=G1();let a=Iterator.Next(edit g);AssertEq(a,Some(1));}}"));
    source
}

#[test]
fn nested_generator_slots_stop_at_128() {
    for count in [127, 128] {
        accepted_both(&nested_generators(count));
    }
    let invalid = nested_generators(129);
    let start = invalid.rfind("G1()").unwrap() as u64;
    rejected_both(&invalid, "value_type_depth", 129, (start, start + 4));
}

#[test]
fn nested_generator_depth_reports_only_the_first_excess() {
    for count in [130, 200] {
        let source = nested_generators(count);
        let start = source.find("G2()").unwrap() as u64;
        // G1 is the first source node whose resolved state exceeds the bound.
        rejected_both(&source, "value_type_depth", 129, (start, start + 4));
    }
}

fn hidden_for_state(levels: usize) -> String {
    let parameter = options(levels);
    let bindings = option_bindings(levels);
    format!(
        "gen fn H(x:move {parameter})->I64{{yield 1;Drop(x);}}gen fn G()->I64{{{bindings}for y in H(move a{levels}){{Drop(y);}}yield 1;}}fn Main(){{let g=G();}}"
    )
}

#[test]
fn generator_hidden_for_iterator_is_an_owned_slot() {
    accepted_both(&hidden_for_state(126));
    let invalid = hidden_for_state(127);
    let start = invalid.rfind("G()").unwrap() as u64;
    rejected_both(&invalid, "value_type_depth", 129, (start, start + 3));
}

fn local_closure_state(mode: &str, levels: usize) -> String {
    let bindings = option_bindings(levels);
    format!(
        "gen fn G()->I64{{{bindings}let f=fn [{mode} a{levels}](){{}};yield 1;}}fn Main(){{let g=G();}}"
    )
}

#[test]
fn generator_local_closure_counts_only_owned_captures() {
    accepted_both(&local_closure_state("view", 127));
    accepted_both(&local_closure_state("copy", 126));
    for mode in ["copy", "move"] {
        let invalid = local_closure_state(mode, 127);
        let start = invalid.rfind("G()").unwrap() as u64;
        rejected_both(&invalid, "value_type_depth", 129, (start, start + 3));
    }
}

fn array_local_state(levels: usize) -> String {
    let mut source = String::from("gen fn G()->I64{let a0=[1];");
    for index in 1..levels {
        source.push_str(&format!("let a{index}=[a{}];", index - 1));
    }
    source.push_str("yield 1;}fn Main(){let g=G();}");
    source
}

#[test]
fn generator_array_field_preserves_array_priority_and_value_limit() {
    accepted_both(&array_local_state(127));
    let invalid = array_local_state(128);
    let start = invalid.rfind("G()").unwrap() as u64;
    rejected_both(&invalid, "value_type_depth", 129, (start, start + 3));
    let array_over = format!("fn Main(){{{} }}", {
        let mut source = String::from("let a0=[1];");
        for index in 1..=128 {
            source.push_str(&format!("let a{index}=[a{}];", index - 1));
        }
        source
    });
    let start = array_over.rfind("[a127]").unwrap() as u64;
    rejected_both(&array_over, "array_type_depth", 129, (start, start + 6));
}

fn j_depth_error(source: &str, kind: &str, span: (usize, usize)) {
    let (status, report) = j_check(source);
    assert_eq!(status, 1, "{report}");
    for field in [
        "\"code\":\"NX-E0003\"".to_string(),
        "\"stage\":\"types\"".into(),
        "\"operation\":\"analysis.limit\"".into(),
        format!("\"budget_kind\":\"{kind}\""),
        "\"limit\":128".into(),
        "\"observed\":129".into(),
        format!("\"start\":{}", span.0),
        format!("\"end\":{}", span.1),
    ] {
        assert!(report.contains(&field), "{field}: {report}");
    }
    assert_eq!(
        report.matches("\"code\":\"NX-E0003\"").count(),
        1,
        "{report}"
    );
}

fn option_chain(levels: usize, seed: &str) -> String {
    let mut source = format!("fn Main(){{let a0={seed};");
    for index in 1..levels {
        source.push_str(&format!("let a{index}=Some(move a{});", index - 1));
    }
    source.push('}');
    source
}

fn result_chain(levels: usize) -> String {
    let mut source =
        String::from("fn Main(){let a0=if true then{value:Ok(1);}else{value:Err(1);};");
    for index in 1..levels {
        source.push_str(&format!(
            "let a{index}=if true then{{value:Ok(a{});}}else{{value:Err(1);}};",
            index - 1
        ));
    }
    source.push('}');
    source
}

fn depth_error(source: &str, kind: &str, observed: u64, span: (u64, u64)) {
    let errors = check_proposed("depth.n", source, false).unwrap_err();
    assert_eq!(errors.len(), 1, "{errors:?}");
    let error = &errors[0];
    assert_eq!(error.code, "NX-E0003", "{errors:?}");
    assert_eq!(error.stage, Stage::Types);
    assert_eq!(error.operation, Operation::AnalysisLimit);
    assert_eq!(
        error.primary_span.as_ref().map(|at| (at.start, at.end)),
        Some(span)
    );
    assert_eq!(
        error.data.get("budget_kind"),
        Some(&DataValue::String(kind.into()))
    );
    assert_eq!(error.data.get("limit"), Some(&DataValue::UInt(128)));
    assert_eq!(error.data.get("observed"), Some(&DataValue::UInt(observed)));
}

#[test]
fn inferred_result_depth_stops_at_first_exceeding_if() {
    for levels in [127, 128] {
        let source = result_chain(levels);
        assert!(
            check_proposed("depth.n", &source, false).is_ok(),
            "{levels}"
        );
    }
    let source = result_chain(129);
    assert_eq!(source.len(), 7270);
    depth_error(&source, "value_type_depth", 129, (7220, 7268));
}

#[test]
fn generator_state_adds_one_level_to_option_depth() {
    for wrappers in [126, 127] {
        let source = format!(
            "gen fn G()->I64{{yield 1;}}{}",
            option_chain(wrappers + 1, "G()")
        );
        let (status, report) = j_check(&source);
        assert_eq!(status, 0, "{wrappers}: {report}");
    }
    let source = format!("gen fn G()->I64{{yield 1;}}{}", option_chain(129, "G()"));
    assert_eq!(source.len(), 3029);
    j_depth_error(&source, "value_type_depth", (3012, 3027));
}

#[test]
fn owned_closure_and_option_paths_share_the_same_depth_bound() {
    let mut source = String::from("fn Main(){let f0=fn [](){};");
    for index in 1..=127 {
        source.push_str(&format!("let f{index}=fn [move f{}](){{}};", index - 1));
    }
    let (status, report) = j_check(&format!("{source}}}"));
    assert_eq!(status, 0, "{report}");
    let last = "let f128=fn [move f127](){};";
    let invalid = format!("{source}{last}}}");
    let anchor = (source.len() + "let f128=".len()) as u64;
    j_depth_error(
        &invalid,
        "value_type_depth",
        (
            anchor as usize,
            anchor as usize + "fn [move f127](){}".len(),
        ),
    );
}

#[test]
fn option_and_array_constructors_stop_at_their_own_depth_bound() {
    let mut option = String::from("fn Main(){let a0=Some(1);");
    let mut array = String::from("fn Main(){let a0=[1];");
    for index in 1..=127 {
        option.push_str(&format!("let a{index}=Some(a{});", index - 1));
        array.push_str(&format!("let a{index}=[a{}];", index - 1));
        if index == 126 || index == 127 {
            for source in [&option, &array] {
                let (status, report) = j_check(&format!("{source}}}"));
                assert_eq!(status, 0, "{report}");
            }
        }
    }
    let next_option = "let a128=Some(a127);";
    let start = option.len() + "let a128=".len();
    j_depth_error(
        &format!("{option}{next_option}}}"),
        "value_type_depth",
        (start, start + "Some(a127)".len()),
    );
    let next_array = "let a128=[a127];";
    let start = array.len() + "let a128=".len();
    j_depth_error(
        &format!("{array}{next_array}}}"),
        "array_type_depth",
        (start, start + "[a127]".len()),
    );
}

#[test]
fn viewed_closure_capture_does_not_extend_owned_depth() {
    let mut source = String::from("fn Main(){let f0=fn [](){};");
    for index in 1..=129 {
        source.push_str(&format!("let f{index}=fn [view f{}](){{}};", index - 1));
    }
    let (status, report) = j_check(&format!("{source}}}"));
    assert_eq!(status, 0, "{report}");
}

#[test]
fn option_depth_reports_once_at_the_first_exceeding_constructor() {
    let source = format!("fn Main(){{{}}}", option_bindings(128));
    let (status, report) = j_check(&source);
    assert_eq!(status, 0, "128: {report}");
    for count in [129, 130, 300] {
        let source = format!("fn Main(){{{}}}", option_bindings(count));
        let start = source.find("Some(a128)").unwrap() as u64;
        rejected_both(&source, "value_type_depth", 129, (start, start + 10));
    }
}

#[test]
fn deeply_nested_owned_resource_reports_one_depth_error() {
    let source = option_chain(1563, "MockOpen(1)");
    let start = source.find("Some(move a128)").unwrap() as u64;
    rejected_both(&source, "value_type_depth", 129, (start, start + 15));
}

#[test]
fn closure_body_locals_do_not_extend_generator_state() {
    let source = format!(
        "gen fn G()->I64{{let f=fn []()->I64{{{}return 1;}};yield 1;}}fn Main(){{let g=Some(G());}}",
        option_bindings(128)
    );
    accepted_both(&source);
}
