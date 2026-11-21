//! Canonical source printer for accepted normalized syntax trees.
use super::cst::Cst;
use super::emit::{Role, SourceToken, Writer};
use super::parse::*;
use super::precedence::{self, EXPRESSION};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PrintWorkerUnavailable;

pub fn print_canonical(ast: &Ast) -> Result<String, PrintWorkerUnavailable> {
    print_spawn(ast, None, true).map(|writer| writer.output)
}
pub(crate) fn print_with_cst(cst: &Cst) -> Result<Writer, PrintWorkerUnavailable> {
    print_spawn(&cst.ast, Some(cst), true)
}
fn print_spawn(
    ast: &Ast,
    cst: Option<&Cst>,
    allowed: bool,
) -> Result<Writer, PrintWorkerUnavailable> {
    if !allowed {
        return Err(PrintWorkerUnavailable);
    }
    std::thread::scope(|scope| {
        let handle = std::thread::Builder::new()
            .stack_size(crate::frontend_stack_size("FORMATTER"))
            .spawn_scoped(scope, || print_inner(ast, cst))
            .map_err(|_| PrintWorkerUnavailable)?;
        match handle.join() {
            Ok(text) => Ok(text),
            Err(payload) => std::panic::resume_unwind(payload),
        }
    })
}
#[cfg(test)]
pub(crate) fn simulate_unavailable_worker(ast: &Ast) -> Result<String, PrintWorkerUnavailable> {
    print_spawn(ast, None, false).map(|writer| writer.output)
}
fn at(out: &mut Writer, text: &str, role: Role, start: usize, end: usize) {
    out.token_at(text, role, false, start, end);
}
fn after(out: &mut Writer, text: &str, role: Role, bound: usize) -> usize {
    out.token_after(text, role, bound)
}
fn before(out: &mut Writer, text: &str, role: Role, bound: usize) -> usize {
    out.token_before(text, role, bound)
}
fn inserted(out: &mut Writer, text: &str, role: Role) {
    out.token(text, role, false);
}
fn word_at(out: &mut Writer, text: &str, start: usize, end: usize) {
    at(out, text, Role::Word, start, end);
}
fn mode(mode: Mode) -> &'static str {
    match mode {
        Mode::Read => "view",
        Mode::Inout => "edit",
        Mode::Consume => "move",
    }
}
fn print_inner(ast: &Ast, cst: Option<&Cst>) -> Writer {
    #[cfg(test)]
    let _allocation_capture = crate::test_alloc::capture_for(&ast.span.file);
    let mut out = if let Some(cst) = cst {
        Writer::with_source(
            cst.tokens
                .iter()
                .map(|t| SourceToken {
                    start: t.span.start as usize,
                    end: t.span.end as usize,
                    id: t.id,
                    text: t.text.clone(),
                })
                .collect(),
            cst.comments.iter().map(|c| (c.start, c.end)).collect(),
            #[cfg(test)]
            &ast.span.file,
        )
    } else {
        Writer::default()
    };
    enum ModuleItem<'a> {
        Function(&'a Function),
        Constant(&'a ConstDecl),
    }
    let mut items: Vec<_> = ast.functions.iter().map(ModuleItem::Function).collect();
    items.extend(ast.consts.iter().map(ModuleItem::Constant));
    items.sort_by_key(|item| match item {
        ModuleItem::Function(function) => function.span.start,
        ModuleItem::Constant(constant) => constant.span.start,
    });
    for item in items {
        let function = match item {
            ModuleItem::Function(function) => function,
            ModuleItem::Constant(constant) => {
                at(
                    &mut out,
                    "const",
                    Role::Word,
                    constant.const_span.start as usize,
                    constant.const_span.end as usize,
                );
                word_at(
                    &mut out,
                    &constant.name,
                    constant.name_span.start as usize,
                    constant.name_span.end as usize,
                );
                let end = after(&mut out, ":", Role::Colon, constant.name_span.end as usize);
                let end = ty(&constant.ty, &mut out, end);
                after(&mut out, "=", Role::Binary, end);
                at(
                    &mut out,
                    "comptime",
                    Role::Word,
                    constant.comptime_span.start as usize,
                    constant.comptime_span.end as usize,
                );
                run_print_frames(Frame::Visit(&constant.value, EXPRESSION), &mut out, true);
                out.terminator(constant.span.end as usize);
                continue;
            }
        };
        let start = function.span.start as usize;
        let fn_start = if function.is_generator {
            at(&mut out, "gen", Role::Word, start, start + 3);
            start + 4
        } else {
            start
        };
        at(&mut out, "fn", Role::Function, fn_start, fn_start + 2);
        word_at(
            &mut out,
            &function.name,
            function.name_span.start as usize,
            function.name_span.end as usize,
        );
        let mut end = after(
            &mut out,
            "(",
            Role::AttachedOpen,
            function.name_span.end as usize,
        );
        for (i, parameter) in function.params.iter().enumerate() {
            if i > 0 {
                end = after(&mut out, ",", Role::Comma, end);
            }
            end = after(&mut out, &parameter.name, Role::Word, end);
            end = after(&mut out, ":", Role::Colon, end);
            end = after(&mut out, mode(parameter.mode), Role::Word, end);
            end = ty(&parameter.ty, &mut out, end);
        }
        end = after(&mut out, ")", Role::Close, end);
        if let Some(result) = &function.result {
            end = after(&mut out, "->", Role::Arrow, end);
            end = ty(result, &mut out, end);
        }
        if let Some(clause) = &function.with_clause {
            end = after(&mut out, "with", Role::Word, end);
            match clause {
                WithClause::Simple(items) => {
                    for (i, item) in items.iter().enumerate() {
                        if i > 0 {
                            end = after(&mut out, ",", Role::Comma, end);
                        }
                        end = after(&mut out, &item.name, Role::Word, end);
                    }
                }
                WithClause::Operations(operations) => {
                    end = after(&mut out, "{", Role::ContractOpen, end);
                    for (i, operation) in operations.iter().enumerate() {
                        if i > 0 {
                            end = after(&mut out, ",", Role::Comma, end);
                        }
                        end = after(&mut out, &operation.name, Role::Word, end);
                        end = after(&mut out, "(", Role::AttachedOpen, end);
                        for (j, restriction) in operation.restrictions.iter().enumerate() {
                            if j > 0 {
                                end = after(&mut out, ",", Role::Comma, end);
                            }
                            end = after(&mut out, &restriction.name, Role::Word, end);
                        }
                        end = after(&mut out, ")", Role::Close, end);
                    }
                    after(&mut out, "}", Role::ContractClose, end);
                }
            }
        }
        block(&function.body, &mut out, true, ast.f5_operators);
    }
    out.line();
    out
}
fn ty(value: &Type, out: &mut Writer, bound: usize) -> usize {
    match value {
        Type::I64 => after(out, "I64", Role::Word, bound),
        Type::Bool => after(out, "Bool", Role::Word, bound),
        Type::Unit => {
            let end = after(out, "(", Role::Open, bound);
            after(out, ")", Role::Close, end)
        }
        Type::MockResource => after(out, "MockResource", Role::Word, bound),
        Type::MockBlockingResource => after(out, "MockBlockingResource", Role::Word, bound),
        Type::Array(inner) => {
            let end = after(out, "Array", Role::Word, bound);
            let end = after(out, "<", Role::TypeOpen, end);
            let end = ty(inner, out, end);
            after(out, ">", Role::TypeClose, end)
        }
        Type::Result(ok, err) => {
            let end = after(out, "Result", Role::Word, bound);
            let end = after(out, "<", Role::TypeOpen, end);
            let end = ty(ok, out, end);
            let end = after(out, ",", Role::Comma, end);
            let end = ty(err, out, end);
            after(out, ">", Role::TypeClose, end)
        }
        Type::Option(some) => {
            let end = after(out, "Option", Role::Word, bound);
            let end = after(out, "<", Role::TypeOpen, end);
            let end = ty(some, out, end);
            after(out, ">", Role::TypeClose, end)
        }
        Type::Closure { .. } => after(out, "Closure", Role::Word, bound),
        Type::Generator { .. } => after(out, "Generator", Role::Word, bound),
    }
}
fn block(value: &Block, out: &mut Writer, structural: bool, f5_operators: bool) {
    run_print_frames(Frame::Block(value, structural), out, f5_operators);
}
enum Frame<'a> {
    Visit(&'a Expr, u8),
    Stmt(&'a Statement),
    StmtElse(&'a Statement),
    StmtIf(&'a Expr),
    Argument(&'a Argument),
    Else(&'a Else),
    Branch(&'a Block, bool),
    Match(&'a Match),
    Arm(&'a Arm, Option<usize>),
    Block(&'a Block, bool),
    CloseBlock(usize, bool),
    Before(&'static str, Role, usize),
    Terminator(usize),
    After(&'a str, Role, usize),
    At(&'static str, Role, usize, usize),
    Inserted(&'static str, Role),
    GroupEnd,
}

fn run_print_frames(root: Frame<'_>, out: &mut Writer, f5_operators: bool) {
    let levels = if f5_operators {
        precedence::F5
    } else {
        precedence::J
    };
    let mut frames = vec![root];
    while let Some(frame) = frames.pop() {
        let (value, parent) = match frame {
            Frame::Visit(value, parent) => (value, parent),
            Frame::Argument(argument) => {
                if let Some(label) = &argument.label {
                    let end = after(out, label, Role::Word, argument.span.start as usize);
                    after(out, ":", Role::Colon, end);
                }
                frames.push(Frame::Visit(&argument.value, EXPRESSION));
                continue;
            }
            Frame::Else(value) => {
                match value {
                    Else::Block(value) => frames.push(Frame::Branch(value, true)),
                    Else::If(value) => {
                        inserted(out, "(", Role::Open);
                        frames.push(Frame::Inserted(")", Role::Close));
                        frames.push(Frame::Visit(value, EXPRESSION));
                    }
                }
                continue;
            }
            Frame::Branch(branch, is_else) => {
                if let [
                    Statement::Value {
                        value: Some(value),
                        marker,
                        ..
                    },
                ] = branch.statements.as_slice()
                {
                    let mut inner = value;
                    while let Expr::Group(grouped, _) = inner {
                        inner = grouped;
                    }
                    let explicit = marker.is_some();
                    if !(is_else && matches!(inner, Expr::If { .. }))
                        && (!explicit
                            || (!out
                                .has_comment(branch.span.start as usize, branch.span.end as usize)
                                && !out.has_comment_before(branch.span.start as usize)
                                && !out.has_comment_after_in_group(branch.span.end as usize)))
                    {
                        if matches!(value, Expr::If { .. }) {
                            inserted(out, "(", Role::Open);
                            frames.push(Frame::Inserted(")", Role::Close));
                        }
                        frames.push(Frame::Visit(value, EXPRESSION));
                        continue;
                    }
                }
                frames.push(Frame::Block(branch, false));
                continue;
            }
            Frame::Match(selection) => {
                let start = selection.keyword_span.start as usize;
                at(out, "match", Role::Word, start, start + 5);
                if let Some(mode_span) = &selection.mode_span {
                    let word = mode(selection.mode.unwrap_or(Mode::Read));
                    at(
                        out,
                        word,
                        Role::Word,
                        mode_span.start as usize,
                        mode_span.end as usize,
                    );
                }
                let end = selection.span.end as usize;
                frames.push(Frame::At("}", Role::BlockClose, end - 1, end));
                for index in (0..selection.arms.len()).rev() {
                    let comma =
                        (index > 0).then(|| selection.arms[index - 1].body.span.end as usize);
                    frames.push(Frame::Arm(&selection.arms[index], comma));
                }
                frames.push(Frame::After(
                    "{",
                    Role::BlockOpen,
                    selection.scrutinee.span().end as usize,
                ));
                frames.push(Frame::Visit(&selection.scrutinee, EXPRESSION));
                continue;
            }
            Frame::Arm(arm, comma) => {
                if let Some(bound) = comma {
                    after(out, ",", Role::Comma, bound);
                }
                print_pattern(&arm.pattern, out);
                frames.push(Frame::Block(&arm.body, false));
                frames.push(Frame::After(
                    "=>",
                    Role::Arrow,
                    arm.pattern.span().end as usize,
                ));
                if let (Some(guard), Some(span)) = (&arm.guard, &arm.guard_span) {
                    at(
                        out,
                        "if",
                        Role::Word,
                        span.start as usize,
                        span.end as usize,
                    );
                    frames.push(Frame::Visit(guard, EXPRESSION));
                }
                continue;
            }
            Frame::Block(value, structural) => {
                let mut current = value;
                let mut outers = Vec::new();
                loop {
                    let start = current.span.start as usize;
                    at(out, "{", Role::BlockOpen, start, start + 1);
                    if let [Statement::Block(next)] = current.statements.as_slice() {
                        outers.push(current.span.end as usize);
                        current = next;
                        continue;
                    }
                    break;
                }
                let nested = !outers.is_empty();
                for (index, end) in outers.into_iter().enumerate() {
                    frames.push(Frame::CloseBlock(end, structural && index == 0));
                }
                frames.push(Frame::CloseBlock(
                    current.span.end as usize,
                    structural && !nested,
                ));
                for statement in current.statements.iter().rev() {
                    frames.push(Frame::Stmt(statement));
                }
                continue;
            }
            Frame::CloseBlock(end, structural) => {
                out.token_at("}", Role::BlockClose, structural, end - 1, end);
                continue;
            }
            Frame::Stmt(statement) => {
                match statement {
                    Statement::Let {
                        mutable,
                        name,
                        annotation,
                        value,
                        span,
                    } => {
                        let start = span.start as usize;
                        let keyword = if *mutable { "var" } else { "let" };
                        at(out, keyword, Role::Word, start, start + keyword.len());
                        let name_end = after(out, name, Role::Word, start + keyword.len());
                        if let Some(annotation) = annotation {
                            let colon_end = after(out, ":", Role::Colon, name_end);
                            ty(annotation, out, colon_end);
                        }
                        frames.push(Frame::Terminator(span.end as usize));
                        frames.push(Frame::Visit(value, EXPRESSION));
                        frames.push(Frame::Before(
                            "=",
                            Role::Binary,
                            value.span().start as usize,
                        ));
                    }
                    Statement::Assign { place, value, span } => {
                        frames.push(Frame::Terminator(span.end as usize));
                        frames.push(Frame::Visit(value, EXPRESSION));
                        frames.push(Frame::After("=", Role::Binary, place.span().end as usize));
                        frames.push(Frame::Visit(place, EXPRESSION));
                    }
                    Statement::Return { value, span } => {
                        let start = span.start as usize;
                        at(out, "return", Role::Word, start, start + 6);
                        frames.push(Frame::Terminator(span.end as usize));
                        if let Some(value) = value {
                            frames.push(Frame::Visit(value, EXPRESSION));
                        }
                    }
                    Statement::GuardControl {
                        token,
                        value,
                        span,
                        token_span,
                        ..
                    } => {
                        at(
                            out,
                            token,
                            Role::Word,
                            token_span.start as usize,
                            token_span.end as usize,
                        );
                        frames.push(Frame::Terminator(span.end as usize));
                        if let Some(value) = value {
                            frames.push(Frame::Visit(value, EXPRESSION));
                        }
                    }
                    Statement::Value {
                        value,
                        marker,
                        span,
                        ..
                    } => {
                        if let Some(marker) = marker {
                            let start = marker.start as usize;
                            at(out, "value", Role::Word, start, start + 5);
                            after(out, ":", Role::Colon, start + 5);
                        }
                        frames.push(Frame::Terminator(span.end as usize));
                        if let Some(value) = value {
                            frames.push(Frame::Visit(value, EXPRESSION));
                        }
                    }
                    Statement::Expr { value, span } => {
                        frames.push(Frame::Terminator(span.end as usize));
                        frames.push(Frame::Visit(
                            value,
                            u8::from(matches!(value, Expr::If { .. })),
                        ));
                    }
                    Statement::Block(value) => frames.push(Frame::Block(value, true)),
                    Statement::While {
                        condition, body, ..
                    } => {
                        let start = statement_span(statement).start as usize;
                        at(out, "while", Role::Word, start, start + 5);
                        frames.push(Frame::Block(body, true));
                        frames.push(Frame::Visit(condition, EXPRESSION));
                    }
                    Statement::For {
                        name,
                        name_span,
                        iterable,
                        body,
                        ..
                    } => {
                        let start = statement_span(statement).start as usize;
                        at(out, "for", Role::Word, start, start + 3);
                        at(
                            out,
                            name,
                            Role::Word,
                            name_span.start as usize,
                            name_span.end as usize,
                        );
                        after(out, "in", Role::Word, name_span.end as usize);
                        frames.push(Frame::Block(body, true));
                        frames.push(Frame::Visit(iterable, EXPRESSION));
                    }
                    Statement::If {
                        condition,
                        then_block,
                        otherwise,
                        ..
                    } => {
                        let start = statement_span(statement).start as usize;
                        at(out, "if", Role::Word, start, start + 2);
                        if let Some(otherwise) = otherwise {
                            frames.push(Frame::StmtElse(otherwise));
                            frames.push(Frame::After(
                                "else",
                                Role::Else,
                                then_block.span.end as usize,
                            ));
                        }
                        frames.push(Frame::Block(then_block, true));
                        frames.push(Frame::Visit(condition, EXPRESSION));
                    }
                    Statement::Match(selection) => frames.push(Frame::Match(selection)),
                }
                continue;
            }
            Frame::StmtElse(statement) => {
                match statement {
                    Statement::Block(value) => frames.push(Frame::Block(value, true)),
                    Statement::Expr {
                        value: value @ Expr::If { .. },
                        ..
                    } => frames.push(Frame::StmtIf(value)),
                    Statement::Expr { value, .. } => frames.push(Frame::Visit(value, EXPRESSION)),
                    other => frames.push(Frame::Stmt(other)),
                }
                continue;
            }
            Frame::StmtIf(value) => {
                if let Expr::If {
                    condition,
                    then_block,
                    otherwise,
                    ..
                } = value
                {
                    let start = value.span().start as usize;
                    at(out, "if", Role::Word, start, start + 2);
                    if let Some(otherwise) = otherwise {
                        match &**otherwise {
                            Else::Block(value) => frames.push(Frame::Block(value, true)),
                            Else::If(value) => frames.push(Frame::StmtIf(value)),
                        }
                        frames.push(Frame::After(
                            "else",
                            Role::Else,
                            then_block.span.end as usize,
                        ));
                    }
                    frames.push(Frame::Block(then_block, true));
                    frames.push(Frame::Visit(condition, EXPRESSION));
                }
                continue;
            }
            Frame::Before(text, role, bound) => {
                before(out, text, role, bound);
                continue;
            }
            Frame::Terminator(bound) => {
                out.terminator(bound);
                continue;
            }
            Frame::After(text, role, bound) => {
                after(out, text, role, bound);
                continue;
            }
            Frame::At(text, role, start, end) => {
                at(out, text, role, start, end);
                continue;
            }
            Frame::Inserted(text, role) => {
                inserted(out, text, role);
                continue;
            }
            Frame::GroupEnd => {
                out.group_end();
                continue;
            }
        };
        match value {
            Expr::Closure(closure) => {
                let start = closure.fn_span.start as usize;
                at(out, "fn", Role::Function, start, start + 2);
                let mut end = closure.fn_span.end as usize;
                if let Some(captures) = &closure.captures {
                    end = after(out, "[", Role::AttachedOpen, end);
                    for (index, capture) in captures.iter().enumerate() {
                        if index > 0 {
                            end = after(out, ",", Role::Comma, end);
                        }
                        let mode = match capture.mode {
                            CaptureMode::View => "view",
                            CaptureMode::Copy => "copy",
                            CaptureMode::Edit => "edit",
                            CaptureMode::Move => "move",
                        };
                        end = after(out, mode, Role::Word, end);
                        end = after(out, &capture.name, Role::Word, end);
                    }
                    end = after(out, "]", Role::Close, end);
                }
                end = after(out, "(", Role::AttachedOpen, end);
                for (index, param) in closure.params.iter().enumerate() {
                    if index > 0 {
                        end = after(out, ",", Role::Comma, end);
                    }
                    end = after(out, &param.name, Role::Word, end);
                    end = after(out, ":", Role::Colon, end);
                    end = after(out, mode(param.mode), Role::Word, end);
                    if let Some(param_type) = &param.ty {
                        end = ty(param_type, out, end);
                    }
                }
                end = after(out, ")", Role::Close, end);
                if let Some(result) = &closure.result {
                    end = after(out, "->", Role::Arrow, end);
                    end = ty(result, out, end);
                }
                if let Some(clause) = &closure.with_clause {
                    end = after(out, "with", Role::Word, end);
                    match clause {
                        WithClause::Simple(items) => {
                            for (index, restriction) in items.iter().enumerate() {
                                if index > 0 {
                                    end = after(out, ",", Role::Comma, end);
                                }
                                end = after(out, &restriction.name, Role::Word, end);
                            }
                        }
                        WithClause::Operations(operations) => {
                            end = after(out, "{", Role::ContractOpen, end);
                            for (index, operation) in operations.iter().enumerate() {
                                if index > 0 {
                                    end = after(out, ",", Role::Comma, end);
                                }
                                end = after(out, &operation.name, Role::Word, end);
                                end = after(out, "(", Role::AttachedOpen, end);
                                for (index, restriction) in
                                    operation.restrictions.iter().enumerate()
                                {
                                    if index > 0 {
                                        end = after(out, ",", Role::Comma, end);
                                    }
                                    end = after(out, &restriction.name, Role::Word, end);
                                }
                                end = after(out, ")", Role::Close, end);
                            }
                            after(out, "}", Role::ContractClose, end);
                        }
                    }
                }
                frames.push(Frame::Block(&closure.body, false));
            }
            Expr::Name(name, span) => word_at(out, name, span.start as usize, span.end as usize),
            Expr::Integer(name, span) => out.token_from_span(
                name,
                Role::Word,
                false,
                span.start as usize,
                span.end as usize,
            ),
            Expr::Bool(value, span) => word_at(
                out,
                if *value { "true" } else { "false" },
                span.start as usize,
                span.end as usize,
            ),
            Expr::Unit(span) => {
                let start = span.start as usize;
                let end = span.end as usize;
                at(out, "(", Role::Open, start, start + 1);
                at(out, ")", Role::Close, end - 1, end);
            }
            Expr::Array(items, span) => {
                let mut closes = vec![span.end as usize];
                let mut elements = items.as_slice();
                let mut start = span.start as usize;
                while let [Expr::Array(inner, inner_span)] = elements {
                    at(out, "[", Role::Open, start, start + 1);
                    start = inner_span.start as usize;
                    closes.push(inner_span.end as usize);
                    elements = inner;
                }
                at(out, "[", Role::Open, start, start + 1);
                for end in closes {
                    frames.push(Frame::At("]", Role::Close, end - 1, end));
                }
                for index in (0..elements.len()).rev() {
                    frames.push(Frame::Visit(&elements[index], EXPRESSION));
                    if index > 0 {
                        frames.push(Frame::After(
                            ",",
                            Role::Comma,
                            elements[index - 1].span().end as usize,
                        ));
                    }
                }
            }
            Expr::Group(inner, span) => {
                out.group_start(span.end as usize);
                let mut groups = vec![(span.start as usize, span.end as usize)];
                let mut item = inner.as_ref();
                while let Expr::Group(next, span) = item {
                    groups.push((span.start as usize, span.end as usize));
                    item = next;
                }
                for &(start, _) in &groups {
                    at(out, "(", Role::Open, start, start + 1);
                }
                frames.push(Frame::GroupEnd);
                for &(_, end) in &groups {
                    frames.push(Frame::At(")", Role::Close, end - 1, end));
                }
                frames.push(Frame::Visit(item, EXPRESSION));
            }
            Expr::Unary {
                mode: Mode::Read,
                value,
                span,
            } => {
                let end = span.end as usize;
                frames.push(Frame::At("?", Role::Suffix, end - 1, end));
                frames.push(Frame::Visit(value, levels.try_guard));
            }
            Expr::Neg { value, span } => {
                at(
                    out,
                    "-",
                    Role::Word,
                    span.start as usize,
                    span.start as usize + 1,
                );
                frames.push(Frame::Visit(value, levels.unary));
            }
            Expr::Not { value, span } => {
                at(
                    out,
                    "not",
                    Role::Word,
                    span.start as usize,
                    span.start as usize + 3,
                );
                frames.push(Frame::Visit(value, levels.unary));
            }
            Expr::Unary {
                mode: prefix,
                value,
                span,
            } => {
                let text = mode(*prefix);
                let start = span.start as usize;
                if f5_operators && parent == levels.try_guard {
                    inserted(out, "(", Role::Open);
                    frames.push(Frame::Inserted(")", Role::Close));
                }
                at(out, text, Role::Word, start, start + text.len());
                frames.push(Frame::Visit(value, levels.postfix));
            }
            Expr::Binary {
                op, left, right, ..
            } => {
                let p = levels.binary(op);
                let wrap = p < parent;
                if wrap {
                    inserted(out, "(", Role::Open);
                    frames.push(Frame::Inserted(")", Role::Close));
                }
                frames.push(Frame::Visit(right, p + 1));
                frames.push(Frame::After(op, Role::Binary, left.span().end as usize));
                frames.push(Frame::Visit(left, p + u8::from(p == levels.comparison)));
            }
            Expr::Call { target, args, span } => {
                let end = span.end as usize;
                frames.push(Frame::At(")", Role::Close, end - 1, end));
                for index in (0..args.len()).rev() {
                    frames.push(Frame::Argument(&args[index]));
                    if index > 0 {
                        frames.push(Frame::After(
                            ",",
                            Role::Comma,
                            args[index - 1].span.end as usize,
                        ));
                    }
                }
                frames.push(Frame::After(
                    "(",
                    Role::AttachedOpen,
                    target.span().end as usize,
                ));
                frames.push(Frame::Visit(target, levels.postfix));
            }
            Expr::Index {
                target,
                index,
                span,
                ..
            } => {
                let end = span.end as usize;
                frames.push(Frame::At("]", Role::Close, end - 1, end));
                frames.push(Frame::Visit(index, EXPRESSION));
                frames.push(Frame::After(
                    "[",
                    Role::AttachedOpen,
                    target.span().end as usize,
                ));
                frames.push(Frame::Visit(target, levels.postfix));
            }
            Expr::If {
                condition,
                then_block,
                otherwise,
                span,
            } => {
                if parent > 0 {
                    inserted(out, "(", Role::Open);
                    frames.push(Frame::Inserted(")", Role::Close));
                }
                let start = span.start as usize;
                at(out, "if", Role::Word, start, start + 2);
                if let Some(otherwise) = otherwise {
                    frames.push(Frame::Else(otherwise));
                    frames.push(Frame::After(
                        "else",
                        Role::Else,
                        then_block.span.end as usize,
                    ));
                }
                frames.push(Frame::Branch(then_block, false));
                frames.push(Frame::After(
                    "then",
                    Role::Word,
                    condition.span().end as usize,
                ));
                frames.push(Frame::Visit(condition, EXPRESSION));
            }
            Expr::Match(selection) => {
                if parent > 0 {
                    inserted(out, "(", Role::Open);
                    frames.push(Frame::Inserted(")", Role::Close));
                }
                frames.push(Frame::Match(selection));
            }
        }
    }
}

fn print_pattern(pattern: &Pattern, out: &mut Writer) {
    enum Frame<'a> {
        Enter(&'a Pattern),
        Comma(usize),
        Close(usize),
    }
    let mut frames = vec![Frame::Enter(pattern)];
    while let Some(frame) = frames.pop() {
        let pattern = match frame {
            Frame::Enter(pattern) => pattern,
            Frame::Comma(bound) => {
                after(out, ",", Role::Comma, bound);
                continue;
            }
            Frame::Close(bound) => {
                after(out, ")", Role::Close, bound);
                continue;
            }
        };
        match pattern {
            Pattern::Wildcard(span) => {
                at(out, "_", Role::Word, span.start as usize, span.end as usize)
            }
            Pattern::Integer(text, span) => at(
                out,
                text,
                Role::Word,
                span.start as usize,
                span.end as usize,
            ),
            Pattern::Bool(value, span) => at(
                out,
                if *value { "true" } else { "false" },
                Role::Word,
                span.start as usize,
                span.end as usize,
            ),
            Pattern::Name {
                name,
                mode: binding_mode,
                span,
            } => {
                if let Some(binding_mode) = binding_mode {
                    let word = mode(*binding_mode);
                    at(
                        out,
                        word,
                        Role::Word,
                        span.start as usize,
                        span.start as usize + word.len(),
                    );
                    after(out, name, Role::Word, span.start as usize + word.len());
                } else {
                    at(
                        out,
                        name,
                        Role::Word,
                        span.start as usize,
                        span.end as usize,
                    );
                }
            }
            Pattern::Constructor { name, args, span } => {
                at(
                    out,
                    name,
                    Role::Word,
                    span.start as usize,
                    span.start as usize + name.len(),
                );
                after(
                    out,
                    "(",
                    Role::AttachedOpen,
                    span.start as usize + name.len(),
                );
                frames.push(Frame::Close(span.end as usize - 1));
                for index in (0..args.len()).rev() {
                    frames.push(Frame::Enter(&args[index]));
                    if index > 0 {
                        frames.push(Frame::Comma(args[index - 1].span().end as usize));
                    }
                }
            }
        }
    }
}
#[cfg(test)]
mod printer_stack_regression {
    use super::*;
    use std::process::Command;

    #[test]
    fn commented_groups_child_print_with_204_kib() {
        if std::env::var_os("J16_PRINT_COMMENTS_CHILD").is_none() {
            return;
        }
        let source = format!(
            "fn Main(){{let x={}1{};}}",
            "(// comentario\n".repeat(126),
            ")".repeat(126)
        );
        let cst = crate::frontend::cst::parse_cst("comments-print.n", &source).unwrap();
        assert_eq!(cst.comments.len(), 126);
        let expected = print_inner(&cst.ast, Some(&cst)).output;
        let printed = std::thread::scope(|scope| {
            std::thread::Builder::new()
                .stack_size(204 * 1024)
                .spawn_scoped(scope, || print_inner(&cst.ast, Some(&cst)).output)
                .unwrap()
                .join()
                .unwrap()
        });
        assert_eq!(printed, expected);
        let formatted = crate::frontend::format::format_cst("comments-print.n", &cst).unwrap();
        assert!(formatted.contains("// comentario"));
    }

    #[test]
    fn commented_groups_print_with_204_kib() {
        let output = Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "frontend::print::printer_stack_regression::commented_groups_child_print_with_204_kib",
                "--nocapture",
            ])
            .env("J16_PRINT_COMMENTS_CHILD", "1")
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{} {}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }

    fn expression_source(family: &str) -> String {
        match family {
            "block" => format!(
                "fn Main(){{{}let x=1;{}}}",
                "{".repeat(126),
                "}".repeat(126)
            ),
            "if_stmt" => format!("fn Main(){{{}{}}}", "if true{".repeat(126), "}".repeat(126)),
            "while" => format!(
                "fn Main(){{{}{}}}",
                "while true{".repeat(126),
                "}".repeat(126)
            ),
            "group" => format!(
                "fn Main(){{let x={}1{};}}",
                "(".repeat(126),
                ")".repeat(126)
            ),
            "array" => format!(
                "fn Main(){{let x={}1{};}}",
                "[".repeat(126),
                "]".repeat(126)
            ),
            "binary_left" => format!("fn Main(){{let x=1{};}}", "+1".repeat(126)),
            "binary_right" => format!(
                "fn Main(){{let x={}1{};}}",
                "1+(".repeat(126),
                ")".repeat(126)
            ),
            "call" => format!(
                "fn F(x:view I64)->I64{{return x;}}fn Main(){{let x={}1{};}}",
                "F(".repeat(126),
                ")".repeat(126)
            ),
            "some" => format!(
                "fn Main(){{let x={}1{};}}",
                "Some(".repeat(126),
                ")".repeat(126)
            ),
            "some_annotated" => format!(
                "fn Main(){{let x:{}I64{}={}1{};}}",
                "Option<".repeat(126),
                ">".repeat(126),
                "Some(".repeat(126),
                ")".repeat(126)
            ),
            "index" => format!(
                "fn Main(){{let a={}1{};let x=a{};}}",
                "[".repeat(126),
                "]".repeat(126),
                "[0]".repeat(126)
            ),
            "if_expr" => {
                let mut value = String::from("1");
                for _ in 0..126 {
                    value = format!("if true then ({value}) else 0");
                }
                format!("fn Main(){{let x={value};}}")
            }
            "else_if_expr" => {
                let mut value = String::from("1");
                for _ in 0..63 {
                    value = format!("if false then 0 else ({value})");
                }
                format!("fn Main(){{let x={value};}}")
            }
            "match_scrutinee" => {
                let mut value = String::from("1");
                for _ in 0..60 {
                    value = format!("match ({value}){{_=>{{value:1;}}}}");
                }
                format!("fn Main(){{let x={value};}}")
            }
            "match_stmt" => {
                let mut body = String::new();
                for _ in 0..60 {
                    body = format!("match 1{{_=>{{{body}}}}}");
                }
                format!("fn Main(){{{body}}}")
            }
            "match_expr" => {
                let mut value = String::from("1");
                for _ in 0..63 {
                    value = format!("match 1{{_=>{{value:{value};}}}}");
                }
                format!("fn Main(){{let x={value};}}")
            }
            "closure" => format!(
                "fn Main(){{let f={}1{};}}",
                "fn(){let g=".repeat(63),
                ";}".repeat(63)
            ),
            "closure_called" => {
                let mut value = String::from("1");
                for _ in 0..40 {
                    value = format!("(fn ()->I64{{return {value};}})()");
                }
                format!("fn Main(){{let x={value};}}")
            }
            "type" => format!(
                "fn F(x:view {}I64{}){{}}fn Main(){{}}",
                "Array<".repeat(126),
                ">".repeat(126)
            ),
            "yield_group" => format!(
                "gen fn G()->I64{{yield {}1{};}}fn Main(){{}}",
                "(".repeat(126),
                ")".repeat(126)
            ),
            "mixed" => format!(
                "fn Main(){{{}let x={}1{};{}}}",
                "{".repeat(63),
                "(".repeat(63),
                ")".repeat(63),
                "}".repeat(63)
            ),
            _ => unreachable!(),
        }
    }

    #[test]
    fn expression_family_child_prints_with_204_kib() {
        let Ok(family) = std::env::var("J16_PRINT_EXPR_CHILD") else {
            return;
        };
        let source = expression_source(&family);
        let ast = parse_new_profile("expr-print.n", &source).unwrap();
        let expected = print_inner(&ast, None).output;
        let printed = std::thread::scope(|scope| {
            std::thread::Builder::new()
                .stack_size(204 * 1024)
                .spawn_scoped(scope, || print_inner(&ast, None).output)
                .unwrap()
                .join()
                .unwrap()
        });
        assert_eq!(printed, expected, "{family}");
    }

    #[test]
    fn expression_families_print_with_204_kib() {
        for family in [
            "block",
            "if_stmt",
            "while",
            "group",
            "array",
            "binary_left",
            "binary_right",
            "call",
            "some",
            "some_annotated",
            "index",
            "if_expr",
            "else_if_expr",
            "match_scrutinee",
            "match_stmt",
            "match_expr",
            "closure",
            "closure_called",
            "type",
            "yield_group",
            "mixed",
        ] {
            let output = Command::new(std::env::current_exe().unwrap())
                .args([
                    "--exact",
                    "frontend::print::printer_stack_regression::expression_family_child_prints_with_204_kib",
                    "--nocapture",
                ])
                .env("J16_PRINT_EXPR_CHILD", family)
                .output()
                .unwrap();
            assert!(
                output.status.success(),
                "{family}: {} {}",
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr)
            );
        }
    }

    #[test]
    fn else_if_child_prints_with_204_kib() {
        if std::env::var_os("J16_PRINT_ELSE_IF_CHILD").is_none() {
            return;
        }
        let source = format!("fn Main(){{{}{}}}", "if true{}else ".repeat(126), "{}");
        let ast = parse_new_profile("else-if-print.n", &source).unwrap();
        let expected = print_inner(&ast, None).output;
        let printed = std::thread::scope(|scope| {
            std::thread::Builder::new()
                .stack_size(204 * 1024)
                .spawn_scoped(scope, || print_inner(&ast, None).output)
                .unwrap()
                .join()
                .unwrap()
        });
        assert_eq!(printed, expected);
    }

    #[test]
    fn else_if_prints_with_204_kib() {
        let output = Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "frontend::print::printer_stack_regression::else_if_child_prints_with_204_kib",
                "--nocapture",
            ])
            .env("J16_PRINT_ELSE_IF_CHILD", "1")
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{} {}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }

    #[test]
    fn nested_pattern_child_prints_with_204_kib() {
        if std::env::var_os("J16_PRINT_PATTERN_CHILD").is_none() {
            return;
        }
        let source = [
            "fn Main(){match 1{".to_owned(),
            "Some(".repeat(126),
            "_".to_owned(),
            ")".repeat(126),
            "=>{},_=>{}}}".to_owned(),
        ]
        .concat();
        let ast = parse_new_profile("pattern-print.n", &source).unwrap();
        let expected = print_inner(&ast, None).output;
        let printed = std::thread::scope(|scope| {
            std::thread::Builder::new()
                .stack_size(204 * 1024)
                .spawn_scoped(scope, || print_inner(&ast, None).output)
                .unwrap()
                .join()
                .unwrap()
        });
        assert_eq!(printed, expected);
    }

    #[test]
    fn nested_pattern_prints_with_204_kib() {
        let output = Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "frontend::print::printer_stack_regression::nested_pattern_child_prints_with_204_kib",
                "--nocapture",
            ])
            .env("J16_PRINT_PATTERN_CHILD", "1")
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
