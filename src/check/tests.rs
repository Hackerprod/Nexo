use super::{check_proposed, names};
use crate::diagnostic::{
    DataValue, Diagnostic, Namespace, Operation, RelatedRole, Severity, Span, Stage,
};
use crate::frontend::parse;

const FILE: &str = "i03.nx";
fn sp(start: u64, end: u64) -> Span {
    Span {
        file: FILE.into(),
        start,
        end,
    }
}

fn error(source: &str, run_entry: bool) -> Box<Diagnostic> {
    let mut errors = check_proposed(FILE, source, run_entry).expect_err(source);
    assert!(!errors.is_empty());
    let d = errors.remove(0);
    d.validate(FILE, source).unwrap();
    d
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
    assert_eq!(d.profile_id, "f0-0002-v1");
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
    check_proposed(FILE, "fn main(){later(1);} fn later(x:i64){}", false).unwrap();
    let d = error("fn f(){} fn f(){} fn main(){}", false);
    exact(
        &d,
        "NX-E0104",
        None,
        Stage::Names,
        Operation::NamesDeclare,
        (12, 13),
        &[(RelatedRole::PreviousDeclaration, 3, 4)],
        &[("declaration_kind", datum("function"))],
    );
    check_proposed(FILE, "fn f(){} fn g(){} fn main(){}", false).unwrap();
}

#[test]
fn parameter_and_body_binding_share_initial_scope_but_child_block_shadows() {
    let d = error("fn f(x:i64,x:i64){} fn main(){}", false);
    exact(
        &d,
        "NX-E0104",
        None,
        Stage::Names,
        Operation::NamesDeclare,
        (11, 12),
        &[(RelatedRole::PreviousDeclaration, 5, 6)],
        &[("declaration_kind", datum("parameter"))],
    );
    let d = error("fn f(x:i64){let x=1;} fn main(){}", false);
    exact(
        &d,
        "NX-E0104",
        None,
        Stage::Names,
        Operation::NamesDeclare,
        (16, 17),
        &[(RelatedRole::PreviousDeclaration, 5, 6)],
        &[("declaration_kind", datum("binding"))],
    );
    check_proposed(
        FILE,
        "fn f(x:i64){{let x=1;assert_eq(x,1);}assert_eq(x,2);} fn main(){}",
        false,
    )
    .unwrap();
    check_proposed(FILE, "fn f(x:i64,y:bool){} fn main(){f(1,true);}", false).unwrap();
}

#[test]
fn duplicate_binding_and_inner_shadowing_are_lexical() {
    let d = error("fn main(){let x=1;let x=2;}", false);
    exact(
        &d,
        "NX-E0104",
        None,
        Stage::Names,
        Operation::NamesDeclare,
        (22, 23),
        &[(RelatedRole::PreviousDeclaration, 14, 15)],
        &[("declaration_kind", datum("binding"))],
    );
    check_proposed(
        FILE,
        "fn main(){let x=1;{let x=true;assert_eq(x,true);}assert_eq(x,1);}",
        false,
    )
    .unwrap();
}

#[test]
fn absent_value_and_call_target_use_distinct_kinds() {
    let d = error("fn main(){ausente();}", false);
    exact(
        &d,
        "NX-E0101",
        None,
        Stage::Names,
        Operation::NamesResolve,
        (10, 17),
        &[],
        &[("use_kind", datum("call_target"))],
    );
    let d = error("fn main(){let x=missing;}", false);
    exact(
        &d,
        "NX-E0101",
        None,
        Stage::Names,
        Operation::NamesResolve,
        (16, 23),
        &[],
        &[("use_kind", datum("value"))],
    );
    check_proposed(
        FILE,
        "fn later(){} fn main(){later();let missing=1;let x=missing;}",
        false,
    )
    .unwrap();
}

#[test]
fn local_shadowing_blocks_calls_to_module_and_prelude_functions() {
    let d = error("fn main(){let f=1;f();}", false);
    exact(
        &d,
        "NX-E0111",
        Some("non_callable_target"),
        Stage::Arguments,
        Operation::CallBind,
        (18, 19),
        &[(RelatedRole::Declaration, 14, 15)],
        &[],
    );
    let d = error("fn f(){} fn main(){let f=1;f();}", false);
    assert_eq!(
        (d.code.as_str(), d.subcause.as_deref(), d.stage),
        ("NX-E0111", Some("non_callable_target"), Stage::Arguments)
    );
    let d = error("fn main(){let print=1;print(2);}", false);
    assert_eq!(
        (d.code.as_str(), d.subcause.as_deref(), d.stage),
        ("NX-E0111", Some("non_callable_target"), Stage::Arguments)
    );
    check_proposed(
        FILE,
        "fn print(x:i64)->i64{return x+1;} fn main(){let n=print(2);assert_eq(n,3);}",
        false,
    )
    .unwrap();
    check_proposed(
        FILE,
        "fn f(x:i64)->i64{return x;} fn main(){let n=((f))(2);assert_eq(n,2);}",
        false,
    )
    .unwrap();
}

#[test]
fn structural_binding_checks_missing_excess_unknown_duplicate_and_order() {
    let cases = [
        (
            "fn f(x:i64){} fn main(){f();}",
            "missing_argument",
            (24, 25),
            vec![(RelatedRole::Declaration, 5, 6)],
        ),
        (
            "fn f(x:i64){} fn main(){f(1,2);}",
            "excess_positional_argument",
            (28, 29),
            vec![],
        ),
        (
            "fn f(x:i64){} fn main(){f(y:1);}",
            "unknown_argument_label",
            (26, 27),
            vec![],
        ),
        (
            "fn f(x:i64){} fn main(){f(x:1,x:2);}",
            "duplicate_argument",
            (30, 31),
            vec![(RelatedRole::Context, 26, 27)],
        ),
        (
            "fn f(x:i64,y:i64){} fn main(){f(x:1,2);}",
            "positional_after_named",
            (36, 37),
            vec![(RelatedRole::Context, 32, 33)],
        ),
    ];
    for (source, sub, primary, related) in cases {
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
        "fn f(x:i64,y:i64){} fn main(){f(y:2,x:1);f(1,2);}",
        false,
    )
    .unwrap();
}

#[test]
fn mode_is_checked_during_left_to_right_association() {
    let d = error("fn f(inout x:i64){} fn main(){var x=1;f(x);}", false);
    exact(
        &d,
        "NX-E0111",
        Some("argument_mode_mismatch"),
        Stage::Arguments,
        Operation::CallBind,
        (40, 41),
        &[(RelatedRole::Declaration, 11, 12)],
        &[],
    );
    check_proposed(
        FILE,
        "fn f(inout x:i64){} fn main(){var x=1;f(inout x);}",
        false,
    )
    .unwrap();
    let d = error(
        "fn f(consume x:i64){} fn main(){let x=1;f(inout x);}",
        false,
    );
    assert_eq!(d.subcause.as_deref(), Some("argument_mode_mismatch"));
    check_proposed(
        FILE,
        "fn f(consume x:i64){} fn main(){let x=1;f(x);f(consume x);}",
        false,
    )
    .unwrap();
}

#[test]
fn written_return_and_empty_return_newline_use_expected_type() {
    let d = error("fn f()->i64{return true;} fn main(){}", false);
    exact(
        &d,
        "NX-E0102",
        Some("type_mismatch"),
        Stage::Types,
        Operation::TypesCheck,
        (19, 23),
        &[
            (RelatedRole::ExpectedType, 8, 11),
            (RelatedRole::FoundType, 19, 23),
        ],
        &[
            ("expected_type", datum("i64")),
            ("found_type", datum("bool")),
        ],
    );
    check_proposed(FILE, "fn f()->i64{return 1;} fn main(){}", false).unwrap();
    let d = error("fn f()->i64{\nreturn\n(1);\n}\nfn main(){}", false);
    exact(
        &d,
        "NX-E0102",
        Some("type_mismatch"),
        Stage::Types,
        Operation::TypesCheck,
        (13, 19),
        &[
            (RelatedRole::ExpectedType, 8, 11),
            (RelatedRole::Delimiter, 19, 20),
        ],
        &[("expected_type", datum("i64")), ("found_type", datum("()"))],
    );
    check_proposed(FILE, "fn f()->i64{\nreturn (\n1);\n}\nfn main(){}", false).unwrap();
}

#[test]
fn empty_written_return_uses_keyword_and_terminating_newline_delimiter() {
    for (source, keyword, delimiter) in [
        ("fn f()->i64{\nreturn\n(1);\n}", (13, 19), Some((19, 20))),
        (
            "fn f()->i64{\r\nreturn\r\n(1);\r\n}",
            (14, 20),
            Some((20, 22)),
        ),
        ("fn f()->i64{\rreturn\r(1);\r}", (13, 19), Some((19, 20))),
        ("fn f()->i64{return;\n}", (12, 18), None),
        ("fn f()->i64{\nreturn  \n(1);\n}", (13, 19), Some((21, 22))),
        (
            "fn f()->i64{\nreturn // c\n(1);\n}",
            (13, 19),
            Some((24, 25)),
        ),
        (
            "fn f()->i64{\r\nreturn // c\r\n(1);\r\n}",
            (14, 20),
            Some((25, 27)),
        ),
        (
            "fn f()->i64{\r\nreturn \t\r\n(1);\r\n}",
            (14, 20),
            Some((22, 24)),
        ),
    ] {
        let d = error(source, false);
        assert_eq!(d.primary_span, Some(sp(keyword.0, keyword.1)));
        assert_eq!(
            d.related_spans
                .iter()
                .find(|r| r.role == RelatedRole::Delimiter)
                .map(|r| (r.span.start, r.span.end)),
            delimiter
        );
        assert_eq!(d.data.get("expected_type"), Some(&datum("i64")));
        assert_eq!(d.data.get("found_type"), Some(&datum("()")));
    }
}

#[test]
fn annotation_closing_angles_exclude_assignment_equals() {
    for (source, expected) in [
        ("fn main(){let x:Array<i64>=true;}", (16, 26)),
        ("fn main(){let x:Result<i64,i64>=true;}", (16, 31)),
        ("fn main(){let x:Array<Array<i64>>=true;}", (16, 33)),
        ("fn main(){let x:Array<i64> =true;}", (16, 26)),
        ("fn main(){let x:Array<i64> // c\n=true;}", (16, 26)),
        ("fn main(){let x:Array<i64> // c\r\n=true;}", (16, 26)),
    ] {
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
        "fn f(a:i64,a:i64){} fn main(){zz();}",
        "fn f(){} fn f(){} fn main(){zz();}",
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
        "fn main(){let x=if true{let a:i64=true;value:1;}else{let b:i64=true;value:2;};}",
        "fn main(){let x=if true{let a:i64=true;let b:i64=true;value:1;}else{value:2;};}",
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
        "fn main(){let x=if true{let a:i64=true;value:false;}else{value:1;};}",
        false,
    )
    .unwrap_err();
    assert_eq!(errors.len(), 2);
    assert_eq!(errors[0].code, "NX-E0102");
    assert_eq!(errors[1].code, "NX-E1303");
}

#[test]
fn expression_if_checks_value_context_after_an_independent_binding_error() {
    let errors = check_proposed(
        FILE,
        "fn main(){let x:i64=if true{let a:i64=true;value:true;}else{value:1;};}",
        false,
    )
    .unwrap_err();
    assert_eq!(errors.len(), 2);
    assert!(errors.iter().all(|d| d.code == "NX-E0102"));
    assert_eq!(errors[0].primary_span, Some(sp(38, 42)));
    assert_eq!(errors[1].primary_span, Some(sp(49, 53)));
    assert_eq!(errors[0].data.get("expected_type"), Some(&datum("i64")));
    assert_eq!(errors[1].data.get("expected_type"), Some(&datum("i64")));
}

#[test]
fn expression_if_restores_taint_from_outer_invalid_binding() {
    let errors = check_proposed(
        FILE,
        "fn main(){let bad:i64=true;let x:i64=if true{value:bad;}else{value:1;};let y:bool=x;}",
        false,
    )
    .unwrap_err();
    assert_eq!(errors.len(), 1, "{errors:?}");
    assert_eq!(errors[0].primary_span, Some(sp(22, 26)));
}

#[test]
fn expression_if_keeps_independent_later_type_error_after_branch_binding_error() {
    let errors = check_proposed(
        FILE,
        "fn main(){let x:i64=if true{let a:i64=true;value:1;}else{value:1;};let y:bool=x;}",
        false,
    )
    .unwrap_err();
    assert_eq!(errors.len(), 2);
    assert_eq!(errors[0].code, "NX-E0102");
    assert_eq!(errors[0].primary_span, Some(sp(38, 42)));
    assert_eq!(errors[1].primary_span, Some(sp(78, 79)));
}

#[test]
fn branch_restores_dependency_taint_after_invalid_return_expression() {
    let errors = check_proposed(FILE,
        "fn f(c:bool)->i64{let bad:i64=true;let x:i64=if c{return bad+true;}else{value:1;};let y:bool=x;return 1;}", false)
        .unwrap_err();
    assert_eq!(errors.len(), 1, "{errors:?}");
    assert_eq!(errors[0].code, "NX-E0102");
}

#[test]
fn expression_if_with_only_returning_branches_has_no_invented_unit_type() {
    for source in [
        "fn f(c:bool)->i64{let x=if c{return 1;}else{return 2;};let y:i64=x;return y;}",
        "fn f(c:bool)->i64{let x:i64=if c{return 1;}else{return 2;};return x;}",
        "fn f(c:bool)->i64{let x=if c{return 1;}else{value:2;};return x;}",
        "fn f(c:bool)->i64{let x=if c{value:1;}else{return 2;};return x;}",
    ] {
        check_proposed(FILE, source, false).unwrap_or_else(|e| panic!("{source}: {e:?}"));
    }
}

#[test]
fn deferred_try_operator_causes_no_type_mismatch_in_this_slice() {
    check_proposed(FILE,
        "fn try_add(x:i64,y:i64)->Result<i64,i64>{return Ok(x+y);} fn f()->i64{let x:i64=try_add(1,2)?;return x;}", false).unwrap();
}

#[test]
fn adr_gap_try_result_type_is_deferred_until_result_semantics() {
    for source in [
        "fn f()->Result<i64,i64>{return Ok(1);} fn main(){let x:i64=f()?;}",
        "fn f()->Result<i64,i64>{return Ok(1);} fn main(){let x:i64=f()?+1;}",
        "fn f()->Result<i64,i64>{return Ok(1);} fn main(){let x:i64=1+f()?;}",
    ] {
        check_proposed(FILE, source, false).unwrap();
    }
}

#[test]
fn independent_call_arguments_are_reported_without_dependent_binding_cascades() {
    let source = "fn f(x:i64,y:i64){} fn main(){f(true,true);}";
    let errors = check_proposed(FILE, source, false).unwrap_err();
    assert_eq!(errors.len(), 2);
    assert_eq!(errors[0].code, "NX-E0102");
    assert_eq!(errors[1].code, "NX-E0102");
    let source = "fn main(){zz(yy);}";
    let errors = check_proposed(FILE, source, false).unwrap_err();
    assert_eq!(errors.len(), 1);
    assert_eq!(errors[0].primary_span, Some(sp(10, 12)));
    let source = "fn main(){if bad {bad2();}}";
    let errors = check_proposed(FILE, source, false).unwrap_err();
    assert_eq!(errors.len(), 1);
    assert_eq!(errors[0].data.get("use_kind"), Some(&datum("value")));
    let source = "fn main(){let x:i64=true;let y:bool=x;}";
    let errors = check_proposed(FILE, source, false).unwrap_err();
    assert_eq!(errors.len(), 1);
    assert_eq!(errors[0].primary_span, Some(sp(20, 24)));
}

#[test]
fn written_parameter_result_and_assignment_type_spans_exclude_trivia() {
    for source in [
        "fn f(x:Array<i64>){} fn main(){f(1);}",
        "fn f(x:Array<i64> // c\n){} fn main(){f(1);}",
    ] {
        let d = error(source, false);
        assert_eq!(
            d.related_spans
                .iter()
                .find(|r| r.role == RelatedRole::ExpectedType)
                .map(|r| (r.span.start, r.span.end)),
            Some((7, 17))
        );
    }
    let d = error("fn f()->Array<i64>{return true;}", false);
    assert_eq!(
        d.related_spans
            .iter()
            .find(|r| r.role == RelatedRole::ExpectedType)
            .map(|r| (r.span.start, r.span.end)),
        Some((8, 18))
    );
    let d = error("fn main(){var x:Array<i64>=[1];x=true;}", false);
    assert_eq!(
        d.related_spans
            .iter()
            .find(|r| r.role == RelatedRole::ExpectedType)
            .map(|r| (r.span.start, r.span.end)),
        Some((16, 26))
    );
}

#[test]
fn duplicate_function_body_is_not_resolved_but_other_functions_are() {
    let source = "fn f(){} fn f(){skip();} fn main(){report();}";
    let errors = check_proposed(FILE, source, false).unwrap_err();
    assert_eq!(errors.len(), 2);
    assert_eq!(errors[0].code, "NX-E0104");
    assert_eq!(errors[1].code, "NX-E0101");
    assert_eq!(errors[1].primary_span, Some(sp(35, 41)));
}

#[test]
fn returning_if_binding_can_flow_into_arithmetic_and_nested_arms() {
    for source in [
        "fn f(c:bool)->i64{let x=if c{return 1;}else{return 2;};return x+1;}",
        "fn f(c:bool)->i64{let x=if c{value:if c{return 1;}else{return 2;};}else{return 3;};return x+1;}",
    ] {
        check_proposed(FILE, source, false).unwrap();
    }
}

#[test]
fn missing_arguments_follow_declaration_order_and_point_to_missing_parameter() {
    let d = error("fn f(x:i64,y:i64){} fn main(){f();}", false);
    exact(
        &d,
        "NX-E0111",
        Some("missing_argument"),
        Stage::Arguments,
        Operation::CallBind,
        (30, 31),
        &[(RelatedRole::Declaration, 5, 6)],
        &[],
    );
    let d = error("fn f(x:i64,y:i64){} fn main(){f(1);}", false);
    exact(
        &d,
        "NX-E0111",
        Some("missing_argument"),
        Stage::Arguments,
        Operation::CallBind,
        (30, 31),
        &[(RelatedRole::Declaration, 11, 12)],
        &[],
    );
}

#[test]
fn heterogeneous_array_uses_first_element_as_expected_type_origin() {
    let d = error("fn main(){let x=[1,true];}", false);
    exact(
        &d,
        "NX-E0102",
        Some("type_mismatch"),
        Stage::Types,
        Operation::TypesCheck,
        (19, 23),
        &[
            (RelatedRole::ExpectedType, 17, 18),
            (RelatedRole::FoundType, 19, 23),
        ],
        &[
            ("expected_type", datum("i64")),
            ("found_type", datum("bool")),
        ],
    );
    check_proposed(FILE, "fn main(){let x=[1,2];}", false).unwrap();
}

#[test]
fn expression_if_condition_requires_bool() {
    let d = error("fn main(){let x=if 1{value:2;}else{value:3;};}", false);
    exact(
        &d,
        "NX-E0102",
        Some("type_mismatch"),
        Stage::Types,
        Operation::TypesCheck,
        (19, 20),
        &[(RelatedRole::FoundType, 19, 20)],
        &[
            ("expected_type", datum("bool")),
            ("found_type", datum("i64")),
        ],
    );
    check_proposed(
        FILE,
        "fn main(){let x=if true{value:2;}else{value:3;};}",
        false,
    )
    .unwrap();
}

#[test]
fn unit_function_accepts_empty_return_and_written_unit() {
    for source in ["fn main(){return;}", "fn main(){return ();}"] {
        check_proposed(FILE, source, false).unwrap();
    }
}

#[test]
fn names_inside_statement_then_and_expression_else_if_are_resolved() {
    let d = error("fn main(){if true{zz();}}", false);
    exact(
        &d,
        "NX-E0101",
        None,
        Stage::Names,
        Operation::NamesResolve,
        (18, 20),
        &[],
        &[("use_kind", datum("call_target"))],
    );
    let d = error(
        "fn main(){let x=if true{value:1;}else if true{zz();value:2;}else{value:3;};}",
        false,
    );
    exact(
        &d,
        "NX-E0101",
        None,
        Stage::Names,
        Operation::NamesResolve,
        (46, 48),
        &[],
        &[("use_kind", datum("call_target"))],
    );
}

#[test]
fn invalid_binding_suppresses_dependent_type_cascade() {
    let errors = check_proposed(FILE, "fn main(){let a=1+true;let b=a+1;}", false).unwrap_err();
    assert_eq!(errors.len(), 1);
    exact(
        &errors[0],
        "NX-E0102",
        Some("type_mismatch"),
        Stage::Types,
        Operation::TypesCheck,
        (18, 22),
        &[(RelatedRole::FoundType, 18, 22)],
        &[
            ("expected_type", datum("i64")),
            ("found_type", datum("bool")),
        ],
    );
}

#[test]
fn if_else_scope_does_not_erase_outer_binding() {
    let d = error("fn main(){let x=1;if true{}else{}let x=2;}", false);
    exact(
        &d,
        "NX-E0104",
        None,
        Stage::Names,
        Operation::NamesDeclare,
        (37, 38),
        &[(RelatedRole::PreviousDeclaration, 14, 15)],
        &[("declaration_kind", datum("binding"))],
    );
    check_proposed(
        FILE,
        "fn main(){let x=1;if true{let x=2;}else{let x=3;}}",
        false,
    )
    .unwrap();
}

#[test]
fn written_type_spans_end_at_type_tokens_after_trivia() {
    for source in [
        "fn f(x:bool)->i64{return true;}",
        "fn f(x:bool)->i64\n{return true;}",
        "fn f(x:bool)->i64 // note\n{return true;}",
    ] {
        let d = error(source, false);
        assert_eq!(
            d.related_spans
                .iter()
                .find(|r| r.role == RelatedRole::ExpectedType)
                .map(|r| (r.span.start, r.span.end)),
            Some((14, 17))
        );
    }
    for source in [
        "fn main(){let a:\ni64=true;}",
        "fn main(){let a: // note\ni64=true;}",
    ] {
        let d = error(source, false);
        let start = source.find("i64").unwrap() as u64;
        assert_eq!(
            d.related_spans
                .iter()
                .find(|r| r.role == RelatedRole::ExpectedType)
                .map(|r| (r.span.start, r.span.end)),
            Some((start, start + 3))
        );
    }
    for source in [
        "fn f(x:\nbool){} fn main(){f(1);}",
        "fn f(x: // note\nbool){} fn main(){f(1);}",
    ] {
        let d = error(source, false);
        let start = source.find("bool").unwrap() as u64;
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
    let source = "fn f(x:i64){} fn main(){f();ausente();}";
    let ds = check_proposed(FILE, source, false).unwrap_err();
    assert_eq!(ds.len(), 1);
    assert_eq!(
        (&ds[0].code, ds[0].stage),
        (&"NX-E0101".to_owned(), Stage::Names)
    );
    let source = "fn f(x:i64){} fn main(){f();let x=1;let x=2;}";
    let ds = check_proposed(FILE, source, false).unwrap_err();
    assert_eq!(ds.len(), 1);
    assert_eq!(
        (&ds[0].code, ds[0].stage),
        (&"NX-E0104".to_owned(), Stage::Names)
    );
    let source = "fn main(){a1();a2();a3();}";
    let ds = check_proposed(FILE, source, false).unwrap_err();
    assert_eq!(ds.len(), 3);
    assert_eq!(
        ds.iter()
            .map(|d| d.primary_span.as_ref().unwrap().start)
            .collect::<Vec<_>>(),
        vec![10, 15, 20]
    );
    assert!(
        ds.iter()
            .all(|d| d.code == "NX-E0101" && d.stage == Stage::Names)
    );
    let source = "fn f(x:i64){} fn main(){f();f();f();}";
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
    let source = "fn f(inout x:i64,y:i64){} fn main(){var a=1;f(a);}";
    assert_eq!(
        error(source, false).subcause.as_deref(),
        Some("argument_mode_mismatch")
    );
    let source = "fn f(inout x:i64,y:i64){} fn main(){var a=1;f(a,1,2);}";
    assert_eq!(
        error(source, false).subcause.as_deref(),
        Some("argument_mode_mismatch")
    );
}

#[test]
fn operator_span_ignores_matching_text_in_comments() {
    for (source, op) in [
        ("fn main(){let x=true\n// <\n< false;}", "<"),
        ("fn main(){let r=mock_open(1);let b=r // ==\n==r;}", "=="),
    ] {
        let d = error(source, false);
        let at = source.rfind(op).unwrap() as u64;
        assert_eq!(d.primary_span, Some(sp(at, at + op.len() as u64)));
    }
}

#[test]
fn len_uses_manifest_label_a() {
    check_proposed(FILE, "fn main(){let n=len(a:[1]);}", false).unwrap();
    let d = error("fn main(){let n=len(v:[1]);}", false);
    assert_eq!(d.subcause.as_deref(), Some("unknown_argument_label"));
}

#[test]
fn manifest_primitives_accept_only_their_written_labels_and_results() {
    let cases = [
        ("mock_open(label:1)", "MockResource", "mock_open(wrong:1)"),
        (
            "mock_blocking_open(label:1)",
            "MockBlockingResource",
            "mock_blocking_open(wrong:1)",
        ),
        (
            "inspect(r:mock_open(1))",
            "()",
            "inspect(wrong:mock_open(1))",
        ),
        ("drop(v:1)", "()", "drop(wrong:1)"),
        ("len(a:[1])", "i64", "len(wrong:[1])"),
        (
            "try_add(a:1,b:2)",
            "Result<i64,i64>",
            "try_add(wrong:1,b:2)",
        ),
        ("assert_eq(a:1,b:1)", "()", "assert_eq(wrong:1,b:1)"),
        ("print(v:1)", "()", "print(wrong:1)"),
        ("fail()", "()", "fail(wrong:1)"),
        (
            "mock_read_records(r:mock_open(1))",
            "Result<Array<Result<i64,i64>>,i64>",
            "mock_read_records(wrong:mock_open(1))",
        ),
        (
            "mock_write_i64(n:1)",
            "Result<(),i64>",
            "mock_write_i64(wrong:1)",
        ),
        ("mock_request()", "Result<i64,i64>", "mock_request(wrong:1)"),
        (
            "mock_reply(status:1)",
            "Result<(),i64>",
            "mock_reply(wrong:1)",
        ),
        ("mock_unknown()", "()", "mock_unknown(wrong:1)"),
    ];
    for (call, result, wrong) in cases {
        let positive = format!("fn main(){{let x:{result}={call};}}");
        check_proposed(FILE, &positive, false)
            .unwrap_or_else(|d| panic!("{call}: {} {:?}", d[0].code, d[0].primary_span));
        let negative = format!("fn main(){{{wrong};}}");
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
        "fn f()->i64{let x=if true{return 1;}else{value:2;};return x;}",
        "fn f()->i64{let x=if true{value:2;}else{return 1;};return x;}",
        "fn f()->i64{let x:i64=if true{return 1;}else{value:2;};return x;}",
        "fn f()->i64{let x:i64=if true{value:2;}else{return 1;};return x;}",
        "fn f()->i64{let x:i64=if true{return 1;}else{return 2;};return 3;}",
        "fn f()->i64{let x:i64=if true{value:if true{return 1;}else{return 2;};}else{value:3;};return x;}",
        "fn f()->i64{let x=if true{value:if true{return 1;}else{return 2;};}else{value:3;};return x;}",
    ] {
        check_proposed(FILE, source, false).unwrap();
    }
}

#[test]
fn branch_without_value_waits_for_flow_analysis() {
    check_proposed(
        FILE,
        "fn main(){let x=if true{}else{value:2;};let y=x+1;}",
        false,
    )
    .unwrap();
}

#[test]
fn else_if_type_union_uses_first_normal_continuation() {
    for source in [
        "fn main(){let x=if true{value:1;}else if false{value:true;}else{value:2;};}",
        "fn main(){let x=if true{value:1;}else if false{value:1;}else{value:true;};}",
    ] {
        let d = error(source, false);
        assert_eq!(d.code, "NX-E1303");
        assert_eq!(d.data.get("expected_type"), Some(&datum("i64")));
        assert_eq!(d.data.get("found_type"), Some(&datum("bool")));
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
    let d = error("fn f(x:i64,y:i64,z:i64){} fn main(){f(x:1,y:2,3);}", false);
    assert_eq!(d.subcause.as_deref(), Some("positional_after_named"));
    assert_eq!(
        d.related_spans
            .iter()
            .find(|r| r.role == RelatedRole::Context)
            .map(|r| (r.span.start, r.span.end)),
        Some((38, 39))
    );
}

#[test]
fn independent_diagnostics_are_sorted_and_invalid_call_descendants_are_skipped() {
    let source = "fn a(){} fn a(){} fn b(){} fn b(){} fn main(){}";
    let ds = check_proposed(FILE, source, false).unwrap_err();
    assert_eq!(ds.len(), 2);
    assert!(
        ds.iter()
            .all(|d| d.code == "NX-E0104" && d.stage == Stage::Names)
    );
    let source = "fn main(){let a:i64=true;let b:bool=1;let c=a;}";
    let ds = check_proposed(FILE, source, false).unwrap_err();
    assert_eq!(ds.len(), 2);
    assert!(
        ds.iter()
            .all(|d| d.code == "NX-E0102" && d.stage == Stage::Types)
    );
    let source = "fn main(){let a=missing;let b=absent;let c=a;}";
    let ds = check_proposed(FILE, source, false).unwrap_err();
    assert_eq!(ds.len(), 2);
    assert!(
        ds.iter()
            .all(|d| d.code == "NX-E0101" && d.stage == Stage::Names)
    );
    let mut source = String::from("fn main(){");
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
    let source = "fn main(){missing(a1());a2();}";
    let ds = check_proposed(FILE, source, false).unwrap_err();
    assert_eq!(ds.len(), 2);
    assert_eq!(
        ds.iter()
            .map(|d| d.primary_span.as_ref().unwrap().start)
            .collect::<Vec<_>>(),
        vec![10, 24]
    );
    let source = "fn f(x:i64){} fn main(){f(z:missing());f();}";
    let ds = check_proposed(FILE, source, false).unwrap_err();
    assert_eq!(ds.len(), 1);
    assert_eq!(ds[0].code, "NX-E0101");
}

#[test]
fn main_signature_context_ends_at_last_signature_token() {
    let d = error("fn main(x:i64) // note\n {}", true);
    assert_eq!(d.subcause.as_deref(), Some("invalid_main_signature"));
    assert_eq!(
        d.related_spans
            .iter()
            .find(|r| r.role == RelatedRole::Context)
            .map(|r| (r.span.start, r.span.end)),
        Some((0, 14))
    );
}

#[test]
fn prelude_functions_cannot_be_read_as_values() {
    for source in ["fn main(){let a=len;}", "fn main(){assert_eq(print,());}"] {
        let d = error(source, false);
        assert_eq!(d.code, "NX-E0006");
        assert_eq!(d.feature_id.as_deref(), Some("function_value"));
        assert_eq!(d.stage, Stage::Names);
    }
}

#[test]
fn missing_argument_points_to_target_after_other_written_arguments() {
    let source = "fn f(x:i64,y:i64){} fn main(){f(1);}";
    let d = error(source, false);
    let at = source.rfind("f(1)").unwrap() as u64;
    assert_eq!(d.subcause.as_deref(), Some("missing_argument"));
    assert_eq!(d.primary_span, Some(sp(at, at + 1)));
    assert_eq!(
        d.related_spans.iter().map(|r| r.role).collect::<Vec<_>>(),
        vec![RelatedRole::Declaration]
    );
}

#[test]
fn main_result_is_unit_or_result_unit_i64() {
    let d = error("fn main()->i64{return 1;}", true);
    assert_eq!(d.subcause.as_deref(), Some("invalid_main_signature"));
    assert_eq!(d.stage, Stage::Entry);
    check_proposed(FILE, "fn main()->Result<(),i64>{return Ok(());}", true).unwrap();
}

#[test]
fn resource_primitive_domains_match_manifest() {
    check_proposed(
        FILE,
        "fn main(){let r=mock_blocking_open(1);inspect(r);}",
        false,
    )
    .unwrap();
    let d = error("fn main(){inspect(1);}", false);
    assert_eq!(d.subcause.as_deref(), Some("resource_required"));
    assert_eq!(
        d.data.get("expected_type"),
        Some(&datum("domain:mock_resource"))
    );
    let d = error(
        "fn main(){let r=mock_blocking_open(1);mock_read_records(r);}",
        false,
    );
    assert_eq!(d.data.get("expected_type"), Some(&datum("MockResource")));
    assert_eq!(
        d.data.get("found_type"),
        Some(&datum("MockBlockingResource"))
    );
    let d = error("fn main(){let r=mock_open(1);assert_eq(a:r,b:r);}", false);
    assert_eq!(
        d.data.get("expected_type"),
        Some(&datum("domain:copyable_data"))
    );
    assert_eq!(d.data.get("found_type"), Some(&datum("MockResource")));
}

#[test]
fn conditions_require_bool_and_assignment_keeps_written_type_origin() {
    for source in ["fn main(){if 1{}}", "fn main(){while 1{}}"] {
        let d = error(source, false);
        assert_eq!(d.data.get("expected_type"), Some(&datum("bool")));
        assert_eq!(d.data.get("found_type"), Some(&datum("i64")));
    }
    let d = error("fn main(){var x:i64=1;x=true;}", false);
    assert_eq!(d.data.get("expected_type"), Some(&datum("i64")));
    assert_eq!(
        d.related_spans
            .iter()
            .find(|r| r.role == RelatedRole::ExpectedType)
            .map(|r| (r.span.start, r.span.end)),
        Some((16, 19))
    );
}

#[test]
fn mode_uses_associated_parameter_not_first_parameter() {
    let source = "fn f(x:i64,inout y:i64){} fn main(){var a=1;f(1,a);}";
    let d = error(source, false);
    assert_eq!(d.subcause.as_deref(), Some("argument_mode_mismatch"));
    assert_eq!(
        d.related_spans
            .iter()
            .find(|r| r.role == RelatedRole::Declaration)
            .map(|r| (r.span.start, r.span.end)),
        Some((17, 18))
    );
    check_proposed(
        FILE,
        "fn f(x:i64,inout y:i64){} fn main(){var a=1;f(1,inout a);}",
        false,
    )
    .unwrap();
}

#[test]
fn adr_gap_array_element_mismatch_uses_element_primary_and_whole_annotation_context() {
    let d = error("fn main(){let a:Array<i64>=[true];}", false);
    assert_eq!(d.primary_span, Some(sp(28, 32)));
    assert_eq!(
        d.related_spans
            .iter()
            .map(|r| (r.role, r.span.start, r.span.end))
            .collect::<Vec<_>>(),
        vec![
            (RelatedRole::ExpectedType, 16, 26),
            (RelatedRole::FoundType, 28, 32)
        ]
    );
}

#[test]
fn adr_gap_primitive_domain_primary_is_first_offending_argument() {
    let d = error("fn main(){len(a:1);}", false);
    assert_eq!(d.primary_span, Some(sp(16, 17)));
    assert_eq!(d.subcause.as_deref(), Some("not_indexable"));
    let source = "fn main(){assert_eq(a:mock_open(1),b:mock_open(2));}";
    let d = error(source, false);
    assert_eq!(
        d.primary_span
            .as_ref()
            .map(|s| &source[s.start as usize..s.end as usize]),
        Some("mock_open(1)")
    );
    assert_eq!(
        d.data.get("expected_type"),
        Some(&datum("domain:copyable_data"))
    );
}

#[test]
fn adr_gap_inout_to_ordinary_parameter_uses_place_primary() {
    let d = error("fn f(x:i64){} fn main(){var x=1;f(inout x);}", false);
    assert_eq!(d.primary_span, Some(sp(40, 41)));
    assert_eq!(d.subcause.as_deref(), Some("argument_mode_mismatch"));
}

#[test]
fn adr_gap_non_name_call_target_uses_whole_target_span() {
    let d = error("fn main(){1();}", false);
    assert_eq!(d.primary_span, Some(sp(10, 11)));
    let d = error("fn f(x:i64)->i64{return x;} fn main(){f(1)(2);}", false);
    assert_eq!(d.primary_span, Some(sp(38, 42)));
    assert_eq!(d.subcause.as_deref(), Some("non_callable_target"));
}

#[test]
fn adr_gap_result_constructor_payload_label_is_v() {
    check_proposed(
        FILE,
        "fn main(){let x:Result<i64,i64>=Ok(v:1);let y:Result<i64,i64>=Err(v:1);}",
        false,
    )
    .unwrap();
    let d = error("fn main(){let x:Result<i64,i64>=Ok(value:1);}", false);
    assert_eq!(d.subcause.as_deref(), Some("unknown_argument_label"));
    assert_eq!(d.primary_span, Some(sp(35, 40)));
}

#[test]
fn adr_gap_empty_array_without_array_context_reports_inference_error() {
    let d = error("fn main(){let x:i64=[];}", false);
    assert_eq!(d.code, "NX-E0103");
    assert_eq!(d.primary_span, Some(sp(20, 22)));
    let d = error("fn main(){let a=[1];let b=[]==a;}", false);
    assert_eq!(d.code, "NX-E0103");
    assert_eq!(d.primary_span, Some(sp(26, 28)));
}

#[test]
fn adr_gap_grouped_absent_target_keeps_call_target_use_kind() {
    let d = error("fn main(){(ausente)();}", false);
    assert_eq!(d.primary_span, Some(sp(11, 18)));
    assert_eq!(d.data.get("use_kind"), Some(&datum("call_target")));
}

#[test]
fn adr_gap_explicit_empty_return_uses_keyword_primary() {
    let d = error("fn f()->i64{return;}", false);
    assert_eq!(d.primary_span, Some(sp(12, 18)));
    assert_eq!(
        d.related_spans
            .iter()
            .map(|r| (r.role, r.span.start, r.span.end))
            .collect::<Vec<_>>(),
        vec![(RelatedRole::ExpectedType, 8, 11)]
    );
}

#[test]
fn integer_boundaries_are_signed_i64() {
    check_proposed(
        FILE,
        "fn main(){let a=9223372036854775807;let b=-9223372036854775808;}",
        false,
    )
    .unwrap();
    for source in [
        "fn main(){let a=9223372036854775808;}",
        "fn main(){let a=-9223372036854775809;}",
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
            Some(if source.contains("=-") {
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
        (
            "fn main(){let b=true<false;}",
            (20, 21),
            "i64",
            "bool",
            (16, 20),
        ),
        (
            "fn main(){let b=[1]<[2];}",
            (19, 20),
            "i64",
            "Array<i64>",
            (16, 19),
        ),
        (
            "fn main(){let b=1==true;}",
            (19, 23),
            "i64",
            "bool",
            (19, 23),
        ),
        (
            "fn main(){let r=mock_open(1);let b=r==r;}",
            (36, 38),
            "domain:copyable_data",
            "MockResource",
            (35, 36),
        ),
    ];
    for (source, primary, expected, found, found_span) in cases {
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
        "fn main(){let a=[1];let b=[2];let x=a==b;let y=1<2;let z=()==();}",
        false,
    )
    .unwrap();
}

#[test]
fn published_comparison_diagnostics_have_exact_related_locations_and_data() {
    let cases = [
        (
            "fn main(){let b=true<false;}",
            (20, 21),
            vec![(RelatedRole::FoundType, 16, 20)],
            "i64",
            "bool",
        ),
        (
            "fn main(){let b=[1]<[2];}",
            (19, 20),
            vec![(RelatedRole::FoundType, 16, 19)],
            "i64",
            "Array<i64>",
        ),
        (
            "fn main(){let b=1==true;}",
            (19, 23),
            vec![
                (RelatedRole::ExpectedType, 16, 17),
                (RelatedRole::FoundType, 19, 23),
            ],
            "i64",
            "bool",
        ),
        (
            "fn main(){let r=mock_open(1);let b=r==r;}",
            (36, 38),
            vec![(RelatedRole::FoundType, 35, 36)],
            "domain:copyable_data",
            "MockResource",
        ),
        (
            "fn main(){let a:Result<i64,i64>=Ok(1);let b=a<a;}",
            (45, 46),
            vec![(RelatedRole::FoundType, 44, 45)],
            "i64",
            "Result<i64,i64>",
        ),
        (
            "fn main(){let a:Result<MockResource,i64>=Ok(mock_open(1));let b=a==a;}",
            (65, 67),
            vec![(RelatedRole::FoundType, 64, 65)],
            "domain:copyable_data",
            "Result<MockResource,i64>",
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
    let source = "fn main(){let r=mock_open(1);let a=[r];}";
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
    let source = "fn f(a:Array<MockResource>){} fn main(){}";
    let d = error(source, false);
    assert_eq!(
        (d.code.as_str(), d.feature_id.as_deref(), d.stage),
        ("NX-E0006", Some("resource_array"), Stage::Types)
    );
    let d = error("fn main(){let r=mock_open(1);drop([r]);}", false);
    assert_eq!(
        (d.code.as_str(), d.feature_id.as_deref(), d.stage),
        ("NX-E0006", Some("resource_array"), Stage::Types)
    );
    check_proposed(FILE, "fn main(){let a:Array<i64>=[];}", false).unwrap();
}

#[test]
fn intrinsic_drop_rejects_projected_owner_but_user_drop_can_read_it() {
    let source = "fn main(){let a=[1];drop(a[0]);}";
    let d = error(source, false);
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
            Some(sp(25, 29))
        )
    );
    assert_eq!(
        d.related_spans
            .iter()
            .map(|r| (r.role, r.span.start, r.span.end))
            .collect::<Vec<_>>(),
        vec![(RelatedRole::Context, 20, 24)]
    );
    assert_eq!(d.data.get("pending_scope"), Some(&datum("later_adr")));
    check_proposed(FILE, "fn main(){let a=[1];drop(a);}", false).unwrap();
    check_proposed(
        FILE,
        "fn drop(x:i64){} fn main(){let a=[1];drop(a[0]);}",
        false,
    )
    .unwrap();
    check_proposed(FILE, "fn main(){drop([1][0]);}", false).unwrap();
}

#[test]
fn array_element_context_and_empty_array_inference_are_independent() {
    let d = error("fn main(){let xs=[];}", false);
    exact(
        &d,
        "NX-E0103",
        None,
        Stage::Types,
        Operation::TypesInfer,
        (17, 19),
        &[],
        &[],
    );
    check_proposed(FILE, "fn main(){let xs:Array<i64>=[];let ys=[1,2];}", false).unwrap();
    let d = error("fn main(){let xs=[1,true];}", false);
    assert_eq!(d.code, "NX-E0102");
    assert_eq!(d.data.get("expected_type"), Some(&datum("i64")));
    assert_eq!(d.data.get("found_type"), Some(&datum("bool")));
}

#[test]
fn index_and_intrinsic_domains_are_typed_after_resolution() {
    let d = error("fn main(){let x=1[0];}", false);
    exact(
        &d,
        "NX-E0102",
        Some("not_indexable"),
        Stage::Types,
        Operation::TypesCheck,
        (16, 17),
        &[(RelatedRole::FoundType, 16, 17)],
        &[
            ("expected_type", datum("domain:array")),
            ("found_type", datum("i64")),
        ],
    );
    let d = error("fn main(){let x=1;inspect(x);}", false);
    exact(
        &d,
        "NX-E0102",
        Some("resource_required"),
        Stage::Types,
        Operation::TypesCheck,
        (26, 27),
        &[(RelatedRole::FoundType, 26, 27)],
        &[
            ("expected_type", datum("domain:mock_resource")),
            ("found_type", datum("i64")),
        ],
    );
    check_proposed(
        FILE,
        "fn main(){let xs=[1];let x=xs[0];let r=mock_open(1);inspect(r);}",
        false,
    )
    .unwrap();
}

#[test]
fn entry_check_is_separate_from_module_check() {
    check_proposed(FILE, "fn f(){}", false).unwrap();
    let d = error("fn f(){}", true);
    exact(
        &d,
        "NX-E0112",
        Some("missing_main"),
        Stage::Entry,
        Operation::EntryCheck,
        (8, 8),
        &[],
        &[("entry", datum("main"))],
    );
    let d = error("fn main(x:i64){}", true);
    exact(
        &d,
        "NX-E0112",
        Some("invalid_main_signature"),
        Stage::Entry,
        Operation::EntryCheck,
        (3, 7),
        &[(RelatedRole::Context, 0, 14)],
        &[("entry", datum("main"))],
    );
    check_proposed(FILE, "fn main(){}", true).unwrap();
}

#[test]
fn unicode_comment_and_crlf_preserve_absent_name_byte_spans() {
    for (source, start, end) in [
        ("// á🙂\nfn main(){\nlet x=missing;\n}".to_owned(), 27, 34),
        (
            "// á🙂\r\nfn main(){\r\nlet x=missing;\r\n}".to_owned(),
            29,
            36,
        ),
        ("// á🙂\nfn main(){\nlet x=missing;\n}\n".to_owned(), 27, 34),
    ] {
        let d = error(&source, false);
        exact(
            &d,
            "NX-E0101",
            None,
            Stage::Names,
            Operation::NamesResolve,
            (start, end),
            &[],
            &[("use_kind", datum("value"))],
        );
        let positive = source.replace("let x=missing;", "let missing=1;let x=missing;");
        check_proposed(FILE, &positive, false).unwrap();
    }
}

#[test]
fn all_preparsed_names_are_resolved_by_lexical_position() {
    let source = "fn f(x:i64){{let f=1;let Option=2;let from=3;let value=4;assert_eq(f+Option+from+value,10);}assert_eq(x,1);} fn main(){f(1);}";
    let ast = parse::parse(FILE, source).unwrap();
    let resolved = names::resolve(FILE, source, &ast).unwrap();
    assert_eq!(resolved.signatures.len(), 2);
    check_proposed(FILE, source, false).unwrap();
}

#[test]
fn written_argument_defects_precede_missing_slots() {
    let d = error(
        "fn f(inout x:i64,y:i64){} fn main(){var x=1;f(z:1);}",
        false,
    );
    assert_eq!(d.subcause.as_deref(), Some("unknown_argument_label"));
    let d = error(
        "fn f(inout x:i64,y:i64){} fn main(){var x=1;f(x:1,x:2);}",
        false,
    );
    assert_eq!(d.subcause.as_deref(), Some("argument_mode_mismatch"));
    let d = error(
        "fn f(inout x:i64,y:i64){} fn main(){var x=1;f(x:1,2);}",
        false,
    );
    assert_eq!(d.subcause.as_deref(), Some("argument_mode_mismatch"));
    let d = error("fn f(x:i64,y:i64,z:i64){} fn main(){f(x:1,y:2,3);}", false);
    assert_eq!(d.subcause.as_deref(), Some("positional_after_named"));
    assert_eq!(
        d.related_spans
            .iter()
            .find(|r| r.role == RelatedRole::Context)
            .map(|r| r.span.clone()),
        Some(sp(38, 39))
    );
}

#[test]
fn inner_scopes_restore_outer_types_and_callability() {
    check_proposed(FILE,"fn f(x:i64)->i64{return x;} fn main(){let f=1;{let f=true;assert_eq(f,true);}assert_eq(f,1);}",false).unwrap();
    check_proposed(FILE,"fn f(x:i64)->i64{return x;} fn main(){{let f=1;assert_eq(f,1);}let n=f(2);assert_eq(n,2);}",false).unwrap();
    check_proposed(
        FILE,
        "fn main(){let x=1;{let x=true;assert_eq(x,true);}let z=x+1;assert_eq(z,2);}",
        false,
    )
    .unwrap();
}

#[test]
fn written_result_types_remain_whole_for_comparison_domains() {
    let d = error("fn main(){let a:Result<i64,i64>=Ok(1);let b=a<a;}", false);
    assert_eq!(d.primary_span, Some(sp(45, 46)));
    assert_eq!(d.data.get("expected_type"), Some(&datum("i64")));
    assert_eq!(d.data.get("found_type"), Some(&datum("Result<i64,i64>")));
    let d = error(
        "fn main(){let a:Result<MockResource,i64>=Ok(mock_open(1));let b=a==a;}",
        false,
    );
    assert_eq!(
        d.data.get("expected_type"),
        Some(&datum("domain:copyable_data"))
    );
    assert_eq!(
        d.data.get("found_type"),
        Some(&datum("Result<MockResource,i64>"))
    );
}

#[test]
fn annotation_parameter_and_assignment_contexts_report_concrete_types() {
    let d = error("fn main(){let x:bool=1;}", false);
    exact(
        &d,
        "NX-E0102",
        Some("type_mismatch"),
        Stage::Types,
        Operation::TypesCheck,
        (21, 22),
        &[
            (RelatedRole::ExpectedType, 16, 20),
            (RelatedRole::FoundType, 21, 22),
        ],
        &[
            ("expected_type", datum("bool")),
            ("found_type", datum("i64")),
        ],
    );
    let d = error("fn f(x:bool){} fn main(){f(1);}", false);
    exact(
        &d,
        "NX-E0102",
        Some("type_mismatch"),
        Stage::Types,
        Operation::TypesCheck,
        (27, 28),
        &[
            (RelatedRole::ExpectedType, 7, 11),
            (RelatedRole::FoundType, 27, 28),
        ],
        &[
            ("expected_type", datum("bool")),
            ("found_type", datum("i64")),
        ],
    );
    let d = error("fn main(){var x=1;x=true;}", false);
    assert_eq!(d.data.get("expected_type"), Some(&datum("i64")));
    assert_eq!(d.data.get("found_type"), Some(&datum("bool")));
}

#[test]
fn unresolved_result_constructor_uses_whole_call_and_known_context() {
    let d = error("fn main(){let r=Ok (1);}", false);
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
            Some(&sp(16, 22))
        )
    );
    assert_eq!(
        d.data.get("unresolved_components"),
        Some(&DataValue::Array(vec![datum("$.err")]))
    );
    check_proposed(FILE, "fn main(){let r:Result<i64,i64>=Ok(1);}", false).unwrap();
}

#[test]
fn argument_labels_select_types_without_changing_written_diagnostic_order() {
    check_proposed(
        FILE,
        "fn f(x:i64,y:bool){} fn main(){f(y:true,x:1);}",
        false,
    )
    .unwrap();
    let d = error("fn f(x:i64,y:bool){} fn main(){f(y:1,x:1);}", false);
    assert_eq!(d.data.get("expected_type"), Some(&datum("bool")));
    assert_eq!(d.data.get("found_type"), Some(&datum("i64")));
    let d = error("fn f(x:i64,y:bool){} fn main(){f(y:true,x:false);}", false);
    assert_eq!(d.data.get("expected_type"), Some(&datum("i64")));
    assert_eq!(d.data.get("found_type"), Some(&datum("bool")));
}

#[test]
fn both_result_variants_must_be_copyable_for_equality() {
    let d = error(
        "fn main(){let a:Result<i64,MockResource>=Err(mock_open(1));let b=a==a;}",
        false,
    );
    assert_eq!(
        d.data.get("expected_type"),
        Some(&datum("domain:copyable_data"))
    );
    assert_eq!(
        d.data.get("found_type"),
        Some(&datum("Result<i64,MockResource>"))
    );
}

#[test]
fn intrinsic_data_and_index_domains_reject_wrong_types() {
    let d = error("fn main(){let r=mock_open(1);print(r);}", false);
    assert_eq!(
        d.data.get("expected_type"),
        Some(&datum("domain:copyable_data"))
    );
    let d = error("fn main(){let x=[1][true];}", false);
    assert_eq!(d.data.get("expected_type"), Some(&datum("i64")));
    assert_eq!(d.data.get("found_type"), Some(&datum("bool")));
    let d = error("fn main(){assert_eq(1,true);}", false);
    assert_eq!(d.data.get("expected_type"), Some(&datum("i64")));
    assert_eq!(d.data.get("found_type"), Some(&datum("bool")));
}

#[test]
fn operand_and_operator_spans_survive_trivia() {
    let source = "fn main(){let b=true /*x*/ < false;}";
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
    check_proposed(FILE, "fn main(){let x=1+2;let y=3-1;}", false).unwrap();
    let source = "fn main(){let x=true+9223372036854775808;}";
    let d = error(source, false);
    assert_eq!(
        (&d.code, d.subcause.as_deref(), d.primary_span),
        (
            &"NX-E0102".to_owned(),
            Some("type_mismatch"),
            Some(sp(16, 20))
        )
    );
    let d = error("fn main(){let x=1+true;}", false);
    assert_eq!(d.data.get("expected_type"), Some(&datum("i64")));
    assert_eq!(d.data.get("found_type"), Some(&datum("bool")));
    let source = "fn main(){assert_eq(b:9223372036854775808,a:9223372036854775808);}";
    let d = error(source, false);
    assert_eq!(d.code, "NX-E0107");
    assert_eq!(d.primary_span, Some(sp(22, 41)));
}

#[test]
fn non_callable_expression_target_has_no_binding_declaration() {
    let d = error("fn main(){(1)();}", false);
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
    let d = error("fn f(x:i64){} fn main(){var x=1;f(inout x);}", false);
    assert_eq!(d.subcause.as_deref(), Some("argument_mode_mismatch"));
    check_proposed(FILE, "fn f(x:i64){} fn main(){var x=1;f(x);}", false).unwrap();
}

#[test]
fn written_unit_annotation_checks_unit_literal_type() {
    check_proposed(FILE, "fn main(){let x:()=();assert_eq(x,());}", false).unwrap();
    let d = error("fn main(){let x:()=1;}", false);
    assert_eq!(d.data.get("expected_type"), Some(&datum("()")));
    assert_eq!(d.data.get("found_type"), Some(&datum("i64")));
}

#[test]
fn branch_values_synthesize_without_context_and_use_written_return_context() {
    let d = error(
        "fn main(){let x=if true{value: 1;}else{value: true;};}",
        false,
    );
    assert_eq!(
        (d.code.as_str(), d.subcause.as_deref(), d.stage),
        ("NX-E1303", None, Stage::Types)
    );
    assert_eq!(d.data.get("expected_type"), Some(&datum("i64")));
    assert_eq!(d.data.get("found_type"), Some(&datum("bool")));
    assert_eq!(
        d.related_spans
            .iter()
            .filter(|r| r.role == RelatedRole::FoundType)
            .count(),
        1
    );
    let d = error(
        "fn f()->i64{return if true{value: 1;}else{value: true;};} fn main(){}",
        false,
    );
    assert_eq!(
        (d.code.as_str(), d.subcause.as_deref(), d.stage),
        ("NX-E0102", Some("type_mismatch"), Stage::Types)
    );
    assert_eq!(d.data.get("expected_type"), Some(&datum("i64")));
    assert_eq!(d.data.get("found_type"), Some(&datum("bool")));
    check_proposed(
        FILE,
        "fn main(){let x=if true{value: 1;}else{value: 2;};}",
        false,
    )
    .unwrap();
}

#[test]
fn adr_gap_duplicate_named_after_positional_relates_first_value() {
    let d = error("fn f(x:i64){} fn main(){f(1,x:2);}", false);
    assert_eq!(d.subcause.as_deref(), Some("duplicate_argument"));
    assert_eq!(
        d.related_spans
            .iter()
            .map(|r| (r.role, r.span.start, r.span.end))
            .collect::<Vec<_>>(),
        vec![(RelatedRole::Context, 26, 27)]
    );
}

#[test]
fn move_and_async_are_reserved_names() {
    for (name, end) in [("move", 18), ("async", 19)] {
        let source = format!("fn main(){{let {name}=1;}}");
        let d = error(&source, false);
        assert_eq!(
            (
                d.code.as_str(),
                d.subcause.as_deref(),
                d.stage,
                d.operation,
                d.primary_span
            ),
            (
                "NX-E0002",
                None,
                Stage::Parse,
                Operation::SyntaxParse,
                Some(sp(14, end))
            )
        );
        assert_eq!(d.parser_context.as_deref(), Some("binding_name"));
        assert_eq!(d.expected_tokens, ["Name"]);
        assert_eq!(d.recognized_prefix, Some(sp(10, 14)));
        assert_eq!(
            d.unexpected_token
                .as_ref()
                .map(|t| (t.text.as_deref(), t.span.clone())),
            Some((Some(name), sp(14, end)))
        );
        let positive = source.replace(name, &format!("{name}_value"));
        check_proposed(FILE, &positive, false).unwrap();
    }
}

#[test]
fn valid_generated_names_shadowing_and_comparisons_have_no_diagnostics() {
    let names = [
        "Option",
        "from",
        "value",
        "Ok",
        "Result",
        "print",
        "drop",
        "mock_open",
        "inspect",
        "assert_eq",
        "len",
        "x",
        "y",
        "z",
        "a",
        "b",
    ];
    for n in 0..8_000_u32 {
        let name = names[n as usize % names.len()];
        let value = if n % 2 == 0 {
            "1"
        } else {
            "9223372036854775807"
        };
        let source = format!(
            "fn f({name}:i64)->i64{{return {name}+1;}} fn main(){{let {name}={value};{{let {name}=true;let condition={name}==true;}}let outcome={name}<9223372036854775807;}} "
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
        "consume",
        "inout",
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
        let mut source = String::from("fn main(){let x=1;let y=x+1;}");
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
    let mut source = String::from("fn main(){");
    for _ in 0..110 {
        source.push('{');
    }
    source.push_str("let x=1;assert_eq(x,1);");
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
                "fn main(){{let x={};}}",
                "(".repeat(depth) + "1" + &")".repeat(depth)
            ),
            1 => format!(
                "fn main(){{let x={};}}",
                "[".repeat(depth) + "1" + &"]".repeat(depth)
            ),
            2 => format!(
                "fn main(){{{}let x=1;{}}}",
                "{".repeat(depth),
                "}".repeat(depth)
            ),
            3 => format!(
                "fn main(){{let x:{}i64{}={}1{};}}",
                "Array<".repeat(depth),
                ">".repeat(depth),
                "[".repeat(depth),
                "]".repeat(depth)
            ),
            4 => format!(
                "fn f(x:i64)->i64{{return x;}} fn main(){{let x={}1{};}}",
                "f(".repeat(depth),
                ")".repeat(depth)
            ),
            5 => format!("fn main(){{let x=[1]{};}}", "[0]".repeat(depth)),
            6 => format!(
                "fn main(){{let x={}{{value:1;}};}}",
                "if true{value:1;}else ".repeat(depth)
            ),
            7 => format!(
                "fn main(){{{}let x=1;{}}}",
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
        "fn main(){{{}let x=1;{}}}",
        "{".repeat(126),
        "}".repeat(126)
    );
    check_proposed(FILE, &source, false).unwrap();
}

#[test]
fn inferred_type_depth_enforces_array_priority_and_result_value_limit() {
    let mut source = String::from("fn main(){let a0=0;");
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

    let mut source = String::from("fn main(){let a0:Result<i64,i64>=Ok(0);");
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
    use std::time::Instant;
    fn source_for(family: usize, target: usize) -> String {
        let mut source = String::new();
        if family == 18 {
            let count = if target < 500_000 { 256 } else { 1023 };
            for i in 0..count {
                source.push_str(&format!("fn f{i}(){{let x=1;}}"));
            }
            source.push_str("fn main(){} ");
        } else if family == 19 {
            let count = target / 100;
            source.push_str("fn f(");
            for i in 0..count {
                if i > 0 {
                    source.push(',');
                }
                source.push_str(&format!("p{i}:i64"));
            }
            source.push_str("){} fn main(){f(");
            for i in 0..count {
                if i > 0 {
                    source.push(',');
                }
                source.push('1');
            }
            source.push_str(");}");
        } else {
            let prelude = if family == 7 || family == 8 || family == 23 {
                "fn f(x:i64){} "
            } else {
                ""
            };
            source.push_str(prelude);
            source.push_str("fn main(){");
            let mut i = 0usize;
            loop {
                let segment = match family {
                    0 => format!("let n{i}=1;"),
                    1 => "x=1;".into(),
                    2 => format!("let n{i}:i64=1;"),
                    3 => format!("let n{i}:bool=true;"),
                    4 => format!("let n{i}=[1,2];"),
                    5 => format!("let n{i}=[1][0];"),
                    6 => "assert_eq(1,1);".into(),
                    7 => "f(1);".into(),
                    8 => "f(x:1);".into(),
                    9 => format!("let n{i}=mock_open(1);inspect(n{i});"),
                    10 => format!("let n{i}:Result<i64,i64>=Ok(1);"),
                    11 => "{let x=1;assert_eq(x,1);}".into(),
                    12 => format!("let n{i}=1<2;"),
                    13 => format!("let n{i}=1==1;"),
                    14 => format!("let n{i}=true==false;"),
                    15 => format!("let n{i}=()==();"),
                    16 => format!("let n{i}=1+2-3;"),
                    17 => "{let Option=1;let from=Option+1;}".into(),
                    20 => "{".repeat(64) + "let x=1;assert_eq(x,1);" + &"}".repeat(64),
                    21 => "{".repeat(32) + "let x=1;" + &"}".repeat(32),
                    22 => {
                        let mut ty = "i64".to_owned();
                        let mut value = "1".to_owned();
                        for _ in 0..32 {
                            ty = format!("Array<{ty}>");
                            value = format!("[{value}]");
                        }
                        format!("let n{i}:{ty}={value};")
                    }
                    23 => "f(x:1);f(2);".into(),
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
            let mut samples = Vec::new();
            for _ in 0..if full { 3 } else { 2 } {
                let start = Instant::now();
                check_proposed(FILE, &source, false).unwrap_or_else(|d| {
                    panic!(
                        "family {family}, size {size}: {} {:?}",
                        d[0].code, d[0].primary_span
                    )
                });
                samples.push(start.elapsed().as_secs_f64());
            }
            samples.sort_by(f64::total_cmp);
            observations.push(samples[samples.len() / 2]);
        }
        let ratio = observations[1] / observations[0];
        println!(
            "semantic family {family:02}: 256KiB={:.4}s 1MiB={:.4}s ratio={ratio:.2}",
            observations[0], observations[1]
        );
        assert!(ratio <= 8.0, "superlinear family {family}: {ratio}");
    }
}
