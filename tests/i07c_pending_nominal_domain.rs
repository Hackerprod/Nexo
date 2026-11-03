use nexo::check::check_proposed;
use nexo::ir::{CompileError, check_scalar};

const DV: &str = "(if true then {return;} else {return;})";

fn source(body: &str) -> String {
    format!("fn Main(){{var q={DV};{body}}}")
}

fn causes(source: &str) -> Vec<(String, Option<String>, String)> {
    check_proposed("pending_nominal.n", source, false)
        .err()
        .unwrap_or_default()
        .iter()
        .map(|error| {
            error.validate("pending_nominal.n", source).unwrap();
            let span = error.primary_span.as_ref().unwrap();
            (
                error.code.clone(),
                error.subcause.clone(),
                source[span.start as usize..span.end as usize].to_owned(),
            )
        })
        .collect()
}

#[test]
fn undecided_domain_preserves_independent_nominal_result() {
    // F0-0002 §4.1.2.3: an undecided predicate is neither a failure nor
    // evidence that its nominal result should disappear at a consumer.
    for (body, primary) in [
        ("let z:I64=(Len(q)<2);", "Len(q)<2"),
        ("let a=(Len(q)==true);", "true"),
        ("let z=((q==q)==1);", "1"),
        ("let z=[1,(Len(q)<2)];", "Len(q)<2"),
    ] {
        let source = source(body);
        assert_eq!(
            causes(&source),
            vec![(
                "NX-E0102".into(),
                Some("type_mismatch".into()),
                primary.into()
            )],
            "{source}"
        );
        assert!(
            matches!(
                check_scalar("pending_nominal.n", &source),
                Err(CompileError::Rejected(_))
            ),
            "{source}"
        );
    }
}

#[test]
fn undecided_domain_positive_controls_stay_accepted() {
    for body in [
        "Print(q);",
        "let z:I64=Len(q);",
        "let z:Bool=(Len(q)<2);",
        "let a=(Len(q)==1);",
        "let z=((q==q)==true);",
        "let z=[true,(Len(q)<2)];",
    ] {
        let source = source(body);
        assert!(causes(&source).is_empty(), "{source}");
        assert!(
            check_scalar("pending_nominal.n", &source).is_ok(),
            "{source}"
        );
    }
}

#[test]
fn undecided_nominal_operation_reaches_independent_consumers() {
    let pending = source("let p=true;AssertEq(Len(q)-7,p);");
    let concrete = "fn Main(){let p=true;let q=[1];AssertEq(Len(q)-7,p);}";
    let expected = vec![("NX-E0102".into(), Some("type_mismatch".into()), "p".into())];
    assert_eq!(causes(&pending), expected);
    assert_eq!(causes(concrete), expected);
    for source in [&pending, concrete] {
        assert!(matches!(
            check_scalar("pending_nominal.n", source),
            Err(CompileError::Rejected(_))
        ));
    }
    for (pending_body, concrete_body, primary) in [
        (
            "let p=true;let a=(if p then true else Len(q)-7);",
            "let p=true;let q=[1];let a=(if p then true else Len(q)-7);",
            "Len(q)-7",
        ),
        (
            "let a=[Len(q)-7,true];",
            "let q=[1];let a=[Len(q)-7,true];",
            "true",
        ),
        (
            "AssertEq(1,(Len(q)<2));",
            "let q=[1];AssertEq(1,(Len(q)<2));",
            "Len(q)<2",
        ),
    ] {
        let pending = source(pending_body);
        let concrete = format!("fn Main(){{{concrete_body}}}");
        let pending_causes = causes(&pending);
        let concrete_causes = causes(&concrete);
        assert_eq!(pending_causes, concrete_causes, "{pending}");
        assert_eq!(pending_causes.len(), 1, "{pending}");
        assert_eq!(pending_causes[0].2, primary, "{pending}");
    }
}

#[test]
fn undecided_domain_inside_numeric_selector_has_executable_type() {
    let pending = "fn G0(a:view I64){} fn Main(){let x=1;let ar=[1];var q=(if true then {return;} else {return;});G0(ar[(x+(Len(q)+x))]);}";
    let concrete =
        "fn G0(a:view I64){} fn Main(){let x=1;let ar=[1];let q=ar;G0(ar[(x+(Len(q)+x))]);}";
    for source in [pending, concrete] {
        assert!(causes(source).is_empty(), "{source}");
        assert!(
            check_scalar("pending_nominal.n", source).is_ok(),
            "{source}"
        );
    }
    let invalid = "fn G4(a:view Array<Array<I64>>){} fn Main(){let ar=[1];var q=(if true then {return;} else {return;});G4(ar[(Len(q)+0)]);}";
    let twin =
        "fn G4(a:view Array<Array<I64>>){} fn Main(){let ar=[1];let q=ar;G4(ar[(Len(q)+0)]);}";
    assert_eq!(causes(invalid), causes(twin));
    assert!(matches!(
        check_scalar("pending_nominal.n", invalid),
        Err(CompileError::Rejected(_))
    ));
}

#[test]
fn nested_assignment_resolves_comparison_inside_array() {
    let pending = "fn Main(){var q=[];let a=[((q == q) == false)];{q=[true];}}";
    let concrete = "fn Main(){let q=[true];let a=[((q == q) == false)];}";
    for source in [pending, concrete] {
        assert!(causes(source).is_empty(), "{source}");
        assert!(
            check_scalar("pending_nominal.n", source).is_ok(),
            "{source}"
        );
    }
    for assignment in ["if true {q=[true];}", "while false {q=[true];}"] {
        let source = format!("fn Main(){{var q=[];let a=[((q == q) == false)];{assignment}}}");
        assert!(causes(&source).is_empty(), "{source}");
        assert!(
            check_scalar("pending_nominal.n", &source).is_ok(),
            "{source}"
        );
    }
}

#[test]
fn failed_first_branch_does_not_anchor_later_branch() {
    let pending = source("let p=true;let a=(if p then [0,(q == q)] else (p));");
    let concrete = "fn Main(){let p=true;let q=[1];let a=(if p then [0,(q == q)] else (p));}";
    let expected = vec![(
        "NX-E0102".into(),
        Some("type_mismatch".into()),
        "q == q".into(),
    )];
    assert_eq!(causes(&pending), expected);
    assert_eq!(causes(concrete), expected);
}

#[test]
fn failed_pending_left_operand_keeps_independent_numeric_right_site() {
    // §4.1.2.4: the projection's selector failure does not consume the
    // enclosing addition's numeric check of its other operand.
    let pending = "fn Main(){let p=true;let ar=[1];var r=[];let a=ar[r]+p;r=[3];}";
    let concrete = "fn Main(){let p=true;let ar=[1];let r=[3];let a=ar[r]+p;}";
    let expected = vec![
        ("NX-E0102".into(), Some("type_mismatch".into()), "r".into()),
        ("NX-E0102".into(), Some("type_mismatch".into()), "p".into()),
    ];
    assert_eq!(causes(pending), expected);
    assert_eq!(causes(concrete), expected);
}

#[test]
fn nested_pending_comparison_compiles_after_domain_resolution() {
    let pending = "fn Main(){let p=true;var r=[];let a=((p == (r == r)) == p);r=[3];}";
    let concrete = "fn Main(){let p=true;let r=[3];let a=((p == (r == r)) == p);}";
    for source in [pending, concrete] {
        assert!(causes(source).is_empty(), "{source}");
        assert!(
            check_scalar("pending_nominal.n", source).is_ok(),
            "{source}"
        );
    }
}

#[test]
fn established_array_selector_conflict_is_independent_of_assignment_order() {
    // §4.1.2.3: `s=[]` establishes an Array root before the indexed use.
    // Completing either array first cannot move I64's conflict to `[true]`.
    let prefix = "fn Main(){var q=[];var s=[];let a=q[0][s];";
    let first = format!("{prefix}q=[[3]];s=[true];}}");
    let second = format!("{prefix}s=[true];q=[[3]];}}");
    let expected = vec![("NX-E0102".into(), Some("type_mismatch".into()), "s".into())];
    assert_eq!(causes(&first), expected);
    assert_eq!(causes(&second), expected);
}

#[test]
fn guarded_right_equality_precedes_operator_domain() {
    // §§4.1.2.4 and 4.6: after the right result becomes Bool, the cross
    // equality fails and the outer `<` has no independent domain cause.
    let pending = "fn Main(){let br=[true];var r=[];let a0=(br < (r == r));r=[3];}";
    let concrete = "fn Main(){let br=[true];let r=[3];let a0=(br < (r == r));}";
    let expected = vec![(
        "NX-E0102".into(),
        Some("type_mismatch".into()),
        "r == r".into(),
    )];
    assert_eq!(causes(pending), expected);
    assert_eq!(causes(concrete), expected);
}

#[test]
fn nested_projection_activates_the_earlier_bool_context() {
    // §4.1.2.3 and RD107: each Array level activates its earlier
    // conditional projection before visiting the later constructor leaf.
    let pending = "fn Main(){var r=[];let a:Bool=r[0][0];r=[[3]];}";
    let valid = "fn Main(){var r=[];let a:I64=r[0][0];r=[[3]];}";
    assert_eq!(
        causes(pending),
        vec![("NX-E0102".into(), Some("type_mismatch".into()), "3".into())]
    );
    assert!(check_scalar("pending_nominal.n", valid).is_ok());
}

#[test]
fn resolved_comparison_inside_array_has_executable_type() {
    // §4.1.2.3: resolving r makes both comparisons Bool before the
    // constructor is lowered. This was InvalidIr with the deferred path.
    let pending = "fn Main(){let p=true;var r=[];let a=[((r == r) == p)];r=[3];}";
    let concrete = "fn Main(){let p=true;let r=[3];let a=[((r == r) == p)];}";
    for source in [pending, concrete] {
        assert!(causes(source).is_empty(), "{source}");
        assert!(
            check_scalar("pending_nominal.n", source).is_ok(),
            "{source}"
        );
    }
}

#[test]
fn nested_selector_causes_are_stable_across_resolution_order() {
    // §4.1.2.3: the selector context and the explicit array assignment
    // each retain their own C in both source orders.
    let prefix = "fn H1(a:view Array<I64>)->Bool{return false;} fn Main(){var r=[];var q=[];let a=q[0][H1(r)];";
    let first = format!("{prefix}r=[true];q=[[3]];}}",);
    let second = format!("{prefix}q=[[3]];r=[true];}}",);
    for source in [&first, &second] {
        assert_eq!(
            causes(source),
            vec![
                (
                    "NX-E0102".into(),
                    Some("type_mismatch".into()),
                    "H1(r)".into()
                ),
                (
                    "NX-E0102".into(),
                    Some("type_mismatch".into()),
                    "true".into()
                ),
            ],
            "{source}"
        );
    }
}

#[test]
fn array_keeps_first_witness_after_independent_element_conflict() {
    // §§4.1.2.4–5: a failed second element cannot replace the first
    // r[x] witness or suppress the third element's independent equality.
    let pending = "fn Main(){let x=1;let ar=[1];var r=[];let a=[r[x],(ar == r),ar];r=[3];}";
    let concrete = "fn Main(){let x=1;let ar=[1];let r=[3];let a=[r[x],(ar == r),ar];}";
    let expected = vec![
        (
            "NX-E0102".into(),
            Some("type_mismatch".into()),
            "ar == r".into(),
        ),
        ("NX-E0102".into(), Some("type_mismatch".into()), "ar".into()),
    ];
    assert_eq!(causes(pending), expected);
    assert_eq!(causes(concrete), expected);
}

#[test]
fn resolved_nested_operations_reject_with_types_diagnostics() {
    // §4.1.2.3–5: a resolved comparison inside an Array condition and a
    // rejected Len domain have the same C as their concrete counterparts.
    // Neither reaches lowering with an unresolved checker type.
    for (pending, concrete, expected) in [
        (
            "fn Main(){var r=[];while [((r == r) == (r == r))] {}r=[3];}",
            "fn Main(){let r=[3];while [((r == r) == (r == r))] {}}",
            ("NX-E0102", "type_mismatch", "[((r == r) == (r == r))]"),
        ),
        (
            "fn Main(){let x=1;var r=[];let a1=Len((r[x] == (r == r)));r=[true];}",
            "fn Main(){let x=1;let r=[true];let a1=Len((r[x] == (r == r)));}",
            ("NX-E0102", "not_indexable", "r[x] == (r == r)"),
        ),
    ] {
        let signature = vec![(
            expected.0.to_owned(),
            Some(expected.1.to_owned()),
            expected.2.to_owned(),
        )];
        assert_eq!(causes(pending), signature);
        assert_eq!(causes(concrete), signature);
        for source in [pending, concrete] {
            assert!(matches!(
                check_scalar("pending_nominal.n", source),
                Err(CompileError::Rejected(_))
            ));
        }
    }
}

#[test]
fn later_array_resolution_preserves_nested_comparison_cause() {
    // F0-0002 §4.1.2.4: a failed equality cannot also report the
    // comparison's domain; the same rule applies with a later constructor.
    for (pending, concrete) in [
        (
            "fn Main(){var r=[];let a=((r == r) < r);r=[3];}",
            "fn Main(){let r=[3];let a=((r == r) < r);}",
        ),
        (
            "fn Main(){var r=[];let a=(((r == r) < r) + r);r=[3];}",
            "fn Main(){let r=[3];let a=(((r == r) < r) + r);}",
        ),
        (
            "fn Main(){var r=[];let a=(r < (r == r));r=[3];}",
            "fn Main(){let r=[3];let a=(r < (r == r));}",
        ),
        (
            "fn Main(){var r=[];Print(((r == r) < r));let a=(if r then 1 else r);r=[3];}",
            "fn Main(){let r=[3];Print(((r == r) < r));let a=(if r then 1 else r);}",
        ),
        (
            "fn G(a:view Bool){} fn Main(){let x=1;var r=[];G([r[x],r,(r == r)]);r=[true];}",
            "fn G(a:view Bool){} fn Main(){let x=1;let r=[true];G([r[x],r,(r == r)]);}",
        ),
        (
            "fn Main(){let x=1;var r=[];var i:Array<Array<Bool>>=[[false]];i[0]=((if r then false else r)==x);let a=((r==r)<r);r=[3];}",
            "fn Main(){let x=1;let r=[3];var i:Array<Array<Bool>>=[[false]];i[0]=((if r then false else r)==x);let a=((r==r)<r);}",
        ),
    ] {
        assert_eq!(causes(pending), causes(concrete), "{pending}");
    }
}

#[test]
fn recursive_index_assignment_keeps_earlier_origin() {
    // §4.1.2.3/.6, RD89: the destination cannot supply the expected-type
    // witness for its own unresolved right-hand occurrence.
    let source = "fn Main(){var r=[];r[0]=r;r=[[3]];}";
    let errors = check_proposed("pending_nominal.n", source, false).unwrap_err();
    assert_eq!(errors.len(), 1, "{errors:?}");
    assert!(
        errors[0]
            .related_spans
            .iter()
            .all(|r| { r.role != nexo::diagnostic::RelatedRole::ExpectedType })
    );
}

#[test]
fn projected_array_siblings_retain_resolved_comparison_domain() {
    let pending = "fn G(a:view Bool){} fn Main(){let x=1;var r=[];G([r[x],r,(r == r)]);r=[true];}";
    let concrete = "fn G(a:view Bool){} fn Main(){let x=1;let r=[true];G([r[x],r,(r == r)]);}";
    assert_eq!(causes(pending), causes(concrete));
}

#[test]
fn multiple_later_arrays_keep_earlier_comparison_equality() {
    // §§4.1.2.4–5,7: q<r unites two pending arrays before their
    // incompatible assignments. Moving both constructors to initializers
    // violates §4.1.2.7, so the two sources have different causal sites.
    let pending =
        "fn Main(){var r=[];var q=[];if (q < r) {}let a=(if q then r else false);r=[3];q=[true];}";
    let concrete = "fn Main(){let r=[3];let q=[true];if (q < r) {}let a=(if q then r else false);}";
    let mismatch = |text: &str| ("NX-E0102".into(), Some("type_mismatch".into()), text.into());
    assert_eq!(
        causes(pending),
        vec![
            mismatch("<"),
            mismatch("q"),
            ("NX-E1303".into(), None, "false".into()),
            mismatch("true")
        ]
    );
    assert_eq!(
        causes(concrete),
        vec![
            mismatch("r"),
            mismatch("q"),
            ("NX-E1303".into(), None, "false".into())
        ]
    );
}

#[test]
fn distinct_assignment_constructors_survive_later_branch_joins() {
    // §§4.1.2.4–5,7: each pending equality precedes incompatible later
    // constructors. The concrete initializers change that earlier context;
    // the deferred source diagnoses its later incompatible assignment.
    let mismatch = |text: &str| ("NX-E0102".into(), Some("type_mismatch".into()), text.into());
    for (pending, concrete, pending_expected, concrete_expected) in [
        (
            "fn Main(){let p=true;var r=[];var q=[];let a=(q<r);let b=(if q then {value:r;} else {value:p;});r=[3];q=[true];}",
            "fn Main(){let p=true;let r=[3];let q=[true];let a=(q<r);let b=(if q then {value:r;} else {value:p;});}",
            vec![
                mismatch("<"),
                mismatch("q"),
                ("NX-E1303".into(), None, "p".into()),
                mismatch("true"),
            ],
            vec![
                mismatch("r"),
                mismatch("q"),
                ("NX-E1303".into(), None, "p".into()),
            ],
        ),
        (
            "fn Main(){let p=true;let ar=[1];var r=[];var q=[];Print(r==q);let a=(if p then {value:Len(ar);} else {value:q;});r=[3];q=[true];}",
            "fn Main(){let p=true;let ar=[1];let r=[3];let q=[true];Print(r==q);let a=(if p then {value:Len(ar);} else {value:q;});}",
            vec![("NX-E1303".into(), None, "q".into()), mismatch("true")],
            vec![mismatch("q"), ("NX-E1303".into(), None, "q".into())],
        ),
    ] {
        assert_eq!(causes(pending), pending_expected, "{pending}");
        assert_eq!(causes(concrete), concrete_expected, "{concrete}");
    }
}

#[test]
fn incompatible_array_assignments_blame_the_later_source() {
    // §§4.1.2.4–5,7: r==q fixes a shared pending component. These
    // assignments are incompatible, so swapping them changes which later
    // source owns the mismatch; the branch C remains at q in both orders.
    let prefix = "fn Main(){let p=true;let ar=[1];var r=[];var q=[];Print(r==q);let a=(if p then {value:Len(ar);} else {value:q;});";
    let first = format!("{prefix}r=[3];q=[[3]];}}");
    let second = format!("{prefix}q=[[3]];r=[3];}}");
    let branch = ("NX-E1303".into(), None, "q".into());
    assert_eq!(
        causes(&first),
        vec![
            branch.clone(),
            (
                "NX-E0102".into(),
                Some("type_mismatch".into()),
                "[3]".into()
            )
        ]
    );
    assert_eq!(
        causes(&second),
        vec![
            branch,
            ("NX-E0102".into(), Some("type_mismatch".into()), "3".into())
        ]
    );
}

#[test]
fn immutable_alias_does_not_block_later_constructor_resolution() {
    // §§4.1.2.4–5: an alias without an independent assignment does not
    // withhold the constructor's decided type from a later comparison.
    let pending = "fn Main(){let x=1;var r=[];let a0=r;AssertEq(x,(r<(r==r)));r=[3];}";
    let concrete = "fn Main(){let x=1;let r=[3];let a0=r;AssertEq(x,(r<(r==r)));}";
    assert_eq!(causes(pending), causes(concrete));

    // §4.1.2.7: [ar,r] fixes a different element type before r=[true].
    // Moving [true] to the initializer is therefore not a valid twin.
    let pending = "fn Main(){let ar=[1];let aar=[[1]];var r=[];AssertEq((if aar then r else 1),(r<(r==r)));if [ar,r] {}r=[true];}";
    let concrete = "fn Main(){let ar=[1];let aar=[[1]];let r=[true];AssertEq((if aar then r else 1),(r<(r==r)));if [ar,r] {}}";
    let primary = |code: &str, text: &str| {
        (
            code.into(),
            (code == "NX-E0102").then(|| "type_mismatch".into()),
            text.into(),
        )
    };
    assert_eq!(
        causes(pending),
        vec![
            primary("NX-E0102", "aar"),
            primary("NX-E1303", "1"),
            primary("NX-E0102", "r==r"),
            primary("NX-E0102", "[ar,r]"),
            primary("NX-E0102", "true"),
        ]
    );
    assert_eq!(
        causes(concrete),
        vec![
            primary("NX-E0102", "aar"),
            primary("NX-E1303", "1"),
            primary("NX-E0102", "r==r"),
            primary("NX-E0102", "r"),
        ]
    );
}

#[test]
fn nested_branch_array_keeps_only_independent_union_causes() {
    // §§4.1.2.4–5,7: the earlier branch fixes r's array shape before
    // r=[true]; the concrete initializer changes that prefix context.
    let pending = "fn Main(){let p=true;let ar=[1];var r=[];var q=[];if [(if ar then {value:Len(ar);} else {value:(if p then (if true then {return;} else {return;}) else r);}),(if r then {value:r;} else {value:ar;})] {}let a1=q;r=[true];q=[3];}";
    let concrete = "fn Main(){let p=true;let ar=[1];let r=[true];let q=[3];if [(if ar then {value:Len(ar);} else {value:(if p then (if true then {return;} else {return;}) else r);}),(if r then {value:r;} else {value:ar;})] {}let a1=q;}";
    let primary = |code: &str, text: &str| {
        (
            code.into(),
            (code == "NX-E0102").then(|| "type_mismatch".into()),
            text.into(),
        )
    };
    assert_eq!(
        causes(pending),
        vec![
            primary("NX-E0102", "ar"),
            primary("NX-E1303", "r"),
            primary("NX-E0102", "r"),
            primary("NX-E0102", "true"),
        ]
    );
    assert_eq!(
        causes(concrete),
        vec![
            primary("NX-E0102", "ar"),
            primary("NX-E1303", "r"),
            primary("NX-E0102", "r"),
            primary("NX-E1303", "ar"),
        ]
    );
}

#[test]
fn resolved_condition_failure_cannot_anchor_array_homogeneity() {
    // ADR §4.1.2.5: an if whose condition fails has no type for its
    // enclosing array. The later [r] cannot inherit its partial witness.
    let pending = "fn Main(){var r=[];let a=[(if r then r else r),[r]];r=[3];}";
    let concrete = "fn Main(){let r=[3];let a=[(if r then r else r),[r]];}";
    let expected = vec![("NX-E0102".into(), Some("type_mismatch".into()), "r".into())];
    assert_eq!(causes(pending), expected);
    assert_eq!(causes(concrete), expected);
    assert!(matches!(
        check_scalar("pending_nominal.n", pending),
        Err(CompileError::Rejected(_))
    ));
}

#[test]
fn late_numeric_domains_keep_one_failure_per_operation() {
    // ADR §§4.1.2 N(d), 4.6: both operands of r-r are Array<I64>, but
    // the operation reports its first incompatible operand only. The
    // separate nested selector still owns its not_indexable cause.
    let pending = "fn Main(){let x=1;var r=[];let a=r[x][(r-r)];r=[3];}";
    let concrete = "fn Main(){let x=1;let r=[3];let a=r[x][(r-r)];}";
    let expected = vec![
        (
            "NX-E0102".into(),
            Some("not_indexable".into()),
            "r[x]".into(),
        ),
        ("NX-E0102".into(), Some("type_mismatch".into()), "r".into()),
    ];
    assert_eq!(causes(pending), expected);
    assert_eq!(causes(concrete), expected);
}

#[test]
fn planned_array_shape_preserves_branch_type_mismatch() {
    // ADR §§4.1.2.2–5: a planned Array shape with an unresolved leaf
    // cannot turn an Array-vs-Bool condition into not_indexable. Once
    // the leaf resolves, the independent branch join still reports E1303.
    let pending = "fn Main(){var r=[];var s=[];let a0=(r==[s]);let a1=(if r then {value:[true];} else {value:r;});r=[[3]];s=[3];}";
    let concrete = "fn Main(){let r=[[3]];let s=[3];let a0=(r==[s]);let a1=(if r then {value:[true];} else {value:r;});}";
    let expected = vec![
        ("NX-E0102".into(), Some("type_mismatch".into()), "r".into()),
        ("NX-E1303".into(), None, "r".into()),
    ];
    assert_eq!(causes(pending), expected);
    assert_eq!(causes(concrete), expected);
    assert!(matches!(
        check_scalar("pending_nominal.n", pending),
        Err(CompileError::Rejected(_))
    ));
}

#[test]
fn nested_array_join_reports_source_mismatch_after_later_resolution() {
    // ADR §§4.1.2.2–5, 13.15: the nested Array child of r is an
    // incomplete constructor, not an indexable-domain operand. The join
    // with br has its own E1303 on the else value after r resolves.
    let prefix = "fn Main(){let x=1;let p=true;let ar=[1];let br=[true];let aar=[[1]];";
    let pending = format!(
        "{prefix}var r=[];var q=[];var s=[];let a0=[(r == [s]),(s < r)];AssertEq(1,[(if r then {{value: br;}} else {{value: r;}}),(if q then 1 else true)]);r=[[3]];q=[3];s=[3];}}"
    );
    let concrete = format!(
        "{prefix}let r=[[3]];let q=[3];let s=[3];let a0=[(r == [s]),(s < r)];AssertEq(1,[(if r then {{value: br;}} else {{value: r;}}),(if q then 1 else true)]);}}"
    );
    let expected = vec![
        ("NX-E0102".into(), Some("type_mismatch".into()), "r".into()),
        ("NX-E0102".into(), Some("type_mismatch".into()), "r".into()),
        ("NX-E1303".into(), None, "r".into()),
        ("NX-E0102".into(), Some("type_mismatch".into()), "q".into()),
        ("NX-E1303".into(), None, "true".into()),
    ];
    assert_eq!(causes(&pending), expected);
    assert_eq!(causes(&concrete), expected);
}

#[test]
fn planned_projection_child_does_not_use_failed_branch_as_witness() {
    use nexo::diagnostic::DataValue;

    // ADR §§4.1.2.2–5, 4.7: q's own later assignment supplies Array<Bool>
    // before the first if is resolved. That branch fails on q; its nominal
    // Bool cannot then make the sibling `true` an independent mismatch.
    let source = "fn Main(){let x=1;let p=true;let ar=[1];let br=[true];let aar=[[1]];var r=[];var q=[];var s=[];var i0:Array<Array<Array<I64>>>=[[[0]]];i0[0]=([s[x],(if 0 then true else q),(if x then q else s)] == x);let a1=s;while Len(q) {}r=br;q=br;s=[[true]];}";
    let at = (source.find("true else q").unwrap() + "true else ".len()) as u64;
    let diagnostics = check_proposed("pending_nominal.n", source, false).unwrap_err();
    let failure = diagnostics
        .iter()
        .find(|d| d.primary_span.as_ref().is_some_and(|span| span.start == at))
        .expect("the first branch owns its equality failure");
    assert_eq!(
        (failure.code.as_str(), failure.subcause.as_deref()),
        ("NX-E1303", None)
    );
    assert_eq!(
        failure
            .primary_span
            .as_ref()
            .map(|span| (span.start, span.end)),
        Some((at, at + 1))
    );
    assert_eq!(
        failure.data.get("expected_type"),
        Some(&DataValue::String("Bool".into()))
    );
    assert_eq!(
        failure.data.get("found_type"),
        Some(&DataValue::String("Array<Bool>".into()))
    );
    assert!(
        diagnostics
            .iter()
            .all(|d| d.subcause.as_deref() != Some("not_indexable"))
    );
    failure.validate("pending_nominal.n", source).unwrap();
}

#[test]
fn cross_type_failure_precedes_less_than_domain_after_resolution() {
    // ADR §4.6: when Bool and Array<I64> differ, the right operand owns
    // the comparison's sole C. The `<` domain is not a second site.
    let deferred = "fn Main(){let x=1;var r=[];let a=r[x][(false<r)];r=[3];}";
    let concrete = "fn Main(){let x=1;let r=[3];let a=r[x][(false<r)];}";
    let expected = vec![
        (
            "NX-E0102".into(),
            Some("not_indexable".into()),
            "r[x]".into(),
        ),
        ("NX-E0102".into(), Some("type_mismatch".into()), "r".into()),
    ];
    assert_eq!(causes(deferred), expected);
    assert_eq!(causes(concrete), expected);

    let deferred = "fn Main(){var r=[];let a=((r<(r==r))==(r==[r]));r=[3];}";
    let concrete = "fn Main(){let r=[3];let a=((r<(r==r))==(r==[r]));}";
    let expected = vec![
        (
            "NX-E0102".into(),
            Some("type_mismatch".into()),
            "r==r".into(),
        ),
        (
            "NX-E0102".into(),
            Some("type_mismatch".into()),
            "[r]".into(),
        ),
    ];
    assert_eq!(causes(deferred), expected);
    assert_eq!(causes(concrete), expected);
}

#[test]
fn pending_len_result_cannot_anchor_branch_during_shape_planning() {
    // ADR §§4.1.2.3, 4.1.2.5, 4.7: Len(r[x]) has a nominal I64 result,
    // but its Array domain is undecided until r's later constructor.
    // Planning cannot bind r[0] to that nominal result first.
    let deferred = "fn Main(){let x=1;var r=[];Print((if true then {value: Len(r[x]);} else {value: r[0];}));r=[[3]];}";
    let concrete = "fn Main(){let x=1;let r=[[3]];Print((if true then {value: Len(r[x]);} else {value: r[0];}));}";
    let expected = vec![("NX-E1303".into(), None, "r[0]".into())];
    assert_eq!(causes(deferred), expected);
    assert_eq!(causes(concrete), expected);
}

#[test]
fn first_accepted_assignment_owns_shared_pending_component() {
    // ADR §4.1.2.7: q and r have an accepted equality before either
    // assignment. The first constructor fixes their shared type; the
    // incompatible later assignment owns the mismatch in each order.
    let prefix = "fn Main(){var r=[];var q=[];let a=(if true then q else r);";
    let first_r = format!("{prefix}r=[[3]];q=[3];}}");
    let first_q = format!("{prefix}q=[3];r=[[3]];}}");
    assert_eq!(
        causes(&first_r),
        vec![("NX-E0102".into(), Some("type_mismatch".into()), "3".into())]
    );
    assert_eq!(
        causes(&first_q),
        vec![(
            "NX-E0102".into(),
            Some("type_mismatch".into()),
            "[3]".into()
        )]
    );
}

#[test]
fn nested_array_shape_survives_later_incompatible_assignment_order() {
    use nexo::diagnostic::DataValue;

    // ADR §4.1.2.2: r==[q] establishes two Array layers for r because q
    // already has its own [] constructor. The later flat [3] fails at its
    // leaf regardless of whether q's compatible assignment comes first.
    for source in [
        "fn Main(){var r=[];var q=[];let a=(r==[q]);r=[3];q=[3];}",
        "fn Main(){var r=[];var q=[];let a=(r==[q]);q=[3];r=[3];}",
    ] {
        let diagnostics = check_proposed("pending_nominal.n", source, false).unwrap_err();
        assert_eq!(diagnostics.len(), 1, "{source}: {diagnostics:?}");
        let failure = &diagnostics[0];
        let at = (source.rfind("r=[3]").unwrap() + "r=[".len()) as u64;
        assert_eq!(
            (failure.code.as_str(), failure.subcause.as_deref()),
            ("NX-E0102", Some("type_mismatch"))
        );
        assert_eq!(
            failure
                .primary_span
                .as_ref()
                .map(|span| (span.start, span.end)),
            Some((at, at + 1))
        );
        assert_eq!(
            failure.data.get("expected_type"),
            Some(&DataValue::String("Array<I64>".into()))
        );
        assert_eq!(
            failure.data.get("found_type"),
            Some(&DataValue::String("I64".into()))
        );
        failure.validate("pending_nominal.n", source).unwrap();
    }
}

#[test]
fn independent_selector_constraints_precede_later_assignment_order() {
    // ADR §§4.1.2.3, 4.1.2.7: the selector of a hypothetical projection
    // has its own branch constraints without waiting for that projection's
    // Array domain. Its valid r=ar equality fixes r before either assignment.
    let prefix = "fn Main(){let ar=[1];var r=[];var q=[];Print(q[0][(if q then r else ar)]);";
    let expected = vec![
        (
            "NX-E0102".into(),
            Some("not_indexable".into()),
            "q[0]".into(),
        ),
        ("NX-E0102".into(), Some("type_mismatch".into()), "q".into()),
        (
            "NX-E0102".into(),
            Some("type_mismatch".into()),
            "true".into(),
        ),
    ];
    for source in [
        format!("{prefix}r=[true];q=[3];}}"),
        format!("{prefix}q=[3];r=[true];}}"),
    ] {
        assert_eq!(causes(&source), expected, "{source}");
    }
}

#[test]
fn incompatible_prefix_with_unresolved_arrays_is_not_a_concrete_twin() {
    // ADR §§4.1.2.2, 4.1.2.7: [[r],q] requires a deeper q than q=[3]
    // supplies. Moving both assignments to initializers changes that
    // prefix, so the deferred constructors may retain inference causes
    // where the concrete source has a branch mismatch.
    let deferred = "fn Main(){let x=1;let p=true;var r=[];var q=[];while [[r],q] {}q=[3];let a=(1<(if x then p else q));r=[3];}";
    let concrete = "fn Main(){let x=1;let p=true;let r=[3];let q=[3];while [[r],q] {}let a=(1<(if x then p else q));}";
    let mismatch = |text: &str| ("NX-E0102".into(), Some("type_mismatch".into()), text.into());
    assert_eq!(
        causes(deferred),
        vec![
            ("NX-E0103".into(), None, "[]".into()),
            ("NX-E0103".into(), None, "[]".into()),
            mismatch("q"),
            mismatch("x")
        ]
    );
    assert_eq!(
        causes(concrete),
        vec![
            mismatch("q"),
            mismatch("x"),
            ("NX-E1303".into(), None, "q".into())
        ]
    );
}

#[test]
fn branch_with_existing_array_element_context_uses_concrete_mismatch() {
    use nexo::diagnostic::DataValue;

    // ADR §§4.1.2.4, 4.7: r[x] establishes the Array element I64
    // context before the branch's second value. The compatible later
    // assignment only completes r; it cannot turn the context mismatch
    // into an uncontextualized branch join.
    for source in [
        "fn Main(){var r=[];let x=1;let a=[r[x],(if true then r[x] else r)];r=[3];}",
        "fn Main(){let r=[3];let x=1;let a=[r[x],(if true then r[x] else r)];}",
    ] {
        let errors = check_proposed("pending_nominal.n", source, false).unwrap_err();
        assert_eq!(errors.len(), 1, "{source}: {errors:?}");
        let error = &errors[0];
        let at = source.find("else r").unwrap() as u64 + "else ".len() as u64;
        assert_eq!(
            (error.code.as_str(), error.subcause.as_deref()),
            ("NX-E0102", Some("type_mismatch"))
        );
        assert_eq!(
            error
                .primary_span
                .as_ref()
                .map(|span| (span.start, span.end)),
            Some((at, at + 1))
        );
        assert_eq!(
            error.data.get("expected_type"),
            Some(&DataValue::String("I64".into()))
        );
        assert_eq!(
            error.data.get("found_type"),
            Some(&DataValue::String("Array<I64>".into()))
        );
        error.validate("pending_nominal.n", source).unwrap();
    }
}

#[test]
fn failed_nested_union_does_not_anchor_an_outer_pending_array() {
    // ADR §§4.1.2.4–5, 4.7: the inner [[q]]/[[1]] join fails by shape.
    // Its first branch cannot establish r's type through the outer anchor;
    // the two later assignments are compatible with their own constructors.
    let prefix =
        "fn Main(){var r=[];var q=[];let a=(if true then r else (if false then [[q]] else [[1]]));";
    for source in [
        format!("{prefix}r=[3];q=[3];}}"),
        format!("{prefix}q=[3];r=[3];}}"),
    ] {
        assert_eq!(
            causes(&source),
            vec![("NX-E1303".into(), None, "[[q]]".into())],
            "{source}"
        );
    }
    let concrete = "fn Main(){let r=[3];let q=[3];let a=(if true then r else (if false then [[q]] else [[1]]));}";
    assert_eq!(
        causes(concrete),
        vec![("NX-E1303".into(), None, "[[q]]".into())]
    );
}

#[test]
fn outer_index_context_makes_later_flat_array_an_ineligible_twin() {
    // ADR §§4.1.2.2, 4.1.2.7, 4.7: the annotated result of the Index
    // supplies Array<Array<I64>> to the explicit array's elements. That
    // accepted prefix is deeper than the later r=[3]. Moving [3] to r's
    // initializer changes the available context and does not preserve C.
    let deferred = "fn Main(){let x=1;var r=[];var s:Array<Array<I64>>=[[0]];s=[r[1],r][x];r=[3];}";
    let concrete = "fn Main(){let x=1;let r=[3];var s:Array<Array<I64>>=[[0]];s=[r[1],r][x];}";
    assert_eq!(
        causes(deferred),
        vec![
            ("NX-E0103".into(), None, "[]".into()),
            ("NX-E0103".into(), None, "[r[1],r][x]".into())
        ]
    );
    assert_eq!(
        causes(concrete),
        vec![("NX-E0102".into(), Some("type_mismatch".into()), "r".into())]
    );
}

#[test]
fn undecided_right_domain_preserves_comparison_result_for_consumers() {
    // F0-0002 §§4.1.2.3–5: Len's unresolved Array domain does not erase
    // the Bool result of the containing comparison. A refuted domain does
    // erase that result before the dependent equality is diagnosed.
    let rejected = "fn Main(){let ar=[1];var q=(if true then {return;} else {return;});AssertEq((1<Len(q)),ar);}";
    assert_eq!(
        causes(rejected),
        vec![("NX-E0102".into(), Some("type_mismatch".into()), "ar".into())]
    );
    assert!(matches!(
        nexo::ir::check_scalar("pending_nominal.n", rejected),
        Err(nexo::ir::CompileError::Rejected(_))
    ));
    let accepted = "fn Main(){let ar=[1];var q=(if true then {return;} else {return;});AssertEq((1<Len(q)),true);}";
    assert!(causes(accepted).is_empty());
    assert!(nexo::ir::check_scalar("pending_nominal.n", accepted).is_ok());
    let refuted = "fn Main(){let ar=[1];var q=(if true then {return;} else {return;});AssertEq((1<Len(q)),ar);q=1;}";
    assert_eq!(
        causes(refuted),
        vec![("NX-E0102".into(), Some("not_indexable".into()), "q".into())]
    );

    let reversed = "fn Main(){let ar=[1];var q=(if true then {return;} else {return;});AssertEq(ar,(1<Len(q)));}";
    assert_eq!(
        causes(reversed),
        vec![(
            "NX-E0102".into(),
            Some("type_mismatch".into()),
            "1<Len(q)".into()
        )]
    );
    let joined = "fn Main(){let x=1;var r=(if true then {return;} else {return;});let y=(if true then 1 else (x<Len(r)));}";
    assert_eq!(
        causes(joined),
        vec![("NX-E1303".into(), None, "x<Len(r)".into())]
    );
}
