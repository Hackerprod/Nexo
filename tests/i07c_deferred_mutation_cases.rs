use nexo::check::check_proposed;
use nexo::diagnostic::{DataValue, RelatedRole};
use nexo::ir::{CompileError, check_scalar};

type Signature = (
    &'static str,
    Option<&'static str>,
    &'static str,
    Option<&'static str>,
    Option<&'static str>,
);
type Cause = (
    String,
    Option<String>,
    String,
    Option<String>,
    Option<String>,
);

fn signatures(source: &str) -> Vec<Cause> {
    let errors = check_proposed("survivor.n", source, false).unwrap_err();
    errors
        .iter()
        .map(|error| {
            error.validate("survivor.n", source).unwrap();
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

fn witness(source: &str, required: &[Signature], forbidden: &[(&str, Option<&str>)]) {
    let actual = signatures(source);
    for &(code, subcause, primary, expected, found) in required {
        let count = actual
            .iter()
            .filter(|item| {
                item.0 == code
                    && item.1.as_deref() == subcause
                    && item.2 == primary
                    && item.3.as_deref() == expected
                    && item.4.as_deref() == found
            })
            .count();
        assert!(
            count >= 1,
            "missing {code}/{subcause:?} at {primary}: {actual:?}"
        );
    }
    for &(code, subcause) in forbidden {
        assert!(
            !actual
                .iter()
                .any(|item| item.0 == code && item.1.as_deref() == subcause),
            "unexpected {code}/{subcause:?}: {actual:?}"
        );
    }
    assert!(
        matches!(
            check_scalar("survivor.n", source),
            Err(CompileError::Rejected(_))
        ),
        "{actual:?}"
    );
}

#[test]
fn m00549_11061_omits_domain_of_union_with_invalid_condition() {
    // ADR §4.1.2.5: the invalid inner condition contaminates its union;
    // the outer index cannot claim the union has a known non-Array type.
    // Each independent empty constructor retains §4.1.2.2's obligation.
    let source =
        "fn Main(){let c=true;let a3=(if c then (if MockOpen(1) then [] else []) else [])[1];}";
    let errors = check_proposed("survivor.n", source, false).unwrap_err();
    let actual: Vec<_> = errors
        .iter()
        .map(|error| {
            error.validate("survivor.n", source).unwrap();
            let span = error.primary_span.as_ref().unwrap();
            (
                error.code.as_str(),
                error.subcause.as_deref(),
                span.start as usize,
                span.end as usize,
            )
        })
        .collect();
    assert_eq!(
        actual,
        [
            ("NX-E0102", Some("type_mismatch"), 43, 54),
            ("NX-E0103", None, 60, 62),
            ("NX-E0103", None, 68, 70),
            ("NX-E0103", None, 77, 79),
        ]
    );
    assert!(matches!(
        check_scalar("survivor.n", source),
        Err(CompileError::Rejected(_))
    ));
}

#[test]
fn m00058_does_not_invent_array_shape_from_hypothetical_write() {
    // ADR §4.1.2.3: an indexed write to a base with no constructor does
    // not establish Array. The earlier selector on that binding cannot be
    // diagnosed as Array versus I64 without a concrete base shape. After
    // the incompatible later equality is rejected, [w2] still has its own
    // unresolved-element obligation under §4.1.2.2.
    let source = "fn Main(){var w2=if true then {return;} else {return;};if w2[w2] {w2[0]=1;} else {Inspect(1);var w7=if 0 then [w2][0] else [1];}}";
    let errors = check_proposed("survivor.n", source, false).unwrap_err();
    let actual: Vec<_> = errors
        .iter()
        .map(|error| {
            error.validate("survivor.n", source).unwrap();
            let span = error.primary_span.as_ref().unwrap();
            (
                error.code.as_str(),
                error.subcause.as_deref(),
                span.start as usize,
                span.end as usize,
            )
        })
        .collect();
    assert_eq!(
        actual,
        [
            ("NX-E0102", Some("resource_required"), 90, 91),
            ("NX-E0102", Some("type_mismatch"), 103, 104),
            ("NX-E0103", None, 110, 114),
            ("NX-E0102", Some("type_mismatch"), 123, 126),
        ]
    );
    assert!(matches!(
        check_scalar("survivor.n", source),
        Err(CompileError::Rejected(_))
    ));
}

macro_rules! mutant {
    ($name:ident, $file:literal, $required:expr, $forbidden:expr) => {
        #[test]
        fn $name() {
            witness(
                include_str!(concat!("fixtures/i07c_deferred_cases/", $file)),
                $required,
                $forbidden,
            );
        }
    };
}

// F0-0002 §§4.1.2.2–5, N(a)–(d),(h): each source is the independent review's
// program witness for a distinct surviving substitution. The expected C
// signature is written here, independently of the implementation under test.
mutant!(
    m00039_preserves_rhs_c,
    "indexed_write_invalid_ir.n",
    &[(
        "NX-E0102",
        Some("type_mismatch"),
        "ar",
        Some("I64"),
        Some("Array<I64>")
    )],
    &[]
);
mutant!(
    m00075_preserves_base_domain,
    "pending_selector_acceptance.n",
    &[(
        "NX-E0102",
        Some("not_indexable"),
        "w",
        Some("domain:array"),
        Some("I64")
    )],
    &[]
);
mutant!(
    m00038_preserves_rhs_constructor,
    "pending_indexed_rhs_inference_loss.n",
    &[("NX-E0103", None, "[]", None, None)],
    &[]
);
mutant!(
    m00509_preserves_rhs_constructor,
    "poisoned_indexed_rhs_inference_loss.n",
    &[("NX-E0103", None, "[]", None, None)],
    &[]
);
mutant!(
    m00050_preserves_base_domain,
    "nested_selector_domain_loss.n",
    &[(
        "NX-E0102",
        Some("not_indexable"),
        "w",
        Some("domain:array"),
        Some("I64")
    )],
    &[]
);
mutant!(
    m00059_keeps_inference_code,
    "nested_selector_inference_code_change.n",
    &[("NX-E0103", None, "[]", None, None)],
    &[("NX-E0102", Some("type_mismatch"))]
);
mutant!(
    m00060_preserves_constructor,
    "pending_equality_inference_loss.n",
    &[("NX-E0103", None, "[]", None, None)],
    &[]
);
mutant!(
    m00671_preserves_constructor,
    "array_index_inference_loss.n",
    &[("NX-E0103", None, "[]", None, None)],
    &[]
);
mutant!(
    m00218_keeps_union_code,
    "indexed_union_code_change.n",
    &[("NX-E1303", None, "ar", Some("I64"), Some("Array<I64>"))],
    &[]
);
mutant!(
    m00494_keeps_operator_domain,
    "comparison_operator_domain_loss.n",
    &[(
        "NX-E0102",
        Some("type_mismatch"),
        "<",
        Some("I64"),
        Some("Bool")
    )],
    &[]
);
mutant!(
    m00462_keeps_operator_domain,
    "array_comparison_domain_loss.n",
    &[(
        "NX-E0102",
        Some("type_mismatch"),
        "<",
        Some("I64"),
        Some("Array<Array<I64>>")
    )],
    &[]
);
mutant!(
    m00454_preserves_two_constructors,
    "conditional_constructor_inference_loss.n",
    &[("NX-E0103", None, "[]", None, None)],
    &[]
);
mutant!(
    m00453_omits_derived_union,
    "conditional_array_spurious_union.n",
    &[],
    &[("NX-E1303", None)]
);
mutant!(
    m00632_keeps_outer_union,
    "nested_branch_outer_union_loss.n",
    &[(
        "NX-E1303",
        None,
        "[[],ar]",
        Some("Array<I64>"),
        Some("Array<Array<I64>>")
    )],
    &[]
);
mutant!(
    m00398_keeps_later_operand_primary,
    "nested_condition_primary_shift.n",
    &[(
        "NX-E0102",
        Some("type_mismatch"),
        "ar",
        Some("I64"),
        Some("Array<I64>")
    )],
    &[]
);
mutant!(
    m00434_keeps_operator_domain,
    "conditional_index_operator_domain_loss.n",
    &[(
        "NX-E0102",
        Some("type_mismatch"),
        "<",
        Some("I64"),
        Some("Bool")
    )],
    &[]
);
mutant!(
    m00497_keeps_operator_domain,
    "print_comparison_operator_domain_loss.n",
    &[(
        "NX-E0102",
        Some("type_mismatch"),
        "<",
        Some("I64"),
        Some("Array<I64>")
    )],
    &[]
);

#[test]
fn m00260_preserves_both_constructor_sites() {
    let source =
        include_str!("fixtures/i07c_deferred_cases/nested_empty_constructor_inference_loss.n");
    let actual = signatures(source);
    assert_eq!(
        actual.iter().filter(|item| item.0 == "NX-E0103").count(),
        2,
        "{actual:?}"
    );
    assert!(matches!(
        check_scalar("survivor.n", source),
        Err(CompileError::Rejected(_))
    ));
}

#[test]
fn independent_numeric_selector_operand_survives_failed_base() {
    // §4.1.2.5 N(d), §4.6: a failed indexed base does not suppress
    // the independent I64 requirement of the selector's subtraction.
    let source =
        "fn Main(){let census29=1;var v3=true;var v4=(if v3 then v3 else {value:[2,3];})[(v3-9)];}";
    let errors = check_proposed("survivor.n", source, false).unwrap_err();
    assert_eq!(errors.len(), 3);
    for error in &errors {
        error.validate("survivor.n", source).unwrap();
    }
    assert!(errors.iter().any(|error| {
        error.code == "NX-E0102"
            && error.subcause.as_deref() == Some("type_mismatch")
            && error.primary_span.as_ref().is_some_and(|span| {
                (span.start, span.end) == (81, 83)
                    && &source[span.start as usize..span.end as usize] == "v3"
            })
            && error.data.get("expected_type") == Some(&DataValue::String("I64".into()))
            && error.data.get("found_type") == Some(&DataValue::String("Bool".into()))
    }));
    let numeric = errors
        .iter()
        .find(|error| {
            error
                .primary_span
                .as_ref()
                .is_some_and(|span| span.start == 81)
        })
        .unwrap();
    assert_eq!(numeric.related_spans.len(), 1);
    assert_eq!(numeric.related_spans[0].role, RelatedRole::FoundType);
    assert_eq!(
        (
            numeric.related_spans[0].span.start,
            numeric.related_spans[0].span.end
        ),
        (81, 83)
    );
    assert!(matches!(
        check_scalar("survivor.n", source),
        Err(CompileError::Rejected(_))
    ));

    let repaired = source.replace("(v3-9)", "(1-9)");
    let repaired_errors = check_proposed("survivor.n", &repaired, false).unwrap_err();
    assert_eq!(repaired_errors.len(), 2);
    assert!(!repaired_errors.iter().any(|error| {
        error.code == "NX-E0102" && error.subcause.as_deref() == Some("type_mismatch")
    }));
}

#[test]
fn pending_array_constructor_preserves_independent_resource_domain() {
    // A constructor with an unresolved element is still an Array, and it
    // cannot satisfy Inspect's MockResource domain (§4.1.2.3).
    let source = "fn Main(){var a=[];Inspect(a);a=[1];}";
    let errors = check_proposed("survivor.n", source, false).unwrap_err();
    assert_eq!(errors.len(), 1);
    let error = &errors[0];
    error.validate("survivor.n", source).unwrap();
    assert_eq!(error.code, "NX-E0102");
    assert_eq!(error.subcause.as_deref(), Some("resource_required"));
    let span = error.primary_span.as_ref().unwrap();
    assert_eq!((span.start, span.end), (27, 28));
    assert_eq!(&source[span.start as usize..span.end as usize], "a");
    assert_eq!(
        error.data.get("expected_type"),
        Some(&DataValue::String("domain:mock_resource".into()))
    );
    assert_eq!(
        error.data.get("found_type"),
        Some(&DataValue::String("Array<I64>".into()))
    );
    assert_eq!(error.related_spans.len(), 1);
    assert_eq!(error.related_spans[0].role, RelatedRole::FoundType);
    assert_eq!(
        (
            error.related_spans[0].span.start,
            error.related_spans[0].span.end
        ),
        (27, 28)
    );
    assert!(matches!(
        check_scalar("survivor.n", source),
        Err(CompileError::Rejected(_))
    ));

    let repaired = source.replace("Inspect(a);", "");
    assert!(check_proposed("survivor.n", &repaired, false).is_ok());
    assert!(check_scalar("survivor.n", &repaired).is_ok());
}

#[test]
fn pending_numeric_comparison_remains_join_witness() {
    // The later q=2 satisfies the pending I64 domain of q<1. Its Bool
    // result remains the first valid continuation of the outer union.
    let source = "fn Main(){var q=(if true then {return;} else {return;});let x=(if true then q<1 else 1);q=2;}";
    let errors = check_proposed("survivor.n", source, false).unwrap_err();
    assert_eq!(errors.len(), 1);
    let error = &errors[0];
    error.validate("survivor.n", source).unwrap();
    assert_eq!(error.code, "NX-E1303");
    assert_eq!(error.subcause, None);
    let span = error.primary_span.as_ref().unwrap();
    assert_eq!((span.start, span.end), (85, 86));
    assert_eq!(&source[span.start as usize..span.end as usize], "1");
    assert_eq!(
        error.data.get("expected_type"),
        Some(&DataValue::String("Bool".into()))
    );
    assert_eq!(
        error.data.get("found_type"),
        Some(&DataValue::String("I64".into()))
    );
    assert_eq!(error.related_spans.len(), 2);
    assert_eq!(error.related_spans[0].role, RelatedRole::ExpectedType);
    assert_eq!(
        (
            error.related_spans[0].span.start,
            error.related_spans[0].span.end
        ),
        (76, 79)
    );
    assert_eq!(error.related_spans[1].role, RelatedRole::FoundType);
    assert_eq!(
        (
            error.related_spans[1].span.start,
            error.related_spans[1].span.end
        ),
        (85, 86)
    );
    assert!(matches!(
        check_scalar("survivor.n", source),
        Err(CompileError::Rejected(_))
    ));

    let repaired = source.replace("else 1);", "else true);");
    assert!(check_proposed("survivor.n", &repaired, false).is_ok());
    assert!(check_scalar("survivor.n", &repaired).is_ok());
}

#[test]
fn pending_equality_keeps_outer_join_cause() {
    // The later assignment resolves r's Array element. The equality has a
    // valid Bool result, independently of the pending array element.
    let source =
        "fn Main(){let p=true;let ar=[1];var r=[];let a=(if p then [ar] else (r==r));r=[3];}";
    let errors = check_proposed("survivor.n", source, false).unwrap_err();
    assert_eq!(errors.len(), 1);
    let error = &errors[0];
    error.validate("survivor.n", source).unwrap();
    assert_eq!(error.code, "NX-E1303");
    assert_eq!(error.subcause, None);
    let span = error.primary_span.as_ref().unwrap();
    assert_eq!((span.start, span.end), (69, 73));
    assert_eq!(&source[span.start as usize..span.end as usize], "r==r");
    assert_eq!(
        error.data.get("expected_type"),
        Some(&DataValue::String("Array<Array<I64>>".into()))
    );
    assert_eq!(
        error.data.get("found_type"),
        Some(&DataValue::String("Bool".into()))
    );
    assert_eq!(error.related_spans.len(), 2);
    assert_eq!(error.related_spans[0].role, RelatedRole::ExpectedType);
    assert_eq!(
        (
            error.related_spans[0].span.start,
            error.related_spans[0].span.end
        ),
        (58, 62)
    );
    assert_eq!(error.related_spans[1].role, RelatedRole::FoundType);
    assert_eq!(
        (
            error.related_spans[1].span.start,
            error.related_spans[1].span.end
        ),
        (69, 73)
    );
    assert!(matches!(
        check_scalar("survivor.n", source),
        Err(CompileError::Rejected(_))
    ));

    let repaired = source.replace("else (r==r)", "else [ar]");
    assert!(check_proposed("survivor.n", &repaired, false).is_ok());
    assert!(check_scalar("survivor.n", &repaired).is_ok());
}
