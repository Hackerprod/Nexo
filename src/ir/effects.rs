//! Post-ownership, IR-only effect summaries and written-contract checks.
use std::collections::{HashMap, HashSet, VecDeque};

use crate::diagnostic::{
    Cause, CauseKind, CauseOperation, DataValue, Entity, EntityKind, Evidence, Operation,
    RelatedRole, RelatedSpan, Span, Stage,
};
use crate::frontend::parse::{Ast, Statement, WithClause};

use super::model::{EffectSummary, Function, Op, Program, Ty};

const ALLOC: u8 = 1;
const BLOCK: u8 = 2;
const SUSPEND: u8 = 4;
const IO: u8 = 8;
const ALL: u8 = ALLOC | BLOCK | SUSPEND | IO;
const EFFECTS: [(u8, &str, &str); 4] = [
    (ALLOC, "alloc", "NX-E0412"),
    (BLOCK, "block", "NX-E0413"),
    (SUSPEND, "suspend", "NX-E0414"),
    (IO, "io", "NX-E0415"),
];
fn charge_effect(
    used: &mut u64,
    span: &Span,
    limit: u64,
    profile_id: &str,
) -> Result<(), Box<crate::diagnostic::Diagnostic>> {
    if *used >= limit {
        let mut diagnostic = crate::check::names::diagnostic(
            "NX-E0003",
            None,
            Stage::Effects,
            Operation::AnalysisLimit,
            span.clone(),
        );
        diagnostic.data.insert(
            "budget_kind".into(),
            DataValue::String("effect_fixpoint_steps".into()),
        );
        diagnostic
            .data
            .insert("limit".into(), DataValue::UInt(limit));
        diagnostic
            .data
            .insert("observed".into(), DataValue::UInt(limit + 1));
        diagnostic.profile_id = profile_id.into();
        return Err(diagnostic);
    }
    *used += 1;
    Ok(())
}

#[derive(Clone)]
struct Witness {
    summary: EffectSummary,
    primary: Span,
    operation: CauseOperation,
    chain: Vec<Cause>,
    related: Vec<RelatedSpan>,
}

fn call_head(span: &Span, source: &str) -> Span {
    let start = span.start as usize;
    let tail = source.get(start..).unwrap_or("");
    let end = tail
        .char_indices()
        .take_while(|(_, c)| c.is_ascii_alphanumeric() || *c == '_')
        .last()
        .map_or(0, |(i, c)| i + c.len_utf8());
    Span {
        file: span.file.clone(),
        start: span.start,
        end: span.start + end as u64,
    }
}

fn resource_effect(ty: &Ty) -> u8 {
    match ty {
        Ty::MockResource => IO,
        Ty::MockBlockingResource => BLOCK | IO,
        Ty::Result(ok, err) => resource_effect(ok) | resource_effect(err),
        _ => 0,
    }
}

fn copies_array(ty: &Ty) -> bool {
    match ty {
        Ty::Array(_) => true,
        Ty::Result(ok, err) => copies_array(ok) || copies_array(err),
        _ => false,
    }
}

fn local_entity(f: &Function, local: super::model::LocalId) -> Entity {
    let local = &f.locals[local.0];
    let kind = if local.parameter_mode.is_some() {
        EntityKind::Parameter
    } else {
        EntityKind::Binding
    };
    Entity {
        kind,
        id: format!("{}:{}", kind.as_str(), local.name_span.start),
    }
}

fn variant_effect(ty: &Ty, variant: Option<bool>) -> u8 {
    match (ty, variant) {
        (Ty::Result(ok, _), Some(true)) => resource_effect(ok),
        (Ty::Result(_, err), Some(false)) => resource_effect(err),
        _ => resource_effect(ty),
    }
}

fn resource_types(ty: &Ty, variant: Option<bool>) -> Vec<&'static str> {
    match (ty, variant) {
        (Ty::MockResource, _) => vec!["MockResource"],
        (Ty::MockBlockingResource, _) => vec!["MockBlockingResource"],
        (Ty::Result(ok, _), Some(true)) => resource_types(ok, None),
        (Ty::Result(_, err), Some(false)) => resource_types(err, None),
        (Ty::Result(ok, err), None) => {
            let mut types = resource_types(ok, None);
            types.extend(resource_types(err, None));
            types
        }
        _ => Vec::new(),
    }
}

fn intrinsic_effect(intrinsic: super::model::Intrinsic) -> (u8, u8, &'static str) {
    use super::model::Intrinsic::*;
    match intrinsic {
        Print | Inspect | MockOpen | MockBlockingOpen | MockWriteI64 | MockRequest | MockReply => {
            (IO, 0, "intrinsic.call")
        }
        MockReadRecords => (ALLOC | IO, 0, "intrinsic.call"),
        MockUnknown => (0, ALL, "intrinsic.call"),
        Fail | AssertEq | DropData | Len | TryAdd => (0, 0, "intrinsic.call"),
    }
}

fn explicit_drop_sources(f: &Function) -> HashMap<usize, super::model::LocalId> {
    use super::model::{Intrinsic, Op};
    let mut transferred = HashMap::new();
    let mut dropped = HashMap::new();
    for ins in std::iter::once(&f.block)
        .chain(&f.blocks)
        .flat_map(|b| &b.instructions)
    {
        if let (Some(output), Op::TransferValue(local)) = (ins.output, &ins.op) {
            transferred.insert(output.0, *local);
        }
        if let Op::CallIntrinsic {
            intrinsic: Intrinsic::DropData,
            args,
        } = &ins.op
            && let Some(arg) = args.first()
            && let Some(local) = transferred.get(&arg.0)
        {
            dropped.insert(ins.origin.0, *local);
        }
    }
    dropped
}

fn function_intrinsic_effect(f: &Function) -> Vec<Witness> {
    let drop_sources = explicit_drop_sources(f);
    let exit_sites: HashMap<_, _> = f
        .exit_info
        .iter()
        .map(|exit| ((exit.block.0, exit.index), exit))
        .collect();
    let mut out = Vec::new();
    let temp_count = std::iter::once(&f.block)
        .chain(&f.blocks)
        .flat_map(|b| &b.instructions)
        .filter_map(|ins| ins.output.map(|id| id.0 + 1))
        .max()
        .unwrap_or(0);
    let mut temp_types = vec![None; temp_count];
    let mut temp_variants = vec![None; temp_count];
    let mut owned_temps = vec![false; temp_count];
    let mut temp_spans = vec![None; temp_count];
    for ins in std::iter::once(&f.block)
        .chain(&f.blocks)
        .flat_map(|b| &b.instructions)
    {
        if let (Some(id), Some(ty)) = (ins.output, ins.ty.as_ref())
            && id.0 < temp_types.len()
        {
            temp_types[id.0] = Some(ty.clone());
            temp_spans[id.0] = Some(ins.span.clone());
        }
        if let (Some(id), Op::BuildVariant { tag, .. }) = (ins.output, &ins.op)
            && id.0 < temp_variants.len()
        {
            temp_variants[id.0] = Some(tag.0 == 0);
        }
        if let Some(id) = ins.output
            && id.0 < owned_temps.len()
            && !matches!(
                ins.op,
                Op::BeginLoan { .. } | Op::BorrowValue(_) | Op::BorrowIndex { .. }
            )
        {
            owned_temps[id.0] = true;
        }
    }
    for block in std::iter::once(&f.block).chain(&f.blocks) {
        for (index, ins) in block.instructions.iter().enumerate() {
            if !super::model::is_drop_data_call(block, index) {
                super::count_cfg_work(1);
            }
            let (known, unknown, operation, entity) = match &ins.op {
                Op::BuildArray(_) => (ALLOC, 0, CauseOperation::BuildArray, "Array".to_owned()),
                Op::CopyValue(local) => {
                    let effect =
                        if f.locals[local.0].ty.copyable() && copies_array(&f.locals[local.0].ty) {
                            ALLOC
                        } else {
                            0
                        };
                    (
                        effect,
                        0,
                        CauseOperation::ValueCopy,
                        format!(
                            "{}:{}",
                            if f.locals[local.0].parameter_mode.is_some() {
                                "parameter"
                            } else {
                                "binding"
                            },
                            f.locals[local.0].name_span.start
                        ),
                    )
                }
                Op::ReadIndex { local, .. } => {
                    let ty = ins.ty.as_ref();
                    let effect = ty
                        .filter(|ty| ty.copyable() && copies_array(ty))
                        .map_or(0, |_| ALLOC);
                    (
                        effect,
                        0,
                        CauseOperation::ValueCopy,
                        format!(
                            "{}:{}",
                            if f.locals[local.0].parameter_mode.is_some() {
                                "parameter"
                            } else {
                                "binding"
                            },
                            f.locals[local.0].name_span.start
                        ),
                    )
                }
                Op::CallIntrinsic { intrinsic, args } => {
                    let (known, unknown, _) = intrinsic_effect(*intrinsic);
                    let name = match intrinsic {
                        super::model::Intrinsic::Print => "print",
                        super::model::Intrinsic::AssertEq => "assert_eq",
                        super::model::Intrinsic::DropData => "drop",
                        super::model::Intrinsic::Fail => "fail",
                        super::model::Intrinsic::Len => "len",
                        super::model::Intrinsic::MockOpen => "mock_open",
                        super::model::Intrinsic::MockBlockingOpen => "mock_blocking_open",
                        super::model::Intrinsic::Inspect => "inspect",
                        super::model::Intrinsic::TryAdd => "try_add",
                        super::model::Intrinsic::MockUnknown => "mock_unknown",
                        super::model::Intrinsic::MockReadRecords => "mock_read_records",
                        super::model::Intrinsic::MockWriteI64 => "mock_write_i64",
                        super::model::Intrinsic::MockRequest => "mock_request",
                        super::model::Intrinsic::MockReply => "mock_reply",
                    };
                    if *intrinsic == super::model::Intrinsic::DropData {
                        let arg = args.first().copied();
                        if let Some(local) = drop_sources.get(&ins.origin.0) {
                            let local_ty = &f.locals[local.0].ty;
                            (
                                resource_effect(local_ty),
                                0,
                                CauseOperation::ValueDrop,
                                format!("binding:{}", f.locals[local.0].name_span.start),
                            )
                        } else {
                            let ty =
                                arg.and_then(|id| temp_types.get(id.0).and_then(Option::as_ref));
                            let variant =
                                arg.and_then(|id| temp_variants.get(id.0).copied().flatten());
                            (
                                ty.map_or(0, |ty| variant_effect(ty, variant)),
                                0,
                                CauseOperation::ValueDrop,
                                "temporary".to_owned(),
                            )
                        }
                    } else {
                        (
                            known,
                            unknown,
                            CauseOperation::IntrinsicCall,
                            format!("prelude:{name}"),
                        )
                    }
                }
                Op::DropBinding(local) => (
                    resource_effect(&f.locals[local.0].ty),
                    0,
                    CauseOperation::ValueDrop,
                    format!("binding:{}", f.locals[local.0].name_span.start),
                ),
                Op::DropLocal(local) => (
                    if exit_sites
                        .get(&(block.id.0, index))
                        .is_some_and(|exit| exit.live_locals.contains(local))
                    {
                        resource_effect(&f.locals[local.0].ty)
                    } else {
                        0
                    },
                    0,
                    CauseOperation::ResourceDrop,
                    f.locals[local.0].ty.ty_name(),
                ),
                Op::DropValue(temp) => {
                    let effect = temp_types
                        .get(temp.0)
                        .and_then(Option::as_ref)
                        .map_or(0, |ty| {
                            variant_effect(ty, temp_variants.get(temp.0).copied().flatten())
                        });
                    (
                        effect,
                        0,
                        CauseOperation::ValueDrop,
                        format!("temporary:{}:0", ins.span.start),
                    )
                }
                _ => (0, 0, CauseOperation::IntrinsicCall, String::new()),
            };
            if known | unknown != 0 {
                let mut chain = Vec::new();
                let mut related = Vec::new();
                for (bit, _, _) in EFFECTS {
                    if known & bit != 0 || unknown & bit != 0 {
                        if let Op::CallIntrinsic {
                            intrinsic: super::model::Intrinsic::DropData,
                            args,
                        } = &ins.op
                        {
                            let arg = args.first().copied();
                            if let Some(local) = drop_sources.get(&ins.origin.0).copied() {
                                chain.push(Cause {
                                    kind: CauseKind::Drop,
                                    operation: CauseOperation::ValueDrop,
                                    span: Some(ins.span.clone()),
                                    entity: Some(local_entity(f, local)),
                                    evidence: Evidence::Verified,
                                });
                                for resource in resource_types(&f.locals[local.0].ty, None) {
                                    chain.push(Cause {
                                        kind: CauseKind::Drop,
                                        operation: CauseOperation::ResourceDrop,
                                        span: None,
                                        entity: Some(Entity {
                                            kind: EntityKind::Type,
                                            id: resource.into(),
                                        }),
                                        evidence: Evidence::Verified,
                                    });
                                }
                            } else {
                                let ty = arg
                                    .and_then(|id| temp_types.get(id.0).and_then(Option::as_ref));
                                let variant =
                                    arg.and_then(|id| temp_variants.get(id.0).copied().flatten());
                                if let Some(ty) = ty {
                                    let temp_span = arg.and_then(|id| {
                                        temp_spans.get(id.0).and_then(Option::as_ref)
                                    });
                                    chain.push(Cause {
                                        kind: CauseKind::Drop,
                                        operation: CauseOperation::ValueDrop,
                                        span: Some(ins.span.clone()),
                                        entity: Some(Entity {
                                            kind: EntityKind::Temporary,
                                            id: format!(
                                                "temporary:{}:0",
                                                temp_span.map_or(ins.span.start, |span| span.start)
                                            ),
                                        }),
                                        evidence: Evidence::Verified,
                                    });
                                    for resource in resource_types(ty, variant) {
                                        chain.push(Cause {
                                            kind: CauseKind::Drop,
                                            operation: CauseOperation::ResourceDrop,
                                            span: None,
                                            entity: Some(Entity {
                                                kind: EntityKind::Type,
                                                id: resource.into(),
                                            }),
                                            evidence: Evidence::Verified,
                                        });
                                    }
                                }
                            }
                        } else if let Op::DropValue(temp) = ins.op {
                            if temp_types.get(temp.0).and_then(Option::as_ref).is_some() {
                                let span = temp_spans.get(temp.0).and_then(Option::as_ref).cloned();
                                chain.push(Cause {
                                    kind: CauseKind::Drop,
                                    operation: CauseOperation::ValueDrop,
                                    span,
                                    entity: Some(Entity {
                                        kind: EntityKind::Temporary,
                                        id: format!(
                                            "temporary:{}:0",
                                            temp_spans[temp.0]
                                                .as_ref()
                                                .map_or(ins.span.start, |s| s.start)
                                        ),
                                    }),
                                    evidence: Evidence::Verified,
                                });
                            }
                        } else if let Some(local) = drop_sources.get(&ins.origin.0).copied() {
                            chain.push(Cause {
                                kind: CauseKind::Drop,
                                operation: CauseOperation::ValueDrop,
                                span: Some(ins.span.clone()),
                                entity: Some(local_entity(f, local)),
                                evidence: Evidence::Verified,
                            });
                            for resource in resource_types(&f.locals[local.0].ty, None) {
                                chain.push(Cause {
                                    kind: CauseKind::Drop,
                                    operation: CauseOperation::ResourceDrop,
                                    span: None,
                                    entity: Some(Entity {
                                        kind: EntityKind::Type,
                                        id: resource.into(),
                                    }),
                                    evidence: Evidence::Verified,
                                });
                            }
                        } else if let Op::DropBinding(local) = ins.op {
                            chain.push(Cause {
                                kind: CauseKind::Drop,
                                operation: CauseOperation::ValueDrop,
                                span: Some(ins.span.clone()),
                                entity: Some(local_entity(f, local)),
                                evidence: Evidence::Verified,
                            });
                            for resource in resource_types(&f.locals[local.0].ty, None) {
                                chain.push(Cause {
                                    kind: CauseKind::Drop,
                                    operation: CauseOperation::ResourceDrop,
                                    span: None,
                                    entity: Some(Entity {
                                        kind: EntityKind::Type,
                                        id: resource.into(),
                                    }),
                                    evidence: Evidence::Verified,
                                });
                            }
                        } else if let Op::DropLocal(local) = ins.op {
                            related.push(RelatedSpan {
                                role: RelatedRole::Origin,
                                span: f.locals[local.0].name_span.clone(),
                            });
                            related.push(RelatedSpan {
                                role: RelatedRole::Exit,
                                span: ins.span.clone(),
                            });
                            chain.push(Cause {
                                kind: CauseKind::Drop,
                                operation: CauseOperation::ValueDrop,
                                span: Some(f.locals[local.0].name_span.clone()),
                                entity: Some(local_entity(f, local)),
                                evidence: Evidence::Verified,
                            });
                            for resource in resource_types(&f.locals[local.0].ty, None) {
                                chain.push(Cause {
                                    kind: CauseKind::Drop,
                                    operation: CauseOperation::ResourceDrop,
                                    span: None,
                                    entity: Some(Entity {
                                        kind: EntityKind::Type,
                                        id: resource.into(),
                                    }),
                                    evidence: Evidence::Verified,
                                });
                            }
                        } else {
                            chain.push(Cause {
                                kind: if matches!(operation, CauseOperation::BuildArray) {
                                    CauseKind::Effect
                                } else if matches!(operation, CauseOperation::ValueCopy) {
                                    CauseKind::Copy
                                } else if matches!(
                                    operation,
                                    CauseOperation::ValueDrop | CauseOperation::ResourceDrop
                                ) {
                                    CauseKind::Drop
                                } else {
                                    CauseKind::Call
                                },
                                operation,
                                span: if matches!(operation, CauseOperation::ResourceDrop) {
                                    None
                                } else if matches!(operation, CauseOperation::IntrinsicCall) {
                                    Some(call_head(&ins.span, &f.source_text))
                                } else {
                                    Some(ins.span.clone())
                                },
                                entity: Some(Entity {
                                    kind: if matches!(
                                        operation,
                                        CauseOperation::ResourceDrop | CauseOperation::BuildArray
                                    ) {
                                        EntityKind::Type
                                    } else if matches!(operation, CauseOperation::ValueCopy) {
                                        if entity.starts_with("parameter:") {
                                            EntityKind::Parameter
                                        } else {
                                            EntityKind::Binding
                                        }
                                    } else {
                                        EntityKind::Intrinsic
                                    },
                                    id: entity.clone(),
                                }),
                                evidence: if unknown & bit != 0 {
                                    Evidence::Unknown
                                } else {
                                    Evidence::Verified
                                },
                            });
                        }
                    }
                }
                let primary = if matches!(
                    operation,
                    CauseOperation::IntrinsicCall | CauseOperation::ValueDrop
                ) || matches!(ins.op, Op::DropBinding(_) | Op::DropValue(_))
                {
                    call_head(&ins.span, &f.source_text)
                } else if let Op::DropBinding(local) | Op::DropLocal(local) = ins.op {
                    f.locals[local.0].name_span.clone()
                } else {
                    ins.span.clone()
                };
                let mut unique_chain = Vec::new();
                for cause in chain {
                    if !unique_chain.contains(&cause) {
                        unique_chain.push(cause);
                    }
                }
                let chain = unique_chain;
                related.sort_by(|a, b| {
                    (&a.span.file, a.span.start, a.span.end, a.role.as_str()).cmp(&(
                        &b.span.file,
                        b.span.start,
                        b.span.end,
                        b.role.as_str(),
                    ))
                });
                related.dedup();
                out.push(Witness {
                    summary: EffectSummary { known, unknown },
                    primary,
                    operation,
                    chain,
                    related,
                });
            }
        }
    }
    for block in std::iter::once(&f.block).chain(&f.blocks) {
        for ins in &block.instructions {
            let Op::CallDirect {
                args, signature, ..
            } = &ins.op
            else {
                continue;
            };
            for (slot, temp) in args {
                if signature
                    .get(*slot)
                    .is_none_or(|(_, mode, _)| *mode != super::model::ParameterMode::Read)
                    || !owned_temps.get(temp.0).copied().unwrap_or(false)
                {
                    continue;
                }
                let Some(ty) = temp_types.get(temp.0).and_then(Option::as_ref) else {
                    continue;
                };
                let variant = temp_variants.get(temp.0).copied().flatten();
                let effect = variant_effect(ty, variant);
                let Some(span) = temp_spans.get(temp.0).and_then(Option::as_ref) else {
                    continue;
                };
                if effect == 0 {
                    continue;
                }
                out.push(Witness {
                    summary: EffectSummary {
                        known: effect,
                        unknown: 0,
                    },
                    primary: call_head(span, &f.source_text),
                    operation: CauseOperation::ValueDrop,
                    chain: vec![Cause {
                        kind: CauseKind::Drop,
                        operation: CauseOperation::ValueDrop,
                        span: Some(span.clone()),
                        entity: Some(Entity {
                            kind: EntityKind::Temporary,
                            id: format!("temporary:{}:0", span.start),
                        }),
                        evidence: Evidence::Verified,
                    }],
                    related: Vec::new(),
                });
            }
        }
    }
    let blocks_by_id: HashMap<_, _> = std::iter::once(&f.block)
        .chain(&f.blocks)
        .map(|block| (block.id.0, block))
        .collect();
    for exit in f.exit_info.iter().filter(|e| {
        matches!(
            e.class,
            super::model::ExitClass::Try | super::model::ExitClass::Fail
        ) || blocks_by_id
            .get(&e.block.0)
            .is_some_and(|block| e.index == block.instructions.len())
    }) {
        super::count_cfg_work(1);
        for local in &exit.live_locals {
            if exit.definitely_dropped.contains(local) {
                continue;
            }
            let ty = &f.locals[local.0].ty;
            let effect = resource_effect(ty);
            if effect == 0 {
                continue;
            }
            let mut related = vec![
                RelatedSpan {
                    role: RelatedRole::Origin,
                    span: f.locals[local.0].name_span.clone(),
                },
                RelatedSpan {
                    role: RelatedRole::Exit,
                    span: exit.span.clone(),
                },
            ];
            related.sort_by(|a, b| {
                (&a.span.file, a.span.start, a.span.end, a.role.as_str()).cmp(&(
                    &b.span.file,
                    b.span.start,
                    b.span.end,
                    b.role.as_str(),
                ))
            });
            let mut chain = vec![Cause {
                kind: CauseKind::Drop,
                operation: CauseOperation::ValueDrop,
                span: Some(f.locals[local.0].name_span.clone()),
                entity: Some(local_entity(f, *local)),
                evidence: Evidence::Verified,
            }];
            chain.extend(resource_types(ty, None).into_iter().map(|resource| Cause {
                kind: CauseKind::Drop,
                operation: CauseOperation::ResourceDrop,
                span: None,
                entity: Some(Entity {
                    kind: EntityKind::Type,
                    id: resource.into(),
                }),
                evidence: Evidence::Verified,
            }));
            out.push(Witness {
                summary: EffectSummary {
                    known: effect,
                    unknown: 0,
                },
                primary: f.locals[local.0].name_span.clone(),
                operation: CauseOperation::ValueDrop,
                chain,
                related,
            });
        }
    }
    out
}

trait LocalTypeName {
    fn ty_name(&self) -> String;
}
impl LocalTypeName for Ty {
    fn ty_name(&self) -> String {
        match self {
            Ty::MockResource => "MockResource",
            Ty::MockBlockingResource => "MockBlockingResource",
            _ => "value",
        }
        .to_owned()
    }
}

fn add(a: EffectSummary, b: EffectSummary) -> EffectSummary {
    EffectSummary {
        known: a.known | b.known,
        unknown: a.unknown | b.unknown,
    }
}

#[cfg(test)]
fn call_components(program: &Program) -> Vec<Vec<usize>> {
    let rank: Vec<_> = (0..program.functions.functions().len()).collect();
    call_components_by_rank(program, &rank)
}

fn call_components_by_rank(program: &Program, rank: &[usize]) -> Vec<Vec<usize>> {
    let functions = program.functions.functions();
    let indices: HashMap<_, _> = functions
        .iter()
        .enumerate()
        .map(|(index, function)| (function.id.clone(), index))
        .collect();
    let mut edges = vec![Vec::new(); functions.len()];
    for (index, function) in functions.iter().enumerate() {
        for block in std::iter::once(&function.block).chain(&function.blocks) {
            for instruction in &block.instructions {
                if let Op::CallDirect { function, .. } = &instruction.op
                    && let Some(target) = indices.get(function).copied()
                {
                    edges[index].push(target);
                }
            }
        }
        edges[index].sort_by_key(|target| rank[*target]);
        edges[index].dedup();
    }
    let mut reverse = vec![Vec::new(); functions.len()];
    for (source, targets) in edges.iter().enumerate() {
        for target in targets {
            reverse[*target].push(source);
        }
    }
    for callers in &mut reverse {
        callers.sort_by_key(|source| rank[*source]);
    }

    let mut seen = vec![false; functions.len()];
    let mut finish = Vec::with_capacity(functions.len());
    let mut roots: Vec<_> = (0..functions.len()).collect();
    roots.sort_by_key(|index| rank[*index]);
    for root in roots {
        if seen[root] {
            continue;
        }
        let mut stack = vec![(root, false)];
        while let Some((node, closing)) = stack.pop() {
            if closing {
                finish.push(node);
                continue;
            }
            if std::mem::replace(&mut seen[node], true) {
                continue;
            }
            stack.push((node, true));
            for target in edges[node].iter().rev() {
                if !seen[*target] {
                    stack.push((*target, false));
                }
            }
        }
    }

    seen.fill(false);
    let mut components = Vec::new();
    // In this order, a callee component precedes every caller component that
    // depends on it; recursion is confined to one component.
    for root in finish.into_iter().rev() {
        if std::mem::replace(&mut seen[root], true) {
            continue;
        }
        let mut component = Vec::new();
        let mut stack = vec![root];
        while let Some(node) = stack.pop() {
            component.push(node);
            for caller in reverse[node].iter().rev() {
                if !std::mem::replace(&mut seen[*caller], true) {
                    stack.push(*caller);
                }
            }
        }
        component.sort_by_key(|index| rank[*index]);
        components.push(component);
    }
    components.reverse();
    components
}

pub(super) fn analyze(
    program: &mut Program,
    ast: &Ast,
    effect_limit: u64,
    profile_id: &str,
) -> Result<(), Vec<crate::diagnostic::Diagnostic>> {
    let source_functions: HashMap<_, _> = ast
        .functions
        .iter()
        .map(|source| (source.name.as_str(), source))
        .collect();
    let source_ranks: HashMap<_, _> = ast
        .functions
        .iter()
        .enumerate()
        .map(|(index, source)| (source.name.as_str(), index))
        .collect();
    let mut local = Vec::new();
    for f in program.functions.functions() {
        local.push(function_intrinsic_effect(f));
    }
    let functions = program.functions.functions();
    let indices: HashMap<_, _> = functions
        .iter()
        .enumerate()
        .map(|(index, function)| (function.id.clone(), index))
        .collect();
    let source_rank: Vec<_> = functions
        .iter()
        .map(|function| {
            source_ranks
                .get(function.name.as_str())
                .copied()
                .unwrap_or(usize::MAX)
        })
        .collect();
    let mut summaries = vec![EffectSummary::default(); local.len()];
    let mut effect_steps = 0;
    for component in call_components_by_rank(program, &source_rank) {
        let members: HashSet<_> = component.iter().copied().collect();
        loop {
            let old = summaries.clone();
            let mut next = old.clone();
            for index in &component {
                let function = &functions[*index];
                let mut summary = local[*index]
                    .iter()
                    .fold(EffectSummary::default(), |sum, witness| {
                        add(sum, witness.summary)
                    });
                for block in std::iter::once(&function.block).chain(&function.blocks) {
                    for instruction in &block.instructions {
                        charge_effect(
                            &mut effect_steps,
                            &instruction.span,
                            effect_limit,
                            profile_id,
                        )
                        .map_err(|error| vec![*error])?;
                        if let Op::CallDirect { function, .. } = &instruction.op
                            && let Some(target) = indices.get(function).copied()
                        {
                            charge_effect(
                                &mut effect_steps,
                                &instruction.span,
                                effect_limit,
                                profile_id,
                            )
                            .map_err(|error| vec![*error])?;
                            summary = add(
                                summary,
                                if members.contains(&target) {
                                    old[target]
                                } else {
                                    summaries[target]
                                },
                            );
                        }
                    }
                }
                next[*index] = summary;
            }
            if component.iter().all(|index| next[*index] == old[*index]) {
                summaries = next;
                break;
            }
            summaries = next;
        }
    }
    for (f, summary) in program.functions.entries.iter_mut().zip(&summaries) {
        f.effects = Some(*summary);
    }

    let mut errors = Vec::new();
    for (index, f) in program.functions.functions().iter().enumerate() {
        let Some(source_function) = source_functions.get(f.name.as_str()).copied() else {
            continue;
        };
        let restrictions: Vec<_> = match source_function.with_clause.as_ref() {
            Some(WithClause::Simple(items)) => items.iter().collect(),
            Some(WithClause::Operations(ops)) => ops
                .iter()
                .filter(|op| op.name == "call")
                .flat_map(|op| op.restrictions.iter())
                .collect(),
            None => Vec::new(),
        };
        for restriction in restrictions {
            if restriction.name == "unrestricted" {
                continue;
            }
            let required = restriction.name.strip_prefix("no_").unwrap_or_default();
            let Some((bit, effect, code)) =
                EFFECTS.iter().find(|(_, effect, _)| *effect == required)
            else {
                continue;
            };
            let summary = summaries[index];
            if (summary.known | summary.unknown) & bit == 0 {
                continue;
            }
            let unknown = summary.known & bit == 0;
            let witness = find_witness(program, ast, &local, index, *bit);
            let mut d = crate::check::names::diagnostic(
                code,
                None,
                Stage::Effects,
                Operation::FunctionCall,
                restriction.span.clone(),
            );
            if let Some(w) = &witness {
                d.primary_span = Some(w.primary.clone());
                d.cause_chain = w.chain.clone();
                d.related_spans.extend(w.related.clone());
            }
            d.related_spans.push(RelatedSpan {
                role: RelatedRole::Contract,
                span: restriction.span.clone(),
            });
            d.data.insert(
                "effect_model".into(),
                DataValue::String(
                    if profile_id.starts_with("f0-0003-") {
                        "f0-0003-sim-v1"
                    } else {
                        "f0-0002-sim-v1"
                    }
                    .into(),
                ),
            );
            d.data.insert(
                "contract_origin".into(),
                DataValue::Object(
                    [
                        (
                            "file".into(),
                            DataValue::String(restriction.span.file.clone()),
                        ),
                        ("start".into(), DataValue::UInt(restriction.span.start)),
                        ("end".into(), DataValue::UInt(restriction.span.end)),
                    ]
                    .into(),
                ),
            );
            d.data.insert(
                "required_effect".into(),
                DataValue::String((*effect).into()),
            );
            d.data.insert(
                "effect_evidence".into(),
                DataValue::String(
                    if unknown {
                        "unknown_effect"
                    } else {
                        "known_effect"
                    }
                    .into(),
                ),
            );
            d.message = format!("La función {} infringe no_{}.", f.name, effect);
            d.operation = witness.map_or(Operation::FunctionCall, |w| match w.operation {
                CauseOperation::BuildArray => Operation::BuildArray,
                CauseOperation::ValueCopy => Operation::ValueCopy,
                CauseOperation::ValueDrop | CauseOperation::ResourceDrop => Operation::ValueDrop,
                CauseOperation::FunctionCall => Operation::FunctionCall,
                _ => Operation::IntrinsicCall,
            });
            d.related_spans.sort_by(|a, b| {
                (&a.span.file, a.span.start, a.span.end, a.role.as_str()).cmp(&(
                    &b.span.file,
                    b.span.start,
                    b.span.end,
                    b.role.as_str(),
                ))
            });
            errors.push(*d);
        }
    }
    errors.sort_by(|a, b| {
        let ka = a
            .primary_span
            .as_ref()
            .map(|s| (s.file.as_str(), s.start, s.end));
        let kb = b
            .primary_span
            .as_ref()
            .map(|s| (s.file.as_str(), s.start, s.end));
        ka.cmp(&kb)
            .then_with(|| a.code.cmp(&b.code))
            .then_with(|| a.subcause.cmp(&b.subcause))
    });
    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

pub(super) fn verify_metadata(program: &Program) -> Result<(), &'static str> {
    let functions = program.functions.functions();
    let functions_by_id: HashMap<_, _> = functions.iter().map(|f| (f.id.clone(), f)).collect();
    let local: Vec<_> = functions
        .iter()
        .map(|f| {
            function_intrinsic_effect(f)
                .into_iter()
                .fold(EffectSummary::default(), |s, w| add(s, w.summary))
        })
        .collect();
    for (index, f) in functions.iter().enumerate() {
        let summary = f.effects.ok_or("effect summary missing")?;
        if (summary.known | summary.unknown) & !ALL != 0 {
            return Err("effect summary bits");
        }
        if local[index].known & !summary.known != 0 || local[index].unknown & !summary.unknown != 0
        {
            return Err("effect summary local inclusion");
        }
        for block in std::iter::once(&f.block).chain(&f.blocks) {
            for ins in &block.instructions {
                if let Op::CallDirect { function, .. } = &ins.op {
                    let callee = functions_by_id.get(function).ok_or("unknown callee")?;
                    let callee_summary = callee.effects.ok_or("effect summary missing")?;
                    if callee_summary.known & !summary.known != 0
                        || callee_summary.unknown & !summary.unknown != 0
                    {
                        return Err("effect summary call inclusion");
                    }
                }
            }
        }
        let blocks_by_id: HashMap<_, _> = std::iter::once(&f.block)
            .chain(&f.blocks)
            .map(|block| (block.id.0, block))
            .collect();
        let mut sites = HashSet::new();
        let mut expected_sites = HashSet::new();
        for block in std::iter::once(&f.block).chain(&f.blocks) {
            for (index, ins) in block.instructions.iter().enumerate() {
                let prior_same_exit = index > 0
                    && matches!(block.instructions[index - 1].op, Op::DropLocal(_))
                    && block.instructions[index - 1].span == ins.span;
                if (matches!(ins.op, Op::DropLocal(_)) && !prior_same_exit)
                    || matches!(
                        ins.op,
                        Op::BranchVariant { .. }
                            | Op::CallIntrinsic {
                                intrinsic: super::model::Intrinsic::Fail,
                                ..
                            }
                    )
                {
                    expected_sites.insert((block.id.0, index));
                }
            }
            if matches!(block.terminator, super::model::Terminator::Return { .. }) {
                expected_sites.insert((block.id.0, block.instructions.len()));
            }
        }
        for exit in &f.exit_info {
            super::count_cfg_work(1);
            let block = blocks_by_id
                .get(&exit.block.0)
                .copied()
                .ok_or("exit block")?;
            if exit.index > block.instructions.len() || !sites.insert((exit.block.0, exit.index)) {
                return Err("exit site");
            }
            if exit
                .live_locals
                .iter()
                .chain(&exit.definitely_dropped)
                .any(|id| id.0 >= f.locals.len())
            {
                return Err("exit local");
            }
            if exit
                .live_locals
                .iter()
                .any(|id| exit.definitely_dropped.contains(id))
            {
                return Err("exit ownership sets overlap");
            }
            let anchored = if exit.index < block.instructions.len() {
                let instruction = &block.instructions[exit.index];
                let expected_class = match &instruction.op {
                    Op::DropLocal(_) => Some(super::model::ExitClass::Fallthrough),
                    Op::BranchVariant { .. } => Some(super::model::ExitClass::Try),
                    Op::CallIntrinsic {
                        intrinsic: super::model::Intrinsic::Fail,
                        ..
                    } => Some(super::model::ExitClass::Fail),
                    _ => None,
                };
                instruction.span == exit.span && expected_class == Some(exit.class)
            } else {
                match &block.terminator {
                    super::model::Terminator::Return { span, .. } => match exit.class {
                        super::model::ExitClass::Fallthrough => true,
                        super::model::ExitClass::Return => {
                            span.start <= exit.span.start && exit.span.end <= span.end
                        }
                        super::model::ExitClass::Try | super::model::ExitClass::Fail => false,
                    },
                    _ => false,
                }
            };
            if !anchored {
                return Err("exit anchor");
            }
            if exit.class == super::model::ExitClass::Fallthrough
                && (exit.span.start >= f.source_text.len() as u64
                    || f.source_text.as_bytes().get(exit.span.start as usize) != Some(&b'}'))
            {
                return Err("exit close span");
            }
        }
        if sites != expected_sites {
            return Err("exit coverage");
        }
    }
    Ok(())
}

pub(super) fn populate_exit_info(
    function: &mut Function,
    source_function: &crate::frontend::parse::Function,
) {
    use super::model::{ExitClass, ExitInfo, Terminator};
    let drop_sources = explicit_drop_sources(function);
    let blocks: Vec<_> = std::iter::once(&function.block)
        .chain(&function.blocks)
        .collect();
    let count = blocks.len();
    let source_returns = return_spans(&source_function.body, &function.source_text);
    super::count_cfg_work(source_returns.len() as u64);
    let tracked: HashSet<_> = function
        .locals
        .iter()
        .filter(|l| {
            matches!(
                l.ty,
                Ty::Array(_) | Ty::MockResource | Ty::MockBlockingResource | Ty::Result(_, _)
            )
        })
        .map(|l| l.id.0)
        .collect();
    let word_count = function.locals.len().div_ceil(64);
    let empty_bits = || vec![0_u64; word_count];
    let mut may_in = if tracked.is_empty() {
        Vec::new()
    } else {
        vec![empty_bits(); count]
    };
    let mut gone_in = if tracked.is_empty() {
        Vec::new()
    } else {
        may_in.clone()
    };
    if !tracked.is_empty() {
        let mut preds = vec![Vec::<usize>::new(); count];
        let mut successors = vec![Vec::<usize>::new(); count];
        for (i, block) in blocks.iter().enumerate() {
            let mut add = |id: super::model::BlockId| {
                if id.0 < count {
                    preds[id.0].push(i);
                    successors[i].push(id.0);
                }
            };
            match block.terminator {
                Terminator::Jump { target, .. } => add(target),
                Terminator::BranchBool { yes, no, .. } => {
                    add(yes);
                    add(no);
                }
                Terminator::Return { .. } => {}
            }
        }
        let mut may_out = may_in.clone();
        let mut gone_out = may_in.clone();
        for parameter in &function.parameters {
            if parameter.mode == super::model::ParameterMode::Consume
                && tracked.contains(&parameter.local.0)
            {
                insert_local_bit(&mut may_in[0], parameter.local.0);
            }
        }
        let mut work = VecDeque::from_iter(0..count);
        let mut queued = vec![true; count];
        while let Some(bi) = work.pop_front() {
            queued[bi] = false;
            if bi != 0 {
                let mut next_may = empty_bits();
                for predecessor in &preds[bi] {
                    for (next, previous) in next_may.iter_mut().zip(&may_out[*predecessor]) {
                        *next |= previous;
                    }
                }
                let mut next_gone = preds[bi]
                    .first()
                    .map_or_else(empty_bits, |first| gone_out[*first].clone());
                for predecessor in preds[bi].iter().skip(1) {
                    for (next, previous) in next_gone.iter_mut().zip(&gone_out[*predecessor]) {
                        *next &= previous;
                    }
                }
                may_in[bi] = next_may;
                gone_in[bi] = next_gone;
            }
            let mut may = may_in[bi].clone();
            let mut gone = gone_in[bi].clone();
            for ins in &blocks[bi].instructions {
                match ins.op {
                    Op::WritePlace { local, .. } | Op::WriteIndex { local, .. }
                        if tracked.contains(&local.0) =>
                    {
                        insert_local_bit(&mut may, local.0);
                        remove_local_bit(&mut gone, local.0);
                    }
                    Op::TransferValue(local) if tracked.contains(&local.0) => {
                        remove_local_bit(&mut may, local.0);
                    }
                    Op::DropBinding(local) | Op::DropLocal(local) if tracked.contains(&local.0) => {
                        remove_local_bit(&mut may, local.0);
                        insert_local_bit(&mut gone, local.0);
                    }
                    Op::CallIntrinsic { .. }
                        if drop_sources
                            .get(&ins.origin.0)
                            .is_some_and(|local| tracked.contains(&local.0)) =>
                    {
                        let local = drop_sources[&ins.origin.0];
                        remove_local_bit(&mut may, local.0);
                        insert_local_bit(&mut gone, local.0);
                    }
                    _ => {}
                }
            }
            let out_changed = may != may_out[bi] || gone != gone_out[bi];
            may_out[bi] = may;
            gone_out[bi] = gone;
            if out_changed {
                for successor in &successors[bi] {
                    if !queued[*successor] {
                        queued[*successor] = true;
                        work.push_back(*successor);
                    }
                }
            }
        }
    }
    let mut records = Vec::new();
    let mut next_source_return = 0usize;
    for (bi, block) in blocks.iter().enumerate() {
        let mut may = if tracked.is_empty() {
            Vec::new()
        } else {
            may_in[bi].clone()
        };
        let mut gone = if tracked.is_empty() {
            Vec::new()
        } else {
            gone_in[bi].clone()
        };
        for (ii, ins) in block.instructions.iter().enumerate() {
            let prior_same_exit = ii > 0
                && matches!(block.instructions[ii - 1].op, Op::DropLocal(_))
                && block.instructions[ii - 1].span == ins.span;
            let class = match ins.op {
                Op::DropLocal(_) if !prior_same_exit => Some(ExitClass::Fallthrough),
                Op::DropLocal(_) => None,
                Op::BranchVariant { .. } => Some(ExitClass::Try),
                Op::CallIntrinsic {
                    intrinsic: super::model::Intrinsic::Fail,
                    ..
                } => Some(ExitClass::Fail),
                _ => None,
            };
            if let Some(class) = class {
                records.push(ExitInfo {
                    class,
                    block: block.id,
                    index: ii,
                    span: ins.span.clone(),
                    live_locals: ordered_local_ids(&may),
                    definitely_dropped: ordered_local_ids(&gone),
                });
            }
            match ins.op {
                Op::WritePlace { local, .. } | Op::WriteIndex { local, .. }
                    if tracked.contains(&local.0) =>
                {
                    insert_local_bit(&mut may, local.0);
                    remove_local_bit(&mut gone, local.0);
                }
                Op::TransferValue(local) if tracked.contains(&local.0) => {
                    remove_local_bit(&mut may, local.0);
                }
                Op::DropBinding(local) | Op::DropLocal(local) if tracked.contains(&local.0) => {
                    remove_local_bit(&mut may, local.0);
                    insert_local_bit(&mut gone, local.0);
                }
                Op::CallIntrinsic { .. }
                    if drop_sources
                        .get(&ins.origin.0)
                        .is_some_and(|local| tracked.contains(&local.0)) =>
                {
                    let local = drop_sources[&ins.origin.0];
                    remove_local_bit(&mut may, local.0);
                    insert_local_bit(&mut gone, local.0);
                }
                _ => {}
            }
        }
        if let Terminator::Return { span, .. } = &block.terminator {
            super::count_cfg_work(1);
            while source_returns
                .get(next_source_return)
                .is_some_and(|candidate| candidate.start < span.start)
            {
                next_source_return += 1;
            }
            let explicit = source_returns
                .get(next_source_return)
                .filter(|candidate| span.start <= candidate.start && candidate.end <= span.end)
                .cloned();
            if explicit.is_some() {
                next_source_return += 1;
            }
            let is_implicit = explicit.is_none() && span == &source_function.body.span;
            let exit_span = if let Some(explicit) = explicit {
                explicit
            } else if is_implicit {
                source_function.body.close_span.clone()
            } else {
                span.clone()
            };
            records.push(ExitInfo {
                class: if is_implicit {
                    ExitClass::Fallthrough
                } else {
                    ExitClass::Return
                },
                block: block.id,
                index: block.instructions.len(),
                span: exit_span,
                live_locals: ordered_local_ids(&may),
                definitely_dropped: ordered_local_ids(&gone),
            });
        }
    }
    function.exit_info = records;
}

fn insert_local_bit(bits: &mut [u64], local: usize) {
    bits[local / 64] |= 1 << (local % 64);
}

fn remove_local_bit(bits: &mut [u64], local: usize) {
    bits[local / 64] &= !(1 << (local % 64));
}

fn ordered_local_ids(bits: &[u64]) -> Vec<super::model::LocalId> {
    bits.iter()
        .enumerate()
        .flat_map(|(word_index, word)| {
            let mut remaining = *word;
            std::iter::from_fn(move || {
                if remaining == 0 {
                    None
                } else {
                    let bit = remaining.trailing_zeros() as usize;
                    remaining &= remaining - 1;
                    Some(super::model::LocalId(word_index * 64 + bit))
                }
            })
        })
        .collect()
}

fn return_spans(block: &crate::frontend::parse::Block, source: &str) -> Vec<Span> {
    let mut spans = Vec::new();
    for statement in &block.statements {
        super::count_cfg_work(1);
        spans.extend(return_spans_statement(statement, source));
    }
    spans
}

fn return_spans_statement(statement: &Statement, source: &str) -> Vec<Span> {
    match statement {
        Statement::Return { span, .. } => {
            let mut span = span.clone();
            if span.end > span.start && source.as_bytes().get(span.end as usize - 1) == Some(&b';')
            {
                span.end -= 1;
            }
            vec![span]
        }
        Statement::Block(block) => return_spans(block, source),
        Statement::If {
            then_block,
            otherwise,
            ..
        } => {
            let mut spans = return_spans(then_block, source);
            if let Some(other) = otherwise {
                spans.extend(return_spans_statement(other, source));
            }
            spans
        }
        Statement::While { body, .. } => return_spans(body, source),
        Statement::Let { .. }
        | Statement::Assign { .. }
        | Statement::Value { .. }
        | Statement::Expr { .. } => Vec::new(),
    }
}

#[cfg(test)]
#[allow(clippy::items_after_test_module)]
mod tests {
    use super::*;

    #[test]
    fn effect_budget_accepts_exact_limit_and_rejects_first_excess_without_charge() {
        let span = Span {
            file: "effect_budget.n".into(),
            start: 0,
            end: 1,
        };
        let mut used = 0;
        for _ in 0..32 {
            charge_effect(&mut used, &span, 32, "f0-0002-b-test-effects-32-v2").unwrap();
        }
        assert_eq!(used, 32);
        let rejected = charge_effect(&mut used, &span, 32, "f0-0002-b-test-effects-32-v2")
            .expect_err("first excess effect unit must reject");
        assert_eq!(used, 32);
        assert_eq!(rejected.data.get("observed"), Some(&DataValue::UInt(33)));
    }
    use crate::ir::{CompileError, compile_scalar};

    fn diagnostics(source: &str) -> Vec<crate::diagnostic::Diagnostic> {
        match compile_scalar("effects.n", source) {
            Err(CompileError::Rejected(errors)) => errors.into_iter().map(|d| *d).collect(),
            Ok(_) => Vec::new(),
            other => panic!("expected effect rejection, got {other:?}"),
        }
    }

    #[test]
    fn implicit_droplocal_violates_no_block_at_owner_and_records_cfg_exit() {
        let source = "fn Main() with no_block{if true{let r=MockBlockingOpen(1);}}";
        let errors = diagnostics(source);
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].code, "NX-E0413", "{errors:?}");
        assert_eq!(
            errors[0].primary_span.as_ref().map(|s| (s.start, s.end)),
            Some((36, 37))
        );
        assert_eq!(
            errors[0].data.get("required_effect"),
            Some(&DataValue::String("block".into()))
        );
        errors[0].validate("effects.n", source).unwrap();

        let source = "fn Main() with no_block{let a=[1];let r=MockBlockingOpen(1);}";
        let errors = diagnostics(source);
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].code, "NX-E0413", "{errors:?}");
        let program = compile_scalar(
            "effects.n",
            "fn Main(){let a=[1];let r=MockBlockingOpen(1);}",
        )
        .unwrap();
        let function = program.functions().function("Main").unwrap();
        let array = function
            .locals
            .iter()
            .find(|local| local.name == "a")
            .unwrap();
        let resource = function
            .locals
            .iter()
            .find(|local| local.name == "r")
            .unwrap();
        assert!(array.id.0 < resource.id.0);
        let owner = source.find("r=").unwrap() as u64;
        assert_eq!(
            errors[0]
                .primary_span
                .as_ref()
                .map(|span| (span.start, span.end)),
            Some((owner, owner + 1))
        );
        errors[0].validate("effects.n", source).unwrap();

        let source = "fn Main() with no_block{if true{let a=MockBlockingOpen(1);let b=MockBlockingOpen(2);}}";
        let errors = diagnostics(source);
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].code, "NX-E0413");
        errors[0].validate("effects.n", source).unwrap();

        let source =
            "fn F(a:move Array<I64>,r:move MockBlockingResource) with no_block{} fn Main(){}";
        let errors = diagnostics(source);
        assert_eq!(errors.len(), 1, "{errors:?}");
        assert_eq!(errors[0].code, "NX-E0413");
        let program = compile_scalar(
            "effects.n",
            "fn F(a:move Array<I64>,r:edit MockBlockingResource){} fn Main(){}",
        )
        .unwrap();
        let function = program.functions().function("F").unwrap();
        let scalar = function
            .locals
            .iter()
            .find(|local| local.name == "a")
            .unwrap();
        let resource = function
            .locals
            .iter()
            .find(|local| local.name == "r")
            .unwrap();
        assert!(scalar.id.0 < resource.id.0);
        errors[0].validate("effects.n", source).unwrap();
    }

    #[test]
    fn implicit_droplocals_do_not_mark_non_io_cleanup_as_io() {
        for source in [
            "fn Main(){if true{let r=MockOpen(1);}}",
            "fn Main(){let x=if true then {let r=MockOpen(1);value:1;} else {value:2;};}",
        ] {
            let program = compile_scalar("effects.n", source)
                .unwrap_or_else(|error| panic!("source {source}: {error:?}"));
            let function = program.functions().function("Main").unwrap();
            let drops: Vec<_> = std::iter::once(&function.block)
                .chain(&function.blocks)
                .flat_map(|block| &block.instructions)
                .filter(|instruction| matches!(instruction.op, Op::DropLocal(_)))
                .collect();
            assert_eq!(drops.len(), 1, "source: {source}");
            assert!(!drops[0].effect_io, "source: {source}");
        }
    }

    #[test]
    fn try_exit_reports_the_owner_contract_and_constructed_exit_spans() {
        let source = "fn F()->Result<I64,I64>{return Err(1);} fn Main()->Result<I64,I64> with no_block{let r=MockBlockingOpen(1);let n=F()?;return Ok(n);}";
        let errors = diagnostics(source);
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].code, "NX-E0413");
        assert_eq!(
            errors[0]
                .primary_span
                .as_ref()
                .map(|span| (span.start, span.end)),
            Some((85, 86))
        );
        assert!(errors[0].related_spans.iter().any(|related| {
            related.role == RelatedRole::Contract
                && (related.span.start, related.span.end) == (72, 80)
        }));
        assert!(errors[0].related_spans.iter().any(|related| {
            related.role == RelatedRole::Exit
                && (related.span.start, related.span.end) == (113, 117)
        }));
        errors[0].validate("effects.n", source).unwrap();
    }

    #[test]
    fn unknown_effects_fail_each_written_restriction_in_code_order() {
        let source = "fn Main() with no_io,no_alloc{MockUnknown();}";
        let errors = diagnostics(source);
        assert_eq!(
            errors.iter().map(|d| d.code.as_str()).collect::<Vec<_>>(),
            ["NX-E0412", "NX-E0415"]
        );
        assert!(
            errors.iter().all(|d| d.data.get("effect_evidence")
                == Some(&DataValue::String("unknown_effect".into())))
        );
        assert!(errors.iter().all(|diagnostic| {
            diagnostic
                .cause_chain
                .iter()
                .any(|cause| cause.evidence == Evidence::Unknown)
        }));
        assert_eq!(
            errors[0].primary_span.as_ref().map(|s| (s.start, s.end)),
            Some((30, 41))
        );
        for error in &errors {
            error.validate("effects.n", source).unwrap();
        }

        let source = "fn Main() with no_alloc,no_io{Print(1);}";
        let errors = diagnostics(source);
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].code, "NX-E0415");
        let print = source.find("Print(1)").unwrap() as u64;
        assert_eq!(
            errors[0]
                .primary_span
                .as_ref()
                .map(|span| (span.start, span.end)),
            Some((print, print + 5))
        );
        errors[0].validate("effects.n", source).unwrap();

        let source = "fn Main() with no_io{Print(1);}";
        let errors = diagnostics(source);
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].message, "La función Main infringe no_io.");
    }

    #[test]
    fn known_effect_precedes_shorter_unknown_witness() {
        let source = "fn Hoja(){Print(1);} fn Main() with no_io{MockUnknown();Hoja();}";
        let errors = diagnostics(source);
        assert_eq!(errors[0].code, "NX-E0415");
        let head = source.find("Hoja();").unwrap() as u64;
        assert_eq!(
            errors[0].primary_span.as_ref().map(|s| (s.start, s.end)),
            Some((head, head + 4))
        );
        assert_eq!(
            errors[0].data.get("effect_evidence"),
            Some(&DataValue::String("known_effect".into()))
        );
        assert!(
            errors[0]
                .cause_chain
                .iter()
                .any(|cause| cause.evidence == Evidence::Verified)
        );
        errors[0].validate("effects.n", source).unwrap();
    }

    #[test]
    fn transitive_witness_preserves_call_chain_and_diagnostic_operation() {
        let source = "fn Hoja(){Print(1);} fn Rama(){Hoja();} fn Main() with no_io{Rama();}";
        let errors = diagnostics(source);
        assert_eq!(errors.len(), 1, "{errors:?}");
        assert_eq!(errors[0].code, "NX-E0415");
        assert_eq!(errors[0].operation, Operation::FunctionCall);
        assert_eq!(
            errors[0]
                .cause_chain
                .iter()
                .map(|cause| cause.operation.as_str())
                .collect::<Vec<_>>(),
            ["function.call", "function.call", "intrinsic.call"]
        );
        let call = source.rfind("Rama();").unwrap() as u64;
        assert_eq!(
            errors[0]
                .primary_span
                .as_ref()
                .map(|span| (span.start, span.end)),
            Some((call, call + 4))
        );
        errors[0].validate("effects.n", source).unwrap();
        let expected = [
            ("function.call", 62, 67, "function", "function:24"),
            ("function.call", 32, 36, "function", "function:3"),
            ("intrinsic.call", 10, 15, "intrinsic", "prelude:print"),
        ];
        let literal = "fn Hoja(){Print(1);} fn Medio(){Hoja();} fn Main() with no_io{Medio();}";
        let errors = diagnostics(literal);
        assert_eq!(errors[0].operation, Operation::FunctionCall);
        for (cause, (operation, start, end, kind, id)) in errors[0].cause_chain.iter().zip(expected)
        {
            assert_eq!(cause.operation.as_str(), operation);
            assert_eq!(
                cause.span.as_ref().map(|span| (span.start, span.end)),
                Some((start, end))
            );
            let entity = cause.entity.as_ref().unwrap();
            assert_eq!(entity.kind.as_str(), kind);
            assert_eq!(entity.id, id);
        }

        let source = "fn Loop(){Loop();Leaf();} fn Leaf(){Print(1);} fn Main() with no_io{Loop();}";
        let errors = diagnostics(source);
        assert_eq!(errors.len(), 1, "{errors:?}");
        assert_eq!(errors[0].code, "NX-E0415");
        assert_eq!(
            errors[0]
                .cause_chain
                .iter()
                .map(|cause| cause.operation.as_str())
                .collect::<Vec<_>>(),
            ["function.call", "function.call", "intrinsic.call"]
        );
        let call = source.rfind("Loop();").unwrap() as u64;
        assert_eq!(
            errors[0]
                .primary_span
                .as_ref()
                .map(|span| (span.start, span.end)),
            Some((call, call + 4))
        );
        errors[0].validate("effects.n", source).unwrap();
    }

    #[test]
    fn no_block_counts_owned_temporary_destroyed_after_intrinsic_read() {
        for source in [
            "fn Main() with no_block{Inspect(MockBlockingOpen(1));}",
            "fn Main() with no_block{let r=MockBlockingOpen(1);Inspect(r);}",
            "fn Main() with no_block{MockBlockingOpen(1);}",
            "fn Main() with no_block{Drop(MockBlockingOpen(1));}",
        ] {
            let errors = diagnostics(source);
            assert_eq!(
                errors
                    .iter()
                    .map(|error| error.code.as_str())
                    .collect::<Vec<_>>(),
                ["NX-E0413"],
                "{source}: {errors:?}"
            );
            for error in errors {
                error.validate("effects.n", source).unwrap();
            }
        }
        assert!(
            crate::ir::check_scalar("effects.n", "fn Main(){Inspect(MockBlockingOpen(1));}")
                .is_ok()
        );
    }

    #[test]
    fn intrinsic_cause_chain_does_not_add_a_duplicate_temporary_destructor() {
        let source = "fn Main(r:view MockBlockingResource) with no_io{Inspect(r);}";
        let errors = diagnostics(source);
        assert_eq!(errors.len(), 1, "{errors:?}");
        assert_eq!(errors[0].code, "NX-E0415");
        assert_eq!(
            errors[0]
                .cause_chain
                .iter()
                .map(|cause| cause.operation.as_str())
                .collect::<Vec<_>>(),
            ["intrinsic.call"]
        );
        assert_eq!(
            errors[0].cause_chain[0]
                .entity
                .as_ref()
                .map(|entity| entity.id.as_str()),
            Some("prelude:inspect"),
            "{errors:#?}"
        );
        errors[0].validate("effects.n", source).unwrap();
    }

    #[test]
    fn with_restriction_origin_uses_the_identifier_token_span() {
        let source = "fn Main() with no_block{MockBlockingOpen(1);}";
        let errors = diagnostics(source);
        let start = source.find("no_block").unwrap() as u64;
        let end = start + "no_block".len() as u64;
        let DataValue::Object(origin) = errors[0].data.get("contract_origin").unwrap() else {
            panic!("contract_origin must be an object");
        };
        assert_eq!(origin.get("start"), Some(&DataValue::UInt(start)));
        assert_eq!(origin.get("end"), Some(&DataValue::UInt(end)));
    }

    #[test]
    fn verify_program_checks_effect_metadata_after_summaries_are_attached() {
        let mut program = compile_scalar("effects.n", "fn Main(){Print(1);}").unwrap();
        let main = program
            .functions
            .entries
            .iter_mut()
            .find(|function| function.name == "Main")
            .unwrap();
        main.effects = Some(EffectSummary {
            known: 0,
            unknown: 0,
        });
        assert_eq!(
            super::super::verify::verify_program(&program),
            Err("effect summary local inclusion")
        );
    }

    #[test]
    fn no_alloc_counts_array_copies_from_index_and_result_bindings() {
        for source in [
            "fn K(a:view Array<Array<I64>>)->Array<I64> with no_alloc{return a[0];} fn Main(){}",
            "fn K(a:view Array<Array<I64>>)->Array<I64> with no_alloc{var b=a[0];return b;} fn Main(){}",
            "fn K(r:view Result<Array<I64>,I64>) with no_alloc{let b=r;} fn Main(){}",
            "fn K(a:view Array<Array<I64>>)->Array<I64> with no_alloc{let k=a[0];return k;} fn Main(){}",
        ] {
            let errors = diagnostics(source);
            assert_eq!(
                errors
                    .iter()
                    .map(|error| error.code.as_str())
                    .collect::<Vec<_>>(),
                ["NX-E0412"],
                "{source}: {errors:?}"
            );
            for error in errors {
                error.validate("effects.n", source).unwrap();
            }
        }
        assert!(
            crate::ir::check_scalar(
                "effects.n",
                "fn K(a:view Array<I64>)->I64 with no_alloc{return a[0];} fn Main(){}"
            )
            .is_ok()
        );
    }

    #[test]
    fn canonical_effect_witness_prefers_shorter_path_and_ascending_offsets() {
        for (source, primary) in [
            (
                "fn Hoja(){Print(1);} fn Medio(){Hoja();} fn Main() with no_io{Medio();Print(2);}",
                (70, 75),
            ),
            ("fn Main() with no_io{Print(1);Print(2);}", (21, 26)),
        ] {
            let errors = diagnostics(source);
            assert_eq!(
                errors[0]
                    .primary_span
                    .as_ref()
                    .map(|span| (span.start, span.end)),
                Some(primary),
                "{errors:?}"
            );
        }
    }

    #[test]
    fn witness_search_keeps_small_repeated_call_ladder_canonical() {
        let mut source = String::from("fn Leaf(){Print(1);} ");
        for level in (0..8).rev() {
            let next = if level == 7 {
                "Leaf".to_owned()
            } else {
                format!("L{}", level + 1)
            };
            source.push_str(&format!("fn L{level}(){{{next}();{next}();}} "));
        }
        source.push_str("fn Main() with no_io{L0();}");
        let errors = diagnostics(&source);
        assert_eq!(errors[0].code, "NX-E0415");
        let call = source.rfind("L0();").unwrap() as u64;
        assert_eq!(
            errors[0]
                .primary_span
                .as_ref()
                .map(|span| (span.start, span.end)),
            Some((call, call + 2))
        );
    }

    #[test]
    fn published_effect_fixture_fields_match_the_literal_contract() {
        let source = "fn Copia(a:view Array<I64>)->Array<I64> with no_alloc{return a;} fn Main(){}";
        let errors = diagnostics(source);
        assert_eq!(errors[0].code, "NX-E0412");
        assert_eq!(errors[0].operation, Operation::ValueCopy);
        assert_eq!(
            errors[0]
                .primary_span
                .as_ref()
                .map(|span| (span.start, span.end)),
            Some((61, 62))
        );
        let cause = errors[0].cause_chain.last().unwrap();
        assert_eq!(cause.operation, CauseOperation::ValueCopy);
        assert_eq!(
            cause.span.as_ref().map(|span| (span.start, span.end)),
            Some((61, 62))
        );
        assert_eq!(cause.entity.as_ref().unwrap().kind, EntityKind::Parameter);
        assert_eq!(cause.entity.as_ref().unwrap().id, "parameter:9");

        for (source, span) in [
            ("fn Main() with no_suspend{MockUnknown();}", (26, 37)),
            ("fn Main() with no_io{if false{Print(1);}}", (30, 35)),
        ] {
            let errors = diagnostics(source);
            assert_eq!(
                errors[0]
                    .cause_chain
                    .last()
                    .unwrap()
                    .span
                    .as_ref()
                    .map(|s| (s.start, s.end)),
                Some(span)
            );
            assert_eq!(
                errors[0]
                    .cause_chain
                    .last()
                    .unwrap()
                    .entity
                    .as_ref()
                    .unwrap()
                    .kind,
                EntityKind::Intrinsic
            );
        }
    }

    #[test]
    fn temporary_destructor_and_dead_tail_have_source_regressions() {
        let source = "fn Main() with no_block{MockBlockingOpen(1);}";
        let errors = diagnostics(source);
        assert_eq!(errors[0].code, "NX-E0413");
        assert_eq!(
            errors[0]
                .primary_span
                .as_ref()
                .map(|span| (span.start, span.end)),
            Some((24, 40))
        );
        assert!(
            errors[0]
                .cause_chain
                .iter()
                .all(|cause| cause.operation != CauseOperation::ResourceDrop),
            "implicit temporary destruction has no resource.drop step: {errors:#?}"
        );
        assert!(
            errors[0].cause_chain.iter().any(|cause| {
                cause.operation == CauseOperation::ValueDrop
                    && cause
                        .span
                        .as_ref()
                        .is_some_and(|span| (span.start, span.end) == (24, 43))
                    && cause
                        .entity
                        .as_ref()
                        .is_some_and(|entity| entity.id == "temporary:24:0")
            }),
            "{errors:#?}"
        );
        assert!(
            crate::ir::check_scalar("effects.n", "fn Main() with no_io{return;Print(1);}").is_ok()
        );
    }

    #[test]
    fn borrowed_user_parameter_does_not_own_the_resource() {
        for source in [
            "fn V(r:view MockBlockingResource) with no_block{} fn Main(){}",
            "fn V(r:edit MockBlockingResource) with no_block{} fn Main(){}",
            "fn V(r:view MockResource) with no_io{} fn Main(){}",
            "fn V(r:edit MockResource) with no_io{} fn Main(){}",
        ] {
            assert!(
                diagnostics(source).is_empty(),
                "{source}: {:?}",
                diagnostics(source)
            );
        }

        let source = "fn V(r:move MockBlockingResource) with no_block{} fn Main(){}";
        let errors = diagnostics(source);
        assert_eq!(errors.len(), 1, "{errors:#?}");
        assert_eq!(errors[0].code, "NX-E0413", "{errors:#?}");
        let parameter = source.find("r:").unwrap() as u64;
        assert_eq!(
            errors[0]
                .primary_span
                .as_ref()
                .map(|span| (span.start, span.end)),
            Some((parameter, parameter + 1))
        );
        errors[0].validate("effects.n", source).unwrap();
    }

    #[test]
    fn borrowed_user_call_counts_the_argument_temporary_in_the_caller() {
        let sources = [
            "fn V(r:view MockBlockingResource){} fn Main() with no_block{V(MockBlockingOpen(1));}",
            "fn Main() with no_block{Inspect(MockBlockingOpen(1));}",
        ];
        for source in sources {
            let errors = diagnostics(source);
            assert_eq!(errors.len(), 1, "{source}: {errors:#?}");
            let error = &errors[0];
            assert_eq!(error.code, "NX-E0413", "{source}: {error:#?}");
            assert_eq!(
                error.operation,
                Operation::ValueDrop,
                "{source}: {error:#?}"
            );
            let open = source.find("MockBlockingOpen(1)").unwrap() as u64;
            assert_eq!(
                error
                    .primary_span
                    .as_ref()
                    .map(|span| (span.start, span.end)),
                Some((open, open + "MockBlockingOpen".len() as u64)),
                "{source}: {error:#?}"
            );
            assert!(
                error.related_spans.iter().any(|related| {
                    related.role == RelatedRole::Contract
                        && related.span.start == source.find("no_block").unwrap() as u64
                        && related.span.end == related.span.start + "no_block".len() as u64
                }),
                "{source}: {error:#?}"
            );
            assert_eq!(
                error
                    .cause_chain
                    .iter()
                    .map(|cause| cause.operation.as_str())
                    .collect::<Vec<_>>(),
                ["value.drop"],
                "{source}: {error:#?}"
            );
            assert_eq!(
                error.cause_chain[0]
                    .span
                    .as_ref()
                    .map(|span| (span.start, span.end)),
                Some((open, open + "MockBlockingOpen(1)".len() as u64)),
                "{source}: {error:#?}"
            );
            let entity = error.cause_chain[0].entity.as_ref().unwrap();
            assert_eq!(entity.kind, EntityKind::Temporary, "{source}: {error:#?}");
            assert_eq!(
                entity.id,
                format!("temporary:{open}:0"),
                "{source}: {error:#?}"
            );
            assert_eq!(
                error.cause_chain[0].kind,
                CauseKind::Drop,
                "{source}: {error:#?}"
            );
            assert_eq!(
                error.cause_chain[0].evidence,
                Evidence::Verified,
                "{source}: {error:#?}"
            );
            assert_eq!(
                error.data.get("required_effect"),
                Some(&DataValue::String("block".into())),
                "{source}: {error:#?}"
            );
            assert_eq!(
                error.data.get("effect_evidence"),
                Some(&DataValue::String("known_effect".into())),
                "{source}: {error:#?}"
            );
            error.validate("effects.n", source).unwrap();
        }
    }

    #[test]
    fn borrowed_user_call_skips_borrowed_bindings_but_visits_later_temporaries() {
        for source in [
            "fn V(r:view MockBlockingResource){} fn Caller(r:view MockBlockingResource) with no_block{V(r);} fn Main(){}",
            "fn V(a:view I64,b:view MockBlockingResource){} fn Caller(n:view I64) with no_block{V(n,MockBlockingOpen(1));} fn Main(){}",
            "fn V(n:view I64,r:view MockBlockingResource){} fn Main() with no_block{V(1,MockBlockingOpen(1));}",
        ] {
            let errors = diagnostics(source);
            if source.contains("Caller(r:view") {
                assert!(errors.is_empty(), "{source}: {errors:#?}");
            } else {
                assert_eq!(errors.len(), 1, "{source}: {errors:#?}");
                assert_eq!(errors[0].code, "NX-E0413", "{source}: {errors:#?}");
                let open = source.find("MockBlockingOpen(1)").unwrap() as u64;
                assert_eq!(
                    errors[0]
                        .primary_span
                        .as_ref()
                        .map(|span| (span.start, span.end)),
                    Some((open, open + "MockBlockingOpen".len() as u64)),
                    "{source}: {errors:#?}"
                );
                assert_eq!(
                    errors[0]
                        .cause_chain
                        .iter()
                        .map(|cause| cause.operation.as_str())
                        .collect::<Vec<_>>(),
                    ["value.drop"],
                    "{source}: {errors:#?}"
                );
                errors[0].validate("effects.n", source).unwrap();
            }
        }
    }

    #[test]
    fn consumed_user_call_does_not_drop_the_transferred_temporary_in_the_caller() {
        let source =
            "fn V(r:move MockBlockingResource){} fn Main() with no_block{V(MockBlockingOpen(1));}";
        let errors = diagnostics(source);
        assert_eq!(errors.len(), 1, "{errors:#?}");
        assert_eq!(errors[0].code, "NX-E0413", "{errors:#?}");
        let call = source.find("V(MockBlockingOpen(1))").unwrap() as u64;
        assert_eq!(
            errors[0]
                .cause_chain
                .iter()
                .map(|cause| cause.operation)
                .collect::<Vec<_>>(),
            [
                CauseOperation::FunctionCall,
                CauseOperation::ValueDrop,
                CauseOperation::ResourceDrop,
            ],
            "{errors:#?}"
        );
        assert_eq!(
            errors[0]
                .primary_span
                .as_ref()
                .map(|span| (span.start, span.end)),
            Some((call, call + 1)),
            "{errors:#?}"
        );
        errors[0].validate("effects.n", source).unwrap();
    }

    #[test]
    fn result_resource_drop_uses_the_type_entity_not_a_fallback_value() {
        let source = "fn Main() with no_block{let r:Result<MockBlockingResource,I64>=Err(1);}";
        let errors = diagnostics(source);
        assert_eq!(errors[0].code, "NX-E0413");
        assert!(
            errors[0].cause_chain.iter().any(|cause| cause.operation
                == CauseOperation::ResourceDrop
                && cause
                    .entity
                    .as_ref()
                    .is_some_and(|entity| entity.kind == EntityKind::Type
                        && entity.id == "MockBlockingResource")),
            "{errors:#?}"
        );
    }

    #[test]
    fn fail_exit_contributes_live_resource_destructor_effects() {
        let source = "fn Main() with no_block{let r=MockBlockingOpen(4);Fail();}";
        let errors = diagnostics(source);
        assert_eq!(errors[0].code, "NX-E0413", "{errors:#?}");
        let fail = source.find("Fail()").unwrap() as u64;
        assert!(
            errors[0].related_spans.iter().any(|related| {
                related.role == RelatedRole::Exit
                    && related.span.start == fail
                    && related.span.end == fail + 6
            }),
            "{errors:#?}"
        );
    }

    #[test]
    fn explicit_drop_is_not_counted_as_an_implicit_droplocal() {
        let source = "fn Main() with no_block{let r=MockBlockingOpen(1);Drop(r);}";
        let errors = diagnostics(source);
        let drop = source.find("Drop(r)").unwrap() as u64;
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].code, "NX-E0413");
        assert_eq!(
            errors[0].primary_span.as_ref().map(|s| (s.start, s.end)),
            Some((drop, drop + 4))
        );
        assert_eq!(
            errors[0]
                .cause_chain
                .iter()
                .map(|c| c.operation.as_str())
                .collect::<Vec<_>>(),
            ["value.drop", "resource.drop"]
        );
        assert!(
            errors[0].cause_chain[0]
                .entity
                .as_ref()
                .is_some_and(|entity| {
                    entity.kind == EntityKind::Binding && entity.id == "binding:28"
                }),
            "{errors:#?}"
        );
        assert_eq!(
            errors[0].cause_chain[0]
                .span
                .as_ref()
                .map(|span| (span.start, span.end)),
            Some((drop, drop + "Drop(r)".len() as u64))
        );
        errors[0].validate("effects.n", source).unwrap();
    }

    #[test]
    fn explicit_temporary_drop_step_covers_the_whole_call() {
        let source = "fn Main() with no_block{Drop(MockBlockingOpen(1));}";
        let errors = diagnostics(source);
        assert_eq!(errors.len(), 1, "{errors:#?}");
        assert_eq!(errors[0].code, "NX-E0413", "{errors:#?}");
        assert_eq!(errors[0].operation, Operation::ValueDrop, "{errors:#?}");
        let call = source.find("Drop(").unwrap() as u64;
        let call_end = source.rfind(')').unwrap() as u64 + 1;
        let step = errors[0]
            .cause_chain
            .iter()
            .find(|cause| cause.operation == CauseOperation::ValueDrop)
            .unwrap();
        assert_eq!(
            step.span.as_ref().map(|span| (span.start, span.end)),
            Some((call, call_end)),
            "{errors:#?}"
        );
        assert_eq!(
            errors[0]
                .primary_span
                .as_ref()
                .map(|span| (span.start, span.end)),
            Some((call, call + 4)),
            "{errors:#?}"
        );
        errors[0].validate("effects.n", source).unwrap();
    }

    #[test]
    fn explicit_drop_of_result_binding_step_covers_the_whole_call() {
        let source =
            "fn Main() with no_block{let r:Result<MockBlockingResource,I64>=Err(1);Drop(r);}";
        let errors = diagnostics(source);
        assert_eq!(errors.len(), 1, "{errors:#?}");
        assert_eq!(errors[0].code, "NX-E0413", "{errors:#?}");
        let call = source.find("Drop(r)").unwrap() as u64;
        let step = errors[0]
            .cause_chain
            .iter()
            .find(|cause| cause.operation == CauseOperation::ValueDrop)
            .unwrap();
        assert_eq!(
            step.span.as_ref().map(|span| (span.start, span.end)),
            Some((call, call + "Drop(r)".len() as u64)),
            "{errors:#?}"
        );
        errors[0].validate("effects.n", source).unwrap();
    }

    #[test]
    fn conditional_explicit_drop_does_not_dominate_function_exit() {
        let source = "fn Main() with no_block{let r=MockBlockingOpen(1);if true{Drop(r);}}";
        let errors = diagnostics(source);
        assert!(
            errors.iter().any(|error| error.code == "NX-E0413"),
            "{errors:?}"
        );
        for error in &errors {
            error.validate("effects.n", source).unwrap();
        }
    }

    #[test]
    fn definite_drop_propagates_to_nested_cfg_exits() {
        let source =
            "fn Main() with no_block{let r=MockBlockingOpen(1);Drop(r);if true{Print(1);}}";
        let errors = diagnostics(source);
        assert_eq!(errors.len(), 1, "{errors:?}");
        assert_eq!(errors[0].code, "NX-E0413");
        let drop = source.find("Drop(r)").unwrap() as u64;
        assert_eq!(
            errors[0]
                .primary_span
                .as_ref()
                .map(|span| (span.start, span.end)),
            Some((drop, drop + 4))
        );
        errors[0].validate("effects.n", source).unwrap();
    }

    #[test]
    fn definite_drop_state_propagates_across_cfg_predecessors() {
        let source =
            "fn F(b:view Bool){let r=MockOpen(1);Drop(r);if b{let x=MockOpen(2);}} fn Main(){}";
        let program = compile_scalar("effects.n", source).unwrap();
        let function = program.functions().function("F").unwrap();
        assert!(function.blocks.len() >= 2, "{function:#?}");
        let resource = function
            .locals
            .iter()
            .find(|local| local.name == "r")
            .unwrap()
            .id;
        let nested_exit = function
            .exit_info
            .iter()
            .find(|exit| {
                exit.class == super::super::model::ExitClass::Fallthrough
                    && exit.live_locals.iter().any(|local| local.0 != resource.0)
            })
            .expect("nested local cleanup exit");
        assert!(nested_exit.definitely_dropped.contains(&resource));
        assert_eq!(verify_metadata(&program), Ok(()));
    }

    #[test]
    fn verifier_rejects_an_exit_local_as_both_maybe_owned_and_gone() {
        let source = "fn Main(){let r=MockBlockingOpen(1);Drop(r);}";
        let mut program = compile_scalar("effects.n", source).unwrap();
        let function = program
            .functions
            .entries
            .iter_mut()
            .find(|function| function.name == "Main")
            .unwrap();
        let exit = function.exit_info.first_mut().unwrap();
        let gone = exit.definitely_dropped[0];
        exit.live_locals.push(gone);
        assert_eq!(
            verify_metadata(&program),
            Err("exit ownership sets overlap")
        );
    }

    #[test]
    fn verifier_rejects_missing_known_or_unknown_local_effect_bits() {
        for source in ["fn Main(){Print(1);}", "fn Main(){MockUnknown();}"] {
            let mut program = compile_scalar("effects.n", source).unwrap();
            let function = program
                .functions
                .entries
                .iter_mut()
                .find(|function| function.name == "Main")
                .unwrap();
            function.effects = Some(EffectSummary {
                known: 0,
                unknown: 0,
            });
            assert_eq!(
                verify_metadata(&program),
                Err("effect summary local inclusion"),
                "source: {source}"
            );
        }
    }

    #[test]
    fn verifier_rejects_missing_known_or_unknown_callee_effect_bits() {
        for source in [
            "fn Leaf(){Print(1);} fn Main(){Leaf();}",
            "fn Leaf(){MockUnknown();} fn Main(){Leaf();}",
        ] {
            let mut program = compile_scalar("effects.n", source).unwrap();
            let function = program
                .functions
                .entries
                .iter_mut()
                .find(|function| function.name == "Main")
                .unwrap();
            function.effects = Some(EffectSummary::default());
            assert_eq!(
                verify_metadata(&program),
                Err("effect summary call inclusion"),
                "source: {source}"
            );
        }
    }

    #[test]
    fn multiple_droplocals_share_one_verified_block_exit() {
        let source = "fn Main(){if true{let a=MockBlockingOpen(1);let b=MockBlockingOpen(2);}}";
        let program = compile_scalar("effects.n", source).unwrap();
        let function = program.functions().function("Main").unwrap();
        let drops: Vec<_> = std::iter::once(&function.block)
            .chain(&function.blocks)
            .flat_map(|block| {
                block
                    .instructions
                    .iter()
                    .enumerate()
                    .filter_map(move |(index, instruction)| {
                        matches!(instruction.op, Op::DropLocal(_)).then_some((
                            block.id,
                            index,
                            instruction.span.clone(),
                        ))
                    })
            })
            .collect();
        assert_eq!(drops.len(), 2);
        assert_eq!(drops[0].0, drops[1].0);
        assert_eq!(drops[0].2, drops[1].2);
        let exit_sites: Vec<_> = function
            .exit_info
            .iter()
            .filter(|exit| {
                exit.class == super::super::model::ExitClass::Fallthrough
                    && exit.block == drops[0].0
                    && exit.index == drops[0].1
            })
            .collect();
        assert_eq!(exit_sites.len(), 1);
        assert_eq!(exit_sites[0].block, drops[0].0);
        assert_eq!(exit_sites[0].index, drops[0].1);
        assert_eq!(verify_metadata(&program), Ok(()));
    }

    #[test]
    fn scalar_droplocals_share_one_exit_without_ownership_liveness() {
        let program = compile_scalar("effects.n", "fn Main(){if true{let a=1;let b=2;}}").unwrap();
        let function = program.functions().function("Main").unwrap();
        let drops: Vec<_> = std::iter::once(&function.block)
            .chain(&function.blocks)
            .flat_map(|block| {
                block
                    .instructions
                    .iter()
                    .enumerate()
                    .filter_map(move |(index, instruction)| {
                        matches!(instruction.op, Op::DropLocal(_)).then_some((
                            block.id,
                            index,
                            instruction.span.clone(),
                        ))
                    })
            })
            .collect();
        assert_eq!(drops.len(), 2);
        assert_eq!(drops[0].0, drops[1].0);
        assert_eq!(drops[0].2, drops[1].2);
        assert!(
            function.exit_info.iter().any(|exit| {
                super::super::count_cfg_work(1);
                exit.class == super::super::model::ExitClass::Fallthrough
                    && exit.block == drops[0].0
                    && exit.index == drops[0].1
            }),
            "{function:#?}"
        );
        assert_eq!(verify_metadata(&program), Ok(()));
    }

    #[test]
    fn verifier_rejects_an_exit_anchored_to_another_closing_brace() {
        let source = "fn Main(){if true{let r=MockBlockingOpen(1);}}";
        let mut program = compile_scalar("effects.n", source).unwrap();
        let function = program
            .functions
            .entries
            .iter_mut()
            .find(|function| function.name == "Main")
            .unwrap();
        let other_close = source.rfind('}').unwrap() as u64;
        let exit = function
            .exit_info
            .iter_mut()
            .find(|exit| {
                exit.class == super::super::model::ExitClass::Fallthrough
                    && exit.span.start < other_close
            })
            .unwrap();
        exit.span = crate::diagnostic::Span {
            file: "effects.n".into(),
            start: other_close,
            end: other_close + 1,
        };
        assert_eq!(verify_metadata(&program), Err("exit anchor"));
    }

    #[test]
    fn return_exit_may_start_at_the_return_terminator() {
        let mut program = compile_scalar("effects.n", "fn Main(){return;}").unwrap();
        {
            let function = program
                .functions
                .entries
                .iter_mut()
                .find(|function| function.name == "Main")
                .unwrap();
            let span = match function.block.terminator {
                super::super::model::Terminator::Return { ref span, .. } => span.clone(),
                _ => panic!("Main must end with Return"),
            };
            let exit = function
                .exit_info
                .iter_mut()
                .find(|exit| exit.class == super::super::model::ExitClass::Return)
                .unwrap();
            exit.span = span;
        }
        assert_eq!(verify_metadata(&program), Ok(()));
        let function = program
            .functions
            .entries
            .iter_mut()
            .find(|function| function.name == "Main")
            .unwrap();
        function
            .exit_info
            .iter_mut()
            .find(|exit| exit.class == super::super::model::ExitClass::Return)
            .unwrap()
            .span
            .end += 1;
        assert_eq!(verify_metadata(&program), Err("exit anchor"));
    }

    #[test]
    fn try_and_fail_exit_classes_require_operation_anchors() {
        for class in [
            super::super::model::ExitClass::Try,
            super::super::model::ExitClass::Fail,
        ] {
            let mut program = compile_scalar("effects.n", "fn Main(){return;}").unwrap();
            let function = program
                .functions
                .entries
                .iter_mut()
                .find(|function| function.name == "Main")
                .unwrap();
            function
                .exit_info
                .iter_mut()
                .find(|exit| exit.class == super::super::model::ExitClass::Return)
                .unwrap()
                .class = class;
            assert_eq!(verify_metadata(&program), Err("exit anchor"));
        }
    }

    #[test]
    fn verifier_rejects_a_return_exit_on_a_non_return_terminator() {
        let source = "fn Main(){if true{let a=[1];}}";
        let mut program = compile_scalar("effects.n", source).unwrap();
        let function = program
            .functions
            .entries
            .iter_mut()
            .find(|function| function.name == "Main")
            .unwrap();
        let (block_id, instruction_count) = std::iter::once(&function.block)
            .chain(&function.blocks)
            .find(|block| {
                !matches!(
                    block.terminator,
                    super::super::model::Terminator::Return { .. }
                )
            })
            .map(|block| (block.id, block.instructions.len()))
            .unwrap();
        function.exit_info.push(super::super::model::ExitInfo {
            class: super::super::model::ExitClass::Return,
            block: block_id,
            index: instruction_count,
            span: crate::diagnostic::Span {
                file: "effects.n".into(),
                start: 0,
                end: 1,
            },
            live_locals: Vec::new(),
            definitely_dropped: Vec::new(),
        });
        assert_eq!(verify_metadata(&program), Err("exit anchor"));
    }

    #[test]
    fn verifier_rejects_a_fallthrough_span_at_source_end() {
        let source = "fn Main(){if true{let a=[1];}}";
        let mut program = compile_scalar("effects.n", source).unwrap();
        let function = program
            .functions
            .entries
            .iter_mut()
            .find(|function| function.name == "Main")
            .unwrap();
        let (block_id, index, span) = {
            let exit = function
                .exit_info
                .iter_mut()
                .find(|exit| exit.class == super::super::model::ExitClass::Fallthrough)
                .unwrap();
            exit.span.start = source.len() as u64;
            exit.span.end = source.len() as u64 + 1;
            (exit.block, exit.index, exit.span.clone())
        };
        let block = std::iter::once(&mut function.block)
            .chain(&mut function.blocks)
            .find(|block| block.id == block_id)
            .unwrap();
        block.instructions[index].span = span;
        assert_eq!(verify_metadata(&program), Err("exit close span"));

        let mut program = compile_scalar("effects.n", source).unwrap();
        let function = program
            .functions
            .entries
            .iter_mut()
            .find(|function| function.name == "Main")
            .unwrap();
        let (block_id, index, span) = {
            let exit = function
                .exit_info
                .iter_mut()
                .find(|exit| exit.class == super::super::model::ExitClass::Fallthrough)
                .unwrap();
            let start = source.find("let a").unwrap() as u64;
            exit.span.start = start;
            exit.span.end = start + 1;
            (exit.block, exit.index, exit.span.clone())
        };
        let block = std::iter::once(&mut function.block)
            .chain(&mut function.blocks)
            .find(|block| block.id == block_id)
            .unwrap();
        block.instructions[index].span = span;
        assert_eq!(verify_metadata(&program), Err("exit close span"));
    }

    #[test]
    fn exit_info_uses_the_matching_ast_function_for_implicit_returns() {
        let source = "fn A(){} fn Main(){}";
        let program = compile_scalar("effects.n", source).unwrap();
        for name in ["A", "Main"] {
            let function = program.functions().function(name).unwrap();
            let expected_close = source.find(&format!("fn {name}(){{")).unwrap() as u64
                + format!("fn {name}(){{").len() as u64;
            let close =
                source[expected_close as usize..].find('}').unwrap() as u64 + expected_close;
            assert!(function.exit_info.iter().any(|exit| {
                exit.class == super::super::model::ExitClass::Fallthrough
                    && (exit.span.start, exit.span.end) == (close, close + 1)
            }));
        }
    }

    #[test]
    fn explicit_return_exit_spans_match_their_own_ast_returns() {
        let source = "fn Main(){if true{let x=1;return;}else{return;}}";
        let program = compile_scalar("effects.n", source).unwrap();
        let function = program.functions().function("Main").unwrap();
        let mut actual: Vec<_> = function
            .exit_info
            .iter()
            .filter(|exit| exit.class == super::super::model::ExitClass::Return)
            .map(|exit| (exit.span.start, exit.span.end))
            .collect();
        actual.sort_unstable();
        let mut expected: Vec<_> = source
            .match_indices("return;")
            .map(|(start, _)| (start as u64, (start + "return".len()) as u64))
            .collect();
        expected.sort_unstable();
        assert_eq!(actual, expected);
    }

    #[test]
    fn exit_population_ignores_out_of_range_cfg_predecessors_without_panicking() {
        let source = "fn Main(){if true{let a=[1];}}";
        let ast = crate::frontend::parse::parse("effects.n", source).unwrap();
        let program = compile_scalar("effects.n", source).unwrap();
        let mut function = program.functions().function("Main").unwrap().clone();
        let block_count = 1 + function.blocks.len();
        let block = std::iter::once(&mut function.block)
            .chain(&mut function.blocks)
            .find(|block| {
                !matches!(
                    block.terminator,
                    super::super::model::Terminator::Return { .. }
                )
            })
            .unwrap();
        match &mut block.terminator {
            super::super::model::Terminator::Jump { target, .. } => {
                *target = super::super::model::BlockId(block_count);
            }
            super::super::model::Terminator::BranchBool { yes, .. } => {
                *yes = super::super::model::BlockId(block_count);
            }
            _ => panic!("fixture must contain an outgoing CFG edge"),
        }
        assert!(
            std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                populate_exit_info(&mut function, &ast.functions[0])
            }))
            .is_ok()
        );
    }

    #[test]
    fn ownership_liveness_propagates_across_cfg_predecessors() {
        let program = compile_scalar(
            "effects.n",
            "fn Main(){let r=MockBlockingOpen(1);if true{Print(1);}}",
        )
        .unwrap();
        let function = program.functions().function("Main").unwrap();
        let owner = function
            .locals
            .iter()
            .find(|local| local.name == "r")
            .unwrap()
            .id;
        assert!(function.exit_info.iter().any(|exit| {
            exit.live_locals.contains(&owner)
                && matches!(
                    exit.class,
                    super::super::model::ExitClass::Fallthrough
                        | super::super::model::ExitClass::Return
                )
        }));

        let program = compile_scalar(
            "effects.n",
            "fn F(b:view Bool){var r=MockOpen(1);Drop(r);while b{r=MockOpen(2);}} fn Main(){}",
        )
        .unwrap();
        let function = program.functions().function("F").unwrap();
        let owner = function
            .locals
            .iter()
            .find(|local| local.name == "r")
            .unwrap()
            .id;
        assert!(function.exit_info.iter().any(|exit| {
            exit.live_locals.contains(&owner)
                && matches!(
                    exit.class,
                    super::super::model::ExitClass::Fallthrough
                        | super::super::model::ExitClass::Return
                )
        }));
    }

    #[test]
    fn resource_parameters_are_owned_at_entry_and_dropped_on_exit() {
        let source = "fn F(r:move MockBlockingResource) with no_block{} fn Main(){}";
        let errors = diagnostics(source);
        assert_eq!(errors.len(), 1, "{errors:?}");
        assert_eq!(errors[0].code, "NX-E0413", "{errors:?}");
        let parameter = source.find("r:").unwrap() as u64;
        assert_eq!(
            errors[0]
                .primary_span
                .as_ref()
                .map(|span| (span.start, span.end)),
            Some((parameter, parameter + 1))
        );
        errors[0].validate("effects.n", source).unwrap();
    }

    #[test]
    fn result_bindings_keep_the_union_of_destructor_effects() {
        let source = "fn Main() with no_block{let r:Result<MockBlockingResource,I64>=Err(1);}";
        let errors = diagnostics(source);
        assert_eq!(errors[0].code, "NX-E0413");
        let owner = source.find("r:").unwrap() as u64;
        assert_eq!(
            errors[0].primary_span.as_ref().map(|s| (s.start, s.end)),
            Some((owner, owner + 1))
        );

        let source = "fn Main() with no_block{let r:Result<I64,MockBlockingResource>=Ok(1);}";
        let errors = diagnostics(source);
        assert_eq!(errors[0].code, "NX-E0413");
        let owner = source.find("r:").unwrap() as u64;
        assert_eq!(
            errors[0].primary_span.as_ref().map(|s| (s.start, s.end)),
            Some((owner, owner + 1))
        );
    }

    #[test]
    fn typed_result_temporary_drop_uses_its_ir_variant_evidence() {
        let source = "fn Main(){let r:Result<MockBlockingResource,MockResource>=Ok(MockBlockingOpen(1));Drop(r);}";
        let program = compile_scalar("effects.n", source).unwrap();
        let mut function = program.functions().function("Main").unwrap().clone();
        let result_temp = function
            .block
            .instructions
            .iter()
            .find_map(|instruction| {
                matches!(
                    instruction.op,
                    Op::BuildVariant {
                        tag: crate::union::VariantTag(0),
                        ..
                    }
                )
                .then_some(instruction.output)
                .flatten()
            })
            .unwrap();
        let drop = function
            .block
            .instructions
            .iter_mut()
            .find(|instruction| {
                matches!(
                    instruction.op,
                    Op::CallIntrinsic {
                        intrinsic: super::super::model::Intrinsic::DropData,
                        ..
                    }
                )
            })
            .unwrap();
        drop.op = Op::CallIntrinsic {
            intrinsic: super::super::model::Intrinsic::DropData,
            args: vec![result_temp],
        };
        let witness = function_intrinsic_effect(&function)
            .into_iter()
            .find(|witness| witness.operation == CauseOperation::ValueDrop)
            .unwrap();
        assert_eq!(witness.summary.known, BLOCK | IO);
        assert!(
            witness
                .chain
                .iter()
                .any(|cause| cause.operation == CauseOperation::ResourceDrop)
        );

        let source =
            "fn Main(){let r:Result<MockBlockingResource,MockResource>=Err(MockOpen(1));Drop(r);}";
        let program = compile_scalar("effects.n", source).unwrap();
        let mut function = program.functions().function("Main").unwrap().clone();
        let result_temp = function
            .block
            .instructions
            .iter()
            .find_map(|instruction| {
                matches!(
                    instruction.op,
                    Op::BuildVariant {
                        tag: crate::union::VariantTag(1),
                        ..
                    }
                )
                .then_some(instruction.output)
                .flatten()
            })
            .unwrap();
        let drop = function
            .block
            .instructions
            .iter_mut()
            .find(|instruction| {
                matches!(
                    instruction.op,
                    Op::CallIntrinsic {
                        intrinsic: super::super::model::Intrinsic::DropData,
                        ..
                    }
                )
            })
            .unwrap();
        drop.op = Op::CallIntrinsic {
            intrinsic: super::super::model::Intrinsic::DropData,
            args: vec![result_temp],
        };
        let witness = function_intrinsic_effect(&function)
            .into_iter()
            .find(|witness| witness.operation == CauseOperation::ValueDrop)
            .unwrap();
        assert_eq!(witness.summary.known, IO);
        let dropped_resources: Vec<_> = witness
            .chain
            .iter()
            .filter(|cause| cause.operation == CauseOperation::ResourceDrop)
            .filter_map(|cause| cause.entity.as_ref().map(|entity| entity.id.as_str()))
            .collect();
        assert_eq!(dropped_resources, ["MockResource"]);
    }

    #[test]
    fn result_payload_from_try_is_transferred_without_a_false_drop() {
        let source = "fn F()->Result<MockBlockingResource,I64>{return Err(1);} fn G()->Result<MockBlockingResource,I64> with no_block{let n=F()?;return Ok(move n);}";
        assert!(crate::ir::check_scalar("effects.n", source).is_ok());
    }

    #[test]
    fn recursive_call_components_converge_and_keep_known_leaf_witnesses() {
        let source = "fn A(){B();} fn B(){A();Print(1);} fn Main() with no_io{A();}";
        let errors = diagnostics(source);
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].code, "NX-E0415");
        assert_eq!(
            errors[0].data.get("effect_evidence"),
            Some(&DataValue::String("known_effect".into()))
        );
        let a_call = source.rfind("A();").unwrap() as u64;
        assert_eq!(
            errors[0].primary_span.as_ref().map(|s| (s.start, s.end)),
            Some((a_call, a_call + 1))
        );
        errors[0].validate("effects.n", source).unwrap();
    }

    #[test]
    fn caller_component_waits_until_a_later_declared_callee_is_solved() {
        let source = "fn A() with no_io{Zeta();} fn Main(){} fn Zeta(){Print(1);}";
        let errors = diagnostics(source);
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].code, "NX-E0415");
        let call = source.find("Zeta();").unwrap() as u64;
        assert_eq!(
            errors[0]
                .primary_span
                .as_ref()
                .map(|span| (span.start, span.end)),
            Some((call, call + 4))
        );
        errors[0].validate("effects.n", source).unwrap();
    }

    #[test]
    fn call_components_are_strong_and_order_callee_before_caller() {
        let program = compile_scalar(
            "effects.n",
            "fn A(){C();} fn B(){C();} fn C(){} fn Main(){A();B();}",
        )
        .unwrap();
        let functions = program.functions.functions();
        let caller = functions.iter().position(|f| f.name == "A").unwrap();
        let second_caller = functions.iter().position(|f| f.name == "B").unwrap();
        let callee = functions.iter().position(|f| f.name == "C").unwrap();
        let entry = functions.iter().position(|f| f.name == "Main").unwrap();
        let components = call_components(&program);
        let caller_component = components
            .iter()
            .position(|component| component == &[caller])
            .unwrap();
        let second_caller_component = components
            .iter()
            .position(|component| component == &[second_caller])
            .unwrap();
        let callee_component = components
            .iter()
            .position(|component| component == &[callee])
            .unwrap();
        let entry_component = components
            .iter()
            .position(|component| component == &[entry])
            .unwrap();
        assert_eq!(
            components.iter().map(Vec::len).sum::<usize>(),
            functions.len()
        );
        assert!(
            callee_component < caller_component,
            "callee SCC precedes caller SCC"
        );
        assert!(callee_component < second_caller_component);
        assert!(caller_component < entry_component && second_caller_component < entry_component);
    }

    #[test]
    fn distinct_constraints_emit_once_each_and_print_does_not_imply_alloc() {
        let scalar_copy = "fn Main() with no_alloc{let x=1;let y=x;}";
        assert!(crate::ir::check_scalar("effects.n", scalar_copy).is_ok());

        let unknown = "fn Main() with no_io,no_alloc{MockUnknown();}";
        let errors = diagnostics(unknown);
        assert_eq!(errors.len(), 2);
        let source = "fn Main() with no_io,no_alloc{Print(1);let a=[1];}";
        let errors = diagnostics(source);
        assert_eq!(
            errors.iter().map(|d| d.code.as_str()).collect::<Vec<_>>(),
            ["NX-E0415", "NX-E0412"]
        );
        assert_eq!(
            errors[0].primary_span.as_ref().map(|s| (s.start, s.end)),
            Some((30, 35))
        );
        let array = source.find("[1]").unwrap() as u64;
        assert_eq!(
            errors[1].primary_span.as_ref().map(|s| (s.start, s.end)),
            Some((array, array + 3))
        );
    }

    #[test]
    fn call_restrictions_use_transitive_effect_summaries() {
        let source = "fn Leaf(){Print(1);} fn Main() with {call(no_io)}{Leaf();}";
        let errors = diagnostics(source);
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].code, "NX-E0415");
        let call = source.rfind("Leaf();").unwrap() as u64;
        assert_eq!(
            errors[0]
                .primary_span
                .as_ref()
                .map(|span| (span.start, span.end)),
            Some((call, call + 4))
        );
        errors[0].validate("effects.n", source).unwrap();
    }
}

fn find_witness(
    program: &Program,
    ast: &Ast,
    local: &[Vec<Witness>],
    start: usize,
    bit: u8,
) -> Option<Witness> {
    #[derive(Clone)]
    struct Path {
        unknown: bool,
        edges: usize,
        next: Option<(usize, Cause)>,
        leaf: (usize, usize),
    }
    fn causes(path: &Path, best: &[Option<Path>], local: &[Vec<Witness>]) -> Vec<Cause> {
        let mut result = Vec::new();
        let mut current = path;
        while let Some((next, cause)) = &current.next {
            super::count_cfg_work(1);
            result.push(cause.clone());
            current = best[*next].as_ref().expect("shortest suffix was solved");
        }
        super::count_cfg_work(local[path.leaf.0][path.leaf.1].chain.len() as u64);
        result.extend(local[path.leaf.0][path.leaf.1].chain.iter().cloned());
        result
    }
    fn compare_paths(
        a: &Path,
        b: &Path,
        best: &[Option<Path>],
        local: &[Vec<Witness>],
    ) -> std::cmp::Ordering {
        let a_causes = a.edges + local[a.leaf.0][a.leaf.1].chain.len();
        let b_causes = b.edges + local[b.leaf.0][b.leaf.1].chain.len();
        // The reference ordering counts the complete cause chain, including
        // the selected leaf's intrinsic causes. Keep call distance as the BFS
        // primary key, then compare that total before offsets and identities.
        let prefix = (a.unknown, a_causes).cmp(&(b.unknown, b_causes));
        if prefix != std::cmp::Ordering::Equal {
            return prefix;
        }
        // Call sites in a source function have distinct starts, so the
        // overwhelmingly common diamond tie is decided without walking or
        // copying the suffix chain.
        if let (Some((_, left)), Some((_, right))) = (&a.next, &b.next)
            && let (Some(left), Some(right)) = (&left.span, &right.span)
            && left.start != right.start
        {
            return left.start.cmp(&right.start);
        }
        let a_chain = causes(a, best, local);
        let b_chain = causes(b, best, local);
        let a_offsets: Vec<_> = a_chain
            .iter()
            .filter_map(|cause| cause.span.as_ref().map(|span| span.start))
            .collect();
        let b_offsets: Vec<_> = b_chain
            .iter()
            .filter_map(|cause| cause.span.as_ref().map(|span| span.start))
            .collect();
        a_offsets.cmp(&b_offsets).then_with(|| {
            let a_ids: Vec<_> = a_chain
                .iter()
                .filter_map(|cause| cause.entity.as_ref().map(|entity| entity.id.as_str()))
                .collect();
            let b_ids: Vec<_> = b_chain
                .iter()
                .filter_map(|cause| cause.entity.as_ref().map(|entity| entity.id.as_str()))
                .collect();
            a_ids.cmp(&b_ids)
        })
    }

    let functions = program.functions.functions();
    let declarations: HashMap<_, _> = ast
        .functions
        .iter()
        .map(|declaration| (declaration.name.as_str(), declaration.name_span.start))
        .collect();
    let indices: HashMap<_, _> = functions
        .iter()
        .enumerate()
        .map(|(index, function)| (function.id.0.as_str(), index))
        .collect();
    let mut callers = vec![Vec::<(usize, Cause)>::new(); functions.len()];
    for (caller, function) in functions.iter().enumerate() {
        for block in std::iter::once(&function.block).chain(&function.blocks) {
            for instruction in &block.instructions {
                let Op::CallDirect {
                    function: target, ..
                } = &instruction.op
                else {
                    continue;
                };
                let Some(&target_index) = indices.get(target.0.as_str()) else {
                    continue;
                };
                callers[caller].push((
                    target_index,
                    Cause {
                        kind: CauseKind::Call,
                        operation: CauseOperation::FunctionCall,
                        span: Some(call_head(&instruction.span, &function.source_text)),
                        entity: Some(Entity {
                            kind: EntityKind::Function,
                            id: format!(
                                "function:{}",
                                declarations
                                    .get(functions[target_index].name.as_str())
                                    .copied()
                                    .unwrap_or(0)
                            ),
                        }),
                        evidence: Evidence::Verified,
                    },
                ));
            }
        }
    }

    // First find minimum call distance from the violating function. The
    // enumerator's total-cause key cannot prefer a longer call prefix, so the
    // BFS shortest-path DAG retains exactly the candidates that can win.
    let mut distance = vec![usize::MAX; functions.len()];
    let mut queue = VecDeque::from([start]);
    distance[start] = 0;
    while let Some(caller) = queue.pop_front() {
        super::count_cfg_work(1);
        for (target, _) in &callers[caller] {
            super::count_cfg_work(1);
            if distance[*target] == usize::MAX {
                distance[*target] = distance[caller] + 1;
                queue.push_back(*target);
            }
        }
    }

    // On the shortest-path DAG, retain one best suffix at each function.
    // Prefixing the same call cause preserves the ordering of the suffix
    // tuple (known, total cause count, offsets, identity), so the best
    // suffix composes to the best complete witness.
    let mut order: Vec<_> = (0..functions.len())
        .filter(|index| distance[*index] != usize::MAX)
        .collect();
    order.sort_unstable_by_key(|index| std::cmp::Reverse(distance[*index]));
    let mut best: Vec<Option<Path>> = vec![None; functions.len()];
    for caller in order {
        for (leaf_index, leaf) in local[caller].iter().enumerate() {
            super::count_cfg_work(1);
            if leaf.summary.known & bit == 0 && leaf.summary.unknown & bit == 0 {
                continue;
            }
            let candidate = Path {
                unknown: leaf.summary.known & bit == 0,
                edges: 0,
                next: None,
                leaf: (caller, leaf_index),
            };
            if best[caller]
                .as_ref()
                .is_none_or(|prior| compare_paths(&candidate, prior, &best, local).is_lt())
            {
                best[caller] = Some(candidate);
            }
        }
        for (target, cause) in &callers[caller] {
            super::count_cfg_work(1);
            if distance[*target] != distance[caller] + 1 {
                continue;
            }
            let Some(suffix) = best[*target].as_ref() else {
                continue;
            };
            let candidate = Path {
                unknown: suffix.unknown,
                edges: suffix.edges + 1,
                next: Some((*target, cause.clone())),
                leaf: suffix.leaf,
            };
            if best[caller]
                .as_ref()
                .is_none_or(|prior| compare_paths(&candidate, prior, &best, local).is_lt())
            {
                best[caller] = Some(candidate);
            }
        }
    }
    best.get(start).and_then(Option::as_ref).map(|path| {
        let mut witness = local[path.leaf.0][path.leaf.1].clone();
        let call_causes = causes(path, &best, local);
        let call_count = path.edges;
        if call_count > 0 {
            witness.primary = call_causes[0].span.clone().unwrap_or(witness.primary);
            witness.operation = CauseOperation::FunctionCall;
            witness
                .chain
                .splice(0..0, call_causes.into_iter().take(call_count));
        }
        witness
    })
}

#[cfg(test)]
fn find_witness_enumerated(
    program: &Program,
    ast: &Ast,
    local: &[Vec<Witness>],
    start: usize,
    bit: u8,
) -> Option<Witness> {
    let functions = program.functions.functions();
    let mut queue = VecDeque::from([(start, Vec::<Cause>::new(), vec![start])]);
    let mut candidates: Vec<(bool, usize, Vec<u64>, String, Witness)> = Vec::new();
    while let Some((at, prefix, path)) = queue.pop_front() {
        for leaf in &local[at] {
            if leaf.summary.known & bit != 0 || leaf.summary.unknown & bit != 0 {
                let mut witness = leaf.clone();
                witness.chain.splice(0..0, prefix.clone());
                if !prefix.is_empty() {
                    witness.primary = prefix[0].span.clone().unwrap_or(witness.primary);
                    witness.operation = CauseOperation::FunctionCall;
                }
                let offsets = witness
                    .chain
                    .iter()
                    .filter_map(|cause| cause.span.as_ref().map(|span| span.start))
                    .collect();
                let identity = witness
                    .chain
                    .iter()
                    .filter_map(|cause| cause.entity.as_ref().map(|entity| entity.id.clone()))
                    .collect::<Vec<_>>()
                    .join("\0");
                candidates.push((
                    leaf.summary.known & bit == 0,
                    witness.chain.len(),
                    offsets,
                    identity,
                    witness,
                ));
            }
        }
        let function = &functions[at];
        for block in std::iter::once(&function.block).chain(&function.blocks) {
            for instruction in &block.instructions {
                let Op::CallDirect {
                    function: target, ..
                } = &instruction.op
                else {
                    continue;
                };
                let Some(target_index) = functions
                    .iter()
                    .position(|candidate| candidate.id == *target)
                else {
                    continue;
                };
                if path.contains(&target_index) {
                    continue;
                }
                let declaration = ast
                    .functions
                    .iter()
                    .find(|decl| decl.name == functions[target_index].name);
                let mut next_prefix = prefix.clone();
                next_prefix.push(Cause {
                    kind: CauseKind::Call,
                    operation: CauseOperation::FunctionCall,
                    span: Some(call_head(&instruction.span, &function.source_text)),
                    entity: Some(Entity {
                        kind: EntityKind::Function,
                        id: format!(
                            "function:{}",
                            declaration.map_or(0, |decl| decl.name_span.start)
                        ),
                    }),
                    evidence: Evidence::Verified,
                });
                let mut next_path = path.clone();
                next_path.push(target_index);
                queue.push_back((target_index, next_prefix, next_path));
            }
        }
    }
    candidates.sort_by(|a, b| (&a.0, &a.1, &a.2, &a.3).cmp(&(&b.0, &b.1, &b.2, &b.3)));
    candidates.into_iter().next().map(|entry| entry.4.clone())
}

#[cfg(test)]
mod witness_differential_tests {
    use super::*;

    fn witness_chain(size: usize) -> (String, Ast, Program, usize, Vec<Vec<Witness>>) {
        let mut source = String::new();
        for index in 0..size {
            source.push_str(&format!("fn F{index}(){{F{}();}}", index + 1));
        }
        source.push_str(&format!("fn F{size}(){{Print(0);}}fn Main(){{F0();}}"));
        let ast = crate::frontend::parse::parse("witness-scale.n", &source).unwrap();
        let program = crate::ir::compile_scalar("witness-scale.n", &source).unwrap();
        let local = program
            .functions
            .functions()
            .iter()
            .map(function_intrinsic_effect)
            .collect();
        let start = program
            .functions
            .functions()
            .iter()
            .position(|function| function.name == "Main")
            .unwrap();
        (source, ast, program, start, local)
    }

    fn synthetic_diamond(size: usize) -> (Ast, Program, usize, Vec<Vec<Witness>>) {
        let source = "fn Leaf(){Print(0);}fn A(){Leaf();}fn Main(){A();}";
        let ast = crate::frontend::parse::parse("witness-diamond.n", source).unwrap();
        let mut program = crate::ir::compile_scalar("witness-diamond.n", source).unwrap();
        let leaf = program.functions.function("Leaf").unwrap().clone();
        let caller = program.functions.function("A").unwrap().clone();
        let main_id = program.functions.function("Main").unwrap().id.clone();
        let target_ids: Vec<_> = (0..size)
            .map(|index| super::super::model::FunctionId(format!("synthetic:A{index}")))
            .collect();
        let leaf_id = leaf.id.clone();
        for index in 0..size {
            let mut function = if index + 1 == size {
                leaf.clone()
            } else {
                caller.clone()
            };
            function.name = format!("A{index}");
            function.id = target_ids[index].clone();
            function.table_index = program.functions.functions().len() + index;
            if index + 1 < size {
                let target = target_ids[index + 1].clone();
                let mut calls = Vec::new();
                for (call_index, instruction) in
                    function.block.instructions.iter().cloned().enumerate()
                {
                    let mut instruction = instruction;
                    if let Op::CallDirect { function, .. } = &mut instruction.op {
                        *function = target.clone();
                    }
                    if call_index == 1 {
                        instruction.span.start += 1;
                    }
                    calls.push(instruction);
                }
                function.block.instructions = calls.clone();
                if let Some(first) = calls.first().cloned() {
                    let mut parallel = first;
                    parallel.span.start += 1;
                    function.block.instructions.push(parallel);
                }
            }
            program.functions.entries.push(function);
        }
        if let Some(main) = program
            .functions
            .entries
            .iter_mut()
            .find(|function| function.id == main_id)
        {
            for instruction in &mut main.block.instructions {
                if let Op::CallDirect { function, .. } = &mut instruction.op {
                    *function = target_ids[0].clone();
                }
            }
        }
        let local = program
            .functions
            .functions()
            .iter()
            .map(function_intrinsic_effect)
            .collect();
        let start = program
            .functions
            .functions()
            .iter()
            .position(|function| function.id == main_id)
            .unwrap();
        let _ = leaf_id;
        (ast, program, start, local)
    }

    #[test]
    fn witness_search_work_tracks_call_graph_size() {
        let units = [32, 128, 512].map(|size| {
            let (_source, ast, program, start, local) = witness_chain(size);
            let (witness, units) = crate::ir::measure_cfg_thread_work(|| {
                find_witness(&program, &ast, &local, start, IO)
            });
            assert!(witness.is_some());
            units
        });
        assert!(units[1] <= units[0] * 5, "{units:?}");
        assert!(units[2] <= units[1] * 5, "{units:?}");
    }

    #[test]
    #[ignore = "release timing campaign; record locally when running scale verification"]
    fn witness_search_release_time_ratio_is_bounded() {
        let elapsed = [32, 128, 512].map(|size| {
            let (_source, ast, program, start, local) = witness_chain(size);
            let began = std::time::Instant::now();
            assert!(find_witness(&program, &ast, &local, start, IO).is_some());
            began.elapsed()
        });
        eprintln!("I10a witness search-only release seconds at 32/128/512: {elapsed:?}");
        assert!(
            elapsed[1].as_secs_f64() <= elapsed[0].as_secs_f64() * 8.0,
            "{elapsed:?}"
        );
        assert!(
            elapsed[2].as_secs_f64() <= elapsed[1].as_secs_f64() * 8.0,
            "{elapsed:?}"
        );
    }

    #[test]
    #[ignore = "release timing repro for validator B1"]
    fn witness_release_timing_matches_validator_scale_shapes() {
        let (_source, ast, program, start, local) = witness_chain(800);
        let began = std::time::Instant::now();
        assert!(find_witness(&program, &ast, &local, start, IO).is_some());
        eprintln!(
            "I10a find_witness-only chain-800: {:.6}s",
            began.elapsed().as_secs_f64()
        );
        for size in [512, 4_000] {
            let (ast, program, start, local) = synthetic_diamond(size);
            let began = std::time::Instant::now();
            assert!(find_witness(&program, &ast, &local, start, IO).is_some());
            let elapsed = began.elapsed().as_secs_f64();
            eprintln!("I10a synthetic diamond n={size}: {elapsed:.3}s");
            assert!(
                elapsed < 10.0,
                "absolute witness time n={size}: {elapsed:.3}s"
            );
        }
    }

    #[test]
    fn dynamic_witness_matches_simple_path_enumerator_on_small_graphs() {
        for depth in 1..=13 {
            let mut source = String::from("fn Leaf(){Print(0);} ");
            for level in (0..depth).rev() {
                let target = if level + 1 == depth {
                    "Leaf".to_owned()
                } else {
                    format!("A{}", level + 1)
                };
                source.push_str(&format!("fn A{level}(){{{target}();{target}();}} "));
            }
            source.push_str("fn Main(){A0();}");
            let ast = crate::frontend::parse::parse("witness-dp.n", &source).unwrap();
            let program = crate::ir::check_scalar("witness-dp.n", &source).unwrap();
            let local: Vec<_> = program
                .functions
                .functions()
                .iter()
                .map(function_intrinsic_effect)
                .collect();
            let main = program
                .functions
                .functions()
                .iter()
                .position(|function| function.name == "Main")
                .unwrap();
            let dynamic = find_witness(&program, &ast, &local, main, IO).unwrap();
            let enumerated = find_witness_enumerated(&program, &ast, &local, main, IO).unwrap();
            assert_eq!(dynamic.summary, enumerated.summary, "depth={depth}");
            assert_eq!(dynamic.primary, enumerated.primary, "depth={depth}");
            assert_eq!(dynamic.operation, enumerated.operation, "depth={depth}");
            assert_eq!(dynamic.chain, enumerated.chain, "depth={depth}");
            assert_eq!(dynamic.related, enumerated.related, "depth={depth}");
        }
    }

    #[test]
    fn dynamic_witness_matches_reference_with_variable_leaf_chains_and_sccs() {
        // Five fixed seeds mirror the independent val36 generator's source
        // shapes while keeping each enumerated graph small and reproducible.
        let mut compared = 0usize;
        let mut leaf_lengths = HashSet::new();
        let mut has_unknown_leaf = false;
        let mut multiple_constraints = 0usize;
        for seed in [
            36_000_001u64,
            36_000_002,
            36_000_003,
            36_000_004,
            36_000_005,
        ] {
            let mut state = seed;
            let mut next = || {
                state ^= state << 13;
                state ^= state >> 7;
                state ^= state << 17;
                state
            };
            for case in 0..500 {
                let count = 3 + (next() as usize % 4);
                let mut source = String::new();
                for index in 0..count {
                    let mut body = String::new();
                    for _ in 0..(next() as usize % 3) {
                        body.push_str(&format!("G{}();", next() as usize % count));
                    }
                    match next() % 4 {
                        0 => body.push_str("Print(0);"),
                        1 => body.push_str("MockUnknown();"),
                        2 => body.push_str("let r=MockOpen(1);Drop(r);"),
                        _ => body.push_str("let r=MockOpen(1);Drop(r);Print(0);"),
                    }
                    source.push_str(&format!("fn G{index}(){{{body}}}"));
                }
                source.push_str("fn Main(){");
                for index in 0..count {
                    source.push_str(&format!("G{index}();"));
                }
                source.push('}');
                let ast = crate::frontend::parse::parse("witness-seeded.n", &source).unwrap();
                let program = crate::ir::check_scalar("witness-seeded.n", &source).unwrap();
                let functions = program.functions.functions();
                let mut local: Vec<_> = functions.iter().map(function_intrinsic_effect).collect();
                // Extend the source-derived local evidence with deterministic
                // longer leaf chains, as the differential oracle in val36 did.
                let leaf_function = functions.iter().position(|f| f.name == "G0").unwrap();
                let base = local[leaf_function][0].clone();
                for length in 1..=3 {
                    let mut leaf = base.clone();
                    leaf.chain.resize(length, base.chain[0].clone());
                    if length == 3 {
                        leaf.summary.known = 0;
                        leaf.summary.unknown |= IO;
                    }
                    local[leaf_function].push(leaf);
                }
                for leaf in local.iter().flatten() {
                    leaf_lengths.insert(leaf.chain.len());
                    has_unknown_leaf |= leaf.summary.unknown != 0;
                }
                let main = functions.iter().position(|f| f.name == "Main").unwrap();
                let mut violated_bits = 0;
                for bit in [IO, ALLOC] {
                    let expected = find_witness_enumerated(&program, &ast, &local, main, bit);
                    if expected.is_none() {
                        continue;
                    }
                    violated_bits += 1;
                    let actual = find_witness(&program, &ast, &local, main, bit);
                    assert_eq!(
                        actual.as_ref().map(|w| (
                            &w.summary,
                            &w.primary,
                            w.operation,
                            &w.chain,
                            &w.related
                        )),
                        expected.as_ref().map(|w| (
                            &w.summary,
                            &w.primary,
                            w.operation,
                            &w.chain,
                            &w.related
                        )),
                        "seed={seed} case={case} bit={bit}\n{source}"
                    );
                    compared += 1;
                }
                multiple_constraints += usize::from(violated_bits == 2);
            }
        }
        assert!(
            compared >= 2_400,
            "only compared {compared} violating cases"
        );
        assert!(
            (1..=3).all(|length| leaf_lengths.contains(&length)),
            "leaf lengths={leaf_lengths:?}"
        );
        assert!(
            has_unknown_leaf,
            "seeded sources must include unknown leaves"
        );
        assert!(
            multiple_constraints > 0,
            "sources must violate multiple constraints"
        );
    }

    #[test]
    fn witness_prefers_total_cause_count_before_callsite_offset() {
        let source = "fn Make()->Result<MockResource,MockResource>{return Ok(MockOpen(1));} fn B()->Result<MockResource,MockResource>{return Make();} fn A(r:move Result<MockResource,MockResource>){Drop(r);} fn Main(){let r=B();A(move r);} ";
        let ast = crate::frontend::parse::parse("witness-key.n", source).unwrap();
        let program = crate::ir::check_scalar("witness-key.n", source).unwrap();
        let functions = program.functions.functions();
        let local: Vec<_> = functions.iter().map(function_intrinsic_effect).collect();
        let main = functions.iter().position(|f| f.name == "Main").unwrap();
        let actual = find_witness(&program, &ast, &local, main, IO).unwrap();
        let expected = find_witness_enumerated(&program, &ast, &local, main, IO).unwrap();
        assert_eq!(actual.summary, expected.summary);
        assert_eq!(actual.primary, expected.primary);
        assert_eq!(actual.operation, expected.operation);
        assert_eq!(actual.chain, expected.chain);
        assert_eq!(actual.related, expected.related);
        let expected_first_target = format!("function:{}", source.find("fn B").unwrap() + 3);
        assert_eq!(
            actual.chain[0]
                .entity
                .as_ref()
                .map(|entity| entity.id.as_str()),
            Some(expected_first_target.as_str())
        );
    }

    #[test]
    fn dynamic_witness_handles_the_twenty_level_diamond_ladder() {
        let depth = 20;
        let mut source = String::new();
        for level in 0..depth {
            source.push_str(&format!(
                "fn A{level}(){{B{level}();C{level}();}}fn B{level}(){{A{}();}}fn C{level}(){{A{}();}}",
                level + 1,
                level + 1
            ));
        }
        source.push_str(&format!(
            "fn A{depth}(){{Print(0);}}fn Main() with no_io{{A0();}}"
        ));
        let Err(super::super::lower::CompileError::Rejected(errors)) =
            crate::ir::check_scalar("witness-ladder.n", &source)
        else {
            panic!("the written no_io restriction must reject the effect");
        };
        let effect = errors
            .iter()
            .find(|diagnostic| diagnostic.code == "NX-E0415")
            .expect("known IO effect");
        assert_eq!(effect.cause_chain.len(), 42);
    }
}
