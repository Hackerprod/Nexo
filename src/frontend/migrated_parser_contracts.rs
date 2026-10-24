//! Parser contracts carried from the pre-B surface with their B sources.
use super::parse::{Expr, Statement, parse};
use super::print::print_canonical;
use super::test_normalize::ast_without_spans;
use crate::diagnostic::{DataValue, ExpectedTokensKind, Operation, RelatedRole, Stage, TokenKind};

fn unique_bytes(source: &str, needle: &str) -> (u64, u64) {
    let at = source.find(needle).expect("fixture marker");
    assert!(
        source[at + needle.len()..].find(needle).is_none(),
        "ambiguous fixture marker"
    );
    (at as u64, (at + needle.len()) as u64)
}

#[test]
fn b_branch_marker_and_scope_repairs_keep_current_boundaries() {
    let source = "fn Main(){let x=if true then {value:1} else {value:2;};}";
    let d = parse("gap.nx", source).unwrap_err();
    assert_eq!(d.code, "NX-E0005");
    d.validate("gap.nx", source).unwrap();
    parse("gap.nx", "fn Main(){while false{}}").unwrap();
    let source = "fn Main(){while false{;}}";
    let d = parse("gap.nx", source).unwrap_err();
    assert_eq!(d.code, "NX-E0002");
    d.validate("gap.nx", source).unwrap();
    parse("gap.nx", "  \n fn Main(){ 1; }").unwrap();
}

#[test]
fn b_incomplete_control_and_postfix_contexts_keep_current_result() {
    for source in [
        "fn F(){while",
        "fn F(){if",
        "fn F(){if a{}else if",
        "fn F(){a[",
        "fn F(){F(a:",
    ] {
        let d = parse("gap.nx", source).unwrap_err();
        assert_eq!(d.code, "NX-E0002", "{source}");
        assert_eq!(
            d.parser_context.as_deref(),
            Some("grammar.Primary"),
            "{source}"
        );
        d.validate("gap.nx", source).unwrap();
    }
    // D15 §2.2.3 makes the bare contextual names ordinary Name expressions.
    for source in ["fn F(){let x=move", "fn F(){let x=edit"] {
        let d = parse("gap.nx", source).unwrap_err();
        assert_eq!(d.code, "NX-E0005", "{source}");
        assert_eq!(
            d.parser_context.as_deref(),
            Some("statement_terminator"),
            "{source}"
        );
        d.validate("gap.nx", source).unwrap();
    }
}

#[test]
fn b_nonterminating_subexpression_follow_keeps_current_result() {
    for (source, context, expected) in [
        ("fn F(){if a<1", "grammar.Block", &["{"][..]),
        ("fn F(){if a<1 ;", "grammar.Block", &["{"][..]),
        ("fn F(){if a<1 )", "grammar.Block", &["{"][..]),
        ("fn F(){if a<1 <", "grammar.CompareExpr", &[";"][..]),
        ("fn F(){if a<1 ==", "grammar.CompareExpr", &[";"][..]),
    ] {
        let d = parse("gap.nx", source).unwrap_err();
        assert_eq!(d.code, "NX-E0002", "{source}");
        assert_eq!(d.parser_context.as_deref(), Some(context), "{source}");
        assert_eq!(d.expected_tokens, expected, "{source}");
        d.validate("gap.nx", source).unwrap();
    }
}

#[test]
fn b_nullable_continuations_and_else_if_depth_keep_current_results() {
    parse("gap.nx", "fn Main(){F(x\n, 1);}").unwrap();
    for source in ["fn Main(){x\n, 1;}", "fn Main(){x\nelse{}}"] {
        assert_eq!(parse("gap.nx", source).unwrap_err().code, "NX-E0002");
    }
    let source = format!("fn Main(){{{}{{}}}}", "if true{}else ".repeat(128));
    let d = parse("gap.nx", &source).unwrap_err();
    assert_eq!(
        d.data.get("budget_kind"),
        Some(&DataValue::String("expression_tree_depth".into()))
    );
    d.validate("gap.nx", &source).unwrap();
}

#[test]
fn b_signature_suffix_has_exact_expected_tokens() {
    let source = "fn Main() ;";
    let d = parse("fixture", source).unwrap_err();
    assert_eq!(d.code, "NX-E0002");
    assert_eq!(d.parser_context.as_deref(), Some("function_suffix"));
    assert_eq!(d.expected_tokens, ["->", "with", "{"]);
    let point = unique_bytes(source, ";");
    assert_eq!(
        d.primary_span.as_ref().map(|s| (s.start, s.end)),
        Some(point)
    );
    assert_eq!(
        d.recognized_prefix.as_ref().map(|s| (s.start, s.end)),
        Some((0, point.0))
    );
    d.validate("fixture", source).unwrap();
}

#[test]
fn b_syntax_nesting_limit_has_published_fields() {
    let source = format!(
        "fn Main(){{let x={}[1{};}}",
        "[".repeat(10_000),
        "]".repeat(10_000)
    );
    let d = parse("deep.nx", &source).unwrap_err();
    assert_eq!(d.code, "NX-E0003");
    assert_eq!(d.stage, Stage::Parse);
    assert_eq!(d.operation, Operation::AnalysisLimit);
    assert_eq!(
        d.primary_span.as_ref().map(|s| (s.start, s.end)),
        Some((143, 144))
    );
    assert_eq!(
        d.data.get("budget_kind"),
        Some(&DataValue::String("syntax_nesting".into()))
    );
    assert_eq!(d.data.get("limit"), Some(&DataValue::UInt(128)));
    assert_eq!(d.data.get("observed"), Some(&DataValue::UInt(129)));
    d.validate("deep.nx", &source).unwrap();
}

#[test]
fn b_multiple_signatures_written_types_and_nested_expressions() {
    let source = "fn value(x:view I64)->I64{return x;} fn Main(){let value=2;let a:Array<Result<I64,Bool>>=[value - 1];let x=if true then {value:a[0];}else{value:[];};}";
    let ast = parse("x", source).unwrap();
    assert_eq!(ast.functions.len(), 2);
    assert_eq!(ast.functions[0].name, "value");
    assert_eq!(ast.functions[0].params[0].ty, super::parse::Type::I64);
    assert!(matches!(
        ast.functions[1].body.statements[2],
        Statement::Let { .. }
    ));
}

#[test]
fn b_terminator_repair_uses_expression_end_and_exact_inserted_text() {
    for (source, terminal) in [
        ("fn Main(){let x=1}", "1"),
        ("//é\nfn Main(){let x=1}", "1"),
        ("fn Main()->I64{return 1}", "1"),
        ("fn Main(){F()}", ")"),
    ] {
        let d = parse("repair.nx", source).unwrap_err();
        assert_eq!(d.code, "NX-E0005", "{source}");
        assert_eq!(d.parser_context.as_deref(), Some("statement_terminator"));
        assert_eq!(d.expected_tokens, [";"]);
        assert_eq!(
            d.expected_tokens_kind,
            Some(ExpectedTokensKind::CanonicalInsertion)
        );
        let edit = &d.repair[0].edits[0];
        let end = source.rfind(terminal).unwrap() as u64 + terminal.len() as u64;
        assert_eq!(edit.replacement, ";");
        assert_eq!(edit.expected_text, "");
        assert_eq!((edit.span.start, edit.span.end), (end, end), "{source}");
        d.validate("repair.nx", source).unwrap();
    }
}

#[test]
fn b_nested_if_conditions_finish_on_a_small_debug_stack() {
    std::thread::Builder::new()
        .stack_size(2 * 1024 * 1024)
        .spawn(|| {
            let source = format!(
                "fn F(){{let x={}true{};}}",
                "if ".repeat(128),
                " then {} else 1".repeat(128)
            );
            let d = parse("stack.nx", &source).unwrap_err();
            assert_eq!(d.code, "NX-E0003");
            assert_eq!(
                d.data.get("budget_kind"),
                Some(&DataValue::String("syntax_nesting".into()))
            );
            d.validate("stack.nx", &source).unwrap();
        })
        .unwrap()
        .join()
        .unwrap();
}

#[test]
fn b_feature_recognition_follows_position_and_function_scope() {
    for source in [
        "fn F(from:view I64)->I64{return from;}",
        "fn Main(){let Option=1;}",
        "fn Main(){let a=[1];let x=a[MockOpen(1)];}",
    ] {
        assert!(parse("scope.nx", source).is_ok(), "{source}");
    }
    parse("scope.nx", "fn Main(){} fn G()->I64{return 1;}").unwrap();
    let source = "fn Main(){} fn G(){return 1;}";
    let d = parse("scope.nx", source).unwrap_err();
    assert_eq!(d.feature_id.as_deref(), Some("inferred_non_unit_result"));
    assert_eq!(
        d.primary_span.as_ref().map(|s| (s.start, s.end)),
        Some(unique_bytes(source, "G"))
    );
    assert_eq!(
        d.related_spans[0].span.start,
        source.find("return").unwrap() as u64
    );
    d.validate("scope.nx", source).unwrap();
    let source = "fn Main(){let x=;} fn G(){for}";
    let d = parse("scope.nx", source).unwrap_err();
    assert_eq!(d.code, "NX-E0002");
    assert_eq!(
        d.unexpected_token.as_ref().and_then(|t| t.text.as_deref()),
        Some(";")
    );
    d.validate("scope.nx", source).unwrap();
}

#[test]
fn b_prefix_place_reports_first_non_place_token() {
    for (source, token) in [
        ("fn Main(){let x=move (a+1);}", "+"),
        ("fn Main(){let x=move (a?);}", "?"),
        ("fn Main(){let x=move (a<b);}", "<"),
        ("fn Main(){let x=edit (a+b)[0];}", "+"),
        ("fn Main(){let x=move (1+2);}", "1"),
    ] {
        let d = parse("place.nx", source).unwrap_err();
        assert_eq!(d.code, "NX-E0002", "{source}");
        assert_eq!(
            d.unexpected_token.as_ref().unwrap().text.as_deref(),
            Some(token)
        );
        assert_eq!(
            d.primary_span.as_ref().unwrap().start,
            source.find(token).unwrap() as u64
        );
        assert_eq!(
            d.recognized_prefix.as_ref().unwrap().end,
            d.primary_span.as_ref().unwrap().start
        );
        d.validate("place.nx", source).unwrap();
    }
}

#[test]
fn b_excluded_binary_operator_is_not_repaired_as_terminator() {
    for op in ["*", "!=", ">", "<=", ">="] {
        let source = format!("fn Main(){{let x=1{op}2;}}");
        let d = parse("fixture", &source).unwrap_err();
        assert_eq!(d.code, "NX-E0002");
        assert_eq!(
            d.parser_context.as_deref(),
            Some("unsupported_binary_operator")
        );
        assert_eq!(d.expected_tokens, ["+", "-", "<", "=="]);
        assert_eq!(
            d.primary_span.as_ref().map(|s| (s.start, s.end)),
            Some(unique_bytes(&source, op))
        );
        d.validate("fixture", &source).unwrap();
    }
}

#[test]
fn b_excluded_operators_keep_diagnostic_across_line_breaks() {
    for (operator, repaired) in [
        ("*", "+"),
        ("!=", "=="),
        (">", "<"),
        ("<=", "<"),
        (">=", "<"),
    ] {
        for newline in ["", "\n", "\r\n"] {
            let source = format!("fn Main(){{let x=1{newline}{operator}2;}}");
            let start = source.find(operator).unwrap() as u64;
            let d = parse("operator.nx", &source).unwrap_err();
            assert_eq!(d.code, "NX-E0002", "{source}");
            assert_eq!(d.stage, Stage::Parse, "{source}");
            assert_eq!(
                d.parser_context.as_deref(),
                Some("unsupported_binary_operator"),
                "{source}"
            );
            assert_eq!(d.expected_tokens, ["+", "-", "<", "=="], "{source}");
            assert_eq!(
                d.primary_span.as_ref().map(|s| (s.start, s.end)),
                Some((start, start + operator.len() as u64)),
                "{source}"
            );
            assert_eq!(
                d.unexpected_token.as_ref().and_then(|t| t.text.as_deref()),
                Some(operator)
            );
            assert_eq!(
                d.recognized_prefix.as_ref().map(|s| (s.start, s.end)),
                Some((10, start))
            );
            assert!(d.repair.is_empty());
            d.validate("operator.nx", &source).unwrap();
            parse("operator.nx", &source.replacen(operator, repaired, 1)).unwrap();
        }
    }
}

#[test]
fn b_missing_terminator_reports_literal_fields_and_repair() {
    let source = "fn Main(){let x=1}";
    let d = parse("fixture", source).unwrap_err();
    assert_eq!(d.code, "NX-E0005");
    assert_eq!(d.stage, Stage::Parse);
    assert_eq!(d.operation, Operation::SyntaxParse);
    assert_eq!(d.parser_context.as_deref(), Some("statement_terminator"));
    assert_eq!(d.expected_tokens, [";"]);
    assert_eq!(
        d.expected_tokens_kind,
        Some(ExpectedTokensKind::CanonicalInsertion)
    );
    assert_eq!(
        d.primary_span.as_ref().map(|s| (s.start, s.end)),
        Some(unique_bytes(source, "}"))
    );
    assert_eq!(
        d.unexpected_token.as_ref().and_then(|t| t.text.as_deref()),
        Some("}")
    );
    assert_eq!(
        d.recognized_prefix.as_ref().map(|s| (s.start, s.end)),
        Some((10, unique_bytes(source, "}").0))
    );
    assert_eq!(d.related_spans[0].span.start, unique_bytes(source, "1").0);
    assert_eq!(d.related_spans[0].span.end, unique_bytes(source, "1").1);
    assert_eq!(d.repair[0].edits[0].replacement, ";");
    d.validate("fixture", source).unwrap();
}

#[test]
fn b_hostile_nesting_in_parens_blocks_arrays_and_types_is_bounded() {
    let sources = [
        format!(
            "fn Main(){{let x={}1{};}}",
            "(".repeat(10_000),
            ")".repeat(10_000)
        ),
        format!("fn Main(){{{}{}}}", "{".repeat(10_000), "}".repeat(10_000)),
        format!(
            "fn Main(){{let x={}1{};}}",
            "[".repeat(10_000),
            "]".repeat(10_000)
        ),
        format!(
            "fn F(x:view {}I64{}){{}}",
            "Array<".repeat(10_000),
            ">".repeat(10_000)
        ),
    ];
    for (index, source) in sources.into_iter().enumerate() {
        let d = parse("nesting.nx", &source).unwrap_err();
        assert_eq!(d.code, "NX-E0003", "{}", &source[..source.len().min(20)]);
        assert_eq!(
            d.data.get("budget_kind"),
            Some(&DataValue::String("syntax_nesting".into()))
        );
        assert_eq!(d.data.get("limit"), Some(&DataValue::UInt(128)));
        assert_eq!(d.data.get("observed"), Some(&DataValue::UInt(129)));
        if index == 3 {
            assert_eq!(
                d.primary_span.as_ref().unwrap().start,
                ("fn F(x:view ".len() + 128 * "Array<".len()) as u64
            );
        }
        d.validate("nesting.nx", &source).unwrap();
    }
}

#[test]
fn b_repeated_and_grouped_modes_classify_after_the_whole_postfix() {
    for (source, feature, needle) in [
        (
            "fn Main(){F(move a);F(move a[0]);}",
            "partial_transfer",
            "a[0]",
        ),
        ("fn Main(){move (a(1));}", "receiver_mode", "move"),
        ("fn Main(){edit F();}", "receiver_mode", "edit"),
        ("fn Main(){move (a)[0];}", "partial_transfer", "(a)[0]"),
        ("fn Main(){move (a[0]);}", "partial_transfer", "(a[0])"),
        ("fn Main(){move a[0][1];}", "partial_transfer", "a[0][1]"),
    ] {
        let d = parse("mode.nx", source).unwrap_err();
        assert_eq!(d.feature_id.as_deref(), Some(feature), "{source}");
        let marker = unique_bytes(source, needle);
        assert_eq!(
            d.primary_span.as_ref().map(|s| (s.start, s.end)),
            Some(marker),
            "{source}"
        );
        d.validate("mode.nx", source).unwrap();
    }
}

#[test]
fn b_function_body_opening_and_structural_closure_follow_production() {
    for source in [
        "fn F()->I64\n{return 1;}",
        "fn F() with no_io\n{}",
        "fn F(){let x=1\n{}}",
        "fn F(){if (if x then {value:1;} else {value:2;}) {}\n x;}",
        "fn F(){while a[if true then {value:0;}else{value:1;}]{}\n x;}",
    ] {
        parse("structure.nx", source).unwrap_or_else(|d| panic!("{source}: {d:?}"));
    }
    let source = "fn F(){return }";
    let d = parse("structure.nx", source).unwrap_err();
    assert_eq!(d.code, "NX-E0005");
    assert_eq!(d.related_spans[0].role, RelatedRole::Context);
    assert_eq!(
        (d.related_spans[0].span.start, d.related_spans[0].span.end),
        unique_bytes(source, "return")
    );
    d.validate("structure.nx", source).unwrap();
}

#[test]
fn b_empty_operation_list_first_is_independent_of_bad_token() {
    const EXPECTED: &[&str] = &["call", "cancel", "copy", "create", "drop", "resume", "}"];
    let prefix = "fn F() with {";
    let eof = parse("with-first.nx", prefix).unwrap_err();
    for token in [
        ")", ";", "x", "1", "value", "(", "[", "]", ",", ":", "=", "->", "with", "else", "let",
        "return", "while", "if", "fn", "no_io", "Array", "I64", "true", "false", "?", "+", "-",
        "<", ">", "==", ".", "@",
    ] {
        let source = format!("{prefix}{token}");
        let d = parse("with-first.nx", &source).unwrap_err();
        assert_eq!(d.code, "NX-E0002", "{source}");
        assert_eq!(d.parser_context, eof.parser_context, "{source}");
        assert_eq!(d.expected_tokens, eof.expected_tokens, "{source}");
        assert_eq!(d.expected_tokens, EXPECTED, "{source}");
        assert_eq!(d.recognized_prefix, eof.recognized_prefix, "{source}");
        assert_eq!(
            d.recognized_prefix.as_ref().map(|s| (s.start, s.end)),
            Some((12, 13))
        );
        assert_eq!(
            d.primary_span.as_ref().unwrap().start as usize,
            prefix.len()
        );
        d.validate("with-first.nx", &source).unwrap();
    }
}

#[test]
fn b_structural_closures_and_commented_crlf_obey_terminator_boundaries() {
    for statement in ["{}", "while true{}", "if true{}else{}"] {
        let source = format!("fn Main(){{{statement}\r\nlet z=2;}}");
        let ast = parse("structure.nx", &source).unwrap();
        assert_eq!(ast.functions[0].body.statements.len(), 2, "{statement}");
        let start = source.find("let z").unwrap() as u64;
        assert!(
            matches!(&ast.functions[0].body.statements[1], Statement::Let { span, .. } if span.start == start)
        );
    }
    let comment = "fn Main(){let x=1//é\r\nlet y=2;}";
    let ast = parse("structure.nx", comment).unwrap();
    let first_end = comment.find("//").unwrap() as u64;
    let second_start = comment.find("let y").unwrap() as u64;
    assert!(
        matches!(&ast.functions[0].body.statements[0], Statement::Let { span, .. } if span.end == first_end)
    );
    assert!(
        matches!(&ast.functions[0].body.statements[1], Statement::Let { span, .. } if span.start == second_start)
    );
    let explicit = "fn Main(){let x=1;//é\r\nlet y=2;}";
    assert_eq!(
        print_canonical(&ast).unwrap(),
        print_canonical(&parse("structure.nx", explicit).unwrap()).unwrap()
    );
    let missing = "fn Main(){let x=if true then {}else{}}";
    let d = parse("structure.nx", missing).unwrap_err();
    assert_eq!(d.code, "NX-E0005");
    assert_eq!(d.parser_context.as_deref(), Some("statement_terminator"));
    assert_eq!(d.expected_tokens, [";"]);
    assert_eq!(
        d.unexpected_token.as_ref().unwrap().text.as_deref(),
        Some("}")
    );
    d.validate("structure.nx", missing).unwrap();
    parse("structure.nx", "fn Main(){let x=if true then {}else{};}").unwrap();
    parse(
        "structure.nx",
        "fn Main(){let x=if true then {}else{}\r\nlet z=2;}",
    )
    .unwrap();
}

#[test]
fn b_l01_l03_l04_keep_parse_trees_and_byte_spans() {
    let l01 = "fn F()->I64{\nreturn\n(1);\n}\nfn Main(){}";
    let ast = parse("l.nx", l01).unwrap();
    assert_eq!(ast.functions.len(), 2);
    assert_eq!(ast.functions[0].body.statements.len(), 2);
    let ret = unique_bytes(l01, "return");
    assert!(
        matches!(&ast.functions[0].body.statements[0], Statement::Return { value: None, span } if (span.start,span.end)==ret)
    );
    assert!(matches!(
        &ast.functions[0].body.statements[1],
        Statement::Expr {
            value: Expr::Group(_, _),
            ..
        }
    ));
    let positive = "fn F()->I64{\nreturn (\n1);\n}\nfn Main(){}";
    let ast = parse("l.nx", positive).unwrap();
    assert_eq!(ast.functions[0].body.statements.len(), 1);
    assert!(matches!(
        &ast.functions[0].body.statements[0],
        Statement::Return {
            value: Some(Expr::Group(_, _)),
            ..
        }
    ));
    let l03 =
        "fn value(x:view I64)->I64{return x;}\nfn Main(){let x=7;let y=value\n(x);AssertEq(y,7);}";
    let ast = parse("l.nx", l03).unwrap();
    let call_start = l03.find("value\n(x)").unwrap() as u64;
    assert!(
        matches!(&ast.functions[1].body.statements[1], Statement::Let { value: Expr::Call { target, span, .. }, .. } if matches!(&**target, Expr::Name(name, _) if name == "value") && (span.start,span.end)==(call_start,call_start+"value\n(x)".len() as u64))
    );
    let l04 = "fn Main(){\nlet value=2\nlet a=value -1\nlet b=value - 1\nAssertEq(a,b)\n}";
    let ast = parse("l.nx", l04).unwrap();
    assert_eq!(ast.functions[0].body.statements.len(), 4);
    let ends: Vec<_> = ast.functions[0]
        .body
        .statements
        .iter()
        .map(|s| match s {
            Statement::Let { span, .. } | Statement::Expr { span, .. } => span.end,
            _ => panic!("unexpected statement"),
        })
        .collect();
    let expected: Vec<_> = [
        "let value=2",
        "let a=value -1",
        "let b=value - 1",
        "AssertEq(a,b)",
    ]
    .iter()
    .map(|needle| unique_bytes(l04, needle).1)
    .collect();
    assert_eq!(ends, expected);
    let binary = unique_bytes(l04, "value -1");
    assert!(
        matches!(&ast.functions[0].body.statements[1], Statement::Let { value: Expr::Binary { op, span, .. }, .. } if op == "-" && (span.start,span.end)==binary)
    );
    let variant = l04.replacen("value -1", "value - 1", 1);
    let other = parse("l.nx", &variant).unwrap();
    let binary_start = variant.find("value - 1").unwrap() as u64;
    let binary = (binary_start, binary_start + "value - 1".len() as u64);
    assert!(
        matches!(&other.functions[0].body.statements[1], Statement::Let { value: Expr::Binary { span, .. }, .. } if (span.start,span.end)==binary)
    );
    assert_eq!(
        print_canonical(&ast).unwrap(),
        print_canonical(&other).unwrap()
    );
}

#[test]
fn b_l05_l06_l07_keep_unit_discard_and_utf8_offsets() {
    use super::lex::{Kind, lex};
    let l05 = "fn Main(){\nlet x=if true then {value: ();}else{value: ();};\nAssertEq(x,());\n}";
    let ast = parse("l.nx", l05).unwrap();
    let value_start = l05.find("value: ();").unwrap() as u64;
    let value = (value_start, value_start + "value: ();".len() as u64);
    assert!(
        matches!(&ast.functions[0].body.statements[0], Statement::Let { value: Expr::If { then_block, .. }, .. } if matches!(&then_block.statements[0], Statement::Value { value: Some(Expr::Unit(_)), span } if (span.start,span.end)==value))
    );
    let l06 = "fn F()->I64{\n1\n}\nfn Main(){}";
    let ast = parse("l.nx", l06).unwrap();
    assert_eq!(ast.functions[0].body.statements.len(), 1);
    let one = unique_bytes(l06, "\n1\n");
    assert!(
        matches!(&ast.functions[0].body.statements[0], Statement::Expr { value: Expr::Integer(_, _), span } if (span.start,span.end)==(one.0+1,one.1-1))
    );
    let positive = "fn F()->I64{\nreturn 1\n}\nfn Main(){}";
    let ast = parse("l.nx", positive).unwrap();
    assert!(matches!(
        &ast.functions[0].body.statements[0],
        Statement::Return {
            value: Some(Expr::Integer(_, _)),
            ..
        }
    ));
    let lf = "// á🙂\nfn Main(){\nlet x=missing;\n}";
    for source in [lf.to_owned(), lf.replace('\n', "\r\n"), format!("{lf}\n")] {
        let tokens = lex("l.nx", &source).unwrap();
        let first = tokens.iter().find(|t| t.kind != Kind::Newline).unwrap();
        assert_eq!(first.text, "fn");
        assert_eq!(first.span.start, unique_bytes(&source, "fn Main").0);
        let missing = unique_bytes(&source, "missing");
        assert!(
            tokens
                .iter()
                .any(|t| t.text == "missing" && (t.span.start, t.span.end) == missing)
        );
        assert_eq!(tokens.last().unwrap().span.start, source.len() as u64);
        let ast = parse("l.nx", &source).unwrap();
        assert!(
            matches!(&ast.functions[0].body.statements[0], Statement::Let { value: Expr::Name(name, span), .. } if name == "missing" && (span.start,span.end)==missing)
        );
        let positive = source.replace("let x=missing;", "let missing=1;let x=missing;");
        let ast = parse("l.nx", &positive).unwrap();
        assert_eq!(ast.functions[0].body.statements.len(), 2);
    }
}

#[test]
fn b_published_first_table_is_ordered_unique_and_applied() {
    const EXPR: &[&str] = &[
        "(",
        "-",
        "IntLiteral",
        "Name",
        "[",
        "edit",
        "false",
        "if",
        "move",
        "true",
    ];
    const BLOCK: &[&str] = &[
        "(",
        "-",
        "IntLiteral",
        "Name",
        "[",
        "edit",
        "false",
        "if",
        "let",
        "move",
        "return",
        "true",
        "var",
        "while",
        "{",
        "}",
    ];
    let table: &[(&str, &str, &[&str])] = &[
        ("Function.Name", "function_name", &["Name"]),
        ("Function.Parameters? EOF", "parameter_list", &[")", "Name"]),
        ("Block.Statement* EOF", "block_item", BLOCK),
        ("Binding.Expr EOF", "binding_initializer", EXPR),
        ("IntLiteral sign EOF", "signed_integer", &["IntLiteral"]),
        ("Array Expr EOF", "array_element", EXPR),
        ("Program trailing token", "module_item", &["EOF", "fn"]),
    ];
    for (production, context, expected) in table {
        assert!(
            expected
                .windows(2)
                .all(|pair| pair[0].as_bytes() < pair[1].as_bytes()),
            "{production}"
        );
        let mut sorted = expected.to_vec();
        sorted.sort_by(|a, b| a.as_bytes().cmp(b.as_bytes()));
        sorted.dedup();
        assert_eq!(sorted, *expected, "{production}");
        assert!(table.iter().any(|(_, candidate, _)| candidate == context));
    }
}

#[test]
fn b_split_type_close_and_first_states_have_regressions() {
    for source in [
        "fn F(){let a:Array<I64>=x;}",
        "fn F(){let a:Result<Array<Array<I64>>,Bool>=x;}",
    ] {
        parse("split.nx", source).unwrap();
    }
    for token in [
        ")", "]", "{", ",", "=", ":", "->", "with", "else", "let", "return", "while", "*", ">",
        "+", "==",
    ] {
        let source = format!("fn F()->I64{{return {token}");
        let d = parse("first.nx", &source).unwrap_err();
        assert_eq!(d.code, "NX-E0005", "{source}");
        let source = format!("fn F(){{let g=if c then {{value: {token}");
        let d = parse("first.nx", &source).unwrap_err();
        assert_eq!(d.code, "NX-E1302", "{source}");
    }
    const WITH: &[&str] = &[
        "no_alloc",
        "no_block",
        "no_heap_alloc",
        "no_io",
        "no_suspend",
        "unrestricted",
        "{",
    ];
    for token in ["foo", "I64", "value", "Array", "1", ";", ")", "="] {
        let source = format!("fn F() with {token}");
        let d = parse("first.nx", &source).unwrap_err();
        assert_eq!(
            d.parser_context.as_deref(),
            Some("grammar.WithClause"),
            "{source}"
        );
        assert_eq!(d.expected_tokens, WITH, "{source}");
    }
    const BLOCK: &[&str] = &[
        "(",
        "-",
        "IntLiteral",
        "Name",
        "[",
        "edit",
        "false",
        "if",
        "let",
        "move",
        "return",
        "true",
        "var",
        "while",
        "{",
        "}",
    ];
    let source = "fn F(){let x=1;fn G(){}";
    let d = parse("first.nx", source).unwrap_err();
    // D15 §3.6 classifies fn in an expression position as deferred closure.
    assert_eq!(d.code, "NX-E0006");
    assert_eq!(d.feature_id.as_deref(), Some("closure"));
    let local_fn = unique_bytes(source, "fn G").0;
    assert_eq!(
        d.primary_span.as_ref().map(|s| (s.start, s.end)),
        Some((local_fn, local_fn + 2))
    );
    let source = "fn F(){let x=1;let y=2;";
    let d = parse("first.nx", source).unwrap_err();
    assert_eq!(d.parser_context.as_deref(), Some("grammar.Block"));
    assert_eq!(d.expected_tokens, BLOCK);
    d.validate("first.nx", source).unwrap();
}

#[test]
fn b_branch_value_negatives_keep_codes_spans_and_related_locations() {
    for (source, subcause, primary, related, related_len) in [
        (
            "fn Main(){value:1;}",
            "invalid_value_target",
            "value:",
            "{value:",
            1,
        ),
        (
            "fn F()->I64{return if true then {value:}else{value:2;};} fn Main(){}",
            "missing_value_operand",
            "value:",
            "{value:",
            1,
        ),
        (
            "fn F()->I64{return if true then {value:1;2;}else{value:2;};} fn Main(){}",
            "non_final_branch_value",
            "value:",
            "2;",
            2,
        ),
        (
            "fn F()->I64{\nreturn if true then {\nvalue:\n(1);\n}else{value:2;};\n}\nfn Main(){}",
            "missing_value_operand",
            "value:",
            "{\nvalue:",
            1,
        ),
    ] {
        let d = parse("branches.nx", source).unwrap_err();
        assert_eq!(d.code, "NX-E1302", "{source}");
        assert_eq!(d.subcause.as_deref(), Some(subcause), "{source}");
        assert_eq!(d.stage, Stage::Parse);
        assert_eq!(d.operation, Operation::SyntaxParse);
        let start = source.find(primary).unwrap() as u64;
        let expected_end = start + "value:".len() as u64;
        assert_eq!(
            d.primary_span.as_ref().map(|s| (s.start, s.end)),
            Some((start, expected_end)),
            "{source}"
        );
        let related_at = source.find(related).unwrap() as u64;
        assert!(
            d.related_spans
                .iter()
                .any(|r| r.role == RelatedRole::Context
                    && (r.span.start, r.span.end) == (related_at, related_at + related_len)),
            "{source}: {d:?}"
        );
        d.validate("branches.nx", source).unwrap();
    }
    let l02 = "fn F()->I64{\nreturn if true then {\nvalue:\n(1);\n}else{value:2;};\n}\nfn Main(){}";
    let d = parse("branches.nx", l02).unwrap_err();
    let newline = l02.find("value:\n").unwrap() as u64 + "value:".len() as u64;
    assert!(
        d.related_spans
            .iter()
            .any(|r| r.role == RelatedRole::Delimiter
                && (r.span.start, r.span.end) == (newline, newline + 1))
    );
    let positive =
        "fn F()->I64{\nreturn if true then {\nvalue: (\n1);\n}else{value:2;};\n}\nfn Main(){}";
    let ast = parse("branches.nx", positive).unwrap();
    assert!(
        matches!(&ast.functions[0].body.statements[0], Statement::Return { value: Some(Expr::If { then_block, .. }), .. } if matches!(&then_block.statements[0], Statement::Value { value: Some(Expr::Group(_, _)), .. }))
    );
    for source in [
        "fn Main(){let value=1;}",
        "fn F()->I64{return if true then {value:1;}else{value:2;};} fn Main(){}",
    ] {
        parse("branches.nx", source).unwrap();
    }
}

#[test]
fn b_with_clause_rows_and_accepted_forms_keep_exact_diagnostics() {
    use super::parse::WithClause;
    for (source, cause, marker, last) in [
        (
            "fn Main() with {call(no_io),call(no_alloc)}{}",
            "duplicate_operation",
            "call",
            true,
        ),
        (
            "fn Main() with {resume(no_io)}{}",
            "invalid_operation",
            "resume",
            false,
        ),
        (
            "fn Main() with no_io,no_io{}",
            "duplicate_restriction",
            "no_io",
            true,
        ),
        ("fn Main() with {}{}", "empty_operation_list", "{}", false),
        (
            "fn Main() with unrestricted,no_io{}",
            "unrestricted_mixed",
            "unrestricted,no_io",
            false,
        ),
        (
            "fn Main() with {call(no_io,unrestricted)}{}",
            "unrestricted_mixed",
            "no_io,unrestricted",
            false,
        ),
        (
            "fn Main() with unrestricted,unrestricted,no_io{}",
            "duplicate_restriction",
            "unrestricted",
            true,
        ),
    ] {
        let d = parse("with.nx", source).unwrap_err();
        assert_eq!(d.code, "NX-E0417", "{source}");
        assert_eq!(d.subcause.as_deref(), Some(cause), "{source}");
        assert_eq!(d.stage, Stage::Parse);
        assert_eq!(d.operation, Operation::SyntaxParse);
        let start = if last {
            source.rfind(marker).unwrap()
        } else {
            source.find(marker).unwrap()
        } as u64;
        assert_eq!(
            d.primary_span.as_ref().map(|s| (s.start, s.end)),
            Some((start, start + marker.len() as u64)),
            "{source}"
        );
        d.validate("with.nx", source).unwrap();
    }
    for source in [
        "fn Main() with no_io{}",
        "fn Main() with unrestricted{}",
        "fn Main() with {call(unrestricted)}{}",
        "fn Main() with {call(no_alloc,no_io)}{}",
    ] {
        let ast = parse("with.nx", source).unwrap();
        let printed = print_canonical(&ast).unwrap();
        assert_eq!(
            print_canonical(&parse("with.nx", &printed).unwrap()).unwrap(),
            printed
        );
        assert!(ast.functions[0].with_clause.is_some());
    }
    let simple = parse("with.nx", "fn Main() with no_io{}").unwrap();
    assert_eq!(
        simple.functions[0].with_clause,
        Some(WithClause::Simple(vec!["no_io".into()]))
    );
    let map = parse("with.nx", "fn Main() with {call(no_alloc,no_io)}{}").unwrap();
    assert!(
        matches!(&map.functions[0].with_clause, Some(WithClause::Operations(ops)) if ops.len()==1 && ops[0].name=="call" && ops[0].restrictions==["no_alloc","no_io"] && (ops[0].span.start,ops[0].span.end)==unique_bytes("fn Main() with {call(no_alloc,no_io)}{}", "call(no_alloc,no_io)"))
    );
    for source in [
        "fn Main() with {call(no_io)}{}",
        "fn Main() with no_io{}",
        "fn Main() with {call(unrestricted)}{}",
    ] {
        parse("with.nx", source).unwrap();
    }
    let source = "fn Main() with no_heap_alloc{}";
    let excluded = parse("with.nx", source).unwrap_err();
    assert_eq!(excluded.code, "NX-E0006");
    assert_eq!(excluded.feature_id.as_deref(), Some("no_heap_alloc"));
    excluded.validate("with.nx", source).unwrap();
    let source = "fn Main() with create(no_io){}";
    let d = parse("with.nx", source).unwrap_err();
    assert_eq!(d.code, "NX-E0002");
    assert_eq!(d.parser_context.as_deref(), Some("grammar.WithClause"));
    d.validate("with.nx", source).unwrap();
}

#[test]
fn member_selection_keeps_its_feature_across_a_line_break() {
    for source in ["fn Main(){x.y;}", "fn Main(){x\n.y;}"] {
        let d = parse("member.nx", source).unwrap_err();
        assert_eq!(d.feature_id.as_deref(), Some("methods"), "{source}");
        d.validate("member.nx", source).unwrap();
    }
}

#[test]
fn local_bindings_and_parameters_shadow_function_names_in_b() {
    for source in [
        "fn value(){} fn Main(){let value=1;let b=value;}",
        "fn value(){} fn Main(value:view I64){let b=value;}",
        "fn value(){} fn Main(){{let value=1;let b=value;}value();}",
    ] {
        parse("scope.nx", source).unwrap();
    }
    for source in [
        "fn F(){} fn Main(){let g=F;}",
        "fn F(){} fn Main(){let g=(F);}",
        "fn F(){} fn Main(){G(F);}",
    ] {
        let d = parse("scope.nx", source).unwrap_err();
        assert_eq!(d.feature_id.as_deref(), Some("function_value"), "{source}");
        assert_eq!(d.stage, Stage::Names);
        d.validate("scope.nx", source).unwrap();
    }
}

#[test]
fn expression_statement_has_no_extra_outer_depth_charge_in_b() {
    for n in [126, 127] {
        parse(
            "nest.nx",
            &format!("fn Main(){{{}1{};}}", "F(".repeat(n), ")".repeat(n)),
        )
        .unwrap();
        parse(
            "nest.nx",
            &format!("fn Main(){{{}1{};}}", "[".repeat(n), "]".repeat(n)),
        )
        .unwrap();
    }
    for n in [128, 129] {
        for (open, close) in [("F(", ")"), ("[", "]")] {
            let source = format!("fn Main(){{{}1{};}}", open.repeat(n), close.repeat(n));
            let d = parse("nest.nx", &source).unwrap_err();
            assert_eq!(
                d.data.get("budget_kind"),
                Some(&DataValue::String("syntax_nesting".into())),
                "{source}"
            );
            d.validate("nest.nx", &source).unwrap();
        }
    }
}

#[test]
fn grouping_does_not_increase_expression_tree_height_in_b() {
    let admitted = format!("fn Main(){{let x=(1{});}}", "+1".repeat(127));
    parse("height.nx", &admitted).unwrap();
    let grouped_child = format!("fn Main(){{let x=(1{})+1;}}", "+1".repeat(126));
    parse("height.nx", &grouped_child).unwrap();
    let excessive = format!("fn Main(){{let x=(1{});}}", "+1".repeat(128));
    let d = parse("height.nx", &excessive).unwrap_err();
    assert_eq!(
        d.data.get("budget_kind"),
        Some(&DataValue::String("expression_tree_depth".into()))
    );
    d.validate("height.nx", &excessive).unwrap();
}

#[test]
fn repeated_mode_prefix_reports_non_place_in_b() {
    for (mode, context) in [("move", "consume_place"), ("edit", "inout_place")] {
        let source = format!("fn Main(){{let x={mode} {mode} a;}}");
        let d = parse("prefix.nx", &source).unwrap_err();
        assert_eq!(d.code, "NX-E0002");
        assert_eq!(d.parser_context.as_deref(), Some(context));
        assert_eq!(d.expected_tokens, ["Place"]);
        assert_eq!(
            d.unexpected_token.as_ref().unwrap().text.as_deref(),
            Some(mode)
        );
        assert_eq!(
            d.primary_span.as_ref().unwrap().start as usize,
            source.rfind(mode).unwrap()
        );
        d.validate("prefix.nx", &source).unwrap();
    }
}

#[test]
fn try_after_access_has_no_postfix_continuation_in_b() {
    for source in ["fn Main(){let y=move x?;}", "fn Main(){F(edit a[0]);}"] {
        parse("try.nx", source).unwrap_or_else(|d| panic!("{source}: {d:?}"));
    }
    for source in ["fn Main(){let y=edit a[0]?;}", "fn Main(){F(edit x?);}"] {
        let d = parse("try.nx", source).unwrap_err();
        assert_eq!(d.code, "NX-E0002");
        assert_eq!(d.parser_context.as_deref(), Some("inout_place"));
        d.validate("try.nx", source).unwrap();
    }
    for token in ["?", "(", "["] {
        let source = format!("fn Main(){{a?{token}1;}}");
        let d = parse("try.nx", &source).unwrap_err();
        assert_eq!(d.code, "NX-E0002");
        assert_eq!(d.parser_context.as_deref(), Some("grammar.TryExpr"));
        assert_eq!(
            d.unexpected_token.as_ref().unwrap().text.as_deref(),
            Some(token)
        );
        d.validate("try.nx", &source).unwrap();
    }
}

#[test]
fn ordinary_names_stay_clear_of_deferred_feature_recognition_in_b() {
    let names = [
        "Option",
        "from",
        "class",
        "value",
        "moveItem",
        "asyncItem",
        "consumeItem",
    ];
    for seed in 0..2_500usize {
        let name = names[seed % names.len()];
        let source = format!(
            "fn F({name}:view I64)->I64{{let option={name};let from=option;let class=from;let case{seed}=class;return case{seed};}} fn Main(){{let {name}=1;}}"
        );
        parse("names.nx", &source).unwrap_or_else(|d| panic!("{seed}: {source}: {d:?}"));
    }
}

#[test]
fn signed_literals_and_operator_precedence_keep_their_ast_shape_in_b() {
    let source = "fn F(){let x=-42;let y=1+2<4;let z=(1<2)+3;let w=4-(2-1);}";
    let ast = parse("precedence.nx", source).unwrap();
    let ss = &ast.functions[0].body.statements;
    assert!(matches!(&ss[0], Statement::Let { value: Expr::Integer(n, _), .. } if n == "-42"));
    assert!(
        matches!(&ss[1], Statement::Let { value: Expr::Binary { op, left, .. }, .. } if op == "<" && matches!(&**left, Expr::Binary { op, .. } if op == "+"))
    );
    let printed = print_canonical(&ast).unwrap();
    assert!(printed.contains("(1 < 2) + 3"));
    assert!(printed.contains("4 - (2 - 1)"));
    assert_eq!(
        ast_without_spans(ast),
        ast_without_spans(parse("precedence.nx", &printed).unwrap())
    );
    let ast = parse("precedence.nx", "fn F(){let x=1<2+3;let y=1-2+3;}").unwrap();
    assert!(
        matches!(&ast.functions[0].body.statements[0], Statement::Let { value: Expr::Binary { op, right, .. }, .. } if op == "<" && matches!(&**right, Expr::Binary { op, .. } if op == "+"))
    );
    assert!(print_canonical(&ast).unwrap().contains("1 - 2 + 3"));
    let mut synthetic = parse("precedence.nx", "fn F(){let x=1+(2<3);}").unwrap();
    if let Statement::Let {
        value: Expr::Binary { right, .. },
        ..
    } = &mut synthetic.functions[0].body.statements[0]
    {
        if let Expr::Group(inner, _) = &mut **right {
            *right = inner.clone();
        } else {
            panic!("expected grouping");
        }
    } else {
        panic!("expected addition");
    }
    assert!(print_canonical(&synthetic).unwrap().contains("1 + (2 < 3)"));
}

#[test]
fn else_block_and_else_if_keep_distinct_ast_and_printed_shapes_in_b() {
    let block = parse(
        "else.nx",
        "fn F(){let x=if true then {value:1;}else{value:2;};}",
    )
    .unwrap();
    let chained = parse(
        "else.nx",
        "fn F(){let x=if true then {value:1;}else(if true then {value:2;}else{value:3;});}",
    )
    .unwrap();
    assert_ne!(
        ast_without_spans(block.clone()),
        ast_without_spans(chained.clone())
    );
    assert!(print_canonical(&block).unwrap().contains("else 2"));
    assert!(
        print_canonical(&chained)
            .unwrap()
            .contains("else (if true then")
    );
}

#[test]
fn committed_rhs_keeps_the_inner_first_production_in_b() {
    for (source, context) in [
        ("fn Main(){var x=0;x=;}", "grammar.Primary"),
        ("fn Main(){let x=1+;}", "grammar.TryExpr"),
        ("fn Main(){let x=1<;}", "grammar.TryExpr"),
    ] {
        let d = parse("gap.nx", source).unwrap_err();
        assert_eq!(d.code, "NX-E0002", "{source}");
        assert_eq!(d.parser_context.as_deref(), Some(context), "{source}");
        d.validate("gap.nx", source).unwrap();
    }
}

#[test]
fn type_unit_terminal_and_lexical_priority_stay_explicit_in_b() {
    let source = "fn F(x:view (;";
    let d = parse("gap.nx", source).unwrap_err();
    assert_eq!(d.parser_context.as_deref(), Some("grammar.Type"));
    assert_eq!(d.expected_tokens, [")"]);
    d.validate("gap.nx", source).unwrap();
    let source = "fn Main(){let x=0.5/1;}";
    let d = parse("gap.nx", source).unwrap_err();
    assert_eq!(d.code, "NX-E0006");
    assert_eq!(d.feature_id.as_deref(), Some("float_literal"));
    d.validate("gap.nx", source).unwrap();
}

#[test]
fn function_value_and_later_syntax_error_priority_stays_explicit_in_b() {
    for (source, code, context) in [
        (
            "fn F(){} fn G(){let x=F<1<2;}",
            "NX-E0002",
            Some("grammar.CompareExpr"),
        ),
        (
            "fn F(){} fn G(){let x=F*2;}",
            "NX-E0002",
            Some("unsupported_binary_operator"),
        ),
        ("fn F(){} fn G(){let x=F 1;}", "NX-E0006", None),
        (
            "fn F(){} fn G(){let x=(F",
            "NX-E0002",
            Some("grammar.Primary"),
        ),
        ("fn F(){} fn G(){G(F", "NX-E0006", None),
    ] {
        let d = parse("priority.nx", source).unwrap_err();
        assert_eq!(d.code, code, "{source}");
        assert_eq!(d.parser_context.as_deref(), context, "{source}");
        d.validate("priority.nx", source).unwrap();
    }
}

#[test]
fn unsupported_operator_prefix_starts_at_the_active_b_statement() {
    for (source, statement) in [
        ("fn Main(){ let x=1*2;}", "let x"),
        ("fn Main(){ if x {}\n a*b;}", "a*b"),
        ("fn Main(){if x {} a*b;}", "a*b"),
    ] {
        let d = parse("operator.nx", source).unwrap_err();
        assert_eq!(
            d.parser_context.as_deref(),
            Some("unsupported_binary_operator")
        );
        let start = source.find(statement).unwrap() as u64;
        let end = source.find('*').unwrap() as u64;
        assert_eq!(
            d.recognized_prefix.as_ref().map(|s| (s.start, s.end)),
            Some((start, end))
        );
        d.validate("operator.nx", source).unwrap();
    }
}

#[test]
fn nested_while_condition_uses_its_own_b_statement_start() {
    let source = "fn F(){let x=1;if ok{while ready * 2 {}}}";
    let d = parse("while-prefix.nx", source).unwrap_err();
    assert_eq!(d.code, "NX-E0002");
    assert_eq!(
        d.parser_context.as_deref(),
        Some("unsupported_binary_operator")
    );
    assert_eq!(d.expected_tokens, ["+", "-", "<", "=="]);
    let operator = source.find('*').unwrap() as u64;
    let statement = source.find("while ready").unwrap() as u64;
    assert_eq!(
        d.primary_span.as_ref().map(|s| (s.start, s.end)),
        Some((operator, operator + 1))
    );
    assert_eq!(
        d.recognized_prefix.as_ref().map(|s| (s.start, s.end)),
        Some((statement, operator))
    );
    assert_eq!(
        d.unexpected_token.as_ref().and_then(|t| t.text.as_deref()),
        Some("*")
    );
    d.validate("while-prefix.nx", source).unwrap();
}

#[test]
fn nullable_return_and_complete_try_at_eof_need_b_terminators() {
    for source in ["fn Main(){return", "fn Main(){let x=1?"] {
        let end = source.len() as u64;
        let d = parse("eof.nx", source).unwrap_err();
        assert_eq!(d.code, "NX-E0005", "{source}");
        assert_eq!(d.parser_context.as_deref(), Some("statement_terminator"));
        assert_eq!(d.expected_tokens, [";"]);
        assert_eq!(
            d.expected_tokens_kind,
            Some(ExpectedTokensKind::CanonicalInsertion)
        );
        assert_eq!(
            d.primary_span.as_ref().map(|s| (s.start, s.end)),
            Some((end, end))
        );
        assert_eq!(d.unexpected_token.as_ref().unwrap().kind, TokenKind::Eof);
        assert_eq!(
            d.unexpected_token.as_ref().unwrap().text.as_deref(),
            Some("")
        );
        assert_eq!(
            d.recognized_prefix.as_ref().map(|s| (s.start, s.end)),
            Some((10, end))
        );
        assert_eq!(d.repair[0].edits[0].replacement, ";");
        d.validate("eof.nx", source).unwrap();
        parse("eof.nx", &format!("{source};}}")).unwrap();
    }
}

#[test]
fn place_requires_a_name_root_through_grouping_and_indexing_in_b() {
    let ast = parse("place.nx", "fn Main(){let a=[0];(a)[0]=1;}").unwrap();
    assert!(matches!(
        ast.functions[0].body.statements[1],
        Statement::Assign { .. }
    ));
    let source = "fn Main(){1=2;}";
    let d = parse("place.nx", source).unwrap_err();
    assert_eq!(d.code, "NX-E0005");
    assert_eq!(d.parser_context.as_deref(), Some("statement_terminator"));
    let eq = source.find('=').unwrap() as u64;
    assert_eq!(
        d.primary_span.as_ref().map(|s| (s.start, s.end)),
        Some((eq, eq + 1))
    );
    d.validate("place.nx", source).unwrap();
    let source = "fn Main(){let a=1;move (1+2);}";
    let d = parse("place.nx", source).unwrap_err();
    assert_eq!(d.code, "NX-E0002");
    assert_eq!(d.parser_context.as_deref(), Some("consume_place"));
    assert_eq!(d.expected_tokens, ["Place"]);
    let integer = source.find("(1").unwrap() as u64 + 1;
    assert_eq!(
        d.primary_span.as_ref().map(|s| (s.start, s.end)),
        Some((integer, integer + 1))
    );
    assert_eq!(
        d.unexpected_token.as_ref().unwrap().kind,
        TokenKind::Integer
    );
    assert_eq!(
        d.unexpected_token.as_ref().unwrap().text.as_deref(),
        Some("1")
    );
    let prefix = source.find("move").unwrap() as u64;
    assert_eq!(
        d.recognized_prefix.as_ref().map(|s| (s.start, s.end)),
        Some((prefix, integer))
    );
    d.validate("place.nx", source).unwrap();
}

#[test]
fn value_colon_uses_one_token_lookahead_at_b_statement_start() {
    let source = "fn F(value:view I64){} fn Main(){let value:\nI64=2;F(value:value);}";
    let ast = parse("value-context.nx", source).unwrap();
    assert!(
        matches!(&ast.functions[1].body.statements[0], Statement::Let { name, annotation: Some(super::parse::Type::I64), .. } if name == "value")
    );
    assert!(
        matches!(&ast.functions[1].body.statements[1], Statement::Expr { value: Expr::Call { args, .. }, .. } if args[0].label.as_deref() == Some("value") && matches!(&args[0].value, Expr::Name(name, _) if name == "value"))
    );
    let source = "fn Main(){value\n:1;}";
    let d = parse("value-context.nx", source).unwrap_err();
    assert_eq!(d.code, "NX-E0002");
    assert_ne!(d.subcause.as_deref(), Some("invalid_value_target"));
    d.validate("value-context.nx", source).unwrap();
    let source = "fn F()->I64{return if true then {value :1;}else{value:2;};} fn Main(){}";
    let ast = parse("value-context.nx", source).unwrap();
    assert!(
        matches!(&ast.functions[0].body.statements[0], Statement::Return { value: Some(Expr::If { then_block, .. }), .. } if matches!(&then_block.statements[0], Statement::Value { .. }))
    );
}

#[test]
fn missing_else_inside_delimiters_precedes_a_later_b_terminator() {
    for source in [
        "fn F(){let x=[if c then {value:1;}+",
        "fn F(){let x=(if c then {value:1;}?",
        "fn F(){G(if c then {value:1;}+",
    ] {
        let d = parse("gap.nx", source).unwrap_err();
        assert_eq!(d.code, "NX-E1302", "{source}: {d:?}");
        assert_eq!(d.subcause.as_deref(), Some("missing_else"), "{source}");
        d.validate("gap.nx", source).unwrap();
    }
}

#[test]
fn statement_local_newlines_do_not_invent_b_source_tokens() {
    for source in [
        "fn Main(){let a=(if c then {\nvalue: 1\n} else {\nvalue: 2\n});}",
        "fn Main(){let x\n=1;}",
        "fn Main(){let x:I64\n=1;}",
        "fn F()\n-> I64{return 1;}",
        "fn F()\nwith no_io{}",
        "fn Main(){let x\n: I64\n=1; let y=2\nlet z=3;}",
    ] {
        parse("newline.nx", source).unwrap_or_else(|d| panic!("{source}: {d:?}"));
    }
    for source in [
        "fn Main(){x\n, 1;}",
        "fn Main(){x\nelse{}}",
        "fn Main(){F(x\ny);}",
    ] {
        let d = parse("newline.nx", source).unwrap_err();
        assert_eq!(d.code, "NX-E0002", "{source}");
        let token = d.unexpected_token.as_ref().unwrap();
        assert!(token.span.start < token.span.end, "{source}");
        d.validate("newline.nx", source).unwrap();
    }
}

#[test]
fn feature_scanner_respects_b_token_boundaries_and_comments() {
    let valid = "// move async class unsafe match for region self Option no_heap_alloc\nfn Main(){let moveCount=1;let asyncTask=2;let className=3;let unsafeFlag=4;let matchable=5;let noHeapAllocator=6;let optionValue=7;let regionName=8;let selfless=9;let noHeapAlloc=10;let result=moveCount+asyncTask+className+unsafeFlag+matchable+noHeapAllocator+optionValue+regionName+selfless+noHeapAlloc;}";
    parse("scanner.nx", valid).unwrap();
    let comptime = "fn Main(){let x=comptime 1;}";
    let d = parse("scanner.nx", comptime).unwrap_err();
    assert_eq!(d.code, "NX-E0006");
    assert_eq!(d.feature_id.as_deref(), Some("comptime"));
    d.validate("scanner.nx", comptime).unwrap();
    let source = "fn Main(){let x=\"move async class unsafe\";}";
    let d = parse("scanner.nx", source).unwrap_err();
    assert_eq!(d.feature_id.as_deref(), Some("text"));
    let start = source.find('"').unwrap() as u64;
    let end = source.rfind('"').unwrap() as u64 + 1;
    assert_eq!(
        d.primary_span.as_ref().map(|s| (s.start, s.end)),
        Some((start, end))
    );
    d.validate("scanner.nx", source).unwrap();
}
