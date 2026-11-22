use std::fs;
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

fn invoke(verb: &str, source: &str, profile: &str) -> (i32, String) {
    static NEXT: AtomicU64 = AtomicU64::new(0);
    let folder = format!(
        "C:/nexo-work/tmp/f0005-operand-domain-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    );
    fs::create_dir_all(&folder).unwrap();
    fs::write(format!("{folder}/case.n"), source).unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_nexo"))
        .current_dir(&folder)
        .args([verb, "case.n", "--profile", profile, "--format", "json"])
        .output()
        .unwrap();
    fs::remove_dir_all(folder).unwrap();
    (
        output.status.code().unwrap(),
        String::from_utf8(output.stdout).unwrap().trim().to_owned(),
    )
}

// F5 §§2.1, 8.1 and 9.2 (F5-O06/F5-O08): the intrinsic arithmetic domain
// requires I64 at its operand, with found_type on that operand and no
// source-origin expected_type. Grouping does not enlarge the primary span.
fn arithmetic_bool_diagnostic(
    start: usize,
    end: usize,
    expected_origin: Option<(usize, usize)>,
) -> String {
    let expected_relation = expected_origin.map_or(String::new(), |(first, last)| {
        format!(
            r#"{{"role":"expected_type","span":{{"file":"case.n","start":{first},"end":{last}}}}},"#
        )
    });
    format!(
        r#"{{"schema_version":1,"diagnostic_namespace":"Nexo-proposed-v1","profile_id":"f0-0005-b-v1","code":"NX-E0102","subcause":"type_mismatch","rule":"NX-E0102/type_mismatch","explanation_id":"NX-E0102/type_mismatch","severity":"error","stage":"types","message":"Invalid name, call or type in the proposed profile.","operation":"types.check","primary_span":{{"file":"case.n","start":{start},"end":{end}}},"related_spans":[{expected_relation}{{"role":"found_type","span":{{"file":"case.n","start":{start},"end":{end}}}}}],"cause_chain":[],"feature_id":null,"unexpected_token":null,"expected_tokens":[],"expected_tokens_kind":null,"parser_context":null,"recognized_prefix":null,"repair":[],"data":{{"expected_type":"I64","found_type":"Bool"}}}}"#
    )
}

fn rejected(diagnostic: &str) -> String {
    format!(
        "{{\"schema_version\":1,\"profile_id\":\"f0-0005-b-v1\",\"outcome\":{{\"kind\":\"rejected\"}},\"stdout\":[],\"events\":[],\"observations\":[],\"observation_bytes\":0,\"reserved_observation_bytes\":0,\"diagnostics\":[{diagnostic}]}}"
    )
}

#[test]
fn grouped_logical_operands_preserve_arithmetic_domain_error() {
    // Spans are byte offsets of the ungrouped logical operands in each
    // literal source, derived from the ADR rule, not from compiler output.
    for (source, start, end, positive) in [
        (
            "fn Main(){let w=1+(true or false);}",
            19,
            32,
            "fn Main(){let w=1+(1+2);}",
        ),
        (
            "fn Main(){let w=(false and true)*2;}",
            17,
            31,
            "fn Main(){let w=(1+2)*2;}",
        ),
        (
            "fn Main(){let w=-(true or false);}",
            18,
            31,
            "fn Main(){let w=-(1+2);}",
        ),
    ] {
        let expected = rejected(&arithmetic_bool_diagnostic(start, end, None));
        for verb in ["check", "run"] {
            let (status, output) = invoke(verb, source, "f0-0005-b-v1");
            assert_eq!(
                (status, output.as_str()),
                (1, expected.as_str()),
                "{verb} {source}"
            );
            let (status, output) = invoke(verb, positive, "f0-0005-b-v1");
            assert_ne!(status, 2, "{verb} {positive}: {output}");
            assert!(
                output.contains("\"diagnostics\":[]")
                    || output.contains("\"pending_scope\":\"L08\""),
                "{verb} {positive}: {output}"
            );
        }
    }
}

#[test]
fn logical_right_operand_is_checked_for_each_integer_operator_and_comparison() {
    // F5 §8.1: arithmetic has an intrinsic I64 domain; six comparisons
    // compare unlike concrete operands on the right with a left-origin span.
    for logical in ["true or false", "false and true"] {
        for op in ["+", "-", "*", "/", "%", "<", "<=", ">", ">=", "==", "!="] {
            let source = format!("fn Main(){{let w=1 {op} ({logical});}}");
            let start = 20 + op.len();
            let end = start + logical.len();
            let origin = matches!(op, "<" | "<=" | ">" | ">=" | "==" | "!=").then_some((16, 17));
            let expected = rejected(&arithmetic_bool_diagnostic(start, end, origin));
            for verb in ["check", "run"] {
                let (status, output) = invoke(verb, &source, "f0-0005-b-v1");
                assert_eq!(
                    (status, output.as_str()),
                    (1, expected.as_str()),
                    "{verb} {source}"
                );
            }
        }
    }
}

#[test]
fn compound_assignments_check_grouped_logical_rhs_domain() {
    // F5 §3.3 and §8.1: RHS of each compound place is intrinsically I64.
    for logical in ["true or false", "false and true"] {
        let expected = rejected(&arithmetic_bool_diagnostic(22, 22 + logical.len(), None));
        for op in ["+=", "-=", "*=", "/=", "%="] {
            let source = format!("fn Main(){{var w=1;w{op}({logical});}}");
            for verb in ["check", "run"] {
                let (status, output) = invoke(verb, &source, "f0-0005-b-v1");
                assert_eq!(
                    (status, output.as_str()),
                    (1, expected.as_str()),
                    "{verb} {source}"
                );
            }
        }
    }
}

#[test]
fn logical_results_respect_annotated_and_inferred_consumer_types() {
    // F5 §8.1 and A2 §13.15: a known source origin contributes exactly one
    // expected_type relation; intrinsic selector I64 has no such relation.
    for (source, start, end, origin, positive) in [
        (
            "fn Main(){let w:I64=(true or false);}",
            21,
            34,
            Some((16, 19)),
            "fn Main(){let w:I64=(1+2);}",
        ),
        (
            "fn Main(){let w=[1,(true or false)];}",
            20,
            33,
            Some((17, 18)),
            "fn Main(){let w=[1,(1+2)];}",
        ),
        (
            "fn Main(){let a=[1];let w=a[(true or false)];}",
            29,
            42,
            None,
            "fn Main(){let a=[1];let w=a[(0+0)];}",
        ),
        (
            "fn F(x:view I64){}fn Main(){F((true or false));}",
            31,
            44,
            Some((12, 15)),
            "fn F(x:view I64){}fn Main(){F((1+2));}",
        ),
        (
            "fn F()->I64{return (true or false);}fn Main(){}",
            20,
            33,
            Some((8, 11)),
            "fn F()->I64{return (1+2);}fn Main(){}",
        ),
        (
            "fn Main(){var w=1;w=(true or false);}",
            21,
            34,
            Some((16, 17)),
            "fn Main(){var w=1;w=(1+2);}",
        ),
    ] {
        let expected = rejected(&arithmetic_bool_diagnostic(start, end, origin));
        for verb in ["check", "run"] {
            let (status, output) = invoke(verb, source, "f0-0005-b-v1");
            assert_eq!(
                (status, output.as_str()),
                (1, expected.as_str()),
                "{verb} {source}"
            );
            let (status, output) = invoke(verb, positive, "f0-0005-b-v1");
            assert_eq!(status, 0, "{verb} {positive}: {output}");
            assert!(
                output.contains("\"diagnostics\":[]"),
                "{verb} {positive}: {output}"
            );
        }
    }
}

#[test]
fn historical_profiles_keep_logical_syntax_boundary_and_f5_valid_domains() {
    // D15 §3.2: the old profiles reject and/or at parse, before type checking.
    for profile in ["f0-0002-b-v2", "f0-0003-b-v1"] {
        for verb in ["check", "run"] {
            let (status, output) = invoke(verb, "fn Main(){let w=1+(true or false);}", profile);
            assert_eq!(status, 1, "{profile} {verb}: {output}");
            assert!(
                output.contains("\"code\":\"NX-E0002\""),
                "{profile} {verb}: {output}"
            );
            assert!(
                output.contains("\"stage\":\"parse\""),
                "{profile} {verb}: {output}"
            );
        }
    }
    // §2.1: numeric nesting is valid; Boolean comparison and logic retain
    // their own domain (L08 is the current, explicitly specified lower gate).
    for source in [
        "fn Main(){let w=1+(1+2);}",
        "fn Main(){let w=(1<2) and true;}",
        "fn Main(){let w=(true or false)==false;}",
    ] {
        for verb in ["check", "run"] {
            let (status, output) = invoke(verb, source, "f0-0005-b-v1");
            assert_ne!(status, 2, "{verb} {source}: {output}");
            assert!(
                output.contains("\"diagnostics\":[]")
                    || output.contains("\"pending_scope\":\"L08\""),
                "{verb} {source}: {output}"
            );
        }
    }
}
