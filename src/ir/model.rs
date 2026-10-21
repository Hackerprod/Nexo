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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Ty {
    I64,
    Bool,
    Unit,
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
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Op {
    ConstI64(i64),
    ConstBool(bool),
    ConstUnit,
    CopyValue(LocalId),
    Binary {
        op: Binary,
        left: TempId,
        right: TempId,
    },
    WritePlace {
        local: LocalId,
        value: TempId,
    },
    CallIntrinsic {
        intrinsic: Intrinsic,
        args: Vec<TempId>,
    },
    DropValue(TempId),
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
        if replacement.result != slot.result {
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
        self.functions.replace_for_development(name, replacement)?;
        crate::ir::verify::verify_program(self)
    }
}

/// FNV-1a over a canonical, span-free rendering of the function's semantic IR.
pub(crate) fn content_hash(f: &Function) -> u64 {
    fn number(bytes: &mut Vec<u8>, n: usize) {
        bytes.extend_from_slice(&(n as u64).to_le_bytes())
    }
    fn ty(t: Ty) -> u8 {
        match t {
            Ty::I64 => 1,
            Ty::Bool => 2,
            Ty::Unit => 3,
        }
    }
    let mut bytes = vec![1, ty(f.result)];
    number(&mut bytes, f.locals.len());
    for local in &f.locals {
        number(&mut bytes, local.id.0);
        bytes.push(ty(local.ty));
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
            Op::CallIntrinsic { intrinsic, args } => {
                bytes.push(7);
                bytes.push(match intrinsic {
                    Intrinsic::Print => 1,
                    Intrinsic::AssertEq => 2,
                    Intrinsic::DropData => 3,
                    Intrinsic::Fail => 4,
                });
                number(&mut bytes, args.len());
                for id in args {
                    number(&mut bytes, id.0)
                }
            }
            Op::DropValue(id) => {
                bytes.push(8);
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
        bytes.push(ins.ty.map_or(0, ty));
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
