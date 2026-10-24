//! Canonical source printer for accepted normalized syntax trees.
use super::cst::Cst;
use super::emit::{Role, SourceToken, Writer};
use super::parse::*;

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
            .stack_size(16 * 1024 * 1024)
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
    for function in &ast.functions {
        let start = function.span.start as usize;
        at(&mut out, "fn", Role::Function, start, start + 2);
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
                        end = after(&mut out, item, Role::Word, end);
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
                            end = after(&mut out, restriction, Role::Word, end);
                        }
                        end = after(&mut out, ")", Role::Close, end);
                    }
                    after(&mut out, "}", Role::ContractClose, end);
                }
            }
        }
        block(&function.body, &mut out, true);
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
    }
}
fn block(value: &Block, out: &mut Writer, structural: bool) {
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
        for statement in &current.statements {
            stmt(statement, out);
        }
        let end = current.span.end as usize;
        out.token_at(
            "}",
            Role::BlockClose,
            structural && outers.is_empty(),
            end - 1,
            end,
        );
        break;
    }
    let outer_count = outers.len();
    for (i, end) in outers.into_iter().rev().enumerate() {
        out.token_at(
            "}",
            Role::BlockClose,
            structural && i + 1 == outer_count,
            end - 1,
            end,
        );
    }
}
fn stmt(statement: &Statement, out: &mut Writer) {
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
            before(out, "=", Role::Binary, value.span().start as usize);
            expr(value, out, 0);
            out.terminator(span.end as usize);
        }
        Statement::Assign { place, value, span } => {
            expr(place, out, 0);
            after(out, "=", Role::Binary, place.span().end as usize);
            expr(value, out, 0);
            out.terminator(span.end as usize);
        }
        Statement::Return { value, span } => {
            let start = span.start as usize;
            at(out, "return", Role::Word, start, start + 6);
            if let Some(value) = value {
                expr(value, out, 0);
            }
            out.terminator(span.end as usize);
        }
        Statement::Value { value, span } => {
            let start = span.start as usize;
            at(out, "value", Role::Word, start, start + 5);
            after(out, ":", Role::Colon, start + 5);
            if let Some(value) = value {
                expr(value, out, 0);
            }
            out.terminator(span.end as usize);
        }
        Statement::Expr { value, span } => {
            expr(value, out, u8::from(matches!(value, Expr::If { .. })));
            out.terminator(span.end as usize);
        }
        Statement::Block(value) => block(value, out, true),
        Statement::While {
            condition, body, ..
        } => {
            let start = statement_span(statement).start as usize;
            at(out, "while", Role::Word, start, start + 5);
            expr(condition, out, 0);
            block(body, out, true);
        }
        Statement::If {
            condition,
            then_block,
            otherwise,
            ..
        } => {
            let start = statement_span(statement).start as usize;
            at(out, "if", Role::Word, start, start + 2);
            expr(condition, out, 0);
            block(then_block, out, true);
            if let Some(otherwise) = otherwise {
                after(out, "else", Role::Else, then_block.span.end as usize);
                match &**otherwise {
                    Statement::Block(value) => block(value, out, true),
                    Statement::Expr {
                        value: value @ Expr::If { .. },
                        ..
                    } => stmt_if_expr(value, out),
                    Statement::Expr { value, .. } => expr(value, out, 0),
                    other => stmt(other, out),
                }
            }
        }
    }
}
fn stmt_if_expr(value: &Expr, out: &mut Writer) {
    if let Expr::If {
        condition,
        then_block,
        otherwise,
        ..
    } = value
    {
        let start = value.span().start as usize;
        at(out, "if", Role::Word, start, start + 2);
        expr(condition, out, 0);
        block(then_block, out, true);
        if let Some(otherwise) = otherwise {
            after(out, "else", Role::Else, then_block.span.end as usize);
            match &**otherwise {
                Else::Block(value) => block(value, out, true),
                Else::If(next) => stmt_if_expr(next, out),
            }
        }
    }
}
fn expr(value: &Expr, out: &mut Writer, parent: u8) {
    match value {
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
            let mut closes = Vec::new();
            let mut elements = items.as_slice();
            let mut start = span.start as usize;
            closes.push(span.end as usize);
            while let [Expr::Array(inner, _)] = elements {
                at(out, "[", Role::Open, start, start + 1);
                start = elements[0].span().start as usize;
                closes.push(elements[0].span().end as usize);
                elements = inner;
            }
            at(out, "[", Role::Open, start, start + 1);
            for (i, item) in elements.iter().enumerate() {
                if i > 0 {
                    after(out, ",", Role::Comma, elements[i - 1].span().end as usize);
                }
                expr(item, out, 0);
            }
            for end in closes.into_iter().rev() {
                out.token_at("]", Role::Close, false, end - 1, end);
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
            expr(item, out, 0);
            for &(_, end) in groups.iter().rev() {
                at(out, ")", Role::Close, end - 1, end);
            }
            out.group_end();
        }
        Expr::Unary {
            mode: Mode::Read,
            value,
            span,
        } => {
            expr(value, out, 3);
            let end = span.end as usize;
            at(out, "?", Role::Suffix, end - 1, end);
        }
        Expr::Unary {
            mode: prefix,
            value,
            span,
        } => {
            let start = span.start as usize;
            let text = mode(*prefix);
            at(out, text, Role::Word, start, start + text.len());
            expr(value, out, 3);
        }
        Expr::Binary {
            op, left, right, ..
        } => {
            let p = if op == "+" || op == "-" { 2 } else { 1 };
            let wrap = p < parent;
            if wrap {
                inserted(out, "(", Role::Open);
            }
            expr(left, out, p + u8::from(p == 1));
            after(out, op, Role::Binary, left.span().end as usize);
            expr(right, out, p + 1);
            if wrap {
                inserted(out, ")", Role::Close);
            }
        }
        Expr::Call { target, args, span } => {
            expr(target, out, 3);
            after(out, "(", Role::AttachedOpen, target.span().end as usize);
            for (i, argument) in args.iter().enumerate() {
                if i > 0 {
                    after(out, ",", Role::Comma, args[i - 1].span.end as usize);
                }
                if let Some(label) = &argument.label {
                    let end = after(out, label, Role::Word, argument.span.start as usize);
                    after(out, ":", Role::Colon, end);
                }
                expr(&argument.value, out, 0);
            }
            out.token_at(
                ")",
                Role::Close,
                false,
                span.end as usize - 1,
                span.end as usize,
            );
        }
        Expr::Index {
            target,
            index,
            span,
        } => {
            expr(target, out, 3);
            after(out, "[", Role::AttachedOpen, target.span().end as usize);
            expr(index, out, 0);
            out.token_at(
                "]",
                Role::Close,
                false,
                span.end as usize - 1,
                span.end as usize,
            );
        }
        Expr::If {
            condition,
            then_block,
            otherwise,
            span,
        } => {
            if parent > 0 {
                inserted(out, "(", Role::Open);
            }
            let start = span.start as usize;
            at(out, "if", Role::Word, start, start + 2);
            expr(condition, out, 0);
            after(out, "then", Role::Word, condition.span().end as usize);
            branch_operand(then_block, out, false);
            if let Some(otherwise) = otherwise {
                after(out, "else", Role::Else, then_block.span.end as usize);
                print_else(otherwise, out);
            }
            if parent > 0 {
                inserted(out, ")", Role::Close);
            }
        }
    }
}
fn print_else(value: &Else, out: &mut Writer) {
    match value {
        Else::Block(value) => branch_operand(value, out, true),
        Else::If(value) => {
            inserted(out, "(", Role::Open);
            expr(value, out, 0);
            inserted(out, ")", Role::Close);
        }
    }
}
fn branch_operand(branch: &Block, out: &mut Writer, is_else: bool) {
    if let [
        Statement::Value {
            value: Some(value), ..
        },
    ] = branch.statements.as_slice()
    {
        let mut inner = value;
        while let Expr::Group(grouped, _) = inner {
            inner = grouped;
        }
        // A parsed explicit branch has both delimiters outside its value span;
        // a synthesized simple branch has neither. The second comparison is
        // redundant for accepted trees.
        let explicit = branch.span.start < value.span().start;
        if !(is_else && matches!(inner, Expr::If { .. }))
            && (!explicit
                || (!out.has_comment(branch.span.start as usize, branch.span.end as usize)
                    && !out.has_comment_before(branch.span.start as usize)
                    && !out.has_comment_after_in_group(branch.span.end as usize)))
        {
            if matches!(value, Expr::If { .. }) {
                inserted(out, "(", Role::Open);
                expr(value, out, 0);
                inserted(out, ")", Role::Close);
            } else {
                expr(value, out, 0);
            }
            return;
        }
    }
    block(branch, out, false);
}
