use nexo::check::check_proposed;
use nexo::diagnostic::{DataValue, Operation, RelatedRole, Stage};
use nexo::ir::{CompileError, check_scalar};

#[test]
fn indexed_write_preserves_expected_and_found_orientation() {
    // F0-0002 §§4.1.2.2, 13.15: the RHS equality supplies Bool before
    // the unresolved indexed destination supplies its I64 element.
    let source = "fn Main(){let c=true;let ar=[1];var t1=(if c then {return;} else {return;});t1[1]=ar==t1;}";
    let errors = check_proposed("indexed_write_orientation.n", source, false).unwrap_err();
    assert_eq!(errors.len(), 1, "{errors:?}");
    let error = &errors[0];
    error
        .validate("indexed_write_orientation.n", source)
        .unwrap();
    assert_eq!(error.code, "NX-E0102");
    assert_eq!(error.subcause.as_deref(), Some("type_mismatch"));
    assert_eq!(error.stage, Stage::Types);
    assert_eq!(error.operation, Operation::TypesCheck);
    assert_eq!(
        error.primary_span.as_ref().map(|s| (s.start, s.end)),
        Some((76, 78))
    );
    assert_eq!(error.data.len(), 2);
    assert_eq!(
        error.data.get("expected_type"),
        Some(&DataValue::String("Bool".into()))
    );
    assert_eq!(
        error.data.get("found_type"),
        Some(&DataValue::String("I64".into()))
    );
    assert_eq!(error.related_spans.len(), 2);
    assert!(
        error
            .related_spans
            .iter()
            .any(|s| s.role == RelatedRole::ExpectedType && (s.span.start, s.span.end) == (82, 88))
    );
    assert!(
        error
            .related_spans
            .iter()
            .any(|s| s.role == RelatedRole::FoundType && (s.span.start, s.span.end) == (76, 78))
    );
    assert!(matches!(
        check_scalar("indexed_write_orientation.n", source),
        Err(CompileError::Rejected(_))
    ));

    let repaired = "fn Main(){let c=true;let ar=[1];var t1=(if c then {return;} else {return;});t1[1]=ar==ar;t1=[true];}";
    assert!(check_proposed("indexed_write_orientation_valid.n", repaired, false).is_ok());
    assert!(check_scalar("indexed_write_orientation_valid.n", repaired).is_ok());
}

#[test]
fn independent_less_than_domain_keeps_intrinsic_origin() {
    // F0-0002 §§4.1.2.4–5, 13.15: when the right operand fails, the
    // earlier left operand retains an intrinsic I64 constraint. A later
    // conflicting assignment has no source `expected_type` relation.
    let source =
        "fn Main(){var q=(if true then {return;} else {return;});let y=q<(true+1);q=true;}";
    let errors = check_proposed("intrinsic_less_domain.n", source, false).unwrap_err();
    assert_eq!(errors.len(), 2, "{errors:?}");
    for error in &errors {
        error.validate("intrinsic_less_domain.n", source).unwrap();
        assert_eq!(error.code, "NX-E0102");
        assert_eq!(error.subcause.as_deref(), Some("type_mismatch"));
        assert_eq!(error.stage, Stage::Types);
        assert_eq!(error.operation, Operation::TypesCheck);
        assert_eq!(error.data.len(), 2);
        assert_eq!(
            error.data.get("expected_type"),
            Some(&DataValue::String("I64".into()))
        );
        assert_eq!(
            error.data.get("found_type"),
            Some(&DataValue::String("Bool".into()))
        );
        assert_eq!(error.related_spans.len(), 1);
        assert_eq!(error.related_spans[0].role, RelatedRole::FoundType);
    }
    assert_eq!(
        errors[0].primary_span.as_ref().map(|s| (s.start, s.end)),
        Some((65, 69))
    );
    assert_eq!(
        errors[1].primary_span.as_ref().map(|s| (s.start, s.end)),
        Some((75, 79))
    );
    assert!(matches!(
        check_scalar("intrinsic_less_domain.n", source),
        Err(CompileError::Rejected(_))
    ));
    let repaired = "fn Main(){var q=(if true then {return;} else {return;});let y=q<(1+1);q=1;}";
    assert!(check_proposed("intrinsic_less_domain_valid.n", repaired, false).is_ok());
    assert!(check_scalar("intrinsic_less_domain_valid.n", repaired).is_ok());
}

#[test]
fn independent_numeric_rhs_keeps_intrinsic_domain_origin() {
    // F0-0002 §§4.1.2.4–5, 13.15: after the left numeric operand fails,
    // the unresolved right operand acquires I64 intrinsically. Its later
    // conflicting assignment has no source expected_type relation.
    let source = "fn Main(){var q=(if true then {return;} else {return;});let z=true+q;q=true;}";
    let errors = check_proposed("intrinsic_numeric_rhs.n", source, false).unwrap_err();
    assert_eq!(errors.len(), 2, "{errors:?}");
    for (error, primary) in errors.iter().zip([(62, 66), (71, 75)]) {
        error.validate("intrinsic_numeric_rhs.n", source).unwrap();
        assert_eq!(error.code, "NX-E0102");
        assert_eq!(error.subcause.as_deref(), Some("type_mismatch"));
        assert_eq!(error.stage, Stage::Types);
        assert_eq!(error.operation, Operation::TypesCheck);
        assert_eq!(
            error
                .primary_span
                .as_ref()
                .map(|span| (span.start, span.end)),
            Some(primary)
        );
        assert_eq!(error.data.len(), 2);
        assert_eq!(
            error.data.get("expected_type"),
            Some(&DataValue::String("I64".into()))
        );
        assert_eq!(
            error.data.get("found_type"),
            Some(&DataValue::String("Bool".into()))
        );
        assert_eq!(error.related_spans.len(), 1);
        assert_eq!(error.related_spans[0].role, RelatedRole::FoundType);
        assert_eq!(&source[primary.0 as usize..primary.1 as usize], "true");
    }
    assert!(matches!(
        check_scalar("intrinsic_numeric_rhs.n", source),
        Err(CompileError::Rejected(_))
    ));
    let valid = "fn Main(){var q=(if true then {return;} else {return;});let z=1+q;q=1;}";
    assert!(check_proposed("intrinsic_numeric_rhs_valid.n", valid, false).is_ok());
    assert!(check_scalar("intrinsic_numeric_rhs_valid.n", valid).is_ok());
}

#[test]
fn projected_comparison_preserves_expected_operand_origin() {
    // F0-0002 §§4.1.2.4–5, 13.15: the concrete left operand supplies the
    // expected I64 origin when the right projection resolves to an array.
    let source = "fn Main(){var q=(if true then {return;} else {return;});let a=1<q[0];q=[[1]];}";
    let errors = check_proposed("projected_compare_origin.n", source, false).unwrap_err();
    assert_eq!(errors.len(), 1, "{errors:?}");
    let error = &errors[0];
    error
        .validate("projected_compare_origin.n", source)
        .unwrap();
    assert_eq!(error.code, "NX-E0102");
    assert_eq!(error.subcause.as_deref(), Some("type_mismatch"));
    assert_eq!(error.stage, Stage::Types);
    assert_eq!(error.operation, Operation::TypesCheck);
    assert_eq!(
        error.primary_span.as_ref().map(|s| (s.start, s.end)),
        Some((64, 68))
    );
    assert_eq!(&source[64..68], "q[0]");
    assert_eq!(&source[62..63], "1");
    assert_eq!(error.data.len(), 2);
    assert_eq!(
        error.data.get("expected_type"),
        Some(&DataValue::String("I64".into()))
    );
    assert_eq!(
        error.data.get("found_type"),
        Some(&DataValue::String("Array<I64>".into()))
    );
    assert_eq!(error.related_spans.len(), 2);
    assert!(
        error
            .related_spans
            .iter()
            .any(|s| s.role == RelatedRole::ExpectedType && (s.span.start, s.span.end) == (62, 63))
    );
    assert!(
        error
            .related_spans
            .iter()
            .any(|s| s.role == RelatedRole::FoundType && (s.span.start, s.span.end) == (64, 68))
    );
    assert!(matches!(
        check_scalar("projected_compare_origin.n", source),
        Err(CompileError::Rejected(_))
    ));

    let valid = "fn Main(){var q=(if true then {return;} else {return;});let a=1<q[0];q=[1];}";
    assert!(check_proposed("projected_compare_origin_valid.n", valid, false).is_ok());
    assert!(check_scalar("projected_compare_origin_valid.n", valid).is_ok());
}

#[test]
fn projected_comparison_after_prior_binding_keeps_expected_source_span() {
    // F0-0002 §§4.1.2.4–5, 13.15: a prior independent binding must not
    // erase the expected operand's source relation at a pending projection.
    let source = "fn Main(){let census8=1; var q2=(if true then {return;} else {return; });let a5=1 < q2[0];q2=[[1]];}";
    let errors = check_proposed("projected_compare_prior_binding.n", source, false).unwrap_err();
    assert_eq!(errors.len(), 1, "{errors:?}");
    let error = &errors[0];
    error
        .validate("projected_compare_prior_binding.n", source)
        .unwrap();
    assert_eq!(error.code, "NX-E0102");
    assert_eq!(error.subcause.as_deref(), Some("type_mismatch"));
    assert_eq!(error.stage, Stage::Types);
    assert_eq!(error.operation, Operation::TypesCheck);
    assert_eq!(
        error.primary_span.as_ref().map(|s| (s.start, s.end)),
        Some((84, 89))
    );
    assert_eq!(&source[80..81], "1");
    assert_eq!(&source[84..89], "q2[0]");
    assert_eq!(error.data.len(), 2);
    assert_eq!(
        error.data.get("expected_type"),
        Some(&DataValue::String("I64".into()))
    );
    assert_eq!(
        error.data.get("found_type"),
        Some(&DataValue::String("Array<I64>".into()))
    );
    assert_eq!(error.related_spans.len(), 2);
    assert!(
        error
            .related_spans
            .iter()
            .any(|s| s.role == RelatedRole::ExpectedType && (s.span.start, s.span.end) == (80, 81))
    );
    assert!(
        error
            .related_spans
            .iter()
            .any(|s| s.role == RelatedRole::FoundType && (s.span.start, s.span.end) == (84, 89))
    );
    assert!(matches!(
        check_scalar("projected_compare_prior_binding.n", source),
        Err(CompileError::Rejected(_))
    ));

    let valid = "fn Main(){let census8=1; var q2=(if true then {return;} else {return; });let a5=1 < q2[0];q2=[1];}";
    assert!(check_proposed("projected_compare_prior_binding_valid.n", valid, false).is_ok());
    assert!(check_scalar("projected_compare_prior_binding_valid.n", valid).is_ok());
}

#[test]
fn array_equality_keeps_the_complete_boolean_result_as_found_type() {
    // F0-0002 §§4.1.2.2, 4.1.2.6, 13.15: the equality fixes w as
    // Array<I64>, but its own result remains Bool in the outer array.
    let source =
        "fn Main(){var w=(if true then {return;} else {return;});let a0=[w,[1]==w];let nonce=1;}";
    let errors = check_proposed("array_equality_result_type.n", source, false).unwrap_err();
    assert_eq!(errors.len(), 1, "{errors:?}");
    let error = &errors[0];
    error
        .validate("array_equality_result_type.n", source)
        .unwrap();
    assert_eq!(error.code, "NX-E0102");
    assert_eq!(error.subcause.as_deref(), Some("type_mismatch"));
    assert_eq!(error.stage, Stage::Types);
    assert_eq!(error.operation, Operation::TypesCheck);
    assert_eq!(
        error.primary_span.as_ref().map(|s| (s.start, s.end)),
        Some((66, 72))
    );
    assert_eq!(&source[64..65], "w");
    assert_eq!(&source[66..69], "[1]");
    assert_eq!(&source[66..72], "[1]==w");
    assert_eq!(error.data.len(), 2);
    assert_eq!(
        error.data.get("expected_type"),
        Some(&DataValue::String("Array<I64>".into()))
    );
    assert_eq!(
        error.data.get("found_type"),
        Some(&DataValue::String("Bool".into()))
    );
    assert_eq!(error.related_spans.len(), 2);
    assert!(
        error
            .related_spans
            .iter()
            .any(|s| s.role == RelatedRole::ExpectedType && (s.span.start, s.span.end) == (66, 69))
    );
    assert!(
        error
            .related_spans
            .iter()
            .any(|s| s.role == RelatedRole::FoundType && (s.span.start, s.span.end) == (66, 72))
    );
    assert!(matches!(
        check_scalar("array_equality_result_type.n", source),
        Err(CompileError::Rejected(_))
    ));

    let valid = "fn Main(){var w=(if true then {return;} else {return;});let a0=[w,[1]];w=[1];}";
    assert!(check_proposed("array_equality_result_type_valid.n", valid, false).is_ok());
    assert!(check_scalar("array_equality_result_type_valid.n", valid).is_ok());
}

#[test]
fn array_assignment_keeps_first_element_origin_after_conflict() {
    // F0-0002 §§4.1.2.2, 13.15: the first concrete array element remains
    // the expected source after another element and an indexed write fail.
    let source = "fn Main(){let x=1;var q=[];q=[[1],[true]];q[0][0]=[x];q=[[1]];}";
    let errors = check_proposed("array_first_origin.n", source, false).unwrap_err();
    assert_eq!(errors.len(), 2, "{errors:?}");
    for error in &errors {
        error.validate("array_first_origin.n", source).unwrap();
        assert_eq!(error.code, "NX-E0102");
        assert_eq!(error.subcause.as_deref(), Some("type_mismatch"));
        assert_eq!(error.stage, Stage::Types);
        assert_eq!(error.operation, Operation::TypesCheck);
        assert_eq!(error.data.len(), 2);
        assert_eq!(
            error.data.get("expected_type"),
            Some(&DataValue::String("I64".into()))
        );
    }
    assert_eq!(
        errors[0].primary_span.as_ref().map(|s| (s.start, s.end)),
        Some((35, 39))
    );
    assert_eq!(&source[35..39], "true");
    assert_eq!(&source[31..32], "1");
    assert_eq!(&source[50..53], "[x]");
    assert_eq!(
        errors[0].data.get("found_type"),
        Some(&DataValue::String("Bool".into()))
    );
    assert_eq!(errors[0].related_spans.len(), 1);
    assert_eq!(errors[0].related_spans[0].role, RelatedRole::FoundType);
    assert_eq!(
        (
            errors[0].related_spans[0].span.start,
            errors[0].related_spans[0].span.end
        ),
        (35, 39)
    );
    assert_eq!(
        errors[1].primary_span.as_ref().map(|s| (s.start, s.end)),
        Some((50, 53))
    );
    assert_eq!(
        errors[1].data.get("found_type"),
        Some(&DataValue::String("Array<I64>".into()))
    );
    assert_eq!(errors[1].related_spans.len(), 2);
    assert!(
        errors[1]
            .related_spans
            .iter()
            .any(|s| s.role == RelatedRole::ExpectedType && (s.span.start, s.span.end) == (31, 32))
    );
    assert!(
        errors[1]
            .related_spans
            .iter()
            .any(|s| s.role == RelatedRole::FoundType && (s.span.start, s.span.end) == (50, 53))
    );
    assert!(matches!(
        check_scalar("array_first_origin.n", source),
        Err(CompileError::Rejected(_))
    ));

    let valid = "fn Main(){let x=1;var q=[];q=[[1]];q[0][0]=x;q=[[1]];}";
    assert!(check_proposed("array_first_origin_valid.n", valid, false).is_ok());
    assert!(check_scalar("array_first_origin_valid.n", valid).is_ok());
}

#[test]
fn independent_numeric_error_has_no_inferred_expected_origin() {
    // F0-0002 §§4.1.2.4–5, 13.15: an intrinsic I64 domain failure in a
    // checked branch has no source expected_type witness from the plan.
    let source = "fn Main(){let c=true;var q=[];if false {let a=[q,true];q=[1,true];let b=c+[q];}AssertEq(q,[1]);}";
    let errors = check_proposed("numeric_plan_origin.n", source, false).unwrap_err();
    assert_eq!(errors.len(), 3, "{errors:?}");
    for error in &errors {
        error.validate("numeric_plan_origin.n", source).unwrap();
        assert_eq!(error.code, "NX-E0102");
        assert_eq!(error.subcause.as_deref(), Some("type_mismatch"));
        assert_eq!(error.stage, Stage::Types);
        assert_eq!(error.operation, Operation::TypesCheck);
        assert_eq!(error.data.len(), 2);
    }
    assert_eq!(
        errors[0].primary_span.as_ref().map(|s| (s.start, s.end)),
        Some((49, 53))
    );
    assert_eq!(&source[47..48], "q");
    assert_eq!(&source[49..53], "true");
    assert_eq!(&source[60..64], "true");
    assert_eq!(&source[72..73], "c");
    assert_eq!(
        errors[0].data.get("expected_type"),
        Some(&DataValue::String("Array<I64>".into()))
    );
    assert_eq!(
        errors[0].data.get("found_type"),
        Some(&DataValue::String("Bool".into()))
    );
    assert_eq!(errors[0].related_spans.len(), 2);
    assert!(
        errors[0]
            .related_spans
            .iter()
            .any(|s| s.role == RelatedRole::ExpectedType && (s.span.start, s.span.end) == (47, 48))
    );
    assert!(
        errors[0]
            .related_spans
            .iter()
            .any(|s| s.role == RelatedRole::FoundType && (s.span.start, s.span.end) == (49, 53))
    );
    for (error, primary) in errors[1..].iter().zip([(60, 64), (72, 73)]) {
        assert_eq!(
            error.primary_span.as_ref().map(|s| (s.start, s.end)),
            Some(primary)
        );
        assert_eq!(
            error.data.get("expected_type"),
            Some(&DataValue::String("I64".into()))
        );
        assert_eq!(
            error.data.get("found_type"),
            Some(&DataValue::String("Bool".into()))
        );
        assert_eq!(error.related_spans.len(), 1);
        assert_eq!(error.related_spans[0].role, RelatedRole::FoundType);
        assert_eq!(
            (
                error.related_spans[0].span.start,
                error.related_spans[0].span.end
            ),
            primary
        );
    }
    assert!(matches!(
        check_scalar("numeric_plan_origin.n", source),
        Err(CompileError::Rejected(_))
    ));

    let valid = "fn Main(){let c=1;var q=[];if false {let a=[q,[1]];q=[1,2];let b=c+1;}q=[1];AssertEq(q,[1]);}";
    assert!(check_proposed("numeric_plan_origin_valid.n", valid, false).is_ok());
    assert!(check_scalar("numeric_plan_origin_valid.n", valid).is_ok());
}

#[test]
fn cyclic_constructor_errors_keep_inference_code_at_each_site() {
    // F0-0002 §4.1.2.2: an unresolved element owns NX-E0103 even when a
    // later equality links its constructor to another unresolved array.
    let source = "fn Main(){var m1=[];AssertEq([true],[m1[7],m1]);}";
    let errors = check_proposed("cyclic_constructor.n", source, false).unwrap_err();
    let sites: Vec<_> = errors
        .iter()
        .map(|error| {
            error.validate("cyclic_constructor.n", source).unwrap();
            assert_eq!(error.stage, Stage::Types);
            assert_eq!(error.operation, Operation::TypesInfer);
            assert_eq!(error.subcause, None);
            assert!(error.data.is_empty());
            assert!(error.related_spans.is_empty());
            (
                error.code.as_str(),
                error.primary_span.as_ref().map(|s| (s.start, s.end)),
            )
        })
        .collect();
    assert_eq!(
        sites,
        [
            ("NX-E0103", Some((17, 19))),
            ("NX-E0103", Some((36, 46))),
            ("NX-E0103", Some((43, 45)))
        ]
    );
    assert!(matches!(
        check_scalar("cyclic_constructor.n", source),
        Err(CompileError::Rejected(_))
    ));
    let repaired = "fn Main(){var m1=[true];AssertEq([true],[m1[0],true]);}";
    assert!(check_proposed("cyclic_constructor_valid.n", repaired, false).is_ok());
    assert!(check_scalar("cyclic_constructor_valid.n", repaired).is_ok());
}

#[test]
fn linked_array_constructors_keep_each_causal_inference_site() {
    // F0-0002 §§4.1.2.2, 13.9: sharing a deferred root cannot create or
    // erase the inference obligation owned by a source constructor.
    for (source, expected) in [
        (
            "fn Main(){let c=true;var k0=[(if true then {return;} else {return;})];var k2=k0;var k3=[];var k4=(if c then {return;} else {return;});k0=[k3];let w2=[k2,k2];let w8=(if c then k3 else k4);k0[0]=k0;}",
            &[(87, 89)][..],
        ),
        (
            "fn Main(){var k0=[[]];var k1=[(if false then {return;} else {return;})];var k2=[[]];k0=[k2];AssertEq(k1,k2);let w2=[k1,k0];let w5=[k0,k0];}",
            &[(80, 82), (130, 137)][..],
        ),
    ] {
        let errors = check_proposed("linked_constructors.n", source, false).unwrap_err();
        let sites: Vec<_> = errors
            .iter()
            .map(|error| {
                error.validate("linked_constructors.n", source).unwrap();
                assert_eq!(error.code, "NX-E0103");
                assert_eq!(error.subcause, None);
                assert_eq!(error.operation, Operation::TypesInfer);
                assert!(error.data.is_empty());
                assert!(error.related_spans.is_empty());
                let span = error.primary_span.as_ref().unwrap();
                (span.start, span.end)
            })
            .collect();
        assert_eq!(sites, expected, "{source}");
        assert!(matches!(
            check_scalar("linked_constructors.n", source),
            Err(CompileError::Rejected(_))
        ));
    }
    let repaired = "fn Main(){var k0=[[1]];let w2=[k0,k0];}";
    assert!(check_proposed("linked_constructors_valid.n", repaired, false).is_ok());
    assert!(check_scalar("linked_constructors_valid.n", repaired).is_ok());
}

#[test]
fn invalid_nested_selector_preserves_independent_len_domain() {
    // F0-0002 §§4.1.2.2–5: the unresolved inner [] and Len(false) own
    // independent causes even when the enclosing union cannot supply a type.
    let source = "fn Main(){let c=true;let ar=[1];let z=((if c then [[]] else ar))[Len(false)];}";
    let errors = check_proposed("nested_selector_domain.n", source, false).unwrap_err();
    assert_eq!(errors.len(), 2, "{errors:?}");
    errors[0]
        .validate("nested_selector_domain.n", source)
        .unwrap();
    assert_eq!(errors[0].code, "NX-E0103");
    assert_eq!(errors[0].operation, Operation::TypesInfer);
    assert_eq!(
        errors[0].primary_span.as_ref().map(|s| (s.start, s.end)),
        Some((51, 53))
    );
    assert_eq!(errors[0].subcause, None);
    assert!(errors[0].data.is_empty());
    errors[1]
        .validate("nested_selector_domain.n", source)
        .unwrap();
    assert_eq!(errors[1].code, "NX-E0102");
    assert_eq!(errors[1].stage, Stage::Types);
    assert_eq!(errors[1].operation, Operation::TypesCheck);
    assert_eq!(errors[1].subcause.as_deref(), Some("not_indexable"));
    assert_eq!(
        errors[1].primary_span.as_ref().map(|s| (s.start, s.end)),
        Some((69, 74))
    );
    assert_eq!(errors[1].data.len(), 2);
    assert_eq!(
        errors[1].data.get("expected_type"),
        Some(&DataValue::String("domain:array".into()))
    );
    assert_eq!(
        errors[1].data.get("found_type"),
        Some(&DataValue::String("Bool".into()))
    );
    assert_eq!(errors[1].related_spans.len(), 1);
    assert_eq!(errors[1].related_spans[0].role, RelatedRole::FoundType);
    assert_eq!(
        (
            errors[1].related_spans[0].span.start,
            errors[1].related_spans[0].span.end
        ),
        (69, 74)
    );
    assert!(matches!(
        check_scalar("nested_selector_domain.n", source),
        Err(CompileError::Rejected(_))
    ));
    let repaired = "fn Main(){let c=true;let z=((if c then [[1]] else [[2]]))[Len([1])];}";
    assert!(check_proposed("nested_selector_domain_valid.n", repaired, false).is_ok());
    assert!(check_scalar("nested_selector_domain_valid.n", repaired).is_ok());
}

#[test]
fn failed_assignment_condition_keeps_prior_array_constructor_obligation() {
    // F0-0002 §4.1.2.2: a failed later RHS cannot discharge or erase the
    // independent unresolved element of the earlier [] constructor.
    let source = "fn Main(){var q=[];q=(if [1] then {value:2;} else 2);}";
    let errors =
        check_proposed("unresolved_array_after_bad_condition.n", source, false).unwrap_err();
    let actual: Vec<_> = errors
        .iter()
        .map(|d| {
            let span = d.primary_span.as_ref().unwrap();
            (d.code.as_str(), d.subcause.as_deref(), span.start, span.end)
        })
        .collect();
    assert_eq!(
        actual,
        [
            ("NX-E0103", None, 16, 18),
            ("NX-E0102", Some("type_mismatch"), 25, 28),
        ]
    );
    for error in &errors {
        error
            .validate("unresolved_array_after_bad_condition.n", source)
            .unwrap();
    }
    assert!(matches!(
        check_scalar("unresolved_array_after_bad_condition.n", source),
        Err(CompileError::Rejected(_))
    ));
    let repaired = "fn Main(){var q=[];q=(if true then {value:[2];} else [2]);}";
    assert!(check_proposed("resolved_array_after_good_condition.n", repaired, false).is_ok());
    assert!(check_scalar("resolved_array_after_good_condition.n", repaired).is_ok());
}

#[test]
fn invalid_assignment_target_preserves_independent_union_error() {
    // F0-0002 §§4.1.2.4–5 and 13.9: the condition of the initializer
    // invalidates w, while the later union still has two incompatible values.
    let source =
        "fn Main(){var w=(if 4 then {return;} else {return;});w=(if true then 42 else [1]);}";
    let errors = check_proposed("tainted_assignment.n", source, false).unwrap_err();
    assert_eq!(errors.len(), 2, "{errors:?}");
    assert_eq!(errors[0].code, "NX-E0102");
    assert_eq!(errors[0].subcause.as_deref(), Some("type_mismatch"));
    assert_eq!(
        errors[0].primary_span.as_ref().map(|s| (s.start, s.end)),
        Some((20, 21))
    );
    assert_eq!(errors[1].code, "NX-E1303");
    assert_eq!(errors[1].subcause, None);
    let incompatible = source.find("[1]").unwrap();
    assert_eq!(
        errors[1].primary_span.as_ref().map(|s| (s.start, s.end)),
        Some((incompatible as u64, incompatible as u64 + 3))
    );
    for error in &errors {
        error.validate("tainted_assignment.n", source).unwrap();
    }
    assert!(matches!(
        check_scalar("tainted_assignment.n", source),
        Err(CompileError::Rejected(_))
    ));
    let repaired =
        "fn Main(){var w=(if true then {return;} else {return;});w=(if true then 42 else 1);}";
    assert!(check_proposed("tainted_assignment_repaired.n", repaired, false).is_ok());
}

#[test]
fn known_array_constructor_preserves_comparison_equality_priority() {
    // F0-0002 §4.1.2.4: `<` compares the synthesized operand types before
    // checking the common I64 domain. `[]` already proves Len's Array domain.
    let source = "fn Main(){var br=[];let a=[1]<Len(br);br=[true];}";
    let errors = check_proposed("pending_len_binary.n", source, false).unwrap_err();
    let causal = errors
        .iter()
        .find(|error| {
            error
                .primary_span
                .as_ref()
                .map(|span| (span.start, span.end))
                == Some((30, 37))
        })
        .expect("cross-operand equality cause");
    assert_eq!(causal.code, "NX-E0102");
    assert_eq!(causal.subcause.as_deref(), Some("type_mismatch"));
    assert_eq!(causal.stage, Stage::Types);
    assert_eq!(causal.operation, Operation::TypesCheck);
    assert_eq!(causal.unexpected_token, None);
    assert!(causal.expected_tokens.is_empty());
    assert_eq!(causal.feature_id, None);
    assert!(causal.repair.is_empty());
    assert!(causal.cause_chain.is_empty());
    assert_eq!(
        causal
            .related_spans
            .iter()
            .map(|site| (site.role, site.span.start, site.span.end))
            .collect::<Vec<_>>(),
        vec![
            (RelatedRole::ExpectedType, 26, 29),
            (RelatedRole::FoundType, 30, 37),
        ]
    );
    assert_eq!(causal.data.len(), 2);
    assert_eq!(
        causal.data.get("expected_type"),
        Some(&DataValue::String("Array<I64>".into()))
    );
    assert_eq!(
        causal.data.get("found_type"),
        Some(&DataValue::String("I64".into()))
    );
    causal.validate("pending_len_binary.n", source).unwrap();
    assert!(matches!(
        check_scalar("pending_len_binary.n", source),
        Err(CompileError::Rejected(_))
    ));
    let isolated = "fn Main(){let a=[1]<1;}";
    assert!(check_proposed("isolated_binary.n", isolated, false).is_err());
    let repaired = "fn Main(){var br=[];let a=1<Len(br);br=[true];}";
    assert!(check_proposed("pending_len_binary_valid.n", repaired, false).is_ok());
    assert!(check_scalar("pending_len_binary_valid.n", repaired).is_ok());
}

#[test]
fn deferred_len_result_rejects_conflicting_annotation_before_lowering() {
    // F0-0002 §§4.1.2.1–2 and 13.9: a later Array element resolves Len's
    // pending domain, while the call result still conflicts with Array<Bool>.
    let source = "fn Main(){var a=[];let x:Array<Bool>=Len(a);a=[[1]];}";
    let errors = check_proposed("len_result.n", source, false).unwrap_err();
    assert_eq!(errors.len(), 1, "{errors:?}");
    let error = &errors[0];
    assert_eq!(error.code, "NX-E0102");
    assert_eq!(error.subcause.as_deref(), Some("type_mismatch"));
    assert_eq!(error.stage, Stage::Types);
    assert_eq!(error.operation, Operation::TypesCheck);
    assert_eq!(
        error
            .primary_span
            .as_ref()
            .map(|span| (span.start, span.end)),
        Some((37, 43))
    );
    assert_eq!(error.unexpected_token, None);
    assert!(error.expected_tokens.is_empty());
    assert_eq!(error.feature_id, None);
    assert!(error.repair.is_empty());
    assert!(error.cause_chain.is_empty());
    assert_eq!(
        error
            .related_spans
            .iter()
            .map(|site| (site.role, site.span.start, site.span.end))
            .collect::<Vec<_>>(),
        vec![
            (RelatedRole::ExpectedType, 25, 36),
            (RelatedRole::FoundType, 37, 43)
        ]
    );
    assert_eq!(error.data.len(), 2);
    assert_eq!(
        error.data.get("expected_type"),
        Some(&DataValue::String("Array<Bool>".into()))
    );
    assert_eq!(
        error.data.get("found_type"),
        Some(&DataValue::String("I64".into()))
    );
    error.validate("len_result.n", source).unwrap();
    assert!(matches!(
        check_scalar("len_result.n", source),
        Err(CompileError::Rejected(_))
    ));

    let repaired = "fn Main(){var a=[];let x:I64=Len(a);a=[[1]];}";
    assert!(check_proposed("len_result_valid.n", repaired, false).is_ok());
    assert!(check_scalar("len_result_valid.n", repaired).is_ok());
}

#[test]
fn unresolved_array_element_branch_conflict_rejects_before_lowering() {
    // F0-0002 §4.1.2.4–5: the Array element's deferred context reaches
    // both value branches; their incompatible leaves cannot reach lowering.
    let source = "fn Main(){var a1=[];var aar=[[1]];let a=[a1,[117],(if true then {value:aar;} else {value:false;})];}";
    let errors = check_proposed("array_branch.n", source, false).unwrap_err();
    assert_eq!(errors.len(), 2, "{errors:?}");
    for (error, (span, expected, found)) in errors.iter().zip([
        ((71, 74), "Array<I64>", "Array<Array<I64>>"),
        ((89, 94), "Array<I64>", "Bool"),
    ]) {
        assert_eq!(
            (error.code.as_str(), error.subcause.as_deref()),
            ("NX-E0102", Some("type_mismatch"))
        );
        assert_eq!(
            error.primary_span.as_ref().map(|s| (s.start, s.end)),
            Some(span)
        );
        assert_eq!(
            error.data.get("expected_type"),
            Some(&DataValue::String(expected.into()))
        );
        assert_eq!(
            error.data.get("found_type"),
            Some(&DataValue::String(found.into()))
        );
    }
    assert!(matches!(
        check_scalar("array_branch.n", source),
        Err(CompileError::Rejected(_))
    ));
    let valid =
        "fn Main(){var a1=[];let a=[a1,[117],(if true then {value:[2];} else {value:[3];})];}";
    assert!(check_proposed("array_branch_valid.n", valid, false).is_ok());
    assert!(check_scalar("array_branch_valid.n", valid).is_ok());
}

#[test]
fn failed_right_intrinsic_keeps_independent_left_comparison_domain() {
    // F0-0002 §§4.1.2.3–5, RD86/RD95: a bad Len argument invalidates
    // the right value; the left operand's domain remains independent.
    let dv = "(if true then{return;}else{return;})";
    for (right, assignment, expected) in [("Len(1)", "q=1", 1), ("Len(1)", "q=true", 2)] {
        let source = format!("fn Main(){{var q={dv};let t=q<{right};{assignment};}}");
        let errors = check_proposed("right_intrinsic.n", &source, false).unwrap_err();
        assert_eq!(errors.len(), expected, "{source}: {errors:?}");
        let len_arg = source.find("Len(1)").unwrap() as u64 + 4;
        let len_error = errors
            .iter()
            .find(|error| {
                error
                    .primary_span
                    .as_ref()
                    .is_some_and(|span| span.start == len_arg)
            })
            .expect("Len conserva su causa propia aunque la asignación posterior falle");
        assert_eq!(len_error.code, "NX-E0102");
        assert_eq!(len_error.subcause.as_deref(), Some("not_indexable"));
        assert_eq!(len_error.stage, Stage::Types);
        assert_eq!(len_error.operation, Operation::TypesCheck);
        assert_eq!(
            len_error.primary_span.as_ref().map(|s| (s.start, s.end)),
            Some((len_arg, len_arg + 1))
        );
        assert_eq!(
            len_error.data.get("expected_type"),
            Some(&DataValue::String("domain:array".into()))
        );
        assert_eq!(
            len_error.data.get("found_type"),
            Some(&DataValue::String("I64".into()))
        );
        assert_eq!(len_error.related_spans.len(), 1);
        assert_eq!(len_error.related_spans[0].role, RelatedRole::FoundType);
        assert_eq!(
            (
                len_error.related_spans[0].span.start,
                len_error.related_spans[0].span.end
            ),
            (len_arg, len_arg + 1)
        );
        if expected == 2 {
            let assigned = source.find("q=true").unwrap() as u64 + 2;
            let comparison = errors
                .iter()
                .find(|error| {
                    error
                        .primary_span
                        .as_ref()
                        .is_some_and(|span| span.start == assigned)
                })
                .expect("la asignación incompatible conserva su causa propia");
            assert_eq!(comparison.code, "NX-E0102");
            assert_eq!(comparison.subcause.as_deref(), Some("type_mismatch"));
            assert_eq!(comparison.stage, Stage::Types);
            assert_eq!(comparison.operation, Operation::TypesCheck);
            assert_eq!(
                comparison.primary_span.as_ref().map(|s| (s.start, s.end)),
                Some((assigned, assigned + 4))
            );
            assert_eq!(
                comparison.data.get("expected_type"),
                Some(&DataValue::String("I64".into()))
            );
            assert_eq!(
                comparison.data.get("found_type"),
                Some(&DataValue::String("Bool".into()))
            );
            assert_eq!(comparison.related_spans.len(), 1);
            assert_eq!(comparison.related_spans[0].role, RelatedRole::FoundType);
            assert_eq!(
                (
                    comparison.related_spans[0].span.start,
                    comparison.related_spans[0].span.end
                ),
                (assigned, assigned + 4)
            );
        }
    }
    let valid = format!("fn Main(){{var q={dv};let t=q<Len([1]);q=1;}}");
    assert!(check_proposed("right_intrinsic_valid.n", &valid, false).is_ok());
    assert!(check_scalar("right_intrinsic_valid.n", &valid).is_ok());
}

#[test]
fn right_assert_eq_failure_keeps_newly_determined_left_domain() {
    // F0-0002 §4.1.2.4–5, RD95/RD109: evaluating the right operand
    // determines q as a resource before its own domain fails. The outer
    // < still checks the independent left operand, and AssertEq reports
    // each argument separately.
    let dv = "(if true then{return;}else{return;})";
    let source = format!("fn Main(){{var q={dv};let r=MockOpen(1);let t=q<AssertEq(q,r);}}");
    let errors = check_proposed("right_assert_eq.n", &source, false).unwrap_err();
    assert_eq!(errors.len(), 3, "{errors:?}");
    let op = source.find("q<").unwrap() as u64 + 1;
    let args = source.find("AssertEq(q,r)").unwrap() as u64 + 9;
    for (error, start, expected) in [
        (&errors[0], op, "I64"),
        (&errors[1], args, "domain:copyable_data"),
        (&errors[2], args + 2, "domain:copyable_data"),
    ] {
        assert_eq!(error.code, "NX-E0102");
        assert_eq!(error.subcause.as_deref(), Some("type_mismatch"));
        assert_eq!(error.stage, Stage::Types);
        assert_eq!(error.operation, Operation::TypesCheck);
        assert_eq!(
            error.primary_span.as_ref().map(|s| (s.start, s.end)),
            Some((start, start + 1))
        );
        assert_eq!(
            error.data.get("expected_type"),
            Some(&DataValue::String(expected.into()))
        );
        assert_eq!(
            error.data.get("found_type"),
            Some(&DataValue::String("MockResource".into()))
        );
        assert_eq!(error.related_spans.len(), 1);
        assert_eq!(error.related_spans[0].role, RelatedRole::FoundType);
        let found = if start == op { op - 1 } else { start };
        assert_eq!(
            (
                error.related_spans[0].span.start,
                error.related_spans[0].span.end
            ),
            (found, found + 1)
        );
    }
    let valid = format!("fn Main(){{var q={dv};q=1;let t=q<1;AssertEq(1,1);}}");
    assert!(check_proposed("right_assert_eq_valid.n", &valid, false).is_ok());
    assert!(check_scalar("right_assert_eq_valid.n", &valid).is_ok());
}

#[test]
fn right_branch_side_effect_keeps_left_domain_after_inner_failure() {
    // F0-0002 §4.1.2.4–5, RD95: the right branch fixes q to Array<I64>
    // before its value fails. The outer < domain and the inner + site are
    // independent, even though their spans belong to nested expressions.
    let dv = "(if true then{return;}else{return;})";
    let source =
        format!("fn Main(){{var q={dv};let t=q<(if true then {{q=[1];value:true+1;}} else 1);}}");
    let errors = check_proposed("right_branch_effect.n", &source, false).unwrap_err();
    assert_eq!(errors.len(), 2, "{errors:?}");
    let op = source.find("q<").unwrap() as u64 + 1;
    let leaf = source.rfind("true+1").unwrap() as u64;
    for (error, start, end, expected, found) in [
        (&errors[0], op, op + 1, "I64", "Array<I64>"),
        (&errors[1], leaf, leaf + 4, "I64", "Bool"),
    ] {
        assert_eq!(error.code, "NX-E0102");
        assert_eq!(error.subcause.as_deref(), Some("type_mismatch"));
        assert_eq!(error.stage, Stage::Types);
        assert_eq!(error.operation, Operation::TypesCheck);
        assert_eq!(
            error.primary_span.as_ref().map(|s| (s.start, s.end)),
            Some((start, end))
        );
        assert_eq!(
            error.data.get("expected_type"),
            Some(&DataValue::String(expected.into()))
        );
        assert_eq!(
            error.data.get("found_type"),
            Some(&DataValue::String(found.into()))
        );
        assert_eq!(error.related_spans.len(), 1);
        assert_eq!(error.related_spans[0].role, RelatedRole::FoundType);
        let found_start = if start == op { op - 1 } else { start };
        let found_end = if start == op { op } else { end };
        assert_eq!(
            (
                error.related_spans[0].span.start,
                error.related_spans[0].span.end
            ),
            (found_start, found_end)
        );
    }
    let valid = format!("fn Main(){{var q={dv};let t=q<(if true then {{q=1;value:1;}} else 1);}}");
    assert!(check_proposed("right_branch_effect_valid.n", &valid, false).is_ok());
    assert!(check_scalar("right_branch_effect_valid.n", &valid).is_ok());
}

#[test]
fn pending_left_array_conflict_keeps_nested_right_error() {
    // F0-0002 §§4.1.2.2,4,5 and RD119: the unresolved Array constructor
    // owns U, while the nested + on the independent right operand owns M.
    let source = "fn Main(){var q=[];let t=q+(true+1);}";
    let errors = check_proposed("pending_left_nested.n", source, false).unwrap_err();
    assert_eq!(errors.len(), 2, "{errors:?}");
    assert_eq!(errors[0].code, "NX-E0103");
    assert_eq!(errors[0].stage, Stage::Types);
    assert_eq!(errors[0].operation, Operation::TypesInfer);
    let constructor = source.find("[]").unwrap() as u64;
    assert_eq!(
        errors[0].primary_span.as_ref().map(|s| (s.start, s.end)),
        Some((constructor, constructor + 2))
    );
    assert!(errors[0].related_spans.is_empty());
    let nested = &errors[1];
    assert_eq!(nested.code, "NX-E0102");
    assert_eq!(nested.subcause.as_deref(), Some("type_mismatch"));
    assert_eq!(nested.stage, Stage::Types);
    assert_eq!(nested.operation, Operation::TypesCheck);
    let leaf = source.find("true+1").unwrap() as u64;
    assert_eq!(
        nested.primary_span.as_ref().map(|s| (s.start, s.end)),
        Some((leaf, leaf + 4))
    );
    assert_eq!(
        nested.data.get("expected_type"),
        Some(&DataValue::String("I64".into()))
    );
    assert_eq!(
        nested.data.get("found_type"),
        Some(&DataValue::String("Bool".into()))
    );
    assert_eq!(nested.related_spans.len(), 1);
    assert_eq!(nested.related_spans[0].role, RelatedRole::FoundType);
    assert_eq!(
        (
            nested.related_spans[0].span.start,
            nested.related_spans[0].span.end
        ),
        (leaf, leaf + 4)
    );

    let only_constructor = source.replace("true+1", "1+1");
    let errors = check_proposed("pending_left_only.n", &only_constructor, false).unwrap_err();
    assert_eq!(errors.len(), 1);
    assert_eq!(errors[0].code, "NX-E0103");
    let only_right = source.replace("var q=[];", "var q=1;");
    let errors = check_proposed("nested_right_only.n", &only_right, false).unwrap_err();
    assert_eq!(errors.len(), 1);
    assert_eq!(errors[0].code, "NX-E0102");
    let valid = only_right.replace("true+1", "1+1");
    assert!(check_proposed("pending_left_valid.n", &valid, false).is_ok());
    assert!(check_scalar("pending_left_valid.n", &valid).is_ok());
}

#[test]
fn failed_assert_eq_argument_suppresses_result_annotation_only() {
    // F0-0002 §§4.1.2.4–5, RD109: a failing argument invalidates the
    // call result, so the Bool annotation cannot compare against its
    // nominal Unit result. A valid call still checks that annotation.
    let source = "fn Main(){let t:Bool=AssertEq(true+1,1);}";
    let errors = check_proposed("assert_arg_failed.n", source, false).unwrap_err();
    assert_eq!(errors.len(), 1, "{errors:?}");
    let error = &errors[0];
    assert_eq!(error.code, "NX-E0102");
    assert_eq!(error.subcause.as_deref(), Some("type_mismatch"));
    assert_eq!(error.stage, Stage::Types);
    assert_eq!(error.operation, Operation::TypesCheck);
    let leaf = source.find("true+1").unwrap() as u64;
    assert_eq!(
        error.primary_span.as_ref().map(|s| (s.start, s.end)),
        Some((leaf, leaf + 4))
    );
    assert_eq!(
        error.data.get("expected_type"),
        Some(&DataValue::String("I64".into()))
    );
    assert_eq!(
        error.data.get("found_type"),
        Some(&DataValue::String("Bool".into()))
    );
    assert_eq!(error.related_spans.len(), 1);
    assert_eq!(error.related_spans[0].role, RelatedRole::FoundType);
    assert_eq!(
        (
            error.related_spans[0].span.start,
            error.related_spans[0].span.end
        ),
        (leaf, leaf + 4)
    );

    let internal_only = source.replace("t:Bool", "t:()");
    let errors = check_proposed("assert_arg_only.n", &internal_only, false).unwrap_err();
    assert_eq!(errors.len(), 1);
    assert_eq!(errors[0].code, "NX-E0102");
    let annotation_only = source.replace("true+1", "1+1");
    let errors = check_proposed("assert_annotation_only.n", &annotation_only, false).unwrap_err();
    assert_eq!(errors.len(), 1);
    assert_eq!(errors[0].code, "NX-E0102");
    assert_eq!(
        errors[0].data.get("expected_type"),
        Some(&DataValue::String("Bool".into()))
    );
    assert_eq!(
        errors[0].data.get("found_type"),
        Some(&DataValue::String("()".into()))
    );
    let valid = annotation_only.replace("t:Bool", "t:()");
    assert!(check_proposed("assert_arg_valid.n", &valid, false).is_ok());
    assert!(check_scalar("assert_arg_valid.n", &valid).is_ok());
}

#[test]
fn late_array_shape_conflict_keeps_intrinsic_numeric_expected() {
    // F0-0002 §§4.1.2.2,4,6 and RD75: the later [1] completes the
    // left operand's found type. + still expects intrinsic I64 at q.
    let source = "fn Main(){var q=[];let t=q+1;q=[1];}";
    let errors = check_proposed("late_numeric_array.n", source, false).unwrap_err();
    assert_eq!(errors.len(), 1, "{errors:?}");
    let error = &errors[0];
    assert_eq!(error.code, "NX-E0102");
    assert_eq!(error.subcause.as_deref(), Some("type_mismatch"));
    assert_eq!(error.stage, Stage::Types);
    assert_eq!(error.operation, Operation::TypesCheck);
    let left = source.find("q+1").unwrap() as u64;
    assert_eq!(
        error.primary_span.as_ref().map(|s| (s.start, s.end)),
        Some((left, left + 1))
    );
    assert_eq!(
        error.data.get("expected_type"),
        Some(&DataValue::String("I64".into()))
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
        (left, left + 1)
    );
    let valid = "fn Main(){var q=(if true then{return;}else{return;});q=1;let t=q+1;}";
    assert!(check_proposed("late_numeric_valid.n", valid, false).is_ok());
    assert!(check_scalar("late_numeric_valid.n", valid).is_ok());
}

#[test]
fn resource_assert_eq_errors_suppress_result_annotation() {
    // F0-0002 §§4.1.2.4-5, RD104: each invalid argument keeps its own
    // copyable_data site; the tainted AssertEq result has no derived M.
    let source = "fn Main(){let u:I64=AssertEq(MockOpen(1),MockOpen(1));}";
    let errors = check_proposed("resource_assert_eq.n", source, false).unwrap_err();
    assert_eq!(errors.len(), 2, "{errors:?}");
    for (error, needle) in errors.iter().zip(["MockOpen(1)", "MockOpen(1)"]) {
        assert_eq!(error.code, "NX-E0102");
        assert_eq!(error.subcause.as_deref(), Some("type_mismatch"));
        assert_eq!(error.stage, Stage::Types);
        assert_eq!(error.operation, Operation::TypesCheck);
        assert_eq!(
            error.data.get("expected_type"),
            Some(&DataValue::String("domain:copyable_data".into()))
        );
        assert_eq!(
            error.data.get("found_type"),
            Some(&DataValue::String("MockResource".into()))
        );
        assert_eq!(error.related_spans.len(), 1);
        assert_eq!(error.related_spans[0].role, RelatedRole::FoundType);
        let start = if std::ptr::eq(error, &errors[0]) {
            source.find(needle).unwrap()
        } else {
            source.rfind(needle).unwrap()
        } as u64;
        let span = (start, start + needle.len() as u64);
        assert_eq!(
            error.primary_span.as_ref().map(|s| (s.start, s.end)),
            Some(span)
        );
        assert_eq!(
            (
                error.related_spans[0].span.start,
                error.related_spans[0].span.end
            ),
            span
        );
    }
    let valid = "fn Main(){let u:()=AssertEq(1,1);}";
    assert!(check_proposed("resource_assert_eq_valid.n", valid, false).is_ok());
    assert!(check_scalar("resource_assert_eq_valid.n", valid).is_ok());
}

#[test]
fn later_resource_binding_keeps_deferred_assert_eq_argument_site() {
    // F0-0002 §§4.1.2.4-5, RD104/RD105: resolving q while visiting the right
    // argument keeps both copyability sites and contaminates the call result.
    let source = "fn Main(){var q=(if true then{return;}else{return;});var r=(if true then{return;}else{return;});let u:I64=AssertEq(q,(if true then {q=MockOpen(1);value:r;} else {value:r;}));}";
    let errors = check_proposed("late_resource_assert_eq.n", source, false).unwrap_err();
    assert_eq!(errors.len(), 2, "{errors:?}");
    let first = source.find("AssertEq(q").unwrap() as u64 + "AssertEq(".len() as u64;
    let second = source.find("(if true then {q=MockOpen").unwrap() as u64 + 1;
    let second_end = source.rfind("));}").unwrap() as u64;
    for (error, span) in errors
        .iter()
        .zip([(first, first + 1), (second, second_end)])
    {
        assert_eq!(error.code, "NX-E0102");
        assert_eq!(error.subcause.as_deref(), Some("type_mismatch"));
        assert_eq!(error.stage, Stage::Types);
        assert_eq!(error.operation, Operation::TypesCheck);
        assert_eq!(
            error.primary_span.as_ref().map(|s| (s.start, s.end)),
            Some(span)
        );
        assert_eq!(
            error.data.get("expected_type"),
            Some(&DataValue::String("domain:copyable_data".into()))
        );
        assert_eq!(
            error.data.get("found_type"),
            Some(&DataValue::String("MockResource".into()))
        );
        assert_eq!(error.related_spans.len(), 1);
        assert_eq!(error.related_spans[0].role, RelatedRole::FoundType);
        assert_eq!(
            (
                error.related_spans[0].span.start,
                error.related_spans[0].span.end
            ),
            span
        );
    }
    let valid = "fn Main(){var q=(if true then{return;}else{return;});q=1;let u:()=AssertEq(q,1);}";
    assert!(check_proposed("late_resource_assert_eq_valid.n", valid, false).is_ok());
    assert!(check_scalar("late_resource_assert_eq_valid.n", valid).is_ok());
}

#[test]
fn contaminated_inner_addition_keeps_outer_right_domain() {
    // F0-0002 §§4.1.2.4-5, RD95/RD126: a pending failure inside the
    // outer left operand is not that outer operation's domain failure.
    let source = "fn Main(){var q=[];let t=(q+1)+true;q=[1];}";
    let errors = check_proposed("nested_addition_domain.n", source, false).unwrap_err();
    assert_eq!(errors.len(), 2, "{errors:?}");
    let q = source.find("q+1").unwrap() as u64;
    let right = source.find("+true").unwrap() as u64 + 1;
    for (error, span, found) in [
        (&errors[0], (q, q + 1), "Array<I64>"),
        (&errors[1], (right, right + 4), "Bool"),
    ] {
        assert_eq!(error.code, "NX-E0102");
        assert_eq!(error.subcause.as_deref(), Some("type_mismatch"));
        assert_eq!(error.stage, Stage::Types);
        assert_eq!(error.operation, Operation::TypesCheck);
        assert_eq!(
            error.primary_span.as_ref().map(|s| (s.start, s.end)),
            Some(span)
        );
        assert_eq!(
            error.data.get("expected_type"),
            Some(&DataValue::String("I64".into()))
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
            span
        );
    }
    let valid = "fn Main(){var q=[];let t=(q+1)+2;q=[1];}";
    // Repair both independent sites, preserving the same expression tree.
    let repaired = valid.replace("q=[]", "q=1").replace("q=[1]", "q=1");
    assert!(check_proposed("nested_addition_domain_valid.n", &repaired, false).is_ok());
    assert!(check_scalar("nested_addition_domain_valid.n", &repaired).is_ok());
}

#[test]
fn inner_right_numeric_conflict_does_not_consume_outer_domain() {
    // F0-0002 §§4.1.2.1,4-5, RD15/RD95: the pending join belongs to the
    // inner +; the outer right Bool is an independent domain site.
    let source = "fn Main(){var r=[];let t=(1+r)+true;r=[1];}";
    let errors = check_proposed("inner_right_domain.n", source, false).unwrap_err();
    assert_eq!(errors.len(), 2, "{errors:?}");
    let inner = source.find("1+r").unwrap() as u64 + 2;
    let outer = source.find("+true").unwrap() as u64 + 1;
    for (error, span, found) in [
        (&errors[0], (inner, inner + 1), "Array<I64>"),
        (&errors[1], (outer, outer + 4), "Bool"),
    ] {
        assert_eq!(error.code, "NX-E0102");
        assert_eq!(error.subcause.as_deref(), Some("type_mismatch"));
        assert_eq!(error.stage, Stage::Types);
        assert_eq!(error.operation, Operation::TypesCheck);
        assert_eq!(
            error.primary_span.as_ref().map(|s| (s.start, s.end)),
            Some(span)
        );
        assert_eq!(
            error.data.get("expected_type"),
            Some(&DataValue::String("I64".into()))
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
            span
        );
    }
    let repaired = "fn Main(){var r=1;let t=(1+r)+2;r=1;}";
    assert!(check_proposed("inner_right_domain_valid.n", repaired, false).is_ok());
    assert!(check_scalar("inner_right_domain_valid.n", repaired).is_ok());
}

#[test]
fn distributed_numeric_context_survives_earlier_binary_branch() {
    // F0-0002 §§4.1.2.1,4-5, RD15/RD84: the I64 context of the outer +
    // reaches the later branch after an earlier binary has been synthesized.
    let source =
        "fn Main(){var q=[];let t=(if true then {value:1+1;} else {value:q;})+true;q=[1];}";
    let errors = check_proposed("distributed_numeric_owner.n", source, false).unwrap_err();
    assert_eq!(errors.len(), 1, "{errors:?}");
    let error = &errors[0];
    let q = source.find("value:q").unwrap() as u64 + "value:".len() as u64;
    assert_eq!(error.code, "NX-E0102");
    assert_eq!(error.subcause.as_deref(), Some("type_mismatch"));
    assert_eq!(error.stage, Stage::Types);
    assert_eq!(error.operation, Operation::TypesCheck);
    assert_eq!(
        error.primary_span.as_ref().map(|s| (s.start, s.end)),
        Some((q, q + 1))
    );
    assert_eq!(
        error.data.get("expected_type"),
        Some(&DataValue::String("I64".into()))
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
        (q, q + 1)
    );
    let repaired = "fn Main(){var q=1;let t=(if true then {value:1+1;} else {value:q;})+true;q=1;}";
    let repaired = repaired.replace("+true", "+2");
    assert!(check_proposed("distributed_numeric_owner_valid.n", &repaired, false).is_ok());
    assert!(check_scalar("distributed_numeric_owner_valid.n", &repaired).is_ok());
}

#[test]
fn failed_comparison_right_keeps_determined_left_domain() {
    // F0-0002 §§4.1.2.4-5, RD95: right fails at its own + site after
    // fixing q to Array<I64>; < keeps the independent left domain site.
    let source = "fn Main(){var q=(if true then{return;}else{return;});let t=q<((if true then {q=[1];value:1;} else 1)+true);}";
    let errors = check_proposed("failed_comparison_right.n", source, false).unwrap_err();
    assert_eq!(errors.len(), 2, "{errors:?}");
    let left = source.find("q<").unwrap() as u64;
    let op = left + 1;
    let bad = source.rfind("+true").unwrap() as u64 + 1;
    for (error, primary, related, found) in [
        (&errors[0], (op, op + 1), (left, left + 1), "Array<I64>"),
        (&errors[1], (bad, bad + 4), (bad, bad + 4), "Bool"),
    ] {
        assert_eq!(error.code, "NX-E0102");
        assert_eq!(error.subcause.as_deref(), Some("type_mismatch"));
        assert_eq!(error.stage, Stage::Types);
        assert_eq!(error.operation, Operation::TypesCheck);
        assert_eq!(
            error.primary_span.as_ref().map(|s| (s.start, s.end)),
            Some(primary)
        );
        assert_eq!(
            error.data.get("expected_type"),
            Some(&DataValue::String("I64".into()))
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
            related
        );
    }
    let valid = "fn Main(){var q=(if true then{return;}else{return;});let t=q<((if true then {q=1;value:1;} else 1)+1);}";
    assert!(check_proposed("failed_comparison_right_valid.n", valid, false).is_ok());
    assert!(check_scalar("failed_comparison_right_valid.n", valid).is_ok());
}

#[test]
fn pending_right_join_at_either_boundary_keeps_left_comparison_domain() {
    // F0-0002 §§4.1.2.1,4-5, RD95: an internal pending + at the start
    // or end of the right operand leaves the left < domain independent.
    for right in ["q+1", "1+q"] {
        let source = format!("fn Main(){{let a=[1];var q=[];let t=a<{right};q=[1];}}");
        let errors = check_proposed("right_boundary_join.n", &source, false).unwrap_err();
        assert_eq!(errors.len(), 2, "{right}: {errors:?}");
        let left = source.find("a<").unwrap() as u64;
        let op = left + 1;
        let q = source.find(right).unwrap() as u64 + right.find('q').unwrap() as u64;
        for (error, primary, found) in [
            (&errors[0], (op, op + 1), (left, left + 1)),
            (&errors[1], (q, q + 1), (q, q + 1)),
        ] {
            assert_eq!(error.code, "NX-E0102");
            assert_eq!(error.subcause.as_deref(), Some("type_mismatch"));
            assert_eq!(error.stage, Stage::Types);
            assert_eq!(error.operation, Operation::TypesCheck);
            assert_eq!(
                error.primary_span.as_ref().map(|s| (s.start, s.end)),
                Some(primary)
            );
            assert_eq!(
                error.data.get("expected_type"),
                Some(&DataValue::String("I64".into()))
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
                found
            );
        }
        let valid = source
            .replace("a=[1]", "a=1")
            .replace("q=[]", "q=1")
            .replace("q=[1]", "q=1");
        assert!(check_proposed("right_boundary_join_valid.n", &valid, false).is_ok());
        assert!(check_scalar("right_boundary_join_valid.n", &valid).is_ok());
    }
}

#[test]
fn failed_inner_union_keeps_outer_incompatible_continuation() {
    // F0-0002 §§4.1.2.4-5, RD95: the inner union supplies its first valid
    // continuation as the outer witness, whose later Bool is independent.
    let source = "fn Main(){var r=[];let q=(if true then{value:(if true then{value:r+1;}else{value:1;});}else{value:true;});r=[1];}";
    let errors = check_proposed("inner_partial_outer_join.n", source, false).unwrap_err();
    assert_eq!(errors.len(), 2, "{errors:?}");
    let first = &errors[0];
    let r = source.find("r+1").unwrap() as u64;
    assert_eq!(first.code, "NX-E0102");
    assert_eq!(first.subcause.as_deref(), Some("type_mismatch"));
    assert_eq!(first.stage, Stage::Types);
    assert_eq!(first.operation, Operation::TypesCheck);
    assert_eq!(
        first.primary_span.as_ref().map(|s| (s.start, s.end)),
        Some((r, r + 1))
    );
    assert_eq!(
        first.data.get("expected_type"),
        Some(&DataValue::String("I64".into()))
    );
    assert_eq!(
        first.data.get("found_type"),
        Some(&DataValue::String("Array<I64>".into()))
    );
    assert_eq!(first.related_spans.len(), 1);
    assert_eq!(first.related_spans[0].role, RelatedRole::FoundType);
    assert_eq!(
        (
            first.related_spans[0].span.start,
            first.related_spans[0].span.end
        ),
        (r, r + 1)
    );
    let second = &errors[1];
    let bool_at = source.find("value:true").unwrap() as u64 + "value:".len() as u64;
    let inner = source
        .find("(if true then{value:r+1;}else{value:1;})")
        .unwrap() as u64;
    assert_eq!(second.code, "NX-E1303");
    assert_eq!(second.subcause, None);
    assert_eq!(second.stage, Stage::Types);
    assert_eq!(second.operation, Operation::TypesCheck);
    assert_eq!(
        second.primary_span.as_ref().map(|s| (s.start, s.end)),
        Some((bool_at, bool_at + 4))
    );
    assert_eq!(
        second.data.get("expected_type"),
        Some(&DataValue::String("I64".into()))
    );
    assert_eq!(
        second.data.get("found_type"),
        Some(&DataValue::String("Bool".into()))
    );
    assert_eq!(second.related_spans.len(), 2);
    assert_eq!(second.related_spans[0].role, RelatedRole::ExpectedType);
    assert_eq!(
        (
            second.related_spans[0].span.start,
            second.related_spans[0].span.end
        ),
        (inner + 1, inner + 39)
    );
    assert_eq!(second.related_spans[1].role, RelatedRole::FoundType);
    assert_eq!(
        (
            second.related_spans[1].span.start,
            second.related_spans[1].span.end
        ),
        (bool_at, bool_at + 4)
    );
    let repaired = source
        .replace("r=[]", "r=1")
        .replace("value:true", "value:1")
        .replace("r=[1]", "r=1");
    assert!(check_proposed("inner_partial_outer_join_valid.n", &repaired, false).is_ok());
    assert!(check_scalar("inner_partial_outer_join_valid.n", &repaired).is_ok());
}

#[test]
fn failed_nested_union_keeps_independent_comparison_right_domain() {
    // F0-0002 §§4.1.2.4-5, RD95: the nested union owns J and the valid
    // right operand of < retains its separate I64 domain site.
    let source = "fn Main(){let t=(if true then{value:1;}else{value:(if true then{value:true;}else{value:1;});})<[1];}";
    let errors = check_proposed("nested_union_comparison.n", source, false).unwrap_err();
    assert_eq!(errors.len(), 2, "{errors:?}");
    let wrong = source.find("value:true").unwrap() as u64 + "value:".len() as u64;
    let origin = source.find("value:1").unwrap() as u64 + "value:".len() as u64;
    let op = source.find("<[1]").unwrap() as u64;
    let first = &errors[0];
    assert_eq!(first.code, "NX-E1303");
    assert_eq!(first.subcause, None);
    assert_eq!(first.stage, Stage::Types);
    assert_eq!(first.operation, Operation::TypesCheck);
    assert_eq!(
        first.primary_span.as_ref().map(|s| (s.start, s.end)),
        Some((wrong, wrong + 4))
    );
    assert_eq!(
        first.data.get("expected_type"),
        Some(&DataValue::String("I64".into()))
    );
    assert_eq!(
        first.data.get("found_type"),
        Some(&DataValue::String("Bool".into()))
    );
    assert_eq!(first.related_spans.len(), 2);
    assert_eq!(first.related_spans[0].role, RelatedRole::ExpectedType);
    assert_eq!(
        (
            first.related_spans[0].span.start,
            first.related_spans[0].span.end
        ),
        (origin, origin + 1)
    );
    assert_eq!(first.related_spans[1].role, RelatedRole::FoundType);
    assert_eq!(
        (
            first.related_spans[1].span.start,
            first.related_spans[1].span.end
        ),
        (wrong, wrong + 4)
    );
    let second = &errors[1];
    assert_eq!(second.code, "NX-E0102");
    assert_eq!(second.subcause.as_deref(), Some("type_mismatch"));
    assert_eq!(second.stage, Stage::Types);
    assert_eq!(second.operation, Operation::TypesCheck);
    assert_eq!(
        second.primary_span.as_ref().map(|s| (s.start, s.end)),
        Some((op, op + 1))
    );
    assert_eq!(
        second.data.get("expected_type"),
        Some(&DataValue::String("I64".into()))
    );
    assert_eq!(
        second.data.get("found_type"),
        Some(&DataValue::String("Array<I64>".into()))
    );
    assert_eq!(second.related_spans.len(), 1);
    assert_eq!(second.related_spans[0].role, RelatedRole::FoundType);
    assert_eq!(
        (
            second.related_spans[0].span.start,
            second.related_spans[0].span.end
        ),
        (op + 1, op + 4)
    );
    let repaired = source
        .replace("value:true", "value:1")
        .replace("<[1]", "<1");
    assert!(check_proposed("nested_union_comparison_valid.n", &repaired, false).is_ok());
    assert!(check_scalar("nested_union_comparison_valid.n", &repaired).is_ok());
}

#[test]
fn alias_of_intrinsically_fixed_binding_keeps_no_source_origin() {
    // F0-0002 §§4.1.2.6 and 13.15, RD120/RD126: the I64 requirement of
    // + stays intrinsic when a later assignment copies the typed binding.
    let dv = "(if true then{return;}else{return;})";
    let source = format!("fn Main(){{var a={dv};var b={dv};let t=b+1;a=b;a=true;}}");
    let errors = check_proposed("intrinsic_alias.n", &source, false).unwrap_err();
    assert_eq!(errors.len(), 1, "{errors:?}");
    let error = &errors[0];
    assert_eq!(error.code, "NX-E0102");
    assert_eq!(error.subcause.as_deref(), Some("type_mismatch"));
    assert_eq!(error.stage, Stage::Types);
    assert_eq!(error.operation, Operation::TypesCheck);
    let start = source.rfind("true;").unwrap() as u64;
    assert_eq!(
        error.primary_span.as_ref().map(|s| (s.start, s.end)),
        Some((start, start + 4))
    );
    assert_eq!(
        error.data.get("expected_type"),
        Some(&DataValue::String("I64".into()))
    );
    assert_eq!(
        error.data.get("found_type"),
        Some(&DataValue::String("Bool".into()))
    );
    assert_eq!(error.related_spans.len(), 1);
    assert_eq!(error.related_spans[0].role, RelatedRole::FoundType);
    assert_eq!(
        (
            error.related_spans[0].span.start,
            error.related_spans[0].span.end
        ),
        (start, start + 4)
    );

    // A real source constraint supplies the relation through the same alias.
    let source = format!("fn Main(){{var a={dv};var b={dv};b=1;a=b;a=true;}}");
    let errors = check_proposed("source_alias.n", &source, false).unwrap_err();
    assert_eq!(errors.len(), 1, "{errors:?}");
    let error = &errors[0];
    assert_eq!(error.code, "NX-E0102");
    assert_eq!(error.subcause.as_deref(), Some("type_mismatch"));
    let witness = source.find("b=1").unwrap() as u64 + 2;
    assert_eq!(error.related_spans[0].role, RelatedRole::ExpectedType);
    assert_eq!(
        (
            error.related_spans[0].span.start,
            error.related_spans[0].span.end
        ),
        (witness, witness + 1)
    );
    let repaired = source.replace("a=true", "a=2");
    assert!(check_proposed("source_alias_valid.n", &repaired, false).is_ok());
}

#[test]
fn projected_alias_keeps_earlier_element_context_when_array_arrives_later() {
    // F0-0002 §4.1.2.3, RD107: the Bool context on a projection follows
    // aliases, then constrains the element when the Array base is established.
    let prefix = "fn Main(){var q=(if true then {return;}else{return;});var r=(if true then {return;}else{return;});let x:Bool=r[0];r=q;q=";
    let repaired = format!("{prefix}[true];}}");
    assert!(check_proposed("projected_alias.n", &repaired, false).is_ok());
    assert!(check_scalar("projected_alias.n", &repaired).is_ok());

    let source = format!("{prefix}[1];}}");
    let errors = check_proposed("projected_alias.n", &source, false).unwrap_err();
    assert_eq!(errors.len(), 1, "{errors:?}");
    let error = &errors[0];
    assert_eq!(error.code, "NX-E0102");
    assert_eq!(error.subcause.as_deref(), Some("type_mismatch"));
    assert_eq!(error.stage, Stage::Types);
    assert_eq!(error.operation, Operation::TypesCheck);
    let primary = error.primary_span.as_ref().unwrap();
    assert_eq!((primary.start, primary.end), (121, 122));
    assert_eq!(&source[primary.start as usize..primary.end as usize], "1");
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
        (104, 108)
    );
    assert_eq!(error.related_spans[1].role, RelatedRole::FoundType);
    assert_eq!(
        (
            error.related_spans[1].span.start,
            error.related_spans[1].span.end
        ),
        (121, 122)
    );
}

#[test]
fn unresolved_projection_keeps_selector_hypothetical_in_lowering() {
    // F0-0002 §4.1.2.3, RD121: without a determined Array base the I64
    // selector context is conditional; a dead hypothetical read emits no IR.
    let source = "fn Main(){var d=(if true then{return;}else{return;});let ar=[1];let x=d[ar];}";
    assert!(check_proposed("unresolved_projection.n", source, false).is_ok());
    assert!(check_scalar("unresolved_projection.n", source).is_ok());

    // Read parameters and AssertEq use the same hypothetical expression
    // without constructing a loan for a place with no live owner (§4.1.2.3).
    for source in [
        "fn FI(a:view I64){} fn Main(){var d=(if true then{return;}else{return;});let ar=[1];FI(d[ar]);}",
        "fn Main(){var d=(if true then{return;}else{return;});let ar=[1];AssertEq(d[ar],false);}",
    ] {
        assert!(check_proposed("unresolved_read_loan.n", source, false).is_ok());
        assert!(check_scalar("unresolved_read_loan.n", source).is_ok());
    }

    // Establishing Array later activates the selector constraint at its own
    // site; the Array value supplied as selector then fails (RD114/RD117).
    let resolved =
        "fn Main(){var d=(if true then{return;}else{return;});let ar=[1];let x=d[ar];d=[1];}";
    let errors = check_proposed("resolved_projection.n", resolved, false).unwrap_err();
    assert_eq!(errors.len(), 1, "{errors:?}");
    assert_eq!(errors[0].code, "NX-E0102");
    assert_eq!(
        errors[0].data.get("expected_type"),
        Some(&DataValue::String("I64".into()))
    );
    assert_eq!(
        errors[0].data.get("found_type"),
        Some(&DataValue::String("Array<I64>".into()))
    );
}

#[test]
fn failed_right_comparison_keeps_the_valid_left_domain() {
    // F0-0002 §§4.1.2.4–5, RD95: the failed right result suppresses cross-type
    // equality, while the valid left operand still fails its own I64 domain.
    for source in [
        "fn Main(){let p=true;if p<(if p then false else 1) {}}",
        "fn Main(){let a=[1];if a<a[true] {}}",
    ] {
        let errors = check_proposed("independent_left_domain.n", source, false).unwrap_err();
        assert_eq!(errors.len(), 2, "{source}: {errors:?}");
        assert!(
            errors
                .iter()
                .all(|d| d.code == "NX-E0102" || d.code == "NX-E1303")
        );
        let domain = errors
            .iter()
            .find(|d| {
                d.primary_span
                    .as_ref()
                    .is_some_and(|s| &source[s.start as usize..s.end as usize] == "<")
            })
            .unwrap();
        assert_eq!(domain.code, "NX-E0102");
        assert_eq!(
            domain.data.get("expected_type"),
            Some(&DataValue::String("I64".into()))
        );
        assert_eq!(domain.related_spans.len(), 1);
        assert_eq!(domain.related_spans[0].role, RelatedRole::FoundType);
    }
}

#[test]
fn nested_later_union_uses_its_first_valid_leaf_against_outer_anchor() {
    // F0-0002 §4.1.2.4–5, RD72: once the outer union has an Array<Bool>
    // witness, a later nested union compares every valid continuation to it.
    let source = "fn Main(){let a=(if true then [true] else (if false then [true,1] else 1));}";
    let errors = check_proposed("nested_later_union.n", source, false).unwrap_err();
    assert_eq!(errors.len(), 2, "{errors:?}");
    assert_eq!(
        errors.iter().map(|d| d.code.as_str()).collect::<Vec<_>>(),
        ["NX-E0102", "NX-E1303"]
    );
    assert_eq!(
        errors
            .iter()
            .map(|d| d.primary_span.as_ref().map(|s| (s.start, s.end)))
            .collect::<Vec<_>>(),
        [Some((63, 64)), Some((71, 72))]
    );
    assert_eq!(
        errors[1].data.get("expected_type"),
        Some(&DataValue::String("Array<Bool>".into()))
    );
    assert_eq!(
        errors[1].data.get("found_type"),
        Some(&DataValue::String("I64".into()))
    );
    assert_eq!(errors[1].related_spans[0].role, RelatedRole::ExpectedType);
    assert_eq!(
        (
            errors[1].related_spans[0].span.start,
            errors[1].related_spans[0].span.end
        ),
        (30, 36)
    );

    let repaired =
        "fn Main(){let a=(if true then [true] else (if false then [true,true] else [true]));}";
    assert!(check_proposed("nested_later_union_repaired.n", repaired, false).is_ok());
}

#[test]
fn unresolved_array_conflict_preserves_constructor_diagnostic() {
    // F0-0002 §4.1.2.2: both [] constructors keep their own inference
    // obligation even after a conflicting scalar use of their component.
    let source = "fn Main(){var r=[];let x=r+1;r=[];}";
    let errors = check_proposed("constructor_conflict.n", source, false).unwrap_err();
    assert_eq!(errors.len(), 2, "{errors:?}");
    assert!(
        errors
            .iter()
            .all(|error| error.code == "NX-E0103" && error.subcause.is_none())
    );
    assert_eq!(
        errors
            .iter()
            .map(|error| error
                .primary_span
                .as_ref()
                .map(|span| (span.start, span.end)))
            .collect::<Vec<_>>(),
        [Some((16, 18)), Some((31, 33))]
    );
    assert!(errors.iter().all(|error| error.related_spans.is_empty()));
}

#[test]
fn empty_return_keeps_written_result_origin() {
    // F0-0002 §13.15: the written result type supplies expected_type;
    // an absent return value has no found_type source relation.
    let source = "fn F()->I64{return;} fn Main(){}";
    let errors = check_proposed("empty_return.n", source, false).unwrap_err();
    assert_eq!(errors.len(), 1, "{errors:?}");
    let error = &errors[0];
    assert_eq!(error.code, "NX-E0102");
    assert_eq!(error.subcause.as_deref(), Some("type_mismatch"));
    assert_eq!(
        error
            .primary_span
            .as_ref()
            .map(|span| (span.start, span.end)),
        Some((12, 18))
    );
    assert_eq!(
        error.data.get("expected_type"),
        Some(&DataValue::String("I64".into()))
    );
    assert_eq!(
        error.data.get("found_type"),
        Some(&DataValue::String("()".into()))
    );
    assert_eq!(error.related_spans.len(), 1);
    assert_eq!(error.related_spans[0].role, RelatedRole::ExpectedType);
    assert_eq!(
        (
            error.related_spans[0].span.start,
            error.related_spans[0].span.end
        ),
        (8, 11)
    );
}

#[test]
fn try_operator_rejects_non_result_operand() {
    // F0-0002 §7.1 requires a Result operand.
    let source = "fn Main(){let a=1?;}";
    let errors = match check_scalar("pending_try.n", source) {
        Err(CompileError::Rejected(errors)) => errors,
        other => panic!("expected type rejection: {other:?}"),
    };
    assert_eq!(errors.len(), 1, "{errors:?}");
    let error = &errors[0];
    assert_eq!(error.code, "NX-E0102");
    assert_eq!(error.subcause.as_deref(), Some("type_mismatch"));
    assert_eq!(
        error
            .primary_span
            .as_ref()
            .map(|span| (span.start, span.end)),
        Some((17, 18))
    );
    assert_eq!(
        error.data.get("expected_type"),
        Some(&DataValue::String("domain:result".into()))
    );
}

#[test]
fn result_constructor_preserves_written_payload_component() {
    // F0-0002 §4.1: Ok constrains the ok component of Result.
    let source = "fn F()->Result<I64,Bool>{return Ok(true);} fn Main(){}";
    let errors = check_proposed("result_payload.n", source, false).unwrap_err();
    assert_eq!(errors.len(), 1, "{errors:?}");
    let error = &errors[0];
    assert_eq!(error.code, "NX-E0102");
    assert_eq!(error.subcause.as_deref(), Some("type_mismatch"));
    assert_eq!(
        error.data.get("expected_type"),
        Some(&DataValue::String("I64".into()))
    );
    assert_eq!(
        error.data.get("found_type"),
        Some(&DataValue::String("Bool".into()))
    );
}

#[test]
fn unresolved_result_constructor_names_missing_component() {
    // F0-0002 §4.1.1: an unconstrained Ok lacks its err component.
    let source = "fn Main(){let x=Ok(1);}";
    let errors = check_proposed("result_missing.n", source, false).unwrap_err();
    assert_eq!(errors.len(), 1, "{errors:?}");
    let error = &errors[0];
    assert_eq!(error.code, "NX-E0113");
    assert_eq!(
        error.data.get("unresolved_components"),
        Some(&DataValue::Array(vec![DataValue::String("$.err".into())]))
    );
}

#[test]
fn print_resource_uses_its_only_operand_as_primary_and_found() {
    // F0-0002 §§4.6, 13.17: the noncopyable value owns the domain error.
    let source = "fn Main(){Print(MockOpen(1));}";
    let errors = check_proposed("print_resource.n", source, false).unwrap_err();
    assert_eq!(errors.len(), 1, "{errors:?}");
    let error = &errors[0];
    assert_eq!(error.code, "NX-E0102");
    assert_eq!(error.subcause.as_deref(), Some("type_mismatch"));
    assert_eq!(
        error.data.get("expected_type"),
        Some(&DataValue::String("domain:copyable_data".into()))
    );
    assert_eq!(
        error.data.get("found_type"),
        Some(&DataValue::String("MockResource".into()))
    );
    assert_eq!(
        error
            .primary_span
            .as_ref()
            .map(|span| (span.start, span.end)),
        Some((16, 27))
    );
    assert_eq!(error.related_spans.len(), 1);
    assert_eq!(error.related_spans[0].role, RelatedRole::FoundType);
    assert_eq!(
        (
            error.related_spans[0].span.start,
            error.related_spans[0].span.end
        ),
        (16, 27)
    );
}

#[test]
fn assert_eq_resource_reports_each_noncopyable_argument() {
    // F0-0002 §4.1.2.6: each AssertEq argument has an independent copyable
    // domain obligation, even when both occurrences read the same binding.
    let source = "fn Main(){let a=MockOpen(1);AssertEq(a,a);}";
    let errors = check_proposed("eq_resource.n", source, false).unwrap_err();
    assert_eq!(errors.len(), 2, "{errors:?}");
    for (error, span) in errors.iter().zip([(37, 38), (39, 40)]) {
        assert_eq!(error.code, "NX-E0102");
        assert_eq!(
            error.data.get("expected_type"),
            Some(&DataValue::String("domain:copyable_data".into()))
        );
        assert_eq!(
            error.data.get("found_type"),
            Some(&DataValue::String("MockResource".into()))
        );
        assert_eq!(
            error
                .primary_span
                .as_ref()
                .map(|span| (span.start, span.end)),
            Some(span)
        );
        assert_eq!(error.related_spans.len(), 1);
        assert_eq!(error.related_spans[0].role, RelatedRole::FoundType);
        assert_eq!(
            (
                error.related_spans[0].span.start,
                error.related_spans[0].span.end
            ),
            span
        );
    }
}

#[test]
fn mock_read_records_requires_the_resource_argument() {
    // F0-0002 §6: the intrinsic consumes the resource domain.
    let good = "fn Main(){let r=MockOpen(1);let a=MockReadRecords(r);}";
    assert!(check_proposed("read_records_good.n", good, false).is_ok());
    let bad = "fn Main(){let a=MockReadRecords(1);}";
    let errors = check_proposed("read_records_bad.n", bad, false).unwrap_err();
    assert_eq!(errors.len(), 1, "{errors:?}");
    assert_eq!(errors[0].code, "NX-E0102");
    assert_eq!(
        errors[0].data.get("expected_type"),
        Some(&DataValue::String("MockResource".into()))
    );
    assert_eq!(
        errors[0].data.get("found_type"),
        Some(&DataValue::String("I64".into()))
    );
}

#[test]
fn nested_intrinsic_operands_keep_independent_errors() {
    let source = "fn Main(){let x=MockReadRecords(1)+MockReadRecords(true);}";
    let errors = check_proposed("nested_intrinsics.n", source, false).unwrap_err();
    assert_eq!(errors.len(), 2, "{errors:?}");
    for (error, (expected_span, found)) in
        errors.iter().zip([((32, 33), "I64"), ((51, 55), "Bool")])
    {
        assert_eq!(error.code, "NX-E0102");
        assert_eq!(error.subcause.as_deref(), Some("type_mismatch"));
        assert_eq!(
            error
                .primary_span
                .as_ref()
                .map(|span| (span.start, span.end)),
            Some(expected_span)
        );
        assert_eq!(
            error.data.get("expected_type"),
            Some(&DataValue::String("MockResource".into()))
        );
        assert_eq!(
            error.data.get("found_type"),
            Some(&DataValue::String(found.into()))
        );
    }
}

#[test]
fn assert_eq_array_child_compares_at_second_operand() {
    // F0-0002 §4.1.2.4: AssertEq compares whole argument types before
    // checking their domain; the first argument is the expected witness.
    let source = "fn FI(a:view I64){} fn FA(a:view Array<I64>){} fn Main(){let p=true;let x=1;var ar=[1];var r=[];AssertEq([x],[r]);r=[1];}";
    let errors = check_proposed("eq_second_child.n", source, false).unwrap_err();
    assert_eq!(errors.len(), 1, "{errors:?}");
    let error = &errors[0];
    assert_eq!(error.code, "NX-E0102");
    assert_eq!(error.subcause.as_deref(), Some("type_mismatch"));
    assert_eq!(
        error
            .primary_span
            .as_ref()
            .map(|span| (span.start, span.end)),
        Some((109, 112))
    );
    assert_eq!(
        error.data.get("expected_type"),
        Some(&DataValue::String("Array<I64>".into()))
    );
    assert_eq!(
        error.data.get("found_type"),
        Some(&DataValue::String("Array<Array<I64>>".into()))
    );
    assert_eq!(error.related_spans.len(), 2);
    assert_eq!(error.related_spans[0].role, RelatedRole::ExpectedType);
    assert_eq!(
        (
            error.related_spans[0].span.start,
            error.related_spans[0].span.end
        ),
        (105, 108)
    );
    assert_eq!(error.related_spans[1].role, RelatedRole::FoundType);
}

#[test]
fn assert_eq_deferred_array_keeps_later_concrete_origin() {
    // F0-0002 §§4.1.2.2, 4.1.2.6, 13.15: later resolution completes the
    // type, while the first AssertEq argument occurrence remains the origin.
    let source = "fn FI(a:view I64){} fn FA(a:view Array<I64>){} fn Main(){let p=true;let x=1;var ar=[1];var r=[[]];AssertEq(r,ar);r=[[1]];}";
    let errors = check_proposed("eq_later_origin.n", source, false).unwrap_err();
    assert_eq!(errors.len(), 1, "{errors:?}");
    let error = &errors[0];
    assert_eq!(error.code, "NX-E0102");
    assert_eq!(error.subcause.as_deref(), Some("type_mismatch"));
    assert_eq!(
        error
            .primary_span
            .as_ref()
            .map(|span| (span.start, span.end)),
        Some((109, 111))
    );
    assert_eq!(
        error.data.get("expected_type"),
        Some(&DataValue::String("Array<Array<I64>>".into()))
    );
    assert_eq!(
        error.data.get("found_type"),
        Some(&DataValue::String("Array<I64>".into()))
    );
    assert_eq!(error.related_spans.len(), 2);
    assert_eq!(error.related_spans[0].role, RelatedRole::ExpectedType);
    assert_eq!(error.related_spans[1].role, RelatedRole::FoundType);
    assert_eq!(
        (
            error.related_spans[0].span.start,
            error.related_spans[0].span.end
        ),
        (107, 108)
    );
}

#[test]
fn less_than_related_variables_use_the_operator_site_after_equality() {
    // F0-0002 §4.1.2.4: < unifies its pending operands before checking
    // the I64 domain; equal Array operands fail once at the operator.
    let source = "fn Main(){var r=[];var s=[];let q=r<s;r=[1];s=[];AssertEq(r,s);}";
    let errors = check_proposed("less_related.n", source, false).unwrap_err();
    assert_eq!(errors.len(), 1, "{errors:?}");
    let (error, primary, expected, found) = (&errors[0], (35, 36), "I64", "Array<I64>");
    assert_eq!(error.code, "NX-E0102");
    assert_eq!(error.subcause.as_deref(), Some("type_mismatch"));
    assert_eq!(
        error
            .primary_span
            .as_ref()
            .map(|span| (span.start, span.end)),
        Some(primary)
    );
    assert_eq!(
        error.data.get("expected_type"),
        Some(&DataValue::String(expected.into()))
    );
    assert_eq!(
        error.data.get("found_type"),
        Some(&DataValue::String(found.into()))
    );
    assert_eq!(errors[0].related_spans.len(), 1);
    assert_eq!(errors[0].related_spans[0].role, RelatedRole::FoundType);
}

#[test]
fn repeated_deferred_less_than_uses_operator_token() {
    // F0-0002 §§4.1.1, 13.17: one binding constrained by both operands
    // diagnoses the comparison token when its later type is an array.
    let source = "fn Main(){var r=[];let q=r < r;r=[true];}";
    let errors = check_proposed("less_same_binding.n", source, false).unwrap_err();
    assert_eq!(errors.len(), 1, "{errors:?}");
    let error = &errors[0];
    assert_eq!(error.code, "NX-E0102");
    assert_eq!(error.subcause.as_deref(), Some("type_mismatch"));
    assert_eq!(
        error
            .primary_span
            .as_ref()
            .map(|span| (span.start, span.end)),
        Some((27, 28))
    );
    assert_eq!(error.related_spans.len(), 1);
    assert_eq!(error.related_spans[0].role, RelatedRole::FoundType);
    assert_eq!(
        error.data.get("expected_type"),
        Some(&DataValue::String("I64".into()))
    );
    assert_eq!(
        error.data.get("found_type"),
        Some(&DataValue::String("Array<Bool>".into()))
    );
}

#[test]
fn later_unrelated_assignment_keeps_earlier_branch_diagnostic() {
    // F0-0002 §§4.1.2.2, 4.1.2.5: the failed condition contributes no
    // result type to its consumer; the independent Bool and [] errors remain.
    let source = "fn Main(){var a0=(if true then{}else{return;});var a1=[];if(if true then 1 else{value:(if true then{}else{value:a0;});}){}a1=1;}";
    let errors = check_proposed("later_unrelated.n", source, false).unwrap_err();
    assert_eq!(errors.len(), 2, "{errors:?}");
    assert_eq!(
        errors.iter().map(|e| e.code.as_str()).collect::<Vec<_>>(),
        ["NX-E0103", "NX-E0102"]
    );
    assert_eq!(
        errors
            .iter()
            .map(|e| e.primary_span.as_ref().map(|s| (s.start, s.end)))
            .collect::<Vec<_>>(),
        [Some((54, 56)), Some((73, 74))]
    );
}

#[test]
fn pending_join_drain_keeps_all_unresolved_array_diagnostics() {
    // F0-0002 §§4.1.2.2, 13.9: errors in later expressions do not erase
    // independent unresolved constructors, including the final nested [].
    let source = "fn Main(){var x=1;var p=true;var ar=[];var br=[true];var aar=[];var r=[];var a0=(if true then{}else{return;});var a1=[];var a2=x;a1=[(if true then{}else{return;})==(if true then 113else 34),false];br=(if false then{value:false;}else a0);AssertEq(()+(if 276then a0 else{(if false then{}else{value:ar;});}),ar)[[[]]][[(if true then{value:79;}else{}),(if true then x else r)]];}";
    let errors = check_proposed("pending_join_drain.n", source, false).unwrap_err();
    assert_eq!(errors.len(), 8, "{errors:?}");
    assert_eq!(
        errors
            .iter()
            .take(3)
            .map(|e| e.code.as_str())
            .collect::<Vec<_>>(),
        ["NX-E0103", "NX-E0103", "NX-E0103"]
    );
    for error in errors.iter().take(3) {
        assert_eq!(error.subcause, None);
    }
    assert_eq!(errors.last().unwrap().code, "NX-E0103");
    assert_eq!(
        errors
            .last()
            .unwrap()
            .primary_span
            .as_ref()
            .map(|s| (s.start, s.end)),
        Some((310, 312))
    );
}

#[test]
fn tainted_annotated_let_invalidates_later_uses() {
    // F0-0002 §13.9: the bad element is the causal error; a later use
    // of the invalid binding cannot create a dependent Bool mismatch.
    let source = "fn Main(){let x:I64=[1,true];let y:Bool=x;}";
    let errors = check_proposed("tainted_let.n", source, false).unwrap_err();
    assert_eq!(errors.len(), 1, "{errors:?}");
    let error = &errors[0];
    assert_eq!(error.code, "NX-E0102");
    assert_eq!(error.subcause.as_deref(), Some("type_mismatch"));
    assert_eq!(
        error.primary_span.as_ref().map(|s| (s.start, s.end)),
        Some((23, 27))
    );
    assert_eq!(
        error.data.get("expected_type"),
        Some(&DataValue::String("I64".into()))
    );
    assert_eq!(
        error.data.get("found_type"),
        Some(&DataValue::String("Bool".into()))
    );
}

#[test]
fn call_arguments_keep_independent_contextual_errors() {
    // F0-0002 §§4.7, 13.9: each argument is checked against its own
    // written formal even after another argument has failed.
    let source = "fn F(a:view I64,b:view Bool){} fn Main(){F([1,true],1);}";
    let errors = check_proposed("call_args.n", source, false).unwrap_err();
    assert_eq!(errors.len(), 2, "{errors:?}");
    for (error, primary, expected, found) in [
        (&errors[0], (46, 50), "I64", "Bool"),
        (&errors[1], (52, 53), "Bool", "I64"),
    ] {
        assert_eq!(error.code, "NX-E0102");
        assert_eq!(error.subcause.as_deref(), Some("type_mismatch"));
        assert_eq!(
            error.primary_span.as_ref().map(|s| (s.start, s.end)),
            Some(primary)
        );
        assert_eq!(
            error.data.get("expected_type"),
            Some(&DataValue::String(expected.into()))
        );
        assert_eq!(
            error.data.get("found_type"),
            Some(&DataValue::String(found.into()))
        );
    }
}

#[test]
fn invalid_call_argument_suppresses_dependent_result_mismatch() {
    // F0-0002 §13.9: the argument error owns the invalid call value;
    // the Bool context must not add a mismatch on its Unit result.
    let source = "fn F(a:view I64){} fn Main(){let y:Bool=F(1+true);}";
    let errors = check_proposed("invalid_call.n", source, false).unwrap_err();
    assert_eq!(errors.len(), 1, "{errors:?}");
    let error = &errors[0];
    assert_eq!(error.code, "NX-E0102");
    assert_eq!(error.subcause.as_deref(), Some("type_mismatch"));
    assert_eq!(
        error.primary_span.as_ref().map(|s| (s.start, s.end)),
        Some((44, 48))
    );
    assert_eq!(
        error.data.get("expected_type"),
        Some(&DataValue::String("I64".into()))
    );
    assert_eq!(
        error.data.get("found_type"),
        Some(&DataValue::String("Bool".into()))
    );
}

#[test]
fn tainted_call_argument_suppresses_dependent_result_mismatch() {
    // F0-0002 §13.9: an array element mismatch taints the call even
    // though checking that argument returns a synthesized type.
    let source = "fn F(a:view I64){} fn Main(){let y:Bool=F([1,true]);}";
    let errors = check_proposed("tainted_call.n", source, false).unwrap_err();
    assert_eq!(errors.len(), 1, "{errors:?}");
    let error = &errors[0];
    assert_eq!(error.code, "NX-E0102");
    assert_eq!(error.subcause.as_deref(), Some("type_mismatch"));
    assert_eq!(
        error.primary_span.as_ref().map(|s| (s.start, s.end)),
        Some((45, 49))
    );
    assert_eq!(
        error.data.get("expected_type"),
        Some(&DataValue::String("I64".into()))
    );
    assert_eq!(
        error.data.get("found_type"),
        Some(&DataValue::String("Bool".into()))
    );
}

#[test]
fn deferred_assert_eq_keeps_its_known_unit_result() {
    // F0-0002 §§4.1.1, 4.6: deferred argument types do not make
    // AssertEq's declared Unit result a deferred value for lowering.
    let source = "fn Main(){var r=[];let x=AssertEq(r,r);r=[1];}";
    check_scalar("deferred_assert_eq_unit.n", source).unwrap();
    let nested = "fn Main(){var r=[];let x=(if true then{value:AssertEq(r,r);}else{value:AssertEq(r,r);});r=[1];}";
    check_scalar("nested_deferred_assert_eq_unit.n", nested).unwrap();
}

#[test]
fn valid_later_call_argument_does_not_clear_earlier_taint() {
    // F0-0002 §13.9: the second argument is valid, while the first
    // invalidates the call's value in its enclosing Bool context.
    let source = "fn F(a:view I64,b:view I64){} fn Main(){let y:Bool=F([1,true],1);}";
    let errors = check_proposed("call_taint_preserved.n", source, false).unwrap_err();
    assert_eq!(errors.len(), 1, "{errors:?}");
    assert_eq!(errors[0].code, "NX-E0102");
    assert_eq!(errors[0].subcause.as_deref(), Some("type_mismatch"));
    assert_eq!(
        errors[0].primary_span.as_ref().map(|s| (s.start, s.end)),
        Some((56, 60))
    );
    assert_eq!(
        errors[0].data.get("expected_type"),
        Some(&DataValue::String("I64".into()))
    );
    assert_eq!(
        errors[0].data.get("found_type"),
        Some(&DataValue::String("Bool".into()))
    );
}

#[test]
fn failed_call_argument_does_not_skip_later_argument() {
    // F0-0002 §§4.7, 13.9: both argument sites are independent.
    let source = "fn F(a:view I64,b:view I64){} fn Main(){F(1+true,1+true);}";
    let errors = check_proposed("two_bad_args.n", source, false).unwrap_err();
    assert_eq!(errors.len(), 2, "{errors:?}");
    for (error, primary) in [(&errors[0], (44, 48)), (&errors[1], (51, 55))] {
        assert_eq!(error.code, "NX-E0102");
        assert_eq!(error.subcause.as_deref(), Some("type_mismatch"));
        assert_eq!(
            error.primary_span.as_ref().map(|s| (s.start, s.end)),
            Some(primary)
        );
        assert_eq!(
            error.data.get("expected_type"),
            Some(&DataValue::String("I64".into()))
        );
        assert_eq!(
            error.data.get("found_type"),
            Some(&DataValue::String("Bool".into()))
        );
    }
}

#[test]
fn assert_eq_resource_constraint_blames_second_deferred_operand() {
    // F0-0002 §§4.1.2.2, 4.1.2.6: the bottom-only variable acquires the
    // resource type; both argument occurrences fail copyability independently.
    let source = "fn Main(){var r=(if true then{return;}else{return;});AssertEq(MockOpen(1),r);}";
    let errors = check_proposed("assert_second_resource.n", source, false).unwrap_err();
    assert_eq!(errors.len(), 2, "{errors:?}");
    for (error, span) in errors.iter().zip([(62, 73), (74, 75)]) {
        assert_eq!(error.code, "NX-E0102");
        assert_eq!(error.subcause.as_deref(), Some("type_mismatch"));
        assert_eq!(
            error.primary_span.as_ref().map(|s| (s.start, s.end)),
            Some(span)
        );
        assert_eq!(
            error.data.get("expected_type"),
            Some(&DataValue::String("domain:copyable_data".into()))
        );
        assert_eq!(
            error.data.get("found_type"),
            Some(&DataValue::String("MockResource".into()))
        );
        assert_eq!(error.related_spans.len(), 1);
        assert_eq!(error.related_spans[0].role, RelatedRole::FoundType);
        assert_eq!(
            (
                error.related_spans[0].span.start,
                error.related_spans[0].span.end
            ),
            span
        );
    }
}

#[test]
fn annotated_unresolved_array_keeps_written_binding_type() {
    // F0-0002 §§4.1.1, 13.15: the unresolved constructor owns its
    // error, while a later use still sees the written I64 annotation.
    let source = "fn Main(){let x:I64=[];let y:Bool=x;}";
    let errors = check_proposed("annotated_unresolved.n", source, false).unwrap_err();
    assert_eq!(errors.len(), 2, "{errors:?}");
    assert_eq!(errors[0].code, "NX-E0103");
    assert_eq!(
        errors[0].primary_span.as_ref().map(|s| (s.start, s.end)),
        Some((20, 22))
    );
    let error = &errors[1];
    assert_eq!(error.code, "NX-E0102");
    assert_eq!(error.subcause.as_deref(), Some("type_mismatch"));
    assert_eq!(
        error.primary_span.as_ref().map(|s| (s.start, s.end)),
        Some((34, 35))
    );
    assert_eq!(
        error.data.get("expected_type"),
        Some(&DataValue::String("Bool".into()))
    );
    assert_eq!(
        error.data.get("found_type"),
        Some(&DataValue::String("I64".into()))
    );
}

#[test]
fn divergent_array_binding_keeps_numeric_comparison_context() {
    // F0-0002 §4.1.2.4: the Array constructor is determined even when its
    // first element diverges; < compares operand types before its I64 domain.
    let source =
        "fn Main(){var r=[];var a=[(if true then{return;}else{return;}),r];let q=a<true;r=[1];}";
    let errors = check_proposed("divergent_array_less.n", source, false).unwrap_err();
    assert_eq!(errors.len(), 1, "{errors:?}");
    let error = &errors[0];
    assert_eq!(error.code, "NX-E0102");
    assert_eq!(error.subcause.as_deref(), Some("type_mismatch"));
    assert_eq!(
        error.primary_span.as_ref().map(|s| (s.start, s.end)),
        Some((74, 78))
    );
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
        (72, 73)
    );
    assert_eq!(error.related_spans[1].role, RelatedRole::FoundType);
    assert_eq!(
        (
            error.related_spans[1].span.start,
            error.related_spans[1].span.end
        ),
        (74, 78)
    );
}

#[test]
fn failed_nested_numeric_join_keeps_outer_partial_witness() {
    // F0-0002 §4.1.2.5: the valid inner continuation supplies a partial
    // witness to the outer union; its else value is checked independently.
    let source = "fn Main(){var r=[];let q=(if true then{value:(if true then{value:r+1;}else{value:1;});}else{value:r;});r=[true];}";
    let errors = check_proposed("nested_numeric_join.n", source, false).unwrap_err();
    assert_eq!(errors.len(), 2, "{errors:?}");
    let error = &errors[0];
    assert_eq!(error.code, "NX-E0102");
    assert_eq!(error.subcause.as_deref(), Some("type_mismatch"));
    assert_eq!(
        error.primary_span.as_ref().map(|s| (s.start, s.end)),
        Some((65, 66))
    );
    assert_eq!(
        error.data.get("expected_type"),
        Some(&DataValue::String("I64".into()))
    );
    assert_eq!(
        error.data.get("found_type"),
        Some(&DataValue::String("Array<Bool>".into()))
    );
    assert_eq!(errors[1].code, "NX-E1303");
    assert_eq!(
        errors[1].primary_span.as_ref().map(|s| (s.start, s.end)),
        Some((98, 99))
    );
    assert_eq!(
        errors[1].data.get("expected_type"),
        Some(&DataValue::String("I64".into()))
    );
    assert_eq!(
        errors[1].data.get("found_type"),
        Some(&DataValue::String("Array<Bool>".into()))
    );
}

#[test]
fn failed_nested_comparison_keeps_outer_partial_witness() {
    // F0-0002 §4.1.2.5: the valid true continuation supplies a partial
    // witness to the outer union despite the failed numeric sibling.
    let source = "fn Main(){var r=[];let q=(if true then{value:(if true then{value:(r+1)==1;}else{value:true;});}else{value:r;});r=[true];}";
    let errors = check_proposed("nested_comparison_join.n", source, false).unwrap_err();
    assert_eq!(errors.len(), 2, "{errors:?}");
    let error = &errors[0];
    assert_eq!(error.code, "NX-E0102");
    assert_eq!(error.subcause.as_deref(), Some("type_mismatch"));
    assert_eq!(
        error.primary_span.as_ref().map(|s| (s.start, s.end)),
        Some((66, 67))
    );
    assert_eq!(
        error.data.get("expected_type"),
        Some(&DataValue::String("I64".into()))
    );
    assert_eq!(
        error.data.get("found_type"),
        Some(&DataValue::String("Array<Bool>".into()))
    );
    assert_eq!(errors[1].code, "NX-E1303");
    assert_eq!(
        errors[1].primary_span.as_ref().map(|s| (s.start, s.end)),
        Some((106, 107))
    );
    assert_eq!(
        errors[1].data.get("expected_type"),
        Some(&DataValue::String("Bool".into()))
    );
    assert_eq!(
        errors[1].data.get("found_type"),
        Some(&DataValue::String("Array<Bool>".into()))
    );
}

#[test]
fn anchored_first_branch_keeps_independent_pending_join() {
    // F0-0002 §§4.1.1, 13.9: a failed later branch cannot erase
    // the earlier pending join between the two unresolved arrays.
    let source = "fn Main(){var r=[];var s=[];let q=(if true then{value:s;}else (if true then{value:(if true then{value:r;}else{value:[];});}else{value:s<1;}));r=[[1]];s=[[]];}";
    let errors = check_proposed("anchored_pending_join.n", source, false).unwrap_err();
    assert_eq!(errors.len(), 3, "{errors:?}");
    assert_eq!(
        errors.iter().map(|e| e.code.as_str()).collect::<Vec<_>>(),
        ["NX-E1303", "NX-E1303", "NX-E0102"]
    );
    assert_eq!(
        errors
            .iter()
            .map(|e| e.primary_span.as_ref().map(|s| (s.start, s.end)))
            .collect::<Vec<_>>(),
        [Some((63, 139)), Some((134, 137)), Some((136, 137))]
    );
    for error in &errors[0..2] {
        assert_eq!(
            error.data.get("expected_type"),
            Some(&DataValue::String("Array<Array<I64>>".into()))
        );
        assert_eq!(
            error.data.get("found_type"),
            Some(&DataValue::String("Bool".into()))
        );
    }
    assert_eq!(errors[2].subcause.as_deref(), Some("type_mismatch"));
}

#[test]
fn deferred_comparison_keeps_valid_operand_domain_after_sibling_failure() {
    // F0-0002 §4.1.2.4–5, RD95: the failed right value suppresses only
    // cross-operand equality. The left array's I64 domain remains a site.
    for source in [
        "fn Main(){var r=[];let a0=r<(true+1);r=[true];}",
        "fn Main(){var r=[true];let a0=r<(true+1);}",
    ] {
        let errors = check_proposed("pending_comparison_domain.n", source, false).unwrap_err();
        assert_eq!(errors.len(), 2, "{source}: {errors:?}");
        assert!(errors.iter().all(|e| e.code == "NX-E0102"));
        assert!(
            errors
                .iter()
                .all(|e| e.subcause.as_deref() == Some("type_mismatch"))
        );
        let op = source.find("r<").unwrap() as u64 + 1;
        assert_eq!(
            errors[0].primary_span.as_ref().map(|s| (s.start, s.end)),
            Some((op, op + 1))
        );
        assert_eq!(
            errors[0].data.get("found_type"),
            Some(&DataValue::String("Array<Bool>".into()))
        );
        assert_eq!(
            errors[1].data.get("found_type"),
            Some(&DataValue::String("Bool".into()))
        );
    }
}

#[test]
fn deferred_comparison_distinguishes_cross_equality_from_inner_join() {
    // F0-0002 §4.1.2.4–5, RD95: a failed cross equality ends before
    // domain checking, while a failed join inside the right value leaves
    // the independent left domain to check.
    for binding in ["var r=[];", "var r=[true];"] {
        let cross = format!("fn Main(){{{binding}let a=true<r;r=[true];}}");
        let errors = check_proposed("cross_equality.n", &cross, false).unwrap_err();
        assert_eq!(errors.len(), 1, "{cross}: {errors:?}");
        assert_eq!(errors[0].code, "NX-E0102");
        assert_eq!(
            errors[0].primary_span.as_ref().map(|s| (s.start, s.end)),
            Some((
                cross.find("<r").unwrap() as u64 + 1,
                cross.find("<r").unwrap() as u64 + 2
            ))
        );

        let inner = format!(
            "fn Main(){{let p=true;let x=1;{binding}let a=true<(if p then r else x);r=[true];}}"
        );
        let errors = check_proposed("inner_join.n", &inner, false).unwrap_err();
        assert_eq!(errors.len(), 2, "{inner}: {errors:?}");
        assert_eq!(errors[0].code, "NX-E0102");
        let op = inner.find("true<").unwrap() as u64 + 4;
        assert_eq!(
            errors[0].primary_span.as_ref().map(|s| (s.start, s.end)),
            Some((op, op + 1))
        );
        assert_eq!(errors[1].code, "NX-E1303");
    }
}

#[test]
fn rd127_constructor_requires_its_own_element_resolution() {
    // F0-0002 §4.1.2.2, §20 and RD127(a): the constructor [q] is rejected
    // even when the hypothetical binding q itself need not materialize.
    let deferred = "fn Main(){var q=(if true then {return;}else{return;});let r=[q];}";
    let errors = check_proposed("rd127_q.n", deferred, false).unwrap_err();
    assert_eq!(errors.len(), 1, "{errors:?}");
    assert_eq!(errors[0].code, "NX-E0103");
    let start = deferred.find("[q]").unwrap() as u64;
    assert_eq!(
        errors[0].primary_span.as_ref().map(|s| (s.start, s.end)),
        Some((start, start + 3))
    );
    assert!(check_proposed("rd127_concrete.n", "fn Main(){let r=[1];}", false).is_ok());
}
