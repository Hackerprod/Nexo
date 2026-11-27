//! ADR F0-0002 §4.1.2 N(a/e/i), §4.1.2.1, §14.10 RD83/RD101.
use super::i07c_normative_literals::{accepted, invoke, rejected, source};
use std::fs;
use std::process::Command;

const PROFILE: &str = "f0-0002-b-v2";

#[test]
fn c0_span_key_audit_is_line_independent_and_rejects_unreviewed_uses() {
    let result = Command::new("python")
        .args(["-B", "scripts/i07c_c0_span_audit.py", "--self-test"])
        .output()
        .expect("Python must run the C0 G3 audit");
    assert!(
        result.status.success(),
        "stdout: {}\nstderr: {}",
        String::from_utf8_lossy(&result.stdout),
        String::from_utf8_lossy(&result.stderr)
    );
    assert!(
        String::from_utf8_lossy(&result.stdout)
            .contains("line shifts accepted; planted span_key rejected")
    );
}

#[test]
fn rd_causals_determine_rejection_with_optional_leaves() {
    // §4.1.2 N(a), §14.10 RD83/RD101: D/V may not replace a C.
    for (body, start, end, code) in [
        (
            "var q=DV;let a=[1,(if true then false else false),q];q=3;",
            79,
            84,
            "NX-E0102",
        ),
        ("let r=[];let a=[r,0];", 16, 18, "NX-E0103"),
    ] {
        let bad = source(body);
        for verb in ["check", "run"] {
            let (exit, json, _) = invoke(verb, &bad, PROFILE);
            assert_eq!(exit, 1, "{bad}: {json}");
            assert!(
                json.contains(&format!("\"code\":\"{code}\"")),
                "{bad}: {json}"
            );
            assert!(
                json.contains(&format!("\"start\":{start},\"end\":{end}")),
                "{bad}: {json}"
            );
            assert!(
                json.contains("\"outcome\":{\"kind\":\"rejected\"}"),
                "{json}"
            );
            for channel in ["stdout", "events", "observations"] {
                assert!(json.contains(&format!("\"{channel}\":[]")), "{json}");
            }
        }
    }
    accepted(
        &source("var q=DV;let a=[1,(if true then 1 else 1),q];q=3;"),
        PROFILE,
    );
    accepted(&source("let r=[1];let a=[r,[0]];"), PROFILE);
}

#[test]
fn rd_optional_leaves_keep_the_causal_rejection() {
    // §4.1.2.4–5 / RD83: C at first false, other D presence is I.
    let bad = source("var q=DV;let a=[1,(if true then false else false),q];q=3;");
    assert_eq!(&bad[79..84], "false");
    for verb in ["check", "run"] {
        let (exit, json, _) = invoke(verb, &bad, PROFILE);
        assert_eq!(exit, 1, "{json}");
        assert!(
            json.contains("\"code\":\"NX-E0102\",\"subcause\":\"type_mismatch\""),
            "{json}"
        );
        assert!(json.contains("\"start\":79,\"end\":84"), "{json}");
        assert_eq!(json.matches("\"start\":79,\"end\":84").count(), 2, "{json}"); // primary + found_type
        assert!(
            json.contains("\"outcome\":{\"kind\":\"rejected\"}"),
            "{json}"
        );
    }
    accepted(
        &source("var q=DV;let a=[1,(if true then 1 else 1),q];q=3;"),
        PROFILE,
    );
}

#[test]
fn diagnostic_relations_have_valid_distinct_expected_spans() {
    // §4.1.2.3 exact literal outside RD; N(i) forbids expected=self.
    let bad = "fn Main(){var r=[];r=[(r==r)==true];r=[1];}";
    rejected(
        bad,
        PROFILE,
        "NX-E0102",
        Some("type_mismatch"),
        (39, 40),
        &[("expected_type", "Bool"), ("found_type", "I64")],
        &[("expected_type", 21, 35), ("found_type", 39, 40)],
    );
    accepted("fn Main(){var r=[];r=[(r==r)==true];r=[true];}", PROFILE);
}

#[test]
fn grouped_operands_keep_one_causal_primary() {
    // §4.1.2.1 applies only to restriction primaries, not later_adr barriers.
    for (bad, good, token) in [
        (
            source("var v=DV;v=1;let x:Bool=(v);"),
            source("var v=DV;v=1;let x:I64=(v);"),
            "v",
        ),
        (
            source("var v=DV;v=1;let x:Bool=((v));"),
            source("var v=DV;v=1;let x:I64=((v));"),
            "v",
        ),
        (source("let x=(true)+1;"), source("let x=(1)+1;"), "true"),
        (
            source("let x=[1,(true)];"),
            source("let x=[1,(1)];"),
            "true",
        ),
    ] {
        let start = bad.rfind(token).unwrap();
        let expected_relation = if token == "v" {
            vec![(
                "expected_type",
                bad.find("Bool").unwrap(),
                bad.find("Bool").unwrap() + 4,
            )]
        } else if bad.contains("[1,") {
            vec![("expected_type", 17, 18)]
        } else {
            vec![]
        };
        let mut relations = expected_relation;
        relations.push(("found_type", start, start + token.len()));
        rejected(
            &bad,
            PROFILE,
            "NX-E0102",
            Some("type_mismatch"),
            (start, start + token.len()),
            &[
                ("expected_type", if token == "v" { "Bool" } else { "I64" }),
                ("found_type", if token == "v" { "I64" } else { "Bool" }),
            ],
            &relations,
        );
        accepted(&good, PROFILE);
    }
    let status = Command::new("python")
        .args(["-B", "scripts/i07c_c0_span_audit.py"])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .unwrap();
    assert!(
        status.status.success(),
        "{}",
        String::from_utf8_lossy(&status.stderr)
    );
    let text = String::from_utf8_lossy(&status.stdout);
    assert!(
        text.contains("G3 span_key audit passed: 246 sites"),
        "{text}"
    );
    let legacy = Command::new("python")
        .args([
            "-B",
            "scripts/i07c_span_audit.py",
            "--self-test-names-effects-lowering",
        ])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .expect("the raw .span() audit for names, effects, and lowering must run");
    assert!(
        legacy.status.success(),
        "stdout: {}\nstderr: {}",
        String::from_utf8_lossy(&legacy.stdout),
        String::from_utf8_lossy(&legacy.stderr)
    );
}

#[test]
fn deferred_operand_keeps_its_source_origin_after_later_assignment() {
    // ADR §4.1.2.1,6: completion supplies a type, not a new source origin.
    let bad = "fn Main(){var s=[];let a=[s[0],(if true then s else s)];s=[1];}";
    rejected(
        bad,
        PROFILE,
        "NX-E0102",
        Some("type_mismatch"),
        (32, 53),
        &[("expected_type", "I64"), ("found_type", "Array<I64>")],
        &[("expected_type", 26, 30), ("found_type", 32, 53)],
    );
    accepted(
        "fn Main(){var s=[1];let a=[s[0],(if true then s[0] else s[0])];s=[1];}",
        PROFILE,
    );
}

#[test]
fn failed_nested_if_cannot_retype_an_earlier_projection() {
    // ADR §4.1.2.1,6: a failed result cannot lend a later fact to q[x][0].
    let bad = "fn Main(){let x=1;var q=[];var r=[];let a0=[r[0],(if true then q[x][0] else (if r then [1] else q))];r=[1];}";
    for verb in ["check", "run"] {
        let (exit, json, _) = invoke(verb, bad, PROFILE);
        assert_eq!(exit, 1, "{json}");
        assert!(
            json.contains("\"code\":\"NX-E0102\",\"subcause\":\"type_mismatch\""),
            "{json}"
        );
        assert!(json.contains("\"start\":80,\"end\":81"), "{json}");
        assert!(!json.contains("\"subcause\":\"not_indexable\""), "{json}");
    }
}

#[test]
fn repeated_check_and_run_keep_diagnostics_deterministic() {
    // §4.1.2 N(e), §13.9: the same file and bytes for both runs per verb.
    let path = std::env::temp_dir()
        .join(format!("i07c-s6-repeat-{}.n", std::process::id()))
        .to_string_lossy()
        .into_owned();
    for body in [
        "var q=DV;let a=[1,(if true then false else false),q];q=3;",
        "let p=true;var r=[];let a:I64=(r==p);r=[1];",
        "let r=[];let a=[r,0];",
        "var r=DV;let a0=DV+(DV-Len(r));let b=[a0,r];",
        "var v=DV;var w=v;v=3;w=true;",
    ] {
        fs::write(&path, source(body)).unwrap();
        for verb in ["check", "run"] {
            let run = || {
                Command::new(env!("CARGO_BIN_EXE_nexo"))
                    .args([verb, &path, "--profile", PROFILE, "--format", "json"])
                    .output()
                    .unwrap()
            };
            let first = run();
            let second = run();
            assert_eq!(first.status, second.status);
            assert_eq!(first.stdout, second.stdout, "{body}");
            assert_eq!(first.stderr, second.stderr, "{body}");
            let json = String::from_utf8_lossy(&first.stdout);
            let starts: Vec<usize> = json
                .split("\"primary_span\":{")
                .skip(1)
                .map(|part| {
                    part.split("\"start\":")
                        .nth(1)
                        .unwrap()
                        .split(',')
                        .next()
                        .unwrap()
                        .parse()
                        .unwrap()
                })
                .collect();
            assert!(
                starts.windows(2).all(|pair| pair[0] <= pair[1]),
                "{body}: {json}"
            );
        }
    }
    fs::remove_file(path).unwrap();
    accepted(
        &source("var r=DV;let a0=DV+(DV-Len(r));let b=[a0,1];"),
        PROFILE,
    );
}

#[test]
fn small_deterministic_source_batch_has_no_internal_errors() {
    // §4.1.2 N(f): bounded, reproducible sources, not an ignored campaign.
    for (number, tail) in [
        "let x=[1,true];",
        "let x=(true)+1;",
        "let x=[1,(true)];",
        "let x=DV+true;",
        "let x=DV+1;",
        "let x=[1,1];",
        "let x=(1)==true;",
        "let x=(1)==1;",
    ]
    .into_iter()
    .enumerate()
    {
        let program = source(&format!("let seed{number}={number};{tail}"));
        for verb in ["check", "run"] {
            let (exit, json, stderr) = invoke(verb, &program, PROFILE);
            assert!(
                exit == 0 || exit == 1,
                "{program}: {exit}: {json}: {stderr}"
            );
            assert!(
                !json.contains("internal_error") && !json.contains("InvalidIr"),
                "{program}: {json}"
            );
            assert!(stderr.is_empty(), "{program}: {stderr}");
        }
    }
}

#[test]
fn grouped_domain_union_and_operator_sites_keep_inner_primary() {
    // F0-0002-lagunas (primario bajo agrupaciones); §4.1.2.3 Len (RD86(b) agrupado);
    // §4.1.2.4 NX-E1303; N(d) una denuncia por operacion.
    for (open, close, start) in [("(", ")", 34usize), ("((", "))", 35)] {
        let bad = format!("fn Main(){{let q=3;let a:Bool=Len({open}q{close});}}");
        assert_eq!(&bad[start..start + 1], "q");
        rejected(
            &bad,
            PROFILE,
            "NX-E0102",
            Some("not_indexable"),
            (start, start + 1),
            &[("expected_type", "domain:array"), ("found_type", "I64")],
            &[("found_type", start, start + 1)],
        );
    }
    accepted("fn Main(){let q=[3];let a:I64=Len((q));}", PROFILE);
    rejected(
        "fn Main(){let x=if true then 1 else (true);}",
        PROFILE,
        "NX-E1303",
        None,
        (37, 41),
        &[("expected_type", "I64"), ("found_type", "Bool")],
        &[("expected_type", 29, 30), ("found_type", 37, 41)],
    );
    accepted("fn Main(){let x=if true then 1 else (1);}", PROFILE);
    rejected(
        "fn Main(){let x=(true)+(true);}",
        PROFILE,
        "NX-E0102",
        Some("type_mismatch"),
        (17, 21),
        &[("expected_type", "I64"), ("found_type", "Bool")],
        &[("found_type", 17, 21)],
    );
    accepted("fn Main(){let x=(1)+(1);}", PROFILE);
}
