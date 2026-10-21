use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use super::model::{
    self, Access, BasicBlock, Binary, BlockId, Function, FunctionId, FunctionTable, Instruction,
    Intrinsic, Local, LocalId, Op, OriginId, Parameter, ParameterMode, Program, ScopeId, TempId,
    Terminator, Ty,
};
use crate::diagnostic::{self, Diagnostic, PROFILE_ID, Span};
use crate::frontend::lex;
use crate::frontend::parse::{self, Ast, Expr, Mode, Statement, Type};

#[derive(Debug)]
pub enum CompileError {
    Rejected(Vec<Box<Diagnostic>>),
    OutsideScalarSlice(&'static str),
    InvalidIr(&'static str),
    Dependent,
}

/// Produces an opaque, verified artifact. No interpreter entry accepts an AST.
pub fn compile_scalar(file: &str, source: &str) -> Result<Program, CompileError> {
    crate::check::check_proposed(file, source, true).map_err(CompileError::Rejected)?;
    let ast = parse::parse(file, source).map_err(|d| CompileError::Rejected(vec![d]))?;
    std::thread::scope(|scope| {
        std::thread::Builder::new()
            .stack_size(16 * 1024 * 1024)
            .spawn_scoped(scope, || compile_ast(&ast, source))
            .map_err(|_| CompileError::InvalidIr("lowering worker unavailable"))?
            .join()
            .map_err(|_| CompileError::InvalidIr("lowering worker panicked"))?
    })
}

pub(super) fn compile_ast(ast: &Ast, source_text: &str) -> Result<Program, CompileError> {
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
                    Mode::Inout => Err(CompileError::OutsideScalarSlice("exclusive parameters")),
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
            consumed_undo: Vec::new(),
            dead_depth: 0,
            poisoned: HashSet::new(),
            tokens: Arc::clone(&tokens),
        };
        let mut parameters = Vec::new();
        for (p, (_, mode, ty)) in source.params.iter().zip(&user_functions[&source.name].0) {
            let id = LocalId(lower.locals.len());
            lower.locals.push(Local {
                id,
                name: p.name.clone(),
                ty: ty.clone(),
                mutable: false,
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
        lower.block(&source.body.statements, 0, &mut diagnostics)?;
        if !diagnostics.is_empty() {
            continue;
        }
        let span = source.body.span.clone();
        let value = if let Some(value) = lower.terminated {
            Some(value)
        } else {
            if result != Ty::Unit {
                return Err(CompileError::OutsideScalarSlice(
                    "control-flow return analysis",
                ));
            }
            Some(lower.emit(Op::ConstUnit, Ty::Unit, span.clone(), Access::Own, false))
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
            block: BasicBlock {
                id: BlockId(0),
                instructions: lower.instructions,
                terminator: Terminator::Return { value, span },
            },
            result,
            cleanup: Vec::new(),
        };
        function.cleanup = function.locals.iter().rev().map(|l| l.id).collect();
        function.content_hash = model::content_hash(&function);
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
    locals: Vec<Local>,
    visible: HashMap<String, Vec<LocalId>>,
    scopes: Vec<Vec<String>>,
    scope: ScopeId,
    next_scope: usize,
    scope_parents: Vec<Option<ScopeId>>,
    next_temp: usize,
    result: Ty,
    terminated: Option<TempId>,
    user_functions: &'a HashMap<String, FunctionSignature>,
    name_spans: Vec<Span>,
    consumed_at: HashMap<LocalId, (Span, bool)>,
    consumed_undo: Vec<(LocalId, Option<(Span, bool)>)>,
    dead_depth: usize,
    poisoned: HashSet<LocalId>,
    tokens: Arc<Vec<lex::Token>>,
}
type FunctionSignature = (Vec<(String, ParameterMode, Ty)>, Ty);
impl Lower<'_> {
    fn mark_consumed(&mut self, id: LocalId, value: Option<(Span, bool)>) {
        if self.dead_depth != 0 {
            self.consumed_undo
                .push((id, self.consumed_at.get(&id).cloned()));
        }
        if let Some(value) = value {
            self.consumed_at.insert(id, value);
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
        consumption: Option<(Span, bool)>,
    ) -> CompileError {
        let mut related = vec![diagnostic::RelatedSpan {
            role: diagnostic::RelatedRole::Declaration,
            span: self.name_spans[local.0].clone(),
        }];
        let mut cause_chain = Vec::new();
        if let Some((site, is_drop)) = consumption {
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
            diagnostic::DataValue::String("read".into()),
        );
        d.data.insert(
            "requested_access".into(),
            diagnostic::DataValue::String(requested.into()),
        );
        d.cause_chain.push(diagnostic::Cause {
            kind: diagnostic::CauseKind::Borrow,
            operation: diagnostic::CauseOperation::BorrowRead,
            span: Some(self.name_spans[id.0].clone()),
            entity: Some(entity),
            evidence: diagnostic::Evidence::Verified,
        });
        CompileError::Rejected(errors)
    }
    fn check_available(&self, id: LocalId, span: Span) -> Result<(), CompileError> {
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
    fn emit(&mut self, op: Op, ty: Ty, span: Span, access: Access, effect_io: bool) -> TempId {
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
        id
    }
    fn statement_op(&mut self, op: Op, span: Span, access: Access, effect_io: bool) {
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
    fn block(
        &mut self,
        statements: &[Statement],
        depth: usize,
        diagnostics: &mut Vec<Diagnostic>,
    ) -> Result<(), CompileError> {
        if depth >= 128 {
            return Err(CompileError::OutsideScalarSlice("lowering depth"));
        }
        let mut unreachable = None;
        for statement in statements {
            if self.terminated.is_some() && unreachable.is_none() {
                unreachable = Some((
                    self.instructions.len(),
                    self.locals.len(),
                    self.scopes.last().unwrap().len(),
                    self.next_scope,
                    self.next_temp,
                    self.terminated,
                    self.consumed_undo.len(),
                ));
                self.dead_depth += 1;
            }
            if unreachable.is_some() {
                self.terminated = None;
            }
            let result = self.statement(statement, depth, diagnostics);
            match result {
                Err(CompileError::Rejected(errors)) => {
                    diagnostics.extend(errors.into_iter().map(|d| *d))
                }
                Err(CompileError::Dependent) => {}
                Err(other) => return Err(other),
                Ok(()) => {}
            }
        }
        if let Some((instructions, locals, scope_len, next_scope, next_temp, terminated, undo)) =
            unreachable
        {
            while self.consumed_undo.len() > undo {
                let (id, old) = self.consumed_undo.pop().unwrap();
                if let Some(old) = old {
                    self.consumed_at.insert(id, old);
                } else {
                    self.consumed_at.remove(&id);
                }
            }
            for name in self.scopes.last_mut().unwrap().drain(scope_len..).rev() {
                self.visible.get_mut(&name).unwrap().pop();
            }
            for id in locals..self.locals.len() {
                self.poisoned.remove(&LocalId(id));
            }
            self.locals.truncate(locals);
            self.name_spans.truncate(locals);
            self.instructions.truncate(instructions);
            self.next_scope = next_scope;
            self.scope_parents.truncate(next_scope);
            self.next_temp = next_temp;
            self.terminated = terminated;
            self.dead_depth -= 1;
            if self.dead_depth == 0 {
                self.consumed_undo.clear();
            }
        }
        Ok(())
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
                let (temp, ty) = match self.expr(value, 0, expected.as_ref()) {
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
                            ty: expected.unwrap_or(Ty::Unit),
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
                    ty,
                    mutable: *mutable,
                    scope: self.scope,
                    parameter_mode: None,
                });
                self.visible.entry(name.clone()).or_default().push(id);
                self.scopes
                    .last_mut()
                    .ok_or(CompileError::InvalidIr("scope stack"))?
                    .push(name.clone());
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
                let name =
                    place_name(root).ok_or(CompileError::OutsideScalarSlice("assignment root"))?;
                let id = self.get(name)?;
                if self.locals[id.0].parameter_mode == Some(ParameterMode::Read) {
                    return Err(self.permission_error(id, root.span().clone(), "write"));
                }
                if !self.locals[id.0].mutable {
                    return Err(self.immutable_write(root, id));
                }
                let mut indexes = Vec::with_capacity(index_exprs.len());
                let mut target_ty = self.locals[id.0].ty.clone();
                for e in index_exprs {
                    let (index, index_ty) = self.expr(e, 0, Some(&Ty::I64))?;
                    if index_ty != Ty::I64 {
                        return Err(CompileError::InvalidIr("index type"));
                    }
                    let Ty::Array(child) = target_ty else {
                        return Err(CompileError::InvalidIr("index target type"));
                    };
                    target_ty = *child;
                    indexes.push(index);
                }
                let (temp, ty) = self.expr(value, 0, Some(&target_ty))?;
                if !indexes.is_empty() {
                    self.check_available(id, root.span().clone())?;
                }
                if indexes.is_empty() {
                    self.mark_consumed(id, None);
                }
                if ty != target_ty {
                    return Err(CompileError::InvalidIr("assignment type passed checker"));
                }
                self.statement_op(
                    if indexes.is_empty() {
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
                let (temp, _) = self.expr(value, 0, None)?;
                self.statement_op(
                    Op::DropValue(temp),
                    statement_span(statement),
                    Access::Own,
                    false,
                );
            }
            Statement::Return { value, .. } => {
                let (temp, ty) = match value {
                    Some(e) => self.expr(e, 0, Some(&self.result.clone()))?,
                    None => {
                        let span = statement_span(statement);
                        (
                            self.emit(Op::ConstUnit, Ty::Unit, span, Access::Own, false),
                            Ty::Unit,
                        )
                    }
                };
                if ty != self.result {
                    return Err(CompileError::InvalidIr("return type passed checker"));
                }
                self.terminated = Some(temp);
            }
            Statement::Block(block) => {
                let saved = self.scope;
                self.scope = ScopeId(self.next_scope);
                self.next_scope += 1;
                self.scope_parents.push(Some(saved));
                self.scopes.push(Vec::new());
                self.block(&block.statements, depth + 1, diagnostics)?;
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
                        self.statement_op(
                            Op::DropLocal(id),
                            block.span.clone(),
                            Access::Own,
                            false,
                        );
                    }
                }
                for name in self
                    .scopes
                    .pop()
                    .ok_or(CompileError::InvalidIr("scope stack"))?
                {
                    let entries = self
                        .visible
                        .get_mut(&name)
                        .ok_or(CompileError::InvalidIr("scope name"))?;
                    entries.pop();
                }
                self.scope = saved;
            }
            Statement::If { .. } | Statement::While { .. } | Statement::Value { .. } => {
                return Err(CompileError::OutsideScalarSlice("control flow"));
            }
        }
        Ok(())
    }
    fn expr(
        &mut self,
        e: &Expr,
        depth: usize,
        expected: Option<&Ty>,
    ) -> Result<(TempId, Ty), CompileError> {
        self.expr_mode(e, depth, expected, false)
    }
    fn expr_mode(
        &mut self,
        e: &Expr,
        depth: usize,
        expected: Option<&Ty>,
        borrow: bool,
    ) -> Result<(TempId, Ty), CompileError> {
        if depth >= 256 {
            return Err(CompileError::OutsideScalarSlice("expression depth"));
        }
        match e {
            Expr::Integer(n, s) => {
                let n = n
                    .parse::<i64>()
                    .map_err(|_| CompileError::InvalidIr("unvalidated integer"))?;
                Ok((
                    self.emit(Op::ConstI64(n), Ty::I64, s.clone(), Access::Own, false),
                    Ty::I64,
                ))
            }
            Expr::Bool(v, s) => Ok((
                self.emit(Op::ConstBool(*v), Ty::Bool, s.clone(), Access::Own, false),
                Ty::Bool,
            )),
            Expr::Unit(s) => Ok((
                self.emit(Op::ConstUnit, Ty::Unit, s.clone(), Access::Own, false),
                Ty::Unit,
            )),
            Expr::Name(name, s) => {
                let id = self.get(name)?;
                self.check_available(id, s.clone())?;
                let ty = self.locals[id.0].ty.clone();
                if !borrow && !ty.copyable() {
                    return Err(self.copy_error(id, s.clone()));
                }
                let op = if borrow {
                    Op::BorrowValue(id)
                } else {
                    Op::CopyValue(id)
                };
                Ok((
                    self.emit(op, ty.clone(), s.clone(), Access::Read, false),
                    ty,
                ))
            }
            Expr::Group(inner, _) => self.expr_mode(inner, depth + 1, expected, borrow),
            Expr::Binary {
                op,
                left,
                right,
                span,
            } => {
                let is_eq = op == "==";
                let (l, lt) = self.expr_mode(left, depth + 1, None, is_eq)?;
                let (r, rt) = self.expr_mode(right, depth + 1, Some(&lt), is_eq)?;
                if lt != rt {
                    return Err(CompileError::InvalidIr("binary operand types"));
                }
                let (op, ty) = match op.as_str() {
                    "+" if lt == Ty::I64 => (Binary::Add, Ty::I64),
                    "-" if lt == Ty::I64 => (Binary::Sub, Ty::I64),
                    "<" if lt == Ty::I64 => (Binary::Less, Ty::Bool),
                    "==" => (Binary::Equal, Ty::Bool),
                    _ => return Err(CompileError::OutsideScalarSlice("binary operation")),
                };
                Ok((
                    self.emit(
                        Op::Binary {
                            op,
                            left: l,
                            right: r,
                        },
                        ty.clone(),
                        span.clone(),
                        Access::Own,
                        false,
                    ),
                    ty,
                ))
            }
            Expr::Call { target, args, span } => {
                let name =
                    place_name(target).ok_or(CompileError::OutsideScalarSlice("call target"))?;
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
                    let mut used = vec![false; params.len()];
                    let mut prepared = Vec::with_capacity(args.len());
                    let mut next_positional = 0usize;
                    for arg in args {
                        let slot = if let Some(label) = &arg.label {
                            params
                                .iter()
                                .position(|(p, _, _)| p == label)
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
                        let (id, actual) = self.expr_mode(
                            &arg.value,
                            depth + 1,
                            Some(expected),
                            *mode == ParameterMode::Read,
                        )?;
                        if actual != *expected {
                            return Err(CompileError::InvalidIr(
                                "call argument type passed checker",
                            ));
                        }
                        prepared.push((slot, id));
                    }
                    let signature = params;
                    return Ok((
                        self.emit(
                            Op::CallDirect {
                                function: FunctionId(name.into()),
                                args: prepared,
                                signature,
                            },
                            result.clone(),
                            span.clone(),
                            Access::Own,
                            false,
                        ),
                        result,
                    ));
                }
                let intrinsic = match name {
                    "print" => Intrinsic::Print,
                    "assert_eq" => Intrinsic::AssertEq,
                    "drop" => Intrinsic::DropData,
                    "fail" => Intrinsic::Fail,
                    "len" => Intrinsic::Len,
                    "mock_open" => Intrinsic::MockOpen,
                    "mock_blocking_open" => Intrinsic::MockBlockingOpen,
                    "inspect" => Intrinsic::Inspect,
                    _ => return Err(CompileError::OutsideScalarSlice("user or deferred call")),
                };
                if intrinsic == Intrinsic::DropData
                    && args.len() == 1
                    && let Some(binding) = place_name(&args[0].value)
                {
                    let local = self.get(binding)?;
                    self.check_available(local, args[0].value.span().clone())?;
                    if self.locals[local.0].parameter_mode == Some(ParameterMode::Read) {
                        return Err(self.permission_error(
                            local,
                            args[0].value.span().clone(),
                            "drop",
                        ));
                    }
                    self.mark_consumed(local, Some((span.clone(), true)));
                    return Ok((
                        self.emit(
                            Op::DropBinding(local),
                            Ty::Unit,
                            span.clone(),
                            Access::Own,
                            false,
                        ),
                        Ty::Unit,
                    ));
                }
                let mut ids = Vec::with_capacity(args.len());
                for arg in args {
                    let (id, _) = self.expr_mode(
                        &arg.value,
                        depth + 1,
                        None,
                        matches!(
                            intrinsic,
                            Intrinsic::Print
                                | Intrinsic::AssertEq
                                | Intrinsic::Len
                                | Intrinsic::Inspect
                        ),
                    )?;
                    ids.push(id);
                }
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
                if ids.len() != arity {
                    return Err(CompileError::InvalidIr("intrinsic arity"));
                }
                Ok((
                    self.emit(
                        Op::CallIntrinsic {
                            intrinsic,
                            args: ids,
                        },
                        match intrinsic {
                            Intrinsic::Len => Ty::I64,
                            Intrinsic::MockOpen => Ty::MockResource,
                            Intrinsic::MockBlockingOpen => Ty::MockBlockingResource,
                            _ => Ty::Unit,
                        },
                        span.clone(),
                        Access::Own,
                        matches!(
                            intrinsic,
                            Intrinsic::Print
                                | Intrinsic::MockOpen
                                | Intrinsic::MockBlockingOpen
                                | Intrinsic::Inspect
                        ),
                    ),
                    match intrinsic {
                        Intrinsic::Len => Ty::I64,
                        Intrinsic::MockOpen => Ty::MockResource,
                        Intrinsic::MockBlockingOpen => Ty::MockBlockingResource,
                        _ => Ty::Unit,
                    },
                ))
            }
            Expr::Array(items, span) => {
                let element_expected = expected.and_then(|ty| match ty {
                    Ty::Array(t) => Some(t.as_ref()),
                    _ => None,
                });
                let mut ids = Vec::with_capacity(items.len());
                let mut element_ty = element_expected.cloned();
                for item in items {
                    let (id, ty) = self.expr(item, depth + 1, element_ty.as_ref())?;
                    if let Some(old) = &element_ty {
                        if old != &ty {
                            return Err(CompileError::InvalidIr("array element type"));
                        }
                    } else {
                        element_ty = Some(ty);
                    }
                    ids.push(id);
                }
                let ty = Ty::Array(Box::new(
                    element_ty.ok_or(CompileError::InvalidIr("empty array type"))?,
                ));
                Ok((
                    self.emit(
                        Op::BuildArray(ids),
                        ty.clone(),
                        span.clone(),
                        Access::Own,
                        false,
                    ),
                    ty,
                ))
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
                    self.check_available(local, root.span().clone())?;
                    let mut ty = self.locals[local.0].ty.clone();
                    let mut ids = Vec::with_capacity(index_exprs.len());
                    for index_expr in index_exprs {
                        let (id, it) = self.expr(index_expr, depth + 1, Some(&Ty::I64))?;
                        if it != Ty::I64 {
                            return Err(CompileError::InvalidIr("index type"));
                        }
                        let Ty::Array(child) = ty else {
                            return Err(CompileError::InvalidIr("index target type"));
                        };
                        ty = *child;
                        ids.push(id);
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
                    return Ok((
                        self.emit(
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
                            ty.clone(),
                            span.clone(),
                            Access::Read,
                            false,
                        ),
                        ty,
                    ));
                }
                let (base, base_ty) = self.expr(target, depth + 1, None)?;
                let (idx, idx_ty) = self.expr(index, depth + 1, Some(&Ty::I64))?;
                if idx_ty != Ty::I64 {
                    return Err(CompileError::InvalidIr("index type"));
                }
                let Ty::Array(child) = base_ty else {
                    return Err(CompileError::InvalidIr("index target type"));
                };
                let ty = *child;
                Ok((
                    self.emit(
                        Op::IndexValue { base, index: idx },
                        ty.clone(),
                        span.clone(),
                        Access::Own,
                        false,
                    ),
                    ty,
                ))
            }
            Expr::Unary {
                mode: Mode::Consume,
                value,
                span,
            } => {
                let Some(name) = place_name(value) else {
                    return Err(CompileError::OutsideScalarSlice("partial transfer"));
                };
                let id = self.get(name)?;
                self.check_available(id, value.span().clone())?;
                if self.locals[id.0].parameter_mode == Some(ParameterMode::Read) {
                    return Err(self.permission_error(id, value.span().clone(), "consume"));
                }
                self.mark_consumed(id, Some((span.clone(), false)));
                let ty = self.locals[id.0].ty.clone();
                Ok((
                    self.emit(
                        Op::TransferValue(id),
                        ty.clone(),
                        span.clone(),
                        Access::Own,
                        false,
                    ),
                    ty,
                ))
            }
            Expr::Unary { .. } => Err(CompileError::OutsideScalarSlice(
                "exclusive loans or result propagation",
            )),
            Expr::If { .. } => Err(CompileError::OutsideScalarSlice("expression control flow")),
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
            _ => return Err(CompileError::OutsideScalarSlice("nonlocal place")),
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
                if depth > 128 {
                    return Err(CompileError::OutsideScalarSlice("array type depth"));
                }
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
            _ => return Err(CompileError::OutsideScalarSlice("resource or result type")),
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

    #[test]
    fn dead_declaration_and_reinitialization_restore_all_lowering_state() {
        let source = "fn main(){var r=mock_open(1);drop(r);{return;let s=r;let x=1;r=mock_open(2);}inspect(r);}";
        let ast = parse::parse("dead_state.nx", source).unwrap();
        let signatures = HashMap::new();
        let mut lower = Lower {
            instructions: Vec::new(),
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
            consumed_undo: Vec::new(),
            dead_depth: 0,
            poisoned: HashSet::new(),
            tokens: Arc::new(lex::lex("dead_state.nx", source).unwrap()),
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
        assert_eq!(lower.name_spans[0].start, 14);
        assert_eq!(lower.visible.get("r").unwrap(), &vec![LocalId(0)]);
        assert!(lower.visible.get("s").is_none_or(Vec::is_empty));
        assert!(lower.visible.get("x").is_none_or(Vec::is_empty));
        assert_eq!(lower.scopes[0], vec!["r"]);
        assert!(lower.poisoned.is_empty());
        assert!(lower.terminated.is_some());
        assert_eq!(lower.consumed_at.get(&LocalId(0)).unwrap().0.start, 29);
        assert_eq!(lower.dead_depth, 0);
        assert!(lower.consumed_undo.is_empty());
    }

    #[test]
    fn poisoned_binding_in_rejected_block_has_no_cleanup_instruction() {
        let source = "fn main(){{let r=mock_open(1);let s=r;}}";
        let ast = parse::parse("poison.nx", source).unwrap();
        let signatures = HashMap::new();
        let mut lower = Lower {
            instructions: Vec::new(),
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
            consumed_undo: Vec::new(),
            dead_depth: 0,
            poisoned: HashSet::new(),
            tokens: Arc::new(lex::lex("poison.nx", source).unwrap()),
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
        let source = "fn main(){let r=mock_open(1);let s=r;s();}";
        let ast = parse::parse("poison.nx", source).unwrap();
        let signatures = HashMap::new();
        let mut lower = Lower {
            instructions: Vec::new(),
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
            consumed_undo: Vec::new(),
            dead_depth: 0,
            poisoned: HashSet::new(),
            tokens: Arc::new(lex::lex("poison.nx", source).unwrap()),
        };
        let mut diagnostics = Vec::new();
        lower
            .block(&ast.functions[0].body.statements, 0, &mut diagnostics)
            .unwrap();
        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].code, "NX-E0201");
    }
}
