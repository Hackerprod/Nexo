use nexo::check::check_proposed;
use nexo::diagnostic::DataValue;
use nexo::ir::{CompileError, check_scalar};

type Cause = (
    String,
    Option<String>,
    String,
    Option<String>,
    Option<String>,
);

fn causal(source: &str) -> Vec<Cause> {
    let errors = check_proposed("deferred_case.n", source, false).unwrap_err();
    errors
        .iter()
        .map(|error| {
            error.validate("deferred_case.n", source).unwrap();
            let site = error.primary_span.as_ref().unwrap();
            let field = |name| match error.data.get(name) {
                Some(DataValue::String(value)) => Some(value.clone()),
                _ => None,
            };
            (
                error.code.clone(),
                error.subcause.clone(),
                source[site.start as usize..site.end as usize].to_owned(),
                field("expected_type"),
                field("found_type"),
            )
        })
        .collect()
}

#[test]
fn deferred_expression_twins_retain_independent_causes() {
    // F0-0002 §§4.1.2.2–7: completing the sole pending r from a later
    // compatible assignment preserves the C sites of its concrete twin.
    let pairs = [
        (
            "less_than_array_union",
            include_str!("fixtures/i07c_deferred_cases/less_than_array_union.n"),
            include_str!("fixtures/i07c_deferred_cases/less_than_array_union_concrete.n"),
        ),
        (
            "numeric_nested_union",
            include_str!("fixtures/i07c_deferred_cases/numeric_nested_union.n"),
            include_str!("fixtures/i07c_deferred_cases/numeric_nested_union_concrete.n"),
        ),
        (
            "nested_less_than_domain",
            include_str!("fixtures/i07c_deferred_cases/nested_less_than_domain.n"),
            include_str!("fixtures/i07c_deferred_cases/nested_less_than_domain_concrete.n"),
        ),
        (
            "projection_equality_domain",
            include_str!("fixtures/i07c_deferred_cases/projection_equality_domain.n"),
            include_str!("fixtures/i07c_deferred_cases/projection_equality_domain_concrete.n"),
        ),
        (
            "nested_array_union",
            include_str!("fixtures/i07c_deferred_cases/nested_array_union.n"),
            include_str!("fixtures/i07c_deferred_cases/nested_array_union_concrete.n"),
        ),
        (
            "comparison_branch_union",
            include_str!("fixtures/i07c_deferred_cases/comparison_branch_union.n"),
            include_str!("fixtures/i07c_deferred_cases/comparison_branch_union_concrete.n"),
        ),
        (
            "multiple_pending_union",
            include_str!("fixtures/i07c_deferred_cases/multiple_pending_union.n"),
            include_str!("fixtures/i07c_deferred_cases/multiple_pending_union_concrete.n"),
        ),
    ];
    let mut differences = Vec::new();
    for (name, pending, concrete) in pairs {
        let pending_causes = causal(pending);
        let concrete_causes = causal(concrete);
        if pending_causes != concrete_causes {
            differences.push((name, pending_causes, concrete_causes));
        }
        assert!(
            matches!(
                check_scalar("deferred_case.n", pending),
                Err(CompileError::Rejected(_))
            ),
            "{name}"
        );
        assert!(
            matches!(
                check_scalar("deferred_case.n", concrete),
                Err(CompileError::Rejected(_))
            ),
            "{name}"
        );
    }
    assert!(differences.is_empty(), "{differences:#?}");
}

#[test]
fn pending_equality_result_and_numeric_rhs_match_concrete_twin() {
    let deferred = include_str!("fixtures/i07c_deferred_cases/equality_numeric_rhs.n");
    let concrete = include_str!("fixtures/i07c_deferred_cases/equality_numeric_rhs_concrete.n");
    assert_eq!(causal(deferred), causal(concrete));
}

#[test]
fn poisoned_assignment_still_checks_rhs() {
    // §4.1.2.5: independent expressions in one statement retain C.
    for (assigned, independent) in [
        (
            include_str!("fixtures/i07c_deferred_cases/poisoned_assignment_selector_loss.n"),
            include_str!("fixtures/i07c_deferred_cases/independent_selector_control.n"),
        ),
        (
            include_str!("fixtures/i07c_deferred_cases/poisoned_assignment_len_domain.n"),
            include_str!("fixtures/i07c_deferred_cases/independent_len_control.n"),
        ),
    ] {
        let assignment_causes = causal(assigned);
        let independent_causes = causal(independent);
        assert_eq!(assignment_causes, independent_causes);
    }
}

#[test]
fn deferred_array_outcomes_have_absolute_acceptance_oracles() {
    // ADR §§4.1.2.2–5: these outcomes are fixed independently of a twin.
    // The negative rows exercise invalid selectors, operands, branches and
    // incompatible resolutions; the positive rows prevent blanket rejection.
    for (source, should_accept) in [
        ("fn Main(){var r=[];r=[3];let x=r[0];}", true),
        ("fn Main(){var r=[];let x=r[0];r=[3];}", true),
        ("fn Main(){var r=[];let a=[r,[3]];r=[3];}", true),
        ("fn Main(){var r=[];let x=Len(r);r=[3];}", true),
        ("fn Main(){var r=[];let x=(r==[3]);r=[3];}", true),
        ("fn Main(){var r=[];let x=r[true];r=[3];}", false),
        ("fn Main(){var r=[];let x=r[0][0];r=[3];}", false),
        ("fn Main(){var r=[];let x=(r+true);r=[3];}", false),
        (
            "fn Main(){var r=[];let x=(if r then 1 else 1);r=[3];}",
            false,
        ),
        (
            "fn Main(){var r=[];var q=[];let x=(r==q);r=[3];q=[true];}",
            false,
        ),
    ] {
        assert_eq!(
            check_proposed("deferred_case.n", source, false).is_ok(),
            should_accept,
            "check: {source}"
        );
        let compile = check_scalar("deferred_case.n", source);
        assert_eq!(compile.is_ok(), should_accept, "run: {source}: {compile:?}");
        if !should_accept {
            assert!(
                matches!(compile, Err(CompileError::Rejected(_))),
                "{source}"
            );
        }
    }
}

#[test]
fn both_unresolved_constructors_retain_own_site() {
    // §4.1.2.2: two unresolved [] constructors require two NX-E0103 sites.
    let source = include_str!("fixtures/i07c_deferred_cases/mixed_union_empty_constructor_loss.n");
    let causes = causal(source);
    assert_eq!(
        causes.iter().filter(|cause| cause.0 == "NX-E0103").count(),
        2,
        "{causes:?}"
    );
    assert!(matches!(
        check_scalar("deferred_case.n", source),
        Err(CompileError::Rejected(_))
    ));
    let twin = include_str!("fixtures/i07c_deferred_cases/two_empty_constructors_control.n");
    assert_eq!(
        causal(twin)
            .iter()
            .filter(|cause| cause.0 == "NX-E0103")
            .count(),
        2
    );
}

#[test]
fn assignment_resolution_order_keeps_causal_leaf_primary() {
    // §4.1.2.6: the incompatible leaf keeps its site independently of the
    // order in which later assignments complete the two operands.
    for source in [
        include_str!("fixtures/i07c_deferred_cases/array_resolution_order_first.n"),
        include_str!("fixtures/i07c_deferred_cases/value_resolution_order_first.n"),
    ] {
        let causes = causal(source);
        assert_eq!(causes.len(), 1, "{causes:?}");
        assert_eq!(causes[0].0, "NX-E0102");
        assert_eq!(causes[0].2, "true", "{causes:?}");
    }
}

#[test]
fn two_nested_unresolved_arrays_are_reported_without_internal_error() {
    let source =
        include_str!("fixtures/i07c_deferred_cases/nested_empty_constructor_inference_loss.n");
    let check = check_proposed("b09.n", source, false);
    assert!(check.is_err(), "{check:?}");
    let run = check_scalar("b09.n", source);
    assert!(matches!(run, Err(CompileError::Rejected(_))), "{run:?}");
    let causes = causal(source);
    assert_eq!(
        causes.iter().filter(|cause| cause.0 == "NX-E0103").count(),
        2,
        "{causes:?}"
    );
}

#[test]
fn failed_pending_domain_does_not_hide_independent_nested_union() {
    // §4.1.2.5: the invalid condition and its branch union remain separate
    // causes when the earlier array element has a failed pending domain.
    let pending = "fn Main(){var q=[];let a=[(q<q),(if [0] then 18 else false)];q=[1];}";
    let concrete = "fn Main(){let q=[1];let a=[(q<q),(if [0] then 18 else false)];}";
    assert_eq!(causal(pending), causal(concrete));
    assert_eq!(causal(pending).len(), 3);
}

#[test]
fn established_array_constructor_satisfies_len_domain_before_element_resolution() {
    let pending = "fn Main(){let p=true;let x=17;let ar=[1];var r=[];let a=(x < ((Len(r) == [ar]) + x));r=[3];}";
    let twin =
        "fn Main(){let p=true;let x=17;let ar=[1];let r=[3];let a=(x < ((Len(r) == [ar]) + x));}";
    assert_eq!(causal(pending), causal(twin));
}

#[test]
fn guarded_equality_result_retains_cross_operand_cause() {
    let pending =
        "fn Main(){let p=true;let x=24;let ar=[1];var r=[];let a=((r == r) == [ar]);r=[[true]];}";
    let twin = "fn Main(){let p=true;let x=24;let ar=[1];let r=[[true]];let a=((r == r) == [ar]);}";
    assert_eq!(causal(pending), causal(twin));
}

#[test]
fn unresolved_guarded_domain_omits_derived_cross_comparison() {
    // §4.1.2.2,5: an unresolved constructor owns its C; the nominal Bool
    // of its pending comparison cannot accuse the outer right operand.
    let causes = causal("fn Main(){var r=[];let a=(r==r)==[1];}");
    assert_eq!(causes.len(), 1, "{causes:?}");
    assert_eq!(causes[0].0, "NX-E0103");
    assert_eq!(causes[0].2, "[]");
}

#[test]
fn pending_branch_context_error_does_not_accuse_its_union() {
    let pending = "fn Main(){let p=true;let x=55;let ar=[1];var r=[];let a=(((if p then (r == r) else x) + x) + x);r=[3];}";
    let twin = "fn Main(){let p=true;let x=55;let ar=[1];let r=[3];let a=(((if p then (r == r) else x) + x) + x);}";
    assert_eq!(causal(pending), causal(twin));
}

#[test]
fn pending_right_comparison_domain_does_not_supply_outer_union_type() {
    let pending = "fn Main(){let p=true;let x=102;let ar=[1];var r=[];let a=(if p then [ar] else (x < (r < r)));r=[[true]];}";
    let twin = "fn Main(){let p=true;let x=102;let ar=[1];let r=[[true]];let a=(if p then [ar] else (x < (r < r)));}";
    assert_eq!(causal(pending), causal(twin));
}

#[test]
fn invalid_pending_selector_does_not_supply_projection_type() {
    let pending = "fn Main(){let p=true;let x=228;let ar=[1];var r=[];let a=(if p then (x < r[(r < r)]) else x);r=[3];}";
    let twin = "fn Main(){let p=true;let x=228;let ar=[1];let r=[3];let a=(if p then (x < r[(r < r)]) else x);}";
    assert_eq!(causal(pending), causal(twin));
}
