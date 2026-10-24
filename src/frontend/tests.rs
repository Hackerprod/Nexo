use super::parse::Statement;
use super::parse::parse;
use super::print::print_canonical;
use super::test_normalize::ast_without_spans;

// Each line identifies one deliberately rejected source and its parser reason.
// The closed lists prevent a parser regression from silently shrinking a campaign.
fn assert_closed_rejections(actual: &[String], expected: &str) {
    assert_eq!(
        actual.iter().map(String::as_str).collect::<Vec<_>>(),
        expected.lines().collect::<Vec<_>>()
    );
}

#[test]
fn omitted_token_comments_keep_selection_and_terminator() {
    let cases = [
        "fn M(){x=if a then 1 else ((if c then 1 else 2));\ny=if a then{//\nvalue:2;}else 1;}",
        "fn M(){w=if M then e else ((if M then{}else a)\n//\n);}",
        "fn M(){let t=if a then 1 else{value:2\n}//\n;}",
    ];
    let expected = [
        "fn M() {\n    x = if a then 1 else (if c then 1 else 2);\n    y = if a then { //\n        value: 2;\n    } else 1;\n}\n",
        "fn M() {\n    w = if M then e else (if M then {} else a\n    //\n    );\n}\n",
        "fn M() {\n    let t = if a then 1 else 2; //\n}\n",
    ];
    for (source, expected) in cases.into_iter().zip(expected) {
        for written in [
            source.to_owned(),
            source.replace('\n', "\r"),
            source.replace('\n', "\r\n"),
        ] {
            let original = parse("omitted-token.n", &written).unwrap();
            let formatted = super::format::format("omitted-token.n", &written).unwrap();
            assert_eq!(formatted, expected, "{written:?}");
            let reparsed = parse("omitted-token.n", &formatted)
                .unwrap_or_else(|error| panic!("{written:?} => {formatted:?}: {error:?}"));
            assert_eq!(
                ast_without_spans(original),
                ast_without_spans(reparsed),
                "{written:?} => {formatted:?}"
            );
            assert_eq!(
                formatted,
                super::format::format("omitted-token.n", &formatted).unwrap(),
                "{written:?}"
            );
            let source_comments = super::cst::parse_cst("omitted-token.n", &written).unwrap();
            let output_comments = super::cst::parse_cst("omitted-token.n", &formatted).unwrap();
            let comment_text = |cst: &super::cst::Cst| {
                cst.comments
                    .iter()
                    .map(|comment| cst.source[comment.start..comment.end].to_owned())
                    .collect::<Vec<_>>()
            };
            assert_eq!(
                comment_text(&source_comments),
                comment_text(&output_comments),
                "{written:?} => {formatted:?}"
            );
        }
    }
}

#[test]
fn written_terminator_after_discarded_else_groups_keeps_its_comment() {
    let cases = [
        "fn M(){b=if c then () else (if u then 4 else 2); //e\n}",
        "fn M(){b=if c then () else (if u then 4 else 2)\n;//e\n}",
        "fn M(){x=[if a then 1 else (if b then 1 else 2)//c\n][0];}",
        "fn M(){x=[if a then 1 else (if b then 1 else 2)//c\n]();}",
        "fn M(){x[if a then 1 else (if b then 1 else 2)//c\n]=1;}",
        "fn M(){x=[if a then 1 else (if b then 1 else 2)//c\n]+1;}",
    ];
    for case in cases {
        for source in [
            case.to_owned(),
            case.replace('\n', "\r"),
            case.replace('\n', "\r\n"),
        ] {
            assert_omitted_token_format(&source);
            let formatted = super::format::format("else-close.n", &source).unwrap();
            if source.contains("//e") {
                assert!(formatted.contains("; //e"), "{source:?} => {formatted:?}");
            }
        }
    }
}

#[test]
fn origin_and_relocation_guards() {
    let cases = [
        "fn M(x: view Result<Array<I64>,Bool>) -> () with {call(no_io,no_alloc)} {return ();}",
        "fn M(){let x=- 1;var y:Array<I64>=[1,2];y[0]=x;}",
        "fn M(){let x=((1));let y=2;}",
        "fn M(){let x=if a then {value:1;}else{value:2;};}",
        "fn M(){x=if a then 1 else ((if b then 2 else 3));}",
        "fn M(){let x=if a then {// branch\nvalue:1;}else 2;}",
        "fn M(){let x=1; // tail\nlet y=2;}",
        "fn M(){if a {{x;}} y;}",
        "fn M(){let x=if a then {value:if b then 1 else 2;} else 3;}",
        "fn M(){let x=if a then // branch\n{value:1;}else 2;}",
        "fn M(){let x=1\n// before terminator\n;}",
        "fn M(){let x=if a then 1 else{value:2\n}// tail\n;}",
        "fn M(){b=if c then () else (if u then 4 else 2); //e\n}",
        "fn M(){b=if c then () else (if u then 4 else 2)\n;//e\n}",
        "fn M(){x=[if a then 1 else (if b then 1 else 2)//c\n][0];}",
        "fn M(){x=[if a then 1 else (if b then 1 else 2)//c\n]();}",
        "fn M(){x[if a then 1 else (if b then 1 else 2)//c\n]=1;}",
        "fn M(){x=[if a then 1 else (if b then 1 else 2)//c\n]+1;}",
        "fn M()->I64{return if a then (F((1))) else ((if b then 2 else 3));}",
        "fn M(){x=F(a:(if b then 1 else 2),z:[(0)]);}",
    ];
    for case in cases {
        for source in [
            case.to_owned(),
            case.replace('\n', "\r"),
            case.replace('\n', "\r\n"),
        ] {
            assert_omitted_token_format(&source);
        }
    }
    let source = "fn M(){x=if a then 1 else\n// next\n(if b then 2 else 3);}";
    let formatted = super::format::format("next-anchor.n", source).unwrap();
    let comment = formatted.find("// next").unwrap();
    let inner_if = formatted.rfind("if b").unwrap();
    assert!(comment < inner_if, "{formatted:?}");
    assert_omitted_token_format(source);

    let source = "fn M(){x=if a then 1 else(// adjacent\nif b then 2 else 3);}";
    let formatted = super::format::format("adjacent-anchor.n", source).unwrap();
    assert!(formatted.contains("else ( // adjacent"), "{formatted:?}");
    assert_omitted_token_format(source);
}

fn assert_omitted_token_format(source: &str) {
    let cst = super::cst::parse_cst("token-origin.n", source).unwrap();
    let emitted = super::print::print_with_cst(&cst).unwrap();
    // Independent, deliberately linear oracle for the complete §5.2 anchor
    // rule. It scans origins for every comment instead of using Anchors.
    let mut expected_slots = Vec::new();
    let mut previous_slot = 0;
    for comment in &cst.comments {
        let (id, trailing) = match comment.attachment {
            super::cst::Attachment::Previous(id) => (id, true),
            super::cst::Attachment::Next(id) => (id, false),
            super::cst::Attachment::Eof => {
                expected_slots.push((
                    emitted.emissions.len(),
                    !expected_slots.is_empty() && previous_slot == emitted.emissions.len(),
                ));
                previous_slot = emitted.emissions.len();
                continue;
            }
        };
        let exact = emitted
            .emissions
            .iter()
            .position(|event| event.source_id == Some(id));
        let preceding = emitted
            .emissions
            .iter()
            .rposition(|event| event.source_id.is_some_and(|first| first < id));
        let following = emitted
            .emissions
            .iter()
            .position(|event| event.source_id.is_some_and(|first| first > id));
        let mut slot = if let Some(at) = exact {
            if trailing { at + 1 } else { at }
        } else if trailing && preceding.is_some() {
            following.unwrap_or(emitted.emissions.len())
        } else if let Some(next) = following {
            preceding
                .filter(|&prev| prev + 1 < next)
                .map_or(next, |prev| prev + 1)
        } else {
            emitted.emissions.len()
        };
        while emitted
            .emissions
            .get(slot)
            .is_some_and(|event| event.role == super::emit::Role::Terminator)
        {
            slot += 1;
        }
        slot = slot.max(previous_slot);
        expected_slots.push((slot, slot == previous_slot && !expected_slots.is_empty()));
        previous_slot = slot;
    }
    let mut actual_slots = vec![(0, false); cst.comments.len()];
    for (slot, comments) in super::format::comment_slots(&cst, &emitted.emissions)
        .iter()
        .enumerate()
    {
        for &(order, shares_anchor) in comments {
            actual_slots[order] = (slot, shares_anchor);
        }
    }
    assert_eq!(actual_slots, expected_slots, "§5.2 anchors: {source:?}");
    let mut seen = vec![false; cst.tokens.len()];
    let mut prior_end = 0;
    for event in &emitted.emissions {
        if let Some(first) = event.source_id {
            assert!(
                first >= prior_end,
                "source identities out of order: {source:?}"
            );
            if event.source_end > first + 1 {
                // print.rs has one multi-token origin: signed integer
                // literals. Composite punctuation is lexical, and the
                // remaining productions emit their source tokens separately.
                assert_eq!(event.source_end, first + 2, "{source:?}");
                assert_eq!(cst.tokens[first].text, "-", "{source:?}");
                assert_eq!(
                    cst.tokens[first + 1].kind,
                    super::lex::Kind::Integer,
                    "{source:?}"
                );
            }
            prior_end = event.source_end;
            let spelled = cst.tokens[first..event.source_end]
                .iter()
                .map(|token| token.text.as_str())
                .collect::<String>();
            assert_eq!(event.text, spelled, "source identity: {source:?}");
            for slot in &mut seen[first..event.source_end] {
                assert!(!*slot, "duplicate source identity: {source:?}");
                *slot = true;
            }
        }
    }
    let formatted = super::format::format_cst("token-origin.n", &cst).unwrap();
    let reparsed = parse("token-origin.n", &formatted)
        .unwrap_or_else(|error| panic!("{source:?} => {formatted:?}: {error:?}"));
    for function in &reparsed.functions {
        assert_simple_terminators(&function.body, &formatted);
    }
    assert_eq!(
        ast_without_spans(cst.ast.clone()),
        ast_without_spans(reparsed),
        "{source:?} => {formatted:?}"
    );
    assert_eq!(
        formatted,
        super::format::format("token-origin.n", &formatted).unwrap(),
        "{source:?}"
    );
    let output = super::cst::parse_cst("token-origin.n", &formatted).unwrap();
    for (source_comment, output_comment) in cst.comments.iter().zip(&output.comments) {
        if let Some((function_index, _)) =
            cst.ast.functions.iter().enumerate().find(|(_, function)| {
                function.span.start as usize <= source_comment.start
                    && source_comment.start < function.span.end as usize
            })
        {
            let function = &output.ast.functions[function_index];
            assert!(
                function.span.start as usize <= output_comment.start
                    && output_comment.start < function.span.end as usize,
                "comment escaped function: {source:?} => {formatted:?}"
            );
        }
    }
    let comment_texts = |cst: &super::cst::Cst| {
        cst.comments
            .iter()
            .map(|comment| {
                cst.source[comment.start..comment.end]
                    .trim_end_matches(super::lex::is_horizontal_space)
                    .to_owned()
            })
            .collect::<Vec<_>>()
    };
    assert_eq!(
        comment_texts(&cst),
        comment_texts(&output),
        "{source:?} => {formatted:?}"
    );
    if cst.comments.is_empty() {
        assert_eq!(formatted, print_canonical(&cst.ast).unwrap(), "{source:?}");
    }
    assert!(formatted.contains(';'), "explicit terminator: {source:?}");
}

fn assert_simple_terminators(block: &super::parse::Block, source: &str) {
    use super::parse::{Else, Expr, Statement};
    fn expression(value: &Expr, source: &str) {
        match value {
            Expr::Array(items, _) => items.iter().for_each(|item| expression(item, source)),
            Expr::Group(inner, _) | Expr::Unary { value: inner, .. } => expression(inner, source),
            Expr::Binary { left, right, .. } => {
                expression(left, source);
                expression(right, source);
            }
            Expr::Call { target, args, .. } => {
                expression(target, source);
                for argument in args {
                    expression(&argument.value, source);
                }
            }
            Expr::Index { target, index, .. } => {
                expression(target, source);
                expression(index, source);
            }
            Expr::If {
                condition,
                then_block,
                otherwise,
                ..
            } => {
                expression(condition, source);
                assert_simple_terminators(then_block, source);
                if let Some(otherwise) = otherwise {
                    match &**otherwise {
                        Else::Block(block) => assert_simple_terminators(block, source),
                        Else::If(value) => expression(value, source),
                    }
                }
            }
            Expr::Name(..) | Expr::Integer(..) | Expr::Bool(..) | Expr::Unit(..) => {}
        }
    }
    for statement in &block.statements {
        match statement {
            Statement::Let { value, span, .. } => {
                assert_eq!(source.as_bytes()[span.end as usize - 1], b';');
                expression(value, source);
            }
            Statement::Assign { place, value, span } => {
                assert_eq!(source.as_bytes()[span.end as usize - 1], b';');
                expression(place, source);
                expression(value, source);
            }
            Statement::Return { value, span } | Statement::Value { value, span } => {
                if !matches!(statement, Statement::Value { .. })
                    || source[span.start as usize..].starts_with("value:")
                {
                    assert_eq!(source.as_bytes()[span.end as usize - 1], b';');
                }
                if let Some(value) = value {
                    let keyword_end = span.start as usize + 6;
                    if matches!(statement, Statement::Return { .. })
                        || source[span.start as usize..].starts_with("value:")
                    {
                        assert!(
                            !source[keyword_end..value.span().start as usize].contains("//"),
                            "line comment between keyword and operand: {source:?}"
                        );
                    }
                    expression(value, source);
                }
            }
            Statement::Expr { value, span } => {
                assert_eq!(source.as_bytes()[span.end as usize - 1], b';');
                expression(value, source);
            }
            Statement::Block(block) => assert_simple_terminators(block, source),
            Statement::While {
                condition, body, ..
            } => {
                expression(condition, source);
                assert_simple_terminators(body, source);
            }
            Statement::If {
                condition,
                then_block,
                otherwise,
                ..
            } => {
                expression(condition, source);
                assert_simple_terminators(then_block, source);
                if let Some(otherwise) = otherwise {
                    match &**otherwise {
                        Statement::Block(block) => assert_simple_terminators(block, source),
                        Statement::Expr { value, .. } => expression(value, source),
                        _ => {}
                    }
                }
            }
        }
    }
}

#[test]
fn omitted_token_neighbor_matrix() {
    let bases = [
        "fn M(){let t=if a then ((1)) else ((if c then 1 else 2));}",
        "fn M(){let t=if a then {value:1;} else {value:2;};}",
        "fn M(){let t=if a then {value:(1); } else {value:(2);};}",
        "fn M(){let t=if a then 1 else ((if c then 1 else 2)); var d=[(t)]; d[(0)]=t;}",
        "fn M(){let t=if a then 1 else {value:2\n}\n;}",
        "fn M(){b=if c then () else ((if u then 4 else 2))\n;}",
        "fn M(){x=[if a then 1 else (if b then 1 else 2)][0];}",
        "fn M(){x[if a then 1 else (if b then 1 else 2)]=1;}",
    ];
    let mut accepted = 0;
    let mut rejected_neighbors = Vec::new();
    let mut kinds = std::collections::BTreeSet::new();
    for (base_id, base) in bases.into_iter().enumerate() {
        assert_omitted_token_format(base);
        let cst = super::cst::parse_cst("token-origin.n", base).unwrap();
        let emitted = super::print::print_with_cst(&cst).unwrap();
        let mut retained = vec![false; cst.tokens.len()];
        for event in emitted.emissions {
            if let Some(id) = event.source_id {
                retained[id..event.source_end].fill(true);
            }
        }
        for (token_id, token) in cst
            .tokens
            .iter()
            .filter(|token| token.token.kind != super::lex::Kind::Eof && !retained[token.id])
            .enumerate()
        {
            kinds.insert(token.text.clone());
            for (newline_id, newline) in ["\n", "\r", "\r\n"].into_iter().enumerate() {
                for (offset, before) in [
                    (token.span.start as usize, true),
                    (token.span.end as usize, false),
                ] {
                    let mut source = base.to_owned();
                    source.insert_str(offset, &format!("// neighbor{newline}"));
                    match parse("token-origin.n", &source) {
                        Ok(_) => {
                            assert_omitted_token_format(&source);
                            accepted += 1;
                        }
                        Err(error) => rejected_neighbors.push(format!(
                            "{base_id},{token_id},{newline_id},{before},false:{}:{}",
                            error.code,
                            error.parser_context.as_deref().unwrap_or("")
                        )),
                    }
                    if before {
                        let mut independent = base.to_owned();
                        independent.insert_str(offset, &format!("{newline}// neighbor{newline}"));
                        match parse("token-origin.n", &independent) {
                            Ok(_) => {
                                assert_omitted_token_format(&independent);
                                accepted += 1;
                            }
                            Err(error) => rejected_neighbors.push(format!(
                                "{base_id},{token_id},{newline_id},{before},true:{}:{}",
                                error.code,
                                error.parser_context.as_deref().unwrap_or("")
                            )),
                        }
                    }
                }
            }
        }
    }
    assert!(
        kinds.contains("(")
            && kinds.contains(")")
            && kinds.contains("{")
            && kinds.contains("}")
            && kinds.contains("value")
            && kinds.contains(":"),
        "{kinds:?}"
    );
    assert_closed_rejections(
        &rejected_neighbors,
        include_str!("neighbor-comment-rejections.txt"),
    );
    assert_eq!(accepted, 300);
    assert_eq!(accepted + rejected_neighbors.len(), 360);
}

#[test]
fn canonical_structure_keeps_block_boundary_and_comparison_precedence() {
    let block = "fn M(){{x;} y;}";
    let formatted = super::format::format("structure.n", block).unwrap();
    assert_eq!(formatted, "fn M() {\n    {\n        x;\n    }\n    y;\n}\n");
    let comparison = "fn M(){x=a<b+c;}";
    let formatted = super::format::format("structure.n", comparison).unwrap();
    assert_eq!(formatted, "fn M() {\n    x = a < b + c;\n}\n");
}

#[test]
fn omitted_token_close_keeps_two_comments_in_source_order() {
    let source = "fn M(){let t=if a then 1 else{value:2\n}// first\n// second\n;}";
    assert_omitted_token_format(source);
    let source = "fn M(){x=if a then 1 else ((// c\nif b then 2 else 3));}";
    let formatted = super::format::format("output.n", source).unwrap();
    assert_eq!(
        formatted,
        "fn M() {\n    x = if a then 1 else ( // c\n    if b then 2 else 3);\n}\n"
    );
    assert_omitted_token_format(source);
}

#[test]
fn omitted_token_output_contracts() {
    let cases = [
        (
            "fn M(){let x=((1));let y=2;}",
            "fn M() {\n    let x = ((1));\n    let y = 2;\n}\n",
        ),
        (
            "fn M(){let x=if a then {value:1;}else{value:2;};}",
            "fn M() {\n    let x = if a then 1 else 2;\n}\n",
        ),
        (
            "fn M(){let x=if a then 1 else ((if b then 2 else 3));}",
            "fn M() {\n    let x = if a then 1 else (if b then 2 else 3);\n}\n",
        ),
        (
            "fn M(){let x=if a then {// branch\nvalue:1;}else 2;}",
            "fn M() {\n    let x = if a then { // branch\n        value: 1;\n    } else 2;\n}\n",
        ),
        (
            "fn M(){let x=1; // tail\nlet y=2;}",
            "fn M() {\n    let x = 1; // tail\n    let y = 2;\n}\n",
        ),
        (
            "fn M(){if a {{x;}} y;}",
            "fn M() {\n    if a {\n        {\n            x;\n        }\n    }\n    y;\n}\n",
        ),
        (
            "fn M(){let x=if a then {value:if b then 1 else 2;} else 3;}",
            "fn M() {\n    let x = if a then (if b then 1 else 2) else 3;\n}\n",
        ),
        (
            "fn M(){let x=if a then // branch\n{value:1;}else 2;}",
            "fn M() {\n    let x = if a then // branch\n    {\n        value: 1;\n    } else 2;\n}\n",
        ),
    ];
    for (source, expected) in cases {
        assert_eq!(super::format::format("output.n", source).unwrap(), expected);
        assert_omitted_token_format(source);
    }
    let source = "fn M(){let x=1\n// before terminator\n;}";
    let cst = super::cst::parse_cst("origin.n", source).unwrap();
    let semicolon = cst.tokens.iter().find(|token| token.text == ";").unwrap();
    let events = super::print::print_with_cst(&cst).unwrap().emissions;
    assert!(
        events
            .iter()
            .any(|event| event.text == ";" && event.source_id == Some(semicolon.id))
    );
    assert_omitted_token_format(source);
    let source = "fn M(){let x=1; // tail\u{b}\u{c}\u{85}\u{2028}\u{2029}\n}";
    let formatted = super::format::format("output.n", source).unwrap();
    assert!(formatted.contains("// tail\n"), "{formatted:?}");
    let source = "fn M(){let x=if a then 1 else{value:2;}// tail\n;}";
    let formatted = super::format::format("output.n", source).unwrap();
    assert_eq!(
        formatted,
        "fn M() {\n    let x = if a then 1 else 2; // tail\n}\n"
    );
    assert_omitted_token_format(source);
    let source = "fn M(){let x=(if a then {value:1;} else {value:2;} // group\n);}";
    let formatted = super::format::format("output.n", source).unwrap();
    assert!(formatted.contains("value: 2;"), "{formatted:?}");
    assert_omitted_token_format(source);
}

#[test]
fn canonical_printer_groups_a_constructed_lower_precedence_child() {
    let mut ast = parse("precedence.n", "fn M(){x=(a<b)+c;}").unwrap();
    let super::parse::Statement::Assign {
        value: super::parse::Expr::Binary { left, .. },
        ..
    } = &mut ast.functions[0].body.statements[0]
    else {
        panic!("expected binary assignment")
    };
    let super::parse::Expr::Group(inner, _) = &mut **left else {
        panic!("expected written group")
    };
    **left = *inner.clone();
    assert_eq!(
        print_canonical(&ast).unwrap(),
        "fn M() {\n    x = (a < b) + c;\n}\n"
    );
}

#[test]
fn constructed_binary_children_and_selection_statement_are_grouped() {
    use super::parse::{Expr, Statement};
    fn operand(source: &str) -> Expr {
        let ast = parse("constructed.n", &format!("fn M(){{x={source};}}")).unwrap();
        let Statement::Assign { value, .. } = &ast.functions[0].body.statements[0] else {
            unreachable!()
        };
        match value {
            Expr::Group(inner, _) => *inner.clone(),
            value => value.clone(),
        }
    }
    let cases = [
        ("a<b", "<", "c", "(a < b) < c"),
        ("a", "<", "b<c", "a < (b < c)"),
        ("a==b", "==", "c", "(a == b) == c"),
        ("a", "==", "b==c", "a == (b == c)"),
        ("a+b", "+", "c", "a + b + c"),
        ("a", "+", "b+c", "a + (b + c)"),
        ("a-b", "-", "c", "a - b - c"),
        ("a", "-", "b-c", "a - (b - c)"),
        ("a<b", "+", "c", "(a < b) + c"),
        ("a", "+", "b<c", "a + (b < c)"),
        ("a+b", "<", "c", "a + b < c"),
        ("a", "<", "b+c", "a < b + c"),
    ];
    for (left, op, right, expected) in cases {
        let mut ast = parse("constructed.n", "fn M(){x=0;}").unwrap();
        let Statement::Assign { value, .. } = &mut ast.functions[0].body.statements[0] else {
            unreachable!()
        };
        let span = value.span().clone();
        *value = Expr::Binary {
            op: op.into(),
            left: Box::new(operand(left)),
            right: Box::new(operand(right)),
            span,
        };
        let printed = print_canonical(&ast).unwrap();
        assert!(
            printed.contains(expected),
            "{left} {op} {right}: {printed:?}"
        );
        parse("constructed.n", &printed).unwrap();
    }
    let mut ast = parse("constructed.n", "fn M(){0;}").unwrap();
    let Statement::Expr { value, .. } = &mut ast.functions[0].body.statements[0] else {
        unreachable!()
    };
    *value = operand("if a then 1 else 2");
    let printed = print_canonical(&ast).unwrap();
    assert!(printed.contains("(if a then 1 else 2);"), "{printed:?}");
    parse("constructed.n", &printed).unwrap();
}

#[test]
fn omitted_else_parentheses_and_comment_order_reach_a_fixed_point() {
    let cases = [
        "fn M(){x=if a then 1 else (\n( //c\nif b then 2 else 3));}",
        "fn M(){x=if a then 1 else (( //c\nif b then 2 else 3));}",
        "fn M(){x=if a then 1 else ((if b then 2 else 3) //c\n);}",
        "fn M(){x=if a then 1 else ((((((if b then 2 else 82))) //c\n)));}",
        "fn M(){x=1\n//d\n;//e\n}",
        "fn M(){x=1\r//d\r;//e\r}",
        "fn M(){x=1\r\n//d\r\n;//e\r\n}",
        "fn M(){x=(a + (if c then {value:1;} else 2) //c\n);}",
        "fn M(){x=((a)[if c then {value:1;} else 2 //c\n]);}",
    ];
    for source in cases {
        assert_omitted_token_format(source);
    }
    let first = super::format::format("fixed-point.n", cases[0]).unwrap();
    assert_eq!(
        first,
        "fn M() {\n    x = if a then 1 else ( //c\n    if b then 2 else 3);\n}\n"
    );
    let formatted = super::format::format("order.n", cases[4]).unwrap();
    assert!(
        formatted.contains("x = 1;\n    //d\n    //e\n"),
        "{formatted:?}"
    );
    let after_group = super::format::format("group.n", cases[7]).unwrap();
    assert!(after_group.contains("then 1 else 2"), "{after_group:?}");
    let inner_group = super::format::format("group.n", cases[8]).unwrap();
    assert!(inner_group.contains("then {\n"), "{inner_group:?}");
}

#[test]
#[ignore = "else parentheses and comment gap campaign"]
fn omitted_else_parenthesis_comment_gaps_stabilize() {
    let _campaign = crate::test_alloc::campaign_guard();
    let bases = [
        "fn M(){x=if a then 1 else (if b then 2 else 3);}",
        "fn M(){x=if a then 1 else ((if b then 2 else 3));}",
        "fn M(){x=if a then (if b then 2 else 3) else ((if c then 4 else 5));}",
        "fn M(){x=if a then {value:1;} else ((if b then {value:2;} else 3));}",
        "fn M(){x=[if a then 1 else ((if b then 2 else 3))][0];}",
        "fn M(){x=F(if a then 1 else ((if b then 2 else 3)));}",
        "fn M(){x=(if a then 1 else ((if b then 2 else 3)))+1;}",
        "fn M(){x=if a then 1 else ((((((if b then 2 else 82))))));}",
    ];
    let mut placements = Vec::new();
    let mut rejected_placements = Vec::new();
    for (base_id, base) in bases.iter().enumerate() {
        let tokens = super::lex::lex("else-gaps.n", base).unwrap();
        for (token_id, token) in tokens.iter().enumerate() {
            if token.kind == super::lex::Kind::Eof {
                continue;
            }
            for before in [false, true] {
                for independent in [false, true] {
                    let offset = if before {
                        token.span.start
                    } else {
                        token.span.end
                    } as usize;
                    let mut sample = (*base).to_owned();
                    sample.insert_str(
                        offset,
                        if independent {
                            "\n// gap\n"
                        } else {
                            "// gap\n"
                        },
                    );
                    match parse("else-gaps.n", &sample) {
                        Ok(_) => placements.push((base_id, token_id, before, independent)),
                        Err(error) => rejected_placements.push(format!(
                            "{base_id},{token_id},{before},{independent}:{}:{}",
                            error.code,
                            error.parser_context.as_deref().unwrap_or("")
                        )),
                    }
                }
            }
        }
    }
    assert_closed_rejections(
        &rejected_placements,
        include_str!("else-comment-rejections.txt"),
    );
    assert_eq!(placements.len(), 892);
    assert_eq!(placements.len() + rejected_placements.len(), 908);
    let limit = std::env::var("NEXO_ELSE_COMMENT_CASES")
        .ok()
        .and_then(|value| value.parse::<usize>().ok())
        .unwrap_or(if cfg!(debug_assertions) {
            30_000
        } else {
            200_000
        });
    for n in 0..limit {
        let (base_id, token_id, before, independent) = placements[n % placements.len()];
        let base = bases[base_id];
        let tokens = super::lex::lex("else-gaps.n", base).unwrap();
        let token = &tokens[token_id];
        let offset = if before {
            token.span.start
        } else {
            token.span.end
        } as usize;
        let newline = ["\n", "\r", "\r\n"][n % 3];
        let mut source = base.to_owned();
        source.insert_str(
            offset,
            &format!(
                "{}// gap{n}{newline}",
                if independent { newline } else { "" }
            ),
        );
        assert_omitted_token_format(&source);
    }
    println!(
        "else comment sources checked: {limit}; placements: {}",
        placements.len()
    );
}

#[test]
#[ignore = "consecutive comments across omitted-token gaps"]
fn consecutive_comment_gaps_preserve_canonical_properties() {
    let _campaign = crate::test_alloc::campaign_guard();
    let bases = [
        "fn M(){x=if a then 1 else ((if b then 2 else 3));}",
        "fn M(){x=if a then (if b then 2 else 3) else ((if c then 4 else 5));}",
        "fn M(){x=(if a then 1 else ((if b then 2 else 3)))+1;}",
        "fn M(){x=F(if a then 1 else ((if b then 2 else 3)));}",
        "fn M(){x=[if a then 1 else ((if b then 2 else 3))][0];}",
        "fn M(){x=if a then {value:1;}else ((if b then 2 else 3));}",
        "fn M(){x=if a then 1 else ((if b then {value:2;} else 3));}",
        "fn M(){x[if a then 1 else ((if b then 2 else 3))]=4;}",
    ];
    let mut gaps = Vec::new();
    let mut rejected_gaps = Vec::new();
    for (base_id, base) in bases.into_iter().enumerate() {
        let tokens = super::lex::lex("multi-gap.n", base).unwrap();
        let positions = tokens
            .iter()
            .filter(|token| token.kind != super::lex::Kind::Eof)
            .map(|token| token.span.end as usize)
            .collect::<Vec<_>>();
        for (gap, &at) in positions.iter().enumerate() {
            for count in 2..=4 {
                for independent_first in [false, true] {
                    let inserted = format!(
                        "{}{}",
                        if independent_first { "\n" } else { " " },
                        (0..count).map(|i| format!("// c{i}\n")).collect::<String>()
                    );
                    let mut source = base.to_owned();
                    source.insert_str(at, &inserted);
                    match parse("multi-gap.n", &source) {
                        Ok(_) => gaps.push((base, gap, count, independent_first)),
                        Err(error) => rejected_gaps.push(format!(
                            "{base_id},{gap},{count},{independent_first}:{}:{}",
                            error.code,
                            error.parser_context.as_deref().unwrap_or("")
                        )),
                    }
                }
            }
        }
    }
    assert_closed_rejections(&rejected_gaps, include_str!("multi-comment-rejections.txt"));
    assert_eq!(gaps.len(), 1320);
    assert_eq!(gaps.len() + rejected_gaps.len(), 1344);
    let limit = std::env::var("NEXO_MULTI_COMMENT_CASES")
        .ok()
        .and_then(|s| s.parse::<usize>().ok())
        .unwrap_or(if cfg!(debug_assertions) {
            30_000
        } else {
            300_000
        });
    let mut accepted = 0;
    let mut attempts = 0;
    let mut rejected_attempts = Vec::new();
    let mut state = 0x7332_9af4_5d1e_u64;
    while accepted < limit {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        let (base, gap, count, independent_first) = gaps[attempts % gaps.len()];
        let newline = ["\n", "\r", "\r\n"][attempts % 3];
        let positions = super::lex::lex("multi-gap.n", base)
            .unwrap()
            .into_iter()
            .filter(|token| token.kind != super::lex::Kind::Eof)
            .map(|token| token.span.end as usize)
            .collect::<Vec<_>>();
        let mut chosen = vec![gap];
        for shift in 0..(attempts % 3) {
            chosen.push(((state >> (shift * 16)) as usize) % positions.len());
        }
        chosen.sort_unstable();
        chosen.dedup();
        let mut source = base.to_owned();
        for &index in chosen.iter().rev() {
            let sequence = (0..count)
                .map(|i| format!("// c{index}_{i}{newline}"))
                .collect::<String>();
            source.insert_str(
                positions[index],
                &format!(
                    "{}{sequence}",
                    if independent_first { newline } else { " " }
                ),
            );
        }
        attempts += 1;
        if let Err(error) = parse("multi-gap.n", &source) {
            rejected_attempts.push(format!(
                "{}:{}:{}",
                attempts - 1,
                error.code,
                error.parser_context.as_deref().unwrap_or("")
            ));
            assert!(attempts < limit * 10, "too few accepted generated sources");
            continue;
        }
        assert_omitted_token_format(&source);
        accepted += 1;
    }
    if limit == 30_000 {
        assert_eq!(attempts, 30_461);
        assert_closed_rejections(
            &rejected_attempts,
            include_str!("multi-attempt-debug-rejections.txt"),
        );
    } else if limit == 300_000 {
        assert_eq!(attempts, 305_045);
        assert_closed_rejections(
            &rejected_attempts,
            include_str!("multi-attempt-release-rejections.txt"),
        );
    } else {
        assert_eq!(accepted + rejected_attempts.len(), attempts);
    }
    println!(
        "consecutive-comment sources: {accepted}/{attempts}; placements: {}",
        gaps.len()
    );
}

#[test]
fn constructed_selection_operand_gets_required_parentheses() {
    use super::parse::{Argument, Else, Expr, Mode, Statement};

    fn expression(source: &str) -> Expr {
        let ast = parse("selection-operand.n", &format!("fn M(){{x={source};}}")).unwrap();
        let Statement::Assign { value, .. } = &ast.functions[0].body.statements[0] else {
            panic!("expected assignment")
        };
        value.clone()
    }

    let selection = expression("if a then 1 else 2");
    let integer = expression("1");
    let name = expression("F");
    let span = selection.span().clone();
    let mut then_arm = expression("if z then 0 else 0");
    let Expr::If { then_block, .. } = &mut then_arm else {
        unreachable!()
    };
    let Statement::Value { value, .. } = &mut then_block.statements[0] else {
        unreachable!()
    };
    *value = Some(selection.clone());
    let mut else_arm = expression("if z then 0 else 0");
    let Expr::If { otherwise, .. } = &mut else_arm else {
        unreachable!()
    };
    *otherwise = Some(Box::new(Else::If(Box::new(selection.clone()))));
    let cases = [
        (
            "binary left",
            Expr::Binary {
                op: "+".into(),
                left: Box::new(selection.clone()),
                right: Box::new(integer.clone()),
                span: span.clone(),
            },
            "(if a then 1 else 2) + 1",
        ),
        (
            "binary right",
            Expr::Binary {
                op: "+".into(),
                left: Box::new(integer.clone()),
                right: Box::new(selection.clone()),
                span: span.clone(),
            },
            "1 + (if a then 1 else 2)",
        ),
        (
            "unary prefix",
            Expr::Unary {
                mode: Mode::Consume,
                value: Box::new(selection.clone()),
                span: span.clone(),
            },
            "move (if a then 1 else 2)",
        ),
        (
            "unary suffix",
            Expr::Unary {
                mode: Mode::Read,
                value: Box::new(selection.clone()),
                span: span.clone(),
            },
            "(if a then 1 else 2)?",
        ),
        (
            "call target",
            Expr::Call {
                target: Box::new(selection.clone()),
                args: vec![],
                span: span.clone(),
            },
            "(if a then 1 else 2)()",
        ),
        (
            "index target",
            Expr::Index {
                target: Box::new(selection.clone()),
                index: Box::new(integer.clone()),
                span: span.clone(),
            },
            "(if a then 1 else 2)[1]",
        ),
        (
            "index operand",
            Expr::Index {
                target: Box::new(name.clone()),
                index: Box::new(selection.clone()),
                span: span.clone(),
            },
            "F[if a then 1 else 2]",
        ),
        (
            "argument",
            Expr::Call {
                target: Box::new(name),
                args: vec![Argument {
                    label: None,
                    value: selection.clone(),
                    span: span.clone(),
                }],
                span: span.clone(),
            },
            "F(if a then 1 else 2)",
        ),
        (
            "array item",
            Expr::Array(vec![selection.clone()], span.clone()),
            "[if a then 1 else 2]",
        ),
        (
            "then arm",
            then_arm,
            "if z then (if a then 1 else 2) else 0",
        ),
        (
            "else arm",
            else_arm,
            "if z then 0 else (if a then 1 else 2)",
        ),
    ];
    for (position, value, expected) in cases {
        let mut ast = parse("selection-operand.n", "fn M(){x=0;}").unwrap();
        let Statement::Assign { value: slot, .. } = &mut ast.functions[0].body.statements[0] else {
            unreachable!()
        };
        *slot = value;
        let printed = print_canonical(&ast).unwrap();
        assert!(printed.contains(expected), "{position}: {printed:?}");
        // `move` requires a Place, so a hand-built selection there is outside
        // the parser's accepted AST domain; its grouping is still printable.
        if position != "unary prefix" {
            parse("selection-operand.n", &printed)
                .unwrap_or_else(|error| panic!("{position}: {printed:?}: {error:?}"));
        }
    }
}

#[test]
#[ignore = "generated token-origin campaign"]
fn omitted_token_generated_programs() {
    let bases = [
        "fn M(){let t=if a then ((1)) else ((if c then 1 else 2));}",
        "fn M(){let t=if a then {value:1;} else {value:2;};}",
        "fn M(){let t=if a then {value:(1);} else {value:(2);};}",
        "fn M(){let t=if a then 1 else ((if c then 1 else 2)); var d=[(t)]; d[(0)]=t;}",
        "fn M(){let t=if a then 1 else {value:2\n}\n;}",
        "fn M(){let t=if a then ((F((1)))) else ((if c then 1 else 2));}",
        "fn M(){b=if c then () else (if u then 4 else 2);}",
        "fn M(){b=if c then () else ((if u then 4 else 2))\n;}",
        "fn M(){x=[if a then 1 else (if b then 1 else 2)][0];}",
        "fn M(){x=[if a then 1 else (if b then 1 else 2)]();}",
        "fn M(){x[if a then 1 else (if b then 1 else 2)]=1;}",
        "fn M(){x=[if a then 1 else (if b then 1 else 2)]+1;}",
    ];
    let mut insertions = Vec::new();
    let mut rejected_insertions = Vec::new();
    for (base_id, base) in bases.iter().enumerate() {
        let tokens = super::lex::lex("token-origin.n", base).unwrap();
        for (token_id, token) in tokens.iter().enumerate() {
            if token.kind == super::lex::Kind::Eof {
                continue;
            }
            for (before, offset) in [
                (true, token.span.start as usize),
                (false, token.span.end as usize),
            ] {
                for leading in [false, true] {
                    let mut candidate = (*base).to_owned();
                    candidate.insert_str(
                        offset,
                        if leading {
                            "\n// generated\n"
                        } else {
                            "// generated\n"
                        },
                    );
                    match parse("token-origin.n", &candidate) {
                        Ok(_) => insertions.push((base_id, token_id, before, leading)),
                        Err(error) => rejected_insertions.push(format!(
                            "{base_id},{token_id},{before},{leading}:{}:{}",
                            error.code,
                            error.parser_context.as_deref().unwrap_or("")
                        )),
                    }
                }
            }
        }
    }
    assert_closed_rejections(
        &rejected_insertions,
        include_str!("token-comment-rejections.txt"),
    );
    assert_eq!(insertions.len(), 1308);
    assert_eq!(insertions.len() + rejected_insertions.len(), 1348);
    let limit = std::env::var("NEXO_TOKEN_ORIGIN_CASES")
        .ok()
        .and_then(|value| value.parse::<usize>().ok())
        .unwrap_or(100_000);
    for n in 0..limit {
        let base_id = n % bases.len();
        let bare = bases[base_id].replacen("fn M()", &format!("fn M{n}()"), 1);
        assert_omitted_token_format(&bare);
        let (base_id, token_id, before, leading) = insertions[n % insertions.len()];
        let mut written = bases[base_id].replacen("fn M()", &format!("fn M{n}()"), 1);
        let tokens = super::lex::lex("token-origin.n", &written).unwrap();
        let token = &tokens[token_id];
        let offset = if before {
            token.span.start
        } else {
            token.span.end
        } as usize;
        let ending = match n % 3 {
            0 => "\n",
            1 => "\r",
            _ => "\r\n",
        };
        written.insert_str(
            offset,
            &format!(
                "{leading}// generated{ending}",
                leading = if leading { ending } else { "" }
            ),
        );
        assert_omitted_token_format(&written);
    }
    println!(
        "generated {limit} sources without and {limit} with comments; {} accepted insertion positions",
        insertions.len()
    );
}

#[test]
fn canonical_assignment_places_nested_branches_and_written_groups() {
    let sources = [
        "fn M(){M[a-b]=1;}",
        "fn M(){M[a<b]=1;}",
        "fn M(){a[if b then {let t=1\nvalue:t\n} else 2]=1;}",
        "fn M(){a[if b then {value:1\n} else 2]=1;}",
        "fn M(){x=if a then {value: if b then 1 else 2;} else 3;}",
        "fn M(){x=if a then {{}\ny;\nvalue:1;} else 2;}",
        "fn M(){if a {}else{{x;}\ny;}}",
        "fn F(){x=if a then 1 else ((if b then 2 else 3));}",
    ];
    for source in sources {
        let original = parse("assignment-places.n", source).unwrap();
        let formatted = super::format::format("assignment-places.n", source).unwrap();
        assert_eq!(formatted, print_canonical(&original).unwrap(), "{source:?}");
        let reparsed = parse("assignment-places.n", &formatted)
            .unwrap_or_else(|error| panic!("{source:?} => {formatted:?}: {error:?}"));
        assert_eq!(
            ast_without_spans(original),
            ast_without_spans(reparsed),
            "{source:?}"
        );
        assert_eq!(
            formatted,
            super::format::format("assignment-places.n", &formatted).unwrap()
        );
    }
}

#[test]
fn independent_comment_trims_only_horizontal_tail() {
    for tail in ["   ", "\t\t", "\u{3000}", "\u{a0}"] {
        let source = format!("fn M(){{x=1; // c{tail}\n}}");
        let formatted = super::format::format("comment-tail.n", &source).unwrap();
        assert!(formatted.contains("// c\n"), "{formatted:?}");
        assert!(!formatted.contains(&format!("// c{tail}")), "{formatted:?}");
        assert_eq!(
            formatted,
            super::format::format("comment-tail.n", &formatted).unwrap()
        );
    }
    for vertical in ["\u{b}", "\u{c}", "\u{85}", "\u{2028}", "\u{2029}"] {
        let source = format!("fn M(){{x=1; // c{vertical}\n}}");
        let formatted = super::format::format("comment-tail.n", &source).unwrap();
        assert!(formatted.contains("// c\n"), "{formatted:?}");
    }
}

#[test]
fn canonical_roles_keep_suffix_contract_and_else_structure() {
    let cases = [
        ("fn M(){let a=x?;}", "let a = x?;"),
        ("fn M() with {call(no_io)}{}", "with {call(no_io)}"),
        ("fn M(){if a {} else {x;} y;}", "}\n    y;"),
    ];
    for (source, literal) in cases {
        let ast = parse("roles.n", source).unwrap();
        let canonical = print_canonical(&ast).unwrap();
        assert!(canonical.contains(literal), "{source:?} => {canonical:?}");
        assert_eq!(super::format::format("roles.n", source).unwrap(), canonical);
        assert_eq!(
            ast_without_spans(ast),
            ast_without_spans(parse("roles.n", &canonical).unwrap())
        );
    }
}

#[test]
fn split_type_close_keeps_independent_comment_before_assignment() {
    let source = "fn F(){let a:Array<I64\n// keep\n>=x;}";
    let cst = super::cst::parse_cst("split-comment.n", source).unwrap();
    let close = cst
        .tokens
        .iter()
        .find(|token| token.text == ">" && token.span.start as usize == source.find(">=").unwrap())
        .unwrap();
    let split_at = source.find(">=").unwrap() as u64;
    assert_eq!((close.span.start, close.span.end), (split_at, split_at + 1));
    let assign = &cst.tokens[close.id + 1];
    assert_eq!(
        (&*assign.text, assign.span.start, assign.span.end),
        ("=", split_at + 1, split_at + 2)
    );
    assert_eq!(
        cst.comments[0].attachment,
        super::cst::Attachment::Next(close.id)
    );
    let original = parse("split-comment.n", source).unwrap();
    let formatted = super::format::format("split-comment.n", source).unwrap();
    assert_eq!(
        formatted,
        "fn F() {\n    let a: Array<I64\n    // keep\n    > = x;\n}\n"
    );
    let reparsed = parse("split-comment.n", &formatted)
        .unwrap_or_else(|error| panic!("{source:?} => {formatted:?}: {error:?}"));
    assert_eq!(ast_without_spans(original), ast_without_spans(reparsed));
    assert!(formatted.contains("// keep"), "{formatted:?}");
    assert_eq!(
        formatted,
        super::format::format("split-comment.n", &formatted).unwrap()
    );
}

#[test]
fn signed_integer_cst_identities_keep_adjacent_comments() {
    for source in [
        "fn Main(){Print(-9223372036854775808// signed\n-1);}",
        "fn Main(){Print(-// signed\n9223372036854775808-1);}",
        "fn Main(){Print(-\n// signed\n9223372036854775808-1);}",
    ] {
        let original = parse("signed-comment.n", source).unwrap();
        let formatted = super::format::format("signed-comment.n", source).unwrap();
        if source.contains("-\n") {
            assert_eq!(
                formatted,
                "fn Main() {\n    Print(-9223372036854775808\n    // signed\n    - 1);\n}\n"
            );
        }
        let reparsed = parse("signed-comment.n", &formatted)
            .unwrap_or_else(|error| panic!("{source:?} => {formatted:?}: {error:?}"));
        assert_eq!(
            ast_without_spans(original),
            ast_without_spans(reparsed),
            "{source:?}"
        );
        assert!(formatted.contains("// signed\n"), "{formatted:?}");
        assert_eq!(
            formatted,
            super::format::format("signed-comment.n", &formatted).unwrap()
        );
    }
}

#[test]
fn fused_sign_digit_comments_stay_after_return_and_value() {
    let cases = [
        "fn F() -> I64 { return -\n// doc\n1; }\nfn Main() { Print(F()); }",
        "fn F() -> I64 { return if true then { value: -\n// doc\n1; } else 0; }\nfn Main() { Print(F()); }",
    ];
    for case in cases {
        for source in [
            case.to_owned(),
            case.replace('\n', "\r"),
            case.replace('\n', "\r\n"),
        ] {
            assert_omitted_token_format(&source);
            let formatted = super::format::format("fused-sign.n", &source).unwrap();
            assert!(
                formatted.contains("-1;\n") && formatted.contains("// doc"),
                "{formatted:?}"
            );
            assert_eq!(
                ast_without_spans(parse("fused-sign.n", &source).unwrap()),
                ast_without_spans(parse("fused-sign.n", &formatted).unwrap())
            );
            assert_eq!(
                formatted,
                super::format::format("fused-sign.n", &formatted).unwrap()
            );
        }
    }
}

#[test]
fn multi_token_construct_comment_gaps_preserve_execution() {
    // Each marker is a lexical gap inside a construction whose surrounding
    // tokens together determine syntax. Composite punctuators (->, ..<,
    // ..=, ? and >= before its parser split) are single lexical tokens and
    // have no source gap inside them.
    let templates = [
        "fn F()->I64{return -~1;}fn Main(){Print(F());}",
        "fn F()->I64{return if true then {value:-~1;}else 0;}fn Main(){Print(F());}",
        "fn Main(){let x=-~1;Print(x);}",
        "fn Main(){Print(-~1);}",
        "fn Main(){let a=[4,5];Print(a[-~0+1]);}",
        "fn Main(){let x=if true then -~1 else 0;Print(x);}",
        "fn F(x:view I64)->I64{return x;}fn Main(){Print(F(x:-~1));}",
        "fn Main(){let a:Array<~I64>=[1];Print(a[0]);}",
        "fn Main(){let x=if true then {value:-~1;} else 0;Print(x);}",
        "fn F(x:view I64)->I64{return x;}fn Main(){Print(F(x:-~1));}",
        "fn F(x:view I64)->I64{return x;}fn Main(){Print(F(~1));}",
        "fn Main(){var a=[1];a[~0]=2;Print(a[0]);}",
    ];
    for (id, template) in templates.into_iter().enumerate() {
        for newline in ["\n", "\r", "\r\n"] {
            let source = template.replace('~', &format!("{newline}// gap{id}{newline}"));
            assert_omitted_token_format(&source);
            let formatted = super::format::format("gaps-run.n", &source).unwrap();
            let original = crate::ir::compile_scalar("gaps-run.n", &source).unwrap();
            let rewritten = crate::ir::compile_scalar("gaps-run.n", &formatted).unwrap();
            let before = crate::interp::run_scalar(&original);
            let after = crate::interp::run_scalar(&rewritten);
            assert!(before.diagnostics.is_empty(), "{source:?}");
            assert!(after.diagnostics.is_empty(), "{formatted:?}");
            assert_eq!(before.outcome, after.outcome, "{source:?} => {formatted:?}");
            assert_eq!(
                before.observations, after.observations,
                "{source:?} => {formatted:?}"
            );
        }
    }
}

#[test]
#[ignore = "large accepted commented execution equivalence campaign"]
fn accepted_commented_sources_keep_run_result_after_format() {
    let _campaign = crate::test_alloc::campaign_guard();
    let templates = [
        "fn F()->I64{return -~{n};}fn Main(){Print(F());}",
        "fn F()->I64{return if true then {value:-~{n};}else 0;}fn Main(){Print(F());}",
        "fn Main(){let x=-~{n};Print(x);}",
        "fn Main(){Print(-~{n});}",
        "fn Main(){let a=[{n},2];Print(a[0]);}",
        "fn Main(){let x=if true then -~{n} else 0;Print(x);}",
        "fn F(x:view I64)->I64{return x;}fn Main(){Print(F(x:-~{n}));}",
    ];
    let total = 100_000;
    for id in 0..total {
        let newline = ["\n", "\r", "\r\n"][id % 3];
        let value = 1 + (id % 10_000);
        let template = templates[id % templates.len()].replace("{n}", &value.to_string());
        let source = template.replace('~', &format!("{newline}// c{id}{newline}"));
        let formatted = super::format::format("run-format.n", &source).unwrap();
        let before =
            crate::interp::run_scalar(&crate::ir::compile_scalar("run-format.n", &source).unwrap());
        let after = crate::interp::run_scalar(
            &crate::ir::compile_scalar("run-format.n", &formatted).unwrap(),
        );
        assert!(before.diagnostics.is_empty(), "{id}: {source:?}");
        assert!(after.diagnostics.is_empty(), "{id}: {formatted:?}");
        assert_eq!(
            before.outcome, after.outcome,
            "{id}: {source:?} => {formatted:?}"
        );
        assert_eq!(
            before.observations, after.observations,
            "{id}: {source:?} => {formatted:?}"
        );
    }
    assert_eq!(total, 100_000);
}

#[test]
#[ignore = "large assignment and comment matrix"]
fn generated_assignment_places_and_comments_preserve_canonical_tree() {
    let endings = ["\n", "\r", "\r\n"];
    for n in 0..100_000_usize {
        let ending = endings[n % endings.len()];
        let index = match n % 8 {
            0 => "j-k".to_owned(),
            1 => "j<k".to_owned(),
            2 => "F(1)".to_owned(),
            3 => "a[j-k]".to_owned(),
            4 => format!("if b then {{value:1{ending}}} else 2"),
            5 => format!("if b then {{let t=1{ending}value:t{ending}}} else 2"),
            6 => "if b then (if c then 1 else 2) else 3".to_owned(),
            _ => "((if b then 1 else 2))".to_owned(),
        };
        let statement = if n % 7 == 0 {
            format!("x=if a then {{value: if b then 1 else 2;}} else {n};")
        } else {
            format!("a[{index}]={n};")
        };
        for commented in [false, true] {
            let source = if commented {
                format!(
                    "// before{ending}fn M(){{{ending}// inside{ending}{statement} // tail   {ending}// end{ending}}}"
                )
            } else {
                format!("fn M(){{{statement}{ending}}}")
            };
            let original = parse("places-matrix.n", &source).unwrap();
            let formatted = super::format::format("places-matrix.n", &source).unwrap();
            if !commented {
                assert_eq!(formatted, print_canonical(&original).unwrap(), "{source:?}");
            }
            let reparsed = parse("places-matrix.n", &formatted)
                .unwrap_or_else(|error| panic!("{source:?} => {formatted:?}: {error:?}"));
            assert_eq!(
                ast_without_spans(original),
                ast_without_spans(reparsed),
                "{source:?}"
            );
            assert_eq!(
                formatted,
                super::format::format("places-matrix.n", &formatted).unwrap()
            );
            if commented {
                let before = super::cst::parse_cst("places-matrix.n", &source).unwrap();
                let after = super::cst::parse_cst("places-matrix.n", &formatted).unwrap();
                let literals = |cst: &super::cst::Cst| {
                    cst.comments
                        .iter()
                        .map(|comment| {
                            cst.source[comment.start..comment.end]
                                .trim_end_matches(|c: char| {
                                    c.is_whitespace() && c != '\r' && c != '\n'
                                })
                                .to_owned()
                        })
                        .collect::<Vec<_>>()
                };
                assert_eq!(
                    literals(&before),
                    literals(&after),
                    "{source:?} => {formatted:?}"
                );
                assert_eq!(after.comments.len(), 4, "{source:?} => {formatted:?}");
                assert!(formatted.contains("// tail\n"), "{formatted:?}");
            }
        }
    }
}

#[test]
fn canonical_formatter_keeps_branch_terminators_and_structural_spacing() {
    let cases = [
        "fn M(){let x=if a then {value:1\n} else 2;}",
        "fn M(){x=if a then {value:1;} else {value:2;}\n}",
        "fn M(){b=if a then {} else {value:_u;}\n c;}",
        "fn F0(){if(){}else if[[]()]{897\r\nzz;}}",
        "fn M(){if a {} else if x<2 {}}",
        "fn M(){let x=a?+b; let y=a?==b; let z=y?<();}",
        "fn M() ->() {}",
        "fn M(){view[0]; view(0);}",
    ];
    for source in cases {
        let original = parse("canonical-format.n", source).unwrap();
        let formatted = super::format::format("canonical-format.n", source).unwrap();
        let printed = print_canonical(&original).unwrap();
        assert_eq!(formatted, printed, "{source:?}");
        let reparsed = parse("canonical-format.n", &formatted)
            .unwrap_or_else(|e| panic!("{source:?} => {formatted:?}: {e:?}"));
        assert_eq!(
            ast_without_spans(original),
            ast_without_spans(reparsed),
            "{source:?}"
        );
        assert_eq!(
            formatted,
            super::format::format("canonical-format.n", &formatted).unwrap(),
            "{source:?}"
        );
    }
}

#[test]
fn systematic_statement_and_branch_terminators_use_the_canonical_emitter() {
    for ending in [";\n", "\n", "\r", "\r\n"] {
        let statements = [
            format!("let x=1{ending}"),
            format!("var x=1{ending}"),
            format!("x=1{ending}"),
            format!("1{ending}"),
            format!("return{ending}"),
            format!("return (){ending}"),
            format!("{{let x=1{ending}}}"),
            format!("if a{{}}else if b{{897{ending}zz;}}"),
            format!("if a{{let x=1{ending}}}else if b{{x=2{ending}}}else{{return{ending}}}"),
            format!("while a{{let x=1{ending}}}"),
            format!("while a{{{{x=1{ending}}}}}"),
            format!("let x=if a then {{value:1{ending}}} else {{value:2{ending}}}{ending}"),
            format!("let x=if a then {{value:1{ending}}} else 2{ending}"),
            format!("let x=if a then 1 else (if b then 2 else 3){ending}"),
            format!(
                "let x=if a then {{value:1{ending}}} else (if b then {{value:2{ending}}} else {{value:3{ending}}}){ending}"
            ),
            format!("b=if a then {{}} else {{value:_u{ending}}}{ending}c;"),
        ];
        for statement in statements {
            let source = format!("fn M(){{{statement}}}");
            let original = parse("terminators.n", &source)
                .unwrap_or_else(|error| panic!("{source:?}: {error:?}"));
            let formatted = super::format::format("terminators.n", &source).unwrap();
            assert_eq!(formatted, print_canonical(&original).unwrap(), "{source:?}");
            let reparsed = parse("terminators.n", &formatted)
                .unwrap_or_else(|error| panic!("{source:?} => {formatted:?}: {error:?}"));
            assert_eq!(
                ast_without_spans(original),
                ast_without_spans(reparsed),
                "{source:?}"
            );
            assert_eq!(
                super::format::format("terminators.n", &formatted).unwrap(),
                formatted,
                "{source:?}"
            );
        }
    }
}

#[test]
fn non_final_value_span_ends_at_actual_colon_with_unicode_space() {
    for space in [" ", "\u{00a0}", "\u{2028}", "\u{3000}"] {
        let source = format!("fn M(){{(if e then 5 else{{value{space}:9;m;}});}}");
        let error = parse("value-space.n", &source).unwrap_err();
        assert_eq!(error.code, "NX-E1302");
        assert_eq!(error.subcause.as_deref(), Some("non_final_branch_value"));
        let start = source.find("value").unwrap();
        let end = source.find(":9").unwrap() + 1;
        let span = error.primary_span.as_ref().unwrap();
        assert_eq!((span.start, span.end), (start as u64, end as u64));
        error.validate("value-space.n", &source).unwrap();
    }
}

#[test]
fn unicode_trivia_diagnostic_spans_stay_on_utf8_boundaries() {
    let spaces = [
        " ", "\u{85}", "\u{a0}", "\u{1680}", "\u{2028}", "\u{2029}", "\u{3000}",
    ];
    for n in 0..20_000 {
        let source = format!(
            "fn{}M(){{(if{}e then 5 else{{value{}:9;m;}});}}",
            spaces[n % spaces.len()],
            spaces[(n / 7) % spaces.len()],
            spaces[(n / 49) % spaces.len()]
        );
        let error = parse("unicode-trivia.n", &source).unwrap_err();
        assert_eq!(error.code, "NX-E1302");
        assert_eq!(error.subcause.as_deref(), Some("non_final_branch_value"));
        error.validate("unicode-trivia.n", &source).unwrap();
    }
}

#[test]
fn adr_gap_receiver_call_group_and_try_line_continuation_keep_current_diagnostics() {
    let source = "fn F(){} fn Main(){let x=edit(F());}";
    let error = parse("gap.n", source).unwrap_err();
    assert_eq!(error.code, "NX-E0006");
    assert_eq!(error.feature_id.as_deref(), Some("receiver_mode"));
    error.validate("gap.n", source).unwrap();

    let source = "fn Main(){let y=x?\n(y);}";
    let error = parse("gap.n", source).unwrap_err();
    assert_eq!(error.code, "NX-E0002");
    assert_eq!(error.parser_context.as_deref(), Some("grammar.TryExpr"));
    assert_eq!(
        error
            .unexpected_token
            .as_ref()
            .and_then(|token| token.text.as_deref()),
        Some("(")
    );
    error.validate("gap.n", source).unwrap();
}

#[test]
fn unicode_horizontal_spaces_separate_b_tokens_without_newline_terminators() {
    for space in ["\u{85}", "\u{a0}", "\u{2028}", "\u{2029}", "\u{3000}"] {
        let source = format!("fn{space}F(){{let{space}x=1;}}");
        assert!(parse("spaces.n", &source).is_ok(), "{space:?}");
        let tokens = super::lex::lex("spaces.n", &source).unwrap();
        assert_eq!(tokens[0].text, "fn");
        assert_eq!(tokens[1].text, "F");
    }
}

#[test]
fn adr_gap_documentary_comment_effect_is_cst_flag_and_cr_association() {
    use super::cst::Attachment;
    let source = "fn F(){let x=1;\r/// docs\rlet y=2; // trailing\r}\n";
    let cst = super::cst::parse_cst("comments.n", source).unwrap();
    assert_eq!(cst.comments.len(), 2);
    assert!(cst.comments[0].documentary);
    assert!(!cst.comments[1].documentary);
    let next = cst
        .tokens
        .iter()
        .position(|token| {
            token.text == "let" && token.span.start as usize > source.find("docs").unwrap()
        })
        .unwrap();
    assert_eq!(cst.comments[0].attachment, Attachment::Next(next));
    let previous = cst
        .tokens
        .iter()
        .position(|token| {
            token.text == ";" && token.span.end as usize == source.find(" // trailing").unwrap()
        })
        .unwrap();
    assert_eq!(cst.comments[1].attachment, Attachment::Previous(previous));
}

#[test]
fn cst_token_identity_owns_each_comment_once() {
    use super::cst::Attachment;
    for source in [
        "// first\nfn F(){let x=1; // inline\n// before close\n}\n// eof",
        "fn F(){let x=(// inner\r\n1);}\r/// docs\r",
        "fn F(){let a=[1,// item\r2];}",
    ] {
        let cst = super::cst::parse_cst("identity.n", source).unwrap();
        for (index, token) in cst.tokens.iter().enumerate() {
            assert_eq!(token.id, index);
        }
        for comment in &cst.comments {
            match comment.attachment {
                Attachment::Previous(id) | Attachment::Next(id) => {
                    assert!(id < cst.tokens.len());
                    assert_ne!(cst.tokens[id].kind, super::lex::Kind::Eof);
                }
                Attachment::Eof => {
                    assert_eq!(cst.tokens.last().unwrap().kind, super::lex::Kind::Eof)
                }
            }
        }
        let formatted = super::format::format_cst("identity.n", &cst).unwrap();
        let reparsed = super::cst::parse_cst("identity.n", &formatted).unwrap();
        let literals = |cst: &super::cst::Cst| {
            cst.comments
                .iter()
                .map(|comment| cst.source[comment.start..comment.end].to_owned())
                .collect::<Vec<_>>()
        };
        assert_eq!(literals(&cst), literals(&reparsed));
        assert_eq!(
            super::format::format("identity.n", &formatted).unwrap(),
            formatted
        );
    }
}

#[test]
fn structural_format_rules_keep_binary_spaces_statement_lines_and_branch_anchors() {
    let cases = [
        ("fn F(x:move()){}", "fn F(x: move ()) {}\n"),
        (
            "fn F(){let a=1-2;let b=1<2; // tail\n}",
            "fn F() {\n    let a = 1 - 2;\n    let b = 1 < 2; // tail\n}\n",
        ),
        (
            "fn F(){if a{}let x=1; // tail\n}",
            "fn F() {\n    if a {}\n    let x = 1; // tail\n}\n",
        ),
        (
            "fn F(){let a=if b then {value:1;}else{value:2;}// tail\r;}",
            "fn F() {\n    let a = if b then 1 else 2; // tail\n}\n",
        ),
        (
            "fn F(){let a=if b then\n// before\n{value:1;} else 2;}",
            "fn F() {\n    let a = if b then\n    // before\n    {\n        value: 1;\n    } else 2;\n}\n",
        ),
    ];
    for (source, expected) in cases {
        let before = super::cst::parse_cst("structural.n", source).unwrap();
        let formatted = super::format::format_cst("structural.n", &before).unwrap();
        assert_eq!(formatted, expected, "{source:?}");
        let after = super::cst::parse_cst("structural.n", &formatted).unwrap();
        assert_eq!(
            ast_without_spans(before.ast.clone()),
            ast_without_spans(after.ast.clone())
        );
        let comments = |cst: &super::cst::Cst| {
            cst.comments
                .iter()
                .map(|comment| cst.source[comment.start..comment.end].to_owned())
                .collect::<Vec<_>>()
        };
        assert_eq!(comments(&before), comments(&after));
        assert_eq!(
            super::format::format("structural.n", &formatted).unwrap(),
            formatted
        );
    }
    let source = "fn F(){let x=(if b then 1 else{value:2;}// c\n);}";
    let before = super::cst::parse_cst("structural.n", source).unwrap();
    let formatted = super::format::format_cst("structural.n", &before).unwrap();
    assert!(formatted.contains("else {"), "{formatted:?}");
    assert!(formatted.contains("// c"), "{formatted:?}");
    let after = super::cst::parse_cst("structural.n", &formatted).unwrap();
    assert_eq!(ast_without_spans(before.ast), ast_without_spans(after.ast));
    assert_eq!(
        super::format::format("structural.n", &formatted).unwrap(),
        formatted
    );
}

#[test]
fn comments_in_branches_and_blocks_reparse_and_stabilize() {
    for source in [
        "fn F(){let x=if a then {value:1;} else//\n2;}",
        "fn F(){while if a then b else c //\n{}\n}",
        "fn F(){while if a then b else(//\n) {}}",
        "fn M(){let x=if a then {value:1 // branch\n} else 2;}",
        "fn M(){if a{}else if x<2{897\r\n// next\r\nzz;}}",
        "fn M(){b=if a then {} else {value:_u;}// branch\r\nc;}",
    ] {
        let original = parse("comments.n", source).unwrap();
        let formatted = super::format::format("comments.n", source).unwrap();
        let reparsed = parse("comments.n", &formatted)
            .unwrap_or_else(|error| panic!("{source:?} => {formatted:?}: {error:?}"));
        assert_eq!(
            ast_without_spans(original),
            ast_without_spans(reparsed),
            "{source:?} => {formatted:?}"
        );
        assert_eq!(
            formatted,
            super::format::format("comments.n", &formatted).unwrap()
        );
        let before = super::cst::parse_cst("comments.n", source).unwrap();
        let after = super::cst::parse_cst("comments.n", &formatted).unwrap();
        let literal = |cst: &super::cst::Cst| {
            cst.comments
                .iter()
                .map(|comment| cst.source[comment.start..comment.end].to_owned())
                .collect::<Vec<_>>()
        };
        assert_eq!(literal(&before), literal(&after));
    }
}

#[test]
fn comments_after_large_token_prefix_keep_canonical_branch_structure() {
    let prefix = "let p=1;".repeat(100);
    let source = format!("fn F(){{{prefix}let x=if a then {{value:1;}} else// branch\n2;}}");
    let formatted = super::format::format("large-comments.n", &source).unwrap();
    let reparsed = parse("large-comments.n", &formatted).unwrap();
    assert_eq!(
        ast_without_spans(parse("large-comments.n", &source).unwrap()),
        ast_without_spans(reparsed)
    );
    assert_eq!(
        formatted,
        super::format::format("large-comments.n", &formatted).unwrap()
    );
    assert!(formatted.contains("let p = 1;"));
}

#[test]
fn standalone_comments_before_delimiters_and_eof_keep_following_tokens() {
    for source in [
        "// only\r\n",
        "fn F(\n// parameter end\n){}",
        "fn F(){let a=[\n// array end\n];}",
        "fn F(){let x=(\n// unit end\n);}",
        "fn F(){while a\n// body open\n{}}",
        "fn F(){let x=if a then 1 else\n// branch\n2;}",
        "fn F(){}\n// eof",
    ] {
        let original = super::cst::parse_cst("standalone.n", source).unwrap();
        let formatted = super::format::format_cst("standalone.n", &original).unwrap();
        let reparsed = super::cst::parse_cst("standalone.n", &formatted)
            .unwrap_or_else(|error| panic!("{source:?} => {formatted:?}: {error:?}"));
        assert_eq!(
            ast_without_spans(original.ast.clone()),
            ast_without_spans(reparsed.ast.clone()),
            "{source:?} => {formatted:?}"
        );
        let literals = |cst: &super::cst::Cst| {
            cst.comments
                .iter()
                .map(|comment| cst.source[comment.start..comment.end].to_owned())
                .collect::<Vec<_>>()
        };
        assert_eq!(literals(&original), literals(&reparsed));
        assert_eq!(
            formatted,
            super::format::format("standalone.n", &formatted).unwrap()
        );
    }
}

#[test]
fn unavailable_printer_worker_returns_a_valid_budget_diagnostic() {
    use crate::diagnostic::{DataValue, Stage};

    let file = "printer-worker.n";
    let source = "fn F(){}";
    let ast = parse(file, source).unwrap();
    let unavailable = super::print::simulate_unavailable_worker(&ast).unwrap_err();
    let diagnostic = super::format::worker_diagnostic(file, source, unavailable);
    assert_eq!(diagnostic.code, "NX-E0003");
    assert_eq!(diagnostic.stage, Stage::Parse);
    assert_eq!(
        diagnostic.data.get("budget_kind"),
        Some(&DataValue::String("syntax_nesting".into()))
    );
    assert_eq!(diagnostic.data.get("limit"), Some(&DataValue::UInt(128)));
    assert_eq!(diagnostic.data.get("observed"), Some(&DataValue::UInt(129)));
    let span = diagnostic.primary_span.as_ref().unwrap();
    assert_eq!((&*span.file, span.start, span.end), (file, 0, 1));
    diagnostic.validate(file, source).unwrap();
}

#[test]
fn commented_format_regressions_do_not_panic_or_change_ast() {
    for source in [
        "fn F(){let x=(if a then 1 else//\ns//\n);}",
        "fn F(){(if a then 1 else value//\n);}",
    ] {
        let ast = parse("comments.n", source).unwrap();
        let formatted = super::format::format("comments.n", source).unwrap();
        let reparsed = parse("comments.n", &formatted)
            .unwrap_or_else(|error| panic!("{source:?} => {formatted:?}: {error:?}"));
        assert_eq!(
            super::test_normalize::ast_without_spans(ast),
            super::test_normalize::ast_without_spans(reparsed),
            "{source:?} => {formatted:?}"
        );
        assert_eq!(
            super::format::format("comments.n", &formatted).unwrap(),
            formatted
        );
    }
}

#[test]
fn consecutive_comments_at_omitted_else_parentheses_keep_source_order() {
    for source in [
        "fn M(){x=if a then 1 else ( //c\n//d\n(if b then 2 else 3));}",
        "fn M(){x=if s then a else ((if 1 then s else a)//c\n//d\n);}",
    ] {
        for newline in ["\n", "\r", "\r\n"] {
            let source = source.replace('\n', newline);
            assert_omitted_token_format(&source);
            let before = super::cst::parse_cst("comments.n", &source).unwrap();
            let formatted = super::format::format_cst("comments.n", &before).unwrap();
            let after = super::cst::parse_cst("comments.n", &formatted).unwrap();
            let texts = |cst: &super::cst::Cst| {
                cst.comments
                    .iter()
                    .map(|c| cst.source[c.start..c.end].to_owned())
                    .collect::<Vec<_>>()
            };
            assert_eq!(texts(&before), texts(&after), "{source:?} => {formatted:?}");
            assert_eq!(
                super::test_normalize::ast_without_spans(before.ast.clone()),
                super::test_normalize::ast_without_spans(after.ast),
                "{source:?} => {formatted:?}"
            );
            assert_eq!(
                super::format::format("comments.n", &formatted).unwrap(),
                formatted
            );
        }
    }
}

#[test]
fn formatter_preserves_comments_after_compact_if_expansion() {
    let source = "fn Main(){let x=if true then ( // next\n1) else 2;}";
    let ast = parse("comments.n", source).unwrap();
    let formatted = super::format::format("comments.n", source).unwrap();
    assert!(formatted.contains("// next"), "{formatted}");
    assert_eq!(
        ast_without_spans(ast),
        ast_without_spans(parse("comments.n", &formatted).unwrap()),
        "{source:?} => {formatted:?}"
    );
    assert_eq!(
        super::format::format("comments.n", &formatted).unwrap(),
        formatted
    );
}

#[test]
fn inserted_terminator_span_stops_before_comment_and_newline() {
    for newline in ["\n", "\r", "\r\n"] {
        for body in [
            "x",
            "let x=1",
            "var x=1;x=2",
            "return",
            "let x=if true then {value:1",
        ] {
            let suffix = if body.contains("if true") {
                "} else 2;"
            } else {
                ""
            };
            let source = format!("fn F(){{ {body} // c{newline}{suffix}}}");
            let ast = parse("span.n", &source).unwrap();
            let statement = if body.contains("if true") {
                let super::parse::Statement::Let {
                    value: super::parse::Expr::If { then_block, .. },
                    ..
                } = &ast.functions[0].body.statements[0]
                else {
                    panic!("expected branch")
                };
                &then_block.statements[0]
            } else {
                ast.functions[0].body.statements.last().unwrap()
            };
            let span = match statement {
                super::parse::Statement::Let { span, .. }
                | super::parse::Statement::Assign { span, .. }
                | super::parse::Statement::Return { span, .. }
                | super::parse::Statement::Value { span, .. }
                | super::parse::Statement::Expr { span, .. } => span,
                _ => panic!("unexpected statement"),
            };
            let end = source.find(" // c").unwrap();
            let start = if body.contains("if true") {
                source.find("value:").unwrap()
            } else if body.contains("x=2") {
                source.find("x=2").unwrap()
            } else {
                8
            };
            assert_eq!(
                (span.start as usize, span.end as usize),
                (start, end),
                "{source:?}"
            );
        }
    }
}

#[test]
fn every_terminated_statement_ends_at_its_last_token_before_trivia() {
    use super::parse::Statement;

    for (kind, body) in [
        ("let", "let x=1"),
        ("assign", "var x=0;x=1"),
        ("return", "return"),
        ("value", "let x=if true then {value:1"),
        ("expr", "x"),
    ] {
        for (trivia, written) in [
            ("\n", false),
            ("\r", false),
            ("\r\n", false),
            (" // c\n", false),
            ("; // c\n", true),
        ] {
            let suffix = if kind == "value" { "} else 2;" } else { "" };
            // The final function brace is followed immediately by EOF.
            let source = format!("fn F(){{{body}{trivia}{suffix}}}");
            let ast = parse("statement-span.n", &source).unwrap();
            let statement = if kind == "value" {
                let Statement::Let {
                    value: super::parse::Expr::If { then_block, .. },
                    ..
                } = &ast.functions[0].body.statements[0]
                else {
                    panic!("expected let with if expression")
                };
                &then_block.statements[0]
            } else {
                ast.functions[0].body.statements.last().unwrap()
            };
            let span = match (kind, statement) {
                ("let", Statement::Let { span, .. })
                | ("assign", Statement::Assign { span, .. })
                | ("return", Statement::Return { span, .. })
                | ("value", Statement::Value { span, .. })
                | ("expr", Statement::Expr { span, .. }) => span,
                _ => panic!("unexpected {kind} statement: {statement:?}"),
            };
            let last = if kind == "assign" {
                "x=1"
            } else if kind == "value" {
                "value:1"
            } else {
                body
            };
            let start = source.find(last).unwrap();
            let end = start + last.len() + usize::from(written);
            assert_eq!(
                (span.start as usize, span.end as usize),
                (start, end),
                "{kind}: {source:?}"
            );
            assert!(source.ends_with('}'));
        }
    }
}

#[test]
fn signed_name_points_at_name_after_minus() {
    let d = parse("signed.n", "fn Main(){let a=-x;}").unwrap_err();
    assert_eq!(d.code, "NX-E0002");
    assert_eq!(d.parser_context.as_deref(), Some("signed_integer"));
    assert_eq!(
        d.unexpected_token.as_ref().unwrap().text.as_deref(),
        Some("x")
    );
    assert_eq!(d.primary_span.as_ref().unwrap().start, 17);
    assert_eq!(d.primary_span.as_ref().unwrap().end, 18);
    assert_eq!(d.recognized_prefix.as_ref().unwrap().start, 16);
    assert_eq!(d.recognized_prefix.as_ref().unwrap().end, 17);
}

#[test]
fn explicit_surface_boundaries_and_printer_rules() {
    use crate::diagnostic::TokenKind;
    let spaced = super::lex::lex("space.n", "a\u{0085}b").unwrap();
    assert_eq!(
        spaced
            .iter()
            .map(|token| token.text.as_str())
            .collect::<Vec<_>>(),
        ["a", "b", ""]
    );

    for (mode, context) in [("move", "consume_place"), ("edit", "inout_place")] {
        let source = format!("fn Main(){{let x={mode} [1];}}");
        let error = parse("prefix.n", &source).unwrap_err();
        assert_eq!(error.code, "NX-E0002");
        assert_eq!(error.parser_context.as_deref(), Some(context));
        assert_eq!(
            error.unexpected_token.as_ref().unwrap().text.as_deref(),
            Some("[")
        );
    }
    let error = parse("cr.n", "fn Main(){if true{value\r:1;}else{}}").unwrap_err();
    assert_eq!(error.code, "NX-E0002");
    assert_eq!(
        error.unexpected_token.as_ref().unwrap().text.as_deref(),
        Some(":")
    );
    let error = parse("else.n", "fn Main(){if true{}else 1;}").unwrap_err();
    assert_eq!(error.code, "NX-E0002");
    assert_eq!(error.parser_context.as_deref(), Some("grammar.IfForm"));
    assert_eq!(error.expected_tokens, ["if", "{"]);
    assert_eq!(
        error.unexpected_token.as_ref().unwrap().kind,
        TokenKind::Integer
    );

    assert!(
        print_canonical(&parse("print.n", "fn Main(){let a=[1,2];}").unwrap())
            .unwrap()
            .contains("[1, 2]")
    );
    let nested = print_canonical(&parse("print.n", "fn Main(){{{let a=1;}}}").unwrap()).unwrap();
    assert!(
        nested.contains("\n    {\n        {\n            let a = 1;\n"),
        "{nested:?}"
    );
    let mut ungrouped_right = parse("print.n", "fn Main(){let a=1+(2+3);}").unwrap();
    let super::parse::Statement::Let {
        value: super::parse::Expr::Binary { right, .. },
        ..
    } = &mut ungrouped_right.functions[0].body.statements[0]
    else {
        panic!("expected binary")
    };
    let super::parse::Expr::Group(inner, _) = &**right else {
        panic!("expected group")
    };
    **right = (**inner).clone();
    assert!(
        print_canonical(&ungrouped_right)
            .unwrap()
            .contains("1 + (2 + 3)")
    );

    let source = "fn Main(){let a=1// first\rlet b=2// second\r}";
    let formatted = super::format::format("cr.n", source).unwrap();
    assert!(formatted.contains("// first\n"));
    assert!(formatted.contains("// second\n"));
    assert!(formatted.find("// first") < formatted.find("// second"));
    assert_eq!(
        super::format::format("cr.n", &formatted).unwrap(),
        formatted
    );
    let source = "fn Main(){let a=1// first\n;// second\n}";
    let formatted = super::format::format("order.n", source).unwrap();
    assert!(formatted.find("// first") < formatted.find("// second"));
    assert_eq!(
        super::cst::parse_cst("order.n", &formatted)
            .unwrap()
            .comments
            .len(),
        2
    );
    assert_eq!(
        super::format::format("order.n", &formatted).unwrap(),
        formatted
    );
}

#[test]
fn adr_gap_edit_outside_admitted_context() {
    use crate::diagnostic::{ExpectedTokensKind, Operation, Stage, TokenKind};
    for body in [
        "var b=edit r;",
        "r=edit r;",
        "[edit r];",
        "edit r;",
        "let b=1+edit r;",
        "let b=edit r+1;",
        "let b=(edit r);",
        "let b=[1][edit r];",
        "return edit r;",
        "if edit r {}",
        "while edit r {}",
        "{edit r;}",
        "let b=if true then edit r else 1;",
        "let b=(edit r)?;",
        "Print(1+edit r);",
        "Print([edit r]);",
        "Print((edit r));",
        "let b=if true then {value: edit r;} else 1;",
        "var b=edit r;Print(1);",
        "var b=edit r;var c=edit r;",
        "if true {edit r;} else {}",
        "if true {} else {edit r;}",
        "while false {edit r;}",
        "let b=if true then 1 else {value: edit r;};",
        "let b=if true then 1 else (if false then edit r else 2);",
    ] {
        let source = format!("fn Main(){{var r=1;{body}}}");
        let error = parse("edit.n", &source).unwrap_err();
        let start = source.find("edit r").unwrap() as u64;
        assert_eq!(error.code, "NX-E0002", "{source}");
        assert_eq!(error.subcause, None);
        assert_eq!(error.stage, Stage::Parse);
        assert_eq!(error.operation, Operation::SyntaxParse);
        assert_eq!(
            error.primary_span.as_ref().map(|s| (s.start, s.end)),
            Some((start, start + 4))
        );
        assert_eq!(error.parser_context.as_deref(), Some("inout_place"));
        assert_eq!(error.expected_tokens, ["Place"]);
        assert_eq!(
            error.expected_tokens_kind,
            Some(ExpectedTokensKind::AdmissibleSet)
        );
        assert_eq!(
            error
                .unexpected_token
                .as_ref()
                .map(|t| (&t.kind, t.text.as_deref())),
            Some((&TokenKind::Name, Some("edit")))
        );
        assert_eq!(
            error.recognized_prefix.as_ref().map(|s| (s.start, s.end)),
            Some((start, start))
        );
        assert!(error.feature_id.is_none());
        assert!(error.data.is_empty());
        assert!(error.related_spans.is_empty());
        assert!(error.cause_chain.is_empty());
        assert!(error.repair.is_empty());
        error.validate("edit.n", &source).unwrap();
    }
    for source in [
        "fn F(x:edit I64){} fn Main(){var r=1;F(edit r);}",
        "fn Main(){let r=1;let b=move r;}",
        "fn Main(){let view=1;let b=view;}",
    ] {
        assert!(parse("edit.n", source).is_ok(), "{source}");
    }
}

#[test]
fn edit_cannot_become_a_call_or_index_target() {
    let source = "fn Main(){var r=1;edit r(1);}";
    let error = parse("target.n", source).unwrap_err();
    assert_eq!(error.code, "NX-E0006");
    assert_eq!(error.feature_id.as_deref(), Some("receiver_mode"));
    error.validate("target.n", source).unwrap();

    for source in [
        "fn Main(){var r=1;(edit r)(1);}",
        "fn Main(){var r=[1];let b=(edit r)[0];}",
    ] {
        let error = parse("target.n", source).unwrap_err();
        assert_eq!(error.code, "NX-E0002");
        assert_eq!(error.parser_context.as_deref(), Some("inout_place"));
        error.validate("target.n", source).unwrap();
    }
}

#[test]
fn structural_budget_boundaries_are_exact() {
    use crate::diagnostic::DataValue;
    let assert_budget = |source: &str, kind: &str| {
        let error = parse("budget.n", source).unwrap_err();
        assert_eq!(error.code, "NX-E0003", "{kind}: {error:?}");
        assert_eq!(
            error.data.get("budget_kind"),
            Some(&DataValue::String(kind.into()))
        );
        assert!(error.validate("budget.n", source).is_ok());
        error
    };
    let ty = |n: usize| {
        format!(
            "fn F(x:view {}I64{}){{}}",
            "Array<".repeat(n),
            ">".repeat(n)
        )
    };
    assert!(parse("budget.n", &ty(127)).is_ok());
    assert_budget(&ty(128), "syntax_nesting");
    let blocks = |n: usize| format!("fn Main(){{{}{} }}", "{".repeat(n), "}".repeat(n));
    assert!(parse("budget.n", &blocks(127)).is_ok());
    let block_error = assert_budget(&blocks(128), "syntax_nesting");
    assert_eq!(block_error.data.get("limit"), Some(&DataValue::UInt(128)));
    assert_eq!(
        block_error.data.get("observed"),
        Some(&DataValue::UInt(129))
    );
    let arrays = |n: usize| format!("fn Main(){{let x={}1{};}}", "[".repeat(n), "]".repeat(n));
    let array_error = assert_budget(&arrays(200), "syntax_nesting");
    assert_eq!(
        array_error
            .primary_span
            .as_ref()
            .map(|span| (span.start, span.end)),
        Some((143, 144))
    );
    let assignment_arrays = |n: usize| {
        format!(
            "fn Main(){{var x=0;x={}1{};}}",
            "[".repeat(n),
            "]".repeat(n)
        )
    };
    assert!(parse("budget.n", &assignment_arrays(0)).is_ok());
    let assignment_error = assert_budget(&assignment_arrays(200), "syntax_nesting");
    assert_eq!(
        assignment_error
            .primary_span
            .as_ref()
            .map(|span| (span.start, span.end)),
        Some((147, 148))
    );
    let binary = |n: usize| format!("fn Main(){{let x=1{};}}", "+1".repeat(n));
    assert!(parse("budget.n", &binary(127)).is_ok());
    assert_budget(&binary(128), "expression_tree_depth");

    let mut functions = String::new();
    for index in 0..1_024 {
        functions.push_str(&format!("fn F{index}(){{}}"));
    }
    assert!(parse("budget.n", &functions).is_ok());
    let rejected_start = functions.len();
    functions.push_str("fn F1024(){}");
    let error = assert_budget(&functions, "function_count");
    assert_eq!(
        error
            .primary_span
            .as_ref()
            .map(|span| (span.start as usize, span.end as usize)),
        Some((rejected_start + 3, rejected_start + 8))
    );

    let source = format!(
        "fn Main(){{}}{}",
        " ".repeat(1_048_577 - "fn Main(){}".len())
    );
    assert_eq!(source.len(), 1_048_577);
    assert!(parse("budget.n", &source[..1_048_576]).is_ok());
    let error = assert_budget(&source, "source_bytes");
    assert_eq!(
        error
            .primary_span
            .as_ref()
            .map(|span| (span.start, span.end)),
        Some((1_048_576, 1_048_577))
    );
}

#[test]
fn composite_expression_depth_uses_the_published_source_boundary() {
    use crate::diagnostic::{DataValue, Stage};

    const TERM: &str = "(if true then 1 else 1)";
    let source = |count: usize| format!("fn Main(){{let x={};}}", vec![TERM; count].join("+"));
    let accepted = source(127);
    assert_eq!(accepted.len(), 3_065);
    assert!(parse("depth.n", &accepted).is_ok());

    let rejected = source(128);
    assert_eq!(rejected.len(), 3_089);
    let error = parse("depth.n", &rejected).unwrap_err();
    assert_eq!(error.code, "NX-E0003");
    assert_eq!(error.stage, Stage::Parse);
    assert_eq!(
        error.data.get("budget_kind"),
        Some(&DataValue::String("expression_tree_depth".into()))
    );
    assert_eq!(error.data.get("limit"), Some(&DataValue::UInt(128)));
    assert_eq!(error.data.get("observed"), Some(&DataValue::UInt(129)));
    assert_eq!(
        error
            .primary_span
            .as_ref()
            .map(|span| (span.start, span.end)),
        Some((16, 3087))
    );
    error.validate("depth.n", &rejected).unwrap();
}

fn assert_adr_gap(source: &str, code: &str, context: &str, token: &str, span: (u64, u64)) {
    let error = parse("gap.n", source).unwrap_err();
    assert_eq!(error.code, code);
    assert_eq!(error.parser_context.as_deref(), Some(context));
    assert_eq!(
        error.unexpected_token.as_ref().unwrap().text.as_deref(),
        Some(token)
    );
    assert_eq!(
        error
            .primary_span
            .as_ref()
            .map(|span| (span.start, span.end)),
        Some(span)
    );
}

#[test]
fn adr_gap_grouped_non_place_points_at_inner_operand() {
    assert_adr_gap(
        "fn Main(){let x=move (1);}",
        "NX-E0002",
        "consume_place",
        "1",
        (22, 23),
    );
}

#[test]
fn adr_gap_nested_move_edit_points_at_edit() {
    assert_adr_gap(
        "fn Main(){let x=move edit x;}",
        "NX-E0002",
        "consume_place",
        "edit",
        (21, 25),
    );
}

#[test]
fn adr_gap_return_at_eof_is_missing_terminator() {
    assert_adr_gap(
        "fn Main(){return",
        "NX-E0005",
        "statement_terminator",
        "",
        (16, 16),
    );
}

#[test]
fn adr_gap_try_followed_by_index_is_try_expr_syntax() {
    assert_adr_gap(
        "fn Main(){let y=x?[0];}",
        "NX-E0002",
        "grammar.TryExpr",
        "[",
        (18, 19),
    );
}

#[test]
fn adr_gap_public_lexer_emits_one_crlf_newline_token() {
    // §2.1 calls CRLF one logical newline. The public lex API exposes it as
    // one token spanning both source bytes; CST and parser ignore the split.
    let tokens = super::lex::lex("gap.n", "x\r\ny").unwrap();
    assert_eq!(tokens.len(), 4);
    assert_eq!(tokens[1].kind, super::lex::Kind::Newline);
    assert_eq!(tokens[1].text, "\r\n");
    assert_eq!((tokens[1].span.start, tokens[1].span.end), (1, 3));
}

#[test]
#[ignore = "large directed comment campaign"]
fn generated_comment_positions_preserve_ast_order_and_idempotence() {
    let _campaign = crate::test_alloc::campaign_guard();
    let seeds = [
        "fn F(){let x=(if a then 1 else s);}",
        "fn F(){(if a then 1 else value);}",
        "fn Main(){let x=1;let y=x+2;}",
        "fn Main(){let x=if true then {value:1;}else{value:2;};}",
        "fn Main(){var a=[1,2];a[0]=3;}",
        "fn F(x:view I64)->I64{return x;}fn Main(){F(1);}",
        "fn F(){let x=if a then {value:1;} else 2;}",
        "fn F(){while if a then b else c {}}",
        "fn F(){while if a then b else() {}}",
        "fn F(x:edit I64,y:move I64){} fn Main(){var a=1;F(edit a,move a);}",
        "fn F(){let a=[1,2,3];let b=F(a[0],a[1]);}",
        "fn F(){let a=if b then {value:1;}else{value:2;};}",
        "fn F(){while (if a then b else c) {}}",
    ];
    let tokens: Vec<Vec<_>> = seeds
        .iter()
        .map(|seed| {
            super::lex::lex("comments.n", seed)
                .unwrap()
                .into_iter()
                .filter(|token| {
                    !matches!(
                        token.kind,
                        super::lex::Kind::Eof | super::lex::Kind::Newline
                    )
                })
                .collect()
        })
        .collect();
    let mut accepted = 0;
    let mut rejected = Vec::new();
    let mut state = 0xC0_11_EC_75_u64;
    for n in 0..120_000_usize {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        let seed_index = n % seeds.len();
        let mut source = seeds[seed_index].to_owned();
        let positions = &tokens[seed_index];
        let first = (state as usize) % positions.len();
        let second = ((state >> 19) as usize) % positions.len();
        let newline = ["\n", "\r", "\r\n"][n % 3];
        let standalone = if n % 4 == 0 { newline } else { "" };
        let mut comments = vec![(
            positions[first].span.end as usize,
            format!("{standalone}// c{n}a{newline}"),
        )];
        if n % 3 == 0 && first != second {
            comments.push((
                positions[second].span.end as usize,
                format!("{standalone}// c{n}b{newline}"),
            ));
        }
        comments.sort_by_key(|item| item.0);
        for (at, comment) in comments.iter().rev() {
            source.insert_str(*at, comment);
        }
        let original = match parse("comments.n", &source) {
            Ok(ast) => ast,
            Err(error) => {
                assert!(error.validate("comments.n", &source).is_ok(), "{n}");
                assert!(super::format::format("comments.n", &source).is_err(), "{n}");
                rejected.push(format!(
                    "{n}:{}:{}",
                    error.code,
                    error.parser_context.as_deref().unwrap_or("")
                ));
                continue;
            }
        };
        accepted += 1;
        let formatted = super::format::format("comments.n", &source).unwrap();
        let reparsed = parse("comments.n", &formatted)
            .unwrap_or_else(|error| panic!("{n} {source:?} => {formatted:?}: {error:?}"));
        assert_eq!(
            super::test_normalize::ast_without_spans(original),
            super::test_normalize::ast_without_spans(reparsed),
            "{n} {source:?} => {formatted:?}"
        );
        assert_eq!(
            super::format::format("comments.n", &formatted).unwrap(),
            formatted,
            "{n} {source:?}"
        );
        let input_comments = super::cst::parse_cst("comments.n", &source).unwrap();
        let output_comments = super::cst::parse_cst("comments.n", &formatted).unwrap();
        let literals = |cst: &super::cst::Cst| {
            cst.comments
                .iter()
                .map(|comment| cst.source[comment.start..comment.end].to_owned())
                .collect::<Vec<_>>()
        };
        assert_eq!(
            literals(&input_comments),
            literals(&output_comments),
            "{n} {source:?} => {formatted:?}"
        );
    }
    assert_eq!(accepted, 115_232);
    assert_eq!(accepted + rejected.len(), 120_000);
    assert_closed_rejections(&rejected, include_str!("random-comment-rejections.txt"));
    println!("accepted commented programs: {accepted}/120_000");
}

#[test]
#[ignore = "systematic F0 comment gap campaign"]
fn every_concrete_gap_preserves_comments_and_tree() {
    let _campaign = crate::test_alloc::campaign_guard();
    let adr = include_str!("../../docs/adr/F0-D15-sintaxis-oficial.md");
    let mut programs = Vec::new();
    let mut rejected_programs = Vec::new();
    let table = adr
        .split_once("### 8.7.")
        .unwrap()
        .1
        .split_once("### 8.8.")
        .unwrap()
        .0;
    for line in table.lines().filter(|line| line.starts_with("| F2-")) {
        let source = line.split('`').nth(1).unwrap().replace(['⟦', '⟧'], "");
        match parse("gaps.n", &source) {
            Ok(_) => programs.push(source),
            Err(error) => rejected_programs.push(format!(
                "{}:{}:{}",
                line.split(" | ").next().unwrap_or("F2"),
                error.code,
                error.parser_context.as_deref().unwrap_or("")
            )),
        }
    }
    for section in [2, 3, 4, 5, 6, 8, 9, 10] {
        let body = adr
            .split_once(&format!("### 8.{section}."))
            .unwrap()
            .1
            .split_once(&format!("### 8.{}.", section + 1))
            .unwrap()
            .0;
        for (block_id, block) in body.split("```nexo\n").skip(1).enumerate() {
            let source = block.split_once("\n```").unwrap().0.replace(['⟦', '⟧'], "");
            match parse("gaps.n", &source) {
                Ok(_) => programs.push(source),
                Err(error) => rejected_programs.push(format!(
                    "8.{section}.{block_id}:{}:{}",
                    error.code,
                    error.parser_context.as_deref().unwrap_or("")
                )),
            }
        }
    }
    let table = adr
        .split_once("### 8.12.")
        .unwrap()
        .1
        .split_once("### 8.13.")
        .unwrap()
        .0;
    for line in table
        .lines()
        .filter(|line| line.starts_with("| T0") || line.starts_with("| D0"))
    {
        let literal_cell = line.split(" | ").nth(1).unwrap();
        for (literal_id, source) in literal_cell.split('`').skip(1).step_by(2).enumerate() {
            match parse("gaps.n", source) {
                Ok(_) => programs.push(source.to_owned()),
                Err(error) => rejected_programs.push(format!(
                    "{}.{literal_id}:{}:{}",
                    line.split(" | ").next().unwrap_or("row"),
                    error.code,
                    error.parser_context.as_deref().unwrap_or("")
                )),
            }
        }
    }
    assert_closed_rejections(
        &rejected_programs,
        include_str!("corpus-comment-rejections.txt"),
    );
    let corpus_programs = programs.len();
    assert_eq!(corpus_programs + rejected_programs.len(), 160);
    let templates = [
        "fn F()->I64{{let x={n};return x;}}",
        "fn F(x:view I64,y:edit I64,z:move I64)->I64{{return x+{n};}}",
        "fn F(){{let a=[{n},2,3];let b=a[0];}}",
        "fn F(){{let x=if a then {{value:{n};}} else {{value:2;}};}}",
        "fn F(){{while if a then b else c {{let x={n};}}}}",
        "fn F(){{let x=F(a: {n}, b: move y);}}",
        "fn F(){{let x=({n}+2);}}",
        "fn F(){{if a {{return;}} else {{let x={n};}}}}",
        "fn F() with {{call(no_io)}}{{return;}}",
        "fn F(x:view Result<Array<I64>,Bool>)->I64{{return {n};}}",
        "fn F(){{var x:I64={n};x={n};}}",
        "fn F(){{F({n});return;}}",
        "fn F(){{if a {{}} else if b {{}} else {{}}}}",
        "fn F(){{(a)[0]={n};}}",
        "fn F(){{let a=true;let b=false;let c=();}}",
        "fn F(){{let a=-{n}+2;let b=a<3;let c=a==2;}}",
        "fn F(){{let x=F()?;}}",
        "fn F(){{F(edit a[0]);let y=move b;}}",
        "fn F() with no_io{{return;}}",
        "fn F(a:view I64,b:edit Bool,c:move (),d:view MockResource,e:view MockBlockingResource,f:view Array<I64>,g:view Result<I64,Bool>){{}}",
    ];
    for n in 0..5_000 {
        let source = templates[n % templates.len()]
            .replace("{{", "{")
            .replace("}}", "}")
            .replace("{n}", &(n % 97).to_string());
        let parsed = parse("gaps.n", &source);
        assert!(
            parsed.is_ok(),
            "template {}: {source}: {:?}",
            n % templates.len(),
            parsed.err()
        );
        programs.push(source);
    }
    let mut accepted_insertions = 0usize;
    let mut attempted_insertions = 0usize;
    let mut rejected_insertions = Vec::new();
    for (program_index, source) in programs.iter().enumerate() {
        let ast = parse("gaps.n", source)
            .unwrap_or_else(|error| panic!("generated template rejected: {source}: {error:?}"));
        let canonical = super::print::print_canonical(&ast).unwrap();
        if super::cst::parse_cst("gaps.n", source)
            .unwrap()
            .comments
            .is_empty()
        {
            assert_eq!(super::format::format("gaps.n", source).unwrap(), canonical);
        }
        let tokens = super::lex::lex("gaps.n", source).unwrap();
        let mut gaps = vec![0usize];
        gaps.extend(
            tokens
                .iter()
                .filter(|token| token.kind != super::lex::Kind::Newline)
                .map(|token| token.span.end as usize),
        );
        gaps.sort_unstable();
        gaps.dedup();
        for gap in gaps {
            for (newline_id, newline) in ["\n", "\r"].into_iter().enumerate() {
                for (prefix_id, prefix) in ["", newline].into_iter().enumerate() {
                    attempted_insertions += 1;
                    let mut inserted = source.clone();
                    inserted.insert_str(gap, &format!("{prefix}// gap{program_index}{newline}"));
                    let original = match parse("gaps.n", &inserted) {
                        Ok(ast) => ast,
                        Err(error) => {
                            rejected_insertions.push(format!(
                                "{program_index},{gap},{newline_id},{prefix_id}:{}:{}",
                                error.code,
                                error.parser_context.as_deref().unwrap_or("")
                            ));
                            continue;
                        }
                    };
                    accepted_insertions += 1;
                    let formatted = super::format::format("gaps.n", &inserted).unwrap();
                    let reparsed = parse("gaps.n", &formatted).unwrap_or_else(|error| {
                        panic!("{program_index} gap {gap} {inserted:?} => {formatted:?}: {error:?}")
                    });
                    assert_eq!(
                        super::test_normalize::ast_without_spans(original),
                        super::test_normalize::ast_without_spans(reparsed),
                        "{program_index} gap {gap} {inserted:?} => {formatted:?}"
                    );
                    assert_eq!(
                        super::format::format("gaps.n", &formatted).unwrap(),
                        formatted
                    );
                    let before = super::cst::parse_cst("gaps.n", &inserted).unwrap();
                    let after = super::cst::parse_cst("gaps.n", &formatted).unwrap();
                    let comments = |cst: &super::cst::Cst| {
                        cst.comments
                            .iter()
                            .map(|comment| cst.source[comment.start..comment.end].to_owned())
                            .collect::<Vec<_>>()
                    };
                    assert_eq!(comments(&before), comments(&after));
                }
            }
        }
    }
    println!(
        "corpus programs: {corpus_programs}; generated: 5000; insertions: {attempted_insertions}; accepted: {accepted_insertions}; failures: 0"
    );
    assert_closed_rejections(
        &rejected_insertions,
        include_str!("gap-comment-rejections.txt"),
    );
    assert_eq!(corpus_programs, 98);
    assert_eq!(programs.len(), 5_098);
    assert_eq!(attempted_insertions, 446_220);
    assert_eq!(accepted_insertions, 442_156);
    assert_eq!(
        accepted_insertions + rejected_insertions.len(),
        attempted_insertions
    );
}

#[test]
fn adr_literal_f2_table_rows_reach_the_published_frontend_boundary() {
    // §8.7 rows whose published result is decided after parsing:
    // F2-S01…S21, S23…S26, S29; P01…P11; PP01…PP05; R01…R03;
    // B02/B03/B09; E01…E06 including E05/no_alloc, /no_block, /no_io; F01.
    const LATER_ROWS: &[&str] = &[
        "F2-S01",
        "F2-S02",
        "F2-S03",
        "F2-S04",
        "F2-S05",
        "F2-S06",
        "F2-S07",
        "F2-S08",
        "F2-S09",
        "F2-S10",
        "F2-S11",
        "F2-S12",
        "F2-S13",
        "F2-S14",
        "F2-S15",
        "F2-S16",
        "F2-S17",
        "F2-S18",
        "F2-S19",
        "F2-S20",
        "F2-S21",
        "F2-S23",
        "F2-S24",
        "F2-S25",
        "F2-S26",
        "F2-S29",
        "F2-P01",
        "F2-P02",
        "F2-P03",
        "F2-P04",
        "F2-P05",
        "F2-P06",
        "F2-P07",
        "F2-P08",
        "F2-P09",
        "F2-P10",
        "F2-P11",
        "F2-PP01",
        "F2-PP02",
        "F2-PP03",
        "F2-PP04",
        "F2-PP05",
        "F2-R01",
        "F2-R02",
        "F2-R03",
        "F2-B02",
        "F2-B03",
        "F2-B09",
        "F2-E01",
        "F2-E02",
        "F2-E03",
        "F2-E04",
        "F2-E05",
        "F2-E06",
        "F2-F01",
        "F2-E05/no_alloc",
        "F2-E05/no_block",
        "F2-E05/no_io",
    ];
    assert_eq!(LATER_ROWS.len(), 58);
    let adr = include_str!("../../docs/adr/F0-D15-sintaxis-oficial.md");
    let table = adr
        .split_once("### 8.7. Fuentes F2")
        .unwrap()
        .1
        .split_once("### 8.8.")
        .unwrap()
        .0;
    let mut ids = std::collections::BTreeSet::new();
    for line in table.lines().filter(|line| line.starts_with("| F2-")) {
        let id = line
            .trim_start_matches("| ")
            .split_whitespace()
            .next()
            .unwrap();
        let literal = line.split('`').nth(1).expect("source literal in ADR row");
        let source = literal.replace(['⟦', '⟧'], "");
        assert!(ids.insert(id.to_owned()), "repeated ADR row {id}");
        let expected = match id {
            "F2-S22" | "F2-S27" | "F2-S28" | "F2-S30" | "F2-S31" | "F2-S32" | "F2-S33"
            | "F2-S34" => Some("NX-E0002"),
            "F2-B01" | "F2-B04" | "F2-B05" | "F2-B06" => Some("NX-E1302"),
            "F2-B07" | "F2-B08" => Some("NX-E0005"),
            "F2-W01" | "F2-W02" | "F2-W03" | "F2-W04" | "F2-W05" | "F2-W06" | "F2-W07" => {
                Some("NX-E0417")
            }
            _ if id.starts_with("F2-F") && id != "F2-F01" => Some("NX-E0006"),
            _ => None,
        };
        match (expected, parse("adr-f2.n", &source)) {
            (None, Ok(ast)) => {
                assert!(
                    LATER_ROWS.contains(&id),
                    "unclassified later-stage row {id}"
                );
                let expected_names: &[&str] = match id {
                    "F2-S13" => &["F", "F", "Main"],
                    "F2-PP01" => &["Leer", "Main"],
                    "F2-PP02" => &["Inc", "Dos", "Main"],
                    "F2-PP03" | "F2-E02" => &["Copia", "Main"],
                    "F2-E04" => &["Hoja", "Medio", "Main"],
                    "F2-S01" | "F2-S02" | "F2-S03" | "F2-S04" | "F2-S05" | "F2-S06" | "F2-S08"
                    | "F2-S11" | "F2-S12" | "F2-S14" | "F2-S24" | "F2-P04" | "F2-P05"
                    | "F2-P06" | "F2-P07" | "F2-P08" | "F2-P09" | "F2-B02" | "F2-B03"
                    | "F2-B09" => &["F", "Main"],
                    _ => &["Main"],
                };
                assert_eq!(
                    ast.functions
                        .iter()
                        .map(|f| f.name.as_str())
                        .collect::<Vec<_>>(),
                    expected_names,
                    "{id}: AST roots"
                );
            }
            (Some(code), Err(error)) => {
                assert_eq!(error.code, code, "{id}: {source}");
                assert!(error.validate("adr-f2.n", &source).is_ok(), "{id}");
            }
            (None, Err(error)) => panic!("{id}: expected parse success for {source}: {error:?}"),
            (Some(code), Ok(_)) => panic!("{id}: expected {code} for {source}"),
        }
    }
    assert_eq!(ids.len(), 111, "F0-D15 §8.7 literal table rows");
}

#[test]
fn adr_literal_program_blocks_are_all_accounted_for_at_parse_boundary() {
    let adr = include_str!("../../docs/adr/F0-D15-sintaxis-oficial.md");
    let cases: &[(usize, &[Option<&str>])] = &[
        (2, &[Some("NX-E0006")]),
        (3, &[Some("NX-E0006")]),
        (4, &[Some("NX-E0006")]),
        (5, &[Some("NX-E0006")]),
        (6, &[Some("NX-E0006"); 4]),
        (8, &[None; 5]),
        (9, &[None; 7]),
        (10, &[None, Some("NX-E1302"), None, None, None, None, None]),
    ];
    let mut seen = 0;
    for &(section, expected) in cases {
        let heading = format!("### 8.{section}.");
        let body = adr
            .split_once(&heading)
            .unwrap()
            .1
            .split_once(&format!("### 8.{}.", section + 1))
            .unwrap()
            .0;
        let blocks: Vec<_> = body
            .split("```nexo\n")
            .skip(1)
            .map(|part| part.split_once("\n```").unwrap().0)
            .collect();
        assert_eq!(blocks.len(), expected.len(), "§8.{section}");
        for (index, (&code, source)) in expected.iter().zip(blocks).enumerate() {
            let source = source.replace(['⟦', '⟧'], "");
            match (code, parse("adr-block.n", &source)) {
                (None, Ok(_)) => {}
                (Some(code), Err(error)) => {
                    assert_eq!(error.code, code, "§8.{section} block {index}");
                    assert!(error.validate("adr-block.n", &source).is_ok());
                }
                (None, Err(error)) => panic!("§8.{section} block {index}: {error:?}"),
                (Some(code), Ok(_)) => panic!("§8.{section} block {index}: expected {code}"),
            }
            seen += 1;
        }
    }
    assert_eq!(seen, 27);
}

#[test]
fn adr_inference_and_placement_rows_parse_the_literal_b_sources() {
    // §8.12 T01…T07, T04/T05 negatives, T07 contamination, D01…D08:
    // All 20 rows are types/names/ownership/runtime decisions.
    let adr = include_str!("../../docs/adr/F0-D15-sintaxis-oficial.md");
    let table = adr
        .split_once("### 8.12.")
        .unwrap()
        .1
        .split_once("### 8.13.")
        .unwrap()
        .0;
    let mut ids = std::collections::BTreeSet::new();
    let mut programs = 0;
    for line in table
        .lines()
        .filter(|line| line.starts_with("| T0") || line.starts_with("| D0"))
    {
        let mut cells = line.split(" | ");
        let id = cells.next().unwrap().trim_start_matches("| ");
        let sources = cells.next().expect("literal source cell");
        assert!(ids.insert(id.to_owned()), "duplicate §8.12 row {id}");
        for source in sources.split('`').skip(1).step_by(2) {
            let ast = parse("adr-inference.n", source)
                .unwrap_or_else(|error| panic!("{id}: {source}: {error:?}"));
            let names: &[&str] = if id == "D03" {
                &["F", "Main"]
            } else {
                &["Main"]
            };
            assert_eq!(
                ast.functions
                    .iter()
                    .map(|f| f.name.as_str())
                    .collect::<Vec<_>>(),
                names,
                "{id}: AST roots"
            );
            assert_eq!(
                super::test_normalize::ast_without_spans(ast.clone()),
                super::test_normalize::ast_without_spans(
                    parse("adr-inference.n", &print_canonical(&ast).unwrap()).unwrap()
                ),
                "{id}"
            );
            programs += 1;
        }
    }
    assert_eq!(ids.len(), 20);
    assert_eq!(programs, 22);
}

#[test]
fn cst_formatter_keeps_comments_and_normalized_ast() {
    let cases = [
        (
            "// módulo\nfn Main(){let x=1; // cola\n}",
            "// módulo\nfn Main() {\n    let x = 1; // cola\n}\n",
        ),
        (
            "fn Main(){let x=if true then { // rama\nvalue:1; // entrega\n}else{value:2;};}",
            "fn Main() {\n    let x = if true then { // rama\n        value: 1; // entrega\n    } else 2;\n}\n",
        ),
        (
            "/// documentación\nfn Main(){\n// independiente\nlet x=(1+2);\n}",
            "/// documentación\nfn Main() {\n    // independiente\n    let x = (1 + 2);\n}\n",
        ),
        ("fn Main(){ // cuerpo\n}", "fn Main() { // cuerpo\n}\n"),
        ("fn Main(){\n// vacío\n}", "fn Main() {\n    // vacío\n}\n"),
        (
            "fn Main(){let x=1 // sin punto y coma\nlet y=2\n}\n// fin",
            "fn Main() {\n    let x = 1; // sin punto y coma\n    let y = 2;\n}\n// fin\n",
        ),
    ];
    for (source, expected) in cases {
        let cst = super::cst::parse_cst("comments.n", source).unwrap();
        assert_eq!(cst.comments.len(), source.matches("//").count());
        let formatted = super::format::format_cst("comments.n", &cst).unwrap();
        assert_eq!(formatted, expected, "{source}");
        let again = super::format::format("comments.n", &formatted).unwrap();
        assert_eq!(again, formatted, "{source}");
        let reparsed = parse("comments.n", &formatted).unwrap();
        assert_eq!(
            super::test_normalize::ast_without_spans(cst.ast),
            super::test_normalize::ast_without_spans(reparsed),
            "{source}"
        );
    }
}

#[test]
fn formatter_abbreviates_only_branches_without_attached_comments() {
    let cases = [
        (
            "// header\nfn Main(){let x=if true then {value:1;}else{value:2;};}",
            "// header\nfn Main() {\n    let x = if true then 1 else 2;\n}\n",
        ),
        (
            "fn Main(){let x=if true then {value:1;}else{value:2;}; // tail\nlet y=3;}",
            "fn Main() {\n    let x = if true then 1 else 2; // tail\n    let y = 3;\n}\n",
        ),
    ];
    for (source, expected) in cases {
        let formatted = super::format::format("branch.n", source).unwrap();
        assert_eq!(formatted, expected);
        assert_eq!(
            super::format::format("branch.n", &formatted).unwrap(),
            expected
        );
    }
}

#[test]
fn formatter_moves_intertoken_comment_without_trailing_spaces() {
    let source = "fn // name\nMain(){}";
    let expected = "fn // name\nMain() {}\n";
    let formatted = super::format::format("comments.n", source).unwrap();
    assert_eq!(formatted, expected);
    assert_eq!(
        super::format::format("comments.n", &formatted).unwrap(),
        expected
    );
}

#[test]
fn formatter_preserves_ast_under_inserted_line_comments() {
    let seeds = [
        "fn Main(){let x=1;let y=x+2;}",
        "fn Main(){let x=if true then {value:1;}else{value:2;};let y=x;}",
        "fn F(x:view I64)->I64{return x;}fn Main(){F(1);}",
        "fn Main(){var a=[1,2];a[0]=3;}",
    ];
    let mut generated = 0;
    let mut accepted = 0;
    let mut rejected = Vec::new();
    for (seed_id, seed) in seeds.into_iter().enumerate() {
        parse("comments.n", seed).expect("comment insertion seed must parse");
        let tokens = super::lex::lex("comments.n", seed).unwrap();
        let tokens: Vec<_> = tokens
            .iter()
            .filter(|t| t.kind != super::lex::Kind::Eof)
            .collect();
        let mut sources = Vec::new();
        for token in &tokens {
            let mut source = seed.to_owned();
            source.insert_str(token.span.end as usize, "// marker\n");
            sources.push(source);
        }
        for left in (0..tokens.len()).step_by(3) {
            for right in ((left + 1)..tokens.len()).step_by(3) {
                let mut source = seed.to_owned();
                source.insert_str(tokens[right].span.end as usize, "// second\n");
                source.insert_str(tokens[left].span.end as usize, "// first\n");
                sources.push(source);
            }
        }
        generated += sources.len();
        for (source_id, source) in sources.into_iter().enumerate() {
            let ast = match parse("comments.n", &source) {
                Ok(ast) => ast,
                Err(error) => {
                    rejected.push(format!(
                        "{seed_id},{source_id}:{}:{}",
                        error.code,
                        error.parser_context.as_deref().unwrap_or("")
                    ));
                    continue;
                }
            };
            accepted += 1;
            let formatted = super::format::format("comments.n", &source).unwrap();
            let reparsed = parse("comments.n", &formatted).unwrap();
            assert_eq!(
                super::test_normalize::ast_without_spans(ast),
                super::test_normalize::ast_without_spans(reparsed),
                "{source:?} => {formatted:?}"
            );
            assert_eq!(
                super::format::format("comments.n", &formatted).unwrap(),
                formatted,
                "{source:?}"
            );
            assert!(
                !formatted.contains("Main(){"),
                "{source:?} => {formatted:?}"
            );
            assert!(
                formatted.lines().all(|line| !line.ends_with(' ')),
                "{source:?} => {formatted:?}"
            );
        }
    }
    assert_eq!(generated, 246);
    assert_eq!(accepted, 226);
    assert_eq!(accepted + rejected.len(), generated);
    assert_closed_rejections(&rejected, include_str!("simple-comment-rejections.txt"));
}

#[test]
fn adr_scope_rows_use_literal_b_sources_and_offsets() {
    // F0-D15 §8.7, F2-F02…F33. These offsets are transcribed from its table.
    let rows: &[(&str, &str, u64, u64)] = &[
        (
            "fn F(){return 1;} fn Main(){}",
            "inferred_non_unit_result",
            3,
            4,
        ),
        ("fn Main(){let x=0.5;}", "float_literal", 16, 19),
        ("fn Main(){let n=1;let x=0..<n;}", "range", 25, 28),
        ("fn Main(){let x=(1,2);}", "tuple", 18, 19),
        ("@derive(JsonEncode) fn Main(){}", "attributes", 0, 1),
        (
            "fn F(x:view I64=1){} fn Main(){F(1);}",
            "default_argument",
            15,
            16,
        ),
        ("fn Main(){let f=fn[](){return ();};}", "closure", 16, 18),
        ("fn F(){} fn Main(){let g=F;}", "function_value", 25, 26),
        ("fn F(){} fn Main(){move F();}", "receiver_mode", 19, 23),
        (
            "fn Main(){let a=[1];let x=move a[0];}",
            "partial_transfer",
            31,
            35,
        ),
        ("fn Main(){let a=[MockOpen(1)];}", "resource_array", 16, 29),
        ("fn Main() with no_heap_alloc{}", "no_heap_alloc", 15, 28),
        ("fn Main(){match true{true=>{},false=>{}}}", "match", 10, 15),
        ("fn F(x:view Option<I64>){} fn Main(){}", "option", 12, 18),
        ("struct S{} fn Main(){}", "user_types", 0, 6),
        ("trait T{} fn Main(){}", "traits_generics", 0, 5),
        ("use std.app;fn Main(){}", "modules_visibility", 0, 3),
        ("extend I64{} fn Main(){}", "extensions", 0, 6),
        (
            "fn F(x:view I64)->I64 from(x){return x;} fn Main(){}",
            "provenance_clauses",
            22,
            26,
        ),
        ("const x:I64=comptime 1;fn Main(){}", "comptime", 0, 5),
        ("fn Main(){let x=\"hola\";}", "text", 16, 22),
        ("fn Main(){let a=[1];a.Foo();}", "methods", 21, 22),
        ("fn Main(){var x=1;x+=1;}", "compound_assignment", 19, 21),
        ("fn Main(){for n in [1]{}}", "for_iteration", 10, 13),
        ("fn Main(){while false{break;}}", "loop_control", 22, 27),
        ("gen fn G(){} fn Main(){}", "generator", 0, 3),
        ("async fn F(){} fn Main(){}", "async", 0, 5),
        ("fn F(x:view U32){} fn Main(){}", "unsupported_type", 12, 15),
        (
            "fn F(x:view String){} fn Main(){}",
            "unsupported_type",
            12,
            18,
        ),
        (
            "fn F(x:view app.Error){} fn Main(){}",
            "unsupported_type",
            12,
            21,
        ),
        ("fn Main(){region r(capacity:0){}}", "region", 10, 16),
        ("fn Main(){let x=self;}", "methods", 16, 20),
    ];
    assert_eq!(rows.len(), 32, "F0-D15 §8.7 F2-F02…F2-F33");
    for &(source, feature, start, end) in rows {
        let error = parse("b.n", source).unwrap_err();
        assert_eq!(error.code, "NX-E0006", "{source}");
        assert_eq!(error.schema_version, 1, "{source}");
        assert_eq!(
            error.diagnostic_namespace,
            crate::diagnostic::Namespace::NexoProposedV1,
            "{source}"
        );
        assert_eq!(error.profile_id, "f0-0002-b-v2", "{source}");
        assert_eq!(error.subcause, None, "{source}");
        assert_eq!(error.rule, "NX-E0006", "{source}");
        assert_eq!(
            error.explanation_id.as_deref(),
            Some("NX-E0006"),
            "{source}"
        );
        assert_eq!(
            error.severity,
            crate::diagnostic::Severity::Error,
            "{source}"
        );
        assert_eq!(error.feature_id.as_deref(), Some(feature), "{source}");
        let (stage, operation) = match feature {
            "function_value" => (
                crate::diagnostic::Stage::Names,
                crate::diagnostic::Operation::NamesResolve,
            ),
            "inferred_non_unit_result" | "resource_array" | "option" | "unsupported_type" => (
                crate::diagnostic::Stage::Types,
                crate::diagnostic::Operation::TypesCheck,
            ),
            _ => (
                crate::diagnostic::Stage::Parse,
                crate::diagnostic::Operation::SyntaxParse,
            ),
        };
        assert_eq!(
            (error.stage, error.operation),
            (stage, operation),
            "{source}"
        );
        assert_eq!(error.cause_chain, [], "{source}");
        assert_eq!(error.unexpected_token, None, "{source}");
        assert_eq!(error.expected_tokens, Vec::<String>::new(), "{source}");
        assert_eq!(error.expected_tokens_kind, None, "{source}");
        assert_eq!(error.parser_context, None, "{source}");
        assert_eq!(error.recognized_prefix, None, "{source}");
        assert_eq!(error.repair, [], "{source}");
        let pending = if matches!(feature, "closure" | "generator" | "option" | "match") {
            "F0-0003"
        } else {
            "later_adr"
        };
        let mut data = std::collections::BTreeMap::from([(
            "pending_scope".to_owned(),
            crate::diagnostic::DataValue::String(pending.to_owned()),
        )]);
        if feature == "unsupported_type" {
            let spelling = match &source[start as usize..end as usize] {
                "U32" => "U32",
                "String" => "String",
                "app.Error" => "app.Error",
                other => panic!("untranscribed type spelling {other}"),
            };
            data.insert(
                "type_spelling".into(),
                crate::diagnostic::DataValue::String(spelling.into()),
            );
        }
        assert_eq!(error.data, data, "{source}");
        let span = error.primary_span.as_ref().unwrap();
        assert_eq!((span.start, span.end), (start, end), "{source}");
        if feature == "partial_transfer" || feature == "inferred_non_unit_result" {
            assert_eq!(error.related_spans.len(), 1, "{source}");
            assert_eq!(
                error.related_spans[0].role,
                crate::diagnostic::RelatedRole::Context,
                "{source}"
            );
            let related = if feature == "partial_transfer" {
                (26, 30)
            } else {
                (7, 15)
            };
            assert_eq!(
                (
                    error.related_spans[0].span.start,
                    error.related_spans[0].span.end
                ),
                related,
                "{source}"
            );
        } else {
            assert_eq!(error.related_spans, [], "{source}");
        }
        assert!(
            error.validate("b.n", source).is_ok(),
            "{source}: {:?}",
            error.validate("b.n", source)
        );
    }
}

#[test]
fn b_terminator_productions_and_continuations_keep_their_line_boundaries() {
    for newline in ["\n", "\r\n", "\r"] {
        for (left, repaired) in [
            ("let x=1", "let x=1;"),
            ("x=1", "x=1;"),
            ("return ()", "return ();"),
            ("F()", "F();"),
            ("{let x=1;}", "{let x=1;}"),
            ("while true{}", "while true{}"),
            ("if true{}else{}", "if true{}else{}"),
        ] {
            let prefix = "fn F(){} fn Main(){";
            let implicit = format!("{prefix}{left}{newline}let z=2;}}");
            let explicit = format!("{prefix}{repaired}let z=2;}}");
            let a = parse("matrix.n", &implicit).unwrap();
            let b = parse("matrix.n", &explicit).unwrap();
            assert_eq!(
                ast_without_spans(a.clone()),
                ast_without_spans(b),
                "{left:?} {newline:?}"
            );
            let printed = print_canonical(&a).unwrap();
            let reparsed = parse("matrix.n", &printed).unwrap();
            assert_eq!(printed, print_canonical(&reparsed).unwrap());
        }
        for continuation in ["(", "[", "?", "+", "-", "<", "==", ","] {
            let source = match continuation {
                "(" => {
                    format!("fn F(x:view I64)->I64{{return x;}} fn Main(){{let x=F{newline}(1);}}")
                }
                "[" => format!("fn Main(){{let a=[1];let x=a{newline}[0];}}"),
                "?" => format!(
                    "fn F()->Result<I64,I64>{{return Ok(1);}} fn Main(){{let x=F(){newline}?;}}"
                ),
                "+" | "-" | "<" | "==" => format!("fn Main(){{let x=1{newline}{continuation}2;}}"),
                _ => format!("fn F(x:view I64,y:view I64){{}} fn Main(){{F(1{newline},2);}}"),
            };
            let continued = parse("continuation.n", &source).unwrap();
            let same_line = source.replacen(newline, "", 1);
            let paired = parse("continuation.n", &same_line).unwrap();
            assert_eq!(
                ast_without_spans(continued.clone()),
                ast_without_spans(paired)
            );
            let split = source.replacen(newline, &format!(";{newline}"), 1);
            match parse("continuation.n", &split) {
                Ok(ast) => assert_ne!(ast_without_spans(continued), ast_without_spans(ast)),
                Err(error) => error.validate("continuation.n", &split).unwrap(),
            }
        }
        for source in [
            format!("fn Main(){{var x=0;x=1{newline}+2;}}"),
            format!("fn F()->I64{{return 1{newline}+2;}} fn Main(){{}}"),
            format!(
                "fn F()->I64{{return if true then {{value:1{newline}+2;}}else 2;}} fn Main(){{}}"
            ),
        ] {
            let continued = parse("continuation.n", &source).unwrap();
            let same_line = source.replacen(newline, "", 1);
            assert_eq!(
                ast_without_spans(continued.clone()),
                ast_without_spans(parse("continuation.n", &same_line).unwrap())
            );
            let split = source.replacen(newline, &format!(";{newline}"), 1);
            match parse("continuation.n", &split) {
                Ok(ast) => assert_ne!(ast_without_spans(continued), ast_without_spans(ast)),
                Err(error) => error.validate("continuation.n", &split).unwrap(),
            }
        }
        let empty_return = format!("fn Main(){{return{newline}(1);}}");
        let ast = parse("lines.n", &empty_return).unwrap();
        assert!(matches!(
            ast.functions[0].body.statements[0],
            Statement::Return { value: None, .. }
        ));

        let empty_value =
            format!("fn Main(){{let x=if true then {{value:{newline}(1);}}else{{value:2;}};}}");
        let diagnostic = parse("lines.n", &empty_value).unwrap_err();
        assert_eq!(diagnostic.rule, "NX-E1302/missing_value_operand");
        diagnostic.validate("lines.n", &empty_value).unwrap();
    }

    let trailing = "fn Main(){} unexpected";
    let diagnostic = parse("eof.n", trailing).unwrap_err();
    assert_eq!(diagnostic.code, "NX-E0002");
    assert_eq!(diagnostic.parser_context.as_deref(), Some("module_item"));
    assert_eq!(
        diagnostic
            .unexpected_token
            .as_ref()
            .unwrap()
            .text
            .as_deref(),
        Some("unexpected")
    );
    diagnostic.validate("eof.n", trailing).unwrap();

    let branch = "fn Main(){let x=if true then 1\r\nelse 2;}";
    assert_eq!(
        parse("continuation.n", branch).unwrap().functions[0]
            .body
            .statements
            .len(),
        1
    );
    let functions = "fn A(){}\r\nfn B(){}";
    assert_eq!(
        parse("continuation.n", functions).unwrap().functions.len(),
        2
    );

    for source in [
        "fn Main(){let x=1  }",
        "fn Main(){let x=1\t}",
        "fn Main(){x=F(1)  }",
        "fn Main(){x=F(1)\t}",
        "fn F()->I64{return 1  }",
        "fn F()->I64{return 1\t}",
    ] {
        let brace = source.rfind('}').unwrap();
        let token_end = source[..brace].trim_end().len();
        let diagnostic = parse("trivia.n", source).unwrap_err();
        assert_eq!(diagnostic.code, "NX-E0005", "{source}");
        assert_eq!(
            diagnostic.primary_span.as_ref().unwrap().start as usize,
            brace
        );
        let edit = &diagnostic.repair[0].edits[0];
        assert_eq!(
            (edit.span.start as usize, edit.span.end as usize),
            (token_end, token_end)
        );
        assert_eq!(edit.replacement, ";");
        diagnostic.validate("trivia.n", source).unwrap();
    }
    for source in [
        "fn Main(){let x=1 // no\n}",
        "fn Main(){x=F(1) // no\r\n}",
        "fn F()->I64{return 1 // no\r\n}",
    ] {
        assert!(parse("trivia.n", source).is_ok(), "{source:?}");
    }
    let no_insert_before_brace = "fn Main(){if true{1}else{}}";
    let error = parse("same-line.n", no_insert_before_brace).unwrap_err();
    assert_eq!(error.code, "NX-E0005");
    let end = no_insert_before_brace.find('1').unwrap() + 1;
    assert_eq!(error.primary_span.as_ref().unwrap().start as usize, end);
    error
        .validate("same-line.n", no_insert_before_brace)
        .unwrap();
}

#[test]
fn commented_crlf_terminators_match_explicit_b_programs() {
    fn token_end(statement: &Statement) -> Option<u64> {
        match statement {
            Statement::Let { span, .. }
            | Statement::Assign { span, .. }
            | Statement::Return { span, .. }
            | Statement::Value { span, .. }
            | Statement::Expr { span, .. } => Some(span.end),
            _ => None,
        }
    }
    for source in [
        "fn Main(){var x=0;x=1//é\r\nlet z=2;}",
        "fn F()->I64{return 1//é\r\nlet z=2;} fn Main(){}",
        "fn F()->I64{return if true then {value:1//é\r\n}else{value:2;};} fn Main(){}",
        "fn Main(){F()//é\r\nlet z=2;}",
    ] {
        let implicit = parse("p2.n", source).unwrap();
        let end = source.find("//é").unwrap() as u64;
        let ended = implicit
            .functions
            .iter()
            .flat_map(|f| &f.body.statements)
            .any(|s| token_end(s) == Some(end))
            || matches!(&implicit.functions[0].body.statements[0], Statement::Return { value: Some(super::parse::Expr::If { then_block, .. }), .. } if token_end(&then_block.statements[0]) == Some(end));
        assert!(ended, "{source:?}");
        let explicit_source = source.replace("//é", ";//é");
        let explicit = parse("p2.n", &explicit_source).unwrap();
        assert_eq!(ast_without_spans(implicit), ast_without_spans(explicit));
    }
    let negative = "fn F()->I64{return if true then {value:1 }else{value:2;};} fn Main(){}";
    let error = parse("p2.n", negative).unwrap_err();
    assert_eq!(error.code, "NX-E0005");
    assert_eq!(
        error.parser_context.as_deref(),
        Some("statement_terminator")
    );
    assert_eq!(error.expected_tokens, [";"]);
    assert_eq!(
        error.unexpected_token.as_ref().unwrap().text.as_deref(),
        Some("}")
    );
    assert_eq!(error.repair[0].edits[0].replacement, ";");
    error.validate("p2.n", negative).unwrap();
    assert!(
        parse(
            "p2.n",
            "fn F()->I64{return if true then {value:1; }else{value:2;};} fn Main(){}"
        )
        .is_ok()
    );
}

#[test]
fn adr_line_endings_contextual_names_and_branch_values() {
    let l02 = "fn F()->I64{\nreturn if true then {\nvalue:\n(1);\n}else{value:2;};\n}\nfn Main(){}";
    let error = parse("b.n", l02).unwrap_err();
    assert_eq!(error.rule, "NX-E1302/missing_value_operand");
    assert_eq!(
        (
            error.primary_span.as_ref().unwrap().start,
            error.primary_span.as_ref().unwrap().end
        ),
        (35, 41)
    );
    assert!(error.validate("b.n", l02).is_ok());

    let l03 =
        "fn value(x:view I64)->I64{return x;}\nfn Main(){let x=7;let y=value\n(x);AssertEq(y,7);}";
    let ast = parse("b.n", l03).unwrap();
    assert_eq!(ast.functions[0].name, "value");
    assert!(print_canonical(&ast).unwrap().contains("value(x)"));
    let l04 = "fn Main(){\nlet value=2\nlet a=value -1\nlet b=value - 1\nAssertEq(a,b)\n}";
    assert!(parse("b.n", l04).is_ok());
    let l05 = "fn Main(){\nlet x=if true then {value: ();}else{value: ();};\nAssertEq(x,());\n}";
    let ast = parse("b.n", l05).unwrap();
    let printed = print_canonical(&ast).unwrap();
    assert!(printed.contains("if true then () else ()"));
    assert_eq!(
        print_canonical(&parse("b.n", &printed).unwrap()).unwrap(),
        printed
    );
}

#[test]
fn adr_missing_terminators_use_published_byte_offsets() {
    for (source, start, end, prefix, preceding, insertion) in [
        (
            "fn F()->I64{return 1 } fn Main(){}",
            21,
            22,
            (12, 21),
            (19, 20),
            (20, 20),
        ),
        (
            "fn F()->I64{return if true then {value:1 }else{value:2;};} fn Main(){}",
            41,
            42,
            (33, 41),
            (39, 40),
            (40, 40),
        ),
    ] {
        let error = parse("b.n", source).unwrap_err();
        assert_eq!(error.code, "NX-E0005", "{source}");
        let primary = error.primary_span.as_ref().unwrap();
        assert_eq!((primary.start, primary.end), (start, end));
        let recognized = error.recognized_prefix.as_ref().unwrap();
        assert_eq!((recognized.start, recognized.end), prefix);
        let related = &error.related_spans[0].span;
        assert_eq!((related.start, related.end), preceding);
        let edit = &error.repair[0].edits[0].span;
        assert_eq!((edit.start, edit.end), insertion);
        assert!(
            error.validate("b.n", source).is_ok(),
            "{source}: {:?}",
            error.validate("b.n", source)
        );
    }
}

#[test]
fn adr_terminator_diagnostic_matches_complete_published_json() {
    // F0-D15 §6.4, transcribed independently of the parser and serializer.
    let file = "terminador.n";
    let source = "fn Main(){let x=1}";
    let error = parse(file, source).unwrap_err();
    assert_eq!(
        error.to_json(file, source).unwrap(),
        concat!(
            "{\"schema_version\":1,\"diagnostic_namespace\":\"Nexo-proposed-v1\",",
            "\"profile_id\":\"f0-0002-b-v2\",\"code\":\"NX-E0005\",\"subcause\":null,",
            "\"rule\":\"NX-E0005\",\"explanation_id\":\"NX-E0005\",\"severity\":\"error\",",
            "\"stage\":\"parse\",\"message\":\"Falta un terminador de sentencia.\",",
            "\"operation\":\"syntax.parse\",",
            "\"primary_span\":{\"file\":\"terminador.n\",\"start\":17,\"end\":18},",
            "\"related_spans\":[{\"role\":\"preceding_expression\",\"span\":{\"file\":\"terminador.n\",\"start\":16,\"end\":17}}],",
            "\"cause_chain\":[],\"feature_id\":null,",
            "\"unexpected_token\":{\"kind\":\"symbol\",\"text\":\"}\",\"span\":{\"file\":\"terminador.n\",\"start\":17,\"end\":18}},",
            "\"expected_tokens\":[\";\"],\"expected_tokens_kind\":\"canonical_insertion\",",
            "\"parser_context\":\"statement_terminator\",",
            "\"recognized_prefix\":{\"file\":\"terminador.n\",\"start\":10,\"end\":17},",
            "\"repair\":[{\"id\":\"insert_terminator\",\"status\":\"proposed\",",
            "\"description\":\"Insertar un terminador explícito.\",",
            "\"edits\":[{\"span\":{\"file\":\"terminador.n\",\"start\":17,\"end\":17},",
            "\"expected_text\":\"\",\"replacement\":\";\"}],\"validation\":null,",
            "\"consequences\":[\"Termina el binding antes del cierre del bloque.\"]}],\"data\":{}}",
        )
    );
}

#[test]
fn contextual_modes_labels_and_else_if_keep_distinct_ast_shapes() {
    use super::parse::{Expr, Mode, Statement};
    let source = "fn F(x:view I64){} fn Main(){let move=1;let edit=2;F(move);F(move: edit);F(move x);move(x);(move)(x);if true{}else if false{}else{}}";
    let ast = parse("b.n", source).unwrap();
    let statements = &ast.functions[1].body.statements;
    assert!(
        matches!(&statements[2], Statement::Expr { value: Expr::Call { args, .. }, .. } if matches!(&args[0].value, Expr::Name(name, _) if name == "move"))
    );
    assert!(
        matches!(&statements[3], Statement::Expr { value: Expr::Call { args, .. }, .. } if args[0].label.as_deref() == Some("move"))
    );
    assert!(
        matches!(&statements[4], Statement::Expr { value: Expr::Call { args, .. }, .. } if matches!(&args[0].value, Expr::Unary { mode: Mode::Consume, .. }))
    );
    assert!(matches!(
        &statements[5],
        Statement::Expr {
            value: Expr::Unary {
                mode: Mode::Consume,
                ..
            },
            ..
        }
    ));
    assert!(
        matches!(&statements[6], Statement::Expr { value: Expr::Call { target, .. }, .. } if matches!(&**target, Expr::Group(_, _)))
    );
    assert!(matches!(&statements[7], Statement::If { .. }));
    let printed = print_canonical(&ast).unwrap();
    assert!(printed.contains("} else if false {} else {}"));
    assert_eq!(
        print_canonical(&parse("b.n", &printed).unwrap()).unwrap(),
        printed
    );
}

#[test]
fn line_endings_and_utf8_comments_preserve_byte_coordinates() {
    for (newline, expected) in [("\n", 27), ("\r\n", 29), ("\r", 27)] {
        let source = format!("// á🙂{newline}fn Main(){{{newline}let x=missing;{newline}}}");
        let tokens = super::lex::lex("b.n", &source).unwrap();
        let missing = tokens.iter().find(|t| t.text == "missing").unwrap();
        assert_eq!(
            (missing.span.start, missing.span.end),
            (expected, expected + 7)
        );
        assert!(parse("b.n", &source).is_ok());
    }
}

#[test]
fn accepted_b_ast_round_trips_without_source_spans() {
    let sources = [
        "fn Main(){}",
        "fn F(x:view I64,y:edit Array<I64>)->I64{return x;} fn Main(){}",
        "fn Main(){let x=-01;var y=[1,2];y[0]=x+2;}",
        "fn Main(){let r=MockOpen(1);let s=move r;Inspect(s);}",
        "fn Main(){let x=if true then 1 else 2;}",
        "fn Main(){let x=if true then {value:1;} else {value:2;};}",
        "fn Main(){let x=if true then 1 else (if false then 2 else 3);}",
        "fn Main(){let x=if true then 1 else {value: (if false then 2 else 3);};}",
        "fn Main(){if true{}else if false{}else{}}",
        "fn Main(){let a=[1];Print(a[0]);}",
        "fn Main() with {call(no_io,no_alloc)}{}",
    ];
    for source in sources {
        let ast = parse("b.n", source).unwrap();
        let printed = print_canonical(&ast).unwrap();
        let reparsed = parse("b.n", &printed).unwrap();
        assert_eq!(
            super::test_normalize::ast_without_spans(ast.clone()),
            super::test_normalize::ast_without_spans(reparsed),
            "{source} => {printed}"
        );
        assert_eq!(
            print_canonical(&parse("b.n", &printed).unwrap()).unwrap(),
            printed
        );
        assert_eq!(super::format::format("b.n", source).unwrap(), printed);
        assert_eq!(super::format::format("b.n", &printed).unwrap(), printed);
    }
    let modes = "fn F(x:edit I64,y:move I64){}";
    let ast = parse("b.n", modes).unwrap();
    assert_eq!(ast.functions[0].params[0].mode, super::parse::Mode::Inout);
    assert_eq!(ast.functions[0].params[1].mode, super::parse::Mode::Consume);
    let printed = print_canonical(&ast).unwrap();
    assert_eq!(
        ast_without_spans(ast),
        ast_without_spans(parse("b.n", &printed).unwrap())
    );
}

#[test]
fn canonical_printer_preserves_full_b_ast_across_body_and_signature_matrix() {
    let bodies = [
        "let x=1;var y:Bool=true;y=false;return;",
        "let x=[1,2];x[0]=3;Drop(x[1]);",
        "let x=(1+2)-3;let y=x<4;let z=x==4;",
        "if true{let x=1;}else if false{let x=2;}else{}while false{}{}",
        "if true{}else{x;}",
        "if true{}else if false{}",
        "if true{}else if false{}else{x;}",
        "let x=if true then {value:1;}else{value:2;};",
        "let x=F(a:1,2)[0];let y=move x;F(edit y);",
        "let x=Ok(1)?;let y=();let z=[];",
        "return (1);",
    ];
    let headers = [
        "fn Main()",
        "fn Main(x:view I64,y:move Array<Result<I64,Bool>>,z:edit MockResource)->MockBlockingResource",
        "fn Main() with no_io",
        "fn Main() with {call(no_alloc,no_io)}",
    ];
    for seed in 0..5_000usize {
        let body =
            bodies[seed % bodies.len()].replace("let x=1;", &format!("let x={};", seed % 100));
        let header = headers[(seed / bodies.len()) % headers.len()];
        let header = if body == "return (1);" && !header.contains("->") {
            if header.contains(" with ") {
                header.replacen(" with ", "->I64 with ", 1)
            } else {
                format!("{header}->I64")
            }
        } else {
            header.to_owned()
        };
        let source = format!("{header}{{{body}}}");
        let original =
            parse("roundtrip.n", &source).unwrap_or_else(|e| panic!("source {source}: {e:?}"));
        let printed = print_canonical(&original).unwrap();
        let reparsed =
            parse("roundtrip.n", &printed).unwrap_or_else(|e| panic!("printed {printed}: {e:?}"));
        assert_eq!(
            ast_without_spans(original),
            ast_without_spans(reparsed.clone()),
            "source {source}"
        );
        assert_eq!(
            printed,
            print_canonical(&reparsed).unwrap(),
            "source {source}"
        );
    }
}

#[test]
fn generated_valid_programs_round_trip() {
    // Seed 0x4e45_584f_5eed_2026, fixed before inspecting results.
    let mut state = 0x4e45_584f_5eed_2026_u64;
    for _ in 0..100_000_u32 {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        let a = state % 1000;
        let b = (state >> 13) % 1000;
        let source = match state % 13 {
            0 => format!("fn Main(){{let x={a};let y=x+{b};AssertEq(y,{});}}", a + b),
            1 => format!("fn F(x:view I64)->I64{{return x+{a};}}fn Main(){{F({b});}}"),
            2 => format!("fn Main(){{let x=if true then {{value:{a};}}else{{value:{b};}};}}"),
            3 => format!("fn Main(){{var a=[{a},{b}];a[0]={a};}}"),
            4 => format!("fn Main() with no_io,no_alloc{{let x=({a}+{b});}}"),
            5 => {
                format!("fn Main(){{if true{{}}else if false{{let x={a}\r\nlet y={b};}}else{{}}}}")
            }
            6 => format!("fn Main(){{let x=if true then {{value:{a}\n}} else {b};}}"),
            7 => format!("fn Main(){{var a={a};while a<{b} {{a=a+1\n}}}}"),
            8 => "fn Main() -> () {return;}".to_owned(),
            9 => format!("fn Main(x:view I64,y:edit I64,z:move I64){{let a=[{a},{b}];a[0]=1;}}"),
            10 => format!("fn Main(){{let x=a?+{a};}}"),
            11 => format!("fn Main(){{view({a});view[0];}}"),
            _ => format!("fn Main(){{let x=if true then {a} else {{value:{b};}}\nlet y=3;}}"),
        };
        let ast = parse("b.n", &source).unwrap();
        let printed = print_canonical(&ast).unwrap();
        let reparsed = parse("b.n", &printed).unwrap();
        assert_eq!(
            super::test_normalize::ast_without_spans(ast.clone()),
            super::test_normalize::ast_without_spans(reparsed)
        );
        assert_eq!(
            print_canonical(&parse("b.n", &printed).unwrap()).unwrap(),
            printed
        );
        let formatted = super::format::format("b.n", &source).unwrap();
        assert_eq!(formatted, printed);
        assert_eq!(super::format::format("b.n", &formatted).unwrap(), formatted);
        assert_eq!(
            super::test_normalize::ast_without_spans(parse("b.n", &formatted).unwrap()),
            super::test_normalize::ast_without_spans(ast),
        );
    }
}

#[test]
fn adr_accepted_corpus_formats_idempotently_with_the_same_normalized_ast() {
    let adr = include_str!("../../docs/adr/F0-D15-sintaxis-oficial.md");
    let mut sources = Vec::new();
    let table = adr
        .split_once("### 8.7.")
        .unwrap()
        .1
        .split_once("### 8.8.")
        .unwrap()
        .0;
    for line in table.lines().filter(|line| line.starts_with("| F2-")) {
        let literal = line.split('`').nth(1).expect("§8.7 literal");
        sources.push(literal.replace(['⟦', '⟧'], ""));
    }
    for section in [2, 3, 4, 5, 6, 8, 9, 10] {
        let body = adr
            .split_once(&format!("### 8.{section}."))
            .unwrap()
            .1
            .split_once(&format!("### 8.{}.", section + 1))
            .unwrap()
            .0;
        for block in body.split("```nexo\n").skip(1) {
            sources.push(block.split_once("\n```").unwrap().0.replace(['⟦', '⟧'], ""));
        }
    }
    let table = adr
        .split_once("### 8.12.")
        .unwrap()
        .1
        .split_once("### 8.13.")
        .unwrap()
        .0;
    for line in table
        .lines()
        .filter(|line| line.starts_with("| T0") || line.starts_with("| D0"))
    {
        let literal_cell = line.split(" | ").nth(1).expect("§8.12 source cell");
        for source in literal_cell.split('`').skip(1).step_by(2) {
            sources.push(source.to_owned());
        }
    }
    assert_eq!(
        sources.len(),
        160,
        "111 table rows + 27 blocks + 22 §8.12 programs"
    );
    let mut accepted = 0;
    let mut rejected = Vec::new();
    for (source_id, source) in sources.into_iter().enumerate() {
        let ast = match parse("adr-fmt.n", &source) {
            Ok(ast) => ast,
            Err(error) => {
                rejected.push(format!(
                    "{source_id}:{}:{}",
                    error.code,
                    error.parser_context.as_deref().unwrap_or("")
                ));
                continue;
            }
        };
        let formatted = super::format::format("adr-fmt.n", &source).unwrap();
        assert!(formatted.len() <= 1_048_576, "{source}");
        assert_eq!(
            super::format::format("adr-fmt.n", &formatted).unwrap(),
            formatted,
            "{source}"
        );
        assert_eq!(
            super::test_normalize::ast_without_spans(parse("adr-fmt.n", &formatted).unwrap()),
            super::test_normalize::ast_without_spans(ast),
            "{source}"
        );
        accepted += 1;
    }
    assert_eq!(
        accepted, 98,
        "accepted corpus programs within the byte budget"
    );
    assert_eq!(accepted + rejected.len(), 160);
    assert_closed_rejections(&rejected, include_str!("adr-format-rejections.txt"));
}

#[test]
fn adr_large_compact_source_obeys_canonical_reparse_budget() {
    use crate::diagnostic::DataValue;
    // F0-D15 §8.13 B-D1: the compact spelling is the published test input.
    let mut source = String::from(
        "fn Big(b:view Bool){var s=MockOpen(1);Drop(s);while b{Inspect(s);s=MockOpen(2);}",
    );
    for _ in 0..96 {
        source.push_str("while b{");
    }
    for index in 0..6000 {
        source.push_str(&format!("let x{index}=0;"));
    }
    source.push_str(&"}".repeat(97));
    source.push_str("fn Main(){}");
    let token_texts = |text: &str| {
        super::lex::lex("b-d1.n", text)
            .unwrap()
            .into_iter()
            .filter(|token| token.kind != super::lex::Kind::Newline)
            .map(|token| token.text)
            .collect::<Vec<_>>()
    };
    // §8.13 rows 1/2 share B-D1; row 3 adds Later. Their ownership and
    // Analysis-limit results are checked in semantic stages; these are AST roots.
    for (compact, compact_size, canonical_size, names) in [
        (source.clone(), 71_846, 2_455_432, &["Big", "Main"][..]),
        (
            source.replace(
                "fn Main(){}",
                "fn Later(){let z=0;Drop(z);Print(z);}fn Main(){}",
            ),
            71_883,
            2_455_490,
            &["Big", "Later", "Main"],
        ),
    ] {
        assert_eq!(compact.len(), compact_size);
        let ast = parse("b-d1.n", &compact).unwrap();
        assert_eq!(
            ast.functions
                .iter()
                .map(|f| f.name.as_str())
                .collect::<Vec<_>>(),
            names
        );
        let printed = print_canonical(&ast).unwrap();
        assert_eq!(printed.len(), canonical_size);
        assert_eq!(token_texts(&compact), token_texts(&printed));
        let error = parse("b-d1.n", &printed).unwrap_err();
        assert_eq!(error.code, "NX-E0003");
        assert_eq!(
            error.data.get("budget_kind"),
            Some(&DataValue::String("source_bytes".into()))
        );
        assert_eq!(
            error.data.get("budget_scope"),
            Some(&DataValue::String("compilation".into()))
        );
        assert_eq!(
            error.data.get("incomplete_analysis"),
            Some(&DataValue::String("input".into()))
        );
        assert_eq!(error.data.get("limit"), Some(&DataValue::UInt(1_048_576)));
        assert_eq!(
            error.data.get("observed"),
            Some(&DataValue::UInt(1_048_577))
        );
        assert_eq!(error.data.len(), 5);
        assert!(error.validate("b-d1.n", &printed).is_ok());
    }
}

#[test]
fn mutated_bytes_and_tokens_never_panic_or_emit_invalid_diagnostics() {
    let seeds = [
        b"fn Main(){let x=1;}".as_slice(),
        b"fn F(x:view I64)->I64{return x;} fn Main(){}".as_slice(),
        b"fn Main(){let x=if true then 1 else 2;}".as_slice(),
        b"fn Main(){var a=[1,2];a[0]=3;}".as_slice(),
    ];
    let replacements = b"(){}[],:;?+-<>=@.*/%0123456789abcdefghijklmnopqrstuvwxyz\r\n\t\xff";
    let mut state = 0x9e37_79b9_u64;
    for n in 0..100_000_usize {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        let mut bytes = seeds[n % seeds.len()].to_vec();
        let at = (state as usize) % bytes.len();
        bytes[at] = replacements[((state >> 17) as usize) % replacements.len()];
        if n % 5 == 0 {
            bytes.insert(
                at,
                replacements[((state >> 29) as usize) % replacements.len()],
            );
        }
        if n % 7 == 0 {
            bytes.remove(at);
        }
        if let Err(crate::diagnostic::InputError::InvalidUtf8 {
            start,
            end,
            bytes_hex,
            ..
        }) = crate::diagnostic::decode_utf8(bytes.clone())
        {
            assert!(start < end && end <= bytes.len(), "raw bytes {n}");
            assert_eq!(bytes_hex.len(), (end - start) * 2, "raw bytes {n}");
        }
        let source = String::from_utf8_lossy(&bytes);
        match super::lex::lex("fuzz.n", &source) {
            Ok(tokens) => {
                assert!(!tokens.is_empty());
            }
            Err(error) => assert!(
                error.validate("fuzz.n", &source).is_ok(),
                "lex {n}: {source:?}: {:?}",
                error.validate("fuzz.n", &source)
            ),
        }
        match parse("fuzz.n", &source) {
            Ok(ast) => {
                let formatted = super::format::format("fuzz.n", &source).unwrap();
                let formatted_ast = parse("fuzz.n", &formatted).unwrap();
                assert_eq!(
                    super::test_normalize::ast_without_spans(ast.clone()),
                    super::test_normalize::ast_without_spans(formatted_ast),
                    "formatted {n}: {source:?}"
                );
                assert_eq!(
                    super::format::format("fuzz.n", &formatted).unwrap(),
                    formatted,
                    "idempotence {n}: {source:?}"
                );
                let printed = print_canonical(&ast).unwrap();
                let reparsed = parse("fuzz.n", &printed).unwrap();
                assert_eq!(
                    super::test_normalize::ast_without_spans(ast),
                    super::test_normalize::ast_without_spans(reparsed),
                    "accepted {n}: {source:?}"
                );
            }
            Err(error) => {
                assert!(
                    error.validate("fuzz.n", &source).is_ok(),
                    "parse {n}: {source:?}: {:?}",
                    error.validate("fuzz.n", &source)
                );
                let format_error = super::format::format("fuzz.n", &source).unwrap_err();
                format_error.validate("fuzz.n", &source).unwrap();
            }
        }
    }
}

#[test]
fn token_sequence_mutations_never_panic_and_accepted_asts_reparse() {
    // Independent token-sequence fuzz, seed 0x4e45_584f_70ce_2026.
    let seeds = [
        "fn Main(){let x=1;}",
        "fn F(x:view I64)->I64{return x;} fn Main(){F(1);}",
        "fn Main(){let x=if true then 1 else 2;}",
        "fn Main(){var a=[1,2];a[0]=3;}",
    ];
    let tokenized = seeds.map(|source| {
        super::lex::lex("fuzz-token.n", source)
            .unwrap()
            .into_iter()
            .filter(|t| !matches!(t.kind, super::lex::Kind::Eof | super::lex::Kind::Newline))
            .map(|t| t.text)
            .collect::<Vec<_>>()
    });
    let replacements = [
        "fn", "Main", "let", "var", "view", "edit", "move", "if", "then", "else", "return",
        "value", "I64", "Bool", "1", "0", "(", ")", "[", "]", "{", "}", ":", ";", ",", "+", "=",
        "?", "@", "🙂",
    ];
    let mut state = 0x4e45_584f_70ce_2026_u64;
    for n in 0..100_000_usize {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        let mut tokens = tokenized[n % tokenized.len()].clone();
        let at = (state as usize) % tokens.len();
        let replacement = replacements[((state >> 17) as usize) % replacements.len()];
        match n % 3 {
            0 => tokens[at] = replacement.into(),
            1 => {
                tokens.insert(at, replacement.into());
            }
            _ => {
                tokens.remove(at);
            }
        }
        let source = tokens.join(" ");
        match super::lex::lex("fuzz-token.n", &source) {
            Ok(_) => {}
            Err(error) => assert!(error.validate("fuzz-token.n", &source).is_ok(), "lex {n}"),
        }
        match parse("fuzz-token.n", &source) {
            Ok(ast) => {
                let formatted = super::format::format("fuzz-token.n", &source).unwrap();
                let formatted_ast = parse("fuzz-token.n", &formatted).unwrap();
                assert_eq!(
                    super::test_normalize::ast_without_spans(ast.clone()),
                    super::test_normalize::ast_without_spans(formatted_ast),
                    "formatted {n}"
                );
                assert_eq!(
                    super::format::format("fuzz-token.n", &formatted).unwrap(),
                    formatted,
                    "idempotence {n}"
                );
                let printed = print_canonical(&ast).unwrap();
                let reparsed = parse("fuzz-token.n", &printed).unwrap();
                assert_eq!(
                    super::test_normalize::ast_without_spans(ast),
                    super::test_normalize::ast_without_spans(reparsed),
                    "{n}"
                );
            }
            Err(error) => {
                assert!(error.validate("fuzz-token.n", &source).is_ok(), "parse {n}");
                let format_error = super::format::format("fuzz-token.n", &source).unwrap_err();
                format_error.validate("fuzz-token.n", &source).unwrap();
            }
        }
    }
}

#[test]
fn lexer_and_parser_scale_linearly_across_surface_families() {
    fn program(fragment: &str, target: usize) -> String {
        let mut source = String::from("fn Main(){");
        while source.len() + fragment.len() < target {
            source.push_str(fragment);
        }
        source.push('}');
        source.push_str(&" ".repeat(target - source.len()));
        source
    }
    let families = [
        ("bindings", "let x=1;"),
        ("calls", "Print(1);"),
        ("arrays", "let x=[1,2,3];"),
        ("indexes", "let a=[1];a[0]=2;"),
        ("if_statements", "if true{}else{}"),
        ("while_statements", "while false{}"),
        ("nested_blocks", "{{let x=1;}}"),
        ("binary_expressions", "let x=1+2-3;"),
        ("if_expressions", "let x=if true then 1 else 2;"),
        ("line_comments", "// comentario\nlet x=1;"),
        ("crlf", "let x=1\r\n"),
        ("cr", "let x=1\r"),
        ("lf", "let x=1\n"),
        ("unicode_comments", "// á🙂 texto\nlet x=1;"),
        ("long_names", "let nombreMuyLargoDePrueba=1;"),
    ];
    for (name, fragment) in families {
        let small = program(fragment, 256 * 1024);
        let large = program(fragment, 1024 * 1024);
        let (_, lex_small) =
            super::lex::measure_lex_work(|| super::lex::lex("b-scale.n", &small).unwrap());
        let (_, lex_large) =
            super::lex::measure_lex_work(|| super::lex::lex("b-scale.n", &large).unwrap());
        let (parsed_small, parse_small) = super::parse::measure_parse_work(&small);
        let (parsed_large, parse_large) = super::parse::measure_parse_work(&large);
        println!(
            "{name}: lex {lex_small}/{lex_large} ({:.3}), parse {parse_small}/{parse_large} ({:.3})",
            lex_large as f64 / lex_small as f64,
            parse_large as f64 / parse_small as f64
        );
        assert!(
            parsed_small.is_ok() && parsed_large.is_ok(),
            "{name}: {:?} {:?}",
            parsed_small.err().map(|e| e.code),
            parsed_large.err().map(|e| e.code)
        );
        assert!(
            (3.8..=4.6).contains(&(lex_large as f64 / lex_small as f64)),
            "lex {name}: {lex_small} {lex_large}"
        );
        assert!(
            (3.8..=4.6).contains(&(parse_large as f64 / parse_small as f64)),
            "parse {name}: {parse_small} {parse_large}"
        );
    }
}

#[test]
fn printer_and_formatter_scale_in_emitted_and_allocated_bytes() {
    fn source(fragment: &str, target: usize) -> String {
        let mut text = String::from("fn Main(){");
        while text.len() + fragment.len() < target - 1 {
            text.push_str(fragment);
        }
        text.push('}');
        text.push_str(&" ".repeat(target - text.len()));
        text
    }
    let families = [
        ("bindings", "let x=1;"),
        ("calls", "Print(1);"),
        ("arrays", "let x=[1,2,3];"),
        ("indexes", "let a=[1];a[0]=2;"),
        ("if_statements", "if true{}else{}"),
        ("while_statements", "while false{}"),
        ("nested_blocks", "{{let x=1;}}"),
        ("binary_expressions", "let x=1+2-3;"),
        ("if_expressions", "let x=if true then 1 else 2;"),
        ("line_comments", "// comentario\nlet x=1;"),
        ("crlf", "let x=1\r\n"),
        ("unicode_comments", "// á🙂 texto\nlet x=1;"),
    ];
    for (name, fragment) in families {
        let mut samples = Vec::new();
        for target in [256 * 1024, 1024 * 1024] {
            let written = source(fragment, target);
            let ast = parse("print_scale_counter.n", &written).unwrap();
            crate::test_alloc::reset("print_scale_counter.n");
            let printed = print_canonical(&ast).unwrap();
            let print_alloc = crate::test_alloc::snapshot("print_scale_counter.n");
            crate::test_alloc::reset("format_scale_counter.n");
            let formatted = super::format::format("format_scale_counter.n", &written).unwrap();
            let format_alloc = crate::test_alloc::snapshot("format_scale_counter.n");
            assert!(print_alloc.0 > 0 && print_alloc.1 > 0, "print {name}");
            assert!(format_alloc.0 > 0 && format_alloc.1 > 0, "format {name}");
            samples.push((printed.len(), print_alloc, formatted.len(), format_alloc));
        }
        let small = samples[0];
        let large = samples[1];
        for (label, earlier, later) in [
            ("print emitted bytes", small.0 as u64, large.0 as u64),
            ("print allocated bytes", small.1.0, large.1.0),
            ("print allocations", small.1.1, large.1.1),
            ("format emitted bytes", small.2 as u64, large.2 as u64),
            ("format allocated bytes", small.3.0, large.3.0),
            ("format allocations", small.3.1, large.3.1),
        ] {
            let ratio = later as f64 / earlier as f64;
            println!("{name}: {label} {earlier}/{later} ({ratio:.3})");
            assert!((3.5..=4.6).contains(&ratio), "{name}: {label} {ratio:.3}");
        }
    }
}

#[test]
fn explicit_branch_comment_search_scales_by_comparison_count() {
    fn source(target: usize) -> String {
        let fragment = "let x=if a then { value: 1; } else 2; // c\n";
        let mut text = String::from("fn Main(){\n");
        while text.len() + fragment.len() + 2 <= target {
            text.push_str(fragment);
        }
        text.push('}');
        text
    }
    let mut steps = Vec::new();
    for size in [256 * 1024, 1024 * 1024] {
        super::emit::reset_comment_search_steps();
        let input = source(size);
        let formatted = super::format::format("comment_search_scale.n", &input).unwrap();
        assert!(formatted.contains("// c"));
        let count = super::emit::comment_search_steps();
        assert!(count > 0, "{size}: search was not exercised");
        steps.push(count);
    }
    let ratio = steps[1] as f64 / steps[0] as f64;
    println!("comment-search steps: {steps:?}, ratio {ratio:.3}");
    assert!(
        ratio <= 4.6,
        "comment-search steps: {steps:?}, ratio {ratio}"
    );
}

#[test]
fn long_identifiers_deep_blocks_and_strings_have_linear_work() {
    fn repeat_inside(prefix: &str, fragment: &str, suffix: &str, target: usize) -> String {
        let mut source = prefix.to_owned();
        while source.len() + fragment.len() + suffix.len() <= target {
            source.push_str(fragment);
        }
        source.push_str(suffix);
        source.push_str(&" ".repeat(target - source.len()));
        source
    }
    let name = format!("let {}=1;", "nombre".repeat(340));
    type ScaleBuilder = (&'static str, Box<dyn Fn(usize) -> String>);
    let builders: [ScaleBuilder; 2] = [
        (
            "long_identifier_2046_bytes",
            Box::new(move |target| repeat_inside("fn Main(){", &name, "}", target)),
        ),
        (
            "deep_blocks_127",
            Box::new(|target| {
                repeat_inside(
                    &format!("fn Main(){{{}", "{".repeat(126)),
                    "let x=1;",
                    &"}".repeat(127),
                    target,
                )
            }),
        ),
    ];
    for (family, builder) in builders {
        let small = builder(256 * 1024);
        let large = builder(1024 * 1024);
        let (_, lex_small) =
            super::lex::measure_lex_work(|| super::lex::lex("b-scale.n", &small).unwrap());
        let (_, lex_large) =
            super::lex::measure_lex_work(|| super::lex::lex("b-scale.n", &large).unwrap());
        let (parsed_small, parse_small) = super::parse::measure_parse_work(&small);
        let (parsed_large, parse_large) = super::parse::measure_parse_work(&large);
        println!(
            "{family}: lex {lex_small}/{lex_large} ({:.3}), parse {parse_small}/{parse_large} ({:.3})",
            lex_large as f64 / lex_small as f64,
            parse_large as f64 / parse_small as f64
        );
        assert!(
            parsed_small.is_ok() && parsed_large.is_ok(),
            "{family}: {:?} {:?}",
            parsed_small.err(),
            parsed_large.err()
        );
        assert!(
            (3.8..=4.6).contains(&(lex_large as f64 / lex_small as f64)),
            "lex {family}: {lex_small}/{lex_large}"
        );
        assert!(
            (3.8..=4.6).contains(&(parse_large as f64 / parse_small as f64)),
            "parse {family}: {parse_small}/{parse_large}"
        );
    }
    // Text is recognized but excluded in F0. Its full scan occurs in the
    // lexer; the parser then stops at the first text token by §3.6.
    let string_source = |target| repeat_inside("fn Main(){let x=\"", "é", "\";}", target);
    let small = string_source(256 * 1024);
    let large = string_source(1024 * 1024);
    for source in [&small, &large] {
        let error = parse("b-scale.n", source).unwrap_err();
        assert_eq!(error.code, "NX-E0006");
        assert_eq!(error.feature_id.as_deref(), Some("text"));
        assert!(error.validate("b-scale.n", source).is_ok());
    }
    let (_, lex_small) =
        super::lex::measure_lex_work(|| super::lex::lex("b-scale.n", &small).unwrap());
    let (_, lex_large) =
        super::lex::measure_lex_work(|| super::lex::lex("b-scale.n", &large).unwrap());
    println!(
        "multibyte_strings: lex {lex_small}/{lex_large} ({:.3})",
        lex_large as f64 / lex_small as f64
    );
    assert!(
        (3.8..=4.6).contains(&(lex_large as f64 / lex_small as f64)),
        "multibyte strings: {lex_small}/{lex_large}"
    );
}

#[test]
fn function_and_parameter_families_scale_linearly() {
    fn functions(target: usize) -> String {
        let function = format!("fn F(){{let x=1;//{}\n}}\n", "a".repeat(1020));
        let mut source = String::new();
        while source.len() + function.len() <= target {
            source.push_str(&function);
        }
        source.push_str(&" ".repeat(target - source.len()));
        source
    }
    fn parameters(target: usize) -> String {
        let mut source = String::from("fn F(");
        let param = "paramLongName:view I64,";
        while source.len() + param.len() + 4 <= target {
            source.push_str(param);
        }
        source.pop();
        source.push_str("){}");
        source.push_str(&" ".repeat(target - source.len()));
        source
    }
    for (name, builder) in [
        ("functions", functions as fn(usize) -> String),
        ("parameters", parameters),
    ] {
        let small = builder(256 * 1024);
        let large = builder(1024 * 1024);
        let (_, lex_small) =
            super::lex::measure_lex_work(|| super::lex::lex("b-scale.n", &small).unwrap());
        let (_, lex_large) =
            super::lex::measure_lex_work(|| super::lex::lex("b-scale.n", &large).unwrap());
        let (parsed_small, parse_small) = super::parse::measure_parse_work(&small);
        let (parsed_large, parse_large) = super::parse::measure_parse_work(&large);
        assert!(
            parsed_small.is_ok() && parsed_large.is_ok(),
            "{name}: {:?} {:?}",
            parsed_small.err().map(|e| e.code),
            parsed_large.err().map(|e| e.code)
        );
        println!(
            "{name}: lex {lex_small}/{lex_large} ({:.3}), parse {parse_small}/{parse_large} ({:.3})",
            lex_large as f64 / lex_small as f64,
            parse_large as f64 / parse_small as f64
        );
        assert!(
            (3.8..=4.6).contains(&(lex_large as f64 / lex_small as f64)),
            "lex {name}"
        );
        assert!(
            (3.8..=4.6).contains(&(parse_large as f64 / parse_small as f64)),
            "parse {name}"
        );
    }
}

#[test]
fn small_stack_child_probe() {
    let Ok(spec) = std::env::var("NEXO_B_STACK_PROBE") else {
        return;
    };
    let parts: Vec<_> = spec.split(':').collect();
    let (family, depth, size) = match parts.as_slice() {
        [family, size] => (*family, 126, size.parse::<usize>().unwrap()),
        [family, depth, size] => (
            *family,
            depth.parse::<usize>().unwrap(),
            size.parse::<usize>().unwrap(),
        ),
        _ => panic!("invalid stack probe specification: {spec}"),
    };
    let source = match family {
        "paren" => format!(
            "fn Main(){{let x={}1{};}}",
            "(".repeat(depth),
            ")".repeat(depth)
        ),
        "array" => format!(
            "fn Main(){{let x={}1{};}}",
            "[".repeat(depth),
            "]".repeat(depth)
        ),
        "block" => format!("fn Main(){{{}{}}}", "{".repeat(depth), "}".repeat(depth)),
        "type" => format!(
            "fn F(x:view {}I64{}){{}}",
            "Array<".repeat(depth),
            ">".repeat(depth)
        ),
        "mixed" => format!(
            "fn Main(){{let x={}1{};}}",
            "([".repeat(depth / 2),
            "])".repeat(depth / 2)
        ),
        "index" => format!("fn Main(){{let x=a{};}}", "[0]".repeat(depth)),
        "call" => format!("fn Main(){{let x=F{};}}", "()".repeat(depth)),
        "binary" => format!(
            "fn Main(){{let x={}1{};}}",
            "1+(".repeat(depth),
            ")".repeat(depth)
        ),
        "if" => format!(
            "fn Main(){{let x={}1{};}}",
            "if true then (".repeat(depth),
            ") else 0".repeat(depth)
        ),
        _ => unreachable!(),
    };
    let probe = move || match parse("stack-b.n", &source) {
        Ok(ast) => {
            let printed = print_canonical(&ast).unwrap();
            assert!(parse("stack-b.n", &printed).is_ok());
            let formatted = super::format::format("stack-b.n", &source).unwrap();
            assert!(parse("stack-b.n", &formatted).is_ok());
        }
        Err(error) => {
            assert_eq!(error.code, "NX-E0003");
            assert!(error.validate("stack-b.n", &source).is_ok());
        }
    };
    if size == 0 {
        probe();
    } else {
        std::thread::Builder::new()
            .stack_size(size * 1024)
            .spawn(probe)
            .unwrap()
            .join()
            .unwrap();
    }
}

#[test]
fn parser_and_printer_survive_small_stacks() {
    let binary = std::env::current_exe().unwrap();
    for family in [
        "paren", "array", "block", "type", "index", "call", "binary", "if",
    ] {
        for size in [128, 256, 512] {
            let spec = format!("{family}:{size}");
            let output = std::process::Command::new(&binary)
                .args(["--exact", "frontend::tests::small_stack_child_probe"])
                .env("NEXO_B_STACK_PROBE", &spec)
                .output()
                .unwrap();
            assert!(
                output.status.success(),
                "{spec}: {} {}",
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr)
            );
        }
    }
}

#[test]
fn parser_small_stack_matrix_retains_nested_b_forms() {
    let binary = std::env::current_exe().unwrap();
    for family in ["paren", "array", "block", "type", "mixed", "if"] {
        for depth in [1, 2, 4, 7, 8, 15, 16, 31, 32, 33, 63, 126, 127] {
            for size in [0, 256, 512] {
                let spec = format!("{family}:{depth}:{size}");
                let output = std::process::Command::new(&binary)
                    .args(["--exact", "frontend::tests::small_stack_child_probe"])
                    .env("NEXO_B_STACK_PROBE", &spec)
                    .output()
                    .unwrap();
                assert!(
                    output.status.success(),
                    "{spec}: {} {}",
                    String::from_utf8_lossy(&output.stdout),
                    String::from_utf8_lossy(&output.stderr)
                );
            }
        }
    }
}

#[test]
fn first_sets_and_excluded_operators_match_published_literals() {
    let rows: &[(&str, &str, &[&str])] = &[
        (
            "fn Main(x:)",
            "grammar.ParamMode",
            &["edit", "move", "view"],
        ),
        (
            "fn Main(x:view)",
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
        ),
        ("fn Main(x:view I64,)", "grammar.Parameter", &["Name"]),
        (
            "fn Main(){let x=;}",
            "binding_initializer",
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
        ),
        (
            "fn Main(){let x=1 and 2;}",
            "unsupported_binary_operator",
            &["+", "-", "<", "=="],
        ),
        (
            "fn Main(){let x=not true;}",
            "unsupported_unary_operator",
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
        ),
        ("fn Main(){let x=1..2;}", "range_endpoint", &["..<", "..="]),
        (
            "fn Main(){let x=if true {value:1;}else{value:2;};}",
            "grammar.IfExpr",
            &["then"],
        ),
        (
            "fn Main(){let x=if true then 1 else if false then 2 else 3;}",
            "grammar.BranchOperand",
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
                "{",
            ],
        ),
    ];
    for &(source, context, expected) in rows {
        let error = parse("b.n", source).unwrap_err();
        assert_eq!(error.code, "NX-E0002", "{source}");
        assert_eq!(error.parser_context.as_deref(), Some(context), "{source}");
        assert_eq!(error.expected_tokens, expected, "{source}");
        assert!(
            error.validate("b.n", source).is_ok(),
            "{source}: {:?}",
            error.validate("b.n", source)
        );
    }
}

#[test]
fn type_close_split_and_contextual_delimiters() {
    use super::parse::{Expr, Statement, Type};
    let source = "fn Main(){let x:Result<I64,I64>=Ok(1);}";
    let ast = parse("b.n", source).unwrap();
    assert!(matches!(
        &ast.functions[0].body.statements[0],
        Statement::Let {
            annotation: Some(Type::Result(_, _)),
            value: Expr::Call { .. },
            ..
        }
    ));
    let printed = print_canonical(&ast).unwrap();
    assert!(printed.contains("Result<I64, I64> = Ok(1)"));
    let invalid = "fn Main(){value:1;}";
    let error = parse("b.n", invalid).unwrap_err();
    assert_eq!(error.rule, "NX-E1302/invalid_value_target");
    assert_eq!(
        (
            error.primary_span.as_ref().unwrap().start,
            error.primary_span.as_ref().unwrap().end
        ),
        (10, 16)
    );
    assert!(error.validate("b.n", invalid).is_ok());
    let empty = "fn Main(){let x=if true then {value:}else{value:1;};}";
    let error = parse("b.n", empty).unwrap_err();
    assert_eq!(error.rule, "NX-E1302/missing_value_operand");
    assert!(error.validate("b.n", empty).is_ok());
}

#[test]
fn published_syntax_rows_have_exact_tokens_spans_and_prefixes() {
    use crate::diagnostic::TokenKind;
    // F0-D15 §6.3. Expected values are transcribed from the ADR.
    let rows = [
        ("fn", "function_name", TokenKind::Eof, (2, 2), (0, 2)),
        ("fn Main(", "parameter_list", TokenKind::Eof, (8, 8), (7, 8)),
        (
            "fn Main(){",
            "block_item",
            TokenKind::Eof,
            (10, 10),
            (9, 10),
        ),
        (
            "fn Main(){let a=",
            "binding_initializer",
            TokenKind::Eof,
            (16, 16),
            (10, 16),
        ),
        (
            "fn Main(){let a=-",
            "signed_integer",
            TokenKind::Eof,
            (17, 17),
            (16, 17),
        ),
        (
            "fn Main(){let a=[1,",
            "array_element",
            TokenKind::Eof,
            (19, 19),
            (16, 19),
        ),
        (
            "fn Main(){let a=1;} garbage",
            "module_item",
            TokenKind::Name,
            (20, 27),
            (0, 20),
        ),
        (
            "// expect: NX-E0002\nfn Main() {} ignored",
            "module_item",
            TokenKind::Name,
            (33, 40),
            (0, 33),
        ),
        (
            "// expect: NX-E0002\r\nfn Main() {} ignored",
            "module_item",
            TokenKind::Name,
            (34, 41),
            (0, 34),
        ),
        (
            "fn Main(){let match=1;}",
            "binding_name",
            TokenKind::Keyword,
            (14, 19),
            (10, 14),
        ),
        (
            "fn Main(){let x=1*2;}",
            "unsupported_binary_operator",
            TokenKind::Symbol,
            (17, 18),
            (10, 17),
        ),
        (
            "fn Main(){let x=1!=2;}",
            "unsupported_binary_operator",
            TokenKind::Symbol,
            (17, 19),
            (10, 17),
        ),
        (
            "fn F(x:edit I64){} fn Main(){F(edit 1);}",
            "inout_place",
            TokenKind::Integer,
            (36, 37),
            (31, 36),
        ),
        (
            "fn Main(){let x=move 1;}",
            "consume_place",
            TokenKind::Integer,
            (21, 22),
            (16, 21),
        ),
        (
            "fn Main(){let x=1>2;}",
            "unsupported_binary_operator",
            TokenKind::Symbol,
            (17, 18),
            (10, 17),
        ),
        (
            "fn Main(){let x=1<=2;}",
            "unsupported_binary_operator",
            TokenKind::Symbol,
            (17, 19),
            (10, 17),
        ),
        (
            "fn Main(){let x=1>=2;}",
            "unsupported_binary_operator",
            TokenKind::Symbol,
            (17, 19),
            (10, 17),
        ),
        (
            "fn Main() ;",
            "function_suffix",
            TokenKind::Symbol,
            (10, 11),
            (0, 10),
        ),
        (
            "fn F(x:I64){}",
            "grammar.ParamMode",
            TokenKind::Name,
            (7, 10),
            (7, 7),
        ),
        (
            "fn F(x:view){}",
            "grammar.Type",
            TokenKind::Symbol,
            (11, 12),
            (11, 11),
        ),
        (
            "fn Main(){let x=if true{value:1;}else{value:2;};}",
            "grammar.IfExpr",
            TokenKind::Symbol,
            (23, 24),
            (16, 23),
        ),
        (
            "fn Main(){let x=0..1;}",
            "range_endpoint",
            TokenKind::Symbol,
            (17, 19),
            (16, 17),
        ),
        (
            "fn Main(){let x=not true;}",
            "unsupported_unary_operator",
            TokenKind::Keyword,
            (16, 19),
            (10, 16),
        ),
        (
            "fn Main(){let x=true and false;}",
            "unsupported_binary_operator",
            TokenKind::Keyword,
            (21, 24),
            (10, 21),
        ),
        (
            "fn Main(){let x=true or false;}",
            "unsupported_binary_operator",
            TokenKind::Keyword,
            (21, 23),
            (10, 21),
        ),
        (
            "fn Main(){let x=1/2;}",
            "unsupported_binary_operator",
            TokenKind::Symbol,
            (17, 18),
            (10, 17),
        ),
        (
            "fn Main(){let x=1%2;}",
            "unsupported_binary_operator",
            TokenKind::Symbol,
            (17, 18),
            (10, 17),
        ),
    ];
    assert_eq!(rows.len(), 27, "F0-D15 §6.3 sintaxis publicada");
    for (source, context, kind, primary, prefix) in rows {
        let error = parse("b.n", source).unwrap_err();
        assert_eq!(error.code, "NX-E0002", "{source}");
        assert_eq!(error.schema_version, 1, "{source}");
        assert_eq!(
            error.diagnostic_namespace,
            crate::diagnostic::Namespace::NexoProposedV1,
            "{source}"
        );
        assert_eq!(error.profile_id, "f0-0002-b-v2", "{source}");
        assert_eq!(error.subcause, None, "{source}");
        assert_eq!(error.rule, "NX-E0002", "{source}");
        assert_eq!(
            error.explanation_id.as_deref(),
            Some("NX-E0002"),
            "{source}"
        );
        assert_eq!(
            error.severity,
            crate::diagnostic::Severity::Error,
            "{source}"
        );
        assert_eq!(error.stage, crate::diagnostic::Stage::Parse, "{source}");
        assert_eq!(
            error.operation,
            crate::diagnostic::Operation::SyntaxParse,
            "{source}"
        );
        assert_eq!(error.related_spans, [], "{source}");
        assert_eq!(error.cause_chain, [], "{source}");
        assert_eq!(error.feature_id, None, "{source}");
        assert_eq!(error.repair, [], "{source}");
        assert!(error.data.is_empty(), "{source}");
        assert_eq!(error.parser_context.as_deref(), Some(context), "{source}");
        let token = error.unexpected_token.as_ref().unwrap();
        assert_eq!(token.kind, kind, "{source}");
        assert_eq!(
            token.text.as_deref(),
            Some(&source[primary.0 as usize..primary.1 as usize]),
            "{source}"
        );
        assert_eq!((token.span.start, token.span.end), primary, "{source}");
        let span = error.primary_span.as_ref().unwrap();
        assert_eq!((span.start, span.end), primary, "{source}");
        let span = error.recognized_prefix.as_ref().unwrap();
        assert_eq!((span.start, span.end), prefix, "{source}");
        assert_eq!(
            error.expected_tokens_kind,
            Some(crate::diagnostic::ExpectedTokensKind::AdmissibleSet),
            "{source}"
        );
        let expected: &[&str] = match context {
            "function_name" | "binding_name" => &["Name"],
            "parameter_list" => &[")", "Name"],
            "block_item" => &[
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
            "binding_initializer" | "unsupported_unary_operator" => &[
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
            "signed_integer" => &["IntLiteral"],
            "array_element" => &[
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
            "module_item" => &["EOF", "fn"],
            "unsupported_binary_operator" => &["+", "-", "<", "=="],
            "inout_place" | "consume_place" => &["Place"],
            "function_suffix" => &["->", "with", "{"],
            "grammar.ParamMode" => &["edit", "move", "view"],
            "grammar.Type" => &[
                "(",
                "Array",
                "Bool",
                "I64",
                "MockBlockingResource",
                "MockResource",
                "Result",
            ],
            "grammar.IfExpr" => &["then"],
            "range_endpoint" => &["..<", "..="],
            _ => panic!("untranscribed ADR context: {context}"),
        };
        assert_eq!(error.expected_tokens, expected, "{source}");
        assert!(
            error.validate("b.n", source).is_ok(),
            "{source}: {:?}",
            error.validate("b.n", source)
        );
    }
    for invalid in ['!', '&', '|'] {
        let source = format!("fn Main(){{let x=1{invalid}2;}}");
        let error = parse("b.n", &source).unwrap_err();
        assert_eq!(error.code, "NX-E0001");
        assert_eq!(
            (
                error.primary_span.as_ref().unwrap().start,
                error.primary_span.as_ref().unwrap().end
            ),
            (17, 18)
        );
        assert!(error.validate("b.n", &source).is_ok());
    }
}
