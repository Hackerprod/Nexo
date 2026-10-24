//! Temporary capability boundaries for the official B pipeline.
use crate::check::names::{self, Resolved, ResolvedUse};
use crate::diagnostic::{DataValue, Diagnostic, Operation, Span, Stage};
use crate::frontend::{
    lex,
    parse::{Ast, Block, Else, Expr, Mode, Statement, Type},
};

#[derive(Clone)]
struct Candidate {
    span: Span,
    scope: &'static str,
}
impl Candidate {
    fn order(&self) -> (u64, u8) {
        let rank = match self.scope {
            "I08" => 0,
            "I09" => 1,
            "I10a" => 2,
            "I10b" => 3,
            "I11a" => 4,
            _ => unreachable!(),
        };
        (self.span.start, rank)
    }
}

fn has_result(ty: &Type) -> bool {
    match ty {
        Type::Result(..) => true,
        Type::Array(inner) => has_result(inner),
        _ => false,
    }
}

pub(super) fn first_pending(
    file: &str,
    source: &str,
    ast: &Ast,
    resolved: &Resolved,
) -> Option<Box<Diagnostic>> {
    let tokens = match lex::lex(file, source) {
        Ok(tokens) => tokens,
        Err(error) => return Some(error),
    };
    let mut found = Vec::new();
    for function in &ast.functions {
        if function.with_clause.is_some()
            && let Some(with) = tokens.iter().find(|t| {
                t.text == "with"
                    && t.span.start >= function.name_span.end
                    && t.span.end <= function.body.span.start
            })
        {
            let end = tokens
                .iter()
                .take_while(|t| t.span.start < function.body.span.start)
                .last()
                .map_or(with.span.end, |t| t.span.end);
            found.push(Candidate {
                span: Span {
                    file: file.into(),
                    start: with.span.start,
                    end,
                },
                scope: "I09",
            });
        }
        for param in &function.params {
            if has_result(&param.ty) {
                collect_result_annotation(
                    &tokens,
                    param.span.start,
                    param.span.end,
                    ":",
                    &mut found,
                );
            }
        }
        if function.result.as_ref().is_some_and(has_result) {
            collect_result_annotation(
                &tokens,
                function.name_span.end,
                function.body.span.start,
                "->",
                &mut found,
            );
        }
        collect_block(&function.body, &tokens, resolved, &mut found);
    }
    let candidate = found.into_iter().min_by_key(Candidate::order)?;
    let mut d = names::diagnostic(
        "NX-E0006",
        None,
        Stage::Types,
        Operation::TypesCheck,
        candidate.span,
    );
    d.feature_id = Some("implementation_limit".into());
    d.message = format!(
        "Capacidad pendiente de implementación en {}.",
        candidate.scope
    );
    d.data.insert(
        "pending_scope".into(),
        DataValue::String(candidate.scope.into()),
    );
    Some(d)
}

fn collect_result_annotation(
    tokens: &[lex::Token],
    start: u64,
    end: u64,
    marker: &str,
    found: &mut Vec<Candidate>,
) {
    let Some(marker_at) = tokens
        .iter()
        .position(|t| t.text == marker && t.span.start >= start && t.span.end <= end)
    else {
        return;
    };
    let Some(result_at) = tokens
        .iter()
        .enumerate()
        .skip(marker_at + 1)
        .find_map(|(i, t)| (t.text == "Result" && t.span.end <= end).then_some(i))
    else {
        return;
    };
    let token = &tokens[result_at];
    let mut level = 0_usize;
    let mut type_end = token.span.end;
    for next in tokens
        .iter()
        .skip(result_at + 1)
        .take_while(|t| t.span.start < end)
    {
        if next.text == "<" {
            level += 1;
        } else if next.text == ">" || next.text == ">=" {
            level = level.saturating_sub(1);
            if level == 0 {
                type_end = next.span.start + 1;
                break;
            }
        }
    }
    found.push(Candidate {
        span: Span {
            file: token.span.file.clone(),
            start: token.span.start,
            end: type_end,
        },
        scope: "I08",
    });
}
fn collect_block(
    block: &Block,
    tokens: &[lex::Token],
    resolved: &Resolved,
    found: &mut Vec<Candidate>,
) {
    for statement in &block.statements {
        collect_statement(statement, tokens, resolved, found);
    }
}
fn collect_statement(
    statement: &Statement,
    tokens: &[lex::Token],
    resolved: &Resolved,
    found: &mut Vec<Candidate>,
) {
    match statement {
        Statement::Let {
            annotation,
            value,
            span,
            ..
        } => {
            if annotation.as_ref().is_some_and(has_result) {
                collect_result_annotation(tokens, span.start, value.span().start, ":", found);
            }
            collect_expr(value, tokens, resolved, found);
        }
        Statement::Return {
            value: Some(value), ..
        }
        | Statement::Value {
            value: Some(value), ..
        }
        | Statement::Expr { value, .. } => {
            collect_expr(value, tokens, resolved, found);
        }
        Statement::Assign { place, value, .. } => {
            collect_expr(place, tokens, resolved, found);
            collect_expr(value, tokens, resolved, found);
        }
        Statement::If {
            condition,
            then_block,
            otherwise,
            ..
        } => {
            collect_expr(condition, tokens, resolved, found);
            collect_block(then_block, tokens, resolved, found);
            if let Some(other) = otherwise {
                collect_statement(other, tokens, resolved, found);
            }
        }
        Statement::While {
            condition, body, ..
        } => {
            collect_expr(condition, tokens, resolved, found);
            collect_block(body, tokens, resolved, found);
        }
        Statement::Block(block) => collect_block(block, tokens, resolved, found),
        Statement::Return { value: None, .. } | Statement::Value { value: None, .. } => {}
    }
}
fn collect_expr(
    expr: &Expr,
    tokens: &[lex::Token],
    resolved: &Resolved,
    found: &mut Vec<Candidate>,
) {
    match expr {
        Expr::Array(items, _) => {
            for item in items {
                collect_expr(item, tokens, resolved, found);
            }
        }
        Expr::Group(inner, _) => collect_expr(inner, tokens, resolved, found),
        Expr::Unary { mode, value, span } => {
            if *mode == Mode::Read {
                found.push(Candidate {
                    span: Span {
                        file: span.file.clone(),
                        start: span.end - 1,
                        end: span.end,
                    },
                    scope: "I08",
                });
            }
            collect_expr(value, tokens, resolved, found);
        }
        Expr::Binary { left, right, .. } => {
            collect_expr(left, tokens, resolved, found);
            collect_expr(right, tokens, resolved, found);
        }
        Expr::Call { target, args, .. } => {
            let mut unwrapped = target.as_ref();
            while let Expr::Group(inner, _) = unwrapped {
                unwrapped = inner;
            }
            if let Expr::Name(name, span) = unwrapped {
                let scope = match name.as_str() {
                    "Ok" | "Err" | "TryAdd" => Some("I08"),
                    "MockUnknown" => Some("I09"),
                    "MockReadRecords" | "MockWriteI64" | "MockRequest" | "MockReply" => {
                        Some("I11a")
                    }
                    _ => None,
                };
                if let Some(scope) = scope
                    && matches!(
                        resolved.uses.get(&span.start),
                        Some(ResolvedUse::Intrinsic(_))
                    )
                {
                    found.push(Candidate {
                        span: span.clone(),
                        scope,
                    });
                }
            }
            collect_expr(target, tokens, resolved, found);
            for arg in args {
                collect_expr(&arg.value, tokens, resolved, found);
            }
        }
        Expr::Index { target, index, .. } => {
            collect_expr(target, tokens, resolved, found);
            collect_expr(index, tokens, resolved, found);
        }
        Expr::If {
            condition,
            then_block,
            otherwise,
            ..
        } => {
            collect_expr(condition, tokens, resolved, found);
            collect_block(then_block, tokens, resolved, found);
            if let Some(other) = otherwise {
                match other.as_ref() {
                    Else::Block(block) => collect_block(block, tokens, resolved, found),
                    Else::If(expr) => collect_expr(expr, tokens, resolved, found),
                }
            }
        }
        Expr::Name(..) | Expr::Integer(..) | Expr::Bool(..) | Expr::Unit(..) => {}
    }
}
