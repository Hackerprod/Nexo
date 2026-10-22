use super::model::{
    Access, Binary, Function, Intrinsic, LoanPermission, Op, ParameterMode, Program, TempId,
    Terminator, Ty,
};

pub(crate) fn verify_program(program: &Program) -> Result<(), &'static str> {
    if program.profile != crate::diagnostic::PROFILE_ID {
        return Err("profile");
    }
    let mut names = std::collections::HashSet::with_capacity(program.functions.entries.len());
    for (index, function) in program.functions.entries.iter().enumerate() {
        if function.table_index != index || function.id.0 != function.name {
            return Err("function identity");
        }
        if !names.insert(&function.name) {
            return Err("duplicate function");
        }
        verify_function(function)?;
    }
    for function in &program.functions.entries {
        for ins in std::iter::once(&function.block)
            .chain(&function.blocks)
            .flat_map(|b| &b.instructions)
        {
            if let Op::CallDirect {
                function: callee,
                signature,
                ..
            } = &ins.op
            {
                let target = program
                    .functions
                    .function(&callee.0)
                    .ok_or("unknown callee")?;
                if signature.len() != target.parameters.len()
                    || ins.ty.as_ref() != Some(&target.result)
                {
                    return Err("callee signature");
                }
                for ((name, mode, ty), param) in signature.iter().zip(&target.parameters) {
                    if *mode != param.mode
                        || *ty != target.locals[param.local.0].ty
                        || *name != target.locals[param.local.0].name
                    {
                        return Err("callee signature");
                    }
                }
            }
        }
    }
    Ok(())
}
pub(crate) fn verify_function(function: &Function) -> Result<(), &'static str> {
    if function.id.0 != function.name || function.scope_count == 0 {
        return Err("function identity");
    }
    if function.scope_count != function.scope_parents.len()
        || function.scope_parents.first() != Some(&None)
        || function
            .scope_parents
            .iter()
            .enumerate()
            .skip(1)
            .any(|(id, parent)| !parent.is_some_and(|p| p.0 < id))
    {
        return Err("scope table");
    }
    if function.block.id.0 != 0 {
        return Err("block identity");
    }
    for (i, local) in function.locals.iter().enumerate() {
        if local.id.0 != i {
            return Err("local identity");
        }
        if local.scope.0 >= function.scope_count {
            return Err("local scope");
        }
        if local.ty.invalid_resource_array() {
            return Err("resource array local");
        }
    }
    if function.result.invalid_resource_array() {
        return Err("resource array result");
    }
    for (index, p) in function.parameters.iter().enumerate() {
        if p.local.0 != index
            || function.locals.get(index).and_then(|l| l.parameter_mode) != Some(p.mode)
            || function.locals[index].scope.0 != 0
            || function.locals[index].mutable != (p.mode == ParameterMode::Inout)
        {
            return Err("parameter layout");
        }
    }
    if function
        .locals
        .iter()
        .skip(function.parameters.len())
        .any(|l| l.parameter_mode.is_some())
    {
        return Err("parameter layout");
    }
    if super::model::content_hash(function) != function.content_hash {
        return Err("content hash");
    }
    if function.cleanup
        != function
            .locals
            .iter()
            .rev()
            .map(|l| l.id)
            .collect::<Vec<_>>()
    {
        return Err("cleanup plan");
    }
    if !function.blocks.is_empty() {
        return verify_cfg(function);
    }
    let mut temps = Vec::<Ty>::new();
    let mut live = Vec::<bool>::new();
    let mut borrowed = Vec::<bool>::new();
    let mut borrow_sources = Vec::<Option<super::model::LocalId>>::new();
    let mut active_borrows = vec![0usize; function.locals.len()];
    let mut active_exclusive = vec![0usize; function.locals.len()];
    let mut loan_permission = Vec::<Option<LoanPermission>>::new();
    let mut loan_ids = std::collections::HashSet::new();
    let mut loan_id_by_temp = Vec::<Option<super::model::LoanId>>::new();
    let mut pending_end = Vec::new();
    let mut initialized = vec![false; function.locals.len()];
    for p in &function.parameters {
        initialized[p.local.0] = true;
    }
    let take = |id: TempId,
                defined: &[Ty],
                live: &mut [bool],
                sources: &[Option<super::model::LocalId>],
                active: &mut [usize]|
     -> Result<Ty, &'static str> {
        let ty = defined.get(id.0).cloned().ok_or("undefined temporary")?;
        let available = live.get_mut(id.0).ok_or("temporary liveness")?;
        if !*available {
            return Err("temporary used twice");
        }
        *available = false;
        if let Some(Some(local)) = sources.get(id.0) {
            active[local.0] = active[local.0].checked_sub(1).ok_or("borrow count")?;
        }
        Ok(ty)
    };
    let valid_span = |span: &crate::diagnostic::Span| {
        span.file == function.source_file
            && span.start <= span.end
            && usize::try_from(span.start)
                .ok()
                .is_some_and(|start| function.source_text.is_char_boundary(start))
            && usize::try_from(span.end)
                .ok()
                .is_some_and(|end| function.source_text.is_char_boundary(end))
    };
    for (position, ins) in function.block.instructions.iter().enumerate() {
        if !pending_end.is_empty() && !matches!(ins.op, Op::EndLoan(_)) {
            return Err("loan end missing");
        }
        if !valid_span(&ins.span) {
            return Err("source origin");
        }
        if ins.origin.0 != position {
            return Err("origin");
        }
        if ins.scope.0 >= function.scope_count {
            return Err("instruction scope");
        }
        let expected = match &ins.op {
            Op::ConstI64(_) => Some(Ty::I64),
            Op::ConstBool(_) => Some(Ty::Bool),
            Op::ConstUnit => Some(Ty::Unit),
            Op::CopyValue(id) => {
                let local = function.locals.get(id.0).ok_or("local reference")?;
                if active_exclusive[id.0] != 0 {
                    return Err("read during exclusive loan");
                }
                if !initialized[id.0] {
                    return Err("uninitialized local");
                }
                if !local.ty.copyable() {
                    return Err("copy noncopyable local");
                }
                Some(local.ty.clone())
            }
            Op::TransferValue(id) => {
                let local = function.locals.get(id.0).ok_or("transfer local")?;
                if !initialized[id.0] {
                    return Err("transfer consumed local");
                }
                if matches!(
                    local.parameter_mode,
                    Some(ParameterMode::Read | ParameterMode::Inout)
                ) {
                    return Err("transfer read parameter");
                }
                if active_borrows[id.0] != 0 {
                    return Err("transfer while borrowed");
                }
                initialized[id.0] = false;
                Some(local.ty.clone())
            }
            Op::BorrowValue(id) => {
                let local = function.locals.get(id.0).ok_or("local reference")?;
                if !initialized[id.0] {
                    return Err("uninitialized local");
                }
                if active_exclusive[id.0] != 0 {
                    return Err("read during exclusive loan");
                }
                Some(local.ty.clone())
            }
            Op::BeginLoan {
                root,
                indexes,
                loan,
                origin_span,
                permission,
                parent,
            } => {
                let local = function.locals.get(root.0).ok_or("loan root")?;
                if !initialized[root.0] {
                    return Err("loan unavailable root");
                }
                if !valid_span(origin_span)
                    || origin_span.start as usize != loan.0
                    || origin_span.start > ins.span.start
                    || origin_span.end < ins.span.end
                    || !loan_ids.insert(loan.0)
                {
                    return Err("loan identity");
                }
                if *parent != (local.parameter_mode == Some(ParameterMode::Inout)).then_some(*root)
                {
                    return Err("loan parent");
                }
                if *permission == LoanPermission::Inout {
                    if !local.mutable || local.parameter_mode == Some(ParameterMode::Read) {
                        return Err("exclusive loan permission");
                    }
                    if active_borrows[root.0] != 0 {
                        return Err("overlapping loan");
                    }
                } else if active_exclusive[root.0] != 0 {
                    return Err("read during exclusive loan");
                }
                let mut ty = local.ty.clone();
                for id in indexes {
                    if take(*id, &temps, &mut live, &borrow_sources, &mut active_borrows)?
                        != Ty::I64
                    {
                        return Err("loan index type");
                    }
                    let Ty::Array(child) = ty else {
                        return Err("loan index base");
                    };
                    ty = *child;
                }
                Some(ty)
            }
            Op::EndLoan(loan) => {
                if pending_end.pop() != Some(loan.0) {
                    return Err("unmatched loan end");
                }
                None
            }
            Op::BuildArray(items) => {
                let Some(Ty::Array(element)) = &ins.ty else {
                    return Err("array result type");
                };
                if !element.copyable() {
                    return Err("resource array");
                }
                for id in items {
                    if borrowed.get(id.0) == Some(&true) {
                        return Err("array borrowed element");
                    }
                    if take(*id, &temps, &mut live, &borrow_sources, &mut active_borrows)?
                        != **element
                    {
                        return Err("array element type");
                    }
                }
                ins.ty.clone()
            }
            Op::ReadIndex { local, indexes } | Op::BorrowIndex { local, indexes } => {
                let local = function.locals.get(local.0).ok_or("read index local")?;
                if active_exclusive[local.id.0] != 0 {
                    return Err("read during exclusive loan");
                }
                if !initialized[local.id.0] {
                    return Err("read index uninitialized local");
                }
                if indexes.is_empty() {
                    return Err("empty index path");
                }
                let mut ty = local.ty.clone();
                for index in indexes {
                    if take(
                        *index,
                        &temps,
                        &mut live,
                        &borrow_sources,
                        &mut active_borrows,
                    )? != Ty::I64
                    {
                        return Err("index type");
                    }
                    let Ty::Array(child) = ty else {
                        return Err("index base type");
                    };
                    ty = *child;
                }
                Some(ty)
            }
            Op::CheckIndex { local, indexes } => {
                let l = function.locals.get(local.0).ok_or("check index local")?;
                if active_exclusive[local.0] != 0 {
                    return Err("read during exclusive loan");
                }
                if !initialized[local.0] {
                    return Err("check index uninitialized local");
                }
                if indexes.is_empty() {
                    return Err("empty index path");
                }
                let mut ty = l.ty.clone();
                for id in indexes {
                    if !live.get(id.0).copied().unwrap_or(false) {
                        return Err("check index undefined temporary");
                    }
                    if temps.get(id.0) != Some(&Ty::I64) {
                        return Err("index type");
                    }
                    let Ty::Array(child) = ty else {
                        return Err("index base type");
                    };
                    ty = *child;
                }
                None
            }
            Op::IndexValue { base, index } => {
                if borrowed.get(base.0) == Some(&true) {
                    return Err("index borrowed base");
                }
                let base = take(
                    *base,
                    &temps,
                    &mut live,
                    &borrow_sources,
                    &mut active_borrows,
                )?;
                if take(
                    *index,
                    &temps,
                    &mut live,
                    &borrow_sources,
                    &mut active_borrows,
                )? != Ty::I64
                {
                    return Err("index type");
                }
                let Ty::Array(child) = base else {
                    return Err("index base type");
                };
                Some(*child)
            }
            Op::Binary { op, left, right } => {
                if *op != Binary::Equal
                    && (borrowed.get(left.0) == Some(&true) || borrowed.get(right.0) == Some(&true))
                {
                    return Err("borrowed arithmetic operand");
                }
                let l = take(
                    *left,
                    &temps,
                    &mut live,
                    &borrow_sources,
                    &mut active_borrows,
                )?;
                let r = take(
                    *right,
                    &temps,
                    &mut live,
                    &borrow_sources,
                    &mut active_borrows,
                )?;
                if l != r {
                    return Err("operand type");
                }
                match op {
                    Binary::Add | Binary::Sub | Binary::Less if l == Ty::I64 => {
                        Some(if *op == Binary::Less {
                            Ty::Bool
                        } else {
                            Ty::I64
                        })
                    }
                    Binary::Equal => Some(Ty::Bool),
                    _ => return Err("binary domain"),
                }
            }
            Op::WritePlace { local, value } => {
                let l = function.locals.get(local.0).ok_or("write local")?;
                if active_borrows[local.0] != 0 {
                    return Err("write while borrowed");
                }
                if initialized[local.0] && !l.mutable {
                    return Err("immutable rewrite");
                }
                if borrowed.get(value.0) == Some(&true) {
                    return Err("write borrowed value");
                }
                if take(
                    *value,
                    &temps,
                    &mut live,
                    &borrow_sources,
                    &mut active_borrows,
                )? != l.ty
                {
                    return Err("write type");
                }
                initialized[local.0] = true;
                None
            }
            Op::WriteIndex {
                local,
                indexes,
                value,
            } => {
                let l = function.locals.get(local.0).ok_or("write index local")?;
                if active_borrows[local.0] != 0 {
                    return Err("write while borrowed");
                }
                if !initialized[local.0] {
                    return Err("write index uninitialized local");
                }
                if !l.mutable {
                    return Err("immutable index root");
                }
                if indexes.is_empty() {
                    return Err("empty index path");
                }
                let mut ty = l.ty.clone();
                for index in indexes {
                    if take(
                        *index,
                        &temps,
                        &mut live,
                        &borrow_sources,
                        &mut active_borrows,
                    )? != Ty::I64
                    {
                        return Err("index type");
                    }
                    let Ty::Array(child) = ty else {
                        return Err("index base type");
                    };
                    ty = *child;
                }
                if borrowed.get(value.0) == Some(&true) {
                    return Err("write borrowed value");
                }
                if take(
                    *value,
                    &temps,
                    &mut live,
                    &borrow_sources,
                    &mut active_borrows,
                )? != ty
                {
                    return Err("write index type");
                }
                None
            }
            Op::CallIntrinsic { intrinsic, args } => {
                let arity = match intrinsic {
                    Intrinsic::Print
                    | Intrinsic::DropData
                    | Intrinsic::Len
                    | Intrinsic::MockOpen
                    | Intrinsic::MockBlockingOpen
                    | Intrinsic::Inspect => 1,
                    Intrinsic::AssertEq => 2,
                    Intrinsic::Fail => 0,
                };
                if args.len() != arity {
                    return Err("intrinsic arity");
                }
                if let Some(first_id) = args.first() {
                    if *intrinsic == Intrinsic::DropData && borrowed.get(first_id.0) == Some(&true)
                    {
                        return Err("drop borrowed value");
                    }
                    let first = take(
                        *first_id,
                        &temps,
                        &mut live,
                        &borrow_sources,
                        &mut active_borrows,
                    )?;
                    if *intrinsic == Intrinsic::Len && !matches!(first, Ty::Array(_)) {
                        return Err("len domain");
                    }
                    if matches!(intrinsic, Intrinsic::MockOpen | Intrinsic::MockBlockingOpen)
                        && first != Ty::I64
                    {
                        return Err("open argument type");
                    }
                    if *intrinsic == Intrinsic::Inspect
                        && !matches!(first, Ty::MockResource | Ty::MockBlockingResource)
                    {
                        return Err("inspect argument type");
                    }
                    if *intrinsic == Intrinsic::Print && !first.copyable() {
                        return Err("print resource");
                    }
                    if *intrinsic == Intrinsic::AssertEq
                        && take(
                            args[1],
                            &temps,
                            &mut live,
                            &borrow_sources,
                            &mut active_borrows,
                        )? != first
                    {
                        return Err("assert type");
                    }
                }
                Some(match intrinsic {
                    Intrinsic::Len => Ty::I64,
                    Intrinsic::MockOpen => Ty::MockResource,
                    Intrinsic::MockBlockingOpen => Ty::MockBlockingResource,
                    _ => Ty::Unit,
                })
            }
            Op::CallDirect {
                args, signature, ..
            } => {
                if args.len() != signature.len() {
                    return Err("call arity");
                }
                let mut seen = vec![false; signature.len()];
                for (slot, id) in args {
                    let Some((_, mode, ty)) = signature.get(*slot) else {
                        return Err("call slot");
                    };
                    if seen[*slot] {
                        return Err("duplicate call slot");
                    }
                    seen[*slot] = true;
                    match mode {
                        ParameterMode::Consume if borrowed.get(id.0) == Some(&true) => {
                            return Err("consuming borrowed argument");
                        }
                        ParameterMode::Inout
                            if loan_permission.get(id.0) != Some(&Some(LoanPermission::Inout)) =>
                        {
                            return Err("inout argument permission");
                        }
                        ParameterMode::Read
                            if loan_permission.get(id.0) == Some(&Some(LoanPermission::Inout)) =>
                        {
                            return Err("exclusive passed as read");
                        }
                        _ => {}
                    }
                    if *mode == ParameterMode::Inout {
                        let root = borrow_sources
                            .get(id.0)
                            .and_then(|r| *r)
                            .ok_or("exclusive loan root")?;
                        active_exclusive[root.0] = active_exclusive[root.0]
                            .checked_sub(1)
                            .ok_or("exclusive loan count")?;
                    }
                    if let Some(Some(loan)) = loan_id_by_temp.get(id.0) {
                        pending_end.push(loan.0);
                    }
                    if take(*id, &temps, &mut live, &borrow_sources, &mut active_borrows)? != *ty {
                        return Err("call argument type");
                    }
                }
                ins.ty.clone()
            }
            Op::DropValue(id) => {
                if loan_permission.get(id.0) == Some(&Some(LoanPermission::Inout)) {
                    let root = borrow_sources
                        .get(id.0)
                        .and_then(|r| *r)
                        .ok_or("exclusive loan root")?;
                    active_exclusive[root.0] = active_exclusive[root.0]
                        .checked_sub(1)
                        .ok_or("exclusive loan count")?;
                }
                take(*id, &temps, &mut live, &borrow_sources, &mut active_borrows)?;
                None
            }
            Op::DropBinding(id) => {
                function.locals.get(id.0).ok_or("drop binding local")?;
                if !initialized[id.0] {
                    return Err("drop uninitialized local");
                }
                if active_borrows[id.0] != 0 {
                    return Err("drop while borrowed");
                }
                if matches!(
                    function.locals[id.0].parameter_mode,
                    Some(ParameterMode::Read | ParameterMode::Inout)
                ) {
                    return Err("drop read parameter");
                }
                initialized[id.0] = false;
                Some(Ty::Unit)
            }
            Op::DropLocal(id) => {
                function.locals.get(id.0).ok_or("drop local")?;
                if active_borrows[id.0] != 0 {
                    return Err("drop while borrowed");
                }
                if !initialized[id.0] {
                    return Err("drop uninitialized local");
                }
                initialized[id.0] = false;
                None
            }
        };
        if expected.as_ref() != ins.ty.as_ref() {
            return Err("instruction result type");
        }
        if ins.effect_io
            != matches!(
                ins.op,
                Op::CallIntrinsic {
                    intrinsic: Intrinsic::Print
                        | Intrinsic::MockOpen
                        | Intrinsic::MockBlockingOpen
                        | Intrinsic::Inspect,
                    ..
                }
            )
        {
            return Err("effect summary");
        }
        let access = match ins.op {
            Op::CopyValue(_)
            | Op::BorrowValue(_)
            | Op::BeginLoan { .. }
            | Op::ReadIndex { .. }
            | Op::BorrowIndex { .. }
            | Op::CheckIndex { .. } => Access::Read,
            Op::WritePlace { .. } | Op::WriteIndex { .. } => Access::Write,
            _ => Access::Own,
        };
        if ins.access != access || ins.tick != !matches!(ins.op, Op::EndLoan(_)) {
            return Err("access or source tick");
        }
        if let Some(ty) = expected {
            let id = ins.output.ok_or("missing output")?;
            if id.0 != temps.len() {
                return Err("temporary identity");
            }
            temps.push(ty);
            live.push(true);
            borrowed.push(matches!(
                ins.op,
                Op::BorrowValue(_) | Op::BorrowIndex { .. } | Op::BeginLoan { .. }
            ));
            let source = match ins.op {
                Op::BorrowValue(id) => Some(id),
                Op::BorrowIndex { local, .. } => Some(local),
                Op::BeginLoan { root, .. } => Some(root),
                _ => None,
            };
            if let Some(id) = source {
                active_borrows[id.0] = active_borrows[id.0].checked_add(1).ok_or("borrow count")?;
            }
            let permission = match ins.op {
                Op::BeginLoan {
                    permission, root, ..
                } => {
                    if permission == LoanPermission::Inout {
                        active_exclusive[root.0] += 1;
                    }
                    Some(permission)
                }
                Op::BorrowValue(_) | Op::BorrowIndex { .. } => Some(LoanPermission::Read),
                _ => None,
            };
            loan_permission.push(permission);
            loan_id_by_temp.push(match ins.op {
                Op::BeginLoan { loan, .. } => Some(loan),
                _ => None,
            });
            borrow_sources.push(source);
        } else if ins.output.is_some() {
            return Err("unexpected output");
        }
    }
    match &function.block.terminator {
        Terminator::Return {
            value: Some(value),
            span,
        } => {
            if !valid_span(span) {
                return Err("terminator origin");
            }
            if borrowed.get(value.0) == Some(&true) {
                return Err("borrowed return");
            }
            if take(
                *value,
                &temps,
                &mut live,
                &borrow_sources,
                &mut active_borrows,
            )? != function.result
            {
                return Err("return type");
            }
        }
        Terminator::Return { value: None, .. } => return Err("missing return value"),
        Terminator::Jump { .. } | Terminator::BranchBool { .. } => {
            return Err("unexpected control terminator");
        }
    }
    if live.iter().any(|v| *v) {
        return Err("unconsumed temporary");
    }
    if !pending_end.is_empty() {
        return Err("loan end missing");
    }
    if active_borrows.iter().any(|count| *count != 0)
        || active_exclusive.iter().any(|count| *count != 0)
    {
        return Err("unreleased loan");
    }
    Ok(())
}

/// Persistent definite-initialization bits: a branch changes only the paths
/// for locals it writes, and shared subtrees make joins proportional to changes.
enum InitNode {
    Uniform(bool),
    Branch(std::sync::Arc<Self>, std::sync::Arc<Self>),
}
struct InitTree {
    size: usize,
    yes_leaf: std::sync::Arc<InitNode>,
    no_leaf: std::sync::Arc<InitNode>,
}
impl InitTree {
    fn new(locals: usize) -> (Self, std::sync::Arc<InitNode>, std::sync::Arc<InitNode>) {
        let size = locals.max(1).next_power_of_two();
        let tree = Self {
            size,
            yes_leaf: std::sync::Arc::new(InitNode::Uniform(true)),
            no_leaf: std::sync::Arc::new(InitNode::Uniform(false)),
        };
        let yes = tree.yes_leaf.clone();
        let no = tree.no_leaf.clone();
        (tree, yes, no)
    }
    fn get(&self, root: &InitNode, id: usize) -> bool {
        let (mut node, mut lo, mut hi) = (root, 0, self.size);
        loop {
            super::count_cfg_work(1);
            match node {
                InitNode::Uniform(value) => return *value,
                InitNode::Branch(left, right) => {
                    let middle = (lo + hi) / 2;
                    if id < middle {
                        node = left;
                        hi = middle;
                    } else {
                        node = right;
                        lo = middle;
                    }
                }
            }
        }
    }
    fn set(
        &self,
        root: &std::sync::Arc<InitNode>,
        id: usize,
        value: bool,
    ) -> std::sync::Arc<InitNode> {
        self.set_at(root, 0, self.size, id, value)
    }
    fn set_at(
        &self,
        root: &std::sync::Arc<InitNode>,
        lo: usize,
        hi: usize,
        id: usize,
        value: bool,
    ) -> std::sync::Arc<InitNode> {
        super::count_cfg_work(1);
        if hi - lo == 1 {
            if self.get(root, id) == value {
                return root.clone();
            }
            return if value {
                self.yes_leaf.clone()
            } else {
                self.no_leaf.clone()
            };
        }
        let (left, right) = match root.as_ref() {
            InitNode::Uniform(_) => (root.clone(), root.clone()),
            InitNode::Branch(left, right) => (left.clone(), right.clone()),
        };
        let middle = (lo + hi) / 2;
        let (new_left, new_right) = if id < middle {
            (self.set_at(&left, lo, middle, id, value), right.clone())
        } else {
            (left.clone(), self.set_at(&right, middle, hi, id, value))
        };
        if std::sync::Arc::ptr_eq(&new_left, &left) && std::sync::Arc::ptr_eq(&new_right, &right) {
            root.clone()
        } else if let (InitNode::Uniform(a), InitNode::Uniform(b)) =
            (new_left.as_ref(), new_right.as_ref())
            && a == b
        {
            if *a {
                self.yes_leaf.clone()
            } else {
                self.no_leaf.clone()
            }
        } else {
            std::sync::Arc::new(InitNode::Branch(new_left, new_right))
        }
    }
    fn meet(
        &self,
        a: &std::sync::Arc<InitNode>,
        b: &std::sync::Arc<InitNode>,
    ) -> std::sync::Arc<InitNode> {
        super::count_cfg_work(1);
        if std::sync::Arc::ptr_eq(a, b) {
            return a.clone();
        }
        match (a.as_ref(), b.as_ref()) {
            (InitNode::Uniform(false), _) => a.clone(),
            (_, InitNode::Uniform(false)) => b.clone(),
            (InitNode::Uniform(true), _) => b.clone(),
            (_, InitNode::Uniform(true)) => a.clone(),
            (InitNode::Branch(al, ar), InitNode::Branch(bl, br)) => {
                let left = self.meet(al, bl);
                let right = self.meet(ar, br);
                if std::sync::Arc::ptr_eq(&left, al) && std::sync::Arc::ptr_eq(&right, ar) {
                    a.clone()
                } else if std::sync::Arc::ptr_eq(&left, bl) && std::sync::Arc::ptr_eq(&right, br) {
                    b.clone()
                } else {
                    std::sync::Arc::new(InitNode::Branch(left, right))
                }
            }
        }
    }
}

#[derive(Clone)]
struct CfgTemp {
    ty: Ty,
    live: bool,
    borrowed_from: Option<usize>,
    exclusive_root: Option<usize>,
    permission: Option<LoanPermission>,
    loan: Option<super::model::LoanId>,
}

fn visit_op_temps(op: &Op, mut visit: impl FnMut(TempId)) {
    match op {
        Op::BeginLoan { indexes, .. }
        | Op::BuildArray(indexes)
        | Op::ReadIndex { indexes, .. }
        | Op::BorrowIndex { indexes, .. }
        | Op::CheckIndex { indexes, .. }
        | Op::CallIntrinsic { args: indexes, .. } => {
            for &id in indexes {
                visit(id);
            }
        }
        Op::IndexValue { base, index } => {
            visit(*base);
            visit(*index);
        }
        Op::Binary { left, right, .. } => {
            visit(*left);
            visit(*right);
        }
        Op::WritePlace { value, .. } | Op::DropValue(value) => visit(*value),
        Op::WriteIndex { indexes, value, .. } => {
            for &id in indexes {
                visit(id);
            }
            visit(*value);
        }
        Op::CallDirect { args, .. } => {
            for &(_, id) in args {
                visit(id);
            }
        }
        _ => {}
    }
}

fn visit_terminator_temps(terminator: &Terminator, mut visit: impl FnMut(TempId)) {
    match terminator {
        Terminator::Return {
            value: Some(id), ..
        } => visit(*id),
        Terminator::BranchBool { condition, .. } => visit(*condition),
        _ => {}
    }
}
struct CfgTemps {
    data: std::collections::HashMap<usize, CfgTemp>,
    reads: std::collections::HashMap<usize, usize>,
    exclusive: std::collections::HashMap<usize, usize>,
    pending_end: Vec<super::model::LoanId>,
}
impl CfgTemps {
    fn new() -> Self {
        Self {
            data: std::collections::HashMap::new(),
            reads: std::collections::HashMap::new(),
            exclusive: std::collections::HashMap::new(),
            pending_end: Vec::new(),
        }
    }
    fn reads(&self, root: usize) -> usize {
        self.reads.get(&root).copied().unwrap_or(0)
    }
    fn exclusive(&self, root: usize) -> usize {
        self.exclusive.get(&root).copied().unwrap_or(0)
    }
    fn read(&mut self, id: TempId, consume: bool) -> Result<Ty, &'static str> {
        let temp = self.data.get_mut(&id.0).ok_or("undefined temporary")?;
        if !temp.live {
            return Err("temporary used twice");
        }
        if consume {
            temp.live = false;
            if let Some(root) = temp.borrowed_from {
                let remaining = self
                    .reads
                    .get(&root)
                    .copied()
                    .unwrap_or(0)
                    .checked_sub(1)
                    .ok_or("borrow count")?;
                self.reads.insert(root, remaining);
            }
        }
        Ok(temp.ty.clone())
    }
    fn info(&self, id: TempId) -> Result<&CfgTemp, &'static str> {
        let temp = self.data.get(&id.0).ok_or("undefined temporary")?;
        if !temp.live {
            return Err("temporary used twice");
        }
        Ok(temp)
    }
}

fn verify_cfg(function: &Function) -> Result<(), &'static str> {
    use std::collections::{HashSet, VecDeque};
    let blocks: Vec<_> = std::iter::once(&function.block)
        .chain(&function.blocks)
        .collect();
    let mut successors = vec![Vec::new(); blocks.len()];
    let mut predecessors = vec![Vec::new(); blocks.len()];
    let valid_span = |span: &crate::diagnostic::Span| {
        span.file == function.source_file
            && span.start <= span.end
            && usize::try_from(span.start)
                .ok()
                .is_some_and(|n| function.source_text.is_char_boundary(n))
            && usize::try_from(span.end)
                .ok()
                .is_some_and(|n| function.source_text.is_char_boundary(n))
    };
    for (index, block) in blocks.iter().enumerate() {
        super::count_cfg_work(1);
        if block.id.0 != index {
            return Err("block identity");
        }
        let (targets, span): (Vec<usize>, _) = match &block.terminator {
            Terminator::Return {
                value: Some(_),
                span,
            } => (vec![], span),
            Terminator::Return { value: None, .. } => return Err("missing return value"),
            Terminator::Jump { target, span } => (vec![target.0], span),
            Terminator::BranchBool { yes, no, span, .. } => (vec![yes.0, no.0], span),
        };
        if !valid_span(span) {
            return Err("terminator origin");
        }
        for target in targets {
            super::count_cfg_work(1);
            if target >= blocks.len() {
                return Err("invalid jump target");
            }
            successors[index].push(target);
            predecessors[target].push(index);
        }
    }
    // Only a while lowering may introduce a backward CFG edge. Its body
    // jumps to the first condition block, immediately after its preheader.
    // Check this before either fixed-point walk: malformed cycles can make
    // dominance and temporary liveness fail to converge.
    for (source, edges) in successors.iter().enumerate() {
        for &target in edges {
            if target > source {
                continue;
            }
            let Terminator::Jump { span: body, .. } = &blocks[source].terminator else {
                return Err("invalid control-flow cycle");
            };
            super::count_cfg_work(1);
            let preheader =
                target
                    .checked_sub(1)
                    .and_then(|before| match &blocks[before].terminator {
                        Terminator::Jump { target: dest, span } if dest.0 == target => Some(span),
                        _ => None,
                    });
            if !preheader.is_some_and(|span| span.start < body.start && span.end >= body.end) {
                return Err("invalid control-flow cycle");
            }
        }
    }
    let work_limit = blocks
        .len()
        .saturating_mul(blocks.len())
        .saturating_mul(8)
        .max(1024);
    let mut reachable = vec![false; blocks.len()];
    let mut pending = vec![0];
    while let Some(block) = pending.pop() {
        super::count_cfg_work(1);
        if std::mem::replace(&mut reachable[block], true) {
            continue;
        }
        pending.extend(successors[block].iter().copied());
    }
    if reachable.iter().any(|v| !v) {
        return Err("unreachable block");
    }
    let (tree, top, bottom) = InitTree::new(function.locals.len());
    let mut entry = bottom;
    for p in &function.parameters {
        entry = tree.set(&entry, p.local.0, true);
    }
    let mut incoming = vec![top.clone(); blocks.len()];
    let mut outgoing = incoming.clone();
    incoming[0] = entry;
    let mut queue: VecDeque<usize> = (0..blocks.len()).collect();
    let mut queued = vec![true; blocks.len()];
    let mut work = 0usize;
    while let Some(index) = queue.pop_front() {
        work += 1;
        if work > work_limit {
            return Err("control-flow analysis limit");
        }
        super::count_cfg_work(1);
        queued[index] = false;
        let mut state = incoming[index].clone();
        for ins in &blocks[index].instructions {
            super::count_cfg_work(1);
            match ins.op {
                Op::WritePlace { local, .. } => {
                    function.locals.get(local.0).ok_or("local reference")?;
                    state = tree.set(&state, local.0, true);
                }
                Op::TransferValue(local) | Op::DropBinding(local) | Op::DropLocal(local) => {
                    function.locals.get(local.0).ok_or("local reference")?;
                    state = tree.set(&state, local.0, false);
                }
                _ => {}
            }
        }
        if !std::sync::Arc::ptr_eq(&state, &outgoing[index]) {
            outgoing[index] = state;
            for &target in &successors[index] {
                if target == 0 {
                    continue;
                }
                let mut merged = top.clone();
                for &pred in &predecessors[target] {
                    merged = tree.meet(&merged, &outgoing[pred]);
                }
                if !std::sync::Arc::ptr_eq(&merged, &incoming[target]) {
                    incoming[target] = merged;
                    if !queued[target] {
                        queue.push_back(target);
                        queued[target] = true;
                    }
                }
            }
        }
    }
    // Temporary liveness is an edge property. An operand evaluated before an if
    // can remain live through both arms and be consumed in their continuation.
    let mut live_in = vec![HashSet::<usize>::new(); blocks.len()];
    let mut live_out = live_in.clone();
    let mut queue: VecDeque<usize> = (0..blocks.len()).rev().collect();
    let mut queued = vec![true; blocks.len()];
    let mut work = 0usize;
    while let Some(index) = queue.pop_front() {
        work += 1;
        if work > work_limit {
            return Err("control-flow analysis limit");
        }
        queued[index] = false;
        let mut out = HashSet::new();
        for &target in &successors[index] {
            out.extend(live_in[target].iter().copied());
        }
        let mut needed = out.clone();
        visit_terminator_temps(&blocks[index].terminator, |id| {
            needed.insert(id.0);
        });
        for ins in blocks[index].instructions.iter().rev() {
            super::count_cfg_work(1);
            if let Some(id) = ins.output {
                needed.remove(&id.0);
            }
            visit_op_temps(&ins.op, |id| {
                needed.insert(id.0);
            });
        }
        if needed != live_in[index] || out != live_out[index] {
            live_in[index] = needed;
            live_out[index] = out;
            for &pred in &predecessors[index] {
                if !queued[pred] {
                    queue.push_back(pred);
                    queued[pred] = true;
                }
            }
        }
    }
    let mut definitions = std::collections::HashMap::<usize, (usize, CfgTemp)>::new();
    for (block_index, block) in blocks.iter().enumerate() {
        for ins in &block.instructions {
            if let (Some(id), Some(ty)) = (ins.output, ins.ty.clone()) {
                let borrowed_from = match ins.op {
                    Op::BorrowValue(root) | Op::BorrowIndex { local: root, .. } => Some(root.0),
                    Op::BeginLoan {
                        root,
                        permission: LoanPermission::Read,
                        ..
                    } => Some(root.0),
                    _ => None,
                };
                let exclusive_root = match ins.op {
                    Op::BeginLoan {
                        root,
                        permission: LoanPermission::Inout,
                        ..
                    } => Some(root.0),
                    _ => None,
                };
                let (permission, loan) = match ins.op {
                    Op::BeginLoan {
                        permission, loan, ..
                    } => (Some(permission), Some(loan)),
                    _ => (None, None),
                };
                if definitions
                    .insert(
                        id.0,
                        (
                            block_index,
                            CfgTemp {
                                ty,
                                live: true,
                                borrowed_from,
                                exclusive_root,
                                permission,
                                loan,
                            },
                        ),
                    )
                    .is_some()
                {
                    return Err("temporary identity");
                }
            }
        }
    }
    // Compute dominance once. Rewalking the whole CFG for every temporary
    // carried across an if made later operands quadratic in the source size.
    let mut seen = vec![false; blocks.len()];
    let mut postorder = Vec::with_capacity(blocks.len());
    let mut dfs = vec![(0usize, 0usize)];
    seen[0] = true;
    while let Some((node, edge)) = dfs.last_mut() {
        if *edge == successors[*node].len() {
            postorder.push(*node);
            dfs.pop();
        } else {
            let next = successors[*node][*edge];
            *edge += 1;
            if !seen[next] {
                seen[next] = true;
                dfs.push((next, 0));
            }
        }
    }
    postorder.reverse();
    let mut rank = vec![0usize; blocks.len()];
    for (at, &node) in postorder.iter().enumerate() {
        rank[node] = at;
    }
    let mut idom = vec![None; blocks.len()];
    idom[0] = Some(0);
    let intersect =
        |mut a: usize, mut b: usize, idom: &[Option<usize>]| -> Result<usize, &'static str> {
            let mut steps = 0usize;
            while a != b {
                steps += 1;
                if steps > idom.len().saturating_mul(2) {
                    return Err("invalid control-flow cycle");
                }
                while rank[a] > rank[b] {
                    let parent = idom[a].ok_or("invalid control-flow cycle")?;
                    if rank[parent] >= rank[a] {
                        return Err("invalid control-flow cycle");
                    }
                    a = parent;
                }
                while rank[b] > rank[a] {
                    let parent = idom[b].ok_or("invalid control-flow cycle")?;
                    if rank[parent] >= rank[b] {
                        return Err("invalid control-flow cycle");
                    }
                    b = parent;
                }
            }
            Ok(a)
        };
    let mut work = 0usize;
    loop {
        work += 1;
        if work > work_limit {
            return Err("control-flow analysis limit");
        }
        let mut changed = false;
        for &node in postorder.iter().skip(1) {
            let mut defined = predecessors[node]
                .iter()
                .copied()
                .filter(|&p| idom[p].is_some());
            let Some(mut next) = defined.next() else {
                continue;
            };
            for predecessor in defined {
                next = intersect(next, predecessor, &idom)?;
            }
            if idom[node] != Some(next) {
                idom[node] = Some(next);
                changed = true;
            }
            super::count_cfg_work(1);
        }
        if !changed {
            break;
        }
    }
    let mut children = vec![Vec::new(); blocks.len()];
    for node in 1..blocks.len() {
        children[idom[node].ok_or("unreachable block")?].push(node);
    }
    let mut entered = vec![0usize; blocks.len()];
    let mut exited = vec![0usize; blocks.len()];
    let mut clock = 0usize;
    let mut walk = vec![(0usize, 0usize)];
    entered[0] = clock;
    clock += 1;
    while let Some((node, child)) = walk.last_mut() {
        if *child == children[*node].len() {
            exited[*node] = clock;
            clock += 1;
            walk.pop();
        } else {
            let next = children[*node][*child];
            *child += 1;
            entered[next] = clock;
            clock += 1;
            walk.push((next, 0));
        }
    }
    let return_temps: HashSet<usize> = blocks
        .iter()
        .filter_map(|block| {
            if let Terminator::Return {
                value: Some(id), ..
            } = block.terminator
            {
                Some(id.0)
            } else {
                None
            }
        })
        .collect();
    let mut outputs = HashSet::new();
    let mut loan_ids = HashSet::new();
    for (index, block) in blocks.iter().enumerate() {
        super::count_cfg_work(1);
        let mut initialized = incoming[index].clone();
        let mut temps = CfgTemps::new();
        for &id in &live_in[index] {
            super::count_cfg_work(1);
            let missing = || match &block.terminator {
                _ if return_temps.contains(&id) => "undefined return temporary",
                Terminator::BranchBool { condition, .. } if condition.0 == id => {
                    "branch condition type"
                }
                _ => "undefined temporary",
            };
            let (defined_in, metadata) = definitions.get(&id).ok_or_else(missing)?;
            if *defined_in == index {
                return Err(missing());
            }
            if !(entered[*defined_in] <= entered[index] && exited[index] <= exited[*defined_in]) {
                return Err(missing());
            }
            if let Some(root) = metadata.borrowed_from {
                *temps.reads.entry(root).or_default() += 1;
            }
            if let Some(root) = metadata.exclusive_root {
                *temps.exclusive.entry(root).or_default() += 1;
            }
            temps.data.insert(id, metadata.clone());
        }
        for (position, ins) in block.instructions.iter().enumerate() {
            super::count_cfg_work(1);
            if !valid_span(&ins.span)
                || ins.origin.0 != position
                || ins.scope.0 >= function.scope_count
            {
                return Err("instruction origin");
            }
            let expected = match &ins.op {
                Op::ConstI64(_) => Some(Ty::I64),
                Op::ConstBool(_) => Some(Ty::Bool),
                Op::ConstUnit => Some(Ty::Unit),
                Op::CopyValue(id) | Op::BorrowValue(id) | Op::TransferValue(id) => {
                    let local = function.locals.get(id.0).ok_or("local reference")?;
                    if !tree.get(&initialized, id.0) {
                        return Err("uninitialized local");
                    }
                    if matches!(ins.op, Op::CopyValue(_)) && !local.ty.copyable() {
                        return Err("copy noncopyable local");
                    }
                    if matches!(ins.op, Op::CopyValue(_) | Op::BorrowValue(_))
                        && temps.exclusive(id.0) != 0
                    {
                        return Err("read during exclusive loan");
                    }
                    if matches!(ins.op, Op::TransferValue(_)) {
                        if matches!(
                            local.parameter_mode,
                            Some(ParameterMode::Read | ParameterMode::Inout)
                        ) {
                            return Err("transfer read parameter");
                        }
                        if temps.reads(id.0) != 0 || temps.exclusive(id.0) != 0 {
                            return Err("transfer while borrowed");
                        }
                        initialized = tree.set(&initialized, id.0, false);
                    }
                    Some(local.ty.clone())
                }
                Op::BeginLoan {
                    root,
                    indexes,
                    loan,
                    origin_span,
                    permission,
                    parent,
                } => {
                    let local = function.locals.get(root.0).ok_or("loan root")?;
                    if !tree.get(&initialized, root.0) {
                        return Err("loan unavailable root");
                    }
                    if !valid_span(origin_span)
                        || origin_span.start as usize != loan.0
                        || origin_span.start > ins.span.start
                        || origin_span.end < ins.span.end
                        || !loan_ids.insert(loan.0)
                    {
                        return Err("loan identity");
                    }
                    if *parent
                        != (local.parameter_mode == Some(ParameterMode::Inout)).then_some(*root)
                    {
                        return Err("loan parent");
                    }
                    if *permission == LoanPermission::Inout {
                        if !local.mutable || local.parameter_mode == Some(ParameterMode::Read) {
                            return Err("exclusive loan permission");
                        }
                        if temps.reads(root.0) != 0 || temps.exclusive(root.0) != 0 {
                            return Err("overlapping loan");
                        }
                        *temps.exclusive.entry(root.0).or_default() += 1;
                    } else {
                        if temps.exclusive(root.0) != 0 {
                            return Err("read during exclusive loan");
                        }
                        *temps.reads.entry(root.0).or_default() += 1;
                    }
                    let mut ty = local.ty.clone();
                    for id in indexes {
                        if temps.read(*id, true)? != Ty::I64 {
                            return Err("index type");
                        }
                        let Ty::Array(child) = ty else {
                            return Err("index base type");
                        };
                        ty = *child;
                    }
                    Some(ty)
                }
                Op::EndLoan(loan) => {
                    if temps.pending_end.pop() != Some(*loan) {
                        return Err("unmatched loan end");
                    }
                    None
                }
                Op::BuildArray(items) => {
                    let Some(Ty::Array(element)) = &ins.ty else {
                        return Err("array result type");
                    };
                    for id in items {
                        if temps.read(*id, true)? != **element {
                            return Err("array element type");
                        }
                    }
                    ins.ty.clone()
                }
                Op::ReadIndex { local, indexes }
                | Op::BorrowIndex { local, indexes }
                | Op::CheckIndex { local, indexes } => {
                    let local = function.locals.get(local.0).ok_or("read index local")?;
                    if !tree.get(&initialized, local.id.0) {
                        return Err("read index uninitialized local");
                    }
                    if temps.exclusive(local.id.0) != 0 {
                        return Err("read during exclusive loan");
                    }
                    if indexes.is_empty() {
                        return Err("empty index path");
                    }
                    let mut ty = local.ty.clone();
                    for id in indexes {
                        if temps.read(*id, !matches!(ins.op, Op::CheckIndex { .. }))? != Ty::I64 {
                            return Err("index type");
                        }
                        let Ty::Array(child) = ty else {
                            return Err("index base type");
                        };
                        ty = *child;
                    }
                    if matches!(ins.op, Op::CheckIndex { .. }) {
                        None
                    } else {
                        Some(ty)
                    }
                }
                Op::IndexValue { base, index } => {
                    let base = temps.read(*base, true)?;
                    if temps.read(*index, true)? != Ty::I64 {
                        return Err("index type");
                    }
                    let Ty::Array(child) = base else {
                        return Err("index base type");
                    };
                    Some(*child)
                }
                Op::Binary { op, left, right } => {
                    let left = temps.read(*left, true)?;
                    let right = temps.read(*right, true)?;
                    if left != right {
                        return Err("operand type");
                    }
                    Some(match op {
                        Binary::Add | Binary::Sub if left == Ty::I64 => Ty::I64,
                        Binary::Less if left == Ty::I64 => Ty::Bool,
                        Binary::Equal => Ty::Bool,
                        _ => return Err("binary domain"),
                    })
                }
                Op::WritePlace { local, value } => {
                    let local = function.locals.get(local.0).ok_or("write local")?;
                    if temps.reads(local.id.0) != 0 || temps.exclusive(local.id.0) != 0 {
                        return Err("write while borrowed");
                    }
                    if tree.get(&initialized, local.id.0) && !local.mutable {
                        return Err("immutable rewrite");
                    }
                    if temps.info(*value)?.borrowed_from.is_some()
                        || temps.info(*value)?.permission == Some(LoanPermission::Inout)
                    {
                        return Err("write borrowed value");
                    }
                    if temps.read(*value, true)? != local.ty {
                        return Err("write type");
                    }
                    initialized = tree.set(&initialized, local.id.0, true);
                    None
                }
                Op::WriteIndex {
                    local,
                    indexes,
                    value,
                } => {
                    let local = function.locals.get(local.0).ok_or("write index local")?;
                    if !tree.get(&initialized, local.id.0) || !local.mutable {
                        return Err("write index root");
                    }
                    if temps.reads(local.id.0) != 0 || temps.exclusive(local.id.0) != 0 {
                        return Err("write while borrowed");
                    }
                    if indexes.is_empty() {
                        return Err("empty index path");
                    }
                    let mut ty = local.ty.clone();
                    for id in indexes {
                        if temps.read(*id, true)? != Ty::I64 {
                            return Err("index type");
                        }
                        let Ty::Array(child) = ty else {
                            return Err("index base type");
                        };
                        ty = *child;
                    }
                    if temps.info(*value)?.borrowed_from.is_some()
                        || temps.info(*value)?.permission == Some(LoanPermission::Inout)
                    {
                        return Err("write borrowed value");
                    }
                    if temps.read(*value, true)? != ty {
                        return Err("write index type");
                    }
                    None
                }
                Op::CallIntrinsic { intrinsic, args } => {
                    let arity = match intrinsic {
                        Intrinsic::Print
                        | Intrinsic::DropData
                        | Intrinsic::Len
                        | Intrinsic::MockOpen
                        | Intrinsic::MockBlockingOpen
                        | Intrinsic::Inspect => 1,
                        Intrinsic::AssertEq => 2,
                        Intrinsic::Fail => 0,
                    };
                    if args.len() != arity {
                        return Err("intrinsic arity");
                    }
                    let first = if let Some(id) = args.first() {
                        if *intrinsic == Intrinsic::DropData
                            && temps.info(*id)?.borrowed_from.is_some()
                        {
                            return Err("drop borrowed value");
                        }
                        Some(temps.read(*id, true)?)
                    } else {
                        None
                    };
                    if *intrinsic == Intrinsic::AssertEq
                        && temps.read(args[1], true)? != first.clone().unwrap()
                    {
                        return Err("assert type");
                    }
                    if *intrinsic == Intrinsic::Len && !matches!(first.as_ref(), Some(Ty::Array(_)))
                    {
                        return Err("len domain");
                    }
                    if matches!(intrinsic, Intrinsic::MockOpen | Intrinsic::MockBlockingOpen)
                        && first != Some(Ty::I64)
                    {
                        return Err("open argument type");
                    }
                    if *intrinsic == Intrinsic::Inspect
                        && !matches!(
                            first.as_ref(),
                            Some(Ty::MockResource | Ty::MockBlockingResource)
                        )
                    {
                        return Err("inspect argument type");
                    }
                    if *intrinsic == Intrinsic::Print
                        && !first.as_ref().is_some_and(|ty| ty.copyable())
                    {
                        return Err("print resource");
                    }
                    Some(match intrinsic {
                        Intrinsic::Len => Ty::I64,
                        Intrinsic::MockOpen => Ty::MockResource,
                        Intrinsic::MockBlockingOpen => Ty::MockBlockingResource,
                        _ => Ty::Unit,
                    })
                }
                Op::CallDirect {
                    args, signature, ..
                } => {
                    if args.len() != signature.len() {
                        return Err("call arity");
                    }
                    let mut seen = HashSet::new();
                    for (slot, id) in args {
                        if !seen.insert(*slot) {
                            return Err("duplicate call slot");
                        }
                        let (_, mode, ty) = signature.get(*slot).ok_or("call slot")?;
                        let info = temps.info(*id)?;
                        if *mode == ParameterMode::Consume
                            && (info.borrowed_from.is_some() || info.exclusive_root.is_some())
                        {
                            return Err("consuming borrowed argument");
                        }
                        if *mode == ParameterMode::Inout
                            && info.permission != Some(LoanPermission::Inout)
                        {
                            return Err("inout argument permission");
                        }
                        if *mode == ParameterMode::Read
                            && info.permission == Some(LoanPermission::Inout)
                        {
                            return Err("exclusive passed as read");
                        }
                        let exclusive_root = info.exclusive_root;
                        let loan = info.loan;
                        if let Some(root) = exclusive_root {
                            let remaining = temps
                                .exclusive(root)
                                .checked_sub(1)
                                .ok_or("exclusive loan count")?;
                            temps.exclusive.insert(root, remaining);
                        }
                        if temps.read(*id, true)? != *ty {
                            return Err("call argument type");
                        }
                        if let Some(loan) = loan {
                            temps.pending_end.push(loan);
                        }
                    }
                    ins.ty.clone()
                }
                Op::DropValue(id) => {
                    if let Some(root) = temps.info(*id)?.exclusive_root {
                        let remaining = temps
                            .exclusive(root)
                            .checked_sub(1)
                            .ok_or("exclusive loan count")?;
                        temps.exclusive.insert(root, remaining);
                    }
                    temps.read(*id, true)?;
                    None
                }
                Op::DropBinding(id) => {
                    let local = function.locals.get(id.0).ok_or("drop binding local")?;
                    if !tree.get(&initialized, id.0) {
                        return Err("drop uninitialized local");
                    }
                    if temps.reads(id.0) != 0 || temps.exclusive(id.0) != 0 {
                        return Err("drop while borrowed");
                    }
                    if matches!(
                        local.parameter_mode,
                        Some(ParameterMode::Read | ParameterMode::Inout)
                    ) {
                        return Err("drop read parameter");
                    }
                    initialized = tree.set(&initialized, id.0, false);
                    Some(Ty::Unit)
                }
                Op::DropLocal(id) => {
                    function.locals.get(id.0).ok_or("drop local")?;
                    if temps.reads(id.0) != 0 || temps.exclusive(id.0) != 0 {
                        return Err("drop while borrowed");
                    }
                    initialized = tree.set(&initialized, id.0, false);
                    None
                }
            };
            if expected.as_ref() != ins.ty.as_ref() {
                return Err("instruction result type");
            }
            let access = match ins.op {
                Op::CopyValue(_)
                | Op::BorrowValue(_)
                | Op::BeginLoan { .. }
                | Op::ReadIndex { .. }
                | Op::BorrowIndex { .. }
                | Op::CheckIndex { .. } => Access::Read,
                Op::WritePlace { .. } | Op::WriteIndex { .. } => Access::Write,
                _ => Access::Own,
            };
            if ins.access != access || ins.tick != !matches!(ins.op, Op::EndLoan(_)) {
                return Err("access or source tick");
            }
            if ins.effect_io
                != matches!(
                    ins.op,
                    Op::CallIntrinsic {
                        intrinsic: Intrinsic::Print
                            | Intrinsic::MockOpen
                            | Intrinsic::MockBlockingOpen
                            | Intrinsic::Inspect,
                        ..
                    }
                )
            {
                return Err("effect summary");
            }
            match expected {
                Some(ty) => {
                    let id = ins.output.ok_or("missing output")?;
                    if !outputs.insert(id.0) {
                        return Err("temporary identity");
                    }
                    let borrowed_from = match ins.op {
                        Op::BorrowValue(root) => Some(root.0),
                        Op::BorrowIndex { local, .. } => Some(local.0),
                        Op::BeginLoan {
                            root,
                            permission: LoanPermission::Read,
                            ..
                        } => Some(root.0),
                        _ => None,
                    };
                    if matches!(ins.op, Op::BorrowValue(_) | Op::BorrowIndex { .. })
                        && let Some(root) = borrowed_from
                    {
                        *temps.reads.entry(root).or_default() += 1;
                    }
                    let exclusive_root = match ins.op {
                        Op::BeginLoan {
                            root,
                            permission: LoanPermission::Inout,
                            ..
                        } => Some(root.0),
                        _ => None,
                    };
                    let (permission, loan) = match ins.op {
                        Op::BeginLoan {
                            permission, loan, ..
                        } => (Some(permission), Some(loan)),
                        _ => (None, None),
                    };
                    temps.data.insert(
                        id.0,
                        CfgTemp {
                            ty,
                            live: true,
                            borrowed_from,
                            exclusive_root,
                            permission,
                            loan,
                        },
                    );
                }
                None if ins.output.is_some() => return Err("unexpected output"),
                None => {}
            }
        }
        match &block.terminator {
            Terminator::Return {
                value: Some(id), ..
            } => {
                let info = temps.info(*id).map_err(|_| "undefined return temporary")?;
                if info.borrowed_from.is_some() || info.exclusive_root.is_some() {
                    return Err("borrowed return");
                }
                if info.ty != function.result {
                    return Err("return type");
                }
                temps.read(*id, true)?;
            }
            Terminator::BranchBool { condition, .. } => {
                if temps.read(*condition, true).ok() != Some(Ty::Bool) {
                    return Err("branch condition type");
                }
            }
            Terminator::Jump { .. } => {}
            Terminator::Return { value: None, .. } => return Err("missing return value"),
        }
        if temps
            .data
            .iter()
            .any(|(id, temp)| temp.live && !live_out[index].contains(id))
        {
            return Err("unconsumed temporary");
        }
        if live_out[index]
            .iter()
            .any(|id| !temps.data.get(id).is_some_and(|temp| temp.live))
        {
            return Err("temporary used twice");
        }
        if !temps.pending_end.is_empty() {
            return Err("loan end missing");
        }
        let mut reads_out = std::collections::HashMap::<usize, usize>::new();
        let mut exclusive_out = std::collections::HashMap::<usize, usize>::new();
        for id in &live_out[index] {
            let temp = temps.data.get(id).ok_or("undefined temporary")?;
            if let Some(root) = temp.borrowed_from {
                *reads_out.entry(root).or_default() += 1;
            }
            if let Some(root) = temp.exclusive_root {
                *exclusive_out.entry(root).or_default() += 1;
            }
        }
        if reads_out
            .iter()
            .any(|(root, count)| temps.reads(*root) != *count)
            || exclusive_out
                .iter()
                .any(|(root, count)| temps.exclusive(*root) != *count)
            || temps
                .reads
                .iter()
                .any(|(root, count)| *count != reads_out.get(root).copied().unwrap_or(0))
            || temps
                .exclusive
                .iter()
                .any(|(root, count)| *count != exclusive_out.get(root).copied().unwrap_or(0))
        {
            return Err("unreleased loan");
        }
    }
    // Every value carried out of a block must be accounted for on each edge.
    // A consumer in one arm does not license silently losing it in the other.
    for (index, targets) in successors.iter().enumerate() {
        for &target in targets {
            if live_out[index] != live_in[target] {
                return Err("temporary leaked on edge");
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ir::compile_scalar;
    #[test]
    fn join_initialization_is_intersection_in_both_predecessor_orders() {
        let (tree, _, bottom) = InitTree::new(3);
        let one_arm = tree.set(&bottom, 1, true);
        for merged in [tree.meet(&bottom, &one_arm), tree.meet(&one_arm, &bottom)] {
            assert!(!tree.get(&merged, 1));
        }
    }
    #[test]
    fn verifier_rejects_index_write_while_reordered_read_loan_is_live() {
        let p = compile_scalar("v.nx", "fn main(){var a=[1];a[0]=2;}").unwrap();
        let mut f = p.functions.entries[0].clone();
        let pos = f
            .block
            .instructions
            .iter()
            .position(|i| matches!(i.op, Op::WriteIndex { .. }))
            .unwrap();
        let next = TempId(
            f.block.instructions[..pos]
                .iter()
                .filter(|i| i.output.is_some())
                .count(),
        );
        let span = f.block.instructions[pos].span.clone();
        for ins in &mut f.block.instructions[pos..] {
            if let Some(id) = &mut ins.output {
                id.0 += 1;
            }
        }
        let Terminator::Return {
            value: Some(id), ..
        } = &mut f.block.terminator
        else {
            panic!("return value")
        };
        id.0 += 1;
        f.block.instructions.insert(
            pos,
            super::super::model::Instruction {
                op: Op::BorrowValue(super::super::model::LocalId(0)),
                output: Some(next),
                ty: Some(Ty::Array(Box::new(Ty::I64))),
                span: span.clone(),
                scope: super::super::model::ScopeId(0),
                origin: super::super::model::OriginId(0),
                access: Access::Read,
                effect_io: false,
                tick: true,
            },
        );
        f.block.instructions.insert(
            pos + 2,
            super::super::model::Instruction {
                op: Op::DropValue(next),
                output: None,
                ty: None,
                span,
                scope: super::super::model::ScopeId(0),
                origin: super::super::model::OriginId(0),
                access: Access::Own,
                effect_io: false,
                tick: true,
            },
        );
        for (index, ins) in f.block.instructions.iter_mut().enumerate() {
            ins.origin.0 = index;
        }
        f.content_hash = super::super::model::content_hash(&f);
        assert_eq!(verify_function(&f), Err("write while borrowed"));
    }
    #[test]
    fn verifier_rejects_drop_binding_while_read_loan_is_live() {
        let p = compile_scalar("v.nx", "fn main(){let a=[1];drop(a);}").unwrap();
        let mut f = p.functions.entries[0].clone();
        let pos = f
            .block
            .instructions
            .iter()
            .position(|i| matches!(i.op, Op::DropBinding(_)))
            .unwrap();
        let next = TempId(
            f.block.instructions[..pos]
                .iter()
                .filter(|i| i.output.is_some())
                .count(),
        );
        let span = f.block.instructions[pos].span.clone();
        for ins in &mut f.block.instructions[pos..] {
            if let Some(id) = &mut ins.output {
                id.0 += 1;
            }
            if let Op::DropValue(id) = &mut ins.op {
                id.0 += 1;
            }
        }
        let Terminator::Return {
            value: Some(id), ..
        } = &mut f.block.terminator
        else {
            panic!("return value")
        };
        id.0 += 1;
        f.block.instructions.insert(
            pos,
            super::super::model::Instruction {
                op: Op::BorrowValue(super::super::model::LocalId(0)),
                output: Some(next),
                ty: Some(Ty::Array(Box::new(Ty::I64))),
                span: span.clone(),
                scope: super::super::model::ScopeId(0),
                origin: super::super::model::OriginId(0),
                access: Access::Read,
                effect_io: false,
                tick: true,
            },
        );
        f.block.instructions.insert(
            pos + 2,
            super::super::model::Instruction {
                op: Op::DropValue(next),
                output: None,
                ty: None,
                span,
                scope: super::super::model::ScopeId(0),
                origin: super::super::model::OriginId(0),
                access: Access::Own,
                effect_io: false,
                tick: true,
            },
        );
        for (index, ins) in f.block.instructions.iter_mut().enumerate() {
            ins.origin.0 = index;
        }
        f.content_hash = super::super::model::content_hash(&f);
        assert_eq!(verify_function(&f), Err("drop while borrowed"));
    }
    #[test]
    fn verifier_rejects_undefined_temporary_and_changed_hash() {
        let p = compile_scalar("v.nx", "fn main(){let x=1;print(x);}").unwrap();
        let mut f = p.functions.entries[0].clone();
        if let Op::WritePlace { value, .. } = &mut f.block.instructions[1].op {
            *value = TempId(999)
        } else {
            panic!("expected write")
        }
        assert_eq!(verify_function(&f), Err("content hash"));
        f.content_hash = super::super::model::content_hash(&f);
        assert_eq!(verify_function(&f), Err("undefined temporary"));
    }
    #[test]
    fn verifier_rejects_uninitialized_local_and_wrong_type() {
        let p = compile_scalar("v.nx", "fn main(){let x=1;print(x);}").unwrap();
        let mut f = p.functions.entries[0].clone();
        f.block.instructions.swap(0, 1);
        f.content_hash = super::super::model::content_hash(&f);
        assert!(verify_function(&f).is_err());
        let mut f = p.functions.entries[0].clone();
        f.locals[0].ty = Ty::Bool;
        f.content_hash = super::super::model::content_hash(&f);
        assert_eq!(verify_function(&f), Err("write type"));
    }
    #[test]
    fn verifier_rejects_effect_access_cleanup_and_terminator_mutations() {
        let p = compile_scalar("v.nx", "fn main(){var x=1;x=x+1;print(x);}").unwrap();
        let original = p.functions.entries[0].clone();
        let mut f = original.clone();
        let call = f
            .block
            .instructions
            .iter_mut()
            .find(|i| matches!(i.op, Op::CallIntrinsic { .. }))
            .unwrap();
        call.effect_io = false;
        f.content_hash = super::super::model::content_hash(&f);
        assert_eq!(verify_function(&f), Err("effect summary"));
        let mut f = original.clone();
        let write = f
            .block
            .instructions
            .iter_mut()
            .find(|i| matches!(i.op, Op::WritePlace { .. }))
            .unwrap();
        write.access = Access::Read;
        f.content_hash = super::super::model::content_hash(&f);
        assert_eq!(verify_function(&f), Err("access or source tick"));
        let mut f = original.clone();
        f.cleanup.clear();
        f.content_hash = super::super::model::content_hash(&f);
        assert_eq!(verify_function(&f), Err("cleanup plan"));
        let mut f = original;
        let Terminator::Return { value, .. } = &mut f.block.terminator else {
            panic!("return terminator")
        };
        *value = None;
        f.content_hash = super::super::model::content_hash(&f);
        assert_eq!(verify_function(&f), Err("missing return value"));
    }
    #[test]
    fn verifier_rejects_duplicate_temporary_consumption_and_orphans() {
        let p = compile_scalar("v.nx", "fn main(){assert_eq(1,2);}").unwrap();
        let original = p.functions.entries[0].clone();
        let mut f = original.clone();
        let call = f
            .block
            .instructions
            .iter_mut()
            .find(|i| matches!(i.op, Op::CallIntrinsic { .. }))
            .unwrap();
        if let Op::CallIntrinsic { args, .. } = &mut call.op {
            args[1] = args[0]
        }
        f.content_hash = super::super::model::content_hash(&f);
        assert_eq!(verify_function(&f), Err("temporary used twice"));
        let mut f = original;
        f.block
            .instructions
            .retain(|i| !matches!(i.op, Op::DropValue(_)));
        for (index, instruction) in f.block.instructions.iter_mut().enumerate() {
            instruction.origin = super::super::model::OriginId(index);
        }
        f.content_hash = super::super::model::content_hash(&f);
        assert_eq!(verify_function(&f), Err("unconsumed temporary"));
    }
    #[test]
    fn verifier_rejects_unknown_scope_duplicate_origin_and_invalid_source_spans() {
        let source = "fn main(){let x=1;print(x);}//é";
        let p = compile_scalar("v.nx", source).unwrap();
        let original = p.functions.entries[0].clone();
        let mut f = original.clone();
        f.block.instructions[0].scope = super::super::model::ScopeId(f.scope_count);
        assert_eq!(verify_function(&f), Err("instruction scope"));
        let mut f = original.clone();
        f.locals[0].scope = super::super::model::ScopeId(f.scope_count);
        f.content_hash = super::super::model::content_hash(&f);
        assert_eq!(verify_function(&f), Err("local scope"));
        let mut f = original.clone();
        f.block.instructions[1].origin = f.block.instructions[0].origin;
        assert_eq!(verify_function(&f), Err("origin"));
        let mut f = original.clone();
        f.block.instructions[0].span.end = (source.len() + 1) as u64;
        assert_eq!(verify_function(&f), Err("source origin"));
        let mut f = original.clone();
        f.block.instructions[0].span.start = 3;
        f.block.instructions[0].span.end = 2;
        assert_eq!(verify_function(&f), Err("source origin"));
        let mut f = original.clone();
        f.block.instructions[0].span.file = "other.nx".into();
        assert_eq!(verify_function(&f), Err("source origin"));
        let mut f = original.clone();
        f.block.instructions[0].span.start = (source.find('é').unwrap() + 1) as u64;
        f.block.instructions[0].span.end = (source.find('é').unwrap() + 'é'.len_utf8()) as u64;
        assert_eq!(verify_function(&f), Err("source origin"));
        let mut f = original.clone();
        f.block.instructions[0].span.end = (source.find('é').unwrap() + 1) as u64;
        assert_eq!(verify_function(&f), Err("source origin"));
        let mut f = original;
        let Terminator::Return { span, .. } = &mut f.block.terminator else {
            panic!("return terminator")
        };
        span.end = (source.len() + 1) as u64;
        assert_eq!(verify_function(&f), Err("terminator origin"));
    }
    #[test]
    fn verifier_rejects_changed_function_identity_position_and_scope_count() {
        let p = compile_scalar("v.nx", "fn helper(){} fn main(){}").unwrap();
        let mut changed = p.clone();
        changed.functions.entries[1].table_index = 0;
        assert_eq!(verify_program(&changed), Err("function identity"));
        let mut f = p.functions.entries[1].clone();
        f.id.0 = "other".into();
        assert_eq!(verify_function(&f), Err("function identity"));
        let mut f = p.functions.entries[1].clone();
        f.scope_count = 0;
        assert_eq!(verify_function(&f), Err("function identity"));
        let mut f = p.functions.entries[1].clone();
        f.scope_count = 1000;
        assert_eq!(verify_function(&f), Err("scope table"));
        let p = compile_scalar("v.nx", "fn main(){{print(1);}}").unwrap();
        let mut f = p.functions.entries[0].clone();
        f.scope_parents[1] = Some(super::super::model::ScopeId(1));
        assert_eq!(verify_function(&f), Err("scope table"));
        let mut f = p.functions.entries[0].clone();
        f.scope_parents[0] = Some(super::super::model::ScopeId(0));
        assert_eq!(verify_function(&f), Err("scope table"));
    }
    fn changed(source: &str, edit: impl FnOnce(&mut Function)) -> Result<(), &'static str> {
        let p = compile_scalar("v.nx", source).unwrap();
        let mut f = p.functions.entries[0].clone();
        edit(&mut f);
        f.content_hash = super::super::model::content_hash(&f);
        verify_function(&f)
    }
    fn changed_main(source: &str, edit: impl FnOnce(&mut Function)) -> Result<(), &'static str> {
        let p = compile_scalar("v.nx", source).unwrap();
        let mut f = p.functions.function("main").unwrap().clone();
        edit(&mut f);
        f.content_hash = super::super::model::content_hash(&f);
        verify_function(&f)
    }
    #[test]
    fn verifier_rejects_array_element_type_mismatch() {
        let got = changed("fn main(){let a=[1,2];}", |f| {
            let i = f
                .block
                .instructions
                .iter_mut()
                .find(|i| matches!(i.op, Op::ConstI64(2)))
                .unwrap();
            i.op = Op::ConstBool(true);
            i.ty = Some(Ty::Bool);
        });
        assert_eq!(got, Err("array element type"));
    }
    #[test]
    fn verifier_rejects_non_integer_read_index() {
        let got = changed("fn main(){let a=[1];print(a[0]);}", |f| {
            let i = f
                .block
                .instructions
                .iter_mut()
                .find(|i| matches!(i.op, Op::ConstI64(0)))
                .unwrap();
            i.op = Op::ConstBool(false);
            i.ty = Some(Ty::Bool);
        });
        assert_eq!(got, Err("index type"));
    }
    #[test]
    fn verifier_rejects_read_index_from_non_array_root() {
        let got = changed("fn main(){let n=1;let a=[2];let b=a[0];}", |f| {
            let i = f
                .block
                .instructions
                .iter_mut()
                .find(|i| matches!(i.op, Op::ReadIndex { .. }))
                .unwrap();
            if let Op::ReadIndex { local, .. } = &mut i.op {
                *local = super::super::model::LocalId(0);
            }
        });
        assert_eq!(got, Err("index base type"));
    }
    #[test]
    fn verifier_rejects_read_index_from_uninitialized_array() {
        let got = changed("fn main(){let a=[1];let b=a[0];let c=[2];}", |f| {
            let i = f
                .block
                .instructions
                .iter_mut()
                .find(|i| matches!(i.op, Op::ReadIndex { .. }))
                .unwrap();
            if let Op::ReadIndex { local, .. } = &mut i.op {
                *local = super::super::model::LocalId(2);
            }
        });
        assert_eq!(got, Err("read index uninitialized local"));
    }
    #[test]
    fn verifier_rejects_write_index_to_immutable_root() {
        let got = changed("fn main(){let b=[1];var a=[2];a[0]=3;}", |f| {
            let i = f
                .block
                .instructions
                .iter_mut()
                .find(|i| matches!(i.op, Op::WriteIndex { .. }))
                .unwrap();
            if let Op::WriteIndex { local, .. } = &mut i.op {
                *local = super::super::model::LocalId(0);
            }
        });
        assert_eq!(got, Err("immutable index root"));
    }
    #[test]
    fn verifier_rejects_write_index_with_wrong_element_type() {
        let got = changed("fn main(){var a=[1];a[0]=3;}", |f| {
            let i = f
                .block
                .instructions
                .iter_mut()
                .find(|i| matches!(i.op, Op::ConstI64(3)))
                .unwrap();
            i.op = Op::ConstBool(true);
            i.ty = Some(Ty::Bool);
        });
        assert_eq!(got, Err("write index type"));
    }
    #[test]
    fn verifier_rejects_index_operations_without_projection() {
        let got = changed("fn main(){var a=[1];a[0]=3;}", |f| {
            let i = f
                .block
                .instructions
                .iter_mut()
                .find(|i| matches!(i.op, Op::WriteIndex { .. }))
                .unwrap();
            if let Op::WriteIndex { indexes, .. } = &mut i.op {
                indexes.clear();
            }
        });
        assert_eq!(got, Err("empty index path"));
    }
    #[test]
    fn verifier_rejects_index_probe_with_wrong_operand_type() {
        let got = changed("fn main(){let a=[1];print(a[0]);}", |f| {
            let i = f
                .block
                .instructions
                .iter_mut()
                .find(|i| matches!(i.op, Op::ConstI64(0)))
                .unwrap();
            i.op = Op::ConstBool(false);
            i.ty = Some(Ty::Bool);
        });
        assert_eq!(got, Err("index type"));
    }
    #[test]
    fn verifier_rejects_copy_of_resource_local() {
        let got = changed("fn main(){let a=mock_open(1);inspect(a);}", |f| {
            let i = f
                .block
                .instructions
                .iter_mut()
                .find(|i| matches!(i.op, Op::BorrowValue(_)))
                .unwrap();
            i.op = Op::CopyValue(super::super::model::LocalId(0));
        });
        assert_eq!(got, Err("copy noncopyable local"));
    }
    #[test]
    fn verifier_rejects_destroying_root_while_read_argument_is_live() {
        let got = changed("fn main(){let a=[1];print(a);}", |f| {
            let i = f
                .block
                .instructions
                .iter_mut()
                .find(|i| matches!(i.op, Op::CallIntrinsic { .. }))
                .unwrap();
            i.op = Op::DropLocal(super::super::model::LocalId(0));
        });
        assert_eq!(got, Err("drop while borrowed"));
    }
    #[test]
    fn verifier_rejects_indexed_write_with_missing_owner_root() {
        let got = changed("fn main(){var a=[1];a[0]=2;}", |f| {
            let i = f
                .block
                .instructions
                .iter_mut()
                .find(|i| matches!(i.op, Op::WriteIndex { .. }))
                .unwrap();
            if let Op::WriteIndex { local, .. } = &mut i.op {
                *local = super::super::model::LocalId(99);
            }
        });
        assert_eq!(got, Err("write index local"));
    }
    #[test]
    fn verifier_rejects_direct_call_to_missing_function() {
        let mut p = compile_scalar("v.nx", "fn f(){} fn main(){f();}").unwrap();
        let f = p
            .functions
            .entries
            .iter_mut()
            .find(|f| f.name == "main")
            .unwrap();
        let call = f
            .block
            .instructions
            .iter_mut()
            .find(|i| matches!(i.op, Op::CallDirect { .. }))
            .unwrap();
        if let Op::CallDirect { function, .. } = &mut call.op {
            function.0 = "absent".into();
        }
        f.content_hash = super::super::model::content_hash(f);
        assert_eq!(verify_program(&p), Err("unknown callee"));
    }
    #[test]
    fn verifier_rejects_direct_call_wrong_arity_and_duplicate_slot() {
        let source = "fn f(a:i64,b:i64){} fn main(){f(1,2);}";
        let got = changed_main(source, |f| {
            if let Op::CallDirect { args, .. } = &mut f
                .block
                .instructions
                .iter_mut()
                .find(|i| matches!(i.op, Op::CallDirect { .. }))
                .unwrap()
                .op
            {
                args.pop();
            }
        });
        assert_eq!(got, Err("call arity"));
        let got = changed_main(source, |f| {
            if let Op::CallDirect { args, .. } = &mut f
                .block
                .instructions
                .iter_mut()
                .find(|i| matches!(i.op, Op::CallDirect { .. }))
                .unwrap()
                .op
            {
                args[1].0 = args[0].0;
            }
        });
        assert_eq!(got, Err("duplicate call slot"));
    }
    #[test]
    fn verifier_rejects_direct_call_signature_mismatch() {
        let mut p = compile_scalar("v.nx", "fn f(x:i64){} fn main(){f(1);}").unwrap();
        let f = p
            .functions
            .entries
            .iter_mut()
            .find(|f| f.name == "main")
            .unwrap();
        let call = f
            .block
            .instructions
            .iter_mut()
            .find(|i| matches!(i.op, Op::CallDirect { .. }))
            .unwrap();
        if let Op::CallDirect { signature, .. } = &mut call.op {
            signature[0].0 = "renamed".into();
        }
        f.content_hash = super::super::model::content_hash(f);
        assert_eq!(verify_program(&p), Err("callee signature"));
    }
    #[test]
    fn verifier_rejects_consuming_a_borrowed_argument() {
        let got = changed_main(
            "fn f(consume x:i64){} fn main(){let x=1;f(consume x);}",
            |f| {
                let transfer = f
                    .block
                    .instructions
                    .iter_mut()
                    .find(|i| matches!(i.op, Op::TransferValue(_)))
                    .unwrap();
                transfer.op = Op::BorrowValue(super::super::model::LocalId(0));
                transfer.access = Access::Read;
            },
        );
        assert_eq!(got, Err("consuming borrowed argument"));
    }
    #[test]
    fn verifier_rejects_transfer_after_local_was_consumed() {
        let got = changed("fn main(){var x=1;let y=consume x;x=2;}", |f| {
            let write = f
                .block
                .instructions
                .iter_mut()
                .filter(|i| {
                    matches!(
                        i.op,
                        Op::WritePlace {
                            local: super::super::model::LocalId(0),
                            ..
                        }
                    )
                })
                .nth(1)
                .unwrap();
            write.op = Op::TransferValue(super::super::model::LocalId(0));
            write.output = Some(TempId(3));
            write.ty = Some(Ty::I64);
            write.access = Access::Own;
        });
        assert_eq!(got, Err("transfer consumed local"));
    }
    #[test]
    fn verifier_rejects_drop_of_read_parameter() {
        let p = compile_scalar("v.nx", "fn f(x:i64){} fn main(){f(1);}").unwrap();
        let mut f = p.functions.function("f").unwrap().clone();
        let unit = f
            .block
            .instructions
            .iter_mut()
            .find(|i| matches!(i.op, Op::ConstUnit))
            .unwrap();
        unit.op = Op::DropBinding(super::super::model::LocalId(0));
        f.content_hash = super::super::model::content_hash(&f);
        assert_eq!(verify_function(&f), Err("drop read parameter"));
    }
    #[test]
    fn verifier_checks_call_scope_origin_and_source_span() {
        let p = compile_scalar("v.nx", "fn f(){} fn main(){f();}").unwrap();
        let original = p.functions.function("main").unwrap();
        let mut f = original.clone();
        let call = f
            .block
            .instructions
            .iter_mut()
            .find(|i| matches!(i.op, Op::CallDirect { .. }))
            .unwrap();
        call.scope.0 = f.scope_count;
        f.content_hash = super::super::model::content_hash(&f);
        assert_eq!(verify_function(&f), Err("instruction scope"));
        let mut f = original.clone();
        let call = f
            .block
            .instructions
            .iter_mut()
            .find(|i| matches!(i.op, Op::CallDirect { .. }))
            .unwrap();
        call.origin.0 = 1;
        assert_eq!(verify_function(&f), Err("origin"));
        let mut f = original.clone();
        let call = f
            .block
            .instructions
            .iter_mut()
            .find(|i| matches!(i.op, Op::CallDirect { .. }))
            .unwrap();
        call.span.end = 1000;
        assert_eq!(verify_function(&f), Err("source origin"));
    }
    #[test]
    fn verifier_rejects_call_when_callee_parameter_count_changed() {
        let mut p = compile_scalar("v.nx", "fn f(x:i64){} fn main(){f(1);}").unwrap();
        let f = p
            .functions
            .entries
            .iter_mut()
            .find(|f| f.name == "f")
            .unwrap();
        let id = super::super::model::LocalId(1);
        f.locals.push(super::super::model::Local {
            id,
            name: "y".into(),
            ty: Ty::I64,
            mutable: false,
            scope: super::super::model::ScopeId(0),
            parameter_mode: Some(ParameterMode::Read),
        });
        f.parameters.push(super::super::model::Parameter {
            local: id,
            mode: ParameterMode::Read,
        });
        f.cleanup.insert(0, id);
        f.content_hash = super::super::model::content_hash(f);
        assert_eq!(verify_function(f), Ok(()));
        assert_eq!(verify_program(&p), Err("callee signature"));
    }
    #[test]
    fn verifier_rejects_call_when_callee_result_changed() {
        let mut p = compile_scalar("v.nx", "fn f(){} fn main(){f();}").unwrap();
        let donor = compile_scalar("v.nx", "fn f()->i64{return 1;} fn main(){f();}").unwrap();
        p.functions.entries[0] = donor.functions.function("f").unwrap().clone();
        assert_eq!(verify_program(&p), Err("callee signature"));
    }
    #[test]
    fn verifier_rejects_call_result_instruction_type_mutation() {
        let mut program =
            compile_scalar("v.nx", "fn f()->i64{return 1;} fn main(){print(f());}").unwrap();
        let caller = program
            .functions
            .entries
            .iter_mut()
            .find(|f| f.name == "main")
            .unwrap();
        let call = caller
            .block
            .instructions
            .iter_mut()
            .find(|ins| matches!(ins.op, Op::CallDirect { .. }))
            .unwrap();
        call.ty = Some(Ty::Bool);
        caller.content_hash = super::super::model::content_hash(caller);
        assert!(verify_program(&program).is_err());
    }
    #[test]
    fn verifier_rejects_transfer_from_read_parameter() {
        let p = compile_scalar("v.nx", "fn f(x:()){} fn main(){f(());}").unwrap();
        let mut f = p.functions.function("f").unwrap().clone();
        let unit = f
            .block
            .instructions
            .iter_mut()
            .find(|i| matches!(i.op, Op::ConstUnit))
            .unwrap();
        unit.op = Op::TransferValue(super::super::model::LocalId(0));
        f.content_hash = super::super::model::content_hash(&f);
        assert_eq!(verify_function(&f), Err("transfer read parameter"));
    }
    #[test]
    fn verifier_rejects_temporary_array_containing_resource() {
        let p = compile_scalar("v.nx", "fn main(){drop(mock_open(1));}").unwrap();
        let mut f = p.functions.function("main").unwrap().clone();
        let drop = f
            .block
            .instructions
            .iter_mut()
            .find(|i| {
                matches!(
                    i.op,
                    Op::CallIntrinsic {
                        intrinsic: Intrinsic::DropData,
                        ..
                    }
                )
            })
            .unwrap();
        drop.op = Op::BuildArray(vec![TempId(1)]);
        drop.ty = Some(Ty::Array(Box::new(Ty::MockResource)));
        f.content_hash = super::super::model::content_hash(&f);
        assert_eq!(verify_function(&f), Err("resource array"));
    }
    #[test]
    fn verifier_rejects_resource_array_in_local_layout() {
        let p = compile_scalar("v.nx", "fn main(){let x=1;}").unwrap();
        let mut f = p.functions.function("main").unwrap().clone();
        f.locals[0].ty = Ty::Array(Box::new(Ty::MockResource));
        f.content_hash = super::super::model::content_hash(&f);
        assert_eq!(verify_function(&f), Err("resource array local"));
    }
    #[test]
    fn verifier_rejects_resource_array_in_result_layout() {
        let p = compile_scalar("v.nx", "fn main(){}").unwrap();
        let mut f = p.functions.function("main").unwrap().clone();
        f.result = Ty::Array(Box::new(Ty::MockResource));
        f.content_hash = super::super::model::content_hash(&f);
        assert_eq!(verify_function(&f), Err("resource array result"));
    }
    #[test]
    fn verifier_rejects_transfer_while_read_loan_remains_live() {
        let p = compile_scalar("v.nx", "fn main(){let x=1;print(x);let y=consume x;}").unwrap();
        let mut f = p.functions.function("main").unwrap().clone();
        let call = f
            .block
            .instructions
            .iter_mut()
            .find(|i| {
                matches!(
                    i.op,
                    Op::CallIntrinsic {
                        intrinsic: Intrinsic::Print,
                        ..
                    }
                )
            })
            .unwrap();
        call.op = Op::ConstUnit;
        call.effect_io = false;
        f.content_hash = super::super::model::content_hash(&f);
        assert_eq!(verify_function(&f), Err("transfer while borrowed"));
    }
}
