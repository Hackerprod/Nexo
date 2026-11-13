//! Normalized AST equality for printer tests.
use crate::diagnostic::Span;
use crate::frontend::parse::{Ast, Block, Else, Expr, Match, Pattern, Statement, WithClause};

fn erase_match_spans(selection: &mut Match) {
    erase_expr_spans(&mut selection.scrutinee);
    for arm in &mut selection.arms {
        erase_pattern_spans(&mut arm.pattern);
        if let Some(guard) = &mut arm.guard {
            erase_expr_spans(guard);
        }
        if let Some(span) = &mut arm.guard_span {
            *span = Span {
                file: String::new(),
                start: 0,
                end: 0,
            };
        }
        erase_block_spans(&mut arm.body);
        arm.span = Span {
            file: String::new(),
            start: 0,
            end: 0,
        };
    }
    if let Some(span) = &mut selection.mode_span {
        *span = Span {
            file: String::new(),
            start: 0,
            end: 0,
        };
    }
    selection.span = Span {
        file: String::new(),
        start: 0,
        end: 0,
    };
    selection.keyword_span = Span {
        file: String::new(),
        start: 0,
        end: 0,
    };
}

fn erase_pattern_spans(pattern: &mut Pattern) {
    match pattern {
        Pattern::Wildcard(span)
        | Pattern::Integer(_, span)
        | Pattern::Bool(_, span)
        | Pattern::Name { span, .. } => {
            *span = Span {
                file: String::new(),
                start: 0,
                end: 0,
            }
        }
        Pattern::Constructor { args, span, .. } => {
            for arg in args {
                erase_pattern_spans(arg);
            }
            *span = Span {
                file: String::new(),
                start: 0,
                end: 0,
            };
        }
    }
}

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
            selector,
        } => {
            blank(span);
            blank(selector);
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
        Expr::Match(selection) => erase_match_spans(selection),
    }
}

fn erase_block_spans(block: &mut Block) {
    block.span.file.clear();
    block.span.start = 0;
    block.span.end = 0;
    block.close_span.file.clear();
    block.close_span.start = 0;
    block.close_span.end = 0;
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
        Statement::Return { value, span } => {
            blank(span);
            if let Some(value) = value {
                erase_expr_spans(value);
            }
        }
        Statement::Value {
            value,
            marker,
            span,
        } => {
            blank(span);
            *marker = None;
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
        Statement::Match(selection) => erase_match_spans(selection),
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
        match &mut function.with_clause {
            Some(WithClause::Simple(items)) => {
                for item in items {
                    item.span.file.clear();
                    item.span.start = 0;
                    item.span.end = 0;
                }
            }
            Some(WithClause::Operations(operations)) => {
                for operation in operations {
                    operation.span.file.clear();
                    operation.span.start = 0;
                    operation.span.end = 0;
                    for restriction in &mut operation.restrictions {
                        restriction.span.file.clear();
                        restriction.span.start = 0;
                        restriction.span.end = 0;
                    }
                }
            }
            None => {}
        }
        erase_block_spans(&mut function.body);
    }
    ast
}
