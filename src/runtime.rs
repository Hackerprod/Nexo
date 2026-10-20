use crate::{
    CheckedProgram, Diagnostic,
    syntax::{Expr, ExprKind, Stmt, StmtKind},
};
#[derive(Clone, Debug, PartialEq, Eq)]
enum Data {
    Int(i64),
    Bool(bool),
    Array(Vec<Data>),
}
impl Data {
    fn display(&self) -> String {
        match self {
            Self::Int(n) => n.to_string(),
            Self::Bool(b) => b.to_string(),
            Self::Array(a) => format!(
                "[{}]",
                a.iter().map(Self::display).collect::<Vec<_>>().join(", ")
            ),
        }
    }
}
// Resource has deliberately no Clone implementation. Only Data is copyable.
struct Resource {
    id: usize,
    label: i64,
}
enum Value {
    Data(Data),
    Resource(Resource),
}
struct Slot {
    name: String,
    value: Option<Value>,
}
#[derive(Debug, PartialEq, Eq)]
pub struct Execution {
    pub output: Vec<String>,
    pub events: Vec<String>,
    pub error: Option<Diagnostic>,
}
struct Machine {
    scopes: Vec<Vec<Slot>>,
    output: Vec<String>,
    events: Vec<String>,
    next_id: usize,
    budget: usize,
}
enum Flow {
    Next,
    Return,
}
impl Machine {
    fn slot(&self, name: &str) -> (usize, usize) {
        for (s, scope) in self.scopes.iter().enumerate().rev() {
            if let Some(i) = scope.iter().position(|b| b.name == name) {
                return (s, i);
            }
        }
        unreachable!("checker must resolve every name")
    }
    fn charge(&mut self, amount: usize, pos: usize) -> Result<(), Diagnostic> {
        self.budget = self
            .budget
            .checked_sub(amount)
            .ok_or_else(|| Diagnostic::new("R0001", pos, "F0 interpreter work budget exhausted"))?;
        Ok(())
    }
    fn data(&mut self, name: &str, pos: usize) -> Result<Data, Diagnostic> {
        let (s, i) = self.slot(name);
        match self.scopes[s][i]
            .value
            .as_ref()
            .expect("checked live binding")
        {
            Value::Data(d) => {
                fn cells(d: &Data) -> usize {
                    match d {
                        Data::Array(a) => 1 + a.iter().map(cells).sum::<usize>(),
                        _ => 1,
                    }
                }
                let size = cells(d);
                self.charge(size, pos)?;
                let Value::Data(d) = self.scopes[s][i].value.as_ref().unwrap() else {
                    unreachable!()
                };
                Ok(d.clone())
            }
            Value::Resource(_) => unreachable!("checked no implicit resource copy"),
        }
    }
    fn expr(&mut self, e: &Expr) -> Result<Data, Diagnostic> {
        let pos = e.pos;
        self.charge(1, pos)?;
        match &e.kind {
            ExprKind::Int(n) => Ok(Data::Int(*n)),
            ExprKind::Bool(b) => Ok(Data::Bool(*b)),
            ExprKind::Var(n) => self.data(n, pos),
            ExprKind::Array(a) => Ok(Data::Array(
                a.iter().map(|e| self.expr(e)).collect::<Result<_, _>>()?,
            )),
            ExprKind::Add(a, b) => {
                let Data::Int(a) = self.expr(a)? else {
                    unreachable!()
                };
                let Data::Int(b) = self.expr(b)? else {
                    unreachable!()
                };
                a.checked_add(b).map(Data::Int).ok_or_else(|| {
                    Diagnostic::new(
                        "R0101",
                        pos,
                        "checked i64 addition overflow; no wrapping performed",
                    )
                })
            }
            ExprKind::Index(a, i) => {
                let Data::Array(a) = self.expr(a)? else {
                    unreachable!()
                };
                let Data::Int(i) = self.expr(i)? else {
                    unreachable!()
                };
                let i = usize::try_from(i).map_err(|_| {
                    Diagnostic::new("R0102", pos, "array index is negative or too large")
                })?;
                a.into_iter()
                    .nth(i)
                    .ok_or_else(|| Diagnostic::new("R0102", pos, "array index out of bounds"))
            }
            ExprKind::Move(_) | ExprKind::Open(_) => {
                unreachable!("resource expressions restricted to initializers")
            }
        }
    }
    fn value(&mut self, e: &Expr) -> Result<Value, Diagnostic> {
        match &e.kind {
            ExprKind::Move(name) => {
                let (s, i) = self.slot(name);
                Ok(self.scopes[s][i].value.take().expect("checked live move"))
            }
            ExprKind::Open(label) => {
                let Data::Int(label) = self.expr(label)? else {
                    unreachable!()
                };
                let id = self.next_id;
                self.next_id += 1;
                self.events.push(format!("open:{id}:{label}"));
                Ok(Value::Resource(Resource { id, label }))
            }
            _ => Ok(Value::Data(self.expr(e)?)),
        }
    }
    fn close(&mut self, v: Value) {
        if let Value::Resource(r) = v {
            self.events.push(format!("close:{}:{}", r.id, r.label));
        }
    }
    fn cleanup(&mut self) {
        let scope = self.scopes.pop().expect("scope");
        for slot in scope.into_iter().rev() {
            if let Some(v) = slot.value {
                self.close(v)
            }
        }
    }
    fn block(&mut self, body: &[Stmt]) -> Result<Flow, Diagnostic> {
        self.scopes.push(Vec::new());
        let result = (|| {
            for stmt in body {
                match self.stmt(stmt)? {
                    Flow::Next => {}
                    Flow::Return => return Ok(Flow::Return),
                }
            }
            Ok(Flow::Next)
        })();
        self.cleanup();
        result
    }
    fn stmt(&mut self, s: &Stmt) -> Result<Flow, Diagnostic> {
        let pos = s.pos;
        self.charge(1, pos)?;
        match &s.kind {
            StmtKind::Bind { name, expr, .. } => {
                let value = self.value(expr)?;
                self.scopes.last_mut().unwrap().push(Slot {
                    name: name.clone(),
                    value: Some(value),
                });
            }
            StmtKind::Assign {
                name,
                indices,
                expr,
            } => {
                // Defined order: evaluate target indices left-to-right, then RHS,
                // then validate bounds and perform the write. No side effects in data expressions.
                let indices = indices
                    .iter()
                    .map(|i| {
                        let Data::Int(n) = self.expr(i)? else {
                            unreachable!()
                        };
                        Ok::<_, Diagnostic>((n, i.pos))
                    })
                    .collect::<Result<Vec<_>, _>>()?;
                let value = self.expr(expr)?;
                let (scope, slot) = self.slot(name);
                let Some(Value::Data(mut_target)) = &mut self.scopes[scope][slot].value else {
                    unreachable!()
                };
                let mut target = mut_target;
                for (index, index_pos) in indices {
                    let index = usize::try_from(index).map_err(|_| {
                        Diagnostic::new("R0102", index_pos, "array index is negative or too large")
                    })?;
                    let Data::Array(values) = target else {
                        unreachable!()
                    };
                    target = values.get_mut(index).ok_or_else(|| {
                        Diagnostic::new("R0102", pos, "array assignment index out of bounds")
                    })?;
                }
                *target = value;
            }
            StmtKind::Assert(a, b) => {
                let a = self.expr(a)?;
                let b = self.expr(b)?;
                if a != b {
                    return Err(Diagnostic::new(
                        "R0201",
                        pos,
                        format!("assertion failed: {} != {}", a.display(), b.display()),
                    ));
                }
            }
            StmtKind::Print(e) => {
                let value = self.expr(e)?;
                self.output.push(value.display());
            }
            StmtKind::Drop(name) => {
                let (s, i) = self.slot(name);
                let v = self.scopes[s][i].value.take().expect("checked live drop");
                self.close(v);
            }
            StmtKind::Inspect(name) => {
                let (s, i) = self.slot(name);
                let Some(Value::Resource(r)) = &self.scopes[s][i].value else {
                    unreachable!()
                };
                self.events.push(format!("inspect:{}:{}", r.id, r.label));
            }
            StmtKind::Block(b) => return self.block(b),
            StmtKind::Return => return Ok(Flow::Return),
            StmtKind::Fail => {
                return Err(Diagnostic::new(
                    "R0301",
                    pos,
                    "simulated recoverable failure",
                ));
            }
        }
        Ok(Flow::Next)
    }
}
pub fn execute(program: &CheckedProgram) -> Execution {
    let mut m = Machine {
        scopes: Vec::new(),
        output: Vec::new(),
        events: Vec::new(),
        next_id: 0,
        budget: 1_000_000,
    };
    let error = m.block(&program.body).err();
    Execution {
        output: m.output,
        events: m.events,
        error,
    }
}
