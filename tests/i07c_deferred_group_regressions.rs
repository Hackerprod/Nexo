use nexo::check::check_proposed;
use nexo::diagnostic::DataValue;
use nexo::ir::{CompileError, check_scalar};

#[test]
fn pending_operation_consumers_match_concrete_causal_twins() {
    // ADR §§4.1.2.3–5,7: a pending inner domain cannot erase the outer
    // operator's independent domain, cross-operand equality, or the
    // consumer's expected type after that inner domain resolves valid.
    let cases = [
        (
            "fn Main(){var r=[];let a=false<(r==r);r=[[1]];}",
            "fn Main(){var r=[[1]];let a=false<(r==r);}",
            "<",
            "I64",
            "Bool",
        ),
        (
            "fn Main(){var r=[];var v:Array<Bool>=[false];v=Len(r[190])+70;r=[[1]];}",
            "fn Main(){var r=[[1]];var v:Array<Bool>=[false];v=Len(r[190])+70;}",
            "Len(r[190])+70",
            "Array<Bool>",
            "I64",
        ),
        (
            "fn Main(){var r=[];var v:Array<Bool>=[false];v=r==(r==r);r=[[1]];}",
            "fn Main(){var r=[[1]];var v:Array<Bool>=[false];v=r==(r==r);}",
            "r==r",
            "Array<Array<I64>>",
            "Bool",
        ),
        (
            "fn Main(){var r=[];var ar=[1];ar[true==(r==r)]=1;r=[true];}",
            "fn Main(){var r=[true];var ar=[1];ar[true==(r==r)]=1;}",
            "true==(r==r)",
            "I64",
            "Bool",
        ),
    ];
    for (deferred, concrete, primary, expected, found) in cases {
        for source in [deferred, concrete] {
            let errors = check_proposed("pending_consumer.n", source, false).expect_err(source);
            assert_eq!(errors.len(), 1, "{source}: {errors:?}");
            let error = &errors[0];
            error.validate("pending_consumer.n", source).unwrap();
            let span = error.primary_span.as_ref().unwrap();
            assert_eq!(&source[span.start as usize..span.end as usize], primary);
            assert_eq!(error.code, "NX-E0102");
            assert_eq!(error.subcause.as_deref(), Some("type_mismatch"));
            assert_eq!(
                error.data.get("expected_type"),
                Some(&DataValue::String(expected.into()))
            );
            assert_eq!(
                error.data.get("found_type"),
                Some(&DataValue::String(found.into()))
            );
            assert!(matches!(
                check_scalar("pending_consumer.n", source),
                Err(CompileError::Rejected(_))
            ));
        }
    }
}

#[test]
fn valid_pending_comparison_results_lower_like_concrete_twins() {
    // ADR §§4.1.2.3–5,7: once the inner domain resolves valid and the
    // cross comparison succeeds, its Bool result is usable by the IR.
    let cases = [
        (
            "fn Main(){var r=[];let a=true==(r==r);r=[[1]];}",
            "fn Main(){var r=[[1]];let a=true==(r==r);}",
        ),
        (
            "fn Main(){var r=[];let a=true==(r==r);let b=a==false;r=[true];}",
            "fn Main(){let r=[true];let a=true==(r==r);let b=a==false;}",
        ),
        (
            "fn FB(a:view Bool){} fn Main(){var r=[];FB((1==1)==(r==r));r=[true];}",
            "fn FB(a:view Bool){} fn Main(){var r=[true];FB((1==1)==(r==r));}",
        ),
        (
            "fn FB(a:view Bool){} fn Main(){var r=[];FB((if true then (r==r)==([1]==r) else {value:true;}));r=[1];}",
            "fn FB(a:view Bool){} fn Main(){var r=[1];FB((if true then (r==r)==([1]==r) else {value:true;}));}",
        ),
        (
            "fn Main(){var r=[];let a:Array<I64>=(if true then [1] else {value:(if true==(r==r) then {value:[1];} else {value:[2];});});r=[true];}",
            "fn Main(){var r=[true];let a:Array<I64>=(if true then [1] else {value:(if true==(r==r) then {value:[1];} else {value:[2];});});}",
        ),
    ];
    for (deferred, concrete) in cases {
        for source in [deferred, concrete] {
            assert!(
                check_proposed("valid_pending_consumer.n", source, false).is_ok(),
                "{source}"
            );
            let lowered = check_scalar("valid_pending_consumer.n", source);
            assert!(lowered.is_ok(), "{source}: {lowered:?}");
        }
    }
}

#[test]
fn pending_comparison_local_use_keeps_its_own_numeric_conflict() {
    // ADR §§4.1.2.1,5–6: the later completion of r cannot erase the
    // incompatible use of the comparison result at `a`.
    for source in [
        "fn Main(){var r=[];let a=true==(r==r);let b=a+1;r=[true];}",
        "fn Main(){let r=[true];let a=true==(r==r);let b=a+1;}",
    ] {
        let errors = check_proposed("pending_local.n", source, false).expect_err(source);
        assert_eq!(errors.len(), 1, "{source}: {errors:?}");
        let error = &errors[0];
        error.validate("pending_local.n", source).unwrap();
        let span = error.primary_span.as_ref().unwrap();
        assert_eq!(&source[span.start as usize..span.end as usize], "a");
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
}
