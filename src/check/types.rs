//! Independent type checking for the accepted non-control core of F0-0002.
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use super::names::{self, Resolved, ResolvedUse};
use crate::diagnostic::{DataValue, Diagnostic, Operation, RelatedRole, RelatedSpan, Span, Stage};
use crate::frontend::{
    lex::{self, Kind, Token},
    parse::{Ast, Block, Else, Expr, Function, Mode, Statement, Type as SyntaxType},
};

#[derive(Clone, Debug, PartialEq, Eq)]
enum InferTy {
    I64,
    Bool,
    Unit,
    MockResource,
    MockBlockingResource,
    Array(Arc<InferTy>),
    Result(Arc<InferTy>, Arc<InferTy>),
    Deferred(u64),
    Never,
    Tainted,
}

#[cfg(test)]
include!("i07c_quota_campaign.rs");

type Type = InferTy;

impl From<&SyntaxType> for InferTy {
    fn from(value: &SyntaxType) -> Self {
        match value {
            SyntaxType::I64 => Self::I64,
            SyntaxType::Bool => Self::Bool,
            SyntaxType::Unit => Self::Unit,
            SyntaxType::MockResource => Self::MockResource,
            SyntaxType::MockBlockingResource => Self::MockBlockingResource,
            SyntaxType::Array(inner) => Self::Array(Arc::new(Self::from(inner.as_ref()))),
            SyntaxType::Result(ok, err) => Self::Result(
                Arc::new(Self::from(ok.as_ref())),
                Arc::new(Self::from(err.as_ref())),
            ),
        }
    }
}

fn syntax_type(value: &InferTy) -> Option<SyntaxType> {
    match value {
        InferTy::I64 => Some(SyntaxType::I64),
        InferTy::Bool => Some(SyntaxType::Bool),
        InferTy::Unit => Some(SyntaxType::Unit),
        InferTy::MockResource => Some(SyntaxType::MockResource),
        InferTy::MockBlockingResource => Some(SyntaxType::MockBlockingResource),
        InferTy::Array(inner) => Some(SyntaxType::Array(Box::new(syntax_type(inner)?))),
        InferTy::Result(ok, err) => Some(SyntaxType::Result(
            Box::new(syntax_type(ok)?),
            Box::new(syntax_type(err)?),
        )),
        InferTy::Deferred(_) | InferTy::Never | InferTy::Tainted => None,
    }
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
            Type::Result(ok, err) => {
                work.push(err);
                work.push(ok);
            }
            Type::MockResource | Type::MockBlockingResource => return false,
            Type::Deferred(_) | Type::Never | Type::Tainted => return false,
        }
    }
    true
}

fn domain_accepts(domain: &str, ty: &Type) -> bool {
    match domain {
        "domain:array" => matches!(ty, Type::Array(_)),
        "domain:copyable_data" => copyable(ty),
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
            Type::Result(ok, err) => {
                work.push(err);
                work.push(ok);
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
            Type::Array(inner) => work.push(inner),
            Type::Result(ok, err) => {
                work.push(err);
                work.push(ok);
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
            Type::Result(ok, err) => {
                work.push(err);
                work.push(ok);
            }
            Type::I64
            | Type::Bool
            | Type::Unit
            | Type::MockResource
            | Type::MockBlockingResource
            | Type::Deferred(_)
            | Type::Never
            | Type::Tainted => {}
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
    names::sort_related(&mut d);
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
    bottom_only: bool,
    // Only presence is tested by bind/union. The earliest span is retained
    // for deterministic propagation, but changing one Some(span) to another
    // Some(span) cannot alter acceptance or a diagnostic: neither site emits
    // this span (the equality copyability rule is §4.6).
    copyable_origin: Option<Span>,
}

struct PendingDeferredJoin {
    left: Type,
    right: Type,
    constructor: u64,
    left_origin: Span,
    found: Span,
    branch: bool,
    related_expected: bool,
    numeric_domain_owner: Option<(u64, u64)>,
    comparison_domain_if_unresolved: Option<PendingComparisonDomain>,
}

struct PendingComparisonDomain {
    left: Type,
    operator: Span,
    found: Span,
    domain: &'static str,
}

struct PendingDomain {
    variable: u64,
    operand: Span,
    domain: &'static str,
}

struct PendingDomainResult {
    variable: u64,
    domain: &'static str,
    call: Span,
    nominal: Type,
    expected: Option<Type>,
    expected_span: Option<Span>,
    depends_on: Option<Span>,
}

struct PendingBinaryDomain {
    variable: u64,
    operator: Span,
    found: Span,
    domain: &'static str,
    result: Span,
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
}

#[derive(Default)]
struct PendingOperandEvidence {
    domains: Vec<(u64, &'static str)>,
    projections: Vec<(u64, u64)>,
    results: Vec<Span>,
}

struct PendingProjection {
    base: u64,
    item: u64,
    base_span: Span,
    result_span: Span,
    selector: Expr,
    selector_variable: Option<u64>,
    checked_without_context: bool,
    selector_invalid: bool,
    expected: Option<Type>,
    expected_span: Option<Span>,
    related_expected: bool,
}

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

enum PendingConstraint {
    Constructor { child: u64, array: u64, span: Span },
    Join(Box<PendingDeferredJoin>),
    Domain(PendingDomain),
    DomainResult(Box<PendingDomainResult>),
    BinaryDomain(Box<PendingBinaryDomain>),
    GuardedComparison(Box<PendingGuardedComparison>),
    Projection(Box<PendingProjection>),
}

#[derive(Default)]
pub(crate) struct InferredTypes {
    pub bindings: HashMap<u64, ResolvedType>,
    pub expressions: HashMap<(u64, u64), ResolvedType>,
}

#[allow(clippy::vec_box)] // Shared semantic diagnostics retain the boxed representation.
pub(crate) struct TypeCheckReport {
    pub inferred: InferredTypes,
    pub errors: Vec<Box<Diagnostic>>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum ResolvedType {
    Concrete(SyntaxType),
    Never,
    Unresolved,
}

fn record_unresolved_sites(ast: &Ast, inferred: &mut InferredTypes) {
    fn expression(expr: &Expr, types: &mut InferredTypes) {
        types
            .expressions
            .entry(span_key(expr.span()))
            .or_insert(ResolvedType::Unresolved);
        match expr {
            Expr::Array(items, _) => {
                for item in items {
                    expression(item, types);
                }
            }
            Expr::Group(inner, _) | Expr::Unary { value: inner, .. } => expression(inner, types),
            Expr::Binary { left, right, .. } => {
                expression(left, types);
                expression(right, types);
            }
            Expr::Call { target, args, .. } => {
                expression(target, types);
                for arg in args {
                    expression(&arg.value, types);
                }
            }
            Expr::Index { target, index, .. } => {
                expression(target, types);
                expression(index, types);
            }
            Expr::If {
                condition,
                then_block,
                otherwise,
                ..
            } => {
                expression(condition, types);
                block(then_block, types);
                match otherwise.as_deref() {
                    Some(crate::frontend::parse::Else::Block(b)) => block(b, types),
                    Some(crate::frontend::parse::Else::If(e)) => expression(e, types),
                    None => {}
                }
            }
            Expr::Name(..) | Expr::Integer(..) | Expr::Bool(..) | Expr::Unit(..) => {}
        }
    }
    fn statement(stmt: &Statement, types: &mut InferredTypes) {
        match stmt {
            Statement::Let { value, .. } | Statement::Expr { value, .. } => {
                expression(value, types)
            }
            Statement::Assign { place, value, .. } => {
                expression(place, types);
                expression(value, types);
            }
            Statement::Return {
                value: Some(value), ..
            }
            | Statement::Value {
                value: Some(value), ..
            } => expression(value, types),
            Statement::If {
                condition,
                then_block,
                otherwise,
                ..
            } => {
                expression(condition, types);
                block(then_block, types);
                if let Some(other) = otherwise {
                    statement(other, types);
                }
            }
            Statement::While {
                condition, body, ..
            } => {
                expression(condition, types);
                block(body, types);
            }
            Statement::Block(b) => block(b, types),
            Statement::Return { value: None, .. } | Statement::Value { value: None, .. } => {}
        }
    }
    fn block(block: &Block, types: &mut InferredTypes) {
        for stmt in &block.statements {
            statement(stmt, types);
        }
    }
    for function in &ast.functions {
        block(&function.body, inferred);
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
    names::sort_related(&mut d);
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

#[allow(clippy::vec_box)] // Public semantic result keeps boxed diagnostics.
struct Checker<'a> {
    file: &'a str,
    tokens: Vec<Token>,
    resolved: &'a Resolved,
    locals: HashMap<u64, Known>,
    type_nodes: HashMap<u64, TypeNode>,
    // Entries own both immutable operands, so their pointer keys cannot be
    // reused. Only fully concrete pairs enter this cache.
    array_unify_cache: HashMap<(usize, usize), (Type, Type, Type, u64)>,
    expr_vars: HashMap<(u64, u64), u64>,
    expression_types: HashMap<(u64, u64), Type>,
    conditional_result_types: HashMap<(u64, u64), Type>,
    conditional_result_vars: Vec<(u64, Span, Type)>,
    next_variable: u64,
    failed_constructors: HashSet<u64>,
    // Root, scalar constraint, expected source, incompatible operand,
    // array-on-left, and branch join (NX-E1303 instead of NX-E0102).
    pending: Vec<PendingConstraint>,
    // A nested block can drain pending constraints while checking an operand.
    // This distinguishes an append-only suffix from a repartitioned queue.
    pending_generation: u64,
    // Avoid repartitioning the full collection after statements that did not
    // create a conditional projection.
    has_pending_projection: bool,
    // The + or - whose I64 context is currently distributed to this site.
    // Nested binary expressions own their internal constraints separately.
    numeric_domain_owner: Option<(u64, u64)>,
    pending_domain_calls: HashSet<(u64, u64)>,
    // §13.9: an operand/constraint site owns one diagnostic, regardless of
    // whether it is discovered immediately or by a deferred join.
    emitted_sites: HashSet<(u64, u64)>,
    solver_steps: u64,
    solver_limit: u64,
    // The first traversal discovers compatible constructor shapes without
    // spending the normative solver budget or emitting diagnostics. The
    // checking traversal owns both charges and diagnostic occurrences.
    planning: bool,
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
    deferred_join_roots: HashSet<u64>,
    branch_join_sites: HashMap<u64, Vec<(u64, u64)>>,
    recursive_join_roots: HashSet<u64>,
    block_depth: usize,
    top_statement_start: u64,
    top_statement_end: u64,
    active_if_span: Option<Span>,
    preseeded_children: HashSet<u64>,
    analysis_exhausted: bool,
    quota_error: Option<Box<Diagnostic>>,
    invalid: HashSet<u64>,
    deferred_exprs: HashSet<(u64, u64)>,
    divergent_exprs: HashSet<(u64, u64)>,
    failed_join_exprs: HashSet<(u64, u64)>,
    failed_projection_results: HashSet<(u64, u64)>,
    // A concrete context was rejected at this expression's own value
    // site (or one of its distributed normal continuations).
    context_rejections: HashSet<(u64, u64)>,
    // A tainted expression can still contribute a valid normal branch type
    // when that type came from a separate, checked continuation (§13.9).
    partial_normal_exprs: HashSet<(u64, u64)>,
    tainted: bool,
    result: Type,
    result_span: Option<Span>,
    errors: Vec<Box<Diagnostic>>,
}

impl Checker<'_> {
    fn intrinsic_assignment_type(&self, expression: &Expr) -> Option<Type> {
        // Only syntax with a determined, context-free constructor type is
        // evidence about this assignment. Cross-operand and branch joins
        // remain obligations of the checking pass (§4.1.2.2–4).
        match expression {
            Expr::Integer(text, _) if text.parse::<i64>().is_ok() => Some(Type::I64),
            Expr::Bool(..) => Some(Type::Bool),
            Expr::Unit(..) => Some(Type::Unit),
            Expr::Group(inner, _) => self.intrinsic_assignment_type(inner),
            Expr::Array(items, _) => {
                let first = self.intrinsic_assignment_type(items.first()?)?;
                if !copyable(&first) || contains_resource(&first) {
                    return None;
                }
                if items
                    .iter()
                    .skip(1)
                    .all(|item| self.intrinsic_assignment_type(item).as_ref() == Some(&first))
                {
                    Some(Type::Array(Arc::new(first)))
                } else {
                    None
                }
            }
            Expr::Name(_, span) => match self.resolved.uses.get(&span.start) {
                Some(ResolvedUse::Local(local)) => self
                    .locals
                    .get(&local.name_span.start)
                    .filter(|known| known.variable.is_none() && internal_state(&known.ty).is_none())
                    .map(|known| known.ty.clone()),
                _ => None,
            },
            _ => None,
        }
    }

    fn collect_planned_shapes(&mut self) -> bool {
        let mut changed = false;
        let eligible: Vec<_> = self.eligible_shapes.iter().copied().collect();
        // Count unresolved assigned peers once per component. Scanning every
        // eligible binding from every other binding makes independent arrays
        // quadratic under the same fixed-point plan.
        let mut unresolved_peers: HashMap<u64, (usize, u64)> = HashMap::new();
        let mut first_assigned_type: HashMap<u64, (u64, Type)> = HashMap::new();
        for other in &eligible {
            if let (Some(id), Some((ty, _)), Some(start)) = (
                self.locals.get(other).and_then(|known| known.variable),
                self.assignment_type_plan.get(other).cloned(),
                self.assignment_starts.get(other).copied(),
            ) {
                let root = self.root(id);
                let entry = first_assigned_type
                    .entry(root)
                    .or_insert_with(|| (start, ty.clone()));
                if start < entry.0 {
                    *entry = (start, ty);
                }
            }
            if self.assigned_bases.contains(other)
                && !self.assignment_type_plan.contains_key(other)
                && let Some(id) = self.locals.get(other).and_then(|known| known.variable)
            {
                let root = self.root(id);
                unresolved_peers
                    .entry(root)
                    .and_modify(|(count, _)| *count += 1)
                    .or_insert((1, *other));
            }
        }
        for site in eligible {
            let Some(known) = self.locals.get(&site).cloned() else {
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
                let node = &self.type_nodes[&current];
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
            self.concrete_plan.remove(&site);
            self.read_plan.remove(&site);
            let projected: Vec<_> = self.projected_bases.iter().copied().collect();
            let has_conditional_projection = projected
                .into_iter()
                .any(|base| chain.contains(&self.root(base)));
            let deferred_joins: Vec<_> = self.deferred_join_roots.iter().copied().collect();
            let has_deferred_join = deferred_joins
                .into_iter()
                .any(|root| chain.contains(&self.root(root)));
            let recursive_joins: Vec<_> = self.recursive_join_roots.iter().copied().collect();
            let has_recursive_join = recursive_joins
                .into_iter()
                .any(|root| chain.contains(&self.root(root)));
            let nonbranch_join_roots: Vec<_> = self
                .pending_joins()
                .filter(|join| !join.branch)
                .map(|join| join.constructor)
                .collect();
            let has_recursive_nonbranch_join = has_recursive_join
                && nonbranch_join_roots
                    .into_iter()
                    .any(|root| chain.contains(&self.root(root)));
            if has_recursive_join && !self.assigned_bases.contains(&site) {
                // Reject the recursive equality at its own occurrence. A
                // later constructor cannot change that earlier occurs check.
                changed |= self.shape_plan.remove(&site).is_some();
                continue;
            }
            let resolved = self
                .prefix_concrete
                .get(&site)
                .cloned()
                .or_else(|| self.concrete_evidence(known.variable.unwrap()));
            let read_type = self
                .assignment_type_plan
                .get(&site)
                .map(|(ty, _)| ty.clone())
                .or_else(|| resolved.as_ref().map(|evidence| evidence.ty.clone()));
            let prefix = match self.pre_assignment_snapshot.get(&site) {
                Some(snapshot) => snapshot.clone(),
                None => self.prefix_concrete.get(&site).cloned(),
            };
            let conflicts_prefix = self
                .assignment_type_plan
                .get(&site)
                .is_some_and(|(ty, _)| prefix.as_ref().is_some_and(|prior| &prior.ty != ty));
            if self.assigned_bases.contains(&site)
                && !has_conditional_projection
                && has_deferred_join
                && !conflicts_prefix
                && let Some(ty) = read_type
                && let Some(&assignment_start) = self.assignment_starts.get(&site)
            {
                let joins: Vec<_> = self
                    .branch_join_sites
                    .iter()
                    .map(|(id, ranges)| (*id, ranges.clone()))
                    .collect();
                let mut blocked = Vec::new();
                for (root, ranges) in joins {
                    if chain.contains(&self.root(root)) {
                        blocked.extend(ranges);
                    }
                }
                if !blocked.is_empty() {
                    blocked.sort_unstable();
                    blocked.dedup();
                    self.read_plan.insert(
                        site,
                        ReadPlan {
                            ty,
                            blocked,
                            assignment_start,
                        },
                    );
                }
            }
            if self.assigned_bases.contains(&site) {
                if let Some(source_type) = self.assignment_type_plan.get(&site).cloned() {
                    let first_assignment =
                        self.first_assignment_rhs_starts.get(&site) == Some(&source_type.1.start);
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
                        && self.assignment_starts.get(&site).is_some_and(|start| {
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
                        self.concrete_plan.insert(site, prior);
                    } else if first_assignment
                        && !has_recursive_nonbranch_join
                        && (!has_conditional_projection || !origins.is_empty())
                        && !conflicts_prefix
                        && !conflicts_shape
                        && !conflicts_earlier_assignment
                        && !unresolved_peer
                        && !self.nested_assignment_sites.contains(&site)
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
                        self.concrete_plan.insert(site, planned_source);
                    }
                } else if !has_deferred_join && let Some(concrete) = resolved {
                    self.concrete_plan.insert(site, concrete);
                }
            }
            changed |= self.shape_plan.get(&site) != Some(&origins);
            self.shape_plan.insert(site, origins);
        }
        changed
    }

    fn seed_planned_shape(&mut self, site: u64, id: u64) {
        let Some(origins) = self.shape_plan.get(&site).cloned() else {
            return;
        };
        let mut current = id;
        for origin in origins {
            current = self.root(current);
            if let Some(child) = self.type_nodes[&current].array_child {
                current = child;
                continue;
            }
            let child = self.next_variable;
            self.next_variable += 1;
            self.type_nodes.insert(
                child,
                TypeNode {
                    parent: child,
                    size: 1,
                    concrete: None,
                    concrete_intrinsic: false,
                    array_child: None,
                    array_origin: None,
                    array_constructed: false,
                    bottom_only: false,
                    copyable_origin: Some(origin.clone()),
                },
            );
            self.type_nodes.get_mut(&current).unwrap().array_child = Some(child);
            self.type_nodes.get_mut(&current).unwrap().array_origin = Some(origin);
            self.preseeded_children.insert(child);
            current = child;
        }
        if !self.planning
            && let Some(evidence) = self.concrete_plan.get(&site).cloned()
        {
            let mut ty = evidence.ty;
            for _ in 0..=128 {
                let Type::Array(child) = ty else {
                    break;
                };
                ty = child.as_ref().clone();
            }
            current = self.root(current);
            if self.type_nodes[&current].concrete.is_none() && internal_state(&ty).is_none() {
                let node = self.type_nodes.get_mut(&current).unwrap();
                node.concrete = Some((ty, evidence.origin));
                node.concrete_intrinsic = evidence.intrinsic;
            }
        }
    }

    fn pending_constructors(&self) -> Vec<(u64, u64, Span)> {
        self.pending
            .iter()
            .filter_map(|entry| match entry {
                PendingConstraint::Constructor { child, array, span } => {
                    Some((*child, *array, span.clone()))
                }
                _ => None,
            })
            .collect()
    }
    fn pending_joins(&self) -> impl Iterator<Item = &PendingDeferredJoin> {
        self.pending.iter().filter_map(|entry| match entry {
            PendingConstraint::Join(join) => Some(join.as_ref()),
            _ => None,
        })
    }

    fn pending_joins_mut(&mut self) -> impl Iterator<Item = &mut PendingDeferredJoin> {
        self.pending.iter_mut().filter_map(|entry| match entry {
            PendingConstraint::Join(join) => Some(join.as_mut()),
            _ => None,
        })
    }

    fn pending_join_count(&self) -> usize {
        self.pending_joins().count()
    }

    fn truncate_pending_joins(&mut self, length: usize) {
        let mut seen = 0;
        let before = self.pending.len();
        self.pending.retain(|entry| match entry {
            PendingConstraint::Join(_) => {
                seen += 1;
                seen <= length
            }
            _ => true,
        });
        if self.pending.len() != before {
            self.pending_generation = self.pending_generation.wrapping_add(1);
        }
    }

    fn pending_domain_count(&self) -> usize {
        self.pending
            .iter()
            .filter(|entry| matches!(entry, PendingConstraint::Domain(_)))
            .count()
    }

    fn pending_domains(&self) -> impl Iterator<Item = &PendingDomain> {
        self.pending.iter().filter_map(|entry| match entry {
            PendingConstraint::Domain(domain) => Some(domain),
            _ => None,
        })
    }

    fn take_pending_projections(&mut self) -> Vec<PendingConstraint> {
        if !self.has_pending_projection {
            return Vec::new();
        }
        self.has_pending_projection = false;
        self.take_pending_kind(|entry| match entry {
            entry @ PendingConstraint::Projection(_) => Ok(entry),
            other => Err(other),
        })
    }

    fn take_pending_kind<T>(
        &mut self,
        mut select: impl FnMut(PendingConstraint) -> Result<T, PendingConstraint>,
    ) -> Vec<T> {
        let mut chosen = Vec::new();
        let mut retained = Vec::new();
        for entry in std::mem::take(&mut self.pending) {
            match select(entry) {
                Ok(value) => chosen.push(value),
                Err(entry) => retained.push(entry),
            }
        }
        self.pending = retained;
        if !chosen.is_empty() {
            self.pending_generation = self.pending_generation.wrapping_add(1);
        }
        chosen
    }

    fn representative(&self, id: u64) -> u64 {
        let mut current = id;
        while self.type_nodes[&current].parent != current {
            current = self.type_nodes[&current].parent;
        }
        current
    }

    fn projection_expected(&mut self, base: u64) -> Option<(Type, Span)> {
        let mut current = self.root(base);
        let mut depth = 0;
        loop {
            let next = self.pending.iter().find_map(|entry| match entry {
                PendingConstraint::Projection(projection)
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
        match value {
            Expr::Array(..) => true,
            Expr::Group(inner, _) | Expr::Unary { value: inner, .. } => {
                self.expression_has_array_root(inner)
            }
            Expr::Name(_, span) => match self.resolved.uses.get(&span.start) {
                Some(ResolvedUse::Local(local)) => {
                    let known = self.locals.get(&local.name_span.start).cloned();
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
            Expr::Call { span, .. } => {
                let binding = &self.resolved.calls[&span.start];
                names::signature(self.resolved, &binding.target).is_some_and(|signature| {
                    matches!(Type::from(&signature.result), Type::Array(_))
                })
            }
            _ => false,
        }
    }

    fn type_shape_hint(ty: &Type) -> ShapeHint {
        match ty {
            Type::Array(child) => ShapeHint::Array(Box::new(Self::type_shape_hint(child))),
            Type::Deferred(_) | Type::Never | Type::Tainted => ShapeHint::Unknown,
            scalar => ShapeHint::Scalar(scalar.clone()),
        }
    }

    fn variable_shape_hint(&mut self, id: u64) -> ShapeHint {
        let mut current = self.root(id);
        let mut depth = 0;
        let leaf = loop {
            let node = &self.type_nodes[&current];
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
        match value {
            Expr::Array(items, _) => {
                let first = items
                    .first()
                    .map(|item| self.expression_shape_hint(item))
                    .unwrap_or(ShapeHint::Unknown);
                if items
                    .iter()
                    .skip(1)
                    .all(|item| self.expression_shape_hint(item) == first)
                {
                    ShapeHint::Array(Box::new(first))
                } else {
                    ShapeHint::Unknown
                }
            }
            Expr::Group(inner, _) => self.expression_shape_hint(inner),
            Expr::Name(_, span) => match self.resolved.uses.get(&span.start) {
                Some(ResolvedUse::Local(local)) => {
                    let known = self.locals.get(&local.name_span.start).cloned();
                    match known {
                        Some(Known {
                            variable: Some(id), ..
                        }) => self.variable_shape_hint(id),
                        Some(known) => Self::type_shape_hint(&known.ty),
                        None => ShapeHint::Unknown,
                    }
                }
                _ => ShapeHint::Unknown,
            },
            Expr::Integer(..) => ShapeHint::Scalar(Type::I64),
            Expr::Bool(..) => ShapeHint::Scalar(Type::Bool),
            Expr::Unit(..) => ShapeHint::Scalar(Type::Unit),
            Expr::Index { target, .. } => match self.expression_shape_hint(target) {
                ShapeHint::Array(child) => *child,
                _ => ShapeHint::Unknown,
            },
            _ => ShapeHint::Unknown,
        }
    }

    fn shape_hints_conflict(left: &ShapeHint, right: &ShapeHint) -> bool {
        match (left, right) {
            (ShapeHint::Array(left), ShapeHint::Array(right)) => {
                Self::shape_hints_conflict(left, right)
            }
            (ShapeHint::Scalar(left), ShapeHint::Scalar(right)) => left != right,
            (ShapeHint::Array(_), ShapeHint::Scalar(_))
            | (ShapeHint::Scalar(_), ShapeHint::Array(_)) => true,
            _ => false,
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
                let PendingConstraint::Projection(mut projection) = entry else {
                    unreachable!("projection partition")
                };
                let root = self.root(projection.base);
                let element = self.type_nodes[&root].array_child;
                let base = self.concrete(projection.base);
                let indexable = element.is_some();
                if !finalize && !indexable && base.is_none() {
                    if !projection.checked_without_context
                        && (self.planning || Self::context_stable_selector(&projection.selector))
                    {
                        // The selector has independent constraints of its own.
                        // Synthesize them in source order even though its I64
                        // context depends on the still-unknown base shape.
                        let prior_taint = self.tainted;
                        self.tainted = false;
                        let selector = self.expr(&projection.selector, None, None);
                        projection.selector_invalid |= self.tainted || selector.is_err();
                        self.tainted = prior_taint;
                        if let Err(error) = selector {
                            if error.operation == Operation::AnalysisLimit {
                                return Err(error);
                            }
                            self.record_error(error);
                        }
                        projection.checked_without_context = true;
                        next.push(PendingConstraint::Projection(projection));
                        advanced = true;
                        continue;
                    }
                    next.push(PendingConstraint::Projection(projection));
                    continue;
                }
                if !indexable && base.is_none() && projection.checked_without_context {
                    next.push(PendingConstraint::Projection(projection));
                    continue;
                }
                let prior_taint = self.tainted;
                self.tainted = false;
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
                        if self.type_nodes[&selector_root].array_child.is_some() {
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
                let selector = self.expr(
                    &projection.selector,
                    (indexable && later_conflict.is_none()).then_some(&Type::I64),
                    None,
                );
                if let Some((ty, origin)) = &later_conflict {
                    self.record_error(mismatch(origin.clone(), origin.clone(), None, "I64", ty));
                }
                let selector_invalid = projection.selector_invalid
                    || later_conflict.is_some()
                    || self.tainted
                    || selector.is_err();
                let result_site = span_key(&projection.result_span);
                if selector_invalid {
                    self.failed_projection_results.insert(result_site);
                }
                self.tainted = prior_taint;
                if let Err(error) = selector {
                    if error.operation == Operation::AnalysisLimit {
                        return Err(error);
                    }
                    self.record_error(error);
                }
                if let Some(child) = element {
                    if !selector_invalid {
                        let result = self.unify(
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
                            self.failed_projection_results.insert(result_site);
                            self.record_error(error);
                        }
                        if let Some(expected) = projection.expected {
                            let found = projection.result_span;
                            let result = self.unify_with_expected_origin(
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
                                self.failed_projection_results.insert(result_site);
                                self.record_error(error);
                            }
                        }
                    }
                } else if let Some((ty, _)) = base {
                    self.failed_projection_results.insert(result_site);
                    self.record_error(self.domain_diagnostic(
                        projection.base_span,
                        &ty,
                        "domain:array",
                    ));
                } else {
                    projection.checked_without_context = true;
                    projection.selector_invalid = selector_invalid;
                    next.push(PendingConstraint::Projection(projection));
                }
                advanced = true;
            }
            next.append(&mut self.take_pending_projections());
            if !advanced {
                self.has_pending_projection = !next.is_empty();
                self.pending.extend(next);
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
            self.type_nodes[&root].array_child.is_some()
        });
        if domain == "domain:array" && array_constructor {
            // `[]` has already fixed the Array constructor. Its element may
            // be unresolved, but that cannot invalidate Len's array domain.
            return Ok(());
        }
        if internal_state(got).is_some() {
            if let Some(id) = self.expression_variable(operand) {
                self.pending.push(PendingConstraint::Domain(PendingDomain {
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
            self.defer_domain_result(
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
    fn defer_domain_result(
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
        self.pending_domain_calls.insert(span_key(call));
        // An established Array constructor can never satisfy the scalar
        // I64 domain, even while its element is unresolved. Delay the
        // serializable diagnostic, but stop offering the nominal result as
        // a witness to siblings now (§§4.1.2.2, 4.1.2.4–5).
        let root = self.root(variable);
        if domain == "I64" && self.type_nodes[&root].array_child.is_some() {
            self.failed_join_exprs.insert(span_key(call));
        }
        self.pending.push(PendingConstraint::DomainResult(Box::new(
            PendingDomainResult {
                variable,
                domain,
                call: call.clone(),
                nominal: nominal.clone(),
                expected: expected.cloned(),
                expected_span: expected_span.cloned(),
                depends_on: depends_on.cloned(),
            },
        )));
    }

    fn pending_operation_guard(&self, operand: &Expr) -> Option<(u64, &'static str)> {
        let site = span_key(&Self::primary_span(operand));
        self.pending.iter().find_map(|pending| match pending {
            PendingConstraint::DomainResult(result) if span_key(&result.call) == site => {
                Some((result.variable, result.domain))
            }
            PendingConstraint::BinaryDomain(result) if span_key(&result.result) == site => {
                Some((result.variable, result.domain))
            }
            PendingConstraint::GuardedComparison(result) if span_key(&result.result) == site => {
                Some((result.variable, result.domain))
            }
            _ => None,
        })
    }

    fn pending_operand_evidence(
        &self,
        operand: &Expr,
        pending_start: usize,
        pending_generation: u64,
    ) -> Option<PendingOperandEvidence> {
        let span = Self::primary_span(operand);
        let inside = |site: &Span| span.start <= site.start && site.end <= span.end;
        let mut evidence = PendingOperandEvidence::default();
        // Source containment remains authoritative if a nested block has
        // repartitioned the queue since this operand began.
        let entries = if self.pending_generation == pending_generation {
            self.pending.get(pending_start..).unwrap_or(&self.pending)
        } else {
            &self.pending
        };
        for pending in entries {
            match pending {
                PendingConstraint::Domain(domain) if inside(&domain.operand) => {
                    evidence.domains.push((domain.variable, domain.domain));
                }
                PendingConstraint::DomainResult(result) if inside(&result.call) => {
                    evidence.domains.push((result.variable, result.domain));
                    evidence.results.push(result.call.clone());
                }
                PendingConstraint::BinaryDomain(result) if inside(&result.result) => {
                    evidence.domains.push((result.variable, result.domain));
                    evidence.results.push(result.result.clone());
                }
                PendingConstraint::GuardedComparison(result) if inside(&result.result) => {
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
                PendingConstraint::Projection(result) if inside(&result.result_span) => {
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
        self.expression_types
            .get(&site)
            .filter(|ty| **ty != Type::Tainted)
            .cloned()
            .or_else(|| {
                self.pending.iter().find_map(|pending| match pending {
                    PendingConstraint::DomainResult(result) if span_key(&result.call) == site => {
                        Some(result.nominal.clone())
                    }
                    PendingConstraint::GuardedComparison(result)
                        if span_key(&result.result) == site =>
                    {
                        Some(Type::Bool)
                    }
                    _ => None,
                })
            })
    }

    fn pending_nominal_available(&mut self, operand: &Expr) -> bool {
        let Some((variable, _)) = self.pending_operation_guard(operand) else {
            return false;
        };
        let root = self.root(variable);
        let child = self.type_nodes[&root].array_child;
        child.is_none_or(|child| self.concrete(child).is_some())
    }

    fn record_error(&mut self, error: Box<Diagnostic>) {
        if self.planning {
            return;
        }
        if error.operation != Operation::AnalysisLimit
            && let Some(span) = &error.primary_span
        {
            // §4.7 excludes a second join diagnosis at an operand that
            // already failed its contextual check. §4.1.1 suppresses
            // derived errors, while other spans remain independent.
            if !self.emitted_sites.insert((span.start, span.end)) {
                return;
            }
        }
        self.errors.push(error);
    }

    fn suppress_result_dependents(&mut self, results: &HashSet<(u64, u64)>) {
        // A nominal result with an unresolved or failed domain is not a
        // witness for its consumers (§4.1.2.5). The same site rule applies
        // at ordinary closure and at a denied solver charge (§11.1).
        self.errors.retain(|error| {
            let result_site = error.primary_span.as_ref().map(span_key);
            let expected_witness = error
                .related_spans
                .iter()
                .find(|relation| relation.role == RelatedRole::ExpectedType)
                .map(|relation| span_key(&relation.span));
            !result_site.is_some_and(|site| results.contains(&site))
                && !expected_witness.is_some_and(|site| results.contains(&site))
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
        let id = self.next_variable;
        self.next_variable += 1;
        self.type_nodes.insert(
            id,
            TypeNode {
                parent: id,
                size: 1,
                concrete: None,
                concrete_intrinsic: false,
                array_child: None,
                array_origin: None,
                array_constructed: false,
                bottom_only: false,
                copyable_origin: None,
            },
        );
        Ok(id)
    }

    fn require_array(&mut self, id: u64, origin: &Span) -> Result<u64, Box<Diagnostic>> {
        self.charge_solver(origin)?;
        let root = self.root(id);
        if let Some(child) = self.type_nodes[&root].array_child {
            if self.preseeded_children.remove(&child) {
                self.charge_solver(origin)?;
            }
            return Ok(child);
        }
        let child = self.new_variable(origin)?;
        self.type_nodes.get_mut(&root).unwrap().array_child = Some(child);
        self.type_nodes.get_mut(&root).unwrap().array_origin = Some(origin.clone());
        // Every newly required element inherits the array's copyable-data
        // domain. Explicit array constructors call require_copyable on their
        // element instead (§4.1.1); removing only the stored origin cannot
        // matter once that child already has a copyability origin.
        self.type_nodes.get_mut(&child).unwrap().copyable_origin = Some(origin.clone());
        self.check_one_inferred_depth(root, origin)?;
        Ok(child)
    }

    fn require_copyable(&mut self, id: u64, origin: &Span) -> Result<(), Box<Diagnostic>> {
        self.charge_solver(origin)?;
        let root = self.root(id);
        if let Some((ty, _)) = self.concrete(root)
            && !copyable(&ty)
        {
            return Err(mismatch(
                origin.clone(),
                origin.clone(),
                None,
                "domain:copyable_data",
                &ty,
            ));
        }
        // Only presence of this domain is consulted by unification. A later
        // failure uses its current operand as primary, so choosing the first
        // recorded origin span cannot change a source-level diagnostic.
        let previous = self.type_nodes[&root].copyable_origin.clone();
        if previous
            .as_ref()
            .is_none_or(|span| origin.start < span.start)
        {
            self.type_nodes.get_mut(&root).unwrap().copyable_origin = Some(origin.clone());
        }
        // require_array and explicit constructors already gave this child
        // the copyable domain. Requiring it again preserves that invariant
        // for manually built solver states, with no new source-level result.
        if let Some(child) = self.type_nodes[&root].array_child {
            self.require_copyable(child, origin)?;
        }
        Ok(())
    }

    fn root(&mut self, id: u64) -> u64 {
        let mut root = id;
        while self.type_nodes[&root].parent != root {
            root = self.type_nodes[&root].parent;
        }
        let mut current = id;
        while current != root {
            let next = self.type_nodes[&current].parent;
            self.type_nodes.get_mut(&current).unwrap().parent = root;
            current = next;
        }
        root
    }

    fn concrete(&mut self, id: u64) -> Option<(Type, Span)> {
        let mut current = self.root(id);
        let mut wrappers = 0;
        loop {
            if let Some((mut ty, origin)) = self.type_nodes[&current].concrete.clone() {
                for _ in 0..wrappers {
                    ty = Type::Array(Arc::new(ty));
                }
                return Some((ty, origin));
            }
            let child = self.type_nodes[&current].array_child?;
            wrappers += 1;
            if wrappers > 128 {
                // The structural guard reports NX-E0003 from a source origin.
                return None;
            }
            current = self.root(child);
        }
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
            let node = &self.type_nodes[&current];
            if node.concrete.is_some() {
                return node.concrete_intrinsic;
            }
            let Some(child) = node.array_child else {
                return false;
            };
            current = self.root(child);
        }
        false
    }

    fn retain_assignment_origin(&mut self, id: u64, origin: &Span) {
        let mut current = self.root(id);
        for _ in 0..=128 {
            if let Some((ty, _)) = self.type_nodes[&current].concrete.clone() {
                self.type_nodes.get_mut(&current).unwrap().concrete = Some((ty, origin.clone()));
                self.type_nodes
                    .get_mut(&current)
                    .unwrap()
                    .concrete_intrinsic = false;
                return;
            }
            let Some(child) = self.type_nodes[&current].array_child else {
                return;
            };
            current = self.root(child);
        }
    }

    fn type_determined(&mut self, ty: &Type) -> bool {
        match ty {
            Type::Deferred(id) => {
                let root = self.root(*id);
                let node = &self.type_nodes[&root];
                node.concrete.is_some() || node.array_child.is_some()
            }
            Type::Never | Type::Tainted => false,
            _ => true,
        }
    }

    fn resolved_syntax_type(&mut self, ty: &Type) -> Option<SyntaxType> {
        match ty {
            Type::Deferred(id) => {
                let (resolved, _) = self.concrete(*id)?;
                syntax_type(&resolved)
            }
            Type::Array(inner) => Some(SyntaxType::Array(Box::new(
                self.resolved_syntax_type(inner)?,
            ))),
            Type::Result(ok, err) => Some(SyntaxType::Result(
                Box::new(self.resolved_syntax_type(ok)?),
                Box::new(self.resolved_syntax_type(err)?),
            )),
            _ => syntax_type(ty),
        }
    }

    fn bind(
        &mut self,
        id: u64,
        ty: Type,
        origin: Span,
        found: Span,
        intrinsic: bool,
    ) -> Result<(), Box<Diagnostic>> {
        // A constructor containing a variable is a structural constraint,
        // never a concrete type stored in a union-find representative.
        if let Type::Deferred(other) = ty {
            return self.union(id, other, found);
        }
        if let Type::Array(element) = ty {
            let root = self.root(id);
            // An earlier conditional selector becomes active if this Array
            // equality succeeds. If its selector is the same monomorphic
            // variable, the required I64 and the proposed Array cannot both
            // hold. Reject the later equality at its source occurrence
            // before changing the representative (§4.1.2.1–3).
            let conditional_selectors: Vec<_> = self
                .pending
                .iter()
                .filter_map(|entry| match entry {
                    PendingConstraint::Projection(projection) => projection
                        .selector_variable
                        .map(|selector| (projection.base, selector)),
                    _ => None,
                })
                .collect();
            if conditional_selectors
                .into_iter()
                .any(|(base, selector)| self.root(base) == root && self.root(selector) == root)
            {
                return Err(mismatch(
                    found.clone(),
                    found,
                    None,
                    "I64",
                    &Type::Array(element),
                ));
            }
            if let Some((previous, previous_origin)) = self.type_nodes[&root].concrete.clone() {
                let expected_label =
                    pretty(&previous).ok_or_else(|| unresolved_type(found.clone()))?;
                return Err(mismatch(
                    found.clone(),
                    found,
                    (!self.origin_intrinsic(root)).then_some(previous_origin),
                    &expected_label,
                    &Type::Array(element),
                ));
            }
            let whole_expected = self.concrete(root).map(|(ty, _)| ty);
            let whole_found = Type::Array(element.clone());
            let child = self.require_array(id, &found)?;
            return self
                .bind(child, element.as_ref().clone(), origin, found, intrinsic)
                .map_err(|error| {
                    if let Some(expected) = &whole_expected {
                        array_mismatch(error, expected, &whole_found)
                    } else {
                        error
                    }
                });
        }
        self.charge_solver(&found)?;
        let root = self.root(id);
        if self.type_nodes[&root].copyable_origin.is_some() && !copyable(&ty) {
            self.fail_constructed_dependents(root);
            return Err(mismatch(
                found.clone(),
                found,
                None,
                "domain:copyable_data",
                &ty,
            ));
        }
        if let Some(child) = self.type_nodes[&root].array_child {
            let Type::Array(element) = &ty else {
                if (self.type_nodes[&root].array_constructed
                    || self.type_nodes[&root]
                        .array_child
                        .is_some_and(|child| self.preseeded_children.contains(&child)))
                    && self.concrete(root).is_none()
                {
                    // §4.1.1 considers all local constraints, irrespective
                    // of solver order. A later constraint may determine the
                    // element and turn this into a concrete §13.15 mismatch.
                    self.defer_shape_conflict(
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
                    return Ok(());
                }
                self.failed_constructors.insert(root);
                return Err(self.array_domain_conflict(root, &ty, found));
            };
            self.bind(
                child,
                element.as_ref().clone(),
                origin.clone(),
                found.clone(),
                intrinsic,
            )?;
        }
        if let Some((previous, previous_origin)) = self.concrete(root) {
            if previous != ty {
                let previous_label =
                    pretty(&previous).ok_or_else(|| unresolved_type(found.clone()))?;
                return Err(mismatch(
                    found.clone(),
                    found,
                    (!self.origin_intrinsic(root)).then_some(previous_origin),
                    &previous_label,
                    &ty,
                ));
            }
        } else {
            self.type_nodes.get_mut(&root).unwrap().concrete = Some((ty, origin));
            self.type_nodes.get_mut(&root).unwrap().concrete_intrinsic = intrinsic;
        }
        Ok(())
    }

    fn fail_constructed_dependents(&mut self, element: u64) {
        // §13.9: an array whose element already failed a concrete constraint
        // must not add an unresolved-constructor consequence at function end.
        for (child, array, _) in self.pending_constructors() {
            let child_root = self.root(child);
            if child_root == element && self.type_nodes[&child_root].bottom_only {
                let array_root = self.root(array);
                self.failed_constructors.insert(array_root);
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
        if self.type_nodes[&root].array_constructed {
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
        let origin = self.type_nodes[&root].array_origin.clone().unwrap_or(found);
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
            let node = &self.type_nodes[&current];
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

    fn defer_shape_conflict(
        &mut self,
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
            self.failed_constructors.insert(constructor);
        }
        if policy.branch {
            self.deferred_join_roots.insert(constructor);
            let scope = self
                .active_if_span
                .as_ref()
                .map_or((self.top_statement_start, self.top_statement_end), |span| {
                    (span.start, span.end)
                });
            self.branch_join_sites
                .entry(constructor)
                .or_default()
                .push(scope);
        }
        self.pending
            .push(PendingConstraint::Join(Box::new(PendingDeferredJoin {
                left,
                right,
                constructor,
                left_origin,
                found,
                branch: policy.branch,
                related_expected: policy.related_expected,
                numeric_domain_owner: self.numeric_domain_owner,
                comparison_domain_if_unresolved: None,
            })));
    }

    fn resolved_join_side(&mut self, ty: Type) -> Option<Type> {
        match ty {
            Type::Deferred(id) => self.concrete(id).map(|(ty, _)| ty),
            Type::Array(child) => Some(Type::Array(Arc::new(
                self.resolved_join_side(child.as_ref().clone())?,
            ))),
            Type::Result(ok, err) => Some(Type::Result(
                Arc::new(self.resolved_join_side(ok.as_ref().clone())?),
                Arc::new(self.resolved_join_side(err.as_ref().clone())?),
            )),
            Type::Never | Type::Tainted => None,
            other => Some(other),
        }
    }

    /// The single structural unification operation used by source joins.
    /// Never contributes no constraint and taint contributes no new error.
    fn unify(
        &mut self,
        left: Type,
        right: Type,
        left_origin: Span,
        right_origin: Span,
        found: Span,
        branch: bool,
    ) -> Result<Type, Box<Diagnostic>> {
        self.unify_with_expected_origin(
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

    fn unify_with_expected_origin(
        &mut self,
        left: Type,
        right: Type,
        left_origin: Span,
        right_origin: Span,
        found: Span,
        policy: UnifyPolicy,
    ) -> Result<Type, Box<Diagnostic>> {
        let UnifyPolicy {
            branch,
            related_expected,
        } = policy;
        // §11.1 charges each logical pair, including every nested array
        // element compared while resolving one source-level constraint.
        self.charge_solver(&found)?;
        match (left, right) {
            (Type::Tainted, _) | (_, Type::Tainted) => Ok(Type::Tainted),
            (Type::Never, other) | (other, Type::Never) => Ok(other),
            (Type::Deferred(a), Type::Deferred(b)) => {
                if branch
                    && let (Some((left, _)), Some((right, _))) =
                        (self.concrete(a), self.concrete(b))
                    && left != right
                {
                    // A branch may make its earlier deferred operand concrete
                    // while evaluating the later continuation (§4.7).
                    return Err(
                        compare_source_site(&left, &right, left_origin, found, policy)
                            .expect("distinct concrete branch types"),
                    );
                }
                // A concrete scalar cannot be united with a still unresolved
                // literal-array child. Keep both whole roots until later
                // constraints determine the child (§§4.1.1, 4.7, 13.17).
                // This check precedes union, which otherwise partly changes
                // child representatives before discovering the contradiction.
                let l = self.concrete(a);
                let r = self.concrete(b);
                let constructor = match (&l, &r) {
                    (Some((ty, _)), None) => {
                        self.unresolved_constructor_shape_conflict(b, ty, &found)?
                    }
                    (None, Some((ty, _))) => {
                        self.unresolved_constructor_shape_conflict(a, ty, &found)?
                    }
                    _ => None,
                };
                if let Some(constructor) = constructor {
                    self.charge_solver(&found)?;
                    self.defer_shape_conflict(
                        Type::Deferred(a),
                        Type::Deferred(b),
                        constructor,
                        left_origin,
                        found,
                        policy,
                    );
                    return Ok(Type::Deferred(self.root(a)));
                }
                let previous = self.pending_join_count();
                match self.union(a, b, found.clone()) {
                    Ok(()) => {}
                    Err(error) if error.code == "NX-E0103" => {
                        // ADR §4.1.2.2: a recursive equality is rejected at
                        // this occurrence, but later compatible constraints
                        // can complete both sides for a serializable M/J.
                        let constructor = self.root(a);
                        self.deferred_join_roots.insert(constructor);
                        self.recursive_join_roots.insert(constructor);
                        self.pending
                            .push(PendingConstraint::Join(Box::new(PendingDeferredJoin {
                                left: Type::Deferred(a),
                                right: Type::Deferred(b),
                                constructor,
                                left_origin,
                                found,
                                branch,
                                related_expected,
                                numeric_domain_owner: self.numeric_domain_owner,
                                comparison_domain_if_unresolved: None,
                            })));
                        return Ok(Type::Deferred(constructor));
                    }
                    Err(error) => return Err(error),
                }
                for join in self.pending_joins_mut().skip(previous) {
                    join.left = Type::Deferred(a);
                    join.right = Type::Deferred(b);
                    join.left_origin = left_origin.clone();
                    join.branch = branch;
                    join.related_expected = related_expected;
                }
                Ok(Type::Deferred(self.root(a)))
            }
            (Type::Deferred(id), other) => {
                if branch
                    && internal_state(&other).is_none()
                    && let Some((left, _)) = self.concrete(id)
                    && left != other
                {
                    return Err(
                        compare_source_site(&left, &other, left_origin, found, policy)
                            .expect("distinct concrete branch types"),
                    );
                }
                if internal_state(&other).is_none()
                    && let Some(constructor) =
                        self.unresolved_constructor_shape_conflict(id, &other, &found)?
                {
                    self.charge_solver(&found)?;
                    self.defer_shape_conflict(
                        Type::Deferred(id),
                        other,
                        constructor,
                        left_origin,
                        found,
                        policy,
                    );
                    return Ok(Type::Deferred(self.root(id)));
                }
                let previous = self.pending_join_count();
                self.bind(id, other.clone(), right_origin, found, !related_expected)?;
                for join in self.pending_joins_mut().skip(previous) {
                    join.left = Type::Deferred(id);
                    join.right = other.clone();
                    join.left_origin = left_origin.clone();
                    join.branch = branch;
                    join.related_expected = related_expected;
                }
                Ok(self
                    .concrete(id)
                    .map_or(Type::Deferred(self.root(id)), |(ty, _)| ty))
            }
            (other, Type::Deferred(id)) => {
                if internal_state(&other).is_none()
                    && let Some((right, _)) = self.concrete(id)
                    && other != right
                {
                    return Err(
                        compare_source_site(&other, &right, left_origin, found, policy)
                            .expect("distinct concrete source types"),
                    );
                }
                if internal_state(&other).is_none()
                    && let Some(constructor) =
                        self.unresolved_constructor_shape_conflict(id, &other, &found)?
                {
                    self.charge_solver(&found)?;
                    self.defer_shape_conflict(
                        other,
                        Type::Deferred(id),
                        constructor,
                        left_origin,
                        found,
                        policy,
                    );
                    return Ok(Type::Deferred(self.root(id)));
                }
                let previous = self.pending_join_count();
                self.bind(
                    id,
                    other.clone(),
                    left_origin.clone(),
                    found,
                    !related_expected,
                )?;
                for join in self.pending_joins_mut().skip(previous) {
                    join.left = other.clone();
                    join.right = Type::Deferred(id);
                    join.left_origin = left_origin.clone();
                    join.branch = branch;
                    join.related_expected = related_expected;
                }
                Ok(self
                    .concrete(id)
                    .map_or(Type::Deferred(self.root(id)), |(ty, _)| ty))
            }
            (Type::Array(a), Type::Array(b)) => {
                let key = (Arc::as_ptr(&a) as usize, Arc::as_ptr(&b) as usize);
                if let Some((_, _, resolved, inner_pairs)) =
                    self.array_unify_cache.get(&key).cloned()
                {
                    // §11.8, point 1 counts every logical pair even when a cache
                    // avoids revisiting the same immutable concrete trees.
                    for _ in 0..inner_pairs {
                        self.charge_solver(&found)?;
                    }
                    return Ok(resolved.clone());
                }
                let before_inner = self.solver_steps;
                let expected = Type::Array(a.clone());
                let found_type = Type::Array(b.clone());
                let pending_before = self.pending_join_count();
                let element = self
                    .unify(
                        a.as_ref().clone(),
                        b.as_ref().clone(),
                        left_origin,
                        right_origin,
                        found,
                        branch,
                    )
                    .map_err(|error| array_mismatch(error, &expected, &found_type))?;
                // A deferred conflict inside the element still belongs to
                // the whole source operand. Complete both serialized types
                // at every enclosing constructor (ADR §4.1.2.6).
                for join in self.pending_joins_mut().skip(pending_before) {
                    join.left = Type::Array(Arc::new(join.left.clone()));
                    join.right = Type::Array(Arc::new(join.right.clone()));
                }
                let resolved = Type::Array(Arc::new(element));
                // A successful mixed pair cannot later change its answer:
                // Deferred is bound or aliased monotonically, conflicting
                // later binds fail at their own site, and Never/Tainted are
                // immutable. Reusing the exact Arc pair would still charge
                // inner_pairs. The concrete-only cache policy keeps entries
                // directly printable and avoids retaining transient nodes.
                if internal_state(&expected).is_none() && internal_state(&found_type).is_none() {
                    self.array_unify_cache.insert(
                        key,
                        (
                            expected,
                            found_type,
                            resolved.clone(),
                            self.solver_steps - before_inner,
                        ),
                    );
                }
                Ok(resolved)
            }
            (Type::Result(a_ok, a_err), Type::Result(b_ok, b_err)) => {
                let ok = self.unify(
                    a_ok.as_ref().clone(),
                    b_ok.as_ref().clone(),
                    left_origin.clone(),
                    right_origin.clone(),
                    found.clone(),
                    branch,
                )?;
                let err = self.unify(
                    a_err.as_ref().clone(),
                    b_err.as_ref().clone(),
                    left_origin,
                    right_origin,
                    found,
                    branch,
                )?;
                Ok(Type::Result(Arc::new(ok), Arc::new(err)))
            }
            (a, b) => match compare_source_site(&a, &b, left_origin, found, policy) {
                Some(error) => Err(error),
                None => Ok(a),
            },
        }
    }

    fn union(&mut self, left: u64, right: u64, found: Span) -> Result<(), Box<Diagnostic>> {
        self.charge_solver(&found)?;
        let l = self.root(left);
        let r = self.root(right);
        if l == r {
            return Ok(());
        }
        if (self.type_nodes[&l].array_child.is_some() || self.type_nodes[&r].array_child.is_some())
            && (self.array_reaches(l, r, &found)? || self.array_reaches(r, l, &found)?)
        {
            self.fail_constructed_dependents(l);
            self.fail_constructed_dependents(r);
            // A finite monomorphic type cannot contain itself. The ADR does
            // not assign a separate occurs-check code; inference remains
            // unresolved at this constructor.
            return Err(names::diagnostic(
                "NX-E0103",
                None,
                Stage::Types,
                Operation::TypesInfer,
                found,
            ));
        }
        // Root rank and numeric ID determine only the private representative.
        // Choosing the other root, even when sizes differ, still attaches a
        // tree with summed size and merges every constraint below. Public
        // origins are selected by source spans, never by representative ID.
        let (a, b) = if self.type_nodes[&l].size > self.type_nodes[&r].size
            || (self.type_nodes[&l].size == self.type_nodes[&r].size && l < r)
        {
            (l, r)
        } else {
            (r, l)
        };
        let a_concrete = self.type_nodes[&a].concrete.clone();
        let b_concrete = self.type_nodes[&b].concrete.clone();
        let a_intrinsic = self.type_nodes[&a].concrete_intrinsic;
        let b_intrinsic = self.type_nodes[&b].concrete_intrinsic;
        let a_child = self.type_nodes[&a].array_child;
        let b_child = self.type_nodes[&b].array_child;
        let array_constructed =
            self.type_nodes[&a].array_constructed || self.type_nodes[&b].array_constructed;
        let bottom_only = self.type_nodes[&a].bottom_only || self.type_nodes[&b].bottom_only;
        let a_copyable = self.type_nodes[&a].copyable_origin.clone();
        let b_copyable = self.type_nodes[&b].copyable_origin.clone();
        if let (Some(_), Some((ty, _))) = (&a_copyable, &b_concrete)
            && !copyable(ty)
        {
            return Err(mismatch(
                found.clone(),
                found,
                None,
                "domain:copyable_data",
                ty,
            ));
        }
        if let (Some(_), Some((ty, _))) = (&b_copyable, &a_concrete)
            && !copyable(ty)
        {
            return Err(mismatch(
                found.clone(),
                found,
                None,
                "domain:copyable_data",
                ty,
            ));
        }
        if let (Some(child), Some((Type::Array(element), origin))) = (a_child, &b_concrete) {
            self.bind(
                child,
                element.as_ref().clone(),
                origin.clone(),
                found.clone(),
                b_intrinsic,
            )?;
        }
        if let (Some(child), Some((Type::Array(element), origin))) = (b_child, &a_concrete) {
            self.bind(
                child,
                element.as_ref().clone(),
                origin.clone(),
                found.clone(),
                a_intrinsic,
            )?;
        }
        if let Some((other, _)) = &b_concrete
            && a_child.is_some()
            && !matches!(other, Type::Array(_))
        {
            return Err(self.array_domain_conflict(a, other, found));
        }
        if let Some((other, _)) = &a_concrete
            && b_child.is_some()
            && !matches!(other, Type::Array(_))
        {
            return Err(self.array_domain_conflict(b, other, found));
        }
        if let (Some((a_ty, a_origin)), Some((b_ty, b_origin))) = (&a_concrete, &b_concrete)
            && a_ty != b_ty
        {
            let (expected, expected_origin, expected_intrinsic, actual) =
                if a_origin.start <= b_origin.start {
                    (a_ty, a_origin, a_intrinsic, b_ty)
                } else {
                    (b_ty, b_origin, b_intrinsic, a_ty)
                };
            let expected_label = pretty(expected).ok_or_else(|| unresolved_type(found.clone()))?;
            return Err(mismatch(
                found.clone(),
                found,
                (!expected_intrinsic).then_some(expected_origin.clone()),
                &expected_label,
                actual,
            ));
        }
        if let (Some(left_child), Some(right_child)) = (a_child, b_child) {
            self.union(left_child, right_child, found.clone())?;
        }
        self.type_nodes.get_mut(&b).unwrap().parent = a;
        self.type_nodes.get_mut(&a).unwrap().size += self.type_nodes[&b].size;
        self.type_nodes.get_mut(&a).unwrap().array_child = a_child.or(b_child);
        self.type_nodes.get_mut(&a).unwrap().array_constructed = array_constructed;
        self.type_nodes.get_mut(&a).unwrap().bottom_only = bottom_only;
        // only presence of copyable_origin participates in constraints.
        // Its saved span is not emitted as a related diagnostic location;
        // choosing either present span leaves the public result unchanged.
        self.type_nodes.get_mut(&a).unwrap().copyable_origin = match (a_copyable, b_copyable) {
            (Some(a), Some(b)) => Some(if a.start <= b.start { a } else { b }),
            (Some(a), None) | (None, Some(a)) => Some(a),
            (None, None) => None,
        };
        let a_array_origin = self.type_nodes[&a].array_origin.clone();
        let b_array_origin = self.type_nodes[&b].array_origin.clone();
        // two independent array-constructor origins joined at this
        // type level are disjoint source spans. Their start order and the
        // comparison of one end with the other start choose the same origin;
        // a containing constructor would have failed the occurs check.
        self.type_nodes.get_mut(&a).unwrap().array_origin = match (a_array_origin, b_array_origin) {
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
        self.type_nodes.get_mut(&a).unwrap().concrete = chosen;
        self.type_nodes.get_mut(&a).unwrap().concrete_intrinsic = intrinsic;
        // Failure belongs to the equivalence class, not to whichever node
        // happened to win the rank comparison.
        let failed = self.failed_constructors.remove(&a) | self.failed_constructors.remove(&b);
        if failed {
            self.failed_constructors.insert(a);
        }
        Ok(())
    }

    fn array_reaches(
        &mut self,
        start: u64,
        target: u64,
        found: &Span,
    ) -> Result<bool, Box<Diagnostic>> {
        let mut current = start;
        for _ in 0..=self.next_variable {
            let Some(child) = self.type_nodes[&current].array_child else {
                return Ok(false);
            };
            self.charge_solver(found)?;
            current = self.root(child);
            if current == target {
                return Ok(true);
            }
        }
        // The union occurs check prevents cycles. In an acyclic graph a walk
        // visits at most next_variable distinct nodes, so this fallback is
        // unreachable; it rejects defensively if the invariant is broken.
        Ok(true)
    }

    fn inferred_array_depth(&mut self, id: u64) -> usize {
        let mut current = id;
        let mut depth = 0;
        loop {
            if let Some((ty, _)) = &self.type_nodes[&current].concrete {
                let mut work = vec![(ty, 0_usize)];
                let mut inner_max = 0;
                while let Some((part, arrays)) = work.pop() {
                    inner_max = inner_max.max(arrays);
                    match part {
                        Type::Array(child) => work.push((child, arrays + 1)),
                        Type::Result(ok, err) => {
                            work.push((ok, arrays));
                            work.push((err, arrays));
                        }
                        _ => {}
                    }
                }
                return depth + inner_max;
            }
            let Some(child) = self.type_nodes[&current].array_child else {
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
        for id in 0..self.next_variable {
            if self.type_nodes[&id].parent != id {
                continue;
            }
            let origin = self.type_nodes[&id].array_origin.clone();
            if let Some(origin) = origin {
                self.check_one_inferred_depth(id, &origin)?;
            }
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
        match e {
            Expr::Name(_, span) => match self.resolved.uses.get(&span.start) {
                Some(ResolvedUse::Local(local)) => {
                    if let Some(known) = self.locals.get(&local.name_span.start).cloned() {
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
            Expr::Group(inner, _) => self.expression_origin(inner),
            Expr::Index { target, .. } => self.expression_origin(target),
            _ => Self::primary_span(e),
        }
    }

    fn expression_variable(&self, e: &Expr) -> Option<u64> {
        self.expr_vars.get(&span_key(e.span())).copied()
    }

    fn place_root_binding_site(&self, place: &Expr) -> Option<u64> {
        match place {
            Expr::Group(inner, _) | Expr::Index { target: inner, .. } => {
                self.place_root_binding_site(inner)
            }
            Expr::Name(_, span) => match self.resolved.uses.get(&span.start) {
                Some(ResolvedUse::Local(local)) => Some(local.name_span.start),
                _ => None,
            },
            _ => None,
        }
    }

    fn expression_reads_binding(&self, expression: &Expr, site: u64) -> bool {
        match expression {
            Expr::Name(_, span) => match self.resolved.uses.get(&span.start) {
                Some(ResolvedUse::Local(local)) => local.name_span.start == site,
                _ => false,
            },
            Expr::Array(items, _) => items
                .iter()
                .any(|item| self.expression_reads_binding(item, site)),
            Expr::Group(inner, _) | Expr::Unary { value: inner, .. } => {
                self.expression_reads_binding(inner, site)
            }
            Expr::Binary { left, right, .. } => {
                self.expression_reads_binding(left, site)
                    || self.expression_reads_binding(right, site)
            }
            Expr::Call { target, args, .. } => {
                self.expression_reads_binding(target, site)
                    || args
                        .iter()
                        .any(|argument| self.expression_reads_binding(&argument.value, site))
            }
            Expr::Index { target, index, .. } => {
                self.expression_reads_binding(target, site)
                    || self.expression_reads_binding(index, site)
            }
            Expr::If {
                condition,
                then_block,
                otherwise,
                ..
            } => {
                self.expression_reads_binding(condition, site)
                    || self.block_reads_binding(then_block, site)
                    || otherwise
                        .as_ref()
                        .is_some_and(|otherwise| match otherwise.as_ref() {
                            Else::Block(block) => self.block_reads_binding(block, site),
                            Else::If(expression) => self.expression_reads_binding(expression, site),
                        })
            }
            Expr::Integer(..) | Expr::Bool(..) | Expr::Unit(..) => false,
        }
    }

    fn block_reads_binding(&self, block: &Block, site: u64) -> bool {
        block
            .statements
            .iter()
            .any(|statement| self.statement_reads_binding(statement, site))
    }

    fn statement_reads_binding(&self, statement: &Statement, site: u64) -> bool {
        match statement {
            Statement::Let { value, .. } | Statement::Expr { value, .. } => {
                self.expression_reads_binding(value, site)
            }
            Statement::Assign { place, value, .. } => {
                self.expression_reads_binding(place, site)
                    || self.expression_reads_binding(value, site)
            }
            Statement::Return { value, .. } | Statement::Value { value, .. } => value
                .as_ref()
                .is_some_and(|value| self.expression_reads_binding(value, site)),
            Statement::If {
                condition,
                then_block,
                otherwise,
                ..
            } => {
                self.expression_reads_binding(condition, site)
                    || self.block_reads_binding(then_block, site)
                    || otherwise
                        .as_ref()
                        .is_some_and(|otherwise| self.statement_reads_binding(otherwise, site))
            }
            Statement::While {
                condition, body, ..
            } => {
                self.expression_reads_binding(condition, site)
                    || self.block_reads_binding(body, site)
            }
            Statement::Block(block) => self.block_reads_binding(block, site),
        }
    }

    fn assignment_value_variable(&self, e: &Expr) -> Option<u64> {
        // Assignment copies the binding's established type and provenance.
        // Equality instead keeps its operand as a source witness
        // (§§4.1.2.4, 4.1.2.6 and 13.15).
        match e {
            Expr::Group(inner, _) => self.assignment_value_variable(inner),
            Expr::Name(_, span) => match self.resolved.uses.get(&span.start) {
                Some(ResolvedUse::Local(local)) => self
                    .locals
                    .get(&local.name_span.start)
                    .and_then(|known| known.variable),
                _ => None,
            },
            _ => self.expression_variable(e),
        }
    }

    fn binding_variable_at_source(&self, e: &Expr) -> Option<u64> {
        match e {
            Expr::Group(inner, _) => self.binding_variable_at_source(inner),
            Expr::Name(_, span) => match self.resolved.uses.get(&span.start) {
                Some(ResolvedUse::Local(local)) => self
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
        match e {
            Expr::Group(inner, _) => Self::primary_span(inner),
            _ => e.span().clone(),
        }
    }

    fn normal_witness_span(&self, expression: &Expr) -> Span {
        match expression {
            Expr::Group(inner, _) => self.normal_witness_span(inner),
            Expr::If {
                then_block,
                otherwise,
                ..
            } => {
                let first = branch_value_expr(then_block);
                let second = match otherwise.as_deref() {
                    Some(Else::Block(block)) => branch_value_expr(block),
                    Some(Else::If(other)) => Some(other.as_ref()),
                    None => None,
                };
                let first_diverges =
                    first.is_some_and(|e| self.divergent_exprs.contains(&span_key(e.span())));
                let second_diverges =
                    second.is_some_and(|e| self.divergent_exprs.contains(&span_key(e.span())));
                if first_diverges
                    && !second_diverges
                    && let Some(second) = second
                {
                    return self.normal_witness_span(second);
                } else if second_diverges
                    && !first_diverges
                    && let Some(first) = first
                {
                    return self.normal_witness_span(first);
                }
                Self::primary_span(expression)
            }
            _ => Self::primary_span(expression),
        }
    }

    // These expressions have only intrinsic operand constraints. A later
    // Array base can add an I64 context to the selector but cannot change
    // how their children are checked. A value union or constructor can
    // change from a join error to leaf mismatches under that context, so its
    // diagnosis must wait for the base shape (§§4.1.2.3–4, RD107).
    fn context_stable_selector(e: &Expr) -> bool {
        match e {
            Expr::Name(..) | Expr::Integer(..) | Expr::Bool(..) | Expr::Unit(..) => true,
            Expr::Group(inner, _) => Self::context_stable_selector(inner),
            Expr::Binary { left, right, .. } => {
                Self::context_stable_selector(left) && Self::context_stable_selector(right)
            }
            Expr::Array(..)
            | Expr::If { .. }
            | Expr::Index { .. }
            | Expr::Call { .. }
            | Expr::Unary { .. } => false,
        }
    }

    fn is_value_if(e: &Expr) -> bool {
        match e {
            Expr::Group(inner, _) => Self::is_value_if(inner),
            Expr::If { .. } => true,
            _ => false,
        }
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
                | Type::MockBlockingResource
                | Type::Deferred(_)
                | Type::Never
                | Type::Tainted => {}
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
        // The span sets below may retain old entries without changing a
        // later lookup: source spans are unique across functions. Locals,
        // expr_vars and expression_types must be cleared because the report
        // harvests their entire maps after every function; stale variable
        // IDs would be looked up in this function's fresh type-node table.
        self.locals.clear();
        self.eligible_shapes.clear();
        self.preseeded_children.clear();
        self.projected_bases.clear();
        self.projection_items.clear();
        self.assigned_bases.clear();
        self.nested_assignment_sites.clear();
        self.pre_assignment_snapshot.clear();
        self.assignment_starts.clear();
        self.first_assignment_rhs_starts.clear();
        self.assignment_type_plan.clear();
        self.prefix_concrete.clear();
        self.deferred_join_roots.clear();
        self.branch_join_sites.clear();
        self.recursive_join_roots.clear();
        self.block_depth = 0;
        self.top_statement_start = 0;
        self.top_statement_end = 0;
        self.active_if_span = None;
        // Node IDs restart at zero below. If this map were retained, newly
        // allocated IDs would overwrite the reachable prefix; stale higher
        // IDs are unreachable. Clearing avoids retaining that stale suffix.
        self.type_nodes.clear();
        // Retaining entries across functions would be semantically equivalent:
        // each entry owns both Arc operands, so no pointer key can be reused.
        // Clear here to release each function's cached trees promptly instead
        // of retaining their sum until the compilation ends.
        self.array_unify_cache.clear();
        self.expr_vars.clear();
        self.expression_types.clear();
        self.conditional_result_types.clear();
        self.conditional_result_vars.clear();
        self.next_variable = 0;
        // Empty constructors are finalized at this function boundary; unlike
        // the span-keyed maps above, retaining this vector would diagnose a
        // prior function again. §13.9 requires separate function sites.

        // Failed constructor IDs restart at zero. Retaining this set would
        // suppress the next function's independent NX-E0103 (§13.9).
        self.failed_constructors.clear();
        // Normal exits drain this vector with mem::take; a quota exit stops
        // the compilation before another function. Clearing is defensive.
        self.pending.clear();
        self.pending_generation = 0;
        self.has_pending_projection = false;
        self.pending_domain_calls.clear();
        self.emitted_sites.clear();
        // These sets use source offsets, which are unique across function
        // bodies in one file. A later function cannot read an earlier span;
        // their clears bound memory but do not change a reachable result.
        self.deferred_exprs.clear();
        self.divergent_exprs.clear();
        self.failed_join_exprs.clear();
        self.failed_projection_results.clear();
        self.context_rejections.clear();
        self.partial_normal_exprs.clear();
        // Binding name offsets have the same per-file uniqueness property.
        self.invalid.clear();
        self.tainted = false;
        self.result = f.result.as_ref().map(Type::from).unwrap_or(Type::Unit);
        self.result_span = f
            .result
            .as_ref()
            .and_then(|_| self.type_annotation_span(f.name_span.end, f.body.span.start, "->"));
        if let Some(span) = &self.result_span {
            self.check_depth(&self.result, span)?;
        }
        let sig = &self.resolved.signatures[&f.name];
        for p in &sig.params {
            let ty = Type::from(&p.ty);
            self.check_depth(&ty, &p.type_span)?;
            self.locals.insert(
                p.name_span.start,
                Known {
                    ty,
                    origin: p.type_span.clone(),
                    variable: None,
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
        for (id, result, nominal) in std::mem::take(&mut self.conditional_result_vars) {
            let guards: Vec<_> = self
                .pending
                .iter()
                .filter_map(|entry| match entry {
                    PendingConstraint::GuardedComparison(guard) if guard.result == result => {
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
                // DomainResult can depend on another nominal operation (for
                // example `Len(q[x]) + 1`). Resolve the entire chain before
                // granting the binding its executable type. An undecided
                // domain remains usable only without an unresolved
                // constructor obligation (§4.1.2.3).
                let mut site = result.clone();
                let mut seen = HashSet::new();
                let mut usable = false;
                while seen.insert(span_key(&site)) {
                    let Some((variable, domain, dependency)) =
                        self.pending.iter().find_map(|entry| match entry {
                            PendingConstraint::DomainResult(pending) if pending.call == site => {
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
            if let Err(error) = self.bind(id, nominal, result, found_site, true) {
                self.record_error(error);
            }
        }
        let mut joins = self.take_pending_kind(|entry| match entry {
            PendingConstraint::Join(join) => Ok(join),
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
                        !domain_accepts(domain.domain, &left)
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
                let child = self.type_nodes[&constructor].array_child;
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
                self.unresolved_constructor_diagnostic(constructor, join.found)
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
                let has_cause = !self.failed_constructors.contains(&root)
                    || self.type_nodes[&root]
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
            let own_inference_cause = causal_inference_sites.iter().any(|&(start, end)| {
                start <= span.start && span.end <= end && (start, end) != span_key(&span)
            });
            // A failed constructor can be caused at a different source site.
            // Its unresolved literal must not add a dependent inference error.
            if (!self.failed_constructors.contains(&root)
                || own_inference_cause
                || shared_inference_roots.contains(&root))
                && self.concrete(child).is_none()
                && !unresolved_arrays.contains(&self.root(child))
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
        for domain in self.take_pending_kind(|entry| match entry {
            PendingConstraint::Domain(domain) => Ok(domain),
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
        for pending in self.take_pending_kind(|entry| match entry {
            PendingConstraint::DomainResult(result) => Ok(result),
            other => Err(other),
        }) {
            let domain_invalid = self.concrete(pending.variable).is_some_and(|(ty, _)| {
                if pending.domain == "I64" {
                    ty != Type::I64
                } else {
                    !domain_accepts(pending.domain, &ty)
                }
            });
            let dependency_failed = pending
                .depends_on
                .as_ref()
                .is_some_and(|site| failed_dependencies.contains(&span_key(site)));
            if domain_invalid || dependency_failed {
                failed_results.insert(span_key(&pending.call));
                failed_dependencies.insert(span_key(&pending.call));
            }
            if !domain_invalid
                && !dependency_failed
                && let Some(expected) = pending.expected.and_then(|ty| self.resolved_join_side(ty))
                && let Some(error) = compare_source_site(
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
            {
                failed_dependencies.insert(span_key(&pending.call));
                self.record_error(error);
            }
        }
        let mut failed_binary_operations = HashSet::new();
        for pending in self.take_pending_kind(|entry| match entry {
            PendingConstraint::BinaryDomain(domain) => Ok(domain),
            other => Err(other),
        }) {
            if let Some((ty, _)) = self.concrete(pending.variable) {
                let valid = if pending.domain == "I64" {
                    ty == Type::I64
                } else {
                    domain_accepts(pending.domain, &ty)
                };
                if !valid {
                    failed_results.insert(span_key(&pending.result));
                    failed_dependencies.insert(span_key(&pending.result));
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
                }
            }
        }
        let guarded = self.take_pending_kind(|entry| match entry {
            PendingConstraint::GuardedComparison(comparison) => Ok(comparison),
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
                            || self.failed_projection_results.contains(&span_key(site))
                    })
                    || source_site_failed(&pending.expected_origin)
                    || source_site_failed(&pending.found_site));
            if domain_refuted
                || unresolved_guard
                || unresolved_arrays.contains(&self.root(pending.variable))
            {
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
                    self.record_error(error);
                } else if let Some(error) = pending.domain_error_if_equal {
                    failed_results.insert(span_key(&pending.result));
                    self.record_error(error);
                } else if let Some(consumer_expected) = pending
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
        }
        // ADR §4.1.2.5: a failed domain invalidates the nominal result of
        // its operation. A consumer whose expected witness is that result,
        // or which checks the result itself, has no independent evidence.
        self.suppress_result_dependents(&failed_results);
        Ok(())
    }

    fn block(&mut self, b: &Block) -> Result<(), Box<Diagnostic>> {
        self.block_depth += 1;
        let result = (|| {
            for statement in &b.statements {
                if self.block_depth == 1 {
                    let span = match statement {
                        Statement::Let { span, .. }
                        | Statement::Assign { span, .. }
                        | Statement::Return { span, .. }
                        | Statement::Value { span, .. }
                        | Statement::If { span, .. }
                        | Statement::While { span, .. }
                        | Statement::Expr { span, .. } => span,
                        Statement::Block(block) => &block.span,
                    };
                    self.top_statement_start = span.start;
                    self.top_statement_end = span.end;
                }
                self.statement_at_site(statement);
                if !self.analysis_exhausted {
                    self.resolve_pending_projections(false)?;
                }
            }
            Ok(())
        })();
        self.block_depth -= 1;
        result
    }

    fn statement_at_site(&mut self, statement: &Statement) {
        if self.analysis_exhausted {
            return;
        }
        // Each statement starts with a clean local taint bit. Its result is
        // merged back afterward, so the bit left by an earlier statement
        // cannot suppress an independent error in this statement (§13.9).
        let previous_taint = self.tainted;
        self.tainted = false;
        let outcome = self.statement(statement);
        let tainted = self.tainted;
        self.tainted = previous_taint || tainted;
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
                .conditional_result_types
                .get(&span_key(value.span()))
                .cloned()
            && self.pending_operation_guard(value).is_some()
        {
            let site = Self::primary_span(value);
            match self.new_variable(&site) {
                Ok(id) => {
                    let name_span =
                        names::token_span(&self.tokens, span.start, value.span().start, name)
                            .unwrap_or_else(|| span.clone());
                    self.locals.insert(
                        name_span.start,
                        Known {
                            ty: Type::Deferred(id),
                            origin: site.clone(),
                            variable: Some(id),
                        },
                    );
                    self.conditional_result_vars.push((id, site, nominal));
                    self.tainted = previous_taint;
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
            // record only. Name uses test self.invalid before reading Known,
            // and the inferred table reads its type but never its origin.
            // Thus changing this fallback span is unobservable once the
            // binding is marked invalid below.
            let origin = self
                .type_annotation_span(span.start, value.span().start, ":")
                .unwrap_or_else(|| Self::primary_span(value));
            self.locals.insert(
                name_span.start,
                Known {
                    ty: annotation.as_ref().map(Type::from).unwrap_or(Type::Tainted),
                    origin,
                    variable: None,
                },
            );
            self.invalid.insert(name_span.start);
        }
        if let Err(error) = outcome {
            self.record_error(error);
        }
    }

    fn statement_condition(&mut self, condition: &Expr) -> Result<(), Box<Diagnostic>> {
        match self.expr(condition, Some(&Type::Bool), None) {
            Ok(_) => Ok(()),
            Err(error) if error.operation == Operation::AnalysisLimit => Err(error),
            Err(error) => {
                // The body is checked hypothetically even when the condition
                // fails. Its declarations and operands own independent sites.
                self.record_error(error);
                self.tainted = true;
                Ok(())
            }
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
                if annotation.is_some() {
                    self.charge_solver(ann_span.as_ref().unwrap_or(&Self::primary_span(value)))?;
                }
                let written = annotation.as_ref().map(Type::from);
                let got = self.expr(value, written.as_ref(), ann_span.as_ref())?;
                let ns = names::token_span(&self.tokens, span.start, value.span().start, name)
                    .unwrap_or_else(|| span.clone());
                let origin = ann_span.unwrap_or_else(|| self.expression_origin(value));
                self.check_depth(&got, &origin)?;
                let deferred = annotation.is_none()
                    && self.deferred_exprs.contains(&span_key(value.span()))
                    && internal_state(&got).is_some()
                    && !self.tainted;
                let variable = if deferred {
                    let id = self.new_variable(&Self::primary_span(value))?;
                    self.type_nodes.get_mut(&id).unwrap().bottom_only =
                        self.divergent_exprs.contains(&span_key(value.span()));
                    if let Some(from) = self.expression_variable(value) {
                        self.unify(
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
                self.locals.insert(
                    ns.start,
                    Known {
                        ty: if let Some(id) = variable {
                            Type::Deferred(id)
                        } else {
                            written.unwrap_or(got)
                        },
                        origin,
                        variable,
                    },
                );
                if let Some(id) = variable {
                    if self.planning {
                        let root = self.root(id);
                        if self.type_nodes[&root].array_child.is_some()
                            && !self.type_nodes[&root].bottom_only
                        {
                            self.eligible_shapes.insert(ns.start);
                        }
                    }
                    self.seed_planned_shape(ns.start, id);
                }
                Ok(())
            }
            Statement::Assign { place, value, .. } => {
                if self.planning
                    && let Expr::Name(_, span) = place
                    && let Some(ResolvedUse::Local(local)) = self.resolved.uses.get(&span.start)
                {
                    if self.eligible_shapes.contains(&local.name_span.start)
                        && !self
                            .pre_assignment_snapshot
                            .contains_key(&local.name_span.start)
                        && let Some(id) = self
                            .locals
                            .get(&local.name_span.start)
                            .and_then(|known| known.variable)
                    {
                        // Each binding has its own prefix. Another local's
                        // earlier assignment cannot snapshot this one before
                        // an intervening source constraint (§4.1.2.7).
                        let concrete = self.concrete_evidence(id);
                        self.pre_assignment_snapshot
                            .insert(local.name_span.start, concrete);
                    }
                    self.assigned_bases.insert(local.name_span.start);
                    self.assignment_starts
                        .entry(local.name_span.start)
                        .or_insert(span.start);
                    self.first_assignment_rhs_starts
                        .entry(local.name_span.start)
                        .or_insert(Self::primary_span(value).start);
                    if let Some(ty @ Type::Array(_)) = self.intrinsic_assignment_type(value) {
                        self.assignment_type_plan
                            .entry(local.name_span.start)
                            .or_insert((ty, Self::primary_span(value)));
                    }
                    if !self.prefix_concrete.contains_key(&local.name_span.start)
                        && let Some(id) = self
                            .locals
                            .get(&local.name_span.start)
                            .and_then(|known| known.variable)
                        && let Some(concrete) = self.concrete_evidence(id)
                    {
                        self.prefix_concrete.insert(local.name_span.start, concrete);
                    }
                }
                if self.planning
                    && matches!(place, Expr::Index { .. })
                    && let Some(site) = self.place_root_binding_site(place)
                    && self.eligible_shapes.contains(&site)
                    && self.expression_reads_binding(value, site)
                {
                    // A self-referential indexed write cannot borrow the
                    // origin of a later root assignment as its own witness
                    // (RD89, §§4.1.2.3 and 4.1.2.7).
                    self.nested_assignment_sites.insert(site);
                }
                let lhs = self.expr(place, None, None)?;
                // The destination and RHS are separate expression sites.
                // A poisoned destination must not suppress the RHS's own
                // contextual checks (F0-0002 §4.1.2.5).
                let lhs_tainted = std::mem::take(&mut self.tainted);
                let rhs_result = (|| {
                    let deferred_local = self.expression_variable(place).or_else(|| {
                        if let Expr::Name(_, span) = place {
                            match self.resolved.uses.get(&span.start) {
                                Some(ResolvedUse::Local(local)) => self
                                    .locals
                                    .get(&local.name_span.start)
                                    .and_then(|known| known.variable),
                                _ => None,
                            }
                        } else {
                            None
                        }
                    });
                    if let Some(local) = deferred_local {
                        let intrinsic_origin = self.origin_intrinsic(local);
                        let concrete = self.concrete(local);
                        let projected =
                            if concrete.is_none() && self.expression_has_array_root(value) {
                                self.projection_expected(local)
                            } else {
                                None
                            };
                        let expected = concrete.as_ref().or(projected.as_ref());
                        // A root can have an established Array shape while
                        // its deepest element remains unresolved. Preserve
                        // that structural context for an array RHS so each
                        // incompatible leaf owns its site (§4.1.2.2–3,6).
                        let root = self.root(local);
                        let nested_shape = self.type_nodes[&root]
                            .array_child
                            .map(|child| self.root(child))
                            .is_some_and(|child| self.type_nodes[&child].array_child.is_some());
                        let shape_context = if expected.is_none()
                            && matches!(place, Expr::Name(..))
                            && self.expression_has_array_root(value)
                            && nested_shape
                        {
                            Some(Type::Deferred(local))
                        } else {
                            None
                        };
                        let rhs = match self.expr(
                            value,
                            expected.map(|(ty, _)| ty).or(shape_context.as_ref()),
                            expected
                                .filter(|_| projected.is_some() || !intrinsic_origin)
                                .filter(|(_, origin)| {
                                    origin.start < Self::primary_span(place).start
                                })
                                .map(|(_, origin)| origin),
                        ) {
                            Ok(rhs) => rhs,
                            Err(error) => {
                                // The forbidden resource-array feature has no
                                // valid element type to propagate through this
                                // destination. Its gate owns the constructor.
                                if error.feature_id.as_deref() == Some("resource_array") {
                                    let root = self.root(local);
                                    if self.type_nodes[&root].array_constructed {
                                        self.failed_constructors.insert(root);
                                    }
                                }
                                return Err(error);
                            }
                        };
                        if self.tainted {
                            return Ok(());
                        }
                        let left_origin = expected
                            .as_ref()
                            // A planned type may be completed by a later
                            // assignment. That future source cannot become
                            // the witness of this earlier write (RD89,
                            // §§4.1.2.3 and 4.1.2.7).
                            .filter(|(_, origin)| origin.start < Self::primary_span(place).start)
                            .map(|(_, origin)| origin.clone())
                            .unwrap_or_else(|| {
                                if matches!(place, Expr::Index { .. }) {
                                    // The destination selects an element; the
                                    // RHS is its source witness (§13.15, RD89).
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
                            Type::Deferred(local),
                            right,
                            left_origin,
                            right_origin.clone(),
                            Self::primary_span(value),
                            false,
                        )?;
                        if concrete.is_none()
                            && matches!(place, Expr::Name(..))
                            && matches!(self.concrete(local), Some((Type::Array(_), _)))
                        {
                            self.retain_assignment_origin(local, &right_origin);
                        }
                        return Ok(());
                    }
                    // An invalid destination cannot impose a type on its RHS.
                    // The RHS still owns independent obligations such as an
                    // internally incompatible union (§4.1.2.5, §13.9).
                    let origin = Some(self.expression_origin(place));
                    let valid_lhs = (internal_state(&lhs) != Some(Type::Tainted)).then_some(&lhs);
                    self.expr(value, valid_lhs, origin.as_ref())?;
                    Ok(())
                })();
                self.tainted |= lhs_tainted;
                rhs_result
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
                    let result_label =
                        pretty(&self.result).ok_or_else(|| unresolved_type(primary.clone()))?;
                    let mut d = mismatch(
                        primary.clone(),
                        Span {
                            file: self.file.into(),
                            start: span.start,
                            end: span.start + 6,
                        },
                        self.result_span.clone(),
                        &result_label,
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
                self.statement_condition(condition)?;
                self.block(then_block)?;
                if let Some(other) = otherwise {
                    self.statement(other)?;
                }
                Ok(())
            }
            Statement::While {
                condition, body, ..
            } => {
                self.statement_condition(condition)?;
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
        self.check_against(e, expected, expected_span, true)
    }

    // A concrete context is checked at every value position. Transparent
    // syntax and constructors pass that context to their children; only a
    // leaf (or a constructor with an incompatible outer shape) compares its
    // own type. This keeps each §13.9 diagnostic at its §13.15 operand.
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

    fn check_against_outer(
        &mut self,
        e: &Expr,
        expected: &Type,
        expected_span: &Span,
    ) -> Result<Type, Box<Diagnostic>> {
        self.check_against_context(
            e,
            CheckContext {
                expected: Some(expected),
                expected_span: Some(expected_span),
                check_here: true,
                anchor: None,
                outer_compare: true,
                related_expected: true,
                deferred_element: false,
            },
        )
    }

    fn check_against_context(
        &mut self,
        e: &Expr,
        context: CheckContext<'_>,
    ) -> Result<Type, Box<Diagnostic>> {
        let CheckContext {
            expected,
            expected_span,
            check_here,
            related_expected,
            ..
        } = context;
        let mut context_use = ContextUse::Compare;
        let inherited_numeric_owner = self.numeric_domain_owner;
        if matches!(
            e,
            Expr::Binary { .. } | Expr::Index { .. } | Expr::Call { .. } | Expr::Array(..)
        ) {
            self.numeric_domain_owner = None;
        }
        let result = self.expr_inner(e, context, &mut context_use);
        self.numeric_domain_owner = inherited_numeric_owner;
        let got = result?;
        if got == Type::Tainted
            && self.pending_domain_calls.contains(&span_key(e.span()))
            && let Expr::Binary { op, .. } = e
        {
            // Keep the executable result separate from the causal type used
            // while the operand domain is still hypothetical. It is exported
            // only after this function's pending obligations have closed.
            self.conditional_result_types.insert(
                span_key(e.span()),
                if matches!(op.as_str(), "<" | "==") {
                    Type::Bool
                } else {
                    Type::I64
                },
            );
        }
        match internal_state(&got) {
            Some(Type::Tainted) => self.tainted = true,
            Some(Type::Never) => {
                self.deferred_exprs.insert(span_key(e.span()));
                // A divergent if records its own span. Len can record
                // divergence while returning its declared I64; Unary can
                // forward a Never child without a local mark, but that
                // wrapper mark has no observable consumer. No reachable
                // expression needs this insertion to diagnose divergence.
                self.divergent_exprs.insert(span_key(e.span()));
            }
            Some(Type::Deferred(_)) => {
                self.deferred_exprs.insert(span_key(e.span()));
            }
            _ => {}
        }
        // Keep the synthesized type even when its contextual comparison
        // fails: a sibling may still constrain an unresolved constructor.
        self.expression_types
            .insert(span_key(e.span()), got.clone());
        if check_here
            && let Some(expected) = expected
            && (internal_state(expected).is_none() || context.deferred_element)
            && !self.tainted
            && !self.pending_domain_calls.contains(&span_key(e.span()))
            && !self.failed_join_exprs.contains(&span_key(e.span()))
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
                            self.pending.iter_mut().find_map(|entry| match entry {
                                PendingConstraint::Projection(projection)
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
                        if !matches!(ty, Type::Array(_)));
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
                    let outcome = self.unify_with_expected_origin(
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
                        self.failed_join_exprs.insert(span_key(e.span()));
                    }
                    outcome.map(|_| ())
                }
            };
            if let Err(error) = result {
                if matches!(context_use, ContextUse::Compare)
                    && error.operation != Operation::AnalysisLimit
                {
                    self.context_rejections.insert(span_key(e.span()));
                }
                // A constructor conflict is also tracked when it is queued
                // or finalized. This mark is defensive: changing the code
                // predicate here alone cannot change a reachable diagnostic
                // while the same root is already in failed_constructors.
                if error.code == "NX-E0103"
                    && let Some(id) = self.expression_variable(e)
                {
                    let root = self.root(id);
                    self.failed_constructors.insert(root);
                }
                return Err(error);
            }
        }
        Ok(got)
    }

    fn expr_inner(
        &mut self,
        e: &Expr,
        context: CheckContext<'_>,
        context_use: &mut ContextUse,
    ) -> Result<Type, Box<Diagnostic>> {
        let CheckContext {
            expected,
            expected_span,
            check_here,
            outer_compare,
            ..
        } = context;
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
                        return Ok(Type::Tainted);
                    }
                    if let Some(known) = self.locals.get(&local.name_span.start).cloned() {
                        if let Some(id) = known.variable {
                            if let Some((ty, _)) = self.concrete(id) {
                                return Ok(ty);
                            }
                            if !self.planning
                                && let Some(plan) = self.read_plan.get(&local.name_span.start)
                                && span.start < plan.assignment_start
                                && plan
                                    .blocked
                                    .iter()
                                    .all(|(start, end)| span.start < *start || span.start >= *end)
                            {
                                return Ok(plan.ty.clone());
                            }
                            self.deferred_exprs.insert(span_key(span));
                            self.expr_vars.insert(span_key(span), id);
                            return Ok(Type::Deferred(id));
                        }
                        Ok(known.ty)
                    } else {
                        Ok(Type::Tainted)
                    }
                }
                _ => Ok(Type::Tainted), // Function values have already been rejected by parse or names.
            },
            Expr::Group(inner, span) => {
                *context_use = ContextUse::Propagated;
                let outcome = self.check_against_context(inner, context);
                // A failed contextual comparison can still leave a partial
                // inferred type for a later deferred join. The group has no
                // independent type or diagnostic site, including on Err.
                if let Some(partial) = self.expression_types.get(&span_key(inner.span())).cloned() {
                    self.expression_types.insert(span_key(span), partial);
                }
                if let Some(nominal) = self
                    .conditional_result_types
                    .get(&span_key(inner.span()))
                    .cloned()
                {
                    self.conditional_result_types
                        .insert(span_key(span), nominal);
                }
                // A contextual branch can synthesize a concrete type while
                // its actual value remains deferred or divergent. Preserve
                // those facts at the group boundary for later constraints.
                if self.deferred_exprs.contains(&span_key(inner.span())) {
                    self.deferred_exprs.insert(span_key(span));
                }
                if let Some(id) = self.expression_variable(inner) {
                    self.expr_vars.insert(span_key(span), id);
                }
                if self.divergent_exprs.contains(&span_key(inner.span())) {
                    self.divergent_exprs.insert(span_key(span));
                }
                if self.partial_normal_exprs.contains(&span_key(inner.span())) {
                    self.partial_normal_exprs.insert(span_key(span));
                }
                if self.context_rejections.contains(&span_key(inner.span())) {
                    self.context_rejections.insert(span_key(span));
                }
                if self.failed_join_exprs.contains(&span_key(inner.span())) {
                    self.failed_join_exprs.insert(span_key(span));
                }
                if self.pending_domain_calls.contains(&span_key(inner.span())) {
                    self.pending_domain_calls.insert(span_key(span));
                }
                outcome
            }
            Expr::Unary {
                mode: Mode::Read,
                value,
                span,
            } => {
                // This postfix `?` belongs to I08. The active profile
                // rejects the construct before types; evaluating its child
                // here is defensive for a direct checker invocation.
                self.expr(value, None, None)?;
                self.deferred_exprs.insert(span_key(span));
                Ok(Type::Tainted) // `?` belongs to I08.
            }
            Expr::Unary { value, .. } => {
                *context_use = ContextUse::Propagated;
                self.check_against(value, expected, expected_span, check_here)
            }
            Expr::Array(xs, span) => {
                let element_expected_type = match expected.filter(|_| !outer_compare) {
                    Some(Type::Array(t)) => Some(t.as_ref().clone()),
                    Some(Type::Deferred(id)) => {
                        let root = self.root(*id);
                        self.type_nodes[&root].array_child.map(Type::Deferred)
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
                    self.type_nodes.get_mut(&array).unwrap().array_child = Some(child);
                    self.type_nodes.get_mut(&array).unwrap().array_origin = Some(span.clone());
                    self.type_nodes.get_mut(&array).unwrap().array_constructed = true;
                    self.require_copyable(child, span)?;
                    self.expr_vars.insert(span_key(e.span()), array);
                    self.deferred_exprs.insert(span_key(e.span()));
                    self.pending.push(PendingConstraint::Constructor {
                        child,
                        array,
                        span: span.clone(),
                    });
                    return Ok(Type::Array(Arc::new(Type::Deferred(child))));
                };
                let mut item_type = element_expected.cloned();
                let mut item_origin = expected_span.cloned();
                let mut pending: Vec<(u64, Span)> = Vec::new();
                let mut diverges = false;
                let mut invalid = false;
                for (index, item) in xs.iter().enumerate() {
                    let origin = if index == 0 {
                        expected_span
                    } else {
                        item_origin.as_ref()
                    };
                    let prior_taint = self.tainted;
                    self.tainted = false;
                    // §4.7 checks each operand against a concrete context at
                    // its own site; §13.9 keeps independent element errors.
                    let item_expected = element_expected
                        .or_else(|| item_type.as_ref().filter(|ty| internal_state(ty).is_none()));
                    let pending_shape = pending.first().and_then(|(id, _)| {
                        let root = self.root(*id);
                        self.type_nodes[&root]
                            .array_child
                            .map(|_| Type::Deferred(*id))
                    });
                    let item_expected = item_expected.or(pending_shape.as_ref());
                    let outcome = self.check_against_context(
                        item,
                        CheckContext {
                            expected: item_expected,
                            expected_span: origin,
                            check_here: true,
                            anchor: None,
                            outer_compare: false,
                            related_expected: origin.is_some(),
                            deferred_element: element_expected.is_some() || pending_shape.is_some(),
                        },
                    );
                    let item_tainted =
                        self.tainted || self.failed_join_exprs.contains(&span_key(item.span()));
                    self.tainted = prior_taint || item_tainted;
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
                            invalid = true;
                            continue;
                        }
                    };
                    if item_tainted {
                        invalid = true;
                        continue;
                    }
                    if self.pending_domain_calls.contains(&span_key(item.span()))
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
                        invalid = true;
                        continue;
                    }
                    diverges |= self.divergent_exprs.contains(&span_key(item.span()));
                    if item_expected.is_some() {
                        // The first concrete element fixes the element type.
                        // Each later element is checked in its own subtree;
                        // an invalid element cannot replace that type.
                        // A result whose domain is still pending cannot be
                        // the sole witness for an independent later site.
                        // Prefer the next valid element of the same type.
                        if item_origin.as_ref().is_some_and(|origin| {
                            self.pending_domain_calls.contains(&span_key(origin))
                        }) && !self.pending_domain_calls.contains(&span_key(item.span()))
                        {
                            item_origin = Some(Self::primary_span(item));
                        }
                        continue;
                    }
                    if self.deferred_exprs.contains(&span_key(item.span())) {
                        if let Some(id) = self.expression_variable(item) {
                            self.require_copyable(id, &Self::primary_span(item))?;
                            if let Some(expected_item) = item_type.as_ref() {
                                // item_type is assigned only from a
                                // non-deferred result (or a concrete source
                                // annotation). check_against just compared
                                // this child with that concrete item type.
                                // Thus both sides cannot be concrete and
                                // incompatible here; branch-mode's special
                                // mismatch arm is unreachable.
                                self.unify(
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
                                        Type::Deferred(*first),
                                        Type::Deferred(id),
                                        first_origin.clone(),
                                        Self::primary_span(item),
                                        Self::primary_span(item),
                                        false,
                                    )?;
                                    if self.pending_join_count() > before {
                                        self.failed_join_exprs.insert(span_key(e.span()));
                                        // The rejected sibling cannot join
                                        // the pending witness set. A later
                                        // element is compared with the first
                                        // valid one (§§4.1.2.4–5, 13.9).
                                        continue;
                                    }
                                }
                                if pending.is_empty() {
                                    item_origin = Some(Self::primary_span(item));
                                }
                                pending.push((id, Self::primary_span(item)));
                            }
                        }
                        continue;
                    }
                    if item_type.is_none() {
                        let concrete_origin = Self::primary_span(item);
                        let mut first_conflict = false;
                        for (id, source_origin) in &pending {
                            let before_conflict = self.pending_join_count();
                            let projected_reference = self.projection_items.contains(id);
                            let source_type = projected_reference
                                .then(|| self.concrete(*id).map(|(ty, _)| ty))
                                .flatten();
                            let equality = self.unify(
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
                            invalid = true;
                            continue;
                        }
                        pending.clear();
                        item_type = Some(got);
                        // ADR §4.1.2.4,6: the first valid array element is
                        // the source witness for later element checks.
                        item_origin = Some(concrete_origin);
                    } else if let Some(prior) = item_type.clone() {
                        let prior_origin = origin.cloned().unwrap_or_else(|| span.clone());
                        let concrete_origin = Self::primary_span(item);
                        match self.unify(
                            prior,
                            got,
                            prior_origin,
                            concrete_origin,
                            Self::primary_span(item),
                            false,
                        ) {
                            Ok(ty) => item_type = Some(ty),
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
                                self.tainted = true;
                                invalid = true;
                            }
                        }
                    }
                }
                if invalid {
                    return Ok(Type::Tainted);
                }
                if let Some(item) = item_type {
                    if contains_resource(&item) {
                        return Err(resource_array(span.clone()));
                    }
                    if diverges {
                        self.divergent_exprs.insert(span_key(e.span()));
                    }
                    Ok(Type::Array(Arc::new(item)))
                } else {
                    self.deferred_exprs.insert(span_key(e.span()));
                    if diverges {
                        self.divergent_exprs.insert(span_key(e.span()));
                    }
                    if let Some((child, _)) = pending.first() {
                        let id = self.new_variable(span)?;
                        self.type_nodes.get_mut(&id).unwrap().array_child = Some(*child);
                        self.type_nodes.get_mut(&id).unwrap().array_origin = Some(span.clone());
                        self.type_nodes.get_mut(&id).unwrap().array_constructed = true;
                        self.check_one_inferred_depth(id, span)?;
                        self.expr_vars.insert(span_key(e.span()), id);
                        // Every constructor owns an element-resolution
                        // obligation, even when the pending element came
                        // from another constructor or a projection (ADR
                        // §4.1.2.2, RD122/RD127).
                        self.pending.push(PendingConstraint::Constructor {
                            child: *child,
                            array: id,
                            span: span.clone(),
                        });
                        Ok(Type::Array(Arc::new(Type::Deferred(*child))))
                    } else if diverges {
                        let child = self.new_variable(span)?;
                        let array = self.new_variable(span)?;
                        self.type_nodes.get_mut(&array).unwrap().array_child = Some(child);
                        self.type_nodes.get_mut(&array).unwrap().array_origin = Some(span.clone());
                        self.type_nodes.get_mut(&array).unwrap().array_constructed = true;
                        self.require_copyable(child, span)?;
                        self.expr_vars.insert(span_key(e.span()), array);
                        self.pending.push(PendingConstraint::Constructor {
                            child,
                            array,
                            span: span.clone(),
                        });
                        Ok(Type::Array(Arc::new(Type::Deferred(child))))
                    } else {
                        Ok(Type::Tainted)
                    }
                }
            }
            Expr::Binary {
                op, left, right, ..
            } => {
                let pending_before_left = self.pending_join_count();
                let inherited_numeric_owner = self.numeric_domain_owner;
                if matches!(op.as_str(), "+" | "-") {
                    self.numeric_domain_owner = Some(span_key(e.span()));
                }
                let left_result = self.check_against(
                    left,
                    (op == "+" || op == "-").then_some(&Type::I64),
                    None,
                    true,
                );
                self.numeric_domain_owner = inherited_numeric_owner;
                let l = match left_result {
                    Ok(left_ty) => left_ty,
                    Err(error) => {
                        if error.operation == Operation::AnalysisLimit {
                            return Err(error);
                        }
                        let left_tainted = self.tainted;
                        self.tainted = false;
                        let left_domain_failed = matches!(op.as_str(), "+" | "-")
                            && error.primary_span.as_ref() == Some(&Self::primary_span(left))
                            && error.data.get("expected_type")
                                == Some(&DataValue::String("I64".into()));
                        let right_tainted =
                            self.independent_binary_rhs(op, left, right, left_domain_failed)?;
                        self.tainted |= left_tainted;
                        if right_tainted {
                            self.record_error(error);
                            self.deferred_exprs.insert(span_key(e.span()));
                            self.tainted = true;
                            return Ok(Type::Tainted);
                        }
                        return Err(error);
                    }
                };
                if self
                    .pending_domain_calls
                    .contains(&span_key(&Self::primary_span(left)))
                {
                    // A pending operand domain does not yet authorize the
                    // operation's nominal result as a cross-operand witness.
                    // The other operand still has its own domain site.
                    let left_domain_failed = matches!(op.as_str(), "+" | "-")
                        && self.type_determined(&l)
                        && l != Type::I64
                        && !self.failed_join_exprs.contains(&span_key(left.span()));
                    let right_tainted =
                        self.independent_binary_rhs(op, left, right, left_domain_failed)?;
                    if right_tainted {
                        // The sibling owns a real failure. Its containing
                        // operation has no nominal value for a selector or
                        // another consumer (§4.1.2.5).
                        self.tainted = true;
                        return Ok(Type::Tainted);
                    }
                    if matches!(op.as_str(), "<" | "==")
                        && !right_tainted
                        && let Some(found) = self.pending_operation_type(right)
                        && let Some((variable, domain)) = self.pending_operation_guard(left)
                    {
                        // The cross equality is conditional on the left
                        // operation's still unresolved domain. Keep its
                        // source site until that domain is decided.
                        self.pending
                            .push(PendingConstraint::GuardedComparison(Box::new(
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
                                },
                            )));
                    }
                    if matches!(op.as_str(), "+" | "-")
                        && let Some((variable, domain)) = self.pending_operation_guard(left)
                    {
                        self.defer_domain_result(
                            variable,
                            domain,
                            e.span(),
                            &Type::I64,
                            expected,
                            expected_span,
                            Some(&Self::primary_span(left)),
                        );
                    } else {
                        self.pending_domain_calls.insert(span_key(e.span()));
                    }
                    if !self.pending_nominal_available(left) {
                        // An unresolved Array constructor can still refute
                        // the nominal result (§4.1.2.3); it is not evidence
                        // for a later assignment or an outer consumer.
                        return Ok(Type::Tainted);
                    }
                    return Ok(if matches!(op.as_str(), "<" | "==") {
                        Type::Bool
                    } else {
                        Type::I64
                    });
                }
                if matches!(op.as_str(), "+" | "-")
                    && self.pending_join_count() > pending_before_left
                {
                    let left_tainted = self.tainted;
                    self.tainted = false;
                    // ADR §4.1.2.4-5: an internal failed join contaminates
                    // this operand but has not consumed this operator's
                    // domain site. A direct left-domain conflict has.
                    let left_domain_failed = self.type_determined(&l) && l != Type::I64;
                    self.independent_binary_rhs(op, left, right, left_domain_failed)?;
                    self.tainted |= left_tainted;
                    self.failed_join_exprs.insert(span_key(e.span()));
                    self.tainted = true;
                    return Ok(Type::I64);
                }
                if self.failed_join_exprs.contains(&span_key(left.span())) {
                    let left_tainted = self.tainted;
                    self.tainted = false;
                    let left_domain_failed =
                        self.context_rejections.contains(&span_key(left.span()));
                    self.independent_binary_rhs(op, left, right, left_domain_failed)?;
                    self.tainted |= left_tainted;
                    self.failed_join_exprs.insert(span_key(e.span()));
                    self.tainted = true;
                    return Ok(if matches!(op.as_str(), "<" | "==") {
                        Type::Bool
                    } else {
                        Type::I64
                    });
                }
                if self.tainted {
                    // A child already recorded its own error. Visit the
                    // independent sibling, but suppress the operator error
                    // that would depend on the invalid left value.
                    self.tainted = false;
                    self.independent_binary_rhs(
                        op,
                        left,
                        right,
                        self.context_rejections.contains(&span_key(left.span())),
                    )?;
                    self.tainted = true;
                    self.deferred_exprs.insert(span_key(e.span()));
                    return Ok(if matches!(op.as_str(), "<" | "==") {
                        Type::Bool
                    } else {
                        Type::I64
                    });
                }
                if self.deferred_exprs.contains(&span_key(left.span())) {
                    let pending_before_operator = self.pending_join_count();
                    let left_variable = self.expression_variable(left);
                    let numeric = matches!(op.as_str(), "+" | "-");
                    let right_ty = match self.expr(right, numeric.then_some(&Type::I64), None) {
                        Ok(ty) => ty,
                        Err(error) if error.operation == Operation::AnalysisLimit => {
                            return Err(error);
                        }
                        Err(error) if matches!(op.as_str(), "<" | "==") => {
                            self.record_error(error);
                            if let Some(domain_error) =
                                self.comparison_domain_error(op, left, right, &l)?
                            {
                                self.record_error(domain_error);
                            }
                            self.tainted = true;
                            return Ok(Type::Tainted);
                        }
                        Err(error) => return Err(error),
                    };
                    if self
                        .pending_domain_calls
                        .contains(&span_key(&Self::primary_span(right)))
                    {
                        if matches!(op.as_str(), "<" | "==") {
                            if let Some(domain_error) =
                                self.comparison_domain_error(op, left, right, &l)?
                            {
                                self.record_error(domain_error);
                            }
                            if let (Some((variable, domain)), Some(found)) = (
                                self.pending_operation_guard(right),
                                self.pending_operation_type(right),
                            ) {
                                self.pending
                                    .push(PendingConstraint::GuardedComparison(Box::new(
                                        PendingGuardedComparison {
                                            variable,
                                            domain,
                                            expected: left_variable
                                                .map(Type::Deferred)
                                                .unwrap_or(l.clone()),
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
                                        },
                                    )));
                            }
                        }
                        self.pending_domain_calls.insert(span_key(e.span()));
                        if !self.tainted && self.pending_nominal_available(right) {
                            return Ok(if matches!(op.as_str(), "<" | "==") {
                                Type::Bool
                            } else {
                                Type::I64
                            });
                        }
                        return Ok(Type::Tainted);
                    }
                    if self.tainted || self.failed_join_exprs.contains(&span_key(right.span())) {
                        if matches!(op.as_str(), "<" | "==")
                            && let Some(domain_error) =
                                self.comparison_domain_error(op, left, right, &l)?
                        {
                            self.record_error(domain_error);
                        }
                        return Ok(Type::Tainted);
                    }
                    if matches!(op.as_str(), "<" | "==") {
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
                            left_type,
                            right_type,
                            Self::primary_span(left),
                            Self::primary_span(right),
                            Self::primary_span(right),
                            false,
                        )?;
                        if self.pending_join_count() > pending_before_operator {
                            self.failed_join_exprs.insert(span_key(e.span()));
                            return Ok(Type::Bool);
                        }
                        let variable = left_variable.or(right_variable);
                        let complete = variable
                            .and_then(|id| self.concrete(id).map(|(ty, _)| ty))
                            .or_else(|| (internal_state(&common).is_none()).then_some(common));
                        let domain = if op == "<" {
                            "I64"
                        } else {
                            "domain:copyable_data"
                        };
                        if let Some(ty) = complete {
                            let valid = if op == "<" {
                                ty == Type::I64
                            } else {
                                copyable(&ty)
                            };
                            if !valid {
                                return Err(mismatch(operator, found_site, None, domain, &ty));
                            }
                        } else if let Some(id) = variable {
                            let root = self.root(id);
                            if op == "<" && self.type_nodes[&root].array_child.is_none() {
                                self.unify_with_expected_origin(
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
                                self.pending.push(PendingConstraint::BinaryDomain(Box::new(
                                    PendingBinaryDomain {
                                        variable: id,
                                        operator,
                                        found: found_site,
                                        domain,
                                        result: e.span().clone(),
                                    },
                                )));
                                self.defer_domain_result(
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
                            Type::Deferred(id),
                            Type::I64,
                            Self::primary_span(left),
                            Self::primary_span(left),
                            Self::primary_span(left),
                            false,
                        )?;
                    }
                    if self.pending_join_count() > pending_before_operator {
                        self.failed_join_exprs.insert(span_key(e.span()));
                    }
                    return Ok(Type::I64);
                }
                match op.as_str() {
                    "+" | "-" => {
                        // The left operand was checked against intrinsic I64
                        // above. Reaching this branch with a different
                        // concrete type is impossible; keep the guard as a
                        // defensive assertion of that invariant.
                        if l != Type::I64 {
                            // A pending error of the left comparison can
                            // invalidate this Bool result later. Its sibling
                            // still has an independent numeric obligation.
                            let pending_left_result =
                                self.pending.iter().any(|entry| match entry {
                                    PendingConstraint::BinaryDomain(domain) => {
                                        domain.result == Self::primary_span(left)
                                    }
                                    _ => false,
                                });
                            self.independent_binary_rhs(op, left, right, !pending_left_result)?;
                            return Err(mismatch(
                                Self::primary_span(left),
                                Self::primary_span(left),
                                None,
                                "I64",
                                &l,
                            ));
                        }
                        let left_origin = Self::primary_span(left);
                        let _r = self.check_against_context(
                            right,
                            CheckContext {
                                expected: Some(&Type::I64),
                                expected_span: Some(&left_origin),
                                check_here: true,
                                anchor: None,
                                outer_compare: false,
                                related_expected: false,
                                deferred_element: false,
                            },
                        )?;
                        if self.tainted {
                            return Ok(Type::Tainted);
                        }
                        if self.failed_join_exprs.contains(&span_key(right.span())) {
                            self.failed_join_exprs.insert(span_key(e.span()));
                            return Ok(Type::I64);
                        }
                        if self.deferred_exprs.contains(&span_key(right.span())) {
                            return Ok(Type::I64);
                        }
                        Ok(Type::I64)
                    }
                    "<" | "==" => {
                        let left_origin = Self::primary_span(left);
                        let pending_before_right = self.pending_join_count();
                        let right_result = self.check_against_outer(right, &l, &left_origin);
                        if let Err(error) = right_result {
                            if error.operation == Operation::AnalysisLimit
                                || self.context_rejections.contains(&span_key(right.span()))
                            {
                                return Err(error);
                            }
                            // §4.1.2.4–5, RD95: the right child failed internally,
                            // so its cross-type equality is suppressed. The
                            // left operand still has its own domain check.
                            self.record_error(error);
                            if let Some(domain_error) =
                                self.comparison_domain_error(op, left, right, &l)?
                            {
                                self.record_error(domain_error);
                            }
                            self.tainted = true;
                            return Ok(Type::Tainted);
                        }
                        if self
                            .pending_domain_calls
                            .contains(&span_key(&Self::primary_span(right)))
                            && let Some((variable, domain)) = self.pending_operation_guard(right)
                        {
                            let domain_error_if_equal =
                                self.comparison_domain_error(op, left, right, &l)?;
                            if let Some(found) = self.pending_operation_type(right) {
                                self.pending
                                    .push(PendingConstraint::GuardedComparison(Box::new(
                                        PendingGuardedComparison {
                                            variable,
                                            domain,
                                            expected: l.clone(),
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
                                        },
                                    )));
                            }
                            self.pending_domain_calls.insert(span_key(e.span()));
                            if !self.tainted && self.pending_nominal_available(right) {
                                return Ok(Type::Bool);
                            }
                            return Ok(Type::Tainted);
                        }
                        if self.tainted {
                            if let Some(domain_error) =
                                self.comparison_domain_error(op, left, right, &l)?
                            {
                                self.record_error(domain_error);
                            }
                            return Ok(Type::Tainted);
                        }
                        if self.pending_join_count() > pending_before_right {
                            let internal_right_join =
                                self.pending_joins().skip(pending_before_right).any(|join| {
                                    right.span().start <= join.left_origin.start
                                        && join.left_origin.end <= right.span().end
                                });
                            if internal_right_join {
                                // §4.1.2.4–5, RD95: an internal pending join
                                // of the right operand cannot cancel the
                                // independent domain of the valid left one.
                                if let Some(domain_error) =
                                    self.comparison_domain_error(op, left, right, &l)?
                                {
                                    self.record_error(domain_error);
                                }
                            } else {
                                // The new join is the cross-operand equality.
                                // Its conflict takes priority when both sides
                                // resolve. If the right constructor remains
                                // unresolved, the left domain is independent.
                                let domain = PendingComparisonDomain {
                                    left: self
                                        .expression_variable(left)
                                        .map(Type::Deferred)
                                        .unwrap_or_else(|| l.clone()),
                                    operator: self.operator_span(left.span(), right.span(), op),
                                    found: Self::primary_span(left),
                                    domain: if op == "<" {
                                        "I64"
                                    } else {
                                        "domain:copyable_data"
                                    },
                                };
                                if let Some(join) =
                                    self.pending_joins_mut().nth(pending_before_right)
                                {
                                    join.comparison_domain_if_unresolved = Some(domain);
                                }
                            }
                            self.failed_join_exprs.insert(span_key(e.span()));
                            return Ok(Type::Bool);
                        }
                        if self.deferred_exprs.contains(&span_key(right.span())) {
                            if let Some(error) =
                                self.comparison_domain_error(op, left, right, &l)?
                            {
                                return Err(error);
                            }
                            return Ok(Type::Bool);
                        }
                        if let Some(error) = self.comparison_domain_error(op, left, right, &l)? {
                            return Err(error);
                        }
                        Ok(Type::Bool)
                    }
                    _ => unreachable!("the parser admits only four intrinsic binary operators"),
                }
            }
            Expr::Index { target, index, .. } => {
                let base = self.expr(target, None, None)?;
                if base == Type::Tainted {
                    let previous_taint = self.tainted;
                    self.tainted = false;
                    let outcome = self.expr(index, Some(&Type::I64), None);
                    let index_tainted = self.tainted;
                    // `previous_taint` is already true when `base` is Tainted.
                    // This branch returns Tainted, and check_against_context
                    // restores the bit before any parent observes the value.
                    // On AnalysisLimit, quota_error owns the denied charge.
                    self.tainted = previous_taint || index_tainted;
                    if let Err(error) = outcome {
                        if error.operation == Operation::AnalysisLimit {
                            return Err(error);
                        }
                        self.record_error(error);
                    }
                    return Ok(Type::Tainted);
                }
                if base == Type::Never {
                    self.expr(index, Some(&Type::I64), None)?;
                    self.deferred_exprs.insert(span_key(e.span()));
                    self.divergent_exprs.insert(span_key(e.span()));
                    return Ok(Type::Never);
                }
                if self.deferred_exprs.contains(&span_key(target.span())) {
                    if let Some(id) = self.expression_variable(target) {
                        let root = self.root(id);
                        let child = if self.type_nodes[&root].array_child.is_some() {
                            let child = self.require_array(id, &Self::primary_span(target))?;
                            self.expr(index, Some(&Type::I64), None)?;
                            if self.failed_join_exprs.contains(&span_key(index.span())) {
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
                            self.pending.push(PendingConstraint::Projection(Box::new(
                                PendingProjection {
                                    base: id,
                                    item: child,
                                    base_span: Self::primary_span(target),
                                    result_span: Self::primary_span(e),
                                    selector: (**index).clone(),
                                    selector_variable,
                                    checked_without_context: false,
                                    selector_invalid: false,
                                    expected: None,
                                    expected_span: None,
                                    related_expected: false,
                                },
                            )));
                            self.projected_bases.insert(id);
                            self.has_pending_projection = true;
                            child
                        };
                        self.expr_vars.insert(span_key(e.span()), child);
                        self.projection_items.insert(child);
                        self.deferred_exprs.insert(span_key(e.span()));
                        return Ok(Type::Deferred(child));
                    }
                    if self.divergent_exprs.contains(&span_key(target.span())) {
                        self.expr(index, Some(&Type::I64), None)?;
                        self.deferred_exprs.insert(span_key(e.span()));
                        self.divergent_exprs.insert(span_key(e.span()));
                        return Ok(Type::Never);
                    }
                }
                if !matches!(base, Type::Array(_)) && internal_state(&base).is_some() {
                    // An invalid or still hypothetical union cannot prove
                    // its result non-indexable. Visit the selector's own
                    // constraints without using the missing Array context.
                    let previous_taint = self.tainted;
                    self.tainted = false;
                    let selector = self.expr(index, None, None);
                    let selector_tainted = self.tainted;
                    self.tainted = previous_taint || selector_tainted;
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
                    let previous_taint = self.tainted;
                    self.tainted = false;
                    let selector = self.expr(index, None, None);
                    let selector_tainted = self.tainted;
                    self.tainted = previous_taint || selector_tainted;
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
                self.expr(index, Some(&Type::I64), None)?;
                if self.failed_join_exprs.contains(&span_key(index.span())) {
                    return Ok(Type::Tainted);
                }
                Ok(item.as_ref().clone())
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
                    let previous_taint = self.tainted;
                    self.tainted = false;
                    let written = Type::from(&formal.ty);
                    let outcome = self.expr(&arg.value, Some(&written), Some(&formal.type_span));
                    let tainted = self.tainted;
                    self.tainted = previous_taint || tainted;
                    if let Err(error) = outcome {
                        if error.operation == Operation::AnalysisLimit {
                            return Err(error);
                        }
                        // §13.9: an argument can return its own error while
                        // also carrying taint from a child. Site ownership
                        // prevents a repeat without hiding an independent
                        // argument error.
                        self.record_error(error);
                        self.tainted = true;
                    }
                }
                Ok(Type::from(&signature.result))
            }
            Expr::If {
                condition,
                then_block,
                otherwise,
                ..
            } => {
                *context_use = if outer_compare {
                    ContextUse::Compare
                } else {
                    ContextUse::Propagated
                };
                let pending_before_if = self.pending_join_count();
                let previous_if = self.active_if_span.replace(e.span().clone());
                let result = self.if_expr(
                    condition,
                    then_block,
                    otherwise.as_deref(),
                    CheckContext {
                        expected: expected.filter(|_| !outer_compare),
                        expected_span: expected_span.filter(|_| !outer_compare),
                        ..context
                    },
                );
                self.active_if_span = previous_if;
                let (normal, value_failed, join_failed) = result?;
                if join_failed
                    || expected.is_some() && self.pending_join_count() > pending_before_if
                {
                    self.failed_join_exprs.insert(span_key(e.span()));
                }
                // ADR §4.1.2.5: an inferred union with a failed join has
                // no value type for ordinary consumers, but its first
                // valid continuation remains a partial witness for an
                // enclosing union.
                if join_failed && expected.is_none() && normal.is_some() {
                    self.partial_normal_exprs.insert(span_key(e.span()));
                }
                if expected.is_some()
                    && (branch_value_expr(then_block).is_some_and(|value| {
                        self.context_rejections.contains(&span_key(value.span()))
                    }) || match otherwise.as_deref() {
                        Some(Else::Block(block)) => branch_value_expr(block).is_some_and(|value| {
                            self.context_rejections.contains(&span_key(value.span()))
                        }),
                        Some(Else::If(value)) => {
                            self.context_rejections.contains(&span_key(value.span()))
                        }
                        None => false,
                    })
                {
                    self.context_rejections.insert(span_key(e.span()));
                }
                if value_failed {
                    self.tainted = true;
                }
                if self.tainted && normal.is_some() {
                    // §13.9: a clean continuation contributes a constraint
                    // even when another reachable child taints this if.
                    self.partial_normal_exprs.insert(span_key(e.span()));
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
                    self.expr_vars.insert(span_key(e.span()), id);
                    self.deferred_exprs.insert(span_key(e.span()));
                    return Ok(Type::Deferred(id));
                }
                // A missing branch value is resolved by ownership flow. Until
                // then it cannot supply a concrete type to an outer operator.
                if normal.is_none() {
                    self.deferred_exprs.insert(span_key(e.span()));
                    self.divergent_exprs.insert(span_key(e.span()));
                }
                Ok(normal.or_else(|| expected.cloned()).unwrap_or(Type::Never))
            }
        }
    }

    fn if_expr(
        &mut self,
        condition: &Expr,
        then_block: &Block,
        otherwise: Option<&Else>,
        context: CheckContext<'_>,
    ) -> Result<(Option<Type>, bool, bool), Box<Diagnostic>> {
        let CheckContext {
            expected,
            expected_span,
            anchor,
            related_expected,
            deferred_element,
            ..
        } = context;
        let prior_taint = self.tainted;
        self.tainted = false;
        let pending_before_condition = self.pending.len();
        let condition_result = self.expr(condition, Some(&Type::Bool), None);
        let condition_site = Self::primary_span(condition);
        let pending_shape_failure = self.pending[pending_before_condition..]
            .iter()
            .any(|entry| match entry {
                PendingConstraint::Join(join) => {
                    join.found == condition_site
                        && self.failed_constructors.contains(&join.constructor)
                }
                _ => false,
            });
        // The condition's Array-vs-Bool conflict is already certain even
        // when its complete type is still pending for the diagnostic. It
        // cannot provide a usable type to the surrounding expression.
        let condition_tainted = self.tainted || pending_shape_failure;
        self.tainted = prior_taint || condition_tainted || condition_result.is_err();
        if let Err(error) = condition_result {
            if error.operation == Operation::AnalysisLimit {
                return Err(error);
            }
            // ADR §4.1.2.5: the branches retain independent obligations
            // even when the condition has no valid result.
            self.record_error(error);
        }
        let (first, first_failed, mut first_anchored) = self.branch(
            then_block,
            expected,
            expected_span,
            related_expected,
            anchor,
            deferred_element,
        )?;
        // A failed nested join has no usable first-branch value for a later
        // comparison. Its own queued source site remains responsible for the
        // error; the other branch is still visited for independent errors.
        first_anchored |= branch_value_expr(then_block).is_some_and(|e| {
            self.failed_join_exprs.contains(&span_key(e.span()))
                && !self.partial_normal_exprs.contains(&span_key(e.span()))
        });
        let first_partial = branch_value_expr(then_block)
            .is_some_and(|e| self.partial_normal_exprs.contains(&span_key(e.span())));
        let mut anchor_error = None;
        let mut anchor_pending = false;
        let first_span = branch_value_span(then_block);
        let first_for_anchor = first.clone().or_else(|| {
            branch_value_expr(then_block)
                .and_then(|e| self.expression_variable(e))
                .map(Type::Deferred)
        });
        if expected.is_none()
            && (!first_failed || first_partial)
            && !first_anchored
            && let (Some((base, base_span)), Some(found)) = (anchor, first_for_anchor.as_ref())
        {
            let pending_before_anchor = self.pending_join_count();
            match self.unify(
                base.clone(),
                found.clone(),
                base_span.clone(),
                first_span
                    .clone()
                    .unwrap_or_else(|| then_block.span.clone()),
                first_span
                    .clone()
                    .unwrap_or_else(|| then_block.span.clone()),
                true,
            ) {
                Err(error) if error.operation == Operation::AnalysisLimit => return Err(error),
                Err(error) => anchor_error = Some(error),
                Ok(_) => {}
            }
            anchor_pending = self.pending_join_count() > pending_before_anchor;
        }
        let anchor = anchor.or_else(|| {
            ((!first_failed || first_partial) && !first_anchored)
                .then(|| first_for_anchor.as_ref().zip(first_span.as_ref()))
                .flatten()
        });
        let (second, second_failed, second_anchored) = match otherwise {
            Some(Else::Block(block)) => {
                let (second, failed, anchored) = self.branch(
                    block,
                    expected,
                    expected_span,
                    related_expected,
                    anchor,
                    deferred_element,
                )?;
                if expected.is_none()
                    && anchor_error.is_none()
                    && !failed
                    && !anchored
                    && let (Some((base, base_span)), Some(found)) = (anchor, second.as_ref())
                {
                    // F0-0002 §4.1.2.4–5: an anchor supplied by an outer
                    // union also compares a valid later continuation when
                    // this nested union's first continuation failed.
                    let found_span = branch_value_span(block).unwrap_or_else(|| block.span.clone());
                    let result = self.unify(
                        base.clone(),
                        found.clone(),
                        base_span.clone(),
                        found_span.clone(),
                        found_span,
                        true,
                    );
                    match result {
                        Err(error) if error.operation == Operation::AnalysisLimit => {
                            return Err(error);
                        }
                        Err(error) => anchor_error = Some(error),
                        Ok(_) => {}
                    }
                }
                (second, failed, anchored)
            }
            Some(Else::If(other)) => {
                // Else-if is an expression like every other value operand.
                // The same contextual operation checks its branches and
                // records its inferred type for lowering.
                let inner_shape_conflict = if let Expr::If {
                    then_block,
                    otherwise: Some(inner_other),
                    ..
                } = other.as_ref()
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
                // A failed inner union cannot fix an unresolved outer
                // anchor. A concrete anchor remains a context available on
                // entry and still checks both inner branches (§§4.1.2.4–5).
                let inner_anchor = if inner_shape_conflict
                    && anchor
                        .as_ref()
                        .is_some_and(|(ty, _)| self.resolved_join_side((*ty).clone()).is_none())
                {
                    None
                } else {
                    anchor
                };
                let ty = self.check_against_context(
                    other,
                    CheckContext {
                        expected,
                        expected_span,
                        check_here: true,
                        anchor: inner_anchor,
                        outer_compare: false,
                        related_expected,
                        deferred_element,
                    },
                )?;
                let failed = self.failed_join_exprs.contains(&span_key(other.span()));
                (internal_state(&ty).is_none().then_some(ty), failed, failed)
            }
            None => (None, false, false), // Missing continuation is diagnosed by I07.
        };
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
            first_root.is_some_and(|id| self.failed_constructors.contains(&id));
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
            let outcome = self.unify(
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

    fn branch(
        &mut self,
        b: &Block,
        expected: Option<&Type>,
        expected_span: Option<&Span>,
        related_expected: bool,
        anchor: Option<(&Type, &Span)>,
        deferred_element: bool,
    ) -> Result<(Option<Type>, bool, bool), Box<Diagnostic>> {
        let mut result = None;
        let mut value_failed = false;
        let mut anchor_consumed = false;
        for s in &b.statements {
            if let Statement::Value { value: Some(e), .. } = s {
                let nested_anchor = expected.is_none() && anchor.is_some() && Self::is_value_if(e);
                let previous_taint = self.tainted;
                self.tainted = false;
                let outcome = self.check_against_context(
                    e,
                    CheckContext {
                        expected,
                        expected_span,
                        check_here: true,
                        anchor: if nested_anchor { anchor } else { None },
                        outer_compare: false,
                        related_expected,
                        deferred_element,
                    },
                );
                let tainted = self.tainted;
                self.tainted = previous_taint || tainted;
                anchor_consumed = nested_anchor;
                match outcome {
                    Ok(ty) => {
                        let deferred = self.deferred_exprs.contains(&span_key(e.span()));
                        let divergent = self.divergent_exprs.contains(&span_key(e.span()));
                        let pending_context_conflict =
                            self.pending_domain_calls.contains(&span_key(e.span()))
                                && expected.is_some_and(|wanted| {
                                    internal_state(wanted).is_none()
                                        && internal_state(&ty).is_none()
                                        && *wanted != ty
                                });
                        // Planning cannot use the nominal result of an
                        // undecided domain as a branch witness. A later
                        // constructor may satisfy that domain and only then
                        // make the branch equality decidable (§4.1.2.5).
                        let usable = (!tainted
                            && !pending_context_conflict
                            && !(self.planning
                                && self.pending_domain_calls.contains(&span_key(e.span()))))
                            || self.partial_normal_exprs.contains(&span_key(e.span()));
                        let partial = self.partial_normal_exprs.contains(&span_key(e.span()));
                        result = (usable
                            && (!deferred || expected.is_some() && !divergent || partial))
                            .then_some(ty);
                        value_failed |= self.failed_join_exprs.contains(&span_key(e.span()));
                        value_failed |= pending_context_conflict;
                        self.tainted |= pending_context_conflict;
                        // A tainted value has no branch type and its taint
                        // is already restored to self.tainted above. Marking
                        // value_failed here would repeat that same state.
                    }
                    Err(error) => {
                        if error.operation == Operation::AnalysisLimit {
                            return Err(error);
                        }
                        self.record_error(error);
                        result = self
                            .expression_types
                            .get(&span_key(e.span()))
                            .cloned()
                            .filter(|ty| {
                                !matches!(internal_state(ty), Some(Type::Tainted | Type::Never))
                            });
                        value_failed = true;
                    }
                }
            } else {
                self.statement_at_site(s);
                result = None;
                anchor_consumed = false;
            }
        }
        Ok((result, value_failed, anchor_consumed))
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
        if !self.tainted
            && internal_state(ty).is_none()
            && !matches!(ty, Type::Array(_))
            && let Some(id) = self.expression_variable(right)
        {
            let root = self.root(id);
            if self.type_nodes[&root].array_child.is_some() {
                return Ok(None);
            }
        }
        if !self.tainted
            && !self.failed_join_exprs.contains(&span_key(right.span()))
            && let Some(id) = self.expression_variable(left)
        {
            let root = self.root(id);
            if self.type_nodes[&root].array_child.is_some()
                && self.pending_operation_type(right).is_some_and(|right_ty| {
                    internal_state(&right_ty).is_none() && !matches!(right_ty, Type::Array(_))
                })
            {
                return Ok(None);
            }
        }
        let domain = if op == "<" {
            "I64"
        } else {
            "domain:copyable_data"
        };
        let variable = self.expression_variable(left);
        let complete = variable
            .and_then(|id| self.concrete(id).map(|(resolved, _)| resolved))
            .or_else(|| (internal_state(ty).is_none()).then_some(ty.clone()));
        if let Some(complete) = complete {
            let invalid = if op == "<" {
                complete != Type::I64
            } else {
                !copyable(&complete)
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
            if op == "<" && self.type_nodes[&root].array_child.is_none() {
                let operator = self.operator_span(left.span(), right.span(), op);
                self.unify_with_expected_origin(
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
            self.pending.push(PendingConstraint::BinaryDomain(Box::new(
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
                },
            )));
        }
        Ok(None)
    }

    fn independent_binary_rhs(
        &mut self,
        op: &str,
        left: &Expr,
        right: &Expr,
        left_domain_failed: bool,
    ) -> Result<bool, Box<Diagnostic>> {
        // ADR §4.1.2.4–5: a tainted left result suppresses only the cross
        // comparison. The right operand still owns its intrinsic domain.
        let numeric = matches!(op, "+" | "-");
        let prior_taint = self.tainted;
        self.tainted = false;
        let previous_numeric_owner = self.numeric_domain_owner;
        if numeric {
            self.numeric_domain_owner = Some((left.span().start, right.span().end));
        }
        let result = self.expr(
            right,
            (numeric && !left_domain_failed).then_some(&Type::I64),
            None,
        );
        let right_failed = result.is_err();
        self.numeric_domain_owner = previous_numeric_owner;
        let right_tainted = self.tainted;
        self.tainted |= prior_taint;
        match result {
            Ok(ty) if numeric && left_domain_failed && !right_tainted => {
                if let Some(id) = self.expression_variable(right)
                    && !self.type_determined(&ty)
                {
                    let operand = Self::primary_span(right);
                    self.unify_with_expected_origin(
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
                let wanted = if op == "<" {
                    "I64"
                } else {
                    "domain:copyable_data"
                };
                let variable = self.expression_variable(right);
                let complete = variable
                    .and_then(|id| self.concrete(id).map(|(resolved, _)| resolved))
                    .or_else(|| (internal_state(&ty).is_none()).then_some(ty));
                if let Some(complete) = complete {
                    let invalid = if op == "<" {
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
                    if op == "<" && self.type_nodes[&root].array_child.is_none() {
                        let operator = self.operator_span(left.span(), right.span(), op);
                        self.unify_with_expected_origin(
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
                        self.pending.push(PendingConstraint::BinaryDomain(Box::new(
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
        let declared_result = Type::from(&declared.result);
        let ordered = |index: usize| -> &Expr {
            &args[slots
                .iter()
                .position(|slot| *slot == index)
                .expect("complete binding")]
            .value
        };
        match name {
            "MockOpen" | "MockBlockingOpen" | "MockWriteI64" | "MockReply" => {
                self.expr(ordered(0), Some(&Type::I64), None)?;
                Ok(declared_result.clone())
            }
            "TryAdd" => {
                // Type checking is in written order even when labels permute slots.
                for arg in args {
                    self.expr(&arg.value, Some(&Type::I64), None)?;
                }
                Ok(declared_result.clone())
            }
            "MockRequest" | "MockUnknown" | "Fail" => Ok(declared_result.clone()),
            "MockReadRecords" => {
                let got = self.expr(ordered(0), None, None)?;
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
                let got = self.expr(ordered(0), None, None)?;
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
                self.expr(ordered(0), None, None)?;
                Ok(declared_result.clone())
            }
            "Len" => {
                let got = self.expr(ordered(0), None, None)?;
                let before = self.pending_domain_count();
                self.check_intrinsic_domain(ordered(0), &got, "domain:array")?;
                self.remember_pending_domain_result(
                    before,
                    call_span,
                    &declared_result,
                    expected,
                    expected_span,
                );
                if self.divergent_exprs.contains(&span_key(ordered(0).span())) {
                    self.deferred_exprs.insert(span_key(call_span));
                    self.divergent_exprs.insert(span_key(call_span));
                }
                Ok(declared_result.clone())
            }
            "Print" => {
                let got = self.expr(ordered(0), None, None)?;
                let before = self.pending_domain_count();
                self.check_intrinsic_domain(ordered(0), &got, "domain:copyable_data")?;
                self.remember_pending_domain_result(
                    before,
                    call_span,
                    &declared_result,
                    expected,
                    expected_span,
                );
                Ok(declared_result.clone())
            }
            "AssertEq" => {
                let mut values: [Option<Type>; 2] = [None, None];
                let mut evidence: [Option<PendingOperandEvidence>; 2] = [None, None];
                let mut invalid = false;
                for (arg, &slot) in args.iter().zip(slots) {
                    let prior_taint = self.tainted;
                    self.tainted = false;
                    let prior_errors = self.errors.len();
                    let pending_start = self.pending.len();
                    let pending_generation = self.pending_generation;
                    let result = self.expr(&arg.value, None, None);
                    let tainted = self.tainted;
                    let site = Self::primary_span(&arg.value);
                    let recover = (tainted || matches!(result, Ok(Type::Tainted)))
                        .then(|| {
                            self.conditional_result_types
                                .get(&span_key(&site))
                                .cloned()
                                .zip(self.pending_operand_evidence(
                                    &arg.value,
                                    pending_start,
                                    pending_generation,
                                ))
                        })
                        .flatten()
                        .filter(|_| {
                            self.errors.len() == prior_errors
                                && !self
                                    .failed_join_exprs
                                    .iter()
                                    .any(|&(start, end)| site.start <= start && end <= site.end)
                        });
                    self.tainted = prior_taint || (tainted && recover.is_none());
                    values[slot] = Some(match result {
                        Ok(_) if let Some((nominal, guards)) = recover => {
                            evidence[slot] = Some(guards);
                            nominal
                        }
                        Ok(ty)
                            if !tainted
                                && !self
                                    .failed_join_exprs
                                    .contains(&span_key(arg.value.span())) =>
                        {
                            ty
                        }
                        Ok(_) => {
                            invalid = true;
                            Type::Tainted
                        }
                        Err(error) if error.operation == Operation::AnalysisLimit => {
                            return Err(error);
                        }
                        Err(error) => {
                            self.record_error(error);
                            invalid = true;
                            Type::Tainted
                        }
                    });
                }
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
                    let composite = !composite_guard.domains.is_empty()
                        || !composite_guard.projections.is_empty();
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
                        self.pending
                            .push(PendingConstraint::GuardedComparison(Box::new(
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
                                },
                            )));
                    } else {
                        self.unify(
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
                        if let Err(error) = self.require_copyable(id, &Self::primary_span(operand))
                        {
                            if error.operation == Operation::AnalysisLimit {
                                return Err(error);
                            }
                            self.record_error(error);
                            invalid = true;
                        }
                    } else if internal_state(&value).is_none() && !copyable(&value) {
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
                self.tainted |= invalid;
                Ok(declared_result.clone())
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
                    return Ok(Type::Result(Arc::new(ok.clone()), Arc::new(err.clone())));
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

#[allow(clippy::vec_box)] // Semantic diagnostics retain the shared boxed representation.
pub(crate) fn check_with_partial(
    file: &str,
    source: &str,
    ast: &Ast,
    resolved: &Resolved,
) -> TypeCheckReport {
    check_with_partial_limit(file, source, ast, resolved, 1_000_000)
}

fn check_with_partial_limit(
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
            };
        }
    };
    let mut checker = Checker {
        file,
        tokens,
        resolved,
        locals: HashMap::new(),
        type_nodes: HashMap::new(),
        array_unify_cache: HashMap::new(),
        expr_vars: HashMap::new(),
        expression_types: HashMap::new(),
        conditional_result_types: HashMap::new(),
        conditional_result_vars: Vec::new(),
        next_variable: 0,
        failed_constructors: HashSet::new(),
        pending: Vec::new(),
        pending_generation: 0,
        has_pending_projection: false,
        numeric_domain_owner: None,
        pending_domain_calls: HashSet::new(),
        emitted_sites: HashSet::new(),
        solver_steps: 0,
        solver_limit,
        planning: false,
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
        deferred_join_roots: HashSet::new(),
        branch_join_sites: HashMap::new(),
        recursive_join_roots: HashSet::new(),
        block_depth: 0,
        top_statement_start: 0,
        top_statement_end: 0,
        active_if_span: None,
        preseeded_children: HashSet::new(),
        analysis_exhausted: false,
        quota_error: None,
        invalid: HashSet::new(),
        deferred_exprs: HashSet::new(),
        divergent_exprs: HashSet::new(),
        failed_join_exprs: HashSet::new(),
        failed_projection_results: HashSet::new(),
        context_rejections: HashSet::new(),
        partial_normal_exprs: HashSet::new(),
        tainted: false,
        result: Type::Unit,
        result_span: None,
        errors: Vec::new(),
    };
    let mut inferred = InferredTypes::default();
    for function in &ast.functions {
        if checker.analysis_exhausted {
            break;
        }
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
        for (&at, known) in checker.locals.clone().iter() {
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
                let resolved = match internal_state(&known.ty) {
                    Some(_) => ResolvedType::Unresolved,
                    None => syntax_type(&known.ty)
                        .map_or(ResolvedType::Unresolved, ResolvedType::Concrete),
                };
                inferred.bindings.insert(at, resolved);
            }
        }
        for (span, ty) in checker.expression_types.clone() {
            let resolved = match internal_state(&ty) {
                Some(Type::Never) => ResolvedType::Never,
                Some(Type::Tainted) => checker
                    .conditional_result_types
                    .get(&span)
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
        for (span, id) in checker.expr_vars.clone() {
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
            .pending
            .iter()
            .filter_map(|entry| match entry {
                PendingConstraint::BinaryDomain(domain) => Some(span_key(&domain.result)),
                PendingConstraint::DomainResult(result) => Some(span_key(&result.call)),
                _ => None,
            })
            .chain(checker.failed_join_exprs.iter().copied())
            .collect();
        checker.suppress_result_dependents(&tentative_results);
        checker
            .errors
            .retain(|error| error.operation != Operation::AnalysisLimit);
        checker.record_error(quota);
    }
    names::sort_diagnostics(&mut checker.errors);
    if !checker.errors.is_empty() {
        record_unresolved_sites(ast, &mut inferred);
    }
    TypeCheckReport {
        inferred,
        errors: checker.errors,
    }
}

#[cfg(test)]
mod quota_tests {
    use super::*;

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
            checker.emitted_sites,
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
            uses: HashMap::new(),
            calls: HashMap::new(),
        };
        let mut checker = empty_checker(&resolved);
        let outside = checker.new_variable(&site(0)).unwrap();
        let inside = checker.new_variable(&site(15)).unwrap();
        checker
            .pending
            .push(PendingConstraint::Domain(PendingDomain {
                variable: outside,
                operand: site(0),
                domain: "I64",
            }));
        let pending_start = checker.pending.len();
        let generation = checker.pending_generation;
        checker
            .pending
            .push(PendingConstraint::Domain(PendingDomain {
                variable: inside,
                operand: site(15),
                domain: "I64",
            }));
        let removed = checker.take_pending_kind(|entry| match entry {
            PendingConstraint::Domain(domain) if domain.variable == outside => Ok(domain),
            other => Err(other),
        });
        assert_eq!(removed.len(), 1);
        checker
            .pending
            .push(PendingConstraint::Domain(PendingDomain {
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
        let evidence = checker
            .pending_operand_evidence(&operand, pending_start, generation)
            .unwrap();
        assert_eq!(evidence.domains, vec![(inside, "I64")]);
    }

    #[test]
    fn solver_states_and_representatives_keep_their_distinct_rules() {
        let resolved = Resolved {
            signatures: HashMap::new(),
            uses: HashMap::new(),
            calls: HashMap::new(),
        };
        let mut checker = empty_checker(&resolved);
        assert_eq!(
            checker
                .unify(Type::Never, Type::I64, site(0), site(1), site(1), false)
                .unwrap(),
            Type::I64
        );
        assert_eq!(
            checker
                .unify(Type::Tainted, Type::I64, site(0), site(1), site(1), false)
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
            uses: HashMap::new(),
            calls: HashMap::new(),
        };
        let mut checker = empty_checker(&resolved);
        assert_eq!(
            checker
                .unify(Type::I64, Type::Never, site(1), site(2), site(3), false)
                .unwrap(),
            Type::I64
        );
        assert_eq!(
            checker
                .unify(Type::I64, Type::Tainted, site(4), site(5), site(6), false)
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
            uses: HashMap::new(),
            calls: HashMap::new(),
        };
        let mut checker = empty_checker(&resolved);
        let left = checker.new_variable(&site(1)).unwrap();
        let right = checker.new_variable(&site(2)).unwrap();
        checker
            .bind(left, Type::I64, site(3), site(4), false)
            .unwrap();
        checker
            .bind(right, Type::Bool, site(5), site(6), false)
            .unwrap();
        let error = checker
            .unify(
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
            uses: HashMap::new(),
            calls: HashMap::new(),
        };
        let mut checker = empty_checker(&resolved);
        let left = checker.new_variable(&site(1)).unwrap();
        let right = checker.new_variable(&site(2)).unwrap();
        checker
            .bind(left, Type::I64, site(10), site(11), false)
            .unwrap();
        checker
            .bind(right, Type::I64, site(20), site(21), false)
            .unwrap();
        checker.union(left, right, site(22)).unwrap();
        let error = checker
            .bind(left, Type::Bool, site(30), site(31), false)
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
        checker.require_copyable(left, &site(3)).unwrap();
        checker
            .bind(right, Type::MockResource, site(4), site(5), false)
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
            uses: HashMap::new(),
            calls: HashMap::new(),
        };
        let mut checker = empty_checker(&resolved);
        let left = checker.new_variable(&site(1)).unwrap();
        let right = checker.new_variable(&site(2)).unwrap();
        checker.solver_limit = checker.solver_steps + 1;
        let error = checker
            .unify(
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
            .bind(left, Type::Deferred(right), site(3), site(7), false)
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
            .bind(later, Type::I64, site(6), site(6), false)
            .unwrap();
        checker
            .bind(earlier, Type::Bool, wide.clone(), site(2), false)
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
            uses: HashMap::new(),
            calls: HashMap::new(),
        };
        let mut checker = empty_checker(&resolved);
        let child = checker.new_variable(&site(1)).unwrap();
        checker
            .bind(child, Type::Bool, site(1), site(1), false)
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
        assert!(checker.deferred_exprs.contains(&span_key(value.span())));
        assert!(checker.divergent_exprs.contains(&span_key(value.span())));
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
            .branch(then_block, Some(&Type::I64), None, false, None, false)
            .unwrap();
        assert_eq!(normal, None);
        assert!(!failed);
    }

    #[test]
    fn conflicting_union_reports_the_earliest_concrete_origin() {
        let resolved = Resolved {
            signatures: HashMap::new(),
            uses: HashMap::new(),
            calls: HashMap::new(),
        };
        let mut checker = empty_checker(&resolved);
        let a = checker.new_variable(&site(0)).unwrap();
        let b = checker.new_variable(&site(1)).unwrap();
        checker.bind(a, Type::I64, site(5), site(5), false).unwrap();
        checker
            .bind(b, Type::Bool, site(9), site(9), false)
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
            uses: HashMap::new(),
            calls: HashMap::new(),
        };
        let mut checker = empty_checker(&resolved);
        let root = checker.new_variable(&site(0)).unwrap();
        let mut ty = Type::I64;
        for _ in 0..127 {
            ty = Type::Array(Arc::new(ty));
        }
        checker.type_nodes.get_mut(&root).unwrap().concrete = Some((ty.clone(), site(0)));
        assert!(checker.check_one_inferred_depth(root, &site(0)).is_ok());
        ty = Type::Array(Arc::new(ty));
        checker.type_nodes.get_mut(&root).unwrap().concrete = Some((ty, site(0)));
        assert!(checker.check_one_inferred_depth(root, &site(0)).is_ok());

        let mut chain = root;
        checker.type_nodes.get_mut(&root).unwrap().concrete = None;
        for _ in 0..64 {
            let next = checker.new_variable(&site(1)).unwrap();
            checker.type_nodes.get_mut(&chain).unwrap().array_child = Some(next);
            chain = next;
        }
        let mut inner = Type::I64;
        for _ in 0..65 {
            inner = Type::Array(Arc::new(inner));
        }
        checker.type_nodes.get_mut(&chain).unwrap().concrete = Some((inner, site(2)));
        let error = checker
            .check_one_inferred_depth(root, &site(0))
            .unwrap_err();
        assert_eq!(error.data.get("observed"), Some(&DataValue::UInt(129)));
    }

    #[test]
    fn inferred_array_root_retains_depth_origin_after_later_constraint() {
        let resolved = Resolved {
            signatures: HashMap::new(),
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
        checker.type_nodes.get_mut(&child).unwrap().concrete = Some((ty, site(1)));
        let error = checker.check_inferred_depth().unwrap_err();
        assert_eq!(error.data.get("observed"), Some(&DataValue::UInt(129)));
    }

    #[test]
    fn deferred_visible_conversion_and_constructor_walk_stop_at_their_boundaries() {
        let resolved = Resolved {
            signatures: HashMap::new(),
            uses: HashMap::new(),
            calls: HashMap::new(),
        };
        let mut checker = empty_checker(&resolved);
        let scalar = checker.new_variable(&site(0)).unwrap();
        checker
            .bind(scalar, Type::I64, site(1), site(1), false)
            .unwrap();
        assert_eq!(
            checker.resolved_syntax_type(&Type::Deferred(scalar)),
            Some(SyntaxType::I64)
        );

        let root = checker.new_variable(&site(2)).unwrap();
        let mut current = root;
        for _ in 0..129 {
            let child = checker.new_variable(&site(3)).unwrap();
            checker.type_nodes.get_mut(&current).unwrap().array_child = Some(child);
            current = child;
        }
        checker.type_nodes.get_mut(&current).unwrap().concrete = Some((Type::I64, site(4)));
        assert!(checker.concrete(root).is_none());
    }

    #[test]
    fn constructor_walk_keeps_exact_depth_boundaries() {
        let resolved = Resolved {
            signatures: HashMap::new(),
            uses: HashMap::new(),
            calls: HashMap::new(),
        };
        for length in [127, 128, 129] {
            let mut checker = empty_checker(&resolved);
            let root = checker.new_variable(&site(0)).unwrap();
            let mut current = root;
            for _ in 0..length {
                let next = checker.new_variable(&site(1)).unwrap();
                checker.type_nodes.get_mut(&current).unwrap().array_child = Some(next);
                current = next;
            }
            checker.type_nodes.get_mut(&current).unwrap().concrete = Some((Type::I64, site(2)));
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
            .bind(element, Type::Bool, site(2), site(2), false)
            .unwrap();
        assert_eq!(
            checker.resolved_syntax_type(&array),
            Some(SyntaxType::Array(Box::new(SyntaxType::Bool)))
        );
        assert_eq!(
            checker
                .unify(Type::Never, array.clone(), site(3), site(4), site(4), false)
                .unwrap(),
            array
        );
        assert_eq!(
            checker
                .unify(Type::Tainted, Type::I64, site(5), site(6), site(6), false)
                .unwrap(),
            Type::Tainted
        );
    }

    #[test]
    fn deferred_result_components_and_structural_array_elements_resolve_separately() {
        let resolved = Resolved {
            signatures: HashMap::new(),
            uses: HashMap::new(),
            calls: HashMap::new(),
        };
        let mut checker = empty_checker(&resolved);
        let ok = checker.new_variable(&site(0)).unwrap();
        let err = checker.new_variable(&site(1)).unwrap();
        let result = Type::Result(Arc::new(Type::Deferred(ok)), Arc::new(Type::Deferred(err)));
        assert_eq!(checker.resolved_syntax_type(&result), None);
        checker
            .bind(ok, Type::I64, site(2), site(2), false)
            .unwrap();
        checker
            .bind(err, Type::Bool, site(3), site(3), false)
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
            .bind(
                array,
                Type::Array(Arc::new(Type::I64)),
                site(5),
                site(5),
                false,
            )
            .unwrap();
        let mismatch = checker
            .bind(
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
                checker.divergent_exprs.contains(&span_key(value.span())),
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
            locals: HashMap::new(),
            type_nodes: HashMap::new(),
            array_unify_cache: HashMap::new(),
            expr_vars: HashMap::new(),
            expression_types: HashMap::new(),
            conditional_result_types: HashMap::new(),
            conditional_result_vars: Vec::new(),
            next_variable: 0,
            failed_constructors: HashSet::new(),
            pending: Vec::new(),
            pending_generation: 0,
            has_pending_projection: false,
            numeric_domain_owner: None,
            pending_domain_calls: HashSet::new(),
            emitted_sites: HashSet::new(),
            solver_steps: 0,
            solver_limit: 1_000_000,
            planning: false,
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
            deferred_join_roots: HashSet::new(),
            branch_join_sites: HashMap::new(),
            recursive_join_roots: HashSet::new(),
            block_depth: 0,
            top_statement_start: 0,
            top_statement_end: 0,
            active_if_span: None,
            preseeded_children: HashSet::new(),
            analysis_exhausted: false,
            quota_error: None,
            invalid: HashSet::new(),
            deferred_exprs: HashSet::new(),
            divergent_exprs: HashSet::new(),
            failed_join_exprs: HashSet::new(),
            failed_projection_results: HashSet::new(),
            context_rejections: HashSet::new(),
            partial_normal_exprs: HashSet::new(),
            tainted: false,
            result: Type::Unit,
            result_span: None,
            errors: Vec::new(),
        }
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
    fn internal_tainted_state_is_recorded_at_the_expression_boundary() {
        let resolved = Resolved {
            signatures: HashMap::new(),
            uses: HashMap::new(),
            calls: HashMap::new(),
        };
        let mut checker = empty_checker(&resolved);
        let expr = Expr::Unary {
            mode: Mode::Read,
            value: Box::new(Expr::Bool(true, site(1))),
            span: site(0),
        };
        assert_eq!(checker.expr(&expr, None, None).unwrap(), Type::Tainted);
        assert!(checker.tainted);
    }

    #[test]
    fn union_reports_resolved_array_constructor_in_both_root_orders() {
        let resolved = Resolved {
            signatures: HashMap::new(),
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
                .type_nodes
                .get_mut(&array)
                .unwrap()
                .array_constructed = true;
            checker
                .bind(child, Type::I64, site(3), site(3), false)
                .unwrap();
            checker
                .bind(scalar, Type::I64, site(4), site(4), false)
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
            .bind(later, Type::I64, later_origin, site(30), false)
            .unwrap();
        checker
            .bind(earlier, Type::Bool, earlier_origin.clone(), site(31), false)
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
            uses: HashMap::new(),
            calls: HashMap::new(),
        };
        let i64_array = Type::Array(Arc::new(Type::I64));
        let bool_array = Type::Array(Arc::new(Type::Bool));
        for change_left in [true, false] {
            let mut checker = empty_checker(&resolved);
            checker
                .unify(
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
                .unify(left, right, site(3), site(4), site(5), false)
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
            .unify(
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
            .unify(left, right, site(3), site(4), site(5), false)
            .unwrap();
        assert_eq!(checker.solver_steps, 66);
    }

    #[test]
    fn unresolved_constructor_pair_is_rechecked_after_its_child_resolves() {
        // A mixed array pair can defer a scalar/array shape conflict. It may
        // be cached only when both operands were fully concrete at entry.
        let resolved = Resolved {
            signatures: HashMap::new(),
            uses: HashMap::new(),
            calls: HashMap::new(),
        };
        let mut checker = empty_checker(&resolved);
        let constructor = checker.new_variable(&site(1)).unwrap();
        let child = checker.require_array(constructor, &site(2)).unwrap();
        checker
            .type_nodes
            .get_mut(&constructor)
            .unwrap()
            .array_constructed = true;
        let left = Type::Array(Arc::new(Type::Deferred(constructor)));
        let right = Type::Array(Arc::new(Type::I64));
        let first = checker.unify(
            left.clone(),
            right.clone(),
            site(3),
            site(4),
            site(5),
            false,
        );
        assert!(first.is_ok(), "{first:?}");
        checker
            .bind(child, Type::I64, site(6), site(6), false)
            .unwrap();
        let second = checker.unify(left, right, site(3), site(4), site(7), false);
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
            .unify(
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
            .unify(left, right, site(1), site(1), site(1), false)
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
        let reach = checker.array_reaches(root, child, &site(3)).unwrap_err();
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
            uses: HashMap::new(),
            calls: HashMap::new(),
        };
        for marked_first in [false, true] {
            let mut checker = empty_checker(&resolved);
            let first = checker.new_variable(&site(0)).unwrap();
            let second = checker.new_variable(&site(1)).unwrap();
            let marked = if marked_first { first } else { second };
            checker.type_nodes.get_mut(&marked).unwrap().bottom_only = true;
            checker.union(first, second, site(2)).unwrap();
            let root = checker.root(first);
            assert!(checker.type_nodes[&root].bottom_only);
        }
    }

    #[test]
    fn adr_gap_shape_walk_charge_per_constructor_link() {
        // The current shape walk charges 128 traversed links as 128 units
        // (adr_gap_shape_walk_charge); §11.1 does not fix that count.
        // One fewer granted unit rejects the final link with limit+1.
        let resolved = Resolved {
            signatures: HashMap::new(),
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
        assert!(checker.array_reaches(first, last, &site(200)).unwrap());
        assert_eq!(checker.solver_steps, before + 128);
        checker.solver_limit += 127;
        let denied = checker.array_reaches(first, last, &site(201)).unwrap_err();
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
            .stack_size(16 * 1024 * 1024)
            .spawn(default_solver_limit_on_wide_stack)
            .unwrap()
            .join()
            .unwrap();
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
            .unify(
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
            .unify(
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
                        0 => checker.bind(a, Type::I64, origin(0), origin(0), false),
                        1 => checker.union(a, b, origin(1)),
                        _ => checker.bind(
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
                        1 => checker.bind(ids[left], Type::I64, at.clone(), at, false),
                        2 => checker.bind(ids[left], Type::Bool, at.clone(), at, false),
                        3 => checker.bind(
                            ids[left],
                            Type::Array(Arc::new(Type::I64)),
                            at.clone(),
                            at,
                            false,
                        ),
                        _ => checker.bind(
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
}
