use std::fs;
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

fn invoke(verb: &str, source: &str, profile: &str) -> (i32, String) {
    static NEXT: AtomicU64 = AtomicU64::new(0);
    let folder = format!(
        "C:/nexo-work/tmp/f0005-b1b-{}-{}",
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
        String::from_utf8(output.stdout).unwrap().trim().into(),
    )
}

fn rejected(diag: &str) -> String {
    format!(
        "{{\"schema_version\":1,\"profile_id\":\"f0-0005-b-v1\",\"outcome\":{{\"kind\":\"rejected\"}},\"stdout\":[],\"events\":[],\"observations\":[],\"observation_bytes\":0,\"reserved_observation_bytes\":0,\"diagnostics\":[{diag}]}}"
    )
}

#[test]
fn pending_f5_destinations_and_const_have_exact_public_diagnostics() {
    let cases = [
        (
            "fn Main(){for x in [1]{}}",
            r#"{"schema_version":1,"diagnostic_namespace":"Nexo-proposed-v1","profile_id":"f0-0005-b-v1","code":"NX-E0006","subcause":null,"rule":"NX-E0006","explanation_id":"NX-E0006","severity":"error","stage":"types","message":"Capacidad pendiente de implementación en L06.","operation":"types.check","primary_span":{"file":"case.n","start":10,"end":13},"related_spans":[],"cause_chain":[],"feature_id":"implementation_limit","unexpected_token":null,"expected_tokens":[],"expected_tokens_kind":null,"parser_context":null,"recognized_prefix":null,"repair":[],"data":{"pending_scope":"L06"}}"#,
        ),
        (
            "fn Main(){var a=[1];Push(edit a,2);}",
            r#"{"schema_version":1,"diagnostic_namespace":"Nexo-proposed-v1","profile_id":"f0-0005-b-v1","code":"NX-E0006","subcause":null,"rule":"NX-E0006","explanation_id":"NX-E0006","severity":"error","stage":"types","message":"Capacidad pendiente de implementación en L07.","operation":"types.check","primary_span":{"file":"case.n","start":20,"end":24},"related_spans":[],"cause_chain":[],"feature_id":"implementation_limit","unexpected_token":null,"expected_tokens":[],"expected_tokens_kind":null,"parser_context":null,"recognized_prefix":null,"repair":[],"data":{"pending_scope":"L07"}}"#,
        ),
        (
            "const k:I64=comptime 1;fn Main(){}",
            r#"{"schema_version":1,"diagnostic_namespace":"Nexo-proposed-v1","profile_id":"f0-0005-b-v1","code":"NX-E0006","subcause":null,"rule":"NX-E0006","explanation_id":"NX-E0006","severity":"error","stage":"types","message":"Capacidad pendiente de implementación en L09.","operation":"types.check","primary_span":{"file":"case.n","start":0,"end":5},"related_spans":[],"cause_chain":[],"feature_id":"implementation_limit","unexpected_token":null,"expected_tokens":[],"expected_tokens_kind":null,"parser_context":null,"recognized_prefix":null,"repair":[],"data":{"pending_scope":"L09"}}"#,
        ),
    ];
    for (source, diagnostic) in cases {
        for verb in ["check", "run"] {
            let (status, output) = invoke(verb, source, "f0-0005-b-v1");
            assert_eq!(status, 1, "{verb} {source}: {output}");
            assert_eq!(output, rejected(diagnostic), "{verb} {source}");
        }
    }
}

#[test]
fn historical_profiles_keep_their_operator_const_and_prelude_boundaries() {
    for (source, feature) in [
        ("const k:I64=comptime 1;fn Main(){}", "comptime"),
        ("fn Main(){var a=[1];Push(edit a,2);}", "use_kind"),
        ("fn Main(){for x in [1]{}}", "for_iteration"),
    ] {
        for profile in ["f0-0002-b-v2", "f0-0003-b-v1"] {
            let (status, output) = invoke("check", source, profile);
            assert_eq!(status, 1, "{profile} {source}: {output}");
            assert!(output.contains(feature), "{profile} {source}: {output}");
        }
    }
}

#[test]
fn completed_f5_operator_rules_reach_types_or_lower_in_order() {
    for (source, stage, scope) in [
        ("fn Main(){let x=1*2;}", "lower", "L08"),
        ("fn Main(){let x=1!=2;}", "lower", "L08"),
        ("fn Main(){let x=1;let y=-x;}", "lower", "L08"),
        ("fn Main(){for i in 0..<2{}}", "types", "L05"),
    ] {
        for verb in ["check", "run"] {
            let (status, output) = invoke(verb, source, "f0-0005-b-v1");
            assert_eq!(status, 1, "{source}: {output}");
            assert!(
                output.contains(&format!("\"stage\":\"{stage}\"")),
                "{source}: {output}"
            );
            assert!(
                output.contains(&format!("\"pending_scope\":\"{scope}\"")),
                "{source}: {output}"
            );
        }
    }
    let (status, output) = invoke("check", "fn Main(){let x=true*2;}", "f0-0005-b-v1");
    assert_eq!(status, 1, "{output}");
    assert!(output.contains("\"code\":\"NX-E0102\""), "{output}");
    assert!(output.contains("\"start\":16,\"end\":20"), "{output}");
}

#[test]
fn f5_numeric_operator_domains_match_both_adr_operand_orders() {
    // F5 §9.2: the arithmetic family has no source expected_type relation;
    // comparison and equality retain the valid operand as that relation.
    for (op, right_start) in [
        ("-", 20),
        ("*", 20),
        ("/", 20),
        ("%", 20),
        ("<", 20),
        ("<=", 21),
        (">", 20),
        (">=", 21),
        ("==", 21),
        ("!=", 21),
    ] {
        let comparison = matches!(op, "<" | "<=" | ">" | ">=" | "==" | "!=");
        for (source, bad_start, expected, found, related_expected) in [
            (
                format!("fn Main(){{let x=1 {op} true;}}"),
                right_start,
                "I64",
                "Bool",
                comparison.then_some((16, 17)),
            ),
            (
                format!("fn Main(){{let x=true {op} 1;}}"),
                if comparison { 22 + op.len() } else { 16 },
                if comparison { "Bool" } else { "I64" },
                if comparison { "I64" } else { "Bool" },
                comparison.then_some((16, 20)),
            ),
        ] {
            let bad_end = bad_start + if found == "Bool" { 4 } else { 1 };
            let expected_relation = related_expected.map_or(String::new(), |(start, end)| {
                format!(r#"{{"role":"expected_type","span":{{"file":"case.n","start":{start},"end":{end}}}}},"#)
            });
            let diagnostic = format!(
                r#"{{"schema_version":1,"diagnostic_namespace":"Nexo-proposed-v1","profile_id":"f0-0005-b-v1","code":"NX-E0102","subcause":"type_mismatch","rule":"NX-E0102/type_mismatch","explanation_id":"NX-E0102/type_mismatch","severity":"error","stage":"types","message":"Invalid name, call or type in the proposed profile.","operation":"types.check","primary_span":{{"file":"case.n","start":{bad_start},"end":{bad_end}}},"related_spans":[{expected_relation}{{"role":"found_type","span":{{"file":"case.n","start":{bad_start},"end":{bad_end}}}}}],"cause_chain":[],"feature_id":null,"unexpected_token":null,"expected_tokens":[],"expected_tokens_kind":null,"parser_context":null,"recognized_prefix":null,"repair":[],"data":{{"expected_type":"{expected}","found_type":"{found}"}}}}"#
            );
            for verb in ["check", "run"] {
                let (status, output) = invoke(verb, &source, "f0-0005-b-v1");
                assert_eq!(status, 1, "{verb}: {source}: {output}");
                assert_eq!(output, rejected(&diagnostic), "{verb}: {source}");
            }
        }
        let repaired = format!("fn Main(){{let x=1 {op} 2;}}");
        let (status, output) = invoke("check", &repaired, "f0-0005-b-v1");
        if matches!(op, "-" | "<" | "==") {
            assert_eq!(status, 0, "{repaired}: {output}");
            assert!(
                output.contains("\"diagnostics\":[]"),
                "{repaired}: {output}"
            );
        } else {
            assert_eq!(status, 1, "{repaired}: {output}");
            assert!(
                output.contains("\"pending_scope\":\"L08\""),
                "{repaired}: {output}"
            );
        }
    }
}

#[test]
fn f5_unary_negation_rejects_bool_and_out_of_range_literal_at_adr_spans() {
    let cases = [
        (
            "fn Main(){let x=-true;}",
            r#"{"schema_version":1,"diagnostic_namespace":"Nexo-proposed-v1","profile_id":"f0-0005-b-v1","code":"NX-E0102","subcause":"type_mismatch","rule":"NX-E0102/type_mismatch","explanation_id":"NX-E0102/type_mismatch","severity":"error","stage":"types","message":"Invalid name, call or type in the proposed profile.","operation":"types.check","primary_span":{"file":"case.n","start":17,"end":21},"related_spans":[{"role":"found_type","span":{"file":"case.n","start":17,"end":21}}],"cause_chain":[],"feature_id":null,"unexpected_token":null,"expected_tokens":[],"expected_tokens_kind":null,"parser_context":null,"recognized_prefix":null,"repair":[],"data":{"expected_type":"I64","found_type":"Bool"}}"#,
        ),
        (
            "fn Main(){let x=-(9223372036854775808);}",
            r#"{"schema_version":1,"diagnostic_namespace":"Nexo-proposed-v1","profile_id":"f0-0005-b-v1","code":"NX-E0107","subcause":null,"rule":"NX-E0107","explanation_id":"NX-E0107","severity":"error","stage":"types","message":"Invalid name, call or type in the proposed profile.","operation":"value.literal","primary_span":{"file":"case.n","start":18,"end":37},"related_spans":[],"cause_chain":[],"feature_id":null,"unexpected_token":null,"expected_tokens":[],"expected_tokens_kind":null,"parser_context":null,"recognized_prefix":null,"repair":[],"data":{}}"#,
        ),
    ];
    for (source, diagnostic) in cases {
        for verb in ["check", "run"] {
            let (status, output) = invoke(verb, source, "f0-0005-b-v1");
            assert_eq!(status, 1, "{verb} {source}: {output}");
            assert_eq!(output, rejected(diagnostic), "{verb}: {source}");
        }
    }
    for source in [
        "fn Main(){let x=-(1);}",
        "fn Main(){let x=-(9223372036854775807);}",
    ] {
        let (status, output) = invoke("check", source, "f0-0005-b-v1");
        assert_eq!(status, 1, "{source}: {output}");
        assert!(
            output.contains("\"pending_scope\":\"L08\""),
            "{source}: {output}"
        );
    }
}

#[test]
fn f5_const_syntax_keeps_parse_boundaries_before_l09() {
    let cases = [
        (
            "const",
            r#"{"schema_version":1,"diagnostic_namespace":"Nexo-proposed-v1","profile_id":"f0-0005-b-v1","code":"NX-E0002","subcause":null,"rule":"NX-E0002","explanation_id":"NX-E0002","severity":"error","stage":"parse","message":"Syntax is not admitted by the grammar.","operation":"syntax.parse","primary_span":{"file":"case.n","start":5,"end":5},"related_spans":[],"cause_chain":[],"feature_id":null,"unexpected_token":{"kind":"eof","text":"","span":{"file":"case.n","start":5,"end":5}},"expected_tokens":["Name"],"expected_tokens_kind":"admissible_set","parser_context":"grammar.ConstDecl","recognized_prefix":{"file":"case.n","start":0,"end":5},"repair":[],"data":{}}"#,
        ),
        (
            "fn Main(){const k:I64=comptime 1;}",
            r#"{"schema_version":1,"diagnostic_namespace":"Nexo-proposed-v1","profile_id":"f0-0005-b-v1","code":"NX-E0006","subcause":null,"rule":"NX-E0006","explanation_id":"NX-E0006","severity":"error","stage":"parse","message":"Recognized construct is outside the profile.","operation":"syntax.parse","primary_span":{"file":"case.n","start":10,"end":15},"related_spans":[],"cause_chain":[],"feature_id":"comptime","unexpected_token":null,"expected_tokens":[],"expected_tokens_kind":null,"parser_context":null,"recognized_prefix":null,"repair":[],"data":{"pending_scope":"later_adr"}}"#,
        ),
        (
            "pub const k:I64=comptime 1;fn Main(){}",
            r#"{"schema_version":1,"diagnostic_namespace":"Nexo-proposed-v1","profile_id":"f0-0005-b-v1","code":"NX-E0006","subcause":null,"rule":"NX-E0006","explanation_id":"NX-E0006","severity":"error","stage":"parse","message":"Recognized construct is outside the profile.","operation":"syntax.parse","primary_span":{"file":"case.n","start":0,"end":3},"related_spans":[],"cause_chain":[],"feature_id":"modules_visibility","unexpected_token":null,"expected_tokens":[],"expected_tokens_kind":null,"parser_context":null,"recognized_prefix":null,"repair":[],"data":{"pending_scope":"later_adr"}}"#,
        ),
    ];
    for (source, diagnostic) in cases {
        for verb in ["check", "run"] {
            let (status, output) = invoke(verb, source, "f0-0005-b-v1");
            assert_eq!(status, 1, "{verb} {source}: {output}");
            assert_eq!(output, rejected(diagnostic), "{verb}: {source}");
        }
    }
    let source = "const k:I64=comptime y;fn Main(){}";
    for verb in ["check", "run"] {
        let (status, output) = invoke(verb, source, "f0-0005-b-v1");
        assert_eq!(status, 1, "{verb}: {output}");
        assert!(output.contains("\"pending_scope\":\"L09\""), "{output}");
        assert!(!output.contains("NX-E0101"), "{output}");
    }
    for verb in ["check", "run"] {
        let (status, output) = invoke(verb, "fn Main(){let x=1+2;}", "f0-0005-b-v1");
        assert_eq!(status, 0, "{verb}: {output}");
        assert!(output.contains("\"diagnostics\":[]"), "{output}");
    }
}

#[test]
fn f5_sealed_prelude_identity_respects_shadowing_and_argument_stage() {
    for verb in ["check", "run"] {
        let (status, output) = invoke(
            verb,
            "fn Push(x:view I64){}fn Main(){Push(1);}",
            "f0-0005-b-v1",
        );
        assert_eq!(status, 0, "{verb}: {output}");
        assert!(output.contains("\"diagnostics\":[]"), "{output}");
    }
    for (source, subcause, start, end) in [
        (
            "fn Main(){let Push=1;Push(2);}",
            "non_callable_target",
            21,
            25,
        ),
        (
            "fn Main(){var a=[1];Push(a,2);}",
            "argument_mode_mismatch",
            25,
            26,
        ),
        (
            "fn Main(){var a=[1];Push(edit a);}",
            "missing_argument",
            20,
            24,
        ),
    ] {
        for verb in ["check", "run"] {
            let (status, output) = invoke(verb, source, "f0-0005-b-v1");
            assert_eq!(status, 1, "{verb} {source}: {output}");
            assert!(output.contains("\"code\":\"NX-E0111\""), "{output}");
            assert!(
                output.contains(&format!("\"subcause\":\"{subcause}\"")),
                "{output}"
            );
            assert!(output.contains("\"stage\":\"arguments\""), "{output}");
            assert!(
                output.contains(&format!(
                    "\"primary_span\":{{\"file\":\"case.n\",\"start\":{start},\"end\":{end}}}"
                )),
                "{output}"
            );
            assert!(!output.contains("\"pending_scope\":\"L07\""), "{output}");
        }
    }
}

#[test]
fn f5_first_sets_keep_block_and_negation_boundaries() {
    let cases = [
        (
            "fn Main(){",
            r#"{"schema_version":1,"diagnostic_namespace":"Nexo-proposed-v1","profile_id":"f0-0005-b-v1","code":"NX-E0002","subcause":null,"rule":"NX-E0002","explanation_id":"NX-E0002","severity":"error","stage":"parse","message":"Syntax is not admitted by the grammar.","operation":"syntax.parse","primary_span":{"file":"case.n","start":10,"end":10},"related_spans":[],"cause_chain":[],"feature_id":null,"unexpected_token":{"kind":"eof","text":"","span":{"file":"case.n","start":10,"end":10}},"expected_tokens":["(","-","IntLiteral","Name","[","break","continue","edit","false","fn","for","if","let","match","move","not","return","true","var","while","yield","{","}"],"expected_tokens_kind":"admissible_set","parser_context":"block_item","recognized_prefix":{"file":"case.n","start":9,"end":10},"repair":[],"data":{}}"#,
        ),
        (
            "fn Main(){let a=-",
            r#"{"schema_version":1,"diagnostic_namespace":"Nexo-proposed-v1","profile_id":"f0-0005-b-v1","code":"NX-E0002","subcause":null,"rule":"NX-E0002","explanation_id":"NX-E0002","severity":"error","stage":"parse","message":"Syntax is not admitted by the grammar.","operation":"syntax.parse","primary_span":{"file":"case.n","start":17,"end":17},"related_spans":[],"cause_chain":[],"feature_id":null,"unexpected_token":{"kind":"eof","text":"","span":{"file":"case.n","start":17,"end":17}},"expected_tokens":["(","-","IntLiteral","Name","[","edit","false","fn","move","not","true"],"expected_tokens_kind":"admissible_set","parser_context":"grammar.NegExpr","recognized_prefix":{"file":"case.n","start":16,"end":17},"repair":[],"data":{}}"#,
        ),
    ];
    for (source, diagnostic) in cases {
        for verb in ["check", "run"] {
            let (status, output) = invoke(verb, source, "f0-0005-b-v1");
            assert_eq!(status, 1, "{verb} {source}: {output}");
            assert_eq!(output, rejected(diagnostic), "{verb}: {source}");
        }
    }
}

#[test]
fn f5_logical_operators_check_both_bool_domains_before_lower() {
    for (source, start, end) in [
        ("fn Main(){let x=not 1;}", 20, 21),
        ("fn Main(){let x=true and 1;}", 25, 26),
        ("fn Main(){let x=1 and false;}", 16, 17),
        ("fn Main(){let x=1 or false;}", 16, 17),
    ] {
        let diagnostic = format!(
            r#"{{"schema_version":1,"diagnostic_namespace":"Nexo-proposed-v1","profile_id":"f0-0005-b-v1","code":"NX-E0102","subcause":"type_mismatch","rule":"NX-E0102/type_mismatch","explanation_id":"NX-E0102/type_mismatch","severity":"error","stage":"types","message":"Invalid name, call or type in the proposed profile.","operation":"types.check","primary_span":{{"file":"case.n","start":{start},"end":{end}}},"related_spans":[{{"role":"found_type","span":{{"file":"case.n","start":{start},"end":{end}}}}}],"cause_chain":[],"feature_id":null,"unexpected_token":null,"expected_tokens":[],"expected_tokens_kind":null,"parser_context":null,"recognized_prefix":null,"repair":[],"data":{{"expected_type":"Bool","found_type":"I64"}}}}"#
        );
        for verb in ["check", "run"] {
            let (status, output) = invoke(verb, source, "f0-0005-b-v1");
            assert_eq!(status, 1, "{verb} {source}: {output}");
            assert_eq!(output, rejected(&diagnostic), "{verb}: {source}");
        }
    }
    for source in [
        "fn Main(){let x=not true;}",
        "fn Main(){let x=false and true;}",
        "fn Main(){let x=true or false;}",
    ] {
        for verb in ["check", "run"] {
            let (status, output) = invoke(verb, source, "f0-0005-b-v1");
            assert_eq!(status, 1, "{verb} {source}: {output}");
            assert!(output.contains("\"stage\":\"lower\""), "{source}: {output}");
            assert!(
                output.contains("\"pending_scope\":\"L08\""),
                "{source}: {output}"
            );
        }
    }
}

#[test]
fn f5_incomplete_not_uses_neg_expr_first_set() {
    let source = "fn Main(){let a=not";
    let diagnostic = r#"{"schema_version":1,"diagnostic_namespace":"Nexo-proposed-v1","profile_id":"f0-0005-b-v1","code":"NX-E0002","subcause":null,"rule":"NX-E0002","explanation_id":"NX-E0002","severity":"error","stage":"parse","message":"Syntax is not admitted by the grammar.","operation":"syntax.parse","primary_span":{"file":"case.n","start":19,"end":19},"related_spans":[],"cause_chain":[],"feature_id":null,"unexpected_token":{"kind":"eof","text":"","span":{"file":"case.n","start":19,"end":19}},"expected_tokens":["(","-","IntLiteral","Name","[","edit","false","fn","move","not","true"],"expected_tokens_kind":"admissible_set","parser_context":"grammar.NegExpr","recognized_prefix":{"file":"case.n","start":16,"end":19},"repair":[],"data":{}}"#;
    for verb in ["check", "run"] {
        let (status, output) = invoke(verb, source, "f0-0005-b-v1");
        assert_eq!(status, 1, "{verb}: {output}");
        assert_eq!(output, rejected(diagnostic), "{verb}");
    }
}

#[test]
fn f5_short_circuit_still_checks_static_rhs_effects() {
    for operator in ["false and", "true or"] {
        let source = format!(
            "fn Dirty()->Bool{{Print(9);return true;}}fn Main() with no_io{{let x={operator} Dirty();}}"
        );
        for verb in ["check", "run"] {
            let (status, output) = invoke(verb, &source, "f0-0005-b-v1");
            assert_eq!(status, 1, "{verb} {source}: {output}");
            assert!(output.contains("\"code\":\"NX-E0415\""), "{verb}: {output}");
            assert!(output.contains("\"stage\":\"effects\""), "{verb}: {output}");
        }
    }
}
