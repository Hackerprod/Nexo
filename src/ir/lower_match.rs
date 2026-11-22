use super::*;
use crate::frontend::parse::Pattern;
use crate::ir::model::MatchLiteral;
use crate::union::{UnionKind, VariantTag};

#[derive(Clone, Copy)]
struct MatchSource {
    local: Option<LocalId>,
    loan: Option<TempId>,
    provisional: bool,
}

pub(super) struct MatchInnerState<'a> {
    selection: &'a parse::Match,
    expression: bool,
    depth: usize,
    expected: Option<Ty>,
    outer_temps: HashSet<TempId>,
    selected_ty: Ty,
    loan_root: Option<LocalId>,
    loan_temp: Option<TempId>,
    root: LocalId,
    result_ty: Ty,
    prior_locals: usize,
    checkpoint: usize,
    temp_checkpoint: usize,
    branch_mode: EmissionMode,
    previous_branches: Vec<BlockId>,
    ends: Vec<BlockId>,
    states: Vec<(HashMap<LocalId, Availability>, bool)>,
    slot: Option<LocalId>,
    arm_index: usize,
}

pub(super) enum MatchStart<'a> {
    Completed(Flow),
    Active(Box<MatchInnerState<'a>>),
}
pub(super) struct MatchSelectionPrelude {
    pub(super) outer_temps: HashSet<TempId>,
    pub(super) selected_ty: Ty,
}

pub(super) struct MatchArmState<'a> {
    arm: &'a parse::Arm,
    saved: ScopeId,
    surviving: HashSet<TempId>,
}
pub(super) enum MatchArmStart<'a> {
    Done(Option<MatchArmState<'a>>),
    Guard {
        arm: &'a parse::Arm,
        guard: &'a Expr,
        saved: ScopeId,
    },
}
pub(super) struct MatchSnapshot {
    locals: usize,
    next_scope: usize,
    visible: HashMap<String, Vec<LocalId>>,
    scopes: Vec<Vec<String>>,
    scope: ScopeId,
    active_loans: HashMap<LocalId, Vec<ActiveLoan>>,
    match_boundaries: Vec<MatchBoundary>,
    consumed_at: HashMap<LocalId, Vec<(Span, bool)>>,
    maybe_consumed_at: HashMap<LocalId, Vec<(Span, bool)>>,
    consumed_undo: usize,
    poisoned: HashSet<LocalId>,
    retained_locals: HashMap<LocalId, Vec<RetainedLoan>>,
    retained_temps: HashMap<TempId, Vec<RetainedLoan>>,
}
impl<'a> MatchArmState<'a> {
    pub(super) fn body(&self) -> &'a parse::Block {
        &self.arm.body
    }
}
impl<'a> MatchInnerState<'a> {
    pub(super) fn depth(&self) -> usize {
        self.depth
    }
    pub(super) fn is_expression(&self) -> bool {
        self.expression
    }
    pub(super) fn expected(&self) -> Option<Ty> {
        self.expected.clone()
    }
    pub(super) fn selection(&self) -> &'a parse::Match {
        self.selection
    }
}

fn guard_violations(expr: &parse::Expr) -> Vec<(&'static str, Span)> {
    enum Frame<'a> {
        Expr(&'a parse::Expr),
        Statement(&'a parse::Statement),
        Block(&'a parse::Block),
        Selection(&'a parse::Match),
    }
    let mut found = Vec::new();
    let mut frames = vec![Frame::Expr(expr)];
    while let Some(frame) = frames.pop() {
        match frame {
            Frame::Block(block) => {
                for statement in block.statements.iter().rev() {
                    frames.push(Frame::Statement(statement));
                }
            }
            Frame::Statement(statement) => match statement {
                parse::Statement::Let { value, .. } | parse::Statement::Expr { value, .. } => {
                    frames.push(Frame::Expr(value));
                }
                parse::Statement::Assign { place, value, .. } => {
                    found.push(("guard_mutation", place.span().clone()));
                    frames.push(Frame::Expr(value));
                    frames.push(Frame::Expr(place));
                }
                parse::Statement::Return { value, span } => {
                    found.push((
                        "guard_control",
                        Span {
                            file: span.file.clone(),
                            start: span.start,
                            end: span.start + 6,
                        },
                    ));
                    if let Some(value) = value {
                        frames.push(Frame::Expr(value));
                    }
                }
                parse::Statement::GuardControl {
                    token_span, value, ..
                } => {
                    found.push(("guard_control", token_span.clone()));
                    if let Some(value) = value {
                        frames.push(Frame::Expr(value));
                    }
                }
                parse::Statement::Value { value, .. } => {
                    if let Some(value) = value {
                        frames.push(Frame::Expr(value));
                    }
                }
                parse::Statement::If {
                    condition,
                    then_block,
                    otherwise,
                    ..
                } => {
                    if let Some(other) = otherwise {
                        frames.push(Frame::Statement(other));
                    }
                    frames.push(Frame::Block(then_block));
                    frames.push(Frame::Expr(condition));
                }
                parse::Statement::While {
                    condition, body, ..
                } => {
                    frames.push(Frame::Block(body));
                    frames.push(Frame::Expr(condition));
                }
                parse::Statement::For { iterable, body, .. } => {
                    frames.push(Frame::Block(body));
                    frames.push(Frame::Expr(iterable));
                }
                parse::Statement::Match(selection) => frames.push(Frame::Selection(selection)),
                parse::Statement::Block(body) => frames.push(Frame::Block(body)),
            },
            Frame::Selection(selection) => {
                if let Some(span) = &selection.mode_span {
                    found.push((
                        if selection.mode == Some(Mode::Consume) {
                            "guard_transfer"
                        } else {
                            "guard_mutation"
                        },
                        span.clone(),
                    ));
                }
                for arm in selection.arms.iter().rev() {
                    frames.push(Frame::Block(&arm.body));
                    if let Some(guard) = &arm.guard {
                        frames.push(Frame::Expr(guard));
                    }
                }
                frames.push(Frame::Expr(&selection.scrutinee));
            }
            Frame::Expr(expr) => match expr {
                parse::Expr::Unary { mode, value, span } => {
                    let cause = match mode {
                        Mode::Consume => Some("guard_transfer"),
                        Mode::Inout => Some("guard_mutation"),
                        Mode::Read
                            if span.start == value.span().start && span.end > value.span().end =>
                        {
                            Some("guard_control")
                        }
                        _ => None,
                    };
                    if let Some(cause) = cause {
                        let token = if cause == "guard_control" {
                            Span {
                                file: span.file.clone(),
                                start: span.end - 1,
                                end: span.end,
                            }
                        } else {
                            Span {
                                file: span.file.clone(),
                                start: span.start,
                                end: span.start + 4,
                            }
                        };
                        found.push((cause, token));
                    }
                    frames.push(Frame::Expr(value));
                }
                parse::Expr::Call { target, args, .. } => {
                    if matches!(target.as_ref(), parse::Expr::Name(name, _) if name == "Drop") {
                        found.push(("guard_transfer", target.span().clone()));
                    }
                    for arg in args.iter().rev() {
                        frames.push(Frame::Expr(&arg.value));
                    }
                    frames.push(Frame::Expr(target));
                }
                parse::Expr::Array(items, _) => {
                    for item in items.iter().rev() {
                        frames.push(Frame::Expr(item));
                    }
                }
                parse::Expr::Group(inner, _)
                | parse::Expr::Neg { value: inner, .. }
                | parse::Expr::Not { value: inner, .. } => frames.push(Frame::Expr(inner)),
                parse::Expr::Binary { left, right, .. } => {
                    frames.push(Frame::Expr(right));
                    frames.push(Frame::Expr(left));
                }
                parse::Expr::Index { target, index, .. } => {
                    frames.push(Frame::Expr(index));
                    frames.push(Frame::Expr(target));
                }
                parse::Expr::If {
                    condition,
                    then_block,
                    otherwise,
                    ..
                } => {
                    if let Some(other) = otherwise {
                        match other.as_ref() {
                            parse::Else::Block(body) => frames.push(Frame::Block(body)),
                            parse::Else::If(inner) => frames.push(Frame::Expr(inner)),
                        }
                    }
                    frames.push(Frame::Block(then_block));
                    frames.push(Frame::Expr(condition));
                }
                parse::Expr::Match(selection) => frames.push(Frame::Selection(selection)),
                parse::Expr::Name(..)
                | parse::Expr::Integer(..)
                | parse::Expr::Bool(..)
                | parse::Expr::Unit(..)
                | parse::Expr::Closure(..) => {}
            },
        }
    }
    found.sort_by_key(|(cause, span)| {
        (
            span.start,
            match *cause {
                "guard_control" => 0,
                "guard_transfer" => 1,
                _ => 2,
            },
        )
    });
    found.dedup_by(|(_, left), (_, right)| left.start == right.start);
    found
}

impl Lower<'_> {
    pub(super) fn capture_match_snapshot(&self) -> MatchSnapshot {
        MatchSnapshot {
            locals: self.locals.len(),
            next_scope: self.next_scope,
            visible: self.visible.clone(),
            scopes: self.scopes.clone(),
            scope: self.scope,
            active_loans: self.active_loans.clone(),
            match_boundaries: self.match_boundaries.clone(),
            consumed_at: self.consumed_at.clone(),
            maybe_consumed_at: self.maybe_consumed_at.clone(),
            consumed_undo: self.consumed_undo.len(),
            poisoned: self.poisoned.clone(),
            retained_locals: self.retained_locals.clone(),
            retained_temps: self.retained_temps.clone(),
        }
    }
    pub(super) fn restore_match_snapshot(&mut self, snapshot: MatchSnapshot) {
        self.locals.truncate(snapshot.locals);
        self.name_spans.truncate(snapshot.locals);
        self.next_scope = snapshot.next_scope;
        self.scope_parents.truncate(snapshot.next_scope);
        self.visible = snapshot.visible;
        self.scopes = snapshot.scopes;
        self.scope = snapshot.scope;
        self.active_loans = snapshot.active_loans;
        self.match_boundaries = snapshot.match_boundaries;
        self.consumed_at = snapshot.consumed_at;
        self.maybe_consumed_at = snapshot.maybe_consumed_at;
        self.consumed_undo.truncate(snapshot.consumed_undo);
        self.poisoned = snapshot.poisoned;
        self.retained_locals = snapshot.retained_locals;
        self.retained_temps = snapshot.retained_temps;
    }

    pub(super) fn begin_match_selection<'a>(
        &mut self,
        selection: &'a parse::Match,
        expression: bool,
        depth: usize,
        expected: Option<&Ty>,
    ) -> Result<MatchStart<'a>, CompileError> {
        let prelude = self.match_selection_prelude(selection)?;
        let selected_ty = prelude.selected_ty.clone();
        let selected = match selection.mode {
            Some(Mode::Consume) => {
                let name = place_name(&selection.scrutinee)
                    .ok_or(CompileError::InvalidIr("match move place"))?;
                let id = self.get(name)?;
                let primary = parse::expression_primary_span(&selection.scrutinee);
                self.check_available(id, primary.clone())?;
                if self.locals[id.0]
                    .parameter_mode
                    .is_some_and(|mode| mode != ParameterMode::Consume)
                {
                    return Err(self.permission_error(id, primary, "consume"));
                }
                self.conflict(id, primary, "consume")?;
                self.mark_consumed(
                    id,
                    Some((
                        selection
                            .mode_span
                            .clone()
                            .unwrap_or_else(|| selection.keyword_span.clone()),
                        false,
                    )),
                );
                self.emitted(
                    Op::TransferValue(id),
                    selected_ty.clone(),
                    selection.scrutinee.span().clone(),
                    Access::Own,
                    false,
                )
            }
            Some(Mode::Inout) => {
                let (place_root, _) = parse::place_parts(&selection.scrutinee)
                    .ok_or(CompileError::InvalidIr("match edit place"))?;
                let name =
                    place_name(place_root).ok_or(CompileError::InvalidIr("match edit place"))?;
                let id = self.get(name)?;
                self.check_available(id, selection.scrutinee.span().clone())?;
                if self.locals[id.0].parameter_mode == Some(ParameterMode::Read) {
                    return Err(self.permission_error(
                        id,
                        selection.scrutinee.span().clone(),
                        "borrow_inout",
                    ));
                }
                if !self.locals[id.0].mutable {
                    return Err(self.immutable_write(&selection.scrutinee, id));
                }
                self.conflict(id, selection.scrutinee.span().clone(), "borrow_inout")?;
                Flow::Value {
                    temp: None,
                    ty: selected_ty.clone(),
                }
            }
            _ if parse::place_parts(&selection.scrutinee).is_some()
                && !(place_name(&selection.scrutinee) == Some("None")
                    && self
                        .visible
                        .get("None")
                        .and_then(|ids| ids.last())
                        .is_none()) =>
            {
                Flow::Value {
                    temp: None,
                    ty: selected_ty.clone(),
                }
            }
            _ => return Err(CompileError::InvalidIr("match scrutinee task")),
        };
        self.begin_match_selection_after_selected(
            selection,
            expression,
            depth,
            expected,
            prelude,
            selected,
            Vec::new(),
            false,
        )
    }
    pub(super) fn match_selection_prelude(
        &mut self,
        selection: &parse::Match,
    ) -> Result<MatchSelectionPrelude, CompileError> {
        let outer_temps = self.live_temps.clone();
        if self.emission == EmissionMode::Live {
            self.pending_charges.push(WorkCharge {
                ticks: 1,
                origin: selection.keyword_span.clone(),
            });
        }
        let selected_ty = self.resolved_expr(&selection.scrutinee)?;
        Ok(MatchSelectionPrelude {
            outer_temps,
            selected_ty,
        })
    }
    pub(super) fn match_needs_scrutinee_expr(&self, selection: &parse::Match) -> bool {
        selection.mode == Some(Mode::Read)
            || (selection.mode.is_none()
                && (parse::place_parts(&selection.scrutinee).is_none()
                    || (place_name(&selection.scrutinee) == Some("None")
                        && self
                            .visible
                            .get("None")
                            .and_then(|ids| ids.last())
                            .is_none())))
    }
    #[allow(clippy::too_many_arguments)] // The continuation carries the already evaluated selector state.
    pub(super) fn begin_match_selection_after_selected<'a>(
        &mut self,
        selection: &'a parse::Match,
        expression: bool,
        depth: usize,
        expected: Option<&Ty>,
        prelude: MatchSelectionPrelude,
        selected: Flow,
        indexes: Vec<TempId>,
        loan_prepared: bool,
    ) -> Result<MatchStart<'a>, CompileError> {
        let MatchSelectionPrelude {
            outer_temps,
            selected_ty,
        } = prelude;
        if selected.diverged() {
            return Ok(MatchStart::Completed(Flow::Diverged {
                ty: expected.cloned(),
            }));
        }
        let loan_root = if selection.mode.is_none() || selection.mode == Some(Mode::Inout) {
            let name =
                parse::place_parts(&selection.scrutinee).and_then(|(root, _)| place_name(root));
            if name == Some("None")
                && self
                    .visible
                    .get("None")
                    .and_then(|ids| ids.last())
                    .is_none()
            {
                None
            } else {
                name.map(|name| self.get(name)).transpose()?
            }
        } else {
            None
        };
        let loan_temp = if let Some(id) = loan_root {
            let origin = selection.scrutinee.span().clone();
            let loan = LoanId(origin.start as usize);
            if selection.mode.is_none() && !loan_prepared {
                self.check_available(id, origin.clone())?;
                self.conflict(id, origin.clone(), "read")?;
            }
            let held = self.emit(
                Op::BeginLoan {
                    root: id,
                    indexes,
                    site: index_site(&selection.scrutinee, origin.clone()),
                    loan,
                    origin_span: origin.clone(),
                    permission: if selection.mode == Some(Mode::Inout) {
                        LoanPermission::Inout
                    } else {
                        LoanPermission::Read
                    },
                    parent: (self.locals[id.0].parameter_mode == Some(ParameterMode::Inout))
                        .then_some(id),
                },
                selected_ty.clone(),
                origin.clone(),
                Access::Read,
                false,
            );
            self.active_loans.entry(id).or_default().push(ActiveLoan {
                permission: if selection.mode == Some(Mode::Inout) {
                    LoanPermission::Inout
                } else {
                    LoanPermission::Read
                },
                span: origin,
                loan,
                parent: None,
                compound_assignment: false,
            });
            held
        } else {
            None
        };
        let owner = selection.mode == Some(Mode::Consume) || loan_root.is_none();
        let root = LocalId(self.locals.len());
        self.locals.push(Local {
            id: root,
            name: String::new(),
            name_span: selection.scrutinee.span().clone(),
            ty: selected_ty.clone(),
            mutable: owner || selection.mode == Some(Mode::Inout),
            scope: self.scope,
            parameter_mode: if selection.mode == Some(Mode::Inout) {
                Some(ParameterMode::Inout)
            } else {
                (!owner).then_some(ParameterMode::Read)
            },
        });
        self.name_spans.push(selection.scrutinee.span().clone());
        if selection.mode == Some(Mode::Inout) {
            self.statement_op(
                Op::BindEdit {
                    local: root,
                    root: loan_root.ok_or(CompileError::InvalidIr("edit source"))?,
                    proof: root,
                    loan: loan_temp,
                    path: Vec::new(),
                },
                selection.scrutinee.span().clone(),
                Access::Write,
                false,
            );
        } else if let Some(value) = selected.temp().or(loan_temp) {
            self.statement_op(
                if owner {
                    Op::WritePlace { local: root, value }
                } else {
                    Op::BindView {
                        local: root,
                        value,
                        hold: loan_temp == Some(value),
                    }
                },
                selection.scrutinee.span().clone(),
                Access::Write,
                false,
            );
        }
        self.match_boundaries.push(MatchBoundary {
            root,
            span: Span {
                file: selection.span.file.clone(),
                start: selection.span.end.saturating_sub(1),
                end: selection.span.end,
            },
        });
        let result_ty = expected.cloned().unwrap_or(Ty::Unit);
        let prior_locals = root.0;
        let checkpoint = self.consumed_undo.len();
        let temp_checkpoint = self.temp_undo.len();
        let branch_mode = self.emission;
        let previous_branches: Vec<BlockId> = Vec::new();
        let ends = Vec::new();
        let states = Vec::new();
        let slot = None;
        self.dead_depth += 1;
        Ok(MatchStart::Active(Box::new(MatchInnerState {
            selection,
            expression,
            depth,
            expected: expected.cloned(),
            outer_temps,
            selected_ty,
            loan_root,
            loan_temp,
            root,
            result_ty,
            prior_locals,
            checkpoint,
            temp_checkpoint,
            branch_mode,
            previous_branches,
            ends,
            states,
            slot,
            arm_index: 0,
        })))
    }
    pub(super) fn begin_match_arm<'a>(
        &mut self,
        state: &mut MatchInnerState<'a>,
        diagnostics: &mut Vec<Diagnostic>,
    ) -> Result<MatchArmStart<'a>, CompileError> {
        let Some(arm) = state.selection.arms.get(state.arm_index) else {
            return Ok(MatchArmStart::Done(None));
        };
        state.arm_index += 1;
        let selection = state.selection;
        let selected_ty = &state.selected_ty;
        let loan_root = state.loan_root;
        let loan_temp = state.loan_temp;
        let root = state.root;
        let branch_mode = state.branch_mode;
        let fallthrough = state.previous_branches.first().copied();
        for branch in state.previous_branches.drain(..) {
            let next = BlockId(self.blocks.len());
            if let Terminator::BranchBool { no, .. } = &mut self.blocks[branch.0].terminator {
                *no = next;
            }
        }
        if let Some(branch) = fallthrough {
            self.enter_edge(branch, 1)?;
        }
        let condition = self.match_condition(root, selected_ty, &arm.pattern)?;
        let branch = if branch_mode == EmissionMode::Live {
            let branch = self.finish_block(Terminator::BranchBool {
                condition: condition.ok_or(CompileError::InvalidIr("match condition"))?,
                yes: BlockId(self.blocks.len() + 1),
                no: BlockId(usize::MAX),
                span: arm.pattern.span().clone(),
                charges: Vec::new(),
            });
            Some(branch)
        } else {
            None
        };
        if let Some(branch) = branch {
            state.previous_branches.push(branch);
        }
        if let Some(guard) = &arm.guard {
            let violations = guard_violations(guard);
            for (subcause, primary) in &violations {
                diagnostics.push(*crate::check::names::diagnostic(
                    "NX-E1305",
                    Some(subcause),
                    diagnostic::Stage::Ownership,
                    diagnostic::Operation::OwnershipCheck,
                    primary.clone(),
                ));
            }
            let saved = self.scope;
            self.scope = ScopeId(self.next_scope);
            self.next_scope += 1;
            self.scope_parents.push(Some(saved));
            self.scopes.push(Vec::new());
            self.bind_match_pattern(
                root,
                selected_ty,
                &arm.pattern,
                &mut Vec::new(),
                selection,
                MatchSource {
                    local: loan_root,
                    loan: loan_temp,
                    provisional: true,
                },
            )?;
            if violations.is_empty() {
                return Ok(MatchArmStart::Guard { arm, guard, saved });
            }
            // A forbidden guard has no committed effects. Use a Bool
            // condition so the same arm and fallthrough analysis can
            // continue and report independent ownership sites.
            let checked = self.emitted(
                Op::ConstBool(true),
                Ty::Bool,
                guard.span().clone(),
                Access::Own,
                false,
            );
            return self
                .finish_match_guard(state, arm, saved, checked)
                .map(|arm| MatchArmStart::Done(Some(arm)));
        }
        self.bind_match_arm_body(state, arm)
            .map(|arm| MatchArmStart::Done(Some(arm)))
    }
    pub(super) fn finish_match_guard<'a>(
        &mut self,
        state: &mut MatchInnerState<'a>,
        arm: &'a parse::Arm,
        saved: ScopeId,
        checked: Flow,
    ) -> Result<MatchArmState<'a>, CompileError> {
        let guard = arm
            .guard
            .as_ref()
            .ok_or(CompileError::InvalidIr("match guard"))?;
        let guard_names = self
            .scopes
            .last()
            .cloned()
            .ok_or(CompileError::InvalidIr("guard scope"))?;
        let guard_locals = guard_names
            .iter()
            .rev()
            .map(|name| self.get(name).map(|id| (id, arm.body.close_span.clone())))
            .collect::<Result<Vec<_>, _>>()?;
        self.plan_exit(
            ExitPlan {
                surviving: self.live_temps.clone(),
                locals: guard_locals,
                reserved: None,
            },
            &arm.body.close_span,
        );
        self.leave_scope(saved)?;
        if state.branch_mode == EmissionMode::Live {
            let guard_branch = self.finish_block(Terminator::BranchBool {
                condition: checked
                    .temp()
                    .ok_or(CompileError::InvalidIr("match guard condition"))?,
                yes: BlockId(self.blocks.len() + 1),
                no: BlockId(usize::MAX),
                span: guard.span().clone(),
                charges: Vec::new(),
            });
            state.previous_branches.push(guard_branch);
        }
        self.bind_match_arm_body(state, arm)
    }
    fn bind_match_arm_body<'a>(
        &mut self,
        state: &mut MatchInnerState<'a>,
        arm: &'a parse::Arm,
    ) -> Result<MatchArmState<'a>, CompileError> {
        let selection = state.selection;
        let selected_ty = &state.selected_ty;
        let root = state.root;
        let loan_root = state.loan_root;
        let loan_temp = state.loan_temp;
        let saved = self.scope;
        self.scope = ScopeId(self.next_scope);
        self.next_scope += 1;
        self.scope_parents.push(Some(saved));
        self.scopes.push(Vec::new());
        let arm_surviving = self.live_temps.clone();
        self.bind_match_pattern(
            root,
            selected_ty,
            &arm.pattern,
            &mut Vec::new(),
            selection,
            MatchSource {
                local: loan_root,
                loan: loan_temp,
                provisional: false,
            },
        )?;
        Ok(MatchArmState {
            arm,
            saved,
            surviving: arm_surviving,
        })
    }
    pub(super) fn finish_match_arm(
        &mut self,
        state: &mut MatchInnerState<'_>,
        arm_state: MatchArmState<'_>,
        value: Option<Flow>,
    ) -> Result<(), CompileError> {
        let arm = arm_state.arm;
        let saved = arm_state.saved;
        let arm_surviving = arm_state.surviving;
        let result_ty = &state.result_ty;
        let branch_mode = state.branch_mode;
        if let Some(Flow::Value {
            temp: Some(temp),
            ty,
        }) = value
        {
            self.union_slot(&mut state.slot, temp, ty, result_ty, &arm.body.span)?;
        }
        let names = self
            .scopes
            .last()
            .cloned()
            .ok_or(CompileError::InvalidIr("match arm scope"))?;
        let locals = names
            .iter()
            .rev()
            .map(|name| self.get(name).map(|id| (id, arm.body.close_span.clone())))
            .collect::<Result<Vec<_>, _>>()?;
        if self
            .terminated
            .is_some_and(|term| matches!(term, Termination::Return(_)))
            && self.emission == EmissionMode::Live
        {
            self.deferred_return_locals.extend(locals);
        } else if self.emission == EmissionMode::Live {
            self.plan_exit(
                ExitPlan {
                    surviving: arm_surviving,
                    locals,
                    reserved: None,
                },
                &arm.body.close_span,
            );
        }
        self.leave_scope(saved)?;
        self.emission = branch_mode;
        let continuing = self.terminated.is_none();
        let termination = self.terminated.take();
        if branch_mode == EmissionMode::Live && termination != Some(Termination::Diverged) {
            let end = self.finish_block(if let Some(Termination::Return(value)) = termination {
                Terminator::Return {
                    value: Some(value),
                    span: arm.body.span.clone(),
                    charges: Vec::new(),
                }
            } else {
                Terminator::Jump {
                    target: BlockId(usize::MAX),
                    span: arm.body.span.clone(),
                    charges: Vec::new(),
                }
            });
            state.ends.push(end);
        }
        state
            .states
            .push((self.restore_availability(state.checkpoint), continuing));
        self.restore_temps(state.temp_checkpoint);
        Ok(())
    }
    pub(super) fn finish_match_selection(
        &mut self,
        state: Box<MatchInnerState<'_>>,
    ) -> Result<Flow, CompileError> {
        let MatchInnerState {
            selection,
            expression,
            outer_temps,
            root,
            result_ty,
            prior_locals,
            temp_checkpoint,
            branch_mode,
            previous_branches,
            ends,
            states,
            slot,
            ..
        } = *state;
        self.dead_depth -= 1;
        if branch_mode == EmissionMode::Live {
            let fallback = BlockId(self.blocks.len());
            let fallthrough = previous_branches.first().copied();
            for branch in previous_branches {
                if let Terminator::BranchBool { no, .. } = &mut self.blocks[branch.0].terminator {
                    *no = fallback;
                }
            }
            if let Some(branch) = fallthrough {
                self.enter_edge(branch, 1)?;
            }
            let close_span = Span {
                file: selection.span.file.clone(),
                start: selection.span.end.saturating_sub(1),
                end: selection.span.end,
            };
            self.plan_exit(
                ExitPlan {
                    surviving: std::collections::HashSet::new(),
                    locals: vec![(root, close_span.clone())],
                    reserved: None,
                },
                &close_span,
            );
            self.finish_block(Terminator::Unreachable {
                span: selection.span.clone(),
                charges: Vec::new(),
            });
            self.restore_temps(temp_checkpoint);
        }
        let join = BlockId(self.blocks.len());
        let continuing_end = ends
            .iter()
            .copied()
            .find(|end| matches!(self.blocks[end.0].terminator, Terminator::Jump { .. }));
        for end in ends {
            if let Terminator::Jump { target, .. } = &mut self.blocks[end.0].terminator {
                *target = join;
            }
        }
        let all_diverge = states.iter().all(|(_, continues)| !continues);
        self.match_boundaries.pop();
        self.merge_match_states(states, prior_locals);
        if all_diverge {
            self.terminated = Some(Termination::Diverged);
            self.emission = EmissionMode::Analyze;
            return Ok(Flow::Diverged {
                ty: expression.then_some(result_ty),
            });
        }
        if let Some(end) = continuing_end {
            self.enter_edge(end, 0)?;
        }
        self.plan_exit(
            ExitPlan {
                surviving: outer_temps,
                locals: vec![(
                    root,
                    Span {
                        file: selection.span.file.clone(),
                        start: selection.span.end.saturating_sub(1),
                        end: selection.span.end,
                    },
                )],
                reserved: None,
            },
            &selection.span,
        );
        if !expression {
            return Ok(Flow::Value {
                temp: None,
                ty: Ty::Unit,
            });
        }
        let Some(slot) = slot else {
            return Ok(Flow::Diverged {
                ty: Some(result_ty),
            });
        };
        Ok(self.emitted(
            Op::TransferValue(slot),
            result_ty,
            selection.span.clone(),
            Access::Own,
            false,
        ))
    }
    fn match_condition(
        &mut self,
        root: LocalId,
        ty: &Ty,
        pattern: &Pattern,
    ) -> Result<Option<TempId>, CompileError> {
        let mut tests = Vec::new();
        self.match_tests(root, ty, pattern, &mut Vec::new(), &mut tests)?;
        let mut condition = None;
        for test in tests {
            condition = Some(if let Some(previous) = condition {
                self.emit(
                    Op::Binary {
                        op: Binary::And,
                        left: previous,
                        right: test,
                    },
                    Ty::Bool,
                    pattern.span().clone(),
                    Access::Own,
                    false,
                )
                .ok_or(CompileError::InvalidIr("match conjunction"))?
            } else {
                test
            });
        }
        if condition.is_none() {
            condition = self.emit(
                Op::ConstBool(true),
                Ty::Bool,
                pattern.span().clone(),
                Access::Own,
                false,
            );
        }
        Ok(condition)
    }

    fn match_tests(
        &mut self,
        root: LocalId,
        ty: &Ty,
        pattern: &Pattern,
        path: &mut Vec<VariantTag>,
        tests: &mut Vec<TempId>,
    ) -> Result<(), CompileError> {
        if self.emission == EmissionMode::Analyze && *ty == Ty::Never {
            return Ok(());
        }
        let mut current_ty = ty;
        let mut current_pattern = pattern;
        let original_path_len = path.len();
        let result = (|| -> Result<(), CompileError> {
            loop {
                let op = match current_pattern {
                    Pattern::Wildcard(_) => Some(Op::TestLiteral {
                        local: root,
                        path: path.clone(),
                        literal: MatchLiteral::Any,
                    }),
                    Pattern::Name { name, .. }
                        if name == "None"
                            && matches!(
                                current_ty,
                                Ty::Union {
                                    kind: UnionKind::Option,
                                    ..
                                }
                            ) =>
                    {
                        Some(Op::TestVariant {
                            local: root,
                            path: path.clone(),
                            tag: VariantTag(0),
                        })
                    }
                    Pattern::Name { .. } => Some(Op::TestLiteral {
                        local: root,
                        path: path.clone(),
                        literal: MatchLiteral::Any,
                    }),
                    Pattern::Integer(value, _) => Some(Op::TestLiteral {
                        local: root,
                        path: path.clone(),
                        literal: MatchLiteral::I64(
                            value
                                .parse()
                                .map_err(|_| CompileError::InvalidIr("pattern integer"))?,
                        ),
                    }),
                    Pattern::Bool(value, _) => Some(Op::TestLiteral {
                        local: root,
                        path: path.clone(),
                        literal: MatchLiteral::Bool(*value),
                    }),
                    Pattern::Constructor { name, args, .. } => {
                        let union = current_ty
                            .union_descriptor()
                            .ok_or(CompileError::InvalidIr("pattern union"))?;
                        let tag = union
                            .kind
                            .variant_tag(name)
                            .ok_or(CompileError::InvalidIr("pattern variant"))?;
                        let op = Op::TestVariant {
                            local: root,
                            path: path.clone(),
                            tag,
                        };
                        if let Some(test) = self.emit(
                            op,
                            Ty::Bool,
                            current_pattern.span().clone(),
                            Access::Read,
                            false,
                        ) {
                            tests.push(test);
                        }
                        if let Some(arg) = args.first() {
                            let payload = union
                                .variant(tag)
                                .and_then(|variant| variant.payload)
                                .ok_or(CompileError::InvalidIr("pattern payload"))?;
                            path.push(tag);
                            current_ty = payload;
                            current_pattern = arg;
                            continue;
                        }
                        break;
                    }
                };
                if let Some(op) = op
                    && let Some(test) = self.emit(
                        op,
                        Ty::Bool,
                        current_pattern.span().clone(),
                        Access::Read,
                        false,
                    )
                {
                    tests.push(test);
                }
                break;
            }
            Ok(())
        })();
        path.truncate(original_path_len);
        result
    }

    fn bind_match_pattern(
        &mut self,
        root: LocalId,
        ty: &Ty,
        pattern: &Pattern,
        path: &mut Vec<VariantTag>,
        selection: &parse::Match,
        source: MatchSource,
    ) -> Result<(), CompileError> {
        let saved_path_len = path.len();
        let result = (|| {
            let mut ty = ty;
            let mut pattern = pattern;
            while let Pattern::Constructor { name, args, .. } = pattern {
                if self.emission == EmissionMode::Analyze && *ty == Ty::Never {
                    if let Some(arg) = args.first() {
                        pattern = arg;
                        continue;
                    }
                    break;
                }
                let union = ty
                    .union_descriptor()
                    .ok_or(CompileError::InvalidIr("pattern union"))?;
                let tag = union
                    .kind
                    .variant_tag(name)
                    .ok_or(CompileError::InvalidIr("pattern variant"))?;
                let (Some(arg), Some(payload)) = (
                    args.first(),
                    union.variant(tag).and_then(|variant| variant.payload),
                ) else {
                    break;
                };
                path.push(tag);
                ty = payload;
                pattern = arg;
            }
            match pattern {
                Pattern::Name { name, mode, span }
                    if !(name == "None"
                        && matches!(
                            ty,
                            Ty::Union {
                                kind: UnionKind::Option,
                                ..
                            }
                        ))
                        && !(self.emission == EmissionMode::Analyze
                            && *ty == Ty::Never
                            && !self.inferred_bindings.bindings.contains_key(&span.start)) =>
                {
                    if !source.provisional
                        && *mode == Some(Mode::Consume)
                        && self.locals[root.0].parameter_mode.is_some()
                    {
                        return Err(self.pattern_permission_error(
                            selection,
                            source.local.unwrap_or(root),
                            span.clone(),
                            "consume",
                        ));
                    }
                    if !source.provisional
                        && *mode == Some(Mode::Inout)
                        && selection.mode != Some(Mode::Inout)
                    {
                        return Err(self.pattern_permission_error(
                            selection,
                            source.local.unwrap_or(root),
                            span.clone(),
                            "borrow_inout",
                        ));
                    }
                    let binding_ty = self.resolved_type(
                        self.inferred_bindings
                            .bindings
                            .get(&span.start)
                            .ok_or(CompileError::InvalidIr("match binding type"))?,
                    )?;
                    let moved = !source.provisional && *mode == Some(Mode::Consume);
                    let edited = !source.provisional && *mode == Some(Mode::Inout);
                    let id = LocalId(self.locals.len());
                    self.locals.push(Local {
                        id,
                        name: name.clone(),
                        name_span: span.clone(),
                        ty: binding_ty.clone(),
                        mutable: edited,
                        scope: self.scope,
                        parameter_mode: if edited {
                            Some(ParameterMode::Inout)
                        } else {
                            (!moved).then_some(ParameterMode::Read)
                        },
                    });
                    self.name_spans.push(span.clone());
                    self.visible.entry(name.clone()).or_default().push(id);
                    self.scopes
                        .last_mut()
                        .ok_or(CompileError::InvalidIr("match scope"))?
                        .push(name.clone());
                    if edited {
                        self.statement_op(
                            Op::BindEdit {
                                local: id,
                                root: source.local.ok_or(CompileError::InvalidIr("edit source"))?,
                                proof: root,
                                loan: source.loan,
                                path: path.clone(),
                            },
                            span.clone(),
                            Access::Write,
                            false,
                        );
                        return Ok(());
                    }
                    let flow = if moved && path.is_empty() {
                        self.mark_consumed(root, Some((span.clone(), false)));
                        self.emitted(
                            Op::TransferValue(root),
                            binding_ty,
                            span.clone(),
                            Access::Own,
                            false,
                        )
                    } else if moved {
                        self.emitted(
                            Op::ExtractPayload {
                                local: root,
                                path: path.clone(),
                            },
                            binding_ty,
                            span.clone(),
                            Access::Write,
                            false,
                        )
                    } else if path.is_empty() {
                        self.emitted(
                            Op::BorrowValue(root),
                            binding_ty,
                            span.clone(),
                            Access::Read,
                            false,
                        )
                    } else {
                        self.emitted(
                            Op::ProjectPayload {
                                local: root,
                                path: path.clone(),
                            },
                            binding_ty,
                            span.clone(),
                            Access::Read,
                            false,
                        )
                    };
                    if let Some(value) = flow.temp() {
                        self.statement_op(
                            if moved {
                                Op::WritePlace { local: id, value }
                            } else {
                                Op::BindView {
                                    local: id,
                                    value,
                                    hold: false,
                                }
                            },
                            span.clone(),
                            Access::Write,
                            false,
                        );
                    }
                }
                Pattern::Constructor { .. } => {}
                _ => {}
            }
            Ok(())
        })();
        path.truncate(saved_path_len);
        result
    }

    fn pattern_permission_error(
        &self,
        selection: &parse::Match,
        source: LocalId,
        primary: Span,
        requested: &str,
    ) -> CompileError {
        let mut errors = match self.ownership_error(
            "NX-E1105",
            Some("permission_escalation"),
            diagnostic::Operation::OwnershipAccess,
            primary,
            source,
            None,
        ) {
            CompileError::Rejected(errors) => errors,
            other => return other,
        };
        let Some(diagnostic) = errors.first_mut() else {
            return CompileError::InvalidIr("missing permission diagnostic");
        };
        let origin = self.local_entity(source);
        let loan = diagnostic::Entity {
            kind: diagnostic::EntityKind::Loan,
            id: format!("loan:{}:0", selection.scrutinee.span().start),
        };
        let data_entity = |entity: &diagnostic::Entity| {
            let mut fields = std::collections::BTreeMap::new();
            fields.insert(
                "kind".into(),
                diagnostic::DataValue::String(entity.kind.as_str().into()),
            );
            fields.insert(
                "id".into(),
                diagnostic::DataValue::String(entity.id.clone()),
            );
            diagnostic::DataValue::Object(fields)
        };
        diagnostic
            .data
            .insert("origin".into(), data_entity(&origin));
        diagnostic
            .data
            .insert("permission_source".into(), data_entity(&loan));
        diagnostic.data.insert(
            "active_permission".into(),
            diagnostic::DataValue::String(
                if selection.mode == Some(Mode::Inout) {
                    "inout"
                } else {
                    "read"
                }
                .into(),
            ),
        );
        diagnostic.data.insert(
            "requested_access".into(),
            diagnostic::DataValue::String(requested.into()),
        );
        diagnostic.related_spans.push(diagnostic::RelatedSpan {
            role: diagnostic::RelatedRole::Origin,
            span: selection.scrutinee.span().clone(),
        });
        diagnostic.cause_chain.push(diagnostic::Cause {
            kind: diagnostic::CauseKind::Borrow,
            operation: if selection.mode == Some(Mode::Inout) {
                diagnostic::CauseOperation::BorrowInout
            } else {
                diagnostic::CauseOperation::BorrowRead
            },
            span: Some(selection.scrutinee.span().clone()),
            entity: Some(loan),
            evidence: diagnostic::Evidence::Verified,
        });
        CompileError::Rejected(errors)
    }

    fn merge_match_states(
        &mut self,
        states: Vec<(HashMap<LocalId, Availability>, bool)>,
        prior_locals: usize,
    ) {
        let ids: HashSet<LocalId> = states
            .iter()
            .flat_map(|(state, _)| state.keys().copied())
            .filter(|id| id.0 < prior_locals)
            .collect();
        for id in ids {
            let before = self.availability(id);
            let continuing: Vec<Availability> = states
                .iter()
                .filter(|(_, continues)| *continues)
                .map(|(state, _)| state.get(&id).cloned().unwrap_or_else(|| before.clone()))
                .collect();
            if continuing.is_empty() {
                continue;
            }
            let all_consumed = continuing.iter().all(|state| state.definitely.is_some());
            let sites = continuing.into_iter().fold(Vec::new(), |sites, state| {
                merge_sites(sites, state.definitely.or(state.maybe).unwrap_or_default())
            });
            self.set_availability(
                id,
                Availability {
                    definitely: all_consumed.then_some(sites.clone()),
                    maybe: (!all_consumed && !sites.is_empty()).then_some(sites),
                },
            );
        }
    }
}

#[cfg(test)]
mod guard_traversal_tests {
    use super::*;
    use std::process::Command;

    #[test]
    fn nested_guard_reports_transfers_in_source_order() {
        let source = "fn Main(){match 1{_ if (move x)==Drop(1)=>{},_=>{}}}";
        let ast = parse::parse_new_profile("guard.n", source).expect("parse guard");
        let parse::Statement::Match(selection) = &ast.functions[0].body.statements[0] else {
            panic!("match statement");
        };
        let guard = selection.arms[0].guard.as_ref().expect("match guard");
        let violations = guard_violations(guard);
        assert_eq!(
            violations
                .iter()
                .map(|(cause, span)| (*cause, span.start, span.end))
                .collect::<Vec<_>>(),
            vec![
                (
                    "guard_transfer",
                    source.find("move").unwrap() as u64,
                    source.find("move").unwrap() as u64 + 4
                ),
                (
                    "guard_transfer",
                    source.find("Drop").unwrap() as u64,
                    source.find("Drop").unwrap() as u64 + 4
                ),
            ]
        );
    }

    #[test]
    fn deep_guard_child() {
        if std::env::var_os("J16_DEEP_GUARD_CHILD").is_none() {
            return;
        }
        let source = format!(
            "fn Main(){{match 1{{_ if {}move x{}=>{{}},_=>{{}}}}}}",
            "(".repeat(120),
            ")".repeat(120)
        );
        let ast = parse::parse_new_profile("guard.n", &source).expect("parse guard");
        let parse::Statement::Match(selection) = &ast.functions[0].body.statements[0] else {
            panic!("match statement");
        };
        let guard = selection.arms[0].guard.as_ref().expect("match guard");
        std::thread::scope(|scope| {
            std::thread::Builder::new()
                .stack_size(64 * 1024)
                .spawn_scoped(scope, || {
                    let violations = guard_violations(guard);
                    assert_eq!(violations.len(), 1);
                    assert_eq!(violations[0].0, "guard_transfer");
                    assert_eq!(violations[0].1.start, source.find("move").unwrap() as u64);
                })
                .unwrap()
                .join()
                .unwrap();
        });
    }

    #[test]
    fn nested_guard_fits_small_worker() {
        let output = Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "ir::lower::lower_match::guard_traversal_tests::deep_guard_child",
            ])
            .env("J16_DEEP_GUARD_CHILD", "1")
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{} {}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }
}
