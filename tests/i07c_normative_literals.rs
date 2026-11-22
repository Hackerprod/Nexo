//! ADR F0-0002 §§4.1.2.3, 14.10, 14.11; F0-0003 §§3.2, 8.2/M05.
use std::fs;
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

const DV: &str = "(if true then {return;} else {return;})";
static NEXT: AtomicU64 = AtomicU64::new(0);

pub(super) fn source(body: &str) -> String {
    format!("fn Main(){{{}}}", body.replace("DV", DV))
}

pub(super) fn invoke(verb: &str, source: &str, profile: &str) -> (i32, String, String) {
    let id = NEXT.fetch_add(1, Ordering::Relaxed);
    let path = std::env::temp_dir()
        .join(format!("i07c-s6-{}-{id}.n", std::process::id()))
        .to_string_lossy()
        .into_owned();
    fs::write(&path, source).unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_nexo"))
        .args([verb, &path, "--profile", profile, "--format", "json"])
        .output()
        .unwrap();
    fs::remove_file(path).unwrap();
    (
        output.status.code().expect("Nexo must not crash"),
        String::from_utf8(output.stdout).unwrap(),
        String::from_utf8(output.stderr).unwrap(),
    )
}

pub(super) fn accepted(source: &str, profile: &str) {
    for verb in ["check", "run"] {
        let (exit, json, stderr) = invoke(verb, source, profile);
        assert_eq!(exit, 0, "{verb}: {source}: {json}: {stderr}");
        assert!(
            json.contains(&format!(
                "\"outcome\":{{\"kind\":\"{}\"}}",
                if verb == "check" { "checked" } else { "unit" }
            )),
            "{source}: {json}"
        );
        for field in ["diagnostics", "stdout", "events", "observations"] {
            assert!(
                json.contains(&format!("\"{field}\":[]")),
                "{source}: {json}"
            );
        }
        assert!(stderr.is_empty(), "{source}: {stderr}");
    }
}

#[allow(clippy::too_many_arguments)]
pub(super) fn rejected(
    source: &str,
    profile: &str,
    code: &str,
    subcause: Option<&str>,
    span: (usize, usize),
    data: &[(&str, &str)],
    related: &[(&str, usize, usize)],
) {
    assert!(source.is_ascii() && span.0 < span.1 && span.1 <= source.len());
    for verb in ["check", "run"] {
        let (exit, json, stderr) = invoke(verb, source, profile);
        assert_eq!(exit, 1, "{verb}: {source}: {json}: {stderr}");
        assert!(
            json.contains("\"outcome\":{\"kind\":\"rejected\"}"),
            "{json}"
        );
        assert_eq!(
            json.matches("\"code\":\"NX-").count(),
            1,
            "{source}: {json}"
        );
        let rule = format!(
            "{code}{}",
            subcause.map(|s| format!("/{s}")).unwrap_or_default()
        );
        for fragment in [
            format!("\"code\":\"{code}\""),
            format!(
                "\"subcause\":{}",
                subcause
                    .map(|s| format!("\"{s}\""))
                    .unwrap_or_else(|| "null".into())
            ),
            format!("\"rule\":\"{rule}\""),
            format!("\"explanation_id\":\"{rule}\""),
            format!("\"start\":{},\"end\":{}", span.0, span.1),
            "\"stage\":\"types\"".into(),
            format!(
                "\"operation\":\"{}\"",
                if code == "NX-E0103" {
                    "types.infer"
                } else {
                    "types.check"
                }
            ),
            "\"cause_chain\":[]".into(),
            "\"repair\":[]".into(),
            "\"feature_id\":null".into(),
            "\"unexpected_token\":null".into(),
            "\"expected_tokens\":[]".into(),
            "\"parser_context\":null".into(),
            "\"recognized_prefix\":null".into(),
            "\"stdout\":[]".into(),
            "\"events\":[]".into(),
            "\"observations\":[]".into(),
        ] {
            assert!(
                json.contains(&fragment),
                "{source}: missing {fragment}: {json}"
            );
        }
        let expected_data = if data.is_empty() {
            "\"data\":{}".to_owned()
        } else {
            format!(
                "\"data\":{{{}}}",
                data.iter()
                    .map(|(key, value)| format!("\"{key}\":\"{value}\""))
                    .collect::<Vec<_>>()
                    .join(",")
            )
        };
        assert!(
            json.contains(&expected_data),
            "{source}: {expected_data}: {json}"
        );
        assert_eq!(json.matches("\"role\":").count(), related.len(), "{json}");
        for (role, start, end) in related {
            assert!(
                json.contains(&format!("\"role\":\"{role}\",\"span\":{{\"file\":"))
                    && json.contains(&format!("\"start\":{start},\"end\":{end}")),
                "{source}: {json}"
            );
        }
        assert!(stderr.is_empty(), "{source}: {stderr}");
    }
}

const A2: &str = "f0-0002-b-v2";

#[test]
fn concrete_array_twin_accepts_matching_assignment() {
    // §4.1.2.3, table "Gemelo negativo", §13.15.
    accepted("fn Main(){var r=[true];r=[true];}", A2);
    rejected(
        "fn Main(){var r=[true];r=[1];}",
        A2,
        "NX-E0102",
        Some("type_mismatch"),
        (26, 27),
        &[("expected_type", "Bool"), ("found_type", "I64")],
        &[("expected_type", 16, 22), ("found_type", 26, 27)],
    );
}

#[test]
fn independent_empty_array_survives_failed_nominal_domain() {
    // §4.1.2.3 b2d + independent Array constructor; §13.17.
    let bad = source("var r=DV;let a0=DV+(DV-Len(r));let b=[a0,r];let free=[];");
    assert_eq!(&bad[148..149], "r");
    assert_eq!(&bad[174..176], "[]");
    for verb in ["check", "run"] {
        let (exit, json, _) = invoke(verb, &bad, A2);
        assert_eq!(exit, 1, "{json}");
        assert_eq!(json.matches("\"code\":\"NX-").count(), 2, "{json}");
        assert!(
            json.contains("\"code\":\"NX-E0102\",\"subcause\":\"not_indexable\""),
            "{json}"
        );
        assert!(json.contains("\"start\":148,\"end\":149"), "{json}");
        assert!(
            json.contains("\"expected_type\":\"domain:array\",\"found_type\":\"I64\""),
            "{json}"
        );
        assert!(
            json.contains("\"code\":\"NX-E0103\",\"subcause\":null"),
            "{json}"
        );
        assert!(json.contains("\"start\":174,\"end\":176"), "{json}");
        assert!(
            json.contains("\"outcome\":{\"kind\":\"rejected\"}"),
            "{json}"
        );
    }
    rejected(
        &source("var r=DV;let a0=DV+(DV-Len(r));let b=[a0,r];"),
        A2,
        "NX-E0102",
        Some("not_indexable"),
        (148, 149),
        &[("expected_type", "domain:array"), ("found_type", "I64")],
        &[("found_type", 148, 149)],
    );
    accepted(
        &source("var r=DV;let a0=DV+(DV-Len(r));let b=[a0,1];let free=[1];"),
        A2,
    );
}

#[test]
fn literal_alias_constraints_accept_equivalent_source_orders() {
    // §14.10 TD01–TD03: source permutations, not internal solver permutation.
    for body in [
        "var v=DV;var w=v;v=3;w=4;",
        "var v=DV;var w=v;w=4;v=3;",
        "var w=DV;var v=w;w=4;v=3;",
        "var v=3;var w=v;v=3;w=4;",
        "var v=3;var w=v;w=4;v=3;",
        "var v=DV;var w=true;w=v;v=false;",
    ] {
        accepted(&source(body), A2);
    }
}

#[test]
fn td_alias_conflicts_have_cli_rejection_contract() {
    // §14.10 TD01–03; §14.11 DG07–08; §13.15.
    for (body, token, wanted, found, origin, positive) in [
        (
            "var v=DV;var w=v;v=3;w=true;",
            "true",
            "I64",
            "Bool",
            (66, 67),
            "var v=DV;var w=v;v=3;w=4;",
        ),
        (
            "var v=3;var w=v;v=3;w=true;",
            "true",
            "I64",
            "Bool",
            (16, 17),
            "var v=3;var w=v;v=3;w=4;",
        ),
        (
            "var v=DV;var w=true;w=v;v=3;",
            "3",
            "Bool",
            "I64",
            (62, 66),
            "var v=DV;var w=true;w=v;v=false;",
        ),
        (
            "var a:Array<I64>=[0];a[0]=true;",
            "true",
            "I64",
            "Bool",
            (16, 26),
            "var a:Array<I64>=[0];a[0]=1;",
        ),
        (
            "var a=[0];a[0]=true;",
            "true",
            "I64",
            "Bool",
            (16, 19),
            "var a=[0];a[0]=1;",
        ),
    ] {
        let bad = source(body);
        let start = bad.rfind(token).unwrap();
        rejected(
            &bad,
            A2,
            "NX-E0102",
            Some("type_mismatch"),
            (start, start + token.len()),
            &[("expected_type", wanted), ("found_type", found)],
            &[
                ("expected_type", origin.0, origin.1),
                ("found_type", start, start + token.len()),
            ],
        );
        accepted(&source(positive), A2);
    }
}

#[test]
fn never_joins_run_to_unit_without_observations() {
    // §14.10 TD04/TD05 and background pair.
    for body in [
        "let x=(DV)==true;",
        "let x=true==DV;",
        "let x=[DV,true];",
        "let x=[true,DV];",
        "let x=DV+1;",
    ] {
        accepted(&source(body), A2);
    }
    let bad = source("let x=DV+true;");
    let start = bad.rfind("true").unwrap();
    rejected(
        &bad,
        A2,
        "NX-E0102",
        Some("type_mismatch"),
        (start, start + 4),
        &[("expected_type", "I64"), ("found_type", "Bool")],
        &[("found_type", start, start + 4)],
    );
}

#[test]
fn bottom_numeric_operands_accept_literal_one() {
    // §14.10 pairs of background, §13.15.
    for (positive, negative) in [
        ("let x=DV+1;", "let x=DV+true;"),
        ("let x=1+DV;", "let x=true+DV;"),
    ] {
        accepted(&source(positive), A2);
        let bad = source(negative);
        let start = bad.find("true").unwrap();
        let start = if negative.starts_with("let x=DV") {
            bad.rfind("true").unwrap()
        } else {
            start
        };
        rejected(
            &bad,
            A2,
            "NX-E0102",
            Some("type_mismatch"),
            (start, start + 4),
            &[("expected_type", "I64"), ("found_type", "Bool")],
            &[("found_type", start, start + 4)],
        );
    }
}

#[test]
fn bottom_array_operands_accept_matching_last_element() {
    // §§4.1.1, 4.1.2.4 and 14.10 pairs of background.
    for (positive, negative) in [
        ("let x=[DV,1,1];", "let x=[DV,1,true];"),
        ("let x=[1,DV,1];", "let x=[1,DV,true];"),
    ] {
        accepted(&source(positive), A2);
        let bad = source(negative);
        let start = bad.rfind("true").unwrap();
        let origin = if negative.starts_with("let x=[1,") {
            17
        } else {
            bad.rfind(",1,").unwrap() + 1
        };
        rejected(
            &bad,
            A2,
            "NX-E0102",
            Some("type_mismatch"),
            (start, start + 4),
            &[("expected_type", "I64"), ("found_type", "Bool")],
            &[
                ("expected_type", origin, origin + 1),
                ("found_type", start, start + 4),
            ],
        );
    }
}

#[test]
fn rd87_run_emits_only_the_literal_print_observation() {
    // §14.10 RD87(a)–(d): only (c) prints [3]; §12.4.
    for body in [
        "var q=DV;Print(q);q=[3];",
        "var q=DV;let a:I64=Len(q);q=[3];",
        "let q=[3];let a:I64=Len(q);",
    ] {
        accepted(&source(body), A2);
    }
    let actual = source("let q=[3];Print(q);");
    let (check, json, _) = invoke("check", &actual, A2);
    assert_eq!(check, 0, "{json}");
    assert!(
        json.contains("\"outcome\":{\"kind\":\"checked\"}"),
        "{json}"
    );
    let (run, json, _) = invoke("run", &actual, A2);
    assert_eq!(run, 0, "{json}");
    assert!(json.contains("\"outcome\":{\"kind\":\"unit\"}"), "{json}");
    assert!(json.contains("\"stdout\":[\"[3]\"]"), "{json}");
    assert!(
        json.contains("\"events\":[]") && json.contains("\"diagnostics\":[]"),
        "{json}"
    );
}

#[test]
fn previously_open_root_fixed_to_option_rejects_result_pattern() {
    // F0-0002 §4.1.1 and F0-0003 §§3.2, 8.2/M05: derived open-root variant.
    let bad = source("let l0=DV;let b:Option<I64>=l0;match l0{Ok(n)=>{},_=>{}}");
    assert_eq!(&bad[87..92], "Ok(n)");
    rejected(
        &bad,
        "f0-0003-b-v1",
        "NX-E1325",
        Some("constructor_type"),
        (87, 92),
        &[
            ("constructor", "prelude:Ok"),
            ("scrutinee_type", "Option<I64>"),
        ],
        &[],
    );
    accepted(
        &source("let l0=DV;let b:Option<I64>=l0;match l0{Some(n)=>{},_=>{}}"),
        "f0-0003-b-v1",
    );
}
