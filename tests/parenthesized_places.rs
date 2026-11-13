use nexo::{diagnostic::Diagnostic, ir};

const FILE: &str = "parenthesized-places.n";

#[allow(clippy::vec_box)] // Mirrors the boxed diagnostic API returned by compilation.
fn rejected(source: &str, run: bool) -> Vec<Box<Diagnostic>> {
    let result = if run {
        ir::compile_scalar(FILE, source)
    } else {
        ir::check_scalar(FILE, source)
    };
    match result {
        Err(ir::CompileError::Rejected(errors)) => errors,
        other => panic!("expected rejection: {source}: {other:?}"),
    }
}

fn assert_same_diagnostic(source: &str, expected: (&str, Option<&str>, (u64, u64))) {
    let check = rejected(source, false);
    let run = rejected(source, true);
    assert_eq!(check.len(), 1, "{check:?}");
    assert_eq!(run.len(), 1, "{run:?}");
    let span = |diagnostic: &Diagnostic| {
        let span = diagnostic.primary_span.as_ref().unwrap();
        (span.start, span.end)
    };
    for diagnostic in [&check[0], &run[0]] {
        assert_eq!(diagnostic.code, expected.0);
        assert_eq!(diagnostic.subcause.as_deref(), expected.1);
        assert_eq!(span(diagnostic), expected.2);
        diagnostic.validate(FILE, source).unwrap();
    }
    assert_eq!(check[0].data, run[0].data);
}

#[test]
fn parenthesized_indexed_place_type_error_keeps_exact_diagnostic() {
    let source = "fn Main(){let a=[1]; (a[0]) = true;}";
    assert_same_diagnostic(source, ("NX-E0102", Some("type_mismatch"), (30, 34)));
}

#[test]
fn parenthesized_immutable_place_keeps_exact_diagnostic() {
    let source = "fn Main(){let x=0; (x) = 1;}";
    assert_same_diagnostic(source, ("NX-E0105", None, (20, 21)));
}
