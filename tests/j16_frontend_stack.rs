use std::process::Command;

fn source(family: &str) -> String {
    match family {
        "block" => format!(
            "fn Main(){{{}let x=1;{}}}",
            "{".repeat(126),
            "}".repeat(126)
        ),
        "group" => format!(
            "fn Main(){{let x={}1{};}}",
            "(".repeat(126),
            ")".repeat(126)
        ),
        "array" => format!(
            "fn Main(){{let x={}1{};}}",
            "[".repeat(126),
            "]".repeat(126)
        ),
        "index" => format!("fn Main(){{let a=[1];let x=a{};}}", "[0]".repeat(126)),
        "binary_left" => format!("fn Main(){{let x=1{};}}", "+1".repeat(126)),
        "binary_right" => format!(
            "fn Main(){{let x={}1{};}}",
            "1+(".repeat(126),
            ")".repeat(126)
        ),
        "call_arg" => format!(
            "fn F(x:view I64)->I64{{return x;}}fn Main(){{let x={}1{};}}",
            "F(".repeat(126),
            ")".repeat(126)
        ),
        "mixed" => format!(
            "fn Main(){{{}let x={}1{};{}}}",
            "{".repeat(63),
            "(".repeat(63),
            ")".repeat(63),
            "}".repeat(63)
        ),
        "yield_group" => format!(
            "gen fn G()->I64{{yield {}1{};}}fn Main(){{}}",
            "(".repeat(126),
            ")".repeat(126)
        ),
        "else_if" => format!("fn Main(){{{}{}}}", "if true{}else ".repeat(126), "{}"),
        "alternating_match_guard" => {
            let prelude = "fn F(p:view I64)->I64{return p;}fn G(p:edit I64)->I64{return 1;}fn Id(p:move I64)->I64{return p;}";
            let mut openings = String::new();
            let mut closings = Vec::new();
            for index in 0..126 {
                if index % 2 == 0 {
                    openings.push_str("if false{}else if true{");
                    closings.push("}");
                } else {
                    openings.push_str("match 1{_ if true=>{");
                    closings.push("},_=>{}}");
                }
            }
            format!(
                "{prelude}fn Main(){{var a=[1];let x=1;{openings}let z=1+1;{}}}",
                closings.into_iter().rev().collect::<String>()
            )
        }
        "if_expr" => {
            let mut expr = String::from("1");
            for _ in 0..126 {
                expr = format!("if true then ({expr}) else 0");
            }
            format!("fn Main(){{let x={expr};}}")
        }
        "if_stmt" => format!("fn Main(){{{}{}}}", "if true{".repeat(126), "}".repeat(126)),
        "while" => format!(
            "fn Main(){{{}{}}}",
            "while true{".repeat(126),
            "}".repeat(126)
        ),
        "closure" => format!(
            "fn Main(){{let f={}1{};}}",
            "fn(){let g=".repeat(63),
            ";}".repeat(63)
        ),
        "closure_called" => {
            let mut expr = String::from("1");
            for _ in 0..40 {
                expr = format!("(fn ()->I64{{return {expr};}})()");
            }
            format!("fn Main(){{let x={expr};}}")
        }
        "match_scrutinee" => {
            let mut expr = String::from("1");
            for _ in 0..60 {
                expr = format!("match ({expr}){{_=>{{value:1;}}}}");
            }
            format!("fn Main(){{let x={expr};}}")
        }
        "match_expr" => {
            let mut expr = String::from("1");
            for _ in 0..63 {
                expr = format!("match 1{{_=>{{value:{expr};}}}}");
            }
            format!("fn Main(){{let x={expr};}}")
        }
        "match_stmt" => {
            let mut body = String::new();
            for _ in 0..60 {
                body = format!("match 1{{_=>{{{body}}}}}");
            }
            format!("fn Main(){{{body}}}")
        }
        "type" => format!(
            "fn F(x:view {}I64{}){{}}fn Main(){{}}",
            "Array<".repeat(126),
            ">".repeat(126)
        ),
        "pattern" => [
            "fn Main(){match 1{".to_string(),
            "(".repeat(126),
            "_".into(),
            ")".repeat(126),
            "=>{},}}".into(),
        ]
        .concat(),
        "block129" => format!(
            "fn Main(){{{}let x=1;{}}}",
            "{".repeat(128),
            "}".repeat(128)
        ),
        "type129" => format!(
            "fn F(x:view {}I64{}){{}}fn Main(){{}}",
            "Array<".repeat(128),
            ">".repeat(128)
        ),
        "pattern129" => [
            "fn Main(){match 1{".to_string(),
            "(".repeat(128),
            "_".into(),
            ")".repeat(128),
            "=>{},}}".into(),
        ]
        .concat(),
        "edit_simple" => "fn F(x:edit I64){}fn Main(){var a=1;F(edit a);}".into(),
        "edit_nested_40" | "edit_nested_42" => {
            let depth = if family == "edit_nested_40" { 40 } else { 42 };
            format!(
                "fn F(x:edit I64)->I64{{return 1;}}fn Main(){{var a=[1];let y={}0{};}}",
                "F(edit a[".repeat(depth),
                "])".repeat(depth)
            )
        }
        "move_call_63" | "edit_call_63" => {
            let mode = if family == "move_call_63" {
                "move"
            } else {
                "edit"
            };
            format!(
                "fn Main(){{let f=fn(p:move I64){{}};let y={}1{};}}",
                format!("{mode} f(").repeat(63),
                ")".repeat(63)
            )
        }
        "move_arg_42" => format!(
            "fn F(x:move I64){{}}fn Main(){{let f=fn(p:move I64){{}};F({}1{});}}",
            "move f(".repeat(42),
            ")".repeat(42)
        ),
        "objectives" => std::fs::read_to_string(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("corpus/objetivo")
                .join(std::env::var("J16_OBJECTIVE_NAME").unwrap()),
        )
        .unwrap(),
        _ => unreachable!("unknown parser family"),
    }
}

#[test]
fn parser_child() {
    let Ok(family) = std::env::var("J16_PARSER_CASE") else {
        return;
    };
    let source = source(&family);
    let result = nexo::ir::compile_scalar_with_profile(
        "parser-stack.n",
        &source,
        nexo::ir::ReducedProfile::from_id("f0-0003-b-v1").unwrap(),
        true,
    );
    let expected_span = match family.as_str() {
        "block129" => Some(((137, 138), "syntax_nesting")),
        "type129" => Some(((780, 783), "syntax_nesting")),
        "pattern129" => Some(((145, 146), "syntax_nesting")),
        "alternating_match_guard" => Some(((125, 3411), "expression_tree_depth")),
        _ => None,
    };
    if let Some(((start, end), kind)) = expected_span {
        let error = match result {
            Err(nexo::ir::CompileError::Rejected(mut diagnostics)) => {
                assert_eq!(diagnostics.len(), 1);
                diagnostics.pop().unwrap()
            }
            _ => panic!("{family}: expected one parse diagnostic"),
        };
        assert_eq!(error.code, "NX-E0003");
        assert_eq!(
            error
                .primary_span
                .as_ref()
                .map(|span| (span.start, span.end)),
            Some((start, end))
        );
        assert_eq!(
            error.data.get("budget_kind"),
            Some(&nexo::diagnostic::DataValue::String(kind.into()))
        );
        assert_eq!(
            error.data.get("limit"),
            Some(&nexo::diagnostic::DataValue::UInt(128))
        );
        assert_eq!(
            error.data.get("observed"),
            Some(&nexo::diagnostic::DataValue::UInt(129))
        );
    } else if let Err(error) = result {
        match error {
            nexo::ir::CompileError::Rejected(diagnostics) => {
                assert!(
                    diagnostics
                        .iter()
                        .all(|diagnostic| diagnostic.stage != nexo::diagnostic::Stage::Parse),
                    "{family}: {diagnostics:?}"
                );
            }
            _ => panic!("{family}: compile failed outside a source diagnostic"),
        }
    }
}

#[test]
fn alternating_if_match_guard_rejection_releases_partial_ast_in_256_kib_worker() {
    let binary = std::env::current_exe().unwrap();
    for repeat in 0..10 {
        let output = Command::new(&binary)
            .args(["--exact", "parser_child"])
            .env("J16_PARSER_CASE", "alternating_match_guard")
            .env("NEXO_STACK_MINIMUM_CASE", "PARSER:alternating_match_guard")
            .env("NEXO_PROBE_PARSER_KIB", "256")
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "repeat {repeat}: {} {}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }
}

#[test]
fn parser_families_finish_in_256_kib_worker() {
    let binary = std::env::current_exe().unwrap();
    for family in [
        "block",
        "group",
        "array",
        "index",
        "binary_left",
        "binary_right",
        "call_arg",
        "mixed",
        "yield_group",
        "else_if",
        "if_expr",
        "if_stmt",
        "while",
        "closure",
        "closure_called",
        "match_scrutinee",
        "match_expr",
        "match_stmt",
        "type",
        "pattern",
        "block129",
        "type129",
        "pattern129",
        "edit_simple",
        "edit_nested_40",
        "edit_nested_42",
        "move_call_63",
        "edit_call_63",
        "move_arg_42",
    ] {
        let output = Command::new(&binary)
            .args(["--exact", "parser_child"])
            .env("J16_PARSER_CASE", family)
            .env("NEXO_STACK_MINIMUM_CASE", format!("PARSER:{family}"))
            .env("NEXO_PROBE_PARSER_KIB", "256")
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{family}: {} {}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }
}

#[test]
fn mode_operands_and_objectives_finish_in_256_kib_parser_workers() {
    let binary = std::env::current_exe().unwrap();
    let objectives = [
        "01_integer_limits.n",
        "02_nested_copy.n",
        "03_resource_transfer.n",
        "04_return_cleanup.n",
        "05_shadow_scope.n",
        "06_result_ok.n",
        "07_result_error.n",
        "08_array_update.n",
        "09_mock_request.n",
        "10_mock_write.n",
        "11_ex04_generador.n",
        "12_option_match.n",
        "13_option_none.n",
        "14_closure_capture.n",
        "15_generator_sum.n",
    ];
    for kib in [256] {
        for (family, objective) in [
            "edit_simple",
            "edit_nested_40",
            "edit_nested_42",
            "move_call_63",
            "edit_call_63",
            "move_arg_42",
        ]
        .into_iter()
        .map(|family| (family, None))
        .chain(
            objectives
                .into_iter()
                .map(|name| ("objectives", Some(name))),
        ) {
            let mut command = Command::new(&binary);
            command
                .args(["--exact", "parser_child"])
                .env("J16_PARSER_CASE", family)
                .env("NEXO_STACK_MINIMUM_CASE", format!("PARSER:{family}"))
                .env("NEXO_PROBE_PARSER_KIB", kib.to_string());
            if let Some(name) = objective {
                command.env("J16_OBJECTIVE_NAME", name);
            }
            let output = command.output().unwrap();
            assert!(
                output.status.success(),
                "{family}/{objective:?}/{kib}: {} {}",
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr)
            );
        }
    }
}

fn deep_match_type_source(case: &str) -> String {
    let (ty, pattern) = match case {
        "option_wildcard" => (
            format!("{}I64{}", "Option<".repeat(126), ">".repeat(126)),
            "_".to_owned(),
        ),
        "result_wildcard" => (
            format!("{}I64{}", "Result<".repeat(126), ",I64>".repeat(126)),
            "_".to_owned(),
        ),
        "option_constructor" => (
            format!("{}I64{}", "Option<".repeat(126), ">".repeat(126)),
            format!("{}_{})", "Some(".repeat(126), ")".repeat(125)),
        ),
        _ => unreachable!("unknown deep match case"),
    };
    let arms = if case == "option_constructor" {
        format!("{pattern}=>{{}},_=>{{}}")
    } else {
        format!("{pattern}=>{{}}")
    };
    format!("fn F(x:view {ty}){{match x{{{arms}}}}}fn Main(){{}}")
}

#[test]
fn deep_match_type_stack_child() {
    let Ok(case) = std::env::var("J16_DEEP_MATCH_CASE") else {
        return;
    };
    let source = deep_match_type_source(&case);
    nexo::ir::compile_scalar_with_profile(
        "deep-match-stack.n",
        &source,
        nexo::ir::ReducedProfile::from_id("f0-0003-b-v1").unwrap(),
        true,
    )
    .unwrap();
}

#[test]
fn deep_option_and_result_matches_fit_checker_stack() {
    let binary = std::env::current_exe().unwrap();
    for case in ["option_wildcard", "result_wildcard", "option_constructor"] {
        let output = Command::new(&binary)
            .args(["--exact", "deep_match_type_stack_child"])
            .env("J16_DEEP_MATCH_CASE", case)
            .env("NEXO_STACK_MINIMUM_CASE", format!("CHECKER:{case}"))
            .env("NEXO_PROBE_CHECKER_KIB", "256")
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{case}: {} {}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }
}
