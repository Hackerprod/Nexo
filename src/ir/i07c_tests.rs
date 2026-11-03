use super::{CompileError, check_scalar, compile_scalar};
use crate::diagnostic::{CauseKind, CauseOperation, Evidence, Operation, RelatedRole, Stage};

#[test]
fn deferred_array_equality_uses_complete_right_operand_type() {
    for source in [
        "fn Main(){let ar=[1];var r=[];let a=ar==[r,ar];r=[3];}",
        "fn Main(){let ar=[1];let r=[3];let a=ar==[r,ar];}",
    ] {
        let ds = rejected(source);
        assert_eq!(ds.len(), 1, "{source}: {ds:?}");
        let right = source.find("[r,ar]").unwrap() as u64;
        let left = source.find("ar==").unwrap() as u64;
        exact_type_site(
            &ds[0],
            source,
            "NX-E0102",
            (right, right + 6),
            "Array<I64>",
            "Array<Array<I64>>",
            Some((left, left + 2)),
        );
        assert!(check_scalar("fixture.n", source).is_err());
    }
    compile_scalar(
        "fixture.n",
        "fn Main(){let ar=[1];var r=[];let a=ar==r;r=[3];}",
    )
    .unwrap();
}

#[test]
fn failed_numeric_branch_does_not_supply_a_union_type() {
    for source in [
        "fn Main(){let p=true;var r=[];let a=(if p then r else 1-r);r=[3];}",
        "fn Main(){let p=true;let r=[3];let a=(if p then r else 1-r);}",
    ] {
        let ds = rejected(source);
        assert_eq!(ds.len(), 1, "{source}: {ds:?}");
        let right = source.find("1-r").unwrap() as u64 + 2;
        exact_type_site(
            &ds[0],
            source,
            "NX-E0102",
            (right, right + 1),
            "I64",
            "Array<I64>",
            None,
        );
        assert!(check_scalar("fixture.n", source).is_err());
    }
    compile_scalar(
        "fixture.n",
        "fn Main(){let p=true;let r=3;let a=(if p then r else 1-r);}",
    )
    .unwrap();
}

#[test]
fn deferred_array_element_domain_result_resolves_leaf_context() {
    for source in [
        "fn Main(){var r=[];let a=[r,[r==r]];r=[3];}",
        "fn Main(){let r=[3];let a=[r,[r==r]];}",
    ] {
        let ds = rejected(source);
        assert_eq!(ds.len(), 1, "{source}: {ds:?}");
        let bad = source.find("r==r").unwrap() as u64;
        let witness = source.find("[r,").unwrap() as u64 + 1;
        exact_type_site(
            &ds[0],
            source,
            "NX-E0102",
            (bad, bad + 4),
            "I64",
            "Bool",
            Some((witness, witness + 1)),
        );
        assert!(check_scalar("fixture.n", source).is_err());
    }
    compile_scalar("fixture.n", "fn Main(){var r=[];let a=[r,[1]];r=[3];}").unwrap();
}

#[test]
fn valid_later_element_witness_preserves_independent_site() {
    for source in [
        "fn Main(){let x=1;var r=[];let a=[r<r,x<x,r];r=[[true]];}",
        "fn Main(){let x=1;let r=[[true]];let a=[r<r,x<x,r];}",
    ] {
        let ds = rejected(source);
        assert_eq!(ds.len(), 2, "{source}: {ds:?}");
        let operator = source.find("r<r").unwrap() as u64 + 1;
        assert_eq!(
            (ds[0].code.as_str(), ds[0].subcause.as_deref()),
            ("NX-E0102", Some("type_mismatch"))
        );
        assert_eq!(
            ds[0].primary_span.as_ref().map(|s| (s.start, s.end)),
            Some((operator, operator + 1))
        );
        let bad = source.find(",r]").unwrap() as u64 + 1;
        let witness = source.find("x<x").unwrap() as u64;
        exact_type_site(
            &ds[1],
            source,
            "NX-E0102",
            (bad, bad + 1),
            "Bool",
            "Array<Array<Bool>>",
            Some((witness, witness + 3)),
        );
        assert!(check_scalar("fixture.n", source).is_err());
    }
    compile_scalar(
        "fixture.n",
        "fn Main(){let x=1;let r=3;let a=[r<r,x<x,true];}",
    )
    .unwrap();
}

#[test]
fn array_wrapping_keeps_independent_invalid_index_base() {
    let source = "fn Main(){let v3=false;let a=[v3[(if true then 1 else v3)]];}";
    let ds = rejected(source);
    assert_eq!(ds.len(), 2, "{ds:?}");
    let base = source.find("v3[(").unwrap() as u64;
    assert_eq!(
        (ds[0].code.as_str(), ds[0].subcause.as_deref()),
        ("NX-E0102", Some("not_indexable"))
    );
    assert_eq!(
        ds[0].primary_span.as_ref().map(|s| (s.start, s.end)),
        Some((base, base + 2))
    );
    let leaf = source.rfind("v3").unwrap() as u64;
    assert_eq!(ds[1].code, "NX-E1303");
    assert_eq!(
        ds[1].primary_span.as_ref().map(|s| (s.start, s.end)),
        Some((leaf, leaf + 2))
    );
    assert!(check_scalar("fixture.n", source).is_err());
    compile_scalar(
        "fixture.n",
        "fn Main(){let v3=[1];let a=[v3[(if true then 1 else 0)]];}",
    )
    .unwrap();
}

#[test]
fn adr_gap_never_index_projection_keeps_current_resolution() {
    // ADR §4.1.2.3 requires a conditional projection when the base has no
    // Array constructor; it does not determine the result type of Never[i]
    // or [Never][i]. Freeze the present acceptance and constructor site.
    let dv = "(if true then {return;} else {return;})";
    let direct = format!("fn Main(){{let a={dv}[0];}}");
    let contextual = format!("fn Main(){{let c=true;let a=(if c then [{dv}][0] else 1);}}");
    compile_scalar("fixture.n", &direct).unwrap();
    compile_scalar("fixture.n", &contextual).unwrap();
    let unresolved = format!("fn Main(){{let a=[{dv}][0];}}");
    let ds = rejected(&unresolved);
    assert_eq!(ds.len(), 1, "{ds:?}");
    assert_eq!(ds[0].code, "NX-E0103");
    let start = unresolved.find("[(if").unwrap() as u64;
    assert_eq!(
        ds[0].primary_span.as_ref().map(|s| (s.start, s.end)),
        Some((start, start + dv.len() as u64 + 2))
    );
}

#[test]
fn adr_gap_resolved_variable_within_element_keeps_current_union_site() {
    // ADR §4.1.2.4–5 specifies leaves under an external concrete context;
    // this intra-element resolution has no prescribed leaf cardinality.
    let source = "fn Main(){let x=1;var q=(if true then {return;} else {return;});let a=[q,(if q then 0-x else 2-3)];q=true;}";
    let ds = rejected(source);
    assert_eq!(ds.len(), 1, "{ds:?}");
    let start = source.find("if q then").unwrap() as u64;
    assert_eq!(
        (ds[0].code.as_str(), ds[0].subcause.as_deref()),
        ("NX-E0102", Some("type_mismatch"))
    );
    assert_eq!(
        ds[0].primary_span.as_ref().map(|s| (s.start, s.end)),
        Some((start, start + 22))
    );
    compile_scalar(
        "fixture.n",
        "fn Main(){let x=1;let q=1;let a=[q,(if true then 0-x else 2-3)];}",
    )
    .unwrap();
}

#[test]
fn empty_array_resolves_from_seven_later_context_forms() {
    // ADR §4.1.1/TD and §4.1.2.2: each later compatible constraint
    // determines the element of [] at its own source site.
    type Site = (&'static str, Option<&'static str>, u64, u64);
    let pairs: &[(&str, &str, &[Site])] = &[
        (
            "fn Main(){Print([]==[48]);}",
            "fn Main(){Print([]==true);}",
            &[("NX-E0103", None, 16, 18)],
        ),
        (
            "fn Main(){var q=[];let r:Array<Bool>=q;}",
            "fn Main(){var q=[];let r:Bool=q;}",
            &[("NX-E0103", None, 16, 18)],
        ),
        (
            "fn Main(){var q=(if true then [] else [false]);}",
            "fn Main(){var q=(if true then [] else true);}",
            &[("NX-E0103", None, 30, 32)],
        ),
        (
            "fn Main(){let q=(if true then {value:[67];} else {value:[];});}",
            "fn Main(){let q=(if true then {value:[67];} else {value:true;});}",
            &[("NX-E1303", None, 56, 60)],
        ),
        (
            "fn Main(){var q=[[],[true,true,false]];}",
            "fn Main(){var q=[[],true];}",
            &[("NX-E0103", None, 17, 19)],
        ),
        (
            "fn Main(){var q=[65]==[];var r=Len([q,q]);}",
            "fn Main(){var q=[65]==true;var r=Len([q,q]);}",
            &[("NX-E0102", Some("type_mismatch"), 22, 26)],
        ),
        (
            "fn Main(){var q=[];let r=12;let a:Array<I64>=q[r];}",
            "fn Main(){var q=[];let r=true;let a:Array<I64>=q[r];}",
            &[
                ("NX-E0103", None, 16, 18),
                ("NX-E0102", Some("type_mismatch"), 49, 50),
            ],
        ),
    ];
    for (positive, negative, expected) in pairs {
        compile_scalar("fixture.n", positive).unwrap();
        assert!(check_scalar("fixture.n", positive).is_ok());
        let ds = rejected(negative);
        let actual: Vec<_> = ds
            .iter()
            .map(|d| {
                let span = d.primary_span.as_ref().unwrap();
                (d.code.as_str(), d.subcause.as_deref(), span.start, span.end)
            })
            .collect();
        assert_eq!(&actual, expected, "{negative}");
        assert!(check_scalar("fixture.n", negative).is_err());
    }
}

#[test]
fn deferred_array_conflict_keeps_whole_operand_types() {
    // §4.1.2.6 and §13.17: delayed element unification reports the type of
    // the complete source occurrence, as its concrete twin does.
    for (source, at, origin, width, expected, found) in [
        (
            "fn Main(){let ar=[1];let aar=[[1]];var r=[];let a=[r,ar,aar];}",
            "aar",
            "ar,aar",
            2,
            "Array<I64>",
            "Array<Array<I64>>",
        ),
        (
            "fn Main(){let ar=[1];let aar=[[1]];let r=[3];let a=[r,ar,aar];}",
            "aar",
            "r,ar",
            1,
            "Array<I64>",
            "Array<Array<I64>>",
        ),
        (
            "fn Main(){let ar=[1];let aar=[[1]];var r=[];let a=[r,aar,ar];}",
            "ar",
            // The first element r establishes the array element shape;
            // §4.7 keeps that witness when ar is the incompatible leaf.
            "r,aar",
            1,
            "Array<Array<I64>>",
            "Array<I64>",
        ),
        (
            "fn Main(){let ar=[1];let aar=[[1]];let r=[3];let a=[r,aar,ar];}",
            "aar",
            "r,aar",
            1,
            "Array<I64>",
            "Array<Array<I64>>",
        ),
    ] {
        let ds = rejected(source);
        assert_eq!(ds.len(), 1, "{source}: {ds:?}");
        let start = source.rfind(at).unwrap() as u64;
        let reference = source.find(origin).unwrap() as u64;
        exact_type_site(
            &ds[0],
            source,
            "NX-E0102",
            (start, start + at.len() as u64),
            expected,
            found,
            Some((reference, reference + width)),
        );
    }
    for source in [
        "fn Main(){let ar=[1];var r=[];let a0=ar==r[r[1]];r=[3];}",
        "fn Main(){let ar=[1];let r=[3];let a0=ar==r[r[1]];}",
    ] {
        let ds = rejected(source);
        assert_eq!(ds.len(), 1, "{source}: {ds:?}");
        let access = source.find("r[r[1]]").unwrap() as u64;
        let ar = source.find("ar==").unwrap() as u64;
        exact_type_site(
            &ds[0],
            source,
            "NX-E0102",
            (access, access + 7),
            "Array<I64>",
            "I64",
            Some((ar, ar + 2)),
        );
    }
    assert!(compile_scalar("fixture.n", "fn Main(){var r=[];let a=[r,[3]];r=[3];}").is_ok());
}

#[test]
fn pending_array_conflict_preserves_independent_numeric_rhs() {
    // §4.1.2.4 N(d): a conflict inside the left array does not consume +.
    for source in [
        "fn Main(){var r=[];let a=[1,r]+[2];r=[3];}",
        "fn Main(){let r=[3];let a=[1,r]+[2];}",
    ] {
        let ds = rejected(source);
        assert_eq!(ds.len(), 2, "{source}: {ds:?}");
        let left = source.find("r]+").unwrap() as u64;
        let first = source.find("[1,r]").unwrap() as u64;
        let right = source.find("[2]").unwrap() as u64;
        exact_type_site(
            &ds[0],
            source,
            "NX-E0102",
            (left, left + 1),
            "I64",
            "Array<I64>",
            Some((first + 1, first + 2)),
        );
        exact_type_site(
            &ds[1],
            source,
            "NX-E0102",
            (right, right + 3),
            "I64",
            "Array<I64>",
            None,
        );
    }
    let source = "fn Main(){var r=[];let a=[1,r]+[2];}";
    let ds = rejected(source);
    assert_eq!(ds.len(), 2, "{ds:?}");
    assert_eq!(ds[0].code, "NX-E0103");
    let right = source.find("[2]").unwrap() as u64;
    exact_type_site(
        &ds[1],
        source,
        "NX-E0102",
        (right, right + 3),
        "I64",
        "Array<I64>",
        None,
    );
    assert!(compile_scalar("fixture.n", "fn Main(){var r=[];let a=[1,r[0]];r=[3];}").is_ok());

    for source in [
        "fn Main(){var r=[];let a=[1,r]+true;r=[3];}",
        "fn Main(){let r=[3];let a=[1,r]+true;}",
    ] {
        let ds = rejected(source);
        assert_eq!(ds.len(), 2, "{source}: {ds:?}");
        let right = source.find("+true").unwrap() as u64 + 1;
        exact_type_site(
            &ds[1],
            source,
            "NX-E0102",
            (right, right + 4),
            "I64",
            "Bool",
            None,
        );
    }

    let source = "fn Main(){let br=[true];var r=[];let a=(r<(if true then {return;} else {return;}))-br;r=[[true]];}";
    let ds = rejected(source);
    assert_eq!(ds.len(), 2, "{ds:?}");
    let right = source.find("-br").unwrap() as u64 + 1;
    exact_type_site(
        &ds[1],
        source,
        "NX-E0102",
        (right, right + 2),
        "I64",
        "Array<Bool>",
        None,
    );
}

#[test]
fn pending_less_than_fixes_operand_before_later_assignment() {
    // §4.1.2.4 and RD120: the independent domain of < fixes q in source order.
    let source =
        "fn Main(){var q=(if true then {return;} else {return;});let y=(true+1)<q;q=true;}";
    let ds = rejected(source);
    assert_eq!(ds.len(), 2, "{ds:?}");
    let first = source.find("true+1").unwrap() as u64;
    let later = source.rfind("true").unwrap() as u64;
    exact_type_site(
        &ds[0],
        source,
        "NX-E0102",
        (first, first + 4),
        "I64",
        "Bool",
        None,
    );
    exact_type_site(
        &ds[1],
        source,
        "NX-E0102",
        (later, later + 4),
        "I64",
        "Bool",
        None,
    );

    let source = "fn Main(){var q=(if true then {return;}else{return;});let a=q[q<q];q=[1];}";
    let ds = rejected(source);
    assert_eq!(ds.len(), 2, "{ds:?}");
    let base = source.find("q[q<q]").unwrap() as u64;
    let later = source.rfind("[1]").unwrap() as u64;
    assert_eq!(
        (ds[0].code.as_str(), ds[0].subcause.as_deref()),
        ("NX-E0102", Some("not_indexable"))
    );
    assert_eq!(
        ds[0].primary_span.as_ref().map(|s| (s.start, s.end)),
        Some((base, base + 1))
    );
    assert_eq!(ds[0].data.len(), 2);
    assert_eq!(
        ds[0].data.get("expected_type"),
        Some(&crate::diagnostic::DataValue::String("domain:array".into()))
    );
    assert_eq!(
        ds[0].data.get("found_type"),
        Some(&crate::diagnostic::DataValue::String("I64".into()))
    );
    assert_eq!(ds[0].related_spans.len(), 1);
    assert_eq!(ds[0].related_spans[0].role, RelatedRole::FoundType);
    assert_eq!(
        (
            ds[0].related_spans[0].span.start,
            ds[0].related_spans[0].span.end
        ),
        (base, base + 1)
    );
    ds[0].validate("fixture.n", source).unwrap();
    exact_type_site(
        &ds[1],
        source,
        "NX-E0102",
        (later, later + 3),
        "I64",
        "Array<I64>",
        None,
    );
    for valid in [
        "fn Main(){var q=(if true then {return;}else{return;});let y=q<q;q=3;}",
        "fn Main(){var q=[1];let a=q[0];}",
    ] {
        assert!(compile_scalar("fixture.n", valid).is_ok(), "{valid}");
    }
}

#[test]
fn unresolved_hypothetical_index_write_remains_lowerable() {
    // §4.1.2.3: the base never gains an Array constructor, so its
    // selector's conditional I64 context never activates.
    let source = "fn Main(){var q=(if true then {return;}else{return;});q[true]=1;}";
    assert!(
        compile_scalar("fixture.n", source).is_ok(),
        "{:?}",
        compile_scalar("fixture.n", source)
    );
}

fn mismatch_at(
    source: &str,
    primary: (u64, u64),
    expected: Option<(u64, u64)>,
    found: &str,
    wanted: &str,
) {
    use crate::diagnostic::DataValue;
    let ds = rejected(source);
    assert_eq!(ds.len(), 1, "{source}: {ds:?}");
    let d = &ds[0];
    assert_eq!(
        (d.code.as_str(), d.subcause.as_deref()),
        ("NX-E0102", Some("type_mismatch"))
    );
    assert_eq!(
        (d.stage, d.operation),
        (Stage::Types, Operation::TypesCheck)
    );
    assert_eq!(
        d.primary_span.as_ref().map(|s| (s.start, s.end)),
        Some(primary)
    );
    assert_eq!(d.data.len(), 2);
    assert_eq!(
        d.data.get("expected_type"),
        Some(&DataValue::String(wanted.into()))
    );
    assert_eq!(
        d.data.get("found_type"),
        Some(&DataValue::String(found.into()))
    );
    let mut related = expected
        .map(|(a, b)| vec![(RelatedRole::ExpectedType, a, b)])
        .unwrap_or_default();
    related.push((RelatedRole::FoundType, primary.0, primary.1));
    related.sort_by_key(|r| (r.1, r.2));
    assert_eq!(
        d.related_spans
            .iter()
            .map(|r| (r.role, r.span.start, r.span.end))
            .collect::<Vec<_>>(),
        related
    );
    assert!(d.cause_chain.is_empty() && d.repair.is_empty());
    d.validate("fixture.n", source).unwrap();
}

fn rejected(source: &str) -> Vec<crate::diagnostic::Diagnostic> {
    match compile_scalar("fixture.n", source) {
        Err(CompileError::Rejected(errors)) => {
            assert_no_duplicate_diagnostics(&errors, source);
            errors.into_iter().map(|d| *d).collect()
        }
        other => panic!("expected rejection for {source}: {other:?}"),
    }
}

fn exact_type_site(
    d: &crate::diagnostic::Diagnostic,
    source: &str,
    code: &str,
    primary: (u64, u64),
    expected: &str,
    found: &str,
    origin: Option<(u64, u64)>,
) {
    use crate::diagnostic::DataValue;
    assert_eq!(
        (d.code.as_str(), d.subcause.as_deref()),
        (code, (code == "NX-E0102").then_some("type_mismatch"))
    );
    assert_eq!(
        (d.stage, d.operation),
        (Stage::Types, Operation::TypesCheck)
    );
    assert_eq!(
        d.primary_span.as_ref().map(|s| (s.start, s.end)),
        Some(primary)
    );
    assert_eq!(d.data.len(), 2);
    assert_eq!(
        d.data.get("expected_type"),
        Some(&DataValue::String(expected.into()))
    );
    assert_eq!(
        d.data.get("found_type"),
        Some(&DataValue::String(found.into()))
    );
    let mut related = origin
        .map(|(a, b)| vec![(RelatedRole::ExpectedType, a, b)])
        .unwrap_or_default();
    related.push((RelatedRole::FoundType, primary.0, primary.1));
    related.sort_by_key(|r| (r.1, r.2));
    assert_eq!(
        d.related_spans
            .iter()
            .map(|r| (r.role, r.span.start, r.span.end))
            .collect::<Vec<_>>(),
        related
    );
    assert!(d.cause_chain.is_empty() && d.repair.is_empty());
    d.validate("fixture.n", source).unwrap();
}

#[test]
fn deferred_sites_emit_once_and_keep_independent_siblings() {
    // §4.7 checks a concrete context at each operand. §13.9 keeps reached
    // independent sites; §4.1.1 suppresses only consequences of taint.
    for (source, expected) in [
        (
            "fn Main(){var r=[];let a:Array<Bool>=(if true then (if true then r else r) else 5);r=[true];}",
            vec![(
                "NX-E0102",
                80,
                81,
                "5",
                "Array<Bool>",
                "I64",
                Some((25, 36)),
            )],
        ),
        (
            "fn Main(){var r=[];let a:Bool=(if true then r else true);r=[[1]];}",
            vec![(
                "NX-E0102",
                44,
                45,
                "r",
                "Bool",
                "Array<Array<I64>>",
                Some((25, 29)),
            )],
        ),
        (
            "fn Main(){let x=1;let ar=[1];let a=(if true then (if true then x else (true+1)) else ar);}",
            vec![
                ("NX-E0102", 71, 75, "true", "I64", "Bool", None),
                (
                    "NX-E1303",
                    85,
                    87,
                    "ar",
                    "I64",
                    "Array<I64>",
                    Some((50, 78)),
                ),
            ],
        ),
        (
            "fn Main(){let x=1;let ar=[1];let a=(if false then (if x<158 then x else {value:(if true then false else 111)+(105-185);}) else ar);}",
            vec![
                ("NX-E0102", 93, 98, "false", "I64", "Bool", None),
                (
                    "NX-E1303",
                    127,
                    129,
                    "ar",
                    "I64",
                    "Array<I64>",
                    Some((51, 120)),
                ),
            ],
        ),
        (
            "fn Main(){let x=1;let p=true;let ar=[1];let a=(if (true) then (if p then x+ar else (x)) else [true,(false),false]);}",
            vec![
                ("NX-E0102", 75, 77, "ar", "I64", "Array<I64>", None),
                (
                    "NX-E1303",
                    93,
                    113,
                    "[true,(false),false]",
                    "I64",
                    "Array<Bool>",
                    Some((63, 86)),
                ),
            ],
        ),
        (
            "fn Main(){let v:I64=true;let w=v==(if true then [true,[1]][0] else (if false then 1 else true));}",
            vec![
                ("NX-E0102", 20, 24, "true", "I64", "Bool", Some((16, 19))),
                (
                    "NX-E0102",
                    54,
                    57,
                    "[1]",
                    "Bool",
                    "Array<I64>",
                    Some((49, 53)),
                ),
                ("NX-E1303", 89, 93, "true", "I64", "Bool", Some((82, 83))),
            ],
        ),
        (
            "fn F1(p1:view I64){}fn Main(){let v7=29;F1((if (true==(if [[1,true]] then 30 else true)) then v7 else true));}",
            vec![
                ("NX-E0102", 62, 66, "true", "I64", "Bool", Some((60, 61))),
                ("NX-E1303", 82, 86, "true", "I64", "Bool", Some((74, 76))),
                ("NX-E0102", 102, 106, "true", "I64", "Bool", Some((14, 17))),
            ],
        ),
    ] {
        let run = rejected(source);
        let check = match check_scalar("fixture.n", source) {
            Err(CompileError::Rejected(errors)) => errors,
            other => panic!("expected check rejection: {other:?}"),
        };
        for diagnostics in [
            run.iter().collect::<Vec<_>>(),
            check.iter().map(|d| d.as_ref()).collect(),
        ] {
            assert_eq!(diagnostics.len(), expected.len(), "{source}");
            let mut distinct = std::collections::HashSet::new();
            for (d, &(code, start, end, token, wanted, found, origin)) in
                diagnostics.iter().zip(&expected)
            {
                assert_eq!(
                    (d.code.as_str(), d.subcause.as_deref()),
                    (code, (code == "NX-E0102").then_some("type_mismatch"))
                );
                assert_eq!(
                    (d.stage, d.operation),
                    (Stage::Types, Operation::TypesCheck)
                );
                assert_eq!(
                    d.primary_span.as_ref().map(|s| (s.start, s.end)),
                    Some((start, end))
                );
                assert_eq!(&source[start as usize..end as usize], token);
                assert_eq!(d.data.len(), 2);
                assert_eq!(
                    d.data.get("expected_type"),
                    Some(&crate::diagnostic::DataValue::String(wanted.into()))
                );
                assert_eq!(
                    d.data.get("found_type"),
                    Some(&crate::diagnostic::DataValue::String(found.into()))
                );
                let mut related = vec![(RelatedRole::FoundType, start, end)];
                if let Some((a, b)) = origin {
                    related.push((RelatedRole::ExpectedType, a, b));
                }
                related.sort_by_key(|r| (r.1, r.2));
                assert_eq!(
                    d.related_spans
                        .iter()
                        .map(|r| (r.role, r.span.start, r.span.end))
                        .collect::<Vec<_>>(),
                    related
                );
                assert!(d.cause_chain.is_empty() && d.repair.is_empty());
                d.validate("fixture.n", source).unwrap();
                assert!(
                    distinct.insert((start, end)),
                    "one diagnostic per site: {source}"
                );
            }
        }
    }
}

#[test]
fn divergent_initializer_and_array_operand_follow_their_distinct_constraints() {
    // §4.1.1: Never does not constrain a binding's monomorphic type. The
    // later indexed write supplies its array constraint in either variant.
    let divergent = "fn Main(){var q=(if true then {return;}else{return;});q[-3]=false;}";
    let live = "fn Main(){var q=[false];q[-3]=false;}";
    for source in [divergent, live] {
        check_scalar("fixture.n", source).unwrap();
        compile_scalar("fixture.n", source).unwrap();
    }
    for source in [
        "fn F(x:view I64){} fn Main(){var q=(if true then {return;}else{return;});F(q[0]);}",
        "fn F(x:view I64){} fn Main(){var q=[1];F(q[0]);}",
    ] {
        check_scalar("fixture.n", source).unwrap();
        compile_scalar("fixture.n", source).unwrap();
    }

    // §4.1.2.2 and RD127 require NX-E0103 for a Never-only element.
    let unresolved = "fn Main(){AssertEq([(if true then {return;}else{return;})],true);}";
    let ds = rejected(unresolved);
    assert_eq!(ds.len(), 1);
    assert_eq!(ds[0].code, "NX-E0103");
    let resolved = "fn Main(){AssertEq([1],true);}";
    let ds = rejected(resolved);
    assert_eq!(ds.len(), 1);
    assert_eq!(ds[0].code, "NX-E0102");
}

#[test]
fn nested_branch_anchor_reports_the_first_incompatible_leaf_once() {
    use crate::diagnostic::DataValue;
    // §4.7 synthesizes normal continuations in source order; §13.9 keeps
    // the independent error in a later child without a parent diagnosis.
    let source = "fn Main(){let x=1;let p=true;var ar=[1];var br=[true];var aar=[[1]];let a=(if p then {value:[p,false];} else (if p then {value:[false,p];} else (if p then (if p then x else [true,p]) else {value:[p];})));}";
    for diagnostics in [
        rejected(source),
        match check_scalar("fixture.n", source) {
            Err(CompileError::Rejected(errors)) => errors.into_iter().map(|d| *d).collect(),
            other => panic!("expected rejection: {other:?}"),
        },
    ] {
        assert_eq!(diagnostics.len(), 1, "{diagnostics:?}");
        let d = &diagnostics[0];
        assert_eq!(d.code, "NX-E1303");
        assert_eq!(
            d.primary_span.as_ref().map(|s| (s.start, s.end)),
            Some((166, 167))
        );
        assert_eq!(
            d.data.get("expected_type"),
            Some(&DataValue::String("Array<Bool>".into()))
        );
        assert_eq!(
            d.data.get("found_type"),
            Some(&DataValue::String("I64".into()))
        );
    }
}

#[test]
fn nested_synthesized_if_uses_the_written_value_operand_as_origin() {
    // §4.7 synthesizes each continuation; §13.15 names the operand of
    // value: in the first normal continuation. Here that operand is the
    // complete inner if, not one of its concrete leaves.
    let source =
        "fn Main(){let x=(if false then (if true then {value:1;} else {value:2;}) else true);}";
    let ds = rejected(source);
    assert_eq!(ds.len(), 1);
    let d = &ds[0];
    assert_eq!((d.code.as_str(), d.subcause.as_deref()), ("NX-E1303", None));
    assert_eq!(
        d.primary_span.as_ref().map(|s| (s.start, s.end)),
        Some((78, 82))
    );
    assert_eq!(d.data.len(), 2);
    assert_eq!(
        d.data.get("expected_type"),
        Some(&crate::diagnostic::DataValue::String("I64".into()))
    );
    assert_eq!(
        d.data.get("found_type"),
        Some(&crate::diagnostic::DataValue::String("Bool".into()))
    );
    assert_eq!(
        d.related_spans
            .iter()
            .map(|r| (r.role, r.span.start, r.span.end))
            .collect::<Vec<_>>(),
        vec![
            (RelatedRole::ExpectedType, 32, 71),
            (RelatedRole::FoundType, 78, 82)
        ]
    );
    let repaired =
        "fn Main(){let x=(if false then (if true then {value:1;} else {value:2;}) else 3);}";
    check_scalar("fixture.n", repaired).unwrap();
    compile_scalar("fixture.n", repaired).unwrap();
}

#[test]
fn contextual_origins_follow_first_array_element_annotation_return_and_formal() {
    // F0-0002 §§4.1.1, 13.15: the first concrete element or the
    // written type is the exact source of the expected-type relation.
    mismatch_at(
        "fn Main(){let x=[1,true];}",
        (19, 23),
        Some((17, 18)),
        "Bool",
        "I64",
    );
    mismatch_at(
        "fn Main(){let x:Array<Bool>=[1,true];}",
        (29, 30),
        Some((16, 27)),
        "I64",
        "Bool",
    );
    mismatch_at(
        "fn F()->I64{return true;} fn Main(){}",
        (19, 23),
        Some((8, 11)),
        "Bool",
        "I64",
    );
    mismatch_at(
        "fn F(x:view I64){} fn Main(){F(true);}",
        (31, 35),
        Some((12, 15)),
        "Bool",
        "I64",
    );
}

#[test]
fn failed_array_element_keeps_annotation_for_later_independent_errors() {
    use crate::diagnostic::DataValue;
    // F0-0002 §§4.1.1/13.9: neither failed element changes the written
    // Bool constraint; each later site retains that annotation as evidence.
    let source = "fn Main(){let x:Array<Bool>=[1,2];}";
    let ds = rejected(source);
    assert_eq!(ds.len(), 2, "{ds:?}");
    for (d, primary) in ds.iter().zip([(29, 30), (31, 32)]) {
        assert_eq!(
            (d.code.as_str(), d.subcause.as_deref()),
            ("NX-E0102", Some("type_mismatch"))
        );
        assert_eq!(
            d.primary_span.as_ref().map(|s| (s.start, s.end)),
            Some(primary)
        );
        assert_eq!(d.data.len(), 2);
        assert_eq!(
            d.data.get("expected_type"),
            Some(&DataValue::String("Bool".into()))
        );
        assert_eq!(
            d.data.get("found_type"),
            Some(&DataValue::String("I64".into()))
        );
        assert_eq!(
            d.related_spans
                .iter()
                .map(|r| (r.role, r.span.start, r.span.end))
                .collect::<Vec<_>>(),
            vec![
                (RelatedRole::ExpectedType, 16, 27),
                (RelatedRole::FoundType, primary.0, primary.1)
            ]
        );
        d.validate("fixture.n", source).unwrap();
    }
}

#[test]
fn tainted_array_constructor_does_not_add_a_parent_shape_error() {
    // F0-0002 §4.1.1: a prior child error contaminates its constructor.
    // §13.9 suppresses only the consequent outer I64/Array mismatch.
    mismatch_at(
        "fn Main(){let x:I64=[1,true];}",
        (23, 27),
        Some((21, 22)),
        "Bool",
        "I64",
    );
}

#[test]
fn bad_binary_left_still_checks_independent_right_branch() {
    use crate::diagnostic::DataValue;
    // F0-0002 §13.9 keeps the right branch's independent join failure.
    let source = "fn Main(){let x=true+(if true then 1 else false);}";
    let ds = rejected(source);
    assert_eq!(ds.len(), 2, "{ds:?}");
    assert_eq!(
        (ds[0].code.as_str(), ds[0].subcause.as_deref()),
        ("NX-E0102", Some("type_mismatch"))
    );
    assert_eq!(
        ds[0].primary_span.as_ref().map(|s| (s.start, s.end)),
        Some((16, 20))
    );
    assert_eq!(
        ds[0].data.get("expected_type"),
        Some(&DataValue::String("I64".into()))
    );
    assert_eq!(
        ds[0].data.get("found_type"),
        Some(&DataValue::String("Bool".into()))
    );
    assert_eq!(
        (ds[1].code.as_str(), ds[1].subcause.as_deref()),
        ("NX-E1303", None)
    );
    assert_eq!(
        ds[1].primary_span.as_ref().map(|s| (s.start, s.end)),
        Some((42, 47))
    );
    assert_eq!(
        ds[1].data.get("expected_type"),
        Some(&DataValue::String("I64".into()))
    );
    assert_eq!(
        ds[1].data.get("found_type"),
        Some(&DataValue::String("Bool".into()))
    );
    assert_eq!(
        ds[1]
            .related_spans
            .iter()
            .map(|r| (r.role, r.span.start, r.span.end))
            .collect::<Vec<_>>(),
        vec![
            (RelatedRole::ExpectedType, 35, 36),
            (RelatedRole::FoundType, 42, 47)
        ]
    );
    for d in &ds {
        d.validate("fixture.n", source).unwrap();
    }
}

#[test]
fn tainted_binary_right_keeps_left_error_without_parent_cascade() {
    use crate::diagnostic::DataValue;
    // F0-0002 §§4.1.1/13.9: both leaves fail independently, while the
    // result of their operator is contaminated and adds no Bool/I64 error.
    let source = "fn Main(){let x:Bool=true+[1,false];}";
    let ds = rejected(source);
    assert_eq!(ds.len(), 2, "{ds:?}");
    for (d, primary, related) in [
        (&ds[0], (21, 25), vec![(RelatedRole::FoundType, 21, 25)]),
        (
            &ds[1],
            (29, 34),
            vec![
                (RelatedRole::ExpectedType, 27, 28),
                (RelatedRole::FoundType, 29, 34),
            ],
        ),
    ] {
        assert_eq!(
            (d.code.as_str(), d.subcause.as_deref()),
            ("NX-E0102", Some("type_mismatch"))
        );
        assert_eq!(
            d.primary_span.as_ref().map(|s| (s.start, s.end)),
            Some(primary)
        );
        assert_eq!(
            d.related_spans
                .iter()
                .map(|r| (r.role, r.span.start, r.span.end))
                .collect::<Vec<_>>(),
            related
        );
        assert_eq!(d.data.len(), 2);
        assert_eq!(
            d.data.get("expected_type"),
            Some(&DataValue::String("I64".into()))
        );
        assert_eq!(
            d.data.get("found_type"),
            Some(&DataValue::String("Bool".into()))
        );
        d.validate("fixture.n", source).unwrap();
    }
}

#[test]
fn equality_accepts_bool_operands_without_numeric_context() {
    // F0-0002 §4.6: == imposes equal copyable operand types, not I64.
    for source in [
        "fn Main(){let x=true==true;}",
        "fn Main(){let x=true==(if true then true else false);}",
        "fn Main(){let x=[true]==[false];}",
    ] {
        assert!(compile_scalar("fixture.n", source).is_ok(), "{source}");
    }
}

#[test]
fn nested_else_if_keeps_first_normal_branch_as_expected_origin() {
    use crate::diagnostic::DataValue;
    // F0-0002 §§4.1.1/4.7: source order fixes the earlier I64 branch.
    let source = "fn Main(){let x=(if true then 1 else (if false then true else 2));}";
    let ds = rejected(source);
    assert_eq!(ds.len(), 1, "{ds:?}");
    let d = &ds[0];
    assert_eq!((d.code.as_str(), d.subcause.as_deref()), ("NX-E1303", None));
    assert_eq!(
        d.primary_span.as_ref().map(|s| (s.start, s.end)),
        Some((52, 56))
    );
    assert_eq!(
        d.related_spans
            .iter()
            .map(|r| (r.role, r.span.start, r.span.end))
            .collect::<Vec<_>>(),
        vec![
            (RelatedRole::ExpectedType, 30, 31),
            (RelatedRole::FoundType, 52, 56)
        ]
    );
    assert_eq!(d.data.len(), 2);
    assert_eq!(
        d.data.get("expected_type"),
        Some(&DataValue::String("I64".into()))
    );
    assert_eq!(
        d.data.get("found_type"),
        Some(&DataValue::String("Bool".into()))
    );
    d.validate("fixture.n", source).unwrap();
}

#[test]
fn else_block_value_retains_written_expected_origin() {
    mismatch_at(
        "fn Main(){let x:Bool=(if true then true else {value:1;});}",
        (52, 53),
        Some((16, 20)),
        "I64",
        "Bool",
    );
}

#[test]
fn deferred_array_branch_conflict_keeps_first_operand_as_expected() {
    use crate::diagnostic::DataValue;
    // F0-0002 §4.1.2.4: the written Array<I64> reaches both branches;
    // the scalar fails, while the deferred Array can satisfy that context.
    let source = "fn Main(){var v1=[];let v3:Array<I64>=(if true then 0 else v1);}";
    let ds = rejected(source);
    assert_eq!(ds.len(), 1, "{ds:?}");
    let (d, primary, expected, found, origin) = (&ds[0], (52, 53), "Array<I64>", "I64", (27, 37));
    assert_eq!(
        (d.code.as_str(), d.subcause.as_deref()),
        ("NX-E0102", Some("type_mismatch"))
    );
    assert_eq!(
        d.primary_span.as_ref().map(|s| (s.start, s.end)),
        Some(primary)
    );
    assert_eq!(
        d.related_spans
            .iter()
            .map(|r| (r.role, r.span.start, r.span.end))
            .collect::<Vec<_>>(),
        vec![
            (RelatedRole::ExpectedType, origin.0, origin.1),
            (RelatedRole::FoundType, primary.0, primary.1)
        ]
    );
    assert_eq!(d.data.len(), 2);
    assert_eq!(
        d.data.get("expected_type"),
        Some(&DataValue::String(expected.into()))
    );
    assert_eq!(
        d.data.get("found_type"),
        Some(&DataValue::String(found.into()))
    );
    d.validate("fixture.n", source).unwrap();
}

#[test]
fn unresolved_array_alias_cycle_keeps_both_constructor_obligations() {
    // F0-0002 §4.1.2.2: there is one unresolved Array constructor;
    // the projected read has no independent constructor obligation.
    let source = "fn Main(){var v1=[];let v2=v1;v1=v2[0];}";
    let ds = rejected(source);
    assert_eq!(ds.len(), 1, "{ds:?}");
    for (d, primary) in ds.iter().zip([(17, 19)]) {
        assert_eq!((d.code.as_str(), d.subcause.as_deref()), ("NX-E0103", None));
        assert_eq!(
            d.primary_span.as_ref().map(|s| (s.start, s.end)),
            Some(primary)
        );
        assert!(d.related_spans.is_empty() && d.data.is_empty());
        d.validate("fixture.n", source).unwrap();
    }
    assert!(
        compile_scalar(
            "fixture.n",
            "fn Main(){var v1:Array<I64>=[];let v2=v1;v1=[v2[0]];}"
        )
        .is_ok()
    );
}

#[test]
fn divergent_array_element_retains_copyable_domain_on_later_index_write() {
    use crate::diagnostic::DataValue;
    // F0-0002 §4.1 limits array elements to copyable data. §§4.1.1/13.17
    // keep a bottom-only element unresolved until a later write constrains it.
    let source = "fn Main(){var v=[(if true then {return;}else{return;})];v[0]=MockOpen(1);}";
    let ds = rejected(source);
    assert_eq!(ds.len(), 2, "{ds:?}");
    assert_eq!(
        (
            ds[0].code.as_str(),
            ds[0].primary_span.as_ref().map(|s| (s.start, s.end))
        ),
        ("NX-E0103", Some((16, 55)))
    );
    assert_eq!(
        (ds[1].code.as_str(), ds[1].subcause.as_deref()),
        ("NX-E0102", Some("type_mismatch"))
    );
    assert_eq!(
        ds[1].primary_span.as_ref().map(|s| (s.start, s.end)),
        Some((61, 72))
    );
    assert_eq!(ds[1].data.len(), 2);
    assert_eq!(
        ds[1].data.get("expected_type"),
        Some(&DataValue::String("domain:copyable_data".into()))
    );
    assert_eq!(
        ds[1].data.get("found_type"),
        Some(&DataValue::String("MockResource".into()))
    );
    assert_eq!(
        ds[1]
            .related_spans
            .iter()
            .map(|r| (r.role, r.span.start, r.span.end))
            .collect::<Vec<_>>(),
        vec![(RelatedRole::FoundType, 61, 72)]
    );
    for d in &ds {
        d.validate("fixture.n", source).unwrap();
    }
}

#[test]
fn comparison_domains_hold_with_concrete_and_deferred_right_operands() {
    use crate::diagnostic::DataValue;
    // F0-0002 §§4.6/13.15: < requires I64 and == requires copyable
    // data whether the right operand is already concrete or resolves later.
    for (source, primary, found_span, expected, found) in [
        (
            "fn Main(){let x=true<false;}",
            (20, 21),
            (16, 20),
            "I64",
            "Bool",
        ),
        (
            "fn Main(){let x=MockOpen(1)==MockOpen(2);}",
            (27, 29),
            (16, 27),
            "domain:copyable_data",
            "MockResource",
        ),
        (
            "fn Main(){let r=MockOpen(1);var q=(if true then {return;}else{return;});let x=r==q;q=MockOpen(2);}",
            (79, 81),
            (78, 79),
            "domain:copyable_data",
            "MockResource",
        ),
    ] {
        let ds = rejected(source);
        assert_eq!(ds.len(), 1, "{source}: {ds:?}");
        let d = &ds[0];
        assert_eq!(
            (d.code.as_str(), d.subcause.as_deref()),
            ("NX-E0102", Some("type_mismatch"))
        );
        assert_eq!(
            d.primary_span.as_ref().map(|s| (s.start, s.end)),
            Some(primary)
        );
        assert_eq!(
            d.related_spans
                .iter()
                .map(|r| (r.role, r.span.start, r.span.end))
                .collect::<Vec<_>>(),
            vec![(RelatedRole::FoundType, found_span.0, found_span.1)]
        );
        assert_eq!(d.data.len(), 2);
        assert_eq!(
            d.data.get("expected_type"),
            Some(&DataValue::String(expected.into()))
        );
        assert_eq!(
            d.data.get("found_type"),
            Some(&DataValue::String(found.into()))
        );
        d.validate("fixture.n", source).unwrap();
    }

    let source = "fn Main(){var q=(if true then {return;}else{return;});let x=true<q;q=1;}";
    let ds = rejected(source);
    assert_eq!(ds.len(), 2, "{ds:?}");
    assert_eq!(
        (
            ds[0].code.as_str(),
            ds[0].primary_span.as_ref().map(|s| (s.start, s.end))
        ),
        ("NX-E0102", Some((64, 65)))
    );
    assert_eq!(
        ds[0]
            .related_spans
            .iter()
            .map(|r| (r.role, r.span.start, r.span.end))
            .collect::<Vec<_>>(),
        vec![(RelatedRole::FoundType, 60, 64)]
    );
    assert_eq!(
        ds[0].data.get("expected_type"),
        Some(&DataValue::String("I64".into()))
    );
    assert_eq!(
        ds[0].data.get("found_type"),
        Some(&DataValue::String("Bool".into()))
    );
    assert_eq!(
        (
            ds[1].code.as_str(),
            ds[1].primary_span.as_ref().map(|s| (s.start, s.end))
        ),
        ("NX-E0102", Some((69, 70)))
    );
    assert_eq!(
        ds[1].data.get("expected_type"),
        Some(&DataValue::String("Bool".into()))
    );
    assert_eq!(
        ds[1].data.get("found_type"),
        Some(&DataValue::String("I64".into()))
    );
    for d in &ds {
        d.validate("fixture.n", source).unwrap();
    }
}

#[test]
fn expected_context_reports_each_independent_branch_and_element() {
    // F0-0002 §4.7: «Con tipo esperado, ese tipo se propaga a los operandos
    // de `value:`». §13.9 preserves independent sites; §13.17 uses each
    // checked leaf's type for found_type.
    for (source, bad) in [
        (
            "fn Main(){let x:Array<Bool>=(if true then [1,true] else [false]);}",
            vec!["1"],
        ),
        (
            "fn F()->Array<Bool>{return (if true then [36,57,true] else [false]);}",
            vec!["36", "57"],
        ),
        (
            "fn Main(){let x:Array<Bool>=(if true then [1,2] else [3]);}",
            vec!["1", "2", "3"],
        ),
        (
            "fn Main(){let x:Bool=(if true then 2 else (if false then 3 else 4));}",
            vec!["2", "3", "4"],
        ),
        (
            "fn Main(){let x:Bool=(if true then (if false then 2 else 3) else 4);}",
            vec!["2", "3", "4"],
        ),
        (
            "fn Main(){let x:Bool=(if true then (if false then 2 else true) else 4);}",
            vec!["2", "4"],
        ),
        (
            "fn F()->Bool{return (if true then 2 else (if false then 3 else 4));} fn Main(){}",
            vec!["2", "3", "4"],
        ),
        (
            "fn Main(){let x:Bool=(if true then 2 else 3);}",
            vec!["2", "3"],
        ),
        (
            "fn Main(){let x:Array<I64>=[true,false];}",
            vec!["true", "false"],
        ),
        (
            "fn Main(){let x:Array<Bool>=[(if true then 2 else true)];}",
            vec!["2"],
        ),
        (
            "fn Main(){let x:Array<Bool>=[(if true then 2 else 0)];}",
            vec!["2", "0"],
        ),
        (
            "fn Main(){var a=[true];a[0]=(if true then 2 else 0);}",
            vec!["2", "0"],
        ),
        (
            "fn F(x:view Bool){} fn Main(){F((if true then 2 else 0));}",
            vec!["2", "0"],
        ),
        (
            "fn Main(){let x=(if true then true else false)+1;}",
            vec!["true", "false"],
        ),
        (
            "fn Main(){let x=1+(if true then true else false);}",
            vec!["true", "false"],
        ),
    ] {
        let ds = rejected(source);
        let actual: Vec<_> = ds
            .iter()
            .map(|d| {
                let span = d.primary_span.as_ref().unwrap();
                (
                    d.code.as_str(),
                    &source[span.start as usize..span.end as usize],
                )
            })
            .collect();
        let wanted: Vec<_> = bad.iter().map(|s| ("NX-E0102", *s)).collect();
        assert_eq!(actual, wanted, "{source}: {ds:?}");
        for (d, token) in ds.iter().zip(bad) {
            assert_eq!(
                (d.subcause.as_deref(), d.stage, d.operation),
                (Some("type_mismatch"), Stage::Types, Operation::TypesCheck)
            );
            let expected = if source.contains("Array<I64>")
                || source.contains("+1")
                || source.contains("1+")
            {
                "I64"
            } else {
                "Bool"
            };
            let found = if token == "true" || token == "false" {
                "Bool"
            } else {
                "I64"
            };
            assert_eq!(d.data.len(), 2, "{source}: {d:?}");
            assert_eq!(
                d.data.get("expected_type"),
                Some(&crate::diagnostic::DataValue::String(expected.into()))
            );
            assert_eq!(
                d.data.get("found_type"),
                Some(&crate::diagnostic::DataValue::String(found.into()))
            );
            assert!(
                d.related_spans
                    .iter()
                    .any(|r| r.role == RelatedRole::FoundType
                        && d.primary_span.as_ref() == Some(&r.span))
            );
            d.validate("fixture.n", source).unwrap();
        }
    }
    // ADR §14.10 RD128: the two causal leaves reject independently;
    // changing both leaves to I64 satisfies the same annotation.
    let repaired = "fn Main(){let x:Array<I64>=[1,2];}";
    check_scalar("fixture.n", repaired).unwrap();
    compile_scalar("fixture.n", repaired).unwrap();
}

#[test]
fn expected_nested_array_reports_scalar_leaf_type() {
    // F0-0002 §13.15: «expresión cuyo tipo incumple la regla»;
    // §13.17: «Tipo de la expresión comprobada».
    let source = "fn Main(){let x:Array<Array<I64>>=[[false]];}";
    let leaf = source.find("false").unwrap() as u64;
    let annotation = source.find("Array<Array<I64>>").unwrap() as u64;
    mismatch_at(
        source,
        (leaf, leaf + 5),
        Some((annotation, annotation + 17)),
        "Bool",
        "I64",
    );
    let positive = source.replace("false", "1");
    assert!(compile_scalar("fixture.n", &positive).is_ok());
}

#[test]
fn expected_context_reaches_else_block_value_with_and_without_deferred_alias() {
    // F0-0002 §4.7: «Con tipo esperado, ese tipo se propaga a los operandos
    // de `value:`»; §13.9 keeps the two incompatible leaves independent.
    for deferred in [false, true] {
        let initial = if deferred {
            "var q=(if true then {return;}else{return;});"
        } else {
            "var q=true;"
        };
        let source = format!(
            "fn Main(){{{initial}let x:Bool=(if true then 2 else {{value:(if false then 3 else q);}});q=true;}}"
        );
        let ds = rejected(&source);
        assert_eq!(ds.len(), 2, "{source}: {ds:?}");
        for (diagnostic, literal) in ds.iter().zip(["2", "3"]) {
            let at = source.find(&format!("then {literal}")).unwrap() as u64 + 5;
            assert_eq!(diagnostic.code, "NX-E0102");
            assert_eq!(diagnostic.subcause.as_deref(), Some("type_mismatch"));
            assert_eq!(
                diagnostic.primary_span.as_ref().map(|s| (s.start, s.end)),
                Some((at, at + 1))
            );
            assert_eq!(
                diagnostic.data.get("expected_type"),
                Some(&crate::diagnostic::DataValue::String("Bool".into()))
            );
            assert_eq!(
                diagnostic.data.get("found_type"),
                Some(&crate::diagnostic::DataValue::String("I64".into()))
            );
            diagnostic.validate("fixture.n", &source).unwrap();
        }
        let positive = source
            .replace("then 2", "then true")
            .replace("then 3", "then false");
        assert!(compile_scalar("fixture.n", &positive).is_ok(), "{positive}");
    }
}

#[test]
fn then_value_relates_the_written_expected_type() {
    // F0-0002 §13.15: «anotación que impone ese contexto» is the
    // expected_type origin; §4.7 checks the incompatible branch operand.
    let source = "fn Main(){let x:Bool=(if true then 2 else true);}";
    let leaf = source.find("then 2").unwrap() as u64 + 5;
    let annotation = source.find("Bool").unwrap() as u64;
    mismatch_at(
        source,
        (leaf, leaf + 1),
        Some((annotation, annotation + 4)),
        "I64",
        "Bool",
    );
    assert!(compile_scalar("fixture.n", &source.replace("then 2", "then false")).is_ok());
}

#[test]
fn failed_else_value_does_not_supply_an_outer_operand_type() {
    // F0-0002 §4.7 gives the intrinsic i64 context to the else value;
    // §13.9 forbids a second operator error derived from that failed value.
    let source = "fn Main(){let x=1+(if true then {return;} else true);}";
    let leaf = source.rfind("true").unwrap() as u64;
    mismatch_at(source, (leaf, leaf + 4), None, "Bool", "I64");
    assert!(compile_scalar("fixture.n", &source.replace("else true", "else 2")).is_ok());
}

#[test]
fn expected_context_reaches_every_value_position_with_deferred_alias() {
    // F0-0002 §4.7 names annotations, assignments, parameters and returns
    // as concrete contexts, and propagates them into branch values. §13.15
    // locates found_type at the incompatible operand; §13.17 reports its
    // scalar type even when surrounding syntax is an array or group.
    let cases = [
        (
            "fn Main(){INIT let x:Bool=(if true then 11 else q);FIN}",
            "11",
        ),
        (
            "fn Main(){INIT let x:Bool=(if false then q else (if true then 12 else q));FIN}",
            "12",
        ),
        ("fn Main(){INIT let x:Bool=((13));FIN}", "13"),
        ("fn Main(){INIT let x:Array<Bool>=[q,14];FIN}", "14"),
        (
            "fn F(v:view Bool){} fn Main(){INIT F((if true then q else 15));FIN}",
            "15",
        ),
        (
            "fn Main(){INIT var a:Array<Bool>=[q];a[0]=(if true then 16 else q);FIN}",
            "16",
        ),
        (
            "fn F()->Bool{INIT return (if true then q else 17);FIN} fn Main(){}",
            "17",
        ),
    ];
    for deferred in [false, true] {
        let initial = if deferred {
            "var q=(if true then {return;}else{return;});"
        } else {
            "var q=true;"
        };
        for (template, bad) in cases {
            let initializer = if deferred && template.starts_with("fn F()->Bool") {
                "var q=(if true then {return true;}else{return false;});"
            } else {
                initial
            };
            let source = template
                .replace("INIT", initializer)
                .replace("FIN", "q=true;");
            let at = source.find(bad).unwrap() as u64;
            let ds = rejected(&source);
            assert_eq!(ds.len(), 1, "{source}: {ds:?}");
            let diagnostic = &ds[0];
            assert_eq!(
                (diagnostic.code.as_str(), diagnostic.subcause.as_deref()),
                ("NX-E0102", Some("type_mismatch")),
                "{source}"
            );
            assert_eq!(
                diagnostic.primary_span.as_ref().map(|s| (s.start, s.end)),
                Some((at, at + 2)),
                "{source}"
            );
            assert_eq!(
                diagnostic.data.get("expected_type"),
                Some(&crate::diagnostic::DataValue::String("Bool".into())),
                "{source}"
            );
            assert_eq!(
                diagnostic.data.get("found_type"),
                Some(&crate::diagnostic::DataValue::String("I64".into())),
                "{source}"
            );
            diagnostic.validate("fixture.n", &source).unwrap();
            let positive = source.replacen(bad, "true", 1);
            assert!(compile_scalar("fixture.n", &positive).is_ok(), "{positive}");
        }
    }
}

#[test]
fn intrinsic_domain_does_not_fabricate_an_expected_type_span() {
    // F0-0002 §13.15: «Si solo se exige un dominio intrínseco, no se
    // fabrica un span para ese dominio». §13.17 gives I64/Bool for + and -.
    for source in [
        "fn Main(){let x=5+(if true then 1 else true);}",
        "fn Main(){let x=(35)+(if true then 1 else true);}",
        "fn Main(){let x=5-(if true then 1 else true);}",
    ] {
        let leaf = source.rfind("true").unwrap() as u64;
        mismatch_at(source, (leaf, leaf + 4), None, "Bool", "I64");
        assert!(compile_scalar("fixture.n", &source.replace("else true", "else 2")).is_ok());
    }
    // §4.7: without a concrete contextual type, a branch join reports
    // NX-E1303 on its later incompatible continuation.
    let source = "fn Main(){let x=(if true then true else 9)<5;}";
    let ds = rejected(source);
    assert_eq!(ds.len(), 1, "{ds:?}");
    assert_eq!(ds[0].code, "NX-E1303");
    let later = source.find("else 9").unwrap() as u64 + 5;
    assert_eq!(
        ds[0].primary_span.as_ref().map(|s| (s.start, s.end)),
        Some((later, later + 1))
    );
}

#[test]
#[ignore = "100000 unique expected-type programs checked against an independent leaf model"]
fn generated_expected_context_matches_recursive_leaf_model() {
    // F0-0002 §4.7 propagates a concrete expected type to every value
    // operand; §13.9 retains independent sites; §13.17 names each leaf's
    // concrete found type. This generator computes its oracle while building
    // source text, without using the checker or its inferred type table.
    #[derive(Clone, Copy)]
    enum Wanted {
        Bool,
        I64,
        ArrayBool,
        ArrayI64,
        NestedI64,
    }
    impl Wanted {
        fn label(self) -> &'static str {
            match self {
                Self::Bool => "Bool",
                Self::I64 => "I64",
                Self::ArrayBool => "Array<Bool>",
                Self::ArrayI64 => "Array<I64>",
                Self::NestedI64 => "Array<Array<I64>>",
            }
        }
        fn element(self) -> Option<Self> {
            match self {
                Self::ArrayBool => Some(Self::Bool),
                Self::ArrayI64 => Some(Self::I64),
                Self::NestedI64 => Some(Self::ArrayI64),
                _ => None,
            }
        }
    }
    #[derive(Clone)]
    struct Leaf {
        offset: usize,
        width: usize,
        expected: &'static str,
        found: &'static str,
    }
    fn next(state: &mut u64) -> u64 {
        *state ^= *state << 13;
        *state ^= *state >> 7;
        *state ^= *state << 17;
        *state
    }
    fn append(text: &mut String, leaves: &mut Vec<Leaf>, child: (String, Vec<Leaf>)) {
        let base = text.len();
        text.push_str(&child.0);
        leaves.extend(child.1.into_iter().map(|mut leaf| {
            leaf.offset += base;
            leaf
        }));
    }
    fn build(wanted: Wanted, depth: u8, state: &mut u64) -> (String, Vec<Leaf>) {
        let choice = next(state) % 7;
        if depth > 0 && choice == 0 {
            let mut text = String::from("(");
            let mut leaves = Vec::new();
            append(&mut text, &mut leaves, build(wanted, depth - 1, state));
            text.push(')');
            return (text, leaves);
        }
        if depth > 0 && (choice == 1 || choice == 2) {
            let block = choice == 2;
            let mut text = if block {
                String::from("(if true then {value:")
            } else {
                String::from("(if true then ")
            };
            let mut leaves = Vec::new();
            append(&mut text, &mut leaves, build(wanted, depth - 1, state));
            text.push_str(if block { ";} else {value:" } else { " else " });
            append(&mut text, &mut leaves, build(wanted, depth - 1, state));
            text.push_str(if block { ";})" } else { ")" });
            return (text, leaves);
        }
        if let Some(element) = wanted.element() {
            let mut text = String::from("[");
            let mut leaves = Vec::new();
            let count = 1 + (next(state) % 3) as usize;
            for index in 0..count {
                if index != 0 {
                    text.push(',');
                }
                append(
                    &mut text,
                    &mut leaves,
                    build(element, depth.saturating_sub(1), state),
                );
            }
            text.push(']');
            return (text, leaves);
        }
        let wrong = !next(state).is_multiple_of(3);
        let (literal, found) = match (wanted, wrong) {
            (Wanted::Bool, true) => (format!("{}", next(state) % 1000), "I64"),
            (Wanted::Bool, false) => ("true".into(), "Bool"),
            (Wanted::I64, true) => ("false".into(), "Bool"),
            (Wanted::I64, false) => (format!("{}", next(state) % 1000), "I64"),
            _ => unreachable!(),
        };
        let leaves = wrong
            .then(|| Leaf {
                offset: 0,
                width: literal.len(),
                expected: wanted.label(),
                found,
            })
            .into_iter()
            .collect();
        (literal, leaves)
    }
    let mut state = 0x8723_ba59_d641_3a07u64;
    let mut accepted = 0usize;
    let mut rejected_count = 0usize;
    let mut leaves_checked = 0usize;
    for nonce in 0..100_000u32 {
        let wanted = match next(&mut state) % 5 {
            0 => Wanted::Bool,
            1 => Wanted::I64,
            2 => Wanted::ArrayBool,
            3 => Wanted::ArrayI64,
            _ => Wanted::NestedI64,
        };
        let (expression, leaves) = build(wanted, 3, &mut state);
        let prefix = format!("fn Main(){{let nonce={nonce};let x:{}=", wanted.label());
        let source = format!("{prefix}{expression};}}",);
        let annotation = source.find(wanted.label()).unwrap() as u64;
        let actual = match compile_scalar("fixture.n", &source) {
            Ok(_) => Vec::new(),
            Err(CompileError::Rejected(errors)) => errors,
            other => panic!("unexpected compile outcome for {source}: {other:?}"),
        };
        assert_eq!(actual.len(), leaves.len(), "{source}: {actual:?}");
        for (diagnostic, leaf) in actual.iter().zip(&leaves) {
            let start = (prefix.len() + leaf.offset) as u64;
            assert_eq!(
                (diagnostic.code.as_str(), diagnostic.subcause.as_deref()),
                ("NX-E0102", Some("type_mismatch")),
                "{source}"
            );
            assert_eq!(
                diagnostic.primary_span.as_ref().map(|s| (s.start, s.end)),
                Some((start, start + leaf.width as u64)),
                "{source}"
            );
            assert_eq!(
                diagnostic.data.get("expected_type"),
                Some(&crate::diagnostic::DataValue::String(leaf.expected.into())),
                "{source}"
            );
            assert_eq!(
                diagnostic.data.get("found_type"),
                Some(&crate::diagnostic::DataValue::String(leaf.found.into())),
                "{source}"
            );
            assert!(
                diagnostic
                    .related_spans
                    .iter()
                    .any(|r| r.role == RelatedRole::ExpectedType && r.span.start == annotation),
                "{source}"
            );
            assert!(
                diagnostic
                    .related_spans
                    .iter()
                    .any(|r| r.role == RelatedRole::FoundType
                        && r.span.start == start
                        && r.span.end == start + leaf.width as u64),
                "{source}"
            );
            diagnostic.validate("fixture.n", &source).unwrap();
        }
        leaves_checked += leaves.len();
        if actual.is_empty() {
            accepted += 1;
        } else {
            rejected_count += 1;
        }
    }
    eprintln!(
        "expected-context model: programs=100000 unique=100000 accepted={accepted} rejected={rejected_count} incompatible_leaves={leaves_checked} discrepancies=0"
    );
}

#[test]
fn deferred_never_element_has_unresolved_array_constructor() {
    // §4.1.2.2 and RD127 require one NX-E0103 on [q1].
    let source = "fn Main(){var q1=(if true then {return;} else {return;});var q2=[q1];}";
    let ds = rejected(source);
    assert_eq!(ds.len(), 1, "{ds:?}");
    assert_eq!(ds[0].code, "NX-E0103");
    assert_eq!(
        ds[0].primary_span.as_ref().map(|s| (s.start, s.end)),
        source
            .find("[q1]")
            .map(|start| (start as u64, start as u64 + 4))
    );
}

#[test]
fn expected_context_accepts_matching_independent_operands() {
    // F0-0002 §§4.1.1, 4.7: propagating context must not reject matching
    // branch values, array elements, call arguments or indexed assignments.
    for source in [
        "fn Main(){let x:Bool=(if true then true else (if false then false else true));}",
        "fn Main(){let x:Bool=(if true then (if false then true else false) else true);}",
        "fn Main(){let x:Array<Bool>=[true,false];}",
        "fn F()->Bool{return (if true then true else false);} fn Main(){}",
        "fn F(x:view Bool){} fn Main(){F((if true then true else false));}",
        "fn Main(){var a=[true];a[0]=(if true then false else true);}",
        "fn Main(){var q1=(if true then {return;}else{return;});let q2=[q1];q1=1;}",
        "fn Main(){var q1=(if true then {return;}else{return;});var q2=q1;let q3=[q2];q1=1;}",
    ] {
        assert!(compile_scalar("fixture.n", source).is_ok(), "{source}");
    }
}

#[test]
fn deferred_never_alias_array_rejects_unresolved_element() {
    // §4.1.1 keeps the aliases in one type class; §4.1.2.2 and RD127
    // require one NX-E0103 on [q2].
    let source = "fn Main(){var q1=(if true then {return;}else{return;});var q2=q1;let q3=[q2];}";
    let ds = rejected(source);
    assert_eq!(ds.len(), 1, "{ds:?}");
    assert_eq!(ds[0].code, "NX-E0103");
    let at = source.find("[q2]").unwrap() as u64;
    assert_eq!(
        ds[0].primary_span.as_ref().map(|s| (s.start, s.end)),
        Some((at, at + 4))
    );
}

#[test]
fn unresolved_array_alias_does_not_duplicate_its_causal_error() {
    // F0-0002 §13.9 avoids consequences that depend only on an earlier
    // error: q's [] is unresolved, so [q] must not add a second NX-E0103.
    let source = "fn Main(){var q=[];let b=[q];}";
    let ds = rejected(source);
    assert_eq!(ds.len(), 1, "{ds:?}");
    assert_eq!(ds[0].code, "NX-E0103");
    let at = source.find("[]").unwrap() as u64;
    assert_eq!(
        ds[0].primary_span.as_ref().map(|s| (s.start, s.end)),
        Some((at, at + 2))
    );
}

#[test]
fn failed_constructor_does_not_add_a_dependent_inference_error() {
    // F0-0002 §4.1.2.2,5: the final [] is an independent constructor
    // obligation despite the earlier failed indexed access.
    let source = "fn Main(){var q=(if true then {return;}else{return;});let z=q[0];q=5;q=[];}";
    let ds = rejected(source);
    assert!(
        ds.iter().any(|d| {
            d.code == "NX-E0103"
                && d.primary_span
                    .as_ref()
                    .is_some_and(|s| &source[s.start as usize..s.end as usize] == "[]")
        }),
        "{ds:?}"
    );
}

#[test]
fn deferred_equality_requires_copyable_left_operand() {
    // F0-0002 §4.6 restricts == to copyable data before the later deferred
    // binding can assign the left operand a resource type.
    let source =
        "fn Main(){var a=(if true then {return;}else{return;});let r=MockOpen(1);let c=a==r;}";
    let ds = rejected(source);
    assert_eq!(ds.len(), 1, "{ds:?}");
    assert_eq!(
        (ds[0].code.as_str(), ds[0].subcause.as_deref()),
        ("NX-E0102", Some("type_mismatch"))
    );
}

#[test]
fn deferred_equality_preserves_rhs_literal_origin() {
    // F0-0002 §4.1.2.6: the right operand occurrence fixes the pending
    // left binding; the later assignment relates that actual witness.
    let source = "fn Main(){var b=(if true then {return;}else{return;});b=5;var a=(if true then {return;}else{return;});let c=a==b;a=true;}";
    let ds = rejected(source);
    let witness = source.find("a==b").unwrap() as u64 + 3;
    assert!(
        ds.iter().any(|d| d.related_spans.iter().any(|r| {
            r.role == RelatedRole::ExpectedType
                && r.span.start == witness
                && r.span.end == witness + 1
        })),
        "{ds:?}"
    );
}

#[test]
fn deferred_branch_assignment_retains_previous_origin() {
    // F0-0002 §13.15 preserves the first concrete origin through bind.
    let source =
        "fn Main(){var q0=(if true then {return;}else{return;});q0=(if false then [1] else [q0]);}";
    let ds = rejected(source);
    assert!(
        ds.iter().any(|d| d
            .related_spans
            .iter()
            .any(|r| r.role == RelatedRole::ExpectedType)),
        "{ds:?}"
    );
}

#[test]
fn union_origin_tie_uses_first_child_in_source_order() {
    // F0-0002 §4.1.1: tied constraints use child order.
    let source = "fn Main(){var a=[];var b=[];a=b;a=5;}";
    let ds = rejected(source);
    let first = source.find("[]").unwrap() as u64;
    assert!(
        ds.iter().any(|d| d
            .primary_span
            .as_ref()
            .is_some_and(|s| s.start == first && s.end == first + 2)),
        "{ds:?}"
    );
}

#[test]
fn distinct_constraint_order_pairs_keep_code_and_primary() {
    // F0-0002 §§4.1.1, 13.15: the first incompatible operand in source
    // order is primary. §4.7 assigns NX-E1303 to the later value when an
    // unconstrained if joins incompatible continuations.
    let contexts = [
        "let u=s[0][0]+1;",
        "let u=s[0][0]-1;",
        "let u=s[0][0]<1;",
        "let u=1+s[0][0];",
        "let u=1-s[0][0];",
        "let u=1==s[0][0];",
        "let u=1<s[0][0];",
        "AssertEq(s[0][0],1);",
        "var n=1;n=s[0][0];",
        "let u:I64=s[0][0];",
    ];
    let mut unique = std::collections::HashSet::new();
    for context in contexts {
        let prefix = "fn Main(){var s=[[]];";
        for (left, right, code, left_primary, right_primary) in [
            (
                format!("{prefix}let t=s==true;{context}}}"),
                format!("{prefix}{context}let t=s==true;}}"),
                "NX-E0102",
                "true",
                "true",
            ),
            (
                format!("{prefix}let t=s==true;{context}}}"),
                format!("{prefix}let t=true==s;{context}}}"),
                "NX-E0102",
                "true",
                "s;",
            ),
            (
                format!("{prefix}let t=(if true then s else false);{context}}}"),
                format!("{prefix}let t=(if true then false else s);{context}}}"),
                "NX-E1303",
                "false",
                "s)",
            ),
        ] {
            assert!(unique.insert((left.clone(), right.clone())));
            for (source, primary) in [(&left, left_primary), (&right, right_primary)] {
                let ds = rejected(source);
                assert_eq!(ds.len(), 1, "{source}: {ds:?}");
                assert_eq!(ds[0].code, code, "{source}: {ds:?}");
                let at = source.rfind(primary).unwrap() as u64;
                let (start, end) = if primary == "s;" || primary == "s)" {
                    (at, at + 1)
                } else {
                    (at, at + primary.len() as u64)
                };
                assert_eq!(
                    ds[0].primary_span.as_ref().map(|s| (s.start, s.end)),
                    Some((start, end)),
                    "{source}: {ds:?}"
                );
            }
        }
    }
    assert_eq!(unique.len(), 30);
}

#[test]
fn structurally_distinct_expected_context_pairs_keep_leaf_primary() {
    // F0-0002 §4.7 propagates concrete contexts to value operands; §13.9
    // retains each independent site; §§13.15, 13.17 locate and name the
    // incompatible checked operand. The markers supply literal expectations.
    fn unmark(marked: &str) -> (String, (u64, u64)) {
        let open = marked.find('⟦').unwrap();
        let close = marked.find('⟧').unwrap();
        let leaf = &marked[open + '⟦'.len_utf8()..close];
        let source = marked.replace(['⟦', '⟧'], "");
        (source, (open as u64, (open + leaf.len()) as u64))
    }
    let cases: [(&str, &str, &str, &str, &str); 30] = [
        (
            "fn Main(){let x:Bool=(if true then ⟦11⟧ else true);}",
            "fn Main(){let x:Bool=(if true then true else ⟦11⟧);}",
            "Bool",
            "I64",
            "true",
        ),
        (
            "fn Main(){let x:Bool=(if true then {value:⟦12⟧;} else {value:true;});}",
            "fn Main(){let x:Bool=(if true then {value:true;} else {value:⟦12⟧;});}",
            "Bool",
            "I64",
            "true",
        ),
        (
            "fn Main(){let x:Bool=(if true then (if false then ⟦13⟧ else true) else true);}",
            "fn Main(){let x:Bool=(if true then (if false then true else ⟦13⟧) else true);}",
            "Bool",
            "I64",
            "true",
        ),
        (
            "fn Main(){let x:Bool=(if true then true else (if false then ⟦14⟧ else true));}",
            "fn Main(){let x:Bool=(if true then true else (if false then true else ⟦14⟧));}",
            "Bool",
            "I64",
            "true",
        ),
        (
            "fn Main(){let x:Bool=((if true then ⟦15⟧ else true));}",
            "fn Main(){let x:Bool=((if true then true else ⟦15⟧));}",
            "Bool",
            "I64",
            "true",
        ),
        (
            "fn Main(){let x:Bool=(if true then {value:(⟦16⟧);} else true);}",
            "fn Main(){let x:Bool=(if true then true else {value:(⟦16⟧);});}",
            "Bool",
            "I64",
            "true",
        ),
        (
            "fn Main(){let x:Array<Bool>=[⟦17⟧,true];}",
            "fn Main(){let x:Array<Bool>=[true,⟦17⟧];}",
            "Bool",
            "I64",
            "true",
        ),
        (
            "fn Main(){let x:Array<Array<Bool>>=[[⟦18⟧],[true]];}",
            "fn Main(){let x:Array<Array<Bool>>=[[true],[⟦18⟧]];}",
            "Bool",
            "I64",
            "true",
        ),
        (
            "fn Main(){let x:Array<Bool>=[(if true then ⟦19⟧ else true)];}",
            "fn Main(){let x:Array<Bool>=[(if true then true else ⟦19⟧)];}",
            "Bool",
            "I64",
            "true",
        ),
        (
            "fn Main(){let x:Array<Bool>=(if true then [⟦20⟧] else [true]);}",
            "fn Main(){let x:Array<Bool>=(if true then [true] else [⟦20⟧]);}",
            "Bool",
            "I64",
            "true",
        ),
        (
            "fn Main(){let x:Array<Bool>=([⟦21⟧]);}",
            "fn Main(){let x:Array<Bool>=([(⟦21⟧)]);}",
            "Bool",
            "I64",
            "true",
        ),
        (
            "fn F()->Bool{return ⟦22⟧;} fn Main(){}",
            "fn F()->Bool{return (⟦22⟧);} fn Main(){}",
            "Bool",
            "I64",
            "true",
        ),
        (
            "fn F()->Bool{return (if true then ⟦23⟧ else true);} fn Main(){}",
            "fn F()->Bool{return (if true then true else ⟦23⟧);} fn Main(){}",
            "Bool",
            "I64",
            "true",
        ),
        (
            "fn F()->Array<Bool>{return [⟦24⟧,true];} fn Main(){}",
            "fn F()->Array<Bool>{return [true,⟦24⟧];} fn Main(){}",
            "Bool",
            "I64",
            "true",
        ),
        (
            "fn F(x:view Bool){} fn Main(){F(⟦25⟧);}",
            "fn F(x:view Bool){} fn Main(){F((⟦25⟧));}",
            "Bool",
            "I64",
            "true",
        ),
        (
            "fn F(x:view Bool){} fn Main(){F((if true then ⟦26⟧ else true));}",
            "fn F(x:view Bool){} fn Main(){F((if true then true else ⟦26⟧));}",
            "Bool",
            "I64",
            "true",
        ),
        (
            "fn F(x:view Array<Bool>){} fn Main(){F([⟦27⟧,true]);}",
            "fn F(x:view Array<Bool>){} fn Main(){F([true,⟦27⟧]);}",
            "Bool",
            "I64",
            "true",
        ),
        (
            "fn Main(){var x:Bool=true;x=⟦28⟧;}",
            "fn Main(){var x:Bool=true;x=(⟦28⟧);}",
            "Bool",
            "I64",
            "true",
        ),
        (
            "fn Main(){var x:Bool=true;x=(if true then ⟦29⟧ else true);}",
            "fn Main(){var x:Bool=true;x=(if true then true else ⟦29⟧);}",
            "Bool",
            "I64",
            "true",
        ),
        (
            "fn Main(){var x:Array<Bool>=[true];x=[⟦30⟧];}",
            "fn Main(){var x:Array<Bool>=[true];x=([⟦30⟧]);}",
            "Bool",
            "I64",
            "true",
        ),
        (
            "fn Main(){var a:Array<Bool>=[true];a[0]=⟦31⟧;}",
            "fn Main(){var a:Array<Bool>=[true];a[0]=(⟦31⟧);}",
            "Bool",
            "I64",
            "true",
        ),
        (
            "fn Main(){var a:Array<Bool>=[true];a[0]=(if true then ⟦32⟧ else true);}",
            "fn Main(){var a:Array<Bool>=[true];a[0]=(if true then true else ⟦32⟧);}",
            "Bool",
            "I64",
            "true",
        ),
        (
            "fn Main(){var a:Array<Array<Bool>>=[[true]];a[0][0]=⟦33⟧;}",
            "fn Main(){var a:Array<Array<Bool>>=[[true]];a[0][0]=(⟦33⟧);}",
            "Bool",
            "I64",
            "true",
        ),
        (
            "fn Main(){if ⟦34⟧ {} }",
            "fn Main(){if (⟦34⟧) {} }",
            "Bool",
            "I64",
            "true",
        ),
        (
            "fn Main(){while ⟦35⟧ {} }",
            "fn Main(){while (⟦35⟧) {} }",
            "Bool",
            "I64",
            "true",
        ),
        (
            "fn Main(){let x=⟦true⟧+1;}",
            "fn Main(){let x=1+⟦true⟧;}",
            "I64",
            "Bool",
            "1",
        ),
        (
            "fn Main(){let x=⟦true⟧-1;}",
            "fn Main(){let x=1-⟦true⟧;}",
            "I64",
            "Bool",
            "1",
        ),
        (
            "fn Main(){let a=[1];let x=a[⟦true⟧];}",
            "fn Main(){let a=[1];let x=a[(⟦true⟧)];}",
            "I64",
            "Bool",
            "1",
        ),
        (
            "fn F(a:view I64,b:view I64){} fn Main(){F(a:⟦true⟧,b:1);}",
            "fn F(a:view I64,b:view I64){} fn Main(){F(a:1,b:⟦true⟧);}",
            "I64",
            "Bool",
            "1",
        ),
        (
            "fn Main(){let r=MockOpen(⟦true⟧);}",
            "fn Main(){let r=MockOpen((⟦true⟧));}",
            "I64",
            "Bool",
            "1",
        ),
    ];
    let mut distinct = std::collections::HashSet::new();
    for (index, (left, right, expected, found, repair)) in cases.into_iter().enumerate() {
        assert!(distinct.insert((left, right)), "duplicate pair {index}");
        for marked in [left, right] {
            let (source, span) = unmark(marked);
            let ds = rejected(&source);
            assert_eq!(ds.len(), 1, "pair {index}: {source}: {ds:?}");
            let d = &ds[0];
            assert_eq!(
                (d.code.as_str(), d.subcause.as_deref()),
                ("NX-E0102", Some("type_mismatch")),
                "pair {index}: {source}: {d:?}"
            );
            assert_eq!(
                d.primary_span.as_ref().map(|s| (s.start, s.end)),
                Some(span),
                "pair {index}: {source}: {d:?}"
            );
            assert_eq!(
                d.data.get("expected_type"),
                Some(&crate::diagnostic::DataValue::String(expected.into())),
                "pair {index}: {source}: {d:?}"
            );
            assert_eq!(
                d.data.get("found_type"),
                Some(&crate::diagnostic::DataValue::String(found.into())),
                "pair {index}: {source}: {d:?}"
            );
            d.validate("fixture.n", &source).unwrap();
            let mut positive = source.clone();
            positive.replace_range(span.0 as usize..span.1 as usize, repair);
            assert!(
                compile_scalar("fixture.n", &positive).is_ok(),
                "pair {index}: {positive}"
            );
        }
    }
    assert_eq!(distinct.len(), 30);
}

#[test]
fn expected_scalar_checks_each_concrete_array_branch() {
    // F0-0002 §4.1.2.2,4: both branches receive I64; [] remains
    // indeterminate and owns U, while [1] has a serializable mismatch.
    for source in [
        "fn Main(){var v0:I64=(if true then [] else [1]);}",
        "fn Main(){var v0:I64=(if true then [1] else []);}",
    ] {
        let ds = rejected(source);
        assert_eq!(ds.len(), 2, "{source}: {ds:?}");
        let mut sites =
            ["[]", "[1]"].map(|text| (source.find(text).unwrap() as u64, text.len() as u64));
        sites.sort();
        for (diagnostic, (at, len)) in ds.iter().zip(sites) {
            let empty = &source[at as usize..(at + len) as usize] == "[]";
            assert_eq!(diagnostic.code, if empty { "NX-E0103" } else { "NX-E0102" });
            assert_eq!(
                diagnostic.primary_span.as_ref().map(|s| (s.start, s.end)),
                Some((at, at + len))
            );
            if empty {
                assert!(diagnostic.data.is_empty());
            } else {
                assert_eq!(
                    diagnostic.data.get("expected_type"),
                    Some(&crate::diagnostic::DataValue::String("I64".into()))
                );
                assert_eq!(
                    diagnostic.data.get("found_type"),
                    Some(&crate::diagnostic::DataValue::String("Array<I64>".into()))
                );
            }
        }
    }
}

#[test]
fn array_conflicts_report_the_complete_expression_type() {
    use crate::diagnostic::DataValue;
    for (source, wanted, found) in [
        (
            "fn Main(){let a=[false]==[1];}",
            "Array<Bool>",
            "Array<I64>",
        ),
        (
            "fn Main(){var v1=[false];var v2=[1];v2=v1;}",
            "Array<I64>",
            "Array<Bool>",
        ),
        (
            "fn Main(){var v1=[false];var v2=[1];AssertEq(v1,v2);}",
            "Array<Bool>",
            "Array<I64>",
        ),
        (
            "fn Main(){let v1=[1];let x:Array<Bool>=v1;}",
            "Array<Bool>",
            "Array<I64>",
        ),
        (
            "fn F(x:view Array<Bool>){} fn Main(){let a=[1];F(a);}",
            "Array<Bool>",
            "Array<I64>",
        ),
        (
            "fn F()->Array<Bool>{let a=[1];return a;} fn Main(){}",
            "Array<Bool>",
            "Array<I64>",
        ),
        (
            "fn Main(){let a=[false];let b=[1];let x=(if true then a else b);}",
            "Array<Bool>",
            "Array<I64>",
        ),
    ] {
        let errors = rejected(source);
        assert_eq!(errors.len(), 1, "{source}: {errors:?}");
        let error = &errors[0];
        assert!(matches!(error.code.as_str(), "NX-E0102" | "NX-E1303"));
        assert_eq!(
            error.data.get("expected_type"),
            Some(&DataValue::String(wanted.into())),
            "{source}: {errors:?}"
        );
        assert_eq!(
            error.data.get("found_type"),
            Some(&DataValue::String(found.into())),
            "{source}: {errors:?}"
        );
    }
}

#[test]
fn array_equality_lowers_both_operands_as_borrows() {
    use super::model::Op;
    let source = "fn Main(){let a=[1,2];let b=[1,2];let equal=a==b;Print(equal);}";
    let program = compile_scalar("fixture.n", source).unwrap();
    let function = program.functions().function("Main").unwrap();
    let borrowed: Vec<_> = std::iter::once(&function.block)
        .chain(function.blocks.iter())
        .flat_map(|block| &block.instructions)
        .filter_map(|instruction| match instruction.op {
            Op::BorrowValue(local)
                if matches!(function.locals[local.0].name.as_str(), "a" | "b") =>
            {
                Some(function.locals[local.0].name.as_str())
            }
            _ => None,
        })
        .collect();
    assert_eq!(borrowed, ["a", "b"]);
    assert_eq!(
        crate::interp::run_scalar(&program).observations.stdout(),
        vec!["true"]
    );
}

#[test]
fn function_values_in_nested_expression_positions_reject_before_lowering() {
    for source in [
        "fn Main(){var v0=0;v0=(if false then Main else v0);v0=0;}",
        "fn Main(){var v=[1];v[Main]=1;}",
        "fn F(){} fn Main(){var v=0;v=(if true then F else v);}",
        "fn Main(){let v=[Main];}",
        "fn Main(){let v=1+Main;}",
    ] {
        let errors = rejected(source);
        assert_eq!(errors.len(), 1, "{source}: {errors:?}");
        assert_eq!(errors[0].code, "NX-E0006", "{source}: {errors:?}");
        assert_eq!(errors[0].feature_id.as_deref(), Some("function_value"));
        errors[0].validate("fixture.n", source).unwrap();
    }
}

#[test]
fn non_value_names_are_rejected_in_every_expression_context() {
    let names = ["Main", "I64", "MockResource", "Print", "AssertEq"];
    let contexts = [
        "let x=@;",
        "let x=(@);",
        "let x=[@];",
        "let x=1+@;",
        "let x=@==1;",
        "let x=if true then @ else 1;",
        "let x=if @ then 1 else 2;",
        "var a=[1];a[@]=2;",
        "var a=[1];let x=a[@];",
        "Print(@);",
        "return @;",
    ];
    for name in names {
        for context in contexts {
            let source = format!("fn Main(){{{}}}", context.replace('@', name));
            let outcome = compile_scalar("fixture.n", &source);
            assert!(
                !matches!(outcome, Err(CompileError::InvalidIr(_))),
                "{source}: {outcome:?}"
            );
            assert!(
                matches!(outcome, Err(CompileError::Rejected(_))),
                "{source}: {outcome:?}"
            );
            if matches!(name, "Main" | "Print" | "AssertEq") && context != "return @;" {
                let Err(CompileError::Rejected(errors)) = outcome else {
                    unreachable!("the rejection was checked above")
                };
                assert!(
                    errors.iter().any(|error| {
                        error.code == "NX-E0006"
                            && error.feature_id.as_deref() == Some("function_value")
                    }),
                    "{source}: {errors:?}"
                );
            }
        }
    }
}

#[test]
fn expected_type_origins_survive_binding_group_index_and_union_paths() {
    use crate::diagnostic::DataValue;
    for (source, expected_origin) in [
        (
            "fn Main(){var a=[];var b=[];b=[[1]];a=b;a=[[true]];}",
            (30, 35),
        ),
        (
            "fn Main(){var a=[];var c=(if true then (if false then [1] else a) else a);c=[true];}",
            (26, 72),
        ),
        (
            "fn Main(){var a=[];var b=[];a=[b];b=[1];AssertEq(a[0],[true]);}",
            // §4.1.2.6: the first AssertEq operand is the real witness.
            (49, 53),
        ),
        (
            "fn Main(){var a=(if true then {return;}else{return;});AssertEq(a,1);a=true;}",
            (65, 66),
        ),
    ] {
        let ds = rejected(source);
        assert_eq!(ds.len(), 1, "{source}: {ds:?}");
        let d = &ds[0];
        assert!(
            d.related_spans
                .iter()
                .any(|r| r.role == RelatedRole::ExpectedType
                    && (r.span.start, r.span.end) == expected_origin),
            "{source}: {ds:?}"
        );
        assert!(
            matches!(d.code.as_str(), "NX-E0102" | "NX-E0103" | "NX-E1303"),
            "{source}: {ds:?}"
        );
        if d.code != "NX-E0103" {
            assert!(matches!(
                d.data.get("expected_type"),
                Some(DataValue::String(_))
            ));
            assert!(matches!(
                d.data.get("found_type"),
                Some(DataValue::String(_))
            ));
        }
    }
    let resource = "fn Main(){var a=[];var m=MockOpen(1);var b=[];b=[m];AssertEq(a,b);}";
    let ds = rejected(resource);
    assert_eq!(ds.len(), 1, "{ds:?}");
    assert_eq!(ds[0].code, "NX-E0006");
    assert_eq!(ds[0].feature_id.as_deref(), Some("resource_array"));
}

#[test]
fn late_scalar_projection_keeps_the_selector_domain_site() {
    use crate::diagnostic::DataValue;
    // §4.1.2.3: the later scalar element fixes the projection base;
    // the earlier selector's domain fails without an expected witness.
    let source = "fn Main(){var a=[];a[0][0]=1;a[0]=1;}";
    let ds = rejected(source);
    assert_eq!(ds.len(), 1, "{ds:?}");
    assert_eq!(
        (ds[0].code.as_str(), ds[0].subcause.as_deref()),
        ("NX-E0102", Some("not_indexable"))
    );
    assert_eq!(
        ds[0].primary_span.as_ref().map(|s| (s.start, s.end)),
        Some((19, 23))
    );
    assert_eq!(
        ds[0].data.get("expected_type"),
        Some(&DataValue::String("domain:array".into()))
    );
    assert_eq!(
        ds[0].data.get("found_type"),
        Some(&DataValue::String("I64".into()))
    );
    assert!(
        ds[0]
            .related_spans
            .iter()
            .all(|r| r.role != RelatedRole::ExpectedType)
    );
}

#[test]
fn all_never_array_elements_reject_without_inventing_an_element_type() {
    // ADR §§4.1.1 and 4.1.2.2 require one NX-E0103 per constructor whose
    // element remains unresolved; RD127 fixes the Never-only example.
    let never = "(if true then {return;}else{return;})";
    for source in [
        format!("fn Main(){{var a=0;a=[{never}];}}"),
        format!("fn Main(){{let x:I64=[{never}];}}"),
        format!("fn Main(){{var v=[{never}];v=3;}}"),
        format!("fn Main(){{var v=[[{never}]];v[0]=3;}}"),
    ] {
        let errors = rejected(&source);
        assert_eq!(errors.len(), 1, "{source}: {errors:?}");
        let error = &errors[0];
        assert_eq!(error.code, "NX-E0103", "{source}: {errors:?}");
        error.validate("fixture.n", &source).unwrap();
        let concrete = source.replace(never, "1");
        let twin = rejected(&concrete);
        assert_eq!(twin.len(), 1, "{concrete}: {twin:?}");
        assert_eq!(twin[0].code, "NX-E0102", "{concrete}: {twin:?}");
    }
}

#[test]
fn nested_array_conflicts_are_source_rejections() {
    for (index, source) in [
        "fn Main(){let a=([1]==[0,true]);}",
        "fn Main(){var v=DV;v=[4];v=[v[0],[v]];}",
        "fn Main(){var v=DV;v=[v[0],[v]];v=[4];}",
    ]
    .into_iter()
    .enumerate()
    {
        let source = source.replace("DV", "(if true then {return;}else{return;})");
        let errors = rejected(&source);
        if index == 0 {
            assert_eq!(errors.len(), 1, "{errors:?}");
            assert_eq!(errors[0].code, "NX-E0102");
            assert_eq!(errors[0].subcause.as_deref(), Some("type_mismatch"));
        }
        assert!(
            errors.iter().all(|d| d.code.starts_with("NX-E01")),
            "{source}: {errors:?}"
        );
        for error in errors {
            error.validate("fixture.n", &source).unwrap();
        }
    }
}

#[test]
fn divergent_array_elements_retain_the_array_constraint() {
    for source in [
        "fn Main(){var v=DV;v=[false,DV];v=[4];}",
        "fn Main(){var v=[false,DV];v=[4];}",
        "fn Main(){var a=0;a=[0,DV];}",
    ] {
        let source = source.replace("DV", "(if true then {return;}else{return;})");
        let errors = rejected(&source);
        assert_eq!(errors.len(), 1, "{source}: {errors:?}");
        assert_eq!(errors[0].code, "NX-E0102", "{source}: {errors:?}");
        assert_eq!(errors[0].subcause.as_deref(), Some("type_mismatch"));
        errors[0].validate("fixture.n", &source).unwrap();
    }
}

#[test]
fn deferred_binding_with_concrete_constraint_checks_branch_context() {
    let divergence = "(if true then {return;}else{return;})";
    let deferred =
        format!("fn Main(){{var v0={divergence};v0=3;v0=(if false then true else v0);}}");
    let concrete = "fn Main(){var v0=3;v0=(if false then true else v0);}";
    let a = rejected(&deferred);
    let b = rejected(concrete);
    assert_eq!(a.len(), 1, "{a:?}");
    assert_eq!(b.len(), 1, "{b:?}");
    assert_eq!(
        (a[0].code.as_str(), a[0].subcause.as_deref()),
        ("NX-E0102", Some("type_mismatch"))
    );
    assert_eq!(
        (a[0].code.as_str(), a[0].subcause.as_deref(), &a[0].data),
        (b[0].code.as_str(), b[0].subcause.as_deref(), &b[0].data)
    );
}

#[test]
fn adr_gap_nested_empty_arrays_resolve_from_outer_branch_in_both_orders() {
    // ADR §4.1.1 unifies branch constraints transitively; §13.17 emits
    // NX-E0103 only while the element remains unresolved. The current
    // result accepts both branch orders after [1] fixes the element type.
    let first = "fn Main(){let t=(if true then [1] else (if false then [] else []));}";
    let second = "fn Main(){let t=(if true then (if false then [] else []) else [1]);}";
    for source in [first, second] {
        let outcome = compile_scalar("fixture.n", source);
        assert!(outcome.is_ok(), "{source}: {outcome:?}");
    }
}

#[test]
fn branch_permutation_rejects_scalar_array_conflicts() {
    let dv = "(if true then {return;}else{return;})";
    let first = format!(
        "fn Main(){{var v0={dv};var v3={dv};v0=(if true then (if true then {dv} else 2) else (if false then [v3] else {dv}));}}"
    );
    let second = format!(
        "fn Main(){{var v0={dv};var v3={dv};v0=(if true then (if false then [v3] else {dv}) else (if true then {dv} else 2));}}"
    );
    for source in [first, second] {
        let errors = rejected(&source);
        assert!(!errors.is_empty(), "{source}");
    }
}

#[test]
fn failed_array_constructor_follows_its_union_class() {
    let dv = "(if true then {return;}else{return;})";
    for source in [
        format!("fn Main(){{var a=[];a=3;var b={dv};b=a;}}"),
        format!("fn Main(){{var b={dv};var a=[];a=3;b=a;}}"),
    ] {
        let errors = rejected(&source);
        assert_eq!(errors.len(), 1, "{source}: {errors:?}");
        assert_eq!(errors[0].code, "NX-E0103", "{source}: {errors:?}");
    }
}

#[test]
fn nested_empty_array_element_remains_resolvable_after_index_write() {
    for depth in 1..=5 {
        let mut source = "fn Main(){var v0=[];".to_owned();
        for level in 1..=depth {
            let place = format!("v0{}", "[0]".repeat(level));
            source.push_str(&format!("{place}=[];"));
        }
        source.push_str(&format!("v0{}=[3];}}", "[0]".repeat(depth)));
        assert!(compile_scalar("fixture.n", &source).is_ok(), "{source}");
    }
}

#[test]
fn divergent_branch_constraints_do_not_create_unresolved_array_errors() {
    let dv = "(if true then {return;}else{return;})";
    for (a, b, expected) in [
        (dv, dv, &["NX-E0103"][..]),
        (dv, "0", &["NX-E0103"][..]),
        (dv, "[0]", &["NX-E0102"][..]),
        ("0", dv, &["NX-E0102"][..]),
        ("0", "0", &["NX-E1303"][..]),
        ("0", "[0]", &["NX-E0102"][..]),
        ("[0]", dv, &["NX-E0102"][..]),
        ("[0]", "0", &["NX-E1303"][..]),
        ("[0]", "[0]", &["NX-E1303", "NX-E1303"][..]),
    ] {
        let source = format!(
            "fn Main(){{var v0={a};var v1={dv};var v2={b};v1=(if (if true then true else false) then v2 else [{dv},v0]);v1=(if true then v2 else (if true then 3 else v1));}}"
        );
        let errors = rejected(&source);
        let codes: Vec<_> = errors.iter().map(|d| d.code.as_str()).collect();
        assert_eq!(codes, expected, "{source}: {errors:?}");
    }
}

#[test]
#[ignore = "300000 generated sources with branch, equality, array, declaration and alias permutations"]
fn generated_type_permutations_preserve_acceptance() {
    use std::hash::{Hash, Hasher};
    let count = std::env::var("NEXO_PERM_CASES")
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(300_000);
    let dv = "(if true then {return;}else{return;})";
    let values = ["0", "true", "[0]", "[]", dv];
    let mut random = 0x6a09e667f3bcc909_u64;
    let mut permutation_sources = std::collections::HashSet::new();
    for case in 0..count {
        random ^= random << 13;
        random ^= random >> 7;
        random ^= random << 17;
        let left = values[(random as usize) % values.len()];
        let right = values[((random >> 16) as usize) % values.len()];
        let (a, b) = match case % 5 {
            0 => (
                format!("fn Main(){{let t=(if true then {left} else {right});}}"),
                format!("fn Main(){{let t=(if true then {right} else {left});}}"),
            ),
            1 => (
                format!("fn Main(){{let t={left}=={right};}}"),
                format!("fn Main(){{let t={right}=={left};}}"),
            ),
            2 => (
                format!("fn Main(){{let t=[{left},{right}];}}"),
                format!("fn Main(){{let t=[{right},{left}];}}"),
            ),
            3 => (
                format!("fn Main(){{var a={dv};var b={dv};a=b;a={left};b={right};}}"),
                format!("fn Main(){{var a={dv};var b={dv};a=b;b={right};a={left};}}"),
            ),
            _ => (
                format!("fn Main(){{var a={dv};var b={dv};a=b;a={left};b={right};}}"),
                format!("fn Main(){{var b={dv};var a={dv};a=b;a={left};b={right};}}"),
            ),
        };
        for source in [&a, &b] {
            let mut hasher = std::collections::hash_map::DefaultHasher::new();
            source.hash(&mut hasher);
            permutation_sources.insert(hasher.finish());
        }
        let first = compile_scalar("permutation.n", &a);
        let second = compile_scalar("permutation.n", &b);
        assert!(
            !matches!(first, Err(CompileError::InvalidIr(_)))
                && !matches!(second, Err(CompileError::InvalidIr(_))),
            "case {case}: {a}: {first:?} / {b}: {second:?}"
        );
        assert_eq!(
            first.is_ok(),
            second.is_ok(),
            "case {case}: {a}: {first:?} / {b}: {second:?}"
        );
        if let (Err(CompileError::Rejected(left)), Err(CompileError::Rejected(right))) =
            (&first, &second)
        {
            assert_no_duplicate_diagnostics(left, &a);
            assert_no_duplicate_diagnostics(right, &b);
            assert_eq!(
                left.len(),
                right.len(),
                "case {case}: {a}: {left:?} / {b}: {right:?}"
            );
            for (l, r) in left.iter().zip(right.iter()) {
                assert_eq!(
                    (&l.code, &l.subcause, l.stage, l.operation),
                    (&r.code, &r.subcause, r.stage, r.operation),
                    "case {case}: {a}: {left:?} / {b}: {right:?}"
                );
                let l_keys: Vec<_> = l.data.keys().collect();
                let r_keys: Vec<_> = r.data.keys().collect();
                assert_eq!(
                    l_keys, r_keys,
                    "case {case}: {a}: {left:?} / {b}: {right:?}"
                );
                for key in l_keys {
                    if key != "expected_type" && key != "found_type" {
                        assert_eq!(
                            l.data[key], r.data[key],
                            "case {case}: {a}: {left:?} / {b}: {right:?}"
                        );
                    }
                }
                if l.data != r.data {
                    assert_eq!(
                        l.data.get("expected_type"),
                        r.data.get("found_type"),
                        "case {case}: {a}: {left:?} / {b}: {right:?}"
                    );
                    assert_eq!(
                        l.data.get("found_type"),
                        r.data.get("expected_type"),
                        "case {case}: {a}: {left:?} / {b}: {right:?}"
                    );
                }
                let l_roles: Vec<_> = l.related_spans.iter().map(|s| s.role).collect();
                let r_roles: Vec<_> = r.related_spans.iter().map(|s| s.role).collect();
                assert_eq!(
                    l_roles, r_roles,
                    "case {case}: {a}: {left:?} / {b}: {right:?}"
                );
                if l.data == r.data && l.code != "NX-E0103" {
                    fn text<'a>(source: &'a str, span: &crate::diagnostic::Span) -> &'a str {
                        &source[span.start as usize..span.end as usize]
                    }
                    assert_eq!(
                        l.primary_span.as_ref().map(|s| text(&a, s)),
                        r.primary_span.as_ref().map(|s| text(&b, s)),
                        "case {case}: {a}: {left:?} / {b}: {right:?}"
                    );
                    for (ls, rs) in l.related_spans.iter().zip(r.related_spans.iter()) {
                        assert_eq!(
                            text(&a, &ls.span),
                            text(&b, &rs.span),
                            "case {case}: {a}: {left:?} / {b}: {right:?}"
                        );
                    }
                }
                l.validate("permutation.n", &a).unwrap();
                r.validate("permutation.n", &b).unwrap();
            }
        }
    }
    if count == 300_000 {
        assert!(
            permutation_sources.len() >= 150,
            "only {} unique permutation sources",
            permutation_sources.len()
        );
    }
    eprintln!("unique permutation sources: {}", permutation_sources.len());
    // The graph generator below varies actual constraint topology and checks
    // each source against a separate AST unifier, beyond the small fixed
    // set of expression permutations above.
    let unique = run_constraint_oracle(50_100, 0);
    assert!(
        unique.len() >= 100_200,
        "only {} unique constraint programs",
        unique.len()
    );
    eprintln!("unique constraint programs: {}", unique.len());
}

#[test]
fn inferred_array_depth_has_a_deterministic_analysis_limit() {
    use crate::diagnostic::DataValue;
    let mut source = String::from("fn Main(){var v=(if true then {return;}else{return;});");
    for depth in 0..129 {
        let previous = if depth == 0 {
            "v".to_owned()
        } else {
            format!("a{}", depth - 1)
        };
        source.push_str(&format!("var a{depth}=[{previous}];"));
    }
    source.push('}');
    let ds = rejected(&source);
    assert_eq!(ds.len(), 1, "{ds:?}");
    let d = &ds[0];
    assert_eq!(
        (d.code.as_str(), d.stage, d.operation),
        ("NX-E0003", Stage::Types, Operation::AnalysisLimit),
        "{d:?}"
    );
    assert_eq!(
        d.data.get("budget_kind"),
        Some(&DataValue::String("array_type_depth".into()))
    );
    assert_eq!(
        d.data.get("budget_scope"),
        Some(&DataValue::String("compilation".into()))
    );
    assert_eq!(
        d.data.get("incomplete_analysis"),
        Some(&DataValue::String("types".into()))
    );
    assert_eq!(d.data.get("limit"), Some(&DataValue::UInt(128)));
    assert_eq!(d.data.get("observed"), Some(&DataValue::UInt(129)));
    d.validate("fixture.n", &source).unwrap();

    let mut after_limit = source.clone();
    after_limit.pop();
    after_limit.push_str("let z=true+1;}");
    let stopped = rejected(&after_limit);
    assert_eq!(stopped.len(), 1, "{stopped:?}");
    assert_eq!(stopped[0].code, "NX-E0003");

    let mut mixed = String::from("fn Main(){var v=(if true then {return;}else{return;});");
    for depth in 0..100 {
        let previous = if depth == 0 {
            "v".to_owned()
        } else {
            format!("a{}", depth - 1)
        };
        mixed.push_str(&format!("var a{depth}=[{previous}];"));
    }
    mixed.push_str("v=");
    mixed.push_str(&"[".repeat(50));
    mixed.push('1');
    mixed.push_str(&"]".repeat(50));
    mixed.push_str(";}");
    let ds = rejected(&mixed);
    assert_eq!(ds.len(), 1, "{ds:?}");
    assert_eq!(ds[0].code, "NX-E0003");
    assert_eq!(
        ds[0].data.get("budget_kind"),
        Some(&DataValue::String("array_type_depth".into()))
    );
}

#[test]
fn late_array_constructor_obeys_the_inferred_root_depth_limit() {
    use crate::diagnostic::DataValue;
    // F0-0002 §4.1.2.3: selectors on a pending base do not create
    // constructors. Only the later explicit arrays add wrappers.
    // §13.13 rejects the 129th array wrapper on that inferred root.
    let source = include_str!("../../tests/fixtures/i07c/inferred_array_root_late_constraint.n");
    check_scalar("fixture.n", source).unwrap();
    compile_scalar("fixture.n", source).unwrap();
    let valid = source.replace("z=a;", "v=[[a]];");
    check_scalar("fixture.n", &valid).unwrap();
    compile_scalar("fixture.n", &valid).unwrap();
    let invalid = source.replace("z=a;", "v=[[[a]]];");

    let checked = match check_scalar("fixture.n", &invalid) {
        Err(CompileError::Rejected(errors)) => errors,
        other => panic!("expected type depth rejection: {other:?}"),
    };
    let compiled = rejected(&invalid);
    for diagnostics in [
        checked.iter().map(|d| d.as_ref()).collect::<Vec<_>>(),
        compiled.iter().collect::<Vec<_>>(),
    ] {
        assert_eq!(diagnostics.len(), 1);
        let d = diagnostics[0];
        assert_eq!((d.code.as_str(), d.subcause.as_deref()), ("NX-E0003", None));
        assert_eq!(
            (d.stage, d.operation),
            (Stage::Types, Operation::AnalysisLimit)
        );
        assert_eq!(
            d.primary_span.as_ref().map(|s| (s.start, s.end)),
            Some((
                invalid.find("[[[a]]]").unwrap() as u64,
                invalid.find("[[[a]]]").unwrap() as u64 + 7
            ))
        );
        assert_eq!(
            &invalid[d.primary_span.as_ref().unwrap().start as usize
                ..d.primary_span.as_ref().unwrap().end as usize],
            "[[[a]]]"
        );
        assert_eq!(d.data.len(), 5);
        assert_eq!(
            d.data.get("budget_kind"),
            Some(&DataValue::String("array_type_depth".into()))
        );
        assert_eq!(
            d.data.get("budget_scope"),
            Some(&DataValue::String("compilation".into()))
        );
        assert_eq!(
            d.data.get("incomplete_analysis"),
            Some(&DataValue::String("types".into()))
        );
        assert_eq!(d.data.get("limit"), Some(&DataValue::UInt(128)));
        assert_eq!(d.data.get("observed"), Some(&DataValue::UInt(129)));
        assert!(d.related_spans.is_empty() && d.cause_chain.is_empty() && d.repair.is_empty());
        d.validate("fixture.n", &invalid).unwrap();
    }
}

#[test]
fn unresolved_array_constructor_keeps_its_cause_without_a_cascade() {
    for (source, code) in [
        ("fn Main(){let a:I64=[];}", "NX-E0103"),
        ("fn Main(){var a=[];a=true;}", "NX-E0103"),
    ] {
        let ds = rejected(source);
        assert_eq!(ds.len(), 1, "{source}: {ds:?}");
        assert_eq!(ds[0].code, code, "{source}: {ds:?}");
    }
    // The later array expression fails independently. It cannot discharge
    // the earlier [] constructor's own unresolved element (§4.1.2.2).
    let failed_rhs = "fn Main(){var a=[];a=[true+1];}";
    let ds = rejected(failed_rhs);
    assert_eq!(ds.len(), 2, "{ds:?}");
    assert_eq!(ds[0].code, "NX-E0103");
    assert_eq!(
        ds[0].primary_span.as_ref().map(|s| (s.start, s.end)),
        Some((16, 18))
    );
    assert_eq!(
        (ds[1].code.as_str(), ds[1].subcause.as_deref()),
        ("NX-E0102", Some("type_mismatch"))
    );
    assert_eq!(
        ds[1].primary_span.as_ref().map(|s| (s.start, s.end)),
        Some((22, 26))
    );
    let unrelated = rejected("fn Main(){var a=[];let z=true+1;}");
    assert_eq!(unrelated.len(), 2, "{unrelated:?}");
    assert_eq!(
        unrelated
            .iter()
            .map(|d| d.code.as_str())
            .collect::<Vec<_>>(),
        ["NX-E0103", "NX-E0102"]
    );
    let across_functions = "fn F(){var a=[];}fn Main(){}";
    let ds = rejected(across_functions);
    assert_eq!(ds.len(), 1, "{ds:?}");
    assert_eq!(ds[0].code, "NX-E0103");
}

#[test]
fn unavailable_index_root_precedes_derived_immutable_write() {
    let source = "fn Main(){let a=[0];Drop(a);a[0]=5;}";
    let ds = rejected(source);
    assert_eq!(ds.len(), 1, "{ds:?}");
    let d = &ds[0];
    assert_eq!(
        (d.code.as_str(), d.subcause.as_deref()),
        ("NX-E0202", Some("definitely_consumed"))
    );
    assert_eq!(
        (d.stage, d.operation),
        (Stage::Ownership, Operation::OwnershipUse)
    );
    assert_eq!(
        (
            d.primary_span.as_ref().unwrap().start,
            d.primary_span.as_ref().unwrap().end
        ),
        (28, 29)
    );
    assert!(d.data.is_empty());
    assert_eq!(
        d.related_spans
            .iter()
            .map(|r| (r.role, r.span.start, r.span.end))
            .collect::<Vec<_>>(),
        vec![
            (RelatedRole::Declaration, 14, 15),
            (RelatedRole::Consumption, 20, 27)
        ]
    );
    assert_eq!(d.cause_chain.len(), 1);
    assert_eq!(
        (d.cause_chain[0].kind, d.cause_chain[0].operation),
        (CauseKind::Drop, CauseOperation::ValueDrop)
    );
    assert_eq!(
        d.cause_chain[0].span.as_ref().map(|s| (s.start, s.end)),
        Some((20, 27))
    );
    assert_eq!(
        d.cause_chain[0].entity.as_ref().map(|e| e.id.as_str()),
        Some("binding:14")
    );
    assert_eq!(d.cause_chain[0].evidence, Evidence::Verified);
    d.validate("fixture.n", source).unwrap();
    let with_loop = rejected("fn Main(){let a=[0];Drop(a);a[0]=5;while false{}}");
    assert_eq!(with_loop.len(), 1, "{with_loop:?}");
    assert_eq!(with_loop[0].code, "NX-E0202");
    compile_scalar("fixture.n", "fn Main(){var a=[0];a[0]=5;}").unwrap();
}

#[test]
fn complete_replacement_after_drop_still_checks_immutability() {
    let ds = rejected("fn Main(){let a=[0];Drop(a);a=[1];}");
    assert_eq!(ds.len(), 1, "{ds:?}");
    let d = &ds[0];
    assert_eq!((d.code.as_str(), d.subcause.as_deref()), ("NX-E0105", None));
    assert_eq!(
        (d.stage, d.operation),
        (Stage::Ownership, Operation::PlaceWrite)
    );
    assert_eq!(
        (
            d.primary_span.as_ref().unwrap().start,
            d.primary_span.as_ref().unwrap().end
        ),
        (28, 29)
    );
    assert_eq!(
        d.related_spans
            .iter()
            .map(|r| (r.role, r.span.start, r.span.end))
            .collect::<Vec<_>>(),
        vec![(RelatedRole::Declaration, 14, 15)]
    );
    assert!(d.data.is_empty() && d.cause_chain.is_empty());
    d.validate("fixture.n", "fn Main(){let a=[0];Drop(a);a=[1];}")
        .unwrap();
    compile_scalar("fixture.n", "fn Main(){var a=[0];Drop(a);a=[1];}").unwrap();
    let looped = rejected("fn Main(){let a=[0];a[0]=1;while false{}}");
    assert_eq!(looped.len(), 1, "{looped:?}");
    assert_eq!(looped[0].code, "NX-E0105");
}

#[test]
fn indexed_write_retains_immutable_root_with_divergent_children() {
    for source in [
        "fn Main(){let a=[1];a[0]=(if true then {return;} else {return;});}",
        "fn Main(){let a=[1];a[(if true then {return;} else {return;})]=2;}",
        "fn Main(){let a=[1];a[DV]=2;}",
    ] {
        // DV is the literal divergent expression used throughout the ADR.
        let source = source.replace("DV", "(if true then {return;} else {return;})");
        let ds = rejected(&source);
        assert!(
            ds.iter().any(|d| d.code == "NX-E0105"
                && d.primary_span
                    .as_ref()
                    .is_some_and(|s| &source[s.start as usize..s.end as usize] == "a")),
            "{source}: {ds:?}"
        );
    }
    compile_scalar(
        "fixture.n",
        "fn Main(){var a=[1];a[0]=(if true then {return;} else {return;});}",
    )
    .unwrap();
}

#[test]
fn contaminated_comparison_sibling_keeps_left_numeric_domain() {
    let source =
        "fn Main(){var q=(if true then {return;} else {return;});let y=q<(true+1);q=true;}";
    let ds = rejected(source);
    let bad_operands = [
        source.find("true+1").unwrap(),
        source.rfind("true").unwrap(),
    ];
    assert_eq!(ds.len(), 2, "{ds:?}");
    for (d, start) in ds.iter().zip(bad_operands) {
        assert_eq!(
            (d.code.as_str(), d.subcause.as_deref()),
            ("NX-E0102", Some("type_mismatch"))
        );
        assert_eq!(
            d.primary_span.as_ref().map(|s| (s.start, s.end)),
            Some((start as u64, start as u64 + 4))
        );
        assert_eq!(
            d.data.get("expected_type"),
            Some(&crate::diagnostic::DataValue::String("I64".into()))
        );
        assert_eq!(
            d.data.get("found_type"),
            Some(&crate::diagnostic::DataValue::String("Bool".into()))
        );
    }
    compile_scalar(
        "fixture.n",
        "fn Main(){var q=(if true then {return;} else {return;});let y=q<(1+1);q=2;}",
    )
    .unwrap();
}

#[test]
fn independent_consumption_and_immutable_write_both_survive() {
    let ds = rejected("fn Main(){let a=[0];let b=0;Drop(a);Print(a);b=1;}");
    assert_eq!(ds.len(), 2, "{ds:?}");
    assert_eq!(
        (ds[0].code.as_str(), ds[0].subcause.as_deref()),
        ("NX-E0202", Some("definitely_consumed"))
    );
    assert_eq!(
        (ds[0].stage, ds[0].operation),
        (Stage::Ownership, Operation::OwnershipUse)
    );
    assert_eq!(
        (
            ds[0].primary_span.as_ref().unwrap().start,
            ds[0].primary_span.as_ref().unwrap().end
        ),
        (42, 43)
    );
    assert_eq!(
        ds[0]
            .related_spans
            .iter()
            .map(|r| (r.role, r.span.start, r.span.end))
            .collect::<Vec<_>>(),
        vec![
            (RelatedRole::Declaration, 14, 15),
            (RelatedRole::Consumption, 28, 35)
        ]
    );
    assert_eq!(
        (ds[0].cause_chain[0].kind, ds[0].cause_chain[0].operation),
        (CauseKind::Drop, CauseOperation::ValueDrop)
    );
    assert_eq!(
        ds[0].cause_chain[0].span.as_ref().map(|s| (s.start, s.end)),
        Some((28, 35))
    );
    assert_eq!(
        ds[0].cause_chain[0].entity.as_ref().map(|e| e.id.as_str()),
        Some("binding:14")
    );
    assert_eq!(ds[0].cause_chain[0].evidence, Evidence::Verified);
    assert_eq!(
        (ds[1].code.as_str(), ds[1].subcause.as_deref()),
        ("NX-E0105", None)
    );
    assert_eq!(
        (ds[1].stage, ds[1].operation),
        (Stage::Ownership, Operation::PlaceWrite)
    );
    assert_eq!(
        (
            ds[1].primary_span.as_ref().unwrap().start,
            ds[1].primary_span.as_ref().unwrap().end
        ),
        (45, 46)
    );
    assert_eq!(
        ds[1]
            .related_spans
            .iter()
            .map(|r| (r.role, r.span.start, r.span.end))
            .collect::<Vec<_>>(),
        vec![(RelatedRole::Declaration, 24, 25)]
    );
    assert!(ds[1].data.is_empty() && ds[1].cause_chain.is_empty());
    for d in &ds {
        d.validate(
            "fixture.n",
            "fn Main(){let a=[0];let b=0;Drop(a);Print(a);b=1;}",
        )
        .unwrap();
    }
    compile_scalar(
        "fixture.n",
        "fn Main(){let a=[0];var b=0;Print(a);Drop(a);b=1;}",
    )
    .unwrap();
}

#[test]
fn monomorphic_aliases_keep_the_first_concrete_origin() {
    mismatch_at(
        "fn Main(){var v=(if true then {return;}else{return;});var w=v;v=3;w=true;}",
        (68, 72),
        Some((64, 65)),
        "Bool",
        "I64",
    );
    mismatch_at(
        "fn Main(){var v=3;var w=v;v=3;w=true;}",
        (32, 36),
        Some((16, 17)),
        "Bool",
        "I64",
    );
    mismatch_at(
        "fn Main(){var v=(if true then {return;}else{return;});var w=true;w=v;v=3;}",
        (71, 72),
        Some((60, 64)),
        "I64",
        "Bool",
    );
    for source in [
        "fn Main(){var v=(if true then {return;}else{return;});var w=v;v=3;w=4;}",
        "fn Main(){var v=3;var w=v;v=3;w=4;}",
        "fn Main(){var v=(if true then {return;}else{return;});var w=true;w=v;v=false;}",
    ] {
        compile_scalar("fixture.n", source).unwrap();
    }
}

#[test]
fn indexed_assignment_uses_annotation_or_inferred_initializer_origin() {
    mismatch_at(
        "fn Main(){var a:Array<I64>=[0];a[0]=true;}",
        (36, 40),
        Some((16, 26)),
        "Bool",
        "I64",
    );
    mismatch_at(
        "fn Main(){var a=[0];a[0]=true;}",
        (25, 29),
        Some((16, 19)),
        "Bool",
        "I64",
    );
    compile_scalar("fixture.n", "fn Main(){var a:Array<I64>=[0];a[0]=1;}").unwrap();
    compile_scalar("fixture.n", "fn Main(){var a=[0];a[0]=1;}").unwrap();
}

#[test]
fn bottom_equality_arrays_and_alias_copies_keep_types_and_data_independent() {
    for source in [
        "fn Main(){let x=((if true then {return;}else{return;}))==true;}",
        "fn Main(){let x=true==(if true then {return;}else{return;});}",
        "fn Main(){let x=[(if true then {return;}else{return;}),true];}",
        "fn Main(){let x=[true,(if true then {return;}else{return;})];}",
        "fn Main(){var v=(if true then {return;}else{return;});var w=v;v=[1];w=[2];w[0]=9;}",
    ] {
        compile_scalar("fixture.n", source).unwrap();
    }
    let twin = "fn Main(){var v=[0];var w=v;v=[1];w=[2];w[0]=9;AssertEq(v,[1]);AssertEq(w,[9]);}";
    let program = compile_scalar("fixture.n", twin).unwrap();
    assert!(crate::interp::run_scalar(&program).diagnostics.is_empty());
}

#[test]
fn bottom_never_hides_independent_operator_or_array_mismatch() {
    mismatch_at(
        "fn Main(){let x=(if true then {return;}else{return;})+true;}",
        (54, 58),
        None,
        "Bool",
        "I64",
    );
    mismatch_at(
        "fn Main(){let x=true+(if true then {return;}else{return;});}",
        (16, 20),
        None,
        "Bool",
        "I64",
    );
    mismatch_at(
        "fn Main(){let x=[(if true then {return;}else{return;}),1,true];}",
        (57, 61),
        Some((55, 56)),
        "Bool",
        "I64",
    );
    mismatch_at(
        "fn Main(){let x=[1,(if true then {return;}else{return;}),true];}",
        (57, 61),
        Some((17, 18)),
        "Bool",
        "I64",
    );
}

#[test]
fn invalid_name_barrier_and_tainted_alias_do_not_cascade() {
    let ds = rejected("fn Main(){let x=missing;let y=x;let z=true+1;}");
    assert_eq!(ds.len(), 1);
    assert_eq!(
        (ds[0].code.as_str(), ds[0].stage, ds[0].operation),
        ("NX-E0101", Stage::Names, Operation::NamesResolve)
    );
    mismatch_at(
        "fn Main(){let x=0;let y=x;let z=true+1;}",
        (32, 36),
        None,
        "Bool",
        "I64",
    );
    let ds = rejected("fn Main(){let x=true+1;let y=x;let z=false+1;}");
    assert_eq!(ds.len(), 2, "{ds:?}");
    assert_eq!(
        ds.iter()
            .map(|d| d.primary_span.as_ref().unwrap().start)
            .collect::<Vec<_>>(),
        vec![16, 37]
    );
    assert!(
        ds.iter()
            .all(|d| d.code == "NX-E0102" && d.subcause.as_deref() == Some("type_mismatch"))
    );
    compile_scalar("fixture.n", "fn Main(){let x=0+1;let y=x;let z=0+1;}").unwrap();

    let array = rejected("fn Main(){let x=true+1;let a=[x,false+1];}");
    assert_eq!(array.len(), 2, "{array:?}");
    assert_eq!(
        array
            .iter()
            .map(|d| d.primary_span.as_ref().unwrap().start)
            .collect::<Vec<_>>(),
        [16, 32]
    );
    assert!(
        array
            .iter()
            .all(|d| d.code == "NX-E0102" && d.subcause.as_deref() == Some("type_mismatch"))
    );
    let array = rejected("fn Main(){let x=true+1;let a=[x,1,true];}");
    assert_eq!(array.len(), 2, "{array:?}");
    assert_eq!(
        array
            .iter()
            .map(|d| d.primary_span.as_ref().unwrap().start)
            .collect::<Vec<_>>(),
        [16, 34]
    );
    assert_eq!(
        array[1].data.get("expected_type"),
        Some(&crate::diagnostic::DataValue::String("I64".into()))
    );
}

#[test]
fn deferred_constraints_cross_branches_loops_arrays_and_equality() {
    let dv = "(if true then {return;}else{return;})";
    for body in [
        "var w=v;var x=w;x=3;v=true;",
        "let x=[v,3];v=true;",
        "var a=[v];v=3;a=[true];",
        "let x=v==true;v=3;",
        "let x=if true then {value:v;}else{value:3;};v=true;",
        "while v{}v=3;",
    ] {
        let source = format!("fn Main(){{var v={dv};{body}}}");
        let ds = rejected(&source);
        assert!(
            ds.iter()
                .any(|d| d.code == "NX-E0102" && d.subcause.as_deref() == Some("type_mismatch")),
            "{source}: {ds:?}"
        );
    }
}

#[test]
fn adr_gap_recursive_array_unification_reports_inference_error() {
    // F0-0002 §4.1.1 requires monomorphic unification but has no distinct
    // occurs-check diagnostic. Preserve NX-E0103 until that gap is settled.
    let source = "fn Main(){var v=(if true then {return;}else{return;});var a=[v];v=a;}";
    let ds = rejected(source);
    assert_eq!(ds.len(), 1, "{ds:?}");
    assert_eq!(
        (ds[0].code.as_str(), ds[0].stage, ds[0].operation),
        ("NX-E0103", Stage::Types, Operation::TypesInfer)
    );
    ds[0].validate("fixture.n", source).unwrap();
}

#[test]
fn constructed_array_child_retains_copyable_domain_after_divergent_element() {
    // F0-0002 §4.1.2.2,5: the unresolved constructor owns U; the
    // projected assignment needs its contaminated element type.
    let source = "fn Main(){var v1=[(if true then {return;}else{return;})];v1[0][0]=MockOpen(1);}";
    let ds = rejected(source);
    assert_eq!(
        ds.iter()
            .map(|d| (d.code.as_str(), d.subcause.as_deref()))
            .collect::<Vec<_>>(),
        [("NX-E0103", None)],
        "{ds:?}"
    );
    for d in ds {
        d.validate("fixture.n", source).unwrap();
    }
}

#[test]
fn unresolved_branch_constructor_reports_the_operand() {
    // F0-0002 §4.1.1: "la primera restricción incompatible con las
    // precedentes se denuncia en su operando". S10 in §14.1 places
    // NX-E0103 on []. §4.7 places NX-E1303 on the value of a later
    // incompatible normal continuation when both types are concrete.
    for (source, primary) in [
        ("fn Main(){let x=(if true then [] else 1);}", (30, 32)),
        ("fn Main(){let x=(if false then 1 else []);}", (38, 40)),
    ] {
        let ds = rejected(source);
        assert_eq!(ds.len(), 1, "{ds:?}");
        assert_eq!(
            (ds[0].code.as_str(), ds[0].subcause.as_deref()),
            ("NX-E0103", None),
            "{ds:?}"
        );
        assert_eq!(
            (ds[0].stage, ds[0].operation),
            (Stage::Types, Operation::TypesInfer)
        );
        assert_eq!(
            ds[0].primary_span.as_ref().map(|s| (s.start, s.end)),
            Some(primary)
        );
        assert_eq!(&source[primary.0 as usize..primary.1 as usize], "[]");
        assert!(ds[0].data.is_empty() && ds[0].related_spans.is_empty());
        ds[0].validate("fixture.n", source).unwrap();
    }
    compile_scalar("fixture.n", "fn Main(){let x=(if true then [] else [1]);}").unwrap();
    compile_scalar("fixture.n", "fn Main(){let x=(if false then [1] else []);}").unwrap();
}

#[test]
fn concrete_branch_join_reports_the_later_value() {
    // F0-0002 §4.7: without an expected type, each continuation synthesizes
    // its type, and the first later incompatible value receives NX-E1303.
    // §13.15 relates the first value as expected and the later as found.
    use crate::diagnostic::DataValue;
    for (source, primary, expected, found, relations) in [
        (
            "fn Main(){let t=(if true then 1 else false);}",
            (37, 42),
            "I64",
            "Bool",
            [
                (RelatedRole::ExpectedType, 30, 31),
                (RelatedRole::FoundType, 37, 42),
            ],
        ),
        (
            "fn Main(){let t=(if true then false else 1);}",
            (41, 42),
            "Bool",
            "I64",
            [
                (RelatedRole::ExpectedType, 30, 35),
                (RelatedRole::FoundType, 41, 42),
            ],
        ),
    ] {
        let ds = rejected(source);
        assert_eq!(ds.len(), 1, "{ds:?}");
        let d = &ds[0];
        assert_eq!((d.code.as_str(), d.subcause.as_deref()), ("NX-E1303", None));
        assert_eq!(
            (d.stage, d.operation),
            (Stage::Types, Operation::TypesCheck)
        );
        assert_eq!(
            d.primary_span.as_ref().map(|s| (s.start, s.end)),
            Some(primary)
        );
        assert_eq!(
            d.related_spans
                .iter()
                .map(|r| (r.role, r.span.start, r.span.end))
                .collect::<Vec<_>>(),
            relations
        );
        assert_eq!(d.data.len(), 2);
        assert_eq!(
            d.data.get("expected_type"),
            Some(&DataValue::String(expected.into()))
        );
        assert_eq!(
            d.data.get("found_type"),
            Some(&DataValue::String(found.into()))
        );
        d.validate("fixture.n", source).unwrap();
    }
    compile_scalar("fixture.n", "fn Main(){let t=(if true then 1 else 2);}").unwrap();
}

#[test]
fn deferred_constructor_conflict_and_resolved_sibling_keep_one_causal_error() {
    // F0-0002 §4.1.2.2 omits the nonserializable mismatch at [] when
    // that constructor emits NX-E0103. The resolved sibling keeps its
    // independent mismatch under §4.1.2.4.
    let unresolved = "fn Main(){var v0=[];v0=5;}";
    let ds = rejected(unresolved);
    assert_eq!(ds.len(), 1, "{ds:?}");
    assert_eq!(
        (ds[0].code.as_str(), ds[0].subcause.as_deref()),
        ("NX-E0103", None)
    );
    assert_eq!(
        ds[0].primary_span.as_ref().map(|s| (s.start, s.end)),
        Some((17, 19))
    );
    assert_eq!(
        (ds[0].stage, ds[0].operation),
        (Stage::Types, Operation::TypesInfer)
    );
    assert!(ds[0].data.is_empty() && ds[0].related_spans.is_empty());
    assert_eq!(&unresolved[17..19], "[]");
    ds[0].validate("fixture.n", unresolved).unwrap();
    compile_scalar("fixture.n", "fn Main(){var v0=[];v0=[5];}").unwrap();
    let mismatch = "fn Main(){var v0=7;v0=(if true then [] else [1]);}";
    let ds = rejected(mismatch);
    assert_eq!(ds.len(), 2, "{ds:?}");
    for (diagnostic, (code, primary)) in ds
        .iter()
        .zip([("NX-E0103", (36, 38)), ("NX-E0102", (44, 47))])
    {
        assert_eq!(diagnostic.code, code);
        assert_eq!(
            diagnostic.primary_span.as_ref().map(|s| (s.start, s.end)),
            Some(primary)
        );
    }
    compile_scalar(
        "fixture.n",
        "fn Main(){var v0=[0];v0=(if true then [] else [1]);}",
    )
    .unwrap();
}

#[test]
fn unresolved_array_outer_shape_is_checked_in_scalar_contexts() {
    // F0-0002 §§4.1.1, 4.7, 13.9: a deferred element does not erase the
    // constructor's array shape. The later assignment resolves its element.
    let prefix = "fn Main(){var q=(if true then {return;}else{return;});";
    for suffix in [
        "if [q] {}q=1;}",
        "while [q] {}q=1;}",
        "let x:I64=[q];q=1;}",
        "var x=1;x=[q];q=1;}",
        "let x=[q]+1;q=1;}",
    ] {
        let source = format!("{prefix}{suffix}");
        let diagnostics = rejected(&source);
        let at = source.find("[q]").unwrap() as u64;
        assert!(
            diagnostics.iter().any(|d| {
                d.code == "NX-E0102"
                    && d.subcause.as_deref() == Some("type_mismatch")
                    && d.primary_span
                        .as_ref()
                        .is_some_and(|s| s.start == at && s.end == at + 3)
                    && d.data.get("found_type")
                        == Some(&crate::diagnostic::DataValue::String("Array<I64>".into()))
            }),
            "{source}: {diagnostics:?}"
        );
    }
    let source = format!("fn F(x:view Bool){{}}{prefix}F([q]);q=1;}}");
    let diagnostics = rejected(&source);
    let at = source.find("[q]").unwrap() as u64;
    assert!(
        diagnostics.iter().any(|d| d.code == "NX-E0102"
            && d.primary_span
                .as_ref()
                .is_some_and(|s| s.start == at && s.end == at + 3)),
        "{diagnostics:?}"
    );
}

#[test]
fn heterogeneous_array_keeps_first_concrete_element_and_independent_errors() {
    // F0-0002 §§4.1.1, 13.9, 13.15: the first concrete element is the
    // expected-type origin; invalid later elements cannot replace it.
    for elements in ["1,true,false", "1,true,2,false"] {
        let source = format!("fn Main(){{let x=[{elements}];}}");
        let diagnostics = rejected(&source);
        assert_eq!(diagnostics.len(), 2, "{source}: {diagnostics:?}");
        assert!(diagnostics.iter().all(|d| d.code == "NX-E0102"
            && d.data.get("expected_type")
                == Some(&crate::diagnostic::DataValue::String("I64".into()))));
    }
    let source = "fn Main(){let x=[1,(if true then true else false)];}";
    let diagnostics = rejected(source);
    assert_eq!(diagnostics.len(), 2, "{diagnostics:?}");
    for (diagnostic, at) in diagnostics.iter().zip([
        source.find("true else").unwrap() as u64,
        source.find("false)").unwrap() as u64,
    ]) {
        assert_eq!(diagnostic.code, "NX-E0102");
        assert_eq!(diagnostic.primary_span.as_ref().map(|s| s.start), Some(at));
    }
    let source =
        "fn Main(){var q=(if true then {return;}else{return;});let x=[1,true,false,q];q=3;}";
    let diagnostics = rejected(source);
    assert_eq!(diagnostics.len(), 2, "{diagnostics:?}");
    assert!(
        diagnostics
            .iter()
            .all(|d| d.primary_span.as_ref().unwrap().start < source.find("q=3").unwrap() as u64)
    );
}

#[test]
fn adr_gap_grouped_scalar_operand_primary() {
    // The ADR leaves a group versus its inner value unspecified. This test
    // fixes the chosen interior primary when that operand is the diagnostic
    // site. For comparison with equal inadmissible types, §4.6 uses the
    // operator; §§13.15/13.17 fix the operands for differing types.
    for (grouped, plain, needle) in [
        (
            "fn Main(){let x:I64=(true);}",
            "fn Main(){let x:I64=true;}",
            "true",
        ),
        (
            "fn Main(){let x=(true)+1;}",
            "fn Main(){let x=true+1;}",
            "true",
        ),
        (
            "fn Main(){let x=1+(true);}",
            "fn Main(){let x=1+true;}",
            "true",
        ),
        (
            "fn Main(){let x=(true)-1;}",
            "fn Main(){let x=true-1;}",
            "true",
        ),
        (
            "fn Main(){let x=1-(true);}",
            "fn Main(){let x=1-true;}",
            "true",
        ),
        (
            "fn Main(){let x=1-([1]);}",
            "fn Main(){let x=1-[1];}",
            "[1]",
        ),
        (
            "fn Main(){let x=(true)<false;}",
            "fn Main(){let x=true<false;}",
            "<",
        ),
        (
            "fn Main(){let x=1<(true);}",
            "fn Main(){let x=1<true;}",
            "true",
        ),
        (
            "fn Main(){let x=(true)==1;}",
            "fn Main(){let x=true==1;}",
            "1",
        ),
        (
            "fn Main(){let x=1==(true);}",
            "fn Main(){let x=1==true;}",
            "true",
        ),
        (
            "fn Main(){AssertEq((true),1);}",
            "fn Main(){AssertEq(true,1);}",
            "1",
        ),
        (
            "fn Main(){AssertEq(1,(true));}",
            "fn Main(){AssertEq(1,true);}",
            "true",
        ),
        (
            "fn F(x:view Bool){}fn Main(){F((25));}",
            "fn F(x:view Bool){}fn Main(){F(25);}",
            "25",
        ),
    ] {
        for source in [grouped, plain] {
            let diagnostics = rejected(source);
            assert_eq!(diagnostics.len(), 1, "{source}: {diagnostics:?}");
            let diagnostic = &diagnostics[0];
            assert_eq!(
                (diagnostic.code.as_str(), diagnostic.subcause.as_deref()),
                ("NX-E0102", Some("type_mismatch")),
                "{source}"
            );
            assert_eq!(diagnostic.data.len(), 2, "{source}");
            assert!(diagnostic.data.contains_key("expected_type"), "{source}");
            assert!(diagnostic.data.contains_key("found_type"), "{source}");
            diagnostic.validate("fixture.n", source).unwrap();
            let at = source.find(needle).unwrap() as u64;
            assert_eq!(
                diagnostics[0]
                    .primary_span
                    .as_ref()
                    .map(|s| (s.start, s.end)),
                Some((at, at + needle.len() as u64)),
                "{source}: {diagnostics:?}"
            );
        }
    }
    for (grouped, plain, needle) in [
        (
            "fn Main(){let x:I64=(true);}",
            "fn Main(){let x:I64=true;}",
            "true",
        ),
        (
            "fn Main(){var x=1;x=(true);}",
            "fn Main(){var x=1;x=true;}",
            "true",
        ),
        (
            "fn F()->I64{return (true);}fn Main(){}",
            "fn F()->I64{return true;}fn Main(){}",
            "true",
        ),
        (
            "fn F(x:view I64){}fn Main(){F((true));}",
            "fn F(x:view I64){}fn Main(){F(true);}",
            "true",
        ),
        (
            "fn Main(){let a=[1];let x=a[(true)];}",
            "fn Main(){let a=[1];let x=a[true];}",
            "true",
        ),
        ("fn Main(){if (1) {}}", "fn Main(){if 1 {}}", "1"),
        ("fn Main(){while (1) {}}", "fn Main(){while 1 {}}", "1"),
        (
            "fn Main(){let x:Array<I64>=[(true)];}",
            "fn Main(){let x:Array<I64>=[true];}",
            "true",
        ),
        (
            "fn Main(){let x:I64=if true then (true) else 1;}",
            "fn Main(){let x:I64=if true then true else 1;}",
            "true",
        ),
        (
            "fn Main(){let x:I64=if true then {value:(true);} else {value:1;};}",
            "fn Main(){let x:I64=if true then {value:true;} else {value:1;};}",
            "true",
        ),
    ] {
        for source in [grouped, plain] {
            let diagnostics = rejected(source);
            assert_eq!(diagnostics.len(), 1, "{source}: {diagnostics:?}");
            assert_eq!(diagnostics[0].code, "NX-E0102", "{source}");
            let at = source.rfind(needle).unwrap() as u64;
            assert_eq!(
                diagnostics[0]
                    .primary_span
                    .as_ref()
                    .map(|s| (s.start, s.end)),
                Some((at, at + needle.len() as u64)),
                "{source}: {diagnostics:?}"
            );
        }
    }
}

#[test]
fn deferred_join_has_one_diagnostic_per_source_site() {
    // F0-0002 §13.9 preserves independent sites. Multiple deferred roots
    // reaching one concrete operand are still one source site.
    for (source, code, primary) in [
        (
            "fn Main(){var r=[];let a=[[r],[r],250];r=[1];}",
            "NX-E0102",
            (34, 37),
        ),
        (
            "fn Main(){var a=(if true then {return;} else {return;});var b=(if true then {return;} else {return;});a=[b];b=[a];}",
            "NX-E0103",
            (110, 113),
        ),
    ] {
        let diagnostics = rejected(source);
        assert_eq!(diagnostics.len(), 1, "{source}: {diagnostics:?}");
        assert_eq!(diagnostics[0].code, code, "{source}: {diagnostics:?}");
        assert_eq!(
            diagnostics[0]
                .primary_span
                .as_ref()
                .map(|span| (span.start, span.end)),
            Some(primary),
            "{source}: {diagnostics:?}"
        );
        for (i, left) in diagnostics.iter().enumerate() {
            assert!(
                diagnostics[i + 1..].iter().all(|right| {
                    left.code != right.code
                        || left.primary_span != right.primary_span
                        || left.related_spans != right.related_spans
                        || left.data != right.data
                }),
                "duplicate site in {source}: {diagnostics:?}"
            );
        }
    }
    let source = "fn Main(){let a=[1,true,false];}";
    let diagnostics = rejected(source);
    assert_eq!(diagnostics.len(), 2, "{diagnostics:?}");
    assert_ne!(diagnostics[0].primary_span, diagnostics[1].primary_span);
    assert!(diagnostics.iter().all(|d| d.code == "NX-E0102"
        && d.data.get("expected_type")
            == Some(&crate::diagnostic::DataValue::String("I64".into()))
        && d.data.get("found_type") == Some(&crate::diagnostic::DataValue::String("Bool".into()))));
}

#[test]
fn tainted_left_operand_preserves_independent_right_join() {
    // F0-0002 §13.9 retains the right continuation's independent failure.
    let source = "fn Main(){let bad:I64=true;let x=(true+(if bad then false else 41));}";
    let ds = rejected(source);
    assert_eq!(ds.len(), 3, "{ds:?}");
    exact_type_site(
        &ds[0],
        source,
        "NX-E0102",
        (22, 26),
        "I64",
        "Bool",
        Some((18, 21)),
    );
    exact_type_site(&ds[1], source, "NX-E0102", (34, 38), "I64", "Bool", None);
    exact_type_site(
        &ds[2],
        source,
        "NX-E1303",
        (63, 65),
        "Bool",
        "I64",
        Some((52, 57)),
    );
}

#[test]
fn contaminated_index_base_keeps_selector_error() {
    // F0-0002 §13.9 keeps an independent selector error after a bad base.
    let source = "fn Main(){let bad:Array<I64>=true;let x=bad[true];}";
    let ds = rejected(source);
    assert_eq!(ds.len(), 2, "{ds:?}");
    exact_type_site(
        &ds[0],
        source,
        "NX-E0102",
        (29, 33),
        "Array<I64>",
        "Bool",
        Some((18, 28)),
    );
    exact_type_site(&ds[1], source, "NX-E0102", (44, 48), "I64", "Bool", None);
}

#[test]
fn divergent_array_child_keeps_its_constructor_constraint() {
    // F0-0002 §4.1.1: Never contributes no concrete element type.
    compile_scalar(
        "fixture.n",
        "fn Main(){var v=[(if true then {return;} else {return;})];v=[true];}",
    )
    .unwrap();
    let ds = rejected(
        "fn Main(){var v=[(if true then {return;} else {return;})];v=[true];AssertEq([v],[1]);}",
    );
    assert_eq!(ds.len(), 1, "{ds:?}");
    assert_eq!(ds[0].code, "NX-E0102");
}

#[test]
fn tainted_array_element_retains_independent_constraint_error() {
    // F0-0002 §13.9 preserves the first independent unresolved site.
    let source = "fn Main(){var q0=(if true then {return;} else {return;});var q1=(if true then {return;} else {return;});var q2=[[]];q0=[q1];AssertEq(q1,[1]);AssertEq([q2],[q0,[[true]]]);}";
    let ds = rejected(source);
    assert_eq!(ds.len(), 2, "{ds:?}");
    assert_eq!(
        (
            ds[0].code.as_str(),
            ds[0].subcause.as_deref(),
            ds[0].stage,
            ds[0].operation
        ),
        ("NX-E0103", None, Stage::Types, Operation::TypesInfer)
    );
    assert_eq!(
        ds[0].primary_span.as_ref().map(|s| (s.start, s.end)),
        Some((112, 114))
    );
    assert!(ds[0].data.is_empty() && ds[0].related_spans.is_empty());
    ds[0].validate("fixture.n", source).unwrap();
    exact_type_site(
        &ds[1],
        source,
        "NX-E0102",
        (161, 165),
        "I64",
        "Bool",
        // §4.1.2.4: q0 is the first valid element of this array.
        Some((156, 158)),
    );
}

#[test]
fn unresolved_inner_array_suppresses_outer_constructor_consequence() {
    // F0-0002 §13.9: the unresolved inner [] is the cause. The [q2]
    // constructor cannot determine its element only because q2 failed.
    let source = "fn Main(){var q0=(if true then {return;} else {return;});var q2=[];q2=q0;AssertEq([q2],[q2,true]);}";
    let ds = rejected(source);
    assert_eq!(ds.len(), 1, "{ds:?}");
    assert_eq!(ds[0].code, "NX-E0103");
    assert_eq!(
        ds[0].primary_span.as_ref().map(|s| (s.start, s.end)),
        Some((64, 66))
    );
    assert!(ds[0].data.is_empty() && ds[0].related_spans.is_empty());
}

#[test]
fn invalid_comparison_result_does_not_feed_an_enclosing_union() {
    // F0-0002 §4.1.2.5: the comparison fails its I64 domain and cannot
    // provide a nominal Bool result for a derived union mismatch.
    let source = "fn Main(){let x=1;let p=true;let ar=[1];var r=[];var s=[];var t=[];AssertEq((if false then (if p then r else s) else r<t),true);s=[[1]];}";
    let ds = rejected(source);
    assert_eq!(ds.len(), 1, "{ds:?}");
    assert_eq!(
        ds.iter()
            .map(|d| (
                d.code.as_str(),
                d.primary_span.as_ref().map(|s| (s.start, s.end))
            ))
            .collect::<Vec<_>>(),
        [("NX-E0102", Some((118, 119)))],
    );
}

#[test]
fn deferred_branch_inference_uses_later_binding_before_reporting_each_operand() {
    // F0-0002 §§4.1.1, 4.7: the written I64 reaches both branch values.
    // A later binding determines r's array element without letting the
    // already invalid ar branch choose that element for r.
    let deferred = "fn Main(){let x=1;let p=true;let ar=[1];let br=[true];let aar=[[1]];var r=[];let a0:I64=(if p then (r) else ar);r=[true];}";
    let concrete = "fn Main(){let x=1;let p=true;let ar=[1];let br=[true];let aar=[[1]];let r=[true];let a0:I64=(if p then (r) else ar);}";
    for (source, r_site, ar_site, annotation) in [
        (deferred, (100, 101), (108, 110), (84, 87)),
        (concrete, (104, 105), (112, 114), (88, 91)),
    ] {
        let ds = rejected(source);
        assert_eq!(ds.len(), 2, "{ds:?}");
        exact_type_site(
            &ds[0],
            source,
            "NX-E0102",
            r_site,
            "I64",
            "Array<Bool>",
            Some(annotation),
        );
        exact_type_site(
            &ds[1],
            source,
            "NX-E0102",
            ar_site,
            "I64",
            "Array<I64>",
            Some(annotation),
        );
    }
}

#[test]
fn deferred_failed_left_preserves_independent_arithmetic_error_on_right() {
    // F0-0002 §13.9 retains the independently invalid right subexpression.
    let source = "fn Main(){let x=1;let p=true;let ar=[1];let br=[true];let aar=[[1]];var r=[];let a0=(if p then r else x)+(true+1);r=[true];}";
    let ds = rejected(source);
    assert_eq!(ds.len(), 2, "{ds:?}");
    exact_type_site(
        &ds[0],
        source,
        "NX-E0102",
        (95, 96),
        "I64",
        "Array<Bool>",
        None,
    );
    exact_type_site(&ds[1], source, "NX-E0102", (106, 110), "I64", "Bool", None);
}

#[test]
fn nested_branch_uses_the_first_deferred_value_as_its_anchor() {
    // F0-0002 §§4.1.1 and 4.7: an unresolved first continuation still
    // supplies the expected type at the first incompatible nested value.
    let deferred = "fn Main(){let x=1;let p=true;let ar=[1];let br=[true];let aar=[[1]];var r=[];let a0=(if p then (r) else (if p then 1 else (r)));r=[true];}";
    let concrete = "fn Main(){let x=1;let p=true;let ar=[1];let br=[true];let aar=[[1]];let r=[true];let a0=(if p then (r) else (if p then 1 else (r)));}";
    for (source, primary, origin) in [
        (deferred, (115, 116), (96, 97)),
        (concrete, (119, 120), (100, 101)),
    ] {
        let ds = rejected(source);
        assert_eq!(ds.len(), 1, "{ds:?}");
        exact_type_site(
            &ds[0],
            source,
            "NX-E1303",
            primary,
            "Array<Bool>",
            "I64",
            Some(origin),
        );
    }
}

#[test]
fn deferred_intrinsic_left_suppresses_independent_domain_consequence() {
    // F0-0002 §§13.15, 13.17: the left operand owns the first mismatch;
    // a built-in numeric domain has no source expected_type span.
    let deferred = "fn Main(){var r=[];let a=r+true;r=[[3]];}";
    let concrete = "fn Main(){let r=[[3]];let a=r+true;}";
    let ds = rejected(deferred);
    let twin = rejected(concrete);
    assert_eq!(ds.len(), 1, "{ds:?}");
    assert_eq!(twin.len(), 1, "{twin:?}");
    exact_type_site(
        &ds[0],
        deferred,
        "NX-E0102",
        (25, 26),
        "I64",
        "Array<Array<I64>>",
        None,
    );
    exact_type_site(
        &twin[0],
        concrete,
        "NX-E0102",
        (28, 29),
        "I64",
        "Array<Array<I64>>",
        None,
    );
}

#[test]
fn deferred_resolution_changes_the_first_incompatible_source_constraint() {
    // F0-0002 §4.1.1: the first source constraint incompatible with its
    // predecessors is reported at that constraint's operand. An unresolved
    // binding can therefore fail at its later assignment; its concrete twin
    // already has that assignment's type before the earlier use.
    for (deferred, concrete, deferred_site, concrete_site) in [
        (
            "fn Main(){var q=(if true then {return;} else {return;});let a=q==true;q=1;}",
            "fn Main(){var q=1;let a=q==true;}",
            ("NX-E0102", (72, 73), "Bool", "I64", Some((65, 69))),
            ("NX-E0102", (27, 31), "I64", "Bool", Some((24, 25))),
        ),
        (
            "fn Main(){var r=[];let a=(if true then r else [1]);r=[true];}",
            "fn Main(){let r=[true];let a=(if true then r else [1]);}",
            ("NX-E0102", (54, 58), "I64", "Bool", Some((46, 49))),
            (
                "NX-E1303",
                (50, 53),
                "Array<Bool>",
                "Array<I64>",
                Some((43, 44)),
            ),
        ),
        (
            "fn Main(){var r=[];let a=[r,[1]];r=[true];}",
            "fn Main(){let r=[true];let a=[r,[1]];}",
            // §4.1.2.2/.6: the literal 1 fixes the element of r, not the
            // enclosing [1] constructor as a synthetic origin.
            ("NX-E0102", (36, 40), "I64", "Bool", Some((29, 30))),
            ("NX-E0102", (33, 34), "Bool", "I64", Some((30, 31))),
        ),
    ] {
        for (source, (code, span, expected, found, origin)) in
            [(deferred, deferred_site), (concrete, concrete_site)]
        {
            let ds = rejected(source);
            assert_eq!(ds.len(), 1, "{source}: {ds:?}");
            exact_type_site(&ds[0], source, code, span, expected, found, origin);
            match check_scalar("fixture.n", source) {
                Err(CompileError::Rejected(check)) => assert_eq!(*check[0], ds[0]),
                other => panic!("expected check rejection: {other:?}"),
            }
        }
    }
}

#[test]
fn deferred_less_than_blames_right_operand_after_resolution() {
    // F0-0002 §4.6 compares the right operand with the left type; §13.17
    // selects the first incompatible operand at that comparison site.
    let deferred = "fn Main(){var r=[];let a=r<1;r=[1];}";
    let concrete = "fn Main(){let r=[1];let a=r<1;}";
    let ds = rejected(deferred);
    let twin = rejected(concrete);
    assert_eq!(ds.len(), 1, "{ds:?}");
    assert_eq!(twin.len(), 1, "{twin:?}");
    exact_type_site(
        &ds[0],
        deferred,
        "NX-E0102",
        (27, 28),
        "Array<I64>",
        "I64",
        Some((25, 26)),
    );
    exact_type_site(
        &twin[0],
        concrete,
        "NX-E0102",
        (28, 29),
        "Array<I64>",
        "I64",
        Some((26, 27)),
    );
}

#[test]
fn deferred_less_than_alias_reports_one_operator_domain_error() {
    // F0-0002 §§4.6, 13.9: two uses of one binding compare equal first;
    // the I64 domain belongs to the operator once that binding resolves.
    for (source, primary, found_origin) in [
        (
            "fn Main(){var r=[];let a=(r)<r;r=[true];}",
            (28, 29),
            (26, 27),
        ),
        ("fn Main(){let r=[true];let a=(r)<r;}", (32, 33), (30, 31)),
    ] {
        let ds = rejected(source);
        assert_eq!(ds.len(), 1, "{ds:?}");
        let d = &ds[0];
        assert_eq!(
            (d.code.as_str(), d.subcause.as_deref(), d.stage, d.operation),
            (
                "NX-E0102",
                Some("type_mismatch"),
                Stage::Types,
                Operation::TypesCheck
            )
        );
        assert_eq!(
            d.primary_span.as_ref().map(|s| (s.start, s.end)),
            Some(primary)
        );
        assert_eq!(d.data.len(), 2);
        assert_eq!(
            d.data.get("expected_type"),
            Some(&crate::diagnostic::DataValue::String("I64".into()))
        );
        assert_eq!(
            d.data.get("found_type"),
            Some(&crate::diagnostic::DataValue::String("Array<Bool>".into()))
        );
        assert_eq!(
            d.related_spans
                .iter()
                .map(|r| (r.role, r.span.start, r.span.end))
                .collect::<Vec<_>>(),
            vec![(RelatedRole::FoundType, found_origin.0, found_origin.1)]
        );
        d.validate("fixture.n", source).unwrap();
    }
}

#[test]
fn deferred_right_shape_conflict_suppresses_derived_operator_domain() {
    // F0-0002 §4.1.1 suppresses the less-than domain after the right
    // operand's comparison with the concrete left operand has failed.
    for (source, primary, origin) in [
        (
            "fn Main(){var r=[];let a=true<r;r=[true];}",
            (30, 31),
            (25, 29),
        ),
        ("fn Main(){let r=[true];let a=true<r;}", (34, 35), (29, 33)),
    ] {
        let ds = rejected(source);
        assert_eq!(ds.len(), 1, "{ds:?}");
        exact_type_site(
            &ds[0],
            source,
            "NX-E0102",
            primary,
            "Bool",
            "Array<Bool>",
            Some(origin),
        );
    }
}

#[test]
fn deferred_nested_branch_uses_outer_expected_type_at_first_operand() {
    // F0-0002 §4.7 passes an already concrete outer branch type into the
    // nested first value, even when the binding is resolved later.
    for (source, primary, origin) in [
        (
            "fn Main(){let x=1;var r=[];let a=(if true then x else (if true then r else x));r=[true];}",
            (68, 69),
            (47, 48),
        ),
        (
            "fn Main(){let x=1;let r=[true];let a=(if true then x else (if true then r else x));}",
            (72, 73),
            (51, 52),
        ),
    ] {
        let ds = rejected(source);
        assert_eq!(ds.len(), 1, "{ds:?}");
        exact_type_site(
            &ds[0],
            source,
            "NX-E1303",
            primary,
            "I64",
            "Array<Bool>",
            Some(origin),
        );
    }
}

#[test]
fn deferred_failed_array_element_does_not_fix_sibling_type() {
    // F0-0002 §4.1.1: a failed first element cannot make a valid later
    // element wrong merely because its deferred binding resolves later.
    for (source, primary, origin) in [
        (
            "fn Main(){let x=1;var r=[];let a=[(if true then r else x),true];r=[true];}",
            (55, 56),
            (48, 49),
        ),
        (
            "fn Main(){let x=1;let r=[true];let a=[(if true then r else x),true];}",
            (59, 60),
            (52, 53),
        ),
    ] {
        let ds = rejected(source);
        assert_eq!(ds.len(), 1, "{ds:?}");
        exact_type_site(
            &ds[0],
            source,
            "NX-E1303",
            primary,
            "Array<Bool>",
            "I64",
            Some(origin),
        );
    }
}

#[test]
fn deferred_branch_join_suppresses_derived_equality_error() {
    // F0-0002 §4.1.1: the failed join contaminates its containing expression.
    let deferred =
        "fn Main(){let x=1;let br=[true];var r=[];let e=(if true then r else x)==br;r=[[1]];}";
    let concrete =
        "fn Main(){let x=1;let br=[true];let r=[[1]];let e=(if true then r else x)==br;}";
    let ds = rejected(deferred);
    let twin = rejected(concrete);
    assert_eq!(ds.len(), 1, "{ds:?}");
    assert_eq!(twin.len(), 1, "{twin:?}");
    exact_type_site(
        &ds[0],
        deferred,
        "NX-E1303",
        (68, 69),
        "Array<Array<I64>>",
        "I64",
        Some((61, 62)),
    );
    exact_type_site(
        &twin[0],
        concrete,
        "NX-E1303",
        (71, 72),
        "Array<Array<I64>>",
        "I64",
        Some((64, 65)),
    );
}

#[test]
fn deferred_equality_error_suppresses_outer_annotation_error() {
    // F0-0002 §13.9: the annotation cannot diagnose the invalid equality
    // result after the equality already owns its incompatible operand.
    let deferred = "fn Main(){var r=[];let p=true;let a:I64=(r==p);r=[[3]];}";
    let concrete = "fn Main(){let r=[[3]];let p=true;let a:I64=(r==p);}";
    let ds = rejected(deferred);
    let twin = rejected(concrete);
    assert_eq!(ds.len(), 1, "{ds:?}");
    assert_eq!(twin.len(), 1, "{twin:?}");
    for (source, d, primary) in [(deferred, &ds[0], (44, 45)), (concrete, &twin[0], (47, 48))] {
        assert_eq!(
            (d.code.as_str(), d.subcause.as_deref(), d.stage, d.operation),
            (
                "NX-E0102",
                Some("type_mismatch"),
                Stage::Types,
                Operation::TypesCheck
            )
        );
        assert_eq!(
            d.primary_span.as_ref().map(|span| (span.start, span.end)),
            Some(primary)
        );
        assert_eq!(
            d.data.get("expected_type"),
            Some(&crate::diagnostic::DataValue::String(
                "Array<Array<I64>>".into()
            ))
        );
        assert_eq!(
            d.data.get("found_type"),
            Some(&crate::diagnostic::DataValue::String("Bool".into()))
        );
        assert!(
            d.related_spans
                .iter()
                .all(|r| r.role != RelatedRole::ExpectedType
                    || (r.span.start, r.span.end) != primary)
        );
        d.validate("fixture.n", source).unwrap();
    }
}

#[test]
fn head_acceptance_families_keep_their_current_check_and_compile_results() {
    // The paired b506d7d outcomes are replayed by i07c_campanas.py.
    // §4.1.1 keeps Never distinct from an unresolved monomorphic variable;
    // §13.17 requires NX-E0103 when an array element stays unresolved.
    let cases = [
        (
            "print_never_array",
            "fn Main(){Print([(if true then {return;}else{return;})]);}",
            Some("NX-E0103"),
        ),
        (
            "indexed_write_never_array",
            "fn Main(){var x1=[true];x1[[(if true then {return;}else{return;})]]=(if true then {return;}else{return;});}",
            Some("NX-E0103"),
        ),
        (
            "assign_divergent_alias",
            "fn Main(){var q=(if true then {return;}else{return;});var c=[];c=q;q=[1];}",
            None,
        ),
        (
            "len_divergent",
            "fn Main(){let n=Len((if true then {return;}else{return;}));}",
            None,
        ),
        (
            "assert_empty_against_concrete",
            "fn Main(){let y=[1];AssertEq([],y);}",
            None,
        ),
        (
            "divergent_condition_value",
            "fn Main(){var q=(if true then {return;}else{return;});let x=(if q then {value:q;}else {value:1;});q=true;}",
            Some("NX-E1303"),
        ),
        (
            "index_divergent",
            "fn Main(){let x=(if true then {return;}else{return;})[0];}",
            None,
        ),
        (
            "index_two_divergent_branches",
            "fn Main(){let x=(if true then (if true then {return;}else{return;}) else (if true then {return;}else{return;}))[0];}",
            None,
        ),
        (
            "less_than_never_array",
            "fn Main(){let x=[(if true then {return;}else{return;})]<(if true then {return;}else{return;});}",
            Some("NX-E0103"),
        ),
        (
            "annotated_never_array",
            "fn Main(){let a:I64=[(if true then {return;}else{return;})];}",
            Some("NX-E0103"),
        ),
        (
            "unannotated_never_array",
            "fn Main(){let a=[(if true then {return;}else{return;})];}",
            Some("NX-E0103"),
        ),
    ];
    for (name, source, expected_code) in cases {
        match expected_code {
            Some(code) => {
                let diagnostics = rejected(source);
                assert_eq!(diagnostics.len(), 1, "{name}: {diagnostics:?}");
                assert_eq!(diagnostics[0].code, code, "{name}");
                let checked = match check_scalar("fixture.n", source) {
                    Err(CompileError::Rejected(errors)) => errors,
                    other => panic!("{name}: check must reject: {other:?}"),
                };
                assert_eq!(checked.len(), 1, "{name}: {checked:?}");
                assert_eq!(checked[0].code, code, "{name}");
                assert_eq!(checked[0].primary_span, diagnostics[0].primary_span);
            }
            None => {
                check_scalar("fixture.n", source)
                    .unwrap_or_else(|error| panic!("{name}: {error:?}"));
                compile_scalar("fixture.n", source)
                    .unwrap_or_else(|error| panic!("{name}: {error:?}"));
            }
        }
    }
}

#[test]
fn deferred_intrinsic_domain_has_no_self_expected_origin() {
    // F0-0002 §13.15: an intrinsic domain has no source span for its
    // expected type, including when the operand resolves after its use.
    let source = "fn Main(){var r=[];let a=r+true;r=[[3]];}";
    let ds = rejected(source);
    let left = ds
        .iter()
        .find(|d| {
            d.primary_span
                .as_ref()
                .is_some_and(|s| (s.start, s.end) == (25, 26))
        })
        .unwrap();
    assert_eq!(left.code, "NX-E0102");
    assert_eq!(
        left.data.get("expected_type"),
        Some(&crate::diagnostic::DataValue::String("I64".into()))
    );
    assert_eq!(
        left.data.get("found_type"),
        Some(&crate::diagnostic::DataValue::String(
            "Array<Array<I64>>".into()
        ))
    );
    assert!(
        left.related_spans
            .iter()
            .all(|r| r.role != RelatedRole::ExpectedType)
    );
    left.validate("fixture.n", source).unwrap();
}

#[test]
fn grouped_operands_keep_diagnostic_identity_in_deferred_and_domain_paths() {
    // The group-primary choice is an adr_gap; F0-0002 §§13.15, 13.17
    // determine the operand and the expected/found data once chosen.
    for (grouped, plain) in [
        (
            "fn Main(){var r=[];let y=r==(1);r=[1];}",
            "fn Main(){var r=[];let y=r==1;r=[1];}",
        ),
        (
            "fn Main(){var r=[];AssertEq(r,(1));r=[1];}",
            "fn Main(){var r=[];AssertEq(r,1);r=[1];}",
        ),
        (
            "fn Main(){var r=[];let y=[r,(1)];r=[1];}",
            "fn Main(){var r=[];let y=[r,1];r=[1];}",
        ),
        (
            "fn Main(){var r=[];r=(1);r=[1];}",
            "fn Main(){var r=[];r=1;r=[1];}",
        ),
        (
            "fn Main(){var r=[];let a=(if true then r else (1));r=[1];}",
            "fn Main(){var r=[];let a=(if true then r else 1);r=[1];}",
        ),
        (
            "fn Main(){let a=(if true then true else (1));}",
            "fn Main(){let a=(if true then true else 1);}",
        ),
        (
            "fn Main(){let a=(if true then true else {value:(1);});}",
            "fn Main(){let a=(if true then true else {value:1;});}",
        ),
        ("fn Main(){let x=(1)[0];}", "fn Main(){let x=1[0];}"),
        ("fn Main(){Len((1));}", "fn Main(){Len(1);}"),
        ("fn Main(){Inspect((1));}", "fn Main(){Inspect(1);}"),
        (
            "fn Main(){var r=[];let a:I64=(r)-1;r=[1];}",
            "fn Main(){var r=[];let a:I64=r-1;r=[1];}",
        ),
        (
            "fn Main(){let x=(true)<true;}",
            "fn Main(){let x=true<true;}",
        ),
        (
            "fn Main(){var r=[];let a:I64=(if true then r else (true));r=[1];}",
            "fn Main(){var r=[];let a:I64=(if true then r else true);r=[1];}",
        ),
    ] {
        let signature = |source: &str| {
            rejected(source)
                .into_iter()
                .map(|d| {
                    let primary = d
                        .primary_span
                        .as_ref()
                        .map(|s| &source[s.start as usize..s.end as usize]);
                    let related: Vec<_> = d
                        .related_spans
                        .iter()
                        .map(|r| (r.role, &source[r.span.start as usize..r.span.end as usize]))
                        .collect();
                    (
                        d.code,
                        d.subcause,
                        primary.map(str::to_owned),
                        related
                            .into_iter()
                            .map(|(role, text)| (role, text.to_owned()))
                            .collect::<Vec<_>>(),
                        d.data,
                    )
                })
                .collect::<Vec<_>>()
        };
        assert_eq!(signature(grouped), signature(plain), "{grouped} / {plain}");
    }
}

#[test]
fn permission_escalation_cause_matches_the_parameter_permission() {
    // F0-0002 §13.15 gives the causal read/inout loan witness for NX-E1105;
    // §13.6 defines its active_permission field.
    for (source, permission, cause) in [
        (
            "fn F(x:view I64){x=1;}fn Main(){}",
            "read",
            CauseOperation::BorrowRead,
        ),
        (
            "fn F(x:edit MockResource){Drop(x);}fn Main(){}",
            "inout",
            CauseOperation::BorrowInout,
        ),
    ] {
        let ds = rejected(source);
        assert_eq!(ds.len(), 1, "{source}: {ds:?}");
        let d = &ds[0];
        assert_eq!(
            (d.code.as_str(), d.subcause.as_deref()),
            ("NX-E1105", Some("permission_escalation"))
        );
        assert_eq!(
            d.data["active_permission"],
            crate::diagnostic::DataValue::String(permission.into())
        );
        assert!(
            d.cause_chain.iter().any(|step| step.operation == cause),
            "{source}: {d:?}"
        );
    }
}

#[test]
fn dependent_consumed_value_stays_rejected_without_internal_error() {
    // F0-0002 §13.9 preserves the ownership error through both if branches.
    let ds = rejected("fn Main(){let a=[1];Drop(a);let c=a[0];let y=if true then c else c;}");
    assert!(ds.iter().any(|d| d.code == "NX-E0202"), "{ds:?}");
}

#[test]
fn independent_index_temporaries_leave_return_paths_valid() {
    // IndexValue consumes its base and index temporaries once on each path.
    compile_scalar(
        "fixture.n",
        "fn F(b:view Bool)->I64{if b{return [1,2][1];}return [3,4][0];}fn Main(){}",
    )
    .unwrap();
}

#[test]
fn nested_comparison_uses_inner_operator_at_same_type_domain_failure() {
    // F0-0002 §§4.6, 13.15: equal inadmissible types locate the operator.
    let source = "fn Main(){let x=((1<2)<true);}";
    let ds = rejected(source);
    assert_eq!(ds.len(), 1, "{ds:?}");
    assert_eq!(ds[0].code, "NX-E0102");
    assert_eq!(
        ds[0].primary_span.as_ref().map(|s| (s.start, s.end)),
        Some((22, 23))
    );
}

#[test]
fn equality_outer_context_does_not_replace_the_branch_join() {
    // F0-0002 §4.7 reports the later incompatible continuation when an if
    // synthesizes its type without an expected type from the expression site.
    let source = "fn Main(){let x=1==(if true then true else 1);}";
    let ds = rejected(source);
    assert_eq!(ds.len(), 1, "{ds:?}");
    assert_eq!(
        (ds[0].code.as_str(), ds[0].subcause.as_deref()),
        ("NX-E1303", None)
    );
    assert_eq!(
        ds[0].primary_span.as_ref().map(|s| (s.start, s.end)),
        Some((43, 44))
    );
    assert_eq!(
        ds[0].data.get("expected_type"),
        Some(&crate::diagnostic::DataValue::String("Bool".into()))
    );
    assert_eq!(
        ds[0].data.get("found_type"),
        Some(&crate::diagnostic::DataValue::String("I64".into()))
    );
    compile_scalar("fixture.n", "fn Main(){let x=1==(if true then 1 else 1);}").unwrap();
}

#[test]
fn outer_bool_context_preserves_distinct_branch_operand_sites() {
    // F0-0002 §4.1.2.4: the outer Bool context reaches both normal
    // leaves, so each incompatible leaf owns an NX-E0102.
    let source = "fn F1(){var v1=[[]];var v2=[true];var a0=[1];if (if [[],[]] then false else (if v1 then a0 else v2)) {}}fn Main(){}";
    let ds = rejected(source);
    for primary in [(88, 90), (96, 98)] {
        assert!(
            ds.iter().any(|d| d.code == "NX-E0102"
                && d.primary_span
                    .as_ref()
                    .is_some_and(|s| (s.start, s.end) == primary)),
            "{ds:?}"
        );
    }
}

#[test]
fn nested_failed_join_supplies_only_its_first_valid_witness_to_outer_join() {
    // F0-0002 §4.1.2.5: the inner failed union has no type for a
    // consumer, but its first valid continuation is the outer union's
    // reference. Both unions own one independent NX-E1303.
    for source in [
        "fn Main(){let x=1;let p=true;var r=[];let a=(if p then (if p then r else x) else 1);r=[true];}",
        "fn Main(){let x=1;let p=true;var r=[true];let a=(if p then (if p then r else x) else 1);r=[true];}",
    ] {
        let diagnostics = rejected(source);
        let joins = diagnostics
            .iter()
            .filter(|d| d.code == "NX-E1303")
            .collect::<Vec<_>>();
        assert_eq!(joins.len(), 2, "{source}: {diagnostics:?}");
        assert_eq!(
            joins
                .iter()
                .map(|d| {
                    let span = d.primary_span.as_ref().unwrap();
                    &source[span.start as usize..span.end as usize]
                })
                .collect::<Vec<_>>(),
            ["x", "1"]
        );
    }
}

#[test]
fn plus_distributed_left_failure_suppresses_second_domain_error() {
    // F0-0002 §4.1.2.4–5: + distributes I64 to the first operand's
    // normal leaves and reports only its first incompatible operand.
    for source in [
        "fn Main(){let p=true;var r=[];let a=(if p then r else 1)+true;r=[true];}",
        "fn Main(){let p=true;var r=[true];let a=(if p then r else 1)+true;}",
    ] {
        let diagnostics = rejected(source);
        assert_eq!(diagnostics.len(), 1, "{source}: {diagnostics:?}");
        assert_eq!(diagnostics[0].code, "NX-E0102");
        let span = diagnostics[0].primary_span.as_ref().unwrap();
        assert_eq!(&source[span.start as usize..span.end as usize], "r");
    }
}

#[test]
fn tainted_comparison_keeps_the_other_operands_deferred_domain() {
    // F0-0002 §4.1.2.3–5: the left sum has no usable result, while
    // the right operand still independently fails the I64 domain of <.
    for source in [
        "fn Main(){var r=[];let a=(true+1)<r;r=[true];}",
        "fn Main(){var r=[true];let a=(true+1)<r;}",
    ] {
        let diagnostics = rejected(source);
        assert_eq!(diagnostics.len(), 2, "{source}: {diagnostics:?}");
        assert_eq!(
            diagnostics
                .iter()
                .map(|d| d.code.as_str())
                .collect::<Vec<_>>(),
            ["NX-E0102", "NX-E0102"]
        );
        let second = &diagnostics[1];
        let span = second.primary_span.as_ref().unwrap();
        assert_eq!(&source[span.start as usize..span.end as usize], "<");
        assert_eq!(
            second.data.get("expected_type"),
            Some(&crate::diagnostic::DataValue::String("I64".into()))
        );
        assert_eq!(
            second.data.get("found_type"),
            Some(&crate::diagnostic::DataValue::String("Array<Bool>".into()))
        );
    }
}

#[test]
fn pending_comparison_domain_suppresses_dependent_result_annotation() {
    // F0-0002 §4.1.2.5: the invalid < cannot offer Bool to the
    // annotation, whether its operand type was known early or late.
    for source in [
        "fn Main(){var r=[];let a:I64=r<r;r=[true];}",
        "fn Main(){var r=[true];let a:I64=r<r;}",
    ] {
        let diagnostics = rejected(source);
        assert_eq!(diagnostics.len(), 1, "{source}: {diagnostics:?}");
        assert_eq!(diagnostics[0].code, "NX-E0102");
        let span = diagnostics[0].primary_span.as_ref().unwrap();
        assert_eq!(&source[span.start as usize..span.end as usize], "<");
        assert_eq!(
            diagnostics[0].data.get("expected_type"),
            Some(&crate::diagnostic::DataValue::String("I64".into()))
        );
        assert_eq!(
            diagnostics[0].data.get("found_type"),
            Some(&crate::diagnostic::DataValue::String("Array<Bool>".into()))
        );
    }
}

#[test]
fn value_if_restores_hypothetical_depth_before_following_statement() {
    // the test-only balance assertion checks depth after this value-if
    // and its following live statement have both been lowered.
    let source = "fn Main(){let x=if true then {value:1;} else {value:2;};AssertEq(x,1);}";
    compile_scalar("fixture.n", source).unwrap();
}

#[test]
fn deferred_if_inside_array_reports_each_site_once() {
    let source = "fn Main(){var r=[];let a=[[1],(if true then r else 163)];}";
    let diagnostics = rejected(source);
    assert_eq!(diagnostics.len(), 1, "{diagnostics:?}");
    assert_eq!(diagnostics[0].code, "NX-E0102");
    assert_eq!(
        diagnostics[0]
            .primary_span
            .as_ref()
            .map(|s| (s.start, s.end)),
        Some((51, 54))
    );
}

#[test]
fn deferred_array_shape_conflict_reports_its_array_site() {
    let source = "fn Main(){var q=[];let a=[q,1];q=[1];}";
    // A later array assignment preserves the earlier element-site mismatch.
    // §4.1.2.6 keeps the first element occurrence as the expected witness.
    mismatch_at(source, (28, 29), Some((26, 27)), "I64", "Array<I64>");
    let repaired = "fn Main(){var q=[];let a=[q,[1]];q=[1];}";
    compile_scalar("fixture.n", repaired).unwrap();
}

#[test]
fn deferred_sibling_array_constraints_keep_element_diagnostic() {
    use crate::diagnostic::DataValue;
    let source = "fn Main(){var q=[];var r=[];var z=[];let a=[q,r,(if true then {q=[1];r=[true];value:z;}else{value:z;})];}";
    // ADR §4.1.2.2/.4/.6: q=[1] fixes the common element and the
    // distributed context also resolves z. The later r=[true] leaf owns
    // the sole mismatch; z no longer has an unresolved element.
    let diagnostics = rejected(source);
    assert_eq!(diagnostics.len(), 1, "{diagnostics:?}");
    let mismatch = &diagnostics[0];
    assert_eq!(
        (mismatch.code.as_str(), mismatch.subcause.as_deref()),
        ("NX-E0102", Some("type_mismatch"))
    );
    assert_eq!(
        mismatch.primary_span.as_ref().map(|s| (s.start, s.end)),
        Some((72, 76))
    );
    assert_eq!(
        mismatch.data.get("expected_type"),
        Some(&DataValue::String("I64".into()))
    );
    assert_eq!(
        mismatch.data.get("found_type"),
        Some(&DataValue::String("Bool".into()))
    );
    assert_eq!(
        mismatch
            .related_spans
            .iter()
            .map(|r| (r.role, r.span.start, r.span.end))
            .collect::<Vec<_>>(),
        vec![
            (RelatedRole::ExpectedType, 65, 68),
            (RelatedRole::FoundType, 72, 76)
        ]
    );
    for diagnostic in &diagnostics {
        diagnostic.validate("fixture.n", source).unwrap();
    }
    let repaired = "fn Main(){var q=[];var r=[];var z=[];let a=[q,r,(if true then {q=[1];r=[1];value:z;}else{value:z;})];}";
    compile_scalar("fixture.n", repaired).unwrap();
}

#[test]
fn failed_array_element_keeps_independent_sibling_error() {
    // F0-0002 §4.1.2.5: a failed condition does not cancel the
    // independent branch join or the later array sibling.
    let source = "fn Main(){let a=[(if [false,1] then true else 3),true+1];}";
    let diagnostics = rejected(source);
    assert_eq!(
        diagnostics
            .iter()
            .map(|d| (
                d.code.as_str(),
                d.primary_span.as_ref().map(|s| (s.start, s.end))
            ))
            .collect::<Vec<_>>(),
        [
            ("NX-E0102", Some((28, 29))),
            ("NX-E1303", Some((46, 47))),
            ("NX-E0102", Some((49, 53)))
        ]
    );
}

#[test]
fn empty_return_uses_its_own_function_keyword() {
    let source = "fn G()->I64{return 1;}fn F()->I64{return;}fn Main(){}";
    let diagnostics = rejected(source);
    assert_eq!(diagnostics.len(), 1, "{diagnostics:?}");
    assert_eq!(diagnostics[0].code, "NX-E0102");
    assert_eq!(
        diagnostics[0]
            .primary_span
            .as_ref()
            .map(|s| (s.start, s.end)),
        Some((34, 40))
    );
}

#[test]
fn deferred_assignment_checks_the_last_independent_constructor() {
    let source = "fn H1(){var q0=if true then {return;} else {return;};var q2=if true then {return;} else {return;};while if if q0 then {value:q2;} else false then q2 else {value:1;} {}q0=if q2 then [] else {value:true;};}fn Main(){}";
    let diagnostics = rejected(source);
    // ADR §4.1.2.2/RD127: the unresolved obligation belongs to [] itself,
    // not to the enclosing union.
    assert!(
        diagnostics.iter().any(|d| d.code == "NX-E0103"
            && d.primary_span
                .as_ref()
                .is_some_and(|s| (s.start, s.end) == (181, 183))),
        "{diagnostics:?}"
    );
}

#[test]
fn nested_result_annotation_limit_uses_whole_type_primary() {
    let source = "fn F(x: view Result<Array<I64>, I64>){}fn Main(){}";
    let diagnostics = rejected(source);
    assert_eq!(diagnostics.len(), 1, "{diagnostics:?}");
    assert_eq!(diagnostics[0].code, "NX-E0006");
    let start = source.find("Result<").unwrap() as u64;
    assert_eq!(
        diagnostics[0]
            .primary_span
            .as_ref()
            .map(|s| (s.start, s.end)),
        Some((start, start + "Result<Array<I64>, I64>".len() as u64))
    );
}

#[test]
fn adjacent_with_clause_limit_includes_its_last_token() {
    use crate::diagnostic::DataValue;
    for source in ["fn Main() with no_io{}", "fn Main() with no_io {}"] {
        // a token ending exactly at the body opener still belongs to
        // the capability clause and its complete diagnostic primary.
        let diagnostics = rejected(source);
        assert_eq!(diagnostics.len(), 1, "{source}: {diagnostics:?}");
        let diagnostic = &diagnostics[0];
        assert_eq!(
            (diagnostic.code.as_str(), diagnostic.subcause.as_deref()),
            ("NX-E0006", None)
        );
        assert_eq!(
            diagnostic.feature_id.as_deref(),
            Some("implementation_limit")
        );
        assert_eq!(diagnostic.data.len(), 1);
        assert_eq!(
            diagnostic.data.get("pending_scope"),
            Some(&DataValue::String("I09".into()))
        );
        let start = source.find("with").unwrap() as u64;
        let end = source.find("no_io").unwrap() as u64 + "no_io".len() as u64;
        assert_eq!(
            diagnostic.primary_span.as_ref().map(|s| (s.start, s.end)),
            Some((start, end))
        );
        assert!(diagnostic.related_spans.is_empty());
        diagnostic.validate("fixture.n", source).unwrap();
    }
}

#[test]
fn divergent_while_condition_keeps_reachable_resource_drop() {
    let source = "fn F0(b:view Bool){var r2=MockOpen(3);return;while b{while b{Drop(r2);return;}}Drop(r2);}fn Main(){}";
    compile_scalar("fixture.n", source).unwrap();
}

#[test]
fn unresolved_array_condition_reports_inference_only() {
    // §4.1.2.2 and RD127 require NX-E0103 for this constructor.
    // §4.1.2.5 suppresses a derived Array<?> domain mismatch.
    let source = "fn Main(){var q=(if true then {return;}else{return;});if [q] {}}";
    let diagnostics = rejected(source);
    assert_eq!(diagnostics.len(), 1, "{diagnostics:?}");
    assert_eq!(diagnostics[0].code, "NX-E0103");
    assert_eq!(
        diagnostics[0]
            .primary_span
            .as_ref()
            .map(|s| &source[s.start as usize..s.end as usize]),
        Some("[q]")
    );
}

#[test]
fn scalar_array_with_inconsistent_elements_reports_only_leaf() {
    // F0-0002 §13.9 preserves independent leaf sites but does not require
    // repeating their error at the parent. Current result is one NX-E0102
    // on false and no separate array-level diagnosis.
    let source = "fn Main(){let x:Bool=[27,false];}";
    let diagnostics = rejected(source);
    assert_eq!(diagnostics.len(), 1, "{diagnostics:?}");
    assert_eq!(diagnostics[0].code, "NX-E0102");
    assert_eq!(
        diagnostics[0]
            .primary_span
            .as_ref()
            .map(|s| &source[s.start as usize..s.end as usize]),
        Some("false")
    );
}

#[test]
fn tainted_expected_context_retains_independent_sibling_errors() {
    // F0-0002 §4.1.2.4–5 retains each independent branch leaf and
    // the bad right operand of subtraction after the left is contaminated.
    let source = "fn Main(){AssertEq((if (if [1]==[2,3] then {value:42<42;} else {value:0;}) then 0 else (if 0 then 0 else 0)),(if 42==0 then {value:[1];} else {value:-3-[true];})-0);}";
    let diagnostics = rejected(source);
    assert_eq!(diagnostics.len(), 4, "{diagnostics:?}");
    assert!(diagnostics.iter().all(|d| d.code == "NX-E0102"));
}

#[test]
fn subtraction_passes_intrinsic_i64_context_through_branch_values() {
    // F0-0002 §§4.7, 13.15: '-' has an intrinsic I64 domain, so each
    // incompatible Boolean branch is NX-E0102 at the Boolean leaf.
    let source = "fn Main(){let x=(if true then true else false)-1;}";
    let diagnostics = rejected(source);
    assert_eq!(diagnostics.len(), 2, "{diagnostics:?}");
    for (diagnostic, token) in diagnostics.iter().zip(["true else", "false)"]) {
        let at = source.find(token).unwrap() as u64;
        assert_eq!(diagnostic.code, "NX-E0102");
        assert_eq!(diagnostic.primary_span.as_ref().map(|s| s.start), Some(at));
        assert_eq!(
            diagnostic.data.get("expected_type"),
            Some(&crate::diagnostic::DataValue::String("I64".into()))
        );
        assert_eq!(
            diagnostic.data.get("found_type"),
            Some(&crate::diagnostic::DataValue::String("Bool".into()))
        );
    }
}

#[test]
fn annotated_empty_array_keeps_its_written_element_type() {
    // F0-0002 §§4.1.1, 13.17: an empty constructor obtains its element
    // type from the written Array context, including Boolean elements.
    let source = "fn Main(){let x:Array<Bool>=[];AssertEq(x,[true]);}";
    compile_scalar("fixture.n", source).unwrap();
    super::check_scalar("fixture.n", source).unwrap();
}

#[test]
fn tainted_if_condition_suppresses_dependent_nested_array_errors() {
    // F0-0002 §4.1.2.5: a failed outer condition does not suppress
    // the independent [false]==0 mismatch inside a branch.
    let source =
        "fn Main(){let b:Bool=0+0==true;let x=(if b then 0 else (if [false]==0 then 0 else 1));}";
    let diagnostics = rejected(source);
    assert_eq!(diagnostics.len(), 2, "{diagnostics:?}");
    let at = source.find("true;").unwrap() as u64;
    assert_eq!(diagnostics[0].code, "NX-E0102");
    assert_eq!(
        diagnostics[0]
            .primary_span
            .as_ref()
            .map(|s| (s.start, s.end)),
        Some((at, at + 4))
    );
    assert_eq!(diagnostics[1].code, "NX-E0102");
    assert_eq!(
        diagnostics[1]
            .primary_span
            .as_ref()
            .map(|s| (s.start, s.end)),
        Some((68, 69))
    );
}

#[test]
fn tainted_array_element_does_not_repeat_its_branch_join_error() {
    // F0-0002 §4.1.2.5: the failed condition leaves the branch join
    // independent; the enclosing array adds no derived error.
    let source = "fn Main(){var v=(if true<0 then 1 else 0);let x=[(if 1<v then 1 else true)];}";
    let diagnostics = rejected(source);
    assert_eq!(diagnostics.len(), 2, "{diagnostics:?}");
    let at = source.find("0 then").unwrap() as u64;
    assert_eq!(diagnostics[0].code, "NX-E0102");
    assert_eq!(
        diagnostics[0]
            .primary_span
            .as_ref()
            .map(|s| (s.start, s.end)),
        Some((at, at + 1))
    );
    assert_eq!(diagnostics[1].code, "NX-E1303");
    assert_eq!(
        diagnostics[1]
            .primary_span
            .as_ref()
            .map(|s| (s.start, s.end)),
        Some((69, 73))
    );
}

#[test]
fn deferred_if_result_uses_first_normal_branch_for_array_resolution() {
    // F0-0002 §§4.1.1, 13.9: a later concrete branch constrains a pending
    // first branch; an unresolved [] has one causal inference error.
    let source = "fn Main(){var v=(if true then {return;}else{return;});var w=(if false then v else -3);v=(if false then {value:v;}else{value:[];});}";
    let diagnostics = rejected(source);
    assert_eq!(diagnostics.len(), 1, "{diagnostics:?}");
    let at = source.find("[]").unwrap() as u64;
    assert_eq!(diagnostics[0].code, "NX-E0103");
    assert_eq!(
        diagnostics[0]
            .primary_span
            .as_ref()
            .map(|s| (s.start, s.end)),
        Some((at, at + 2))
    );
}

#[test]
fn deferred_array_conflict_uses_concrete_type_found_later_in_either_statement_order() {
    // F0-0002 §4.1.1 determines acceptance from all local constraints;
    // §13.17 reserves NX-E0103 for an empty array whose element stays unresolved.
    // §13.15 uses the concrete type and the incompatible operand for E0102.
    for (source, primary) in [
        (
            "fn Main(){var v0=[[]];let t=v0==true;let v2=v0[0][0]+1;}",
            (32, 36),
        ),
        (
            "fn Main(){var v0=[[]];let v2=v0[0][0]+1;let t=v0==true;}",
            (50, 54),
        ),
    ] {
        let ds = rejected(source);
        assert_eq!(ds.len(), 1, "{source}: {ds:?}");
        let d = &ds[0];
        assert_eq!(
            (d.code.as_str(), d.subcause.as_deref()),
            ("NX-E0102", Some("type_mismatch"))
        );
        assert_eq!(
            d.primary_span.as_ref().map(|s| (s.start, s.end)),
            Some(primary)
        );
        assert_eq!(
            d.data.get("expected_type"),
            Some(&crate::diagnostic::DataValue::String(
                "Array<Array<I64>>".into()
            ))
        );
        assert_eq!(
            d.data.get("found_type"),
            Some(&crate::diagnostic::DataValue::String("Bool".into()))
        );
        d.validate("fixture.n", source).unwrap();
    }
    compile_scalar(
        "fixture.n",
        "fn Main(){var v0=[[]];let v2=v0[0][0]+1;let t=v0==[[1]];}",
    )
    .unwrap();
}

#[test]
fn deferred_array_branch_order_uses_later_incompatible_value_and_branch_code() {
    // F0-0002 §4.7: without an expected type, conflicting continuations
    // produce NX-E1303 on the value of the later normal continuation.
    // §13.15 assigns the first value to expected_type and the later one to
    // found_type, even when its concrete array element is fixed afterwards.
    for (source, primary, expected, found, relations) in [
        (
            "fn Main(){var v=[[]];let t=(if true then v else true);let u=v[0][0]+1;}",
            (48, 52),
            "Array<Array<I64>>",
            "Bool",
            [
                (RelatedRole::ExpectedType, 41, 42),
                (RelatedRole::FoundType, 48, 52),
            ],
        ),
        (
            "fn Main(){var v=[[]];let t=(if true then true else v);let u=v[0][0]+1;}",
            (51, 52),
            "Bool",
            "Array<Array<I64>>",
            [
                (RelatedRole::ExpectedType, 41, 45),
                (RelatedRole::FoundType, 51, 52),
            ],
        ),
    ] {
        let ds = rejected(source);
        assert_eq!(ds.len(), 1, "{source}: {ds:?}");
        let d = &ds[0];
        assert_eq!((d.code.as_str(), d.subcause.as_deref()), ("NX-E1303", None));
        assert_eq!(
            (d.stage, d.operation),
            (Stage::Types, Operation::TypesCheck)
        );
        assert_eq!(
            d.primary_span.as_ref().map(|s| (s.start, s.end)),
            Some(primary)
        );
        assert!(matches!(
            &source[primary.0 as usize..primary.1 as usize],
            "true" | "v"
        ));
        assert_eq!(
            d.related_spans
                .iter()
                .map(|r| (r.role, r.span.start, r.span.end))
                .collect::<Vec<_>>(),
            relations
        );
        assert_eq!(d.data.len(), 2);
        assert_eq!(
            d.data.get("expected_type"),
            Some(&crate::diagnostic::DataValue::String(expected.into()))
        );
        assert_eq!(
            d.data.get("found_type"),
            Some(&crate::diagnostic::DataValue::String(found.into()))
        );
        d.validate("fixture.n", source).unwrap();
    }
    compile_scalar(
        "fixture.n",
        "fn Main(){var v=[[]];let t=(if true then v else [[1]]);}",
    )
    .unwrap();
}

#[test]
fn nested_deferred_branch_join_waits_for_the_full_array_type() {
    // F0-0002 §4.1.1 resolves all local constraints before selecting source
    // evidence; §13.17 permits NX-E0103 only if the inner [] stays unknown.
    // §4.7 requires NX-E1303 at the later branch value, and §13.15 relates
    // the complete types of both continuations.
    use crate::diagnostic::DataValue;
    for (source, expected, found, expected_span, primary) in [
        (
            "fn Main(){var a=[];var b=[[]];let t=(if true then a else {a=[1];value:b;});let u=b[0][0]+1;}",
            "Array<I64>",
            "Array<Array<I64>>",
            (50, 51),
            (70, 71),
        ),
        (
            "fn Main(){var a=[];var b=[[]];let t=(if true then {a=[1];value:b;}else a);let u=b[0][0]+1;}",
            "Array<Array<I64>>",
            "Array<I64>",
            (63, 64),
            (71, 72),
        ),
    ] {
        let ds = rejected(source);
        assert_eq!(ds.len(), 1, "{ds:?}");
        let d = &ds[0];
        assert_eq!((d.code.as_str(), d.subcause.as_deref()), ("NX-E1303", None));
        assert_eq!(
            (d.stage, d.operation),
            (Stage::Types, Operation::TypesCheck)
        );
        assert_eq!(
            d.primary_span.as_ref().map(|s| (s.start, s.end)),
            Some(primary)
        );
        assert_eq!(
            d.related_spans
                .iter()
                .map(|r| (r.role, r.span.start, r.span.end))
                .collect::<Vec<_>>(),
            [
                (RelatedRole::ExpectedType, expected_span.0, expected_span.1),
                (RelatedRole::FoundType, primary.0, primary.1)
            ]
        );
        assert_eq!(d.data.len(), 2);
        assert_eq!(
            d.data.get("expected_type"),
            Some(&DataValue::String(expected.into()))
        );
        assert_eq!(
            d.data.get("found_type"),
            Some(&DataValue::String(found.into()))
        );
        d.validate("fixture.n", source).unwrap();
    }
    compile_scalar(
        "fixture.n",
        "fn Main(){var a=[];var b=[[]];let t=(if true then a else {a=[[1]];value:b;});let u=b[0][0]+1;}",
    )
    .unwrap();
}

#[test]
fn branch_join_uses_the_concrete_type_acquired_during_later_continuation() {
    // F0-0002 §4.7: a join without an expected type rejects incompatible
    // normal continuations with NX-E1303 at the later value. §13.15 relates
    // the first value as expected_type, even if its deferred constructor
    // becomes concrete while checking the later continuation.
    use crate::diagnostic::DataValue;
    let source = "fn Main(){var a=[];let t=(if true then a else {a=[1];value:true;});}";
    let ds = rejected(source);
    assert_eq!(ds.len(), 1, "{ds:?}");
    let d = &ds[0];
    assert_eq!((d.code.as_str(), d.subcause.as_deref()), ("NX-E1303", None));
    assert_eq!(
        (d.stage, d.operation),
        (Stage::Types, Operation::TypesCheck)
    );
    assert_eq!(
        d.primary_span.as_ref().map(|s| (s.start, s.end)),
        Some((59, 63))
    );
    assert_eq!(
        d.related_spans
            .iter()
            .map(|r| (r.role, r.span.start, r.span.end))
            .collect::<Vec<_>>(),
        [
            (RelatedRole::ExpectedType, 39, 40),
            (RelatedRole::FoundType, 59, 63)
        ]
    );
    assert_eq!(d.data.len(), 2);
    assert_eq!(
        d.data.get("expected_type"),
        Some(&DataValue::String("Array<I64>".into()))
    );
    assert_eq!(
        d.data.get("found_type"),
        Some(&DataValue::String("Bool".into()))
    );
    d.validate("fixture.n", source).unwrap();
    compile_scalar(
        "fixture.n",
        "fn Main(){var a=[];let t=(if true then a else {a=[1];value:[2];});}",
    )
    .unwrap();
}

#[test]
fn deferred_array_equality_obeys_left_operand_expected_type_in_both_orders() {
    // F0-0002 §13.15: "Para igualdad o comparación entre tipos distintos
    // se usa el operando izquierdo". §4.1.1 still diagnoses the right
    // operand that introduces the incompatible constraint.
    for (source, primary, expected, found, relations) in [
        (
            "fn Main(){var v=[[]];let t=v==true;let u=v[0][0]+1;}",
            (30, 34),
            "Array<Array<I64>>",
            "Bool",
            [
                (RelatedRole::ExpectedType, 27, 28),
                (RelatedRole::FoundType, 30, 34),
            ],
        ),
        (
            "fn Main(){var v=[[]];let t=true==v;let u=v[0][0]+1;}",
            (33, 34),
            "Bool",
            "Array<Array<I64>>",
            [
                (RelatedRole::ExpectedType, 27, 31),
                (RelatedRole::FoundType, 33, 34),
            ],
        ),
    ] {
        let ds = rejected(source);
        assert_eq!(ds.len(), 1, "{source}: {ds:?}");
        let d = &ds[0];
        assert_eq!(
            (d.code.as_str(), d.subcause.as_deref()),
            ("NX-E0102", Some("type_mismatch"))
        );
        assert_eq!(
            (d.stage, d.operation),
            (Stage::Types, Operation::TypesCheck)
        );
        assert_eq!(
            d.primary_span.as_ref().map(|s| (s.start, s.end)),
            Some(primary)
        );
        assert!(matches!(
            &source[primary.0 as usize..primary.1 as usize],
            "true" | "v"
        ));
        assert_eq!(
            d.related_spans
                .iter()
                .map(|r| (r.role, r.span.start, r.span.end))
                .collect::<Vec<_>>(),
            relations
        );
        assert_eq!(d.data.len(), 2);
        assert_eq!(
            d.data.get("expected_type"),
            Some(&crate::diagnostic::DataValue::String(expected.into()))
        );
        assert_eq!(
            d.data.get("found_type"),
            Some(&crate::diagnostic::DataValue::String(found.into()))
        );
        d.validate("fixture.n", source).unwrap();
    }
}

#[test]
fn resolved_deferred_array_preserves_distinct_bad_assignment_sites() {
    // F0-0002 §13.9 suppresses only errors dependent on an earlier error;
    // §4.1.1 diagnoses each independent incompatible operand. §13.17 stops
    // using NX-E0103 once the element is fixed by the later array value.
    for source in [
        "fn Main(){var a=[];a=true;a=5;a=[1];}",
        "fn Main(){var a=[0];a=true;a=5;}",
    ] {
        let ds = rejected(source);
        assert_eq!(ds.len(), 2, "{source}: {ds:?}");
        let sites = [
            (source.find("=true;").unwrap() as u64 + 1, 4, "Bool"),
            (source.find("=5;").unwrap() as u64 + 1, 1, "I64"),
        ];
        for (d, (start, width, found)) in ds.iter().zip(sites) {
            assert_eq!(
                (d.code.as_str(), d.subcause.as_deref()),
                ("NX-E0102", Some("type_mismatch"))
            );
            assert_eq!(
                d.primary_span.as_ref().map(|s| (s.start, s.end)),
                Some((start, start + width))
            );
            assert_eq!(
                d.data.get("expected_type"),
                Some(&crate::diagnostic::DataValue::String("Array<I64>".into()))
            );
            assert_eq!(
                d.data.get("found_type"),
                Some(&crate::diagnostic::DataValue::String(found.into()))
            );
            d.validate("fixture.n", source).unwrap();
        }
    }
    compile_scalar("fixture.n", "fn Main(){var a=[];a=[1];a=[2];}").unwrap();
}

#[test]
fn self_referential_deferred_assignment_uses_the_resolved_full_type() {
    // F0-0002 §4.1.1 keeps the deferred binding monomorphic; this test
    // fixes the full resolved type reported by the two assignment forms.
    for (source, primary, expected_origin) in [
        (
            "fn Main(){var v0=(if true then {return;}else{return;});v0=[1]==v0;}",
            (58, 65),
            (58, 61),
        ),
        ("fn Main(){var v0=[0];v0=[1]==v0;}", (24, 31), (17, 20)),
    ] {
        mismatch_at(source, primary, Some(expected_origin), "Bool", "Array<I64>");
    }
    compile_scalar(
        "fixture.n",
        "fn Main(){var v0=(if true then {return;}else{return;});v0=[1];AssertEq(v0,[1]);}",
    )
    .unwrap();
}

#[test]
fn unresolved_constructors_in_distinct_functions_report_independently() {
    // F0-0002 §13.9 preserves independent errors in different functions.
    let source = "fn F(){var a=[];a=true;} fn G(){var b=[];} fn Main(){}";
    let ds = rejected(source);
    assert_eq!(
        ds.iter().map(|d| d.code.as_str()).collect::<Vec<_>>(),
        ["NX-E0103", "NX-E0103"]
    );
    assert_eq!(
        ds.iter()
            .map(|d| d.primary_span.as_ref().map(|s| (s.start, s.end)))
            .collect::<Vec<_>>(),
        [Some((13, 15)), Some((38, 40))]
    );
    compile_scalar(
        "fixture.n",
        "fn F(){var a=[1];a=[2];} fn G(){var b=[1];} fn Main(){}",
    )
    .unwrap();
}

#[test]
fn source_array_depth_reports_the_complete_excessive_expression() {
    // F0-0002 §§11.4, 11.5, 13.13: the 129th Array wrapper is rejected
    // at the complete source expression that synthesizes that type. This
    // budget has an exact primary span, unlike type_solver_steps.
    use crate::diagnostic::DataValue;
    for deferred in [false, true] {
        for depth in [127, 128, 129] {
            let mut source = if deferred {
                String::from("fn Main(){var a0=(if true then {return;}else{return;});")
            } else {
                String::from("fn Main(){var a0=0;")
            };
            for index in 1..=depth {
                source.push_str(&format!("var a{index}=[a{}];", index - 1));
            }
            if deferred {
                source.push_str("a0=0;");
            }
            source.push('}');
            if depth < 129 {
                compile_scalar("fixture.n", &source).unwrap();
                continue;
            }
            let expected = source.find("[a128]").unwrap() as u64;
            let ds = rejected(&source);
            assert_eq!(ds.len(), 1, "{ds:?}");
            let d = &ds[0];
            assert_eq!((d.code.as_str(), d.subcause.as_deref()), ("NX-E0003", None));
            assert_eq!(
                (d.stage, d.operation),
                (Stage::Types, Operation::AnalysisLimit)
            );
            assert_eq!(
                d.primary_span.as_ref().map(|s| (s.start, s.end)),
                Some((expected, expected + 6))
            );
            assert_eq!(&source[expected as usize..expected as usize + 6], "[a128]");
            assert_eq!(
                d.data.get("budget_kind"),
                Some(&DataValue::String("array_type_depth".into()))
            );
            assert_eq!(d.data.get("limit"), Some(&DataValue::UInt(128)));
            assert_eq!(d.data.get("observed"), Some(&DataValue::UInt(129)));
            d.validate("fixture.n", &source).unwrap();
        }
    }
}

#[test]
fn deferred_array_comparison_diagnostic_follows_first_incompatible_constraint() {
    // F0-0002 §4.1.1 orders evidence by source constraints; §13.17 reserves
    // NX-E0103 for a constructor whose element remains unresolved. Both
    // forms determine I64 before comparing the complete array with Bool.
    let inline = "fn Main(){var v0=[[]];let v2=v0==(v0[0][0]<1);}";
    let staged = "fn Main(){var v0=[[]];let t=(v0[0][0]<1);let v2=v0==t;}";
    let inline_errors = rejected(inline);
    let staged_errors = rejected(staged);
    assert_eq!(inline_errors.len(), 1, "{inline_errors:?}");
    assert_eq!(staged_errors.len(), 1, "{staged_errors:?}");
    assert_eq!(inline_errors[0].code, "NX-E0102", "{inline_errors:?}");
    assert_eq!(staged_errors[0].code, "NX-E0102", "{staged_errors:?}");
    for d in [&inline_errors[0], &staged_errors[0]] {
        assert_eq!(d.subcause.as_deref(), Some("type_mismatch"));
        assert_eq!(
            d.data.get("expected_type"),
            Some(&crate::diagnostic::DataValue::String(
                "Array<Array<I64>>".into()
            ))
        );
        assert_eq!(
            d.data.get("found_type"),
            Some(&crate::diagnostic::DataValue::String("Bool".into()))
        );
    }
    assert_eq!(
        inline_errors[0]
            .primary_span
            .as_ref()
            .map(|s| (s.start, s.end)),
        Some((34, 44))
    );
    assert_eq!(
        staged_errors[0]
            .primary_span
            .as_ref()
            .map(|s| (s.start, s.end)),
        Some((52, 53))
    );
    assert_eq!(
        inline_errors[0]
            .related_spans
            .iter()
            .map(|related| (related.role, related.span.start, related.span.end))
            .collect::<Vec<_>>(),
        [
            (RelatedRole::ExpectedType, 29, 31),
            (RelatedRole::FoundType, 34, 44)
        ]
    );
    assert_eq!(
        staged_errors[0]
            .related_spans
            .iter()
            .map(|related| (related.role, related.span.start, related.span.end))
            .collect::<Vec<_>>(),
        [
            (RelatedRole::ExpectedType, 48, 50),
            (RelatedRole::FoundType, 52, 53)
        ]
    );
    for d in inline_errors {
        d.validate("fixture.n", inline).unwrap();
    }
    for d in staged_errors {
        d.validate("fixture.n", staged).unwrap();
    }
}

#[test]
fn empty_array_element_is_resolved_by_later_local_assignment() {
    let source = "fn Main(){var a=[];a=[1];AssertEq(a,[1]);}";
    let program = compile_scalar("fixture.n", source).unwrap();
    assert!(crate::interp::run_scalar(&program).diagnostics.is_empty());
    let equality = "fn Main(){let a=[1];let b=[]==a;AssertEq(b,false);}";
    let program = compile_scalar("fixture.n", equality).unwrap();
    assert!(crate::interp::run_scalar(&program).diagnostics.is_empty());
    for nested in [
        "fn Main(){var a=[];a=[[1]];AssertEq(a,[[1]]);}",
        "fn Main(){var a=[[]];a=[[1]];AssertEq(a,[[1]]);}",
        "fn Main(){var a=[];var b=a;b=[1];AssertEq(a,[]);AssertEq(b,[1]);}",
        "fn Main(){var a=[];var b=[];AssertEq(a,b);a=[1];b=[2];AssertEq(a,[1]);AssertEq(b,[2]);}",
    ] {
        let program = compile_scalar("fixture.n", nested).unwrap();
        assert!(crate::interp::run_scalar(&program).diagnostics.is_empty());
    }
    let unresolved = rejected("fn Main(){var a=[];}");
    assert_eq!(unresolved.len(), 1, "{unresolved:?}");
    assert_eq!(
        (
            unresolved[0].code.as_str(),
            unresolved[0].stage,
            unresolved[0].operation
        ),
        ("NX-E0103", Stage::Types, Operation::TypesInfer)
    );
    assert_eq!(
        unresolved[0]
            .primary_span
            .as_ref()
            .map(|s| (s.start, s.end)),
        Some((16, 18))
    );
}

#[test]
fn separate_concrete_roots_and_pending_array_members_cannot_disagree() {
    let dv = "(if true then {return;}else{return;})";
    let cases = [
        format!("fn Main(){{var v={dv};var w={dv};v=3;w=true;v=w;}}"),
        format!("fn Main(){{var v={dv};var w={dv};let a=[v,w];v=3;w=true;}}"),
        format!("fn Main(){{var v={dv};var w={dv};let a=[w,v];w=true;v=3;}}"),
        "fn Main(){var v=[];var w=[];v=[1];w=[true];v=w;}".to_string(),
        "fn Main(){var v=[];var w=[];v=[1];w=[true];w=v;}".to_string(),
    ];
    for source in &cases {
        let ds = rejected(source);
        assert_eq!(ds.len(), 1, "{source}: {ds:?}");
        assert_eq!(ds[0].code, "NX-E0102", "{source}: {ds:?}");
    }
}

#[test]
fn deferred_array_domain_is_constrained_before_later_assignment() {
    let dv = "(if true then {return;}else{return;})";
    for body in ["let n=Len(v);v=[1];", "let x=v[0];v=[1];"] {
        let source = format!("fn Main(){{var v={dv};{body}}}");
        compile_scalar("fixture.n", &source).unwrap();
    }
    for body in [
        "let n=Len(v);v=true;",
        "let x=v[0];v=true;",
        "let n=Len(v);v=1;",
        "let x=v[0];v=1;",
    ] {
        let source = format!("fn Main(){{var v={dv};{body}}}");
        let ds = rejected(&source);
        assert_eq!(ds.len(), 1, "{source}: {ds:?}");
        assert_eq!(
            (ds[0].code.as_str(), ds[0].subcause.as_deref()),
            ("NX-E0102", Some("not_indexable")),
            "{source}: {ds:?}"
        );
        let found = if let Some(call) = source.find("Len(v)") {
            (call + 4) as u64
        } else {
            source.find("v[0]").unwrap() as u64
        };
        let (found_label, width) = if body.ends_with("true;") {
            ("Bool", 1)
        } else {
            ("I64", 1)
        };
        let d = &ds[0];
        assert_eq!(
            (d.stage, d.operation),
            (Stage::Types, Operation::TypesCheck)
        );
        assert_eq!(d.data.len(), 2);
        assert_eq!(
            d.data.get("expected_type"),
            Some(&crate::diagnostic::DataValue::String("domain:array".into()))
        );
        assert_eq!(
            d.data.get("found_type"),
            Some(&crate::diagnostic::DataValue::String(found_label.into()))
        );
        assert_eq!(
            d.primary_span.as_ref().map(|s| (s.start, s.end)),
            Some((found, found + width))
        );
        assert_eq!(
            d.related_spans
                .iter()
                .map(|s| (s.role, s.span.start, s.span.end))
                .collect::<Vec<_>>(),
            [(RelatedRole::FoundType, found, found + width)]
        );
        d.validate("fixture.n", &source).unwrap();
    }
}

#[test]
fn deferred_copyable_domains_reject_later_resource_bindings() {
    let dv = "(if true then {return;}else{return;})";
    for body in [
        "Print(v);v=MockOpen(1);",
        "let x=v==v;v=MockOpen(1);",
        "let a=[v];v=MockOpen(1);",
    ] {
        let source = format!("fn Main(){{var v={dv};{body}}}");
        let ds = rejected(&source);
        assert_eq!(ds.len(), 1, "{source}: {ds:?}");
        assert_eq!(ds[0].code, "NX-E0102", "{source}: {ds:?}");
        assert_eq!(
            ds[0].data.get("expected_type"),
            Some(&crate::diagnostic::DataValue::String(
                "domain:copyable_data".into()
            ))
        );
    }
}

#[test]
fn deferred_index_element_constraints_survive_later_array_binding() {
    let dv = "(if true then {return;}else{return;})";
    for body in ["var x=v[0];v=[1];x=2;", "v[0]=1;v=[2];"] {
        compile_scalar("fixture.n", &format!("fn Main(){{var v={dv};{body}}}")).unwrap();
    }
    for body in ["var x=v[0];v=[1];x=true;", "v[0]=true;v=[2];"] {
        let source = format!("fn Main(){{var v={dv};{body}}}");
        let ds = rejected(&source);
        assert_eq!(ds.len(), 1, "{source}: {ds:?}");
        assert_eq!(ds[0].code, "NX-E0102", "{source}: {ds:?}");
    }
}

#[test]
fn array_constructor_survives_union_into_larger_alias_group() {
    let dv = "(if true then {return;}else{return;})";
    let positive =
        format!("fn Main(){{var v={dv};var w=v;var z=v;var a=[];v=a;a=[1];w=[2];z=[3];}}");
    compile_scalar("fixture.n", &positive).unwrap();
    let negative = format!("fn Main(){{var v={dv};var w=v;var z=v;var a=[];v=a;a=[1];w=[true];}}");
    let ds = rejected(&negative);
    assert_eq!(ds.len(), 1, "{ds:?}");
    assert_eq!(
        (ds[0].code.as_str(), ds[0].subcause.as_deref()),
        ("NX-E0102", Some("type_mismatch"))
    );
}

#[test]
fn concrete_equality_and_branch_values_fix_deferred_origins() {
    let dv = "(if true then {return;}else{return;})";
    for body in [
        "let x=3+v;v=true;",
        "let x=3==v;v=true;",
        "let x=if true then {value:3;}else{value:v;};v=true;",
    ] {
        let source = format!("fn Main(){{var v={dv};{body}}}");
        let ds = rejected(&source);
        assert_eq!(ds.len(), 1, "{source}: {ds:?}");
        assert_eq!(ds[0].code, "NX-E0102", "{source}: {ds:?}");
        assert_eq!(
            ds[0].data.get("expected_type"),
            Some(&crate::diagnostic::DataValue::String("I64".into()))
        );
        assert_eq!(
            ds[0].data.get("found_type"),
            Some(&crate::diagnostic::DataValue::String("Bool".into()))
        );
        let concrete = source.find("3").unwrap() as u64;
        // §4.1.2.6: the intrinsic I64 requirement of + has no source R;
        // equality and branch joining use their first operand as witness.
        let expected_origin = ds[0]
            .related_spans
            .iter()
            .find(|r| r.role == RelatedRole::ExpectedType);
        if body.contains("3+v") {
            assert!(expected_origin.is_none(), "{source}: {ds:?}");
        } else {
            assert_eq!(
                expected_origin.map(|r| (r.span.start, r.span.end)),
                Some((concrete, concrete + 1)),
                "{source}: {ds:?}"
            );
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum OracleType {
    Integer,
    Boolean,
    IntegerArray,
    BooleanArray,
}

#[derive(Clone)]
struct OracleConstraints {
    group: [usize; 4],
    ty: [Option<OracleType>; 4],
}

impl OracleConstraints {
    fn new() -> Self {
        Self {
            group: [0, 1, 2, 3],
            ty: [None; 4],
        }
    }
    fn bind(&mut self, variable: usize, ty: OracleType) -> bool {
        let root = self.group[variable];
        if let Some(previous) = self.ty[root] {
            previous != ty
        } else {
            self.ty[root] = Some(ty);
            false
        }
    }
    fn equal(&mut self, left: usize, right: usize) -> bool {
        let a = self.group[left];
        let b = self.group[right];
        if a == b {
            return false;
        }
        if self.ty[a].is_some() && self.ty[b].is_some() && self.ty[a] != self.ty[b] {
            return true;
        }
        let ty = self.ty[a].or(self.ty[b]);
        for group in &mut self.group {
            if *group == b {
                *group = a;
            }
        }
        self.ty[a] = ty;
        false
    }
}

#[derive(Clone, Copy)]
struct ConstraintEvent {
    kind: u8,
    left: usize,
    right: usize,
}

fn oracle_program(events: &[ConstraintEvent]) -> (String, Vec<u64>) {
    let dv = "(if true then {return;}else{return;})";
    let mut source = String::from("fn Main(){");
    for i in 0..4 {
        source.push_str(&format!("var v{i}={dv};"));
    }
    let mut model = OracleConstraints::new();
    let mut errors = Vec::new();
    for (index, event) in events.iter().enumerate() {
        let a = event.left;
        let b = event.right;
        let start = source.len();
        let prior = model.ty[model.group[a]];
        let failure = match event.kind {
            0 => {
                source.push_str(&format!("v{a}=v{b};"));
                model.equal(a, b)
            }
            1 => {
                source.push_str(&format!("v{a}=3;"));
                model.bind(a, OracleType::Integer)
            }
            2 => {
                source.push_str(&format!("v{a}=true;"));
                model.bind(a, OracleType::Boolean)
            }
            3 => {
                source.push_str(&format!("v{a}=[1];"));
                model.bind(a, OracleType::IntegerArray)
            }
            4 => {
                source.push_str(&format!("v{a}=[true];"));
                model.bind(a, OracleType::BooleanArray)
            }
            5 => {
                source.push_str(&format!("let t{index}=v{a}==v{b};"));
                model.equal(a, b)
            }
            6 => {
                source.push_str(&format!("let t{index}=[v{a},1];"));
                model.bind(a, OracleType::Integer)
            }
            7 => {
                source.push_str(&format!(
                    "let t{index}=if true then {{value:v{a};}}else{{value:3;}};"
                ));
                model.bind(a, OracleType::Integer)
            }
            8 => {
                source.push_str(&format!("while v{a}{{}}"));
                model.bind(a, OracleType::Boolean)
            }
            9 => {
                source.push_str(&format!("let t{index}=v{a}+1;"));
                model.bind(a, OracleType::Integer)
            }
            _ => unreachable!(),
        };
        if failure {
            let segment = &source[start..];
            let relative = match event.kind {
                0 => segment.find('=').unwrap() + 1,
                1..=4 => {
                    let value = segment.find('=').unwrap() + 1;
                    if matches!(event.kind, 3 | 4)
                        && matches!(
                            prior,
                            Some(OracleType::IntegerArray | OracleType::BooleanArray)
                        )
                    {
                        value + 1
                    } else {
                        value
                    }
                }
                5 => segment.find("==").unwrap() + 2,
                6 => segment.rfind(',').unwrap() + 1,
                7 => segment.rfind("value:3").unwrap() + 6,
                8 => "while ".len(),
                9 => segment.find('=').unwrap() + 1,
                _ => unreachable!(),
            };
            errors.push((start + relative) as u64);
        }
    }
    source.push('}');
    (source, errors)
}

fn independent_ast_unifier(source: &str) -> Vec<u64> {
    use crate::frontend::parse::{self, Else, Expr, Statement};
    fn index(e: &Expr) -> usize {
        let Expr::Name(name, _) = e else {
            panic!("oracle expected a binding: {e:?}")
        };
        name.strip_prefix('v').unwrap().parse().unwrap()
    }
    fn branch_value(block: &parse::Block) -> &Expr {
        let Some(Statement::Value {
            value: Some(value), ..
        }) = block.statements.last()
        else {
            panic!("oracle expected a branch value: {block:?}")
        };
        value
    }
    let ast = parse::parse("oracle.n", source).unwrap();
    let mut model = OracleConstraints::new();
    let mut failures = Vec::new();
    let body = &ast.functions[0].body.statements;
    assert_eq!(body.len(), 12);
    for statement in body.iter().skip(4) {
        let (fails, at) = match statement {
            Statement::Assign { place, value, .. } => {
                let left = index(place);
                let prior = model.ty[model.group[left]];
                let fails = match value {
                    Expr::Name(..) => model.equal(left, index(value)),
                    Expr::Integer(..) => model.bind(left, OracleType::Integer),
                    Expr::Bool(..) => model.bind(left, OracleType::Boolean),
                    Expr::Array(elements, _) => match elements.as_slice() {
                        [Expr::Integer(..)] => model.bind(left, OracleType::IntegerArray),
                        [Expr::Bool(..)] => model.bind(left, OracleType::BooleanArray),
                        _ => panic!("oracle array form: {value:?}"),
                    },
                    _ => panic!("oracle assignment form: {value:?}"),
                };
                let at = if matches!(value, Expr::Array(..))
                    && matches!(
                        prior,
                        Some(OracleType::IntegerArray | OracleType::BooleanArray)
                    ) {
                    value.span().start + 1
                } else {
                    value.span().start
                };
                (fails, at)
            }
            Statement::Let { value, .. } => match value {
                Expr::Binary {
                    op, left, right, ..
                } if op == "==" => (model.equal(index(left), index(right)), right.span().start),
                Expr::Binary {
                    op, left, right, ..
                } if op == "+" => (
                    model.bind(index(left), OracleType::Integer),
                    left.span().start,
                ),
                Expr::Array(elements, _) => match elements.as_slice() {
                    [first, Expr::Integer(_, span)] => {
                        (model.bind(index(first), OracleType::Integer), span.start)
                    }
                    _ => panic!("oracle array form: {value:?}"),
                },
                Expr::If {
                    then_block,
                    otherwise: Some(otherwise),
                    ..
                } => {
                    let first = branch_value(then_block);
                    let Else::Block(other) = &**otherwise else {
                        panic!("oracle else form")
                    };
                    let second = branch_value(other);
                    let Expr::Integer(_, span) = second else {
                        panic!("oracle branch form: {second:?}")
                    };
                    (model.bind(index(first), OracleType::Integer), span.start)
                }
                _ => panic!("oracle let form: {value:?}"),
            },
            Statement::While { condition, .. } => (
                model.bind(index(condition), OracleType::Boolean),
                condition.span().start,
            ),
            _ => panic!("oracle statement form: {statement:?}"),
        };
        if fails {
            failures.push(at);
        }
    }
    failures
}

fn run_constraint_oracle_with_orders(
    cases: usize,
    seed: u64,
    orders: usize,
) -> std::collections::HashSet<u64> {
    use std::hash::{Hash, Hasher};
    let mut unique = std::collections::HashSet::new();
    let mut random = 0x9e3779b97f4a7c15_u64 ^ seed.wrapping_mul(0xd1b54a32d192ed03);
    for case in 0..cases {
        let mut events = Vec::with_capacity(8);
        for _ in 0..8 {
            random ^= random << 13;
            random ^= random >> 7;
            random ^= random << 17;
            events.push(ConstraintEvent {
                kind: (random % 10) as u8,
                left: ((random >> 8) % 4) as usize,
                right: ((random >> 16) % 4) as usize,
            });
        }
        let mut accepted = None;
        for order in 0..orders {
            let arranged = match order {
                0 => events.clone(),
                1 => events.iter().rev().copied().collect(),
                2 => events[4..].iter().chain(&events[..4]).copied().collect(),
                3 => events
                    .iter()
                    .step_by(2)
                    .chain(events.iter().skip(1).step_by(2))
                    .copied()
                    .collect(),
                _ => unreachable!("only four orderings are defined"),
            };
            let (source, expected_spans) = oracle_program(&arranged);
            let mut hasher = std::collections::hash_map::DefaultHasher::new();
            source.hash(&mut hasher);
            unique.insert(hasher.finish());
            let ast_expected = independent_ast_unifier(&source);
            assert_eq!(
                ast_expected, expected_spans,
                "oracle generator/model mismatch: {source}"
            );
            let result = compile_scalar("oracle.n", &source);
            let is_accepted = result.is_ok();
            if let Some(prior) = accepted {
                assert_eq!(
                    is_accepted, prior,
                    "case {case}, order {order}: {source}: {result:?}"
                );
            }
            accepted = Some(is_accepted);
            match result {
                Ok(_) => assert!(
                    expected_spans.is_empty(),
                    "case {case}, order {order}: {source}"
                ),
                Err(CompileError::Rejected(ds)) => {
                    assert_no_duplicate_diagnostics(&ds, &source);
                    let actual = ds
                        .iter()
                        .filter(|d| matches!(d.code.as_str(), "NX-E0102" | "NX-E1303"))
                        .count();
                    assert_eq!(
                        actual,
                        expected_spans.len(),
                        "case {case}, order {order}: {source}: {ds:?}"
                    );
                    assert_eq!(
                        ds.len(),
                        actual,
                        "case {case}, order {order}: {source}: {ds:?}"
                    );
                    let starts: Vec<_> = ds
                        .iter()
                        .map(|d| d.primary_span.as_ref().unwrap().start)
                        .collect();
                    assert_eq!(
                        starts, expected_spans,
                        "case {case}, order {order}: {source}: {ds:?}"
                    );
                    for d in ds {
                        d.validate("oracle.n", &source).unwrap();
                    }
                }
                Err(other) => panic!("case {case}, order {order}: {source}: {other:?}"),
            }
        }
    }
    unique
}

fn run_constraint_oracle(cases: usize, seed: u64) -> std::collections::HashSet<u64> {
    run_constraint_oracle_with_orders(cases, seed, 2)
}

#[test]
#[ignore = "150000 constraint programs in four statement orders"]
fn statement_permutations_match_independent_constraint_oracle() {
    let unique = run_constraint_oracle_with_orders(37_500, 0x478e_72ac, 4);
    assert!(
        unique.len() >= 150_000,
        "only {} unique programs",
        unique.len()
    );
    eprintln!("unique programs in four orders: {}", unique.len());
}

#[test]
fn generated_monomorphic_constraints_match_independent_graph_oracle() {
    let unique = run_constraint_oracle(512, 0);
    assert!(
        unique.len() >= 1_000,
        "only {} unique constraint programs",
        unique.len()
    );
}

#[test]
#[ignore = "exact 200000-pair constraint graph campaign; run explicitly"]
fn extensive_monomorphic_constraints_match_independent_graph_oracle() {
    let mut unique = std::collections::HashSet::new();
    for seed in 0..5 {
        unique.extend(run_constraint_oracle(40_000, seed));
    }
    assert!(
        unique.len() >= 400_000,
        "only {} unique constraint programs",
        unique.len()
    );
    eprintln!(
        "unique constraint programs across five seeds: {}",
        unique.len()
    );
}

fn deferred_scale_source(family: usize, target: usize) -> String {
    let dv = "(if true then {return;}else{return;})";
    let mut source = if family == 0 {
        format!("fn Main(){{var v0={dv};")
    } else {
        format!("fn Main(){{var v={dv};")
    };
    let mut index = 0;
    loop {
        let unit = match family {
            0 => format!("var v{}=v{};", index + 1, index),
            1 => format!("var w{index}=v;"),
            2 => "v=3;".to_string(),
            3 => format!("let e{index}=v==3;"),
            4 => format!("let e{index}=[v,3];"),
            5 => format!("let e{index}=if true then {{value:v;}}else{{value:3;}};"),
            6 => "while v{}".to_string(),
            7 => format!("var a{index}=[];a{index}=[1];"),
            8 => format!("var a{index}=[];var b{index}=a{index};b{index}=[1];"),
            9 => format!("var x{index}={dv};var a{index}=[x{index}];x{index}=3;"),
            10 => format!("var x{index}={dv};var a{index}=[[x{index}]];x{index}=3;"),
            11 => format!("var x{index}={dv};var y{index}=x{index};"),
            12 => format!("var w{index}=v;w{index}=true;"),
            // Keep the same assignment density at both sizes while leaving
            // the 1 MiB source below the normative solver-step quota.
            13 => "v = [1];".to_string(),
            14 => format!("var x{index}={dv};var y{index}=x{index};x{index}=3;y{index}=4;"),
            _ => unreachable!(),
        };
        if source.len() + unit.len() + 1 > target {
            break;
        }
        source.push_str(&unit);
        index += 1;
    }
    assert!(index > 0);
    source.push('}');
    source
}

#[test]
#[ignore = "exact 15-family 256 KiB to 1 MiB deterministic scale campaign"]
fn deferred_unification_scale_is_linear_in_steps_and_allocations() {
    use std::sync::atomic::Ordering::Relaxed;
    let _campaign = crate::test_alloc::campaign_guard();
    for family in 0..15 {
        let mut measurements = Vec::new();
        for target in [256 * 1024, 1024 * 1024] {
            let source = deferred_scale_source(family, target);
            crate::check::types::TYPE_SOLVER_WORK.store(0, Relaxed);
            crate::test_alloc::reset("semantic_scale_counter.n");
            let compiled = compile_scalar("semantic_scale_counter.n", &source);
            let (bytes, allocations) = crate::test_alloc::snapshot("semantic_scale_counter.n");
            compiled.unwrap_or_else(|e| panic!("family {family}, bytes {}: {e:?}", source.len()));
            let steps = crate::check::types::TYPE_SOLVER_WORK.load(Relaxed);
            assert!(steps > 0 && bytes > 0 && allocations > 0, "family {family}");
            measurements.push((steps, bytes, allocations));
        }
        let smaller = measurements[0];
        let larger = measurements[1];
        for (kind, first, second) in [
            ("steps", smaller.0, larger.0),
            ("bytes", smaller.1, larger.1),
            ("allocations", smaller.2, larger.2),
        ] {
            let ratio = second as f64 / first as f64;
            assert!(
                ratio <= 4.6,
                "family {family}, {kind}: {first} -> {second}, ratio {ratio}"
            );
        }
    }
}

fn nested_assert_eq_source(depth: usize, target_bytes: usize) -> String {
    let value = format!("{}1{}", "[".repeat(depth), "]".repeat(depth));
    let mut source = format!("fn Main(){{let a={value};let b={value};");
    // Keep the 1 MiB case within §11.1's one-million logical-pair budget at
    // depth 100. Spaces enlarge the source without adding type constraints;
    // each added assertion still contributes the same 101 logical pairs.
    let statement = format!("AssertEq(a,b);{}", " ".repeat(192));
    while source.len() + statement.len() < target_bytes {
        source.push_str(&statement);
    }
    source.extend(std::iter::repeat_n(' ', target_bytes - source.len() - 1));
    source.push('}');
    source
}

#[test]
fn assert_eq_logical_pair_charge_is_identical_for_live_and_deferred_values() {
    // F0-0002 §11.1 charges one logical pair per compared type node. The
    // assertion's incremental cost is depth + 1 regardless of when a
    // monomorphic binding became concrete.
    use std::sync::atomic::Ordering::Relaxed;
    let _campaign = crate::test_alloc::campaign_guard();
    fn steps(source: &str) -> u64 {
        crate::check::types::TYPE_SOLVER_WORK.store(0, Relaxed);
        compile_scalar("semantic_scale_counter.n", source).unwrap();
        crate::check::types::TYPE_SOLVER_WORK.load(Relaxed)
    }
    for depth in [0, 1, 25, 50, 100] {
        let value = format!("{}1{}", "[".repeat(depth), "]".repeat(depth));
        let live = format!("fn Main(){{let a={value};let b={value};");
        let deferred = format!(
            "fn Main(){{var a=(if true then {{return;}}else{{return;}});a={value};let b={value};"
        );
        for prefix in [live, deferred] {
            let baseline = format!("{prefix}}}");
            let asserted = format!("{prefix}AssertEq(a,b);}}");
            assert_eq!(
                steps(&asserted) - steps(&baseline),
                depth as u64 + 1,
                "{asserted}"
            );
        }
    }
}

#[test]
#[ignore = "nested AssertEq depth and 256 KiB to 1 MiB deterministic scale campaign"]
fn nested_assert_eq_unification_scale_tracks_depth_and_source_bytes() {
    use std::sync::atomic::Ordering::Relaxed;
    let _campaign = crate::test_alloc::campaign_guard();
    for depth in [25, 50, 100] {
        let mut samples = Vec::new();
        for bytes in [256 * 1024, 1024 * 1024] {
            let source = nested_assert_eq_source(depth, bytes);
            crate::check::types::TYPE_SOLVER_WORK.store(0, Relaxed);
            crate::test_alloc::reset("semantic_scale_counter.n");
            let start = std::time::Instant::now();
            let result = compile_scalar("semantic_scale_counter.n", &source);
            let elapsed = start.elapsed();
            let steps = crate::check::types::TYPE_SOLVER_WORK.load(Relaxed);
            let (allocated, allocations) = crate::test_alloc::snapshot("semantic_scale_counter.n");
            assert!(
                steps > 0 && steps <= 1_000_000,
                "depth {depth}, {bytes}: {result:?}"
            );
            assert!(allocated > 0 && allocations > 0);
            if let Err(CompileError::Rejected(errors)) = &result {
                assert!(
                    errors.iter().any(|d| d.code == "NX-E0003"),
                    "depth {depth}, {bytes}: {errors:?}"
                );
            } else {
                result.unwrap_or_else(|e| panic!("depth {depth}, {bytes}: {e:?}"));
            }
            eprintln!(
                "nested AssertEq depth={depth} source={} steps={steps} elapsed={elapsed:?}",
                source.len()
            );
            samples.push((steps, allocated, allocations));
        }
        for (kind, small, large) in [
            ("steps", samples[0].0, samples[1].0),
            ("bytes", samples[0].1, samples[1].1),
            ("allocations", samples[0].2, samples[1].2),
        ] {
            assert!(
                large as f64 / small as f64 <= 4.6,
                "depth {depth}, {kind}: {small} -> {large}"
            );
        }
    }
    let mut times = Vec::new();
    for depth in [25, 50, 100] {
        let value = format!("{}1{}", "[".repeat(depth), "]".repeat(depth));
        let assertions = "AssertEq(a,b);".repeat(8_000);
        let source = format!("fn Main(){{let a={value};let b={value};{assertions}}}");
        let start = std::time::Instant::now();
        compile_scalar("nested_time.n", &source).unwrap();
        times.push(start.elapsed().as_secs_f64());
    }
    eprintln!("nested AssertEq depth times: {times:?}");
    // The old 15,000-call depth-100 shape must reject the first ungranted
    // logical pair, not spend unbounded work after crossing the quota.
    let value = format!("{}1{}", "[".repeat(100), "]".repeat(100));
    let source = format!(
        "fn Main(){{let a={value};let b={value};{}}}",
        "AssertEq(a,b);".repeat(15_000)
    );
    let ds = rejected(&source);
    assert_eq!(ds.len(), 1, "{ds:?}");
    let quota = &ds[0];
    assert_eq!(
        (quota.code.as_str(), quota.stage, quota.operation),
        ("NX-E0003", Stage::Types, Operation::AnalysisLimit)
    );
    assert_eq!(
        quota.data.get("budget_kind"),
        Some(&crate::diagnostic::DataValue::String(
            "type_solver_steps".into()
        ))
    );
    assert_eq!(
        quota.data.get("limit"),
        Some(&crate::diagnostic::DataValue::UInt(1_000_000))
    );
    assert_eq!(
        quota.data.get("observed"),
        Some(&crate::diagnostic::DataValue::UInt(1_000_001))
    );
}

#[test]
fn nested_deferred_arrays_compile_from_small_caller_stacks() {
    let unresolved = format!("{}{}", "[".repeat(64), "]".repeat(64));
    let concrete = format!("{}1{}", "[".repeat(64), "]".repeat(64));
    let source = format!("fn Main(){{var a={unresolved};a={concrete};}}");
    for bytes in [128, 256, 512] {
        let source = source.clone();
        std::thread::Builder::new()
            .stack_size(bytes * 1024)
            .spawn(move || compile_scalar("small_stack.n", &source))
            .unwrap()
            .join()
            .unwrap()
            .unwrap();
    }
}

fn run_deferred_check_run_fuzz(pairs: usize, seed: u64) -> std::collections::HashSet<u64> {
    use std::hash::{Hash, Hasher};
    let mut unique = std::collections::HashSet::new();
    let dv = "(if true then {return;}else{return;})";
    let types = [
        ("0", "3", "true"),
        ("true", "false", "3"),
        ("[0]", "[3]", "[true]"),
        ("[true]", "[false]", "[3]"),
    ];
    let mut random = 0xd1b54a32d192ed03_u64 ^ seed.wrapping_mul(0x9e3779b97f4a7c15);
    for case in 0..pairs {
        random ^= random << 13;
        random ^= random >> 7;
        random ^= random << 17;
        let (initial, next, conflict) = types[(random as usize) % types.len()];
        let invalid = (random >> 9).is_multiple_of(3);
        let mut body = String::new();
        for j in 0..2 + ((random >> 16) % 5) as usize {
            let segment = match ((random >> (j * 5)) % 6) as u8 {
                0 => format!("v={next};"),
                1 => format!("w={next};"),
                2 => format!("let e{j}=v==w;"),
                3 => format!("let a{j}=[v,w];"),
                4 => format!("let e{j}=if true then {{value:v;}}else{{value:w;}};"),
                _ => format!("while false{{v={next};}}"),
            };
            body.push_str(&segment);
        }
        body.push_str(&format!(
            "v={next};w={};",
            if invalid { conflict } else { next }
        ));
        for first in [initial, dv] {
            let source = format!("fn Main(){{var v={first};var w=v;{body}}}");
            let mut hasher = std::collections::hash_map::DefaultHasher::new();
            source.hash(&mut hasher);
            unique.insert(hasher.finish());
            let checked = super::check_scalar("fuzz.n", &source);
            let compiled = compile_scalar("fuzz.n", &source);
            assert_eq!(
                checked.is_ok(),
                compiled.is_ok(),
                "case {case}: {source}: {checked:?} / {compiled:?}"
            );
            if invalid {
                for result in [checked, compiled] {
                    let Err(CompileError::Rejected(ds)) = result else {
                        panic!("case {case}: missing type mismatch: {source}");
                    };
                    assert_no_duplicate_diagnostics(&ds, &source);
                    assert_eq!(ds.len(), 1, "case {case}: {source}: {ds:?}");
                    assert_eq!(
                        (ds[0].code.as_str(), ds[0].subcause.as_deref()),
                        ("NX-E0102", Some("type_mismatch")),
                        "case {case}: {source}: {ds:?}"
                    );
                    ds[0].validate("fuzz.n", &source).unwrap();
                }
            } else {
                checked.unwrap_or_else(|e| panic!("case {case}: {source}: {e:?}"));
                let program = compiled.unwrap_or_else(|e| panic!("case {case}: {source}: {e:?}"));
                let execution = crate::interp::run_scalar(&program);
                assert!(
                    execution.diagnostics.is_empty(),
                    "case {case}: {source}: {execution:?}"
                );
            }
        }
        // A constructor with a deferred element retains its Array shape in
        // every scalar context. The later assignment resolves the element,
        // and both public entry points must reject the same source.
        let (head, context) = match case % 9 {
            0 => ("fn Main(){", "if [q] {}"),
            1 => ("fn Main(){", "while [q] {}"),
            2 => ("fn Main(){", "let x:I64=[q];"),
            3 => ("fn Main(){var x=1;", "x=[q];"),
            4 => ("fn F(x:view Bool){}fn Main(){", "F([q]);"),
            5 => ("fn Main(){var a=[1];", "let x=a[[q]];"),
            6 => ("fn Main(){", "let x=[q]+1;"),
            7 => ("fn Main(){", "let x=(if [q] then 1 else 2);"),
            _ => ("fn Main()->I64{", "return [q];"),
        };
        let source = format!("{head}var q={dv};{context}q={};}}", case % 17);
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        source.hash(&mut hasher);
        unique.insert(hasher.finish());
        for result in [
            super::check_scalar("fuzz.n", &source),
            compile_scalar("fuzz.n", &source),
        ] {
            let Err(CompileError::Rejected(ds)) = result else {
                panic!("case {case}: deferred Array in scalar context was accepted: {source}");
            };
            assert_no_duplicate_diagnostics(&ds, &source);
            let at = source.find("[q]").unwrap() as u64;
            assert!(
                ds.iter().any(|d| d.code == "NX-E0102"
                    && d.primary_span
                        .as_ref()
                        .is_some_and(|s| s.start == at && s.end == at + 3)),
                "case {case}: {source}: {ds:?}"
            );
        }
        let source = format!(
            "fn Main(){{var r=[];let a=[[{}],(if true then r else {})];}}",
            case % 97,
            (random >> 12) % 101
        );
        for result in [
            super::check_scalar("fuzz.n", &source),
            compile_scalar("fuzz.n", &source),
        ] {
            let Err(CompileError::Rejected(ds)) = result else {
                panic!("case {case}: expected incompatible array branch: {source}");
            };
            assert_no_duplicate_diagnostics(&ds, &source);
        }
    }
    unique
}

fn assert_no_duplicate_diagnostics<T>(ds: &[T], source: &str)
where
    T: std::borrow::Borrow<crate::diagnostic::Diagnostic> + std::fmt::Debug,
{
    for (index, left) in ds.iter().enumerate() {
        let left: &crate::diagnostic::Diagnostic = left.borrow();
        assert!(
            ds[index + 1..].iter().all(|right| {
                let right: &crate::diagnostic::Diagnostic = right.borrow();
                if left.stage == Stage::Types && right.stage == Stage::Types {
                    return left.primary_span != right.primary_span;
                }
                left.code != right.code
                    || left.primary_span != right.primary_span
                    || left.related_spans != right.related_spans
                    || left.data != right.data
            }),
            "duplicate diagnostic for {source}: {ds:?}"
        );
    }
}

#[test]
fn structured_deferred_check_and_run_match_live_twins() {
    run_deferred_check_run_fuzz(128, 0);
}

#[test]
#[ignore = "750000-source structured check/run live/dead/array campaign"]
fn extensive_structured_deferred_check_and_run_match_live_twins() {
    let per_seed = std::env::var("NEXO_FUZZ_PAIRS_PER_SEED")
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(50_000);
    let mut unique = std::collections::HashSet::new();
    for seed in 0..5 {
        unique.extend(run_deferred_check_run_fuzz(per_seed, seed));
    }
    if per_seed == 50_000 {
        assert!(
            unique.len() >= 136_274,
            "only {} unique sources",
            unique.len()
        );
    }
    eprintln!("unique structured twin sources: {}", unique.len());
}
