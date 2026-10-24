use super::{check_proposed, names};
use crate::diagnostic::{
    DataValue, Diagnostic, Namespace, Operation, RelatedRole, Severity, Span, Stage,
};
use crate::frontend::parse;

const FILE: &str = "i03.nx";

fn byte_span(source: &str, needle: &str, occurrence: usize) -> (u64, u64) {
    let mut offset = 0;
    let mut found = 0;
    while let Some(local) = source[offset..].find(needle) {
        let start = offset + local;
        if found == occurrence {
            return (start as u64, (start + needle.len()) as u64);
        }
        found += 1;
        offset = start + needle.len();
    }
    panic!("missing occurrence {occurrence} of {needle:?}");
}

#[test]
fn binary_and_collection_siblings_keep_both_independent_type_errors() {
    // F0-0002 §13.9: each faulty operand is an independent diagnostic site.
    for (body, first, second) in [
        ("let w=((1+())+(1+()));", "1+()", "1+()"),
        ("let w=((1+())-(1+()));", "1+()", "1+()"),
        ("let w=((1+())<(1+()));", "1+()", "1+()"),
        ("let w=((1+())==(1+()));", "1+()", "1+()"),
        ("let w=[1][true]+(1+());", "true", "1+()"),
        ("let w=[(1+()),(1+())];", "1+()", "1+()"),
    ] {
        let source = format!("fn Main(){{{body}}}");
        let first_at = source.find(first).unwrap();
        let second_at = source[first_at + first.len()..]
            .find(second)
            .map(|at| at + first_at + first.len());
        let expected = if first == "true" {
            vec![
                (first_at as u64, (first_at + 4) as u64),
                (
                    (second_at.unwrap() + 2) as u64,
                    (second_at.unwrap() + 4) as u64,
                ),
            ]
        } else {
            vec![
                ((first_at + 2) as u64, (first_at + 4) as u64),
                (
                    (second_at.unwrap() + 2) as u64,
                    (second_at.unwrap() + 4) as u64,
                ),
            ]
        };
        let diagnostics = check_proposed(FILE, &source, true).expect_err(&source);
        assert_eq!(diagnostics.len(), 2, "{source}: {diagnostics:?}");
        for (diagnostic, (start, end)) in diagnostics.iter().zip(expected) {
            assert_eq!(diagnostic.code, "NX-E0102", "{source}");
            assert_eq!(
                diagnostic.subcause.as_deref(),
                Some("type_mismatch"),
                "{source}"
            );
            assert_eq!(diagnostic.stage, Stage::Types, "{source}");
            assert_eq!(diagnostic.operation, Operation::TypesCheck, "{source}");
            assert_eq!(diagnostic.primary_span, Some(sp(start, end)), "{source}");
            assert_eq!(diagnostic.unexpected_token, None, "{source}");
            assert!(diagnostic.expected_tokens.is_empty(), "{source}");
            assert_eq!(diagnostic.data.len(), 2, "{source}");
            assert_eq!(
                diagnostic.data.get("expected_type"),
                Some(&DataValue::String("I64".into())),
                "{source}"
            );
            assert_eq!(
                diagnostic.data.get("found_type"),
                Some(&DataValue::String(
                    if first == "true" && start == first_at as u64 {
                        "Bool".into()
                    } else {
                        "()".into()
                    }
                )),
                "{source}"
            );
            diagnostic.validate(FILE, &source).unwrap();
        }
    }
}

#[test]
fn type_errors_sort_across_operands_statements_arguments_and_functions() {
    // F0-0002 §13.9 orders independent sites by primary_span.start.
    for source in [
        "fn Main() {\n    let w = ((1 + ()) + (1 + ()));\n    let x: I64 = ();\n}\n",
        "fn F(x: view I64, y: view I64) {}\nfn Main() {\n    F(1 + (), 1 + ());\n    let z: I64 = ();\n}\n",
        "fn A() {\n    let x = ((1 + ()) + (1 + ()));\n}\nfn Main() {\n    let y: I64 = ();\n}\n",
    ] {
        let expected = source
            .match_indices("()")
            .filter(|(at, _)| source[..*at].ends_with("+ ") || source[..*at].ends_with("= "))
            .map(|(at, _)| at as u64)
            .collect::<Vec<_>>();
        let diagnostics = check_proposed(FILE, source, true).expect_err(source);
        let actual = diagnostics
            .iter()
            .map(|d| {
                assert_eq!(d.code, "NX-E0102", "{source}: {d:?}");
                d.primary_span.as_ref().unwrap().start
            })
            .collect::<Vec<_>>();
        assert_eq!(actual, expected, "{source}: {diagnostics:?}");
        assert!(actual.len() >= 3);
    }
}

#[test]
fn comparison_with_divergent_right_marks_operator_for_invalid_bool_left() {
    // F0-0002 S16: '<' requires i64; a divergent right operand contributes no type.
    for left in ["true", "()", "1"] {
        for (op, right) in [
            ("<", "(if true then {return;} else {return;})"),
            ("<", "false"),
            ("==", "(if true then {return;} else {return;})"),
            ("==", "false"),
        ] {
            let source = format!("fn Main(){{let v0={left};let w=v0{op}{right};}}");
            let result = check_proposed(FILE, &source, true);
            let invalid = (op == "<" && (left != "1" || right == "false"))
                || (op == "==" && left != "true" && right == "false");
            if invalid {
                let diagnostics = result.expect_err(&source);
                assert_eq!(diagnostics.len(), 1, "{source}");
                let diagnostic = &diagnostics[0];
                assert_eq!(diagnostic.code, "NX-E0102", "{source}");
                assert_eq!(
                    diagnostic.subcause.as_deref(),
                    Some("type_mismatch"),
                    "{source}"
                );
                assert_eq!(diagnostic.stage, Stage::Types, "{source}");
                assert_eq!(diagnostic.operation, Operation::TypesCheck, "{source}");
                let operator_site = op == "<" && left == "true";
                let start = if operator_site {
                    source.find('<').unwrap()
                } else {
                    source
                        .find("false")
                        .unwrap_or_else(|| source.find('<').unwrap())
                } as u64;
                let width = if operator_site || right.starts_with("(if") {
                    1
                } else {
                    5
                };
                assert_eq!(
                    diagnostic.primary_span,
                    Some(sp(start, start + width)),
                    "{source}"
                );
            } else {
                result.unwrap_or_else(|diagnostics| panic!("{source}: {diagnostics:?}"));
            }
        }
    }
}
fn sp(start: u64, end: u64) -> Span {
    Span {
        file: FILE.into(),
        start,
        end,
    }
}

#[test]
fn divergent_left_keeps_independent_numeric_operand_sites() {
    let dv = "(if true then {return;} else {return;})";
    for body in [
        "let a=@+();let b:I64=();",
        "let b:I64=();let a=@-();",
        "let a=@<();let b:I64=();",
        "let a=(@+())-();let b:I64=();",
    ] {
        let source = format!("fn Main(){{{}}}", body.replacen('@', dv, 1));
        let errors = check_proposed(FILE, &source, true).expect_err(&source);
        assert_eq!(errors.len(), 2, "{source}: {errors:?}");
        if body.contains("@<()") {
            let unit = source.find("<()").unwrap() as u64 + 1;
            let comparison = errors
                .iter()
                .find(|d| d.primary_span.as_ref().is_some_and(|s| s.start == unit))
                .unwrap();
            assert!(
                comparison
                    .related_spans
                    .iter()
                    .any(|s| s.role == RelatedRole::ExpectedType)
            );
        }
        for error in errors {
            assert_eq!(error.code, "NX-E0102", "{source}");
            assert_eq!(error.subcause.as_deref(), Some("type_mismatch"));
            assert_eq!(error.stage, Stage::Types);
            assert_eq!(error.operation, Operation::TypesCheck);
            assert_eq!(
                error.data.get("expected_type"),
                Some(&DataValue::String("I64".into()))
            );
            assert_eq!(
                error.data.get("found_type"),
                Some(&DataValue::String("()".into()))
            );
            error.validate(FILE, &source).unwrap();
        }
    }
}

#[test]
fn nested_calls_and_arrays_collect_each_independent_type_site() {
    let dv = "(if true then {return;} else {return;})";
    for source in [
        format!("fn G(x:view I64){{}} fn Main(){{G({dv}+());G(());}}"),
        format!("fn G(x:view I64){{}} fn Main(){{let a=[G({dv}+()),G(())];}}"),
        format!("fn G(x:view I64){{}} fn Main(){{let a=G({dv}+())+G(());}}"),
        format!("fn G(x:view I64){{}} fn Main(){{let a=G(())-G({dv}+());}}"),
    ] {
        let errors = check_proposed(FILE, &source, true).expect_err(&source);
        assert_eq!(errors.len(), 2, "{source}: {errors:?}");
        for error in errors {
            assert_eq!(error.code, "NX-E0102", "{source}");
            assert_eq!(error.subcause.as_deref(), Some("type_mismatch"));
            assert_eq!(error.stage, Stage::Types);
            assert_eq!(error.operation, Operation::TypesCheck);
            assert_eq!(
                error.data.get("expected_type"),
                Some(&DataValue::String("I64".into()))
            );
            assert_eq!(
                error.data.get("found_type"),
                Some(&DataValue::String("()".into()))
            );
            error.validate(FILE, &source).unwrap();
        }
    }
}

#[test]
fn deferred_equality_does_not_force_numeric_right_operand() {
    for right in ["true", "false", "()"] {
        let source =
            format!("fn Main(){{let v=if true then {{return;}}else{{return;}};let b=v=={right};}}");
        check_proposed(FILE, &source, true).unwrap_or_else(|errors| panic!("{source}: {errors:?}"));
    }
}

#[test]
fn tainted_left_does_not_suppress_direct_error_on_right() {
    let source = "fn G(x: view I64) {}\nfn Main() {\n    let a = G(()) - ((if true then {\n        return;\n    } else {\n        return;\n    }) + ());\n}\n";
    let errors = check_proposed(FILE, source, true).expect_err(source);
    assert_eq!(errors.len(), 2, "{errors:?}");
    for error in errors {
        assert_eq!(error.code, "NX-E0102");
        error.validate(FILE, source).unwrap();
    }
}

#[test]
fn array_visits_second_element_after_first_type_error() {
    let source = "fn Main() {\n    let a = [0 + (), 1 + ()];\n}\n";
    let errors = check_proposed(FILE, source, true).expect_err(source);
    assert_eq!(errors.len(), 2, "{errors:?}");
    for error in errors {
        assert_eq!(error.code, "NX-E0102");
        error.validate(FILE, source).unwrap();
    }
}

fn error(source: &str, run_entry: bool) -> Box<Diagnostic> {
    let mut errors = check_proposed(FILE, source, run_entry).expect_err(source);
    assert!(!errors.is_empty());
    let d = errors.remove(0);
    d.validate(FILE, source).unwrap();
    d
}

#[test]
fn adr_gap_function_name_is_not_an_assignable_place() {
    for source in [
        "fn M(){M=();}",
        "fn Main(){Main=();}",
        "fn F(){} fn Main(){F=();}",
        "fn F(){} fn Main(){(F)=();}",
        "fn F(){} fn Main(){F[0]=();}",
    ] {
        let d = error(source, true);
        assert_eq!(d.code, "NX-E0006", "{source}");
        assert_eq!(d.feature_id.as_deref(), Some("function_value"), "{source}");
        assert_eq!(d.stage, Stage::Names, "{source}");
        assert_eq!(d.operation, Operation::NamesResolve, "{source}");
        let root = if source.contains("F=") || source.contains("(F)") || source.contains("F[") {
            source.rfind('F').unwrap()
        } else if source.contains("M=()") {
            source.rfind('M').unwrap()
        } else {
            source.rfind("Main").unwrap()
        } as u64;
        assert_eq!(
            d.primary_span.as_ref().map(|s| (s.start, s.end)),
            Some((root, root + if source.contains("Main=()") { 4 } else { 1 }))
        );
    }
}

#[allow(clippy::too_many_arguments)]
fn exact(
    d: &Diagnostic,
    code: &str,
    sub: Option<&str>,
    stage: Stage,
    operation: Operation,
    primary: (u64, u64),
    related: &[(RelatedRole, u64, u64)],
    data: &[(&str, DataValue)],
) {
    assert_eq!(d.schema_version, 1);
    assert_eq!(d.diagnostic_namespace, Namespace::NexoProposedV1);
    assert_eq!(d.profile_id, "f0-0002-b-v2");
    assert_eq!(d.code, code);
    assert_eq!(d.subcause.as_deref(), sub);
    let rule = sub.map_or_else(|| code.to_owned(), |s| format!("{code}/{s}"));
    assert_eq!(d.rule, rule);
    assert_eq!(d.explanation_id.as_deref(), Some(rule.as_str()));
    assert_eq!(d.severity, Severity::Error);
    assert_eq!(d.stage, stage);
    assert_eq!(d.operation, operation);
    assert_eq!(d.primary_span, Some(sp(primary.0, primary.1)));
    assert_eq!(
        d.related_spans
            .iter()
            .map(|r| (r.role, r.span.start, r.span.end))
            .collect::<Vec<_>>(),
        related
    );
    assert_eq!(
        d.data,
        data.iter()
            .map(|(k, v)| ((*k).to_owned(), v.clone()))
            .collect()
    );
    assert_eq!(d.feature_id, None);
    assert_eq!(d.unexpected_token, None);
    assert!(d.expected_tokens.is_empty());
    assert_eq!(d.expected_tokens_kind, None);
    assert_eq!(d.parser_context, None);
    assert_eq!(d.recognized_prefix, None);
    assert!(d.cause_chain.is_empty());
    assert!(d.repair.is_empty());
}
fn datum(value: &str) -> DataValue {
    DataValue::String(value.into())
}

#[test]
fn function_signatures_are_registered_before_bodies_and_duplicates_keep_first_site() {
    check_proposed(
        FILE,
        "fn Main() {\n    Later(1);\n}\nfn Later(x: view I64) {}\n",
        false,
    )
    .unwrap();
    let source = "fn F() {}\nfn F() {}\nfn Main() {}\n";
    let d = error(source, false);
    exact(
        &d,
        "NX-E0104",
        None,
        Stage::Names,
        Operation::NamesDeclare,
        byte_span(source, "F", 1),
        &[(
            RelatedRole::PreviousDeclaration,
            byte_span(source, "F", 0).0,
            byte_span(source, "F", 0).1,
        )],
        &[("declaration_kind", datum("function"))],
    );
    check_proposed(FILE, "fn F() {}\nfn G() {}\nfn Main() {}\n", false).unwrap();
}

#[test]
fn parameter_and_body_binding_share_initial_scope_but_child_block_shadows() {
    let source = "fn F(x: view I64, x: view I64) {}\nfn Main() {}\n";
    let d = error(source, false);
    let first = byte_span(source, "x:", 0).0;
    let second = byte_span(source, "x:", 1).0;
    exact(
        &d,
        "NX-E0104",
        None,
        Stage::Names,
        Operation::NamesDeclare,
        (second, second + 1),
        &[(RelatedRole::PreviousDeclaration, first, first + 1)],
        &[("declaration_kind", datum("parameter"))],
    );
    let source = "fn F(x: view I64) {\n    let x = 1;\n}\nfn Main() {}\n";
    let d = error(source, false);
    let parameter = byte_span(source, "x:", 0).0;
    let binding = byte_span(source, "x =", 0).0;
    exact(
        &d,
        "NX-E0104",
        None,
        Stage::Names,
        Operation::NamesDeclare,
        (binding, binding + 1),
        &[(RelatedRole::PreviousDeclaration, parameter, parameter + 1)],
        &[("declaration_kind", datum("binding"))],
    );
    check_proposed(
        FILE,
        "fn F(x: view I64) {\n    {\n        let x = 1;\n        AssertEq(x, 1);\n    }\n    AssertEq(x, 2);\n}\nfn Main() {}\n",
        false,
    )
    .unwrap();
    check_proposed(
        FILE,
        "fn F(x: view I64, y: view Bool) {}\nfn Main() {\n    F(1, true);\n}\n",
        false,
    )
    .unwrap();
}

#[test]
fn duplicate_binding_and_inner_shadowing_are_lexical() {
    let source = "fn Main() {\n    let x = 1;\n    let x = 2;\n}\n";
    let d = error(source, false);
    let duplicate = byte_span(source, "x =", 1).0;
    let first = byte_span(source, "x =", 0).0;
    exact(
        &d,
        "NX-E0104",
        None,
        Stage::Names,
        Operation::NamesDeclare,
        (duplicate, duplicate + 1),
        &[(RelatedRole::PreviousDeclaration, first, first + 1)],
        &[("declaration_kind", datum("binding"))],
    );
    check_proposed(
        FILE,
        "fn Main() {\n    let x = 1;\n    {\n        let x = true;\n        AssertEq(x, true);\n    }\n    AssertEq(x, 1);\n}\n",
        false,
    )
    .unwrap();
}

#[test]
fn absent_value_and_call_target_use_distinct_kinds() {
    let source = "fn Main() {\n    ausente();\n}\n";
    let d = error(source, false);
    exact(
        &d,
        "NX-E0101",
        None,
        Stage::Names,
        Operation::NamesResolve,
        byte_span(source, "ausente", 0),
        &[],
        &[("use_kind", datum("call_target"))],
    );
    let source = "fn Main() {\n    let x = missing;\n}\n";
    let d = error(source, false);
    exact(
        &d,
        "NX-E0101",
        None,
        Stage::Names,
        Operation::NamesResolve,
        byte_span(source, "missing", 0),
        &[],
        &[("use_kind", datum("value"))],
    );
    check_proposed(
        FILE,
        "fn Later() {}\nfn Main() {\n    Later();\n    let missing = 1;\n    let x = missing;\n}\n",
        false,
    )
    .unwrap();
}

#[test]
fn local_shadowing_blocks_calls_to_module_and_prelude_functions() {
    let source = "fn Main() {\n    let f = 1;\n    f();\n}\n";
    let d = error(source, false);
    let call = byte_span(source, "f()", 0).0;
    let declaration = byte_span(source, "f =", 0).0;
    exact(
        &d,
        "NX-E0111",
        Some("non_callable_target"),
        Stage::Arguments,
        Operation::CallBind,
        (call, call + 1),
        &[(RelatedRole::Declaration, declaration, declaration + 1)],
        &[],
    );
    let d = error(
        "fn F() {}\nfn Main() {\n    let F = 1;\n    F();\n}\n",
        false,
    );
    assert_eq!(
        (d.code.as_str(), d.subcause.as_deref(), d.stage),
        ("NX-E0111", Some("non_callable_target"), Stage::Arguments)
    );
    let d = error("fn Main() {\n    let Print = 1;\n    Print(2);\n}\n", false);
    assert_eq!(
        (d.code.as_str(), d.subcause.as_deref(), d.stage),
        ("NX-E0111", Some("non_callable_target"), Stage::Arguments)
    );
    check_proposed(
        FILE,
        "fn Print(x: view I64) -> I64 {\n    return x + 1;\n}\nfn Main() {\n    let n = Print(2);\n    AssertEq(n, 3);\n}\n",
        false,
    )
    .unwrap();
    check_proposed(
        FILE,
        "fn F(x: view I64) -> I64 {\n    return x;\n}\nfn Main() {\n    let n = ((F))(2);\n    AssertEq(n, 2);\n}\n",
        false,
    )
    .unwrap();
}

#[test]
fn structural_binding_checks_missing_excess_unknown_duplicate_and_order() {
    let cases = [
        (
            "fn F(x: view I64) {}\nfn Main() {\n    F();\n}\n",
            "missing_argument",
        ),
        (
            "fn F(x: view I64) {}\nfn Main() {\n    F(1, 2);\n}\n",
            "excess_positional_argument",
        ),
        (
            "fn F(x: view I64) {}\nfn Main() {\n    F(y: 1);\n}\n",
            "unknown_argument_label",
        ),
        (
            "fn F(x: view I64) {}\nfn Main() {\n    F(x: 1, x: 2);\n}\n",
            "duplicate_argument",
        ),
        (
            "fn F(x: view I64, y: view I64) {}\nfn Main() {\n    F(x: 1, 2);\n}\n",
            "positional_after_named",
        ),
    ];
    for (source, sub) in cases {
        let (primary, related) = match sub {
            "missing_argument" => {
                let target = byte_span(source, "F()", 0).0;
                let parameter = byte_span(source, "x:", 0).0;
                (
                    (target, target + 1),
                    vec![(RelatedRole::Declaration, parameter, parameter + 1)],
                )
            }
            "excess_positional_argument" => (byte_span(source, "2", 0), vec![]),
            "unknown_argument_label" => {
                let label = byte_span(source, "y:", 0).0;
                ((label, label + 1), vec![])
            }
            "duplicate_argument" => {
                let second = byte_span(source, "x:", 2).0;
                let first = byte_span(source, "x:", 1).0;
                (
                    (second, second + 1),
                    vec![(RelatedRole::Context, first, first + 1)],
                )
            }
            "positional_after_named" => {
                let label = byte_span(source, "x:", 1).0;
                (
                    byte_span(source, "2", 0),
                    vec![(RelatedRole::Context, label, label + 1)],
                )
            }
            _ => unreachable!(),
        };
        let d = error(source, false);
        exact(
            &d,
            "NX-E0111",
            Some(sub),
            Stage::Arguments,
            Operation::CallBind,
            primary,
            &related,
            &[],
        );
    }
    check_proposed(
        FILE,
        "fn F(x: view I64, y: view I64) {}\nfn Main() {\n    F(y: 2, x: 1);\n    F(1, 2);\n}\n",
        false,
    )
    .unwrap();
}

#[test]
fn mode_is_checked_during_left_to_right_association() {
    let source = "fn F(x: edit I64) {}\nfn Main() {\n    var x = 1;\n    F(x);\n}\n";
    let d = error(source, false);
    let argument = byte_span(source, "F(x)", 0).0 + 2;
    let parameter = byte_span(source, "x:", 0).0;
    exact(
        &d,
        "NX-E0111",
        Some("argument_mode_mismatch"),
        Stage::Arguments,
        Operation::CallBind,
        (argument, argument + 1),
        &[(RelatedRole::Declaration, parameter, parameter + 1)],
        &[],
    );
    check_proposed(
        FILE,
        "fn F(x: edit I64) {}\nfn Main() {\n    var x = 1;\n    F(edit x);\n}\n",
        false,
    )
    .unwrap();
    let d = error(
        "fn F(x: move I64) {}\nfn Main() {\n    let x = 1;\n    F(edit x);\n}\n",
        false,
    );
    assert_eq!(d.subcause.as_deref(), Some("argument_mode_mismatch"));
    check_proposed(
        FILE,
        "fn F(x: move I64) {}\nfn Main() {\n    let x = 1;\n    F(x);\n    F(move x);\n}\n",
        false,
    )
    .unwrap();
}

#[test]
fn written_return_and_empty_return_newline_use_expected_type() {
    let source = "fn F() -> I64 {\n    return true;\n}\nfn Main() {}\n";
    let d = error(source, false);
    let expected = byte_span(source, "I64", 0);
    let found = byte_span(source, "true", 0);
    exact(
        &d,
        "NX-E0102",
        Some("type_mismatch"),
        Stage::Types,
        Operation::TypesCheck,
        found,
        &[
            (RelatedRole::ExpectedType, expected.0, expected.1),
            (RelatedRole::FoundType, found.0, found.1),
        ],
        &[
            ("expected_type", datum("I64")),
            ("found_type", datum("Bool")),
        ],
    );
    check_proposed(
        FILE,
        "fn F() -> I64 {\n    return 1;\n}\nfn Main() {}\n",
        false,
    )
    .unwrap();
    let source = "fn F()->I64{\nreturn\n(1);\n}\nfn Main(){}";
    let d = error(source, false);
    let expected = byte_span(source, "I64", 0);
    let keyword = byte_span(source, "return", 0);
    let delimiter = (keyword.1, keyword.1 + 1);
    exact(
        &d,
        "NX-E0102",
        Some("type_mismatch"),
        Stage::Types,
        Operation::TypesCheck,
        keyword,
        &[
            (RelatedRole::ExpectedType, expected.0, expected.1),
            (RelatedRole::Delimiter, delimiter.0, delimiter.1),
        ],
        &[("expected_type", datum("I64")), ("found_type", datum("()"))],
    );
    check_proposed(
        FILE,
        "fn F() -> I64 {\n    return (1);\n}\nfn Main() {}\n",
        false,
    )
    .unwrap();
}

#[test]
fn empty_written_return_uses_keyword_and_terminating_newline_delimiter() {
    for source in [
        "fn F()->I64{\nreturn\n(1);\n}",
        "fn F()->I64{\r\nreturn\r\n(1);\r\n}",
        "fn F()->I64{\rreturn\r(1);\r}",
        "fn F()->I64{return;\n}",
        "fn F()->I64{\nreturn  \n(1);\n}",
        "fn F()->I64{\nreturn // c\n(1);\n}",
        "fn F()->I64{\r\nreturn // c\r\n(1);\r\n}",
        "fn F()->I64{\r\nreturn \t\r\n(1);\r\n}",
    ] {
        let keyword = byte_span(source, "return", 0);
        let after = &source[keyword.1 as usize..];
        let delimiter = if after.starts_with(';') {
            None
        } else {
            let relative = after
                .bytes()
                .position(|byte| byte == b'\r' || byte == b'\n')
                .expect("newline after empty return");
            let start = keyword.1 + relative as u64;
            let width = if source.as_bytes()[start as usize] == b'\r'
                && source.as_bytes().get(start as usize + 1) == Some(&b'\n')
            {
                2
            } else {
                1
            };
            Some((start, start + width))
        };
        let d = error(source, false);
        assert_eq!(d.primary_span, Some(sp(keyword.0, keyword.1)));
        assert_eq!(
            d.related_spans
                .iter()
                .find(|r| r.role == RelatedRole::Delimiter)
                .map(|r| (r.span.start, r.span.end)),
            delimiter
        );
        assert_eq!(d.data.get("expected_type"), Some(&datum("I64")));
        assert_eq!(d.data.get("found_type"), Some(&datum("()")));
    }
}

#[test]
fn annotation_closing_angles_exclude_assignment_equals() {
    for source in [
        "fn Main(){let x:Array<I64>=true;}",
        "fn Main(){let x:Result<I64,I64>=true;}",
        "fn Main(){let x:Array<Array<I64>>=true;}",
        "fn Main(){let x:Array<I64> =true;}",
        "fn Main(){let x:Array<I64> // c\n=true;}",
        "fn Main(){let x:Array<I64> // c\r\n=true;}",
    ] {
        let type_name = if source.contains("Result<I64,I64>") {
            "Result<I64,I64>"
        } else if source.contains("Array<Array<I64>>") {
            "Array<Array<I64>>"
        } else {
            "Array<I64>"
        };
        let expected = byte_span(source, type_name, 0);
        let d = error(source, false);
        assert_eq!(d.code, "NX-E0102", "{source}");
        assert_eq!(
            d.related_spans
                .iter()
                .find(|r| r.role == RelatedRole::ExpectedType)
                .map(|r| (r.span.start, r.span.end)),
            Some(expected),
            "{source}"
        );
    }
}

#[test]
fn registration_errors_keep_independent_body_errors_in_names_stage() {
    for source in [
        "fn F(a: view I64, a: view I64) {}\nfn Main() {\n    zz();\n}\n",
        "fn F() {}\nfn F() {}\nfn Main() {\n    zz();\n}\n",
    ] {
        let errors = check_proposed(FILE, source, false).unwrap_err();
        assert_eq!(errors.len(), 2, "{source}");
        assert_eq!(errors[0].code, "NX-E0104");
        assert_eq!(errors[1].code, "NX-E0101");
        assert!(errors.iter().all(|d| d.stage == Stage::Names));
    }
}

#[test]
fn expression_if_collects_independent_statement_type_errors() {
    for source in [
        "fn Main() {\n    let x = if true then {\n        let a: I64 = true;\n        value: 1;\n    } else {\n        let b: I64 = true;\n        value: 2;\n    };\n}\n",
        "fn Main() {\n    let x = if true then {\n        let a: I64 = true;\n        let b: I64 = true;\n        value: 1;\n    } else 2;\n}\n",
    ] {
        let errors = check_proposed(FILE, source, false).unwrap_err();
        assert_eq!(errors.len(), 2, "{source}");
        assert!(
            errors
                .iter()
                .all(|d| d.code == "NX-E0102" && d.stage == Stage::Types)
        );
    }
}

#[test]
fn expression_if_resets_taint_before_independent_branch_value() {
    let errors = check_proposed(
        FILE,
        "fn Main() {\n    let x = if true then {\n        let a: I64 = true;\n        value: false;\n    } else 1;\n}\n",
        false,
    )
    .unwrap_err();
    assert_eq!(errors.len(), 2);
    assert_eq!(errors[0].code, "NX-E0102");
    assert_eq!(errors[1].code, "NX-E1303");
}

#[test]
fn expression_if_checks_value_context_after_an_independent_binding_error() {
    let source = "fn Main() {\n    let x: I64 = if true then {\n        let a: I64 = true;\n        value: true;\n    } else 1;\n}\n";
    let errors = check_proposed(FILE, source, false).unwrap_err();
    assert_eq!(errors.len(), 2);
    assert!(errors.iter().all(|d| d.code == "NX-E0102"));
    let binding_true = byte_span(source, "true", 1);
    let value_true = byte_span(source, "true", 2);
    assert_eq!(
        errors[0].primary_span,
        Some(sp(binding_true.0, binding_true.1))
    );
    assert_eq!(errors[1].primary_span, Some(sp(value_true.0, value_true.1)));
    assert_eq!(errors[0].data.get("expected_type"), Some(&datum("I64")));
    assert_eq!(errors[1].data.get("expected_type"), Some(&datum("I64")));
}

#[test]
fn expression_if_restores_taint_from_outer_invalid_binding() {
    let source = "fn Main() {\n    let bad: I64 = true;\n    let x: I64 = if true then bad else 1;\n    let y: Bool = x;\n}\n";
    let errors = check_proposed(FILE, source, false).unwrap_err();
    assert_eq!(errors.len(), 1, "{errors:?}");
    let mismatch = byte_span(source, "true", 0);
    assert_eq!(errors[0].primary_span, Some(sp(mismatch.0, mismatch.1)));
}

#[test]
fn expression_if_keeps_independent_later_type_error_after_branch_binding_error() {
    let source = "fn Main() {\n    let x: I64 = if true then {\n        let a: I64 = true;\n        value: 1;\n    } else 1;\n    let y: Bool = x;\n}\n";
    let errors = check_proposed(FILE, source, false).unwrap_err();
    assert_eq!(errors.len(), 2);
    assert_eq!(errors[0].code, "NX-E0102");
    let binding_true = byte_span(source, "true", 1);
    let later_x = byte_span(source, "= x", 0).0 + 2;
    assert_eq!(
        errors[0].primary_span,
        Some(sp(binding_true.0, binding_true.1))
    );
    assert_eq!(errors[1].primary_span, Some(sp(later_x, later_x + 1)));
}

#[test]
fn branch_restores_dependency_taint_after_invalid_return_expression() {
    let errors = check_proposed(FILE,
        "fn F(c: view Bool) -> I64 {\n    let bad: I64 = true;\n    let x: I64 = if c then {\n        return bad + true;\n    } else 1;\n    let y: Bool = x;\n    return 1;\n}\n", false)
        .unwrap_err();
    assert_eq!(errors.len(), 1, "{errors:?}");
    assert_eq!(errors[0].code, "NX-E0102");
}

#[test]
fn expression_if_with_only_returning_branches_has_no_invented_unit_type() {
    for source in [
        "fn F(c: view Bool) -> I64 {\n    let x = if c then {\n        return 1;\n    } else {\n        return 2;\n    };\n    let y: I64 = x;\n    return y;\n}\n",
        "fn F(c: view Bool) -> I64 {\n    let x: I64 = if c then {\n        return 1;\n    } else {\n        return 2;\n    };\n    return x;\n}\n",
        "fn F(c: view Bool) -> I64 {\n    let x = if c then {\n        return 1;\n    } else 2;\n    return x;\n}\n",
        "fn F(c: view Bool) -> I64 {\n    let x = if c then 1 else {\n        return 2;\n    };\n    return x;\n}\n",
    ] {
        check_proposed(FILE, source, false).unwrap_or_else(|e| panic!("{source}: {e:?}"));
    }
}

#[test]
fn deferred_try_operator_causes_no_type_mismatch_in_this_slice() {
    check_proposed(FILE,
        "fn TryAdd(x: view I64, y: view I64) -> Result<I64, I64> {\n    return Ok(x + y);\n}\nfn F() -> I64 {\n    let x: I64 = TryAdd(1, 2)?;\n    return x;\n}\n", false).unwrap();
}

#[test]
fn adr_gap_try_result_type_is_deferred_until_result_semantics() {
    for source in [
        "fn F() -> Result<I64, I64> {\n    return Ok(1);\n}\nfn Main() {\n    let x: I64 = F()?;\n}\n",
        "fn F() -> Result<I64, I64> {\n    return Ok(1);\n}\nfn Main() {\n    let x: I64 = F()? + 1;\n}\n",
        "fn F() -> Result<I64, I64> {\n    return Ok(1);\n}\nfn Main() {\n    let x: I64 = 1 + F()?;\n}\n",
    ] {
        check_proposed(FILE, source, false).unwrap();
    }
}

#[test]
fn independent_call_arguments_are_reported_without_dependent_binding_cascades() {
    let source = "fn F(x: view I64, y: view I64) {}\nfn Main() {\n    F(true, true);\n}\n";
    let errors = check_proposed(FILE, source, false).unwrap_err();
    assert_eq!(errors.len(), 2);
    assert_eq!(errors[0].code, "NX-E0102");
    assert_eq!(errors[1].code, "NX-E0102");
    let source = "fn Main() {\n    zz(yy);\n}\n";
    let errors = check_proposed(FILE, source, false).unwrap_err();
    assert_eq!(errors.len(), 1);
    let target = byte_span(source, "zz", 0);
    assert_eq!(errors[0].primary_span, Some(sp(target.0, target.1)));
    let source = "fn Main() {\n    if bad {\n        bad2();\n    }\n}\n";
    let errors = check_proposed(FILE, source, false).unwrap_err();
    assert_eq!(errors.len(), 1);
    assert_eq!(errors[0].data.get("use_kind"), Some(&datum("value")));
    let source = "fn Main() {\n    let x: I64 = true;\n    let y: Bool = x;\n}\n";
    let errors = check_proposed(FILE, source, false).unwrap_err();
    assert_eq!(errors.len(), 1);
    let mismatch = byte_span(source, "true", 0);
    assert_eq!(errors[0].primary_span, Some(sp(mismatch.0, mismatch.1)));
}

#[test]
fn written_parameter_result_and_assignment_type_spans_exclude_trivia() {
    for source in [
        "fn F(x:view Array<I64>){} fn Main(){F(1);}",
        "fn F(x:view Array<I64> // c\n){} fn Main(){F(1);}",
    ] {
        let d = error(source, false);
        assert_eq!(
            d.related_spans
                .iter()
                .find(|r| r.role == RelatedRole::ExpectedType)
                .map(|r| (r.span.start, r.span.end)),
            Some(byte_span(source, "Array<I64>", 0))
        );
    }
    let source = "fn F()->Array<I64>{return true;}";
    let d = error(source, false);
    assert_eq!(
        d.related_spans
            .iter()
            .find(|r| r.role == RelatedRole::ExpectedType)
            .map(|r| (r.span.start, r.span.end)),
        Some(byte_span(source, "Array<I64>", 0))
    );
    let source = "fn Main(){var x:Array<I64>=[1];x=true;}";
    let d = error(source, false);
    assert_eq!(
        d.related_spans
            .iter()
            .find(|r| r.role == RelatedRole::ExpectedType)
            .map(|r| (r.span.start, r.span.end)),
        Some(byte_span(source, "Array<I64>", 0))
    );
}

#[test]
fn duplicate_function_body_is_not_resolved_but_other_functions_are() {
    let source = "fn F() {}\nfn F() {\n    skip();\n}\nfn Main() {\n    report();\n}\n";
    let errors = check_proposed(FILE, source, false).unwrap_err();
    assert_eq!(errors.len(), 2);
    assert_eq!(errors[0].code, "NX-E0104");
    assert_eq!(errors[1].code, "NX-E0101");
    let target = byte_span(source, "report", 0);
    assert_eq!(errors[1].primary_span, Some(sp(target.0, target.1)));
}

#[test]
fn returning_if_binding_can_flow_into_arithmetic_and_nested_arms() {
    for source in [
        "fn F(c: view Bool) -> I64 {\n    let x = if c then {\n        return 1;\n    } else {\n        return 2;\n    };\n    return x + 1;\n}\n",
        "fn F(c: view Bool) -> I64 {\n    let x = if c then (if c then {\n        return 1;\n    } else {\n        return 2;\n    }) else {\n        return 3;\n    };\n    return x + 1;\n}\n",
    ] {
        check_proposed(FILE, source, false).unwrap();
    }
}

#[test]
fn missing_arguments_follow_declaration_order_and_point_to_missing_parameter() {
    let source = "fn F(x: view I64, y: view I64) {}\nfn Main() {\n    F();\n}\n";
    let d = error(source, false);
    let target = byte_span(source, "F()", 0).0;
    let first = byte_span(source, "x:", 0).0;
    exact(
        &d,
        "NX-E0111",
        Some("missing_argument"),
        Stage::Arguments,
        Operation::CallBind,
        (target, target + 1),
        &[(RelatedRole::Declaration, first, first + 1)],
        &[],
    );
    let source = "fn F(x: view I64, y: view I64) {}\nfn Main() {\n    F(1);\n}\n";
    let d = error(source, false);
    let target = byte_span(source, "F(1)", 0).0;
    let second = byte_span(source, "y:", 0).0;
    exact(
        &d,
        "NX-E0111",
        Some("missing_argument"),
        Stage::Arguments,
        Operation::CallBind,
        (target, target + 1),
        &[(RelatedRole::Declaration, second, second + 1)],
        &[],
    );
}

#[test]
fn heterogeneous_array_uses_first_element_as_expected_type_origin() {
    let source = "fn Main() {\n    let x = [1, true];\n}\n";
    let d = error(source, false);
    let first = byte_span(source, "[1,", 0).0 + 1;
    let found = byte_span(source, "true", 0);
    exact(
        &d,
        "NX-E0102",
        Some("type_mismatch"),
        Stage::Types,
        Operation::TypesCheck,
        found,
        &[
            (RelatedRole::ExpectedType, first, first + 1),
            (RelatedRole::FoundType, found.0, found.1),
        ],
        &[
            ("expected_type", datum("I64")),
            ("found_type", datum("Bool")),
        ],
    );
    check_proposed(FILE, "fn Main() {\n    let x = [1, 2];\n}\n", false).unwrap();
}

#[test]
fn expression_if_condition_requires_bool() {
    let source = "fn Main() {\n    let x = if 1 then 2 else 3;\n}\n";
    let d = error(source, false);
    let condition = byte_span(source, "if 1", 0).0 + 3;
    exact(
        &d,
        "NX-E0102",
        Some("type_mismatch"),
        Stage::Types,
        Operation::TypesCheck,
        (condition, condition + 1),
        &[(RelatedRole::FoundType, condition, condition + 1)],
        &[
            ("expected_type", datum("Bool")),
            ("found_type", datum("I64")),
        ],
    );
    check_proposed(
        FILE,
        "fn Main() {\n    let x = if true then 2 else 3;\n}\n",
        false,
    )
    .unwrap();
}

#[test]
fn unit_function_accepts_empty_return_and_written_unit() {
    for source in [
        "fn Main() {\n    return;\n}\n",
        "fn Main() {\n    return ();\n}\n",
    ] {
        check_proposed(FILE, source, false).unwrap();
    }
}

#[test]
fn names_inside_statement_then_and_expression_else_if_are_resolved() {
    let source = "fn Main() {\n    if true {\n        zz();\n    }\n}\n";
    let d = error(source, false);
    exact(
        &d,
        "NX-E0101",
        None,
        Stage::Names,
        Operation::NamesResolve,
        byte_span(source, "zz", 0),
        &[],
        &[("use_kind", datum("call_target"))],
    );
    let source = "fn Main() {\n    let x = if true then 1 else (if true then {\n        zz();\n        value: 2;\n    } else 3);\n}\n";
    let d = error(source, false);
    exact(
        &d,
        "NX-E0101",
        None,
        Stage::Names,
        Operation::NamesResolve,
        byte_span(source, "zz", 0),
        &[],
        &[("use_kind", datum("call_target"))],
    );
}

#[test]
fn invalid_binding_suppresses_dependent_type_cascade() {
    let source = "fn Main() {\n    let a = 1 + true;\n    let b = a + 1;\n}\n";
    let errors = check_proposed(FILE, source, false).unwrap_err();
    assert_eq!(errors.len(), 1);
    let mismatch = byte_span(source, "true", 0);
    exact(
        &errors[0],
        "NX-E0102",
        Some("type_mismatch"),
        Stage::Types,
        Operation::TypesCheck,
        mismatch,
        &[(RelatedRole::FoundType, mismatch.0, mismatch.1)],
        &[
            ("expected_type", datum("I64")),
            ("found_type", datum("Bool")),
        ],
    );
}

#[test]
fn if_else_scope_does_not_erase_outer_binding() {
    let source = "fn Main() {\n    let x = 1;\n    if true {} else {}\n    let x = 2;\n}\n";
    let d = error(source, false);
    let first = byte_span(source, "x =", 0).0;
    let second = byte_span(source, "x =", 1).0;
    exact(
        &d,
        "NX-E0104",
        None,
        Stage::Names,
        Operation::NamesDeclare,
        (second, second + 1),
        &[(RelatedRole::PreviousDeclaration, first, first + 1)],
        &[("declaration_kind", datum("binding"))],
    );
    check_proposed(
        FILE,
        "fn Main() {\n    let x = 1;\n    if true {\n        let x = 2;\n    } else {\n        let x = 3;\n    }\n}\n",
        false,
    )
    .unwrap();
}

#[test]
fn written_type_spans_end_at_type_tokens_after_trivia() {
    for source in [
        "fn F(x:view Bool)->I64{return true;}",
        "fn F(x:view Bool)->I64\n{return true;}",
        "fn F(x:view Bool)->I64 // note\n{return true;}",
    ] {
        let d = error(source, false);
        assert_eq!(
            d.related_spans
                .iter()
                .find(|r| r.role == RelatedRole::ExpectedType)
                .map(|r| (r.span.start, r.span.end)),
            Some(byte_span(source, "I64", 0))
        );
    }
    for source in [
        "fn Main(){let a:\nI64=true;}",
        "fn Main(){let a: // note\nI64=true;}",
    ] {
        let d = error(source, false);
        let start = source.find("I64").unwrap() as u64;
        assert_eq!(
            d.related_spans
                .iter()
                .find(|r| r.role == RelatedRole::ExpectedType)
                .map(|r| (r.span.start, r.span.end)),
            Some((start, start + 3))
        );
    }
    for source in [
        "fn F(x:\nview Bool){} fn Main(){F(1);}",
        "fn F(x: // note\nview Bool){} fn Main(){F(1);}",
    ] {
        let d = error(source, false);
        let start = source.find("Bool").unwrap() as u64;
        assert_eq!(
            d.related_spans
                .iter()
                .find(|r| r.role == RelatedRole::ExpectedType)
                .map(|r| (r.span.start, r.span.end)),
            Some((start, start + 4))
        );
    }
}

#[test]
fn names_errors_precede_arguments_and_independent_sites_accumulate() {
    let source = "fn F(x: view I64) {}\nfn Main() {\n    F();\n    ausente();\n}\n";
    let ds = check_proposed(FILE, source, false).unwrap_err();
    assert_eq!(ds.len(), 1);
    assert_eq!(
        (&ds[0].code, ds[0].stage),
        (&"NX-E0101".to_owned(), Stage::Names)
    );
    let source = "fn F(x: view I64) {}\nfn Main() {\n    F();\n    let x = 1;\n    let x = 2;\n}\n";
    let ds = check_proposed(FILE, source, false).unwrap_err();
    assert_eq!(ds.len(), 1);
    assert_eq!(
        (&ds[0].code, ds[0].stage),
        (&"NX-E0104".to_owned(), Stage::Names)
    );
    let source = "fn Main() {\n    a1();\n    a2();\n    a3();\n}\n";
    let ds = check_proposed(FILE, source, false).unwrap_err();
    assert_eq!(ds.len(), 3);
    assert_eq!(
        ds.iter()
            .map(|d| d.primary_span.as_ref().unwrap().start)
            .collect::<Vec<_>>(),
        vec![
            byte_span(source, "a1", 0).0,
            byte_span(source, "a2", 0).0,
            byte_span(source, "a3", 0).0,
        ]
    );
    assert!(
        ds.iter()
            .all(|d| d.code == "NX-E0101" && d.stage == Stage::Names)
    );
    let source = "fn F(x: view I64) {}\nfn Main() {\n    F();\n    F();\n    F();\n}\n";
    let ds = check_proposed(FILE, source, false).unwrap_err();
    assert_eq!(ds.len(), 3);
    assert!(
        ds.iter()
            .all(|d| d.subcause.as_deref() == Some("missing_argument")
                && d.stage == Stage::Arguments)
    );
}

#[test]
fn mode_defect_precedes_missing_or_excess_arguments() {
    let source = "fn F(x: edit I64, y: view I64) {}\nfn Main() {\n    var a = 1;\n    F(a);\n}\n";
    assert_eq!(
        error(source, false).subcause.as_deref(),
        Some("argument_mode_mismatch")
    );
    let source =
        "fn F(x: edit I64, y: view I64) {}\nfn Main() {\n    var a = 1;\n    F(a, 1, 2);\n}\n";
    assert_eq!(
        error(source, false).subcause.as_deref(),
        Some("argument_mode_mismatch")
    );
}

#[test]
fn operator_span_ignores_matching_text_in_comments() {
    for (source, op) in [
        ("fn Main() {\n    let x = true < false;\n}\n", "<"),
        (
            "fn Main() {\n    let r = MockOpen(1);\n    let b = r == r;\n}\n",
            "==",
        ),
    ] {
        let d = error(source, false);
        let at = source.rfind(op).unwrap() as u64;
        assert_eq!(d.primary_span, Some(sp(at, at + op.len() as u64)));
    }
}

#[test]
fn len_uses_manifest_label_a() {
    check_proposed(FILE, "fn Main() {\n    let n = Len(a: [1]);\n}\n", false).unwrap();
    let d = error("fn Main() {\n    let n = Len(v: [1]);\n}\n", false);
    assert_eq!(d.subcause.as_deref(), Some("unknown_argument_label"));
}

#[test]
fn manifest_primitives_accept_only_their_written_labels_and_results() {
    let cases = [
        ("MockOpen(label:1)", "MockResource", "MockOpen(wrong:1)"),
        (
            "MockBlockingOpen(label:1)",
            "MockBlockingResource",
            "MockBlockingOpen(wrong:1)",
        ),
        ("Inspect(r:MockOpen(1))", "()", "Inspect(wrong:MockOpen(1))"),
        ("Drop(v:1)", "()", "Drop(wrong:1)"),
        ("Len(a:[1])", "I64", "Len(wrong:[1])"),
        ("TryAdd(a:1,b:2)", "Result<I64,I64>", "TryAdd(wrong:1,b:2)"),
        ("AssertEq(a:1,b:1)", "()", "AssertEq(wrong:1,b:1)"),
        ("Print(v:1)", "()", "Print(wrong:1)"),
        ("Fail()", "()", "Fail(wrong:1)"),
        (
            "MockReadRecords(r:MockOpen(1))",
            "Result<Array<Result<I64,I64>>,I64>",
            "MockReadRecords(wrong:MockOpen(1))",
        ),
        (
            "MockWriteI64(n:1)",
            "Result<(),I64>",
            "MockWriteI64(wrong:1)",
        ),
        ("MockRequest()", "Result<I64,I64>", "MockRequest(wrong:1)"),
        (
            "MockReply(status:1)",
            "Result<(),I64>",
            "MockReply(wrong:1)",
        ),
        ("MockUnknown()", "()", "MockUnknown(wrong:1)"),
    ];
    for (call, result, wrong) in cases {
        let positive = format!("fn Main(){{let x:{result}={call};}}");
        check_proposed(FILE, &positive, false)
            .unwrap_or_else(|d| panic!("{call}: {} {:?}", d[0].code, d[0].primary_span));
        let negative = format!("fn Main(){{{wrong};}}");
        let d = error(&negative, false);
        assert_eq!(
            d.subcause.as_deref(),
            Some("unknown_argument_label"),
            "{wrong}"
        );
    }
}

#[test]
fn a_returning_if_branch_does_not_contribute_a_value_type() {
    for source in [
        "fn F() -> I64 {\n    let x = if true then {\n        return 1;\n    } else 2;\n    return x;\n}\n",
        "fn F() -> I64 {\n    let x = if true then 2 else {\n        return 1;\n    };\n    return x;\n}\n",
        "fn F() -> I64 {\n    let x: I64 = if true then {\n        return 1;\n    } else 2;\n    return x;\n}\n",
        "fn F() -> I64 {\n    let x: I64 = if true then 2 else {\n        return 1;\n    };\n    return x;\n}\n",
        "fn F() -> I64 {\n    let x: I64 = if true then {\n        return 1;\n    } else {\n        return 2;\n    };\n    return 3;\n}\n",
        "fn F() -> I64 {\n    let x: I64 = if true then (if true then {\n        return 1;\n    } else {\n        return 2;\n    }) else 3;\n    return x;\n}\n",
        "fn F() -> I64 {\n    let x = if true then (if true then {\n        return 1;\n    } else {\n        return 2;\n    }) else 3;\n    return x;\n}\n",
    ] {
        check_proposed(FILE, source, false).unwrap();
    }
}

#[test]
fn branch_without_value_waits_for_flow_analysis() {
    check_proposed(
        FILE,
        "fn Main() {\n    let x = if true then {} else 2;\n    let y = x + 1;\n}\n",
        false,
    )
    .unwrap();
}

#[test]
fn else_if_type_union_uses_first_normal_continuation() {
    for source in [
        "fn Main(){let x=if true then {value:1;}else (if false then {value:true;}else{value:2;});}",
        "fn Main(){let x=if true then {value:1;}else (if false then {value:1;}else{value:true;});}",
    ] {
        let d = error(source, false);
        assert_eq!(d.code, "NX-E1303");
        assert_eq!(d.data.get("expected_type"), Some(&datum("I64")));
        assert_eq!(d.data.get("found_type"), Some(&datum("Bool")));
        let first = source.find("value:1;").unwrap() as u64 + 6;
        let bad = source.find("value:true;").unwrap() as u64 + 6;
        assert_eq!(d.primary_span, Some(sp(bad, bad + 4)));
        assert_eq!(
            d.related_spans
                .iter()
                .map(|r| (r.role, r.span.start, r.span.end))
                .collect::<Vec<_>>(),
            vec![
                (RelatedRole::ExpectedType, first, first + 1),
                (RelatedRole::FoundType, bad, bad + 4)
            ]
        );
    }
}

#[test]
fn positional_after_multiple_named_arguments_relates_first_label() {
    let source =
        "fn F(x: view I64, y: view I64, z: view I64) {}\nfn Main() {\n    F(x: 1, y: 2, 3);\n}\n";
    let d = error(source, false);
    assert_eq!(d.subcause.as_deref(), Some("positional_after_named"));
    assert_eq!(
        d.related_spans
            .iter()
            .find(|r| r.role == RelatedRole::Context)
            .map(|r| (r.span.start, r.span.end)),
        Some((
            byte_span(source, "F(x:", 1).0 + 2,
            byte_span(source, "F(x:", 1).0 + 3
        ))
    );
}

#[test]
fn independent_diagnostics_are_sorted_and_invalid_call_descendants_are_skipped() {
    let source = "fn A() {}\nfn A() {}\nfn B() {}\nfn B() {}\nfn Main() {}\n";
    let ds = check_proposed(FILE, source, false).unwrap_err();
    assert_eq!(ds.len(), 2);
    assert!(
        ds.iter()
            .all(|d| d.code == "NX-E0104" && d.stage == Stage::Names)
    );
    let source = "fn Main() {\n    let a: I64 = true;\n    let b: Bool = 1;\n    let c = a;\n}\n";
    let ds = check_proposed(FILE, source, false).unwrap_err();
    assert_eq!(ds.len(), 2);
    assert!(
        ds.iter()
            .all(|d| d.code == "NX-E0102" && d.stage == Stage::Types)
    );
    let source = "fn Main() {\n    let a = missing;\n    let b = absent;\n    let c = a;\n}\n";
    let ds = check_proposed(FILE, source, false).unwrap_err();
    assert_eq!(ds.len(), 2);
    assert!(
        ds.iter()
            .all(|d| d.code == "NX-E0101" && d.stage == Stage::Names)
    );
    let mut source = String::from("fn Main(){");
    for i in 0..8 {
        source.push_str(&format!("a{i}();"));
    }
    source.push('}');
    let ds = check_proposed(FILE, &source, false).unwrap_err();
    assert_eq!(ds.len(), 8);
    assert!(
        ds.windows(2)
            .all(|pair| pair[0].primary_span.as_ref().unwrap().start
                < pair[1].primary_span.as_ref().unwrap().start)
    );
    let source = "fn Main() {\n    missing(a1());\n    a2();\n}\n";
    let ds = check_proposed(FILE, source, false).unwrap_err();
    assert_eq!(ds.len(), 2);
    assert_eq!(
        ds.iter()
            .map(|d| d.primary_span.as_ref().unwrap().start)
            .collect::<Vec<_>>(),
        vec![
            byte_span(source, "missing", 0).0,
            byte_span(source, "a2", 0).0
        ]
    );
    let source = "fn F(x: view I64) {}\nfn Main() {\n    F(z: missing());\n    F();\n}\n";
    let ds = check_proposed(FILE, source, false).unwrap_err();
    assert_eq!(ds.len(), 1);
    assert_eq!(ds[0].code, "NX-E0101");
}

#[test]
fn main_signature_context_ends_at_last_signature_token() {
    let source = "fn Main(x: view I64) {}\n";
    let d = error(source, true);
    assert_eq!(d.subcause.as_deref(), Some("invalid_main_signature"));
    assert_eq!(
        d.related_spans
            .iter()
            .find(|r| r.role == RelatedRole::Context)
            .map(|r| (r.span.start, r.span.end)),
        Some((0, byte_span(source, ")", 0).1))
    );
}

#[test]
fn prelude_functions_cannot_be_read_as_values() {
    for source in [
        "fn Main() {\n    let a = Len;\n}\n",
        "fn Main() {\n    AssertEq(Print, ());\n}\n",
    ] {
        let d = error(source, false);
        assert_eq!(d.code, "NX-E0006");
        assert_eq!(d.feature_id.as_deref(), Some("function_value"));
        assert_eq!(d.stage, Stage::Names);
    }
}

#[test]
fn missing_argument_points_to_target_after_other_written_arguments() {
    let source = "fn F(x: view I64, y: view I64) {}\nfn Main() {\n    F(1);\n}\n";
    let d = error(source, false);
    let at = byte_span(source, "F(1)", 0).0;
    assert_eq!(d.subcause.as_deref(), Some("missing_argument"));
    assert_eq!(d.primary_span, Some(sp(at, at + 1)));
    assert_eq!(
        d.related_spans.iter().map(|r| r.role).collect::<Vec<_>>(),
        vec![RelatedRole::Declaration]
    );
}

#[test]
fn main_result_is_unit_or_result_unit_i64() {
    let d = error("fn Main() -> I64 {\n    return 1;\n}\n", true);
    assert_eq!(d.subcause.as_deref(), Some("invalid_main_signature"));
    assert_eq!(d.stage, Stage::Entry);
    check_proposed(
        FILE,
        "fn Main() -> Result<(), I64> {\n    return Ok(());\n}\n",
        true,
    )
    .unwrap();
}

#[test]
fn resource_primitive_domains_match_manifest() {
    check_proposed(
        FILE,
        "fn Main() {\n    let r = MockBlockingOpen(1);\n    Inspect(r);\n}\n",
        false,
    )
    .unwrap();
    let d = error("fn Main() {\n    Inspect(1);\n}\n", false);
    assert_eq!(d.subcause.as_deref(), Some("resource_required"));
    assert_eq!(
        d.data.get("expected_type"),
        Some(&datum("domain:mock_resource"))
    );
    let d = error(
        "fn Main() {\n    let r = MockBlockingOpen(1);\n    MockReadRecords(r);\n}\n",
        false,
    );
    assert_eq!(d.data.get("expected_type"), Some(&datum("MockResource")));
    assert_eq!(
        d.data.get("found_type"),
        Some(&datum("MockBlockingResource"))
    );
    let d = error(
        "fn Main() {\n    let r = MockOpen(1);\n    AssertEq(a: r, b: r);\n}\n",
        false,
    );
    assert_eq!(
        d.data.get("expected_type"),
        Some(&datum("domain:copyable_data"))
    );
    assert_eq!(d.data.get("found_type"), Some(&datum("MockResource")));
}

#[test]
fn conditions_require_bool_and_assignment_keeps_written_type_origin() {
    for source in [
        "fn Main() {\n    if 1 {}\n}\n",
        "fn Main() {\n    while 1 {}\n}\n",
    ] {
        let d = error(source, false);
        assert_eq!(d.data.get("expected_type"), Some(&datum("Bool")));
        assert_eq!(d.data.get("found_type"), Some(&datum("I64")));
    }
    let source = "fn Main() {\n    var x: I64 = 1;\n    x = true;\n}\n";
    let d = error(source, false);
    assert_eq!(d.data.get("expected_type"), Some(&datum("I64")));
    assert_eq!(
        d.related_spans
            .iter()
            .find(|r| r.role == RelatedRole::ExpectedType)
            .map(|r| (r.span.start, r.span.end)),
        Some(byte_span(source, "I64", 0))
    );
}

#[test]
fn mode_uses_associated_parameter_not_first_parameter() {
    let source =
        "fn F(x: view I64, y: edit I64) {}\nfn Main() {\n    var a = 1;\n    F(1, a);\n}\n";
    let d = error(source, false);
    let parameter = byte_span(source, "y:", 0).0;
    assert_eq!(d.subcause.as_deref(), Some("argument_mode_mismatch"));
    assert_eq!(
        d.related_spans
            .iter()
            .find(|r| r.role == RelatedRole::Declaration)
            .map(|r| (r.span.start, r.span.end)),
        Some((parameter, parameter + 1))
    );
    check_proposed(
        FILE,
        "fn F(x: view I64, y: edit I64) {}\nfn Main() {\n    var a = 1;\n    F(1, edit a);\n}\n",
        false,
    )
    .unwrap();
}

#[test]
fn adr_gap_array_element_mismatch_uses_element_primary_and_whole_annotation_context() {
    let source = "fn Main() {\n    let a: Array<I64> = [true];\n}\n";
    let d = error(source, false);
    let found = byte_span(source, "true", 0);
    let expected = byte_span(source, "Array<I64>", 0);
    assert_eq!(d.primary_span, Some(sp(found.0, found.1)));
    assert_eq!(
        d.related_spans
            .iter()
            .map(|r| (r.role, r.span.start, r.span.end))
            .collect::<Vec<_>>(),
        vec![
            (RelatedRole::ExpectedType, expected.0, expected.1),
            (RelatedRole::FoundType, found.0, found.1)
        ]
    );
}

#[test]
fn adr_gap_primitive_domain_primary_is_first_offending_argument() {
    let source = "fn Main() {\n    Len(a: 1);\n}\n";
    let d = error(source, false);
    let primary = byte_span(source, "1", 0);
    assert_eq!(d.primary_span, Some(sp(primary.0, primary.1)));
    assert_eq!(d.subcause.as_deref(), Some("not_indexable"));
    let source = "fn Main() {\n    AssertEq(a: MockOpen(1), b: MockOpen(2));\n}\n";
    let d = error(source, false);
    assert_eq!(
        d.primary_span
            .as_ref()
            .map(|s| &source[s.start as usize..s.end as usize]),
        Some("MockOpen(1)")
    );
    assert_eq!(
        d.data.get("expected_type"),
        Some(&datum("domain:copyable_data"))
    );
}

#[test]
fn adr_gap_inout_to_ordinary_parameter_uses_place_primary() {
    let source = "fn F(x: view I64) {}\nfn Main() {\n    var x = 1;\n    F(edit x);\n}\n";
    let d = error(source, false);
    let primary = byte_span(source, "x", 2);
    assert_eq!(d.primary_span, Some(sp(primary.0, primary.1)));
    assert_eq!(d.subcause.as_deref(), Some("argument_mode_mismatch"));
}

#[test]
fn adr_gap_non_name_call_target_uses_whole_target_span() {
    let source = "fn Main() {\n    1();\n}\n";
    let d = error(source, false);
    let primary = byte_span(source, "1", 0);
    assert_eq!(d.primary_span, Some(sp(primary.0, primary.1)));
    let source = "fn F(x: view I64) -> I64 {\n    return x;\n}\nfn Main() {\n    F(1)(2);\n}\n";
    let d = error(source, false);
    let primary = byte_span(source, "F(1)", 0);
    assert_eq!(d.primary_span, Some(sp(primary.0, primary.1)));
    assert_eq!(d.subcause.as_deref(), Some("non_callable_target"));
}

#[test]
fn adr_gap_result_constructor_payload_label_is_v() {
    check_proposed(
        FILE,
        "fn Main() {\n    let x: Result<I64, I64> = Ok(v: 1);\n    let y: Result<I64, I64> = Err(v: 1);\n}\n",
        false,
    )
    .unwrap();
    let source = "fn Main() {\n    let x: Result<I64, I64> = Ok(value: 1);\n}\n";
    let d = error(source, false);
    assert_eq!(d.subcause.as_deref(), Some("unknown_argument_label"));
    let primary = byte_span(source, "value", 0);
    assert_eq!(d.primary_span, Some(sp(primary.0, primary.1)));
}

#[test]
fn adr_gap_empty_array_without_array_context_reports_inference_error() {
    let source = "fn Main() {\n    let x: I64 = [];\n}\n";
    let d = error(source, false);
    assert_eq!(d.code, "NX-E0103");
    let primary = byte_span(source, "[]", 0);
    assert_eq!(d.primary_span, Some(sp(primary.0, primary.1)));
    let source = "fn Main() {\n    let a = [1];\n    let b = [] == a;\n}\n";
    let d = error(source, false);
    assert_eq!(d.code, "NX-E0103");
    let primary = byte_span(source, "[]", 0);
    assert_eq!(d.primary_span, Some(sp(primary.0, primary.1)));
}

#[test]
fn adr_gap_grouped_absent_target_keeps_call_target_use_kind() {
    let source = "fn Main() {\n    (ausente)();\n}\n";
    let d = error(source, false);
    let primary = byte_span(source, "ausente", 0);
    assert_eq!(d.primary_span, Some(sp(primary.0, primary.1)));
    assert_eq!(d.data.get("use_kind"), Some(&datum("call_target")));
}

#[test]
fn adr_gap_explicit_empty_return_uses_keyword_primary() {
    let source = "fn F() -> I64 {\n    return;\n}\n";
    let d = error(source, false);
    let primary = byte_span(source, "return", 0);
    let expected = byte_span(source, "I64", 0);
    assert_eq!(d.primary_span, Some(sp(primary.0, primary.1)));
    assert_eq!(
        d.related_spans
            .iter()
            .map(|r| (r.role, r.span.start, r.span.end))
            .collect::<Vec<_>>(),
        vec![(RelatedRole::ExpectedType, expected.0, expected.1)]
    );
}

#[test]
fn integer_boundaries_are_signed_i64() {
    check_proposed(
        FILE,
        "fn Main() {\n    let a = 9223372036854775807;\n    let b = -9223372036854775808;\n}\n",
        false,
    )
    .unwrap();
    for source in [
        "fn Main() {\n    let a = 9223372036854775808;\n}\n",
        "fn Main() {\n    let a = -9223372036854775809;\n}\n",
    ] {
        let d = error(source, false);
        assert_eq!(
            (d.code.as_str(), d.subcause.as_deref(), d.stage, d.operation),
            ("NX-E0107", None, Stage::Types, Operation::ValueLiteral)
        );
        assert_eq!(
            d.primary_span
                .as_ref()
                .map(|s| &source[s.start as usize..s.end as usize]),
            Some(if source.contains("-9223372036854775809") {
                "-9223372036854775809"
            } else {
                "9223372036854775808"
            })
        );
        assert!(d.data.is_empty());
    }
}

#[test]
fn comparisons_use_right_operand_for_mismatch_and_operator_for_domain() {
    let cases = [
        ("fn Main(){let b=true<false;}", "<", "I64", "Bool", "true"),
        ("fn Main(){let b=[1]<[2];}", "<", "I64", "Array<I64>", "[1]"),
        ("fn Main(){let b=1==true;}", "true", "I64", "Bool", "true"),
        (
            "fn Main() {\n    let r = MockOpen(1);\n    let b = r == r;\n}\n",
            "==",
            "domain:copyable_data",
            "MockResource",
            "r ==",
        ),
    ];
    for (source, primary_needle, expected, found, found_needle) in cases {
        let primary = byte_span(source, primary_needle, 0);
        let found_span = if found_needle == "r ==" {
            let start = byte_span(source, found_needle, 0).0;
            (start, start + 1)
        } else {
            byte_span(source, found_needle, 0)
        };
        let d = error(source, false);
        assert_eq!(
            (d.code.as_str(), d.subcause.as_deref(), d.stage, d.operation),
            (
                "NX-E0102",
                Some("type_mismatch"),
                Stage::Types,
                Operation::TypesCheck
            )
        );
        assert_eq!(d.primary_span, Some(sp(primary.0, primary.1)));
        assert_eq!(d.data.get("expected_type"), Some(&datum(expected)));
        assert_eq!(d.data.get("found_type"), Some(&datum(found)));
        assert!(
            d.related_spans
                .iter()
                .any(|s| s.role == RelatedRole::FoundType
                    && s.span == sp(found_span.0, found_span.1)),
            "{source}: {:?}",
            d.related_spans
        );
    }
    check_proposed(
        FILE,
        "fn Main() {\n    let a = [1];\n    let b = [2];\n    let x = a == b;\n    let y = 1 < 2;\n    let z = () == ();\n}\n",
        false,
    )
    .unwrap();
}

#[test]
fn published_comparison_diagnostics_have_exact_related_locations_and_data() {
    let cases = [
        (
            "fn Main(){let b=true<false;}",
            (20, 21),
            vec![(RelatedRole::FoundType, 16, 20)],
            "I64",
            "Bool",
        ),
        (
            "fn Main(){let b=[1]<[2];}",
            (19, 20),
            vec![(RelatedRole::FoundType, 16, 19)],
            "I64",
            "Array<I64>",
        ),
        (
            "fn Main(){let b=1==true;}",
            (19, 23),
            vec![
                (RelatedRole::ExpectedType, 16, 17),
                (RelatedRole::FoundType, 19, 23),
            ],
            "I64",
            "Bool",
        ),
        (
            "fn Main(){let r=MockOpen(1);let b=r==r;}",
            (35, 37),
            vec![(RelatedRole::FoundType, 34, 35)],
            "domain:copyable_data",
            "MockResource",
        ),
        (
            "fn Main(){let a:Result<I64,I64>=Ok(1);let b=a<a;}",
            (45, 46),
            vec![(RelatedRole::FoundType, 44, 45)],
            "I64",
            "Result<I64,I64>",
        ),
        (
            "fn Main(){let a:Result<MockResource,I64>=Ok(MockOpen(1));let b=a==a;}",
            (64, 66),
            vec![(RelatedRole::FoundType, 63, 64)],
            "domain:copyable_data",
            "Result<MockResource,I64>",
        ),
    ];
    for (source, primary, related, expected, found) in cases {
        let d = error(source, false);
        exact(
            &d,
            "NX-E0102",
            Some("type_mismatch"),
            Stage::Types,
            Operation::TypesCheck,
            primary,
            &related,
            &[
                ("expected_type", datum(expected)),
                ("found_type", datum(found)),
            ],
        );
    }
}

#[test]
fn resource_arrays_are_rejected_after_alias_resolution_and_in_written_types() {
    let source = "fn Main() {\n    let r = MockOpen(1);\n    let a = [r];\n}\n";
    let d = error(source, false);
    assert_eq!(
        (
            d.code.as_str(),
            d.feature_id.as_deref(),
            d.stage,
            d.operation
        ),
        (
            "NX-E0006",
            Some("resource_array"),
            Stage::Types,
            Operation::TypesCheck
        )
    );
    assert_eq!(d.data.get("pending_scope"), Some(&datum("later_adr")));
    let source = "fn F(a: view Array<MockResource>) {}\nfn Main() {}\n";
    let d = error(source, false);
    assert_eq!(
        (d.code.as_str(), d.feature_id.as_deref(), d.stage),
        ("NX-E0006", Some("resource_array"), Stage::Types)
    );
    let d = error(
        "fn Main() {\n    let r = MockOpen(1);\n    Drop([r]);\n}\n",
        false,
    );
    assert_eq!(
        (d.code.as_str(), d.feature_id.as_deref(), d.stage),
        ("NX-E0006", Some("resource_array"), Stage::Types)
    );
    check_proposed(FILE, "fn Main() {\n    let a: Array<I64> = [];\n}\n", false).unwrap();
}

#[test]
fn intrinsic_drop_rejects_projected_owner_but_user_drop_can_read_it() {
    let source = "fn Main() {\n    let a = [1];\n    Drop(a[0]);\n}\n";
    let d = error(source, false);
    let index = byte_span(source, "a[0]", 0);
    let target = byte_span(source, "Drop", 0);
    assert_eq!(
        (
            d.code.as_str(),
            d.feature_id.as_deref(),
            d.stage,
            d.operation,
            d.primary_span
        ),
        (
            "NX-E0006",
            Some("partial_transfer"),
            Stage::Arguments,
            Operation::CallBind,
            Some(sp(index.0, index.1))
        )
    );
    assert_eq!(
        d.related_spans
            .iter()
            .map(|r| (r.role, r.span.start, r.span.end))
            .collect::<Vec<_>>(),
        vec![(RelatedRole::Context, target.0, target.1)]
    );
    assert_eq!(d.data.get("pending_scope"), Some(&datum("later_adr")));
    check_proposed(
        FILE,
        "fn Main() {\n    let a = [1];\n    Drop(a);\n}\n",
        false,
    )
    .unwrap();
    check_proposed(
        FILE,
        "fn Drop(x: view I64) {}\nfn Main() {\n    let a = [1];\n    Drop(a[0]);\n}\n",
        false,
    )
    .unwrap();
    check_proposed(FILE, "fn Main() {\n    Drop([1][0]);\n}\n", false).unwrap();
}

#[test]
fn array_element_context_and_empty_array_inference_are_independent() {
    let source = "fn Main() {\n    let xs = [];\n}\n";
    let d = error(source, false);
    exact(
        &d,
        "NX-E0103",
        None,
        Stage::Types,
        Operation::TypesInfer,
        byte_span(source, "[]", 0),
        &[],
        &[],
    );
    check_proposed(
        FILE,
        "fn Main() {\n    let xs: Array<I64> = [];\n    let ys = [1, 2];\n}\n",
        false,
    )
    .unwrap();
    let d = error("fn Main() {\n    let xs = [1, true];\n}\n", false);
    assert_eq!(d.code, "NX-E0102");
    assert_eq!(d.data.get("expected_type"), Some(&datum("I64")));
    assert_eq!(d.data.get("found_type"), Some(&datum("Bool")));
}

#[test]
fn index_and_intrinsic_domains_are_typed_after_resolution() {
    let source = "fn Main() {\n    let x = 1[0];\n}\n";
    let d = error(source, false);
    let root = byte_span(source, "1[0]", 0).0;
    exact(
        &d,
        "NX-E0102",
        Some("not_indexable"),
        Stage::Types,
        Operation::TypesCheck,
        (root, root + 1),
        &[(RelatedRole::FoundType, root, root + 1)],
        &[
            ("expected_type", datum("domain:array")),
            ("found_type", datum("I64")),
        ],
    );
    let source = "fn Main() {\n    let x = 1;\n    Inspect(x);\n}\n";
    let d = error(source, false);
    let argument = byte_span(source, "Inspect(x)", 0).0 + "Inspect(".len() as u64;
    exact(
        &d,
        "NX-E0102",
        Some("resource_required"),
        Stage::Types,
        Operation::TypesCheck,
        (argument, argument + 1),
        &[(RelatedRole::FoundType, argument, argument + 1)],
        &[
            ("expected_type", datum("domain:mock_resource")),
            ("found_type", datum("I64")),
        ],
    );
    check_proposed(
        FILE,
        "fn Main() {\n    let xs = [1];\n    let x = xs[0];\n    let r = MockOpen(1);\n    Inspect(r);\n}\n",
        false,
    )
    .unwrap();
}

#[test]
fn entry_check_is_separate_from_module_check() {
    let source = "fn F() {}\n";
    check_proposed(FILE, source, false).unwrap();
    let d = error(source, true);
    let eof = source.len() as u64;
    exact(
        &d,
        "NX-E0112",
        Some("missing_main"),
        Stage::Entry,
        Operation::EntryCheck,
        (eof, eof),
        &[],
        &[("entry", datum("Main"))],
    );
    let source = "fn Main(x: view I64) {}\n";
    let d = error(source, true);
    exact(
        &d,
        "NX-E0112",
        Some("invalid_main_signature"),
        Stage::Entry,
        Operation::EntryCheck,
        byte_span(source, "Main", 0),
        &[(RelatedRole::Context, 0, byte_span(source, ")", 0).1)],
        &[("entry", datum("Main"))],
    );
    check_proposed(FILE, "fn Main() {}\n", true).unwrap();
}

#[test]
fn unicode_comment_and_crlf_preserve_absent_name_byte_spans() {
    for source in [
        "// á🙂\nfn Main(){\nlet x=missing;\n}",
        "// á🙂\r\nfn Main(){\r\nlet x=missing;\r\n}",
        "// á🙂\nfn Main(){\nlet x=missing;\n}\n",
    ] {
        let d = error(source, false);
        exact(
            &d,
            "NX-E0101",
            None,
            Stage::Names,
            Operation::NamesResolve,
            byte_span(source, "missing", 0),
            &[],
            &[("use_kind", datum("value"))],
        );
        let positive = source.replace("let x=missing;", "let missing=1;let x=missing;");
        check_proposed(FILE, &positive, false).unwrap();
    }
}

#[test]
fn all_preparsed_names_are_resolved_by_lexical_position() {
    let source = "fn F(x:view I64){{let f=1;let Option=2;let from=3;let value=4;AssertEq(f+Option+from+value,10);}AssertEq(x,1);} fn Main(){F(1);}";
    let ast = parse::parse(FILE, source).unwrap();
    let resolved = names::resolve(FILE, source, &ast).unwrap();
    assert_eq!(resolved.signatures.len(), 2);
    check_proposed(FILE, source, false).unwrap();
}

#[test]
fn written_argument_defects_precede_missing_slots() {
    let d = error(
        "fn F(x: edit I64, y: view I64) {}\nfn Main() {\n    var x = 1;\n    F(z: 1);\n}\n",
        false,
    );
    assert_eq!(d.subcause.as_deref(), Some("unknown_argument_label"));
    let d = error(
        "fn F(x: edit I64, y: view I64) {}\nfn Main() {\n    var x = 1;\n    F(x: 1, x: 2);\n}\n",
        false,
    );
    assert_eq!(d.subcause.as_deref(), Some("argument_mode_mismatch"));
    let d = error(
        "fn F(x: edit I64, y: view I64) {}\nfn Main() {\n    var x = 1;\n    F(x: 1, 2);\n}\n",
        false,
    );
    assert_eq!(d.subcause.as_deref(), Some("argument_mode_mismatch"));
    let source =
        "fn F(x: view I64, y: view I64, z: view I64) {}\nfn Main() {\n    F(x: 1, y: 2, 3);\n}\n";
    let d = error(source, false);
    assert_eq!(d.subcause.as_deref(), Some("positional_after_named"));
    assert_eq!(
        d.related_spans
            .iter()
            .find(|r| r.role == RelatedRole::Context)
            .map(|r| r.span.clone()),
        Some({
            let label = byte_span(source, "F(x:", 1).0 + 2;
            sp(label, label + 1)
        })
    );
}

#[test]
fn inner_scopes_restore_outer_types_and_callability() {
    check_proposed(FILE,"fn F(x:view I64)->I64{return x;} fn Main(){let f=1;{let f=true;AssertEq(f,true);}AssertEq(f,1);}",false).unwrap();
    check_proposed(FILE,"fn F(x:view I64)->I64{return x;} fn Main(){{let f=1;AssertEq(f,1);}let n=F(2);AssertEq(n,2);}",false).unwrap();
    check_proposed(
        FILE,
        "fn Main() {\n    let x = 1;\n    {\n        let x = true;\n        AssertEq(x, true);\n    }\n    let z = x + 1;\n    AssertEq(z, 2);\n}\n",
        false,
    )
    .unwrap();
}

#[test]
fn written_result_types_remain_whole_for_comparison_domains() {
    let source = "fn Main() {\n    let a: Result<I64, I64> = Ok(1);\n    let b = a < a;\n}\n";
    let d = error(source, false);
    let comparison = byte_span(source, "a < a", 0).0 + 2;
    assert_eq!(d.primary_span, Some(sp(comparison, comparison + 1)));
    assert_eq!(d.data.get("expected_type"), Some(&datum("I64")));
    assert_eq!(d.data.get("found_type"), Some(&datum("Result<I64,I64>")));
    let d = error(
        "fn Main() {\n    let a: Result<MockResource, I64> = Ok(MockOpen(1));\n    let b = a == a;\n}\n",
        false,
    );
    assert_eq!(
        d.data.get("expected_type"),
        Some(&datum("domain:copyable_data"))
    );
    assert_eq!(
        d.data.get("found_type"),
        Some(&datum("Result<MockResource,I64>"))
    );
}

#[test]
fn annotation_parameter_and_assignment_contexts_report_concrete_types() {
    let source = "fn Main() {\n    let x: Bool = 1;\n}\n";
    let d = error(source, false);
    let value = byte_span(source, "1", 0);
    let annotation = byte_span(source, "Bool", 0);
    exact(
        &d,
        "NX-E0102",
        Some("type_mismatch"),
        Stage::Types,
        Operation::TypesCheck,
        value,
        &[
            (RelatedRole::ExpectedType, annotation.0, annotation.1),
            (RelatedRole::FoundType, value.0, value.1),
        ],
        &[
            ("expected_type", datum("Bool")),
            ("found_type", datum("I64")),
        ],
    );
    let source = "fn F(x: view Bool) {}\nfn Main() {\n    F(1);\n}\n";
    let d = error(source, false);
    let value = byte_span(source, "F(1)", 0).0 + 2;
    let annotation = byte_span(source, "Bool", 0);
    exact(
        &d,
        "NX-E0102",
        Some("type_mismatch"),
        Stage::Types,
        Operation::TypesCheck,
        (value, value + 1),
        &[
            (RelatedRole::ExpectedType, annotation.0, annotation.1),
            (RelatedRole::FoundType, value, value + 1),
        ],
        &[
            ("expected_type", datum("Bool")),
            ("found_type", datum("I64")),
        ],
    );
    let d = error("fn Main() {\n    var x = 1;\n    x = true;\n}\n", false);
    assert_eq!(d.data.get("expected_type"), Some(&datum("I64")));
    assert_eq!(d.data.get("found_type"), Some(&datum("Bool")));
}

#[test]
fn unresolved_result_constructor_uses_whole_call_and_known_context() {
    let source = "fn Main() {\n    let r = Ok(1);\n}\n";
    let d = error(source, false);
    let call = byte_span(source, "Ok(1)", 0);
    assert_eq!(
        (
            d.code.as_str(),
            d.stage,
            d.operation,
            d.primary_span.as_ref()
        ),
        (
            "NX-E0113",
            Stage::Types,
            Operation::TypesInfer,
            Some(&sp(call.0, call.1))
        )
    );
    assert_eq!(
        d.data.get("unresolved_components"),
        Some(&DataValue::Array(vec![datum("$.err")]))
    );
    check_proposed(
        FILE,
        "fn Main() {\n    let r: Result<I64, I64> = Ok(1);\n}\n",
        false,
    )
    .unwrap();
}

#[test]
fn argument_labels_select_types_without_changing_written_diagnostic_order() {
    check_proposed(
        FILE,
        "fn F(x: view I64, y: view Bool) {}\nfn Main() {\n    F(y: true, x: 1);\n}\n",
        false,
    )
    .unwrap();
    let d = error(
        "fn F(x: view I64, y: view Bool) {}\nfn Main() {\n    F(y: 1, x: 1);\n}\n",
        false,
    );
    assert_eq!(d.data.get("expected_type"), Some(&datum("Bool")));
    assert_eq!(d.data.get("found_type"), Some(&datum("I64")));
    let d = error(
        "fn F(x: view I64, y: view Bool) {}\nfn Main() {\n    F(y: true, x: false);\n}\n",
        false,
    );
    assert_eq!(d.data.get("expected_type"), Some(&datum("I64")));
    assert_eq!(d.data.get("found_type"), Some(&datum("Bool")));
}

#[test]
fn both_result_variants_must_be_copyable_for_equality() {
    let d = error(
        "fn Main() {\n    let a: Result<I64, MockResource> = Err(MockOpen(1));\n    let b = a == a;\n}\n",
        false,
    );
    assert_eq!(
        d.data.get("expected_type"),
        Some(&datum("domain:copyable_data"))
    );
    assert_eq!(
        d.data.get("found_type"),
        Some(&datum("Result<I64,MockResource>"))
    );
}

#[test]
fn intrinsic_data_and_index_domains_reject_wrong_types() {
    let d = error(
        "fn Main() {\n    let r = MockOpen(1);\n    Print(r);\n}\n",
        false,
    );
    assert_eq!(
        d.data.get("expected_type"),
        Some(&datum("domain:copyable_data"))
    );
    let d = error("fn Main() {\n    let x = [1][true];\n}\n", false);
    assert_eq!(d.data.get("expected_type"), Some(&datum("I64")));
    assert_eq!(d.data.get("found_type"), Some(&datum("Bool")));
    let d = error("fn Main() {\n    AssertEq(1, true);\n}\n", false);
    assert_eq!(d.data.get("expected_type"), Some(&datum("I64")));
    assert_eq!(d.data.get("found_type"), Some(&datum("Bool")));
}

#[test]
fn operand_and_operator_spans_survive_trivia() {
    let source = "fn Main(){let b=true /*x*/ < false;}";
    // `/*...*/` is outside this lexer; use line breaks as legal trivia.
    let source = source.replace(" /*x*/ ", "\n ");
    let d = error(&source, false);
    let at = source.find('<').unwrap() as u64;
    assert_eq!(d.primary_span, Some(sp(at, at + 1)));
    assert_eq!(
        d.related_spans
            .iter()
            .find(|r| r.role == RelatedRole::FoundType)
            .map(|r| r.span.clone()),
        Some(sp(16, 20))
    );
}

#[test]
fn arithmetic_and_intrinsic_arguments_fail_in_written_order() {
    check_proposed(
        FILE,
        "fn Main() {\n    let x = 1 + 2;\n    let y = 3 - 1;\n}\n",
        false,
    )
    .unwrap();
    let source = "fn Main() {\n    let x = true + 9223372036854775808;\n}\n";
    let d = error(source, false);
    let mismatch = byte_span(source, "true", 0);
    assert_eq!(
        (&d.code, d.subcause.as_deref(), d.primary_span),
        (
            &"NX-E0102".to_owned(),
            Some("type_mismatch"),
            Some(sp(mismatch.0, mismatch.1))
        )
    );
    let d = error("fn Main() {\n    let x = 1 + true;\n}\n", false);
    assert_eq!(d.data.get("expected_type"), Some(&datum("I64")));
    assert_eq!(d.data.get("found_type"), Some(&datum("Bool")));
    let source = "fn Main() {\n    AssertEq(b: 9223372036854775808, a: 9223372036854775808);\n}\n";
    let d = error(source, false);
    assert_eq!(d.code, "NX-E0107");
    let first_argument = byte_span(source, "9223372036854775808", 0);
    assert_eq!(d.primary_span, Some(sp(first_argument.0, first_argument.1)));
}

#[test]
fn non_callable_expression_target_has_no_binding_declaration() {
    let d = error("fn Main() {\n    (1)();\n}\n", false);
    assert_eq!(
        (d.code.as_str(), d.subcause.as_deref(), d.stage, d.operation),
        (
            "NX-E0111",
            Some("non_callable_target"),
            Stage::Arguments,
            Operation::CallBind
        )
    );
    assert!(d.related_spans.is_empty());
}

#[test]
fn ordinary_parameters_reject_explicit_inout_mode() {
    let d = error(
        "fn F(x: view I64) {}\nfn Main() {\n    var x = 1;\n    F(edit x);\n}\n",
        false,
    );
    assert_eq!(d.subcause.as_deref(), Some("argument_mode_mismatch"));
    check_proposed(
        FILE,
        "fn F(x: view I64) {}\nfn Main() {\n    var x = 1;\n    F(x);\n}\n",
        false,
    )
    .unwrap();
}

#[test]
fn written_unit_annotation_checks_unit_literal_type() {
    check_proposed(
        FILE,
        "fn Main() {\n    let x: () = ();\n    AssertEq(x, ());\n}\n",
        false,
    )
    .unwrap();
    let d = error("fn Main() {\n    let x: () = 1;\n}\n", false);
    assert_eq!(d.data.get("expected_type"), Some(&datum("()")));
    assert_eq!(d.data.get("found_type"), Some(&datum("I64")));
}

#[test]
fn branch_values_synthesize_without_context_and_use_written_return_context() {
    let d = error(
        "fn Main() {\n    let x = if true then 1 else true;\n}\n",
        false,
    );
    assert_eq!(
        (d.code.as_str(), d.subcause.as_deref(), d.stage),
        ("NX-E1303", None, Stage::Types)
    );
    assert_eq!(d.data.get("expected_type"), Some(&datum("I64")));
    assert_eq!(d.data.get("found_type"), Some(&datum("Bool")));
    assert_eq!(
        d.related_spans
            .iter()
            .filter(|r| r.role == RelatedRole::FoundType)
            .count(),
        1
    );
    let d = error(
        "fn F() -> I64 {\n    return if true then 1 else true;\n}\nfn Main() {}\n",
        false,
    );
    assert_eq!(
        (d.code.as_str(), d.subcause.as_deref(), d.stage),
        ("NX-E0102", Some("type_mismatch"), Stage::Types)
    );
    assert_eq!(d.data.get("expected_type"), Some(&datum("I64")));
    assert_eq!(d.data.get("found_type"), Some(&datum("Bool")));
    check_proposed(
        FILE,
        "fn Main() {\n    let x = if true then 1 else 2;\n}\n",
        false,
    )
    .unwrap();
}

#[test]
fn adr_gap_duplicate_named_after_positional_relates_first_value() {
    let source = "fn F(x: view I64) {}\nfn Main() {\n    F(1, x: 2);\n}\n";
    let d = error(source, false);
    assert_eq!(d.subcause.as_deref(), Some("duplicate_argument"));
    assert_eq!(
        d.related_spans
            .iter()
            .map(|r| (r.role, r.span.start, r.span.end))
            .collect::<Vec<_>>(),
        vec![(
            RelatedRole::Context,
            byte_span(source, "1", 0).0,
            byte_span(source, "1", 0).1
        )]
    );
}

#[test]
fn contextual_move_name_and_reserved_async_have_distinct_results() {
    check_proposed(FILE, "fn Main(){let move=1;AssertEq(move,1);}", false).unwrap();
    let source = "fn Main(){let async=1;}";
    let d = error(source, false);
    assert_eq!(d.code, "NX-E0002");
    assert_eq!(d.stage, Stage::Parse);
    assert_eq!(d.operation, Operation::SyntaxParse);
    assert_eq!(d.parser_context.as_deref(), Some("binding_name"));
    assert_eq!(d.expected_tokens, ["Name"]);
    let (start, end) = byte_span(source, "async", 0);
    assert_eq!(d.primary_span, Some(sp(start, end)));
    assert_eq!(
        d.unexpected_token.as_ref().and_then(|t| t.text.as_deref()),
        Some("async")
    );
}

#[test]
fn valid_generated_names_shadowing_and_comparisons_have_no_diagnostics() {
    let names = [
        "Option", "from", "value", "Ok", "Result", "Print", "Drop", "MockOpen", "Inspect",
        "AssertEq", "Len", "x", "y", "z", "a", "b",
    ];
    for n in 0..8_000_u32 {
        let name = names[n as usize % names.len()];
        let value = if n % 2 == 0 {
            "1"
        } else {
            "9223372036854775807"
        };
        let source = format!(
            "fn F({name}:view I64)->I64{{return {name}+1;}} fn Main(){{let {name}={value};{{let {name}=true;let condition={name}==true;}}let outcome={name}<9223372036854775807;}} "
        );
        check_proposed(FILE, &source, false)
            .unwrap_or_else(|d| panic!("{n}: {source}: {} {:?}", d[0].code, d[0].primary_span));
    }
}

#[test]
fn generated_valid_mutations_return_valid_diagnostics_without_panics() {
    let atoms = [
        "fn",
        "main",
        "let",
        "var",
        "x",
        "y",
        "1",
        "true",
        "false",
        "[",
        "]",
        "(",
        ")",
        "{",
        "}",
        "=",
        ";",
        ":",
        ",",
        "+",
        "-",
        "<",
        "==",
        "move",
        "edit",
        "Ok",
        "Err",
        "\n",
        "\r\n",
        "// á🙂\n",
    ];
    let mut seed = 0x9e3779b97f4a7c15_u64;
    for i in 0..20_000 {
        seed ^= seed << 7;
        seed ^= seed >> 9;
        seed ^= seed << 8;
        let mut source = String::from("fn Main() {\n    let x = 1;\n    let y = x + 1;\n}\n");
        let count = (seed as usize % 6) + 1;
        for j in 0..count {
            let at = (seed.rotate_left((j * 7) as u32) as usize) % (source.len() + 1);
            let at = (at..=source.len())
                .find(|p| source.is_char_boundary(*p))
                .unwrap_or(source.len());
            source.insert_str(
                at,
                atoms[(seed.rotate_right((j * 5) as u32) as usize) % atoms.len()],
            );
        }
        let outcome = std::panic::catch_unwind(|| check_proposed(FILE, &source, i % 3 == 0));
        let outcome = outcome.unwrap_or_else(|_| panic!("panic at fuzz case {i}: {source}"));
        if let Err(ds) = outcome {
            for d in ds {
                d.validate(FILE, &source)
                    .unwrap_or_else(|e| panic!("case {i}: {source}: {e}"));
            }
        }
    }
}

#[test]
fn semantic_passes_run_on_a_512_kib_caller_stack() {
    let mut source = String::from("fn Main(){");
    for _ in 0..110 {
        source.push('{');
    }
    source.push_str("let x=1;AssertEq(x,1);");
    for _ in 0..110 {
        source.push('}');
    }
    source.push('}');
    std::thread::Builder::new()
        .stack_size(512 * 1024)
        .spawn(move || {
            check_proposed(FILE, &source, false).unwrap();
        })
        .unwrap()
        .join()
        .unwrap();
}

#[test]
fn semantic_passes_use_bounded_wide_stack_for_nested_forms() {
    fn source(family: usize, depth: usize) -> String {
        match family {
            0 => format!(
                "fn Main(){{let x={};}}",
                "(".repeat(depth) + "1" + &")".repeat(depth)
            ),
            1 => format!(
                "fn Main(){{let x={};}}",
                "[".repeat(depth) + "1" + &"]".repeat(depth)
            ),
            2 => format!(
                "fn Main(){{{}let x=1;{}}}",
                "{".repeat(depth),
                "}".repeat(depth)
            ),
            3 => format!(
                "fn Main(){{let x:{}I64{}={}1{};}}",
                "Array<".repeat(depth),
                ">".repeat(depth),
                "[".repeat(depth),
                "]".repeat(depth)
            ),
            4 => format!(
                "fn F(x:view I64)->I64{{return x;}} fn Main(){{let x={}1{};}}",
                "F(".repeat(depth),
                ")".repeat(depth)
            ),
            5 => format!("fn Main(){{let x=[1]{};}}", "[0]".repeat(depth)),
            6 => {
                let mut expression = "1".to_owned();
                for _ in 0..depth {
                    expression = format!("if true then {{value:1;}} else ({expression})");
                }
                format!("fn Main(){{let x={expression};}}")
            }
            7 => format!(
                "fn Main(){{{}let x=1;{}}}",
                "{let y=1;".repeat(depth),
                "}".repeat(depth)
            ),
            _ => unreachable!(),
        }
    }
    for stack in [256 * 1024, 512 * 1024] {
        for family in 0..8 {
            for depth in [1, 2, 4, 7, 8, 15, 16, 31, 32, 63, 126, 127] {
                let input = source(family, depth);
                std::thread::Builder::new()
                    .stack_size(stack)
                    .spawn(move || {
                        if let Err(errors) = check_proposed(FILE, &input, false) {
                            if depth < 126 {
                                assert_ne!(
                                    errors[0].stage,
                                    Stage::Parse,
                                    "family {family}, depth {depth}: {} {:?}",
                                    errors[0].code,
                                    errors[0].primary_span
                                );
                            }
                            for d in errors {
                                d.validate(FILE, &input).unwrap();
                            }
                        }
                    })
                    .unwrap()
                    .join()
                    .unwrap();
            }
        }
    }
}

#[test]
fn adr_gap_parser_depth_bounds_semantic_guard_for_accepted_blocks() {
    let source = format!(
        "fn Main(){{{}let x=1;{}}}",
        "{".repeat(126),
        "}".repeat(126)
    );
    check_proposed(FILE, &source, false).unwrap();
}

#[test]
fn inferred_type_depth_enforces_array_priority_and_result_value_limit() {
    let mut source = String::from("fn Main(){let a0=0;");
    for i in 1..=128 {
        source.push_str(&format!("let a{i}=[a{}];", i - 1));
    }
    source.push('}');
    check_proposed(FILE, &source, false).unwrap();
    let mut excessive = source[..source.len() - 1].to_owned();
    let start = excessive.len() + "let a129=".len();
    excessive.push_str("let a129=[a128];}");
    let d = error(&excessive, false);
    assert_eq!(
        (d.code.as_str(), d.stage, d.operation, d.primary_span),
        (
            "NX-E0003",
            Stage::Types,
            Operation::AnalysisLimit,
            Some(sp(start as u64, (start + 6) as u64))
        )
    );
    assert_eq!(d.data.get("budget_kind"), Some(&datum("array_type_depth")));
    assert_eq!(d.data.get("limit"), Some(&DataValue::UInt(128)));
    assert_eq!(d.data.get("observed"), Some(&DataValue::UInt(129)));

    let mut source = String::from("fn Main(){let a0:Result<I64,I64>=Ok(0);");
    for i in 1..=127 {
        source.push_str(&format!("let a{i}=[a{}];", i - 1));
    }
    source.push('}');
    check_proposed(FILE, &source, false).unwrap();
    let mut excessive = source[..source.len() - 1].to_owned();
    let start = excessive.len() + "let a128=".len();
    excessive.push_str("let a128=[a127];}");
    let d = error(&excessive, false);
    assert_eq!(
        (d.code.as_str(), d.stage, d.operation, d.primary_span),
        (
            "NX-E0003",
            Stage::Types,
            Operation::AnalysisLimit,
            Some(sp(start as u64, (start + 6) as u64))
        )
    );
    assert_eq!(d.data.get("budget_kind"), Some(&datum("value_type_depth")));
    assert_eq!(d.data.get("limit"), Some(&DataValue::UInt(128)));
    assert_eq!(d.data.get("observed"), Some(&DataValue::UInt(129)));
}

#[cfg(not(debug_assertions))]
#[test]
fn semantic_passes_scale_with_source_bytes() {
    fn source_for(family: usize, target: usize) -> String {
        let mut source = String::new();
        if family == 18 {
            let count = if target < 500_000 { 256 } else { 1023 };
            for i in 0..count {
                source.push_str(&format!("fn F{i}(){{let x=1;}}"));
            }
            source.push_str("fn Main() {}\n");
        } else if family == 19 {
            let count = target / 100;
            source.push_str("fn F(");
            for i in 0..count {
                if i > 0 {
                    source.push(',');
                }
                source.push_str(&format!("p{i}:view I64"));
            }
            source.push_str("){} fn Main(){F(");
            for i in 0..count {
                if i > 0 {
                    source.push(',');
                }
                source.push('1');
            }
            source.push_str(");}");
        } else {
            let prelude = if family == 7 || family == 8 || family == 23 {
                "fn F(x: view I64) {}\n"
            } else {
                ""
            };
            source.push_str(prelude);
            source.push_str("fn Main(){");
            let mut i = 0usize;
            loop {
                let segment = match family {
                    0 => format!("let n{i}=1;"),
                    1 => "x=1;".into(),
                    2 => format!("let n{i}:I64=1;"),
                    3 => format!("let n{i}:Bool=true;"),
                    4 => format!("let n{i}=[1,2];"),
                    5 => format!("let n{i}=[1][0];"),
                    6 => "AssertEq(1,1);".into(),
                    7 => "F(1);".into(),
                    8 => "F(x:1);".into(),
                    9 => format!("let n{i}=MockOpen(1);Inspect(n{i});"),
                    10 => format!("let n{i}:Result<I64,I64>=Ok(1);"),
                    11 => "{let x=1;AssertEq(x,1);}".into(),
                    12 => format!("let n{i}=1<2;"),
                    13 => format!("let n{i}=1==1;"),
                    14 => format!("let n{i}=true==false;"),
                    15 => format!("let n{i}=()==();"),
                    16 => format!("let n{i}=1+2-3;"),
                    17 => "{let Option=1;let from=Option+1;}".into(),
                    20 => "{".repeat(64) + "let x=1;AssertEq(x,1);" + &"}".repeat(64),
                    21 => "{".repeat(32) + "let x=1;" + &"}".repeat(32),
                    22 => {
                        let mut ty = "I64".to_owned();
                        let mut value = "1".to_owned();
                        for _ in 0..32 {
                            ty = format!("Array<{ty}>");
                            value = format!("[{value}]");
                        }
                        format!("let n{i}:{ty}={value};")
                    }
                    23 => "F(x:1);F(2);".into(),
                    _ => unreachable!(),
                };
                if family == 1 && i == 0 {
                    source.push_str("var x=0;");
                }
                if source.len() + segment.len() + 3 > target - 32 {
                    break;
                }
                source.push_str(&segment);
                i += 1;
            }
            source.push('}');
        }
        assert!(
            source.len() <= target - 2,
            "family {family} exceeded target"
        );
        source.push_str("//");
        source.push_str(&"x".repeat(target - source.len()));
        assert_eq!(source.len(), target);
        source
    }
    let full = std::env::var_os("NEXO_FULL_SCALE").is_some();
    let families: Vec<usize> = if full {
        (0..24).collect()
    } else {
        vec![0, 1, 2, 4, 6, 7, 8, 9, 10, 12, 16, 18, 19, 20, 22, 23]
    };
    for family in families {
        let mut observations = Vec::new();
        for size in [262_144, 1_048_576] {
            let source = source_for(family, size);
            crate::test_alloc::reset("semantic_scale_counter.nx");
            check_proposed("semantic_scale_counter.nx", &source, false).unwrap_or_else(|d| {
                panic!(
                    "family {family}, size {size}: {} {:?}",
                    d[0].code, d[0].primary_span
                )
            });
            observations.push(crate::test_alloc::snapshot("semantic_scale_counter.nx"));
        }
        println!("semantic family {family:02}: {observations:?}");
        for (small, large) in [
            (observations[0].0, observations[1].0),
            (observations[0].1, observations[1].1),
        ] {
            assert!(
                large <= small.saturating_mul(8),
                "superlinear family {family}: {observations:?}"
            );
        }
    }
}
#[test]
fn isolated_edit_is_a_parse_diagnostic_in_check() {
    for body in ["var b=edit r;", "r=edit r;", "[edit r];", "edit r;"] {
        let source = format!("fn Main(){{var r=1;{body}}}");
        let errors = super::check_proposed("edit.nx", &source, false).unwrap_err();
        assert_eq!(errors.len(), 1);
        let error = &errors[0];
        assert_eq!(error.code, "NX-E0002");
        assert_eq!(error.parser_context.as_deref(), Some("inout_place"));
        assert_eq!(
            error.primary_span.as_ref().unwrap().start as usize,
            source.find("edit r").unwrap()
        );
        assert!(error.data.is_empty());
        error.validate("edit.nx", &source).unwrap();
    }
    super::check_proposed(
        "edit.nx",
        "fn F(x:edit I64){} fn Main(){var r=1;F(edit r);}",
        false,
    )
    .unwrap();
}
