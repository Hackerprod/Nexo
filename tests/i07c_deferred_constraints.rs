use nexo::check::check_proposed;
use nexo::diagnostic::{DataValue, Operation, RelatedRole, Stage};
use nexo::ir::{CompileError, check_scalar, compile_scalar};

const DV: &str = "(if true then {return;} else {return;})";

#[test]
fn independent_numeric_causes_survive_a_contaminated_binding() {
    // F0-0002 §4.1.2: each source occurrence owns its C; reading x adds none.
    let source = "fn main(){let x=true+1;let y=x;let z=false+1;}";
    let errors = check_proposed("independent.n", source, false).unwrap_err();
    assert_eq!(errors.len(), 2, "{errors:?}");
    for (mut error, (expected_span, token)) in errors
        .into_iter()
        .zip([((16, 20), "true"), ((37, 42), "false")])
    {
        error.publish("independent.n", source).unwrap();
        assert_eq!(
            (error.code.as_str(), error.subcause.as_deref()),
            ("NX-E0102", Some("type_mismatch"))
        );
        assert_eq!(
            (error.stage, error.operation),
            (Stage::Types, Operation::TypesCheck)
        );
        let primary = error.primary_span.as_ref().unwrap();
        assert_eq!((primary.start, primary.end), expected_span);
        assert_eq!(&source[primary.start as usize..primary.end as usize], token);
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
            expected_span
        );
    }
    for repaired in [
        "fn main(){let x=1+1;let y=x;let z=false+1;}",
        "fn main(){let x=true+1;let y=x;let z=1+1;}",
    ] {
        assert_eq!(
            check_proposed("independent.n", repaired, false)
                .unwrap_err()
                .len(),
            1
        );
    }
    assert!(
        check_proposed(
            "independent.n",
            "fn main(){let x=1+1;let y=x;let z=1+1;}",
            false
        )
        .is_ok()
    );
}

#[test]
fn unresolved_constructor_keeps_its_own_site_and_accepts_a_finite_element() {
    // F0-0002 §4.1.2.2: the constructor closes at its original occurrence.
    let source = "fn Main(){var r=[];r=[r];}";
    let errors = check_proposed("constructor.n", source, false).unwrap_err();
    assert_eq!(errors.len(), 1, "{errors:?}");
    let error = &errors[0];
    assert_eq!(error.code, "NX-E0103");
    let primary = error.primary_span.as_ref().unwrap();
    assert_eq!((primary.start, primary.end), (16, 18));
    assert_eq!(&source[primary.start as usize..primary.end as usize], "[]");
    assert!(check_proposed("constructor.n", "fn Main(){var r=[];r=[1];}", false).is_ok());
}

#[test]
fn deferred_array_reports_its_first_incompatible_element() {
    // F0-0002 §4.1.2.2: the later incompatible element does not move its C.
    let source = "fn Main(){var r=[];r=[(r==r)==true];r=[1];}";
    let errors = check_proposed("array.n", source, false).unwrap_err();
    assert_eq!(errors.len(), 1, "{errors:?}");
    let mut error = errors.into_iter().next().unwrap();
    error.publish("array.n", source).unwrap();
    assert_eq!(
        (error.code.as_str(), error.subcause.as_deref()),
        ("NX-E0102", Some("type_mismatch"))
    );
    assert_eq!(
        (error.stage, error.operation),
        (Stage::Types, Operation::TypesCheck)
    );
    let primary = error.primary_span.as_ref().unwrap();
    assert_eq!((primary.start, primary.end), (39, 40));
    assert_eq!(&source[primary.start as usize..primary.end as usize], "1");
    assert_eq!(
        error.data.get("expected_type"),
        Some(&DataValue::String("Bool".into()))
    );
    assert_eq!(
        error.data.get("found_type"),
        Some(&DataValue::String("I64".into()))
    );
    assert!(error.related_spans.iter().any(|related| {
        related.role == RelatedRole::ExpectedType
            && (related.span.start, related.span.end) == (21, 35)
    }));
    assert!(
        check_proposed(
            "array.n",
            "fn Main(){var r=[];r=[(r==r)==true];r=[true];}",
            false
        )
        .is_ok()
    );
}

#[test]
fn pattern_shape_conflicts_belong_to_later_equality() {
    for (body, expected, primary, expected_span, origin_span) in [
        (
            "let l0=DV;match l0{Some(x)=>{},_=>{}}let b:Bool=l0;",
            "Bool",
            (95, 97),
            (90, 94),
            (66, 73),
        ),
        (
            "let l0=DV;match l0{Some(x)=>{},_=>{}}let b:Result<I64,I64>=l0;",
            "Result<I64,I64>",
            (106, 108),
            (90, 105),
            (66, 73),
        ),
        (
            "let l0=DV;match l0{Ok(Some(x))=>{},_=>{}}let b:Result<Bool,I64>=l0;",
            "Result<Bool,I64>",
            (111, 113),
            (94, 110),
            (69, 76),
        ),
    ] {
        let src = source(body);
        for execute in [false, true] {
            let Err(CompileError::Rejected(mut errors)) = nexo::ir::compile_scalar_with_profile(
                "deferred_constraints.n",
                &src,
                nexo::ir::ReducedProfile::JNormal,
                execute,
            ) else {
                panic!("expected rejection: {src}")
            };
            assert_eq!(errors.len(), 1, "{src}: {errors:?}");
            let mut error = errors.remove(0);
            error.publish("deferred_constraints.n", &src).unwrap();
            assert_eq!(
                (error.code.as_str(), error.subcause.as_deref()),
                ("NX-E0102", Some("constructor_shape"))
            );
            assert_eq!(
                (error.stage, error.operation),
                (Stage::Types, Operation::TypesCheck)
            );
            assert_eq!(
                (
                    error.primary_span.as_ref().unwrap().start,
                    error.primary_span.as_ref().unwrap().end
                ),
                primary
            );
            assert_eq!(&src[primary.0 as usize..primary.1 as usize], "l0");
            assert_eq!(error.data.len(), 2);
            assert_eq!(
                error.data.get("expected_type"),
                Some(&DataValue::String(expected.into()))
            );
            assert_eq!(
                error.data.get("found_constructor"),
                Some(&DataValue::String("Option".into()))
            );
            assert_eq!(
                error
                    .related_spans
                    .iter()
                    .map(|r| (r.role, r.span.start, r.span.end))
                    .collect::<Vec<_>>(),
                vec![
                    (RelatedRole::Origin, origin_span.0, origin_span.1),
                    (RelatedRole::ExpectedType, expected_span.0, expected_span.1),
                    (RelatedRole::FoundType, primary.0, primary.1),
                ]
            );
            assert!(error.cause_chain.is_empty());
            assert!(error.repair.is_empty());
            assert!(error.feature_id.is_none());
            assert!(error.unexpected_token.is_none());
            assert!(error.expected_tokens.is_empty());
            assert!(error.expected_tokens_kind.is_none());
            assert!(error.parser_context.is_none());
            assert!(error.recognized_prefix.is_none());
        }
    }
    let positive = source("let l0=DV;match l0{Some(x)=>{},_=>{}}let b:Option<I64>=l0;");
    for execute in [false, true] {
        nexo::ir::compile_scalar_with_profile(
            "deferred_constraints.n",
            &positive,
            nexo::ir::ReducedProfile::JNormal,
            execute,
        )
        .unwrap();
    }
}

#[test]
fn value_constructors_still_require_closed_live_components() {
    let unresolved = "fn Main(){let l0=None;match l0{Some(x)=>{},None=>{}}}";
    for execute in [false, true] {
        let Err(CompileError::Rejected(errors)) = nexo::ir::compile_scalar_with_profile(
            "deferred_constraints.n",
            unresolved,
            nexo::ir::ReducedProfile::JNormal,
            execute,
        ) else {
            panic!("expected NX-E0113")
        };
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].code, "NX-E0113");
    }
    let closed = "fn Main(){let l0:Option<I64>=Some(1);match l0{Some(x)=>{},None=>{}}}";
    for execute in [false, true] {
        nexo::ir::compile_scalar_with_profile(
            "deferred_constraints.n",
            closed,
            nexo::ir::ReducedProfile::JNormal,
            execute,
        )
        .unwrap();
    }
}

#[test]
fn partial_option_domain_has_no_internal_failure() {
    let src = source("let l0=DV;match l0{Some(x)=>{},_=>{}}let n=Len(l0);");
    for execute in [false, true] {
        assert!(
            matches!(
                nexo::ir::compile_scalar_with_profile(
                    "deferred_constraints.n",
                    &src,
                    nexo::ir::ReducedProfile::JNormal,
                    execute
                ),
                Ok(_) | Err(CompileError::Rejected(_))
            ),
            "{src}"
        );
    }
}

fn source(body: &str) -> String {
    format!("fn Main(){{{}}}", body.replace("DV", DV))
}

struct Expected<'a> {
    body: &'a str,
    primary: (u64, u64),
    token: &'a str,
    subcause: &'a str,
    expected_type: &'a str,
    found_type: &'a str,
    related: &'a [(RelatedRole, u64, u64)],
}

fn assert_single_c(expected: &Expected<'_>) {
    let source = source(expected.body);
    let errors = check_proposed("deferred_constraints.n", &source, false).unwrap_err();
    assert_eq!(errors.len(), 1, "{}: {errors:?}", expected.body);
    let mut error = errors.into_iter().next().unwrap();
    error.publish("deferred_constraints.n", &source).unwrap();
    assert_eq!(error.code, "NX-E0102");
    assert_eq!(error.subcause.as_deref(), Some(expected.subcause));
    assert_eq!(error.stage, Stage::Types);
    assert_eq!(error.operation, Operation::TypesCheck);
    assert_eq!(error.rule, format!("NX-E0102/{}", expected.subcause));
    assert_eq!(error.explanation_id.as_deref(), Some(error.rule.as_str()));
    assert_eq!(
        error
            .primary_span
            .as_ref()
            .map(|span| (span.start, span.end)),
        Some(expected.primary)
    );
    assert_eq!(
        &source[expected.primary.0 as usize..expected.primary.1 as usize],
        expected.token
    );
    assert_eq!(error.data.len(), 2);
    assert_eq!(
        error.data.get("expected_type"),
        Some(&DataValue::String(expected.expected_type.into()))
    );
    assert_eq!(
        error.data.get("found_type"),
        Some(&DataValue::String(expected.found_type.into()))
    );
    let related: Vec<_> = error
        .related_spans
        .iter()
        .map(|item| (item.role, item.span.start, item.span.end))
        .collect();
    assert_eq!(related, expected.related);
    assert!(error.cause_chain.is_empty());
    assert!(error.repair.is_empty());
    assert_eq!(error.feature_id, None);
    assert_eq!(error.unexpected_token, None);
    assert!(error.expected_tokens.is_empty());
    assert_eq!(error.expected_tokens_kind, None);
    assert_eq!(error.parser_context, None);
    assert_eq!(error.recognized_prefix, None);
    assert!(matches!(
        check_scalar("deferred_constraints.n", &source),
        Err(CompileError::Rejected(_))
    ));
    assert!(matches!(
        compile_scalar("deferred_constraints.n", &source),
        Err(CompileError::Rejected(_))
    ));
}

fn assert_valid(body: &str) {
    let source = source(body);
    assert!(
        check_proposed("deferred_constraints.n", &source, false).is_ok(),
        "{source}"
    );
    assert!(
        check_scalar("deferred_constraints.n", &source).is_ok(),
        "{source}"
    );
    assert!(
        compile_scalar("deferred_constraints.n", &source).is_ok(),
        "{source}"
    );
}

#[test]
fn nominal_results_reach_incompatible_consumers() {
    // F0-0002 §4.1.2.3, cierre de dominio indeciso; §13.15.
    for expected in [
        Expected {
            body: "var q=DV;let z:Bool=DV+Len(q);",
            primary: (67, 113),
            token: "(if true then {return;} else {return;})+Len(q)",
            subcause: "type_mismatch",
            expected_type: "Bool",
            found_type: "I64",
            related: &[
                (RelatedRole::ExpectedType, 62, 66),
                (RelatedRole::FoundType, 67, 113),
            ],
        },
        Expected {
            body: "var q=DV;let z:I64=DV<Len(q);",
            primary: (66, 112),
            token: "(if true then {return;} else {return;})<Len(q)",
            subcause: "type_mismatch",
            expected_type: "I64",
            found_type: "Bool",
            related: &[
                (RelatedRole::ExpectedType, 62, 65),
                (RelatedRole::FoundType, 66, 112),
            ],
        },
        Expected {
            body: "var q=DV;let z:I64=DV==(q==q);",
            primary: (66, 113),
            token: "(if true then {return;} else {return;})==(q==q)",
            subcause: "type_mismatch",
            expected_type: "I64",
            found_type: "Bool",
            related: &[
                (RelatedRole::ExpectedType, 62, 65),
                (RelatedRole::FoundType, 66, 113),
            ],
        },
        Expected {
            body: "var q=DV;let z:I64=Len(q)<DV;",
            primary: (66, 112),
            token: "Len(q)<(if true then {return;} else {return;})",
            subcause: "type_mismatch",
            expected_type: "I64",
            found_type: "Bool",
            related: &[
                (RelatedRole::ExpectedType, 62, 65),
                (RelatedRole::FoundType, 66, 112),
            ],
        },
        Expected {
            body: "let br=[true];var q=DV;let a0=(DV-Len(q))<1;let a1=a0==br;",
            primary: (139, 141),
            token: "br",
            subcause: "type_mismatch",
            expected_type: "Bool",
            found_type: "Array<Bool>",
            related: &[
                (RelatedRole::ExpectedType, 135, 137),
                (RelatedRole::FoundType, 139, 141),
            ],
        },
    ] {
        assert_single_c(&expected);
    }
    assert_valid("var q=DV;let z:I64=DV+Len(q);");
    assert_valid("var q=DV;let z:Bool=DV<Len(q);");
    assert_valid("var q=DV;let z:Bool=DV==(q==q);");
    assert_valid("var q=DV;let z:Bool=Len(q)<DV;");
    assert_valid("let br=true;var q=DV;let a0=(DV-Len(q))<1;let a1=a0==br;");
}

#[test]
fn nominal_result_fixes_circular_domain_before_its_failure() {
    // F0-0002 §4.1.2.3, table "Ejemplos de cierre del dominio".
    for expected in [
        Expected {
            body: "var r=DV;let a0=DV+(DV-Len(r));let b=[a0,r];",
            primary: (148, 149),
            token: "r",
            subcause: "not_indexable",
            expected_type: "domain:array",
            found_type: "I64",
            related: &[(RelatedRole::FoundType, 148, 149)],
        },
        Expected {
            body: "var r=DV;let a0=Len(r);let b=[a0,r];",
            primary: (67, 68),
            token: "r",
            subcause: "not_indexable",
            expected_type: "domain:array",
            found_type: "I64",
            related: &[(RelatedRole::FoundType, 67, 68)],
        },
        Expected {
            body: "var r=DV;let a0=1+Len(r);let b=[a0,r];",
            primary: (69, 70),
            token: "r",
            subcause: "not_indexable",
            expected_type: "domain:array",
            found_type: "I64",
            related: &[(RelatedRole::FoundType, 69, 70)],
        },
        Expected {
            body: "var r=DV;let a0=Len(r);let c:Bool=a0;let b=[a0,r];",
            primary: (67, 68),
            token: "r",
            subcause: "not_indexable",
            expected_type: "domain:array",
            found_type: "I64",
            related: &[(RelatedRole::FoundType, 67, 68)],
        },
        Expected {
            body: "let r=3;let a0=1+(1-Len(r));let b=[a0,r];",
            primary: (34, 35),
            token: "r",
            subcause: "not_indexable",
            expected_type: "domain:array",
            found_type: "I64",
            related: &[(RelatedRole::FoundType, 34, 35)],
        },
        Expected {
            body: "var r=DV;let b=[1,r];let a0=Len(r);",
            primary: (79, 80),
            token: "r",
            subcause: "not_indexable",
            expected_type: "domain:array",
            found_type: "I64",
            related: &[(RelatedRole::FoundType, 79, 80)],
        },
    ] {
        assert_single_c(&expected);
    }
    assert_valid("var r=DV;let a0=DV+(DV-Len(r));let b=[a0,1];");
    assert_valid("var r=DV;let a0=Len(r);let b=[a0,1];");
    assert_valid("var r=DV;let a0=1+Len(r);let b=[a0,1];");
    assert_valid("let r=3;let a0=1+(1-3);let b=[a0,r];");
    assert_valid("var r=DV;let b=[[1],r];let a0=Len(r);");
}

#[test]
fn failed_nominal_withdraws_only_dependent_diagnostics() {
    // F0-0002 §4.1.2.3 and N(d): the earlier a0 error is derived, while
    // true+1 owns a separate numeric-domain C.
    let source = source("var r=DV;let a0=Len(r);let c:Bool=a0;let d=true+1;let b=[a0,r];");
    let errors = check_proposed("deferred_constraints.n", &source, false).unwrap_err();
    assert_eq!(errors.len(), 2, "{errors:?}");
    let sites: Vec<_> = errors
        .iter()
        .map(|error| {
            let span = error.primary_span.as_ref().unwrap();
            (
                error.code.as_str(),
                error.subcause.as_deref(),
                span.start,
                span.end,
            )
        })
        .collect();
    assert_eq!(
        sites,
        [
            ("NX-E0102", Some("not_indexable"), 67, 68),
            ("NX-E0102", Some("type_mismatch"), 90, 94)
        ]
    );
    assert!(matches!(
        check_scalar("deferred_constraints.n", &source),
        Err(CompileError::Rejected(_))
    ));
    assert!(matches!(
        compile_scalar("deferred_constraints.n", &source),
        Err(CompileError::Rejected(_))
    ));
}

#[test]
fn compatible_constraints_survive_indecision_and_reject_later_conflicts() {
    // F0-0002 §§4.1.1 and 4.1.2.2–5; only the later incompatible use fails.
    for expected in [
        Expected {
            body: "let p=true;let ar=[1];let aar=[[1]];var r=[DV];let a=(if (r==r)==p then ar else ar);let c=a==r;r=aar;",
            primary: (144, 147),
            token: "aar",
            subcause: "type_mismatch",
            expected_type: "Array<I64>",
            found_type: "Array<Array<I64>>",
            related: &[
                (RelatedRole::ExpectedType, 137, 138),
                (RelatedRole::FoundType, 144, 147),
            ],
        },
        Expected {
            body: "let p=true;let ar=[1];let aar=[[1]];var r=[DV];let c=(if (r==r)==p then 1 else 2);let d:Bool=c;r=aar;",
            primary: (140, 141),
            token: "c",
            subcause: "type_mismatch",
            expected_type: "Bool",
            found_type: "I64",
            related: &[
                (RelatedRole::ExpectedType, 135, 139),
                (RelatedRole::FoundType, 140, 141),
            ],
        },
    ] {
        assert_single_c(&expected);
    }
    assert_valid(
        "let p=true;let ar=[1];var r=[DV];let a=(if (r==r)==p then ar else ar);let c=a==r;r=ar;",
    );
    assert_valid(
        "let p=true;let ar=[1];var r=[DV];let c=(if (r==r)==p then 1 else 2);let d:I64=c;r=ar;",
    );
}

#[test]
fn deferred_constraint_sequences_never_reach_internal_failure() {
    // Directed, reproducible syntax-valid fuzz for F0-0002 §4.1.2 N(f).
    // The same test runs in debug and release through validate.sh.
    let operations = [
        "Len(r)",
        "1+Len(r)",
        "DV+(DV-Len(r))",
        "Len(r)+1",
        "r==r",
        "(r==r)==true",
        "Len(r)<2",
    ];
    let consumers = [
        "let b=[a0,r];",
        "let b=[a0,1];",
        "let b=[a0,true];",
        "let c:Bool=a0;let b=[a0,r];",
        "let c:I64=a0;let b=[a0,r];",
    ];
    let assignments = ["", "r=1;", "r=true;", "r=[1];", "r=[true];"];
    for seed in [17021_u64, 17022, 10495373245766874321] {
        let mut state = seed;
        for _ in 0..256 {
            let mut pick = |limit: usize| {
                state = state.wrapping_mul(6364136223846793005).wrapping_add(1);
                (state >> 32) as usize % limit
            };
            let operation = operations[pick(operations.len())];
            let consumer = consumers[pick(consumers.len())];
            let assignment = assignments[pick(assignments.len())];
            let source = source(&format!(
                "var r=DV;let a0={operation};{consumer}{assignment}"
            ));
            match check_proposed("deferred_fuzz.n", &source, false) {
                Ok(()) => {}
                Err(errors) => {
                    for error in errors {
                        error.validate("deferred_fuzz.n", &source).unwrap();
                    }
                }
            }
            assert!(
                matches!(
                    check_scalar("deferred_fuzz.n", &source),
                    Ok(_) | Err(CompileError::Rejected(_))
                ),
                "{source}"
            );
            assert!(
                matches!(
                    compile_scalar("deferred_fuzz.n", &source),
                    Ok(_) | Err(CompileError::Rejected(_))
                ),
                "{source}"
            );
        }
    }
}
