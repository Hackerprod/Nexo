//! Literal parser decisions from F0-D15 §§8.7 and 8.11.
use super::parse::parse;
use crate::diagnostic::{
    ExpectedTokensKind, Namespace, Operation, RelatedRole, RepairStatus, Severity, Stage, TokenKind,
};

fn coordinates(span: &crate::diagnostic::Span) -> (u64, u64) {
    (span.start, span.end)
}

fn common(
    error: &crate::diagnostic::Diagnostic,
    source: &str,
    code: &str,
    sub: Option<&str>,
    primary: (u64, u64),
) {
    assert_eq!(error.schema_version, 1, "{source}");
    assert_eq!(
        error.diagnostic_namespace,
        Namespace::NexoProposedV1,
        "{source}"
    );
    assert_eq!(error.profile_id, "f0-0002-b-v2", "{source}");
    assert_eq!(error.code, code, "{source}");
    assert_eq!(error.subcause.as_deref(), sub, "{source}");
    let rule = sub.map_or_else(|| code.to_owned(), |sub| format!("{code}/{sub}"));
    assert_eq!(error.rule, rule, "{source}");
    assert_eq!(
        error.explanation_id.as_deref(),
        Some(rule.as_str()),
        "{source}"
    );
    assert_eq!(error.severity, Severity::Error, "{source}");
    assert_eq!(error.stage, Stage::Parse, "{source}");
    assert_eq!(error.operation, Operation::SyntaxParse, "{source}");
    assert_eq!(
        coordinates(error.primary_span.as_ref().unwrap()),
        primary,
        "{source}"
    );
    assert!(error.cause_chain.is_empty(), "{source}");
    assert!(error.validate("rows.n", source).is_ok(), "{source}");
}

#[test]
fn literal_branch_and_contract_rows_have_complete_parser_diagnostics() {
    // F2-B01/B04/B05/B06 and F2-W01…W07: literal §8.7 sources and byte spans.
    let rows = [
        (
            "F2-B01",
            "fn F(b:view Bool)->I64{return if b then {value:1;};} fn Main(){}",
            "NX-E1302",
            "missing_else",
            (30, 32),
            &[][..],
        ),
        (
            "F2-B04",
            "fn Main(){value:1;}",
            "NX-E1302",
            "invalid_value_target",
            (10, 16),
            &[(RelatedRole::Context, (9, 10))][..],
        ),
        (
            "F2-B05",
            "fn F()->I64{return if true then {value:}else{value:2;};} fn Main(){}",
            "NX-E1302",
            "missing_value_operand",
            (33, 39),
            &[
                (RelatedRole::Context, (32, 33)),
                (RelatedRole::Delimiter, (39, 40)),
            ],
        ),
        (
            "F2-B06",
            "fn F()->I64{return if true then {value:1;2;}else{value:2;};} fn Main(){}",
            "NX-E1302",
            "non_final_branch_value",
            (33, 39),
            &[(RelatedRole::Context, (41, 43))],
        ),
        (
            "F2-W01",
            "fn Main() with {call(no_io),call(no_alloc)}{}",
            "NX-E0417",
            "duplicate_operation",
            (28, 32),
            &[(RelatedRole::Context, (16, 20))],
        ),
        (
            "F2-W02",
            "fn Main() with {resume(no_io)}{}",
            "NX-E0417",
            "invalid_operation",
            (16, 22),
            &[(RelatedRole::Context, (3, 7))],
        ),
        (
            "F2-W03",
            "fn Main() with no_io,no_io{}",
            "NX-E0417",
            "duplicate_restriction",
            (21, 26),
            &[(RelatedRole::Context, (15, 20))],
        ),
        (
            "F2-W04",
            "fn Main() with {}{}",
            "NX-E0417",
            "empty_operation_list",
            (15, 17),
            &[],
        ),
        (
            "F2-W05",
            "fn Main() with unrestricted,no_io{}",
            "NX-E0417",
            "unrestricted_mixed",
            (15, 33),
            &[],
        ),
        (
            "F2-W06",
            "fn Main() with {call(no_io,unrestricted)}{}",
            "NX-E0417",
            "unrestricted_mixed",
            (21, 39),
            &[],
        ),
        (
            "F2-W07",
            "fn Main() with unrestricted,unrestricted,no_io{}",
            "NX-E0417",
            "duplicate_restriction",
            (28, 40),
            &[(RelatedRole::Context, (15, 27))],
        ),
    ];
    for (id, source, code, sub, primary, context) in rows {
        let error = parse("rows.n", source).unwrap_err();
        common(&error, source, code, Some(sub), primary);
        assert_eq!(error.feature_id, None, "{id}");
        assert_eq!(error.unexpected_token, None, "{id}");
        assert!(error.expected_tokens.is_empty(), "{id}");
        assert_eq!(error.expected_tokens_kind, None, "{id}");
        assert_eq!(error.parser_context, None, "{id}");
        assert_eq!(error.recognized_prefix, None, "{id}");
        assert!(error.repair.is_empty(), "{id}");
        assert!(error.data.is_empty(), "{id}");
        assert_eq!(error.related_spans.len(), context.len(), "{id}");
        for (actual, &(role, span)) in error.related_spans.iter().zip(context) {
            assert_eq!(actual.role, role, "{id}");
            assert_eq!(coordinates(&actual.span), span, "{id}");
        }
    }
}

#[test]
fn literal_line_break_row_has_complete_parser_diagnostic() {
    // F2-L02 (§8.11): also checks the newline delimiter.
    let source =
        "fn F()->I64{\nreturn if true then {\nvalue:\n(1);\n}else{value:2;};\n}\nfn Main(){}";
    let error = parse("rows.n", source).unwrap_err();
    common(
        &error,
        source,
        "NX-E1302",
        Some("missing_value_operand"),
        (35, 41),
    );
    assert_eq!(error.related_spans.len(), 2);
    assert_eq!(error.related_spans[0].role, RelatedRole::Context);
    assert_eq!(coordinates(&error.related_spans[0].span), (33, 34));
    assert_eq!(error.related_spans[1].role, RelatedRole::Delimiter);
    assert_eq!(coordinates(&error.related_spans[1].span), (41, 42));
    assert_eq!(error.feature_id, None);
    assert_eq!(error.unexpected_token, None);
    assert!(error.expected_tokens.is_empty());
    assert_eq!(error.expected_tokens_kind, None);
    assert_eq!(error.parser_context, None);
    assert_eq!(error.recognized_prefix, None);
    assert!(error.repair.is_empty());
    assert!(error.data.is_empty());
}

#[test]
fn literal_terminator_rows_and_supplements_have_complete_parser_diagnostics() {
    // F2-B07/B08 in §8.7; the same two IDs have supplemental rows in §8.11.
    let rows = [
        (
            "F2-B07",
            "fn F()->I64{return 1 } fn Main(){}",
            (21, 22),
            (12, 21),
            (19, 20),
            (20, 20),
        ),
        (
            "F2-B08",
            "fn F()->I64{return if true then {value:1 }else{value:2;};} fn Main(){}",
            (41, 42),
            (33, 41),
            (39, 40),
            (40, 40),
        ),
    ];
    for (id, source, primary, prefix, preceding, insertion) in rows {
        let error = parse("rows.n", source).unwrap_err();
        common(&error, source, "NX-E0005", None, primary);
        assert_eq!(error.feature_id, None, "{id}");
        assert_eq!(error.related_spans.len(), 1, "{id}");
        assert_eq!(
            error.related_spans[0].role,
            RelatedRole::PrecedingExpression,
            "{id}"
        );
        assert_eq!(coordinates(&error.related_spans[0].span), preceding, "{id}");
        let token = error.unexpected_token.as_ref().unwrap();
        assert_eq!(token.kind, TokenKind::Symbol, "{id}");
        assert_eq!(token.text.as_deref(), Some("}"), "{id}");
        assert_eq!(coordinates(&token.span), primary, "{id}");
        assert_eq!(error.expected_tokens, [";"], "{id}");
        assert_eq!(
            error.expected_tokens_kind,
            Some(ExpectedTokensKind::CanonicalInsertion),
            "{id}"
        );
        assert_eq!(
            error.parser_context.as_deref(),
            Some("statement_terminator"),
            "{id}"
        );
        assert_eq!(
            coordinates(error.recognized_prefix.as_ref().unwrap()),
            prefix,
            "{id}"
        );
        assert_eq!(error.repair.len(), 1, "{id}");
        let repair = &error.repair[0];
        assert_eq!(repair.id, "insert_terminator", "{id}");
        assert_eq!(repair.status, RepairStatus::Proposed, "{id}");
        assert_eq!(repair.validation, None, "{id}");
        assert_eq!(repair.edits.len(), 1, "{id}");
        assert_eq!(coordinates(&repair.edits[0].span), insertion, "{id}");
        assert_eq!(repair.edits[0].expected_text, "", "{id}");
        assert_eq!(repair.edits[0].replacement, ";", "{id}");
        assert_eq!(
            repair.consequences,
            ["Termina la sentencia antes del siguiente token."],
            "{id}"
        );
        assert!(error.data.is_empty(), "{id}");
    }
}

#[test]
fn later_stage_supplement_rows_keep_the_expected_ast_roots() {
    // §8.11 semantic/execution rows: L01, L03…L07,
    // L07 CRLF/CR/LF-final, PP06, 14.5-1, 14.5-5, P04…P07.
    let l07 = "// á🙂\nfn Main(){\nlet x=missing;\n}";
    let sources: &[(&str, &str, &[&str])] = &[
        (
            "F2-L01",
            "fn F()->I64{\nreturn\n(1);\n}\nfn Main(){}",
            &["F", "Main"],
        ),
        (
            "F2-L03",
            "fn value(x:view I64)->I64{return x;}\nfn Main(){let x=7;let y=value\n(x);AssertEq(y,7);}",
            &["value", "Main"],
        ),
        (
            "F2-L04",
            "fn Main(){\nlet value=2\nlet a=value -1\nlet b=value - 1\nAssertEq(a,b)\n}",
            &["Main"],
        ),
        (
            "F2-L05",
            "fn Main(){\nlet x=if true then {value: ();}else{value: ();};\nAssertEq(x,());\n}",
            &["Main"],
        ),
        ("F2-L06", "fn F()->I64{\n1\n}\nfn Main(){}", &["F", "Main"]),
        ("F2-L07", l07, &["Main"]),
        (
            "F2-L07 CRLF",
            "// á🙂\r\nfn Main(){\r\nlet x=missing;\r\n}",
            &["Main"],
        ),
        (
            "F2-L07 CR",
            "// á🙂\rfn Main(){\rlet x=missing;\r}",
            &["Main"],
        ),
        (
            "F2-L07 LF con LF final",
            "// á🙂\nfn Main(){\nlet x=missing;\n}\n",
            &["Main"],
        ),
        (
            "F2-PP06",
            "fn Print(x:view I64)->I64{return x+1;}\nfn Main() with no_io{let n=Print(2);AssertEq(n,3);}",
            &["Print", "Main"],
        ),
        (
            "F2-14.5-1",
            "fn F(r:move MockResource,n:view I64){}\nfn Main(){F(MockOpen(1),9223372036854775807+1);}",
            &["F", "Main"],
        ),
        (
            "F2-14.5-5",
            "fn Main(){let r=MockOpen(1);var a=[0];a[-1]=9223372036854775807+1;}",
            &["Main"],
        ),
        (
            "F2-P04",
            "fn F(x:view MockResource)->MockResource{return move x;} fn Main(){}",
            &["F", "Main"],
        ),
        (
            "F2-P05",
            "fn F(x:edit I64,y:edit I64){} fn Main(){var x=1;F(edit x,edit x);}",
            &["F", "Main"],
        ),
        (
            "F2-P06",
            "fn F(x:view I64,y:edit I64){} fn Main(){var x=1;F(x,edit x);}",
            &["F", "Main"],
        ),
        (
            "F2-P07",
            "fn F(x:edit I64,y:edit I64){} fn Main(){var a=[1,2];F(edit a[0],edit a[1]);}",
            &["F", "Main"],
        ),
    ];
    assert_eq!(sources.len(), 16);
    for (id, source, names) in sources {
        let ast = parse("rows.n", source).unwrap_or_else(|e| panic!("{id}: {e:?}"));
        assert_eq!(
            ast.functions
                .iter()
                .map(|f| f.name.as_str())
                .collect::<Vec<_>>(),
            *names,
            "{id}"
        );
    }
}

#[test]
fn partial_transfer_argument_row_has_indexed_ast_pending_cp3() {
    // F2-S29 is arguments stage: parser B must accept Drop(a[0]) as a call
    // with an indexed place; semantic analysis decides NX-E0006/partial_transfer.
    use crate::frontend::parse::{Expr, Statement};
    let source = "fn Main(){let a=[1];Drop(a[0]);}";
    let ast = parse("rows.n", source).unwrap();
    assert_eq!(ast.functions.len(), 1);
    assert_eq!(ast.functions[0].name, "Main");
    assert_eq!(ast.functions[0].body.statements.len(), 2);
    let Statement::Expr {
        value: Expr::Call { args, .. },
        ..
    } = &ast.functions[0].body.statements[1]
    else {
        panic!("F2-S29: expected call AST");
    };
    assert_eq!(args.len(), 1);
    assert!(matches!(args[0].value, Expr::Index { .. }));
}
