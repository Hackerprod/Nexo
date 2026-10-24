use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use super::model::{
    self, Access, BasicBlock, Binary, BlockId, Function, FunctionId, FunctionTable, Instruction,
    Intrinsic, LoanId, LoanPermission, Local, LocalId, Op, OriginId, Parameter, ParameterMode,
    Program, ScopeId, TempId, Terminator, Ty,
};
use crate::diagnostic::{self, Diagnostic, PROFILE_ID, Span};
use crate::frontend::lex;
use crate::frontend::parse::{self, Ast, Expr, Mode, Statement, Type};

#[cfg(test)]
pub(super) static LOAN_WORK: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
#[cfg(test)]
fn count_loan_work() {
    LOAN_WORK.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
}
#[cfg(not(test))]
fn count_loan_work() {}

#[derive(Debug)]
// InvalidIr is a guard on an invariant, never a source diagnostic. The
// source path is parse -> names/limits/types -> lowering -> verify_function
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
// binary/index operand flow, exclusive loans or result propagation.
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

fn binary_result_type(op: &str) -> Result<Ty, CompileError> {
    match op {
        "+" | "-" => Ok(Ty::I64),
        "<" | "==" => Ok(Ty::Bool),
        _ => Err(CompileError::InvalidIr("binary operation")),
    }
}

fn type_label(ty: &Ty) -> String {
    match ty {
        Ty::I64 => "I64".into(),
        Ty::Bool => "Bool".into(),
        Ty::Unit => "()".into(),
        Ty::Array(inner) => format!("Array<{}>", type_label(inner)),
        Ty::MockResource => "MockResource".into(),
        Ty::MockBlockingResource => "MockBlockingResource".into(),
        Ty::Never => "Never".into(),
    }
}

fn type_mismatch(
    span: &Span,
    expected: &Ty,
    found: &Ty,
    expected_span: Option<&Span>,
) -> CompileError {
    let mut d = crate::check::names::diagnostic(
        "NX-E0102",
        Some("type_mismatch"),
        diagnostic::Stage::Types,
        diagnostic::Operation::TypesCheck,
        span.clone(),
    );
    d.data.insert(
        "expected_type".into(),
        diagnostic::DataValue::String(type_label(expected)),
    );
    d.data.insert(
        "found_type".into(),
        diagnostic::DataValue::String(type_label(found)),
    );
    if let Some(span) = expected_span {
        d.related_spans.push(diagnostic::RelatedSpan {
            role: diagnostic::RelatedRole::ExpectedType,
            span: span.clone(),
        });
    }
    d.related_spans.push(diagnostic::RelatedSpan {
        role: diagnostic::RelatedRole::FoundType,
        span: span.clone(),
    });
    crate::check::names::sort_related(&mut d);
    CompileError::Rejected(vec![d])
}

/// Produces an opaque, verified artifact. No interpreter entry accepts an AST.
pub fn compile_scalar(file: &str, source: &str) -> Result<Program, CompileError> {
    compile_scalar_with_entry(file, source, true)
}

/// Check and verify a library without requiring an entry point.
pub fn check_scalar(file: &str, source: &str) -> Result<Program, CompileError> {
    compile_scalar_with_entry(file, source, false)
}

fn compile_scalar_with_entry(
    file: &str,
    source: &str,
    run_entry: bool,
) -> Result<Program, CompileError> {
    let ast = parse::parse(file, source).map_err(|d| CompileError::Rejected(vec![d]))?;
    let checked = crate::check::check_for_compilation(file, source, &ast, run_entry);
    if let Err(errors) = &checked
        && errors
            .iter()
            .any(|d| d.feature_id.as_deref() == Some("implementation_limit"))
    {
        return Err(CompileError::Rejected(errors.clone()));
    }
    let lowered = std::thread::scope(|scope| {
        std::thread::Builder::new()
            .stack_size(16 * 1024 * 1024)
            .spawn_scoped(scope, || compile_ast(&ast, source))
            .map_err(|_| CompileError::InvalidIr("lowering worker unavailable"))?
            .join()
            .map_err(|_| CompileError::InvalidIr("lowering worker panicked"))?
    });
    if let Err(errors) = checked {
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
        return Err(CompileError::Rejected(errors));
    }
    lowered
}

pub(super) fn compile_ast(ast: &Ast, source_text: &str) -> Result<Program, CompileError> {
    #[cfg(test)]
    let _allocation_capture = crate::test_alloc::capture_for(&ast.span.file);
    let source_text: std::sync::Arc<str> = std::sync::Arc::from(source_text);
    let tokens = Arc::new(
        lex::lex(&ast.span.file, &source_text).map_err(|d| CompileError::Rejected(vec![d]))?,
    );
    let mut user_functions = HashMap::new();
    for f in &ast.functions {
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
        let result = f
            .result
            .as_ref()
            .map(lower_type)
            .transpose()?
            .unwrap_or(Ty::Unit);
        user_functions.insert(f.name.clone(), (params, result));
    }
    let mut functions: Vec<_> = ast.functions.iter().collect();
    functions.sort_by(|a, b| a.name.cmp(&b.name));
    let mut entries = Vec::with_capacity(functions.len());
    let mut diagnostics = Vec::new();
    for (index, source) in functions.into_iter().enumerate() {
        let result = match source.result.as_ref() {
            None | Some(Type::Unit) => Ty::Unit,
            Some(Type::I64) => Ty::I64,
            Some(Type::Bool) => Ty::Bool,
            Some(ty) => lower_type(ty)?,
        };
        let mut lower = Lower {
            instructions: Vec::new(),
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
            name_spans: Vec::new(),
            consumed_at: HashMap::new(),
            maybe_consumed_at: HashMap::new(),
            consumed_undo: Vec::new(),
            dead_depth: 0,
            emission: EmissionMode::Live,
            poisoned: HashSet::new(),
            tokens: Arc::clone(&tokens),
            defer_ownership: {
                let first = tokens.partition_point(|t| t.span.start < source.span.start);
                tokens[first..]
                    .iter()
                    .take_while(|t| t.span.start < source.span.end)
                    .any(|t| t.text == "while")
            },
            active_loans: HashMap::new(),
            live_temps: HashSet::new(),
            temp_undo: Vec::new(),
        };
        let mut parameters = Vec::new();
        for (p, (_, mode, ty)) in source.params.iter().zip(&user_functions[&source.name].0) {
            let id = LocalId(lower.locals.len());
            lower.locals.push(Local {
                id,
                name: p.name.clone(),
                ty: ty.clone(),
                mutable: *mode == ParameterMode::Inout,
                scope: ScopeId(0),
                parameter_mode: Some(*mode),
            });
            let token_index = lower
                .tokens
                .partition_point(|t| t.span.start < p.span.start);
            let name_span = lower.tokens[token_index..]
                .iter()
                .take_while(|t| t.span.start < p.span.end)
                .find(|t| {
                    t.span.start >= p.span.start && t.span.end <= p.span.end && t.text == p.name
                })
                .map(|t| t.span.clone())
                .ok_or(CompileError::InvalidIr("parameter name token"))?;
            lower.name_spans.push(name_span);
            lower.visible.entry(p.name.clone()).or_default().push(id);
            lower.scopes[0].push(p.name.clone());
            parameters.push(Parameter {
                local: id,
                mode: *mode,
            });
        }
        let before_body = diagnostics.len();
        lower.block(&source.body.statements, 0, &mut diagnostics)?;
        let missing_branch_value = diagnostics[before_body..]
            .iter()
            .any(|d| d.code == "NX-E1302" && d.subcause.as_deref() == Some("missing_branch_value"));
        if missing_branch_value {
            diagnostics.retain(|d| d.stage != diagnostic::Stage::Types);
        }
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
            continue;
        }
        let span = source.body.span.clone();
        let all_paths_return = lower.terminated == Some(Termination::Diverged);
        let value = if all_paths_return {
            None
        } else if let Some(Termination::Return(value)) = lower.terminated {
            Some(value)
        } else {
            if result != Ty::Unit {
                return Err(CompileError::InvalidIr("control-flow return analysis"));
            }
            Some(
                lower
                    .emit(Op::ConstUnit, Ty::Unit, span.clone(), Access::Own, false)
                    .ok_or(CompileError::InvalidIr("function return flow"))?,
            )
        };
        let (entry, other_blocks) = if all_paths_return {
            let mut all = lower.blocks;
            if all.is_empty() {
                return Err(CompileError::InvalidIr("return flow"));
            }
            (all.remove(0), all)
        } else if lower.blocks.is_empty() {
            (
                BasicBlock {
                    id: BlockId(0),
                    instructions: lower.instructions,
                    terminator: Terminator::Return { value, span },
                },
                Vec::new(),
            )
        } else {
            lower.blocks.push(BasicBlock {
                id: BlockId(lower.blocks.len()),
                instructions: lower.instructions,
                terminator: Terminator::Return { value, span },
            });
            let mut all = lower.blocks;
            (all.remove(0), all)
        };
        let mut function = Function {
            id: FunctionId(source.name.clone()),
            table_index: index,
            name: source.name.clone(),
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
            cleanup: Vec::new(),
        };
        function.cleanup = function.locals.iter().rev().map(|l| l.id).collect();
        function.content_hash = model::content_hash(&function);
        let flow_errors = if lower.defer_ownership {
            let mut ownership_flow_steps = 0_u64;
            match super::ownership_flow::analyze(
                &function,
                &lower.name_spans,
                &tokens,
                &mut ownership_flow_steps,
            ) {
                Ok(errors) => errors,
                Err(CompileError::Rejected(errors)) => {
                    diagnostics.extend(errors.into_iter().map(|d| *d));
                    continue;
                }
                Err(other) => return Err(other),
            }
        } else {
            Vec::new()
        };
        if !flow_errors.is_empty() {
            diagnostics.extend(flow_errors);
            continue;
        }
        super::verify::verify_function(&function).map_err(CompileError::InvalidIr)?;
        entries.push(function);
    }
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
        functions: FunctionTable { entries },
        profile: PROFILE_ID,
    };
    super::verify::verify_program(&program).map_err(CompileError::InvalidIr)?;
    Ok(program)
}

struct Lower<'a> {
    instructions: Vec<Instruction>,
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
    name_spans: Vec<Span>,
    consumed_at: HashMap<LocalId, Vec<(Span, bool)>>,
    maybe_consumed_at: HashMap<LocalId, Vec<(Span, bool)>>,
    consumed_undo: Vec<AvailabilityUndo>,
    dead_depth: usize,
    emission: EmissionMode,
    poisoned: HashSet<LocalId>,
    tokens: Arc<Vec<lex::Token>>,
    defer_ownership: bool,
    active_loans: HashMap<LocalId, Vec<ActiveLoan>>,
    live_temps: HashSet<TempId>,
    temp_undo: Vec<(TempId, bool)>,
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
type PreparedCallArguments = (Vec<(usize, TempId)>, Vec<(LocalId, LoanId, Span)>, bool);
#[derive(Clone)]
struct ActiveLoan {
    permission: LoanPermission,
    span: Span,
    loan: LoanId,
    parent: Option<LocalId>,
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
        crate::check::names::sort_related(d);
        d.related_spans
            .dedup_by(|a, b| a.role == b.role && a.span == b.span);
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
impl Lower<'_> {
    fn emitted(&mut self, op: Op, ty: Ty, span: Span, access: Access, effect_io: bool) -> Flow {
        let temp = self.emit(op, ty.clone(), span, access, effect_io);
        Flow::Value { temp, ty }
    }
    fn place_ty(&self, expr: &Expr) -> Option<Ty> {
        match expr {
            Expr::Name(name, _) => self
                .visible
                .get(name)
                .and_then(|ids| ids.last())
                .map(|id| self.locals[id.0].ty.clone()),
            Expr::Group(inner, _) => self.place_ty(inner),
            Expr::Index { target, .. } => match self.place_ty(target)? {
                Ty::Array(item) => Some(*item),
                _ => None,
            },
            _ => None,
        }
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
            Op::IndexValue { base, index } => {
                self.temp_live(*base, false);
                self.temp_live(*index, false);
            }
            Op::Binary { left, right, .. } => {
                self.temp_live(*left, false);
                self.temp_live(*right, false);
            }
            Op::WritePlace { value, .. } | Op::DropValue(value) => self.temp_live(*value, false),
            Op::WriteIndex { indexes, value, .. } => {
                for &id in indexes {
                    self.temp_live(id, false);
                }
                self.temp_live(*value, false);
            }
            Op::CallDirect { args, .. } => {
                for &(_, id) in args {
                    self.temp_live(id, false);
                }
            }
            _ => {}
        }
    }
    fn union_slot(
        &mut self,
        slot: &mut Option<LocalId>,
        value: TempId,
        ty: Ty,
        span: &Span,
    ) -> Result<(), CompileError> {
        let id = if let Some(id) = slot {
            if self.locals[id.0].ty != ty && self.locals[id.0].ty != Ty::Never && ty != Ty::Never {
                return Err(CompileError::InvalidIr("branch value type"));
            }
            if self.locals[id.0].ty == Ty::Never {
                self.locals[id.0].ty = ty;
            }
            *id
        } else {
            let id = LocalId(self.locals.len());
            self.locals.push(Local {
                id,
                name: String::new(),
                ty,
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
    fn if_expression(
        &mut self,
        condition: &Expr,
        then_block: &parse::Block,
        otherwise: Option<&parse::Else>,
        span: &Span,
        depth: usize,
        expected: Option<&Ty>,
    ) -> Result<Flow, CompileError> {
        let Some(otherwise) = otherwise else {
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
        let test_flow = self.expr(condition, depth + 1, Some(&Ty::Bool))?;
        let condition_diverges = test_flow.diverged();
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
        let mut diagnostics = Vec::new();
        let mut dependent = false;
        let mut slot = None;
        let mut inferred_ty = expected.cloned();
        self.dead_depth += 1;
        let then_value =
            match self.scoped_value_block(then_block, expected, span, depth, &mut diagnostics) {
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
        if let Some(Flow::Value { temp, ty }) = then_value {
            if inferred_ty.as_ref().is_none_or(|prior| *prior == Ty::Never) {
                inferred_ty = Some(ty.clone());
            }
            if let Some(temp) = temp {
                self.union_slot(&mut slot, temp, ty, &then_block.span)?;
            }
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
                    }
                } else {
                    Terminator::Jump {
                        target: BlockId(usize::MAX),
                        span: then_block.span.clone(),
                    }
                }),
            )
        };
        let then_state = self.restore_availability(checkpoint);
        self.restore_temps(temp_checkpoint);
        self.dead_depth -= 1;
        let else_start = BlockId(self.blocks.len());
        if let Some(branch) = branch
            && let Terminator::BranchBool { no, .. } = &mut self.blocks[branch.0].terminator
        {
            *no = else_start;
        }
        self.dead_depth += 1;
        let else_value_result = match otherwise {
            parse::Else::Block(block) => {
                self.scoped_value_block(block, expected, span, depth, &mut diagnostics)
            }
            parse::Else::If(expr) => self.expr(expr, depth + 1, expected).map(Some),
        };
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
        if let Some(Flow::Value { temp, ty }) = else_value {
            if inferred_ty.as_ref().is_none_or(|prior| *prior == Ty::Never) {
                inferred_ty = Some(ty.clone());
            }
            if let Some(temp) = temp {
                self.union_slot(&mut slot, temp, ty, else_span)?;
            }
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
                    }
                } else {
                    Terminator::Jump {
                        target: BlockId(usize::MAX),
                        span: else_span.clone(),
                    }
                }),
            )
        };
        let else_state = self.restore_availability(checkpoint);
        self.restore_temps(temp_checkpoint);
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
                ty: inferred_ty.filter(|ty| *ty != Ty::Never),
            });
        }
        let Some(slot) = slot else {
            if self.emission == EmissionMode::Analyze {
                return Ok(Flow::Value {
                    temp: None,
                    ty: inferred_ty.unwrap_or(Ty::Never),
                });
            }
            return Err(CompileError::InvalidIr("branch value slot"));
        };
        let ty = self.locals[slot.0].ty.clone();
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
    fn while_statement(
        &mut self,
        condition: &Expr,
        body: &parse::Block,
        span: &Span,
        depth: usize,
        diagnostics: &mut Vec<Diagnostic>,
    ) -> Result<(), CompileError> {
        let entry_emission = self.emission;
        let prior_locals = self.locals.len();
        let preheader = self.finish_block(Terminator::Jump {
            target: BlockId(self.blocks.len() + 1),
            span: span.clone(),
        });
        // The condition may itself create branches. Its first block is the
        // loop header; every iteration must evaluate the entire condition.
        let condition_header = BlockId(self.blocks.len());
        let checkpoint = self.consumed_undo.len();
        let temp_checkpoint = self.temp_undo.len();
        self.dead_depth += 1;
        let test_flow = self.expr(condition, 0, Some(&Ty::Bool))?;
        let condition_diverges = test_flow.diverged();
        if condition_diverges || entry_emission == EmissionMode::Analyze {
            self.terminated = None;
            self.emission = EmissionMode::Analyze;
            let before_errors = diagnostics.len();
            self.scoped_block(body, depth, diagnostics)?;
            let body_continues = self.terminated.is_none() && !condition_diverges;
            let body_state = self.restore_availability(checkpoint);
            self.restore_temps(temp_checkpoint);
            if body_continues && diagnostics.len() == before_errors {
                let affected: HashSet<u64> = body_state
                    .iter()
                    .filter(|(id, state)| id.0 < prior_locals && *state != &self.availability(**id))
                    .map(|(id, _)| self.name_spans[id.0].start)
                    .collect();
                if !affected.is_empty() {
                    self.merge_availability(body_state, HashMap::new(), true, true, prior_locals);
                    let second_checkpoint = self.consumed_undo.len();
                    let second_temps = self.temp_undo.len();
                    let locals = self.locals.len();
                    let scopes = self.next_scope;
                    self.terminated = None;
                    let before_second_errors = diagnostics.len();
                    let second = self.expr(condition, 0, Some(&Ty::Bool)).and_then(|flow| {
                        if flow.diverged() {
                            Ok(())
                        } else {
                            self.terminated = None;
                            self.scoped_block(body, depth, diagnostics)
                        }
                    });
                    self.restore_availability(second_checkpoint);
                    self.restore_temps(second_temps);
                    self.locals.truncate(locals);
                    self.name_spans.truncate(locals);
                    self.next_scope = scopes;
                    self.scope_parents.truncate(scopes);
                    for d in &mut diagnostics[before_second_errors..] {
                        add_backedge_to_diagnostic(d, &body.span, &affected);
                    }
                    if let Err(error) = second {
                        self.dead_depth -= 1;
                        self.terminated = condition_diverges.then_some(Termination::Diverged);
                        return Err(add_backedge_to_error(error, &body.span, &affected));
                    }
                }
            }
            self.dead_depth -= 1;
            self.terminated = condition_diverges.then_some(Termination::Diverged);
            return Ok(());
        }
        if test_flow.ty() != Some(&Ty::Bool) {
            return Err(CompileError::InvalidIr("while condition type"));
        }
        let condition_state = self.restore_availability(checkpoint);
        let test = test_flow
            .temp()
            .ok_or(CompileError::InvalidIr("while condition flow"))?;
        let branch = self.finish_block(Terminator::BranchBool {
            condition: test,
            yes: BlockId(self.blocks.len() + 1),
            no: BlockId(usize::MAX),
            span: condition.span().clone(),
        });
        // The complete condition has one ownership state. Both outcomes
        // inherit its joined effects, regardless of the values in its arms.
        self.restore_temps(temp_checkpoint);
        // Both successors observe the effects of evaluating the condition.
        // In particular, a complete write in an `if` expression can make an
        // owner available before the first body instruction.
        for (id, state) in &condition_state {
            self.set_availability(*id, state.clone());
        }
        self.scoped_block(body, depth, diagnostics)?;
        self.emission = entry_emission;
        let body_continues = self.terminated.is_none();
        let body_return = self.terminated.take();
        let body_end = if body_return == Some(Termination::Diverged) {
            None
        } else if let Some(Termination::Return(value)) = body_return {
            Some(self.finish_block(Terminator::Return {
                value: Some(value),
                span: body.span.clone(),
            }))
        } else {
            Some(self.finish_block(Terminator::Jump {
                target: condition_header,
                span: body.span.clone(),
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
        let _ = (prior_locals, body_continues);
        let exit = BlockId(self.blocks.len());
        if let Terminator::BranchBool { no, .. } = &mut self.blocks[branch.0].terminator {
            *no = exit;
        }
        let _ = (preheader, body_end);
        Ok(())
    }
    fn if_statement(
        &mut self,
        condition: &Expr,
        then_block: &parse::Block,
        otherwise: Option<&Statement>,
        span: &Span,
        depth: usize,
        diagnostics: &mut Vec<Diagnostic>,
    ) -> Result<(), CompileError> {
        self.if_statement_form(
            condition,
            then_block,
            otherwise.map(StatementElse::Statement),
            span,
            depth,
            diagnostics,
        )
    }
    fn if_statement_form(
        &mut self,
        condition: &Expr,
        then_block: &parse::Block,
        otherwise: Option<StatementElse<'_>>,
        span: &Span,
        depth: usize,
        diagnostics: &mut Vec<Diagnostic>,
    ) -> Result<(), CompileError> {
        let entry_emission = self.emission;
        let condition_flow = self.expr(condition, 0, Some(&Ty::Bool))?;
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
        self.scoped_block(then_block, depth, diagnostics)?;
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
                        }
                    } else {
                        Terminator::Jump {
                            target: BlockId(usize::MAX),
                            span: then_block.span.clone(),
                        }
                    }),
                )
            };
        let then_state = self.restore_availability(checkpoint);
        self.restore_temps(temp_checkpoint);
        self.dead_depth -= 1;
        let else_start = BlockId(self.blocks.len());
        if let Some(branch) = branch
            && let Terminator::BranchBool { no, .. } = &mut self.blocks[branch.0].terminator
        {
            *no = else_start;
        }
        let (else_state, else_continues, else_end) = if let Some(otherwise) = otherwise {
            self.dead_depth += 1;
            let else_span = match otherwise {
                StatementElse::Statement(stmt) => statement_span(stmt),
                StatementElse::Else(parse::Else::Block(block)) => block.span.clone(),
                StatementElse::Else(parse::Else::If(expr)) => expr.span().clone(),
            };
            match otherwise {
                StatementElse::Statement(Statement::Expr {
                    value:
                        Expr::If {
                            condition,
                            then_block,
                            otherwise,
                            span,
                        },
                    ..
                }) => {
                    self.if_statement_form(
                        condition,
                        then_block,
                        otherwise.as_deref().map(StatementElse::Else),
                        span,
                        depth + 1,
                        diagnostics,
                    )?;
                }
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
                    self.if_statement_form(
                        condition,
                        then_block,
                        otherwise.as_deref().map(StatementElse::Else),
                        span,
                        depth + 1,
                        diagnostics,
                    )?;
                }
                StatementElse::Statement(stmt) => self.statement(stmt, depth + 1, diagnostics)?,
                StatementElse::Else(parse::Else::Block(block)) => {
                    self.scoped_block(block, depth + 1, diagnostics)?
                }
            }
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
                        }
                    } else {
                        Terminator::Jump {
                            target: BlockId(usize::MAX),
                            span: else_span,
                        }
                    }),
                )
            };
            let state = self.restore_availability(checkpoint);
            self.restore_temps(temp_checkpoint);
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
        } else if otherwise.is_none()
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
        if let Terminator::Return {
            value: Some(value),
            span,
        } = &terminator
        {
            let mut pending: Vec<_> = self
                .live_temps
                .iter()
                .copied()
                .filter(|id| *id != *value)
                .collect();
            pending.sort_by_key(|id| std::cmp::Reverse(id.0));
            for id in pending {
                self.statement_op(Op::DropValue(id), span.clone(), Access::Own, false);
            }
            self.temp_live(*value, false);
        } else if let Terminator::BranchBool { condition, .. } = &terminator {
            self.temp_live(*condition, false);
        }
        super::count_cfg_work(1);
        let id = BlockId(self.blocks.len());
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
        if active.permission == LoanPermission::Read && requested == "read" {
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
            diagnostic::DataValue::String(requested.into()),
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
            message: "Acceso propietario inválido.".into(),
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
        self.conflict(id, place.span().clone(), requested)
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
            primary_span: Some(root.span().clone()),
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
        self.consume_op_temps(&op);
        count_loan_work();
        super::count_cfg_work(1);
        let id = TempId(self.next_temp);
        let origin = OriginId(self.instructions.len());
        self.next_temp += 1;
        self.instructions.push(Instruction {
            op,
            output: Some(id),
            ty: Some(ty),
            origin,
            span,
            scope: self.scope,
            access,
            effect_io,
            tick: true,
        });
        self.temp_live(id, true);
        Some(id)
    }
    fn statement_op(&mut self, op: Op, span: Span, access: Access, effect_io: bool) {
        if self.emission == EmissionMode::Analyze {
            return;
        }
        self.consume_op_temps(&op);
        count_loan_work();
        super::count_cfg_work(1);
        let id = self.instructions.len();
        self.instructions.push(Instruction {
            op,
            output: None,
            ty: None,
            origin: OriginId(id),
            span,
            scope: self.scope,
            access,
            effect_io,
            tick: true,
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
    fn call_argument(
        &mut self,
        arg: &parse::Argument,
        mode: ParameterMode,
        expected: &Ty,
        depth: usize,
        retain_place_loan: bool,
    ) -> Result<(Flow, Option<LocalId>), CompileError> {
        count_loan_work();
        if !retain_place_loan {
            return self
                .expr_mode(
                    &arg.value,
                    depth + 1,
                    (expected != &Ty::Never).then_some(expected),
                    mode == ParameterMode::Read,
                )
                .map(|flow| (flow, None));
        }
        let place = if mode == ParameterMode::Inout {
            match &arg.value {
                Expr::Unary {
                    mode: Mode::Inout,
                    value,
                    ..
                } => Some(value.as_ref()),
                _ => return Err(CompileError::InvalidIr("edit mode passed checker")),
            }
        } else if mode == ParameterMode::Read {
            Some(&arg.value)
        } else {
            None
        };
        if let Some(place) = place
            && let Ok((root, index_exprs)) = place_parts(place)
            && let Some(name) = place_name(root)
        {
            let local = self.get(name)?;
            let indexed_place = !index_exprs.is_empty();
            if mode == ParameterMode::Inout {
                if self.locals[local.0].parameter_mode == Some(ParameterMode::Read) {
                    return Err(self.permission_error(local, root.span().clone(), "borrow_inout"));
                }
                if !self.locals[local.0].mutable {
                    return Err(self.immutable_write(root, local));
                }
            }
            let requested = if mode == ParameterMode::Inout {
                "borrow_inout"
            } else {
                "read"
            };
            // ADR §14.2 marks the complete selected place in P07 (`a[1]`).
            let mut indexes = Vec::with_capacity(index_exprs.len());
            let mut ty = self.locals[local.0].ty.clone();
            for index_expr in index_exprs {
                let index_flow = self.expr(index_expr, depth + 1, Some(&Ty::I64))?;
                if index_flow.diverged() {
                    return Ok((
                        Flow::Diverged {
                            ty: Some(expected.clone()),
                        },
                        None,
                    ));
                }
                if index_flow.ty() != Some(&Ty::I64) {
                    return Err(CompileError::InvalidIr("loan index type"));
                }
                ty = match ty {
                    Ty::Array(child) => *child,
                    Ty::Never => Ty::Never,
                    _ => return Err(CompileError::InvalidIr("loan target type")),
                };
                if let Some(id) = index_flow.temp() {
                    indexes.push(id);
                }
            }
            self.check_place_access(local, root, place, requested, indexed_place)?;
            if ty == Ty::Never {
                return Ok((
                    Flow::Value {
                        temp: None,
                        ty: expected.clone(),
                    },
                    None,
                ));
            }
            let permission = if mode == ParameterMode::Inout {
                LoanPermission::Inout
            } else {
                LoanPermission::Read
            };
            let loan = LoanId(arg.span.start as usize);
            let parent = if self.locals[local.0].parameter_mode == Some(ParameterMode::Inout) {
                Some(local)
            } else {
                None
            };
            let flow = self.emitted(
                Op::BeginLoan {
                    root: local,
                    indexes,
                    loan,
                    origin_span: arg.span.clone(),
                    permission,
                    parent,
                },
                ty.clone(),
                arg.value.span().clone(),
                Access::Read,
                false,
            );
            self.active_loans
                .entry(local)
                .or_default()
                .push(ActiveLoan {
                    permission,
                    span: arg.span.clone(),
                    loan,
                    parent,
                });
            return Ok((flow, Some(local)));
        }
        if mode == ParameterMode::Inout {
            return Err(CompileError::InvalidIr("edit place passed parser"));
        }
        let flow = self.expr_mode(
            &arg.value,
            depth + 1,
            (expected != &Ty::Never).then_some(expected),
            mode == ParameterMode::Read,
        )?;
        Ok((flow, None))
    }

    // A call holds each place loan until all later arguments have been
    // evaluated. Both user calls and intrinsics use this ownership boundary.
    fn prepare_call_arguments(
        &mut self,
        args: &[parse::Argument],
        params: &[(String, ParameterMode, Ty)],
        depth: usize,
        check_types: bool,
        retain_place_loans: bool,
    ) -> Result<PreparedCallArguments, CompileError> {
        let mut acquired = Vec::new();
        let result = (|| {
            let mut used = vec![false; params.len()];
            let slot_by_name: HashMap<&str, usize> = params
                .iter()
                .enumerate()
                .map(|(index, (name, _, _))| (name.as_str(), index))
                .collect();
            let mut prepared = Vec::with_capacity(args.len());
            let mut argument_errors = Vec::new();
            let mut dependent = false;
            let mut diverged = false;
            let mut next_positional = 0usize;
            for arg in args {
                if diverged {
                    self.terminated = None;
                }
                count_loan_work();
                let slot = if let Some(label) = &arg.label {
                    slot_by_name
                        .get(label.as_str())
                        .copied()
                        .ok_or(CompileError::InvalidIr("call label passed checker"))?
                } else {
                    while used.get(next_positional) == Some(&true) {
                        next_positional += 1;
                    }
                    next_positional
                };
                let (_, mode, expected) = params
                    .get(slot)
                    .ok_or(CompileError::InvalidIr("call parameter slot"))?;
                if used[slot] {
                    return Err(CompileError::InvalidIr("duplicate call argument"));
                }
                used[slot] = true;
                let (argument_flow, loan_root) =
                    match self.call_argument(arg, *mode, expected, depth, retain_place_loans) {
                        Ok(value) => value,
                        Err(CompileError::Rejected(errors)) => {
                            argument_errors.extend(errors);
                            continue;
                        }
                        Err(CompileError::Dependent) => {
                            dependent = true;
                            continue;
                        }
                        Err(other) => return Err(other),
                    };
                if argument_flow.diverged() {
                    diverged = true;
                    self.emission = EmissionMode::Analyze;
                    continue;
                }
                if let Some(root) = loan_root {
                    acquired.push((root, LoanId(arg.span.start as usize), arg.span.clone()));
                }
                if check_types
                    && argument_flow
                        .ty()
                        .is_some_and(|actual| *actual != *expected && *actual != Ty::Never)
                {
                    return Err(CompileError::InvalidIr("call argument type passed checker"));
                }
                if let Some(id) = argument_flow.temp() {
                    prepared.push((slot, id));
                }
            }
            if !argument_errors.is_empty() {
                return Err(CompileError::Rejected(argument_errors));
            }
            if dependent {
                return Err(CompileError::Dependent);
            }
            if diverged {
                self.terminated = Some(Termination::Diverged);
            }
            Ok((prepared, diverged))
        })();
        if result.is_err() {
            self.release_call_loans(&acquired);
        }
        result.map(|(prepared, diverged)| (prepared, acquired, diverged))
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
                last.tick = false;
            }
        }
    }
    fn scoped_block(
        &mut self,
        block: &parse::Block,
        depth: usize,
        diagnostics: &mut Vec<Diagnostic>,
    ) -> Result<(), CompileError> {
        let saved = self.scope;
        self.scope = ScopeId(self.next_scope);
        self.next_scope += 1;
        self.scope_parents.push(Some(saved));
        self.scopes.push(Vec::new());
        let body_result = self.block(&block.statements, depth + 1, diagnostics);
        if let Err(error) = body_result {
            self.leave_scope(saved)?;
            return Err(error);
        }
        let dropping = self
            .scopes
            .last()
            .ok_or(CompileError::InvalidIr("scope stack"))?
            .clone();
        for name in dropping.iter().rev() {
            let id = *self
                .visible
                .get(name)
                .and_then(|v| v.last())
                .ok_or(CompileError::InvalidIr("scope name"))?;
            if self.poisoned.contains(&id) {
                continue;
            }
            if !self.consumed_at.contains_key(&id) {
                self.statement_op(Op::DropLocal(id), block.span.clone(), Access::Own, false);
            }
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
    fn scoped_value_block(
        &mut self,
        block: &parse::Block,
        expected: Option<&Ty>,
        if_span: &Span,
        depth: usize,
        diagnostics: &mut Vec<Diagnostic>,
    ) -> Result<Option<Flow>, CompileError> {
        let saved = self.scope;
        self.scope = ScopeId(self.next_scope);
        self.next_scope += 1;
        self.scope_parents.push(Some(saved));
        self.scopes.push(Vec::new());
        let final_value_result = (|| -> Result<_, CompileError> {
            Ok(match block.statements.last() {
                Some(Statement::Value {
                    value: Some(value), ..
                }) => {
                    let evaluated = self.block_with_tail(
                        &block.statements[..block.statements.len() - 1],
                        depth + 1,
                        diagnostics,
                        |lower, prior_terminated| {
                            let saved_termination = lower.terminated.take();
                            let saved_emission = lower.emission;
                            if prior_terminated {
                                lower.emission = EmissionMode::Analyze;
                            }
                            let result = lower.expr(value, 0, expected);
                            if saved_termination.is_some() {
                                lower.terminated = saved_termination;
                                lower.emission = saved_emission;
                            }
                            result
                        },
                    )?;
                    if let (Some(expected), Some(actual)) = (expected, evaluated.ty())
                        && *actual != *expected
                        && *actual != Ty::Never
                    {
                        return Err(type_mismatch(value.span(), expected, actual, None));
                    }
                    Some(evaluated)
                }
                _ => {
                    self.block(&block.statements, depth + 1, diagnostics)?;
                    if self.terminated.is_none() {
                        let closing = Span {
                            file: block.span.file.clone(),
                            start: block.span.end - 1,
                            end: block.span.end,
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
                            span: if_span.clone(),
                        });
                        diagnostics.push(*d);
                    }
                    None
                }
            })
        })();
        let final_value = match final_value_result {
            Ok(value) => value,
            Err(error) => {
                self.leave_scope(saved)?;
                return Err(error);
            }
        };
        let dropping = self
            .scopes
            .last()
            .ok_or(CompileError::InvalidIr("scope stack"))?
            .clone();
        for name in dropping.iter().rev() {
            let id = *self
                .visible
                .get(name)
                .and_then(|v| v.last())
                .ok_or(CompileError::InvalidIr("scope name"))?;
            if !self.poisoned.contains(&id) && !self.consumed_at.contains_key(&id) {
                self.statement_op(Op::DropLocal(id), block.span.clone(), Access::Own, false);
            }
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
        let mut unreachable = None;
        for statement in statements {
            super::count_cfg_work(1);
            if self.terminated.is_some() && unreachable.is_none() {
                unreachable = Some((
                    std::mem::take(&mut self.instructions),
                    self.blocks.len(),
                    self.locals.len(),
                    self.scopes
                        .last()
                        .ok_or(CompileError::InvalidIr("scope stack"))?
                        .len(),
                    self.next_scope,
                    self.next_temp,
                    self.terminated,
                    self.emission,
                    self.consumed_undo.len(),
                    self.temp_undo.len(),
                ));
                self.dead_depth += 1;
                self.emission = EmissionMode::Analyze;
            }
            if unreachable.is_some() {
                self.terminated = None;
            }
            let prior_dead_depth = self.dead_depth;
            let prior_terminated = self.terminated;
            let prior_emission = self.emission;
            let temp_checkpoint = self.temp_undo.len();
            let before_statement_errors = diagnostics.len();
            let result = self.statement(statement, depth, diagnostics);
            if result.is_err() {
                self.dead_depth = prior_dead_depth;
                self.terminated = prior_terminated;
                self.emission = prior_emission;
                self.restore_temps(temp_checkpoint);
            }
            match result {
                Err(CompileError::Rejected(errors)) => {
                    diagnostics.extend(errors.into_iter().map(|d| *d))
                }
                Err(CompileError::Dependent) => {}
                Err(other) => return Err(other),
                Ok(()) => {}
            }
            if diagnostics.len() > before_statement_errors {
                // A rejected statement may have left an incomplete CFG. Keep
                // checking subsequent source statements directly in that case.
                self.defer_ownership = false;
            }
        }
        // An unreachable tail is already in Analyze mode. Only a return in
        // the final preceding statement still needs this mode transition.
        let was_terminated = self.terminated.is_some();
        let tail_result = tail(self, was_terminated);
        if let Some((
            instructions,
            blocks,
            locals,
            scope_len,
            next_scope,
            next_temp,
            terminated,
            emission,
            undo,
            temp_undo,
        )) = unreachable
        {
            while self.consumed_undo.len() > undo {
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
            if scope_len > scope.len() {
                return Err(CompileError::InvalidIr("scope restore length"));
            }
            for name in scope.drain(scope_len..).rev() {
                self.visible
                    .get_mut(&name)
                    .ok_or(CompileError::InvalidIr("scope name"))?
                    .pop();
            }
            for id in locals..self.locals.len() {
                self.poisoned.remove(&LocalId(id));
            }
            self.locals.truncate(locals);
            self.name_spans.truncate(locals);
            self.instructions = instructions;
            self.blocks.truncate(blocks);
            self.next_scope = next_scope;
            self.scope_parents.truncate(next_scope);
            self.next_temp = next_temp;
            self.terminated = terminated;
            self.emission = emission;
            self.restore_temps(temp_undo);
            self.dead_depth -= 1;
            if self.dead_depth == 0 {
                self.consumed_undo.clear();
            }
        }
        tail_result
    }
    fn statement(
        &mut self,
        statement: &Statement,
        depth: usize,
        diagnostics: &mut Vec<Diagnostic>,
    ) -> Result<(), CompileError> {
        match statement {
            Statement::Let {
                mutable,
                name,
                annotation,
                value,
                ..
            } => {
                let expected = annotation.as_ref().map(lower_type).transpose()?;
                let flow = match self.expr(value, 0, expected.as_ref()) {
                    Ok(value) => value,
                    Err(error @ (CompileError::Rejected(_) | CompileError::Dependent)) => {
                        let id = LocalId(self.locals.len());
                        self.name_spans.push(self.name_span(
                            statement,
                            name,
                            value.span().start,
                        )?);
                        self.locals.push(Local {
                            id,
                            name: name.clone(),
                            ty: expected.unwrap_or(Ty::Never),
                            mutable: *mutable,
                            scope: self.scope,
                            parameter_mode: None,
                        });
                        self.visible.entry(name.clone()).or_default().push(id);
                        self.scopes
                            .last_mut()
                            .ok_or(CompileError::InvalidIr("scope stack"))?
                            .push(name.clone());
                        self.poisoned.insert(id);
                        return Err(error);
                    }
                    Err(error) => return Err(error),
                };
                let id = LocalId(self.locals.len());
                self.name_spans
                    .push(self.name_span(statement, name, value.span().start)?);
                self.locals.push(Local {
                    id,
                    name: name.clone(),
                    ty: expected
                        .clone()
                        .unwrap_or_else(|| flow.ty().cloned().unwrap_or(Ty::Never)),
                    mutable: *mutable,
                    scope: self.scope,
                    parameter_mode: None,
                });
                self.visible.entry(name.clone()).or_default().push(id);
                self.scopes
                    .last_mut()
                    .ok_or(CompileError::InvalidIr("scope stack"))?
                    .push(name.clone());
                if flow.diverged() || self.emission == EmissionMode::Analyze {
                    return Ok(());
                }
                let temp = flow
                    .temp()
                    .ok_or(CompileError::InvalidIr("binding value flow"))?;
                self.statement_op(
                    Op::WritePlace {
                        local: id,
                        value: temp,
                    },
                    statement_span(statement),
                    Access::Write,
                    false,
                );
            }
            Statement::Assign { place, value, .. } => {
                let (root, index_exprs) = place_parts(place)?;
                let indexed_place = !index_exprs.is_empty();
                let name = place_name(root).ok_or(CompileError::InvalidIr("assignment root"))?;
                let id = self.get(name)?;
                if self.locals[id.0].parameter_mode == Some(ParameterMode::Read) {
                    return Err(self.permission_error(id, root.span().clone(), "write"));
                }
                if !self.locals[id.0].mutable {
                    return Err(self.immutable_write(root, id));
                }
                let mut indexes = Vec::with_capacity(index_exprs.len());
                let mut target_ty = self.locals[id.0].ty.clone();
                let mut diverged = false;
                for e in index_exprs {
                    if diverged {
                        self.terminated = None;
                    }
                    let index_flow = self.expr(e, 0, Some(&Ty::I64))?;
                    if index_flow.diverged() {
                        diverged = true;
                        self.emission = EmissionMode::Analyze;
                    }
                    if index_flow
                        .ty()
                        .is_some_and(|ty| *ty != Ty::I64 && *ty != Ty::Never)
                    {
                        return Err(CompileError::InvalidIr("index type"));
                    }
                    target_ty = match target_ty {
                        Ty::Array(child) => *child,
                        Ty::Never if self.emission == EmissionMode::Analyze => Ty::Never,
                        _ => return Err(CompileError::InvalidIr("index target type")),
                    };
                    if let Some(index) = index_flow.temp() {
                        indexes.push(index);
                    }
                }
                if diverged {
                    self.terminated = None;
                }
                let value_flow =
                    self.expr(value, 0, (target_ty != Ty::Never).then_some(&target_ty))?;
                if !indexed_place
                    && target_ty == Ty::Never
                    && !value_flow.diverged()
                    && let Some(ty) = value_flow.ty().filter(|ty| **ty != Ty::Never)
                {
                    self.locals[id.0].ty = ty.clone();
                    target_ty = ty.clone();
                }
                if diverged {
                    self.terminated = Some(Termination::Diverged);
                    return Ok(());
                }
                if value_flow.diverged() {
                    return Ok(());
                }
                self.check_place_access(id, root, root, "write", indexed_place)?;
                if !indexed_place {
                    self.mark_consumed(id, None);
                }
                if self.emission == EmissionMode::Analyze {
                    return Ok(());
                }
                let temp = value_flow
                    .temp()
                    .ok_or(CompileError::InvalidIr("assignment value flow"))?;
                if value_flow.ty().is_some_and(|ty| {
                    *ty != target_ty && *ty != Ty::Never && target_ty != Ty::Never
                }) {
                    return Err(CompileError::InvalidIr("assignment type passed checker"));
                }
                self.statement_op(
                    if !indexed_place {
                        Op::WritePlace {
                            local: id,
                            value: temp,
                        }
                    } else {
                        Op::WriteIndex {
                            local: id,
                            indexes,
                            value: temp,
                        }
                    },
                    statement_span(statement),
                    Access::Write,
                    false,
                );
            }
            Statement::Expr { value, .. } => {
                let flow = self.expr(value, 0, None)?;
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
            }
            Statement::Return { value, .. } => {
                let flow = match value {
                    Some(e) => self.expr(e, 0, Some(&self.result.clone()))?,
                    None => {
                        let span = statement_span(statement);
                        self.emitted(Op::ConstUnit, Ty::Unit, span, Access::Own, false)
                    }
                };
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
            }
            Statement::Block(block) => self.scoped_block(block, depth, diagnostics)?,
            Statement::If {
                condition,
                then_block,
                otherwise,
                span,
            } => self.if_statement(
                condition,
                then_block,
                otherwise.as_deref(),
                span,
                depth,
                diagnostics,
            )?,
            Statement::While {
                condition,
                body,
                span,
            } => self.while_statement(condition, body, span, depth, diagnostics)?,
            Statement::Value { .. } => {
                return Err(CompileError::InvalidIr("control flow"));
            }
        }
        Ok(())
    }
    fn expr(
        &mut self,
        e: &Expr,
        depth: usize,
        expected: Option<&Ty>,
    ) -> Result<Flow, CompileError> {
        self.expr_mode(e, depth, expected, false)
    }
    fn expr_mode(
        &mut self,
        e: &Expr,
        depth: usize,
        expected: Option<&Ty>,
        borrow: bool,
    ) -> Result<Flow, CompileError> {
        match e {
            Expr::Integer(n, s) => {
                let n = n
                    .parse::<i64>()
                    .map_err(|_| CompileError::InvalidIr("unvalidated integer"))?;
                Ok(self.emitted(Op::ConstI64(n), Ty::I64, s.clone(), Access::Own, false))
            }
            Expr::Bool(v, s) => {
                Ok(self.emitted(Op::ConstBool(*v), Ty::Bool, s.clone(), Access::Own, false))
            }
            Expr::Unit(s) => {
                Ok(self.emitted(Op::ConstUnit, Ty::Unit, s.clone(), Access::Own, false))
            }
            Expr::Name(name, s) => {
                let id = self.get(name)?;
                self.check_available(id, s.clone())?;
                if self.locals[id.0].ty == Ty::Never {
                    // A binding produced by a diverging initializer is read
                    // only in hypothetical flow. Reading it does not make a
                    // containing branch diverge a second time.
                    return Ok(Flow::Value {
                        temp: None,
                        ty: expected.cloned().unwrap_or(Ty::Never),
                    });
                }
                self.conflict(id, s.clone(), "read")?;
                let ty = self.locals[id.0].ty.clone();
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
            Expr::Group(inner, _) => self.expr_mode(inner, depth + 1, expected, borrow),
            Expr::Binary {
                op,
                left,
                right,
                span,
            } => {
                // Scalar equality reads independent values. Array equality
                // borrows its operands to avoid deep copies and retains the
                // loan across evaluation of the right operand.
                let borrow_left = op == "=="
                    && self
                        .place_ty(left)
                        .is_some_and(|ty| matches!(ty, Ty::Array(_)));
                let numeric_operand = matches!(op.as_str(), "+" | "-" | "<");
                let left_flow = self.expr_mode(
                    left,
                    depth + 1,
                    numeric_operand.then_some(&Ty::I64),
                    borrow_left,
                )?;
                let left_diverged = left_flow.diverged();
                let left_ty = left_flow.ty().cloned();
                // This repeats the I64 expectation passed to expr_mode. Keep
                // the local check as a defensive invariant for flows that
                // return a type without consuming their expected type.
                if numeric_operand
                    && let Some(actual) = left_ty.as_ref()
                    && *actual != Ty::I64
                    && *actual != Ty::Never
                {
                    return Err(type_mismatch(left.span(), &Ty::I64, actual, None));
                }
                if left_diverged {
                    self.emission = EmissionMode::Analyze;
                    self.terminated = None;
                }
                let borrow_right = op == "==" && matches!(left_ty, Some(Ty::Array(_)));
                let left_root = if borrow_left {
                    place_parts(left)
                        .ok()
                        .and_then(|(root, _)| place_name(root))
                        .and_then(|name| self.visible.get(name).and_then(|ids| ids.last()).copied())
                } else {
                    None
                };
                if let Some(root) = left_root {
                    // Conflict reporting reads the first live loan. Append so
                    // an outer loan keeps its source origin while this nested
                    // comparison runs; pop below removes precisely this loan.
                    self.active_loans.entry(root).or_default().push(ActiveLoan {
                        permission: LoanPermission::Read,
                        span: left.span().clone(),
                        loan: LoanId(left.span().start as usize),
                        parent: None,
                    });
                }
                let right_expected = if numeric_operand {
                    Some(&Ty::I64)
                } else {
                    left_ty.as_ref().filter(|ty| **ty != Ty::Never)
                };
                let right_result = self.expr_mode(right, depth + 1, right_expected, borrow_right);
                if let Some(root) = left_root {
                    self.active_loans
                        .get_mut(&root)
                        .and_then(|loans| loans.pop());
                }
                let right_flow = right_result?;
                let right_diverged = right_flow.diverged();
                let right_ty = right_flow.ty();
                if let Some(rt) = right_ty.filter(|ty| **ty != Ty::Never) {
                    let expected_right = if numeric_operand {
                        Ty::I64
                    } else {
                        left_ty
                            .as_ref()
                            .filter(|ty| **ty != Ty::Never)
                            .cloned()
                            .unwrap_or_else(|| rt.clone())
                    };
                    if *rt != expected_right {
                        return Err(type_mismatch(
                            right.span(),
                            &expected_right,
                            rt,
                            matches!(op.as_str(), "<" | "==").then_some(left.span()),
                        ));
                    }
                }
                let operator = match op.as_str() {
                    "+" => Binary::Add,
                    "-" => Binary::Sub,
                    "<" => Binary::Less,
                    "==" => Binary::Equal,
                    _ => return Err(CompileError::InvalidIr("binary operation")),
                };
                let ty = binary_result_type(op)?;
                if left_diverged || right_diverged {
                    self.terminated = Some(Termination::Diverged);
                    self.emission = EmissionMode::Analyze;
                    return Ok(Flow::Diverged { ty: Some(ty) });
                }
                if self.emission == EmissionMode::Analyze {
                    return Ok(Flow::Value { temp: None, ty });
                }
                let (Some(l), Some(r)) = (left_flow.temp(), right_flow.temp()) else {
                    return Err(CompileError::InvalidIr("binary operand flow"));
                };
                Ok(self.emitted(
                    Op::Binary {
                        op: operator,
                        left: l,
                        right: r,
                    },
                    ty,
                    span.clone(),
                    Access::Own,
                    false,
                ))
            }
            Expr::Call { target, args, span } => {
                let name = place_name(target).ok_or(CompileError::InvalidIr("call target"))?;
                if self.visible.get(name).and_then(|v| v.last()).is_some() {
                    self.get(name)?;
                    return Err(CompileError::InvalidIr(
                        "non-callable target passed checker",
                    ));
                }
                if let Some((params, result)) = self.user_functions.get(name).cloned() {
                    if args.len() != params.len() {
                        return Err(CompileError::InvalidIr("call arity passed checker"));
                    }
                    let (prepared, acquired, diverged) =
                        self.prepare_call_arguments(args, &params, depth, true, true)?;
                    self.release_call_loans(&acquired);
                    if diverged {
                        return Ok(Flow::Diverged { ty: Some(result) });
                    }
                    if self.emission == EmissionMode::Analyze {
                        return Ok(Flow::Value {
                            temp: None,
                            ty: result,
                        });
                    }
                    let signature = params;
                    let result_flow = self.emitted(
                        Op::CallDirect {
                            function: FunctionId(name.into()),
                            args: prepared,
                            signature,
                        },
                        result.clone(),
                        span.clone(),
                        Access::Own,
                        false,
                    );
                    self.end_call_loans(acquired);
                    return Ok(result_flow);
                }
                let intrinsic = match name {
                    "Print" => Intrinsic::Print,
                    "AssertEq" => Intrinsic::AssertEq,
                    "Drop" => Intrinsic::DropData,
                    "Fail" => Intrinsic::Fail,
                    "Len" => Intrinsic::Len,
                    "MockOpen" => Intrinsic::MockOpen,
                    "MockBlockingOpen" => Intrinsic::MockBlockingOpen,
                    "Inspect" => Intrinsic::Inspect,
                    _ => return Err(CompileError::InvalidIr("user or deferred call")),
                };
                if intrinsic == Intrinsic::DropData
                    && args.len() == 1
                    && let Some(binding) = place_name(&args[0].value)
                {
                    let local = self.get(binding)?;
                    self.check_available(local, args[0].value.span().clone())?;
                    if self.locals[local.0]
                        .parameter_mode
                        .is_some_and(|m| m != ParameterMode::Consume)
                    {
                        return Err(self.permission_error(
                            local,
                            args[0].value.span().clone(),
                            "drop",
                        ));
                    }
                    self.conflict(local, args[0].value.span().clone(), "drop")?;
                    self.mark_consumed(local, Some((span.clone(), true)));
                    return Ok(self.emitted(
                        Op::DropBinding(local),
                        Ty::Unit,
                        span.clone(),
                        Access::Own,
                        false,
                    ));
                }
                let parameters: Vec<(String, ParameterMode, Ty)> = match intrinsic {
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
                    Intrinsic::Inspect => vec![("r".into(), ParameterMode::Read, Ty::Never)],
                    Intrinsic::MockOpen | Intrinsic::MockBlockingOpen => {
                        vec![("label".into(), ParameterMode::Consume, Ty::I64)]
                    }
                    Intrinsic::Fail => vec![],
                };
                let (prepared, acquired, diverged) =
                    self.prepare_call_arguments(args, &parameters, depth, false, args.len() > 1)?;
                self.release_call_loans(&acquired);
                if diverged {
                    return Ok(Flow::Diverged { ty: None });
                }
                // AssertEq is symmetric; retain source order so loan release
                // is the exact reverse of argument evaluation.
                let ids: Vec<_> = prepared.into_iter().map(|(_, id)| id).collect();
                let arity = match intrinsic {
                    Intrinsic::Print
                    | Intrinsic::DropData
                    | Intrinsic::Len
                    | Intrinsic::MockOpen
                    | Intrinsic::MockBlockingOpen
                    | Intrinsic::Inspect => 1,
                    Intrinsic::AssertEq => 2,
                    Intrinsic::Fail => 0,
                };
                if args.len() != arity {
                    return Err(CompileError::InvalidIr("intrinsic arity"));
                }
                let ty = match intrinsic {
                    Intrinsic::Len => Ty::I64,
                    Intrinsic::MockOpen => Ty::MockResource,
                    Intrinsic::MockBlockingOpen => Ty::MockBlockingResource,
                    _ => Ty::Unit,
                };
                let result_flow = self.emitted(
                    Op::CallIntrinsic {
                        intrinsic,
                        args: ids,
                    },
                    ty,
                    span.clone(),
                    Access::Own,
                    matches!(
                        intrinsic,
                        Intrinsic::Print
                            | Intrinsic::MockOpen
                            | Intrinsic::MockBlockingOpen
                            | Intrinsic::Inspect
                    ),
                );
                self.end_call_loans(acquired);
                Ok(result_flow)
            }
            Expr::Array(items, span) => {
                let element_expected = expected.and_then(|ty| match ty {
                    Ty::Array(t) => Some(t.as_ref()),
                    _ => None,
                });
                let mut ids = Vec::with_capacity(items.len());
                let mut element_ty = element_expected.cloned();
                let mut diverged = false;
                let mut item_errors = Vec::new();
                let mut dependent = false;
                for item in items {
                    if diverged {
                        self.terminated = None;
                    }
                    let item_flow = match self.expr(item, depth + 1, element_ty.as_ref()) {
                        Ok(value) => value,
                        Err(CompileError::Rejected(errors)) => {
                            item_errors.extend(errors);
                            continue;
                        }
                        Err(CompileError::Dependent) => {
                            dependent = true;
                            continue;
                        }
                        Err(error) => return Err(error),
                    };
                    if item_flow.diverged() {
                        diverged = true;
                        self.emission = EmissionMode::Analyze;
                        continue;
                    }
                    let Some(ty) = item_flow.ty().cloned() else {
                        continue;
                    };
                    if ty == Ty::Never {
                        continue;
                    }
                    if let Some(old) = &element_ty {
                        if old != &ty {
                            return Err(CompileError::InvalidIr("array element type"));
                        }
                    } else {
                        element_ty = Some(ty);
                    }
                    if let Some(id) = item_flow.temp() {
                        ids.push(id);
                    }
                }
                if !item_errors.is_empty() {
                    return Err(CompileError::Rejected(item_errors));
                }
                if dependent {
                    return Err(CompileError::Dependent);
                }
                let inferred_element = element_ty.unwrap_or(Ty::Never);
                if inferred_element == Ty::Never {
                    if diverged {
                        self.terminated = Some(Termination::Diverged);
                        return Ok(Flow::Diverged { ty: None });
                    }
                    return Ok(Flow::Value {
                        temp: None,
                        ty: Ty::Never,
                    });
                }
                if diverged {
                    self.terminated = Some(Termination::Diverged);
                    return Ok(Flow::Diverged {
                        ty: Some(Ty::Array(Box::new(inferred_element))),
                    });
                }
                let ty = Ty::Array(Box::new(inferred_element));
                Ok(self.emitted(Op::BuildArray(ids), ty, span.clone(), Access::Own, false))
            }
            Expr::Index {
                target,
                index,
                span,
            } => {
                if let Ok((root, index_exprs)) = place_parts(e)
                    && let Some(name) = place_name(root)
                {
                    let local = self.get(name)?;
                    let mut ty = self.locals[local.0].ty.clone();
                    let mut ids = Vec::with_capacity(index_exprs.len());
                    let mut diverged = false;
                    for index_expr in index_exprs {
                        if diverged {
                            self.terminated = None;
                            self.emission = EmissionMode::Analyze;
                        }
                        let index_flow = self.expr(index_expr, depth + 1, Some(&Ty::I64))?;
                        if index_flow.diverged() {
                            diverged = true;
                            self.emission = EmissionMode::Analyze;
                        }
                        if index_flow
                            .ty()
                            .is_some_and(|it| *it != Ty::I64 && *it != Ty::Never)
                        {
                            return Err(CompileError::InvalidIr("index type"));
                        }
                        ty = match ty {
                            Ty::Array(child) => *child,
                            Ty::Never => Ty::Never,
                            _ => return Err(CompileError::InvalidIr("index target type")),
                        };
                        if let Some(id) = index_flow.temp() {
                            ids.push(id);
                        }
                        // The root is acquired after each index expression. An
                        // index may consume it, including on one branch only.
                        self.check_place_access(local, root, e, "read", true)?;
                        self.statement_op(
                            Op::CheckIndex {
                                local,
                                indexes: ids.clone(),
                            },
                            span.clone(),
                            Access::Read,
                            false,
                        );
                    }
                    if diverged {
                        self.terminated = Some(Termination::Diverged);
                        return Ok(Flow::Diverged {
                            ty: expected.cloned(),
                        });
                    }
                    if ty == Ty::Never {
                        return Ok(Flow::Value {
                            temp: None,
                            ty: expected.cloned().unwrap_or(Ty::Never),
                        });
                    }
                    return Ok(self.emitted(
                        if borrow {
                            Op::BorrowIndex {
                                local,
                                indexes: ids,
                            }
                        } else {
                            Op::ReadIndex {
                                local,
                                indexes: ids,
                            }
                        },
                        ty,
                        span.clone(),
                        Access::Read,
                        false,
                    ));
                }
                let base_flow = self.expr(target, depth + 1, None)?;
                if base_flow.diverged() {
                    self.terminated = None;
                    self.emission = EmissionMode::Analyze;
                    self.expr(index, depth + 1, Some(&Ty::I64))?;
                    self.terminated = Some(Termination::Diverged);
                    return Ok(Flow::Diverged {
                        ty: expected.cloned(),
                    });
                }
                let index_flow = self.expr(index, depth + 1, Some(&Ty::I64))?;
                if index_flow.diverged() {
                    return Ok(Flow::Diverged {
                        ty: expected.cloned(),
                    });
                }
                if index_flow
                    .ty()
                    .is_some_and(|ty| *ty != Ty::I64 && *ty != Ty::Never)
                {
                    return Err(CompileError::InvalidIr("index type"));
                }
                let ty = match base_flow.ty() {
                    Some(Ty::Array(child)) => *child.clone(),
                    Some(Ty::Never) => expected.cloned().unwrap_or(Ty::Never),
                    _ => return Err(CompileError::InvalidIr("index target type")),
                };
                if self.emission == EmissionMode::Analyze {
                    return Ok(Flow::Value { temp: None, ty });
                }
                let (Some(base), Some(idx)) = (base_flow.temp(), index_flow.temp()) else {
                    return Err(CompileError::InvalidIr("index operand flow"));
                };
                Ok(self.emitted(
                    Op::IndexValue { base, index: idx },
                    ty,
                    span.clone(),
                    Access::Own,
                    false,
                ))
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
                self.check_available(id, value.span().clone())?;
                if self.locals[id.0]
                    .parameter_mode
                    .is_some_and(|m| m != ParameterMode::Consume)
                {
                    return Err(self.permission_error(id, value.span().clone(), "consume"));
                }
                self.conflict(id, value.span().clone(), "consume")?;
                self.mark_consumed(id, Some((span.clone(), false)));
                let ty = self.locals[id.0].ty.clone();
                if ty == Ty::Never {
                    return Ok(Flow::Value {
                        temp: None,
                        ty: expected.cloned().unwrap_or(Ty::Never),
                    });
                }
                Ok(self.emitted(Op::TransferValue(id), ty, span.clone(), Access::Own, false))
            }
            // parse::unadmitted_edit_in_expr rejects an exclusive prefix except
            // as an immediate call argument; lower_call handles that argument.
            // limits::first_pending rejects result propagation before lowering.
            // The source-level witnesses live in isolated_edit_never_reaches_lowering
            // and access_prefix_contexts_never_escape_as_invalid_ir.
            Expr::Unary { .. } => Err(CompileError::InvalidIr(
                "exclusive loans or result propagation",
            )),
            Expr::If {
                condition,
                then_block,
                otherwise,
                span,
            } => self.if_expression(
                condition,
                then_block,
                otherwise.as_deref(),
                span,
                depth,
                expected,
            ),
        }
    }
}
fn place_name(expr: &Expr) -> Option<&str> {
    match expr {
        Expr::Name(n, _) => Some(n),
        Expr::Group(inner, _) => place_name(inner),
        _ => None,
    }
}
fn place_parts(expr: &Expr) -> Result<(&Expr, Vec<&Expr>), CompileError> {
    let mut node = expr;
    let mut indexes = Vec::new();
    loop {
        match node {
            Expr::Group(inner, _) => node = inner,
            Expr::Index { target, index, .. } => {
                indexes.push(index.as_ref());
                node = target;
            }
            Expr::Name(..) => {
                indexes.reverse();
                return Ok((node, indexes));
            }
            _ => return Err(CompileError::InvalidIr("nonlocal place")),
        }
    }
}
fn lower_type(ty: &Type) -> Result<Ty, CompileError> {
    let mut node = ty;
    let mut depth = 0;
    loop {
        match node {
            Type::Array(child) => {
                depth += 1;
                node = child;
            }
            Type::I64 | Type::Bool | Type::Unit => {
                let mut result = match node {
                    Type::I64 => Ty::I64,
                    Type::Bool => Ty::Bool,
                    _ => Ty::Unit,
                };
                for _ in 0..depth {
                    result = Ty::Array(Box::new(result));
                }
                return Ok(result);
            }
            Type::MockResource | Type::MockBlockingResource => {
                let mut result = if matches!(node, Type::MockResource) {
                    Ty::MockResource
                } else {
                    Ty::MockBlockingResource
                };
                for _ in 0..depth {
                    result = Ty::Array(Box::new(result));
                }
                return Ok(result);
            }
            _ => return Err(CompileError::InvalidIr("resource or result type")),
        }
    }
}
fn statement_span(stmt: &Statement) -> Span {
    match stmt {
        Statement::Let { span, .. }
        | Statement::Assign { span, .. }
        | Statement::Return { span, .. }
        | Statement::Value { span, .. }
        | Statement::If { span, .. }
        | Statement::While { span, .. }
        | Statement::Expr { span, .. } => span.clone(),
        Statement::Block(block) => block.span.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn analyzed_lower<'a>(
        source: &str,
        signatures: &'a HashMap<String, FunctionSignature>,
        binding: (&str, Ty),
    ) -> Lower<'a> {
        let (name, ty) = binding;
        let span = Span {
            file: "flow_contract.nx".into(),
            start: source.find(name).unwrap_or(0) as u64,
            end: source.find(name).unwrap_or(0) as u64 + name.len() as u64,
        };
        Lower {
            instructions: Vec::new(),
            blocks: Vec::new(),
            locals: vec![Local {
                id: LocalId(0),
                name: name.into(),
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
            name_spans: vec![span],
            consumed_at: HashMap::new(),
            maybe_consumed_at: HashMap::new(),
            consumed_undo: Vec::new(),
            dead_depth: 1,
            emission: EmissionMode::Analyze,
            poisoned: HashSet::new(),
            tokens: Arc::new(lex::lex("flow_contract.nx", source).unwrap()),
            defer_ownership: false,
            active_loans: HashMap::new(),
            live_temps: HashSet::new(),
            temp_undo: Vec::new(),
        }
    }

    #[test]
    fn analyzed_binding_loan_and_index_preserve_flow_type() {
        let source = "fn Main() {\n    h(v);\n    h(a[(if true then {\n        return;\n    } else {\n        return;\n    })]);\n}\n";
        let ast = parse::parse("flow_contract.nx", source).unwrap();
        let signatures = HashMap::new();
        let statements = &ast.functions[0].body.statements;
        let Statement::Expr {
            value: Expr::Call { args: first, .. },
            ..
        } = &statements[0]
        else {
            panic!("call")
        };
        let mut binding = analyzed_lower(source, &signatures, ("v", Ty::Never));
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

        let Statement::Expr {
            value: Expr::Call { args: second, .. },
            ..
        } = &statements[1]
        else {
            panic!("call")
        };
        let mut indexed = analyzed_lower(source, &signatures, ("a", Ty::Array(Box::new(Ty::I64))));
        let (flow, _) = indexed
            .call_argument(&second[0], ParameterMode::Read, &Ty::I64, 0, true)
            .unwrap();
        assert!(matches!(flow, Flow::Diverged { ty: Some(Ty::I64) }));
    }

    #[test]
    fn analyzed_branch_retains_type_of_only_continuing_value() {
        let source = "fn Main() {\n    let w = if true then 1 else {\n        return;\n    };\n}\n";
        let ast = parse::parse("flow_contract.nx", source).unwrap();
        let signatures = HashMap::new();
        let mut lower = analyzed_lower(source, &signatures, ("unused", Ty::Never));
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
        let ast = parse::parse("bottom_flow.nx", source).unwrap();
        let signatures = HashMap::new();
        let declaration = &ast.functions[0].body.statements[0];
        let Statement::Let { value, .. } = declaration else {
            panic!("first binding")
        };
        let name_span = Span {
            file: "bottom_flow.nx".into(),
            start: statement_span(declaration).start + 4,
            end: statement_span(declaration).start + 5,
        };
        let mut lower = Lower {
            instructions: Vec::new(),
            blocks: Vec::new(),
            locals: vec![Local {
                id: LocalId(0),
                name: "v".into(),
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
            name_spans: vec![name_span],
            consumed_at: HashMap::new(),
            maybe_consumed_at: HashMap::new(),
            consumed_undo: Vec::new(),
            dead_depth: 1,
            emission: EmissionMode::Analyze,
            poisoned: HashSet::new(),
            tokens: Arc::new(lex::lex("bottom_flow.nx", source).unwrap()),
            defer_ownership: false,
            active_loans: HashMap::new(),
            live_temps: HashSet::new(),
            temp_undo: Vec::new(),
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
            .union_slot(&mut slot, TempId(0), Ty::Never, value.span())
            .unwrap();
        lower
            .union_slot(&mut slot, TempId(0), Ty::I64, value.span())
            .unwrap();
        assert_eq!(lower.locals[slot.unwrap().0].ty, Ty::I64);
    }

    #[test]
    fn divergent_call_argument_does_not_leave_orphan_call_instruction() {
        let source = "fn H(a: view I64, b: view I64) {}\nfn Main() {\n    H(1, if true then {\n        return;\n    } else {\n        return;\n    });\n}\n";
        let ast = parse::parse("orphan_call.nx", source).unwrap();
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
            name_spans: Vec::new(),
            consumed_at: HashMap::new(),
            maybe_consumed_at: HashMap::new(),
            consumed_undo: Vec::new(),
            dead_depth: 0,
            emission: EmissionMode::Live,
            poisoned: HashSet::new(),
            tokens: Arc::new(lex::lex("orphan_call.nx", source).unwrap()),
            defer_ownership: false,
            active_loans: HashMap::new(),
            live_temps: HashSet::new(),
            temp_undo: Vec::new(),
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
            lower.finish_block(Terminator::Return { value: None, span }),
            BlockId(usize::MAX)
        );
        assert_eq!(lower.instructions.len(), instructions);
        assert_eq!(lower.next_temp, temporaries);
        assert_eq!(lower.blocks.len(), blocks);
    }

    #[test]
    fn scoped_block_error_pops_scope_and_restores_parent() {
        let source = "fn Main() {\n    {\n        let x = TryAdd(1, 2)?;\n    }\n}\n";
        let ast = parse::parse("scope_error.nx", source).unwrap();
        let signatures = HashMap::new();
        let mut lower = Lower {
            instructions: Vec::new(),
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
            name_spans: Vec::new(),
            consumed_at: HashMap::new(),
            maybe_consumed_at: HashMap::new(),
            consumed_undo: Vec::new(),
            dead_depth: 0,
            emission: EmissionMode::Live,
            poisoned: HashSet::new(),
            tokens: Arc::new(lex::lex("scope_error.nx", source).unwrap()),
            defer_ownership: false,
            active_loans: HashMap::new(),
            live_temps: HashSet::new(),
            temp_undo: Vec::new(),
        };
        assert!(matches!(
            lower.statement(&ast.functions[0].body.statements[0], 0, &mut Vec::new()),
            Err(CompileError::InvalidIr(_))
        ));
        assert_eq!(lower.scopes.len(), 1);
        assert_eq!(lower.scope, ScopeId(0));
        assert!(lower.visible.is_empty());
    }

    #[test]
    fn rejected_while_condition_restores_statement_checkpoint() {
        let source = "fn Main() {\n    while (if true then true else {}) {}\n    let x = 1;\n}\n";
        let ast = parse::parse("while_error_checkpoint.nx", source).unwrap();
        let signatures = HashMap::new();
        let mut lower = Lower {
            instructions: Vec::new(),
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
            name_spans: Vec::new(),
            consumed_at: HashMap::new(),
            maybe_consumed_at: HashMap::new(),
            consumed_undo: Vec::new(),
            dead_depth: 0,
            emission: EmissionMode::Live,
            poisoned: HashSet::new(),
            tokens: Arc::new(lex::lex("while_error_checkpoint.nx", source).unwrap()),
            defer_ownership: false,
            active_loans: HashMap::new(),
            live_temps: HashSet::new(),
            temp_undo: Vec::new(),
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
    }

    #[test]
    fn merged_consumption_sites_keep_both_sides_sorted_without_duplicates() {
        let at = |start| Span {
            file: "merge.nx".into(),
            start,
            end: start + 1,
        };
        let left = vec![(at(8), false), (at(2), true)];
        let right = vec![(at(5), false), (at(2), true)];
        assert_eq!(
            merge_sites(left, right),
            vec![(at(2), true), (at(5), false), (at(8), false)]
        );
    }

    #[test]
    fn branch_local_consumption_does_not_escape_its_scope() {
        let source = "fn Main() {\n    if true {\n        let r = MockOpen(1);\n        Drop(r);\n    } else {}\n}\n";
        let ast = parse::parse("branch_scope.nx", source).unwrap();
        let signatures = HashMap::new();
        let mut lower = Lower {
            instructions: Vec::new(),
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
            name_spans: Vec::new(),
            consumed_at: HashMap::new(),
            maybe_consumed_at: HashMap::new(),
            consumed_undo: Vec::new(),
            dead_depth: 0,
            emission: EmissionMode::Live,
            poisoned: HashSet::new(),
            tokens: Arc::new(lex::lex("branch_scope.nx", source).unwrap()),
            defer_ownership: false,
            active_loans: HashMap::new(),
            live_temps: HashSet::new(),
            temp_undo: Vec::new(),
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
        let ast = parse::parse("loan_write.nx", source).unwrap();
        let signatures = HashMap::new();
        let mut lower = Lower {
            instructions: Vec::new(),
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
            name_spans: Vec::new(),
            consumed_at: HashMap::new(),
            maybe_consumed_at: HashMap::new(),
            consumed_undo: Vec::new(),
            dead_depth: 0,
            emission: EmissionMode::Live,
            poisoned: HashSet::new(),
            tokens: Arc::new(lex::lex("loan_write.nx", source).unwrap()),
            defer_ownership: false,
            active_loans: HashMap::new(),
            live_temps: HashSet::new(),
            temp_undo: Vec::new(),
        };
        let statements = &ast.functions[0].body.statements;
        lower.statement(&statements[0], 0, &mut Vec::new()).unwrap();
        lower.active_loans.insert(
            LocalId(0),
            vec![ActiveLoan {
                permission: LoanPermission::Read,
                span: Span {
                    file: "loan_write.nx".into(),
                    start: 18,
                    end: 19,
                },
                loan: LoanId(18),
                parent: None,
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
        errors[0].validate("loan_write.nx", source).unwrap();
    }

    #[test]
    fn dead_declaration_and_reinitialization_restore_all_lowering_state() {
        let source = "fn Main() {\n    var r = MockOpen(1);\n    Drop(r);\n    {\n        return;\n        let s = r;\n        let x = 1;\n        r = MockOpen(2);\n        return;\n    }\n    Inspect(r);\n}\n";
        let ast = parse::parse("dead_state.nx", source).unwrap();
        let signatures = HashMap::new();
        let mut lower = Lower {
            instructions: Vec::new(),
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
            name_spans: Vec::new(),
            consumed_at: HashMap::new(),
            maybe_consumed_at: HashMap::new(),
            consumed_undo: Vec::new(),
            dead_depth: 0,
            emission: EmissionMode::Live,
            poisoned: HashSet::new(),
            tokens: Arc::new(lex::lex("dead_state.nx", source).unwrap()),
            defer_ownership: false,
            active_loans: HashMap::new(),
            live_temps: HashSet::new(),
            temp_undo: Vec::new(),
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
        let ast = parse::parse("poison.nx", source).unwrap();
        let signatures = HashMap::new();
        let mut lower = Lower {
            instructions: Vec::new(),
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
            name_spans: Vec::new(),
            consumed_at: HashMap::new(),
            maybe_consumed_at: HashMap::new(),
            consumed_undo: Vec::new(),
            dead_depth: 0,
            emission: EmissionMode::Live,
            poisoned: HashSet::new(),
            tokens: Arc::new(lex::lex("poison.nx", source).unwrap()),
            defer_ownership: false,
            active_loans: HashMap::new(),
            live_temps: HashSet::new(),
            temp_undo: Vec::new(),
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
        let ast = parse::parse("poison.nx", source).unwrap();
        let signatures = HashMap::new();
        let mut lower = Lower {
            instructions: Vec::new(),
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
            name_spans: Vec::new(),
            consumed_at: HashMap::new(),
            maybe_consumed_at: HashMap::new(),
            consumed_undo: Vec::new(),
            dead_depth: 0,
            emission: EmissionMode::Live,
            poisoned: HashSet::new(),
            tokens: Arc::new(lex::lex("poison.nx", source).unwrap()),
            defer_ownership: false,
            active_loans: HashMap::new(),
            live_temps: HashSet::new(),
            temp_undo: Vec::new(),
        };
        let mut diagnostics = Vec::new();
        lower
            .block(&ast.functions[0].body.statements, 0, &mut diagnostics)
            .unwrap();
        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].code, "NX-E0201");
    }
}
