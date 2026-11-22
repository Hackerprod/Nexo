//! ADR F0-0002 §4.1.2 N(a/e/i), §4.1.2.1, §14.10 RD83/RD101.
use super::i07c_normative_literals::{accepted, invoke, rejected, source};
use std::fs;
use std::process::Command;

const PROFILE: &str = "f0-0002-b-v2";

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
        .args(["-B", "scripts/i07c_span_audit.py"])
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
        text.contains("pendiente_decision: 3 lines=8283,12301,14613"),
        "{text}"
    );
    let status = Command::new("python")
        .args(["-B", "scripts/i07c_span_audit.py", "--self-test"])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .status()
        .unwrap();
    assert!(status.success());
}

#[test]
fn repeated_check_and_run_keep_diagnostics_deterministic() {
    // §4.1.2 N(e), §13.9: the same file and bytes for both runs per verb.
    let folder = format!("{}/.scratch/tmp-s6", env!("CARGO_MANIFEST_DIR"));
    let path = format!("{folder}/i07c-s6-repeat-{}.n", std::process::id());
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
