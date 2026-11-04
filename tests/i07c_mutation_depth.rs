use nexo::check::check_proposed;
use nexo::diagnostic::{DataValue, RelatedRole};

#[test]
fn inferred_array_wrappers_preserve_both_result_component_depths() {
    // Inferred Result wrappers obey the value-depth limit of F0-0002 §13.13.
    for left in [true, false] {
        let mut ty = "I64".to_owned();
        for _ in 0..127 {
            ty = if left {
                format!("Result<{ty},I64>")
            } else {
                format!("Result<I64,{ty}>")
            };
        }
        let source = format!("fn F(p:view {ty}){{let a=[[p]];}} fn Main(){{}}");
        let errors = check_proposed("result_depth.n", &source, false).unwrap_err();
        assert_eq!(errors.len(), 1, "{errors:?}");
        let error = &errors[0];
        assert_eq!(error.code, "NX-E0003");
        assert_eq!(
            error.data.get("budget_kind"),
            Some(&DataValue::String("value_type_depth".into()))
        );
        assert_eq!(error.data.get("limit"), Some(&DataValue::UInt(128)));
        assert_eq!(error.data.get("observed"), Some(&DataValue::UInt(129)));
        let at = source.find("[[p]]").unwrap() as u64;
        assert_eq!(
            error
                .primary_span
                .as_ref()
                .map(|span| (span.start, span.end)),
            Some((at, at + 5))
        );
    }
}

#[test]
fn indexed_binding_preserves_inferred_depth_inside_each_result_component() {
    for left in [true, false] {
        let inner = format!("{}I64{}", "Array<".repeat(126), ">".repeat(126));
        let ty = if left {
            format!("Result<{inner},I64>")
        } else {
            format!("Result<I64,{inner}>")
        };
        let source = format!("fn F(p:view {ty}){{var v=[[[]]];v[0][0][0]=p;}} fn Main(){{}}");
        let errors = check_proposed("inferred_result_depth.n", &source, false).unwrap_err();
        assert_eq!(errors.len(), 1, "{errors:?}");
        let error = &errors[0];
        assert_eq!(error.code, "NX-E0003");
        assert_eq!(
            error.data.get("budget_kind"),
            Some(&DataValue::String("array_type_depth".into()))
        );
        assert_eq!(error.data.get("limit"), Some(&DataValue::UInt(128)));
        assert_eq!(error.data.get("observed"), Some(&DataValue::UInt(129)));
    }
}

#[test]
fn adr_gap_result_component_mismatch_keeps_written_expected_type() {
    for (parameter, result, expected, found) in [
        ("Result<Bool,Bool>", "Result<I64,Bool>", "I64", "Bool"),
        ("Result<I64,I64>", "Result<I64,Bool>", "Bool", "I64"),
    ] {
        let source = format!("fn F(x:view {parameter})->{result}{{return x;}} fn Main(){{}}");
        let errors = check_proposed("result_component.n", &source, false).unwrap_err();
        assert_eq!(errors.len(), 1, "{source}: {errors:?}");
        let error = &errors[0];
        assert_eq!(error.code, "NX-E0102", "{source}: {errors:?}");
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
fn nested_deferred_never_constraint_keeps_later_concrete_origin() {
    // F0-0002 §§4.1.1, 4.1.2.6, 13.15: the later Bool array fixes the type;
    // AssertEq compares against its first argument occurrence as the origin.
    let source = "fn Main(){var r=[[]];AssertEq(r,[if true then {return;} else {return;}]);r=[[true]];AssertEq(r,[[1]]);}";
    let errors = check_proposed("nested_never.n", source, false).unwrap_err();
    assert_eq!(errors.len(), 1, "{errors:?}");
    let error = &errors[0];
    assert_eq!(error.code, "NX-E0102");
    assert_eq!(error.subcause.as_deref(), Some("type_mismatch"));
    assert_eq!(
        error
            .primary_span
            .as_ref()
            .map(|span| (span.start, span.end)),
        Some((95, 100))
    );
    assert_eq!(
        error.data.get("expected_type"),
        Some(&DataValue::String("Array<Array<Bool>>".into()))
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
        (93, 94)
    );
    assert_eq!(error.related_spans[1].role, RelatedRole::FoundType);
}
