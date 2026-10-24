//! Source ownership facts carried around the CFG's while backedges.
//!
//! Absence from a state means available (or not yet declared). A present fact
//! means consumed on at least one path; `definite` means on every path. A
//! complete write kills the fact. Loan permissions are checked during lowering
//! and by the IR verifier; call loans end before a CFG terminator.
use std::collections::BTreeMap;
#[cfg(test)]
use std::collections::VecDeque;

use super::lower::CompileError;
use super::model::{Function, LocalId, Op, Terminator};
use crate::diagnostic::{
    self, Cause, CauseKind, CauseOperation, Diagnostic, RelatedRole, RelatedSpan, Span,
};
use crate::frontend::lex::Token;

const FLOW_LIMIT: u64 = 1_000_000;

#[cfg(test)]
pub(super) static FLOW_WORK: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
#[cfg(test)]
pub(super) static SPARSE_FLOW_WORK: std::sync::atomic::AtomicU64 =
    std::sync::atomic::AtomicU64::new(0);
#[cfg(test)]
pub(super) static PREFIX_FLOW_WORK: std::sync::atomic::AtomicU64 =
    std::sync::atomic::AtomicU64::new(0);

#[cfg(test)]
thread_local! {
    pub(super) static FLOW_STATE_COUNT_ENABLED: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}
#[cfg(test)]
pub(super) static FLOW_STATE_ALLOCS: std::sync::atomic::AtomicU64 =
    std::sync::atomic::AtomicU64::new(0);
#[cfg(test)]
pub(super) static FLOW_STATE_BYTES: std::sync::atomic::AtomicU64 =
    std::sync::atomic::AtomicU64::new(0);

#[derive(Clone, Debug, PartialEq, Eq)]
struct Site {
    span: Span,
    is_drop: bool,
    /// In traversal order, with each syntactic edge at most once.
    backedges: Vec<Span>,
    distance: u32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct Fact {
    definite: bool,
    sites: BTreeMap<(u64, u64, bool), Site>,
}

#[cfg(test)]
type State = BTreeMap<usize, Fact>;

struct Budget {
    used: u64,
}

impl Budget {
    fn charge(&mut self, span: &Span) -> Result<(), CompileError> {
        #[cfg(test)]
        let limit = if span.file == "quota_preserve.nx" {
            64
        } else {
            FLOW_LIMIT
        };
        #[cfg(not(test))]
        let limit = FLOW_LIMIT;
        super::count_cfg_work(1);
        #[cfg(test)]
        if matches!(
            span.file.as_str(),
            "flow_short_scale.nx" | "cfg_scale_counter.nx"
        ) {
            FLOW_WORK.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        }
        #[cfg(test)]
        if span.file == "sparse_scale.nx" {
            SPARSE_FLOW_WORK.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        }
        #[cfg(test)]
        if span.file == "prefix_scale.nx" {
            PREFIX_FLOW_WORK.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        }
        self.used += 1;
        if self.used <= limit {
            return Ok(());
        }
        let mut d = crate::check::names::diagnostic(
            "NX-E0003",
            None,
            diagnostic::Stage::Ownership,
            diagnostic::Operation::AnalysisLimit,
            span.clone(),
        );
        d.data.insert(
            "budget_kind".into(),
            diagnostic::DataValue::String("ownership_flow_steps".into()),
        );
        d.data
            .insert("limit".into(), diagnostic::DataValue::UInt(limit));
        d.data
            .insert("observed".into(), diagnostic::DataValue::UInt(self.used));
        Err(CompileError::Rejected(vec![d]))
    }
}

#[cfg(test)]
fn transfer(state: &mut State, ins: &super::model::Instruction, function: &Function) {
    match ins.op {
        Op::WritePlace { local, .. } | Op::DropLocal(local) => {
            state.remove(&local.0);
        }
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
            state.insert(
                local.0,
                Fact {
                    definite: true,
                    sites: BTreeMap::from([((site.span.start, site.span.end, site.is_drop), site)]),
                },
            );
        }
        _ => {}
    }
}

#[cfg(test)]
fn crossed_backedge(state: &mut State, body: &Span, loop_span: &Span, name_spans: &[Span]) {
    state.retain(|&local, _| {
        let declared = &name_spans[local];
        declared.start < loop_span.start || declared.end > loop_span.end
    });
    let closing = closing_span(body);
    for fact in state.values_mut() {
        for site in fact.sites.values_mut() {
            site.distance = site.distance.saturating_add(1);
            if !site.backedges.contains(&closing) {
                site.backedges.push(closing.clone());
            }
        }
    }
}

#[cfg(test)]
fn join(
    into: &mut State,
    from: &State,
    budget: &mut Budget,
    span: &Span,
) -> Result<bool, CompileError> {
    let mut changed = false;
    for (&local, old) in into.iter_mut() {
        budget.charge(span)?;
        if let Some(incoming) = from.get(&local) {
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
        } else if old.definite {
            old.definite = false;
            changed = true;
        }
    }
    for (&local, fact) in from {
        if let std::collections::btree_map::Entry::Vacant(entry) = into.entry(local) {
            budget.charge(span)?;
            let mut fact = fact.clone();
            fact.definite = false;
            entry.insert(fact);
            changed = true;
        }
    }
    Ok(changed)
}

fn closing_span(span: &Span) -> Span {
    Span {
        file: span.file.clone(),
        start: span.end.saturating_sub(1),
        end: span.end,
    }
}

fn use_local(op: &Op) -> Option<LocalId> {
    match op {
        Op::CopyValue(id)
        | Op::BorrowValue(id)
        | Op::TransferValue(id)
        | Op::DropBinding(id)
        | Op::CheckIndex { local: id, .. }
        | Op::WriteIndex { local: id, .. } => Some(*id),
        Op::BeginLoan { root, .. } => Some(*root),
        Op::ReadIndex { local, .. } | Op::BorrowIndex { local, .. } => Some(*local),
        _ => None,
    }
}

fn primary_span(
    function: &Function,
    tokens: &[Token],
    local: LocalId,
    ins: &super::model::Instruction,
) -> Span {
    if matches!(ins.op, Op::DropBinding(_) | Op::WriteIndex { .. }) {
        let at = tokens.partition_point(|t| t.span.start < ins.span.start);
        if let Some(token) = tokens[at..]
            .iter()
            .take_while(|t| t.span.end <= ins.span.end)
            .find(|t| t.text == function.locals[local.0].name)
        {
            return token.span.clone();
        }
    }
    ins.span.clone()
}

fn diagnostic(
    function: &Function,
    name_spans: &[Span],
    tokens: &[Token],
    local: LocalId,
    ins: &super::model::Instruction,
    fact: &Fact,
) -> Diagnostic {
    let mut d = *crate::check::names::diagnostic(
        "NX-E0202",
        Some(if fact.definite {
            "definitely_consumed"
        } else {
            "maybe_consumed"
        }),
        diagnostic::Stage::Ownership,
        diagnostic::Operation::OwnershipUse,
        primary_span(function, tokens, local, ins),
    );
    let declaration = &name_spans[local.0];
    d.related_spans.push(RelatedSpan {
        role: RelatedRole::Declaration,
        span: declaration.clone(),
    });
    for site in fact.sites.values() {
        d.related_spans.push(RelatedSpan {
            role: RelatedRole::Consumption,
            span: site.span.clone(),
        });
        for backedge in &site.backedges {
            d.related_spans.push(RelatedSpan {
                role: RelatedRole::Backedge,
                span: backedge.clone(),
            });
        }
    }
    crate::check::names::sort_related(&mut d);
    d.related_spans
        .dedup_by(|a, b| a.role == b.role && a.span == b.span);
    let witness = fact
        .sites
        .values()
        .min_by_key(|s| (s.distance, s.span.start, s.span.end))
        .expect("nonempty consumption fact");
    let entity = diagnostic::Entity {
        kind: if function.locals[local.0].parameter_mode.is_some() {
            diagnostic::EntityKind::Parameter
        } else {
            diagnostic::EntityKind::Binding
        },
        id: format!(
            "{}:{}",
            if function.locals[local.0].parameter_mode.is_some() {
                "parameter"
            } else {
                "binding"
            },
            declaration.start
        ),
    };
    for edge in witness.backedges.iter().rev() {
        d.cause_chain.push(Cause {
            kind: CauseKind::Backedge,
            operation: CauseOperation::FlowBackedge,
            span: Some(edge.clone()),
            entity: Some(entity.clone()),
            evidence: diagnostic::Evidence::Verified,
        });
    }
    d.cause_chain.push(Cause {
        kind: if witness.is_drop {
            CauseKind::Drop
        } else {
            CauseKind::Consume
        },
        operation: if witness.is_drop {
            CauseOperation::ValueDrop
        } else {
            CauseOperation::ValueConsume
        },
        span: Some(witness.span.clone()),
        entity: Some(entity),
        evidence: diagnostic::Evidence::Verified,
    });
    d
}

#[cfg(test)]
fn analyze_dense(
    function: &Function,
    name_spans: &[Span],
    tokens: &[Token],
    used: &mut u64,
) -> Result<Vec<Diagnostic>, CompileError> {
    let blocks: Vec<_> = std::iter::once(&function.block)
        .chain(&function.blocks)
        .collect();
    let mut incoming = vec![None::<State>; blocks.len()];
    incoming[0] = Some(State::new());
    let mut queue = VecDeque::from([0]);
    let mut queued = vec![false; blocks.len()];
    let mut budget = Budget { used: *used };
    while let Some(index) = queue.pop_front() {
        queued[index] = false;
        let block = blocks[index];
        let mut state = incoming[index].clone().expect("queued block has input");
        for ins in &block.instructions {
            budget.charge(&ins.span)?;
            transfer(&mut state, ins, function);
        }
        let targets: &[super::model::BlockId] = match &block.terminator {
            Terminator::Return { .. } => &[],
            Terminator::Jump { target, .. } => std::slice::from_ref(target),
            Terminator::BranchBool { yes, no, .. } => {
                // The two successors are handled below without allocating.
                for target in [yes, no] {
                    propagate(
                        index,
                        target.0,
                        &state,
                        &blocks,
                        &mut incoming,
                        &mut queue,
                        &mut queued,
                        &mut budget,
                        name_spans,
                    )?;
                }
                continue;
            }
        };
        for target in targets {
            propagate(
                index,
                target.0,
                &state,
                &blocks,
                &mut incoming,
                &mut queue,
                &mut queued,
                &mut budget,
                name_spans,
            )?;
        }
    }
    let mut errors = Vec::new();
    for (index, block) in blocks.iter().enumerate() {
        let Some(mut state) = incoming[index].clone() else {
            continue;
        };
        for ins in &block.instructions {
            budget.charge(&ins.span)?;
            if let Some(local) = use_local(&ins.op)
                && !function.locals[local.0].name.is_empty()
                && let Some(fact) = state.get(&local.0)
            {
                errors.push(diagnostic(function, name_spans, tokens, local, ins, fact));
            }
            transfer(&mut state, ins, function);
        }
    }
    errors.sort_by_key(|d| d.primary_span.as_ref().map(|s| (s.start, s.end)));
    errors.dedup_by(|a, b| {
        a.primary_span == b.primary_span && a.code == b.code && a.subcause == b.subcause
    });
    *used = budget.used;
    Ok(errors)
}

mod sparse;
pub(super) use sparse::analyze;

#[cfg(test)]
#[allow(clippy::too_many_arguments)]
fn propagate(
    source: usize,
    target: usize,
    state: &State,
    blocks: &[&super::model::BasicBlock],
    incoming: &mut [Option<State>],
    queue: &mut VecDeque<usize>,
    queued: &mut [bool],
    budget: &mut Budget,
    name_spans: &[Span],
) -> Result<(), CompileError> {
    // Test-only dense oracle: generated_sparse_ownership_matches_dense_reference_
    // in_regular_tests compares this with the production sparse pass. The
    // source compiler allocates every target and the while preheader together.
    if target >= blocks.len() {
        return Err(CompileError::InvalidIr("invalid jump target"));
    }
    let mut edge_state = state.clone();
    if target <= source {
        let Terminator::Jump { span, .. } = &blocks[source].terminator else {
            return Err(CompileError::InvalidIr("invalid control-flow cycle"));
        };
        let loop_span = target
            .checked_sub(1)
            .and_then(|preheader| {
                if let Terminator::Jump { target: dest, span } = &blocks[preheader].terminator
                    && dest.0 == target
                {
                    Some(span)
                } else {
                    None
                }
            })
            .ok_or(CompileError::InvalidIr("invalid control-flow cycle"))?;
        crossed_backedge(&mut edge_state, span, loop_span, name_spans);
    } else {
        for fact in edge_state.values_mut() {
            for site in fact.sites.values_mut() {
                site.distance = site.distance.saturating_add(1);
            }
        }
    }
    let changed = if let Some(prior) = &mut incoming[target] {
        let span = match &blocks[target].terminator {
            Terminator::Return { span, .. }
            | Terminator::Jump { span, .. }
            | Terminator::BranchBool { span, .. } => span,
        };
        join(prior, &edge_state, budget, span)?
    } else {
        incoming[target] = Some(edge_state);
        true
    };
    if changed && !queued[target] {
        queue.push_back(target);
        queued[target] = true;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn span(start: u64, end: u64) -> Span {
        Span {
            file: "flow_state.nx".into(),
            start,
            end,
        }
    }

    fn site(at: u64) -> Site {
        Site {
            span: span(at, at + 1),
            is_drop: true,
            backedges: Vec::new(),
            distance: 0,
        }
    }

    fn fact(at: u64, definite: bool) -> Fact {
        let site = site(at);
        Fact {
            definite,
            sites: BTreeMap::from([((at, at + 1, true), site)]),
        }
    }

    #[test]
    fn full_write_kills_only_its_root_and_transfer_keeps_source_kind() {
        use super::super::model::{LocalId, Op};
        let source = "fn Main() {\n    var r = MockOpen(1);\n    var s = MockOpen(2);\n    Drop(r);\n    Drop(s);\n}\n";
        let program = super::super::compile_scalar("flow_state.nx", source).unwrap();
        let function = program.functions.function("Main").unwrap();
        let ins = function
            .block
            .instructions
            .iter()
            .find(|ins| matches!(ins.op, Op::DropBinding(_)))
            .unwrap();
        let mut state = State::new();
        let mut drop_r = ins.clone();
        drop_r.op = Op::DropBinding(LocalId(0));
        transfer(&mut state, &drop_r, function);
        let dropped = state.get(&0).unwrap();
        assert!(dropped.definite);
        assert!(dropped.sites.values().next().unwrap().is_drop);
        assert_eq!(dropped.sites.values().next().unwrap().distance, 0);
        let mut consumed_s = ins.clone();
        consumed_s.op = Op::TransferValue(LocalId(1));
        transfer(&mut state, &consumed_s, function);
        assert!(
            !state
                .get(&1)
                .unwrap()
                .sites
                .values()
                .next()
                .unwrap()
                .is_drop
        );
        let mut write_r = ins.clone();
        write_r.op = Op::WritePlace {
            local: LocalId(0),
            value: super::super::model::TempId(0),
        };
        transfer(&mut state, &write_r, function);
        assert!(!state.contains_key(&0));
        assert!(state.contains_key(&1));
        let mut leave_s = ins.clone();
        leave_s.op = Op::DropLocal(LocalId(1));
        transfer(&mut state, &leave_s, function);
        assert!(state.is_empty());
    }

    #[test]
    fn synthetic_union_slot_does_not_become_a_source_owner() {
        let source = "fn Main() {\n    let v = if true then 1 else 2;\n}\n";
        let program = super::super::compile_scalar("flow_state.nx", source).unwrap();
        let function = program.functions.function("Main").unwrap();
        let slot = function
            .locals
            .iter()
            .find(|local| local.name.is_empty())
            .unwrap()
            .id;
        let mut ins = function.block.instructions[0].clone();
        ins.op = Op::TransferValue(slot);
        let mut state = State::new();
        transfer(&mut state, &ins, function);
        assert!(state.is_empty());
    }

    #[test]
    fn join_is_a_may_union_and_preserves_both_consumption_origins() {
        let mut left = State::from([(0, fact(2, true))]);
        let right = State::from([(0, fact(4, false))]);
        let mut budget = Budget { used: 0 };
        assert!(join(&mut left, &right, &mut budget, &span(1, 2)).unwrap());
        assert!(!left[&0].definite);
        assert_eq!(left[&0].sites.len(), 2);
        assert!(left[&0].sites.contains_key(&(2, 3, true)));
        assert!(left[&0].sites.contains_key(&(4, 5, true)));
        assert!(!join(&mut left, &right, &mut budget, &span(1, 2)).unwrap());

        let mut only_site_change = State::from([(0, fact(2, false))]);
        assert!(
            join(
                &mut only_site_change,
                &State::from([(0, fact(4, false))]),
                &mut budget,
                &span(1, 2)
            )
            .unwrap()
        );
        assert_eq!(only_site_change[&0].sites.len(), 2);

        let mut only_definiteness_changes = State::from([(0, fact(2, true))]);
        assert!(
            join(
                &mut only_definiteness_changes,
                &State::from([(0, fact(2, false))]),
                &mut budget,
                &span(1, 2)
            )
            .unwrap()
        );
        assert!(!only_definiteness_changes[&0].definite);

        let mut one_path = State::from([(0, fact(2, true))]);
        assert!(join(&mut one_path, &State::new(), &mut budget, &span(1, 2)).unwrap());
        assert!(!one_path[&0].definite);
        let mut other_path = State::new();
        assert!(
            join(
                &mut other_path,
                &State::from([(0, fact(2, true))]),
                &mut budget,
                &span(1, 2)
            )
            .unwrap()
        );
        assert!(!other_path[&0].definite);
    }

    #[test]
    fn repeated_origin_keeps_the_shortest_reaching_path() {
        let mut long = fact(7, true);
        long.sites.get_mut(&(7, 8, true)).unwrap().distance = 9;
        let mut short = fact(7, true);
        short.sites.get_mut(&(7, 8, true)).unwrap().distance = 3;
        let mut into = State::from([(0, long)]);
        let from = State::from([(0, short)]);
        let mut budget = Budget { used: 0 };
        assert!(join(&mut into, &from, &mut budget, &span(1, 2)).unwrap());
        assert_eq!(into[&0].sites[&(7, 8, true)].distance, 3);
    }

    #[test]
    fn backedge_restarts_loop_local_but_retains_external_witness() {
        let mut state = State::from([(0, fact(2, true)), (1, fact(22, true))]);
        let names = [span(2, 3), span(22, 23)];
        crossed_backedge(&mut state, &span(20, 39), &span(10, 40), &names);
        assert_eq!(state.len(), 1);
        assert!(!state.contains_key(&1));
        let witness = state[&0].sites.values().next().unwrap();
        assert_eq!(witness.distance, 1);
        assert_eq!(witness.backedges, [span(38, 39)]);
    }

    #[test]
    fn changed_successors_enter_the_fifo_in_construction_order() {
        use super::super::model::{BasicBlock, BlockId, TempId, Terminator};
        let blocks = [
            BasicBlock {
                id: BlockId(0),
                instructions: vec![],
                terminator: Terminator::Return {
                    value: Some(TempId(0)),
                    span: span(0, 1),
                },
            },
            BasicBlock {
                id: BlockId(1),
                instructions: vec![],
                terminator: Terminator::Return {
                    value: Some(TempId(0)),
                    span: span(1, 2),
                },
            },
            BasicBlock {
                id: BlockId(2),
                instructions: vec![],
                terminator: Terminator::Return {
                    value: Some(TempId(0)),
                    span: span(2, 3),
                },
            },
        ];
        let refs: Vec<_> = blocks.iter().collect();
        let mut incoming = vec![None; 3];
        let mut queue = VecDeque::new();
        let mut queued = vec![false; 3];
        let mut budget = Budget { used: 0 };
        for target in [1, 2] {
            propagate(
                0,
                target,
                &State::new(),
                &refs,
                &mut incoming,
                &mut queue,
                &mut queued,
                &mut budget,
                &[],
            )
            .unwrap();
        }
        assert_eq!(queue.into_iter().collect::<Vec<_>>(), [1, 2]);
    }

    #[test]
    fn ownership_flow_budget_rejects_exactly_first_excess_step() {
        let source = "fn Main() {\n    while false {}\n}\n";
        let span = Span {
            file: "budget_flow.nx".into(),
            start: 10,
            end: 23,
        };
        let mut budget = Budget {
            used: FLOW_LIMIT.saturating_sub(1),
        };
        assert!(budget.charge(&span).is_ok());
        let Err(CompileError::Rejected(errors)) = budget.charge(&span) else {
            panic!("expected ownership budget diagnostic");
        };
        let d = &errors[0];
        assert_eq!(d.code, "NX-E0003");
        assert_eq!(d.stage, diagnostic::Stage::Ownership);
        assert_eq!(d.operation, diagnostic::Operation::AnalysisLimit);
        assert_eq!(
            d.data.get("budget_kind"),
            Some(&diagnostic::DataValue::String(
                "ownership_flow_steps".into()
            ))
        );
        assert_eq!(
            d.data.get("limit"),
            Some(&diagnostic::DataValue::UInt(1_000_000))
        );
        assert_eq!(
            d.data.get("observed"),
            Some(&diagnostic::DataValue::UInt(1_000_001))
        );
        d.validate("budget_flow.nx", source).unwrap();
    }

    #[test]
    fn ownership_flow_counts_summary_instruction_once() {
        let source = "fn Main() {\n    while false {}\n}\n";
        let program = super::super::compile_scalar("summary_charge.nx", source).unwrap();
        let function = program.functions.function("Main").unwrap();
        let tokens = crate::frontend::lex::lex("summary_charge.nx", source).unwrap();
        let mut used = 0;
        analyze(function, &[], &tokens, &mut used).unwrap();
        assert_eq!(used, 5);
    }
}
