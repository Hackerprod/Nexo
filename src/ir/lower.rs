use std::collections::{HashMap, HashSet};

use super::model::{
    self, Access, BasicBlock, Binary, BlockId, Function, FunctionId, FunctionTable, Instruction,
    Intrinsic, Local, LocalId, Op, OriginId, Program, ScopeId, TempId, Terminator, Ty,
};
use crate::diagnostic::{Diagnostic, PROFILE_ID, Span};
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
    for (index, source) in functions.into_iter().enumerate() {
        if !source.params.is_empty() {
            return Err(CompileError::OutsideScalarSlice("function parameters"));
        }
        let result = match source.result.as_ref() {
            None | Some(Type::Unit) => Ty::Unit,
            Some(Type::I64) => Ty::I64,
            Some(Type::Bool) => Ty::Bool,
            _ => return Err(CompileError::OutsideScalarSlice("non-scalar result")),
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
            result,
            terminated: None,
            user_functions: &user_functions,
        };
        lower.block(&source.body.statements, 0)?;
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
}
impl Lower<'_> {
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
    fn block(&mut self, statements: &[Statement], depth: usize) -> Result<(), CompileError> {
        if depth >= 128 {
            return Err(CompileError::OutsideScalarSlice("lowering depth"));
        }
        for statement in statements {
            if self.terminated.is_some() {
                return Err(CompileError::OutsideScalarSlice(
                    "unreachable tail after return",
                ));
            }
            match statement {
                Statement::Let {
                    mutable,
                    name,
                    value,
                    ..
                } => {
                    let (temp, ty) = self.expr(value, 0)?;
                    let id = LocalId(self.locals.len());
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
                    let name = place_name(place)
                        .ok_or(CompileError::OutsideScalarSlice("indexed assignment"))?;
                    let id = self.get(name)?;
                    if !self.locals[id.0].mutable {
                        return Err(CompileError::OutsideScalarSlice(
                            "immutable write awaits ownership checking",
                        ));
                    }
                    let (temp, ty) = self.expr(value, 0)?;
                    if ty != self.locals[id.0].ty {
                        return Err(CompileError::InvalidIr("assignment type passed checker"));
                    }
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
                Statement::Expr { value, .. } => {
                    let (temp, _) = self.expr(value, 0)?;
                    self.statement_op(
                        Op::DropValue(temp),
                        statement_span(statement),
                        Access::Own,
                        false,
                    );
                }
                Statement::Return { value, .. } => {
                    let (temp, ty) = match value {
                        Some(e) => self.expr(e, 0)?,
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
                    self.block(&block.statements, depth + 1)?;
                    let dropping = self
                        .scopes
                        .last()
                        .ok_or(CompileError::InvalidIr("scope stack"))?
                        .clone();
                    for name in dropping.iter().rev() {
                        let id = self.get(name)?;
                        self.statement_op(
                            Op::DropLocal(id),
                            block.span.clone(),
                            Access::Own,
                            false,
                        );
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
        }
        Ok(())
    }
    fn expr(&mut self, e: &Expr, depth: usize) -> Result<(TempId, Ty), CompileError> {
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
                let ty = self.locals[id.0].ty;
                Ok((
                    self.emit(Op::CopyValue(id), ty, s.clone(), Access::Read, false),
                    ty,
                ))
            }
            Expr::Group(inner, _) => self.expr(inner, depth + 1),
            Expr::Binary {
                op,
                left,
                right,
                span,
            } => {
                let (l, lt) = self.expr(left, depth + 1)?;
                let (r, rt) = self.expr(right, depth + 1)?;
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
                        ty,
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
                    _ => return Err(CompileError::OutsideScalarSlice("user or deferred call")),
                };
                let mut ids = Vec::with_capacity(args.len());
                for arg in args {
                    let (id, _) = self.expr(&arg.value, depth + 1)?;
                    ids.push(id);
                }
                let arity = match intrinsic {
                    Intrinsic::Print | Intrinsic::DropData => 1,
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
                        Ty::Unit,
                        span.clone(),
                        Access::Own,
                        intrinsic == Intrinsic::Print,
                    ),
                    Ty::Unit,
                ))
            }
            Expr::Array(..) | Expr::Index { .. } => {
                Err(CompileError::OutsideScalarSlice("arrays and indexing"))
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
