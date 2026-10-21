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
    let mut initialized = vec![false; function.locals.len()];
    let take = |id: TempId, defined: &[Ty], live: &mut [bool]| -> Result<Ty, &'static str> {
        let ty = defined.get(id.0).copied().ok_or("undefined temporary")?;
        let available = live.get_mut(id.0).ok_or("temporary liveness")?;
        if !*available {
            return Err("temporary used twice");
        }
        *available = false;
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
                Some(local.ty)
            }
            Op::Binary { op, left, right } => {
                let l = take(*left, &temps, &mut live)?;
                let r = take(*right, &temps, &mut live)?;
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
                if initialized[local.0] && !l.mutable {
                    return Err("immutable rewrite");
                }
                if take(*value, &temps, &mut live)? != l.ty {
                    return Err("write type");
                }
                initialized[local.0] = true;
                None
            }
            Op::CallIntrinsic { intrinsic, args } => {
                let arity = match intrinsic {
                    Intrinsic::Print | Intrinsic::DropData => 1,
                    Intrinsic::AssertEq => 2,
                    Intrinsic::Fail => 0,
                };
                if args.len() != arity {
                    return Err("intrinsic arity");
                }
                if let Some(first_id) = args.first() {
                    let first = take(*first_id, &temps, &mut live)?;
                    if *intrinsic == Intrinsic::AssertEq
                        && take(args[1], &temps, &mut live)? != first
                    {
                        return Err("assert type");
                    }
                }
                Some(Ty::Unit)
            }
            Op::DropValue(id) => {
                take(*id, &temps, &mut live)?;
                None
            }
            Op::DropLocal(id) => {
                function.locals.get(id.0).ok_or("drop local")?;
                if !initialized[id.0] {
                    return Err("drop uninitialized local");
                }
                initialized[id.0] = false;
                None
            }
        };
        if expected != ins.ty {
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
            Op::CopyValue(_) => Access::Read,
            Op::WritePlace { .. } => Access::Write,
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
            if take(*value, &temps, &mut live)? != function.result {
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
}
