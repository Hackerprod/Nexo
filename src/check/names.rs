//! Name registration, lexical scopes and structural call binding for F0-0002.
//! This module is deliberately separate from the slice-1 checker.
use std::borrow::Cow;
use std::collections::{HashMap, HashSet};

use crate::diagnostic::{
    self, DataValue, Diagnostic, Namespace, Operation, RelatedRole, RelatedSpan, Severity, Span,
    Stage,
};
use crate::frontend::{
    lex::{self, Kind, Token},
    parse::{Argument, Ast, Block, Else, Expr, Function, Mode, Statement, Type},
};

#[derive(Clone, Debug)]
pub struct ParameterSig {
    pub name: String,
    pub name_span: Span,
    pub type_span: Span,
    pub mode: Mode,
    pub ty: Type,
}

#[derive(Clone, Debug)]
pub struct Signature {
    pub name_span: Span,
    pub params: Vec<ParameterSig>,
    pub result: Type,
    pub intrinsic: bool,
}

#[derive(Clone, Debug)]
pub struct Local {
    pub name_span: Span,
    pub mode: Mode,
    pub mutable: bool,
}

#[derive(Clone, Debug)]
pub enum ResolvedUse {
    Local(Local),
    Function(String),
    Intrinsic(String),
}

#[derive(Clone, Debug)]
pub struct CallBinding {
    pub target: ResolvedUse,
    /// The index in the declared parameter list for each written argument.
    pub slots: Vec<usize>,
}

#[derive(Clone, Debug)]
pub struct Resolved {
    pub signatures: HashMap<String, Signature>,
    pub uses: HashMap<u64, ResolvedUse>,
    pub calls: HashMap<u64, CallBinding>,
}

#[allow(clippy::vec_box)] // Semantic diagnostics use the shared boxed representation.
type Registered = (Resolved, Vec<Box<Diagnostic>>);

pub(crate) fn diagnostic(
    code: &str,
    subcause: Option<&str>,
    stage: Stage,
    operation: Operation,
    span: Span,
) -> Box<Diagnostic> {
    let rule = subcause.map_or_else(|| code.to_owned(), |s| format!("{code}/{s}"));
    Box::new(Diagnostic {
        schema_version: diagnostic::SCHEMA_VERSION,
        diagnostic_namespace: Namespace::NexoProposedV1,
        profile_id: diagnostic::PROFILE_ID.into(),
        code: code.into(),
        subcause: subcause.map(str::to_owned),
        rule: rule.clone(),
        explanation_id: Some(rule),
        severity: Severity::Error,
        stage,
        message: "Invalid name, call or type in the proposed profile.".into(),
        operation,
        primary_span: Some(span),
        related_spans: Vec::new(),
        cause_chain: Vec::new(),
        feature_id: None,
        unexpected_token: None,
        expected_tokens: Vec::new(),
        expected_tokens_kind: None,
        parser_context: None,
        recognized_prefix: None,
        repair: Vec::new(),
        data: Default::default(),
    })
}

fn related(d: &mut Diagnostic, role: RelatedRole, span: Span) {
    d.related_spans.push(RelatedSpan { role, span });
}

pub(crate) fn sort_related(d: &mut Diagnostic) {
    d.related_spans.sort_by(|a, b| {
        (&a.span.file, a.span.start, a.span.end, a.role).cmp(&(
            &b.span.file,
            b.span.start,
            b.span.end,
            b.role,
        ))
    });
}

fn duplicate(span: Span, first: Span, kind: &str) -> Box<Diagnostic> {
    let mut d = diagnostic(
        "NX-E0104",
        None,
        Stage::Names,
        Operation::NamesDeclare,
        span,
    );
    related(&mut d, RelatedRole::PreviousDeclaration, first);
    d.data
        .insert("declaration_kind".into(), DataValue::String(kind.into()));
    d
}

fn unknown(span: Span, kind: &str) -> Box<Diagnostic> {
    let mut d = diagnostic(
        "NX-E0101",
        None,
        Stage::Names,
        Operation::NamesResolve,
        span,
    );
    d.data
        .insert("use_kind".into(), DataValue::String(kind.into()));
    d
}

fn argument_error(
    subcause: &str,
    span: Span,
    related_span: Option<(RelatedRole, Span)>,
) -> Box<Diagnostic> {
    let mut d = diagnostic(
        "NX-E0111",
        Some(subcause),
        Stage::Arguments,
        Operation::CallBind,
        span,
    );
    if let Some((role, span)) = related_span {
        related(&mut d, role, span);
    }
    d
}

fn label_span(argument: &Argument) -> Option<Span> {
    argument.label.as_ref().map(|label| Span {
        file: argument.span.file.clone(),
        start: argument.span.start,
        end: argument.span.start + label.len() as u64,
    })
}

/// Find a token in a short production window. Token starts are searched by binary search,
/// avoiding a scan of the whole source for each binding or parameter.
pub(crate) fn token_span(tokens: &[Token], start: u64, end: u64, text: &str) -> Option<Span> {
    let mut at = tokens.partition_point(|t| t.span.start < start);
    while let Some(t) = tokens.get(at) {
        if t.span.start >= end {
            break;
        }
        if t.text == text && t.kind == Kind::Name {
            return Some(t.span.clone());
        }
        at += 1;
    }
    None
}

fn parameter_span(tokens: &[Token], p: &crate::frontend::parse::Parameter) -> Span {
    token_span(tokens, p.span.start, p.span.end, &p.name).unwrap_or_else(|| p.span.clone())
}

fn binding_span(tokens: &[Token], name: &str, stmt_span: &Span, value: &Expr) -> Span {
    token_span(tokens, stmt_span.start, value.span().start, name)
        .unwrap_or_else(|| stmt_span.clone())
}

/// Register all module signatures before any body is checked. The first duplicate wins
/// diagnostically, and no arbitrary declaration is selected for subsequent resolution.
pub fn register(file: &str, source: &str, ast: &Ast) -> Result<Registered, Vec<Box<Diagnostic>>> {
    let tokens = lex::lex(file, source).map_err(|d| vec![d])?;
    let mut signatures = HashMap::with_capacity(ast.functions.len());
    let mut errors = Vec::new();
    for function in &ast.functions {
        if let Some(previous) = signatures.get(&function.name) {
            let previous: &Signature = previous;
            errors.push(duplicate(
                function.name_span.clone(),
                previous.name_span.clone(),
                "function",
            ));
            continue;
        }
        let mut seen = HashMap::<&str, Span>::with_capacity(function.params.len());
        let mut params = Vec::with_capacity(function.params.len());
        for p in &function.params {
            let name_span = parameter_span(&tokens, p);
            let colon = tokens
                .iter()
                .skip(tokens.partition_point(|t| t.span.start < name_span.end))
                .find(|t| t.span.start < p.span.end && t.text == ":");
            let type_span = colon
                .and_then(|colon| {
                    tokens
                        .iter()
                        .skip(tokens.partition_point(|t| t.span.start < colon.span.end))
                        .find(|t| t.kind != Kind::Newline)
                })
                .map(|first| Span {
                    file: file.into(),
                    start: first.span.start,
                    end: tokens[..tokens.partition_point(|t| t.span.start < p.span.end)]
                        .iter()
                        .rev()
                        .find(|t| t.kind != Kind::Newline)
                        .map_or(first.span.end, |t| t.span.end),
                })
                .unwrap_or_else(|| p.span.clone());
            if let Some(previous) = seen.get(p.name.as_str()) {
                errors.push(duplicate(name_span, previous.clone(), "parameter"));
                continue;
            }
            seen.insert(&p.name, name_span.clone());
            params.push(ParameterSig {
                name: p.name.clone(),
                name_span,
                type_span,
                mode: p.mode,
                ty: p.ty.clone(),
            });
        }
        signatures.insert(
            function.name.clone(),
            Signature {
                name_span: function.name_span.clone(),
                params,
                result: function.result.clone().unwrap_or(Type::Unit),
                intrinsic: false,
            },
        );
    }
    sort_diagnostics(&mut errors);
    Ok((
        Resolved {
            signatures,
            uses: HashMap::new(),
            calls: HashMap::new(),
        },
        errors,
    ))
}

pub(crate) fn intrinsic(name: &str) -> Option<Signature> {
    use Type::{I64, MockBlockingResource, MockResource, Unit};
    let (params, result) = match name {
        "mock_open" => (vec![("label", I64)], MockResource),
        "mock_blocking_open" => (vec![("label", I64)], MockBlockingResource),
        "inspect" => (vec![("r", MockResource)], Unit),
        "mock_read_records" => (
            vec![("r", MockResource)],
            Type::Result(
                Box::new(Type::Array(Box::new(Type::Result(
                    Box::new(I64),
                    Box::new(I64),
                )))),
                Box::new(I64),
            ),
        ),
        "mock_write_i64" => (
            vec![("n", I64)],
            Type::Result(Box::new(Unit), Box::new(I64)),
        ),
        "mock_request" => (vec![], Type::Result(Box::new(I64), Box::new(I64))),
        "mock_reply" => (
            vec![("status", I64)],
            Type::Result(Box::new(Unit), Box::new(I64)),
        ),
        "try_add" => (
            vec![("a", I64), ("b", I64)],
            Type::Result(Box::new(I64), Box::new(I64)),
        ),
        "len" => (vec![("a", Unit)], I64),
        "print" | "drop" => (vec![("v", Unit)], Unit),
        "assert_eq" => (vec![("a", Unit), ("b", Unit)], Unit),
        "fail" | "mock_unknown" => (vec![], Unit),
        "Ok" | "Err" => (vec![("v", Unit)], Unit),
        _ => return None,
    };
    let params = params
        .into_iter()
        .map(|(name, ty)| ParameterSig {
            name: name.into(),
            name_span: Span {
                file: "<prelude>".into(),
                start: 0,
                end: 0,
            },
            type_span: Span {
                file: "<prelude>".into(),
                start: 0,
                end: 0,
            },
            mode: Mode::Read,
            ty,
        })
        .collect();
    Some(Signature {
        name_span: Span {
            file: "<prelude>".into(),
            start: 0,
            end: 0,
        },
        params,
        result,
        intrinsic: true,
    })
}

pub(crate) fn signature<'a>(
    resolved: &'a Resolved,
    target: &ResolvedUse,
) -> Option<Cow<'a, Signature>> {
    match target {
        ResolvedUse::Function(name) => resolved.signatures.get(name).map(Cow::Borrowed),
        ResolvedUse::Intrinsic(name) => intrinsic(name).map(Cow::Owned),
        ResolvedUse::Local(_) => None,
    }
}

fn projected_named_place(expr: &Expr) -> bool {
    let mut node = expr;
    while let Expr::Group(inner, _) = node {
        node = inner;
    }
    if !matches!(node, Expr::Index { .. }) {
        return false;
    }
    loop {
        node = match node {
            Expr::Group(inner, _) => inner,
            Expr::Index { target, .. } => target,
            Expr::Name(..) => return true,
            _ => return false,
        };
    }
}

#[allow(clippy::vec_box)] // Public semantic result keeps boxed diagnostics.
struct Resolver {
    tokens: Vec<Token>,
    resolved: Resolved,
    scopes: Vec<HashMap<String, Local>>,
    depth: usize,
    phase: Stage,
    errors: Vec<Box<Diagnostic>>,
}

impl Resolver {
    fn lookup(&self, name: &str) -> Option<ResolvedUse> {
        for scope in self.scopes.iter().rev() {
            if let Some(local) = scope.get(name) {
                return Some(ResolvedUse::Local(local.clone()));
            }
        }
        if self.resolved.signatures.contains_key(name) {
            Some(ResolvedUse::Function(name.into()))
        } else {
            intrinsic(name).map(|_| ResolvedUse::Intrinsic(name.into()))
        }
    }

    fn enter(&mut self, span: &Span) -> Result<(), Box<Diagnostic>> {
        // The parser bounds syntax and expression height, but the checker still
        // guards every recursive activation before descending into untrusted ASTs.
        if self.depth >= 256 {
            let mut d = diagnostic(
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

    fn block(&mut self, block: &Block, child: bool) -> Result<(), Box<Diagnostic>> {
        self.enter(&block.span)?;
        if child {
            self.scopes.push(HashMap::new());
        }
        for statement in &block.statements {
            if let Err(error) = self.statement(statement) {
                self.errors.push(error);
            }
        }
        if child {
            self.scopes.pop();
        }
        self.depth -= 1;
        Ok(())
    }

    fn statement(&mut self, s: &Statement) -> Result<(), Box<Diagnostic>> {
        match s {
            Statement::Let {
                mutable,
                name,
                value,
                span,
                ..
            } => {
                let ns = binding_span(&self.tokens, name, span, value);
                if let Err(error) = self.expr(value, false) {
                    let scope = self.scopes.last_mut().expect("function scope exists");
                    scope.entry(name.clone()).or_insert_with(|| Local {
                        name_span: ns.clone(),
                        mode: Mode::Read,
                        mutable: *mutable,
                    });
                    return Err(error);
                }
                let scope = self.scopes.last_mut().expect("function scope exists");
                if self.phase == Stage::Names
                    && let Some(previous) = scope.get(name)
                {
                    return Err(duplicate(ns, previous.name_span.clone(), "binding"));
                }
                scope.insert(
                    name.clone(),
                    Local {
                        name_span: ns,
                        mode: Mode::Read,
                        mutable: *mutable,
                    },
                );
                Ok(())
            }
            Statement::Assign { place, value, .. } => {
                self.expr(place, false)?;
                self.expr(value, false)
            }
            Statement::Return { value, .. } | Statement::Value { value, .. } => {
                if let Some(e) = value {
                    self.expr(e, false)?;
                }
                Ok(())
            }
            Statement::Expr { value, .. } => self.expr(value, false),
            Statement::Block(b) => self.block(b, true),
            Statement::If {
                condition,
                then_block,
                otherwise,
                ..
            } => {
                self.expr(condition, false)?;
                self.block(then_block, true)?;
                if let Some(o) = otherwise {
                    self.statement(o)?;
                }
                Ok(())
            }
            Statement::While {
                condition, body, ..
            } => {
                self.expr(condition, false)?;
                self.block(body, true)
            }
        }
    }

    fn else_branch(&mut self, branch: &Else) -> Result<(), Box<Diagnostic>> {
        match branch {
            Else::Block(b) => self.block(b, true),
            Else::If(e) => self.expr(e, false),
        }
    }

    fn expr(&mut self, e: &Expr, call_target: bool) -> Result<(), Box<Diagnostic>> {
        self.enter(e.span())?;
        let result = self.expr_inner(e, call_target);
        self.depth -= 1;
        result
    }

    fn expr_inner(&mut self, e: &Expr, call_target: bool) -> Result<(), Box<Diagnostic>> {
        match e {
            Expr::Name(name, span) => {
                if self.phase == Stage::Arguments {
                    return Ok(());
                }
                let kind = if call_target { "call_target" } else { "value" };
                let found = self
                    .lookup(name)
                    .ok_or_else(|| unknown(span.clone(), kind))?;
                if !call_target && matches!(found, ResolvedUse::Intrinsic(_)) {
                    let mut d = diagnostic(
                        "NX-E0006",
                        None,
                        Stage::Names,
                        Operation::NamesResolve,
                        span.clone(),
                    );
                    d.feature_id = Some("function_value".into());
                    d.data.insert(
                        "pending_scope".into(),
                        DataValue::String("later_adr".into()),
                    );
                    return Err(d);
                }
                self.resolved.uses.insert(span.start, found);
                Ok(())
            }
            Expr::Integer(..) | Expr::Bool(..) | Expr::Unit(..) => Ok(()),
            Expr::Array(xs, _) => xs.iter().try_for_each(|x| self.expr(x, false)),
            Expr::Group(x, _) => self.expr(x, call_target),
            Expr::Unary { value, .. } => self.expr(value, false),
            Expr::Binary { left, right, .. } => {
                self.expr(left, false)?;
                self.expr(right, false)
            }
            Expr::Index { target, index, .. } => {
                self.expr(target, false)?;
                self.expr(index, false)
            }
            Expr::If {
                condition,
                then_block,
                otherwise,
                ..
            } => {
                self.expr(condition, false)?;
                self.block(then_block, true)?;
                if let Some(o) = otherwise {
                    self.else_branch(o)?;
                }
                Ok(())
            }
            Expr::Call { target, args, span } => {
                self.expr(target, true)?;
                if self.phase == Stage::Names {
                    let mut first = None;
                    for arg in args {
                        if let Err(error) = self.expr(&arg.value, false) {
                            if first.is_none() {
                                first = Some(error);
                            } else {
                                self.errors.push(error);
                            }
                        }
                    }
                    return first.map_or(Ok(()), Err);
                }
                let mut unwrapped = &**target;
                while let Expr::Group(inner, _) = unwrapped {
                    unwrapped = inner;
                }
                let target_use = match unwrapped {
                    Expr::Name(_, s) => self.resolved.uses.get(&s.start),
                    _ => None,
                }
                .cloned();
                let Some(target_use) = target_use else {
                    return Err(argument_error(
                        "non_callable_target",
                        target.span().clone(),
                        None,
                    ));
                };
                if let ResolvedUse::Local(local) = &target_use {
                    return Err(argument_error(
                        "non_callable_target",
                        target.span().clone(),
                        Some((RelatedRole::Declaration, local.name_span.clone())),
                    ));
                }
                let sig = signature(&self.resolved, &target_use).expect("registered target");
                let slots = self.bind(&sig, target.span(), args)?;
                if matches!(&target_use,ResolvedUse::Intrinsic(name) if name == "drop")
                    && let Some(arg) = args.first()
                    && projected_named_place(&arg.value)
                {
                    let mut d = diagnostic(
                        "NX-E0006",
                        None,
                        Stage::Arguments,
                        Operation::CallBind,
                        arg.value.span().clone(),
                    );
                    d.feature_id = Some("partial_transfer".into());
                    d.data.insert(
                        "pending_scope".into(),
                        DataValue::String("later_adr".into()),
                    );
                    related(&mut d, RelatedRole::Context, target.span().clone());
                    return Err(d);
                }
                self.resolved.calls.insert(
                    span.start,
                    CallBinding {
                        target: target_use,
                        slots,
                    },
                );
                args.iter().try_for_each(|a| self.expr(&a.value, false))
            }
        }
    }

    fn bind(
        &self,
        sig: &Signature,
        target: &Span,
        args: &[Argument],
    ) -> Result<Vec<usize>, Box<Diagnostic>> {
        let mut slots = Vec::with_capacity(args.len());
        let mut occupied: Vec<Option<Span>> = vec![None; sig.params.len()];
        let labels: HashMap<&str, usize> = sig
            .params
            .iter()
            .enumerate()
            .map(|(i, p)| (p.name.as_str(), i))
            .collect();
        let mut next_positional = 0;
        let mut first_named: Option<Span> = None;
        for a in args {
            let slot = if let Some(label) = &a.label {
                let label_span = label_span(a).expect("named argument");
                first_named.get_or_insert_with(|| label_span.clone());
                *labels
                    .get(label.as_str())
                    .ok_or_else(|| argument_error("unknown_argument_label", label_span, None))?
            } else {
                if let Some(first) = &first_named {
                    return Err(argument_error(
                        "positional_after_named",
                        a.value.span().clone(),
                        Some((RelatedRole::Context, first.clone())),
                    ));
                }
                let slot = next_positional;
                next_positional += 1;
                if slot >= sig.params.len() {
                    return Err(argument_error(
                        "excess_positional_argument",
                        a.value.span().clone(),
                        None,
                    ));
                }
                slot
            };
            if let Some(previous) = &occupied[slot] {
                let primary = label_span(a).unwrap_or_else(|| a.value.span().clone());
                return Err(argument_error(
                    "duplicate_argument",
                    primary,
                    Some((RelatedRole::Context, previous.clone())),
                ));
            }
            occupied[slot] = Some(label_span(a).unwrap_or_else(|| a.value.span().clone()));
            let actual = match &a.value {
                Expr::Unary { mode, .. } => *mode,
                _ => Mode::Read,
            };
            let formal = sig.params[slot].mode;
            let compatible = match formal {
                Mode::Read => actual != Mode::Inout,
                Mode::Inout => actual == Mode::Inout,
                Mode::Consume => actual != Mode::Inout,
            };
            if !compatible {
                let related = (!sig.intrinsic)
                    .then(|| (RelatedRole::Declaration, sig.params[slot].name_span.clone()));
                let span = match &a.value {
                    Expr::Unary { value, .. } if actual != Mode::Read => value.span().clone(),
                    _ => a.value.span().clone(),
                };
                return Err(argument_error("argument_mode_mismatch", span, related));
            }
            slots.push(slot);
        }
        for (i, occupied) in occupied.iter().enumerate() {
            if occupied.is_none() {
                let related = (!sig.intrinsic)
                    .then(|| (RelatedRole::Declaration, sig.params[i].name_span.clone()));
                return Err(argument_error("missing_argument", target.clone(), related));
            }
        }
        Ok(slots)
    }
}

pub fn resolve(file: &str, source: &str, ast: &Ast) -> Result<Resolved, Vec<Box<Diagnostic>>> {
    let (registered, registration_errors) = register(file, source, ast)?;
    let tokens = lex::lex(file, source).map_err(|d| vec![d])?;
    let mut resolver = Resolver {
        tokens,
        resolved: registered,
        scopes: Vec::new(),
        depth: 0,
        phase: Stage::Names,
        errors: registration_errors,
    };
    let mut seen_functions = HashSet::new();
    for function in &ast.functions {
        if !seen_functions.insert(function.name.as_str()) {
            continue;
        }
        if let Err(error) = resolver.function(function) {
            resolver.errors.push(error);
        }
    }
    if !resolver.errors.is_empty() {
        sort_diagnostics(&mut resolver.errors);
        return Err(resolver.errors);
    }
    resolver.phase = Stage::Arguments;
    for function in &ast.functions {
        if let Err(error) = resolver.function(function) {
            resolver.errors.push(error);
        }
    }
    if !resolver.errors.is_empty() {
        sort_diagnostics(&mut resolver.errors);
        return Err(resolver.errors);
    }
    Ok(resolver.resolved)
}

pub(crate) fn sort_diagnostics(errors: &mut [Box<Diagnostic>]) {
    errors.sort_by(|a, b| {
        let x = a.primary_span.as_ref().expect("semantic diagnostic span");
        let y = b.primary_span.as_ref().expect("semantic diagnostic span");
        (
            &x.file,
            x.start,
            x.end,
            &a.code,
            a.subcause.as_deref().unwrap_or(""),
        )
            .cmp(&(
                &y.file,
                y.start,
                y.end,
                &b.code,
                b.subcause.as_deref().unwrap_or(""),
            ))
    });
}

impl Resolver {
    fn function(&mut self, f: &Function) -> Result<(), Box<Diagnostic>> {
        let mut initial = HashMap::with_capacity(f.params.len());
        let sig = &self.resolved.signatures[&f.name];
        for p in &sig.params {
            initial.insert(
                p.name.clone(),
                Local {
                    name_span: p.name_span.clone(),
                    mode: p.mode,
                    mutable: p.mode == Mode::Inout,
                },
            );
        }
        self.scopes.push(initial);
        let result = self.block(&f.body, false);
        self.scopes.pop();
        result
    }
}

pub fn entry(
    file: &str,
    source: &str,
    resolved: &Resolved,
    ast: &Ast,
) -> Result<(), Box<Diagnostic>> {
    let Some(sig) = resolved.signatures.get("main") else {
        let mut d = diagnostic(
            "NX-E0112",
            Some("missing_main"),
            Stage::Entry,
            Operation::EntryCheck,
            Span {
                file: file.into(),
                start: source.len() as u64,
                end: source.len() as u64,
            },
        );
        d.data
            .insert("entry".into(), DataValue::String("main".into()));
        return Err(d);
    };
    let valid_result = sig.result == Type::Unit
        || sig.result == Type::Result(Box::new(Type::Unit), Box::new(Type::I64));
    if !sig.params.is_empty() || !valid_result {
        let mut d = diagnostic(
            "NX-E0112",
            Some("invalid_main_signature"),
            Stage::Entry,
            Operation::EntryCheck,
            sig.name_span.clone(),
        );
        d.data
            .insert("entry".into(), DataValue::String("main".into()));
        if let Some(f) = ast.functions.iter().find(|f| f.name == "main") {
            let tokens = lex::lex(file, source)?;
            let end = tokens
                .iter()
                .take_while(|t| t.span.start < f.body.span.start)
                .filter(|t| t.kind != Kind::Newline)
                .last()
                .map_or(f.name_span.end, |t| t.span.end);
            related(
                &mut d,
                RelatedRole::Context,
                Span {
                    file: file.into(),
                    start: f.span.start,
                    end,
                },
            );
        }
        return Err(d);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn function_scope_restores_preexisting_resolver_state() {
        let source = "fn main(){}";
        let ast = crate::frontend::parse::parse("scope.nx", source).unwrap();
        let (resolved, errors) = register("scope.nx", source, &ast).unwrap();
        assert!(errors.is_empty());
        let tokens = lex::lex("scope.nx", source).unwrap();
        let mut outer = HashMap::new();
        outer.insert(
            "sentinel".into(),
            Local {
                name_span: Span {
                    file: "scope.nx".into(),
                    start: 0,
                    end: 1,
                },
                mode: Mode::Read,
                mutable: false,
            },
        );
        let mut resolver = Resolver {
            tokens,
            resolved,
            scopes: vec![outer],
            depth: 0,
            phase: Stage::Names,
            errors: Vec::new(),
        };
        resolver.function(&ast.functions[0]).unwrap();
        assert_eq!(resolver.scopes.len(), 1);
        assert!(resolver.scopes[0].contains_key("sentinel"));
    }
}
