use nexo::check::check_proposed;
use nexo::diagnostic::{DataValue, Operation, Stage};
use std::{
    fs,
    process::Command,
    sync::atomic::{AtomicUsize, Ordering},
};

static SOURCE_ID: AtomicUsize = AtomicUsize::new(0);

fn j_check(source: &str) -> (i32, String) {
    let id = SOURCE_ID.fetch_add(1, Ordering::Relaxed);
    let path = std::env::temp_dir().join(format!("j14-s4-depth-{}-{id}.n", std::process::id()));
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
    (
        output.status.code().unwrap(),
        String::from_utf8(output.stdout).unwrap(),
    )
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
    assert_eq!(error.code, "NX-E0003");
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
