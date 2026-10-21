use std::collections::{HashMap, HashSet};

use super::model::{
    self, Access, BasicBlock, Binary, BlockId, Function, FunctionId, FunctionTable, Instruction,
    Intrinsic, Local, LocalId, Op, OriginId, Program, ScopeId, TempId, Terminator, Ty,
};
use crate::diagnostic::{self, Diagnostic, PROFILE_ID, Span};
use crate::frontend::lex;
use crate::frontend::parse::{self, Ast, Expr, Statement, Type};

#[derive(Debug)]
pub enum CompileError {
    Rejected(Vec<Box<Diagnostic>>),
    OutsideScalarSlice(&'static str),
    InvalidIr(&'static str),
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
    let user_functions: HashSet<&str> = ast.functions.iter().map(|f| f.name.as_str()).collect();
    let mut functions: Vec<_> = ast.functions.iter().collect();
    functions.sort_by(|a, b| a.name.cmp(&b.name));
    let mut entries = Vec::with_capacity(functions.len());
    let mut diagnostics = Vec::new();
    for (index, source) in functions.into_iter().enumerate() {
        if !source.params.is_empty() {
            return Err(CompileError::OutsideScalarSlice("function parameters"));
        }
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
            dropped: HashSet::new(),
            tokens: lex::lex(&ast.span.file, &source_text)
                .map_err(|d| CompileError::Rejected(vec![d]))?,
        };
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
    user_functions: &'a HashSet<&'a str>,
    name_spans: Vec<Span>,
    dropped: HashSet<LocalId>,
    tokens: Vec<lex::Token>,
}
impl Lower<'_> {
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
        self.visible
            .get(name)
            .and_then(|v| v.last())
            .copied()
            .ok_or(CompileError::InvalidIr("unresolved local"))
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
        for statement in statements {
            if self.terminated.is_some() {
                return Err(CompileError::OutsideScalarSlice(
                    "unreachable tail after return",
                ));
            }
            let result = self.statement(statement, depth, diagnostics);
            match result {
                Err(CompileError::Rejected(errors)) => {
                    diagnostics.extend(errors.into_iter().map(|d| *d))
                }
                Err(other) => return Err(other),
                Ok(()) => {}
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
                let (temp, ty) = self.expr(value, 0, expected.as_ref())?;
                let id = LocalId(self.locals.len());
                self.name_spans
                    .push(self.name_span(statement, name, value.span().start)?);
                self.locals.push(Local {
                    id,
                    name: name.clone(),
                    ty,
                    mutable: *mutable,
                    scope: self.scope,
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
                    let id = self.get(name)?;
                    if !self.dropped.contains(&id) {
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
                let ty = self.locals[id.0].ty.clone();
                let op = if borrow && matches!(ty, Ty::Array(_)) {
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
                if self.user_functions.contains(name) {
                    return Err(CompileError::OutsideScalarSlice("user call"));
                }
                let intrinsic = match name {
                    "print" => Intrinsic::Print,
                    "assert_eq" => Intrinsic::AssertEq,
                    "drop" => Intrinsic::DropData,
                    "fail" => Intrinsic::Fail,
                    "len" => Intrinsic::Len,
                    _ => return Err(CompileError::OutsideScalarSlice("user or deferred call")),
                };
                if intrinsic == Intrinsic::DropData
                    && args.len() == 1
                    && let Some(binding) = place_name(&args[0].value)
                {
                    let local = self.get(binding)?;
                    self.dropped.insert(local);
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
                            Intrinsic::Print | Intrinsic::AssertEq | Intrinsic::Len
                        ),
                    )?;
                    ids.push(id);
                }
                let arity = match intrinsic {
                    Intrinsic::Print | Intrinsic::DropData | Intrinsic::Len => 1,
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
                        if intrinsic == Intrinsic::Len {
                            Ty::I64
                        } else {
                            Ty::Unit
                        },
                        span.clone(),
                        Access::Own,
                        intrinsic == Intrinsic::Print,
                    ),
                    if intrinsic == Intrinsic::Len {
                        Ty::I64
                    } else {
                        Ty::Unit
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
            Expr::Unary { .. } => Err(CompileError::OutsideScalarSlice(
                "consume, inout or result propagation",
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
