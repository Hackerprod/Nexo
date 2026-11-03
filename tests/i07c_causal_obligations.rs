use nexo::check::check_proposed;
use nexo::diagnostic::{DataValue, RelatedRole};
use nexo::ir::{CompileError, check_scalar, compile_scalar};

type Cause<'a> = (&'a str, Option<&'a str>, &'a str);

fn causes(source: &str) -> Vec<(String, Option<String>, String)> {
    check_proposed("causal_obligations.n", source, false)
        .err()
        .unwrap_or_default()
        .iter()
        .map(|error| {
            error.validate("causal_obligations.n", source).unwrap();
            let span = error.primary_span.as_ref().unwrap();
            (
                error.code.clone(),
                error.subcause.clone(),
                source[span.start as usize..span.end as usize].to_owned(),
            )
        })
        .collect()
}

#[allow(clippy::too_many_arguments)] // Each argument is an independently checked diagnostic field.
fn exact_cause(
    source: &str,
    index: usize,
    code: &str,
    subcause: Option<&str>,
    token: &str,
    expected: Option<&str>,
    found: Option<&str>,
    related: &[(RelatedRole, &str)],
) {
    let errors = check_proposed("causal_obligations.n", source, false).unwrap_err();
    let error = &errors[index];
    error.validate("causal_obligations.n", source).unwrap();
    assert_eq!(error.code, code);
    assert_eq!(error.subcause.as_deref(), subcause);
    let span = error.primary_span.as_ref().unwrap();
    assert_eq!(&source[span.start as usize..span.end as usize], token);
    assert_eq!(
        error.data.len(),
        expected.is_some() as usize + found.is_some() as usize
    );
    assert_eq!(
        error.data.get("expected_type"),
        expected
            .map(|value| DataValue::String(value.into()))
            .as_ref()
    );
    assert_eq!(
        error.data.get("found_type"),
        found.map(|value| DataValue::String(value.into())).as_ref()
    );
    let actual: Vec<_> = error
        .related_spans
        .iter()
        .map(|item| {
            (
                item.role,
                &source[item.span.start as usize..item.span.end as usize],
            )
        })
        .collect();
    assert_eq!(actual, related);
}

#[test]
fn independent_operands_keep_their_causal_sites() {
    // F0-0002 §§4.1.2.3–5, 4.6, 4.7 and 13.9: each valid sibling
    // retains its own obligation when another operand has failed.
    let cases: &[(&str, &[Cause<'_>])] = &[
        (
            "fn Main(){let a=(if true then {value:[];} else 1)-true;}",
            &[
                ("NX-E0103", None, "[]"),
                ("NX-E0102", Some("type_mismatch"), "true"),
            ],
        ),
        (
            "fn Main(){let ar=1;var t1=ar[ar];t1[1][1]=(if true then {value:[];} else 1)-true-1;}",
            &[
                ("NX-E0102", Some("not_indexable"), "ar"),
                ("NX-E0103", None, "[]"),
                ("NX-E0102", Some("type_mismatch"), "true"),
            ],
        ),
        (
            "fn Main(){var v0=[];var v1=(if true then {return;} else {return;});let q2=v0<v1;v1=[true];}",
            &[("NX-E0102", Some("type_mismatch"), "<")],
        ),
        (
            "fn Main(){var q=(if true then {return;} else {return;});let a:I64=Print(q);}",
            &[("NX-E0102", Some("type_mismatch"), "Print(q)")],
        ),
        (
            "fn Main(){let c=true;var m2=(if true then {return;} else {return;});var m3=(Len(m2)-c)[1];}",
            &[("NX-E0102", Some("type_mismatch"), "c")],
        ),
        (
            "fn Main(){let c=true;let x=1;var t1=(if true then {return;} else {return;});let u=[Len(1),t1[(x<false)]];}",
            &[
                ("NX-E0102", Some("not_indexable"), "1"),
                ("NX-E0102", Some("type_mismatch"), "false"),
            ],
        ),
        (
            "fn Main(){let l2=(1==([]==[]))<1;}",
            &[
                ("NX-E0103", None, "[]"),
                ("NX-E0102", Some("type_mismatch"), "[]==[]"),
                ("NX-E0103", None, "[]"),
            ],
        ),
        (
            "fn Main(){let ar=true;let br=ar;let l2=(br<([]<[]))[1]<ar;}",
            &[
                ("NX-E0102", Some("type_mismatch"), "<"),
                ("NX-E0103", None, "[]"),
                ("NX-E0103", None, "[]"),
                ("NX-E0102", Some("type_mismatch"), "<"),
            ],
        ),
        (
            "fn Main(){var m5=(if [] then {value:1;} else [])-1;}",
            &[("NX-E0103", None, "[]"), ("NX-E0103", None, "[]")],
        ),
        (
            "fn Main(){let ar=1;var m0=([]==[])-ar;m0=1;m0=true;}",
            &[
                ("NX-E0103", None, "[]"),
                ("NX-E0102", Some("type_mismatch"), "[]==[]"),
                ("NX-E0103", None, "[]"),
                ("NX-E0102", Some("type_mismatch"), "true"),
            ],
        ),
        (
            "fn Main(){let c=true;let ar=1;let b0=1;var b3=[];let b5=(if (if c then b3 else {value:[[],[]];})[(if b0 then b0 else b0)][Len([1])] then 1 else ar);}",
            &[
                ("NX-E0103", None, "[]"),
                ("NX-E0102", Some("type_mismatch"), "b0"),
            ],
        ),
        (
            "fn Main(){let ar=1;let br=1;var m0=(if true then {return;} else {return;});m0=Len([m0,(if m0 then {value:br;} else br),Len(ar)]);}",
            &[
                (
                    "NX-E0102",
                    Some("type_mismatch"),
                    "if m0 then {value:br;} else br",
                ),
                ("NX-E0102", Some("not_indexable"), "ar"),
            ],
        ),
        (
            "fn Main(){let ar=true;var t0=1;var t1=(if true then {return;} else {return;});AssertEq(t0,(if 1 then {value:Len(t1);} else ar));}",
            &[
                ("NX-E0102", Some("type_mismatch"), "1"),
                ("NX-E1303", None, "ar"),
            ],
        ),
    ];
    for (source, expected) in cases {
        let actual = causes(source);
        let expected: Vec<_> = expected
            .iter()
            .map(|(code, subcause, primary)| {
                (
                    code.to_string(),
                    subcause.map(str::to_string),
                    primary.to_string(),
                )
            })
            .collect();
        assert_eq!(actual, expected, "{source}");
        assert!(
            matches!(
                check_scalar("causal_obligations.n", source),
                Err(CompileError::Rejected(_))
            ),
            "{source}"
        );
    }
}

#[test]
fn nested_union_numeric_repair_is_valid() {
    let source = "fn Main(){let a=(if true then {value:1;} else 1)-1;}";
    assert!(check_scalar("causal_obligations.n", source).is_ok());
}

#[test]
fn failed_selector_condition_retains_independent_body_constructor() {
    // F0-0002 §§4.1.2.5 and 13.9: a failed condition does not erase a
    // separate constructor obligation in the body.
    let source = "fn Main(){let ar=[1];if ar+(if ar then {value:1;} else 1)[1] {let w1=[];}}";
    assert_eq!(
        causes(source),
        vec![
            ("NX-E0102".into(), Some("type_mismatch".into()), "ar".into()),
            (
                "NX-E0102".into(),
                Some("not_indexable".into()),
                "if ar then {value:1;} else 1".into(),
            ),
            ("NX-E0102".into(), Some("type_mismatch".into()), "ar".into()),
            ("NX-E0103".into(), None, "[]".into()),
        ]
    );
    let body_twin = "fn Main(){let ar=[1];if ar+(if ar then {value:1;} else 1)[1] {let w1=[1];}}";
    assert_eq!(causes(body_twin).len(), 3);
    let repaired = "fn Main(){let ar=[1];if true {let w1=[1];}}";
    assert!(check_scalar("causal_obligations.n", repaired).is_ok());
}

#[test]
fn resolved_array_left_consumes_only_its_numeric_domain() {
    let deferred = "fn Main(){var q=[];let a=q-true;q=[1];}";
    let concrete = "fn Main(){let q=[1];let a=q-true;}";
    let expected = vec![("NX-E0102".into(), Some("type_mismatch".into()), "q".into())];
    assert_eq!(causes(deferred), expected);
    assert_eq!(causes(concrete), expected);
}

#[test]
fn two_failed_numeric_operands_do_not_supply_an_assignment_type() {
    let source = "fn Main(){let x=1;var ar=[1];var v:Array<Array<I64>>=[[0]];v=(x==177)+(ar-61);}";
    assert_eq!(
        causes(source),
        vec![
            (
                "NX-E0102".into(),
                Some("type_mismatch".into()),
                "x==177".into()
            ),
            ("NX-E0102".into(), Some("type_mismatch".into()), "ar".into()),
        ]
    );
    let repair = "fn Main(){let x=1;var ar=[1];var v:Array<Array<I64>>=[[0]];v=[[x+177]];}";
    assert!(check_scalar("causal_obligations.n", repair).is_ok());
}

#[test]
fn adr_gap_nested_union_and_array_element_keep_current_causal_sites() {
    let nested =
        "fn Main(){let c=true;let br=1;let m0=(if c then [] else (if c then [1] else br));}";
    let array = "fn Main(){let br=1;let a=[[],[1],br];}";
    assert_eq!(
        causes(nested),
        vec![
            ("NX-E0103".into(), None, "[]".into()),
            ("NX-E1303".into(), None, "br".into()),
        ]
    );
    assert_eq!(
        causes(array),
        vec![("NX-E0102".into(), Some("type_mismatch".into()), "br".into())]
    );
}

#[test]
fn adr_gap_entry_gate_is_reserved_for_execution() {
    let source = "fn Other(){}";
    assert!(check_scalar("causal_obligations.n", source).is_ok());
    let CompileError::Rejected(errors) =
        compile_scalar("causal_obligations.n", source).unwrap_err()
    else {
        panic!("expected entry rejection");
    };
    assert_eq!(errors.len(), 1);
    let error = &errors[0];
    error.validate("causal_obligations.n", source).unwrap();
    assert_eq!(error.code, "NX-E0112");
    assert_eq!(error.subcause.as_deref(), Some("missing_main"));
}

#[test]
fn guarded_comparison_keeps_expected_origin_after_domain_resolution() {
    // F0-0002 §§4.1.2.4 and 13.15: the expected relation belongs to the
    // nominal left result even when its domain is resolved later.
    let source =
        "fn Main(){var q=(if true then {return;} else {return;});let a=(Len(q)==true);q=[1];}";
    let errors = check_proposed("causal_obligations.n", source, false).unwrap_err();
    assert_eq!(errors.len(), 1);
    let error = &errors[0];
    error.validate("causal_obligations.n", source).unwrap();
    assert_eq!(error.code, "NX-E0102");
    assert_eq!(error.subcause.as_deref(), Some("type_mismatch"));
    assert_eq!(
        error
            .primary_span
            .as_ref()
            .map(|s| &source[s.start as usize..s.end as usize]),
        Some("true")
    );
    assert_eq!(
        error.data.get("expected_type"),
        Some(&DataValue::String("I64".into()))
    );
    assert_eq!(
        error.data.get("found_type"),
        Some(&DataValue::String("Bool".into()))
    );
    let related: Vec<_> = error
        .related_spans
        .iter()
        .map(|r| (r.role, &source[r.span.start as usize..r.span.end as usize]))
        .collect();
    assert_eq!(
        related,
        vec![
            (RelatedRole::ExpectedType, "Len(q)"),
            (RelatedRole::FoundType, "true")
        ]
    );
    let repair =
        "fn Main(){var q=(if true then {return;} else {return;});let a=(Len(q)==1);q=[1];}";
    assert!(check_scalar("causal_obligations.n", repair).is_ok());
}

#[test]
fn first_equality_origin_survives_later_alias_assignments() {
    // F0-0002 §§4.1.2 N(g), 4.1.2.2 and 13.15: the first equality,
    // rather than a later assignment to the same alias, owns the origin.
    let source =
        "fn Main(){var v0=[];var v1=[];let q1=(v1==v0);let q2=(v0+1);v0=3;v1=[true];v0=[true];}";
    let errors = check_proposed("causal_obligations.n", source, false).unwrap_err();
    assert_eq!(errors.len(), 2);
    let error = errors
        .iter()
        .find(|error| {
            let span = error.primary_span.as_ref().unwrap();
            &source[span.start as usize..span.end as usize] == "3"
        })
        .unwrap();
    error.validate("causal_obligations.n", source).unwrap();
    assert_eq!(error.code, "NX-E0102");
    assert_eq!(error.subcause.as_deref(), Some("type_mismatch"));
    assert_eq!(
        error.data.get("expected_type"),
        Some(&DataValue::String("Array<Bool>".into()))
    );
    assert_eq!(
        error.data.get("found_type"),
        Some(&DataValue::String("I64".into()))
    );
    let related: Vec<_> = error
        .related_spans
        .iter()
        .map(|r| (r.role, &source[r.span.start as usize..r.span.end as usize]))
        .collect();
    assert_eq!(
        related,
        vec![
            (RelatedRole::ExpectedType, "v1"),
            (RelatedRole::FoundType, "3")
        ]
    );
    let repaired = "fn Main(){var v0=[];var v1=[];let q1=(v1==v0);v0=[true];v1=[true];v0=[true];}";
    assert!(check_scalar("causal_obligations.n", repaired).is_ok());
}

#[test]
fn indexed_assignment_origin_survives_earlier_pending_domain() {
    // F0-0002 §§4.1.2 N(g), 4.1.2.3 and 13.15: the array element
    // assignment establishes the later mismatch's actual expected type.
    let source = "fn Main(){var v0=[];var v1=[];let q1=v0<v1;v1=true;v1[0]=[1];v0=[true];}";
    let errors = check_proposed("causal_obligations.n", source, false).unwrap_err();
    assert_eq!(errors.len(), 3);
    let final_true = source.rfind("true").unwrap();
    let error = errors
        .iter()
        .find(|error| error.primary_span.as_ref().unwrap().start as usize == final_true)
        .unwrap();
    error.validate("causal_obligations.n", source).unwrap();
    assert_eq!(error.code, "NX-E0102");
    assert_eq!(error.subcause.as_deref(), Some("type_mismatch"));
    assert_eq!(
        error.data.get("expected_type"),
        Some(&DataValue::String("Array<I64>".into()))
    );
    assert_eq!(
        error.data.get("found_type"),
        Some(&DataValue::String("Bool".into()))
    );
    let related: Vec<_> = error
        .related_spans
        .iter()
        .map(|r| (r.role, &source[r.span.start as usize..r.span.end as usize]))
        .collect();
    assert_eq!(
        related,
        vec![
            (RelatedRole::ExpectedType, "[1]"),
            (RelatedRole::FoundType, "true")
        ]
    );
    let repaired = "fn Main(){var v0=[];var v1=[];let q1=(v0==v1);v1=[[1]];v0=[[1]];}";
    assert!(check_scalar("causal_obligations.n", repaired).is_ok());
}

#[test]
fn mixed_nested_join_keeps_constructor_and_shape_sites() {
    // F0-0002 §§4.1.2.2, 4.1.2.4 and 4.7: a failed join cannot
    // suppress the unresolved constructor in its other branch.
    let source = "fn Main(){let b=false;let o=(if b then [[]] else (if b then [[false],[true]] else [2,3]));}";
    assert_eq!(
        causes(source),
        vec![
            ("NX-E0103".into(), None, "[]".into()),
            ("NX-E1303".into(), None, "[2,3]".into()),
        ]
    );
    let repaired = "fn Main(){let b=false;let o=(if b then [[false]] else (if b then [[false],[true]] else [[false],[true]]));}";
    assert!(check_scalar("causal_obligations.n", repaired).is_ok());
}

#[test]
fn failed_outer_condition_keeps_nested_join_constructor() {
    // F0-0002 §§4.1.2.2, 4.1.2.5 and 13.9: the condition error does
    // not consume the first branch's unresolved constructor.
    let source =
        "fn Main(){let x=1;let br=[true];var m0=(if 1 then [] else (if x then [1] else br))[1];}";
    assert_eq!(
        causes(source),
        vec![
            ("NX-E0102".into(), Some("type_mismatch".into()), "1".into()),
            ("NX-E0103".into(), None, "[]".into()),
            ("NX-E0102".into(), Some("type_mismatch".into()), "x".into()),
            ("NX-E1303".into(), None, "br".into()),
        ]
    );
    let repaired = "fn Main(){let x=true;let br=[1];let m0=(if true then [1] else (if x then [1] else br))[0];}";
    assert!(check_scalar("causal_obligations.n", repaired).is_ok());
}

#[test]
fn alias_join_has_executable_type() {
    // F0-0002 §4.1.2 N(f): a valid inferred union must lower with one type.
    let source = "fn Main(){let ar=[1];let b0=ar;let b2=ar;var b4=(if b0==[] then ar else b2);}";
    assert!(causes(source).is_empty());
    assert!(check_scalar("causal_obligations.n", source).is_ok());
}

#[test]
fn reduced_independent_sites_remain_causal() {
    // F0-0002 §§4.1.2 N(b)–N(d), N(h), 4.1.2.2–5 and 13.17.
    // Each entry is a reduced source and a site which can fail without
    // depending on another diagnosed expression in that same source.
    let cases = [
        (
            "fn Main(){let c=true;var m2=if c then {return; } else {return; }; var m3=(Len(m2)-c)[1];}",
            "NX-E0102",
            Some("type_mismatch"),
            "c",
        ),
        ("fn Main(){var t0=[];t0[[]+1]=1;}", "NX-E0103", None, "[]"),
        (
            "fn Main(){let c=true;let ar=1;let l1=if [] then ((if ar then ar else c)==1)-c else 1;}",
            "NX-E0102",
            Some("type_mismatch"),
            "c",
        ),
        (
            "fn Main(){let ar=1;let br=[];var m0=br;m0[(if 0 then ar else true)<0]=ar;}",
            "NX-E0103",
            None,
            "[]",
        ),
        (
            "fn Main(){let c=true;var m1=1[c<(if c then {value:[];} else c)];}",
            "NX-E0102",
            Some("type_mismatch"),
            "<",
        ),
        (
            "fn Main(){let c=true;let br=c;let l0=(if 1 then 0+br else [])[c<true[1]];}",
            "NX-E0102",
            Some("type_mismatch"),
            "<",
        ),
        (
            "fn Main(){let c=true;let a2:Array<Bool>=if (if c then [] else true)[1[c]][c] then 111 else 1;}",
            "NX-E0102",
            Some("type_mismatch"),
            "c",
        ),
        (
            "fn Main(){let ar=[1];if ar+(if ar then {value:1;} else 1)[1] {let w1=[];}}",
            "NX-E0103",
            None,
            "[]",
        ),
        (
            "fn Main(){let c=true;let x=1;var m0=1;var m2=if x-[] then {value:1;} else m0;let l3=m2==c;}",
            "NX-E0102",
            Some("type_mismatch"),
            "c",
        ),
        (
            "fn Main(){let x=1;let ar=true;var m0=[];var m1=if false then {return;} else {return;};m1=m1[true]-(m0==[])==ar+x;}",
            "NX-E0102",
            Some("type_mismatch"),
            "ar",
        ),
        (
            "fn Main(){let c=true;let x=1;var m0=if false then {return;} else {return;};m0=if ((c<x)==m0)+1 then 11==(m0<true)+1 else 1;}",
            "NX-E0102",
            Some("type_mismatch"),
            "<",
        ),
    ];
    for (source, code, subcause, primary) in cases {
        let actual = causes(source);
        assert!(
            actual
                .iter()
                .any(|(c, s, p)| c == code && s.as_deref() == subcause && p == primary),
            "missing {code}/{subcause:?} at {primary} in {source}: {actual:?}"
        );
        assert!(matches!(
            check_scalar("causal_obligations.n", source),
            Err(CompileError::Rejected(_))
        ));
    }
}

#[test]
fn failed_statement_conditions_keep_hypothetical_body_obligations() {
    // F0-0002 §4.1.2.5: a failed condition does not erase independent
    // obligations in either hypothetical body.
    let negative = "fn Main(){if 1 {let a=[];let b=true+1;}while 1 {let a=[];let b=true+1;}}";
    assert_eq!(
        causes(negative),
        vec![
            ("NX-E0102".into(), Some("type_mismatch".into()), "1".into()),
            ("NX-E0103".into(), None, "[]".into()),
            (
                "NX-E0102".into(),
                Some("type_mismatch".into()),
                "true".into()
            ),
            ("NX-E0102".into(), Some("type_mismatch".into()), "1".into()),
            ("NX-E0103".into(), None, "[]".into()),
            (
                "NX-E0102".into(),
                Some("type_mismatch".into()),
                "true".into()
            ),
        ]
    );
    let positive = "fn Main(){if true {let a=[1];let b=1+1;}while false {let a=[1];let b=1+1;}}";
    assert!(causes(positive).is_empty());
    assert!(check_scalar("causal_obligations.n", positive).is_ok());
}

#[test]
fn two_unresolved_constructors_keep_two_obligations() {
    // F0-0002 §4.1.2.2 and N(h): each unresolved constructor owns a C.
    let negative = "fn Main(){var t0=[];t0[[]+1]=1;}";
    assert_eq!(
        causes(negative),
        vec![
            ("NX-E0103".into(), None, "[]".into()),
            ("NX-E0103".into(), None, "[]".into()),
        ]
    );
    let positive = "fn Main(){var t0=[1];t0[0+1]=1;}";
    assert!(causes(positive).is_empty());
    assert!(check_scalar("causal_obligations.n", positive).is_ok());
}

#[test]
fn numeric_operation_reports_only_first_invalid_domain() {
    // F0-0002 §4.1.2 N(d), §4.6: an Array shape already refutes I64 for
    // the left operand. Its unresolved element has its own constructor C;
    // the same operation cannot report the later Bool domain as another C.
    let negative = "fn Main(){var t=[];let x=t+false;}";
    assert_eq!(
        causes(negative),
        vec![("NX-E0103".into(), None, "[]".into())]
    );
    let concrete_twin = "fn Main(){var t=[1];let x=t+false;}";
    assert_eq!(
        causes(concrete_twin),
        vec![("NX-E0102".into(), Some("type_mismatch".into()), "t".into())]
    );
    let positive = "fn Main(){let t=1;let x=t+2;}";
    assert!(causes(positive).is_empty());
    assert!(check_scalar("causal_obligations.n", positive).is_ok());
}

#[test]
fn later_scalar_assignment_keeps_first_concrete_array_origin() {
    // F0-0002 §4.1.2 N(g), §§4.1.2.2 and 13.17: the first compatible
    // assignment to a deferred binding establishes the real source origin.
    let source = "fn Main(){var v=(if true then {return;} else {return;});v=[true];v=[1];}";
    let errors = check_proposed("causal_obligations.n", source, false).unwrap_err();
    assert_eq!(errors.len(), 1);
    let error = &errors[0];
    error.validate("causal_obligations.n", source).unwrap();
    assert_eq!(error.code, "NX-E0102");
    assert_eq!(error.subcause.as_deref(), Some("type_mismatch"));
    assert_eq!(
        error
            .primary_span
            .as_ref()
            .map(|s| &source[s.start as usize..s.end as usize]),
        Some("1")
    );
    assert_eq!(
        error.data.get("expected_type"),
        Some(&DataValue::String("Bool".into()))
    );
    assert_eq!(
        error.data.get("found_type"),
        Some(&DataValue::String("I64".into()))
    );
    let related: Vec<_> = error
        .related_spans
        .iter()
        .map(|r| (r.role, &source[r.span.start as usize..r.span.end as usize]))
        .collect();
    assert_eq!(
        related,
        vec![
            (RelatedRole::ExpectedType, "[true]"),
            (RelatedRole::FoundType, "1")
        ]
    );
    let positive = "fn Main(){var v=(if true then {return;} else {return;});v=[true];v=[false];}";
    assert!(check_scalar("causal_obligations.n", positive).is_ok());
}

#[test]
fn nested_array_assignment_keeps_first_concrete_origin() {
    // F0-0002 §4.1.2 N(g), §§4.1.2.2 and 13.17: origin traversal
    // descends through Array nodes without losing the first assignment.
    let source = "fn Main(){var v=[];v=[[1]];v=[[true]];}";
    let errors = check_proposed("causal_obligations.n", source, false).unwrap_err();
    assert_eq!(errors.len(), 1);
    let error = &errors[0];
    error.validate("causal_obligations.n", source).unwrap();
    assert_eq!(error.code, "NX-E0102");
    assert_eq!(error.subcause.as_deref(), Some("type_mismatch"));
    assert_eq!(
        error
            .primary_span
            .as_ref()
            .map(|s| &source[s.start as usize..s.end as usize]),
        Some("true")
    );
    assert_eq!(
        error.data.get("expected_type"),
        Some(&DataValue::String("I64".into()))
    );
    assert_eq!(
        error.data.get("found_type"),
        Some(&DataValue::String("Bool".into()))
    );
    let related: Vec<_> = error
        .related_spans
        .iter()
        .map(|r| (r.role, &source[r.span.start as usize..r.span.end as usize]))
        .collect();
    assert_eq!(
        related,
        vec![
            (RelatedRole::ExpectedType, "[[1]]"),
            (RelatedRole::FoundType, "true")
        ]
    );
    let positive = "fn Main(){var v=[];v=[[1]];v=[[2]];}";
    assert!(check_scalar("causal_obligations.n", positive).is_ok());
}

#[test]
fn indexed_element_origin_survives_compatible_whole_assignment() {
    // F0-0002 §4.1.2.2 and RD89: the first indexed write fixes the
    // element; a later compatible root assignment cannot replace its origin.
    let source = "fn Main(){var r=[];r[0]=true;r=[true];r=[3];}";
    let errors = check_proposed("causal_obligations.n", source, false).unwrap_err();
    assert_eq!(errors.len(), 1);
    let error = &errors[0];
    error.validate("causal_obligations.n", source).unwrap();
    assert_eq!(error.code, "NX-E0102");
    assert_eq!(error.subcause.as_deref(), Some("type_mismatch"));
    assert_eq!(
        error
            .primary_span
            .as_ref()
            .map(|s| &source[s.start as usize..s.end as usize]),
        Some("3")
    );
    assert_eq!(
        error.data.get("expected_type"),
        Some(&DataValue::String("Bool".into()))
    );
    assert_eq!(
        error.data.get("found_type"),
        Some(&DataValue::String("I64".into()))
    );
    assert_eq!(error.related_spans.len(), 2);
    let first_true = source.find("true").unwrap();
    assert!(
        error.related_spans.iter().any(|r| {
            r.role == RelatedRole::ExpectedType && r.span.start as usize == first_true
        })
    );
    let direct_twin = "fn Main(){var r=[];r[0]=true;r=[3];}";
    let direct = check_proposed("causal_obligations.n", direct_twin, false).unwrap_err();
    assert_eq!(direct.len(), 1);
    assert_eq!(
        direct[0]
            .related_spans
            .iter()
            .find(|r| r.role == RelatedRole::ExpectedType)
            .map(|r| &direct_twin[r.span.start as usize..r.span.end as usize]),
        Some("true")
    );
    let positive = "fn Main(){var r=[false];r[0]=true;r=[true];r=[false];}";
    assert!(check_scalar("causal_obligations.n", positive).is_ok());
}

#[test]
fn intrinsic_condition_origin_stays_unrelated_after_compatible_assignment() {
    // F0-0002 §§4.1.2.3, 4.1.2.6, 13.15 and RD28: the Bool
    // context of a condition fixes the element without a source origin.
    for source in [
        "fn Main(){var r=[];if r[0] {}r=[3];}",
        "fn Main(){var r=[];if r[0] {}r=[true];if r[0] {}r=[3];}",
    ] {
        let errors = check_proposed("causal_obligations.n", source, false).unwrap_err();
        assert_eq!(errors.len(), 1, "{source}");
        let error = &errors[0];
        error.validate("causal_obligations.n", source).unwrap();
        assert_eq!(error.code, "NX-E0102", "{source}");
        assert_eq!(error.subcause.as_deref(), Some("type_mismatch"), "{source}");
        assert_eq!(
            error
                .primary_span
                .as_ref()
                .map(|span| &source[span.start as usize..span.end as usize]),
            Some("3"),
            "{source}"
        );
        assert_eq!(
            error.data.get("expected_type"),
            Some(&DataValue::String("Bool".into()))
        );
        assert_eq!(
            error.data.get("found_type"),
            Some(&DataValue::String("I64".into()))
        );
        assert_eq!(error.related_spans.len(), 1, "{source}");
        assert_eq!(error.related_spans[0].role, RelatedRole::FoundType);
        assert!(matches!(
            check_scalar("causal_obligations.n", source),
            Err(CompileError::Rejected(_))
        ));
        assert!(matches!(
            compile_scalar("causal_obligations.n", source),
            Err(CompileError::Rejected(_))
        ));
    }
    let positive = "fn Main(){var r=[];if r[0] {}r=[true];if r[0] {}r=[false];}";
    assert!(check_scalar("causal_obligations.n", positive).is_ok());
    assert!(compile_scalar("causal_obligations.n", positive).is_ok());
}

#[test]
fn failed_right_constructor_preserves_independent_left_comparison_domain() {
    // F0-0002 §4.1.2.4: an unresolved constructor on the right cannot
    // consume the determined left operand's own domain obligation.
    let source = "fn Main(){let br=[true];let a0:Array<I64>=br<[[]];}";
    let errors = check_proposed("causal_obligations.n", source, false).unwrap_err();
    assert_eq!(errors.len(), 2);
    for error in &errors {
        error.validate("causal_obligations.n", source).unwrap();
    }
    let domain = errors.iter().find(|e| e.code == "NX-E0102").unwrap();
    assert_eq!(domain.subcause.as_deref(), Some("type_mismatch"));
    let span = domain.primary_span.as_ref().unwrap();
    assert_eq!(&source[span.start as usize..span.end as usize], "<");
    assert_eq!(domain.data.len(), 2);
    assert_eq!(
        domain.data.get("expected_type"),
        Some(&DataValue::String("I64".into()))
    );
    assert_eq!(
        domain.data.get("found_type"),
        Some(&DataValue::String("Array<Bool>".into()))
    );
    assert_eq!(domain.related_spans.len(), 1);
    assert_eq!(domain.related_spans[0].role, RelatedRole::FoundType);
    let found = &domain.related_spans[0].span;
    assert_eq!(&source[found.start as usize..found.end as usize], "br");
    let unresolved = errors.iter().find(|e| e.code == "NX-E0103").unwrap();
    let span = unresolved.primary_span.as_ref().unwrap();
    assert_eq!(&source[span.start as usize..span.end as usize], "[]");
    assert!(matches!(
        check_scalar("causal_obligations.n", source),
        Err(CompileError::Rejected(_))
    ));
    assert!(matches!(
        compile_scalar("causal_obligations.n", source),
        Err(CompileError::Rejected(_))
    ));
    let positive = "fn Main(){let br=1;let a0:Bool=br<2;}";
    assert!(check_scalar("causal_obligations.n", positive).is_ok());
    assert!(compile_scalar("causal_obligations.n", positive).is_ok());

    let resolved = "fn Main(){let br=[true];var r=[[]];let a0=br<r;r=[[1]];}";
    let errors = check_proposed("causal_obligations.n", resolved, false).unwrap_err();
    assert_eq!(errors.len(), 1, "{errors:?}");
    let error = &errors[0];
    error.validate("causal_obligations.n", resolved).unwrap();
    assert_eq!(error.code, "NX-E0102");
    let span = error.primary_span.as_ref().unwrap();
    assert_eq!(&resolved[span.start as usize..span.end as usize], "r");
}

#[test]
fn late_binary_domain_failure_does_not_erase_later_argument_mismatch() {
    // F0-0002 §§4.1.2.2, 4.1.2.4–5: a failed comparison and a rejected
    // assignment do not invalidate the established type at another use.
    let source = "fn Main(){var v0=(if false then {return;} else {return;});var v1=[];let q1=v1<v0;v0=[1];v0=3;AssertEq(v0,1);}";
    let errors = check_proposed("causal_obligations.n", source, false).unwrap_err();
    assert_eq!(errors.len(), 3);
    let last = &errors[2];
    last.validate("causal_obligations.n", source).unwrap();
    assert_eq!(last.code, "NX-E0102");
    assert_eq!(last.subcause.as_deref(), Some("type_mismatch"));
    let span = last.primary_span.as_ref().unwrap();
    assert_eq!(&source[span.start as usize..span.end as usize], "1");
    assert_eq!(last.data.len(), 2);
    assert_eq!(
        last.data.get("expected_type"),
        Some(&DataValue::String("Array<I64>".into()))
    );
    assert_eq!(
        last.data.get("found_type"),
        Some(&DataValue::String("I64".into()))
    );
    assert_eq!(last.related_spans.len(), 2);
    let expected = last
        .related_spans
        .iter()
        .find(|r| r.role == RelatedRole::ExpectedType)
        .unwrap();
    assert_eq!(
        &source[expected.span.start as usize..expected.span.end as usize],
        "v0"
    );
    let found = last
        .related_spans
        .iter()
        .find(|r| r.role == RelatedRole::FoundType)
        .unwrap();
    assert_eq!(
        &source[found.span.start as usize..found.span.end as usize],
        "1"
    );
    let twin = source.replace("AssertEq(v0,1)", "AssertEq(v0,[2])");
    let twin_errors = check_proposed("causal_obligations.n", &twin, false).unwrap_err();
    assert_eq!(twin_errors.len(), 2);
    let positive = "fn Main(){var v0=[1];AssertEq(v0,[2]);}";
    assert!(check_scalar("causal_obligations.n", positive).is_ok());
    assert!(compile_scalar("causal_obligations.n", positive).is_ok());

    // The intervening independent reads must not let a failed binary domain
    // or a rejected assignment hide the later argument constraint.
    let with_reads = "fn Main(){let c=true;let x=1;let ar=[1];let br=[true];var v0=(if false then {return;} else {return;});var v1=[];let q1=(v1<v0);Print(v0);Print(v1);v0=[1];v0=3;AssertEq(v0,x);}";
    let observed = causes(with_reads);
    assert_eq!(
        observed,
        vec![
            ("NX-E0102".into(), Some("type_mismatch".into()), "<".into()),
            ("NX-E0102".into(), Some("type_mismatch".into()), "3".into()),
            ("NX-E0102".into(), Some("type_mismatch".into()), "x".into()),
        ]
    );
    let errors = check_proposed("causal_obligations.n", with_reads, false).unwrap_err();
    let argument = &errors[2];
    assert_eq!(argument.data.len(), 2);
    assert_eq!(
        argument.data.get("expected_type"),
        Some(&DataValue::String("Array<I64>".into()))
    );
    assert_eq!(
        argument.data.get("found_type"),
        Some(&DataValue::String("I64".into()))
    );
    assert_eq!(argument.related_spans.len(), 2);
    let expected = argument
        .related_spans
        .iter()
        .find(|r| r.role == RelatedRole::ExpectedType)
        .unwrap();
    let found = argument
        .related_spans
        .iter()
        .find(|r| r.role == RelatedRole::FoundType)
        .unwrap();
    assert_eq!(
        &with_reads[expected.span.start as usize..expected.span.end as usize],
        "v0"
    );
    assert_eq!(
        &with_reads[found.span.start as usize..found.span.end as usize],
        "x"
    );
    let repaired_argument = with_reads.replace("AssertEq(v0,x)", "AssertEq(v0,[2])");
    assert_eq!(causes(&repaired_argument).len(), 2);
}

#[test]
fn guarded_comparison_keeps_later_assert_argument_domain() {
    // F0-0002 §4.1.2.4 N(d): the later AssertEq argument has its own
    // concrete I64/Bool conflict, independent of the pending Len domain.
    let source = "fn Main(){var q=(if true then {return;} else {return;});let a=Len(q)<1;AssertEq(1,1<1);q=[1];}";
    let errors = check_proposed("causal_obligations.n", source, false).unwrap_err();
    assert_eq!(errors.len(), 1);
    let error = &errors[0];
    error.validate("causal_obligations.n", source).unwrap();
    assert_eq!(error.code, "NX-E0102");
    assert_eq!(error.subcause.as_deref(), Some("type_mismatch"));
    let span = error.primary_span.as_ref().unwrap();
    assert_eq!(&source[span.start as usize..span.end as usize], "1<1");
    assert_eq!(error.data.len(), 2);
    assert_eq!(
        error.data.get("expected_type"),
        Some(&DataValue::String("I64".into()))
    );
    assert_eq!(
        error.data.get("found_type"),
        Some(&DataValue::String("Bool".into()))
    );
    assert_eq!(error.related_spans.len(), 2);
    let expected = error
        .related_spans
        .iter()
        .find(|r| r.role == RelatedRole::ExpectedType)
        .unwrap();
    let found = error
        .related_spans
        .iter()
        .find(|r| r.role == RelatedRole::FoundType)
        .unwrap();
    assert_eq!(
        &source[expected.span.start as usize..expected.span.end as usize],
        "1"
    );
    assert_eq!(
        &source[found.span.start as usize..found.span.end as usize],
        "1<1"
    );
    let twin = source.replace("AssertEq(1,1<1)", "AssertEq(1,1)");
    assert!(check_scalar("causal_obligations.n", &twin).is_ok());
    assert!(compile_scalar("causal_obligations.n", &twin).is_ok());
    let positive = "fn Main(){let q=[1];let a=Len(q)<1;AssertEq(1,1);}";
    assert!(check_scalar("causal_obligations.n", positive).is_ok());
    assert!(compile_scalar("causal_obligations.n", positive).is_ok());
}

#[test]
fn grouped_failed_join_consumes_only_one_outer_numeric_domain_site() {
    // F0-0002 §4.1.2.4 N(d): the valid else branch is checked against
    // the outer subtraction's I64 context. Its failure uses that operation's
    // sole domain diagnostic; the independent internal errors remain.
    let source = "fn Main(){let c=true;let ar=[1];let br=[true];let a=((if [1][[[]]] then (c-ar) else br)-[true]);}";
    assert_eq!(
        causes(source),
        vec![
            ("NX-E0103".into(), None, "[]".into()),
            ("NX-E0102".into(), Some("type_mismatch".into()), "c".into()),
            ("NX-E0102".into(), Some("type_mismatch".into()), "br".into()),
        ]
    );
    let errors = check_proposed("causal_obligations.n", source, false).unwrap_err();
    assert_eq!(errors[2].data.len(), 2);
    assert_eq!(
        errors[2].data.get("expected_type"),
        Some(&DataValue::String("I64".into()))
    );
    assert_eq!(
        errors[2].data.get("found_type"),
        Some(&DataValue::String("Array<Bool>".into()))
    );
    assert_eq!(errors[2].related_spans.len(), 1);
    assert_eq!(errors[2].related_spans[0].role, RelatedRole::FoundType);
    assert_eq!(
        &source[errors[2].related_spans[0].span.start as usize
            ..errors[2].related_spans[0].span.end as usize],
        "br"
    );
    let repaired_rhs = source.replace("-[true]", "-1");
    assert_eq!(causes(&repaired_rhs), causes(source));
    let positive = "fn Main(){let c=1;let br=2;let a=((if true then c else br)-1);}";
    assert!(check_scalar("causal_obligations.n", positive).is_ok());
    assert!(compile_scalar("causal_obligations.n", positive).is_ok());
}

#[test]
fn failed_inner_join_preserves_first_valid_branch_for_outer_join() {
    // F0-0002 §4.1.2.5: the inner join cannot type ordinary consumers,
    // but its first valid normal continuation is an outer join witness.
    let source = "fn Main(){let c=true;let a=(if (if [] then true else true) then (if c then 1 else [[]]) else [[1]]);}";
    assert_eq!(
        causes(source),
        vec![
            ("NX-E0103".into(), None, "[]".into()),
            ("NX-E0103".into(), None, "[]".into()),
            ("NX-E1303".into(), None, "[[1]]".into()),
        ]
    );
    let errors = check_proposed("causal_obligations.n", source, false).unwrap_err();
    let outer = &errors[2];
    assert_eq!(outer.data.len(), 2);
    assert_eq!(
        outer.data.get("expected_type"),
        Some(&DataValue::String("I64".into()))
    );
    assert_eq!(
        outer.data.get("found_type"),
        Some(&DataValue::String("Array<Array<I64>>".into()))
    );
    assert_eq!(outer.related_spans.len(), 2);
    let expected = outer
        .related_spans
        .iter()
        .find(|r| r.role == RelatedRole::ExpectedType)
        .unwrap();
    let found = outer
        .related_spans
        .iter()
        .find(|r| r.role == RelatedRole::FoundType)
        .unwrap();
    assert_eq!(
        &source[expected.span.start as usize..expected.span.end as usize],
        "if c then 1 else [[]]"
    );
    assert_eq!(
        &source[found.span.start as usize..found.span.end as usize],
        "[[1]]"
    );
    let repaired = source.replace("else [[1]]", "else 2");
    assert_eq!(causes(&repaired).len(), 2);
    let positive = "fn Main(){let c=true;let a=(if true then (if c then 1 else 2) else 3);}";
    assert!(check_scalar("causal_obligations.n", positive).is_ok());
    assert!(compile_scalar("causal_obligations.n", positive).is_ok());
}

#[test]
fn nested_comparison_keeps_right_independent_domain_site() {
    // F0-0002 §§4.1.2.4-5: the failed left comparison cannot erase
    // the right comparison's own Bool-versus-I64 domain failure.
    let source = "fn Main(){let c=true;let ar=[1];let a=(ar<c)==(c<[]);}";
    assert_eq!(
        causes(source),
        vec![
            ("NX-E0102".into(), Some("type_mismatch".into()), "c".into()),
            ("NX-E0102".into(), Some("type_mismatch".into()), "<".into()),
            ("NX-E0103".into(), None, "[]".into()),
        ]
    );
    let errors = check_proposed("causal_obligations.n", source, false).unwrap_err();
    let domain = &errors[1];
    assert_eq!(domain.data.len(), 2);
    assert_eq!(
        domain.data.get("expected_type"),
        Some(&DataValue::String("I64".into()))
    );
    assert_eq!(
        domain.data.get("found_type"),
        Some(&DataValue::String("Bool".into()))
    );
    assert_eq!(domain.related_spans.len(), 1);
    assert_eq!(domain.related_spans[0].role, RelatedRole::FoundType);
    assert_eq!(
        &source[domain.related_spans[0].span.start as usize
            ..domain.related_spans[0].span.end as usize],
        "c"
    );
    let repaired_left = source.replace("ar<c", "1<c");
    assert!(causes(&repaired_left).iter().any(|cause| cause.2 == "<"));
    let positive = "fn Main(){let c=1;let ar=2;let a=(ar<c)==(c<3);}";
    assert!(check_scalar("causal_obligations.n", positive).is_ok());
    assert!(compile_scalar("causal_obligations.n", positive).is_ok());
}

#[test]
fn partial_array_witness_resolves_outer_empty_branch() {
    // F0-0002 §§4.1.2.2,4.1.2.5 N(h): the first valid Array branch
    // remains a partial witness after the inner mismatch and resolves the
    // outer empty constructor.
    let source = "fn Main(){let a=(if true then (if false then [1] else 2) else []);}";
    assert_eq!(causes(source), vec![("NX-E1303".into(), None, "2".into())]);
    let errors = check_proposed("causal_obligations.n", source, false).unwrap_err();
    let inner = &errors[0];
    assert_eq!(inner.data.len(), 2);
    assert_eq!(
        inner.data.get("expected_type"),
        Some(&DataValue::String("Array<I64>".into()))
    );
    assert_eq!(
        inner.data.get("found_type"),
        Some(&DataValue::String("I64".into()))
    );
    assert_eq!(inner.related_spans.len(), 2);
    let expected = inner
        .related_spans
        .iter()
        .find(|r| r.role == RelatedRole::ExpectedType)
        .unwrap();
    let found = inner
        .related_spans
        .iter()
        .find(|r| r.role == RelatedRole::FoundType)
        .unwrap();
    assert_eq!(
        &source[expected.span.start as usize..expected.span.end as usize],
        "[1]"
    );
    assert_eq!(
        &source[found.span.start as usize..found.span.end as usize],
        "2"
    );
    let repaired = source.replace("else 2", "else [2]");
    assert!(check_scalar("causal_obligations.n", &repaired).is_ok());
    assert!(compile_scalar("causal_obligations.n", &repaired).is_ok());
    let positive = "fn Main(){let a=(if true then (if false then [1] else [2]) else [3]);}";
    assert!(check_scalar("causal_obligations.n", positive).is_ok());
    assert!(compile_scalar("causal_obligations.n", positive).is_ok());
}

#[test]
fn nested_failed_union_keeps_its_outer_shape_conflict() {
    // F0-0002 §§4.1.2.4-5 N(d): the invalid condition does not erase
    // either union's own first incompatible continuation.
    let source = "fn Main(){let a=(if true then (if false then [9] else (if [[2],[3]] then [[1]] else [[1]])) else [[1]]);}";
    assert_eq!(
        causes(source),
        vec![
            (
                "NX-E0102".into(),
                Some("type_mismatch".into()),
                "[[2],[3]]".into(),
            ),
            ("NX-E1303".into(), None, "[[1]]".into()),
            ("NX-E1303".into(), None, "[[1]]".into()),
        ]
    );
    let errors = check_proposed("causal_obligations.n", source, false).unwrap_err();
    for (index, expected_origin) in [
        "[9]",
        "if false then [9] else (if [[2],[3]] then [[1]] else [[1]])",
    ]
    .into_iter()
    .enumerate()
    {
        let conflict = &errors[index + 1];
        assert_eq!(conflict.data.len(), 2);
        assert_eq!(
            conflict.data.get("expected_type"),
            Some(&DataValue::String("Array<I64>".into()))
        );
        assert_eq!(
            conflict.data.get("found_type"),
            Some(&DataValue::String("Array<Array<I64>>".into()))
        );
        assert_eq!(conflict.related_spans.len(), 2);
        let expected = conflict
            .related_spans
            .iter()
            .find(|r| r.role == RelatedRole::ExpectedType)
            .unwrap();
        assert_eq!(
            &source[expected.span.start as usize..expected.span.end as usize],
            expected_origin
        );
    }
    let repaired_outer = source.replace(")) else [[1]]);}", ")) else [7]);}");
    assert_eq!(causes(&repaired_outer).len(), 2);
    let positive = "fn Main(){let a=(if true then (if false then [9] else (if true then [1] else [2])) else [3]);}";
    assert!(check_scalar("causal_obligations.n", positive).is_ok());
    assert!(compile_scalar("causal_obligations.n", positive).is_ok());
}

#[test]
fn failed_comparison_keeps_independent_numeric_right_operand() {
    // F0-0002 §4.1.2.4 N(d): the right operand of + has its own I64
    // requirement even when the left comparison depends on an unresolved
    // constructor and cannot provide a valid result to +.
    let source =
        "fn Main(){let ar=[1];var v=(if true then {return;} else {return;});let a=((v<[[]])+ar);}";
    let errors = check_proposed("causal_obligations.n", source, false).unwrap_err();
    let rhs = errors
        .iter()
        .find(|error| {
            error
                .primary_span
                .as_ref()
                .is_some_and(|span| &source[span.start as usize..span.end as usize] == "ar")
        })
        .expect("independent numeric RHS");
    rhs.validate("causal_obligations.n", source).unwrap();
    assert_eq!(rhs.code, "NX-E0102");
    assert_eq!(rhs.subcause.as_deref(), Some("type_mismatch"));
    assert_eq!(rhs.data.len(), 2);
    assert_eq!(
        rhs.data.get("expected_type"),
        Some(&DataValue::String("I64".into()))
    );
    assert_eq!(
        rhs.data.get("found_type"),
        Some(&DataValue::String("Array<I64>".into()))
    );
    assert_eq!(rhs.related_spans.len(), 1);
    assert_eq!(rhs.related_spans[0].role, RelatedRole::FoundType);
    assert_eq!(
        rhs.related_spans[0].span,
        *rhs.primary_span.as_ref().unwrap()
    );
    let repaired_rhs = source.replace("+ar", "+1");
    assert!(!causes(&repaired_rhs).iter().any(|cause| cause.2 == "ar"));
    let positive = "fn Main(){let ar=[1];let a=(1+Len(ar));}";
    assert!(check_scalar("causal_obligations.n", positive).is_ok());
    assert!(compile_scalar("causal_obligations.n", positive).is_ok());
}

#[test]
fn failed_inner_join_keeps_partial_witness_for_outer_join() {
    // F0-0002 §4.1.2.5: the failed inner union has no value for ordinary
    // consumers, but its first valid arm remains a witness for an outer union.
    let source = "fn Main(){let x=(if true then (if true then 1 else [false]) else [[1]]);}";
    let errors = check_proposed("causal_obligations.n", source, false).unwrap_err();
    assert_eq!(errors.len(), 2);
    for error in &errors {
        error.validate("causal_obligations.n", source).unwrap();
        assert_eq!(error.code, "NX-E1303");
        assert!(error.subcause.is_none());
        assert_eq!(error.data.len(), 2);
    }
    let outer = &errors[1];
    let span = outer.primary_span.as_ref().unwrap();
    assert_eq!(&source[span.start as usize..span.end as usize], "[[1]]");
    assert_eq!(
        outer.data.get("expected_type"),
        Some(&DataValue::String("I64".into()))
    );
    assert_eq!(
        outer.data.get("found_type"),
        Some(&DataValue::String("Array<Array<I64>>".into()))
    );
    assert_eq!(outer.related_spans.len(), 2);
    let expected = outer
        .related_spans
        .iter()
        .find(|r| r.role == RelatedRole::ExpectedType)
        .unwrap();
    assert_eq!(
        &source[expected.span.start as usize..expected.span.end as usize],
        "if true then 1 else [false]"
    );
    let found = outer
        .related_spans
        .iter()
        .find(|r| r.role == RelatedRole::FoundType)
        .unwrap();
    assert_eq!(
        &source[found.span.start as usize..found.span.end as usize],
        "[[1]]"
    );
    let twin = "fn Main(){let x=(if true then (if true then 1 else 2) else [[1]]);}";
    let twin_errors = check_proposed("causal_obligations.n", twin, false).unwrap_err();
    assert_eq!(twin_errors.len(), 1);
    let positive = "fn Main(){let x=(if true then (if true then 1 else 2) else 3);}";
    assert!(check_scalar("causal_obligations.n", positive).is_ok());
    assert!(compile_scalar("causal_obligations.n", positive).is_ok());
}

#[test]
fn unresolved_array_domain_keeps_known_len_result_equality() {
    // F0-0002 §§4.1.2.2–4: Len's result remains I64 when its operand
    // already has Array shape, even if the element never resolves.
    let source =
        "fn Main(){var t2=[];let x=Len(t2)==[(if false then {return;} else {return;}),[[1]]];}";
    let errors = check_proposed("causal_obligations.n", source, false).unwrap_err();
    assert_eq!(errors.len(), 2);
    let unresolved = &errors[0];
    unresolved.validate("causal_obligations.n", source).unwrap();
    assert_eq!(unresolved.code, "NX-E0103");
    let span = unresolved.primary_span.as_ref().unwrap();
    assert_eq!(&source[span.start as usize..span.end as usize], "[]");
    let mismatch = &errors[1];
    mismatch.validate("causal_obligations.n", source).unwrap();
    assert_eq!(mismatch.code, "NX-E0102");
    assert_eq!(mismatch.subcause.as_deref(), Some("type_mismatch"));
    let span = mismatch.primary_span.as_ref().unwrap();
    assert_eq!(
        &source[span.start as usize..span.end as usize],
        "[(if false then {return;} else {return;}),[[1]]]"
    );
    assert_eq!(mismatch.data.len(), 2);
    assert_eq!(
        mismatch.data.get("expected_type"),
        Some(&DataValue::String("I64".into()))
    );
    assert_eq!(
        mismatch.data.get("found_type"),
        Some(&DataValue::String("Array<Array<Array<I64>>>".into()))
    );
    assert_eq!(mismatch.related_spans.len(), 2);
    let expected = mismatch
        .related_spans
        .iter()
        .find(|r| r.role == RelatedRole::ExpectedType)
        .unwrap();
    assert_eq!(
        &source[expected.span.start as usize..expected.span.end as usize],
        "Len(t2)"
    );
    let found = mismatch
        .related_spans
        .iter()
        .find(|r| r.role == RelatedRole::FoundType)
        .unwrap();
    assert_eq!(
        &source[found.span.start as usize..found.span.end as usize],
        "[(if false then {return;} else {return;}),[[1]]]"
    );
    let twin = source.replace("==[(if false then {return;} else {return;}),[[1]]]", "==1");
    let twin_errors = check_proposed("causal_obligations.n", &twin, false).unwrap_err();
    assert_eq!(twin_errors.len(), 1);
    assert_eq!(twin_errors[0].code, "NX-E0103");
    let positive = "fn Main(){let x=Len([1])==1;}";
    assert!(check_scalar("causal_obligations.n", positive).is_ok());
    assert!(compile_scalar("causal_obligations.n", positive).is_ok());

    let assigned = "fn Main(){var t0=[];var t2=[];t0=(Len(t2)==[(if false then {return;} else {return;}),[[1]]]);}";
    let assigned_errors = check_proposed("causal_obligations.n", assigned, false).unwrap_err();
    assert_eq!(assigned_errors.len(), 3);
    let independent = &assigned_errors[2];
    independent
        .validate("causal_obligations.n", assigned)
        .unwrap();
    assert_eq!(independent.code, "NX-E0102");
    assert_eq!(independent.subcause.as_deref(), Some("type_mismatch"));
    let primary = independent.primary_span.as_ref().unwrap();
    assert_eq!(
        &assigned[primary.start as usize..primary.end as usize],
        "[(if false then {return;} else {return;}),[[1]]]"
    );
    assert_eq!(independent.data.len(), 2);
    assert_eq!(
        independent.data.get("expected_type"),
        Some(&DataValue::String("I64".into()))
    );
    assert_eq!(
        independent.data.get("found_type"),
        Some(&DataValue::String("Array<Array<Array<I64>>>".into()))
    );
    assert_eq!(independent.related_spans.len(), 2);
    let expected = independent
        .related_spans
        .iter()
        .find(|r| r.role == RelatedRole::ExpectedType)
        .unwrap();
    let found = independent
        .related_spans
        .iter()
        .find(|r| r.role == RelatedRole::FoundType)
        .unwrap();
    assert_eq!(
        &assigned[expected.span.start as usize..expected.span.end as usize],
        "Len(t2)"
    );
    assert_eq!(found.span, *primary);
    let repaired = assigned.replace("==[(if false then {return;} else {return;}),[[1]]]", "==1");
    assert!(causes(&repaired).iter().all(|cause| cause.0 != "NX-E0102"));
}

#[test]
fn pending_len_domain_keeps_independent_result_consumers() {
    // F0-0002 §4.1.2.3 N and §4.1.2.5 N(d): an undecided Len domain
    // leaves the nominal I64 result usable at an independent consumer.
    // A later compatible assignment may resolve the domain after the use.
    let cases = [
        (
            "fn Main(){let p=true;var q=(if true then {return;} else {return;});AssertEq(Len(q)-1,p);}",
            "p",
            "Len(q)-1",
            "Bool",
            "fn Main(){let p=1;var q=(if true then {return;} else {return;});AssertEq(Len(q)-1,p);}",
        ),
        (
            "fn Main(){var q=(if true then {return;} else {return;});let a:I64=Len(q)<2;q=[3];}",
            "Len(q)<2",
            "I64",
            "Bool",
            "fn Main(){var q=(if true then {return;} else {return;});let a:Bool=Len(q)<2;q=[3];}",
        ),
    ];
    for (source, primary, origin, found_type, twin) in cases {
        let errors = check_proposed("causal_obligations.n", source, false).unwrap_err();
        assert_eq!(errors.len(), 1, "{source}: {errors:?}");
        let error = &errors[0];
        error.validate("causal_obligations.n", source).unwrap();
        assert_eq!(error.code, "NX-E0102");
        assert_eq!(error.subcause.as_deref(), Some("type_mismatch"));
        let span = error.primary_span.as_ref().unwrap();
        assert_eq!(&source[span.start as usize..span.end as usize], primary);
        assert_eq!(error.data.len(), 2);
        assert_eq!(
            error.data.get("expected_type"),
            Some(&DataValue::String("I64".into()))
        );
        assert_eq!(
            error.data.get("found_type"),
            Some(&DataValue::String(found_type.into()))
        );
        assert_eq!(error.related_spans.len(), 2);
        let expected = error
            .related_spans
            .iter()
            .find(|r| r.role == RelatedRole::ExpectedType)
            .unwrap();
        let found = error
            .related_spans
            .iter()
            .find(|r| r.role == RelatedRole::FoundType)
            .unwrap();
        assert_eq!(
            &source[expected.span.start as usize..expected.span.end as usize],
            origin
        );
        assert_eq!(
            &source[found.span.start as usize..found.span.end as usize],
            primary
        );
        assert!(check_scalar("causal_obligations.n", twin).is_ok());
        assert!(compile_scalar("causal_obligations.n", twin).is_ok());
    }
    let positive = "fn Main(){let q=[3];let p=1;AssertEq(Len(q)-1,p);let a:Bool=Len(q)<2;}";
    assert!(check_scalar("causal_obligations.n", positive).is_ok());
    assert!(compile_scalar("causal_obligations.n", positive).is_ok());
}

#[test]
fn partial_union_witness_resolves_outer_empty_constructor() {
    // F0-0002 §§4.1.2.2, 4.1.2.5: the valid normal continuation of a
    // tainted inner union may compare with an outer empty Array constructor.
    let source = "fn Main(){let ar=[1];let a=(if true then (if [true] then {value:(1+true);} else ar) else []);}";
    let errors = check_proposed("causal_obligations.n", source, false).unwrap_err();
    assert_eq!(errors.len(), 2);
    for error in &errors {
        error.validate("causal_obligations.n", source).unwrap();
        assert_eq!(error.code, "NX-E0102");
        assert_eq!(error.subcause.as_deref(), Some("type_mismatch"));
    }
    let first = errors[0].primary_span.as_ref().unwrap();
    assert_eq!(&source[first.start as usize..first.end as usize], "[true]");
    let second = errors[1].primary_span.as_ref().unwrap();
    assert_eq!(&source[second.start as usize..second.end as usize], "true");
    assert_eq!(
        errors[0].data.get("expected_type"),
        Some(&DataValue::String("Bool".into()))
    );
    assert_eq!(
        errors[0].data.get("found_type"),
        Some(&DataValue::String("Array<Bool>".into()))
    );
    assert_eq!(
        errors[1].data.get("expected_type"),
        Some(&DataValue::String("I64".into()))
    );
    assert_eq!(
        errors[1].data.get("found_type"),
        Some(&DataValue::String("Bool".into()))
    );
    let positive = "fn Main(){let ar=[1];let a=(if true then (if true then ar else ar) else []);}";
    assert!(check_scalar("causal_obligations.n", positive).is_ok());
    assert!(compile_scalar("causal_obligations.n", positive).is_ok());
}

#[test]
fn unresolved_comparison_does_not_erase_later_assert_argument() {
    // F0-0002 §§4.1.2.4–5: a pending comparison cannot transfer its
    // failed domain to a separate, concrete AssertEq argument.
    let source = "fn Main(){let a=[]==[];AssertEq(1,1<1);}";
    let errors = check_proposed("causal_obligations.n", source, false).unwrap_err();
    assert_eq!(errors.len(), 3);
    for error in &errors {
        error.validate("causal_obligations.n", source).unwrap();
    }
    assert_eq!(errors[0].code, "NX-E0103");
    assert_eq!(errors[1].code, "NX-E0103");
    for unresolved in &errors[..2] {
        assert_eq!(unresolved.subcause, None);
        assert!(unresolved.data.is_empty());
        assert!(unresolved.related_spans.is_empty());
        let span = unresolved.primary_span.as_ref().unwrap();
        assert_eq!(&source[span.start as usize..span.end as usize], "[]");
    }
    let mismatch = &errors[2];
    assert_eq!(mismatch.code, "NX-E0102");
    assert_eq!(mismatch.subcause.as_deref(), Some("type_mismatch"));
    let span = mismatch.primary_span.as_ref().unwrap();
    assert_eq!(&source[span.start as usize..span.end as usize], "1<1");
    assert_eq!(mismatch.data.len(), 2);
    assert_eq!(
        mismatch.data.get("expected_type"),
        Some(&DataValue::String("I64".into()))
    );
    assert_eq!(
        mismatch.data.get("found_type"),
        Some(&DataValue::String("Bool".into()))
    );
    assert_eq!(mismatch.related_spans.len(), 2);
    let expected = mismatch
        .related_spans
        .iter()
        .find(|r| r.role == RelatedRole::ExpectedType)
        .unwrap();
    assert_eq!(
        &source[expected.span.start as usize..expected.span.end as usize],
        "1"
    );
    let found = mismatch
        .related_spans
        .iter()
        .find(|r| r.role == RelatedRole::FoundType)
        .unwrap();
    assert_eq!(
        &source[found.span.start as usize..found.span.end as usize],
        "1<1"
    );
    let twin = "fn Main(){let a=[1]==[1];AssertEq(1,1<1);}";
    assert_eq!(
        causes(twin),
        vec![(
            "NX-E0102".into(),
            Some("type_mismatch".into()),
            "1<1".into()
        )]
    );
    let positive = "fn Main(){let a=[1]==[1];AssertEq(1,1);}";
    assert!(check_scalar("causal_obligations.n", positive).is_ok());
    assert!(compile_scalar("causal_obligations.n", positive).is_ok());
}

#[test]
fn pending_comparison_keeps_independent_numeric_rhs_domain() {
    // F0-0002 §§4.1.2.4–5: the right operand of + has a concrete
    // Array domain even when the left comparison is unresolved.
    let source = "fn Main(){let ar=[1];let a=((if true then {return;} else {return;})<[[]])+ar;}";
    let errors = check_proposed("causal_obligations.n", source, false).unwrap_err();
    assert_eq!(errors.len(), 3);
    for error in &errors {
        error.validate("causal_obligations.n", source).unwrap();
    }
    assert_eq!(errors[0].code, "NX-E0102");
    assert_eq!(errors[0].subcause.as_deref(), Some("type_mismatch"));
    let comparison = errors[0].primary_span.as_ref().unwrap();
    assert_eq!(
        &source[comparison.start as usize..comparison.end as usize],
        "(if true then {return;} else {return;})<[[]]"
    );
    assert_eq!(errors[0].data.len(), 2);
    assert_eq!(
        errors[0].data.get("expected_type"),
        Some(&DataValue::String("I64".into()))
    );
    assert_eq!(
        errors[0].data.get("found_type"),
        Some(&DataValue::String("Bool".into()))
    );
    assert_eq!(errors[0].related_spans.len(), 1);
    assert_eq!(errors[0].related_spans[0].role, RelatedRole::FoundType);
    assert_eq!(errors[1].code, "NX-E0103");
    assert_eq!(errors[1].subcause, None);
    assert!(errors[1].data.is_empty());
    assert!(errors[1].related_spans.is_empty());
    let unresolved = errors[1].primary_span.as_ref().unwrap();
    assert_eq!(
        &source[unresolved.start as usize..unresolved.end as usize],
        "[]"
    );
    let rhs = &errors[2];
    assert_eq!(rhs.code, "NX-E0102");
    assert_eq!(rhs.subcause.as_deref(), Some("type_mismatch"));
    let span = rhs.primary_span.as_ref().unwrap();
    assert_eq!(&source[span.start as usize..span.end as usize], "ar");
    assert_eq!(rhs.data.len(), 2);
    assert_eq!(
        rhs.data.get("expected_type"),
        Some(&DataValue::String("I64".into()))
    );
    assert_eq!(
        rhs.data.get("found_type"),
        Some(&DataValue::String("Array<I64>".into()))
    );
    assert_eq!(rhs.related_spans.len(), 1);
    assert_eq!(rhs.related_spans[0].role, RelatedRole::FoundType);
    let related = &rhs.related_spans[0].span;
    assert_eq!(&source[related.start as usize..related.end as usize], "ar");
    let twin = source.replace("+ar", "+1");
    assert_eq!(
        causes(&twin),
        vec![
            (
                "NX-E0102".into(),
                Some("type_mismatch".into()),
                "(if true then {return;} else {return;})<[[]]".into(),
            ),
            ("NX-E0103".into(), None, "[]".into()),
        ]
    );
    let positive = "fn Main(){let ar=1;let a=(ar+1);}";
    assert!(check_scalar("causal_obligations.n", positive).is_ok());
    assert!(compile_scalar("causal_obligations.n", positive).is_ok());
}

#[test]
fn intrinsic_numeric_prefix_has_no_source_expected_relation() {
    // F0-0002 §§4.1.2.2, 4.1.2.4 and 13.15: the intrinsic I64 domain
    // of a numeric operation has no source span to cite as expected_type.
    let source = "fn Main(){var r=[];let a:Bool=1+r[0];r=[true];}";
    let errors = check_proposed("causal_obligations.n", source, false).unwrap_err();
    assert_eq!(errors.len(), 2);
    for error in &errors {
        error.validate("causal_obligations.n", source).unwrap();
        assert_eq!(error.code, "NX-E0102");
        assert_eq!(error.subcause.as_deref(), Some("type_mismatch"));
        assert_eq!(error.data.len(), 2);
    }
    let outer = &errors[0];
    let span = outer.primary_span.as_ref().unwrap();
    assert_eq!(&source[span.start as usize..span.end as usize], "1+r[0]");
    assert_eq!(
        outer.data.get("expected_type"),
        Some(&DataValue::String("Bool".into()))
    );
    assert_eq!(
        outer.data.get("found_type"),
        Some(&DataValue::String("I64".into()))
    );
    assert_eq!(outer.related_spans.len(), 2);
    let assignment = &errors[1];
    let span = assignment.primary_span.as_ref().unwrap();
    assert_eq!(&source[span.start as usize..span.end as usize], "true");
    assert_eq!(
        assignment.data.get("expected_type"),
        Some(&DataValue::String("I64".into()))
    );
    assert_eq!(
        assignment.data.get("found_type"),
        Some(&DataValue::String("Bool".into()))
    );
    assert_eq!(assignment.related_spans.len(), 1);
    assert_eq!(assignment.related_spans[0].role, RelatedRole::FoundType);
    let related = &assignment.related_spans[0].span;
    assert_eq!(
        &source[related.start as usize..related.end as usize],
        "true"
    );
    let twin = "fn Main(){var r=[];let a:Bool=1+r[0];r=[1];}";
    assert_eq!(
        causes(twin),
        vec![(
            "NX-E0102".into(),
            Some("type_mismatch".into()),
            "1+r[0]".into()
        )]
    );
    let positive = "fn Main(){var r=[];let a:I64=1+r[0];r=[1];}";
    assert!(check_scalar("causal_obligations.n", positive).is_ok());
    assert!(compile_scalar("causal_obligations.n", positive).is_ok());
}

#[test]
fn first_indexed_array_assignment_owns_later_element_origin() {
    // F0-0002 §§4.1.2.2, 4.1.2.6 and 13.17: an indexed expression
    // may supply a concrete first assignment without moving its origin
    // from the complete accepted Array constructor to the indexed child.
    let source = "fn Main(){var o=[];o=[(if true then [1] else [2])[0]];o=[true];}";
    let errors = check_proposed("causal_obligations.n", source, false).unwrap_err();
    assert_eq!(errors.len(), 1);
    let error = &errors[0];
    error.validate("causal_obligations.n", source).unwrap();
    assert_eq!(error.code, "NX-E0102");
    assert_eq!(error.subcause.as_deref(), Some("type_mismatch"));
    let span = error.primary_span.as_ref().unwrap();
    assert_eq!(&source[span.start as usize..span.end as usize], "true");
    assert_eq!(error.data.len(), 2);
    assert_eq!(
        error.data.get("expected_type"),
        Some(&DataValue::String("I64".into()))
    );
    assert_eq!(
        error.data.get("found_type"),
        Some(&DataValue::String("Bool".into()))
    );
    assert_eq!(error.related_spans.len(), 2);
    let expected = error
        .related_spans
        .iter()
        .find(|r| r.role == RelatedRole::ExpectedType)
        .unwrap();
    assert_eq!(
        &source[expected.span.start as usize..expected.span.end as usize],
        "[(if true then [1] else [2])[0]]"
    );
    let found = error
        .related_spans
        .iter()
        .find(|r| r.role == RelatedRole::FoundType)
        .unwrap();
    assert_eq!(
        &source[found.span.start as usize..found.span.end as usize],
        "true"
    );
    let twin = source.replace("o=[true]", "o=[3]");
    assert!(check_scalar("causal_obligations.n", &twin).is_ok());
    let positive = "fn Main(){var o=[];o=[(if true then [1] else [2])[0]];o=[3];}";
    assert!(compile_scalar("causal_obligations.n", positive).is_ok());

    let nested =
        "fn Main(){var o=[];o=[(if true then [[1]] else [[2],[3]])[0],[0,1],[2,3]];o=[true];}";
    let nested_errors = check_proposed("causal_obligations.n", nested, false).unwrap_err();
    assert_eq!(nested_errors.len(), 1);
    let nested_error = &nested_errors[0];
    nested_error
        .validate("causal_obligations.n", nested)
        .unwrap();
    assert_eq!(nested_error.code, "NX-E0102");
    assert_eq!(nested_error.subcause.as_deref(), Some("type_mismatch"));
    let span = nested_error.primary_span.as_ref().unwrap();
    assert_eq!(&nested[span.start as usize..span.end as usize], "true");
    assert_eq!(nested_error.data.len(), 2);
    assert_eq!(
        nested_error.data.get("expected_type"),
        Some(&DataValue::String("Array<I64>".into()))
    );
    assert_eq!(
        nested_error.data.get("found_type"),
        Some(&DataValue::String("Bool".into()))
    );
    assert_eq!(nested_error.related_spans.len(), 2);
    let origin = nested_error
        .related_spans
        .iter()
        .find(|r| r.role == RelatedRole::ExpectedType)
        .unwrap();
    assert_eq!(
        &nested[origin.span.start as usize..origin.span.end as usize],
        "[(if true then [[1]] else [[2],[3]])[0],[0,1],[2,3]]"
    );
    let nested_twin = nested.replace("o=[true]", "o=[[4],[5]]");
    assert!(check_scalar("causal_obligations.n", &nested_twin).is_ok());
    assert!(compile_scalar("causal_obligations.n", &nested_twin).is_ok());
}

#[test]
fn late_array_assignment_resolves_guarded_assert_equality() {
    // F0-0002 §§4.1.2.2–5: once the later Array assignment establishes
    // both selector and Len domains, their comparison has a Bool result.
    // AssertEq must then check its independent I64 argument.
    let deferred = "fn Main(){let ar=[1];var r=(if true then {return;} else {return;});AssertEq((r[0]-Len(r))<1,1);r=ar;}";
    let concrete = "fn Main(){let ar=[1];let r=ar;AssertEq((r[0]-Len(r))<1,1);}";
    let aliased = "fn Main(){let ar=[1];var r=(if true then {return;} else {return;});var q=r;AssertEq((r[0]-Len(r))<1,1);r=ar;}";
    for source in [deferred, concrete, aliased] {
        let errors = check_proposed("causal_obligations.n", source, false).unwrap_err();
        assert_eq!(errors.len(), 1, "{source}");
        let error = &errors[0];
        error.validate("causal_obligations.n", source).unwrap();
        assert_eq!(error.code, "NX-E0102");
        assert_eq!(error.subcause.as_deref(), Some("type_mismatch"));
        let span = error.primary_span.as_ref().unwrap();
        assert_eq!(&source[span.start as usize..span.end as usize], "1");
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
        let expected = error
            .related_spans
            .iter()
            .find(|r| r.role == RelatedRole::ExpectedType)
            .unwrap();
        assert_eq!(
            &source[expected.span.start as usize..expected.span.end as usize],
            "(r[0]-Len(r))<1"
        );
        let found = error
            .related_spans
            .iter()
            .find(|r| r.role == RelatedRole::FoundType)
            .unwrap();
        assert_eq!(
            &source[found.span.start as usize..found.span.end as usize],
            "1"
        );
        assert!(matches!(
            check_scalar("causal_obligations.n", source),
            Err(CompileError::Rejected(_))
        ));
    }
    let positive = "fn Main(){let ar=[1];var r=(if true then {return;} else {return;});AssertEq((r[0]-Len(r))<1,true);r=ar;}";
    assert!(check_scalar("causal_obligations.n", positive).is_ok());
    assert!(compile_scalar("causal_obligations.n", positive).is_ok());
}

#[test]
fn guarded_equality_requires_every_pending_operand_domain() {
    let unresolved =
        "fn Main(){var r=(if true then {return;} else {return;});AssertEq((r[0]-Len(r))<1,1);}";
    let wrong_element = "fn Main(){let ar=[true];var r=(if true then {return;} else {return;});AssertEq((r[0]-Len(r))<1,1);r=ar;}";
    let wrong_shape =
        "fn Main(){var r=(if true then {return;} else {return;});AssertEq((r[0]-Len(r))<1,1);r=2;}";
    let wrong_selector = "fn Main(){let ar=[1];var r=(if true then {return;} else {return;});AssertEq((r[true]-Len(r))<1,1);r=ar;}";
    for source in [unresolved, wrong_element, wrong_shape, wrong_selector] {
        let errors = check_proposed("causal_obligations.n", source, false)
            .err()
            .unwrap_or_default();
        for error in &errors {
            error.validate("causal_obligations.n", source).unwrap();
            assert!(
                !error.related_spans.iter().any(|related| {
                    related.role == RelatedRole::ExpectedType
                        && source[related.span.start as usize..related.span.end as usize]
                            .starts_with("(r[")
                }),
                "conditional Bool/I64 comparison leaked after a failed domain: {source}: {error:?}"
            );
        }
        if source != unresolved {
            assert!(matches!(
                check_scalar("causal_obligations.n", source),
                Err(CompileError::Rejected(_))
            ));
        }
    }
    let concrete_selector = "fn Main(){let ar=[1];let r=ar;AssertEq((r[true]-Len(r))<1,1);}";
    for source in [wrong_selector, concrete_selector] {
        let errors = check_proposed("causal_obligations.n", source, false).unwrap_err();
        assert_eq!(errors.len(), 1, "{source}");
        let error = &errors[0];
        error.validate("causal_obligations.n", source).unwrap();
        assert_eq!(error.code, "NX-E0102");
        assert_eq!(error.subcause.as_deref(), Some("type_mismatch"));
        let primary = error.primary_span.as_ref().unwrap();
        assert_eq!(
            &source[primary.start as usize..primary.end as usize],
            "true"
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
        assert_eq!(error.related_spans[0].span, *primary);
        assert!(matches!(
            compile_scalar("causal_obligations.n", source),
            Err(CompileError::Rejected(_))
        ));
    }
    let repaired = concrete_selector
        .replace("r[true]", "r[0]")
        .replace(",1);", ",true);");
    assert!(check_scalar("causal_obligations.n", &repaired).is_ok());
    assert!(compile_scalar("causal_obligations.n", &repaired).is_ok());
}

#[test]
fn failed_empty_comparison_keeps_later_assert_argument() {
    // F0-0002 §4.1.2.4–5 N(d): the later AssertEq argument has its own C.
    let source = "fn Main(){if (([]==[])==false) {}AssertEq(1,());}";
    assert_eq!(causes(source).len(), 3);
    exact_cause(source, 0, "NX-E0103", None, "[]", None, None, &[]);
    exact_cause(source, 1, "NX-E0103", None, "[]", None, None, &[]);
    exact_cause(
        source,
        2,
        "NX-E0102",
        Some("type_mismatch"),
        "()",
        Some("I64"),
        Some("()"),
        &[
            (RelatedRole::ExpectedType, "1"),
            (RelatedRole::FoundType, "()"),
        ],
    );
    let twin = source.replace("AssertEq(1,())", "AssertEq(1,2)");
    assert_eq!(causes(&twin).len(), 2);
    let positive = source
        .replace("[]==[]", "[1]==[1]")
        .replace("AssertEq(1,())", "AssertEq(1,1)");
    assert!(check_scalar("causal_obligations.n", &positive).is_ok());
    assert!(compile_scalar("causal_obligations.n", &positive).is_ok());
}

#[test]
fn deferred_array_siblings_keep_independent_mismatches() {
    // F0-0002 §4.1.2.4–5 N(d): each sibling has an external Bool element type.
    let source =
        "fn Main(){var q=(if true then {return;} else {return;});let a=1-[true,Len(q),()];q=[1];}";
    assert_eq!(causes(source).len(), 2);
    exact_cause(
        source,
        0,
        "NX-E0102",
        Some("type_mismatch"),
        "Len(q)",
        Some("Bool"),
        Some("I64"),
        &[
            (RelatedRole::ExpectedType, "true"),
            (RelatedRole::FoundType, "Len(q)"),
        ],
    );
    exact_cause(
        source,
        1,
        "NX-E0102",
        Some("type_mismatch"),
        "()",
        Some("Bool"),
        Some("()"),
        &[
            (RelatedRole::ExpectedType, "true"),
            (RelatedRole::FoundType, "()"),
        ],
    );
    let twin = source.replace("Len(q),()", "Len(q),false");
    assert_eq!(causes(&twin).len(), 1);
    let positive = "fn Main(){let a=[true,false,false];}";
    assert!(check_scalar("causal_obligations.n", positive).is_ok());
    assert!(compile_scalar("causal_obligations.n", positive).is_ok());
}

#[test]
fn first_array_constraint_owns_later_assignment_origin() {
    // F0-0002 §§4.1.2.2, 4.1.2.6, 13.15 N(g): use the first
    // source constraint on v0, not the prior assignment to v2.
    let source = "fn Main(){var v0=[];var v2=(if false then {return;} else {return;});v2=[1];let q3=(v0<v2);v0=true;v0=v2;}";
    assert_eq!(causes(source).len(), 2);
    exact_cause(
        source,
        0,
        "NX-E0102",
        Some("type_mismatch"),
        "<",
        Some("I64"),
        Some("Array<I64>"),
        &[(RelatedRole::FoundType, "v0")],
    );
    exact_cause(
        source,
        1,
        "NX-E0102",
        Some("type_mismatch"),
        "true",
        Some("Array<I64>"),
        Some("Bool"),
        &[
            (RelatedRole::ExpectedType, "v2"),
            (RelatedRole::FoundType, "true"),
        ],
    );
    let twin = source.replace("v0=true", "v0=[2]");
    assert_eq!(causes(&twin).len(), 1);
    let positive = "fn Main(){var v0=[1];var v2=[2];let q3=(v0==v2);v0=v2;}";
    assert!(check_scalar("causal_obligations.n", positive).is_ok());
    assert!(compile_scalar("causal_obligations.n", positive).is_ok());
}

#[test]
fn equality_domain_relation_points_to_resolved_operand() {
    // F0-0002 §§4.6, 13.15 N(g,i): the MockResource operand is a.
    let source = "fn Main(){var a=(if true then {return;} else {return;});var b=(if true then {return;} else {return;});let c=a==b;a=MockOpen(1);}";
    assert_eq!(causes(source).len(), 1);
    exact_cause(
        source,
        0,
        "NX-E0102",
        Some("type_mismatch"),
        "==",
        Some("domain:copyable_data"),
        Some("MockResource"),
        &[(RelatedRole::FoundType, "a")],
    );
    let positive = source.replace("a=MockOpen(1)", "a=1;b=2");
    assert!(check_scalar("causal_obligations.n", &positive).is_ok());
    assert!(compile_scalar("causal_obligations.n", &positive).is_ok());
}

#[test]
fn tainted_assert_argument_keeps_inner_comparison_cause() {
    // F0-0002 §4.1.2.4–5 N(d): the RHS comparison has an independent C.
    let source = "fn Main(){var t0=Len(true);AssertEq(t0,([1]<true));}";
    assert_eq!(causes(source).len(), 2);
    exact_cause(
        source,
        0,
        "NX-E0102",
        Some("not_indexable"),
        "true",
        Some("domain:array"),
        Some("Bool"),
        &[(RelatedRole::FoundType, "true")],
    );
    exact_cause(
        source,
        1,
        "NX-E0102",
        Some("type_mismatch"),
        "true",
        Some("Array<I64>"),
        Some("Bool"),
        &[
            (RelatedRole::ExpectedType, "[1]"),
            (RelatedRole::FoundType, "true"),
        ],
    );
    let twin = source.replace("[1]<true", "[1]<[2]");
    assert_eq!(causes(&twin).len(), 2);
    let positive = "fn Main(){var t0=Len([1]);AssertEq(t0,Len([2]));}";
    assert!(check_scalar("causal_obligations.n", positive).is_ok());
    assert!(compile_scalar("causal_obligations.n", positive).is_ok());
}

#[test]
fn invalid_selector_condition_keeps_unresolved_constructor() {
    // F0-0002 §§4.1.2.2, 4.1.2.5 N(h): [] keeps its own obligation
    // even when another intrinsic domain or selector subexpression fails.
    let source = "fn Main(){let c=true;let x=1;var t0=Len(c);var t2=(if c then {return;} else {return;});t2[(if [] then {value:c;} else t0)]=(t2[x]);let u2=t2[(0-false)];}";
    assert_eq!(causes(source).len(), 3);
    exact_cause(
        source,
        0,
        "NX-E0102",
        Some("not_indexable"),
        "c",
        Some("domain:array"),
        Some("Bool"),
        &[(RelatedRole::FoundType, "c")],
    );
    exact_cause(source, 1, "NX-E0103", None, "[]", None, None, &[]);
    exact_cause(
        source,
        2,
        "NX-E0102",
        Some("type_mismatch"),
        "false",
        Some("I64"),
        Some("Bool"),
        &[(RelatedRole::FoundType, "false")],
    );
    let twin = source.replace("if [] then", "if true then");
    assert_eq!(causes(&twin).len(), 2);
    let positive = "fn Main(){let a=[1];let x=0;let y=a[x];}";
    assert!(check_scalar("causal_obligations.n", positive).is_ok());
    assert!(compile_scalar("causal_obligations.n", positive).is_ok());
}

#[test]
fn concrete_array_element_owns_homogeneity_origin() {
    // F0-0002 §§4.1.2.4, 13.15 N(g,i): w has no constructor witness;
    // [1] is the source of the expected Array<I64>.
    let source = "fn Main(){var w=(if true then {return;} else {return;});let a0=[w,[1]==w];}";
    assert_eq!(causes(source).len(), 1);
    exact_cause(
        source,
        0,
        "NX-E0102",
        Some("type_mismatch"),
        "[1]==w",
        Some("Array<I64>"),
        Some("Bool"),
        &[
            (RelatedRole::ExpectedType, "[1]"),
            (RelatedRole::FoundType, "[1]==w"),
        ],
    );
    let positive = source.replace("[1]==w", "[1]");
    assert!(check_scalar("causal_obligations.n", &positive).is_ok());
    assert!(compile_scalar("causal_obligations.n", &positive).is_ok());
}

#[test]
fn nested_union_does_not_erase_later_array_element_cause() {
    // F0-0002 §4.1.2.4–5 N(d): true is a later, independent array element.
    let source = "fn Main(){let x=1;let ar=[1];var m0=([[],(if ((if false then {return;} else {return;})) then {value:(0==x);} else (if x then ar else [])),true]+1);}";
    assert_eq!(causes(source).len(), 3);
    exact_cause(
        source,
        0,
        "NX-E0102",
        Some("type_mismatch"),
        "0==x",
        Some("Array<I64>"),
        Some("Bool"),
        &[
            (RelatedRole::ExpectedType, "[]"),
            (RelatedRole::FoundType, "0==x"),
        ],
    );
    exact_cause(
        source,
        1,
        "NX-E0102",
        Some("type_mismatch"),
        "x",
        Some("Bool"),
        Some("I64"),
        &[(RelatedRole::FoundType, "x")],
    );
    exact_cause(
        source,
        2,
        "NX-E0102",
        Some("type_mismatch"),
        "true",
        Some("Array<I64>"),
        Some("Bool"),
        &[
            (RelatedRole::ExpectedType, "[]"),
            (RelatedRole::FoundType, "true"),
        ],
    );
    let twin = source.replace(")),true]+1", ")),ar]+1");
    assert_eq!(causes(&twin).len(), 2);
    let positive = "fn Main(){let ar=[1];let a=[ar,ar,ar];}";
    assert!(check_scalar("causal_obligations.n", positive).is_ok());
    assert!(compile_scalar("causal_obligations.n", positive).is_ok());
}

#[test]
fn self_referential_len_guard_uses_later_array_witness() {
    // F0-0002 §4.1.2.3: Len(q)'s nominal I64 may be compared only
    // conditionally; that comparison cannot itself refute Len's Array domain.
    let source =
        "fn Main(){var q=(if true then {return;} else {return;});AssertEq(q,Len(q));q=[1];}";
    assert_eq!(causes(source).len(), 1);
    exact_cause(
        source,
        0,
        "NX-E0102",
        Some("type_mismatch"),
        "Len(q)",
        Some("Array<I64>"),
        Some("I64"),
        &[
            (RelatedRole::ExpectedType, "q"),
            (RelatedRole::FoundType, "Len(q)"),
        ],
    );
    let twin = source.replace("q=[1]", "q=1");
    assert_eq!(causes(&twin).len(), 1);
    exact_cause(
        &twin,
        0,
        "NX-E0102",
        Some("not_indexable"),
        "q",
        Some("domain:array"),
        Some("I64"),
        &[(RelatedRole::FoundType, "q")],
    );
    let positive = "fn Main(){let q=[1];AssertEq(Len(q),1);}";
    assert!(check_scalar("causal_obligations.n", positive).is_ok());
    assert!(compile_scalar("causal_obligations.n", positive).is_ok());
}

#[test]
fn compatible_array_alias_keeps_first_concrete_element_origin() {
    // F0-0002 §§4.1.2.1–2, 13.15 N(g): the equality relates
    // unknown elements, while [true] first fixes their concrete type.
    let source = "fn Main(){var v0=[];var v1=[];let q1=(v1==v0);v1=[true];v0=[true];v1[0]=[1];}";
    assert_eq!(causes(source).len(), 1);
    exact_cause(
        source,
        0,
        "NX-E0102",
        Some("type_mismatch"),
        "[1]",
        Some("Bool"),
        Some("Array<I64>"),
        &[
            (RelatedRole::ExpectedType, "[true]"),
            (RelatedRole::FoundType, "[1]"),
        ],
    );
    let positive = source.replace("v1[0]=[1]", "v1[0]=true");
    assert!(check_scalar("causal_obligations.n", &positive).is_ok());
    assert!(compile_scalar("causal_obligations.n", &positive).is_ok());
}

#[test]
fn recursive_array_assignment_keeps_expected_source_origin() {
    // F0-0002 §§4.1.2.2, 13.15 N(g): the recursive assignment fails
    // at its RHS; the later valid indexed write completes its types only.
    let source = "fn Main(){var t0=[];t0=t0[1];t0[0]=1;}";
    assert_eq!(causes(source).len(), 1);
    exact_cause(
        source,
        0,
        "NX-E0102",
        Some("type_mismatch"),
        "t0[1]",
        Some("Array<I64>"),
        Some("I64"),
        &[
            (RelatedRole::ExpectedType, "t0"),
            (RelatedRole::FoundType, "t0[1]"),
        ],
    );
    let unresolved = source.replace("t0[0]=1;", "");
    assert_eq!(causes(&unresolved).len(), 1);
    exact_cause(&unresolved, 0, "NX-E0103", None, "[]", None, None, &[]);
    let positive = source.replace("t0=t0[1]", "t0=[1]");
    assert!(check_scalar("causal_obligations.n", &positive).is_ok());
    assert!(compile_scalar("causal_obligations.n", &positive).is_ok());
}

#[test]
fn failed_inner_union_preserves_outer_empty_constructor_witness() {
    // F0-0002 §§4.1.2.2 and 4.1.2.5 N(h): the valid `ar` arm of
    // the inner failed union still resolves the outer union's `[]`.
    let source = "fn Main(){let ar=[1];let m3=Len(true);let l4=[(if true then {value:(if true then (if true then {value:m3;} else [[]]) else ar);} else [])];}";
    assert_eq!(causes(source).len(), 2);
    exact_cause(
        source,
        0,
        "NX-E0102",
        Some("not_indexable"),
        "true",
        Some("domain:array"),
        Some("Bool"),
        &[(RelatedRole::FoundType, "true")],
    );
    exact_cause(source, 1, "NX-E0103", None, "[]", None, None, &[]);
    let inner_resolved = source.replace("else [[]]", "else [1]");
    assert_eq!(causes(&inner_resolved).len(), 1);
    exact_cause(
        &inner_resolved,
        0,
        "NX-E0102",
        Some("not_indexable"),
        "true",
        Some("domain:array"),
        Some("Bool"),
        &[(RelatedRole::FoundType, "true")],
    );
    let outer_resolved = source.replace("else [])];}", "else [1])];}");
    assert_eq!(causes(&outer_resolved).len(), 2);
    let positive = "fn Main(){let ar=[[1]];let m3=[[2]];let l4=[(if true then {value:(if true then (if true then {value:m3;} else [[]]) else ar);} else [])];}";
    assert!(check_scalar("causal_obligations.n", positive).is_ok());
    assert!(compile_scalar("causal_obligations.n", positive).is_ok());
}

#[test]
fn pending_inner_constructor_preserves_outer_branch_comparison() {
    // F0-0002 §4.1.2.5: an unresolved constructor cannot erase the
    // independent valid `false` continuation of an inner union when an
    // outer union already provides the concrete I64 anchor.
    let source = "fn Main(){let ar=[1];let c=true;var m1=(if true then {value:ar[0];} else (if c then [[]] else false));}";
    assert_eq!(causes(source).len(), 2);
    exact_cause(source, 0, "NX-E0103", None, "[]", None, None, &[]);
    exact_cause(
        source,
        1,
        "NX-E1303",
        None,
        "false",
        Some("I64"),
        Some("Bool"),
        &[
            (RelatedRole::ExpectedType, "ar[0]"),
            (RelatedRole::FoundType, "false"),
        ],
    );
    let repaired_false = source.replace("else false", "else 1");
    assert_eq!(causes(&repaired_false).len(), 1);
    exact_cause(&repaired_false, 0, "NX-E0103", None, "[]", None, None, &[]);
    let repaired_constructor = source.replace("then [[]]", "then 0");
    assert_eq!(causes(&repaired_constructor).len(), 1);
    exact_cause(
        &repaired_constructor,
        0,
        "NX-E1303",
        None,
        "false",
        Some("I64"),
        Some("Bool"),
        &[
            (RelatedRole::ExpectedType, "ar[0]"),
            (RelatedRole::FoundType, "false"),
        ],
    );
    let positive = repaired_constructor.replace("else false", "else 1");
    assert!(check_scalar("causal_obligations.n", &positive).is_ok());
    assert!(compile_scalar("causal_obligations.n", &positive).is_ok());
}

#[test]
fn recursive_shape_planning_keeps_causal_sites_stable() {
    // F0-0002 §4.1.2 N(a), §4.1.2.2 and §13.15 N(g): removing
    // a recursive shape may not skip later source sites in the plan.
    let source = "fn Main(){let br=[true];var m4=[];var m5=[];m4=[m5,[],(if true then {value:(if br then {value:m5;} else m4);} else [br])];}";
    for _ in 0..32 {
        assert_eq!(causes(source).len(), 2);
        exact_cause(
            source,
            0,
            "NX-E0102",
            Some("type_mismatch"),
            "br",
            Some("Bool"),
            Some("Array<Bool>"),
            &[(RelatedRole::FoundType, "br")],
        );
        exact_cause(
            source,
            1,
            "NX-E0102",
            Some("type_mismatch"),
            "m4",
            Some("Array<Array<Bool>>"),
            Some("Array<Array<Array<Bool>>>"),
            &[(RelatedRole::FoundType, "m4")],
        );
    }
    let repaired_condition = source.replace("if br then", "if true then");
    assert_eq!(causes(&repaired_condition).len(), 1);
    let positive = repaired_condition.replace("else m4", "else m5");
    assert!(check_scalar("causal_obligations.n", &positive).is_ok());
    assert!(compile_scalar("causal_obligations.n", &positive).is_ok());
}

#[test]
fn numeric_operation_keeps_one_domain_rejection() {
    // F0-0002 §4.1.2.4 N(d) and §4.6: a failed subtraction
    // reports its first incompatible numeric operand only.
    let source =
        "fn Main(){let c=true;let a=(if ((if [] then (c) else {return;})-[true]) then 1 else 1);}";
    assert_eq!(causes(source).len(), 2);
    exact_cause(source, 0, "NX-E0103", None, "[]", None, None, &[]);
    exact_cause(
        source,
        1,
        "NX-E0102",
        Some("type_mismatch"),
        "c",
        Some("I64"),
        Some("Bool"),
        &[(RelatedRole::FoundType, "c")],
    );
    let repaired_right = source.replace("-[true]", "-1");
    assert_eq!(causes(&repaired_right).len(), 2);
    let positive = "fn Main(){let c=1;let a=((if true then c else {return;})-1);}";
    assert!(check_scalar("causal_obligations.n", positive).is_ok());
    assert!(compile_scalar("causal_obligations.n", positive).is_ok());
}

#[test]
fn failed_outer_array_keeps_nested_element_cause() {
    // F0-0002 §4.1.2.4–5 N(d): unresolved outer siblings do not
    // consume the independent mismatch within the last array element.
    let source = "fn Main(){let x=1;var t2=[];t2[1]=[t2[x],[t2],[x,[1]]];}";
    assert_eq!(causes(source).len(), 3);
    exact_cause(source, 0, "NX-E0103", None, "[]", None, None, &[]);
    exact_cause(source, 1, "NX-E0103", None, "[t2]", None, None, &[]);
    exact_cause(
        source,
        2,
        "NX-E0102",
        Some("type_mismatch"),
        "[1]",
        Some("I64"),
        Some("Array<I64>"),
        &[
            (RelatedRole::ExpectedType, "x"),
            (RelatedRole::FoundType, "[1]"),
        ],
    );
    let repaired_element = source.replace("[x,[1]]", "[x,1]");
    assert!(
        !causes(&repaired_element)
            .iter()
            .any(|cause| cause.2 == "[1]")
    );
    let positive = "fn Main(){var t2=[[1]];t2[0]=[2];}";
    assert!(check_scalar("causal_obligations.n", positive).is_ok());
    assert!(compile_scalar("causal_obligations.n", positive).is_ok());
}

#[test]
fn unresolved_constructor_survives_failed_nested_union_condition() {
    // F0-0002 §4.1.2.2 and §4.1.2.5 N(d): a failed condition cannot
    // discharge the independent unresolved constructor in its branch.
    let source = "fn Main(){var t0=[];(if true then{value:(if true then{value:(if 1 then{}else t0);}else(if true then true else true));}else([5]));}";
    assert_eq!(
        causes(source),
        vec![
            ("NX-E0103".into(), None, "[]".into()),
            ("NX-E0102".into(), Some("type_mismatch".into()), "1".into()),
            ("NX-E1303".into(), None, "[5]".into()),
        ]
    );
    exact_cause(source, 0, "NX-E0103", None, "[]", None, None, &[]);
    let errors = check_proposed("causal_obligations.n", source, false).unwrap_err();
    assert_eq!(errors[0].primary_span.as_ref().unwrap().start, 17);

    let valid_condition = source.replace("if 1 then", "if true then");
    assert!(
        causes(&valid_condition)
            .iter()
            .any(|(code, _, primary)| code == "NX-E0103" && primary == "[]")
    );
    let resolved = source.replace("var t0=[]", "var t0=[1]");
    assert!(
        !causes(&resolved)
            .iter()
            .any(|(code, _, _)| code == "NX-E0103")
    );
    let positive = "fn Main(){var t0=[1];let x=t0[0];AssertEq(x,1);}";
    assert!(check_scalar("causal_obligations.n", positive).is_ok());
    assert!(compile_scalar("causal_obligations.n", positive).is_ok());
}

#[test]
fn comparison_rejects_once_at_first_cross_type_conflict() {
    // F0-0002 §4.6 and §4.1.2.4 N(d): different concrete operand
    // types use the right operand site, without another `<` rejection.
    let source = "fn Main(){let ar=[1];var t0=(if true then {return;} else {return;});var t1=(if true then {return;} else {return;});let a=(ar<(t0==t1));}";
    assert_eq!(causes(source).len(), 1);
    exact_cause(
        source,
        0,
        "NX-E0102",
        Some("type_mismatch"),
        "t0==t1",
        Some("Array<I64>"),
        Some("Bool"),
        &[
            (RelatedRole::ExpectedType, "ar"),
            (RelatedRole::FoundType, "t0==t1"),
        ],
    );
    let scalar_left = source.replace("let ar=[1]", "let ar=1");
    assert_eq!(causes(&scalar_left).len(), 1);
    let positive = "fn Main(){let ar=1;let a=(ar<2);}";
    assert!(check_scalar("causal_obligations.n", positive).is_ok());
    assert!(compile_scalar("causal_obligations.n", positive).is_ok());
}

#[test]
fn pending_len_left_keeps_independent_comparison_right() {
    // F0-0002 §4.1.2.3–5 N(d): the unresolved left domain does not
    // consume the concrete numeric mismatch on the right of `<`.
    let source = "fn Main(){let x=1;let m0=(Len([][x])+1)<[1];}";
    assert_eq!(causes(source).len(), 3);
    exact_cause(source, 0, "NX-E0103", None, "[]", None, None, &[]);
    exact_cause(
        source,
        2,
        "NX-E0102",
        Some("type_mismatch"),
        "[1]",
        Some("I64"),
        Some("Array<I64>"),
        &[
            (RelatedRole::ExpectedType, "Len([][x])+1"),
            (RelatedRole::FoundType, "[1]"),
        ],
    );
    let repaired_right = source.replace("<[1]", "<2");
    assert!(!causes(&repaired_right).iter().any(|cause| cause.2 == "[1]"));
    let positive = "fn Main(){let m0=(Len([[1]][0])+1)<2;}";
    assert!(check_scalar("causal_obligations.n", positive).is_ok());
    assert!(compile_scalar("causal_obligations.n", positive).is_ok());
}

#[test]
fn two_pending_array_domains_keep_distinct_use_sites() {
    // F0-0002 §4.1.2.3–5 N(d): resolving a deferred variable to Bool
    // must reject both independent Array-domain operations.
    let source =
        "fn Main(){var q=(if true then {return;} else {return;});let x=Len(q);let y=q[0];q=true;}";
    assert_eq!(causes(source).len(), 2);
    for index in 0..2 {
        exact_cause(
            source,
            index,
            "NX-E0102",
            Some("not_indexable"),
            "q",
            Some("domain:array"),
            Some("Bool"),
            &[(RelatedRole::FoundType, "q")],
        );
    }
    let repaired = source.replace("q=true", "q=[1]");
    assert!(check_scalar("causal_obligations.n", &repaired).is_ok());
    assert!(compile_scalar("causal_obligations.n", &repaired).is_ok());
    let second_removed = source.replace("let y=q[0];", "");
    assert_eq!(causes(&second_removed).len(), 1);
}
