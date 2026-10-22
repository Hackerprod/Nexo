use crate::{
    Diagnostic,
    syntax::{Expr, ExprKind, Stmt, StmtKind},
};
#[derive(Clone, Debug, PartialEq, Eq)]
enum Ty {
    Int,
    Bool,
    Array(Box<Ty>),
    Resource,
}
#[derive(Clone)]
struct Binding {
    name: String,
    ty: Ty,
    mutable: bool,
    live: bool,
}
struct Checker {
    scopes: Vec<Vec<Binding>>,
}
impl Checker {
    fn find(&self, name: &str, pos: usize) -> Result<(usize, usize), Diagnostic> {
        for (s, scope) in self.scopes.iter().enumerate().rev() {
            if let Some(i) = scope.iter().position(|b| b.name == name) {
                return Ok((s, i));
            }
        }
        Err(Diagnostic::new(
            "E0101",
            pos,
            format!("unknown binding '{name}'"),
        ))
    }
    fn get(&self, name: &str, pos: usize) -> Result<&Binding, Diagnostic> {
        let (s, i) = self.find(name, pos)?;
        let b = &self.scopes[s][i];
        if !b.live {
            return Err(Diagnostic::new(
                "E0202",
                pos,
                format!("'{name}' was transferred or closed; use its current owner"),
            ));
        }
        Ok(b)
    }
    fn consume(&mut self, name: &str, pos: usize) -> Result<(), Diagnostic> {
        if self.get(name, pos)?.ty != Ty::Resource {
            return Err(Diagnostic::new(
                "E0102",
                pos,
                "transfer/drop requires a resource in this slice",
            ));
        }
        let (s, i) = self.find(name, pos)?;
        self.scopes[s][i].live = false;
        Ok(())
    }
    fn expr(&mut self, e: &Expr, resource_binding: bool) -> Result<Ty, Diagnostic> {
        let pos = e.pos;
        match &e.kind {
            ExprKind::Int(_) => Ok(Ty::Int),
            ExprKind::Bool(_) => Ok(Ty::Bool),
            ExprKind::Var(name) => {
                let ty = self.get(name, pos)?.ty.clone();
                if ty == Ty::Resource {
                    return Err(Diagnostic::new(
                        "E0201",
                        pos,
                        format!(
                            "implicit resource copy from '{name}' is forbidden; transfer with move or inspect by read-only primitive"
                        ),
                    ));
                }
                Ok(ty)
            }
            ExprKind::Move(name) => {
                if !resource_binding {
                    return Err(Diagnostic::new(
                        "E0203",
                        pos,
                        "move is supported only as the entire resource binding initializer",
                    ));
                }
                self.consume(name, pos)?;
                Ok(Ty::Resource)
            }
            ExprKind::Open(label) => {
                if !resource_binding {
                    return Err(Diagnostic::new(
                        "E0203",
                        pos,
                        "mock_open is supported only as the entire resource binding initializer; nested resources are not yet modeled",
                    ));
                }
                self.expect(label, Ty::Int)?;
                Ok(Ty::Resource)
            }
            ExprKind::Array(values) => {
                let Some(first) = values.first() else {
                    return Err(Diagnostic::new(
                        "E0103",
                        pos,
                        "empty arrays need element annotations, unsupported in slice 1",
                    ));
                };
                let ty = self.expr(first, false)?;
                for value in &values[1..] {
                    self.expect(value, ty.clone())?;
                }
                let mut depth = 1;
                let mut inner = &ty;
                while let Ty::Array(child) = inner {
                    depth += 1;
                    inner = child;
                }
                if depth > 128 {
                    return Err(Diagnostic::new(
                        "E0003",
                        pos,
                        "inferred array type depth exceeds F0 limit 128",
                    ));
                }
                Ok(Ty::Array(Box::new(ty)))
            }
            ExprKind::Index(array, index) => {
                let ty = self.expr(array, false)?;
                self.expect(index, Ty::Int)?;
                match ty {
                    Ty::Array(t) => Ok(*t),
                    _ => Err(Diagnostic::new("E0102", pos, "indexing requires an array")),
                }
            }
            ExprKind::Add(a, b) => {
                self.expect(a, Ty::Int)?;
                self.expect(b, Ty::Int)?;
                Ok(Ty::Int)
            }
        }
    }
    fn expect(&mut self, e: &Expr, expected: Ty) -> Result<(), Diagnostic> {
        let actual = self.expr(e, false)?;
        if actual != expected {
            return Err(Diagnostic::new(
                "E0102",
                e.pos,
                format!("expected {expected:?}, found {actual:?}"),
            ));
        }
        Ok(())
    }
    fn block(&mut self, body: &[Stmt]) -> Result<(), Diagnostic> {
        self.scopes.push(Vec::new());
        // Intentionally check even unreachable statements. No effects are executed.
        for stmt in body {
            let pos = stmt.pos;
            match &stmt.kind {
                StmtKind::Bind {
                    name,
                    mutable,
                    expr,
                } => {
                    if self.scopes.last().unwrap().iter().any(|b| b.name == *name) {
                        return Err(Diagnostic::new(
                            "E0104",
                            pos,
                            "duplicate binding in the same scope",
                        ));
                    }
                    let ty = self.expr(expr, true)?;
                    self.scopes.last_mut().unwrap().push(Binding {
                        name: name.clone(),
                        ty,
                        mutable: *mutable,
                        live: true,
                    });
                }
                StmtKind::Assign {
                    name,
                    indices,
                    expr,
                } => {
                    let b = self.get(name, pos)?;
                    if !b.mutable {
                        return Err(Diagnostic::new(
                            "E0105",
                            pos,
                            format!("'{name}' is immutable; declare var to permit mutation"),
                        ));
                    }
                    let mut ty = b.ty.clone();
                    if ty == Ty::Resource {
                        return Err(Diagnostic::new(
                            "E0203",
                            pos,
                            "resource rebinding is not supported; use a fresh owner",
                        ));
                    }
                    for index in indices {
                        self.expect(index, Ty::Int)?;
                        ty = match ty {
                            Ty::Array(inner) => *inner,
                            _ => {
                                return Err(Diagnostic::new(
                                    "E0102",
                                    pos,
                                    "too many indices for target type",
                                ));
                            }
                        };
                    }
                    self.expect(expr, ty)?;
                }
                StmtKind::Assert(a, b) => {
                    let ty = self.expr(a, false)?;
                    self.expect(b, ty)?;
                }
                StmtKind::Print(e) => {
                    self.expr(e, false)?;
                }
                StmtKind::Drop(name) => self.consume(name, pos)?,
                StmtKind::Inspect(name) => {
                    if self.get(name, pos)?.ty != Ty::Resource {
                        return Err(Diagnostic::new("E0102", pos, "inspect requires a resource"));
                    }
                }
                StmtKind::Block(body) => self.block(body)?,
                StmtKind::Return | StmtKind::Fail => {}
            }
        }
        self.scopes.pop();
        Ok(())
    }
}
pub fn check(body: &[Stmt]) -> Result<(), Diagnostic> {
    Checker { scopes: Vec::new() }.block(body)
}
pub mod names;
#[cfg(test)]
mod tests;
pub mod types;

/// Proposed F0-0002 checker, intentionally separate from slice-1 `check`.
pub fn check_proposed(
    file: &str,
    source: &str,
    run_entry: bool,
) -> Result<(), Vec<Box<crate::diagnostic::Diagnostic>>> {
    let ast = crate::frontend::parse::parse(file, source).map_err(|d| vec![d])?;
    std::thread::scope(|scope| {
        let worker = std::thread::Builder::new()
            .stack_size(16 * 1024 * 1024)
            .spawn_scoped(scope, move || {
                #[cfg(test)]
                let _allocation_capture = crate::test_alloc::capture_for(file);
                let resolved = names::resolve(file, source, &ast)?;
                types::check(file, source, &ast, &resolved)?;
                if run_entry {
                    names::entry(file, source, &resolved, &ast).map_err(|d| vec![d])?;
                }
                Ok(())
            });
        match worker {
            Ok(worker) => match worker.join() {
                Ok(result) => result,
                Err(payload) => std::panic::resume_unwind(payload),
            },
            Err(_) => {
                use crate::diagnostic::{DataValue, Operation, Span, Stage};
                let mut d = names::diagnostic(
                    "NX-E0003",
                    None,
                    Stage::Types,
                    Operation::AnalysisLimit,
                    Span {
                        file: file.into(),
                        start: 0,
                        end: 1,
                    },
                );
                d.data.insert(
                    "budget_kind".into(),
                    DataValue::String("type_solver_steps".into()),
                );
                d.data.insert("limit".into(), DataValue::UInt(1_000_000));
                d.data.insert("observed".into(), DataValue::UInt(1_000_001));
                Err(vec![d])
            }
        }
    })
}
