//! Remaining parser contracts migrated from the previous surface.
use super::lex::{self, Kind};
use super::parse::{Expr, Statement, parse};
use crate::diagnostic::{
    DataValue, ExpectedTokensKind, Operation, RelatedRole, Span, Stage, TokenKind,
};

fn byte_span(source: &str, marker: &str, occurrence: usize) -> (u64, u64) {
    let (start, _) = source
        .match_indices(marker)
        .nth(occurrence)
        .expect("fixture marker");
    (start as u64, (start + marker.len()) as u64)
}

const EXPR_START: &[&str] = &[
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
const TRY_START: &[&str] = &[
    "(",
    "-",
    "IntLiteral",
    "Name",
    "[",
    "edit",
    "false",
    "move",
    "true",
];
const BLOCK_NEXT: &[&str] = &[
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
const TYPE_START: &[&str] = &[
    "(",
    "Array",
    "Bool",
    "I64",
    "MockBlockingResource",
    "MockResource",
    "Result",
];

fn assert_first_row(
    source: &str,
    context: &str,
    expected: &[&str],
    primary: (u64, u64),
    prefix: (u64, u64),
    token: (TokenKind, &str),
) {
    let d = parse("first.nx", source).unwrap_err();
    assert_eq!(d.code, "NX-E0002", "{source}");
    assert_eq!(
        (d.stage, d.operation),
        (Stage::Parse, Operation::SyntaxParse)
    );
    assert_eq!(d.parser_context.as_deref(), Some(context), "{source}");
    assert_eq!(d.expected_tokens, expected, "{source}");
    assert_eq!(
        d.expected_tokens_kind,
        Some(ExpectedTokensKind::AdmissibleSet)
    );
    assert_eq!(
        d.primary_span.as_ref().map(|s| (s.start, s.end)),
        Some(primary)
    );
    let unexpected = d.unexpected_token.as_ref().unwrap();
    assert_eq!(
        (unexpected.kind, unexpected.text.as_deref()),
        (token.0, Some(token.1))
    );
    assert_eq!((unexpected.span.start, unexpected.span.end), primary);
    assert_eq!(
        d.recognized_prefix.as_ref().map(|s| (s.start, s.end)),
        Some(prefix)
    );
    d.validate("first.nx", source).unwrap();
}

#[test]
fn b_published_first_rows_keep_all_literal_diagnostic_fields() {
    // F0-D15 §6.3, seven literal rows with bytes counted independently of parse.
    for (source, context, expected, primary, prefix, token) in [
        (
            "fn",
            "function_name",
            &["Name"][..],
            (2, 2),
            (0, 2),
            (TokenKind::Eof, ""),
        ),
        (
            "fn Main(",
            "parameter_list",
            &[")", "Name"][..],
            (8, 8),
            (7, 8),
            (TokenKind::Eof, ""),
        ),
        (
            "fn Main(){",
            "block_item",
            BLOCK_NEXT,
            (10, 10),
            (9, 10),
            (TokenKind::Eof, ""),
        ),
        (
            "fn Main(){let a=",
            "binding_initializer",
            EXPR_START,
            (16, 16),
            (10, 16),
            (TokenKind::Eof, ""),
        ),
        (
            "fn Main(){let a=-",
            "signed_integer",
            &["IntLiteral"][..],
            (17, 17),
            (16, 17),
            (TokenKind::Eof, ""),
        ),
        (
            "fn Main(){let a=[1,",
            "array_element",
            EXPR_START,
            (19, 19),
            (16, 19),
            (TokenKind::Eof, ""),
        ),
        (
            "fn Main(){let a=1;} garbage",
            "module_item",
            &["EOF", "fn"][..],
            (20, 27),
            (0, 20),
            (TokenKind::Name, "garbage"),
        ),
    ] {
        assert_first_row(source, context, expected, primary, prefix, token);
    }
}

#[test]
fn b_grammar_first_truncation_rows_keep_sorted_sets_and_byte_prefixes() {
    let arguments: &[&str] = &[
        "(",
        ")",
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
    let rows: &[(&str, &str, &[&str], u64)] = &[
        ("fn", "function_name", &["Name"], 0),
        ("fn Main(", "parameter_list", &[")", "Name"], 7),
        ("fn F(x:view ", "grammar.Type", TYPE_START, 12),
        ("fn Main(){", "block_item", BLOCK_NEXT, 9),
        ("fn Main(){let a=", "binding_initializer", EXPR_START, 10),
        ("fn Main(){let a=-", "signed_integer", &["IntLiteral"], 16),
        ("fn Main(){let a=[1,", "array_element", EXPR_START, 16),
        ("fn Main(){let x=1+", "grammar.TryExpr", TRY_START, 18),
        ("fn Main(){F(", "grammar.Arguments", arguments, 11),
        ("fn Main(){let x:Array<", "grammar.Type", TYPE_START, 22),
    ];
    for &(source, context, expected, prefix_start) in rows {
        let at = source.len() as u64;
        assert_first_row(
            source,
            context,
            expected,
            (at, at),
            (prefix_start, at),
            (TokenKind::Eof, ""),
        );
        assert!(
            expected
                .windows(2)
                .all(|w| w[0].as_bytes() < w[1].as_bytes()),
            "{source}"
        );
    }
}

#[test]
fn b_grammar_rejects_trailing_commas_chained_comparisons_and_if_operands() {
    let rows: &[(&str, &str, &[&str], &str, usize)] = &[
        (
            "fn F(a:view I64,){}",
            "grammar.Parameter",
            &["Name"],
            ")",
            0,
        ),
        ("fn F(){G(1,);}", "grammar.Argument", EXPR_START, ")", 1),
        ("fn F(){let x=[1,];}", "array_element", EXPR_START, "]", 0),
        (
            "fn F(){let x=a<b<c;}",
            "grammar.CompareExpr",
            &[";"],
            "<",
            1,
        ),
        (
            "fn F(){let x=a==b==c;}",
            "grammar.CompareExpr",
            &[";"],
            "==",
            1,
        ),
        (
            "fn F(){let x=a<b==c;}",
            "grammar.CompareExpr",
            &[";"],
            "==",
            0,
        ),
        (
            "fn F(){let x=1+if true then {value:1;}else{value:2;};}",
            "grammar.TryExpr",
            TRY_START,
            "if",
            0,
        ),
        (
            "fn F(){let x=1<if true then {value:1;}else{value:2;};}",
            "grammar.TryExpr",
            TRY_START,
            "if",
            0,
        ),
        (
            "fn F(){let x=if true then {value:1;}else{value:2;}+1;}",
            "grammar.Expr",
            &[";"],
            "+",
            0,
        ),
        (
            "fn F(){let x=if true then {value:1;}else{value:2;}?;}",
            "grammar.Expr",
            &[";"],
            "?",
            0,
        ),
    ];
    for &(source, context, expected, token, occurrence) in rows {
        let d = parse("grammar.nx", source).unwrap_err();
        assert_eq!(d.code, "NX-E0002", "{source}: {d:?}");
        assert_eq!(d.parser_context.as_deref(), Some(context), "{source}");
        assert_eq!(d.expected_tokens, expected, "{source}");
        assert_eq!(
            d.unexpected_token.as_ref().and_then(|t| t.text.as_deref()),
            Some(token),
            "{source}"
        );
        assert_eq!(
            d.primary_span.as_ref().map(|s| (s.start, s.end)),
            Some(byte_span(source, token, occurrence)),
            "{source}"
        );
        d.validate("grammar.nx", source).unwrap();
    }
    for source in [
        "fn F(a:view I64){}",
        "fn F(){G(1);}",
        "fn F(){let x=[1];}",
        "fn F(){let x=a<b;}",
        "fn F(){let x=a==b;}",
        "fn F(){let x=(if true then {value:1;}else{value:2;})+1;}",
    ] {
        parse("grammar.nx", source).unwrap_or_else(|d| panic!("{source}: {d:?}"));
    }
}

#[test]
fn b_erroneous_tokens_keep_literal_first_sets_of_committed_states() {
    let group: &[&str] = &[
        "(",
        ")",
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
    let array: &[&str] = &[
        "(",
        "-",
        "IntLiteral",
        "Name",
        "[",
        "]",
        "edit",
        "false",
        "if",
        "move",
        "true",
    ];
    let rows: &[(&str, &str, &[&str])] = &[
        ("fn Main(;", "parameter_list", &[")", "Name"]),
        ("fn Main(){;}", "block_item", BLOCK_NEXT),
        ("fn Main(){1+;}", "grammar.TryExpr", TRY_START),
        ("fn Main(){if true{}else ;}", "grammar.IfForm", &["if", "{"]),
        (
            "fn F() with ;",
            "grammar.WithClause",
            &[
                "no_alloc",
                "no_block",
                "no_heap_alloc",
                "no_io",
                "no_suspend",
                "unrestricted",
                "{",
            ],
        ),
        ("fn Main(){let a=(;}", "grammar.Primary", group),
        ("fn Main(){let a=[;}", "grammar.Primary", array),
    ];
    for &(source, context, expected) in rows {
        let d = parse("first.nx", source).unwrap_err();
        assert_eq!(d.code, "NX-E0002", "{source}");
        assert_eq!(d.parser_context.as_deref(), Some(context), "{source}");
        assert_eq!(d.expected_tokens, expected, "{source}");
        assert_eq!(
            d.unexpected_token.as_ref().and_then(|t| t.text.as_deref()),
            Some(";")
        );
        d.validate("first.nx", source).unwrap();
    }
    // D15 §2.2.3: bare edit/move are Names, so A's Postfix FIRST row is retired.
    for source in ["fn Main(){let x=edit ;}", "fn Main(){let x=move ;}"] {
        let ast = parse("first.nx", source).unwrap();
        assert!(matches!(
            &ast.functions[0].body.statements[0],
            Statement::Let {
                value: Expr::Name(_, _),
                ..
            }
        ));
    }
}

#[test]
fn b_first_sets_follow_the_committed_production() {
    let with_first: &[&str] = &[
        "no_alloc",
        "no_block",
        "no_heap_alloc",
        "no_io",
        "no_suspend",
        "unrestricted",
        "{",
    ];
    let rows: &[(&str, &str, &[&str])] = &[
        ("fn F()", "function_suffix", &["->", "with", "{"]),
        ("fn F()->I64", "function_suffix", &["with", "{"]),
        ("fn F() with", "grammar.WithClause", with_first),
        ("fn F() with no_io", "function_suffix", &[",", "{"]),
        (
            "fn F() with {",
            "grammar.OperationList",
            &["call", "cancel", "copy", "create", "drop", "resume", "}"],
        ),
        ("fn F(){let x", "grammar.Binding", &[":", "="]),
        ("fn F(){let x=1+", "grammar.TryExpr", TRY_START),
        ("fn F(){let x=1<", "grammar.TryExpr", TRY_START),
        ("fn F(){let x=1==", "grammar.TryExpr", TRY_START),
        ("fn F(x:", "grammar.ParamMode", &["edit", "move", "view"]),
        ("fn F(x:view ", "grammar.Type", TYPE_START),
    ];
    for &(source, context, expected) in rows {
        let d = parse("first.nx", source).unwrap_err();
        assert_eq!(d.code, "NX-E0002", "{source}");
        assert_eq!(d.parser_context.as_deref(), Some(context), "{source}");
        assert_eq!(d.expected_tokens, expected, "{source}");
        d.validate("first.nx", source).unwrap();
    }
    // The former mode-first Postfix rows have no B counterpart (§2.2.3).
}

#[test]
fn b_with_clause_truncations_keep_inner_productions() {
    // F0-D15 §§3.1, 6.2: each expected set is literal, independent of the parser.
    let rows: &[(&str, &str, &[&str], u64)] = &[
        (
            "fn Main() with {",
            "grammar.OperationList",
            &["call", "cancel", "copy", "create", "drop", "resume", "}"],
            15,
        ),
        ("fn Main() with {call", "grammar.OperationLimit", &["("], 16),
        (
            "fn Main() with {call(",
            "grammar.RestrictionItem",
            &[
                "no_alloc",
                "no_block",
                "no_heap_alloc",
                "no_io",
                "no_suspend",
                "unrestricted",
            ],
            21,
        ),
        (
            "fn Main() with {call(no_io",
            "grammar.OperationLimit",
            &[")", ","],
            16,
        ),
        (
            "fn Main() with {call(no_io)",
            "grammar.OperationList",
            &[",", "}"],
            15,
        ),
        (
            "fn Main() with {call(no_io),",
            "grammar.Operation",
            &["call", "cancel", "copy", "create", "drop", "resume"],
            28,
        ),
        ("fn Main() with {call(no_io)}", "function_suffix", &["{"], 0),
        (
            "fn Main() with no_io,",
            "grammar.RestrictionItem",
            &[
                "no_alloc",
                "no_block",
                "no_heap_alloc",
                "no_io",
                "no_suspend",
                "unrestricted",
            ],
            15,
        ),
    ];
    for &(source, context, expected, prefix_start) in rows {
        let d = parse("with-trunc.nx", source).unwrap_err();
        let end = source.len() as u64;
        assert_eq!(d.code, "NX-E0002", "{source}");
        assert_eq!(d.parser_context.as_deref(), Some(context), "{source}");
        assert_eq!(d.expected_tokens, expected, "{source}");
        assert_eq!(
            d.expected_tokens_kind,
            Some(ExpectedTokensKind::AdmissibleSet)
        );
        assert_eq!(
            d.primary_span.as_ref().map(|s| (s.start, s.end)),
            Some((end, end))
        );
        let token = d.unexpected_token.as_ref().unwrap();
        assert_eq!(
            (token.kind, token.text.as_deref()),
            (TokenKind::Eof, Some(""))
        );
        assert_eq!(
            d.recognized_prefix.as_ref().map(|s| (s.start, s.end)),
            Some((prefix_start, end)),
            "{source}"
        );
        d.validate("with-trunc.nx", source).unwrap();
    }
}

#[test]
fn b_nested_place_roots_and_program_alternatives_keep_inner_first() {
    let valid = "fn Main(){let a=[0];((a))[0]=1;}";
    let ast = parse("root.nx", valid).unwrap();
    assert!(
        matches!(&ast.functions[0].body.statements[1], Statement::Assign { place: Expr::Index { target, .. }, .. } if matches!(&**target, Expr::Group(_, _)))
    );
    for (source, context, expected, prefix_start) in [
        ("fn Main(){((a", "grammar.Primary", &[")"][..], 11),
        ("fn Main(){((a)", "grammar.Primary", &[")"][..], 10),
        ("fn Main(){((a))[0", "grammar.Index", &["]"][..], 15),
    ] {
        let d = parse("root.nx", source).unwrap_err();
        assert_eq!(d.code, "NX-E0002", "{source}");
        assert_eq!(d.parser_context.as_deref(), Some(context));
        assert_eq!(d.expected_tokens, expected);
        assert_eq!(
            d.expected_tokens_kind,
            Some(ExpectedTokensKind::AdmissibleSet)
        );
        let end = source.len() as u64;
        assert_eq!(
            d.primary_span.as_ref().map(|s| (s.start, s.end)),
            Some((end, end))
        );
        assert_eq!(
            d.unexpected_token
                .as_ref()
                .map(|t| (t.kind, t.text.as_deref())),
            Some((TokenKind::Eof, Some("")))
        );
        assert_eq!(
            d.recognized_prefix.as_ref().map(|s| (s.start, s.end)),
            Some((prefix_start, end))
        );
        d.validate("root.nx", source).unwrap();
    }
    for (source, marker, kind, text) in [
        (
            "fn A(){} fn B(){} let x=1;",
            "let",
            TokenKind::Keyword,
            "let",
        ),
        ("fn A(){} fn B(){} ;", ";", TokenKind::Symbol, ";"),
        (
            "fn A(){} fn B(){} value:1;",
            "value",
            TokenKind::Name,
            "value",
        ),
    ] {
        let d = parse("root.nx", source).unwrap_err();
        assert_eq!(d.parser_context.as_deref(), Some("module_item"));
        assert_eq!(d.expected_tokens, ["EOF", "fn"]);
        let start = byte_span(source, marker, 0).0;
        assert_eq!(
            d.unexpected_token
                .as_ref()
                .map(|t| (t.kind, t.text.as_deref())),
            Some((kind, Some(text)))
        );
        assert_eq!(d.primary_span.as_ref().map(|s| s.start), Some(start));
        assert_eq!(
            d.recognized_prefix.as_ref().map(|s| (s.start, s.end)),
            Some((0, start))
        );
        d.validate("root.nx", source).unwrap();
    }
}

#[test]
fn b_excluded_operator_prefix_tracks_active_statement_across_terminators_and_blocks() {
    fn case(n: usize, op: &str) -> (String, usize, usize) {
        let mut source = String::from("fn F(){");
        match n % 4 {
            1 => source.push_str("if true{"),
            2 => source.push_str("while true{"),
            3 => source.push_str("if true{}else{"),
            _ => (),
        }
        if n % 5 == 3 {
            source.push_str("let z=if true then {");
        }
        if n % 5 == 1 {
            source.push_str("var b=0;");
        }
        source.push_str(match (n / 20) % 6 {
            0 => "let a=1\n",
            1 => "let a=1\r\n",
            2 => "let a=1\r",
            3 => "let a=1;",
            4 => "let a=1;//comment\n",
            _ => "let a=1;\r\n//comment\r",
        });
        let start = source.len();
        source.push_str(match n % 5 {
            0 => "let b=2 ",
            1 => "b=2 ",
            2 => "return 2 ",
            3 => "value:2 ",
            _ => "2 ",
        });
        let operator = source.len();
        source.push_str(op);
        source.push_str(" 3;");
        if n % 5 == 3 {
            source.push_str("}else{value:4;};");
        }
        if !n.is_multiple_of(4) {
            source.push('}');
        }
        source.push('}');
        (source, start, operator)
    }
    for n in 0..5_000 {
        let op = ["*", "!=", ">", "<=", ">=", "..", "+="][n % 7];
        let (source, start, operator) = case(n, op);
        let d = parse("prefix.nx", &source).unwrap_err();
        assert_eq!(
            d.primary_span
                .as_ref()
                .map(|s| (s.start as usize, s.end as usize)),
            Some((operator, operator + op.len())),
            "{source}"
        );
        if op == "+=" {
            assert_eq!(d.code, "NX-E0006", "{source}");
            assert_eq!(d.feature_id.as_deref(), Some("compound_assignment"));
        } else {
            assert_eq!(d.code, "NX-E0002", "{source}");
            assert_eq!(d.stage, Stage::Parse);
            assert_eq!(
                d.parser_context.as_deref(),
                Some(if op == ".." {
                    "range_endpoint"
                } else {
                    "unsupported_binary_operator"
                })
            );
            assert_eq!(
                d.expected_tokens,
                if op == ".." {
                    &["..<", "..="][..]
                } else {
                    &["+", "-", "<", "=="][..]
                }
            );
            let prefix_start = if op == ".." {
                source[..operator].rfind('2').unwrap()
            } else {
                start
            };
            assert_eq!(
                d.recognized_prefix
                    .as_ref()
                    .map(|s| (s.start as usize, s.end as usize)),
                Some((prefix_start, operator)),
                "{source}"
            );
            assert!(d.repair.is_empty());
        }
        d.validate("prefix.nx", &source).unwrap();
    }
}

#[test]
fn b_excluded_operator_after_nested_statement_keeps_outer_prefix() {
    // The two literal offsets are byte positions, counted from the beginning
    // of each source. In particular, the nested value statement starts later.
    for (source, operator, outer) in [
        (
            "fn F(){\nlet x = (if c then {\nvalue: 1\n} else {\nvalue: 2\n}) * 3;\n}",
            "*",
            "let x",
        ),
        (
            "fn F(){\nlet x = 1\nlet y = (if c then {\nvalue: 1;\n} else {\nvalue: 2;\n}) != 3;\n}",
            "!=",
            "let y",
        ),
    ] {
        let token = byte_span(source, operator, 0);
        let prefix = (byte_span(source, outer, 0).0, token.0);
        let d = parse("nested-prefix.nx", source).unwrap_err();
        assert_eq!(d.code, "NX-E0002", "{source}");
        assert_eq!(
            d.parser_context.as_deref(),
            Some("unsupported_binary_operator")
        );
        assert_eq!(d.expected_tokens, ["+", "-", "<", "=="]);
        assert_eq!(
            (
                d.primary_span.as_ref().unwrap().start,
                d.primary_span.as_ref().unwrap().end
            ),
            token,
            "{source}"
        );
        assert_eq!(
            d.recognized_prefix.as_ref().map(|s| (s.start, s.end)),
            Some(prefix),
            "{source}"
        );
        assert_eq!(
            d.unexpected_token.as_ref().and_then(|t| t.text.as_deref()),
            Some(&source[token.0 as usize..token.1 as usize])
        );
        d.validate("nested-prefix.nx", source).unwrap();
    }

    let operators = ["*", "!=", ">", "<=", ">=", "..", "+="];
    let newlines = ["\n", "\r\n", "\r"];
    for (context, prefix, suffix) in [
        ("let", "let result = ", ";"),
        ("assign", "result = ", ";"),
        ("return", "return ", ";"),
        ("value", "value: ", ";"),
        ("expr", "", ";"),
    ] {
        for (shape, inner) in [
            (
                "if-expression",
                "let inner=if d then {value:1;}else{value:2;};",
            ),
            ("if-statement", "if d{let inner=1;}else{let inner=2;}"),
            ("while", "while d{let inner=1;}"),
            ("block", "{let inner=1;}"),
        ] {
            for depth in 1..=3 {
                for newline in newlines {
                    for op in operators {
                        let mut source = String::from("fn F(){");
                        source.push_str(newline);
                        if context == "assign" {
                            source.push_str("var result=0;");
                            source.push_str(newline);
                        }
                        if context == "value" {
                            source.push_str("let outer=if ready then {");
                            source.push_str(newline);
                        }
                        let outer_start = source.len();
                        source.push_str(prefix);
                        source.push_str("(if c then {");
                        source.push_str(newline);
                        for _ in 1..depth {
                            source.push('{');
                        }
                        source.push_str(inner);
                        for _ in 1..depth {
                            source.push('}');
                        }
                        source.push_str("// nested statement");
                        source.push_str(newline);
                        source.push_str("value: 1;");
                        source.push_str(newline);
                        source.push_str("} else {");
                        source.push_str(newline);
                        source.push_str("value: 2;");
                        source.push_str(newline);
                        source.push_str("}) ");
                        let operator = source.len();
                        source.push_str(op);
                        source.push_str(" 3");
                        source.push_str(suffix);
                        source.push_str(newline);
                        if context == "value" {
                            source.push_str("}else{value:0;};");
                        }
                        source.push('}');

                        let d = parse("nested-prefix.nx", &source).unwrap_err();
                        assert_eq!(
                            (
                                d.primary_span.as_ref().unwrap().start as usize,
                                d.primary_span.as_ref().unwrap().end as usize
                            ),
                            (operator, operator + op.len()),
                            "{context}/{shape}/{depth}/{newline:?}/{op}: {source}: {d:?}"
                        );
                        if op == "+=" {
                            assert_eq!(d.code, "NX-E0006", "{source}");
                            assert_eq!(
                                d.feature_id.as_deref(),
                                Some("compound_assignment"),
                                "{source}"
                            );
                        } else {
                            assert_eq!(d.code, "NX-E0002", "{source}");
                            assert_eq!(
                                d.unexpected_token.as_ref().and_then(|t| t.text.as_deref()),
                                Some(op),
                                "{source}"
                            );
                            assert_eq!(
                                d.parser_context.as_deref(),
                                Some(if op == ".." {
                                    "range_endpoint"
                                } else {
                                    "unsupported_binary_operator"
                                }),
                                "{source}"
                            );
                            assert_eq!(
                                d.expected_tokens,
                                if op == ".." {
                                    &["..<", "..="][..]
                                } else {
                                    &["+", "-", "<", "=="][..]
                                },
                                "{source}"
                            );
                            let prefix_start = if op == ".." {
                                source.find("(if c then {").unwrap()
                            } else {
                                outer_start
                            };
                            assert_eq!(
                                d.recognized_prefix
                                    .as_ref()
                                    .map(|s| (s.start as usize, s.end as usize)),
                                Some((prefix_start, operator)),
                                "{source}"
                            );
                        }
                        d.validate("nested-prefix.nx", &source).unwrap();
                    }
                }
            }
        }
    }
}

#[test]
fn b_generated_nested_statements_restore_the_outer_operator_prefix() {
    // All expected byte offsets come from the generator's own concatenation
    // points, before the parser sees the source. A fixed LCG makes failures
    // reproducible while varying contexts independently of source length.
    let mut state = 0x2d35_8dcc_a3b7_9461_u64;
    let mut choose = |bound: usize| {
        state = state
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        ((state >> 32) as usize) % bound
    };
    for case in 0..3_500 {
        let context = choose(5);
        let depth = choose(3) + 1;
        let newline = ["\n", "\r\n", "\r"][choose(3)];
        let op = ["*", "!=", ">", "<=", ">="][choose(5)];
        let nested = [
            "if flag{let n=1;}else{let n=2;}",
            "while flag{let n=1;}",
            "{let n=1;}",
            "let n=if flag then {value:1;}else{value:2;};",
        ][choose(4)];
        let mut source = String::from("fn F(){");
        source.push_str(newline);
        if context == 1 {
            source.push_str("var out=0;");
            source.push_str(newline);
        }
        if context == 3 {
            source.push_str("let out=if ready then {");
            source.push_str(newline);
        }
        let outer_start = source.len();
        source.push_str(["let out=", "out=", "return ", "value: ", ""][context]);
        source.push_str("(if ready then {");
        source.push_str(newline);
        for _ in 1..depth {
            source.push('{');
        }
        source.push_str(nested);
        for _ in 1..depth {
            source.push('}');
        }
        if choose(2) == 0 {
            source.push_str("// comment");
            source.push_str(newline);
        }
        source.push_str("value:1;");
        source.push_str(newline);
        source.push_str("}else{value:2;}) ");
        let operator_start = source.len();
        source.push_str(op);
        source.push_str(" 3;");
        source.push_str(newline);
        if context == 3 {
            source.push_str("}else{value:0;};");
        }
        source.push('}');
        let d = parse("generated-prefix.nx", &source).unwrap_err();
        assert_eq!(d.code, "NX-E0002", "case {case}: {source}");
        assert_eq!(
            d.parser_context.as_deref(),
            Some("unsupported_binary_operator")
        );
        assert_eq!(d.expected_tokens, ["+", "-", "<", "=="]);
        assert_eq!(
            (
                d.primary_span.as_ref().unwrap().start as usize,
                d.primary_span.as_ref().unwrap().end as usize
            ),
            (operator_start, operator_start + op.len()),
            "case {case}: {source}"
        );
        assert_eq!(
            d.recognized_prefix
                .as_ref()
                .map(|s| (s.start as usize, s.end as usize)),
            Some((outer_start, operator_start)),
            "case {case}: {source}"
        );
        assert_eq!(
            d.unexpected_token.as_ref().and_then(|t| t.text.as_deref()),
            Some(op),
            "case {case}: {source}"
        );
        d.validate("generated-prefix.nx", &source).unwrap();
    }
}

#[test]
fn b_terminator_statement_cases_have_isolated_crlf_and_repair_pairs() {
    // Offsets are literal byte positions in the sources built below.
    let cases = [
        ("fn Main(){", "let x=1", 17, 16),
        ("fn Main(){", "x=1", 13, 12),
        ("fn Main()->I64{", "return 1", 23, 22),
        ("fn Main(){", "f()", 13, 10),
    ];
    for (prefix, statement, end, expression_start) in cases {
        let negative = format!("{prefix}{statement}}}");
        let d = parse("terminator.nx", &negative).unwrap_err();
        assert_eq!(d.code, "NX-E0005", "{statement}");
        assert_eq!(d.stage, Stage::Parse);
        assert_eq!(d.operation, Operation::SyntaxParse);
        assert_eq!(d.parser_context.as_deref(), Some("statement_terminator"));
        assert_eq!(d.expected_tokens, [";"]);
        assert_eq!(
            d.expected_tokens_kind,
            Some(ExpectedTokensKind::CanonicalInsertion)
        );
        assert_eq!(
            *d.primary_span.as_ref().unwrap(),
            Span {
                file: "terminator.nx".into(),
                start: end,
                end: end + 1
            }
        );
        let unexpected = d.unexpected_token.as_ref().unwrap();
        assert_eq!(unexpected.kind, TokenKind::Symbol);
        assert_eq!(unexpected.text.as_deref(), Some("}"));
        assert_eq!(unexpected.span, *d.primary_span.as_ref().unwrap());
        assert_eq!(
            d.recognized_prefix.as_ref().unwrap(),
            &Span {
                file: "terminator.nx".into(),
                start: prefix.len() as u64,
                end
            }
        );
        assert_eq!(d.related_spans[0].span.start, expression_start);
        assert_eq!(d.related_spans[0].span.end, end);
        let edit = &d.repair[0].edits[0];
        assert_eq!(
            edit.span,
            Span {
                file: "terminator.nx".into(),
                start: end,
                end
            }
        );
        assert_eq!(edit.expected_text, "");
        assert_eq!(edit.replacement, ";");
        d.validate("terminator.nx", &negative).unwrap();

        let explicit = format!("{prefix}{statement};}}");
        let positive = parse("terminator.nx", &explicit).unwrap();
        assert_eq!(positive.functions[0].body.statements.len(), 1);
        for newline in ["\n", "\r\n", "\r"] {
            let source = format!("{prefix}{statement}{newline}let z=2;}}");
            let inserted = parse("terminator.nx", &source).unwrap();
            assert_eq!(inserted.functions[0].body.statements.len(), 2);
            let first = &inserted.functions[0].body.statements[0];
            let first_end = match first {
                Statement::Let { span, .. }
                | Statement::Assign { span, .. }
                | Statement::Return { span, .. }
                | Statement::Value { span, .. }
                | Statement::Expr { span, .. } => span.end,
                _ => panic!("unexpected statement for {statement}"),
            };
            assert_eq!(first_end, end, "{statement} {newline:?}");
            assert!(matches!(
                &inserted.functions[0].body.statements[1],
                Statement::Let { span, .. } if span.start == end + newline.len() as u64
            ));
        }
        let eof_source = format!("{prefix}{statement}");
        let eof = parse("terminator.nx", &eof_source).unwrap_err();
        assert_eq!(eof.code, "NX-E0005", "{statement}");
        assert_eq!(eof.parser_context.as_deref(), Some("statement_terminator"));
        assert_eq!(eof.expected_tokens, [";"]);
        assert_eq!(
            eof.expected_tokens_kind,
            Some(ExpectedTokensKind::CanonicalInsertion)
        );
        assert_eq!(
            *eof.primary_span.as_ref().unwrap(),
            Span {
                file: "terminator.nx".into(),
                start: end,
                end
            }
        );
        assert_eq!(eof.unexpected_token.as_ref().unwrap().kind, TokenKind::Eof);
        assert_eq!(
            eof.unexpected_token.as_ref().unwrap().text.as_deref(),
            Some("")
        );
        assert_eq!(
            eof.recognized_prefix.as_ref().unwrap(),
            &Span {
                file: "terminator.nx".into(),
                start: prefix.len() as u64,
                end
            }
        );
        assert_eq!(eof.repair[0].edits[0].span.start, end);
        eof.validate("terminator.nx", &eof_source).unwrap();
    }
}

#[test]
fn b_related_locations_use_first_duplicate_and_full_crlf_delimiter() {
    for (source, subcause, first) in [
        (
            "fn F() with {call(no_io),call(no_alloc)}{}",
            "duplicate_operation",
            (13, 17),
        ),
        (
            "fn F() with no_io,no_io{}",
            "duplicate_restriction",
            (12, 17),
        ),
    ] {
        let d = parse("related.nx", source).unwrap_err();
        assert_eq!(d.subcause.as_deref(), Some(subcause));
        assert_eq!(d.related_spans[0].role, RelatedRole::Context);
        assert_eq!(
            (d.related_spans[0].span.start, d.related_spans[0].span.end),
            first
        );
    }
    let source = "fn F(){let x=if true then {value:\r\n}else{value:1;};}";
    let d = parse("related.nx", source).unwrap_err();
    assert_eq!(d.subcause.as_deref(), Some("missing_value_operand"));
    let offset = source.find("\r\n").unwrap() as u64;
    assert!(
        d.related_spans
            .iter()
            .any(|r| r.role == RelatedRole::Delimiter
                && (r.span.start, r.span.end) == (offset, offset + 2))
    );
    let split_source = "fn F(){let x=if true then {value\r:1;}else{value:2;};}";
    let d = parse("related.nx", split_source).unwrap_err();
    assert_eq!(d.code, "NX-E0002");
    assert_eq!(d.feature_id, None);
    let colon = byte_span(split_source, ":1", 0).0;
    assert_eq!(
        d.primary_span.as_ref().map(|s| (s.start, s.end)),
        Some((colon, colon + 1))
    );
    d.validate("related.nx", split_source).unwrap();
}

#[test]
fn b_logical_syntax_entries_have_boundaries_in_each_operand_position() {
    let cases = [
        ("assignment_rhs", "fn F(){var x=0;x=", ";}", 126),
        ("return_operand", "fn F()->I64{return ", ";}", 126),
        ("if_condition", "fn F(){if ", "{}}", 126),
        ("call_argument", "fn F(){f(", ");}", 126),
        ("index_operand", "fn F(){a[", "];}", 126),
        (
            "value_operand",
            "fn F(){let x=if true then {value:",
            ";}else{value:0;};}",
            124,
        ),
    ];
    for (name, prefix, suffix, admitted) in cases {
        for n in [admitted - 1, admitted] {
            let source = format!("{prefix}{}1{}{suffix}", "(".repeat(n), ")".repeat(n));
            parse("entry.nx", &source).unwrap_or_else(|d| panic!("{name} {n}: {d:?}"));
        }
        let n = admitted + 1;
        let source = format!("{prefix}{}1{}{suffix}", "(".repeat(n), ")".repeat(n));
        let d = parse("entry.nx", &source).unwrap_err();
        assert_eq!(d.code, "NX-E0003", "{name}");
        assert_eq!(
            d.data.get("budget_kind"),
            Some(&DataValue::String("syntax_nesting".into())),
            "{name}"
        );
        assert_eq!(d.data.get("limit"), Some(&DataValue::UInt(128)), "{name}");
        assert_eq!(
            d.data.get("observed"),
            Some(&DataValue::UInt(129)),
            "{name}"
        );
        assert_eq!(
            d.primary_span.as_ref().unwrap().start,
            (prefix.len() + n) as u64,
            "{name}"
        );
        assert_eq!(
            d.primary_span.as_ref().unwrap().end,
            (prefix.len() + n + 1) as u64,
            "{name}"
        );
        d.validate("entry.nx", &source).unwrap();
    }
    for n in [126, 127] {
        parse(
            "entry.nx",
            &format!(
                "fn F(x:view {}I64{}){{}}",
                "Result<".repeat(n),
                ",I64>".repeat(n)
            ),
        )
        .unwrap();
    }
    let source = format!(
        "fn F(x:view {}I64{}){{}}",
        "Result<".repeat(128),
        ",I64>".repeat(128)
    );
    let d = parse("entry.nx", &source).unwrap_err();
    assert_eq!(
        d.data.get("budget_kind"),
        Some(&DataValue::String("syntax_nesting".into()))
    );
    assert_eq!(
        d.primary_span.as_ref().unwrap().start,
        byte_span(&source, "I64", 0).0
    );
    d.validate("entry.nx", &source).unwrap();
}

#[test]
fn b_expression_height_counts_composition_without_precedence_entries() {
    for additions in [126, 127] {
        parse(
            "height.nx",
            &format!("fn F(){{let x=1{};}}", "+1".repeat(additions)),
        )
        .unwrap();
    }
    let source = format!("fn F(){{let x=1{};}}", "+1".repeat(128));
    let d = parse("height.nx", &source).unwrap_err();
    assert_eq!(d.code, "NX-E0003");
    assert_eq!(
        d.data.get("budget_kind"),
        Some(&DataValue::String("expression_tree_depth".into()))
    );
    assert_eq!(d.data.get("limit"), Some(&DataValue::UInt(128)));
    assert_eq!(d.data.get("observed"), Some(&DataValue::UInt(129)));
    assert_eq!(d.primary_span.as_ref().unwrap().start, 13);
    assert_eq!(d.primary_span.as_ref().unwrap().end, 270);
    d.validate("height.nx", &source).unwrap();
    parse(
        "height.nx",
        &format!("fn F(){{let x=1{};}}", "+1".repeat(100)),
    )
    .unwrap();
    parse(
        "height.nx",
        &format!("fn F(){{let x=1<{}2{};}}", "(".repeat(126), ")".repeat(126)),
    )
    .unwrap();
    for (prefix, part, suffix) in [("fn F(){a", "[1]", ";}"), ("fn F(){a", "()", ";}")] {
        for n in [126, 127] {
            parse("height.nx", &format!("{prefix}{}{suffix}", part.repeat(n))).unwrap();
        }
        let source = format!("{prefix}{}{suffix}", part.repeat(128));
        let d = parse("height.nx", &source).unwrap_err();
        assert_eq!(d.code, "NX-E0003");
        assert_eq!(
            d.data.get("budget_kind"),
            Some(&DataValue::String("expression_tree_depth".into()))
        );
        assert_eq!(d.data.get("limit"), Some(&DataValue::UInt(128)));
        assert_eq!(d.data.get("observed"), Some(&DataValue::UInt(129)));
        assert_eq!(d.primary_span.as_ref().unwrap().start, 7);
        assert_eq!(
            d.primary_span.as_ref().unwrap().end,
            (prefix.len() + part.len() * 128) as u64
        );
        d.validate("height.nx", &source).unwrap();
    }
    parse("height.nx", "fn F(){a?;}").unwrap();
    assert_eq!(
        parse("height.nx", "fn F(){a??;}").unwrap_err().code,
        "NX-E0002"
    );
    for n in [126, 127] {
        parse(
            "height.nx",
            &format!("fn F(){{{}{{}}}}", "if true{}else ".repeat(n)),
        )
        .unwrap();
    }
    let source = format!("fn F(){{{}{{}}}}", "if true{}else ".repeat(128));
    let d = parse("height.nx", &source).unwrap_err();
    assert_eq!(
        d.data.get("budget_kind"),
        Some(&DataValue::String("expression_tree_depth".into()))
    );
    assert_eq!(d.data.get("observed"), Some(&DataValue::UInt(129)));
    d.validate("height.nx", &source).unwrap();
    parse(
        "height.nx",
        &format!("fn F(){{let x=[1{}];}}", "+1".repeat(126)),
    )
    .unwrap();
    let source = format!("fn F(){{let x=[1{}];}}", "+1".repeat(127));
    let d = parse("height.nx", &source).unwrap_err();
    assert_eq!(
        d.data.get("budget_kind"),
        Some(&DataValue::String("expression_tree_depth".into()))
    );
    assert_eq!(d.data.get("observed"), Some(&DataValue::UInt(129)));
    assert_eq!(d.primary_span.as_ref().unwrap().start, 13);
    assert_eq!(d.primary_span.as_ref().unwrap().end, 270);
    d.validate("height.nx", &source).unwrap();
}

#[test]
fn b_structural_budgets_have_literal_boundaries_and_diagnostic_fields() {
    let check =
        |source: String, kind: &str, limit: u64, observed: u64, start: usize, end: usize| {
            let d = parse("budget.nx", &source).unwrap_err();
            assert_eq!(d.code, "NX-E0003", "{kind}");
            assert_eq!(
                d.stage,
                if kind == "source_bytes" {
                    Stage::Input
                } else {
                    Stage::Parse
                }
            );
            assert_eq!(d.operation, Operation::AnalysisLimit);
            assert_eq!(
                d.primary_span.as_ref().unwrap(),
                &Span {
                    file: "budget.nx".into(),
                    start: start as u64,
                    end: end as u64
                }
            );
            assert_eq!(
                d.data.get("budget_kind"),
                Some(&DataValue::String(kind.into()))
            );
            assert_eq!(d.data.get("limit"), Some(&DataValue::UInt(limit)));
            assert_eq!(d.data.get("observed"), Some(&DataValue::UInt(observed)));
            assert!(d.unexpected_token.is_none());
            assert!(d.expected_tokens.is_empty());
            assert!(d.parser_context.is_none());
            d.validate("budget.nx", &source).unwrap();
        };
    for n in [127, 128] {
        parse(
            "budget.nx",
            &format!("fn F(){{{}{}}}", "{".repeat(n - 1), "}".repeat(n - 1)),
        )
        .unwrap();
    }
    check(
        format!("fn F(){{{}{}}}", "{".repeat(128), "}".repeat(128)),
        "syntax_nesting",
        128,
        129,
        134,
        135,
    );
    for n in [127, 128] {
        parse(
            "budget.nx",
            &format!(
                "fn F(x:view {}I64{}){{}}",
                "Array<".repeat(n - 1),
                ">".repeat(n - 1)
            ),
        )
        .unwrap();
    }
    check(
        format!(
            "fn F(x:view {}I64{}){{}}",
            "Array<".repeat(128),
            ">".repeat(128)
        ),
        "syntax_nesting",
        128,
        129,
        780,
        783,
    );
    for n in [125, 126] {
        parse(
            "budget.nx",
            &format!("fn F(){{let x={}1{};}}", "[".repeat(n), "]".repeat(n)),
        )
        .unwrap();
    }
    check(
        format!("fn F(){{let x={}1{};}}", "[".repeat(127), "]".repeat(127)),
        "syntax_nesting",
        128,
        129,
        140,
        141,
    );
    for n in [125, 126] {
        parse(
            "budget.nx",
            &format!("fn F(){{let x={}1{};}}", "(".repeat(n), ")".repeat(n)),
        )
        .unwrap();
    }
    check(
        format!("fn F(){{let x={}1{};}}", "(".repeat(127), ")".repeat(127)),
        "syntax_nesting",
        128,
        129,
        140,
        141,
    );
    let rhs = format!(
        "fn Main(){{var x=0;x={}[1{};}}",
        "[".repeat(200),
        "]".repeat(200)
    );
    check(rhs, "syntax_nesting", 128, 129, 147, 148);
    for count in [1_023, 1_024] {
        parse("budget.nx", &"fn F(){}".repeat(count)).unwrap();
    }
    let functions = "fn F(){}".repeat(1_025);
    check(functions, "function_count", 1_024, 1_025, 8_195, 8_196);
    let incomplete = format!("{}fn", "fn F(){}".repeat(1_024));
    let d = parse("budget.nx", &incomplete).unwrap_err();
    assert_eq!(d.code, "NX-E0002");
    assert_eq!(d.parser_context.as_deref(), Some("function_name"));
    d.validate("budget.nx", &incomplete).unwrap();
    let bytes = "x".repeat(1_048_577);
    check(
        bytes,
        "source_bytes",
        1_048_576,
        1_048_577,
        1_048_576,
        1_048_577,
    );
    for size in [1_048_575, 1_048_576] {
        let source = format!("fn F(){{}}//{}", "x".repeat(size - 10));
        assert_eq!(source.len(), size);
        parse("budget.nx", &source).unwrap();
    }
}

#[test]
fn b_first_state_is_independent_of_the_offending_token() {
    let prefixes = [
        "fn",
        "fn F(",
        "fn F(x:",
        "fn F(x:view Array<",
        "fn F(x:view Result<I64,",
        "fn F(x:view I64)",
        "fn F(x:view I64)->Array<",
        "fn F()->",
        "fn F() with no_io",
        "fn F() with {call(no_io)}",
        "fn F() with {call(no_io),copy(no_alloc),",
        "fn Main(",
        "fn Main(){",
        "fn Main(){1+",
        "fn Main(){let x=edit (",
        "fn Main(){if true{}else ",
        "fn F() with ",
        "fn F() with {",
        "fn F() with {call",
        "fn F() with {call(",
        "fn F() with {call(no_io",
        "fn F() with {call(no_io)",
        "fn F() with {call(no_io),",
        "fn F() with {copy(",
        "fn F() with {call(no_io),copy",
        "fn F() with {call(no_io),copy(",
        "fn F() with {call(no_io),copy(no_alloc",
        "fn F() with {call(no_io),copy(no_alloc)",
        "fn F() with no_io,",
        "fn F(){let ",
        "fn F(){let x",
        "fn F(){let x:",
        "fn F(){let x:I64",
        "fn F(){var x=(",
        "fn F(){x=",
        "fn F(){x=(",
        "fn F(){a[",
        "fn F(){f(a:",
        "fn F(){let x=move (",
        "fn F(){let x=1+",
        "fn F(){let x=1<",
        "fn F(){if true",
        "fn F(){while true",
        "fn F(){let x=if ",
        "fn F(){while ",
        "fn F(){return (",
        "fn Main(){let a=(",
        "fn Main(){let a=[",
        "fn F(x:view I64,",
        "fn Main(){1;",
        "fn F(x:view I64",
        "fn Main(){let x=",
        "fn Main(){f(",
        "fn Main(){f(1,",
        "fn Main(){let a=[1,",
    ];
    let mut cases = 0;
    let offending = [
        ")",
        "]",
        "}",
        "{",
        "fn",
        "x",
        "1",
        "with",
        "else",
        "let",
        "return",
        "while",
        ",",
        "=",
        ":",
        "->",
        "*",
        ">",
        "+",
        "==",
        "<",
        "?",
        ".",
        "@",
        "\"s\"",
        ";",
        "false",
        "true",
        "[",
        "(",
        "no_io",
        "unrestricted",
    ];
    for prefix in prefixes {
        for padding in 0..100 {
            let source_eof = format!("{prefix}{}", " ".repeat(padding));
            let eof = parse("first.nx", &source_eof).unwrap_err();
            assert_eq!(eof.code, "NX-E0002", "{source_eof}");
            for token in offending {
                let source_bad = format!("{source_eof}{token}");
                if let Err(bad) = parse("first.nx", &source_bad)
                    && bad.code == "NX-E0002"
                    && bad
                        .primary_span
                        .as_ref()
                        .is_some_and(|s| s.start as usize == source_eof.len())
                    && !matches!(
                        bad.parser_context.as_deref(),
                        Some("inout_place" | "consume_place" | "unsupported_binary_operator")
                    )
                {
                    assert_eq!(eof.parser_context, bad.parser_context, "{source_bad}");
                    assert_eq!(eof.expected_tokens, bad.expected_tokens, "{source_bad}");
                }
                cases += 1;
            }
        }
    }
    assert_eq!(cases, 176_000);
}

#[test]
fn b_first_consistency_across_nested_grammar_positions() {
    let offending = [
        ")",
        "]",
        "}",
        "{",
        "fn",
        "x",
        "1",
        "with",
        "else",
        "let",
        "return",
        "while",
        ",",
        "=",
        ":",
        "->",
        "*",
        ">",
        "+",
        "==",
        "<",
        "?",
        ".",
        "@",
        "\"s\"",
        ";",
        "false",
        "true",
        "[",
        "(",
        "no_io",
        "unrestricted",
    ];
    for source in [
        "fn F(x:edit Array<Result<I64,Bool>>,y:move I64)->Result<I64,Bool> with no_io {let a=[1,2];let b=if true then {value:1;}else{value:3;};while false{a[0]=a[1]+1;}return Ok(b);} fn Main(){}",
        "fn F() with {call(no_io,no_block)}{} fn Main(){}",
        "fn F(x:view I64){} fn Main(){var a=[1];(a)[0]=1;F(a[0]);}",
        "fn F(){if true{}else if false{}else{}while false{} {let x=(1);}}",
    ] {
        parse("positions.nx", source).unwrap();
        for token in lex::lex("positions.nx", source)
            .unwrap()
            .into_iter()
            .filter(|t| t.kind != Kind::Eof)
        {
            let prefix = &source[..token.span.start as usize];
            let eof = parse("positions.nx", prefix);
            for spelling in offending {
                let bad = parse("positions.nx", &format!("{prefix}{spelling}"));
                if let (Err(eof), Err(bad)) = (&eof, bad)
                    && eof.code == "NX-E0002"
                    && bad.code == "NX-E0002"
                    && bad
                        .primary_span
                        .as_ref()
                        .is_some_and(|s| s.start as usize == prefix.len())
                    && !matches!(
                        bad.parser_context.as_deref(),
                        Some("inout_place" | "consume_place" | "unsupported_binary_operator")
                    )
                {
                    assert_eq!(eof.parser_context, bad.parser_context, "{prefix}{spelling}");
                    assert_eq!(
                        eof.expected_tokens, bad.expected_tokens,
                        "{prefix}{spelling}"
                    );
                }
            }
        }
    }
}

#[test]
fn b_hostile_chains_return_diagnostics_on_a_small_stack() {
    std::thread::Builder::new()
        .stack_size(2 * 1024 * 1024)
        .spawn(|| {
            for count in [10_000, 200_000, 1_000_000] {
                let sources = [
                    format!("fn F(){{{}{{}}}}", "if true{}else ".repeat(count)),
                    format!("fn F(){{let x=1{};}}", "+1".repeat(count)),
                    format!("fn F(){{a{};}}", "[1]".repeat(count)),
                    format!("fn F(){{a{};}}", "?".repeat(count)),
                    format!(
                        "fn F(){{let x={}1{};}}",
                        "(".repeat(count),
                        ")".repeat(count)
                    ),
                    format!("fn F(){{{}{}}}", "{".repeat(count), "}".repeat(count)),
                    format!(
                        "fn F(){{let x={}1{};}}",
                        "[".repeat(count),
                        "]".repeat(count)
                    ),
                    format!(
                        "fn F(x:view {}I64{}){{}}",
                        "Array<".repeat(count),
                        ">".repeat(count)
                    ),
                    format!("fn F() with {}{{}}", "{".repeat(count)),
                    format!(
                        "fn F(){{{}{}}}",
                        "if true{".repeat(count),
                        "}".repeat(count)
                    ),
                    format!(
                        "fn F(){{let x={}1{};}}",
                        "if true then {value:".repeat(count),
                        "}".repeat(count)
                    ),
                    format!("fn F(){{let x={}a;}}", "move ".repeat(count)),
                    format!("fn F(){{let x={}a;}}", "edit ".repeat(count)),
                ];
                for (pattern, source) in sources.into_iter().enumerate() {
                    let diagnostic = parse("hostile.nx", &source).unwrap_err();
                    if pattern != 8 && (pattern != 3 && pattern < 11 || source.len() > 1_048_576) {
                        assert_eq!(
                            diagnostic.code, "NX-E0003",
                            "pattern {pattern}, count {count}"
                        );
                    } else if pattern == 3 || pattern >= 11 {
                        assert_eq!(
                            diagnostic.code, "NX-E0002",
                            "pattern {pattern}, count {count}"
                        );
                    }
                    diagnostic.validate("hostile.nx", &source).unwrap();
                }
            }
        })
        .unwrap()
        .join()
        .unwrap();
}

#[test]
fn b_grammar_truncations_cover_nested_production_boundaries() {
    // Expected sets are expanded from §3 EBNF and byte-sorted per §13.14.
    // Each row stops at a production boundary; offsets count UTF-8 bytes.
    type Row<'a> = (&'a str, &'a str, &'a [&'a str], u64);
    let rows: [Row<'_>; 55] = [
        ("fn Main", "grammar.Function", &["("], 0),
        ("fn Main()", "function_suffix", &["->", "with", "{"], 0),
        (
            "fn Main() with",
            "grammar.WithClause",
            &[
                "no_alloc",
                "no_block",
                "no_heap_alloc",
                "no_io",
                "no_suspend",
                "unrestricted",
                "{",
            ],
            10,
        ),
        (
            "fn Main() with no_io,",
            "grammar.RestrictionItem",
            &[
                "no_alloc",
                "no_block",
                "no_heap_alloc",
                "no_io",
                "no_suspend",
                "unrestricted",
            ],
            15,
        ),
        ("fn Main() with no_io", "function_suffix", &[",", "{"], 0),
        ("fn Main()->I64", "function_suffix", &["with", "{"], 0),
        ("fn F(x:view I64,", "grammar.Parameter", &["Name"], 16),
        ("fn F(x:edit", "grammar.Type", TYPE_START, 11),
        ("fn F(x:move", "grammar.Type", TYPE_START, 11),
        ("fn F(x", "grammar.Parameter", &[":"], 5),
        ("fn F(x:view Result", "grammar.Type", &["<"], 12),
        (
            "fn F(x:view Result<",
            "grammar.Type",
            &[
                "(",
                "Array",
                "Bool",
                "I64",
                "MockBlockingResource",
                "MockResource",
                "Result",
            ],
            19,
        ),
        ("fn F(x:view Array", "grammar.Type", &["<"], 12),
        ("fn F(x:view Array<I64", "grammar.Type", &[">"], 18),
        ("fn F(x:view Result<I64", "grammar.Type", &[","], 19),
        (
            "fn F(x:view Result<I64,",
            "grammar.Type",
            &[
                "(",
                "Array",
                "Bool",
                "I64",
                "MockBlockingResource",
                "MockResource",
                "Result",
            ],
            23,
        ),
        ("fn F(x:view Result<I64,Bool", "grammar.Type", &[">"], 23),
        ("fn Main(){f(1", "grammar.Arguments", &[")", ","], 11),
        ("fn Main(){f(x:1", "grammar.Arguments", &[")", ","], 11),
        ("fn Main(){let", "binding_name", &["Name"], 10),
        ("fn Main(){var", "binding_name", &["Name"], 10),
        ("fn Main(){let x:I64", "grammar.Binding", &["="], 10),
        (
            "fn Main(){{",
            "block_item",
            &[
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
            ],
            10,
        ),
        (
            "fn Main(){if true{}else if",
            "grammar.Primary",
            &[
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
            ],
            26,
        ),
        (
            "fn Main(){f(x:",
            "grammar.Primary",
            &[
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
            ],
            14,
        ),
        (
            "fn Main(){a[",
            "grammar.Primary",
            &[
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
            ],
            12,
        ),
        (
            "fn Main(){x=",
            "grammar.Primary",
            &[
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
            ],
            12,
        ),
        (
            "fn Main(){let x=(",
            "grammar.Primary",
            &[
                "(",
                ")",
                "-",
                "IntLiteral",
                "Name",
                "[",
                "edit",
                "false",
                "if",
                "move",
                "true",
            ],
            16,
        ),
        (
            "fn Main(){let x=move (",
            "grammar.Primary",
            &[
                "(",
                ")",
                "-",
                "IntLiteral",
                "Name",
                "[",
                "edit",
                "false",
                "if",
                "move",
                "true",
            ],
            21,
        ),
        (
            "fn Main(){let x=edit (",
            "grammar.Primary",
            &[
                "(",
                ")",
                "-",
                "IntLiteral",
                "Name",
                "[",
                "edit",
                "false",
                "if",
                "move",
                "true",
            ],
            21,
        ),
        (
            "fn Main(){let x=1-",
            "grammar.TryExpr",
            &[
                "(",
                "-",
                "IntLiteral",
                "Name",
                "[",
                "edit",
                "false",
                "move",
                "true",
            ],
            18,
        ),
        (
            "fn Main(){let x=1==",
            "grammar.TryExpr",
            &[
                "(",
                "-",
                "IntLiteral",
                "Name",
                "[",
                "edit",
                "false",
                "move",
                "true",
            ],
            19,
        ),
        (
            "fn Main(){while",
            "grammar.Primary",
            &[
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
            ],
            15,
        ),
        (
            "fn Main(){var x:",
            "grammar.Type",
            &[
                "(",
                "Array",
                "Bool",
                "I64",
                "MockBlockingResource",
                "MockResource",
                "Result",
            ],
            16,
        ),
        ("fn Main(){let x=a[1", "grammar.Index", &["]"], 17),
        (
            "fn Main(){let x=[",
            "grammar.Primary",
            &[
                "(",
                "-",
                "IntLiteral",
                "Name",
                "[",
                "]",
                "edit",
                "false",
                "if",
                "move",
                "true",
            ],
            16,
        ),
        (
            "fn Main(){let x=1;",
            "grammar.Block",
            &[
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
            ],
            9,
        ),
        (
            "fn Main(){if true{}else",
            "grammar.IfForm",
            &["if", "{"],
            10,
        ),
        ("fn Main(){if true", "grammar.Block", &["{"], 17),
        ("fn Main(){while true", "grammar.Block", &["{"], 20),
        ("fn Main(){let x", "grammar.Binding", &[":", "="], 10),
        ("fn F(x:view (", "grammar.Type", &[")"], 12),
        (
            "fn F(x:view I64)->",
            "grammar.Type",
            &[
                "(",
                "Array",
                "Bool",
                "I64",
                "MockBlockingResource",
                "MockResource",
                "Result",
            ],
            18,
        ),
        ("fn F(x:view I64", "grammar.Parameters", &[")", ","], 5),
        ("fn F(x:", "grammar.ParamMode", &["edit", "move", "view"], 7),
        (
            "fn Main(){if ",
            "grammar.Primary",
            &[
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
            ],
            13,
        ),
        (
            "fn Main(){while true{let x=1;",
            "grammar.Block",
            &[
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
            ],
            20,
        ),
        (
            "fn Main(){let x=1+",
            "grammar.TryExpr",
            &[
                "(",
                "-",
                "IntLiteral",
                "Name",
                "[",
                "edit",
                "false",
                "move",
                "true",
            ],
            18,
        ),
        (
            "fn Main(){let x=1<",
            "grammar.TryExpr",
            &[
                "(",
                "-",
                "IntLiteral",
                "Name",
                "[",
                "edit",
                "false",
                "move",
                "true",
            ],
            18,
        ),
        (
            "fn Main(){let x=1?+",
            "grammar.TryExpr",
            &[
                "(",
                "-",
                "IntLiteral",
                "Name",
                "[",
                "edit",
                "false",
                "move",
                "true",
            ],
            19,
        ),
        (
            "fn Main(){let x=a[",
            "grammar.Primary",
            &[
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
            ],
            18,
        ),
        (
            "fn Main(){f(1,",
            "grammar.Argument",
            &[
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
            ],
            14,
        ),
        ("fn Main(){let x=(1", "grammar.Primary", &[")"], 16),
        ("fn Main(){let x=[1", "grammar.Primary", &[",", "]"], 16),
        (
            "//é\nfn Main(){let x=1+",
            "grammar.TryExpr",
            &[
                "(",
                "-",
                "IntLiteral",
                "Name",
                "[",
                "edit",
                "false",
                "move",
                "true",
            ],
            23,
        ),
    ];
    for (source, context, expected, prefix_start) in rows {
        let d = parse("trunc.nx", source).unwrap_err();
        assert_eq!(d.code, "NX-E0002", "{source}");
        assert_eq!(d.parser_context.as_deref(), Some(context), "{source}");
        assert_eq!(d.expected_tokens, expected, "{source}");
        assert_eq!(
            d.expected_tokens_kind,
            Some(ExpectedTokensKind::AdmissibleSet)
        );
        assert_eq!(
            *d.primary_span.as_ref().unwrap(),
            Span {
                file: "trunc.nx".into(),
                start: source.len() as u64,
                end: source.len() as u64
            },
            "{source}"
        );
        assert_eq!(
            d.unexpected_token.as_ref().unwrap().kind,
            TokenKind::Eof,
            "{source}: {:?}",
            d.unexpected_token
        );
        assert_eq!(
            d.unexpected_token.as_ref().unwrap().text.as_deref(),
            Some(""),
            "{source}"
        );
        assert_eq!(
            d.recognized_prefix.as_ref().unwrap(),
            &Span {
                file: "trunc.nx".into(),
                start: prefix_start,
                end: source.len() as u64
            },
            "{source}"
        );
        d.validate("trunc.nx", source).unwrap();
    }
}

#[test]
fn b_published_feature_rows_have_isolated_positive_repairs() {
    // Pair for adr_scope_rows_use_literal_b_sources_and_offsets: D15 ?6.5/?8.7.
    let positives: &[(&str, &str)] = &[
        (
            "inferred_non_unit_result",
            "fn F()->I64{return 1;} fn Main(){}",
        ),
        ("float_literal", "fn Main(){let x=0;}"),
        ("range", "fn Main(){let n=1;let x=0+n;}"),
        ("tuple", "fn Main(){let x=(1);}"),
        ("attributes", "fn Main(){}"),
        ("default_argument", "fn F(x:view I64){} fn Main(){F(1);}"),
        ("closure", "fn Main(){let f=();}"),
        ("function_value", "fn F(){} fn Main(){F();}"),
        ("receiver_mode", "fn F(){} fn Main(){F();}"),
        ("partial_transfer", "fn Main(){let a=[1];let x=a[0];}"),
        ("resource_array", "fn Main(){let a=MockOpen(1);}"),
        ("no_heap_alloc", "fn Main() with no_alloc{}"),
        ("match", "fn Main(){if true{}else{}}"),
        ("option", "fn F(x:view Result<I64,I64>){} fn Main(){}"),
        ("user_types", "fn Main(){}"),
        ("traits_generics", "fn Main(){}"),
        ("modules_visibility", "fn Main(){}"),
        ("extensions", "fn Main(){}"),
        (
            "provenance_clauses",
            "fn F(x:view I64)->I64 {return x;} fn Main(){}",
        ),
        ("comptime", "fn Main(){}"),
        ("text", "fn Main(){let x=1;}"),
        ("methods", "fn Main(){let a=[1];Len(a);}"),
        ("compound_assignment", "fn Main(){var x=1;x=x+1;}"),
        ("for_iteration", "fn Main(){while false{}}"),
        ("loop_control", "fn Main(){while false{}}"),
        ("generator", "fn G(){} fn Main(){}"),
        ("async", "fn F(){} fn Main(){}"),
        ("unsupported_type", "fn F(x:view I64){} fn Main(){}"),
        ("unsupported_type", "fn F(x:view Array<I64>){} fn Main(){}"),
        ("unsupported_type", "fn F(x:view I64){} fn Main(){}"),
        ("region", "fn Main(){{}}"),
        ("methods", "fn Main(){let x=0;}"),
    ];
    assert_eq!(positives.len(), 32);
    for &(feature, source) in positives {
        parse("feature.nx", source)
            .unwrap_or_else(|d| panic!("{feature} positive {source}: {d:?}"));
    }
}
