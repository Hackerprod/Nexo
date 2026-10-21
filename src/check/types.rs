//! Independent type checking for the accepted non-control core of F0-0002.
use std::collections::{HashMap, HashSet};

use super::names::{self, Resolved, ResolvedUse};
use crate::diagnostic::{DataValue, Diagnostic, Operation, RelatedRole, RelatedSpan, Span, Stage};
use crate::frontend::{
    lex::{self, Kind, Token},
    parse::{Ast, Block, Else, Expr, Function, Mode, Statement, Type},
};

fn pretty(ty: &Type) -> String {
    enum Part<'a> {
        Type(&'a Type),
        Text(&'static str),
    }
    let mut output = String::new();
    let mut work = vec![Part::Type(ty)];
    while let Some(part) = work.pop() {
        match part {
            Part::Text(text) => output.push_str(text),
            Part::Type(Type::I64) => output.push_str("i64"),
            Part::Type(Type::Bool) => output.push_str("bool"),
            Part::Type(Type::Unit) => output.push_str("()"),
            Part::Type(Type::MockResource) => output.push_str("MockResource"),
            Part::Type(Type::MockBlockingResource) => output.push_str("MockBlockingResource"),
            Part::Type(Type::Array(inner)) => {
                work.push(Part::Text(">"));
                work.push(Part::Type(inner));
                work.push(Part::Text("Array<"));
            }
            Part::Type(Type::Result(ok, err)) => {
                work.push(Part::Text(">"));
                work.push(Part::Type(err));
                work.push(Part::Text(","));
                work.push(Part::Type(ok));
                work.push(Part::Text("Result<"));
            }
        }
    }
    output
}

fn copyable(ty: &Type) -> bool {
    let mut work = vec![ty];
    while let Some(node) = work.pop() {
        match node {
            Type::I64 | Type::Bool | Type::Unit => {}
            Type::Array(inner) => work.push(inner),
            Type::Result(ok, err) => {
                work.push(err);
                work.push(ok);
            }
            Type::MockResource | Type::MockBlockingResource => return false,
        }
    }
    true
}

fn contains_resource(ty: &Type) -> bool {
    let mut work = vec![ty];
    while let Some(node) = work.pop() {
        match node {
            Type::MockResource | Type::MockBlockingResource => return true,
            Type::Array(inner) => work.push(inner),
            Type::Result(ok, err) => {
                work.push(err);
                work.push(ok);
            }
            Type::I64 | Type::Bool | Type::Unit => {}
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
            Type::Result(ok, err) => {
                work.push(err);
                work.push(ok);
            }
            Type::I64
            | Type::Bool
            | Type::Unit
            | Type::MockResource
            | Type::MockBlockingResource => {}
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
        .insert("found_type".into(), DataValue::String(pretty(found)));
    if let Some(span) = expected_span {
        d.related_spans.push(RelatedSpan {
            role: RelatedRole::ExpectedType,
            span,
        });
    }
    d.related_spans.push(RelatedSpan {
        role: RelatedRole::FoundType,
        span: found_span,
    });
    names::sort_related(&mut d);
    d
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
    d.subcause = Some(subcause.into());
    d.rule = format!("NX-E0102/{subcause}");
    d.explanation_id = Some(d.rule.clone());
    d
}

#[derive(Clone)]
struct Known {
    ty: Type,
    origin: Span,
    /// This binding has no type yet: a diverging branch or deferred `?`.
    deferred: bool,
}

// Only the syntactic, unconditional returns needed to exclude a branch from
// the value union are recognized here. General reachability belongs to I07.
fn block_returns(block: &Block) -> bool {
    block.statements.last().is_some_and(statement_returns)
}

fn branch_value_span(block: &Block) -> Option<Span> {
    block.statements.iter().find_map(|statement| {
        if let Statement::Value {
            value: Some(value), ..
        } = statement
        {
            Some(value.span().clone())
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
    let mut d = names::diagnostic(
        "NX-E1303",
        None,
        Stage::Types,
        Operation::TypesCheck,
        found_span.clone(),
    );
    d.data
        .insert("expected_type".into(), DataValue::String(pretty(expected)));
    d.data
        .insert("found_type".into(), DataValue::String(pretty(found)));
    d.related_spans.push(RelatedSpan {
        role: RelatedRole::ExpectedType,
        span: expected_span.clone(),
    });
    d.related_spans.push(RelatedSpan {
        role: RelatedRole::FoundType,
        span: found_span,
    });
    names::sort_related(&mut d);
    d
}

fn statement_returns(statement: &Statement) -> bool {
    match statement {
        Statement::Return { .. } => true,
        Statement::Block(block) => block_returns(block),
        Statement::If {
            then_block,
            otherwise: Some(other),
            ..
        } => block_returns(then_block) && statement_returns(other),
        Statement::Expr { value, .. } => expr_returns(value),
        Statement::Value {
            value: Some(value), ..
        } => expr_returns(value),
        _ => false,
    }
}

fn expr_returns(expr: &Expr) -> bool {
    match expr {
        Expr::Group(inner, _) => expr_returns(inner),
        Expr::If {
            then_block,
            otherwise: Some(other),
            ..
        } => {
            block_returns(then_block)
                && match &**other {
                    Else::Block(block) => block_returns(block),
                    Else::If(expr) => expr_returns(expr),
                }
        }
        _ => false,
    }
}

#[allow(clippy::vec_box)] // Public semantic result keeps boxed diagnostics.
struct Checker<'a> {
    file: &'a str,
    tokens: Vec<Token>,
    resolved: &'a Resolved,
    locals: HashMap<u64, Known>,
    invalid: HashSet<u64>,
    deferred_exprs: HashSet<u64>,
    tainted: bool,
    result: Type,
    result_span: Option<Span>,
    depth: usize,
    errors: Vec<Box<Diagnostic>>,
}

impl Checker<'_> {
    fn enter(&mut self, span: &Span) -> Result<(), Box<Diagnostic>> {
        if self.depth >= 256 {
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
            d.data.insert("limit".into(), DataValue::UInt(1_000_000));
            d.data.insert("observed".into(), DataValue::UInt(1_000_001));
            return Err(d);
        }
        self.depth += 1;
        Ok(())
    }

    fn check_depth(&self, ty: &Type, origin: &Span) -> Result<(), Box<Diagnostic>> {
        if has_resource_array(ty) {
            return Err(resource_array(origin.clone()));
        }
        let mut arrays = 0;
        let mut values = 0;
        let mut work = vec![(ty, 0usize, 0usize)];
        while let Some((node, a, v)) = work.pop() {
            arrays = arrays.max(a);
            values = values.max(v);
            match node {
                Type::Array(inner) => work.push((inner, a + 1, v + 1)),
                Type::Result(ok, err) => {
                    work.push((err, a, v + 1));
                    work.push((ok, a, v + 1));
                }
                Type::I64
                | Type::Bool
                | Type::Unit
                | Type::MockResource
                | Type::MockBlockingResource => {}
            }
        }
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
        d.data
            .insert("observed".into(), DataValue::UInt(observed as u64));
        Err(d)
    }

    fn type_annotation_span(&self, start: u64, end: u64, marker: &str) -> Option<Span> {
        let first_at = self.tokens.partition_point(|t| t.span.start < start);
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
        self.locals.clear();
        self.invalid.clear();
        self.tainted = false;
        self.result = f.result.clone().unwrap_or(Type::Unit);
        self.result_span = f
            .result
            .as_ref()
            .and_then(|_| self.type_annotation_span(f.name_span.end, f.body.span.start, "->"));
        if let Some(span) = &self.result_span {
            self.check_depth(&self.result, span)?;
        }
        let sig = &self.resolved.signatures[&f.name];
        for p in &sig.params {
            self.check_depth(&p.ty, &p.type_span)?;
            self.locals.insert(
                p.name_span.start,
                Known {
                    ty: p.ty.clone(),
                    origin: p.type_span.clone(),
                    deferred: false,
                },
            );
        }
        self.block(&f.body)
    }

    fn block(&mut self, b: &Block) -> Result<(), Box<Diagnostic>> {
        self.enter(&b.span)?;
        for statement in &b.statements {
            self.statement_at_site(statement);
        }
        self.depth -= 1;
        Ok(())
    }

    fn statement_at_site(&mut self, statement: &Statement) {
        let previous_taint = self.tainted;
        self.tainted = false;
        let outcome = self.statement(statement);
        let tainted = self.tainted;
        self.tainted = previous_taint || tainted;
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
            let origin = self
                .type_annotation_span(span.start, value.span().start, ":")
                .unwrap_or_else(|| value.span().clone());
            self.locals.insert(
                name_span.start,
                Known {
                    ty: annotation.clone().unwrap_or(Type::Unit),
                    origin,
                    deferred: false,
                },
            );
            self.invalid.insert(name_span.start);
        }
        if let Err(error) = outcome
            && !tainted
        {
            self.errors.push(error);
        }
    }

    fn statement(&mut self, s: &Statement) -> Result<(), Box<Diagnostic>> {
        match s {
            Statement::Let {
                name,
                annotation,
                value,
                span,
                ..
            } => {
                let ann_span = annotation
                    .as_ref()
                    .and_then(|_| self.type_annotation_span(span.start, value.span().start, ":"));
                let got = self.expr(value, annotation.as_ref(), ann_span.as_ref())?;
                let ns = names::token_span(&self.tokens, span.start, value.span().start, name)
                    .unwrap_or_else(|| span.clone());
                let origin = ann_span.unwrap_or_else(|| value.span().clone());
                self.check_depth(&got, &origin)?;
                self.locals.insert(
                    ns.start,
                    Known {
                        ty: got,
                        origin,
                        deferred: self.deferred_exprs.contains(&value.span().start),
                    },
                );
                Ok(())
            }
            Statement::Assign { place, value, .. } => {
                let lhs = self.expr(place, None, None)?;
                let origin = if let Expr::Name(_, s) = place {
                    if let Some(ResolvedUse::Local(local)) = self.resolved.uses.get(&s.start) {
                        self.locals
                            .get(&local.name_span.start)
                            .map(|known| known.origin.clone())
                    } else {
                        None
                    }
                } else {
                    Some(place.span().clone())
                };
                self.expr(value, Some(&lhs), origin.as_ref())?;
                Ok(())
            }
            Statement::Return { value, span } => {
                if let Some(value) = value {
                    let expected = self.result.clone();
                    let origin = self.result_span.clone();
                    self.expr(value, Some(&expected), origin.as_ref())?;
                } else if self.result != Type::Unit {
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
                    let mut d = mismatch(
                        primary.clone(),
                        Span {
                            file: self.file.into(),
                            start: span.start,
                            end: span.start + 6,
                        },
                        self.result_span.clone(),
                        &pretty(&self.result),
                        &Type::Unit,
                    );
                    // An empty return has no expression to serve as found_type.
                    d.related_spans.retain(|r| r.role != RelatedRole::FoundType);
                    let next = self.tokens.iter().find(|t| t.span.start >= primary.end);
                    if let Some(newline) = next.filter(|t| t.kind == Kind::Newline) {
                        d.related_spans.push(RelatedSpan {
                            role: RelatedRole::Delimiter,
                            span: newline.span.clone(),
                        });
                        names::sort_related(&mut d);
                    }
                    return Err(d);
                }
                Ok(())
            }
            Statement::Value { value, .. } => {
                if let Some(value) = value {
                    self.expr(value, None, None)?;
                }
                Ok(())
            }
            Statement::Expr { value, .. } => {
                self.expr(value, None, None)?;
                Ok(())
            }
            Statement::Block(b) => self.block(b),
            Statement::If {
                condition,
                then_block,
                otherwise,
                ..
            } => {
                self.expr(condition, Some(&Type::Bool), None)?;
                self.block(then_block)?;
                if let Some(other) = otherwise {
                    self.statement(other)?;
                }
                Ok(())
            }
            Statement::While {
                condition, body, ..
            } => {
                self.expr(condition, Some(&Type::Bool), None)?;
                self.block(body)
            }
        }
    }

    fn expr(
        &mut self,
        e: &Expr,
        expected: Option<&Type>,
        expected_span: Option<&Span>,
    ) -> Result<Type, Box<Diagnostic>> {
        self.enter(e.span())?;
        let result = self.expr_inner(e, expected, expected_span);
        self.depth -= 1;
        let got = result?;
        if let Some(expected) = expected
            && !self.deferred_exprs.contains(&e.span().start)
            && &got != expected
        {
            return Err(mismatch(
                e.span().clone(),
                e.span().clone(),
                expected_span.cloned(),
                &pretty(expected),
                &got,
            ));
        }
        Ok(got)
    }

    fn expr_inner(
        &mut self,
        e: &Expr,
        expected: Option<&Type>,
        expected_span: Option<&Span>,
    ) -> Result<Type, Box<Diagnostic>> {
        match e {
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
            Expr::Unit(..) => Ok(Type::Unit),
            Expr::Name(_, span) => match self.resolved.uses.get(&span.start) {
                Some(ResolvedUse::Local(local)) => {
                    if self.invalid.contains(&local.name_span.start) {
                        self.tainted = true;
                    }
                    if let Some(known) = self.locals.get(&local.name_span.start) {
                        if known.deferred {
                            self.deferred_exprs.insert(span.start);
                        }
                        Ok(known.ty.clone())
                    } else {
                        Ok(Type::Unit)
                    }
                }
                _ => Ok(Type::Unit), // Function values have already been rejected by parse or names.
            },
            Expr::Group(inner, span) => {
                let ty = self.expr(inner, expected, expected_span)?;
                if self.deferred_exprs.contains(&inner.span().start) {
                    self.deferred_exprs.insert(span.start);
                }
                Ok(ty)
            }
            Expr::Unary {
                mode: Mode::Read,
                value,
                span,
            } => {
                self.expr(value, None, None)?;
                self.deferred_exprs.insert(span.start);
                Ok(Type::Unit) // `?` belongs to I08.
            }
            Expr::Unary { value, .. } => self.expr(value, expected, expected_span),
            Expr::Array(xs, span) => {
                let element_expected = match expected {
                    Some(Type::Array(t)) => Some(&**t),
                    _ => None,
                };
                let Some(first) = xs.first() else {
                    return element_expected
                        .cloned()
                        .map(|x| Type::Array(Box::new(x)))
                        .ok_or_else(|| {
                            names::diagnostic(
                                "NX-E0103",
                                None,
                                Stage::Types,
                                Operation::TypesInfer,
                                span.clone(),
                            )
                        });
                };
                let item = self.expr(first, element_expected, expected_span)?;
                if self.deferred_exprs.contains(&first.span().start) {
                    for later in &xs[1..] {
                        self.expr(later, None, None)?;
                    }
                    self.deferred_exprs.insert(e.span().start);
                    return Ok(Type::Array(Box::new(Type::Unit)));
                }
                if contains_resource(&item) {
                    return Err(resource_array(span.clone()));
                }
                for later in &xs[1..] {
                    self.expr(later, Some(&item), Some(first.span()))?;
                }
                Ok(Type::Array(Box::new(item)))
            }
            Expr::Binary {
                op, left, right, ..
            } => {
                let l = self.expr(left, None, None)?;
                if self.deferred_exprs.contains(&left.span().start) {
                    self.expr(right, None, None)?;
                    self.deferred_exprs.insert(e.span().start);
                    return Ok(l);
                }
                let op_span = self.operator_span(left.span(), right.span(), op);
                match op.as_str() {
                    "+" | "-" => {
                        if l != Type::I64 {
                            return Err(mismatch(
                                left.span().clone(),
                                left.span().clone(),
                                None,
                                "i64",
                                &l,
                            ));
                        }
                        let r = self.expr(right, None, None)?;
                        if self.deferred_exprs.contains(&right.span().start) {
                            self.deferred_exprs.insert(e.span().start);
                            return Ok(Type::I64);
                        }
                        if r != Type::I64 {
                            return Err(mismatch(
                                right.span().clone(),
                                right.span().clone(),
                                None,
                                "i64",
                                &r,
                            ));
                        }
                        Ok(Type::I64)
                    }
                    "<" | "==" => {
                        let r = self.expr(right, None, None)?;
                        if self.deferred_exprs.contains(&right.span().start) {
                            self.deferred_exprs.insert(e.span().start);
                            return Ok(Type::Bool);
                        }
                        if l != r {
                            return Err(mismatch(
                                right.span().clone(),
                                right.span().clone(),
                                Some(left.span().clone()),
                                &pretty(&l),
                                &r,
                            ));
                        }
                        if op == "<" && l != Type::I64 {
                            return Err(mismatch(op_span, left.span().clone(), None, "i64", &l));
                        }
                        if op == "==" && !copyable(&l) {
                            return Err(mismatch(
                                op_span,
                                left.span().clone(),
                                None,
                                "domain:copyable_data",
                                &l,
                            ));
                        }
                        Ok(Type::Bool)
                    }
                    _ => unreachable!("the parser admits only four intrinsic binary operators"),
                }
            }
            Expr::Index { target, index, .. } => {
                let base = self.expr(target, None, None)?;
                let Type::Array(item) = base else {
                    return Err(domain_error(
                        "not_indexable",
                        target.span().clone(),
                        target.span().clone(),
                        None,
                        "domain:array",
                        &base,
                    ));
                };
                self.expr(index, Some(&Type::I64), None)?;
                Ok(*item)
            }
            Expr::Call { args, span, .. } => {
                let binding = &self.resolved.calls[&span.start];
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
                    );
                }
                for (arg, slot) in args.iter().zip(&binding.slots) {
                    let formal = &signature.params[*slot];
                    self.expr(&arg.value, Some(&formal.ty), Some(&formal.type_span))?;
                }
                Ok(signature.result.clone())
            }
            Expr::If {
                condition,
                then_block,
                otherwise,
                ..
            } => {
                let normal = self.if_expr(
                    condition,
                    then_block,
                    otherwise.as_deref(),
                    expected,
                    expected_span,
                    None,
                )?;
                if normal.is_none() && expr_returns(e) {
                    self.deferred_exprs.insert(e.span().start);
                }
                Ok(normal.or_else(|| expected.cloned()).unwrap_or(Type::Unit))
            }
        }
    }

    fn if_expr(
        &mut self,
        condition: &Expr,
        then_block: &Block,
        otherwise: Option<&Else>,
        expected: Option<&Type>,
        expected_span: Option<&Span>,
        anchor: Option<(&Type, &Span)>,
    ) -> Result<Option<Type>, Box<Diagnostic>> {
        self.expr(condition, Some(&Type::Bool), None)?;
        let first = self.branch(then_block, expected, expected_span)?;
        let first_span = branch_value_span(then_block);
        if expected.is_none()
            && let (Some((base, base_span)), Some(found)) = (anchor, first.as_ref())
            && base != found
        {
            return Err(branch_mismatch(
                base,
                found,
                base_span,
                first_span
                    .clone()
                    .unwrap_or_else(|| then_block.span.clone()),
            ));
        }
        let anchor = anchor.or_else(|| first.as_ref().zip(first_span.as_ref()));
        let second = match otherwise {
            Some(Else::Block(block)) => {
                let second = self.branch(block, expected, expected_span)?;
                if expected.is_none()
                    && let (Some((base, base_span)), Some(found)) = (anchor, second.as_ref())
                    && base != found
                {
                    return Err(branch_mismatch(
                        base,
                        found,
                        base_span,
                        branch_value_span(block).unwrap_or_else(|| block.span.clone()),
                    ));
                }
                second
            }
            Some(Else::If(other)) => {
                self.enter(other.span())?;
                let result = if let Expr::If {
                    condition,
                    then_block,
                    otherwise,
                    ..
                } = &**other
                {
                    self.if_expr(
                        condition,
                        then_block,
                        otherwise.as_deref(),
                        expected,
                        expected_span,
                        anchor,
                    )
                } else {
                    self.expr(other, expected, expected_span).map(Some)
                };
                self.depth -= 1;
                result?
            }
            None => None, // Missing continuation is diagnosed by I07.
        };
        Ok(first.or(second))
    }

    fn branch(
        &mut self,
        b: &Block,
        expected: Option<&Type>,
        expected_span: Option<&Span>,
    ) -> Result<Option<Type>, Box<Diagnostic>> {
        let mut result = None;
        for s in &b.statements {
            if let Statement::Value { value: Some(e), .. } = s {
                let previous_taint = self.tainted;
                self.tainted = false;
                let outcome = self.expr(e, expected, expected_span);
                let tainted = self.tainted;
                self.tainted = previous_taint || tainted;
                match outcome {
                    Ok(ty) => result = (!expr_returns(e)).then_some(ty),
                    Err(error) => {
                        if !tainted {
                            self.errors.push(error);
                        }
                        result = None;
                    }
                }
            } else {
                self.statement_at_site(s);
                result = None;
            }
        }
        Ok(result)
    }

    fn operator_span(&self, left: &Span, right: &Span, op: &str) -> Span {
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

    fn check_intrinsic(
        &mut self,
        name: &str,
        args: &[crate::frontend::parse::Argument],
        slots: &[usize],
        call_span: &Span,
        expected: Option<&Type>,
        expected_span: Option<&Span>,
    ) -> Result<Type, Box<Diagnostic>> {
        let declared = names::intrinsic(name).expect("intrinsic resolved from closed manifest");
        let ordered = |index: usize| -> &Expr {
            &args[slots
                .iter()
                .position(|slot| *slot == index)
                .expect("complete binding")]
            .value
        };
        match name {
            "mock_open" | "mock_blocking_open" | "mock_write_i64" | "mock_reply" => {
                self.expr(ordered(0), Some(&Type::I64), None)?;
                Ok(declared.result)
            }
            "try_add" => {
                // Type checking is in written order even when labels permute slots.
                for arg in args {
                    self.expr(&arg.value, Some(&Type::I64), None)?;
                }
                Ok(declared.result)
            }
            "mock_request" | "mock_unknown" | "fail" => Ok(declared.result),
            "mock_read_records" => {
                let got = self.expr(ordered(0), None, None)?;
                if got != Type::MockResource {
                    return Err(mismatch(
                        ordered(0).span().clone(),
                        ordered(0).span().clone(),
                        None,
                        "MockResource",
                        &got,
                    ));
                }
                Ok(declared.result)
            }
            "inspect" => {
                let got = self.expr(ordered(0), None, None)?;
                if !matches!(got, Type::MockResource | Type::MockBlockingResource) {
                    return Err(domain_error(
                        "resource_required",
                        ordered(0).span().clone(),
                        ordered(0).span().clone(),
                        None,
                        "domain:mock_resource",
                        &got,
                    ));
                }
                Ok(declared.result)
            }
            "drop" => {
                self.expr(ordered(0), None, None)?;
                Ok(declared.result)
            }
            "len" => {
                let got = self.expr(ordered(0), None, None)?;
                if !matches!(got, Type::Array(_)) {
                    return Err(domain_error(
                        "not_indexable",
                        ordered(0).span().clone(),
                        ordered(0).span().clone(),
                        None,
                        "domain:array",
                        &got,
                    ));
                }
                Ok(declared.result)
            }
            "print" => {
                let got = self.expr(ordered(0), None, None)?;
                if !copyable(&got) {
                    return Err(mismatch(
                        ordered(0).span().clone(),
                        ordered(0).span().clone(),
                        None,
                        "domain:copyable_data",
                        &got,
                    ));
                }
                Ok(declared.result)
            }
            "assert_eq" => {
                let mut values: [Option<Type>; 2] = [None, None];
                for (arg, &slot) in args.iter().zip(slots) {
                    values[slot] = Some(self.expr(&arg.value, None, None)?);
                }
                let first = values[0].take().expect("complete binding");
                let second = values[1].take().expect("complete binding");
                if first != second {
                    return Err(mismatch(
                        ordered(1).span().clone(),
                        ordered(1).span().clone(),
                        Some(ordered(0).span().clone()),
                        &pretty(&first),
                        &second,
                    ));
                }
                if !copyable(&first) {
                    return Err(mismatch(
                        ordered(0).span().clone(),
                        ordered(0).span().clone(),
                        None,
                        "domain:copyable_data",
                        &first,
                    ));
                }
                Ok(declared.result)
            }
            "Ok" | "Err" => {
                let expected_result = match expected {
                    Some(Type::Result(ok, err)) => Some((&**ok, &**err)),
                    _ => None,
                };
                let payload_expected =
                    expected_result.map(|(ok, err)| if name == "Ok" { ok } else { err });
                let payload = self.expr(ordered(0), payload_expected, expected_span)?;
                if let Some((ok, err)) = expected_result {
                    return Ok(Type::Result(Box::new(ok.clone()), Box::new(err.clone())));
                }
                let mut d = names::diagnostic(
                    "NX-E0113",
                    None,
                    Stage::Types,
                    Operation::TypesInfer,
                    call_span.clone(),
                );
                let path = if name == "Ok" { "$.err" } else { "$.ok" };
                let _ = payload;
                d.data.insert(
                    "unresolved_components".into(),
                    DataValue::Array(vec![DataValue::String(path.into())]),
                );
                Err(d)
            }
            _ => unreachable!("closed intrinsic registry"),
        }
    }
}

pub fn check(
    file: &str,
    source: &str,
    ast: &Ast,
    resolved: &Resolved,
) -> Result<(), Vec<Box<Diagnostic>>> {
    let tokens = lex::lex(file, source).map_err(|d| vec![d])?;
    let mut checker = Checker {
        file,
        tokens,
        resolved,
        locals: HashMap::new(),
        invalid: HashSet::new(),
        deferred_exprs: HashSet::new(),
        tainted: false,
        result: Type::Unit,
        result_span: None,
        depth: 0,
        errors: Vec::new(),
    };
    for function in &ast.functions {
        if let Err(error) = checker.function(function) {
            checker.errors.push(error);
        }
    }
    if !checker.errors.is_empty() {
        names::sort_diagnostics(&mut checker.errors);
        return Err(checker.errors);
    }
    Ok(())
}
