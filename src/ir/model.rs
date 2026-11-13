//! Typed, source-originated linear IR. Later cuts extend the block graph.
use crate::diagnostic::Span;
use crate::union::{UnionDescriptor, UnionKind, VariantTag};

/// Stable canonical identity. The function table position is a separate index.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct FunctionId(pub String);
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct BlockId(pub usize);
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct LocalId(pub usize);
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct TempId(pub usize);
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct TypeId(pub usize);
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct LoanId(pub usize);
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ScopeId(pub usize);
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct OriginId(pub usize);

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Ty {
    /// Internal absence of a normal continuation; never appears in emitted IR.
    Never,
    I64,
    Bool,
    Unit,
    Array(Box<Ty>),
    Result(Box<Ty>, Box<Ty>),
    MockResource,
    MockBlockingResource,
}
impl Ty {
    pub fn union_descriptor(&self) -> Option<UnionDescriptor<&Ty>> {
        match self {
            Self::Result(ok, err) => Some(UnionDescriptor::result(ok, err)),
            _ => None,
        }
    }
    pub fn has_resource_destructor(&self) -> bool {
        match self {
            Self::MockResource | Self::MockBlockingResource => true,
            Self::Result(ok, err) => ok.has_resource_destructor() || err.has_resource_destructor(),
            Self::Array(inner) => inner.has_resource_destructor(),
            Self::Never | Self::I64 | Self::Bool | Self::Unit => false,
        }
    }

    pub fn copyable(&self) -> bool {
        match self {
            Self::MockResource | Self::MockBlockingResource => false,
            Self::Array(inner) => inner.copyable(),
            Self::Result(ok, err) => ok.copyable() && err.copyable(),
            _ => true,
        }
    }
    pub fn invalid_resource_array(&self) -> bool {
        let mut pending = vec![(self, false)];
        while let Some((node, within_array)) = pending.pop() {
            match node {
                Self::Array(inner) => pending.push((inner, true)),
                Self::Result(ok, err) => {
                    pending.push((ok, within_array));
                    pending.push((err, within_array));
                }
                Self::MockResource | Self::MockBlockingResource if within_array => return true,
                _ => {}
            }
        }
        false
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Access {
    Own,
    Read,
    Write,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Binary {
    Add,
    Sub,
    Less,
    Equal,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Intrinsic {
    Print,
    AssertEq,
    DropData,
    Fail,
    Len,
    MockOpen,
    MockBlockingOpen,
    Inspect,
    TryAdd,
    MockUnknown,
    MockReadRecords,
    MockWriteI64,
    MockRequest,
    MockReply,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ParameterMode {
    Read,
    Consume,
    Inout,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LoanPermission {
    Read,
    Inout,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Parameter {
    pub local: LocalId,
    pub mode: ParameterMode,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Op {
    ConstI64(i64),
    ConstBool(bool),
    ConstUnit,
    CopyValue(LocalId),
    TransferValue(LocalId),
    BorrowValue(LocalId),
    /// A call argument loan. Its root remains stable even when indexes are evaluated.
    BeginLoan {
        root: LocalId,
        indexes: Vec<TempId>,
        site: IndexSite,
        loan: LoanId,
        origin_span: Span,
        permission: LoanPermission,
        /// An inout parameter is the parent permission for a reborrow.
        parent: Option<LocalId>,
    },
    /// Marks the end of the callee cleanup and release of a call argument loan.
    EndLoan(LoanId),
    BuildArray(Vec<TempId>),
    BuildVariant {
        tag: VariantTag,
        payload: Option<TempId>,
    },
    /// Selects the active variant. On Err the current frame returns
    /// Result<U,E> through its ordinary cleanup path.
    BranchVariant {
        value: TempId,
        result: Ty,
    },
    ReadIndex {
        local: LocalId,
        indexes: Vec<TempId>,
        site: IndexSite,
    },
    BorrowIndex {
        local: LocalId,
        indexes: Vec<TempId>,
        site: IndexSite,
    },
    CheckIndex {
        local: LocalId,
        indexes: Vec<TempId>,
        site: IndexSite,
    },
    IndexValue {
        base: TempId,
        index: TempId,
        site: IndexSite,
    },
    Binary {
        op: Binary,
        left: TempId,
        right: TempId,
    },
    WritePlace {
        local: LocalId,
        value: TempId,
    },
    WriteIndex {
        local: LocalId,
        indexes: Vec<TempId>,
        site: IndexSite,
        value: TempId,
    },
    CallIntrinsic {
        intrinsic: Intrinsic,
        args: Vec<TempId>,
    },
    CallDirect {
        function: FunctionId,
        /// Written order. Each entry carries its declared parameter slot.
        args: Vec<(usize, TempId)>,
        signature: Vec<(String, ParameterMode, Ty)>,
    },
    DropValue(TempId),
    DropBinding(LocalId),
    DropLocal(LocalId),
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Instruction {
    pub op: Op,
    pub output: Option<TempId>,
    pub ty: Option<Ty>,
    pub span: Span,
    pub scope: ScopeId,
    pub origin: OriginId,
    pub access: Access,
    pub effect_io: bool,
    pub charges: Vec<WorkCharge>,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WorkCharge {
    pub ticks: u64,
    pub origin: Span,
}
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct IndexSite {
    pub context: Span,
    pub selectors: Vec<Span>,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Terminator {
    Return {
        value: Option<TempId>,
        span: Span,
        charges: Vec<WorkCharge>,
    },
    Jump {
        target: BlockId,
        span: Span,
        charges: Vec<WorkCharge>,
    },
    BranchBool {
        condition: TempId,
        yes: BlockId,
        no: BlockId,
        span: Span,
        charges: Vec<WorkCharge>,
    },
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BasicBlock {
    pub id: BlockId,
    pub instructions: Vec<Instruction>,
    pub terminator: Terminator,
}

pub(crate) fn is_drop_data_call(block: &BasicBlock, index: usize) -> bool {
    let Some(ins) = block.instructions.get(index) else {
        return false;
    };
    let Op::CallIntrinsic {
        intrinsic: Intrinsic::DropData,
        args,
    } = &ins.op
    else {
        return false;
    };
    let Some(arg) = args.first() else {
        return false;
    };
    index
        .checked_sub(1)
        .and_then(|previous| block.instructions.get(previous))
        .is_some_and(|ins| matches!(ins.op, Op::TransferValue(_)) && ins.output == Some(*arg))
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Local {
    pub id: LocalId,
    pub name: String,
    pub name_span: Span,
    pub ty: Ty,
    pub mutable: bool,
    pub scope: ScopeId,
    pub parameter_mode: Option<ParameterMode>,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Function {
    pub id: FunctionId,
    pub table_index: usize,
    pub name: String,
    pub source_file: String,
    pub source_text: std::sync::Arc<str>,
    pub scope_count: usize,
    pub scope_parents: Vec<Option<ScopeId>>,
    pub content_hash: u64,
    pub locals: Vec<Local>,
    pub parameters: Vec<Parameter>,
    pub block: BasicBlock,
    /// Additional CFG blocks. The entry block remains `block` for linear artifacts.
    pub blocks: Vec<BasicBlock>,
    pub result: Ty,
    pub cleanup: Vec<LocalId>,
    /// Filled by the post-ownership effects pass before an artifact is sealed.
    pub effects: Option<EffectSummary>,
    /// Control-flow exits recorded by lowering and consumed by effect analysis.
    pub exit_info: Vec<ExitInfo>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct EffectSummary {
    pub known: u8,
    pub unknown: u8,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ExitClass {
    Fallthrough,
    Return,
    Try,
    Fail,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExitInfo {
    pub class: ExitClass,
    pub block: BlockId,
    /// Instruction index for operation exits; `instructions.len()` for terminators.
    pub index: usize,
    pub span: Span,
    pub live_locals: Vec<LocalId>,
    pub definitely_dropped: Vec<LocalId>,
}
#[derive(Clone, Debug)]
pub struct FunctionTable {
    pub(crate) entries: Vec<Function>,
}
impl FunctionTable {
    pub fn function(&self, name: &str) -> Option<&Function> {
        self.entries.iter().find(|f| f.name == name)
    }
    pub fn functions(&self) -> &[Function] {
        &self.entries
    }
    pub(crate) fn replace_for_development(
        &mut self,
        name: &str,
        replacement: Function,
    ) -> Result<(), &'static str> {
        let Some(slot) = self.entries.iter_mut().find(|f| f.name == name) else {
            return Err("unknown function");
        };
        if replacement.name != name || replacement.id != slot.id {
            return Err("identity mismatch");
        }
        if replacement.result != slot.result
            || replacement.parameters.len() != slot.parameters.len()
            || replacement
                .parameters
                .iter()
                .zip(&slot.parameters)
                .any(|(a, b)| {
                    a.mode != b.mode
                        || replacement.locals[a.local.0].ty != slot.locals[b.local.0].ty
                        || replacement.locals[a.local.0].name != slot.locals[b.local.0].name
                })
        {
            return Err("signature mismatch");
        }
        let mut replacement = replacement;
        replacement.table_index = slot.table_index;
        *slot = replacement;
        Ok(())
    }
}
#[derive(Clone, Debug)]
pub struct Program {
    pub(crate) functions: FunctionTable,
    pub(crate) profile: &'static str,
}
impl Program {
    pub fn functions(&self) -> &FunctionTable {
        &self.functions
    }
    pub fn profile(&self) -> &str {
        self.profile
    }
}

impl Program {
    /// Development hook: swap a function from another verified artifact.
    /// The receiver is verified again at execution; source callers cannot inject raw IR.
    pub fn substitute_for_development(
        &mut self,
        name: &str,
        donor: &Program,
    ) -> Result<(), &'static str> {
        crate::ir::verify::verify_program(donor)?;
        let replacement = donor
            .functions
            .function(name)
            .ok_or("unknown donor function")?
            .clone();
        let previous = self
            .functions
            .function(name)
            .ok_or("unknown function")?
            .clone();
        self.functions.replace_for_development(name, replacement)?;
        if let Err(reason) = crate::ir::verify::verify_program(self) {
            self.functions
                .replace_for_development(name, previous)
                .map_err(|_| "substitution rollback failed")?;
            return Err(reason);
        }
        Ok(())
    }
}

/// FNV-1a over canonical semantic IR and source-relative work-charge spans.
pub(crate) fn content_hash(f: &Function) -> u64 {
    fn number(bytes: &mut Vec<u8>, n: usize) {
        bytes.extend_from_slice(&(n as u64).to_le_bytes())
    }
    fn ty(bytes: &mut Vec<u8>, t: &Ty) {
        enum Item<'a> {
            Type(&'a Ty),
            End,
        }
        let mut pending = vec![Item::Type(t)];
        while let Some(item) = pending.pop() {
            match item {
                Item::End => bytes.push(0),
                Item::Type(Ty::Never) => bytes.push(7),
                Item::Type(Ty::I64) => bytes.push(1),
                Item::Type(Ty::Bool) => bytes.push(2),
                Item::Type(Ty::Unit) => bytes.push(3),
                Item::Type(Ty::MockResource) => bytes.push(5),
                Item::Type(Ty::MockBlockingResource) => bytes.push(6),
                Item::Type(Ty::Array(child)) => {
                    bytes.push(4);
                    pending.push(Item::End);
                    pending.push(Item::Type(child));
                }
                Item::Type(Ty::Result(ok, err)) => {
                    bytes.push(8);
                    pending.push(Item::End);
                    pending.push(Item::Type(err));
                    pending.push(Item::Type(ok));
                }
            }
        }
    }
    fn hash_span(bytes: &mut Vec<u8>, span: &Span, base: u64) {
        number(bytes, span.start.saturating_sub(base) as usize);
        number(bytes, span.end.saturating_sub(base) as usize);
    }
    fn index_site(bytes: &mut Vec<u8>, site: &IndexSite, base: u64) {
        hash_span(bytes, &site.context, base);
        number(bytes, site.selectors.len());
        for selector in &site.selectors {
            hash_span(bytes, selector, base);
        }
    }
    let source_base = f.source_text.find(&format!("fn {}", f.name)).unwrap_or(0) as u64;
    let mut bytes = vec![1];
    ty(&mut bytes, &f.result);
    number(&mut bytes, f.parameters.len());
    for p in &f.parameters {
        number(&mut bytes, p.local.0);
        let name = &f.locals[p.local.0].name;
        number(&mut bytes, name.len());
        bytes.extend_from_slice(name.as_bytes());
        bytes.push(match p.mode {
            ParameterMode::Read => 1,
            ParameterMode::Consume => 2,
            ParameterMode::Inout => 3,
        });
        ty(&mut bytes, &f.locals[p.local.0].ty);
    }
    number(&mut bytes, f.locals.len());
    for local in &f.locals {
        number(&mut bytes, local.id.0);
        ty(&mut bytes, &local.ty);
        bytes.push(u8::from(local.mutable));
        number(&mut bytes, local.scope.0);
    }
    if !f.blocks.is_empty() {
        number(&mut bytes, f.blocks.len() + 1);
    }
    for block in std::iter::once(&f.block).chain(&f.blocks) {
        if !f.blocks.is_empty() {
            number(&mut bytes, block.id.0);
        }
        number(&mut bytes, block.instructions.len());
        for ins in &block.instructions {
            match &ins.op {
                Op::ConstI64(n) => {
                    bytes.push(1);
                    bytes.extend_from_slice(&n.to_le_bytes())
                }
                Op::ConstBool(b) => {
                    bytes.push(2);
                    bytes.push(u8::from(*b))
                }
                Op::ConstUnit => bytes.push(3),
                Op::CopyValue(id) => {
                    bytes.push(4);
                    number(&mut bytes, id.0)
                }
                Op::TransferValue(id) => {
                    bytes.push(18);
                    number(&mut bytes, id.0);
                }
                Op::BorrowValue(id) => {
                    bytes.push(14);
                    number(&mut bytes, id.0);
                }
                Op::BeginLoan {
                    root,
                    indexes,
                    site,
                    permission,
                    parent,
                    ..
                } => {
                    bytes.push(20);
                    number(&mut bytes, root.0);
                    number(&mut bytes, indexes.len());
                    for id in indexes {
                        number(&mut bytes, id.0);
                    }
                    index_site(&mut bytes, site, source_base);
                    bytes.push(match permission {
                        LoanPermission::Read => 1,
                        LoanPermission::Inout => 2,
                    });
                    match parent {
                        Some(id) => {
                            bytes.push(1);
                            number(&mut bytes, id.0);
                        }
                        None => bytes.push(0),
                    }
                }
                Op::EndLoan(_) => {
                    bytes.push(21);
                }
                Op::BuildArray(items) => {
                    bytes.push(10);
                    number(&mut bytes, items.len());
                    for id in items {
                        number(&mut bytes, id.0);
                    }
                }
                Op::BuildVariant { tag, payload } => {
                    bytes.push(22);
                    // Preserve the published A2 hash encoding for existing Result IR.
                    bytes.push(
                        if matches!(
                            ins.ty.as_ref().and_then(Ty::union_descriptor),
                            Some(union) if union.kind == UnionKind::Result
                        ) {
                            tag.0 ^ 1
                        } else {
                            tag.0
                        },
                    );
                    if let Some(payload) = payload {
                        number(&mut bytes, payload.0);
                    }
                }
                Op::BranchVariant { value, result } => {
                    bytes.push(23);
                    number(&mut bytes, value.0);
                    ty(&mut bytes, result);
                }
                Op::ReadIndex {
                    local,
                    indexes,
                    site,
                } => {
                    bytes.push(11);
                    number(&mut bytes, local.0);
                    number(&mut bytes, indexes.len());
                    for id in indexes {
                        number(&mut bytes, id.0);
                    }
                    index_site(&mut bytes, site, source_base);
                }
                Op::BorrowIndex {
                    local,
                    indexes,
                    site,
                } => {
                    bytes.push(16);
                    number(&mut bytes, local.0);
                    number(&mut bytes, indexes.len());
                    for id in indexes {
                        number(&mut bytes, id.0);
                    }
                    index_site(&mut bytes, site, source_base);
                }
                Op::CheckIndex {
                    local,
                    indexes,
                    site,
                } => {
                    bytes.push(15);
                    number(&mut bytes, local.0);
                    number(&mut bytes, indexes.len());
                    for id in indexes {
                        number(&mut bytes, id.0);
                    }
                    index_site(&mut bytes, site, source_base);
                }
                Op::IndexValue { base, index, site } => {
                    bytes.push(12);
                    number(&mut bytes, base.0);
                    number(&mut bytes, index.0);
                    index_site(&mut bytes, site, source_base);
                }
                Op::Binary { op, left, right } => {
                    bytes.push(5);
                    bytes.push(match op {
                        Binary::Add => 1,
                        Binary::Sub => 2,
                        Binary::Less => 3,
                        Binary::Equal => 4,
                    });
                    number(&mut bytes, left.0);
                    number(&mut bytes, right.0)
                }
                Op::WritePlace { local, value } => {
                    bytes.push(6);
                    number(&mut bytes, local.0);
                    number(&mut bytes, value.0)
                }
                Op::WriteIndex {
                    local,
                    indexes,
                    value,
                    site,
                } => {
                    bytes.push(13);
                    number(&mut bytes, local.0);
                    number(&mut bytes, indexes.len());
                    for id in indexes {
                        number(&mut bytes, id.0);
                    }
                    index_site(&mut bytes, site, source_base);
                    number(&mut bytes, value.0);
                }
                Op::CallIntrinsic { intrinsic, args } => {
                    bytes.push(7);
                    bytes.push(match intrinsic {
                        Intrinsic::Print => 1,
                        Intrinsic::AssertEq => 2,
                        Intrinsic::DropData => 3,
                        Intrinsic::Fail => 4,
                        Intrinsic::Len => 5,
                        Intrinsic::MockOpen => 6,
                        Intrinsic::MockBlockingOpen => 7,
                        Intrinsic::Inspect => 8,
                        Intrinsic::TryAdd => 9,
                        Intrinsic::MockUnknown => 10,
                        Intrinsic::MockReadRecords => 11,
                        Intrinsic::MockWriteI64 => 12,
                        Intrinsic::MockRequest => 13,
                        Intrinsic::MockReply => 14,
                    });
                    number(&mut bytes, args.len());
                    for id in args {
                        number(&mut bytes, id.0)
                    }
                }
                Op::CallDirect {
                    function,
                    args,
                    signature,
                } => {
                    bytes.push(19);
                    number(&mut bytes, function.0.len());
                    bytes.extend_from_slice(function.0.as_bytes());
                    number(&mut bytes, signature.len());
                    for (name, mode, ty_value) in signature {
                        number(&mut bytes, name.len());
                        bytes.extend_from_slice(name.as_bytes());
                        bytes.push(match mode {
                            ParameterMode::Read => 1,
                            ParameterMode::Consume => 2,
                            ParameterMode::Inout => 3,
                        });
                        ty(&mut bytes, ty_value);
                    }
                    number(&mut bytes, args.len());
                    for (slot, id) in args {
                        number(&mut bytes, *slot);
                        number(&mut bytes, id.0);
                    }
                }
                Op::DropValue(id) => {
                    bytes.push(8);
                    number(&mut bytes, id.0)
                }
                Op::DropBinding(id) => {
                    bytes.push(17);
                    number(&mut bytes, id.0)
                }
                Op::DropLocal(id) => {
                    bytes.push(9);
                    number(&mut bytes, id.0)
                }
            }
            match ins.output {
                Some(id) => {
                    bytes.push(1);
                    number(&mut bytes, id.0)
                }
                None => bytes.push(0),
            }
            match &ins.ty {
                Some(t) => ty(&mut bytes, t),
                None => bytes.push(0),
            }
            bytes.push(match ins.access {
                Access::Own => 1,
                Access::Read => 2,
                Access::Write => 3,
            });
            bytes.push(u8::from(ins.effect_io));
            bytes.extend_from_slice(&(ins.charges.len() as u64).to_le_bytes());
            for charge in &ins.charges {
                bytes.extend_from_slice(&charge.ticks.to_le_bytes());
                bytes.extend_from_slice(&(charge.origin.file.len() as u64).to_le_bytes());
                bytes.extend_from_slice(charge.origin.file.as_bytes());
                bytes.extend_from_slice(
                    &charge
                        .origin
                        .start
                        .saturating_sub(source_base)
                        .to_le_bytes(),
                );
                bytes.extend_from_slice(
                    &charge.origin.end.saturating_sub(source_base).to_le_bytes(),
                );
            }
        }
        if !f.blocks.is_empty() {
            match &block.terminator {
                Terminator::Return { value, .. } => {
                    bytes.push(1);
                    match value {
                        Some(id) => {
                            bytes.push(1);
                            number(&mut bytes, id.0);
                        }
                        None => bytes.push(0),
                    }
                }
                Terminator::Jump { target, .. } => {
                    bytes.push(2);
                    number(&mut bytes, target.0);
                }
                Terminator::BranchBool {
                    condition, yes, no, ..
                } => {
                    bytes.push(3);
                    number(&mut bytes, condition.0);
                    number(&mut bytes, yes.0);
                    number(&mut bytes, no.0);
                }
            }
        }
        let charges = match &block.terminator {
            Terminator::Return { charges, .. }
            | Terminator::Jump { charges, .. }
            | Terminator::BranchBool { charges, .. } => charges,
        };
        bytes.extend_from_slice(&(charges.len() as u64).to_le_bytes());
        for charge in charges {
            bytes.extend_from_slice(&charge.ticks.to_le_bytes());
            bytes.extend_from_slice(&(charge.origin.file.len() as u64).to_le_bytes());
            bytes.extend_from_slice(charge.origin.file.as_bytes());
            bytes.extend_from_slice(
                &charge
                    .origin
                    .start
                    .saturating_sub(source_base)
                    .to_le_bytes(),
            );
            bytes.extend_from_slice(&charge.origin.end.saturating_sub(source_base).to_le_bytes());
        }
    }
    number(&mut bytes, f.cleanup.len());
    for id in &f.cleanup {
        number(&mut bytes, id.0)
    }
    if f.blocks.is_empty() {
        match &f.block.terminator {
            Terminator::Return {
                value: Some(id), ..
            } => {
                bytes.push(1);
                number(&mut bytes, id.0);
            }
            _ => bytes.push(0),
        }
    }
    bytes.into_iter().fold(0xcbf29ce484222325u64, |h, b| {
        (h ^ u64::from(b)).wrapping_mul(0x100000001b3)
    })
}
