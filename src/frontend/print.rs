//! Canonical source printer for accepted normalized syntax trees.
use super::parse::*;

pub fn print_canonical(ast: &Ast) -> String {
    let mut out = String::new();
    for (i, f) in ast.functions.iter().enumerate() {
        if i > 0 {
            out.push('\n')
        }
        out.push_str("fn ");
        out.push_str(&f.name);
        out.push('(');
        for (j, p) in f.params.iter().enumerate() {
            if j > 0 {
                out.push_str(", ")
            }
            match p.mode {
                Mode::Read => {}
                Mode::Inout => out.push_str("inout "),
                Mode::Consume => out.push_str("consume "),
            }
            out.push_str(&p.name);
            out.push_str(": ");
            ty(&p.ty, &mut out)
        }
        out.push(')');
        if let Some(t) = &f.result {
            out.push_str(" -> ");
            ty(t, &mut out)
        }
        if let Some(clause) = &f.with_clause {
            out.push_str(" with ");
            match clause {
                WithClause::Simple(items) => out.push_str(&items.join(", ")),
                WithClause::Operations(operations) => {
                    out.push('{');
                    for (index, operation) in operations.iter().enumerate() {
                        if index > 0 {
                            out.push_str(", ");
                        }
                        out.push_str(&operation.name);
                        out.push('(');
                        out.push_str(&operation.restrictions.join(", "));
                        out.push(')');
                    }
                    out.push('}');
                }
            }
        }
        out.push(' ');
        block(&f.body, &mut out, 0)
    }
    out
}
fn ty(t: &Type, o: &mut String) {
    match t {
        Type::I64 => o.push_str("i64"),
        Type::Bool => o.push_str("bool"),
        Type::Unit => o.push_str("()"),
        Type::MockResource => o.push_str("MockResource"),
        Type::MockBlockingResource => o.push_str("MockBlockingResource"),
        Type::Array(x) => {
            o.push_str("Array<");
            ty(x, o);
            o.push('>')
        }
        Type::Result(a, b) => {
            o.push_str("Result<");
            ty(a, o);
            o.push_str(", ");
            ty(b, o);
            o.push('>')
        }
    }
}
fn block(b: &Block, o: &mut String, d: usize) {
    o.push('{');
    for s in &b.statements {
        o.push('\n');
        o.push_str(&"    ".repeat(d + 1));
        stmt(s, o, d + 1)
    }
    if !b.statements.is_empty() {
        o.push('\n');
        o.push_str(&"    ".repeat(d))
    }
    o.push('}')
}
fn stmt(s: &Statement, o: &mut String, d: usize) {
    match s {
        Statement::Let {
            mutable,
            name,
            annotation,
            value,
            ..
        } => {
            o.push_str(if *mutable { "var " } else { "let " });
            o.push_str(name);
            if let Some(t) = annotation {
                o.push_str(": ");
                ty(t, o)
            }
            o.push_str(" = ");
            expr(value, o, 0);
            o.push(';')
        }
        Statement::Assign { place, value, .. } => {
            expr(place, o, 0);
            o.push_str(" = ");
            expr(value, o, 0);
            o.push(';')
        }
        Statement::Return { value, .. } => {
            o.push_str("return");
            if let Some(x) = value {
                o.push(' ');
                expr(x, o, 0)
            }
            o.push(';')
        }
        Statement::Value { value, .. } => {
            o.push_str("value:");
            if let Some(x) = value {
                o.push(' ');
                expr(x, o, 0)
            }
            o.push(';')
        }
        Statement::Expr { value, .. } => {
            expr(value, o, 0);
            o.push(';')
        }
        Statement::Block(b) => block(b, o, d),
        Statement::While {
            condition, body, ..
        } => {
            o.push_str("while ");
            expr(condition, o, 0);
            o.push(' ');
            block(body, o, d)
        }
        Statement::If {
            condition,
            then_block,
            otherwise,
            ..
        } => {
            o.push_str("if ");
            expr(condition, o, 0);
            o.push(' ');
            block(then_block, o, d);
            if let Some(x) = otherwise {
                o.push_str(" else ");
                match &**x {
                    Statement::Block(b) => block(b, o, d),
                    Statement::Expr { value, .. } => expr(value, o, 0),
                    _ => stmt(x, o, d),
                }
            }
        }
    }
}
fn expr(e: &Expr, o: &mut String, parent: u8) {
    match e {
        Expr::Name(n, _) => o.push_str(n),
        Expr::Integer(n, _) => o.push_str(n),
        Expr::Bool(v, _) => o.push_str(if *v { "true" } else { "false" }),
        Expr::Unit(_) => o.push_str("()"),
        Expr::Array(xs, _) => {
            o.push('[');
            for (i, x) in xs.iter().enumerate() {
                if i > 0 {
                    o.push_str(", ")
                }
                expr(x, o, 0)
            }
            o.push(']')
        }
        Expr::Group(x, _) => {
            o.push('(');
            expr(x, o, 0);
            o.push(')')
        }
        Expr::Unary { mode, value, .. } => match mode {
            Mode::Read => {
                expr(value, o, 3);
                o.push('?');
            }
            Mode::Consume => {
                o.push_str("consume ");
                expr(value, o, 3);
            }
            Mode::Inout => {
                o.push_str("inout ");
                expr(value, o, 3);
            }
        },
        Expr::Binary {
            op, left, right, ..
        } => {
            let p = if op == "+" || op == "-" { 2 } else { 1 };
            let wrap = p < parent;
            if wrap {
                o.push('(')
            }
            expr(left, o, p);
            o.push(' ');
            o.push_str(op);
            o.push(' ');
            expr(right, o, p + 1);
            if wrap {
                o.push(')')
            }
        }
        Expr::Call { target, args, .. } => {
            expr(target, o, 3);
            o.push('(');
            for (i, a) in args.iter().enumerate() {
                if i > 0 {
                    o.push_str(", ")
                }
                if let Some(l) = &a.label {
                    o.push_str(l);
                    o.push_str(": ")
                }
                expr(&a.value, o, 0)
            }
            o.push(')')
        }
        Expr::Index { target, index, .. } => {
            expr(target, o, 3);
            o.push('[');
            expr(index, o, 0);
            o.push(']')
        }
        Expr::If {
            condition,
            then_block,
            otherwise,
            ..
        } => {
            o.push_str("if ");
            expr(condition, o, 0);
            o.push(' ');
            block(then_block, o, 0);
            if let Some(x) = otherwise {
                o.push_str(" else ");
                print_else(x, o, 0)
            }
        }
    }
}

fn print_else(value: &Else, out: &mut String, depth: usize) {
    match value {
        Else::Block(block_value) => block(block_value, out, depth),
        Else::If(expr_value) => expr(expr_value, out, 0),
    }
}
