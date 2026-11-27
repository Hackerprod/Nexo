//! Independent type checking for the accepted non-control core of F0-0002.
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use super::names::{self, Resolved, ResolvedUse};
use crate::diagnostic::{DataValue, Diagnostic, Operation, RelatedRole, RelatedSpan, Span, Stage};
use crate::frontend::{
    lex::{self, Kind, Token},
    parse::{
        self, Ast, Block, CaptureMode, Else, Expr, Function, Mode, Pattern, Statement,
        Type as SyntaxType,
    },
};
use crate::union::{UnionDescriptor, UnionKind};

#[cfg(test)]
#[path = "types/pass_meter.rs"]
mod pass_meter;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) enum InferTy {
    I64,
    Bool,
    Unit,
    MockResource,
    MockBlockingResource,
    Array(Arc<InferTy>),
    Union(UnionKind, Arc<InferTy>, Option<Arc<InferTy>>),
    Generator {
        site: u64,
        item: Arc<InferTy>,
    },
    Closure {
        site: u64,
        copyable: bool,
        owns_resource: bool,
    },
    Deferred(u64),
    Never,
    Tainted,
}

impl InferTy {
    fn result(ok: Arc<Self>, err: Arc<Self>) -> Self {
        Self::Union(UnionKind::Result, ok, Some(err))
    }

    fn option(some: Arc<Self>) -> Self {
        Self::Union(UnionKind::Option, some, None)
    }
}

pub(super) fn concrete_union(ty: &InferTy) -> Option<UnionDescriptor<&InferTy>> {
    match ty {
        InferTy::Union(UnionKind::Result, ok, Some(err)) => {
            Some(UnionDescriptor::result(ok.as_ref(), err.as_ref()))
        }
        InferTy::Union(UnionKind::Option, some, None) => {
            Some(UnionDescriptor::option(some.as_ref()))
        }
        _ => None,
    }
}

#[cfg(test)]
include!("i07c_quota_campaign.rs");

pub(super) type Type = InferTy;

impl From<&SyntaxType> for InferTy {
    fn from(value: &SyntaxType) -> Self {
        enum Frame<'a> {
            Enter(&'a SyntaxType),
            Array,
            Result,
            Option,
            Generator(u64),
        }
        let mut frames = vec![Frame::Enter(value)];
        let mut values = Vec::new();
        while let Some(frame) = frames.pop() {
            match frame {
                Frame::Enter(ty) => match ty {
                    SyntaxType::I64 => values.push(Self::I64),
                    SyntaxType::Bool => values.push(Self::Bool),
                    SyntaxType::Unit => values.push(Self::Unit),
                    SyntaxType::MockResource => values.push(Self::MockResource),
                    SyntaxType::MockBlockingResource => values.push(Self::MockBlockingResource),
                    SyntaxType::Array(inner) => {
                        frames.push(Frame::Array);
                        frames.push(Frame::Enter(inner));
                    }
                    SyntaxType::Result(ok, err) => {
                        frames.push(Frame::Result);
                        frames.push(Frame::Enter(err));
                        frames.push(Frame::Enter(ok));
                    }
                    SyntaxType::Option(some) => {
                        frames.push(Frame::Option);
                        frames.push(Frame::Enter(some));
                    }
                    SyntaxType::Generator { site, item } => {
                        frames.push(Frame::Generator(*site));
                        frames.push(Frame::Enter(item));
                    }
                    SyntaxType::Closure {
                        site,
                        copyable,
                        owns_resource,
                    } => values.push(Self::Closure {
                        site: *site,
                        copyable: *copyable,
                        owns_resource: *owns_resource,
                    }),
                },
                Frame::Array => {
                    let child = values.pop().unwrap();
                    values.push(Self::Array(Arc::new(child)));
                }
                Frame::Option => {
                    let child = values.pop().unwrap();
                    values.push(Self::option(Arc::new(child)));
                }
                Frame::Generator(site) => {
                    let child = values.pop().unwrap();
                    values.push(Self::Generator {
                        site,
                        item: Arc::new(child),
                    });
                }
                Frame::Result => {
                    let err = values.pop().unwrap();
                    let ok = values.pop().unwrap();
                    values.push(Self::result(Arc::new(ok), Arc::new(err)));
                }
            }
        }
        values.pop().unwrap()
    }
}

fn syntax_type(value: &InferTy) -> Option<SyntaxType> {
    enum Frame<'a> {
        Enter(&'a InferTy),
        Array,
        Result,
        Option,
        Generator(u64),
    }
    let mut frames = vec![Frame::Enter(value)];
    let mut values = Vec::new();
    while let Some(frame) = frames.pop() {
        match frame {
            Frame::Enter(ty) => match ty {
                InferTy::I64 => values.push(SyntaxType::I64),
                InferTy::Bool => values.push(SyntaxType::Bool),
                InferTy::Unit => values.push(SyntaxType::Unit),
                InferTy::MockResource => values.push(SyntaxType::MockResource),
                InferTy::MockBlockingResource => values.push(SyntaxType::MockBlockingResource),
                InferTy::Array(inner) => {
                    frames.push(Frame::Array);
                    frames.push(Frame::Enter(inner));
                }
                InferTy::Union(UnionKind::Result, ok, Some(err)) => {
                    frames.push(Frame::Result);
                    frames.push(Frame::Enter(err));
                    frames.push(Frame::Enter(ok));
                }
                InferTy::Union(UnionKind::Option, some, None) => {
                    frames.push(Frame::Option);
                    frames.push(Frame::Enter(some));
                }
                InferTy::Closure {
                    site,
                    copyable,
                    owns_resource,
                } => values.push(SyntaxType::Closure {
                    site: *site,
                    copyable: *copyable,
                    owns_resource: *owns_resource,
                }),
                InferTy::Generator { site, item } => {
                    frames.push(Frame::Generator(*site));
                    frames.push(Frame::Enter(item));
                }
                InferTy::Union(..) | InferTy::Deferred(_) | InferTy::Never | InferTy::Tainted => {
                    return None;
                }
            },
            Frame::Array => {
                let child = values.pop().unwrap();
                values.push(SyntaxType::Array(Box::new(child)));
            }
            Frame::Option => {
                let child = values.pop().unwrap();
                values.push(SyntaxType::Option(Box::new(child)));
            }
            Frame::Generator(site) => {
                let child = values.pop().unwrap();
                values.push(SyntaxType::Generator {
                    site,
                    item: Box::new(child),
                });
            }
            Frame::Result => {
                let err = values.pop().unwrap();
                let ok = values.pop().unwrap();
                values.push(SyntaxType::Result(Box::new(ok), Box::new(err)));
            }
        }
    }
    values.pop()
}

// Deferred projections keep a selector after its source visit. Clone every
// expression and nested block through one heap work list, including Array,
// Option/Result calls, branch bodies and patterns at the admitted depth.
fn clone_projection_selector(value: &Expr) -> Expr {
    enum Work<'a> {
        EnterExpr(&'a Expr),
        ExitExpr(&'a Expr),
        EnterBlock(&'a Block),
        ExitBlock(&'a Block),
        EnterStatement(&'a Statement),
        ExitStatement(&'a Statement),
        EnterElse(&'a Else),
        ExitElse(&'a Else),
        EnterMatch(&'a parse::Match),
        ExitMatch(&'a parse::Match),
        EnterArm(&'a parse::Arm),
        ExitArm(&'a parse::Arm),
        EnterPattern(&'a Pattern),
        ExitPattern(&'a Pattern),
        EnterClosure(&'a parse::Closure),
        ExitClosure(&'a parse::Closure),
    }
    enum Value {
        Expr(Expr),
        Block(Block),
        Statement(Statement),
        Else(Else),
        Match(parse::Match),
        Arm(parse::Arm),
        Pattern(Pattern),
        Closure(parse::Closure),
    }
    fn expr(values: &mut Vec<Value>) -> Expr {
        match values.pop().unwrap() {
            Value::Expr(value) => value,
            _ => unreachable!("expr"),
        }
    }
    fn block(values: &mut Vec<Value>) -> Block {
        match values.pop().unwrap() {
            Value::Block(value) => value,
            _ => unreachable!("block"),
        }
    }
    fn statement(values: &mut Vec<Value>) -> Statement {
        match values.pop().unwrap() {
            Value::Statement(value) => value,
            _ => unreachable!("statement"),
        }
    }
    fn branch(values: &mut Vec<Value>) -> Else {
        match values.pop().unwrap() {
            Value::Else(value) => value,
            _ => unreachable!("else"),
        }
    }
    fn selection(values: &mut Vec<Value>) -> parse::Match {
        match values.pop().unwrap() {
            Value::Match(value) => value,
            _ => unreachable!("match"),
        }
    }
    fn arm(values: &mut Vec<Value>) -> parse::Arm {
        match values.pop().unwrap() {
            Value::Arm(value) => value,
            _ => unreachable!("arm"),
        }
    }
    fn pattern(values: &mut Vec<Value>) -> Pattern {
        match values.pop().unwrap() {
            Value::Pattern(value) => value,
            _ => unreachable!("pattern"),
        }
    }
    fn closure(values: &mut Vec<Value>) -> parse::Closure {
        match values.pop().unwrap() {
            Value::Closure(value) => value,
            _ => unreachable!("closure"),
        }
    }
    fn syntax(value: &SyntaxType) -> SyntaxType {
        syntax_type(&Type::from(value)).unwrap()
    }
    let mut work = vec![Work::EnterExpr(value)];
    let mut values = Vec::new();
    while let Some(frame) = work.pop() {
        match frame {
            Work::EnterExpr(value) => {
                work.push(Work::ExitExpr(value));
                match value {
                    Expr::Array(items, _) => {
                        for item in items.iter().rev() {
                            work.push(Work::EnterExpr(item));
                        }
                    }
                    Expr::Group(inner, _)
                    | Expr::Unary { value: inner, .. }
                    | Expr::Neg { value: inner, .. }
                    | Expr::Not { value: inner, .. } => {
                        work.push(Work::EnterExpr(inner));
                    }
                    Expr::Binary { left, right, .. } => {
                        work.push(Work::EnterExpr(right));
                        work.push(Work::EnterExpr(left));
                    }
                    Expr::Call { target, args, .. } => {
                        for arg in args.iter().rev() {
                            work.push(Work::EnterExpr(&arg.value));
                        }
                        work.push(Work::EnterExpr(target));
                    }
                    Expr::Index { target, index, .. } => {
                        work.push(Work::EnterExpr(index));
                        work.push(Work::EnterExpr(target));
                    }
                    Expr::If {
                        condition,
                        then_block,
                        otherwise,
                        ..
                    } => {
                        if let Some(otherwise) = otherwise {
                            work.push(Work::EnterElse(otherwise));
                        }
                        work.push(Work::EnterBlock(then_block));
                        work.push(Work::EnterExpr(condition));
                    }
                    Expr::Match(selection) => work.push(Work::EnterMatch(selection)),
                    Expr::Closure(closure) => work.push(Work::EnterClosure(closure)),
                    Expr::Name(..) | Expr::Integer(..) | Expr::Bool(..) | Expr::Unit(..) => {}
                }
            }
            Work::ExitExpr(value) => {
                let cloned = match value {
                    Expr::Name(name, span) => Expr::Name(name.clone(), span.clone()),
                    Expr::Integer(text, span) => Expr::Integer(text.clone(), span.clone()),
                    Expr::Bool(value, span) => Expr::Bool(*value, span.clone()),
                    Expr::Unit(span) => Expr::Unit(span.clone()),
                    Expr::Array(items, span) => {
                        let mut cloned = Vec::with_capacity(items.len());
                        for _ in items {
                            cloned.push(expr(&mut values));
                        }
                        cloned.reverse();
                        Expr::Array(cloned, span.clone())
                    }
                    Expr::Group(_, span) => Expr::Group(Box::new(expr(&mut values)), span.clone()),
                    Expr::Unary { mode, span, .. } => Expr::Unary {
                        mode: *mode,
                        value: Box::new(expr(&mut values)),
                        span: span.clone(),
                    },
                    Expr::Neg { span, .. } => Expr::Neg {
                        value: Box::new(expr(&mut values)),
                        span: span.clone(),
                    },
                    Expr::Not { span, .. } => Expr::Not {
                        value: Box::new(expr(&mut values)),
                        span: span.clone(),
                    },
                    Expr::Binary { op, span, .. } => {
                        let right = expr(&mut values);
                        let left = expr(&mut values);
                        Expr::Binary {
                            op: op.clone(),
                            left: Box::new(left),
                            right: Box::new(right),
                            span: span.clone(),
                        }
                    }
                    Expr::Call { args, span, .. } => {
                        let mut cloned = Vec::with_capacity(args.len());
                        for arg in args.iter().rev() {
                            cloned.push(parse::Argument {
                                label: arg.label.clone(),
                                value: expr(&mut values),
                                span: arg.span.clone(),
                            });
                        }
                        cloned.reverse();
                        Expr::Call {
                            target: Box::new(expr(&mut values)),
                            args: cloned,
                            span: span.clone(),
                        }
                    }
                    Expr::Index { selector, span, .. } => {
                        let index = expr(&mut values);
                        let target = expr(&mut values);
                        Expr::Index {
                            target: Box::new(target),
                            index: Box::new(index),
                            selector: selector.clone(),
                            span: span.clone(),
                        }
                    }
                    Expr::If {
                        otherwise, span, ..
                    } => {
                        let branch = otherwise.as_ref().map(|_| Box::new(branch(&mut values)));
                        let then_block = block(&mut values);
                        let condition = expr(&mut values);
                        Expr::If {
                            condition: Box::new(condition),
                            then_block,
                            otherwise: branch,
                            span: span.clone(),
                        }
                    }
                    Expr::Match(_) => Expr::Match(Box::new(selection(&mut values))),
                    Expr::Closure(_) => Expr::Closure(Box::new(closure(&mut values))),
                };
                values.push(Value::Expr(cloned));
            }
            Work::EnterBlock(value) => {
                work.push(Work::ExitBlock(value));
                for statement in value.statements.iter().rev() {
                    work.push(Work::EnterStatement(statement));
                }
            }
            Work::ExitBlock(value) => {
                let mut statements = Vec::with_capacity(value.statements.len());
                for _ in &value.statements {
                    statements.push(statement(&mut values));
                }
                statements.reverse();
                values.push(Value::Block(Block {
                    statements,
                    span: value.span.clone(),
                    close_span: value.close_span.clone(),
                }));
            }
            Work::EnterStatement(value) => {
                work.push(Work::ExitStatement(value));
                match value {
                    Statement::Let { value, .. } | Statement::Expr { value, .. } => {
                        work.push(Work::EnterExpr(value))
                    }
                    Statement::Assign { place, value, .. } => {
                        work.push(Work::EnterExpr(value));
                        work.push(Work::EnterExpr(place));
                    }
                    Statement::Return { value, .. }
                    | Statement::GuardControl { value, .. }
                    | Statement::Value { value, .. } => {
                        if let Some(value) = value {
                            work.push(Work::EnterExpr(value));
                        }
                    }
                    Statement::If {
                        condition,
                        then_block,
                        otherwise,
                        ..
                    } => {
                        if let Some(otherwise) = otherwise {
                            work.push(Work::EnterStatement(otherwise));
                        }
                        work.push(Work::EnterBlock(then_block));
                        work.push(Work::EnterExpr(condition));
                    }
                    Statement::While {
                        condition, body, ..
                    } => {
                        work.push(Work::EnterBlock(body));
                        work.push(Work::EnterExpr(condition));
                    }
                    Statement::For { iterable, body, .. } => {
                        work.push(Work::EnterBlock(body));
                        work.push(Work::EnterExpr(iterable));
                    }
                    Statement::Match(selection) => work.push(Work::EnterMatch(selection)),
                    Statement::Block(block) => work.push(Work::EnterBlock(block)),
                }
            }
            Work::ExitStatement(value) => {
                let cloned = match value {
                    Statement::Let {
                        mutable,
                        name,
                        annotation,
                        span,
                        ..
                    } => Statement::Let {
                        mutable: *mutable,
                        name: name.clone(),
                        annotation: annotation.as_ref().map(syntax),
                        value: expr(&mut values),
                        span: span.clone(),
                    },
                    Statement::Assign { op, span, .. } => {
                        let value = expr(&mut values);
                        let place = expr(&mut values);
                        Statement::Assign {
                            place,
                            op: *op,
                            value,
                            span: span.clone(),
                        }
                    }
                    Statement::Return { value, span } => Statement::Return {
                        value: value.as_ref().map(|_| expr(&mut values)),
                        span: span.clone(),
                    },
                    Statement::GuardControl {
                        token,
                        value,
                        guarded,
                        token_span,
                        span,
                    } => Statement::GuardControl {
                        token: token.clone(),
                        value: value.as_ref().map(|_| expr(&mut values)),
                        guarded: *guarded,
                        token_span: token_span.clone(),
                        span: span.clone(),
                    },
                    Statement::Value {
                        value,
                        marker,
                        span,
                    } => Statement::Value {
                        value: value.as_ref().map(|_| expr(&mut values)),
                        marker: marker.clone(),
                        span: span.clone(),
                    },
                    Statement::If {
                        otherwise, span, ..
                    } => {
                        let otherwise =
                            otherwise.as_ref().map(|_| Box::new(statement(&mut values)));
                        let then_block = block(&mut values);
                        let condition = expr(&mut values);
                        Statement::If {
                            condition,
                            then_block,
                            otherwise,
                            span: span.clone(),
                        }
                    }
                    Statement::While { span, .. } => {
                        let body = block(&mut values);
                        let condition = expr(&mut values);
                        Statement::While {
                            condition,
                            body,
                            span: span.clone(),
                        }
                    }
                    Statement::For {
                        name,
                        name_span,
                        span,
                        ..
                    } => {
                        let body = block(&mut values);
                        let iterable = expr(&mut values);
                        Statement::For {
                            name: name.clone(),
                            name_span: name_span.clone(),
                            iterable,
                            body,
                            span: span.clone(),
                        }
                    }
                    Statement::Match(_) => Statement::Match(selection(&mut values)),
                    Statement::Block(_) => Statement::Block(block(&mut values)),
                    Statement::Expr { span, .. } => Statement::Expr {
                        value: expr(&mut values),
                        span: span.clone(),
                    },
                };
                values.push(Value::Statement(cloned));
            }
            Work::EnterElse(value) => {
                work.push(Work::ExitElse(value));
                match value {
                    Else::Block(block) => work.push(Work::EnterBlock(block)),
                    Else::If(expr) => work.push(Work::EnterExpr(expr)),
                }
            }
            Work::ExitElse(value) => {
                let cloned = match value {
                    Else::Block(_) => Else::Block(block(&mut values)),
                    Else::If(_) => Else::If(Box::new(expr(&mut values))),
                };
                values.push(Value::Else(cloned));
            }
            Work::EnterMatch(value) => {
                work.push(Work::ExitMatch(value));
                for arm in value.arms.iter().rev() {
                    work.push(Work::EnterArm(arm));
                }
                work.push(Work::EnterExpr(&value.scrutinee));
            }
            Work::ExitMatch(value) => {
                let mut arms = Vec::with_capacity(value.arms.len());
                for _ in &value.arms {
                    arms.push(arm(&mut values));
                }
                arms.reverse();
                let scrutinee = Box::new(expr(&mut values));
                values.push(Value::Match(parse::Match {
                    scrutinee,
                    mode: value.mode,
                    mode_span: value.mode_span.clone(),
                    arms,
                    span: value.span.clone(),
                    keyword_span: value.keyword_span.clone(),
                }));
            }
            Work::EnterArm(value) => {
                work.push(Work::ExitArm(value));
                work.push(Work::EnterBlock(&value.body));
                if let Some(guard) = &value.guard {
                    work.push(Work::EnterExpr(guard));
                }
                work.push(Work::EnterPattern(&value.pattern));
            }
            Work::ExitArm(value) => {
                let body = block(&mut values);
                let guard = value.guard.as_ref().map(|_| expr(&mut values));
                let pattern = pattern(&mut values);
                values.push(Value::Arm(parse::Arm {
                    pattern,
                    guard,
                    guard_span: value.guard_span.clone(),
                    body,
                    span: value.span.clone(),
                }));
            }
            Work::EnterPattern(value) => {
                work.push(Work::ExitPattern(value));
                if let Pattern::Constructor { args, .. } = value {
                    for arg in args.iter().rev() {
                        work.push(Work::EnterPattern(arg));
                    }
                }
            }
            Work::ExitPattern(value) => {
                let cloned = match value {
                    Pattern::Wildcard(span) => Pattern::Wildcard(span.clone()),
                    Pattern::Integer(text, span) => Pattern::Integer(text.clone(), span.clone()),
                    Pattern::Bool(value, span) => Pattern::Bool(*value, span.clone()),
                    Pattern::Name { name, mode, span } => Pattern::Name {
                        name: name.clone(),
                        mode: *mode,
                        span: span.clone(),
                    },
                    Pattern::Constructor { name, args, span } => {
                        let mut cloned = Vec::with_capacity(args.len());
                        for _ in args {
                            cloned.push(pattern(&mut values));
                        }
                        cloned.reverse();
                        Pattern::Constructor {
                            name: name.clone(),
                            args: cloned,
                            span: span.clone(),
                        }
                    }
                };
                values.push(Value::Pattern(cloned));
            }
            Work::EnterClosure(value) => {
                work.push(Work::ExitClosure(value));
                work.push(Work::EnterBlock(&value.body));
            }
            Work::ExitClosure(value) => {
                let body = block(&mut values);
                let params = value
                    .params
                    .iter()
                    .map(|parameter| parse::ClosureParameter {
                        mode: parameter.mode,
                        name: parameter.name.clone(),
                        ty: parameter.ty.as_ref().map(syntax),
                        span: parameter.span.clone(),
                    })
                    .collect();
                values.push(Value::Closure(parse::Closure {
                    fn_span: value.fn_span.clone(),
                    captures: value.captures.clone(),
                    params,
                    result: value.result.as_ref().map(syntax),
                    with_clause: value.with_clause.clone(),
                    body,
                    span: value.span.clone(),
                }));
            }
        }
    }
    expr(&mut values)
}

#[cfg(test)]
pub(crate) static TYPE_SOLVER_WORK: std::sync::atomic::AtomicU64 =
    std::sync::atomic::AtomicU64::new(0);

fn span_key(span: &Span) -> (u64, u64) {
    (span.start, span.end)
}

fn pretty(ty: &Type) -> Option<String> {
    let ty = syntax_type(ty)?;
    enum Part<'a> {
        Type(&'a SyntaxType),
        Text(&'static str),
    }
    let mut output = String::new();
    let mut work = vec![Part::Type(&ty)];
    while let Some(part) = work.pop() {
        match part {
            Part::Text(text) => output.push_str(text),
            Part::Type(SyntaxType::I64) => output.push_str("I64"),
            Part::Type(SyntaxType::Bool) => output.push_str("Bool"),
            Part::Type(SyntaxType::Unit) => output.push_str("()"),
            Part::Type(SyntaxType::MockResource) => output.push_str("MockResource"),
            Part::Type(SyntaxType::MockBlockingResource) => output.push_str("MockBlockingResource"),
            Part::Type(SyntaxType::Array(inner)) => {
                work.push(Part::Text(">"));
                work.push(Part::Type(inner));
                work.push(Part::Text("Array<"));
            }
            Part::Type(SyntaxType::Result(ok, err)) => {
                work.push(Part::Text(">"));
                work.push(Part::Type(err));
                work.push(Part::Text(","));
                work.push(Part::Type(ok));
                work.push(Part::Text("Result<"));
            }
            Part::Type(SyntaxType::Option(some)) => {
                work.push(Part::Text(">"));
                work.push(Part::Type(some));
                work.push(Part::Text("Option<"));
            }
            Part::Type(SyntaxType::Closure { site, .. }) => {
                output.push_str("Closure@");
                output.push_str(&site.to_string());
            }
            Part::Type(SyntaxType::Generator { site, .. }) => {
                output.push_str("Generator@");
                output.push_str(&site.to_string());
            }
        }
    }
    Some(output)
}

fn unresolved_type(primary: Span) -> Box<Diagnostic> {
    names::diagnostic(
        "NX-E0103",
        None,
        Stage::Types,
        Operation::TypesInfer,
        primary,
    )
}

fn copyable(ty: &Type) -> bool {
    let mut work = vec![ty];
    while let Some(node) = work.pop() {
        match node {
            Type::I64 | Type::Bool | Type::Unit => {}
            Type::Array(inner) => work.push(inner),
            Type::Union(_, first, second) => {
                if let Some(second) = second {
                    work.push(second);
                }
                work.push(first);
            }
            Type::Closure {
                copyable: false, ..
            }
            | Type::MockResource
            | Type::MockBlockingResource => return false,
            Type::Closure { copyable: true, .. } => {}
            Type::Generator { .. } => return false,
            Type::Deferred(_) | Type::Never | Type::Tainted => return false,
        }
    }
    true
}

fn comparable_data(ty: &Type) -> bool {
    let mut work = vec![ty];
    while let Some(node) = work.pop() {
        match node {
            Type::Closure { .. } => return false,
            Type::Array(inner) => work.push(inner),
            Type::Union(_, first, second) => {
                work.push(first);
                if let Some(second) = second {
                    work.push(second);
                }
            }
            _ => {}
        }
    }
    copyable(ty)
}

fn domain_accepts(domain: &str, ty: &Type) -> bool {
    match domain {
        "domain:array" => matches!(ty, Type::Array(_)),
        "domain:copyable_data" => comparable_data(ty),
        "domain:mock_resource" => {
            matches!(ty, Type::MockResource | Type::MockBlockingResource)
        }
        _ => unreachable!("closed intrinsic domains"),
    }
}

fn internal_state(ty: &Type) -> Option<Type> {
    let mut work = vec![ty];
    let mut state = None;
    while let Some(node) = work.pop() {
        match node {
            Type::Tainted => return Some(Type::Tainted),
            Type::Never => state = Some(Type::Never),
            Type::Deferred(id) if state.is_none() => state = Some(Type::Deferred(*id)),
            Type::Array(child) => work.push(child),
            Type::Union(_, first, second) => {
                if let Some(second) = second {
                    work.push(second);
                }
                work.push(first);
            }
            _ => {}
        }
    }
    state
}

fn contains_resource(ty: &Type) -> bool {
    let mut work = vec![ty];
    while let Some(node) = work.pop() {
        match node {
            Type::MockResource | Type::MockBlockingResource => return true,
            Type::Generator { .. } => return true,
            Type::Closure {
                owns_resource: true,
                ..
            } => return true,
            Type::Closure {
                owns_resource: false,
                ..
            } => {}
            Type::Array(inner) => work.push(inner),
            Type::Union(_, first, second) => {
                if let Some(second) = second {
                    work.push(second);
                }
                work.push(first);
            }
            Type::I64 | Type::Bool | Type::Unit => {}
            Type::Deferred(_) | Type::Never | Type::Tainted => {}
        }
    }
    false
}

fn has_resource_array(ty: &Type) -> bool {
    let mut work = vec![ty];
    while let Some(node) = work.pop() {
        match node {
            Type::Array(inner) => {
                if contains_resource(inner) {
                    return true;
                }
                work.push(inner);
            }
            Type::Union(_, first, second) => {
                if let Some(second) = second {
                    work.push(second);
                }
                work.push(first);
            }
            Type::I64
            | Type::Bool
            | Type::Unit
            | Type::MockResource
            | Type::MockBlockingResource
            | Type::Closure { .. }
            | Type::Generator { .. }
            | Type::Deferred(_)
            | Type::Never
            | Type::Tainted => {}
        }
    }
    false
}

fn has_closure_array(ty: &Type) -> bool {
    let mut work = vec![(ty, false)];
    while let Some((node, within_array)) = work.pop() {
        match node {
            Type::Array(inner) => work.push((inner, true)),
            Type::Union(_, first, second) => {
                work.push((first, within_array));
                if let Some(second) = second {
                    work.push((second, within_array));
                }
            }
            Type::Closure { .. } if within_array => return true,
            _ => {}
        }
    }
    false
}

fn resource_array(span: Span) -> Box<Diagnostic> {
    let mut d = names::diagnostic("NX-E0006", None, Stage::Types, Operation::TypesCheck, span);
    d.feature_id = Some("resource_array".into());
    d.data.insert(
        "pending_scope".into(),
        DataValue::String("later_adr".into()),
    );
    d
}

fn mismatch(
    primary: Span,
    found_span: Span,
    expected_span: Option<Span>,
    expected: &str,
    found: &Type,
) -> Box<Diagnostic> {
    // An unresolved expected type cannot form a schema-valid mismatch.
    // Its constructor owns the inference diagnostic instead.
    if expected.is_empty() {
        return unresolved_type(primary);
    }
    let Some(found_label) = pretty(found) else {
        return unresolved_type(primary);
    };
    let mut d = names::diagnostic(
        "NX-E0102",
        Some("type_mismatch"),
        Stage::Types,
        Operation::TypesCheck,
        primary,
    );
    d.data
        .insert("expected_type".into(), DataValue::String(expected.into()));
    d.data
        .insert("found_type".into(), DataValue::String(found_label));
    if let Some(span) = expected_span.filter(|span| span != d.primary_span.as_ref().unwrap()) {
        d.related_spans.push(RelatedSpan {
            role: RelatedRole::ExpectedType,
            span,
        });
    }
    d.related_spans.push(RelatedSpan {
        role: RelatedRole::FoundType,
        span: found_span,
    });
    d
}

fn array_mismatch(mut error: Box<Diagnostic>, expected: &Type, found: &Type) -> Box<Diagnostic> {
    if ((error.code == "NX-E0102" && error.subcause.as_deref() == Some("type_mismatch"))
        || error.code == "NX-E1303")
        && let (Some(expected), Some(found)) = (pretty(expected), pretty(found))
    {
        error
            .data
            .insert("expected_type".into(), DataValue::String(expected));
        error
            .data
            .insert("found_type".into(), DataValue::String(found));
    }
    error
}

fn domain_error(
    subcause: &str,
    primary: Span,
    found_span: Span,
    expected_span: Option<Span>,
    expected: &str,
    found: &Type,
) -> Box<Diagnostic> {
    let mut d = mismatch(primary, found_span, expected_span, expected, found);
    if d.code != "NX-E0102" {
        // An unresolved operand has no serializable domain mismatch. In
        // particular, never relabel its inference diagnostic as R0102.
        return d;
    }
    d.subcause = Some(subcause.into());
    d.rule = format!("NX-E0102/{subcause}");
    d.explanation_id = Some(d.rule.clone());
    d
}

#[derive(Clone)]
struct Known {
    ty: Type,
    origin: Span,
    /// Stable traversal-order identity of an unresolved monomorphic binding.
    variable: Option<u64>,
    invalid: bool,
    nominal_sources: HashSet<(u64, u64)>,
}

#[derive(Clone)]
struct ConcreteEvidence {
    ty: Type,
    origin: Span,
    intrinsic: bool,
}

#[derive(Clone)]
struct TypeNode {
    parent: u64,
    size: usize,
    concrete: Option<(Type, Span)>,
    concrete_intrinsic: bool,
    array_child: Option<u64>,
    array_origin: Option<Span>,
    array_constructed: bool,
    variant_children: Option<UnionDescriptor<u64>>,
    variant_origin: Option<Span>,
    variant_constructed: bool,
    bottom_only: bool,
    failed_constructor: bool,
    deferred_join: bool,
    recursive_join: bool,
    branch_join_scopes: Vec<(u64, u64)>,
    // Only presence is tested by bind/union. The earliest span is retained
    // for deterministic propagation, but changing one Some(span) to another
    // Some(span) cannot alter acceptance or a diagnostic: neither site emits
    // this span (the equality copyability rule is §4.6).
    copyable_origin: Option<Span>,
}

// Ported from the R6 prototype: capture each node on its first write in an
// equality. Node IDs and the solver budget remain monotonic on rollback.
#[derive(Clone, Default)]
struct NodeJournal {
    before: HashMap<u64, Option<TypeNode>>,
    order: Vec<u64>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
struct EqualityId(u64);

#[derive(Clone)]
struct EqualityRecord {
    id: EqualityId,
    operands: (Type, Type),
    found: Span,
    join_start: usize,
    state: EqualityState,
}

#[derive(Clone, Copy)]
enum EqualityState {
    Attempting,
    Rejected,
    Waiting,
    Committed,
}

#[derive(Clone)]
struct PendingDeferredJoin {
    equality: Option<EqualityId>,
    left: Type,
    right: Type,
    constructor: u64,
    left_origin: Span,
    found: Span,
    recursive_origin: Option<Span>,
    branch: bool,
    related_expected: bool,
    numeric_domain_owner: Option<(u64, u64)>,
    comparison_domain_if_unresolved: Option<PendingComparisonDomain>,
}

#[derive(Clone)]
struct PendingComparisonDomain {
    left: Type,
    operator: Span,
    found: Span,
    domain: &'static str,
}

#[derive(Clone)]
struct PendingDomain {
    variable: u64,
    operand: Span,
    domain: &'static str,
}

#[derive(Clone)]
struct PendingNominalDomain {
    variable: u64,
    domain: &'static str,
    call: Span,
    nominal: Type,
    expected: Option<Type>,
    expected_span: Option<Span>,
    depends_on: Option<Span>,
    conditional_bindings: Vec<(u64, Type)>,
}

#[derive(Clone)]
struct PendingBinaryDomain {
    variable: u64,
    operator: Span,
    found: Span,
    domain: &'static str,
    result: Span,
    conditional_bindings: Vec<(u64, Type)>,
}

#[derive(Clone)]
struct PendingGuardedComparison {
    variable: u64,
    domain: &'static str,
    expected: Type,
    found: Type,
    expected_origin: Span,
    found_site: Span,
    result: Span,
    consumer_expected: Option<Type>,
    consumer_origin: Option<Span>,
    domain_error_if_equal: Option<Box<Diagnostic>>,
    additional_domains: Vec<(u64, &'static str)>,
    projections: Vec<(u64, u64)>,
    dependent_results: Vec<Span>,
    require_resolved: bool,
    conditional_bindings: Vec<(u64, Type)>,
}

#[derive(Default)]
struct PendingOperandEvidence {
    domains: Vec<(u64, &'static str)>,
    projections: Vec<(u64, u64)>,
    results: Vec<Span>,
}

#[derive(Clone)]
struct PendingProjection {
    base: u64,
    item: u64,
    base_span: Span,
    result_span: Span,
    selector: Arc<Expr>,
    selector_occurrence_aliases: Vec<(OccurrenceKey, OccurrenceId)>,
    selector_variable: Option<u64>,
    checked_without_context: bool,
    selector_invalid: bool,
    expected: Option<Type>,
    expected_span: Option<Span>,
    related_expected: bool,
}

#[derive(Clone)]
struct ReadPlan {
    ty: Type,
    blocked: Vec<(u64, u64)>,
    assignment_start: u64,
}

#[derive(Clone, PartialEq, Eq)]
enum ShapeHint {
    Unknown,
    Scalar(Type),
    Array(Box<ShapeHint>),
}

#[derive(Clone)]
enum ObligationKind {
    Constructor {
        child: u64,
        array: u64,
        span: Span,
        has_element_witness: bool,
    },
    VariantConstructor {
        union: u64,
        span: Span,
    },
    PatternConstructor {
        union: u64,
        span: Span,
    },
    Join(Box<PendingDeferredJoin>),
    Domain(PendingDomain),
    DomainNominal(Box<PendingNominalDomain>),
    BinaryDomain(Box<PendingBinaryDomain>),
    GuardedComparison(Box<PendingGuardedComparison>),
    Projection(Box<PendingProjection>),
}

#[derive(Clone, Copy)]
enum ObligationClass {
    Constructor = 0,
    VariantConstructor = 1,
    Join = 2,
    Domain = 3,
    DomainNominal = 4,
    BinaryDomain = 5,
    GuardedComparison = 6,
    Projection = 7,
    PatternConstructor = 8,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum ObligationStatus {
    Pending,
    Satisfied,
    Refuted,
    Contaminated,
}

impl ObligationKind {
    fn occurrence(&self) -> (ObligationClass, (u64, u64)) {
        match self {
            Self::Constructor { span, .. } => (ObligationClass::Constructor, span_key(span)),
            Self::VariantConstructor { span, .. } => {
                (ObligationClass::VariantConstructor, span_key(span))
            }
            Self::PatternConstructor { span, .. } => {
                (ObligationClass::PatternConstructor, span_key(span))
            }
            Self::Join(join) => (ObligationClass::Join, span_key(&join.found)),
            Self::Domain(domain) => (ObligationClass::Domain, span_key(&domain.operand)),
            Self::DomainNominal(domain) => (ObligationClass::DomainNominal, span_key(&domain.call)),
            Self::BinaryDomain(domain) => (ObligationClass::BinaryDomain, span_key(&domain.result)),
            Self::GuardedComparison(comparison) => (
                ObligationClass::GuardedComparison,
                span_key(&comparison.result),
            ),
            Self::Projection(projection) => (
                ObligationClass::Projection,
                span_key(&projection.result_span),
            ),
        }
    }
}

#[derive(Default)]
pub(crate) struct InferredTypes {
    pub bindings: HashMap<u64, ResolvedType>,
    pub expressions: HashMap<(u64, u64), ResolvedType>,
    pub closures: HashMap<u64, ClosureInfo>,
}

#[derive(Clone, Debug)]
pub(crate) struct ClosureCaptureInfo {
    pub name: String,
    pub mode: CaptureMode,
    pub usage: Mode,
    pub source: Span,
    pub capture_span: Span,
    pub ty: SyntaxType,
    pub mutable: bool,
}

#[derive(Clone, Debug)]
pub(crate) struct ClosureInfo {
    pub site: u64,
    pub required_mode: Mode,
    pub captures: Vec<ClosureCaptureInfo>,
    pub params: Vec<(String, Mode, SyntaxType)>,
    pub result: SyntaxType,
}

#[derive(Clone)]
struct ClosureDraft {
    captures: Vec<(String, CaptureMode, Mode, Span, Span, Type, bool)>,
    params: Vec<(String, Mode, Span, Type)>,
    result: Type,
}

struct PreparedLet {
    annotation_span: Option<Span>,
    written: Option<Type>,
}

fn compound_operand_type(op: parse::AssignOp) -> Option<Type> {
    let base = op.text().strip_suffix('=')?;
    super::operators::binary(base).map(|(domain, result)| {
        debug_assert_eq!(domain, super::operators::Domain::Integer);
        Type::from(&result)
    })
}

struct AssignmentProgress<'a> {
    place: &'a Expr,
    value: &'a Expr,
    local: Option<u64>,
    concrete: Option<(Type, Span)>,
    expected: Option<(Type, Span)>,
    parent_fact: FactId,
}

struct BinaryIndependentProgress<'a> {
    expr: &'a Expr,
    context: OwnedCheckContext,
    left: Type,
    pending_before: usize,
    left_domain_failed: bool,
    binary_fact: FactId,
}

struct IndexSelectorProgress<'a> {
    expr: &'a Expr,
    context: OwnedCheckContext,
    base: Result<Type, Box<Diagnostic>>,
    parent_fact: FactId,
    deferred_child: Option<u64>,
}

struct ArrayFrameState<'a> {
    expr: &'a Expr,
    items: &'a [Expr],
    consumer: ConsumerId,
    span: &'a Span,
    context: OwnedCheckContext,
    parent_fact: FactId,
    element_expected: Option<Type>,
    next: usize,
    current_expected: Option<Type>,
    current_origin: Option<Span>,
    item_type: Option<Type>,
    item_origin: Option<Span>,
    pending: Vec<(u64, Span)>,
    diverges: bool,
    invalid: bool,
}

struct BinaryRightProgress<'a> {
    expr: &'a Expr,
    context: OwnedCheckContext,
    left: Type,
    pending_before: usize,
    binary_fact: FactId,
}

struct BinaryDeferredProgress<'a> {
    expr: &'a Expr,
    context: OwnedCheckContext,
    left: Type,
    pending_before_left: usize,
    pending_before_operator: usize,
    binary_fact: FactId,
}

struct BinaryLeftErrorProgress<'a> {
    expr: &'a Expr,
    context: OwnedCheckContext,
    error: Box<Diagnostic>,
    left_domain_failed: bool,
    binary_fact: FactId,
}

struct CallProgress<'a> {
    expr: &'a Expr,
    context: OwnedCheckContext,
    arg_index: usize,
    parent_fact: FactId,
}

struct ClosureCallFrameState<'a> {
    expr: &'a Expr,
    context: OwnedCheckContext,
    draft: ClosureDraft,
    slots: Vec<usize>,
    next: usize,
    parent_fact: FactId,
}

struct TryAddFrameState<'a> {
    expr: &'a Expr,
    context: OwnedCheckContext,
    next: usize,
    failed: bool,
    parent_fact: FactId,
}

struct AssertEqFrameState<'a> {
    expr: &'a Expr,
    context: OwnedCheckContext,
    next: usize,
    call_fact: FactId,
    prior_errors: usize,
    values: [Option<Type>; 2],
    evidence: [Option<PendingOperandEvidence>; 2],
    invalid: bool,
}

struct UnionCallProgress<'a> {
    expr: &'a Expr,
    context: OwnedCheckContext,
    parent_fact: FactId,
}

struct ClosureProgress<'a> {
    closure: &'a parse::Closure,
    context: OwnedCheckContext,
    prior_sites: HashSet<u64>,
    draft: ClosureDraft,
    previous_result: Type,
    previous_result_span: Option<Span>,
    previous_generator_item: Option<Type>,
    previous_loop_depth: usize,
    copyable: bool,
    owns_resource: bool,
    next: usize,
    pending_statement: Option<ClosurePending<'a>>,
    pending_direct: bool,
}

struct ClosureLet<'a> {
    statement: &'a Statement,
    prepared: PreparedLet,
    parent_fact: FactId,
}

enum ClosurePending<'a> {
    Let(ClosureLet<'a>),
    Return {
        statement: &'a Statement,
        expected: Type,
        parent_fact: FactId,
    },
}

enum ClosureAction<'a> {
    Value(&'a Expr, OwnedCheckContext),
    Statement(&'a Statement),
    Done(Result<(), Box<Diagnostic>>),
}

fn capture_usage(closure: &parse::Closure, source: u64, resolved: &Resolved) -> Mode {
    enum Node<'a> {
        Expr(&'a Expr, Mode),
        Name(&'a Span, Mode),
        Block(&'a Block),
        Statement(&'a Statement),
        Match(&'a parse::Match),
    }
    fn rank(mode: Mode) -> u8 {
        match mode {
            Mode::Read => 0,
            Mode::Inout => 1,
            Mode::Consume => 2,
        }
    }
    let mut found = Mode::Read;
    let mut pending = vec![Node::Block(&closure.body)];
    while let Some(node) = pending.pop() {
        match node {
            Node::Name(span, requested) => {
                if matches!(resolved.uses.get(&span.start), Some(ResolvedUse::Local(local)) if local.name_span.start == source)
                    && rank(requested) > rank(found)
                {
                    found = requested;
                }
            }
            Node::Expr(value, requested) => match value {
                Expr::Name(_, span) => pending.push(Node::Name(span, requested)),
                Expr::Closure(inner) => {
                    if let Some(captures) = &inner.captures {
                        for capture in captures.iter().rev() {
                            let mode = match capture.mode {
                                CaptureMode::View | CaptureMode::Copy => Mode::Read,
                                CaptureMode::Edit => Mode::Inout,
                                CaptureMode::Move => Mode::Consume,
                            };
                            pending.push(Node::Name(&capture.name_span, mode));
                        }
                    } else {
                        pending.push(Node::Block(&inner.body));
                    }
                }
                Expr::Unary { mode, value, .. } => {
                    pending.push(Node::Expr(
                        value,
                        if rank(*mode) > rank(requested) {
                            *mode
                        } else {
                            requested
                        },
                    ));
                }
                Expr::Neg { value, .. } | Expr::Not { value, .. } => {
                    pending.push(Node::Expr(value, requested));
                }
                Expr::Group(inner, _) => pending.push(Node::Expr(inner, requested)),
                Expr::Array(items, _) => {
                    pending.extend(items.iter().rev().map(|item| Node::Expr(item, Mode::Read)));
                }
                Expr::Binary { left, right, .. }
                | Expr::Index {
                    target: left,
                    index: right,
                    ..
                } => {
                    pending.push(Node::Expr(right, Mode::Read));
                    pending.push(Node::Expr(left, requested));
                }
                Expr::Call { target, args, span } => {
                    let signature =
                        resolved
                            .calls
                            .get(&(span.start, span.end))
                            .and_then(|binding| {
                                names::signature(resolved, &binding.target)
                                    .map(|signature| (binding, signature))
                            });
                    for (index, arg) in args.iter().enumerate().rev() {
                        let mode = if index == 0
                            && matches!(&**target, Expr::Name(name, _) if name == "Drop")
                        {
                            Mode::Consume
                        } else {
                            signature
                                .as_ref()
                                .and_then(|(binding, signature)| {
                                    binding
                                        .slots
                                        .get(index)
                                        .and_then(|slot| signature.params.get(*slot))
                                })
                                .map_or(Mode::Read, |p| p.mode)
                        };
                        pending.push(Node::Expr(&arg.value, mode));
                    }
                    pending.push(Node::Expr(target, requested));
                }
                Expr::If {
                    condition,
                    then_block,
                    otherwise,
                    ..
                } => {
                    match otherwise.as_deref() {
                        Some(Else::Block(other)) => pending.push(Node::Block(other)),
                        Some(Else::If(other)) => pending.push(Node::Expr(other, Mode::Read)),
                        None => {}
                    }
                    pending.push(Node::Block(then_block));
                    pending.push(Node::Expr(condition, Mode::Read));
                }
                Expr::Match(selection) => pending.push(Node::Match(selection)),
                Expr::Integer(..) | Expr::Bool(..) | Expr::Unit(..) => {}
            },
            Node::Block(block) => {
                pending.extend(block.statements.iter().rev().map(Node::Statement))
            }
            Node::Statement(statement) => match statement {
                Statement::Let { value, .. } | Statement::Expr { value, .. } => {
                    pending.push(Node::Expr(value, Mode::Read))
                }
                Statement::Assign { place, value, .. } => {
                    pending.push(Node::Expr(value, Mode::Read));
                    pending.push(Node::Expr(place, Mode::Inout));
                }
                Statement::Return { value, .. }
                | Statement::GuardControl { value, .. }
                | Statement::Value { value, .. } => {
                    if let Some(value) = value {
                        pending.push(Node::Expr(value, Mode::Read));
                    }
                }
                Statement::If {
                    condition,
                    then_block,
                    otherwise,
                    ..
                } => {
                    if let Some(otherwise) = otherwise {
                        pending.push(Node::Statement(otherwise));
                    }
                    pending.push(Node::Block(then_block));
                    pending.push(Node::Expr(condition, Mode::Read));
                }
                Statement::While {
                    condition, body, ..
                } => {
                    pending.push(Node::Block(body));
                    pending.push(Node::Expr(condition, Mode::Read));
                }
                Statement::For { iterable, body, .. } => {
                    pending.push(Node::Block(body));
                    pending.push(Node::Expr(iterable, Mode::Read));
                }
                Statement::Match(selection) => pending.push(Node::Match(selection)),
                Statement::Block(inner) => pending.push(Node::Block(inner)),
            },
            Node::Match(selection) => {
                for arm in selection.arms.iter().rev() {
                    pending.push(Node::Block(&arm.body));
                    if let Some(guard) = &arm.guard {
                        pending.push(Node::Expr(guard, Mode::Read));
                    }
                }
                pending.push(Node::Expr(
                    &selection.scrutinee,
                    selection.mode.unwrap_or(Mode::Read),
                ));
            }
        }
    }
    found
}

fn closure_has_value_return(block: &Block) -> bool {
    enum Node<'a> {
        Block(&'a Block),
        Statement(&'a Statement),
    }
    let mut pending = vec![Node::Block(block)];
    while let Some(node) = pending.pop() {
        match node {
            Node::Block(block) => {
                pending.extend(block.statements.iter().rev().map(Node::Statement))
            }
            Node::Statement(statement) => match statement {
                Statement::Return { value: Some(_), .. } => return true,
                Statement::If {
                    then_block,
                    otherwise,
                    ..
                } => {
                    if let Some(otherwise) = otherwise {
                        pending.push(Node::Statement(otherwise));
                    }
                    pending.push(Node::Block(then_block));
                }
                Statement::While { body, .. } | Statement::Block(body) => {
                    pending.push(Node::Block(body));
                }
                Statement::Match(selection) => {
                    pending.extend(
                        selection
                            .arms
                            .iter()
                            .rev()
                            .map(|arm| Node::Block(&arm.body)),
                    );
                }
                _ => {}
            },
        }
    }
    false
}

#[allow(clippy::vec_box)] // Shared semantic diagnostics retain the boxed representation.
pub(crate) struct TypeCheckReport {
    pub(crate) inferred: InferredTypes,
    pub(crate) errors: Vec<Box<Diagnostic>>,
    #[cfg(test)]
    stats: Vec<FunctionCheckStats>,
}

/// Read-only checker traversal counts for one function.
#[cfg(test)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct FunctionCheckStats {
    pub function_ordinal: u32,
    pub passes: u32,
    pub planning_calls: u32,
}

#[cfg(test)]
impl TypeCheckReport {
    /// Returns the type-check traversal counters without changing diagnostics or CLI output.
    pub(crate) fn function_stats(&self) -> &[FunctionCheckStats] {
        &self.stats
    }
}

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
enum OccurrenceRole {
    Statement,
    Expression,
    Block,
    Pattern,
    Arm,
}

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
struct OccurrenceId {
    function_ordinal: u32,
    preorder: u32,
    role: OccurrenceRole,
    local_index: u32,
}

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
enum ConsumerClass {
    ArrayElement,
    IfContinuation,
}

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
struct ConsumerId {
    owner: OccurrenceId,
    class: ConsumerClass,
    slot: u32,
}

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
struct OccurrenceKey(OccurrenceRole, usize);

#[derive(Clone, Default)]
struct OccurrenceIndex {
    by_node: HashMap<OccurrenceKey, OccurrenceId>,
}

impl OccurrenceIndex {
    fn for_function(function_ordinal: u32, body: &Block) -> Self {
        enum Node<'a> {
            Statement(&'a Statement),
            Expression(&'a Expr),
            Block(&'a Block),
            Pattern(&'a Pattern),
            Arm(&'a parse::Arm),
        }
        fn push_match<'a>(selection: &'a parse::Match, pending: &mut Vec<Node<'a>>) {
            for arm in selection.arms.iter().rev() {
                pending.push(Node::Arm(arm));
            }
            pending.push(Node::Expression(&selection.scrutinee));
        }
        fn push_statement<'a>(statement: &'a Statement, pending: &mut Vec<Node<'a>>) {
            match statement {
                Statement::Let { value, .. } | Statement::Expr { value, .. } => {
                    pending.push(Node::Expression(value));
                }
                Statement::Assign { place, value, .. } => {
                    pending.push(Node::Expression(value));
                    pending.push(Node::Expression(place));
                }
                Statement::Return { value, .. }
                | Statement::GuardControl { value, .. }
                | Statement::Value { value, .. } => {
                    if let Some(value) = value {
                        pending.push(Node::Expression(value));
                    }
                }
                Statement::If {
                    condition,
                    then_block,
                    otherwise,
                    ..
                } => {
                    if let Some(otherwise) = otherwise {
                        pending.push(Node::Statement(otherwise));
                    }
                    pending.push(Node::Block(then_block));
                    pending.push(Node::Expression(condition));
                }
                Statement::While {
                    condition, body, ..
                } => {
                    pending.push(Node::Block(body));
                    pending.push(Node::Expression(condition));
                }
                Statement::For { iterable, body, .. } => {
                    pending.push(Node::Block(body));
                    pending.push(Node::Expression(iterable));
                }
                Statement::Match(selection) => push_match(selection, pending),
                Statement::Block(block) => pending.push(Node::Block(block)),
            }
        }
        let mut result = Self::default();
        let mut pending = vec![Node::Block(body)];
        let mut preorder = 0u32;
        while let Some(node) = pending.pop() {
            let (role, address) = match node {
                Node::Statement(value) => {
                    push_statement(value, &mut pending);
                    (OccurrenceRole::Statement, std::ptr::from_ref(value).addr())
                }
                Node::Expression(value) => {
                    match value {
                        Expr::Closure(closure) => pending.push(Node::Block(&closure.body)),
                        Expr::Array(items, _) => {
                            pending.extend(items.iter().rev().map(Node::Expression))
                        }
                        Expr::Group(inner, _)
                        | Expr::Unary { value: inner, .. }
                        | Expr::Neg { value: inner, .. }
                        | Expr::Not { value: inner, .. } => pending.push(Node::Expression(inner)),
                        Expr::Binary { left, right, .. }
                        | Expr::Index {
                            target: left,
                            index: right,
                            ..
                        } => {
                            pending.push(Node::Expression(right));
                            pending.push(Node::Expression(left));
                        }
                        Expr::Call { target, args, .. } => {
                            for arg in args.iter().rev() {
                                pending.push(Node::Expression(&arg.value));
                            }
                            pending.push(Node::Expression(target));
                        }
                        Expr::If {
                            condition,
                            then_block,
                            otherwise,
                            ..
                        } => {
                            if let Some(otherwise) = otherwise {
                                match otherwise.as_ref() {
                                    Else::Block(block) => pending.push(Node::Block(block)),
                                    Else::If(expr) => pending.push(Node::Expression(expr)),
                                }
                            }
                            pending.push(Node::Block(then_block));
                            pending.push(Node::Expression(condition));
                        }
                        Expr::Match(selection) => push_match(selection, &mut pending),
                        Expr::Name(..) | Expr::Integer(..) | Expr::Bool(..) | Expr::Unit(..) => {}
                    }
                    let address = std::ptr::from_ref(value).addr();
                    (OccurrenceRole::Expression, address)
                }
                Node::Block(value) => {
                    pending.extend(value.statements.iter().rev().map(Node::Statement));
                    (OccurrenceRole::Block, std::ptr::from_ref(value).addr())
                }
                Node::Pattern(value) => {
                    if let Pattern::Constructor { args, .. } = value {
                        pending.extend(args.iter().rev().map(Node::Pattern));
                    }
                    (OccurrenceRole::Pattern, std::ptr::from_ref(value).addr())
                }
                Node::Arm(value) => {
                    if let Some(guard) = &value.guard {
                        pending.push(Node::Expression(guard));
                    }
                    pending.push(Node::Block(&value.body));
                    pending.push(Node::Pattern(&value.pattern));
                    (OccurrenceRole::Arm, std::ptr::from_ref(value).addr())
                }
            };
            let id = OccurrenceId {
                function_ordinal,
                preorder,
                role,
                local_index: 0,
            };
            preorder = preorder
                .checked_add(1)
                .expect("function occurrence index exhausted");
            result.by_node.insert(OccurrenceKey(role, address), id);
        }
        result
    }

    fn get(&self, role: OccurrenceRole, address: usize) -> Option<OccurrenceId> {
        self.by_node.get(&OccurrenceKey(role, address)).copied()
    }

    fn map_cloned_expression(
        &mut self,
        original: &Expr,
        cloned: &Expr,
    ) -> Vec<(OccurrenceKey, OccurrenceId)> {
        enum Pair<'a> {
            Expression(&'a Expr, &'a Expr),
            Statement(&'a Statement, &'a Statement),
            Block(&'a Block, &'a Block),
            Arm(&'a parse::Arm, &'a parse::Arm),
            Match(&'a parse::Match, &'a parse::Match),
        }
        let mut pending = vec![Pair::Expression(original, cloned)];
        let mut aliases = Vec::new();
        while let Some(pair) = pending.pop() {
            match pair {
                Pair::Expression(source, copy) => {
                    let source_id = self.get(
                        OccurrenceRole::Expression,
                        std::ptr::from_ref(source).addr(),
                    );
                    #[cfg(test)]
                    assert!(
                        source_id.is_some(),
                        "cloned selector source lacks prepassed ID"
                    );
                    if let Some(id) = source_id {
                        let key = OccurrenceKey(
                            OccurrenceRole::Expression,
                            std::ptr::from_ref(copy).addr(),
                        );
                        self.by_node.insert(key, id);
                        aliases.push((key, id));
                    }
                    match (source, copy) {
                        (Expr::Array(left, _), Expr::Array(right, _)) => {
                            pending.extend(
                                left.iter()
                                    .zip(right)
                                    .rev()
                                    .map(|(a, b)| Pair::Expression(a, b)),
                            );
                        }
                        (Expr::Group(a, _), Expr::Group(b, _))
                        | (Expr::Unary { value: a, .. }, Expr::Unary { value: b, .. })
                        | (Expr::Neg { value: a, .. }, Expr::Neg { value: b, .. })
                        | (Expr::Not { value: a, .. }, Expr::Not { value: b, .. }) => {
                            pending.push(Pair::Expression(a, b))
                        }
                        (
                            Expr::Binary {
                                left: al,
                                right: ar,
                                ..
                            },
                            Expr::Binary {
                                left: bl,
                                right: br,
                                ..
                            },
                        )
                        | (
                            Expr::Index {
                                target: al,
                                index: ar,
                                ..
                            },
                            Expr::Index {
                                target: bl,
                                index: br,
                                ..
                            },
                        ) => {
                            pending.push(Pair::Expression(ar, br));
                            pending.push(Pair::Expression(al, bl));
                        }
                        (
                            Expr::Call {
                                target: at,
                                args: aa,
                                ..
                            },
                            Expr::Call {
                                target: bt,
                                args: ba,
                                ..
                            },
                        ) => {
                            pending.extend(
                                aa.iter()
                                    .zip(ba)
                                    .rev()
                                    .map(|(a, b)| Pair::Expression(&a.value, &b.value)),
                            );
                            pending.push(Pair::Expression(at, bt));
                        }
                        (
                            Expr::If {
                                condition: ac,
                                then_block: ab,
                                otherwise: ao,
                                ..
                            },
                            Expr::If {
                                condition: bc,
                                then_block: bb,
                                otherwise: bo,
                                ..
                            },
                        ) => {
                            if let (Some(a), Some(b)) = (ao.as_deref(), bo.as_deref()) {
                                match (a, b) {
                                    (Else::Block(a), Else::Block(b)) => {
                                        pending.push(Pair::Block(a, b))
                                    }
                                    (Else::If(a), Else::If(b)) => {
                                        pending.push(Pair::Expression(a, b))
                                    }
                                    _ => unreachable!("cloned else shape changed"),
                                }
                            }
                            pending.push(Pair::Block(ab, bb));
                            pending.push(Pair::Expression(ac, bc));
                        }
                        (Expr::Match(a), Expr::Match(b)) => pending.push(Pair::Match(a, b)),
                        (Expr::Closure(a), Expr::Closure(b)) => {
                            pending.push(Pair::Block(&a.body, &b.body))
                        }
                        (Expr::Name(..), Expr::Name(..))
                        | (Expr::Integer(..), Expr::Integer(..))
                        | (Expr::Bool(..), Expr::Bool(..))
                        | (Expr::Unit(..), Expr::Unit(..)) => {}
                        _ => unreachable!("cloned expression shape changed"),
                    }
                }
                Pair::Statement(source, copy) => match (source, copy) {
                    (Statement::Let { value: a, .. }, Statement::Let { value: b, .. })
                    | (Statement::Expr { value: a, .. }, Statement::Expr { value: b, .. }) => {
                        pending.push(Pair::Expression(a, b))
                    }
                    (
                        Statement::Assign {
                            place: ap,
                            value: av,
                            ..
                        },
                        Statement::Assign {
                            place: bp,
                            value: bv,
                            ..
                        },
                    ) => {
                        pending.push(Pair::Expression(av, bv));
                        pending.push(Pair::Expression(ap, bp));
                    }
                    (Statement::Return { value: a, .. }, Statement::Return { value: b, .. })
                    | (
                        Statement::GuardControl { value: a, .. },
                        Statement::GuardControl { value: b, .. },
                    )
                    | (Statement::Value { value: a, .. }, Statement::Value { value: b, .. }) => {
                        if let (Some(a), Some(b)) = (a.as_ref(), b.as_ref()) {
                            pending.push(Pair::Expression(a, b));
                        }
                    }
                    (
                        Statement::If {
                            condition: ac,
                            then_block: ab,
                            otherwise: ao,
                            ..
                        },
                        Statement::If {
                            condition: bc,
                            then_block: bb,
                            otherwise: bo,
                            ..
                        },
                    ) => {
                        if let (Some(a), Some(b)) = (ao.as_deref(), bo.as_deref()) {
                            pending.push(Pair::Statement(a, b));
                        }
                        pending.push(Pair::Block(ab, bb));
                        pending.push(Pair::Expression(ac, bc));
                    }
                    (
                        Statement::While {
                            condition: ac,
                            body: ab,
                            ..
                        },
                        Statement::While {
                            condition: bc,
                            body: bb,
                            ..
                        },
                    ) => {
                        pending.push(Pair::Block(ab, bb));
                        pending.push(Pair::Expression(ac, bc));
                    }
                    (
                        Statement::For {
                            iterable: a,
                            body: ab,
                            ..
                        },
                        Statement::For {
                            iterable: b,
                            body: bb,
                            ..
                        },
                    ) => {
                        pending.push(Pair::Block(ab, bb));
                        pending.push(Pair::Expression(a, b));
                    }
                    (Statement::Match(a), Statement::Match(b)) => pending.push(Pair::Match(a, b)),
                    (Statement::Block(a), Statement::Block(b)) => pending.push(Pair::Block(a, b)),
                    _ => unreachable!("cloned statement shape changed"),
                },
                Pair::Block(source, copy) => pending.extend(
                    source
                        .statements
                        .iter()
                        .zip(&copy.statements)
                        .rev()
                        .map(|(a, b)| Pair::Statement(a, b)),
                ),
                Pair::Arm(source, copy) => {
                    if let (Some(a), Some(b)) = (source.guard.as_ref(), copy.guard.as_ref()) {
                        pending.push(Pair::Expression(a, b));
                    }
                    pending.push(Pair::Block(&source.body, &copy.body));
                }
                Pair::Match(source, copy) => {
                    for (a, b) in source.arms.iter().zip(&copy.arms).rev() {
                        pending.push(Pair::Arm(a, b));
                    }
                    pending.push(Pair::Expression(&source.scrutinee, &copy.scrutinee));
                }
            }
        }
        aliases
    }

    fn restore_aliases(&mut self, aliases: &[(OccurrenceKey, OccurrenceId)]) {
        self.by_node.extend(aliases.iter().copied());
    }
}

impl ConsumerId {
    fn new(owner: OccurrenceId, class: ConsumerClass, slot: u32) -> Self {
        Self { owner, class, slot }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum ResolvedType {
    Concrete(SyntaxType),
    Never,
    Unresolved,
}

fn record_unresolved_sites(ast: &Ast, inferred: &mut InferredTypes) {
    enum Node<'a> {
        Expression(&'a Expr),
        Statement(&'a Statement),
        Block(&'a Block),
        Match(&'a parse::Match),
    }
    let mut pending = Vec::new();
    for function in ast.functions.iter().rev() {
        pending.push(Node::Block(&function.body));
    }
    while let Some(node) = pending.pop() {
        match node {
            Node::Expression(expr) => {
                inferred
                    .expressions
                    .entry(span_key(expr.span()))
                    .or_insert(ResolvedType::Unresolved);
                match expr {
                    Expr::Array(items, _) => {
                        pending.extend(items.iter().rev().map(Node::Expression))
                    }
                    Expr::Group(inner, _)
                    | Expr::Unary { value: inner, .. }
                    | Expr::Neg { value: inner, .. }
                    | Expr::Not { value: inner, .. } => {
                        pending.push(Node::Expression(inner));
                    }
                    Expr::Binary { left, right, .. }
                    | Expr::Index {
                        target: left,
                        index: right,
                        ..
                    } => {
                        pending.push(Node::Expression(right));
                        pending.push(Node::Expression(left));
                    }
                    Expr::Call { target, args, .. } => {
                        pending.extend(args.iter().rev().map(|arg| Node::Expression(&arg.value)));
                        pending.push(Node::Expression(target));
                    }
                    Expr::If {
                        condition,
                        then_block,
                        otherwise,
                        ..
                    } => {
                        match otherwise.as_deref() {
                            Some(Else::Block(block)) => pending.push(Node::Block(block)),
                            Some(Else::If(expr)) => pending.push(Node::Expression(expr)),
                            None => {}
                        }
                        pending.push(Node::Block(then_block));
                        pending.push(Node::Expression(condition));
                    }
                    Expr::Match(selection) => pending.push(Node::Match(selection)),
                    Expr::Name(..)
                    | Expr::Integer(..)
                    | Expr::Bool(..)
                    | Expr::Unit(..)
                    | Expr::Closure(..) => {}
                }
            }
            Node::Statement(stmt) => match stmt {
                Statement::Let { value, .. } | Statement::Expr { value, .. } => {
                    pending.push(Node::Expression(value));
                }
                Statement::Assign { place, value, .. } => {
                    pending.push(Node::Expression(value));
                    pending.push(Node::Expression(place));
                }
                Statement::Return { value, .. }
                | Statement::GuardControl { value, .. }
                | Statement::Value { value, .. } => {
                    if let Some(value) = value {
                        pending.push(Node::Expression(value));
                    }
                }
                Statement::If {
                    condition,
                    then_block,
                    otherwise,
                    ..
                } => {
                    if let Some(otherwise) = otherwise {
                        pending.push(Node::Statement(otherwise));
                    }
                    pending.push(Node::Block(then_block));
                    pending.push(Node::Expression(condition));
                }
                Statement::While {
                    condition, body, ..
                } => {
                    pending.push(Node::Block(body));
                    pending.push(Node::Expression(condition));
                }
                Statement::For { iterable, body, .. } => {
                    pending.push(Node::Block(body));
                    pending.push(Node::Expression(iterable));
                }
                Statement::Match(selection) => {
                    inferred
                        .expressions
                        .entry(span_key(&selection.span))
                        .or_insert(ResolvedType::Unresolved);
                    pending.push(Node::Match(selection));
                }
                Statement::Block(block) => pending.push(Node::Block(block)),
            },
            Node::Block(block) => {
                pending.extend(block.statements.iter().rev().map(Node::Statement))
            }
            Node::Match(selection) => {
                for arm in selection.arms.iter().rev() {
                    pending.push(Node::Block(&arm.body));
                    if let Some(guard) = &arm.guard {
                        pending.push(Node::Expression(guard));
                    }
                }
                pending.push(Node::Expression(&selection.scrutinee));
            }
        }
    }
}

fn branch_value_span(block: &Block) -> Option<Span> {
    block.statements.iter().find_map(|statement| {
        if let Statement::Value {
            value: Some(value), ..
        } = statement
        {
            Some(Checker::primary_span(value))
        } else {
            None
        }
    })
}

fn branch_value_expr(block: &Block) -> Option<&Expr> {
    block.statements.iter().find_map(|statement| {
        if let Statement::Value {
            value: Some(value), ..
        } = statement
        {
            Some(value)
        } else {
            None
        }
    })
}

fn branch_mismatch(
    expected: &Type,
    found: &Type,
    expected_span: &Span,
    found_span: Span,
) -> Box<Diagnostic> {
    let (Some(expected_label), Some(found_label)) = (pretty(expected), pretty(found)) else {
        return unresolved_type(found_span);
    };
    let mut d = names::diagnostic(
        "NX-E1303",
        None,
        Stage::Types,
        Operation::TypesCheck,
        found_span.clone(),
    );
    d.data
        .insert("expected_type".into(), DataValue::String(expected_label));
    d.data
        .insert("found_type".into(), DataValue::String(found_label));
    d.related_spans.push(RelatedSpan {
        role: RelatedRole::ExpectedType,
        span: expected_span.clone(),
    });
    d.related_spans.push(RelatedSpan {
        role: RelatedRole::FoundType,
        span: found_span,
    });
    d
}

// The expression derivation reports how it used a concrete expected type.
// The caller performs exactly one final action, independent of AST shape.
#[derive(Clone, Copy)]
enum ContextUse {
    Compare,
    Propagated,
    ConstructorPair,
}

#[derive(Clone, Copy)]
struct CheckContext<'a> {
    expected: Option<&'a Type>,
    expected_span: Option<&'a Span>,
    check_here: bool,
    anchor: Option<(&'a Type, &'a Span)>,
    outer_compare: bool,
    related_expected: bool,
    deferred_element: bool,
}

#[derive(Clone)]
struct OwnedCheckContext {
    expected: Option<Type>,
    expected_span: Option<Span>,
    check_here: bool,
    anchor: Option<(Type, Span)>,
    outer_compare: bool,
    related_expected: bool,
    deferred_element: bool,
}

impl OwnedCheckContext {
    fn new(context: CheckContext<'_>) -> Self {
        Self {
            expected: context.expected.cloned(),
            expected_span: context.expected_span.cloned(),
            check_here: context.check_here,
            anchor: context.anchor.map(|(ty, span)| (ty.clone(), span.clone())),
            outer_compare: context.outer_compare,
            related_expected: context.related_expected,
            deferred_element: context.deferred_element,
        }
    }

    fn borrowed(&self) -> CheckContext<'_> {
        CheckContext {
            expected: self.expected.as_ref(),
            expected_span: self.expected_span.as_ref(),
            check_here: self.check_here,
            anchor: self.anchor.as_ref().map(|(ty, span)| (ty, span)),
            outer_compare: self.outer_compare,
            related_expected: self.related_expected,
            deferred_element: self.deferred_element,
        }
    }
}

enum DriveStart<'a> {
    Expression(&'a Expr, OwnedCheckContext),
    Block(&'a Block),
    #[cfg(test)]
    StatementSite(&'a Statement),
}

struct BranchProgress<'a> {
    block: &'a Block,
    context: OwnedCheckContext,
    next: usize,
    result: Option<Type>,
    value_failed: bool,
    anchor_consumed: bool,
}

enum BranchAction<'a> {
    Statement(&'a Statement),
    Value {
        expr: &'a Expr,
        context: OwnedCheckContext,
        parent_fact: FactId,
        nested_anchor: bool,
    },
    Done(Option<Type>, bool, bool),
}

struct MatchProgress<'a> {
    selection: &'a parse::Match,
    context: OwnedCheckContext,
    selected: Type,
    selected_origin: Span,
    joined: Option<(Type, Span)>,
    arm_index: usize,
    branch: Option<BranchProgress<'a>>,
    stage: MatchStage,
}

enum MatchStage {
    Arm,
    Guard,
    Branch,
    Value {
        parent_fact: FactId,
        nested_anchor: bool,
    },
}

enum MatchAction<'a> {
    Statement(&'a Statement),
    Value(&'a Expr, OwnedCheckContext),
    Done(Result<Type, Box<Diagnostic>>),
}

struct IfProgress<'a> {
    expr: &'a Expr,
    consumer: ConsumerId,
    next_child_index: u32,
    condition: &'a Expr,
    then_block: &'a Block,
    otherwise: Option<&'a Else>,
    context: OwnedCheckContext,
    branch_context: OwnedCheckContext,
    previous_if: Option<Span>,
    pending_before_if: usize,
    parent_fact: FactId,
    pending_before_condition: usize,
    stage: IfStage,
    branch: Option<BranchProgress<'a>>,
    first: Option<Type>,
    first_failed: bool,
    first_anchored: bool,
    first_partial: bool,
    first_span: Option<Span>,
    first_for_anchor: Option<Type>,
    anchor_error: Option<Box<Diagnostic>>,
    anchor_pending: bool,
    anchor: Option<(Type, Span)>,
}

enum IfStage {
    Condition,
    FirstBranch,
    FirstValue {
        parent_fact: FactId,
        nested_anchor: bool,
    },
    SecondBranch,
    SecondValue {
        parent_fact: FactId,
        nested_anchor: bool,
    },
    ElseIf,
}

enum IfAction<'a> {
    Statement(&'a Statement),
    Value(&'a Expr, OwnedCheckContext),
    Done(IfResult),
}

type IfResult = Result<(Option<Type>, bool, bool), Box<Diagnostic>>;

enum PrecomputedType {
    Whole(Result<Type, Box<Diagnostic>>),
    ArrayComplete {
        result: Result<Type, Box<Diagnostic>>,
        constructor_pair: bool,
    },
    IndexTarget {
        target: Result<Type, Box<Diagnostic>>,
        selector: Option<Result<Type, Box<Diagnostic>>>,
        deferred_child: Option<u64>,
    },
    UnaryOperand(Result<Type, Box<Diagnostic>>),
    BinaryLeft {
        result: Result<Type, Box<Diagnostic>>,
        pending_before: usize,
        independent_right: Option<bool>,
    },
    BinaryRight {
        left: Type,
        result: Result<Type, Box<Diagnostic>>,
        pending_before: usize,
    },
    BinaryDeferredRight {
        left: Type,
        result: Result<Type, Box<Diagnostic>>,
        pending_before_left: usize,
        pending_before_operator: usize,
    },
    CallComplete(Result<Type, Box<Diagnostic>>),
    CallTarget(Result<Type, Box<Diagnostic>>),
    MatchComplete(Result<Type, Box<Diagnostic>>),
    ClosureComplete(Result<Type, Box<Diagnostic>>),
    IfComplete {
        result: IfResult,
        pending_before: usize,
    },
}

struct IndexPrepared {
    target: Result<Type, Box<Diagnostic>>,
    selector: Option<Result<Type, Box<Diagnostic>>>,
    deferred_child: Option<u64>,
}
type BinaryLeftPrepared = (Result<Type, Box<Diagnostic>>, usize, Option<bool>);
type BinaryFrame<'a> = (&'a Expr, OwnedCheckContext, usize, FactId);
type GroupFrame<'a> = (&'a Expr, &'a Expr, &'a Span, OwnedCheckContext);

#[derive(Clone, Copy)]
struct UnifyPolicy {
    branch: bool,
    related_expected: bool,
}

// One source-site comparison is used both when unify knows the types now and
// when a pending constructor is diagnosed after its element becomes known.
// The caller decides when the pair is ready; the diagnostic policy is shared.
fn compare_source_site(
    left: &Type,
    right: &Type,
    left_origin: Span,
    found: Span,
    policy: UnifyPolicy,
) -> Option<Box<Diagnostic>> {
    if left == right {
        return None;
    }
    if policy.branch {
        return Some(branch_mismatch(left, right, &left_origin, found));
    }
    let Some(label) = pretty(left) else {
        return Some(unresolved_type(found));
    };
    let related = (policy.related_expected && left_origin != found).then_some(left_origin);
    Some(mismatch(found.clone(), found, related, &label, right))
}

enum BindingReadNode<'a> {
    Expr(&'a Expr),
    Block(&'a Block),
    Statement(&'a Statement),
    Match(&'a parse::Match),
}

#[derive(Clone, Copy)]
enum OccurrenceFlag {
    PendingDomain = 1,
    Deferred = 2,
    Divergent = 4,
    FailedJoin = 8,
    FailedProjection = 16,
    ContextRejected = 32,
    PartialNormal = 64,
    Tainted = 128,
}

#[derive(Clone, Default)]
struct OccurrenceState {
    flags: u8,
    conditional_result: Option<Type>,
    nominal_sources: HashSet<(u64, u64)>,
    obligations: [Option<ObligationStatus>; 9],
}

// A fact identifies one occurrence or solver obligation during an active
// visit. Its values live only in InferenceGraph; the ID never owns a snapshot.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
struct FactId(u64);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum FactRole {
    Expression,
    Statement,
    Operator,
    Selector,
    Solver(SolverBindSite),
}

#[derive(Clone, Copy)]
enum FactInheritance {
    Inherit,
    Empty,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct VisitFact {
    generation: u64,
    parent: Option<FactId>,
    site: (u64, u64),
    role: FactRole,
    tainted: bool,
    numeric_domain_owner: Option<(u64, u64)>,
}

#[cfg(test)]
#[derive(Clone, Copy, Debug)]
struct BoundaryEdgeTrace {
    site: (u64, u64),
    role: FactRole,
    parent_tainted: bool,
    child_tainted: bool,
    parent_owner: Option<(u64, u64)>,
    child_owner: Option<(u64, u64)>,
}

#[cfg(test)]
type ContextBoundaryEvent = (
    &'static str,
    &'static str,
    Option<(u64, u64)>,
    usize,
    bool,
    bool,
);
#[cfg(test)]
type FrameExitEvent = (&'static str, Option<(u64, u64)>, bool, usize, FactRole);

#[cfg(test)]
thread_local! {
    static FACT_REENTRY_ENABLED: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
    static FACT_REENTRY_STACK: std::cell::RefCell<Vec<(u64, HashMap<FactId, VisitFact>)>> = const { std::cell::RefCell::new(Vec::new()) };
    static FACT_REENTRY_COUNT: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
    static FACT_STATEMENT_TRACE_ENABLED: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
    static FACT_STATEMENT_TRACE: std::cell::RefCell<Vec<(bool, bool)>> = const { std::cell::RefCell::new(Vec::new()) };
    static CONTEXT_BOUNDARY_TRACE_ENABLED: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
    static CONTEXT_BOUNDARY_TRACE: std::cell::RefCell<Vec<ContextBoundaryEvent>> = const { std::cell::RefCell::new(Vec::new()) };
    static CONTEXT_FACT_DEPTH_TRACE: std::cell::RefCell<Vec<(&'static str, usize)>> = const { std::cell::RefCell::new(Vec::new()) };
    static FRAME_EXIT_TRACE: std::cell::RefCell<Vec<FrameExitEvent>> = const { std::cell::RefCell::new(Vec::new()) };
    static BOUNDARY_EDGE_TRACE: std::cell::RefCell<Vec<BoundaryEdgeTrace>> = const { std::cell::RefCell::new(Vec::new()) };
    static BOUNDARY_ROOT_OWNER: std::cell::Cell<Option<(u64, u64)>> = const { std::cell::Cell::new(None) };
    static BOUNDARY_ROOT_TAINT: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
    static BOUNDARY_FORCE_TAINT_EDGE: std::cell::Cell<Option<((u64, u64), FactRole)>> = const { std::cell::Cell::new(None) };
}

#[derive(Clone)]
struct InferenceGraph {
    equality_clock: u64,
    equality_records: Vec<EqualityRecord>,
    active_equality: Option<EqualityId>,
    node_journal: Option<NodeJournal>,
    active_facts: HashMap<FactId, VisitFact>,
    visit_generations: Vec<u64>,
    // Clones used by AtomicEnd share the clocks: rollback must not reuse an
    // ID allocated in the transaction that was discarded.
    fact_clock: std::rc::Rc<std::cell::Cell<u64>>,
    generation_clock: std::rc::Rc<std::cell::Cell<u64>>,
    locals: HashMap<u64, Known>,
    type_nodes: HashMap<u64, TypeNode>,
    // Entries own both immutable operands, so their pointer keys cannot be
    // reused. Only fully concrete pairs enter this cache.
    array_unify_cache: HashMap<(usize, usize), (Type, Type, Type, u64)>,
    expr_vars: HashMap<(u64, u64), u64>,
    expression_types: HashMap<(u64, u64), Type>,
    nominal_depth_memo: std::cell::RefCell<HashMap<(u8, u64), (usize, usize)>>,
    next_variable: u64,
    // Root, scalar constraint, expected source, incompatible operand,
    // array-on-left, and branch join (NX-E1303 instead of NX-E0102).
    obligations: Vec<ObligationKind>,
    occurrences: HashMap<(u64, u64), OccurrenceState>,
    // §13.9: an operand/constraint site owns one diagnostic, regardless of
    // whether it is discovered immediately or by a deferred join.
    emitted_sites: HashSet<(u64, u64)>,
    eligible_shapes: HashSet<u64>,
    shape_plan: HashMap<u64, Vec<Span>>,
    concrete_plan: HashMap<u64, ConcreteEvidence>,
    read_plan: HashMap<u64, ReadPlan>,
    assignment_type_plan: HashMap<u64, (Type, Span)>,
    nested_assignment_sites: HashSet<u64>,
    pre_assignment_snapshot: HashMap<u64, Option<ConcreteEvidence>>,
    projected_bases: HashSet<u64>,
    projection_items: HashSet<u64>,
    assigned_bases: HashSet<u64>,
    assignment_starts: HashMap<u64, u64>,
    first_assignment_rhs_starts: HashMap<u64, u64>,
    prefix_concrete: HashMap<u64, ConcreteEvidence>,
    preseeded_children: HashSet<u64>,
    result: Type,
    result_span: Option<Span>,
}

impl InferenceGraph {
    fn begin_equality(&mut self, left: Type, right: Type, found: Span) -> EqualityId {
        self.equality_clock += 1;
        let id = EqualityId(self.equality_clock);
        self.equality_records.push(EqualityRecord {
            id,
            operands: (left, right),
            found,
            join_start: self.obligations.len(),
            state: EqualityState::Attempting,
        });
        self.active_equality = Some(id);
        self.node_journal = Some(NodeJournal::default());
        id
    }

    fn touch_type_node(&mut self, id: u64) {
        if let Some(journal) = &mut self.node_journal
            && !journal.before.contains_key(&id)
        {
            journal.before.insert(id, self.type_nodes.get(&id).cloned());
            journal.order.push(id);
        }
    }

    fn type_node_mut(&mut self, id: &u64) -> Option<&mut TypeNode> {
        self.touch_type_node(*id);
        self.type_nodes.get_mut(id)
    }

    fn insert_type_node(&mut self, id: u64, node: TypeNode) -> Option<TypeNode> {
        self.touch_type_node(id);
        self.type_nodes.insert(id, node)
    }

    fn finish_equality(&mut self, id: EqualityId, publish: bool) {
        debug_assert_eq!(self.active_equality, Some(id));
        let record = self.equality_records.last_mut().expect("equality record");
        debug_assert_eq!(record.id, id);
        let _ = (&record.operands, &record.found);
        let waiting = publish
            && self.obligations[record.join_start..]
                .iter()
                .any(|obligation| matches!(obligation, ObligationKind::Join(_)));
        record.state = if publish {
            if waiting {
                EqualityState::Waiting
            } else {
                EqualityState::Committed
            }
        } else {
            EqualityState::Rejected
        };
        let journal = self.node_journal.take().expect("equality journal");
        if !publish {
            for key in journal.order.into_iter().rev() {
                match journal.before[&key].clone() {
                    Some(node) => {
                        self.type_nodes.insert(key, node);
                    }
                    None => {
                        self.type_nodes.remove(&key);
                    }
                }
            }
            self.obligations.truncate(record.join_start);
        }
        self.active_equality = None;
    }
    fn next(clock: &std::cell::Cell<u64>) -> u64 {
        let id = clock.get().checked_add(1).expect("fact clock exhausted");
        clock.set(id);
        id
    }

    fn begin_visit(&mut self, site: (u64, u64), role: FactRole) -> FactId {
        #[cfg(test)]
        FACT_REENTRY_ENABLED.with(|enabled| {
            if enabled.get() && !self.visit_generations.is_empty() {
                let prior = self.active_facts.clone();
                let suspended = *self.visit_generations.last().unwrap();
                FACT_REENTRY_STACK.with(|stack| stack.borrow_mut().push((suspended, prior)));
                FACT_REENTRY_COUNT.with(|count| count.set(count.get() + 1));
            }
        });
        let generation = Self::next(&self.generation_clock);
        self.visit_generations.push(generation);
        let id = FactId(Self::next(&self.fact_clock));
        self.active_facts.insert(
            id,
            VisitFact {
                generation,
                parent: None,
                site,
                role,
                tainted: false,
                numeric_domain_owner: None,
            },
        );
        #[cfg(test)]
        {
            let fact = self.active_facts.get_mut(&id).expect("new root fact");
            BOUNDARY_ROOT_OWNER.with(|owner| fact.numeric_domain_owner = owner.get());
            BOUNDARY_ROOT_TAINT.with(|taint| fact.tainted = taint.get());
        }
        id
    }

    fn fact(&self, id: FactId) -> &VisitFact {
        let fact = self.active_facts.get(&id).expect("closed fact ID");
        assert_eq!(
            self.visit_generations.last().copied(),
            Some(fact.generation),
            "fact from a suspended visit at {:?} ({:?})",
            fact.site,
            fact.role,
        );
        fact
    }

    fn fact_mut(&mut self, id: FactId) -> &mut VisitFact {
        let generation = self.visit_generations.last().copied();
        let fact = self.active_facts.get_mut(&id).expect("closed fact ID");
        assert_eq!(
            generation,
            Some(fact.generation),
            "fact from a suspended visit at {:?} ({:?})",
            fact.site,
            fact.role,
        );
        fact
    }

    fn child_fact(
        &mut self,
        parent: FactId,
        site: (u64, u64),
        role: FactRole,
        taint: FactInheritance,
        owner: FactInheritance,
    ) -> FactId {
        #[cfg(test)]
        BOUNDARY_FORCE_TAINT_EDGE.with(|forced| {
            if forced.get() == Some((site, role)) {
                self.fact_mut(parent).tainted = true;
            }
        });
        let parent_fact = self.fact(parent);
        let generation = parent_fact.generation;
        #[cfg(test)]
        let parent_tainted = parent_fact.tainted;
        #[cfg(test)]
        let parent_owner = parent_fact.numeric_domain_owner;
        let tainted = matches!(taint, FactInheritance::Inherit) && parent_fact.tainted;
        let numeric_domain_owner = match owner {
            FactInheritance::Inherit => parent_fact.numeric_domain_owner,
            FactInheritance::Empty => None,
        };
        let id = FactId(Self::next(&self.fact_clock));
        self.active_facts.insert(
            id,
            VisitFact {
                generation,
                parent: Some(parent),
                site,
                role,
                tainted,
                numeric_domain_owner,
            },
        );
        #[cfg(test)]
        CONTEXT_BOUNDARY_TRACE_ENABLED.with(|enabled| {
            if enabled.get() {
                BOUNDARY_EDGE_TRACE.with(|trace| {
                    trace.borrow_mut().push(BoundaryEdgeTrace {
                        site,
                        role,
                        parent_tainted,
                        child_tainted: tainted,
                        parent_owner,
                        child_owner: numeric_domain_owner,
                    });
                });
            }
        });
        #[cfg(test)]
        if role == FactRole::Statement {
            FACT_STATEMENT_TRACE_ENABLED.with(|enabled| {
                if enabled.get() {
                    FACT_STATEMENT_TRACE.with(|trace| {
                        trace
                            .borrow_mut()
                            .push((parent_tainted, self.fact(id).tainted));
                    });
                }
            });
        }
        id
    }

    fn merge_child(&mut self, parent: FactId, child: FactId, merge_taint: bool) {
        let child_fact = self.fact(child);
        assert_eq!(child_fact.parent, Some(parent), "unrelated fact IDs");
        let tainted = child_fact.tainted;
        if merge_taint {
            self.fact_mut(parent).tainted |= tainted;
        }
        self.active_facts.remove(&child);
    }

    fn merge_to_parent(&mut self, child: FactId, merge_taint: bool) -> FactId {
        let parent = self.fact(child).parent.expect("fact child expected");
        self.merge_child(parent, child, merge_taint);
        parent
    }

    fn retire_fact(&mut self, id: FactId) {
        self.fact(id);
        assert!(
            !self
                .active_facts
                .values()
                .any(|entry| entry.parent == Some(id)),
            "solver fact still has a live child"
        );
        self.active_facts.remove(&id);
    }

    fn end_visit(&mut self, root: FactId) -> bool {
        let fact = self.fact(root);
        assert!(fact.parent.is_none(), "visit root expected");
        let generation = fact.generation;
        let tainted = fact.tainted;
        self.active_facts
            .retain(|_, entry| entry.generation != generation);
        assert_eq!(self.visit_generations.pop(), Some(generation));
        #[cfg(test)]
        FACT_REENTRY_ENABLED.with(|enabled| {
            if enabled.get() && !self.visit_generations.is_empty() {
                let (suspended, prior) = FACT_REENTRY_STACK
                    .with(|stack| stack.borrow_mut().pop().expect("nested visit snapshot"));
                assert_eq!(self.visit_generations.last().copied(), Some(suspended));
                assert_eq!(
                    self.active_facts, prior,
                    "outer visit facts changed during reentry"
                );
            }
        });
        tainted
    }

    #[cfg(test)]
    fn active_count(&self) -> usize {
        self.active_facts.len()
    }

    #[cfg(test)]
    fn fact_depth(&self, mut id: FactId) -> usize {
        let mut depth = 0;
        while let Some(parent) = self.fact(id).parent {
            depth += 1;
            id = parent;
        }
        depth
    }

    fn register(&mut self, obligation: ObligationKind) {
        let (class, site) = obligation.occurrence();
        let state = self.occurrences.entry(site).or_default();
        state.obligations[class as usize].get_or_insert(ObligationStatus::Pending);
        self.obligations.push(obligation);
    }

    fn transition(&mut self, site: (u64, u64), class: ObligationClass, status: ObligationStatus) {
        let slot = &mut self.occurrences.entry(site).or_default().obligations[class as usize];
        let rank = |state| match state {
            ObligationStatus::Satisfied => 0,
            ObligationStatus::Pending => 1,
            ObligationStatus::Contaminated => 2,
            ObligationStatus::Refuted => 3,
        };
        if slot.is_none_or(|previous| rank(status) >= rank(previous)) {
            *slot = Some(status);
        }
    }

    fn refuted_result_sites(&self) -> HashSet<(u64, u64)> {
        self.occurrences
            .iter()
            .filter_map(|(&site, state)| {
                (matches!(
                    state.obligations[ObligationClass::DomainNominal as usize],
                    Some(ObligationStatus::Refuted | ObligationStatus::Contaminated)
                ) || matches!(
                    state.obligations[ObligationClass::BinaryDomain as usize],
                    Some(ObligationStatus::Refuted)
                ) || matches!(
                    state.obligations[ObligationClass::GuardedComparison as usize],
                    Some(ObligationStatus::Refuted)
                ))
                .then_some(site)
            })
            .collect()
    }

    fn attach_conditional_binding(&mut self, result: &Span, id: u64, nominal: Type) {
        let target = self.obligations.iter_mut().find_map(|entry| match entry {
            ObligationKind::DomainNominal(obligation) if obligation.call == *result => {
                Some(&mut obligation.conditional_bindings)
            }
            ObligationKind::BinaryDomain(obligation) if obligation.result == *result => {
                Some(&mut obligation.conditional_bindings)
            }
            ObligationKind::GuardedComparison(obligation) if obligation.result == *result => {
                Some(&mut obligation.conditional_bindings)
            }
            _ => None,
        });
        if let Some(target) = target {
            target.push((id, nominal));
        }
    }

    fn conditional_bindings(&self) -> Vec<(u64, Span, Type)> {
        self.obligations
            .iter()
            .flat_map(|entry| match entry {
                ObligationKind::DomainNominal(obligation) => obligation
                    .conditional_bindings
                    .iter()
                    .map(|(id, nominal)| (*id, obligation.call.clone(), nominal.clone()))
                    .collect::<Vec<_>>(),
                ObligationKind::BinaryDomain(obligation) => obligation
                    .conditional_bindings
                    .iter()
                    .map(|(id, nominal)| (*id, obligation.result.clone(), nominal.clone()))
                    .collect::<Vec<_>>(),
                ObligationKind::GuardedComparison(obligation) => obligation
                    .conditional_bindings
                    .iter()
                    .map(|(id, nominal)| (*id, obligation.result.clone(), nominal.clone()))
                    .collect::<Vec<_>>(),
                _ => Vec::new(),
            })
            .collect()
    }

    fn mark_failed_constructor(&mut self, id: u64) -> bool {
        let node = self.type_nodes.get_mut(&id).expect("known inference node");
        let was_new = !node.failed_constructor;
        node.failed_constructor = true;
        was_new
    }

    fn take_failed_constructor(&mut self, id: u64) -> bool {
        let node = self.type_nodes.get_mut(&id).expect("known inference node");
        std::mem::take(&mut node.failed_constructor)
    }

    fn failed_constructor(&self, id: u64) -> bool {
        self.type_nodes
            .get(&id)
            .is_some_and(|node| node.failed_constructor)
    }

    fn clear_failed_constructors(&mut self) {
        for node in self.type_nodes.values_mut() {
            node.failed_constructor = false;
        }
    }

    fn conditional_result(&self, site: &(u64, u64)) -> Option<&Type> {
        self.occurrences.get(site)?.conditional_result.as_ref()
    }

    fn set_conditional_result(&mut self, site: (u64, u64), ty: Type) {
        self.occurrences.entry(site).or_default().conditional_result = Some(ty);
    }

    fn mark(&mut self, site: (u64, u64), flag: OccurrenceFlag) -> bool {
        let state = self.occurrences.entry(site).or_default();
        let bit = flag as u8;
        let was_new = state.flags & bit == 0;
        state.flags |= bit;
        was_new
    }

    fn has(&self, site: &(u64, u64), flag: OccurrenceFlag) -> bool {
        self.occurrences
            .get(site)
            .is_some_and(|state| state.flags & flag as u8 != 0)
    }

    fn marked(&self, flag: OccurrenceFlag) -> impl Iterator<Item = (u64, u64)> + '_ {
        self.occurrences
            .iter()
            .filter_map(move |(&site, state)| (state.flags & flag as u8 != 0).then_some(site))
    }
}

#[allow(clippy::vec_box)] // Public semantic result keeps boxed diagnostics.
#[derive(Clone)]
struct Checker<'a> {
    file: &'a str,
    tokens: Vec<Token>,
    resolved: &'a Resolved,
    f5: bool,
    graph: InferenceGraph,
    closures: HashMap<u64, ClosureDraft>,
    saved_closures: HashMap<u64, ClosureDraft>,
    generator_slots: HashMap<u64, Vec<Type>>,
    hidden_for: Vec<Type>,
    closure_local_sites: HashSet<u64>,
    pending_depth: std::cell::RefCell<Vec<(Type, Span)>>,
    defer_depth: bool,
    depth_memo_ready: bool,
    solver_steps: u64,
    solver_limit: u64,
    // The first traversal discovers compatible constructor shapes without
    // spending the normative solver budget or emitting diagnostics. The
    // checking traversal owns both charges and diagnostic occurrences.
    planning: bool,
    occurrences: OccurrenceIndex,
    function_ordinal: u32,
    current_passes: u32,
    current_planning_calls: u32,
    block_depth: usize,
    top_statement_start: u64,
    top_statement_end: u64,
    active_if_span: Option<Span>,
    analysis_exhausted: bool,
    quota_error: Option<Box<Diagnostic>>,
    generator_item: Option<Type>,
    loop_depth: usize,
    errors: Vec<Box<Diagnostic>>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum SolverBindSite {
    Ordinary,
    RequireUnion,
    UnionArrayA,
    UnionArrayB,
    UnionPayloadA,
    UnionPayloadB,
    InheritedPayload,
}

struct SolverBind {
    fact: FactId,
    parent_fact: FactId,
    site: SolverBindSite,
    id: u64,
    ty: Type,
    origin: Span,
    found: Span,
    intrinsic: bool,
}

impl SolverBind {
    #[allow(clippy::too_many_arguments)]
    fn child(
        parent: FactId,
        site: SolverBindSite,
        id: u64,
        ty: Type,
        origin: Span,
        found: Span,
        intrinsic: bool,
    ) -> Self {
        Self {
            fact: parent,
            parent_fact: parent,
            site,
            id,
            ty,
            origin,
            found,
            intrinsic,
        }
    }
}

struct SolverUnify {
    fact: FactId,
    left: Type,
    right: Type,
    left_origin: Span,
    right_origin: Span,
    found: Span,
    policy: UnifyPolicy,
}

struct SolverUnion {
    left: u64,
    right: u64,
    found: Span,
    parent_fact: FactId,
}

struct SolverUnionJoin {
    a: u64,
    b: u64,
    found: Span,
    parent_fact: FactId,
    a_concrete: Option<(Type, Span)>,
    b_concrete: Option<(Type, Span)>,
    a_intrinsic: bool,
    b_intrinsic: bool,
    a_child: Option<u64>,
    b_child: Option<u64>,
    a_result: Option<UnionDescriptor<u64>>,
    b_result: Option<UnionDescriptor<u64>>,
    a_full: Option<(Type, Span)>,
    b_full: Option<(Type, Span)>,
    variant_constructed: bool,
    array_constructed: bool,
    bottom_only: bool,
    a_copyable: Option<Span>,
    b_copyable: Option<Span>,
}

enum SolverRequest {
    Bind(SolverBind),
    RequireUnion {
        id: u64,
        kind: UnionKind,
        origin: Span,
        parent_fact: FactId,
    },
    Union(SolverUnion),
    Unify(SolverUnify),
}

enum SolverValue {
    Unit,
    Union(UnionDescriptor<u64>),
    Type(Type),
}

#[cfg(test)]
#[derive(Clone, Copy)]
struct SolverBindTrace {
    site: SolverBindSite,
    parent: FactId,
    child: FactId,
    parent_generation: u64,
    child_generation: u64,
    child_parent: Option<FactId>,
    parent_tainted: bool,
    parent_owner: Option<(u64, u64)>,
    child_tainted: bool,
    child_owner: Option<(u64, u64)>,
}

#[cfg(test)]
thread_local! {
    static SOLVER_BIND_TRACE_ENABLED: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
    static SOLVER_BIND_TRACE: std::cell::RefCell<Vec<SolverBindTrace>> = const { std::cell::RefCell::new(Vec::new()) };
}

#[cfg(test)]
fn record_solver_bind_entry(
    graph: &InferenceGraph,
    site: SolverBindSite,
    parent: FactId,
    child: FactId,
) {
    SOLVER_BIND_TRACE_ENABLED.with(|enabled| {
        if enabled.get() {
            SOLVER_BIND_TRACE.with(|trace| {
                trace.borrow_mut().push(SolverBindTrace {
                    site,
                    parent,
                    child,
                    parent_generation: graph.fact(parent).generation,
                    child_generation: graph.fact(child).generation,
                    child_parent: graph.fact(child).parent,
                    parent_tainted: graph.fact(parent).tainted,
                    parent_owner: graph.fact(parent).numeric_domain_owner,
                    child_tainted: graph.fact(child).tainted,
                    child_owner: graph.fact(child).numeric_domain_owner,
                });
            });
        }
    });
}

impl Checker<'_> {
    fn closure_capture_sites(
        &self,
        closure: &parse::Closure,
    ) -> Vec<(String, CaptureMode, Span, Span, bool)> {
        if let Some(explicit) = &closure.captures {
            return explicit
                .iter()
                .filter_map(|capture| {
                    let ResolvedUse::Local(local) =
                        self.resolved.uses.get(&capture.name_span.start)?
                    else {
                        return None;
                    };
                    Some((
                        capture.name.clone(),
                        capture.mode,
                        local.name_span.clone(),
                        capture.span.clone(),
                        local.mutable,
                    ))
                })
                .collect();
        }
        let mut uses: Vec<_> = self
            .resolved
            .uses
            .iter()
            .filter_map(|(&at, resolved)| {
                let ResolvedUse::Local(local) = resolved else {
                    return None;
                };
                (at >= closure.body.span.start
                    && at < closure.body.span.end
                    && local.name_span.start < closure.fn_span.start)
                    .then_some((at, local.name_span.clone(), local.mutable))
            })
            .collect();
        uses.sort_by_key(|(at, _, _)| *at);
        let mut seen = HashSet::new();
        uses.into_iter()
            .filter_map(|(at, source, mutable)| {
                if !seen.insert(source.start) {
                    return None;
                }
                let token = self.tokens.iter().find(|token| token.span.start == at)?;
                Some((
                    token.text.clone(),
                    CaptureMode::View,
                    source,
                    token.span.clone(),
                    mutable,
                ))
            })
            .collect()
    }

    fn closure_begin<'b>(
        &mut self,
        closure: &'b parse::Closure,
        context: OwnedCheckContext,
    ) -> Result<ClosureProgress<'b>, Box<Diagnostic>> {
        let prior_sites: HashSet<u64> = self.graph.locals.keys().copied().collect();
        let sites = self.closure_capture_sites(closure);
        let mut captures = Vec::new();
        for (name, mode, source, capture_span, mutable) in sites {
            if let Some(known) = self.graph.locals.get(&source.start).cloned() {
                let ty = self
                    .resolved_syntax_type(&known.ty)
                    .map(|ty| Type::from(&ty))
                    .unwrap_or(known.ty);
                captures.push((
                    name,
                    mode,
                    capture_usage(closure, source.start, self.resolved),
                    source,
                    capture_span,
                    ty,
                    mutable,
                ));
            }
        }
        for (name, mode, _, source, capture_span, ty, _) in &captures {
            if *mode == CaptureMode::Copy && !copyable(ty) {
                let primary =
                    names::token_span(&self.tokens, capture_span.start, capture_span.end, name)
                        .unwrap_or_else(|| capture_span.clone());
                let mut error = names::diagnostic(
                    "NX-E0201",
                    None,
                    Stage::Ownership,
                    Operation::ValueCopy,
                    primary,
                );
                error.related_spans.push(RelatedSpan {
                    role: RelatedRole::Declaration,
                    span: source.clone(),
                });
                self.record_error(error);
            }
        }
        let copyable = captures.iter().all(|(_, mode, _, _, _, ty, _)| match mode {
            CaptureMode::View => true,
            CaptureMode::Edit => false,
            CaptureMode::Copy | CaptureMode::Move => copyable(ty),
        });
        let owns_resource = captures.iter().any(|(_, mode, _, _, _, ty, _)| {
            matches!(mode, CaptureMode::Move | CaptureMode::Copy) && contains_resource(ty)
        });
        let mut params = Vec::with_capacity(closure.params.len());
        for param in &closure.params {
            let name_span =
                names::token_span(&self.tokens, param.span.start, param.span.end, &param.name)
                    .unwrap_or_else(|| param.span.clone());
            let ty = if let Some(written) = &param.ty {
                Type::from(written)
            } else {
                Type::Deferred(self.new_variable(&name_span)?)
            };
            self.graph.locals.insert(
                name_span.start,
                Known {
                    ty: ty.clone(),
                    origin: name_span.clone(),
                    variable: None,
                    invalid: false,
                    nominal_sources: HashSet::new(),
                },
            );
            params.push((param.name.clone(), param.mode, name_span, ty));
        }
        let previous_result = self.graph.result.clone();
        let previous_result_span = self.graph.result_span.clone();
        let previous_generator_item = self.generator_item.take();
        let previous_loop_depth = std::mem::replace(&mut self.loop_depth, 0);
        let result = if let Some(written) = &closure.result {
            Type::from(written)
        } else if !closure_has_value_return(&closure.body) {
            Type::Unit
        } else {
            Type::Deferred(self.new_variable(&closure.fn_span)?)
        };
        self.graph.result = result.clone();
        self.graph.result_span = closure.result.as_ref().and_then(|_| {
            self.type_annotation_span(closure.fn_span.end, closure.body.span.start, "->")
        });
        Ok(ClosureProgress {
            closure,
            context,
            prior_sites,
            draft: ClosureDraft {
                captures,
                params,
                result,
            },
            previous_result,
            previous_result_span,
            previous_generator_item,
            previous_loop_depth,
            copyable,
            owns_resource,
            next: 0,
            pending_statement: None,
            pending_direct: false,
        })
    }

    fn closure_finish(
        &mut self,
        progress: ClosureProgress<'_>,
        checked: Result<(), Box<Diagnostic>>,
    ) -> Result<Type, Box<Diagnostic>> {
        self.graph.result = progress.previous_result;
        self.graph.result_span = progress.previous_result_span;
        self.generator_item = progress.previous_generator_item;
        self.loop_depth = progress.previous_loop_depth;
        checked?;
        self.closure_local_sites.extend(
            self.graph
                .locals
                .keys()
                .copied()
                .filter(|site| !progress.prior_sites.contains(site)),
        );
        let mut draft = progress.draft;
        draft.result = self
            .resolved_syntax_type(&draft.result)
            .map(|ty| Type::from(&ty))
            .unwrap_or(draft.result);
        self.closures.insert(progress.closure.fn_span.start, draft);
        Ok(Type::Closure {
            site: progress.closure.fn_span.start,
            copyable: progress.copyable,
            owns_resource: progress.owns_resource,
        })
    }

    fn closure_body_advance<'b>(
        &mut self,
        visit: &mut FactId,
        progress: &mut ClosureProgress<'b>,
        returned: Option<Result<Type, Box<Diagnostic>>>,
    ) -> ClosureAction<'b> {
        if progress.pending_direct {
            progress.pending_direct = false;
            if !self.analysis_exhausted
                && let Err(error) = self.resolve_pending_projections(false)
            {
                self.block_depth -= 1;
                return ClosureAction::Done(Err(error));
            }
        }
        if let Some(pending) = progress.pending_statement.take() {
            let returned = returned.expect("closure statement has a result");
            let (statement, parent_fact, outcome) = match pending {
                ClosurePending::Let(pending) => (
                    pending.statement,
                    pending.parent_fact,
                    self.finish_let(*visit, pending.statement, pending.prepared, returned),
                ),
                ClosurePending::Return {
                    statement,
                    expected,
                    parent_fact,
                } => {
                    let outcome = returned.and_then(|actual| {
                        if matches!(expected, Type::Deferred(_))
                            && !self.graph.fact_mut(*visit).tainted
                        {
                            let Statement::Return {
                                value: Some(value), ..
                            } = statement
                            else {
                                unreachable!("return continuation owns a value")
                            };
                            let site = Self::primary_span(value);
                            self.unify(
                                *visit,
                                expected,
                                actual,
                                site.clone(),
                                site.clone(),
                                site,
                                false,
                            )?;
                        }
                        Ok(())
                    });
                    (statement, parent_fact, outcome)
                }
            };
            let prior_taint = self.graph.fact(parent_fact).tainted;
            self.finish_statement_at_site(*visit, statement, prior_taint, outcome);
            self.graph.fact_mut(parent_fact).tainted = self.graph.fact(*visit).tainted;
            self.graph.merge_child(parent_fact, *visit, false);
            *visit = parent_fact;
            if !self.analysis_exhausted
                && let Err(error) = self.resolve_pending_projections(false)
            {
                self.block_depth -= 1;
                return ClosureAction::Done(Err(error));
            }
        }
        while let Some(statement) = progress.closure.body.statements.get(progress.next) {
            progress.next += 1;
            if self.block_depth == 1 {
                let span = parse::statement_span(statement);
                self.top_statement_start = span.start;
                self.top_statement_end = span.end;
            }
            if !self.analysis_exhausted
                && let Statement::Let {
                    annotation,
                    value,
                    span,
                    ..
                } = statement
            {
                match self.prepare_let(annotation.as_ref(), value, span) {
                    Ok(prepared) => {
                        let context = OwnedCheckContext::new(CheckContext {
                            expected: prepared.written.as_ref(),
                            expected_span: prepared.annotation_span.as_ref(),
                            check_here: true,
                            anchor: None,
                            outer_compare: false,
                            related_expected: prepared.annotation_span.is_some(),
                            deferred_element: false,
                        });
                        progress.pending_statement = Some(ClosurePending::Let(ClosureLet {
                            statement,
                            prepared,
                            parent_fact: *visit,
                        }));
                        return ClosureAction::Value(value, context);
                    }
                    Err(error) => {
                        let child = self.graph.child_fact(
                            *visit,
                            span_key(parse::statement_span(statement)),
                            FactRole::Statement,
                            FactInheritance::Empty,
                            FactInheritance::Inherit,
                        );
                        let prior_taint = self.graph.fact(*visit).tainted;
                        self.finish_statement_at_site(child, statement, prior_taint, Err(error));
                        self.graph.fact_mut(*visit).tainted = self.graph.fact(child).tainted;
                        self.graph.merge_child(*visit, child, false);
                    }
                }
            } else if !self.analysis_exhausted
                && self.generator_item.is_none()
                && let Statement::Return {
                    value: Some(value), ..
                } = statement
            {
                let expected = self.graph.result.clone();
                let origin = self.graph.result_span.clone();
                progress.pending_statement = Some(ClosurePending::Return {
                    statement,
                    expected: expected.clone(),
                    parent_fact: *visit,
                });
                return ClosureAction::Value(
                    value,
                    OwnedCheckContext::new(CheckContext {
                        expected: Some(&expected),
                        expected_span: origin.as_ref(),
                        check_here: true,
                        anchor: None,
                        outer_compare: false,
                        related_expected: origin.is_some(),
                        deferred_element: false,
                    }),
                );
            } else {
                progress.pending_direct = true;
                return ClosureAction::Statement(statement);
            }
            if !self.analysis_exhausted
                && let Err(error) = self.resolve_pending_projections(false)
            {
                self.block_depth -= 1;
                return ClosureAction::Done(Err(error));
            }
        }
        self.block_depth -= 1;
        ClosureAction::Done(Ok(()))
    }

    fn intrinsic_assignment_type(&self, expression: &Expr) -> Option<Type> {
        // Only context-free constructors are evidence for this assignment.
        enum Frame<'a> {
            Enter(&'a Expr),
            Array {
                items: &'a [Expr],
                next: usize,
                first: Option<Type>,
            },
        }
        let mut frames = vec![Frame::Enter(expression)];
        let mut result = None;
        while let Some(frame) = frames.pop() {
            match frame {
                Frame::Enter(expression) => match expression {
                    Expr::Integer(text, _) if text.parse::<i64>().is_ok() => {
                        result = Some(Type::I64)
                    }
                    Expr::Bool(..) => result = Some(Type::Bool),
                    Expr::Unit(..) => result = Some(Type::Unit),
                    Expr::Group(inner, _) => frames.push(Frame::Enter(inner)),
                    Expr::Array(items, _) => {
                        if let Some(first) = items.first() {
                            frames.push(Frame::Array {
                                items,
                                next: 0,
                                first: None,
                            });
                            frames.push(Frame::Enter(first));
                        } else {
                            result = None;
                        }
                    }
                    Expr::Name(_, span) => {
                        result = match self.resolved.uses.get(&span.start) {
                            Some(ResolvedUse::Local(local)) => self
                                .graph
                                .locals
                                .get(&local.name_span.start)
                                .filter(|known| {
                                    known.variable.is_none() && internal_state(&known.ty).is_none()
                                })
                                .map(|known| known.ty.clone()),
                            _ => None,
                        };
                    }
                    _ => result = None,
                },
                Frame::Array { items, next, first } => {
                    let Some(value) = result.take() else {
                        continue;
                    };
                    let first = if let Some(first) = first {
                        if value != first {
                            continue;
                        }
                        first
                    } else {
                        if !copyable(&value) || contains_resource(&value) {
                            continue;
                        }
                        value
                    };
                    if next + 1 == items.len() {
                        result = Some(Type::Array(Arc::new(first)));
                    } else {
                        frames.push(Frame::Array {
                            items,
                            next: next + 1,
                            first: Some(first),
                        });
                        frames.push(Frame::Enter(&items[next + 1]));
                    }
                }
            }
        }
        result
    }

    fn collect_planned_shapes(&mut self) -> bool {
        let mut changed = false;
        let eligible: Vec<_> = self.graph.eligible_shapes.iter().copied().collect();
        // Count unresolved assigned peers once per component. Scanning every
        // eligible binding from every other binding makes independent arrays
        // quadratic under the same fixed-point plan.
        let mut unresolved_peers: HashMap<u64, (usize, u64)> = HashMap::new();
        let mut first_assigned_type: HashMap<u64, (u64, Type)> = HashMap::new();
        for other in &eligible {
            if let (Some(id), Some((ty, _)), Some(start)) = (
                self.graph
                    .locals
                    .get(other)
                    .and_then(|known| known.variable),
                self.graph.assignment_type_plan.get(other).cloned(),
                self.graph.assignment_starts.get(other).copied(),
            ) {
                let root = self.root(id);
                let entry = first_assigned_type
                    .entry(root)
                    .or_insert_with(|| (start, ty.clone()));
                if start < entry.0 {
                    *entry = (start, ty);
                }
            }
            if self.graph.assigned_bases.contains(other)
                && !self.graph.assignment_type_plan.contains_key(other)
                && let Some(id) = self
                    .graph
                    .locals
                    .get(other)
                    .and_then(|known| known.variable)
            {
                let root = self.root(id);
                unresolved_peers
                    .entry(root)
                    .and_modify(|(count, _)| *count += 1)
                    .or_insert((1, *other));
            }
        }
        for site in eligible {
            let Some(known) = self.graph.locals.get(&site).cloned() else {
                continue;
            };
            let Some(mut current) = known.variable else {
                continue;
            };
            let mut origins = Vec::new();
            let mut chain = Vec::new();
            for _ in 0..=128 {
                current = self.root(current);
                chain.push(current);
                let node = &self.graph.type_nodes[&current];
                let Some(child) = node.array_child else {
                    break;
                };
                origins.push(
                    node.array_origin
                        .clone()
                        .unwrap_or_else(|| known.origin.clone()),
                );
                current = child;
            }
            self.graph.concrete_plan.remove(&site);
            self.graph.read_plan.remove(&site);
            let projected: Vec<_> = self.graph.projected_bases.iter().copied().collect();
            let has_conditional_projection = projected
                .into_iter()
                .any(|base| chain.contains(&self.root(base)));
            let has_deferred_join = chain
                .iter()
                .any(|root| self.graph.type_nodes[root].deferred_join);
            let has_recursive_join = chain
                .iter()
                .any(|root| self.graph.type_nodes[root].recursive_join);
            let nonbranch_join_roots: Vec<_> = self
                .pending_joins()
                .filter(|join| !join.branch)
                .map(|join| join.constructor)
                .collect();
            let has_recursive_nonbranch_join = has_recursive_join
                && nonbranch_join_roots
                    .into_iter()
                    .any(|root| chain.contains(&self.root(root)));
            if has_recursive_join && !self.graph.assigned_bases.contains(&site) {
                // Reject the recursive equality at its own occurrence. A
                // later constructor cannot change that earlier occurs check.
                changed |= self.graph.shape_plan.remove(&site).is_some();
                continue;
            }
            let resolved = self
                .graph
                .prefix_concrete
                .get(&site)
                .cloned()
                .or_else(|| self.concrete_evidence(known.variable.unwrap()));
            let read_type = self
                .graph
                .assignment_type_plan
                .get(&site)
                .map(|(ty, _)| ty.clone())
                .or_else(|| resolved.as_ref().map(|evidence| evidence.ty.clone()));
            let prefix = match self.graph.pre_assignment_snapshot.get(&site) {
                Some(snapshot) => snapshot.clone(),
                None => self.graph.prefix_concrete.get(&site).cloned(),
            };
            let conflicts_prefix = self
                .graph
                .assignment_type_plan
                .get(&site)
                .is_some_and(|(ty, _)| prefix.as_ref().is_some_and(|prior| &prior.ty != ty));
            if self.graph.assigned_bases.contains(&site)
                && !has_conditional_projection
                && has_deferred_join
                && !conflicts_prefix
                && let Some(ty) = read_type
                && let Some(&assignment_start) = self.graph.assignment_starts.get(&site)
            {
                let mut blocked: Vec<_> = chain
                    .iter()
                    .flat_map(|root| {
                        self.graph.type_nodes[root]
                            .branch_join_scopes
                            .iter()
                            .copied()
                    })
                    .collect();
                if !blocked.is_empty() {
                    blocked.sort_unstable();
                    blocked.dedup();
                    self.graph.read_plan.insert(
                        site,
                        ReadPlan {
                            ty,
                            blocked,
                            assignment_start,
                        },
                    );
                }
            }
            if self.graph.assigned_bases.contains(&site) {
                if let Some(source_type) = self.graph.assignment_type_plan.get(&site).cloned() {
                    let first_assignment = self.graph.first_assignment_rhs_starts.get(&site)
                        == Some(&source_type.1.start);
                    let mut source_depth = 0;
                    let mut layer = &source_type.0;
                    while let Type::Array(child) = layer {
                        source_depth += 1;
                        layer = child;
                    }
                    // An accepted structural equality can establish more
                    // Array layers than a later assignment supplies. Do not
                    // erase that prefix shape with the assignment (§4.1.2.2).
                    let conflicts_shape = source_depth < origins.len();
                    let conflicts_earlier_assignment = !has_deferred_join
                        && self
                            .graph
                            .assignment_starts
                            .get(&site)
                            .is_some_and(|start| {
                                chain
                                    .first()
                                    .and_then(|root| first_assigned_type.get(root))
                                    .is_some_and(|(first, prior)| {
                                        first < start && prior != &source_type.0
                                    })
                            });
                    let unresolved_peer = chain.iter().any(|root| {
                        unresolved_peers
                            .get(root)
                            .is_some_and(|(count, other)| *count > 1 || *other != site)
                    });
                    if conflicts_prefix
                        && let Some(prior) = prefix.clone()
                        && !has_recursive_join
                        && !has_deferred_join
                    {
                        // The earlier compatible prefix type is the one
                        // available to uses before this rejected assignment.
                        // Seed it, not the incompatible later RHS (§4.1.2.7).
                        self.graph.concrete_plan.insert(site, prior);
                    } else if first_assignment
                        && !has_recursive_nonbranch_join
                        && (!has_conditional_projection || !origins.is_empty())
                        && !conflicts_prefix
                        && !conflicts_shape
                        && !conflicts_earlier_assignment
                        && !unresolved_peer
                        && !self.graph.nested_assignment_sites.contains(&site)
                    {
                        // The whole-Array RHS may be the first root
                        // assignment while an earlier indexed write has
                        // already fixed its element. Seed the first
                        // compatible evidence, not this later RHS (§4.1.2.2,
                        // RD89). An intrinsic origin has no source relation.
                        let planned_source = resolved
                            .as_ref()
                            .filter(|evidence| {
                                evidence.ty == source_type.0
                                    && evidence.origin.start < source_type.1.start
                            })
                            .cloned()
                            .unwrap_or(ConcreteEvidence {
                                ty: source_type.0,
                                origin: source_type.1,
                                intrinsic: false,
                            });
                        origins.clear();
                        let mut layer = &planned_source.ty;
                        while let Type::Array(child) = layer {
                            origins.push(planned_source.origin.clone());
                            layer = child;
                        }
                        self.graph.concrete_plan.insert(site, planned_source);
                    }
                } else if !has_deferred_join && let Some(concrete) = resolved {
                    self.graph.concrete_plan.insert(site, concrete);
                }
            }
            changed |= self.graph.shape_plan.get(&site) != Some(&origins);
            self.graph.shape_plan.insert(site, origins);
        }
        changed
    }

    fn seed_planned_shape(&mut self, site: u64, id: u64) {
        let Some(origins) = self.graph.shape_plan.get(&site).cloned() else {
            return;
        };
        let mut current = id;
        for origin in origins {
            current = self.root(current);
            if let Some(child) = self.graph.type_nodes[&current].array_child {
                current = child;
                continue;
            }
            let child = self.graph.next_variable;
            self.graph.next_variable += 1;
            self.graph.insert_type_node(
                child,
                TypeNode {
                    parent: child,
                    size: 1,
                    concrete: None,
                    concrete_intrinsic: false,
                    array_child: None,
                    array_origin: None,
                    array_constructed: false,
                    variant_children: None,
                    variant_origin: None,
                    variant_constructed: false,
                    bottom_only: false,
                    failed_constructor: false,
                    deferred_join: false,
                    recursive_join: false,
                    branch_join_scopes: Vec::new(),
                    copyable_origin: Some(origin.clone()),
                },
            );
            self.graph.type_node_mut(&current).unwrap().array_child = Some(child);
            self.graph.type_node_mut(&current).unwrap().array_origin = Some(origin);
            self.graph.preseeded_children.insert(child);
            current = child;
        }
        if !self.planning
            && let Some(evidence) = self.graph.concrete_plan.get(&site).cloned()
        {
            let mut ty = evidence.ty;
            for _ in 0..=128 {
                let Type::Array(child) = ty else {
                    break;
                };
                ty = child.as_ref().clone();
            }
            current = self.root(current);
            if self.graph.type_nodes[&current].concrete.is_none() && internal_state(&ty).is_none() {
                let node = self.graph.type_node_mut(&current).unwrap();
                node.concrete = Some((ty, evidence.origin));
                node.concrete_intrinsic = evidence.intrinsic;
            }
        }
    }

    fn pending_constructors(&self) -> Vec<(u64, u64, Span)> {
        self.graph
            .obligations
            .iter()
            .filter_map(|entry| match entry {
                ObligationKind::Constructor {
                    child, array, span, ..
                } => Some((*child, *array, span.clone())),
                _ => None,
            })
            .collect()
    }

    fn pending_variant_constructors(&self) -> Vec<(u64, Span)> {
        self.graph
            .obligations
            .iter()
            .filter_map(|entry| match entry {
                ObligationKind::VariantConstructor { union, span } => Some((*union, span.clone())),
                _ => None,
            })
            .collect()
    }

    fn unresolved_union_paths(
        &mut self,
        id: u64,
        path: String,
        depth: usize,
        paths: &mut Vec<DataValue>,
    ) {
        let mut frames = vec![(id, path, depth)];
        while let Some((id, path, depth)) = frames.pop() {
            if depth > 128 || self.concrete(id).is_some() {
                continue;
            }
            let root = self.root(id);
            if let Some(child) = self.graph.type_nodes[&root].array_child {
                frames.push((child, format!("{path}.element"), depth + 1));
            } else if let Some(union) = self.graph.type_nodes[&root].variant_children {
                for variant in union.variants.iter().rev() {
                    if let Some(child) = variant.payload {
                        frames.push((
                            child,
                            format!("{path}.{}", variant.name.to_ascii_lowercase()),
                            depth + 1,
                        ));
                    }
                }
            } else {
                paths.push(DataValue::String(path));
            }
        }
    }
    fn pending_joins(&self) -> impl Iterator<Item = &PendingDeferredJoin> {
        self.graph
            .obligations
            .iter()
            .filter_map(|entry| match entry {
                ObligationKind::Join(join) => Some(join.as_ref()),
                _ => None,
            })
    }

    fn pending_joins_mut(&mut self) -> impl Iterator<Item = &mut PendingDeferredJoin> {
        self.graph
            .obligations
            .iter_mut()
            .filter_map(|entry| match entry {
                ObligationKind::Join(join) => Some(join.as_mut()),
                _ => None,
            })
    }

    fn pending_join_count(&self) -> usize {
        self.pending_joins().count()
    }

    fn truncate_pending_joins(&mut self, length: usize) {
        let mut seen = 0;
        self.graph.obligations.retain(|entry| match entry {
            ObligationKind::Join(_) => {
                seen += 1;
                seen <= length
            }
            _ => true,
        });
    }

    fn pending_domain_count(&self) -> usize {
        self.graph
            .obligations
            .iter()
            .filter(|entry| matches!(entry, ObligationKind::Domain(_)))
            .count()
    }

    fn pending_domains(&self) -> impl Iterator<Item = &PendingDomain> {
        self.graph
            .obligations
            .iter()
            .filter_map(|entry| match entry {
                ObligationKind::Domain(domain) => Some(domain),
                _ => None,
            })
    }

    fn take_pending_projections(&mut self) -> Vec<ObligationKind> {
        if !self
            .graph
            .obligations
            .iter()
            .any(|entry| matches!(entry, ObligationKind::Projection(_)))
        {
            return Vec::new();
        }
        self.take_pending_kind(|entry| match entry {
            entry @ ObligationKind::Projection(_) => Ok(entry),
            other => Err(other),
        })
    }

    fn take_pending_kind<T>(
        &mut self,
        mut select: impl FnMut(ObligationKind) -> Result<T, ObligationKind>,
    ) -> Vec<T> {
        let mut chosen = Vec::new();
        let mut retained = Vec::new();
        for entry in std::mem::take(&mut self.graph.obligations) {
            match select(entry) {
                Ok(value) => chosen.push(value),
                Err(entry) => retained.push(entry),
            }
        }
        self.graph.obligations = retained;
        chosen
    }

    fn representative(&self, id: u64) -> u64 {
        let mut current = id;
        while self.graph.type_nodes[&current].parent != current {
            current = self.graph.type_nodes[&current].parent;
        }
        current
    }

    fn projection_expected(&mut self, base: u64) -> Option<(Type, Span)> {
        let mut current = self.root(base);
        let mut depth = 0;
        loop {
            let next = self
                .graph
                .obligations
                .iter()
                .find_map(|entry| match entry {
                    ObligationKind::Projection(projection)
                        if self.representative(projection.base) == current =>
                    {
                        Some(projection.item)
                    }
                    _ => None,
                })?;
            depth += 1;
            if depth > 128 {
                return None;
            }
            if let Some((mut ty, origin)) = self.concrete(next) {
                for _ in 0..depth {
                    ty = Type::Array(Arc::new(ty));
                }
                return Some((ty, origin));
            }
            current = self.root(next);
        }
    }

    fn expression_has_array_root(&mut self, value: &Expr) -> bool {
        let mut value = value;
        while let Expr::Group(inner, _) | Expr::Unary { value: inner, .. } = value {
            value = inner;
        }
        match value {
            Expr::Array(..) => true,
            Expr::Name(_, span) => match self.resolved.uses.get(&span.start) {
                Some(ResolvedUse::Local(local)) => {
                    let known = self.graph.locals.get(&local.name_span.start).cloned();
                    known.is_some_and(|known| {
                        let ty = known
                            .variable
                            .and_then(|id| self.concrete(id).map(|(ty, _)| ty))
                            .unwrap_or(known.ty);
                        matches!(ty, Type::Array(_))
                    })
                }
                _ => false,
            },
            Expr::Call { span, .. } => self
                .resolved
                .calls
                .get(&(span.start, span.end))
                .and_then(|binding| names::signature(self.resolved, &binding.target))
                .is_some_and(|signature| matches!(Type::from(&signature.result), Type::Array(_))),
            _ => false,
        }
    }

    fn type_shape_hint(ty: &Type) -> ShapeHint {
        let mut current = ty;
        let mut depth = 0;
        while let Type::Array(child) = current {
            depth += 1;
            current = child;
        }
        let leaf = match current {
            Type::Deferred(_) | Type::Never | Type::Tainted => ShapeHint::Unknown,
            scalar => ShapeHint::Scalar(scalar.clone()),
        };
        (0..depth).fold(leaf, |shape, _| ShapeHint::Array(Box::new(shape)))
    }

    fn variable_shape_hint(&mut self, id: u64) -> ShapeHint {
        let mut current = self.root(id);
        let mut depth = 0;
        let leaf = loop {
            let node = &self.graph.type_nodes[&current];
            if let Some((ty, _)) = &node.concrete {
                break Self::type_shape_hint(ty);
            }
            let Some(child) = node.array_child else {
                break ShapeHint::Unknown;
            };
            depth += 1;
            if depth > 128 {
                break ShapeHint::Unknown;
            }
            current = self.root(child);
        };
        (0..depth).fold(leaf, |shape, _| ShapeHint::Array(Box::new(shape)))
    }

    fn expression_shape_hint(&mut self, value: &Expr) -> ShapeHint {
        enum Frame<'a> {
            Enter(&'a Expr),
            Array {
                items: &'a [Expr],
                next: usize,
                first: Option<ShapeHint>,
            },
            Index,
        }
        let mut frames = vec![Frame::Enter(value)];
        let mut result = ShapeHint::Unknown;
        while let Some(frame) = frames.pop() {
            match frame {
                Frame::Enter(value) => match value {
                    Expr::Array(items, _) => {
                        if items.is_empty() {
                            result = ShapeHint::Array(Box::new(ShapeHint::Unknown));
                        } else {
                            frames.push(Frame::Array {
                                items,
                                next: 0,
                                first: None,
                            });
                            frames.push(Frame::Enter(&items[0]));
                        }
                    }
                    Expr::Group(inner, _) => frames.push(Frame::Enter(inner)),
                    Expr::Name(_, span) => {
                        result = match self.resolved.uses.get(&span.start) {
                            Some(ResolvedUse::Local(local)) => {
                                let known = self.graph.locals.get(&local.name_span.start).cloned();
                                match known {
                                    Some(Known {
                                        variable: Some(id), ..
                                    }) => self.variable_shape_hint(id),
                                    Some(known) => Self::type_shape_hint(&known.ty),
                                    None => ShapeHint::Unknown,
                                }
                            }
                            _ => ShapeHint::Unknown,
                        };
                    }
                    Expr::Integer(..) => result = ShapeHint::Scalar(Type::I64),
                    Expr::Bool(..) => result = ShapeHint::Scalar(Type::Bool),
                    Expr::Unit(..) => result = ShapeHint::Scalar(Type::Unit),
                    Expr::Index { target, .. } => {
                        frames.push(Frame::Index);
                        frames.push(Frame::Enter(target));
                    }
                    _ => result = ShapeHint::Unknown,
                },
                Frame::Array { items, next, first } => {
                    let first = if let Some(first) = first {
                        if result != first {
                            result = ShapeHint::Unknown;
                            continue;
                        }
                        first
                    } else {
                        std::mem::replace(&mut result, ShapeHint::Unknown)
                    };
                    if next + 1 == items.len() {
                        result = ShapeHint::Array(Box::new(first));
                    } else {
                        frames.push(Frame::Array {
                            items,
                            next: next + 1,
                            first: Some(first),
                        });
                        frames.push(Frame::Enter(&items[next + 1]));
                    }
                }
                Frame::Index => {
                    result = match result {
                        ShapeHint::Array(child) => *child,
                        _ => ShapeHint::Unknown,
                    };
                }
            }
        }
        result
    }

    fn shape_hints_conflict(left: &ShapeHint, right: &ShapeHint) -> bool {
        let (mut left, mut right) = (left, right);
        loop {
            match (left, right) {
                (ShapeHint::Array(a), ShapeHint::Array(b)) => {
                    left = a;
                    right = b;
                }
                (ShapeHint::Scalar(a), ShapeHint::Scalar(b)) => return a != b,
                (ShapeHint::Array(_), ShapeHint::Scalar(_))
                | (ShapeHint::Scalar(_), ShapeHint::Array(_)) => return true,
                _ => return false,
            }
        }
    }

    fn resolve_pending_projections(&mut self, finalize: bool) -> Result<(), Box<Diagnostic>> {
        // A projection is conditional until its base has an Array root.
        // Resolving one projection can determine another one's base, so
        // retain undecided projections until the graph reaches a fixed point.
        let mut pending = self.take_pending_projections();
        while !pending.is_empty() {
            let mut next = Vec::new();
            let mut advanced = false;
            for entry in pending {
                let ObligationKind::Projection(mut projection) = entry else {
                    unreachable!("projection partition")
                };
                let root = self.root(projection.base);
                let element = self.graph.type_nodes[&root].array_child;
                let base = self.concrete(projection.base);
                let indexable = element.is_some();
                if !finalize && !indexable && base.is_none() {
                    if !projection.checked_without_context
                        && (self.planning || Self::context_stable_selector(&projection.selector))
                    {
                        // The selector has independent constraints of its own.
                        // Synthesize them in source order even though its I64
                        // context depends on the still-unknown base shape.
                        self.occurrences
                            .restore_aliases(&projection.selector_occurrence_aliases);
                        let (selector, selector_tainted) =
                            self.expr_with_visit_result(&projection.selector, None, None);
                        projection.selector_invalid |= selector_tainted || selector.is_err();
                        if let Err(error) = selector {
                            if error.operation == Operation::AnalysisLimit {
                                return Err(error);
                            }
                            self.record_error(error);
                        }
                        projection.checked_without_context = true;
                        next.push(ObligationKind::Projection(projection));
                        advanced = true;
                        continue;
                    }
                    next.push(ObligationKind::Projection(projection));
                    continue;
                }
                if !indexable && base.is_none() && projection.checked_without_context {
                    next.push(ObligationKind::Projection(projection));
                    continue;
                }
                // The selector occurrence predates a later binding that may
                // have fixed its variable. Once Array activates the earlier
                // I64 requirement, the later incompatible binding is the
                // conflicting site (§4.1.2.2–3, RD92/RD114).
                let later_conflict = if indexable {
                    projection.selector_variable.and_then(|id| {
                        // An earlier `[]` already established Array for this
                        // selector. I64 conflicts at the selector occurrence;
                        // a later assignment only completes its element.
                        let selector_root = self.root(id);
                        if self.graph.type_nodes[&selector_root].array_child.is_some() {
                            return None;
                        }
                        self.concrete(id).and_then(|(ty, origin)| {
                            (ty != Type::I64 && origin.start > projection.selector.span().start)
                                .then_some((ty, origin))
                        })
                    })
                } else {
                    None
                };
                self.occurrences
                    .restore_aliases(&projection.selector_occurrence_aliases);
                let (selector, selector_tainted) = self.expr_with_visit_result(
                    &projection.selector,
                    (indexable && later_conflict.is_none()).then_some(&Type::I64),
                    None,
                );
                if let Some((ty, origin)) = &later_conflict {
                    self.record_error(mismatch(origin.clone(), origin.clone(), None, "I64", ty));
                }
                let selector_invalid = projection.selector_invalid
                    || later_conflict.is_some()
                    || selector_tainted
                    || selector.is_err();
                let result_site = span_key(&projection.result_span);
                if selector_invalid {
                    self.graph
                        .mark(result_site, OccurrenceFlag::FailedProjection);
                }
                if let Err(error) = selector {
                    if error.operation == Operation::AnalysisLimit {
                        return Err(error);
                    }
                    self.record_error(error);
                }
                if let Some(child) = element {
                    if !selector_invalid {
                        let result = self.unify_independent(
                            Type::Deferred(projection.item),
                            Type::Deferred(child),
                            projection.base_span.clone(),
                            projection.base_span.clone(),
                            projection.base_span.clone(),
                            false,
                        );
                        if let Err(error) = result {
                            if error.operation == Operation::AnalysisLimit {
                                return Err(error);
                            }
                            self.graph
                                .mark(result_site, OccurrenceFlag::FailedProjection);
                            self.record_error(error);
                        }
                        if let Some(expected) = projection.expected {
                            let found = projection.result_span;
                            let result = self.unify_with_expected_origin_independent(
                                expected,
                                Type::Deferred(projection.item),
                                projection.expected_span.unwrap_or_else(|| found.clone()),
                                found.clone(),
                                found,
                                UnifyPolicy {
                                    branch: false,
                                    related_expected: projection.related_expected,
                                },
                            );
                            if let Err(error) = result {
                                if error.operation == Operation::AnalysisLimit {
                                    return Err(error);
                                }
                                self.graph
                                    .mark(result_site, OccurrenceFlag::FailedProjection);
                                self.record_error(error);
                            }
                        }
                    }
                } else if let Some((ty, _)) = base {
                    self.graph
                        .mark(result_site, OccurrenceFlag::FailedProjection);
                    self.record_error(self.domain_diagnostic(
                        projection.base_span,
                        &ty,
                        "domain:array",
                    ));
                } else {
                    projection.checked_without_context = true;
                    projection.selector_invalid = selector_invalid;
                    next.push(ObligationKind::Projection(projection));
                }
                advanced = true;
            }
            next.append(&mut self.take_pending_projections());
            if !advanced {
                self.graph.obligations.extend(next);
                break;
            }
            pending = next;
        }
        Ok(())
    }

    fn check_intrinsic_domain(
        &mut self,
        operand: &Expr,
        got: &Type,
        domain: &'static str,
    ) -> Result<(), Box<Diagnostic>> {
        let span = Self::primary_span(operand);
        let valid = domain_accepts(domain, got);
        if valid || matches!(got, Type::Never | Type::Tainted) {
            return Ok(());
        }
        let array_constructor = self.expression_variable(operand).is_some_and(|id| {
            let root = self.root(id);
            self.graph.type_nodes[&root].array_child.is_some()
        });
        if domain == "domain:array" && array_constructor {
            // `[]` has already fixed the Array constructor. Its element may
            // be unresolved, but that cannot invalidate Len's array domain.
            return Ok(());
        }
        if internal_state(got).is_some() {
            if let Some(id) = self.expression_variable(operand) {
                self.graph.register(ObligationKind::Domain(PendingDomain {
                    variable: id,
                    operand: span,
                    domain,
                }));
            }
            return Ok(());
        }
        Err(self.domain_diagnostic(span, got, domain))
    }

    fn domain_diagnostic(&self, span: Span, got: &Type, domain: &str) -> Box<Diagnostic> {
        match domain {
            "domain:array" => domain_error("not_indexable", span.clone(), span, None, domain, got),
            "domain:mock_resource" => {
                domain_error("resource_required", span.clone(), span, None, domain, got)
            }
            "domain:copyable_data" => mismatch(span.clone(), span, None, domain, got),
            _ => unreachable!("closed intrinsic domains"),
        }
    }

    fn remember_pending_domain_result(
        &mut self,
        before: usize,
        call: &Span,
        nominal: &Type,
        expected: Option<&Type>,
        expected_span: Option<&Span>,
    ) {
        let found = self
            .pending_domains()
            .nth(before)
            .map(|domain| (domain.variable, domain.domain));
        if let Some((variable, domain)) = found {
            self.defer_nominal_domain(
                variable,
                domain,
                call,
                nominal,
                expected,
                expected_span,
                None,
            );
        }
    }

    #[allow(clippy::too_many_arguments)] // One pending constraint carries its causal sites and guard.
    fn defer_nominal_domain(
        &mut self,
        variable: u64,
        domain: &'static str,
        call: &Span,
        nominal: &Type,
        expected: Option<&Type>,
        expected_span: Option<&Span>,
        depends_on: Option<&Span>,
    ) {
        // ADR §4.1.2.5: a nominal result cannot satisfy a consumer
        // until the argument or operand domain is known to hold.
        self.graph
            .mark(span_key(call), OccurrenceFlag::PendingDomain);
        // An established Array constructor can never satisfy the scalar
        // I64 domain, even while its element is unresolved. Delay the
        // serializable diagnostic, but stop offering the nominal result as
        // a witness to siblings now (§§4.1.2.2, 4.1.2.4–5).
        let root = self.root(variable);
        if domain == "I64" && self.graph.type_nodes[&root].array_child.is_some() {
            self.graph.mark(span_key(call), OccurrenceFlag::FailedJoin);
        }
        self.graph.register(ObligationKind::DomainNominal(Box::new(
            PendingNominalDomain {
                variable,
                domain,
                call: call.clone(),
                nominal: nominal.clone(),
                expected: expected.cloned(),
                expected_span: expected_span.cloned(),
                depends_on: depends_on.cloned(),
                conditional_bindings: Vec::new(),
            },
        )));
    }

    fn pending_operation_guard(&self, operand: &Expr) -> Option<(u64, &'static str)> {
        let site = span_key(&Self::primary_span(operand));
        self.graph
            .obligations
            .iter()
            .find_map(|pending| match pending {
                ObligationKind::DomainNominal(result) if span_key(&result.call) == site => {
                    Some((result.variable, result.domain))
                }
                ObligationKind::BinaryDomain(result) if span_key(&result.result) == site => {
                    Some((result.variable, result.domain))
                }
                ObligationKind::GuardedComparison(result) if span_key(&result.result) == site => {
                    Some((result.variable, result.domain))
                }
                _ => None,
            })
    }

    fn pending_operand_evidence(&self, operand: &Expr) -> Option<PendingOperandEvidence> {
        let span = Self::primary_span(operand);
        let inside = |site: &Span| span.start <= site.start && site.end <= span.end;
        let mut evidence = PendingOperandEvidence::default();
        // The occurrence span, rather than a mutable queue offset, owns its
        // obligations even when a nested block repartitions the work list.
        for pending in &self.graph.obligations {
            match pending {
                ObligationKind::Domain(domain) if inside(&domain.operand) => {
                    evidence.domains.push((domain.variable, domain.domain));
                }
                ObligationKind::DomainNominal(result) if inside(&result.call) => {
                    evidence.domains.push((result.variable, result.domain));
                    evidence.results.push(result.call.clone());
                }
                ObligationKind::BinaryDomain(result) if inside(&result.result) => {
                    evidence.domains.push((result.variable, result.domain));
                    evidence.results.push(result.result.clone());
                }
                ObligationKind::GuardedComparison(result) if inside(&result.result) => {
                    evidence.domains.push((result.variable, result.domain));
                    evidence
                        .domains
                        .extend(result.additional_domains.iter().copied());
                    evidence
                        .projections
                        .extend(result.projections.iter().copied());
                    evidence.results.push(result.result.clone());
                    evidence
                        .results
                        .extend(result.dependent_results.iter().cloned());
                }
                ObligationKind::Projection(result) if inside(&result.result_span) => {
                    evidence.projections.push((result.base, result.item));
                    evidence.results.push(result.result_span.clone());
                }
                _ => {}
            }
        }
        if evidence.domains.is_empty() && evidence.projections.is_empty() {
            return None;
        }
        evidence.domains.sort_unstable();
        evidence.domains.dedup();
        evidence.projections.sort_unstable();
        evidence.projections.dedup();
        evidence.results.sort_by_key(|site| (site.start, site.end));
        evidence.results.dedup();
        Some(evidence)
    }

    fn pending_domain_state(&mut self, variable: u64, domain: &str) -> Option<bool> {
        self.concrete(variable).map(|(ty, _)| {
            if domain == "I64" {
                ty == Type::I64
            } else {
                domain_accepts(domain, &ty)
            }
        })
    }

    fn pending_operation_type(&self, operand: &Expr) -> Option<Type> {
        let site = span_key(&Self::primary_span(operand));
        self.graph
            .expression_types
            .get(&site)
            .filter(|ty| **ty != Type::Tainted)
            .cloned()
            .or_else(|| {
                self.graph
                    .obligations
                    .iter()
                    .find_map(|pending| match pending {
                        ObligationKind::DomainNominal(result) if span_key(&result.call) == site => {
                            Some(result.nominal.clone())
                        }
                        ObligationKind::GuardedComparison(result)
                            if span_key(&result.result) == site =>
                        {
                            Some(Type::Bool)
                        }
                        _ => None,
                    })
            })
    }

    fn pending_nominal_available(&mut self, operand: &Expr) -> bool {
        let Some((variable, domain)) = self.pending_operation_guard(operand) else {
            return false;
        };
        let root = self.root(variable);
        if self.pending_domain_state(root, domain) == Some(false) {
            return false;
        }
        let node = &self.graph.type_nodes[&root];
        if domain == "I64" && (node.array_child.is_some() || node.variant_children.is_some()) {
            return false;
        }
        if node.failed_constructor {
            return false;
        }
        let child = node.array_child;
        if child.is_none_or(|child| self.concrete(child).is_some())
            || self.graph.type_nodes[&root].bottom_only
        {
            return true;
        }
        // A binding can receive constraints after this occurrence, including
        // through a value constructor that contains this operation's result.
        // A bare unresolved constructor instead owns its own NX-E0103 and
        // cannot provide such a witness to an enclosing expression.
        let bindings: Vec<_> = self
            .graph
            .locals
            .values()
            .filter_map(|known| known.variable)
            .collect();
        bindings.into_iter().any(|id| self.root(id) == root)
    }

    fn capture_expression_dependencies(&mut self, expression: &Expr) {
        let site = span_key(expression.span());
        let mut children: Vec<&Expr> = Vec::new();
        let mut sources = HashSet::new();
        match expression {
            Expr::Name(_, span) => {
                if let Some(ResolvedUse::Local(local)) = self.resolved.uses.get(&span.start)
                    && let Some(binding) = self.graph.locals.get(&local.name_span.start)
                {
                    sources.extend(binding.nominal_sources.iter().copied());
                }
            }
            Expr::Array(items, _) => children.extend(items),
            Expr::Group(inner, _)
            | Expr::Unary { value: inner, .. }
            | Expr::Neg { value: inner, .. }
            | Expr::Not { value: inner, .. } => children.push(inner),
            Expr::Binary { left, right, .. } => children.extend([left.as_ref(), right.as_ref()]),
            Expr::Call { target, args, .. } => {
                children.push(target);
                children.extend(args.iter().map(|arg| &arg.value));
            }
            Expr::Index { target, index, .. } => {
                children.extend([target.as_ref(), index.as_ref()]);
            }
            Expr::If {
                condition,
                then_block,
                otherwise,
                ..
            } => {
                children.push(condition);
                if let Some(value) = branch_value_expr(then_block) {
                    children.push(value);
                }
                match otherwise.as_deref() {
                    Some(Else::Block(block)) => {
                        if let Some(value) = branch_value_expr(block) {
                            children.push(value);
                        }
                    }
                    Some(Else::If(value)) => children.push(value),
                    None => {}
                }
            }
            Expr::Match(selection) => {
                children.push(&selection.scrutinee);
                for arm in &selection.arms {
                    if let Some(guard) = &arm.guard {
                        children.push(guard);
                    }
                    if let Some(value) = branch_value_expr(&arm.body) {
                        children.push(value);
                    }
                }
            }
            Expr::Closure(_) | Expr::Integer(..) | Expr::Bool(..) | Expr::Unit(..) => {}
        }
        for child in children {
            if let Some(state) = self.graph.occurrences.get(&span_key(child.span())) {
                sources.extend(state.nominal_sources.iter().copied());
            }
        }
        if self.pending_operation_guard(expression).is_some() {
            sources.insert(site);
        }
        self.graph
            .occurrences
            .entry(site)
            .or_default()
            .nominal_sources
            .extend(sources);
    }

    fn record_error(&mut self, error: Box<Diagnostic>) {
        if self.planning {
            return;
        }
        if error.code == "NX-E0003"
            && error.data.get("budget_kind") == Some(&DataValue::String("value_type_depth".into()))
            && let Some(previous) = self.errors.iter_mut().find(|previous| {
                previous.code == "NX-E0003"
                    && previous.data.get("budget_kind")
                        == Some(&DataValue::String("value_type_depth".into()))
            })
        {
            let site = |diagnostic: &Diagnostic| {
                diagnostic
                    .primary_span
                    .as_ref()
                    .map_or((u64::MAX, u64::MAX), |span| (span.start, span.end))
            };
            if site(&error) < site(previous) {
                *previous = error;
            }
            return;
        }
        if error.operation != Operation::AnalysisLimit
            && let Some(span) = &error.primary_span
        {
            // §4.7 excludes a second join diagnosis at an operand that
            // already failed its contextual check. §4.1.1 suppresses
            // derived errors, while other spans remain independent.
            if !self.graph.emitted_sites.insert((span.start, span.end)) {
                return;
            }
        }
        self.errors.push(error);
    }

    fn suppress_result_dependents(&mut self, results: &HashSet<(u64, u64)>) {
        // A nominal result with an unresolved or failed domain is not a
        // witness for its consumers (§4.1.2.5). The same site rule applies
        // at ordinary closure and at a denied solver charge (§11.1).
        let occurrences = &self.graph.occurrences;
        self.errors.retain(|error| {
            let result_site = error.primary_span.as_ref().map(span_key);
            let expected_witness = error
                .related_spans
                .iter()
                .find(|relation| relation.role == RelatedRole::ExpectedType)
                .map(|relation| span_key(&relation.span));
            !result_site.is_some_and(|site| results.contains(&site))
                && !expected_witness.is_some_and(|site| results.contains(&site))
                && !result_site.is_some_and(|site| {
                    occurrences.get(&site).is_some_and(|state| {
                        state
                            .nominal_sources
                            .iter()
                            .any(|source| results.contains(source))
                    })
                })
        });
    }

    fn charge_solver(&mut self, span: &Span) -> Result<(), Box<Diagnostic>> {
        if self.planning {
            return Ok(());
        }
        let limit = self.solver_limit;
        // The counter starts at zero, increments only below the limit, and
        // stops at the first denied charge (§11.1). Thus it never exceeds
        // limit; replacing == with >= cannot change a reachable result.
        if self.solver_steps == limit {
            self.analysis_exhausted = true;
            let mut d = names::diagnostic(
                "NX-E0003",
                None,
                Stage::Types,
                Operation::AnalysisLimit,
                span.clone(),
            );
            d.data.insert(
                "budget_kind".into(),
                DataValue::String("type_solver_steps".into()),
            );
            d.data.insert(
                "budget_scope".into(),
                DataValue::String("compilation".into()),
            );
            d.data.insert(
                "incomplete_analysis".into(),
                DataValue::String("types".into()),
            );
            d.data.insert("limit".into(), DataValue::UInt(limit));
            d.data.insert("observed".into(), DataValue::UInt(limit + 1));
            if self.quota_error.is_none() {
                self.quota_error = Some(d.clone());
            }
            return Err(d);
        }
        self.solver_steps += 1;
        #[cfg(test)]
        if matches!(
            self.file,
            "semantic_scale_counter.n" | "quota_exact_default.n" | "adr_gap_unresolved_cost.n"
        ) {
            TYPE_SOLVER_WORK.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        }
        Ok(())
    }

    fn new_variable(&mut self, span: &Span) -> Result<u64, Box<Diagnostic>> {
        self.charge_solver(span)?;
        let id = self.graph.next_variable;
        self.graph.next_variable += 1;
        self.graph.insert_type_node(
            id,
            TypeNode {
                parent: id,
                size: 1,
                concrete: None,
                concrete_intrinsic: false,
                array_child: None,
                array_origin: None,
                array_constructed: false,
                variant_children: None,
                variant_origin: None,
                variant_constructed: false,
                bottom_only: false,
                failed_constructor: false,
                deferred_join: false,
                recursive_join: false,
                branch_join_scopes: Vec::new(),
                copyable_origin: None,
            },
        );
        Ok(id)
    }

    fn require_array(&mut self, id: u64, origin: &Span) -> Result<u64, Box<Diagnostic>> {
        self.charge_solver(origin)?;
        let root = self.root(id);
        if let Some(child) = self.graph.type_nodes[&root].array_child {
            if self.graph.preseeded_children.remove(&child) {
                self.charge_solver(origin)?;
            }
            return Ok(child);
        }
        if self.graph.type_nodes[&root].variant_children.is_some()
            || self.graph.type_nodes[&root]
                .concrete
                .as_ref()
                .is_some_and(|(ty, _)| !matches!(ty, Type::Array(..)))
        {
            return Err(unresolved_type(origin.clone()));
        }
        let child = self.new_variable(origin)?;
        self.graph.type_node_mut(&root).unwrap().array_child = Some(child);
        self.graph.type_node_mut(&root).unwrap().array_origin = Some(origin.clone());
        // Every newly required element inherits the array's copyable-data
        // domain. Explicit array constructors call require_copyable on their
        // element instead (§4.1.1); removing only the stored origin cannot
        // matter once that child already has a copyability origin.
        self.graph
            .type_nodes
            .get_mut(&child)
            .unwrap()
            .copyable_origin = Some(origin.clone());
        self.check_one_inferred_depth(root, origin)?;
        Ok(child)
    }

    fn require_union(
        &mut self,
        id: u64,
        kind: UnionKind,
        origin: &Span,
    ) -> Result<UnionDescriptor<u64>, Box<Diagnostic>> {
        let result = self.with_independent_fact(span_key(origin), |checker, fact| {
            checker.solve(SolverRequest::RequireUnion {
                id,
                kind,
                origin: origin.clone(),
                parent_fact: fact,
            })
        })?;
        match result {
            SolverValue::Union(children) => Ok(children),
            _ => unreachable!("require_union result"),
        }
    }

    fn require_copyable(
        &mut self,
        id: u64,
        origin: &Span,
        data_only: bool,
    ) -> Result<(), Box<Diagnostic>> {
        let mut frames = vec![id];
        while let Some(id) = frames.pop() {
            self.charge_solver(origin)?;
            let root = self.root(id);
            if let Some((ty, _)) = self.concrete(root)
                && !(if data_only {
                    comparable_data(&ty)
                } else {
                    copyable(&ty)
                })
            {
                return Err(mismatch(
                    origin.clone(),
                    origin.clone(),
                    None,
                    "domain:copyable_data",
                    &ty,
                ));
            }
            if self.graph.type_nodes[&root].variant_children.is_some()
                && self.concrete(root).is_none()
            {
                self.graph.register(ObligationKind::Domain(PendingDomain {
                    variable: root,
                    operand: origin.clone(),
                    domain: "domain:copyable_data",
                }));
            }
            // Only presence of this domain is consulted by unification. A later
            // failure uses its current operand as primary, so choosing the first
            // recorded origin span cannot change a source-level diagnostic.
            let previous = self.graph.type_nodes[&root].copyable_origin.clone();
            if previous
                .as_ref()
                .is_none_or(|span| origin.start < span.start)
            {
                self.graph
                    .type_nodes
                    .get_mut(&root)
                    .unwrap()
                    .copyable_origin = Some(origin.clone());
            }
            // require_array and explicit constructors already gave this child
            // the copyable domain. Requiring it again preserves that invariant
            // for manually built solver states, with no new source-level result.
            if let Some(child) = self.graph.type_nodes[&root].array_child {
                frames.push(child);
            }
        }
        Ok(())
    }

    fn root(&mut self, id: u64) -> u64 {
        let mut root = id;
        while self.graph.type_nodes[&root].parent != root {
            root = self.graph.type_nodes[&root].parent;
        }
        let mut current = id;
        while current != root {
            let next = self.graph.type_nodes[&current].parent;
            self.graph.type_node_mut(&current).unwrap().parent = root;
            current = next;
        }
        root
    }

    fn concrete(&mut self, id: u64) -> Option<(Type, Span)> {
        self.concrete_with_depth(id, 0)
    }

    fn concrete_with_depth(&mut self, id: u64, depth: usize) -> Option<(Type, Span)> {
        enum Frame {
            Enter(u64, usize),
            Array,
            Union {
                kind: UnionKind,
                origin: Option<Span>,
                remaining: Vec<u64>,
                payloads: Vec<(Type, Span)>,
                depth: usize,
            },
        }
        let mut frames = vec![Frame::Enter(id, depth)];
        let mut result = None;
        while let Some(frame) = frames.pop() {
            match frame {
                Frame::Enter(id, depth) => {
                    if depth > 128 {
                        return None;
                    }
                    let root = self.root(id);
                    let node = self.graph.type_nodes[&root].clone();
                    if let Some(concrete) = node.concrete {
                        if node.array_child.is_some() && !matches!(concrete.0, Type::Array(..))
                            || node.variant_children.is_some_and(|children| {
                                concrete_union(&concrete.0)
                                    .is_none_or(|union| union.kind != children.kind)
                            })
                        {
                            return None;
                        }
                        result = Some(concrete);
                    } else if let Some(child) = node.array_child {
                        frames.push(Frame::Array);
                        frames.push(Frame::Enter(child, depth + 1));
                    } else {
                        let union = node.variant_children?;
                        let remaining: Vec<_> = union
                            .variants
                            .iter()
                            .filter_map(|variant| variant.payload)
                            .collect();
                        if let Some(&first) = remaining.first() {
                            frames.push(Frame::Union {
                                kind: union.kind,
                                origin: node.variant_origin,
                                remaining: remaining[1..].to_vec(),
                                payloads: Vec::new(),
                                depth,
                            });
                            frames.push(Frame::Enter(first, depth + 1));
                        } else {
                            return None;
                        }
                    }
                }
                Frame::Array => {
                    let (ty, origin) = result.take()?;
                    result = Some((Type::Array(Arc::new(ty)), origin));
                }
                Frame::Union {
                    kind,
                    origin,
                    mut remaining,
                    mut payloads,
                    depth,
                } => {
                    payloads.push(result.take()?);
                    if !remaining.is_empty() {
                        let next = remaining.remove(0);
                        frames.push(Frame::Union {
                            kind,
                            origin,
                            remaining,
                            payloads,
                            depth,
                        });
                        frames.push(Frame::Enter(next, depth + 1));
                    } else {
                        let origin = origin.or_else(|| payloads.first().map(|p| p.1.clone()))?;
                        result = match (kind, payloads.as_slice()) {
                            (UnionKind::Option, [(some, _)]) => {
                                Some((Type::option(Arc::new(some.clone())), origin))
                            }
                            (UnionKind::Result, [(ok, _), (err, _)]) => Some((
                                Type::result(Arc::new(ok.clone()), Arc::new(err.clone())),
                                origin,
                            )),
                            _ => None,
                        };
                    }
                }
            }
        }
        result
    }

    fn concrete_evidence(&mut self, id: u64) -> Option<ConcreteEvidence> {
        let (ty, origin) = self.concrete(id)?;
        Some(ConcreteEvidence {
            ty,
            origin,
            intrinsic: self.origin_intrinsic(id),
        })
    }

    fn origin_intrinsic(&mut self, id: u64) -> bool {
        let mut current = self.root(id);
        for _ in 0..=128 {
            let node = &self.graph.type_nodes[&current];
            if node.concrete.is_some() {
                return node.concrete_intrinsic;
            }
            if let Some(child) = node.array_child {
                current = self.root(child);
            } else {
                return false;
            }
        }
        false
    }

    fn retain_assignment_origin(&mut self, id: u64, origin: &Span) {
        let mut current = self.root(id);
        for _ in 0..=128 {
            if let Some((ty, _)) = self.graph.type_nodes[&current].concrete.clone() {
                self.graph.type_node_mut(&current).unwrap().concrete = Some((ty, origin.clone()));
                self.graph
                    .type_nodes
                    .get_mut(&current)
                    .unwrap()
                    .concrete_intrinsic = false;
                return;
            }
            let Some(child) = self.graph.type_nodes[&current].array_child else {
                return;
            };
            current = self.root(child);
        }
    }

    fn type_determined(&mut self, ty: &Type) -> bool {
        match ty {
            Type::Deferred(id) => {
                let root = self.root(*id);
                let node = &self.graph.type_nodes[&root];
                node.concrete.is_some()
                    || node.array_child.is_some()
                    || node.variant_children.is_some()
            }
            Type::Never | Type::Tainted => false,
            _ => true,
        }
    }

    fn resolved_syntax_type(&mut self, ty: &Type) -> Option<SyntaxType> {
        enum Frame {
            Enter(Type),
            Array,
            Option,
            Result,
        }
        let mut frames = vec![Frame::Enter(ty.clone())];
        let mut values = Vec::new();
        while let Some(frame) = frames.pop() {
            match frame {
                Frame::Enter(ty) => match ty {
                    Type::Deferred(id) => {
                        let (resolved, _) = self.concrete(id)?;
                        values.push(syntax_type(&resolved)?);
                    }
                    Type::Array(inner) => {
                        frames.push(Frame::Array);
                        frames.push(Frame::Enter(inner.as_ref().clone()));
                    }
                    Type::Union(UnionKind::Result, ok, Some(err)) => {
                        frames.push(Frame::Result);
                        frames.push(Frame::Enter(err.as_ref().clone()));
                        frames.push(Frame::Enter(ok.as_ref().clone()));
                    }
                    Type::Union(UnionKind::Option, some, None) => {
                        frames.push(Frame::Option);
                        frames.push(Frame::Enter(some.as_ref().clone()));
                    }
                    Type::Union(..) => return None,
                    _ => values.push(syntax_type(&ty)?),
                },
                Frame::Array => {
                    let child = values.pop()?;
                    values.push(SyntaxType::Array(Box::new(child)));
                }
                Frame::Option => {
                    let child = values.pop()?;
                    values.push(SyntaxType::Option(Box::new(child)));
                }
                Frame::Result => {
                    let err = values.pop()?;
                    let ok = values.pop()?;
                    values.push(SyntaxType::Result(Box::new(ok), Box::new(err)));
                }
            }
        }
        values.pop()
    }

    // Coverage and pattern bindings need the constructor identities already
    // established in the inference graph, even when a payload stays open.
    fn selection_type(&mut self, ty: &Type, depth: usize) -> Type {
        enum Frame {
            Enter(Type, usize),
            Array,
            Option,
            Result,
        }
        let mut frames = vec![Frame::Enter(ty.clone(), depth)];
        let mut values = Vec::new();
        while let Some(frame) = frames.pop() {
            match frame {
                Frame::Enter(ty, depth) => {
                    if depth > 128 {
                        values.push(ty);
                        continue;
                    }
                    match ty {
                        Type::Deferred(id) => {
                            if let Some((concrete, _)) = self.concrete(id) {
                                values.push(concrete);
                                continue;
                            }
                            let root = self.root(id);
                            let node = self.graph.type_nodes[&root].clone();
                            if let Some(child) = node.array_child {
                                frames.push(Frame::Array);
                                frames.push(Frame::Enter(Type::Deferred(child), depth + 1));
                            } else if let Some(union) = node.variant_children {
                                let payloads: Vec<_> = union
                                    .variants
                                    .iter()
                                    .filter_map(|variant| variant.payload)
                                    .collect();
                                match (union.kind, payloads.as_slice()) {
                                    (UnionKind::Option, [some]) => {
                                        frames.push(Frame::Option);
                                        frames.push(Frame::Enter(Type::Deferred(*some), depth + 1));
                                    }
                                    (UnionKind::Result, [ok, err]) => {
                                        frames.push(Frame::Result);
                                        frames.push(Frame::Enter(Type::Deferred(*err), depth + 1));
                                        frames.push(Frame::Enter(Type::Deferred(*ok), depth + 1));
                                    }
                                    _ => values.push(Type::Deferred(id)),
                                }
                            } else {
                                values.push(Type::Deferred(id));
                            }
                        }
                        Type::Array(child) => {
                            frames.push(Frame::Array);
                            frames.push(Frame::Enter(child.as_ref().clone(), depth + 1));
                        }
                        Type::Union(UnionKind::Option, some, None) => {
                            frames.push(Frame::Option);
                            frames.push(Frame::Enter(some.as_ref().clone(), depth + 1));
                        }
                        Type::Union(UnionKind::Result, ok, Some(err)) => {
                            frames.push(Frame::Result);
                            frames.push(Frame::Enter(err.as_ref().clone(), depth + 1));
                            frames.push(Frame::Enter(ok.as_ref().clone(), depth + 1));
                        }
                        other => values.push(other),
                    }
                }
                Frame::Array => {
                    let child = values.pop().unwrap();
                    values.push(Type::Array(Arc::new(child)));
                }
                Frame::Option => {
                    let child = values.pop().unwrap();
                    values.push(Type::option(Arc::new(child)));
                }
                Frame::Result => {
                    let err = values.pop().unwrap();
                    let ok = values.pop().unwrap();
                    values.push(Type::result(Arc::new(ok), Arc::new(err)));
                }
            }
        }
        values.pop().unwrap()
    }

    fn with_independent_fact<R>(
        &mut self,
        site: (u64, u64),
        action: impl FnOnce(&mut Self, FactId) -> R,
    ) -> R {
        let fact = self
            .graph
            .begin_visit(site, FactRole::Solver(SolverBindSite::Ordinary));
        let result = action(self, fact);
        self.graph.end_visit(fact);
        result
    }

    #[allow(clippy::too_many_arguments)]
    fn bind_independent(
        &mut self,
        id: u64,
        ty: Type,
        origin: Span,
        found: Span,
        intrinsic: bool,
    ) -> Result<(), Box<Diagnostic>> {
        self.with_independent_fact(span_key(&found), |checker, fact| {
            checker.bind(fact, id, ty, origin, found, intrinsic)
        })
    }

    fn bind(
        &mut self,
        fact: FactId,
        id: u64,
        ty: Type,
        origin: Span,
        found: Span,
        intrinsic: bool,
    ) -> Result<(), Box<Diagnostic>> {
        let request = SolverBind {
            fact,
            parent_fact: fact,
            site: SolverBindSite::Ordinary,
            id,
            ty,
            origin,
            found,
            intrinsic,
        };
        match self.solve(SolverRequest::Bind(request))? {
            SolverValue::Unit => Ok(()),
            _ => unreachable!("bind result"),
        }
    }

    fn fail_constructed_dependents(&mut self, element: u64) {
        // §13.9: an array whose element already failed a concrete constraint
        // must not add an unresolved-constructor consequence at function end.
        for (child, array, _) in self.pending_constructors() {
            let child_root = self.root(child);
            if child_root == element && self.graph.type_nodes[&child_root].bottom_only {
                let array_root = self.root(array);
                self.graph.mark_failed_constructor(array_root);
            }
        }
    }

    fn array_domain_conflict(&mut self, root: u64, other: &Type, found: Span) -> Box<Diagnostic> {
        // A deferred destination may acquire a concrete array element while
        // its RHS is checked. Report that concrete expected type before the
        // fallback domain label; §13.17 lists the allowed domain spelling.
        if let Some((array, array_origin)) = self.concrete(root)
            && let Some(label) = pretty(&array)
        {
            let related = (!self.origin_intrinsic(root)).then_some(array_origin);
            return mismatch(found.clone(), found, related, &label, other);
        }
        if self.graph.type_nodes[&root].array_constructed {
            return self.unresolved_constructor_diagnostic(root, found);
        }
        domain_error(
            "not_indexable",
            found.clone(),
            found,
            None,
            "domain:array",
            other,
        )
    }

    fn unresolved_constructor_diagnostic(&self, root: u64, found: Span) -> Box<Diagnostic> {
        // S10 and §4.1.1 locate an unresolved constructor at its source,
        // even when a later operand exposes the incompatible constraint.
        let origin = self.graph.type_nodes[&root]
            .array_origin
            .clone()
            .unwrap_or(found);
        names::diagnostic(
            "NX-E0103",
            None,
            Stage::Types,
            Operation::TypesInfer,
            origin,
        )
    }

    fn recursive_path_origin(&self, start: u64, target: u64) -> Option<(Span, usize)> {
        let root = |mut id| {
            while self.graph.type_nodes[&id].parent != id {
                id = self.graph.type_nodes[&id].parent;
            }
            id
        };
        let constructors: Vec<_> = self
            .graph
            .obligations
            .iter()
            .filter_map(|entry| match entry {
                ObligationKind::Constructor {
                    child,
                    array,
                    span,
                    has_element_witness,
                } => Some((*child, *array, span, *has_element_witness)),
                _ => None,
            })
            .collect();
        let mut work = vec![(start, None::<Span>, 0_usize)];
        let mut seen = HashSet::new();
        while let Some((id, origin, constructor_count)) = work.pop() {
            let id = root(id);
            if !seen.insert(id) {
                continue;
            }
            if id == target {
                return origin.map(|span| (span, constructor_count));
            }
            let node = &self.graph.type_nodes[&id];
            if let Some(child) = node.array_child {
                let child = root(child);
                let edges: Vec<_> = constructors
                    .iter()
                    .filter(|(element, array, _, _)| root(*array) == id && root(*element) == child)
                    .collect();
                let edge = edges
                    .iter()
                    .filter(|(_, _, _, witnessed)| *witnessed)
                    .max_by_key(|(_, _, span, _)| (span.start, span.end))
                    .map(|(_, _, span, _)| (*span).clone());
                let latest = match (origin.clone(), edge) {
                    (Some(a), Some(b)) if b.start > a.start => Some(b),
                    (a @ Some(_), _) => a,
                    (None, b) => b,
                };
                work.push((child, latest, constructor_count + edges.len()));
            }
            if let Some(union) = node.variant_children {
                for variant in union.variants {
                    if let Some(child) = variant.payload {
                        work.push((child, origin.clone(), constructor_count));
                    }
                }
            }
        }
        None
    }

    fn recursive_equality_diagnostic(&self, left: u64, right: u64, found: Span) -> Box<Diagnostic> {
        // A prior constructor owns the diagnostic only when it is the sole
        // constructor duty on the causal path. With several duties, the
        // equality that discovers the cycle owns its own conflict; choosing
        // a global constructor span would consume another duty's evidence.
        let origin = self
            .recursive_path_origin(left, right)
            .or_else(|| self.recursive_path_origin(right, left))
            .filter(|(span, count)| {
                *count == 1 && span.end <= self.top_statement_start && self.active_if_span.is_none()
            })
            .map_or(found, |(span, _)| span);
        names::diagnostic(
            "NX-E0103",
            None,
            Stage::Types,
            Operation::TypesInfer,
            origin,
        )
    }

    fn unresolved_constructor_shape_conflict(
        &mut self,
        id: u64,
        against: &Type,
        found: &Span,
    ) -> Result<Option<u64>, Box<Diagnostic>> {
        let mut current = self.root(id);
        let mut part = against;
        let mut constructed_owner = None;
        // source types deeper than 128 are rejected by the array-depth
        // limit before this shape walk; iteration 129 has no reachable node.
        for _ in 0..=128 {
            let node = self.graph.type_nodes[&current].clone();
            if node.variant_children.is_some() && self.result_shape_conflict(current, part, 0) {
                return Ok(Some(current));
            }
            if node.array_constructed {
                constructed_owner = Some(current);
            }
            if node.concrete.is_some() {
                // A concrete array has already bound every literal child
                // along this path. Continuing would find no unresolved
                // constructed child; the guard avoids needless traversal.
                return Ok(None);
            }
            let Some(child) = node.array_child else {
                return Ok(None);
            };
            self.charge_solver(found)?;
            if let Type::Array(element) = part {
                current = self.root(child);
                part = element;
            } else {
                return Ok(constructed_owner);
            }
        }
        Ok(None)
    }

    fn pattern_shape_mismatch(
        &mut self,
        id: u64,
        expected: &Type,
        expected_origin: &Span,
        found: &Span,
    ) -> Option<Box<Diagnostic>> {
        let expected_label = pretty(expected)?;
        let root = self.root(id);
        if self.concrete(root).is_some() {
            return None;
        }
        let pattern_sites: Vec<_> = self
            .graph
            .obligations
            .iter()
            .filter_map(|obligation| match obligation {
                ObligationKind::PatternConstructor { union, span } => Some((*union, span.clone())),
                _ => None,
            })
            .collect();
        let mut origins = HashMap::new();
        for (union, span) in pattern_sites {
            origins.entry(self.root(union)).or_insert(span);
        }
        let mut pending = vec![(root, expected.clone(), 0_usize)];
        let mut conflict = None;
        while let Some((id, wanted, depth)) = pending.pop() {
            if depth > 128 {
                break;
            }
            let root = self.root(id);
            if self.concrete(root).is_some() {
                continue;
            }
            let Some(actual) = self.graph.type_nodes[&root].variant_children else {
                continue;
            };
            let wanted_union = concrete_union(&wanted);
            if wanted_union.is_none_or(|union| union.kind != actual.kind) {
                if let Some(origin) = origins.get(&root) {
                    conflict = Some((actual.kind, origin.clone()));
                    break;
                }
                continue;
            }
            let wanted_union = wanted_union.unwrap();
            for (actual, wanted) in actual.variants.iter().zip(wanted_union.variants).rev() {
                if let (Some(child), Some(expected_child)) = (actual.payload, wanted.payload) {
                    pending.push((child, expected_child.clone(), depth + 1));
                }
            }
        }
        let (kind, origin) = conflict?;
        let mut diagnostic = names::diagnostic(
            "NX-E0102",
            Some("constructor_shape"),
            Stage::Types,
            Operation::TypesCheck,
            found.clone(),
        );
        diagnostic
            .data
            .insert("expected_type".into(), DataValue::String(expected_label));
        diagnostic.data.insert(
            "found_constructor".into(),
            DataValue::String(match kind {
                UnionKind::Option => "Option".into(),
                UnionKind::Result => "Result".into(),
            }),
        );
        diagnostic.message = "La forma del valor no coincide con el tipo esperado.".into();
        diagnostic.related_spans.push(RelatedSpan {
            role: RelatedRole::Origin,
            span: origin,
        });
        if expected_origin != found {
            diagnostic.related_spans.push(RelatedSpan {
                role: RelatedRole::ExpectedType,
                span: expected_origin.clone(),
            });
        }
        diagnostic.related_spans.push(RelatedSpan {
            role: RelatedRole::FoundType,
            span: found.clone(),
        });
        Some(diagnostic)
    }

    fn result_shape_conflict(&mut self, id: u64, against: &Type, depth: usize) -> bool {
        let mut frames = vec![(id, against.clone(), depth)];
        while let Some((id, against, depth)) = frames.pop() {
            if depth > 128 || matches!(against, Type::Never | Type::Tainted | Type::Deferred(_)) {
                continue;
            }
            let root = self.root(id);
            let Some(union) = self.graph.type_nodes[&root].variant_children else {
                continue;
            };
            let Some(wanted) = concrete_union(&against) else {
                return true;
            };
            if union.kind != wanted.kind {
                return true;
            }
            for (actual, wanted) in union.variants.iter().zip(wanted.variants).rev() {
                match (actual.payload, wanted.payload) {
                    (Some(actual), Some(wanted)) => {
                        frames.push((actual, wanted.clone(), depth + 1));
                    }
                    (None, None) => {}
                    _ => return true,
                }
            }
        }
        false
    }

    #[allow(clippy::too_many_arguments)] // The visit is threaded through the existing solver call.
    fn defer_shape_conflict(
        &mut self,
        fact: FactId,
        left: Type,
        right: Type,
        constructor: u64,
        left_origin: Span,
        found: Span,
        policy: UnifyPolicy,
    ) {
        // A failed branch equality belongs to that use; it does not
        // invalidate the constructor shared by later independent uses
        // (§4.1.2.5). Other shape failures can suppress a consequent
        // unresolved-constructor diagnostic at finalization.
        if !policy.branch {
            self.graph.mark_failed_constructor(constructor);
        }
        if policy.branch {
            self.graph
                .type_node_mut(&constructor)
                .unwrap()
                .deferred_join = true;
            let scope = self
                .active_if_span
                .as_ref()
                .map_or((self.top_statement_start, self.top_statement_end), |span| {
                    (span.start, span.end)
                });
            self.graph
                .type_node_mut(&constructor)
                .unwrap()
                .branch_join_scopes
                .push(scope);
        }
        self.graph
            .register(ObligationKind::Join(Box::new(PendingDeferredJoin {
                equality: self.graph.active_equality,
                left,
                right,
                constructor,
                left_origin,
                found,
                recursive_origin: None,
                branch: policy.branch,
                related_expected: policy.related_expected,
                numeric_domain_owner: self.graph.fact(fact).numeric_domain_owner,
                comparison_domain_if_unresolved: None,
            })));
    }

    fn resolved_join_side(&mut self, ty: Type) -> Option<Type> {
        enum Frame {
            Enter(Type),
            Array,
            Option,
            Result,
        }
        let mut frames = vec![Frame::Enter(ty)];
        let mut values = Vec::new();
        while let Some(frame) = frames.pop() {
            match frame {
                Frame::Enter(ty) => match ty {
                    Type::Deferred(id) => values.push(self.concrete(id)?.0),
                    Type::Array(child) => {
                        frames.push(Frame::Array);
                        frames.push(Frame::Enter(child.as_ref().clone()));
                    }
                    Type::Union(UnionKind::Result, ok, Some(err)) => {
                        frames.push(Frame::Result);
                        frames.push(Frame::Enter(err.as_ref().clone()));
                        frames.push(Frame::Enter(ok.as_ref().clone()));
                    }
                    Type::Union(UnionKind::Option, some, None) => {
                        frames.push(Frame::Option);
                        frames.push(Frame::Enter(some.as_ref().clone()));
                    }
                    Type::Union(..) | Type::Never | Type::Tainted => return None,
                    other => values.push(other),
                },
                Frame::Array => {
                    let child = values.pop()?;
                    values.push(Type::Array(Arc::new(child)));
                }
                Frame::Option => {
                    let child = values.pop()?;
                    values.push(Type::option(Arc::new(child)));
                }
                Frame::Result => {
                    let err = values.pop()?;
                    let ok = values.pop()?;
                    values.push(Type::result(Arc::new(ok), Arc::new(err)));
                }
            }
        }
        values.pop()
    }

    /// The single structural unification operation used by source joins.
    /// Never contributes no constraint and taint contributes no new error.
    #[allow(clippy::too_many_arguments)]
    fn unify_independent(
        &mut self,
        left: Type,
        right: Type,
        left_origin: Span,
        right_origin: Span,
        found: Span,
        branch: bool,
    ) -> Result<Type, Box<Diagnostic>> {
        self.with_independent_fact(span_key(&found), |checker, fact| {
            checker.unify(fact, left, right, left_origin, right_origin, found, branch)
        })
    }

    #[allow(clippy::too_many_arguments)]
    fn unify_with_expected_origin_independent(
        &mut self,
        left: Type,
        right: Type,
        left_origin: Span,
        right_origin: Span,
        found: Span,
        policy: UnifyPolicy,
    ) -> Result<Type, Box<Diagnostic>> {
        self.with_independent_fact(span_key(&found), |checker, fact| {
            checker.unify_with_expected_origin(
                fact,
                left,
                right,
                left_origin,
                right_origin,
                found,
                policy,
            )
        })
    }

    #[allow(clippy::too_many_arguments)] // The visit is threaded through the existing solver call.
    fn unify(
        &mut self,
        fact: FactId,
        left: Type,
        right: Type,
        left_origin: Span,
        right_origin: Span,
        found: Span,
        branch: bool,
    ) -> Result<Type, Box<Diagnostic>> {
        self.unify_with_expected_origin(
            fact,
            left,
            right,
            left_origin,
            right_origin,
            found,
            UnifyPolicy {
                branch,
                related_expected: true,
            },
        )
    }

    #[allow(clippy::too_many_arguments)] // The visit is threaded through the existing solver call.
    fn unify_with_expected_origin(
        &mut self,
        fact: FactId,
        left: Type,
        right: Type,
        left_origin: Span,
        right_origin: Span,
        found: Span,
        policy: UnifyPolicy,
    ) -> Result<Type, Box<Diagnostic>> {
        let equality = self
            .graph
            .begin_equality(left.clone(), right.clone(), found.clone());
        let request = SolverUnify {
            fact,
            left,
            right,
            left_origin,
            right_origin,
            found,
            policy,
        };
        let result = self.solve(SolverRequest::Unify(request));
        self.graph.finish_equality(equality, result.is_ok());
        match result? {
            SolverValue::Type(ty) => Ok(ty),
            _ => unreachable!("unify result"),
        }
    }

    #[cfg(test)]
    fn union(&mut self, left: u64, right: u64, found: Span) -> Result<(), Box<Diagnostic>> {
        let result = self.with_independent_fact(span_key(&found), |checker, fact| {
            checker.solve(SolverRequest::Union(SolverUnion {
                left,
                right,
                found,
                parent_fact: fact,
            }))
        })?;
        match result {
            SolverValue::Unit => Ok(()),
            _ => unreachable!("union result"),
        }
    }

    fn solve(&mut self, request: SolverRequest) -> Result<SolverValue, Box<Diagnostic>> {
        enum Frame<'a> {
            Enter(SolverRequest),
            EndFact(FactId),
            AtomicEnd(Box<Checker<'a>>),
            UnionInner(SolverUnion),
            BindUnionAfterRequire {
                request: SolverBind,
                root: u64,
                was_concrete: bool,
            },
            BindUnionFinish {
                root: u64,
                was_concrete: bool,
                origin: Span,
            },
            BindArrayDone {
                expected: Option<Type>,
                found: Type,
            },
            RequireUnionFinish {
                root: u64,
                children: UnionDescriptor<u64>,
                origin: Span,
            },
            UnionAfterBinds(SolverUnionJoin),
            UnionMerge(SolverUnionJoin),
            UnifyAfterUnion {
                request: SolverUnify,
                a: u64,
                b: u64,
                a_root: u64,
                b_root: u64,
                pending_before: usize,
            },
            UnifyAfterBind {
                request: SolverUnify,
                id: u64,
                other: Type,
                left_deferred: bool,
                pending_before: usize,
            },
            UnifyArrayDone {
                key: (usize, usize),
                before_inner: u64,
                pending_before: usize,
                expected: Type,
                found: Type,
            },
            UnifyUnionNext {
                request: SolverUnify,
                kind: UnionKind,
                expected: Type,
                found: Type,
                remaining: Vec<(Type, Type)>,
                payloads: Vec<Type>,
                waiting: bool,
            },
            UnifyUnionWrap {
                expected: Type,
                found: Type,
            },
        }

        let mut frames = vec![Frame::Enter(request)];
        let mut outcome: Result<SolverValue, Box<Diagnostic>> = Ok(SolverValue::Unit);
        while let Some(frame) = frames.pop() {
            match frame {
                Frame::Enter(request) => {
                    if outcome.is_err() {
                        continue;
                    }
                    match request {
                        SolverRequest::Bind(mut request) => {
                            let inheritance = match request.site {
                                SolverBindSite::RequireUnion
                                | SolverBindSite::UnionArrayA
                                | SolverBindSite::UnionArrayB
                                | SolverBindSite::UnionPayloadA
                                | SolverBindSite::UnionPayloadB => FactInheritance::Empty,
                                SolverBindSite::Ordinary | SolverBindSite::InheritedPayload => {
                                    FactInheritance::Inherit
                                }
                            };
                            request.fact = self.graph.child_fact(
                                request.parent_fact,
                                span_key(&request.found),
                                FactRole::Solver(request.site),
                                inheritance,
                                inheritance,
                            );
                            frames.push(Frame::EndFact(request.fact));
                            let SolverBind {
                                fact: visit,
                                parent_fact,
                                site: _site,
                                id,
                                ty,
                                origin,
                                found,
                                intrinsic,
                            } = request;
                            #[cfg(test)]
                            record_solver_bind_entry(&self.graph, _site, parent_fact, visit);
                            if matches!(ty, Type::Never) {
                                outcome = Ok(SolverValue::Unit);
                                continue;
                            }
                            if let Type::Deferred(other) = ty {
                                frames.push(Frame::Enter(SolverRequest::Union(SolverUnion {
                                    left: id,
                                    right: other,
                                    found,
                                    parent_fact: visit,
                                })));
                                continue;
                            }
                            if let Some(wanted) = concrete_union(&ty) {
                                let root = self.root(id);
                                if internal_state(&ty).is_some()
                                    || self.graph.type_nodes[&root].variant_children.is_some()
                                {
                                    let kind = wanted.kind;
                                    frames.push(Frame::AtomicEnd(Box::new(self.clone())));
                                    let was_concrete = self.concrete(root).is_some();
                                    if let Some((previous, previous_origin)) =
                                        self.graph.type_nodes[&root].concrete.clone()
                                        && concrete_union(&previous)
                                            .is_none_or(|union| union.kind != kind)
                                    {
                                        outcome = match pretty(&previous) {
                                            Some(expected) => Err(mismatch(
                                                found.clone(),
                                                found,
                                                (!self.origin_intrinsic(root))
                                                    .then_some(previous_origin),
                                                &expected,
                                                &ty,
                                            )),
                                            None => Err(unresolved_type(found)),
                                        };
                                        continue;
                                    }
                                    frames.push(Frame::BindUnionAfterRequire {
                                        request: SolverBind {
                                            fact: visit,
                                            parent_fact,
                                            site: SolverBindSite::Ordinary,
                                            id,
                                            ty,
                                            origin,
                                            found: found.clone(),
                                            intrinsic,
                                        },
                                        root,
                                        was_concrete,
                                    });
                                    frames.push(Frame::Enter(SolverRequest::RequireUnion {
                                        id,
                                        kind,
                                        origin: found,
                                        parent_fact: visit,
                                    }));
                                    continue;
                                }
                            }
                            let root = self.root(id);
                            if self.graph.type_nodes[&root].variant_children.is_some() {
                                outcome = Err(unresolved_type(found));
                                continue;
                            }
                            if let Type::Array(element) = ty {
                                let conditional_selectors: Vec<_> = self
                                    .graph
                                    .obligations
                                    .iter()
                                    .filter_map(|entry| match entry {
                                        ObligationKind::Projection(projection) => projection
                                            .selector_variable
                                            .map(|selector| (projection.base, selector)),
                                        _ => None,
                                    })
                                    .collect();
                                if conditional_selectors.into_iter().any(|(base, selector)| {
                                    self.root(base) == root && self.root(selector) == root
                                }) {
                                    outcome = Err(mismatch(
                                        found.clone(),
                                        found,
                                        None,
                                        "I64",
                                        &Type::Array(element),
                                    ));
                                    continue;
                                }
                                if let Some((previous, previous_origin)) =
                                    self.graph.type_nodes[&root].concrete.clone()
                                {
                                    outcome = match pretty(&previous) {
                                        Some(expected) => Err(mismatch(
                                            found.clone(),
                                            found,
                                            (!self.origin_intrinsic(root))
                                                .then_some(previous_origin),
                                            &expected,
                                            &Type::Array(element),
                                        )),
                                        None => Err(unresolved_type(found)),
                                    };
                                    continue;
                                }
                                let whole_expected = self.concrete(root).map(|(ty, _)| ty);
                                let whole_found = Type::Array(element.clone());
                                match self.require_array(id, &found) {
                                    Ok(child) => {
                                        frames.push(Frame::BindArrayDone {
                                            expected: whole_expected,
                                            found: whole_found,
                                        });
                                        frames.push(Frame::Enter(SolverRequest::Bind(
                                            SolverBind::child(
                                                visit,
                                                SolverBindSite::Ordinary,
                                                child,
                                                element.as_ref().clone(),
                                                origin,
                                                found,
                                                intrinsic,
                                            ),
                                        )));
                                    }
                                    Err(error) => outcome = Err(error),
                                }
                                continue;
                            }
                            let root = self.root(id);
                            if let Err(error) = self.charge_solver(&found) {
                                outcome = Err(error);
                                continue;
                            }
                            if self.graph.type_nodes[&root].copyable_origin.is_some()
                                && !copyable(&ty)
                            {
                                self.fail_constructed_dependents(root);
                                outcome = Err(mismatch(
                                    found.clone(),
                                    found,
                                    None,
                                    "domain:copyable_data",
                                    &ty,
                                ));
                                continue;
                            }
                            if self.graph.type_nodes[&root].array_child.is_some() {
                                let child = self.graph.type_nodes[&root].array_child.unwrap();
                                if (self.graph.type_nodes[&root].array_constructed
                                    || self.graph.preseeded_children.contains(&child))
                                    && self.concrete(root).is_none()
                                {
                                    self.defer_shape_conflict(
                                        visit,
                                        Type::Deferred(root),
                                        ty,
                                        root,
                                        origin,
                                        found,
                                        UnifyPolicy {
                                            branch: false,
                                            related_expected: !intrinsic,
                                        },
                                    );
                                    outcome = Ok(SolverValue::Unit);
                                } else {
                                    self.graph.mark_failed_constructor(root);
                                    outcome = Err(self.array_domain_conflict(root, &ty, found));
                                }
                                continue;
                            }
                            if let Some((previous, previous_origin)) = self.concrete(root) {
                                if previous != ty {
                                    outcome = match pretty(&previous) {
                                        Some(expected) => Err(mismatch(
                                            found.clone(),
                                            found,
                                            (!self.origin_intrinsic(root))
                                                .then_some(previous_origin),
                                            &expected,
                                            &ty,
                                        )),
                                        None => Err(unresolved_type(found)),
                                    };
                                } else {
                                    outcome = Ok(SolverValue::Unit);
                                }
                            } else {
                                self.graph.type_node_mut(&root).unwrap().concrete =
                                    Some((ty, origin));
                                self.graph
                                    .type_nodes
                                    .get_mut(&root)
                                    .unwrap()
                                    .concrete_intrinsic = intrinsic;
                                outcome = Ok(SolverValue::Unit);
                            }
                        }
                        SolverRequest::RequireUnion {
                            id,
                            kind,
                            origin,
                            parent_fact,
                        } => {
                            let parent_fact = self.graph.child_fact(
                                parent_fact,
                                span_key(&origin),
                                FactRole::Solver(SolverBindSite::Ordinary),
                                FactInheritance::Inherit,
                                FactInheritance::Inherit,
                            );
                            frames.push(Frame::EndFact(parent_fact));
                            if let Err(error) = self.charge_solver(&origin) {
                                outcome = Err(error);
                                continue;
                            }
                            let root = self.root(id);
                            if let Some(children) = self.graph.type_nodes[&root].variant_children {
                                outcome = if children.kind == kind {
                                    Ok(SolverValue::Union(children))
                                } else {
                                    Err(unresolved_type(origin))
                                };
                                continue;
                            }
                            if self.graph.type_nodes[&root].array_child.is_some()
                                || self.graph.type_nodes[&root].concrete.as_ref().is_some_and(
                                    |(ty, _)| {
                                        concrete_union(ty).is_none_or(|union| union.kind != kind)
                                    },
                                )
                            {
                                outcome = Err(unresolved_type(origin));
                                continue;
                            }
                            let first = match self.new_variable(&origin) {
                                Ok(id) => id,
                                Err(error) => {
                                    outcome = Err(error);
                                    continue;
                                }
                            };
                            let second = if kind == UnionKind::Result {
                                match self.new_variable(&origin) {
                                    Ok(id) => Some(id),
                                    Err(error) => {
                                        outcome = Err(error);
                                        continue;
                                    }
                                }
                            } else {
                                None
                            };
                            let children = if let Some(second) = second {
                                UnionDescriptor::result(first, second)
                            } else {
                                UnionDescriptor::option(first)
                            };
                            if let Some((concrete, previous_origin)) =
                                self.graph.type_nodes[&root].concrete.clone()
                                && let Some(wanted) = concrete_union(&concrete)
                            {
                                self.graph.type_node_mut(&root).unwrap().concrete = None;
                                let intrinsic = self.graph.type_nodes[&root].concrete_intrinsic;
                                let binds: Vec<_> = children
                                    .variants
                                    .iter()
                                    .zip(wanted.variants)
                                    .filter_map(|(actual, target)| {
                                        Some((actual.payload?, target.payload?.clone()))
                                    })
                                    .collect();
                                frames.push(Frame::RequireUnionFinish {
                                    root,
                                    children,
                                    origin: origin.clone(),
                                });
                                for (actual, target) in binds.into_iter().rev() {
                                    frames.push(Frame::Enter(SolverRequest::Bind(
                                        SolverBind::child(
                                            parent_fact,
                                            SolverBindSite::RequireUnion,
                                            actual,
                                            target,
                                            previous_origin.clone(),
                                            origin.clone(),
                                            intrinsic,
                                        ),
                                    )));
                                }
                            } else {
                                self.graph
                                    .type_nodes
                                    .get_mut(&root)
                                    .unwrap()
                                    .variant_children = Some(children);
                                self.graph.type_node_mut(&root).unwrap().variant_origin =
                                    Some(origin);
                                outcome = Ok(SolverValue::Union(children));
                            }
                        }
                        SolverRequest::Union(mut request) => {
                            request.parent_fact = self.graph.child_fact(
                                request.parent_fact,
                                span_key(&request.found),
                                FactRole::Solver(SolverBindSite::Ordinary),
                                FactInheritance::Inherit,
                                FactInheritance::Inherit,
                            );
                            frames.push(Frame::EndFact(request.parent_fact));
                            let l = self.root(request.left);
                            let r = self.root(request.right);
                            if self.graph.type_nodes[&l].variant_children.is_some()
                                || self.graph.type_nodes[&r].variant_children.is_some()
                            {
                                frames.push(Frame::AtomicEnd(Box::new(self.clone())));
                            }
                            frames.push(Frame::UnionInner(request));
                        }
                        SolverRequest::Unify(mut request) => {
                            request.fact = self.graph.child_fact(
                                request.fact,
                                span_key(&request.found),
                                FactRole::Solver(SolverBindSite::Ordinary),
                                FactInheritance::Inherit,
                                FactInheritance::Inherit,
                            );
                            frames.push(Frame::EndFact(request.fact));
                            if let Err(error) = self.charge_solver(&request.found) {
                                outcome = Err(error);
                                continue;
                            }
                            match (request.left.clone(), request.right.clone()) {
                                (Type::Tainted, _) | (_, Type::Tainted) => {
                                    outcome = Ok(SolverValue::Type(Type::Tainted));
                                }
                                (Type::Never, other) | (other, Type::Never) => {
                                    outcome = Ok(SolverValue::Type(other));
                                }
                                (Type::Deferred(a), Type::Deferred(b)) => {
                                    let prepared = (|| -> Result<Option<Type>, Box<Diagnostic>> {
                                        if request.policy.branch
                                            && let (Some((left, _)), Some((right, _))) =
                                                (self.concrete(a), self.concrete(b))
                                            && left != right
                                        {
                                            return Err(compare_source_site(
                                                &left,
                                                &right,
                                                request.left_origin.clone(),
                                                request.found.clone(),
                                                request.policy,
                                            )
                                            .unwrap_or_else(|| {
                                                unresolved_type(request.found.clone())
                                            }));
                                        }
                                        let l = self.concrete(a);
                                        let r = self.concrete(b);
                                        let a_root = self.root(a);
                                        let b_root = self.root(b);
                                        let incompatible_shapes =
                                            self.graph.type_nodes[&a_root].array_child.is_some()
                                                && self.graph.type_nodes[&b_root]
                                                    .variant_children
                                                    .is_some()
                                                || self.graph.type_nodes[&a_root]
                                                    .variant_children
                                                    .is_some()
                                                    && self.graph.type_nodes[&b_root]
                                                        .array_child
                                                        .is_some();
                                        if incompatible_shapes {
                                            let constructor = if self.graph.type_nodes[&a_root]
                                                .variant_children
                                                .is_some()
                                            {
                                                a_root
                                            } else {
                                                b_root
                                            };
                                            self.defer_shape_conflict(
                                                request.fact,
                                                Type::Deferred(a),
                                                Type::Deferred(b),
                                                constructor,
                                                request.left_origin.clone(),
                                                request.found.clone(),
                                                request.policy,
                                            );
                                            return Ok(Some(Type::Deferred(a_root)));
                                        }
                                        let constructor = match (&l, &r) {
                                            (Some((ty, _)), None) => self
                                                .unresolved_constructor_shape_conflict(
                                                    b,
                                                    ty,
                                                    &request.found,
                                                )?,
                                            (None, Some((ty, _))) => self
                                                .unresolved_constructor_shape_conflict(
                                                    a,
                                                    ty,
                                                    &request.found,
                                                )?,
                                            _ => None,
                                        };
                                        if let Some(constructor) = constructor {
                                            self.charge_solver(&request.found)?;
                                            self.defer_shape_conflict(
                                                request.fact,
                                                Type::Deferred(a),
                                                Type::Deferred(b),
                                                constructor,
                                                request.left_origin.clone(),
                                                request.found.clone(),
                                                request.policy,
                                            );
                                            return Ok(Some(Type::Deferred(self.root(a))));
                                        }
                                        Ok(None)
                                    })();
                                    match prepared {
                                        Ok(Some(ty)) => outcome = Ok(SolverValue::Type(ty)),
                                        Ok(None) => {
                                            let a_root = self.root(a);
                                            let b_root = self.root(b);
                                            let pending_before = self.pending_join_count();
                                            let union_found = request.found.clone();
                                            let union_visit = request.fact;
                                            frames.push(Frame::UnifyAfterUnion {
                                                request: SolverUnify {
                                                    left: Type::Deferred(a),
                                                    right: Type::Deferred(b),
                                                    ..request
                                                },
                                                a,
                                                b,
                                                a_root,
                                                b_root,
                                                pending_before,
                                            });
                                            frames.push(Frame::Enter(SolverRequest::Union(
                                                SolverUnion {
                                                    left: a,
                                                    right: b,
                                                    found: union_found,
                                                    parent_fact: union_visit,
                                                },
                                            )));
                                        }
                                        Err(error) => outcome = Err(error),
                                    }
                                }
                                (Type::Deferred(id), other) => {
                                    let prepared = (|| -> Result<Option<Type>, Box<Diagnostic>> {
                                        if let Some(error) = self.pattern_shape_mismatch(
                                            id,
                                            &other,
                                            &request.right_origin,
                                            &request.found,
                                        ) {
                                            return Err(error);
                                        }
                                        if request.policy.branch
                                            && internal_state(&other).is_none()
                                            && let Some((left, _)) = self.concrete(id)
                                            && left != other
                                        {
                                            return Err(compare_source_site(
                                                &left,
                                                &other,
                                                request.left_origin.clone(),
                                                request.found.clone(),
                                                request.policy,
                                            )
                                            .unwrap_or_else(|| {
                                                unresolved_type(request.found.clone())
                                            }));
                                        }
                                        if internal_state(&other).is_none()
                                            && let Some(constructor) = self
                                                .unresolved_constructor_shape_conflict(
                                                    id,
                                                    &other,
                                                    &request.found,
                                                )?
                                        {
                                            self.charge_solver(&request.found)?;
                                            self.defer_shape_conflict(
                                                request.fact,
                                                Type::Deferred(id),
                                                other.clone(),
                                                constructor,
                                                request.left_origin.clone(),
                                                request.found.clone(),
                                                request.policy,
                                            );
                                            return Ok(Some(Type::Deferred(self.root(id))));
                                        }
                                        Ok(None)
                                    })();
                                    match prepared {
                                        Ok(Some(ty)) => outcome = Ok(SolverValue::Type(ty)),
                                        Ok(None) => {
                                            let pending_before = self.pending_join_count();
                                            let bind = SolverBind::child(
                                                request.fact,
                                                SolverBindSite::Ordinary,
                                                id,
                                                other.clone(),
                                                request.right_origin.clone(),
                                                request.found.clone(),
                                                !request.policy.related_expected,
                                            );
                                            frames.push(Frame::UnifyAfterBind {
                                                request,
                                                id,
                                                other,
                                                left_deferred: true,
                                                pending_before,
                                            });
                                            frames.push(Frame::Enter(SolverRequest::Bind(bind)));
                                        }
                                        Err(error) => outcome = Err(error),
                                    }
                                }
                                (other, Type::Deferred(id)) => {
                                    let prepared = (|| -> Result<Option<Type>, Box<Diagnostic>> {
                                        if let Some(error) = self.pattern_shape_mismatch(
                                            id,
                                            &other,
                                            &request.left_origin,
                                            &request.found,
                                        ) {
                                            return Err(error);
                                        }
                                        if internal_state(&other).is_none()
                                            && let Some((right, _)) = self.concrete(id)
                                            && other != right
                                        {
                                            return Err(compare_source_site(
                                                &other,
                                                &right,
                                                request.left_origin.clone(),
                                                request.found.clone(),
                                                request.policy,
                                            )
                                            .unwrap_or_else(|| {
                                                unresolved_type(request.found.clone())
                                            }));
                                        }
                                        if internal_state(&other).is_none()
                                            && let Some(constructor) = self
                                                .unresolved_constructor_shape_conflict(
                                                    id,
                                                    &other,
                                                    &request.found,
                                                )?
                                        {
                                            self.charge_solver(&request.found)?;
                                            self.defer_shape_conflict(
                                                request.fact,
                                                other.clone(),
                                                Type::Deferred(id),
                                                constructor,
                                                request.left_origin.clone(),
                                                request.found.clone(),
                                                request.policy,
                                            );
                                            return Ok(Some(Type::Deferred(self.root(id))));
                                        }
                                        Ok(None)
                                    })();
                                    match prepared {
                                        Ok(Some(ty)) => outcome = Ok(SolverValue::Type(ty)),
                                        Ok(None) => {
                                            let pending_before = self.pending_join_count();
                                            let bind = SolverBind::child(
                                                request.fact,
                                                SolverBindSite::Ordinary,
                                                id,
                                                other.clone(),
                                                request.left_origin.clone(),
                                                request.found.clone(),
                                                !request.policy.related_expected,
                                            );
                                            frames.push(Frame::UnifyAfterBind {
                                                request,
                                                id,
                                                other,
                                                left_deferred: false,
                                                pending_before,
                                            });
                                            frames.push(Frame::Enter(SolverRequest::Bind(bind)));
                                        }
                                        Err(error) => outcome = Err(error),
                                    }
                                }
                                (Type::Array(a), Type::Array(b)) => {
                                    let key = (Arc::as_ptr(&a) as usize, Arc::as_ptr(&b) as usize);
                                    if let Some((_, _, resolved, inner_pairs)) =
                                        self.graph.array_unify_cache.get(&key).cloned()
                                    {
                                        let mut error = None;
                                        for _ in 0..inner_pairs {
                                            if let Err(failed) = self.charge_solver(&request.found)
                                            {
                                                error = Some(failed);
                                                break;
                                            }
                                        }
                                        outcome = match error {
                                            Some(error) => Err(error),
                                            None => Ok(SolverValue::Type(resolved)),
                                        };
                                        continue;
                                    }
                                    let before_inner = self.solver_steps;
                                    let expected = Type::Array(a.clone());
                                    let found = Type::Array(b.clone());
                                    let pending_before = self.pending_join_count();
                                    let child = SolverUnify {
                                        fact: request.fact,
                                        left: a.as_ref().clone(),
                                        right: b.as_ref().clone(),
                                        left_origin: request.left_origin,
                                        right_origin: request.right_origin,
                                        found: request.found,
                                        policy: UnifyPolicy {
                                            branch: request.policy.branch,
                                            related_expected: true,
                                        },
                                    };
                                    frames.push(Frame::UnifyArrayDone {
                                        key,
                                        before_inner,
                                        pending_before,
                                        expected,
                                        found,
                                    });
                                    frames.push(Frame::Enter(SolverRequest::Unify(child)));
                                }
                                (expected, found_type)
                                    if concrete_union(&expected).is_some()
                                        && concrete_union(&found_type).is_some() =>
                                {
                                    let left_union = concrete_union(&expected).unwrap();
                                    let right_union = concrete_union(&found_type).unwrap();
                                    if left_union.kind != right_union.kind {
                                        outcome = match compare_source_site(
                                            &expected,
                                            &found_type,
                                            request.left_origin,
                                            request.found,
                                            request.policy,
                                        ) {
                                            Some(error) => Err(error),
                                            None => Ok(SolverValue::Type(expected)),
                                        };
                                        continue;
                                    }
                                    let pairs: Vec<_> = left_union
                                        .variants
                                        .iter()
                                        .zip(right_union.variants)
                                        .filter_map(|(left, right)| {
                                            Some((left.payload?.clone(), right.payload?.clone()))
                                        })
                                        .collect();
                                    frames.push(Frame::UnifyUnionWrap {
                                        expected: expected.clone(),
                                        found: found_type.clone(),
                                    });
                                    if internal_state(&expected).is_some()
                                        || internal_state(&found_type).is_some()
                                    {
                                        frames.push(Frame::AtomicEnd(Box::new(self.clone())));
                                    }
                                    frames.push(Frame::UnifyUnionNext {
                                        request,
                                        kind: left_union.kind,
                                        expected,
                                        found: found_type,
                                        remaining: pairs,
                                        payloads: Vec::new(),
                                        waiting: false,
                                    });
                                }
                                (a, b) => {
                                    outcome = match compare_source_site(
                                        &a,
                                        &b,
                                        request.left_origin,
                                        request.found,
                                        request.policy,
                                    ) {
                                        Some(error) => Err(error),
                                        None => Ok(SolverValue::Type(a)),
                                    };
                                }
                            }
                        }
                    }
                }
                Frame::UnionInner(request) => {
                    if outcome.is_err() {
                        continue;
                    }
                    let prepared = (|| -> Result<Option<(SolverUnionJoin, Vec<SolverBind>)>, Box<Diagnostic>> {
                        self.charge_solver(&request.found)?;
                        let l = self.root(request.left);
                        let r = self.root(request.right);
                        if l == r {
                            return Ok(None);
                        }
                        if self.graph.type_nodes[&l].array_child.is_some()
                            && self.graph.type_nodes[&r].variant_children.is_some()
                            || self.graph.type_nodes[&l].variant_children.is_some()
                                && self.graph.type_nodes[&r].array_child.is_some()
                        {
                            return Err(unresolved_type(request.found.clone()));
                        }
                        if (self.graph.type_nodes[&l].array_child.is_some()
                            || self.graph.type_nodes[&l].variant_children.is_some()
                            || self.graph.type_nodes[&r].array_child.is_some()
                            || self.graph.type_nodes[&r].variant_children.is_some())
                            && (self.type_reaches(l, r, &request.found)?
                                || self.type_reaches(r, l, &request.found)?)
                        {
                            if self.graph.type_nodes[&l].array_child.is_some()
                                || self.graph.type_nodes[&r].array_child.is_some()
                            {
                                self.fail_constructed_dependents(l);
                                self.fail_constructed_dependents(r);
                            }
                            return Err(self.recursive_equality_diagnostic(
                                l, r, request.found.clone(),
                            ));
                        }
                        let (a, b) = if self.graph.type_nodes[&l].size
                            > self.graph.type_nodes[&r].size
                            || (self.graph.type_nodes[&l].size
                                == self.graph.type_nodes[&r].size
                                && l < r)
                        {
                            (l, r)
                        } else {
                            (r, l)
                        };
                        let a_concrete = self.graph.type_nodes[&a].concrete.clone();
                        let b_concrete = self.graph.type_nodes[&b].concrete.clone();
                        let a_intrinsic = self.graph.type_nodes[&a].concrete_intrinsic;
                        let b_intrinsic = self.graph.type_nodes[&b].concrete_intrinsic;
                        let a_child = self.graph.type_nodes[&a].array_child;
                        let b_child = self.graph.type_nodes[&b].array_child;
                        let a_result = self.graph.type_nodes[&a].variant_children;
                        let b_result = self.graph.type_nodes[&b].variant_children;
                        if let (Some(left), Some(right)) = (a_result, b_result)
                            && left.kind != right.kind
                        {
                            return Err(unresolved_type(request.found.clone()));
                        }
                        if let Some((other, _)) = &b_concrete
                            && a_result.is_some()
                            && concrete_union(other)
                                .is_none_or(|union| Some(union.kind) != a_result.map(|u| u.kind))
                        {
                            return Err(unresolved_type(request.found.clone()));
                        }
                        if let Some((other, _)) = &a_concrete
                            && b_result.is_some()
                            && concrete_union(other)
                                .is_none_or(|union| Some(union.kind) != b_result.map(|u| u.kind))
                        {
                            return Err(unresolved_type(request.found.clone()));
                        }
                        let (a_full, b_full) = if a_result.is_some() || b_result.is_some() {
                            (self.concrete(a), self.concrete(b))
                        } else {
                            (None, None)
                        };
                        let variant_constructed = self.graph.type_nodes[&a].variant_constructed
                            || self.graph.type_nodes[&b].variant_constructed;
                        let array_constructed = self.graph.type_nodes[&a].array_constructed
                            || self.graph.type_nodes[&b].array_constructed;
                        let bottom_only = self.graph.type_nodes[&a].bottom_only
                            || self.graph.type_nodes[&b].bottom_only;
                        let a_copyable = self.graph.type_nodes[&a].copyable_origin.clone();
                        let b_copyable = self.graph.type_nodes[&b].copyable_origin.clone();
                        if let (Some(_), Some((ty, _))) = (&a_copyable, &b_concrete)
                            && !copyable(ty)
                        {
                            return Err(mismatch(
                                request.found.clone(),
                                request.found.clone(),
                                None,
                                "domain:copyable_data",
                                ty,
                            ));
                        }
                        if let (Some(_), Some((ty, _))) = (&b_copyable, &a_concrete)
                            && !copyable(ty)
                        {
                            return Err(mismatch(
                                request.found.clone(),
                                request.found.clone(),
                                None,
                                "domain:copyable_data",
                                ty,
                            ));
                        }
                        let mut binds = Vec::new();
                        let mut push_bind = |site, id, ty, origin, intrinsic| {
                            binds.push(SolverBind::child(
                                request.parent_fact,
                                site,
                                id,
                                ty,
                                origin,
                                request.found.clone(),
                                intrinsic,
                            ));
                        };
                        if let (Some(child), Some((Type::Array(element), origin))) =
                            (a_child, &b_concrete)
                        {
                            push_bind(
                                SolverBindSite::UnionArrayA,
                                child,
                                element.as_ref().clone(),
                                origin.clone(),
                                b_intrinsic,
                            );
                        }
                        if let (Some(child), Some((Type::Array(element), origin))) =
                            (b_child, &a_concrete)
                        {
                            push_bind(
                                SolverBindSite::UnionArrayB,
                                child,
                                element.as_ref().clone(),
                                origin.clone(),
                                a_intrinsic,
                            );
                        }
                        if let (Some(children), Some((ty, origin))) = (a_result, &b_concrete)
                            && let Some(concrete) = concrete_union(ty)
                        {
                            for (child, target) in children.variants.iter().zip(concrete.variants) {
                                if let (Some(child), Some(target)) = (child.payload, target.payload) {
                                    push_bind(
                                        SolverBindSite::UnionPayloadA,
                                        child,
                                        target.clone(),
                                        origin.clone(),
                                        b_intrinsic,
                                    );
                                }
                            }
                        }
                        if let (Some(children), Some((ty, origin))) = (b_result, &a_concrete)
                            && let Some(concrete) = concrete_union(ty)
                        {
                            for (child, target) in children.variants.iter().zip(concrete.variants) {
                                if let (Some(child), Some(target)) = (child.payload, target.payload) {
                                    push_bind(
                                        SolverBindSite::UnionPayloadB,
                                        child,
                                        target.clone(),
                                        origin.clone(),
                                        a_intrinsic,
                                    );
                                }
                            }
                        }
                        Ok(Some((SolverUnionJoin {
                            a,
                            b,
                            found: request.found,
                            parent_fact: request.parent_fact,
                            a_concrete,
                            b_concrete,
                            a_intrinsic,
                            b_intrinsic,
                            a_child,
                            b_child,
                            a_result,
                            b_result,
                            a_full,
                            b_full,
                            variant_constructed,
                            array_constructed,
                            bottom_only,
                            a_copyable,
                            b_copyable,
                        }, binds)))
                    })();
                    match prepared {
                        Ok(None) => outcome = Ok(SolverValue::Unit),
                        Ok(Some((state, binds))) => {
                            frames.push(Frame::UnionAfterBinds(state));
                            for bind in binds.into_iter().rev() {
                                frames.push(Frame::Enter(SolverRequest::Bind(bind)));
                            }
                        }
                        Err(error) => outcome = Err(error),
                    }
                }
                Frame::EndFact(id) => self.graph.retire_fact(id),
                Frame::AtomicEnd(snapshot) => {
                    if outcome.is_err() {
                        let steps = self.solver_steps;
                        let quota = self.quota_error.clone();
                        *self = *snapshot;
                        self.solver_steps = steps;
                        if quota.is_some() {
                            self.quota_error = quota;
                        }
                    }
                }
                Frame::BindUnionAfterRequire {
                    request,
                    root,
                    was_concrete,
                } => {
                    if outcome.is_err() {
                        continue;
                    }
                    let SolverValue::Union(actual) = &outcome.as_ref().unwrap() else {
                        unreachable!("require_union continuation")
                    };
                    let wanted = concrete_union(&request.ty).unwrap();
                    let binds: Vec<_> = actual
                        .variants
                        .iter()
                        .zip(wanted.variants)
                        .filter_map(|(actual, target)| {
                            Some((actual.payload?, target.payload?.clone()))
                        })
                        .collect();
                    frames.push(Frame::BindUnionFinish {
                        root,
                        was_concrete,
                        origin: request.origin.clone(),
                    });
                    for (actual, target) in binds.into_iter().rev() {
                        frames.push(Frame::Enter(SolverRequest::Bind(SolverBind::child(
                            request.fact,
                            SolverBindSite::InheritedPayload,
                            actual,
                            target,
                            request.origin.clone(),
                            request.found.clone(),
                            request.intrinsic,
                        ))));
                    }
                }
                Frame::BindUnionFinish {
                    root,
                    was_concrete,
                    origin,
                } => {
                    if outcome.is_err() {
                        continue;
                    }
                    if !was_concrete && self.concrete(root).is_some() {
                        self.graph.type_node_mut(&root).unwrap().variant_origin = Some(origin);
                    }
                    outcome = Ok(SolverValue::Unit);
                }
                Frame::BindArrayDone { expected, found } => {
                    if let Err(error) = outcome {
                        outcome = Err(if let Some(expected) = expected {
                            array_mismatch(error, &expected, &found)
                        } else {
                            error
                        });
                    }
                }
                Frame::RequireUnionFinish {
                    root,
                    children,
                    origin,
                } => {
                    if outcome.is_err() {
                        continue;
                    }
                    self.graph.type_node_mut(&root).unwrap().variant_children = Some(children);
                    self.graph.type_node_mut(&root).unwrap().variant_origin = Some(origin);
                    outcome = Ok(SolverValue::Union(children));
                }
                Frame::UnionAfterBinds(state) => {
                    if outcome.is_err() {
                        continue;
                    }
                    let prepared = (|| -> Result<Vec<SolverUnion>, Box<Diagnostic>> {
                        if let Some((other, _)) = &state.b_concrete
                            && state.a_child.is_some()
                            && !matches!(other, Type::Array(_))
                        {
                            return Err(self.array_domain_conflict(
                                state.a,
                                other,
                                state.found.clone(),
                            ));
                        }
                        if let Some((other, _)) = &state.a_concrete
                            && state.b_child.is_some()
                            && !matches!(other, Type::Array(_))
                        {
                            return Err(self.array_domain_conflict(
                                state.b,
                                other,
                                state.found.clone(),
                            ));
                        }
                        if let (Some((a_ty, a_origin)), Some((b_ty, b_origin))) =
                            (&state.a_concrete, &state.b_concrete)
                            && a_ty != b_ty
                        {
                            let (expected, expected_origin, expected_intrinsic, actual) =
                                if a_origin.start <= b_origin.start {
                                    (a_ty, a_origin, state.a_intrinsic, b_ty)
                                } else {
                                    (b_ty, b_origin, state.b_intrinsic, a_ty)
                                };
                            let expected_label = pretty(expected)
                                .ok_or_else(|| unresolved_type(state.found.clone()))?;
                            return Err(mismatch(
                                state.found.clone(),
                                state.found.clone(),
                                (!expected_intrinsic).then_some(expected_origin.clone()),
                                &expected_label,
                                actual,
                            ));
                        }
                        let mut unions = Vec::new();
                        if let (Some(left), Some(right)) = (state.a_child, state.b_child) {
                            unions.push(SolverUnion {
                                left,
                                right,
                                found: state.found.clone(),
                                parent_fact: state.parent_fact,
                            });
                        }
                        if let (Some(left), Some(right)) = (state.a_result, state.b_result) {
                            for (left, right) in left.variants.iter().zip(right.variants) {
                                if let (Some(left), Some(right)) = (left.payload, right.payload) {
                                    unions.push(SolverUnion {
                                        left,
                                        right,
                                        found: state.found.clone(),
                                        parent_fact: state.parent_fact,
                                    });
                                }
                            }
                        }
                        Ok(unions)
                    })();
                    match prepared {
                        Ok(unions) => {
                            frames.push(Frame::UnionMerge(state));
                            for union in unions.into_iter().rev() {
                                frames.push(Frame::Enter(SolverRequest::Union(union)));
                            }
                        }
                        Err(error) => outcome = Err(error),
                    }
                }
                Frame::UnionMerge(state) => {
                    if outcome.is_err() {
                        continue;
                    }
                    let SolverUnionJoin {
                        a,
                        b,
                        found,
                        a_concrete,
                        b_concrete,
                        a_intrinsic,
                        b_intrinsic,
                        a_child,
                        b_child,
                        a_result,
                        b_result,
                        a_full,
                        b_full,
                        variant_constructed,
                        array_constructed,
                        bottom_only,
                        a_copyable,
                        b_copyable,
                        ..
                    } = state;
                    self.graph.type_node_mut(&b).unwrap().parent = a;
                    let b_join = self.graph.type_nodes[&b].clone();
                    let a_join = self.graph.type_node_mut(&a).unwrap();
                    a_join.deferred_join |= b_join.deferred_join;
                    a_join.recursive_join |= b_join.recursive_join;
                    a_join.branch_join_scopes.extend(b_join.branch_join_scopes);
                    self.graph.type_node_mut(&a).unwrap().size += self.graph.type_nodes[&b].size;
                    self.graph.type_node_mut(&a).unwrap().array_child = a_child.or(b_child);
                    self.graph.type_node_mut(&a).unwrap().variant_children = a_result.or(b_result);
                    self.graph.type_node_mut(&a).unwrap().variant_constructed = variant_constructed;
                    self.graph.type_node_mut(&a).unwrap().variant_origin = self.graph.type_nodes
                        [&a]
                        .variant_origin
                        .clone()
                        .or_else(|| self.graph.type_nodes[&b].variant_origin.clone());
                    if self.graph.type_nodes[&a].variant_children.is_some() {
                        if let Some(origin) = a_full.or(b_full).map(|(_, origin)| origin) {
                            self.graph.type_node_mut(&a).unwrap().variant_origin = Some(origin);
                        } else if self.concrete(a).is_some() {
                            self.graph.type_node_mut(&a).unwrap().variant_origin =
                                Some(found.clone());
                        }
                    }
                    self.graph.type_node_mut(&a).unwrap().array_constructed = array_constructed;
                    self.graph.type_node_mut(&a).unwrap().bottom_only = bottom_only;
                    self.graph.type_node_mut(&a).unwrap().copyable_origin =
                        match (a_copyable, b_copyable) {
                            (Some(a), Some(b)) => Some(if a.start <= b.start { a } else { b }),
                            (Some(a), None) | (None, Some(a)) => Some(a),
                            (None, None) => None,
                        };
                    let a_array_origin = self.graph.type_nodes[&a].array_origin.clone();
                    let b_array_origin = self.graph.type_nodes[&b].array_origin.clone();
                    self.graph.type_node_mut(&a).unwrap().array_origin =
                        match (a_array_origin, b_array_origin) {
                            (Some(a), Some(b)) => Some(if a.start <= b.start { a } else { b }),
                            (Some(a), None) | (None, Some(a)) => Some(a),
                            (None, None) => None,
                        };
                    let (chosen, intrinsic) = match (a_concrete, b_concrete) {
                        (Some(a), Some(b)) if a.1.start <= b.1.start => (Some(a), a_intrinsic),
                        (Some(_), Some(b)) => (Some(b), b_intrinsic),
                        (Some(a), None) => (Some(a), a_intrinsic),
                        (None, Some(b)) => (Some(b), b_intrinsic),
                        (None, None) => (None, false),
                    };
                    self.graph.type_node_mut(&a).unwrap().concrete = chosen;
                    self.graph.type_node_mut(&a).unwrap().concrete_intrinsic = intrinsic;
                    let failed = self.graph.take_failed_constructor(a)
                        | self.graph.take_failed_constructor(b);
                    if failed {
                        self.graph.mark_failed_constructor(a);
                    }
                    outcome = Ok(SolverValue::Unit);
                }
                Frame::UnifyAfterUnion {
                    request,
                    a,
                    b,
                    a_root,
                    b_root,
                    pending_before,
                } => {
                    if let Err(error) = outcome {
                        if error.code == "NX-E0102"
                            && (self.graph.type_nodes[&a_root].variant_children.is_some()
                                || self.graph.type_nodes[&b_root].variant_children.is_some())
                            && (self.concrete(a).is_none() || self.concrete(b).is_none())
                        {
                            let constructor = self.root(a);
                            self.graph.register(ObligationKind::Join(Box::new(
                                PendingDeferredJoin {
                                    equality: self.graph.active_equality,
                                    left: Type::Deferred(a),
                                    right: Type::Deferred(b),
                                    constructor,
                                    left_origin: request.left_origin,
                                    found: request.found,
                                    recursive_origin: None,
                                    branch: request.policy.branch,
                                    related_expected: request.policy.related_expected,
                                    numeric_domain_owner: self
                                        .graph
                                        .fact(request.fact)
                                        .numeric_domain_owner,
                                    comparison_domain_if_unresolved: None,
                                },
                            )));
                            outcome = Ok(SolverValue::Type(Type::Deferred(constructor)));
                        } else if error.code == "NX-E0103" {
                            let recursive_origin = error
                                .primary_span
                                .clone()
                                .unwrap_or_else(|| request.found.clone());
                            let constructor = self.root(a);
                            let node = self.graph.type_node_mut(&constructor).unwrap();
                            node.deferred_join = true;
                            node.recursive_join = true;
                            self.graph.register(ObligationKind::Join(Box::new(
                                PendingDeferredJoin {
                                    equality: self.graph.active_equality,
                                    left: Type::Deferred(a),
                                    right: Type::Deferred(b),
                                    constructor,
                                    left_origin: request.left_origin,
                                    found: request.found,
                                    recursive_origin: Some(recursive_origin),
                                    branch: request.policy.branch,
                                    related_expected: request.policy.related_expected,
                                    numeric_domain_owner: self
                                        .graph
                                        .fact(request.fact)
                                        .numeric_domain_owner,
                                    comparison_domain_if_unresolved: None,
                                },
                            )));
                            outcome = Ok(SolverValue::Type(Type::Deferred(constructor)));
                        } else {
                            outcome = Err(error);
                        }
                        continue;
                    }
                    for join in self.pending_joins_mut().skip(pending_before) {
                        join.left = Type::Deferred(a);
                        join.right = Type::Deferred(b);
                        join.left_origin = request.left_origin.clone();
                        join.branch = request.policy.branch;
                        join.related_expected = request.policy.related_expected;
                    }
                    outcome = Ok(SolverValue::Type(Type::Deferred(self.root(a))));
                }
                Frame::UnifyAfterBind {
                    request,
                    id,
                    other,
                    left_deferred,
                    pending_before,
                } => {
                    if let Err(error) = outcome {
                        let root = self.root(id);
                        if error.code == "NX-E0102"
                            && concrete_union(&other).is_some()
                            && self.graph.type_nodes[&root].variant_children.is_some()
                            && self.concrete(id).is_none()
                        {
                            self.graph.register(ObligationKind::Join(Box::new(
                                PendingDeferredJoin {
                                    equality: self.graph.active_equality,
                                    left: if left_deferred {
                                        Type::Deferred(id)
                                    } else {
                                        other.clone()
                                    },
                                    right: if left_deferred {
                                        other
                                    } else {
                                        Type::Deferred(id)
                                    },
                                    constructor: root,
                                    left_origin: request.left_origin,
                                    found: request.found,
                                    recursive_origin: None,
                                    branch: request.policy.branch,
                                    related_expected: request.policy.related_expected,
                                    numeric_domain_owner: self
                                        .graph
                                        .fact(request.fact)
                                        .numeric_domain_owner,
                                    comparison_domain_if_unresolved: None,
                                },
                            )));
                            outcome = Ok(SolverValue::Type(Type::Deferred(root)));
                        } else {
                            outcome = Err(error);
                        }
                        continue;
                    }
                    for join in self.pending_joins_mut().skip(pending_before) {
                        join.left = if left_deferred {
                            Type::Deferred(id)
                        } else {
                            other.clone()
                        };
                        join.right = if left_deferred {
                            other.clone()
                        } else {
                            Type::Deferred(id)
                        };
                        join.left_origin = request.left_origin.clone();
                        join.branch = request.policy.branch;
                        join.related_expected = request.policy.related_expected;
                    }
                    let ty = self
                        .concrete(id)
                        .map_or(Type::Deferred(self.root(id)), |(ty, _)| ty);
                    outcome = Ok(SolverValue::Type(ty));
                }
                Frame::UnifyArrayDone {
                    key,
                    before_inner,
                    pending_before,
                    expected,
                    found,
                } => {
                    let element = match outcome {
                        Ok(SolverValue::Type(element)) => element,
                        Ok(_) => unreachable!("array child result"),
                        Err(error) => {
                            outcome = Err(array_mismatch(error, &expected, &found));
                            continue;
                        }
                    };
                    for join in self.pending_joins_mut().skip(pending_before) {
                        join.left = Type::Array(Arc::new(join.left.clone()));
                        join.right = Type::Array(Arc::new(join.right.clone()));
                    }
                    let resolved = Type::Array(Arc::new(element));
                    if internal_state(&expected).is_none() && internal_state(&found).is_none() {
                        self.graph.array_unify_cache.insert(
                            key,
                            (
                                expected,
                                found,
                                resolved.clone(),
                                self.solver_steps - before_inner,
                            ),
                        );
                    }
                    outcome = Ok(SolverValue::Type(resolved));
                }
                Frame::UnifyUnionNext {
                    request,
                    kind,
                    expected,
                    found,
                    mut remaining,
                    mut payloads,
                    waiting,
                } => {
                    if outcome.is_err() {
                        continue;
                    }
                    if waiting {
                        let previous = std::mem::replace(&mut outcome, Ok(SolverValue::Unit));
                        let Ok(SolverValue::Type(ty)) = previous else {
                            unreachable!("union payload result")
                        };
                        payloads.push(ty);
                    }
                    if !remaining.is_empty() {
                        let (left, right) = remaining.remove(0);
                        let child = SolverUnify {
                            fact: request.fact,
                            left,
                            right,
                            left_origin: request.left_origin.clone(),
                            right_origin: request.right_origin.clone(),
                            found: request.found.clone(),
                            policy: UnifyPolicy {
                                branch: request.policy.branch,
                                related_expected: true,
                            },
                        };
                        frames.push(Frame::UnifyUnionNext {
                            request,
                            kind,
                            expected,
                            found,
                            remaining,
                            payloads,
                            waiting: true,
                        });
                        frames.push(Frame::Enter(SolverRequest::Unify(child)));
                    } else {
                        outcome = match (kind, payloads.as_slice()) {
                            (UnionKind::Option, [some]) => {
                                Ok(SolverValue::Type(Type::option(Arc::new(some.clone()))))
                            }
                            (UnionKind::Result, [ok, err]) => Ok(SolverValue::Type(Type::result(
                                Arc::new(ok.clone()),
                                Arc::new(err.clone()),
                            ))),
                            _ => Err(unresolved_type(request.found)),
                        };
                    }
                }
                Frame::UnifyUnionWrap { expected, found } => {
                    if let Err(error) = outcome {
                        outcome = Err(array_mismatch(error, &expected, &found));
                    }
                }
            }
        }
        outcome
    }

    fn type_reaches(
        &mut self,
        start: u64,
        target: u64,
        found: &Span,
    ) -> Result<bool, Box<Diagnostic>> {
        let target = self.root(target);
        let mut work = vec![(self.root(start), false)];
        let mut seen = HashSet::new();
        while let Some((current, traversed)) = work.pop() {
            if traversed {
                self.charge_solver(found)?;
            }
            let current = self.root(current);
            if !seen.insert(current) {
                continue;
            }
            if current == target {
                return Ok(true);
            }
            let array_child = self.graph.type_nodes[&current].array_child;
            let variant_children = self.graph.type_nodes[&current].variant_children;
            if let Some(child) = array_child {
                work.push((child, true));
            }
            if let Some(union) = variant_children {
                for variant in union.variants.into_iter().rev() {
                    if let Some(child) = variant.payload {
                        work.push((child, true));
                    }
                }
            }
        }
        Ok(false)
    }

    fn inferred_array_depth(&mut self, id: u64) -> usize {
        let mut current = id;
        let mut depth = 0;
        loop {
            if let Some((ty, _)) = &self.graph.type_nodes[&current].concrete {
                let mut work = vec![(ty, 0_usize)];
                let mut inner_max = 0;
                while let Some((part, arrays)) = work.pop() {
                    inner_max = inner_max.max(arrays);
                    match part {
                        Type::Array(child) => work.push((child, arrays + 1)),
                        Type::Union(_, first, second) => {
                            work.push((first, arrays));
                            if let Some(second) = second {
                                work.push((second, arrays));
                            }
                        }
                        _ => {}
                    }
                }
                return depth + inner_max;
            }
            let Some(child) = self.graph.type_nodes[&current].array_child else {
                return depth;
            };
            depth += 1;
            if depth > 128 {
                return depth;
            }
            current = self.root(child);
        }
    }

    fn check_inferred_depth(&mut self) -> Result<(), Box<Diagnostic>> {
        let mut constructed = Vec::new();
        for id in 0..self.graph.next_variable {
            if self.graph.type_nodes[&id].parent != id {
                continue;
            }
            let origin = self.graph.type_nodes[&id].array_origin.clone();
            if let Some(origin) = origin {
                self.check_one_inferred_depth(id, &origin)?;
                if let Some((ty, _)) = self.concrete(id) {
                    constructed.push((origin, ty));
                }
            }
        }
        constructed.sort_by_key(|(origin, _)| (origin.start, origin.end));
        for (origin, ty) in constructed {
            self.check_depth(&ty, &origin)?;
        }
        Ok(())
    }

    fn check_one_inferred_depth(&mut self, id: u64, origin: &Span) -> Result<(), Box<Diagnostic>> {
        if self.inferred_array_depth(id) <= 128 {
            return Ok(());
        }
        self.analysis_exhausted = true;
        // §13.13 requires the exact complete source construction whose
        // synthesized type exceeds depth 128. Callers pass that expression
        // or the complete written annotation; the stored array_origin is
        // the outer constructor's source span. The concrete-type origin
        // used for mismatch relations is separate and cannot select this
        // budget's primary span.
        let mut d = names::diagnostic(
            "NX-E0003",
            None,
            Stage::Types,
            Operation::AnalysisLimit,
            origin.clone(),
        );
        d.data.insert(
            "budget_kind".into(),
            DataValue::String("array_type_depth".into()),
        );
        d.data.insert(
            "budget_scope".into(),
            DataValue::String("compilation".into()),
        );
        d.data.insert(
            "incomplete_analysis".into(),
            DataValue::String("types".into()),
        );
        d.data.insert("limit".into(), DataValue::UInt(128));
        // The first rejected wrapper is always limit+1, even if a composed
        // type was attached in one operation with greater final depth.
        d.data.insert("observed".into(), DataValue::UInt(129));
        Err(d)
    }

    fn expression_origin(&mut self, e: &Expr) -> Span {
        let mut e = e;
        while let Expr::Group(inner, _) | Expr::Index { target: inner, .. } = e {
            e = inner;
        }
        match e {
            Expr::Name(_, span) => match self.resolved.uses.get(&span.start) {
                Some(ResolvedUse::Local(local)) => {
                    if let Some(known) = self.graph.locals.get(&local.name_span.start).cloned() {
                        known
                            .variable
                            .and_then(|id| self.concrete(id).map(|(_, origin)| origin))
                            .unwrap_or(known.origin)
                    } else {
                        span.clone()
                    }
                }
                _ => span.clone(),
            },
            _ => Self::primary_span(e),
        }
    }

    fn expression_variable(&self, e: &Expr) -> Option<u64> {
        self.graph.expr_vars.get(&span_key(e.span())).copied()
    }

    fn place_root_binding_site(&self, place: &Expr) -> Option<u64> {
        let (root, _) = parse::place_parts(place)?;
        let Expr::Name(_, span) = root else {
            return None;
        };
        match self.resolved.uses.get(&span.start) {
            Some(ResolvedUse::Local(local)) => Some(local.name_span.start),
            _ => None,
        }
    }

    fn expression_reads_binding(&self, expression: &Expr, site: u64) -> bool {
        self.reads_binding_node(BindingReadNode::Expr(expression), site)
    }

    fn reads_binding_node(&self, node: BindingReadNode<'_>, site: u64) -> bool {
        use BindingReadNode as Node;
        let mut pending = vec![node];
        while let Some(node) = pending.pop() {
            match node {
                Node::Expr(expression) => match expression {
                    Expr::Name(_, span) => {
                        if matches!(self.resolved.uses.get(&span.start), Some(ResolvedUse::Local(local)) if local.name_span.start == site)
                        {
                            return true;
                        }
                    }
                    Expr::Array(items, _) => pending.extend(items.iter().rev().map(Node::Expr)),
                    Expr::Group(inner, _)
                    | Expr::Unary { value: inner, .. }
                    | Expr::Neg { value: inner, .. }
                    | Expr::Not { value: inner, .. } => pending.push(Node::Expr(inner)),
                    Expr::Binary { left, right, .. }
                    | Expr::Index {
                        target: left,
                        index: right,
                        ..
                    } => {
                        pending.push(Node::Expr(right));
                        pending.push(Node::Expr(left));
                    }
                    Expr::Call { target, args, .. } => {
                        pending.extend(args.iter().rev().map(|arg| Node::Expr(&arg.value)));
                        pending.push(Node::Expr(target));
                    }
                    Expr::If {
                        condition,
                        then_block,
                        otherwise,
                        ..
                    } => {
                        match otherwise.as_deref() {
                            Some(Else::Block(block)) => pending.push(Node::Block(block)),
                            Some(Else::If(expr)) => pending.push(Node::Expr(expr)),
                            None => {}
                        }
                        pending.push(Node::Block(then_block));
                        pending.push(Node::Expr(condition));
                    }
                    Expr::Match(selection) => pending.push(Node::Match(selection)),
                    Expr::Integer(..) | Expr::Bool(..) | Expr::Unit(..) | Expr::Closure(..) => {}
                },
                Node::Block(block) => {
                    pending.extend(block.statements.iter().rev().map(Node::Statement))
                }
                Node::Statement(statement) => match statement {
                    Statement::Let { value, .. } | Statement::Expr { value, .. } => {
                        pending.push(Node::Expr(value))
                    }
                    Statement::Assign { place, value, .. } => {
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
                        if let Some(otherwise) = otherwise {
                            pending.push(Node::Statement(otherwise));
                        }
                        pending.push(Node::Block(then_block));
                        pending.push(Node::Expr(condition));
                    }
                    Statement::While {
                        condition, body, ..
                    } => {
                        pending.push(Node::Block(body));
                        pending.push(Node::Expr(condition));
                    }
                    Statement::For { iterable, body, .. } => {
                        pending.push(Node::Block(body));
                        pending.push(Node::Expr(iterable));
                    }
                    Statement::Match(selection) => pending.push(Node::Match(selection)),
                    Statement::Block(block) => pending.push(Node::Block(block)),
                },
                Node::Match(selection) => {
                    for arm in selection.arms.iter().rev() {
                        pending.push(Node::Block(&arm.body));
                        if let Some(guard) = &arm.guard {
                            pending.push(Node::Expr(guard));
                        }
                    }
                    pending.push(Node::Expr(&selection.scrutinee));
                }
            }
        }
        false
    }

    fn assignment_value_variable(&self, e: &Expr) -> Option<u64> {
        // Assignment copies the binding's established type and provenance.
        // Equality instead keeps its operand as a source witness
        // (§§4.1.2.4, 4.1.2.6 and 13.15).
        let mut e = e;
        while let Expr::Group(inner, _) = e {
            e = inner;
        }
        match e {
            Expr::Name(_, span) => match self.resolved.uses.get(&span.start) {
                Some(ResolvedUse::Local(local)) => self
                    .graph
                    .locals
                    .get(&local.name_span.start)
                    .and_then(|known| known.variable),
                _ => None,
            },
            _ => self.expression_variable(e),
        }
    }

    fn binding_variable_at_source(&self, e: &Expr) -> Option<u64> {
        let mut e = e;
        while let Expr::Group(inner, _) = e {
            e = inner;
        }
        match e {
            Expr::Name(_, span) => match self.resolved.uses.get(&span.start) {
                Some(ResolvedUse::Local(local)) => self
                    .graph
                    .locals
                    .get(&local.name_span.start)
                    .and_then(|known| known.variable),
                _ => None,
            },
            _ => None,
        }
    }

    // Grouping has no diagnostic site of its own. Every expression site
    // passed to the solver or a diagnostic constructor uses this span.
    fn primary_span(e: &Expr) -> Span {
        crate::frontend::parse::expression_primary_span(e)
    }

    fn normal_witness_span(&self, expression: &Expr) -> Span {
        let mut expression = expression;
        loop {
            if let Expr::Group(inner, _) = expression {
                expression = inner;
                continue;
            }
            if let Expr::If {
                then_block,
                otherwise,
                ..
            } = expression
            {
                let first = branch_value_expr(then_block);
                let second = match otherwise.as_deref() {
                    Some(Else::Block(block)) => branch_value_expr(block),
                    Some(Else::If(other)) => Some(other.as_ref()),
                    None => None,
                };
                let first_diverges = first.is_some_and(|e| {
                    self.graph
                        .has(&span_key(e.span()), OccurrenceFlag::Divergent)
                });
                let second_diverges = second.is_some_and(|e| {
                    self.graph
                        .has(&span_key(e.span()), OccurrenceFlag::Divergent)
                });
                if first_diverges
                    && !second_diverges
                    && let Some(second) = second
                {
                    expression = second;
                    continue;
                } else if second_diverges
                    && !first_diverges
                    && let Some(first) = first
                {
                    expression = first;
                    continue;
                }
            }
            return Self::primary_span(expression);
        }
    }

    // These expressions have only intrinsic operand constraints. A later
    // Array base can add an I64 context to the selector but cannot change
    // how their children are checked. A value union or constructor can
    // change from a join error to leaf mismatches under that context, so its
    // diagnosis must wait for the base shape (§§4.1.2.3–4, RD107).
    fn context_stable_selector(e: &Expr) -> bool {
        let mut pending = vec![e];
        while let Some(e) = pending.pop() {
            match e {
                Expr::Name(..) | Expr::Integer(..) | Expr::Bool(..) | Expr::Unit(..) => {}
                Expr::Group(inner, _) => pending.push(inner),
                Expr::Binary { left, right, .. } => {
                    pending.push(right);
                    pending.push(left);
                }
                Expr::Array(..)
                | Expr::If { .. }
                | Expr::Index { .. }
                | Expr::Call { .. }
                | Expr::Unary { .. }
                | Expr::Neg { .. }
                | Expr::Not { .. }
                | Expr::Match(..)
                | Expr::Closure(..) => return false,
            }
        }
        true
    }

    fn is_value_if(e: &Expr) -> bool {
        let mut e = e;
        while let Expr::Group(inner, _) = e {
            e = inner;
        }
        matches!(e, Expr::If { .. })
    }

    fn check_depth(&self, ty: &Type, origin: &Span) -> Result<(), Box<Diagnostic>> {
        if self.defer_depth && !self.planning && self.depth_may_contain_generator(ty) {
            // A known array path has priority even when a deferred edge may
            // later reveal a Generator state. Its own first excess is final.
            if matches!(ty, Type::Deferred(_)) && self.compound_depth(ty).0 > 128 {
                return self.check_depth_now(ty, origin);
            }
            self.pending_depth
                .borrow_mut()
                .push((ty.clone(), origin.clone()));
            return Ok(());
        }
        self.check_depth_now(ty, origin)
    }

    fn depth_may_contain_generator(&self, ty: &Type) -> bool {
        let mut pending = vec![ty];
        let mut seen_closures = HashSet::new();
        while let Some(next) = pending.pop() {
            match next {
                Type::Generator { .. } | Type::Deferred(_) => return true,
                Type::Array(child) => pending.push(child),
                Type::Union(_, first, second) => {
                    pending.push(first);
                    if let Some(second) = second {
                        pending.push(second);
                    }
                }
                Type::Closure { site, .. } => {
                    if seen_closures.insert(*site)
                        && let Some(draft) = self.closures.get(site)
                    {
                        pending.extend(draft.captures.iter().filter_map(|capture| {
                            matches!(capture.1, CaptureMode::Copy | CaptureMode::Move)
                                .then_some(&capture.5)
                        }));
                    }
                }
                _ => {}
            }
        }
        false
    }

    fn check_depth_now(&self, ty: &Type, origin: &Span) -> Result<(), Box<Diagnostic>> {
        if has_resource_array(ty) {
            return Err(resource_array(origin.clone()));
        }
        if has_closure_array(ty) {
            let mut error = names::diagnostic(
                "NX-E0006",
                None,
                Stage::Types,
                Operation::TypesCheck,
                origin.clone(),
            );
            error.feature_id = Some("closure".into());
            error.data.insert(
                "pending_scope".into(),
                DataValue::String("later_adr".into()),
            );
            return Err(error);
        }
        let (arrays, values) = self.compound_depth(ty);
        let (kind, observed) = if arrays > 128 {
            ("array_type_depth", arrays)
        } else if values > 128 {
            ("value_type_depth", values)
        } else {
            return Ok(());
        };
        let mut d = names::diagnostic(
            "NX-E0003",
            None,
            Stage::Types,
            Operation::AnalysisLimit,
            origin.clone(),
        );
        d.data
            .insert("budget_kind".into(), DataValue::String(kind.into()));
        d.data.insert("limit".into(), DataValue::UInt(128));
        // The first rejected constructor is the only charge: later nesting
        // cannot advance the observed depth beyond the rejected value.
        d.data.insert(
            "observed".into(),
            DataValue::UInt(if kind == "value_type_depth" {
                129
            } else {
                observed as u64
            }),
        );
        Err(d)
    }

    fn compound_depth(&self, ty: &Type) -> (usize, usize) {
        enum Part<'a> {
            Ty(&'a Type),
            Var(u64),
            Leave(u64),
            LeaveNominal((u8, u64)),
        }
        struct NominalFrame {
            key: (u8, u64),
            prefix_arrays: usize,
            prefix_values: usize,
            max_arrays: usize,
            max_values: usize,
            cacheable: bool,
        }
        impl NominalFrame {
            fn new(key: (u8, u64), arrays: usize, values: usize) -> Self {
                Self {
                    key,
                    prefix_arrays: arrays,
                    prefix_values: values,
                    max_arrays: 0,
                    max_values: 1,
                    cacheable: true,
                }
            }
            fn observe(&mut self, arrays: usize, values: usize) {
                self.max_arrays = self.max_arrays.max(arrays - self.prefix_arrays);
                self.max_values = self.max_values.max(values - self.prefix_values);
            }
        }
        let mut work = vec![(Part::Ty(ty), 0_usize, 0_usize)];
        let mut active = HashSet::new();
        let mut active_nominal = HashSet::new();
        let mut nominal_path: Vec<NominalFrame> = Vec::new();
        let (mut arrays, mut values) = (0, 0);
        while let Some((part, array_depth, value_depth)) = work.pop() {
            if let Part::Leave(root) = part {
                active.remove(&root);
                continue;
            }
            if let Part::LeaveNominal(key) = part {
                active_nominal.remove(&key);
                let frame = nominal_path.pop().unwrap();
                debug_assert_eq!(frame.key, key);
                if self.depth_memo_ready && frame.cacheable {
                    self.graph
                        .nominal_depth_memo
                        .borrow_mut()
                        .insert(key, (frame.max_arrays, frame.max_values));
                }
                continue;
            }
            arrays = arrays.max(array_depth);
            values = values.max(value_depth);
            for frame in &mut nominal_path {
                frame.observe(array_depth, value_depth);
            }
            match part {
                Part::Leave(_) | Part::LeaveNominal(_) => unreachable!(),
                Part::Ty(Type::Array(child)) => {
                    work.push((Part::Ty(child), array_depth + 1, value_depth + 1));
                }
                Part::Ty(Type::Union(_, first, second)) => {
                    work.push((Part::Ty(first), array_depth, value_depth + 1));
                    if let Some(second) = second {
                        work.push((Part::Ty(second), array_depth, value_depth + 1));
                    }
                }
                Part::Ty(Type::Generator { site, item }) => {
                    values = values.max(value_depth + 1);
                    let key = (0, *site);
                    if active_nominal.contains(&key) {
                        for frame in &mut nominal_path {
                            frame.cacheable = false;
                        }
                    } else if let Some((cached_arrays, cached_values)) = self
                        .depth_memo_ready
                        .then(|| self.graph.nominal_depth_memo.borrow().get(&key).copied())
                        .flatten()
                    {
                        let total_arrays = array_depth + cached_arrays;
                        let total_values = value_depth + cached_values;
                        arrays = arrays.max(total_arrays);
                        values = values.max(total_values);
                        for frame in &mut nominal_path {
                            frame.observe(total_arrays, total_values);
                        }
                    } else {
                        active_nominal.insert(key);
                        nominal_path.push(NominalFrame::new(key, array_depth, value_depth));
                        work.push((Part::LeaveNominal(key), array_depth, value_depth));
                        work.push((Part::Ty(item), array_depth, value_depth + 1));
                        if let Some(slots) = self.generator_slots.get(site) {
                            for slot in slots {
                                work.push((Part::Ty(slot), array_depth, value_depth + 1));
                            }
                        }
                    }
                }
                Part::Ty(Type::Closure { site, .. }) => {
                    values = values.max(value_depth + 1);
                    let key = (1, *site);
                    if active_nominal.contains(&key) {
                        for frame in &mut nominal_path {
                            frame.cacheable = false;
                        }
                    } else if let Some((cached_arrays, cached_values)) = self
                        .depth_memo_ready
                        .then(|| self.graph.nominal_depth_memo.borrow().get(&key).copied())
                        .flatten()
                    {
                        let total_arrays = array_depth + cached_arrays;
                        let total_values = value_depth + cached_values;
                        arrays = arrays.max(total_arrays);
                        values = values.max(total_values);
                        for frame in &mut nominal_path {
                            frame.observe(total_arrays, total_values);
                        }
                    } else {
                        active_nominal.insert(key);
                        nominal_path.push(NominalFrame::new(key, array_depth, value_depth));
                        work.push((Part::LeaveNominal(key), array_depth, value_depth));
                        if let Some(draft) = self
                            .saved_closures
                            .get(site)
                            .or_else(|| self.closures.get(site))
                        {
                            for (_, mode, _, _, _, captured, _) in &draft.captures {
                                if matches!(mode, CaptureMode::Copy | CaptureMode::Move) {
                                    work.push((Part::Ty(captured), array_depth, value_depth + 1));
                                }
                            }
                        }
                    }
                }
                Part::Ty(Type::Deferred(id)) => {
                    work.push((Part::Var(*id), array_depth, value_depth));
                }
                Part::Var(id) => {
                    let mut root = id;
                    while let Some(node) = self.graph.type_nodes.get(&root) {
                        if node.parent == root {
                            break;
                        }
                        root = node.parent;
                    }
                    let Some(node) = self.graph.type_nodes.get(&root) else {
                        continue;
                    };
                    // Occurs checks reject cycles while solving. Guard this
                    // walk before expanding a node so malformed graphs never
                    // turn depth accounting into an unbounded traversal.
                    if !active.insert(root) {
                        continue;
                    }
                    work.push((Part::Leave(root), array_depth, value_depth));
                    if let Some((concrete, _)) = &node.concrete {
                        work.push((Part::Ty(concrete), array_depth, value_depth));
                    } else if let Some(child) = node.array_child {
                        work.push((Part::Var(child), array_depth + 1, value_depth + 1));
                    } else if let Some(children) = node.variant_children {
                        for variant in children.variants {
                            if let Some(child) = variant.payload {
                                work.push((Part::Var(child), array_depth, value_depth + 1));
                            }
                        }
                    }
                }
                _ => {}
            }
        }
        (arrays, values)
    }

    fn type_annotation_span(&self, start: u64, end: u64, marker: &str) -> Option<Span> {
        let first_at = self.tokens.partition_point(|t| t.span.start < start);
        // `end` is the start of the initializer/body after the
        // annotation. A marker token beginning exactly there cannot be the
        // annotation separator of this binding.
        let marker_at = (first_at..self.tokens.len())
            .find(|&i| self.tokens[i].span.start < end && self.tokens[i].text == marker)?;
        let first = self.tokens[marker_at + 1..]
            .iter()
            .find(|t| t.kind != Kind::Newline)?;
        let last = self.tokens[marker_at + 1..]
            .iter()
            .take_while(|t| {
                t.span.start < end && t.text != "=" && t.text != "with" && t.text != "{"
            })
            .filter(|t| t.kind != Kind::Newline)
            .last()?;
        Some(Span {
            file: self.file.into(),
            start: first.span.start,
            // The lexer keeps `>=` together even when the parser consumes its
            // `>` as a type close and the `=` as the binding separator.
            end: last.span.end - u64::from(last.text == ">="),
        })
    }

    fn function(&mut self, f: &Function) -> Result<(), Box<Diagnostic>> {
        if self.planning {
            self.current_planning_calls = self.current_planning_calls.saturating_add(1);
        } else {
            self.current_passes = self.current_passes.saturating_add(1);
        }
        if f.is_generator {
            for (parameter, signature) in f
                .params
                .iter()
                .zip(&self.resolved.signatures[&f.name].params)
            {
                if parameter.mode == Mode::Inout
                    || (parameter.mode == Mode::Read
                        && !matches!(
                            parameter.ty,
                            SyntaxType::I64 | SyntaxType::Bool | SyntaxType::Unit
                        ))
                {
                    let mode = if parameter.mode == Mode::Inout {
                        "edit"
                    } else {
                        "view"
                    };
                    let span = self
                        .tokens
                        .iter()
                        .find(|token| {
                            token.span.start >= parameter.span.start
                                && token.span.end <= parameter.span.end
                                && token.text == mode
                        })
                        .map_or_else(|| parameter.span.clone(), |token| token.span.clone());
                    let mut error = names::diagnostic(
                        "NX-E1110",
                        None,
                        Stage::Ownership,
                        Operation::OwnershipCheck,
                        span,
                    );
                    error.related_spans.push(RelatedSpan {
                        role: RelatedRole::Declaration,
                        span: signature.name_span.clone(),
                    });
                    return Err(error);
                }
            }
        }
        // The span sets below may retain old entries without changing a
        // later lookup: source spans are unique across functions. Locals,
        // expr_vars and expression_types must be cleared because the report
        // harvests their entire maps after every function; stale variable
        // IDs would be looked up in this function's fresh type-node table.
        self.graph.locals.clear();
        self.graph.eligible_shapes.clear();
        self.graph.preseeded_children.clear();
        self.graph.projected_bases.clear();
        self.graph.projection_items.clear();
        self.graph.assigned_bases.clear();
        self.graph.nested_assignment_sites.clear();
        self.graph.pre_assignment_snapshot.clear();
        self.graph.assignment_starts.clear();
        self.graph.first_assignment_rhs_starts.clear();
        self.graph.assignment_type_plan.clear();
        self.graph.prefix_concrete.clear();
        self.block_depth = 0;
        self.top_statement_start = 0;
        self.top_statement_end = 0;
        self.active_if_span = None;
        // Node IDs restart at zero below. If this map were retained, newly
        // allocated IDs would overwrite the reachable prefix; stale higher
        // IDs are unreachable. Clearing avoids retaining that stale suffix.
        self.graph.type_nodes.clear();
        // Retaining entries across functions would be semantically equivalent:
        // each entry owns both Arc operands, so no pointer key can be reused.
        // Clear here to release each function's cached trees promptly instead
        // of retaining their sum until the compilation ends.
        self.graph.array_unify_cache.clear();
        self.graph.expr_vars.clear();
        self.graph.expression_types.clear();
        self.closures.clear();
        self.hidden_for.clear();
        self.closure_local_sites.clear();
        self.graph.occurrences.clear();
        self.graph.next_variable = 0;
        // Empty constructors are finalized at this function boundary; unlike
        // the span-keyed maps above, retaining this vector would diagnose a
        // prior function again. §13.9 requires separate function sites.

        // Failed constructor IDs restart at zero. Retaining this set would
        // suppress the next function's independent NX-E0103 (§13.9).
        self.graph.clear_failed_constructors();
        // Normal exits drain this vector with mem::take; a quota exit stops
        // the compilation before another function. Clearing is defensive.
        self.graph.obligations.clear();
        self.graph.emitted_sites.clear();
        // These sets use source offsets, which are unique across function
        // bodies in one file. A later function cannot read an earlier span;
        // their clears bound memory but do not change a reachable result.
        // Binding name offsets have the same per-file uniqueness property.
        self.graph.result = if f.is_generator {
            Type::Unit
        } else {
            f.result.as_ref().map(Type::from).unwrap_or(Type::Unit)
        };
        self.generator_item = f
            .is_generator
            .then(|| f.result.as_ref().map(Type::from).unwrap_or(Type::Unit));
        self.loop_depth = 0;
        self.graph.result_span = f
            .result
            .as_ref()
            .and_then(|_| self.type_annotation_span(f.name_span.end, f.body.span.start, "->"))
            .map(|mut span| {
                if f.is_generator {
                    span.start = span.start.saturating_sub(2);
                }
                span
            });
        if let Some(span) = &self.graph.result_span {
            self.check_depth(&self.graph.result, span)?;
        }
        let sig = &self.resolved.signatures[&f.name];
        for p in &sig.params {
            let ty = Type::from(&p.ty);
            self.check_depth(&ty, &p.type_span)?;
            self.graph.locals.insert(
                p.name_span.start,
                Known {
                    ty,
                    origin: p.type_span.clone(),
                    variable: None,
                    invalid: false,
                    nominal_sources: HashSet::new(),
                },
            );
        }
        self.block(&f.body)?;
        if self.analysis_exhausted {
            return Ok(());
        }
        self.resolve_pending_projections(true)?;
        self.check_inferred_depth()?;
        let pending_constructors = self.pending_constructors();
        let unresolved_nominal_roots: HashSet<_> = pending_constructors
            .into_iter()
            .filter_map(|(child, array, _)| {
                self.concrete(child).is_none().then(|| self.root(array))
            })
            .collect();
        for (id, result, nominal) in self.graph.conditional_bindings() {
            let guards: Vec<_> = self
                .graph
                .obligations
                .iter()
                .filter_map(|entry| match entry {
                    ObligationKind::GuardedComparison(guard) if guard.result == result => {
                        Some((**guard).clone())
                    }
                    _ => None,
                })
                .collect();
            if let [guard] = guards.as_slice() {
                let domain_valid = self.concrete(guard.variable).is_some_and(|(ty, _)| {
                    if guard.domain == "I64" {
                        ty == Type::I64
                    } else {
                        domain_accepts(guard.domain, &ty)
                    }
                });
                if !domain_valid {
                    continue;
                }
                let (Some(expected), Some(found)) = (
                    self.resolved_join_side(guard.expected.clone()),
                    self.resolved_join_side(guard.found.clone()),
                ) else {
                    continue;
                };
                if compare_source_site(
                    &expected,
                    &found,
                    guard.expected_origin.clone(),
                    guard.found_site.clone(),
                    UnifyPolicy {
                        branch: false,
                        related_expected: true,
                    },
                )
                .is_some()
                {
                    continue;
                }
            } else if guards.is_empty() {
                // DomainNominal can depend on another nominal operation (for
                // example `Len(q[x]) + 1`). Resolve the entire chain before
                // granting the binding its executable type. An undecided
                // domain remains usable only without an unresolved
                // constructor obligation (§4.1.2.3).
                let mut site = result.clone();
                let mut seen = HashSet::new();
                let mut usable = false;
                while seen.insert(span_key(&site)) {
                    let Some((variable, domain, dependency)) =
                        self.graph.obligations.iter().find_map(|entry| match entry {
                            ObligationKind::DomainNominal(pending) if pending.call == site => {
                                Some((pending.variable, pending.domain, pending.depends_on.clone()))
                            }
                            _ => None,
                        })
                    else {
                        break;
                    };
                    let domain_valid = self.concrete(variable).is_some_and(|(ty, _)| {
                        if domain == "I64" {
                            ty == Type::I64
                        } else {
                            domain_accepts(domain, &ty)
                        }
                    });
                    if !domain_valid && unresolved_nominal_roots.contains(&self.root(variable)) {
                        break;
                    }
                    if self.concrete(variable).is_some() && !domain_valid {
                        break;
                    }
                    if let Some(dependency) = dependency {
                        site = dependency;
                    } else {
                        usable = true;
                        break;
                    }
                }
                if !usable {
                    continue;
                }
            } else {
                continue;
            }
            let found_site = self
                .concrete(id)
                .map(|(_, origin)| origin)
                .unwrap_or_else(|| result.clone());
            if let Err(error) = self.bind_independent(id, nominal, result, found_site, true) {
                self.record_error(error);
            }
        }
        let mut joins = self.take_pending_kind(|entry| match entry {
            ObligationKind::Join(join) => Ok(join),
            other => Err(other),
        });
        // Source order makes dedup deterministic. Reversing this private
        // traversal cannot change output: resolved sites have distinct keys
        // and are sorted with other diagnostics; unresolved sites share the
        // same constructor primary (§13.9).
        joins.sort_by_key(|join| join.found.start);
        let mut failed_numeric_operations = HashSet::new();
        for join in joins {
            let numeric_owner = join.numeric_domain_owner;
            let comparison_domain_if_unresolved = join.comparison_domain_if_unresolved;
            // IDs are never reused before this drain. Storing the original
            // ID or its current representative is equivalent after root().
            if join.equality.is_some_and(|id| {
                self.graph.equality_records.iter().any(|record| {
                    record.id == id && matches!(record.state, EqualityState::Rejected)
                })
            }) {
                continue;
            }
            let constructor = self.root(join.constructor);
            // A constructor with no determined element has one causal
            // NX-E0103. Once its element is concrete, independent source
            // operands keep their own NX-E0102/NX-E1303 (§13.9).
            let left = self.resolved_join_side(join.left);
            let right = self.resolved_join_side(join.right);
            // A queued conflict belongs to the source operand whether or
            // not both types resolve. Different constructor roots can reach
            // the same operand; another operand keeps its distinct span.
            let error = if let (Some(left), Some(right)) = (left, right) {
                // ADR §4.1.2.1,6: resolving a component later completes the
                // type at this source occurrence; it never relocates the
                // expected witness to the later assignment or initializer.
                let origin = join.left_origin;
                let policy = UnifyPolicy {
                    branch: join.branch,
                    related_expected: join.related_expected,
                };
                let Some(error) = compare_source_site(&left, &right, origin, join.found, policy)
                else {
                    // A queued structural conflict cannot become compatible:
                    // binding completes a constructor without erasing its
                    // incompatible scalar/array shape.
                    continue;
                };
                error
            } else {
                if let Some(domain) = comparison_domain_if_unresolved
                    && let Some(left) = self.resolved_join_side(domain.left)
                    && (if domain.domain == "I64" {
                        left != Type::I64
                    } else {
                        !comparable_data(&left)
                    })
                {
                    self.record_error(mismatch(
                        domain.operator,
                        domain.found,
                        None,
                        domain.domain,
                        &left,
                    ));
                }
                // An enclosing constructor has only a consequent inference
                // obligation when its element is itself an unresolved
                // constructor with its own source site (§4.1.2.2).
                let child = self.graph.type_nodes[&constructor].array_child;
                if let Some(child) = child {
                    let child_root = self.root(child);
                    let own_unresolved_child =
                        self.pending_constructors()
                            .into_iter()
                            .any(|(element, array, _)| {
                                self.root(array) == child_root && self.concrete(element).is_none()
                            });
                    if own_unresolved_child {
                        continue;
                    }
                }
                if self.graph.type_nodes[&constructor]
                    .variant_children
                    .is_some()
                {
                    continue;
                }
                self.unresolved_constructor_diagnostic(
                    constructor,
                    join.recursive_origin.unwrap_or(join.found),
                )
            };
            if error.code == "NX-E0102"
                && numeric_owner.is_some_and(|owner| !failed_numeric_operations.insert(owner))
            {
                continue;
            }
            self.record_error(error);
        }
        // §13.9: an outer constructor whose unresolved element is itself
        // an unresolved constructor has only a consequent obligation. The
        // inner constructor owns the causal NX-E0103. This is decided from
        // the type graph before either diagnostic is emitted.
        let causal_inference_sites: HashSet<(u64, u64)> = self
            .errors
            .iter()
            .filter(|error| error.code == "NX-E0103")
            .filter_map(|error| error.primary_span.as_ref().map(span_key))
            .collect();
        let unresolved_arrays: HashSet<u64> = self
            .pending_constructors()
            .into_iter()
            .filter_map(|(child, array, _)| {
                let root = self.root(array);
                let has_cause = !self.graph.failed_constructor(root)
                    || self.graph.type_nodes[&root]
                        .array_origin
                        .as_ref()
                        .is_some_and(|span| causal_inference_sites.contains(&span_key(span)));
                (has_cause && self.concrete(child).is_none()).then_some(root)
            })
            .collect();
        let shared_inference_roots: HashSet<u64> = self
            .pending_constructors()
            .into_iter()
            .filter_map(|(_, array, span)| {
                causal_inference_sites
                    .contains(&span_key(&span))
                    .then(|| self.root(array))
            })
            .collect();
        for (child, array, span) in self.pending_constructors() {
            let root = self.root(array);
            let child_root = self.root(child);
            let result_child_owns_inference = self.graph.type_nodes[&child_root]
                .variant_children
                .is_some()
                && self
                    .pending_variant_constructors()
                    .into_iter()
                    .any(|(result, _)| self.root(result) == child_root);
            let own_inference_cause = causal_inference_sites.iter().any(|&(start, end)| {
                start <= span.start && span.end <= end && (start, end) != span_key(&span)
            });
            // A failed constructor can be caused at a different source site.
            // Its unresolved literal must not add a dependent inference error.
            if (!self.graph.failed_constructor(root)
                || own_inference_cause
                || shared_inference_roots.contains(&root))
                && self.concrete(child).is_none()
                && !unresolved_arrays.contains(&self.root(child))
                && !result_child_owns_inference
            {
                self.record_error(names::diagnostic(
                    "NX-E0103",
                    None,
                    Stage::Types,
                    Operation::TypesInfer,
                    span,
                ));
            }
        }
        for (result, span) in self.pending_variant_constructors() {
            let array_cause = self.errors.iter().any(|error| {
                error.code == "NX-E0103"
                    && error
                        .primary_span
                        .as_ref()
                        .is_some_and(|site| span.start <= site.start && site.end <= span.end)
            });
            if array_cause {
                continue;
            }
            let mut paths = Vec::new();
            self.unresolved_union_paths(result, "$".into(), 0, &mut paths);
            if !paths.is_empty() {
                let mut diagnostic =
                    names::diagnostic("NX-E0113", None, Stage::Types, Operation::TypesInfer, span);
                diagnostic
                    .data
                    .insert("unresolved_components".into(), DataValue::Array(paths));
                self.record_error(diagnostic);
            }
        }
        for domain in self.take_pending_kind(|entry| match entry {
            ObligationKind::Domain(domain) => Ok(domain),
            other => Err(other),
        }) {
            if let Some((ty, _)) = self.concrete(domain.variable)
                && !domain_accepts(domain.domain, &ty)
            {
                self.record_error(self.domain_diagnostic(domain.operand, &ty, domain.domain));
            }
        }
        let mut failed_results = HashSet::new();
        let mut failed_dependencies = HashSet::new();
        let nominal_obligations = self.take_pending_kind(|entry| match entry {
            ObligationKind::DomainNominal(result) => Ok(result),
            other => Err(other),
        });
        let mut nominal_states = Vec::with_capacity(nominal_obligations.len());
        for pending in &nominal_obligations {
            let domain_state = self.pending_domain_state(pending.variable, pending.domain);
            let error = if domain_state != Some(false) {
                pending
                    .expected
                    .clone()
                    .and_then(|ty| self.resolved_join_side(ty))
                    .and_then(|expected| {
                        compare_source_site(
                            &expected,
                            &pending.nominal,
                            pending
                                .expected_span
                                .clone()
                                .unwrap_or_else(|| pending.call.clone()),
                            pending.call.clone(),
                            UnifyPolicy {
                                branch: false,
                                related_expected: pending.expected_span.is_some(),
                            },
                        )
                    })
            } else {
                None
            };
            if domain_state == Some(false) || error.is_some() {
                failed_dependencies.insert(span_key(&pending.call));
            }
            nominal_states.push((domain_state, error));
        }
        // Dependencies can be discovered in any work-list order. Propagate
        // invalidity to a fixed point before deciding any consumer result.
        loop {
            let mut changed = false;
            for pending in &nominal_obligations {
                if pending
                    .depends_on
                    .as_ref()
                    .is_some_and(|site| failed_dependencies.contains(&span_key(site)))
                {
                    changed |= failed_dependencies.insert(span_key(&pending.call));
                }
            }
            if !changed {
                break;
            }
        }
        for (pending, (domain_state, error)) in nominal_obligations.into_iter().zip(nominal_states)
        {
            let domain_invalid = domain_state == Some(false);
            let dependency_failed = pending
                .depends_on
                .as_ref()
                .is_some_and(|site| failed_dependencies.contains(&span_key(site)));
            if domain_invalid || dependency_failed {
                failed_results.insert(span_key(&pending.call));
            }
            self.graph.transition(
                span_key(&pending.call),
                ObligationClass::DomainNominal,
                if domain_invalid {
                    ObligationStatus::Refuted
                } else if dependency_failed {
                    ObligationStatus::Contaminated
                } else if domain_state == Some(true) {
                    ObligationStatus::Satisfied
                } else {
                    ObligationStatus::Pending
                },
            );
            if !domain_invalid
                && !dependency_failed
                && let Some(error) = error
            {
                self.record_error(error);
            }
        }
        let mut failed_binary_operations = HashSet::new();
        for pending in self.take_pending_kind(|entry| match entry {
            ObligationKind::BinaryDomain(domain) => Ok(domain),
            other => Err(other),
        }) {
            if let Some((ty, _)) = self.concrete(pending.variable) {
                let valid = if pending.domain == "I64" {
                    ty == Type::I64
                } else {
                    comparable_data(&ty)
                };
                if !valid {
                    failed_results.insert(span_key(&pending.result));
                    failed_dependencies.insert(span_key(&pending.result));
                    self.graph.transition(
                        span_key(&pending.result),
                        ObligationClass::BinaryDomain,
                        ObligationStatus::Refuted,
                    );
                    // A numeric or comparison operation owns one domain
                    // failure: the first incompatible operand in source
                    // order (§§4.1.2 N(d), 4.6). Resolving both operands
                    // late must not create an extra diagnostic.
                    if failed_binary_operations.insert(span_key(&pending.result)) {
                        self.record_error(mismatch(
                            pending.operator,
                            pending.found,
                            None,
                            pending.domain,
                            &ty,
                        ));
                    }
                } else {
                    self.graph.transition(
                        span_key(&pending.result),
                        ObligationClass::BinaryDomain,
                        ObligationStatus::Satisfied,
                    );
                }
            }
        }
        let guarded = self.take_pending_kind(|entry| match entry {
            ObligationKind::GuardedComparison(comparison) => Ok(comparison),
            other => Err(other),
        });
        let mut failed_source_sites: Vec<(u64, u64)> = if guarded.is_empty() {
            Vec::new()
        } else {
            self.errors
                .iter()
                .filter_map(|error| error.primary_span.as_ref().map(span_key))
                .collect()
        };
        failed_source_sites.sort_unstable();
        failed_source_sites.dedup();
        let source_site_failed = |operand: &Span| {
            let first = failed_source_sites.partition_point(|&(start, _)| start < operand.start);
            failed_source_sites[first..]
                .iter()
                .take_while(|&&(start, _)| start <= operand.end)
                .any(|&(_, end)| end <= operand.end)
        };
        for pending in guarded {
            let primary_state = self.pending_domain_state(pending.variable, pending.domain);
            let domain_refuted = primary_state == Some(false)
                || pending
                    .additional_domains
                    .iter()
                    .any(|&(variable, domain)| {
                        self.pending_domain_state(variable, domain) == Some(false)
                    });
            let unresolved_guard = pending.require_resolved
                && (primary_state.is_none()
                    || pending
                        .additional_domains
                        .iter()
                        .any(|&(variable, domain)| {
                            self.pending_domain_state(variable, domain).is_none()
                        })
                    || pending.projections.iter().any(|&(base, item)| {
                        match (self.concrete(base), self.concrete(item)) {
                            (Some((Type::Array(element), _)), Some((ty, _))) => *element != ty,
                            _ => true,
                        }
                    })
                    || pending.dependent_results.iter().any(|site| {
                        failed_results.contains(&span_key(site))
                            || self
                                .graph
                                .has(&span_key(site), OccurrenceFlag::FailedProjection)
                    })
                    || source_site_failed(&pending.expected_origin)
                    || source_site_failed(&pending.found_site));
            if domain_refuted
                || unresolved_guard
                || unresolved_arrays.contains(&self.root(pending.variable))
            {
                self.graph.transition(
                    span_key(&pending.result),
                    ObligationClass::GuardedComparison,
                    ObligationStatus::Contaminated,
                );
                if let Some(error) = pending.domain_error_if_equal {
                    self.record_error(error);
                }
                continue;
            }
            // §4.1.2.3 has three domain states. An undecided predicate does
            // not taint the operation's independently known nominal result;
            // only a refuted predicate withdraws that evidence. Keep the
            // consumer comparison until all compatible constraints have run.
            if !failed_results.contains(&span_key(&pending.expected_origin))
                && !failed_results.contains(&span_key(&pending.found_site))
                && let (Some(expected), Some(found)) = (
                    self.resolved_join_side(pending.expected),
                    self.resolved_join_side(pending.found),
                )
            {
                if let Some(error) = compare_source_site(
                    &expected,
                    &found,
                    pending.expected_origin,
                    pending.found_site,
                    UnifyPolicy {
                        branch: false,
                        related_expected: true,
                    },
                ) {
                    failed_results.insert(span_key(&pending.result));
                    self.graph.transition(
                        span_key(&pending.result),
                        ObligationClass::GuardedComparison,
                        ObligationStatus::Refuted,
                    );
                    self.record_error(error);
                } else if let Some(error) = pending.domain_error_if_equal {
                    failed_results.insert(span_key(&pending.result));
                    self.graph.transition(
                        span_key(&pending.result),
                        ObligationClass::GuardedComparison,
                        ObligationStatus::Refuted,
                    );
                    self.record_error(error);
                }
            }
            if !failed_results.contains(&span_key(&pending.result)) {
                self.graph.transition(
                    span_key(&pending.result),
                    ObligationClass::GuardedComparison,
                    ObligationStatus::Satisfied,
                );
            }
            if !failed_results.contains(&span_key(&pending.result))
                && let Some(consumer_expected) = pending
                    .consumer_expected
                    .and_then(|ty| self.resolved_join_side(ty))
                && let Some(error) = compare_source_site(
                    &consumer_expected,
                    &Type::Bool,
                    pending
                        .consumer_origin
                        .clone()
                        .unwrap_or_else(|| pending.result.clone()),
                    pending.result.clone(),
                    UnifyPolicy {
                        branch: false,
                        related_expected: pending.consumer_origin.is_some(),
                    },
                )
            {
                self.record_error(error);
            }
        }
        // ADR §4.1.2.5: a failed domain invalidates the nominal result of
        // its operation. A consumer whose expected witness is that result,
        // or which checks the result itself, has no independent evidence.
        let refuted_results = self.graph.refuted_result_sites();
        self.suppress_result_dependents(&refuted_results);
        Ok(())
    }

    #[inline(never)]
    fn frame_occurrence_id(&self, expression: &Expr) -> OccurrenceId {
        if let Some(id) = self.occurrences.get(
            OccurrenceRole::Expression,
            std::ptr::from_ref(expression).addr(),
        ) {
            return id;
        }
        #[cfg(test)]
        {
            // Private unit tests can drive an isolated expression directly; production checks
            // always use the function prepass above.
            OccurrenceId {
                function_ordinal: self.function_ordinal,
                preorder: u32::MAX,
                role: OccurrenceRole::Expression,
                local_index: std::ptr::from_ref(expression).addr() as u32,
            }
        }
        #[cfg(not(test))]
        panic!(
            "expression frame has no prepassed occurrence: {:?} (function {})",
            expression.span(),
            self.function_ordinal
        );
    }

    fn block(&mut self, b: &Block) -> Result<(), Box<Diagnostic>> {
        self.drive(DriveStart::Block(b)).0.map(|_| ())
    }

    #[cfg(test)]
    fn statement_at_site(&mut self, statement: &Statement) {
        let _ = self.drive(DriveStart::StatementSite(statement));
    }

    fn finish_statement_at_site(
        &mut self,
        visit: FactId,
        statement: &Statement,
        previous_taint: bool,
        outcome: Result<(), Box<Diagnostic>>,
    ) {
        let tainted = self.graph.fact_mut(visit).tainted
            || matches!(statement, Statement::Let { value, .. }
                if self.graph.has(&span_key(value.span()), OccurrenceFlag::Tainted));
        self.graph.fact_mut(visit).tainted = previous_taint || tainted;
        if let Statement::Let {
            name,
            annotation: None,
            value,
            span,
            ..
        } = statement
            && tainted
            && outcome.is_ok()
            && let Some(nominal) = self
                .graph
                .conditional_result(&span_key(value.span()))
                .cloned()
            && self.pending_operation_guard(value).is_some()
        {
            let site = Self::primary_span(value);
            match self.new_variable(&site) {
                Ok(id) => {
                    let name_span =
                        names::token_span(&self.tokens, span.start, value.span().start, name)
                            .unwrap_or_else(|| span.clone());
                    self.graph.locals.insert(
                        name_span.start,
                        Known {
                            ty: Type::Deferred(id),
                            origin: site.clone(),
                            variable: Some(id),
                            invalid: false,
                            nominal_sources: self
                                .graph
                                .occurrences
                                .get(&span_key(value.span()))
                                .map(|state| state.nominal_sources.clone())
                                .unwrap_or_default(),
                        },
                    );
                    self.graph.attach_conditional_binding(&site, id, nominal);
                    self.graph.fact_mut(visit).tainted = previous_taint;
                    return;
                }
                Err(error) => self.record_error(error),
            }
        }
        if let Statement::Let {
            name,
            annotation,
            value,
            span,
            ..
        } = statement
            && (tainted || outcome.is_err())
        {
            let name_span = names::token_span(&self.tokens, span.start, value.span().start, name)
                .unwrap_or_else(|| span.clone());
            // this origin is retained for the invalid binding's internal
            // record only. Name uses the binding's invalid state before reading its type,
            // and the inferred table reads its type but never its origin.
            // Thus changing this fallback span is unobservable once the
            // binding is marked invalid below.
            let origin = self
                .type_annotation_span(span.start, value.span().start, ":")
                .unwrap_or_else(|| Self::primary_span(value));
            self.graph.locals.insert(
                name_span.start,
                Known {
                    ty: annotation.as_ref().map(Type::from).unwrap_or(Type::Tainted),
                    origin,
                    variable: None,
                    invalid: true,
                    nominal_sources: HashSet::new(),
                },
            );
        }
        if let Err(error) = outcome {
            self.record_error(error);
        }
    }

    fn prepare_for_type(
        &mut self,
        for_span: &Span,
        name_span: &Span,
        iterable: &Expr,
        got: Type,
    ) -> Result<(), Box<Diagnostic>> {
        let got = self
            .resolved_syntax_type(&got)
            .map(|ty| Type::from(&ty))
            .unwrap_or(got);
        if self.generator_item.is_some() && matches!(got, Type::Generator { .. }) {
            self.hidden_for.push(got.clone());
        }
        let item = match got {
            Type::Generator { item, .. } => (*item).clone(),
            Type::Array(_) => {
                if self.f5 {
                    return Err(super::pending_f5::diagnostic(
                        "L06",
                        Span {
                            file: for_span.file.clone(),
                            start: for_span.start,
                            end: for_span.start + 3,
                        },
                    ));
                }
                let mut error = names::diagnostic(
                    "NX-E0006",
                    None,
                    Stage::Types,
                    Operation::TypesCheck,
                    Self::primary_span(iterable),
                );
                error.feature_id = Some("for_iteration".into());
                error.data.insert(
                    "pending_scope".into(),
                    DataValue::String("later_adr".into()),
                );
                return Err(error);
            }
            other if pretty(&other).is_none() => Type::Tainted,
            other => {
                return Err(mismatch(
                    Self::primary_span(iterable),
                    Self::primary_span(iterable),
                    None,
                    "domain:iterable",
                    &other,
                ));
            }
        };
        self.graph.locals.insert(
            name_span.start,
            Known {
                ty: item,
                origin: name_span.clone(),
                variable: None,
                invalid: false,
                nominal_sources: HashSet::new(),
            },
        );
        Ok(())
    }

    fn prepare_assignment_plan(&mut self, place: &Expr, value: &Expr) {
        if self.planning
            && let Expr::Name(_, span) = place
            && let Some(ResolvedUse::Local(local)) = self.resolved.uses.get(&span.start)
        {
            if self.graph.eligible_shapes.contains(&local.name_span.start)
                && !self
                    .graph
                    .pre_assignment_snapshot
                    .contains_key(&local.name_span.start)
                && let Some(id) = self
                    .graph
                    .locals
                    .get(&local.name_span.start)
                    .and_then(|known| known.variable)
            {
                let concrete = self.concrete_evidence(id);
                self.graph
                    .pre_assignment_snapshot
                    .insert(local.name_span.start, concrete);
            }
            self.graph.assigned_bases.insert(local.name_span.start);
            self.graph
                .assignment_starts
                .entry(local.name_span.start)
                .or_insert(span.start);
            self.graph
                .first_assignment_rhs_starts
                .entry(local.name_span.start)
                .or_insert(Self::primary_span(value).start);
            if let Some(ty @ Type::Array(_)) = self.intrinsic_assignment_type(value) {
                self.graph
                    .assignment_type_plan
                    .entry(local.name_span.start)
                    .or_insert((ty, Self::primary_span(value)));
            }
            if !self
                .graph
                .prefix_concrete
                .contains_key(&local.name_span.start)
                && let Some(id) = self
                    .graph
                    .locals
                    .get(&local.name_span.start)
                    .and_then(|known| known.variable)
                && let Some(concrete) = self.concrete_evidence(id)
            {
                self.graph
                    .prefix_concrete
                    .insert(local.name_span.start, concrete);
            }
        }
        if self.planning
            && matches!(place, Expr::Index { .. })
            && let Some(site) = self.place_root_binding_site(place)
            && self.graph.eligible_shapes.contains(&site)
            && self.expression_reads_binding(value, site)
        {
            self.graph.nested_assignment_sites.insert(site);
        }
    }

    fn prepare_assignment_rhs<'a>(
        &mut self,
        visit: FactId,
        place: &'a Expr,
        value: &'a Expr,
        lhs: Type,
    ) -> (OwnedCheckContext, AssignmentProgress<'a>) {
        let local = self.expression_variable(place).or_else(|| {
            if let Expr::Name(_, span) = place {
                match self.resolved.uses.get(&span.start) {
                    Some(ResolvedUse::Local(local)) => self
                        .graph
                        .locals
                        .get(&local.name_span.start)
                        .and_then(|known| known.variable),
                    _ => None,
                }
            } else {
                None
            }
        });
        let (context, concrete, expected) = if let Some(local) = local {
            let intrinsic_origin = self.origin_intrinsic(local);
            let concrete = self.concrete(local);
            let projected = if concrete.is_none() && self.expression_has_array_root(value) {
                self.projection_expected(local)
            } else {
                None
            };
            let expected = concrete.clone().or(projected.clone());
            let root = self.root(local);
            let nested_shape = self.graph.type_nodes[&root]
                .array_child
                .map(|child| self.root(child))
                .is_some_and(|child| self.graph.type_nodes[&child].array_child.is_some());
            let shape_context = if expected.is_none()
                && matches!(place, Expr::Name(..))
                && self.expression_has_array_root(value)
                && nested_shape
            {
                Some(Type::Deferred(local))
            } else {
                None
            };
            let expected_span = expected
                .as_ref()
                .filter(|_| projected.is_some() || !intrinsic_origin)
                .filter(|(_, origin)| origin.start < Self::primary_span(place).start)
                .map(|(_, origin)| origin.clone());
            (
                OwnedCheckContext {
                    expected: expected
                        .as_ref()
                        .map(|(ty, _)| ty.clone())
                        .or(shape_context),
                    related_expected: expected_span.is_some(),
                    expected_span,
                    check_here: true,
                    anchor: None,
                    outer_compare: false,
                    deferred_element: false,
                },
                concrete,
                expected,
            )
        } else {
            let origin = self.expression_origin(place);
            (
                OwnedCheckContext {
                    expected: (internal_state(&lhs) != Some(Type::Tainted)).then_some(lhs),
                    expected_span: Some(origin),
                    check_here: true,
                    anchor: None,
                    outer_compare: false,
                    related_expected: true,
                    deferred_element: false,
                },
                None,
                None,
            )
        };
        (
            context,
            AssignmentProgress {
                place,
                value,
                local,
                concrete,
                expected,
                parent_fact: visit,
            },
        )
    }

    fn finish_assignment_rhs(
        &mut self,
        visit: FactId,
        progress: AssignmentProgress<'_>,
        result: Result<Type, Box<Diagnostic>>,
    ) -> Result<(), Box<Diagnostic>> {
        (|| {
            let Some(local) = progress.local else {
                result?;
                return Ok(());
            };
            let rhs = match result {
                Ok(rhs) => rhs,
                Err(error) => {
                    if error.feature_id.as_deref() == Some("resource_array") {
                        let root = self.root(local);
                        if self.graph.type_nodes[&root].array_constructed {
                            self.graph.mark_failed_constructor(root);
                        }
                    }
                    return Err(error);
                }
            };
            if self.graph.fact_mut(visit).tainted {
                return Ok(());
            }
            let place = progress.place;
            let value = progress.value;
            let left_origin = progress
                .expected
                .as_ref()
                .filter(|(_, origin)| origin.start < Self::primary_span(place).start)
                .map(|(_, origin)| origin.clone())
                .unwrap_or_else(|| {
                    if matches!(place, Expr::Index { .. }) {
                        Self::primary_span(value)
                    } else {
                        Self::primary_span(place)
                    }
                });
            let right = self
                .assignment_value_variable(value)
                .map(Type::Deferred)
                .unwrap_or(rhs);
            let right_origin = self.expression_origin(value);
            self.unify(
                visit,
                Type::Deferred(local),
                right,
                left_origin,
                right_origin.clone(),
                Self::primary_span(value),
                false,
            )?;
            if progress.concrete.is_none()
                && matches!(place, Expr::Name(..))
                && matches!(self.concrete(local), Some((Type::Array(_), _)))
            {
                self.retain_assignment_origin(local, &right_origin);
            }
            Ok(())
        })()
    }

    fn prepare_let(
        &mut self,
        annotation: Option<&SyntaxType>,
        value: &Expr,
        span: &Span,
    ) -> Result<PreparedLet, Box<Diagnostic>> {
        let annotation_span =
            annotation.and_then(|_| self.type_annotation_span(span.start, value.span().start, ":"));
        if annotation.is_some() {
            self.charge_solver(
                annotation_span
                    .as_ref()
                    .unwrap_or(&Self::primary_span(value)),
            )?;
        }
        Ok(PreparedLet {
            annotation_span,
            written: annotation.map(Type::from),
        })
    }

    fn finish_let(
        &mut self,
        visit: FactId,
        statement: &Statement,
        prepared: PreparedLet,
        got: Result<Type, Box<Diagnostic>>,
    ) -> Result<(), Box<Diagnostic>> {
        let Statement::Let {
            name,
            annotation,
            value,
            span,
            ..
        } = statement
        else {
            unreachable!("let continuation owns a let statement")
        };
        let got = got?;
        let ns = names::token_span(&self.tokens, span.start, value.span().start, name)
            .unwrap_or_else(|| span.clone());
        let origin = prepared
            .annotation_span
            .unwrap_or_else(|| self.expression_origin(value));
        self.check_depth(&got, &origin)?;
        let deferred = annotation.is_none()
            && self
                .graph
                .has(&span_key(value.span()), OccurrenceFlag::Deferred)
            && internal_state(&got).is_some()
            && !self.graph.fact_mut(visit).tainted;
        let variable = if deferred {
            let id = self.new_variable(&Self::primary_span(value))?;
            self.graph.type_node_mut(&id).unwrap().bottom_only = self
                .graph
                .has(&span_key(value.span()), OccurrenceFlag::Divergent);
            if let Some(from) = self.expression_variable(value) {
                self.unify(
                    visit,
                    Type::Deferred(id),
                    Type::Deferred(from),
                    Self::primary_span(value),
                    Self::primary_span(value),
                    Self::primary_span(value),
                    false,
                )?;
            }
            Some(id)
        } else {
            None
        };
        let nominal_sources = self
            .graph
            .occurrences
            .get(&span_key(value.span()))
            .map(|state| state.nominal_sources.clone())
            .unwrap_or_default();
        self.graph.locals.insert(
            ns.start,
            Known {
                ty: if let Some(id) = variable {
                    Type::Deferred(id)
                } else {
                    prepared.written.unwrap_or(got)
                },
                origin,
                variable,
                invalid: false,
                nominal_sources,
            },
        );
        if let Some(id) = variable {
            if self.planning {
                let root = self.root(id);
                if self.graph.type_nodes[&root].array_child.is_some()
                    && !self.graph.type_nodes[&root].bottom_only
                {
                    self.graph.eligible_shapes.insert(ns.start);
                }
            }
            self.seed_planned_shape(ns.start, id);
        }
        Ok(())
    }

    fn statement_without_expression(
        &mut self,
        statement: &Statement,
    ) -> Result<(), Box<Diagnostic>> {
        match statement {
            Statement::Return { value, span } => {
                if value.is_some() && self.generator_item.is_some() {
                    let primary = Span {
                        file: self.file.into(),
                        start: span.start,
                        end: span.start + 6,
                    };
                    let mut error = names::diagnostic(
                        "NX-E1324",
                        None,
                        Stage::Types,
                        Operation::TypesCheck,
                        primary,
                    );
                    if let Some(expected) = &self.graph.result_span {
                        error.related_spans.push(RelatedSpan {
                            role: RelatedRole::ExpectedType,
                            span: expected.clone(),
                        });
                    }
                    return Err(error);
                }
                debug_assert!(value.is_none());
                if matches!(self.graph.result, Type::Deferred(_)) {
                    let site = Span {
                        file: self.file.into(),
                        start: span.start,
                        end: span.start + 6,
                    };
                    self.unify_independent(
                        self.graph.result.clone(),
                        Type::Unit,
                        site.clone(),
                        site.clone(),
                        site,
                        false,
                    )?;
                } else if self.graph.result != Type::Unit {
                    let primary = self
                        .tokens
                        .iter()
                        .find(|t| t.span.start == span.start && t.text == "return")
                        .map(|t| t.span.clone())
                        .unwrap_or_else(|| Span {
                            file: self.file.into(),
                            start: span.start,
                            end: span.start + 6,
                        });
                    let result_label = pretty(&self.graph.result)
                        .ok_or_else(|| unresolved_type(primary.clone()))?;
                    let mut diagnostic = mismatch(
                        primary.clone(),
                        Span {
                            file: self.file.into(),
                            start: span.start,
                            end: span.start + 6,
                        },
                        self.graph.result_span.clone(),
                        &result_label,
                        &Type::Unit,
                    );
                    diagnostic
                        .related_spans
                        .retain(|span| span.role != RelatedRole::FoundType);
                    let next = self
                        .tokens
                        .iter()
                        .find(|token| token.span.start >= primary.end);
                    if let Some(newline) = next.filter(|token| token.kind == Kind::Newline) {
                        diagnostic.related_spans.push(RelatedSpan {
                            role: RelatedRole::Delimiter,
                            span: newline.span.clone(),
                        });
                    }
                    return Err(diagnostic);
                }
                Ok(())
            }
            Statement::GuardControl {
                token,
                token_span,
                value,
                guarded,
                ..
            } => {
                if matches!(token.as_str(), "break" | "continue") && !guarded {
                    if self.loop_depth == 0 {
                        return Err(names::diagnostic(
                            "NX-E1326",
                            Some(if token == "break" {
                                "break_outside_loop"
                            } else {
                                "continue_outside_loop"
                            }),
                            Stage::Types,
                            Operation::TypesCheck,
                            token_span.clone(),
                        ));
                    }
                    return Ok(());
                }
                if token == "yield" && !guarded {
                    if let Some(item) = self.generator_item.clone() {
                        debug_assert!(value.is_none());
                        if item != Type::Unit {
                            return Err(mismatch(
                                token_span.clone(),
                                token_span.clone(),
                                self.graph.result_span.clone(),
                                &pretty(&item).unwrap_or_default(),
                                &Type::Unit,
                            ));
                        }
                        return Ok(());
                    }
                    return Err(names::diagnostic(
                        "NX-E1213",
                        None,
                        Stage::Types,
                        Operation::TypesCheck,
                        token_span.clone(),
                    ));
                }
                debug_assert!(value.is_none());
                Ok(())
            }
            Statement::Value { value: None, .. } => Ok(()),
            _ => unreachable!("statement without expression must be a leaf"),
        }
    }

    #[cfg(test)]
    fn expr(
        &mut self,
        e: &Expr,
        expected: Option<&Type>,
        expected_span: Option<&Span>,
    ) -> Result<Type, Box<Diagnostic>> {
        self.check_against(e, expected, expected_span, true)
    }

    fn expr_with_visit_result(
        &mut self,
        e: &Expr,
        expected: Option<&Type>,
        expected_span: Option<&Span>,
    ) -> (Result<Type, Box<Diagnostic>>, bool) {
        self.drive(DriveStart::Expression(
            e,
            OwnedCheckContext::new(CheckContext {
                expected,
                expected_span,
                check_here: true,
                anchor: None,
                outer_compare: false,
                related_expected: expected_span.is_some(),
                deferred_element: false,
            }),
        ))
    }

    // A concrete context is checked at every value position. Transparent
    // syntax and constructors pass that context to their children; only a
    // leaf (or a constructor with an incompatible outer shape) compares its
    // own type. This keeps each §13.9 diagnostic at its §13.15 operand.
    #[cfg(test)]
    fn check_against(
        &mut self,
        e: &Expr,
        expected: Option<&Type>,
        expected_span: Option<&Span>,
        check_here: bool,
    ) -> Result<Type, Box<Diagnostic>> {
        self.check_against_context(
            e,
            CheckContext {
                expected,
                expected_span,
                check_here,
                anchor: None,
                outer_compare: false,
                related_expected: expected_span.is_some(),
                deferred_element: false,
            },
        )
    }

    #[cfg(test)]
    fn check_against_context(
        &mut self,
        e: &Expr,
        context: CheckContext<'_>,
    ) -> Result<Type, Box<Diagnostic>> {
        self.drive(DriveStart::Expression(e, OwnedCheckContext::new(context)))
            .0
    }

    fn drive(&mut self, start: DriveStart<'_>) -> (Result<Type, Box<Diagnostic>>, bool) {
        enum Task<'a> {
            StatementSite(&'a Statement),
            Statement(&'a Statement),
            Block(&'a Block),
        }
        struct LogicalRightProgress<'a> {
            expr: &'a Expr,
            context: OwnedCheckContext,
            left: Result<Type, Box<Diagnostic>>,
        }
        enum Frame<'a> {
            StatementSite(&'a Statement, FactId),
            StatementExpr,
            StatementLet(&'a Statement, Box<PreparedLet>),
            StatementReturn(&'a Expr, Type),
            AssignmentPlace(&'a Statement),
            AssignmentValue(Box<AssignmentProgress<'a>>),
            StatementIf(&'a Statement, u8),
            StatementWhile(&'a Statement, u8),
            StatementFor(&'a Statement, u8),
            StatementMatchScrutinee(&'a parse::Match),
            StatementMatch {
                selection: &'a parse::Match,
                selected: Type,
                selected_origin: Span,
                next: usize,
                stage: u8,
            },
            Block(&'a Block, usize),
            Group(Box<GroupFrame<'a>>),
            Unary(Box<(&'a Expr, OwnedCheckContext)>),
            UnaryPropagated(Box<(&'a Expr, OwnedCheckContext)>),
            Array(Box<ArrayFrameState<'a>>),
            Index(Box<(&'a Expr, OwnedCheckContext, FactId)>),
            IndexSelector(Box<IndexSelectorProgress<'a>>),
            Binary(Box<BinaryFrame<'a>>),
            BinaryRight(Box<BinaryRightProgress<'a>>),
            BinaryDeferredRight(Box<BinaryDeferredProgress<'a>>),
            BinaryIndependentLeftError(Box<BinaryLeftErrorProgress<'a>>),
            BinaryIndependentRight(Box<BinaryIndependentProgress<'a>>),
            LogicalLeft(Box<(&'a Expr, OwnedCheckContext)>),
            LogicalRight(Box<LogicalRightProgress<'a>>),
            Call(Box<CallProgress<'a>>),
            CallTarget(Box<(&'a Expr, OwnedCheckContext, FactId)>),
            ClosureCall(Box<ClosureCallFrameState<'a>>),
            UnionCall(Box<UnionCallProgress<'a>>),
            IntrinsicOne(Box<(&'a Expr, OwnedCheckContext, FactId)>),
            TryAdd(Box<TryAddFrameState<'a>>),
            AssertEq(Box<AssertEqFrameState<'a>>),
            MatchScrutinee(Box<(&'a Expr, OwnedCheckContext)>),
            Match {
                expr: &'a Expr,
                progress: Box<MatchProgress<'a>>,
            },
            If(Box<IfProgress<'a>>),
            Closure {
                expr: &'a Expr,
                progress: Box<ClosureProgress<'a>>,
            },
        }
        // Keep non-selection continuations in a separate frame. The live
        // selection_type walk must not inherit their debug stack slots.
        enum Resume {
            Next(Result<Type, Box<Diagnostic>>, FactId),
            Evaluate(FactId),
        }
        struct Entry<'a> {
            frame: Frame<'a>,
            child: &'a Expr,
            context: OwnedCheckContext,
            visit: FactId,
        }
        #[inline(never)]
        fn enter_assignment<'a>(
            checker: &mut Checker<'_>,
            statement: &'a Statement,
            visit: FactId,
        ) -> Entry<'a> {
            let Statement::Assign {
                place, op, value, ..
            } = statement
            else {
                unreachable!("assignment entry owns an assignment")
            };
            checker.prepare_assignment_plan(place, value);
            Entry {
                frame: Frame::AssignmentPlace(statement),
                child: place,
                context: OwnedCheckContext {
                    expected: compound_operand_type(*op),
                    expected_span: None,
                    check_here: true,
                    anchor: None,
                    outer_compare: false,
                    related_expected: false,
                    deferred_element: false,
                },
                visit,
            }
        }
        #[inline(never)]
        fn enter_neg_not<'a>(
            expr: &'a Expr,
            context: OwnedCheckContext,
            expected: Type,
            visit: FactId,
        ) -> Entry<'a> {
            let child = match expr {
                Expr::Neg { value, .. } | Expr::Not { value, .. } => value.as_ref(),
                _ => unreachable!("unary entry owns negation or not"),
            };
            Entry {
                frame: Frame::Unary(Box::new((expr, context))),
                child,
                context: OwnedCheckContext {
                    expected: Some(expected),
                    expected_span: None,
                    check_here: true,
                    anchor: None,
                    outer_compare: false,
                    related_expected: false,
                    deferred_element: false,
                },
                visit,
            }
        }
        #[inline(never)]
        fn enter_binary<'a>(
            checker: &mut Checker<'_>,
            visit: FactId,
            expr: &'a Expr,
            context: OwnedCheckContext,
        ) -> Entry<'a> {
            let Expr::Binary { op, left, .. } = expr else {
                unreachable!("binary entry owns a binary expression")
            };
            if checker.f5 && matches!(op.as_str(), "and" | "or") {
                return Entry {
                    frame: Frame::LogicalLeft(Box::new((expr, context))),
                    child: left,
                    context: OwnedCheckContext {
                        expected: Some(Type::Bool),
                        expected_span: None,
                        check_here: true,
                        anchor: None,
                        outer_compare: false,
                        related_expected: false,
                        deferred_element: false,
                    },
                    visit,
                };
            }
            let pending_before = checker.pending_join_count();
            let numeric = super::operators::is_arithmetic(op, checker.f5);
            let operator_site = span_key(expr.span());
            let frame = Frame::Binary(Box::new((expr, context, pending_before, visit)));
            let child_visit = checker.graph.child_fact(
                visit,
                operator_site,
                FactRole::Operator,
                FactInheritance::Inherit,
                FactInheritance::Empty,
            );
            if numeric {
                checker.graph.fact_mut(child_visit).numeric_domain_owner = Some(operator_site);
            }
            Entry {
                frame,
                child: left,
                context: OwnedCheckContext {
                    expected: numeric.then_some(Type::I64),
                    expected_span: None,
                    check_here: true,
                    anchor: None,
                    outer_compare: false,
                    related_expected: false,
                    deferred_element: false,
                },
                visit: child_visit,
            }
        }
        #[inline(never)]
        fn for_range_error(
            checker: &Checker<'_>,
            statement: &Statement,
        ) -> Option<Box<Diagnostic>> {
            let Statement::For { iterable, span, .. } = statement else {
                unreachable!("for range entry owns a for statement")
            };
            (checker.f5 && super::pending_f5::range_for(iterable)).then(|| {
                super::pending_f5::diagnostic(
                    "L05",
                    Span {
                        file: span.file.clone(),
                        start: span.start,
                        end: span.start + 3,
                    },
                )
            })
        }
        #[inline(never)]
        fn index_range_error(checker: &Checker<'_>, expr: &Expr) -> Option<Box<Diagnostic>> {
            let Expr::Index {
                index, selector, ..
            } = expr
            else {
                unreachable!("index range entry owns an index")
            };
            (checker.f5 && super::pending_f5::range_for(index))
                .then(|| super::pending_f5::range_outside_for(selector.clone()))
        }
        #[inline(never)]
        fn binary_range_error(checker: &Checker<'_>, expr: &Expr) -> Option<Box<Diagnostic>> {
            let Expr::Binary {
                op, left, right, ..
            } = expr
            else {
                unreachable!("binary range entry owns a binary expression")
            };
            (checker.f5 && matches!(op.as_str(), "..<" | "..=")).then(|| {
                super::pending_f5::range_outside_for(checker.operator_span(
                    left.span(),
                    right.span(),
                    op,
                ))
            })
        }
        #[inline(never)]
        #[allow(clippy::too_many_arguments)]
        fn resume_statement_match<'a>(
            checker: &mut Checker<'_>,
            visit: FactId,
            frame: Frame<'a>,
            result: Result<Type, Box<Diagnostic>>,
            frames: &mut Vec<Frame<'a>>,
            inner: &mut &'a Expr,
            inner_context: &mut OwnedCheckContext,
            pending_task: &mut Option<Task<'a>>,
        ) -> Resume {
            match frame {
                Frame::StatementMatchScrutinee(selection) => {
                    let inferred = match result {
                        Ok(inferred) => inferred,
                        Err(error) => return Resume::Next(Err(error), visit),
                    };
                    if let Err(error) = checker.require_match_pattern_shapes(selection, &inferred) {
                        return Resume::Next(Err(error), visit);
                    }
                    frames.push(Frame::StatementMatch {
                        selection,
                        selected: checker.selection_type(&inferred, 0),
                        selected_origin: Checker::primary_span(&selection.scrutinee),
                        next: 0,
                        stage: 0,
                    });
                    Resume::Next(Ok(Type::Unit), visit)
                }
                Frame::StatementMatch {
                    selection,
                    mut selected,
                    selected_origin,
                    mut next,
                    stage,
                } => {
                    if stage == 2 && result.is_err() {
                        return Resume::Next(result, visit);
                    }
                    if stage == 1 {
                        if let Err(error) = result {
                            if error.operation == Operation::AnalysisLimit {
                                return Resume::Next(Err(error), visit);
                            }
                            checker.record_error(error);
                        }
                        let arm = &selection.arms[next];
                        next += 1;
                        *pending_task = Some(Task::Block(&arm.body));
                        frames.push(Frame::StatementMatch {
                            selection,
                            selected,
                            selected_origin,
                            next,
                            stage: 2,
                        });
                        return Resume::Evaluate(visit);
                    }
                    if next == selection.arms.len() {
                        return Resume::Next(
                            checker
                                .finish_selection(selection, &selected, &None, false, None)
                                .map(|_| Type::Unit),
                            visit,
                        );
                    }
                    let arm = &selection.arms[next];
                    selected = checker.selection_type(&selected, 0);
                    if let Err(error) =
                        checker.pattern_type(&arm.pattern, &selected, &selected_origin)
                    {
                        return Resume::Next(Err(error), visit);
                    }
                    if let Some(guard) = &arm.guard {
                        *inner = guard;
                        *inner_context = OwnedCheckContext {
                            expected: Some(Type::Bool),
                            expected_span: arm.guard_span.clone(),
                            check_here: true,
                            anchor: None,
                            outer_compare: false,
                            related_expected: arm.guard_span.is_some(),
                            deferred_element: false,
                        };
                        frames.push(Frame::StatementMatch {
                            selection,
                            selected,
                            selected_origin,
                            next,
                            stage: 1,
                        });
                        return Resume::Evaluate(visit);
                    }
                    next += 1;
                    *pending_task = Some(Task::Block(&arm.body));
                    frames.push(Frame::StatementMatch {
                        selection,
                        selected,
                        selected_origin,
                        next,
                        stage: 2,
                    });
                    Resume::Evaluate(visit)
                }
                _ => unreachable!("statement selection frame"),
            }
        }
        #[inline(never)]
        #[allow(clippy::too_many_arguments)]
        fn resume_expression_match<'a>(
            checker: &mut Checker<'_>,
            mut visit: FactId,
            frame: Frame<'a>,
            result: Result<Type, Box<Diagnostic>>,
            frames: &mut Vec<Frame<'a>>,
            inner: &mut &'a Expr,
            inner_context: &mut OwnedCheckContext,
            pending_task: &mut Option<Task<'a>>,
        ) -> Resume {
            let (expr, mut progress, prior) = match frame {
                Frame::MatchScrutinee(boxed) => {
                    let (expr, context) = *boxed;
                    let Expr::Match(selection) = expr else {
                        unreachable!("match scrutinee frame owns a match")
                    };
                    let inferred = match result {
                        Err(error) => {
                            return Resume::Next(
                                checker.check_against_context_one(
                                    visit,
                                    expr,
                                    context.borrowed(),
                                    Some(PrecomputedType::MatchComplete(Err(error))),
                                ),
                                visit,
                            );
                        }
                        Ok(inferred) => inferred,
                    };
                    if let Err(error) = checker.require_match_pattern_shapes(selection, &inferred) {
                        return Resume::Next(
                            checker.check_against_context_one(
                                visit,
                                expr,
                                context.borrowed(),
                                Some(PrecomputedType::MatchComplete(Err(error))),
                            ),
                            visit,
                        );
                    }
                    (
                        expr,
                        Box::new(MatchProgress {
                            selection,
                            context,
                            selected: checker.selection_type(&inferred, 0),
                            selected_origin: Checker::primary_span(&selection.scrutinee),
                            joined: None,
                            arm_index: 0,
                            branch: None,
                            stage: MatchStage::Arm,
                        }),
                        None,
                    )
                }
                Frame::Match { expr, progress } => (expr, progress, Some(result)),
                _ => unreachable!("expression selection frame"),
            };
            match checker.match_advance(&mut visit, &mut progress, prior) {
                MatchAction::Statement(statement) => {
                    frames.push(Frame::Match { expr, progress });
                    *pending_task = Some(Task::StatementSite(statement));
                    Resume::Evaluate(visit)
                }
                MatchAction::Value(value, context) => {
                    let branch_value = matches!(progress.stage, MatchStage::Value { .. });
                    frames.push(Frame::Match { expr, progress });
                    *inner = value;
                    *inner_context = context;
                    if branch_value {
                        visit = checker.graph.child_fact(
                            visit,
                            span_key(value.span()),
                            FactRole::Expression,
                            FactInheritance::Empty,
                            FactInheritance::Inherit,
                        );
                    }
                    Resume::Evaluate(visit)
                }
                MatchAction::Done(outcome) => Resume::Next(
                    checker.check_against_context_one(
                        visit,
                        expr,
                        progress.context.borrowed(),
                        Some(PrecomputedType::MatchComplete(outcome)),
                    ),
                    visit,
                ),
            }
        }
        #[inline(never)]
        #[allow(clippy::too_many_arguments)] // The selector stays on the drive's frame stack.
        fn index_failed_base<'a>(
            checker: &mut Checker<'_>,
            expr: &'a Expr,
            context: OwnedCheckContext,
            error: Box<Diagnostic>,
            visit: FactId,
            frames: &mut Vec<Frame<'a>>,
            inner: &mut &'a Expr,
            inner_context: &mut OwnedCheckContext,
        ) -> Resume {
            let Expr::Index { index, .. } = expr else {
                unreachable!("index frame owns an index")
            };
            frames.push(Frame::IndexSelector(Box::new(IndexSelectorProgress {
                expr,
                context,
                base: Err(error),
                parent_fact: visit,
                deferred_child: None,
            })));
            *inner = index;
            *inner_context = OwnedCheckContext {
                expected: None,
                expected_span: None,
                check_here: true,
                anchor: None,
                outer_compare: false,
                related_expected: false,
                deferred_element: false,
            };
            let selector_fact = checker.graph.child_fact(
                visit,
                span_key(index.span()),
                FactRole::Selector,
                FactInheritance::Empty,
                FactInheritance::Inherit,
            );
            Resume::Evaluate(selector_fact)
        }
        #[inline(never)]
        #[allow(clippy::boxed_local)] // Unbox only in the small helper, never in resume_non_match.
        fn finish_index_selector(
            checker: &mut Checker<'_>,
            progress: Box<IndexSelectorProgress<'_>>,
            selector_fact: FactId,
            selector: Result<Type, Box<Diagnostic>>,
        ) -> (Result<Type, Box<Diagnostic>>, FactId) {
            let IndexSelectorProgress {
                expr,
                context,
                base,
                parent_fact,
                deferred_child,
            } = *progress;
            checker.graph.merge_child(parent_fact, selector_fact, true);
            let result = checker.check_against_context_one(
                parent_fact,
                expr,
                context.borrowed(),
                Some(PrecomputedType::IndexTarget {
                    target: base,
                    selector: Some(selector),
                    deferred_child,
                }),
            );
            (result, parent_fact)
        }
        #[inline(never)]
        #[allow(clippy::too_many_arguments)] // Continuation carries one drive's cursor.
        fn resume_non_match<'a>(
            checker: &mut Checker<'_>,
            mut visit: FactId,
            frame: Frame<'a>,
            mut result: Result<Type, Box<Diagnostic>>,
            frames: &mut Vec<Frame<'a>>,
            inner: &mut &'a Expr,
            inner_context: &mut OwnedCheckContext,
            pending_task: &mut Option<Task<'a>>,
        ) -> Resume {
            #[cfg(test)]
            let boundary_frame = match &frame {
                Frame::Array(_) => Some("Array"),
                Frame::Index(_) => Some("Index"),
                Frame::IndexSelector(_) => Some("IndexSelector"),
                Frame::Binary(_) => Some("Binary"),
                Frame::BinaryRight(_) => Some("BinaryRight"),
                Frame::BinaryDeferredRight(_) => Some("BinaryDeferredRight"),
                Frame::BinaryIndependentLeftError(_) => Some("BinaryIndependentLeftError"),
                Frame::BinaryIndependentRight(_) => Some("BinaryIndependentRight"),
                Frame::Call(_) => Some("Call"),
                Frame::CallTarget(_) => Some("CallTarget"),
                Frame::ClosureCall(_) => Some("ClosureCall"),
                Frame::UnionCall(_) => Some("UnionCall"),
                Frame::IntrinsicOne(_) => Some("IntrinsicOne"),
                Frame::TryAdd(_) => Some("TryAdd"),
                Frame::AssertEq(_) => Some("AssertEq"),
                _ => None,
            };
            match frame {
                Frame::StatementSite(statement, parent_fact) => {
                    let prior_taint = checker.graph.fact(parent_fact).tainted;
                    checker.finish_statement_at_site(
                        visit,
                        statement,
                        prior_taint,
                        result.map(|_| ()),
                    );
                    checker.graph.fact_mut(parent_fact).tainted = checker.graph.fact(visit).tainted;
                    checker.graph.merge_child(parent_fact, visit, false);
                    visit = parent_fact;
                    result = Ok(Type::Unit);
                }
                Frame::StatementExpr => {
                    result = result.map(|_| Type::Unit);
                }
                Frame::StatementLet(statement, prepared) => {
                    result = checker
                        .finish_let(visit, statement, *prepared, result)
                        .map(|_| Type::Unit);
                }
                Frame::StatementReturn(value, expected) => {
                    result = result.and_then(|actual| {
                        if matches!(expected, Type::Deferred(_))
                            && !checker.graph.fact_mut(visit).tainted
                        {
                            let site = Checker::primary_span(value);
                            checker.unify(
                                visit,
                                expected,
                                actual,
                                site.clone(),
                                site.clone(),
                                site,
                                false,
                            )?;
                        }
                        Ok(Type::Unit)
                    });
                }
                Frame::AssignmentPlace(statement) => {
                    let lhs = match result {
                        Ok(lhs) => lhs,
                        Err(error) => {
                            result = Err(error);
                            return Resume::Next(result, visit);
                        }
                    };
                    let Statement::Assign {
                        place, op, value, ..
                    } = statement
                    else {
                        unreachable!("assignment frame owns an assignment")
                    };
                    let (mut context, progress) =
                        checker.prepare_assignment_rhs(visit, place, value, lhs);
                    if let Some(required) = compound_operand_type(*op) {
                        context.expected = Some(required);
                        context.expected_span = None;
                        context.check_here = true;
                        context.related_expected = false;
                    }
                    frames.push(Frame::AssignmentValue(Box::new(progress)));
                    visit = checker.graph.child_fact(
                        visit,
                        span_key(value.span()),
                        FactRole::Expression,
                        FactInheritance::Empty,
                        FactInheritance::Inherit,
                    );
                    *inner = value;
                    *inner_context = context;
                    return Resume::Evaluate(visit);
                }
                Frame::AssignmentValue(progress) => {
                    let parent_fact = progress.parent_fact;
                    result = checker
                        .finish_assignment_rhs(visit, *progress, result)
                        .map(|_| Type::Unit);
                    checker.graph.merge_child(parent_fact, visit, true);
                    visit = parent_fact;
                }
                Frame::StatementIf(statement, stage) => {
                    if stage == 0 {
                        match result {
                            Err(error) if error.operation == Operation::AnalysisLimit => {
                                result = Err(error);
                                return Resume::Next(result, visit);
                            }
                            Err(error) => {
                                checker.record_error(error);
                                checker.graph.fact_mut(visit).tainted = true;
                            }
                            Ok(_) => {}
                        }
                        let Statement::If { then_block, .. } = statement else {
                            unreachable!("if frame owns an if statement")
                        };
                        frames.push(Frame::StatementIf(statement, 1));
                        *pending_task = Some(Task::Block(then_block));
                        return Resume::Evaluate(visit);
                    }
                    if stage == 1
                        && result.is_ok()
                        && let Statement::If {
                            otherwise: Some(other),
                            ..
                        } = statement
                    {
                        frames.push(Frame::StatementIf(statement, 2));
                        *pending_task = Some(Task::Statement(other));
                        return Resume::Evaluate(visit);
                    }
                }
                Frame::StatementWhile(statement, stage) => {
                    if stage == 0 {
                        match result {
                            Err(error) if error.operation == Operation::AnalysisLimit => {
                                result = Err(error);
                                return Resume::Next(result, visit);
                            }
                            Err(error) => {
                                checker.record_error(error);
                                checker.graph.fact_mut(visit).tainted = true;
                            }
                            Ok(_) => {}
                        }
                        let Statement::While { body, .. } = statement else {
                            unreachable!("while frame owns a while statement")
                        };
                        checker.loop_depth += 1;
                        frames.push(Frame::StatementWhile(statement, 1));
                        *pending_task = Some(Task::Block(body));
                        return Resume::Evaluate(visit);
                    }
                    checker.loop_depth -= 1;
                }
                Frame::StatementFor(statement, stage) => {
                    if stage == 0 {
                        let Statement::For {
                            name_span,
                            iterable,
                            body,
                            span: for_span,
                            ..
                        } = statement
                        else {
                            unreachable!("for frame owns a for statement")
                        };
                        result = result.and_then(|got| {
                            checker
                                .prepare_for_type(for_span, name_span, iterable, got)
                                .map(|_| Type::Unit)
                        });
                        if result.is_err() {
                            return Resume::Next(result, visit);
                        }
                        checker.loop_depth += 1;
                        frames.push(Frame::StatementFor(statement, 1));
                        *pending_task = Some(Task::Block(body));
                        return Resume::Evaluate(visit);
                    }
                    checker.loop_depth -= 1;
                }
                Frame::Block(block, next) => {
                    if result.is_err() {
                        checker.block_depth -= 1;
                        return Resume::Next(result, visit);
                    }
                    if !checker.analysis_exhausted
                        && let Err(error) = checker.resolve_pending_projections(false)
                    {
                        checker.block_depth -= 1;
                        result = Err(error);
                        return Resume::Next(result, visit);
                    }
                    if let Some(statement) = block.statements.get(next) {
                        if checker.block_depth == 1 {
                            let span = parse::statement_span(statement);
                            checker.top_statement_start = span.start;
                            checker.top_statement_end = span.end;
                        }
                        frames.push(Frame::Block(block, next + 1));
                        *pending_task = Some(Task::StatementSite(statement));
                        return Resume::Evaluate(visit);
                    }
                    checker.block_depth -= 1;
                    result = Ok(Type::Unit);
                }
                Frame::Group(progress) => {
                    let (group, child, span, context) = *progress;
                    checker.propagate_group(child, span);
                    result = checker.check_against_context_one(
                        visit,
                        group,
                        context.borrowed(),
                        Some(PrecomputedType::Whole(result)),
                    );
                }
                Frame::Unary(progress) => {
                    let (expr, context) = *progress;
                    result = checker.check_against_context_one(
                        visit,
                        expr,
                        context.borrowed(),
                        Some(PrecomputedType::UnaryOperand(result)),
                    );
                }
                Frame::UnaryPropagated(progress) => {
                    let (expr, context) = *progress;
                    result = checker.check_against_context_one(
                        visit,
                        expr,
                        context.borrowed(),
                        Some(PrecomputedType::Whole(result)),
                    );
                }
                Frame::LogicalLeft(progress) => {
                    let (expr, context) = *progress;
                    let Expr::Binary { right, .. } = expr else {
                        unreachable!("logical frame owns a binary expression")
                    };
                    frames.push(Frame::LogicalRight(Box::new(LogicalRightProgress {
                        expr,
                        context,
                        left: result,
                    })));
                    *inner = right;
                    *inner_context = OwnedCheckContext::new(CheckContext {
                        expected: Some(&Type::Bool),
                        expected_span: None,
                        check_here: true,
                        anchor: None,
                        outer_compare: false,
                        related_expected: false,
                        deferred_element: false,
                    });
                    return Resume::Evaluate(visit);
                }
                Frame::LogicalRight(progress) => {
                    let LogicalRightProgress {
                        expr,
                        context,
                        left,
                    } = *progress;
                    let combined = match (left, result) {
                        (Err(error), _) | (_, Err(error)) => Err(error),
                        (Ok(Type::Tainted), _) | (_, Ok(Type::Tainted)) => Ok(Type::Tainted),
                        (Ok(_), Ok(_)) => Ok(Type::Bool),
                    };
                    result = checker.check_against_context_one(
                        visit,
                        expr,
                        context.borrowed(),
                        Some(PrecomputedType::Whole(combined)),
                    );
                }
                Frame::Array(progress) => {
                    let mut state = *progress;
                    let accepted = checker.array_accept(visit, &mut state, result);
                    let parent_fact = state.parent_fact;
                    checker.graph.merge_child(parent_fact, visit, true);
                    visit = parent_fact;
                    if accepted.is_ok() {
                        state.next += 1;
                        state.consumer.slot = state.next as u32;
                        if state.next < state.items.len() {
                            (*inner, *inner_context) = checker.array_next(visit, &mut state);
                            frames.push(Frame::Array(Box::new(state)));
                            visit = checker.graph.child_fact(
                                visit,
                                span_key(inner.span()),
                                FactRole::Expression,
                                FactInheritance::Empty,
                                FactInheritance::Empty,
                            );
                            return Resume::Evaluate(visit);
                        }
                    }
                    let expr = state.expr;
                    let context = state.context.clone();
                    let constructor_pair = state.element_expected.is_some();
                    let outcome = accepted.and_then(|_| checker.array_finish(state));
                    result = checker.check_against_context_one(
                        visit,
                        expr,
                        context.borrowed(),
                        Some(PrecomputedType::ArrayComplete {
                            result: outcome,
                            constructor_pair,
                        }),
                    );
                }
                Frame::Index(progress) => {
                    let (expr, context, parent_fact) = *progress;
                    checker.graph.merge_child(parent_fact, visit, true);
                    visit = parent_fact;
                    match result {
                        Ok(base) => {
                            let Expr::Index { target, index, .. } = expr else {
                                unreachable!("index frame owns an index")
                            };
                            let deferred_target = checker
                                .graph
                                .has(&span_key(target.span()), OccurrenceFlag::Deferred);
                            let mut deferred_child = None;
                            let mut pending_projection = false;
                            if deferred_target
                                && !matches!(base, Type::Tainted | Type::Never)
                                && let Some(id) = checker.expression_variable(target)
                            {
                                let root = checker.root(id);
                                if checker.graph.type_nodes[&root].array_child.is_some() {
                                    match checker.require_array(id, &Checker::primary_span(target))
                                    {
                                        Ok(child) => deferred_child = Some(child),
                                        Err(error) => {
                                            result = checker.check_against_context_one(
                                                visit,
                                                expr,
                                                context.borrowed(),
                                                Some(PrecomputedType::Whole(Err(error))),
                                            );
                                            return Resume::Next(result, visit);
                                        }
                                    }
                                } else {
                                    pending_projection = true;
                                }
                            }
                            if pending_projection {
                                result = checker.check_against_context_one(
                                    visit,
                                    expr,
                                    context.borrowed(),
                                    Some(PrecomputedType::IndexTarget {
                                        target: Ok(base),
                                        selector: None,
                                        deferred_child: None,
                                    }),
                                );
                                return Resume::Next(result, visit);
                            }
                            let divergent_target = deferred_target
                                && checker
                                    .graph
                                    .has(&span_key(target.span()), OccurrenceFlag::Divergent);
                            let expected =
                                matches!(base, Type::Array(_) | Type::Tainted | Type::Never)
                                    || deferred_child.is_some()
                                    || divergent_target;
                            let separate_taint = !matches!(base, Type::Array(_) | Type::Never)
                                && deferred_child.is_none()
                                && !divergent_target;
                            frames.push(Frame::IndexSelector(Box::new(IndexSelectorProgress {
                                expr,
                                context,
                                base: Ok(base),
                                parent_fact: visit,
                                deferred_child,
                            })));
                            *inner = index;
                            *inner_context = OwnedCheckContext {
                                expected: expected.then_some(Type::I64),
                                expected_span: None,
                                check_here: true,
                                anchor: None,
                                outer_compare: false,
                                related_expected: false,
                                deferred_element: false,
                            };
                            visit = checker.graph.child_fact(
                                visit,
                                span_key(inner.span()),
                                FactRole::Selector,
                                if separate_taint {
                                    FactInheritance::Empty
                                } else {
                                    FactInheritance::Inherit
                                },
                                FactInheritance::Inherit,
                            );
                            return Resume::Evaluate(visit);
                        }
                        Err(error) if error.operation != Operation::AnalysisLimit => {
                            return index_failed_base(
                                checker,
                                expr,
                                context,
                                error,
                                visit,
                                frames,
                                inner,
                                inner_context,
                            );
                        }
                        other => {
                            result = checker.check_against_context_one(
                                visit,
                                expr,
                                context.borrowed(),
                                Some(PrecomputedType::IndexTarget {
                                    target: other,
                                    selector: None,
                                    deferred_child: None,
                                }),
                            );
                        }
                    }
                }
                Frame::IndexSelector(progress) => {
                    (result, visit) = finish_index_selector(checker, progress, visit, result);
                }
                Frame::Binary(progress) => {
                    let (expr, context, pending_before, outer_fact) = *progress;
                    checker.graph.fact_mut(visit).numeric_domain_owner = None;
                    if matches!(&result, Err(error) if error.operation != Operation::AnalysisLimit)
                    {
                        let Err(error) = result else { unreachable!() };
                        let Expr::Binary {
                            op, left, right, ..
                        } = expr
                        else {
                            unreachable!("binary frame owns a binary")
                        };
                        let numeric = super::operators::is_arithmetic(op, checker.f5);
                        let left_domain_failed = numeric
                            && error.primary_span.as_ref() == Some(&Checker::primary_span(left))
                            && error.data.get("expected_type")
                                == Some(&DataValue::String("I64".into()));
                        if numeric {
                            checker.graph.fact_mut(visit).numeric_domain_owner =
                                Some((left.span().start, right.span().end));
                        }
                        frames.push(Frame::BinaryIndependentLeftError(Box::new(
                            BinaryLeftErrorProgress {
                                expr,
                                context,
                                error,
                                left_domain_failed,
                                binary_fact: visit,
                            },
                        )));
                        *inner = right;
                        *inner_context = OwnedCheckContext {
                            expected: (numeric && !left_domain_failed).then_some(Type::I64),
                            expected_span: None,
                            check_here: true,
                            anchor: None,
                            outer_compare: false,
                            related_expected: false,
                            deferred_element: false,
                        };
                        visit = checker.graph.child_fact(
                            visit,
                            span_key(inner.span()),
                            FactRole::Expression,
                            FactInheritance::Empty,
                            FactInheritance::Inherit,
                        );
                        return Resume::Evaluate(visit);
                    }
                    if let (
                        Expr::Binary {
                            op, left, right, ..
                        },
                        Ok(left_type),
                    ) = (expr, &result)
                    {
                        let numeric = super::operators::is_arithmetic(op, checker.f5);
                        let pending_domain = checker.graph.has(
                            &span_key(&Checker::primary_span(left)),
                            OccurrenceFlag::PendingDomain,
                        );
                        let joined = checker.pending_join_count() > pending_before;
                        let failed_join = checker
                            .graph
                            .has(&span_key(left.span()), OccurrenceFlag::FailedJoin);
                        let left_domain_failed = if pending_domain {
                            Some(
                                numeric
                                    && checker.type_determined(left_type)
                                    && *left_type != Type::I64
                                    && !failed_join,
                            )
                        } else if numeric && joined {
                            Some(checker.type_determined(left_type) && *left_type != Type::I64)
                        } else if failed_join || checker.graph.fact_mut(visit).tainted {
                            Some(
                                checker
                                    .graph
                                    .has(&span_key(left.span()), OccurrenceFlag::ContextRejected),
                            )
                        } else if numeric
                            && *left_type != Type::I64
                            && !checker
                                .graph
                                .has(&span_key(left.span()), OccurrenceFlag::Deferred)
                        {
                            let pending_left_result =
                                checker.graph.obligations.iter().any(|entry| match entry {
                                    ObligationKind::BinaryDomain(domain) => {
                                        domain.result == Checker::primary_span(left)
                                    }
                                    _ => false,
                                });
                            Some(!pending_left_result)
                        } else {
                            None
                        };
                        if let Some(left_domain_failed) = left_domain_failed {
                            if numeric {
                                checker.graph.fact_mut(visit).numeric_domain_owner =
                                    Some((left.span().start, right.span().end));
                            }
                            frames.push(Frame::BinaryIndependentRight(Box::new(
                                BinaryIndependentProgress {
                                    expr,
                                    context,
                                    left: left_type.clone(),
                                    pending_before,
                                    left_domain_failed,
                                    binary_fact: visit,
                                },
                            )));
                            *inner = right;
                            *inner_context = OwnedCheckContext {
                                expected: (numeric && !left_domain_failed).then_some(Type::I64),
                                expected_span: None,
                                check_here: true,
                                anchor: None,
                                outer_compare: false,
                                related_expected: false,
                                deferred_element: false,
                            };
                            visit = checker.graph.child_fact(
                                visit,
                                span_key(inner.span()),
                                FactRole::Expression,
                                FactInheritance::Empty,
                                FactInheritance::Inherit,
                            );
                            return Resume::Evaluate(visit);
                        }
                    }
                    if let (
                        Expr::Binary {
                            op, left, right, ..
                        },
                        Ok(left_type),
                    ) = (expr, &result)
                        && checker
                            .graph
                            .has(&span_key(left.span()), OccurrenceFlag::Deferred)
                    {
                        let numeric = super::operators::is_arithmetic(op, checker.f5);
                        let pending_before_operator = checker.pending_join_count();
                        frames.push(Frame::BinaryDeferredRight(Box::new(
                            BinaryDeferredProgress {
                                expr,
                                context,
                                left: left_type.clone(),
                                pending_before_left: pending_before,
                                pending_before_operator,
                                binary_fact: visit,
                            },
                        )));
                        *inner = right;
                        *inner_context = OwnedCheckContext {
                            expected: numeric.then_some(Type::I64),
                            expected_span: None,
                            check_here: true,
                            anchor: None,
                            outer_compare: false,
                            related_expected: false,
                            deferred_element: false,
                        };
                        visit = checker.graph.child_fact(
                            visit,
                            span_key(inner.span()),
                            FactRole::Expression,
                            FactInheritance::Inherit,
                            FactInheritance::Empty,
                        );
                        return Resume::Evaluate(visit);
                    }
                    if let (
                        Expr::Binary {
                            op, left, right, ..
                        },
                        Ok(left_type),
                    ) = (expr, &result)
                    {
                        let numeric = super::operators::is_arithmetic(op, checker.f5);
                        if !checker.graph.has(
                            &span_key(&Checker::primary_span(left)),
                            OccurrenceFlag::PendingDomain,
                        ) && !(numeric && checker.pending_join_count() > pending_before)
                            && !checker
                                .graph
                                .has(&span_key(left.span()), OccurrenceFlag::FailedJoin)
                            && !checker.graph.fact_mut(visit).tainted
                            && !checker
                                .graph
                                .has(&span_key(left.span()), OccurrenceFlag::Deferred)
                            && (!numeric || *left_type == Type::I64)
                        {
                            let left_type = left_type.clone();
                            let right_context = OwnedCheckContext {
                                expected: Some(if numeric {
                                    Type::I64
                                } else {
                                    left_type.clone()
                                }),
                                expected_span: Some(Checker::primary_span(left)),
                                check_here: true,
                                anchor: None,
                                outer_compare: !numeric,
                                related_expected: !numeric,
                                deferred_element: false,
                            };
                            let pending_before = checker.pending_join_count();
                            frames.push(Frame::BinaryRight(Box::new(BinaryRightProgress {
                                expr,
                                context,
                                left: left_type,
                                pending_before,
                                binary_fact: visit,
                            })));
                            *inner = right;
                            *inner_context = right_context;
                            visit = checker.graph.child_fact(
                                visit,
                                span_key(inner.span()),
                                FactRole::Expression,
                                FactInheritance::Inherit,
                                FactInheritance::Empty,
                            );
                            return Resume::Evaluate(visit);
                        }
                    }
                    result = checker.check_against_context_one(
                        visit,
                        expr,
                        context.borrowed(),
                        Some(PrecomputedType::BinaryLeft {
                            result,
                            pending_before,
                            independent_right: None,
                        }),
                    );
                    checker.graph.merge_child(outer_fact, visit, true);
                    visit = outer_fact;
                }
                Frame::BinaryRight(progress) => {
                    let BinaryRightProgress {
                        expr,
                        context,
                        left,
                        pending_before,
                        binary_fact,
                    } = *progress;
                    checker.graph.merge_child(binary_fact, visit, true);
                    visit = binary_fact;
                    checker.graph.fact_mut(visit).numeric_domain_owner = None;
                    result = checker.check_against_context_one(
                        visit,
                        expr,
                        context.borrowed(),
                        Some(PrecomputedType::BinaryRight {
                            left,
                            result,
                            pending_before,
                        }),
                    );
                    visit = checker.graph.merge_to_parent(visit, true);
                }
                Frame::BinaryDeferredRight(progress) => {
                    let BinaryDeferredProgress {
                        expr,
                        context,
                        left,
                        pending_before_left,
                        pending_before_operator,
                        binary_fact,
                    } = *progress;
                    checker.graph.merge_child(binary_fact, visit, true);
                    visit = binary_fact;
                    checker.graph.fact_mut(visit).numeric_domain_owner = None;
                    result = checker.check_against_context_one(
                        visit,
                        expr,
                        context.borrowed(),
                        Some(PrecomputedType::BinaryDeferredRight {
                            left,
                            result,
                            pending_before_left,
                            pending_before_operator,
                        }),
                    );
                    visit = checker.graph.merge_to_parent(visit, true);
                }
                Frame::BinaryIndependentLeftError(progress) => {
                    let BinaryLeftErrorProgress {
                        expr,
                        context,
                        error,
                        left_domain_failed,
                        binary_fact,
                    } = *progress;
                    let Expr::Binary {
                        op, left, right, ..
                    } = expr
                    else {
                        unreachable!("independent frame owns a binary")
                    };
                    let right_tainted = checker.graph.fact_mut(visit).tainted;
                    checker.graph.merge_child(binary_fact, visit, true);
                    visit = binary_fact;
                    checker.graph.fact_mut(visit).numeric_domain_owner = None;
                    let outcome = checker.finish_independent_binary_rhs(
                        op,
                        left,
                        right,
                        left_domain_failed,
                        result,
                        right_tainted,
                    );
                    let binary_result = match outcome {
                        Ok(right_failed) => {
                            if right_failed {
                                checker.record_error(error);
                                checker
                                    .graph
                                    .mark(span_key(expr.span()), OccurrenceFlag::Deferred);
                                checker.graph.fact_mut(visit).tainted = true;
                                Ok(Type::Tainted)
                            } else {
                                Err(error)
                            }
                        }
                        Err(error) => Err(error),
                    };
                    result = checker.check_against_context_one(
                        visit,
                        expr,
                        context.borrowed(),
                        Some(PrecomputedType::Whole(binary_result)),
                    );
                    visit = checker.graph.merge_to_parent(visit, true);
                }
                Frame::BinaryIndependentRight(progress) => {
                    let BinaryIndependentProgress {
                        expr,
                        context,
                        left,
                        pending_before,
                        left_domain_failed,
                        binary_fact,
                    } = *progress;
                    let Expr::Binary {
                        op,
                        left: left_expr,
                        right,
                        ..
                    } = expr
                    else {
                        unreachable!("independent frame owns a binary")
                    };
                    let right_tainted = checker.graph.fact_mut(visit).tainted;
                    checker.graph.merge_child(binary_fact, visit, true);
                    visit = binary_fact;
                    checker.graph.fact_mut(visit).numeric_domain_owner = None;
                    let outcome = checker.finish_independent_binary_rhs(
                        op,
                        left_expr,
                        right,
                        left_domain_failed,
                        result,
                        right_tainted,
                    );
                    result = match outcome {
                        Ok(right_failed) => checker.check_against_context_one(
                            visit,
                            expr,
                            context.borrowed(),
                            Some(PrecomputedType::BinaryLeft {
                                result: Ok(left),
                                pending_before,
                                independent_right: Some(right_failed),
                            }),
                        ),
                        Err(error) => Err(error),
                    };
                    visit = checker.graph.merge_to_parent(visit, true);
                }
                Frame::Call(progress) => {
                    let CallProgress {
                        expr,
                        context,
                        arg_index,
                        parent_fact,
                    } = *progress;
                    let Expr::Call { args, span, .. } = expr else {
                        unreachable!("call frame owns a call")
                    };
                    checker.graph.merge_child(parent_fact, visit, true);
                    visit = parent_fact;
                    if let Err(error) = result {
                        if error.operation == Operation::AnalysisLimit {
                            result = Err(error);
                            return Resume::Next(result, visit);
                        }
                        checker.record_error(error);
                        checker.graph.fact_mut(visit).tainted = true;
                    }
                    let next = arg_index + 1;
                    let binding = &checker.resolved.calls[&(span.start, span.end)];
                    let signature = names::signature(checker.resolved, &binding.target)
                        .expect("callable binding");
                    if next < args.len() {
                        let formal = &signature.params[binding.slots[next]];
                        let next_context = OwnedCheckContext::new(CheckContext {
                            expected: Some(&Type::from(&formal.ty)),
                            expected_span: Some(&formal.type_span),
                            check_here: true,
                            anchor: None,
                            outer_compare: false,
                            related_expected: true,
                            deferred_element: false,
                        });
                        frames.push(Frame::Call(Box::new(CallProgress {
                            expr,
                            context,
                            arg_index: next,
                            parent_fact: visit,
                        })));
                        *inner = &args[next].value;
                        *inner_context = next_context;
                        visit = checker.graph.child_fact(
                            visit,
                            span_key(inner.span()),
                            FactRole::Expression,
                            FactInheritance::Empty,
                            FactInheritance::Empty,
                        );
                        return Resume::Evaluate(visit);
                    }
                    result = checker.check_against_context_one(
                        visit,
                        expr,
                        context.borrowed(),
                        Some(PrecomputedType::CallComplete(Ok(Type::from(
                            &signature.result,
                        )))),
                    );
                }
                Frame::CallTarget(progress) => {
                    let (expr, context, parent_fact) = *progress;
                    checker.graph.merge_child(parent_fact, visit, true);
                    visit = parent_fact;
                    if let Ok(Type::Closure { site, .. }) = &result {
                        let Expr::Call { target, args, .. } = expr else {
                            unreachable!("call target frame owns a call")
                        };
                        match checker.prepare_closure_call(*site, target, args) {
                            Ok((draft, slots)) if !args.is_empty() => {
                                let formal = &draft.params[slots[0]];
                                *inner = &args[0].value;
                                *inner_context = OwnedCheckContext::new(CheckContext {
                                    expected: Some(&formal.3),
                                    expected_span: Some(&formal.2),
                                    check_here: true,
                                    anchor: None,
                                    outer_compare: false,
                                    related_expected: true,
                                    deferred_element: false,
                                });
                                frames.push(Frame::ClosureCall(Box::new(ClosureCallFrameState {
                                    expr,
                                    context,
                                    draft,
                                    slots,
                                    next: 0,
                                    parent_fact: visit,
                                })));
                                visit = checker.graph.child_fact(
                                    visit,
                                    span_key(inner.span()),
                                    FactRole::Expression,
                                    FactInheritance::Empty,
                                    FactInheritance::Empty,
                                );
                                return Resume::Evaluate(visit);
                            }
                            Ok((draft, _)) => {
                                result = checker.check_against_context_one(
                                    visit,
                                    expr,
                                    context.borrowed(),
                                    Some(PrecomputedType::CallComplete(Ok(draft.result))),
                                );
                            }
                            Err(error) => {
                                result = checker.check_against_context_one(
                                    visit,
                                    expr,
                                    context.borrowed(),
                                    Some(PrecomputedType::CallComplete(Err(error))),
                                );
                            }
                        }
                    } else {
                        result = checker.check_against_context_one(
                            visit,
                            expr,
                            context.borrowed(),
                            Some(PrecomputedType::CallTarget(result)),
                        );
                    }
                }
                Frame::ClosureCall(progress) => {
                    let mut state = *progress;
                    let Expr::Call { args, .. } = state.expr else {
                        unreachable!("closure call frame owns a call")
                    };
                    let formal = &state.draft.params[state.slots[state.next]];
                    let outcome = result.and_then(|got| {
                        checker.unify(
                            visit,
                            formal.3.clone(),
                            got,
                            formal.2.clone(),
                            Checker::primary_span(&args[state.next].value),
                            Checker::primary_span(&args[state.next].value),
                            false,
                        )
                    });
                    checker.graph.merge_child(state.parent_fact, visit, true);
                    visit = state.parent_fact;
                    if let Err(error) = outcome {
                        if error.operation == Operation::AnalysisLimit {
                            result = Err(error);
                            return Resume::Next(result, visit);
                        }
                        checker.record_error(error);
                        checker.graph.fact_mut(visit).tainted = true;
                    }
                    state.next += 1;
                    if state.next < args.len() {
                        let formal = &state.draft.params[state.slots[state.next]];
                        *inner = &args[state.next].value;
                        *inner_context = OwnedCheckContext::new(CheckContext {
                            expected: Some(&formal.3),
                            expected_span: Some(&formal.2),
                            check_here: true,
                            anchor: None,
                            outer_compare: false,
                            related_expected: true,
                            deferred_element: false,
                        });
                        frames.push(Frame::ClosureCall(Box::new(state)));
                        visit = checker.graph.child_fact(
                            visit,
                            span_key(inner.span()),
                            FactRole::Expression,
                            FactInheritance::Empty,
                            FactInheritance::Empty,
                        );
                        return Resume::Evaluate(visit);
                    }
                    result = checker.check_against_context_one(
                        visit,
                        state.expr,
                        state.context.borrowed(),
                        Some(PrecomputedType::CallComplete(Ok(state.draft.result))),
                    );
                }
                Frame::UnionCall(progress) => {
                    let UnionCallProgress {
                        expr,
                        context,
                        parent_fact,
                    } = *progress;
                    let Expr::Call { args, span, .. } = expr else {
                        unreachable!("union call frame owns a call")
                    };
                    let binding = &checker.resolved.calls[&(span.start, span.end)];
                    let ResolvedUse::Intrinsic(name) = &binding.target else {
                        unreachable!("union call has an intrinsic binding")
                    };
                    let argument = &args[binding
                        .slots
                        .iter()
                        .position(|slot| *slot == 0)
                        .expect("complete union binding")]
                    .value;
                    let payload_tainted = checker.graph.fact_mut(visit).tainted;
                    checker.graph.merge_child(parent_fact, visit, true);
                    visit = checker.graph.child_fact(
                        parent_fact,
                        span_key(expr.span()),
                        FactRole::Operator,
                        FactInheritance::Inherit,
                        FactInheritance::Empty,
                    );
                    let outcome = checker.finish_union_call(
                        name,
                        argument,
                        span,
                        context.expected.as_ref(),
                        result,
                        payload_tainted,
                    );
                    result = checker.check_against_context_one(
                        visit,
                        expr,
                        context.borrowed(),
                        Some(PrecomputedType::CallComplete(outcome)),
                    );
                    checker.graph.merge_child(parent_fact, visit, true);
                    visit = parent_fact;
                }
                Frame::IntrinsicOne(progress) => {
                    let (expr, context, parent_fact) = *progress;
                    let Expr::Call { args, span, .. } = expr else {
                        unreachable!("intrinsic frame owns a call")
                    };
                    let binding = &checker.resolved.calls[&(span.start, span.end)];
                    let ResolvedUse::Intrinsic(name) = &binding.target else {
                        unreachable!("intrinsic frame has an intrinsic binding")
                    };
                    let outcome = checker.check_intrinsic(
                        name,
                        args,
                        &binding.slots,
                        span,
                        context.expected.as_ref(),
                        context.expected_span.as_ref(),
                        Some(result),
                    );
                    result = checker.check_against_context_one(
                        visit,
                        expr,
                        context.borrowed(),
                        Some(PrecomputedType::CallComplete(outcome)),
                    );
                    checker.graph.merge_child(parent_fact, visit, true);
                    visit = parent_fact;
                }
                Frame::TryAdd(progress) => {
                    let mut state = *progress;
                    let Expr::Call { args, .. } = state.expr else {
                        unreachable!("TryAdd frame owns a call")
                    };
                    match result {
                        Err(error) if error.operation == Operation::AnalysisLimit => {
                            checker.graph.merge_child(state.parent_fact, visit, false);
                            visit = state.parent_fact;
                            result = Err(error);
                            return Resume::Next(result, visit);
                        }
                        Err(error) => {
                            checker.record_error(error);
                            state.failed = true;
                        }
                        Ok(_) => {}
                    }
                    if state.next < args.len() {
                        *inner = &args[state.next].value;
                        state.next += 1;
                        frames.push(Frame::TryAdd(Box::new(state)));
                        *inner_context = OwnedCheckContext::new(CheckContext {
                            expected: Some(&Type::I64),
                            expected_span: None,
                            check_here: true,
                            anchor: None,
                            outer_compare: false,
                            related_expected: false,
                            deferred_element: false,
                        });
                        return Resume::Evaluate(visit);
                    }
                    let result_type = if state.failed {
                        Type::Tainted
                    } else {
                        Type::from(&names::intrinsic("TryAdd").expect("closed intrinsic").result)
                    };
                    result = checker.check_against_context_one(
                        visit,
                        state.expr,
                        state.context.borrowed(),
                        Some(PrecomputedType::CallComplete(Ok(result_type))),
                    );
                    checker.graph.merge_child(state.parent_fact, visit, true);
                    visit = state.parent_fact;
                }
                Frame::AssertEq(progress) => {
                    let mut state = *progress;
                    let accepted = checker.assert_eq_accept(visit, &mut state, result);
                    checker.graph.merge_child(state.call_fact, visit, false);
                    visit = state.call_fact;
                    if let Err(error) = accepted {
                        visit = checker.graph.merge_to_parent(visit, false);
                        result = Err(error);
                        return Resume::Next(result, visit);
                    }
                    let Expr::Call { args, .. } = state.expr else {
                        unreachable!("AssertEq frame owns a call")
                    };
                    state.next += 1;
                    if state.next < args.len() {
                        checker.assert_eq_prepare(&mut state);
                        *inner = &args[state.next].value;
                        frames.push(Frame::AssertEq(Box::new(state)));
                        *inner_context = OwnedCheckContext::new(CheckContext {
                            expected: None,
                            expected_span: None,
                            check_here: true,
                            anchor: None,
                            outer_compare: false,
                            related_expected: false,
                            deferred_element: false,
                        });
                        visit = checker.graph.child_fact(
                            visit,
                            span_key(inner.span()),
                            FactRole::Expression,
                            FactInheritance::Empty,
                            FactInheritance::Empty,
                        );
                        return Resume::Evaluate(visit);
                    }
                    let expr = state.expr;
                    let context = state.context.clone();
                    let outcome = checker.finish_assert_eq(visit, state);
                    result = checker.check_against_context_one(
                        visit,
                        expr,
                        context.borrowed(),
                        Some(PrecomputedType::CallComplete(outcome)),
                    );
                    visit = checker.graph.merge_to_parent(visit, true);
                }
                Frame::If(mut progress) => {
                    match checker.if_advance(&mut visit, &mut progress, Some(result)) {
                        IfAction::Statement(statement) => {
                            frames.push(Frame::If(progress));
                            *pending_task = Some(Task::StatementSite(statement));
                            return Resume::Evaluate(visit);
                        }
                        IfAction::Value(value, context) => {
                            let branch_value = matches!(
                                progress.stage,
                                IfStage::FirstValue { .. } | IfStage::SecondValue { .. }
                            );
                            frames.push(Frame::If(progress));
                            *inner = value;
                            *inner_context = context;
                            if branch_value {
                                visit = checker.graph.child_fact(
                                    visit,
                                    span_key(value.span()),
                                    FactRole::Expression,
                                    FactInheritance::Empty,
                                    FactInheritance::Inherit,
                                );
                            }
                            return Resume::Evaluate(visit);
                        }
                        IfAction::Done(outcome) => {
                            checker.active_if_span = progress.previous_if;
                            result = checker.check_against_context_one(
                                visit,
                                progress.expr,
                                progress.context.borrowed(),
                                Some(PrecomputedType::IfComplete {
                                    result: outcome,
                                    pending_before: progress.pending_before_if,
                                }),
                            );
                        }
                    }
                }
                Frame::Closure { expr, mut progress } => {
                    match checker.closure_body_advance(&mut visit, &mut progress, Some(result)) {
                        ClosureAction::Statement(statement) => {
                            frames.push(Frame::Closure { expr, progress });
                            *pending_task = Some(Task::StatementSite(statement));
                            return Resume::Evaluate(visit);
                        }
                        ClosureAction::Value(value, context) => {
                            let statement_child = progress.pending_statement.is_some();
                            frames.push(Frame::Closure { expr, progress });
                            *inner = value;
                            *inner_context = context;
                            if statement_child {
                                visit = checker.graph.child_fact(
                                    visit,
                                    span_key(value.span()),
                                    FactRole::Statement,
                                    FactInheritance::Empty,
                                    FactInheritance::Inherit,
                                );
                            }
                            return Resume::Evaluate(visit);
                        }
                        ClosureAction::Done(checked) => {
                            let context = progress.context.clone();
                            let outcome = checker.closure_finish(*progress, checked);
                            result = checker.check_against_context_one(
                                visit,
                                expr,
                                context.borrowed(),
                                Some(PrecomputedType::ClosureComplete(outcome)),
                            );
                        }
                    }
                }
                _ => unreachable!("match frame handled by drive"),
            }
            #[cfg(test)]
            CONTEXT_BOUNDARY_TRACE_ENABLED.with(|enabled| {
                if enabled.get()
                    && let Some(tag) = boundary_frame
                {
                    FRAME_EXIT_TRACE.with(|trace| {
                        let fact = checker.graph.fact(visit);
                        trace.borrow_mut().push((
                            tag,
                            fact.numeric_domain_owner,
                            fact.tainted,
                            checker.graph.fact_depth(visit),
                            fact.role,
                        ));
                    });
                }
            });
            Resume::Next(result, visit)
        }
        let (site, role) = match &start {
            DriveStart::Expression(expr, _) => (span_key(expr.span()), FactRole::Expression),
            DriveStart::Block(block) => (span_key(&block.span), FactRole::Statement),
            #[cfg(test)]
            DriveStart::StatementSite(_) => ((0, 0), FactRole::Statement),
        };
        let mut visit = self.graph.begin_visit(site, role);
        let mut frames = Vec::new();
        let sentinel = Expr::Unit(Span {
            file: self.file.into(),
            start: 0,
            end: 0,
        });
        let (mut inner, mut inner_context, mut pending_task) = match start {
            DriveStart::Expression(e, context) => (e, context, None),
            DriveStart::Block(block) => (
                &sentinel,
                OwnedCheckContext {
                    expected: None,
                    expected_span: None,
                    check_here: true,
                    anchor: None,
                    outer_compare: false,
                    related_expected: false,
                    deferred_element: false,
                },
                Some(Task::Block(block)),
            ),
            #[cfg(test)]
            DriveStart::StatementSite(statement) => (
                &sentinel,
                OwnedCheckContext {
                    expected: None,
                    expected_span: None,
                    check_here: true,
                    anchor: None,
                    outer_compare: false,
                    related_expected: false,
                    deferred_element: false,
                },
                Some(Task::StatementSite(statement)),
            ),
        };
        let mut ready_result: Option<Result<Type, Box<Diagnostic>>> = None;
        'evaluate: loop {
            if let Some(task) = pending_task.take() {
                match task {
                    Task::StatementSite(statement) => {
                        if self.analysis_exhausted {
                            ready_result = Some(Ok(Type::Unit));
                        } else {
                            let parent_fact = visit;
                            visit = self.graph.child_fact(
                                parent_fact,
                                span_key(parse::statement_span(statement)),
                                FactRole::Statement,
                                FactInheritance::Empty,
                                FactInheritance::Inherit,
                            );
                            frames.push(Frame::StatementSite(statement, parent_fact));
                            pending_task = Some(Task::Statement(statement));
                            continue 'evaluate;
                        }
                    }
                    Task::Statement(statement) => match statement {
                        Statement::Assign { .. } => {
                            let entry = enter_assignment(self, statement, visit);
                            frames.push(entry.frame);
                            inner = entry.child;
                            inner_context = entry.context;
                            visit = entry.visit;
                            continue 'evaluate;
                        }
                        Statement::Let {
                            annotation,
                            value,
                            span,
                            ..
                        } => match self.prepare_let(annotation.as_ref(), value, span) {
                            Ok(prepared) => {
                                inner_context = OwnedCheckContext {
                                    expected: prepared.written.clone(),
                                    expected_span: prepared.annotation_span.clone(),
                                    check_here: true,
                                    anchor: None,
                                    outer_compare: false,
                                    related_expected: prepared.annotation_span.is_some(),
                                    deferred_element: false,
                                };
                                frames.push(Frame::StatementLet(statement, Box::new(prepared)));
                                inner = value;
                                continue 'evaluate;
                            }
                            Err(error) => ready_result = Some(Err(error)),
                        },
                        Statement::Expr { value, .. } => {
                            frames.push(Frame::StatementExpr);
                            inner = value;
                            inner_context = OwnedCheckContext::new(CheckContext {
                                expected: None,
                                expected_span: None,
                                check_here: true,
                                anchor: None,
                                outer_compare: false,
                                related_expected: false,
                                deferred_element: false,
                            });
                            continue 'evaluate;
                        }
                        Statement::Value {
                            value: Some(value), ..
                        } => {
                            frames.push(Frame::StatementExpr);
                            inner = value;
                            inner_context = OwnedCheckContext {
                                expected: None,
                                expected_span: None,
                                check_here: true,
                                anchor: None,
                                outer_compare: false,
                                related_expected: false,
                                deferred_element: false,
                            };
                            continue 'evaluate;
                        }
                        Statement::Return {
                            value: Some(value), ..
                        } if self.generator_item.is_none() => {
                            let expected = self.graph.result.clone();
                            frames.push(Frame::StatementReturn(value, expected.clone()));
                            inner = value;
                            inner_context = OwnedCheckContext {
                                expected: Some(expected),
                                expected_span: self.graph.result_span.clone(),
                                check_here: true,
                                anchor: None,
                                outer_compare: false,
                                related_expected: self.graph.result_span.is_some(),
                                deferred_element: false,
                            };
                            continue 'evaluate;
                        }
                        Statement::GuardControl {
                            token,
                            value: Some(value),
                            guarded,
                            ..
                        } if !matches!(token.as_str(), "break" | "continue") || *guarded => {
                            if token == "yield" && !guarded && self.generator_item.is_none() {
                                ready_result = Some(
                                    self.statement_without_expression(statement)
                                        .map(|_| Type::Unit),
                                );
                            } else {
                                let item = (token == "yield" && !guarded)
                                    .then(|| self.generator_item.clone())
                                    .flatten();
                                frames.push(Frame::StatementExpr);
                                inner = value;
                                inner_context = OwnedCheckContext {
                                    expected: item,
                                    expected_span: (token == "yield" && !guarded)
                                        .then(|| self.graph.result_span.clone())
                                        .flatten(),
                                    check_here: true,
                                    anchor: None,
                                    outer_compare: false,
                                    related_expected: token == "yield"
                                        && !guarded
                                        && self.graph.result_span.is_some(),
                                    deferred_element: false,
                                };
                                continue 'evaluate;
                            }
                        }
                        Statement::If { condition, .. } => {
                            frames.push(Frame::StatementIf(statement, 0));
                            inner = condition;
                            inner_context = OwnedCheckContext::new(CheckContext {
                                expected: Some(&Type::Bool),
                                expected_span: None,
                                check_here: true,
                                anchor: None,
                                outer_compare: false,
                                related_expected: false,
                                deferred_element: false,
                            });
                            continue 'evaluate;
                        }
                        Statement::While { condition, .. } => {
                            frames.push(Frame::StatementWhile(statement, 0));
                            inner = condition;
                            inner_context = OwnedCheckContext {
                                expected: Some(Type::Bool),
                                expected_span: None,
                                check_here: true,
                                anchor: None,
                                outer_compare: false,
                                related_expected: false,
                                deferred_element: false,
                            };
                            continue 'evaluate;
                        }
                        Statement::For { iterable, .. } => {
                            if let Some(error) = for_range_error(self, statement) {
                                ready_result = Some(Err(error));
                                continue 'evaluate;
                            }
                            frames.push(Frame::StatementFor(statement, 0));
                            inner = iterable;
                            inner_context = OwnedCheckContext {
                                expected: None,
                                expected_span: None,
                                check_here: true,
                                anchor: None,
                                outer_compare: false,
                                related_expected: false,
                                deferred_element: false,
                            };
                            continue 'evaluate;
                        }
                        Statement::Match(selection) => {
                            frames.push(Frame::StatementMatchScrutinee(selection));
                            inner = &selection.scrutinee;
                            inner_context = OwnedCheckContext {
                                expected: None,
                                expected_span: None,
                                check_here: true,
                                anchor: None,
                                outer_compare: false,
                                related_expected: false,
                                deferred_element: false,
                            };
                            continue 'evaluate;
                        }
                        Statement::Block(block) => {
                            pending_task = Some(Task::Block(block));
                            continue 'evaluate;
                        }
                        _ => {
                            ready_result = Some(
                                self.statement_without_expression(statement)
                                    .map(|_| Type::Unit),
                            )
                        }
                    },
                    Task::Block(block) => {
                        self.block_depth += 1;
                        if let Some(statement) = block.statements.first() {
                            if self.block_depth == 1 {
                                let span = parse::statement_span(statement);
                                self.top_statement_start = span.start;
                                self.top_statement_end = span.end;
                            }
                            frames.push(Frame::Block(block, 1));
                            pending_task = Some(Task::StatementSite(statement));
                            continue 'evaluate;
                        }
                        self.block_depth -= 1;
                        ready_result = Some(Ok(Type::Unit));
                    }
                }
            }
            let mut result = if let Some(result) = ready_result.take() {
                result
            } else {
                let mut computed_leaf = None;
                loop {
                    match inner {
                        Expr::Closure(closure) => {
                            let context = inner_context.clone();
                            match self.closure_begin(closure, inner_context.clone()) {
                                Err(error) => {
                                    inner_context = context;
                                    computed_leaf =
                                        Some(PrecomputedType::ClosureComplete(Err(error)));
                                    break;
                                }
                                Ok(mut progress) => {
                                    self.block_depth += 1;
                                    match self.closure_body_advance(&mut visit, &mut progress, None) {
                                        ClosureAction::Statement(statement) => {
                                            frames.push(Frame::Closure {
                                                expr: inner,
                                                progress: Box::new(progress),
                                            });
                                            pending_task = Some(Task::StatementSite(statement));
                                            continue 'evaluate;
                                        }
                                        ClosureAction::Value(value, child_context) => {
                                            let statement_child = progress.pending_statement.is_some();
                                            frames.push(Frame::Closure {
                                                expr: inner,
                                                progress: Box::new(progress),
                                            });
                                            inner = value;
                                            inner_context = child_context;
                                            if statement_child {
                                                visit = self.graph.child_fact(
                                                    visit,
                                                    span_key(value.span()),
                                                    FactRole::Statement,
                                                    FactInheritance::Empty,
                                                    FactInheritance::Inherit,
                                                );
                                            }
                                            continue 'evaluate;
                                        }
                                        ClosureAction::Done(checked) => {
                                            computed_leaf = Some(PrecomputedType::ClosureComplete(
                                                self.closure_finish(progress, checked),
                                            ));
                                            inner_context = context;
                                            break;
                                        }
                                    }
                                }
                            }
                        }
                        Expr::Group(value, span) => {
                            let child_context = inner_context.clone();
                            frames.push(Frame::Group(Box::new((
                                inner,
                                value,
                                span,
                                inner_context,
                            ))));
                            inner = value;
                            inner_context = child_context;
                        }
                        Expr::Array(items, span) if !items.is_empty() => {
                            let element_expected = match inner_context
                                .expected
                                .as_ref()
                                .filter(|_| !inner_context.outer_compare)
                            {
                                Some(Type::Array(element)) => Some(element.as_ref().clone()),
                                Some(Type::Deferred(id)) => {
                                    let root = self.root(*id);
                                    self.graph.type_nodes[&root].array_child.map(Type::Deferred)
                                }
                                _ => None,
                            };
                            let mut state = ArrayFrameState {
                                expr: inner,
                                items,
                                consumer: ConsumerId::new(
                                    self.frame_occurrence_id(inner),
                                    ConsumerClass::ArrayElement,
                                    0,
                                ),
                                span,
                                item_type: element_expected.clone(),
                                item_origin: inner_context.expected_span.clone(),
                                context: inner_context,
                                parent_fact: visit,
                                element_expected,
                                next: 0,
                                current_expected: None,
                                current_origin: None,
                                pending: Vec::new(),
                                diverges: false,
                                invalid: false,
                            };
                            (inner, inner_context) = self.array_next(visit, &mut state);
                            frames.push(Frame::Array(Box::new(state)));
                            visit = self.graph.child_fact(
                                visit,
                                span_key(inner.span()),
                                FactRole::Expression,
                                FactInheritance::Empty,
                                FactInheritance::Empty,
                            );
                        }
                        Expr::Index { target, .. } => {
                            if let Some(error) = index_range_error(self, inner) {
                                ready_result = Some(Err(error));
                                continue 'evaluate;
                            }
                            frames.push(Frame::Index(Box::new((
                                inner,
                                inner_context,
                                visit,
                            ))));
                            inner = target;
                            inner_context = OwnedCheckContext {
                                expected: None,
                                expected_span: None,
                                check_here: true,
                                anchor: None,
                                outer_compare: false,
                                related_expected: false,
                                deferred_element: false,
                            };
                            visit = self.graph.child_fact(
                                visit,
                                span_key(inner.span()),
                                FactRole::Expression,
                                FactInheritance::Inherit,
                                FactInheritance::Empty,
                            );
                        }
                        Expr::Unary { mode: Mode::Read, .. }
                            if self.generator_item.is_some() =>
                        {
                            break;
                        }
                        Expr::Neg { .. } | Expr::Not { .. } => {
                            let expected = if matches!(inner, Expr::Neg { .. }) {
                                Type::I64
                            } else {
                                Type::Bool
                            };
                            let entry = enter_neg_not(inner, inner_context, expected, visit);
                            frames.push(entry.frame);
                            inner = entry.child;
                            inner_context = entry.context;
                            visit = entry.visit;
                        }
                        Expr::Unary {
                            mode: Mode::Read,
                            value,
                            ..
                        } => {
                            frames.push(Frame::Unary(Box::new((inner, inner_context))));
                            inner = value;
                            inner_context = OwnedCheckContext::new(CheckContext {
                                expected: None,
                                expected_span: None,
                                check_here: true,
                                anchor: None,
                                outer_compare: false,
                                related_expected: false,
                                deferred_element: false,
                            });
                        }
                        Expr::Unary { value, .. } => {
                            let child_context = OwnedCheckContext::new(CheckContext {
                                expected: inner_context.expected.as_ref(),
                                expected_span: inner_context.expected_span.as_ref(),
                                check_here: inner_context.check_here,
                                anchor: None,
                                outer_compare: false,
                                related_expected: inner_context.expected_span.is_some(),
                                deferred_element: false,
                            });
                            frames.push(Frame::UnaryPropagated(Box::new((inner, inner_context))));
                            inner = value;
                            inner_context = child_context;
                        }
                        Expr::Binary { .. } => {
                            if let Some(error) = binary_range_error(self, inner) {
                                ready_result = Some(Err(error));
                                continue 'evaluate;
                            }
                            let logical = self.f5 && matches!(inner, Expr::Binary { op, .. } if matches!(op.as_str(), "and" | "or"));
                            let entry = enter_binary(self, visit, inner, inner_context);
                            frames.push(entry.frame);
                            inner = entry.child;
                            inner_context = entry.context;
                            visit = entry.visit;
                            if logical {
                                continue;
                            }
                        }
                        Expr::Call { args, span, .. }
                            if !args.is_empty()
                                && self
                                    .resolved
                                    .calls
                                    .get(&(span.start, span.end))
                                    .is_some_and(|binding| {
                                        !matches!(binding.target, ResolvedUse::Intrinsic(_))
                                    }) =>
                        {
                            let binding = &self.resolved.calls[&(span.start, span.end)];
                            let signature = names::signature(self.resolved, &binding.target)
                                .expect("callable binding");
                            let formal = &signature.params[binding.slots[0]];
                            frames.push(Frame::Call(Box::new(CallProgress {
                                expr: inner,
                                context: inner_context,
                                arg_index: 0,
                                parent_fact: visit,
                            })));
                            inner = &args[0].value;
                            inner_context = OwnedCheckContext::new(CheckContext {
                                expected: Some(&Type::from(&formal.ty)),
                                expected_span: Some(&formal.type_span),
                                check_here: true,
                                anchor: None,
                                outer_compare: false,
                                related_expected: true,
                                deferred_element: false,
                            });
                            visit = self.graph.child_fact(
                                visit,
                                span_key(inner.span()),
                                FactRole::Expression,
                                FactInheritance::Empty,
                                FactInheritance::Empty,
                            );
                        }
                        Expr::Call { args, span, .. }
                            if self
                                .resolved
                                .calls
                                .get(&(span.start, span.end))
                                .is_some_and(|binding| {
                                    matches!(&binding.target, ResolvedUse::Intrinsic(name)
                                    if matches!(name.as_str(), "Some" | "Ok" | "Err"))
                                }) =>
                        {
                            let binding = &self.resolved.calls[&(span.start, span.end)];
                            let ResolvedUse::Intrinsic(name) = &binding.target else {
                                unreachable!("union call has an intrinsic binding")
                            };
                            let argument = &args[binding
                                .slots
                                .iter()
                                .position(|slot| *slot == 0)
                                .expect("complete union binding")]
                            .value;
                            let kind = if name == "Some" {
                                UnionKind::Option
                            } else {
                                UnionKind::Result
                            };
                            let tag = kind.variant_tag(name).expect("closed union intrinsic");
                            let payload_expected = inner_context
                                .expected
                                .as_ref()
                                .and_then(concrete_union)
                                .filter(|union| union.kind == kind)
                                .and_then(|union| {
                                    union.variant(tag).and_then(|variant| variant.payload)
                                })
                                .cloned();
                            let child_context = OwnedCheckContext {
                                expected: payload_expected,
                                expected_span: inner_context.expected_span.clone(),
                                check_here: true,
                                anchor: None,
                                outer_compare: false,
                                related_expected: inner_context.expected_span.is_some(),
                                deferred_element: false,
                            };
                            frames.push(Frame::UnionCall(Box::new(UnionCallProgress {
                                expr: inner,
                                context: inner_context,
                                parent_fact: visit,
                            })));
                            inner = argument;
                            inner_context = child_context;
                            visit = self.graph.child_fact(
                                visit,
                                span_key(inner.span()),
                                FactRole::Expression,
                                FactInheritance::Empty,
                                FactInheritance::Empty,
                            );
                        }
                        Expr::Call { args, span, .. }
                            if self.resolved.calls.get(&(span.start, span.end)).is_some_and(|binding| {
                                matches!(&binding.target, ResolvedUse::Intrinsic(name) if name == "AssertEq")
                            }) =>
                        {
                            let call_fact = self.graph.child_fact(
                                visit,
                                span_key(inner.span()),
                                FactRole::Operator,
                                FactInheritance::Inherit,
                                FactInheritance::Empty,
                            );
                            let mut state = AssertEqFrameState {
                                expr: inner,
                                context: inner_context,
                                next: 0,
                                call_fact,
                                prior_errors: 0,
                                values: [None, None],
                                evidence: [None, None],
                                invalid: false,
                            };
                            self.assert_eq_prepare(&mut state);
                            frames.push(Frame::AssertEq(Box::new(state)));
                            inner = &args[0].value;
                            inner_context = OwnedCheckContext::new(CheckContext {
                                expected: None,
                                expected_span: None,
                                check_here: true,
                                anchor: None,
                                outer_compare: false,
                                related_expected: false,
                                deferred_element: false,
                            });
                            visit = self.graph.child_fact(
                                call_fact,
                                span_key(inner.span()),
                                FactRole::Expression,
                                FactInheritance::Empty,
                                FactInheritance::Empty,
                            );
                        }
                        Expr::Call { args, span, .. }
                            if self.resolved.calls.get(&(span.start, span.end)).is_some_and(|binding| {
                                matches!(&binding.target, ResolvedUse::Intrinsic(name) if name == "TryAdd")
                            }) =>
                        {
                            frames.push(Frame::TryAdd(Box::new(TryAddFrameState {
                                expr: inner,
                                context: inner_context,
                                next: 1,
                                failed: false,
                                parent_fact: visit,
                            })));
                            inner = &args[0].value;
                            inner_context = OwnedCheckContext::new(CheckContext {
                                expected: Some(&Type::I64),
                                expected_span: None,
                                check_here: true,
                                anchor: None,
                                outer_compare: false,
                                related_expected: false,
                                deferred_element: false,
                            });
                            visit = self.graph.child_fact(
                                visit,
                                span_key(inner.span()),
                                FactRole::Operator,
                                FactInheritance::Inherit,
                                FactInheritance::Empty,
                            );
                        }
                        Expr::Call { args, span, .. }
                            if self.resolved.calls.get(&(span.start, span.end)).is_some_and(|binding| {
                                matches!(&binding.target, ResolvedUse::Intrinsic(name)
                                    if !matches!(name.as_str(), "Some" | "Ok" | "Err" | "TryAdd" | "AssertEq" | "MockRequest" | "MockUnknown" | "Fail"))
                            }) =>
                        {
                            let binding = &self.resolved.calls[&(span.start, span.end)];
                            let ResolvedUse::Intrinsic(name) = &binding.target else { unreachable!() };
                            let argument = &args[binding.slots.iter().position(|slot| *slot == 0).expect("complete intrinsic binding")].value;
                            let numeric = matches!(name.as_str(), "MockOpen" | "MockBlockingOpen" | "MockWriteI64" | "MockReply");
                            frames.push(Frame::IntrinsicOne(Box::new((inner, inner_context, visit))));
                            inner = argument;
                            inner_context = OwnedCheckContext {
                                expected: numeric.then_some(Type::I64),
                                expected_span: None,
                                check_here: true,
                                anchor: None,
                                outer_compare: false,
                                related_expected: false,
                                deferred_element: false,
                            };
                            visit = self.graph.child_fact(
                                visit,
                                span_key(inner.span()),
                                FactRole::Expression,
                                FactInheritance::Inherit,
                                FactInheritance::Empty,
                            );
                        }
                        Expr::Call { target, span, .. }
                            if !self.resolved.calls.contains_key(&(span.start, span.end)) =>
                        {
                            frames.push(Frame::CallTarget(Box::new((
                                inner,
                                inner_context,
                                visit,
                            ))));
                            inner = target;
                            inner_context = OwnedCheckContext::new(CheckContext {
                                expected: None,
                                expected_span: None,
                                check_here: true,
                                anchor: None,
                                outer_compare: false,
                                related_expected: false,
                                deferred_element: false,
                            });
                            visit = self.graph.child_fact(
                                visit,
                                span_key(inner.span()),
                                FactRole::Expression,
                                FactInheritance::Inherit,
                                FactInheritance::Empty,
                            );
                        }
                        Expr::If {
                            condition,
                            then_block,
                            otherwise,
                            ..
                        } => {
                            let pending_before_if = self.pending_join_count();
                            let previous_if = self.active_if_span.replace(inner.span().clone());
                            let pending_before_condition = self.graph.obligations.len();
                            let branch_context = OwnedCheckContext {
                                expected: inner_context
                                    .expected
                                    .as_ref()
                                    .filter(|_| !inner_context.outer_compare)
                                    .cloned(),
                                expected_span: inner_context
                                    .expected_span
                                    .as_ref()
                                    .filter(|_| !inner_context.outer_compare)
                                    .cloned(),
                                ..inner_context.clone()
                            };
                            frames.push(Frame::If(Box::new(IfProgress {
                                expr: inner,
                                consumer: ConsumerId::new(
                                    self.frame_occurrence_id(inner),
                                    ConsumerClass::IfContinuation,
                                    0,
                                ),
                                next_child_index: 0,
                                condition,
                                then_block,
                                otherwise: otherwise.as_deref(),
                                context: inner_context,
                                branch_context,
                                previous_if,
                                pending_before_if,
                                parent_fact: visit,
                                pending_before_condition,
                                stage: IfStage::Condition,
                                branch: None,
                                first: None,
                                first_failed: false,
                                first_anchored: false,
                                first_partial: false,
                                first_span: None,
                                first_for_anchor: None,
                                anchor_error: None,
                                anchor_pending: false,
                                anchor: None,
                            })));
                            inner = condition;
                            inner_context = OwnedCheckContext::new(CheckContext {
                                expected: Some(&Type::Bool),
                                expected_span: None,
                                check_here: true,
                                anchor: None,
                                outer_compare: false,
                                related_expected: false,
                                deferred_element: false,
                            });
                            visit = self.graph.child_fact(
                                visit,
                                span_key(inner.span()),
                                FactRole::Expression,
                                FactInheritance::Empty,
                                FactInheritance::Inherit,
                            );
                        }
                        Expr::Match(selection) => {
                            frames.push(Frame::MatchScrutinee(Box::new((inner, inner_context))));
                            inner = &selection.scrutinee;
                            inner_context = OwnedCheckContext::new(CheckContext {
                                expected: None,
                                expected_span: None,
                                check_here: true,
                                anchor: None,
                                outer_compare: false,
                                related_expected: false,
                                deferred_element: false,
                            });
                        }
                        _ => break,
                    }
                }
                self.check_against_context_one(
                    visit,
                    inner,
                    inner_context.borrowed(),
                    computed_leaf,
                )
            };
            while let Some(frame) = frames.pop() {
                match frame {
                    frame @ (Frame::StatementMatchScrutinee(_) | Frame::StatementMatch { .. }) => {
                        match resume_statement_match(
                            self,
                            visit,
                            frame,
                            result,
                            &mut frames,
                            &mut inner,
                            &mut inner_context,
                            &mut pending_task,
                        ) {
                            Resume::Next(next, next_fact) => {
                                result = next;
                                visit = next_fact;
                            }
                            Resume::Evaluate(next_fact) => {
                                visit = next_fact;
                                continue 'evaluate;
                            }
                        }
                    }
                    frame @ (Frame::MatchScrutinee(_) | Frame::Match { .. }) => {
                        match resume_expression_match(
                            self,
                            visit,
                            frame,
                            result,
                            &mut frames,
                            &mut inner,
                            &mut inner_context,
                            &mut pending_task,
                        ) {
                            Resume::Next(next, next_fact) => {
                                result = next;
                                visit = next_fact;
                            }
                            Resume::Evaluate(next_fact) => {
                                visit = next_fact;
                                continue 'evaluate;
                            }
                        }
                    }
                    frame => match resume_non_match(
                        self,
                        visit,
                        frame,
                        result,
                        &mut frames,
                        &mut inner,
                        &mut inner_context,
                        &mut pending_task,
                    ) {
                        Resume::Next(next, next_fact) => {
                            result = next;
                            visit = next_fact;
                        }
                        Resume::Evaluate(next_fact) => {
                            visit = next_fact;
                            continue 'evaluate;
                        }
                    },
                }
            }
            let tainted = self.graph.end_visit(visit);
            return (result, tainted);
        }
    }

    fn propagate_group(&mut self, inner: &Expr, span: &Span) {
        // A group has no independent type or diagnostic site, even when its
        // child failed after leaving partial inference evidence.
        if let Some(partial) = self
            .graph
            .expression_types
            .get(&span_key(inner.span()))
            .cloned()
        {
            self.graph.expression_types.insert(span_key(span), partial);
        }
        if let Some(nominal) = self
            .graph
            .conditional_result(&span_key(inner.span()))
            .cloned()
        {
            self.graph.set_conditional_result(span_key(span), nominal);
        }
        if self
            .graph
            .has(&span_key(inner.span()), OccurrenceFlag::Deferred)
        {
            self.graph.mark(span_key(span), OccurrenceFlag::Deferred);
        }
        if let Some(id) = self.expression_variable(inner) {
            self.graph.expr_vars.insert(span_key(span), id);
        }
        if self
            .graph
            .has(&span_key(inner.span()), OccurrenceFlag::Divergent)
        {
            self.graph.mark(span_key(span), OccurrenceFlag::Divergent);
        }
        if self
            .graph
            .has(&span_key(inner.span()), OccurrenceFlag::PartialNormal)
        {
            self.graph
                .mark(span_key(span), OccurrenceFlag::PartialNormal);
        }
        if self
            .graph
            .has(&span_key(inner.span()), OccurrenceFlag::ContextRejected)
        {
            self.graph
                .mark(span_key(span), OccurrenceFlag::ContextRejected);
        }
        if self
            .graph
            .has(&span_key(inner.span()), OccurrenceFlag::FailedJoin)
        {
            self.graph.mark(span_key(span), OccurrenceFlag::FailedJoin);
        }
        if self
            .graph
            .has(&span_key(inner.span()), OccurrenceFlag::PendingDomain)
        {
            self.graph
                .mark(span_key(span), OccurrenceFlag::PendingDomain);
        }
    }

    fn check_against_context_one(
        &mut self,
        visit: FactId,
        e: &Expr,
        context: CheckContext<'_>,
        computed: Option<PrecomputedType>,
    ) -> Result<Type, Box<Diagnostic>> {
        let mut unary_operand = None;
        let computed = match computed {
            Some(PrecomputedType::UnaryOperand(result)) => {
                unary_operand = Some(result);
                None
            }
            other => other,
        };
        let CheckContext {
            expected,
            expected_span,
            check_here,
            related_expected,
            ..
        } = context;
        let mut context_use = if matches!(computed, Some(PrecomputedType::Whole(_))) {
            ContextUse::Propagated
        } else if matches!(
            computed,
            Some(PrecomputedType::ArrayComplete {
                constructor_pair: true,
                ..
            })
        ) {
            ContextUse::ConstructorPair
        } else {
            ContextUse::Compare
        };
        let comparison_parent = matches!(
            e,
            Expr::Binary { .. } | Expr::Index { .. } | Expr::Call { .. } | Expr::Array(..)
        )
        .then_some(visit);
        #[cfg(test)]
        let boundary_stage = match &computed {
            Some(PrecomputedType::BinaryLeft { .. }) => "BinaryLeft",
            Some(PrecomputedType::BinaryRight { .. }) => "BinaryRight",
            Some(PrecomputedType::BinaryDeferredRight { .. }) => "BinaryDeferredRight",
            Some(PrecomputedType::Whole(_)) => "Whole",
            Some(PrecomputedType::ArrayComplete { .. }) => "ArrayComplete",
            Some(PrecomputedType::IndexTarget { .. }) => "IndexTarget",
            Some(PrecomputedType::CallComplete(_)) => "CallComplete",
            _ => "Other",
        };
        let visit = if let Some(parent) = comparison_parent {
            self.graph.child_fact(
                parent,
                span_key(e.span()),
                FactRole::Operator,
                FactInheritance::Inherit,
                FactInheritance::Empty,
            )
        } else {
            visit
        };
        let (whole, binary_left, binary_right, index_target, if_result, call_target) =
            match computed {
                Some(PrecomputedType::ArrayComplete { result, .. }) => {
                    (Some(result), None, None, None, None, None)
                }
                Some(PrecomputedType::IndexTarget {
                    target,
                    selector,
                    deferred_child,
                }) => (
                    None,
                    None,
                    None,
                    Some(IndexPrepared {
                        target,
                        selector,
                        deferred_child,
                    }),
                    None,
                    None,
                ),
                Some(PrecomputedType::BinaryLeft {
                    result,
                    pending_before,
                    independent_right,
                }) => (
                    None,
                    Some((result, pending_before, independent_right)),
                    None,
                    None,
                    None,
                    None,
                ),
                Some(PrecomputedType::BinaryRight {
                    left,
                    result,
                    pending_before,
                }) => (
                    None,
                    None,
                    Some((left, result, pending_before)),
                    None,
                    None,
                    None,
                ),
                Some(PrecomputedType::BinaryDeferredRight {
                    left,
                    result,
                    pending_before_left,
                    pending_before_operator,
                }) => (
                    None,
                    Some((Ok(left.clone()), pending_before_left, None)),
                    Some((left, result, pending_before_operator)),
                    None,
                    None,
                    None,
                ),
                Some(
                    PrecomputedType::CallComplete(result)
                    | PrecomputedType::MatchComplete(result)
                    | PrecomputedType::ClosureComplete(result),
                ) => (Some(result), None, None, None, None, None),
                Some(PrecomputedType::CallTarget(result)) => {
                    (None, None, None, None, None, Some(result))
                }
                Some(PrecomputedType::IfComplete {
                    result,
                    pending_before,
                }) => (None, None, None, None, Some((result, pending_before)), None),
                Some(PrecomputedType::Whole(result)) => {
                    (Some(result), None, None, None, None, None)
                }
                Some(PrecomputedType::UnaryOperand(_)) => unreachable!("unary operand separated"),
                None => (None, None, None, None, None, None),
            };
        let result = match whole {
            Some(result) => result,
            None => self.expr_inner(
                visit,
                e,
                context,
                &mut context_use,
                binary_left,
                binary_right,
                unary_operand,
                index_target,
                if_result,
                call_target,
            ),
        };
        #[cfg(test)]
        CONTEXT_BOUNDARY_TRACE_ENABLED.with(|enabled| {
            if enabled.get() {
                CONTEXT_BOUNDARY_TRACE.with(|trace| {
                    trace.borrow_mut().push((
                        "synthesis",
                        boundary_stage,
                        self.graph.fact(visit).numeric_domain_owner,
                        self.graph.active_count(),
                        comparison_parent.is_some_and(|parent| parent == visit),
                        self.graph.fact(visit).role == FactRole::Operator,
                    ));
                });
            }
        });
        let visit = if let Some(parent) = comparison_parent {
            self.graph.merge_child(parent, visit, true);
            parent
        } else {
            visit
        };
        #[cfg(test)]
        CONTEXT_BOUNDARY_TRACE_ENABLED.with(|enabled| {
            if enabled.get() {
                CONTEXT_FACT_DEPTH_TRACE.with(|trace| {
                    trace
                        .borrow_mut()
                        .push((boundary_stage, self.graph.fact_depth(visit)));
                });
            }
        });
        #[cfg(test)]
        CONTEXT_BOUNDARY_TRACE_ENABLED.with(|enabled| {
            if enabled.get() {
                CONTEXT_BOUNDARY_TRACE.with(|trace| {
                    trace.borrow_mut().push((
                        "after_synthesis",
                        boundary_stage,
                        self.graph.fact(visit).numeric_domain_owner,
                        self.graph.active_count(),
                        comparison_parent.is_some_and(|parent| parent == visit),
                        self.graph.fact(visit).role == FactRole::Operator,
                    ));
                });
            }
        });
        let got = result?;
        self.capture_expression_dependencies(e);
        if got == Type::Tainted
            && self
                .graph
                .has(&span_key(e.span()), OccurrenceFlag::PendingDomain)
            && let Expr::Binary { op, .. } = e
        {
            // Keep the executable result separate from the causal type used
            // while the operand domain is still hypothetical. It is exported
            // only after this function's pending obligations have closed.
            self.graph.set_conditional_result(
                span_key(e.span()),
                if super::operators::is_comparison(op, self.f5) {
                    Type::Bool
                } else {
                    Type::I64
                },
            );
        }
        match internal_state(&got) {
            Some(Type::Tainted) => {
                self.graph.mark(span_key(e.span()), OccurrenceFlag::Tainted);
                self.graph.fact_mut(visit).tainted = true;
            }
            Some(Type::Never) => {
                self.graph
                    .mark(span_key(e.span()), OccurrenceFlag::Deferred);
                // A divergent if records its own span. Len can record
                // divergence while returning its declared I64; Unary can
                // forward a Never child without a local mark, but that
                // wrapper mark has no observable consumer. No reachable
                // expression needs this insertion to diagnose divergence.
                self.graph
                    .mark(span_key(e.span()), OccurrenceFlag::Divergent);
            }
            Some(Type::Deferred(_)) => {
                self.graph
                    .mark(span_key(e.span()), OccurrenceFlag::Deferred);
            }
            _ => {}
        }
        // Keep the synthesized type even when its contextual comparison
        // fails: a sibling may still constrain an unresolved constructor.
        self.graph
            .expression_types
            .insert(span_key(e.span()), got.clone());
        if check_here
            && let Some(expected) = expected
            && (internal_state(expected).is_none() || context.deferred_element)
            && !self.graph.fact_mut(visit).tainted
            && !self
                .graph
                .has(&span_key(e.span()), OccurrenceFlag::PendingDomain)
            && !self
                .graph
                .has(&span_key(e.span()), OccurrenceFlag::FailedJoin)
        {
            let result: Result<(), Box<Diagnostic>> = match context_use {
                ContextUse::ConstructorPair => {
                    // Child checks already compared the elements. The array
                    // constructor itself still counts as one logical pair.
                    self.charge_solver(&Self::primary_span(e))
                }
                ContextUse::Propagated => Ok(()),
                ContextUse::Compare => {
                    // A projection has no result type until its base is
                    // proved Array. Retain the consumer's obligation at the
                    // original access site instead of binding its hypothetical
                    // item before the base shape is known.
                    if context.outer_compare
                        && let Some(projection) =
                            self.graph
                                .obligations
                                .iter_mut()
                                .find_map(|entry| match entry {
                                    ObligationKind::Projection(projection)
                                        if projection.result_span == Self::primary_span(e) =>
                                    {
                                        Some(projection)
                                    }
                                    _ => None,
                                })
                    {
                        projection.expected = Some(expected.clone());
                        projection.expected_span = expected_span.cloned();
                        projection.related_expected = related_expected;
                        return Ok(got);
                    }
                    // Compare the complete representative when an array
                    // with a pending element faces an exterior comparison,
                    // or when its established Array shape faces a scalar.
                    // This keeps the source operand's full type and delays
                    // serialization until its element is determined.
                    let pending_outer_shape = matches!((&got, expected), (Type::Array(_), ty)
                        if !matches!(ty, Type::Array(_)))
                        || concrete_union(&got).is_some_and(|union| {
                            concrete_union(expected).is_none_or(|other| other.kind != union.kind)
                        });
                    let compared = if (context.outer_compare || pending_outer_shape)
                        && internal_state(&got).is_some()
                    {
                        self.expression_variable(e)
                            .map(Type::Deferred)
                            .unwrap_or(got.clone())
                    } else {
                        got.clone()
                    };
                    let pending_before_comparison = self.pending_join_count();
                    #[cfg(test)]
                    CONTEXT_BOUNDARY_TRACE_ENABLED.with(|enabled| {
                        if enabled.get() {
                            CONTEXT_BOUNDARY_TRACE.with(|trace| {
                                trace.borrow_mut().push((
                                    "comparison",
                                    boundary_stage,
                                    self.graph.fact(visit).numeric_domain_owner,
                                    self.graph.active_count(),
                                    comparison_parent.is_some_and(|parent| parent == visit),
                                    self.graph.fact(visit).role == FactRole::Operator,
                                ));
                            });
                        }
                    });
                    let outcome = self.unify_with_expected_origin(
                        visit,
                        expected.clone(),
                        compared,
                        expected_span
                            .cloned()
                            .unwrap_or_else(|| Self::primary_span(e)),
                        Self::primary_span(e),
                        Self::primary_span(e),
                        UnifyPolicy {
                            branch: false,
                            related_expected,
                        },
                    );
                    if self.pending_join_count() > pending_before_comparison {
                        // The conflict is already causal evidence that this
                        // occurrence cannot supply a valid result to its
                        // consumer, even before its types can be printed.
                        self.graph
                            .mark(span_key(e.span()), OccurrenceFlag::FailedJoin);
                    }
                    outcome.map(|_| ())
                }
            };
            if let Err(error) = result {
                if matches!(context_use, ContextUse::Compare)
                    && error.operation != Operation::AnalysisLimit
                {
                    self.graph
                        .mark(span_key(e.span()), OccurrenceFlag::ContextRejected);
                }
                // A constructor conflict is also tracked when it is queued
                // or finalized. This mark is defensive: changing the code
                // predicate here alone cannot change a reachable diagnostic
                // while the same root is already in failed_constructors.
                if error.code == "NX-E0103"
                    && let Some(id) = self.expression_variable(e)
                {
                    let root = self.root(id);
                    self.graph.mark_failed_constructor(root);
                }
                return Err(error);
            }
        }
        Ok(got)
    }

    #[allow(clippy::too_many_arguments)] // Each prepared child belongs to one frame stage.
    fn expr_inner(
        &mut self,
        visit: FactId,
        e: &Expr,
        context: CheckContext<'_>,
        context_use: &mut ContextUse,
        binary_left: Option<BinaryLeftPrepared>,
        binary_right: Option<(Type, Result<Type, Box<Diagnostic>>, usize)>,
        unary_operand: Option<Result<Type, Box<Diagnostic>>>,
        index_target: Option<IndexPrepared>,
        if_result: Option<(IfResult, usize)>,
        call_target: Option<Result<Type, Box<Diagnostic>>>,
    ) -> Result<Type, Box<Diagnostic>> {
        let CheckContext {
            expected,
            expected_span,
            outer_compare,
            ..
        } = context;
        match e {
            Expr::Closure(_) => unreachable!("closures complete through their frame"),
            Expr::Integer(text, span) => {
                if text.parse::<i64>().is_err() {
                    return Err(names::diagnostic(
                        "NX-E0107",
                        None,
                        Stage::Types,
                        Operation::ValueLiteral,
                        span.clone(),
                    ));
                }
                Ok(Type::I64)
            }
            Expr::Bool(..) => Ok(Type::Bool),
            Expr::Neg { value, .. } => {
                let got = unary_operand.expect("negated operand is scheduled by driver")?;
                if got == Type::Tainted {
                    return Ok(Type::Tainted);
                }
                if got != Type::I64 {
                    return Err(mismatch(
                        Self::primary_span(value),
                        Self::primary_span(value),
                        None,
                        "I64",
                        &got,
                    ));
                }
                Ok(Type::I64)
            }
            Expr::Not { value, .. } => {
                let got = unary_operand.expect("not operand is scheduled by driver")?;
                if got == Type::Tainted {
                    return Ok(Type::Tainted);
                }
                if got != Type::Bool {
                    return Err(mismatch(
                        Self::primary_span(value),
                        Self::primary_span(value),
                        None,
                        "Bool",
                        &got,
                    ));
                }
                Ok(Type::Bool)
            }
            Expr::Unit(..) => Ok(Type::Unit),
            Expr::Name(name, span) => match self.resolved.uses.get(&span.start) {
                Some(ResolvedUse::Constant(_)) => Ok(Type::Tainted),
                Some(ResolvedUse::Intrinsic(intrinsic)) if intrinsic == "None" => {
                    self.option_none(span, expected)
                }
                Some(ResolvedUse::Local(local)) => {
                    if name == "None" && !self.graph.locals.contains_key(&local.name_span.start) {
                        return self.option_none(span, expected);
                    }
                    if self
                        .graph
                        .locals
                        .get(&local.name_span.start)
                        .is_some_and(|known| known.invalid)
                    {
                        self.graph.fact_mut(visit).tainted = true;
                        return Ok(Type::Tainted);
                    }
                    if let Some(known) = self.graph.locals.get(&local.name_span.start).cloned() {
                        if let Some(id) = known.variable {
                            if let Some((ty, _)) = self.concrete(id) {
                                return Ok(ty);
                            }
                            if !self.planning
                                && let Some(plan) = self.graph.read_plan.get(&local.name_span.start)
                                && span.start < plan.assignment_start
                                && plan
                                    .blocked
                                    .iter()
                                    .all(|(start, end)| span.start < *start || span.start >= *end)
                            {
                                return Ok(plan.ty.clone());
                            }
                            self.graph.mark(span_key(span), OccurrenceFlag::Deferred);
                            self.graph.expr_vars.insert(span_key(span), id);
                            return Ok(Type::Deferred(id));
                        }
                        Ok(known.ty)
                    } else {
                        Ok(Type::Tainted)
                    }
                }
                _ => Ok(Type::Tainted), // Function values have already been rejected by parse or names.
            },
            Expr::Group(..) => {
                unreachable!("group expressions are visited by check_against_context")
            }
            Expr::Unary {
                mode: Mode::Read,
                value,
                span,
            } => {
                if self.generator_item.is_some() {
                    let question = Span {
                        file: span.file.clone(),
                        start: span.end - 1,
                        end: span.end,
                    };
                    return Err(names::diagnostic(
                        "NX-E1214",
                        None,
                        Stage::Types,
                        Operation::TypesCheck,
                        question,
                    ));
                }
                let operand = unary_operand.expect("unary operand is prepared by driver")?;
                if self.graph.fact_mut(visit).tainted || operand == Type::Tainted {
                    return Ok(Type::Tainted);
                }
                if operand == Type::Never {
                    return Ok(Type::Never);
                }
                // A monomorphic binding can still be represented by its
                // deferred root even after Ok/Err established Result children.
                // Project those same children so `?` constrains the graph
                // shared by every use of the binding and its aliases.
                let operand = match operand {
                    Type::Deferred(id) => {
                        if let Some((ty, _)) = self.concrete(id) {
                            ty
                        } else {
                            let root = self.root(id);
                            if self.graph.type_nodes[&root].variant_children.is_none() {
                                Type::Deferred(id)
                            } else {
                                let (ok, err) = self
                                    .require_union(
                                        id,
                                        UnionKind::Result,
                                        &Self::primary_span(value),
                                    )?
                                    .result_payloads()
                                    .unwrap();
                                Type::result(
                                    Arc::new(Type::Deferred(ok)),
                                    Arc::new(Type::Deferred(err)),
                                )
                            }
                        }
                    }
                    other => other,
                };
                let Type::Union(UnionKind::Result, ok, Some(err)) = operand else {
                    return Err(mismatch(
                        Span {
                            file: span.file.clone(),
                            start: span.end - 1,
                            end: span.end,
                        },
                        Self::primary_span(value),
                        None,
                        "domain:result",
                        &operand,
                    ));
                };
                let Type::Union(UnionKind::Result, _, Some(function_err)) = &self.graph.result
                else {
                    return Err(mismatch(
                        Span {
                            file: span.file.clone(),
                            start: span.end - 1,
                            end: span.end,
                        },
                        self.graph
                            .result_span
                            .clone()
                            .unwrap_or_else(|| span.clone()),
                        None,
                        "domain:result",
                        &self.graph.result,
                    ));
                };
                let function_err = function_err.as_ref().clone();
                let try_site = Span {
                    file: span.file.clone(),
                    start: span.end - 1,
                    end: span.end,
                };
                if let Some(err_type) = self.resolved_join_side(err.as_ref().clone())
                    && err_type != function_err
                {
                    return Err(mismatch(
                        try_site,
                        Self::primary_span(value),
                        self.graph.result_span.clone(),
                        &pretty(&function_err).unwrap_or_default(),
                        &err_type,
                    ));
                }
                self.unify(
                    visit,
                    function_err,
                    err.as_ref().clone(),
                    self.graph
                        .result_span
                        .clone()
                        .unwrap_or_else(|| try_site.clone()),
                    Self::primary_span(value),
                    try_site,
                    false,
                )?;
                if let Type::Deferred(id) = ok.as_ref() {
                    self.graph.expr_vars.insert(span_key(e.span()), *id);
                }
                Ok(ok.as_ref().clone())
            }
            Expr::Unary { .. } => unreachable!("unary operand is scheduled by driver"),
            Expr::Array(xs, span) => {
                let element_expected_type = match expected.filter(|_| !outer_compare) {
                    Some(Type::Array(t)) => Some(t.as_ref().clone()),
                    Some(Type::Deferred(id)) => {
                        let root = self.root(*id);
                        self.graph.type_nodes[&root].array_child.map(Type::Deferred)
                    }
                    _ => None,
                };
                let element_expected = element_expected_type.as_ref();
                if element_expected.is_some() {
                    *context_use = ContextUse::ConstructorPair;
                }
                let Some(_) = xs.first() else {
                    if let Some(element) = element_expected {
                        return Ok(Type::Array(Arc::new(element.clone())));
                    }
                    let child = self.new_variable(span)?;
                    let array = self.new_variable(span)?;
                    self.graph.type_node_mut(&array).unwrap().array_child = Some(child);
                    self.graph.type_node_mut(&array).unwrap().array_origin = Some(span.clone());
                    self.graph.type_node_mut(&array).unwrap().array_constructed = true;
                    self.require_copyable(child, span, false)?;
                    self.graph.expr_vars.insert(span_key(e.span()), array);
                    self.graph
                        .mark(span_key(e.span()), OccurrenceFlag::Deferred);
                    self.graph.register(ObligationKind::Constructor {
                        child,
                        array,
                        span: span.clone(),
                        has_element_witness: false,
                    });
                    return Ok(Type::Array(Arc::new(Type::Deferred(child))));
                };
                unreachable!("nonempty array is scheduled by driver")
            }
            Expr::Binary {
                op, left, right, ..
            } => {
                if binary_left.is_some()
                    && let Some((left_type, right_result, pending_before_operator)) = binary_right
                {
                    return self.finish_deferred_binary_right(
                        visit,
                        e,
                        op,
                        left,
                        right,
                        left_type,
                        right_result,
                        pending_before_operator,
                        expected,
                        expected_span,
                    );
                }
                if binary_left.is_none()
                    && let Some((left_type, right_result, pending_before_right)) = binary_right
                {
                    return self.finish_binary_right(
                        visit,
                        e,
                        op,
                        left,
                        right,
                        &left_type,
                        right_result,
                        pending_before_right,
                        expected,
                        expected_span,
                    );
                }
                let (left_result, pending_before_left, independent_right) =
                    binary_left.expect("binary left operand is prepared by check_against_context");
                let l = left_result?;
                if self.graph.has(
                    &span_key(&Self::primary_span(left)),
                    OccurrenceFlag::PendingDomain,
                ) {
                    // A pending operand domain does not yet authorize the
                    // operation's nominal result as a cross-operand witness.
                    // The other operand still has its own domain site.
                    let right_tainted =
                        independent_right.expect("independent binary right operand");
                    if right_tainted {
                        // The sibling owns a real failure. Its containing
                        // operation has no nominal value for a selector or
                        // another consumer (§4.1.2.5).
                        self.graph.fact_mut(visit).tainted = true;
                        return Ok(Type::Tainted);
                    }
                    if super::operators::is_comparison(op, self.f5)
                        && !right_tainted
                        && let Some(found) = self.pending_operation_type(right)
                        && let Some((variable, domain)) = self.pending_operation_guard(left)
                    {
                        // The cross equality is conditional on the left
                        // operation's still unresolved domain. Keep its
                        // source site until that domain is decided.
                        self.graph
                            .register(ObligationKind::GuardedComparison(Box::new(
                                PendingGuardedComparison {
                                    variable,
                                    domain,
                                    expected: self
                                        .pending_operation_type(left)
                                        .unwrap_or_else(|| l.clone()),
                                    found,
                                    expected_origin: Self::primary_span(left),
                                    found_site: Self::primary_span(right),
                                    result: Self::primary_span(e),
                                    consumer_expected: expected.cloned(),
                                    consumer_origin: expected_span.cloned(),
                                    domain_error_if_equal: None,
                                    additional_domains: Vec::new(),
                                    projections: Vec::new(),
                                    dependent_results: Vec::new(),
                                    require_resolved: false,
                                    conditional_bindings: Vec::new(),
                                },
                            )));
                    }
                    if super::operators::is_arithmetic(op, self.f5)
                        && let Some((variable, domain)) = self.pending_operation_guard(left)
                    {
                        self.defer_nominal_domain(
                            variable,
                            domain,
                            e.span(),
                            &Type::I64,
                            expected,
                            expected_span,
                            Some(&Self::primary_span(left)),
                        );
                    } else {
                        self.graph
                            .mark(span_key(e.span()), OccurrenceFlag::PendingDomain);
                    }
                    if !self.pending_nominal_available(left) {
                        // An unresolved Array constructor can still refute
                        // the nominal result (§4.1.2.3); it is not evidence
                        // for a later assignment or an outer consumer.
                        return Ok(Type::Tainted);
                    }
                    return Ok(if super::operators::is_comparison(op, self.f5) {
                        Type::Bool
                    } else {
                        Type::I64
                    });
                }
                if super::operators::is_arithmetic(op, self.f5)
                    && self.pending_join_count() > pending_before_left
                {
                    // ADR §4.1.2.4-5: an internal failed join contaminates
                    // this operand but has not consumed this operator's
                    // domain site. A direct left-domain conflict has.
                    independent_right.expect("independent binary right operand");
                    self.graph
                        .mark(span_key(e.span()), OccurrenceFlag::FailedJoin);
                    self.graph.fact_mut(visit).tainted = true;
                    return Ok(Type::I64);
                }
                if self
                    .graph
                    .has(&span_key(left.span()), OccurrenceFlag::FailedJoin)
                {
                    independent_right.expect("independent binary right operand");
                    self.graph
                        .mark(span_key(e.span()), OccurrenceFlag::FailedJoin);
                    self.graph.fact_mut(visit).tainted = true;
                    return Ok(if super::operators::is_comparison(op, self.f5) {
                        Type::Bool
                    } else {
                        Type::I64
                    });
                }
                if self.graph.fact_mut(visit).tainted {
                    // A child already recorded its own error. Visit the
                    // independent sibling, but suppress the operator error
                    // that would depend on the invalid left value.
                    independent_right.expect("independent binary right operand");
                    self.graph.fact_mut(visit).tainted = true;
                    self.graph
                        .mark(span_key(e.span()), OccurrenceFlag::Deferred);
                    return Ok(if super::operators::is_comparison(op, self.f5) {
                        Type::Bool
                    } else {
                        Type::I64
                    });
                }
                if super::operators::is_arithmetic(op, self.f5) && l != Type::I64 {
                    independent_right.expect("independent binary right operand");
                    return Err(mismatch(
                        Self::primary_span(left),
                        Self::primary_span(left),
                        None,
                        "I64",
                        &l,
                    ));
                }
                unreachable!("binary right operand is prepared by driver")
            }
            Expr::Index { target, index, .. } => {
                let IndexPrepared {
                    target: base,
                    selector: mut precomputed_selector,
                    deferred_child,
                } = index_target.expect("index target is prepared by check_against_context");
                let base = match base {
                    Ok(base) => base,
                    Err(error) => {
                        if let Some(Err(selector_error)) = precomputed_selector.take() {
                            if selector_error.operation == Operation::AnalysisLimit {
                                return Err(selector_error);
                            }
                            self.record_error(selector_error);
                        }
                        return Err(error);
                    }
                };
                if base == Type::Tainted {
                    let outcome = precomputed_selector.take().expect("tainted index selector");
                    if let Err(error) = outcome {
                        if error.operation == Operation::AnalysisLimit {
                            return Err(error);
                        }
                        self.record_error(error);
                    }
                    return Ok(Type::Tainted);
                }
                if base == Type::Never {
                    precomputed_selector.take().expect("never index selector")?;
                    self.graph
                        .mark(span_key(e.span()), OccurrenceFlag::Deferred);
                    self.graph
                        .mark(span_key(e.span()), OccurrenceFlag::Divergent);
                    return Ok(Type::Never);
                }
                if self
                    .graph
                    .has(&span_key(target.span()), OccurrenceFlag::Deferred)
                {
                    if let Some(id) = self.expression_variable(target) {
                        let root = self.root(id);
                        let child = if self.graph.type_nodes[&root].array_child.is_some() {
                            let child =
                                deferred_child.expect("deferred array prepared by index frame");
                            precomputed_selector
                                .take()
                                .expect("deferred array selector")?;
                            if self
                                .graph
                                .has(&span_key(index.span()), OccurrenceFlag::FailedJoin)
                            {
                                // The selector's deferred domain is already
                                // incompatible with its established Array
                                // operand. It cannot prove a projection type.
                                return Ok(Type::Tainted);
                            }
                            child
                        } else {
                            // ADR §4.1.2.3: a selector records a conditional
                            // projection; it does not manufacture an Array
                            // constructor for an unresolved base.
                            let child = self.new_variable(&Self::primary_span(target))?;
                            let selector_variable = self
                                .binding_variable_at_source(index)
                                .filter(|id| self.concrete(*id).is_none());
                            let selector = Arc::new(clone_projection_selector(index));
                            let selector_occurrence_aliases = self
                                .occurrences
                                .map_cloned_expression(index, selector.as_ref());
                            self.graph.register(ObligationKind::Projection(Box::new(
                                PendingProjection {
                                    base: id,
                                    item: child,
                                    base_span: Self::primary_span(target),
                                    result_span: Self::primary_span(e),
                                    selector,
                                    selector_occurrence_aliases,
                                    selector_variable,
                                    checked_without_context: false,
                                    selector_invalid: false,
                                    expected: None,
                                    expected_span: None,
                                    related_expected: false,
                                },
                            )));
                            self.graph.projected_bases.insert(id);
                            child
                        };
                        self.graph.expr_vars.insert(span_key(e.span()), child);
                        self.graph.projection_items.insert(child);
                        self.graph
                            .mark(span_key(e.span()), OccurrenceFlag::Deferred);
                        return Ok(Type::Deferred(child));
                    }
                    if self
                        .graph
                        .has(&span_key(target.span()), OccurrenceFlag::Divergent)
                    {
                        precomputed_selector
                            .take()
                            .expect("divergent index selector")?;
                        self.graph
                            .mark(span_key(e.span()), OccurrenceFlag::Deferred);
                        self.graph
                            .mark(span_key(e.span()), OccurrenceFlag::Divergent);
                        return Ok(Type::Never);
                    }
                }
                if !matches!(base, Type::Array(_)) && internal_state(&base).is_some() {
                    // An invalid or still hypothetical union cannot prove
                    // its result non-indexable. Visit the selector's own
                    // constraints without using the missing Array context.
                    let selector = precomputed_selector.take().expect("union index selector");
                    if let Err(error) = selector {
                        if error.operation == Operation::AnalysisLimit {
                            return Err(error);
                        }
                        self.record_error(error);
                    }
                    return Ok(Type::Tainted);
                }
                let Type::Array(item) = base else {
                    // ADR §4.1.2.3: failure of the base's Array domain does
                    // not suppress an independent error inside the selector.
                    // Without an Array base the I64 selector context is not
                    // activated (RD117).
                    let selector = precomputed_selector
                        .take()
                        .expect("nonarray index selector");
                    if let Err(error) = selector {
                        if error.operation == Operation::AnalysisLimit {
                            return Err(error);
                        }
                        self.record_error(error);
                    }
                    return Err(domain_error(
                        "not_indexable",
                        Self::primary_span(target),
                        Self::primary_span(target),
                        None,
                        "domain:array",
                        &base,
                    ));
                };
                precomputed_selector.take().expect("array index selector")?;
                if self
                    .graph
                    .has(&span_key(index.span()), OccurrenceFlag::FailedJoin)
                {
                    return Ok(Type::Tainted);
                }
                Ok(item.as_ref().clone())
            }
            Expr::Call { target, args, span } => {
                let Some(binding) = self.resolved.calls.get(&(span.start, span.end)) else {
                    let target_ty =
                        call_target.expect("call target is prepared by check_against_context")?;
                    if target_ty == Type::Tainted {
                        self.graph.fact_mut(visit).tainted = true;
                        return Ok(Type::Tainted);
                    }
                    let mut error = names::diagnostic(
                        "NX-E0111",
                        Some("non_callable_target"),
                        Stage::Arguments,
                        Operation::CallBind,
                        Self::primary_span(target),
                    );
                    if let Expr::Name(_, name_span) =
                        crate::frontend::parse::expression_primary(target)
                        && let Some(ResolvedUse::Local(local)) =
                            self.resolved.uses.get(&name_span.start)
                    {
                        error.related_spans.push(RelatedSpan {
                            role: RelatedRole::Declaration,
                            span: local.name_span.clone(),
                        });
                    }
                    return Err(error);
                };
                let signature =
                    names::signature(self.resolved, &binding.target).expect("callable binding");
                if let ResolvedUse::Intrinsic(name) = &binding.target {
                    return self.check_intrinsic(
                        name,
                        args,
                        &binding.slots,
                        span,
                        expected,
                        expected_span,
                        None,
                    );
                }
                // Nonempty resolved calls enter their argument frames before
                // this completion site, one argument at a time.
                debug_assert!(args.is_empty());
                Ok(Type::from(&signature.result))
            }
            Expr::If {
                condition: _,
                then_block,
                otherwise,
                ..
            } => {
                *context_use = if outer_compare {
                    ContextUse::Compare
                } else {
                    ContextUse::Propagated
                };
                let (result, pending_before_if) =
                    if_result.expect("if expression is prepared by check_against_context");
                let (normal, value_failed, join_failed) = result?;
                if join_failed
                    || expected.is_some() && self.pending_join_count() > pending_before_if
                {
                    self.graph
                        .mark(span_key(e.span()), OccurrenceFlag::FailedJoin);
                }
                // ADR §4.1.2.5: an inferred union with a failed join has
                // no value type for ordinary consumers, but its first
                // valid continuation remains a partial witness for an
                // enclosing union.
                if join_failed && expected.is_none() && normal.is_some() {
                    self.graph
                        .mark(span_key(e.span()), OccurrenceFlag::PartialNormal);
                }
                if expected.is_some()
                    && (branch_value_expr(then_block).is_some_and(|value| {
                        self.graph
                            .has(&span_key(value.span()), OccurrenceFlag::ContextRejected)
                    }) || match otherwise.as_deref() {
                        Some(Else::Block(block)) => branch_value_expr(block).is_some_and(|value| {
                            self.graph
                                .has(&span_key(value.span()), OccurrenceFlag::ContextRejected)
                        }),
                        Some(Else::If(value)) => self
                            .graph
                            .has(&span_key(value.span()), OccurrenceFlag::ContextRejected),
                        None => false,
                    })
                {
                    self.graph
                        .mark(span_key(e.span()), OccurrenceFlag::ContextRejected);
                }
                if value_failed {
                    self.graph.fact_mut(visit).tainted = true;
                }
                if self.graph.fact_mut(visit).tainted && normal.is_some() {
                    // §13.9: a clean continuation contributes a constraint
                    // even when another reachable child taints this if.
                    self.graph
                        .mark(span_key(e.span()), OccurrenceFlag::PartialNormal);
                }
                let first_var =
                    branch_value_expr(then_block).and_then(|e| self.expression_variable(e));
                let second_var = match otherwise.as_deref() {
                    Some(Else::Block(b)) => {
                        branch_value_expr(b).and_then(|e| self.expression_variable(e))
                    }
                    Some(Else::If(e)) => self.expression_variable(e),
                    None => None,
                };
                if let Some(id) = first_var.or(second_var)
                    && normal.is_none()
                {
                    self.graph.expr_vars.insert(span_key(e.span()), id);
                    self.graph
                        .mark(span_key(e.span()), OccurrenceFlag::Deferred);
                    return Ok(Type::Deferred(id));
                }
                // A missing branch value is resolved by ownership flow. Until
                // then it cannot supply a concrete type to an outer operator.
                if normal.is_none() {
                    self.graph
                        .mark(span_key(e.span()), OccurrenceFlag::Deferred);
                    self.graph
                        .mark(span_key(e.span()), OccurrenceFlag::Divergent);
                }
                Ok(normal.or_else(|| expected.cloned()).unwrap_or(Type::Never))
            }
            Expr::Match(_) => unreachable!("match expressions complete through their frame"),
        }
    }

    #[allow(clippy::too_many_arguments)] // The continuation preserves the parent's sites.
    #[allow(clippy::too_many_arguments)]
    fn array_next<'a>(
        &mut self,
        _visit: FactId,
        state: &mut ArrayFrameState<'a>,
    ) -> (&'a Expr, OwnedCheckContext) {
        let index = state.next;
        let item = &state.items[index];
        let origin = if index == 0 {
            state.context.expected_span.clone()
        } else {
            state.item_origin.clone()
        };
        let pending_shape = state.pending.first().and_then(|(id, _)| {
            let root = self.root(*id);
            self.graph.type_nodes[&root]
                .array_child
                .map(|_| Type::Deferred(*id))
        });
        let expected = state
            .element_expected
            .as_ref()
            .or_else(|| {
                state
                    .item_type
                    .as_ref()
                    .filter(|ty| internal_state(ty).is_none())
            })
            .or(pending_shape.as_ref())
            .cloned();
        state.current_expected = expected.clone();
        state.current_origin = origin.clone();
        (
            item,
            OwnedCheckContext {
                expected,
                expected_span: origin.clone(),
                check_here: true,
                anchor: None,
                outer_compare: false,
                related_expected: origin.is_some(),
                deferred_element: state.element_expected.is_some() || pending_shape.is_some(),
            },
        )
    }

    fn array_accept(
        &mut self,
        visit: FactId,
        state: &mut ArrayFrameState<'_>,
        outcome: Result<Type, Box<Diagnostic>>,
    ) -> Result<(), Box<Diagnostic>> {
        let ArrayFrameState {
            expr: e,
            items,
            span,
            next,
            current_expected,
            current_origin,
            item_type,
            item_origin,
            pending,
            diverges,
            invalid,
            ..
        } = state;
        let item = &items[*next];
        let item_expected = current_expected.as_ref();
        let origin = current_origin.as_ref();
        let item_tainted = self.graph.fact_mut(visit).tainted
            || self
                .graph
                .has(&span_key(item.span()), OccurrenceFlag::FailedJoin);
        self.graph.fact_mut(visit).tainted = item_tainted;
        let got = match outcome {
            Ok(got) => got,
            Err(error) => {
                // charge_solver has already retained the first
                // denied global charge. The compilation barrier
                // emits it once even if this return is swallowed
                // by a tainted enclosing expression. Swallowing
                // it here could change only the internal inferred
                // table after exhaustion; NX-E0003 still blocks
                // every consumer of executable inferred types.
                if error.operation == Operation::AnalysisLimit {
                    return Err(error);
                }
                // A child can report one internal error and
                // return another independent one. Its taint
                // suppresses only a comparison of the child's
                // unusable result with this array's element.
                self.record_error(error);
                // `invalid` forces the constructor to return
                // Tainted after visiting its other elements.
                // Each child has an isolated taint bit, so setting
                // the parent bit here cannot affect an independent
                // child or the final result.
                *invalid = true;
                return Ok(());
            }
        };
        if item_tainted {
            *invalid = true;
            return Ok(());
        }
        if self
            .graph
            .has(&span_key(item.span()), OccurrenceFlag::PendingDomain)
            && let Some(wanted) = item_expected
            && internal_state(wanted).is_none()
            && internal_state(&got).is_none()
            && compare_source_site(
                wanted,
                &got,
                origin.cloned().unwrap_or_else(|| Self::primary_span(item)),
                Self::primary_span(item),
                UnifyPolicy {
                    branch: false,
                    related_expected: origin.is_some(),
                },
            )
            .is_some()
        {
            // Closure owns the child's diagnostic. The array
            // cannot lend its provisional element type to a
            // sibling while that obligation remains open.
            *invalid = true;
            return Ok(());
        }
        *diverges |= self
            .graph
            .has(&span_key(item.span()), OccurrenceFlag::Divergent);
        if item_expected.is_some() {
            // The first concrete element fixes the element type.
            // Each later element is checked in its own subtree;
            // an invalid element cannot replace that type.
            // A result whose domain is still pending cannot be
            // the sole witness for an independent later site.
            // Prefer the next valid element of the same type.
            if item_origin.as_ref().is_some_and(|origin| {
                self.graph
                    .has(&span_key(origin), OccurrenceFlag::PendingDomain)
            }) && !self
                .graph
                .has(&span_key(item.span()), OccurrenceFlag::PendingDomain)
            {
                *item_origin = Some(Self::primary_span(item));
            }
            return Ok(());
        }
        if self
            .graph
            .has(&span_key(item.span()), OccurrenceFlag::Deferred)
        {
            if let Some(id) = self.expression_variable(item) {
                self.require_copyable(id, &Self::primary_span(item), false)?;
                if let Some(expected_item) = item_type.as_ref() {
                    // item_type is assigned only from a
                    // non-deferred result (or a concrete source
                    // annotation). check_against just compared
                    // this child with that concrete item type.
                    // Thus both sides cannot be concrete and
                    // incompatible here; branch-mode's special
                    // mismatch arm is unreachable.
                    self.unify(
                        visit,
                        expected_item.clone(),
                        Type::Deferred(id),
                        origin.cloned().unwrap_or_else(|| span.clone()),
                        Self::primary_span(item),
                        Self::primary_span(item),
                        false,
                    )?;
                } else {
                    if let Some((first, first_origin)) = pending.first() {
                        // Deferred siblings are constrained at
                        // the second occurrence, in source order.
                        // Waiting until the array closes would
                        // move an eventual conflict to the whole
                        // constructor (§4.1.2.6).
                        let before = self.pending_join_count();
                        self.unify(
                            visit,
                            Type::Deferred(*first),
                            Type::Deferred(id),
                            first_origin.clone(),
                            Self::primary_span(item),
                            Self::primary_span(item),
                            false,
                        )?;
                        if self.pending_join_count() > before {
                            self.graph
                                .mark(span_key(e.span()), OccurrenceFlag::FailedJoin);
                            // The rejected sibling cannot join
                            // the pending witness set. A later
                            // element is compared with the first
                            // valid one (§§4.1.2.4–5, 13.9).
                            return Ok(());
                        }
                    }
                    if pending.is_empty() {
                        *item_origin = Some(Self::primary_span(item));
                    }
                    pending.push((id, Self::primary_span(item)));
                }
            }
            return Ok(());
        }
        if item_type.is_none() {
            let concrete_origin = Self::primary_span(item);
            let mut first_conflict = false;
            for (id, source_origin) in pending.iter() {
                let before_conflict = self.pending_join_count();
                let projected_reference = self.graph.projection_items.contains(id);
                let source_type = projected_reference
                    .then(|| self.concrete(*id).map(|(ty, _)| ty))
                    .flatten();
                let equality = self.unify(
                    visit,
                    Type::Deferred(*id),
                    got.clone(),
                    source_origin.clone(),
                    concrete_origin.clone(),
                    Self::primary_span(item),
                    false,
                );
                match equality {
                    Ok(_) => {}
                    Err(error) if error.operation == Operation::AnalysisLimit => {
                        return Err(error);
                    }
                    Err(error) => {
                        // A rejected element does not replace the
                        // first witness. Later elements retain
                        // independent equality sites (§4.1.2.4).
                        let error = source_type
                            .as_ref()
                            .and_then(|source_type| {
                                compare_source_site(
                                    source_type,
                                    &got,
                                    source_origin.clone(),
                                    Self::primary_span(item),
                                    UnifyPolicy {
                                        branch: false,
                                        related_expected: true,
                                    },
                                )
                            })
                            .unwrap_or(error);
                        self.record_error(error);
                        first_conflict = true;
                    }
                }
                first_conflict |= self.pending_join_count() > before_conflict;
            }
            if first_conflict {
                // ADR §4.1.2.2,4: a rejected shape equality
                // cannot replace the first array element as the
                // reference for later siblings.
                *invalid = true;
                return Ok(());
            }
            pending.clear();
            *item_type = Some(got);
            // ADR §4.1.2.4,6: the first valid array element is
            // the source witness for later element checks.
            *item_origin = Some(concrete_origin);
        } else if let Some(prior) = item_type.clone() {
            let prior_origin = origin.cloned().unwrap_or_else(|| span.clone());
            let concrete_origin = Self::primary_span(item);
            match self.unify(
                visit,
                prior,
                got,
                prior_origin,
                concrete_origin,
                Self::primary_span(item),
                false,
            ) {
                Ok(ty) => *item_type = Some(ty),
                // Returning the quota here avoids recording more
                // partial inferred entries. If swallowed, the
                // global quota_error still emits the same sole
                // NX-E0003, and that exhausted inferred table
                // is never consumed for executable IR.
                Err(error) if error.operation == Operation::AnalysisLimit => {
                    return Err(error);
                }
                Err(error) => {
                    // `invalid` returns Tainted at the array
                    // boundary, which sets the same parent taint.
                    // Siblings reset their own taint bit.
                    self.record_error(error);
                    self.graph.fact_mut(visit).tainted = true;
                    *invalid = true;
                }
            }
        }
        Ok(())
    }

    fn array_finish(&mut self, state: ArrayFrameState<'_>) -> Result<Type, Box<Diagnostic>> {
        let ArrayFrameState {
            expr: e,
            span,
            item_type,
            pending,
            diverges,
            invalid,
            ..
        } = state;
        if invalid {
            return Ok(Type::Tainted);
        }
        if let Some(item) = item_type {
            if contains_resource(&item) {
                return Err(resource_array(span.clone()));
            }
            let resolved_item = self
                .resolved_syntax_type(&item)
                .map(|ty| Type::from(&ty))
                .unwrap_or_else(|| item.clone());
            self.check_depth(&Type::Array(Arc::new(resolved_item)), span)?;
            if diverges {
                self.graph
                    .mark(span_key(e.span()), OccurrenceFlag::Divergent);
            }
            Ok(Type::Array(Arc::new(item)))
        } else {
            self.graph
                .mark(span_key(e.span()), OccurrenceFlag::Deferred);
            if diverges {
                self.graph
                    .mark(span_key(e.span()), OccurrenceFlag::Divergent);
            }
            if let Some((child, _)) = pending.first() {
                let id = self.new_variable(span)?;
                self.graph.type_node_mut(&id).unwrap().array_child = Some(*child);
                self.graph.type_node_mut(&id).unwrap().array_origin = Some(span.clone());
                self.graph
                    .type_nodes
                    .get_mut(&id)
                    .unwrap()
                    .array_constructed = true;
                self.check_one_inferred_depth(id, span)?;
                self.graph.expr_vars.insert(span_key(e.span()), id);
                // Every constructor owns an element-resolution
                // obligation, even when the pending element came
                // from another constructor or a projection (ADR
                // §4.1.2.2, RD122/RD127).
                self.graph.register(ObligationKind::Constructor {
                    child: *child,
                    array: id,
                    span: span.clone(),
                    has_element_witness: true,
                });
                Ok(Type::Array(Arc::new(Type::Deferred(*child))))
            } else if diverges {
                let child = self.new_variable(span)?;
                let array = self.new_variable(span)?;
                self.graph.type_node_mut(&array).unwrap().array_child = Some(child);
                self.graph.type_node_mut(&array).unwrap().array_origin = Some(span.clone());
                self.graph
                    .type_nodes
                    .get_mut(&array)
                    .unwrap()
                    .array_constructed = true;
                self.require_copyable(child, span, false)?;
                self.graph.expr_vars.insert(span_key(e.span()), array);
                self.graph.register(ObligationKind::Constructor {
                    child,
                    array,
                    span: span.clone(),
                    has_element_witness: false,
                });
                Ok(Type::Array(Arc::new(Type::Deferred(child))))
            } else {
                Ok(Type::Tainted)
            }
        }
    }

    #[allow(clippy::too_many_arguments)] // The deferred operator retains both source sites.
    fn finish_deferred_binary_right(
        &mut self,
        visit: FactId,
        e: &Expr,
        op: &str,
        left: &Expr,
        right: &Expr,
        l: Type,
        right_result: Result<Type, Box<Diagnostic>>,
        pending_before_operator: usize,
        expected: Option<&Type>,
        expected_span: Option<&Span>,
    ) -> Result<Type, Box<Diagnostic>> {
        let left_variable = self.expression_variable(left);
        let right_ty = match right_result {
            Ok(ty) => ty,
            Err(error) if error.operation == Operation::AnalysisLimit => {
                return Err(error);
            }
            Err(error) if super::operators::is_comparison(op, self.f5) => {
                self.record_error(error);
                if let Some(domain_error) =
                    self.comparison_domain_error(visit, op, left, right, &l)?
                {
                    self.record_error(domain_error);
                }
                self.graph.fact_mut(visit).tainted = true;
                return Ok(Type::Tainted);
            }
            Err(error) => return Err(error),
        };
        if self.graph.has(
            &span_key(&Self::primary_span(right)),
            OccurrenceFlag::PendingDomain,
        ) {
            if super::operators::is_comparison(op, self.f5) {
                if let Some(domain_error) =
                    self.comparison_domain_error(visit, op, left, right, &l)?
                {
                    self.record_error(domain_error);
                }
                if let (Some((variable, domain)), Some(found)) = (
                    self.pending_operation_guard(right),
                    self.pending_operation_type(right),
                ) {
                    self.graph
                        .register(ObligationKind::GuardedComparison(Box::new(
                            PendingGuardedComparison {
                                variable,
                                domain,
                                expected: left_variable.map(Type::Deferred).unwrap_or(l.clone()),
                                found,
                                expected_origin: Self::primary_span(left),
                                found_site: Self::primary_span(right),
                                result: Self::primary_span(e),
                                consumer_expected: expected.cloned(),
                                consumer_origin: expected_span.cloned(),
                                domain_error_if_equal: None,
                                additional_domains: Vec::new(),
                                projections: Vec::new(),
                                dependent_results: Vec::new(),
                                require_resolved: false,
                                conditional_bindings: Vec::new(),
                            },
                        )));
                }
            }
            if super::operators::is_arithmetic(op, self.f5)
                && let Some((variable, domain)) = self.pending_operation_guard(right)
            {
                self.defer_nominal_domain(
                    variable,
                    domain,
                    e.span(),
                    &Type::I64,
                    expected,
                    expected_span,
                    Some(&Self::primary_span(right)),
                );
            }
            self.graph
                .mark(span_key(e.span()), OccurrenceFlag::PendingDomain);
            if !self.graph.fact_mut(visit).tainted && self.pending_nominal_available(right) {
                return Ok(if super::operators::is_comparison(op, self.f5) {
                    Type::Bool
                } else {
                    Type::I64
                });
            }
            return Ok(Type::Tainted);
        }
        if self.graph.fact_mut(visit).tainted
            || self
                .graph
                .has(&span_key(right.span()), OccurrenceFlag::FailedJoin)
        {
            if super::operators::is_comparison(op, self.f5)
                && let Some(domain_error) =
                    self.comparison_domain_error(visit, op, left, right, &l)?
            {
                self.record_error(domain_error);
            }
            return Ok(Type::Tainted);
        }
        if super::operators::is_comparison(op, self.f5) {
            // ADR §4.1.2.4: both operands are synthesized before
            // equality; only then is the common domain tested.
            let right_variable = self.expression_variable(right);
            let left_type = left_variable.map(Type::Deferred).unwrap_or(l.clone());
            let right_type = right_variable
                .map(Type::Deferred)
                .unwrap_or(right_ty.clone());
            let left_determined = self.type_determined(&left_type);
            let right_determined = self.type_determined(&right_type);
            let found_site = if left_determined || !right_determined {
                Self::primary_span(left)
            } else {
                Self::primary_span(right)
            };
            let operator = self.operator_span(left.span(), right.span(), op);
            let common = self.unify(
                visit,
                left_type,
                right_type,
                Self::primary_span(left),
                Self::primary_span(right),
                Self::primary_span(right),
                false,
            )?;
            if self.pending_join_count() > pending_before_operator {
                self.graph
                    .mark(span_key(e.span()), OccurrenceFlag::FailedJoin);
                return Ok(Type::Bool);
            }
            let variable = left_variable.or(right_variable);
            let complete = variable
                .and_then(|id| self.concrete(id).map(|(ty, _)| ty))
                .or_else(|| (internal_state(&common).is_none()).then_some(common));
            let domain = if super::operators::is_relational(op, self.f5) {
                "I64"
            } else {
                "domain:copyable_data"
            };
            if let Some(ty) = complete {
                let valid = if super::operators::is_relational(op, self.f5) {
                    ty == Type::I64
                } else {
                    comparable_data(&ty)
                };
                if !valid {
                    return Err(mismatch(operator, found_site, None, domain, &ty));
                }
            } else if let Some(id) = variable {
                let root = self.root(id);
                if super::operators::is_relational(op, self.f5)
                    && self.graph.type_nodes[&root].array_child.is_none()
                {
                    self.unify_with_expected_origin(
                        visit,
                        Type::I64,
                        Type::Deferred(id),
                        operator.clone(),
                        operator.clone(),
                        operator,
                        UnifyPolicy {
                            branch: false,
                            related_expected: false,
                        },
                    )?;
                } else {
                    self.graph.register(ObligationKind::BinaryDomain(Box::new(
                        PendingBinaryDomain {
                            variable: id,
                            operator,
                            found: found_site,
                            domain,
                            result: e.span().clone(),
                            conditional_bindings: Vec::new(),
                        },
                    )));
                    self.defer_nominal_domain(
                        id,
                        domain,
                        e.span(),
                        &Type::Bool,
                        expected,
                        expected_span,
                        None,
                    );
                }
            }
            return Ok(Type::Bool);
        }
        if let Some(id) = left_variable {
            // < and == returned above; only + and - reach here.
            self.unify(
                visit,
                Type::Deferred(id),
                Type::I64,
                Self::primary_span(left),
                Self::primary_span(left),
                Self::primary_span(left),
                false,
            )?;
        }
        if self.pending_join_count() > pending_before_operator {
            self.graph
                .mark(span_key(e.span()), OccurrenceFlag::FailedJoin);
        }
        Ok(Type::I64)
    }

    #[allow(clippy::too_many_arguments)] // The continuation preserves the parent's sites.
    fn finish_binary_right(
        &mut self,
        visit: FactId,
        e: &Expr,
        op: &str,
        left: &Expr,
        right: &Expr,
        left_type: &Type,
        right_result: Result<Type, Box<Diagnostic>>,
        pending_before_right: usize,
        expected: Option<&Type>,
        expected_span: Option<&Span>,
    ) -> Result<Type, Box<Diagnostic>> {
        if super::operators::is_arithmetic(op, self.f5) {
            right_result?;
            if self.graph.fact_mut(visit).tainted {
                return Ok(Type::Tainted);
            }
            if self
                .graph
                .has(&span_key(right.span()), OccurrenceFlag::FailedJoin)
            {
                self.graph
                    .mark(span_key(e.span()), OccurrenceFlag::FailedJoin);
                return Ok(Type::I64);
            }
            return Ok(Type::I64);
        }
        if let Err(error) = right_result {
            if error.operation == Operation::AnalysisLimit
                || self
                    .graph
                    .has(&span_key(right.span()), OccurrenceFlag::ContextRejected)
            {
                return Err(error);
            }
            self.record_error(error);
            if let Some(domain_error) =
                self.comparison_domain_error(visit, op, left, right, left_type)?
            {
                self.record_error(domain_error);
            }
            self.graph.fact_mut(visit).tainted = true;
            return Ok(Type::Tainted);
        }
        if self.graph.has(
            &span_key(&Self::primary_span(right)),
            OccurrenceFlag::PendingDomain,
        ) && let Some((variable, domain)) = self.pending_operation_guard(right)
        {
            let domain_error_if_equal =
                self.comparison_domain_error(visit, op, left, right, left_type)?;
            if let Some(found) = self.pending_operation_type(right) {
                self.graph
                    .register(ObligationKind::GuardedComparison(Box::new(
                        PendingGuardedComparison {
                            variable,
                            domain,
                            expected: left_type.clone(),
                            found,
                            expected_origin: Self::primary_span(left),
                            found_site: Self::primary_span(right),
                            result: Self::primary_span(e),
                            consumer_expected: expected.cloned(),
                            consumer_origin: expected_span.cloned(),
                            domain_error_if_equal,
                            additional_domains: Vec::new(),
                            projections: Vec::new(),
                            dependent_results: Vec::new(),
                            require_resolved: false,
                            conditional_bindings: Vec::new(),
                        },
                    )));
            }
            self.graph
                .mark(span_key(e.span()), OccurrenceFlag::PendingDomain);
            if !self.graph.fact_mut(visit).tainted && self.pending_nominal_available(right) {
                return Ok(Type::Bool);
            }
            return Ok(Type::Tainted);
        }
        if self.graph.fact_mut(visit).tainted {
            if let Some(domain_error) =
                self.comparison_domain_error(visit, op, left, right, left_type)?
            {
                self.record_error(domain_error);
            }
            return Ok(Type::Tainted);
        }
        if self.pending_join_count() > pending_before_right {
            let internal_right_join = self.pending_joins().skip(pending_before_right).any(|join| {
                right.span().start <= join.left_origin.start
                    && join.left_origin.end <= right.span().end
            });
            if internal_right_join {
                if let Some(domain_error) =
                    self.comparison_domain_error(visit, op, left, right, left_type)?
                {
                    self.record_error(domain_error);
                }
            } else {
                let domain = PendingComparisonDomain {
                    left: self
                        .expression_variable(left)
                        .map(Type::Deferred)
                        .unwrap_or_else(|| left_type.clone()),
                    operator: self.operator_span(left.span(), right.span(), op),
                    found: Self::primary_span(left),
                    domain: if super::operators::is_relational(op, self.f5) {
                        "I64"
                    } else {
                        "domain:copyable_data"
                    },
                };
                if let Some(join) = self.pending_joins_mut().nth(pending_before_right) {
                    join.comparison_domain_if_unresolved = Some(domain);
                }
            }
            self.graph
                .mark(span_key(e.span()), OccurrenceFlag::FailedJoin);
            return Ok(Type::Bool);
        }
        if self
            .graph
            .has(&span_key(right.span()), OccurrenceFlag::Deferred)
        {
            if let Some(error) = self.comparison_domain_error(visit, op, left, right, left_type)? {
                return Err(error);
            }
            return Ok(Type::Bool);
        }
        if let Some(error) = self.comparison_domain_error(visit, op, left, right, left_type)? {
            return Err(error);
        }
        Ok(Type::Bool)
    }

    fn option_none(
        &mut self,
        span: &Span,
        expected: Option<&Type>,
    ) -> Result<Type, Box<Diagnostic>> {
        if let Some(complete @ Type::Union(UnionKind::Option, _, None)) = expected {
            return Ok(complete.clone());
        }
        let result = self.new_variable(span)?;
        let children = self.require_union(result, UnionKind::Option, span)?;
        let some = children.variants[1]
            .payload
            .ok_or_else(|| unresolved_type(span.clone()))?;
        self.graph
            .type_nodes
            .get_mut(&result)
            .unwrap()
            .variant_constructed = true;
        self.graph.expr_vars.insert(span_key(span), result);
        self.graph.mark(span_key(span), OccurrenceFlag::Deferred);
        self.graph.register(ObligationKind::VariantConstructor {
            union: result,
            span: span.clone(),
        });
        Ok(Type::option(Arc::new(Type::Deferred(some))))
    }

    fn finish_selection(
        &mut self,
        selection: &parse::Match,
        selected: &Type,
        joined: &Option<(Type, Span)>,
        expression: bool,
        expected: Option<&Type>,
    ) -> Result<Type, Box<Diagnostic>> {
        let selected = self.selection_type(selected, 0);
        let mut previous: Vec<&Pattern> = Vec::new();
        for arm in &selection.arms {
            if crate::check::match_coverage::subsumed(&selected, &arm.pattern, &previous) {
                let mut diagnostic = names::diagnostic(
                    "NX-E1304",
                    Some("unreachable_arm"),
                    Stage::Types,
                    Operation::TypesCheck,
                    arm.pattern.span().clone(),
                );
                for pattern in &previous {
                    diagnostic.related_spans.push(RelatedSpan {
                        role: RelatedRole::Context,
                        span: pattern.span().clone(),
                    });
                }
                return Err(diagnostic);
            }
            if arm.guard.is_none() {
                previous.push(&arm.pattern);
            }
        }
        if let Some(witness) = crate::check::match_coverage::missing(&selected, &previous) {
            let mut diagnostic = names::diagnostic(
                "NX-E1304",
                Some("non_exhaustive"),
                Stage::Types,
                Operation::TypesCheck,
                selection.keyword_span.clone(),
            );
            diagnostic
                .data
                .insert("missing_pattern".into(), DataValue::String(witness));
            return Err(diagnostic);
        }
        Ok(if expression {
            joined.as_ref().map_or_else(
                || expected.cloned().unwrap_or(Type::Never),
                |(ty, _)| {
                    self.resolved_syntax_type(ty)
                        .map(|resolved| Type::from(&resolved))
                        .unwrap_or_else(|| ty.clone())
                },
            )
        } else {
            Type::Unit
        })
    }

    fn pattern_constructor_error(
        &self,
        name: &str,
        kind: UnionKind,
        actual: Option<&Type>,
        span: &Span,
    ) -> Box<Diagnostic> {
        let partial = actual.is_none_or(|ty| pretty(ty).is_none());
        let mut diagnostic = names::diagnostic(
            "NX-E1325",
            Some(if partial {
                "constructor_shape"
            } else {
                "constructor_type"
            }),
            Stage::Types,
            Operation::TypesCheck,
            span.clone(),
        );
        diagnostic.data.insert(
            "constructor".into(),
            DataValue::String(format!("prelude:{name}")),
        );
        diagnostic.data.insert(
            if partial {
                "scrutinee_constructor"
            } else {
                "scrutinee_type"
            }
            .into(),
            DataValue::String(if partial {
                match kind {
                    UnionKind::Option => "Option".into(),
                    UnionKind::Result => "Result".into(),
                }
            } else {
                pretty(actual.unwrap()).unwrap_or_default()
            }),
        );
        if partial {
            diagnostic.message = "Constructor incompatible con la forma seleccionada.".into();
        }
        diagnostic
    }

    fn require_match_pattern_shapes(
        &mut self,
        selection: &parse::Match,
        selected: &Type,
    ) -> Result<(), Box<Diagnostic>> {
        for arm in &selection.arms {
            let mut pattern = &arm.pattern;
            let mut current = selected.clone();
            while let Pattern::Constructor { name, args, span } = pattern {
                let kind = match name.as_str() {
                    "Some" | "None" => UnionKind::Option,
                    "Ok" | "Err" => UnionKind::Result,
                    _ => break,
                };
                let Some(tag) = kind.variant_tag(name) else {
                    break;
                };
                let payload = if let Type::Deferred(id) = &current {
                    let root = self.root(*id);
                    if let Some((concrete, _)) = self.concrete(root) {
                        let Some(union) = concrete_union(&concrete).filter(|u| u.kind == kind)
                        else {
                            return Err(self.pattern_constructor_error(
                                name,
                                kind,
                                Some(&concrete),
                                span,
                            ));
                        };
                        union
                            .variant(tag)
                            .and_then(|variant| variant.payload)
                            .cloned()
                    } else {
                        if let Some(existing) = self.graph.type_nodes[&root].variant_children
                            && existing.kind != kind
                        {
                            return Err(self.pattern_constructor_error(
                                name,
                                existing.kind,
                                None,
                                span,
                            ));
                        }
                        if self.graph.type_nodes[&root].array_child.is_some() {
                            break;
                        }
                        let is_new = self.graph.type_nodes[&root].variant_children.is_none();
                        let children = self.require_union(root, kind, span)?;
                        if is_new {
                            self.graph.register(ObligationKind::PatternConstructor {
                                union: root,
                                span: span.clone(),
                            });
                        }
                        children
                            .variant(tag)
                            .and_then(|variant| variant.payload)
                            .map(Type::Deferred)
                    }
                } else if let Some(union) = concrete_union(&current) {
                    if union.kind != kind {
                        return Err(self.pattern_constructor_error(
                            name,
                            union.kind,
                            Some(&current),
                            span,
                        ));
                    }
                    union
                        .variant(tag)
                        .and_then(|variant| variant.payload)
                        .cloned()
                } else {
                    break;
                };
                let Some(arg) = args.first() else { break };
                let Some(payload) = payload else { break };
                current = payload;
                pattern = arg;
            }
        }
        Ok(())
    }

    fn pattern_type(
        &mut self,
        pattern: &Pattern,
        selected: &Type,
        origin: &Span,
    ) -> Result<(), Box<Diagnostic>> {
        let mut pattern = pattern;
        let mut selected = selected.clone();
        loop {
            self.charge_solver(pattern.span())?;
            match pattern {
                Pattern::Wildcard(_) => return Ok(()),
                Pattern::Integer(_, span) => {
                    if selected == Type::I64 || pretty(&selected).is_none() {
                        return Ok(());
                    } else {
                        return Err(mismatch(
                            span.clone(),
                            span.clone(),
                            Some(origin.clone()),
                            &pretty(&selected).unwrap_or_default(),
                            &Type::I64,
                        ));
                    }
                }
                Pattern::Bool(_, span) => {
                    if selected == Type::Bool || pretty(&selected).is_none() {
                        return Ok(());
                    } else {
                        return Err(mismatch(
                            span.clone(),
                            span.clone(),
                            Some(origin.clone()),
                            &pretty(&selected).unwrap_or_default(),
                            &Type::Bool,
                        ));
                    }
                }
                Pattern::Name { name, span, .. }
                    if name == "None"
                        && matches!(
                            self.selection_type(&selected, 0),
                            Type::Union(UnionKind::Option, ..)
                        ) =>
                {
                    return Ok(());
                }
                Pattern::Name { span, .. } => {
                    self.graph.locals.insert(
                        span.start,
                        Known {
                            ty: selected.clone(),
                            origin: origin.clone(),
                            variable: if let Type::Deferred(id) = selected {
                                Some(id)
                            } else {
                                None
                            },
                            invalid: false,
                            nominal_sources: HashSet::new(),
                        },
                    );
                    return Ok(());
                }
                Pattern::Constructor { name, args, span } => {
                    let kind = match name.as_str() {
                        "Some" | "None" => UnionKind::Option,
                        "Ok" | "Err" => UnionKind::Result,
                        _ => return Ok(()), // names.resolve owns unknown constructors.
                    };
                    let shaped = self.selection_type(&selected, 0);
                    let Some(descriptor) =
                        concrete_union(&shaped).filter(|union| union.kind == kind)
                    else {
                        if pretty(&shaped).is_none() {
                            // The scrutinee's unresolved constructor reports NX-E0113.
                            return Ok(());
                        }
                        let mut diagnostic = names::diagnostic(
                            "NX-E1325",
                            Some("constructor_type"),
                            Stage::Types,
                            Operation::TypesCheck,
                            span.clone(),
                        );
                        diagnostic.data.insert(
                            "constructor".into(),
                            DataValue::String(format!("prelude:{name}")),
                        );
                        diagnostic.data.insert(
                            "scrutinee_type".into(),
                            DataValue::String(pretty(&shaped).unwrap_or_default()),
                        );
                        return Err(diagnostic);
                    };
                    let Some(tag) = kind.variant_tag(name) else {
                        return Ok(());
                    };
                    let Some(variant) = descriptor.variant(tag) else {
                        return Ok(());
                    };
                    let arity = usize::from(variant.payload.is_some());
                    if args.len() != arity {
                        let mut diagnostic = names::diagnostic(
                            "NX-E1325",
                            Some("constructor_arity"),
                            Stage::Types,
                            Operation::TypesCheck,
                            span.clone(),
                        );
                        diagnostic.data.insert(
                            "constructor".into(),
                            DataValue::String(format!("prelude:{name}")),
                        );
                        diagnostic
                            .data
                            .insert("expected_arity".into(), DataValue::UInt(arity as u64));
                        diagnostic
                            .data
                            .insert("found_arity".into(), DataValue::UInt(args.len() as u64));
                        return Err(diagnostic);
                    }
                    if let (Some(payload), Some(arg)) = (variant.payload, args.first()) {
                        pattern = arg;
                        selected = payload.clone();
                        continue;
                    }
                    return Ok(());
                }
            }
        }
    }

    fn if_finish_join(
        &mut self,
        progress: &mut IfProgress<'_>,
        second: Option<Type>,
        second_failed: bool,
        second_anchored: bool,
    ) -> Result<(Option<Type>, bool, bool), Box<Diagnostic>> {
        let condition = progress.condition;
        let then_block = progress.then_block;
        let otherwise = progress.otherwise;
        let expected = progress.branch_context.expected.as_ref();
        let first = progress.first.clone();
        let first_failed = progress.first_failed;
        let first_anchored = progress.first_anchored;
        let first_partial = progress.first_partial;
        let first_for_anchor = progress.first_for_anchor.clone();
        let anchor_error = progress.anchor_error.take();
        let anchor_pending = progress.anchor_pending;
        let first_var = branch_value_expr(then_block).and_then(|e| self.expression_variable(e));
        let second_var = match otherwise {
            Some(Else::Block(b)) => branch_value_expr(b).and_then(|e| self.expression_variable(e)),
            Some(Else::If(e)) => self.expression_variable(e),
            None => None,
        };
        let first_join = first_var.map(Type::Deferred).or_else(|| first.clone());
        let second_join = second_var.map(Type::Deferred).or_else(|| second.clone());
        let right_source = match otherwise {
            Some(Else::Block(b)) => branch_value_expr(b),
            Some(Else::If(e)) => Some(e.as_ref()),
            None => None,
        };
        // If the second branch already failed and both sides are fully
        // determined, its partial type cannot resolve anything. Rejoining
        // that type would repeat its existing diagnostic at finalization.
        // §4.7: a concrete expected type is checked at each branch operand.
        // A second join would compare those operands again and can blame a
        // valid sibling for a constraint already rejected at the other site.
        let first_root = first_var.map(|id| self.root(id));
        let first_has_failed_constructor =
            first_root.is_some_and(|id| self.graph.failed_constructor(id));
        let first_unresolved = first_var
            .map(|id| self.concrete(id).is_none())
            .unwrap_or_else(|| {
                first_join
                    .as_ref()
                    .is_some_and(|ty| matches!(internal_state(ty), Some(Type::Deferred(_))))
            });
        let second_unresolved = second_var
            .map(|id| self.concrete(id).is_none())
            .unwrap_or_else(|| {
                second_join
                    .as_ref()
                    .is_some_and(|ty| matches!(internal_state(ty), Some(Type::Deferred(_))))
            });
        let needs_deferred_join = expected.is_some()
            && first_join.is_some()
            && second_join.is_some()
            && matches!((&first_join, &second_join), (Some(left), Some(right))
                if matches!(internal_state(left), Some(Type::Deferred(_)))
                    || matches!(internal_state(right), Some(Type::Deferred(_))))
            && (!first_has_failed_constructor || second_failed)
            && (!second_failed || first_unresolved || second_unresolved);
        // ADR §4.1.2.4–5: a branch that already failed its distributed
        // context cannot participate in a later inferred join. Its valid
        // sibling keeps its own constraints without rechecking the failure.
        let skip_failed_context_join = (first_failed && !first_partial)
            || (second_failed && !first_anchored)
            || (needs_deferred_join && first_failed);
        let pending_before_join = self.pending_join_count();
        let join = if anchor_error.is_none()
            && !anchor_pending
            && !skip_failed_context_join
            && (expected.is_none() || needs_deferred_join)
            && let (Some(left), Some(right)) = (first_join, second_join)
        {
            let left_origin = branch_value_expr(then_block)
                .map(|value| self.normal_witness_span(value))
                .unwrap_or_else(|| Self::primary_span(condition));
            let right_origin = right_source
                .map(|value| self.normal_witness_span(value))
                .unwrap_or_else(|| Self::primary_span(condition));
            let outcome = self.unify_independent(
                left,
                right,
                left_origin,
                right_origin.clone(),
                right_origin,
                expected.is_none(),
            );
            if needs_deferred_join {
                // Swallowing AnalysisLimit here can change the internal
                // inferred table after exhaustion, but the global quota
                // barrier still emits NX-E0003 and blocks lowering.
                match outcome {
                    // The first denied charge is also stored globally by
                    // charge_solver and emitted at the report boundary;
                    // this propagation only avoids more local traversal.
                    Err(error) if error.operation == Operation::AnalysisLimit => Err(error),
                    _ => Ok(()),
                }
            } else {
                outcome.map(|_| ())
            }
        } else {
            Ok(())
        };
        // A failed branch may retain a partial inferred type. Joining it can
        // still resolve an earlier deferred constructor before diagnostics
        // are finalized. Reordering this check after join? changes only the
        // inferred table once a quota has already exhausted analysis; that
        // table is not consumed when NX-E0003 prevents executable IR.
        if let Some(error) = anchor_error {
            // §13.9: visit the later branch before reporting the first
            // incompatible continuation. Its independent child errors must
            // survive even though this join has already failed.
            self.record_error(error);
            return Ok((first.or(second), true, true));
        }
        if anchor_pending {
            return Ok((first.or(second), true, true));
        }
        if first_failed || second_failed {
            if first_anchored && first_failed || second_anchored && second_failed {
                // The comparison was still visited for solver accounting.
                // Its queued conflict depends on a branch that already owns
                // the failure (F0-0002 §13.9), so it has no source site of
                // its own at the final drain.
                self.truncate_pending_joins(pending_before_join);
            }
            // §13.9 retains a later independent branch error even when the
            // earlier branch has an invalid child. The first branch can still
            // supply a valid partial type; the second branch's own failure
            // cannot be used to accuse that same second operand again.
            if first_failed
                && !second_failed
                && !first_anchored
                && let Err(error) = join
            {
                if error.operation == Operation::AnalysisLimit {
                    return Err(error);
                }
                self.record_error(error);
            }
            return Ok((
                if first_failed && !first_partial {
                    second
                        .or_else(|| {
                            (!second_failed)
                                .then(|| second_var.map(Type::Deferred))
                                .flatten()
                        })
                        .or(first)
                } else {
                    first.or(second)
                },
                true,
                first_anchored && first_failed || second_anchored && second_failed,
            ));
        }
        join?;
        let pending_join = self.pending_join_count() > pending_before_join;
        Ok((
            if pending_join {
                // ADR §4.1.2.5: retain the first valid continuation as
                // partial evidence for an enclosing union. A later
                // concrete continuation cannot replace its source site.
                first_for_anchor.or(first).or(second)
            } else {
                first.or(second)
            },
            false,
            pending_join,
        ))
    }

    fn if_advance<'b>(
        &mut self,
        visit: &mut FactId,
        progress: &mut IfProgress<'b>,
        returned: Option<Result<Type, Box<Diagnostic>>>,
    ) -> IfAction<'b> {
        let mut returned = returned;
        loop {
            progress.consumer.slot = progress.next_child_index;
            match progress.stage {
                IfStage::Condition => {
                    let condition_result = returned.take().expect("if condition has a result");
                    let condition_site = Self::primary_span(progress.condition);
                    let pending_shape_failure = self.graph.obligations
                        [progress.pending_before_condition..]
                        .iter()
                        .any(|entry| match entry {
                            ObligationKind::Join(join) => {
                                join.found == condition_site
                                    && self.graph.failed_constructor(join.constructor)
                            }
                            _ => false,
                        });
                    let condition_tainted =
                        self.graph.fact(*visit).tainted || pending_shape_failure;
                    self.graph.merge_child(progress.parent_fact, *visit, false);
                    *visit = progress.parent_fact;
                    self.graph.fact_mut(*visit).tainted |=
                        condition_tainted || condition_result.is_err();
                    if let Err(error) = condition_result {
                        if error.operation == Operation::AnalysisLimit {
                            return IfAction::Done(Err(error));
                        }
                        self.record_error(error);
                    }
                    progress.branch = Some(BranchProgress {
                        block: progress.then_block,
                        context: progress.branch_context.clone(),
                        next: 0,
                        result: None,
                        value_failed: false,
                        anchor_consumed: false,
                    });
                    progress.stage = IfStage::FirstBranch;
                }
                IfStage::FirstValue {
                    parent_fact,
                    nested_anchor,
                }
                | IfStage::SecondValue {
                    parent_fact,
                    nested_anchor,
                } => {
                    let branch = progress.branch.as_mut().expect("if branch exists");
                    let Statement::Value {
                        value: Some(expr), ..
                    } = &branch.block.statements[branch.next - 1]
                    else {
                        unreachable!("if value stage follows a value statement")
                    };
                    if let Err(error) = self.branch_accept(
                        visit,
                        branch,
                        expr,
                        parent_fact,
                        nested_anchor,
                        returned.take().expect("if value has a result"),
                    ) {
                        return IfAction::Done(Err(error));
                    }
                    progress.stage = if matches!(progress.stage, IfStage::FirstValue { .. }) {
                        IfStage::FirstBranch
                    } else {
                        IfStage::SecondBranch
                    };
                }
                IfStage::FirstBranch => {
                    let branch = progress.branch.as_mut().expect("first branch exists");
                    match self.branch_next(*visit, branch) {
                        BranchAction::Statement(statement) => {
                            return IfAction::Statement(statement);
                        }
                        BranchAction::Value {
                            expr,
                            context,
                            parent_fact,
                            nested_anchor,
                        } => {
                            progress.stage = IfStage::FirstValue {
                                parent_fact,
                                nested_anchor,
                            };
                            return IfAction::Value(expr, context);
                        }
                        BranchAction::Done(first, failed, anchored) => {
                            progress.first = first;
                            progress.first_failed = failed;
                            progress.first_anchored = anchored;
                            progress.branch = None;
                            if let Err(error) = self.if_after_first(progress) {
                                return IfAction::Done(Err(error));
                            }
                            if let Some(Else::Block(block)) = progress.otherwise {
                                progress.next_child_index = 1;
                                progress.branch = Some(BranchProgress {
                                    block,
                                    context: OwnedCheckContext {
                                        expected: progress.branch_context.expected.clone(),
                                        expected_span: progress
                                            .branch_context
                                            .expected_span
                                            .clone(),
                                        check_here: true,
                                        anchor: progress.anchor.clone(),
                                        outer_compare: false,
                                        related_expected: progress.branch_context.related_expected,
                                        deferred_element: progress.branch_context.deferred_element,
                                    },
                                    next: 0,
                                    result: None,
                                    value_failed: false,
                                    anchor_consumed: false,
                                });
                                progress.stage = IfStage::SecondBranch;
                            } else if let Some(Else::If(other)) = progress.otherwise {
                                progress.next_child_index = 1;
                                let context = self.if_else_context(progress, other);
                                progress.stage = IfStage::ElseIf;
                                return IfAction::Value(other, context);
                            } else {
                                return IfAction::Done(
                                    self.if_finish_join(progress, None, false, false),
                                );
                            }
                        }
                    }
                }
                IfStage::SecondBranch => {
                    let branch = progress.branch.as_mut().expect("second branch exists");
                    match self.branch_next(*visit, branch) {
                        BranchAction::Statement(statement) => {
                            return IfAction::Statement(statement);
                        }
                        BranchAction::Value {
                            expr,
                            context,
                            parent_fact,
                            nested_anchor,
                        } => {
                            progress.stage = IfStage::SecondValue {
                                parent_fact,
                                nested_anchor,
                            };
                            return IfAction::Value(expr, context);
                        }
                        BranchAction::Done(second, failed, anchored) => {
                            if let Some(Else::Block(block)) = progress.otherwise
                                && progress.branch_context.expected.is_none()
                                && progress.anchor_error.is_none()
                                && !failed
                                && !anchored
                                && let (Some((base, base_span)), Some(found)) =
                                    (progress.anchor.as_ref(), second.as_ref())
                            {
                                let found_span =
                                    branch_value_span(block).unwrap_or_else(|| block.span.clone());
                                match self.unify(
                                    *visit,
                                    base.clone(),
                                    found.clone(),
                                    base_span.clone(),
                                    found_span.clone(),
                                    found_span,
                                    true,
                                ) {
                                    Err(error) if error.operation == Operation::AnalysisLimit => {
                                        return IfAction::Done(Err(error));
                                    }
                                    Err(error) => progress.anchor_error = Some(error),
                                    Ok(_) => {}
                                }
                            }
                            return IfAction::Done(
                                self.if_finish_join(progress, second, failed, anchored),
                            );
                        }
                    }
                }
                IfStage::ElseIf => {
                    let result = returned.take().expect("else if has a result");
                    return IfAction::Done(match result {
                        Ok(ty) => {
                            let Some(Else::If(other)) = progress.otherwise else {
                                unreachable!("else if stage owns an else if")
                            };
                            let failed = self
                                .graph
                                .has(&span_key(other.span()), OccurrenceFlag::FailedJoin);
                            self.if_finish_join(
                                progress,
                                internal_state(&ty).is_none().then_some(ty),
                                failed,
                                failed,
                            )
                        }
                        Err(error) => Err(error),
                    });
                }
            }
        }
    }

    fn if_after_first(&mut self, progress: &mut IfProgress<'_>) -> Result<(), Box<Diagnostic>> {
        let then_block = progress.then_block;
        progress.first_anchored |= branch_value_expr(then_block).is_some_and(|expr| {
            self.graph
                .has(&span_key(expr.span()), OccurrenceFlag::FailedJoin)
                && !self
                    .graph
                    .has(&span_key(expr.span()), OccurrenceFlag::PartialNormal)
        });
        progress.first_partial = branch_value_expr(then_block).is_some_and(|expr| {
            self.graph
                .has(&span_key(expr.span()), OccurrenceFlag::PartialNormal)
        });
        progress.first_span = branch_value_span(then_block);
        progress.first_for_anchor = progress.first.clone().or_else(|| {
            branch_value_expr(then_block)
                .and_then(|expr| self.expression_variable(expr))
                .map(Type::Deferred)
        });
        if progress.branch_context.expected.is_none()
            && (!progress.first_failed || progress.first_partial)
            && !progress.first_anchored
            && let (Some((base, base_span)), Some(found)) = (
                progress.branch_context.anchor.as_ref(),
                progress.first_for_anchor.as_ref(),
            )
        {
            let pending_before_anchor = self.pending_join_count();
            let found_span = progress
                .first_span
                .clone()
                .unwrap_or_else(|| then_block.span.clone());
            match self.unify_independent(
                base.clone(),
                found.clone(),
                base_span.clone(),
                found_span.clone(),
                found_span,
                true,
            ) {
                Err(error) if error.operation == Operation::AnalysisLimit => return Err(error),
                Err(error) => progress.anchor_error = Some(error),
                Ok(_) => {}
            }
            progress.anchor_pending = self.pending_join_count() > pending_before_anchor;
        }
        progress.anchor = progress.branch_context.anchor.clone().or_else(|| {
            ((!progress.first_failed || progress.first_partial) && !progress.first_anchored)
                .then(|| {
                    progress
                        .first_for_anchor
                        .as_ref()
                        .zip(progress.first_span.as_ref())
                        .map(|(ty, span)| (ty.clone(), span.clone()))
                })
                .flatten()
        });
        Ok(())
    }

    fn if_else_context(&mut self, progress: &IfProgress<'_>, other: &Expr) -> OwnedCheckContext {
        let inner_shape_conflict = if let Expr::If {
            then_block,
            otherwise: Some(inner_other),
            ..
        } = other
        {
            let first = branch_value_expr(then_block);
            let second = match inner_other.as_ref() {
                Else::Block(block) => branch_value_expr(block),
                Else::If(_) => None,
            };
            first.zip(second).is_some_and(|(first, second)| {
                let first = self.expression_shape_hint(first);
                let second = self.expression_shape_hint(second);
                Self::shape_hints_conflict(&first, &second)
            })
        } else {
            false
        };
        let anchor = if inner_shape_conflict
            && progress
                .anchor
                .as_ref()
                .is_some_and(|(ty, _)| self.resolved_join_side(ty.clone()).is_none())
        {
            None
        } else {
            progress.anchor.clone()
        };
        OwnedCheckContext {
            expected: progress.branch_context.expected.clone(),
            expected_span: progress.branch_context.expected_span.clone(),
            check_here: true,
            anchor,
            outer_compare: false,
            related_expected: progress.branch_context.related_expected,
            deferred_element: progress.branch_context.deferred_element,
        }
    }

    #[cfg(test)]
    #[allow(clippy::too_many_arguments)] // Test helper forwards the drive cursor.
    fn branch(
        &mut self,
        visit: FactId,
        block: &Block,
        expected: Option<&Type>,
        expected_span: Option<&Span>,
        related_expected: bool,
        anchor: Option<(&Type, &Span)>,
        deferred_element: bool,
    ) -> Result<(Option<Type>, bool, bool), Box<Diagnostic>> {
        let mut progress = BranchProgress {
            block,
            context: OwnedCheckContext::new(CheckContext {
                expected,
                expected_span,
                check_here: true,
                anchor,
                outer_compare: false,
                related_expected,
                deferred_element,
            }),
            next: 0,
            result: None,
            value_failed: false,
            anchor_consumed: false,
        };
        loop {
            match self.branch_next(visit, &mut progress) {
                BranchAction::Statement(statement) => self.statement_at_site(statement),
                BranchAction::Value {
                    expr,
                    context,
                    parent_fact,
                    nested_anchor,
                } => {
                    let mut child = self.graph.child_fact(
                        parent_fact,
                        span_key(expr.span()),
                        FactRole::Expression,
                        FactInheritance::Empty,
                        FactInheritance::Inherit,
                    );
                    let outcome = self.check_against_context(expr, context.borrowed());
                    self.branch_accept(
                        &mut child,
                        &mut progress,
                        expr,
                        parent_fact,
                        nested_anchor,
                        outcome,
                    )?;
                }
                BranchAction::Done(result, failed, anchored) => {
                    return Ok((result, failed, anchored));
                }
            }
        }
    }

    fn branch_next<'b>(
        &mut self,
        visit: FactId,
        progress: &mut BranchProgress<'b>,
    ) -> BranchAction<'b> {
        if let Some(statement) = progress.block.statements.get(progress.next) {
            progress.next += 1;
            if let Statement::Value {
                value: Some(expr), ..
            } = statement
            {
                let nested_anchor = progress.context.expected.is_none()
                    && progress.context.anchor.is_some()
                    && Self::is_value_if(expr);
                let context = OwnedCheckContext {
                    expected: progress.context.expected.clone(),
                    expected_span: progress.context.expected_span.clone(),
                    check_here: true,
                    anchor: if nested_anchor {
                        progress.context.anchor.clone()
                    } else {
                        None
                    },
                    outer_compare: false,
                    related_expected: progress.context.related_expected,
                    deferred_element: progress.context.deferred_element,
                };
                return BranchAction::Value {
                    expr,
                    context,
                    parent_fact: visit,
                    nested_anchor,
                };
            }
            progress.result = None;
            progress.anchor_consumed = false;
            return BranchAction::Statement(statement);
        }
        BranchAction::Done(
            progress.result.take(),
            progress.value_failed,
            progress.anchor_consumed,
        )
    }

    fn branch_accept(
        &mut self,
        visit: &mut FactId,
        progress: &mut BranchProgress<'_>,
        expr: &Expr,
        parent_fact: FactId,
        nested_anchor: bool,
        outcome: Result<Type, Box<Diagnostic>>,
    ) -> Result<(), Box<Diagnostic>> {
        let tainted = self.graph.fact(*visit).tainted;
        self.graph.merge_child(parent_fact, *visit, true);
        *visit = parent_fact;
        progress.anchor_consumed = nested_anchor;
        match outcome {
            Ok(ty) => {
                let deferred = self
                    .graph
                    .has(&span_key(expr.span()), OccurrenceFlag::Deferred);
                let divergent = self
                    .graph
                    .has(&span_key(expr.span()), OccurrenceFlag::Divergent);
                let pending_context_conflict = self
                    .graph
                    .has(&span_key(expr.span()), OccurrenceFlag::PendingDomain)
                    && progress.context.expected.as_ref().is_some_and(|wanted| {
                        internal_state(wanted).is_none()
                            && internal_state(&ty).is_none()
                            && *wanted != ty
                    });
                let usable = (!tainted
                    && !pending_context_conflict
                    && !(self.planning
                        && self
                            .graph
                            .has(&span_key(expr.span()), OccurrenceFlag::PendingDomain)))
                    || self
                        .graph
                        .has(&span_key(expr.span()), OccurrenceFlag::PartialNormal);
                let partial = self
                    .graph
                    .has(&span_key(expr.span()), OccurrenceFlag::PartialNormal);
                progress.result = (usable
                    && (!deferred || progress.context.expected.is_some() && !divergent || partial))
                    .then_some(ty);
                progress.value_failed |= self
                    .graph
                    .has(&span_key(expr.span()), OccurrenceFlag::FailedJoin);
                progress.value_failed |= pending_context_conflict;
                self.graph.fact_mut(*visit).tainted |= pending_context_conflict;
            }
            Err(error) => {
                if error.operation == Operation::AnalysisLimit {
                    return Err(error);
                }
                self.record_error(error);
                progress.result = self
                    .graph
                    .expression_types
                    .get(&span_key(expr.span()))
                    .cloned()
                    .filter(|ty| !matches!(internal_state(ty), Some(Type::Tainted | Type::Never)));
                progress.value_failed = true;
            }
        }
        Ok(())
    }

    fn match_advance<'b>(
        &mut self,
        visit: &mut FactId,
        progress: &mut MatchProgress<'b>,
        returned: Option<Result<Type, Box<Diagnostic>>>,
    ) -> MatchAction<'b> {
        let mut returned = returned;
        loop {
            if progress.arm_index == progress.selection.arms.len() {
                return MatchAction::Done(self.finish_selection(
                    progress.selection,
                    &progress.selected,
                    &progress.joined,
                    true,
                    progress.context.expected.as_ref(),
                ));
            }
            let arm = &progress.selection.arms[progress.arm_index];
            match progress.stage {
                MatchStage::Arm => {
                    progress.selected = self.selection_type(&progress.selected, 0);
                    if let Err(error) = self.pattern_type(
                        &arm.pattern,
                        &progress.selected,
                        &progress.selected_origin,
                    ) {
                        return MatchAction::Done(Err(error));
                    }
                    if let Some(guard) = &arm.guard {
                        progress.stage = MatchStage::Guard;
                        return MatchAction::Value(
                            guard,
                            OwnedCheckContext::new(CheckContext {
                                expected: Some(&Type::Bool),
                                expected_span: arm.guard_span.as_ref(),
                                check_here: true,
                                anchor: None,
                                outer_compare: false,
                                related_expected: arm.guard_span.is_some(),
                                deferred_element: false,
                            }),
                        );
                    }
                    progress.stage = MatchStage::Branch;
                }
                MatchStage::Guard => {
                    if let Some(Err(error)) = returned.take() {
                        if error.operation == Operation::AnalysisLimit {
                            return MatchAction::Done(Err(error));
                        }
                        self.record_error(error);
                    }
                    progress.stage = MatchStage::Branch;
                }
                MatchStage::Value {
                    parent_fact,
                    nested_anchor,
                } => {
                    let result = returned.take().expect("match value has a result");
                    let branch = progress.branch.as_mut().expect("match branch exists");
                    let Statement::Value {
                        value: Some(expr), ..
                    } = &branch.block.statements[branch.next - 1]
                    else {
                        unreachable!("match value stage follows a value statement")
                    };
                    if let Err(error) =
                        self.branch_accept(visit, branch, expr, parent_fact, nested_anchor, result)
                    {
                        return MatchAction::Done(Err(error));
                    }
                    progress.stage = MatchStage::Branch;
                }
                MatchStage::Branch => {}
            }
            if progress.branch.is_none() {
                progress.branch = Some(BranchProgress {
                    block: &arm.body,
                    context: OwnedCheckContext {
                        expected: progress.context.expected.clone(),
                        expected_span: progress.context.expected_span.clone(),
                        check_here: true,
                        anchor: progress.joined.clone(),
                        outer_compare: false,
                        related_expected: progress.context.expected.is_some(),
                        deferred_element: false,
                    },
                    next: 0,
                    result: None,
                    value_failed: false,
                    anchor_consumed: false,
                });
            }
            let branch = progress.branch.as_mut().unwrap();
            match self.branch_next(*visit, branch) {
                BranchAction::Statement(statement) => {
                    return MatchAction::Statement(statement);
                }
                BranchAction::Value {
                    expr,
                    context,
                    parent_fact,
                    nested_anchor,
                } => {
                    progress.stage = MatchStage::Value {
                        parent_fact,
                        nested_anchor,
                    };
                    return MatchAction::Value(expr, context);
                }
                BranchAction::Done(value, failed, _) => {
                    let value = value.or_else(|| {
                        branch_value_expr(&arm.body)
                            .and_then(|expr| self.expression_variable(expr))
                            .map(Type::Deferred)
                    });
                    if !failed && let Some(value) = value {
                        let origin =
                            branch_value_span(&arm.body).unwrap_or_else(|| arm.body.span.clone());
                        if let Some((first, first_origin)) = &progress.joined {
                            let unified = self.unify(
                                *visit,
                                first.clone(),
                                value.clone(),
                                first_origin.clone(),
                                origin.clone(),
                                origin,
                                progress.context.expected.is_none(),
                            );
                            match unified {
                                Ok(ty) => progress.joined = Some((ty, first_origin.clone())),
                                Err(error) => return MatchAction::Done(Err(error)),
                            }
                        } else {
                            progress.joined = Some((value, origin));
                        }
                    }
                    progress.branch = None;
                    progress.arm_index += 1;
                    progress.stage = MatchStage::Arm;
                }
            }
        }
    }

    fn operator_span(&self, left: &Span, right: &Span, op: &str) -> Span {
        // left.end is a token boundary. The operator starts after it,
        // so including a token beginning exactly there only adds a
        // nonmatching delimiter before the same operator search.
        let at = self.tokens.partition_point(|t| t.span.start < left.end);
        self.tokens[at..]
            .iter()
            .take_while(|t| t.span.start < right.start)
            .find(|t| t.text == op)
            .map(|t| t.span.clone())
            .unwrap_or_else(|| Span {
                file: self.file.into(),
                start: left.end,
                end: left.end + op.len() as u64,
            })
    }

    fn comparison_domain_error(
        &mut self,
        visit: FactId,
        op: &str,
        left: &Expr,
        right: &Expr,
        ty: &Type,
    ) -> Result<Option<Box<Diagnostic>>, Box<Diagnostic>> {
        // F0-0002 §4.1.2.4–5, RD95: the domain of a determined, valid operand
        // remains independent when its sibling's result is contaminated.
        // §4.6 gives a cross-type mismatch priority over the operator
        // domain when both operand shapes already prove them different.
        // A planned Array root is such evidence even before its leaf is
        // concrete; a failed inner expression supplies no such evidence.
        if !self.graph.fact_mut(visit).tainted
            && internal_state(ty).is_none()
            && !matches!(ty, Type::Array(_))
            && let Some(id) = self.expression_variable(right)
        {
            let root = self.root(id);
            if self.graph.type_nodes[&root].array_child.is_some() {
                return Ok(None);
            }
        }
        if !self.graph.fact_mut(visit).tainted
            && !self
                .graph
                .has(&span_key(right.span()), OccurrenceFlag::FailedJoin)
            && let Some(id) = self.expression_variable(left)
        {
            let root = self.root(id);
            if self.graph.type_nodes[&root].array_child.is_some()
                && self.pending_operation_type(right).is_some_and(|right_ty| {
                    internal_state(&right_ty).is_none() && !matches!(right_ty, Type::Array(_))
                })
            {
                return Ok(None);
            }
        }
        let domain = if super::operators::is_relational(op, self.f5) {
            "I64"
        } else {
            "domain:copyable_data"
        };
        let variable = self.expression_variable(left);
        let complete = variable
            .and_then(|id| self.concrete(id).map(|(resolved, _)| resolved))
            .or_else(|| (internal_state(ty).is_none()).then_some(ty.clone()));
        if let Some(complete) = complete {
            let invalid = if super::operators::is_relational(op, self.f5) {
                complete != Type::I64
            } else {
                !comparable_data(&complete)
            };
            return Ok(invalid.then(|| {
                mismatch(
                    self.operator_span(left.span(), right.span(), op),
                    Self::primary_span(left),
                    None,
                    domain,
                    &complete,
                )
            }));
        }
        if let Some(id) = variable {
            // §4.1.2.4–5, RD95: the independent valid operand retains its
            // domain site while its deferred type is completed later.
            let root = self.root(id);
            if super::operators::is_relational(op, self.f5)
                && self.graph.type_nodes[&root].array_child.is_none()
            {
                let operator = self.operator_span(left.span(), right.span(), op);
                self.unify_with_expected_origin(
                    visit,
                    Type::I64,
                    Type::Deferred(id),
                    operator.clone(),
                    operator.clone(),
                    operator,
                    UnifyPolicy {
                        branch: false,
                        related_expected: false,
                    },
                )?;
                return Ok(None);
            }
            self.graph.register(ObligationKind::BinaryDomain(Box::new(
                PendingBinaryDomain {
                    variable: id,
                    operator: self.operator_span(left.span(), right.span(), op),
                    found: Self::primary_span(left),
                    domain,
                    result: Span {
                        file: self.file.into(),
                        start: left.span().start,
                        end: right.span().end,
                    },
                    conditional_bindings: Vec::new(),
                },
            )));
        }
        Ok(None)
    }

    fn finish_independent_binary_rhs(
        &mut self,
        op: &str,
        left: &Expr,
        right: &Expr,
        left_domain_failed: bool,
        result: Result<Type, Box<Diagnostic>>,
        right_tainted: bool,
    ) -> Result<bool, Box<Diagnostic>> {
        let numeric = super::operators::is_arithmetic(op, self.f5);
        let right_failed = result.is_err();
        match result {
            Ok(ty) if numeric && left_domain_failed && !right_tainted => {
                if let Some(id) = self.expression_variable(right)
                    && !self.type_determined(&ty)
                {
                    let operand = Self::primary_span(right);
                    self.unify_with_expected_origin_independent(
                        Type::I64,
                        Type::Deferred(id),
                        operand.clone(),
                        operand.clone(),
                        operand,
                        UnifyPolicy {
                            branch: false,
                            related_expected: false,
                        },
                    )?;
                }
            }
            Ok(ty) if !right_tainted && !numeric => {
                let wanted = if super::operators::is_relational(op, self.f5) {
                    "I64"
                } else {
                    "domain:copyable_data"
                };
                let variable = self.expression_variable(right);
                let complete = variable
                    .and_then(|id| self.concrete(id).map(|(resolved, _)| resolved))
                    .or_else(|| (internal_state(&ty).is_none()).then_some(ty));
                if let Some(complete) = complete {
                    let invalid = if super::operators::is_relational(op, self.f5) {
                        complete != Type::I64
                    } else {
                        !copyable(&complete)
                    };
                    if invalid {
                        self.record_error(mismatch(
                            self.operator_span(left.span(), right.span(), op),
                            Self::primary_span(right),
                            None,
                            wanted,
                            &complete,
                        ));
                    }
                } else if let Some(id) = variable {
                    // ADR §4.1.2.3–5: the valid operand retains its own
                    // intrinsic domain when its sibling is contaminated.
                    // A later type completes the same operator site.
                    let root = self.root(id);
                    if super::operators::is_relational(op, self.f5)
                        && self.graph.type_nodes[&root].array_child.is_none()
                    {
                        let operator = self.operator_span(left.span(), right.span(), op);
                        self.unify_with_expected_origin_independent(
                            Type::I64,
                            Type::Deferred(id),
                            operator.clone(),
                            operator.clone(),
                            operator,
                            UnifyPolicy {
                                branch: false,
                                related_expected: false,
                            },
                        )?;
                    } else {
                        self.graph.register(ObligationKind::BinaryDomain(Box::new(
                            PendingBinaryDomain {
                                variable: id,
                                operator: self.operator_span(left.span(), right.span(), op),
                                found: Self::primary_span(right),
                                domain: wanted,
                                result: Span {
                                    file: self.file.into(),
                                    start: left.span().start,
                                    end: right.span().end,
                                },
                                conditional_bindings: Vec::new(),
                            },
                        )));
                    }
                }
            }
            Err(error) if error.operation == Operation::AnalysisLimit => return Err(error),
            Err(error) => {
                // + and - emit only the first invalid domain operand. A
                // nested error remains independent of that local limit.
                let same_domain = numeric
                    && left_domain_failed
                    && error.code == "NX-E0102"
                    && error.subcause.as_deref() == Some("type_mismatch")
                    && error.primary_span.as_ref() == Some(&Self::primary_span(right))
                    && error.data.get("expected_type") == Some(&DataValue::String("I64".into()));
                if !same_domain {
                    self.record_error(error);
                }
            }
            _ => {}
        }
        Ok(right_tainted || right_failed)
    }

    fn finish_union_call(
        &mut self,
        name: &str,
        argument: &Expr,
        call_span: &Span,
        expected: Option<&Type>,
        payload_result: Result<Type, Box<Diagnostic>>,
        payload_tainted: bool,
    ) -> Result<Type, Box<Diagnostic>> {
        let kind = if name == "Some" {
            UnionKind::Option
        } else {
            UnionKind::Result
        };
        let tag = kind
            .variant_tag(name)
            .ok_or_else(|| unresolved_type(call_span.clone()))?;
        let expected_union = expected
            .and_then(concrete_union)
            .filter(|union| union.kind == kind);
        let payload = payload_result?;
        if payload_tainted || payload == Type::Tainted {
            return Ok(Type::Tainted);
        }
        if let Some(complete) = expected.filter(|_| expected_union.is_some()) {
            return Ok(complete.clone());
        }
        let result = self.new_variable(call_span)?;
        let children = self.require_union(result, kind, call_span)?;
        let payload_child = children
            .variant(tag)
            .and_then(|variant| variant.payload)
            .ok_or_else(|| unresolved_type(call_span.clone()))?;
        self.bind_independent(
            payload_child,
            payload,
            Self::primary_span(argument),
            Self::primary_span(argument),
            true,
        )?;
        self.graph
            .type_nodes
            .get_mut(&result)
            .unwrap()
            .variant_constructed = true;
        self.graph.expr_vars.insert(span_key(call_span), result);
        self.graph
            .mark(span_key(call_span), OccurrenceFlag::Deferred);
        self.graph.register(ObligationKind::VariantConstructor {
            union: result,
            span: call_span.clone(),
        });
        match kind {
            UnionKind::Option => Ok(Type::option(Arc::new(Type::Deferred(payload_child)))),
            UnionKind::Result => {
                let (ok, err) = children
                    .result_payloads()
                    .ok_or_else(|| unresolved_type(call_span.clone()))?;
                Ok(Type::result(
                    Arc::new(Type::Deferred(ok)),
                    Arc::new(Type::Deferred(err)),
                ))
            }
        }
    }

    fn prepare_closure_call(
        &self,
        site: u64,
        target: &Expr,
        args: &[crate::frontend::parse::Argument],
    ) -> Result<(ClosureDraft, Vec<usize>), Box<Diagnostic>> {
        let draft = self.closures.get(&site).cloned().ok_or_else(|| {
            names::diagnostic(
                "NX-E1111",
                None,
                Stage::Types,
                Operation::TypesCheck,
                Self::primary_span(target),
            )
        })?;
        let params = draft
            .params
            .iter()
            .map(|(name, mode, name_span, _)| names::ParameterSig {
                name: name.clone(),
                name_span: name_span.clone(),
                type_span: name_span.clone(),
                mode: *mode,
                ty: SyntaxType::Unit,
            })
            .collect();
        let slots = names::bind_arguments(params, &Self::primary_span(target), args)?;
        Ok((draft, slots))
    }

    fn assert_eq_prepare(&mut self, state: &mut AssertEqFrameState<'_>) {
        state.prior_errors = self.errors.len();
    }

    fn assert_eq_accept(
        &mut self,
        visit: FactId,
        state: &mut AssertEqFrameState<'_>,
        result: Result<Type, Box<Diagnostic>>,
    ) -> Result<(), Box<Diagnostic>> {
        let Expr::Call { args, span, .. } = state.expr else {
            unreachable!("AssertEq frame owns a call")
        };
        let arg = &args[state.next];
        let slot = self.resolved.calls[&(span.start, span.end)].slots[state.next];
        let tainted = self.graph.fact_mut(visit).tainted;
        let site = Self::primary_span(&arg.value);
        let recover = self
            .pending_operand_evidence(&arg.value)
            .and_then(|guards| {
                self.pending_operation_type(&arg.value)
                    .or_else(|| self.graph.conditional_result(&span_key(&site)).cloned())
                    .map(|nominal| (nominal, guards))
            })
            .filter(|_| {
                self.errors.len() == state.prior_errors
                    && !self
                        .graph
                        .marked(OccurrenceFlag::FailedJoin)
                        .any(|(start, end)| site.start <= start && end <= site.end)
            });
        if tainted && recover.is_none() {
            self.graph.fact_mut(state.call_fact).tainted = true;
        }
        state.values[slot] = Some(match result {
            Ok(_) if let Some((nominal, guards)) = recover => {
                state.evidence[slot] = Some(guards);
                nominal
            }
            Ok(ty)
                if !tainted
                    && !self
                        .graph
                        .has(&span_key(arg.value.span()), OccurrenceFlag::FailedJoin) =>
            {
                ty
            }
            Ok(_) => {
                state.invalid = true;
                Type::Tainted
            }
            Err(error) if error.operation == Operation::AnalysisLimit => return Err(error),
            Err(error) => {
                self.record_error(error);
                state.invalid = true;
                Type::Tainted
            }
        });
        Ok(())
    }

    fn finish_assert_eq(
        &mut self,
        visit: FactId,
        state: AssertEqFrameState<'_>,
    ) -> Result<Type, Box<Diagnostic>> {
        let Expr::Call {
            args,
            span: call_span,
            ..
        } = state.expr
        else {
            unreachable!("AssertEq frame owns a call")
        };
        let slots = self.resolved.calls[&(call_span.start, call_span.end)]
            .slots
            .clone();
        let ordered = |index: usize| -> &Expr {
            &args[slots
                .iter()
                .position(|slot| *slot == index)
                .expect("complete binding")]
            .value
        };
        let values = state.values;
        let evidence = state.evidence;
        let mut invalid = state.invalid;
        let declared_result = Type::from(
            &names::intrinsic("AssertEq")
                .expect("closed intrinsic")
                .result,
        );
        let first_var = self.expression_variable(ordered(0));
        let second_var = self.expression_variable(ordered(1));
        let first = values[0].as_ref().expect("complete binding").clone();
        let second = values[1].as_ref().expect("complete binding").clone();
        let left = first_var.map(Type::Deferred).unwrap_or(first.clone());
        let right = second_var.map(Type::Deferred).unwrap_or(second.clone());
        let left_origin = Self::primary_span(ordered(0));
        let right_origin = Self::primary_span(ordered(1));
        if !invalid {
            let composite_guard = evidence.into_iter().flatten().fold(
                PendingOperandEvidence::default(),
                |mut all, side| {
                    all.domains.extend(side.domains);
                    all.projections.extend(side.projections);
                    all.results.extend(side.results);
                    all
                },
            );
            let guard = self
                .pending_operation_guard(ordered(0))
                .or_else(|| self.pending_operation_guard(ordered(1)));
            let composite = !composite_guard.projections.is_empty();
            if let Some((variable, domain)) = guard
                .or_else(|| {
                    composite_guard.domains.first().copied().or_else(|| {
                        composite_guard
                            .projections
                            .first()
                            .map(|&(base, _)| (base, "domain:array"))
                    })
                })
                .filter(|_| {
                    // A nominal result with an undecided domain must
                    // not bind the other, still pending operand before
                    // that domain is resolved (§4.1.2.3).
                    internal_state(&left).is_none() || internal_state(&right).is_none()
                })
            {
                let additional_domains = composite_guard
                    .domains
                    .into_iter()
                    .filter(|&entry| entry != (variable, domain))
                    .collect();
                self.graph
                    .register(ObligationKind::GuardedComparison(Box::new(
                        PendingGuardedComparison {
                            variable,
                            domain,
                            expected: left,
                            found: right,
                            expected_origin: left_origin,
                            found_site: right_origin,
                            result: call_span.clone(),
                            consumer_expected: None,
                            consumer_origin: None,
                            domain_error_if_equal: None,
                            additional_domains,
                            projections: composite_guard.projections,
                            dependent_results: composite_guard.results,
                            require_resolved: composite,
                            conditional_bindings: Vec::new(),
                        },
                    )));
            } else {
                self.unify(
                    visit,
                    left,
                    right,
                    left_origin,
                    right_origin,
                    Self::primary_span(ordered(1)),
                    false,
                )?;
            }
        }
        // ADR §4.1.2.4–5: equality precedes this domain check, and
        // each valid argument retains its own independent site.
        for (index, value) in [first, second].into_iter().enumerate() {
            if value == Type::Tainted {
                continue;
            }
            let operand = ordered(index);
            if let Some(id) = self.expression_variable(operand) {
                if let Err(error) = self.require_copyable(id, &Self::primary_span(operand), true) {
                    if error.operation == Operation::AnalysisLimit {
                        return Err(error);
                    }
                    self.record_error(error);
                    invalid = true;
                }
            } else if internal_state(&value).is_none() && !comparable_data(&value) {
                self.record_error(mismatch(
                    Self::primary_span(operand),
                    Self::primary_span(operand),
                    None,
                    "domain:copyable_data",
                    &value,
                ));
                invalid = true;
            }
        }
        self.graph.fact_mut(visit).tainted |= invalid;
        Ok(declared_result.clone())
    }

    #[allow(clippy::too_many_arguments)] // The prepared argument retains the call's source context.
    fn check_intrinsic(
        &mut self,
        name: &str,
        args: &[crate::frontend::parse::Argument],
        slots: &[usize],
        call_span: &Span,
        expected: Option<&Type>,
        expected_span: Option<&Span>,
        prepared: Option<Result<Type, Box<Diagnostic>>>,
    ) -> Result<Type, Box<Diagnostic>> {
        if self.f5 && super::pending_f5::intrinsic_scope(name).is_some() {
            let spelling = crate::prelude::f5_array_intrinsic(name)
                .expect("sealed F5 intrinsic")
                .spelling;
            let span = Span {
                file: call_span.file.clone(),
                start: call_span.start,
                end: call_span.start + spelling.len() as u64,
            };
            return Err(super::pending_f5::diagnostic("L07", span));
        }
        let mut prepared = prepared;
        let declared = names::intrinsic(name).expect("intrinsic resolved from closed manifest");
        let declared_result = Type::from(&declared.result);
        let ordered = |index: usize| -> &Expr {
            &args[slots
                .iter()
                .position(|slot| *slot == index)
                .expect("complete binding")]
            .value
        };
        match name {
            "Iterator.Next" => {
                let argument = ordered(0);
                let got = prepared.take().expect("intrinsic argument is scheduled")?;
                if let Type::Generator { item, .. } = got {
                    Ok(Type::option(item))
                } else {
                    Err(mismatch(
                        Self::primary_span(argument),
                        Self::primary_span(argument),
                        None,
                        "domain:generator",
                        &got,
                    ))
                }
            }
            "MockOpen" | "MockBlockingOpen" | "MockWriteI64" | "MockReply" => {
                prepared.take().expect("intrinsic argument is scheduled")?;
                Ok(declared_result.clone())
            }
            "TryAdd" => unreachable!("TryAdd arguments are scheduled by driver"),
            "MockRequest" | "MockUnknown" | "Fail" => Ok(declared_result.clone()),
            "MockReadRecords" => {
                let got = prepared.take().expect("intrinsic argument is scheduled")?;
                if got != Type::MockResource {
                    return Err(mismatch(
                        Self::primary_span(ordered(0)),
                        Self::primary_span(ordered(0)),
                        None,
                        "MockResource",
                        &got,
                    ));
                }
                Ok(declared_result.clone())
            }
            "Inspect" => {
                let got = prepared.take().expect("intrinsic argument is scheduled")?;
                let before = self.pending_domain_count();
                self.check_intrinsic_domain(ordered(0), &got, "domain:mock_resource")?;
                self.remember_pending_domain_result(
                    before,
                    call_span,
                    &declared_result,
                    expected,
                    expected_span,
                );
                Ok(declared_result.clone())
            }
            "Drop" => {
                prepared.take().expect("intrinsic argument is scheduled")?;
                Ok(declared_result.clone())
            }
            "Len" => {
                let got = prepared.take().expect("intrinsic argument is scheduled")?;
                let before = self.pending_domain_count();
                self.check_intrinsic_domain(ordered(0), &got, "domain:array")?;
                self.remember_pending_domain_result(
                    before,
                    call_span,
                    &declared_result,
                    expected,
                    expected_span,
                );
                if self
                    .graph
                    .has(&span_key(ordered(0).span()), OccurrenceFlag::Divergent)
                {
                    self.graph
                        .mark(span_key(call_span), OccurrenceFlag::Deferred);
                    self.graph
                        .mark(span_key(call_span), OccurrenceFlag::Divergent);
                }
                Ok(declared_result.clone())
            }
            "Print" => {
                let got = prepared.take().expect("intrinsic argument is scheduled")?;
                let before = self.pending_domain_count();
                self.check_intrinsic_domain(ordered(0), &got, "domain:copyable_data")?;
                if internal_state(&got).is_none() && !comparable_data(&got) {
                    return Err(mismatch(
                        Self::primary_span(ordered(0)),
                        Self::primary_span(ordered(0)),
                        None,
                        "domain:copyable_data",
                        &got,
                    ));
                }
                self.remember_pending_domain_result(
                    before,
                    call_span,
                    &declared_result,
                    expected,
                    expected_span,
                );
                Ok(declared_result.clone())
            }
            "AssertEq" => unreachable!("AssertEq arguments are scheduled by driver"),
            _ => unreachable!("union constructors complete in expression frames"),
        }
    }
}

#[allow(clippy::vec_box)] // Semantic diagnostics retain the shared boxed representation.
#[cfg(test)]
pub(crate) fn check_with_partial(
    file: &str,
    source: &str,
    ast: &Ast,
    resolved: &Resolved,
) -> TypeCheckReport {
    check_with_partial_limit(file, source, ast, resolved, 1_000_000)
}

fn cyclic_generator<'a>(ast: &'a Ast, resolved: &Resolved) -> Option<&'a Function> {
    enum Node<'a> {
        Expr(&'a Expr),
        Block(&'a Block),
        Statement(&'a Statement),
        Match(&'a parse::Match),
    }
    fn calls_in_block(
        block: &Block,
        names: &HashMap<&str, usize>,
        resolved: &Resolved,
        edges: &mut HashSet<usize>,
    ) {
        let mut pending = vec![Node::Block(block)];
        while let Some(node) = pending.pop() {
            match node {
                Node::Block(block) => {
                    pending.extend(block.statements.iter().rev().map(Node::Statement));
                }
                Node::Statement(statement) => match statement {
                    Statement::Let { value, .. } | Statement::Expr { value, .. } => {
                        pending.push(Node::Expr(value));
                    }
                    Statement::Assign { place, value, .. } => {
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
                        if let Some(otherwise) = otherwise {
                            pending.push(Node::Statement(otherwise));
                        }
                        pending.push(Node::Block(then_block));
                        pending.push(Node::Expr(condition));
                    }
                    Statement::While {
                        condition, body, ..
                    } => {
                        pending.push(Node::Block(body));
                        pending.push(Node::Expr(condition));
                    }
                    Statement::For { iterable, body, .. } => {
                        pending.push(Node::Block(body));
                        pending.push(Node::Expr(iterable));
                    }
                    Statement::Match(selection) => pending.push(Node::Match(selection)),
                    Statement::Block(inner) => pending.push(Node::Block(inner)),
                },
                Node::Expr(expr) => match expr {
                    Expr::Call { target, args, span } => {
                        if let Some(names::CallBinding {
                            target: ResolvedUse::Function(name),
                            ..
                        }) = resolved.calls.get(&(span.start, span.end))
                            && let Some(index) = names.get(name.as_str())
                        {
                            edges.insert(*index);
                        }
                        pending.extend(args.iter().rev().map(|arg| Node::Expr(&arg.value)));
                        pending.push(Node::Expr(target));
                    }
                    Expr::Array(items, _) => {
                        pending.extend(items.iter().rev().map(Node::Expr));
                    }
                    Expr::Group(inner, _)
                    | Expr::Unary { value: inner, .. }
                    | Expr::Neg { value: inner, .. }
                    | Expr::Not { value: inner, .. } => {
                        pending.push(Node::Expr(inner));
                    }
                    Expr::Binary { left, right, .. }
                    | Expr::Index {
                        target: left,
                        index: right,
                        ..
                    } => {
                        pending.push(Node::Expr(right));
                        pending.push(Node::Expr(left));
                    }
                    Expr::If {
                        condition,
                        then_block,
                        otherwise,
                        ..
                    } => {
                        match otherwise.as_deref() {
                            Some(Else::Block(block)) => pending.push(Node::Block(block)),
                            Some(Else::If(next)) => pending.push(Node::Expr(next)),
                            None => {}
                        }
                        pending.push(Node::Block(then_block));
                        pending.push(Node::Expr(condition));
                    }
                    Expr::Match(selection) => pending.push(Node::Match(selection)),
                    // Closure bodies run only when called.
                    Expr::Closure(_)
                    | Expr::Name(..)
                    | Expr::Integer(..)
                    | Expr::Bool(..)
                    | Expr::Unit(..) => {}
                },
                Node::Match(selection) => {
                    for arm in selection.arms.iter().rev() {
                        pending.push(Node::Block(&arm.body));
                        if let Some(guard) = &arm.guard {
                            pending.push(Node::Expr(guard));
                        }
                    }
                    pending.push(Node::Expr(&selection.scrutinee));
                }
            }
        }
    }
    let generators: Vec<&Function> = ast
        .functions
        .iter()
        .filter(|function| function.is_generator)
        .collect();
    let names: HashMap<&str, usize> = generators
        .iter()
        .enumerate()
        .map(|(index, function)| (function.name.as_str(), index))
        .collect();
    let graph: Vec<HashSet<usize>> = generators
        .iter()
        .map(|function| {
            let mut edges = HashSet::new();
            calls_in_block(&function.body, &names, resolved, &mut edges);
            edges
        })
        .collect();
    for (root, function) in generators.iter().enumerate() {
        let mut seen = HashSet::new();
        let mut pending: Vec<usize> = graph[root].iter().copied().collect();
        while let Some(index) = pending.pop() {
            if index == root {
                return Some(function);
            }
            if seen.insert(index) {
                pending.extend(graph[index].iter().copied());
            }
        }
    }
    None
}

pub(crate) fn check_with_partial_limit(
    file: &str,
    source: &str,
    ast: &Ast,
    resolved: &Resolved,
    solver_limit: u64,
) -> TypeCheckReport {
    let tokens = match lex::lex(file, source) {
        Ok(tokens) => tokens,
        Err(error) => {
            return TypeCheckReport {
                inferred: InferredTypes::default(),
                errors: vec![error],
                #[cfg(test)]
                stats: Vec::new(),
            };
        }
    };
    if let Some(function) = cyclic_generator(ast, resolved) {
        let mut error = names::diagnostic(
            "NX-E0006",
            None,
            Stage::Types,
            Operation::TypesCheck,
            function.name_span.clone(),
        );
        error.feature_id = Some("generator".into());
        error.data.insert(
            "pending_scope".into(),
            DataValue::String("later_adr".into()),
        );
        return TypeCheckReport {
            inferred: InferredTypes::default(),
            errors: vec![error],
            #[cfg(test)]
            stats: Vec::new(),
        };
    }
    let mut checker = Checker {
        file,
        tokens,
        resolved,
        f5: ast.f5_operators,
        closures: HashMap::new(),
        saved_closures: HashMap::new(),
        generator_slots: HashMap::new(),
        hidden_for: Vec::new(),
        closure_local_sites: HashSet::new(),
        pending_depth: std::cell::RefCell::new(Vec::new()),
        defer_depth: true,
        depth_memo_ready: false,
        solver_steps: 0,
        solver_limit,
        planning: false,
        occurrences: OccurrenceIndex::default(),
        function_ordinal: 0,
        current_passes: 0,
        current_planning_calls: 0,
        block_depth: 0,
        top_statement_start: 0,
        top_statement_end: 0,
        active_if_span: None,
        analysis_exhausted: false,
        quota_error: None,
        generator_item: None,
        loop_depth: 0,
        errors: Vec::new(),
        graph: InferenceGraph {
            equality_clock: 0,
            equality_records: Vec::new(),
            active_equality: None,
            node_journal: None,
            active_facts: HashMap::new(),
            visit_generations: Vec::new(),
            fact_clock: std::rc::Rc::new(std::cell::Cell::new(0)),
            generation_clock: std::rc::Rc::new(std::cell::Cell::new(0)),
            locals: HashMap::new(),
            type_nodes: HashMap::new(),
            array_unify_cache: HashMap::new(),
            expr_vars: HashMap::new(),
            expression_types: HashMap::new(),
            nominal_depth_memo: std::cell::RefCell::new(HashMap::new()),
            next_variable: 0,
            obligations: Vec::new(),
            occurrences: HashMap::new(),
            emitted_sites: HashSet::new(),
            eligible_shapes: HashSet::new(),
            shape_plan: HashMap::new(),
            concrete_plan: HashMap::new(),
            read_plan: HashMap::new(),
            assignment_type_plan: HashMap::new(),
            nested_assignment_sites: HashSet::new(),
            pre_assignment_snapshot: HashMap::new(),
            projected_bases: HashSet::new(),
            projection_items: HashSet::new(),
            assigned_bases: HashSet::new(),
            assignment_starts: HashMap::new(),
            first_assignment_rhs_starts: HashMap::new(),
            prefix_concrete: HashMap::new(),
            preseeded_children: HashSet::new(),
            result: Type::Unit,
            result_span: None,
        },
    };
    let mut inferred = InferredTypes::default();
    let mut late_depth: HashMap<(u64, u64), (Type, Span)> = HashMap::new();
    #[cfg(test)]
    let mut stats = Vec::with_capacity(ast.functions.len());
    for (function_ordinal, function) in ast.functions.iter().enumerate() {
        if checker.analysis_exhausted {
            break;
        }
        checker.function_ordinal = u32::try_from(function_ordinal).unwrap_or(u32::MAX);
        checker.occurrences =
            OccurrenceIndex::for_function(checker.function_ordinal, &function.body);
        checker.current_passes = 0;
        checker.current_planning_calls = 0;
        checker.planning = true;
        for _ in 0..=128 {
            checker.analysis_exhausted = false;
            checker.quota_error = None;
            let _ = checker.function(function);
            if !checker.collect_planned_shapes() {
                break;
            }
        }
        checker.planning = false;
        checker.analysis_exhausted = false;
        checker.quota_error = None;
        if let Err(error) = checker.function(function) {
            checker.record_error(error);
        }
        #[cfg(test)]
        stats.push(FunctionCheckStats {
            function_ordinal: checker.function_ordinal,
            passes: checker.current_passes,
            planning_calls: checker.current_planning_calls,
        });
        for (ty, origin) in checker.pending_depth.take() {
            if let Some(resolved) = checker.resolved_syntax_type(&ty) {
                let concrete = Type::from(&resolved);
                if checker.depth_may_contain_generator(&concrete) {
                    late_depth
                        .entry(span_key(&origin))
                        .or_insert((concrete, origin));
                } else if let Err(error) = checker.check_depth_now(&concrete, &origin)
                    && !checker.errors.iter().any(|existing| {
                        existing.code == error.code
                            && existing.primary_span == error.primary_span
                            && existing.data == error.data
                    })
                {
                    checker.record_error(error);
                }
            }
        }
        for (&site, draft) in checker.closures.clone().iter() {
            let mut saved = draft.clone();
            for capture in &mut saved.captures {
                if let Some(ty) = checker.resolved_syntax_type(&capture.5) {
                    capture.5 = Type::from(&ty);
                }
            }
            checker.saved_closures.insert(site, saved);
        }
        if function.is_generator {
            let mut slots = Vec::new();
            let mut bindings: Vec<_> = checker.graph.locals.clone().into_iter().collect();
            bindings.sort_by_key(|(site, _)| *site);
            for (site, known) in bindings {
                if checker.closure_local_sites.contains(&site) {
                    continue;
                }
                if let Some(ty) = checker.resolved_syntax_type(&known.ty) {
                    slots.push(Type::from(&ty));
                }
            }
            for hidden in checker.hidden_for.clone() {
                if let Some(ty) = checker.resolved_syntax_type(&hidden) {
                    slots.push(Type::from(&ty));
                }
            }
            checker
                .generator_slots
                .insert(function.name_span.start, slots);
        }
        for (&site, draft) in checker.closures.clone().iter() {
            let mut captures = Vec::with_capacity(draft.captures.len());
            for (name, mode, usage, source, capture_span, ty, mutable) in &draft.captures {
                if let Some(ty) = checker.resolved_syntax_type(ty) {
                    captures.push(ClosureCaptureInfo {
                        name: name.clone(),
                        mode: *mode,
                        usage: *usage,
                        source: source.clone(),
                        capture_span: capture_span.clone(),
                        ty,
                        mutable: *mutable,
                    });
                }
            }
            let mut params = Vec::with_capacity(draft.params.len());
            for (name, mode, name_span, ty) in &draft.params {
                if let Some(ty) = checker.resolved_syntax_type(ty) {
                    params.push((name.clone(), *mode, ty));
                } else {
                    let mut error = names::diagnostic(
                        "NX-E1111",
                        None,
                        Stage::Types,
                        Operation::TypesCheck,
                        name_span.clone(),
                    );
                    error.related_spans.push(RelatedSpan {
                        role: RelatedRole::Declaration,
                        span: name_span.clone(),
                    });
                    checker.record_error(error);
                }
            }
            let result = checker
                .resolved_syntax_type(&draft.result)
                .unwrap_or(SyntaxType::Unit);
            inferred.closures.insert(
                site,
                ClosureInfo {
                    site,
                    required_mode: captures.iter().fold(Mode::Read, |current, capture| {
                        let needed =
                            if capture.mode == CaptureMode::Edit && capture.usage == Mode::Read {
                                Mode::Inout
                            } else {
                                capture.usage
                            };
                        match (current, needed) {
                            (Mode::Consume, _) | (_, Mode::Consume) => Mode::Consume,
                            (Mode::Inout, _) | (_, Mode::Inout) => Mode::Inout,
                            _ => Mode::Read,
                        }
                    }),
                    captures,
                    params,
                    result,
                },
            );
        }
        for (&at, known) in checker.graph.locals.clone().iter() {
            if let Some(id) = known.variable {
                let resolved = checker.concrete(id).and_then(|(ty, _)| syntax_type(&ty));
                inferred.bindings.insert(
                    at,
                    resolved.map_or_else(
                        || {
                            if matches!(internal_state(&known.ty), Some(Type::Never)) {
                                ResolvedType::Never
                            } else {
                                ResolvedType::Unresolved
                            }
                        },
                        ResolvedType::Concrete,
                    ),
                );
            } else {
                let resolved = checker.resolved_syntax_type(&known.ty).map_or_else(
                    || {
                        if matches!(internal_state(&known.ty), Some(Type::Never)) {
                            ResolvedType::Never
                        } else {
                            ResolvedType::Unresolved
                        }
                    },
                    ResolvedType::Concrete,
                );
                inferred.bindings.insert(at, resolved);
            }
        }
        for (span, ty) in checker.graph.expression_types.clone() {
            let resolved = match internal_state(&ty) {
                Some(Type::Never) => ResolvedType::Never,
                Some(Type::Tainted) => checker
                    .graph
                    .conditional_result(&span)
                    .cloned()
                    .and_then(|nominal| checker.resolved_syntax_type(&nominal))
                    .filter(|_| checker.errors.is_empty())
                    .map_or(ResolvedType::Unresolved, ResolvedType::Concrete),
                _ => checker
                    .resolved_syntax_type(&ty)
                    .map_or(ResolvedType::Unresolved, ResolvedType::Concrete),
            };
            inferred.expressions.insert(span, resolved);
        }
        for (span, id) in checker.graph.expr_vars.clone() {
            let resolved = checker.concrete(id).and_then(|(ty, _)| syntax_type(&ty));
            if let Some(ty) = resolved {
                inferred
                    .expressions
                    .insert(span, ResolvedType::Concrete(ty));
            } else {
                inferred
                    .expressions
                    .entry(span)
                    .or_insert(ResolvedType::Unresolved);
            }
        }
    }
    // A denied global charge has one diagnostic, even if an enclosing
    // expression had already accumulated independent child errors. Some
    // enclosing statements suppress returned errors when tainted; retain the
    // first denied charge here and emit it at the compilation boundary.
    if let Some(quota) = checker.quota_error.take() {
        // The current function has not reached its pending-domain closure.
        // Its nominal results may later fail, so consumers depending on them
        // are not converged diagnostics. Prior completed functions have no
        // pending entries and keep their established diagnostics.
        let tentative_results: HashSet<_> = checker
            .graph
            .obligations
            .iter()
            .filter_map(|entry| match entry {
                ObligationKind::BinaryDomain(domain) => Some(span_key(&domain.result)),
                ObligationKind::DomainNominal(result) => Some(span_key(&result.call)),
                _ => None,
            })
            .chain(checker.graph.marked(OccurrenceFlag::FailedJoin))
            .collect();
        checker.suppress_result_dependents(&tentative_results);
        checker
            .errors
            .retain(|error| error.operation != Operation::AnalysisLimit);
        checker.record_error(quota);
    }
    checker.depth_memo_ready = true;
    let mut late_depth: Vec<_> = late_depth.into_values().collect();
    late_depth.sort_by_key(|(_, origin)| (origin.start, origin.end));
    for (ty, origin) in late_depth {
        if let Err(error) = checker.check_depth_now(&ty, &origin)
            && !checker.errors.iter().any(|existing| {
                existing.code == error.code
                    && existing.primary_span == error.primary_span
                    && existing.data == error.data
            })
        {
            checker.record_error(error);
        }
    }
    if ast.f5_operators {
        if !ast.consts.is_empty() {
            // Before L09, the module's constant capability is pending. Other
            // temporary capability reports do not add actionable information.
            checker
                .errors
                .retain(|error| error.feature_id.as_deref() != Some("implementation_limit"));
        }
        checker
            .errors
            .extend(ast.consts.iter().map(super::pending_f5::const_pending));
    }
    names::sort_diagnostics(&mut checker.errors);
    if !checker.errors.is_empty() {
        record_unresolved_sites(ast, &mut inferred);
    }
    TypeCheckReport {
        inferred,
        errors: checker.errors,
        #[cfg(test)]
        stats,
    }
}

#[cfg(test)]
mod j16_probe_types {
    use super::*;
    use std::process::Command;

    fn source(family: &str) -> String {
        match family {
            "block_closure_alternation" => format!(
                "fn Main(){{{}{} }}",
                "{let f=fn(){".repeat(30),
                "};}".repeat(30)
            ),
            "else_if_body" => format!(
                "fn Main(){{{}{} }}",
                "if false{}else if true{".repeat(126),
                "}".repeat(126)
            ),
            "for_body" => format!(
                "gen fn Gen()->I64{{yield 1;}}fn Main(){{{}{} }}",
                "for x in Gen(){".repeat(126),
                "}".repeat(126)
            ),
            "index_branch" => {
                let mut body = "let z=1;".to_owned();
                for _ in 0..30 {
                    body = format!("let v=[1][if true then {{{body}value:0;}} else 0];");
                }
                format!("fn Main(){{{body}}}")
            }
            "index_nonarray" => {
                let mut body = "let z=1;".to_owned();
                for _ in 0..30 {
                    body = format!("let v=true[if true then {{{body}value:0;}} else 0];");
                }
                format!("fn Main(){{{body}}}")
            }
            "index_deferred" => {
                let mut body = "let z=1;".to_owned();
                for _ in 0..30 {
                    body =
                        format!("var a=[];let v=a[if true then {{{body}value:0;}} else 0];a=[1];");
                }
                format!("fn Main(){{{body}}}")
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
            "index" => format!(
                "fn Main(){{let a={}1{};let x=a{};}}",
                "[".repeat(126),
                "]".repeat(126),
                "[0]".repeat(126)
            ),
            "binary_left" => format!("fn Main(){{let x=1{};}}", "+1".repeat(126)),
            "binary_right" => format!(
                "fn Main(){{let x={}1{};}}",
                "1+(".repeat(126),
                ")".repeat(126)
            ),
            "binary_error_right" => format!(
                "fn Main(){{let x={}true{};}}",
                "true+(".repeat(60),
                ")".repeat(60)
            ),
            "call_arg" => format!(
                "fn F(x:view I64)->I64{{return x;}}fn Main(){{let x={}1{};}}",
                "F(".repeat(126),
                ")".repeat(126)
            ),
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
            "else_if" => format!("fn Main(){{{}{{}}}}", "if true{}else ".repeat(126)),
            "if_expr" => {
                let mut expr = "1".to_owned();
                for _ in 0..126 {
                    expr = format!("if true then ({expr}) else 0");
                }
                format!("fn Main(){{let x={expr};}}")
            }
            "if_stmt" => format!(
                "fn Main(){{{}{} }}",
                "if true{".repeat(126),
                "}".repeat(126)
            ),
            "while" => format!(
                "fn Main(){{{}{} }}",
                "while true{".repeat(126),
                "}".repeat(126)
            ),
            "closure" => format!(
                "fn Main(){{let f={}1{};}}",
                "fn(){let g=".repeat(63),
                ";}".repeat(63)
            ),
            "closure_called" => {
                let mut expr = "1".to_owned();
                for _ in 0..40 {
                    expr = format!("(fn ()->I64{{return {expr};}})()");
                }
                format!("fn Main(){{let x={expr};}}")
            }
            "match_scrutinee" => {
                let mut expr = "1".to_owned();
                for _ in 0..60 {
                    expr = format!("match ({expr}){{_=>{{value:1;}}}}");
                }
                format!("fn Main(){{let x={expr};}}")
            }
            "match_expr" => {
                let mut expr = "1".to_owned();
                for _ in 0..63 {
                    expr = format!("match 1{{_=>{{value:{expr};}}}}");
                }
                format!("fn Main(){{let x={expr};}}")
            }
            "match_stmt" => {
                let mut body = String::new();
                for _ in 0..60 {
                    body = format!("match 1{{_=>{{{body}}}}}");
                }
                format!("fn Main(){{{body}}}")
            }
            "pattern_group" => [
                "fn Main(){match 1{",
                &"(".repeat(126),
                "_",
                &")".repeat(126),
                "=>{}}}",
            ]
            .concat(),
            "pattern_guard_arms" => [
                "fn Main(){match 0{",
                &"0 if true=>{},".repeat(126),
                "_=>{}}}",
            ]
            .concat(),
            "pattern_integer_arms" => [
                "fn Main(){match 0{",
                &(0..126).map(|n| format!("{n}=>{{}},")).collect::<String>(),
                "_=>{}}}",
            ]
            .concat(),
            _ => panic!("unknown family: {family}"),
        }
    }

    #[test]
    fn child() {
        let Ok(family) = std::env::var("J16_TYPES_FAMILY") else {
            return;
        };
        let source = std::env::var("J16_TYPES_SOURCE")
            .map(|path| std::fs::read_to_string(path).unwrap())
            .unwrap_or_else(|_| source(&family));
        let ast = parse::parse_new_profile("stack.n", &source).unwrap();
        let resolved = names::resolve_with_option("stack.n", &source, &ast, true).unwrap();
        let report = std::thread::Builder::new()
            .stack_size(
                std::env::var("J16_STACK_KIB")
                    .ok()
                    .and_then(|value| value.parse::<usize>().ok())
                    .unwrap_or(204)
                    * 1024,
            )
            .spawn(move || check_with_partial_limit("stack.n", &source, &ast, &resolved, 1_000_000))
            .unwrap()
            .join()
            .unwrap();
        if let Ok(expected) = std::env::var("J16_TYPES_EXPECT_ERROR") {
            assert_eq!(
                report.errors.first().map(|error| error.code.as_str()),
                Some(expected.as_str())
            );
        } else {
            assert!(report.errors.is_empty(), "{family}: {:?}", report.errors);
        }
    }

    #[test]
    fn s3_types_or_checker_child() {
        let Ok(source_path) = std::env::var("I07C_S3_SOURCE") else {
            return;
        };
        let source = std::fs::read_to_string(source_path).unwrap();
        let ast = parse::parse_new_profile("stack.n", &source).unwrap();
        let mode = std::env::var("I07C_S3_MODE").unwrap();
        let kib = std::env::var("I07C_S3_STACK_KIB")
            .unwrap()
            .parse::<usize>()
            .unwrap();
        let resolved = if mode == "TYPES" {
            Some(names::resolve_with_option("stack.n", &source, &ast, true).unwrap())
        } else {
            None
        };
        let resolved_ref = resolved.as_ref();
        let source_ref = &source;
        let ast_ref = &ast;
        let report = std::thread::scope(|scope| {
            std::thread::Builder::new()
                .stack_size(kib * 1024)
                .spawn_scoped(scope, move || {
                    if let Some(resolved) = resolved_ref {
                        check_with_partial_limit(
                            "stack.n", source_ref, ast_ref, resolved, 1_000_000,
                        )
                    } else {
                        let resolved =
                            names::resolve_with_option("stack.n", source_ref, ast_ref, true)
                                .unwrap();
                        check_with_partial_limit(
                            "stack.n", source_ref, ast_ref, &resolved, 1_000_000,
                        )
                    }
                })
                .unwrap()
                .join()
                .unwrap()
        });
        let wide = if let Some(resolved) = resolved.as_ref() {
            check_with_partial_limit("stack.n", &source, &ast, resolved, 1_000_000)
        } else {
            let resolved = names::resolve_with_option("stack.n", &source, &ast, true).unwrap();
            check_with_partial_limit("stack.n", &source, &ast, &resolved, 1_000_000)
        };
        assert_eq!(report.errors, wide.errors);
        if let Ok(expected) = std::env::var("I07C_S3_EXPECT_ERROR") {
            assert_eq!(
                report.errors.first().map(|e| e.code.as_str()),
                Some(expected.as_str())
            );
        } else {
            assert!(report.errors.is_empty(), "{:?}", report.errors);
        }
    }

    #[test]
    fn s3_clone_selector_child() {
        let Ok(source_path) = std::env::var("I07C_S3_SOURCE") else {
            return;
        };
        let source = std::fs::read_to_string(source_path).unwrap();
        let ast = parse::parse_new_profile("clone.n", &source).unwrap();
        let Statement::Let {
            value: Expr::Index { index, .. },
            ..
        } = &ast.functions[0].body.statements[0]
        else {
            panic!("expected index selector");
        };
        let drop_in_worker = std::env::var_os("I07C_S3_DROP_IN_WORKER").is_some();
        let cloned = std::thread::scope(|scope| {
            std::thread::Builder::new()
                .stack_size(204 * 1024)
                .spawn_scoped(scope, || {
                    let cloned = clone_projection_selector(index);
                    if drop_in_worker {
                        drop(cloned);
                        None
                    } else {
                        Some(cloned)
                    }
                })
                .unwrap()
                .join()
                .unwrap()
        });
        if let Some(cloned) = cloned {
            assert_eq!(cloned, **index);
        }
    }

    #[test]
    fn parent() {
        let exe = std::env::current_exe().unwrap();
        for family in [
            "block_closure_alternation",
            "else_if_body",
            "for_body",
            "index_branch",
            "index_deferred",
            "block",
            "group",
            "array",
            "mixed",
            "yield_group",
            "binary_left",
            "binary_right",
            "call_arg",
            "match_scrutinee",
            "match_expr",
            "match_stmt",
            "pattern_group",
            "pattern_guard_arms",
            "pattern_integer_arms",
            "if_expr",
            "else_if",
            "closure",
            "closure_called",
            "index",
            "if_stmt",
            "while",
        ] {
            let output = Command::new(&exe)
                .args([
                    "--exact",
                    "check::types::j16_probe_types::child",
                    "--nocapture",
                ])
                .env("J16_TYPES_FAMILY", family)
                .output()
                .unwrap();
            assert!(
                output.status.success(),
                "{family}: {} {}",
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr)
            );
        }
        for (family, error) in [
            ("binary_error_right", "NX-E0102"),
            ("index_nonarray", "NX-E0102"),
        ] {
            let output = Command::new(&exe)
                .args(["--exact", "check::types::j16_probe_types::child"])
                .env("J16_TYPES_FAMILY", family)
                .env("J16_TYPES_EXPECT_ERROR", error)
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

#[cfg(test)]
mod quota_tests {
    use super::*;

    #[test]
    fn j16_direct_pattern_type_child() {
        let Ok(kind) = std::env::var("J16_DIRECT_PATTERN_TYPE") else {
            return;
        };
        let span = Span {
            file: "pattern.n".into(),
            start: 0,
            end: 1,
        };
        let mut pattern = Pattern::Wildcard(span.clone());
        let mut ty = Type::I64;
        for _ in 0..126 {
            pattern = Pattern::Constructor {
                name: kind.clone(),
                args: vec![pattern],
                span: span.clone(),
            };
            ty = match kind.as_str() {
                "Some" => Type::option(Arc::new(ty)),
                "Ok" => Type::result(Arc::new(ty), Arc::new(Type::I64)),
                "Err" => Type::result(Arc::new(Type::I64), Arc::new(ty)),
                _ => unreachable!(),
            };
        }
        let pattern = Arc::new(pattern);
        let ty = Arc::new(ty);
        let ast = parse::parse_new_profile("pattern.n", "fn Main(){}").unwrap();
        let resolved = names::resolve_with_option("pattern.n", "fn Main(){}", &ast, true).unwrap();
        std::thread::Builder::new()
            .stack_size(204 * 1024)
            .spawn(move || {
                let mut checker = empty_checker(&resolved);
                checker.pattern_type(&pattern, &ty, &span).unwrap();
                assert_eq!(checker.solver_steps, 127);
            })
            .unwrap()
            .join()
            .unwrap();
    }

    #[test]
    fn j16_direct_pattern_type_parent() {
        for kind in ["Some", "Ok", "Err"] {
            let output = std::process::Command::new(std::env::current_exe().unwrap())
                .args([
                    "--exact",
                    "check::types::quota_tests::j16_direct_pattern_type_child",
                ])
                .env("J16_DIRECT_PATTERN_TYPE", kind)
                .output()
                .unwrap();
            assert!(
                output.status.success(),
                "{kind}: {} {}",
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr)
            );
        }
    }

    #[test]
    fn reduced_quota_withholds_consumer_of_unclosed_binary_domain() {
        // §4.1.2.5: the result of q<q cannot supply the Bool witness for
        // the following array element once q is fixed to Array<I64>.
        // §11.1: a denied charge exposes only diagnostics that have already
        // converged. Limits 11..=14 stop before the pending domain closes;
        // the dependent join no longer charges a nonexistent source pair.
        let source = "fn Main(){var q=[];let a=[(q<q),(if [0] then 18 else false)];q=[1];}";
        let ast = crate::frontend::parse::parse("quota.n", source).unwrap();
        let resolved = names::resolve("quota.n", source, &ast).unwrap();
        let full = check_with_partial_limit("quota.n", source, &ast, &resolved, 1_000_000);
        // The concrete twin also retains the second union's independent
        // E1303 after its condition fails (§4.1.2.5).
        assert_eq!(full.errors.len(), 3, "{:?}", full.errors);
        assert_eq!(full.errors[0].code, "NX-E0102");
        assert_eq!(full.errors[0].subcause.as_deref(), Some("type_mismatch"));
        assert_eq!(
            full.errors[0].primary_span.as_ref().map(span_key),
            Some((28, 29))
        );
        assert_eq!(full.errors[1].code, "NX-E0102");
        assert_eq!(
            full.errors[1].primary_span.as_ref().map(span_key),
            Some((36, 39))
        );
        assert_eq!(full.errors[2].code, "NX-E1303");
        assert_eq!(
            full.errors[2].primary_span.as_ref().map(span_key),
            Some((53, 58))
        );
        for limit in 11..=14 {
            let reduced = check_with_partial_limit("quota.n", source, &ast, &resolved, limit);
            let quota = reduced
                .errors
                .iter()
                .find(|d| d.code == "NX-E0003")
                .unwrap();
            assert_eq!(
                quota.data.get("observed"),
                Some(&DataValue::UInt(limit + 1))
            );
            for diagnostic in reduced.errors.iter().filter(|d| d.code != "NX-E0003") {
                assert!(
                    full.errors.iter().any(|baseline| {
                        baseline.code == diagnostic.code
                            && baseline.subcause == diagnostic.subcause
                            && baseline.primary_span == diagnostic.primary_span
                            && baseline.related_spans == diagnostic.related_spans
                            && baseline.data == diagnostic.data
                    }),
                    "limit {limit}: {diagnostic:?}"
                );
            }
        }
    }

    #[test]
    fn diagnostic_site_ownership_survives_distinct_causes_and_preserves_siblings() {
        // §13.9 and §4.7: a deferred join may reach an operand after its
        // immediate check. §4.1.1 suppresses that consequence by site, while
        // a distinct sibling retains its own diagnosis.
        let source = "fn Main(){}";
        let ast = crate::frontend::parse::parse("quota.n", source).unwrap();
        let resolved = names::resolve("quota.n", source, &ast).unwrap();
        let mut checker = empty_checker(&resolved);
        let first = site(4);
        let enclosing = Span {
            file: "solver.n".into(),
            start: 4,
            end: 6,
        };
        let sibling = site(9);
        for (code, span) in [
            ("NX-E0102", first.clone()),
            ("NX-E1303", first.clone()),
            ("NX-E1303", enclosing),
            ("NX-E1303", sibling.clone()),
        ] {
            checker.record_error(names::diagnostic(
                code,
                None,
                Stage::Types,
                Operation::TypesCheck,
                span,
            ));
        }
        assert_eq!(
            checker.graph.emitted_sites,
            HashSet::from([(4, 5), (4, 6), (9, 10)])
        );
        assert_eq!(checker.errors.len(), 3);
        assert_eq!(checker.errors[0].code, "NX-E0102");
        assert_eq!(checker.errors[1].code, "NX-E1303");
        assert_eq!(checker.errors[2].code, "NX-E1303");
    }

    fn site(start: u64) -> Span {
        Span {
            file: "solver.n".into(),
            start,
            end: start + 1,
        }
    }

    #[test]
    fn operand_evidence_survives_nested_pending_repartition() {
        let resolved = Resolved {
            signatures: HashMap::new(),
            constants: HashMap::new(),
            uses: HashMap::new(),
            calls: HashMap::new(),
        };
        let mut checker = empty_checker(&resolved);
        let outside = checker.new_variable(&site(0)).unwrap();
        let inside = checker.new_variable(&site(15)).unwrap();
        checker
            .graph
            .register(ObligationKind::Domain(PendingDomain {
                variable: outside,
                operand: site(0),
                domain: "I64",
            }));
        checker
            .graph
            .register(ObligationKind::Domain(PendingDomain {
                variable: inside,
                operand: site(15),
                domain: "I64",
            }));
        let removed = checker.take_pending_kind(|entry| match entry {
            ObligationKind::Domain(domain) if domain.variable == outside => Ok(domain),
            other => Err(other),
        });
        assert_eq!(removed.len(), 1);
        checker
            .graph
            .register(ObligationKind::Domain(PendingDomain {
                variable: outside,
                operand: site(40),
                domain: "I64",
            }));
        let operand = Expr::Integer(
            "1".into(),
            Span {
                file: "solver.n".into(),
                start: 10,
                end: 30,
            },
        );
        let evidence = checker.pending_operand_evidence(&operand).unwrap();
        assert_eq!(evidence.domains, vec![(inside, "I64")]);
    }

    #[test]
    fn solver_states_and_representatives_keep_their_distinct_rules() {
        let resolved = Resolved {
            signatures: HashMap::new(),
            constants: HashMap::new(),
            uses: HashMap::new(),
            calls: HashMap::new(),
        };
        let mut checker = empty_checker(&resolved);
        assert_eq!(
            checker
                .unify_independent(Type::Never, Type::I64, site(0), site(1), site(1), false)
                .unwrap(),
            Type::I64
        );
        assert_eq!(
            checker
                .unify_independent(Type::Tainted, Type::I64, site(0), site(1), site(1), false)
                .unwrap(),
            Type::Tainted
        );

        let a = checker.new_variable(&site(2)).unwrap();
        let b = checker.new_variable(&site(3)).unwrap();
        let c = checker.new_variable(&site(4)).unwrap();
        let d = checker.new_variable(&site(5)).unwrap();
        checker.union(c, d, site(6)).unwrap();
        assert_eq!(
            checker.root(c),
            c,
            "the lower stable id wins an equal-size union"
        );
        checker.union(a, b, site(7)).unwrap();
        checker.union(a, c, site(8)).unwrap();
        assert_eq!(
            checker.root(d),
            a,
            "equal-size groups use the lower stable id"
        );
        // ADR §4.1.2.3: a projection follows aliases of its base before
        // applying the conditional element constraint.
        assert_eq!(checker.representative(d), a);

        let small = checker.new_variable(&site(9)).unwrap();
        checker.union(small, c, site(10)).unwrap();
        assert_eq!(
            checker.root(small),
            a,
            "the larger representative remains the root"
        );
    }

    #[test]
    fn right_hand_bottom_and_taint_keep_their_algebraic_states() {
        // F0-0002 §4.1.1: Never is the bottom in either order; an invalid
        // right operand contaminates the equality without a derived error.
        let resolved = Resolved {
            signatures: HashMap::new(),
            constants: HashMap::new(),
            uses: HashMap::new(),
            calls: HashMap::new(),
        };
        let mut checker = empty_checker(&resolved);
        assert_eq!(
            checker
                .unify_independent(Type::I64, Type::Never, site(1), site(2), site(3), false)
                .unwrap(),
            Type::I64
        );
        assert_eq!(
            checker
                .unify_independent(Type::I64, Type::Tainted, site(4), site(5), site(6), false)
                .unwrap(),
            Type::Tainted
        );
    }

    #[test]
    fn concrete_deferred_branch_join_reports_branch_mismatch() {
        // F0-0002 §4.7: two previously constrained representatives retain
        // the branch-specific diagnostic when their values are joined.
        let resolved = Resolved {
            signatures: HashMap::new(),
            constants: HashMap::new(),
            uses: HashMap::new(),
            calls: HashMap::new(),
        };
        let mut checker = empty_checker(&resolved);
        let left = checker.new_variable(&site(1)).unwrap();
        let right = checker.new_variable(&site(2)).unwrap();
        checker
            .bind_independent(left, Type::I64, site(3), site(4), false)
            .unwrap();
        checker
            .bind_independent(right, Type::Bool, site(5), site(6), false)
            .unwrap();
        let error = checker
            .unify_independent(
                Type::Deferred(left),
                Type::Deferred(right),
                site(3),
                site(5),
                site(7),
                true,
            )
            .unwrap_err();
        assert_eq!(
            (error.code.as_str(), error.operation),
            ("NX-E1303", Operation::TypesCheck)
        );
        assert_eq!(
            error.primary_span.as_ref().map(|s| (s.start, s.end)),
            Some((7, 8))
        );
    }

    #[test]
    fn union_retains_earliest_concrete_origin_and_copyable_domain() {
        // F0-0002 §§4.1.1/13.15: same-type representatives keep the first
        // origin; a copyable obligation rejects a resource on the other root.
        let resolved = Resolved {
            signatures: HashMap::new(),
            constants: HashMap::new(),
            uses: HashMap::new(),
            calls: HashMap::new(),
        };
        let mut checker = empty_checker(&resolved);
        let left = checker.new_variable(&site(1)).unwrap();
        let right = checker.new_variable(&site(2)).unwrap();
        checker
            .bind_independent(left, Type::I64, site(10), site(11), false)
            .unwrap();
        checker
            .bind_independent(right, Type::I64, site(20), site(21), false)
            .unwrap();
        checker.union(left, right, site(22)).unwrap();
        let error = checker
            .bind_independent(left, Type::Bool, site(30), site(31), false)
            .unwrap_err();
        assert_eq!(error.code, "NX-E0102");
        assert!(
            error
                .related_spans
                .iter()
                .any(|r| r.role == RelatedRole::ExpectedType && r.span.start == 10)
        );

        let mut checker = empty_checker(&resolved);
        let left = checker.new_variable(&site(1)).unwrap();
        let right = checker.new_variable(&site(2)).unwrap();
        checker.require_copyable(left, &site(3), false).unwrap();
        checker
            .bind_independent(right, Type::MockResource, site(4), site(5), false)
            .unwrap();
        let error = checker.union(left, right, site(6)).unwrap_err();
        assert_eq!(error.code, "NX-E0102");
        assert_eq!(
            error.data.get("expected_type"),
            Some(&DataValue::String("domain:copyable_data".into()))
        );
    }

    #[test]
    fn deferred_alias_union_charges_the_written_conflict_site() {
        // The current solver charges this union (adr_gap_union_charge);
        // this regression fixes the
        // current charge site at the operand passed to unify.
        let resolved = Resolved {
            signatures: HashMap::new(),
            constants: HashMap::new(),
            uses: HashMap::new(),
            calls: HashMap::new(),
        };
        let mut checker = empty_checker(&resolved);
        let left = checker.new_variable(&site(1)).unwrap();
        let right = checker.new_variable(&site(2)).unwrap();
        checker.solver_limit = checker.solver_steps + 1;
        let error = checker
            .unify_independent(
                Type::Deferred(left),
                Type::Deferred(right),
                site(3),
                site(4),
                site(7),
                false,
            )
            .unwrap_err();
        assert_eq!(error.code, "NX-E0003");
        assert_eq!(
            error.primary_span.as_ref().map(|s| (s.start, s.end)),
            Some((7, 8))
        );
        assert_eq!(
            error.data.get("observed"),
            Some(&DataValue::UInt(checker.solver_limit + 1))
        );

        // Binding a deferred right-hand alias uses the same union primitive.
        // Its declaration origin is distinct from the rejected use site.
        let mut checker = empty_checker(&resolved);
        let left = checker.new_variable(&site(1)).unwrap();
        let right = checker.new_variable(&site(2)).unwrap();
        checker.solver_limit = checker.solver_steps;
        let error = checker
            .bind_independent(left, Type::Deferred(right), site(3), site(7), false)
            .unwrap_err();
        assert_eq!(error.code, "NX-E0003");
        assert_eq!(
            error.primary_span.as_ref().map(|s| (s.start, s.end)),
            Some((7, 8))
        );
        assert_eq!(
            error.data.get("observed"),
            Some(&DataValue::UInt(checker.solver_limit + 1))
        );
    }

    #[test]
    fn overlapping_type_origins_choose_the_earlier_start() {
        // F0-0002 §§4.1.1, 13.15 order conflicting constraints by their
        // source origin starts. A later start can lie inside the earlier span.
        let resolved = Resolved {
            signatures: HashMap::new(),
            constants: HashMap::new(),
            uses: HashMap::new(),
            calls: HashMap::new(),
        };
        let mut checker = empty_checker(&resolved);
        let later = checker.new_variable(&site(0)).unwrap();
        let earlier = checker.new_variable(&site(1)).unwrap();
        let wide = Span {
            file: "quota.n".into(),
            start: 2,
            end: 12,
        };
        checker
            .bind_independent(later, Type::I64, site(6), site(6), false)
            .unwrap();
        checker
            .bind_independent(earlier, Type::Bool, wide.clone(), site(2), false)
            .unwrap();
        let error = checker.union(later, earlier, site(13)).unwrap_err();
        assert_eq!(
            error.data.get("expected_type"),
            Some(&DataValue::String("Bool".into()))
        );
        assert_eq!(
            error.data.get("found_type"),
            Some(&DataValue::String("I64".into()))
        );
        assert!(
            error
                .related_spans
                .iter()
                .any(|r| r.role == RelatedRole::ExpectedType && r.span == wide)
        );
    }

    #[test]
    fn inout_value_delegates_one_contextual_pair_to_its_operand() {
        // F0-0002 §§4.1.1/11.1: a transparent access mode propagates the
        // type to its child and does not charge a second wrapper pair.
        let resolved = Resolved {
            signatures: HashMap::new(),
            constants: HashMap::new(),
            uses: HashMap::new(),
            calls: HashMap::new(),
        };
        let value = Expr::Unary {
            mode: Mode::Inout,
            value: Box::new(Expr::Bool(true, site(1))),
            span: site(0),
        };
        let mut checker = empty_checker(&resolved);
        checker.solver_limit = 1;
        assert_eq!(
            checker.expr(&value, Some(&Type::Bool), None).unwrap(),
            Type::Bool
        );
        assert_eq!(checker.solver_steps, 1);
        let wrong = Expr::Unary {
            mode: Mode::Inout,
            value: Box::new(Expr::Integer("1".into(), site(3))),
            span: site(2),
        };
        let mut checker = empty_checker(&resolved);
        let error = checker.expr(&wrong, Some(&Type::Bool), None).unwrap_err();
        assert_eq!(
            (error.code.as_str(), error.subcause.as_deref()),
            ("NX-E0102", Some("type_mismatch"))
        );
        assert_eq!(
            error.primary_span.as_ref().map(|s| (s.start, s.end)),
            Some((3, 4))
        );
    }

    #[test]
    fn failed_deferred_array_element_keeps_constraint_for_later_elements() {
        // F0-0002 §§4.1.1/13.9: an unresolved element type constrained to
        // Bool stays in the array state after one I64 conflict. Taking it
        // would let the second independent I64 element escape validation.
        let resolved = Resolved {
            signatures: HashMap::new(),
            constants: HashMap::new(),
            uses: HashMap::new(),
            calls: HashMap::new(),
        };
        let mut checker = empty_checker(&resolved);
        let child = checker.new_variable(&site(1)).unwrap();
        checker
            .bind_independent(child, Type::Bool, site(1), site(1), false)
            .unwrap();
        let expected = Type::Array(Arc::new(Type::Deferred(child)));
        let array = Expr::Array(
            vec![
                Expr::Integer("1".into(), site(3)),
                Expr::Integer("2".into(), site(5)),
            ],
            site(2),
        );
        assert_eq!(
            checker.expr(&array, Some(&expected), None).unwrap(),
            Type::Tainted
        );
        assert_eq!(checker.errors.len(), 2, "{:?}", checker.errors);
        for (error, primary) in checker.errors.iter().zip([(3, 4), (5, 6)]) {
            assert_eq!(
                (error.code.as_str(), error.subcause.as_deref()),
                ("NX-E0102", Some("type_mismatch"))
            );
            assert_eq!(
                error.primary_span.as_ref().map(|s| (s.start, s.end)),
                Some(primary)
            );
            assert_eq!(
                error.data.get("expected_type"),
                Some(&DataValue::String("Bool".into()))
            );
            assert_eq!(
                error.data.get("found_type"),
                Some(&DataValue::String("I64".into()))
            );
        }
    }

    #[test]
    fn contextual_all_divergent_if_retains_expected_type_and_divergence() {
        // F0-0002 §4.1.1: Never contributes no normal value. The written
        // I64 context is kept for checking while the flow remains divergent.
        let source = "fn Main(){let x:I64=(if true then {return;}else{return;});}";
        let ast = crate::frontend::parse::parse("solver.n", source).unwrap();
        let resolved = names::resolve("solver.n", source, &ast).unwrap();
        let Statement::Let { value, .. } = &ast.functions[0].body.statements[0] else {
            panic!("expected let")
        };
        let mut checker = empty_checker(&resolved);
        assert_eq!(
            checker.expr(value, Some(&Type::I64), None).unwrap(),
            Type::I64
        );
        assert!(
            checker
                .graph
                .has(&span_key(value.span()), OccurrenceFlag::Deferred)
        );
        assert!(
            checker
                .graph
                .has(&span_key(value.span()), OccurrenceFlag::Divergent)
        );
    }

    #[test]
    fn nested_divergent_branch_contributes_no_normal_type_under_context() {
        // F0-0002 §4.1.1: even with an expected I64, a branch whose value
        // always returns cannot become a normal I64 continuation.
        let source = "fn Main(){let x:I64=(if true then {value:(if true then {return;}else{return;});} else {value:1;});}";
        let ast = crate::frontend::parse::parse("solver.n", source).unwrap();
        let resolved = names::resolve("solver.n", source, &ast).unwrap();
        let Statement::Let {
            value: Expr::Group(inner, _),
            ..
        } = &ast.functions[0].body.statements[0]
        else {
            panic!("expected grouped if")
        };
        let Expr::If { then_block, .. } = inner.as_ref() else {
            panic!("expected if")
        };
        let mut checker = empty_checker(&resolved);
        let (normal, failed, _) = checker
            .with_independent_fact(span_key(&then_block.span), |checker, fact| {
                checker.branch(fact, then_block, Some(&Type::I64), None, false, None, false)
            })
            .unwrap();
        assert_eq!(normal, None);
        assert!(!failed);
    }

    #[test]
    fn conflicting_union_reports_the_earliest_concrete_origin() {
        let resolved = Resolved {
            signatures: HashMap::new(),
            constants: HashMap::new(),
            uses: HashMap::new(),
            calls: HashMap::new(),
        };
        let mut checker = empty_checker(&resolved);
        let a = checker.new_variable(&site(0)).unwrap();
        let b = checker.new_variable(&site(1)).unwrap();
        checker
            .bind_independent(a, Type::I64, site(5), site(5), false)
            .unwrap();
        checker
            .bind_independent(b, Type::Bool, site(9), site(9), false)
            .unwrap();
        let error = checker.union(a, b, site(11)).unwrap_err();
        assert_eq!(
            error.data.get("expected_type"),
            Some(&DataValue::String("I64".into()))
        );
        assert_eq!(
            error.data.get("found_type"),
            Some(&DataValue::String("Bool".into()))
        );
        assert!(
            error
                .related_spans
                .iter()
                .any(|r| r.role == RelatedRole::ExpectedType && r.span == site(5))
        );
    }

    #[test]
    fn inferred_array_depth_accepts_127_and_128_and_rejects_129() {
        let resolved = Resolved {
            signatures: HashMap::new(),
            constants: HashMap::new(),
            uses: HashMap::new(),
            calls: HashMap::new(),
        };
        let mut checker = empty_checker(&resolved);
        let root = checker.new_variable(&site(0)).unwrap();
        let mut ty = Type::I64;
        for _ in 0..127 {
            ty = Type::Array(Arc::new(ty));
        }
        checker.graph.type_nodes.get_mut(&root).unwrap().concrete = Some((ty.clone(), site(0)));
        assert!(checker.check_one_inferred_depth(root, &site(0)).is_ok());
        ty = Type::Array(Arc::new(ty));
        checker.graph.type_nodes.get_mut(&root).unwrap().concrete = Some((ty, site(0)));
        assert!(checker.check_one_inferred_depth(root, &site(0)).is_ok());

        let mut chain = root;
        checker.graph.type_nodes.get_mut(&root).unwrap().concrete = None;
        for _ in 0..64 {
            let next = checker.new_variable(&site(1)).unwrap();
            checker.graph.type_node_mut(&chain).unwrap().array_child = Some(next);
            chain = next;
        }
        let mut inner = Type::I64;
        for _ in 0..65 {
            inner = Type::Array(Arc::new(inner));
        }
        checker.graph.type_nodes.get_mut(&chain).unwrap().concrete = Some((inner, site(2)));
        let error = checker
            .check_one_inferred_depth(root, &site(0))
            .unwrap_err();
        assert_eq!(error.data.get("observed"), Some(&DataValue::UInt(129)));
    }

    #[test]
    fn inferred_array_root_retains_depth_origin_after_later_constraint() {
        let resolved = Resolved {
            signatures: HashMap::new(),
            constants: HashMap::new(),
            uses: HashMap::new(),
            calls: HashMap::new(),
        };
        let mut checker = empty_checker(&resolved);
        let root = checker.new_variable(&site(0)).unwrap();
        let child = checker.require_array(root, &site(0)).unwrap();
        let mut ty = Type::I64;
        for _ in 0..128 {
            ty = Type::Array(Arc::new(ty));
        }
        checker.graph.type_nodes.get_mut(&child).unwrap().concrete = Some((ty, site(1)));
        let error = checker.check_inferred_depth().unwrap_err();
        assert_eq!(error.data.get("observed"), Some(&DataValue::UInt(129)));
    }

    #[test]
    fn deferred_visible_conversion_and_constructor_walk_stop_at_their_boundaries() {
        let resolved = Resolved {
            signatures: HashMap::new(),
            constants: HashMap::new(),
            uses: HashMap::new(),
            calls: HashMap::new(),
        };
        let mut checker = empty_checker(&resolved);
        let scalar = checker.new_variable(&site(0)).unwrap();
        checker
            .bind_independent(scalar, Type::I64, site(1), site(1), false)
            .unwrap();
        assert_eq!(
            checker.resolved_syntax_type(&Type::Deferred(scalar)),
            Some(SyntaxType::I64)
        );

        let root = checker.new_variable(&site(2)).unwrap();
        let mut current = root;
        for _ in 0..129 {
            let child = checker.new_variable(&site(3)).unwrap();
            checker.graph.type_node_mut(&current).unwrap().array_child = Some(child);
            current = child;
        }
        checker.graph.type_nodes.get_mut(&current).unwrap().concrete = Some((Type::I64, site(4)));
        assert!(checker.concrete(root).is_none());
    }

    #[test]
    fn constructor_walk_keeps_exact_depth_boundaries() {
        let resolved = Resolved {
            signatures: HashMap::new(),
            constants: HashMap::new(),
            uses: HashMap::new(),
            calls: HashMap::new(),
        };
        for length in [127, 128, 129] {
            let mut checker = empty_checker(&resolved);
            let root = checker.new_variable(&site(0)).unwrap();
            let mut current = root;
            for _ in 0..length {
                let next = checker.new_variable(&site(1)).unwrap();
                checker
                    .graph
                    .type_nodes
                    .get_mut(&current)
                    .unwrap()
                    .array_child = Some(next);
                current = next;
            }
            checker.graph.type_nodes.get_mut(&current).unwrap().concrete =
                Some((Type::I64, site(2)));
            assert_eq!(checker.concrete(root).is_some(), length <= 128);
        }
    }

    #[test]
    fn partial_type_table_preserves_never_and_taint_states() {
        let source = "fn Main(){var v=(if true then {return;}else{return;});let x=true+1;let y=x;}";
        let ast = crate::frontend::parse::parse("states.n", source).unwrap();
        let resolved = names::resolve("states.n", source, &ast).unwrap();
        let report = check_with_partial("states.n", source, &ast, &resolved);
        assert!(!report.errors.is_empty());
        let never = source.find("(if true").unwrap() as u64;
        let never_end = source.find(";let x").unwrap() as u64;
        assert_eq!(
            report.inferred.expressions.get(&(never, never_end)),
            Some(&ResolvedType::Never)
        );
        let tainted = source.find("y=x").unwrap() as u64 + 2;
        assert_eq!(
            report.inferred.expressions.get(&(tainted, tainted + 1)),
            Some(&ResolvedType::Unresolved)
        );
    }

    #[test]
    fn visible_conversion_keeps_internal_states_out_of_diagnostics() {
        let resolved = Resolved {
            signatures: HashMap::new(),
            constants: HashMap::new(),
            uses: HashMap::new(),
            calls: HashMap::new(),
        };
        let mut checker = empty_checker(&resolved);
        let unresolved = checker.new_variable(&site(0)).unwrap();
        for internal in [Type::Never, Type::Tainted, Type::Deferred(unresolved)] {
            assert_eq!(syntax_type(&internal), None);
            assert_eq!(pretty(&internal), None);
            assert_eq!(checker.resolved_syntax_type(&internal), None);
        }
        let element = checker.new_variable(&site(1)).unwrap();
        let array = Type::Array(Arc::new(Type::Deferred(element)));
        assert_eq!(checker.resolved_syntax_type(&array), None);
        checker
            .bind_independent(element, Type::Bool, site(2), site(2), false)
            .unwrap();
        assert_eq!(
            checker.resolved_syntax_type(&array),
            Some(SyntaxType::Array(Box::new(SyntaxType::Bool)))
        );
        assert_eq!(
            checker
                .unify_independent(Type::Never, array.clone(), site(3), site(4), site(4), false)
                .unwrap(),
            array
        );
        assert_eq!(
            checker
                .unify_independent(Type::Tainted, Type::I64, site(5), site(6), site(6), false)
                .unwrap(),
            Type::Tainted
        );
    }

    #[test]
    fn deferred_result_components_and_structural_array_elements_resolve_separately() {
        let resolved = Resolved {
            signatures: HashMap::new(),
            constants: HashMap::new(),
            uses: HashMap::new(),
            calls: HashMap::new(),
        };
        let mut checker = empty_checker(&resolved);
        let ok = checker.new_variable(&site(0)).unwrap();
        let err = checker.new_variable(&site(1)).unwrap();
        let result = Type::result(Arc::new(Type::Deferred(ok)), Arc::new(Type::Deferred(err)));
        assert_eq!(checker.resolved_syntax_type(&result), None);
        checker
            .bind_independent(ok, Type::I64, site(2), site(2), false)
            .unwrap();
        checker
            .bind_independent(err, Type::Bool, site(3), site(3), false)
            .unwrap();
        assert_eq!(
            checker.resolved_syntax_type(&result),
            Some(SyntaxType::Result(
                Box::new(SyntaxType::I64),
                Box::new(SyntaxType::Bool)
            ))
        );

        let array = checker.new_variable(&site(4)).unwrap();
        checker
            .bind_independent(
                array,
                Type::Array(Arc::new(Type::I64)),
                site(5),
                site(5),
                false,
            )
            .unwrap();
        let mismatch = checker
            .bind_independent(
                array,
                Type::Array(Arc::new(Type::Bool)),
                site(6),
                site(6),
                false,
            )
            .unwrap_err();
        assert_eq!(mismatch.code, "NX-E0102");
        assert_eq!(
            mismatch.data.get("expected_type"),
            Some(&DataValue::String("Array<I64>".into()))
        );
        assert_eq!(
            mismatch.data.get("found_type"),
            Some(&DataValue::String("Array<Bool>".into()))
        );
    }

    #[test]
    fn concrete_arrays_record_divergence_only_from_never_elements() {
        for (source, divergent) in [
            ("fn Main(){let x=[1];}", false),
            (
                "fn Main(){let x=[(if true then {return;}else{return;}),1];}",
                true,
            ),
        ] {
            let ast = crate::frontend::parse::parse("arrays.n", source).unwrap();
            let resolved = names::resolve("arrays.n", source, &ast).unwrap();
            let mut checker = empty_checker(&resolved);
            let Statement::Let { value, .. } = &ast.functions[0].body.statements[0] else {
                panic!("array fixture must be a binding");
            };
            assert_eq!(
                checker.expr(value, None, None).unwrap(),
                Type::Array(Arc::new(Type::I64))
            );
            assert_eq!(
                checker
                    .graph
                    .has(&span_key(value.span()), OccurrenceFlag::Divergent),
                divergent,
                "{source}"
            );
        }
    }

    fn empty_checker<'a>(resolved: &'a Resolved) -> Checker<'a> {
        Checker {
            file: "quota.n",
            tokens: Vec::new(),
            resolved,
            f5: false,
            solver_steps: 0,
            solver_limit: 1_000_000,
            planning: false,
            occurrences: OccurrenceIndex::default(),
            function_ordinal: 0,
            current_passes: 0,
            current_planning_calls: 0,
            block_depth: 0,
            top_statement_start: 0,
            top_statement_end: 0,
            active_if_span: None,
            analysis_exhausted: false,
            quota_error: None,
            generator_item: None,
            loop_depth: 0,
            errors: Vec::new(),
            closures: HashMap::new(),
            saved_closures: HashMap::new(),
            generator_slots: HashMap::new(),
            hidden_for: Vec::new(),
            closure_local_sites: HashSet::new(),
            pending_depth: std::cell::RefCell::new(Vec::new()),
            defer_depth: false,
            depth_memo_ready: false,
            graph: InferenceGraph {
                equality_clock: 0,
                equality_records: Vec::new(),
                active_equality: None,
                node_journal: None,
                active_facts: HashMap::new(),
                visit_generations: Vec::new(),
                fact_clock: std::rc::Rc::new(std::cell::Cell::new(0)),
                generation_clock: std::rc::Rc::new(std::cell::Cell::new(0)),
                locals: HashMap::new(),
                type_nodes: HashMap::new(),
                array_unify_cache: HashMap::new(),
                expr_vars: HashMap::new(),
                expression_types: HashMap::new(),
                next_variable: 0,
                obligations: Vec::new(),
                occurrences: HashMap::new(),
                emitted_sites: HashSet::new(),
                eligible_shapes: HashSet::new(),
                shape_plan: HashMap::new(),
                concrete_plan: HashMap::new(),
                read_plan: HashMap::new(),
                assignment_type_plan: HashMap::new(),
                nested_assignment_sites: HashSet::new(),
                pre_assignment_snapshot: HashMap::new(),
                projected_bases: HashSet::new(),
                projection_items: HashSet::new(),
                assigned_bases: HashSet::new(),
                assignment_starts: HashMap::new(),
                first_assignment_rhs_starts: HashMap::new(),
                prefix_concrete: HashMap::new(),
                preseeded_children: HashSet::new(),
                result: Type::Unit,
                result_span: None,
                nominal_depth_memo: std::cell::RefCell::new(HashMap::new()),
            },
        }
    }

    #[test]
    fn solver_bind_dispatch_applies_each_fact_boundary() {
        let resolved = Resolved {
            signatures: HashMap::new(),
            constants: HashMap::new(),
            uses: HashMap::new(),
            calls: HashMap::new(),
        };
        fn bind_with_parent(
            checker: &mut Checker<'_>,
            id: u64,
            ty: Type,
            origin: Span,
            found: Span,
        ) {
            let parent = checker
                .graph
                .begin_visit(span_key(&found), FactRole::Solver(SolverBindSite::Ordinary));
            checker.graph.fact_mut(parent).tainted = true;
            checker.graph.fact_mut(parent).numeric_domain_owner = Some((1, 2));
            checker.bind(parent, id, ty, origin, found, false).unwrap();
            assert!(checker.graph.fact(parent).tainted);
            assert_eq!(
                checker.graph.fact(parent).numeric_domain_owner,
                Some((1, 2))
            );
            checker.graph.end_visit(parent);
            assert_eq!(checker.graph.active_count(), 0);
        }
        SOLVER_BIND_TRACE.with(|trace| trace.borrow_mut().clear());
        SOLVER_BIND_TRACE_ENABLED.with(|enabled| enabled.set(true));

        // A concrete Option is decomposed by require_union while an outer
        // bind is active. Its payload bind starts a separate visit; the bind
        // of the wanted payload after require_union inherits the outer visit.
        let mut checker = empty_checker(&resolved);
        let root = checker.new_variable(&site(1)).unwrap();
        let payload = checker.new_variable(&site(2)).unwrap();
        checker
            .bind_independent(
                root,
                Type::option(Arc::new(Type::I64)),
                site(3),
                site(4),
                false,
            )
            .unwrap();
        bind_with_parent(
            &mut checker,
            root,
            Type::option(Arc::new(Type::Deferred(payload))),
            site(5),
            site(6),
        );

        // Array concrete representatives are injected only in this internal
        // fixture: source Array values are decomposed into array_child.
        for concrete_first in [false, true] {
            let mut checker = empty_checker(&resolved);
            let first = checker.new_variable(&site(10)).unwrap();
            let second = checker.new_variable(&site(11)).unwrap();
            let structural = if concrete_first { second } else { first };
            let concrete = if concrete_first { first } else { second };
            checker.require_array(structural, &site(12)).unwrap();
            checker.graph.type_node_mut(&concrete).unwrap().concrete =
                Some((Type::Array(Arc::new(Type::I64)), site(13)));
            bind_with_parent(
                &mut checker,
                first,
                Type::Deferred(second),
                site(14),
                site(15),
            );
        }

        for concrete_first in [false, true] {
            let mut checker = empty_checker(&resolved);
            let first = checker.new_variable(&site(20)).unwrap();
            let second = checker.new_variable(&site(21)).unwrap();
            let structural = if concrete_first { second } else { first };
            let concrete = if concrete_first { first } else { second };
            checker
                .require_union(structural, UnionKind::Option, &site(22))
                .unwrap();
            checker
                .bind_independent(
                    concrete,
                    Type::option(Arc::new(Type::I64)),
                    site(23),
                    site(24),
                    false,
                )
                .unwrap();
            bind_with_parent(
                &mut checker,
                first,
                Type::Deferred(second),
                site(25),
                site(26),
            );
        }

        SOLVER_BIND_TRACE_ENABLED.with(|enabled| enabled.set(false));
        let traces = SOLVER_BIND_TRACE.with(|trace| trace.borrow().clone());
        for site in [
            SolverBindSite::RequireUnion,
            SolverBindSite::UnionArrayA,
            SolverBindSite::UnionArrayB,
            SolverBindSite::UnionPayloadA,
            SolverBindSite::UnionPayloadB,
        ] {
            let entries: Vec<_> = traces.iter().filter(|entry| entry.site == site).collect();
            assert_eq!(entries.len(), 1, "{site:?} must enter one child bind");
            let entry = entries[0];
            assert_ne!(entry.parent, entry.child, "{site:?}");
            assert_eq!(entry.parent_generation, entry.child_generation, "{site:?}");
            assert_eq!(entry.child_parent, Some(entry.parent), "{site:?}");
            assert!(entry.parent_tainted, "{site:?}");
            assert_eq!(entry.parent_owner, Some((1, 2)), "{site:?}");
            assert!(!entry.child_tainted, "{site:?}");
            assert_eq!(entry.child_owner, None, "{site:?}");
        }
        let inherited: Vec<_> = traces
            .iter()
            .filter(|entry| entry.site == SolverBindSite::InheritedPayload)
            .collect();
        assert_eq!(inherited.len(), 1);
        assert_ne!(inherited[0].parent, inherited[0].child);
        assert_eq!(inherited[0].child_parent, Some(inherited[0].parent));
        assert_eq!(
            inherited[0].parent_generation,
            inherited[0].child_generation
        );
        assert!(inherited[0].child_tainted);
        assert_eq!(inherited[0].child_owner, Some((1, 2)));
    }

    #[test]
    fn s3_solver_unify_depth_child() {
        if std::env::var_os("I07C_S3_DIRECT_UNIFY").is_none() {
            return;
        }
        let resolved = Resolved {
            signatures: HashMap::new(),
            constants: HashMap::new(),
            uses: HashMap::new(),
            calls: HashMap::new(),
        };
        let mut left = Type::I64;
        let mut right = Type::I64;
        for _ in 0..126 {
            left = Type::Array(Arc::new(left));
            right = Type::Array(Arc::new(right));
        }
        let outcome = std::thread::scope(|scope| {
            std::thread::Builder::new()
                .stack_size(204 * 1024)
                .spawn_scoped(scope, || {
                    let mut checker = empty_checker(&resolved);
                    checker.unify_independent(left, right, site(1), site(2), site(3), false)
                })
                .unwrap()
                .join()
                .unwrap()
        });
        assert!(outcome.is_ok(), "{outcome:?}");
    }

    #[test]
    fn drive_facts_close_after_successive_visits() {
        let source = "fn F(){let a=[1,true]+1;let b:Bool=1;} fn G(){let c=1+2;}";
        let ast = crate::frontend::parse::parse("visits.n", source).unwrap();
        let resolved = names::resolve("visits.n", source, &ast).unwrap();
        let mut checker = empty_checker(&resolved);

        FACT_STATEMENT_TRACE_ENABLED.with(|enabled| enabled.set(true));
        FACT_STATEMENT_TRACE.with(|trace| trace.borrow_mut().clear());

        let (_, first) = checker.drive(DriveStart::Block(&ast.functions[0].body));
        assert!(first);
        assert_eq!(checker.graph.active_count(), 0);
        FACT_STATEMENT_TRACE
            .with(|trace| assert_eq!(trace.borrow().as_slice(), &[(false, false), (true, false)]));
        FACT_STATEMENT_TRACE.with(|trace| trace.borrow_mut().clear());

        let (_, second) = checker.drive(DriveStart::Block(&ast.functions[1].body));
        assert!(!second);
        assert_eq!(checker.graph.active_count(), 0);
        FACT_STATEMENT_TRACE.with(|trace| assert_eq!(trace.borrow().as_slice(), &[(false, false)]));
        FACT_STATEMENT_TRACE_ENABLED.with(|enabled| enabled.set(false));

        let Statement::Let { value, .. } = &ast.functions[1].body.statements[0] else {
            unreachable!()
        };
        for _ in 0..2 {
            let (result, tainted) = checker.expr_with_visit_result(value, None, None);
            assert!(result.is_ok());
            assert!(!tainted);
            assert_eq!(checker.graph.active_count(), 0);
        }
    }

    #[test]
    fn boundary_context_owner_closes_constructor_before_comparison() {
        let source = "fn Main(){let z=Len([1]);}";
        let ast = parse::parse_new_profile("boundary.n", source).unwrap();
        let resolved = names::resolve_with_option("boundary.n", source, &ast, true).unwrap();
        let Statement::Let { value, .. } = &ast.functions[0].body.statements[0] else {
            panic!("expected call binding")
        };
        for (inherited_owner, inherited_taint) in [
            (Some((26, 41)), false),
            (None, false),
            (Some((26, 41)), true),
        ] {
            let mut checker = empty_checker(&resolved);
            let parent = checker
                .graph
                .begin_visit(span_key(value.span()), FactRole::Expression);
            checker.graph.fact_mut(parent).numeric_domain_owner = inherited_owner;
            checker.graph.fact_mut(parent).tainted = inherited_taint;
            CONTEXT_BOUNDARY_TRACE.with(|trace| trace.borrow_mut().clear());
            CONTEXT_BOUNDARY_TRACE_ENABLED.with(|enabled| enabled.set(true));
            let expected = Type::Bool;
            let context = CheckContext {
                expected: Some(&expected),
                expected_span: None,
                check_here: true,
                anchor: None,
                outer_compare: false,
                related_expected: false,
                deferred_element: false,
            };
            let result = checker.check_against_context_one(
                parent,
                value,
                context,
                Some(PrecomputedType::CallComplete(Ok(Type::I64))),
            );
            if inherited_taint {
                assert_eq!(result.unwrap(), Type::I64);
            } else {
                assert_eq!(result.unwrap_err().code, "NX-E0102");
            }
            CONTEXT_BOUNDARY_TRACE_ENABLED.with(|enabled| enabled.set(false));
            CONTEXT_BOUNDARY_TRACE.with(|trace| {
                let events = trace.borrow();
                assert_eq!(events.len(), if inherited_taint { 2 } else { 3 });
                assert_eq!(
                    events[0],
                    ("synthesis", "CallComplete", None, 2, false, true)
                );
                assert_eq!(
                    events[1],
                    (
                        "after_synthesis",
                        "CallComplete",
                        inherited_owner,
                        1,
                        true,
                        false
                    )
                );
                if !inherited_taint {
                    assert_eq!(
                        events[2],
                        (
                            "comparison",
                            "CallComplete",
                            inherited_owner,
                            1,
                            true,
                            false
                        )
                    );
                }
            });
            assert_eq!(
                checker.graph.fact(parent).numeric_domain_owner,
                inherited_owner
            );
            assert_eq!(checker.graph.fact(parent).tainted, inherited_taint);
            assert_eq!(checker.graph.active_count(), 1);
            checker.graph.end_visit(parent);
            assert_eq!(checker.graph.active_count(), 0);
        }
    }

    #[test]
    fn boundary_binary_context_closes_after_prepared_comparison() {
        for (source, stage) in [
            ("fn Main(){let z=1+2;}", "BinaryRight"),
            ("fn Main(){let z=true+1;}", "Whole"),
            ("fn Main(){let z=1+true;}", "BinaryRight"),
            ("fn Main(){var r=[];let z=r+1;r=1;}", "BinaryLeft"),
            ("fn Main(){var r=[];let z=1+r;r=1;}", "BinaryRight"),
        ] {
            let ast = parse::parse_new_profile("boundary-binary.n", source).unwrap();
            let resolved =
                names::resolve_with_option("boundary-binary.n", source, &ast, true).unwrap();
            let mut checker = empty_checker(&resolved);
            BOUNDARY_ROOT_OWNER.with(|owner| owner.set(Some((26, 41))));
            BOUNDARY_ROOT_TAINT.with(|taint| taint.set(true));
            CONTEXT_BOUNDARY_TRACE.with(|trace| trace.borrow_mut().clear());
            CONTEXT_FACT_DEPTH_TRACE.with(|trace| trace.borrow_mut().clear());
            CONTEXT_BOUNDARY_TRACE_ENABLED.with(|enabled| enabled.set(true));
            let _ = checker.drive(DriveStart::Block(&ast.functions[0].body));
            CONTEXT_BOUNDARY_TRACE_ENABLED.with(|enabled| enabled.set(false));
            BOUNDARY_ROOT_OWNER.with(|owner| owner.set(None));
            BOUNDARY_ROOT_TAINT.with(|taint| taint.set(false));
            CONTEXT_FACT_DEPTH_TRACE.with(|trace| {
                let observed: Vec<_> = trace
                    .borrow()
                    .iter()
                    .copied()
                    .filter(|event| event.0 == stage)
                    .collect();
                assert_eq!(observed, [(stage, 2)], "{source}: {:?}", trace.borrow());
            });
            CONTEXT_BOUNDARY_TRACE.with(|trace| {
                let observed: Vec<_> = trace
                    .borrow()
                    .iter()
                    .copied()
                    .filter(|event| event.0 == "after_synthesis" && event.1 == stage)
                    .collect();
                assert_eq!(
                    observed,
                    [("after_synthesis", stage, None, 3, true, true)],
                    "{source}: {:?}",
                    trace.borrow().as_slice()
                );
            });
        }
    }

    #[test]
    fn boundary_binary_left_quota_keeps_operator_until_context_close() {
        let source = "fn Main(){let z=1+2;}";
        let ast = parse::parse_new_profile("boundary-quota.n", source).unwrap();
        let resolved = names::resolve_with_option("boundary-quota.n", source, &ast, true).unwrap();
        let mut checker = empty_checker(&resolved);
        checker.solver_limit = 0;
        BOUNDARY_ROOT_OWNER.with(|owner| owner.set(Some((26, 41))));
        BOUNDARY_ROOT_TAINT.with(|taint| taint.set(true));
        CONTEXT_BOUNDARY_TRACE.with(|trace| trace.borrow_mut().clear());
        CONTEXT_FACT_DEPTH_TRACE.with(|trace| trace.borrow_mut().clear());
        CONTEXT_BOUNDARY_TRACE_ENABLED.with(|enabled| enabled.set(true));
        let (result, _) = checker.drive(DriveStart::Block(&ast.functions[0].body));
        assert!(result.is_ok());
        CONTEXT_BOUNDARY_TRACE_ENABLED.with(|enabled| enabled.set(false));
        BOUNDARY_ROOT_OWNER.with(|owner| owner.set(None));
        BOUNDARY_ROOT_TAINT.with(|taint| taint.set(false));
        CONTEXT_BOUNDARY_TRACE.with(|trace| {
            assert!(
                trace
                    .borrow()
                    .contains(&("after_synthesis", "BinaryLeft", None, 3, true, true,)),
                "{:?}",
                trace.borrow()
            );
        });
        CONTEXT_FACT_DEPTH_TRACE.with(|trace| {
            assert!(
                trace.borrow().contains(&("BinaryLeft", 2)),
                "{:?}",
                trace.borrow()
            );
        });
    }

    #[test]
    fn boundary_frame_owner_survives_completed_calls() {
        for (label, source) in [
            (
                "Call",
                "fn F(x:view I64)->I64{return x;}fn Main(){let z=F(1);}",
            ),
            ("UnionCall", "fn Main(){let z=Some(1);}"),
            ("TryAdd", "fn Main(){let z=TryAdd(1,1);}"),
            ("IntrinsicOne", "fn Main(){let z=Len([1]);}"),
            ("CallTarget", "fn Main(){let z=(fn ()->I64{return 1;})();}"),
            ("AssertEq", "fn Main(){AssertEq(1,1);}"),
        ] {
            let ast = parse::parse_new_profile("boundary-call.n", source).unwrap();
            let resolved =
                names::resolve_with_option("boundary-call.n", source, &ast, true).unwrap();
            let mut checker = empty_checker(&resolved);
            BOUNDARY_ROOT_OWNER.with(|owner| owner.set(Some((26, 41))));
            BOUNDARY_ROOT_TAINT.with(|taint| taint.set(true));
            FRAME_EXIT_TRACE.with(|trace| trace.borrow_mut().clear());
            CONTEXT_BOUNDARY_TRACE_ENABLED.with(|enabled| enabled.set(true));
            let _ = checker.drive(DriveStart::Block(&ast.functions.last().unwrap().body));
            CONTEXT_BOUNDARY_TRACE_ENABLED.with(|enabled| enabled.set(false));
            BOUNDARY_ROOT_OWNER.with(|owner| owner.set(None));
            BOUNDARY_ROOT_TAINT.with(|taint| taint.set(false));
            FRAME_EXIT_TRACE.with(|trace| {
                assert!(
                    trace.borrow().iter().any(|event| {
                        *event == (label, Some((26, 41)), false, 1, FactRole::Statement)
                    }),
                    "{label}: {:?}",
                    trace.borrow()
                );
            });
        }
    }

    #[test]
    fn boundary_closure_call_restores_parent_fact() {
        let source = "fn Main(){let z=(fn [](x:view I64){return x;})(1);}";
        let ast = parse::parse_new_profile("boundary-closure.n", source).unwrap();
        let resolved =
            names::resolve_with_option("boundary-closure.n", source, &ast, true).unwrap();
        BOUNDARY_ROOT_OWNER.with(|owner| owner.set(Some((26, 41))));
        BOUNDARY_ROOT_TAINT.with(|taint| taint.set(true));
        FRAME_EXIT_TRACE.with(|trace| trace.borrow_mut().clear());
        CONTEXT_BOUNDARY_TRACE_ENABLED.with(|enabled| enabled.set(true));
        let report = check_with_partial("boundary-closure.n", source, &ast, &resolved);
        CONTEXT_BOUNDARY_TRACE_ENABLED.with(|enabled| enabled.set(false));
        BOUNDARY_ROOT_OWNER.with(|owner| owner.set(None));
        BOUNDARY_ROOT_TAINT.with(|taint| taint.set(false));
        assert!(report.errors.is_empty(), "{:?}", report.errors);
        FRAME_EXIT_TRACE.with(|trace| {
            let observed: Vec<_> = trace
                .borrow()
                .iter()
                .filter(|event| event.0 == "ClosureCall")
                .copied()
                .collect();
            assert!(!observed.is_empty());
            assert!(
                observed.iter().all(|event| {
                    *event == ("ClosureCall", Some((26, 41)), false, 1, FactRole::Statement)
                }),
                "{observed:?}"
            );
        });
    }

    #[test]
    fn boundary_taint_edges_clear_before_local_values() {
        for (label, source, marker, offset, length, role) in [
            (
                "statement",
                "fn Main(){let x=1;let y=2;}",
                "let y=2;",
                0,
                8,
                FactRole::Statement,
            ),
            (
                "assignment",
                "fn Main(){var x=1;x=2;}",
                "x=2;",
                2,
                1,
                FactRole::Expression,
            ),
            (
                "if",
                "fn Main(){let z=(if true then 1 else 2);}",
                "then 1",
                5,
                1,
                FactRole::Expression,
            ),
            (
                "match",
                "fn Main(){let z=match true{true=>{value:1;},false=>{value:2;}};}",
                "value:2",
                6,
                1,
                FactRole::Expression,
            ),
            (
                "closure",
                "fn Main(){let f=fn []()->I64{let x=1;return 2;};}",
                "return 2",
                7,
                1,
                FactRole::Statement,
            ),
        ] {
            let ast = parse::parse_new_profile("boundary-taint.n", source).unwrap();
            let resolved =
                names::resolve_with_option("boundary-taint.n", source, &ast, true).unwrap();
            let start = (source.find(marker).unwrap() + offset) as u64;
            let site = (start, start + length);
            BOUNDARY_ROOT_OWNER.with(|owner| owner.set(Some((26, 41))));
            BOUNDARY_ROOT_TAINT.with(|taint| taint.set(true));
            BOUNDARY_FORCE_TAINT_EDGE.with(|forced| forced.set(Some((site, role))));
            BOUNDARY_EDGE_TRACE.with(|trace| trace.borrow_mut().clear());
            CONTEXT_BOUNDARY_TRACE_ENABLED.with(|enabled| enabled.set(true));
            let _report = check_with_partial("boundary-taint.n", source, &ast, &resolved);
            CONTEXT_BOUNDARY_TRACE_ENABLED.with(|enabled| enabled.set(false));
            BOUNDARY_FORCE_TAINT_EDGE.with(|forced| forced.set(None));
            BOUNDARY_ROOT_OWNER.with(|owner| owner.set(None));
            BOUNDARY_ROOT_TAINT.with(|taint| taint.set(false));
            BOUNDARY_EDGE_TRACE.with(|trace| {
                let selected: Vec<_> = trace
                    .borrow()
                    .iter()
                    .copied()
                    .filter(|edge| edge.site == site && edge.role == role)
                    .collect();
                assert!(!selected.is_empty(), "{label}: {:?}", trace.borrow());
                assert!(
                    selected.iter().all(|edge| {
                        edge.parent_tainted
                            && !edge.child_tainted
                            && edge.parent_owner == Some((26, 41))
                            && edge.child_owner == Some((26, 41))
                    }),
                    "{label}: {selected:?}"
                );
            });
        }
    }

    #[test]
    fn boundary_array_and_index_restore_owner_before_consumers() {
        let source = "fn Main(){let x=[1];let z=x[0];}";
        let ast = parse::parse_new_profile("boundary-index.n", source).unwrap();
        let resolved = names::resolve_with_option("boundary-index.n", source, &ast, true).unwrap();
        let mut checker = empty_checker(&resolved);
        BOUNDARY_ROOT_OWNER.with(|owner| owner.set(Some((26, 41))));
        BOUNDARY_ROOT_TAINT.with(|taint| taint.set(true));
        BOUNDARY_EDGE_TRACE.with(|trace| trace.borrow_mut().clear());
        CONTEXT_BOUNDARY_TRACE.with(|trace| trace.borrow_mut().clear());
        FRAME_EXIT_TRACE.with(|trace| trace.borrow_mut().clear());
        CONTEXT_BOUNDARY_TRACE_ENABLED.with(|enabled| enabled.set(true));
        let _ = checker.drive(DriveStart::Block(&ast.functions[0].body));
        CONTEXT_BOUNDARY_TRACE_ENABLED.with(|enabled| enabled.set(false));
        BOUNDARY_ROOT_OWNER.with(|owner| owner.set(None));
        BOUNDARY_ROOT_TAINT.with(|taint| taint.set(false));
        BOUNDARY_EDGE_TRACE.with(|trace| {
            let edges = trace.borrow();
            assert!(edges.iter().any(|edge| {
                edge.site == (17, 18)
                    && edge.role == FactRole::Expression
                    && edge.parent_owner == Some((26, 41))
                    && edge.child_owner.is_none()
            }));
            assert!(edges.iter().any(|edge| {
                edge.site == (26, 27)
                    && edge.role == FactRole::Expression
                    && edge.parent_owner == Some((26, 41))
                    && edge.child_owner.is_none()
            }));
            assert!(edges.iter().any(|edge| {
                edge.site == (28, 29)
                    && edge.role == FactRole::Selector
                    && edge.parent_owner == Some((26, 41))
                    && edge.child_owner == Some((26, 41))
            }));
        });
        CONTEXT_BOUNDARY_TRACE.with(|trace| {
            let events = trace.borrow();
            assert!(events.contains(&(
                "after_synthesis",
                "ArrayComplete",
                Some((26, 41)),
                2,
                true,
                false,
            )));
            assert!(events.contains(&(
                "after_synthesis",
                "IndexTarget",
                Some((26, 41)),
                2,
                true,
                false,
            )));
        });
        FRAME_EXIT_TRACE.with(|trace| {
            assert!(trace.borrow().contains(&(
                "Array",
                Some((26, 41)),
                false,
                1,
                FactRole::Statement,
            )));
        });
    }

    #[test]
    fn boundary_binary_deferred_rhs_keeps_owner_until_result() {
        let source = "fn Main(){var r=[];let z=(r<1);r=1;}";
        let ast = parse::parse_new_profile("boundary-deferred.n", source).unwrap();
        let resolved =
            names::resolve_with_option("boundary-deferred.n", source, &ast, true).unwrap();
        BOUNDARY_ROOT_OWNER.with(|owner| owner.set(Some((26, 41))));
        BOUNDARY_ROOT_TAINT.with(|taint| taint.set(true));
        FRAME_EXIT_TRACE.with(|trace| trace.borrow_mut().clear());
        CONTEXT_BOUNDARY_TRACE.with(|trace| trace.borrow_mut().clear());
        CONTEXT_BOUNDARY_TRACE_ENABLED.with(|enabled| enabled.set(true));
        let _ = check_with_partial("boundary-deferred.n", source, &ast, &resolved);
        CONTEXT_BOUNDARY_TRACE_ENABLED.with(|enabled| enabled.set(false));
        BOUNDARY_ROOT_OWNER.with(|owner| owner.set(None));
        BOUNDARY_ROOT_TAINT.with(|taint| taint.set(false));
        FRAME_EXIT_TRACE.with(|trace| {
            assert!(trace.borrow().contains(&(
                "BinaryDeferredRight",
                Some((26, 41)),
                false,
                1,
                FactRole::Statement,
            )));
        });
        CONTEXT_BOUNDARY_TRACE.with(|trace| {
            assert!(trace.borrow().contains(&(
                "after_synthesis",
                "BinaryDeferredRight",
                None,
                3,
                true,
                true,
            )));
        });
    }

    #[test]
    fn fact_generations_preserve_outer_values_and_reject_closed_ids() {
        let resolved = Resolved {
            signatures: HashMap::new(),
            constants: HashMap::new(),
            uses: HashMap::new(),
            calls: HashMap::new(),
        };
        let mut checker = empty_checker(&resolved);
        let outer = checker.graph.begin_visit((1, 2), FactRole::Expression);
        checker.graph.fact_mut(outer).tainted = true;
        checker.graph.fact_mut(outer).numeric_domain_owner = Some((3, 4));
        let inner = checker.graph.begin_visit((1, 2), FactRole::Selector);
        assert_eq!(checker.graph.active_count(), 2);
        assert!(
            std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| checker.graph.fact(outer)))
                .is_err()
        );
        checker.graph.fact_mut(inner).tainted = true;
        assert!(checker.graph.end_visit(inner));
        assert_eq!(checker.graph.active_count(), 1);
        assert!(checker.graph.fact(outer).tainted);
        assert_eq!(checker.graph.fact(outer).numeric_domain_owner, Some((3, 4)));

        let snapshot = checker.clone();
        let aborted = checker.graph.child_fact(
            outer,
            (5, 6),
            FactRole::Solver(SolverBindSite::Ordinary),
            FactInheritance::Empty,
            FactInheritance::Empty,
        );
        checker = snapshot;
        assert!(
            std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| checker.graph.fact(aborted)))
                .is_err()
        );
        let replacement = checker.graph.child_fact(
            outer,
            (5, 6),
            FactRole::Solver(SolverBindSite::Ordinary),
            FactInheritance::Empty,
            FactInheritance::Empty,
        );
        assert_ne!(aborted, replacement);
        checker.graph.retire_fact(replacement);
        assert!(checker.graph.end_visit(outer));
        assert_eq!(checker.graph.active_count(), 0);
        assert!(
            std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| checker.graph.fact(outer)))
                .is_err()
        );
    }

    #[test]
    fn deferred_selector_reentry_keeps_outer_facts_for_array_and_scalar_bindings() {
        FACT_REENTRY_ENABLED.with(|enabled| enabled.set(true));
        FACT_REENTRY_COUNT.with(|count| count.set(0));
        FACT_REENTRY_STACK.with(|stack| stack.borrow_mut().clear());
        for (source, expected) in [
            (
                "fn Main(){var q=(if true then {return;} else {return;});let a=q[(if true then 1 else true)];q=[3];}",
                vec![("NX-E0102", Some("type_mismatch"), 85, 89)],
            ),
            (
                "fn Main(){var q=(if true then {return;} else {return;});let a=q[(if true then 1 else true)];q=3;}",
                vec![
                    ("NX-E0102", Some("not_indexable"), 62, 63),
                    ("NX-E1303", None, 85, 89),
                ],
            ),
        ] {
            let ast = parse::parse_new_profile("reentry.n", source).unwrap();
            let resolved = names::resolve_with_option("reentry.n", source, &ast, true).unwrap();
            let report = check_with_partial("reentry.n", source, &ast, &resolved);
            let actual: Vec<_> = report
                .errors
                .iter()
                .map(|error| {
                    let primary = error.primary_span.as_ref().unwrap();
                    (
                        error.code.as_str(),
                        error.subcause.as_deref(),
                        primary.start,
                        primary.end,
                    )
                })
                .collect();
            assert_eq!(actual, expected);
        }
        FACT_REENTRY_ENABLED.with(|enabled| enabled.set(false));
        assert!(FACT_REENTRY_COUNT.with(|count| count.get()) >= 2);
        FACT_REENTRY_STACK.with(|stack| assert!(stack.borrow().is_empty()));
    }

    #[test]
    fn nested_visit_quota_closes_inner_facts_without_changing_outer_cause() {
        let source = "fn Main(){let x=1+2;}";
        let ast = parse::parse_new_profile("quota-reentry.n", source).unwrap();
        let resolved = names::resolve_with_option("quota-reentry.n", source, &ast, true).unwrap();
        let Statement::Let { value, .. } = &ast.functions[0].body.statements[0] else {
            unreachable!()
        };
        let mut checker = empty_checker(&resolved);
        let outer = checker
            .graph
            .begin_visit((0, source.len() as u64), FactRole::Statement);
        checker.graph.fact_mut(outer).tainted = true;
        checker.graph.fact_mut(outer).numeric_domain_owner = Some((5, 6));
        checker.solver_limit = checker.solver_steps;
        let (result, _) = checker.expr_with_visit_result(value, None, None);
        let error = result.unwrap_err();
        assert_eq!(error.code, "NX-E0003");
        assert_eq!(checker.graph.active_count(), 1);
        assert!(checker.graph.fact(outer).tainted);
        assert_eq!(checker.graph.fact(outer).numeric_domain_owner, Some((5, 6)));
        assert!(checker.quota_error.is_some());
        checker.graph.end_visit(outer);
        assert_eq!(checker.graph.active_count(), 0);
    }

    #[test]
    fn malformed_generator_state_cycle_is_bounded() {
        let resolved = Resolved {
            signatures: HashMap::new(),
            constants: HashMap::new(),
            uses: HashMap::new(),
            calls: HashMap::new(),
        };
        let mut checker = empty_checker(&resolved);
        let recursive = Type::Generator {
            site: 1,
            item: Arc::new(Type::I64),
        };
        checker.generator_slots.insert(1, vec![recursive.clone()]);
        assert_eq!(checker.compound_depth(&recursive), (0, 2));
    }

    #[test]
    fn inferred_array_path_keeps_array_depth_priority_over_value_depth() {
        let resolved = Resolved {
            signatures: HashMap::new(),
            constants: HashMap::new(),
            uses: HashMap::new(),
            calls: HashMap::new(),
        };
        let mut checker = empty_checker(&resolved);
        let mut roots = Vec::new();
        for _ in 0..=129 {
            roots.push(checker.new_variable(&site(0)).unwrap());
        }
        for pair in roots.windows(2) {
            checker.graph.type_node_mut(&pair[0]).unwrap().array_child = Some(pair[1]);
        }
        let error = checker
            .check_depth(&Type::Deferred(roots[0]), &site(7))
            .unwrap_err();
        assert_eq!(error.code, "NX-E0003");
        assert_eq!(error.stage, Stage::Types);
        assert_eq!(error.operation, Operation::AnalysisLimit);
        assert_eq!(error.primary_span, Some(site(7)));
        assert_eq!(
            error.data.get("budget_kind"),
            Some(&DataValue::String("array_type_depth".into()))
        );
        assert_eq!(error.data.get("limit"), Some(&DataValue::UInt(128)));
        assert_eq!(error.data.get("observed"), Some(&DataValue::UInt(129)));
    }

    #[test]
    fn contextual_array_charges_each_leaf_once_at_the_reduced_boundary() {
        // F0-0002 §§4.7, 11.1: two scalar element comparisons and one
        // constructor comparison are three logical solver steps. Rejoining
        // already checked elements would spend a fourth step.
        let source = "fn Main(){let a=[1,2];}";
        let ast = crate::frontend::parse::parse("quota.n", source).unwrap();
        let resolved = names::resolve("quota.n", source, &ast).unwrap();
        let Statement::Let { value, .. } = &ast.functions[0].body.statements[0] else {
            panic!("expected binding fixture");
        };
        let expected = Type::Array(Arc::new(Type::I64));
        let mut granted = empty_checker(&resolved);
        granted.solver_limit = 3;
        assert_eq!(
            granted.expr(value, Some(&expected), None).unwrap(),
            expected
        );
        assert_eq!(granted.solver_steps, 3);

        let mut denied = empty_checker(&resolved);
        denied.solver_limit = 2;
        let error = denied.expr(value, Some(&expected), None).unwrap_err();
        assert_eq!(error.code, "NX-E0003");
        assert_eq!(error.data.get("limit"), Some(&DataValue::UInt(2)));
        assert_eq!(error.data.get("observed"), Some(&DataValue::UInt(3)));
    }

    #[test]
    fn contextual_group_and_if_charge_only_their_value_operands() {
        // F0-0002 §11.1 counts logical comparisons. A transparent group
        // delegates its one pair; an if delegates its condition and values.
        let source = "fn Main(){let a=(1);let b=(if true then 1 else 2);}";
        let ast = crate::frontend::parse::parse("quota.n", source).unwrap();
        let resolved = names::resolve("quota.n", source, &ast).unwrap();
        let mut values = ast.functions[0].body.statements.iter().map(|statement| {
            let Statement::Let { value, .. } = statement else {
                panic!("expected binding fixture")
            };
            value
        });
        let group = values.next().unwrap();
        let branch = values.next().unwrap();
        let mut checker = empty_checker(&resolved);
        checker.solver_limit = 1;
        assert_eq!(
            checker.expr(group, Some(&Type::I64), None).unwrap(),
            Type::I64
        );
        assert_eq!(checker.solver_steps, 1);
        let mut checker = empty_checker(&resolved);
        checker.solver_limit = 3;
        assert_eq!(
            checker.expr(branch, Some(&Type::I64), None).unwrap(),
            Type::I64
        );
        assert_eq!(checker.solver_steps, 3);
    }

    #[test]
    fn unresolved_expected_type_waits_and_unchecked_group_preserves_its_leaf() {
        // F0-0002 §4.1.1 permits a pending constructor to be fixed later.
        let resolved = Resolved {
            signatures: HashMap::new(),
            constants: HashMap::new(),
            uses: HashMap::new(),
            calls: HashMap::new(),
        };
        let mut checker = empty_checker(&resolved);
        let pending = checker.new_variable(&site(0)).unwrap();
        let expected = Type::Array(Arc::new(Type::Deferred(pending)));
        let bool_value = Expr::Bool(true, site(1));
        assert_eq!(
            checker.expr(&bool_value, Some(&expected), None).unwrap(),
            Type::Bool
        );

        let group = Expr::Group(Box::new(Expr::Bool(true, site(2))), site(3));
        assert_eq!(
            checker
                .check_against(&group, Some(&Type::I64), None, false)
                .unwrap(),
            Type::Bool
        );
    }

    #[test]
    fn non_result_try_is_rejected_at_the_expression_boundary() {
        let resolved = Resolved {
            signatures: HashMap::new(),
            constants: HashMap::new(),
            uses: HashMap::new(),
            calls: HashMap::new(),
        };
        let mut checker = empty_checker(&resolved);
        let expr = Expr::Unary {
            mode: Mode::Read,
            value: Box::new(Expr::Bool(true, site(1))),
            span: site(0),
        };
        let error = checker.expr(&expr, None, None).unwrap_err();
        assert_eq!(error.code, "NX-E0102");
        assert_eq!(error.primary_span, Some(site(0)));
    }

    #[test]
    fn union_reports_resolved_array_constructor_in_both_root_orders() {
        let resolved = Resolved {
            signatures: HashMap::new(),
            constants: HashMap::new(),
            uses: HashMap::new(),
            calls: HashMap::new(),
        };
        for array_first in [true, false] {
            let mut checker = empty_checker(&resolved);
            let first = checker.new_variable(&site(0)).unwrap();
            let second = checker.new_variable(&site(1)).unwrap();
            let (array, scalar) = if array_first {
                (first, second)
            } else {
                (second, first)
            };
            let child = checker.require_array(array, &site(2)).unwrap();
            checker
                .graph
                .type_node_mut(&array)
                .unwrap()
                .array_constructed = true;
            checker
                .bind_independent(child, Type::I64, site(3), site(3), false)
                .unwrap();
            checker
                .bind_independent(scalar, Type::I64, site(4), site(4), false)
                .unwrap();
            let error = checker.union(first, second, site(9)).unwrap_err();
            assert_eq!(
                (error.code.as_str(), error.subcause.as_deref()),
                ("NX-E0102", Some("type_mismatch"))
            );
            assert_eq!(
                error.data.get("expected_type"),
                Some(&DataValue::String("Array<I64>".into()))
            );
            assert_eq!(
                error.data.get("found_type"),
                Some(&DataValue::String("I64".into()))
            );
            assert_eq!(
                error.primary_span.as_ref().map(|s| (s.start, s.end)),
                Some((9, 10))
            );
            assert_eq!(
                error
                    .related_spans
                    .iter()
                    .map(|s| (s.role, s.span.start, s.span.end))
                    .collect::<Vec<_>>(),
                [
                    (crate::diagnostic::RelatedRole::ExpectedType, 3, 4),
                    (crate::diagnostic::RelatedRole::FoundType, 9, 10)
                ]
            );
            error.validate("solver.n", "xxxxxxxxxx").unwrap();
        }
    }

    #[test]
    fn union_selects_earlier_origin_by_start_even_when_spans_overlap() {
        // The origin comparison is based on starts. An outer expression and
        // one of its nested values can overlap without sharing a type.
        let resolved = Resolved {
            signatures: HashMap::new(),
            constants: HashMap::new(),
            uses: HashMap::new(),
            calls: HashMap::new(),
        };
        let mut checker = empty_checker(&resolved);
        let later = checker.new_variable(&site(0)).unwrap();
        let earlier = checker.new_variable(&site(1)).unwrap();
        let later_origin = Span {
            file: "quota.n".into(),
            start: 10,
            end: 20,
        };
        let earlier_origin = Span {
            file: "quota.n".into(),
            start: 5,
            end: 15,
        };
        checker
            .bind_independent(later, Type::I64, later_origin, site(30), false)
            .unwrap();
        checker
            .bind_independent(earlier, Type::Bool, earlier_origin.clone(), site(31), false)
            .unwrap();
        let error = checker.union(later, earlier, site(32)).unwrap_err();
        assert_eq!(
            error.data.get("expected_type"),
            Some(&DataValue::String("Bool".into()))
        );
        assert_eq!(
            error.data.get("found_type"),
            Some(&DataValue::String("I64".into()))
        );
        assert!(error.related_spans.iter().any(|related| {
            related.role == RelatedRole::ExpectedType && related.span == earlier_origin
        }));
    }

    #[test]
    fn concrete_array_cache_keys_both_operands_and_charges_a_cached_constraint() {
        let resolved = Resolved {
            signatures: HashMap::new(),
            constants: HashMap::new(),
            uses: HashMap::new(),
            calls: HashMap::new(),
        };
        let i64_array = Type::Array(Arc::new(Type::I64));
        let bool_array = Type::Array(Arc::new(Type::Bool));
        for change_left in [true, false] {
            let mut checker = empty_checker(&resolved);
            checker
                .unify_independent(
                    i64_array.clone(),
                    i64_array.clone(),
                    site(0),
                    site(1),
                    site(2),
                    false,
                )
                .unwrap();
            let (left, right, expected, found) = if change_left {
                (
                    bool_array.clone(),
                    i64_array.clone(),
                    "Array<Bool>",
                    "Array<I64>",
                )
            } else {
                (
                    i64_array.clone(),
                    bool_array.clone(),
                    "Array<I64>",
                    "Array<Bool>",
                )
            };
            let error = checker
                .unify_independent(left, right, site(3), site(4), site(5), false)
                .unwrap_err();
            assert_eq!(
                (error.code.as_str(), error.subcause.as_deref()),
                ("NX-E0102", Some("type_mismatch"))
            );
            assert_eq!(
                error.data.get("expected_type"),
                Some(&DataValue::String(expected.into()))
            );
            assert_eq!(
                error.data.get("found_type"),
                Some(&DataValue::String(found.into()))
            );
        }
        let mut checker = empty_checker(&resolved);
        let nested = |base| {
            let mut ty = base;
            for _ in 0..32 {
                ty = Type::Array(Arc::new(ty));
            }
            ty
        };
        let left = nested(Type::I64);
        let right = nested(Type::I64);
        checker
            .unify_independent(
                left.clone(),
                right.clone(),
                site(0),
                site(1),
                site(2),
                false,
            )
            .unwrap();
        assert_eq!(checker.solver_steps, 33);
        checker
            .unify_independent(left, right, site(3), site(4), site(5), false)
            .unwrap();
        assert_eq!(checker.solver_steps, 66);
    }

    #[test]
    fn unresolved_constructor_pair_is_rechecked_after_its_child_resolves() {
        // A mixed array pair can defer a scalar/array shape conflict. It may
        // be cached only when both operands were fully concrete at entry.
        let resolved = Resolved {
            signatures: HashMap::new(),
            constants: HashMap::new(),
            uses: HashMap::new(),
            calls: HashMap::new(),
        };
        let mut checker = empty_checker(&resolved);
        let constructor = checker.new_variable(&site(1)).unwrap();
        let child = checker.require_array(constructor, &site(2)).unwrap();
        checker
            .graph
            .type_nodes
            .get_mut(&constructor)
            .unwrap()
            .array_constructed = true;
        let left = Type::Array(Arc::new(Type::Deferred(constructor)));
        let right = Type::Array(Arc::new(Type::I64));
        let first = checker.unify_independent(
            left.clone(),
            right.clone(),
            site(3),
            site(4),
            site(5),
            false,
        );
        assert!(first.is_ok(), "{first:?}");
        checker
            .bind_independent(child, Type::I64, site(6), site(6), false)
            .unwrap();
        let second = checker.unify_independent(left, right, site(3), site(4), site(7), false);
        assert!(second.is_ok(), "{second:?}");
        assert_eq!(
            checker
                .pending_joins()
                .map(|join| join.found.start)
                .collect::<Vec<_>>(),
            vec![5, 7],
            "each source comparison keeps its own conflict"
        );
    }

    #[test]
    fn cached_arrays_charge_every_logical_pair_at_the_reduced_limit() {
        // F0-0002 §11.8, point 1: sharing or caching saves CPU but never changes
        // the 33 logical pairs for Array^32<I64>.
        let resolved = Resolved {
            signatures: HashMap::new(),
            constants: HashMap::new(),
            uses: HashMap::new(),
            calls: HashMap::new(),
        };
        let mut checker = empty_checker(&resolved);
        let mut left = Type::I64;
        let mut right = Type::I64;
        for _ in 0..32 {
            left = Type::Array(Arc::new(left));
            right = Type::Array(Arc::new(right));
        }
        checker
            .unify_independent(
                left.clone(),
                right.clone(),
                site(0),
                site(0),
                site(0),
                false,
            )
            .unwrap();
        assert_eq!(checker.solver_steps, 33);
        checker.solver_limit = 65;
        let error = checker
            .unify_independent(left, right, site(1), site(1), site(1), false)
            .unwrap_err();
        assert_eq!(checker.solver_steps, 65);
        assert_eq!(error.code, "NX-E0003");
        assert_eq!(error.data.get("limit"), Some(&DataValue::UInt(65)));
        assert_eq!(error.data.get("observed"), Some(&DataValue::UInt(66)));
        assert_eq!(
            error.data.get("budget_kind"),
            Some(&DataValue::String("type_solver_steps".into()))
        );
    }

    #[test]
    fn existing_array_domain_and_occurs_walk_charge_at_the_reduced_limit() {
        // F0-0002 §11.1 counts domain restrictions and logical constructor
        // pairs in a union; neither may bypass the exhausted global budget.
        let resolved = Resolved {
            signatures: HashMap::new(),
            constants: HashMap::new(),
            uses: HashMap::new(),
            calls: HashMap::new(),
        };
        let mut checker = empty_checker(&resolved);
        let root = checker.new_variable(&site(0)).unwrap();
        let child = checker.require_array(root, &site(1)).unwrap();
        checker.solver_limit = checker.solver_steps;
        let domain = checker.require_array(root, &site(2)).unwrap_err();
        assert_eq!(domain.code, "NX-E0003");
        assert_eq!(
            domain.data.get("observed"),
            Some(&DataValue::UInt(checker.solver_limit + 1))
        );
        checker.analysis_exhausted = false;
        let reach = checker.type_reaches(root, child, &site(3)).unwrap_err();
        assert_eq!(reach.code, "NX-E0003");
        assert_eq!(
            reach.data.get("observed"),
            Some(&DataValue::UInt(checker.solver_limit + 1))
        );
    }

    #[test]
    fn adr_gap_union_charge_at_the_exact_reduced_boundary() {
        // The current solver counts a union as one processed restriction
        // (adr_gap_union_charge); §11.1 does not specify union cost.
        let resolved = Resolved {
            signatures: HashMap::new(),
            constants: HashMap::new(),
            uses: HashMap::new(),
            calls: HashMap::new(),
        };
        let mut checker = empty_checker(&resolved);
        let left = checker.new_variable(&site(0)).unwrap();
        let right = checker.new_variable(&site(1)).unwrap();
        let limit = checker.solver_steps;
        checker.solver_limit = limit;
        let error = checker.union(left, right, site(2)).unwrap_err();
        assert_eq!(error.code, "NX-E0003");
        assert_eq!(error.data.get("limit"), Some(&DataValue::UInt(limit)));
        assert_eq!(
            error.data.get("observed"),
            Some(&DataValue::UInt(limit + 1))
        );
        let mut granted = empty_checker(&resolved);
        let left = granted.new_variable(&site(0)).unwrap();
        let right = granted.new_variable(&site(1)).unwrap();
        let before = granted.solver_steps;
        granted.solver_limit = before + 1;
        granted.union(left, right, site(2)).unwrap();
        assert_eq!(granted.solver_steps, before + 1);
    }

    #[test]
    fn union_preserves_bottom_marker_from_either_representative() {
        // F0-0002 §§4.1.1, 13.17: an unresolved Never alias carries its
        // constructor obligation through union regardless of root rank.
        let resolved = Resolved {
            signatures: HashMap::new(),
            constants: HashMap::new(),
            uses: HashMap::new(),
            calls: HashMap::new(),
        };
        for marked_first in [false, true] {
            let mut checker = empty_checker(&resolved);
            let first = checker.new_variable(&site(0)).unwrap();
            let second = checker.new_variable(&site(1)).unwrap();
            let marked = if marked_first { first } else { second };
            checker.graph.type_node_mut(&marked).unwrap().bottom_only = true;
            checker.union(first, second, site(2)).unwrap();
            let root = checker.root(first);
            assert!(checker.graph.type_nodes[&root].bottom_only);
        }
    }

    #[test]
    fn adr_gap_shape_walk_charge_per_constructor_link() {
        // The current shape walk charges 128 traversed links as 128 units
        // (adr_gap_shape_walk_charge); §11.1 does not fix that count.
        // One fewer granted unit rejects the final link with limit+1.
        let resolved = Resolved {
            signatures: HashMap::new(),
            constants: HashMap::new(),
            uses: HashMap::new(),
            calls: HashMap::new(),
        };
        let mut checker = empty_checker(&resolved);
        let first = checker.new_variable(&site(0)).unwrap();
        let mut last = first;
        for offset in 1..=128 {
            last = checker.require_array(last, &site(offset)).unwrap();
        }
        let before = checker.solver_steps;
        checker.solver_limit = before + 128;
        assert!(checker.type_reaches(first, last, &site(200)).unwrap());
        assert_eq!(checker.solver_steps, before + 128);
        checker.solver_limit += 127;
        let denied = checker.type_reaches(first, last, &site(201)).unwrap_err();
        assert_eq!(checker.solver_steps, checker.solver_limit);
        assert_eq!(denied.code, "NX-E0003");
        assert_eq!(
            denied.data.get("budget_kind"),
            Some(&DataValue::String("type_solver_steps".into()))
        );
        assert_eq!(
            denied.data.get("observed"),
            Some(&DataValue::UInt(checker.solver_limit + 1))
        );
    }

    #[test]
    fn solver_charges_the_final_granted_unit_and_reports_one_rejected_unit() {
        let resolved = Resolved {
            signatures: HashMap::new(),
            constants: HashMap::new(),
            uses: HashMap::new(),
            calls: HashMap::new(),
        };
        let mut checker = empty_checker(&resolved);
        checker.solver_steps = 999_999;
        let span = Span {
            file: "quota.n".into(),
            start: 0,
            end: 1,
        };
        checker.charge_solver(&span).unwrap();
        assert_eq!(checker.solver_steps, 1_000_000);
        let d = checker.charge_solver(&span).unwrap_err();
        assert_eq!(checker.solver_steps, 1_000_000);
        assert!(checker.analysis_exhausted);
        assert_eq!(
            (d.code.as_str(), d.stage, d.operation),
            ("NX-E0003", Stage::Types, Operation::AnalysisLimit)
        );
        assert_eq!(
            d.data.get("budget_kind"),
            Some(&DataValue::String("type_solver_steps".into()))
        );
        assert_eq!(
            d.data.get("budget_scope"),
            Some(&DataValue::String("compilation".into()))
        );
        assert_eq!(
            d.data.get("incomplete_analysis"),
            Some(&DataValue::String("types".into()))
        );
        assert_eq!(d.data.get("limit"), Some(&DataValue::UInt(1_000_000)));
        assert_eq!(d.data.get("observed"), Some(&DataValue::UInt(1_000_001)));
        d.validate("quota.n", "x").unwrap();
    }

    #[test]
    fn default_solver_limit_accepts_its_last_granted_step() {
        // F0-0002 §4.1.2.2/RD127 and §11.1: the generated source exercises
        // the normative solver limit; its debug AST needs a wider test stack.
        std::thread::Builder::new()
            .stack_size(crate::frontend_stack_size("SOLVER_TEST"))
            .spawn(default_solver_limit_on_wide_stack)
            .unwrap()
            .join()
            .unwrap();
    }

    #[test]
    fn measured_solver_test_stack_minimum_still_accepts_last_granted_step() {
        let minimum = if cfg!(debug_assertions) { 4096 } else { 512 };
        let output = std::process::Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "check::types::quota_tests::default_solver_limit_accepts_its_last_granted_step",
            ])
            .env("NEXO_PROBE_SOLVER_TEST_KIB", minimum.to_string())
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{minimum} KiB: {} {}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }

    fn default_solver_limit_on_wide_stack() {
        use std::sync::atomic::Ordering::Relaxed;
        let _campaign = crate::test_alloc::campaign_guard();
        let file = "quota_exact_default.n";
        let value = format!("{}1{}", "[".repeat(100), "]".repeat(100));
        let prefix = format!("fn Main(){{let a={value};let b={value};");
        let measure = |source: &str| {
            TYPE_SOLVER_WORK.store(0, Relaxed);
            let ast = crate::frontend::parse::parse(file, source).unwrap();
            let resolved = names::resolve(file, source, &ast).unwrap();
            let report = check_with_partial_limit(file, source, &ast, &resolved, 1_100_000);
            assert!(report.errors.is_empty(), "{:?}", report.errors);
            TYPE_SOLVER_WORK.load(Relaxed)
        };
        let baseline = measure(&format!("{prefix}}}"));
        let array_call = "AssertEq(a,b);";
        let scalar_call = "AssertEq(1,1);";
        let array_step = measure(&format!("{prefix}{array_call}}}")) - baseline;
        let scalar_step = measure(&format!("{prefix}{scalar_call}}}")) - baseline;
        assert_eq!((array_step, scalar_step), (101, 1));
        let remaining = 1_000_000 - baseline;
        let source = format!(
            "{prefix}{}{}}}",
            array_call.repeat((remaining / array_step) as usize),
            scalar_call.repeat((remaining % array_step) as usize)
        );
        let ast = crate::frontend::parse::parse(file, &source).unwrap();
        let resolved = names::resolve(file, &source, &ast).unwrap();
        let default = check_with_partial(file, &source, &ast, &resolved);
        assert!(default.errors.is_empty(), "{:?}", default.errors);
        let shortened = check_with_partial_limit(file, &source, &ast, &resolved, 999_999);
        assert_eq!(shortened.errors.len(), 1, "{:?}", shortened.errors);
        assert_eq!(shortened.errors[0].code, "NX-E0003");
        assert_eq!(
            shortened.errors[0].data.get("observed"),
            Some(&DataValue::UInt(1_000_000))
        );
    }

    #[test]
    fn adr_gap_unresolved_assert_eq_charges_each_deferred_constraint() {
        // F0-0002 §11.1 fixes logical solver charges but leaves the cost of
        // a pending leaf constraint open. This source has no later
        // assignment, so its type is determined inside AssertEq itself.
        // Current behavior is 3d+3 versus d+1 for concrete values.
        use std::sync::atomic::Ordering::Relaxed;
        let _campaign = crate::test_alloc::campaign_guard();
        let file = "adr_gap_unresolved_cost.n";
        let steps = |source: &str| {
            TYPE_SOLVER_WORK.store(0, Relaxed);
            let ast = crate::frontend::parse::parse(file, source).unwrap();
            let resolved = names::resolve(file, source, &ast).unwrap();
            let _ = check_with_partial(file, source, &ast, &resolved);
            TYPE_SOLVER_WORK.load(Relaxed)
        };
        for depth in 1..=4 {
            let concrete = format!("{}1{}", "[".repeat(depth), "]".repeat(depth));
            let deferred_value = format!("{}q{}", "[".repeat(depth), "]".repeat(depth));
            let prefix = format!(
                "fn Main(){{var q=(if true then {{return;}}else{{return;}});let a={deferred_value};let b={concrete};"
            );
            assert_eq!(
                steps(&format!("{prefix}AssertEq(a,b);}}")) - steps(&format!("{prefix}}}")),
                3 * depth as u64 + 3,
                "deferred leaf at depth {depth}"
            );
            let prefix = format!("fn Main(){{let a={concrete};let b={concrete};");
            assert_eq!(
                steps(&format!("{prefix}AssertEq(a,b);}}")) - steps(&format!("{prefix}}}")),
                depth as u64 + 1,
                "concrete leaf at depth {depth}"
            );
        }
    }

    #[test]
    fn nested_unification_charges_each_array_pair_at_the_reduced_boundary() {
        let resolved = Resolved {
            signatures: HashMap::new(),
            constants: HashMap::new(),
            uses: HashMap::new(),
            calls: HashMap::new(),
        };
        let span = Span {
            file: "quota.n".into(),
            start: 0,
            end: 1,
        };
        let nested = |depth| {
            let mut ty = Type::I64;
            for _ in 0..depth {
                ty = Type::Array(Arc::new(ty));
            }
            ty
        };
        let mut granted = empty_checker(&resolved);
        granted.solver_limit = 32;
        granted
            .unify_independent(
                nested(31),
                nested(31),
                span.clone(),
                span.clone(),
                span.clone(),
                false,
            )
            .unwrap();
        assert_eq!(granted.solver_steps, 32);
        let mut denied = empty_checker(&resolved);
        denied.solver_limit = 32;
        let mut error = denied
            .unify_independent(
                nested(32),
                nested(32),
                span.clone(),
                span.clone(),
                span.clone(),
                false,
            )
            .unwrap_err();
        assert_eq!(denied.solver_steps, 32);
        assert_eq!(error.code, "NX-E0003");
        assert_eq!(
            error.data.get("budget_kind"),
            Some(&DataValue::String("type_solver_steps".into()))
        );
        assert_eq!(error.data.get("observed"), Some(&DataValue::UInt(33)));
        error.profile_id = "f0-0002-b-test-types-32-v2".into();
        error.validate("quota.n", "x").unwrap();
    }

    #[test]
    fn deferred_if_join_charges_the_reduced_boundary() {
        // F0-0002 §4.1.2.2/RD127 resolves the constructor; §11.1 and the
        // §11.1 reports the first denied solver charge once with
        // observed=limit+1; exact reduced thresholds belong to I10a.
        let source = "fn Main(){let x=(if true then [] else [1]);}";
        let ast = crate::frontend::parse::parse("probe.n", source).unwrap();
        let resolved = names::resolve("probe.n", source, &ast).unwrap();
        let full = check_with_partial_limit("probe.n", source, &ast, &resolved, 1_000_000);
        assert!(full.errors.is_empty(), "{:?}", full.errors);
        for limit in 1..=8 {
            let report = check_with_partial_limit("probe.n", source, &ast, &resolved, limit);
            assert_eq!(report.errors.len(), 1, "limit {limit}: {:?}", report.errors);
            let d = &report.errors[0];
            assert_eq!(
                (d.code.as_str(), d.stage, d.operation),
                ("NX-E0003", Stage::Types, Operation::AnalysisLimit)
            );
            assert_eq!(
                d.data.get("budget_kind"),
                Some(&DataValue::String("type_solver_steps".into()))
            );
            assert_eq!(
                d.data.get("budget_scope"),
                Some(&DataValue::String("compilation".into()))
            );
            assert_eq!(
                d.data.get("incomplete_analysis"),
                Some(&DataValue::String("types".into()))
            );
            assert_eq!(d.data.get("limit"), Some(&DataValue::UInt(limit)));
            assert_eq!(d.data.get("observed"), Some(&DataValue::UInt(limit + 1)));
        }
    }

    #[test]
    fn nested_array_value_keeps_one_global_quota_error() {
        // F0-0002 §§11.1, 13.9: the first denied global charge is unique,
        // even when an earlier array child has already tainted the value.
        let source = "fn Main(){let x:Array<Array<Bool>>=(if true then [[true,54968],[false,true]] else {value:[[false,true],[9588,72065]];});}";
        let ast = crate::frontend::parse::parse("quota.n", source).unwrap();
        let resolved = names::resolve("quota.n", source, &ast).unwrap();
        let full = check_with_partial_limit("quota.n", source, &ast, &resolved, 1_000_000);
        for limit in 1..=11 {
            let reduced = check_with_partial_limit("quota.n", source, &ast, &resolved, limit);
            let quotas: Vec<_> = reduced
                .errors
                .iter()
                .filter(|d| d.code == "NX-E0003")
                .collect();
            {
                assert_eq!(quotas.len(), 1, "limit {limit}: {:?}", reduced.errors);
                assert_eq!(
                    quotas[0].data.get("observed"),
                    Some(&DataValue::UInt(limit + 1))
                );
                for d in reduced.errors.iter().filter(|d| d.code != "NX-E0003") {
                    assert!(
                        full.errors.iter().any(|baseline| baseline.code == d.code
                            && baseline.subcause == d.subcause
                            && baseline.primary_span == d.primary_span
                            && baseline.data == d.data),
                        "limit {limit}: {d:?}"
                    );
                }
            }
        }
    }

    #[test]
    fn nested_copyable_check_charges_each_recursive_child() {
        let source = "fn F0(x: view Array<Array<I64>>){let v0:I64=85009;AssertEq([[]],[[false]]);}fn Main(){let v0:Array<Array<Bool>>=[[false],[false]];}";
        let ast = crate::frontend::parse::parse("quota.n", source).unwrap();
        let resolved = names::resolve("quota.n", source, &ast).unwrap();
        let report = check_with_partial_limit("quota.n", source, &ast, &resolved, 6);
        let quota: Vec<_> = report
            .errors
            .iter()
            .filter(|d| d.code == "NX-E0003")
            .collect();
        assert_eq!(quota.len(), 1, "{:?}", report.errors);
        assert_eq!(
            quota[0].primary_span.as_ref().map(|s| (s.start, s.end)),
            Some((60, 62))
        );
        assert_eq!(quota[0].data.get("limit"), Some(&DataValue::UInt(6)));
        assert_eq!(quota[0].data.get("observed"), Some(&DataValue::UInt(7)));
    }

    #[test]
    fn deferred_assert_eq_copyability_uses_first_operand_at_quota_boundary() {
        // F0-0002 §11.1 reports the first denied solver charge once
        // with observed=limit+1. The exact reduced-profile site is I10a.
        let source =
            include_str!("../../tests/fixtures/i07c/deferred_assert_eq_copyability.n").trim();
        let ast = crate::frontend::parse::parse("quota.n", source).unwrap();
        let resolved = names::resolve("quota.n", source, &ast).unwrap();
        for limit in 26..=28 {
            let report = check_with_partial_limit("quota.n", source, &ast, &resolved, limit);
            let quota = report
                .errors
                .iter()
                .filter(|d| d.code == "NX-E0003")
                .collect::<Vec<_>>();
            assert_eq!(quota.len(), 1, "limit {limit}: {:?}", report.errors);
            assert_eq!(
                quota[0].data.get("observed"),
                Some(&DataValue::UInt(limit + 1))
            );
        }
    }

    #[test]
    fn deferred_array_selector_keeps_exact_quota_charge_site() {
        // F0-0002 §11.1 charges the same logical restriction at this
        // reduced boundary whether an array result is already concrete.
        let source =
            include_str!("../../tests/fixtures/i07c/deferred_array_selector_quota.n").trim();
        let ast = crate::frontend::parse::parse("quota.n", source).unwrap();
        let resolved = names::resolve("quota.n", source, &ast).unwrap();
        let report = check_with_partial_limit("quota.n", source, &ast, &resolved, 33);
        let quota = report.errors.iter().find(|d| d.code == "NX-E0003").unwrap();
        assert_eq!(
            quota.primary_span.as_ref().map(|s| (s.start, s.end)),
            Some((184, 188)),
            "{:?}",
            report.errors
        );
        assert_eq!(quota.data.get("observed"), Some(&DataValue::UInt(34)));
    }

    #[test]
    fn failed_branch_join_keeps_reduced_quota_error_prefix() {
        // F0-0002 §11.1 keeps the first denied charge and the converged
        // prefix; a failed branch must not add an unrelated capability error.
        let source = include_str!("../../tests/fixtures/i07c/failed_branch_join_quota.n").trim();
        let ast = crate::frontend::parse::parse("quota.n", source).unwrap();
        let resolved = names::resolve("quota.n", source, &ast).unwrap();
        for limit in [30, 31] {
            let report = check_with_partial_limit("quota.n", source, &ast, &resolved, limit);
            assert!(
                report.errors.iter().any(|d| d.code == "NX-E0003"),
                "{limit}: {:?}",
                report.errors
            );
            assert!(
                !report.errors.iter().any(|d| d.code == "NX-E0006"
                    && d.primary_span
                        .as_ref()
                        .is_some_and(|s| (s.start, s.end) == (176, 213))),
                "{limit}: {:?}",
                report.errors
            );
        }
    }

    #[test]
    fn tainted_array_stops_after_the_first_denied_charge() {
        // F0-0002 §11.1 retains only converged diagnostics at a reduced
        // boundary; swallowing the denied charge can fabricate NX-E0113.
        let source = include_str!("../../tests/fixtures/i07c/tainted_array_quota.n")
            .trim()
            .replace("<NL>", "\n");
        let ast = crate::frontend::parse::parse("quota.n", &source).unwrap();
        let resolved = names::resolve("quota.n", &source, &ast).unwrap();
        for limit in [3, 4, 6, 7] {
            let report = check_with_partial_limit("quota.n", &source, &ast, &resolved, limit);
            assert!(
                report.errors.iter().any(|d| d.code == "NX-E0003"),
                "{limit}: {:?}",
                report.errors
            );
            assert!(
                !report.errors.iter().any(|d| d.code == "NX-E0113"
                    && d.primary_span
                        .as_ref()
                        .is_some_and(|s| (s.start, s.end) == (180, 188))),
                "{limit}: {:?}",
                report.errors
            );
        }
    }

    #[test]
    fn quota_does_not_hide_converged_later_capability_error() {
        // F0-0002 §11.1 preserves established independent diagnostics.
        // The exact reduced-profile charge boundary belongs to I10a.
        let source = include_str!("../../tests/fixtures/i07c/later_capability_quota.n").trim();
        let ast = crate::frontend::parse::parse("quota.n", source).unwrap();
        let resolved = names::resolve("quota.n", source, &ast).unwrap();
        let mut observed_with_quota = false;
        for limit in 0..=40 {
            let report = check_with_partial_limit("quota.n", source, &ast, &resolved, limit);
            let quota = report
                .errors
                .iter()
                .filter(|d| d.code == "NX-E0003")
                .collect::<Vec<_>>();
            assert!(quota.len() <= 1, "{limit}: {:?}", report.errors);
            if let Some(denied) = quota.first() {
                assert_eq!(
                    denied.data.get("observed"),
                    Some(&DataValue::UInt(limit + 1))
                );
                if report.errors.iter().any(|d| {
                    d.code == "NX-E0006"
                        && d.primary_span
                            .as_ref()
                            .is_some_and(|s| (s.start, s.end) == (257, 261))
                }) {
                    observed_with_quota = true;
                }
            }
        }
        assert!(observed_with_quota);
    }

    #[test]
    #[ignore = "three-function reduced type-solver quota campaign"]
    fn generated_multifunction_quota_has_one_denied_charge_and_exact_prefix() {
        // F0-0002 §§11.1, 13.9: 75 source shapes × 40 limits.
        // Main contains enough independent annotated bindings to guarantee
        // exhaustion at every reduced limit even if A and B finish first.
        let padding = (0..41)
            .map(|index| format!("let pad{index}:I64={index};"))
            .collect::<String>();
        for case in 0..75 {
            let atom = case + 1;
            let first = match case % 5 {
                0 => format!("let x:Array<Array<Bool>>=[[true,{atom}],[false,true]];"),
                1 => format!("let x=(if true then [true,{atom}] else [false,true]);"),
                2 => format!("AssertEq([{atom},true],[false,{atom}]);"),
                3 => format!("let x:Array<I64>=[true,{atom},false];"),
                _ => format!("let x=(if true then {{value:[{atom},true];}} else [false,{atom}]);"),
            };
            let second = match case % 3 {
                0 => format!("let y=[{atom},true,false];"),
                1 => format!("let y:I64=(if true then true else {atom});"),
                _ => format!("let y:Array<Bool>=[{atom},false];"),
            };
            let source = format!(
                "fn A(){{{first}}}fn B(){{{second}}}fn Main(){{let z=[1,true,false];{padding}}}"
            );
            let ast = crate::frontend::parse::parse("quota.n", &source).unwrap();
            let resolved = names::resolve("quota.n", &source, &ast).unwrap();
            let full = check_with_partial_limit("quota.n", &source, &ast, &resolved, 1_000_000);
            for limit in 1..=40 {
                let reduced = check_with_partial_limit("quota.n", &source, &ast, &resolved, limit);
                let quotas: Vec<_> = reduced
                    .errors
                    .iter()
                    .filter(|d| d.code == "NX-E0003")
                    .collect();
                assert_eq!(
                    quotas.len(),
                    1,
                    "case {case}, limit {limit}: {:?}",
                    reduced.errors
                );
                if let Some(quota) = quotas.first() {
                    assert_eq!(
                        quota.data.get("observed"),
                        Some(&DataValue::UInt(limit + 1))
                    );
                }
                for d in reduced.errors.iter().filter(|d| d.code != "NX-E0003") {
                    assert!(
                        full.errors.iter().any(|baseline| baseline.code == d.code
                            && baseline.subcause == d.subcause
                            && baseline.primary_span == d.primary_span
                            && baseline.related_spans == d.related_spans
                            && baseline.data == d.data),
                        "case {case}, limit {limit}: {d:?}"
                    );
                }
            }
        }
    }

    #[test]
    fn reduced_type_solver_profile_keeps_exact_converged_subset() {
        let mut source = String::from("fn A(){let x=true+1;}fn B(){");
        for index in 0..64 {
            source.push_str(&format!("let x{index}:I64=0;"));
        }
        source.push_str("}fn C(){let y=false+1;}");
        let ast = crate::frontend::parse::parse("type_quota.n", &source).unwrap();
        let resolved = names::resolve("type_quota.n", &source, &ast).unwrap();
        let unlimited =
            check_with_partial_limit("type_quota.n", &source, &ast, &resolved, 1_000_000);
        assert_eq!(unlimited.errors.len(), 2);
        let mut reduced = check_with_partial_limit("type_quota.n", &source, &ast, &resolved, 32);
        for error in &mut reduced.errors {
            error.profile_id = "f0-0002-b-test-types-32-v2".into();
        }
        assert_eq!(reduced.errors.len(), 2);
        let prior = &reduced.errors[0];
        let baseline = &unlimited.errors[0];
        assert_eq!(
            (
                prior.code.as_str(),
                prior.subcause.as_deref(),
                prior.stage,
                prior.operation
            ),
            (
                "NX-E0102",
                Some("type_mismatch"),
                Stage::Types,
                Operation::TypesCheck
            )
        );
        assert_eq!(prior.data.len(), 2);
        assert_eq!(
            prior.data.get("expected_type"),
            Some(&DataValue::String("I64".into()))
        );
        assert_eq!(
            prior.data.get("found_type"),
            Some(&DataValue::String("Bool".into()))
        );
        assert_eq!(
            prior
                .primary_span
                .as_ref()
                .map(|span| (span.start, span.end)),
            Some((13, 17))
        );
        assert_eq!(prior.related_spans.len(), 1);
        assert_eq!(
            prior.related_spans[0].role,
            crate::diagnostic::RelatedRole::FoundType
        );
        assert_eq!(
            (
                prior.related_spans[0].span.start,
                prior.related_spans[0].span.end
            ),
            (13, 17)
        );
        assert_eq!(
            (
                &prior.code,
                &prior.subcause,
                prior.stage,
                &prior.data,
                &prior.primary_span,
                &prior.related_spans
            ),
            (
                &baseline.code,
                &baseline.subcause,
                baseline.stage,
                &baseline.data,
                &baseline.primary_span,
                &baseline.related_spans
            ),
        );
        assert_eq!(prior.profile_id, "f0-0002-b-test-types-32-v2");
        let quota = &reduced.errors[1];
        assert_eq!(
            (quota.code.as_str(), quota.stage),
            ("NX-E0003", Stage::Types)
        );
        assert_eq!(quota.subcause, None);
        assert_eq!(quota.operation, Operation::AnalysisLimit);
        assert_eq!(quota.data.len(), 5);
        assert!(quota.related_spans.is_empty());
        assert_eq!(quota.profile_id, "f0-0002-b-test-types-32-v2");
        assert_eq!(
            quota.data.get("budget_kind"),
            Some(&DataValue::String("type_solver_steps".into()))
        );
        assert_eq!(
            quota.data.get("budget_scope"),
            Some(&DataValue::String("compilation".into()))
        );
        assert_eq!(
            quota.data.get("incomplete_analysis"),
            Some(&DataValue::String("types".into()))
        );
        assert_eq!(quota.data.get("limit"), Some(&DataValue::UInt(32)));
        assert_eq!(quota.data.get("observed"), Some(&DataValue::UInt(33)));
        assert!(
            quota
                .primary_span
                .as_ref()
                .is_some_and(|s| s.start < s.end && s.end <= source.len() as u64)
        );
        assert!(
            unlimited.errors[1].primary_span.as_ref().unwrap().start
                > quota.primary_span.as_ref().unwrap().start
        );
        for error in &reduced.errors {
            error.validate("type_quota.n", &source).unwrap();
        }
    }

    #[test]
    fn monomorphic_solution_is_invariant_under_constraint_permutations() {
        let resolved = Resolved {
            signatures: HashMap::new(),
            constants: HashMap::new(),
            uses: HashMap::new(),
            calls: HashMap::new(),
        };
        let origin = |start| Span {
            file: "quota.n".into(),
            start,
            end: start + 1,
        };
        for permutation in [
            [0, 1, 2],
            [0, 2, 1],
            [1, 0, 2],
            [1, 2, 0],
            [2, 0, 1],
            [2, 1, 0],
        ] {
            for contradictory in [false, true] {
                let mut checker = empty_checker(&resolved);
                let a = checker.new_variable(&origin(0)).unwrap();
                let b = checker.new_variable(&origin(1)).unwrap();
                let mut rejected = false;
                for step in permutation {
                    let result = match step {
                        0 => checker.bind_independent(a, Type::I64, origin(0), origin(0), false),
                        1 => checker.union(a, b, origin(1)),
                        _ => checker.bind_independent(
                            b,
                            if contradictory { Type::Bool } else { Type::I64 },
                            origin(2),
                            origin(2),
                            false,
                        ),
                    };
                    if let Err(error) = result {
                        assert_eq!(error.code, "NX-E0102");
                        rejected = true;
                        break;
                    }
                }
                assert_eq!(rejected, contradictory, "{permutation:?}");
                if !rejected {
                    assert_eq!(checker.concrete(a).map(|v| v.0), Some(Type::I64));
                    assert_eq!(checker.concrete(b).map(|v| v.0), Some(Type::I64));
                }
            }
        }
    }

    #[test]
    fn generated_constraint_orders_have_the_same_acceptance() {
        let resolved = Resolved {
            signatures: HashMap::new(),
            constants: HashMap::new(),
            uses: HashMap::new(),
            calls: HashMap::new(),
        };
        let mut random = 0xa0761d6478bd642f_u64;
        for case in 0..10_000 {
            let mut events = [(0_u8, 0_usize, 0_usize); 8];
            for event in &mut events {
                random ^= random << 13;
                random ^= random >> 7;
                random ^= random << 17;
                *event = (
                    (random % 5) as u8,
                    ((random >> 8) % 4) as usize,
                    ((random >> 16) % 4) as usize,
                );
            }
            // Reference: equality edges form connected components regardless
            // of processing order; a component may carry only one concrete tag.
            let mut labels = [0_usize, 1, 2, 3];
            for &(kind, left, right) in &events {
                if kind != 0 {
                    continue;
                }
                let from = labels[right];
                let to = labels[left];
                for label in &mut labels {
                    if *label == from {
                        *label = to;
                    }
                }
            }
            let mut tags = [0_u8; 4];
            let mut contradictory = false;
            for &(kind, left, _) in &events {
                if kind == 0 {
                    continue;
                }
                let tag = 1_u8 << (kind - 1);
                tags[labels[left]] |= tag;
                contradictory |= tags[labels[left]].count_ones() > 1;
            }
            for order in 0..5 {
                let mut permutation = [0_usize, 1, 2, 3, 4, 5, 6, 7];
                if order > 0 {
                    for i in (1..permutation.len()).rev() {
                        random ^= random << 13;
                        random ^= random >> 7;
                        random ^= random << 17;
                        permutation.swap(i, (random as usize) % (i + 1));
                    }
                }
                let mut checker = empty_checker(&resolved);
                let span = |n| Span {
                    file: "quota.n".into(),
                    start: n,
                    end: n + 1,
                };
                let ids: [u64; 4] =
                    std::array::from_fn(|i| checker.new_variable(&span(i as u64)).unwrap());
                let mut rejected = false;
                for event_index in permutation {
                    let (kind, left, right) = events[event_index];
                    let at = span((event_index + 4) as u64);
                    let result = match kind {
                        0 => checker.union(ids[left], ids[right], at),
                        1 => checker.bind_independent(ids[left], Type::I64, at.clone(), at, false),
                        2 => checker.bind_independent(ids[left], Type::Bool, at.clone(), at, false),
                        3 => checker.bind_independent(
                            ids[left],
                            Type::Array(Arc::new(Type::I64)),
                            at.clone(),
                            at,
                            false,
                        ),
                        _ => checker.bind_independent(
                            ids[left],
                            Type::Array(Arc::new(Type::Bool)),
                            at.clone(),
                            at,
                            false,
                        ),
                    };
                    if let Err(error) = result {
                        assert_eq!(error.code, "NX-E0102", "case {case}, order {order}");
                        rejected = true;
                        break;
                    }
                }
                assert_eq!(
                    rejected, contradictory,
                    "case {case}, order {order}: {events:?}"
                );
            }
        }
    }
    #[test]
    fn i07c_survivor_refuted_binary_and_guarded_results() {
        let source = "fn Main(){}";
        let ast = parse::parse_new_profile("survivor.n", source).unwrap();
        let resolved = names::resolve_with_option("survivor.n", source, &ast, true).unwrap();
        let mut graph = empty_checker(&resolved).graph;
        let binary = (10, 20);
        let guarded = (30, 40);
        graph.transition(
            binary,
            ObligationClass::BinaryDomain,
            ObligationStatus::Refuted,
        );
        assert!(graph.refuted_result_sites().contains(&binary));
        graph.occurrences.clear();
        graph.transition(
            guarded,
            ObligationClass::GuardedComparison,
            ObligationStatus::Refuted,
        );
        assert!(graph.refuted_result_sites().contains(&guarded));
    }

    #[test]
    fn i07c_survivor_conditional_bindings_follow_their_result_sites() {
        let source = "fn Main(){}";
        let ast = parse::parse_new_profile("survivor.n", source).unwrap();
        let resolved = names::resolve_with_option("survivor.n", source, &ast, true).unwrap();
        let mut graph = empty_checker(&resolved).graph;
        let site = |start| Span {
            file: "survivor.n".into(),
            start,
            end: start + 1,
        };
        let nominal = site(10);
        let binary = site(20);
        let guarded = site(30);
        graph.obligations = vec![
            ObligationKind::DomainNominal(Box::new(PendingNominalDomain {
                variable: 0,
                domain: "domain:array",
                call: nominal.clone(),
                nominal: Type::I64,
                expected: None,
                expected_span: None,
                depends_on: None,
                conditional_bindings: Vec::new(),
            })),
            ObligationKind::BinaryDomain(Box::new(PendingBinaryDomain {
                variable: 1,
                operator: binary.clone(),
                found: binary.clone(),
                domain: "I64",
                result: binary.clone(),
                conditional_bindings: Vec::new(),
            })),
            ObligationKind::GuardedComparison(Box::new(PendingGuardedComparison {
                variable: 2,
                domain: "I64",
                expected: Type::I64,
                found: Type::Bool,
                expected_origin: guarded.clone(),
                found_site: guarded.clone(),
                result: guarded.clone(),
                consumer_expected: None,
                consumer_origin: None,
                domain_error_if_equal: None,
                additional_domains: Vec::new(),
                projections: Vec::new(),
                dependent_results: Vec::new(),
                require_resolved: false,
                conditional_bindings: Vec::new(),
            })),
        ];
        for (id, result) in [(3, &nominal), (4, &binary), (5, &guarded)] {
            graph.attach_conditional_binding(result, id, Type::I64);
        }
        assert_eq!(
            graph.conditional_bindings(),
            vec![
                (3, nominal, Type::I64),
                (4, binary, Type::I64),
                (5, guarded, Type::I64),
            ]
        );
    }

    #[test]
    fn i07c_survivor2_nested_shape_hints_reach_the_leaf() {
        let array = Type::Array(Arc::new(Type::I64));
        assert!(
            Checker::type_shape_hint(&array)
                == ShapeHint::Array(Box::new(ShapeHint::Scalar(Type::I64)))
        );
        let left = ShapeHint::Array(Box::new(ShapeHint::Scalar(Type::I64)));
        let right = ShapeHint::Array(Box::new(ShapeHint::Scalar(Type::Bool)));
        assert!(Checker::shape_hints_conflict(&left, &right));
        assert!(!Checker::shape_hints_conflict(&left, &left));
    }

    #[test]
    fn i07c_survivor2_pending_operation_types_match_their_sites() {
        let resolved = Resolved {
            signatures: HashMap::new(),
            constants: HashMap::new(),
            uses: HashMap::new(),
            calls: HashMap::new(),
        };
        let mut checker = empty_checker(&resolved);
        let nominal = site(10);
        let guarded = site(20);
        checker
            .graph
            .register(ObligationKind::DomainNominal(Box::new(
                PendingNominalDomain {
                    variable: 0,
                    domain: "I64",
                    call: nominal.clone(),
                    nominal: Type::I64,
                    expected: None,
                    expected_span: None,
                    depends_on: None,
                    conditional_bindings: Vec::new(),
                },
            )));
        checker
            .graph
            .register(ObligationKind::GuardedComparison(Box::new(
                PendingGuardedComparison {
                    variable: 0,
                    domain: "I64",
                    expected: Type::I64,
                    found: Type::Bool,
                    expected_origin: guarded.clone(),
                    found_site: guarded.clone(),
                    result: guarded.clone(),
                    consumer_expected: None,
                    consumer_origin: None,
                    domain_error_if_equal: None,
                    additional_domains: Vec::new(),
                    projections: Vec::new(),
                    dependent_results: Vec::new(),
                    require_resolved: false,
                    conditional_bindings: Vec::new(),
                },
            )));
        assert!(
            checker.pending_operation_type(&Expr::Integer("1".into(), nominal)) == Some(Type::I64)
        );
        assert!(checker.pending_operation_type(&Expr::Bool(true, guarded)) == Some(Type::Bool));
        assert!(
            checker
                .pending_operation_type(&Expr::Bool(true, site(30)))
                .is_none()
        );
    }

    #[test]
    fn i07c_survivor2_pending_nominal_requires_live_domain() {
        let resolved = Resolved {
            signatures: HashMap::new(),
            constants: HashMap::new(),
            uses: HashMap::new(),
            calls: HashMap::new(),
        };
        for (concrete, array, failed, bottom, expected) in [
            (Some(Type::Bool), false, false, false, false),
            (None, true, false, false, false),
            (None, true, false, true, false),
            (None, false, true, false, false),
            (None, false, false, false, true),
            (None, false, false, true, true),
        ] {
            let mut checker = empty_checker(&resolved);
            let root = checker.new_variable(&site(1)).unwrap();
            let child = checker.new_variable(&site(2)).unwrap();
            let node = checker.graph.type_nodes.get_mut(&root).unwrap();
            node.concrete = concrete.map(|ty| (ty, site(1)));
            node.array_child = array.then_some(child);
            node.failed_constructor = failed;
            node.bottom_only = bottom;
            checker
                .graph
                .register(ObligationKind::BinaryDomain(Box::new(
                    PendingBinaryDomain {
                        variable: root,
                        operator: site(3),
                        found: site(3),
                        domain: "I64",
                        result: site(4),
                        conditional_bindings: Vec::new(),
                    },
                )));
            assert_eq!(
                checker.pending_nominal_available(&Expr::Bool(true, site(4))),
                expected,
                "array={array} failed={failed} bottom={bottom}"
            );
        }
    }

    #[test]
    fn i07c_survivor2_solver_preserves_constructor_identity() {
        let resolved = Resolved {
            signatures: HashMap::new(),
            constants: HashMap::new(),
            uses: HashMap::new(),
            calls: HashMap::new(),
        };
        let mut checker = empty_checker(&resolved);
        let root = checker.new_variable(&site(1)).unwrap();
        let child = checker.new_variable(&site(2)).unwrap();
        checker
            .graph
            .type_nodes
            .get_mut(&root)
            .unwrap()
            .variant_children = Some(UnionDescriptor::option(child));
        assert!(checker.require_array(root, &site(3)).is_err());

        checker.graph.type_nodes.get_mut(&root).unwrap().concrete =
            Some((Type::option(Arc::new(Type::I64)), site(4)));
        assert!(checker.concrete(root).is_some());
        checker.graph.type_nodes.get_mut(&root).unwrap().concrete = Some((
            Type::result(Arc::new(Type::I64), Arc::new(Type::Bool)),
            site(5),
        ));
        assert!(checker.concrete(root).is_none());

        let scalar = checker.new_variable(&site(6)).unwrap();
        checker.graph.type_nodes.get_mut(&scalar).unwrap().concrete = Some((Type::Bool, site(6)));
        assert!(checker.require_array(scalar, &site(7)).is_err());
        let element = checker.new_variable(&site(8)).unwrap();
        checker
            .graph
            .type_nodes
            .get_mut(&scalar)
            .unwrap()
            .array_child = Some(element);
        assert!(checker.concrete(scalar).is_none());
    }

    #[test]
    fn i07c_survivor2_seeded_array_child_starts_clean_and_keeps_existing_leaf() {
        let resolved = Resolved {
            signatures: HashMap::new(),
            constants: HashMap::new(),
            uses: HashMap::new(),
            calls: HashMap::new(),
        };
        let mut checker = empty_checker(&resolved);
        let root = checker.new_variable(&site(1)).unwrap();
        checker.graph.shape_plan.insert(10, vec![site(2)]);
        checker.seed_planned_shape(10, root);
        let child = checker.graph.type_nodes[&root].array_child.unwrap();
        assert!(!checker.graph.type_nodes[&child].failed_constructor);
        assert!(!checker.graph.type_nodes[&child].deferred_join);
        checker.graph.type_nodes.get_mut(&child).unwrap().concrete = Some((Type::Bool, site(3)));
        checker.graph.concrete_plan.insert(
            10,
            ConcreteEvidence {
                ty: Type::Array(Arc::new(Type::I64)),
                origin: site(4),
                intrinsic: false,
            },
        );
        checker.seed_planned_shape(10, root);
        assert!(checker.graph.type_nodes[&child].concrete == Some((Type::Bool, site(3))));
    }

    #[test]
    fn i07c_survivor2_closure_restores_enclosing_result_site() {
        let source = "fn Main(){let f=fn []()->I64{return 1;};}";
        let ast = parse::parse_new_profile("closure.n", source).unwrap();
        let resolved = names::resolve_with_option("closure.n", source, &ast, true).unwrap();
        let Statement::Let {
            value: Expr::Closure(closure),
            ..
        } = &ast.functions[0].body.statements[0]
        else {
            panic!("expected closure binding")
        };
        let mut checker = empty_checker(&resolved);
        let enclosing_site = site(90);
        checker.graph.result = Type::Bool;
        checker.graph.result_span = Some(enclosing_site.clone());
        let context = OwnedCheckContext {
            expected: None,
            expected_span: None,
            check_here: false,
            anchor: None,
            outer_compare: false,
            related_expected: false,
            deferred_element: false,
        };
        let progress = checker.closure_begin(closure, context).unwrap();
        checker.closure_finish(progress, Ok(())).unwrap();
        assert_eq!(checker.graph.result, Type::Bool);
        assert_eq!(checker.graph.result_span, Some(enclosing_site));
    }

    #[test]
    fn i07c_survivor2_concrete_closure_result_does_not_bind_pending_return() {
        let source = "fn Main(){let f=fn []()->Bool{var q=[];return q[0];};}";
        let ast = parse::parse_new_profile("closure.n", source).unwrap();
        let resolved = names::resolve_with_option("closure.n", source, &ast, true).unwrap();
        let Statement::Let {
            value: Expr::Closure(closure),
            ..
        } = &ast.functions[0].body.statements[0]
        else {
            panic!("expected closure binding")
        };
        let mut checker = empty_checker(&resolved);
        let unresolved = checker.new_variable(&site(1)).unwrap();
        let context = OwnedCheckContext {
            expected: None,
            expected_span: None,
            check_here: false,
            anchor: None,
            outer_compare: false,
            related_expected: false,
            deferred_element: false,
        };
        let mut progress = checker.closure_begin(closure, context).unwrap();
        progress.next = closure.body.statements.len();
        checker.block_depth = 1;
        let action = checker.with_independent_fact((0, 0), |checker, fact| {
            let mut child = checker.graph.child_fact(
                fact,
                span_key(parse::statement_span(&closure.body.statements[1])),
                FactRole::Statement,
                FactInheritance::Empty,
                FactInheritance::Inherit,
            );
            progress.pending_statement = Some(ClosurePending::Return {
                statement: &closure.body.statements[1],
                expected: Type::Bool,
                parent_fact: fact,
            });
            checker.closure_body_advance(
                &mut child,
                &mut progress,
                Some(Ok(Type::Deferred(unresolved))),
            )
        });
        assert!(matches!(action, ClosureAction::Done(Ok(()))));
        assert!(checker.graph.type_nodes[&unresolved].concrete.is_none());
    }

    #[test]
    fn i07c_survivor2_pending_selector_keeps_independent_taint() {
        let source = "fn Main(){var q=[];var r=[];let x=q[(r-r)];r=[3];q=[1];}";
        let ast = parse::parse_new_profile("selector.n", source).unwrap();
        let resolved = names::resolve_with_option("selector.n", source, &ast, true).unwrap();
        let Statement::Let {
            value: Expr::Index { target, index, .. },
            ..
        } = &ast.functions[0].body.statements[2]
        else {
            panic!("expected pending projection")
        };
        let mut checker = empty_checker(&resolved);
        let origin = Span {
            file: "selector.n".into(),
            start: source.find("r=[]").unwrap() as u64,
            end: source.find("r=[]").unwrap() as u64 + 1,
        };
        let base = checker.new_variable(target.span()).unwrap();
        let item = checker.new_variable(index.span()).unwrap();
        let r = checker.new_variable(&origin).unwrap();
        checker.require_array(r, &origin).unwrap();
        checker.graph.locals.insert(
            origin.start,
            Known {
                ty: Type::Deferred(r),
                origin: origin.clone(),
                variable: Some(r),
                invalid: false,
                nominal_sources: HashSet::new(),
            },
        );
        checker
            .graph
            .register(ObligationKind::Projection(Box::new(PendingProjection {
                base,
                item,
                base_span: target.span().clone(),
                result_span: parse::statement_span(&ast.functions[0].body.statements[2]).clone(),
                selector: Arc::new(index.as_ref().clone()),
                selector_occurrence_aliases: Vec::new(),
                selector_variable: None,
                checked_without_context: false,
                selector_invalid: false,
                expected: None,
                expected_span: None,
                related_expected: false,
            })));
        checker.resolve_pending_projections(false).unwrap();
        let flag = checker.graph.obligations.iter().find_map(|obligation| {
            let ObligationKind::Projection(projection) = obligation else {
                return None;
            };
            Some((
                projection.checked_without_context,
                projection.selector_invalid,
            ))
        });
        assert_eq!(flag, Some((true, true)));
    }

    #[test]
    fn i07c_survivor2_selection_depth_keeps_the_current_type() {
        let resolved = Resolved {
            signatures: HashMap::new(),
            constants: HashMap::new(),
            uses: HashMap::new(),
            calls: HashMap::new(),
        };
        let mut checker = empty_checker(&resolved);
        assert!(checker.selection_type(&Type::I64, 129) == Type::I64);
        let nested = Type::Array(Arc::new(Type::I64));
        assert!(checker.selection_type(&nested, 128) == nested);
    }

    #[test]
    fn i07c_survivor2_resolved_join_rebuilds_option_payload() {
        let resolved = Resolved {
            signatures: HashMap::new(),
            constants: HashMap::new(),
            uses: HashMap::new(),
            calls: HashMap::new(),
        };
        let mut checker = empty_checker(&resolved);
        let root = checker.new_variable(&site(1)).unwrap();
        checker.graph.type_nodes.get_mut(&root).unwrap().concrete = Some((Type::I64, site(2)));
        let side = Type::option(Arc::new(Type::Deferred(root)));
        assert!(checker.resolved_join_side(side) == Some(Type::option(Arc::new(Type::I64))));
    }

    #[test]
    fn i07c_survivor2_expression_dependencies_cover_branch_children() {
        fn value_block(value: Expr, at: u64) -> Block {
            Block {
                statements: vec![Statement::Value {
                    value: Some(value),
                    marker: None,
                    span: site(at),
                }],
                span: site(at),
                close_span: site(at + 1),
            }
        }
        fn assert_child(checker: &mut Checker<'_>, parent: &Expr, child: &Expr) {
            let source = (child.span().start + 100, child.span().end + 100);
            checker.graph.occurrences.clear();
            checker
                .graph
                .occurrences
                .entry(span_key(child.span()))
                .or_default()
                .nominal_sources
                .insert(source);
            checker.capture_expression_dependencies(parent);
            assert!(
                checker.graph.occurrences[&span_key(parent.span())]
                    .nominal_sources
                    .contains(&source),
                "missing dependency from {:?}",
                child.span()
            );
        }
        let resolved = Resolved {
            signatures: HashMap::new(),
            constants: HashMap::new(),
            uses: HashMap::new(),
            calls: HashMap::new(),
        };
        let mut checker = empty_checker(&resolved);
        let call = Expr::Call {
            target: Box::new(Expr::Name("F".into(), site(10))),
            args: Vec::new(),
            span: site(9),
        };
        let Expr::Call { target, .. } = &call else {
            unreachable!()
        };
        assert_child(&mut checker, &call, target);

        let branch = Expr::If {
            condition: Box::new(Expr::Bool(true, site(20))),
            then_block: value_block(Expr::Integer("1".into(), site(21)), 22),
            otherwise: Some(Box::new(Else::Block(value_block(
                Expr::Integer("2".into(), site(23)),
                24,
            )))),
            span: site(19),
        };
        let Expr::If {
            condition,
            then_block,
            otherwise,
            ..
        } = &branch
        else {
            unreachable!()
        };
        assert_child(&mut checker, &branch, condition);
        assert_child(
            &mut checker,
            &branch,
            branch_value_expr(then_block).unwrap(),
        );
        let Some(Else::Block(otherwise)) = otherwise.as_deref() else {
            unreachable!()
        };
        assert_child(&mut checker, &branch, branch_value_expr(otherwise).unwrap());

        let selection = Expr::Match(Box::new(parse::Match {
            scrutinee: Box::new(Expr::Bool(true, site(31))),
            mode: None,
            mode_span: None,
            arms: vec![parse::Arm {
                pattern: Pattern::Wildcard(site(32)),
                guard: Some(Expr::Bool(true, site(33))),
                guard_span: Some(site(33)),
                body: value_block(Expr::Integer("3".into(), site(34)), 35),
                span: site(36),
            }],
            span: site(30),
            keyword_span: site(30),
        }));
        let Expr::Match(selection_parts) = &selection else {
            unreachable!()
        };
        assert_child(&mut checker, &selection, &selection_parts.scrutinee);
        assert_child(
            &mut checker,
            &selection,
            selection_parts.arms[0].guard.as_ref().unwrap(),
        );
        assert_child(
            &mut checker,
            &selection,
            branch_value_expr(&selection_parts.arms[0].body).unwrap(),
        );
    }

    #[test]
    fn i07c_survivor2_root_compresses_the_entire_parent_chain() {
        let resolved = Resolved {
            signatures: HashMap::new(),
            constants: HashMap::new(),
            uses: HashMap::new(),
            calls: HashMap::new(),
        };
        let mut checker = empty_checker(&resolved);
        let leaf = checker.new_variable(&site(1)).unwrap();
        let middle = checker.new_variable(&site(2)).unwrap();
        let root = checker.new_variable(&site(3)).unwrap();
        checker.graph.type_nodes.get_mut(&leaf).unwrap().parent = middle;
        checker.graph.type_nodes.get_mut(&middle).unwrap().parent = root;
        assert_eq!(checker.root(leaf), root);
        assert_eq!(checker.graph.type_nodes[&leaf].parent, root);
    }

    #[test]
    fn i07c_survivor2_result_shape_conflict_keeps_payload_identity() {
        let resolved = Resolved {
            signatures: HashMap::new(),
            constants: HashMap::new(),
            uses: HashMap::new(),
            calls: HashMap::new(),
        };
        let mut checker = empty_checker(&resolved);
        let root = checker.new_variable(&site(1)).unwrap();
        let child = checker.new_variable(&site(2)).unwrap();
        checker
            .graph
            .type_nodes
            .get_mut(&root)
            .unwrap()
            .variant_children = Some(UnionDescriptor::option(child));
        let grandchild = checker.new_variable(&site(3)).unwrap();
        checker
            .graph
            .type_nodes
            .get_mut(&child)
            .unwrap()
            .variant_children = Some(UnionDescriptor::option(grandchild));
        assert!(checker.result_shape_conflict(root, &Type::I64, 0));
        assert!(checker.result_shape_conflict(
            root,
            &Type::result(Arc::new(Type::I64), Arc::new(Type::I64)),
            0
        ));
        assert!(checker.result_shape_conflict(root, &Type::option(Arc::new(Type::I64)), 0));
        assert!(!checker.result_shape_conflict(root, &Type::Never, 0));
        assert!(!checker.result_shape_conflict(root, &Type::I64, 129));
    }

    #[test]
    fn i07c_survivor2_shape_conflicts_continue_past_irrelevant_children() {
        let resolved = Resolved {
            signatures: HashMap::new(),
            constants: HashMap::new(),
            uses: HashMap::new(),
            calls: HashMap::new(),
        };
        let mut checker = empty_checker(&resolved);
        let root = checker.new_variable(&site(1)).unwrap();
        let ok = checker.new_variable(&site(2)).unwrap();
        let err = checker.new_variable(&site(3)).unwrap();
        let payload = checker.new_variable(&site(4)).unwrap();
        checker
            .graph
            .type_nodes
            .get_mut(&root)
            .unwrap()
            .variant_children = Some(UnionDescriptor::result(ok, err));
        checker
            .graph
            .type_nodes
            .get_mut(&err)
            .unwrap()
            .variant_children = Some(UnionDescriptor::option(payload));
        let expected = Type::result(Arc::new(Type::Never), Arc::new(Type::I64));
        assert!(checker.result_shape_conflict(root, &expected, 0));

        checker.graph.register(ObligationKind::PatternConstructor {
            union: err,
            span: site(5),
        });
        let pattern_expected = Type::result(Arc::new(Type::I64), Arc::new(Type::I64));
        let diagnostic = checker
            .pattern_shape_mismatch(root, &pattern_expected, &site(6), &site(7))
            .unwrap();
        assert_eq!(diagnostic.code, "NX-E0102");
        assert_eq!(diagnostic.subcause.as_deref(), Some("constructor_shape"));
        assert_eq!(diagnostic.primary_span.as_ref().map(span_key), Some((7, 8)));
        assert_eq!(
            diagnostic.message,
            "La forma del valor no coincide con el tipo esperado."
        );
        assert_eq!(
            diagnostic.data.get("found_constructor"),
            Some(&DataValue::String("Option".into()))
        );
        checker.graph.type_nodes.get_mut(&ok).unwrap().concrete = Some((Type::I64, site(8)));
        assert!(
            checker
                .pattern_shape_mismatch(root, &pattern_expected, &site(6), &site(7))
                .is_some()
        );
    }

    #[test]
    fn i07c_survivor2_pattern_shape_chooses_first_conflicting_child() {
        let resolved = Resolved {
            signatures: HashMap::new(),
            constants: HashMap::new(),
            uses: HashMap::new(),
            calls: HashMap::new(),
        };
        let mut checker = empty_checker(&resolved);
        let root = checker.new_variable(&site(1)).unwrap();
        let ok = checker.new_variable(&site(2)).unwrap();
        let err = checker.new_variable(&site(3)).unwrap();
        let ok_payload = checker.new_variable(&site(4)).unwrap();
        let err_left = checker.new_variable(&site(5)).unwrap();
        let err_right = checker.new_variable(&site(6)).unwrap();
        checker
            .graph
            .type_nodes
            .get_mut(&root)
            .unwrap()
            .variant_children = Some(UnionDescriptor::result(ok, err));
        checker
            .graph
            .type_nodes
            .get_mut(&ok)
            .unwrap()
            .variant_children = Some(UnionDescriptor::option(ok_payload));
        checker
            .graph
            .type_nodes
            .get_mut(&err)
            .unwrap()
            .variant_children = Some(UnionDescriptor::result(err_left, err_right));
        for (id, span) in [(ok, site(7)), (err, site(8))] {
            checker
                .graph
                .register(ObligationKind::PatternConstructor { union: id, span });
        }
        let expected = Type::result(Arc::new(Type::I64), Arc::new(Type::Bool));
        let diagnostic = checker
            .pattern_shape_mismatch(root, &expected, &site(9), &site(10))
            .unwrap();
        assert_eq!(
            diagnostic.data.get("found_constructor"),
            Some(&DataValue::String("Option".into()))
        );
        assert_eq!(diagnostic.related_spans[0].span, site(7));
    }

    #[test]
    fn i07c_survivor3_shape_walk_checks_later_result_payload() {
        let resolved = Resolved {
            signatures: HashMap::new(),
            constants: HashMap::new(),
            uses: HashMap::new(),
            calls: HashMap::new(),
        };
        let mut checker = empty_checker(&resolved);
        let root = checker.new_variable(&site(1)).unwrap();
        let ok = checker.new_variable(&site(2)).unwrap();
        let err = checker.new_variable(&site(3)).unwrap();
        let nested = checker.new_variable(&site(4)).unwrap();
        checker
            .graph
            .type_nodes
            .get_mut(&root)
            .unwrap()
            .variant_children = Some(UnionDescriptor::result(ok, err));
        checker
            .graph
            .type_nodes
            .get_mut(&err)
            .unwrap()
            .variant_children = Some(UnionDescriptor::option(nested));
        let expected = Type::result(Arc::new(Type::I64), Arc::new(Type::Bool));
        assert!(checker.result_shape_conflict(root, &expected, 0));
        let unrelated = checker.new_variable(&site(8)).unwrap();
        checker
            .graph
            .type_nodes
            .get_mut(&ok)
            .unwrap()
            .variant_children = Some(UnionDescriptor::option(unrelated));
        checker.graph.register(ObligationKind::PatternConstructor {
            union: err,
            span: site(5),
        });
        let diagnostic = checker
            .pattern_shape_mismatch(root, &expected, &site(6), &site(7))
            .unwrap();
        assert_eq!(diagnostic.code, "NX-E0102");
        assert_eq!(diagnostic.subcause.as_deref(), Some("constructor_shape"));
        assert_eq!(diagnostic.related_spans[0].span, site(5));
    }

    #[test]
    fn i07c_survivor3_require_union_moves_concrete_payloads_to_children() {
        let resolved = Resolved {
            signatures: HashMap::new(),
            constants: HashMap::new(),
            uses: HashMap::new(),
            calls: HashMap::new(),
        };
        let mut checker = empty_checker(&resolved);
        let root = checker.new_variable(&site(1)).unwrap();
        checker.graph.type_nodes.get_mut(&root).unwrap().concrete = Some((
            Type::result(Arc::new(Type::I64), Arc::new(Type::Bool)),
            site(2),
        ));
        let children = checker
            .require_union(root, UnionKind::Result, &site(3))
            .unwrap();
        assert!(checker.graph.type_nodes[&root].concrete.is_none());
        assert_eq!(
            checker.concrete(root).map(|(ty, _)| ty),
            Some(Type::result(Arc::new(Type::I64), Arc::new(Type::Bool)))
        );
        assert_eq!(
            checker.graph.type_nodes[&root].variant_origin,
            Some(site(3))
        );
        let payloads: Vec<_> = children.variants.iter().filter_map(|v| v.payload).collect();
        assert_eq!(payloads.len(), 2);
        assert_eq!(
            checker.concrete(payloads[0]).map(|(ty, _)| ty),
            Some(Type::I64)
        );
        assert_eq!(
            checker.concrete(payloads[1]).map(|(ty, _)| ty),
            Some(Type::Bool)
        );
    }

    #[test]
    fn i07c_survivor3_union_transfers_single_sided_shape_and_origin() {
        let resolved = Resolved {
            signatures: HashMap::new(),
            constants: HashMap::new(),
            uses: HashMap::new(),
            calls: HashMap::new(),
        };
        let mut checker = empty_checker(&resolved);
        let shaped = checker.new_variable(&site(1)).unwrap();
        let plain = checker.new_variable(&site(2)).unwrap();
        let payload = checker.new_variable(&site(3)).unwrap();
        checker
            .graph
            .type_nodes
            .get_mut(&shaped)
            .unwrap()
            .variant_children = Some(UnionDescriptor::option(payload));
        checker
            .graph
            .type_nodes
            .get_mut(&shaped)
            .unwrap()
            .variant_constructed = true;
        let concrete_origin = site(4);
        checker.graph.type_nodes.get_mut(&plain).unwrap().concrete =
            Some((Type::option(Arc::new(Type::I64)), concrete_origin.clone()));
        checker.union(shaped, plain, site(5)).unwrap();
        let root = checker.root(shaped);
        assert_eq!(root, checker.root(plain));
        assert!(checker.graph.type_nodes[&root].variant_constructed);
        assert_eq!(
            checker.graph.type_nodes[&root].variant_origin,
            Some(concrete_origin)
        );
        assert_eq!(checker.concrete(payload).map(|(ty, _)| ty), Some(Type::I64));
        assert_eq!(
            checker.concrete(root).map(|(ty, _)| ty),
            Some(Type::option(Arc::new(Type::I64)))
        );
    }

    #[test]
    fn i07c_survivor3_failed_union_restores_both_constructor_roots() {
        let resolved = Resolved {
            signatures: HashMap::new(),
            constants: HashMap::new(),
            uses: HashMap::new(),
            calls: HashMap::new(),
        };
        let mut checker = empty_checker(&resolved);
        let left = checker.new_variable(&site(1)).unwrap();
        let right = checker.new_variable(&site(2)).unwrap();
        let left_payload = checker.new_variable(&site(3)).unwrap();
        let right_payload = checker.new_variable(&site(4)).unwrap();
        checker
            .graph
            .type_nodes
            .get_mut(&left)
            .unwrap()
            .variant_children = Some(UnionDescriptor::option(left_payload));
        checker
            .graph
            .type_nodes
            .get_mut(&right)
            .unwrap()
            .variant_children = Some(UnionDescriptor::option(right_payload));
        checker
            .bind_independent(left_payload, Type::I64, site(5), site(5), false)
            .unwrap();
        checker
            .bind_independent(right_payload, Type::Bool, site(6), site(6), false)
            .unwrap();
        let before_steps = checker.solver_steps;
        let error = checker.union(left, right, site(7)).unwrap_err();
        assert_eq!(error.code, "NX-E0102");
        assert_eq!(checker.root(left), left);
        assert_eq!(checker.root(right), right);
        assert_eq!(
            checker.concrete(left_payload).map(|(ty, _)| ty),
            Some(Type::I64)
        );
        assert_eq!(
            checker.concrete(right_payload).map(|(ty, _)| ty),
            Some(Type::Bool)
        );
        assert!(checker.solver_steps > before_steps);
    }

    #[test]
    fn i07c_survivor3_unify_concrete_union_kinds_and_payloads() {
        let resolved = Resolved {
            signatures: HashMap::new(),
            constants: HashMap::new(),
            uses: HashMap::new(),
            calls: HashMap::new(),
        };
        let mut checker = empty_checker(&resolved);
        let option_i64 = Type::option(Arc::new(Type::I64));
        let result_i64 = Type::result(Arc::new(Type::I64), Arc::new(Type::Bool));
        let mismatch = checker
            .unify_independent(
                option_i64.clone(),
                result_i64,
                site(1),
                site(2),
                site(3),
                false,
            )
            .unwrap_err();
        assert_eq!(mismatch.code, "NX-E0102");
        assert_eq!(
            mismatch.data.get("expected_type"),
            Some(&DataValue::String("Option<I64>".into()))
        );
        assert_eq!(
            mismatch.data.get("found_type"),
            Some(&DataValue::String("Result<I64,Bool>".into()))
        );
        let nested_kind = checker
            .unify_independent(
                Type::Array(Arc::new(option_i64.clone())),
                Type::Array(Arc::new(Type::result(
                    Arc::new(Type::I64),
                    Arc::new(Type::Bool),
                ))),
                site(10),
                site(11),
                site(12),
                false,
            )
            .unwrap_err();
        assert_eq!(
            nested_kind.data.get("expected_type"),
            Some(&DataValue::String("Array<Option<I64>>".into()))
        );
        assert_eq!(
            nested_kind.data.get("found_type"),
            Some(&DataValue::String("Array<Result<I64,Bool>>".into()))
        );
        let same = checker
            .unify_independent(
                option_i64.clone(),
                option_i64.clone(),
                site(4),
                site(5),
                site(6),
                false,
            )
            .unwrap();
        assert_eq!(same, option_i64);
        let nested = checker
            .unify_independent(
                Type::option(Arc::new(Type::option(Arc::new(Type::I64)))),
                Type::option(Arc::new(Type::option(Arc::new(Type::Bool)))),
                site(7),
                site(8),
                site(9),
                false,
            )
            .unwrap_err();
        assert_eq!(nested.code, "NX-E0102");
        assert!(nested.related_spans.iter().any(|related| {
            related.role == RelatedRole::ExpectedType && related.span == site(7)
        }));
    }

    #[test]
    fn i07c_survivor3_shape_requirements_reject_incompatible_roots() {
        let resolved = Resolved {
            signatures: HashMap::new(),
            constants: HashMap::new(),
            uses: HashMap::new(),
            calls: HashMap::new(),
        };
        let mut checker = empty_checker(&resolved);
        let array = checker.new_variable(&site(1)).unwrap();
        checker.require_array(array, &site(2)).unwrap();
        assert_eq!(
            checker
                .require_union(array, UnionKind::Option, &site(3))
                .unwrap_err()
                .code,
            "NX-E0103"
        );
        let scalar = checker.new_variable(&site(4)).unwrap();
        checker
            .bind_independent(scalar, Type::Bool, site(5), site(5), false)
            .unwrap();
        assert_eq!(
            checker
                .require_union(scalar, UnionKind::Result, &site(6))
                .unwrap_err()
                .code,
            "NX-E0103"
        );
        let option = checker.new_variable(&site(7)).unwrap();
        checker
            .require_union(option, UnionKind::Option, &site(8))
            .unwrap();
        assert_eq!(
            checker
                .bind_independent(option, Type::I64, site(9), site(9), false)
                .unwrap_err()
                .code,
            "NX-E0103"
        );
    }

    #[test]
    fn i07c_survivor3_failed_single_sided_union_keeps_preexisting_shape() {
        let resolved = Resolved {
            signatures: HashMap::new(),
            constants: HashMap::new(),
            uses: HashMap::new(),
            calls: HashMap::new(),
        };
        let mut checker = empty_checker(&resolved);
        let shaped = checker.new_variable(&site(1)).unwrap();
        let plain = checker.new_variable(&site(2)).unwrap();
        let child = checker.new_variable(&site(3)).unwrap();
        checker
            .graph
            .type_nodes
            .get_mut(&shaped)
            .unwrap()
            .variant_children = Some(UnionDescriptor::option(child));
        checker
            .bind_independent(child, Type::Bool, site(4), site(4), false)
            .unwrap();
        checker.graph.type_nodes.get_mut(&plain).unwrap().concrete =
            Some((Type::option(Arc::new(Type::I64)), site(5)));
        let error = checker.union(shaped, plain, site(6)).unwrap_err();
        assert_eq!(error.code, "NX-E0102");
        assert_eq!(checker.root(shaped), shaped);
        assert_eq!(checker.root(plain), plain);
        assert_eq!(checker.concrete(child).map(|(ty, _)| ty), Some(Type::Bool));
    }

    #[test]
    fn i07c_survivor3_failed_result_union_rolls_back_earlier_payload_bind() {
        let resolved = Resolved {
            signatures: HashMap::new(),
            constants: HashMap::new(),
            uses: HashMap::new(),
            calls: HashMap::new(),
        };
        let mut checker = empty_checker(&resolved);
        let shaped = checker.new_variable(&site(1)).unwrap();
        let plain = checker.new_variable(&site(2)).unwrap();
        let first = checker.new_variable(&site(3)).unwrap();
        let second = checker.new_variable(&site(4)).unwrap();
        checker
            .graph
            .type_nodes
            .get_mut(&shaped)
            .unwrap()
            .variant_children = Some(UnionDescriptor::result(first, second));
        checker
            .bind_independent(second, Type::I64, site(5), site(5), false)
            .unwrap();
        checker.graph.type_nodes.get_mut(&plain).unwrap().concrete = Some((
            Type::result(Arc::new(Type::I64), Arc::new(Type::Bool)),
            site(6),
        ));
        assert_eq!(
            checker.union(shaped, plain, site(7)).unwrap_err().code,
            "NX-E0102"
        );
        assert!(checker.concrete(first).is_none());
        assert_eq!(checker.concrete(second).map(|(ty, _)| ty), Some(Type::I64));
        assert_eq!(checker.root(shaped), shaped);
        assert_eq!(checker.root(plain), plain);
    }

    #[test]
    fn i07c_survivor3_union_prefers_larger_component_before_id() {
        let resolved = Resolved {
            signatures: HashMap::new(),
            constants: HashMap::new(),
            uses: HashMap::new(),
            calls: HashMap::new(),
        };
        let mut checker = empty_checker(&resolved);
        let smaller_id = checker.new_variable(&site(1)).unwrap();
        let larger_id = checker.new_variable(&site(2)).unwrap();
        checker.graph.type_nodes.get_mut(&larger_id).unwrap().size = 3;
        checker.union(smaller_id, larger_id, site(3)).unwrap();
        assert_eq!(checker.root(smaller_id), larger_id);
        assert_eq!(checker.graph.type_nodes[&larger_id].size, 4);
    }

    #[test]
    fn i07c_survivor3_never_payload_keeps_later_result_bind() {
        let resolved = Resolved {
            signatures: HashMap::new(),
            constants: HashMap::new(),
            uses: HashMap::new(),
            calls: HashMap::new(),
        };
        let mut checker = empty_checker(&resolved);
        let root = checker.new_variable(&site(1)).unwrap();
        let children = checker
            .require_union(root, UnionKind::Result, &site(2))
            .unwrap();
        checker
            .bind_independent(
                root,
                Type::result(Arc::new(Type::Never), Arc::new(Type::I64)),
                site(3),
                site(3),
                false,
            )
            .unwrap();
        let payloads: Vec<_> = children.variants.iter().filter_map(|v| v.payload).collect();
        assert_eq!(
            checker.concrete(payloads[1]).map(|(ty, _)| ty),
            Some(Type::I64)
        );
    }

    #[test]
    fn i07c_survivor3_cached_array_child_rebuilds_outer_shape() {
        let resolved = Resolved {
            signatures: HashMap::new(),
            constants: HashMap::new(),
            uses: HashMap::new(),
            calls: HashMap::new(),
        };
        let mut checker = empty_checker(&resolved);
        let left = Type::Array(Arc::new(Type::I64));
        let right = Type::Array(Arc::new(Type::I64));
        checker
            .unify_independent(
                left.clone(),
                right.clone(),
                site(1),
                site(2),
                site(3),
                false,
            )
            .unwrap();
        let left_outer = Type::Array(Arc::new(left));
        let right_outer = Type::Array(Arc::new(right));
        let joined = checker
            .unify_independent(
                left_outer.clone(),
                right_outer,
                site(4),
                site(5),
                site(6),
                false,
            )
            .unwrap();
        assert_eq!(joined, left_outer);
    }

    #[test]
    fn i07c_survivor3_failed_atomic_union_preserves_quota_error() {
        let resolved = Resolved {
            signatures: HashMap::new(),
            constants: HashMap::new(),
            uses: HashMap::new(),
            calls: HashMap::new(),
        };
        let mut checker = empty_checker(&resolved);
        let shaped = checker.new_variable(&site(1)).unwrap();
        let plain = checker.new_variable(&site(2)).unwrap();
        let payload = checker.new_variable(&site(3)).unwrap();
        checker
            .graph
            .type_nodes
            .get_mut(&shaped)
            .unwrap()
            .variant_children = Some(UnionDescriptor::option(payload));
        checker.solver_limit = checker.solver_steps;
        let error = checker.union(shaped, plain, site(4)).unwrap_err();
        assert_eq!(error.operation, Operation::AnalysisLimit);
        assert!(checker.quota_error.is_some());
        assert_eq!(checker.solver_steps, checker.solver_limit);
        assert_eq!(checker.graph.active_count(), 0);
    }

    #[test]
    fn i07c_survivor3_completed_option_uses_binding_origin() {
        let resolved = Resolved {
            signatures: HashMap::new(),
            constants: HashMap::new(),
            uses: HashMap::new(),
            calls: HashMap::new(),
        };
        let mut checker = empty_checker(&resolved);
        let root = checker.new_variable(&site(1)).unwrap();
        checker
            .require_union(root, UnionKind::Option, &site(2))
            .unwrap();
        checker
            .bind_independent(
                root,
                Type::option(Arc::new(Type::I64)),
                site(3),
                site(4),
                false,
            )
            .unwrap();
        assert_eq!(
            checker.graph.type_nodes[&root].variant_origin,
            Some(site(3))
        );
        assert_eq!(
            checker.concrete(root).map(|(ty, _)| ty),
            Some(Type::option(Arc::new(Type::I64)))
        );
    }

    #[test]
    fn i07c_survivor3_failed_result_payload_restores_earlier_deferred_bind() {
        let resolved = Resolved {
            signatures: HashMap::new(),
            constants: HashMap::new(),
            uses: HashMap::new(),
            calls: HashMap::new(),
        };
        let mut checker = empty_checker(&resolved);
        let pending = checker.new_variable(&site(1)).unwrap();
        let left = Type::result(Arc::new(Type::Deferred(pending)), Arc::new(Type::Bool));
        let right = Type::result(Arc::new(Type::I64), Arc::new(Type::I64));
        assert_eq!(
            checker
                .unify_independent(left, right, site(2), site(3), site(4), false)
                .unwrap_err()
                .code,
            "NX-E0102"
        );
        assert!(checker.concrete(pending).is_none());
    }

    #[test]
    fn i07c_survivor3_union_rejects_variant_against_array_in_both_orders() {
        let resolved = Resolved {
            signatures: HashMap::new(),
            constants: HashMap::new(),
            uses: HashMap::new(),
            calls: HashMap::new(),
        };
        for reversed in [false, true] {
            let mut checker = empty_checker(&resolved);
            let variant = checker.new_variable(&site(1)).unwrap();
            let array = checker.new_variable(&site(2)).unwrap();
            checker
                .require_union(variant, UnionKind::Option, &site(3))
                .unwrap();
            checker.require_array(array, &site(4)).unwrap();
            let (left, right) = if reversed {
                (array, variant)
            } else {
                (variant, array)
            };
            assert_eq!(
                checker.union(left, right, site(5)).unwrap_err().code,
                "NX-E0103"
            );
            assert_eq!(checker.root(variant), variant);
            assert_eq!(checker.root(array), array);
        }
    }

    #[test]
    fn i07c_survivor3_require_union_quota_propagates_through_bind() {
        let resolved = Resolved {
            signatures: HashMap::new(),
            constants: HashMap::new(),
            uses: HashMap::new(),
            calls: HashMap::new(),
        };
        let mut checker = empty_checker(&resolved);
        let root = checker.new_variable(&site(1)).unwrap();
        checker
            .require_union(root, UnionKind::Option, &site(2))
            .unwrap();
        checker.solver_limit = checker.solver_steps;
        let error = checker
            .bind_independent(
                root,
                Type::option(Arc::new(Type::I64)),
                site(3),
                site(4),
                false,
            )
            .unwrap_err();
        assert_eq!(error.operation, Operation::AnalysisLimit);
        assert!(checker.quota_error.is_some());
    }

    #[test]
    fn i07c_survivor3_rebinding_known_union_keeps_first_shape_origin() {
        let resolved = Resolved {
            signatures: HashMap::new(),
            constants: HashMap::new(),
            uses: HashMap::new(),
            calls: HashMap::new(),
        };
        let mut checker = empty_checker(&resolved);
        let root = checker.new_variable(&site(1)).unwrap();
        let ty = Type::option(Arc::new(Type::I64));
        checker.graph.type_nodes.get_mut(&root).unwrap().concrete = Some((ty.clone(), site(2)));
        checker
            .require_union(root, UnionKind::Option, &site(3))
            .unwrap();
        checker
            .bind_independent(root, ty, site(4), site(5), false)
            .unwrap();
        assert_eq!(
            checker.graph.type_nodes[&root].variant_origin,
            Some(site(3))
        );
    }

    #[test]
    fn i07c_survivor3_failed_result_bind_restores_first_payload() {
        let resolved = Resolved {
            signatures: HashMap::new(),
            constants: HashMap::new(),
            uses: HashMap::new(),
            calls: HashMap::new(),
        };
        let mut checker = empty_checker(&resolved);
        let root = checker.new_variable(&site(1)).unwrap();
        let children = checker
            .require_union(root, UnionKind::Result, &site(2))
            .unwrap();
        let payloads: Vec<_> = children.variants.iter().filter_map(|v| v.payload).collect();
        checker
            .require_union(payloads[1], UnionKind::Option, &site(3))
            .unwrap();
        assert_eq!(
            checker
                .bind_independent(
                    root,
                    Type::result(Arc::new(Type::I64), Arc::new(Type::Bool)),
                    site(4),
                    site(5),
                    false,
                )
                .unwrap_err()
                .code,
            "NX-E0103"
        );
        assert!(checker.concrete(payloads[0]).is_none());
    }

    #[test]
    fn i07c_survivor3_noncopyable_second_payload_restores_first_bind() {
        let resolved = Resolved {
            signatures: HashMap::new(),
            constants: HashMap::new(),
            uses: HashMap::new(),
            calls: HashMap::new(),
        };
        let mut checker = empty_checker(&resolved);
        let root = checker.new_variable(&site(1)).unwrap();
        let children = checker
            .require_union(root, UnionKind::Result, &site(2))
            .unwrap();
        let payloads: Vec<_> = children.variants.iter().filter_map(|v| v.payload).collect();
        checker
            .graph
            .type_nodes
            .get_mut(&payloads[1])
            .unwrap()
            .copyable_origin = Some(site(3));
        assert_eq!(
            checker
                .bind_independent(
                    root,
                    Type::result(Arc::new(Type::I64), Arc::new(Type::MockResource)),
                    site(4),
                    site(5),
                    false,
                )
                .unwrap_err()
                .code,
            "NX-E0102"
        );
        assert!(checker.concrete(payloads[0]).is_none());
    }

    #[test]
    fn i07c_survivor3_equal_start_concrete_origins_choose_left_diagnostic_span() {
        let resolved = Resolved {
            signatures: HashMap::new(),
            constants: HashMap::new(),
            uses: HashMap::new(),
            calls: HashMap::new(),
        };
        let mut checker = empty_checker(&resolved);
        let left = checker.new_variable(&site(1)).unwrap();
        let right = checker.new_variable(&site(2)).unwrap();
        let first = Span {
            file: "quota.n".into(),
            start: 10,
            end: 11,
        };
        let second = Span {
            file: "quota.n".into(),
            start: 10,
            end: 12,
        };
        checker.graph.type_nodes.get_mut(&left).unwrap().concrete =
            Some((Type::I64, first.clone()));
        checker.graph.type_nodes.get_mut(&right).unwrap().concrete = Some((Type::I64, second));
        checker.union(left, right, site(13)).unwrap();
        let error = checker
            .bind_independent(left, Type::Bool, site(14), site(15), false)
            .unwrap_err();
        assert_eq!(error.code, "NX-E0102");
        assert!(
            error
                .related_spans
                .iter()
                .any(|r| r.role == RelatedRole::ExpectedType && r.span == first)
        );
    }

    #[test]
    fn i07c_survivor3_incompatible_deferred_shapes_defer_without_recursion() {
        let resolved = Resolved {
            signatures: HashMap::new(),
            constants: HashMap::new(),
            uses: HashMap::new(),
            calls: HashMap::new(),
        };
        let mut checker = empty_checker(&resolved);
        let variant = checker.new_variable(&site(1)).unwrap();
        let array = checker.new_variable(&site(2)).unwrap();
        checker
            .require_union(variant, UnionKind::Option, &site(3))
            .unwrap();
        checker.require_array(array, &site(4)).unwrap();
        let joined = checker
            .unify_independent(
                Type::Deferred(variant),
                Type::Deferred(array),
                site(5),
                site(6),
                site(7),
                false,
            )
            .unwrap();
        assert_eq!(joined, Type::Deferred(variant));
        assert_eq!(checker.root(variant), variant);
        assert_eq!(checker.root(array), array);
        assert!(!checker.graph.type_nodes[&variant].recursive_join);
        assert_eq!(checker.pending_join_count(), 1);
    }

    #[test]
    fn i07c_survivor3_union_child_allocation_propagates_quota_errors() {
        let resolved = Resolved {
            signatures: HashMap::new(),
            constants: HashMap::new(),
            uses: HashMap::new(),
            calls: HashMap::new(),
        };
        for (kind, allowance) in [(UnionKind::Option, 1), (UnionKind::Result, 2)] {
            let mut checker = empty_checker(&resolved);
            let root = checker.new_variable(&site(1)).unwrap();
            checker.solver_limit = checker.solver_steps + allowance;
            let error = checker.require_union(root, kind, &site(2)).unwrap_err();
            assert_eq!(error.operation, Operation::AnalysisLimit);
            assert!(checker.quota_error.is_some());
            assert!(checker.graph.type_nodes[&root].variant_children.is_none());
        }
    }

    #[test]
    fn i07c_survivor3_late_result_payload_quota_rolls_back_first_bind() {
        let resolved = Resolved {
            signatures: HashMap::new(),
            constants: HashMap::new(),
            uses: HashMap::new(),
            calls: HashMap::new(),
        };
        let mut checker = empty_checker(&resolved);
        let pending = checker.new_variable(&site(1)).unwrap();
        checker.solver_limit = checker.solver_steps + 3;
        let left = Type::result(Arc::new(Type::Deferred(pending)), Arc::new(Type::Bool));
        let right = Type::result(Arc::new(Type::I64), Arc::new(Type::Bool));
        let error = checker
            .unify_independent(left, right, site(2), site(3), site(4), false)
            .unwrap_err();
        assert_eq!(error.operation, Operation::AnalysisLimit);
        assert!(checker.concrete(pending).is_none());
        assert!(checker.quota_error.is_some());
    }

    #[test]
    fn i07c_survivor3_partial_union_allocation_rolls_back_on_quota() {
        let resolved = Resolved {
            signatures: HashMap::new(),
            constants: HashMap::new(),
            uses: HashMap::new(),
            calls: HashMap::new(),
        };
        let mut checker = empty_checker(&resolved);
        let root = checker.new_variable(&site(1)).unwrap();
        let pending = checker.new_variable(&site(2)).unwrap();
        let before_nodes = checker.graph.next_variable;
        checker.solver_limit = checker.solver_steps + 2;
        let error = checker
            .bind_independent(
                root,
                Type::result(Arc::new(Type::Deferred(pending)), Arc::new(Type::I64)),
                site(3),
                site(4),
                false,
            )
            .unwrap_err();
        assert_eq!(error.operation, Operation::AnalysisLimit);
        assert_eq!(checker.graph.next_variable, before_nodes);
        assert!(checker.graph.type_nodes[&root].variant_children.is_none());
        assert!(checker.quota_error.is_some());
    }

    #[test]
    fn i07c_survivor3_equal_start_array_origins_choose_left_diagnostic_span() {
        let resolved = Resolved {
            signatures: HashMap::new(),
            constants: HashMap::new(),
            uses: HashMap::new(),
            calls: HashMap::new(),
        };
        let mut checker = empty_checker(&resolved);
        let left = checker.new_variable(&site(1)).unwrap();
        let right = checker.new_variable(&site(2)).unwrap();
        let first = Span {
            file: "quota.n".into(),
            start: 10,
            end: 11,
        };
        let second = Span {
            file: "quota.n".into(),
            start: 10,
            end: 12,
        };
        checker.require_array(left, &first).unwrap();
        checker.require_array(right, &second).unwrap();
        checker.union(left, right, site(3)).unwrap();
        let root = checker.root(left);
        let error = checker.unresolved_constructor_diagnostic(root, site(4));
        assert_eq!(error.code, "NX-E0103");
        assert_eq!(error.primary_span, Some(first));
    }

    #[test]
    fn i07c_survivor3_failed_concrete_union_expansion_restores_root() {
        let resolved = Resolved {
            signatures: HashMap::new(),
            constants: HashMap::new(),
            uses: HashMap::new(),
            calls: HashMap::new(),
        };
        let mut checker = empty_checker(&resolved);
        let root = checker.new_variable(&site(1)).unwrap();
        let pending = checker.new_variable(&site(2)).unwrap();
        let concrete = Type::result(Arc::new(Type::I64), Arc::new(Type::Bool));
        checker.graph.type_nodes.get_mut(&root).unwrap().concrete =
            Some((concrete.clone(), site(3)));
        checker.solver_limit = checker.solver_steps + 4;
        let error = checker
            .bind_independent(
                root,
                Type::result(Arc::new(Type::Deferred(pending)), Arc::new(Type::Bool)),
                site(4),
                site(5),
                false,
            )
            .unwrap_err();
        assert_eq!(error.operation, Operation::AnalysisLimit);
        assert_eq!(
            checker.graph.type_nodes[&root].concrete,
            Some((concrete, site(3)))
        );
        assert!(checker.graph.type_nodes[&root].variant_children.is_none());
    }

    #[test]
    fn i07c_survivor3_deferred_bind_rewrites_pending_join_to_source_operands() {
        let resolved = Resolved {
            signatures: HashMap::new(),
            constants: HashMap::new(),
            uses: HashMap::new(),
            calls: HashMap::new(),
        };
        let mut checker = empty_checker(&resolved);
        let root = checker.new_variable(&site(1)).unwrap();
        let child = checker.require_array(root, &site(2)).unwrap();
        checker.graph.preseeded_children.insert(child);
        let joined = checker
            .unify_independent(
                Type::Deferred(root),
                Type::I64,
                site(3),
                site(4),
                site(5),
                false,
            )
            .unwrap();
        assert_eq!(joined, Type::Deferred(root));
        let pending = checker.pending_joins().next().unwrap();
        assert_eq!(pending.left, Type::Deferred(root));
        assert_eq!(pending.right, Type::I64);
        assert_eq!(pending.left_origin, site(3));
        assert!(!pending.branch);
        assert!(pending.related_expected);
    }

    #[test]
    fn i07c_survivor3_deferred_union_rewrites_child_join_to_parent_operands() {
        let resolved = Resolved {
            signatures: HashMap::new(),
            constants: HashMap::new(),
            uses: HashMap::new(),
            calls: HashMap::new(),
        };
        let mut checker = empty_checker(&resolved);
        let shaped = checker.new_variable(&site(1)).unwrap();
        let concrete = checker.new_variable(&site(2)).unwrap();
        let child = checker.require_array(shaped, &site(3)).unwrap();
        let grandchild = checker.require_array(child, &site(4)).unwrap();
        checker.graph.preseeded_children.insert(grandchild);
        checker
            .graph
            .type_nodes
            .get_mut(&concrete)
            .unwrap()
            .concrete = Some((Type::Array(Arc::new(Type::I64)), site(5)));
        let joined = checker
            .unify_with_expected_origin_independent(
                Type::Deferred(shaped),
                Type::Deferred(concrete),
                site(6),
                site(7),
                site(8),
                UnifyPolicy {
                    branch: true,
                    related_expected: false,
                },
            )
            .unwrap();
        assert_eq!(joined, Type::Deferred(shaped));
        let pending = checker.pending_joins().next().unwrap();
        assert_eq!(pending.left, Type::Deferred(shaped));
        assert_eq!(pending.right, Type::Deferred(concrete));
        assert_eq!(pending.left_origin, site(6));
        assert!(pending.branch);
        assert!(!pending.related_expected);
    }

    #[test]
    fn i07c_survivor3_partial_result_conflict_defers_join() {
        let resolved = Resolved {
            signatures: HashMap::new(),
            constants: HashMap::new(),
            uses: HashMap::new(),
            calls: HashMap::new(),
        };
        let mut checker = empty_checker(&resolved);
        let partial = checker.new_variable(&site(1)).unwrap();
        let concrete = checker.new_variable(&site(2)).unwrap();
        let children = checker
            .require_union(partial, UnionKind::Result, &site(3))
            .unwrap();
        let payloads: Vec<_> = children.variants.iter().filter_map(|v| v.payload).collect();
        checker
            .bind_independent(payloads[1], Type::Bool, site(4), site(4), false)
            .unwrap();
        checker
            .graph
            .type_nodes
            .get_mut(&concrete)
            .unwrap()
            .concrete = Some((
            Type::result(Arc::new(Type::I64), Arc::new(Type::I64)),
            site(5),
        ));
        let joined = checker
            .unify_independent(
                Type::Deferred(partial),
                Type::Deferred(concrete),
                site(6),
                site(7),
                site(8),
                false,
            )
            .unwrap();
        assert_eq!(joined, Type::Deferred(partial));
        assert!(checker.concrete(payloads[0]).is_none());
        assert_eq!(checker.pending_join_count(), 1);
    }

    #[test]
    fn i07c_survivor3_array_join_wraps_pending_operand_shapes() {
        let resolved = Resolved {
            signatures: HashMap::new(),
            constants: HashMap::new(),
            uses: HashMap::new(),
            calls: HashMap::new(),
        };
        let mut checker = empty_checker(&resolved);
        let root = checker.new_variable(&site(1)).unwrap();
        let child = checker.require_array(root, &site(2)).unwrap();
        checker.graph.preseeded_children.insert(child);
        let left = Type::Array(Arc::new(Type::Deferred(root)));
        let right = Type::Array(Arc::new(Type::I64));
        let joined = checker
            .unify_independent(
                left.clone(),
                right.clone(),
                site(3),
                site(4),
                site(5),
                false,
            )
            .unwrap();
        assert_eq!(joined, left);
        let pending = checker.pending_joins().next().unwrap();
        assert_eq!(pending.left, Type::Array(Arc::new(Type::Deferred(root))));
        assert_eq!(pending.right, right);
    }

    #[test]
    fn i07c_survivor3_nested_partial_result_deferral_keeps_array_shape() {
        let resolved = Resolved {
            signatures: HashMap::new(),
            constants: HashMap::new(),
            uses: HashMap::new(),
            calls: HashMap::new(),
        };
        for deferred_right in [false, true] {
            let mut checker = empty_checker(&resolved);
            let partial = checker.new_variable(&site(1)).unwrap();
            let other = checker.new_variable(&site(2)).unwrap();
            let children = checker
                .require_union(partial, UnionKind::Result, &site(3))
                .unwrap();
            let payloads: Vec<_> = children.variants.iter().filter_map(|v| v.payload).collect();
            checker
                .bind_independent(payloads[1], Type::Bool, site(4), site(4), false)
                .unwrap();
            let concrete = Type::result(Arc::new(Type::I64), Arc::new(Type::I64));
            if deferred_right {
                checker.graph.type_nodes.get_mut(&other).unwrap().concrete =
                    Some((concrete.clone(), site(5)));
            }
            let right = if deferred_right {
                Type::Deferred(other)
            } else {
                concrete
            };
            let joined = checker
                .unify_independent(
                    Type::Array(Arc::new(Type::Deferred(partial))),
                    Type::Array(Arc::new(right)),
                    site(6),
                    site(7),
                    site(8),
                    false,
                )
                .unwrap();
            assert_eq!(joined, Type::Array(Arc::new(Type::Deferred(partial))));
            assert_eq!(checker.pending_join_count(), 1);
        }
    }

    #[test]
    fn i07c_survivor4_deferred_bind_keeps_branch_policy() {
        let resolved = Resolved {
            signatures: HashMap::new(),
            constants: HashMap::new(),
            uses: HashMap::new(),
            calls: HashMap::new(),
        };
        let mut checker = empty_checker(&resolved);
        let root = checker.new_variable(&site(1)).unwrap();
        let child = checker.require_array(root, &site(2)).unwrap();
        checker.graph.preseeded_children.insert(child);
        let joined = checker
            .unify_with_expected_origin_independent(
                Type::Deferred(root),
                Type::I64,
                site(3),
                site(4),
                site(5),
                UnifyPolicy {
                    branch: true,
                    related_expected: false,
                },
            )
            .unwrap();
        assert_eq!(joined, Type::Deferred(root));
        let pending = checker.pending_joins().next().unwrap();
        assert_eq!(pending.left, Type::Deferred(root));
        assert_eq!(pending.right, Type::I64);
        assert_eq!(pending.left_origin, site(3));
        assert!(pending.branch);
        assert!(!pending.related_expected);
    }

    #[test]
    fn i07c_survivor4_invalid_control_condition_taints_visit() {
        for (source, condition) in [("fn Main(){if 1{}}", "1"), ("fn Main(){while 1{}}", "1")] {
            let ast = parse::parse_new_profile("control.n", source).unwrap();
            let resolved = names::resolve_with_option("control.n", source, &ast, true).unwrap();
            let mut checker = empty_checker(&resolved);
            let (_, visit) = checker.drive(DriveStart::Block(&ast.functions[0].body));
            assert!(visit, "{source}");
            assert_eq!(checker.errors.len(), 1, "{source}");
            let error = &checker.errors[0];
            assert_eq!(error.code, "NX-E0102");
            assert_eq!(error.subcause.as_deref(), Some("type_mismatch"));
            let span = error.primary_span.as_ref().unwrap();
            assert_eq!(&source[span.start as usize..span.end as usize], condition);
        }
        for source in ["fn Main(){if true{}}", "fn Main(){while false{}}"] {
            let ast = parse::parse_new_profile("control.n", source).unwrap();
            let resolved = names::resolve_with_option("control.n", source, &ast, true).unwrap();
            let mut checker = empty_checker(&resolved);
            let (_, visit) = checker.drive(DriveStart::Block(&ast.functions[0].body));
            assert!(!visit, "{source}");
            assert!(checker.errors.is_empty(), "{source}");
        }
    }
}
