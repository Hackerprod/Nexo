//! Persistent ownership facts. Only paths leading to changed locals are copied.
//! Edge distance and backedge witnesses are deferred until a fact is inspected.
use super::*;
use std::cmp::Reverse;
use std::collections::BinaryHeap;
use std::collections::HashMap;

fn work_priority(index: usize) -> Reverse<usize> {
    Reverse(index)
}
use std::rc::Rc;

#[cfg(test)]
struct StateCountGuard;

#[cfg(test)]
impl Drop for StateCountGuard {
    fn drop(&mut self) {
        FLOW_STATE_COUNT_ENABLED.with(|enabled| enabled.set(false));
    }
}

#[derive(Clone)]
enum EdgeList {
    One(Span),
    Pair(Rc<EdgeList>, Rc<EdgeList>),
}

#[derive(Clone, Default)]
struct Lazy {
    distance: u32,
    edges: Option<Rc<EdgeList>>,
    maybe: bool,
}

impl Lazy {
    fn is_empty(&self) -> bool {
        self.distance == 0 && self.edges.is_none() && !self.maybe
    }

    fn then(&self, later: &Self) -> Self {
        let edges = match (&self.edges, &later.edges) {
            (Some(a), Some(b)) => Some(Rc::new(EdgeList::Pair(a.clone(), b.clone()))),
            (Some(a), None) => Some(a.clone()),
            (None, Some(b)) => Some(b.clone()),
            (None, None) => None,
        };
        Self {
            distance: self.distance.saturating_add(later.distance),
            edges,
            maybe: self.maybe || later.maybe,
        }
    }

    fn apply(&self, fact: &mut Fact) {
        if self.maybe {
            fact.definite = false;
        }
        for site in fact.sites.values_mut() {
            site.distance = site.distance.saturating_add(self.distance);
            if let Some(edges) = &self.edges {
                let mut pending = vec![edges.as_ref()];
                while let Some(edge) = pending.pop() {
                    match edge {
                        EdgeList::One(span) => {
                            if !site.backedges.contains(span) {
                                site.backedges.push(span.clone());
                            }
                        }
                        EdgeList::Pair(a, b) => {
                            pending.push(b.as_ref());
                            pending.push(a.as_ref());
                        }
                    }
                }
            }
        }
    }
}

// A fixed-depth binary trie makes insertion independent of the number of
// bindings already present. Empty subtrees are represented by None.
#[derive(Clone)]
struct Node {
    left: Option<Rc<Node>>,
    right: Option<Rc<Node>>,
    fact: Option<Fact>,
    lazy: Lazy,
    facts: u64,
    sites: u64,
    definite: u64,
}

impl Node {
    fn allocated(node: Self) -> Rc<Self> {
        #[cfg(test)]
        FLOW_STATE_COUNT_ENABLED.with(|enabled| {
            if enabled.get() {
                FLOW_STATE_ALLOCS.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                FLOW_STATE_BYTES.fetch_add(
                    std::mem::size_of::<Self>() as u64,
                    std::sync::atomic::Ordering::Relaxed,
                );
            }
        });
        Rc::new(node)
    }

    fn leaf(fact: Fact) -> Rc<Self> {
        Self::allocated(Self {
            left: None,
            right: None,
            facts: 1,
            sites: fact.sites.len() as u64,
            definite: u64::from(fact.definite),
            fact: Some(fact),
            lazy: Lazy::default(),
        })
    }

    fn branch(left: Option<Rc<Self>>, right: Option<Rc<Self>>) -> Option<Rc<Self>> {
        if left.is_none() && right.is_none() {
            return None;
        }
        let facts = left.as_ref().map_or(0, |n| n.facts) + right.as_ref().map_or(0, |n| n.facts);
        let sites = left.as_ref().map_or(0, |n| n.sites) + right.as_ref().map_or(0, |n| n.sites);
        let definite = left.as_ref().map_or(0, |n| n.effective_definite())
            + right.as_ref().map_or(0, |n| n.effective_definite());
        Some(Self::allocated(Self {
            left,
            right,
            fact: None,
            lazy: Lazy::default(),
            facts,
            sites,
            definite,
        }))
    }

    fn effective_definite(&self) -> u64 {
        if self.lazy.maybe { 0 } else { self.definite }
    }

    fn with_lazy(node: &Rc<Self>, later: &Lazy) -> Rc<Self> {
        if later.is_empty() {
            return node.clone();
        }
        Self::allocated(Self {
            lazy: node.lazy.then(later),
            ..(**node).clone()
        })
    }
}

#[derive(Clone)]
struct SparseState {
    root: Option<Rc<Node>>,
    depth: u32,
}

impl SparseState {
    fn new(locals: usize) -> Self {
        Self {
            root: None,
            depth: (usize::BITS - locals.saturating_sub(1).leading_zeros()).max(16),
        }
    }

    fn edge(&mut self, span: Option<Span>) {
        if let Some(root) = &self.root {
            let lazy = Lazy {
                distance: 1,
                edges: span.map(|s| Rc::new(EdgeList::One(s))),
                maybe: false,
            };
            self.root = Some(Node::with_lazy(root, &lazy));
        }
    }

    fn get(&self, local: usize) -> Option<Fact> {
        let mut node = self.root.as_ref()?;
        let mut accumulated = Lazy::default();
        for bit in (0..self.depth).rev() {
            accumulated = node.lazy.then(&accumulated);
            node = if (local >> bit) & 1 == 0 {
                node.left.as_ref()?
            } else {
                node.right.as_ref()?
            };
        }
        accumulated = node.lazy.then(&accumulated);
        let mut fact = node.fact.clone()?;
        accumulated.apply(&mut fact);
        Some(fact)
    }

    fn set(&mut self, local: usize, fact: Option<Fact>) {
        self.root = set_node(self.root.as_ref(), self.depth, local, fact);
    }
}

fn set_node(
    node: Option<&Rc<Node>>,
    depth: u32,
    local: usize,
    fact: Option<Fact>,
) -> Option<Rc<Node>> {
    if depth == 0 {
        return fact.map(Node::leaf);
    }
    let bit = depth - 1;
    let mut left = node.and_then(|n| n.left.clone());
    let mut right = node.and_then(|n| n.right.clone());
    if let Some(n) = node {
        if !n.lazy.is_empty() && left.is_some() {
            left = left.map(|child| Node::with_lazy(&child, &n.lazy));
        }
        if !n.lazy.is_empty() && right.is_some() {
            right = right.map(|child| Node::with_lazy(&child, &n.lazy));
        }
    }
    if (local >> bit) & 1 == 0 {
        left = set_node(left.as_ref(), bit, local, fact);
    } else {
        right = set_node(right.as_ref(), bit, local, fact);
    }
    Node::branch(left, right)
}

fn sparse_transfer(
    state: &mut SparseState,
    ins: &super::super::model::Instruction,
    function: &Function,
) {
    match ins.op {
        Op::WritePlace { local, .. } | Op::DropLocal(local) => state.set(local.0, None),
        Op::TransferValue(local) | Op::DropBinding(local) => {
            if function.locals[local.0].name.is_empty() {
                return;
            }
            let site = Site {
                span: ins.span.clone(),
                is_drop: matches!(ins.op, Op::DropBinding(_)),
                backedges: Vec::new(),
                distance: 0,
            };
            state.set(
                local.0,
                Some(Fact {
                    definite: true,
                    sites: BTreeMap::from([((site.span.start, site.span.end, site.is_drop), site)]),
                }),
            );
        }
        _ => {}
    }
}

fn join_fact(
    old: &mut Fact,
    incoming: &Fact,
    budget: &mut Budget,
    span: &Span,
) -> Result<bool, CompileError> {
    let mut changed = false;
    budget.charge(span)?;
    if !incoming.definite && old.definite {
        old.definite = false;
        changed = true;
    }
    for (&key, site) in &incoming.sites {
        budget.charge(span)?;
        match old.sites.get_mut(&key) {
            Some(prior) if site.distance < prior.distance => {
                *prior = site.clone();
                changed = true;
            }
            None => {
                old.sites.insert(key, site.clone());
                changed = true;
            }
            _ => {}
        }
    }
    Ok(changed)
}

fn join_node(
    into: Option<&Rc<Node>>,
    from: Option<&Rc<Node>>,
    depth: u32,
    into_after: &Lazy,
    from_after: &Lazy,
    budget: &mut Budget,
    span: &Span,
) -> Result<(Option<Rc<Node>>, bool), CompileError> {
    match (into, from) {
        (None, None) => Ok((None, false)),
        (Some(old), None) => {
            budget.charge(span)?;
            let effective = old.lazy.then(into_after);
            let changed = !effective.maybe && old.definite > 0;
            let added = Lazy {
                maybe: true,
                ..into_after.clone()
            };
            Ok((Some(Node::with_lazy(old, &added)), changed))
        }
        (None, Some(new)) => {
            budget.charge(span)?;
            let added = Lazy {
                maybe: true,
                ..from_after.clone()
            };
            Ok((Some(Node::with_lazy(new, &added)), true))
        }
        (Some(old), Some(new)) if Rc::ptr_eq(old, new) => {
            budget.charge(span)?;
            let old_effective = old.lazy.then(into_after);
            let new_effective = new.lazy.then(from_after);
            let shorter = new_effective.distance < old_effective.distance;
            let changes_definite = !old_effective.maybe && new_effective.maybe && old.definite > 0;
            let changed = shorter || changes_definite;
            let selected = if shorter { from_after } else { into_after };
            let result = Lazy {
                maybe: old_effective.maybe || new_effective.maybe,
                ..selected.clone()
            };
            if changed {
                Ok((Some(Node::with_lazy(old, &result)), true))
            } else {
                Ok((Some(Node::with_lazy(old, into_after)), false))
            }
        }
        (Some(old), Some(new)) if depth == 0 => {
            let old_lazy = old.lazy.then(into_after);
            let new_lazy = new.lazy.then(from_after);
            let mut old_fact = old.fact.clone().expect("ownership trie leaf");
            let mut new_fact = new.fact.clone().expect("ownership trie leaf");
            old_lazy.apply(&mut old_fact);
            new_lazy.apply(&mut new_fact);
            let changed = join_fact(&mut old_fact, &new_fact, budget, span)?;
            Ok((Some(Node::leaf(old_fact)), changed))
        }
        (Some(old), Some(new)) => {
            let old_lazy = old.lazy.then(into_after);
            let new_lazy = new.lazy.then(from_after);
            let (left, changed_left) = join_node(
                old.left.as_ref(),
                new.left.as_ref(),
                depth - 1,
                &old_lazy,
                &new_lazy,
                budget,
                span,
            )?;
            let (right, changed_right) = join_node(
                old.right.as_ref(),
                new.right.as_ref(),
                depth - 1,
                &old_lazy,
                &new_lazy,
                budget,
                span,
            )?;
            let changed = changed_left || changed_right;
            if changed {
                Ok((Node::branch(left, right), true))
            } else {
                Ok((Some(Node::with_lazy(old, into_after)), false))
            }
        }
    }
}

fn join_sparse(
    into: &mut SparseState,
    from: &SparseState,
    budget: &mut Budget,
    span: &Span,
) -> Result<bool, CompileError> {
    let (root, changed) = join_node(
        into.root.as_ref(),
        from.root.as_ref(),
        into.depth,
        &Lazy::default(),
        &Lazy::default(),
        budget,
        span,
    )?;
    if changed {
        into.root = root;
    }
    Ok(changed)
}

// At a loop header, facts outside the loop's dense touched set have the same
// source value along the entry and backedge. The entry path is shorter, so the
// backedge cannot improve their witness. Keep that part of the trie by reference.
fn join_loop(
    into: &mut SparseState,
    from: &SparseState,
    touched: &[LoopEffect],
    budget: &mut Budget,
    span: &Span,
) -> Result<bool, CompileError> {
    let mut changed = false;
    for effect in touched {
        if !(effect.read || effect.generated || effect.kill) {
            continue;
        }
        let old = into.get(effect.local);
        let incoming = from.get(effect.local);
        match (old, incoming) {
            (None, None) => {}
            (Some(mut old), None) => {
                budget.charge(span)?;
                if old.definite {
                    old.definite = false;
                    into.set(effect.local, Some(old));
                    changed = true;
                }
            }
            (None, Some(mut incoming)) => {
                budget.charge(span)?;
                incoming.definite = false;
                into.set(effect.local, Some(incoming));
                changed = true;
            }
            (Some(mut old), Some(incoming)) => {
                if join_fact(&mut old, &incoming, budget, span)? {
                    into.set(effect.local, Some(old));
                    changed = true;
                }
            }
        }
    }
    Ok(changed)
}

struct Backedge {
    closing: Span,
    // Dense, loop-local index of every source binding read, consumed, written
    // or borrowed by the condition and body. The two bits summarize transfer.
    touched: Vec<LoopEffect>,
}

impl Backedge {
    fn cross(&self, state: &mut SparseState) {
        for effect in &self.touched {
            // A loop-local generation belongs to this iteration. Its
            // successor starts with a fresh declaration, regardless of
            // which path subsequently killed that generation.
            if effect.declared_here && (effect.generated || effect.kill) {
                state.set(effect.local, None);
            }
        }
        state.edge(Some(self.closing.clone()));
    }
}

struct LoopEffect {
    local: usize,
    read: bool,
    generated: bool,
    kill: bool,
    declared_here: bool,
}

fn summarize_backedge(
    blocks: &[&super::super::model::BasicBlock],
    source: usize,
    target: usize,
    name_spans: &[Span],
    budget: &mut Budget,
) -> Result<Backedge, CompileError> {
    // lower::build_while creates a Jump backedge with a preceding Jump
    // preheader. verify_function rejects malformed targets/cycles in forged
    // IR; semantic_cfg_sources_never_create_invalid_ir and the generated
    // sparse/dense comparison exercise accepted source CFGs.
    let Terminator::Jump { span, .. } = &blocks[source].terminator else {
        return Err(CompileError::InvalidIr("invalid control-flow cycle"));
    };
    let loop_span = target
        .checked_sub(1)
        .and_then(|preheader| {
            if let Terminator::Jump { target: dest, span } = &blocks[preheader].terminator {
                (dest.0 == target).then_some(span)
            } else {
                None
            }
        })
        .ok_or(CompileError::InvalidIr("invalid control-flow cycle"))?;
    let mut dense = HashMap::<usize, usize>::new();
    let mut touched = Vec::<LoopEffect>::new();
    for member in &blocks[target..=source] {
        for ins in &member.instructions {
            budget.charge(&ins.span)?;
            let local = match ins.op {
                Op::WritePlace { local, .. } | Op::DropLocal(local) => Some(local),
                _ => use_local(&ins.op),
            };
            let Some(local) = local else {
                continue;
            };
            let slot = *dense.entry(local.0).or_insert_with(|| {
                let slot = touched.len();
                let declared = &name_spans[local.0];
                touched.push(LoopEffect {
                    local: local.0,
                    read: false,
                    generated: false,
                    kill: false,
                    declared_here: declared.start >= loop_span.start
                        && declared.end <= loop_span.end,
                });
                slot
            });
            touched[slot].read |= use_local(&ins.op).is_some();
            touched[slot].generated |= matches!(ins.op, Op::TransferValue(_) | Op::DropBinding(_));
            touched[slot].kill |= matches!(ins.op, Op::WritePlace { .. } | Op::DropLocal(_));
        }
    }
    Ok(Backedge {
        closing: closing_span(span),
        touched,
    })
}

pub(in crate::ir) fn analyze(
    function: &Function,
    name_spans: &[Span],
    tokens: &[Token],
    used: &mut u64,
) -> Result<Vec<Diagnostic>, CompileError> {
    let blocks = std::iter::once(&function.block).chain(&function.blocks);
    // Only these instructions create ownership facts. Without a source fact,
    // every incoming state stays empty, so a fixed-point walk cannot report
    // a use after consumption. Charge the single inspection pass to the same
    // budget and leave the full analysis for functions that can generate facts.
    if !blocks.clone().any(|block| {
        block
            .instructions
            .iter()
            .any(|ins| matches!(ins.op, Op::TransferValue(_) | Op::DropBinding(_)))
    }) {
        let mut budget = Budget { used: *used };
        for (index, block) in blocks.enumerate() {
            if index > 0 {
                let terminator_span = match &block.terminator {
                    Terminator::Return { span, .. }
                    | Terminator::Jump { span, .. }
                    | Terminator::BranchBool { span, .. } => span,
                };
                budget.charge(terminator_span)?;
            }
            for ins in &block.instructions {
                budget.charge(&ins.span)?;
            }
        }
        *used = budget.used;
        return Ok(Vec::new());
    }
    let mut observed = Vec::new();
    let result = analyze_inner(function, name_spans, tokens, used, &mut observed);
    match result {
        Err(CompileError::Rejected(mut limit)) => {
            observed.extend(limit.drain(..).map(|d| *d));
            canonicalize(&mut observed);
            Err(CompileError::Rejected(
                observed.into_iter().map(Box::new).collect(),
            ))
        }
        other => other,
    }
}

// A source use has one diagnostic. If distinct stable paths name that same
// use, keep the joined (conditional) result and its matching causal evidence.
fn canonicalize(errors: &mut Vec<Diagnostic>) {
    errors.sort_by_key(|d| d.primary_span.as_ref().map(|s| (s.start, s.end)));
    errors.dedup_by(|later, earlier| {
        let same_use = later.primary_span == earlier.primary_span
            && later.code == earlier.code
            && later.operation == earlier.operation;
        if same_use && later.subcause.as_deref() == Some("maybe_consumed") {
            *earlier = later.clone();
        }
        same_use
    });
}

// A prefix is stable once all work that can flow backwards into it has
// finished. An unfinished loop can still change every block from its header
// through its backedge, even if one of those blocks was visited earlier.
fn stable_prefix(
    blocks: &[&super::super::model::BasicBlock],
    current: usize,
    queued: &[bool],
) -> usize {
    let mut end = queued
        .iter()
        .enumerate()
        .filter_map(|(index, pending)| pending.then_some(index))
        .min()
        .map_or(current, |index| index.min(current));
    for (source, block) in blocks.iter().enumerate() {
        if source < end {
            continue;
        }
        if let Terminator::Jump { target, .. } = &block.terminator
            && target.0 <= source
            && target.0 < end
        {
            end = target.0;
        }
    }
    end
}

fn collect_diagnostics(
    function: &Function,
    name_spans: &[Span],
    tokens: &[Token],
    blocks: &[&super::super::model::BasicBlock],
    incoming: &[Option<SparseState>],
    end: usize,
) -> Vec<Diagnostic> {
    let mut errors = Vec::new();
    for (index, block) in blocks.iter().enumerate().take(end) {
        let Some(mut state) = incoming[index].clone() else {
            continue;
        };
        for ins in &block.instructions {
            if let Some(local) = use_local(&ins.op)
                && !function.locals[local.0].name.is_empty()
                && let Some(fact) = state.get(local.0)
            {
                errors.push(diagnostic(function, name_spans, tokens, local, ins, &fact));
            }
            sparse_transfer(&mut state, ins, function);
        }
    }
    canonicalize(&mut errors);
    errors
}

fn analyze_inner(
    function: &Function,
    name_spans: &[Span],
    tokens: &[Token],
    used: &mut u64,
    observed: &mut Vec<Diagnostic>,
) -> Result<Vec<Diagnostic>, CompileError> {
    #[cfg(test)]
    let original_used = *used;
    #[cfg(test)]
    FLOW_STATE_COUNT_ENABLED.with(|enabled| enabled.set(function.source_file == "sparse_scale.n"));
    #[cfg(test)]
    let _state_count_guard = StateCountGuard;
    let blocks: Vec<_> = std::iter::once(&function.block)
        .chain(&function.blocks)
        .collect();
    let mut budget = Budget { used: *used };
    let mut backedges: Vec<Option<Backedge>> = (0..blocks.len()).map(|_| None).collect();
    let mut incoming = vec![None::<SparseState>; blocks.len()];
    incoming[0] = Some(SparseState::new(function.locals.len()));
    // CFG construction places a loop body before its exit. Smallest index
    // first lets a changed backedge converge before the exit is propagated.
    let mut queue = BinaryHeap::from([(work_priority(0), 0)]);
    let mut queued = vec![false; blocks.len()];
    let mut current = 0;
    let fixed_point = (|| -> Result<(), CompileError> {
        while let Some((_, index)) = queue.pop() {
            current = index;
            queued[index] = false;
            let block = blocks[index];
            let mut state = incoming[index].clone().expect("queued block has input");
            for ins in &block.instructions {
                budget.charge(&ins.span)?;
                sparse_transfer(&mut state, ins, function);
            }
            let mut propagate = |target: usize| -> Result<(), CompileError> {
                // Targets are allocated from the current function's block
                // table. The guards remain for forged CFGs tested by the
                // verifier; accepted while/if sources exercise this closure.
                if target >= blocks.len() {
                    return Err(CompileError::InvalidIr("invalid jump target"));
                }
                let mut edge_state = state.clone();
                if target <= index {
                    if backedges[index].is_none() {
                        backedges[index] = Some(summarize_backedge(
                            &blocks,
                            index,
                            target,
                            name_spans,
                            &mut budget,
                        )?);
                    }
                    let edge = backedges[index]
                        .as_ref()
                        .ok_or(CompileError::InvalidIr("invalid control-flow cycle"))?;
                    edge.cross(&mut edge_state);
                } else {
                    edge_state.edge(None);
                }
                let changed = if let Some(prior) = &mut incoming[target] {
                    let span = match &blocks[target].terminator {
                        Terminator::Return { span, .. }
                        | Terminator::Jump { span, .. }
                        | Terminator::BranchBool { span, .. } => span,
                    };
                    if target <= index {
                        let edge = backedges[index].as_ref().expect("validated backedge");
                        join_loop(prior, &edge_state, &edge.touched, &mut budget, span)?
                    } else {
                        join_sparse(prior, &edge_state, &mut budget, span)?
                    }
                } else {
                    incoming[target] = Some(edge_state);
                    true
                };
                if changed && !queued[target] {
                    queue.push((work_priority(target), target));
                    queued[target] = true;
                }
                Ok(())
            };
            match &block.terminator {
                Terminator::Return { .. } => {}
                Terminator::Jump { target, .. } => propagate(target.0)?,
                Terminator::BranchBool { yes, no, .. } => {
                    propagate(yes.0)?;
                    propagate(no.0)?;
                }
            }
        }
        Ok(())
    })();
    if let Err(error) = fixed_point {
        if matches!(error, CompileError::Rejected(_)) {
            let end = stable_prefix(&blocks, current, &queued);
            *observed = collect_diagnostics(function, name_spans, tokens, &blocks, &incoming, end);
        }
        return Err(error);
    }
    let mut errors = Vec::new();
    for (index, block) in blocks.iter().enumerate() {
        let Some(mut state) = incoming[index].clone() else {
            continue;
        };
        for ins in &block.instructions {
            if let Err(error) = budget.charge(&ins.span) {
                if matches!(error, CompileError::Rejected(_)) {
                    *observed = collect_diagnostics(
                        function,
                        name_spans,
                        tokens,
                        &blocks,
                        &incoming,
                        blocks.len(),
                    );
                }
                return Err(error);
            }
            if let Some(local) = use_local(&ins.op)
                && !function.locals[local.0].name.is_empty()
                && let Some(fact) = state.get(local.0)
            {
                errors.push(diagnostic(function, name_spans, tokens, local, ins, &fact));
            }
            sparse_transfer(&mut state, ins, function);
        }
    }
    canonicalize(&mut errors);
    *used = budget.used;
    #[cfg(test)]
    if function.source_file == "sparse_dense_oracle.n" {
        let mut dense_used = original_used;
        let dense = super::analyze_dense(function, name_spans, tokens, &mut dense_used).unwrap();
        assert_eq!(
            errors, dense,
            "sparse and dense ownership diagnostics differ"
        );
    }
    Ok(errors)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn span(start: u64) -> Span {
        Span {
            file: "sparse_witness.n".into(),
            start,
            end: start + 1,
        }
    }

    fn consumed(start: u64) -> Fact {
        let site = Site {
            span: span(start),
            is_drop: true,
            backedges: Vec::new(),
            distance: 0,
        };
        Fact {
            definite: true,
            sites: BTreeMap::from([((start, start + 1, true), site)]),
        }
    }

    #[test]
    fn leaf_lazy_is_materialized_when_read_after_an_edge() {
        let mut state = SparseState::new(1);
        state.set(0, Some(consumed(1)));
        state.edge(Some(span(10)));
        // Rewriting one empty sibling at each level pushes the deferred edge
        // all the way to the leaf without changing the surviving fact.
        for bit in (0..16).rev() {
            state.set(1 << bit, None);
        }
        let fact = state.get(0).unwrap();
        assert!(fact.definite);
        let site = fact.sites.values().next().unwrap();
        assert_eq!(site.distance, 1);
        assert_eq!(site.backedges, [span(10)]);
    }

    #[test]
    fn right_subtree_retains_deferred_edge_when_left_is_rewritten() {
        let mut state = SparseState::new(131_073);
        for local in 0..130 {
            state.set(local * 1024, Some(consumed(local as u64 + 1)));
        }
        state.edge(Some(span(900)));
        for local in 0..65 {
            state.set(local * 1024, None);
        }
        let fact = state.get(129 * 1024).unwrap();
        let site = fact.sites.values().next().unwrap();
        assert_eq!(site.distance, 1);
        assert_eq!(site.backedges, [span(900)]);
        // The trie must address locals beyond its former 16-bit floor.
        assert!(state.get(65 * 1024).is_some());
    }

    #[test]
    fn joining_a_repeated_site_keeps_its_shorter_witness() {
        let mut old = consumed(1);
        old.sites.values_mut().next().unwrap().distance = 3;
        let mut incoming = consumed(1);
        incoming.sites.values_mut().next().unwrap().distance = 1;
        let mut budget = Budget { used: 0 };
        assert!(join_fact(&mut old, &incoming, &mut budget, &span(10)).unwrap());
        assert_eq!(old.sites.values().next().unwrap().distance, 1);
    }

    #[test]
    fn shared_trie_nodes_keep_the_shortest_of_twenty_unequal_paths() {
        for gap in 1..=20 {
            for incoming_is_shorter in [false, true] {
                let root = Node::leaf(consumed(1));
                let (old_distance, new_distance) = if incoming_is_shorter {
                    (gap + 1, 1)
                } else {
                    (1, gap + 1)
                };
                let old_edge = span(100 + gap as u64);
                let new_edge = span(200 + gap as u64);
                let old = Lazy {
                    distance: old_distance,
                    edges: Some(Rc::new(EdgeList::One(old_edge.clone()))),
                    maybe: false,
                };
                let new = Lazy {
                    distance: new_distance,
                    edges: Some(Rc::new(EdgeList::One(new_edge.clone()))),
                    maybe: false,
                };
                let mut budget = Budget { used: 0 };
                let (joined, _) = join_node(
                    Some(&root),
                    Some(&root),
                    0,
                    &old,
                    &new,
                    &mut budget,
                    &span(9),
                )
                .unwrap();
                let joined = joined.unwrap();
                let mut fact = joined.fact.clone().unwrap();
                joined.lazy.apply(&mut fact);
                let site = fact.sites.values().next().unwrap();
                assert_eq!(
                    site.distance, 1,
                    "gap={gap}, incoming_is_shorter={incoming_is_shorter}"
                );
                assert_eq!(
                    site.backedges,
                    [if incoming_is_shorter {
                        new_edge
                    } else {
                        old_edge
                    }],
                    "gap={gap}, incoming_is_shorter={incoming_is_shorter}"
                );
            }
        }
    }

    #[test]
    fn shared_trie_nodes_preserve_conditional_definiteness() {
        for depth in 0..20 {
            for old_is_maybe in [false, true] {
                let root = Node::leaf(consumed(depth + 1));
                let old = Lazy {
                    distance: depth as u32 + u32::from(old_is_maybe),
                    maybe: old_is_maybe,
                    ..Lazy::default()
                };
                let new = Lazy {
                    distance: depth as u32,
                    maybe: !old_is_maybe,
                    ..Lazy::default()
                };
                let mut budget = Budget { used: 0 };
                let (joined, changed) = join_node(
                    Some(&root),
                    Some(&root),
                    0,
                    &old,
                    &new,
                    &mut budget,
                    &span(100 + depth),
                )
                .unwrap();
                assert!(changed, "case {depth}, old_is_maybe={old_is_maybe}");
                let joined = joined.unwrap();
                let mut fact = joined.fact.clone().unwrap();
                joined.lazy.apply(&mut fact);
                assert!(!fact.definite, "case {depth}");
                assert_eq!(fact.sites.values().next().unwrap().distance, depth as u32);
            }
        }
    }

    #[test]
    fn quota_discards_intermediate_loop_observations() {
        use super::super::super::model::Op;
        let mut source = String::from(
            "fn F(b:view Bool){var s=MockOpen(1);Inspect(s);Drop(s);while b{s=MockOpen(2);}",
        );
        for index in 0..100 {
            source.push_str(&format!(
                "var r{index}=MockOpen(1);Drop(r{index});while b{{r{index}=MockOpen(2);}}"
            ));
        }
        source.push_str("}fn Main(){}");
        let program = super::super::super::compile_scalar("quota_forged.n", &source).unwrap();
        let mut function = program.functions.function("F").unwrap().clone();
        let tokens = crate::frontend::lex::lex("quota_forged.n", &source).unwrap();
        let s = function
            .locals
            .iter()
            .find(|local| local.name == "s")
            .unwrap()
            .id;
        let inspect_at = source.find("Inspect(s)").unwrap() as u64;
        let inspected = function
            .block
            .instructions
            .iter()
            .find(|ins| {
                ins.span.start >= inspect_at
                    && ins.span.end <= inspect_at + "Inspect(s)".len() as u64
                    && use_local(&ins.op) == Some(s)
            })
            .expect("source inspection instruction")
            .clone();
        let loop_at = source.find("while b{s=").unwrap() as u64;
        let body = function
            .blocks
            .iter_mut()
            .find(|block| {
                block.instructions.iter().any(|ins| {
                    ins.span.start > loop_at
                        && matches!(ins.op, Op::WritePlace { local, .. } if local == s)
                })
            })
            .expect("loop body");
        body.instructions.insert(0, inspected);
        let blocks: Vec<_> = std::iter::once(&function.block)
            .chain(&function.blocks)
            .collect();
        let (first_backedge, header) = blocks
            .iter()
            .enumerate()
            .find_map(|(index, block)| {
                if let Terminator::Jump { target, .. } = &block.terminator {
                    (target.0 <= index).then_some((index, target.0))
                } else {
                    None
                }
            })
            .unwrap();
        assert_eq!(
            stable_prefix(&blocks, first_backedge, &vec![false; blocks.len()]),
            header
        );
        let names: Vec<_> = function
            .locals
            .iter()
            .map(|local| {
                tokens
                    .iter()
                    .find(|token| token.text == local.name && !local.name.is_empty())
                    .map_or_else(|| span(0), |token| token.span.clone())
            })
            .collect();
        let mut used = 0;
        let complete = analyze(&function, &names, &tokens, &mut used).unwrap();
        assert_eq!(complete.len(), 1, "{complete:?}");
        assert_eq!(complete[0].subcause.as_deref(), Some("maybe_consumed"));
        let mut kept_with_limit = false;
        for remaining in 1..=256 {
            let mut used = FLOW_LIMIT - remaining;
            let result = analyze(&function, &names, &tokens, &mut used);
            let Err(CompileError::Rejected(errors)) = result else {
                continue;
            };
            let mut preserved = Vec::new();
            let mut limits = 0;
            for error in errors {
                if error.code == "NX-E0003" {
                    limits += 1;
                } else {
                    preserved.push(*error);
                }
            }
            assert_eq!(limits, 1);
            assert!(
                preserved.iter().all(|d| complete.contains(d)),
                "remaining={remaining}: {preserved:?}"
            );
            assert!(preserved.len() <= 1);
            kept_with_limit |= preserved == complete;
        }
        assert!(
            kept_with_limit,
            "converged loop diagnostic was never retained on quota"
        );
        // The final charged instruction belongs to the diagnostic traversal:
        // the complete run has already converged and the last charge fails.
        let mut used_at_last_diagnostic_step = FLOW_LIMIT - (used - 1);
        let Err(CompileError::Rejected(errors)) = analyze(
            &function,
            &names,
            &tokens,
            &mut used_at_last_diagnostic_step,
        ) else {
            panic!("expected the last diagnostic charge to exhaust the quota");
        };
        let preserved: Vec<_> = errors
            .into_iter()
            .filter(|d| d.code != "NX-E0003")
            .map(|d| *d)
            .collect();
        assert_eq!(preserved, complete);
    }

    #[test]
    fn diagnostic_pass_quota_preserves_one_to_three_independent_loop_uses() {
        for nested in [false, true] {
            for uses in 1..=3 {
                let mut source = String::from("fn F(b:view Bool){");
                for index in 0..uses {
                    source.push_str(&format!(
                        "var s{index}=MockOpen(1);Inspect(s{index});Drop(s{index});"
                    ));
                    if nested {
                        source.push_str(&format!("while b{{while b{{s{index}=MockOpen(2);}}}}"));
                    } else {
                        source.push_str(&format!("while b{{s{index}=MockOpen(2);}}"));
                    }
                }
                source.push_str("}fn Main(){}");
                let file = "quota_forged.n";
                let program = super::super::super::compile_scalar(file, &source).unwrap();
                let mut function = program.functions.function("F").unwrap().clone();
                let tokens = crate::frontend::lex::lex(file, &source).unwrap();
                let names: Vec<_> = function
                    .locals
                    .iter()
                    .map(|local| {
                        tokens
                            .iter()
                            .find(|token| token.text == local.name && !local.name.is_empty())
                            .map_or_else(|| span(0), |token| token.span.clone())
                    })
                    .collect();
                for index in 0..uses {
                    let name = format!("s{index}");
                    let local = function
                        .locals
                        .iter()
                        .find(|local| local.name == name)
                        .unwrap()
                        .id;
                    let at = source.find(&format!("Inspect({name})")).unwrap() as u64;
                    let inspected = std::iter::once(&function.block)
                        .chain(&function.blocks)
                        .flat_map(|block| &block.instructions)
                        .find(|ins| {
                            ins.span.start >= at
                                && ins.span.end <= at + 8 + name.len() as u64
                                && use_local(&ins.op) == Some(local)
                        })
                        .unwrap()
                        .clone();
                    let loop_at = source
                        .find(&format!(
                            "while b{{{}s{index}=",
                            if nested { "while b{" } else { "" }
                        ))
                        .unwrap() as u64;
                    let body = function.blocks.iter_mut().find(|block| {
                        block.instructions.iter().any(|ins| ins.span.start > loop_at && matches!(ins.op,
                            super::super::super::model::Op::WritePlace { local: written, .. } if written == local))
                    }).unwrap();
                    body.instructions.insert(0, inspected);
                }
                let mut full_used = 0;
                let complete = analyze(&function, &names, &tokens, &mut full_used).unwrap();
                assert_eq!(complete.len(), uses, "nested={nested}: {complete:?}");
                assert!(complete.iter().all(
                    |d| d.code == "NX-E0202" && d.subcause.as_deref() == Some("maybe_consumed")
                ));
                for remaining in 1..=3 {
                    let mut quota_used = FLOW_LIMIT - (full_used - remaining);
                    let Err(CompileError::Rejected(errors)) =
                        analyze(&function, &names, &tokens, &mut quota_used)
                    else {
                        panic!("expected diagnostic-pass quota exhaustion");
                    };
                    let limits = errors.iter().filter(|d| d.code == "NX-E0003").count();
                    assert_eq!(limits, 1);
                    let preserved: Vec<_> = errors
                        .into_iter()
                        .filter(|d| d.code != "NX-E0003")
                        .map(|d| *d)
                        .collect();
                    assert!(
                        preserved.iter().all(|d| complete.contains(d)),
                        "nested={nested}, uses={uses}, remaining={remaining}: {preserved:?}"
                    );
                    assert_eq!(preserved.len(), uses);
                }
            }
        }
    }

    #[test]
    fn same_source_use_prefers_joined_subcause_and_matching_cause() {
        let source = "fn F(b: view Bool) {\n    var r = MockOpen(1);\n    Drop(r);\n    while b {\n        Inspect(r);\n        r = MockOpen(2);\n    }\n}\nfn Main() {}\n";
        let Err(CompileError::Rejected(errors)) =
            super::super::super::compile_scalar("same_use.n", source)
        else {
            panic!("expected ownership diagnostic");
        };
        let joined = errors
            .iter()
            .find(|d| d.code == "NX-E0202")
            .unwrap()
            .as_ref()
            .clone();
        assert_eq!(joined.subcause.as_deref(), Some("maybe_consumed"));
        let mut intermediate = joined.clone();
        intermediate.subcause = Some("definitely_consumed".into());
        intermediate.cause_chain.clear();
        let mut candidates = vec![intermediate, joined.clone()];
        canonicalize(&mut candidates);
        assert_eq!(candidates.len(), 1);
        assert_eq!(candidates[0], joined);
        let mut other_operation = joined.clone();
        other_operation.operation = diagnostic::Operation::AnalysisLimit;
        let mut distinct = vec![joined.clone(), other_operation.clone()];
        canonicalize(&mut distinct);
        assert_eq!(distinct, [joined, other_operation]);
    }

    #[test]
    fn newly_reaching_consumption_is_conditional_at_loop_header() {
        let mut into = SparseState::new(2);
        let mut from = SparseState::new(2);
        from.set(0, Some(consumed(1)));
        let touched = [LoopEffect {
            local: 0,
            read: true,
            generated: true,
            kill: false,
            declared_here: false,
        }];
        let mut budget = Budget { used: 0 };
        assert!(join_loop(&mut into, &from, &touched, &mut budget, &span(10)).unwrap());
        assert!(!into.get(0).unwrap().definite);
    }

    #[test]
    fn deferred_maybe_flag_weakens_a_definite_fact() {
        let mut fact = consumed(1);
        Lazy {
            maybe: true,
            ..Lazy::default()
        }
        .apply(&mut fact);
        assert!(!fact.definite);
    }

    #[test]
    fn root_deferred_edge_is_read_before_descending() {
        let mut state = SparseState::new(2);
        state.set(0, Some(consumed(1)));
        state.edge(Some(span(10)));
        let fact = state.get(0).unwrap();
        assert_eq!(fact.sites.values().next().unwrap().backedges, [span(10)]);
    }

    #[test]
    fn joining_an_empty_path_weakens_a_consumed_fact() {
        let mut into = SparseState::new(2);
        into.set(0, Some(consumed(1)));
        let from = SparseState::new(2);
        let mut budget = Budget { used: 0 };
        assert!(join_sparse(&mut into, &from, &mut budget, &span(10)).unwrap());
        assert!(!into.get(0).unwrap().definite);
    }

    #[test]
    fn deferred_backedges_keep_traversal_order_after_other_binding_changes() {
        let site = Site {
            span: span(1),
            is_drop: true,
            backedges: Vec::new(),
            distance: 0,
        };
        let mut state = SparseState::new(32_769);
        state.set(
            0,
            Some(Fact {
                definite: true,
                sites: BTreeMap::from([((1, 2, true), site)]),
            }),
        );
        state.edge(Some(span(10)));
        // The other branch of the trie changes, pushing the first deferred
        // backedge below the root while retaining this fact by reference.
        state.set(32_768, None);
        state.edge(Some(span(20)));
        let fact = state.get(0).unwrap();
        let witness = fact.sites.values().next().unwrap();
        assert_eq!(witness.backedges, [span(10), span(20)]);
        assert_eq!(witness.distance, 2);
    }

    #[test]
    fn backedge_discards_the_previous_iteration_local_generation() {
        let site = Site {
            span: span(1),
            is_drop: true,
            backedges: Vec::new(),
            distance: 0,
        };
        let mut state = SparseState::new(2);
        state.set(
            0,
            Some(Fact {
                definite: true,
                sites: BTreeMap::from([((1, 2, true), site)]),
            }),
        );
        let edge = Backedge {
            closing: span(10),
            touched: vec![LoopEffect {
                local: 0,
                read: true,
                generated: true,
                kill: false,
                declared_here: true,
            }],
        };
        edge.cross(&mut state);
        assert!(state.get(0).is_none());
    }

    #[test]
    fn loop_summary_indexes_local_consumption_and_scope_kill() {
        let source = "fn F(b: view Bool) {\n    while b {\n        let r = MockOpen(1);\n        Drop(r);\n    }\n}\nfn Main() {}\n";
        let program = super::super::super::compile_scalar("loop_summary.n", source).unwrap();
        let function = program.functions.function("F").unwrap();
        let names: Vec<_> = function
            .locals
            .iter()
            .map(|local| {
                let start = match local.name.as_str() {
                    "b" => source.find("b: view Bool").unwrap(),
                    "r" => source.find("let r =").unwrap() + 4,
                    _ => 0,
                } as u64;
                Span {
                    file: "loop_summary.n".into(),
                    start,
                    end: start + local.name.len() as u64,
                }
            })
            .collect();
        let blocks: Vec<_> = std::iter::once(&function.block)
            .chain(&function.blocks)
            .collect();
        let (source_block, target) = blocks
            .iter()
            .enumerate()
            .find_map(|(index, block)| match &block.terminator {
                Terminator::Jump { target, .. } if target.0 <= index => Some((index, target.0)),
                _ => None,
            })
            .unwrap();
        let mut budget = Budget { used: 0 };
        let summary =
            summarize_backedge(&blocks, source_block, target, &names, &mut budget).unwrap();
        let resource = summary
            .touched
            .iter()
            .find(|effect| function.locals[effect.local].name == "r")
            .unwrap();
        assert!(resource.read && resource.generated && resource.kill && resource.declared_here);
        let condition = summary
            .touched
            .iter()
            .find(|effect| function.locals[effect.local].name == "b")
            .unwrap();
        assert!(
            condition.read && !condition.generated && !condition.kill && !condition.declared_here
        );
    }
}
