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
fn f5_ranges_outside_direct_for_report_adr_boundary() {
    // F5-H03, F5-D01/D02 and F5 §3.1: an ordinary range has no Bool type.
    let cases = [
        ("fn Main(){let r=0..<3;}", 17, 20, "fn Main(){let r=0+3;}"),
        ("fn Main(){let r=0..=3;}", 17, 20, "fn Main(){let r=0+3;}"),
        (
            "fn Main(){let r=(0..<3);}",
            18,
            21,
            "fn Main(){let r=(0+3);}",
        ),
        ("fn Main(){if 0..<3{}}", 14, 17, "fn Main(){if true{}}"),
        (
            "fn Main(){var a=[1];let b=a[0..<1];}",
            27,
            34,
            "fn Main(){var a=[1];let b=a[0];}",
        ),
        (
            "fn F()->I64{return 0..<3;}fn Main(){}",
            20,
            23,
            "fn F()->I64{return 0+3;}fn Main(){}",
        ),
    ];
    for (source, start, end, positive) in cases {
        let diagnostic = format!(
            r#"{{"schema_version":1,"diagnostic_namespace":"Nexo-proposed-v1","profile_id":"f0-0005-b-v1","code":"NX-E0006","subcause":null,"rule":"NX-E0006","explanation_id":"NX-E0006","severity":"error","stage":"types","message":"Recognized construct is outside the profile.","operation":"types.check","primary_span":{{"file":"case.n","start":{start},"end":{end}}},"related_spans":[],"cause_chain":[],"feature_id":"range","unexpected_token":null,"expected_tokens":[],"expected_tokens_kind":null,"parser_context":null,"recognized_prefix":null,"repair":[],"data":{{"pending_scope":"later_adr"}}}}"#
        );
        for verb in ["check", "run"] {
            let (status, output) = invoke(verb, source, "f0-0005-b-v1");
            assert_eq!(status, 1, "{verb} {source}: {output}");
            assert_eq!(output, rejected(&diagnostic), "{verb} {source}");
            let (status, output) = invoke(verb, positive, "f0-0005-b-v1");
            assert_eq!(status, 0, "{verb} {positive}: {output}");
        }
    }
}

#[test]
fn f5_field_access_uses_j_names_boundary() {
    // F5 §8.2 retains J's methods/later_adr rule.
    let source = "fn Main(){var x=1;x.f=1;}";
    let diagnostic = r#"{"schema_version":1,"diagnostic_namespace":"Nexo-proposed-v1","profile_id":"f0-0005-b-v1","code":"NX-E0006","subcause":null,"rule":"NX-E0006","explanation_id":"NX-E0006","severity":"error","stage":"names","message":"Recognized construct is outside the profile.","operation":"names.resolve","primary_span":{"file":"case.n","start":19,"end":20},"related_spans":[],"cause_chain":[],"feature_id":"methods","unexpected_token":null,"expected_tokens":[],"expected_tokens_kind":null,"parser_context":null,"recognized_prefix":null,"repair":[],"data":{"pending_scope":"later_adr"}}"#;
    for verb in ["check", "run"] {
        let (status, output) = invoke(verb, source, "f0-0005-b-v1");
        assert_eq!(status, 1, "{verb}: {output}");
        assert_eq!(output, rejected(diagnostic), "{verb}");
        let (status, output) = invoke(verb, "fn Main(){var x=1;x=1;}", "f0-0005-b-v1");
        assert_eq!(status, 0, "{verb}: {output}");
    }
}

#[test]
fn f5_both_direct_for_ranges_keep_l05_at_for() {
    // The ..= alternative is essential: D's M04 used to survive the suite.
    let diagnostic = r#"{"schema_version":1,"diagnostic_namespace":"Nexo-proposed-v1","profile_id":"f0-0005-b-v1","code":"NX-E0006","subcause":null,"rule":"NX-E0006","explanation_id":"NX-E0006","severity":"error","stage":"types","message":"Capacidad pendiente de implementación en L05.","operation":"types.check","primary_span":{"file":"case.n","start":10,"end":13},"related_spans":[],"cause_chain":[],"feature_id":"implementation_limit","unexpected_token":null,"expected_tokens":[],"expected_tokens_kind":null,"parser_context":null,"recognized_prefix":null,"repair":[],"data":{"pending_scope":"L05"}}"#;
    for source in [
        "fn Main(){for i in 0..<2{}}",
        "fn Main(){for i in 0..=2{}}",
        "fn Main(){for i in (0..<0){}}",
        "fn Main(){for i in (0..=0){}}",
        "fn Main(){for i in 3..<2{}}",
        "fn Main(){for i in 3..=2{}}",
    ] {
        for verb in ["check", "run"] {
            let (status, output) = invoke(verb, source, "f0-0005-b-v1");
            assert_eq!(status, 1, "{verb} {source}: {output}");
            assert_eq!(output, rejected(diagnostic), "{verb} {source}");
        }
    }
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

#[test]
fn f5_compound_assignment_types_and_permission_precede_l08() {
    for (source, code, start, end) in [
        ("fn Main(){let x=1;x+=2;}", "NX-E0105", 18, 19),
        ("fn Main(){var x=1;x+=true;}", "NX-E0102", 21, 25),
        ("fn Main(){var x=true;x+=1;}", "NX-E0102", 21, 22),
    ] {
        let diagnostic = if code == "NX-E0105" {
            r#"{"schema_version":1,"diagnostic_namespace":"Nexo-proposed-v1","profile_id":"f0-0005-b-v1","code":"NX-E0105","subcause":null,"rule":"NX-E0105","explanation_id":"NX-E0105","severity":"error","stage":"ownership","message":"La raíz no admite escritura.","operation":"place.write","primary_span":{"file":"case.n","start":18,"end":19},"related_spans":[{"role":"declaration","span":{"file":"case.n","start":14,"end":15}}],"cause_chain":[],"feature_id":null,"unexpected_token":null,"expected_tokens":[],"expected_tokens_kind":null,"parser_context":null,"recognized_prefix":null,"repair":[],"data":{}}"#.to_owned()
        } else {
            format!(
                r#"{{"schema_version":1,"diagnostic_namespace":"Nexo-proposed-v1","profile_id":"f0-0005-b-v1","code":"NX-E0102","subcause":"type_mismatch","rule":"NX-E0102/type_mismatch","explanation_id":"NX-E0102/type_mismatch","severity":"error","stage":"types","message":"Invalid name, call or type in the proposed profile.","operation":"types.check","primary_span":{{"file":"case.n","start":{start},"end":{end}}},"related_spans":[{{"role":"found_type","span":{{"file":"case.n","start":{start},"end":{end}}}}}],"cause_chain":[],"feature_id":null,"unexpected_token":null,"expected_tokens":[],"expected_tokens_kind":null,"parser_context":null,"recognized_prefix":null,"repair":[],"data":{{"expected_type":"I64","found_type":"Bool"}}}}"#
            )
        };
        for verb in ["check", "run"] {
            let (status, output) = invoke(verb, source, "f0-0005-b-v1");
            assert_eq!(status, 1, "{verb} {source}: {output}");
            assert_eq!(output, rejected(&diagnostic), "{verb} {source}");
        }
    }
    for verb in ["check", "run"] {
        let (status, output) = invoke(verb, "fn Main(){var x=1;x+=2;}", "f0-0005-b-v1");
        assert_eq!(status, 1, "{verb}: {output}");
        assert!(output.contains("\"stage\":\"lower\""), "{output}");
        assert!(output.contains("\"pending_scope\":\"L08\""), "{output}");
    }
    for symbol in ["-=", "*=", "/=", "%="] {
        let bad_rhs = format!("fn Main(){{var x=1;x{symbol}true;}}");
        let bad_place = format!("fn Main(){{var x=true;x{symbol}1;}}");
        for (source, start, end) in [(&bad_rhs, 21, 25), (&bad_place, 21, 22)] {
            let (status, output) = invoke("check", source, "f0-0005-b-v1");
            assert_eq!(status, 1, "{source}: {output}");
            assert!(output.contains("\"code\":\"NX-E0102\""), "{output}");
            assert!(
                output.contains(&format!("\"start\":{start},\"end\":{end}")),
                "{output}"
            );
        }
    }
}

#[test]
fn f5_compound_assignment_holds_exclusive_loan_through_rhs() {
    for (source, start, end, declaration_end, origin_start, origin_end) in [
        ("fn Main(){var x=1;x+=x;}", 21, 22, 15, 18, 19),
        ("fn Main(){var a=[1];a[0]+=a[0];}", 26, 27, 15, 20, 24),
        (
            "fn Main(){var a=[1,2];let i=1;a[i]+=a[i-1];}",
            36,
            37,
            15,
            30,
            34,
        ),
        ("fn Main(){var sum=4;sum+=sum/2;}", 25, 28, 17, 20, 23),
    ] {
        let diagnostic = format!(
            r#"{{"schema_version":1,"diagnostic_namespace":"Nexo-proposed-v1","profile_id":"f0-0005-b-v1","code":"NX-E1105","subcause":"active_borrow_conflict","rule":"NX-E1105/active_borrow_conflict","explanation_id":"NX-E1105/active_borrow_conflict","severity":"error","stage":"ownership","message":"active_borrow_conflict: acceso incompatible con préstamo vivo; permission_escalation: derivar permiso más fuerte del recibido; overlapping_mutable_borrows: crear segundo préstamo mutable solapado.","operation":"ownership.access","primary_span":{{"file":"case.n","start":{start},"end":{end}}},"related_spans":[{{"role":"declaration","span":{{"file":"case.n","start":14,"end":{declaration_end}}}}},{{"role":"origin","span":{{"file":"case.n","start":{origin_start},"end":{origin_end}}}}}],"cause_chain":[{{"kind":"borrow","operation":"borrow.inout","span":{{"file":"case.n","start":{origin_start},"end":{origin_end}}},"entity":{{"kind":"loan","id":"loan:{origin_start}:0"}},"evidence":"verified"}}],"feature_id":null,"unexpected_token":null,"expected_tokens":[],"expected_tokens_kind":null,"parser_context":null,"recognized_prefix":null,"repair":[],"data":{{"active_permission":"inout","binding":{{"id":"binding:14","kind":"binding"}},"loan":{{"id":"loan:{origin_start}:0","kind":"loan"}},"origin":{{"id":"binding:14","kind":"binding"}},"requested_access":"read"}}}}"#
        );
        for verb in ["check", "run"] {
            let (status, output) = invoke(verb, source, "f0-0005-b-v1");
            assert_eq!(status, 1, "{verb} {source}: {output}");
            assert_eq!(output, rejected(&diagnostic), "{verb} {source}");
        }
    }
    for source in [
        "fn Main(){var x=1;let rhs=x;x+=rhs;}",
        "fn Main(){var a=[1,2];let i=1;let rhs=a[i-1];a[i]+=rhs;}",
        "fn Main(){var sum=4;let rhs=sum/2;sum+=rhs;}",
    ] {
        for verb in ["check", "run"] {
            let (status, output) = invoke(verb, source, "f0-0005-b-v1");
            assert_eq!(status, 1, "{verb} {source}: {output}");
            assert!(output.contains("\"pending_scope\":\"L08\""), "{output}");
        }
    }
}

#[test]
fn f5_effect_contracts_publish_the_profile_model() {
    // F5 §7.1/F5-B05 retain J's effect rule, operation and source site.
    let cases = [
        ("fn Main(){let a=[1];let f=fn [copy a]() with {create(no_alloc),call(unrestricted)}{};}", "NX-E0412", "closure.create", 26, 28, "alloc"),
        ("fn Main(){let a=[1];let f=fn [move a]() with {copy(no_alloc),call(unrestricted)}{};}", "NX-E0412", "value.copy", 26, 28, "alloc"),
        ("fn Main(){let r=MockBlockingOpen(1);let f=fn [move r]() with {drop(no_block),call(unrestricted)}{};}", "NX-E0413", "value.drop", 42, 44, "block"),
        ("fn Main(){let r=MockOpen(1);let f=fn [move r]() with {drop(no_io),call(unrestricted)}{};}", "NX-E0415", "value.drop", 34, 36, "io"),
        ("gen fn G()->I64{yield 1;}fn Main(){let g=G();let f=fn [move g]() with no_suspend{var h=move g;let n=Iterator.Next(edit h);};}", "NX-E0414", "generator.next", 109, 113, "suspend"),
    ];
    for (source, code, operation, start, end, effect) in cases {
        for verb in ["check", "run"] {
            let (status, output) = invoke(verb, source, "f0-0005-b-v1");
            assert_eq!(status, 1, "{verb} {source}: {output}");
            assert_eq!(output.matches("\"diagnostic_namespace\"").count(), 1, "{output}");
            assert!(output.contains(&format!("\"code\":\"{code}\"")), "{output}");
            assert!(output.contains(&format!("\"operation\":\"{operation}\"")), "{output}");
            assert!(output.contains(&format!("\"primary_span\":{{\"file\":\"case.n\",\"start\":{start},\"end\":{end}}}")), "{output}");
            assert!(output.contains("\"effect_model\":\"f0-0005-sim-v1\""), "{output}");
            assert!(output.contains("\"effect_evidence\":\"known_effect\""), "{output}");
            assert!(output.contains(&format!("\"required_effect\":\"{effect}\"")), "{output}");
            let (j_status, j_output) = invoke(verb, source, "f0-0003-b-v1");
            assert_eq!(j_status, 1, "{j_output}");
            assert_eq!(
                output,
                j_output
                    .replace("f0-0003-b-v1", "f0-0005-b-v1")
                    .replace("f0-0003-sim-v1", "f0-0005-sim-v1"),
                "{verb} {source}"
            );
        }
    }
}

#[test]
fn f5_match_guard_effect_uses_the_same_model() {
    let source = "fn Main(){match true{true if MockUnknown()==()=>{},_=>{}}}";
    for verb in ["check", "run"] {
        let (status, output) = invoke(verb, source, "f0-0005-b-v1");
        assert_eq!(status, 1, "{output}");
        assert_eq!(output.matches("\"diagnostic_namespace\"").count(), 1, "{output}");
        assert!(output.contains("\"code\":\"NX-E1305\""), "{output}");
        assert!(output.contains("\"effect_model\":\"f0-0005-sim-v1\""), "{output}");
    }
}
