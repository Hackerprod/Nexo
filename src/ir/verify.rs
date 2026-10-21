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
        for ins in &function.block.instructions {
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
                    return Err("drop exclusive loan");
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ir::compile_scalar;
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
        let Terminator::Return { value, .. } = &mut f.block.terminator;
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
        let Terminator::Return { span, .. } = &mut f.block.terminator;
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
