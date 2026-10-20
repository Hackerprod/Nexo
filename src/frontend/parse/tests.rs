use super::*;
use crate::frontend::print::print_canonical;
fn span(d: &Diagnostic) -> &Span {
    d.primary_span.as_ref().unwrap()
}

// §13.14 rows transcribed from the ADR; FIRST sets are byte-sorted and
// keep category names (Name/IntLiteral) rather than concrete spellings.
// The remaining grammar.P contexts are derived from §3 EBNF as required
// by §13.14, rule 3; they are listed as parser call-site cases below.
const PUBLISHED_FIRST_TABLE: &[(&str, &str, &[&str])] = &[
    ("Function.Name", "function_name", &["Name"]),
    (
        "Function.Parameters? EOF",
        "parameter_list",
        &[")", "Name", "consume", "inout"],
    ),
    (
        "Block.Statement* EOF",
        "block_item",
        &[
            "(",
            "-",
            "IntLiteral",
            "Name",
            "[",
            "consume",
            "false",
            "if",
            "inout",
            "let",
            "return",
            "true",
            "var",
            "while",
            "{",
            "}",
        ],
    ),
    (
        "Binding.Expr EOF",
        "binding_initializer",
        &[
            "(",
            "-",
            "IntLiteral",
            "Name",
            "[",
            "consume",
            "false",
            "if",
            "inout",
            "true",
        ],
    ),
    ("IntLiteral sign EOF", "signed_integer", &["IntLiteral"]),
    (
        "Array Expr EOF",
        "array_element",
        &[
            "(",
            "-",
            "IntLiteral",
            "Name",
            "[",
            "consume",
            "false",
            "if",
            "inout",
            "true",
        ],
    ),
    ("Program trailing token", "module_item", &["EOF", "fn"]),
];
#[test]
fn published_first_rows_have_exact_context_tokens_and_byte_spans() {
    type FirstRow<'a> = (&'a str, &'a str, &'a [&'a str], u64, u64, u64, u64);
    let rows: [FirstRow<'_>; 7] = [
        ("fn", "function_name", &["Name"], 2, 2, 0, 2),
        (
            "fn main(",
            "parameter_list",
            &[")", "Name", "consume", "inout"],
            8,
            8,
            7,
            8,
        ),
        (
            "fn main(){",
            "block_item",
            &[
                "(",
                "-",
                "IntLiteral",
                "Name",
                "[",
                "consume",
                "false",
                "if",
                "inout",
                "let",
                "return",
                "true",
                "var",
                "while",
                "{",
                "}",
            ],
            10,
            10,
            9,
            10,
        ),
        (
            "fn main(){let a=",
            "binding_initializer",
            &[
                "(",
                "-",
                "IntLiteral",
                "Name",
                "[",
                "consume",
                "false",
                "if",
                "inout",
                "true",
            ],
            16,
            16,
            10,
            16,
        ),
        (
            "fn main(){let a=-",
            "signed_integer",
            &["IntLiteral"],
            17,
            17,
            16,
            17,
        ),
        (
            "fn main(){let a=[1,",
            "array_element",
            &[
                "(",
                "-",
                "IntLiteral",
                "Name",
                "[",
                "consume",
                "false",
                "if",
                "inout",
                "true",
            ],
            19,
            19,
            16,
            19,
        ),
        (
            "fn main(){let a=1;} garbage",
            "module_item",
            &["EOF", "fn"],
            20,
            27,
            0,
            20,
        ),
    ];
    for (source, context, expected, lo, hi, prefix_lo, prefix_hi) in rows {
        let d = parse("first.nx", source).unwrap_err();
        assert_eq!(d.code, "NX-E0002", "{source}");
        assert_eq!(d.stage, Stage::Parse, "{source}");
        assert_eq!(d.operation, Operation::SyntaxParse, "{source}");
        assert_eq!(d.parser_context.as_deref(), Some(context), "{source}");
        assert_eq!(
            d.expected_tokens,
            expected.iter().map(|x| x.to_string()).collect::<Vec<_>>(),
            "{source}"
        );
        assert_eq!(
            d.expected_tokens_kind,
            Some(ExpectedTokensKind::AdmissibleSet)
        );
        assert_eq!(span(&d).start, lo, "{source}");
        assert_eq!(span(&d).end, hi, "{source}");
        assert_eq!(d.unexpected_token.as_ref().unwrap().span, *span(&d));
        let (token_kind, token_text) = if context == "module_item" {
            (TokenKind::Name, "garbage")
        } else {
            (TokenKind::Eof, "")
        };
        assert_eq!(d.unexpected_token.as_ref().unwrap().kind, token_kind);
        assert_eq!(
            d.unexpected_token.as_ref().unwrap().text.as_deref(),
            Some(token_text)
        );
        assert_eq!(
            d.recognized_prefix.as_ref().unwrap().start,
            prefix_lo,
            "{source}"
        );
        assert_eq!(
            d.recognized_prefix.as_ref().unwrap().end,
            prefix_hi,
            "{source}"
        );
        d.validate("first.nx", source).unwrap();
    }
}

#[test]
fn published_first_table_is_ordered_unique_and_applied_at_each_published_row() {
    for (production, context, expected) in PUBLISHED_FIRST_TABLE {
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
        assert!(
            PUBLISHED_FIRST_TABLE
                .iter()
                .any(|(_, candidate, _)| candidate == context)
        );
    }
}

#[test]
fn grammar_first_truncation_rows_report_exact_sorted_sets_and_byte_prefixes() {
    type Row<'a> = (&'a str, &'a str, &'a [&'a str], u64, u64);
    let rows: [Row<'_>; 10] = [
        ("fn", "function_name", &["Name"], 2, 0),
        (
            "fn main(",
            "parameter_list",
            &[")", "Name", "consume", "inout"],
            8,
            7,
        ),
        (
            "fn f(x:",
            "grammar.Type",
            &[
                "(",
                "Array",
                "MockBlockingResource",
                "MockResource",
                "Result",
                "bool",
                "i64",
            ],
            7,
            7,
        ),
        (
            "fn main(){",
            "block_item",
            &[
                "(",
                "-",
                "IntLiteral",
                "Name",
                "[",
                "consume",
                "false",
                "if",
                "inout",
                "let",
                "return",
                "true",
                "var",
                "while",
                "{",
                "}",
            ],
            10,
            9,
        ),
        (
            "fn main(){let a=",
            "binding_initializer",
            &[
                "(",
                "-",
                "IntLiteral",
                "Name",
                "[",
                "consume",
                "false",
                "if",
                "inout",
                "true",
            ],
            16,
            10,
        ),
        (
            "fn main(){let a=-",
            "signed_integer",
            &["IntLiteral"],
            17,
            16,
        ),
        (
            "fn main(){let a=[1,",
            "array_element",
            &[
                "(",
                "-",
                "IntLiteral",
                "Name",
                "[",
                "consume",
                "false",
                "if",
                "inout",
                "true",
            ],
            19,
            16,
        ),
        (
            "fn main(){let x=1+",
            "grammar.TryExpr",
            &[
                "(",
                "-",
                "IntLiteral",
                "Name",
                "[",
                "consume",
                "false",
                "inout",
                "true",
            ],
            18,
            18,
        ),
        (
            "fn main(){f(",
            "grammar.Arguments",
            &[
                "(",
                ")",
                "-",
                "IntLiteral",
                "Name",
                "[",
                "consume",
                "false",
                "if",
                "inout",
                "true",
            ],
            12,
            11,
        ),
        (
            "fn main(){let x:Array<",
            "grammar.Type",
            &[
                "(",
                "Array",
                "MockBlockingResource",
                "MockResource",
                "Result",
                "bool",
                "i64",
            ],
            22,
            22,
        ),
    ];
    for (source, context, expected, offset, prefix_start) in rows {
        let d = parse("grammar.nx", source).unwrap_err();
        assert_eq!(d.code, "NX-E0002", "{source}");
        assert_eq!(d.parser_context.as_deref(), Some(context), "{source}");
        assert_eq!(d.expected_tokens, expected, "{source}");
        assert_eq!(
            d.expected_tokens_kind,
            Some(ExpectedTokensKind::AdmissibleSet)
        );
        assert_eq!(
            *span(&d),
            Span {
                file: "grammar.nx".into(),
                start: offset,
                end: offset
            },
            "{source}"
        );
        assert_eq!(
            d.unexpected_token.as_ref().unwrap().kind,
            TokenKind::Eof,
            "{source}"
        );
        assert_eq!(
            d.unexpected_token.as_ref().unwrap().text.as_deref(),
            Some("")
        );
        assert_eq!(
            d.recognized_prefix.as_ref().unwrap(),
            &Span {
                file: "grammar.nx".into(),
                start: prefix_start,
                end: offset
            },
            "{source}"
        );
        d.validate("grammar.nx", source).unwrap();
    }
}

#[test]
fn grammar_truncations_cover_nested_production_boundaries() {
    // Expected sets are expanded from §3 EBNF and byte-sorted per §13.14.
    // Each row stops at a production boundary; offsets count UTF-8 bytes.
    type Row<'a> = (&'a str, &'a str, &'a [&'a str], u64, u64);
    let rows: [Row<'_>; 55] = [
        ("fn main", "grammar.Function", &["("], 7, 0),
        ("fn main()", "function_suffix", &["->", "with", "{"], 9, 0),
        (
            "fn main() with",
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
            14,
            10,
        ),
        (
            "fn main() with no_io,",
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
            15,
        ),
        (
            "fn main() with no_io",
            "function_suffix",
            &[",", "{"],
            20,
            0,
        ),
        ("fn main()->i64", "function_suffix", &["with", "{"], 14, 0),
        (
            "fn f(x:i64,",
            "grammar.Parameter",
            &["Name", "consume", "inout"],
            11,
            11,
        ),
        ("fn f(inout", "grammar.Parameter", &["Name"], 10, 5),
        ("fn f(consume", "grammar.Parameter", &["Name"], 12, 5),
        ("fn f(x", "grammar.Parameter", &[":"], 6, 5),
        ("fn f(x:Result", "grammar.Type", &["<"], 13, 7),
        (
            "fn f(x:Result<",
            "grammar.Type",
            &[
                "(",
                "Array",
                "MockBlockingResource",
                "MockResource",
                "Result",
                "bool",
                "i64",
            ],
            14,
            14,
        ),
        ("fn f(x:Array", "grammar.Type", &["<"], 12, 7),
        ("fn f(x:Array<i64", "grammar.Type", &[">"], 16, 13),
        ("fn f(x:Result<i64", "grammar.Type", &[","], 17, 14),
        (
            "fn f(x:Result<i64,",
            "grammar.Type",
            &[
                "(",
                "Array",
                "MockBlockingResource",
                "MockResource",
                "Result",
                "bool",
                "i64",
            ],
            18,
            18,
        ),
        ("fn f(x:Result<i64,bool", "grammar.Type", &[">"], 22, 18),
        ("fn main(){f(1", "grammar.Arguments", &[")", ","], 13, 11),
        ("fn main(){f(x:1", "grammar.Arguments", &[")", ","], 15, 11),
        ("fn main(){let", "binding_name", &["Name"], 13, 10),
        ("fn main(){var", "binding_name", &["Name"], 13, 10),
        ("fn main(){let x:i64", "grammar.Binding", &["="], 19, 10),
        (
            "fn main(){{",
            "block_item",
            &[
                "(",
                "-",
                "IntLiteral",
                "Name",
                "[",
                "consume",
                "false",
                "if",
                "inout",
                "let",
                "return",
                "true",
                "var",
                "while",
                "{",
                "}",
            ],
            11,
            10,
        ),
        (
            "fn main(){if true{}else if",
            "grammar.Primary",
            &[
                "(",
                "-",
                "IntLiteral",
                "Name",
                "[",
                "consume",
                "false",
                "if",
                "inout",
                "true",
            ],
            26,
            26,
        ),
        (
            "fn main(){f(x:",
            "grammar.Primary",
            &[
                "(",
                "-",
                "IntLiteral",
                "Name",
                "[",
                "consume",
                "false",
                "if",
                "inout",
                "true",
            ],
            14,
            14,
        ),
        (
            "fn main(){a[",
            "grammar.Primary",
            &[
                "(",
                "-",
                "IntLiteral",
                "Name",
                "[",
                "consume",
                "false",
                "if",
                "inout",
                "true",
            ],
            12,
            12,
        ),
        (
            "fn main(){x=",
            "grammar.Primary",
            &[
                "(",
                "-",
                "IntLiteral",
                "Name",
                "[",
                "consume",
                "false",
                "if",
                "inout",
                "true",
            ],
            12,
            12,
        ),
        (
            "fn main(){let x=(",
            "grammar.Primary",
            &[
                "(",
                ")",
                "-",
                "IntLiteral",
                "Name",
                "[",
                "consume",
                "false",
                "if",
                "inout",
                "true",
            ],
            17,
            16,
        ),
        (
            "fn main(){let x=consume",
            "grammar.Postfix",
            &["(", "-", "IntLiteral", "Name", "[", "false", "true"],
            23,
            16,
        ),
        (
            "fn main(){let x=inout",
            "grammar.Postfix",
            &["(", "-", "IntLiteral", "Name", "[", "false", "true"],
            21,
            16,
        ),
        (
            "fn main(){let x=1-",
            "grammar.TryExpr",
            &[
                "(",
                "-",
                "IntLiteral",
                "Name",
                "[",
                "consume",
                "false",
                "inout",
                "true",
            ],
            18,
            18,
        ),
        (
            "fn main(){let x=1==",
            "grammar.TryExpr",
            &[
                "(",
                "-",
                "IntLiteral",
                "Name",
                "[",
                "consume",
                "false",
                "inout",
                "true",
            ],
            19,
            19,
        ),
        (
            "fn main(){while",
            "grammar.Primary",
            &[
                "(",
                "-",
                "IntLiteral",
                "Name",
                "[",
                "consume",
                "false",
                "if",
                "inout",
                "true",
            ],
            15,
            15,
        ),
        (
            "fn main(){var x:",
            "grammar.Type",
            &[
                "(",
                "Array",
                "MockBlockingResource",
                "MockResource",
                "Result",
                "bool",
                "i64",
            ],
            16,
            16,
        ),
        ("fn main(){let x=a[1", "grammar.Index", &["]"], 19, 17),
        (
            "fn main(){let x=[",
            "grammar.Primary",
            &[
                "(",
                "-",
                "IntLiteral",
                "Name",
                "[",
                "]",
                "consume",
                "false",
                "if",
                "inout",
                "true",
            ],
            17,
            16,
        ),
        (
            "fn main(){let x=1;",
            "grammar.Block",
            &[
                "(",
                "-",
                "IntLiteral",
                "Name",
                "[",
                "consume",
                "false",
                "if",
                "inout",
                "let",
                "return",
                "true",
                "var",
                "while",
                "{",
                "}",
            ],
            18,
            9,
        ),
        (
            "fn main(){if true{}else",
            "grammar.IfForm",
            &["if", "{"],
            23,
            10,
        ),
        ("fn main(){if true", "grammar.Block", &["{"], 17, 17),
        ("fn main(){while true", "grammar.Block", &["{"], 20, 20),
        ("fn main(){let x", "grammar.Binding", &[":", "="], 15, 10),
        ("fn f(x:(", "grammar.Type", &[")"], 8, 7),
        (
            "fn f(x:i64)->",
            "grammar.Type",
            &[
                "(",
                "Array",
                "MockBlockingResource",
                "MockResource",
                "Result",
                "bool",
                "i64",
            ],
            13,
            13,
        ),
        ("fn f(x:i64", "grammar.Parameters", &[")", ","], 10, 5),
        (
            "fn f(x:",
            "grammar.Type",
            &[
                "(",
                "Array",
                "MockBlockingResource",
                "MockResource",
                "Result",
                "bool",
                "i64",
            ],
            7,
            7,
        ),
        (
            "fn main(){if ",
            "grammar.Primary",
            &[
                "(",
                "-",
                "IntLiteral",
                "Name",
                "[",
                "consume",
                "false",
                "if",
                "inout",
                "true",
            ],
            13,
            13,
        ),
        (
            "fn main(){while true{let x=1;",
            "grammar.Block",
            &[
                "(",
                "-",
                "IntLiteral",
                "Name",
                "[",
                "consume",
                "false",
                "if",
                "inout",
                "let",
                "return",
                "true",
                "var",
                "while",
                "{",
                "}",
            ],
            29,
            20,
        ),
        (
            "fn main(){let x=1+",
            "grammar.TryExpr",
            &[
                "(",
                "-",
                "IntLiteral",
                "Name",
                "[",
                "consume",
                "false",
                "inout",
                "true",
            ],
            18,
            18,
        ),
        (
            "fn main(){let x=1<",
            "grammar.TryExpr",
            &[
                "(",
                "-",
                "IntLiteral",
                "Name",
                "[",
                "consume",
                "false",
                "inout",
                "true",
            ],
            18,
            18,
        ),
        (
            "fn main(){let x=1?+",
            "grammar.TryExpr",
            &[
                "(",
                "-",
                "IntLiteral",
                "Name",
                "[",
                "consume",
                "false",
                "inout",
                "true",
            ],
            19,
            19,
        ),
        (
            "fn main(){let x=a[",
            "grammar.Primary",
            &[
                "(",
                "-",
                "IntLiteral",
                "Name",
                "[",
                "consume",
                "false",
                "if",
                "inout",
                "true",
            ],
            18,
            18,
        ),
        (
            "fn main(){f(1,",
            "grammar.Argument",
            &[
                "(",
                "-",
                "IntLiteral",
                "Name",
                "[",
                "consume",
                "false",
                "if",
                "inout",
                "true",
            ],
            14,
            14,
        ),
        ("fn main(){let x=(1", "grammar.Primary", &[")"], 18, 16),
        ("fn main(){let x=[1", "grammar.Primary", &[",", "]"], 18, 16),
        (
            "//é\nfn main(){let x=1+",
            "grammar.TryExpr",
            &[
                "(",
                "-",
                "IntLiteral",
                "Name",
                "[",
                "consume",
                "false",
                "inout",
                "true",
            ],
            23,
            23,
        ),
    ];
    for (source, context, expected, offset, prefix_start) in rows {
        let d = parse("trunc.nx", source).unwrap_err();
        assert_eq!(d.code, "NX-E0002", "{source}");
        assert_eq!(d.parser_context.as_deref(), Some(context), "{source}");
        assert_eq!(d.expected_tokens, expected, "{source}");
        assert_eq!(
            d.expected_tokens_kind,
            Some(ExpectedTokensKind::AdmissibleSet)
        );
        assert_eq!(
            *span(&d),
            Span {
                file: "trunc.nx".into(),
                start: offset,
                end: offset
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
                end: offset
            },
            "{source}"
        );
        d.validate("trunc.nx", source).unwrap();
    }
}

#[test]
fn syntax_nesting_limit_is_reported_as_the_published_budget() {
    let source = format!(
        "fn main(){{let x={}[1{};}}",
        "[".repeat(10_000),
        "]".repeat(10_000)
    );
    let d = parse("deep.nx", &source).unwrap_err();
    assert_eq!(d.code, "NX-E0003");
    assert_eq!(d.stage, Stage::Parse);
    assert_eq!(d.operation, Operation::AnalysisLimit);
    assert_eq!(d.primary_span.as_ref().unwrap().start, 143);
    assert_eq!(d.primary_span.as_ref().unwrap().end, 144);
    assert_eq!(
        d.data.get("budget_kind"),
        Some(&DataValue::String("syntax_nesting".into()))
    );
    assert_eq!(d.data.get("limit"), Some(&DataValue::UInt(128)));
    assert_eq!(d.data.get("observed"), Some(&DataValue::UInt(129)));
    d.validate("deep.nx", &source).unwrap();
}

#[test]
fn structural_budgets_have_literal_boundaries_and_diagnostic_fields() {
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
            &format!("fn f(){{{}{}}}", "{".repeat(n - 1), "}".repeat(n - 1)),
        )
        .unwrap();
    }
    check(
        format!("fn f(){{{}{}}}", "{".repeat(128), "}".repeat(128)),
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
                "fn f(x:{}i64{}){{}}",
                "Array<".repeat(n - 1),
                ">".repeat(n - 1)
            ),
        )
        .unwrap();
    }
    check(
        format!("fn f(x:{}i64{}){{}}", "Array<".repeat(128), ">".repeat(128)),
        "syntax_nesting",
        128,
        129,
        775,
        778,
    );
    for n in [125, 126] {
        parse(
            "budget.nx",
            &format!("fn f(){{let x={}1{};}}", "[".repeat(n), "]".repeat(n)),
        )
        .unwrap();
    }
    check(
        format!("fn f(){{let x={}1{};}}", "[".repeat(127), "]".repeat(127)),
        "syntax_nesting",
        128,
        129,
        140,
        141,
    );
    for n in [125, 126] {
        parse(
            "budget.nx",
            &format!("fn f(){{let x={}1{};}}", "(".repeat(n), ")".repeat(n)),
        )
        .unwrap();
    }
    check(
        format!("fn f(){{let x={}1{};}}", "(".repeat(127), ")".repeat(127)),
        "syntax_nesting",
        128,
        129,
        140,
        141,
    );
    let rhs = format!(
        "fn main(){{var x=0;x={}[1{};}}",
        "[".repeat(200),
        "]".repeat(200)
    );
    check(rhs, "syntax_nesting", 128, 129, 147, 148);
    for count in [1_023, 1_024] {
        parse("budget.nx", &"fn f(){}".repeat(count)).unwrap();
    }
    let functions = "fn f(){}".repeat(1_025);
    check(functions, "function_count", 1_024, 1_025, 8_195, 8_196);
    let incomplete = format!("{}fn", "fn f(){}".repeat(1_024));
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
        let source = format!("fn f(){{}}//{}", "x".repeat(size - 10));
        assert_eq!(source.len(), size);
        parse("budget.nx", &source).unwrap();
    }
}

#[test]
fn expression_height_counts_composition_without_precedence_entries() {
    for additions in [126, 127] {
        parse(
            "height.nx",
            &format!("fn f(){{let x=1{};}}", "+1".repeat(additions)),
        )
        .unwrap();
    }
    let source = format!("fn f(){{let x=1{};}}", "+1".repeat(128));
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
        &format!("fn f(){{let x=1{};}}", "+1".repeat(100)),
    )
    .unwrap();
    parse(
        "height.nx",
        &format!("fn f(){{let x=1<{}2{};}}", "(".repeat(126), ")".repeat(126)),
    )
    .unwrap();
    for (prefix, part, suffix) in [("fn f(){a", "[1]", ";}"), ("fn f(){a", "()", ";}")] {
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
    parse("height.nx", "fn f(){a?;}").unwrap();
    assert_eq!(
        parse("height.nx", "fn f(){a??;}").unwrap_err().code,
        "NX-E0002"
    );
    for n in [126, 127] {
        parse(
            "height.nx",
            &format!("fn f(){{{}{{}}}}", "if true{}else ".repeat(n)),
        )
        .unwrap();
    }
    let source = format!("fn f(){{{}{{}}}}", "if true{}else ".repeat(128));
    let d = parse("height.nx", &source).unwrap_err();
    assert_eq!(
        d.data.get("budget_kind"),
        Some(&DataValue::String("expression_tree_depth".into()))
    );
    assert_eq!(d.data.get("observed"), Some(&DataValue::UInt(129)));
    d.validate("height.nx", &source).unwrap();
    parse(
        "height.nx",
        &format!("fn f(){{let x=[1{}];}}", "+1".repeat(126)),
    )
    .unwrap();
    let source = format!("fn f(){{let x=[1{}];}}", "+1".repeat(127));
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
fn logical_syntax_entries_have_boundaries_in_each_operand_position() {
    let cases = [
        ("assignment_rhs", "fn f(){var x=0;x=", ";}", 126),
        ("return_operand", "fn f()->i64{return ", ";}", 126),
        ("if_condition", "fn f(){if ", "{}}", 126),
        ("call_argument", "fn f(){f(", ");}", 126),
        ("index_operand", "fn f(){a[", "];}", 126),
        (
            "value_operand",
            "fn f(){let x=if true{value:",
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
                "fn f(x:{}i64{}){{}}",
                "Result<".repeat(n),
                ",i64>".repeat(n)
            ),
        )
        .unwrap();
    }
    let source = format!(
        "fn f(x:{}i64{}){{}}",
        "Result<".repeat(128),
        ",i64>".repeat(128)
    );
    let d = parse("entry.nx", &source).unwrap_err();
    assert_eq!(
        d.data.get("budget_kind"),
        Some(&DataValue::String("syntax_nesting".into()))
    );
    assert_eq!(d.primary_span.as_ref().unwrap().start, 903);
    d.validate("entry.nx", &source).unwrap();
}

#[test]
fn hostile_chains_return_diagnostics_on_a_small_stack() {
    std::thread::Builder::new()
        .stack_size(2 * 1024 * 1024)
        .spawn(|| {
            for count in [10_000, 200_000, 1_000_000] {
                let sources = [
                    format!("fn f(){{{}{{}}}}", "if true{}else ".repeat(count)),
                    format!("fn f(){{let x=1{};}}", "+1".repeat(count)),
                    format!("fn f(){{a{};}}", "[1]".repeat(count)),
                    format!("fn f(){{a{};}}", "?".repeat(count)),
                    format!(
                        "fn f(){{let x={}1{};}}",
                        "(".repeat(count),
                        ")".repeat(count)
                    ),
                    format!("fn f(){{{}{}}}", "{".repeat(count), "}".repeat(count)),
                    format!(
                        "fn f(){{let x={}1{};}}",
                        "[".repeat(count),
                        "]".repeat(count)
                    ),
                    format!(
                        "fn f(x:{}i64{}){{}}",
                        "Array<".repeat(count),
                        ">".repeat(count)
                    ),
                    format!("fn f() with {}{{}}", "{".repeat(count)),
                    format!(
                        "fn f(){{{}{}}}",
                        "if true{".repeat(count),
                        "}".repeat(count)
                    ),
                    format!(
                        "fn f(){{let x={}1{};}}",
                        "if true{value:".repeat(count),
                        "}".repeat(count)
                    ),
                    format!("fn f(){{let x={}a;}}", "consume ".repeat(count)),
                    format!("fn f(){{let x={}a;}}", "inout ".repeat(count)),
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
#[cfg(not(debug_assertions))]
fn parser_token_patterns_scale_with_source_length() {
    use std::time::{Duration, Instant};
    fn source(pattern: &str, bytes: usize) -> String {
        let (prefix, unit, suffix) = match pattern {
            "statements" => ("fn f(){", "a;", "}"),
            "functions" => ("", "fn f(){}", ""),
            "expressions" => ("fn f(){let x=1", "+1", ";}"),
            "parameters" => ("fn f(", "x:i64,", "x:i64){}"),
            "arguments" => ("fn f(){f(", "1,", "1);}"),
            "blocks" => ("fn f(){", "{}", "}"),
            "bindings" => ("fn f(){", "let x=1;", "}"),
            "typed_bindings" => ("fn f(){", "let x:i64=1;", "}"),
            "assignments" => ("fn f(){var x=0;", "x=1;", "}"),
            "returns" => ("fn f(){", "return;", "}"),
            "if_statements" => ("fn f(){", "if true{}", "}"),
            "while_statements" => ("fn f(){", "while false{}", "}"),
            "array_literals" => ("fn f(){", "let x=[1];", "}"),
            "calls" => ("fn f(){", "f(1);", "}"),
            "lf_statements" => ("fn f(){", "a\n", "}"),
            "crlf_statements" => ("fn f(){", "a\r\n", "}"),
            "cr_statements" => ("fn f(){", "a\r", "}"),
            "commented_statements" => ("fn f(){", "a;//x\n", "}"),
            "spaced_bindings" => ("fn f(){", "let x = 1; ", "}"),
            "paren_exprs" => ("fn f(){", "(1);", "}"),
            "return_values" => ("fn f(){", "return 1;", "}"),
            "indexed_expressions" => ("fn f(){", "a[1];", "}"),
            "adjacent_type_close" => ("fn f(){", "let a:Array<i64>=x;", "}"),
            "nested_type_close" => ("fn f(){", "let a:Result<Array<Array<i64>>,bool>=x;", "}"),
            _ => unreachable!(),
        };
        format!(
            "{prefix}{}{suffix}",
            unit.repeat((bytes - prefix.len() - suffix.len()) / unit.len())
        )
    }
    fn measure(pattern: &str, bytes: usize) -> Duration {
        let source = source(pattern, bytes);
        let mut minima = [Duration::MAX; 5];
        for minimum in &mut minima {
            for _ in 0..2 {
                let start = Instant::now();
                let _ = parse("scale.nx", &source);
                *minimum = (*minimum).min(start.elapsed());
            }
        }
        minima.sort();
        minima[2]
    }
    for pattern in [
        "statements",
        "functions",
        "expressions",
        "parameters",
        "arguments",
        "blocks",
        "bindings",
        "typed_bindings",
        "assignments",
        "returns",
        "if_statements",
        "while_statements",
        "array_literals",
        "calls",
        "lf_statements",
        "crlf_statements",
        "cr_statements",
        "commented_statements",
        "spaced_bindings",
        "paren_exprs",
        "return_values",
        "indexed_expressions",
        "adjacent_type_close",
        "nested_type_close",
    ] {
        let samples: Vec<_> = [65_536, 131_072, 262_144, 524_288, 1_048_576]
            .into_iter()
            .map(|bytes| measure(pattern, bytes))
            .collect();
        let ratio = samples[4].as_secs_f64() / samples[2].as_secs_f64();
        eprintln!(
            "linearity {pattern}: 256 KiB {:?}, 1 MiB {:?}, ratio {:.2}",
            samples[2], samples[4], ratio
        );
        assert!(
            ratio <= 8.0,
            "{pattern}: {:?} to {:?}",
            samples[2],
            samples[4]
        );
        for pair in samples.windows(2) {
            assert!(
                pair[1].as_secs_f64() / pair[0].as_secs_f64() <= 3.5,
                "{pattern}: per-byte cost rose from {:?} to {:?}",
                pair[0],
                pair[1]
            );
        }
    }
}

#[test]
fn adversarial_inputs_preserve_parser_diagnostic_validity() {
    let soup = b"fn main(){let x=([,)]?+-<>!=@/*\r\n0123value";
    for seed in 0..20_000usize {
        let mut state = (seed as u64 + 1).wrapping_mul(0x9e3779b97f4a7c15);
        let mut next = || {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            state as u8
        };
        let len = seed % 193;
        let bytes: Vec<u8> = match seed % 4 {
            0 => (0..len).map(|_| next()).collect(),
            1 => (0..len)
                .map(|_| soup[(next() as usize) % soup.len()])
                .collect(),
            2 => {
                let mut b = b"fn main(){let x=1;//".to_vec();
                b.extend_from_slice(&[0xc3, 0xa9, 0xf0, 0x9f, 0x99, 0x82][..seed % 7]);
                b
            }
            _ => format!("fn main(){{let x={};}}", next() % 100).into_bytes(),
        };
        if let Ok(source) = diagnostic::decode_utf8(bytes)
            && let Err(d) = parse("fuzz.nx", &source)
        {
            d.validate("fuzz.nx", &source)
                .unwrap_or_else(|e| panic!("invalid parser diagnostic for seed {seed}: {e}"));
        }
    }
}

#[test]
fn lexer_comment_scan_scales_with_source_bytes() {
    use std::time::Instant;
    let measure = |mebibytes: usize| {
        let source = format!("fn main(){{}}//{}", "x".repeat(mebibytes * 1_048_576));
        let mut best = std::time::Duration::MAX;
        for _ in 0..3 {
            let start = Instant::now();
            lex::lex("linear.nx", &source).unwrap();
            best = best.min(start.elapsed());
        }
        best
    };
    let one = measure(1);
    let four = measure(4);
    assert!(
        four.as_nanos() <= one.as_nanos() * 8,
        "1 MiB {one:?}; 4 MiB {four:?}"
    );
}

#[test]
fn terminator_insertions_cover_lf_crlf_and_cr_without_closing_brace_insertion() {
    for newline in ["\n", "\r\n", "\r"] {
        let source = format!("fn main(){{let x=1{newline}let y=x+1;}}");
        let ast = parse("lines.nx", &source).unwrap();
        assert_eq!(ast.functions[0].body.statements.len(), 2);

        let empty_return = format!("fn main(){{return{newline}(1);}}");
        let ast = parse("lines.nx", &empty_return).unwrap();
        assert!(matches!(
            ast.functions[0].body.statements[0],
            Statement::Return { value: None, .. }
        ));

        let empty_value =
            format!("fn main(){{let x=if true{{value:{newline}(1);}}else{{value: 2;}};}}");
        let d = parse("lines.nx", &empty_value).unwrap_err();
        assert_eq!(d.rule, "NX-E1302/missing_value_operand");
        d.validate("lines.nx", &empty_value).unwrap();
    }
    let source = "fn main(){if true{1}else{}}";
    let d = parse("same-line.nx", source).unwrap_err();
    assert_eq!(d.code, "NX-E0005");
    assert_eq!(d.primary_span.as_ref().unwrap().start, 19);
    assert_eq!(d.primary_span.as_ref().unwrap().end, 20);
    d.validate("same-line.nx", source).unwrap();
}

#[test]
fn terminator_production_context_matrix_matches_explicit_repair_programs() {
    // §3.1 insertion matrix: each eligible completed statement production
    // is paired with the same program after replacing the newline by `;`.
    // Structural statements end at their closing brace; continuation rows
    // deliberately keep the expression open across the line break.
    for newline in ["\n", "\r\n", "\r"] {
        let cases = [
            ("let x=1", "let x=1;"),
            ("x=1", "x=1;"),
            ("return ()", "return ();"),
            ("f()", "f();"),
            ("{let x=1;}", "{let x=1;}"),
            ("while true{}", "while true{}"),
            ("if true{}else{}", "if true{}else{}"),
        ];
        for (left, repaired) in cases {
            let implicit = format!("fn main(){{{left}{newline}let z=2;}} ");
            let explicit = format!("fn main(){{{repaired}let z=2;}} ");
            let a = print_canonical(&parse("matrix.nx", &implicit).unwrap());
            let b = print_canonical(&parse("matrix.nx", &explicit).unwrap());
            assert_eq!(a, b, "{left:?} {newline:?}");
        }
        for continuation in ["(", "[", "?", "+", "-", "<", "==", ","] {
            let source = match continuation {
                "(" => {
                    format!("fn f(x:i64)->i64{{return x;}} fn main(){{let x=f{newline}(1);}}")
                }
                "[" => format!("fn main(){{let a=[1];let x=a{newline}[0];}}"),
                "?" => format!(
                    "fn f()->Result<i64,i64>{{return Ok(1);}} fn main(){{let x=f(){newline}?;}}"
                ),
                "+" | "-" | "<" | "==" => {
                    format!("fn main(){{let x=1{newline}{continuation}2;}}")
                }
                _ => format!("fn f(x:i64,y:i64){{}} fn main(){{f(1{newline},2);}}"),
            };
            assert!(
                parse("continuation.nx", &source).is_ok(),
                "{continuation:?} {newline:?}: {source}"
            );
        }
    }
}

#[test]
fn terminator_repair_uses_expression_end_and_exact_inserted_text() {
    for (src, preceding_end) in [
        ("fn main(){let x=1}", 17usize),
        ("//é\nfn main(){let x=1}", 22usize),
        ("fn main()->i64{return 1}", 23usize),
        ("fn main(){f()}", 13usize),
    ] {
        let d = parse("repair.nx", src).unwrap_err();
        assert_eq!(d.code, "NX-E0005", "{src}");
        assert_eq!(d.parser_context.as_deref(), Some("statement_terminator"));
        assert_eq!(d.expected_tokens, [";"]);
        assert_eq!(
            d.expected_tokens_kind,
            Some(ExpectedTokensKind::CanonicalInsertion)
        );
        let edit = &d.repair[0].edits[0];
        assert_eq!(edit.replacement, ";");
        assert_eq!(edit.expected_text, "");
        assert_eq!(edit.span.start, preceding_end as u64, "{src}");
        assert_eq!(edit.span.end, preceding_end as u64, "{src}");
        d.validate("repair.nx", src).unwrap();
    }
}
#[test]
fn terminator_repair_precedes_intervening_trivia() {
    // §3.1 and §13.14 place the insertion at the end of the completed
    // expression even when the next token is separated by trivia.
    for source in [
        "fn main(){let x=1  }",
        "fn main(){let x=1\t}",
        "fn main(){x=f(1)  }",
        "fn main(){x=f(1)\t}",
        "fn main()->i64{return 1  }",
        "fn main()->i64{return 1\t}",
    ] {
        let brace = source.rfind('}').unwrap();
        let expression_end = source[..brace].trim_end().len();
        let d = parse("trivia.nx", source).unwrap_err();
        assert_eq!(d.code, "NX-E0005", "{source}");
        assert_eq!(d.primary_span.as_ref().unwrap().start as usize, brace);
        assert_eq!(
            d.unexpected_token.as_ref().unwrap().text.as_deref(),
            Some("}")
        );
        assert_eq!(
            d.repair[0].edits[0].span.start as usize, expression_end,
            "{source}"
        );
        assert_eq!(
            d.repair[0].edits[0].span.end as usize, expression_end,
            "{source}"
        );
        d.validate("trivia.nx", source).unwrap();
    }
    for source in [
        "fn main(){let x=1 // no\n}",
        "fn main(){x=f(1) // no\r\n}",
        "fn main()->i64{return 1 // no\r\n}",
    ] {
        parse("trivia.nx", source).unwrap();
    }
}
#[test]
fn terminator_statement_cases_have_isolated_crlf_and_repair_pairs() {
    // Offsets are literal byte positions in the sources built below.
    let cases = [
        ("fn main(){", "let x=1", 17, 16),
        ("fn main(){", "x=1", 13, 12),
        ("fn main()->i64{", "return 1", 23, 22),
        ("fn main(){", "f()", 13, 10),
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
            *span(&d),
            Span {
                file: "terminator.nx".into(),
                start: end,
                end: end + 1
            }
        );
        let unexpected = d.unexpected_token.as_ref().unwrap();
        assert_eq!(unexpected.kind, TokenKind::Symbol);
        assert_eq!(unexpected.text.as_deref(), Some("}"));
        assert_eq!(unexpected.span, *span(&d));
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
            *span(&eof),
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
fn eof_is_final_in_the_parser_token_stream() {
    let d = parse("eof.nx", "fn main(){} unexpected").unwrap_err();
    assert_eq!(d.code, "NX-E0002");
    assert_eq!(d.parser_context.as_deref(), Some("module_item"));
    assert_eq!(
        d.unexpected_token.as_ref().unwrap().text.as_deref(),
        Some("unexpected")
    );
}
#[test]
fn else_and_next_function_continue_across_crlf() {
    let branch = "fn main(){let x=if true{}\r\nelse{};}";
    let ast = parse("continuation.nx", branch).unwrap();
    assert_eq!(ast.functions[0].body.statements.len(), 1);
    let functions = "fn a(){}\r\nfn b(){}";
    let ast = parse("continuation.nx", functions).unwrap();
    assert_eq!(ast.functions.len(), 2);
}
#[test]
fn crlf_terminates_a_statement_at_the_first_newline_byte() {
    let source = "fn main(){let x=1\r\nlet y=2;}";
    let ast = parse("crlf.nx", source).unwrap();
    assert_eq!(ast.functions[0].body.statements[0].span().end, 17);
}
#[test]
fn value_marker_followed_by_crlf_has_no_operand() {
    let source = "fn main(){let x=if true{value:\r\n(1);}else{value:2;};}";
    let d = parse("value.nx", source).unwrap_err();
    assert_eq!(d.rule, "NX-E1302/missing_value_operand");
    d.validate("value.nx", source).unwrap();
}
#[test]
fn place_requires_a_name_root_even_through_grouping_and_indexing() {
    let valid = parse("place.nx", "fn main(){let a=[0];(a)[0]=1;}").unwrap();
    assert!(matches!(
        valid.functions[0].body.statements[1],
        Statement::Assign { .. }
    ));

    let invalid_target = "fn main(){1=2;}";
    let d = parse("place.nx", invalid_target).unwrap_err();
    assert_eq!(d.code, "NX-E0005");
    assert_eq!(d.parser_context.as_deref(), Some("statement_terminator"));
    assert_eq!(
        *span(&d),
        Span {
            file: "place.nx".into(),
            start: 11,
            end: 12
        }
    );
    d.validate("place.nx", invalid_target).unwrap();

    let invalid_prefix = "fn main(){let a=1;consume (1+2);}";
    let d = parse("place.nx", invalid_prefix).unwrap_err();
    assert_eq!(d.code, "NX-E0002");
    assert_eq!(d.parser_context.as_deref(), Some("consume_place"));
    assert_eq!(d.expected_tokens, ["Place"]);
    assert_eq!(
        *span(&d),
        Span {
            file: "place.nx".into(),
            start: 27,
            end: 28
        }
    );
    assert_eq!(
        d.unexpected_token.as_ref().unwrap().kind,
        TokenKind::Integer
    );
    assert_eq!(
        d.unexpected_token.as_ref().unwrap().text.as_deref(),
        Some("1")
    );
    assert_eq!(
        d.recognized_prefix.as_ref().unwrap(),
        &Span {
            file: "place.nx".into(),
            start: 18,
            end: 27
        }
    );
    d.validate("place.nx", invalid_prefix).unwrap();
}
#[test]
fn nullable_return_value_and_completed_try_expression_at_eof_need_terminators() {
    for (source, end) in [("fn main(){return", 16), ("fn main(){let x=1?", 18)] {
        let d = parse("eof.nx", source).unwrap_err();
        assert_eq!(d.code, "NX-E0005", "{source}");
        assert_eq!(d.parser_context.as_deref(), Some("statement_terminator"));
        assert_eq!(d.expected_tokens, [";"]);
        assert_eq!(
            d.expected_tokens_kind,
            Some(ExpectedTokensKind::CanonicalInsertion)
        );
        assert_eq!(
            *span(&d),
            Span {
                file: "eof.nx".into(),
                start: end,
                end
            }
        );
        assert_eq!(d.unexpected_token.as_ref().unwrap().kind, TokenKind::Eof);
        assert_eq!(
            d.unexpected_token.as_ref().unwrap().text.as_deref(),
            Some("")
        );
        assert_eq!(
            d.recognized_prefix.as_ref().unwrap(),
            &Span {
                file: "eof.nx".into(),
                start: 10,
                end
            }
        );
        assert_eq!(d.repair[0].edits[0].replacement, ";");
        d.validate("eof.nx", source).unwrap();
        assert!(parse("eof.nx", &format!("{source};}}")).is_ok());
    }
}
#[test]
fn structural_closures_and_commented_crlf_obey_terminator_boundaries() {
    for (statement, end) in [("{}", 12), ("while true{}", 22), ("if true{}else{}", 25)] {
        let source = format!("fn main(){{{statement}\r\nlet z=2;}}");
        let ast = parse("structure.nx", &source).unwrap();
        assert_eq!(ast.functions[0].body.statements.len(), 2, "{statement}");
        assert!(
            matches!(&ast.functions[0].body.statements[1], Statement::Let { span, .. } if span.start == end + 2)
        );
    }

    let comment = "fn main(){let x=1//é\r\nlet y=2;}";
    let ast = parse("structure.nx", comment).unwrap();
    assert!(
        matches!(&ast.functions[0].body.statements[0], Statement::Let { span, .. } if span.end == 21)
    );
    assert!(
        matches!(&ast.functions[0].body.statements[1], Statement::Let { span, .. } if span.start == 23)
    );
    let explicit = "fn main(){let x=1;//é\r\nlet y=2;}";
    assert_eq!(
        print_canonical(&ast),
        print_canonical(&parse("structure.nx", explicit).unwrap())
    );

    let missing = "fn main(){let x=if true{}else{}}";
    let d = parse("structure.nx", missing).unwrap_err();
    assert_eq!(d.code, "NX-E0005");
    assert_eq!(d.parser_context.as_deref(), Some("statement_terminator"));
    assert_eq!(d.expected_tokens, [";"]);
    assert_eq!(
        d.unexpected_token.as_ref().unwrap().text.as_deref(),
        Some("}")
    );
    d.validate("structure.nx", missing).unwrap();
    assert!(parse("structure.nx", "fn main(){let x=if true{}else{};}").is_ok());
    assert!(
        parse(
            "structure.nx",
            "fn main(){let x=if true{}else{}\r\nlet z=2;}"
        )
        .is_ok()
    );
}
#[test]
fn parses_multiple_signatures_written_types_and_nested_expressions() {
    let src = "fn value(x:i64)->i64{return x;} fn main(){let value=2;let a:Array<Result<i64,bool>>=[value -1];let x=if true{value: a[0];}else{value: [];};}";
    let ast = parse("x", src).unwrap();
    assert_eq!(ast.functions.len(), 2);
    assert_eq!(ast.functions[0].name, "value");
    assert_eq!(ast.functions[0].params[0].ty, Type::I64);
    assert!(matches!(
        ast.functions[1].body.statements[2],
        Statement::Let { .. }
    ));
}
#[test]
fn type_close_token_ge_splits_only_in_type_context() {
    let ast = parse("x", "fn main(){let r:Result<i64,i64>=Ok(1);}").unwrap();
    assert_eq!(ast.functions[0].body.statements.len(), 1);
    let source = "Result<i64,i64>=Ok(1)";
    let tokens = lex::lex("split.nx", source).unwrap();
    let at = tokens.iter().position(|t| t.text == ">=").unwrap();
    let start = tokens[at].span.start;
    let mut parser = Parser {
        file: "split.nx",
        source,
        tokens,
        at,
        statement_start: 0,
        split_type_close: None,
        depth: 0,
        function_names: HashSet::new(),
        local_scopes: Vec::new(),
    };
    parser.expect_type_close().unwrap();
    assert_eq!(
        (parser.prev_span().start, parser.prev_span().end),
        (start, start + 1)
    );
    assert_eq!(parser.peek().text, "=");
    assert_eq!(
        (parser.peek().span.start, parser.peek().span.end),
        (start + 1, start + 2)
    );
    parser.take();
    assert_eq!(
        (parser.prev_span().start, parser.prev_span().end),
        (start + 1, start + 2)
    );
}
#[test]
fn contextual_value_name_stays_an_ordinary_identifier() {
    let ast = parse(
        "x",
        "fn value(x:i64)->i64{return x;} fn main(){let value=2;let y=value -1;}",
    )
    .unwrap();
    assert_eq!(ast.functions[1].body.statements.len(), 2);
}
#[test]
fn expression_newline_continuations_do_not_insert_a_terminator() {
    let ast = parse(
        "x",
        "fn value(x:i64)->i64{return x;} fn main(){let y=value\n(x);}",
    )
    .unwrap();
    assert_eq!(ast.functions[1].body.statements.len(), 1);
}
#[test]
fn canonical_print_parse_is_idempotent_and_normalizes_terminators() {
    let ast=parse("x","fn f(consume x:Array<i64>)->i64{return -1;} fn main(){let value=2\nlet x=value -1\nassert_eq(x,1);}").unwrap();
    let text = print_canonical(&ast);
    let reparsed = parse("x", &text).unwrap();
    assert_eq!(text, print_canonical(&reparsed));
}
#[test]
fn return_and_value_colon_newline_end_empty_operands() {
    let ret = parse("x", "fn f()->i64{return\n(1);} fn main(){}").unwrap();
    assert!(matches!(
        ret.functions[0].body.statements[0],
        Statement::Return { value: None, .. }
    ));
    let source = "fn main(){let x=if true{value:\n(1);}else{value: 2;};}";
    let val = parse("x", source).unwrap_err();
    assert_eq!(val.rule, "NX-E1302/missing_value_operand");
    val.validate("x", source).unwrap();
}
#[test]
fn missing_terminator_reports_literal_fields_and_repair() {
    let src = "fn main(){let x=1}";
    let d = parse("fixture", src).unwrap_err();
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
        *span(&d),
        Span {
            file: "fixture".into(),
            start: 17,
            end: 18
        }
    );
    assert_eq!(
        d.unexpected_token.as_ref().unwrap().text.as_deref(),
        Some("}")
    );
    assert_eq!(
        d.recognized_prefix.as_ref().unwrap(),
        &Span {
            file: "fixture".into(),
            start: 10,
            end: 17
        }
    );
    assert_eq!(
        d.related_spans[0].span,
        Span {
            file: "fixture".into(),
            start: 16,
            end: 17
        }
    );
    assert_eq!(d.repair[0].edits[0].replacement, ";");
    d.validate("fixture", src).unwrap();
}
#[test]
fn reserved_match_in_binding_name_is_exact_syntax_diagnostic() {
    let src = "fn main(){let match=1;}";
    let d = parse("fixture", src).unwrap_err();
    assert_eq!(d.code, "NX-E0002");
    assert_eq!(d.parser_context.as_deref(), Some("binding_name"));
    assert_eq!(d.expected_tokens, ["Name"]);
    assert_eq!(
        d.expected_tokens_kind,
        Some(ExpectedTokensKind::AdmissibleSet)
    );
    assert_eq!(
        *span(&d),
        Span {
            file: "fixture".into(),
            start: 14,
            end: 19
        }
    );
    assert_eq!(
        d.unexpected_token.as_ref().unwrap().kind,
        TokenKind::Keyword
    );
    assert_eq!(
        d.recognized_prefix.as_ref().unwrap(),
        &Span {
            file: "fixture".into(),
            start: 10,
            end: 14
        }
    );
    d.validate("fixture", src).unwrap();
}
#[test]
fn place_prefix_negatives_report_the_published_contexts_and_spans() {
    for (src, context, start, end, prefix_start) in [
        (
            "fn f(inout x:i64){} fn main(){f(inout 1);}",
            "inout_place",
            38,
            39,
            32,
        ),
        ("fn main(){let x=consume 1;}", "consume_place", 24, 25, 16),
    ] {
        let d = parse("fixture", src).unwrap_err();
        assert_eq!(d.code, "NX-E0002");
        assert_eq!(d.stage, Stage::Parse);
        assert_eq!(d.parser_context.as_deref(), Some(context));
        assert_eq!(d.expected_tokens, ["Place"]);
        assert_eq!(
            *span(&d),
            Span {
                file: "fixture".into(),
                start,
                end
            }
        );
        assert_eq!(
            d.recognized_prefix.as_ref().unwrap(),
            &Span {
                file: "fixture".into(),
                start: prefix_start,
                end: start
            }
        );
        d.validate("fixture", src).unwrap();
    }
}
#[test]
fn migrated_k02_signature_suffix_has_exact_expected_tokens() {
    let src = "fn main() ;";
    let d = parse("fixture", src).unwrap_err();
    assert_eq!(d.code, "NX-E0002");
    assert_eq!(d.parser_context.as_deref(), Some("function_suffix"));
    assert_eq!(d.expected_tokens, ["->", "with", "{"]);
    assert_eq!(
        *span(&d),
        Span {
            file: "fixture".into(),
            start: 10,
            end: 11
        }
    );
    assert_eq!(
        d.recognized_prefix.as_ref().unwrap(),
        &Span {
            file: "fixture".into(),
            start: 0,
            end: 10
        }
    );
}
#[test]
fn excluded_binary_operator_is_not_repaired_as_terminator() {
    for (op, start, end) in [
        ("*", 17, 18),
        ("!=", 17, 19),
        (">", 17, 18),
        ("<=", 17, 19),
        (">=", 17, 19),
    ] {
        let src = format!("fn main(){{let x=1{op}2;}}");
        let d = parse("fixture", &src).unwrap_err();
        assert_eq!(d.code, "NX-E0002");
        assert_eq!(
            d.parser_context.as_deref(),
            Some("unsupported_binary_operator")
        );
        assert_eq!(d.expected_tokens, ["+", "-", "<", "=="]);
        assert_eq!(
            *span(&d),
            Span {
                file: "fixture".into(),
                start,
                end
            }
        );
    }
}
#[test]
fn excluded_operator_prefix_starts_at_active_statement_across_terminators_and_blocks() {
    fn case(n: usize, op: &str) -> (String, usize, usize) {
        let mut source = String::from("fn f(){");
        match n % 4 {
            1 => source.push_str("if true{"),
            2 => source.push_str("while true{"),
            3 => source.push_str("if true{}else{"),
            _ => (),
        }
        if n % 5 == 3 {
            source.push_str("let z=if true{");
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
        assert_eq!(span(&d).start as usize, operator, "{source}");
        assert_eq!(span(&d).end as usize, operator + op.len(), "{source}");
        if matches!(op, ".." | "+=") {
            assert_eq!(d.code, "NX-E0006", "{source}");
            assert_eq!(
                d.feature_id.as_deref(),
                Some(if op == ".." {
                    "range"
                } else {
                    "compound_assignment"
                }),
                "{source}"
            );
        } else {
            assert_eq!(d.code, "NX-E0002", "{source}");
            assert_eq!(d.stage, Stage::Parse);
            assert_eq!(
                d.parser_context.as_deref(),
                Some("unsupported_binary_operator")
            );
            assert_eq!(d.expected_tokens, ["+", "-", "<", "=="]);
            assert_eq!(
                d.recognized_prefix
                    .as_ref()
                    .map(|s| (s.start as usize, s.end as usize)),
                Some((start, operator)),
                "{source}"
            );
            assert!(d.repair.is_empty());
        }
    }
}
#[test]
fn excluded_operator_after_nested_statement_keeps_outer_prefix() {
    // The two literal offsets are byte positions, counted from the beginning
    // of each source. In particular, the nested value statement starts later.
    for (source, token, prefix) in [
        (
            "fn f(){\nlet x = (if c {\nvalue: 1\n} else {\nvalue: 2\n}) * 3;\n}",
            (54, 55),
            (8, 54),
        ),
        (
            "fn f(){\nlet x = 1\nlet y = (if c {\nvalue: 1;\n} else {\nvalue: 2;\n}) != 3;\n}",
            (66, 68),
            (18, 66),
        ),
    ] {
        let d = parse("nested-prefix.nx", source).unwrap_err();
        assert_eq!(d.code, "NX-E0002", "{source}");
        assert_eq!(
            d.parser_context.as_deref(),
            Some("unsupported_binary_operator")
        );
        assert_eq!(d.expected_tokens, ["+", "-", "<", "=="]);
        assert_eq!((span(&d).start, span(&d).end), token, "{source}");
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
            ("if-expression", "let inner=if d{value:1;}else{value:2;};"),
            ("if-statement", "if d{let inner=1;}else{let inner=2;}"),
            ("while", "while d{let inner=1;}"),
            ("block", "{let inner=1;}"),
        ] {
            for depth in 1..=3 {
                for newline in newlines {
                    for op in operators {
                        let mut source = String::from("fn f(){");
                        source.push_str(newline);
                        if context == "assign" {
                            source.push_str("var result=0;");
                            source.push_str(newline);
                        }
                        if context == "value" {
                            source.push_str("let outer=if ready{");
                            source.push_str(newline);
                        }
                        let outer_start = source.len();
                        source.push_str(prefix);
                        source.push_str("(if c {");
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
                            (span(&d).start as usize, span(&d).end as usize),
                            (operator, operator + op.len()),
                            "{context}/{shape}/{depth}/{newline:?}/{op}: {source}: {d:?}"
                        );
                        if matches!(op, ".." | "+=") {
                            assert_eq!(d.code, "NX-E0006", "{source}");
                            assert_eq!(
                                d.feature_id.as_deref(),
                                Some(if op == ".." {
                                    "range"
                                } else {
                                    "compound_assignment"
                                }),
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
                                Some("unsupported_binary_operator"),
                                "{source}"
                            );
                            assert_eq!(d.expected_tokens, ["+", "-", "<", "=="], "{source}");
                            assert_eq!(
                                d.recognized_prefix
                                    .as_ref()
                                    .map(|s| (s.start as usize, s.end as usize)),
                                Some((outer_start, operator)),
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
fn generated_nested_statements_restore_the_outer_operator_prefix() {
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
            "let n=if flag{value:1;}else{value:2;};",
        ][choose(4)];
        let mut source = String::from("fn f(){");
        source.push_str(newline);
        if context == 1 {
            source.push_str("var out=0;");
            source.push_str(newline);
        }
        if context == 3 {
            source.push_str("let out=if ready{");
            source.push_str(newline);
        }
        let outer_start = source.len();
        source.push_str(["let out=", "out=", "return ", "value: ", ""][context]);
        source.push_str("(if ready{");
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
            (span(&d).start as usize, span(&d).end as usize),
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
fn statement_start_scope_restores_on_success_and_error() {
    for (source, expect_error) in [
        ("  let x=1;", false),
        ("  let x=1;else", false),
        ("  let x=1*2;", true),
        ("  let x=(if c{let a=1;value:2;}else{value:3;})*4;", true),
    ] {
        let tokens = lex::lex("scope.nx", source)
            .unwrap()
            .into_iter()
            .filter(|t| t.kind != Kind::Newline)
            .collect();
        let mut parser = Parser {
            file: "scope.nx",
            source,
            tokens,
            at: 0,
            statement_start: 97,
            split_type_close: None,
            depth: 0,
            function_names: HashSet::new(),
            local_scopes: Vec::new(),
        };
        let result = parser.statement(false, 0);
        assert_eq!(result.is_err(), expect_error, "{source}");
        if expect_error {
            let d = result.unwrap_err();
            assert_eq!(d.code, "NX-E0002", "{source}");
            assert_eq!(
                d.recognized_prefix.as_ref().map(|s| s.start),
                Some(2),
                "{source}"
            );
        }
        assert_eq!(parser.statement_start, 97, "{source}");
    }
}
#[test]
fn nested_while_condition_uses_its_own_statement_start() {
    let source = "fn f(){let x=1;if ok{while ready * 2 {}}}";
    let d = parse("while-prefix.nx", source).unwrap_err();
    assert_eq!(d.code, "NX-E0002");
    assert_eq!(
        d.parser_context.as_deref(),
        Some("unsupported_binary_operator")
    );
    assert_eq!(d.expected_tokens, ["+", "-", "<", "=="]);
    assert_eq!((span(&d).start, span(&d).end), (33, 34));
    assert_eq!(
        d.recognized_prefix.as_ref().map(|s| (s.start, s.end)),
        Some((21, 33))
    );
    assert_eq!(
        d.unexpected_token.as_ref().and_then(|t| t.text.as_deref()),
        Some("*")
    );
    d.validate("while-prefix.nx", source).unwrap();
}
#[test]
fn parser_small_source_worker_cost_is_measured() {
    use std::time::Instant;
    const COUNT: usize = 20_000;
    let source = "fn f(){let x=1;}";
    let start = Instant::now();
    for _ in 0..COUNT {
        std::hint::black_box(parse_inner("cost.nx", source).unwrap());
    }
    let direct = start.elapsed();
    let before = PARSE_WORKER_SPAWNS.with(std::cell::Cell::get);
    let start = Instant::now();
    for _ in 0..COUNT {
        std::hint::black_box(parse("cost.nx", source).unwrap());
    }
    let worker = start.elapsed();
    let spawns = PARSE_WORKER_SPAWNS.with(std::cell::Cell::get) - before;
    eprintln!("20k small parses: direct {direct:?}, worker {worker:?}");
    assert!(spawns <= 1, "{spawns} worker spawns for 20k parses");
}
#[test]
fn unavailable_parser_worker_has_a_structural_budget_diagnostic() {
    let source = "fn f(){}";
    let d = worker_unavailable("worker.nx", source);
    assert_eq!(d.code, "NX-E0003");
    assert_eq!(d.stage, Stage::Parse);
    assert_eq!(
        d.data.get("budget_kind"),
        Some(&DataValue::String("syntax_nesting".into()))
    );
    assert_eq!(d.data.get("limit"), Some(&DataValue::UInt(128)));
    assert_eq!(d.data.get("observed"), Some(&DataValue::UInt(129)));
    assert_eq!(
        span(&d),
        &Span {
            file: "worker.nx".into(),
            start: 0,
            end: 1
        }
    );
    d.validate("worker.nx", source).unwrap();
}
#[test]
fn adr_gap_function_value_and_later_syntax_error_priority_remains_explicit() {
    // §3 expression grammar, §4.1 name resolution and §13.14 do not publish
    // a priority row for F09 versus a later syntax error in one statement.
    for (source, code, context) in [
        (
            "fn f(){} fn g(){let x=f<1<2;}",
            "NX-E0002",
            Some("grammar.CompareExpr"),
        ),
        (
            "fn f(){} fn g(){let x=f*2;}",
            "NX-E0002",
            Some("unsupported_binary_operator"),
        ),
        ("fn f(){} fn g(){let x=f 1;}", "NX-E0006", None),
        (
            "fn f(){} fn g(){let x=(f",
            "NX-E0002",
            Some("grammar.Primary"),
        ),
        ("fn f(){} fn g(){g(f", "NX-E0006", None),
    ] {
        let d = parse("priority.nx", source).unwrap_err();
        assert_eq!(d.code, code, "{source}");
        assert_eq!(d.parser_context.as_deref(), context, "{source}");
        d.validate("priority.nx", source).unwrap();
    }
}
#[test]
fn f03_f04_and_option_are_recognized_with_their_feature_ids() {
    for (src, id, lo, hi) in [
        ("fn main(){let x=0.5;}", "float_literal", 16, 19),
        ("fn main(){let n=1;let x=0..n;}", "range", 25, 27),
        ("fn f(x:Option<i64>){} fn main(){}", "option", 7, 13),
    ] {
        let d = parse("fixture", src).unwrap_err();
        assert_eq!(d.code, "NX-E0006");
        assert_eq!(d.subcause, None);
        assert_eq!(d.feature_id.as_deref(), Some(id));
        assert_eq!(
            *span(&d),
            Span {
                file: "fixture".into(),
                start: lo,
                end: hi
            }
        );
        d.validate("fixture", src).unwrap();
    }
}
#[test]
fn parse_stage_feature_fixtures_keep_their_published_ids() {
    let fixtures: [(&str, &str); 33] = [
        ("fn main(){let r=mock_open(1);let s=move r;}", "legacy_move"),
        ("fn f(){return 1;} fn main(){}", "inferred_non_unit_result"),
        ("fn main(){let x=0.5;}", "float_literal"),
        ("fn main(){let n=1;let x=0..n;}", "range"),
        ("fn main(){let x=(1,2);}", "tuple"),
        ("@derive(JsonEncode) fn main(){}", "attributes"),
        ("fn f(x:i64=1){} fn main(){f(1);}", "default_argument"),
        ("fn main(){let f=fn[](){return ();};}", "closure"),
        ("fn f(){} fn main(){let g=f;}", "function_value"),
        ("fn f(){} fn main(){consume f();}", "receiver_mode"),
        (
            "fn main(){let a=[1];let x=consume a[0];}",
            "partial_transfer",
        ),
        ("fn main(){let a=[mock_open(1)];}", "resource_array"),
        ("fn main() with no_heap_alloc{}", "no_heap_alloc"),
        ("fn main(){match true{true=>{},false=>{}}}", "match"),
        ("fn f(x:Option<i64>){} fn main(){}", "option"),
        ("struct S{} fn main(){}", "user_types"),
        ("trait T{} fn main(){}", "traits_generics"),
        ("use std.app;fn main(){}", "modules_visibility"),
        ("extend i64{} fn main(){}", "extensions"),
        (
            "fn f(x:i64)->i64 from(x){return x;} fn main(){}",
            "provenance_clauses",
        ),
        ("const X:i64=comptime 1;fn main(){}", "comptime"),
        ("fn main(){let x=\"hola\";}", "text"),
        ("fn main(){let a=[1];a.foo();}", "methods"),
        ("fn main(){var x=1;x+=1;}", "compound_assignment"),
        ("fn main(){for n in [1]{}}", "for_iteration"),
        ("fn main(){while false{break;}}", "loop_control"),
        ("gen fn g(){} fn main(){}", "generator"),
        ("async fn f(){} fn main(){}", "async"),
        ("fn f(x:u32){} fn main(){}", "unsupported_type"),
        ("fn f(x:String){} fn main(){}", "unsupported_type"),
        ("fn f(x:app.Error){} fn main(){}", "unsupported_type"),
        ("fn main(){region r{}}", "region"),
        ("fn main(){let x=self;}", "methods"),
    ];
    for (src, expected) in fixtures {
        let d = parse("f2", src).unwrap_err();
        assert_eq!(d.code, "NX-E0006", "{src}");
        assert_eq!(d.subcause, None, "{src}");
        assert_eq!(d.feature_id.as_deref(), Some(expected), "{src}");
        d.validate("f2", src).unwrap();
    }
}
#[test]
fn branch_value_negatives_match_b04_b05_b06_and_l02() {
    let cases = [
        (
            "fn main(){value:1;}",
            "invalid_value_target",
            10,
            16,
            RelatedRole::Context,
            9,
            10,
        ),
        (
            "fn f()->i64{return if true{value:}else{value:2;};} fn main(){}",
            "missing_value_operand",
            27,
            33,
            RelatedRole::Context,
            26,
            27,
        ),
        (
            "fn f()->i64{return if true{value:1;2;}else{value:2;};} fn main(){}",
            "non_final_branch_value",
            27,
            33,
            RelatedRole::Context,
            35,
            37,
        ),
        (
            "fn f()->i64{\nreturn if true{\nvalue:\n(1);\n}else{value:2;};\n}\nfn main(){}",
            "missing_value_operand",
            29,
            35,
            RelatedRole::Context,
            27,
            28,
        ),
    ];
    for (source, subcause, start, end, role, related_start, related_end) in cases {
        let d = parse("branches.nx", source).unwrap_err();
        assert_eq!(d.code, "NX-E1302", "{source}");
        assert_eq!(d.subcause.as_deref(), Some(subcause), "{source}");
        assert_eq!(d.stage, Stage::Parse);
        assert_eq!(d.operation, Operation::SyntaxParse);
        assert_eq!(
            d.primary_span,
            Some(Span {
                file: "branches.nx".into(),
                start,
                end
            })
        );
        assert!(
            d.related_spans.iter().any(|r| r.role == role
                && r.span.start == related_start
                && r.span.end == related_end)
        );
        d.validate("branches.nx", source).unwrap();
    }
    let l02 = "fn f()->i64{\nreturn if true{\nvalue:\n(1);\n}else{value:2;};\n}\nfn main(){}";
    let d = parse("branches.nx", l02).unwrap_err();
    assert!(
        d.related_spans
            .iter()
            .any(|r| r.role == RelatedRole::Delimiter && r.span.start == 35 && r.span.end == 36)
    );
    let positive = "fn f()->i64{\nreturn if true{\nvalue: (\n1);\n}else{value:2;};\n}\nfn main(){}";
    let ast = parse("branches.nx", positive).unwrap();
    assert!(
        matches!(&ast.functions[0].body.statements[0], Statement::Return { value: Some(Expr::If { then_block, .. }), .. } if matches!(&then_block.statements[0], Statement::Value { value: Some(Expr::Group(_, _)), .. }))
    );
    for source in [
        "fn main(){let value=1;}",
        "fn f()->i64{return if true{value:1;}else{value:2;};} fn main(){}",
    ] {
        parse("branches.nx", source).unwrap();
    }
}

#[test]
fn with_clause_w01_to_w07_and_accepted_forms() {
    let negatives = [
        (
            "fn main() with {call(no_io),call(no_alloc)}{}",
            "duplicate_operation",
            28,
            32,
        ),
        (
            "fn main() with {resume(no_io)}{}",
            "invalid_operation",
            16,
            22,
        ),
        (
            "fn main() with no_io,no_io{}",
            "duplicate_restriction",
            21,
            26,
        ),
        ("fn main() with {}{}", "empty_operation_list", 15, 17),
        (
            "fn main() with unrestricted,no_io{}",
            "unrestricted_mixed",
            15,
            33,
        ),
        (
            "fn main() with {call(no_io,unrestricted)}{}",
            "unrestricted_mixed",
            21,
            39,
        ),
        (
            "fn main() with unrestricted,unrestricted,no_io{}",
            "duplicate_restriction",
            28,
            40,
        ),
    ];
    for (source, cause, start, end) in negatives {
        let d = parse("with.nx", source).unwrap_err();
        assert_eq!(d.code, "NX-E0417", "{source}");
        assert_eq!(d.subcause.as_deref(), Some(cause), "{source}");
        assert_eq!(d.stage, Stage::Parse);
        assert_eq!(d.operation, Operation::SyntaxParse);
        assert_eq!(
            d.primary_span,
            Some(Span {
                file: "with.nx".into(),
                start,
                end
            })
        );
        d.validate("with.nx", source).unwrap();
    }
    for source in [
        "fn main() with no_io{}",
        "fn main() with unrestricted{}",
        "fn main() with {call(unrestricted)}{}",
        "fn main() with {call(no_alloc,no_io)}{}",
    ] {
        let ast = parse("with.nx", source).unwrap();
        let printed = print_canonical(&ast);
        assert_eq!(
            print_canonical(&parse("with.nx", &printed).unwrap()),
            printed
        );
        assert!(ast.functions[0].with_clause.is_some());
    }
    let simple = parse("with.nx", "fn main() with no_io{}").unwrap();
    assert_eq!(
        simple.functions[0].with_clause,
        Some(WithClause::Simple(vec!["no_io".into()]))
    );
    let map = parse("with.nx", "fn main() with {call(no_alloc,no_io)}{}").unwrap();
    assert!(
        matches!(&map.functions[0].with_clause, Some(WithClause::Operations(ops)) if ops.len() == 1 && ops[0].name == "call" && ops[0].restrictions == ["no_alloc", "no_io"] && ops[0].span.start == 16 && ops[0].span.end == 36)
    );
    for positive in [
        "fn main() with {call(no_io)}{}",
        "fn main() with {call(no_io)}{}",
        "fn main() with no_io{}",
        "fn main() with {call(unrestricted)}{}",
        "fn main() with no_io{}",
        "fn main() with {call(no_io)}{}",
        "fn main() with no_io{}",
    ] {
        parse("with.nx", positive).unwrap();
    }
    let excluded = parse("with.nx", "fn main() with no_heap_alloc{}").unwrap_err();
    assert_eq!(excluded.code, "NX-E0006");
    assert_eq!(excluded.feature_id.as_deref(), Some("no_heap_alloc"));
    excluded
        .validate("with.nx", "fn main() with no_heap_alloc{}")
        .unwrap();
    let syntax = parse("with.nx", "fn main() with create(no_io){}").unwrap_err();
    assert_eq!(syntax.code, "NX-E0002");
    assert_eq!(syntax.parser_context.as_deref(), Some("grammar.WithClause"));
}
#[test]
fn l01_l03_l04_have_published_parse_trees_and_byte_spans() {
    let l01 = "fn f()->i64{\nreturn\n(1);\n}\nfn main(){}";
    let ast = parse("l.nx", l01).unwrap();
    assert_eq!(ast.functions.len(), 2);
    assert_eq!(ast.functions[0].body.statements.len(), 2);
    assert!(
        matches!(&ast.functions[0].body.statements[0], Statement::Return { value: None, span } if span.start == 13 && span.end == 19)
    );
    assert!(matches!(
        &ast.functions[0].body.statements[1],
        Statement::Expr {
            value: Expr::Group(_, _),
            ..
        }
    ));
    let positive = "fn f()->i64{\nreturn (\n1);\n}\nfn main(){}";
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
        "fn value(x:i64)->i64{return x;}\nfn main(){let x=7;let y=value\n(x);assert_eq(y,7);}";
    let ast = parse("l.nx", l03).unwrap();
    assert!(
        matches!(&ast.functions[1].body.statements[1], Statement::Let { value: Expr::Call { target, span, .. }, .. } if matches!(&**target, Expr::Name(name, _) if name == "value") && span.start == 56 && span.end == 65)
    );

    let l04 = "fn main(){\nlet value=2\nlet a=value -1\nlet b=value - 1\nassert_eq(a,b)\n}";
    let ast = parse("l.nx", l04).unwrap();
    assert_eq!(ast.functions[0].body.statements.len(), 4);
    let ends: Vec<_> = ast.functions[0]
        .body
        .statements
        .iter()
        .map(|s| s.span().end)
        .collect();
    assert_eq!(ends, [22, 37, 53, 68]);
    assert!(
        matches!(&ast.functions[0].body.statements[1], Statement::Let { value: Expr::Binary { op, span, .. }, .. } if op == "-" && span.start == 29 && span.end == 37)
    );
    let variant = l04.replacen("value -1", "value - 1", 1);
    let other = parse("l.nx", &variant).unwrap();
    assert!(
        matches!(&other.functions[0].body.statements[1], Statement::Let { value: Expr::Binary { span, .. }, .. } if span.start == 29 && span.end == 38)
    );
    assert_eq!(print_canonical(&ast), print_canonical(&other));
}

#[test]
fn l05_l06_l07_keep_unit_discard_and_original_utf8_offsets() {
    let l05 = "fn main(){\nlet x=if true{value: ();}else{value: ();};\nassert_eq(x,());\n}";
    let ast = parse("l.nx", l05).unwrap();
    assert!(
        matches!(&ast.functions[0].body.statements[0], Statement::Let { value: Expr::If { then_block, .. }, .. } if matches!(&then_block.statements[0], Statement::Value { value: Some(Expr::Unit(_)), span } if span.start == 25 && span.end == 35))
    );

    let l06 = "fn f()->i64{\n1\n}\nfn main(){}";
    let ast = parse("l.nx", l06).unwrap();
    assert_eq!(ast.functions[0].body.statements.len(), 1);
    assert!(
        matches!(&ast.functions[0].body.statements[0], Statement::Expr { value: Expr::Integer(_, _), span } if span.start == 13 && span.end == 14)
    );
    let positive = "fn f()->i64{\nreturn 1\n}\nfn main(){}";
    let ast = parse("l.nx", positive).unwrap();
    assert!(matches!(
        &ast.functions[0].body.statements[0],
        Statement::Return {
            value: Some(Expr::Integer(_, _)),
            ..
        }
    ));

    let lf = "// á🙂\nfn main(){\nlet x=missing;\n}";
    for (source, start, end) in [
        (lf.to_owned(), 27, 34),
        (lf.replace('\n', "\r\n"), 29, 36),
        (format!("{lf}\n"), 27, 34),
    ] {
        let tokens = lex::lex("l.nx", &source).unwrap();
        let first = tokens
            .iter()
            .find(|token| token.kind != Kind::Newline)
            .unwrap();
        assert_eq!(first.text, "fn");
        assert_eq!(first.span.start, if start == 29 { 11 } else { 10 });
        assert!(tokens.iter().any(|token| token.text == "missing"
            && token.span.start == start
            && token.span.end == end));
        assert_eq!(tokens.last().unwrap().span.start, source.len() as u64);
        let ast = parse("l.nx", &source).unwrap();
        assert!(
            matches!(&ast.functions[0].body.statements[0], Statement::Let { value: Expr::Name(name, span), .. } if name == "missing" && span.start == start && span.end == end)
        );
        let positive = source.replace("let x=missing;", "let missing=1;let x=missing;");
        let ast = parse("l.nx", &positive).unwrap();
        assert_eq!(ast.functions[0].body.statements.len(), 2);
    }
}
#[test]
fn nested_place_roots_and_program_alternatives_keep_inner_first_sets() {
    let valid = "fn main(){let a=[0];((a))[0]=1;}";
    let ast = parse("root.nx", valid).unwrap();
    assert!(
        matches!(&ast.functions[0].body.statements[1], Statement::Assign { place: Expr::Index { target, .. }, .. } if matches!(&**target, Expr::Group(_, _)))
    );
    let rows: &[(&str, &str, &[&str], u64, u64)] = &[
        ("fn main(){((a", "grammar.Primary", &[")"], 13, 11),
        ("fn main(){((a)", "grammar.Primary", &[")"], 14, 10),
        ("fn main(){((a))[0", "grammar.Index", &["]"], 17, 15),
    ];
    for &(source, context, expected, end, prefix) in rows {
        let d = parse("root.nx", source).unwrap_err();
        assert_eq!(d.code, "NX-E0002", "{source}");
        assert_eq!(d.parser_context.as_deref(), Some(context));
        assert_eq!(d.expected_tokens, expected);
        assert_eq!(
            d.expected_tokens_kind,
            Some(ExpectedTokensKind::AdmissibleSet)
        );
        assert_eq!(
            d.primary_span,
            Some(Span {
                file: "root.nx".into(),
                start: end,
                end
            })
        );
        assert_eq!(d.unexpected_token.as_ref().unwrap().kind, TokenKind::Eof);
        assert_eq!(
            d.unexpected_token.as_ref().unwrap().text.as_deref(),
            Some("")
        );
        assert_eq!(
            d.recognized_prefix,
            Some(Span {
                file: "root.nx".into(),
                start: prefix,
                end
            })
        );
        d.validate("root.nx", source).unwrap();
    }
    for (source, start, kind, text) in [
        ("fn a(){} fn b(){} let x=1;", 18, TokenKind::Keyword, "let"),
        ("fn a(){} fn b(){} ;", 18, TokenKind::Symbol, ";"),
        ("fn a(){} fn b(){} value:1;", 18, TokenKind::Name, "value"),
    ] {
        let d = parse("root.nx", source).unwrap_err();
        assert_eq!(d.parser_context.as_deref(), Some("module_item"), "{source}");
        assert_eq!(d.expected_tokens, ["EOF", "fn"]);
        assert_eq!(d.unexpected_token.as_ref().unwrap().kind, kind);
        assert_eq!(
            d.unexpected_token.as_ref().unwrap().text.as_deref(),
            Some(text)
        );
        assert_eq!(d.primary_span.as_ref().unwrap().start, start);
        assert_eq!(d.recognized_prefix.as_ref().unwrap().start, 0);
        assert_eq!(d.recognized_prefix.as_ref().unwrap().end, start);
        d.validate("root.nx", source).unwrap();
    }
}
#[test]
fn commented_crlf_terminators_cover_assignment_return_value_and_expr_statement() {
    let cases = [
        ("fn main(){var x=0;x=1//é\r\nlet z=2;}", 25),
        ("fn f()->i64{return 1//é\r\nlet z=2;} fn main(){}", 24),
        (
            "fn f()->i64{return if true{value:1//é\r\n}else{value:2;};} fn main(){}",
            38,
        ),
        ("fn main(){f()//é\r\nlet z=2;}", 17),
    ];
    for (source, cr) in cases {
        let implicit = parse("p2.nx", source).unwrap();
        let ended = implicit
            .functions
            .iter()
            .flat_map(|f| &f.body.statements)
            .any(|s| s.span().end == cr)
            || matches!(&implicit.functions[0].body.statements[0], Statement::Return { value: Some(Expr::If { then_block, .. }), .. } if then_block.statements[0].span().end == cr);
        assert!(ended, "{source}");
        let explicit_source = source.replace("//é", ";//é");
        let explicit = parse("p2.nx", &explicit_source).unwrap();
        assert_eq!(print_canonical(&implicit), print_canonical(&explicit));
    }
    let negative = "fn f()->i64{return if true{value:1 }else{value:2;};} fn main(){}";
    let d = parse("p2.nx", negative).unwrap_err();
    assert_eq!(d.code, "NX-E0005");
    assert_eq!(d.parser_context.as_deref(), Some("statement_terminator"));
    assert_eq!(d.expected_tokens, [";"]);
    assert_eq!(
        d.unexpected_token.as_ref().unwrap().text.as_deref(),
        Some("}")
    );
    assert_eq!(d.repair[0].edits[0].replacement, ";");
    d.validate("p2.nx", negative).unwrap();
    parse(
        "p2.nx",
        "fn f()->i64{return if true{value:1; }else{value:2;};} fn main(){}",
    )
    .unwrap();
}
#[test]
fn with_clause_truncations_use_d11_inner_productions() {
    let rows: &[(&str, &str, &[&str], u64)] = &[
        (
            "fn main() with {",
            "grammar.OperationList",
            &["call", "cancel", "copy", "create", "drop", "resume", "}"],
            15,
        ),
        ("fn main() with {call", "grammar.OperationLimit", &["("], 16),
        (
            "fn main() with {call(",
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
            "fn main() with {call(no_io",
            "grammar.OperationLimit",
            &[")", ","],
            16,
        ),
        (
            "fn main() with {call(no_io)",
            "grammar.OperationList",
            &[",", "}"],
            15,
        ),
        (
            "fn main() with {call(no_io),",
            "grammar.Operation",
            &["call", "cancel", "copy", "create", "drop", "resume"],
            28,
        ),
        ("fn main() with {call(no_io)}", "function_suffix", &["{"], 0),
        (
            "fn main() with no_io,",
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
            d.primary_span,
            Some(Span {
                file: "with-trunc.nx".into(),
                start: end,
                end
            })
        );
        assert_eq!(d.unexpected_token.as_ref().unwrap().kind, TokenKind::Eof);
        assert_eq!(
            d.unexpected_token.as_ref().unwrap().text.as_deref(),
            Some("")
        );
        assert_eq!(
            d.recognized_prefix,
            Some(Span {
                file: "with-trunc.nx".into(),
                start: prefix_start,
                end
            })
        );
        d.validate("with-trunc.nx", source).unwrap();
    }
}
#[test]
fn empty_operation_list_first_does_not_depend_on_bad_token() {
    // D11 §5.2 and §13.14 rules 3–4: OperationList may also close with '}'.
    let prefix = "fn f() with {";
    let eof = parse("with-first.nx", prefix).unwrap_err();
    for token in [
        ")", ";", "x", "1", "value", "(", "[", "]", ",", ":", "=", "->", "with", "else", "let",
        "return", "while", "if", "fn", "no_io", "Array", "i64", "true", "false", "?", "+", "-",
        "<", ">", "==", ".", "@",
    ] {
        let source = format!("{prefix}{token}");
        let d = parse("with-first.nx", &source).unwrap_err();
        assert_eq!(d.code, "NX-E0002", "{source}");
        assert_eq!(d.parser_context, eof.parser_context, "{source}");
        assert_eq!(d.expected_tokens, eof.expected_tokens, "{source}");
        assert_eq!(d.expected_tokens, OPERATION_LIST_FIRST, "{source}");
        assert_eq!(d.recognized_prefix, eof.recognized_prefix, "{source}");
        assert_eq!(d.recognized_prefix.as_ref().unwrap().start as usize, 12);
        assert_eq!(d.recognized_prefix.as_ref().unwrap().end as usize, 13);
        assert_eq!(
            d.primary_span.as_ref().unwrap().start as usize,
            prefix.len()
        );
        d.validate("with-first.nx", &source).unwrap();
    }
}
#[test]
fn roadmap_ex01_to_ex04_keep_contextual_value_as_name() {
    let roadmap =
        include_str!("../../../docs/baseline/Nexo_ROADMAP_rev0.2.md").replace("\r\n", "\n");
    for (id, value_span) in [
        ("EX01.", Some((355, 360))),
        ("EX02.", None),
        ("EX03.", None),
        ("EX04.", Some((244, 249))),
    ] {
        let section = roadmap.split(&format!("### {id}")).nth(1).unwrap();
        let source = section
            .split("```nexo\n")
            .nth(1)
            .unwrap()
            .split("```")
            .next()
            .unwrap()
            .trim_end_matches('\n');
        let tokens = lex::lex("ex.nx", source).unwrap();
        if let Some((start, end)) = value_span {
            assert!(
                tokens.iter().any(|token| token.text == "value"
                    && token.kind == Kind::Name
                    && token.span.start == start
                    && token.span.end == end),
                "{id}"
            );
        }
        let d = parse("ex.nx", source).unwrap_err();
        assert_eq!(d.code, "NX-E0006", "{id}");
        assert_eq!(d.feature_id.as_deref(), Some("modules_visibility"), "{id}");
        assert_eq!(
            d.primary_span,
            Some(Span {
                file: "ex.nx".into(),
                start: 0,
                end: 3
            })
        );
        d.validate("ex.nx", source).unwrap();
    }
}
#[test]
fn value_colon_uses_one_token_lookahead_only_at_statement_start() {
    let source = "fn f(value:i64){} fn main(){let value:\ni64=2;f(value:value);}";
    let ast = parse("value-context.nx", source).unwrap();
    assert!(
        matches!(&ast.functions[1].body.statements[0], Statement::Let { name, annotation: Some(Type::I64), .. } if name == "value")
    );
    assert!(
        matches!(&ast.functions[1].body.statements[1], Statement::Expr { value: Expr::Call { args, .. }, .. } if args[0].label.as_deref() == Some("value") && matches!(&args[0].value, Expr::Name(name, _) if name == "value"))
    );
    let separated = "fn main(){value\n:1;}";
    let d = parse("value-context.nx", separated).unwrap_err();
    assert_eq!(d.code, "NX-E0002");
    assert_ne!(d.subcause.as_deref(), Some("invalid_value_target"));
    d.validate("value-context.nx", separated).unwrap();
    let direct = "fn f()->i64{return if true{value :1;}else{value:2;};} fn main(){}";
    let ast = parse("value-context.nx", direct).unwrap();
    assert!(
        matches!(&ast.functions[0].body.statements[0], Statement::Return { value: Some(Expr::If { then_block, .. }), .. } if matches!(&then_block.statements[0], Statement::Value { .. }))
    );
}
#[test]
fn continuation_tokens_keep_the_full_expression_in_each_statement_context() {
    let cases = [
        ("fn main(){let x=f\n(1);}", 17),
        ("fn main(){let a=[1];let x=a\n[0];}", 27),
        ("fn main(){let x=try_add(1,0)\n?;}", 28),
        ("fn main(){let x=1\n+2;}", 17),
        ("fn main(){let x=1\n-2;}", 17),
        ("fn main(){let x=1\n<2;}", 17),
        ("fn main(){let x=1\n==2;}", 17),
        ("fn f(x:i64,y:i64){} fn main(){f(1\n,2);}", 33),
        ("fn main(){var x=0;x=1\n+2;}", 21),
        ("fn f()->i64{return 1\n+2;} fn main(){}", 20),
        (
            "fn f()->i64{return if true{value:1\n+2;}else{value:2;};} fn main(){}",
            34,
        ),
        ("fn main(){f(1)\n?;}", 14),
    ];
    for (source, newline) in cases {
        assert_eq!(source.as_bytes()[newline as usize], b'\n');
        let continued = parse("continuation.nx", source).unwrap();
        let same_line = source.replacen('\n', "", 1);
        let paired = parse("continuation.nx", &same_line).unwrap();
        assert_eq!(
            print_canonical(&continued),
            print_canonical(&paired),
            "{source}"
        );
        let split = source.replacen('\n', ";\n", 1);
        match parse("continuation.nx", &split) {
            Ok(ast) => assert_ne!(
                print_canonical(&continued),
                print_canonical(&ast),
                "{source}"
            ),
            Err(d) => d.validate("continuation.nx", &split).unwrap(),
        }
    }
}

#[test]
fn published_feature_rows_have_exact_spans_and_literal_repairs() {
    // §14.6: each span is the byte interval delimited by the published markers.
    // Positives apply the edit in the fourth column literally.
    let rows: [(&str, &str, u64, u64, &str); 33] = [
        (
            "fn main(){let r=mock_open(1);let s=move r;}",
            "legacy_move",
            35,
            39,
            "fn main(){let r=mock_open(1);let s=consume r;}",
        ),
        (
            "fn f(){return 1;} fn main(){}",
            "inferred_non_unit_result",
            3,
            4,
            "fn f()->i64{return 1;} fn main(){}",
        ),
        (
            "fn main(){let x=0.5;}",
            "float_literal",
            16,
            19,
            "fn main(){let x=0;}",
        ),
        (
            "fn main(){let n=1;let x=0..n;}",
            "range",
            25,
            27,
            "fn main(){let n=1;let x=0+n;}",
        ),
        (
            "fn main(){let x=(1,2);}",
            "tuple",
            18,
            19,
            "fn main(){let x=(1);}",
        ),
        (
            "@derive(JsonEncode) fn main(){}",
            "attributes",
            0,
            1,
            "fn main(){}",
        ),
        (
            "fn f(x:i64=1){} fn main(){f(1);}",
            "default_argument",
            10,
            11,
            "fn f(x:i64){} fn main(){f(1);}",
        ),
        (
            "fn main(){let f=fn[](){return ();};}",
            "closure",
            16,
            18,
            "fn main(){let f=();}",
        ),
        (
            "fn f(){} fn main(){let g=f;}",
            "function_value",
            25,
            26,
            "fn f(){} fn main(){f();}",
        ),
        (
            "fn f(){} fn main(){consume f();}",
            "receiver_mode",
            19,
            26,
            "fn f(){} fn main(){f();}",
        ),
        (
            "fn main(){let a=[1];let x=consume a[0];}",
            "partial_transfer",
            34,
            38,
            "fn main(){let a=[1];let x=a[0];}",
        ),
        (
            "fn main(){let a=[mock_open(1)];}",
            "resource_array",
            16,
            30,
            "fn main(){let a=mock_open(1);}",
        ),
        (
            "fn main() with no_heap_alloc{}",
            "no_heap_alloc",
            15,
            28,
            "fn main() with no_alloc{}",
        ),
        (
            "fn main(){match true{true=>{},false=>{}}}",
            "match",
            10,
            15,
            "fn main(){if true{}else{}}",
        ),
        (
            "fn f(x:Option<i64>){} fn main(){}",
            "option",
            7,
            13,
            "fn f(x:Result<i64,i64>){} fn main(){}",
        ),
        ("struct S{} fn main(){}", "user_types", 0, 6, "fn main(){}"),
        (
            "trait T{} fn main(){}",
            "traits_generics",
            0,
            5,
            "fn main(){}",
        ),
        (
            "use std.app;fn main(){}",
            "modules_visibility",
            0,
            3,
            "fn main(){}",
        ),
        (
            "extend i64{} fn main(){}",
            "extensions",
            0,
            6,
            "fn main(){}",
        ),
        (
            "fn f(x:i64)->i64 from(x){return x;} fn main(){}",
            "provenance_clauses",
            17,
            21,
            "fn f(x:i64)->i64 {return x;} fn main(){}",
        ),
        (
            "const X:i64=comptime 1;fn main(){}",
            "comptime",
            0,
            5,
            "fn main(){}",
        ),
        (
            "fn main(){let x=\"hola\";}",
            "text",
            16,
            22,
            "fn main(){let x=1;}",
        ),
        (
            "fn main(){let a=[1];a.foo();}",
            "methods",
            21,
            22,
            "fn main(){let a=[1];len(a);}",
        ),
        (
            "fn main(){var x=1;x+=1;}",
            "compound_assignment",
            19,
            21,
            "fn main(){var x=1;x=x+1;}",
        ),
        (
            "fn main(){for n in [1]{}}",
            "for_iteration",
            10,
            13,
            "fn main(){while false{}}",
        ),
        (
            "fn main(){while false{break;}}",
            "loop_control",
            22,
            27,
            "fn main(){while false{}}",
        ),
        (
            "gen fn g(){} fn main(){}",
            "generator",
            0,
            3,
            "fn g(){} fn main(){}",
        ),
        (
            "async fn f(){} fn main(){}",
            "async",
            0,
            5,
            "fn f(){} fn main(){}",
        ),
        (
            "fn f(x:u32){} fn main(){}",
            "unsupported_type",
            7,
            10,
            "fn f(x:i64){} fn main(){}",
        ),
        (
            "fn f(x:String){} fn main(){}",
            "unsupported_type",
            7,
            13,
            "fn f(x:Array<i64>){} fn main(){}",
        ),
        (
            "fn f(x:app.Error){} fn main(){}",
            "unsupported_type",
            7,
            16,
            "fn f(x:i64){} fn main(){}",
        ),
        ("fn main(){region r{}}", "region", 10, 16, "fn main(){{}}"),
        (
            "fn main(){let x=self;}",
            "methods",
            16,
            20,
            "fn main(){let x=0;}",
        ),
    ];
    for (negative, feature, start, end, positive) in rows {
        let d = parse("feature.nx", negative).unwrap_err();
        assert_eq!(d.code, "NX-E0006", "{negative}");
        assert_eq!(d.subcause, None, "{negative}");
        assert_eq!(d.feature_id.as_deref(), Some(feature), "{negative}");
        let expected_stage = if matches!(
            feature,
            "inferred_non_unit_result" | "resource_array" | "option" | "unsupported_type"
        ) {
            Stage::Types
        } else if feature == "function_value" {
            Stage::Names
        } else {
            Stage::Parse
        };
        assert_eq!(d.stage, expected_stage, "{negative}");
        assert_eq!(
            d.operation,
            match expected_stage {
                Stage::Types => Operation::TypesCheck,
                Stage::Names => Operation::NamesResolve,
                _ => Operation::SyntaxParse,
            },
            "{negative}"
        );
        assert_eq!(
            d.primary_span.as_ref().map(|s| (s.start, s.end)),
            Some((start, end)),
            "{negative}"
        );
        assert!(d.unexpected_token.is_none(), "{negative}");
        assert!(d.expected_tokens.is_empty(), "{negative}");
        assert_eq!(d.expected_tokens_kind, None, "{negative}");
        assert!(d.parser_context.is_none(), "{negative}");
        assert!(d.recognized_prefix.is_none(), "{negative}");
        assert!(d.repair.is_empty(), "{negative}");
        if feature == "inferred_non_unit_result" {
            assert_eq!(d.related_spans.len(), 1);
            assert_eq!(d.related_spans[0].role, RelatedRole::Context);
            assert_eq!(
                (d.related_spans[0].span.start, d.related_spans[0].span.end),
                (7, 15)
            );
        }
        if feature == "partial_transfer" {
            assert_eq!(d.related_spans.len(), 1);
            assert_eq!(d.related_spans[0].role, RelatedRole::Context);
            assert_eq!(
                (d.related_spans[0].span.start, d.related_spans[0].span.end),
                (26, 33)
            );
        }
        if !matches!(feature, "inferred_non_unit_result" | "partial_transfer") {
            assert!(d.related_spans.is_empty(), "{negative}");
        }
        d.validate("feature.nx", negative).unwrap();
        parse("feature.nx", positive).unwrap_or_else(|e| panic!("positive {positive}: {e:?}"));
    }
}

fn erase_expr_spans(expr: &mut Expr) {
    let blank = |span: &mut Span| {
        span.file.clear();
        span.start = 0;
        span.end = 0;
    };
    match expr {
        Expr::Name(_, span) | Expr::Integer(_, span) | Expr::Bool(_, span) | Expr::Unit(span) => {
            blank(span)
        }
        Expr::Array(items, span) => {
            blank(span);
            for item in items {
                erase_expr_spans(item);
            }
        }
        Expr::Group(value, span) | Expr::Unary { value, span, .. } => {
            blank(span);
            erase_expr_spans(value);
        }
        Expr::Binary {
            left, right, span, ..
        } => {
            blank(span);
            erase_expr_spans(left);
            erase_expr_spans(right);
        }
        Expr::Call { target, args, span } => {
            blank(span);
            erase_expr_spans(target);
            for arg in args {
                blank(&mut arg.span);
                erase_expr_spans(&mut arg.value);
            }
        }
        Expr::Index {
            target,
            index,
            span,
        } => {
            blank(span);
            erase_expr_spans(target);
            erase_expr_spans(index);
        }
        Expr::If {
            condition,
            then_block,
            otherwise,
            span,
        } => {
            blank(span);
            erase_expr_spans(condition);
            erase_block_spans(then_block);
            if let Some(value) = otherwise {
                match &mut **value {
                    Else::Block(block) => erase_block_spans(block),
                    Else::If(expr) => erase_expr_spans(expr),
                }
            }
        }
    }
}

fn erase_block_spans(block: &mut Block) {
    block.span.file.clear();
    block.span.start = 0;
    block.span.end = 0;
    for statement in &mut block.statements {
        erase_statement_spans(statement);
    }
}

fn erase_statement_spans(statement: &mut Statement) {
    let blank = |span: &mut Span| {
        span.file.clear();
        span.start = 0;
        span.end = 0;
    };
    match statement {
        Statement::Let { value, span, .. }
        | Statement::Assign { value, span, .. }
        | Statement::Expr { value, span } => {
            blank(span);
            erase_expr_spans(value);
            if let Statement::Assign { place, .. } = statement {
                erase_expr_spans(place);
            }
        }
        Statement::Return { value, span } | Statement::Value { value, span } => {
            blank(span);
            if let Some(value) = value {
                erase_expr_spans(value);
            }
        }
        Statement::If {
            condition,
            then_block,
            otherwise,
            span,
        } => {
            blank(span);
            erase_expr_spans(condition);
            erase_block_spans(then_block);
            if let Some(otherwise) = otherwise {
                erase_statement_spans(otherwise);
            }
        }
        Statement::While {
            condition,
            body,
            span,
        } => {
            blank(span);
            erase_expr_spans(condition);
            erase_block_spans(body);
        }
        Statement::Block(block) => erase_block_spans(block),
    }
}

fn ast_without_spans(mut ast: Ast) -> Ast {
    ast.span.file.clear();
    ast.span.start = 0;
    ast.span.end = 0;
    for function in &mut ast.functions {
        function.span.file.clear();
        function.span.start = 0;
        function.span.end = 0;
        function.name_span.file.clear();
        function.name_span.start = 0;
        function.name_span.end = 0;
        for parameter in &mut function.params {
            parameter.span.file.clear();
            parameter.span.start = 0;
            parameter.span.end = 0;
        }
        if let Some(WithClause::Operations(operations)) = &mut function.with_clause {
            for operation in operations {
                operation.span.file.clear();
                operation.span.start = 0;
                operation.span.end = 0;
            }
        }
        erase_block_spans(&mut function.body);
    }
    ast
}

#[test]
fn canonical_printer_preserves_every_ast_field_except_spans() {
    let bodies = [
        "let x=1;var y:bool=true;y=false;return;",
        "let x=[1,2];x[0]=3;drop(x[1]);",
        "let x=(1+2)-3;let y=x<4;let z=x==4;",
        "if true{let x=1;}else if false{let x=2;}else{}while false{}{}",
        "if true{}else{x;}",
        "if true{}else if false{}",
        "if true{}else if false{}else{x;}",
        "let x=if true{value:1;}else{value:2;};",
        "let x=f(a:1,2)[0];let y=consume x;let z=inout y;",
        "let x=Ok(1)?;let y=();let z=[];",
        "return (1);",
    ];
    let headers = [
        "fn main()",
        "fn main(x:i64,consume y:Array<Result<i64,bool>>,inout z:MockResource)->MockBlockingResource",
        "fn main() with no_io",
        "fn main() with {call(no_alloc,no_io)}",
    ];
    for seed in 0..5_000usize {
        let body =
            bodies[seed % bodies.len()].replace("let x=1;", &format!("let x={};", seed % 100));
        let header = headers[(seed / bodies.len()) % headers.len()];
        let header = if body == "return (1);" && !header.contains("->") {
            if header.contains(" with ") {
                header.replacen(" with ", "->i64 with ", 1)
            } else {
                format!("{header}->i64")
            }
        } else {
            header.to_string()
        };
        let source = format!("{header}{{{body}}}");
        let original =
            parse("roundtrip.nx", &source).unwrap_or_else(|e| panic!("source {source}: {e:?}"));
        let printed = print_canonical(&original);
        if body.starts_with("let x=if true") {
            assert!(
                printed.contains("else {"),
                "canonical else block: {printed}"
            );
        }
        let reparsed =
            parse("roundtrip.nx", &printed).unwrap_or_else(|e| panic!("printed {printed}: {e:?}"));
        assert_eq!(
            ast_without_spans(original),
            ast_without_spans(reparsed.clone()),
            "source {source}"
        );
        assert_eq!(printed, print_canonical(&reparsed), "source {source}");
    }
}

#[test]
fn hostile_nesting_in_parens_blocks_arrays_and_types_is_bounded() {
    let sources = [
        format!(
            "fn main(){{let x={}1{};}}",
            "(".repeat(10_000),
            ")".repeat(10_000)
        ),
        format!("fn main(){{{}{}}}", "{".repeat(10_000), "}".repeat(10_000)),
        format!(
            "fn main(){{let x={}1{};}}",
            "[".repeat(10_000),
            "]".repeat(10_000)
        ),
        format!(
            "fn f(x:{}i64{}){{}}",
            "Array<".repeat(10_000),
            ">".repeat(10_000)
        ),
    ];
    for (index, source) in sources.into_iter().enumerate() {
        let diagnostic = parse("nesting.nx", &source).unwrap_err();
        assert_eq!(
            diagnostic.code,
            "NX-E0003",
            "{}",
            &source[..source.len().min(20)]
        );
        assert_eq!(
            diagnostic.data.get("budget_kind"),
            Some(&DataValue::String("syntax_nesting".into()))
        );
        assert_eq!(diagnostic.data.get("limit"), Some(&DataValue::UInt(128)));
        assert_eq!(diagnostic.data.get("observed"), Some(&DataValue::UInt(129)));
        if index == 3 {
            assert_eq!(diagnostic.primary_span.as_ref().unwrap().start, 775);
        }
        diagnostic.validate("nesting.nx", &source).unwrap();
    }
}

#[test]
fn feature_scanner_respects_token_boundaries_and_comment_trivia() {
    let valid = "// move async class unsafe match for region self Option no_heap_alloc\nfn main(){let move_count=1;let async_task=2;let class_name=3;let unsafe_flag=4;let matchable=5;let no_heap_allocator=6;let OptionValue=7;let region_name=8;let selfless=9;let no_heap_alloc=10;let result=move_count+async_task+class_name+unsafe_flag+matchable+no_heap_allocator+OptionValue+region_name+selfless+no_heap_alloc;}";
    parse("scanner.nx", valid).unwrap();
    let comptime = "fn main(){let x=comptime 1;}";
    let diagnostic = parse("scanner.nx", comptime).unwrap_err();
    assert_eq!(diagnostic.code, "NX-E0006");
    assert_eq!(diagnostic.feature_id.as_deref(), Some("comptime"));
    diagnostic.validate("scanner.nx", comptime).unwrap();
    let string = "fn main(){let x=\"move async class unsafe\";}";
    let diagnostic = parse("scanner.nx", string).unwrap_err();
    assert_eq!(diagnostic.feature_id.as_deref(), Some("text"));
    assert_eq!(
        diagnostic.primary_span.as_ref().map(|s| (s.start, s.end)),
        Some((16, 41))
    );
    diagnostic.validate("scanner.nx", string).unwrap();
}

#[test]
fn excluded_operators_keep_diagnostic_across_line_breaks() {
    for (operator, width, repaired) in [
        ("*", 1, "+"),
        ("!=", 2, "=="),
        (">", 1, "<"),
        ("<=", 2, "<"),
        (">=", 2, "<"),
    ] {
        for newline in ["", "\n", "\r\n"] {
            let source = format!("fn main(){{let x=1{newline}{operator}2;}}");
            let start = 17 + newline.len() as u64;
            let diagnostic = parse("operator.nx", &source).unwrap_err();
            assert_eq!(diagnostic.code, "NX-E0002", "{source}");
            assert_eq!(diagnostic.stage, Stage::Parse, "{source}");
            assert_eq!(
                diagnostic.parser_context.as_deref(),
                Some("unsupported_binary_operator"),
                "{source}"
            );
            assert_eq!(
                diagnostic.expected_tokens,
                ["+", "-", "<", "=="],
                "{source}"
            );
            assert_eq!(
                diagnostic.primary_span.as_ref().map(|s| (s.start, s.end)),
                Some((start, start + width)),
                "{source}"
            );
            assert_eq!(
                diagnostic
                    .unexpected_token
                    .as_ref()
                    .and_then(|t| t.text.as_deref()),
                Some(operator),
                "{source}"
            );
            assert_eq!(
                diagnostic
                    .recognized_prefix
                    .as_ref()
                    .map(|s| (s.start, s.end)),
                Some((10, start)),
                "{source}"
            );
            assert!(diagnostic.repair.is_empty(), "{source}");
            diagnostic.validate("operator.nx", &source).unwrap();
            let positive = source.replacen(operator, repaired, 1);
            parse("operator.nx", &positive).unwrap();
        }
    }
}

#[test]
fn feature_recognition_follows_parser_position_and_function_scope() {
    for source in [
        "fn f(from:i64)->i64{return from;}",
        "fn main(){let Option=1;}",
        "fn main(){let a=[1];let x=a[mock_open(1)];}",
    ] {
        assert!(parse("scope.nx", source).is_ok(), "{source}");
    }
    parse("scope.nx", "fn main(){} fn g()->i64{return 1;}").unwrap();
    let d = parse("scope.nx", "fn main(){} fn g(){return 1;}").unwrap_err();
    assert_eq!(d.feature_id.as_deref(), Some("inferred_non_unit_result"));
    assert_eq!(
        d.primary_span.as_ref().map(|s| (s.start, s.end)),
        Some((15, 16))
    );
    assert_eq!(d.related_spans[0].span.start, 19);
    let d = parse("scope.nx", "fn main(){let x=;} fn g(){for}").unwrap_err();
    assert_eq!(d.code, "NX-E0002");
    assert_eq!(
        d.unexpected_token.as_ref().and_then(|t| t.text.as_deref()),
        Some(";")
    );
}

#[test]
fn repeated_and_grouped_modes_are_classified_after_the_whole_postfix() {
    for (source, feature, range) in [
        (
            "fn main(){foo(consume a);foo(consume a[0]);}",
            "partial_transfer",
            (37, 41),
        ),
        ("fn main(){consume (a(1));}", "receiver_mode", (10, 17)),
        ("fn main(){inout f();}", "receiver_mode", (10, 15)),
        ("fn main(){consume (a)[0];}", "partial_transfer", (18, 24)),
        ("fn main(){consume (a[0]);}", "partial_transfer", (18, 24)),
        ("fn main(){consume a[0][1];}", "partial_transfer", (18, 25)),
    ] {
        let d = parse("mode.nx", source).unwrap_err();
        assert_eq!(d.feature_id.as_deref(), Some(feature), "{source}");
        assert_eq!(
            d.primary_span.as_ref().map(|s| (s.start, s.end)),
            Some(range),
            "{source}"
        );
    }
}

#[test]
fn excluded_member_selection_survives_a_line_break() {
    for source in ["fn main(){x.y;}", "fn main(){x\n.y;}"] {
        let d = parse("member.nx", source).unwrap_err();
        assert_eq!(d.feature_id.as_deref(), Some("methods"), "{source}");
    }
}

#[test]
fn ordinary_names_do_not_trigger_deferred_features() {
    let names = [
        "Option",
        "from",
        "class",
        "value",
        "move_item",
        "async_item",
        "consume_item",
    ];
    for seed in 0..2_500usize {
        let name = names[seed % names.len()];
        let source = format!(
            "fn f({name}:i64)->i64{{let Option={name};let from=Option;let class=from;let case_{seed}=class;return case_{seed};}} fn main(){{let {name}=1;}} "
        );
        parse("names.nx", &source).unwrap_or_else(|d| panic!("{seed}: {source}: {d:?}"));
    }
}

#[test]
fn function_body_opening_and_structural_closure_follow_the_production() {
    for source in [
        "fn f()->i64\n{return 1;}",
        "fn f() with no_io\n{}",
        "fn f(){let x=1\n{}}",
        "fn f(){if (if x {value:1;} else {value:2;}) {}\n x;}",
        "fn f(){while a[if true{value:0;}else{value:1;}]{}\n x;}",
    ] {
        parse("structure.nx", source).unwrap_or_else(|d| panic!("{source}: {d:?}"));
    }
    let d = parse("structure.nx", "fn f(){return }").unwrap_err();
    assert_eq!(d.code, "NX-E0005");
    assert_eq!(d.related_spans[0].role, RelatedRole::Context);
    assert_eq!(
        (d.related_spans[0].span.start, d.related_spans[0].span.end),
        (7, 13)
    );
}

#[test]
fn grammar_rejects_trailing_commas_chained_comparisons_and_if_operands() {
    type BadForm<'a> = (&'a str, &'a str, &'a [&'a str], &'a str, u64, u64);
    let rows: &[BadForm<'_>] = &[
        (
            "fn f(a:i64,){}",
            "grammar.Parameter",
            &["Name", "consume", "inout"],
            ")",
            11,
            12,
        ),
        (
            "fn f(){g(1,);}",
            "grammar.Argument",
            EXPR_START,
            ")",
            11,
            12,
        ),
        (
            "fn f(){let x=[1,];}",
            "array_element",
            EXPR_START,
            "]",
            16,
            17,
        ),
        (
            "fn f(){let x=a<b<c;}",
            "grammar.CompareExpr",
            &[";"],
            "<",
            16,
            17,
        ),
        (
            "fn f(){let x=a==b==c;}",
            "grammar.CompareExpr",
            &[";"],
            "==",
            17,
            19,
        ),
        (
            "fn f(){let x=a<b==c;}",
            "grammar.CompareExpr",
            &[";"],
            "==",
            16,
            18,
        ),
        (
            "fn f(){let x=1+if true{value:1;}else{value:2;};}",
            "grammar.TryExpr",
            TRY_START,
            "if",
            15,
            17,
        ),
        (
            "fn f(){let x=1<if true{value:1;}else{value:2;};}",
            "grammar.TryExpr",
            TRY_START,
            "if",
            15,
            17,
        ),
        (
            "fn f(){let x=if true{value:1;}else{value:2;}+1;}",
            "grammar.Expr",
            &[";"],
            "+",
            44,
            45,
        ),
        (
            "fn f(){let x=if true{value:1;}else{value:2;}?;}",
            "grammar.Expr",
            &[";"],
            "?",
            44,
            45,
        ),
    ];
    for &(source, context, expected, token, start, end) in rows {
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
            Some((start, end)),
            "{source}"
        );
        d.validate("grammar.nx", source).unwrap();
    }
    for source in [
        "fn f(a:i64){}",
        "fn f(){g(1);}",
        "fn f(){let x=[1];}",
        "fn f(){let x=a<b;}",
        "fn f(){let x=a==b;}",
        "fn f(){let x=(if true{value:1;}else{value:2;})+1;}",
    ] {
        parse("grammar.nx", source).unwrap_or_else(|d| panic!("{source}: {d:?}"));
    }
}

#[test]
fn else_block_and_else_if_keep_distinct_structure_without_spans() {
    let block = parse("else.nx", "fn f(){let x=if true{value:1;}else{value:2;};}").unwrap();
    let chained = parse(
        "else.nx",
        "fn f(){let x=if true{value:1;}else if true{value:2;};}",
    )
    .unwrap();
    assert_ne!(
        ast_without_spans(block.clone()),
        ast_without_spans(chained.clone())
    );
    assert!(print_canonical(&block).contains("else {"));
    assert!(print_canonical(&chained).contains("else if true"));
}

#[test]
fn unsupported_operator_prefix_starts_at_the_statement_token() {
    for (source, start, end) in [
        ("fn main(){ let x=1*2;}", 11, 18),
        ("fn main(){ if x {}\n a*b;}", 20, 21),
        ("fn main(){if x {} a*b;}", 18, 19),
    ] {
        let d = parse("operator.nx", source).unwrap_err();
        assert_eq!(
            d.parser_context.as_deref(),
            Some("unsupported_binary_operator")
        );
        assert_eq!(
            d.recognized_prefix.as_ref().map(|s| (s.start, s.end)),
            Some((start, end))
        );
    }
}

#[test]
fn first_sets_are_emitted_by_the_committed_production() {
    let rows: &[(&str, &str, &[&str])] = &[
        ("fn f()", "function_suffix", &["->", "with", "{"]),
        ("fn f()->i64", "function_suffix", &["with", "{"]),
        (
            "fn f() with",
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
        ("fn f() with no_io", "function_suffix", &[",", "{"]),
        (
            "fn f() with {",
            "grammar.OperationList",
            &["call", "cancel", "copy", "create", "drop", "resume", "}"],
        ),
        ("fn f(){let x", "grammar.Binding", &[":", "="]),
        ("fn f(){let x=1+", "grammar.TryExpr", TRY_START),
        ("fn f(){let x=1<", "grammar.TryExpr", TRY_START),
        ("fn f(){let x=1==", "grammar.TryExpr", TRY_START),
        ("fn f(){let x=consume", "grammar.Postfix", POSTFIX_START),
        ("fn f(){let x=inout", "grammar.Postfix", POSTFIX_START),
    ];
    for &(source, context, expected) in rows {
        let d = parse("first.nx", source).unwrap_err();
        assert_eq!(d.code, "NX-E0002", "{source}");
        assert_eq!(d.parser_context.as_deref(), Some(context), "{source}");
        assert_eq!(d.expected_tokens, expected, "{source}");
        d.validate("first.nx", source).unwrap();
    }
}

#[test]
fn exact_nesting_boundary_and_written_modes_survive_round_trip() {
    // A chain of nested blocks, rather than siblings, reaches activation 129.
    let admitted = format!("fn f(){{{}{}}}", "{".repeat(127), "}".repeat(127));
    parse("limit.nx", &admitted).unwrap();
    let excessive = format!("fn f(){{{}{}}}", "{".repeat(128), "}".repeat(128));
    let d = parse("limit.nx", &excessive).unwrap_err();
    assert_eq!(d.code, "NX-E0003");
    assert_eq!(d.data.get("limit"), Some(&DataValue::UInt(128)));
    assert_eq!(d.data.get("observed"), Some(&DataValue::UInt(129)));
    let source = "fn f(inout x:i64,consume y:i64){}";
    let ast = parse("mode.nx", source).unwrap();
    assert_eq!(ast.functions[0].params[0].mode, Mode::Inout);
    assert_eq!(ast.functions[0].params[1].mode, Mode::Consume);
    assert_eq!(
        ast_without_spans(ast.clone()),
        ast_without_spans(parse("mode.nx", &print_canonical(&ast)).unwrap())
    );
}

#[test]
fn signed_literals_and_operator_precedence_are_structural() {
    let source = "fn f(){let x=-42;let y=1+2<4;let z=(1<2)+3;let w=4-(2-1);}";
    let ast = parse("precedence.nx", source).unwrap();
    let ss = &ast.functions[0].body.statements;
    assert!(matches!(&ss[0], Statement::Let { value: Expr::Integer(n, _), .. } if n == "-42"));
    assert!(
        matches!(&ss[1], Statement::Let { value: Expr::Binary { op, left, .. }, .. } if op == "<" && matches!(&**left, Expr::Binary { op, .. } if op == "+"))
    );
    let printed = print_canonical(&ast);
    assert!(printed.contains("(1 < 2) + 3"));
    assert!(printed.contains("4 - (2 - 1)"));
    assert_eq!(
        ast_without_spans(ast),
        ast_without_spans(parse("precedence.nx", &printed).unwrap())
    );
    let ast = parse("precedence.nx", "fn f(){let x=1<2+3;let y=1-2+3;}").unwrap();
    assert!(
        matches!(&ast.functions[0].body.statements[0], Statement::Let { value: Expr::Binary { op, right, .. }, .. } if op == "<" && matches!(&**right, Expr::Binary { op, .. } if op == "+"))
    );
    assert!(print_canonical(&ast).contains("1 - 2 + 3"));
    let mut synthetic = parse("precedence.nx", "fn f(){let x=1+(2<3);}").unwrap();
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
    assert!(print_canonical(&synthetic).contains("1 + (2 < 3)"));
}

#[test]
fn related_locations_use_first_duplicate_and_full_crlf_delimiter() {
    for (source, subcause, first) in [
        (
            "fn f() with {call(no_io),call(no_alloc)}{}",
            "duplicate_operation",
            (13, 17),
        ),
        (
            "fn f() with no_io,no_io{}",
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
    let source = "fn f(){let x=if true{value:\r\n}else{value:1;};}";
    let d = parse("related.nx", source).unwrap_err();
    assert_eq!(d.subcause.as_deref(), Some("missing_value_operand"));
    let offset = source.find("\r\n").unwrap() as u64;
    assert!(
        d.related_spans
            .iter()
            .any(|r| r.role == RelatedRole::Delimiter
                && (r.span.start, r.span.end) == (offset, offset + 2))
    );
    let d = parse(
        "related.nx",
        "fn f(){let x=if true{value\r:1;}else{value:2;};}",
    )
    .unwrap_err();
    assert_eq!(d.code, "NX-E0002");
    assert_eq!(d.feature_id, None);
    // Exercise the parser's contextual check with newline trivia retained
    // in the source but without the separate terminator insertion pass.
    let source = "fn f(){let x=if true{value\r:1;}else{value:2;};}";
    let tokens = lex::lex("related.nx", source)
        .unwrap()
        .into_iter()
        .filter(|t| t.kind != Kind::Newline)
        .collect();
    let d = Parser {
        file: "related.nx",
        source,
        tokens,
        at: 0,
        statement_start: 0,
        split_type_close: None,
        depth: 0,
        function_names: HashSet::new(),
        local_scopes: Vec::new(),
    }
    .program()
    .unwrap_err();
    // §3.1: the CR ends the expression; ':' begins an invalid statement.
    assert_eq!(d.code, "NX-E0002");
}

#[test]
fn statement_local_newlines_never_invent_source_tokens() {
    for source in [
        "fn main(){let a=(if c {\nvalue: 1\n} else {\nvalue: 2\n});}",
        "fn main(){let x\n=1;}",
        "fn main(){let x:i64\n=1;}",
        "fn f()\n-> i64{return 1;}",
        "fn f()\nwith no_io{}",
    ] {
        parse("newline.nx", source).unwrap_or_else(|d| panic!("{source}: {d:?}"));
    }
    let source = "fn main(){let x\n: i64\n=1; let y=2\nlet z=3;}";
    parse("newline.nx", source).unwrap();
    for source in [
        "fn main(){x\n, 1;}",
        "fn main(){x\nelse{}}",
        "fn main(){f(x\ny);}",
    ] {
        let d = parse("newline.nx", source).unwrap_err();
        assert_eq!(d.code, "NX-E0002", "{source}");
        let t = d.unexpected_token.unwrap();
        assert!(t.span.start < t.span.end, "{source}");
    }
}

#[test]
fn try_binds_after_access_and_has_no_postfix_continuation() {
    for source in [
        "fn main(){let y=consume x?;}",
        "fn main(){let y=inout a[0]?;}",
        "fn main(){f(inout x?);}",
    ] {
        parse("try.nx", source).unwrap_or_else(|d| panic!("{source}: {d:?}"));
    }
    for token in ["?", "(", "["] {
        let source = format!("fn main(){{a?{token}1;}}");
        let d = parse("try.nx", &source).unwrap_err();
        assert_eq!(d.code, "NX-E0002");
        assert_eq!(d.parser_context.as_deref(), Some("grammar.TryExpr"));
        assert_eq!(
            d.unexpected_token.as_ref().unwrap().text.as_deref(),
            Some(token)
        );
    }
}

#[test]
fn first_state_is_independent_of_the_offending_token() {
    let prefixes = [
        "fn",
        "fn f(",
        "fn f(x:",
        "fn f(x:Array<",
        "fn f(x:Result<i64,",
        "fn f(x:i64)",
        "fn f(x:i64)->Array<",
        "fn f()->",
        "fn f() with no_io",
        "fn f() with {call(no_io)}",
        "fn f() with {call(no_io),copy(no_alloc),",
        "fn main(",
        "fn main(){",
        "fn main(){1+",
        "fn main(){inout ",
        "fn main(){if true{}else ",
        "fn f() with ",
        "fn f() with {",
        "fn f() with {call",
        "fn f() with {call(",
        "fn f() with {call(no_io",
        "fn f() with {call(no_io)",
        "fn f() with {call(no_io),",
        "fn f() with {copy(",
        "fn f() with {call(no_io),copy",
        "fn f() with {call(no_io),copy(",
        "fn f() with {call(no_io),copy(no_alloc",
        "fn f() with {call(no_io),copy(no_alloc)",
        "fn f() with no_io,",
        "fn f(){let ",
        "fn f(){let x",
        "fn f(){let x:",
        "fn f(){let x:i64",
        "fn f(){var x=(",
        "fn f(){x=",
        "fn f(){x=(",
        "fn f(){a[",
        "fn f(){f(a:",
        "fn f(){let x=consume ",
        "fn f(){let x=1+",
        "fn f(){let x=1<",
        "fn f(){if true",
        "fn f(){while true",
        "fn f(){let x=if ",
        "fn f(){while ",
        "fn f(){return (",
        "fn main(){let a=(",
        "fn main(){let a=[",
        "fn f(x:i64,",
        "fn main(){1;",
        "fn f(x:i64",
        "fn main(){let x=",
        "fn main(){f(",
        "fn main(){f(1,",
        "fn main(){let a=[1,",
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
fn erroneous_tokens_use_literal_first_sets_of_committed_states() {
    let rows = [
        ("fn main(;", "parameter_list", ") Name consume inout"),
        (
            "fn main(){;}",
            "block_item",
            "( - IntLiteral Name [ consume false if inout let return true var while { }",
        ),
        (
            "fn main(){1+;}",
            "grammar.TryExpr",
            "( - IntLiteral Name [ consume false inout true",
        ),
        (
            "fn main(){inout ;}",
            "grammar.Postfix",
            "( - IntLiteral Name [ false true",
        ),
        ("fn main(){if true{}else ;}", "grammar.IfForm", "if {"),
        (
            "fn f() with ;",
            "grammar.WithClause",
            "no_alloc no_block no_heap_alloc no_io no_suspend unrestricted {",
        ),
        (
            "fn main(){let a=(;}",
            "grammar.Primary",
            "( ) - IntLiteral Name [ consume false if inout true",
        ),
        (
            "fn main(){let a=[;}",
            "grammar.Primary",
            "( - IntLiteral Name [ ] consume false if inout true",
        ),
    ];
    for (source, context, expected) in rows {
        let d = parse("first.nx", source).unwrap_err();
        assert_eq!(d.code, "NX-E0002", "{source}");
        assert_eq!(d.parser_context.as_deref(), Some(context), "{source}");
        assert_eq!(d.expected_tokens.join(" "), expected, "{source}");
        assert_eq!(
            d.unexpected_token.as_ref().unwrap().text.as_deref(),
            Some(";")
        );
        d.validate("first.nx", source).unwrap();
    }
}

#[test]
fn first_consistency_across_nested_grammar_positions() {
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
        "fn f(inout x:Array<Result<i64,bool>>,consume y:i64)->Result<i64,bool> with no_io {let a=[1,2];let b=if true{value:1;}else if false{value:2;}else{value:3;};while false{a[0]=a[1]+1;}return Ok(b);} fn main(){}",
        "fn f() with {call(no_io,no_block)}{} fn main(){}",
        "fn f(x:i64){} fn main(){var a=[1];(a)[0]=1;f(a[0]);}",
        "fn f(){if true{}else if false{}else{}while false{} {let x=(1);}}",
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
fn local_bindings_and_parameters_shadow_function_names() {
    for source in [
        "fn a(){} fn main(){let a=1;let b=a;}",
        "fn a(){} fn main(a:i64){let b=a;}",
        "fn a(){} fn main(){{let a=1;let b=a;}a();}",
    ] {
        parse("scope.nx", source).unwrap();
    }
    for source in [
        "fn f(){} fn main(){let g=f;}",
        "fn f(){} fn main(){let g=(f);}",
        "fn f(){} fn main(){g(f);}",
    ] {
        let d = parse("scope.nx", source).unwrap_err();
        assert_eq!(d.feature_id.as_deref(), Some("function_value"), "{source}");
        assert_eq!(d.stage, Stage::Names);
    }
}

#[test]
fn expression_statement_does_not_charge_an_outer_expr_entry() {
    for n in [126, 127] {
        parse(
            "nest.nx",
            &format!("fn main(){{{}1{};}}", "f(".repeat(n), ")".repeat(n)),
        )
        .unwrap();
        parse(
            "nest.nx",
            &format!("fn main(){{{}1{};}}", "[".repeat(n), "]".repeat(n)),
        )
        .unwrap();
    }
    for n in [128, 129] {
        for (open, close) in [("f(", ")"), ("[", "]")] {
            let d = parse(
                "nest.nx",
                &format!("fn main(){{{}1{};}}", open.repeat(n), close.repeat(n)),
            )
            .unwrap_err();
            assert_eq!(
                d.data.get("budget_kind"),
                Some(&DataValue::String("syntax_nesting".into()))
            );
        }
    }
}

#[test]
fn prefix_place_reports_first_non_place_token() {
    for (source, token) in [
        ("fn main(){let x=consume (a+1);}", "+"),
        ("fn main(){let x=consume (a?);}", "?"),
        ("fn main(){let x=consume (a<b);}", "<"),
        ("fn main(){let x=inout (a+b)[0];}", "+"),
        ("fn main(){let x=consume (1+2);}", "1"),
    ] {
        let d = parse("place.nx", source).unwrap_err();
        assert_eq!(d.code, "NX-E0002", "{source}");
        assert_eq!(
            d.unexpected_token.as_ref().unwrap().text.as_deref(),
            Some(token)
        );
        assert_eq!(
            d.primary_span.as_ref().unwrap().start as usize,
            source.find(token).unwrap()
        );
        assert_eq!(
            d.recognized_prefix.as_ref().unwrap().end,
            d.primary_span.as_ref().unwrap().start
        );
    }
}

#[test]
fn repeated_mode_prefix_is_rejected_before_recursive_descent() {
    for mode in ["consume", "inout"] {
        let source = format!("fn main(){{let x={mode} {mode} a;}}");
        let d = parse("prefix.nx", &source).unwrap_err();
        assert_eq!(d.code, "NX-E0002");
        assert_eq!(d.parser_context.as_deref(), Some("grammar.Postfix"));
        assert_eq!(
            d.expected_tokens,
            ["(", "-", "IntLiteral", "Name", "[", "false", "true"]
        );
        assert_eq!(
            d.unexpected_token.as_ref().unwrap().text.as_deref(),
            Some(mode)
        );
        assert_eq!(
            d.primary_span.as_ref().unwrap().start as usize,
            source.rfind(mode).unwrap()
        );
    }
}

#[test]
fn non_unit_returns_are_found_in_source_order_through_all_branches() {
    for source in [
        "fn f(){if a{return 1;}}",
        "fn f(){if a{return 1;}else{}}",
        "fn f(){if a{return 1;}else if b{}}",
        "fn f(){while true{return 1;}}",
        "fn f(){if a{}else if b{return 1;}}",
        "fn f(){if a{}else if b{}else{return 1;}}",
        "fn f(){let x=if a{return 1;}else{value:2;};}",
        "fn f(){{return 1;}}",
        "fn f(){let x=if a{value:0;}else if b{return 1;};}",
    ] {
        let d = parse("return.nx", source).unwrap_err();
        assert_eq!(
            d.feature_id.as_deref(),
            Some("inferred_non_unit_result"),
            "{source}: {d:?}"
        );
        assert_eq!(d.related_spans.len(), 1);
        assert_eq!(
            d.related_spans[0].span.start as usize,
            source.find("return 1").unwrap()
        );
    }
}

#[test]
fn split_type_close_and_first_states_have_regressions() {
    for source in [
        "fn f(){let a:Array<i64>=x;}",
        "fn f(){let a:Result<Array<Array<i64>>,bool>=x;}",
    ] {
        parse("split.nx", source).unwrap();
    }
    for token in [
        ")", "]", "{", ",", "=", ":", "->", "with", "else", "let", "return", "while", "*", ">",
        "+", "==",
    ] {
        let source = format!("fn a()->i64{{return {token}");
        let d = parse("first.nx", &source).unwrap_err();
        assert_eq!(d.code, "NX-E0005", "{source}");
        let source = format!("fn a(){{let g=if c{{value: {token}");
        let d = parse("first.nx", &source).unwrap_err();
        assert_eq!(d.code, "NX-E1302", "{source}");
    }
    for token in ["foo", "i64", "value", "Array", "1", ";", ")", "="] {
        let source = format!("fn a() with {token}");
        let d = parse("first.nx", &source).unwrap_err();
        assert_eq!(
            d.parser_context.as_deref(),
            Some("grammar.WithClause"),
            "{source}"
        );
        assert_eq!(d.expected_tokens, WITH_FIRST, "{source}");
    }
    let d = parse("first.nx", "fn a(){let x=1;fn b(){}").unwrap_err();
    assert_eq!(d.parser_context.as_deref(), Some("grammar.Block"));
    assert_eq!(d.expected_tokens, BLOCK_NEXT);
}

#[test]
fn nested_if_conditions_finish_on_a_small_debug_stack() {
    std::thread::Builder::new()
        .stack_size(2 * 1024 * 1024)
        .spawn(|| {
            let source = format!(
                "fn f(){{let x={}true{{}}{};}}",
                "if ".repeat(128),
                "{}".repeat(128)
            );
            let diagnostic = parse("stack.nx", &source).unwrap_err();
            assert_eq!(diagnostic.code, "NX-E0003");
            assert_eq!(
                diagnostic.data.get("budget_kind"),
                Some(&DataValue::String("syntax_nesting".into()))
            );
        })
        .unwrap()
        .join()
        .unwrap();
}

#[test]
fn parser_small_stack_probe_child() {
    let Ok(spec) = std::env::var("NEXO_PARSER_STACK_PROBE") else {
        return;
    };
    let mut parts = spec.split(':');
    let family = parts.next().unwrap();
    let depth: usize = parts.next().unwrap().parse().unwrap();
    let stack_kib: usize = parts.next().unwrap().parse().unwrap();
    let source = match family {
        "paren" => format!(
            "fn f(){{let x={}1{};}}",
            "(".repeat(depth),
            ")".repeat(depth)
        ),
        "array" => format!(
            "fn f(){{let x={}1{};}}",
            "[".repeat(depth),
            "]".repeat(depth)
        ),
        "block" => format!("fn f(){{{}{}}}", "{".repeat(depth), "}".repeat(depth)),
        "type" => format!(
            "fn f(x:{}i64{}){{}}",
            "Array<".repeat(depth),
            ">".repeat(depth)
        ),
        "mixed" => format!(
            "fn f(){{let x={}1{};}}",
            "([".repeat(depth / 2),
            "])".repeat(depth / 2)
        ),
        "if" => format!(
            "fn f(){{let x={}true{{}}{};}}",
            "if ".repeat(depth),
            "{}".repeat(depth)
        ),
        _ => unreachable!(),
    };
    let probe = move || match parse("stack-probe.nx", &source) {
        Ok(_) => (),
        Err(d) => {
            assert!(
                matches!(d.code.as_str(), "NX-E0002" | "NX-E0003" | "NX-E0005"),
                "{d:?}"
            );
            d.validate("stack-probe.nx", &source).unwrap();
        }
    };
    if stack_kib == 0 {
        probe();
        return;
    }
    std::thread::Builder::new()
        .stack_size(stack_kib * 1024)
        .spawn(probe)
        .unwrap()
        .join()
        .unwrap();
}

#[test]
fn parser_small_stack_probe_survives_nested_forms() {
    // A stack overflow aborts the process on Windows, so each case runs in
    // a child test process. The parent observes its exit status.
    let binary = std::env::current_exe().unwrap();
    for family in ["paren", "array", "block", "type", "mixed", "if"] {
        for depth in [1, 2, 4, 7, 8, 15, 16, 31, 32, 33, 63, 126, 127] {
            for stack_kib in [0, 256, 512] {
                let spec = format!("{family}:{depth}:{stack_kib}");
                let output = std::process::Command::new(&binary)
                    .args([
                        "--exact",
                        "frontend::parse::tests::parser_small_stack_probe_child",
                    ])
                    .env("NEXO_PARSER_STACK_PROBE", &spec)
                    .output()
                    .unwrap();
                assert!(
                    output.status.success(),
                    "{spec}: {}",
                    String::from_utf8_lossy(&output.stdout)
                );
            }
        }
    }
}

#[test]
fn adr_gap_incomplete_control_and_postfix_contexts_keep_current_result() {
    // §13.14 rules 3–4 do not give exact rows for these incomplete children.
    for source in [
        "fn f(){while",
        "fn f(){if",
        "fn f(){if a{}else if",
        "fn f(){a[",
        "fn f(){f(a:",
    ] {
        let d = parse("gap.nx", source).unwrap_err();
        assert_eq!(d.code, "NX-E0002", "{source}");
        assert_eq!(
            d.parser_context.as_deref(),
            Some("grammar.Primary"),
            "{source}"
        );
    }
    for source in ["fn f(){let x=consume", "fn f(){let x=inout"] {
        let d = parse("gap.nx", source).unwrap_err();
        assert_eq!(
            d.parser_context.as_deref(),
            Some("grammar.Postfix"),
            "{source}"
        );
    }
}

#[test]
fn adr_gap_completed_if_inside_delimiters_keeps_current_terminator() {
    // §3.1 and §13.14 do not assign a diagnostic row after this nullable IfForm.
    for source in [
        "fn f(){let x=[if c{value:1;}+",
        "fn f(){let x=(if c{value:1;}?",
        "fn f(){g(if c{value:1;}+",
    ] {
        let d = parse("gap.nx", source).unwrap_err();
        assert_eq!(d.code, "NX-E0002", "{source}: {d:?}");
        assert_eq!(d.expected_tokens, [";"], "{source}");
    }
}

#[test]
fn missing_terminator_relates_the_preceding_expression() {
    let source = "fn main(){let x=1}";
    let d = parse("terminator.nx", source).unwrap_err();
    assert_eq!(d.code, "NX-E0005");
    assert_eq!(d.related_spans[0].role, RelatedRole::PrecedingExpression);
    assert_eq!(
        (d.related_spans[0].span.start, d.related_spans[0].span.end),
        (16, 17)
    );
}

#[test]
fn grouping_does_not_increase_expression_tree_height() {
    let admitted = format!("fn main(){{let x=(1{});}}", "+1".repeat(127));
    parse("height.nx", &admitted).unwrap();
    let grouped_child = format!("fn main(){{let x=(1{})+1;}}", "+1".repeat(126));
    parse("height.nx", &grouped_child).unwrap();
    let excessive = format!("fn main(){{let x=(1{});}}", "+1".repeat(128));
    let d = parse("height.nx", &excessive).unwrap_err();
    assert_eq!(
        d.data.get("budget_kind"),
        Some(&DataValue::String("expression_tree_depth".into()))
    );
}

#[test]
fn adr_gap_signed_name_minus_keeps_current_token_and_prefix() {
    // §3 defines '-' only in IntLiteral; §13.14 does not fix the
    // non-EOF diagnostic when a Name follows it.
    let source = "fn main(){let x=-name;}";
    let d = parse("gap.nx", source).unwrap_err();
    assert_eq!(d.code, "NX-E0002");
    assert_eq!(d.parser_context.as_deref(), Some("signed_integer"));
    assert_eq!(
        d.unexpected_token.as_ref().unwrap().text.as_deref(),
        Some("-")
    );
    assert_eq!(d.recognized_prefix.as_ref().unwrap().start, 16);
}

#[test]
fn adr_gap_committed_rhs_first_preserves_current_inner_productions() {
    // §13.14 fixes the committed-production rule, but not every
    // non-EOF production name at an absent RHS operand.
    for (source, context) in [
        ("fn main(){var x=0;x=;}", "grammar.Primary"),
        ("fn main(){let x=1+;}", "grammar.TryExpr"),
        ("fn main(){let x=1<;}", "grammar.TryExpr"),
    ] {
        let d = parse("gap.nx", source).unwrap_err();
        assert_eq!(d.code, "NX-E0002", "{source}");
        assert_eq!(d.parser_context.as_deref(), Some(context), "{source}");
    }
}

#[test]
fn adr_gap_type_unit_terminal_and_lexical_priority_remain_explicit() {
    // §3 spells the terminal as "()" while the parser commits '(' then ')'.
    let d = parse("gap.nx", "fn f(x:(;").unwrap_err();
    assert_eq!(d.parser_context.as_deref(), Some("grammar.Type"));
    assert_eq!(d.expected_tokens, [")"]);
    // §3.2 leaves the cross-stage priority after a lexically bad suffix open.
    let d = parse("gap.nx", "fn main(){let x=0.5/1;}").unwrap_err();
    assert_eq!(d.code, "NX-E0001");
}

#[test]
fn adr_gap_branch_marker_and_scope_repairs_keep_current_boundaries() {
    // §3.1/§14 distinguish missing terminator from missing value operand.
    let d = parse("gap.nx", "fn main(){let x=if true{value:1}else{value:2;};}").unwrap_err();
    assert_eq!(d.code, "NX-E0005");
    // F26 removes the complete loop-control statement, including ';'.
    parse("gap.nx", "fn main(){while false{}}").unwrap();
    let d = parse("gap.nx", "fn main(){while false{;}}").unwrap_err();
    assert_eq!(d.code, "NX-E0002");
    // F06/F10/F16–F19/F27/F28/F32 repairs permit trivia left behind.
    parse("gap.nx", "  \n fn main(){ 1; }").unwrap();
}

#[test]
fn adr_gap_nullable_continuations_and_else_if_depth_keep_current_results() {
    // §3.1 leaves nullable list continuations and stray comma/else
    // classifications to the production state.
    parse("gap.nx", "fn main(){f(x\n, 1);}").unwrap();
    for source in ["fn main(){x\n, 1;}", "fn main(){x\nelse{}}"] {
        assert_eq!(parse("gap.nx", source).unwrap_err().code, "NX-E0002");
    }
    let source = format!("fn main(){{{}{{}}}}", "if true{}else ".repeat(128));
    let d = parse("gap.nx", &source).unwrap_err();
    assert_eq!(
        d.data.get("budget_kind"),
        Some(&DataValue::String("expression_tree_depth".into()))
    );
}

#[test]
fn adr_gap_nonterminating_subexpression_follow_keeps_current_result() {
    // §3 CompareExpr/AddExpr/TryExpr and §13.14 leave the diagnostic
    // classification after a complete condition and a stray follower open.
    for (source, context, expected) in [
        ("fn f(){if a<1", "grammar.Block", &["{"][..]),
        ("fn f(){if a<1 ;", "grammar.Block", &["{"][..]),
        ("fn f(){if a<1 )", "grammar.Block", &["{"][..]),
        ("fn f(){if a<1 <", "grammar.CompareExpr", &[";"][..]),
        ("fn f(){if a<1 ==", "grammar.CompareExpr", &[";"][..]),
    ] {
        let d = parse("gap.nx", source).unwrap_err();
        assert_eq!(d.code, "NX-E0002", "{source}");
        assert_eq!(d.parser_context.as_deref(), Some(context), "{source}");
        assert_eq!(d.expected_tokens, expected, "{source}");
        d.validate("gap.nx", source).unwrap();
    }
}
