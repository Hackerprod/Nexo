//! Normalized AST equality for printer tests.
use crate::diagnostic::Span;
use crate::frontend::parse::{Ast, Block, Else, Expr, Statement, WithClause};

fn erase_expr_spans(expr: &mut Expr) {
    let blank = |span: &mut Span| {
        span.file.clear();
        span.start = 0;
        span.end = 0;
    };
    match expr {
        Expr::Name(_, span) | Expr::Integer(_, span) | Expr::Bool(_, span) | Expr::Unit(span) => {
            blank(span)
        }
        Expr::Array(items, span) => {
            blank(span);
            for item in items {
                erase_expr_spans(item);
            }
        }
        Expr::Group(value, _) => {
            erase_expr_spans(value);
            // F0-D15 §4.2: N discards only syntactic grouping, after the
            // parser has retained the enclosing expression's context.
            *expr = (**value).clone();
        }
        Expr::Unary { value, span, .. } => {
            blank(span);
            erase_expr_spans(value);
        }
        Expr::Binary {
            left, right, span, ..
        } => {
            blank(span);
            erase_expr_spans(left);
            erase_expr_spans(right);
        }
        Expr::Call { target, args, span } => {
            blank(span);
            erase_expr_spans(target);
            for arg in args {
                blank(&mut arg.span);
                erase_expr_spans(&mut arg.value);
            }
        }
        Expr::Index {
            target,
            index,
            span,
        } => {
            blank(span);
            erase_expr_spans(target);
            erase_expr_spans(index);
        }
        Expr::If {
            condition,
            then_block,
            otherwise,
            span,
        } => {
            blank(span);
            erase_expr_spans(condition);
            erase_block_spans(then_block);
            if let Some(value) = otherwise {
                match &mut **value {
                    Else::Block(block) => erase_block_spans(block),
                    Else::If(expr) => erase_expr_spans(expr),
                }
            }
        }
    }
}

fn erase_block_spans(block: &mut Block) {
    block.span.file.clear();
    block.span.start = 0;
    block.span.end = 0;
    for statement in &mut block.statements {
        erase_statement_spans(statement);
    }
}

fn erase_statement_spans(statement: &mut Statement) {
    let blank = |span: &mut Span| {
        span.file.clear();
        span.start = 0;
        span.end = 0;
    };
    match statement {
        Statement::Let { value, span, .. }
        | Statement::Assign { value, span, .. }
        | Statement::Expr { value, span } => {
            blank(span);
            erase_expr_spans(value);
            if let Statement::Assign { place, .. } = statement {
                erase_expr_spans(place);
            }
        }
        Statement::Return { value, span } | Statement::Value { value, span } => {
            blank(span);
            if let Some(value) = value {
                erase_expr_spans(value);
            }
        }
        Statement::If {
            condition,
            then_block,
            otherwise,
            span,
        } => {
            blank(span);
            erase_expr_spans(condition);
            erase_block_spans(then_block);
            if let Some(otherwise) = otherwise {
                erase_statement_spans(otherwise);
            }
        }
        Statement::While {
            condition,
            body,
            span,
        } => {
            blank(span);
            erase_expr_spans(condition);
            erase_block_spans(body);
        }
        Statement::Block(block) => erase_block_spans(block),
    }
}

pub(crate) fn ast_without_spans(mut ast: Ast) -> Ast {
    ast.span.file.clear();
    ast.span.start = 0;
    ast.span.end = 0;
    for function in &mut ast.functions {
        function.span.file.clear();
        function.span.start = 0;
        function.span.end = 0;
        function.name_span.file.clear();
        function.name_span.start = 0;
        function.name_span.end = 0;
        for parameter in &mut function.params {
            parameter.span.file.clear();
            parameter.span.start = 0;
            parameter.span.end = 0;
        }
        if let Some(WithClause::Operations(operations)) = &mut function.with_clause {
            for operation in operations {
                operation.span.file.clear();
                operation.span.start = 0;
                operation.span.end = 0;
            }
        }
        erase_block_spans(&mut function.body);
    }
    ast
}
