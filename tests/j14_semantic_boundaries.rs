use std::{
    fs,
    process::Command,
    sync::atomic::{AtomicUsize, Ordering},
};

static SOURCE_ID: AtomicUsize = AtomicUsize::new(0);

fn cli(command: &str, source: &str) -> (i32, String) {
    let id = SOURCE_ID.fetch_add(1, Ordering::Relaxed);
    let path =
        std::env::temp_dir().join(format!("nexo-j14-semantic-{}-{id}.n", std::process::id()));
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
    fs::remove_file(&path).unwrap();
    (
        output.status.code().expect("Nexo must not crash"),
        String::from_utf8(output.stdout).unwrap(),
    )
}

fn checked(source: &str) {
    for command in ["check", "run"] {
        let (status, report) = cli(command, source);
        assert_eq!(status, 0, "{command}: {source}: {report}");
        assert!(report.contains("\"diagnostics\":[]"), "{source}: {report}");
    }
}

fn rejected(source: &str, code: &str) -> String {
    let mut reports = Vec::new();
    for command in ["check", "run"] {
        let (status, report) = cli(command, source);
        assert_eq!(status, 1, "{command}: {source}: {report}");
        assert_eq!(
            report.matches(&format!("\"code\":\"{code}\"")).count(),
            1,
            "{source}: {report}"
        );
        assert!(
            !report.contains("internal_error") && !report.contains("\"diagnostics\":[null"),
            "{source}: {report}"
        );
        reports.push(report);
    }
    reports.remove(0)
}

#[test]
fn nested_match_restores_enclosing_temporaries_on_impossible_edges() {
    let inner = "match true{true=>{value:true;},false=>{value:false;}}";
    let outer = format!("match ({inner}){{true=>{{value:1;}},false=>{{value:2;}}}}");
    for body in [
        format!("let x=1+({outer});"),
        format!("let x=({outer})+1;"),
        format!("let x=1+({outer})+2;"),
        format!("let x=if true then{{value:1+({outer});}}else{{value:0;}};"),
    ] {
        checked(&format!("fn Main(){{{body}}}"));
    }
}

#[test]
fn transitive_suspend_safe_diagnostic_is_canonically_ordered() {
    for wrapper in ["Some(move f)", "Some(Some(move f))", "fn [move f](){}"] {
        let source = format!(
            "gen fn G()->I64{{var x=0;var f=fn [edit x](){{x=1;}};let o={wrapper};yield 1;}}fn Main(){{}}"
        );
        let report = rejected(&source, "NX-E1204");
        assert!(report.contains("\"related_spans\":["), "{report}");
    }
    checked("gen fn G()->I64{var x=0;let o=Some(x);yield 1;}fn Main(){}");
}

#[test]
fn unresolved_scrutinee_precedes_guard_body_constraints() {
    rejected(
        "fn Main(){match if true then{value:None;}else{value:Some(Err(1));}{None if true=>{let v:Option<Result<I64,I64>>=None;},Some(Ok(p))=>{},_=>{}}}",
        "NX-E0113",
    );
    checked(
        "fn Main(){let o:Option<Result<I64,I64>>=Some(Err(1));match o{None if true=>{},Some(Ok(p))=>{},_=>{}}}",
    );
}

#[test]
fn value_depth_counts_inferred_option_and_owned_closure_captures() {
    let mut options = String::from("fn Main(){let a0=Some(1);");
    for index in 1..=127 {
        options.push_str(&format!("let a{index}=Some(a{});", index - 1));
    }
    checked(&format!("{options}}}"));
    let report = rejected(&format!("{options}let a128=Some(a127);}}"), "NX-E0003");
    assert!(
        report.contains("\"budget_kind\":\"value_type_depth\"")
            && report.contains("\"observed\":129"),
        "{report}"
    );

    let mut closures = String::from("fn Main(){let f0=fn [](){};");
    for index in 1..=127 {
        closures.push_str(&format!("let f{index}=fn [move f{}](){{}};", index - 1));
    }
    checked(&format!("{closures}}}"));
    rejected(
        &format!("{closures}let f128=fn [move f127](){{}};}}"),
        "NX-E0003",
    );
}

#[test]
fn closure_literals_count_as_invocables_at_their_fn_token() {
    let mut source = String::from("fn Main(){");
    for index in 0..1023 {
        source.push_str(&format!("let f{index}=fn [](){{}};"));
    }
    checked(&format!("{source}}}"));
    let report = rejected(&format!("{source}let f1023=fn [](){{}};}}"), "NX-E0003");
    let literal_start = source.len() + "let f1023=".len();
    assert!(
        report.contains("\"budget_kind\":\"function_count\"")
            && report.contains("\"limit\":1024")
            && report.contains("\"observed\":1025")
            && report.contains(&format!("\"start\":{literal_start}"))
            && report.contains(&format!("\"end\":{}", literal_start + 2)),
        "{report}"
    );
}

#[test]
fn declarations_and_nested_literals_share_source_order_quota() {
    let mut source = String::from("gen fn G()->I64{yield 1;}");
    for index in 0..1021 {
        source.push_str(&format!("fn F{index}(){{}}"));
    }
    source.push_str("fn Main(){let outer=fn [](){let inner=fn [](){};};}");
    let inner = source.rfind("fn [](){}").unwrap();
    let report = rejected(&source, "NX-E0003");
    assert!(
        report.contains("\"budget_kind\":\"function_count\"")
            && report.contains("\"observed\":1025")
            && report.contains(&format!("\"start\":{inner}"))
            && report.contains(&format!("\"end\":{}", inner + 2)),
        "{report}"
    );
    checked("fn Main(){var i=0;while i<2{let f=fn [](){};i=i+1;}}");
}

#[test]
fn completed_features_use_syntax_errors_and_official_delimiters() {
    for source in [
        "match",
        "fn Main(){}for",
        "continue Main(){}",
        "fn Main(){let x=gen;}",
        "fn Main(){let x=yield;}",
        "fn Main(){var o:Option<I64>=break(1);}",
        "fn Main(){let f=fn [](x:view I64,){};}",
        "fn Main(){let x=1;let f=fn [copy x,](){};}",
        "fn Main(){let o:Option<I64>=Some(1);match o{Some(n,)=>{},None=>{}}}",
        "gen fn G(){yield;}fn Main(){}",
    ] {
        rejected(source, "NX-E0002");
    }
    checked("fn Main(){match 1{(1)=>{},_=>{}}}");
    rejected("fn Main(){match 1{(1,2)=>{},_=>{}}}", "NX-E0006");
    checked(
        "gen fn G()->I64{yield 1;}fn Main(){let f=fn [](x:view I64){};let x=1;let g=fn [copy x](){};}",
    );
}

#[test]
fn retired_introducers_have_syntax_coordinates_in_each_parser_state() {
    for source in [
        "match",
        "fn Main(){}for",
        "continue Main(){}",
        "fn Main(){let x=gen;}",
        "fn Main(){let x=yield;}",
        "fn Main(){var o:Option<I64>=break(1);}",
        "fn Main(){let x=1;\r\nlet y=gen;}",
    ] {
        let report = rejected(source, "NX-E0002");
        assert!(
            report.contains("\"stage\":\"parse\"")
                && report.contains("\"operation\":\"syntax.parse\"")
                && report.contains("\"feature_id\":null")
                && report.contains("\"expected_tokens_kind\":\"admissible_set\"")
                && !report.contains("\"pending_scope\":\"J"),
            "{source}: {report}"
        );
    }
    for (source, feature) in [
        (
            "gen fn G()->I64{yield 1;}fn Main(){for edit x in G(){}}",
            "for_iteration",
        ),
        ("fn Main(){let f=fn [](){};let a=[f];}", "closure"),
    ] {
        let report = rejected(source, "NX-E0006");
        assert!(
            report.contains(&format!("\"feature_id\":\"{feature}\""))
                && report.contains("\"pending_scope\":\"later_adr\""),
            "{report}"
        );
    }
}

#[test]
fn grouped_patterns_charge_shared_syntax_nesting() {
    fn grouped_source(groups: usize) -> String {
        format!(
            "fn Main(){{match 1{{{}1{}=>{{}},_=>{{}}}}}}",
            "(".repeat(groups),
            ")".repeat(groups)
        )
    }
    // Main's Block is one level; each group and the final pattern add one.
    checked(&grouped_source(125)); // 127 total
    checked(&grouped_source(126)); // 128 total
    for groups in [127, 128, 200, 2_000] {
        let source = grouped_source(groups);
        let report = rejected(&source, "NX-E0003");
        let opening = "fn Main(){match 1{".len() + 127;
        assert!(
            report.contains("\"budget_kind\":\"syntax_nesting\"")
                && report.contains("\"limit\":128")
                && report.contains("\"observed\":129")
                && report.contains(&format!("\"start\":{opening}")),
            "{groups}: {report}"
        );
    }
    checked("fn Main(){let o:Option<I64>=Some(1);match o{((Some(x)))=>{},None=>{}}}");
    let tuple = rejected("fn Main(){match 1{(a,b)=>{},_=>{}}}", "NX-E0006");
    assert!(tuple.contains("\"feature_id\":\"tuple\""), "{tuple}");
    rejected("fn Main(){match 1{()=>{},_=>{}}}", "NX-E0002");
}

#[test]
fn match_expression_unifies_deferred_option_arms() {
    checked("fn Main(){let x=match true{true=>{value:Some(1);},false=>{value:None;}};}");
    rejected(
        "fn Main(){let x=match true{true=>{value:Some(1);},false=>{value:Some(true);}};}",
        "NX-E1303",
    );
}

#[test]
fn small_runtime_stack_handles_nested_values_closures_and_generators() {
    let mut values = String::from("fn Main(){let a0=Some(MockOpen(1));");
    for index in 1..=104 {
        values.push_str(&format!("let a{index}=Some(move a{});", index - 1));
    }
    checked(&format!("{values}}}"));

    let mut closures = String::from("fn Main(){let f1=fn []()->I64{return 1;};");
    for index in 2..=241 {
        closures.push_str(&format!(
            "let f{index}=fn [view f{}]()->I64{{return f{}();}};",
            index - 1,
            index - 1
        ));
    }
    checked(&format!("{closures}AssertEq(f241(),1);}}"));

    let mut generators = String::new();
    for index in 1..=225 {
        generators.push_str(&format!(
            "gen fn G{index}()->I64{{var g=G{}();let a=Iterator.Next(edit g);yield 1;}}",
            index + 1
        ));
    }
    generators
        .push_str("gen fn G226()->I64{yield 1;}fn Main(){var g=G1();let a=Iterator.Next(edit g);}");
    checked(&generators);
}
