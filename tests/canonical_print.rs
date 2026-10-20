use nexo::frontend::parse::{Expr, Statement, parse};
use nexo::frontend::print::print_canonical;

#[test]
fn canonical_text_round_trips_written_modes_and_statement_terminators() {
    let source = "fn f(inout x:i64,consume y:i64)->i64{\nlet z=x+y\nreturn z\n}";
    let ast = parse("canonical.nx", source).unwrap();
    let text = print_canonical(&ast);
    assert!(text.contains("inout x: i64"));
    assert!(text.contains("consume y: i64"));
    assert!(text.contains("let z = x + y;"));
    assert!(text.contains("return z;"));
    assert_eq!(
        print_canonical(&parse("canonical.nx", &text).unwrap()),
        text
    );
    assert!(matches!(
        &ast.functions[0].body.statements[0],
        Statement::Let {
            value: Expr::Binary { .. },
            ..
        }
    ));
}

fn ast_without_spans(ast: &nexo::frontend::parse::Ast) -> String {
    let debug = format!("{ast:?}");
    let mut result = String::new();
    let mut rest = debug.as_str();
    while let Some(start) = rest.find("Span {") {
        result.push_str(&rest[..start]);
        let end = rest[start..].find('}').expect("span closes") + start + 1;
        result.push_str("Span");
        rest = &rest[end..];
    }
    result.push_str(rest);
    result
}

#[test]
fn canonical_statement_if_else_variants_preserve_structure_and_text() {
    let headers = [
        "fn f()",
        "fn f(x:i64)",
        "fn f() with no_io",
        "fn f() with {call(no_alloc)}",
    ];
    let bodies = [
        "if true{}else{}",
        "if true{}else{x;}",
        "if true{}else if false{}",
        "if true{}else if false{}else{}",
        "if true{}else if false{}else{x;}",
        "{if true{}else if false{}else{x;}}",
    ];
    for header in headers {
        for body in bodies {
            let source = format!("{header}{{{body}}}");
            let original = parse("canonical.nx", &source).unwrap();
            let printed = print_canonical(&original);
            let reparsed = parse("canonical.nx", &printed).unwrap();
            assert_eq!(
                ast_without_spans(&original),
                ast_without_spans(&reparsed),
                "{source}"
            );
            assert_eq!(printed, print_canonical(&reparsed), "{source}");
        }
    }
}
