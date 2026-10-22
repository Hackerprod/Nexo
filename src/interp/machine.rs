use std::sync::Arc;

use super::observation::Log;
use crate::diagnostic::{self, DataValue, Diagnostic, Namespace, Operation, Severity, Stage};
use crate::ir::model::{Binary, Function, Intrinsic, Op, ParameterMode, Program, TempId, Ty};

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
    #[cfg(test)]
    pub(crate) disposals: u64,
}
#[derive(Clone, Debug, PartialEq, Eq)]
enum Value {
    Int(i64),
    Bool(bool),
    Unit,
    Array(Arc<Vec<Value>>, u64),
    Resource { id: u64, label: i64, blocking: bool },
}
impl Value {
    fn deep_copy(&self) -> Self {
        match self {
            Self::Array(items, cells) => Self::Array(
                Arc::new(items.iter().map(Self::deep_copy).collect()),
                *cells,
            ),
            _ => self.clone(),
        }
    }
    fn text(&self) -> String {
        match self {
            Self::Int(i) => i.to_string(),
            Self::Bool(b) => b.to_string(),
            Self::Unit => "()".into(),
            Self::Array(xs, _) => format!(
                "[{}]",
                xs.iter().map(Self::text).collect::<Vec<_>>().join(", ")
            ),
            Self::Resource { .. } => "<resource>".into(),
        }
    }
    fn has_ty(&self, ty: &Ty) -> bool {
        match (self, ty) {
            (Self::Int(_), Ty::I64) | (Self::Bool(_), Ty::Bool) | (Self::Unit, Ty::Unit) => true,
            (Self::Array(xs, _), Ty::Array(element)) => xs.iter().all(|x| x.has_ty(element)),
            (
                Self::Resource {
                    blocking: false, ..
                },
                Ty::MockResource,
            )
            | (Self::Resource { blocking: true, .. }, Ty::MockBlockingResource) => true,
            _ => false,
        }
    }
    fn cells(&self) -> u64 {
        match self {
            Self::Array(_, n) => *n,
            _ => 1,
        }
    }
    fn element(&self, index: i64) -> Option<&Value> {
        let Self::Array(xs, _) = self else {
            return None;
        };
        xs.get(usize::try_from(index).ok()?)
    }
    fn element_mut(&mut self, index: i64) -> Option<&mut Value> {
        let Self::Array(xs, _) = self else {
            return None;
        };
        Arc::get_mut(xs)?.get_mut(usize::try_from(index).ok()?)
    }
}
enum TempValue {
    Own(Value),
    Borrow(crate::ir::model::LocalId),
    BorrowIndex(crate::ir::model::LocalId, Vec<i64>),
    Exclusive(crate::ir::model::LocalId, Vec<i64>),
}
impl TempValue {
    fn owned_cells(&self) -> u64 {
        match self {
            Self::Own(v) => v.cells(),
            Self::Borrow(_) | Self::BorrowIndex(_, _) | Self::Exclusive(_, _) => 0,
        }
    }
    fn as_value<'a>(&'a self, locals: &'a [Option<Value>]) -> Result<&'a Value, &'static str> {
        match self {
            Self::Own(v) => Ok(v),
            Self::Borrow(id) => locals
                .get(id.0)
                .ok_or("borrow local index")?
                .as_ref()
                .ok_or("borrowed empty local"),
            Self::BorrowIndex(id, path) => {
                let mut value = locals
                    .get(id.0)
                    .ok_or("borrow index local")?
                    .as_ref()
                    .ok_or("borrowed empty local")?;
                for index in path {
                    value = value.element(*index).ok_or("borrow index path")?;
                }
                Ok(value)
            }
            Self::Exclusive(_, _) => Err("exclusive loan used as value"),
        }
    }
    fn into_owned(self) -> Result<Value, &'static str> {
        match self {
            Self::Own(v) => Ok(v),
            Self::Borrow(_) | Self::BorrowIndex(_, _) | Self::Exclusive(_, _) => {
                Err("borrowed value used as owner")
            }
        }
    }
}
struct FrameReturn {
    value: Option<Value>,
    inout: Vec<Option<Value>>,
}

pub fn run_scalar(program: &Program) -> Execution {
    std::thread::scope(|scope| {
        match std::thread::Builder::new()
            .stack_size(16 * 1024 * 1024)
            .spawn_scoped(scope, || run_verified(program))
        {
            Ok(worker) => worker
                .join()
                .unwrap_or_else(|_| internal_execution("interpreter worker panicked")),
            Err(_) => internal_execution("interpreter worker unavailable"),
        }
    })
}
fn internal_execution(reason: &'static str) -> Execution {
    Execution {
        outcome: Outcome::InternalError(reason),
        observations: Log::default(),
        diagnostics: vec![],
        work_consumed: 0,
        #[cfg(test)]
        disposals: 0,
    }
}
fn run_verified(program: &Program) -> Execution {
    let mut state = Machine {
        log: Log::default(),
        work: 0,
        cells: 0,
        fault: None,
        frames: 0,
        next_resource: 0,
        #[cfg(test)]
        disposals: 0,
    };
    let result = crate::ir::verify::verify_program(program)
        .and_then(|()| program.functions.function("main").ok_or("missing main"));
    let Ok(function) = result else {
        return Execution {
            outcome: Outcome::InternalError(result.err().unwrap_or("invalid program")),
            observations: state.log,
            diagnostics: vec![],
            work_consumed: 0,
            #[cfg(test)]
            disposals: state.disposals,
        };
    };
    let outcome = match state.run_frame(
        function,
        Some(program),
        vec![],
        &function.block.terminator_span(),
    ) {
        Ok(FrameReturn {
            value: Some(Value::Unit),
            ..
        }) => {
            state.cells = state.cells.saturating_sub(1);
            Outcome::Unit
        }
        Ok(FrameReturn { value: None, .. }) => Outcome::Unit,
        Ok(FrameReturn {
            value: Some(other), ..
        }) => {
            state.dispose(other).ok();
            Outcome::InternalError("non-unit main result")
        }
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
        #[cfg(test)]
        disposals: state.disposals,
    }
}
struct Machine {
    log: Log,
    work: u64,
    cells: u64,
    fault: Option<Box<Diagnostic>>,
    frames: u64,
    next_resource: u64,
    #[cfg(test)]
    disposals: u64,
}
impl Machine {
    fn dispose(&mut self, value: Value) -> Result<(), &'static str> {
        #[cfg(test)]
        {
            self.disposals += 1;
        }
        self.cells = self.cells.saturating_sub(value.cells());
        if let Value::Resource { id, label, .. } = value {
            self.log
                .emit_reserved_event(format!("close:{id}:{label}"), "mock-resource:")?;
        }
        Ok(())
    }
    fn observation_fault(&mut self, span: &diagnostic::Span, observed: u64) {
        self.fault(
            "R0002",
            span,
            vec![
                (
                    "runtime_budget_kind",
                    DataValue::String("observation_bytes".into()),
                ),
                ("limit", DataValue::UInt(65_536)),
                ("observed", DataValue::UInt(observed)),
            ],
        );
    }
    fn fault(&mut self, code: &str, span: &diagnostic::Span, data: Vec<(&str, DataValue)>) {
        if self.fault.is_some() {
            return;
        }
        let operation = match code {
            "R0001" | "R0002" => Operation::RuntimeLimit,
            "R0101" => Operation::ValueAdd,
            "R0103" => Operation::ValueSub,
            "R0102" => Operation::ValueIndex,
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
        self.reserve_cells(1, span)
    }
    fn reserve_cells(&mut self, n: u64, span: &diagnostic::Span) -> bool {
        let proposed = self.cells.saturating_add(n);
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
    fn take(temps: &mut [Option<TempValue>], id: TempId) -> Result<TempValue, &'static str> {
        temps
            .get_mut(id.0)
            .ok_or("temporary index")?
            .take()
            .ok_or("temporary already used")
    }
    fn install_temp(
        temps: &mut Vec<Option<TempValue>>,
        id: TempId,
        value: TempValue,
    ) -> Result<(), &'static str> {
        if id.0 > 1_000_000 {
            return Err("temporary identity");
        }
        if id.0 >= temps.len() {
            temps.resize_with(id.0 + 1, || None);
        }
        if temps[id.0].replace(value).is_some() {
            return Err("temporary still live");
        }
        Ok(())
    }
    #[cfg(test)]
    fn run(&mut self, function: &Function) -> Result<(), &'static str> {
        let result = self.run_frame(function, None, vec![], &function.block.terminator_span())?;
        if let Some(v) = result.value {
            self.dispose(v)?;
        }
        Ok(())
    }
    fn run_frame(
        &mut self,
        function: &Function,
        program: Option<&Program>,
        args: Vec<(Value, bool)>,
        call_span: &diagnostic::Span,
    ) -> Result<FrameReturn, &'static str> {
        if !self.tick(1, call_span) {
            let mut inout = (0..args.len()).map(|_| None).collect::<Vec<_>>();
            for (slot, (v, borrowed)) in args.into_iter().enumerate().rev() {
                if function.parameters[slot].mode == ParameterMode::Inout {
                    inout[slot] = Some(v);
                } else if !borrowed {
                    self.dispose(v)?;
                }
            }
            return Ok(FrameReturn { value: None, inout });
        }
        if self.frames >= 256 {
            self.fault(
                "R0002",
                call_span,
                vec![
                    (
                        "runtime_budget_kind",
                        DataValue::String("call_frames".into()),
                    ),
                    ("limit", DataValue::UInt(256)),
                    ("observed", DataValue::UInt(257)),
                ],
            );
            let mut inout = (0..args.len()).map(|_| None).collect::<Vec<_>>();
            for (slot, (v, borrowed)) in args.into_iter().enumerate().rev() {
                if function.parameters[slot].mode == ParameterMode::Inout {
                    inout[slot] = Some(v);
                } else if !borrowed {
                    self.dispose(v)?;
                }
            }
            return Ok(FrameReturn { value: None, inout });
        }
        self.frames += 1;
        let mut locals: Vec<Option<Value>> = vec![None; function.locals.len()];
        let mut borrowed_locals = vec![false; function.locals.len()];
        if args.len() != function.parameters.len() {
            return Err("frame argument count");
        }
        for (p, (v, borrowed)) in function.parameters.iter().zip(args) {
            if borrowed != (p.mode != ParameterMode::Consume) {
                return Err("frame argument mode");
            }
            borrowed_locals[p.local.0] = borrowed;
            locals[p.local.0] = Some(v);
        }
        let mut temps: Vec<Option<TempValue>> = Vec::new();
        let mut current = &function.block;
        let mut returned = None;
        'cfg: loop {
            for ins in &current.instructions {
                if !self.tick(u64::from(ins.tick), &ins.span) {
                    break;
                }
                if let Op::BorrowValue(id) = &ins.op {
                    Self::install_temp(
                        &mut temps,
                        ins.output.ok_or("borrow output identity")?,
                        TempValue::Borrow(*id),
                    )?;
                    continue;
                }
                if let Op::BeginLoan {
                    root,
                    indexes,
                    permission,
                    ..
                } = &ins.op
                {
                    let mut path = Vec::with_capacity(indexes.len());
                    for id in indexes {
                        let Value::Int(index) = Self::take(&mut temps, *id)?.into_owned()? else {
                            return Err("loan index type");
                        };
                        self.cells = self.cells.saturating_sub(1);
                        path.push(index);
                    }
                    let mut selected = locals
                        .get(root.0)
                        .ok_or("loan root")?
                        .as_ref()
                        .ok_or("loan unavailable root")?;
                    let mut valid = true;
                    for index in &path {
                        if let Some(next) = selected.element(*index) {
                            selected = next;
                        } else {
                            valid = false;
                            break;
                        }
                    }
                    if !valid {
                        self.fault("R0102", &ins.span, vec![]);
                        break;
                    }
                    let loan_value = match permission {
                        crate::ir::model::LoanPermission::Read if path.is_empty() => {
                            TempValue::Borrow(*root)
                        }
                        crate::ir::model::LoanPermission::Read => {
                            TempValue::BorrowIndex(*root, path)
                        }
                        crate::ir::model::LoanPermission::Inout => {
                            TempValue::Exclusive(*root, path)
                        }
                    };
                    Self::install_temp(
                        &mut temps,
                        ins.output.ok_or("loan output identity")?,
                        loan_value,
                    )?;
                    continue;
                }
                if let Op::BorrowIndex { local, indexes } = &ins.op {
                    let mut path = Vec::with_capacity(indexes.len());
                    for id in indexes {
                        let Value::Int(index) = Self::take(&mut temps, *id)?.into_owned()? else {
                            return Err("borrow index type");
                        };
                        self.cells = self.cells.saturating_sub(1);
                        path.push(index);
                    }
                    let mut value = locals
                        .get(local.0)
                        .ok_or("borrow index local")?
                        .as_ref()
                        .ok_or("borrowed empty local")?;
                    let mut valid = true;
                    for index in &path {
                        if let Some(next) = value.element(*index) {
                            value = next;
                        } else {
                            valid = false;
                            break;
                        }
                    }
                    if !valid {
                        self.fault("R0102", &ins.span, vec![]);
                        break;
                    }
                    Self::install_temp(
                        &mut temps,
                        ins.output.ok_or("borrow index output identity")?,
                        TempValue::BorrowIndex(*local, path),
                    )?;
                    continue;
                }
                let value = match &ins.op {
                    Op::BorrowValue(_) => return Err("borrow dispatch"),
                    Op::BorrowIndex { .. } => return Err("borrow index dispatch"),
                    Op::BeginLoan { .. } => return Err("loan dispatch"),
                    Op::EndLoan(_) => None,
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
                            .as_ref()
                            .ok_or("empty local")?;
                        if !self.tick(v.cells(), &ins.span)
                            || !self.reserve_cells(v.cells(), &ins.span)
                        {
                            break;
                        }
                        Some(v.deep_copy())
                    }
                    Op::TransferValue(id) => {
                        let v = locals
                            .get_mut(id.0)
                            .ok_or("transfer local index")?
                            .take()
                            .ok_or("transfer consumed local")?;
                        Some(v)
                    }
                    Op::BuildArray(items) => {
                        let mut xs = Vec::with_capacity(items.len());
                        let mut total = 1u64;
                        for id in items {
                            let v = Self::take(&mut temps, *id)?.into_owned()?;
                            total = total.checked_add(v.cells()).ok_or("array size overflow")?;
                            xs.push(v);
                        }
                        if !self.tick(1, &ins.span) || !self.cell(&ins.span) {
                            self.cells = self.cells.saturating_sub(total - 1);
                            break;
                        }
                        Some(Value::Array(Arc::new(xs), total))
                    }
                    Op::ReadIndex { local, indexes } => {
                        let mut path = Vec::with_capacity(indexes.len());
                        for id in indexes {
                            let Value::Int(n) = Self::take(&mut temps, *id)?.into_owned()? else {
                                return Err("index value type");
                            };
                            self.cells = self.cells.saturating_sub(1);
                            path.push(n);
                        }
                        let mut selected = locals
                            .get(local.0)
                            .ok_or("read index local")?
                            .as_ref()
                            .ok_or("empty local")?;
                        let mut valid = true;
                        for index in path {
                            if let Some(next) = selected.element(index) {
                                selected = next;
                            } else {
                                valid = false;
                                break;
                            }
                        }
                        if !valid {
                            self.fault("R0102", &ins.span, vec![]);
                            break;
                        }
                        if !self.tick(selected.cells(), &ins.span)
                            || !self.reserve_cells(selected.cells(), &ins.span)
                        {
                            break;
                        }
                        Some(selected.deep_copy())
                    }
                    Op::CheckIndex { local, indexes } => {
                        let mut selected = locals
                            .get(local.0)
                            .ok_or("check index local")?
                            .as_ref()
                            .ok_or("empty local")?;
                        let mut valid = true;
                        for id in indexes {
                            let Some(TempValue::Own(Value::Int(n))) =
                                temps.get(id.0).and_then(Option::as_ref)
                            else {
                                return Err("check index temporary");
                            };
                            if let Some(next) = selected.element(*n) {
                                selected = next;
                            } else {
                                valid = false;
                                break;
                            }
                        }
                        if !valid {
                            self.fault("R0102", &ins.span, vec![]);
                            break;
                        }
                        None
                    }
                    Op::IndexValue { base, index } => {
                        let base = Self::take(&mut temps, *base)?.into_owned()?;
                        let Value::Int(n) = Self::take(&mut temps, *index)?.into_owned()? else {
                            return Err("index value type");
                        };
                        let size = base.cells();
                        let Some(selected) = base.element(n) else {
                            self.cells = self.cells.saturating_sub(size + 1);
                            self.fault("R0102", &ins.span, vec![]);
                            break;
                        };
                        let selected = selected.clone();
                        self.cells = self.cells.saturating_sub(size + 1 - selected.cells());
                        Some(selected)
                    }
                    Op::Binary { op, left, right } => {
                        let l = Self::take(&mut temps, *left)?;
                        let r = Self::take(&mut temps, *right)?;
                        self.cells = self.cells.saturating_sub(l.owned_cells() + r.owned_cells());
                        let lv = l.as_value(&locals)?;
                        let rv = r.as_value(&locals)?;
                        let v = match (op, lv, rv) {
                            (Binary::Add, Value::Int(a), Value::Int(b)) => {
                                match a.checked_add(*b) {
                                    Some(n) => Value::Int(n),
                                    None => {
                                        self.fault("R0101", &ins.span, vec![]);
                                        break;
                                    }
                                }
                            }
                            (Binary::Sub, Value::Int(a), Value::Int(b)) => {
                                match a.checked_sub(*b) {
                                    Some(n) => Value::Int(n),
                                    None => {
                                        self.fault("R0103", &ins.span, vec![]);
                                        break;
                                    }
                                }
                            }
                            (Binary::Less, Value::Int(a), Value::Int(b)) => {
                                if !self.tick(2, &ins.span) {
                                    break;
                                }
                                Value::Bool(a < b)
                            }
                            (Binary::Equal, a, b) => {
                                if !self.tick(a.cells().saturating_add(b.cells()), &ins.span) {
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
                        let v = Self::take(&mut temps, *value)?.into_owned()?;
                        let dest = locals.get_mut(local.0).ok_or("write index")?;
                        if let Some(old) = dest.replace(v) {
                            self.dispose(old)?;
                        }
                        None
                    }
                    Op::WriteIndex {
                        local,
                        indexes,
                        value,
                    } => {
                        let mut path = Vec::with_capacity(indexes.len());
                        for id in indexes {
                            let Value::Int(n) = Self::take(&mut temps, *id)?.into_owned()? else {
                                return Err("index value type");
                            };
                            self.cells = self.cells.saturating_sub(1);
                            path.push(n);
                        }
                        let replacement = Self::take(&mut temps, *value)?.into_owned()?;
                        let root = locals
                            .get_mut(local.0)
                            .ok_or("write index local")?
                            .as_mut()
                            .ok_or("empty local")?;
                        let Some(node) = get_mut_path(root, &path) else {
                            self.cells = self.cells.saturating_sub(replacement.cells());
                            self.fault("R0102", &ins.span, vec![]);
                            break;
                        };
                        let previous = std::mem::replace(node, replacement);
                        let old_size = previous.cells();
                        let new_size = node.cells();
                        self.dispose(previous)?;
                        if let Some(root) = locals[local.0].as_mut() {
                            adjust_size(root, &path, old_size, new_size);
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
                            if !matches!(
                                values.last(),
                                Some(TempValue::Own(Value::Resource { .. }))
                            ) {
                                self.cells = self.cells.saturating_sub(
                                    values.last().map_or(0, TempValue::owned_cells),
                                );
                            }
                        }
                        let mut pre_reserved_output = false;
                        let output = match intrinsic {
                            Intrinsic::Print => {
                                let value = values[0].as_value(&locals)?;
                                if !self.tick(value.cells(), &ins.span) {
                                    break;
                                }
                                if !self.cell(&ins.span) {
                                    break;
                                }
                                let line = value.text();
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
                                Value::Unit
                            }
                            Intrinsic::AssertEq => {
                                let left = values[0].as_value(&locals)?;
                                let right = values[1].as_value(&locals)?;
                                if !self.tick(left.cells().saturating_add(right.cells()), &ins.span)
                                {
                                    break;
                                }
                                if left != right {
                                    self.fault("R0201", &ins.span, vec![]);
                                    break;
                                }
                                Value::Unit
                            }
                            Intrinsic::DropData => Value::Unit,
                            Intrinsic::Fail => {
                                self.fault("R0301", &ins.span, vec![]);
                                break;
                            }
                            Intrinsic::Len => {
                                let Value::Array(xs, _) = values[0].as_value(&locals)? else {
                                    return Err("len domain");
                                };
                                let n =
                                    i64::try_from(xs.len()).map_err(|_| "array length overflow")?;
                                Value::Int(n)
                            }
                            Intrinsic::MockOpen | Intrinsic::MockBlockingOpen => {
                                let Value::Int(label) = values[0].as_value(&locals)? else {
                                    return Err("open label type");
                                };
                                let label = *label;
                                if !self.cell(&ins.span) {
                                    break;
                                }
                                pre_reserved_output = true;
                                let id = self.next_resource;
                                let close = format!("close:{id}:{label}");
                                let close_bytes = ("mock-resource:".len() + close.len() + 1) as u64;
                                if let Err(observed) = self.log.reserve(close_bytes) {
                                    self.cells -= 1;
                                    self.observation_fault(&ins.span, observed);
                                    break;
                                }
                                if let Err(observed) = self
                                    .log
                                    .push_event(format!("open:{id}:{label}"), "mock-resource:")
                                {
                                    self.log.release_reservation(close_bytes)?;
                                    self.cells -= 1;
                                    self.observation_fault(&ins.span, observed);
                                    break;
                                }
                                self.next_resource += 1;
                                Value::Resource {
                                    id,
                                    label,
                                    blocking: *intrinsic == Intrinsic::MockBlockingOpen,
                                }
                            }
                            Intrinsic::Inspect => {
                                let Value::Resource { id, label, .. } =
                                    values[0].as_value(&locals)?
                                else {
                                    return Err("inspect type");
                                };
                                let text = format!("inspect:{id}:{label}");
                                if !self.cell(&ins.span) {
                                    for v in values.drain(..).rev() {
                                        if matches!(&v, TempValue::Own(Value::Resource { .. })) {
                                            self.dispose(v.into_owned()?)?;
                                        }
                                    }
                                    break;
                                }
                                pre_reserved_output = true;
                                if let Err(observed) = self.log.push_event(text, "mock-resource:") {
                                    self.cells -= 1;
                                    self.observation_fault(&ins.span, observed);
                                    for v in values.drain(..).rev() {
                                        if matches!(&v, TempValue::Own(Value::Resource { .. })) {
                                            self.dispose(v.into_owned()?)?;
                                        }
                                    }
                                    break;
                                }
                                Value::Unit
                            }
                        };
                        for v in values.into_iter().rev() {
                            if matches!(&v, TempValue::Own(Value::Resource { .. })) {
                                self.dispose(v.into_owned()?)?;
                            }
                        }
                        if *intrinsic != Intrinsic::Print
                            && !pre_reserved_output
                            && !self.cell(&ins.span)
                        {
                            break;
                        }
                        Some(output)
                    }
                    Op::CallDirect {
                        function: callee,
                        args,
                        signature,
                    } => {
                        if !self.tick(1, &ins.span) {
                            break;
                        }
                        let program = program.ok_or("call table unavailable")?;
                        let target = program
                            .functions
                            .function(&callee.0)
                            .ok_or("unknown callee")?;
                        let mut passed: Vec<Option<(Value, bool)>> = vec![None; signature.len()];
                        let mut retained = Vec::new();
                        let mut targets = vec![None; signature.len()];
                        for (slot, id) in args {
                            let temp = Self::take(&mut temps, *id)?;
                            let item = match signature[*slot].1 {
                                ParameterMode::Read => {
                                    let borrowed = temp.as_value(&locals)?.clone();
                                    if let TempValue::Own(owner) = temp {
                                        retained.push(owner);
                                    }
                                    (borrowed, true)
                                }
                                ParameterMode::Consume => (temp.into_owned()?, false),
                                ParameterMode::Inout => {
                                    let TempValue::Exclusive(root, path) = temp else {
                                        return Err("inout argument loan");
                                    };
                                    let value = if path.is_empty() {
                                        locals
                                            .get_mut(root.0)
                                            .ok_or("inout root")?
                                            .take()
                                            .ok_or("inout unavailable root")?
                                    } else {
                                        let root_value = locals
                                            .get_mut(root.0)
                                            .ok_or("inout root")?
                                            .as_mut()
                                            .ok_or("inout unavailable root")?;
                                        // Move the selected owner into the callee. The unit is an
                                        // inaccessible structural placeholder, not a live Nexo cell.
                                        let selected = get_mut_path(root_value, &path)
                                            .ok_or("inout index path")?;
                                        let value = std::mem::replace(selected, Value::Unit);
                                        adjust_size(root_value, &path, value.cells(), 1);
                                        value
                                    };
                                    targets[*slot] = Some((root, path));
                                    (value, true)
                                }
                            };
                            passed[*slot] = Some(item);
                        }
                        let prepared = passed
                            .into_iter()
                            .map(|v| v.ok_or("missing call slot"))
                            .collect::<Result<Vec<_>, _>>()?;
                        if self.fault.is_some() {
                            break;
                        }
                        let result = self.run_frame(target, Some(program), prepared, &ins.span)?;
                        for (slot, returned) in result.inout.into_iter().enumerate() {
                            if let Some(value) = returned {
                                let (root, path) =
                                    targets[slot].take().ok_or("inout return target")?;
                                if path.is_empty() {
                                    if locals[root.0].replace(value).is_some() {
                                        return Err("inout occupied root");
                                    }
                                } else {
                                    let root_value =
                                        locals[root.0].as_mut().ok_or("inout missing root")?;
                                    let node = get_mut_path(root_value, &path)
                                        .ok_or("inout return path")?;
                                    let previous = std::mem::replace(node, value);
                                    if previous != Value::Unit {
                                        return Err("inout placeholder replaced");
                                    }
                                    let new_size = node.cells();
                                    adjust_size(root_value, &path, 1, new_size);
                                }
                            }
                        }
                        for value in retained.into_iter().rev() {
                            self.dispose(value)?;
                        }
                        if self.fault.is_some() {
                            break;
                        }
                        result.value
                    }
                    Op::DropValue(id) => {
                        let v = Self::take(&mut temps, *id)?;
                        if let TempValue::Own(value) = v {
                            self.dispose(value)?;
                        }
                        None
                    }
                    Op::DropBinding(id) => {
                        let old = locals
                            .get_mut(id.0)
                            .ok_or("drop binding index")?
                            .take()
                            .ok_or("drop empty binding")?;
                        self.dispose(old)?;
                        if !self.cell(&ins.span) {
                            break;
                        }
                        Some(Value::Unit)
                    }
                    Op::DropLocal(id) => {
                        if let Some(old) = locals.get_mut(id.0).ok_or("drop local index")?.take()
                            && !borrowed_locals[id.0]
                        {
                            self.dispose(old)?;
                        }
                        None
                    }
                };
                if let Some(v) = value {
                    let id = ins.output.ok_or("missing output")?;
                    if !v.has_ty(ins.ty.as_ref().ok_or("output type")?) {
                        return Err("output mismatch");
                    }
                    Self::install_temp(&mut temps, id, TempValue::Own(v))?;
                }
            }
            if self.fault.is_some() {
                break 'cfg;
            }
            match &current.terminator {
                crate::ir::model::Terminator::Return {
                    value: Some(id), ..
                } => {
                    returned = Some(Self::take(&mut temps, *id)?.into_owned()?);
                    break 'cfg;
                }
                crate::ir::model::Terminator::Return { value: None, .. } => {
                    return Err("return terminator");
                }
                crate::ir::model::Terminator::Jump { target, span } => {
                    if !self.tick(1, span) {
                        break 'cfg;
                    }
                    current = if target.0 == 0 {
                        &function.block
                    } else {
                        function.blocks.get(target.0 - 1).ok_or("jump target")?
                    };
                }
                crate::ir::model::Terminator::BranchBool {
                    condition,
                    yes,
                    no,
                    span,
                } => {
                    if !self.tick(1, span) {
                        break 'cfg;
                    }
                    let Value::Bool(test) = Self::take(&mut temps, *condition)?.into_owned()?
                    else {
                        return Err("branch condition type");
                    };
                    self.cells = self.cells.saturating_sub(1);
                    let target = if test { yes } else { no };
                    current = if target.0 == 0 {
                        &function.block
                    } else {
                        function.blocks.get(target.0 - 1).ok_or("branch target")?
                    };
                }
            }
        }
        for v in temps.into_iter().rev().flatten() {
            if let TempValue::Own(value) = v {
                self.dispose(value)?;
            }
        }
        let mut inout = (0..function.parameters.len())
            .map(|_| None)
            .collect::<Vec<_>>();
        for (slot, p) in function.parameters.iter().enumerate() {
            if p.mode == ParameterMode::Inout {
                inout[slot] = locals[p.local.0].take();
            }
        }
        for id in &function.cleanup {
            if let Some(old) = locals.get_mut(id.0).ok_or("cleanup index")?.take()
                && !borrowed_locals[id.0]
            {
                self.dispose(old)?;
            }
        }
        self.frames -= 1;
        Ok(FrameReturn {
            value: returned,
            inout,
        })
    }
}
fn get_mut_path<'a>(mut node: &'a mut Value, path: &[i64]) -> Option<&'a mut Value> {
    for &index in path {
        node = node.element_mut(index)?;
    }
    Some(node)
}
fn adjust_size(root: &mut Value, path: &[i64], old: u64, new: u64) {
    let mut node = root;
    for &index in path {
        let Value::Array(xs, cells) = node else {
            return;
        };
        *cells = cells.saturating_sub(old).saturating_add(new);
        let Ok(index) = usize::try_from(index) else {
            return;
        };
        let Some(next) = Arc::get_mut(xs).and_then(|items| items.get_mut(index)) else {
            return;
        };
        node = next;
    }
}
trait TerminatorSpan {
    fn terminator_span(&self) -> diagnostic::Span;
}
impl TerminatorSpan for crate::ir::model::BasicBlock {
    fn terminator_span(&self) -> diagnostic::Span {
        match &self.terminator {
            crate::ir::model::Terminator::Return { span, .. } => span.clone(),
            crate::ir::model::Terminator::Jump { span, .. }
            | crate::ir::model::Terminator::BranchBool { span, .. } => span.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn drop_binding_disarms_runtime_slot_before_later_instruction() {
        let program =
            crate::ir::compile_scalar("slot.nx", "fn main(){let a=[1];drop(a);}").unwrap();
        let mut function = program.functions().function("main").unwrap().clone();
        let drop_pos = function
            .block
            .instructions
            .iter()
            .position(|i| matches!(i.op, Op::DropBinding(_)))
            .unwrap();
        let next = TempId(
            function.block.instructions[..=drop_pos]
                .iter()
                .filter(|i| i.output.is_some())
                .count(),
        );
        let span = function.block.instructions[drop_pos].span.clone();
        function.block.instructions.insert(
            drop_pos + 1,
            crate::ir::model::Instruction {
                op: Op::CopyValue(crate::ir::model::LocalId(0)),
                output: Some(next),
                ty: Some(Ty::Array(Box::new(Ty::I64))),
                span,
                scope: crate::ir::model::ScopeId(0),
                origin: crate::ir::model::OriginId(drop_pos + 1),
                access: crate::ir::model::Access::Read,
                effect_io: false,
                tick: true,
            },
        );
        let mut state = machine();
        assert_eq!(state.run(&function), Err("empty local"));
    }
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
            frames: 0,
            next_resource: 0,
            disposals: 0,
        }
    }
    #[test]
    fn denied_frame_tick_releases_prepared_owners_in_reverse_and_preserves_borrow() {
        let program = crate::ir::compile_scalar(
            "limits.nx",
            "fn own(consume a:MockResource,consume b:MockResource){} fn read(a:MockResource,consume b:MockResource){} fn main(){}",
        )
        .unwrap();
        let resource = |id| Value::Resource {
            id,
            label: id as i64 + 1,
            blocking: false,
        };
        let mut own = machine();
        own.work = 1_000_000;
        own.cells = 2;
        for id in 0..2 {
            let close = format!("close:{id}:{}", id + 1);
            own.log
                .reserve(("mock-resource:".len() + close.len() + 1) as u64)
                .unwrap();
        }
        let target = program.functions().function("own").unwrap();
        assert!(
            own.run_frame(
                target,
                Some(&program),
                vec![(resource(0), false), (resource(1), false)],
                &span()
            )
            .unwrap()
            .value
            .is_none()
        );
        assert_eq!(own.log.events(), ["close:1:2", "close:0:1"]);
        assert_eq!(own.cells, 0);

        let mut read = machine();
        read.work = 1_000_000;
        read.cells = 2;
        for id in 0..2 {
            let close = format!("close:{id}:{}", id + 1);
            read.log
                .reserve(("mock-resource:".len() + close.len() + 1) as u64)
                .unwrap();
        }
        let target = program.functions().function("read").unwrap();
        assert!(
            read.run_frame(
                target,
                Some(&program),
                vec![(resource(0), true), (resource(1), false)],
                &span()
            )
            .unwrap()
            .value
            .is_none()
        );
        assert_eq!(read.log.events(), ["close:1:2"]);
        read.dispose(resource(0)).unwrap();
        assert_eq!(read.log.events(), ["close:1:2", "close:0:1"]);
        assert_eq!(read.cells, 0);
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
    fn indexed_inout_and_read_have_same_exact_cell_boundary() {
        for source in [
            "fn f(inout x:Array<i64>){} fn main(){var a=[[1]];f(inout a[0]);}",
            "fn f(x:Array<i64>){} fn main(){var a=[[1]];f(a[0]);}",
        ] {
            let p = crate::ir::compile_scalar("limits.nx", source).unwrap();
            for (preload, accepted) in [(65_531, true), (65_532, true), (65_533, false)] {
                let mut m = machine();
                m.cells = preload;
                let returned = m
                    .run_frame(
                        p.functions().function("main").unwrap(),
                        Some(&p),
                        vec![],
                        &span(),
                    )
                    .unwrap();
                if let Some(value) = returned.value {
                    m.dispose(value).unwrap();
                }
                assert_eq!(m.fault.is_none(), accepted, "{source}: {preload}");
                if let Some(fault) = m.fault.as_ref() {
                    assert_eq!(fault.code, "R0002");
                    assert_eq!(fault.data["observed"], DataValue::UInt(65_537));
                    fault.validate("limits.nx", source).unwrap();
                }
                assert_eq!(m.cells, preload, "{source}: {preload}");
            }
        }
    }
    #[test]
    fn indexed_array_growth_uses_exact_cached_cell_count() {
        let source = "fn main(){var a=[[0]];a[0]=[1,2];let b=a;}";
        let p = crate::ir::compile_scalar("growth.nx", source).unwrap();
        for (preload, accepted) in [(65_526, true), (65_527, true), (65_528, false)] {
            let mut m = machine();
            m.cells = preload;
            m.run(p.functions().function("main").unwrap()).unwrap();
            assert_eq!(m.fault.is_none(), accepted, "{preload}");
            if let Some(fault) = m.fault.as_ref() {
                assert_eq!(fault.code, "R0002");
                assert_eq!(fault.data["observed"], DataValue::UInt(65_537));
                fault.validate("growth.nx", source).unwrap();
            }
            assert_eq!(m.cells, preload);
        }
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
    #[test]
    fn failed_indexed_write_releases_rhs_and_root_cells() {
        let p = crate::ir::compile_scalar("limits.nx", "fn main(){var a=[[1]];a[2]=[3];}").unwrap();
        let mut m = machine();
        m.run(p.functions().function("main").unwrap()).unwrap();
        assert_eq!(m.fault.as_ref().map(|d| d.code.as_str()), Some("R0102"));
        assert_eq!(m.cells, 0);
    }
    #[test]
    fn rejected_array_copy_releases_prepared_values_and_all_bindings() {
        let mut source = String::from("fn main(){let a0=[0];");
        for k in 1..=14 {
            source.push_str(&format!("let a{k}=[a{},a{}];", k - 1, k - 1));
        }
        source.push('}');
        let p = crate::ir::compile_scalar("limits.nx", &source).unwrap();
        let mut m = machine();
        m.run(p.functions().function("main").unwrap()).unwrap();
        assert_eq!(m.fault.as_ref().map(|d| d.code.as_str()), Some("R0002"));
        assert_eq!(m.cells, 0);
    }
    #[test]
    fn cell_quota_accepts_occupancy_above_sixty_thousand_below_limit() {
        let mut m = machine();
        m.cells = 60_000;
        assert!(m.cell(&span()));
        assert_eq!(m.cells, 60_001);
        assert!(m.fault.is_none());
    }
    #[test]
    fn rejected_array_header_restores_prepared_element_cell() {
        let p = crate::ir::compile_scalar("limits.nx", "fn main(){let a=[1];}").unwrap();
        let mut m = machine();
        m.cells = 65_535;
        m.run(p.functions().function("main").unwrap()).unwrap();
        assert_eq!(m.fault.as_ref().map(|d| d.code.as_str()), Some("R0002"));
        assert_eq!(m.cells, 65_535);
    }
    #[test]
    fn repeated_borrowed_index_releases_consumed_index_cells() {
        let source = format!("fn main(){{let a=[[1]];{}}}", "len(a[0]);".repeat(66_000));
        let p = crate::ir::compile_scalar("limits.nx", &source).unwrap();
        let mut m = machine();
        m.run(p.functions().function("main").unwrap()).unwrap();
        assert!(m.fault.is_none(), "{:?}", m.fault);
        assert_eq!(m.cells, 0);
        assert_eq!(m.work, 528_010);
    }
    #[test]
    fn repeated_index_value_releases_unselected_array_cells() {
        let source = format!("fn main(){{{}}}", "len([[1,2,3],[4]][0]);".repeat(25_000));
        let p = crate::ir::compile_scalar("limits.nx", &source).unwrap();
        let mut m = machine();
        m.run(p.functions().function("main").unwrap()).unwrap();
        assert!(m.fault.is_none(), "{:?}", m.fault);
        assert_eq!(m.cells, 0);
        assert_eq!(m.work, 525_003);
    }
    #[test]
    fn ordinary_array_read_shares_storage_but_owner_copy_is_deep() {
        let original = Value::Array(Arc::new(vec![Value::Int(1)]), 2);
        let borrowed = original.clone();
        let owned = original.deep_copy();
        let (Value::Array(a, _), Value::Array(b, _), Value::Array(c, _)) =
            (&original, &borrowed, &owned)
        else {
            panic!("arrays")
        };
        assert!(Arc::ptr_eq(a, b));
        assert!(!Arc::ptr_eq(a, c));
        assert_eq!(original, owned);
    }
    #[test]
    fn failed_inspection_of_temporary_resource_still_emits_reserved_close() {
        let p =
            crate::ir::compile_scalar("limits.nx", "fn main(){inspect(mock_open(1));}").unwrap();
        let mut m = machine();
        m.log.push_stdout("x".repeat(65_488)).unwrap();
        m.run(p.functions().function("main").unwrap()).unwrap();
        assert_eq!(m.fault.as_ref().map(|d| d.code.as_str()), Some("R0002"));
        assert_eq!(m.log.events(), ["open:0:1", "close:0:1"]);
        assert_eq!(m.log.reserved_bytes(), 0);
        assert_eq!(m.cells, 0);
    }
    #[test]
    fn rejected_open_does_not_emit_acquisition_or_leave_cell() {
        let p = crate::ir::compile_scalar("limits.nx", "fn main(){let r=mock_open(1);}").unwrap();
        let mut m = machine();
        m.log.push_stdout("x".repeat(65_490)).unwrap();
        m.run(p.functions().function("main").unwrap()).unwrap();
        assert_eq!(m.fault.as_ref().map(|d| d.code.as_str()), Some("R0002"));
        assert!(m.log.events().is_empty());
        assert_eq!(m.log.reserved_bytes(), 0);
        assert_eq!(m.cells, 0);
    }
    #[test]
    fn rejected_open_event_releases_prepared_close_reservation() {
        let p = crate::ir::compile_scalar("limits.nx", "fn main(){mock_open(1);}").unwrap();
        let mut m = machine();
        m.log.push_stdout("x".repeat(65_499)).unwrap();
        m.run(p.functions().function("main").unwrap()).unwrap();
        assert_eq!(m.fault.as_ref().map(|d| d.code.as_str()), Some("R0002"));
        assert!(m.log.events().is_empty());
        assert_eq!(m.log.reserved_bytes(), 0);
        assert_eq!(m.cells, 0);
    }
    #[test]
    fn rejected_close_reservation_restores_new_resource_cell() {
        let p = crate::ir::compile_scalar("limits.nx", "fn main(){mock_open(1);}").unwrap();
        let mut m = machine();
        m.log.push_stdout("x".repeat(65_529)).unwrap();
        m.run(p.functions().function("main").unwrap()).unwrap();
        assert_eq!(m.fault.as_ref().map(|d| d.code.as_str()), Some("R0002"));
        assert!(m.log.events().is_empty());
        assert_eq!(m.log.reserved_bytes(), 0);
        assert_eq!(m.cells, 0);
    }
}
