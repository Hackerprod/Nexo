use super::{
    CompileError, ReducedProfile, check_scalar, compile_scalar, compile_scalar_with_profile,
    compile_scalar_with_sparse_ownership,
};
use crate::{
    diagnostic::DataValue,
    interp::{Outcome, run_scalar},
};

#[test]
fn cfg_dominance_rejects_temporary_from_sibling_branch() {
    use super::model::{Access, BasicBlock, Instruction, OriginId, ScopeId, TempId, Terminator};

    let program = compile_scalar(
        "dominance.n",
        "fn Main(){let x=if true then true else false;}",
    )
    .unwrap();
    let mut function = program.functions.entries[0].clone();
    let branch_value = function.blocks[0].instructions[0].output.unwrap();
    assert_eq!(
        function.blocks[0].instructions[0].ty,
        Some(super::model::Ty::Bool)
    );
    let original = function.blocks[0].instructions[0].clone();
    let replacement_id = TempId(
        function
            .blocks
            .iter()
            .flat_map(|block| &block.instructions)
            .filter_map(|instruction| instruction.output)
            .map(|temp| temp.0)
            .max()
            .unwrap()
            + 1,
    );
    let mut replacement = original;
    replacement.output = Some(replacement_id);
    replacement.origin = OriginId(1);
    function.blocks[0].instructions.insert(1, replacement);
    for (index, instruction) in function.blocks[0].instructions.iter_mut().enumerate() {
        instruction.origin = OriginId(index);
        if let super::model::Op::WritePlace { value, .. } = &mut instruction.op {
            *value = replacement_id;
        }
    }
    let join = &mut function.blocks[2];
    let span = join.instructions[0].span.clone();
    join.instructions
        .retain(|instruction| !matches!(instruction.op, super::model::Op::ConstUnit));
    join.terminator = Terminator::BranchBool {
        condition: branch_value,
        yes: super::model::BlockId(4),
        no: super::model::BlockId(5),
        span: span.clone(),
        charges: vec![],
    };
    for id in [4, 5] {
        let output = TempId(replacement_id.0 + (id - 3));
        function.blocks.push(BasicBlock {
            id: super::model::BlockId(id),
            instructions: vec![Instruction {
                op: super::model::Op::ConstUnit,
                output: Some(output),
                ty: Some(super::model::Ty::Unit),
                span: span.clone(),
                scope: ScopeId(0),
                origin: OriginId(0),
                access: Access::Own,
                effect_io: false,
                charges: vec![super::model::WorkCharge {
                    ticks: 1,
                    origin: span.clone(),
                }],
            }],
            terminator: Terminator::Return {
                value: Some(output),
                span: span.clone(),
                charges: vec![],
            },
        });
    }
    function.content_hash = super::model::content_hash(&function);
    assert_eq!(
        super::verify::verify_function(&function),
        Err("branch condition type")
    );
}

fn nested_if(height: usize) -> String {
    let expression = "(if true then ".repeat(height) + "1" + &" else 1)".repeat(height);
    format!("fn Main(){{let x={expression};}}")
}

fn expression_depth(terms: usize) -> String {
    let term = "(if true then 1 else 1)";
    format!("fn Main(){{let x={};}}", vec![term; terms].join("+"))
}

fn nested_ownership(name: &str, bindings: usize) -> String {
    format!("fn {name}(b:view Bool){{")
        + &"while b{".repeat(96)
        + &(0..bindings)
            .map(|index| format!("let x{index}=0;"))
            .collect::<String>()
        + &"}".repeat(97)
}

#[test]
fn ownership_copy_diagnostic_message_comes_from_its_catalog_entry() {
    let source = "fn Main(){let r=MockOpen(1);let duplicate=r;}";
    let Err(CompileError::Rejected(errors)) = check_scalar("copy_message.n", source) else {
        panic!("copying a resource must be rejected");
    };
    let copy = errors
        .iter()
        .find(|error| error.code == "NX-E0201")
        .expect("ownership copy error");
    assert_eq!(
        copy.message,
        crate::diagnostic::catalog_message("NX-E0201").unwrap()
    );
}

#[test]
fn repeated_resource_use_diagnostic_message_comes_from_its_catalog_entry() {
    let source = "fn Main(){let r=MockOpen(1);Drop(r);Drop(r);}";
    let Err(CompileError::Rejected(errors)) = check_scalar("drop_message.n", source) else {
        panic!("using a consumed resource must be rejected");
    };
    let consumed = errors
        .iter()
        .find(|error| error.code == "NX-E0202")
        .expect("use-after-consume error");
    assert_eq!(
        consumed.message,
        crate::diagnostic::catalog_message("NX-E0202").unwrap()
    );
}

#[test]
fn nested_if_names_depth_boundaries_match_parser_and_run() {
    for height in [86, 100, 125] {
        let source = nested_if(height);
        check_scalar("names_depth.n", &source).unwrap();
        let program = compile_scalar("names_depth.n", &source).unwrap();
        assert!(
            !matches!(run_scalar(&program).outcome, Outcome::InternalError(_)),
            "height {height}"
        );
    }
    let source = nested_if(126);
    for result in [
        check_scalar("names_depth.n", &source),
        compile_scalar("names_depth.n", &source),
    ] {
        let Err(CompileError::Rejected(errors)) = result else {
            panic!("syntax nesting 126 must be rejected");
        };
        let quota = errors.iter().find(|d| d.code == "NX-E0003").expect("quota");
        assert_eq!(
            quota.data.get("budget_kind"),
            Some(&DataValue::String("syntax_nesting".into()))
        );
    }
}

#[test]
fn official_expression_tree_depth_127_and_128_boundaries() {
    let source_127 = expression_depth(127);
    assert_eq!(source_127.len(), 3_065);
    let program = check_scalar("expression_depth.n", &source_127).unwrap();
    assert!(!matches!(
        run_scalar(&program).outcome,
        Outcome::InternalError(_)
    ));
    let source_128 = expression_depth(128);
    assert_eq!(source_128.len(), 3_089);
    let Err(CompileError::Rejected(errors)) = check_scalar("expression_depth.n", &source_128)
    else {
        panic!("height 128 source has expression tree depth 129");
    };
    let quota = errors.iter().find(|d| d.code == "NX-E0003").expect("quota");
    assert_eq!(
        quota.data.get("budget_kind"),
        Some(&DataValue::String("expression_tree_depth".into()))
    );
    assert_eq!(
        quota
            .primary_span
            .as_ref()
            .map(|span| (span.start, span.end)),
        Some((16, 3_087))
    );
}

#[test]
fn ownership_flow_limits_distinguish_local_and_compilation_scope() {
    let a1 = format!(
        "fn F(b:view Bool){{{}{}}}fn Main(){{}}",
        (0..1024)
            .map(|index| format!("let x{index}=0;"))
            .collect::<String>(),
        "if b{}else{}".repeat(1024)
    );
    assert_eq!(a1.len(), 23_496);
    check_scalar("flow_limits.n", &a1).unwrap();
    for bindings in [64, 256, 1024] {
        let source = format!("{}fn Main(){{}}", nested_ownership("F", bindings));
        check_scalar("flow_limits.n", &source).unwrap();
    }
    let l1 = format!("{}fn Main(){{}}", nested_ownership("F", 6000));
    assert_eq!(l1.len(), 71_784);
    let Err(CompileError::Rejected(local)) = check_scalar("flow_limits.n", &l1) else {
        panic!("one function must exhaust its local ownership quota");
    };
    let local = local
        .iter()
        .find(|diagnostic| diagnostic.code == "NX-E0003")
        .expect("local ownership quota");
    assert_eq!(
        local.data.get("budget_scope"),
        Some(&DataValue::String("function".into()))
    );
    assert_eq!(local.data.get("limit"), Some(&DataValue::UInt(1_000_000)));
    assert_eq!(
        local.data.get("observed"),
        Some(&DataValue::UInt(1_000_001))
    );
    assert_eq!(
        local.data.get("analysis_function"),
        Some(&DataValue::Object(
            [
                ("kind".into(), DataValue::String("function".into())),
                ("id".into(), DataValue::String("function:3".into()))
            ]
            .into_iter()
            .collect()
        ))
    );
    let global = format!(
        "{}fn Main(){{}}",
        (0..14)
            .map(|index| nested_ownership(&format!("F{index}"), 1024))
            .collect::<String>()
    );
    assert_eq!(global.len(), 168_883);
    let Err(CompileError::Rejected(errors)) = check_scalar("flow_limits.n", &global) else {
        panic!("fourteen functions must exhaust the compilation ownership quota");
    };
    let global = errors
        .iter()
        .find(|diagnostic| diagnostic.code == "NX-E0003")
        .expect("global ownership quota");
    assert_eq!(
        global.data.get("budget_scope"),
        Some(&DataValue::String("compilation".into()))
    );
    assert_eq!(global.data.get("limit"), Some(&DataValue::UInt(4_000_000)));
    assert_eq!(
        global.data.get("observed"),
        Some(&DataValue::UInt(4_000_001))
    );
    assert!(!global.data.contains_key("analysis_function"));
    global
        .validate(
            "flow_limits.n",
            &format!(
                "{}fn Main(){{}}",
                (0..14)
                    .map(|index| nested_ownership(&format!("F{index}"), 1024))
                    .collect::<String>()
            ),
        )
        .unwrap();
    let thirteen = format!(
        "{}fn Main(){{}}",
        (0..13)
            .map(|index| nested_ownership(&format!("F{index}"), 1024))
            .collect::<String>()
    );
    check_scalar("flow_limits.n", &thirteen).unwrap();
}

#[test]
fn rejected_local_ownership_charges_do_not_consume_global_allowance() {
    let rejected = || {
        "fn {name}(b:view Bool){".replace("{name}", "A")
            + &"while b{".repeat(96)
            + &(0..6000)
                .map(|index| format!("let x{index}=0;"))
                .collect::<String>()
            + &"}".repeat(97)
    };
    let mut source = String::new();
    for name in ["A0", "A1", "A2"] {
        source.push_str(&rejected().replace("fn A(", &format!("fn {name}(")));
    }
    let p_nested = "while b{".repeat(63)
        + &(0..3384)
            .map(|index| format!("let p{index}=0;"))
            .collect::<String>()
        + &"}".repeat(63);
    let p_final = (3384..3447)
        .map(|index| format!("let p{index}=0;"))
        .collect::<String>();
    source.push_str(&format!("fn P(b:view Bool){{{}{}}}", p_nested, p_final));
    source.push_str("fn Main(){}");
    let Err(CompileError::Rejected(errors)) = check_scalar("rejected_charge.n", &source) else {
        panic!("the three oversized functions must each exceed their local quota");
    };
    assert_eq!(
        errors.len(),
        3,
        "later cost within remaining global quota: {errors:?}"
    );
    assert!(errors.iter().all(|error| {
        error.code == "NX-E0003"
            && error.data.get("budget_scope") == Some(&DataValue::String("function".into()))
    }));
}

#[test]
fn global_flow_limit_currently_keeps_later_nonloop_property_diagnostics() {
    let first = format!(
        "fn First(){{{}}}",
        (0..100)
            .map(|index| format!("let x{index}=0;"))
            .collect::<String>()
    );
    let source = format!("{first}fn Later(){{let z=0;Drop(z);Print(z);}}fn Main(){{}}");
    let Err(CompileError::Rejected(errors)) =
        compile_scalar_with_profile("global_later.n", &source, ReducedProfile::FlowCompilation32)
    else {
        panic!("the reduced global budget must reject the first function");
    };
    assert!(errors.iter().any(|error| {
        error.code == "NX-E0003"
            && error.data.get("budget_scope") == Some(&DataValue::String("compilation".into()))
    }));
    assert!(errors.iter().any(|error| {
        error.code == "NX-E0202"
            && error
                .primary_span
                .as_ref()
                .is_some_and(|span| span.start > first.len() as u64)
    }));
}

#[test]
fn remaining_resource_branch_and_loop_patterns_keep_current_acceptance() {
    let mut source = String::from("fn Branches(b:view Bool){");
    for index in 0..8 {
        source.push_str(&format!(
            "let r{index}=MockOpen(1);if b{{Drop(r{index});return;}}"
        ));
    }
    source.push_str("}fn Loops(b:view Bool){");
    for index in 0..8 {
        source.push_str(&format!(
            "var v{index}=MockOpen(1);Drop(v{index});while b{{v{index}=MockOpen(2);}}"
        ));
    }
    source.push_str("}fn Main(){}");
    check_scalar("remaining_resource_patterns.n", &source)
        .unwrap_or_else(|errors| panic!("current ownership pattern changed: {errors:?}"));
}

#[test]
fn lowering_worker_preserves_configured_stack_margin() {
    let source = format!("fn Main(){{{}let x=1;{}}}", "{".repeat(80), "}".repeat(80));
    std::thread::Builder::new()
        .stack_size(512 * 1024)
        .spawn(move || compile_scalar("lower_stack_margin.n", &source).unwrap())
        .unwrap()
        .join()
        .unwrap();
}

#[test]
fn ownership_limit_keeps_earlier_and_later_diagnostics_in_source_order() {
    let big = || {
        "fn Big(b:view Bool){var s=MockOpen(1);Drop(s);while b{Inspect(s);s=MockOpen(2);}"
            .to_owned()
            + &"while b{".repeat(96)
            + &(0..6000)
                .map(|index| format!("let x{index}=0;"))
                .collect::<String>()
            + &"}".repeat(97)
    };
    let source = big() + "fn Main(){}";
    assert_eq!(source.len(), 71_846);
    let Err(CompileError::Rejected(errors)) = check_scalar("flow_d1.n", &source) else {
        panic!("B-D1 must retain the local ownership rejection");
    };
    assert_eq!(errors.len(), 2);
    assert_eq!(
        (
            errors[0].primary_span.as_ref().unwrap().start,
            errors[0].primary_span.as_ref().unwrap().end
        ),
        (62, 63)
    );
    assert_eq!(
        (
            errors[0].related_spans[0].span.start,
            errors[0].related_spans[0].span.end
        ),
        (24, 25)
    );
    assert_eq!(errors[0].cause_chain.len(), 1);
    assert_eq!(
        errors[0].cause_chain[0]
            .span
            .as_ref()
            .map(|span| (span.start, span.end)),
        Some((38, 45))
    );
    assert_eq!(
        errors[0].cause_chain[0]
            .entity
            .as_ref()
            .map(|entity| entity.id.as_str()),
        Some("binding:24")
    );
    assert_eq!(errors[1].code, "NX-E0003");
    assert_eq!(
        errors[1].data.get("observed"),
        Some(&DataValue::UInt(1_000_001))
    );
    let limit_span = errors[1].primary_span.as_ref().expect("quota span");
    assert_eq!((limit_span.start, limit_span.end), (49_740, 49_741));
    assert_eq!(errors[1].profile_id, "f0-0002-b-v2");
    for diagnostic in &errors {
        diagnostic.validate("flow_d1.n", &source).unwrap();
    }

    let later = "fn Later(){let z=0;Drop(z);Print(z);}";
    let with_later = big() + later + "fn Main(){}";
    assert_eq!(with_later.len(), 71_883);
    let Err(CompileError::Rejected(errors)) = check_scalar("flow_d1.n", &with_later) else {
        panic!("B-D1 continuation must preserve later independent errors");
    };
    assert_eq!(errors.len(), 3);
    assert_eq!(errors[0].primary_span.as_ref().unwrap().start, 62);
    assert_eq!(
        errors[1].data.get("observed"),
        Some(&DataValue::UInt(1_000_001))
    );
    assert_eq!(
        (
            errors[2].primary_span.as_ref().unwrap().start,
            errors[2].primary_span.as_ref().unwrap().end
        ),
        (71_868, 71_869)
    );
    assert_eq!(errors[2].related_spans[0].span.start, 71_850);
    assert_eq!(errors[2].related_spans[0].span.end, 71_851);
    assert_eq!(errors[2].cause_chain.len(), 1);
    assert_eq!(
        errors[2].cause_chain[0]
            .span
            .as_ref()
            .map(|span| (span.start, span.end)),
        Some((71_854, 71_861))
    );
    assert_eq!(
        errors[2].cause_chain[0]
            .entity
            .as_ref()
            .map(|entity| entity.id.as_str()),
        Some("binding:71850")
    );
    for diagnostic in &errors {
        diagnostic.validate("flow_d1.n", &with_later).unwrap();
    }

    let later_flow =
        "fn Later(b:view Bool){var t=MockOpen(1);Drop(t);while b{Inspect(t);t=MockOpen(2);}}";
    let continuation = big() + later_flow + "fn Main(){}";
    let Err(CompileError::Rejected(errors)) = check_scalar("flow_d1.n", &continuation) else {
        panic!("a local quota must not suppress ownership analysis of a later function");
    };
    assert!(errors.iter().any(|diagnostic| {
        diagnostic.code == "NX-E0003"
            && diagnostic.data.get("budget_scope") == Some(&DataValue::String("function".into()))
    }));
    let later_error = errors
        .iter()
        .find(|diagnostic| {
            diagnostic.code == "NX-E0202"
                && diagnostic
                    .primary_span
                    .as_ref()
                    .is_some_and(|span| span.start > 71_846)
        })
        .expect("later function ownership diagnostic");
    assert!(later_error.primary_span.as_ref().unwrap().start > big().len() as u64);
}

#[test]
fn ir_item_limit_is_deferred_behind_ownership_and_effect_errors() {
    let declarations = (0..65_537)
        .map(|index| format!("let x{index}=0;"))
        .collect::<String>();
    let ownership = format!("fn Main(){{let r=MockOpen(1);Drop(r);Inspect(r);{declarations}}}");
    assert!(ownership.len() < 1_048_576);
    let Err(CompileError::Rejected(ownership_errors)) = compile_scalar("ir_defer.n", &ownership)
    else {
        panic!("ownership rejection must precede the deferred IR quota");
    };
    assert!(
        ownership_errors
            .iter()
            .any(|diagnostic| diagnostic.code == "NX-E0202")
    );
    assert!(!ownership_errors.iter().any(|diagnostic| {
        diagnostic.data.get("budget_kind") == Some(&DataValue::String("ir_items".into()))
    }));

    let effects = format!("fn Main() with no_io{{Print(0);{declarations}}}");
    assert!(effects.len() < 1_048_576);
    let Err(CompileError::Rejected(effect_errors)) = compile_scalar("ir_defer.n", &effects) else {
        panic!("effect rejection must precede the deferred IR quota");
    };
    assert!(
        effect_errors
            .iter()
            .any(|diagnostic| diagnostic.code == "NX-E0415")
    );
    assert!(!effect_errors.iter().any(|diagnostic| {
        diagnostic.data.get("budget_kind") == Some(&DataValue::String("ir_items".into()))
    }));
}

#[test]
fn effect_fixpoint_charges_each_operation_and_callsite_to_global_limit() {
    let mut source = String::new();
    for index in 0..1022 {
        source.push_str(&format!(
            "fn F{index}(){{F{}();{}{}}}",
            (index + 1) % 1022,
            if index == 1021 { "Print(0);" } else { "" },
            ""
        ));
    }
    source.push_str("fn Main(){}");
    assert_eq!(source.len(), 18_240);
    let Err(CompileError::Rejected(errors)) = check_scalar("effects_limit.n", &source) else {
        panic!("the synchronous SCC ring exceeds the effects quota");
    };
    let quota = errors
        .iter()
        .find(|diagnostic| diagnostic.code == "NX-E0003")
        .expect("effects quota");
    assert_eq!(quota.stage, crate::diagnostic::Stage::Effects);
    assert_eq!(
        quota.data.get("budget_kind"),
        Some(&DataValue::String("effect_fixpoint_steps".into()))
    );
    assert_eq!(
        quota.data.get("budget_scope"),
        Some(&DataValue::String("compilation".into()))
    );
    assert_eq!(quota.data.get("limit"), Some(&DataValue::UInt(1_000_000)));
    assert_eq!(
        quota.data.get("observed"),
        Some(&DataValue::UInt(1_000_001))
    );
    quota.validate("effects_limit.n", &source).unwrap();
}

#[test]
fn effect_program_accepts_exact_budget_and_rejects_first_excess() {
    let source = |count: usize| {
        (0..count)
            .map(|index| format!("fn F{index}(){{}}"))
            .collect::<String>()
            + "fn Main(){}"
    };
    let exact = source(31);
    let program = compile_scalar("effects_exact.n", &exact).unwrap();
    let emitted = program
        .functions
        .functions()
        .iter()
        .map(|function| function.block.instructions.len())
        .sum::<usize>();
    assert_eq!(emitted, 32, "one effect-work unit per empty function");
    compile_scalar_with_profile("effects_exact.n", &exact, ReducedProfile::Effects32).unwrap();

    let excess = source(32);
    let Err(CompileError::Rejected(errors)) =
        compile_scalar_with_profile("effects_excess.n", &excess, ReducedProfile::Effects32)
    else {
        panic!("the first program unit beyond the effect limit must reject");
    };
    let diagnostic = errors
        .iter()
        .find(|error| error.code == "NX-E0003")
        .unwrap();
    assert_eq!(diagnostic.data.get("limit"), Some(&DataValue::UInt(32)));
    assert_eq!(diagnostic.data.get("observed"), Some(&DataValue::UInt(33)));
}

#[test]
fn reduced_analysis_profiles_are_selected_through_internal_rust_api() {
    let nested = |functions: usize| {
        (0..functions)
            .map(|index| {
                format!(
                    "fn F{index}(b:view Bool){{{}{}}}",
                    "while b{".repeat(8),
                    "}".repeat(8)
                )
            })
            .collect::<String>()
            + "fn Main(){}"
    };
    let Err(CompileError::Rejected(local)) = compile_scalar_with_profile(
        "profile_local.n",
        &nested(1),
        ReducedProfile::FlowFunction32,
    ) else {
        panic!("function flow profile must enforce its reduced limit");
    };
    let local = local.iter().find(|d| d.code == "NX-E0003").unwrap();
    assert_eq!(local.profile_id, "f0-0002-b-test-flow-32-v2");
    assert_eq!(
        local.data.get("budget_scope"),
        Some(&DataValue::String("function".into()))
    );
    assert_eq!(local.data.get("limit"), Some(&DataValue::UInt(32)));
    local.validate("profile_local.n", &nested(1)).unwrap();

    let source = nested(8);
    let Err(CompileError::Rejected(global)) = compile_scalar_with_profile(
        "profile_global.n",
        &source,
        ReducedProfile::FlowCompilation32,
    ) else {
        panic!("compilation flow profile must enforce its reduced limit");
    };
    let global = global.iter().find(|d| d.code == "NX-E0003").unwrap();
    assert_eq!(global.profile_id, "f0-0002-b-test-flow-global-32-v2");
    assert_eq!(
        global.data.get("budget_scope"),
        Some(&DataValue::String("compilation".into()))
    );
    assert_eq!(global.data.get("limit"), Some(&DataValue::UInt(32)));
    assert!(!global.data.contains_key("analysis_function"));
    global.validate("profile_global.n", &source).unwrap();

    let mut wrong_local_scope = local.clone();
    wrong_local_scope.data.insert(
        "budget_scope".into(),
        DataValue::String("compilation".into()),
    );
    wrong_local_scope.data.remove("analysis_function");
    assert!(
        wrong_local_scope
            .validate("profile_local.n", &nested(1))
            .is_err()
    );

    let mut wrong_global_scope = global.clone();
    wrong_global_scope
        .data
        .insert("budget_scope".into(), DataValue::String("function".into()));
    wrong_global_scope.data.insert(
        "analysis_function".into(),
        DataValue::Object(std::collections::BTreeMap::from([
            ("kind".into(), DataValue::String("function".into())),
            ("id".into(), DataValue::String("function:F0".into())),
        ])),
    );
    assert!(
        wrong_global_scope
            .validate("profile_global.n", &source)
            .is_err()
    );

    let mut effects = String::new();
    for index in 0..40 {
        effects.push_str(&format!(
            "fn F{index}(){{{}{}}}",
            if index < 39 {
                format!("F{}();", index + 1)
            } else {
                "Print(0);".into()
            },
            ""
        ));
    }
    effects.push_str("fn Main(){}");
    let Err(CompileError::Rejected(errors)) =
        compile_scalar_with_profile("profile_effects.n", &effects, ReducedProfile::Effects32)
    else {
        panic!("effects profile must enforce its reduced limit");
    };
    let effects_quota = errors.iter().find(|d| d.code == "NX-E0003").unwrap();
    assert_eq!(effects_quota.profile_id, "f0-0002-b-test-effects-32-v2");
    assert_eq!(effects_quota.data.get("limit"), Some(&DataValue::UInt(32)));
    effects_quota
        .validate("profile_effects.n", &effects)
        .unwrap();

    let ir = "fn Main(){let a=0;let b=1;let c=2;let d=3;let e=4;}";
    let Err(CompileError::Rejected(ir_errors)) =
        compile_scalar_with_profile("profile_ir.n", ir, ReducedProfile::IrItems32)
    else {
        panic!("IR profile must enforce its reduced limit");
    };
    let ir_quota = ir_errors.iter().find(|d| d.code == "NX-E0003").unwrap();
    assert_eq!(ir_quota.profile_id, "f0-0002-b-test-ir-32-v2");
    assert_eq!(
        ir_quota.data.get("budget_kind"),
        Some(&DataValue::String("ir_items".into()))
    );
    assert_eq!(ir_quota.data.get("limit"), Some(&DataValue::UInt(32)));
    ir_quota.validate("profile_ir.n", ir).unwrap();
}

#[test]
fn sparse_ownership_matches_live_lowering_tracker_without_loops() {
    let sources = [
        "fn Main(){let x=1;Print(x);}",
        "fn F(){let r=MockOpen(1);Drop(r);Drop(r);}",
        "fn F(b:view Bool){var r=MockOpen(1);if b{Drop(r);}Inspect(r);}",
        "fn F(b:view Bool){let x=if b then 1 else 2;Print(x);}",
    ];
    for (index, source) in sources.into_iter().enumerate() {
        let file = format!("ownership_differential_{index}.n");
        let tracker = check_scalar(&file, source);
        let sparse = compile_scalar_with_sparse_ownership(&file, source);
        assert_eq!(format!("{tracker:?}"), format!("{sparse:?}"), "{source}");
    }
}

#[test]
fn verifier_and_effect_work_scale_with_branch_count() {
    let work = |source: &str| {
        let (result, units) =
            super::measure_cfg_thread_work(|| compile_scalar("i10a_scale.n", source));
        result.unwrap();
        units
    };
    let ret = |count: usize| {
        format!(
            "fn F(b:view Bool){{{}{}}}fn Main(){{}}",
            "if b{return;}".repeat(count),
            ""
        )
    };
    let mut ret_units = Vec::new();
    for count in [750, 1500, 3000] {
        ret_units.push(work(&ret(count)));
    }
    eprintln!("I10a return branch CFG units: {ret_units:?}");
    assert!(
        ret_units[1] as f64 <= ret_units[0] as f64 * 2.6,
        "{ret_units:?}"
    );
    assert!(
        ret_units[2] as f64 <= ret_units[1] as f64 * 2.6,
        "{ret_units:?}"
    );

    let array = |count: usize| {
        format!(
            "fn F(b:view Bool){{let a=[{}];}}fn Main(){{}}",
            vec!["(if b then 1 else 2)"; count].join(",")
        )
    };
    let mut array_units = Vec::new();
    for count in [400, 800, 1600] {
        array_units.push(work(&array(count)));
    }
    eprintln!("I10a array branch CFG units: {array_units:?}");
    assert!(
        array_units[1] as f64 <= array_units[0] as f64 * 2.6,
        "{array_units:?}"
    );
    assert!(
        array_units[2] as f64 <= array_units[1] as f64 * 2.6,
        "{array_units:?}"
    );

    let resource_returns = |count: usize| {
        let mut source = String::from("fn F(b:view Bool){");
        for index in 0..count {
            source.push_str(&format!(
                "let r{index}=MockOpen(1);if b{{Drop(r{index});return;}}"
            ));
        }
        source.push_str("}fn Main(){}");
        source
    };
    let mut resource_units = Vec::new();
    for count in [64, 128, 256] {
        resource_units.push(work(&resource_returns(count)));
    }
    eprintln!("I10a resource return CFG units: {resource_units:?}");
    assert!(
        resource_units[1] <= resource_units[0] * 3,
        "{resource_units:?}"
    );
    assert!(
        resource_units[2] <= resource_units[1] * 3,
        "{resource_units:?}"
    );
}

#[test]
fn verifier_work_scales_with_sparse_loan_temporaries() {
    let work = |source: &str| {
        let (result, units) =
            super::measure_cfg_thread_work(|| compile_scalar("i10a_loan_scale.n", source));
        result.unwrap();
        units
    };
    let units = [24, 48, 96].map(|count| {
        let mut source =
            String::from("fn Pred(x:view Array<I64>)->Bool{return true;}fn F(a:move Array<I64>){");
        source.push_str(&"if Pred(a){}".repeat(count));
        source.push_str("}fn Main(){}");
        work(&source)
    });
    eprintln!("I10a sparse loan temporary CFG units: {units:?}");
    assert!(units[1] as f64 <= units[0] as f64 * 2.65, "{units:?}");
    assert!(units[2] as f64 <= units[1] as f64 * 2.65, "{units:?}");
}

#[test]
fn exit_scan_work_scales_with_droppable_returns() {
    let work = |source: &str| {
        let (result, units) =
            super::measure_cfg_thread_work(|| compile_scalar("i10a_exit_scan_scale.n", source));
        let program = result.unwrap();
        let drop_locals = program
            .functions
            .functions()
            .iter()
            .flat_map(|function| std::iter::once(&function.block).chain(&function.blocks))
            .flat_map(|block| &block.instructions)
            .filter(|instruction| matches!(instruction.op, super::model::Op::DropLocal(_)))
            .count();
        assert!(drop_locals > 0, "source produced no DropLocal instructions");
        units
    };
    let units = [24, 48, 96].map(|count| {
        let mut source = String::from("fn F(b:view Bool){");
        for index in 0..count {
            source.push_str(&format!("if b{{let x{index}={index};}}"));
        }
        source.push_str("}fn Main(){}");
        work(&source)
    });
    eprintln!("I10a droppable return CFG units: {units:?}");
    assert!(units[1] as f64 <= units[0] as f64 * 2.1, "{units:?}");
    assert!(units[2] as f64 <= units[1] as f64 * 2.1, "{units:?}");
}

#[test]
#[cfg(not(debug_assertions))]
#[ignore = "release timing campaign; t(24000)/t(3000) takes over four seconds locally"]
fn verifier_effect_release_time_campaign_records_branch_ratios() {
    let measure = |count: usize| {
        let source = format!(
            "fn F(b:view Bool){{{}{}}}fn Main(){{}}",
            "if b{return;}".repeat(count),
            ""
        );
        super::lower::compile_scalar_with_unbounded_ir_for_scale("i10a_branch_timing.n", &source)
            .unwrap();
        let mut samples = Vec::new();
        for _ in 0..3 {
            let began = std::time::Instant::now();
            super::lower::compile_scalar_with_unbounded_ir_for_scale(
                "i10a_branch_timing.n",
                &source,
            )
            .unwrap();
            samples.push(began.elapsed().as_secs_f64());
        }
        samples.sort_by(f64::total_cmp);
        samples[1]
    };
    let small = measure(3_000);
    let large = measure(24_000);
    let ratio = large / small.max(0.001);
    eprintln!(
        "I10a release if-return t(3000)/t(24000), medians of 3: {small:.6}s/{large:.6}s = {ratio:.3}"
    );
    // Umbral calibrado: el control lineal da ~9,2-9,4 (efectos de cache); una reposicion
    // cuadratica da >= 15 (S1/S2). Una reposicion marginal (S4, ~12,3) queda en el borde.
    assert!(ratio < 12.0, "release time ratio={ratio:.3}");
}
