use super::model::{Access, Binary, Function, Intrinsic, Op, Program, TempId, Terminator, Ty};

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
    if super::model::content_hash(function) != function.content_hash {
        return Err("content hash");
    }
    for (i, local) in function.locals.iter().enumerate() {
        if local.id.0 != i {
            return Err("local identity");
        }
        if local.scope.0 >= function.scope_count {
            return Err("local scope");
        }
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
    let mut initialized = vec![false; function.locals.len()];
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
                if !initialized[id.0] {
                    return Err("uninitialized local");
                }
                Some(local.ty.clone())
            }
            Op::BorrowValue(id) => {
                let local = function.locals.get(id.0).ok_or("local reference")?;
                if !initialized[id.0] {
                    return Err("uninitialized local");
                }
                if !matches!(local.ty, Ty::Array(_)) {
                    return Err("borrow domain");
                }
                Some(local.ty.clone())
            }
            Op::BuildArray(items) => {
                let Some(Ty::Array(element)) = &ins.ty else {
                    return Err("array result type");
                };
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
                    Intrinsic::Print | Intrinsic::DropData | Intrinsic::Len => 1,
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
                Some(if *intrinsic == Intrinsic::Len {
                    Ty::I64
                } else {
                    Ty::Unit
                })
            }
            Op::DropValue(id) => {
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
                    intrinsic: Intrinsic::Print,
                    ..
                }
            )
        {
            return Err("effect summary");
        }
        let access = match ins.op {
            Op::CopyValue(_)
            | Op::BorrowValue(_)
            | Op::ReadIndex { .. }
            | Op::BorrowIndex { .. }
            | Op::CheckIndex { .. } => Access::Read,
            Op::WritePlace { .. } | Op::WriteIndex { .. } => Access::Write,
            _ => Access::Own,
        };
        if ins.access != access || !ins.tick {
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
                Op::BorrowValue(_) | Op::BorrowIndex { .. }
            ));
            let source = match ins.op {
                Op::BorrowValue(id) => Some(id),
                Op::BorrowIndex { local, .. } => Some(local),
                _ => None,
            };
            if let Some(id) = source {
                active_borrows[id.0] = active_borrows[id.0].checked_add(1).ok_or("borrow count")?;
            }
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
    fn verifier_rejects_borrow_of_scalar_local() {
        let got = changed("fn main(){let a=1;print(a);}", |f| {
            let i = f
                .block
                .instructions
                .iter_mut()
                .find(|i| matches!(i.op, Op::CopyValue(_)))
                .unwrap();
            i.op = Op::BorrowValue(super::super::model::LocalId(0));
        });
        assert_eq!(got, Err("borrow domain"));
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
}
