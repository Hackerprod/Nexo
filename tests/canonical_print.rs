use nexo::frontend::parse::{Expr, Statement, parse};
use nexo::frontend::print::print_canonical;

#[test]
fn accepted_corpus_files_format_like_the_canonical_printer() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("corpus");
    let mut visited = 0;
    let mut accepted = 0;
    for subdir in ["pass", "fail"] {
        for file in std::fs::read_dir(root.join(subdir)).unwrap() {
            let path = file.unwrap().path();
            if path.extension().is_none_or(|extension| extension != "nx") {
                continue;
            }
            visited += 1;
            let source = std::fs::read_to_string(&path).unwrap();
            let source = source
                .strip_prefix("// expect:")
                .map_or(source.as_str(), |_| source.split_once('\n').unwrap().1);
            let Ok(ast) = parse(path.to_str().unwrap(), source) else {
                continue;
            };
            accepted += 1;
            let canonical = print_canonical(&ast).unwrap();
            assert_eq!(
                nexo::frontend::format::format(path.to_str().unwrap(), source).unwrap(),
                canonical,
                "{}",
                path.display()
            );
            assert_eq!(
                print_canonical(&parse(path.to_str().unwrap(), &canonical).unwrap()).unwrap(),
                canonical
            );
        }
    }
    assert_eq!(visited, 15);
    assert_eq!(accepted, 13, "accepted {accepted}/15 corpus files");
}

#[test]
fn canonical_text_round_trips_written_modes_and_statement_terminators() {
    let source = "fn F(x: edit I64, y: move I64) -> I64 {\n    let z = x + y;\n    return z;\n}\n";
    let ast = parse("canonical.nx", source).unwrap();
    let text = print_canonical(&ast).unwrap();
    assert!(text.contains("x: edit I64"));
    assert!(text.contains("y: move I64"));
    assert!(text.contains("let z = x + y;"));
    assert!(text.contains("return z;"));
    assert_eq!(
        print_canonical(&parse("canonical.nx", &text).unwrap()).unwrap(),
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
        "fn f(x:view I64)",
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
            let printed = print_canonical(&original).unwrap();
            let reparsed = parse("canonical.nx", &printed).unwrap();
            assert_eq!(
                ast_without_spans(&original),
                ast_without_spans(&reparsed),
                "{source}"
            );
            assert_eq!(printed, print_canonical(&reparsed).unwrap(), "{source}");
        }
    }
}
