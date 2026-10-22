use super::model::{self, BlockId, Op, TempId, Terminator};
use super::{CompileError, compile_scalar};

#[test]
fn divergent_condition_operands_do_not_create_unreachable_cfg_blocks() {
    for source in [
        "fn main(){if ((if true{return;}else{return;})==0){print(1);}else{print(2);}}",
        "fn main(){while ((if true{return;}else{return;})<1){print(1);}}",
        "fn main(){var v1=if (1==(if true{return;}else{return;})){value:5;}else{value:6;};print(v1+1);}",
        "fn main(){if ((if true{return;}else{return;})==0){print(1);}}",
        "fn main(){while true{if ((if true{return;}else{return;})==0){print(1);}else{print(2);}}}",
        "fn id(x:i64)->bool{return x<1;} fn main(){if id(if true{return;}else{return;}){print(1);}else{print(2);}}",
        "fn id(x:i64)->bool{return x<1;} fn main(){while id(if true{return;}else{return;}){print(1);}}",
    ] {
        let program = compile_scalar("divergent_condition_operand.nx", source)
            .unwrap_or_else(|error| panic!("{source}: {error:?}"));
        assert!(
            crate::interp::run_scalar(&program)
                .observations
                .stdout()
                .is_empty()
        );
    }
}

#[test]
fn hypothetical_bottom_values_in_both_arms_need_no_ir_slot() {
    for source in [
        "fn main(){let v=if true{return;}else{return;};let w=(if true{value:v;}else{value:v;});}",
        "fn main(){let v=if true{return;}else{return;};let w=(if false{value:v+1;}else{value:v+2;});}",
    ] {
        let program = compile_scalar("bottom_branch_arms.nx", source).unwrap();
        assert!(
            crate::interp::run_scalar(&program)
                .observations
                .stdout()
                .is_empty()
        );
    }
}

#[test]
fn nested_divergent_left_keeps_missing_branch_value() {
    let source =
        "fn main(){print((1+(if true{return;}else{return;}))+(if true{print(2);}else{value:1;}));}";
    let Err(CompileError::Rejected(errors)) = compile_scalar("nested_branch_value.nx", source)
    else {
        panic!("expected missing branch value");
    };
    assert!(
        errors
            .iter()
            .any(|d| d.code == "NX-E1302" && d.subcause.as_deref() == Some("missing_branch_value"))
    );
}

#[test]
fn deferred_binding_reads_preserve_bottom_type_across_uses() {
    let diverges = "if true{return;}else{return;}";
    let sources = [
        format!("fn main(){{let v={diverges};print(v==2);}}"),
        format!("fn main(){{let v={diverges};print(2==v);}}"),
        format!("fn main(){{let v={diverges};let w=v;print(w+1);}}"),
        format!("fn main(){{let v={diverges};print(if true{{value:v;}}else{{value:1;}});}}"),
        format!("fn main(){{let v={diverges};let w=[v,2];}}"),
        format!("fn main(){{let w:Array<i64>=[{diverges},2];}}"),
        format!("fn main(){{let w=({diverges})+[{diverges}];}}"),
        format!("fn main(){{var x={diverges};x=2;}}"),
        format!("fn main(){{let v=1+({diverges});print(v==1);}}"),
        format!("fn g(x:i64)->i64{{return x;}} fn main(){{let v=g({diverges});print(v==1);}}"),
        format!("fn main(){{var v1={diverges};var v5=v1;var v6=v5+v1;}}"),
        format!("fn main(){{let v={diverges};let a=[1];print(a[v]);}}"),
        format!("fn g(x:i64){{}} fn main(){{let v={diverges};g(v);}}"),
        format!("fn main(){{let v={diverges};if v{{print(1);}}}}"),
        format!("fn main(){{let v={diverges};var w=1;w=v;}}"),
        format!("fn g(x:i64){{}} fn main(){{let v={diverges};let a=[1];g(a[v]);}}"),
        "fn f()->i64{let v=if true{return 1;}else{return 2;};return v;} fn main(){f();}".into(),
    ];
    for source in sources {
        let program = compile_scalar("deferred_binding_uses.nx", &source)
            .unwrap_or_else(|error| panic!("{source}: {error:?}"));
        assert!(
            crate::interp::run_scalar(&program)
                .observations
                .stdout()
                .is_empty()
        );
    }
}

#[test]
fn divergent_index_of_borrowed_argument_is_checked_without_ir_operand() {
    let source = "fn g(x:i64){} fn main(){let a=[1];g(a[if true{return;}else{return;}]);}";
    let program = compile_scalar("loan_divergent_index.nx", source).unwrap();
    assert!(
        crate::interp::run_scalar(&program)
            .observations
            .stdout()
            .is_empty()
    );
}

#[test]
fn intrinsic_binary_type_is_independent_of_divergent_operand_position() {
    let diverges = "if true{return;}else{return;}";
    for (op, wrong_type) in [("+", "bool"), ("-", "bool"), ("<", "i64")] {
        for expression in [format!("({diverges}){op}1"), format!("1{op}({diverges})")] {
            let source = format!("fn main(){{let v={expression};let w:{wrong_type}=v;}}");
            let result = compile_scalar("binary_result_type.nx", &source);
            assert!(
                matches!(result, Err(CompileError::Rejected(ref errors)) if errors.iter().any(|d| d.code == "NX-E0102" && d.subcause.as_deref() == Some("type_mismatch"))),
                "{source}: {result:?}"
            );
        }
    }
    let source = format!("fn main(){{let v=({diverges})+({diverges});let w:bool=v;}}");
    let result = compile_scalar("binary_result_type.nx", &source);
    assert!(
        matches!(result, Err(CompileError::Rejected(ref errors)) if errors.iter().any(|d| d.code == "NX-E0102" && d.subcause.as_deref() == Some("type_mismatch"))),
        "{source}: {result:?}"
    );
}

#[test]
fn left_divergent_binary_keeps_result_type_in_lowered_local() {
    for (op, expected) in [
        ("+", model::Ty::I64),
        ("-", model::Ty::I64),
        ("<", model::Ty::Bool),
    ] {
        let source = format!("fn main(){{let v=(if true{{return;}}else{{return;}}){op}1;}}");
        let program = compile_scalar("left_binary_type.nx", &source).unwrap();
        assert_eq!(
            program.functions.function("main").unwrap().locals[0].ty,
            expected,
            "{source}"
        );
    }
}

#[test]
fn direct_call_keeps_declared_result_type_after_divergent_argument() {
    let source = "fn g(x:i64)->i64{return x;} fn main(){let v=g(if true{return;}else{return;});}";
    let program = compile_scalar("call_result_type.nx", source).unwrap();
    assert_eq!(
        program.functions.function("main").unwrap().locals[0].ty,
        model::Ty::I64
    );
}

#[test]
fn array_of_deferred_reads_infers_concrete_siblings_or_remains_bottom() {
    let source =
        "fn main(){let v=if true{return;}else{return;};let w=(if true{return;}else{return;})+[v];}";
    compile_scalar("bottom_array_operand.nx", source).unwrap();
    let source = "fn main(){let v=if true{return;}else{return;};let w=[v,2];let z:Array<bool>=w;}";
    let result = compile_scalar("bottom_array_inference.nx", source);
    assert!(
        matches!(result, Err(CompileError::Rejected(ref errors)) if errors.iter().any(|d| d.code == "NX-E0102" && d.subcause.as_deref() == Some("type_mismatch"))),
        "{result:?}"
    );
}

#[test]
fn divergent_left_less_than_checks_numeric_right_operand() {
    for source in [
        "fn main(){let v=(if true{return;}else{return;})<true;}",
        "fn main(){let v=true<(if true{return;}else{return;});}",
    ] {
        let result = compile_scalar("less_numeric.nx", source);
        assert!(
            matches!(result, Err(CompileError::Rejected(ref errors)) if errors.iter().any(|d| d.code == "NX-E0102" && d.subcause.as_deref() == Some("type_mismatch"))),
            "{source}: {result:?}"
        );
    }
}

#[test]
fn dead_binary_placeholder_does_not_change_branch_flow() {
    let source = "fn main(){return;print(if false{value:4+3;}else{return;});}";
    let program = compile_scalar("dead_binary.nx", source).unwrap();
    assert!(
        crate::interp::run_scalar(&program)
            .observations
            .stdout()
            .is_empty()
    );

    let invalid = "fn main(){return;print(if false{value:1;}else{print(4-2);});}";
    let Err(CompileError::Rejected(errors)) = compile_scalar("dead_branch.nx", invalid) else {
        panic!("missing branch value must be rejected");
    };
    assert!(
        errors
            .iter()
            .any(|d| d.code == "NX-E1302" && d.subcause.as_deref() == Some("missing_branch_value"))
    );
}

#[test]
fn while_backedge_rechecks_complete_branching_condition() {
    let sources = [
        (
            "fn main(){var c=0;while (if c<1{value:true;}else{value:false;}){c=c+1;}print(c);}",
            "1",
        ),
        (
            "fn main(){var c=0;while (if c<1{value:(if c==0{value:true;}else{value:false;});}else{value:false;}){c=c+1;}print(c);}",
            "1",
        ),
        (
            "fn main(){var c=0;while (if false{return;}else{value:c<1;}){c=c+1;}print(c);}",
            "1",
        ),
    ];
    for (source, expected) in sources {
        let program = compile_scalar("branching_loop.nx", source).unwrap();
        assert_eq!(
            crate::interp::run_scalar(&program).observations.stdout(),
            [expected]
        );
    }
}

#[test]
fn annotated_divergent_bindings_and_binary_results_keep_declared_types() {
    let bad = [
        "fn main(){let v:i64=if true{return;}else{return;};if v{print(1);}}",
        "fn h(x:bool){} fn main(){let v:i64=if true{return;}else{return;};h(v);}",
        "fn main(){let v:i64=if true{return;}else{return;};let w:bool=v;}",
        "fn main(){let v:i64=if true{return;}else{return;};print(v==true);}",
    ];
    for source in bad {
        let result = compile_scalar("divergent_type.nx", source);
        assert!(
            matches!(result, Err(CompileError::Rejected(ref ds)) if ds.iter().any(|d| d.code == "NX-E0102" && d.subcause.as_deref() == Some("type_mismatch"))),
            "{source}: {result:?}"
        );
    }
}

#[test]
fn binary_result_type_survives_divergent_operand() {
    for source in [
        "fn main(){let v=1+(if true{return;}else{return;});let w:bool=v;}",
        "fn main(){let v=1<(if true{return;}else{return;});let w:i64=v;}",
    ] {
        let result = compile_scalar("divergent_binary_type.nx", source);
        assert!(
            matches!(result, Err(CompileError::Rejected(ref ds)) if ds.iter().any(|d| d.code == "NX-E0102" && d.subcause.as_deref() == Some("type_mismatch"))),
            "{source}: {result:?}"
        );
    }
}

#[test]
fn reading_deferred_binding_does_not_diverge_branch() {
    let source =
        "fn main(){let v=if true{return;}else{return;};print(if true{value:1;}else{print(v);});}";
    let result = compile_scalar("deferred_read.nx", source);
    assert!(
        matches!(result, Err(CompileError::Rejected(ref ds)) if ds.iter().any(|d| d.code == "NX-E1302" && d.subcause.as_deref() == Some("missing_branch_value"))),
        "{result:?}"
    );
}

#[test]
fn missing_branch_value_precedes_deferred_binding_type() {
    for source in [
        "fn main(){var v=if true{value:1+(if true{return;}else{return;});}else{};v=5;}",
        "fn main(){var v=if true{return;}else{return;};v=(if true{value:1;}else{print(2);});}",
    ] {
        let result = compile_scalar("precedence.nx", source);
        assert!(
            matches!(result, Err(CompileError::Rejected(ref ds)) if ds.iter().any(|d| d.code == "NX-E1302" && d.subcause.as_deref() == Some("missing_branch_value"))),
            "{source}: {result:?}"
        );
    }
}

#[test]
fn dead_declaration_in_returning_branch_does_not_break_later_value() {
    let source = "fn main(){print(if true{value:1;}else{return;let v=4;value:v;});}";
    let program = compile_scalar("dead_local.nx", source).unwrap();
    assert_eq!(
        crate::interp::run_scalar(&program).observations.stdout(),
        ["1"]
    );
}

#[test]
fn dead_value_call_after_return_does_not_mutate_emitted_tick() {
    let source = "fn g(x:i64,y:i64)->i64{return y;} fn main(){let v1=1;(if true{return;value:g(7,v1);}else{value:2;});}";
    compile_scalar("dead_call_tick.nx", source).unwrap();
}

#[test]
#[ignore = "explicit 256 KiB to 1 MiB verifier loop scale measurement"]
fn loop_backedges_scale_in_steps_allocation_and_time() {
    use std::time::Instant;
    let patterns = [
        "while false{}",
        "while false{while false{}}",
        "while (if false{value:true;}else{value:false;}){}",
    ];
    for (family, pattern) in patterns.iter().enumerate() {
        let mut measurements = Vec::new();
        for target in [256 * 1024, 1024 * 1024] {
            let mut source = String::from("fn main(){");
            while source.len() + pattern.len() < target {
                source.push_str(pattern);
            }
            source.push('}');
            crate::test_alloc::reset("cfg_scale_counter.nx");
            super::CFG_WORK.store(0, std::sync::atomic::Ordering::Relaxed);
            let start = Instant::now();
            let result = compile_scalar("cfg_scale_counter.nx", &source);
            result.unwrap_or_else(|e| panic!("family {family}: {e:?}"));
            let steps = super::CFG_WORK.load(std::sync::atomic::Ordering::Relaxed);
            let elapsed = start.elapsed().as_nanos();
            let traffic = crate::test_alloc::snapshot("cfg_scale_counter.nx");
            measurements.push((steps, traffic.0, traffic.1, elapsed));
        }
        let small = measurements[0];
        let large = measurements[1];
        for (metric, low, high, limit) in [
            ("steps", small.0, large.0, 4.6),
            ("bytes", small.1, large.1, 4.6),
            ("allocations", small.2, large.2, 4.6),
            ("clock", small.3 as u64, large.3 as u64, 8.0),
        ] {
            let ratio = high as f64 / low as f64;
            eprintln!("loop {family} {metric}: {low} -> {high}, ratio {ratio:.3}");
            assert!(ratio <= limit, "loop {family} {metric}: {ratio:.3}");
        }
    }
}

#[test]
fn repeated_while_preheaders_have_linear_verification_time() {
    use std::time::Instant;
    let mut elapsed = [0_u128; 2];
    for (at, target) in [32 * 1024, 128 * 1024].into_iter().enumerate() {
        let mut source = String::from("fn main(){");
        while source.len() + "while false{}".len() < target {
            source.push_str("while false{}");
        }
        source.push('}');
        let start = Instant::now();
        compile_scalar("loop_time.nx", &source).unwrap();
        elapsed[at] = start.elapsed().as_nanos();
    }
    assert!(elapsed[1] as f64 / (elapsed[0] as f64) < 8.0, "{elapsed:?}");
}

#[test]
fn adr_gap_deferred_binding_takes_first_concrete_assignment_type() {
    let prefix = "fn main(){var v=if true{return;}else{return;};v=5;";
    compile_scalar("deferred_assign.nx", &format!("{prefix}print(v);}}"))
        .expect("the hypothetical binding may receive its first concrete type");
    for tail in ["v=true;", "let w:bool=v;"] {
        let source = format!("{prefix}{tail}}}");
        let result = compile_scalar("deferred_assign.nx", &source);
        assert!(
            matches!(result, Err(CompileError::Rejected(ref ds)) if ds.iter().any(|d| d.code == "NX-E0102" && d.subcause.as_deref() == Some("type_mismatch"))),
            "{source}: {result:?}"
        );
    }
    let bool_prefix = "fn main(){var v=if true{return;}else{return;};v=true;";
    compile_scalar("deferred_assign.nx", &format!("{bool_prefix}print(v);}}"))
        .expect("the first concrete type may also be bool");
    let source = format!("{bool_prefix}v=5;}}");
    let result = compile_scalar("deferred_assign.nx", &source);
    assert!(
        matches!(result, Err(CompileError::Rejected(ref ds)) if ds.iter().any(|d| d.code == "NX-E0102" && d.subcause.as_deref() == Some("type_mismatch"))),
        "{source}: {result:?}"
    );
    compile_scalar("deferred_assign.nx", &format!("{prefix}let w:i64=v;}}"))
        .expect("the first concrete type is preserved by a later binding");
}

#[test]
fn annotated_divergent_binding_reports_its_declared_type() {
    let source = "fn main(){let v:i64=if true{return;}else{return;};if v{}}";
    let Err(CompileError::Rejected(errors)) = compile_scalar("ann_type.nx", source) else {
        panic!("expected a type error");
    };
    let mismatch = errors.iter().find(|d| d.code == "NX-E0102").unwrap();
    assert_eq!(
        mismatch.data.get("found_type"),
        Some(&crate::diagnostic::DataValue::String("i64".into()))
    );
    assert_eq!(
        mismatch.data.get("expected_type"),
        Some(&crate::diagnostic::DataValue::String("bool".into()))
    );
    let valid = "fn main(){let v:i64=if true{return;}else{return;};print(v+1);}";
    let program = compile_scalar("ann_type.nx", valid).unwrap();
    assert_eq!(
        program.functions.function("main").unwrap().locals[0].ty,
        model::Ty::I64
    );
}

#[test]
fn divergent_binary_result_types_allow_matching_annotations() {
    for source in [
        "fn main(){let v=1+(if true{return;}else{return;});let w:i64=v;}",
        "fn main(){let v=1<(if true{return;}else{return;});let w:bool=v;}",
    ] {
        compile_scalar("binary_result_positive.nx", source).unwrap();
    }
}

#[test]
fn value_immediately_after_return_is_checked_without_emission() {
    let source = "fn main(){print(if true{return;value:1;}else{value:2;});}";
    let program = compile_scalar("dead_value.nx", source).unwrap();
    let main = program.functions.function("main").unwrap();
    let dead_literals = std::iter::once(&main.block)
        .chain(&main.blocks)
        .flat_map(|block| &block.instructions)
        .filter(|ins| matches!(ins.op, Op::ConstI64(1)))
        .count();
    assert_eq!(dead_literals, 0, "dead value must not emit an instruction");
    assert!(
        crate::interp::run_scalar(&program)
            .observations
            .stdout()
            .is_empty()
    );
}

#[test]
#[ignore = "explicit 200000-program divergence campaign with template oracle"]
fn directed_divergence_positions_match_reference_acceptance_and_diagnostics() {
    let limit = std::env::var("NEXO_DIVERGENCE_CAMPAIGN_LIMIT")
        .ok()
        .and_then(|n| n.parse::<usize>().ok())
        .unwrap_or(200_000);
    let dv = "(if true{return;}else{return;})";
    for seed in 0..limit {
        let case = seed % 36;
        let n = seed + 1;
        // The reference contract is assigned by the grammar template, before
        // compilation. The compiler's output never chooses the expectation.
        let (body, expected): (String, Option<(&str, &str)>) = match case {
            0 => (format!("print({dv}+base);"), None),
            1 => (format!("print(base+{dv});"), None),
            2 => (format!("h(base,{dv});"), None),
            3 => (format!("let a=[base];print(a[{dv}]);"), None),
            4 => (format!("var a=base;a={dv};"), None),
            5 => (format!("if {dv}{{print(base);}}"), None),
            6 => (format!("while {dv}{{print(base);}}"), None),
            7 => (
                format!("print(if true{{value:{dv};}}else{{value:base;}});"),
                None,
            ),
            8 => ("h();".into(), None),
            9 => (format!("return;print(base+{dv});"), None),
            10 => (format!("while false{{print({dv});}}"), None),
            11 => (format!("let a:i64={dv};print(a+base);"), None),
            12 => (format!("let a={dv};print(a+base);"), None),
            13 => (
                "print(if true{value:base;}else{print(base);});".into(),
                Some(("NX-E1302", "missing_branch_value")),
            ),
            14 => (
                "return;print(if true{value:base;}else{print(base);});".into(),
                Some(("NX-E1302", "missing_branch_value")),
            ),
            15 => (
                format!("print({dv}+true);"),
                Some(("NX-E0102", "type_mismatch")),
            ),
            16 => (
                "return;let r=mock_open(1);drop(r);inspect(r);".into(),
                Some(("NX-E0202", "definitely_consumed")),
            ),
            17 => (
                format!("let a:i64={dv};if a{{}}"),
                Some(("NX-E0102", "type_mismatch")),
            ),
            18 => (
                format!("while (if base<1{{value:{dv};}}else{{value:false;}}){{}}"),
                None,
            ),
            19 => (
                format!("h({dv},if true{{value:base;}}else{{print(base);}});"),
                Some(("NX-E1302", "missing_branch_value")),
            ),
            20 => (format!("let v={dv};print(v==base);"), None),
            21 => (format!("let v={dv};let w=v;print(w+base);"), None),
            22 => (
                format!("let v={dv};print(if true{{value:v;}}else{{value:base;}});"),
                None,
            ),
            23 => (format!("let v={dv};let w=[v,base];"), None),
            24 => (format!("var v={dv};v=base;"), None),
            25 => (format!("let v=base+{dv};print(v==base);"), None),
            26 => (format!("let v={dv};let a=[base];print(a[v]);"), None),
            27 => (format!("let v={dv};h(v,base);"), None),
            28 => (format!("let v={dv};if v{{print(base);}}"), None),
            29 => ("f();".into(), None),
            30 => (format!("let v={dv};let a=[base];h(a[v],base);"), None),
            31 => (format!("let v={dv};print(base<v);"), None),
            32 => (format!("let v={dv};let w:bool=v;"), None),
            33 => (format!("let v={dv};let w=v+v;"), None),
            34 => (
                format!("let v=({dv})+base;let w:bool=v;"),
                Some(("NX-E0102", "type_mismatch")),
            ),
            _ => (
                format!("let v=({dv})<base;let w:i64=v;"),
                Some(("NX-E0102", "type_mismatch")),
            ),
        };
        let helper = if case == 8 {
            format!(
                "fn h()->i64{{let base={n};return if true{{return base;}}else{{return base;}};}} "
            )
        } else if case == 29 {
            "fn f()->i64{let v=if true{return 1;}else{return 2;};return v;} ".into()
        } else if case == 2 || case == 19 || case == 27 || case == 30 {
            "fn h(a:i64,b:i64){} ".to_string()
        } else {
            String::new()
        };
        let source = format!("{helper}fn main(){{let base={n};{body}}}");
        let result = compile_scalar("divergence_campaign.nx", &source);
        match (expected, result) {
            (None, Ok(program)) => {
                let execution = crate::interp::run_scalar(&program);
                assert!(
                    execution.observations.stdout().is_empty(),
                    "seed {seed}: {source}"
                );
            }
            (Some((code, subcause)), Err(CompileError::Rejected(errors))) => {
                assert!(
                    errors
                        .iter()
                        .any(|d| d.code == code && d.subcause.as_deref() == Some(subcause)),
                    "seed {seed}: {source}: {errors:?}"
                );
                for error in errors {
                    error.validate("divergence_campaign.nx", &source).unwrap();
                }
            }
            (_, other) => panic!("seed {seed}: {source}: {other:?}"),
        }
    }
}

#[test]
fn divergent_if_condition_checks_both_hypothetical_branches() {
    let valid = "fn main(){print(if (if true{return;}else{return;}){value:1;}else{value:2;});}";
    let program = compile_scalar("divergent_condition.nx", valid).unwrap();
    assert!(
        crate::interp::run_scalar(&program)
            .observations
            .stdout()
            .is_empty()
    );
    let invalid = "fn main(){print(if (if true{return;}else{return;}){value:1;}else{});}";
    let result = compile_scalar("divergent_condition.nx", invalid);
    assert!(
        matches!(result, Err(CompileError::Rejected(ref ds)) if ds.iter().any(|d| d.code == "NX-E1302" && d.subcause.as_deref() == Some("missing_branch_value"))),
        "{result:?}"
    );
}

#[test]
fn divergent_else_if_and_deferred_call_argument_keep_branch_structure() {
    let valid = "fn main(){print(if true{value:1;}else if true{return;}else{return;});}";
    let program = compile_scalar("else_if_never.nx", valid).unwrap();
    assert_eq!(
        crate::interp::run_scalar(&program).observations.stdout(),
        ["1"]
    );
    let invalid = "fn h(x:i64){} fn main(){let v=if true{return;}else{return;};print(if true{value:1;}else{h(v);});}";
    let result = compile_scalar("deferred_arg.nx", invalid);
    assert!(
        matches!(result, Err(CompileError::Rejected(ref ds)) if ds.iter().any(|d| d.code == "NX-E1302" && d.subcause.as_deref() == Some("missing_branch_value"))),
        "{result:?}"
    );
}

#[test]
fn divergent_left_binary_operand_still_checks_numeric_right_type() {
    let source = "fn main(){print((if true{return;}else{return;})+true);}";
    let result = compile_scalar("left_never.nx", source);
    assert!(
        matches!(result, Err(CompileError::Rejected(ref ds)) if ds.iter().any(|d| d.code == "NX-E0102" && d.subcause.as_deref() == Some("type_mismatch"))),
        "{result:?}"
    );
    compile_scalar(
        "left_never.nx",
        "fn main(){print((if true{return;}else{return;})+1);}",
    )
    .expect("a diverging left operand coerces to i64");
}

#[test]
fn verifier_rejects_a_forged_unreachable_branch_block() {
    let program = compile_scalar(
        "unreachable_cfg.nx",
        "fn main(){print(if true{value:1;}else{value:2;});}",
    )
    .unwrap();
    let mut function = program.functions.function("main").unwrap().clone();
    let Terminator::BranchBool { yes, no, .. } = &mut function.block.terminator else {
        panic!("expected branching entry");
    };
    *no = *yes;
    function.content_hash = model::content_hash(&function);
    let (result, steps) =
        super::measure_cfg_thread_work(|| super::verify::verify_function(&function));
    assert_eq!(result, Err("unreachable block"));
    assert!(steps <= 5 * (function.blocks.len() as u64 + 1), "{steps}");
}

#[test]
fn branch_diagnostic_restores_nested_scope_state() {
    for source in [
        "fn main(){let v=(if false{();}else{value:5;});return;(if true{value:v;}else{value:1;});}",
        "fn main(){let v=if true{var w=(if false{value:4;}else{});value:(w==w);}else{value:true;};}",
    ] {
        let result = compile_scalar("branch_error.nx", source);
        assert!(
            matches!(result, Err(CompileError::Rejected(ref errors)) if errors.iter().any(|d| d.code == "NX-E1302" && d.subcause.as_deref() == Some("missing_branch_value"))),
            "{source}: {result:?}"
        );
    }
}

#[test]
fn nested_branch_error_keeps_later_independent_ownership_error() {
    let source = "fn main(){let r=mock_open(1);let v=if true{let x=if false{value:4;}else{};drop(r);value:r;}else{value:r;};}";
    let result = compile_scalar("branch_two_errors.nx", source);
    let Err(CompileError::Rejected(errors)) = result else {
        panic!("{result:?}")
    };
    let codes: Vec<_> = errors
        .iter()
        .map(|d| (d.code.as_str(), d.subcause.as_deref()))
        .collect();
    assert!(
        codes.contains(&("NX-E1302", Some("missing_branch_value"))),
        "{codes:?}"
    );
    assert!(
        codes.contains(&("NX-E0202", Some("definitely_consumed"))),
        "{codes:?}"
    );
}

#[test]
fn failed_while_condition_restores_lowering_mode_for_later_errors() {
    let source =
        "fn main(){while (if true{value:true;}else{}){}let r=mock_open(1);drop(r);inspect(r);}";
    let result = compile_scalar("while_error_state.nx", source);
    let Err(CompileError::Rejected(errors)) = result else {
        panic!("{result:?}")
    };
    let codes: Vec<_> = errors
        .iter()
        .map(|d| (d.code.as_str(), d.subcause.as_deref()))
        .collect();
    assert!(
        codes.contains(&("NX-E1302", Some("missing_branch_value"))),
        "{codes:?}"
    );
    assert!(
        codes.contains(&("NX-E0202", Some("definitely_consumed"))),
        "{codes:?}"
    );
}

#[test]
fn missing_branch_value_suppresses_dependent_missing_return() {
    // ADR §13.9: the absent branch value is diagnosed before a return
    // obligation derived solely from that absent value.
    let source = "fn f()->i64{let x=if true{1;}else{value:2;};} fn main(){}";
    let Err(CompileError::Rejected(errors)) = compile_scalar("branch_return.nx", source) else {
        panic!("expected source diagnostic");
    };
    assert_eq!(errors.len(), 1);
    assert_eq!(errors[0].code, "NX-E1302");
    assert_eq!(errors[0].subcause.as_deref(), Some("missing_branch_value"));
}

#[test]
fn equality_copies_indexed_left_value_before_rhs_write() {
    let source = "fn main(){var a=[2];print(a[0]==(if true{a[0]=5;value:2;}else{value:2;}));}";
    let program = compile_scalar("eq_copy.nx", source).unwrap();
    let result = crate::interp::run_scalar(&program);
    assert_eq!(result.observations.stdout(), ["true"]);
    let addition = "fn main(){var a=[2];print(a[0]+(if true{a[0]=5;value:2;}else{value:2;}));}";
    let program = compile_scalar("add_copy.nx", addition).unwrap();
    assert_eq!(
        crate::interp::run_scalar(&program).observations.stdout(),
        ["4"]
    );
    let call =
        "fn h(x:i64,y:i64){} fn main(){var a=[2];h(a[0],if true{a[0]=5;value:2;}else{value:2;});}";
    let rejected = compile_scalar("call_loan.nx", call);
    assert!(
        matches!(rejected, Err(CompileError::Rejected(ref errors)) if errors.iter().any(|d| d.code == "NX-E1105")),
        "{rejected:?}"
    );
    let array =
        "fn main(){var a=[[2]];print(a[0]==(if true{a[0]=[5];value:[2];}else{value:[2];}));}";
    let rejected = compile_scalar("array_eq_loan.nx", array);
    if let Err(CompileError::Rejected(errors)) = &rejected {
        for diagnostic in errors {
            diagnostic.validate("array_eq_loan.nx", array).unwrap();
        }
    }
    assert!(
        matches!(rejected, Err(CompileError::Rejected(ref errors)) if errors.iter().any(|d| d.code == "NX-E1105" && d.subcause.as_deref() == Some("active_borrow_conflict"))),
        "{rejected:?}"
    );
}

#[test]
fn divergent_if_operands_and_bindings_do_not_form_invalid_ir() {
    let sources = [
        "fn main(){print((if true{return;}else{return;})+0);}",
        "fn main(){print(1+(if true{return;}else{return;}));}",
        "fn h(a:i64,b:i64)->i64{return a;} fn main(){h(1,if true{return;}else{return;});}",
        "fn main(){var a=[1];a[0]=if true{return;}else{return;};}",
        "fn main(){let x=1;if true{return;}else{return;}}",
        "fn main(){var a=[1];a[0]=2;if true{return;}else{return;}}",
        "fn main(){if (if true{return;}else{return;}){print(1);}}",
        "fn main(){while (if true{return;}else{return;}){print(1);}}",
        "fn main(){while false{print(if true{return;}else{return;});}}",
        "fn main(){let v=if true{return;}else{return;};print(v+1);}",
        "fn main(){let v=if true{return;}else{return;};if v{print(1);}}",
        "fn h(a:i64,b:i64){} fn main(){let v=if true{return;}else{return;};h(1,v);}",
        "fn main(){let v=if true{return;}else{return;};var a=[1];a[v]=2;}",
    ];
    for source in sources {
        let result = compile_scalar("diverge_operand.nx", source);
        assert!(
            matches!(result, Ok(_) | Err(CompileError::Rejected(_))),
            "{source}: {result:?}"
        );
    }
}

#[test]
fn direct_lowering_discards_divergent_composite_index_operands() {
    for source in [
        "fn main(){print((if true{return;}else{return;})[0]);}",
        "fn main(){print(([1,2])[if true{return;}else{return; }]);}",
    ] {
        let ast = crate::frontend::parse::parse("direct_diverge_index.nx", source).unwrap();
        let result = super::lower::compile_ast(&ast, source);
        assert!(result.is_ok(), "{source}: {result:?}");
    }
}

#[test]
fn divergent_values_cover_branch_shapes_operand_positions_and_annotations() {
    let forms = [
        "if true{return;}else{value:1;}",
        "if true{value:1;}else{return;}",
        "if true{return;}else{return;}",
        "if true{value:if false{return;}else{value:1;};}else{value:2;}",
    ];
    for form in forms {
        let bodies = [
            format!("print(({form})+1);"),
            format!("print(1+({form}));"),
            format!("h({form},2);"),
            format!("h(1,{form});"),
            format!("var a=[1,2];print(a[{form}]);"),
            format!("print(({form})[0]);"),
            format!("print(([1,2])[{form}]);"),
            format!("var a=[1,2];a[{form}]=2;"),
            format!("var a=[1,2];a[0]={form};"),
            format!("if {form}{{print(1);}}"),
            format!("while {form}{{print(1);}}"),
            format!("let v={form};print(v+1);"),
            format!("let v:i64={form};print(v+1);"),
            format!("let v={form};h(1,v);"),
            format!("let v={form};var a=[1];a[v]=2;"),
            format!("let v={form};if v{{print(1);}}"),
            format!("let v:bool={form};if v{{print(1);}}"),
            format!("let v=if true{{value:{form};}}else{{value:1;}};print(v);"),
        ];
        for body in bodies {
            let source = format!("fn h(a:i64,b:i64){{}} fn main(){{{body}}}");
            let result = compile_scalar("diverge_matrix.nx", &source);
            assert!(
                matches!(result, Ok(_) | Err(CompileError::Rejected(_))),
                "{source}: {result:?}"
            );
        }
        let source = format!("fn f()->i64{{return {form};}} fn main(){{print(f());}}");
        let result = compile_scalar("diverge_return.nx", &source);
        assert!(
            matches!(result, Ok(_) | Err(CompileError::Rejected(_))),
            "{source}: {result:?}"
        );
    }
}

#[test]
fn divergent_later_argument_releases_pending_read_and_inout_loans() {
    for source in [
        "fn h(a:MockResource,b:i64){} fn main(){let r=mock_open(1);h(r,if true{return;}else{return;});}",
        "fn h(inout a:i64,b:i64){} fn main(){var a=1;h(inout a,if true{return;}else{return;});}",
    ] {
        let program = compile_scalar("diverge_loan.nx", source).unwrap();
        let result = crate::interp::run_scalar(&program);
        assert!(
            matches!(result.outcome, crate::interp::Outcome::Unit),
            "{source}: {result:?}"
        );
    }
}

#[test]
fn if_values_after_other_operands_remain_available_across_cfg_edges() {
    for (source, expected) in [
        (
            "fn f(a:i64,b:i64){print(b);} fn main(){f(1,if true{value:2;}else{value:3;});}",
            "2\n",
        ),
        (
            "fn main(){print(1+(if true{value:2;}else{value:3;}));}",
            "3\n",
        ),
    ] {
        let program = compile_scalar("live_temp.nx", source).unwrap();
        let result = crate::interp::run_scalar(&program);
        assert_eq!(result.observations.stdout(), [expected.trim_end()]);
    }
}

#[test]
fn nested_branch_values_work_in_later_arguments_indexes_and_assignments() {
    let source = "fn f(a:i64,b:i64,c:i64){assert_eq(a+b+c,6);} fn main(){f(1,2,if true{value:3;}else{value:4;});var a=[1,2,3];a[if true{value:1;}else{value:0;}]=if false{value:8;}else{value:9;};print(a[1]);print((if true{value:1;}else{value:2;})+(if true{value:2;}else{value:3;}));}";
    let program = compile_scalar("nested_live.nx", source).unwrap();
    let run = crate::interp::run_scalar(&program);
    assert_eq!(run.observations.stdout(), ["9", "3"]);
}

#[test]
fn read_loan_survives_branch_value_in_later_call_argument() {
    let source = "fn f(r:MockResource,n:i64){inspect(r);print(n);} fn main(){let r=mock_open(7);f(r,if true{value:2;}else{value:3;});inspect(r);}";
    let program = compile_scalar("loan_live.nx", source).unwrap();
    let run = crate::interp::run_scalar(&program);
    assert_eq!(run.observations.stdout(), ["2"]);
    assert_eq!(
        run.observations.events(),
        ["open:0:7", "inspect:0:7", "inspect:0:7", "close:0:7"]
    );
}

#[test]
fn fault_in_branch_argument_releases_earlier_live_read_loan_and_owner() {
    let source = "fn f(r:MockResource,n:i64){inspect(r);} fn main(){let r=mock_open(7);f(r,if true{fail();value:2;}else{value:3;});}";
    let program = compile_scalar("branch_fault.nx", source).unwrap();
    let run = crate::interp::run_scalar(&program);
    assert!(matches!(
        run.outcome,
        crate::interp::Outcome::SimulatorFault
    ));
    assert_eq!(run.diagnostics[0].code, "R0301");
    assert_eq!(run.observations.events(), ["open:0:7", "close:0:7"]);
}

#[test]
fn while_condition_consuming_external_resource_needs_flow_fixpoint() {
    for source in [
        "fn main(){let r=mock_open(1);while drop(r)==(){} }",
        "fn main(){let r=mock_open(1);while drop(r)==(){} inspect(r);}",
        "fn main(){let r=mock_open(1);while drop(r)==(){inspect(r);}}",
    ] {
        assert!(
            matches!(
                compile_scalar("while_consume.nx", source),
                Err(CompileError::OutsideScalarSlice("ownership fixpoint"))
            ),
            "{source}"
        );
    }
    let borrowed = "fn main(){let r=mock_open(1);while inspect(r)==(){return;}inspect(r);}";
    assert!(compile_scalar("while_borrow.nx", borrowed).is_ok());
}

#[test]
fn branch_false_target_and_instruction_type_affect_semantic_hash() {
    let source = "fn main(){if true{print(1);}else{print(2);}}";
    let function = compile_scalar("hash_branch.nx", source)
        .unwrap()
        .functions
        .function("main")
        .unwrap()
        .clone();
    let original = model::content_hash(&function);
    let mut changed = function.clone();
    if let Terminator::BranchBool { yes, no, .. } = &mut changed.block.terminator {
        *no = *yes;
    }
    assert_ne!(model::content_hash(&changed), original);
    let mut scope_only = function.clone();
    assert!(scope_only.scope_count > 1);
    scope_only.block.instructions[0].scope = model::ScopeId(1);
    assert_eq!(model::content_hash(&scope_only), original);
    assert_eq!(super::verify::verify_function(&scope_only), Ok(()));
    let mut changed = function;
    changed.block.instructions[0].ty = Some(model::Ty::I64);
    assert_ne!(model::content_hash(&changed), original);
}

#[test]
fn true_branch_and_instruction_scope_have_distinct_hash_roles() {
    let source = "fn main(){if true{print(1);}else{print(2);}}";
    let program = compile_scalar("hash_yes.nx", source).unwrap();
    let function = program.functions.function("main").unwrap();
    let original = model::content_hash(function);
    let mut changed = function.clone();
    if let Terminator::BranchBool { yes, no, .. } = &mut changed.block.terminator {
        *yes = *no;
    }
    assert_ne!(model::content_hash(&changed), original);
    let mut scope_only = program.clone();
    let main = scope_only
        .functions
        .entries
        .iter_mut()
        .find(|f| f.name == "main")
        .unwrap();
    main.block.instructions[0].scope = model::ScopeId(1);
    assert_eq!(model::content_hash(main), original);
    assert_eq!(super::verify::verify_function(main), Ok(()));
    let original_run = crate::interp::run_scalar(&program);
    let scope_run = crate::interp::run_scalar(&scope_only);
    assert_eq!(
        original_run.observations.stdout(),
        scope_run.observations.stdout()
    );
    assert_eq!(
        original_run.observations.events(),
        scope_run.observations.events()
    );
}

#[test]
fn terminator_operand_stays_live_across_jump() {
    let source = "fn main(){print(1);}";
    let mut function = compile_scalar("term_live.nx", source)
        .unwrap()
        .functions
        .function("main")
        .unwrap()
        .clone();
    let original = function.block.terminator.clone();
    let span = match &original {
        Terminator::Return { span, .. } => span.clone(),
        _ => panic!("expected return"),
    };
    function.block.terminator = Terminator::Jump {
        target: BlockId(1),
        span,
    };
    function.blocks.push(model::BasicBlock {
        id: BlockId(1),
        instructions: Vec::new(),
        terminator: original,
    });
    function.content_hash = model::content_hash(&function);
    assert_eq!(super::verify::verify_function(&function), Ok(()));
}

#[test]
fn verifier_rejects_temporary_dropped_on_only_one_branch() {
    let source = "fn main(){print(1+(if true{return;}else{return;}));}";
    let mut function = compile_scalar("edge_leak.nx", source)
        .unwrap()
        .functions
        .function("main")
        .unwrap()
        .clone();
    let arm = function
        .blocks
        .iter_mut()
        .find(|block| {
            block
                .instructions
                .iter()
                .any(|ins| matches!(ins.op, Op::DropValue(_)))
        })
        .unwrap();
    let at = arm
        .instructions
        .iter()
        .position(|ins| matches!(ins.op, Op::DropValue(_)))
        .unwrap();
    arm.instructions.remove(at);
    for (index, ins) in arm.instructions.iter_mut().enumerate() {
        ins.origin = model::OriginId(index);
    }
    function.content_hash = model::content_hash(&function);
    assert_eq!(
        super::verify::verify_function(&function),
        Err("temporary leaked on edge")
    );
}

#[test]
fn dominator_of_loop_header_includes_nonentry_preheader() {
    let source = "fn main(){if true{}else{}while false{}}";
    let mut function = compile_scalar("dom_loop.nx", source)
        .unwrap()
        .functions
        .function("main")
        .unwrap()
        .clone();
    let condition = function
        .blocks
        .iter()
        .find(|block| {
            block
                .instructions
                .iter()
                .any(|ins| matches!(ins.op, Op::ConstBool(false)))
        })
        .unwrap()
        .id;
    let preheader = function.blocks.iter().find(|block| block.id.0 < condition.0
        && matches!(block.terminator, Terminator::Jump { target, .. } if target == condition))
        .unwrap().id;
    let exit = match function
        .blocks
        .iter()
        .find(|block| block.id == condition)
        .unwrap()
        .terminator
    {
        Terminator::BranchBool { no, .. } => no,
        _ => panic!("while condition must branch"),
    };
    let temp = TempId(10_000);
    let before = function
        .blocks
        .iter_mut()
        .find(|block| block.id == preheader)
        .unwrap();
    let span = match &before.terminator {
        Terminator::Jump { span, .. } => span.clone(),
        _ => unreachable!(),
    };
    before.instructions.push(model::Instruction {
        op: Op::ConstI64(99),
        output: Some(temp),
        ty: Some(model::Ty::I64),
        span,
        scope: model::ScopeId(0),
        origin: model::OriginId(before.instructions.len()),
        access: model::Access::Own,
        effect_io: false,
        tick: true,
    });
    let after = function
        .blocks
        .iter_mut()
        .find(|block| block.id == exit)
        .unwrap();
    let span = match &after.terminator {
        Terminator::Return { span, .. } => span.clone(),
        _ => unreachable!(),
    };
    after.instructions.insert(
        0,
        model::Instruction {
            op: Op::DropValue(temp),
            output: None,
            ty: None,
            span,
            scope: model::ScopeId(0),
            origin: model::OriginId(0),
            access: model::Access::Own,
            effect_io: false,
            tick: true,
        },
    );
    for (index, ins) in after.instructions.iter_mut().enumerate() {
        ins.origin = model::OriginId(index);
    }
    function.content_hash = model::content_hash(&function);
    assert_eq!(super::verify::verify_function(&function), Ok(()));
}

#[test]
fn cfg_verifier_rejects_orphan_output_on_a_branch() {
    let source = "fn main(){if true{}else{}}";
    let mut function = compile_scalar("orphan_branch.nx", source)
        .unwrap()
        .functions
        .function("main")
        .unwrap()
        .clone();
    let branch = &mut function.blocks[0];
    let span = match &branch.terminator {
        Terminator::Jump { span, .. } => span.clone(),
        _ => panic!("branch shape"),
    };
    branch.instructions.push(model::Instruction {
        op: Op::ConstI64(9),
        output: Some(TempId(10_000)),
        ty: Some(model::Ty::I64),
        span,
        scope: model::ScopeId(0),
        origin: model::OriginId(branch.instructions.len()),
        access: model::Access::Own,
        effect_io: false,
        tick: true,
    });
    function.content_hash = model::content_hash(&function);
    assert_eq!(
        super::verify::verify_function(&function),
        Err("unconsumed temporary")
    );
}

#[test]
fn cfg_verifier_rejects_operand_consumed_on_only_one_predecessor() {
    let source = "fn f(a:i64,b:i64){print(b);} fn main(){f(1,if true{value:2;}else{value:3;});}";
    let mut function = compile_scalar("partial_temp.nx", source)
        .unwrap()
        .functions
        .function("main")
        .unwrap()
        .clone();
    let carried = function
        .block
        .instructions
        .iter()
        .find_map(|ins| matches!(ins.op, Op::ConstI64(1)).then_some(ins.output.unwrap()))
        .unwrap();
    let branch = &mut function.blocks[0];
    let span = match &branch.terminator {
        Terminator::Jump { span, .. } => span.clone(),
        _ => panic!("branch shape"),
    };
    branch.instructions.push(model::Instruction {
        op: Op::DropValue(carried),
        output: None,
        ty: None,
        span,
        scope: model::ScopeId(0),
        origin: model::OriginId(branch.instructions.len()),
        access: model::Access::Own,
        effect_io: false,
        tick: true,
    });
    function.content_hash = model::content_hash(&function);
    assert_eq!(
        super::verify::verify_function(&function),
        Err("temporary used twice")
    );
}

#[test]
fn branch_union_rechecks_value_type_when_semantic_gate_is_bypassed() {
    let source = "fn main(){let v=if true{value:1;}else{value:true;};}";
    let ast = crate::frontend::parse::parse("bad_union.nx", source).unwrap();
    assert!(matches!(
        super::lower::compile_ast(&ast, source),
        Err(CompileError::InvalidIr("branch value type"))
    ));
}

#[test]
fn consumed_scoped_binding_has_no_second_cleanup_instruction() {
    let source = "fn main(){{let r=mock_open(1);drop(r);}}";
    let program = compile_scalar("scope_consumed.nx", source).unwrap();
    let function = program.functions.function("main").unwrap();
    assert!(
        !function
            .block
            .instructions
            .iter()
            .any(|ins| matches!(ins.op, Op::DropLocal(_)))
    );
    assert_eq!(
        crate::interp::run_scalar(&program).observations.events(),
        ["open:0:1", "close:0:1"]
    );
}

#[test]
fn diverging_if_value_does_not_create_an_ir_temporary() {
    for source in [
        "fn main(){if false{let v=if true{return;}else{return;};}}",
        "fn f()->i64{let v=if true{return 1;}else{return 2;};return v;} fn main(){}",
        "fn main(){let a=if true{value:if true{return;}else{return;};}else{value:3;};print(a);}",
    ] {
        let result = compile_scalar("never.nx", source);
        assert!(
            !matches!(result, Err(CompileError::InvalidIr(_))),
            "{source}: {result:?}"
        );
    }
}

#[test]
fn a_returning_branch_coerces_to_the_other_branch_value_type() {
    for (source, expected) in [
        (
            "fn f(b:bool)->i64{let v=if b{return 1;}else{value:2;};return v;} fn main(){print(f(false));}",
            "2",
        ),
        (
            "fn f(b:bool)->i64{let v=if b{value:2;}else{return 1;};return v;} fn main(){print(f(true));}",
            "2",
        ),
        (
            "fn f()->i64{let v:i64=if true{return 1;}else{return 2;};return v;} fn main(){print(f());}",
            "1",
        ),
        (
            "fn f()->i64{let v=if true{value:if true{return 1;}else{return 2;};}else{value:3;};return v;} fn main(){print(f());}",
            "1",
        ),
    ] {
        let program =
            compile_scalar("coerce_never.nx", source).unwrap_or_else(|e| panic!("{source}: {e:?}"));
        assert_eq!(
            crate::interp::run_scalar(&program).observations.stdout(),
            [expected]
        );
    }
}

#[test]
fn all_reaching_consumptions_are_reported_in_source_order() {
    let source = "fn f(b:bool,c:bool){let r=mock_open(1);if b{drop(r);}else if c{drop(r);}inspect(r);} fn main(){}";
    let Err(CompileError::Rejected(errors)) = compile_scalar("consumptions.nx", source) else {
        panic!("expected ownership error");
    };
    let error = errors.iter().find(|e| e.code == "NX-E0202").unwrap();
    let actual: Vec<_> = error
        .related_spans
        .iter()
        .filter(|r| r.role == crate::diagnostic::RelatedRole::Consumption)
        .map(|r| r.span.start)
        .collect();
    let expected: Vec<_> = source
        .match_indices("drop(r)")
        .map(|(at, _)| at as u64)
        .collect();
    assert_eq!(actual, expected);
}

#[test]
fn nested_and_transferred_consumption_sites_reach_the_final_use() {
    for source in [
        "fn f(b:bool,c:bool,d:bool){let r=mock_open(1);if b{drop(r);}else if c{drop(r);}else if d{drop(r);}inspect(r);} fn main(){}",
        "fn f(b:bool,c:bool){let r=mock_open(1);if b{if c{drop(r);}else{let q=consume r;}}else{drop(r);}inspect(r);} fn main(){}",
    ] {
        let Err(CompileError::Rejected(errors)) = compile_scalar("consumption_branches.nx", source)
        else {
            panic!("expected ownership error");
        };
        let error = errors.iter().find(|e| e.code == "NX-E0202").unwrap();
        let mut actual: Vec<_> = error
            .related_spans
            .iter()
            .filter(|r| r.role == crate::diagnostic::RelatedRole::Consumption)
            .map(|r| r.span.start)
            .collect();
        let mut expected: Vec<_> = source
            .match_indices("drop(r)")
            .map(|(at, _)| at as u64)
            .chain(source.match_indices("consume r").map(|(at, _)| at as u64))
            .collect();
        actual.sort_unstable();
        expected.sort_unstable();
        assert_eq!(actual, expected, "{source}");
    }
}

#[test]
fn all_paths_consumed_reports_every_reaching_site() {
    let source = "fn main(){let r=mock_open(1);if true{drop(r);}else{drop(r);}inspect(r);}";
    let Err(CompileError::Rejected(errors)) = compile_scalar("definitely_sites.nx", source) else {
        panic!("expected ownership error");
    };
    let error = errors.iter().find(|e| e.code == "NX-E0202").unwrap();
    assert_eq!(error.subcause.as_deref(), Some("definitely_consumed"));
    let actual: Vec<_> = error
        .related_spans
        .iter()
        .filter(|r| r.role == crate::diagnostic::RelatedRole::Consumption)
        .map(|r| r.span.start)
        .collect();
    let expected: Vec<_> = source
        .match_indices("drop(r)")
        .map(|(at, _)| at as u64)
        .collect();
    assert_eq!(actual, expected);
}

#[test]
fn adr_gap_missing_final_else_in_chain_marks_first_if_keyword() {
    // F0-0002 §14.3 B01 anchors a single if. The chain's outer anchor is
    // current behavior; the ADR does not specify this nested case explicitly.
    let source = "fn main(){let q=if true{value:1;}else if false{value:2;};}";
    let Err(CompileError::Rejected(errors)) = compile_scalar("else_chain.nx", source) else {
        panic!("expected parse error");
    };
    assert_eq!(errors.len(), 1);
    assert_eq!(errors[0].subcause.as_deref(), Some("missing_else"));
    assert_eq!(
        errors[0].primary_span.as_ref().unwrap().start,
        source.find("if true").unwrap() as u64
    );
    assert_eq!(
        errors[0].primary_span.as_ref().unwrap().end,
        source.find("if true").unwrap() as u64 + 2
    );
}

#[test]
fn missing_branch_value_in_one_function_does_not_hide_later_missing_return() {
    let source = "fn a(){let q=if true{value:1;}else{};} fn f()->i64{} fn main(){}";
    let Err(CompileError::Rejected(errors)) = compile_scalar("independent_flow.nx", source) else {
        panic!("expected flow diagnostics");
    };
    assert_eq!(
        errors
            .iter()
            .map(|e| (e.code.as_str(), e.subcause.as_deref()))
            .collect::<Vec<_>>(),
        [
            ("NX-E1302", Some("missing_branch_value")),
            ("NX-E1317", None)
        ]
    );
}

#[test]
fn missing_else_is_the_first_parse_failure_across_functions() {
    for source in [
        "fn f(){let a:i64=true;} fn g(){let q=if true{value:1;};} fn main(){}",
        "fn g(){let q=if true{value:1;};} fn f()->i64{ } fn main(){}",
        "fn g(){let q=if true{value:1;};} fn h(){ value: 1; } fn main(){}",
    ] {
        let Err(CompileError::Rejected(errors)) = compile_scalar("parse_stage.nx", source) else {
            panic!("expected missing else");
        };
        assert_eq!(errors.len(), 1, "{source}: {errors:?}");
        let error = &errors[0];
        assert_eq!(error.code, "NX-E1302");
        assert_eq!(error.subcause.as_deref(), Some("missing_else"));
        assert_eq!(error.stage, crate::diagnostic::Stage::Parse);
        assert_eq!(
            error.primary_span.as_ref().unwrap().start,
            source.find("if true").unwrap() as u64
        );
    }
}

#[test]
fn independent_missing_return_and_consumed_use_are_both_reported() {
    let source = "fn f(b:bool)->i64{let r=mock_open(1);if b{drop(r);}inspect(r);} fn main(){}";
    let Err(CompileError::Rejected(errors)) = compile_scalar("return_and_use.nx", source) else {
        panic!("expected ownership errors");
    };
    let codes: Vec<_> = errors.iter().map(|e| e.code.as_str()).collect();
    assert_eq!(codes, ["NX-E0202", "NX-E1317"]);
    assert!(
        errors
            .iter()
            .all(|e| e.stage == crate::diagnostic::Stage::Ownership)
    );
}

#[test]
fn generated_cfg_programs_match_independent_integer_model() {
    fn model(start: i64, threshold: i64, choose_first: bool) -> i64 {
        let mut value = start;
        if choose_first {
            value += 2;
        } else {
            value += 3;
        }
        while value < threshold {
            value += 1;
        }
        value
    }
    for seed in 0..8_000_u64 {
        let start = (seed % 7) as i64;
        let threshold = (seed / 7 % 12) as i64;
        let choose_first = seed / 84 % 2 == 0;
        let source = format!(
            "fn main(){{var x={start};if {choose_first}{{x=x+2;}}else{{x=x+3;}}while x<{threshold}{{x=x+1;}}assert_eq(x,{});}}",
            model(start, threshold, choose_first),
        );
        let program = compile_scalar("generated.nx", &source)
            .unwrap_or_else(|error| panic!("seed {seed}: {error:?}"));
        let result = crate::interp::run_scalar(&program);
        assert!(
            matches!(result.outcome, crate::interp::Outcome::Unit),
            "seed {seed}: {result:?}"
        );
    }
}

#[test]
fn semantic_cfg_sources_never_create_invalid_ir() {
    for seed in 0..30_000_u64 {
        let lhs = (seed % 5) as i64;
        let rhs = (seed / 5 % 5) as i64;
        let body = match seed / 25 % 12 {
            0 => format!("if true{{value:{lhs};}}else{{value:{rhs};}}"),
            1 => format!("let x=if true{{value:{lhs};}}else{{value:{rhs};}};"),
            2 => format!("var x={lhs};while x<{rhs}{{x=x+1;}}"),
            3 => format!("let r=mock_open({lhs});if true{{drop(r);}}inspect(r);"),
            4 => format!("let r=mock_open({lhs});while false{{drop(r);}}"),
            5 => format!("if true{{return;}}else{{return;}}let x={lhs};"),
            6 => format!("let x=if true{{return;}}else{{value:{lhs};}};"),
            7 => format!("if false{{let x={lhs};}}else if true{{let x={rhs};}}"),
            8 => format!("var x={lhs};while x<{rhs}{{if x<2{{x=x+1;}}else{{x=x+2;}}}}"),
            9 => "let x=if true{value: ();}else{value: ();};drop(x);".to_string(),
            10 => format!("var x={lhs};if true{{x={rhs};}}else{{x={lhs};}}drop(x);"),
            _ => format!("if true{{return;}}else{{while false{{let x={lhs};}}}}"),
        };
        let source = format!("fn main(){{{body}}}");
        match compile_scalar("fuzz.nx", &source) {
            Ok(_) | Err(CompileError::OutsideScalarSlice(_)) => {}
            Err(CompileError::Rejected(errors)) => {
                for diagnostic in errors {
                    diagnostic.validate("fuzz.nx", &source).unwrap();
                }
            }
            Err(other) => panic!("seed {seed}: {source}: {other:?}"),
        }
    }
}

#[test]
fn cfg_verifier_rejects_bad_edges_branch_type_and_missing_join_definition() {
    let source = "fn f(b:bool)->i64{return if b{value:1;}else{value:2;};} fn main(){}";
    let program = compile_scalar("verify_cfg.nx", source).unwrap();
    let original = program.functions.function("f").unwrap().clone();

    let mut invalid = original.clone();
    if let Terminator::BranchBool { no, .. } = &mut invalid.block.terminator {
        *no = BlockId(999);
    }
    invalid.content_hash = model::content_hash(&invalid);
    assert_eq!(
        super::verify::verify_function(&invalid),
        Err("invalid jump target")
    );

    let mut invalid = original.clone();
    if let Terminator::BranchBool { condition, .. } = &mut invalid.block.terminator {
        *condition = TempId(999);
    }
    invalid.content_hash = model::content_hash(&invalid);
    assert_eq!(
        super::verify::verify_function(&invalid),
        Err("branch condition type")
    );

    let mut invalid = original;
    let else_write = invalid
        .blocks
        .iter_mut()
        .flat_map(|b| &mut b.instructions)
        .find(|ins| matches!(ins.op, Op::WritePlace { .. }) && ins.span.start > 40)
        .unwrap();
    if let Op::WritePlace { value, .. } = else_write.op {
        else_write.op = Op::DropValue(value);
        else_write.access = model::Access::Own;
    }
    invalid.content_hash = model::content_hash(&invalid);
    assert_eq!(
        super::verify::verify_function(&invalid),
        Err("uninitialized local")
    );
}

#[test]
fn cfg_verifier_rejects_block_identity_exact_end_target_and_unreachable_arm() {
    let source = "fn f(b:bool)->i64{return if b{value:1;}else{value:2;};} fn main(){}";
    let original = compile_scalar("cfg_shape.nx", source)
        .unwrap()
        .functions
        .function("f")
        .unwrap()
        .clone();

    let mut bad = original.clone();
    bad.blocks[0].id = BlockId(100);
    bad.content_hash = model::content_hash(&bad);
    assert_eq!(super::verify::verify_function(&bad), Err("block identity"));

    let mut bad = original.clone();
    let past_end = BlockId(bad.blocks.len() + 1);
    if let Terminator::BranchBool { no, .. } = &mut bad.block.terminator {
        *no = past_end;
    }
    bad.content_hash = model::content_hash(&bad);
    assert_eq!(
        super::verify::verify_function(&bad),
        Err("invalid jump target")
    );

    let mut bad = original;
    if let Terminator::BranchBool { yes, no, .. } = &mut bad.block.terminator {
        *no = *yes;
    }
    bad.content_hash = model::content_hash(&bad);
    assert_eq!(
        super::verify::verify_function(&bad),
        Err("unreachable block")
    );
}

#[test]
fn cfg_verifier_revisits_loop_header_after_backedge_consumes_owner() {
    let source = "fn main(){var x=0;while false{x=x+1;}print(x);}";
    let mut bad = compile_scalar("backedge.nx", source)
        .unwrap()
        .functions
        .function("main")
        .unwrap()
        .clone();
    let body = bad
        .blocks
        .iter_mut()
        .find(|block| {
            matches!(block.terminator,
        Terminator::Jump { target, .. } if target.0 < block.id.0)
        })
        .unwrap();
    let span = body.instructions.last().unwrap().span.clone();
    let scope = body.instructions.last().unwrap().scope;
    body.instructions.push(model::Instruction {
        op: Op::DropLocal(model::LocalId(0)),
        output: None,
        ty: None,
        span,
        scope,
        origin: model::OriginId(body.instructions.len()),
        access: model::Access::Own,
        effect_io: false,
        tick: true,
    });
    bad.content_hash = model::content_hash(&bad);
    assert_eq!(
        super::verify::verify_function(&bad),
        Err("uninitialized local")
    );
}

#[test]
fn cfg_jump_to_entry_keeps_work_quota_active() {
    let source = "fn main(){while true{}}";
    let mut function = compile_scalar("entry_jump.nx", source)
        .unwrap()
        .functions
        .function("main")
        .unwrap()
        .clone();
    let backedge = function
        .blocks
        .iter_mut()
        .find(|block| {
            matches!(block.terminator,
        Terminator::Jump { target, .. } if target.0 < block.id.0)
        })
        .unwrap();
    if let Terminator::Jump { target, .. } = &mut backedge.terminator {
        *target = BlockId(0);
    }
    function.content_hash = model::content_hash(&function);
    assert_eq!(
        super::verify::verify_function(&function),
        Err("invalid control-flow cycle")
    );
}

#[test]
fn verifier_bounded_on_exhaustive_small_cfg_destinations() {
    let source = "fn main(){var a=[1,2];a[if true{value:0;}else{value:1;}]=if false{value:5;}else{value:6;};print(a);}";
    let original = compile_scalar("cyclic_cfg.nx", source)
        .unwrap()
        .functions
        .function("main")
        .unwrap()
        .clone();
    let blocks: Vec<_> = std::iter::once(&original.block)
        .chain(&original.blocks)
        .collect();
    let branch = blocks
        .iter()
        .position(|b| matches!(b.terminator, Terminator::BranchBool { .. }))
        .unwrap();
    let jumps: Vec<_> = blocks
        .iter()
        .enumerate()
        .filter(|(i, b)| *i > branch && matches!(b.terminator, Terminator::Jump { .. }))
        .map(|(i, _)| i)
        .take(2)
        .collect();
    assert_eq!(jumps.len(), 2);
    for &jump in &jumps {
        let mut cyclic = original.clone();
        let mut all: Vec<_> = std::iter::once(&mut cyclic.block)
            .chain(&mut cyclic.blocks)
            .collect();
        if let Terminator::Jump { target, .. } = &mut all[jump].terminator {
            *target = BlockId(branch);
        }
        cyclic.content_hash = model::content_hash(&cyclic);
        assert_eq!(
            super::verify::verify_function(&cyclic),
            Err("invalid control-flow cycle")
        );
    }
    for yes in 0..blocks.len() {
        for no in 0..blocks.len() {
            for first in 0..blocks.len() {
                for second in 0..blocks.len() {
                    let mut changed = original.clone();
                    let mut all: Vec<_> = std::iter::once(&mut changed.block)
                        .chain(&mut changed.blocks)
                        .collect();
                    if let Terminator::BranchBool { yes: y, no: n, .. } =
                        &mut all[branch].terminator
                    {
                        *y = BlockId(yes);
                        *n = BlockId(no);
                    }
                    for (index, dest) in jumps.iter().zip([first, second]) {
                        if let Terminator::Jump { target, .. } = &mut all[*index].terminator {
                            *target = BlockId(dest);
                        }
                    }
                    changed.content_hash = model::content_hash(&changed);
                    let (_, steps) =
                        super::measure_cfg_thread_work(|| super::verify::verify_function(&changed));
                    assert!(
                        steps <= 20_000,
                        "destinations {yes},{no},{first},{second}: {steps} steps"
                    );
                }
            }
        }
    }
}

#[test]
fn cfg_lowering_requires_bool_without_checker() {
    // Direct AST lowering checks its own invariant even when the semantic gate is bypassed.
    for source in [
        "fn main(){let x=if 1{value:2;}else{value:3;};}",
        "fn main(){if 1{}else{}}",
    ] {
        let ast = crate::frontend::parse::parse("bad_cond.nx", source).unwrap();
        assert!(matches!(
            super::lower::compile_ast(&ast, source),
            Err(CompileError::InvalidIr("if condition type"))
        ));
    }
}

#[test]
fn cfg_verifier_queue_avoids_duplicate_work_for_pending_blocks() {
    let source = "fn f(b:bool)->i64{return if b{value:1;}else{value:2;};} fn main(){}";
    let function = compile_scalar("queue.nx", source)
        .unwrap()
        .functions
        .function("f")
        .unwrap()
        .clone();
    let (result, steps) =
        super::measure_cfg_thread_work(|| super::verify::verify_function(&function));
    assert_eq!(result, Ok(()));
    assert!(steps <= 80, "duplicate CFG verification work: {steps}");
}

#[test]
fn mutated_cfg_sources_keep_rejections_structured() {
    let bases = [
        "fn main(){var x=0;if true{x=1;}else{x=2;}while x<3{x=x+1;}}",
        "fn f(b:bool)->i64{return if b{value:1;}else{value:2;};} fn main(){assert_eq(f(true),1);}",
        "fn main(){let r=mock_open(1);if true{inspect(r);}else{drop(r);}}",
        "fn main(){var x=0;while x<2{if x<1{x=x+1;}else{x=x+2;}}}",
    ];
    let replacement = *b" (){}:;01?+-";
    let mut state = 0x5a17_d001_u64;
    for seed in 0..20_000 {
        state = state.wrapping_mul(6364136223846793005).wrapping_add(1);
        let mut bytes = bases[seed % bases.len()].as_bytes().to_vec();
        let at = (state as usize) % bytes.len();
        bytes[at] = replacement[(state >> 32) as usize % replacement.len()];
        let source = String::from_utf8(bytes).unwrap();
        match compile_scalar("mutated_cfg.nx", &source) {
            Ok(_) | Err(CompileError::OutsideScalarSlice(_)) => {}
            Err(CompileError::Rejected(errors)) => {
                for diagnostic in errors {
                    diagnostic.validate("mutated_cfg.nx", &source).unwrap();
                }
            }
            Err(other) => panic!("seed {seed}: {source}: {other:?}"),
        }
    }
}

#[test]
fn directed_branch_error_and_divergence_fuzz_has_no_internal_failure() {
    let error_bases = [
        "fn main(){let v=(if false{();}else{value:5;});return;(if true{value:v;}else{value:1;});}",
        "fn main(){let v=if true{var w=(if false{value:4;}else{});value:(w==w);}else{value:true;};}",
        "fn main(){print(1+(if true{var w=if false{}else{value:2;};value:w;}else{value:3;}));}",
        "fn main(){var a=[1];a[0]=if true{let x=if false{value:1;}else{};value:x;}else{value:2;};}",
    ];
    let divergence_bases = [
        "fn main(){print(1+(if true{return;}else{return;}));}",
        "fn h(a:i64,b:i64){} fn main(){h(1,if true{return;}else{return;});}",
        "fn main(){var a=[1];a[0]=if true{return;}else{return;};}",
        "fn main(){if (if true{return;}else{return;}){print(1);}}",
    ];
    let replacements = *b" (){}:;01?+-";
    let mut state = 0x679a_07a3_u64;
    for seed in 0..40_000usize {
        let bases = if seed < 20_000 {
            &error_bases
        } else {
            &divergence_bases
        };
        state = state.wrapping_mul(6364136223846793005).wrapping_add(1);
        let mut bytes = bases[seed % bases.len()].as_bytes().to_vec();
        let at = (state as usize) % bytes.len();
        bytes[at] = replacements[(state >> 32) as usize % replacements.len()];
        let source = String::from_utf8(bytes).unwrap();
        match compile_scalar("branch_fuzz.nx", &source) {
            Ok(_) | Err(CompileError::OutsideScalarSlice(_)) => {}
            Err(CompileError::Rejected(errors)) => {
                for diagnostic in errors {
                    diagnostic.validate("branch_fuzz.nx", &source).unwrap();
                }
            }
            Err(other) => panic!("seed {seed}: {source}: {other:?}"),
        }
    }
}

#[test]
#[ignore = "explicit 256 KiB to 1 MiB CFG work measurement"]
fn cfg_dense_sources_have_bounded_deterministic_work() {
    use std::sync::atomic::Ordering;
    let patterns = [
        "if true{}else{}",
        "while false{}",
        "if true{let x=1;}else{let x=2;}",
        "{let x=if true{value:1;}else{value:2;};}",
        "{var x=0;while x<2{x=x+1;}}",
        "{let r=mock_open(1);if true{drop(r);}else{drop(r);}}",
        "{return;let a=1;if true{}else{}}",
        "{var x=0;if true{x=1;}else{x=2;}}",
        "{if true{let x=1;}else{let x=2;}}",
        "{var x=0;while false{if true{x=x+1;}else{x=x+2;}}}",
        "if true{}else if false{}else{}",
        "{let a=[1,2];if true{print(a[0]);}else{print(a[1]);}}",
        "{let r=mock_open(1);while false{inspect(r);}drop(r);}",
        "{let a=1;if true{}else{}}",
        "{let a=1;while false{print(a);}}",
        "{let a=1;if true{if false{print(a);}}else{print(a);}}",
        "{let a=if true{value:1;}else{value:2;};print(a);}",
        "{let a=if false{value:1;}else if true{value:2;}else{value:3;};}",
        "{let r=mock_open(1);if true{inspect(r);}else{inspect(r);}drop(r);}",
        "{let a=0;while false{let b=1;print(b);}}",
        "{let a=1;if true{let b=2;}else{let c=3;}}",
        "{var a=1;if true{a=2;}else{a=3;}while false{a=4;}}",
    ];
    for (family, pattern) in patterns.iter().enumerate() {
        if let Ok(only) = std::env::var("NEXO_CFG_SCALE_FAMILY")
            && only.parse::<usize>().ok() != Some(family)
        {
            continue;
        }
        let mut work = [0_u64; 2];
        let mut traffic = [(0_u64, 0_u64); 2];
        for (at, target) in [256 * 1024, 1024 * 1024].into_iter().enumerate() {
            let mut source = String::from("fn main(){");
            while source.len() + pattern.len() < target {
                source.push_str(pattern);
            }
            source.push('}');
            super::CFG_WORK.store(0, Ordering::Relaxed);
            crate::test_alloc::reset("cfg_scale_counter.nx");
            compile_scalar("cfg_scale_counter.nx", &source)
                .unwrap_or_else(|e| panic!("family {family}: {e:?}"));
            work[at] = super::CFG_WORK.load(Ordering::Relaxed);
            traffic[at] = crate::test_alloc::snapshot("cfg_scale_counter.nx");
        }
        let ratio = work[1] as f64 / work[0] as f64;
        eprintln!("cfg family {family}: {:?}, ratio {ratio:.3}", work);
        assert!(ratio <= 4.6, "family {family}: {work:?}");
        for metric in 0..2 {
            let small = if metric == 0 {
                traffic[0].0
            } else {
                traffic[0].1
            };
            let large = if metric == 0 {
                traffic[1].0
            } else {
                traffic[1].1
            };
            let ratio = large as f64 / small as f64;
            eprintln!("cfg family {family} metric {metric}: {small}, {large}, ratio {ratio:.3}");
            assert!(ratio <= 4.6, "family {family} metric {metric}: {traffic:?}");
        }
    }
}

#[test]
#[ignore = "explicit 256 KiB to 1 MiB later-operand CFG scale measurement"]
fn cfg_later_operands_scale_linearly_in_work_and_allocation() {
    use std::sync::atomic::Ordering;
    let families = [
        (
            "fn f(a:i64,b:i64){} fn main(){let b=true;",
            "f(1,if b{value:1;}else{value:2;});",
        ),
        (
            "fn main(){let b=true;",
            "print(1+(if b{value:1;}else{value:2;}));",
        ),
        (
            "fn main(){let b=true;var a=[1];",
            "a[0]=if b{value:1;}else{value:2;};",
        ),
        (
            "fn g(r:MockResource,n:i64){} fn main(){let b=true;let r=mock_open(1);",
            "g(r,if b{value:1;}else{value:2;});",
        ),
        (
            "fn main(){let b=true;",
            "print(1+(if b{value:if b{value:1;}else{value:2;};}else{value:3;}));",
        ),
        (
            "fn q(a:i64,b:i64,c:i64,d:i64){} fn main(){let b=true;",
            "q(1,2,3,if b{value:4;}else{value:5;});",
        ),
        (
            "fn main(){let b=true;var a=[1,2];",
            "print(a[if b{value:1;}else{value:0;}]);",
        ),
        (
            "fn main(){let b=true;",
            "print((if b{value:1;}else{value:2;})+(if b{value:3;}else{value:4;}));",
        ),
        (
            "fn many(a:i64,b:i64,c:i64,d:i64,e:i64,f:i64,g:i64,h:i64,i:i64){} fn main(){let b=true;",
            "many(1,2,3,4,5,6,7,8,if b{value:9;}else{value:10;});",
        ),
    ];
    for (family, (prefix, pattern)) in families.into_iter().enumerate() {
        if let Ok(only) = std::env::var("NEXO_CFG_SCALE_FAMILY")
            && only.parse::<usize>().ok() != Some(family)
        {
            continue;
        }
        let mut work = [0_u64; 2];
        let mut traffic = [(0_u64, 0_u64); 2];
        for (at, target) in [256 * 1024, 1024 * 1024].into_iter().enumerate() {
            let mut source = String::from(prefix);
            while source.len() + pattern.len() < target {
                source.push_str(pattern);
            }
            source.push('}');
            super::CFG_WORK.store(0, Ordering::Relaxed);
            crate::test_alloc::reset("cfg_scale_counter.nx");
            compile_scalar("cfg_scale_counter.nx", &source).unwrap();
            work[at] = super::CFG_WORK.load(Ordering::Relaxed);
            traffic[at] = crate::test_alloc::snapshot("cfg_scale_counter.nx");
        }
        let ratio = work[1] as f64 / work[0] as f64;
        eprintln!(
            "later operand family {family}: work {work:?}, traffic {traffic:?}, ratio {ratio:.3}"
        );
        assert!(ratio <= 4.6, "family {family}: work {work:?}");
        for metric in 0..2 {
            let small = if metric == 0 {
                traffic[0].0
            } else {
                traffic[0].1
            };
            let large = if metric == 0 {
                traffic[1].0
            } else {
                traffic[1].1
            };
            assert!(
                (large as f64 / small as f64) <= 4.6,
                "family {family} metric {metric}: {traffic:?}"
            );
        }
    }
}

#[test]
fn later_operand_dominance_verification_has_no_quadratic_allocation() {
    let mut traffic = [(0_u64, 0_u64); 2];
    for (at, target) in [64 * 1024, 256 * 1024].into_iter().enumerate() {
        let mut source = String::from("fn f(a:i64,b:i64){} fn main(){let b=true;");
        let pattern = "f(1,if b{value:1;}else{value:2;});";
        while source.len() + pattern.len() < target {
            source.push_str(pattern);
        }
        source.push('}');
        crate::test_alloc::reset("later_operand_scale_counter.nx");
        compile_scalar("later_operand_scale_counter.nx", &source).unwrap();
        traffic[at] = crate::test_alloc::snapshot("later_operand_scale_counter.nx");
    }
    for metric in 0..2 {
        let small = if metric == 0 {
            traffic[0].0
        } else {
            traffic[0].1
        };
        let large = if metric == 0 {
            traffic[1].0
        } else {
            traffic[1].1
        };
        assert!(
            (large as f64 / small as f64) <= 4.6,
            "quadratic dominance traffic metric {metric}: {traffic:?}"
        );
    }
}

#[test]
fn cfg_local_branch_scale_uses_sparse_temp_counters() {
    let mut traffic = [(0_u64, 0_u64); 2];
    for (at, target) in [64 * 1024, 256 * 1024].into_iter().enumerate() {
        let mut source = String::from("fn main(){let b=true;");
        let mut id = 0;
        while source.len() + 64 < target {
            source.push_str(&format!("let v{id}=if b{{value:1;}}else{{value:2;}};"));
            id += 1;
        }
        source.push('}');
        crate::test_alloc::reset("cfg_scale_counter.nx");
        compile_scalar("cfg_scale_counter.nx", &source).unwrap();
        traffic[at] = crate::test_alloc::snapshot("cfg_scale_counter.nx");
    }
    for metric in 0..2 {
        let small = if metric == 0 {
            traffic[0].0
        } else {
            traffic[0].1
        };
        let large = if metric == 0 {
            traffic[1].0
        } else {
            traffic[1].1
        };
        assert!(
            (large as f64 / small as f64) <= 4.6,
            "metric {metric}: {traffic:?}"
        );
    }
}

#[test]
fn nested_cfg_compiles_and_runs_from_small_caller_stacks() {
    for stack in [128, 256, 512].map(|kib| kib * 1024) {
        std::thread::Builder::new()
            .stack_size(stack)
            .spawn(|| {
                for opening in ["if true{", "while false{"] {
                    let source = format!("fn main(){{{}{}}}", opening.repeat(80), "}".repeat(80),);
                    let program = compile_scalar("cfg_stack.nx", &source).unwrap();
                    let execution = crate::interp::run_scalar(&program);
                    assert!(matches!(execution.outcome, crate::interp::Outcome::Unit));
                }
            })
            .unwrap()
            .join()
            .unwrap();
    }
}

#[test]
fn dead_code_after_all_returning_branch_does_not_reuse_live_temp_ids() {
    let source = "fn g(x:i64)->i64{return x;} fn main(){{return;g(g(1));}if true{return;}else{return;}g(g(1));}";
    let program = compile_scalar("dead_cfg.nx", source).unwrap();
    assert!(matches!(
        crate::interp::run_scalar(&program).outcome,
        crate::interp::Outcome::Unit
    ));
}

#[test]
fn cfg_verifier_rejects_cross_block_temp_duplicate_and_wrong_join_type() {
    let source = "fn f(b:bool)->i64{return if b{value:1;}else{value:2;};} fn main(){}";
    let original = compile_scalar("cfg_mut.nx", source)
        .unwrap()
        .functions
        .function("f")
        .unwrap()
        .clone();
    let mut bad = original.clone();
    let sibling_temp = bad
        .blocks
        .iter()
        .flat_map(|b| &b.instructions)
        .find(|ins| matches!(ins.op, Op::ConstI64(1)))
        .unwrap()
        .output
        .unwrap();
    let exit = bad.blocks.last_mut().unwrap();
    if let Terminator::Return { value, .. } = &mut exit.terminator {
        *value = Some(sibling_temp);
    }
    bad.content_hash = model::content_hash(&bad);
    assert_eq!(
        super::verify::verify_function(&bad),
        Err("undefined return temporary")
    );

    let mut bad = original.clone();
    let mut seen = false;
    for ins in bad.blocks.iter_mut().flat_map(|b| &mut b.instructions) {
        if matches!(ins.op, Op::ConstI64(_)) {
            if seen {
                ins.output = Some(sibling_temp);
                break;
            }
            seen = true;
        }
    }
    bad.content_hash = model::content_hash(&bad);
    assert_eq!(
        super::verify::verify_function(&bad),
        Err("temporary identity")
    );

    let mut bad = original;
    let slot = bad
        .locals
        .iter_mut()
        .find(|local| local.name.is_empty())
        .unwrap();
    slot.ty = model::Ty::Bool;
    bad.content_hash = model::content_hash(&bad);
    assert_eq!(super::verify::verify_function(&bad), Err("write type"));
}

#[test]
fn cfg_verifier_rejects_loan_metadata_and_unreleased_loan() {
    let source = "fn f(inout x:i64){x=x+1;} fn main(){var a=0;if true{f(inout a);}else{}}";
    let original = compile_scalar("cfg_loan.nx", source)
        .unwrap()
        .functions
        .function("main")
        .unwrap()
        .clone();
    let mut bad = original.clone();
    let loan = bad
        .blocks
        .iter_mut()
        .flat_map(|b| &mut b.instructions)
        .find(|ins| matches!(ins.op, Op::BeginLoan { .. }))
        .unwrap();
    if let Op::BeginLoan { parent, .. } = &mut loan.op {
        *parent = Some(model::LocalId(0));
    }
    bad.content_hash = model::content_hash(&bad);
    assert_eq!(super::verify::verify_function(&bad), Err("loan parent"));

    let mut bad = original.clone();
    let end = bad
        .blocks
        .iter_mut()
        .flat_map(|b| &mut b.instructions)
        .find(|ins| matches!(ins.op, Op::EndLoan(_)))
        .unwrap();
    end.op = Op::EndLoan(model::LoanId(999));
    bad.content_hash = model::content_hash(&bad);
    assert_eq!(
        super::verify::verify_function(&bad),
        Err("unmatched loan end")
    );

    let mut bad = original;
    let loan = bad
        .blocks
        .iter_mut()
        .flat_map(|b| &mut b.instructions)
        .find(|ins| matches!(ins.op, Op::BeginLoan { .. }))
        .unwrap();
    if let Op::BeginLoan { permission, .. } = &mut loan.op {
        *permission = model::LoanPermission::Read;
    }
    bad.content_hash = model::content_hash(&bad);
    assert_eq!(
        super::verify::verify_function(&bad),
        Err("inout argument permission")
    );
}

#[test]
fn published_branch_structure_diagnostics_identify_exact_marker_and_context() {
    use crate::diagnostic::{Operation, RelatedRole, Stage};
    // F0-0002 §14.3 B01 and B02: markers are `if` and the closing brace.
    for (source, subcause, stage, operation, primary) in [
        (
            "fn f(b:bool)->i64{return if b{value:1;};} fn main(){}",
            "missing_else",
            Stage::Parse,
            Operation::SyntaxParse,
            (25, 27),
        ),
        (
            "fn f()->i64{return if true{1;}else{value:2;};} fn main(){}",
            "missing_branch_value",
            Stage::Ownership,
            Operation::Branch,
            (29, 30),
        ),
    ] {
        let CompileError::Rejected(errors) = compile_scalar("branch.nx", source).unwrap_err()
        else {
            panic!("expected structural rejection")
        };
        assert_eq!(errors.len(), 1);
        let d = &errors[0];
        assert_eq!(
            (d.code.as_str(), d.subcause.as_deref()),
            ("NX-E1302", Some(subcause))
        );
        assert_eq!((d.stage, d.operation), (stage, operation));
        assert_eq!(
            d.primary_span.as_ref().map(|s| (s.start, s.end)),
            Some(primary)
        );
        if subcause == "missing_branch_value" {
            assert_eq!(d.related_spans.len(), 1);
            assert_eq!(d.related_spans[0].role, RelatedRole::Context);
        }
        assert!(d.data.is_empty());
        assert!(d.unexpected_token.is_none());
        d.validate("branch.nx", source).unwrap();
    }
}
#[test]
fn divergent_bindings_allow_unreachable_assignments_and_indexes() {
    let sources = [
        "fn main(){var v:i64=if true{return;}else{return;};v=5;}",
        "fn main(){var v=1+(if true{return;}else{return;});v=5;}",
        "fn g(a:i64,b:i64)->i64{return a+b;} fn main(){var v=g(1,if true{return;}else{return;});v=5;}",
        "fn main(){var w:bool=if true{return;}else{return;};w=true;}",
        "fn main(){var v=[1,if true{return;}else{return;}];v[0]=1;}",
    ];
    for source in sources {
        assert!(
            compile_scalar("dead_binding.nx", source).is_ok(),
            "{source}"
        );
    }
}

#[test]
fn divergent_left_operand_still_checks_later_branch_and_ownership() {
    for (source, code, subcause) in [
        (
            "fn main(){print((if true{return;}else{return;})+(if true{1;}else{value:2;}));}",
            "NX-E1302",
            "missing_branch_value",
        ),
        (
            "fn main(){let r=mock_open(1);drop(r);print((if true{return;}else{return;})+(if true{inspect(r);value:1;}else{value:2;}));}",
            "NX-E0202",
            "definitely_consumed",
        ),
    ] {
        let Err(CompileError::Rejected(errors)) = compile_scalar("dead_operand.nx", source) else {
            panic!("expected {code}/{subcause}: {source}");
        };
        assert!(
            errors
                .iter()
                .any(|d| d.code == code && d.subcause.as_deref() == Some(subcause)),
            "{source}: {errors:?}"
        );
    }
}

#[test]
fn branch_structure_precedes_type_mismatch_with_divergent_operand() {
    for source in [
        "fn main(){print((if true{return;}else{})+1);}",
        "fn main(){print(1+(if true{return;}else{}));}",
    ] {
        let Err(CompileError::Rejected(errors)) = compile_scalar("dead_structure.nx", source)
        else {
            panic!("expected missing branch value: {source}");
        };
        assert_eq!(errors.len(), 1, "{source}: {errors:?}");
        assert_eq!(
            (errors[0].code.as_str(), errors[0].subcause.as_deref()),
            ("NX-E1302", Some("missing_branch_value")),
            "{source}"
        );
        let closing = source.rfind("else{}").unwrap() + "else{".len();
        assert_eq!(
            errors[0].primary_span.as_ref().map(|s| (s.start, s.end)),
            Some((closing as u64, closing as u64 + 1))
        );
        assert!(errors[0].data.is_empty());
        assert!(errors[0].unexpected_token.is_none());
        errors[0].validate("dead_structure.nx", source).unwrap();
    }
}

#[test]
fn divergent_condition_checks_unreachable_branch_ownership() {
    let source =
        "fn main(){if (if true{return;}else{return;}){let r=mock_open(1);drop(r);inspect(r);}}";
    let Err(CompileError::Rejected(errors)) = compile_scalar("dead_condition.nx", source) else {
        panic!("expected consumed resource diagnostic");
    };
    assert!(
        errors
            .iter()
            .any(|d| d.code == "NX-E0202" && d.subcause.as_deref() == Some("definitely_consumed")),
        "{errors:?}"
    );
}

#[test]
fn later_divergent_arguments_preserve_branch_value_diagnostics() {
    let sources = [
        "fn k3(x:i64,y:i64,z:i64)->i64{return x+z;} fn main(){let v=k3(1,if false{value:1;}else if false{print(6);}else{value:7;},(if false{return;}else{return;}));}",
        "fn k3(x:i64,y:i64,z:i64)->i64{return x+z;} fn main(){k3(if(if true{let q=if true{return;}else{};return;}else{return;}){value:6;}else{value:4;},(if true{return;}else{return;})+(),1);}",
    ];
    for source in sources {
        let outcome = compile_scalar("dead_args.nx", source);
        let Err(CompileError::Rejected(errors)) = outcome else {
            panic!("expected missing branch value: {source}: {outcome:?}");
        };
        assert_eq!(errors.len(), 1, "{source}: {errors:?}");
        assert!(
            errors
                .iter()
                .any(|d| d.code == "NX-E1302"
                    && d.subcause.as_deref() == Some("missing_branch_value")),
            "{source}: {errors:?}"
        );
        let closing = if let Some(at) = source.find("print(6);}") {
            at + "print(6);".len()
        } else {
            source.find("else{}").unwrap() + "else{".len()
        };
        assert_eq!(
            errors[0].primary_span.as_ref().map(|s| (s.start, s.end)),
            Some((closing as u64, closing as u64 + 1)),
            "{source}"
        );
        assert!(errors[0].data.is_empty());
        assert!(errors[0].unexpected_token.is_none());
        errors[0].validate("dead_args.nx", source).unwrap();
    }
}

#[test]
fn divergent_left_operands_reject_unit_with_numeric_expected_type() {
    let cases = [
        "fn main(){let v=(if true{return;}else{return;})+();}",
        "fn main(){let v=(if true{return;}else{return;})-();}",
        "fn main(){let v=if true{return;}else{return;};let w=v+();}",
    ];
    for source in cases {
        let Err(CompileError::Rejected(errors)) = compile_scalar("divergent_unit.nx", source)
        else {
            panic!("expected type mismatch: {source}");
        };
        assert_eq!(errors.len(), 1, "{source}: {errors:?}");
        let d = &errors[0];
        assert_eq!(d.code, "NX-E0102", "{source}");
        assert_eq!(d.subcause.as_deref(), Some("type_mismatch"), "{source}");
        assert_eq!(d.stage, crate::diagnostic::Stage::Types, "{source}");
        assert_eq!(
            d.operation,
            crate::diagnostic::Operation::TypesCheck,
            "{source}"
        );
        assert_eq!(
            d.data.get("expected_type"),
            Some(&crate::diagnostic::DataValue::String("i64".into())),
            "{source}"
        );
        assert_eq!(
            d.data.get("found_type"),
            Some(&crate::diagnostic::DataValue::String("()".into())),
            "{source}"
        );
        assert_eq!(d.data.len(), 2, "{source}");
        assert!(
            d.unexpected_token.is_none() && d.expected_tokens.is_empty(),
            "{source}"
        );
        let unit = source.rfind("()").unwrap() as u64;
        assert_eq!(
            d.primary_span.as_ref().map(|s| (s.start, s.end)),
            Some((unit, unit + 2)),
            "{source}"
        );
        assert!(
            d.related_spans
                .iter()
                .any(|r| r.role == crate::diagnostic::RelatedRole::FoundType
                    && r.span.start == unit
                    && r.span.end == unit + 2),
            "{source}"
        );
        d.validate("divergent_unit.nx", source).unwrap();
    }
}

#[test]
fn deferred_numeric_operands_pass_expected_type_into_branch_values() {
    for op in ["+", "<"] {
        for left in ["v", "(if true{return;}else{return;})"] {
            let source = format!(
                "fn main(){{let v=if true{{return;}}else{{return;}};let w={left}{op}(if true{{value:();}}else{{return;}});}}"
            );
            let Err(CompileError::Rejected(errors)) = compile_scalar("numeric_branch.nx", &source)
            else {
                panic!("expected mismatch: {source}");
            };
            assert_eq!(errors.len(), 1, "{source}: {errors:?}");
            let d = &errors[0];
            let unit = source.find("value:();").unwrap() as u64 + "value:".len() as u64;
            assert_eq!(d.code, "NX-E0102", "{source}");
            assert_eq!(d.subcause.as_deref(), Some("type_mismatch"), "{source}");
            assert_eq!(
                d.primary_span.as_ref().map(|s| (s.start, s.end)),
                Some((unit, unit + 2)),
                "{source}"
            );
            assert_eq!(
                d.data.get("expected_type"),
                Some(&crate::diagnostic::DataValue::String("i64".into())),
                "{source}"
            );
            assert_eq!(
                d.data.get("found_type"),
                Some(&crate::diagnostic::DataValue::String("()".into())),
                "{source}"
            );
            d.validate("numeric_branch.nx", &source).unwrap();
        }
    }
}

#[test]
fn wholly_deferred_array_keeps_its_contextual_bottom_type() {
    let source = "fn main(){let v=if true{return;}else{return;};let a=[v];let w=a+1;}";
    let program = compile_scalar("deferred_array.nx", source).unwrap();
    assert!(
        crate::interp::run_scalar(&program)
            .observations
            .stdout()
            .is_empty()
    );
}

#[test]
fn numeric_lowering_rejects_non_numeric_left_without_checker() {
    let source = "fn main(){let a=[1];let v=a+1;}";
    let ast = crate::frontend::parse::parse("numeric_left.nx", source).unwrap();
    let Err(CompileError::Rejected(errors)) = super::lower::compile_ast(&ast, source) else {
        panic!("lowering must preserve the numeric operand rule");
    };
    assert_eq!(errors.len(), 1);
    let d = &errors[0];
    let at = source.find("a+1").unwrap() as u64;
    assert_eq!(d.code, "NX-E0102");
    assert_eq!(d.subcause.as_deref(), Some("type_mismatch"));
    assert_eq!(
        d.primary_span.as_ref().map(|s| (s.start, s.end)),
        Some((at, at + 1))
    );
    assert_eq!(
        d.data.get("expected_type"),
        Some(&crate::diagnostic::DataValue::String("i64".into()))
    );
    assert_eq!(
        d.data.get("found_type"),
        Some(&crate::diagnostic::DataValue::String("Array<i64>".into()))
    );
    d.validate("numeric_left.nx", source).unwrap();
}

#[test]
fn adr_gap_never_is_bottom_for_equality_and_array_inference() {
    let dv = "(if true{return;}else{return;})";
    for source in [
        format!("fn main(){{let x={dv}==true;}}"),
        format!("fn main(){{let x={dv}==(if true{{value:true;}}else{{value:false;}});}}"),
        format!("fn main(){{let x=true=={dv};}}"),
        format!("fn main(){{let x=[{dv},false];}}"),
        format!("fn main(){{let x=[false,{dv}];}}"),
        format!("fn main(){{let x=[{dv},true,true];}}"),
        format!("fn main(){{let x=[true,{dv},true];}}"),
        format!("fn main(){{let x={dv}==();}}"),
        format!("fn main(){{let x=()=={dv};}}"),
    ] {
        compile_scalar("never_bottom.nx", &source).unwrap_or_else(|e| panic!("{source}: {e:?}"));
    }
    let source = format!("fn main(){{let x=[{dv},true,1];}}");
    let Err(CompileError::Rejected(errors)) = compile_scalar("never_bottom.nx", &source) else {
        panic!("expected array mismatch: {source}");
    };
    assert_eq!(errors[0].code, "NX-E0102");
    let at = source.rfind('1').unwrap() as u64;
    assert_eq!(
        errors[0].primary_span.as_ref().map(|s| (s.start, s.end)),
        Some((at, at + 1))
    );
    let source = format!("fn main(){{let x=[{dv},1];}}");
    let program = compile_scalar("never_bottom.nx", &source).unwrap();
    assert_eq!(
        program.functions.function("main").unwrap().locals[0].ty,
        model::Ty::Array(Box::new(model::Ty::I64))
    );
    let source = "fn main(){let x:Array<i64>=[];}";
    let program = compile_scalar("never_bottom.nx", source).unwrap();
    assert_eq!(
        program.functions.function("main").unwrap().locals[0].ty,
        model::Ty::Array(Box::new(model::Ty::I64))
    );
}

#[test]
fn deferred_var_assignment_accepts_value_branches_and_fixes_type_once() {
    let dv = "if true{return;}else{return;}";
    for (prefix, rhs) in [
        ("", "3"),
        ("", "1+2"),
        ("", "if true{value:3;}else{value:7;}"),
        ("let a=[3];", "a[0]"),
        ("", "f()"),
    ] {
        let source = format!("fn f()->i64{{return 3;}} fn main(){{{prefix}var v={dv};v={rhs};}}");
        compile_scalar("deferred_assignment.nx", &source)
            .unwrap_or_else(|e| panic!("{source}: {e:?}"));
    }
    for body in ["if true{v=3;}else{v=7;}", "while false{v=3;}", "{v=3;}"] {
        let source = format!("fn main(){{var v={dv};{body}}}");
        compile_scalar("deferred_assignment.nx", &source)
            .unwrap_or_else(|e| panic!("{source}: {e:?}"));
    }
    let source = format!("fn main(){{var v={dv};v=if true{{value:3;}}else{{value:7;}};v=true;}}");
    let Err(CompileError::Rejected(errors)) = compile_scalar("deferred_assignment.nx", &source)
    else {
        panic!("expected later type mismatch");
    };
    assert_eq!(errors[0].code, "NX-E0102");
    assert_eq!(
        errors[0].data.get("expected_type"),
        Some(&crate::diagnostic::DataValue::String("i64".into()))
    );
    assert_eq!(
        errors[0].data.get("found_type"),
        Some(&crate::diagnostic::DataValue::String("bool".into()))
    );
    let at = source.rfind("true;").unwrap() as u64;
    assert_eq!(
        errors[0].primary_span.as_ref().map(|s| (s.start, s.end)),
        Some((at, at + 4))
    );
    assert_eq!(errors[0].subcause.as_deref(), Some("type_mismatch"));
    errors[0]
        .validate("deferred_assignment.nx", &source)
        .unwrap();
    let source = format!("fn main(){{var v={dv};let w=v;v=3;let z:i64=w;}}");
    compile_scalar("deferred_assignment.nx", &source)
        .expect("reading before the first assignment preserves the deferred type");
    let source = format!("fn main(){{var v={dv};v=if true{{return;}}else{{return;}};v=true;}}");
    compile_scalar("deferred_assignment.nx", &source)
        .expect("a diverging assignment does not fix the deferred type");
}

#[test]
fn analyzed_binary_assignment_records_intrinsic_result_type() {
    let dv = "if true{return;}else{return;}";
    for (rhs, ty) in [("1+2", model::Ty::I64), ("1<2", model::Ty::Bool)] {
        let source = format!("fn main(){{var v={dv};v={rhs};}}");
        let program = compile_scalar("analyzed_binary.nx", &source).unwrap();
        assert_eq!(
            program.functions.function("main").unwrap().locals[0].ty,
            ty,
            "{source}"
        );
    }
}

#[test]
fn nested_binary_operand_error_precedes_enclosing_type_error() {
    let dv = "(if true{return;}else{return;})";
    for source in [
        format!("fn main(){{let w=((({dv})-())-());}}"),
        format!("fn h(x:bool){{}} fn main(){{h(({dv}-()));}}"),
    ] {
        let Err(CompileError::Rejected(errors)) = compile_scalar("nested_operand.nx", &source)
        else {
            panic!("expected operand mismatch: {source}");
        };
        assert_eq!(errors.len(), 1, "{source}: {errors:?}");
        let first_unit = source.find("-()").unwrap() as u64 + 1;
        assert_eq!(
            errors[0].primary_span.as_ref().map(|s| (s.start, s.end)),
            Some((first_unit, first_unit + 2)),
            "{source}"
        );
        assert_eq!(errors[0].code, "NX-E0102");
    }
}

#[test]
fn independent_right_operand_errors_survive_divergence_and_other_errors() {
    let dv = "(if true{return;}else{return;})";
    let templates = [
        "fn main(){let a=@+();let b:i64=();}",
        "fn main(){let b:i64=();let a=@+();}",
        "fn g(x:i64){} fn main(){g(@+());g(());}",
        "fn g(x:i64){} fn main(){g(());g(@+());}",
        "fn k(x:i64,y:i64){} fn g(x:i64){} fn main(){k(@+(),g(()));}",
        "fn g(x:i64){} fn main(){let a=[g(@+()),g(())];}",
        "fn g(x:i64){} fn main(){let a=g(@+())+g(());}",
        "fn g(x:i64){} fn main(){let a=g(())+g(@+());}",
        "fn g(x:i64){} fn main(){let a=(@+())-g(());}",
        "fn g(x:i64){} fn main(){let a=g(())-(@+());}",
        "fn h(){let a=@+();} fn main(){let b:i64=();}",
        "fn main(){let a=@+();let b:i64=();let c:i64=();let d:i64=();}",
    ];
    for (index, template) in templates.into_iter().enumerate() {
        let divergent = template.replacen('@', dv, 1);
        let live = template.replacen('@', "0", 1);
        let Err(CompileError::Rejected(dead)) = compile_scalar("independent.nx", &divergent) else {
            panic!("expected rejection: {divergent}");
        };
        let Err(CompileError::Rejected(alive)) = compile_scalar("independent.nx", &live) else {
            panic!("expected live rejection: {live}");
        };
        let origin = template.find('@').unwrap() as u64;
        let map = |offset: u64| {
            let end = origin + dv.len() as u64;
            if offset <= origin {
                offset
            } else if offset >= end {
                offset - (dv.len() - 1) as u64
            } else {
                origin + (offset - origin).min(1)
            }
        };
        assert_eq!(dead.len(), alive.len(), "{divergent}");
        assert_eq!(
            dead.len(),
            if index == 11 { 4 } else { 2 },
            "{divergent}: {dead:?}"
        );
        for (a, b) in dead.iter().zip(alive.iter()) {
            assert_eq!(
                (&a.code, &a.subcause, a.stage, a.operation, &a.data),
                (&b.code, &b.subcause, b.stage, b.operation, &b.data),
                "{divergent}"
            );
            assert_eq!(
                a.primary_span.as_ref().map(|s| (map(s.start), map(s.end))),
                b.primary_span.as_ref().map(|s| (s.start, s.end)),
                "{divergent}"
            );
        }
    }
}

#[test]
fn deferred_left_equality_value_branch_keeps_never_internal() {
    let branch = "(if false{value:3;}else{value:0;})";
    for body in [
        format!("if v=={branch}{{print(1);}}"),
        format!("while v=={branch}{{print(1);}}"),
        format!("let b=v=={branch};"),
    ] {
        let source = format!("fn main(){{let v=if true{{return;}}else{{return;}};{body}}}");
        let result = compile_scalar("deferred_equal.nx", &source);
        assert!(result.is_ok(), "{source}: {result:?}");
    }
}

#[test]
fn numeric_lowering_expects_branch_value_at_its_own_span() {
    let source =
        "fn main(){let v=if true{return;}else{return;};let w=v+(if true{value:();}else{return;});}";
    let ast = crate::frontend::parse::parse("numeric_expected.nx", source).unwrap();
    let Err(CompileError::Rejected(errors)) = super::lower::compile_ast(&ast, source) else {
        panic!("expected mismatch");
    };
    assert_eq!(errors.len(), 1, "{errors:?}");
    let unit = source.find("value:();").unwrap() as u64 + "value:".len() as u64;
    assert_eq!(errors[0].code, "NX-E0102");
    assert_eq!(
        errors[0].primary_span.as_ref().map(|s| (s.start, s.end)),
        Some((unit, unit + 2))
    );
    assert_eq!(
        errors[0].data.get("expected_type"),
        Some(&crate::diagnostic::DataValue::String("i64".into()))
    );
    assert_eq!(
        errors[0].data.get("found_type"),
        Some(&crate::diagnostic::DataValue::String("()".into()))
    );
}

#[test]
fn long_value_branch_chain_reports_nominal_lowering_limit() {
    let term = "(if true{value:1;}else{value:2;})";
    let source = format!("fn main(){{let x={};}}", vec![term; 127].join("+"));
    assert!(matches!(
        compile_scalar("long_branch_chain.nx", &source),
        Err(CompileError::OutsideScalarSlice("lowering depth"))
    ));
}

#[test]
fn adr_gap_deferred_var_aliases_do_not_unify_types() {
    let dv = "if true{return;}else{return;}";
    for body in [
        format!("var v={dv};var w=v;v=3;w=true;"),
        format!("var v={dv};var w=true;w=v;v=3;"),
    ] {
        let source = format!("fn main(){{{body}}}");
        compile_scalar("deferred_alias.nx", &source)
            .unwrap_or_else(|error| panic!("{source}: {error:?}"));
    }
}

#[test]
#[ignore = "explicit 300000-program differential divergence campaign"]
fn injected_divergence_matches_live_twin_diagnostics() {
    let templates: &[(&str, &str, bool)] = &[
        ("fn main(){let seed=SEED;let v=@+();}", "1", false),
        ("fn main(){let seed=SEED;let v=@==();}", "()", false),
        ("fn main(){let seed=SEED;let v=@<();}", "1", false),
        ("fn main(){let seed=SEED;let v=[@,()];}", "()", false),
        ("fn main(){let seed=SEED;let v=@;let w=v+();}", "1", false),
        ("fn main(){let seed=SEED;var v=@;let w=v==();}", "()", false),
        (
            "fn main(){let seed=SEED;let v=@;let w=[v,()];}",
            "()",
            false,
        ),
        ("fn main(){let seed=SEED;print(@+seed);}", "1", false),
        ("fn main(){let seed=SEED;print(seed+@);}", "1", false),
        ("fn main(){let seed=SEED;print(seed<@);}", "1", false),
        (
            "fn main(){let seed=SEED;let a=[seed];print(a[@]);}",
            "0",
            false,
        ),
        ("fn main(){let seed=SEED;var a=[seed];a[@]=2;}", "0", false),
        ("fn main(){let seed=SEED;var a=seed;a=@;}", "1", false),
        (
            "fn main(){let seed=SEED;if @ {print(seed);}}",
            "true",
            false,
        ),
        (
            "fn main(){let seed=SEED;if @ {print(seed);}else{print(2);}}",
            "true",
            false,
        ),
        (
            "fn main(){let seed=SEED;while @ {print(seed);}}",
            "false",
            false,
        ),
        (
            "fn main(){let seed=SEED;let v=if @ {value:seed;}else{value:2;};}",
            "true",
            false,
        ),
        (
            "fn main(){let seed=SEED;let v=if true{value:@;}else{value:seed;};}",
            "1",
            false,
        ),
        (
            "fn main(){let seed=SEED;let v=if true{value:seed;}else{value:@;};}",
            "1",
            false,
        ),
        ("fn main(){let seed=SEED;let v=[@,seed];}", "1", false),
        ("fn h(x:i64){} fn main(){let seed=SEED;h(@);}", "1", false),
        (
            "fn h(x:i64,y:i64){} fn main(){let seed=SEED;h(seed,@);}",
            "1",
            false,
        ),
        (
            "fn h(x:i64,y:i64){} fn main(){let seed=SEED;h(@,());}",
            "1",
            false,
        ),
        (
            "fn main(){let seed=SEED;let v=@;print(v==seed);}",
            "1",
            false,
        ),
        (
            "fn main(){let seed=SEED;let v=@;let w=v;print(w+seed);}",
            "1",
            false,
        ),
        (
            "fn main(){let seed=SEED;let v=@;let w=if true{value:v;}else{value:v;};}",
            "1",
            false,
        ),
        (
            "fn main(){let seed=SEED;print(@+(if true{print(2);}else{value:seed;}));}",
            "1",
            false,
        ),
        (
            "fn main(){let seed=SEED;return;let v=@;print(seed);}",
            "1",
            true,
        ),
        ("fn main(){let seed=SEED;return;print(@+seed);}", "1", true),
        ("fn main(){let seed=SEED;return;let v=@;}", "1", true),
        (
            "fn main(){let seed=SEED;let v=@;if v{print(seed);}}",
            "true",
            false,
        ),
        (
            "fn main(){let seed=SEED;let v=@;while v{print(seed);}}",
            "false",
            false,
        ),
        (
            "fn main(){let seed=SEED;let v=@;let a=[seed];print(a[v]);}",
            "0",
            false,
        ),
        (
            "fn h(x:i64){} fn main(){let seed=SEED;let v=@;h(v);}",
            "1",
            false,
        ),
        (
            "fn h(x:i64){} fn main(){let seed=SEED;let v=@;let a=[seed];h(a[v]);}",
            "0",
            false,
        ),
        (
            "fn main(){let seed=SEED;let v=@;var x=seed;x=v;}",
            "1",
            false,
        ),
        (
            "fn main(){let seed=SEED;let v=@;let w=[v,seed];}",
            "1",
            false,
        ),
        ("fn main(){let seed=SEED;var v=@;v=seed;}", "1", false),
        (
            "fn main(){let seed=SEED;let v=@;print(v<seed);}",
            "1",
            false,
        ),
        ("fn u(){} fn main(){let seed=SEED;let v=@+u();}", "1", false),
        (
            "fn main(){let seed=SEED;var v=@;v=if true{value:3;}else{value:7;};}",
            "1",
            false,
        ),
        ("fn main(){let seed=SEED;var v=@;v=1+2;}", "1", false),
        ("fn main(){let seed=SEED;var v=@;v=true;}", "true", false),
        (
            "fn main(){let seed=SEED;var v=@;v=false==true;}",
            "true",
            false,
        ),
        ("fn main(){let seed=SEED;let w=((@-())-());}", "0", false),
        (
            "fn h(x:bool){} fn main(){let seed=SEED;h((@-()));}",
            "0",
            false,
        ),
        (
            "fn main(){let seed=SEED;let w=(((@-())-())-());}",
            "0",
            false,
        ),
        ("fn main(){let seed=SEED;let w=@==true;}", "true", false),
        ("fn main(){let seed=SEED;let w=true==@;}", "true", false),
        ("fn main(){let seed=SEED;let w=[@,false];}", "false", false),
        ("fn main(){let seed=SEED;let w=[false,@];}", "false", false),
        (
            "fn main(){let seed=SEED;let w=[@,true,true];}",
            "true",
            false,
        ),
        (
            "fn main(){let seed=SEED;let w=[true,@,true];}",
            "true",
            false,
        ),
        ("fn main(){let seed=SEED;let w=[@,true,1];}", "true", false),
        ("fn main(){let seed=SEED;let w=()==@;}", "()", false),
        (
            "fn main(){let seed=SEED;let a=@+();let b:i64=();}",
            "0",
            false,
        ),
        (
            "fn main(){let seed=SEED;let b:i64=();let a=@+();}",
            "0",
            false,
        ),
        (
            "fn g(x:i64){} fn main(){let seed=SEED;g(@+());g(());}",
            "0",
            false,
        ),
        (
            "fn g(x:i64){} fn main(){let seed=SEED;g(());g(@+());}",
            "0",
            false,
        ),
        (
            "fn k(x:i64,y:i64){} fn g(x:i64){} fn main(){let seed=SEED;k(@+(),g(()));}",
            "0",
            false,
        ),
        (
            "fn g(x:i64){} fn main(){let seed=SEED;let a=[g(@+()),g(())];}",
            "0",
            false,
        ),
        (
            "fn g(x:i64){} fn main(){let seed=SEED;let a=g(@+())+g(());}",
            "0",
            false,
        ),
        (
            "fn g(x:i64){} fn main(){let seed=SEED;let a=g(())+g(@+());}",
            "0",
            false,
        ),
        (
            "fn g(x:i64){} fn main(){let seed=SEED;let a=(@+())-g(());}",
            "0",
            false,
        ),
        (
            "fn g(x:i64){} fn main(){let seed=SEED;let a=g(())-(@+());}",
            "0",
            false,
        ),
        (
            "fn h(){let a=@+();} fn main(){let seed=SEED;let b:i64=();}",
            "0",
            false,
        ),
        (
            "fn main(){let seed=SEED;let a=@+();let b:i64=();let c:i64=();let d:i64=();}",
            "0",
            false,
        ),
    ];
    let limit = std::env::var("NEXO_TWIN_LIMIT")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(300_000usize);
    let divergence = "(if true{return;}else{return;})";
    for seed in 0..limit {
        let template_index = seed % templates.len();
        let (template, literal, ineffective) = templates[template_index];
        // Expected rejections come from the written value and operator rules.
        let expected_rejection =
            matches!(template_index, 0 | 2 | 4 | 22 | 26 | 39 | 44..=46 | 53 | 55..=66);
        let template = template.replace("SEED", &(seed + 1).to_string());
        let at = template.find('@').unwrap() as u64;
        let divergent = template.replacen('@', divergence, 1);
        let live = template.replacen('@', literal, 1);
        let dead_result = compile_scalar("twin.nx", &divergent);
        let live_result = compile_scalar("twin.nx", &live);
        match (dead_result, live_result) {
            (Ok(dead), Ok(alive)) => {
                assert!(!expected_rejection, "incorrect acceptance at seed {seed}");
                if ineffective {
                    assert_eq!(
                        crate::interp::run_scalar(&dead).observations.stdout(),
                        crate::interp::run_scalar(&alive).observations.stdout(),
                        "seed {seed}"
                    );
                }
            }
            (Err(CompileError::Rejected(dead)), Err(CompileError::Rejected(alive))) => {
                assert!(expected_rejection, "incorrect rejection at seed {seed}");
                assert_eq!(dead.len(), alive.len(), "seed {seed}: {divergent}");
                for (dead, alive) in dead.iter().zip(alive.iter()) {
                    assert!(
                        !format!("{:?}", dead.data).contains("Never"),
                        "seed {seed}: {divergent}"
                    );
                    dead.validate("twin.nx", &divergent).unwrap();
                    alive.validate("twin.nx", &live).unwrap();
                    let map = |offset: u64| -> u64 {
                        let end = at + divergence.len() as u64;
                        if offset <= at {
                            offset
                        } else if offset >= end {
                            offset - (divergence.len() - literal.len()) as u64
                        } else {
                            at + (offset - at).min(literal.len() as u64)
                        }
                    };
                    let mapped = |span: &crate::diagnostic::Span| (map(span.start), map(span.end));
                    assert_eq!(
                        (
                            &dead.code,
                            &dead.subcause,
                            dead.stage,
                            dead.operation,
                            &dead.data
                        ),
                        (
                            &alive.code,
                            &alive.subcause,
                            alive.stage,
                            alive.operation,
                            &alive.data
                        ),
                        "seed {seed}: {divergent}"
                    );
                    assert_eq!(
                        dead.primary_span.as_ref().map(mapped),
                        alive.primary_span.as_ref().map(|s| (s.start, s.end)),
                        "seed {seed}: {divergent}"
                    );
                    assert_eq!(
                        dead.related_spans
                            .iter()
                            .map(|r| (r.role, mapped(&r.span)))
                            .collect::<Vec<_>>(),
                        alive
                            .related_spans
                            .iter()
                            .map(|r| (r.role, (r.span.start, r.span.end)))
                            .collect::<Vec<_>>(),
                        "seed {seed}: {divergent}"
                    );
                }
            }
            (dead, alive) => panic!("seed {seed}: {divergent}: divergent={dead:?}, live={alive:?}"),
        }
    }
}

#[test]
fn divergence_in_argument_and_index_checks_later_values() {
    let sources = [
        "fn h(a:i64,b:i64){} fn main(){h((if true{return;}else{return;}),if true{1;}else{value:2;});}",
        "fn main(){var a=[1];a[if true{return;}else{return;}]=if true{1;}else{value:2;};}",
    ];
    for source in sources {
        let Err(CompileError::Rejected(errors)) = compile_scalar("dead_later.nx", source) else {
            panic!("expected missing branch value: {source}");
        };
        assert!(
            errors
                .iter()
                .any(|d| d.code == "NX-E1302"
                    && d.subcause.as_deref() == Some("missing_branch_value")),
            "{source}: {errors:?}"
        );
    }
}

#[test]
fn value_after_return_is_checked_in_dead_branch() {
    let source =
        "fn main(){let x=if true{return;value:(if true{1;}else{value:2;});}else{value:3;};}";
    let Err(CompileError::Rejected(errors)) = compile_scalar("dead_value.nx", source) else {
        panic!("expected missing branch value");
    };
    assert!(
        errors
            .iter()
            .any(|d| d.code == "NX-E1302" && d.subcause.as_deref() == Some("missing_branch_value")),
        "{errors:?}"
    );
}

#[test]
fn divergent_while_condition_checks_body() {
    let source =
        "fn main(){while (if true{return;}else{return;}){let r=mock_open(1);drop(r);inspect(r);}}";
    let Err(CompileError::Rejected(errors)) = compile_scalar("dead_while.nx", source) else {
        panic!("expected consumed resource diagnostic");
    };
    assert!(
        errors
            .iter()
            .any(|d| d.code == "NX-E0202" && d.subcause.as_deref() == Some("definitely_consumed")),
        "{errors:?}"
    );
}

#[test]
fn divergent_intrinsic_and_array_elements_check_later_branches() {
    let sources = [
        "fn main(){assert_eq((if true{return;}else{return;}),if true{1;}else{value:2;});}",
        "fn main(){let a=[(if true{return;}else{return;}),if true{1;}else{value:2;}];}",
    ];
    for source in sources {
        let Err(CompileError::Rejected(errors)) = compile_scalar("dead_items.nx", source) else {
            panic!("expected missing branch value: {source}");
        };
        assert!(
            errors
                .iter()
                .any(|d| d.code == "NX-E1302"
                    && d.subcause.as_deref() == Some("missing_branch_value")),
            "{source}: {errors:?}"
        );
    }
}

#[test]
fn dead_analysis_reports_branch_structure_across_expression_positions() {
    let sources = [
        "fn f()->i64{return (if true{return 1;}else{return 2;})+(if true{1;}else{value:2;});} fn main(){f();}",
        "fn main(){return;let x=if true{1;}else{value:2;};}",
        "fn main(){while false{return;let x=if true{1;}else{value:2;};}}",
        "fn main(){if (if true{return;}else{return;}){let x=if true{1;}else{value:2;};}}",
        "fn main(){var a=[1];a[if true{return;}else{return;}]=if true{1;}else{value:2;};}",
        "fn h(a:i64,b:i64){} fn main(){let v=if true{return;}else{return;};h(v,if true{1;}else{value:2;});}",
        "fn main(){var a=[1];let v=if true{return;}else{return;};a[v]=if true{1;}else{value:2;};}",
    ];
    for source in sources {
        let Err(CompileError::Rejected(errors)) = compile_scalar("dead_matrix.nx", source) else {
            panic!("expected missing branch value: {source}");
        };
        assert!(
            errors
                .iter()
                .any(|d| d.code == "NX-E1302"
                    && d.subcause.as_deref() == Some("missing_branch_value")),
            "{source}: {errors:?}"
        );
    }
}

#[test]
fn valid_unreachable_siblings_do_not_create_ir_and_independent_errors_accumulate() {
    let valid =
        "fn main(){print((if true{return;}else{return;})+(if true{value:1;}else{value:2;}));}";
    assert!(compile_scalar("dead_valid.nx", valid).is_ok());

    let invalid = "fn main(){let r=mock_open(1);drop(r);print((if true{return;}else{return;})+(if true{1;}else{value:2;}));inspect(r);}";
    let Err(CompileError::Rejected(errors)) = compile_scalar("dead_errors.nx", invalid) else {
        panic!("expected two independent diagnostics");
    };
    assert_eq!(errors.len(), 2, "{errors:?}");
    assert_eq!(
        (errors[0].code.as_str(), errors[0].subcause.as_deref()),
        ("NX-E1302", Some("missing_branch_value"))
    );
    assert_eq!(
        (errors[1].code.as_str(), errors[1].subcause.as_deref()),
        ("NX-E0202", Some("definitely_consumed"))
    );
}

#[test]
fn never_coerces_branch_value_without_a_visible_unit_type() {
    let valid = "fn main(){let x=if true{value:(if true{return;}else{return;})-3;}else{value:1;};}";
    let result = compile_scalar("never_value.nx", valid);
    assert!(result.is_ok(), "{result:?}");
}

#[test]
fn divergent_left_addition_still_checks_the_right_operand_domain() {
    use crate::diagnostic::{DataValue, Operation, RelatedRole, Stage};
    // F0-0002 §4.7 permits the divergent operand to coerce; §4.8 keeps + on i64.
    let source = "fn main(){print((if true{return;}else{return;})+true);}";
    let Err(CompileError::Rejected(errors)) = compile_scalar("never_add_bool.nx", source) else {
        panic!("expected i64 operand mismatch");
    };
    assert_eq!(errors.len(), 1, "{errors:?}");
    let error = &errors[0];
    assert_eq!(
        (error.code.as_str(), error.subcause.as_deref()),
        ("NX-E0102", Some("type_mismatch"))
    );
    assert_eq!(
        (error.stage, error.operation),
        (Stage::Types, Operation::TypesCheck)
    );
    let at = source.find("true);").unwrap() as u64;
    assert_eq!(
        error.primary_span.as_ref().map(|s| (s.start, s.end)),
        Some((at, at + 4))
    );
    assert_eq!(error.related_spans.len(), 1);
    assert_eq!(error.related_spans[0].role, RelatedRole::FoundType);
    assert_eq!(
        (
            error.related_spans[0].span.start,
            error.related_spans[0].span.end
        ),
        (at, at + 4)
    );
    assert_eq!(error.data.len(), 2);
    assert_eq!(
        error.data.get("expected_type"),
        Some(&DataValue::String("i64".into()))
    );
    assert_eq!(
        error.data.get("found_type"),
        Some(&DataValue::String("bool".into()))
    );
    error.validate("never_add_bool.nx", source).unwrap();
}

#[test]
fn later_uses_of_divergent_bindings_keep_structural_diagnostics() {
    let sources = [
        "fn main(){let v=if true{return;}else{return;};print(v+(if true{1;}else{value:2;}));}",
        "fn main(){let v=if true{return;}else{return;};if v{let x=if true{1;}else{value:2;};}}",
        "fn main(){let v=if true{return;}else{return;};while v{let x=if true{1;}else{value:2;};}}",
        "fn main(){var a:i64=0;let v=if true{return;}else{return;};a=v+(if true{1;}else{value:2;});}",
        "fn main(){let v=if true{return;}else{return;};let x=if true{value:v+(if true{1;}else{value:2;});}else{value:1;};}",
        "fn f()->i64{let v=if true{return 1;}else{return 2;};return v+(if true{1;}else{value:2;});} fn main(){f();}",
    ];
    for source in sources {
        let Err(CompileError::Rejected(errors)) = compile_scalar("dead_binding_use.nx", source)
        else {
            panic!("expected missing branch value: {source}");
        };
        assert!(
            errors
                .iter()
                .any(|d| d.code == "NX-E1302"
                    && d.subcause.as_deref() == Some("missing_branch_value")),
            "{source}: {errors:?}"
        );
    }
}

#[test]
fn independent_branch_ownership_errors_accumulate_in_live_and_dead_modes() {
    for prefix in ["", "return;"] {
        let source = format!(
            "fn main(){{{prefix}let r=mock_open(1);drop(r);let x=if true{{value:inspect(r);}}else{{value:inspect(r);}};}}"
        );
        let Err(CompileError::Rejected(errors)) = compile_scalar("branch_cardinality.nx", &source)
        else {
            panic!("expected ownership diagnostics: {source}");
        };
        assert_eq!(errors.len(), 2, "{source}: {errors:?}");
        for error in errors {
            assert_eq!(
                (error.code.as_str(), error.subcause.as_deref()),
                ("NX-E0202", Some("definitely_consumed"))
            );
            error.validate("branch_cardinality.nx", &source).unwrap();
        }
    }
}

#[test]
fn consumed_divergent_binding_is_checked_when_used_as_read_argument() {
    let source =
        "fn h(a:i64,b:i64){} fn main(){let v=if true{return;}else{return;};drop(v);h(v,1);}";
    let Err(CompileError::Rejected(errors)) = compile_scalar("dead_read_arg.nx", source) else {
        panic!("expected consumed binding diagnostic");
    };
    assert_eq!(errors.len(), 1, "{errors:?}");
    assert_eq!(
        (errors[0].code.as_str(), errors[0].subcause.as_deref()),
        ("NX-E0202", Some("definitely_consumed"))
    );
    errors[0].validate("dead_read_arg.nx", source).unwrap();
}

#[test]
fn independent_intrinsic_arguments_accumulate_ownership_errors_in_dead_mode() {
    for prefix in ["", "return;"] {
        let source =
            format!("fn main(){{{prefix}var a=[1];var b=[2];drop(a);drop(b);assert_eq(a,b);}}");
        let Err(CompileError::Rejected(errors)) = compile_scalar("dead_intrinsic_args.nx", &source)
        else {
            panic!("expected two consumed array diagnostics: {source}");
        };
        assert_eq!(errors.len(), 2, "{source}: {errors:?}");
        for error in errors {
            assert_eq!(
                (error.code.as_str(), error.subcause.as_deref()),
                ("NX-E0202", Some("definitely_consumed"))
            );
            error.validate("dead_intrinsic_args.nx", &source).unwrap();
        }
    }
}

#[test]
fn independent_array_elements_accumulate_ownership_errors_in_dead_mode() {
    for prefix in ["", "return;"] {
        let source =
            format!("fn main(){{{prefix}var a=[1];var b=[2];drop(a);drop(b);let c=[a,b];}}");
        let Err(CompileError::Rejected(errors)) = compile_scalar("dead_array_items.nx", &source)
        else {
            panic!("expected two consumed array diagnostics: {source}");
        };
        assert_eq!(errors.len(), 2, "{source}: {errors:?}");
        for error in errors {
            assert_eq!(
                (error.code.as_str(), error.subcause.as_deref()),
                ("NX-E0202", Some("definitely_consumed"))
            );
            error.validate("dead_array_items.nx", &source).unwrap();
        }
    }
}

#[test]
fn branch_value_contract_covers_divergent_expression_positions() {
    use crate::diagnostic::{Operation, RelatedRole, Stage};

    // F0-0002 §4.7, §6.2 and §13.9: a branch that can continue without
    // `value:` is anchored at its closing brace, with the whole if as context.
    // The offsets below come from source bytes, independently of diagnostics.
    const BAD: &str = "if true{1;}else{value:2;}";
    let sources = [
        "fn main(){print((if true{return;}else{return;})+(if true{1;}else{value:2;}));}",
        "fn main(){print((if true{1;}else{value:2;})+(if true{return;}else{return;}));}",
        "fn h(a:i64,b:i64,c:i64){} fn main(){h(if true{return;}else{return;},if true{1;}else{value:2;},3);}",
        "fn h(a:i64,b:i64,c:i64){} fn main(){h(1,if true{return;}else{return;},if true{1;}else{value:2;});}",
        "fn main(){var a=[1];a[if true{return;}else{return;}]=if true{1;}else{value:2;};}",
        "fn main(){var a=[1];a[0]=if true{return;}else{return;};let x=if true{1;}else{value:2;};}",
        "fn main(){if (if true{return;}else{return;}){let x=if true{1;}else{value:2;};}}",
        "fn main(){while (if true{return;}else{return;}){let x=if true{1;}else{value:2;};}}",
        "fn main(){let x=if true{value:(if true{return;}else{return;})+(if true{1;}else{value:2;});}else{value:1;};}",
        "fn f()->i64{return (if true{return 1;}else{return 2;})+(if true{1;}else{value:2;});} fn main(){f();}",
        "fn main(){return;let x=if true{1;}else{value:2;};}",
        "fn main(){while false{return;let x=if true{1;}else{value:2;};}}",
        "fn main(){let v=if true{return;}else{return;};print(v+(if true{1;}else{value:2;}));}",
        "fn main(){let v:i64=if true{return;}else{return;};print(v+(if true{1;}else{value:2;}));}",
        "fn main(){let v=if true{return;}else{return;};if v{let x=if true{1;}else{value:2;};}}",
        "fn main(){let v:bool=if true{return;}else{return;};while v{let x=if true{1;}else{value:2;};}}",
        "fn main(){var a=[1];let v=if true{return;}else{return;};a[v]=if true{1;}else{value:2;};}",
        "fn h(a:i64,b:i64){} fn main(){let v=if true{return;}else{return;};h(v,if true{1;}else{value:2;});}",
    ];
    for source in sources {
        let expected: Vec<_> = source
            .match_indices(BAD)
            .map(|(start, _)| {
                let first_close = start + "if true{1;".len();
                (
                    (first_close as u64, first_close as u64 + 1),
                    (start as u64, (start + BAD.len()) as u64),
                )
            })
            .collect();
        assert_eq!(expected.len(), 1, "fixture: {source}");
        let Err(CompileError::Rejected(errors)) = compile_scalar("branch_position.nx", source)
        else {
            panic!("expected branch rejection: {source}")
        };
        assert_eq!(errors.len(), expected.len(), "{source}: {errors:?}");
        for (error, (primary, context)) in errors.iter().zip(expected) {
            assert_eq!(
                (error.code.as_str(), error.subcause.as_deref()),
                ("NX-E1302", Some("missing_branch_value")),
                "{source}"
            );
            assert_eq!(
                (error.stage, error.operation),
                (Stage::Ownership, Operation::Branch),
                "{source}"
            );
            assert_eq!(
                error.primary_span.as_ref().map(|s| (s.start, s.end)),
                Some(primary),
                "{source}"
            );
            assert_eq!(error.related_spans.len(), 1, "{source}");
            assert_eq!(
                error.related_spans[0].role,
                RelatedRole::Context,
                "{source}"
            );
            assert_eq!(
                (
                    error.related_spans[0].span.start,
                    error.related_spans[0].span.end
                ),
                context,
                "{source}"
            );
            assert!(error.data.is_empty(), "{source}");
            assert!(error.unexpected_token.is_none(), "{source}");
            error.validate("branch_position.nx", source).unwrap();
        }
    }
}

#[test]
fn independent_branch_value_errors_keep_exact_contract_after_divergence() {
    use crate::diagnostic::{Operation, RelatedRole, Stage};
    // F0-0002 §4.7, §6.2, §13.9: each independent continuation lacking
    // `value:` contributes one flow diagnostic, including in dead analysis.
    let cases = [
        (
            "fn main(){return;let a=if true{1;}else{value:2;};let b=if true{value:3;}else{4;};}",
            ["if true{1;}else{value:2;}", "if true{value:3;}else{4;}"],
        ),
        (
            "fn main(){let x=if true{return;}else{return;};let a=if true{1;}else{value:2;};let b=if true{value:3;}else{4;};}",
            ["if true{1;}else{value:2;}", "if true{value:3;}else{4;}"],
        ),
        (
            "fn main(){let x=if true{value:if true{1;}else{value:2;};}else{value:if true{value:3;}else{4;};};}",
            ["if true{1;}else{value:2;}", "if true{value:3;}else{4;}"],
        ),
    ];
    for (source, forms) in cases {
        let Err(CompileError::Rejected(errors)) = compile_scalar("branch_many.nx", source) else {
            panic!("expected two branch errors: {source}")
        };
        assert_eq!(errors.len(), 2, "{source}: {errors:?}");
        for (error, form) in errors.iter().zip(forms) {
            let start = source.find(form).unwrap();
            let closing = if form.starts_with("if true{1;") {
                start + "if true{1;".len()
            } else {
                start + form.len() - 1
            };
            assert_eq!(
                (error.code.as_str(), error.subcause.as_deref()),
                ("NX-E1302", Some("missing_branch_value")),
                "{source}"
            );
            assert_eq!(
                (error.stage, error.operation),
                (Stage::Ownership, Operation::Branch)
            );
            assert_eq!(
                error.primary_span.as_ref().map(|s| (s.start, s.end)),
                Some((closing as u64, closing as u64 + 1)),
                "{source}"
            );
            assert_eq!(error.related_spans.len(), 1, "{source}");
            assert_eq!(error.related_spans[0].role, RelatedRole::Context);
            assert_eq!(
                (
                    error.related_spans[0].span.start,
                    error.related_spans[0].span.end
                ),
                (start as u64, (start + form.len()) as u64),
                "{source}"
            );
            assert!(error.data.is_empty());
            assert!(error.unexpected_token.is_none());
            error.validate("branch_many.nx", source).unwrap();
        }
    }
}

#[test]
#[ignore = "explicit 256 KiB to 1 MiB divergent and live scale measurement"]
fn divergent_and_live_sources_scale_in_work_bytes_and_allocations() {
    // F0-0002 §6.2 retains dead-code checks; §13.9 retains independent sites.
    // Allocation traffic is captured around compilation, so cloning a growing
    // lowering state or temporary list changes the ratio and fails this test.
    let families = [
        (
            "fn main(){let b=true;",
            "print(1+(if b{return;}else{return;}));",
            "print(1+(if b{value:2;}else{value:3;}));",
        ),
        (
            "fn f(a:i64,b:i64){} fn main(){let b=true;",
            "f(1,if b{return;}else{return;});",
            "f(1,if b{value:2;}else{value:3;});",
        ),
        (
            "fn main(){let b=true;",
            "if (if b{return;}else{return;}){print(1);}",
            "if (if b{value:true;}else{value:false;}){print(1);}",
        ),
        (
            "fn main(){let b=true;",
            "while (if b{return;}else{return;}){print(1);}",
            "print(1);",
        ),
        (
            "fn main(){let b=true;",
            "print(1+(if b{value:if b{return;}else{return;};}else{return;}));",
            "print(1+(if b{value:if b{value:2;}else{value:3;};}else{value:4;}));",
        ),
        (
            "fn main(){",
            "return;{let x=1;print(x);}",
            "{let x=1;print(x);}",
        ),
        (
            "fn main(){let b=true;var a=[1];",
            "a[0]=if b{return;}else{return;};",
            "a[0]=if b{value:2;}else{value:3;};",
        ),
        (
            "fn main(){let b=true;var a=[1];",
            "a[if b{return;}else{return;}]=2;",
            "a[if b{value:0;}else{value:0;}]=2;",
        ),
        (
            "fn main(){let b=true;",
            "while false{print(1+(if b{return;}else{return;}));}",
            "print(1+(if b{value:2;}else{value:3;}));",
        ),
        (
            "fn f(a:i64,b:i64,c:i64){} fn main(){let b=true;",
            "f(1,if b{return;}else{return;},3);",
            "f(1,if b{value:2;}else{value:3;},3);",
        ),
        (
            "fn main(){let b=true;",
            "{var v=if b{return;}else{return;};v=if b{value:3;}else{value:7;};}",
            "{var v=1;v=if b{value:3;}else{value:7;};}",
        ),
        (
            "fn main(){let b=true;",
            "{let w=(((if b{return;}else{return;})-1)-2);}",
            "{let w=((0-1)-2);}",
        ),
        (
            "fn main(){let b=true;",
            "{let w=[(if b{return;}else{return;}),true,true];}",
            "{let w=[true,true,true];}",
        ),
    ];
    for (family, (prefix, dead, live)) in families.iter().enumerate() {
        if let Ok(only) = std::env::var("NEXO_DIVERGENT_SCALE_FAMILY")
            && only.parse::<usize>().ok() != Some(family)
        {
            continue;
        }
        for (mode, pattern) in [("dead", dead), ("live", live)] {
            let mut work = [0_u64; 2];
            let mut traffic = [(0_u64, 0_u64); 2];
            let small = std::env::var("NEXO_DIVERGENT_SCALE_SMALL_KIB")
                .ok()
                .and_then(|n| n.parse::<usize>().ok())
                .unwrap_or(256)
                * 1024;
            for (at, target) in [small, small * 4].into_iter().enumerate() {
                let mut source = String::from(*prefix);
                while source.len() + pattern.len() < target {
                    source.push_str(pattern);
                }
                source.push('}');
                crate::test_alloc::reset("dead_scale_counter.nx");
                super::CFG_WORK.store(0, std::sync::atomic::Ordering::Relaxed);
                let result = compile_scalar("dead_scale_counter.nx", &source);
                result.unwrap_or_else(|e| panic!("family {family} {mode}: {e:?}"));
                work[at] = super::CFG_WORK.load(std::sync::atomic::Ordering::Relaxed);
                traffic[at] = crate::test_alloc::snapshot("dead_scale_counter.nx");
            }
            for (name, small, large) in [
                ("steps", work[0], work[1]),
                ("bytes", traffic[0].0, traffic[1].0),
                ("allocations", traffic[0].1, traffic[1].1),
            ] {
                let ratio = large as f64 / small as f64;
                eprintln!("family {family} {mode} {name}: {small} -> {large}, ratio {ratio:.3}");
                let minimum = if name == "bytes" { 2.5 } else { 3.5 };
                assert!(
                    (minimum..=4.6).contains(&ratio),
                    "family {family} {mode} {name}: {small} -> {large}"
                );
            }
        }
    }
}
