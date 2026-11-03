use nexo::check::check_proposed;
use nexo::diagnostic::{DataValue, Operation, RelatedRole, Stage};
use nexo::ir::{CompileError, check_scalar};

type Expected = (&'static str, Option<&'static str>, usize, usize);

#[test]
fn adr_gap_self_selector_without_concrete_base_keeps_current_acceptance() {
    // ADR §4.1.2.3 leaves a missing Array domain pending and makes the
    // selector's I64 requirement conditional on an indexable base. It does
    // not settle a self-selector on a hypothetical binding with no concrete
    // type or constructor; preserve the current result for later design.
    let hypothetical = "fn Main(){var w=(if true then {return;} else {return;});let a=w[w];}";
    assert!(check_proposed("adr_gap_self_selector.n", hypothetical, false).is_ok());
    assert!(check_scalar("adr_gap_self_selector.n", hypothetical).is_ok());

    for source in [
        "fn Main(){let w=1;let a=w[w];}",
        "fn Main(){let w=[1];let a=w[w];}",
    ] {
        assert!(check_proposed("self_selector_concrete.n", source, false).is_err());
        assert!(check_scalar("self_selector_concrete.n", source).is_err());
    }
}

#[test]
fn deferred_projection_and_constructor_programs_finish_with_causal_results() {
    // ADR §§4.1.2.2–5, 11.1 and 13.9: these programs exercise source
    // constraints. A checker failure or nontermination is a test failure.
    let cases: &[(&str, &str, &[Expected])] = &[
        (
            "nested_projection_on_divergence",
            include_str!("fixtures/i07c/divergent_binding_nested_projection.n"),
            &[],
        ),
        (
            "deferred_selector_corpus_obligation",
            // §4.1.2.3: the selector's later Bool arm independently violates I64.
            include_str!("fixtures/i07c/deferred_selector_corpus_obligation.n"),
            &[("NX-E0102", Some("type_mismatch"), 85, 89)],
        ),
        (
            "deferred_selector_corpus_valid",
            "fn Main(){var q=(if true then {return;} else {return;});let a=q[(if true then 1 else 2)];q=[1];}",
            &[],
        ),
        (
            "invalid_condition_before_array_index",
            // §§4.1.2.4–5, 13.9: the condition and numeric RHS are two C sites.
            include_str!("fixtures/i07c/invalid_condition_before_array_index.n"),
            &[
                ("NX-E0102", Some("type_mismatch"), 62, 64),
                ("NX-E0102", Some("type_mismatch"), 81, 85),
            ],
        ),
        (
            "condition_and_array_index_valid",
            "fn Main(){let v=(if true then [1] else [2]);let w=((if true then 29 else 1)+v[0]);let nonce=1;}",
            &[],
        ),
        (
            "resource_array_write",
            include_str!("fixtures/i07c/deferred_resource_array_write.n"),
            &[
                ("NX-E0102", Some("type_mismatch"), 57, 59),
                ("NX-E0102", Some("type_mismatch"), 78, 80),
            ],
        ),
        (
            "aliased_projection",
            include_str!("fixtures/i07c/aliased_deferred_projection_terminates.n"),
            &[("NX-E0102", Some("not_indexable"), 123, 128)],
        ),
        (
            "projection_rhs",
            include_str!("fixtures/i07c/deferred_projection_rhs_conflict.n"),
            &[("NX-E0102", Some("type_mismatch"), 70, 75)],
        ),
        (
            "indexed_write_equality",
            include_str!("fixtures/i07c/deferred_indexed_write_equality_conflict.n"),
            &[("NX-E0102", Some("type_mismatch"), 87, 90)],
        ),
        (
            "len_divergent_element",
            include_str!("fixtures/i07c/len_divergent_array_element.n"),
            &[("NX-E0103", None, 17, 63)],
        ),
        (
            "divergent_union_assignment",
            include_str!("fixtures/i07c/deferred_union_array_assignment_accepts.n"),
            &[],
        ),
        (
            "nested_branch_unresolved",
            include_str!("fixtures/i07c/call_nested_branch_unresolved_array.n"),
            &[
                ("NX-E0102", Some("type_mismatch"), 69, 76),
                ("NX-E0102", Some("type_mismatch"), 90, 93),
                ("NX-E0103", None, 104, 106),
            ],
        ),
        (
            "recursive_array_obligation",
            include_str!("fixtures/i07c/indexed_recursive_array_obligation.n"),
            &[("NX-E0103", None, 38, 40)],
        ),
        (
            "deferred_alias_literal",
            include_str!("fixtures/i07c/array_literal_after_deferred_alias.n"),
            &[("NX-E0103", None, 75, 77)],
        ),
        (
            "resource_projection",
            include_str!("fixtures/i07c/resource_projection_and_scalar_base.n"),
            &[
                ("NX-E0102", Some("type_mismatch"), 57, 62),
                ("NX-E0102", Some("type_mismatch"), 81, 83),
                ("NX-E0102", Some("not_indexable"), 94, 96),
            ],
        ),
        (
            "invalid_base_selector",
            include_str!("fixtures/i07c/selector_error_on_invalid_base.n"),
            &[
                ("NX-E0102", Some("type_mismatch"), 41, 47),
                ("NX-E0102", Some("type_mismatch"), 94, 106),
            ],
        ),
        (
            "indexed_array_rhs",
            // ADR §§4.1.2.2, 4.1.2.7 and 13.15: the failed first
            // assignment leaves the valid first element as Array<I64>.
            // The later indexed write has a determined I64 destination;
            // its incompatible RHS is [x], as in the concrete twin.
            include_str!("fixtures/i07c/indexed_array_conflicting_rhs.n"),
            &[
                ("NX-E0102", Some("type_mismatch"), 37, 41),
                ("NX-E0102", Some("type_mismatch"), 53, 56),
            ],
        ),
        (
            "projection_domain",
            include_str!("fixtures/i07c/deferred_projection_comparison_domain.n"),
            &[("NX-E0102", Some("type_mismatch"), 70, 75)],
        ),
        (
            "constructor_and_projection",
            include_str!("fixtures/i07c/unresolved_constructor_before_projection_domain.n"),
            &[
                ("NX-E0103", None, 17, 19),
                ("NX-E0102", Some("type_mismatch"), 80, 85),
            ],
        ),
        (
            "repeated_constructor_sites",
            include_str!("fixtures/i07c/repeated_unresolved_array_sites.n"),
            &[
                ("NX-E0103", None, 17, 19),
                ("NX-E0103", None, 27, 37),
                ("NX-E0103", None, 34, 36),
            ],
        ),
        (
            "projection_sibling",
            include_str!("fixtures/i07c/array_projection_sibling_conflict.n"),
            &[("NX-E0102", Some("type_mismatch"), 34, 36)],
        ),
        (
            "branch_constructor",
            include_str!("fixtures/i07c/independent_unresolved_constructor_in_branch.n"),
            &[
                ("NX-E0103", None, 56, 58),
                ("NX-E0102", Some("type_mismatch"), 81, 83),
                ("NX-E0103", None, 115, 117),
                ("NX-E0102", Some("type_mismatch"), 132, 134),
                ("NX-E0102", Some("type_mismatch"), 179, 181),
                ("NX-E0102", Some("not_indexable"), 194, 195),
            ],
        ),
        (
            "two_unresolved_assignments",
            include_str!("fixtures/i07c/two_unresolved_assignments_after_condition.n"),
            &[
                ("NX-E0103", None, 37, 39),
                ("NX-E0102", Some("type_mismatch"), 55, 56),
                ("NX-E0102", Some("type_mismatch"), 75, 76),
                ("NX-E0103", None, 87, 89),
            ],
        ),
        (
            "argument_constructor",
            include_str!("fixtures/i07c/unresolved_argument_after_invalid_array.n"),
            &[
                ("NX-E0102", Some("type_mismatch"), 45, 57),
                ("NX-E0103", None, 139, 141),
            ],
        ),
        (
            "indexed_write_type",
            include_str!("fixtures/i07c/deferred_indexed_write_later_type_conflict.n"),
            &[
                ("NX-E0102", Some("type_mismatch"), 77, 81),
                ("NX-E0102", Some("type_mismatch"), 99, 100),
            ],
        ),
        (
            "function_array_sibling",
            include_str!("fixtures/i07c/function_array_element_after_invalid_selector.n"),
            &[
                ("NX-E0102", Some("not_indexable"), 92, 122),
                ("NX-E0102", Some("type_mismatch"), 95, 97),
                ("NX-E1303", None, 118, 120),
                ("NX-E0102", Some("type_mismatch"), 133, 135),
            ],
        ),
        (
            "invalid_target_union",
            include_str!("fixtures/i07c/invalid_indexed_target_keeps_union_error.n"),
            &[("NX-E0103", None, 17, 19), ("NX-E1303", None, 67, 73)],
        ),
        (
            "selector_numeric_leaf",
            include_str!("fixtures/i07c/selector_numeric_leaf_after_invalid_union.n"),
            &[
                ("NX-E0102", Some("not_indexable"), 30, 63),
                ("NX-E1303", None, 56, 61),
                ("NX-E0102", Some("type_mismatch"), 66, 68),
            ],
        ),
        (
            "array_minus_binding",
            include_str!("fixtures/i07c/array_minus_same_deferred_binding.n"),
            &[("NX-E0102", Some("type_mismatch"), 28, 30)],
        ),
        (
            "array_and_condition",
            include_str!("fixtures/i07c/deferred_array_and_condition_independent_sites.n"),
            &[
                ("NX-E0102", Some("type_mismatch"), 52, 56),
                ("NX-E0102", Some("type_mismatch"), 64, 68),
                ("NX-E0102", Some("type_mismatch"), 77, 78),
            ],
        ),
    ];
    for (name, source, expected) in cases {
        let file = format!("{name}.n");
        let result = check_proposed(&file, source, false);
        if expected.is_empty() {
            assert!(result.is_ok(), "{name}: {result:?}");
            assert!(check_scalar(&file, source).is_ok(), "{name}");
            continue;
        }
        let errors = result.unwrap_err();
        let actual: Vec<_> = errors
            .iter()
            .map(|d| {
                let span = d.primary_span.as_ref().unwrap();
                (
                    d.code.as_str(),
                    d.subcause.as_deref(),
                    span.start as usize,
                    span.end as usize,
                )
            })
            .collect();
        assert_eq!(&actual, expected, "{name}");
        for error in &errors {
            error.validate(&file, source).unwrap();
        }
        assert!(
            matches!(check_scalar(&file, source), Err(CompileError::Rejected(_))),
            "{name}"
        );
    }
}

#[test]
fn contaminated_base_keeps_independent_selector_type_error() {
    // F0-0002 §§4.1.2.3, 4.1.2.5 and 13.9: a failed condition does not
    // suppress the selector's own I64 requirement.
    type Site = (u64, u64, &'static str, &'static str);
    type Case = (&'static str, &'static [Site]);
    let cases: &[Case] = &[
        (
            "fn Main(){let v2=(if [true] then [[1]] else [[1],[2]]);let a=v2[[false,true]];}",
            &[
                (21, 27, "Bool", "Array<Bool>"),
                (64, 76, "I64", "Array<Bool>"),
            ],
        ),
        (
            "fn Main(){let v2=(if true then [[1]] else [[1],[2]]);let a=v2[[false,true]];}",
            &[(62, 74, "I64", "Array<Bool>")],
        ),
        (
            "fn Main(){let v2=(if [true] then [[1]] else [[1],[2]]);let a=v2[0];}",
            &[(21, 27, "Bool", "Array<Bool>")],
        ),
    ];
    for (source, expected) in cases {
        let errors = check_proposed("selector_independent.n", source, false).unwrap_err();
        assert_eq!(errors.len(), expected.len(), "{source}: {errors:?}");
        for (error, &(start, end, required, found)) in errors.iter().zip(*expected) {
            assert_eq!(error.code, "NX-E0102");
            assert_eq!(error.subcause.as_deref(), Some("type_mismatch"));
            assert_eq!(error.stage, Stage::Types);
            assert_eq!(error.operation, Operation::TypesCheck);
            assert_eq!(
                error
                    .primary_span
                    .as_ref()
                    .map(|span| (span.start, span.end)),
                Some((start, end))
            );
            assert_eq!(
                error.data.get("expected_type"),
                Some(&DataValue::String(required.into()))
            );
            assert_eq!(
                error.data.get("found_type"),
                Some(&DataValue::String(found.into()))
            );
            assert_eq!(error.related_spans.len(), 1);
            assert_eq!(error.related_spans[0].role, RelatedRole::FoundType);
            assert_eq!(
                (
                    error.related_spans[0].span.start,
                    error.related_spans[0].span.end
                ),
                (start, end)
            );
            error.validate("selector_independent.n", source).unwrap();
        }
        assert!(matches!(
            check_scalar("selector_independent.n", source),
            Err(CompileError::Rejected(_))
        ));
    }
    let valid = "fn Main(){let v2=(if true then [[1]] else [[1],[2]]);let a=v2[0];}";
    assert!(check_proposed("selector_independent.n", valid, false).is_ok());
    assert!(check_scalar("selector_independent.n", valid).is_ok());
}

#[test]
fn informative_consumer_fixtures_keep_their_independent_causes() {
    // ADR §4.1.2.5 permits extra V at these consumers; the source causes
    // below are stable and each fixture remains an executable program.
    let cases: &[(&str, &str, &[Expected])] = &[
        (
            "failed_join",
            include_str!("fixtures/i07c/failed_join_later_consumer.n"),
            &[("NX-E1303", None, 44, 48)],
        ),
        (
            "invalid_union",
            include_str!("fixtures/i07c/invalid_union_less_than.n"),
            &[
                ("NX-E0102", Some("type_mismatch"), 20, 21),
                ("NX-E1303", None, 61, 62),
            ],
        ),
        (
            "nested_condition",
            include_str!("fixtures/i07c/nested_tainted_condition_later_consumer.n"),
            &[("NX-E0102", Some("type_mismatch"), 34, 35)],
        ),
        (
            "tainted_branch",
            include_str!("fixtures/i07c/tainted_branch_later_consumer.n"),
            &[("NX-E0102", Some("type_mismatch"), 20, 21)],
        ),
    ];
    for (name, source, expected) in cases {
        let file = format!("{name}.n");
        let errors = check_proposed(&file, source, false).unwrap_err();
        for item in *expected {
            assert!(
                errors.iter().any(|d| {
                    let span = d.primary_span.as_ref().unwrap();
                    (
                        d.code.as_str(),
                        d.subcause.as_deref(),
                        span.start as usize,
                        span.end as usize,
                    ) == *item
                }),
                "{name}: {errors:?}"
            );
        }
        for error in &errors {
            error.validate(&file, source).unwrap();
        }
        assert!(matches!(
            check_scalar(&file, source),
            Err(CompileError::Rejected(_))
        ));
    }
}

#[test]
fn independent_causal_sites_survive_deferred_resolution() {
    // Each expected result follows F0-0002 §§4.1.2.2–5 and
    // 13.9. Every case is a source program, including the invalid bindings
    // whose errors must not hide a separate causal site.
    let cases: &[(&str, &str, &[Expected])] = &[
        (
            "comparison_operands",
            "fn Main(){AssertEq(1<true,2<true);}",
            &[
                ("NX-E0102", Some("type_mismatch"), 21, 25),
                ("NX-E0102", Some("type_mismatch"), 28, 32),
            ],
        ),
        (
            "branch_and_selector",
            "fn Main(){let v=(if true then [false] else [true]);let w=((if 18 then 29 else 1)+v[0]);}",
            &[
                ("NX-E0102", Some("type_mismatch"), 62, 64),
                ("NX-E0102", Some("type_mismatch"), 81, 85),
            ],
        ),
        (
            "condition_and_independent_numeric_operand",
            "fn Main(){var w=DV;let a0=(if w then w[0] else 7)+true;}",
            &[
                ("NX-E0102", Some("not_indexable"), 74, 75),
                ("NX-E0102", Some("type_mismatch"), 87, 91),
            ],
        ),
        (
            "two_unresolved_constructors",
            "fn Main(){var a=[];AssertEq([[]],a);AssertEq(a,true);var b=[];AssertEq(b,true);}",
            &[("NX-E0103", None, 29, 31), ("NX-E0103", None, 59, 61)],
        ),
        (
            "unresolved_equality_constructor_before_lowering",
            "fn Main(){var r=[];let a0:Array<I64>=[]==(r[r]);r=[true];}",
            &[
                ("NX-E0103", None, 37, 39),
                ("NX-E0102", Some("type_mismatch"), 44, 45),
            ],
        ),
        (
            "unresolved_constructor_before_join",
            "fn F1(){var v=[];let w=(if true then [5] else (if true then true else 3));}fn Main(){}",
            &[("NX-E0103", None, 14, 16), ("NX-E1303", None, 60, 64)],
        ),
        (
            "unresolved_element_of_assignment",
            "fn Main(){var q1=DV;var q2=[[]];var q3=DV;AssertEq([true],q1);q1=q2;}",
            &[("NX-E0103", None, 65, 67)],
        ),
        (
            "indexed_assignment",
            include_str!("fixtures/i07c/indexed_assignment_keeps_independent_errors.n"),
            &[
                ("NX-E0102", Some("type_mismatch"), 83, 86),
                ("NX-E0102", Some("type_mismatch"), 100, 103),
                ("NX-E0102", Some("type_mismatch"), 119, 133),
            ],
        ),
        (
            "nested_constructor",
            include_str!("fixtures/i07c/unresolved_nested_constructor_keeps_both_sites.n"),
            &[("NX-E0103", None, 28, 30), ("NX-E0103", None, 73, 77)],
        ),
        (
            "code_of_array_equality",
            "fn Main(){var w=DV;let a0=[w,[1]==w];}",
            &[("NX-E0102", Some("type_mismatch"), 66, 72)],
        ),
        (
            "constructor_after_invalid_len",
            "fn Main(){var w=DV;AssertEq((if true then Len(true) else [w]),[]);w=[[3]];}",
            &[
                ("NX-E0102", Some("not_indexable"), 83, 87),
                ("NX-E0103", None, 99, 101),
            ],
        ),
        (
            "later_assignment_and_array_element",
            "fn Main(){let ar=[1];var q=DV;Print([q[0],[ar]]);let a1:Array<I64>=q;q=[true];}",
            &[
                ("NX-E0102", Some("type_mismatch"), 74, 75),
                ("NX-E0102", Some("type_mismatch"), 109, 113),
            ],
        ),
        (
            "deferred_selector_condition",
            "fn Main(){var q=DV;let a=q[(if true then 1 else true)];q=[3];}",
            &[("NX-E0102", Some("type_mismatch"), 85, 89)],
        ),
        (
            "binary_operand_and_later_assignment",
            "fn Main(){var q=DV;let a=true+q;q=true;}",
            &[
                ("NX-E0102", Some("type_mismatch"), 62, 66),
                ("NX-E0102", Some("type_mismatch"), 71, 75),
            ],
        ),
    ];
    for (name, source, expected) in cases {
        let source = source.replace("DV", "(if true then {return;} else {return;})");
        let file = format!("{name}.n");
        let errors = check_proposed(&file, &source, false).unwrap_err();
        let actual: Vec<_> = errors
            .iter()
            .map(|d| {
                let span = d.primary_span.as_ref().unwrap();
                (
                    d.code.as_str(),
                    d.subcause.as_deref(),
                    span.start as usize,
                    span.end as usize,
                )
            })
            .collect();
        assert_eq!(&actual, expected, "{name}: {source}");
        for error in &errors {
            error.validate(&file, &source).unwrap();
        }
        assert!(
            matches!(check_scalar(&file, &source), Err(CompileError::Rejected(_))),
            "{name}"
        );
    }
}

#[test]
fn complex_condition_and_selector_keep_three_independent_c_sites() {
    // F0-0002 §§4.1.2.3–5 and 13.9: the three incompatible operands are
    // independent. Additional dependent V diagnostics do not affect this test.
    let source = "fn Main(){let v2=(if (if false then [[1],[2]] else {value:[[1]];}) then ((if false then 16 else {value:29;})) else (if 17 then [false,true] else {value:[true];})[[false]]);}";
    let errors = check_proposed("complex_sites.n", source, false).unwrap_err();
    for (start, end) in [(36, 45), (58, 63), (119, 121)] {
        assert!(
            errors.iter().any(|diagnostic| {
                let span = diagnostic.primary_span.as_ref().unwrap();
                diagnostic.code == "NX-E0102"
                    && diagnostic.subcause.as_deref() == Some("type_mismatch")
                    && span.start as usize == start
                    && span.end as usize == end
            }),
            "missing C at {start}-{end}: {errors:?}"
        );
    }
    for diagnostic in &errors {
        diagnostic.validate("complex_sites.n", source).unwrap();
    }
    assert!(matches!(
        check_scalar("complex_sites.n", source),
        Err(CompileError::Rejected(_))
    ));
    let valid = "fn Main(){let v2=(if true then [[1],[2]] else [[1]]);let a=v2[0];}";
    assert!(check_proposed("complex_sites_valid.n", valid, false).is_ok());
    assert!(check_scalar("complex_sites_valid.n", valid).is_ok());
}

#[test]
fn later_assignment_resolves_empty_array_element() {
    // F0-0002 §4.1.1/TD: [] establishes Array<α>; the later compatible
    // assignment determines α without moving a conflict to the constructor.
    let valid = "fn Main(){var r=[];r=[3];}";
    assert!(check_proposed("array_element_resolved.n", valid, false).is_ok());
    assert!(check_scalar("array_element_resolved.n", valid).is_ok());
    let invalid = "fn Main(){var r=[];r=[3];r=true;}";
    let errors = check_proposed("array_shape_conflict.n", invalid, false).unwrap_err();
    assert_eq!(errors.len(), 1, "{errors:?}");
    assert_eq!(errors[0].code, "NX-E0102");
    assert_eq!(errors[0].subcause.as_deref(), Some("type_mismatch"));
    errors[0]
        .validate("array_shape_conflict.n", invalid)
        .unwrap();
}
