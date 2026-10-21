use super::observation::Log;
use crate::diagnostic::{self, DataValue, Diagnostic, Namespace, Operation, Severity, Stage};
use crate::ir::model::{Binary, Function, Intrinsic, Op, Program, TempId, Ty};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Outcome {
    Unit,
    SimulatorFault,
    InternalError(&'static str),
}
#[derive(Clone, Debug)]
pub struct Execution {
    pub outcome: Outcome,
    pub observations: Log,
    pub diagnostics: Vec<Box<Diagnostic>>,
    pub work_consumed: u64,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Value {
    Int(i64),
    Bool(bool),
    Unit,
}
impl Value {
    fn text(self) -> String {
        match self {
            Self::Int(i) => i.to_string(),
            Self::Bool(b) => b.to_string(),
            Self::Unit => "()".into(),
        }
    }
    fn ty(self) -> Ty {
        match self {
            Self::Int(_) => Ty::I64,
            Self::Bool(_) => Ty::Bool,
            Self::Unit => Ty::Unit,
        }
    }
}

pub fn run_scalar(program: &Program) -> Execution {
    let mut state = Machine {
        log: Log::default(),
        work: 0,
        cells: 0,
        fault: None,
    };
    let result = crate::ir::verify::verify_program(program)
        .and_then(|()| program.functions.function("main").ok_or("missing main"));
    let Ok(function) = result else {
        return Execution {
            outcome: Outcome::InternalError(result.err().unwrap_or("invalid program")),
            observations: state.log,
            diagnostics: vec![],
            work_consumed: 0,
        };
    };
    let outcome = match state.run(function) {
        Ok(()) => Outcome::Unit,
        Err(e) => Outcome::InternalError(e),
    };
    let outcome = if state.fault.is_some() {
        Outcome::SimulatorFault
    } else {
        outcome
    };
    Execution {
        outcome,
        observations: state.log,
        diagnostics: state.fault.into_iter().collect(),
        work_consumed: state.work,
    }
}
struct Machine {
    log: Log,
    work: u64,
    cells: u64,
    fault: Option<Box<Diagnostic>>,
}
impl Machine {
    fn fault(&mut self, code: &str, span: &diagnostic::Span, data: Vec<(&str, DataValue)>) {
        if self.fault.is_some() {
            return;
        }
        let operation = match code {
            "R0001" | "R0002" => Operation::RuntimeLimit,
            "R0101" => Operation::ValueAdd,
            "R0103" => Operation::ValueSub,
            "R0201" => Operation::AssertEq,
            _ => Operation::Fail,
        };
        let mut d = Diagnostic {
            schema_version: diagnostic::SCHEMA_VERSION,
            diagnostic_namespace: Namespace::F0002Sim,
            profile_id: diagnostic::PROFILE_ID.into(),
            code: code.into(),
            subcause: None,
            rule: code.into(),
            explanation_id: None,
            severity: Severity::Error,
            stage: Stage::Runtime,
            message: match code {
                "R0103" => "Desbordamiento de resta i64.",
                "R0101" => "Desbordamiento de suma i64.",
                _ => "Fallo del simulador.",
            }
            .into(),
            operation,
            primary_span: Some(span.clone()),
            related_spans: vec![],
            cause_chain: vec![],
            feature_id: None,
            unexpected_token: None,
            expected_tokens: vec![],
            expected_tokens_kind: None,
            parser_context: None,
            recognized_prefix: None,
            repair: vec![],
            data: Default::default(),
        };
        for (k, v) in data {
            d.data.insert(k.into(), v);
        }
        self.fault = Some(Box::new(d));
    }
    fn tick(&mut self, n: u64, span: &diagnostic::Span) -> bool {
        let proposed = self.work.saturating_add(n);
        if proposed > 1_000_000 {
            self.work = 1_000_000;
            self.fault(
                "R0001",
                span,
                vec![
                    ("limit", DataValue::UInt(1_000_000)),
                    ("observed", DataValue::UInt(1_000_001)),
                    ("work_consumed", DataValue::UInt(1_000_000)),
                ],
            );
            false
        } else {
            self.work = proposed;
            true
        }
    }
    fn cell(&mut self, span: &diagnostic::Span) -> bool {
        let proposed = self.cells.saturating_add(1);
        if proposed > 65_536 {
            self.fault(
                "R0002",
                span,
                vec![
                    (
                        "runtime_budget_kind",
                        DataValue::String("live_value_cells".into()),
                    ),
                    ("limit", DataValue::UInt(65_536)),
                    ("observed", DataValue::UInt(proposed)),
                ],
            );
            false
        } else {
            self.cells = proposed;
            true
        }
    }
    fn take(temps: &mut [Option<Value>], id: TempId) -> Result<Value, &'static str> {
        temps
            .get_mut(id.0)
            .ok_or("temporary index")?
            .take()
            .ok_or("temporary already used")
    }
    fn run(&mut self, function: &Function) -> Result<(), &'static str> {
        if !self.tick(1, &function.block.terminator_span()) {
            return Ok(());
        }
        let mut locals = vec![None; function.locals.len()];
        let mut temps: Vec<Option<Value>> = Vec::new();
        for ins in &function.block.instructions {
            if !self.tick(u64::from(ins.tick), &ins.span) {
                break;
            }
            let value = match &ins.op {
                Op::ConstI64(n) => {
                    if !self.tick(1, &ins.span) || !self.cell(&ins.span) {
                        break;
                    }
                    Some(Value::Int(*n))
                }
                Op::ConstBool(b) => {
                    if !self.tick(1, &ins.span) || !self.cell(&ins.span) {
                        break;
                    }
                    Some(Value::Bool(*b))
                }
                Op::ConstUnit => {
                    if !self.tick(1, &ins.span) || !self.cell(&ins.span) {
                        break;
                    }
                    Some(Value::Unit)
                }
                Op::CopyValue(id) => {
                    let v = locals
                        .get(id.0)
                        .ok_or("local index")?
                        .ok_or("empty local")?;
                    if !self.cell(&ins.span) {
                        break;
                    }
                    Some(v)
                }
                Op::Binary { op, left, right } => {
                    let l = Self::take(&mut temps, *left)?;
                    let r = Self::take(&mut temps, *right)?;
                    self.cells = self.cells.saturating_sub(2);
                    let v = match (op, l, r) {
                        (Binary::Add, Value::Int(a), Value::Int(b)) => match a.checked_add(b) {
                            Some(n) => Value::Int(n),
                            None => {
                                self.fault("R0101", &ins.span, vec![]);
                                break;
                            }
                        },
                        (Binary::Sub, Value::Int(a), Value::Int(b)) => match a.checked_sub(b) {
                            Some(n) => Value::Int(n),
                            None => {
                                self.fault("R0103", &ins.span, vec![]);
                                break;
                            }
                        },
                        (Binary::Less, Value::Int(a), Value::Int(b)) => {
                            if !self.tick(2, &ins.span) {
                                break;
                            }
                            Value::Bool(a < b)
                        }
                        (Binary::Equal, a, b) => {
                            if !self.tick(2, &ins.span) {
                                break;
                            }
                            Value::Bool(a == b)
                        }
                        _ => return Err("verified binary domain"),
                    };
                    if !self.tick(1, &ins.span) || !self.cell(&ins.span) {
                        break;
                    }
                    Some(v)
                }
                Op::WritePlace { local, value } => {
                    let v = Self::take(&mut temps, *value)?;
                    let dest = locals.get_mut(local.0).ok_or("write index")?;
                    if dest.replace(v).is_some() {
                        self.cells = self.cells.saturating_sub(1)
                    }
                    None
                }
                Op::CallIntrinsic { intrinsic, args } => {
                    if !self.tick(2, &ins.span) {
                        break;
                    } // resolved target name and intrinsic entry
                    let mut values = Vec::with_capacity(args.len());
                    for id in args {
                        values.push(Self::take(&mut temps, *id)?);
                        self.cells = self.cells.saturating_sub(1);
                    }
                    match intrinsic {
                        Intrinsic::Print => {
                            if !self.tick(1, &ins.span) {
                                break;
                            }
                            if !self.cell(&ins.span) {
                                break;
                            }
                            let line = values[0].text();
                            if let Err(observed) = self.log.push_stdout(line) {
                                self.cells = self.cells.saturating_sub(1);
                                self.fault(
                                    "R0002",
                                    &ins.span,
                                    vec![
                                        (
                                            "runtime_budget_kind",
                                            DataValue::String("observation_bytes".into()),
                                        ),
                                        ("limit", DataValue::UInt(65_536)),
                                        ("observed", DataValue::UInt(observed)),
                                    ],
                                );
                                break;
                            }
                        }
                        Intrinsic::AssertEq => {
                            if !self.tick(2, &ins.span) {
                                break;
                            }
                            if values[0] != values[1] {
                                self.fault("R0201", &ins.span, vec![]);
                                break;
                            }
                        }
                        Intrinsic::DropData => {}
                        Intrinsic::Fail => {
                            self.fault("R0301", &ins.span, vec![]);
                            break;
                        }
                    }
                    if *intrinsic != Intrinsic::Print && !self.cell(&ins.span) {
                        break;
                    }
                    Some(Value::Unit)
                }
                Op::DropValue(id) => {
                    Self::take(&mut temps, *id)?;
                    self.cells = self.cells.saturating_sub(1);
                    None
                }
                Op::DropLocal(id) => {
                    if locals
                        .get_mut(id.0)
                        .ok_or("drop local index")?
                        .take()
                        .is_some()
                    {
                        self.cells = self.cells.saturating_sub(1)
                    }
                    None
                }
            };
            if let Some(v) = value {
                let id = ins.output.ok_or("missing output")?;
                if id.0 != temps.len() || v.ty() != ins.ty.ok_or("output type")? {
                    return Err("output mismatch");
                }
                temps.push(Some(v));
            }
        }
        if self.fault.is_none() {
            let crate::ir::model::Terminator::Return {
                value: Some(id), ..
            } = &function.block.terminator
            else {
                return Err("return terminator");
            };
            let v = Self::take(&mut temps, *id)?;
            self.cells = self.cells.saturating_sub(1);
            if v != Value::Unit {
                return Err("non-unit main result");
            }
        }
        for id in &function.cleanup {
            if locals
                .get_mut(id.0)
                .ok_or("cleanup index")?
                .take()
                .is_some()
            {
                self.cells = self.cells.saturating_sub(1)
            }
        }
        Ok(())
    }
}
trait TerminatorSpan {
    fn terminator_span(&self) -> diagnostic::Span;
}
impl TerminatorSpan for crate::ir::model::BasicBlock {
    fn terminator_span(&self) -> diagnostic::Span {
        match &self.terminator {
            crate::ir::model::Terminator::Return { span, .. } => span.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn span() -> diagnostic::Span {
        diagnostic::Span {
            file: "limits.nx".into(),
            start: 1,
            end: 2,
        }
    }
    fn machine() -> Machine {
        Machine {
            log: Log::default(),
            work: 0,
            cells: 0,
            fault: None,
        }
    }
    #[test]
    fn work_boundary_grants_limit_and_rejects_next_unit_with_exact_fields() {
        let mut m = machine();
        m.work = 999_999;
        assert!(m.tick(1, &span()));
        assert_eq!(m.work, 1_000_000);
        assert!(!m.tick(1, &span()));
        assert_eq!(m.work, 1_000_000);
        let d = m.fault.unwrap();
        assert_eq!(
            (
                &*d.code,
                d.operation.as_str(),
                d.primary_span.as_ref().map(|s| (s.start, s.end))
            ),
            ("R0001", "runtime.limit", Some((1, 2)))
        );
        assert_eq!(d.data.get("limit"), Some(&DataValue::UInt(1_000_000)));
        assert_eq!(d.data.get("observed"), Some(&DataValue::UInt(1_000_001)));
        assert_eq!(
            d.data.get("work_consumed"),
            Some(&DataValue::UInt(1_000_000))
        );
        d.validate("limits.nx", "abc").unwrap();
    }
    #[test]
    fn cell_boundary_rejects_proposed_occupation_without_committing_it() {
        let mut m = machine();
        m.cells = 65_535;
        assert!(m.cell(&span()));
        assert_eq!(m.cells, 65_536);
        assert!(!m.cell(&span()));
        assert_eq!(m.cells, 65_536);
        let d = m.fault.unwrap();
        assert_eq!(d.code, "R0002");
        assert_eq!(
            d.data.get("runtime_budget_kind"),
            Some(&DataValue::String("live_value_cells".into()))
        );
        assert_eq!(d.data.get("limit"), Some(&DataValue::UInt(65_536)));
        assert_eq!(d.data.get("observed"), Some(&DataValue::UInt(65_537)));
        d.validate("limits.nx", "abc").unwrap();
    }
    #[test]
    fn rejected_print_releases_prepared_result_cell_before_cleanup() {
        let program = crate::ir::compile_scalar("limits.nx", "fn main(){print(0);}").unwrap();
        let mut m = machine();
        m.log.push_stdout("x".repeat(65_535)).unwrap();
        m.run(program.functions().function("main").unwrap())
            .unwrap();
        assert_eq!(m.fault.as_ref().map(|d| d.code.as_str()), Some("R0002"));
        assert_eq!(m.cells, 0);
        assert_eq!(m.log.bytes(), 65_536);
    }
}
