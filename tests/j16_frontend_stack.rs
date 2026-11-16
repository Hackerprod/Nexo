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
        "block129" => Some((137, 138)),
        "type129" => Some((780, 783)),
        "pattern129" => Some((145, 146)),
        _ => None,
    };
    if let Some((start, end)) = expected_span {
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
            Some(&nexo::diagnostic::DataValue::String(
                "syntax_nesting".into()
            ))
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
