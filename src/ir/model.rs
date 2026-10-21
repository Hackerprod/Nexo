//! Typed, source-originated linear IR. Later cuts extend the block graph.
use crate::diagnostic::Span;

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
    I64,
    Bool,
    Unit,
    Array(Box<Ty>),
    MockResource,
    MockBlockingResource,
}
impl Ty {
    pub fn copyable(&self) -> bool {
        match self {
            Self::MockResource | Self::MockBlockingResource => false,
            Self::Array(inner) => inner.copyable(),
            _ => true,
        }
    }
    pub fn invalid_resource_array(&self) -> bool {
        let mut node = self;
        let mut within_array = false;
        loop {
            match node {
                Self::Array(inner) => {
                    within_array = true;
                    node = inner;
                }
                Self::MockResource | Self::MockBlockingResource => return within_array,
                _ => return false,
            }
        }
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
        loan: LoanId,
        origin_span: Span,
        permission: LoanPermission,
        /// An inout parameter is the parent permission for a reborrow.
        parent: Option<LocalId>,
    },
    /// Marks the end of the callee cleanup and release of a call argument loan.
    EndLoan(LoanId),
    BuildArray(Vec<TempId>),
    ReadIndex {
        local: LocalId,
        indexes: Vec<TempId>,
    },
    BorrowIndex {
        local: LocalId,
        indexes: Vec<TempId>,
    },
    CheckIndex {
        local: LocalId,
        indexes: Vec<TempId>,
    },
    IndexValue {
        base: TempId,
        index: TempId,
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
    pub tick: bool,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Terminator {
    Return { value: Option<TempId>, span: Span },
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BasicBlock {
    pub id: BlockId,
    pub instructions: Vec<Instruction>,
    pub terminator: Terminator,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Local {
    pub id: LocalId,
    pub name: String,
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
    pub result: Ty,
    pub cleanup: Vec<LocalId>,
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

/// FNV-1a over a canonical, span-free rendering of the function's semantic IR.
pub(crate) fn content_hash(f: &Function) -> u64 {
    fn number(bytes: &mut Vec<u8>, n: usize) {
        bytes.extend_from_slice(&(n as u64).to_le_bytes())
    }
    fn ty(bytes: &mut Vec<u8>, t: &Ty) {
        let mut node = t;
        loop {
            match node {
                Ty::I64 => {
                    bytes.push(1);
                    break;
                }
                Ty::Bool => {
                    bytes.push(2);
                    break;
                }
                Ty::Unit => {
                    bytes.push(3);
                    break;
                }
                Ty::Array(child) => {
                    bytes.push(4);
                    node = child;
                }
                Ty::MockResource => {
                    bytes.push(5);
                    break;
                }
                Ty::MockBlockingResource => {
                    bytes.push(6);
                    break;
                }
            }
        }
    }
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
    number(&mut bytes, f.block.instructions.len());
    for ins in &f.block.instructions {
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
            Op::ReadIndex { local, indexes } => {
                bytes.push(11);
                number(&mut bytes, local.0);
                number(&mut bytes, indexes.len());
                for id in indexes {
                    number(&mut bytes, id.0);
                }
            }
            Op::BorrowIndex { local, indexes } => {
                bytes.push(16);
                number(&mut bytes, local.0);
                number(&mut bytes, indexes.len());
                for id in indexes {
                    number(&mut bytes, id.0);
                }
            }
            Op::CheckIndex { local, indexes } => {
                bytes.push(15);
                number(&mut bytes, local.0);
                number(&mut bytes, indexes.len());
                for id in indexes {
                    number(&mut bytes, id.0);
                }
            }
            Op::IndexValue { base, index } => {
                bytes.push(12);
                number(&mut bytes, base.0);
                number(&mut bytes, index.0);
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
            } => {
                bytes.push(13);
                number(&mut bytes, local.0);
                number(&mut bytes, indexes.len());
                for id in indexes {
                    number(&mut bytes, id.0);
                }
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
        bytes.push(u8::from(ins.tick));
    }
    number(&mut bytes, f.cleanup.len());
    for id in &f.cleanup {
        number(&mut bytes, id.0)
    }
    match f.block.terminator_value() {
        Some(id) => {
            bytes.push(1);
            number(&mut bytes, id.0)
        }
        None => bytes.push(0),
    }
    bytes.into_iter().fold(0xcbf29ce484222325u64, |h, b| {
        (h ^ u64::from(b)).wrapping_mul(0x100000001b3)
    })
}
impl BasicBlock {
    fn terminator_value(&self) -> Option<TempId> {
        match &self.terminator {
            Terminator::Return { value, .. } => *value,
        }
    }
}
