use super::*;
use crate::frontend::parse::Pattern;
use crate::ir::model::MatchLiteral;
use crate::union::{UnionKind, VariantTag};

#[derive(Clone, Copy)]
struct MatchSource {
    local: Option<LocalId>,
    loan: Option<TempId>,
}

impl Lower<'_> {
    pub(super) fn match_selection(
        &mut self,
        selection: &parse::Match,
        expression: bool,
        depth: usize,
        expected: Option<&Ty>,
        diagnostics: &mut Vec<Diagnostic>,
    ) -> Result<Flow, CompileError> {
        let selected_ty = self.resolved_expr(&selection.scrutinee)?;
        let selected = match selection.mode {
            Some(Mode::Consume) => {
                let name = place_name(&selection.scrutinee)
                    .ok_or(CompileError::InvalidIr("match move place"))?;
                let id = self.get(name)?;
                self.check_available(id, selection.scrutinee.span().clone())?;
                if self.locals[id.0]
                    .parameter_mode
                    .is_some_and(|mode| mode != ParameterMode::Consume)
                {
                    return Err(self.permission_error(
                        id,
                        selection.scrutinee.span().clone(),
                        "consume",
                    ));
                }
                self.conflict(id, selection.scrutinee.span().clone(), "consume")?;
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
            _ => self.expr(&selection.scrutinee, depth + 1, Some(&selected_ty))?,
        };
        if selected.diverged() {
            return Ok(Flow::Diverged {
                ty: expected.cloned(),
            });
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
            if selection.mode.is_none() {
                self.check_available(id, origin.clone())?;
                self.conflict(id, origin.clone(), "read")?;
            }
            let (_, index_exprs) = parse::place_parts(&selection.scrutinee)
                .ok_or(CompileError::InvalidIr("match loan place"))?;
            let mut indexes = Vec::with_capacity(index_exprs.len());
            for index_expr in index_exprs {
                let flow = self.expr_mode(index_expr, depth + 1, Some(&Ty::I64), true)?;
                if flow.diverged() {
                    return Ok(Flow::Diverged {
                        ty: expected.cloned(),
                    });
                }
                indexes.push(flow.temp().ok_or(CompileError::InvalidIr("match index"))?);
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
        let result_ty = expected.cloned().unwrap_or(Ty::Unit);
        let prior_locals = root.0;
        let checkpoint = self.consumed_undo.len();
        let temp_checkpoint = self.temp_undo.len();
        let branch_mode = self.emission;
        let mut previous_branch: Option<BlockId> = None;
        let mut ends = Vec::new();
        let mut states = Vec::new();
        let mut slot = None;
        self.dead_depth += 1;
        for (index, arm) in selection.arms.iter().enumerate() {
            if let Some(branch) = previous_branch {
                let next = BlockId(self.blocks.len());
                if let Terminator::BranchBool { no, .. } = &mut self.blocks[branch.0].terminator {
                    *no = next;
                }
            }
            let condition = self.match_condition(root, &selected_ty, &arm.pattern)?;
            let branch = if branch_mode == EmissionMode::Live {
                let final_arm = index + 1 == selection.arms.len();
                let no = if final_arm {
                    BlockId(self.blocks.len() + 1)
                } else {
                    BlockId(usize::MAX)
                };
                let yes = BlockId(self.blocks.len() + if final_arm { 2 } else { 1 });
                let branch = self.finish_block(Terminator::BranchBool {
                    condition: condition.ok_or(CompileError::InvalidIr("match condition"))?,
                    yes,
                    no,
                    span: arm.pattern.span().clone(),
                    charges: Vec::new(),
                });
                if final_arm {
                    let close_span = Span {
                        file: selection.span.file.clone(),
                        start: selection.span.end.saturating_sub(1),
                        end: selection.span.end,
                    };
                    self.statement_op(Op::DropLocal(root), close_span.clone(), Access::Own, false);
                    if let Some(held) = loan_temp {
                        self.statement_op(Op::DropValue(held), close_span, Access::Own, false);
                    }
                    self.finish_block(Terminator::Unreachable {
                        span: arm.pattern.span().clone(),
                        charges: Vec::new(),
                    });
                }
                Some(branch)
            } else {
                None
            };
            previous_branch = branch;
            let saved = self.scope;
            self.scope = ScopeId(self.next_scope);
            self.next_scope += 1;
            self.scope_parents.push(Some(saved));
            self.scopes.push(Vec::new());
            self.bind_match_pattern(
                root,
                &selected_ty,
                &arm.pattern,
                &mut Vec::new(),
                selection,
                MatchSource {
                    local: loan_root,
                    loan: loan_temp,
                },
            )?;
            let value = if expression {
                self.scoped_value_block(
                    &arm.body,
                    expected,
                    &selection.keyword_span,
                    depth + 1,
                    diagnostics,
                )?
            } else {
                self.scoped_block(&arm.body, depth + 1, diagnostics)?;
                None
            };
            if let Some(Flow::Value {
                temp: Some(temp),
                ty,
            }) = value
            {
                self.union_slot(&mut slot, temp, ty, &result_ty, &arm.body.span)?;
            }
            let names = self
                .scopes
                .last()
                .cloned()
                .ok_or(CompileError::InvalidIr("match arm scope"))?;
            for name in names.iter().rev() {
                let id = self.get(name)?;
                if !self.consumed_at.contains_key(&id) {
                    self.statement_op(
                        Op::DropLocal(id),
                        arm.body.close_span.clone(),
                        Access::Own,
                        false,
                    );
                }
            }
            self.leave_scope(saved)?;
            self.emission = branch_mode;
            let continuing = self.terminated.is_none();
            let termination = self.terminated.take();
            if branch_mode == EmissionMode::Live && termination != Some(Termination::Diverged) {
                let end =
                    self.finish_block(if let Some(Termination::Return(value)) = termination {
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
                ends.push(end);
            }
            states.push((self.restore_availability(checkpoint), continuing));
            self.restore_temps(temp_checkpoint);
        }
        self.dead_depth -= 1;
        let join = BlockId(self.blocks.len());
        for end in ends {
            if let Terminator::Jump { target, .. } = &mut self.blocks[end.0].terminator {
                *target = join;
            }
        }
        let all_diverge = states.iter().all(|(_, continues)| !continues);
        self.merge_match_states(states, prior_locals);
        if all_diverge {
            self.terminated = Some(Termination::Diverged);
            self.emission = EmissionMode::Analyze;
            return Ok(Flow::Diverged {
                ty: expression.then_some(result_ty),
            });
        }
        if let Some(held) = loan_temp {
            self.statement_op(
                Op::DropValue(held),
                selection.span.clone(),
                Access::Own,
                false,
            );
        }
        if let Some(id) = loan_root
            && let Some(loans) = self.active_loans.get_mut(&id)
        {
            loans.pop();
        }
        self.statement_op(
            Op::DropLocal(root),
            Span {
                file: selection.span.file.clone(),
                start: selection.span.end.saturating_sub(1),
                end: selection.span.end,
            },
            Access::Own,
            false,
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
        let op = match pattern {
            Pattern::Wildcard(_) => None,
            Pattern::Name { name, .. }
                if name == "None"
                    && matches!(
                        ty,
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
            Pattern::Name { .. } => None,
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
                let union = ty
                    .union_descriptor()
                    .ok_or(CompileError::InvalidIr("pattern union"))?;
                let tag = union
                    .kind
                    .variant_tag(name)
                    .ok_or(CompileError::InvalidIr("pattern variant"))?;
                let op = Some(Op::TestVariant {
                    local: root,
                    path: path.clone(),
                    tag,
                });
                if let Some(arg) = args.first() {
                    let payload = union
                        .variant(tag)
                        .and_then(|variant| variant.payload)
                        .ok_or(CompileError::InvalidIr("pattern payload"))?;
                    path.push(tag);
                    self.match_tests(root, payload, arg, path, tests)?;
                    path.pop();
                }
                op
            }
        };
        if let Some(op) = op
            && let Some(test) = self.emit(op, Ty::Bool, pattern.span().clone(), Access::Read, false)
        {
            tests.push(test);
        }
        Ok(())
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
        match pattern {
            Pattern::Name { name, mode, span }
                if !(name == "None"
                    && matches!(
                        ty,
                        Ty::Union {
                            kind: UnionKind::Option,
                            ..
                        }
                    )) =>
            {
                if *mode == Some(Mode::Consume) && self.locals[root.0].parameter_mode.is_some() {
                    return Err(self.pattern_permission_error(
                        selection,
                        source.local.unwrap_or(root),
                        span.clone(),
                        "consume",
                    ));
                }
                if *mode == Some(Mode::Inout) && selection.mode != Some(Mode::Inout) {
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
                let moved = *mode == Some(Mode::Consume);
                let edited = *mode == Some(Mode::Inout);
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
            Pattern::Constructor { name, args, .. } => {
                let union = ty
                    .union_descriptor()
                    .ok_or(CompileError::InvalidIr("pattern union"))?;
                let tag = union
                    .kind
                    .variant_tag(name)
                    .ok_or(CompileError::InvalidIr("pattern variant"))?;
                if let (Some(arg), Some(payload)) = (
                    args.first(),
                    union.variant(tag).and_then(|variant| variant.payload),
                ) {
                    path.push(tag);
                    self.bind_match_pattern(root, payload, arg, path, selection, source)?;
                    path.pop();
                }
            }
            _ => {}
        }
        Ok(())
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
        crate::check::names::sort_related(diagnostic);
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
