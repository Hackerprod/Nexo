use std::borrow::Cow;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use super::model::{
    self, Access, BasicBlock, Binary, BlockId, CaptureKind, Function, FunctionId, FunctionTable,
    IndexSite, Instruction, Intrinsic, LoanId, LoanPermission, Local, LocalId, Op, OriginId,
    Parameter, ParameterMode, Program, ScopeId, TempId, Terminator, Ty, Unary, WorkCharge,
};
#[path = "lower_match.rs"]
mod lower_match;
use super::work::{WorkEvent, WorkPlanner};
use crate::check::types::ResolvedType;
use crate::diagnostic::{self, Diagnostic, PROFILE_ID, Span};
use crate::frontend::lex;
use crate::frontend::parse::{self, Ast, Expr, Mode, Statement, Type};

#[cfg(test)]
pub(super) static LOAN_WORK: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
#[cfg(test)]
fn count_loan_work() {
    LOAN_WORK.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
}

#[cfg(test)]
mod lowering_stack_regression {
    use super::*;
    use std::process::Command;

    #[test]
    fn else_if_child_lowers_with_204_kib() {
        if std::env::var_os("J16_LOWER_ELSE_IF_CHILD").is_none() {
            return;
        }
        let source = format!("fn Main(){{{}{}}}", "if true{}else ".repeat(126), "{}");
        let ast = parse::parse_new_profile("else-if-stack.n", &source).unwrap();
        let checked = crate::check::check_for_compilation("else-if-stack.n", &source, &ast, false);
        assert!(checked.errors.is_empty(), "{:?}", checked.errors);
        let lowered = std::thread::scope(|scope| {
            std::thread::Builder::new()
                .stack_size(204 * 1024)
                .spawn_scoped(scope, || {
                    compile_ast_with_flow_limit(
                        &ast,
                        &source,
                        &checked.inferred,
                        AnalysisLimits::NORMAL,
                    )
                })
                .unwrap()
                .join()
                .unwrap()
        });
        assert!(lowered.is_ok(), "{lowered:?}");
    }

    #[test]
    fn else_if_lowers_with_204_kib() {
        let output = Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "ir::lower::lowering_stack_regression::else_if_child_lowers_with_204_kib",
                "--nocapture",
            ])
            .env("J16_LOWER_ELSE_IF_CHILD", "1")
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{} {}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }

    fn family_source(family: &str) -> String {
        match family {
            "external" => {
                std::fs::read_to_string(std::env::var("J16_LOWER_SOURCE_FILE").unwrap()).unwrap()
            }
            "print_binary" => "fn Main(){Print(1+2);}".into(),
            "index_nested" => format!(
                "fn Main(){{let a=[1];let x={}0{};}}",
                "a[".repeat(126),
                "]".repeat(126)
            ),
            "edit_nested" => format!(
                "fn G(p:edit I64)->I64{{return 1;}}fn Main(){{var a=[1];let y={}0{};}}",
                "G(edit a[".repeat(40),
                "])".repeat(40)
            ),
            "alternating_binary_call" => {
                let mut expr = "1".to_owned();
                for _ in 0..63 {
                    expr = format!("(F({expr}))-2");
                }
                format!("fn F(x:view I64)->I64{{return x;}}fn Main(){{let y={expr};}}")
            }
            "block" => format!(
                "fn Main(){{{}let x=1;{}}}",
                "{".repeat(126),
                "}".repeat(126)
            ),
            "group" => format!(
                "fn Main(){{let x={}1{};}}",
                "(".repeat(126),
                ")".repeat(126)
            ),
            "array" => format!(
                "fn Main(){{let x={}1{};}}",
                "[".repeat(126),
                "]".repeat(126)
            ),
            "binary_left" => format!("fn Main(){{let x=1{};}}", "+1".repeat(126)),
            "binary_right" => format!(
                "fn Main(){{let x={}1{};}}",
                "1+(".repeat(126),
                ")".repeat(126)
            ),
            "if_stmt" => format!("fn Main(){{{}{}}}", "if true{".repeat(126), "}".repeat(126)),
            "while" => format!(
                "fn Main(){{{}{}}}",
                "while true{".repeat(126),
                "}".repeat(126)
            ),
            "while_if" => format!(
                "fn Main(){{{}{}}}",
                "while false{if true{".repeat(63),
                "}".repeat(126)
            ),
            "if_expr" => {
                let mut value = String::from("1");
                for _ in 0..126 {
                    value = format!("if true then ({value}) else 0");
                }
                format!("fn Main(){{let x={value};}}")
            }
            "else_if_expr" => {
                let mut value = String::from("1");
                for _ in 0..63 {
                    value = format!("if false then 0 else ({value})");
                }
                format!("fn Main(){{let x={value};}}")
            }
            "call_arg" => format!(
                "fn F(x:view I64)->I64{{return x;}}fn Main(){{let x={}1{};}}",
                "F(".repeat(126),
                ")".repeat(126)
            ),
            "some_expr" => format!(
                "fn Main(){{let x={}1{};}}",
                "Some(".repeat(126),
                ")".repeat(126)
            ),
            "some_annotated" => format!(
                "fn Main(){{let x:{}I64{}={}1{};}}",
                "Option<".repeat(126),
                ">".repeat(126),
                "Some(".repeat(126),
                ")".repeat(126)
            ),
            "pattern_ctor" => format!(
                "fn Main(){{let x={}1{};match x{{{}_{}=>{{}},_=>{{}}}}}}",
                "Some(".repeat(126),
                ")".repeat(126),
                "Some(".repeat(126),
                ")".repeat(126)
            ),
            "tryadd_arg" => {
                let mut value = String::from("1");
                for _ in 0..60 {
                    value = format!("TryAdd({value},1)?");
                }
                format!("fn Main()->Result<(),I64>{{let x={value};return Ok(());}}")
            }
            "index" => format!(
                "fn Main(){{let a={}1{};let x=a{};}}",
                "[".repeat(126),
                "]".repeat(126),
                "[0]".repeat(126)
            ),
            "match_stmt" => {
                let mut body = String::new();
                for _ in 0..60 {
                    body = format!("match 1{{_=>{{{body}}}}}");
                }
                format!("fn Main(){{{body}}}")
            }
            "match_dead" => {
                let mut body = String::new();
                for _ in 0..60 {
                    body = format!("match 1{{_=>{{{body}}}}}");
                }
                format!("fn Main(){{return;{body}}}")
            }
            "match_expr" => {
                let mut value = String::from("1");
                for _ in 0..63 {
                    value = format!("match 1{{_=>{{value:{value};}}}}");
                }
                format!("fn Main(){{let x={value};}}")
            }
            "match_scrutinee" => {
                let mut value = String::from("1");
                for _ in 0..60 {
                    value = format!("match ({value}){{_=>{{value:1;}}}}");
                }
                format!("fn Main(){{let x={value};}}")
            }
            "closure" => format!(
                "fn Main(){{let f={}1{};}}",
                "fn(){let g=".repeat(63),
                ";}".repeat(63)
            ),
            "type" => format!(
                "fn F(x:view {}I64{}){{}}fn Main(){{}}",
                "Array<".repeat(126),
                ">".repeat(126)
            ),
            "pattern" => [
                "fn Main(){match 1{".to_string(),
                "(".repeat(126),
                "_".into(),
                ")".repeat(126),
                "=>{},}}".into(),
            ]
            .concat(),
            "mixed" => format!(
                "fn Main(){{{}let x={}1{};{}}}",
                "{".repeat(63),
                "(".repeat(63),
                ")".repeat(63),
                "}".repeat(63)
            ),
            "yield_group" => format!(
                "gen fn G()->I64{{yield {}1{};}}fn Main(){{}}",
                "(".repeat(126),
                ")".repeat(126)
            ),
            "closure_called" => {
                let mut value = String::from("1");
                for _ in 0..40 {
                    value = format!("(fn ()->I64{{return {value};}})()");
                }
                format!("fn Main(){{let x={value};}}")
            }
            _ => unreachable!(),
        }
    }

    #[test]
    fn family_child_lowers_with_204_kib() {
        let Ok(family) = std::env::var("J16_LOWER_FAMILY_CHILD") else {
            return;
        };
        let source = family_source(&family);
        let ast = parse::parse_new_profile("family-stack.n", &source).unwrap();
        let checked = crate::check::check_for_compilation_with_type_limit(
            "family-stack.n",
            &source,
            &ast,
            false,
            1_000_000,
            "f0-0003-b-v1",
        );
        assert!(checked.errors.is_empty(), "{family}: {:?}", checked.errors);
        let stack_kib = std::env::var("J16_LOWER_STACK_KIB")
            .map(|value| value.parse::<usize>().unwrap())
            .unwrap_or(204);
        let lowered = std::thread::scope(|scope| {
            std::thread::Builder::new()
                .stack_size(stack_kib * 1024)
                .spawn_scoped(scope, || {
                    compile_ast_with_flow_limit(
                        &ast,
                        &source,
                        &checked.inferred,
                        AnalysisLimits::NORMAL,
                    )
                })
                .unwrap()
                .join()
                .unwrap()
        });
        assert!(lowered.is_ok(), "{family}: {lowered:?}");
        let program = lowered.unwrap();
        for function in program.functions.functions() {
            crate::ir::verify::verify_function(function).unwrap();
        }
        crate::ir::verify::verify_program(&program).unwrap();
        drop(program);
    }

    #[test]
    fn families_lower_with_204_kib() {
        for family in [
            "print_binary",
            "index_nested",
            "edit_nested",
            "alternating_binary_call",
            "block",
            "group",
            "array",
            "binary_left",
            "binary_right",
            "if_stmt",
            "else_if_expr",
            "if_expr",
            "call_arg",
            "some_expr",
            "some_annotated",
            "pattern_ctor",
            "tryadd_arg",
            "while",
            "while_if",
            "match_stmt",
            "match_dead",
            "match_expr",
            "match_scrutinee",
            "index",
            "type",
            "pattern",
            "closure",
            "mixed",
            "yield_group",
            "closure_called",
        ] {
            let output = Command::new(std::env::current_exe().unwrap())
                .args([
                    "--exact",
                    "ir::lower::lowering_stack_regression::family_child_lowers_with_204_kib",
                    "--nocapture",
                ])
                .env("J16_LOWER_FAMILY_CHILD", family)
                .env("J16_LOWER_STACK_ISOLATE_VERIFY", "1")
                .output()
                .unwrap();
            assert!(
                output.status.success(),
                "{family}: {} {}",
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr)
            );
        }
    }
}
#[cfg(not(test))]
fn count_loan_work() {}

#[derive(Debug)]
// InvalidIr is a guard on an invariant, never a source diagnostic. The
// source path is parse -> names/types -> lowering -> verify_function
// -> verify_program. The source and forged-IR tests below exercise both sides
// of every guard family; see the per-site inventory in the D15 implementation
// notes. The exclusive-prefix case is screened in parse::function.
//
// Lexical/AST invariants: binary operation (both sites), parameter name token,
// declaration token, unvalidated integer, assignment root, call target,
// partial transfer, edit place passed parser, nonlocal place.
// Semantic invariants: unresolved local, edit mode passed checker,
// non-callable target passed checker, call arity/label/argument type passed
// checker, duplicate call argument, call parameter slot, intrinsic arity,
// user or deferred call, assignment/return type passed checker, resource or
// result type, array element type, index type/target type (all sites), loan
// index/target type, if/while condition type (all sites).
// Flow/scope invariants: control-flow return analysis, function return flow,
// return flow, branch value type/slot, if/while condition flow (all sites),
// else if form, scope stack/name (all sites), availability undo, scope restore
// length, binding/assignment/expression/return value flow, control flow,
// binary/index operand flow, exclusive loan expression.
// Infrastructure/verified-IR invariants: lowering worker unavailable/panicked,
// verify_function and verify_program adapters. Tests: semantic_cfg_sources_
// never_create_invalid_ir, semantic_loan_fuzz_never_constructs_invalid_ir,
// array_sources_with_mutated_syntax_never_panic_and_validate_rejections,
// access_prefix_contexts_never_escape_as_invalid_ir, unicode_sources_never_
// reach_internal_failure, and ir::verify forged-function tests.
pub enum CompileError {
    Rejected(Vec<Box<Diagnostic>>),
    InvalidIr(&'static str),
    Dependent,
}

#[derive(Clone, Copy)]
struct AnalysisLimits {
    type_solver_steps: u64,
    flow_function: u64,
    flow_compilation: u64,
    effects: u64,
    ir_items: u64,
    profile_id: &'static str,
    force_sparse_ownership: bool,
    literal_true_loops: bool,
}

impl AnalysisLimits {
    const NORMAL: Self = Self {
        type_solver_steps: 1_000_000,
        flow_function: 1_000_000,
        flow_compilation: 4_000_000,
        effects: 1_000_000,
        ir_items: 262_144,
        profile_id: PROFILE_ID,
        force_sparse_ownership: false,
        literal_true_loops: false,
    };
}

#[derive(Clone, Copy)]
pub enum ReducedProfile {
    Normal,
    JNormal,
    F5Normal,
    /// Internal F5 control flow probe; deliberately absent from `from_id`.
    F5ControlInternal,
    Types32,
    JTypes32,
    FlowFunction32,
    JFlowFunction32,
    FlowCompilation32,
    JFlowCompilation32,
    Effects32,
    JEffects32,
    IrItems32,
    JIrItems32,
    Frames8,
    JFrames8,
    Cells64,
    JCells64,
    Observations64,
    JObservations64,
}
impl ReducedProfile {
    pub fn from_id(id: &str) -> Option<Self> {
        Some(match id {
            "f0-0002-b-v2" => Self::Normal,
            "f0-0002-b-test-types-32-v2" => Self::Types32,
            "f0-0002-b-test-flow-32-v2" => Self::FlowFunction32,
            "f0-0002-b-test-flow-global-32-v2" => Self::FlowCompilation32,
            "f0-0002-b-test-effects-32-v2" => Self::Effects32,
            "f0-0002-b-test-ir-32-v2" => Self::IrItems32,
            "f0-0002-b-test-frames-8-v2" => Self::Frames8,
            "f0-0002-b-test-cells-64-v2" => Self::Cells64,
            "f0-0002-b-test-observations-64-v2" => Self::Observations64,
            "f0-0003-b-v1" => Self::JNormal,
            "f0-0005-b-v1" => Self::F5Normal,
            "f0-0003-b-test-types-32-v1" => Self::JTypes32,
            "f0-0003-b-test-flow-32-v1" => Self::JFlowFunction32,
            "f0-0003-b-test-flow-global-32-v1" => Self::JFlowCompilation32,
            "f0-0003-b-test-effects-32-v1" => Self::JEffects32,
            "f0-0003-b-test-ir-32-v1" => Self::JIrItems32,
            "f0-0003-b-test-frames-8-v1" => Self::JFrames8,
            "f0-0003-b-test-cells-64-v1" => Self::JCells64,
            "f0-0003-b-test-observations-64-v1" => Self::JObservations64,
            _ => return None,
        })
    }
}

fn analysis_limits(profile: ReducedProfile) -> AnalysisLimits {
    match profile {
        ReducedProfile::Normal => AnalysisLimits::NORMAL,
        ReducedProfile::JNormal => AnalysisLimits {
            profile_id: "f0-0003-b-v1",
            ..AnalysisLimits::NORMAL
        },
        ReducedProfile::F5Normal => AnalysisLimits {
            profile_id: "f0-0005-b-v1",
            literal_true_loops: true,
            ..AnalysisLimits::NORMAL
        },
        ReducedProfile::F5ControlInternal => AnalysisLimits {
            profile_id: "f0-0003-b-v1",
            literal_true_loops: true,
            ..AnalysisLimits::NORMAL
        },
        ReducedProfile::Types32 => AnalysisLimits {
            type_solver_steps: 32,
            profile_id: "f0-0002-b-test-types-32-v2",
            ..AnalysisLimits::NORMAL
        },
        ReducedProfile::JTypes32 => AnalysisLimits {
            type_solver_steps: 32,
            profile_id: "f0-0003-b-test-types-32-v1",
            ..AnalysisLimits::NORMAL
        },
        ReducedProfile::FlowFunction32 => AnalysisLimits {
            flow_function: 32,
            profile_id: "f0-0002-b-test-flow-32-v2",
            ..AnalysisLimits::NORMAL
        },
        ReducedProfile::JFlowFunction32 => AnalysisLimits {
            flow_function: 32,
            profile_id: "f0-0003-b-test-flow-32-v1",
            ..AnalysisLimits::NORMAL
        },
        ReducedProfile::FlowCompilation32 => AnalysisLimits {
            flow_compilation: 32,
            profile_id: "f0-0002-b-test-flow-global-32-v2",
            ..AnalysisLimits::NORMAL
        },
        ReducedProfile::JFlowCompilation32 => AnalysisLimits {
            flow_compilation: 32,
            profile_id: "f0-0003-b-test-flow-global-32-v1",
            ..AnalysisLimits::NORMAL
        },
        ReducedProfile::Effects32 => AnalysisLimits {
            effects: 32,
            profile_id: "f0-0002-b-test-effects-32-v2",
            ..AnalysisLimits::NORMAL
        },
        ReducedProfile::JEffects32 => AnalysisLimits {
            effects: 32,
            profile_id: "f0-0003-b-test-effects-32-v1",
            ..AnalysisLimits::NORMAL
        },
        ReducedProfile::IrItems32 => AnalysisLimits {
            ir_items: 32,
            profile_id: "f0-0002-b-test-ir-32-v2",
            ..AnalysisLimits::NORMAL
        },
        ReducedProfile::JIrItems32 => AnalysisLimits {
            ir_items: 32,
            profile_id: "f0-0003-b-test-ir-32-v1",
            ..AnalysisLimits::NORMAL
        },
        ReducedProfile::Frames8 => AnalysisLimits {
            profile_id: "f0-0002-b-test-frames-8-v2",
            ..AnalysisLimits::NORMAL
        },
        ReducedProfile::JFrames8 => AnalysisLimits {
            profile_id: "f0-0003-b-test-frames-8-v1",
            ..AnalysisLimits::NORMAL
        },
        ReducedProfile::Cells64 => AnalysisLimits {
            profile_id: "f0-0002-b-test-cells-64-v2",
            ..AnalysisLimits::NORMAL
        },
        ReducedProfile::JCells64 => AnalysisLimits {
            profile_id: "f0-0003-b-test-cells-64-v1",
            ..AnalysisLimits::NORMAL
        },
        ReducedProfile::Observations64 => AnalysisLimits {
            profile_id: "f0-0002-b-test-observations-64-v2",
            ..AnalysisLimits::NORMAL
        },
        ReducedProfile::JObservations64 => AnalysisLimits {
            profile_id: "f0-0003-b-test-observations-64-v1",
            ..AnalysisLimits::NORMAL
        },
    }
}

#[cfg(test)]
pub(crate) fn analysis_limits_for_test(
    profile: ReducedProfile,
) -> (&'static str, u64, u64, u64, u64, u64) {
    let limits = analysis_limits(profile);
    (
        limits.profile_id,
        limits.type_solver_steps,
        limits.flow_function,
        limits.flow_compilation,
        limits.effects,
        limits.ir_items,
    )
}

/// Produces an opaque, verified artifact. No interpreter entry accepts an AST.
pub fn compile_scalar(file: &str, source: &str) -> Result<Program, CompileError> {
    compile_scalar_with_entry(file, source, true)
}

/// Check and verify a library without requiring an entry point.
pub fn check_scalar(file: &str, source: &str) -> Result<Program, CompileError> {
    compile_scalar_with_entry(file, source, false)
}

#[cfg(test)]
pub(crate) fn compile_scalar_with_flow_limit(
    file: &str,
    source: &str,
    limit: u64,
) -> Result<Program, CompileError> {
    let limits = AnalysisLimits {
        flow_function: limit,
        ..AnalysisLimits::NORMAL
    };
    // This test hook intentionally uses a quota that has no published profile.
    // Production entries always pass through diagnostic schema validation.
    compile_scalar_with_analysis_limits_unchecked(file, source, false, limits)
}

pub fn compile_scalar_with_profile(
    file: &str,
    source: &str,
    profile: ReducedProfile,
    run_entry: bool,
) -> Result<Program, CompileError> {
    let limits = analysis_limits(profile);
    compile_scalar_with_analysis_limits(file, source, run_entry, limits)
}

#[cfg(test)]
pub(crate) fn compile_scalar_with_sparse_ownership(
    file: &str,
    source: &str,
) -> Result<Program, CompileError> {
    compile_scalar_with_analysis_limits(
        file,
        source,
        false,
        AnalysisLimits {
            force_sparse_ownership: true,
            ..AnalysisLimits::NORMAL
        },
    )
}

fn compile_scalar_with_entry(
    file: &str,
    source: &str,
    run_entry: bool,
) -> Result<Program, CompileError> {
    compile_scalar_with_analysis_limits(file, source, run_entry, AnalysisLimits::NORMAL)
}

fn compile_scalar_with_analysis_limits(
    file: &str,
    source: &str,
    run_entry: bool,
    limits: AnalysisLimits,
) -> Result<Program, CompileError> {
    let mut result = compile_scalar_with_analysis_limits_unchecked(file, source, run_entry, limits);
    if let Err(CompileError::Rejected(errors)) = &mut result {
        for error in errors {
            error.profile_id = limits.profile_id.into();
            if let Err(reason) = error.publish(file, source) {
                eprintln!("invalid compiler diagnostic: {reason}");
                return Err(CompileError::InvalidIr("invalid compiler diagnostic"));
            }
        }
    }
    result
}

fn compile_scalar_with_analysis_limits_unchecked(
    file: &str,
    source: &str,
    run_entry: bool,
    limits: AnalysisLimits,
) -> Result<Program, CompileError> {
    let ast = if limits.profile_id.starts_with("f0-0005-") {
        parse::parse_f5_hidden(file, source)
    } else if limits.profile_id.starts_with("f0-0003-") {
        parse::parse_new_profile(file, source)
    } else {
        parse::parse(file, source)
    }
    .map_err(|d| CompileError::Rejected(vec![d]))?;
    let checked = crate::check::check_for_compilation_with_type_limit(
        file,
        source,
        &ast,
        false,
        limits.type_solver_steps,
        limits.profile_id,
    );
    if let Some(reason) = checked.internal_error {
        let _ = reason;
        return Err(CompileError::InvalidIr("internal_error"));
    }
    let inferred = &checked.inferred;
    if !inferred.closures.is_empty() && !checked.errors.is_empty() {
        return Err(CompileError::Rejected(checked.errors));
    }
    #[cfg(test)]
    let measure_cfg = super::cfg_measure_active();
    let lowered = std::thread::scope(|scope| {
        std::thread::Builder::new()
            .stack_size(crate::frontend_stack_size("LOWERING"))
            .spawn_scoped(scope, || {
                #[cfg(test)]
                if measure_cfg {
                    return super::measure_cfg_thread_work(|| {
                        compile_ast_with_flow_limit(&ast, source, inferred, limits)
                    });
                }
                #[cfg(test)]
                {
                    (
                        compile_ast_with_flow_limit(&ast, source, inferred, limits),
                        0,
                    )
                }
                #[cfg(not(test))]
                {
                    compile_ast_with_flow_limit(&ast, source, inferred, limits)
                }
            })
            .map_err(|_| CompileError::InvalidIr("lowering worker unavailable"))?
            .join()
            .map_err(|_| CompileError::InvalidIr("lowering worker panicked"))
    })?;
    #[cfg(test)]
    let lowered = {
        let (result, steps) = lowered;
        super::cfg_forward_work(steps);
        result
    };
    if !checked.errors.is_empty() {
        // Branch value structure precedes type errors, even when a later
        // independent operand already failed in the checker.
        if let Err(CompileError::Rejected(lower_errors)) = lowered {
            let missing: Vec<_> = lower_errors
                .into_iter()
                .filter(|d| {
                    d.code == "NX-E1302" && d.subcause.as_deref() == Some("missing_branch_value")
                })
                .collect();
            if !missing.is_empty() {
                return Err(CompileError::Rejected(missing));
            }
        }
        return Err(CompileError::Rejected(checked.errors));
    }
    let program = literal_true_barrier(file, source, limits, &ast, lowered)?;
    if run_entry {
        let resolved = checked
            .resolved
            .as_ref()
            .ok_or(CompileError::InvalidIr("resolved names unavailable"))?;
        crate::check::names::entry(file, source, resolved, &ast)
            .map_err(|error| CompileError::Rejected(vec![error]))?;
    }
    Ok(program)
}

/// F0-0005 §2.2 and §8.2: a literal-`true` loop is a new capability, so it meets the L08
/// barrier, exactly when its missing false edge changes the verdict. The verdict is the same
/// lowering run once more with the historical edge; there is no second analysis. The anchor is
/// the first such loop in a function where the historical rule reports an error, and it competes
/// with the other capabilities by source offset.
fn literal_true_barrier(
    file: &str,
    source: &str,
    limits: AnalysisLimits,
    ast: &Ast,
    lowered: Result<Program, CompileError>,
) -> Result<Program, CompileError> {
    if !ast.f5_operators || !limits.literal_true_loops {
        return lowered;
    }
    let barrier_only = match &lowered {
        Ok(_) => true,
        Err(CompileError::Rejected(errors)) => errors.iter().all(|d| is_l08_barrier(d)),
        Err(_) => false,
    };
    if !barrier_only {
        return lowered;
    }
    let loops = f5_capabilities(ast).literal_true_loops;
    if loops.is_empty() {
        return lowered;
    }
    let historical = compile_scalar_with_analysis_limits_unchecked(
        file,
        source,
        false,
        AnalysisLimits {
            literal_true_loops: false,
            ..limits
        },
    );
    let errors = match historical {
        Err(CompileError::Rejected(errors)) => errors,
        Err(other) => return Err(other),
        Ok(_) => return lowered,
    };
    let rejected_at: Vec<u64> = errors
        .iter()
        .filter(|d| !is_l08_barrier(d))
        .filter_map(|d| d.primary_span.as_ref().map(|span| span.start))
        .collect();
    if rejected_at.is_empty() {
        return lowered;
    }
    let in_rejected_function = |span: &Span| {
        ast.functions.iter().any(|function| {
            function.body.span.start <= span.start
                && span.start < function.body.span.end
                && rejected_at
                    .iter()
                    .any(|at| function.span.start <= *at && *at < function.span.end)
        })
    };
    let anchor = loops
        .into_iter()
        .filter(|span| in_rejected_function(span))
        .min_by_key(|span| span.start);
    let Some(anchor) = anchor else {
        return lowered;
    };
    if let Err(CompileError::Rejected(existing)) = &lowered
        && existing.iter().any(|d| {
            d.primary_span
                .as_ref()
                .is_some_and(|s| s.start <= anchor.start)
        })
    {
        return lowered;
    }
    Err(CompileError::Rejected(vec![l08_barrier(anchor)]))
}

#[cfg(test)]
#[allow(dead_code)] // Called by the release-only timing campaign.
/// Compile a scale fixture while bypassing only the normative IR-item cap.
/// These fixtures measure compiler work beyond the public profile's limit;
/// production entry points continue to use `AnalysisLimits::NORMAL`.
pub(super) fn compile_scalar_with_unbounded_ir_for_scale(
    file: &str,
    source: &str,
) -> Result<Program, CompileError> {
    compile_scalar_with_analysis_limits(
        file,
        source,
        false,
        AnalysisLimits {
            ir_items: u64::MAX,
            ..AnalysisLimits::NORMAL
        },
    )
}

#[cfg(test)]
pub(super) fn compile_ast(ast: &Ast, source_text: &str) -> Result<Program, CompileError> {
    // this test-only entry ignores checker errors and forwards only
    // inferred types. The run-entry flag affects only an emitted entry-point
    // diagnostic, not the inference consumed by this helper.
    let checked = crate::check::check_for_compilation(&ast.span.file, source_text, ast, false);
    if let Some(reason) = checked.internal_error {
        let _ = reason;
        return Err(CompileError::InvalidIr("internal_error"));
    }
    compile_ast_with_inference(ast, source_text, &checked.inferred)
}

#[cfg(test)]
fn compile_ast_with_inference(
    ast: &Ast,
    source_text: &str,
    inferred: &crate::check::types::InferredTypes,
) -> Result<Program, CompileError> {
    compile_ast_with_flow_limit(ast, source_text, inferred, AnalysisLimits::NORMAL)
}

pub(super) fn collect_closures(ast: &Ast) -> Vec<&parse::Closure> {
    enum Visit<'a> {
        Expr(&'a Expr),
        Statement(&'a Statement),
        Block(&'a parse::Block),
    }

    let mut out = Vec::new();
    let mut visits = Vec::new();
    for function in ast.functions.iter().rev() {
        visits.push(Visit::Block(&function.body));
    }
    while let Some(visit) = visits.pop() {
        match visit {
            Visit::Block(block) => {
                for statement in block.statements.iter().rev() {
                    visits.push(Visit::Statement(statement));
                }
            }
            Visit::Statement(statement) => match statement {
                Statement::Let { value, .. } | Statement::Expr { value, .. } => {
                    visits.push(Visit::Expr(value));
                }
                Statement::Assign { place, value, .. } => {
                    visits.push(Visit::Expr(value));
                    visits.push(Visit::Expr(place));
                }
                Statement::Return { value, .. }
                | Statement::GuardControl { value, .. }
                | Statement::Value { value, .. } => {
                    if let Some(value) = value {
                        visits.push(Visit::Expr(value));
                    }
                }
                Statement::If {
                    condition,
                    then_block,
                    otherwise,
                    ..
                } => {
                    if let Some(otherwise) = otherwise {
                        visits.push(Visit::Statement(otherwise));
                    }
                    visits.push(Visit::Block(then_block));
                    visits.push(Visit::Expr(condition));
                }
                Statement::While {
                    condition, body, ..
                }
                | Statement::For {
                    iterable: condition,
                    body,
                    ..
                } => {
                    visits.push(Visit::Block(body));
                    visits.push(Visit::Expr(condition));
                }
                Statement::Match(selection) => {
                    for arm in selection.arms.iter().rev() {
                        visits.push(Visit::Block(&arm.body));
                        if let Some(guard) = &arm.guard {
                            visits.push(Visit::Expr(guard));
                        }
                    }
                    visits.push(Visit::Expr(&selection.scrutinee));
                }
                Statement::Block(block) => visits.push(Visit::Block(block)),
            },
            Visit::Expr(expr) => match expr {
                Expr::Closure(closure) => {
                    out.push(closure.as_ref());
                    visits.push(Visit::Block(&closure.body));
                }
                Expr::Array(items, _) => {
                    for item in items.iter().rev() {
                        visits.push(Visit::Expr(item));
                    }
                }
                Expr::Group(inner, _)
                | Expr::Unary { value: inner, .. }
                | Expr::Neg { value: inner, .. }
                | Expr::Not { value: inner, .. } => {
                    visits.push(Visit::Expr(inner));
                }
                Expr::Binary { left, right, .. }
                | Expr::Index {
                    target: left,
                    index: right,
                    ..
                } => {
                    visits.push(Visit::Expr(right));
                    visits.push(Visit::Expr(left));
                }
                Expr::Call { target, args, .. } => {
                    for arg in args.iter().rev() {
                        visits.push(Visit::Expr(&arg.value));
                    }
                    visits.push(Visit::Expr(target));
                }
                Expr::If {
                    condition,
                    then_block,
                    otherwise,
                    ..
                } => {
                    match otherwise.as_deref() {
                        Some(parse::Else::Block(block)) => visits.push(Visit::Block(block)),
                        Some(parse::Else::If(expr)) => visits.push(Visit::Expr(expr)),
                        None => {}
                    }
                    visits.push(Visit::Block(then_block));
                    visits.push(Visit::Expr(condition));
                }
                Expr::Match(selection) => {
                    for arm in selection.arms.iter().rev() {
                        visits.push(Visit::Block(&arm.body));
                        if let Some(guard) = &arm.guard {
                            visits.push(Visit::Expr(guard));
                        }
                    }
                    visits.push(Visit::Expr(&selection.scrutinee));
                }
                Expr::Name(..) | Expr::Integer(..) | Expr::Bool(..) | Expr::Unit(..) => {}
            },
        }
    }
    out.sort_by_key(|closure| closure.fn_span.start);
    out
}

fn mode_candidate_consumed(
    diagnostic: &Diagnostic,
    local: LocalId,
    use_span: &Span,
    name_spans: &[Span],
) -> bool {
    diagnostic.code == "NX-E0202"
        && diagnostic.primary_span.as_ref() == Some(use_span)
        && diagnostic.related_spans.iter().any(|related| {
            related.role == diagnostic::RelatedRole::Declaration
                && related.span == name_spans[local.0]
        })
}

fn compile_ast_with_flow_limit<'a>(
    ast: &'a Ast,
    source_text: &str,
    inferred: &'a crate::check::types::InferredTypes,
    limits: AnalysisLimits,
) -> Result<Program, CompileError> {
    struct SourceFunction<'a> {
        is_generator: bool,
        name: Cow<'a, str>,
        name_span: &'a Span,
        params: Cow<'a, [parse::Parameter]>,
        result: Option<&'a Type>,
        body: &'a parse::Block,
        span: &'a Span,
    }
    #[cfg(test)]
    let isolate_lowering = std::env::var_os("J16_LOWER_STACK_ISOLATE_VERIFY").is_some();
    #[cfg(not(test))]
    let isolate_lowering = false;
    #[cfg(test)]
    let _allocation_capture = crate::test_alloc::capture_for(&ast.span.file);
    let source_text: std::sync::Arc<str> = std::sync::Arc::from(source_text);
    let tokens = Arc::new(
        lex::lex(&ast.span.file, &source_text).map_err(|d| CompileError::Rejected(vec![d]))?,
    );
    let mut all_functions: Vec<SourceFunction<'a>> = ast
        .functions
        .iter()
        .map(|function| SourceFunction {
            is_generator: function.is_generator,
            name: Cow::Borrowed(&function.name),
            name_span: &function.name_span,
            params: Cow::Borrowed(&function.params),
            result: function.result.as_ref(),
            body: &function.body,
            span: &function.span,
        })
        .collect();
    for closure in collect_closures(ast) {
        let info = inferred
            .closures
            .get(&closure.fn_span.start)
            .ok_or(CompileError::InvalidIr("closure signature missing"))?;
        let mut params = Vec::with_capacity(info.captures.len() + info.params.len());
        for capture in &info.captures {
            params.push(parse::Parameter {
                mode: match capture.mode {
                    parse::CaptureMode::View => Mode::Read,
                    parse::CaptureMode::Edit => Mode::Inout,
                    parse::CaptureMode::Copy | parse::CaptureMode::Move => capture.usage,
                },
                name: capture.name.clone(),
                ty: capture.ty.clone(),
                span: capture.capture_span.clone(),
            });
        }
        for (index, param) in closure.params.iter().enumerate() {
            params.push(parse::Parameter {
                mode: param.mode,
                name: param.name.clone(),
                ty: info.params[index].2.clone(),
                span: param.span.clone(),
            });
        }
        all_functions.push(SourceFunction {
            is_generator: false,
            name: Cow::Owned(format!("$closure:{}", info.site)),
            name_span: &closure.fn_span,
            params: Cow::Owned(params),
            result: Some(&info.result),
            body: &closure.body,
            span: &closure.span,
        });
    }
    let mut user_functions = HashMap::new();
    for f in &all_functions {
        let params = f
            .params
            .iter()
            .map(|p| {
                let mode = match p.mode {
                    Mode::Read => Ok(ParameterMode::Read),
                    Mode::Consume => Ok(ParameterMode::Consume),
                    Mode::Inout => Ok(ParameterMode::Inout),
                }?;
                Ok((p.name.clone(), mode, lower_type(&p.ty)?))
            })
            .collect::<Result<Vec<_>, CompileError>>()?;
        let result = if f.is_generator {
            Ty::Generator {
                site: f.name_span.start,
                item: Box::new(f.result.map(lower_type).transpose()?.unwrap_or(Ty::Unit)),
            }
        } else {
            f.result.map(lower_type).transpose()?.unwrap_or(Ty::Unit)
        };
        user_functions.insert(f.name.to_string(), (params, result));
    }
    let functions: Vec<_> = all_functions.iter().collect();
    let mut entries = Vec::with_capacity(functions.len());
    let mut diagnostics = Vec::new();
    let mut ir_items = 0_u64;
    let mut ir_limit_span = None;
    for (index, source) in functions.into_iter().enumerate() {
        let result = match source.result {
            _ if source.is_generator => Ty::Unit,
            None | Some(Type::Unit) => Ty::Unit,
            Some(Type::I64) => Ty::I64,
            Some(Type::Bool) => Ty::Bool,
            Some(ty) => lower_type(ty)?,
        };
        let mut lower = Lower {
            instructions: Vec::new(),
            pending_charges: Vec::new(),
            blocks: Vec::new(),
            locals: Vec::new(),
            visible: HashMap::new(),
            scopes: vec![Vec::new()],
            scope: ScopeId(0),
            next_scope: 1,
            scope_parents: vec![None],
            next_temp: 0,
            result: result.clone(),
            terminated: None,
            user_functions: &user_functions,
            inferred_bindings: inferred,
            name_spans: Vec::new(),
            consumed_at: HashMap::new(),
            maybe_consumed_at: HashMap::new(),
            consumed_undo: Vec::new(),
            dead_depth: 0,
            emission: EmissionMode::Live,
            literal_true_loops: limits.literal_true_loops,
            poisoned: HashSet::new(),
            tokens: Arc::clone(&tokens),
            defer_ownership: limits.force_sparse_ownership || {
                let first = tokens.partition_point(|t| t.span.start < source.span.start);
                tokens[first..]
                    .iter()
                    .take_while(|t| t.span.start < source.span.end)
                    .any(|t| t.text == "while")
            },
            active_loans: HashMap::new(),
            loop_stack: Vec::new(),
            match_boundaries: Vec::new(),
            retained_locals: HashMap::new(),
            retained_temps: HashMap::new(),
            requested_receiver: None,
            live_temps: HashSet::new(),
            live_locals: HashSet::new(),
            local_head: None,
            local_history: Vec::new(),
            temp_loans: HashMap::new(),
            temp_undo: Vec::new(),
            temp_block_entry: EdgeState::default(),
            temp_blocks: Vec::new(),
            deferred_return_locals: Vec::new(),
            mode_candidates: Vec::new(),
        };
        let mut parameters = Vec::new();
        for (index, (p, (_, mode, ty))) in source
            .params
            .iter()
            .zip(&user_functions[source.name.as_ref()].0)
            .enumerate()
        {
            let id = LocalId(lower.locals.len());
            lower.locals.push(Local {
                id,
                name: p.name.clone(),
                name_span: p.span.clone(),
                ty: ty.clone(),
                mutable: inferred
                    .closures
                    .get(&source.name_span.start)
                    .and_then(|closure| closure.captures.get(index))
                    .map_or(*mode == ParameterMode::Inout, |capture| {
                        *mode == ParameterMode::Inout
                            && (capture.mode == parse::CaptureMode::Edit || capture.mutable)
                    }),
                scope: ScopeId(0),
                parameter_mode: Some(*mode),
            });
            let token_index = lower
                .tokens
                .partition_point(|t| t.span.start < p.span.start);
            let name_span = lower.tokens[token_index..]
                .iter()
                .take_while(|t| t.span.start < p.span.end)
                // the parameter-name token is parsed from within the
                // parameter span; its full token ends within that span.
                .find(|t| {
                    t.span.start >= p.span.start && t.span.end <= p.span.end && t.text == p.name
                })
                .map(|t| t.span.clone())
                .ok_or(CompileError::InvalidIr("parameter name token"))?;
            lower.name_spans.push(name_span);
            lower.locals[id.0].name_span = lower.name_spans.last().unwrap().clone();
            lower.visible.entry(p.name.clone()).or_default().push(id);
            lower.scopes[0].push(p.name.clone());
            lower.local_live(id, true);
            parameters.push(Parameter {
                local: id,
                mode: *mode,
            });
        }
        let before_body = diagnostics.len();
        lower.block(&source.body.statements, 0, &mut diagnostics)?;
        #[cfg(test)]
        if diagnostics.len() == before_body {
            assert_eq!(
                lower.dead_depth, 0,
                "hypothetical-flow depth must balance before a valid function ends"
            );
        }
        let missing_branch_value = diagnostics[before_body..]
            .iter()
            .any(|d| d.code == "NX-E1302" && d.subcause.as_deref() == Some("missing_branch_value"));
        if missing_branch_value {
            diagnostics.retain(|d| d.stage != diagnostic::Stage::Types);
        }
        // with no new diagnostic, a terminal return has already set
        // `terminated`; this flag then cannot affect the missing-return path.
        let invalid_terminal_return = diagnostics.len() > before_body
            && matches!(
                source.body.statements.last(),
                Some(Statement::Return { .. })
            );
        if !missing_branch_value
            && !invalid_terminal_return
            && lower.terminated.is_none()
            && result != Ty::Unit
        {
            let closing = Span {
                file: ast.span.file.clone(),
                start: source.body.span.end - 1,
                end: source.body.span.end,
            };
            // the arrow follows the complete parameter list, so it
            // cannot begin at the end of the function name token.
            let after_arrow = tokens
                .iter()
                .position(|t| t.span.start >= source.name_span.end && t.text == "->")
                .and_then(|at| {
                    tokens[at + 1..]
                        .iter()
                        .find(|t| t.kind != lex::Kind::Newline)
                });
            let last_type = after_arrow.and_then(|first| {
                tokens
                    .iter()
                    .filter(|t| {
                        t.span.start >= first.span.start && t.span.end <= source.body.span.start
                    })
                    .take_while(|t| t.text != "with" && t.text != "{")
                    .filter(|t| t.kind != lex::Kind::Newline)
                    .last()
                    .map(|last| Span {
                        file: ast.span.file.clone(),
                        start: first.span.start,
                        end: last.span.end,
                    })
            });
            let mut related_spans = Vec::new();
            if let Some(span) = last_type {
                related_spans.push(diagnostic::RelatedSpan {
                    role: diagnostic::RelatedRole::ExpectedType,
                    span,
                });
            }
            diagnostics.push(Diagnostic {
                schema_version: diagnostic::SCHEMA_VERSION,
                diagnostic_namespace: diagnostic::Namespace::NexoProposedV1,
                profile_id: PROFILE_ID.into(),
                code: "NX-E1317".into(),
                subcause: None,
                rule: "NX-E1317".into(),
                explanation_id: Some("NX-E1317".into()),
                severity: diagnostic::Severity::Error,
                stage: diagnostic::Stage::Ownership,
                message: "Falta un retorno en un camino normal.".into(),
                operation: diagnostic::Operation::Return,
                primary_span: Some(closing),
                related_spans,
                cause_chain: Vec::new(),
                feature_id: None,
                unexpected_token: None,
                expected_tokens: Vec::new(),
                expected_tokens_kind: None,
                parser_context: None,
                recognized_prefix: None,
                repair: Vec::new(),
                data: Default::default(),
            });
        }
        if diagnostics.len() > before_body {
            diagnostics.extend(lower.mode_candidates.drain(..).map(|candidate| candidate.2));
        }
        if diagnostics.len() > before_body {
            ir_items = ir_items.saturating_add(count_partial_ir_items(&lower));
            if ir_items > limits.ir_items && ir_limit_span.is_none() {
                ir_limit_span = Some(source.name_span.clone());
            }
            continue;
        }
        let span = source.body.span.clone();
        let final_unit_origin = Span {
            file: span.file.clone(),
            start: span.end.saturating_sub(1),
            end: span.end,
        };
        let all_paths_return = lower.terminated == Some(Termination::Diverged);
        let value = if all_paths_return {
            None
        } else if let Some(Termination::Return(value)) = lower.terminated {
            Some(value)
        } else {
            if result != Ty::Unit {
                return Err(CompileError::InvalidIr("control-flow return analysis"));
            }
            if lower.emission == EmissionMode::Live {
                lower.pending_charges.push(WorkPlanner::charge(
                    WorkEvent::FinalUnit,
                    final_unit_origin.clone(),
                ));
            }
            let unit = lower
                .emit(Op::ConstUnit, Ty::Unit, span.clone(), Access::Own, false)
                .ok_or(CompileError::InvalidIr("function return flow"))?;
            if let Some(instruction) = lower.instructions.last_mut()
                && instruction.output == Some(unit)
                && matches!(instruction.op, Op::ConstUnit)
                && let Some(last) = instruction.charges.last_mut()
            {
                last.origin = final_unit_origin;
            }
            Some(unit)
        };
        if !all_paths_return {
            lower.finish_block(Terminator::Return {
                value,
                span,
                charges: vec![],
            });
        }
        lower.validate_temp_edges()?;
        let mut all = lower.blocks;
        if all.is_empty() {
            return Err(CompileError::InvalidIr("return flow"));
        }
        let entry = all.remove(0);
        let other_blocks = all;
        let mut function = Function {
            id: FunctionId(source.name.to_string()),
            table_index: index,
            name: source.name.to_string(),
            source_file: ast.span.file.clone(),
            source_text: source_text.clone(),
            scope_count: lower.next_scope,
            scope_parents: lower.scope_parents,
            content_hash: 0,
            locals: lower.locals,
            parameters,
            block: entry,
            blocks: other_blocks,
            result,
            generator_site: source.is_generator.then_some(source.name_span.start),
            generator_item: source.is_generator.then(|| {
                source
                    .result
                    .map(lower_type)
                    .transpose()
                    .expect("checked item type")
                    .unwrap_or(Ty::Unit)
            }),
            cleanup: Vec::new(),
            effects: None,
            exit_info: Vec::new(),
        };
        function.cleanup = function.locals.iter().rev().map(|l| l.id).collect();
        function.content_hash = model::content_hash(&function);
        ir_items = ir_items.saturating_add(count_function_ir_items(&function));
        if ir_items > limits.ir_items && ir_limit_span.is_none() {
            ir_limit_span = Some(source.name_span.clone());
        }
        entries.push((
            function,
            lower.name_spans,
            source.name_span.start,
            source,
            lower.mode_candidates,
        ));
    }
    // The complete constructible IR is collected before sparse ownership
    // analysis starts; its traversal follows source order and includes
    // functions without loops.
    let mut ownership_global_steps = 0_u64;
    let mut ownership_global_limited = false;
    let mut owned_entries = Vec::with_capacity(entries.len());
    for (mut function, name_spans, function_start, source_function, mode_candidates) in entries {
        if ownership_global_limited {
            continue;
        }
        let mut ownership_flow_steps = 0_u64;
        let flow_errors = match super::ownership_flow::analyze(
            &function,
            &name_spans,
            &tokens,
            &mut ownership_flow_steps,
            &mut ownership_global_steps,
            function_start,
            limits.flow_function,
            limits.flow_compilation,
            limits.profile_id,
        ) {
            Ok(errors) => errors,
            Err(CompileError::Rejected(errors)) => {
                ownership_global_limited = errors.iter().any(|error| {
                    error.data.get("budget_scope")
                        == Some(&diagnostic::DataValue::String("compilation".into()))
                });
                diagnostics.extend(errors.into_iter().map(|d| *d));
                let retained: Vec<_> = mode_candidates
                    .into_iter()
                    .filter_map(|(id, span, error)| {
                        (!diagnostics
                            .iter()
                            .any(|d| mode_candidate_consumed(d, id, &span, &name_spans)))
                        .then_some(error)
                    })
                    .collect();
                diagnostics.extend(retained);
                continue;
            }
            Err(other) => return Err(other),
        };
        if !flow_errors.is_empty() || !mode_candidates.is_empty() {
            diagnostics.extend(mode_candidates.into_iter().filter_map(|(id, span, error)| {
                (!flow_errors
                    .iter()
                    .any(|d| mode_candidate_consumed(d, id, &span, &name_spans)))
                .then_some(error)
            }));
            diagnostics.extend(flow_errors);
            continue;
        }
        super::effects::populate_exit_info(&mut function, source_function.body);
        if !isolate_lowering {
            super::verify::verify_function(&function).map_err(CompileError::InvalidIr)?;
        }
        owned_entries.push(function);
    }
    let entries = owned_entries;
    if !diagnostics.is_empty() {
        fn diagnostic_key(d: &Diagnostic) -> (&str, u64, u64, &str, &str) {
            (
                d.primary_span.as_ref().map_or("", |s| s.file.as_str()),
                d.primary_span.as_ref().map_or(0, |s| s.start),
                d.primary_span.as_ref().map_or(0, |s| s.end),
                d.code.as_str(),
                d.subcause.as_deref().unwrap_or(""),
            )
        }
        diagnostics.sort_by(|a, b| diagnostic_key(a).cmp(&diagnostic_key(b)));
        return Err(CompileError::Rejected(
            diagnostics.into_iter().map(Box::new).collect(),
        ));
    }
    let program = Program {
        functions: FunctionTable {
            entries: {
                let mut entries = entries;
                entries.sort_by(|a, b| a.name.cmp(&b.name));
                for (table_index, function) in entries.iter_mut().enumerate() {
                    function.table_index = table_index;
                }
                entries
            },
        },
        profile: PROFILE_ID,
    };
    let mut program = program;
    if let Err(errors) = super::effects::analyze(
        &mut program,
        ast,
        inferred,
        limits.effects,
        limits.profile_id,
    ) {
        return Err(CompileError::Rejected(
            errors.into_iter().map(Box::new).collect(),
        ));
    }
    if let Some(span) = ir_limit_span {
        let mut diagnostic = crate::check::names::diagnostic(
            "NX-E0003",
            None,
            diagnostic::Stage::Lower,
            diagnostic::Operation::AnalysisLimit,
            span,
        );
        diagnostic.profile_id = limits.profile_id.into();
        diagnostic.data.insert(
            "budget_kind".into(),
            diagnostic::DataValue::String("ir_items".into()),
        );
        diagnostic.data.insert(
            "budget_scope".into(),
            diagnostic::DataValue::String("compilation".into()),
        );
        diagnostic
            .data
            .insert("limit".into(), diagnostic::DataValue::UInt(limits.ir_items));
        diagnostic.data.insert(
            "observed".into(),
            diagnostic::DataValue::UInt(limits.ir_items.saturating_add(1)),
        );
        diagnostic.data.insert(
            "incomplete_analysis".into(),
            diagnostic::DataValue::String("lower".into()),
        );
        return Err(CompileError::Rejected(vec![diagnostic]));
    }
    if !isolate_lowering {
        super::verify::verify_program(&program).map_err(CompileError::InvalidIr)?;
    }
    if ast.f5_operators
        && let Some(span) = f5_capabilities(ast).first.map(|(_, span)| span)
    {
        return Err(CompileError::Rejected(vec![l08_barrier(span)]));
    }
    Ok(program)
}

/// The public L08 barrier of F0-0005 §8.2 on the first node of a new capability.
fn l08_barrier(span: Span) -> Box<Diagnostic> {
    let mut diagnostic = crate::check::names::diagnostic(
        "NX-E0006",
        None,
        diagnostic::Stage::Lower,
        diagnostic::Operation::IrLower,
        span,
    );
    diagnostic.feature_id = Some("implementation_limit".into());
    diagnostic.message = "Capacidad pendiente de implementación en L08.".into();
    diagnostic.data.insert(
        "pending_scope".into(),
        diagnostic::DataValue::String("L08".into()),
    );
    diagnostic
}

fn is_l08_barrier(diagnostic: &Diagnostic) -> bool {
    diagnostic.code == "NX-E0006"
        && diagnostic.stage == diagnostic::Stage::Lower
        && diagnostic.data.get("pending_scope")
            == Some(&diagnostic::DataValue::String("L08".into()))
}

/// Source nodes that the F5 profile admits only behind the L08 barrier.
struct F5Capabilities {
    /// Raw start and primary span of the first operator-like capability.
    first: Option<(u64, Span)>,
    /// `while` keywords whose condition is a literal `true` (groups allowed).
    literal_true_loops: Vec<Span>,
}

fn f5_capabilities(ast: &Ast) -> F5Capabilities {
    enum Node<'a> {
        Expr(&'a Expr),
        Statement(&'a Statement),
        Block(&'a parse::Block),
        Match(&'a parse::Match),
    }
    let mut pending: Vec<_> = ast.functions.iter().map(|f| Node::Block(&f.body)).collect();
    // The raw start orders candidates; the span is what a diagnostic reports.
    let mut first: Option<(u64, Span)> = None;
    let mut literal_true_loops = Vec::new();
    while let Some(node) = pending.pop() {
        match node {
            Node::Block(block) => pending.extend(block.statements.iter().map(Node::Statement)),
            Node::Statement(statement) => match statement {
                Statement::Let { value, .. } | Statement::Expr { value, .. } => {
                    pending.push(Node::Expr(value))
                }
                Statement::Assign {
                    place, op, value, ..
                } => {
                    if *op != parse::AssignOp::Set
                        && first
                            .as_ref()
                            .is_none_or(|(prior, _)| place.span().start < *prior)
                    {
                        first = Some((place.span().start, parse::expression_primary_span(place)));
                    }
                    pending.push(Node::Expr(value));
                    pending.push(Node::Expr(place));
                }
                Statement::Return { value, .. }
                | Statement::GuardControl { value, .. }
                | Statement::Value { value, .. } => {
                    if let Some(value) = value {
                        pending.push(Node::Expr(value));
                    }
                }
                Statement::If {
                    condition,
                    then_block,
                    otherwise,
                    ..
                } => {
                    pending.push(Node::Expr(condition));
                    pending.push(Node::Block(then_block));
                    if let Some(otherwise) = otherwise {
                        pending.push(Node::Statement(otherwise));
                    }
                }
                Statement::While {
                    condition,
                    body,
                    span,
                } => {
                    let mut ungrouped = condition;
                    while let Expr::Group(inner, _) = ungrouped {
                        ungrouped = inner;
                    }
                    if matches!(ungrouped, Expr::Bool(true, _)) {
                        literal_true_loops.push(Span {
                            file: span.file.clone(),
                            start: span.start,
                            end: span.start + "while".len() as u64,
                        });
                    }
                    pending.push(Node::Expr(condition));
                    pending.push(Node::Block(body));
                }
                Statement::For { iterable, body, .. } => {
                    pending.push(Node::Expr(iterable));
                    pending.push(Node::Block(body));
                }
                Statement::Match(selection) => pending.push(Node::Match(selection)),
                Statement::Block(block) => pending.push(Node::Block(block)),
            },
            Node::Match(selection) => {
                pending.push(Node::Expr(&selection.scrutinee));
                for arm in &selection.arms {
                    if let Some(guard) = &arm.guard {
                        pending.push(Node::Expr(guard));
                    }
                    pending.push(Node::Block(&arm.body));
                }
            }
            Node::Expr(expr) => match expr {
                Expr::Binary {
                    op,
                    left,
                    right,
                    span,
                } => {
                    if crate::check::operators::needs_f5(op)
                        && first.as_ref().is_none_or(|(prior, _)| span.start < *prior)
                    {
                        first = Some((span.start, span.clone()));
                    }
                    pending.push(Node::Expr(left));
                    pending.push(Node::Expr(right));
                }
                Expr::Closure(closure) => pending.push(Node::Block(&closure.body)),
                Expr::Array(items, _) => pending.extend(items.iter().map(Node::Expr)),
                Expr::Neg { value, span } | Expr::Not { value, span } => {
                    if first.as_ref().is_none_or(|(prior, _)| span.start < *prior) {
                        first = Some((span.start, span.clone()));
                    }
                    pending.push(Node::Expr(value));
                }
                Expr::Group(value, _) | Expr::Unary { value, .. } => {
                    pending.push(Node::Expr(value))
                }
                Expr::Call { target, args, .. } => {
                    pending.push(Node::Expr(target));
                    pending.extend(args.iter().map(|arg| Node::Expr(&arg.value)));
                }
                Expr::Index { target, index, .. } => {
                    pending.push(Node::Expr(target));
                    pending.push(Node::Expr(index));
                }
                Expr::If {
                    condition,
                    then_block,
                    otherwise,
                    ..
                } => {
                    pending.push(Node::Expr(condition));
                    pending.push(Node::Block(then_block));
                    if let Some(otherwise) = otherwise {
                        match otherwise.as_ref() {
                            parse::Else::Block(block) => pending.push(Node::Block(block)),
                            parse::Else::If(expr) => pending.push(Node::Expr(expr)),
                        }
                    }
                }
                Expr::Match(selection) => pending.push(Node::Match(selection)),
                Expr::Name(..) | Expr::Integer(..) | Expr::Bool(..) | Expr::Unit(..) => {}
            },
        }
    }
    F5Capabilities {
        first,
        literal_true_loops,
    }
}

fn ir_operand_count(op: &Op) -> u64 {
    match op {
        Op::ConstI64(_) | Op::ConstBool(_) | Op::ConstUnit => 1,
        Op::CopyValue(_)
        | Op::BeginArrayIteration(_)
        | Op::TransferValue(_)
        | Op::BorrowValue(_)
        | Op::DropBinding(_)
        | Op::DropLocal(_)
        | Op::DropValue(_)
        | Op::Unary { .. }
        | Op::CompoundLoad { .. }
        | Op::EndLoan(_) => 1,
        Op::BeginLoan { indexes, .. } => 1 + indexes.len() as u64,
        Op::BuildArray(values) | Op::CallIntrinsic { args: values, .. } => values.len() as u64,
        Op::BuildClosure { fields, .. } => 1 + fields.len() as u64,
        Op::BuildVariant { .. }
        | Op::BranchVariant { .. }
        | Op::TestVariant { .. }
        | Op::TestLiteral { .. }
        | Op::ProjectPayload { .. }
        | Op::ExtractPayload { .. } => 1,
        Op::BindView { .. } | Op::BindEdit { .. } => 2,
        Op::ReadIndex { indexes, .. }
        | Op::BorrowIndex { indexes, .. }
        | Op::CheckIndex { indexes, .. } => 1 + indexes.len() as u64,
        Op::IndexValue { .. }
        | Op::BorrowArrayElement { .. }
        | Op::Binary { .. }
        | Op::RangeTest { .. }
        | Op::RangeAdvance { .. }
        | Op::WritePlace { .. } => 2,
        Op::CompoundStore { .. } => 3,
        Op::WriteIndex { indexes, .. } => 2 + indexes.len() as u64,
        Op::CallDirect { args, .. } | Op::BuildGenerator { args, .. } => 1 + args.len() as u64,
        Op::ResumeGenerator { .. } => 2,
        Op::CallClosure { args, .. } => 2 + args.len() as u64,
    }
}

fn ir_instruction_count(instruction: &Instruction) -> u64 {
    1 + u64::from(instruction.output.is_some())
        + ir_operand_count(&instruction.op)
        + u64::from(matches!(instruction.op, Op::BeginLoan { .. }))
}

fn ir_terminator_count(terminator: &Terminator) -> u64 {
    1 + match terminator {
        Terminator::Unreachable { .. } => 0,
        Terminator::Return { value, .. } => u64::from(value.is_some()),
        Terminator::Jump { .. } => 1,
        Terminator::BranchBool { .. } => 3,
        Terminator::Yield { .. } => 2,
    }
}

fn count_function_ir_items(function: &Function) -> u64 {
    let blocks = std::iter::once(&function.block).chain(&function.blocks);
    1 + function.blocks.len() as u64
        + 1
        + function.locals.len() as u64
        + function.cleanup.len() as u64
        + blocks
            .map(|block| {
                block
                    .instructions
                    .iter()
                    .map(ir_instruction_count)
                    .sum::<u64>()
                    + ir_terminator_count(&block.terminator)
            })
            .sum::<u64>()
}

fn count_partial_ir_items(lower: &Lower<'_>) -> u64 {
    1 + lower.blocks.len() as u64
        + 1
        + lower.locals.len() as u64
        + lower
            .blocks
            .iter()
            .map(|block| {
                block
                    .instructions
                    .iter()
                    .map(ir_instruction_count)
                    .sum::<u64>()
                    + ir_terminator_count(&block.terminator)
            })
            .sum::<u64>()
        + lower
            .instructions
            .iter()
            .map(ir_instruction_count)
            .sum::<u64>()
}

#[derive(Clone)]
struct TempBlockState {
    entry: EdgeState,
    edges: Vec<EdgeState>,
}

#[derive(Clone, Default)]
struct EdgeState {
    reachable: bool,
    temps: HashSet<TempId>,
    locals: Option<usize>,
    loan_details: HashMap<LocalId, Vec<ActiveLoan>>,
    suspended: bool,
    delivered: Option<TempId>,
}

struct LocalChange {
    id: LocalId,
    was_live: bool,
    now_live: bool,
    parent: Option<usize>,
    depth: usize,
}

#[derive(Clone)]
struct ExitPlan {
    surviving: HashSet<TempId>,
    locals: Vec<(LocalId, Span)>,
    reserved: Option<TempId>,
}

struct Lower<'a> {
    instructions: Vec<Instruction>,
    pending_charges: Vec<WorkCharge>,
    blocks: Vec<BasicBlock>,
    locals: Vec<Local>,
    visible: HashMap<String, Vec<LocalId>>,
    scopes: Vec<Vec<String>>,
    scope: ScopeId,
    next_scope: usize,
    scope_parents: Vec<Option<ScopeId>>,
    next_temp: usize,
    result: Ty,
    terminated: Option<Termination>,
    user_functions: &'a HashMap<String, FunctionSignature>,
    inferred_bindings: &'a crate::check::types::InferredTypes,
    name_spans: Vec<Span>,
    consumed_at: HashMap<LocalId, Vec<(Span, bool)>>,
    maybe_consumed_at: HashMap<LocalId, Vec<(Span, bool)>>,
    consumed_undo: Vec<AvailabilityUndo>,
    dead_depth: usize,
    emission: EmissionMode,
    literal_true_loops: bool,
    poisoned: HashSet<LocalId>,
    tokens: Arc<Vec<lex::Token>>,
    defer_ownership: bool,
    active_loans: HashMap<LocalId, Vec<ActiveLoan>>,
    loop_stack: Vec<LoopContext>,
    match_boundaries: Vec<MatchBoundary>,
    retained_locals: HashMap<LocalId, Vec<RetainedLoan>>,
    retained_temps: HashMap<TempId, Vec<RetainedLoan>>,
    requested_receiver: Option<(ParameterMode, Span)>,
    live_temps: HashSet<TempId>,
    live_locals: HashSet<LocalId>,
    local_head: Option<usize>,
    local_history: Vec<LocalChange>,
    temp_loans: HashMap<TempId, (LocalId, LoanId)>,
    temp_undo: Vec<(TempId, bool)>,
    temp_block_entry: EdgeState,
    temp_blocks: Vec<TempBlockState>,
    deferred_return_locals: Vec<(LocalId, Span)>,
    mode_candidates: Vec<(LocalId, Span, Diagnostic)>,
}

impl Lower<'_> {
    fn loop_control(&mut self, token: &str, span: &Span) -> Result<(), CompileError> {
        if self.emission == EmissionMode::Analyze {
            self.terminated = Some(Termination::Diverged);
            return Ok(());
        }
        let context = self
            .loop_stack
            .last()
            .ok_or(CompileError::InvalidIr("loop control without loop"))?;
        let scope_depth = context.scope_depth;
        let continue_target = context.continue_target;
        let cleanup_span = context.cleanup_span.clone();
        let match_depth = context.match_depth;
        let dropping: Vec<_> = self.scopes[scope_depth..]
            .iter()
            .rev()
            .flat_map(|scope| scope.iter().rev())
            .filter_map(|name| self.visible.get(name).and_then(|ids| ids.last()).copied())
            .collect();
        let mut locals: Vec<_> = dropping
            .into_iter()
            .map(|id| (id, cleanup_span.clone()))
            .collect();
        locals.extend(self.boundary_locals(match_depth));
        self.plan_exit(
            ExitPlan {
                surviving: context.surviving_temps.clone(),
                locals,
                reserved: None,
            },
            span,
        );
        let id = self.finish_block(Terminator::Jump {
            target: if token == "break" {
                BlockId(usize::MAX)
            } else {
                continue_target
            },
            span: span.clone(),
            charges: vec![],
        });
        if token == "break" {
            self.loop_stack
                .last_mut()
                .expect("loop context")
                .breaks
                .push(id);
        } else if continue_target.0 == usize::MAX {
            self.loop_stack
                .last_mut()
                .expect("loop context")
                .continues
                .push(id);
        }
        self.emission = EmissionMode::Analyze;
        self.terminated = Some(Termination::Diverged);
        Ok(())
    }

    fn boundary_locals(&self, keep: usize) -> Vec<(LocalId, Span)> {
        self.match_boundaries[keep..]
            .iter()
            .rev()
            .map(|boundary| (boundary.root, boundary.span.clone()))
            .collect()
    }

    fn release_retained(&mut self, loans: Vec<RetainedLoan>) {
        for retained in loans.into_iter().rev() {
            if let Some(active) = self.active_loans.get_mut(&retained.root)
                && let Some(index) = active
                    .iter()
                    .rposition(|loan| loan.loan == retained.active.loan)
            {
                active.remove(index);
            }
        }
    }

    fn output_retained(&mut self, op: &Op) -> Vec<RetainedLoan> {
        match op {
            Op::CopyValue(local) => {
                let copied = self.retained_locals.get(local).cloned().unwrap_or_default();
                for loan in &copied {
                    self.active_loans
                        .entry(loan.root)
                        .or_default()
                        .push(loan.active.clone());
                }
                copied
            }
            Op::TransferValue(local) => self.retained_locals.remove(local).unwrap_or_default(),
            Op::BuildVariant {
                payload: Some(temp),
                ..
            } => self.retained_temps.remove(temp).unwrap_or_default(),
            Op::BuildClosure { fields, .. } => fields
                .iter()
                .flat_map(|(_, temp)| self.retained_temps.remove(temp).unwrap_or_default())
                .collect(),
            Op::CallIntrinsic {
                intrinsic: Intrinsic::DropData,
                args,
            } => {
                for temp in args {
                    if let Some(loans) = self.retained_temps.remove(temp) {
                        self.release_retained(loans);
                    }
                }
                Vec::new()
            }
            Op::CallClosure {
                receiver,
                mode: ParameterMode::Consume,
                ..
            } => {
                if let Some(loans) = self.retained_temps.remove(receiver) {
                    self.release_retained(loans);
                }
                Vec::new()
            }
            _ => Vec::new(),
        }
    }

    fn statement_retained(&mut self, op: &Op) {
        match op {
            Op::WritePlace { local, value } => {
                if let Some(old) = self.retained_locals.remove(local) {
                    self.release_retained(old);
                }
                if let Some(loans) = self.retained_temps.remove(value) {
                    self.retained_locals.insert(*local, loans);
                }
            }
            Op::DropLocal(local) | Op::DropBinding(local) => {
                if let Some(loans) = self.retained_locals.remove(local) {
                    self.release_retained(loans);
                }
            }
            Op::DropValue(temp) => {
                if let Some(loans) = self.retained_temps.remove(temp) {
                    self.release_retained(loans);
                }
            }
            _ => {}
        }
    }

    fn resolved_type(&self, resolved: &ResolvedType) -> Result<Ty, CompileError> {
        match resolved {
            ResolvedType::Concrete(ty) => lower_type(ty),
            // Neither state may create a live slot in an accepted program:
            // Never has no normal continuation; Unresolved appears only in
            // hypothetical/tainted paths that prevent executable output
            // (§4.1.1). Both therefore use the existing no-value IR marker.
            ResolvedType::Never | ResolvedType::Unresolved => Ok(Ty::Never),
        }
    }

    fn resolved_expr(&self, expr: &Expr) -> Result<Ty, CompileError> {
        let ty = self
            .inferred_bindings
            .expressions
            .get(&(expr.span().start, expr.span().end))
            .ok_or(CompileError::InvalidIr("missing checker expression type"))?;
        self.resolved_type(ty)
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum EmissionMode {
    Live,
    Analyze,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Termination {
    Return(TempId),
    Diverged,
}
struct LoopContext {
    continue_target: BlockId,
    scope_depth: usize,
    cleanup_span: Span,
    breaks: Vec<BlockId>,
    continues: Vec<BlockId>,
    match_depth: usize,
    surviving_temps: HashSet<TempId>,
}
#[derive(Clone)]
struct MatchBoundary {
    root: LocalId,
    span: Span,
}
#[derive(Clone)]
enum Flow {
    Value { temp: Option<TempId>, ty: Ty },
    Diverged { ty: Option<Ty> },
}
impl Flow {
    fn ty(&self) -> Option<&Ty> {
        match self {
            Self::Value { ty, .. } => Some(ty),
            Self::Diverged { ty } => ty.as_ref(),
        }
    }

    fn temp(&self) -> Option<TempId> {
        match self {
            Self::Value { temp, .. } => *temp,
            Self::Diverged { .. } => None,
        }
    }

    fn diverged(&self) -> bool {
        matches!(self, Self::Diverged { .. })
    }
}
type FunctionSignature = (Vec<(String, ParameterMode, Ty)>, Ty);
type ReceiverLoan = (LocalId, LoanId, Span);
type CallParameters = Vec<(String, ParameterMode, Ty)>;
#[derive(Clone)]
struct ActiveLoan {
    permission: LoanPermission,
    span: Span,
    loan: LoanId,
    parent: Option<LocalId>,
    compound_assignment: bool,
}
#[derive(Clone)]
struct RetainedLoan {
    root: LocalId,
    active: ActiveLoan,
    source: Span,
}
#[derive(Clone, PartialEq, Eq)]
struct Availability {
    definitely: Option<Vec<(Span, bool)>>,
    maybe: Option<Vec<(Span, bool)>>,
}
type AvailabilityUndo = (
    LocalId,
    Option<Vec<(Span, bool)>>,
    Option<Vec<(Span, bool)>>,
);
fn merge_sites(mut left: Vec<(Span, bool)>, right: Vec<(Span, bool)>) -> Vec<(Span, bool)> {
    left.extend(right);
    left.sort_by_key(|a| (a.0.start, a.0.end, a.1));
    left.dedup();
    left
}
fn branch_error(error: CompileError, diagnostics: Vec<Diagnostic>) -> CompileError {
    if diagnostics.is_empty() {
        return error;
    }
    let mut errors: Vec<Box<Diagnostic>> = diagnostics.into_iter().map(Box::new).collect();
    if let CompileError::Rejected(mut additional) = error {
        errors.append(&mut additional);
    }
    CompileError::Rejected(errors)
}

fn add_backedge_to_diagnostic(
    d: &mut Diagnostic,
    body: &Span,
    affected_declarations: &HashSet<u64>,
) {
    let closing = Span {
        file: body.file.clone(),
        start: body.end.saturating_sub(1),
        end: body.end,
    };
    if d.code == "NX-E0202"
        && d.related_spans.iter().any(|r| {
            r.role == diagnostic::RelatedRole::Declaration
                && affected_declarations.contains(&r.span.start)
        })
    {
        d.related_spans.push(diagnostic::RelatedSpan {
            role: diagnostic::RelatedRole::Backedge,
            span: closing.clone(),
        });
        if let Some(entity) = d.cause_chain.last().and_then(|c| c.entity.clone()) {
            d.cause_chain.insert(
                0,
                diagnostic::Cause {
                    kind: diagnostic::CauseKind::Backedge,
                    operation: diagnostic::CauseOperation::FlowBackedge,
                    span: Some(closing.clone()),
                    entity: Some(entity),
                    evidence: diagnostic::Evidence::Verified,
                },
            );
        }
    }
}

fn add_backedge_to_error(
    error: CompileError,
    body: &Span,
    affected_declarations: &HashSet<u64>,
) -> CompileError {
    let CompileError::Rejected(mut errors) = error else {
        return error;
    };
    for d in &mut errors {
        add_backedge_to_diagnostic(d, body, affected_declarations);
    }
    CompileError::Rejected(errors)
}
#[derive(Clone, Copy)]
enum StatementElse<'a> {
    Statement(&'a Statement),
    Else(&'a parse::Else),
}
enum LowerRoot<'a> {
    Block(&'a [Statement], usize),
    #[cfg(test)]
    Statement(&'a Statement, usize),
    #[cfg(test)]
    Expr(&'a Expr, usize, Option<Ty>),
    #[cfg(test)]
    Argument(&'a parse::Argument, ParameterMode, Ty, usize, bool),
}
struct IfBeginState {
    entry_emission: EmissionMode,
    branch_mode: EmissionMode,
    branch: Option<BlockId>,
    condition_diverges: bool,
    temp_checkpoint: usize,
    prior_locals: usize,
    checkpoint: usize,
}
enum ForState {
    Analyze {
        saved: ScopeId,
        loan_root: Option<LocalId>,
    },
    RangeLive {
        header: BlockId,
        branch: BlockId,
        saved: ScopeId,
        availability_checkpoint: usize,
        binding: LocalId,
        cursor: LocalId,
        limit: LocalId,
        inclusive: bool,
    },
    ArrayLive {
        header: BlockId,
        branch: BlockId,
        saved: ScopeId,
        availability_checkpoint: usize,
        binding: LocalId,
        cursor: LocalId,
        limit: LocalId,
        root: LocalId,
        owned_root: bool,
        guard: TempId,
        source: TempId,
    },
    Live {
        header: BlockId,
        branch: BlockId,
        saved: ScopeId,
        availability_checkpoint: usize,
        binding: LocalId,
        option: LocalId,
        iterator: LocalId,
    },
}
struct IfState {
    entry_emission: EmissionMode,
    branch_mode: EmissionMode,
    branch: Option<BlockId>,
    condition_diverges: bool,
    temp_checkpoint: usize,
    prior_locals: usize,
    checkpoint: usize,
    then_continues: bool,
    then_end: Option<BlockId>,
    then_state: HashMap<LocalId, Availability>,
}
struct IfExprState<'a> {
    otherwise: &'a parse::Else,
    span: &'a Span,
    depth: usize,
    branch_mode: EmissionMode,
    condition_diverges: bool,
    temp_checkpoint: usize,
    prior_locals: usize,
    checkpoint: usize,
    diagnostics: Vec<Diagnostic>,
    dependent: bool,
    slot: Option<LocalId>,
    inferred_ty: Ty,
    then_continues: bool,
    then_end: Option<BlockId>,
    then_state: HashMap<LocalId, Availability>,
}
struct IfExprStarted<'a> {
    otherwise: &'a parse::Else,
    span: &'a Span,
    then_block: &'a parse::Block,
    depth: usize,
    branch_mode: EmissionMode,
    branch: Option<BlockId>,
    condition_diverges: bool,
    temp_checkpoint: usize,
    prior_locals: usize,
    checkpoint: usize,
    diagnostics: Vec<Diagnostic>,
    dependent: bool,
    slot: Option<LocalId>,
    inferred_ty: Ty,
}
impl Lower<'_> {
    fn edge_state(&self) -> EdgeState {
        EdgeState {
            reachable: true,
            temps: self.live_temps.clone(),
            locals: self.local_head,
            loan_details: self
                .active_loans
                .iter()
                .filter(|(_, loans)| !loans.is_empty())
                .map(|(root, loans)| (*root, loans.clone()))
                .collect(),
            suspended: false,
            delivered: None,
        }
    }
    fn local_live(&mut self, id: LocalId, live: bool) {
        let was_live = self.live_locals.contains(&id);
        if was_live == live {
            return;
        }
        if live {
            self.live_locals.insert(id);
        } else {
            self.live_locals.remove(&id);
        }
        let depth = self
            .local_head
            .map_or(1, |parent| self.local_history[parent].depth + 1);
        self.local_history.push(LocalChange {
            id,
            was_live,
            now_live: live,
            depth,
            parent: self.local_head,
        });
        self.local_head = Some(self.local_history.len() - 1);
    }
    fn switch_local_state(&mut self, target: Option<usize>) {
        let mut from = self.local_head;
        let mut to = target;
        let mut replay = Vec::new();
        let depth = |node: Option<usize>| node.map_or(0, |node| self.local_history[node].depth);
        while depth(from) > depth(to) {
            let node = from.expect("local state depth");
            let node = &self.local_history[node];
            if node.was_live {
                self.live_locals.insert(node.id);
            } else {
                self.live_locals.remove(&node.id);
            }
            from = node.parent;
        }
        while depth(to) > depth(from) {
            let node = to.expect("local state depth");
            to = self.local_history[node].parent;
            replay.push(node);
        }
        while from != to {
            let old = from.expect("local state common ancestor");
            let old = &self.local_history[old];
            if old.was_live {
                self.live_locals.insert(old.id);
            } else {
                self.live_locals.remove(&old.id);
            }
            from = old.parent;
            let next = to.expect("local state common ancestor");
            to = self.local_history[next].parent;
            replay.push(next);
        }
        for node in replay
            .into_iter()
            .rev()
            .map(|node| &self.local_history[node])
        {
            if node.now_live {
                self.live_locals.insert(node.id);
            } else {
                self.live_locals.remove(&node.id);
            }
        }
        self.local_head = target;
    }
    fn track_local_op(&mut self, op: &Op) {
        match op {
            Op::WritePlace { local, .. }
            | Op::BindView { local, .. }
            | Op::BindEdit { local, .. } => self.local_live(*local, true),
            Op::DropLocal(id) | Op::DropBinding(id) | Op::TransferValue(id) => {
                self.local_live(*id, false);
            }
            _ => {}
        }
    }
    fn emitted(&mut self, op: Op, ty: Ty, span: Span, access: Access, effect_io: bool) -> Flow {
        let temp = self.emit(op, clone_ty_iterative(&ty), span, access, effect_io);
        Flow::Value { temp, ty }
    }
    fn temp_live(&mut self, id: TempId, live: bool) {
        let was = self.live_temps.contains(&id);
        if was != live {
            self.temp_undo.push((id, was));
            if live {
                self.live_temps.insert(id);
            } else {
                self.live_temps.remove(&id);
            }
        }
    }
    fn restore_temps(&mut self, checkpoint: usize) {
        while self.temp_undo.len() > checkpoint {
            let Some((id, was)) = self.temp_undo.pop() else {
                break;
            };
            if was {
                self.live_temps.insert(id);
            } else {
                self.live_temps.remove(&id);
            }
        }
        if self.instructions.is_empty() {
            self.temp_block_entry = self.edge_state();
        }
    }
    fn enter_edge(&mut self, block: BlockId, edge: usize) -> Result<(), CompileError> {
        let state = self
            .temp_blocks
            .get(block.0)
            .and_then(|block| block.edges.get(edge))
            .ok_or(CompileError::InvalidIr("temporary edge source"))?
            .clone();
        let leaving: Vec<_> = self.live_temps.difference(&state.temps).copied().collect();
        let entering: Vec<_> = state.temps.difference(&self.live_temps).copied().collect();
        for id in leaving {
            self.temp_live(id, false);
        }
        for id in entering {
            self.temp_live(id, true);
        }
        self.switch_local_state(state.locals);
        self.active_loans = state.loan_details.clone();
        self.temp_block_entry = state;
        Ok(())
    }
    fn plan_exit(&mut self, plan: ExitPlan, span: &Span) {
        let state = self.edge_state();
        let mut pending: Vec<_> = state
            .temps
            .iter()
            .copied()
            .filter(|id| !plan.surviving.contains(id) && Some(*id) != plan.reserved)
            .collect();
        pending.sort_by_key(|id| std::cmp::Reverse(id.0));
        for id in pending {
            self.statement_op(Op::DropValue(id), span.clone(), Access::Own, false);
        }
        for (id, local_span) in plan.locals {
            if self.live_locals.contains(&id)
                && !self.poisoned.contains(&id)
                && !self.consumed_at.contains_key(&id)
            {
                self.statement_op(Op::DropLocal(id), local_span, Access::Own, false);
            }
        }
    }
    fn validate_temp_edges(&self) -> Result<(), CompileError> {
        if self.blocks.len() != self.temp_blocks.len() {
            return Err(CompileError::InvalidIr("temporary block state"));
        }
        for (block, state) in self.blocks.iter().zip(&self.temp_blocks) {
            let targets: Vec<BlockId> = match &block.terminator {
                Terminator::Jump { target, .. } => vec![*target],
                Terminator::BranchBool { yes, no, .. } => vec![*yes, *no],
                Terminator::Yield { resume, .. } => vec![*resume],
                Terminator::Return { .. } | Terminator::Unreachable { .. } => Vec::new(),
            };
            if targets.len() != state.edges.len() {
                return Err(CompileError::InvalidIr("temporary edge count"));
            }
            for (target, edge) in targets.into_iter().zip(&state.edges) {
                let successor = self
                    .temp_blocks
                    .get(target.0)
                    .ok_or(CompileError::InvalidIr("temporary edge target"))?;
                if !edge.reachable || edge.temps != successor.entry.temps {
                    return Err(CompileError::InvalidIr("temporary edge state"));
                }
                if edge.suspended != matches!(block.terminator, Terminator::Yield { .. })
                    || (edge.suspended && edge.delivered.is_none())
                {
                    return Err(CompileError::InvalidIr("temporary suspension state"));
                }
            }
        }
        Ok(())
    }
    fn consume_op_temps(&mut self, op: &Op) {
        match op {
            Op::BeginLoan { indexes, .. }
            | Op::BuildArray(indexes)
            | Op::ReadIndex { indexes, .. }
            | Op::BorrowIndex { indexes, .. }
            | Op::CallIntrinsic { args: indexes, .. } => {
                for &id in indexes {
                    self.temp_live(id, false);
                }
            }
            Op::BuildClosure { fields, .. } => {
                for &(_, id) in fields {
                    self.temp_live(id, false);
                }
            }
            Op::CallClosure { receiver, args, .. } => {
                self.temp_live(*receiver, false);
                for &(_, id) in args {
                    self.temp_live(id, false);
                }
            }
            Op::IndexValue { base, index, .. } => {
                self.temp_live(*base, false);
                self.temp_live(*index, false);
            }
            Op::BorrowArrayElement { index, .. } => self.temp_live(*index, false),
            Op::BuildVariant {
                payload: Some(id), ..
            } => self.temp_live(*id, false),
            Op::BuildVariant { payload: None, .. } => {}
            Op::BranchVariant { value, .. } => self.temp_live(*value, false),
            Op::Unary { operand, .. } => self.temp_live(*operand, false),
            Op::Binary { left, right, .. } => {
                self.temp_live(*left, false);
                self.temp_live(*right, false);
            }
            Op::CompoundStore { loan, old, rhs, .. } => {
                self.temp_live(*loan, false);
                self.temp_live(*old, false);
                self.temp_live(*rhs, false);
            }
            Op::WritePlace { value, .. } | Op::DropValue(value) => self.temp_live(*value, false),
            Op::BindView { value, hold, .. } => self.temp_live(*value, *hold),
            Op::WriteIndex { indexes, value, .. } => {
                for &id in indexes {
                    self.temp_live(id, false);
                }
                self.temp_live(*value, false);
            }
            Op::CallDirect { args, .. } | Op::BuildGenerator { args, .. } => {
                for &(_, id) in args {
                    self.temp_live(id, false);
                }
            }
            Op::ResumeGenerator { receiver, .. } => self.temp_live(*receiver, false),
            _ => {}
        }
    }
    fn union_slot(
        &mut self,
        slot: &mut Option<LocalId>,
        value: TempId,
        branch_ty: Ty,
        resolved: &Ty,
        span: &Span,
    ) -> Result<(), CompileError> {
        if branch_ty != *resolved && branch_ty != Ty::Never {
            return Err(CompileError::InvalidIr("branch value type"));
        }
        let id = if let Some(id) = slot {
            *id
        } else {
            let id = LocalId(self.locals.len());
            self.locals.push(Local {
                id,
                name: String::new(),
                name_span: span.clone(),
                ty: resolved.clone(),
                mutable: true,
                scope: self.scope,
                parameter_mode: None,
            });
            self.name_spans.push(span.clone());
            *slot = Some(id);
            id
        };
        self.statement_op(
            Op::WritePlace { local: id, value },
            span.clone(),
            Access::Write,
            false,
        );
        Ok(())
    }
    fn begin_if_expression<'b>(
        &mut self,
        then_block: &'b parse::Block,
        otherwise: Option<&'b parse::Else>,
        span: &'b Span,
        depth: usize,
        expected: Option<&Ty>,
        test_flow: Flow,
    ) -> Result<IfExprStarted<'b>, CompileError> {
        let Some(otherwise) = otherwise else {
            // source parsing already emits missing_else at the `if`
            // token and stops before lowering. This fallback is reachable
            // only from an AST fabricated inside a test.
            let primary = Span {
                file: span.file.clone(),
                start: span.start,
                end: span.start + 2,
            };
            return Err(CompileError::Rejected(vec![
                crate::check::names::diagnostic(
                    "NX-E1302",
                    Some("missing_else"),
                    diagnostic::Stage::Parse,
                    diagnostic::Operation::SyntaxParse,
                    primary,
                ),
            ]));
        };
        let condition_diverges = test_flow.diverged();
        // expr() already enforces the Bool expectation for a live value;
        // Never is the only non-Bool flow admitted here. This guard is a
        // defensive invariant check and cannot change a source diagnostic.
        if test_flow
            .ty()
            .is_some_and(|ty| *ty != Ty::Bool && *ty != Ty::Never)
            && !condition_diverges
        {
            return Err(CompileError::InvalidIr("if condition type"));
        }
        if condition_diverges {
            self.emission = EmissionMode::Analyze;
        }
        let branch_mode = self.emission;
        let branch = if branch_mode == EmissionMode::Live {
            let test = test_flow
                .temp()
                .ok_or(CompileError::InvalidIr("if condition flow"))?;
            Some(self.finish_block(Terminator::BranchBool {
                condition: test,
                yes: BlockId(self.blocks.len() + 1),
                no: BlockId(usize::MAX),
                span: span.clone(),
                charges: vec![],
            }))
        } else {
            None
        };
        if condition_diverges {
            self.terminated = None;
        }
        let temp_checkpoint = self.temp_undo.len();
        let prior_locals = self.locals.len();
        let checkpoint = self.consumed_undo.len();
        let diagnostics = Vec::new();
        let dependent = false;
        let slot = None;
        let inferred_ty = expected
            .cloned()
            .ok_or(CompileError::InvalidIr("missing checker branch type"))?;
        self.dead_depth += 1;
        Ok(IfExprStarted {
            otherwise,
            span,
            then_block,
            depth,
            branch_mode,
            branch,
            condition_diverges,
            temp_checkpoint,
            prior_locals,
            checkpoint,
            diagnostics,
            dependent,
            slot,
            inferred_ty,
        })
    }
    fn finish_if_then_expression<'b>(
        &mut self,
        started: IfExprStarted<'b>,
        then_value_result: Result<Option<Flow>, CompileError>,
    ) -> Result<IfExprState<'b>, CompileError> {
        let IfExprStarted {
            otherwise,
            span,
            then_block,
            depth,
            branch_mode,
            branch,
            condition_diverges,
            temp_checkpoint,
            prior_locals,
            checkpoint,
            mut diagnostics,
            mut dependent,
            mut slot,
            inferred_ty,
        } = started;
        let then_value = match then_value_result {
            Ok(value) => value,
            Err(CompileError::Rejected(errors)) => {
                diagnostics.extend(errors.into_iter().map(|d| *d));
                None
            }
            Err(CompileError::Dependent) => {
                dependent = true;
                None
            }
            Err(error) => return Err(branch_error(error, diagnostics)),
        };
        self.emission = branch_mode;
        if let Some(Flow::Value {
            temp: Some(temp),
            ty,
        }) = then_value
        {
            self.union_slot(&mut slot, temp, ty, &inferred_ty, &then_block.span)?;
        }
        let then_continues = self.terminated.is_none();
        let return_value = self.terminated.take();
        let then_end = if branch_mode == EmissionMode::Analyze
            || return_value == Some(Termination::Diverged)
        {
            None
        } else {
            Some(
                self.finish_block(if let Some(Termination::Return(value)) = return_value {
                    Terminator::Return {
                        value: Some(value),
                        span: then_block.span.clone(),
                        charges: vec![],
                    }
                } else {
                    Terminator::Jump {
                        target: BlockId(usize::MAX),
                        span: then_block.span.clone(),
                        charges: vec![],
                    }
                }),
            )
        };
        let then_state = self.restore_availability(checkpoint);
        self.restore_temps(temp_checkpoint);
        if let Some(branch) = branch {
            self.enter_edge(branch, 1)?;
        }
        self.dead_depth -= 1;
        let else_start = BlockId(self.blocks.len());
        if let Some(branch) = branch
            && let Terminator::BranchBool { no, .. } = &mut self.blocks[branch.0].terminator
        {
            *no = else_start;
        }
        Ok(IfExprState {
            otherwise,
            span,
            depth,
            branch_mode,
            condition_diverges,
            temp_checkpoint,
            prior_locals,
            checkpoint,
            diagnostics,
            dependent,
            slot,
            inferred_ty,
            then_continues,
            then_end,
            then_state,
        })
    }
    fn finish_if_expression(
        &mut self,
        state: IfExprState<'_>,
        else_value_result: Result<Option<Flow>, CompileError>,
    ) -> Result<Flow, CompileError> {
        let IfExprState {
            otherwise,
            span,
            depth: _,
            branch_mode,
            condition_diverges,
            temp_checkpoint,
            prior_locals,
            checkpoint,
            mut diagnostics,
            mut dependent,
            mut slot,
            inferred_ty,
            then_continues,
            then_end,
            then_state,
        } = state;
        let else_value = match else_value_result {
            Ok(value) => value,
            Err(CompileError::Rejected(errors)) => {
                diagnostics.extend(errors.into_iter().map(|d| *d));
                None
            }
            Err(CompileError::Dependent) => {
                dependent = true;
                None
            }
            Err(error) => return Err(branch_error(error, diagnostics)),
        };
        self.emission = branch_mode;
        let else_span = match otherwise {
            parse::Else::Block(block) => &block.span,
            parse::Else::If(expr) => expr.span(),
        };
        if let Some(Flow::Value {
            temp: Some(temp),
            ty,
        }) = else_value
        {
            self.union_slot(&mut slot, temp, ty, &inferred_ty, else_span)?;
        }
        let else_continues = self.terminated.is_none();
        let return_value = self.terminated.take();
        let else_end = if branch_mode == EmissionMode::Analyze
            || return_value == Some(Termination::Diverged)
        {
            None
        } else {
            Some(
                self.finish_block(if let Some(Termination::Return(value)) = return_value {
                    Terminator::Return {
                        value: Some(value),
                        span: else_span.clone(),
                        charges: vec![],
                    }
                } else {
                    Terminator::Jump {
                        target: BlockId(usize::MAX),
                        span: else_span.clone(),
                        charges: vec![],
                    }
                }),
            )
        };
        let else_state = self.restore_availability(checkpoint);
        self.restore_temps(temp_checkpoint);
        if let Some(end) = [then_end, else_end]
            .into_iter()
            .flatten()
            .find(|end| matches!(self.blocks[end.0].terminator, Terminator::Jump { .. }))
        {
            self.enter_edge(end, 0)?;
        }
        self.dead_depth -= 1;
        let join = BlockId(self.blocks.len());
        if let Some(end) = then_end
            && let Terminator::Jump { target, .. } = &mut self.blocks[end.0].terminator
        {
            *target = join;
        }
        if let Some(end) = else_end
            && let Terminator::Jump { target, .. } = &mut self.blocks[end.0].terminator
        {
            *target = join;
        }
        self.merge_availability(
            then_state,
            else_state,
            then_continues,
            else_continues,
            prior_locals,
        );
        if !diagnostics.is_empty() {
            return Err(CompileError::Rejected(
                diagnostics.into_iter().map(Box::new).collect(),
            ));
        }
        if dependent {
            return Err(CompileError::Dependent);
        }
        if condition_diverges || (!then_continues && !else_continues) {
            self.terminated = Some(Termination::Diverged);
            self.emission = EmissionMode::Analyze;
            return Ok(Flow::Diverged {
                ty: (inferred_ty != Ty::Never).then_some(inferred_ty),
            });
        }
        let Some(slot) = slot else {
            if self.emission == EmissionMode::Analyze {
                return Ok(Flow::Value {
                    temp: None,
                    ty: inferred_ty,
                });
            }
            return Err(CompileError::InvalidIr("branch value slot"));
        };
        let ty = inferred_ty;
        if self.emission == EmissionMode::Analyze {
            return Ok(Flow::Value { temp: None, ty });
        }
        Ok(self.emitted(
            Op::TransferValue(slot),
            ty,
            span.clone(),
            Access::Own,
            false,
        ))
    }
    fn hidden_range_slot(&mut self, span: &Span, mutable: bool) -> LocalId {
        let id = LocalId(self.locals.len());
        self.locals.push(Local {
            id,
            name: String::new(),
            name_span: span.clone(),
            ty: Ty::I64,
            mutable,
            scope: self.scope,
            parameter_mode: None,
        });
        self.name_spans.push(span.clone());
        id
    }

    fn begin_for_range(
        &mut self,
        statement: &Statement,
        lower: Flow,
        upper: Flow,
        inclusive: bool,
        surviving_temps: HashSet<TempId>,
    ) -> Result<ForState, CompileError> {
        let Statement::For {
            name,
            name_span,
            iterable,
            body,
            span,
        } = statement
        else {
            return Err(CompileError::InvalidIr("range for statement"));
        };
        if lower.ty().is_some_and(|ty| *ty != Ty::I64)
            || upper.ty().is_some_and(|ty| *ty != Ty::I64)
        {
            return Err(CompileError::InvalidIr("range endpoint type"));
        }
        if lower.diverged() || upper.diverged() || self.emission == EmissionMode::Analyze {
            let saved = self.scope;
            self.scope = ScopeId(self.next_scope);
            self.next_scope += 1;
            self.scope_parents.push(Some(saved));
            self.scopes.push(vec![name.into()]);
            let binding = LocalId(self.locals.len());
            self.locals.push(Local {
                id: binding,
                name: name.into(),
                name_span: name_span.clone(),
                ty: Ty::I64,
                mutable: false,
                scope: self.scope,
                parameter_mode: None,
            });
            self.name_spans.push(name_span.clone());
            self.visible.entry(name.into()).or_default().push(binding);
            self.emission = EmissionMode::Analyze;
            return Ok(ForState::Analyze {
                saved,
                loan_root: None,
            });
        }
        let (lower_temp, upper_temp) = (
            lower
                .temp()
                .ok_or(CompileError::InvalidIr("range lower flow"))?,
            upper
                .temp()
                .ok_or(CompileError::InvalidIr("range upper flow"))?,
        );
        let origin_lower = self.hidden_range_slot(iterable.span(), false);
        let origin_upper = self.hidden_range_slot(iterable.span(), false);
        let cursor = self.hidden_range_slot(iterable.span(), true);
        let limit = self.hidden_range_slot(iterable.span(), false);
        for (local, value) in [(origin_lower, lower_temp), (origin_upper, upper_temp)] {
            self.statement_op(
                Op::WritePlace { local, value },
                iterable.span().clone(),
                Access::Write,
                false,
            );
        }
        for (source, target) in [(origin_lower, cursor), (origin_upper, limit)] {
            let copy = self
                .emitted(
                    Op::CopyValue(source),
                    Ty::I64,
                    iterable.span().clone(),
                    Access::Read,
                    false,
                )
                .temp()
                .ok_or(CompileError::InvalidIr("range slot copy"))?;
            self.statement_op(
                Op::WritePlace {
                    local: target,
                    value: copy,
                },
                iterable.span().clone(),
                Access::Write,
                false,
            );
        }
        for local in [origin_upper, origin_lower] {
            let value = self
                .emitted(
                    Op::TransferValue(local),
                    Ty::I64,
                    iterable.span().clone(),
                    Access::Own,
                    false,
                )
                .temp()
                .ok_or(CompileError::InvalidIr("range scratch transfer"))?;
            self.statement_op(
                Op::DropValue(value),
                iterable.span().clone(),
                Access::Own,
                false,
            );
        }
        self.finish_block(Terminator::Jump {
            target: BlockId(self.blocks.len() + 1),
            span: span.clone(),
            charges: vec![],
        });
        let header = BlockId(self.blocks.len());
        let for_token = Span {
            file: span.file.clone(),
            start: span.start,
            end: span.start + 3,
        };
        self.pending_charges
            .push(WorkPlanner::charge(WorkEvent::RangeTest, for_token.clone()));
        let test = self
            .emitted(
                Op::RangeTest {
                    cursor,
                    limit,
                    inclusive,
                    final_test: false,
                },
                Ty::Bool,
                for_token.clone(),
                Access::Read,
                false,
            )
            .temp()
            .ok_or(CompileError::InvalidIr("range entry test"))?;
        let branch = self.finish_block(Terminator::BranchBool {
            condition: test,
            yes: BlockId(self.blocks.len() + 1),
            no: BlockId(usize::MAX),
            span: for_token.clone(),
            charges: vec![],
        });
        let saved = self.scope;
        let scope_depth = self.scopes.len();
        self.scope = ScopeId(self.next_scope);
        self.next_scope += 1;
        self.scope_parents.push(Some(saved));
        self.scopes.push(vec![name.into()]);
        let binding = LocalId(self.locals.len());
        self.locals.push(Local {
            id: binding,
            name: name.into(),
            name_span: name_span.clone(),
            ty: Ty::I64,
            mutable: false,
            scope: self.scope,
            parameter_mode: None,
        });
        self.name_spans.push(name_span.clone());
        self.visible.entry(name.into()).or_default().push(binding);
        let index = self
            .emitted(
                Op::CopyValue(cursor),
                Ty::I64,
                name_span.clone(),
                Access::Read,
                false,
            )
            .temp()
            .ok_or(CompileError::InvalidIr("range binding"))?;
        self.statement_op(
            Op::WritePlace {
                local: binding,
                value: index,
            },
            name_span.clone(),
            Access::Write,
            false,
        );
        self.loop_stack.push(LoopContext {
            continue_target: BlockId(usize::MAX),
            scope_depth,
            cleanup_span: body.close_span.clone(),
            breaks: Vec::new(),
            continues: Vec::new(),
            match_depth: self.match_boundaries.len(),
            surviving_temps,
        });
        let availability_checkpoint = self.consumed_undo.len();
        self.dead_depth += 1;
        Ok(ForState::RangeLive {
            header,
            branch,
            saved,
            availability_checkpoint,
            binding,
            cursor,
            limit,
            inclusive,
        })
    }

    fn begin_for_array(
        &mut self,
        statement: &Statement,
        flow: Flow,
        surviving_temps: HashSet<TempId>,
    ) -> Result<ForState, CompileError> {
        let Statement::For {
            name,
            name_span,
            iterable,
            body,
            span,
        } = statement
        else {
            return Err(CompileError::InvalidIr("array for statement"));
        };
        let Some(Ty::Array(item)) = flow.ty() else {
            return Err(CompileError::InvalidIr("array for iterable type"));
        };
        let item = clone_ty_iterative(item);
        let array_ty =
            clone_ty_iterative(flow.ty().ok_or(CompileError::InvalidIr("array for type"))?);
        if flow.diverged() || self.emission == EmissionMode::Analyze {
            let loan_root = place_parts(iterable)
                .ok()
                .and_then(|(root, _)| place_name(root))
                .map(|name| self.get(name))
                .transpose()?;
            if let Some(root) = loan_root {
                self.active_loans.entry(root).or_default().push(ActiveLoan {
                    permission: LoanPermission::Read,
                    span: iterable.span().clone(),
                    loan: LoanId(iterable.span().start as usize),
                    parent: None,
                    compound_assignment: false,
                });
            }
            let saved = self.scope;
            self.scope = ScopeId(self.next_scope);
            self.next_scope += 1;
            self.scope_parents.push(Some(saved));
            self.scopes.push(vec![name.into()]);
            let binding = LocalId(self.locals.len());
            self.locals.push(Local {
                id: binding,
                name: name.into(),
                name_span: name_span.clone(),
                ty: item,
                mutable: false,
                scope: self.scope,
                parameter_mode: Some(ParameterMode::Read),
            });
            self.name_spans.push(name_span.clone());
            self.visible.entry(name.into()).or_default().push(binding);
            self.emission = EmissionMode::Analyze;
            return Ok(ForState::Analyze { saved, loan_root });
        }
        let value = flow
            .temp()
            .ok_or(CompileError::InvalidIr("array for flow"))?;
        let borrowed_root = place_parts(iterable)
            .ok()
            .and_then(|(root, _)| place_name(root))
            .map(|name| self.get(name))
            .transpose()?;
        let (root, owned_root) = if let Some(root) = borrowed_root {
            (root, false)
        } else {
            let root = LocalId(self.locals.len());
            self.locals.push(Local {
                id: root,
                name: String::new(),
                name_span: iterable.span().clone(),
                ty: clone_ty_iterative(&array_ty),
                mutable: false,
                scope: self.scope,
                parameter_mode: None,
            });
            self.name_spans.push(iterable.span().clone());
            self.statement_op(
                Op::WritePlace { local: root, value },
                iterable.span().clone(),
                Access::Write,
                false,
            );
            (root, true)
        };
        let source = if owned_root {
            self.emitted(
                Op::BorrowValue(root),
                clone_ty_iterative(&array_ty),
                iterable.span().clone(),
                Access::Read,
                false,
            )
            .temp()
            .ok_or(CompileError::InvalidIr("array iteration source"))?
        } else {
            value
        };
        let loan = LoanId(iterable.span().start as usize);
        self.conflict(root, iterable.span().clone(), "borrow_read")?;
        let guard = self
            .emitted(
                Op::BeginLoan {
                    root,
                    indexes: Vec::new(),
                    site: IndexSite {
                        context: iterable.span().clone(),
                        selectors: Vec::new(),
                    },
                    loan,
                    origin_span: iterable.span().clone(),
                    permission: LoanPermission::Read,
                    parent: (self.locals[root.0].parameter_mode == Some(ParameterMode::Inout))
                        .then_some(root),
                },
                clone_ty_iterative(&self.locals[root.0].ty),
                iterable.span().clone(),
                Access::Read,
                false,
            )
            .temp()
            .ok_or(CompileError::InvalidIr("array for guard"))?;
        self.active_loans.entry(root).or_default().push(ActiveLoan {
            permission: LoanPermission::Read,
            span: iterable.span().clone(),
            loan,
            parent: (self.locals[root.0].parameter_mode == Some(ParameterMode::Inout))
                .then_some(root),
            compound_assignment: false,
        });
        let cursor = self.hidden_range_slot(iterable.span(), true);
        let limit = self.hidden_range_slot(iterable.span(), false);
        let zero = self
            .emitted(
                Op::ConstI64(0),
                Ty::I64,
                iterable.span().clone(),
                Access::Own,
                false,
            )
            .temp()
            .ok_or(CompileError::InvalidIr("array cursor zero"))?;
        self.statement_op(
            Op::WritePlace {
                local: cursor,
                value: zero,
            },
            iterable.span().clone(),
            Access::Write,
            false,
        );
        self.pending_charges.push(WorkPlanner::charge(
            WorkEvent::ArrayIterationLength,
            iterable.span().clone(),
        ));
        let length = self
            .emitted(
                Op::BeginArrayIteration(source),
                Ty::I64,
                iterable.span().clone(),
                Access::Read,
                false,
            )
            .temp()
            .ok_or(CompileError::InvalidIr("array length"))?;
        self.statement_op(
            Op::WritePlace {
                local: limit,
                value: length,
            },
            iterable.span().clone(),
            Access::Write,
            false,
        );
        self.finish_block(Terminator::Jump {
            target: BlockId(self.blocks.len() + 1),
            span: span.clone(),
            charges: vec![],
        });
        let header = BlockId(self.blocks.len());
        let for_token = Span {
            file: span.file.clone(),
            start: span.start,
            end: span.start + 3,
        };
        self.pending_charges
            .push(WorkPlanner::charge(WorkEvent::RangeTest, for_token.clone()));
        let test = self
            .emitted(
                Op::RangeTest {
                    cursor,
                    limit,
                    inclusive: false,
                    final_test: false,
                },
                Ty::Bool,
                for_token.clone(),
                Access::Read,
                false,
            )
            .temp()
            .ok_or(CompileError::InvalidIr("array test"))?;
        let branch = self.finish_block(Terminator::BranchBool {
            condition: test,
            yes: BlockId(self.blocks.len() + 1),
            no: BlockId(usize::MAX),
            span: for_token.clone(),
            charges: vec![],
        });
        let saved = self.scope;
        let scope_depth = self.scopes.len();
        self.scope = ScopeId(self.next_scope);
        self.next_scope += 1;
        self.scope_parents.push(Some(saved));
        self.scopes.push(vec![name.into()]);
        let binding = LocalId(self.locals.len());
        self.locals.push(Local {
            id: binding,
            name: name.into(),
            name_span: name_span.clone(),
            ty: clone_ty_iterative(&item),
            mutable: false,
            scope: self.scope,
            parameter_mode: Some(ParameterMode::Read),
        });
        self.name_spans.push(name_span.clone());
        self.visible.entry(name.into()).or_default().push(binding);
        let index = self
            .emitted(
                Op::BorrowValue(cursor),
                Ty::I64,
                name_span.clone(),
                Access::Read,
                false,
            )
            .temp()
            .ok_or(CompileError::InvalidIr("array index"))?;
        let selected = self
            .emitted(
                Op::BorrowArrayElement {
                    source,
                    index,
                    site: IndexSite {
                        context: name_span.clone(),
                        selectors: vec![name_span.clone()],
                    },
                },
                item,
                name_span.clone(),
                Access::Read,
                false,
            )
            .temp()
            .ok_or(CompileError::InvalidIr("array element alias"))?;
        self.statement_op(
            Op::BindView {
                local: binding,
                value: selected,
                hold: false,
            },
            name_span.clone(),
            Access::Write,
            false,
        );
        let mut surviving_temps = surviving_temps;
        surviving_temps.insert(guard);
        surviving_temps.insert(source);
        self.loop_stack.push(LoopContext {
            continue_target: BlockId(usize::MAX),
            scope_depth,
            cleanup_span: body.close_span.clone(),
            breaks: Vec::new(),
            continues: Vec::new(),
            match_depth: self.match_boundaries.len(),
            surviving_temps,
        });
        let availability_checkpoint = self.consumed_undo.len();
        self.dead_depth += 1;
        Ok(ForState::ArrayLive {
            header,
            branch,
            saved,
            availability_checkpoint,
            binding,
            cursor,
            limit,
            root,
            owned_root,
            guard,
            source,
        })
    }

    fn begin_for_after_iterable(
        &mut self,
        statement: &Statement,
        flow: Flow,
        surviving_temps: HashSet<TempId>,
    ) -> Result<ForState, CompileError> {
        if self.literal_true_loops && matches!(flow.ty(), Some(Ty::Array(_))) {
            return self.begin_for_array(statement, flow, surviving_temps);
        }
        let Statement::For {
            name,
            name_span,
            iterable,
            body,
            span,
        } = statement
        else {
            return Err(CompileError::InvalidIr("for statement"));
        };
        let Some(Ty::Generator { item, .. }) = flow.ty() else {
            return Err(CompileError::InvalidIr("for iterable type"));
        };
        let item = item.as_ref().clone();
        let generator_ty = flow
            .ty()
            .ok_or(CompileError::InvalidIr("for iterator type"))?
            .clone();
        if flow.diverged() || self.emission == EmissionMode::Analyze {
            let saved = self.scope;
            self.scope = ScopeId(self.next_scope);
            self.next_scope += 1;
            self.scope_parents.push(Some(saved));
            self.scopes.push(vec![name.into()]);
            let id = LocalId(self.locals.len());
            self.locals.push(Local {
                id,
                name: name.into(),
                name_span: name_span.clone(),
                ty: item,
                mutable: false,
                scope: self.scope,
                parameter_mode: None,
            });
            self.name_spans.push(name_span.clone());
            self.visible.entry(name.into()).or_default().push(id);
            self.emission = EmissionMode::Analyze;
            return Ok(ForState::Analyze {
                saved,
                loan_root: None,
            });
        }
        let iterator = LocalId(self.locals.len());
        self.name_spans.push(span.clone());
        self.locals.push(Local {
            id: iterator,
            name: String::new(),
            name_span: span.clone(),
            ty: generator_ty.clone(),
            mutable: true,
            scope: self.scope,
            parameter_mode: None,
        });
        let state = flow
            .temp()
            .ok_or(CompileError::InvalidIr("for iterator flow"))?;
        self.statement_op(
            Op::WritePlace {
                local: iterator,
                value: state,
            },
            iterable.span().clone(),
            Access::Write,
            false,
        );
        self.finish_block(Terminator::Jump {
            target: BlockId(self.blocks.len() + 1),
            span: span.clone(),
            charges: vec![],
        });
        let header = BlockId(self.blocks.len());
        let for_token = Span {
            file: span.file.clone(),
            start: span.start,
            end: span.start + 3,
        };
        let loan_id = LoanId(span.start as usize);
        let loan = self
            .emitted(
                Op::BeginLoan {
                    root: iterator,
                    indexes: Vec::new(),
                    site: IndexSite {
                        context: for_token.clone(),
                        selectors: Vec::new(),
                    },
                    loan: loan_id,
                    origin_span: for_token.clone(),
                    permission: LoanPermission::Inout,
                    parent: None,
                },
                generator_ty,
                for_token.clone(),
                Access::Read,
                false,
            )
            .temp()
            .ok_or(CompileError::InvalidIr("for loan"))?;
        let option_ty = Ty::option(Box::new(item.clone()));
        self.pending_charges.push(WorkCharge {
            ticks: 1,
            origin: for_token.clone(),
        });
        let next = self
            .emitted(
                Op::ResumeGenerator {
                    receiver: loan,
                    item: item.clone(),
                },
                option_ty.clone(),
                for_token.clone(),
                Access::Own,
                false,
            )
            .temp()
            .ok_or(CompileError::InvalidIr("for next"))?;
        self.statement_op(Op::EndLoan(loan_id), for_token.clone(), Access::Own, false);
        let option = LocalId(self.locals.len());
        self.name_spans.push(for_token.clone());
        self.locals.push(Local {
            id: option,
            name: String::new(),
            name_span: for_token.clone(),
            ty: option_ty,
            mutable: true,
            scope: self.scope,
            parameter_mode: None,
        });
        self.statement_op(
            Op::WritePlace {
                local: option,
                value: next,
            },
            for_token.clone(),
            Access::Write,
            false,
        );
        let test = self
            .emitted(
                Op::TestVariant {
                    local: option,
                    path: Vec::new(),
                    tag: crate::union::VariantTag(1),
                },
                Ty::Bool,
                for_token.clone(),
                Access::Read,
                false,
            )
            .temp()
            .ok_or(CompileError::InvalidIr("for option test"))?;
        let branch = self.finish_block(Terminator::BranchBool {
            condition: test,
            yes: BlockId(self.blocks.len() + 1),
            no: BlockId(usize::MAX),
            span: for_token.clone(),
            charges: vec![],
        });
        let saved = self.scope;
        let scope_depth = self.scopes.len();
        self.scope = ScopeId(self.next_scope);
        self.next_scope += 1;
        self.scope_parents.push(Some(saved));
        self.scopes.push(vec![name.into()]);
        let binding = LocalId(self.locals.len());
        self.locals.push(Local {
            id: binding,
            name: name.into(),
            name_span: name_span.clone(),
            ty: item.clone(),
            mutable: false,
            scope: self.scope,
            parameter_mode: None,
        });
        self.name_spans.push(name_span.clone());
        self.visible.entry(name.into()).or_default().push(binding);
        let payload = self
            .emitted(
                Op::ExtractPayload {
                    local: option,
                    path: vec![crate::union::VariantTag(1)],
                },
                item,
                name_span.clone(),
                Access::Write,
                false,
            )
            .temp()
            .ok_or(CompileError::InvalidIr("for payload"))?;
        self.statement_op(
            Op::WritePlace {
                local: binding,
                value: payload,
            },
            name_span.clone(),
            Access::Write,
            false,
        );
        self.loop_stack.push(LoopContext {
            continue_target: BlockId(usize::MAX),
            scope_depth,
            cleanup_span: body.close_span.clone(),
            breaks: Vec::new(),
            continues: Vec::new(),
            match_depth: self.match_boundaries.len(),
            surviving_temps,
        });
        let availability_checkpoint = self.consumed_undo.len();
        self.dead_depth += 1;
        Ok(ForState::Live {
            header,
            branch,
            saved,
            availability_checkpoint,
            binding,
            option,
            iterator,
        })
    }
    fn finish_for_counted_after_body(
        &mut self,
        state: ForState,
        body: &parse::Block,
        body_result: Result<(), CompileError>,
    ) -> Result<(), CompileError> {
        let (
            header,
            branch,
            saved,
            availability_checkpoint,
            binding,
            cursor,
            limit,
            inclusive,
            array,
        ) = match state {
            ForState::RangeLive {
                header,
                branch,
                saved,
                availability_checkpoint,
                binding,
                cursor,
                limit,
                inclusive,
            } => (
                header,
                branch,
                saved,
                availability_checkpoint,
                binding,
                cursor,
                limit,
                inclusive,
                None,
            ),
            ForState::ArrayLive {
                header,
                branch,
                saved,
                availability_checkpoint,
                binding,
                cursor,
                limit,
                root,
                owned_root,
                guard,
                source,
            } => (
                header,
                branch,
                saved,
                availability_checkpoint,
                binding,
                cursor,
                limit,
                false,
                Some((root, owned_root, guard, source)),
            ),
            _ => return Err(CompileError::InvalidIr("counted for state")),
        };
        let loop_context = self
            .loop_stack
            .pop()
            .ok_or(CompileError::InvalidIr("range loop stack"))?;
        body_result?;
        self.emission = EmissionMode::Live;
        let body_return = self.terminated.take();
        let normal_backedge = body_return.is_none();
        if body_return != Some(Termination::Diverged) {
            let local = (binding, body.close_span.clone());
            if matches!(body_return, Some(Termination::Return(_))) {
                self.deferred_return_locals.push(local);
            } else {
                self.plan_exit(
                    ExitPlan {
                        surviving: loop_context.surviving_temps.clone(),
                        locals: vec![local],
                        reserved: None,
                    },
                    &body.close_span,
                );
            }
            self.finish_block(match body_return {
                Some(Termination::Return(value)) => Terminator::Return {
                    value: Some(value),
                    span: body.span.clone(),
                    charges: vec![],
                },
                _ => Terminator::Jump {
                    target: BlockId(self.blocks.len() + 1),
                    span: body.span.clone(),
                    charges: vec![],
                },
            });
        }
        self.leave_scope(saved)?;
        self.restore_availability(availability_checkpoint);
        self.dead_depth -= 1;
        let mut final_branch = None;
        if normal_backedge || !loop_context.continues.is_empty() {
            let advance = BlockId(self.blocks.len());
            if !normal_backedge {
                self.enter_edge(loop_context.continues[0], 0)?;
            }
            let Terminator::BranchBool {
                span: for_token, ..
            } = &self.blocks[branch.0].terminator
            else {
                return Err(CompileError::InvalidIr("range header branch"));
            };
            let for_token = for_token.clone();
            if inclusive {
                self.pending_charges.push(WorkPlanner::charge(
                    WorkEvent::RangeFinalTest,
                    for_token.clone(),
                ));
                let last = self
                    .emitted(
                        Op::RangeTest {
                            cursor,
                            limit,
                            inclusive: true,
                            final_test: true,
                        },
                        Ty::Bool,
                        for_token.clone(),
                        Access::Read,
                        false,
                    )
                    .temp()
                    .ok_or(CompileError::InvalidIr("range final test"))?;
                let test_block = self.finish_block(Terminator::BranchBool {
                    condition: last,
                    yes: BlockId(usize::MAX),
                    no: BlockId(self.blocks.len() + 1),
                    span: for_token.clone(),
                    charges: vec![],
                });
                self.enter_edge(test_block, 1)?;
                final_branch = Some(test_block);
            }
            self.pending_charges.push(WorkPlanner::charge(
                WorkEvent::RangeAdvance,
                for_token.clone(),
            ));
            self.statement_op(
                Op::RangeAdvance { cursor, limit },
                for_token.clone(),
                Access::Write,
                false,
            );
            self.finish_block(Terminator::Jump {
                target: header,
                span: body.span.clone(),
                charges: vec![],
            });
            for id in loop_context.continues {
                if let Terminator::Jump { target, .. } = &mut self.blocks[id.0].terminator {
                    *target = advance;
                }
            }
        }
        let exit = BlockId(self.blocks.len());
        self.enter_edge(branch, 1)?;
        if let Terminator::BranchBool { no, .. } = &mut self.blocks[branch.0].terminator {
            *no = exit;
        }
        if let Some(final_branch) = final_branch
            && let Terminator::BranchBool { yes, .. } = &mut self.blocks[final_branch.0].terminator
        {
            *yes = exit;
        }
        for id in loop_context.breaks {
            if let Terminator::Jump { target, .. } = &mut self.blocks[id.0].terminator {
                *target = exit;
            }
        }
        if let Some((_, _, guard, source)) = array {
            self.statement_op(
                Op::DropValue(guard),
                body.close_span.clone(),
                Access::Own,
                false,
            );
            self.statement_op(
                Op::DropValue(source),
                body.close_span.clone(),
                Access::Own,
                false,
            );
        }
        let mut locals = vec![
            (limit, body.close_span.clone()),
            (cursor, body.close_span.clone()),
        ];
        if let Some((root, true, _, _)) = array {
            locals.push((root, body.close_span.clone()));
        }
        self.plan_exit(
            ExitPlan {
                surviving: self.live_temps.clone(),
                locals,
                reserved: None,
            },
            &body.close_span,
        );
        Ok(())
    }

    fn finish_for_after_body(
        &mut self,
        state: ForState,
        body: &parse::Block,
        body_result: Result<(), CompileError>,
    ) -> Result<(), CompileError> {
        if matches!(
            state,
            ForState::RangeLive { .. } | ForState::ArrayLive { .. }
        ) {
            return self.finish_for_counted_after_body(state, body, body_result);
        }
        let ForState::Live {
            header,
            branch,
            saved,
            availability_checkpoint,
            binding,
            option,
            iterator,
        } = state
        else {
            let ForState::Analyze { saved, loan_root } = state else {
                unreachable!()
            };
            self.leave_scope(saved)?;
            if let Some(root) = loan_root {
                self.active_loans
                    .get_mut(&root)
                    .expect("array for hypothetical loan")
                    .pop();
            }
            return body_result;
        };
        let loop_context = self
            .loop_stack
            .pop()
            .ok_or(CompileError::InvalidIr("for loop stack"))?;
        body_result?;
        self.emission = EmissionMode::Live;
        let body_return = self.terminated.take();
        let normal_backedge = body_return.is_none();
        if body_return != Some(Termination::Diverged) {
            let local = (binding, body.close_span.clone());
            if matches!(body_return, Some(Termination::Return(_))) {
                self.deferred_return_locals.push(local);
            } else {
                self.plan_exit(
                    ExitPlan {
                        surviving: loop_context.surviving_temps.clone(),
                        locals: vec![local],
                        reserved: None,
                    },
                    &body.close_span,
                );
            }
            self.finish_block(match body_return {
                Some(Termination::Return(value)) => Terminator::Return {
                    value: Some(value),
                    span: body.span.clone(),
                    charges: vec![],
                },
                _ => Terminator::Jump {
                    target: BlockId(self.blocks.len() + 1),
                    span: body.span.clone(),
                    charges: vec![],
                },
            });
        }
        self.leave_scope(saved)?;
        self.restore_availability(availability_checkpoint);
        self.dead_depth -= 1;
        if normal_backedge || !loop_context.continues.is_empty() {
            let advance = BlockId(self.blocks.len());
            if !normal_backedge {
                self.enter_edge(loop_context.continues[0], 0)?;
            }
            self.plan_exit(
                ExitPlan {
                    surviving: self.live_temps.clone(),
                    locals: vec![(option, body.close_span.clone())],
                    reserved: None,
                },
                &body.close_span,
            );
            self.finish_block(Terminator::Jump {
                target: header,
                span: body.span.clone(),
                charges: vec![],
            });
            for id in loop_context.continues {
                if let Terminator::Jump { target, .. } = &mut self.blocks[id.0].terminator {
                    *target = advance;
                }
            }
        }
        let exit = BlockId(self.blocks.len());
        self.enter_edge(branch, 1)?;
        if let Terminator::BranchBool { no, .. } = &mut self.blocks[branch.0].terminator {
            *no = exit;
        }
        for id in loop_context.breaks {
            if let Terminator::Jump { target, .. } = &mut self.blocks[id.0].terminator {
                *target = exit;
            }
        }
        self.plan_exit(
            ExitPlan {
                surviving: self.live_temps.clone(),
                locals: vec![
                    (option, body.close_span.clone()),
                    (iterator, body.close_span.clone()),
                ],
                reserved: None,
            },
            &body.close_span,
        );
        Ok(())
    }
    fn begin_if_statement(
        &mut self,
        span: &Span,
        condition_flow: Flow,
    ) -> Result<IfBeginState, CompileError> {
        let entry_emission = self.emission;
        let condition_diverges = condition_flow.diverged();
        if condition_flow
            .ty()
            .is_some_and(|ty| *ty != Ty::Bool && *ty != Ty::Never)
            && !condition_diverges
        {
            return Err(CompileError::InvalidIr("if condition type"));
        }
        if condition_diverges {
            self.emission = EmissionMode::Analyze;
        }
        let branch_mode = self.emission;
        let branch = if branch_mode == EmissionMode::Live {
            let condition = condition_flow
                .temp()
                .ok_or(CompileError::InvalidIr("if condition flow"))?;
            Some(self.finish_block(Terminator::BranchBool {
                condition,
                yes: BlockId(self.blocks.len() + 1),
                no: BlockId(usize::MAX),
                span: span.clone(),
                charges: vec![],
            }))
        } else {
            None
        };
        if condition_diverges {
            self.terminated = None;
        }
        let temp_checkpoint = self.temp_undo.len();
        let prior_locals = self.locals.len();
        let checkpoint = self.consumed_undo.len();
        self.dead_depth += 1;
        Ok(IfBeginState {
            entry_emission,
            branch_mode,
            branch,
            condition_diverges,
            temp_checkpoint,
            prior_locals,
            checkpoint,
        })
    }
    fn finish_if_then(
        &mut self,
        started: IfBeginState,
        then_block: &parse::Block,
    ) -> Result<IfState, CompileError> {
        let IfBeginState {
            entry_emission,
            branch_mode,
            branch,
            condition_diverges,
            temp_checkpoint,
            prior_locals,
            checkpoint,
        } = started;
        self.emission = branch_mode;
        let then_continues = self.terminated.is_none();
        let then_value = self.terminated.take();
        let then_end =
            if branch_mode == EmissionMode::Analyze || then_value == Some(Termination::Diverged) {
                None
            } else {
                Some(
                    self.finish_block(if let Some(Termination::Return(value)) = then_value {
                        Terminator::Return {
                            value: Some(value),
                            span: then_block.span.clone(),
                            charges: vec![],
                        }
                    } else {
                        Terminator::Jump {
                            target: BlockId(usize::MAX),
                            span: then_block.span.clone(),
                            charges: vec![],
                        }
                    }),
                )
            };
        let then_state = self.restore_availability(checkpoint);
        self.restore_temps(temp_checkpoint);
        if let Some(branch) = branch {
            self.enter_edge(branch, 1)?;
        }
        self.dead_depth -= 1;
        let else_start = BlockId(self.blocks.len());
        if let Some(branch) = branch
            && let Terminator::BranchBool { no, .. } = &mut self.blocks[branch.0].terminator
        {
            *no = else_start;
        }
        Ok(IfState {
            entry_emission,
            branch_mode,
            branch,
            condition_diverges,
            temp_checkpoint,
            prior_locals,
            checkpoint,
            then_continues,
            then_end,
            then_state,
        })
    }
    fn finish_if_else(
        &mut self,
        state: IfState,
        else_span: Option<Span>,
    ) -> Result<(), CompileError> {
        let has_else = else_span.is_some();
        let IfState {
            entry_emission,
            branch_mode,
            branch,
            condition_diverges,
            temp_checkpoint,
            prior_locals,
            checkpoint,
            then_continues,
            then_end,
            then_state,
        } = state;
        let (else_state, else_continues, else_end) = if let Some(else_span) = else_span {
            self.emission = branch_mode;
            let continues = self.terminated.is_none();
            let else_value = self.terminated.take();
            let end = if branch_mode == EmissionMode::Analyze
                || else_value == Some(Termination::Diverged)
            {
                None
            } else {
                Some(
                    self.finish_block(if let Some(Termination::Return(value)) = else_value {
                        Terminator::Return {
                            value: Some(value),
                            span: else_span.clone(),
                            charges: vec![],
                        }
                    } else {
                        Terminator::Jump {
                            target: BlockId(usize::MAX),
                            span: else_span,
                            charges: vec![],
                        }
                    }),
                )
            };
            let state = self.restore_availability(checkpoint);
            self.restore_temps(temp_checkpoint);
            if let Some(end) = [then_end, end]
                .into_iter()
                .flatten()
                .find(|end| matches!(self.blocks[end.0].terminator, Terminator::Jump { .. }))
            {
                self.enter_edge(end, 0)?;
            }
            self.dead_depth -= 1;
            (state, continues, end)
        } else {
            (HashMap::new(), true, None)
        };
        let join = BlockId(self.blocks.len());
        if let Some(end) = then_end
            && let Terminator::Jump { target, .. } = &mut self.blocks[end.0].terminator
        {
            *target = join;
        }
        if let Some(end) = else_end {
            if let Terminator::Jump { target, .. } = &mut self.blocks[end.0].terminator {
                *target = join;
            }
        } else if !has_else
            && let Some(branch) = branch
            && let Terminator::BranchBool { no, .. } = &mut self.blocks[branch.0].terminator
        {
            *no = join;
        }
        self.merge_availability(
            then_state,
            else_state,
            then_continues,
            else_continues,
            prior_locals,
        );
        if condition_diverges || (!then_continues && !else_continues) {
            // The enclosing block still checks later statements in a hypothetical state.
            self.terminated = Some(Termination::Diverged);
            self.emission = EmissionMode::Analyze;
        } else {
            self.emission = entry_emission;
        }
        Ok(())
    }
    fn availability(&self, id: LocalId) -> Availability {
        Availability {
            definitely: self.consumed_at.get(&id).cloned(),
            maybe: self.maybe_consumed_at.get(&id).cloned(),
        }
    }
    fn restore_availability(&mut self, checkpoint: usize) -> HashMap<LocalId, Availability> {
        let checkpoint = checkpoint.min(self.consumed_undo.len());
        let mut changed = HashMap::new();
        for (id, _, _) in &self.consumed_undo[checkpoint..] {
            super::count_cfg_work(1);
            changed.insert(*id, self.availability(*id));
        }
        while self.consumed_undo.len() > checkpoint {
            super::count_cfg_work(1);
            let Some((id, def, maybe)) = self.consumed_undo.pop() else {
                break;
            };
            if let Some(site) = def {
                self.consumed_at.insert(id, site);
            } else {
                self.consumed_at.remove(&id);
            }
            if let Some(site) = maybe {
                self.maybe_consumed_at.insert(id, site);
            } else {
                self.maybe_consumed_at.remove(&id);
            }
        }
        changed
    }
    fn set_availability(&mut self, id: LocalId, value: Availability) {
        if self.dead_depth != 0 {
            self.consumed_undo.push((
                id,
                self.consumed_at.get(&id).cloned(),
                self.maybe_consumed_at.get(&id).cloned(),
            ));
        }
        if let Some(site) = value.definitely {
            self.consumed_at.insert(id, site);
        } else {
            self.consumed_at.remove(&id);
        }
        if let Some(site) = value.maybe {
            self.maybe_consumed_at.insert(id, site);
        } else {
            self.maybe_consumed_at.remove(&id);
        }
    }
    fn merge_availability(
        &mut self,
        left: HashMap<LocalId, Availability>,
        right: HashMap<LocalId, Availability>,
        left_continues: bool,
        right_continues: bool,
        prior_locals: usize,
    ) {
        let mut ids: HashSet<_> = left.keys().chain(right.keys()).copied().collect();
        ids.retain(|id| id.0 < prior_locals);
        for id in ids {
            let before = self.availability(id);
            let a = left.get(&id).cloned().unwrap_or_else(|| before.clone());
            let b = right.get(&id).cloned().unwrap_or(before);
            let next = match (left_continues, right_continues) {
                (true, false) => a,
                (false, true) => b,
                (false, false) => continue,
                (true, true) => match (a.definitely, b.definitely) {
                    (Some(left), Some(right)) => Availability {
                        definitely: Some(merge_sites(left, right)),
                        maybe: None,
                    },
                    (left, right) => {
                        let left = left.or(a.maybe).unwrap_or_default();
                        let right = right.or(b.maybe).unwrap_or_default();
                        let sites = merge_sites(left, right);
                        Availability {
                            definitely: None,
                            maybe: (!sites.is_empty()).then_some(sites),
                        }
                    }
                },
            };
            self.set_availability(id, next);
        }
    }
    fn finish_block(&mut self, terminator: Terminator) -> BlockId {
        if self.emission == EmissionMode::Analyze {
            return BlockId(usize::MAX);
        }
        let mut terminator = terminator;
        let pending = std::mem::take(&mut self.pending_charges);
        match &mut terminator {
            Terminator::Unreachable { charges, .. }
            | Terminator::Return { charges, .. }
            | Terminator::Jump { charges, .. }
            | Terminator::BranchBool { charges, .. } => {
                charges.splice(0..0, pending);
            }
            Terminator::Yield { charges, .. } => {
                charges.splice(0..0, pending);
            }
        }
        if let Terminator::Return {
            value: Some(value),
            span,
            ..
        } = &terminator
        {
            let mut locals = std::mem::take(&mut self.deferred_return_locals);
            locals.extend(self.boundary_locals(0));
            self.plan_exit(
                ExitPlan {
                    surviving: HashSet::new(),
                    locals,
                    reserved: Some(*value),
                },
                span,
            );
            self.temp_live(*value, false);
        } else if let Terminator::BranchBool { condition, .. } = &terminator {
            self.temp_live(*condition, false);
        } else if let Terminator::Yield { value, .. } = &terminator {
            self.temp_live(*value, false);
        } else if let Terminator::Unreachable { span, .. } = &terminator {
            self.plan_exit(
                ExitPlan {
                    surviving: HashSet::new(),
                    locals: Vec::new(),
                    reserved: None,
                },
                span,
            );
        }
        super::count_cfg_work(1);
        let id = BlockId(self.blocks.len());
        let edge_count = match terminator {
            Terminator::BranchBool { .. } => 2,
            Terminator::Jump { .. } | Terminator::Yield { .. } => 1,
            Terminator::Return { .. } | Terminator::Unreachable { .. } => 0,
        };
        let mut edge = self.edge_state();
        if let Terminator::Yield { value, .. } = terminator {
            edge.suspended = true;
            edge.delivered = Some(value);
        }
        self.temp_blocks.push(TempBlockState {
            entry: self.temp_block_entry.clone(),
            edges: vec![edge; edge_count],
        });
        self.temp_block_entry = self.edge_state();
        self.blocks.push(BasicBlock {
            id,
            instructions: std::mem::take(&mut self.instructions),
            terminator,
        });
        id
    }
    fn conflict(&self, id: LocalId, primary: Span, requested: &str) -> Result<(), CompileError> {
        count_loan_work();
        let Some(active) = self.active_loans.get(&id).and_then(|v| v.first()) else {
            return Ok(());
        };
        if active.permission == LoanPermission::Read && matches!(requested, "read" | "borrow_read")
        {
            return Ok(());
        }
        let subcause = if active.permission == LoanPermission::Inout && requested == "borrow_inout"
        {
            "overlapping_mutable_borrows"
        } else {
            "active_borrow_conflict"
        };
        let CompileError::Rejected(mut errors) = self.ownership_error(
            "NX-E1105",
            Some(subcause),
            diagnostic::Operation::OwnershipAccess,
            primary,
            id,
            None,
        ) else {
            unreachable!()
        };
        let d = &mut errors[0];
        d.related_spans.push(diagnostic::RelatedSpan {
            role: diagnostic::RelatedRole::Origin,
            span: active.span.clone(),
        });
        d.related_spans
            .sort_by_key(|r| (r.span.start, r.span.end, r.role.as_str()));
        let entity = self.local_entity(id);
        let loan = diagnostic::Entity {
            kind: diagnostic::EntityKind::Loan,
            id: format!("loan:{}:0", active.loan.0),
        };
        let entity_value = |entity: &diagnostic::Entity| {
            let mut object = std::collections::BTreeMap::new();
            object.insert(
                "kind".into(),
                diagnostic::DataValue::String(entity.kind.as_str().into()),
            );
            object.insert(
                "id".into(),
                diagnostic::DataValue::String(entity.id.clone()),
            );
            diagnostic::DataValue::Object(object)
        };
        d.data.insert("origin".into(), entity_value(&entity));
        d.data.insert("binding".into(), entity_value(&entity));
        d.data.insert("loan".into(), entity_value(&loan));
        d.data.insert(
            "active_permission".into(),
            diagnostic::DataValue::String(
                if active.permission == LoanPermission::Read {
                    "read"
                } else {
                    "inout"
                }
                .into(),
            ),
        );
        d.data.insert(
            "requested_access".into(),
            diagnostic::DataValue::String(
                if requested == "borrow_read" {
                    "read"
                } else {
                    requested
                }
                .into(),
            ),
        );
        d.cause_chain.push(diagnostic::Cause {
            kind: diagnostic::CauseKind::Borrow,
            operation: if active.permission == LoanPermission::Read {
                diagnostic::CauseOperation::BorrowRead
            } else {
                diagnostic::CauseOperation::BorrowInout
            },
            span: Some(active.span.clone()),
            entity: Some(loan),
            evidence: diagnostic::Evidence::Verified,
        });
        if let Some(parent) = active.parent {
            d.cause_chain.push(diagnostic::Cause {
                kind: diagnostic::CauseKind::Reborrow,
                operation: diagnostic::CauseOperation::BorrowReborrow,
                span: Some(self.name_spans[parent.0].clone()),
                entity: Some(self.local_entity(parent)),
                evidence: diagnostic::Evidence::Verified,
            });
        }
        Err(CompileError::Rejected(errors))
    }
    fn mark_consumed(&mut self, id: LocalId, value: Option<(Span, bool)>) {
        if self.dead_depth != 0 {
            self.consumed_undo.push((
                id,
                self.consumed_at.get(&id).cloned(),
                self.maybe_consumed_at.get(&id).cloned(),
            ));
        }
        self.maybe_consumed_at.remove(&id);
        if let Some(value) = value {
            self.consumed_at.insert(id, vec![value]);
        } else {
            self.consumed_at.remove(&id);
        }
    }
    fn ownership_error(
        &self,
        code: &str,
        subcause: Option<&str>,
        operation: diagnostic::Operation,
        primary: Span,
        local: LocalId,
        consumption: Option<Vec<(Span, bool)>>,
    ) -> CompileError {
        let mut related = vec![diagnostic::RelatedSpan {
            role: diagnostic::RelatedRole::Declaration,
            span: self.name_spans[local.0].clone(),
        }];
        let mut cause_chain = Vec::new();
        if let Some(sites) = consumption {
            for (site, is_drop) in sites {
                related.push(diagnostic::RelatedSpan {
                    role: diagnostic::RelatedRole::Consumption,
                    span: site.clone(),
                });
                cause_chain.push(diagnostic::Cause {
                    kind: if is_drop {
                        diagnostic::CauseKind::Drop
                    } else {
                        diagnostic::CauseKind::Consume
                    },
                    operation: if is_drop {
                        diagnostic::CauseOperation::ValueDrop
                    } else {
                        diagnostic::CauseOperation::ValueConsume
                    },
                    span: Some(site),
                    entity: Some(self.local_entity(local)),
                    evidence: diagnostic::Evidence::Verified,
                });
            }
        }
        related.sort_by(|a, b| {
            (a.span.start, a.span.end, a.role.as_str()).cmp(&(
                b.span.start,
                b.span.end,
                b.role.as_str(),
            ))
        });
        CompileError::Rejected(vec![Box::new(Diagnostic {
            schema_version: diagnostic::SCHEMA_VERSION,
            diagnostic_namespace: diagnostic::Namespace::NexoProposedV1,
            profile_id: PROFILE_ID.into(),
            code: code.into(),
            subcause: subcause.map(str::to_owned),
            rule: subcause.map_or_else(|| code.to_owned(), |sub| format!("{code}/{sub}")),
            explanation_id: Some(
                subcause.map_or_else(|| code.to_owned(), |sub| format!("{code}/{sub}")),
            ),
            severity: diagnostic::Severity::Error,
            stage: diagnostic::Stage::Ownership,
            message: diagnostic::catalog_message(code)
                .unwrap_or("Acceso propietario inválido.")
                .into(),
            operation,
            primary_span: Some(primary),
            related_spans: related,
            cause_chain,
            feature_id: None,
            unexpected_token: None,
            expected_tokens: vec![],
            expected_tokens_kind: None,
            parser_context: None,
            recognized_prefix: None,
            repair: vec![],
            data: Default::default(),
        })])
    }
    fn local_entity(&self, id: LocalId) -> diagnostic::Entity {
        let parameter = self.locals[id.0].parameter_mode.is_some();
        diagnostic::Entity {
            kind: if parameter {
                diagnostic::EntityKind::Parameter
            } else {
                diagnostic::EntityKind::Binding
            },
            id: format!(
                "{}:{}",
                if parameter { "parameter" } else { "binding" },
                self.name_spans[id.0].start
            ),
        }
    }
    fn permission_error(&self, id: LocalId, primary: Span, requested: &str) -> CompileError {
        let entity = self.local_entity(id);
        let entity_value = |entity: &diagnostic::Entity| {
            let mut m = std::collections::BTreeMap::new();
            m.insert(
                "kind".into(),
                diagnostic::DataValue::String(entity.kind.as_str().into()),
            );
            m.insert(
                "id".into(),
                diagnostic::DataValue::String(entity.id.clone()),
            );
            diagnostic::DataValue::Object(m)
        };
        let CompileError::Rejected(mut errors) = self.ownership_error(
            "NX-E1105",
            Some("permission_escalation"),
            diagnostic::Operation::OwnershipAccess,
            primary,
            id,
            None,
        ) else {
            unreachable!()
        };
        let d = &mut errors[0];
        d.data.insert("origin".into(), entity_value(&entity));
        d.data.insert("parameter".into(), entity_value(&entity));
        d.data.insert(
            "active_permission".into(),
            diagnostic::DataValue::String(
                if self.locals[id.0].parameter_mode == Some(ParameterMode::Inout) {
                    "inout"
                } else {
                    "read"
                }
                .into(),
            ),
        );
        d.data.insert(
            "requested_access".into(),
            diagnostic::DataValue::String(requested.into()),
        );
        d.cause_chain.push(diagnostic::Cause {
            kind: diagnostic::CauseKind::Borrow,
            operation: if self.locals[id.0].parameter_mode == Some(ParameterMode::Inout) {
                diagnostic::CauseOperation::BorrowInout
            } else {
                diagnostic::CauseOperation::BorrowRead
            },
            span: Some(self.name_spans[id.0].clone()),
            entity: Some(entity),
            evidence: diagnostic::Evidence::Verified,
        });
        CompileError::Rejected(errors)
    }
    fn check_available(&self, id: LocalId, span: Span) -> Result<(), CompileError> {
        if self.defer_ownership && self.emission == EmissionMode::Live {
            return Ok(());
        }
        if let Some(site) = self.maybe_consumed_at.get(&id) {
            return Err(self.ownership_error(
                "NX-E0202",
                Some("maybe_consumed"),
                diagnostic::Operation::OwnershipUse,
                span,
                id,
                Some(site.clone()),
            ));
        }
        if let Some(site) = self.consumed_at.get(&id) {
            return Err(self.ownership_error(
                "NX-E0202",
                Some("definitely_consumed"),
                diagnostic::Operation::OwnershipUse,
                span,
                id,
                Some(site.clone()),
            ));
        }
        Ok(())
    }
    // An indexed place is acquired only after its index expressions have run.
    // Those expressions may consume or borrow the root, so every access path
    // (read, write, and call loan) crosses this same boundary.
    fn check_place_access(
        &self,
        id: LocalId,
        root: &Expr,
        place: &Expr,
        requested: &str,
        indexed: bool,
    ) -> Result<(), CompileError> {
        if indexed || requested != "write" {
            self.check_available(id, root.span().clone())?;
        }
        let access_span = if indexed
            && requested == "read"
            && self
                .active_loans
                .get(&id)
                .and_then(|loans| loans.first())
                .is_some_and(|loan| loan.compound_assignment)
        {
            root.span().clone()
        } else {
            parse::expression_primary_span(place)
        };
        self.conflict(id, access_span, requested)
    }
    fn copy_error(&self, id: LocalId, span: Span) -> CompileError {
        self.ownership_error(
            "NX-E0201",
            None,
            diagnostic::Operation::ValueCopy,
            span,
            id,
            None,
        )
    }
    fn name_span(&self, stmt: &Statement, name: &str, before: u64) -> Result<Span, CompileError> {
        let start = statement_span(stmt).start;
        let pos = self.tokens.partition_point(|t| t.span.start < start);
        self.tokens[pos..]
            .iter()
            .take_while(|t| t.span.start < before)
            .find(|t| t.kind == lex::Kind::Name && t.text == name)
            .map(|t| t.span.clone())
            .ok_or(CompileError::InvalidIr("declaration token"))
    }
    fn immutable_write(&self, root: &Expr, id: LocalId) -> CompileError {
        let mut d = Diagnostic {
            schema_version: diagnostic::SCHEMA_VERSION,
            diagnostic_namespace: diagnostic::Namespace::NexoProposedV1,
            profile_id: PROFILE_ID.into(),
            code: "NX-E0105".into(),
            subcause: None,
            rule: "NX-E0105".into(),
            explanation_id: Some("NX-E0105".into()),
            severity: diagnostic::Severity::Error,
            stage: diagnostic::Stage::Ownership,
            message: "La raíz no admite escritura.".into(),
            operation: diagnostic::Operation::PlaceWrite,
            primary_span: Some(parse::expression_primary_span(root)),
            related_spans: vec![diagnostic::RelatedSpan {
                role: diagnostic::RelatedRole::Declaration,
                span: self.name_spans[id.0].clone(),
            }],
            cause_chain: vec![],
            feature_id: None,
            unexpected_token: None,
            expected_tokens: vec![],
            expected_tokens_kind: None,
            parser_context: None,
            recognized_prefix: None,
            repair: vec![],
            data: Default::default(),
        };
        d.data.clear();
        CompileError::Rejected(vec![Box::new(d)])
    }
    fn emit(
        &mut self,
        op: Op,
        ty: Ty,
        span: Span,
        access: Access,
        effect_io: bool,
    ) -> Option<TempId> {
        if self.emission == EmissionMode::Analyze {
            return None;
        }
        let retained = self.output_retained(&op);
        let loan = if let Op::BeginLoan { root, loan, .. } = &op {
            Some((*root, *loan))
        } else {
            None
        };
        self.consume_op_temps(&op);
        self.track_local_op(&op);
        count_loan_work();
        super::count_cfg_work(1);
        let id = TempId(self.next_temp);
        let origin = OriginId(self.instructions.len());
        self.next_temp += 1;
        let administrative = matches!(
            op,
            Op::EndLoan(_) | Op::DropLocal(_) | Op::DropValue(_) | Op::DropBinding(_)
        );
        let charges = if administrative {
            Vec::new()
        } else {
            std::mem::take(&mut self.pending_charges)
        };
        self.instructions.push(Instruction {
            op,
            output: Some(id),
            ty: Some(ty),
            origin,
            span: span.clone(),
            scope: self.scope,
            access,
            effect_io,
            charges,
        });
        self.temp_live(id, true);
        if let Some(loan) = loan {
            self.temp_loans.insert(id, loan);
        }
        if !retained.is_empty() {
            self.retained_temps.insert(id, retained);
        }
        Some(id)
    }
    fn statement_op(&mut self, op: Op, span: Span, access: Access, effect_io: bool) {
        if self.emission == EmissionMode::Analyze {
            return;
        }
        self.statement_retained(&op);
        if let Op::DropValue(temp) = &op
            && let Some((root, loan)) = self.temp_loans.get(temp)
            && let Some(active) = self.active_loans.get_mut(root)
        {
            active.retain(|held| held.loan != *loan);
        }
        self.consume_op_temps(&op);
        self.track_local_op(&op);
        count_loan_work();
        super::count_cfg_work(1);
        let id = self.instructions.len();
        let administrative = matches!(
            op,
            Op::EndLoan(_) | Op::DropLocal(_) | Op::DropValue(_) | Op::DropBinding(_)
        );
        let charges = if administrative {
            Vec::new()
        } else {
            std::mem::take(&mut self.pending_charges)
        };
        self.instructions.push(Instruction {
            op,
            output: None,
            ty: None,
            origin: OriginId(id),
            span: span.clone(),
            scope: self.scope,
            access,
            effect_io,
            charges,
        });
    }
    fn get(&self, name: &str) -> Result<LocalId, CompileError> {
        let id = self
            .visible
            .get(name)
            .and_then(|v| v.last())
            .copied()
            .ok_or(CompileError::InvalidIr("unresolved local"))?;
        if self.poisoned.contains(&id) {
            Err(CompileError::Dependent)
        } else {
            Ok(id)
        }
    }
    fn release_call_loans(&mut self, acquired: &[(LocalId, LoanId, Span)]) {
        for (root, _, _) in acquired.iter().rev() {
            if let Some(loans) = self.active_loans.get_mut(root) {
                loans.pop();
            }
        }
    }

    fn end_call_loans(&mut self, acquired: Vec<(LocalId, LoanId, Span)>) {
        for (_, loan, origin) in acquired.into_iter().rev() {
            self.statement_op(Op::EndLoan(loan), origin, Access::Own, false);
            if self.emission == EmissionMode::Live
                && let Some(last) = self.instructions.last_mut()
            {
                last.charges.clear();
            }
        }
    }

    // A read argument can be an owned temporary. The operation borrows it,
    // then the caller destroys that value immediately after the call.
    fn drop_owned_read_arguments(
        &mut self,
        prepared: &[(usize, TempId)],
        params: &[(String, ParameterMode, Ty)],
    ) {
        for &(slot, temp) in prepared {
            if params
                .get(slot)
                .is_none_or(|(_, mode, _)| *mode != ParameterMode::Read)
            {
                continue;
            }
            let Some(instruction) = self
                .instructions
                .iter()
                .find(|instruction| instruction.output == Some(temp))
            else {
                continue;
            };
            if matches!(
                instruction.op,
                Op::BeginLoan { .. } | Op::BorrowValue(_) | Op::BorrowIndex { .. }
            ) {
                continue;
            }
            if !instruction
                .ty
                .as_ref()
                .is_some_and(Ty::has_resource_destructor)
            {
                continue;
            }
            let span = instruction.span.clone();
            self.statement_op(Op::DropValue(temp), span, Access::Own, false);
        }
    }
    fn finish_scoped_block(
        &mut self,
        block: &parse::Block,
        surviving: HashSet<TempId>,
        saved: ScopeId,
    ) -> Result<(), CompileError> {
        let dropping = self
            .scopes
            .last()
            .ok_or(CompileError::InvalidIr("scope stack"))?
            .clone();
        let locals = dropping
            .iter()
            .rev()
            .map(|name| {
                self.visible
                    .get(name)
                    .and_then(|v| v.last())
                    .copied()
                    .map(|id| (id, block.close_span.clone()))
                    .ok_or(CompileError::InvalidIr("scope name"))
            })
            .collect::<Result<Vec<_>, _>>()?;
        if self
            .terminated
            .is_some_and(|term| matches!(term, Termination::Return(_)))
            && self.emission == EmissionMode::Live
        {
            self.deferred_return_locals.extend(locals);
        } else if self.emission == EmissionMode::Live {
            self.plan_exit(
                ExitPlan {
                    surviving,
                    locals,
                    reserved: None,
                },
                &block.close_span,
            );
        }
        self.leave_scope(saved)?;
        Ok(())
    }
    // Scope restoration must run even when lowering an inner branch reports a
    // source diagnostic. The caller may continue in a hypothetical dead block.
    fn leave_scope(&mut self, saved: ScopeId) -> Result<(), CompileError> {
        for name in self
            .scopes
            .pop()
            .ok_or(CompileError::InvalidIr("scope stack"))?
        {
            self.visible
                .get_mut(&name)
                .ok_or(CompileError::InvalidIr("scope name"))?
                .pop();
        }
        self.scope = saved;
        Ok(())
    }
    fn finish_scoped_value_block(
        &mut self,
        block: &parse::Block,
        surviving: HashSet<TempId>,
        saved: ScopeId,
        final_value: Option<Flow>,
    ) -> Result<Option<Flow>, CompileError> {
        if let Some(Flow::Value {
            temp: Some(temp), ..
        }) = &final_value
            && let Some(escaping) = self.retained_temps.get(temp)
            && let Some(loan) = escaping
                .iter()
                .find(|loan| self.locals[loan.root.0].scope == self.scope)
        {
            let primary = match block.statements.last() {
                Some(Statement::Value {
                    value: Some(value), ..
                }) => parse::expression_primary_span(value),
                _ => block.span.clone(),
            };
            let mut error = crate::check::names::diagnostic(
                "NX-E1104",
                None,
                diagnostic::Stage::Ownership,
                diagnostic::Operation::OwnershipCheck,
                primary,
            );
            error.related_spans.extend([
                diagnostic::RelatedSpan {
                    role: diagnostic::RelatedRole::Declaration,
                    span: loan.source.clone(),
                },
                diagnostic::RelatedSpan {
                    role: diagnostic::RelatedRole::Origin,
                    span: loan.active.span.clone(),
                },
                diagnostic::RelatedSpan {
                    role: diagnostic::RelatedRole::Exit,
                    span: block.close_span.clone(),
                },
            ]);
            error.cause_chain.push(diagnostic::Cause {
                kind: diagnostic::CauseKind::Borrow,
                operation: if loan.active.permission == LoanPermission::Read {
                    diagnostic::CauseOperation::BorrowRead
                } else {
                    diagnostic::CauseOperation::BorrowInout
                },
                span: Some(loan.active.span.clone()),
                entity: Some(diagnostic::Entity {
                    kind: diagnostic::EntityKind::Loan,
                    id: format!("loan:{}:0", loan.active.loan.0),
                }),
                evidence: diagnostic::Evidence::Verified,
            });
            self.leave_scope(saved)?;
            return Err(CompileError::Rejected(vec![error]));
        }
        let dropping = self
            .scopes
            .last()
            .ok_or(CompileError::InvalidIr("scope stack"))?
            .clone();
        let locals = dropping
            .iter()
            .rev()
            .map(|name| {
                self.visible
                    .get(name)
                    .and_then(|v| v.last())
                    .copied()
                    .map(|id| (id, block.close_span.clone()))
                    .ok_or(CompileError::InvalidIr("scope name"))
            })
            .collect::<Result<Vec<_>, _>>()?;
        if self
            .terminated
            .is_some_and(|term| matches!(term, Termination::Return(_)))
            && self.emission == EmissionMode::Live
        {
            self.deferred_return_locals.extend(locals);
        } else if self.emission == EmissionMode::Live {
            self.plan_exit(
                ExitPlan {
                    surviving,
                    locals,
                    reserved: final_value.as_ref().and_then(Flow::temp),
                },
                &block.close_span,
            );
        }
        self.leave_scope(saved)?;
        Ok(final_value)
    }
    fn block(
        &mut self,
        statements: &[Statement],
        depth: usize,
        diagnostics: &mut Vec<Diagnostic>,
    ) -> Result<(), CompileError> {
        self.block_with_tail(statements, depth, diagnostics, |_, _| Ok(()))
    }
    fn block_with_tail<T>(
        &mut self,
        statements: &[Statement],
        depth: usize,
        diagnostics: &mut Vec<Diagnostic>,
        tail: impl FnOnce(&mut Self, bool) -> Result<T, CompileError>,
    ) -> Result<T, CompileError> {
        self.run_lower_tasks(
            LowerRoot::Block(statements, depth),
            diagnostics,
            |lower, done, _| tail(lower, done),
        )
    }
    fn run_lower_tasks<'a, T>(
        &mut self,
        root: LowerRoot<'a>,
        diagnostics: &mut Vec<Diagnostic>,
        tail: impl FnOnce(&mut Self, bool, Option<Flow>) -> Result<T, CompileError>,
    ) -> Result<T, CompileError> {
        struct Saved {
            instructions: Vec<Instruction>,
            pending_charges: Vec<WorkCharge>,
            blocks: usize,
            locals: usize,
            scope_len: usize,
            next_scope: usize,
            next_temp: usize,
            terminated: Option<Termination>,
            emission: EmissionMode,
            undo: usize,
            temp_undo: usize,
            temp_block_entry: EdgeState,
        }
        struct Pending {
            prior_dead_depth: usize,
            prior_terminated: Option<Termination>,
            prior_emission: EmissionMode,
            temp_checkpoint: usize,
            before_statement_errors: usize,
        }
        struct Scoped<'a> {
            block: &'a parse::Block,
            surviving: HashSet<TempId>,
            saved: ScopeId,
        }
        struct ValueTail<'a> {
            value: &'a Expr,
            marker: Option<&'a Span>,
            expected: Option<Ty>,
            saved_termination: Option<Termination>,
            saved_emission: EmissionMode,
            started: bool,
            result: Option<Result<Flow, CompileError>>,
        }
        struct ScopedValue<'a> {
            block: &'a parse::Block,
            if_span: &'a Span,
            surviving: HashSet<TempId>,
            saved: ScopeId,
        }
        struct Frame<'a> {
            statements: &'a [Statement],
            depth: usize,
            index: usize,
            unreachable: Option<Saved>,
            scoped: Option<Scoped<'a>>,
            pending: Option<Pending>,
            value_tail: Option<ValueTail<'a>>,
        }
        enum CallKind {
            User(String),
            Intrinsic(Intrinsic),
            GeneratorNext {
                item: Ty,
            },
            Closure {
                site: u64,
                receiver: Flow,
                receiver_mode: ParameterMode,
                capture_count: usize,
                signature: Vec<(String, ParameterMode, Ty)>,
                receiver_loan: Option<(LocalId, LoanId, Span)>,
            },
        }
        struct UserCallState<'a> {
            args: &'a [parse::Argument],
            kind: CallKind,
            span: &'a Span,
            depth: usize,
            inferred: Ty,
            params: Vec<(String, ParameterMode, Ty)>,
            check_types: bool,
            retain_place_loans: bool,
            used: Vec<bool>,
            slot_by_name: HashMap<String, usize>,
            next_positional: usize,
            index: usize,
            prepared: Vec<(usize, TempId)>,
            acquired: Vec<(LocalId, LoanId, Span)>,
            argument_errors: Vec<Diagnostic>,
            dependent: bool,
            diverged: bool,
        }
        struct BinaryState<'a> {
            op: &'a str,
            left: &'a Expr,
            right: &'a Expr,
            span: &'a Span,
            inferred: Ty,
            depth: usize,
            left_flow: Option<Flow>,
            left_root: Option<LocalId>,
        }
        struct LogicalState<'a> {
            op: &'a str,
            right: &'a Expr,
            span: &'a Span,
            depth: usize,
            inferred: Ty,
            branch_mode: EmissionMode,
            skip_end: Option<BlockId>,
            skip_state: HashMap<LocalId, Availability>,
            checkpoint: usize,
            temp_checkpoint: usize,
            prior_locals: usize,
            slot: Option<LocalId>,
            left_diverges: bool,
        }
        enum IndexKind<'a> {
            Place {
                root: &'a Expr,
                local: LocalId,
                selectors: Vec<&'a Expr>,
                ids: Vec<TempId>,
                diverged: bool,
            },
            Unresolved,
            Value {
                base: Option<Flow>,
            },
        }
        struct IndexState<'a> {
            expr: &'a Expr,
            index: &'a Expr,
            selector: &'a Span,
            span: &'a Span,
            inferred: Ty,
            depth: usize,
            borrow: bool,
            kind: IndexKind<'a>,
        }
        enum StatementExprState<'a> {
            Let {
                statement: &'a Statement,
                mutable: bool,
                name: &'a str,
                binding_span: Span,
                expected: Ty,
            },
            Expr(&'a Statement),
            Return(&'a Statement),
        }
        struct ArgumentPlaceState<'a> {
            arg: &'a parse::Argument,
            place: &'a Expr,
            root: &'a Expr,
            local: LocalId,
            mode: ParameterMode,
            ty: Ty,
            depth: usize,
            indexes: Vec<&'a Expr>,
            ids: Vec<TempId>,
        }
        struct ArrayState<'a> {
            items: &'a [Expr],
            span: &'a Span,
            inferred: Ty,
            depth: usize,
            next: usize,
            ids: Vec<TempId>,
            diverged: bool,
            item_errors: Vec<Diagnostic>,
            dependent: bool,
        }
        struct AssignState<'a> {
            statement: &'a Statement,
            place: &'a Expr,
            root: &'a Expr,
            value: &'a Expr,
            op: parse::AssignOp,
            local: LocalId,
            selectors: Vec<&'a Expr>,
            selector_spans: Vec<Span>,
            next: usize,
            ids: Vec<TempId>,
            target_ty: Ty,
            base_unresolved: bool,
            diverged: bool,
            compound: Option<(LoanId, TempId, TempId)>,
        }
        struct ClosureCallState<'a> {
            args: &'a [parse::Argument],
            span: &'a Span,
            depth: usize,
            inferred: Ty,
            site: u64,
            receiver_mode: ParameterMode,
        }
        struct WhileConditionState<'a> {
            condition: &'a Expr,
            body: &'a parse::Block,
            depth: usize,
            entry_emission: EmissionMode,
            prior_locals: usize,
            preheader: BlockId,
            condition_header: BlockId,
            checkpoint: usize,
            temp_checkpoint: usize,
            surviving_temps: HashSet<TempId>,
        }
        struct WhileSecondConditionState<'a> {
            body: &'a parse::Block,
            depth: usize,
            checkpoint: usize,
            temp_checkpoint: usize,
            locals: usize,
            scopes: usize,
            before_errors: usize,
            affected: HashSet<u64>,
            condition_diverges: bool,
        }
        struct MatchIndexState<'a> {
            selection: &'a parse::Match,
            depth: usize,
            expected: Option<Ty>,
            prelude: lower_match::MatchSelectionPrelude,
            selected: Flow,
            selectors: Vec<&'a Expr>,
            ids: Vec<TempId>,
        }
        enum Task<'a> {
            Block(Box<Frame<'a>>),
            StatementEnter {
                statement: &'a Statement,
                depth: usize,
            },
            StatementExprEnter(&'a Statement),
            StatementExprAfter(Box<StatementExprState<'a>>),
            AssignEnter(&'a Statement),
            AssignNextSelector(Box<AssignState<'a>>),
            AssignAfterSelector(Box<AssignState<'a>>),
            AssignAfterValue(Box<AssignState<'a>>),
            ForEnter {
                statement: &'a Statement,
                depth: usize,
            },
            ForAfterIterable {
                statement: &'a Statement,
                depth: usize,
                surviving_temps: HashSet<TempId>,
            },
            ForRangeAfterLower {
                statement: &'a Statement,
                upper: &'a Expr,
                depth: usize,
                inclusive: bool,
                surviving_temps: HashSet<TempId>,
            },
            ForRangeAfterUpper {
                statement: &'a Statement,
                depth: usize,
                inclusive: bool,
                lower: Flow,
                surviving_temps: HashSet<TempId>,
            },
            ForAfterBody {
                state: Box<ForState>,
                body: &'a parse::Block,
            },
            YieldEnter {
                statement: &'a Statement,
                depth: usize,
            },
            YieldAfterValue {
                token_span: &'a Span,
                span: &'a Span,
            },
            ArgumentEnter {
                arg: &'a parse::Argument,
                mode: ParameterMode,
                expected: Ty,
                depth: usize,
                retain_place_loan: bool,
            },
            ArgumentPlaceNext(Box<ArgumentPlaceState<'a>>),
            ArgumentPlaceAfterIndex(Box<ArgumentPlaceState<'a>>),
            ValueBlockEnter {
                block: &'a parse::Block,
                expected: Option<Ty>,
                if_span: &'a Span,
                depth: usize,
            },
            ValueBlockAfter(ScopedValue<'a>),
            Expr {
                value: &'a Expr,
                expected: Option<Ty>,
                depth: usize,
                borrow: bool,
            },
            BinaryLeft(Box<BinaryState<'a>>),
            BinaryRight(Box<BinaryState<'a>>),
            LogicalLeft(Box<LogicalState<'a>>),
            LogicalRight(Box<LogicalState<'a>>),
            ArrayNext(Box<ArrayState<'a>>),
            ArrayAfterItem(Box<ArrayState<'a>>),
            IndexNext(Box<IndexState<'a>>),
            IndexAfterSelector(Box<IndexState<'a>>),
            IndexAfterBase(Box<IndexState<'a>>),
            IndexAfterValueSelector(Box<IndexState<'a>>),
            IfExprEnter {
                condition: &'a Expr,
                then_block: &'a parse::Block,
                otherwise: Option<&'a parse::Else>,
                span: &'a Span,
                depth: usize,
                expected: Option<Ty>,
            },
            IfExprAfterCondition {
                then_block: &'a parse::Block,
                otherwise: Option<&'a parse::Else>,
                span: &'a Span,
                depth: usize,
                expected: Option<Ty>,
            },
            IfExprAfterThen(Box<IfExprStarted<'a>>, Vec<Diagnostic>),
            IfExprElse(Box<IfExprState<'a>>),
            IfExprAfterElse(Box<IfExprState<'a>>, Vec<Diagnostic>),
            MatchEnter {
                selection: &'a parse::Match,
                depth: usize,
                expected: Option<Ty>,
            },
            MatchExprFinish(Vec<Diagnostic>),
            MatchAfterScrutinee {
                selection: &'a parse::Match,
                depth: usize,
                expected: Option<Ty>,
                prelude: lower_match::MatchSelectionPrelude,
            },
            MatchIndexNext(Box<MatchIndexState<'a>>),
            MatchIndexAfterSelector(Box<MatchIndexState<'a>>),
            UserCallEnter {
                args: &'a [parse::Argument],
                name: String,
                span: &'a Span,
                depth: usize,
                inferred: Ty,
            },
            IntrinsicCallEnter {
                args: &'a [parse::Argument],
                intrinsic: Intrinsic,
                span: &'a Span,
                depth: usize,
                inferred: Ty,
            },
            GeneratorNextEnter {
                args: &'a [parse::Argument],
                span: &'a Span,
                depth: usize,
                inferred: Ty,
            },
            OtherCallEnter {
                target: &'a Expr,
                args: &'a [parse::Argument],
                span: &'a Span,
                depth: usize,
                inferred: Ty,
            },
            ClosureAfterReceiver(Box<ClosureCallState<'a>>),
            UnaryReadEnter {
                value: &'a Expr,
                span: &'a Span,
                depth: usize,
                inferred: Ty,
            },
            UnaryReadAfter {
                span: &'a Span,
                inferred: Ty,
            },
            OperatorUnaryAfter {
                span: &'a Span,
                inferred: Ty,
                op: Unary,
            },
            UserCallNext(Box<UserCallState<'a>>),
            UserCallAfterArgument {
                state: Box<UserCallState<'a>>,
                slot: usize,
                loan_root: Option<LocalId>,
            },
            VariantEnter {
                value: &'a Expr,
                tag: crate::union::VariantTag,
                span: &'a Span,
                depth: usize,
                inferred: Ty,
            },
            VariantAfter {
                tag: crate::union::VariantTag,
                span: &'a Span,
                inferred: Ty,
            },
            MatchNext(Box<lower_match::MatchInnerState<'a>>),
            MatchGuardAfter {
                state: Box<lower_match::MatchInnerState<'a>>,
                arm: &'a parse::Arm,
                saved: ScopeId,
            },
            MatchArmReady {
                state: Box<lower_match::MatchInnerState<'a>>,
                arm: lower_match::MatchArmState<'a>,
            },
            MatchAfterArm {
                state: Box<lower_match::MatchInnerState<'a>>,
                arm: lower_match::MatchArmState<'a>,
            },
            MatchRestore(Box<lower_match::MatchSnapshot>),
            WhileEnter {
                condition: &'a Expr,
                body: &'a parse::Block,
                span: &'a Span,
                depth: usize,
            },
            WhileAfterCondition(Box<WhileConditionState<'a>>),
            WhileAfterSecondCondition(Box<WhileSecondConditionState<'a>>),
            WhileAfterBody {
                state: Box<WhileState<'a>>,
            },
            WhileAfterSecond {
                checkpoint: usize,
                temp_checkpoint: usize,
                locals: usize,
                scopes: usize,
                before_errors: usize,
                affected: HashSet<u64>,
                body_span: Span,
                condition_diverges: bool,
            },
            IfEnter {
                condition: &'a Expr,
                then_block: &'a parse::Block,
                otherwise: Option<StatementElse<'a>>,
                span: &'a Span,
                depth: usize,
            },
            IfAfterCondition {
                then_block: &'a parse::Block,
                otherwise: Option<StatementElse<'a>>,
                span: &'a Span,
                depth: usize,
            },
            IfAfterThen {
                started: IfBeginState,
                then_block: &'a parse::Block,
                otherwise: Option<StatementElse<'a>>,
                depth: usize,
            },
            IfElse {
                state: IfState,
                otherwise: Option<StatementElse<'a>>,
                depth: usize,
            },
            IfAfterElse {
                state: IfState,
                else_span: Option<Span>,
            },
        }
        fn schedule_for_body<'a>(
            lower: &mut Lower<'_>,
            tasks: &mut Vec<Task<'a>>,
            statement: &'a Statement,
            depth: usize,
            state: ForState,
        ) -> Result<(), CompileError> {
            let Statement::For { body, .. } = statement else {
                return Err(CompileError::InvalidIr("for statement"));
            };
            let surviving = lower.live_temps.clone();
            let saved = lower.scope;
            lower.scope = ScopeId(lower.next_scope);
            lower.next_scope += 1;
            lower.scope_parents.push(Some(saved));
            lower.scopes.push(Vec::new());
            tasks.push(Task::ForAfterBody {
                state: Box::new(state),
                body,
            });
            tasks.push(Task::Block(Box::new(Frame {
                statements: &body.statements,
                depth: depth + 1,
                index: 0,
                unreachable: None,
                scoped: Some(Scoped {
                    block: body,
                    surviving,
                    saved,
                }),
                pending: None,
                value_tail: None,
            })));
            Ok(())
        }
        enum WhileState<'a> {
            Analyze {
                condition: &'a Expr,
                body: &'a parse::Block,
                depth: usize,
                checkpoint: usize,
                temp_checkpoint: usize,
                prior_locals: usize,
                condition_diverges: bool,
                before_errors: usize,
            },
            Live {
                entry_emission: EmissionMode,
                condition_header: BlockId,
                checkpoint: usize,
                temp_checkpoint: usize,
                prior_locals: usize,
                condition_state: HashMap<LocalId, Availability>,
                branch: BlockId,
                literal_true: bool,
                preheader: BlockId,
                body: &'a parse::Block,
            },
        }
        fn finish_statement(
            lower: &mut Lower<'_>,
            pending: Pending,
            result: Result<(), CompileError>,
            diagnostics: &mut Vec<Diagnostic>,
        ) -> Result<(), CompileError> {
            if result.is_err() {
                lower.dead_depth = pending.prior_dead_depth;
                lower.terminated = pending.prior_terminated;
                lower.emission = pending.prior_emission;
                lower.restore_temps(pending.temp_checkpoint);
            }
            match result {
                Err(CompileError::Rejected(errors)) => {
                    diagnostics.extend(errors.into_iter().map(|d| *d));
                }
                Err(CompileError::Dependent) => {}
                Err(other) => return Err(other),
                Ok(()) => {}
            }
            if diagnostics.len() > pending.before_statement_errors {
                lower.defer_ownership = false;
            }
            Ok(())
        }
        fn finish_user_argument(
            state: &mut UserCallState<'_>,
            slot: usize,
            arg: &parse::Argument,
            result: Result<(Flow, Option<LocalId>), CompileError>,
        ) -> Result<(), CompileError> {
            let (flow, loan_root) = match result {
                Ok(value) => value,
                Err(CompileError::Rejected(errors)) => {
                    state.argument_errors.extend(errors.into_iter().map(|d| *d));
                    return Ok(());
                }
                Err(CompileError::Dependent) => {
                    state.dependent = true;
                    return Ok(());
                }
                Err(other) => return Err(other),
            };
            if flow.diverged() {
                state.diverged = true;
                return Ok(());
            }
            if let Some(root) = loan_root {
                state
                    .acquired
                    .push((root, LoanId(arg.span.start as usize), arg.span.clone()));
            }
            let (_, _, expected) = &state.params[slot];
            if state.check_types
                && flow
                    .ty()
                    .is_some_and(|actual| *actual != *expected && *actual != Ty::Never)
            {
                return Err(CompileError::InvalidIr("call argument type passed checker"));
            }
            if let Some(id) = flow.temp() {
                state.prepared.push((slot, id));
            }
            Ok(())
        }
        fn declare_binding(
            lower: &mut Lower<'_>,
            name: &str,
            mutable: bool,
            binding_span: Span,
            expected: Ty,
        ) -> Result<LocalId, CompileError> {
            let id = LocalId(lower.locals.len());
            lower.name_spans.push(binding_span.clone());
            lower.locals.push(Local {
                id,
                name: name.to_owned(),
                name_span: binding_span,
                ty: expected,
                mutable,
                scope: lower.scope,
                parameter_mode: None,
            });
            lower.visible.entry(name.to_owned()).or_default().push(id);
            lower
                .scopes
                .last_mut()
                .ok_or(CompileError::InvalidIr("scope stack"))?
                .push(name.to_owned());
            Ok(id)
        }
        fn closure_call_state<'a>(
            lower: &mut Lower<'_>,
            state: ClosureCallState<'a>,
            receiver: Flow,
            receiver_loan: Option<(LocalId, LoanId, Span)>,
        ) -> Result<UserCallState<'a>, CompileError> {
            let info = lower
                .inferred_bindings
                .closures
                .get(&state.site)
                .ok_or(CompileError::InvalidIr("closure call signature"))?;
            let params = info
                .params
                .iter()
                .map(|(name, mode, ty)| {
                    Ok((
                        name.clone(),
                        match mode {
                            Mode::Read => ParameterMode::Read,
                            Mode::Inout => ParameterMode::Inout,
                            Mode::Consume => ParameterMode::Consume,
                        },
                        lower_type(ty)?,
                    ))
                })
                .collect::<Result<Vec<_>, CompileError>>()?;
            let capture_count = info.captures.len();
            let signature = lower
                .user_functions
                .get(&format!("$closure:{}", state.site))
                .ok_or(CompileError::InvalidIr("closure body signature"))?
                .0
                .clone();
            let slot_by_name = params
                .iter()
                .enumerate()
                .map(|(index, (label, _, _))| (label.clone(), index))
                .collect();
            Ok(UserCallState {
                args: state.args,
                kind: CallKind::Closure {
                    site: state.site,
                    receiver,
                    receiver_mode: state.receiver_mode,
                    capture_count,
                    signature,
                    receiver_loan,
                },
                span: state.span,
                depth: state.depth,
                inferred: state.inferred,
                used: vec![false; params.len()],
                slot_by_name,
                params,
                check_types: true,
                retain_place_loans: true,
                next_positional: 0,
                index: 0,
                prepared: Vec::with_capacity(state.args.len()),
                acquired: Vec::new(),
                argument_errors: Vec::new(),
                dependent: false,
                diverged: false,
            })
        }
        fn named_closure_receiver(
            lower: &mut Lower<'_>,
            target: &Expr,
            receiver_mode: ParameterMode,
            receiver_span: &Span,
        ) -> Result<(Flow, Option<ReceiverLoan>), CompileError> {
            let name = place_name(target).ok_or(CompileError::InvalidIr("closure receiver"))?;
            let local = lower.get(name)?;
            let primary = parse::expression_primary_span(target);
            lower.check_available(local, primary.clone())?;
            if receiver_mode == ParameterMode::Consume
                && lower.locals[local.0]
                    .parameter_mode
                    .is_some_and(|mode| mode != ParameterMode::Consume)
            {
                return Err(lower.permission_error(local, primary, "consume"));
            }
            if receiver_mode == ParameterMode::Inout
                && lower.locals[local.0].parameter_mode == Some(ParameterMode::Read)
            {
                return Err(lower.permission_error(local, primary, "borrow_inout"));
            }
            if receiver_mode == ParameterMode::Inout && !lower.locals[local.0].mutable {
                return Err(lower.immutable_write(target, local));
            }
            if receiver_mode == ParameterMode::Consume {
                lower.conflict(local, primary, "consume")?;
                lower.mark_consumed(local, Some((receiver_span.clone(), false)));
                let flow = lower.emitted(
                    Op::TransferValue(local),
                    lower.locals[local.0].ty.clone(),
                    receiver_span.clone(),
                    Access::Own,
                    false,
                );
                return Ok((flow, None));
            }
            let permission = if receiver_mode == ParameterMode::Inout {
                LoanPermission::Inout
            } else {
                LoanPermission::Read
            };
            lower.conflict(
                local,
                primary,
                if permission == LoanPermission::Inout {
                    "borrow_inout"
                } else {
                    "read"
                },
            )?;
            let loan = LoanId(target.span().start as usize);
            let parent = (lower.locals[local.0].parameter_mode == Some(ParameterMode::Inout))
                .then_some(local);
            let active = ActiveLoan {
                permission,
                span: target.span().clone(),
                loan,
                parent,
                compound_assignment: false,
            };
            let flow = lower.emitted(
                Op::BeginLoan {
                    root: local,
                    indexes: Vec::new(),
                    site: IndexSite {
                        context: target.span().clone(),
                        selectors: Vec::new(),
                    },
                    loan,
                    origin_span: target.span().clone(),
                    permission,
                    parent,
                },
                lower.locals[local.0].ty.clone(),
                target.span().clone(),
                Access::Read,
                false,
            );
            lower.active_loans.entry(local).or_default().push(active);
            Ok((flow, Some((local, loan, target.span().clone()))))
        }
        let mut tasks = vec![match root {
            LowerRoot::Block(statements, depth) => Task::Block(Box::new(Frame {
                statements,
                depth,
                index: 0,
                unreachable: None,
                scoped: None,
                pending: None,
                value_tail: None,
            })),
            #[cfg(test)]
            LowerRoot::Statement(statement, depth) => Task::StatementEnter { statement, depth },
            #[cfg(test)]
            LowerRoot::Expr(value, depth, expected) => Task::Expr {
                value,
                expected,
                depth,
                borrow: false,
            },
            #[cfg(test)]
            LowerRoot::Argument(arg, mode, expected, depth, retain_place_loan) => {
                Task::ArgumentEnter {
                    arg,
                    mode,
                    expected,
                    depth,
                    retain_place_loan,
                }
            }
        }];
        let mut completed: Option<Result<(), CompileError>> = None;
        let mut completed_value: Option<Result<Option<Flow>, CompileError>> = None;
        let mut tail = Some(tail);
        loop {
            if let Some(result) = completed_value.take() {
                if tasks.is_empty() {
                    return tail
                        .take()
                        .ok_or(CompileError::InvalidIr("lower value tail"))?(
                        self,
                        self.terminated.is_some(),
                        result?,
                    );
                }
                match tasks.last_mut() {
                    Some(Task::YieldAfterValue { .. }) => {
                        let Some(Task::YieldAfterValue { token_span, span }) = tasks.pop() else {
                            return Err(CompileError::InvalidIr("yield continuation"));
                        };
                        completed = Some(result.and_then(|value| {
                            self.finish_yield(
                                value.ok_or(CompileError::InvalidIr("yield value flow"))?,
                                token_span,
                                span,
                            )
                        }));
                    }
                    Some(Task::ForAfterIterable { .. }) => {
                        let Some(Task::ForAfterIterable {
                            statement,
                            depth,
                            surviving_temps,
                        }) = tasks.pop()
                        else {
                            return Err(CompileError::InvalidIr("for iterable continuation"));
                        };
                        let started = result.and_then(|value| {
                            self.begin_for_after_iterable(
                                statement,
                                value.ok_or(CompileError::InvalidIr("for iterable flow"))?,
                                surviving_temps,
                            )
                        });
                        match started {
                            Err(error) => completed = Some(Err(error)),
                            Ok(state) => {
                                schedule_for_body(self, &mut tasks, statement, depth, state)?
                            }
                        }
                    }
                    Some(Task::ForRangeAfterLower { .. }) => {
                        let Some(Task::ForRangeAfterLower {
                            statement,
                            upper,
                            depth,
                            inclusive,
                            surviving_temps,
                        }) = tasks.pop()
                        else {
                            return Err(CompileError::InvalidIr("range lower continuation"));
                        };
                        match result {
                            Err(error) => completed = Some(Err(error)),
                            Ok(Some(lower)) => {
                                tasks.push(Task::ForRangeAfterUpper {
                                    statement,
                                    depth,
                                    inclusive,
                                    lower,
                                    surviving_temps,
                                });
                                tasks.push(Task::Expr {
                                    value: upper,
                                    expected: Some(Ty::I64),
                                    depth,
                                    borrow: false,
                                });
                            }
                            Ok(None) => {
                                completed = Some(Err(CompileError::InvalidIr("range lower flow")))
                            }
                        }
                    }
                    Some(Task::ForRangeAfterUpper { .. }) => {
                        let Some(Task::ForRangeAfterUpper {
                            statement,
                            depth,
                            inclusive,
                            lower,
                            surviving_temps,
                        }) = tasks.pop()
                        else {
                            return Err(CompileError::InvalidIr("range upper continuation"));
                        };
                        let started = result.and_then(|value| {
                            self.begin_for_range(
                                statement,
                                lower,
                                value.ok_or(CompileError::InvalidIr("range upper flow"))?,
                                inclusive,
                                surviving_temps,
                            )
                        });
                        match started {
                            Err(error) => completed = Some(Err(error)),
                            Ok(state) => {
                                schedule_for_body(self, &mut tasks, statement, depth, state)?
                            }
                        }
                    }
                    Some(Task::MatchGuardAfter { .. }) => {
                        let Some(Task::MatchGuardAfter {
                            mut state,
                            arm,
                            saved,
                        }) = tasks.pop()
                        else {
                            return Err(CompileError::InvalidIr("match guard continuation"));
                        };
                        let expression = state.is_expression();
                        let ready = result.and_then(|value| {
                            self.finish_match_guard(
                                &mut state,
                                arm,
                                saved,
                                value.ok_or(CompileError::InvalidIr("match guard flow"))?,
                            )
                        });
                        match ready {
                            Ok(arm) => tasks.push(Task::MatchArmReady { state, arm }),
                            Err(error) => {
                                if expression {
                                    completed_value = Some(Err(error));
                                } else {
                                    completed = Some(Err(error));
                                }
                            }
                        }
                    }
                    Some(Task::MatchIndexAfterSelector(_)) => {
                        let Some(Task::MatchIndexAfterSelector(mut state)) = tasks.pop() else {
                            return Err(CompileError::InvalidIr("match index continuation"));
                        };
                        let expression = state.expected.is_some();
                        match result {
                            Err(error) => {
                                if expression {
                                    completed_value = Some(Err(error));
                                } else {
                                    completed = Some(Err(error));
                                }
                            }
                            Ok(None) => {
                                let error = CompileError::InvalidIr("match index flow");
                                if expression {
                                    completed_value = Some(Err(error));
                                } else {
                                    completed = Some(Err(error));
                                }
                            }
                            Ok(Some(flow)) if flow.diverged() => {
                                let flow = Flow::Diverged { ty: state.expected };
                                if expression {
                                    completed_value = Some(Ok(Some(flow)));
                                } else {
                                    completed = Some(Ok(()));
                                }
                            }
                            Ok(Some(flow)) => {
                                if let Some(temp) = flow.temp() {
                                    state.ids.push(temp);
                                } else if self.emission == EmissionMode::Live {
                                    let error = CompileError::InvalidIr("match index");
                                    if expression {
                                        completed_value = Some(Err(error));
                                    } else {
                                        completed = Some(Err(error));
                                    }
                                    continue;
                                }
                                tasks.push(Task::MatchIndexNext(state));
                            }
                        }
                    }
                    Some(Task::WhileAfterSecondCondition(_)) => {
                        let Some(Task::WhileAfterSecondCondition(state)) = tasks.pop() else {
                            return Err(CompileError::InvalidIr("while backedge condition"));
                        };
                        if matches!(&result, Ok(Some(flow)) if !flow.diverged()) {
                            self.terminated = None;
                            let surviving = self.live_temps.clone();
                            let saved = self.scope;
                            self.scope = ScopeId(self.next_scope);
                            self.next_scope += 1;
                            self.scope_parents.push(Some(saved));
                            self.scopes.push(Vec::new());
                            tasks.push(Task::WhileAfterSecond {
                                checkpoint: state.checkpoint,
                                temp_checkpoint: state.temp_checkpoint,
                                locals: state.locals,
                                scopes: state.scopes,
                                before_errors: state.before_errors,
                                affected: state.affected,
                                body_span: state.body.span.clone(),
                                condition_diverges: state.condition_diverges,
                            });
                            tasks.push(Task::Block(Box::new(Frame {
                                statements: &state.body.statements,
                                depth: state.depth + 1,
                                index: 0,
                                unreachable: None,
                                scoped: Some(Scoped {
                                    block: state.body,
                                    surviving,
                                    saved,
                                }),
                                pending: None,
                                value_tail: None,
                            })));
                            continue;
                        }
                        self.restore_availability(state.checkpoint);
                        self.restore_temps(state.temp_checkpoint);
                        self.locals.truncate(state.locals);
                        self.name_spans.truncate(state.locals);
                        self.next_scope = state.scopes;
                        self.scope_parents.truncate(state.scopes);
                        self.dead_depth -= 1;
                        self.terminated = state.condition_diverges.then_some(Termination::Diverged);
                        completed = Some(match result {
                            Err(error) => Err(add_backedge_to_error(
                                error,
                                &state.body.span,
                                &state.affected,
                            )),
                            Ok(None) => Err(CompileError::InvalidIr("while condition flow")),
                            Ok(Some(_)) => Ok(()),
                        });
                    }
                    Some(Task::WhileAfterCondition(_)) => {
                        let Some(Task::WhileAfterCondition(state)) = tasks.pop() else {
                            return Err(CompileError::InvalidIr("while condition continuation"));
                        };
                        let test_flow = match result {
                            Ok(Some(flow)) => flow,
                            Ok(None) => {
                                completed =
                                    Some(Err(CompileError::InvalidIr("while condition flow")));
                                continue;
                            }
                            Err(error) => {
                                completed = Some(Err(error));
                                continue;
                            }
                        };
                        let condition_diverges = test_flow.diverged();
                        let while_state = if condition_diverges
                            || state.entry_emission == EmissionMode::Analyze
                        {
                            self.terminated = None;
                            self.emission = EmissionMode::Analyze;
                            WhileState::Analyze {
                                condition: state.condition,
                                body: state.body,
                                depth: state.depth,
                                checkpoint: state.checkpoint,
                                temp_checkpoint: state.temp_checkpoint,
                                prior_locals: state.prior_locals,
                                condition_diverges,
                                before_errors: diagnostics.len(),
                            }
                        } else {
                            if test_flow.ty() != Some(&Ty::Bool) {
                                completed =
                                    Some(Err(CompileError::InvalidIr("while condition type")));
                                continue;
                            }
                            let condition_state = self.restore_availability(state.checkpoint);
                            let test = test_flow
                                .temp()
                                .ok_or(CompileError::InvalidIr("while condition flow"))?;
                            let mut ungrouped = state.condition;
                            while let Expr::Group(inner, _) = ungrouped {
                                ungrouped = inner;
                            }
                            let literal_true =
                                self.literal_true_loops && matches!(ungrouped, Expr::Bool(true, _));
                            let branch = if literal_true {
                                self.statement_op(
                                    Op::DropValue(test),
                                    state.condition.span().clone(),
                                    Access::Own,
                                    false,
                                );
                                self.finish_block(Terminator::Jump {
                                    target: BlockId(self.blocks.len() + 1),
                                    span: state.condition.span().clone(),
                                    charges: vec![],
                                })
                            } else {
                                self.finish_block(Terminator::BranchBool {
                                    condition: test,
                                    yes: BlockId(self.blocks.len() + 1),
                                    no: BlockId(usize::MAX),
                                    span: state.condition.span().clone(),
                                    charges: vec![],
                                })
                            };
                            self.restore_temps(state.temp_checkpoint);
                            for (id, value) in &condition_state {
                                self.set_availability(*id, value.clone());
                            }
                            self.loop_stack.push(LoopContext {
                                continue_target: state.condition_header,
                                scope_depth: self.scopes.len(),
                                cleanup_span: state.body.close_span.clone(),
                                breaks: Vec::new(),
                                continues: Vec::new(),
                                match_depth: self.match_boundaries.len(),
                                surviving_temps: state.surviving_temps,
                            });
                            WhileState::Live {
                                entry_emission: state.entry_emission,
                                condition_header: state.condition_header,
                                checkpoint: state.checkpoint,
                                temp_checkpoint: state.temp_checkpoint,
                                prior_locals: state.prior_locals,
                                condition_state,
                                branch,
                                literal_true,
                                preheader: state.preheader,
                                body: state.body,
                            }
                        };
                        let surviving = self.live_temps.clone();
                        let saved = self.scope;
                        self.scope = ScopeId(self.next_scope);
                        self.next_scope += 1;
                        self.scope_parents.push(Some(saved));
                        self.scopes.push(Vec::new());
                        let body = state.body;
                        let depth = state.depth;
                        tasks.push(Task::WhileAfterBody {
                            state: Box::new(while_state),
                        });
                        tasks.push(Task::Block(Box::new(Frame {
                            statements: &body.statements,
                            depth: depth + 1,
                            index: 0,
                            unreachable: None,
                            scoped: Some(Scoped {
                                block: body,
                                surviving,
                                saved,
                            }),
                            pending: None,
                            value_tail: None,
                        })));
                    }
                    Some(Task::IfAfterCondition { .. }) => {
                        let Some(Task::IfAfterCondition {
                            then_block,
                            otherwise,
                            span,
                            depth,
                        }) = tasks.pop()
                        else {
                            return Err(CompileError::InvalidIr("if statement continuation"));
                        };
                        let started = result.and_then(|value| {
                            self.begin_if_statement(
                                span,
                                value.ok_or(CompileError::InvalidIr("if condition flow"))?,
                            )
                        });
                        match started {
                            Ok(started) => {
                                let surviving = self.live_temps.clone();
                                let saved = self.scope;
                                self.scope = ScopeId(self.next_scope);
                                self.next_scope += 1;
                                self.scope_parents.push(Some(saved));
                                self.scopes.push(Vec::new());
                                tasks.push(Task::IfAfterThen {
                                    started,
                                    then_block,
                                    otherwise,
                                    depth,
                                });
                                tasks.push(Task::Block(Box::new(Frame {
                                    statements: &then_block.statements,
                                    depth: depth + 1,
                                    index: 0,
                                    unreachable: None,
                                    scoped: Some(Scoped {
                                        block: then_block,
                                        surviving,
                                        saved,
                                    }),
                                    pending: None,
                                    value_tail: None,
                                })));
                            }
                            Err(error) => completed = Some(Err(error)),
                        }
                    }
                    Some(Task::IfExprAfterCondition { .. }) => {
                        let Some(Task::IfExprAfterCondition {
                            then_block,
                            otherwise,
                            span,
                            depth,
                            expected,
                        }) = tasks.pop()
                        else {
                            return Err(CompileError::InvalidIr("if condition continuation"));
                        };
                        let started = result.and_then(|value| {
                            self.begin_if_expression(
                                then_block,
                                otherwise,
                                span,
                                depth,
                                expected.as_ref(),
                                value.ok_or(CompileError::InvalidIr("if condition flow"))?,
                            )
                        });
                        match started {
                            Ok(started) => {
                                let expected = Some(started.inferred_ty.clone());
                                let outer_diagnostics = std::mem::take(diagnostics);
                                tasks.push(Task::IfExprAfterThen(
                                    Box::new(started),
                                    outer_diagnostics,
                                ));
                                tasks.push(Task::ValueBlockEnter {
                                    block: then_block,
                                    expected,
                                    if_span: span,
                                    depth,
                                });
                            }
                            Err(error) => completed_value = Some(Err(error)),
                        }
                    }
                    Some(Task::ClosureAfterReceiver(_)) => {
                        let Some(Task::ClosureAfterReceiver(state)) = tasks.pop() else {
                            return Err(CompileError::InvalidIr("closure receiver continuation"));
                        };
                        match result {
                            Err(error) => completed_value = Some(Err(error)),
                            Ok(None) => {
                                completed_value =
                                    Some(Err(CompileError::InvalidIr("closure receiver flow")))
                            }
                            Ok(Some(receiver)) if receiver.diverged() => {
                                completed_value = Some(Ok(Some(Flow::Diverged {
                                    ty: Some(state.inferred),
                                })));
                            }
                            Ok(Some(receiver)) => {
                                match closure_call_state(self, *state, receiver, None) {
                                    Ok(call) => tasks.push(Task::UserCallNext(Box::new(call))),
                                    Err(error) => completed_value = Some(Err(error)),
                                }
                            }
                        }
                    }
                    Some(Task::AssignAfterSelector(_)) => {
                        let Some(Task::AssignAfterSelector(mut state)) = tasks.pop() else {
                            return Err(CompileError::InvalidIr(
                                "assignment selector continuation",
                            ));
                        };
                        completed_value = match result {
                            Err(error) => Some(Err(error)),
                            Ok(None) => {
                                Some(Err(CompileError::InvalidIr("assignment selector flow")))
                            }
                            Ok(Some(flow)) => {
                                if flow.diverged() {
                                    state.diverged = true;
                                    self.emission = EmissionMode::Analyze;
                                }
                                if !state.base_unresolved
                                    && flow
                                        .ty()
                                        .is_some_and(|ty| *ty != Ty::I64 && *ty != Ty::Never)
                                {
                                    Some(Err(CompileError::InvalidIr("index type")))
                                } else {
                                    if let Some(id) = flow.temp() {
                                        state.ids.push(id);
                                    }
                                    state.next += 1;
                                    tasks.push(Task::AssignNextSelector(state));
                                    None
                                }
                            }
                        };
                    }
                    Some(Task::AssignAfterValue(_)) => {
                        let Some(Task::AssignAfterValue(state)) = tasks.pop() else {
                            return Err(CompileError::InvalidIr("assignment value continuation"));
                        };
                        let acquired = state.compound.as_ref().map(|(loan, _, _)| {
                            vec![(state.local, *loan, state.place.span().clone())]
                        });
                        completed = Some(result.and_then(|value| {
                            let flow =
                                value.ok_or(CompileError::InvalidIr("assignment value flow"))?;
                            let indexed = !state.selectors.is_empty();
                            if indexed {
                                self.check_available(state.local, state.root.span().clone())?;
                                if !self.locals[state.local.0].mutable
                                    && !(self.defer_ownership
                                        && self.emission == EmissionMode::Live)
                                {
                                    return Err(self.immutable_write(state.root, state.local));
                                }
                            }
                            if state.diverged {
                                self.terminated = Some(Termination::Diverged);
                                return Ok(());
                            }
                            if flow.diverged() {
                                if let Some((loan, _, _)) = state.compound {
                                    self.statement_op(
                                        Op::EndLoan(loan),
                                        state.place.span().clone(),
                                        Access::Own,
                                        false,
                                    );
                                }
                                return Ok(());
                            }
                            if let Some((loan, loan_temp, old)) = state.compound {
                                if self.emission == EmissionMode::Analyze {
                                    return Ok(());
                                }
                                let rhs = flow
                                    .temp()
                                    .ok_or(CompileError::InvalidIr("compound rhs flow"))?;
                                if flow.ty() != Some(&Ty::I64) {
                                    return Err(CompileError::InvalidIr(
                                        "compound rhs type passed checker",
                                    ));
                                }
                                let op = match state.op {
                                    parse::AssignOp::Add => Binary::Add,
                                    parse::AssignOp::Sub => Binary::Sub,
                                    parse::AssignOp::Mul => Binary::Mul,
                                    parse::AssignOp::Div => Binary::Div,
                                    parse::AssignOp::Rem => Binary::Rem,
                                    parse::AssignOp::Set => {
                                        unreachable!("plain assignment has no loan")
                                    }
                                };
                                self.statement_op(
                                    Op::CompoundStore {
                                        loan: loan_temp,
                                        old,
                                        rhs,
                                        op,
                                    },
                                    Span {
                                        file: state.place.span().file.clone(),
                                        start: state.place.span().start,
                                        end: state.value.span().end,
                                    },
                                    Access::Write,
                                    false,
                                );
                                self.statement_op(
                                    Op::EndLoan(loan),
                                    state.place.span().clone(),
                                    Access::Own,
                                    false,
                                );
                                return Ok(());
                            }
                            self.check_place_access(
                                state.local,
                                state.root,
                                state.root,
                                "write",
                                indexed,
                            )?;
                            if !indexed {
                                self.mark_consumed(state.local, None);
                            }
                            if self.emission == EmissionMode::Analyze {
                                return Ok(());
                            }
                            let temp = flow
                                .temp()
                                .ok_or(CompileError::InvalidIr("assignment value flow"))?;
                            if flow.ty().is_some_and(|ty| {
                                *ty != state.target_ty
                                    && *ty != Ty::Never
                                    && state.target_ty != Ty::Never
                            }) {
                                return Err(CompileError::InvalidIr(
                                    "assignment type passed checker",
                                ));
                            }
                            self.statement_op(
                                if indexed {
                                    Op::WriteIndex {
                                        local: state.local,
                                        indexes: state.ids,
                                        site: index_site(state.place, state.place.span().clone()),
                                        value: temp,
                                    }
                                } else {
                                    Op::WritePlace {
                                        local: state.local,
                                        value: temp,
                                    }
                                },
                                statement_span(state.statement),
                                Access::Write,
                                false,
                            );
                            Ok(())
                        }));
                        if let Some(acquired) = acquired {
                            self.release_call_loans(&acquired);
                        }
                    }
                    Some(Task::StatementExprAfter(_)) => {
                        let Some(Task::StatementExprAfter(state)) = tasks.pop() else {
                            return Err(CompileError::InvalidIr(
                                "statement expression continuation",
                            ));
                        };
                        completed = Some(match *state {
                            StatementExprState::Let {
                                statement,
                                mutable,
                                name,
                                binding_span,
                                expected,
                            } => match result {
                                Err(
                                    error @ (CompileError::Rejected(_) | CompileError::Dependent),
                                ) => {
                                    let id = declare_binding(
                                        self,
                                        name,
                                        mutable,
                                        binding_span,
                                        expected,
                                    )?;
                                    self.poisoned.insert(id);
                                    Err(error)
                                }
                                Err(error) => Err(error),
                                Ok(Some(flow)) => {
                                    let id = declare_binding(
                                        self,
                                        name,
                                        mutable,
                                        binding_span,
                                        expected,
                                    )?;
                                    if flow.diverged() || self.emission == EmissionMode::Analyze {
                                        Ok(())
                                    } else if let Some(temp) = flow.temp() {
                                        self.statement_op(
                                            Op::WritePlace {
                                                local: id,
                                                value: temp,
                                            },
                                            statement_span(statement),
                                            Access::Write,
                                            false,
                                        );
                                        Ok(())
                                    } else {
                                        Err(CompileError::InvalidIr("binding value flow"))
                                    }
                                }
                                Ok(None) => Err(CompileError::InvalidIr("binding value flow")),
                            },
                            StatementExprState::Expr(statement) => result.and_then(|value| {
                                let flow = value
                                    .ok_or(CompileError::InvalidIr("expression value flow"))?;
                                if flow.diverged() || self.emission == EmissionMode::Analyze {
                                    return Ok(());
                                }
                                let temp = flow
                                    .temp()
                                    .ok_or(CompileError::InvalidIr("expression value flow"))?;
                                self.statement_op(
                                    Op::DropValue(temp),
                                    statement_span(statement),
                                    Access::Own,
                                    false,
                                );
                                Ok(())
                            }),
                            StatementExprState::Return(_statement) => result.and_then(|value| {
                                let flow =
                                    value.ok_or(CompileError::InvalidIr("return value flow"))?;
                                if flow.diverged() || self.emission == EmissionMode::Analyze {
                                    self.terminated = Some(Termination::Diverged);
                                    return Ok(());
                                }
                                let temp = flow
                                    .temp()
                                    .ok_or(CompileError::InvalidIr("return value flow"))?;
                                if flow.ty() != Some(&self.result) && self.dead_depth == 0 {
                                    return Err(CompileError::InvalidIr(
                                        "return type passed checker",
                                    ));
                                }
                                self.terminated = Some(Termination::Return(temp));
                                Ok(())
                            }),
                        });
                    }
                    Some(Task::ArgumentPlaceAfterIndex(_)) => {
                        let Some(Task::ArgumentPlaceAfterIndex(mut state)) = tasks.pop() else {
                            return Err(CompileError::InvalidIr("loan index continuation"));
                        };
                        completed_value = match result {
                            Err(error) => Some(Err(error)),
                            Ok(None) => Some(Err(CompileError::InvalidIr("loan index flow"))),
                            Ok(Some(flow)) if flow.diverged() => Some(Ok(Some(Flow::Diverged {
                                ty: (state.ty != Ty::Never).then_some(state.ty),
                            }))),
                            Ok(Some(flow)) if flow.ty() != Some(&Ty::I64) => {
                                Some(Err(CompileError::InvalidIr("loan index type")))
                            }
                            Ok(Some(flow)) => {
                                if let Some(id) = flow.temp() {
                                    state.ids.push(id);
                                }
                                tasks.push(Task::ArgumentPlaceNext(state));
                                None
                            }
                        };
                    }
                    Some(Task::LogicalLeft(_)) => {
                        let Some(Task::LogicalLeft(mut state)) = tasks.pop() else {
                            unreachable!()
                        };
                        completed_value = match result {
                            Err(error) => Some(Err(error)),
                            Ok(None) => Some(Err(CompileError::InvalidIr("logical left flow"))),
                            Ok(Some(left)) => {
                                if left.ty() != Some(&Ty::Bool) && !left.diverged() {
                                    Some(Err(CompileError::InvalidIr("logical left type")))
                                } else {
                                    state.left_diverges = left.diverged();
                                    if state.left_diverges {
                                        self.emission = EmissionMode::Analyze;
                                        self.terminated = None;
                                    }
                                    state.branch_mode = self.emission;
                                    state.temp_checkpoint = self.temp_undo.len();
                                    state.checkpoint = self.consumed_undo.len();
                                    state.prior_locals = self.locals.len();
                                    if state.branch_mode == EmissionMode::Live {
                                        let condition = left
                                            .temp()
                                            .ok_or(CompileError::InvalidIr("logical condition"))?;
                                        let branch = self.finish_block(Terminator::BranchBool {
                                            condition,
                                            yes: BlockId(usize::MAX),
                                            no: BlockId(usize::MAX),
                                            span: state.span.clone(),
                                            charges: vec![],
                                        });
                                        let skip_edge = usize::from(state.op == "and");
                                        self.enter_edge(branch, skip_edge)?;
                                        let skip_start = BlockId(self.blocks.len());
                                        if let Terminator::BranchBool { yes, no, .. } =
                                            &mut self.blocks[branch.0].terminator
                                        {
                                            if skip_edge == 0 {
                                                *yes = skip_start;
                                            } else {
                                                *no = skip_start;
                                            }
                                        }
                                        let skip_value = self.emitted(
                                            Op::ConstBool(state.op == "or"),
                                            Ty::Bool,
                                            state.span.clone(),
                                            Access::Own,
                                            false,
                                        );
                                        let skip_temp = skip_value
                                            .temp()
                                            .ok_or(CompileError::InvalidIr("logical skip value"))?;
                                        self.union_slot(
                                            &mut state.slot,
                                            skip_temp,
                                            Ty::Bool,
                                            &state.inferred,
                                            state.span,
                                        )?;
                                        state.skip_end =
                                            Some(self.finish_block(Terminator::Jump {
                                                target: BlockId(usize::MAX),
                                                span: state.span.clone(),
                                                charges: vec![],
                                            }));
                                        state.skip_state =
                                            self.restore_availability(state.checkpoint);
                                        self.restore_temps(state.temp_checkpoint);
                                        let rhs_edge = 1 - skip_edge;
                                        self.enter_edge(branch, rhs_edge)?;
                                        let rhs_start = BlockId(self.blocks.len());
                                        if let Terminator::BranchBool { yes, no, .. } =
                                            &mut self.blocks[branch.0].terminator
                                        {
                                            if rhs_edge == 0 {
                                                *yes = rhs_start;
                                            } else {
                                                *no = rhs_start;
                                            }
                                        }
                                        self.dead_depth += 1;
                                    }
                                    let right = state.right;
                                    let depth = state.depth;
                                    tasks.push(Task::LogicalRight(state));
                                    tasks.push(Task::Expr {
                                        value: right,
                                        expected: Some(Ty::Bool),
                                        depth: depth + 1,
                                        borrow: false,
                                    });
                                    None
                                }
                            }
                        };
                    }
                    Some(Task::LogicalRight(_)) => {
                        let Some(Task::LogicalRight(mut state)) = tasks.pop() else {
                            unreachable!()
                        };
                        completed_value = Some(result.and_then(|value| {
                            let right =
                                value.ok_or(CompileError::InvalidIr("logical right flow"))?;
                            if right.ty() != Some(&Ty::Bool) && !right.diverged() {
                                return Err(CompileError::InvalidIr("logical right type"));
                            }
                            if state.branch_mode == EmissionMode::Analyze {
                                if state.left_diverges {
                                    self.terminated = Some(Termination::Diverged);
                                    return Ok(Some(Flow::Diverged {
                                        ty: Some(state.inferred),
                                    }));
                                }
                                return Ok(Some(Flow::Value {
                                    temp: None,
                                    ty: state.inferred,
                                }));
                            }
                            self.emission = state.branch_mode;
                            if let Some(temp) = right.temp() {
                                self.union_slot(
                                    &mut state.slot,
                                    temp,
                                    Ty::Bool,
                                    &state.inferred,
                                    state.span,
                                )?;
                            }
                            let rhs_continues = !right.diverged() && self.terminated.is_none();
                            let terminated = self.terminated.take();
                            let rhs_end =
                                if right.diverged() || terminated == Some(Termination::Diverged) {
                                    None
                                } else {
                                    Some(self.finish_block(
                                        if let Some(Termination::Return(value)) = terminated {
                                            Terminator::Return {
                                                value: Some(value),
                                                span: state.span.clone(),
                                                charges: vec![],
                                            }
                                        } else {
                                            Terminator::Jump {
                                                target: BlockId(usize::MAX),
                                                span: state.span.clone(),
                                                charges: vec![],
                                            }
                                        },
                                    ))
                                };
                            let rhs_state = self.restore_availability(state.checkpoint);
                            self.restore_temps(state.temp_checkpoint);
                            self.enter_edge(
                                state
                                    .skip_end
                                    .ok_or(CompileError::InvalidIr("logical skip edge"))?,
                                0,
                            )?;
                            self.dead_depth -= 1;
                            let join = BlockId(self.blocks.len());
                            if let Some(skip_end) = state.skip_end
                                && let Terminator::Jump { target, .. } =
                                    &mut self.blocks[skip_end.0].terminator
                            {
                                *target = join;
                            }
                            if let Some(rhs_end) = rhs_end
                                && let Terminator::Jump { target, .. } =
                                    &mut self.blocks[rhs_end.0].terminator
                            {
                                *target = join;
                            }
                            self.merge_availability(
                                state.skip_state,
                                rhs_state,
                                true,
                                rhs_continues,
                                state.prior_locals,
                            );
                            let slot = state
                                .slot
                                .ok_or(CompileError::InvalidIr("logical branch value slot"))?;
                            Ok(Some(self.emitted(
                                Op::TransferValue(slot),
                                state.inferred,
                                state.span.clone(),
                                Access::Own,
                                false,
                            )))
                        }));
                    }
                    Some(Task::BinaryLeft(_)) => {
                        let Some(Task::BinaryLeft(mut state)) = tasks.pop() else {
                            return Err(CompileError::InvalidIr("binary left continuation"));
                        };
                        completed_value = match result {
                            Err(error) => Some(Err(error)),
                            Ok(None) => Some(Err(CompileError::InvalidIr("binary left flow"))),
                            Ok(Some(left_flow)) => {
                                if left_flow.diverged() {
                                    self.emission = EmissionMode::Analyze;
                                    self.terminated = None;
                                }
                                let left_type = if state.op == "==" {
                                    Some(self.resolved_expr(state.left))
                                } else {
                                    None
                                };
                                match left_type.transpose() {
                                    Err(error) => Some(Err(error)),
                                    Ok(left_type) => {
                                        if matches!(left_type, Some(Ty::Array(_))) {
                                            state.left_root = place_parts(state.left)
                                                .ok()
                                                .and_then(|(root, _)| place_name(root))
                                                .and_then(|name| {
                                                    self.visible
                                                        .get(name)
                                                        .and_then(|ids| ids.last())
                                                        .copied()
                                                });
                                        }
                                        if let Some(root) = state.left_root {
                                            self.active_loans.entry(root).or_default().push(
                                                ActiveLoan {
                                                    permission: LoanPermission::Read,
                                                    span: state.left.span().clone(),
                                                    loan: LoanId(state.left.span().start as usize),
                                                    parent: None,
                                                    compound_assignment: false,
                                                },
                                            );
                                        }
                                        state.left_flow = Some(left_flow);
                                        let right = state.right;
                                        let depth = state.depth;
                                        tasks.push(Task::BinaryRight(state));
                                        tasks.push(Task::Expr {
                                            value: right,
                                            expected: None,
                                            depth: depth + 1,
                                            borrow: true,
                                        });
                                        None
                                    }
                                }
                            }
                        };
                    }
                    Some(Task::BinaryRight(_)) => {
                        let Some(Task::BinaryRight(state)) = tasks.pop() else {
                            return Err(CompileError::InvalidIr("binary right continuation"));
                        };
                        if let Some(root) = state.left_root {
                            self.active_loans
                                .get_mut(&root)
                                .and_then(|loans| loans.pop());
                        }
                        completed_value = Some(result.and_then(|value| {
                            let right_flow =
                                value.ok_or(CompileError::InvalidIr("binary right flow"))?;
                            let left_flow = state
                                .left_flow
                                .ok_or(CompileError::InvalidIr("binary left flow"))?;
                            let operator = match state.op {
                                "+" => Binary::Add,
                                "-" => Binary::Sub,
                                "*" => Binary::Mul,
                                "/" => Binary::Div,
                                "%" => Binary::Rem,
                                "<" => Binary::Less,
                                "<=" => Binary::LessEqual,
                                ">" => Binary::Greater,
                                ">=" => Binary::GreaterEqual,
                                "==" => Binary::Equal,
                                "!=" => Binary::NotEqual,
                                _ => return Err(CompileError::InvalidIr("binary operation")),
                            };
                            let ty = state.inferred;
                            if left_flow.diverged() || right_flow.diverged() {
                                self.terminated = Some(Termination::Diverged);
                                self.emission = EmissionMode::Analyze;
                                Ok(Some(Flow::Diverged { ty: Some(ty) }))
                            } else if self.emission == EmissionMode::Analyze {
                                Ok(Some(Flow::Value { temp: None, ty }))
                            } else if let (Some(left), Some(right)) =
                                (left_flow.temp(), right_flow.temp())
                            {
                                Ok(Some(self.emitted(
                                    Op::Binary {
                                        op: operator,
                                        left,
                                        right,
                                    },
                                    ty,
                                    state.span.clone(),
                                    Access::Own,
                                    false,
                                )))
                            } else {
                                Err(CompileError::InvalidIr("binary operand flow"))
                            }
                        }));
                    }
                    Some(Task::ArrayAfterItem(_)) => {
                        let Some(Task::ArrayAfterItem(mut state)) = tasks.pop() else {
                            return Err(CompileError::InvalidIr("array item continuation"));
                        };
                        state.next += 1;
                        completed_value = match result {
                            Err(CompileError::Rejected(errors)) => {
                                state
                                    .item_errors
                                    .extend(errors.into_iter().map(|error| *error));
                                tasks.push(Task::ArrayNext(state));
                                None
                            }
                            Err(CompileError::Dependent) => {
                                state.dependent = true;
                                tasks.push(Task::ArrayNext(state));
                                None
                            }
                            Err(error) => Some(Err(error)),
                            Ok(None) => Some(Err(CompileError::InvalidIr("array item flow"))),
                            Ok(Some(flow)) => {
                                if flow.diverged() {
                                    state.diverged = true;
                                    self.emission = EmissionMode::Analyze;
                                } else if let Some(id) = flow.temp() {
                                    state.ids.push(id);
                                }
                                tasks.push(Task::ArrayNext(state));
                                None
                            }
                        };
                    }
                    Some(Task::IndexAfterSelector(_)) => {
                        let Some(Task::IndexAfterSelector(mut state)) = tasks.pop() else {
                            return Err(CompileError::InvalidIr("index selector continuation"));
                        };
                        completed_value = match result {
                            Err(error) => Some(Err(error)),
                            Ok(None) => Some(Err(CompileError::InvalidIr("index selector flow"))),
                            Ok(Some(flow)) => {
                                let IndexKind::Place {
                                    root,
                                    local,
                                    ids,
                                    diverged,
                                    ..
                                } = &mut state.kind
                                else {
                                    return Err(CompileError::InvalidIr("index place state"));
                                };
                                if flow.diverged() {
                                    *diverged = true;
                                    self.emission = EmissionMode::Analyze;
                                }
                                if flow
                                    .ty()
                                    .is_some_and(|ty| *ty != Ty::I64 && *ty != Ty::Never)
                                {
                                    Some(Err(CompileError::InvalidIr("index type")))
                                } else {
                                    if let Some(id) = flow.temp() {
                                        ids.push(id);
                                    }
                                    match self
                                        .check_place_access(*local, root, state.expr, "read", true)
                                    {
                                        Err(error) => Some(Err(error)),
                                        Ok(()) => {
                                            self.statement_op(
                                                Op::CheckIndex {
                                                    local: *local,
                                                    indexes: ids.clone(),
                                                    site: IndexSite {
                                                        context: state.expr.span().clone(),
                                                        selectors: index_selectors(state.expr)
                                                            .into_iter()
                                                            .take(ids.len())
                                                            .collect(),
                                                    },
                                                },
                                                state.span.clone(),
                                                Access::Read,
                                                false,
                                            );
                                            tasks.push(Task::IndexNext(state));
                                            None
                                        }
                                    }
                                }
                            }
                        };
                    }
                    Some(Task::IndexAfterBase(_)) => {
                        let Some(Task::IndexAfterBase(mut state)) = tasks.pop() else {
                            return Err(CompileError::InvalidIr("index base continuation"));
                        };
                        completed_value = match result {
                            Err(error) => Some(Err(error)),
                            Ok(None) => Some(Err(CompileError::InvalidIr("index base flow"))),
                            Ok(Some(base)) => {
                                if let IndexKind::Value { base: slot } = &mut state.kind {
                                    if base.diverged() {
                                        self.terminated = None;
                                        self.emission = EmissionMode::Analyze;
                                    }
                                    *slot = Some(base);
                                }
                                let index = state.index;
                                let depth = state.depth;
                                tasks.push(Task::IndexAfterValueSelector(state));
                                tasks.push(Task::Expr {
                                    value: index,
                                    expected: Some(Ty::I64),
                                    depth: depth + 1,
                                    borrow: true,
                                });
                                None
                            }
                        };
                    }
                    Some(Task::IndexAfterValueSelector(_)) => {
                        let Some(Task::IndexAfterValueSelector(state)) = tasks.pop() else {
                            return Err(CompileError::InvalidIr("index value continuation"));
                        };
                        completed_value = Some(result.and_then(|value| {
                            let index_flow =
                                value.ok_or(CompileError::InvalidIr("index selector flow"))?;
                            if matches!(state.kind, IndexKind::Unresolved) {
                                return Ok(Some(Flow::Value {
                                    temp: None,
                                    ty: state.inferred,
                                }));
                            }
                            let IndexKind::Value {
                                base: Some(base_flow),
                            } = state.kind
                            else {
                                return Err(CompileError::InvalidIr("index base state"));
                            };
                            if base_flow.diverged() {
                                self.terminated = Some(Termination::Diverged);
                                return Ok(Some(Flow::Diverged {
                                    ty: (state.inferred != Ty::Never).then_some(state.inferred),
                                }));
                            }
                            if index_flow.diverged() {
                                return Ok(Some(Flow::Diverged {
                                    ty: (state.inferred != Ty::Never).then_some(state.inferred),
                                }));
                            }
                            if index_flow
                                .ty()
                                .is_some_and(|ty| *ty != Ty::I64 && *ty != Ty::Never)
                            {
                                return Err(CompileError::InvalidIr("index type"));
                            }
                            if self.emission == EmissionMode::Analyze {
                                return Ok(Some(Flow::Value {
                                    temp: None,
                                    ty: state.inferred,
                                }));
                            }
                            let (Some(base), Some(index)) = (base_flow.temp(), index_flow.temp())
                            else {
                                return Err(CompileError::InvalidIr("index operand flow"));
                            };
                            Ok(Some(self.emitted(
                                Op::IndexValue {
                                    base,
                                    index,
                                    site: IndexSite {
                                        context: state.span.clone(),
                                        selectors: vec![state.selector.clone()],
                                    },
                                },
                                state.inferred,
                                state.span.clone(),
                                Access::Own,
                                false,
                            )))
                        }));
                    }
                    Some(Task::Block(frame)) if frame.value_tail.is_some() => {
                        let value_tail = frame
                            .value_tail
                            .as_mut()
                            .ok_or(CompileError::InvalidIr("value block continuation"))?;
                        if !value_tail.started {
                            return Err(CompileError::InvalidIr("value block not started"));
                        }
                        if let Some(saved) = value_tail.saved_termination.take() {
                            self.terminated = Some(saved);
                            self.emission = value_tail.saved_emission;
                        }
                        value_tail.result = Some(result.and_then(|value| {
                            value.ok_or(CompileError::InvalidIr("value block flow"))
                        }));
                    }
                    Some(Task::ValueBlockAfter(_)) => {
                        let Some(Task::ValueBlockAfter(scoped)) = tasks.pop() else {
                            return Err(CompileError::InvalidIr("value scope continuation"));
                        };
                        completed_value = Some(match result {
                            Err(error) => {
                                self.leave_scope(scoped.saved)?;
                                Err(error)
                            }
                            Ok(value) => {
                                if value.is_none() && self.terminated.is_none() {
                                    let closing = Span {
                                        file: scoped.block.span.file.clone(),
                                        start: scoped.block.span.end - 1,
                                        end: scoped.block.span.end,
                                    };
                                    let mut d = crate::check::names::diagnostic(
                                        "NX-E1302",
                                        Some("missing_branch_value"),
                                        diagnostic::Stage::Ownership,
                                        diagnostic::Operation::Branch,
                                        closing,
                                    );
                                    d.related_spans.push(diagnostic::RelatedSpan {
                                        role: diagnostic::RelatedRole::Context,
                                        span: scoped.if_span.clone(),
                                    });
                                    diagnostics.push(*d);
                                }
                                self.finish_scoped_value_block(
                                    scoped.block,
                                    scoped.surviving,
                                    scoped.saved,
                                    value,
                                )
                            }
                        });
                    }
                    Some(Task::MatchAfterArm { .. }) => {
                        let Some(Task::MatchAfterArm { mut state, arm }) = tasks.pop() else {
                            return Err(CompileError::InvalidIr("match value continuation"));
                        };
                        completed = match result {
                            Ok(value) => match self.finish_match_arm(&mut state, arm, value) {
                                Ok(()) => {
                                    tasks.push(Task::MatchNext(state));
                                    None
                                }
                                Err(error) => Some(Err(error)),
                            },
                            Err(error) => Some(Err(error)),
                        };
                    }
                    Some(Task::MatchAfterScrutinee { .. }) => {
                        let Some(Task::MatchAfterScrutinee {
                            selection,
                            depth,
                            expected,
                            prelude,
                        }) = tasks.pop()
                        else {
                            return Err(CompileError::InvalidIr("match scrutinee continuation"));
                        };
                        let started = result.and_then(|value| {
                            self.begin_match_selection_after_selected(
                                selection,
                                expected.is_some(),
                                depth,
                                expected.as_ref(),
                                prelude,
                                value.ok_or(CompileError::InvalidIr("match scrutinee flow"))?,
                                Vec::new(),
                                false,
                            )
                        });
                        match started {
                            Ok(lower_match::MatchStart::Completed(flow)) => {
                                if expected.is_some() {
                                    completed_value = Some(Ok(Some(flow)));
                                } else {
                                    completed = Some(Ok(()));
                                }
                            }
                            Ok(lower_match::MatchStart::Active(state)) => {
                                tasks.push(Task::MatchNext(state));
                            }
                            Err(error) => {
                                if expected.is_some() {
                                    completed_value = Some(Err(error));
                                } else {
                                    completed = Some(Err(error));
                                }
                            }
                        }
                    }
                    Some(Task::UserCallAfterArgument { .. }) => {
                        let Some(Task::UserCallAfterArgument {
                            mut state,
                            slot,
                            loan_root,
                        }) = tasks.pop()
                        else {
                            return Err(CompileError::InvalidIr("call argument continuation"));
                        };
                        let arg = &state.args[state.index - 1];
                        let result = result.and_then(|flow| {
                            Ok((
                                flow.ok_or(CompileError::InvalidIr("call argument flow"))?,
                                loan_root,
                            ))
                        });
                        match finish_user_argument(&mut state, slot, arg, result) {
                            Ok(()) => {
                                if state.diverged {
                                    self.emission = EmissionMode::Analyze;
                                }
                                tasks.push(Task::UserCallNext(state));
                            }
                            Err(error) => {
                                self.release_call_loans(&state.acquired);
                                completed_value = Some(Err(error));
                            }
                        }
                    }
                    Some(Task::VariantAfter { .. }) => {
                        let Some(Task::VariantAfter {
                            tag,
                            span,
                            inferred,
                        }) = tasks.pop()
                        else {
                            return Err(CompileError::InvalidIr("variant continuation"));
                        };
                        completed_value = Some(result.and_then(|value| {
                            let flow =
                                value.ok_or(CompileError::InvalidIr("variant payload flow"))?;
                            if flow.diverged() {
                                return Ok(Some(Flow::Diverged { ty: Some(inferred) }));
                            }
                            if self.emission == EmissionMode::Analyze {
                                return Ok(Some(Flow::Value {
                                    temp: None,
                                    ty: inferred,
                                }));
                            }
                            let payload = flow
                                .temp()
                                .ok_or(CompileError::InvalidIr("result payload"))?;
                            Ok(Some(self.emitted(
                                Op::BuildVariant {
                                    tag,
                                    payload: Some(payload),
                                },
                                inferred,
                                span.clone(),
                                Access::Own,
                                false,
                            )))
                        }));
                    }
                    Some(Task::OperatorUnaryAfter { .. }) => {
                        let Some(Task::OperatorUnaryAfter { span, inferred, op }) = tasks.pop()
                        else {
                            unreachable!()
                        };
                        completed_value = Some(result.and_then(|value| {
                            let operand =
                                value.ok_or(CompileError::InvalidIr("unary operand flow"))?;
                            if operand.diverged() {
                                return Ok(Some(Flow::Diverged { ty: Some(inferred) }));
                            }
                            if self.emission == EmissionMode::Analyze {
                                return Ok(Some(Flow::Value {
                                    temp: None,
                                    ty: inferred,
                                }));
                            }
                            let operand = operand
                                .temp()
                                .ok_or(CompileError::InvalidIr("unary operand"))?;
                            Ok(Some(self.emitted(
                                Op::Unary { op, operand },
                                inferred,
                                span.clone(),
                                Access::Own,
                                false,
                            )))
                        }));
                    }
                    Some(Task::UnaryReadAfter { .. }) => {
                        let Some(Task::UnaryReadAfter { span, inferred }) = tasks.pop() else {
                            return Err(CompileError::InvalidIr("propagation continuation"));
                        };
                        completed_value = Some(result.and_then(|value| {
                            let operand =
                                value.ok_or(CompileError::InvalidIr("propagation flow"))?;
                            if operand.diverged() {
                                return Ok(Some(Flow::Diverged { ty: Some(inferred) }));
                            }
                            if self.emission == EmissionMode::Analyze {
                                return Ok(Some(Flow::Value {
                                    temp: None,
                                    ty: inferred,
                                }));
                            }
                            let value = operand
                                .temp()
                                .ok_or(CompileError::InvalidIr("propagation operand"))?;
                            Ok(Some(self.emitted(
                                Op::BranchVariant {
                                    value,
                                    result: clone_ty_iterative(&self.result),
                                },
                                inferred,
                                span.clone(),
                                Access::Own,
                                false,
                            )))
                        }));
                    }
                    Some(Task::MatchRestore(_)) => {
                        let Some(Task::MatchRestore(snapshot)) = tasks.pop() else {
                            return Err(CompileError::InvalidIr("match value restore"));
                        };
                        self.restore_match_snapshot(*snapshot);
                        completed_value = Some(result);
                    }
                    Some(Task::MatchExprFinish(_)) => {
                        let Some(Task::MatchExprFinish(outer_diagnostics)) = tasks.pop() else {
                            return Err(CompileError::InvalidIr("match value finish"));
                        };
                        let inner_diagnostics = std::mem::replace(diagnostics, outer_diagnostics);
                        completed_value = Some(if inner_diagnostics.is_empty() {
                            result
                        } else {
                            match result {
                                Ok(_) | Err(CompileError::Dependent) => {
                                    Err(CompileError::Rejected(
                                        inner_diagnostics.into_iter().map(Box::new).collect(),
                                    ))
                                }
                                Err(error) => Err(branch_error(error, inner_diagnostics)),
                            }
                        });
                    }
                    Some(Task::IfExprAfterThen(..)) => {
                        let Some(Task::IfExprAfterThen(mut started, outer_diagnostics)) =
                            tasks.pop()
                        else {
                            return Err(CompileError::InvalidIr("if value continuation"));
                        };
                        started.diagnostics = std::mem::take(diagnostics);
                        *diagnostics = outer_diagnostics;
                        match self.finish_if_then_expression(*started, result) {
                            Ok(state) => tasks.push(Task::IfExprElse(Box::new(state))),
                            Err(error) => completed_value = Some(Err(error)),
                        }
                    }
                    Some(Task::IfExprAfterElse(..)) => {
                        let Some(Task::IfExprAfterElse(mut state, outer_diagnostics)) = tasks.pop()
                        else {
                            return Err(CompileError::InvalidIr("else value continuation"));
                        };
                        state.diagnostics = std::mem::take(diagnostics);
                        *diagnostics = outer_diagnostics;
                        completed_value = Some(self.finish_if_expression(*state, result).map(Some));
                    }
                    _ => return Err(CompileError::InvalidIr("lower value continuation")),
                }
                continue;
            }
            if let Some(result) = completed.take() {
                if tasks.is_empty() {
                    result?;
                    return tail.take().ok_or(CompileError::InvalidIr("lower tail"))?(
                        self,
                        self.terminated.is_some(),
                        None,
                    );
                }
                match tasks.last_mut() {
                    Some(Task::ForAfterBody { .. }) => {
                        let Some(Task::ForAfterBody { state, body }) = tasks.pop() else {
                            return Err(CompileError::InvalidIr("for body continuation"));
                        };
                        completed = Some(self.finish_for_after_body(*state, body, result));
                    }
                    Some(Task::Block(frame)) => {
                        let pending = frame
                            .pending
                            .take()
                            .ok_or(CompileError::InvalidIr("statement continuation"))?;
                        match finish_statement(self, pending, result, diagnostics) {
                            Ok(()) => frame.index += 1,
                            Err(error) => {
                                let Some(Task::Block(frame)) = tasks.pop() else {
                                    return Err(CompileError::InvalidIr("block continuation"));
                                };
                                if let Some(scoped) = frame.scoped {
                                    self.leave_scope(scoped.saved)?;
                                }
                                if tasks.is_empty() {
                                    return Err(error);
                                }
                                completed = Some(Err(error));
                            }
                        }
                    }
                    Some(Task::ValueBlockAfter(_)) => {
                        let Some(Task::ValueBlockAfter(scoped)) = tasks.pop() else {
                            return Err(CompileError::InvalidIr("value scope continuation"));
                        };
                        completed_value = Some(match result {
                            Ok(()) => Ok(None),
                            Err(error) => {
                                self.leave_scope(scoped.saved)?;
                                Err(error)
                            }
                        });
                    }
                    Some(Task::IfAfterThen { .. }) => {
                        let Some(Task::IfAfterThen {
                            started,
                            then_block,
                            otherwise,
                            depth,
                        }) = tasks.pop()
                        else {
                            return Err(CompileError::InvalidIr("if continuation"));
                        };
                        match result {
                            Ok(()) => {
                                let state = self.finish_if_then(started, then_block)?;
                                tasks.push(Task::IfElse {
                                    state,
                                    otherwise,
                                    depth,
                                });
                            }
                            Err(error) => completed = Some(Err(error)),
                        }
                    }
                    Some(Task::IfAfterElse { .. }) => {
                        let Some(Task::IfAfterElse { state, else_span }) = tasks.pop() else {
                            return Err(CompileError::InvalidIr("else continuation"));
                        };
                        completed =
                            Some(result.and_then(|()| self.finish_if_else(state, else_span)));
                    }
                    Some(Task::WhileAfterBody { .. }) => {
                        let Some(Task::WhileAfterBody { state }) = tasks.pop() else {
                            return Err(CompileError::InvalidIr("while continuation"));
                        };
                        match *state {
                            WhileState::Analyze {
                                condition,
                                body,
                                depth,
                                checkpoint,
                                temp_checkpoint,
                                prior_locals,
                                condition_diverges,
                                before_errors,
                            } => {
                                if let Err(error) = result {
                                    completed = Some(Err(error));
                                    continue;
                                }
                                let body_continues =
                                    self.terminated.is_none() && !condition_diverges;
                                let body_state = self.restore_availability(checkpoint);
                                self.restore_temps(temp_checkpoint);
                                if body_continues && diagnostics.len() == before_errors {
                                    let affected: HashSet<u64> = body_state
                                        .iter()
                                        .filter(|(id, state)| {
                                            id.0 < prior_locals
                                                && *state != &self.availability(**id)
                                        })
                                        .map(|(id, _)| self.name_spans[id.0].start)
                                        .collect();
                                    if !affected.is_empty() {
                                        self.merge_availability(
                                            body_state,
                                            HashMap::new(),
                                            true,
                                            true,
                                            prior_locals,
                                        );
                                        let second_checkpoint = self.consumed_undo.len();
                                        let second_temps = self.temp_undo.len();
                                        let locals = self.locals.len();
                                        let scopes = self.next_scope;
                                        self.terminated = None;
                                        let before_second_errors = diagnostics.len();
                                        tasks.push(Task::WhileAfterSecondCondition(Box::new(
                                            WhileSecondConditionState {
                                                body,
                                                depth,
                                                checkpoint: second_checkpoint,
                                                temp_checkpoint: second_temps,
                                                locals,
                                                scopes,
                                                before_errors: before_second_errors,
                                                affected,
                                                condition_diverges,
                                            },
                                        )));
                                        tasks.push(Task::Expr {
                                            value: condition,
                                            expected: Some(Ty::Bool),
                                            depth: 0,
                                            borrow: true,
                                        });
                                        continue;
                                    }
                                }
                                self.dead_depth -= 1;
                                self.terminated =
                                    condition_diverges.then_some(Termination::Diverged);
                                completed = Some(Ok(()));
                            }
                            WhileState::Live {
                                entry_emission,
                                condition_header,
                                checkpoint,
                                temp_checkpoint,
                                prior_locals,
                                condition_state,
                                branch,
                                literal_true,
                                preheader,
                                body,
                            } => {
                                let loop_context = self
                                    .loop_stack
                                    .pop()
                                    .ok_or(CompileError::InvalidIr("loop stack"))?;
                                if let Err(error) = result {
                                    completed = Some(Err(error));
                                    continue;
                                }
                                self.emission = entry_emission;
                                let body_return = self.terminated.take();
                                let body_end = if body_return == Some(Termination::Diverged) {
                                    None
                                } else if let Some(Termination::Return(value)) = body_return {
                                    Some(self.finish_block(Terminator::Return {
                                        value: Some(value),
                                        span: body.span.clone(),
                                        charges: vec![],
                                    }))
                                } else {
                                    Some(self.finish_block(Terminator::Jump {
                                        target: condition_header,
                                        span: body.span.clone(),
                                        charges: vec![],
                                    }))
                                };
                                let _body_state = self.restore_availability(checkpoint);
                                self.restore_temps(temp_checkpoint);
                                self.dead_depth -= 1;
                                for (id, state) in condition_state {
                                    if id.0 < prior_locals {
                                        self.set_availability(id, state);
                                    }
                                }
                                let exit = BlockId(self.blocks.len());
                                if literal_true {
                                    if let Some(&first_break) = loop_context.breaks.first() {
                                        self.enter_edge(first_break, 0)?;
                                    } else {
                                        self.emission = EmissionMode::Analyze;
                                        self.terminated = Some(Termination::Diverged);
                                    }
                                } else {
                                    self.enter_edge(branch, 1)?;
                                    if let Terminator::BranchBool { no, .. } =
                                        &mut self.blocks[branch.0].terminator
                                    {
                                        *no = exit;
                                    }
                                }
                                for id in loop_context.breaks {
                                    if let Terminator::Jump { target, .. } =
                                        &mut self.blocks[id.0].terminator
                                    {
                                        *target = exit;
                                    }
                                }
                                let _ = (preheader, body_end);
                                completed = Some(Ok(()));
                            }
                        }
                    }
                    Some(Task::WhileAfterSecond { .. }) => {
                        let Some(Task::WhileAfterSecond {
                            checkpoint,
                            temp_checkpoint,
                            locals,
                            scopes,
                            before_errors,
                            affected,
                            body_span,
                            condition_diverges,
                        }) = tasks.pop()
                        else {
                            return Err(CompileError::InvalidIr("while backedge continuation"));
                        };
                        self.restore_availability(checkpoint);
                        self.restore_temps(temp_checkpoint);
                        self.locals.truncate(locals);
                        self.name_spans.truncate(locals);
                        self.next_scope = scopes;
                        self.scope_parents.truncate(scopes);
                        for d in &mut diagnostics[before_errors..] {
                            add_backedge_to_diagnostic(d, &body_span, &affected);
                        }
                        self.dead_depth -= 1;
                        self.terminated = condition_diverges.then_some(Termination::Diverged);
                        completed =
                            Some(result.map_err(|error| {
                                add_backedge_to_error(error, &body_span, &affected)
                            }));
                    }
                    Some(Task::MatchAfterArm { .. }) => {
                        let Some(Task::MatchAfterArm { mut state, arm }) = tasks.pop() else {
                            return Err(CompileError::InvalidIr("match arm continuation"));
                        };
                        completed = match result {
                            Ok(()) => match self.finish_match_arm(&mut state, arm, None) {
                                Ok(()) => {
                                    tasks.push(Task::MatchNext(state));
                                    None
                                }
                                Err(error) => Some(Err(error)),
                            },
                            Err(error) => Some(Err(error)),
                        };
                    }
                    Some(Task::MatchRestore(_)) => {
                        let Some(Task::MatchRestore(snapshot)) = tasks.pop() else {
                            return Err(CompileError::InvalidIr("match restore continuation"));
                        };
                        self.restore_match_snapshot(*snapshot);
                        completed = Some(result);
                    }
                    Some(Task::MatchExprFinish(_)) => {
                        completed_value = Some(
                            result.and(Err(CompileError::InvalidIr("match expression result"))),
                        );
                    }
                    Some(Task::MatchAfterScrutinee { .. }) => {
                        return Err(CompileError::InvalidIr("match scrutinee unit result"));
                    }
                    Some(Task::UserCallAfterArgument { .. }) => {
                        return Err(CompileError::InvalidIr("call argument unit result"));
                    }
                    Some(Task::VariantAfter { .. }) => {
                        return Err(CompileError::InvalidIr("variant unit result"));
                    }
                    Some(Task::UnaryReadAfter { .. }) => {
                        return Err(CompileError::InvalidIr("propagation unit result"));
                    }
                    Some(Task::OperatorUnaryAfter { .. }) => {
                        return Err(CompileError::InvalidIr("unary operation unit result"));
                    }
                    _ => return Err(CompileError::InvalidIr("lower task continuation")),
                }
                continue;
            }
            if !matches!(tasks.last(), Some(Task::Block(_))) {
                let task = tasks.pop().ok_or(CompileError::InvalidIr("lower task"))?;
                match task {
                    Task::StatementEnter { statement, depth } => match statement {
                        Statement::Block(block) => {
                            if self.emission == EmissionMode::Live {
                                self.charge_block_statement(block);
                            }
                            let surviving = self.live_temps.clone();
                            let saved = self.scope;
                            self.scope = ScopeId(self.next_scope);
                            self.next_scope += 1;
                            self.scope_parents.push(Some(saved));
                            self.scopes.push(Vec::new());
                            tasks.push(Task::Block(Box::new(Frame {
                                statements: &block.statements,
                                depth: depth + 1,
                                index: 0,
                                unreachable: None,
                                scoped: Some(Scoped {
                                    block,
                                    surviving,
                                    saved,
                                }),
                                pending: None,
                                value_tail: None,
                            })));
                        }
                        Statement::If {
                            condition,
                            then_block,
                            otherwise,
                            span,
                        } => {
                            if self.emission == EmissionMode::Live {
                                self.charge_control_statement(span);
                            }
                            tasks.push(Task::IfEnter {
                                condition,
                                then_block,
                                otherwise: otherwise.as_deref().map(StatementElse::Statement),
                                span,
                                depth,
                            });
                        }
                        Statement::While {
                            condition,
                            body,
                            span,
                        } => {
                            if self.emission == EmissionMode::Live {
                                self.charge_control_statement(span);
                            }
                            tasks.push(Task::WhileEnter {
                                condition,
                                body,
                                span,
                                depth,
                            });
                        }
                        Statement::Match(selection) => tasks.push(Task::MatchEnter {
                            selection,
                            depth,
                            expected: None,
                        }),
                        Statement::Let { .. }
                        | Statement::Expr { .. }
                        | Statement::Return { value: Some(_), .. } => {
                            tasks.push(Task::StatementExprEnter(statement));
                        }
                        Statement::Assign { .. } => tasks.push(Task::AssignEnter(statement)),
                        Statement::For { .. } => tasks.push(Task::ForEnter { statement, depth }),
                        Statement::GuardControl {
                            token,
                            guarded: false,
                            ..
                        } if token == "yield" => {
                            tasks.push(Task::YieldEnter { statement, depth });
                        }
                        _ => completed = Some(self.statement_leaf(statement)),
                    },
                    Task::YieldEnter { statement, depth } => {
                        let Statement::GuardControl {
                            token,
                            value,
                            token_span,
                            guarded: false,
                            span,
                        } = statement
                        else {
                            return Err(CompileError::InvalidIr("yield task"));
                        };
                        if token != "yield" {
                            return Err(CompileError::InvalidIr("yield task"));
                        }
                        if self.emission == EmissionMode::Live {
                            self.pending_charges.push(WorkCharge {
                                ticks: 1,
                                origin: token_span.clone(),
                            });
                        }
                        if let Some(value) = value {
                            tasks.push(Task::YieldAfterValue { token_span, span });
                            tasks.push(Task::Expr {
                                value,
                                expected: None,
                                depth,
                                borrow: false,
                            });
                        } else {
                            let flow = self.emitted(
                                Op::ConstUnit,
                                Ty::Unit,
                                token_span.clone(),
                                Access::Own,
                                false,
                            );
                            completed = Some(self.finish_yield(flow, token_span, span));
                        }
                    }
                    Task::ForEnter { statement, depth } => {
                        let Statement::For { iterable, span, .. } = statement else {
                            return Err(CompileError::InvalidIr("for statement"));
                        };
                        if self.emission == EmissionMode::Live {
                            self.charge_control_statement(span);
                        }
                        let surviving_temps = self.live_temps.clone();
                        let mut ungrouped = iterable;
                        while let Expr::Group(inner, _) = ungrouped {
                            ungrouped = inner;
                        }
                        if let Expr::Binary {
                            op, left, right, ..
                        } = ungrouped
                            && matches!(op.as_str(), "..<" | "..=")
                        {
                            tasks.push(Task::ForRangeAfterLower {
                                statement,
                                upper: right,
                                depth,
                                inclusive: op == "..=",
                                surviving_temps,
                            });
                            tasks.push(Task::Expr {
                                value: left,
                                expected: Some(Ty::I64),
                                depth,
                                borrow: false,
                            });
                        } else {
                            let array_binding = place_parts(iterable)
                                .ok()
                                .and_then(|(root, _)| place_name(root))
                                .is_some()
                                && matches!(self.resolved_expr(iterable), Ok(Ty::Array(_)));
                            tasks.push(Task::ForAfterIterable {
                                statement,
                                depth,
                                surviving_temps,
                            });
                            tasks.push(Task::Expr {
                                value: iterable,
                                expected: None,
                                depth,
                                borrow: array_binding,
                            });
                        }
                    }
                    Task::AssignEnter(statement) => {
                        let Statement::Assign {
                            place, op, value, ..
                        } = statement
                        else {
                            return Err(CompileError::InvalidIr("assignment task"));
                        };
                        if self.emission == EmissionMode::Live {
                            self.pending_charges.push(WorkPlanner::charge(
                                WorkEvent::Assignment,
                                parse::expression_primary_span(place),
                            ));
                        }
                        let prepared = (|| -> Result<AssignState<'_>, CompileError> {
                            let (root, selectors) = place_parts(place)?;
                            let selector_spans = index_selectors(place);
                            let name = place_name(root)
                                .ok_or(CompileError::InvalidIr("assignment root"))?;
                            let local = self.get(name)?;
                            if self.locals[local.0].parameter_mode == Some(ParameterMode::Read) {
                                return Err(self.permission_error(
                                    local,
                                    root.span().clone(),
                                    "write",
                                ));
                            }
                            if selectors.is_empty() && !self.locals[local.0].mutable {
                                return Err(self.immutable_write(root, local));
                            }
                            let target_ty = self.resolved_expr(place)?;
                            let base_unresolved = matches!(
                                self.inferred_bindings
                                    .expressions
                                    .get(&(root.span().start, root.span().end)),
                                Some(ResolvedType::Unresolved)
                            );
                            Ok(AssignState {
                                statement,
                                place,
                                root,
                                value,
                                op: *op,
                                local,
                                ids: Vec::with_capacity(selectors.len()),
                                selectors,
                                selector_spans,
                                next: 0,
                                target_ty,
                                base_unresolved,
                                diverged: false,
                                compound: None,
                            })
                        })();
                        match prepared {
                            Ok(state) => tasks.push(Task::AssignNextSelector(Box::new(state))),
                            Err(error) => completed = Some(Err(error)),
                        }
                    }
                    Task::AssignNextSelector(mut state) => {
                        if state.next < state.selectors.len() {
                            if state.diverged {
                                self.terminated = None;
                            }
                            if self.emission == EmissionMode::Live {
                                self.pending_charges.push(WorkPlanner::charge(
                                    WorkEvent::PlaceSelector,
                                    state.selector_spans[state.next].clone(),
                                ));
                            }
                            let selector = state.selectors[state.next];
                            let expected = (!state.base_unresolved).then_some(Ty::I64);
                            tasks.push(Task::AssignAfterSelector(state));
                            tasks.push(Task::Expr {
                                value: selector,
                                expected,
                                depth: 0,
                                borrow: false,
                            });
                        } else {
                            if state.diverged {
                                self.terminated = None;
                            }
                            let prepared = (|| -> Result<(), CompileError> {
                                if state.op == parse::AssignOp::Set
                                    || state.diverged
                                    || self.emission == EmissionMode::Analyze
                                {
                                    return Ok(());
                                }
                                if state.target_ty != Ty::I64 {
                                    return Err(CompileError::InvalidIr(
                                        "compound place type passed checker",
                                    ));
                                }
                                let indexed = !state.selectors.is_empty();
                                if !self.locals[state.local.0].mutable
                                    && !(self.defer_ownership
                                        && self.emission == EmissionMode::Live)
                                {
                                    return Err(self.immutable_write(state.root, state.local));
                                }
                                self.check_place_access(
                                    state.local,
                                    state.root,
                                    state.place,
                                    "borrow_inout",
                                    indexed,
                                )?;
                                let loan = LoanId(state.place.span().start as usize);
                                let parent = (self.locals[state.local.0].parameter_mode
                                    == Some(ParameterMode::Inout))
                                .then_some(state.local);
                                let loan_temp = self
                                    .emitted(
                                        Op::BeginLoan {
                                            root: state.local,
                                            indexes: state.ids.clone(),
                                            site: index_site(
                                                state.place,
                                                state.place.span().clone(),
                                            ),
                                            loan,
                                            origin_span: state.place.span().clone(),
                                            permission: LoanPermission::Inout,
                                            parent,
                                        },
                                        Ty::I64,
                                        state.place.span().clone(),
                                        Access::Read,
                                        false,
                                    )
                                    .temp()
                                    .ok_or(CompileError::InvalidIr("compound place loan"))?;
                                self.active_loans.entry(state.local).or_default().push(
                                    ActiveLoan {
                                        permission: LoanPermission::Inout,
                                        span: state.place.span().clone(),
                                        loan,
                                        parent,
                                        compound_assignment: true,
                                    },
                                );
                                let old = self
                                    .emitted(
                                        Op::CompoundLoad { loan: loan_temp },
                                        Ty::I64,
                                        state.place.span().clone(),
                                        Access::Read,
                                        false,
                                    )
                                    .temp()
                                    .ok_or(CompileError::InvalidIr("compound old value"))?;
                                state.compound = Some((loan, loan_temp, old));
                                Ok(())
                            })();
                            if let Err(error) = prepared {
                                completed = Some(Err(error));
                            } else {
                                let value = state.value;
                                let expected = (state.target_ty != Ty::Never)
                                    .then_some(state.target_ty.clone());
                                tasks.push(Task::AssignAfterValue(state));
                                tasks.push(Task::Expr {
                                    value,
                                    expected,
                                    depth: 0,
                                    borrow: false,
                                });
                            }
                        }
                    }
                    Task::ArgumentEnter {
                        arg,
                        mode,
                        expected,
                        depth,
                        retain_place_loan,
                    } => {
                        count_loan_work();
                        if self.emission == EmissionMode::Live
                            && mode == ParameterMode::Inout
                            && let Some(edit) = self.tokens.iter().find(|token| {
                                token.text == "edit"
                                    && token.span.start >= arg.span.start
                                    && token.span.end <= arg.span.end
                            })
                        {
                            self.pending_charges.push(WorkPlanner::charge(
                                WorkEvent::InoutPrefix,
                                edit.span.clone(),
                            ));
                        }
                        if !retain_place_loan {
                            tasks.push(Task::Expr {
                                value: &arg.value,
                                expected: (expected != Ty::Never).then_some(expected),
                                depth: depth + 1,
                                borrow: mode == ParameterMode::Read,
                            });
                            continue;
                        }
                        let place = if mode == ParameterMode::Inout {
                            match &arg.value {
                                Expr::Unary {
                                    mode: Mode::Inout,
                                    value,
                                    ..
                                } => Some(value.as_ref()),
                                _ => {
                                    completed_value = Some(Err(CompileError::InvalidIr(
                                        "edit mode passed checker",
                                    )));
                                    continue;
                                }
                            }
                        } else if mode == ParameterMode::Read {
                            Some(&arg.value)
                        } else {
                            None
                        };
                        let rooted = place
                            .and_then(|place| {
                                place_parts(place)
                                    .ok()
                                    .map(|(root, indexes)| (place, root, indexes))
                            })
                            .and_then(|(place, root, indexes)| {
                                place_name(root).map(|name| (place, root, indexes, name))
                            });
                        if let Some((place, root, indexes, name)) = rooted {
                            if name == "None"
                                && self.visible.get(name).and_then(|v| v.last()).is_none()
                            {
                                tasks.push(Task::Expr {
                                    value: &arg.value,
                                    expected: Some(expected),
                                    depth: depth + 1,
                                    borrow: mode == ParameterMode::Read,
                                });
                                continue;
                            }
                            let local = match self.get(name) {
                                Ok(local) => local,
                                Err(error) => {
                                    completed_value = Some(Err(error));
                                    continue;
                                }
                            };
                            if mode == ParameterMode::Read
                                && matches!(
                                    self.inferred_bindings
                                        .expressions
                                        .get(&(root.span().start, root.span().end)),
                                    Some(ResolvedType::Unresolved)
                                )
                            {
                                tasks.push(Task::Expr {
                                    value: &arg.value,
                                    expected: None,
                                    depth: depth + 1,
                                    borrow: true,
                                });
                                continue;
                            }
                            if mode == ParameterMode::Read && self.emission == EmissionMode::Live {
                                if !indexes.is_empty() {
                                    self.pending_charges.extend(
                                        index_access_spans(&arg.value).into_iter().map(|origin| {
                                            WorkPlanner::charge(WorkEvent::ReadSelector, origin)
                                        }),
                                    );
                                }
                                self.pending_charges.push(WorkPlanner::charge(
                                    WorkEvent::ReadName,
                                    root.span().clone(),
                                ));
                            }
                            if mode == ParameterMode::Inout && self.emission == EmissionMode::Live {
                                self.pending_charges.extend(
                                    index_selectors(place).into_iter().map(|origin| {
                                        WorkPlanner::charge(WorkEvent::PlaceSelector, origin)
                                    }),
                                );
                            }
                            if mode == ParameterMode::Inout {
                                if self.locals[local.0].parameter_mode == Some(ParameterMode::Read)
                                {
                                    completed_value = Some(Err(self.permission_error(
                                        local,
                                        root.span().clone(),
                                        "borrow_inout",
                                    )));
                                    continue;
                                }
                                if !self.locals[local.0].mutable {
                                    completed_value = Some(Err(self.immutable_write(root, local)));
                                    continue;
                                }
                            }
                            match self.resolved_expr(&arg.value) {
                                Err(error) => completed_value = Some(Err(error)),
                                Ok(ty) => tasks.push(Task::ArgumentPlaceNext(Box::new(
                                    ArgumentPlaceState {
                                        arg,
                                        place,
                                        root,
                                        local,
                                        mode,
                                        ty,
                                        depth,
                                        indexes,
                                        ids: Vec::new(),
                                    },
                                ))),
                            }
                        } else if mode == ParameterMode::Inout {
                            completed_value =
                                Some(Err(CompileError::InvalidIr("edit place passed parser")));
                        } else {
                            tasks.push(Task::Expr {
                                value: &arg.value,
                                expected: (expected != Ty::Never).then_some(expected),
                                depth: depth + 1,
                                borrow: mode == ParameterMode::Read,
                            });
                        }
                    }
                    Task::ArgumentPlaceNext(mut state) => {
                        if !state.indexes.is_empty() {
                            let next = state.indexes.remove(0);
                            let depth = state.depth;
                            tasks.push(Task::ArgumentPlaceAfterIndex(state));
                            tasks.push(Task::Expr {
                                value: next,
                                expected: Some(Ty::I64),
                                depth: depth + 1,
                                borrow: true,
                            });
                            continue;
                        }
                        let indexed = !index_selectors(state.place).is_empty();
                        let requested = if state.mode == ParameterMode::Inout {
                            "borrow_inout"
                        } else {
                            "read"
                        };
                        if let Err(error) = self.check_place_access(
                            state.local,
                            state.root,
                            state.place,
                            requested,
                            indexed,
                        ) {
                            completed_value = Some(Err(error));
                            continue;
                        }
                        if self.locals[state.local.0].ty == Ty::Never {
                            completed_value = Some(Ok(Some(Flow::Value {
                                temp: None,
                                ty: state.ty,
                            })));
                            continue;
                        }
                        let permission = if state.mode == ParameterMode::Inout {
                            LoanPermission::Inout
                        } else {
                            LoanPermission::Read
                        };
                        let loan = LoanId(state.arg.span.start as usize);
                        let parent = (self.locals[state.local.0].parameter_mode
                            == Some(ParameterMode::Inout))
                        .then_some(state.local);
                        let flow = self.emitted(
                            Op::BeginLoan {
                                root: state.local,
                                indexes: state.ids,
                                site: index_site(state.place, state.arg.span.clone()),
                                loan,
                                origin_span: state.arg.span.clone(),
                                permission,
                                parent,
                            },
                            state.ty,
                            state.arg.value.span().clone(),
                            Access::Read,
                            false,
                        );
                        self.active_loans
                            .entry(state.local)
                            .or_default()
                            .push(ActiveLoan {
                                permission,
                                span: state.arg.span.clone(),
                                loan,
                                parent,
                                compound_assignment: false,
                            });
                        match tasks.last_mut() {
                            Some(Task::UserCallAfterArgument { loan_root, .. }) => {
                                *loan_root = Some(state.local);
                            }
                            #[cfg(test)]
                            None => {}
                            _ => {
                                return Err(CompileError::InvalidIr("loan call continuation"));
                            }
                        }
                        completed_value = Some(Ok(Some(flow)));
                    }
                    Task::StatementExprEnter(statement) => match statement {
                        Statement::Let {
                            mutable,
                            name,
                            value,
                            ..
                        } => {
                            let prepared = (|| -> Result<(Span, Ty), CompileError> {
                                let binding_span =
                                    self.name_span(statement, name, value.span().start)?;
                                if self.emission == EmissionMode::Live {
                                    let statement_start = statement_span(statement).start;
                                    if let Some(token) = self
                                        .tokens
                                        .iter()
                                        .find(|token| token.span.start == statement_start)
                                    {
                                        self.pending_charges.push(WorkPlanner::charge(
                                            WorkEvent::Binding,
                                            token.span.clone(),
                                        ));
                                    }
                                }
                                let expected = self.resolved_type(
                                    self.inferred_bindings
                                        .bindings
                                        .get(&binding_span.start)
                                        .ok_or(CompileError::InvalidIr(
                                            "missing checker binding type",
                                        ))?,
                                )?;
                                Ok((binding_span, expected))
                            })();
                            match prepared {
                                Err(error) => completed = Some(Err(error)),
                                Ok((binding_span, expected)) => {
                                    tasks.push(Task::StatementExprAfter(Box::new(
                                        StatementExprState::Let {
                                            statement,
                                            mutable: *mutable,
                                            name,
                                            binding_span,
                                            expected: expected.clone(),
                                        },
                                    )));
                                    tasks.push(Task::Expr {
                                        value,
                                        expected: Some(expected),
                                        depth: 0,
                                        borrow: false,
                                    });
                                }
                            }
                        }
                        Statement::Expr { value, .. } => {
                            if self.emission == EmissionMode::Live {
                                self.pending_charges.push(WorkPlanner::charge(
                                    WorkEvent::ExpressionStatement,
                                    parse::expression_primary_span(value),
                                ));
                            }
                            tasks.push(Task::StatementExprAfter(Box::new(
                                StatementExprState::Expr(statement),
                            )));
                            tasks.push(Task::Expr {
                                value,
                                expected: None,
                                depth: 0,
                                borrow: false,
                            });
                        }
                        Statement::Return {
                            value: Some(value),
                            span,
                        } => {
                            if self.emission == EmissionMode::Live {
                                let origin = Span {
                                    file: span.file.clone(),
                                    start: span.start,
                                    end: span
                                        .end
                                        .min(span.start.saturating_add("return".len() as u64)),
                                };
                                self.pending_charges
                                    .push(WorkPlanner::charge(WorkEvent::ReturnStatement, origin));
                            }
                            tasks.push(Task::StatementExprAfter(Box::new(
                                StatementExprState::Return(statement),
                            )));
                            tasks.push(Task::Expr {
                                value,
                                expected: Some(self.result.clone()),
                                depth: 0,
                                borrow: false,
                            });
                        }
                        _ => return Err(CompileError::InvalidIr("statement expression task")),
                    },
                    Task::ValueBlockEnter {
                        block,
                        expected,
                        if_span,
                        depth,
                    } => {
                        let surviving = self.live_temps.clone();
                        let saved = self.scope;
                        self.scope = ScopeId(self.next_scope);
                        self.next_scope += 1;
                        self.scope_parents.push(Some(saved));
                        self.scopes.push(Vec::new());
                        let (statements, value_tail) = match block.statements.last() {
                            Some(Statement::Value {
                                value: Some(value),
                                marker,
                                ..
                            }) => (
                                &block.statements[..block.statements.len() - 1],
                                Some(ValueTail {
                                    value,
                                    marker: marker.as_ref(),
                                    expected,
                                    saved_termination: None,
                                    saved_emission: self.emission,
                                    started: false,
                                    result: None,
                                }),
                            ),
                            _ => (block.statements.as_slice(), None),
                        };
                        tasks.push(Task::ValueBlockAfter(ScopedValue {
                            block,
                            if_span,
                            surviving,
                            saved,
                        }));
                        tasks.push(Task::Block(Box::new(Frame {
                            statements,
                            depth: depth + 1,
                            index: 0,
                            unreachable: None,
                            scoped: None,
                            pending: None,
                            value_tail,
                        })));
                    }
                    Task::Expr {
                        value,
                        expected,
                        depth,
                        borrow,
                    } => {
                        let mut inner = value;
                        let mut inner_depth = depth;
                        while let Expr::Group(next, _) = inner {
                            inner = next;
                            inner_depth += 1;
                        }
                        if let Expr::Binary {
                            op,
                            left,
                            right,
                            span,
                        } = inner
                        {
                            match self.prepare_expr(inner) {
                                Ok(inferred) => {
                                    if matches!(op.as_str(), "and" | "or") {
                                        tasks.push(Task::LogicalLeft(Box::new(LogicalState {
                                            op,
                                            right,
                                            span,
                                            depth: inner_depth,
                                            inferred,
                                            branch_mode: EmissionMode::Analyze,
                                            skip_end: None,
                                            skip_state: HashMap::new(),
                                            checkpoint: 0,
                                            temp_checkpoint: 0,
                                            prior_locals: 0,
                                            slot: None,
                                            left_diverges: false,
                                        })));
                                    } else {
                                        tasks.push(Task::BinaryLeft(Box::new(BinaryState {
                                            op,
                                            left,
                                            right,
                                            span,
                                            inferred,
                                            depth: inner_depth,
                                            left_flow: None,
                                            left_root: None,
                                        })));
                                    }
                                    tasks.push(Task::Expr {
                                        value: left,
                                        expected: matches!(op.as_str(), "and" | "or")
                                            .then_some(Ty::Bool),
                                        depth: inner_depth + 1,
                                        borrow: true,
                                    });
                                }
                                Err(error) => completed_value = Some(Err(error)),
                            }
                        } else if let Expr::Array(items, span) = inner {
                            match self.prepare_expr(inner) {
                                Err(error) => completed_value = Some(Err(error)),
                                Ok(inferred) => tasks.push(Task::ArrayNext(Box::new(ArrayState {
                                    items,
                                    span,
                                    inferred,
                                    depth: inner_depth,
                                    next: 0,
                                    ids: Vec::with_capacity(items.len()),
                                    diverged: false,
                                    item_errors: Vec::new(),
                                    dependent: false,
                                }))),
                            }
                        } else if let Expr::Index {
                            target,
                            index,
                            selector,
                            span,
                        } = inner
                        {
                            match self.prepare_expr(inner) {
                                Err(error) => completed_value = Some(Err(error)),
                                Ok(inferred) => {
                                    let unresolved = matches!(
                                        self.inferred_bindings
                                            .expressions
                                            .get(&(target.span().start, target.span().end)),
                                        Some(ResolvedType::Unresolved)
                                    );
                                    if unresolved && self.emission == EmissionMode::Live {
                                        completed_value = Some(Err(CompileError::InvalidIr(
                                            "live unresolved index base",
                                        )));
                                        continue;
                                    }
                                    let place = if unresolved {
                                        None
                                    } else {
                                        place_parts(inner).ok().and_then(|(root, selectors)| {
                                            place_name(root).map(|name| (root, selectors, name))
                                        })
                                    };
                                    if let Some((root, selectors, name)) = place {
                                        match self.get(name) {
                                            Err(error) => completed_value = Some(Err(error)),
                                            Ok(local) => {
                                                if self.emission == EmissionMode::Live {
                                                    self.pending_charges.extend(
                                                        index_access_spans(inner)
                                                            .into_iter()
                                                            .skip(1)
                                                            .map(|origin| {
                                                                WorkPlanner::charge(
                                                                    WorkEvent::IndexExpression,
                                                                    origin,
                                                                )
                                                            }),
                                                    );
                                                    self.pending_charges.push(WorkPlanner::charge(
                                                        WorkEvent::ReadName,
                                                        root.span().clone(),
                                                    ));
                                                }
                                                tasks.push(Task::IndexNext(Box::new(IndexState {
                                                    expr: inner,
                                                    index,
                                                    selector,
                                                    span,
                                                    inferred,
                                                    depth: inner_depth,
                                                    borrow,
                                                    kind: IndexKind::Place {
                                                        root,
                                                        local,
                                                        ids: Vec::with_capacity(selectors.len()),
                                                        selectors,
                                                        diverged: false,
                                                    },
                                                })));
                                            }
                                        }
                                    } else {
                                        let kind = if unresolved {
                                            IndexKind::Unresolved
                                        } else {
                                            IndexKind::Value { base: None }
                                        };
                                        tasks.push(Task::IndexAfterBase(Box::new(IndexState {
                                            expr: inner,
                                            index,
                                            selector,
                                            span,
                                            inferred,
                                            depth: inner_depth,
                                            borrow,
                                            kind,
                                        })));
                                        tasks.push(Task::Expr {
                                            value: target,
                                            expected: None,
                                            depth: inner_depth + 1,
                                            borrow: false,
                                        });
                                    }
                                }
                            }
                        } else if let Expr::If {
                            condition,
                            then_block,
                            otherwise,
                            span,
                        } = inner
                        {
                            match self.prepare_expr(inner) {
                                Ok(inferred) => tasks.push(Task::IfExprEnter {
                                    condition,
                                    then_block,
                                    otherwise: otherwise.as_deref(),
                                    span,
                                    depth: inner_depth,
                                    expected: Some(inferred),
                                }),
                                Err(error) => completed_value = Some(Err(error)),
                            }
                        } else if let Expr::Unary {
                            mode: Mode::Inout | Mode::Consume,
                            value,
                            span,
                        } = inner
                            && matches!(value.as_ref(), Expr::Call { .. })
                        {
                            match self.prepare_expr(inner) {
                                Err(error) => completed_value = Some(Err(error)),
                                Ok(_) => {
                                    let mode = if matches!(
                                        inner,
                                        Expr::Unary {
                                            mode: Mode::Inout,
                                            ..
                                        }
                                    ) {
                                        ParameterMode::Inout
                                    } else {
                                        ParameterMode::Consume
                                    };
                                    self.requested_receiver = Some((mode, span.clone()));
                                    tasks.push(Task::Expr {
                                        value,
                                        expected,
                                        depth: inner_depth + 1,
                                        borrow: false,
                                    });
                                }
                            }
                        } else if let Expr::Neg { value, span } | Expr::Not { value, span } = inner
                        {
                            match self.prepare_expr(inner) {
                                Ok(inferred) => {
                                    let op = if matches!(inner, Expr::Neg { .. }) {
                                        Unary::Neg
                                    } else {
                                        Unary::Not
                                    };
                                    tasks.push(Task::OperatorUnaryAfter { span, inferred, op });
                                    tasks.push(Task::Expr {
                                        value,
                                        expected: Some(if op == Unary::Neg {
                                            Ty::I64
                                        } else {
                                            Ty::Bool
                                        }),
                                        depth: inner_depth + 1,
                                        borrow: false,
                                    });
                                }
                                Err(error) => completed_value = Some(Err(error)),
                            }
                        } else if let Expr::Unary {
                            mode: Mode::Read,
                            value,
                            span,
                        } = inner
                        {
                            match self.prepare_expr(inner) {
                                Ok(inferred) => tasks.push(Task::UnaryReadEnter {
                                    value,
                                    span,
                                    depth: inner_depth,
                                    inferred,
                                }),
                                Err(error) => completed_value = Some(Err(error)),
                            }
                        } else if let Expr::Call { target, args, span } = inner
                            && place_name(target) == Some("Iterator.Next")
                            && self
                                .visible
                                .get("Iterator.Next")
                                .and_then(|ids| ids.last())
                                .is_none()
                        {
                            match self.prepare_expr(inner) {
                                Ok(inferred) => tasks.push(Task::GeneratorNextEnter {
                                    args,
                                    span,
                                    depth: inner_depth,
                                    inferred,
                                }),
                                Err(error) => completed_value = Some(Err(error)),
                            }
                        } else if let Expr::Call { target, args, span } = inner
                            && let Some(name) = place_name(target)
                            && self.visible.get(name).and_then(|ids| ids.last()).is_none()
                            && self.user_functions.contains_key(name)
                        {
                            match self.prepare_expr(inner) {
                                Ok(inferred) => tasks.push(Task::UserCallEnter {
                                    args,
                                    name: name.to_owned(),
                                    span,
                                    depth: inner_depth,
                                    inferred,
                                }),
                                Err(error) => completed_value = Some(Err(error)),
                            }
                        } else if let Expr::Call { target, args, span } = inner
                            && let Some(name @ ("Some" | "Ok" | "Err")) = place_name(target)
                        {
                            match self.prepare_expr(inner) {
                                Ok(inferred) => {
                                    let tag = inferred
                                        .union_descriptor()
                                        .and_then(|union| union.kind.variant_tag(name));
                                    match (tag, args.as_slice()) {
                                        (Some(tag), [arg]) => tasks.push(Task::VariantEnter {
                                            value: &arg.value,
                                            tag,
                                            span,
                                            depth: inner_depth,
                                            inferred,
                                        }),
                                        (Some(_), _) => {
                                            completed_value = Some(Err(CompileError::InvalidIr(
                                                "variant constructor arity",
                                            )));
                                        }
                                        (None, _) => {
                                            completed_value = Some(Err(CompileError::InvalidIr(
                                                "variant constructor type",
                                            )));
                                        }
                                    }
                                }
                                Err(error) => completed_value = Some(Err(error)),
                            }
                        } else if let Expr::Call { target, args, span } = inner
                            && let Some(name) = place_name(target)
                            && self.visible.get(name).and_then(|ids| ids.last()).is_none()
                            && !self.user_functions.contains_key(name)
                            && let Some(intrinsic) = intrinsic_for_name(name)
                            && !(intrinsic == Intrinsic::DropData
                                && args.len() == 1
                                && place_name(&args[0].value).is_some())
                        {
                            match self.prepare_expr(inner) {
                                Ok(inferred) => tasks.push(Task::IntrinsicCallEnter {
                                    args,
                                    intrinsic,
                                    span,
                                    depth: inner_depth,
                                    inferred,
                                }),
                                Err(error) => completed_value = Some(Err(error)),
                            }
                        } else if let Expr::Call { target, args, span } = inner {
                            match self.prepare_expr(inner) {
                                Ok(inferred) => tasks.push(Task::OtherCallEnter {
                                    target,
                                    args,
                                    span,
                                    depth: inner_depth,
                                    inferred,
                                }),
                                Err(error) => completed_value = Some(Err(error)),
                            }
                        } else if let Expr::Match(selection) = inner {
                            match self.prepare_expr(inner) {
                                Ok(inferred) => {
                                    let outer_diagnostics = std::mem::take(diagnostics);
                                    tasks.push(Task::MatchExprFinish(outer_diagnostics));
                                    tasks.push(Task::MatchEnter {
                                        selection,
                                        depth: inner_depth,
                                        expected: Some(inferred),
                                    });
                                }
                                Err(error) => completed_value = Some(Err(error)),
                            }
                        } else {
                            completed_value = Some(
                                self.expr_mode_non_group(
                                    inner,
                                    inner_depth,
                                    expected.as_ref(),
                                    borrow,
                                )
                                .map(Some),
                            );
                        }
                    }
                    Task::IfExprEnter {
                        condition,
                        then_block,
                        otherwise,
                        span,
                        depth,
                        expected,
                    } => {
                        if otherwise.is_none() {
                            let primary = Span {
                                file: span.file.clone(),
                                start: span.start,
                                end: span.start + 2,
                            };
                            completed_value = Some(Err(CompileError::Rejected(vec![
                                crate::check::names::diagnostic(
                                    "NX-E1302",
                                    Some("missing_else"),
                                    diagnostic::Stage::Parse,
                                    diagnostic::Operation::SyntaxParse,
                                    primary,
                                ),
                            ])));
                        } else {
                            tasks.push(Task::IfExprAfterCondition {
                                then_block,
                                otherwise,
                                span,
                                depth,
                                expected,
                            });
                            tasks.push(Task::Expr {
                                value: condition,
                                expected: Some(Ty::Bool),
                                depth: depth + 1,
                                borrow: true,
                            });
                        }
                    }
                    Task::IfExprElse(mut state) => {
                        self.dead_depth += 1;
                        let expected = Some(state.inferred_ty.clone());
                        let depth = state.depth;
                        let if_span = state.span;
                        let otherwise = state.otherwise;
                        let outer_diagnostics = std::mem::take(diagnostics);
                        *diagnostics = std::mem::take(&mut state.diagnostics);
                        tasks.push(Task::IfExprAfterElse(state, outer_diagnostics));
                        match otherwise {
                            parse::Else::Block(block) => tasks.push(Task::ValueBlockEnter {
                                block,
                                expected,
                                if_span,
                                depth,
                            }),
                            parse::Else::If(expr) => tasks.push(Task::Expr {
                                value: expr,
                                expected,
                                depth: depth + 1,
                                borrow: false,
                            }),
                        }
                    }
                    Task::MatchEnter {
                        selection,
                        depth,
                        expected,
                    } => {
                        if self.emission != EmissionMode::Live {
                            tasks.push(Task::MatchRestore(Box::new(self.capture_match_snapshot())));
                        }
                        let rooted = if matches!(selection.mode, None | Some(Mode::Inout)) {
                            parse::place_parts(&selection.scrutinee)
                                .and_then(|(root, indexes)| {
                                    place_name(root).map(|name| (root, indexes, name))
                                })
                                .filter(|(_, _, name)| {
                                    *name != "None"
                                        || self
                                            .visible
                                            .get(*name)
                                            .and_then(|ids| ids.last())
                                            .is_some()
                                })
                        } else {
                            None
                        };
                        if let Some((_, indexes, name)) = rooted {
                            let prepared = (|| -> Result<MatchIndexState<'_>, CompileError> {
                                let prelude = self.match_selection_prelude(selection)?;
                                let local = self.get(name)?;
                                let origin = parse::expression_primary_span(&selection.scrutinee);
                                if selection.mode == Some(Mode::Inout) {
                                    self.check_available(local, origin.clone())?;
                                    if self.locals[local.0].parameter_mode
                                        == Some(ParameterMode::Read)
                                    {
                                        return Err(self.permission_error(
                                            local,
                                            origin.clone(),
                                            "borrow_inout",
                                        ));
                                    }
                                    if !self.locals[local.0].mutable {
                                        return Err(
                                            self.immutable_write(&selection.scrutinee, local)
                                        );
                                    }
                                    self.conflict(local, origin, "borrow_inout")?;
                                } else {
                                    self.check_available(local, origin.clone())?;
                                    self.conflict(local, origin, "read")?;
                                }
                                let selected = Flow::Value {
                                    temp: None,
                                    ty: prelude.selected_ty.clone(),
                                };
                                Ok(MatchIndexState {
                                    selection,
                                    depth,
                                    expected: expected.clone(),
                                    prelude,
                                    selected,
                                    selectors: indexes,
                                    ids: Vec::new(),
                                })
                            })();
                            match prepared {
                                Ok(state) => tasks.push(Task::MatchIndexNext(Box::new(state))),
                                Err(error) => {
                                    if expected.is_some() {
                                        completed_value = Some(Err(error));
                                    } else {
                                        completed = Some(Err(error));
                                    }
                                }
                            }
                            continue;
                        }
                        if self.match_needs_scrutinee_expr(selection) {
                            match self.match_selection_prelude(selection) {
                                Ok(prelude) => {
                                    let selected_ty = prelude.selected_ty.clone();
                                    tasks.push(Task::MatchAfterScrutinee {
                                        selection,
                                        depth,
                                        expected,
                                        prelude,
                                    });
                                    tasks.push(Task::Expr {
                                        value: &selection.scrutinee,
                                        expected: Some(selected_ty),
                                        depth: depth + 1,
                                        borrow: false,
                                    });
                                }
                                Err(error) => {
                                    if expected.is_some() {
                                        completed_value = Some(Err(error));
                                    } else {
                                        completed = Some(Err(error));
                                    }
                                }
                            }
                            continue;
                        }
                        match self.begin_match_selection(
                            selection,
                            expected.is_some(),
                            depth,
                            expected.as_ref(),
                        ) {
                            Ok(lower_match::MatchStart::Completed(flow)) => {
                                if expected.is_some() {
                                    completed_value = Some(Ok(Some(flow)));
                                } else {
                                    completed = Some(Ok(()));
                                }
                            }
                            Ok(lower_match::MatchStart::Active(state)) => {
                                tasks.push(Task::MatchNext(state));
                            }
                            Err(error) => {
                                if expected.is_some() {
                                    completed_value = Some(Err(error));
                                } else {
                                    completed = Some(Err(error));
                                }
                            }
                        }
                    }
                    Task::MatchIndexNext(mut state) => {
                        if !state.selectors.is_empty() {
                            let selector = state.selectors.remove(0);
                            let depth = state.depth;
                            tasks.push(Task::MatchIndexAfterSelector(state));
                            tasks.push(Task::Expr {
                                value: selector,
                                expected: Some(Ty::I64),
                                depth: depth + 1,
                                borrow: true,
                            });
                            continue;
                        }
                        let expression = state.expected.is_some();
                        match self.begin_match_selection_after_selected(
                            state.selection,
                            expression,
                            state.depth,
                            state.expected.as_ref(),
                            state.prelude,
                            state.selected,
                            state.ids,
                            true,
                        ) {
                            Ok(lower_match::MatchStart::Completed(flow)) => {
                                if expression {
                                    completed_value = Some(Ok(Some(flow)));
                                } else {
                                    completed = Some(Ok(()));
                                }
                            }
                            Ok(lower_match::MatchStart::Active(next)) => {
                                tasks.push(Task::MatchNext(next));
                            }
                            Err(error) => {
                                if expression {
                                    completed_value = Some(Err(error));
                                } else {
                                    completed = Some(Err(error));
                                }
                            }
                        }
                    }
                    Task::MatchNext(mut state) => {
                        match self.begin_match_arm(&mut state, diagnostics) {
                            Ok(lower_match::MatchArmStart::Done(Some(arm))) => {
                                tasks.push(Task::MatchArmReady { state, arm });
                            }
                            Ok(lower_match::MatchArmStart::Guard { arm, guard, saved }) => {
                                let depth = state.depth();
                                tasks.push(Task::MatchGuardAfter { state, arm, saved });
                                tasks.push(Task::Expr {
                                    value: guard,
                                    expected: Some(Ty::Bool),
                                    depth: depth + 1,
                                    borrow: false,
                                });
                            }
                            Ok(lower_match::MatchArmStart::Done(None)) => {
                                let expression = state.is_expression();
                                let result = self.finish_match_selection(state);
                                if expression {
                                    completed_value = Some(result.map(Some));
                                } else {
                                    completed = Some(result.map(|_| ()));
                                }
                            }
                            Err(error) => {
                                if state.is_expression() {
                                    completed_value = Some(Err(error));
                                } else {
                                    completed = Some(Err(error));
                                }
                            }
                        }
                    }
                    Task::MatchArmReady { state, arm } => {
                        let block = arm.body();
                        let depth = state.depth();
                        let expression = state.is_expression();
                        let expected = state.expected();
                        let if_span = &state.selection().keyword_span;
                        if expression {
                            tasks.push(Task::MatchAfterArm { state, arm });
                            tasks.push(Task::ValueBlockEnter {
                                block,
                                expected,
                                if_span,
                                depth: depth + 1,
                            });
                            continue;
                        }
                        let surviving = self.live_temps.clone();
                        let saved = self.scope;
                        self.scope = ScopeId(self.next_scope);
                        self.next_scope += 1;
                        self.scope_parents.push(Some(saved));
                        self.scopes.push(Vec::new());
                        tasks.push(Task::MatchAfterArm { state, arm });
                        tasks.push(Task::Block(Box::new(Frame {
                            statements: &block.statements,
                            depth: depth + 1,
                            index: 0,
                            unreachable: None,
                            scoped: Some(Scoped {
                                block,
                                surviving,
                                saved,
                            }),
                            pending: None,
                            value_tail: None,
                        })));
                    }
                    Task::ArrayNext(state) => {
                        if state.next == state.items.len() {
                            completed_value = Some(if !state.item_errors.is_empty() {
                                Err(CompileError::Rejected(
                                    state.item_errors.into_iter().map(Box::new).collect(),
                                ))
                            } else if state.dependent {
                                Err(CompileError::Dependent)
                            } else if state.diverged {
                                self.terminated = Some(Termination::Diverged);
                                Ok(Some(Flow::Diverged {
                                    ty: (state.inferred != Ty::Never).then_some(state.inferred),
                                }))
                            } else if state.inferred == Ty::Never {
                                if self.emission == EmissionMode::Live {
                                    Err(CompileError::InvalidIr("unresolved checker array type"))
                                } else {
                                    Ok(Some(Flow::Value {
                                        temp: None,
                                        ty: state.inferred,
                                    }))
                                }
                            } else {
                                Ok(Some(self.emitted(
                                    Op::BuildArray(state.ids),
                                    state.inferred,
                                    state.span.clone(),
                                    Access::Own,
                                    false,
                                )))
                            });
                        } else {
                            if state.diverged {
                                self.terminated = None;
                            }
                            let item = &state.items[state.next];
                            let depth = state.depth;
                            tasks.push(Task::ArrayAfterItem(state));
                            tasks.push(Task::Expr {
                                value: item,
                                expected: None,
                                depth: depth + 1,
                                borrow: false,
                            });
                        }
                    }
                    Task::IndexNext(mut state) => {
                        let IndexKind::Place {
                            root,
                            local,
                            selectors,
                            ids,
                            diverged,
                        } = &mut state.kind
                        else {
                            return Err(CompileError::InvalidIr("index place state"));
                        };
                        if !selectors.is_empty() {
                            if *diverged {
                                self.terminated = None;
                                self.emission = EmissionMode::Analyze;
                            }
                            let next = selectors.remove(0);
                            let depth = state.depth;
                            tasks.push(Task::IndexAfterSelector(state));
                            tasks.push(Task::Expr {
                                value: next,
                                expected: Some(Ty::I64),
                                depth: depth + 1,
                                borrow: true,
                            });
                            continue;
                        }
                        if *diverged {
                            self.terminated = Some(Termination::Diverged);
                            completed_value = Some(Ok(Some(Flow::Diverged {
                                ty: (state.inferred != Ty::Never).then_some(state.inferred),
                            })));
                        } else if state.inferred == Ty::Never {
                            completed_value = Some(Ok(Some(Flow::Value {
                                temp: None,
                                ty: state.inferred,
                            })));
                        } else {
                            let operation = if state.borrow {
                                Op::BorrowIndex {
                                    local: *local,
                                    indexes: std::mem::take(ids),
                                    site: index_site(state.expr, state.span.clone()),
                                }
                            } else {
                                Op::ReadIndex {
                                    local: *local,
                                    indexes: std::mem::take(ids),
                                    site: index_site(state.expr, state.span.clone()),
                                }
                            };
                            let _ = root;
                            completed_value = Some(Ok(Some(self.emitted(
                                operation,
                                state.inferred,
                                state.span.clone(),
                                Access::Read,
                                false,
                            ))));
                        }
                    }
                    Task::OtherCallEnter {
                        target,
                        args,
                        span,
                        depth,
                        inferred,
                    } => {
                        let (mut receiver_mode, receiver_span) = self
                            .requested_receiver
                            .take()
                            .unwrap_or((ParameterMode::Read, target.span().clone()));
                        let target_ty = if let Some(name) = place_name(target) {
                            if self.visible.get(name).and_then(|ids| ids.last()).is_some() {
                                self.resolved_expr(target).map(Some)
                            } else {
                                Ok(None)
                            }
                        } else {
                            self.resolved_expr(target).map(Some)
                        };
                        let target_ty = match target_ty {
                            Ok(value) => value,
                            Err(error) => {
                                completed_value = Some(Err(error));
                                continue;
                            }
                        };
                        if let Some(Ty::Closure { site, .. }) = target_ty {
                            let required = match self.inferred_bindings.closures.get(&site) {
                                Some(info) => match info.required_mode {
                                    Mode::Read => ParameterMode::Read,
                                    Mode::Inout => ParameterMode::Inout,
                                    Mode::Consume => ParameterMode::Consume,
                                },
                                None => {
                                    completed_value = Some(Err(CompileError::InvalidIr(
                                        "closure call signature",
                                    )));
                                    continue;
                                }
                            };
                            let mode_rank = |mode| match mode {
                                ParameterMode::Read => 0,
                                ParameterMode::Inout => 1,
                                ParameterMode::Consume => 2,
                            };
                            let mode_name = |mode| match mode {
                                ParameterMode::Read => "read",
                                ParameterMode::Inout => "inout",
                                ParameterMode::Consume => "consume",
                            };
                            if mode_rank(receiver_mode) < mode_rank(required) {
                                let mut error = crate::check::names::diagnostic(
                                    "NX-E1107",
                                    None,
                                    diagnostic::Stage::Ownership,
                                    diagnostic::Operation::OwnershipCheck,
                                    parse::expression_primary_span(target),
                                );
                                if let Some(name) = place_name(target)
                                    && let Ok(local) = self.get(name)
                                {
                                    error.related_spans.push(diagnostic::RelatedSpan {
                                        role: diagnostic::RelatedRole::Declaration,
                                        span: self.name_spans[local.0].clone(),
                                    });
                                }
                                error.data.insert(
                                    "required_mode".into(),
                                    diagnostic::DataValue::String(mode_name(required).into()),
                                );
                                error.data.insert(
                                    "requested_mode".into(),
                                    diagnostic::DataValue::String(mode_name(receiver_mode).into()),
                                );
                                if let Some(local) =
                                    place_name(target).and_then(|name| self.get(name).ok())
                                {
                                    self.mode_candidates.push((
                                        local,
                                        target.span().clone(),
                                        *error,
                                    ));
                                    receiver_mode = ParameterMode::Read;
                                } else {
                                    completed_value =
                                        Some(Err(CompileError::Rejected(vec![error])));
                                    continue;
                                }
                            }
                            let state = ClosureCallState {
                                args,
                                span,
                                depth,
                                inferred,
                                site,
                                receiver_mode,
                            };
                            if place_name(target).is_some() {
                                match named_closure_receiver(
                                    self,
                                    target,
                                    receiver_mode,
                                    &receiver_span,
                                ) {
                                    Err(error) => {
                                        if let CompileError::Rejected(errors) = &error
                                            && errors.iter().any(|d| d.code == "NX-E0202")
                                            && let Some(local) = place_name(target)
                                                .and_then(|name| self.get(name).ok())
                                        {
                                            self.mode_candidates.retain(|(id, span, _)| {
                                                *id != local || span != target.span()
                                            });
                                        }
                                        completed_value = Some(Err(error));
                                    }
                                    Ok((receiver, receiver_loan)) if receiver.diverged() => {
                                        let _ = receiver_loan;
                                        completed_value = Some(Ok(Some(Flow::Diverged {
                                            ty: Some(state.inferred),
                                        })));
                                    }
                                    Ok((receiver, receiver_loan)) => {
                                        match closure_call_state(
                                            self,
                                            state,
                                            receiver,
                                            receiver_loan,
                                        ) {
                                            Ok(call) => {
                                                tasks.push(Task::UserCallNext(Box::new(call)))
                                            }
                                            Err(error) => completed_value = Some(Err(error)),
                                        }
                                    }
                                }
                            } else if receiver_mode != ParameterMode::Read {
                                completed_value = Some(Err(CompileError::InvalidIr(
                                    "non-place closure receiver",
                                )));
                            } else {
                                tasks.push(Task::ClosureAfterReceiver(Box::new(state)));
                                tasks.push(Task::Expr {
                                    value: target,
                                    expected: None,
                                    depth: depth + 1,
                                    borrow: false,
                                });
                            }
                            continue;
                        }
                        let Some(name) = place_name(target) else {
                            completed_value = Some(Err(CompileError::InvalidIr("call target")));
                            continue;
                        };
                        if self.visible.get(name).and_then(|ids| ids.last()).is_some() {
                            completed_value = Some(match self.get(name) {
                                Err(error) => Err(error),
                                Ok(_) => Err(CompileError::InvalidIr(
                                    "non-callable target passed checker",
                                )),
                            });
                            continue;
                        }
                        if name == "Drop"
                            && args.len() == 1
                            && let Some(binding) = place_name(&args[0].value)
                        {
                            let dropped = (|| -> Result<Flow, CompileError> {
                                let local = self.get(binding)?;
                                let primary = parse::expression_primary_span(&args[0].value);
                                self.check_available(local, primary.clone())?;
                                if self.locals[local.0]
                                    .parameter_mode
                                    .is_some_and(|mode| mode != ParameterMode::Consume)
                                {
                                    return Err(self.permission_error(local, primary, "drop"));
                                }
                                self.conflict(local, primary, "drop")?;
                                self.mark_consumed(local, Some((span.clone(), true)));
                                let value = self.emitted(
                                    Op::TransferValue(local),
                                    self.locals[local.0].ty.clone(),
                                    span.clone(),
                                    Access::Own,
                                    false,
                                );
                                let Some(value) = value.temp() else {
                                    return Ok(Flow::Diverged { ty: Some(inferred) });
                                };
                                Ok(self.emitted(
                                    Op::CallIntrinsic {
                                        intrinsic: Intrinsic::DropData,
                                        args: vec![value],
                                    },
                                    inferred,
                                    span.clone(),
                                    Access::Own,
                                    false,
                                ))
                            })();
                            completed_value = Some(dropped.map(Some));
                        } else {
                            completed_value =
                                Some(Err(CompileError::InvalidIr("user or deferred call")));
                        }
                    }
                    Task::UserCallEnter {
                        args,
                        name,
                        span,
                        depth,
                        inferred,
                    } => {
                        self.requested_receiver.take();
                        let Some((params, _)) = self.user_functions.get(&name).cloned() else {
                            completed_value =
                                Some(Err(CompileError::InvalidIr("user call signature")));
                            continue;
                        };
                        if args.len() != params.len() {
                            completed_value =
                                Some(Err(CompileError::InvalidIr("call arity passed checker")));
                            continue;
                        }
                        let slot_by_name = params
                            .iter()
                            .enumerate()
                            .map(|(index, (label, _, _))| (label.clone(), index))
                            .collect();
                        tasks.push(Task::UserCallNext(Box::new(UserCallState {
                            args,
                            kind: CallKind::User(name),
                            span,
                            depth,
                            inferred,
                            used: vec![false; params.len()],
                            slot_by_name,
                            params,
                            check_types: true,
                            retain_place_loans: true,
                            next_positional: 0,
                            index: 0,
                            prepared: Vec::with_capacity(args.len()),
                            acquired: Vec::new(),
                            argument_errors: Vec::new(),
                            dependent: false,
                            diverged: false,
                        })));
                    }
                    Task::IntrinsicCallEnter {
                        args,
                        intrinsic,
                        span,
                        depth,
                        inferred,
                    } => {
                        self.requested_receiver.take();
                        let params = intrinsic_parameters(intrinsic);
                        let slot_by_name = params
                            .iter()
                            .enumerate()
                            .map(|(index, (label, _, _))| (label.clone(), index))
                            .collect();
                        tasks.push(Task::UserCallNext(Box::new(UserCallState {
                            args,
                            kind: CallKind::Intrinsic(intrinsic),
                            span,
                            depth,
                            inferred,
                            used: vec![false; params.len()],
                            slot_by_name,
                            params,
                            check_types: false,
                            retain_place_loans: args.len() > 1,
                            next_positional: 0,
                            index: 0,
                            prepared: Vec::with_capacity(args.len()),
                            acquired: Vec::new(),
                            argument_errors: Vec::new(),
                            dependent: false,
                            diverged: false,
                        })));
                    }
                    Task::GeneratorNextEnter {
                        args,
                        span,
                        depth,
                        inferred,
                    } => {
                        self.requested_receiver.take();
                        let prepared = (|| -> Result<(Ty, CallParameters), CompileError> {
                            let argument = args
                                .first()
                                .ok_or(CompileError::InvalidIr("next argument"))?;
                            let generator = self.resolved_expr(&argument.value)?;
                            let Ty::Generator { item, .. } = &generator else {
                                return Err(CompileError::InvalidIr("next receiver type"));
                            };
                            Ok((
                                item.as_ref().clone(),
                                vec![("g".into(), ParameterMode::Inout, generator)],
                            ))
                        })();
                        match prepared {
                            Err(error) => completed_value = Some(Err(error)),
                            Ok((item, params)) => {
                                let slot_by_name = params
                                    .iter()
                                    .enumerate()
                                    .map(|(index, (label, _, _))| (label.clone(), index))
                                    .collect();
                                tasks.push(Task::UserCallNext(Box::new(UserCallState {
                                    args,
                                    kind: CallKind::GeneratorNext { item },
                                    span,
                                    depth,
                                    inferred,
                                    used: vec![false; params.len()],
                                    slot_by_name,
                                    params,
                                    check_types: true,
                                    retain_place_loans: true,
                                    next_positional: 0,
                                    index: 0,
                                    prepared: Vec::with_capacity(args.len()),
                                    acquired: Vec::new(),
                                    argument_errors: Vec::new(),
                                    dependent: false,
                                    diverged: false,
                                })));
                            }
                        }
                    }
                    Task::VariantEnter {
                        value,
                        tag,
                        span,
                        depth,
                        inferred,
                    } => {
                        tasks.push(Task::VariantAfter {
                            tag,
                            span,
                            inferred,
                        });
                        tasks.push(Task::Expr {
                            value,
                            expected: None,
                            depth: depth + 1,
                            borrow: false,
                        });
                    }
                    Task::UnaryReadEnter {
                        value,
                        span,
                        depth,
                        inferred,
                    } => {
                        tasks.push(Task::UnaryReadAfter { span, inferred });
                        tasks.push(Task::Expr {
                            value,
                            expected: None,
                            depth: depth + 1,
                            borrow: false,
                        });
                    }
                    Task::UserCallNext(mut state) => {
                        if state.index == state.args.len() {
                            self.release_call_loans(&state.acquired);
                            if !state.argument_errors.is_empty() {
                                completed_value = Some(Err(CompileError::Rejected(
                                    state.argument_errors.into_iter().map(Box::new).collect(),
                                )));
                            } else if state.dependent {
                                completed_value = Some(Err(CompileError::Dependent));
                            } else if state.diverged {
                                self.terminated = Some(Termination::Diverged);
                                completed_value = Some(Ok(Some(Flow::Diverged {
                                    ty: match state.kind {
                                        CallKind::User(_)
                                        | CallKind::GeneratorNext { .. }
                                        | CallKind::Closure { .. } => Some(state.inferred),
                                        CallKind::Intrinsic(_) => None,
                                    },
                                })));
                            } else {
                                match state.kind {
                                    CallKind::User(name) => {
                                        if self.emission == EmissionMode::Analyze {
                                            completed_value = Some(Ok(Some(Flow::Value {
                                                temp: None,
                                                ty: state.inferred,
                                            })));
                                            continue;
                                        }
                                        let operation =
                                            if matches!(state.inferred, Ty::Generator { .. }) {
                                                Op::BuildGenerator {
                                                    function: FunctionId(name),
                                                    args: state.prepared,
                                                    signature: state.params,
                                                }
                                            } else {
                                                Op::CallDirect {
                                                    function: FunctionId(name),
                                                    args: state.prepared,
                                                    signature: state.params,
                                                }
                                            };
                                        let flow = self.emitted(
                                            operation,
                                            state.inferred,
                                            state.span.clone(),
                                            Access::Own,
                                            false,
                                        );
                                        self.end_call_loans(state.acquired);
                                        completed_value = Some(Ok(Some(flow)));
                                    }
                                    CallKind::Intrinsic(intrinsic) => {
                                        if state.args.len() != intrinsic_arity(intrinsic) {
                                            completed_value = Some(Err(CompileError::InvalidIr(
                                                "intrinsic arity",
                                            )));
                                            continue;
                                        }
                                        let ids =
                                            state.prepared.iter().map(|(_, id)| *id).collect();
                                        let flow = self.emitted(
                                            Op::CallIntrinsic {
                                                intrinsic,
                                                args: ids,
                                            },
                                            state.inferred,
                                            state.span.clone(),
                                            Access::Own,
                                            intrinsic_effect_io(intrinsic),
                                        );
                                        self.end_call_loans(state.acquired);
                                        self.drop_owned_read_arguments(
                                            &state.prepared,
                                            &state.params,
                                        );
                                        completed_value = Some(Ok(Some(flow)));
                                    }
                                    CallKind::GeneratorNext { item } => {
                                        if self.emission == EmissionMode::Analyze {
                                            completed_value = Some(Ok(Some(Flow::Value {
                                                temp: None,
                                                ty: state.inferred,
                                            })));
                                            continue;
                                        }
                                        let Some((_, receiver)) = state.prepared.first() else {
                                            completed_value = Some(Err(CompileError::InvalidIr(
                                                "next receiver flow",
                                            )));
                                            continue;
                                        };
                                        let flow = self.emitted(
                                            Op::ResumeGenerator {
                                                receiver: *receiver,
                                                item,
                                            },
                                            state.inferred,
                                            state.span.clone(),
                                            Access::Own,
                                            false,
                                        );
                                        self.end_call_loans(state.acquired);
                                        completed_value = Some(Ok(Some(flow)));
                                    }
                                    CallKind::Closure {
                                        site,
                                        receiver,
                                        receiver_mode,
                                        capture_count,
                                        signature,
                                        receiver_loan,
                                    } => {
                                        if self.emission == EmissionMode::Analyze {
                                            if let Some((root, loan, origin)) = receiver_loan {
                                                self.release_call_loans(&[(
                                                    root,
                                                    loan,
                                                    origin.clone(),
                                                )]);
                                                self.end_call_loans(vec![(root, loan, origin)]);
                                            }
                                            completed_value = Some(Ok(Some(Flow::Value {
                                                temp: None,
                                                ty: state.inferred,
                                            })));
                                            continue;
                                        }
                                        let Some(receiver) = receiver.temp() else {
                                            completed_value = Some(Err(CompileError::InvalidIr(
                                                "closure receiver flow",
                                            )));
                                            continue;
                                        };
                                        let flow = self.emitted(
                                            Op::CallClosure {
                                                site,
                                                receiver,
                                                mode: receiver_mode,
                                                capture_count,
                                                args: state.prepared,
                                                signature,
                                            },
                                            state.inferred,
                                            state.span.clone(),
                                            Access::Own,
                                            false,
                                        );
                                        self.end_call_loans(state.acquired);
                                        if let Some((root, loan, origin)) = receiver_loan {
                                            self.release_call_loans(&[(
                                                root,
                                                loan,
                                                origin.clone(),
                                            )]);
                                            self.end_call_loans(vec![(root, loan, origin)]);
                                        }
                                        completed_value = Some(Ok(Some(flow)));
                                    }
                                }
                            }
                            continue;
                        }
                        if state.diverged {
                            self.terminated = None;
                        }
                        count_loan_work();
                        let arg = &state.args[state.index];
                        let slot = if let Some(label) = &arg.label {
                            match state.slot_by_name.get(label).copied() {
                                Some(slot) => slot,
                                None => {
                                    self.release_call_loans(&state.acquired);
                                    completed_value = Some(Err(CompileError::InvalidIr(
                                        "call label passed checker",
                                    )));
                                    continue;
                                }
                            }
                        } else {
                            while state.used.get(state.next_positional) == Some(&true) {
                                state.next_positional += 1;
                            }
                            state.next_positional
                        };
                        let Some((_, mode, expected)) = state.params.get(slot) else {
                            self.release_call_loans(&state.acquired);
                            completed_value =
                                Some(Err(CompileError::InvalidIr("call parameter slot")));
                            continue;
                        };
                        if state.used[slot] {
                            self.release_call_loans(&state.acquired);
                            completed_value =
                                Some(Err(CompileError::InvalidIr("duplicate call argument")));
                            continue;
                        }
                        state.used[slot] = true;
                        let mode = *mode;
                        let expected = expected.clone();
                        state.index += 1;
                        let depth = state.depth;
                        let retain_place_loan = state.retain_place_loans;
                        tasks.push(Task::UserCallAfterArgument {
                            state,
                            slot,
                            loan_root: None,
                        });
                        tasks.push(Task::ArgumentEnter {
                            arg,
                            mode,
                            expected,
                            depth,
                            retain_place_loan,
                        });
                    }
                    Task::WhileEnter {
                        condition,
                        body,
                        span,
                        depth,
                    } => {
                        let entry_emission = self.emission;
                        let surviving_temps = self.live_temps.clone();
                        let prior_locals = self.locals.len();
                        let preheader = self.finish_block(Terminator::Jump {
                            target: BlockId(self.blocks.len() + 1),
                            span: span.clone(),
                            charges: vec![],
                        });
                        let condition_header = BlockId(self.blocks.len());
                        let checkpoint = self.consumed_undo.len();
                        let temp_checkpoint = self.temp_undo.len();
                        self.dead_depth += 1;
                        if entry_emission == EmissionMode::Live {
                            self.pending_charges.push(WorkPlanner::charge(
                                WorkEvent::WhileCondition,
                                parse::expression_primary_span(condition),
                            ));
                        }
                        tasks.push(Task::WhileAfterCondition(Box::new(WhileConditionState {
                            condition,
                            body,
                            depth,
                            entry_emission,
                            prior_locals,
                            preheader,
                            condition_header,
                            checkpoint,
                            temp_checkpoint,
                            surviving_temps,
                        })));
                        tasks.push(Task::Expr {
                            value: condition,
                            expected: Some(Ty::Bool),
                            depth: 0,
                            borrow: true,
                        });
                    }
                    Task::IfEnter {
                        condition,
                        then_block,
                        otherwise,
                        span,
                        depth,
                    } => {
                        tasks.push(Task::IfAfterCondition {
                            then_block,
                            otherwise,
                            span,
                            depth,
                        });
                        tasks.push(Task::Expr {
                            value: condition,
                            expected: Some(Ty::Bool),
                            depth: 0,
                            borrow: true,
                        });
                    }
                    Task::IfElse {
                        state,
                        otherwise,
                        depth,
                    } => {
                        let Some(otherwise) = otherwise else {
                            completed = Some(self.finish_if_else(state, None));
                            continue;
                        };
                        self.dead_depth += 1;
                        let else_span = match otherwise {
                            StatementElse::Statement(stmt) => statement_span(stmt),
                            StatementElse::Else(parse::Else::Block(block)) => block.span.clone(),
                            StatementElse::Else(parse::Else::If(expr)) => expr.span().clone(),
                        };
                        tasks.push(Task::IfAfterElse {
                            state,
                            else_span: Some(else_span),
                        });
                        match otherwise {
                            StatementElse::Statement(Statement::If {
                                condition,
                                then_block,
                                otherwise,
                                span,
                            }) => {
                                if self.emission == EmissionMode::Live {
                                    self.charge_control_statement(span);
                                }
                                tasks.push(Task::IfEnter {
                                    condition,
                                    then_block,
                                    otherwise: otherwise.as_deref().map(StatementElse::Statement),
                                    span,
                                    depth: depth + 1,
                                });
                            }
                            StatementElse::Statement(Statement::Expr {
                                value:
                                    Expr::If {
                                        condition,
                                        then_block,
                                        otherwise,
                                        span,
                                    },
                                ..
                            }) => tasks.push(Task::IfEnter {
                                condition,
                                then_block,
                                otherwise: otherwise.as_deref().map(StatementElse::Else),
                                span,
                                depth: depth + 1,
                            }),
                            StatementElse::Else(parse::Else::If(expr)) => {
                                let Expr::If {
                                    condition,
                                    then_block,
                                    otherwise,
                                    span,
                                } = expr.as_ref()
                                else {
                                    return Err(CompileError::InvalidIr("else if form"));
                                };
                                tasks.push(Task::IfEnter {
                                    condition,
                                    then_block,
                                    otherwise: otherwise.as_deref().map(StatementElse::Else),
                                    span,
                                    depth: depth + 1,
                                });
                            }
                            StatementElse::Statement(Statement::Block(block))
                            | StatementElse::Else(parse::Else::Block(block)) => {
                                let surviving = self.live_temps.clone();
                                let saved = self.scope;
                                self.scope = ScopeId(self.next_scope);
                                self.next_scope += 1;
                                self.scope_parents.push(Some(saved));
                                self.scopes.push(Vec::new());
                                tasks.push(Task::Block(Box::new(Frame {
                                    statements: &block.statements,
                                    depth: depth + 2,
                                    index: 0,
                                    unreachable: None,
                                    scoped: Some(Scoped {
                                        block,
                                        surviving,
                                        saved,
                                    }),
                                    pending: None,
                                    value_tail: None,
                                })));
                            }
                            StatementElse::Statement(stmt) => {
                                tasks.push(Task::StatementEnter {
                                    statement: stmt,
                                    depth: depth + 1,
                                });
                            }
                        }
                    }
                    Task::Block(_)
                    | Task::StatementExprAfter(_)
                    | Task::ForAfterIterable { .. }
                    | Task::ForRangeAfterLower { .. }
                    | Task::ForRangeAfterUpper { .. }
                    | Task::ForAfterBody { .. }
                    | Task::YieldAfterValue { .. }
                    | Task::AssignAfterSelector(_)
                    | Task::AssignAfterValue(_)
                    | Task::ArgumentPlaceAfterIndex(_)
                    | Task::ClosureAfterReceiver(_)
                    | Task::IfExprAfterCondition { .. }
                    | Task::IfAfterCondition { .. }
                    | Task::WhileAfterCondition(_)
                    | Task::WhileAfterSecondCondition(_)
                    | Task::MatchIndexAfterSelector(_)
                    | Task::MatchGuardAfter { .. }
                    | Task::BinaryLeft(_)
                    | Task::BinaryRight(_)
                    | Task::LogicalLeft(_)
                    | Task::LogicalRight(_)
                    | Task::ArrayAfterItem(_)
                    | Task::IndexAfterSelector(_)
                    | Task::IndexAfterBase(_)
                    | Task::IndexAfterValueSelector(_)
                    | Task::ValueBlockAfter(_)
                    | Task::IfExprAfterThen(..)
                    | Task::IfExprAfterElse(..)
                    | Task::MatchAfterArm { .. }
                    | Task::MatchExprFinish(_)
                    | Task::MatchAfterScrutinee { .. }
                    | Task::UserCallAfterArgument { .. }
                    | Task::VariantAfter { .. }
                    | Task::UnaryReadAfter { .. }
                    | Task::OperatorUnaryAfter { .. }
                    | Task::MatchRestore(_)
                    | Task::IfAfterThen { .. }
                    | Task::IfAfterElse { .. }
                    | Task::WhileAfterBody { .. }
                    | Task::WhileAfterSecond { .. } => {
                        return Err(CompileError::InvalidIr("lower task state"));
                    }
                }
                continue;
            }
            let frame = match tasks.last_mut() {
                Some(Task::Block(frame)) => frame,
                Some(_) => return Err(CompileError::InvalidIr("lower task state")),
                None => return Err(CompileError::InvalidIr("block continuation")),
            };
            if frame.index == frame.statements.len() {
                if let Some(value_tail) = frame.value_tail.as_mut() {
                    if !value_tail.started {
                        value_tail.started = true;
                        value_tail.saved_termination = self.terminated.take();
                        value_tail.saved_emission = self.emission;
                        if value_tail.saved_termination.is_some() {
                            self.emission = EmissionMode::Analyze;
                        }
                        if self.emission == EmissionMode::Live {
                            self.pending_charges.push(WorkPlanner::charge(
                                WorkEvent::ValueStatement,
                                value_tail.marker.cloned().unwrap_or_else(|| {
                                    parse::expression_primary_span(value_tail.value)
                                }),
                            ));
                        }
                        let value = value_tail.value;
                        let expected = value_tail.expected.clone();
                        tasks.push(Task::Expr {
                            value,
                            expected,
                            depth: 0,
                            borrow: false,
                        });
                        continue;
                    }
                    if value_tail.result.is_none() {
                        return Err(CompileError::InvalidIr("value block waiting"));
                    }
                }
                let Some(Task::Block(frame)) = tasks.pop() else {
                    return Err(CompileError::InvalidIr("block continuation"));
                };
                let value_result = frame.value_tail.and_then(|tail| tail.result);
                let root = tasks.is_empty();
                let tail_result = if root {
                    Some(tail.take().ok_or(CompileError::InvalidIr("block tail"))?(
                        self,
                        self.terminated.is_some(),
                        None,
                    ))
                } else {
                    None
                };
                if let Some(saved) = frame.unreachable {
                    while self.consumed_undo.len() > saved.undo {
                        let (id, old, maybe) = self
                            .consumed_undo
                            .pop()
                            .ok_or(CompileError::InvalidIr("availability undo"))?;
                        if let Some(old) = old {
                            self.consumed_at.insert(id, old);
                        } else {
                            self.consumed_at.remove(&id);
                        }
                        if let Some(maybe) = maybe {
                            self.maybe_consumed_at.insert(id, maybe);
                        } else {
                            self.maybe_consumed_at.remove(&id);
                        }
                    }
                    let scope = self
                        .scopes
                        .last_mut()
                        .ok_or(CompileError::InvalidIr("scope stack"))?;
                    if saved.scope_len > scope.len() {
                        return Err(CompileError::InvalidIr("scope restore length"));
                    }
                    for name in scope.drain(saved.scope_len..).rev() {
                        self.visible
                            .get_mut(&name)
                            .ok_or(CompileError::InvalidIr("scope name"))?
                            .pop();
                    }
                    for id in saved.locals..self.locals.len() {
                        self.poisoned.remove(&LocalId(id));
                    }
                    self.locals.truncate(saved.locals);
                    self.name_spans.truncate(saved.locals);
                    self.instructions = saved.instructions;
                    self.pending_charges = saved.pending_charges;
                    self.blocks.truncate(saved.blocks);
                    self.temp_blocks.truncate(saved.blocks);
                    self.next_scope = saved.next_scope;
                    self.scope_parents.truncate(saved.next_scope);
                    self.next_temp = saved.next_temp;
                    self.terminated = saved.terminated;
                    self.emission = saved.emission;
                    self.restore_temps(saved.temp_undo);
                    self.temp_block_entry = saved.temp_block_entry;
                    self.dead_depth -= 1;
                    if self.dead_depth == 0 {
                        self.consumed_undo.clear();
                    }
                }
                if let Some(scoped) = frame.scoped {
                    completed = Some(self.finish_scoped_block(
                        scoped.block,
                        scoped.surviving,
                        scoped.saved,
                    ));
                }
                if matches!(tasks.last(), Some(Task::ValueBlockAfter(_))) {
                    completed_value = Some(match value_result {
                        Some(value) => value.map(Some),
                        None => Ok(None),
                    });
                }
                if let Some(result) = tail_result {
                    return result;
                }
                continue;
            }
            super::count_cfg_work(1);
            if self.terminated.is_some() && frame.unreachable.is_none() {
                frame.unreachable = Some(Saved {
                    instructions: std::mem::take(&mut self.instructions),
                    pending_charges: std::mem::take(&mut self.pending_charges),
                    blocks: self.blocks.len(),
                    locals: self.locals.len(),
                    scope_len: self
                        .scopes
                        .last()
                        .ok_or(CompileError::InvalidIr("scope stack"))?
                        .len(),
                    next_scope: self.next_scope,
                    next_temp: self.next_temp,
                    terminated: self.terminated,
                    emission: self.emission,
                    undo: self.consumed_undo.len(),
                    temp_undo: self.temp_undo.len(),
                    temp_block_entry: self.temp_block_entry.clone(),
                });
                self.dead_depth += 1;
                self.emission = EmissionMode::Analyze;
            }
            if frame.unreachable.is_some() {
                self.terminated = None;
            }
            let pending = Pending {
                prior_dead_depth: self.dead_depth,
                prior_terminated: self.terminated,
                prior_emission: self.emission,
                temp_checkpoint: self.temp_undo.len(),
                before_statement_errors: diagnostics.len(),
            };
            let statement = &frame.statements[frame.index];
            let depth = frame.depth;
            frame.pending = Some(pending);
            tasks.push(Task::StatementEnter { statement, depth });
        }
    }
    fn charge_control_statement(&mut self, span: &Span) {
        let origin = self
            .tokens
            .iter()
            .find(|token| token.span.start == span.start)
            .map_or_else(|| span.clone(), |token| token.span.clone());
        self.pending_charges
            .push(WorkPlanner::charge(WorkEvent::IfStatement, origin));
    }
    fn charge_block_statement(&mut self, block: &parse::Block) {
        self.pending_charges.push(WorkPlanner::charge(
            WorkEvent::Block,
            Span {
                file: block.span.file.clone(),
                start: block.span.start,
                end: (block.span.start + 1).min(block.span.end),
            },
        ));
    }
    fn statement_leaf(&mut self, statement: &Statement) -> Result<(), CompileError> {
        match statement {
            Statement::Return { value: None, span } => {
                let origin = Span {
                    file: span.file.clone(),
                    start: span.start,
                    end: span
                        .end
                        .min(span.start.saturating_add("return".len() as u64)),
                };
                if self.emission == EmissionMode::Live {
                    self.pending_charges.push(WorkPlanner::charge(
                        WorkEvent::ReturnStatement,
                        origin.clone(),
                    ));
                }
                let flow = self.emitted(Op::ConstUnit, Ty::Unit, origin, Access::Own, false);
                if flow.diverged() || self.emission == EmissionMode::Analyze {
                    self.terminated = Some(Termination::Diverged);
                    return Ok(());
                }
                let temp = flow
                    .temp()
                    .ok_or(CompileError::InvalidIr("return value flow"))?;
                if flow.ty() != Some(&self.result) && self.dead_depth == 0 {
                    return Err(CompileError::InvalidIr("return type passed checker"));
                }
                self.terminated = Some(Termination::Return(temp));
                Ok(())
            }
            Statement::Value {
                value,
                marker,
                span,
            } => {
                if self.emission == EmissionMode::Live {
                    if let Some(value) = value {
                        self.pending_charges.push(WorkPlanner::charge(
                            WorkEvent::ValueStatement,
                            marker
                                .clone()
                                .unwrap_or_else(|| parse::expression_primary_span(value)),
                        ));
                    } else {
                        self.pending_charges
                            .push(WorkPlanner::charge(WorkEvent::EmptyValue, span.clone()));
                    }
                }
                Err(CompileError::InvalidIr("control flow"))
            }
            Statement::GuardControl {
                token,
                token_span,
                guarded: false,
                ..
            } if matches!(token.as_str(), "break" | "continue") => {
                if self.emission == EmissionMode::Live {
                    self.pending_charges.push(WorkCharge {
                        ticks: 1,
                        origin: token_span.clone(),
                    });
                }
                self.loop_control(token, token_span)
            }
            Statement::GuardControl { token_span, .. } => Err(CompileError::Rejected(vec![
                crate::check::names::diagnostic(
                    "NX-E1305",
                    Some("guard_control"),
                    diagnostic::Stage::Ownership,
                    diagnostic::Operation::OwnershipCheck,
                    token_span.clone(),
                ),
            ])),
            _ => Err(CompileError::InvalidIr("statement task")),
        }
    }
    fn finish_yield(
        &mut self,
        flow: Flow,
        token_span: &Span,
        span: &Span,
    ) -> Result<(), CompileError> {
        if flow.diverged() || self.emission == EmissionMode::Analyze {
            return Ok(());
        }
        let incompatible = self
            .retained_locals
            .iter()
            .flat_map(|(holder, loans)| loans.iter().map(move |loan| (*holder, loan)))
            .filter(|(_, loan)| loan.active.permission == LoanPermission::Inout)
            .min_by_key(|(holder, loan)| (holder.0, loan.active.loan.0));
        if let Some((holder, loan)) = incompatible {
            let mut error = crate::check::names::diagnostic(
                "NX-E1204",
                None,
                diagnostic::Stage::Ownership,
                diagnostic::Operation::OwnershipCheck,
                token_span.clone(),
            );
            error.data.insert(
                "property_evidence".into(),
                diagnostic::DataValue::String("incompatible".into()),
            );
            error.data.insert(
                "suspension_kind".into(),
                diagnostic::DataValue::String("yield".into()),
            );
            error.related_spans.push(diagnostic::RelatedSpan {
                role: diagnostic::RelatedRole::Declaration,
                span: self.locals[holder.0].name_span.clone(),
            });
            error.related_spans.push(diagnostic::RelatedSpan {
                role: diagnostic::RelatedRole::Origin,
                span: loan.active.span.clone(),
            });
            error.cause_chain.push(diagnostic::Cause {
                kind: diagnostic::CauseKind::Effect,
                operation: diagnostic::CauseOperation::SuspendSafe,
                span: Some(token_span.clone()),
                entity: Some(self.local_entity(holder)),
                evidence: diagnostic::Evidence::Verified,
            });
            error.cause_chain.push(diagnostic::Cause {
                kind: diagnostic::CauseKind::Borrow,
                operation: diagnostic::CauseOperation::BorrowInout,
                span: Some(loan.active.span.clone()),
                entity: Some(diagnostic::Entity {
                    kind: diagnostic::EntityKind::Loan,
                    id: format!("loan:{}:0", loan.active.loan.0),
                }),
                evidence: diagnostic::Evidence::Verified,
            });
            return Err(CompileError::Rejected(vec![error]));
        }
        if let Some((root, loan)) = self
            .active_loans
            .iter()
            .flat_map(|(root, loans)| loans.iter().map(move |loan| (*root, loan)))
            .filter(|(_, loan)| loan.permission == LoanPermission::Inout)
            .min_by_key(|(root, loan)| (root.0, loan.loan.0))
        {
            let mut error = crate::check::names::diagnostic(
                "NX-E1204",
                None,
                diagnostic::Stage::Ownership,
                diagnostic::Operation::OwnershipCheck,
                token_span.clone(),
            );
            error.data.insert(
                "property_evidence".into(),
                diagnostic::DataValue::String("incompatible".into()),
            );
            error.data.insert(
                "suspension_kind".into(),
                diagnostic::DataValue::String("yield".into()),
            );
            error.related_spans.push(diagnostic::RelatedSpan {
                role: diagnostic::RelatedRole::Declaration,
                span: self.locals[root.0].name_span.clone(),
            });
            error.related_spans.push(diagnostic::RelatedSpan {
                role: diagnostic::RelatedRole::Origin,
                span: loan.span.clone(),
            });
            error.cause_chain.push(diagnostic::Cause::for_operation(
                diagnostic::CauseOperation::SuspendSafe,
                Some(token_span.clone()),
                Some(self.local_entity(root)),
                diagnostic::Evidence::Verified,
            ));
            error.cause_chain.push(diagnostic::Cause::for_operation(
                diagnostic::CauseOperation::BorrowInout,
                Some(loan.span.clone()),
                Some(diagnostic::Entity {
                    kind: diagnostic::EntityKind::Loan,
                    id: format!("loan:{}:0", loan.loan.0),
                }),
                diagnostic::Evidence::Verified,
            ));
            return Err(CompileError::Rejected(vec![error]));
        }
        let item = flow
            .temp()
            .ok_or(CompileError::InvalidIr("yield item flow"))?;
        let resume = BlockId(self.blocks.len() + 1);
        self.finish_block(Terminator::Yield {
            value: item,
            resume,
            span: span.clone(),
            charges: vec![],
        });
        Ok(())
    }
    #[cfg(test)]
    fn statement(
        &mut self,
        statement: &Statement,
        depth: usize,
        diagnostics: &mut Vec<Diagnostic>,
    ) -> Result<(), CompileError> {
        self.run_lower_tasks(
            LowerRoot::Statement(statement, depth),
            diagnostics,
            |_, _, _| Ok(()),
        )
    }
    #[cfg(test)]
    fn expr(
        &mut self,
        value: &Expr,
        depth: usize,
        expected: Option<&Ty>,
    ) -> Result<Flow, CompileError> {
        self.run_lower_tasks(
            LowerRoot::Expr(value, depth, expected.cloned()),
            &mut Vec::new(),
            |_, _, flow| flow.ok_or(CompileError::InvalidIr("expression flow")),
        )
    }
    #[cfg(test)]
    fn call_argument(
        &mut self,
        arg: &parse::Argument,
        mode: ParameterMode,
        expected: &Ty,
        depth: usize,
        retain_place_loan: bool,
    ) -> Result<(Flow, Option<LocalId>), CompileError> {
        let flow = self.run_lower_tasks(
            LowerRoot::Argument(arg, mode, expected.clone(), depth, retain_place_loan),
            &mut Vec::new(),
            |_, _, flow| flow.ok_or(CompileError::InvalidIr("argument flow")),
        )?;
        let root = self.active_loans.iter().find_map(|(root, loans)| {
            loans
                .iter()
                .any(|loan| loan.loan == LoanId(arg.span.start as usize))
                .then_some(*root)
        });
        Ok((flow, root))
    }
    fn prepare_expr(&mut self, e: &Expr) -> Result<Ty, CompileError> {
        if self.emission == EmissionMode::Live {
            let mut own = Vec::new();
            match e {
                Expr::Name(_, span)
                | Expr::Integer(_, span)
                | Expr::Bool(_, span)
                | Expr::Unit(span) => own.push(span.clone()),
                Expr::Array(_, span)
                | Expr::Binary { span, .. }
                | Expr::Index { span, .. }
                | Expr::If { span, .. } => own.push(span.clone()),
                Expr::Call { target, span, .. } => {
                    own.push(span.clone());
                    let callee = parse::expression_primary(target);
                    if matches!(callee, Expr::Name(..)) {
                        own.push(callee.span().clone());
                    }
                }
                Expr::Unary {
                    mode: Mode::Consume | Mode::Inout,
                    span,
                    ..
                } => {
                    let token = self
                        .tokens
                        .iter()
                        .find(|token| token.span.start == span.start)
                        .map_or_else(|| span.clone(), |token| token.span.clone());
                    own.push(token);
                }
                Expr::Unary {
                    mode: Mode::Read,
                    value,
                    span,
                } => {
                    if span.end > value.span().end {
                        own.push(span.clone());
                    }
                }
                Expr::Group(..) => {}
                Expr::Neg { span, .. } | Expr::Not { span, .. } => own.push(span.clone()),
                Expr::Match(selection) => own.push(selection.span.clone()),
                Expr::Closure(closure) => own.push(closure.fn_span.clone()),
            }
            let event = if matches!(e, Expr::Binary { .. } | Expr::Neg { .. } | Expr::Not { .. }) {
                WorkEvent::OperatorExpression
            } else {
                WorkEvent::IfExpression
            };
            self.pending_charges.extend(
                own.into_iter()
                    .map(|origin| WorkPlanner::charge(event, origin)),
            );
        }
        // This threaded AST depth is not
        // read by lowering or serialized into IR. The checker owns the
        // source-depth limit before lowering starts, so changing one caller's
        // depth increment alone cannot change an accepted program.
        self.resolved_expr(e)
    }
    fn expr_mode_non_group(
        &mut self,
        e: &Expr,
        _depth: usize,
        _expected: Option<&Ty>,
        borrow: bool,
    ) -> Result<Flow, CompileError> {
        let inferred = self.prepare_expr(e)?;
        match e {
            Expr::Closure(closure) => {
                let info = self
                    .inferred_bindings
                    .closures
                    .get(&closure.fn_span.start)
                    .ok_or(CompileError::InvalidIr("closure signature missing"))?
                    .clone();
                let mut fields = Vec::with_capacity(info.captures.len());
                let mut signature = Vec::with_capacity(info.captures.len());
                for capture in &info.captures {
                    let id = self.get(&capture.name)?;
                    self.check_available(id, capture.capture_span.clone())?;
                    let ty = lower_type(&capture.ty)?;
                    let capture_name = self
                        .tokens
                        .iter()
                        .find(|token| {
                            token.span.start >= capture.capture_span.start
                                && token.span.end <= capture.capture_span.end
                                && token.text == capture.name
                        })
                        .map(|token| token.span.clone())
                        .unwrap_or_else(|| capture.capture_span.clone());
                    let op = match capture.mode {
                        parse::CaptureMode::Copy => Op::CopyValue(id),
                        parse::CaptureMode::Move => {
                            if self.locals[id.0]
                                .parameter_mode
                                .is_some_and(|mode| mode != ParameterMode::Consume)
                            {
                                return Err(self.permission_error(
                                    id,
                                    capture_name.clone(),
                                    "consume",
                                ));
                            }
                            self.conflict(id, capture.capture_span.clone(), "consume")?;
                            self.mark_consumed(id, Some((capture.capture_span.clone(), true)));
                            Op::TransferValue(id)
                        }
                        parse::CaptureMode::View | parse::CaptureMode::Edit => {
                            if capture.mode == parse::CaptureMode::Edit {
                                if self.locals[id.0].parameter_mode == Some(ParameterMode::Read) {
                                    return Err(self.permission_error(
                                        id,
                                        capture_name.clone(),
                                        "borrow_inout",
                                    ));
                                }
                                if !self.locals[id.0].mutable {
                                    return Err(self.immutable_write(
                                        &Expr::Name(capture.name.clone(), capture_name.clone()),
                                        id,
                                    ));
                                }
                            }
                            let permission = if capture.mode == parse::CaptureMode::View {
                                LoanPermission::Read
                            } else {
                                LoanPermission::Inout
                            };
                            self.conflict(
                                id,
                                capture.capture_span.clone(),
                                if permission == LoanPermission::Read {
                                    "borrow_read"
                                } else {
                                    "borrow_inout"
                                },
                            )?;
                            Op::BeginLoan {
                                root: id,
                                indexes: Vec::new(),
                                site: IndexSite {
                                    context: capture.capture_span.clone(),
                                    selectors: Vec::new(),
                                },
                                loan: LoanId(capture.capture_span.start as usize),
                                origin_span: capture.capture_span.clone(),
                                permission,
                                parent: (self.locals[id.0].parameter_mode
                                    == Some(ParameterMode::Inout))
                                .then_some(id),
                            }
                        }
                    };
                    let flow = self.emitted(
                        op.clone(),
                        ty.clone(),
                        capture.capture_span.clone(),
                        if matches!(op, Op::CopyValue(_) | Op::BeginLoan { .. }) {
                            Access::Read
                        } else {
                            Access::Own
                        },
                        false,
                    );
                    if matches!(
                        capture.mode,
                        parse::CaptureMode::View | parse::CaptureMode::Edit
                    ) && self.emission == EmissionMode::Live
                        && let Some(last) = self.instructions.last_mut()
                    {
                        last.charges.push(WorkCharge {
                            ticks: 1,
                            origin: capture.capture_span.clone(),
                        });
                    }
                    let Some(temp) = flow.temp() else {
                        return Ok(Flow::Diverged { ty: Some(inferred) });
                    };
                    if let Op::BeginLoan {
                        loan,
                        permission,
                        parent,
                        ..
                    } = op
                    {
                        let active = ActiveLoan {
                            permission,
                            span: capture.capture_span.clone(),
                            loan,
                            parent,
                            compound_assignment: false,
                        };
                        self.active_loans
                            .entry(id)
                            .or_default()
                            .push(active.clone());
                        self.retained_temps.insert(
                            temp,
                            vec![RetainedLoan {
                                root: id,
                                active,
                                source: capture.source.clone(),
                            }],
                        );
                    }
                    fields.push((
                        match capture.mode {
                            parse::CaptureMode::Copy => CaptureKind::Copy,
                            parse::CaptureMode::Move => CaptureKind::Move,
                            parse::CaptureMode::View => CaptureKind::View,
                            parse::CaptureMode::Edit => CaptureKind::Edit,
                        },
                        temp,
                    ));
                    signature.push(ty);
                }
                Ok(self.emitted(
                    Op::BuildClosure {
                        site: info.site,
                        fields,
                        signature,
                    },
                    inferred,
                    closure.span.clone(),
                    Access::Own,
                    false,
                ))
            }
            Expr::Integer(n, s) => {
                let n = n
                    .parse::<i64>()
                    .map_err(|_| CompileError::InvalidIr("unvalidated integer"))?;
                Ok(self.emitted(Op::ConstI64(n), inferred, s.clone(), Access::Own, false))
            }
            Expr::Bool(v, s) => {
                Ok(self.emitted(Op::ConstBool(*v), inferred, s.clone(), Access::Own, false))
            }
            Expr::Unit(s) => {
                Ok(self.emitted(Op::ConstUnit, inferred, s.clone(), Access::Own, false))
            }
            Expr::Name(name, s) => {
                if name == "None" && self.visible.get(name).and_then(|v| v.last()).is_none() {
                    return Ok(self.emitted(
                        Op::BuildVariant {
                            tag: crate::union::VariantTag(0),
                            payload: None,
                        },
                        inferred,
                        s.clone(),
                        Access::Own,
                        false,
                    ));
                }
                let id = self.get(name)?;
                self.check_available(id, s.clone())?;
                if self.locals[id.0].ty == Ty::Never {
                    // A binding produced by a diverging initializer is read
                    // only in hypothetical flow. Reading it does not make a
                    // containing branch diverge a second time.
                    return Ok(Flow::Value {
                        temp: None,
                        ty: inferred,
                    });
                }
                self.conflict(id, s.clone(), "read")?;
                let ty = inferred;
                if !borrow && !ty.copyable() {
                    return Err(self.copy_error(id, s.clone()));
                }
                let op = if borrow {
                    Op::BorrowValue(id)
                } else {
                    Op::CopyValue(id)
                };
                Ok(self.emitted(op, ty, s.clone(), Access::Read, false))
            }
            Expr::Unary {
                mode: Mode::Consume,
                value,
                span,
            } => {
                let Some(name) = place_name(value) else {
                    return Err(CompileError::InvalidIr("partial transfer"));
                };
                let id = self.get(name)?;
                let primary = parse::expression_primary_span(value);
                self.check_available(id, primary.clone())?;
                if self.locals[id.0]
                    .parameter_mode
                    .is_some_and(|m| m != ParameterMode::Consume)
                {
                    return Err(self.permission_error(id, primary, "consume"));
                }
                self.conflict(id, primary, "consume")?;
                self.mark_consumed(id, Some((span.clone(), false)));
                let ty = inferred;
                if ty == Ty::Never {
                    return Ok(Flow::Value { temp: None, ty });
                }
                Ok(self.emitted(Op::TransferValue(id), ty, span.clone(), Access::Own, false))
            }
            Expr::Unary { .. } => Err(CompileError::InvalidIr("exclusive loan expression")),
            _ => Err(CompileError::InvalidIr("expression task dispatch")),
        }
    }
}
fn place_name(expr: &Expr) -> Option<&str> {
    let mut current = expr;
    while let Expr::Group(inner, _) = current {
        current = inner;
    }
    if let Expr::Name(name, _) = current {
        Some(name)
    } else {
        None
    }
}
fn place_parts(expr: &Expr) -> Result<(&Expr, Vec<&Expr>), CompileError> {
    parse::place_parts(expr).ok_or(CompileError::InvalidIr("nonlocal place"))
}
fn index_selectors(expr: &Expr) -> Vec<Span> {
    let mut selectors = Vec::new();
    let mut current = expr;
    loop {
        match current {
            Expr::Group(inner, _) => current = inner,
            Expr::Index {
                target, selector, ..
            } => {
                selectors.push(selector.clone());
                current = target;
            }
            _ => break,
        }
    }
    selectors.reverse();
    selectors
}
fn index_access_spans(expr: &Expr) -> Vec<Span> {
    let mut spans = Vec::new();
    let mut current = expr;
    loop {
        match current {
            Expr::Index { target, span, .. } => {
                spans.push(span.clone());
                current = target;
            }
            Expr::Group(inner, _) => current = inner,
            _ => break,
        }
    }
    spans
}
fn index_site(expr: &Expr, context: Span) -> IndexSite {
    IndexSite {
        context,
        selectors: index_selectors(expr),
    }
}
pub(super) fn clone_ty_iterative(ty: &Ty) -> Ty {
    enum Frame<'a> {
        Enter(&'a Ty),
        Array,
        Union(crate::union::UnionKind, bool),
        Generator(u64),
    }
    let mut frames = vec![Frame::Enter(ty)];
    let mut values = Vec::new();
    while let Some(frame) = frames.pop() {
        match frame {
            Frame::Enter(ty) => match ty {
                Ty::Never => values.push(Ty::Never),
                Ty::I64 => values.push(Ty::I64),
                Ty::Bool => values.push(Ty::Bool),
                Ty::Unit => values.push(Ty::Unit),
                Ty::MockResource => values.push(Ty::MockResource),
                Ty::MockBlockingResource => values.push(Ty::MockBlockingResource),
                Ty::Closure {
                    site,
                    copyable,
                    owns_resource,
                } => values.push(Ty::Closure {
                    site: *site,
                    copyable: *copyable,
                    owns_resource: *owns_resource,
                }),
                Ty::Array(inner) => {
                    frames.push(Frame::Array);
                    frames.push(Frame::Enter(inner));
                }
                Ty::Union {
                    kind,
                    first,
                    second,
                } => {
                    frames.push(Frame::Union(*kind, second.is_some()));
                    if let Some(second) = second {
                        frames.push(Frame::Enter(second));
                    }
                    frames.push(Frame::Enter(first));
                }
                Ty::Generator { site, item } => {
                    frames.push(Frame::Generator(*site));
                    frames.push(Frame::Enter(item));
                }
            },
            Frame::Array => {
                let inner = values.pop().expect("type clone child");
                values.push(Ty::Array(Box::new(inner)));
            }
            Frame::Union(kind, has_second) => {
                let second = has_second.then(|| Box::new(values.pop().expect("type clone second")));
                let first = Box::new(values.pop().expect("type clone first"));
                values.push(Ty::Union {
                    kind,
                    first,
                    second,
                });
            }
            Frame::Generator(site) => {
                let item = Box::new(values.pop().expect("type clone item"));
                values.push(Ty::Generator { site, item });
            }
        }
    }
    values.pop().expect("type clone root")
}
fn intrinsic_for_name(name: &str) -> Option<Intrinsic> {
    Some(match name {
        "Print" => Intrinsic::Print,
        "AssertEq" => Intrinsic::AssertEq,
        "Drop" => Intrinsic::DropData,
        "Fail" => Intrinsic::Fail,
        "Len" => Intrinsic::Len,
        "MockOpen" => Intrinsic::MockOpen,
        "MockBlockingOpen" => Intrinsic::MockBlockingOpen,
        "Inspect" => Intrinsic::Inspect,
        "TryAdd" => Intrinsic::TryAdd,
        "MockUnknown" => Intrinsic::MockUnknown,
        "MockReadRecords" => Intrinsic::MockReadRecords,
        "MockWriteI64" => Intrinsic::MockWriteI64,
        "MockRequest" => Intrinsic::MockRequest,
        "MockReply" => Intrinsic::MockReply,
        _ => return None,
    })
}
fn intrinsic_parameters(intrinsic: Intrinsic) -> Vec<(String, ParameterMode, Ty)> {
    match intrinsic {
        Intrinsic::AssertEq => vec![
            ("a".into(), ParameterMode::Read, Ty::Never),
            ("b".into(), ParameterMode::Read, Ty::Never),
        ],
        Intrinsic::Print | Intrinsic::DropData => vec![(
            "v".into(),
            if intrinsic == Intrinsic::Print {
                ParameterMode::Read
            } else {
                ParameterMode::Consume
            },
            Ty::Never,
        )],
        Intrinsic::Len => vec![("a".into(), ParameterMode::Read, Ty::Never)],
        Intrinsic::Push => vec![
            ("a".into(), ParameterMode::Inout, Ty::Never),
            ("value".into(), ParameterMode::Consume, Ty::Never),
        ],
        Intrinsic::Pop => vec![("a".into(), ParameterMode::Inout, Ty::Never)],
        Intrinsic::JoinArrays => vec![
            ("left".into(), ParameterMode::Read, Ty::Never),
            ("right".into(), ParameterMode::Read, Ty::Never),
        ],
        Intrinsic::Inspect => vec![("r".into(), ParameterMode::Read, Ty::Never)],
        Intrinsic::MockOpen | Intrinsic::MockBlockingOpen => {
            vec![("label".into(), ParameterMode::Consume, Ty::I64)]
        }
        Intrinsic::TryAdd => vec![
            ("a".into(), ParameterMode::Consume, Ty::I64),
            ("b".into(), ParameterMode::Consume, Ty::I64),
        ],
        Intrinsic::MockReadRecords => {
            vec![("r".into(), ParameterMode::Read, Ty::MockResource)]
        }
        Intrinsic::MockWriteI64 => vec![("n".into(), ParameterMode::Read, Ty::I64)],
        Intrinsic::MockRequest => vec![],
        Intrinsic::MockReply => vec![("status".into(), ParameterMode::Read, Ty::I64)],
        Intrinsic::Fail | Intrinsic::MockUnknown => vec![],
    }
}
fn intrinsic_arity(intrinsic: Intrinsic) -> usize {
    match intrinsic {
        Intrinsic::Print
        | Intrinsic::DropData
        | Intrinsic::Len
        | Intrinsic::Pop
        | Intrinsic::MockOpen
        | Intrinsic::MockBlockingOpen
        | Intrinsic::Inspect
        | Intrinsic::MockReadRecords
        | Intrinsic::MockWriteI64
        | Intrinsic::MockReply => 1,
        Intrinsic::AssertEq | Intrinsic::TryAdd | Intrinsic::Push | Intrinsic::JoinArrays => 2,
        Intrinsic::MockRequest | Intrinsic::Fail | Intrinsic::MockUnknown => 0,
    }
}
fn intrinsic_effect_io(intrinsic: Intrinsic) -> bool {
    matches!(
        intrinsic,
        Intrinsic::Print
            | Intrinsic::MockOpen
            | Intrinsic::MockBlockingOpen
            | Intrinsic::Inspect
            | Intrinsic::MockReadRecords
            | Intrinsic::MockWriteI64
            | Intrinsic::MockRequest
            | Intrinsic::MockReply
    )
}
pub(super) fn lower_type(ty: &Type) -> Result<Ty, CompileError> {
    enum Frame<'a> {
        Enter(&'a Type),
        Array,
        Result,
        Option,
        Generator(u64),
    }
    let mut frames = vec![Frame::Enter(ty)];
    let mut values = Vec::new();
    while let Some(frame) = frames.pop() {
        match frame {
            Frame::Enter(ty) => match ty {
                Type::I64 => values.push(Ty::I64),
                Type::Bool => values.push(Ty::Bool),
                Type::Unit => values.push(Ty::Unit),
                Type::MockResource => values.push(Ty::MockResource),
                Type::MockBlockingResource => values.push(Ty::MockBlockingResource),
                Type::Array(child) => {
                    frames.push(Frame::Array);
                    frames.push(Frame::Enter(child));
                }
                Type::Result(ok, err) => {
                    frames.push(Frame::Result);
                    frames.push(Frame::Enter(err));
                    frames.push(Frame::Enter(ok));
                }
                Type::Option(some) => {
                    frames.push(Frame::Option);
                    frames.push(Frame::Enter(some));
                }
                Type::Closure {
                    site,
                    copyable,
                    owns_resource,
                } => values.push(Ty::Closure {
                    site: *site,
                    copyable: *copyable,
                    owns_resource: *owns_resource,
                }),
                Type::Generator { site, item } => {
                    frames.push(Frame::Generator(*site));
                    frames.push(Frame::Enter(item));
                }
            },
            Frame::Array => {
                let child = values.pop().expect("array child");
                values.push(Ty::Array(Box::new(child)));
            }
            Frame::Result => {
                let err = values.pop().expect("result err");
                let ok = values.pop().expect("result ok");
                values.push(Ty::result(Box::new(ok), Box::new(err)));
            }
            Frame::Option => {
                let child = values.pop().expect("option child");
                values.push(Ty::option(Box::new(child)));
            }
            Frame::Generator(site) => {
                let item = values.pop().expect("generator item");
                values.push(Ty::Generator {
                    site,
                    item: Box::new(item),
                });
            }
        }
    }
    Ok(values.pop().expect("type output"))
}
fn statement_span(stmt: &Statement) -> Span {
    match stmt {
        Statement::Let { span, .. }
        | Statement::Assign { span, .. }
        | Statement::Return { span, .. }
        | Statement::GuardControl { span, .. }
        | Statement::Value { span, .. }
        | Statement::If { span, .. }
        | Statement::While { span, .. }
        | Statement::For { span, .. }
        | Statement::Expr { span, .. } => span.clone(),
        Statement::Match(selection) => selection.span.clone(),
        Statement::Block(block) => block.span.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lowering_requires_checker_binding_and_expression_entries() {
        let source =
            "fn Main(){var a=(if true then {return;}else{return;});a=[3];let b=a[0];Print(b);}";
        let ast = parse::parse("resolved_types.n", source).unwrap();
        let mut report =
            crate::check::check_for_compilation("resolved_types.n", source, &ast, true);
        assert!(report.errors.is_empty(), "{:?}", report.errors);
        let signatures = HashMap::new();
        let lower = analyzed_lower(source, &signatures, &report.inferred, ("a", Ty::Never));
        assert_eq!(
            lower.resolved_type(&ResolvedType::Never).unwrap(),
            Ty::Never
        );
        assert_eq!(
            lower.resolved_type(&ResolvedType::Unresolved).unwrap(),
            Ty::Never
        );
        assert!(compile_ast_with_inference(&ast, source, &report.inferred).is_ok());

        let binding = source.find("a=(if").unwrap() as u64;
        let original = report.inferred.bindings.remove(&binding).unwrap();
        assert!(matches!(
            compile_ast_with_inference(&ast, source, &report.inferred),
            Err(CompileError::InvalidIr("missing checker binding type"))
        ));
        report.inferred.bindings.insert(binding, original);

        let index = source.find("a[0]").unwrap() as u64;
        let key = (index, index + 4);
        let original = report.inferred.expressions.remove(&key).unwrap();
        assert!(matches!(
            compile_ast_with_inference(&ast, source, &report.inferred),
            Err(CompileError::InvalidIr("missing checker expression type"))
        ));
        report.inferred.expressions.insert(key, original);
        assert!(compile_ast_with_inference(&ast, source, &report.inferred).is_ok());

        let live_source = "fn Main(){let a=[3];Print(a[0]);}";
        let live_ast = parse::parse("resolved_live.n", live_source).unwrap();
        let mut live =
            crate::check::check_for_compilation("resolved_live.n", live_source, &live_ast, true);
        assert!(live.errors.is_empty());
        let binding = live_source.find("a=[3]").unwrap() as u64;
        let saved = live
            .inferred
            .bindings
            .insert(binding, ResolvedType::Concrete(Type::Bool))
            .unwrap();
        assert!(matches!(
            compile_ast_with_inference(&live_ast, live_source, &live.inferred),
            Err(CompileError::InvalidIr(_) | CompileError::Rejected(_))
        ));
        live.inferred.bindings.insert(binding, saved);
        let literal = live_source.find('3').unwrap() as u64;
        let original = live
            .inferred
            .expressions
            .insert((literal, literal + 1), ResolvedType::Concrete(Type::Bool))
            .unwrap();
        assert!(matches!(
            compile_ast_with_inference(&live_ast, live_source, &live.inferred),
            Err(CompileError::InvalidIr(_))
        ));
        live.inferred
            .expressions
            .insert((literal, literal + 1), original);
        assert!(compile_ast_with_inference(&live_ast, live_source, &live.inferred).is_ok());

        let unit_source = "fn Main(){let u=();}";
        let unit_ast = parse::parse("resolved_unit.n", unit_source).unwrap();
        let unit =
            crate::check::check_for_compilation("resolved_unit.n", unit_source, &unit_ast, true);
        assert!(unit.errors.is_empty());
        assert!(compile_ast_with_inference(&unit_ast, unit_source, &unit.inferred).is_ok());
    }

    #[test]
    fn lowering_consumes_operator_index_and_branch_types() {
        // F0-0002 §4.1.1 gives lowering only the checker's resolved table.
        // Each expression family must reject both a missing and forged entry.
        for (source, expression) in [
            ("fn Main(){let x=1+2;Print(x);}", "1+2"),
            ("fn Main(){let x=[3][0];Print(x);}", "[3][0]"),
            (
                "fn Main(){let x=if true then 1 else 2;Print(x);}",
                "if true then 1 else 2",
            ),
        ] {
            let ast = parse::parse("table_consumption.n", source).unwrap();
            let mut report =
                crate::check::check_for_compilation("table_consumption.n", source, &ast, true);
            assert!(report.errors.is_empty(), "{source}: {:?}", report.errors);
            assert!(
                compile_ast_with_inference(&ast, source, &report.inferred).is_ok(),
                "{source}"
            );
            let start = source.find(expression).unwrap() as u64;
            let key = (start, start + expression.len() as u64);
            let original = report.inferred.expressions.remove(&key).expect(expression);
            assert!(
                matches!(
                    compile_ast_with_inference(&ast, source, &report.inferred),
                    Err(CompileError::InvalidIr(_))
                ),
                "missing {expression}"
            );
            report
                .inferred
                .expressions
                .insert(key, ResolvedType::Concrete(Type::Bool));
            assert!(
                matches!(
                    compile_ast_with_inference(&ast, source, &report.inferred),
                    Err(CompileError::InvalidIr(_) | CompileError::Rejected(_))
                ),
                "forged {expression}"
            );
            report.inferred.expressions.insert(key, original);
            assert!(compile_ast_with_inference(&ast, source, &report.inferred).is_ok());
        }
    }

    #[test]
    fn lowering_has_no_result_type_derivation_api() {
        let source = include_str!("lower.rs");
        assert!(!source.contains(concat!("fn binary_", "result_type(")));
        assert!(!source.contains(concat!("fn place_", "ty(")));
        assert!(!source.contains(concat!("let mut element_", "ty =")));
        assert!(!source.contains(concat!("let mut inferred_", "ty =")));
    }

    #[test]
    fn f5_internal_literal_true_loop_has_only_reachable_control_edges() {
        let returned = "fn F()->I64{while true{return 5;}}fn Main(){}";
        let Err(CompileError::Rejected(old_errors)) =
            compile_scalar_with_profile("case.n", returned, ReducedProfile::JNormal, false)
        else {
            panic!("the historical J profile retains its false edge");
        };
        assert!(old_errors.iter().any(|d| d.code == "NX-E1317"));
        let program = compile_scalar_with_profile(
            "case.n",
            returned,
            ReducedProfile::F5ControlInternal,
            false,
        )
        .expect("literal true with return must satisfy I64 return");
        let function = program.functions.function("F").unwrap();
        assert!(
            function
                .blocks
                .iter()
                .all(|block| !matches!(block.terminator, Terminator::BranchBool { .. }))
        );

        for source in [
            "fn F()->I64{while (true){return 5;}}fn Main(){}",
            "fn F()->I64{while (((true))){return 5;}}fn Main(){}",
        ] {
            compile_scalar_with_profile("case.n", source, ReducedProfile::F5ControlInternal, false)
                .expect("grouped true has the same reachable CFG");
        }

        let broken = "fn F()->I64{while true{break;}}fn Main(){}";
        let Err(CompileError::Rejected(errors)) =
            compile_scalar_with_profile("case.n", broken, ReducedProfile::F5ControlInternal, false)
        else {
            panic!("break must retain the normal exit");
        };
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].code, "NX-E1317");
        assert_eq!(errors[0].primary_span.as_ref().unwrap().start, 30);
        assert_eq!(errors[0].primary_span.as_ref().unwrap().end, 31);

        for condition in ["false", "x"] {
            let source =
                format!("fn F()->I64{{let x=true;while {condition}{{return 5;}}}}fn Main(){{}}");
            let Err(CompileError::Rejected(errors)) = compile_scalar_with_profile(
                "case.n",
                &source,
                ReducedProfile::F5ControlInternal,
                false,
            ) else {
                panic!("only literal true proves total return");
            };
            assert!(errors.iter().any(|d| d.code == "NX-E1317"));
        }
        assert!(ReducedProfile::from_id("f0-0005-b-v1").is_some());
    }

    #[test]
    fn f5_neg_reaches_lowering_boundary() {
        let result = compile_scalar_with_profile(
            "case.n",
            "fn Main(){let x=1;let y=-x;}",
            ReducedProfile::F5Normal,
            false,
        );
        let Err(CompileError::Rejected(errors)) = result else {
            panic!("expected L08, got {result:?}");
        };
        assert_eq!(errors.len(), 1);
        assert_eq!(
            errors[0].data.get("pending_scope"),
            Some(&diagnostic::DataValue::String("L08".into()))
        );
    }

    #[test]
    fn f5_source_logical_cfg_skips_only_untaken_rhs() {
        let source = "fn Right()->Bool{Print(9);return true;}fn Main(){AssertEq(not false,true);AssertEq(not true,false);AssertEq(false and Right(),false);AssertEq(true or Right(),true);AssertEq(true and Right(),true);AssertEq(false or Right(),true);}";
        let mut ast = parse::parse_f5_hidden("case.n", source).unwrap();
        let checked = crate::check::check_for_compilation("case.n", source, &ast, true);
        assert!(checked.errors.is_empty(), "{:?}", checked.errors);
        // L08 remains the public boundary; this unit test exercises the
        // already verified CFG of L02 before that later quota work is wired.
        ast.f5_operators = false;
        let program = compile_ast_with_inference(&ast, source, &checked.inferred).unwrap();
        let execution = crate::interp::run_with_limits(
            &program,
            crate::interp::MockEnvironment::empty(),
            crate::interp::ReducedExecutionProfile::F5Normal,
        );
        assert_eq!(execution.outcome, crate::interp::Outcome::Unit);
        assert_eq!(execution.observations.stdout(), vec!["9", "9"]);
        assert!(execution.observations.events().is_empty());
        assert_eq!(execution.observations.bytes(), 4);
        assert_eq!(execution.observations.reserved_bytes(), 0);
    }

    #[test]
    fn f5_source_compound_assignment_reuses_prepared_scalar_place() {
        let source = "fn Main(){var x=7;x+=3;x-=2;x*=3;x/=5;x%=3;AssertEq(x,1);}";
        let mut ast = parse::parse_f5_hidden("case.n", source).unwrap();
        let checked = crate::check::check_for_compilation("case.n", source, &ast, true);
        assert!(checked.errors.is_empty(), "{:?}", checked.errors);
        ast.f5_operators = false;
        let program = compile_ast_with_inference(&ast, source, &checked.inferred).unwrap();
        let execution = crate::interp::run_with_limits(
            &program,
            crate::interp::MockEnvironment::empty(),
            crate::interp::ReducedExecutionProfile::F5Normal,
        );
        assert_eq!(execution.outcome, crate::interp::Outcome::Unit);
        assert!(execution.observations.stdout().is_empty());
        assert!(execution.observations.events().is_empty());
    }

    #[test]
    fn f5_source_compound_index_evaluates_selector_before_rhs_once() {
        for (index, rhs, expected_code, expected_stdout) in [
            (0, "2", None, vec!["1", "2", "[12]"]),
            (2, "2", Some("R0102"), vec!["1"]),
            (0, "7/0", Some("R0502"), vec!["1", "2"]),
        ] {
            let source = format!(
                "fn Index()->I64{{Print(1);return {index};}}fn Rhs()->I64{{Print(2);return {rhs};}}fn Main(){{var a=[10];a[Index()]+=Rhs();Print(a);}}"
            );
            let mut ast = parse::parse_f5_hidden("case.n", &source).unwrap();
            let checked = crate::check::check_for_compilation("case.n", &source, &ast, true);
            assert!(checked.errors.is_empty(), "{:?}", checked.errors);
            ast.f5_operators = false;
            let program = compile_ast_with_inference(&ast, &source, &checked.inferred).unwrap();
            let execution = crate::interp::run_with_limits(
                &program,
                crate::interp::MockEnvironment::empty(),
                crate::interp::ReducedExecutionProfile::F5Normal,
            );
            assert_eq!(
                execution.diagnostics.first().map(|d| d.code.as_str()),
                expected_code
            );
            assert_eq!(execution.observations.stdout(), expected_stdout, "{source}");
            assert!(execution.observations.events().is_empty());
            if rhs == "7/0" {
                let (_, snapshots) = crate::interp::Execution::run_with_fault_snapshots(
                    &program,
                    crate::interp::MockEnvironment::empty(),
                );
                assert!(
                    snapshots
                        .iter()
                        .any(|frame| frame.iter().any(|local| local.as_deref() == Some("[10]")))
                );
            }
        }
    }

    #[test]
    fn f5_source_compound_division_fault_has_operator_statement_span() {
        let source = "fn Main(){var x=7;x/=0;}";
        let mut ast = parse::parse_f5_hidden("case.n", source).unwrap();
        let checked = crate::check::check_for_compilation("case.n", source, &ast, true);
        assert!(checked.errors.is_empty(), "{:?}", checked.errors);
        ast.f5_operators = false;
        let program = compile_ast_with_inference(&ast, source, &checked.inferred).unwrap();
        let execution = crate::interp::run_with_limits(
            &program,
            crate::interp::MockEnvironment::empty(),
            crate::interp::ReducedExecutionProfile::F5Normal,
        );
        assert_eq!(execution.outcome, crate::interp::Outcome::SimulatorFault);
        assert_eq!(execution.diagnostics[0].code, "R0502");
        assert_eq!(
            execution.diagnostics[0]
                .primary_span
                .as_ref()
                .map(|span| (span.start, span.end)),
            Some((18, 22))
        );
    }

    #[test]
    fn f5_source_compound_rhs_return_ends_exclusive_loan() {
        let source = "fn F()->I64{var x=1;x+=if true then {return 2;} else 3;return x;}fn Main(){AssertEq(F(),2);}";
        let mut ast = parse::parse_f5_hidden("case.n", source).unwrap();
        let checked = crate::check::check_for_compilation("case.n", source, &ast, true);
        assert!(checked.errors.is_empty(), "{:?}", checked.errors);
        ast.f5_operators = false;
        let program = compile_ast_with_inference(&ast, source, &checked.inferred).unwrap();
        let execution = crate::interp::run_with_limits(
            &program,
            crate::interp::MockEnvironment::empty(),
            crate::interp::ReducedExecutionProfile::F5Normal,
        );
        assert_eq!(execution.outcome, crate::interp::Outcome::Unit);
        assert!(execution.diagnostics.is_empty());
    }

    #[test]
    fn f5_internal_literal_loop_continue_and_break_clean_scopes() {
        let source = "fn Main(){var n=0;while true{let r=MockOpen(n);if n==0{n=1;continue;}break;}AssertEq(n,1);}";
        let program =
            compile_scalar_with_profile("case.n", source, ReducedProfile::F5ControlInternal, true)
                .expect("continue and break must retain valid ownership and CFG joins");
        let execution = crate::interp::run_with_limits(
            &program,
            crate::interp::MockEnvironment::empty(),
            crate::interp::ReducedExecutionProfile::JNormal,
        );
        assert_eq!(execution.outcome, crate::interp::Outcome::Unit);
        assert_eq!(
            execution.observations.events(),
            ["open:0:0", "close:0:0", "open:1:1", "close:1:1"]
        );

        let consumed = "fn Main(){let r=MockOpen(1);while true{Drop(r);break;}Inspect(r);}";
        let Err(CompileError::Rejected(errors)) = compile_scalar_with_profile(
            "case.n",
            consumed,
            ReducedProfile::F5ControlInternal,
            true,
        ) else {
            panic!("the break edge must carry its resource consumption");
        };
        assert!(errors.iter().any(|error| {
            error.code == "NX-E0202" && error.subcause.as_deref() == Some("definitely_consumed")
        }));

        let joined = "fn Main(){let r=MockOpen(1);let b=true;while true{if b{Drop(r);break;}break;}Inspect(r);}";
        let Err(CompileError::Rejected(errors)) =
            compile_scalar_with_profile("case.n", joined, ReducedProfile::F5ControlInternal, true)
        else {
            panic!("distinct break edges must join their ownership states");
        };
        assert!(errors.iter().any(|error| {
            error.code == "NX-E0202" && error.subcause.as_deref() == Some("maybe_consumed")
        }));
    }

    fn hidden_f5_range_program(source: &str) -> Result<Program, CompileError> {
        let ast = parse::parse_f5_hidden("case.n", source).unwrap();
        let mut checked = crate::check::check_for_compilation("case.n", source, &ast, false);
        assert!(checked.internal_error.is_none());
        enum Node<'a> {
            Block(&'a parse::Block),
            Statement(&'a Statement),
            Expr(&'a Expr),
        }
        let tokens = crate::frontend::lex::lex("case.n", source).unwrap();
        let mut pending: Vec<_> = ast.functions.iter().map(|f| Node::Block(&f.body)).collect();
        while let Some(node) = pending.pop() {
            match node {
                Node::Block(block) => pending.extend(block.statements.iter().map(Node::Statement)),
                Node::Statement(statement) => match statement {
                    Statement::Let {
                        name, value, span, ..
                    } => {
                        if let Some(token) = tokens.iter().find(|token| {
                            token.text == *name
                                && token.span.start >= span.start
                                && token.span.end <= span.end
                        }) && let Expr::Call { target, .. } = value
                            && matches!(target.as_ref(), Expr::Name(name, _) if name == "MockOpen")
                        {
                            checked.inferred.bindings.insert(
                                token.span.start,
                                crate::check::types::ResolvedType::Concrete(
                                    parse::Type::MockResource,
                                ),
                            );
                        }
                        pending.push(Node::Expr(value));
                    }
                    Statement::If {
                        condition,
                        then_block,
                        otherwise,
                        ..
                    } => {
                        pending.push(Node::Expr(condition));
                        pending.push(Node::Block(then_block));
                        if let Some(otherwise) = otherwise {
                            pending.push(Node::Statement(otherwise));
                        }
                    }
                    Statement::For { iterable, body, .. }
                    | Statement::While {
                        condition: iterable,
                        body,
                        ..
                    } => {
                        pending.push(Node::Expr(iterable));
                        pending.push(Node::Block(body));
                    }
                    Statement::Expr { value, .. } => pending.push(Node::Expr(value)),
                    Statement::Assign { place, value, .. } => {
                        pending.push(Node::Expr(place));
                        pending.push(Node::Expr(value));
                    }
                    Statement::Return {
                        value: Some(value), ..
                    } => pending.push(Node::Expr(value)),
                    Statement::Block(block) => pending.push(Node::Block(block)),
                    _ => {}
                },
                Node::Expr(expr) => {
                    let ty = match expr {
                        Expr::Name(name, _) if name == "i" || name == "x" => Some(parse::Type::I64),
                        Expr::Integer(..) => Some(parse::Type::I64),
                        Expr::Bool(..) => Some(parse::Type::Bool),
                        Expr::Index { target, index, .. } => {
                            pending.push(Node::Expr(target));
                            pending.push(Node::Expr(index));
                            checked
                                .inferred
                                .expressions
                                .get(&(expr.span().start, expr.span().end))
                                .and_then(|resolved| match resolved {
                                    crate::check::types::ResolvedType::Concrete(ty) => {
                                        Some(ty.clone())
                                    }
                                    _ => None,
                                })
                        }
                        Expr::Binary {
                            op, left, right, ..
                        } => {
                            pending.push(Node::Expr(left));
                            pending.push(Node::Expr(right));
                            matches!(op.as_str(), "==" | "!=" | "<" | "<=" | ">" | ">=")
                                .then_some(parse::Type::Bool)
                        }
                        Expr::Call { target, args, .. } => {
                            pending.extend(args.iter().map(|arg| Node::Expr(&arg.value)));
                            match target.as_ref() {
                                Expr::Name(name, _) if name == "Print" => Some(parse::Type::Unit),
                                Expr::Name(name, _) if name == "MockOpen" => {
                                    Some(parse::Type::MockResource)
                                }
                                Expr::Name(name, _)
                                    if name == "Lo" || name == "Hi" || name == "F" =>
                                {
                                    Some(parse::Type::I64)
                                }
                                _ => None,
                            }
                        }
                        Expr::Group(value, _)
                        | Expr::Unary { value, .. }
                        | Expr::Neg { value, .. }
                        | Expr::Not { value, .. } => {
                            pending.push(Node::Expr(value));
                            None
                        }
                        _ => None,
                    };
                    if let Some(ty) = ty {
                        let span = expr.span();
                        checked.inferred.expressions.insert(
                            (span.start, span.end),
                            crate::check::types::ResolvedType::Concrete(ty),
                        );
                    }
                }
            }
        }
        compile_ast_with_flow_limit(
            &ast,
            source,
            &checked.inferred,
            analysis_limits(ReducedProfile::F5ControlInternal),
        )
    }

    #[test]
    fn public_j_profile_keeps_array_iteration_out_of_lowering() {
        let source = "fn Main(){var v2=[];let n=Len(v2);for x in v2{}v2[0]=7;}";
        assert!(matches!(
            compile_scalar_with_profile("case.n", source, ReducedProfile::JNormal, false),
            Err(CompileError::InvalidIr("for iterable type"))
        ));
    }

    #[test]
    fn f5_internal_array_for_captures_length_and_charges_t03() {
        let source = "fn Main(){for x in [1,2,3]{}}";
        let result = hidden_f5_range_program(source);
        assert!(result.is_ok(), "{result:?}");
        let program = result.unwrap();
        crate::ir::verify::verify_program(&program).unwrap();
        let function = program
            .functions
            .entries
            .iter()
            .find(|f| f.name == "Main")
            .unwrap();
        assert_eq!(
            std::iter::once(&function.block)
                .chain(&function.blocks)
                .flat_map(|block| &block.instructions)
                .filter(|ins| matches!(ins.op, Op::BeginArrayIteration(_)))
                .count(),
            1,
        );
        let execution =
            crate::interp::run_scalar(&program, crate::interp::MockEnvironment::empty());
        assert_eq!(
            execution.outcome,
            crate::interp::Outcome::Unit,
            "{execution:?}"
        );
        assert_eq!(execution.work_consumed, 20);
    }

    #[test]
    fn f5_internal_array_for_borrow_move_and_control_edges() {
        for (source, expected) in [
            (
                "fn Main(){let a=[1,2,3];for x in a{Print(x);}AssertEq(a,[1,2,3]);}",
                vec!["1", "2", "3"],
            ),
            (
                "fn Main(){var a=[1,2,3];for x in move a{if x==1{continue;}Print(x);if x==2{break;}}}",
                vec!["2"],
            ),
            (
                "fn F()->I64{for x in [4,5]{return x;}return -1;}fn Main(){Print(F());}",
                vec!["4"],
            ),
            (
                "fn Main(){let a:Array<I64>=[];for x in a{Print(x);}AssertEq(a,[]);}",
                vec![],
            ),
            ("fn Main(){for x in [[1],[2]]{}}", vec![]),
        ] {
            let program =
                super::super::ownership_flow::with_dense_oracle(|| hidden_f5_range_program(source))
                    .expect("typed array for IR and sparse/dense ownership parity");
            let execution =
                crate::interp::run_scalar(&program, crate::interp::MockEnvironment::empty());
            assert_eq!(
                execution.outcome,
                crate::interp::Outcome::Unit,
                "{source}: {execution:?}"
            );
            assert_eq!(execution.observations.stdout(), expected, "{source}");
        }
    }

    #[test]
    fn f5_array_for_borrows_source_without_defensive_copy() {
        let program = hidden_f5_range_program("fn Main(){let a=[1,2];for x in a{Print(x);}}")
            .expect("typed borrowed array for");
        let function = program
            .functions
            .entries
            .iter()
            .find(|f| f.name == "Main")
            .unwrap();
        let root = function
            .locals
            .iter()
            .find(|local| local.name == "a")
            .unwrap()
            .id;
        let instructions = std::iter::once(&function.block)
            .chain(&function.blocks)
            .flat_map(|block| &block.instructions)
            .collect::<Vec<_>>();
        assert!(
            !instructions
                .iter()
                .any(|ins| matches!(ins.op, Op::CopyValue(id) if id == root))
        );
        assert!(instructions.iter().any(|ins| matches!(ins.op, Op::BeginLoan { root: id, permission: LoanPermission::Read, .. } if id == root)));
    }

    #[test]
    fn f5_array_for_indexed_place_keeps_original_root_borrowed() {
        let source = "fn Main(){var rows=[[1,2]];for x in rows[0]{rows[0][1]=9;Print(x);}}";
        let result = hidden_f5_range_program(source);
        let Err(CompileError::Rejected(errors)) = result else {
            panic!("indexed Array iterator was detached from rows: {result:?}");
        };
        assert!(
            errors.iter().any(|error| {
                error.code == "NX-E1105"
                    && error.subcause.as_deref() == Some("active_borrow_conflict")
                    && error
                        .primary_span
                        .as_ref()
                        .is_some_and(|span| (span.start, span.end) == (44, 48))
            }),
            "{errors:?}"
        );

        for source in [
            "fn Main(){var a=[1,2];for x in a{a[0]=9;return;}}",
            "fn Main(){var rows=[[1,2]];for x in rows[0]{rows[0][1]=9;break;}}",
            "fn Main(){var rows=[[1,2]];for x in rows[0]{rows[0][1]=9;return;}}",
            "fn Main(){var rows=[[1,2]];return;for x in rows[0]{rows[0][1]=9;}}",
            "fn Main(){var rows=[[1,2]];for x in rows[0]{continue;rows[0][1]=9;}}",
            "fn Main(){var cube=[[[1,2]]];for x in cube[0][0]{cube[0][0][1]=9;}}",
        ] {
            let Err(CompileError::Rejected(errors)) = hidden_f5_range_program(source) else {
                panic!("indexed iterator mutation accepted: {source}");
            };
            assert!(
                errors.iter().any(|error| {
                    error.code == "NX-E1105"
                        && error.subcause.as_deref() == Some("active_borrow_conflict")
                }),
                "{source}: {errors:?}"
            );
        }

        for (source, expected) in [
            (
                "fn Main(){var rows=[[1,2]];for x in rows[0]{Print(x);}rows[0][1]=9;AssertEq(rows,[[1,9]]);}",
                vec!["1", "2"],
            ),
            (
                "fn Main(){var rows=[[1,2]];for x in (rows[0]){Print(x);}}",
                vec!["1", "2"],
            ),
            (
                "fn Main(){var rows=[[1,2]];for x in rows[0]{break;}rows[0][1]=9;AssertEq(rows,[[1,9]]);}",
                vec![],
            ),
            (
                "fn Main(){var rows=[[1,2]];for x in rows[0]{if x==1{continue;}Print(x);}}",
                vec!["2"],
            ),
            (
                "fn Make()->Array<I64>{return [4,5];}fn Main(){for x in Make(){Print(x);}}",
                vec!["4", "5"],
            ),
            (
                "fn Main(){var rows=[[1,2]];for x in rows[0]{return;}}",
                vec![],
            ),
            ("fn Main(){var a=[1,2];for x in a{return;}}", vec![]),
            (
                "fn Main(){var cube=[[[1,2]]];for x in cube[0][0]{Print(x);}}",
                vec!["1", "2"],
            ),
            (
                "fn Main(){var cube=[[[1,2]]];for x in cube[0][0]{break;}cube[0][0][1]=9;}",
                vec![],
            ),
            (
                "fn Main(){var cube=[[[1,2]]];for x in cube[0][0]{return;}}",
                vec![],
            ),
            (
                "fn Make()->Array<I64>{return [4,5];}fn Main(){var a=[0];for x in Make(){a[0]=9;}AssertEq(a,[9]);}",
                vec![],
            ),
            (
                "fn Make()->Array<I64>{return [4,5];}fn Main(){for x in Make(){break;}}",
                vec![],
            ),
            (
                "fn Make()->Array<I64>{return [4,5];}fn Main(){for x in Make(){return;}}",
                vec![],
            ),
            ("fn Main(){var a=[1,2];for x in move a{break;}}", vec![]),
            ("fn Main(){var a=[1,2];for x in move a{return;}}", vec![]),
        ] {
            let program = hidden_f5_range_program(source).expect(source);
            let execution =
                crate::interp::run_scalar(&program, crate::interp::MockEnvironment::empty());
            assert_eq!(
                execution.outcome,
                crate::interp::Outcome::Unit,
                "{source}: {execution:?}"
            );
            assert_eq!(execution.observations.stdout(), expected, "{source}");
        }

        let source = "fn Main(){var rows=[[1,2]];for x in rows[0]{Print(x);}}";
        let program = hidden_f5_range_program(source).expect("indexed Array IR");
        let function = program
            .functions
            .entries
            .iter()
            .find(|f| f.name == "Main")
            .unwrap();
        let root = function
            .locals
            .iter()
            .find(|local| local.name == "rows")
            .unwrap()
            .id;
        let iterable = source.find("rows[0]").unwrap() as u64;
        let instructions = std::iter::once(&function.block)
            .chain(&function.blocks)
            .flat_map(|block| &block.instructions)
            .collect::<Vec<_>>();
        assert!(instructions.iter().any(|ins| {
            matches!(ins.op, Op::BorrowIndex { local, .. } if local == root)
                && ins.span.start == iterable
        }));
        assert!(instructions.iter().any(|ins| {
            matches!(ins.op, Op::BeginLoan { root: id, permission: LoanPermission::Read, .. } if id == root)
        }));
        assert!(
            !instructions.iter().any(|ins| {
                matches!(ins.op, Op::ReadIndex { .. }) && ins.span.start == iterable
            })
        );

        let mut malformed = program.clone();
        let function = malformed
            .functions
            .entries
            .iter_mut()
            .find(|f| f.name == "Main")
            .unwrap();
        for block in std::iter::once(&mut function.block).chain(&mut function.blocks) {
            for instruction in &mut block.instructions {
                if let Op::BorrowArrayElement { source, .. } = &mut instruction.op {
                    *source = TempId(usize::MAX);
                }
            }
        }
        function.content_hash = super::super::model::content_hash(function);
        assert_eq!(
            super::super::verify::verify_program(&malformed),
            Err("undefined temporary")
        );
    }

    #[test]
    fn f5_array_for_projected_sources_do_not_panic() {
        for seed in 0..80 {
            let selector = ["0", "1", "-1", "2"][seed % 4];
            let body = [
                "Print(x);",
                "break;",
                "continue;",
                "return;",
                "rows[0][1]=9;",
            ][(seed / 4) % 5];
            let source =
                format!("fn Main(){{var rows=[[1,2],[3,4]];for x in rows[{selector}]{{{body}}}}}");
            if let Ok(program) = hidden_f5_range_program(&source) {
                let _ =
                    crate::interp::run_scalar(&program, crate::interp::MockEnvironment::empty());
            }
        }
    }

    #[test]
    fn f5_array_for_deep_projection_fits_small_stacks() {
        let nested = format!("{}1{}", "[".repeat(126), "]".repeat(126));
        let source = format!("fn Main(){{var rows={nested};for x in rows[0]{{}}}}");
        for bytes in [204, 256] {
            let source = source.clone();
            std::thread::Builder::new()
                .stack_size(bytes * 1024)
                .spawn(move || {
                    let program = hidden_f5_range_program(&source)
                        .expect("deep indexed Array for remains within the type limit");
                    let execution = crate::interp::run_scalar(
                        &program,
                        crate::interp::MockEnvironment::empty(),
                    );
                    assert_eq!(execution.outcome, crate::interp::Outcome::Unit);
                })
                .unwrap()
                .join()
                .unwrap();
        }
    }

    #[test]
    fn f5_array_for_move_consumes_source_binding() {
        let source = "fn Main(){let a=[1];for x in move a{}Print(a);}";
        let Err(CompileError::Rejected(errors)) = hidden_f5_range_program(source) else {
            panic!("moved Array remained available");
        };
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].code, "NX-E0202");

        let source = "fn Main(){var a=[1];for x in move a{a[0]=9;}}";
        let Err(CompileError::Rejected(errors)) = hidden_f5_range_program(source) else {
            panic!("moved Array root remained available in loop body");
        };
        assert!(errors.iter().any(|error| error.code == "NX-E0202"));
    }

    #[test]
    fn f5_internal_array_for_blocks_root_write_and_releases_after_break() {
        let blocked = "fn Main(){var a=[1];for x in a{a[0]=2;}}";
        let result = hidden_f5_range_program(blocked);
        let Err(CompileError::Rejected(errors)) = result else {
            panic!("{result:?}")
        };
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].code, "NX-E1105");
        assert_eq!(
            errors[0].subcause.as_deref(),
            Some("active_borrow_conflict")
        );
        let primary = errors[0].primary_span.as_ref().unwrap();
        assert_eq!((primary.start, primary.end), (31, 32));
        assert_eq!(
            errors[0].cause_chain[0].entity.as_ref().unwrap().id,
            "loan:29:0"
        );
        let origin = errors[0]
            .related_spans
            .iter()
            .find(|r| r.role == diagnostic::RelatedRole::Origin)
            .unwrap();
        assert_eq!((origin.span.start, origin.span.end), (29, 30));

        let independent = "fn Main(){var a=[1];for x in [1]{a[0]=2;}}";
        let program =
            hidden_f5_range_program(independent).expect("temporary array does not loan a");
        let execution =
            crate::interp::run_scalar(&program, crate::interp::MockEnvironment::empty());
        assert_eq!(
            execution.outcome,
            crate::interp::Outcome::Unit,
            "{execution:?}"
        );

        let released = "fn Main(){var a=[1];for x in a{break;}a[0]=2;AssertEq(a,[2]);}";
        let program = hidden_f5_range_program(released).expect("array loan ends on break");
        let execution =
            crate::interp::run_scalar(&program, crate::interp::MockEnvironment::empty());
        assert_eq!(
            execution.outcome,
            crate::interp::Outcome::Unit,
            "{execution:?}"
        );
    }

    #[test]
    fn f5_array_for_checks_borrow_in_dead_code() {
        for source in [
            "fn Main(){var a=[1];return;for x in a{a[0]=2;}}",
            "fn Main(){var a=[1];while true{break;for x in a{a[0]=2;}}}",
            "fn Main(){var a=[1];while true{continue;for x in a{a[0]=2;}}}",
        ] {
            let Err(CompileError::Rejected(errors)) = hidden_f5_range_program(source) else {
                panic!("dead Array for lost its read loan: {source}");
            };
            assert!(
                errors.iter().any(|error| {
                    error.code == "NX-E1105"
                        && error.subcause.as_deref() == Some("active_borrow_conflict")
                        && error.primary_span.as_ref().is_some_and(|span| {
                            span.start == source.rfind("a[0]").unwrap() as u64
                                && span.end == span.start + 1
                        })
                }),
                "{source}: {errors:?}"
            );
        }

        hidden_f5_range_program("fn Main(){var a=[1];return;for x in a{}a[0]=2;}")
            .expect("hypothetical Array loan ends after the dead loop");
    }

    #[test]
    fn f5_array_iteration_verifier_checks_root_and_loan_cleanup() {
        let source = "fn Main(){let a=[1];for x in a{Print(x);}}";
        let program = hidden_f5_range_program(source).expect("typed array for IR");
        let mut wrong_root = program.clone();
        let function = wrong_root
            .functions
            .entries
            .iter_mut()
            .find(|f| f.name == "Main")
            .unwrap();
        for block in std::iter::once(&mut function.block).chain(&mut function.blocks) {
            for instruction in &mut block.instructions {
                if let Op::BeginArrayIteration(root) = &mut instruction.op {
                    *root = TempId(usize::MAX);
                }
            }
        }
        function.content_hash = super::super::model::content_hash(function);
        assert_eq!(
            super::super::verify::verify_program(&wrong_root),
            Err("undefined temporary")
        );

        let mut missing_cleanup = program;
        let function = missing_cleanup
            .functions
            .entries
            .iter_mut()
            .find(|f| f.name == "Main")
            .unwrap();
        let guard = std::iter::once(&function.block)
            .chain(&function.blocks)
            .flat_map(|b| &b.instructions)
            .find_map(|ins| {
                matches!(ins.op, Op::BeginLoan { .. })
                    .then_some(ins.output)
                    .flatten()
            })
            .unwrap();
        for block in std::iter::once(&mut function.block).chain(&mut function.blocks) {
            block
                .instructions
                .retain(|ins| !matches!(ins.op, Op::DropValue(id) if id == guard));
            for (position, instruction) in block.instructions.iter_mut().enumerate() {
                instruction.origin = OriginId(position);
            }
        }
        function.content_hash = super::super::model::content_hash(function);
        assert!(super::super::verify::verify_program(&missing_cleanup).is_err());
    }

    #[test]
    fn f5_internal_range_ir_has_literal_exclusive_and_inclusive_ticks() {
        for (source, ticks) in [
            ("fn Main(){for i in 0..<3{}}", 19),
            ("fn Main(){for i in 0..=2{}}", 20),
        ] {
            let program = hidden_f5_range_program(source).expect("typed range IR");
            let execution =
                crate::interp::run_scalar(&program, crate::interp::MockEnvironment::empty());
            assert_eq!(execution.outcome, crate::interp::Outcome::Unit, "{source}");
            assert_eq!(execution.work_consumed, ticks, "{source}");
        }
    }

    #[test]
    fn f5_internal_range_limits_are_evaluated_once_and_max_never_advances() {
        for (source, stdout) in [
            (
                "fn Lo()->I64{Print(1);return 0;}fn Hi()->I64{Print(2);return 2;}fn Main(){for i in Lo()..<Hi(){Print(i);}}",
                vec!["1", "2", "0", "1"],
            ),
            (
                "fn Main(){for i in 2..<2{Print(i);}for i in 3..=2{Print(i);}}",
                vec![],
            ),
            (
                "fn Main(){for i in 9223372036854775807..=9223372036854775807{Print(i);}}",
                vec!["9223372036854775807"],
            ),
            (
                "fn Main(){for i in -9223372036854775808..=-9223372036854775808{Print(i);}}",
                vec!["-9223372036854775808"],
            ),
        ] {
            let program = hidden_f5_range_program(source).expect("typed range IR");
            let function = program.functions.function("Main").unwrap();
            assert!(
                std::iter::once(&function.block)
                    .chain(&function.blocks)
                    .all(|block| {
                        block
                            .instructions
                            .iter()
                            .all(|ins| !matches!(ins.op, Op::BuildArray(_)))
                    })
            );
            let execution =
                crate::interp::run_scalar(&program, crate::interp::MockEnvironment::empty());
            assert_eq!(execution.outcome, crate::interp::Outcome::Unit, "{source}");
            assert_eq!(execution.observations.stdout(), stdout, "{source}");
        }
    }

    #[test]
    fn f5_internal_range_values_match_independent_integer_oracle() {
        for (lower, upper, inclusive) in [
            (0_i64, 3_i64, false),
            (0, 2, true),
            (2, 2, false),
            (2, 2, true),
            (3, 2, true),
            (i64::MIN, i64::MIN, true),
            (i64::MAX, i64::MAX, true),
        ] {
            let operation = if inclusive { "..=" } else { "..<" };
            let source = format!("fn Main(){{for i in {lower}{operation}{upper}{{Print(i);}}}}");
            let program = hidden_f5_range_program(&source).expect("typed range IR");
            let execution =
                crate::interp::run_scalar(&program, crate::interp::MockEnvironment::empty());
            let expected: Vec<_> = if inclusive {
                (lower..=upper).map(|value| value.to_string()).collect()
            } else {
                (lower..upper).map(|value| value.to_string()).collect()
            };
            assert_eq!(execution.outcome, crate::interp::Outcome::Unit);
            assert_eq!(execution.observations.stdout(), expected, "{source}");
        }
    }

    #[test]
    fn f5_internal_range_verifier_rejects_invalid_final_test_and_cursor() {
        let source = "fn Main(){for i in 0..=1{}}";
        let mut final_test = hidden_f5_range_program(source).expect("typed range IR");
        let function = final_test
            .functions
            .entries
            .iter_mut()
            .find(|f| f.name == "Main")
            .unwrap();
        let instruction = std::iter::once(&mut function.block)
            .chain(function.blocks.iter_mut())
            .flat_map(|block| block.instructions.iter_mut())
            .find(|ins| {
                matches!(
                    ins.op,
                    Op::RangeTest {
                        final_test: true,
                        ..
                    }
                )
            })
            .unwrap();
        if let Op::RangeTest { inclusive, .. } = &mut instruction.op {
            *inclusive = false;
        }
        function.content_hash = super::super::model::content_hash(function);
        assert_eq!(
            crate::ir::verify::verify_program(&final_test),
            Err("exclusive range final test")
        );

        let mut advance = hidden_f5_range_program(source).expect("typed range IR");
        let function = advance
            .functions
            .entries
            .iter_mut()
            .find(|f| f.name == "Main")
            .unwrap();
        let instruction = std::iter::once(&mut function.block)
            .chain(function.blocks.iter_mut())
            .flat_map(|block| block.instructions.iter_mut())
            .find(|ins| matches!(ins.op, Op::RangeAdvance { .. }))
            .unwrap();
        if let Op::RangeAdvance { cursor, limit } = &mut instruction.op {
            *cursor = *limit;
        }
        function.content_hash = super::super::model::content_hash(function);
        assert_eq!(
            crate::ir::verify::verify_program(&advance),
            Err("range cursor write")
        );
    }

    #[test]
    fn f5_internal_range_binding_is_not_visible_after_loop() {
        let source = "fn Main(){for i in 0..<1{}Print(i);}";
        assert!(matches!(
            hidden_f5_range_program(source),
            Err(CompileError::InvalidIr("unresolved local"))
        ));
    }

    #[test]
    fn f5_internal_range_control_closes_each_body_before_advancing() {
        let source = "fn Main(){for i in 0..<2{let r=MockOpen(i);if i==0{continue;}break;}}";
        let program = hidden_f5_range_program(source).expect("typed range with control edges");
        let execution =
            crate::interp::run_scalar(&program, crate::interp::MockEnvironment::empty());
        assert_eq!(execution.outcome, crate::interp::Outcome::Unit);
        assert_eq!(
            execution.observations.events(),
            ["open:0:0", "close:0:0", "open:1:1", "close:1:1"]
        );
        let returning = "fn Main(){for i in 0..<2{let r=MockOpen(i);return;}}";
        let program = hidden_f5_range_program(returning).expect("typed returning range");
        let execution =
            crate::interp::run_scalar(&program, crate::interp::MockEnvironment::empty());
        assert_eq!(execution.outcome, crate::interp::Outcome::Unit);
        assert_eq!(execution.observations.events(), ["open:0:0", "close:0:0"]);
        let max_continue =
            "fn Main(){for i in 9223372036854775807..=9223372036854775807{continue;}}";
        let program = hidden_f5_range_program(max_continue).expect("typed MAX continue range");
        let execution =
            crate::interp::run_scalar(&program, crate::interp::MockEnvironment::empty());
        assert_eq!(execution.outcome, crate::interp::Outcome::Unit);
        let early_break = "fn Main(){for i in 0..<3{if i==0{break;}Print(i);}}";
        let program = hidden_f5_range_program(early_break).expect("typed early break range");
        let execution =
            crate::interp::run_scalar(&program, crate::interp::MockEnvironment::empty());
        assert_eq!(execution.outcome, crate::interp::Outcome::Unit);
        assert!(execution.observations.stdout().is_empty());
        let return_value = "fn F()->I64{for i in 0..<2{return i;}return -1;}fn Main(){Print(F());}";
        let program = hidden_f5_range_program(return_value).expect("typed returning value range");
        let execution =
            crate::interp::run_scalar(&program, crate::interp::MockEnvironment::empty());
        assert_eq!(execution.outcome, crate::interp::Outcome::Unit);
        assert_eq!(execution.observations.stdout(), ["0"]);
    }

    fn analyzed_lower<'a>(
        source: &str,
        signatures: &'a HashMap<String, FunctionSignature>,
        inferred: &'a crate::check::types::InferredTypes,
        binding: (&str, Ty),
    ) -> Lower<'a> {
        let (name, ty) = binding;
        let span = Span {
            file: "flow_contract.n".into(),
            start: source.find(name).unwrap_or(0) as u64,
            end: source.find(name).unwrap_or(0) as u64 + name.len() as u64,
        };
        Lower {
            instructions: Vec::new(),
            pending_charges: Vec::new(),
            blocks: Vec::new(),
            locals: vec![Local {
                id: LocalId(0),
                name: name.into(),
                name_span: Span {
                    file: "bottom_flow.n".into(),
                    start: 0,
                    end: 0,
                },
                ty,
                mutable: true,
                scope: ScopeId(0),
                parameter_mode: None,
            }],
            visible: HashMap::from([(name.into(), vec![LocalId(0)])]),
            scopes: vec![vec![name.into()]],
            scope: ScopeId(0),
            next_scope: 1,
            scope_parents: vec![None],
            next_temp: 0,
            result: Ty::Unit,
            terminated: None,
            user_functions: signatures,
            inferred_bindings: inferred,
            name_spans: vec![span],
            consumed_at: HashMap::new(),
            maybe_consumed_at: HashMap::new(),
            consumed_undo: Vec::new(),
            dead_depth: 1,
            emission: EmissionMode::Analyze,
            literal_true_loops: false,
            poisoned: HashSet::new(),
            tokens: Arc::new(lex::lex("flow_contract.n", source).unwrap()),
            defer_ownership: false,
            active_loans: HashMap::new(),
            loop_stack: Vec::new(),
            match_boundaries: Vec::new(),
            retained_locals: HashMap::new(),
            retained_temps: HashMap::new(),
            requested_receiver: None,
            live_temps: HashSet::new(),
            live_locals: HashSet::new(),
            local_head: None,
            local_history: Vec::new(),
            temp_loans: HashMap::new(),
            temp_undo: Vec::new(),
            temp_block_entry: EdgeState::default(),
            temp_blocks: Vec::new(),
            deferred_return_locals: Vec::new(),
            mode_candidates: Vec::new(),
        }
    }

    #[test]
    fn indexed_write_consumes_index_and_value_temporaries() {
        let signatures = HashMap::new();
        let inferred = crate::check::types::InferredTypes::default();
        let mut lower = analyzed_lower(
            "fn Main(){let a=1;}",
            &signatures,
            &inferred,
            ("a", Ty::I64),
        );
        let index = TempId(1);
        let value = TempId(2);
        let site_span = lower.tokens[0].span.clone();
        lower.live_temps.extend([index, value]);
        lower.consume_op_temps(&Op::WriteIndex {
            local: LocalId(0),
            indexes: vec![index],
            site: IndexSite {
                context: site_span.clone(),
                selectors: vec![site_span],
            },
            value,
        });
        assert!(!lower.live_temps.contains(&index));
        assert!(!lower.live_temps.contains(&value));
    }

    #[test]
    fn analyzed_binding_loan_and_index_preserve_flow_type() {
        let source = "fn Main() {\n    h(v);\n    h(a[(if true then {\n        return;\n    } else {\n        return;\n    })]);\n}\n";
        let ast = parse::parse("flow_contract.n", source).unwrap();
        let mut inferred =
            crate::check::check_for_compilation(&ast.span.file, source, &ast, false).inferred;
        let signatures = HashMap::new();
        let statements = &ast.functions[0].body.statements;
        let Statement::Expr {
            value: Expr::Call { args: first, .. },
            ..
        } = &statements[0]
        else {
            panic!("call")
        };
        inferred.expressions.insert(
            (first[0].value.span().start, first[0].value.span().end),
            ResolvedType::Concrete(Type::I64),
        );
        let Statement::Expr {
            value: Expr::Call { args: second, .. },
            ..
        } = &statements[1]
        else {
            panic!("second call")
        };
        let Expr::Index { index, .. } = &second[0].value else {
            panic!("index")
        };
        inferred.expressions.insert(
            (second[0].value.span().start, second[0].value.span().end),
            ResolvedType::Concrete(Type::I64),
        );
        let Expr::Group(inner, _) = index.as_ref() else {
            panic!("group")
        };
        let Expr::If { condition, .. } = inner.as_ref() else {
            panic!("if")
        };
        for expr in [index.as_ref(), inner.as_ref()] {
            inferred
                .expressions
                .insert((expr.span().start, expr.span().end), ResolvedType::Never);
        }
        inferred.expressions.insert(
            (condition.span().start, condition.span().end),
            ResolvedType::Concrete(Type::Bool),
        );
        let mut binding = analyzed_lower(source, &signatures, &inferred, ("v", Ty::Never));
        let (flow, loan) = binding
            .call_argument(&first[0], ParameterMode::Read, &Ty::I64, 0, true)
            .unwrap();
        assert!(matches!(
            flow,
            Flow::Value {
                temp: None,
                ty: Ty::I64
            }
        ));
        assert_eq!(loan, None);

        let mut indexed = analyzed_lower(
            source,
            &signatures,
            &inferred,
            ("a", Ty::Array(Box::new(Ty::I64))),
        );
        let (flow, _) = indexed
            .call_argument(&second[0], ParameterMode::Read, &Ty::I64, 0, true)
            .unwrap();
        assert!(matches!(flow, Flow::Diverged { ty: Some(Ty::I64) }));
    }

    #[test]
    fn analyzed_branch_retains_type_of_only_continuing_value() {
        let source = "fn Main() {\n    let w = if true then 1 else {\n        return;\n    };\n}\n";
        let ast = parse::parse("flow_contract.n", source).unwrap();
        let inferred =
            crate::check::check_for_compilation(&ast.span.file, source, &ast, false).inferred;
        let signatures = HashMap::new();
        let mut lower = analyzed_lower(source, &signatures, &inferred, ("unused", Ty::Never));
        let Statement::Let { value, .. } = &ast.functions[0].body.statements[0] else {
            panic!("binding")
        };
        let flow = lower.expr(value, 0, None).unwrap();
        assert!(matches!(
            flow,
            Flow::Value {
                temp: None,
                ty: Ty::I64
            }
        ));
    }

    #[test]
    fn hypothetical_bottom_read_preserves_rhs_flow_and_branch_join_type() {
        let source = "fn Main() {\n    let v = if true then {\n        return;\n    } else {\n        return;\n    };\n    let w = v == (if true then {\n        return;\n    } else {\n        return;\n    });\n}\n";
        let ast = parse::parse("bottom_flow.n", source).unwrap();
        let inferred =
            crate::check::check_for_compilation(&ast.span.file, source, &ast, false).inferred;
        let signatures = HashMap::new();
        let declaration = &ast.functions[0].body.statements[0];
        let Statement::Let { value, .. } = declaration else {
            panic!("first binding")
        };
        let name_span = Span {
            file: "bottom_flow.n".into(),
            start: statement_span(declaration).start + 4,
            end: statement_span(declaration).start + 5,
        };
        let mut lower = Lower {
            instructions: Vec::new(),
            pending_charges: Vec::new(),
            blocks: Vec::new(),
            locals: vec![Local {
                id: LocalId(0),
                name: "v".into(),
                name_span: Span {
                    file: "bottom_flow.n".into(),
                    start: 0,
                    end: 0,
                },
                ty: Ty::Never,
                mutable: false,
                scope: ScopeId(0),
                parameter_mode: None,
            }],
            visible: HashMap::from([("v".into(), vec![LocalId(0)])]),
            scopes: vec![vec!["v".into()]],
            scope: ScopeId(0),
            next_scope: 1,
            scope_parents: vec![None],
            next_temp: 0,
            result: Ty::Unit,
            terminated: None,
            user_functions: &signatures,
            inferred_bindings: &inferred,
            name_spans: vec![name_span],
            consumed_at: HashMap::new(),
            maybe_consumed_at: HashMap::new(),
            consumed_undo: Vec::new(),
            dead_depth: 1,
            emission: EmissionMode::Analyze,
            literal_true_loops: false,
            poisoned: HashSet::new(),
            tokens: Arc::new(lex::lex("bottom_flow.n", source).unwrap()),
            defer_ownership: false,
            active_loans: HashMap::new(),
            loop_stack: Vec::new(),
            match_boundaries: Vec::new(),
            retained_locals: HashMap::new(),
            retained_temps: HashMap::new(),
            requested_receiver: None,
            live_temps: HashSet::new(),
            live_locals: HashSet::new(),
            local_head: None,
            local_history: Vec::new(),
            temp_loans: HashMap::new(),
            temp_undo: Vec::new(),
            temp_block_entry: EdgeState::default(),
            temp_blocks: Vec::new(),
            deferred_return_locals: Vec::new(),
            mode_candidates: Vec::new(),
        };
        let Statement::Let {
            value: expression, ..
        } = &ast.functions[0].body.statements[1]
        else {
            panic!("second binding")
        };
        let flow = lower.expr(expression, 0, None).unwrap();
        assert_eq!(flow.ty(), Some(&Ty::Bool));
        assert_eq!(lower.terminated, Some(Termination::Diverged));
        let mut slot = None;
        lower
            .union_slot(&mut slot, TempId(0), Ty::Never, &Ty::I64, value.span())
            .unwrap();
        lower
            .union_slot(&mut slot, TempId(0), Ty::I64, &Ty::I64, value.span())
            .unwrap();
        assert_eq!(lower.locals[slot.unwrap().0].ty, Ty::I64);
    }

    #[test]
    fn divergent_call_argument_does_not_leave_orphan_call_instruction() {
        let source = "fn H(a: view I64, b: view I64) {}\nfn Main() {\n    H(1, if true then {\n        return;\n    } else {\n        return;\n    });\n}\n";
        let ast = parse::parse("orphan_call.n", source).unwrap();
        let inferred =
            crate::check::check_for_compilation(&ast.span.file, source, &ast, false).inferred;
        let mut signatures = HashMap::new();
        signatures.insert(
            "H".into(),
            (
                vec![
                    ("a".into(), ParameterMode::Read, Ty::I64),
                    ("b".into(), ParameterMode::Read, Ty::I64),
                ],
                Ty::Unit,
            ),
        );
        let mut lower = Lower {
            instructions: Vec::new(),
            pending_charges: Vec::new(),
            blocks: Vec::new(),
            locals: Vec::new(),
            visible: HashMap::new(),
            scopes: vec![Vec::new()],
            scope: ScopeId(0),
            next_scope: 1,
            scope_parents: vec![None],
            next_temp: 0,
            result: Ty::Unit,
            terminated: None,
            user_functions: &signatures,
            inferred_bindings: &inferred,
            name_spans: Vec::new(),
            consumed_at: HashMap::new(),
            maybe_consumed_at: HashMap::new(),
            consumed_undo: Vec::new(),
            dead_depth: 0,
            emission: EmissionMode::Live,
            literal_true_loops: false,
            poisoned: HashSet::new(),
            tokens: Arc::new(lex::lex("orphan_call.n", source).unwrap()),
            defer_ownership: false,
            active_loans: HashMap::new(),
            loop_stack: Vec::new(),
            match_boundaries: Vec::new(),
            retained_locals: HashMap::new(),
            retained_temps: HashMap::new(),
            requested_receiver: None,
            live_temps: HashSet::new(),
            live_locals: HashSet::new(),
            local_head: None,
            local_history: Vec::new(),
            temp_loans: HashMap::new(),
            temp_undo: Vec::new(),
            temp_block_entry: EdgeState::default(),
            temp_blocks: Vec::new(),
            deferred_return_locals: Vec::new(),
            mode_candidates: Vec::new(),
        };
        let main = ast.functions.iter().find(|f| f.name == "Main").unwrap();
        lower
            .statement(&main.body.statements[0], 0, &mut Vec::new())
            .unwrap();
        assert_eq!(lower.terminated, Some(Termination::Diverged));
        assert!(
            !lower
                .instructions
                .iter()
                .any(|i| matches!(i.op, Op::CallDirect { .. }))
        );
        let instructions = lower.instructions.len();
        let blocks = lower.blocks.len();
        let temporaries = lower.next_temp;
        lower.emission = EmissionMode::Analyze;
        let span = main.body.span.clone();
        assert_eq!(
            lower.emit(Op::ConstI64(3), Ty::I64, span.clone(), Access::Own, false),
            None
        );
        lower.statement_op(Op::DropLocal(LocalId(0)), span.clone(), Access::Own, false);
        assert_eq!(
            lower.finish_block(Terminator::Return {
                value: None,
                span,
                charges: vec![]
            }),
            BlockId(usize::MAX)
        );
        assert_eq!(lower.instructions.len(), instructions);
        assert_eq!(lower.next_temp, temporaries);
        assert_eq!(lower.blocks.len(), blocks);
    }

    #[test]
    fn scoped_block_error_pops_scope_and_restores_parent() {
        let source = "fn Main() {\n    {\n        let x = MockUnknown();\n    }\n}\n";
        let ast = parse::parse("scope_error.n", source).unwrap();
        let inferred =
            crate::check::check_for_compilation(&ast.span.file, source, &ast, false).inferred;
        let signatures = HashMap::new();
        let mut lower = Lower {
            instructions: Vec::new(),
            pending_charges: Vec::new(),
            blocks: Vec::new(),
            locals: Vec::new(),
            visible: HashMap::new(),
            scopes: vec![Vec::new()],
            scope: ScopeId(0),
            next_scope: 1,
            scope_parents: vec![None],
            next_temp: 0,
            result: Ty::Unit,
            terminated: None,
            user_functions: &signatures,
            inferred_bindings: &inferred,
            name_spans: Vec::new(),
            consumed_at: HashMap::new(),
            maybe_consumed_at: HashMap::new(),
            consumed_undo: Vec::new(),
            dead_depth: 0,
            emission: EmissionMode::Live,
            literal_true_loops: false,
            poisoned: HashSet::new(),
            tokens: Arc::new(lex::lex("scope_error.n", source).unwrap()),
            defer_ownership: false,
            active_loans: HashMap::new(),
            loop_stack: Vec::new(),
            match_boundaries: Vec::new(),
            retained_locals: HashMap::new(),
            retained_temps: HashMap::new(),
            requested_receiver: None,
            live_temps: HashSet::new(),
            live_locals: HashSet::new(),
            local_head: None,
            local_history: Vec::new(),
            temp_loans: HashMap::new(),
            temp_undo: Vec::new(),
            temp_block_entry: EdgeState::default(),
            temp_blocks: Vec::new(),
            deferred_return_locals: Vec::new(),
            mode_candidates: Vec::new(),
        };
        lower
            .statement(&ast.functions[0].body.statements[0], 0, &mut Vec::new())
            .unwrap();
        assert_eq!(lower.scopes.len(), 1);
        assert_eq!(lower.scope, ScopeId(0));
        assert!(lower.visible.values().all(Vec::is_empty));
    }

    #[test]
    fn rejected_while_condition_restores_statement_checkpoint() {
        let source = "fn Main() {\n    while (if true then true else {}) {}\n    let x = 1;\n}\n";
        let ast = parse::parse("while_error_checkpoint.n", source).unwrap();
        let inferred =
            crate::check::check_for_compilation(&ast.span.file, source, &ast, false).inferred;
        let signatures = HashMap::new();
        let mut lower = Lower {
            instructions: Vec::new(),
            pending_charges: Vec::new(),
            blocks: Vec::new(),
            locals: Vec::new(),
            visible: HashMap::new(),
            scopes: vec![Vec::new()],
            scope: ScopeId(0),
            next_scope: 1,
            scope_parents: vec![None],
            next_temp: 0,
            result: Ty::Unit,
            terminated: None,
            user_functions: &signatures,
            inferred_bindings: &inferred,
            name_spans: Vec::new(),
            consumed_at: HashMap::new(),
            maybe_consumed_at: HashMap::new(),
            consumed_undo: Vec::new(),
            dead_depth: 0,
            emission: EmissionMode::Live,
            literal_true_loops: false,
            poisoned: HashSet::new(),
            tokens: Arc::new(lex::lex("while_error_checkpoint.n", source).unwrap()),
            defer_ownership: false,
            active_loans: HashMap::new(),
            loop_stack: Vec::new(),
            match_boundaries: Vec::new(),
            retained_locals: HashMap::new(),
            retained_temps: HashMap::new(),
            requested_receiver: None,
            live_temps: HashSet::new(),
            live_locals: HashSet::new(),
            local_head: None,
            local_history: Vec::new(),
            temp_loans: HashMap::new(),
            temp_undo: Vec::new(),
            temp_block_entry: EdgeState::default(),
            temp_blocks: Vec::new(),
            deferred_return_locals: Vec::new(),
            mode_candidates: Vec::new(),
        };
        let mut diagnostics = Vec::new();
        lower
            .block(&ast.functions[0].body.statements, 0, &mut diagnostics)
            .unwrap();
        assert!(
            diagnostics
                .iter()
                .any(|d| d.subcause.as_deref() == Some("missing_branch_value"))
        );
        assert_eq!(lower.dead_depth, 0);
        assert_eq!(lower.terminated, None);
        assert_eq!(lower.scopes.len(), 1);
        assert!(lower.visible.contains_key("x"));
        let x = lower.visible["x"][0];
        let at = |start, end| Span {
            file: "while_error_checkpoint.n".into(),
            start,
            end,
        };
        let CompileError::Rejected(errors) = lower.ownership_error(
            "NX-E0202",
            Some("consumed"),
            diagnostic::Operation::OwnershipAccess,
            at(30, 31),
            x,
            Some(vec![(at(8, 9), false), (at(4, 20), false)]),
        ) else {
            panic!("expected ownership diagnostic");
        };
        let consumption: Vec<_> = errors[0]
            .related_spans
            .iter()
            .filter(|related| related.role == diagnostic::RelatedRole::Consumption)
            .map(|related| (related.span.start, related.span.end))
            .collect();
        assert_eq!(consumption, vec![(4, 20), (8, 9)]);
    }

    #[test]
    fn merged_consumption_sites_keep_both_sides_sorted_without_duplicates() {
        // Identical sites deduplicate, while independent starts remain in
        // source order, including overlapping spans.
        let at = |start| Span {
            file: "merge.n".into(),
            start,
            end: start + 1,
        };
        let left = vec![(at(8), false), (at(2), true)];
        let right = vec![(at(5), false), (at(2), true)];
        assert_eq!(
            merge_sites(left, right),
            vec![(at(2), true), (at(5), false), (at(8), false)]
        );
        let nested = |start| Span {
            file: "merge.n".into(),
            start,
            end: 12,
        };
        assert_eq!(
            merge_sites(vec![(nested(9), false)], vec![(nested(3), false)]),
            vec![(nested(3), false), (nested(9), false)]
        );
        let same_start = |end| Span {
            file: "merge.n".into(),
            start: 4,
            end,
        };
        assert_eq!(
            merge_sites(vec![(same_start(11), false)], vec![(same_start(7), false)]),
            vec![(same_start(7), false), (same_start(11), false)]
        );
    }

    #[test]
    fn branch_local_consumption_does_not_escape_its_scope() {
        let source = "fn Main() {\n    if true {\n        let r = MockOpen(1);\n        Drop(r);\n    } else {}\n}\n";
        let ast = parse::parse("branch_scope.n", source).unwrap();
        let inferred =
            crate::check::check_for_compilation(&ast.span.file, source, &ast, false).inferred;
        let signatures = HashMap::new();
        let mut lower = Lower {
            instructions: Vec::new(),
            pending_charges: Vec::new(),
            blocks: Vec::new(),
            locals: Vec::new(),
            visible: HashMap::new(),
            scopes: vec![Vec::new()],
            scope: ScopeId(0),
            next_scope: 1,
            scope_parents: vec![None],
            next_temp: 0,
            result: Ty::Unit,
            terminated: None,
            user_functions: &signatures,
            inferred_bindings: &inferred,
            name_spans: Vec::new(),
            consumed_at: HashMap::new(),
            maybe_consumed_at: HashMap::new(),
            consumed_undo: Vec::new(),
            dead_depth: 0,
            emission: EmissionMode::Live,
            literal_true_loops: false,
            poisoned: HashSet::new(),
            tokens: Arc::new(lex::lex("branch_scope.n", source).unwrap()),
            defer_ownership: false,
            active_loans: HashMap::new(),
            loop_stack: Vec::new(),
            match_boundaries: Vec::new(),
            retained_locals: HashMap::new(),
            retained_temps: HashMap::new(),
            requested_receiver: None,
            live_temps: HashSet::new(),
            live_locals: HashSet::new(),
            local_head: None,
            local_history: Vec::new(),
            temp_loans: HashMap::new(),
            temp_undo: Vec::new(),
            temp_block_entry: EdgeState::default(),
            temp_blocks: Vec::new(),
            deferred_return_locals: Vec::new(),
            mode_candidates: Vec::new(),
        };
        lower
            .statement(&ast.functions[0].body.statements[0], 0, &mut Vec::new())
            .unwrap();
        assert!(lower.consumed_at.is_empty());
        assert!(lower.maybe_consumed_at.is_empty());
    }

    #[test]
    fn pending_read_loan_rejects_assignment_with_write_access() {
        let source = "fn Main() {\n    var x = 1;\n    x = 2;\n}\n";
        let ast = parse::parse("loan_write.n", source).unwrap();
        let inferred =
            crate::check::check_for_compilation(&ast.span.file, source, &ast, false).inferred;
        let signatures = HashMap::new();
        let mut lower = Lower {
            instructions: Vec::new(),
            pending_charges: Vec::new(),
            blocks: Vec::new(),
            locals: Vec::new(),
            visible: HashMap::new(),
            scopes: vec![Vec::new()],
            scope: ScopeId(0),
            next_scope: 1,
            scope_parents: vec![None],
            next_temp: 0,
            result: Ty::Unit,
            terminated: None,
            user_functions: &signatures,
            inferred_bindings: &inferred,
            name_spans: Vec::new(),
            consumed_at: HashMap::new(),
            maybe_consumed_at: HashMap::new(),
            consumed_undo: Vec::new(),
            dead_depth: 0,
            emission: EmissionMode::Live,
            literal_true_loops: false,
            poisoned: HashSet::new(),
            tokens: Arc::new(lex::lex("loan_write.n", source).unwrap()),
            defer_ownership: false,
            active_loans: HashMap::new(),
            loop_stack: Vec::new(),
            match_boundaries: Vec::new(),
            retained_locals: HashMap::new(),
            retained_temps: HashMap::new(),
            requested_receiver: None,
            live_temps: HashSet::new(),
            live_locals: HashSet::new(),
            local_head: None,
            local_history: Vec::new(),
            temp_loans: HashMap::new(),
            temp_undo: Vec::new(),
            temp_block_entry: EdgeState::default(),
            temp_blocks: Vec::new(),
            deferred_return_locals: Vec::new(),
            mode_candidates: Vec::new(),
        };
        let statements = &ast.functions[0].body.statements;
        lower.statement(&statements[0], 0, &mut Vec::new()).unwrap();
        lower.active_loans.insert(
            LocalId(0),
            vec![ActiveLoan {
                permission: LoanPermission::Read,
                span: Span {
                    file: "loan_write.n".into(),
                    start: 18,
                    end: 19,
                },
                loan: LoanId(18),
                parent: None,
                compound_assignment: false,
            }],
        );
        let CompileError::Rejected(errors) = lower
            .statement(&statements[1], 0, &mut Vec::new())
            .unwrap_err()
        else {
            panic!("expected active read loan conflict")
        };
        assert_eq!(
            errors[0].subcause.as_deref(),
            Some("active_borrow_conflict")
        );
        assert_eq!(
            errors[0].data["requested_access"],
            diagnostic::DataValue::String("write".into())
        );
        errors[0].validate("loan_write.n", source).unwrap();
    }

    #[test]
    fn dead_declaration_and_reinitialization_restore_all_lowering_state() {
        let source = "fn Main() {\n    var r = MockOpen(1);\n    Drop(r);\n    {\n        return;\n        let s = r;\n        let x = 1;\n        r = MockOpen(2);\n        return;\n    }\n    Inspect(r);\n}\n";
        let ast = parse::parse("dead_state.n", source).unwrap();
        let inferred =
            crate::check::check_for_compilation(&ast.span.file, source, &ast, false).inferred;
        let signatures = HashMap::new();
        let mut lower = Lower {
            instructions: Vec::new(),
            pending_charges: Vec::new(),
            blocks: Vec::new(),
            locals: Vec::new(),
            visible: HashMap::new(),
            scopes: vec![Vec::new()],
            scope: ScopeId(0),
            next_scope: 1,
            scope_parents: vec![None],
            next_temp: 0,
            result: Ty::Unit,
            terminated: None,
            user_functions: &signatures,
            inferred_bindings: &inferred,
            name_spans: Vec::new(),
            consumed_at: HashMap::new(),
            maybe_consumed_at: HashMap::new(),
            consumed_undo: Vec::new(),
            dead_depth: 0,
            emission: EmissionMode::Live,
            literal_true_loops: false,
            poisoned: HashSet::new(),
            tokens: Arc::new(lex::lex("dead_state.n", source).unwrap()),
            defer_ownership: false,
            active_loans: HashMap::new(),
            loop_stack: Vec::new(),
            match_boundaries: Vec::new(),
            retained_locals: HashMap::new(),
            retained_temps: HashMap::new(),
            requested_receiver: None,
            live_temps: HashSet::new(),
            live_locals: HashSet::new(),
            local_head: None,
            local_history: Vec::new(),
            temp_loans: HashMap::new(),
            temp_undo: Vec::new(),
            temp_block_entry: EdgeState::default(),
            temp_blocks: Vec::new(),
            deferred_return_locals: Vec::new(),
            mode_candidates: Vec::new(),
        };
        let mut diagnostics = Vec::new();
        lower
            .block(&ast.functions[0].body.statements, 0, &mut diagnostics)
            .unwrap();
        assert_eq!(
            diagnostics
                .iter()
                .map(|d| d.code.as_str())
                .collect::<Vec<_>>(),
            ["NX-E0202", "NX-E0202"]
        );
        assert_eq!(lower.locals.len(), 1);
        assert_eq!(lower.name_spans.len(), 1);
        assert_eq!(
            lower.name_spans[0].start,
            source.find("r = MockOpen").unwrap() as u64
        );
        assert_eq!(lower.visible.get("r").unwrap(), &vec![LocalId(0)]);
        assert!(lower.visible.get("s").is_none_or(Vec::is_empty));
        assert!(lower.visible.get("x").is_none_or(Vec::is_empty));
        assert_eq!(lower.scopes[0], vec!["r"]);
        assert!(lower.poisoned.is_empty());
        assert!(lower.terminated.is_some());
        assert_eq!(
            lower.live_temps,
            HashSet::from([match lower.terminated.unwrap() {
                Termination::Return(id) => id,
                Termination::Diverged => unreachable!(),
            }])
        );
        assert_eq!(
            lower.consumed_at.get(&LocalId(0)).unwrap()[0].0.start,
            source.find("Drop(r)").unwrap() as u64
        );
        assert_eq!(lower.dead_depth, 0);
        assert!(lower.consumed_undo.is_empty());
    }

    #[test]
    fn poisoned_binding_in_rejected_block_has_no_cleanup_instruction() {
        let source =
            "fn Main() {\n    {\n        let r = MockOpen(1);\n        let s = r;\n    }\n}\n";
        let ast = parse::parse("poison.n", source).unwrap();
        let inferred =
            crate::check::check_for_compilation(&ast.span.file, source, &ast, false).inferred;
        let signatures = HashMap::new();
        let mut lower = Lower {
            instructions: Vec::new(),
            pending_charges: Vec::new(),
            blocks: Vec::new(),
            locals: Vec::new(),
            visible: HashMap::new(),
            scopes: vec![Vec::new()],
            scope: ScopeId(0),
            next_scope: 1,
            scope_parents: vec![None],
            next_temp: 0,
            result: Ty::Unit,
            terminated: None,
            user_functions: &signatures,
            inferred_bindings: &inferred,
            name_spans: Vec::new(),
            consumed_at: HashMap::new(),
            maybe_consumed_at: HashMap::new(),
            consumed_undo: Vec::new(),
            dead_depth: 0,
            emission: EmissionMode::Live,
            literal_true_loops: false,
            poisoned: HashSet::new(),
            tokens: Arc::new(lex::lex("poison.n", source).unwrap()),
            defer_ownership: false,
            active_loans: HashMap::new(),
            loop_stack: Vec::new(),
            match_boundaries: Vec::new(),
            retained_locals: HashMap::new(),
            retained_temps: HashMap::new(),
            requested_receiver: None,
            live_temps: HashSet::new(),
            live_locals: HashSet::new(),
            local_head: None,
            local_history: Vec::new(),
            temp_loans: HashMap::new(),
            temp_undo: Vec::new(),
            temp_block_entry: EdgeState::default(),
            temp_blocks: Vec::new(),
            deferred_return_locals: Vec::new(),
            mode_candidates: Vec::new(),
        };
        let mut diagnostics = Vec::new();
        lower
            .block(&ast.functions[0].body.statements, 0, &mut diagnostics)
            .unwrap();
        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].code, "NX-E0201");
        assert!(lower.poisoned.contains(&LocalId(1)));
        assert!(
            !lower
                .instructions
                .iter()
                .any(|ins| matches!(ins.op, Op::DropLocal(LocalId(1))))
        );
    }

    #[test]
    fn poisoned_call_target_preserves_original_lowering_diagnostic() {
        let source = "fn Main() {\n    let r = MockOpen(1);\n    let s = r;\n    s();\n}\n";
        let ast = parse::parse("poison.n", source).unwrap();
        let mut inferred =
            crate::check::check_for_compilation(&ast.span.file, source, &ast, false).inferred;
        let statements = &ast.functions[0].body.statements;
        let Statement::Let { value: first, .. } = &statements[0] else {
            panic!("first")
        };
        let Statement::Let { value: second, .. } = &statements[1] else {
            panic!("second")
        };
        let Statement::Expr { value: third, .. } = &statements[2] else {
            panic!("third")
        };
        inferred.bindings.insert(
            source.find("r =").unwrap() as u64,
            ResolvedType::Concrete(Type::MockResource),
        );
        inferred.bindings.insert(
            source.find("s =").unwrap() as u64,
            ResolvedType::Concrete(Type::MockResource),
        );
        inferred.expressions.insert(
            (first.span().start, first.span().end),
            ResolvedType::Concrete(Type::MockResource),
        );
        inferred.expressions.insert(
            (second.span().start, second.span().end),
            ResolvedType::Concrete(Type::MockResource),
        );
        inferred.expressions.insert(
            (third.span().start, third.span().end),
            ResolvedType::Unresolved,
        );
        let Expr::Call { args, .. } = first else {
            panic!("open")
        };
        inferred.expressions.insert(
            (args[0].value.span().start, args[0].value.span().end),
            ResolvedType::Concrete(Type::I64),
        );
        let signatures = HashMap::new();
        let mut lower = Lower {
            instructions: Vec::new(),
            pending_charges: Vec::new(),
            blocks: Vec::new(),
            locals: Vec::new(),
            visible: HashMap::new(),
            scopes: vec![Vec::new()],
            scope: ScopeId(0),
            next_scope: 1,
            scope_parents: vec![None],
            next_temp: 0,
            result: Ty::Unit,
            terminated: None,
            user_functions: &signatures,
            inferred_bindings: &inferred,
            name_spans: Vec::new(),
            consumed_at: HashMap::new(),
            maybe_consumed_at: HashMap::new(),
            consumed_undo: Vec::new(),
            dead_depth: 0,
            emission: EmissionMode::Live,
            literal_true_loops: false,
            poisoned: HashSet::new(),
            tokens: Arc::new(lex::lex("poison.n", source).unwrap()),
            defer_ownership: false,
            active_loans: HashMap::new(),
            loop_stack: Vec::new(),
            match_boundaries: Vec::new(),
            retained_locals: HashMap::new(),
            retained_temps: HashMap::new(),
            requested_receiver: None,
            live_temps: HashSet::new(),
            live_locals: HashSet::new(),
            local_head: None,
            local_history: Vec::new(),
            temp_loans: HashMap::new(),
            temp_undo: Vec::new(),
            temp_block_entry: EdgeState::default(),
            temp_blocks: Vec::new(),
            deferred_return_locals: Vec::new(),
            mode_candidates: Vec::new(),
        };
        let mut diagnostics = Vec::new();
        lower
            .block(&ast.functions[0].body.statements, 0, &mut diagnostics)
            .unwrap();
        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].code, "NX-E0201");
    }
}
