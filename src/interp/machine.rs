use std::sync::Arc;

use super::mock_env::{MockEnvironment, Reply};
use super::observation::Log;
use crate::diagnostic::{
    self, DataValue, Diagnostic, Namespace, Operation, RelatedRole, RelatedSpan, Severity, Stage,
};
use crate::ir::model::{
    Binary, Function, Intrinsic, MatchLiteral, Op, ParameterMode, Program, TempId, Ty,
};
use crate::ir::work::{WorkEvent, WorkPlanner};
use crate::union::{UnionKind, VariantTag};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Outcome {
    Unit,
    Ok,
    Err(i64),
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
#[derive(Clone, Copy, Debug)]
pub struct ExecutionLimits {
    profile_id: &'static str,
    work: u64,
    frames: u64,
    cells: u64,
    observations: u64,
}
impl ExecutionLimits {
    const NORMAL: Self = Self {
        profile_id: diagnostic::PROFILE_ID,
        work: 1_000_000,
        frames: 256,
        cells: 65_536,
        observations: 65_536,
    };
}
#[derive(Clone, Copy)]
pub enum ReducedExecutionProfile {
    Normal,
    JNormal,
    Types32,
    JTypes32,
    FlowFunction32,
    JFlowFunction32,
    FlowCompilation32,
    JFlowCompilation32,
    Effects32,
    JEffects32,
    IrItems32,
    JIrItems32,
    Frames8,
    JFrames8,
    Cells64,
    JCells64,
    Observations64,
    JObservations64,
}
impl ReducedExecutionProfile {
    pub fn from_id(id: &str) -> Option<Self> {
        Some(match id {
            "f0-0002-b-v2" => Self::Normal,
            "f0-0002-b-test-types-32-v2" => Self::Types32,
            "f0-0002-b-test-flow-32-v2" => Self::FlowFunction32,
            "f0-0002-b-test-flow-global-32-v2" => Self::FlowCompilation32,
            "f0-0002-b-test-effects-32-v2" => Self::Effects32,
            "f0-0002-b-test-ir-32-v2" => Self::IrItems32,
            "f0-0002-b-test-frames-8-v2" => Self::Frames8,
            "f0-0002-b-test-cells-64-v2" => Self::Cells64,
            "f0-0002-b-test-observations-64-v2" => Self::Observations64,
            "f0-0003-b-v1" => Self::JNormal,
            "f0-0003-b-test-types-32-v1" => Self::JTypes32,
            "f0-0003-b-test-flow-32-v1" => Self::JFlowFunction32,
            "f0-0003-b-test-flow-global-32-v1" => Self::JFlowCompilation32,
            "f0-0003-b-test-effects-32-v1" => Self::JEffects32,
            "f0-0003-b-test-ir-32-v1" => Self::JIrItems32,
            "f0-0003-b-test-frames-8-v1" => Self::JFrames8,
            "f0-0003-b-test-cells-64-v1" => Self::JCells64,
            "f0-0003-b-test-observations-64-v1" => Self::JObservations64,
            _ => return None,
        })
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
enum Value {
    Int(i64),
    Bool(bool),
    Unit,
    Array(Arc<Vec<Value>>, u64),
    Union {
        kind: UnionKind,
        tag: VariantTag,
        payload: Option<Box<Value>>,
    },
    Resource {
        id: u64,
        label: i64,
        blocking: bool,
    },
}
impl Value {
    fn at_variant_path(&self, path: &[VariantTag]) -> Option<&Self> {
        let mut value = self;
        for tag in path {
            let Self::Union {
                tag: actual,
                payload: Some(next),
                ..
            } = value
            else {
                return None;
            };
            if actual != tag {
                return None;
            }
            value = next;
        }
        Some(value)
    }

    fn at_variant_path_mut(&mut self, path: &[VariantTag]) -> Option<&mut Self> {
        let mut value = self;
        for tag in path {
            let Self::Union {
                tag: actual,
                payload: Some(next),
                ..
            } = value
            else {
                return None;
            };
            if actual != tag {
                return None;
            }
            value = next;
        }
        Some(value)
    }
    fn owns_resource(&self) -> bool {
        let mut node = self;
        while let Self::Union {
            payload: Some(payload),
            ..
        } = node
        {
            node = payload;
        }
        matches!(node, Self::Resource { .. })
    }
    fn deep_copy(&self) -> Self {
        match self {
            Self::Array(items, cells) => Self::Array(
                Arc::new(items.iter().map(Self::deep_copy).collect()),
                *cells,
            ),
            Self::Union { kind, tag, payload } => Self::Union {
                kind: *kind,
                tag: *tag,
                payload: payload.as_ref().map(|value| Box::new(value.deep_copy())),
            },
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
            Self::Union { kind, tag, payload } => {
                let name = kind.variant_name(*tag).unwrap_or("<invalid-variant>");
                payload.as_ref().map_or_else(
                    || name.to_owned(),
                    |value| format!("{name}({})", value.text()),
                )
            }
            Self::Resource { .. } => "<resource>".into(),
        }
    }
    fn has_ty(&self, ty: &Ty) -> bool {
        match (self, ty) {
            (Self::Int(_), Ty::I64) | (Self::Bool(_), Ty::Bool) | (Self::Unit, Ty::Unit) => true,
            (Self::Array(xs, _), Ty::Array(element)) => xs.iter().all(|x| x.has_ty(element)),
            (Self::Union { kind, tag, payload }, ty) if ty.union_descriptor().is_some() => {
                let descriptor = ty.union_descriptor().unwrap();
                *kind == descriptor.kind
                    && descriptor.variant(*tag).is_some_and(|variant| {
                        match (&variant.payload, payload) {
                            (Some(expected), Some(value)) => value.has_ty(expected),
                            (None, None) => true,
                            _ => false,
                        }
                    })
            }
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
            Self::Union { payload, .. } => 1 + payload.as_ref().map_or(0, |value| value.cells()),
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
fn equal_pairs(left: &Value, right: &Value) -> u64 {
    match (left, right) {
        (Value::Array(a, _), Value::Array(b, _)) => {
            let mut work = 2u64;
            if a.len() != b.len() {
                return work;
            }
            for (x, y) in a.iter().zip(b.iter()) {
                work = work.saturating_add(equal_pairs(x, y));
                if x != y {
                    break;
                }
            }
            work
        }
        (
            Value::Union {
                kind: ak,
                tag: a,
                payload: x,
            },
            Value::Union {
                kind: bk,
                tag: b,
                payload: y,
            },
        ) => {
            if ak != bk || a != b {
                2
            } else {
                2u64.saturating_add(
                    x.as_ref()
                        .zip(y.as_ref())
                        .map_or(0, |(x, y)| equal_pairs(x, y)),
                )
            }
        }
        _ => 2,
    }
}
enum TempValue {
    Own(Value),
    Borrow(crate::ir::model::LocalId),
    BorrowIndex(crate::ir::model::LocalId, Vec<i64>),
    ReadScalar(Value),
    Exclusive(crate::ir::model::LocalId, Vec<i64>),
}
type EditAlias = (crate::ir::model::LocalId, Vec<i64>, Vec<VariantTag>);
impl TempValue {
    fn owned_cells(&self) -> u64 {
        match self {
            Self::Own(v) => v.cells(),
            Self::Borrow(_)
            | Self::BorrowIndex(_, _)
            | Self::ReadScalar(_)
            | Self::Exclusive(_, _) => 0,
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
            Self::ReadScalar(value) => Ok(value),
            Self::Exclusive(_, _) => Err("exclusive loan used as value"),
        }
    }
    fn into_owned(self) -> Result<Value, &'static str> {
        match self {
            Self::Own(v) => Ok(v),
            Self::Borrow(_)
            | Self::BorrowIndex(_, _)
            | Self::ReadScalar(_)
            | Self::Exclusive(_, _) => Err("borrowed value used as owner"),
        }
    }
}
struct FrameReturn {
    value: Option<Value>,
    inout: Vec<Option<Value>>,
}

enum MockReplyValue {
    Read(Reply<Vec<Reply<i64>>>),
    Unit(Reply<()>),
    Scalar(Reply<i64>),
}
impl MockReplyValue {
    fn into_value(self) -> Value {
        let (ok, payload) = match self {
            Self::Read(Ok(records)) => {
                let values: Vec<_> = records
                    .into_iter()
                    .map(|record| match record {
                        Ok(n) => Value::Union {
                            kind: UnionKind::Result,
                            tag: VariantTag(0),
                            payload: Some(Box::new(Value::Int(n))),
                        },
                        Err(n) => Value::Union {
                            kind: UnionKind::Result,
                            tag: VariantTag(1),
                            payload: Some(Box::new(Value::Int(n))),
                        },
                    })
                    .collect();
                let cells = 1 + (values.len() as u64) * 2;
                (true, Value::Array(Arc::new(values), cells))
            }
            Self::Read(Err(n)) | Self::Scalar(Err(n)) => (false, Value::Int(n)),
            Self::Scalar(Ok(n)) => (true, Value::Int(n)),
            Self::Unit(Ok(())) => (true, Value::Unit),
            Self::Unit(Err(n)) => (false, Value::Int(n)),
        };
        Value::Union {
            kind: UnionKind::Result,
            tag: VariantTag(u8::from(!ok)),
            payload: Some(Box::new(payload)),
        }
    }
}

pub fn run_scalar(program: &Program, mock_env: MockEnvironment) -> Execution {
    // Source programs arrive through compile_scalar, which verifies the IR.
    // The worker failure branches describe host failure, not a source form.
    // unicode_sources_never_reach_internal_failure and the source/IR model
    // tests exercise the accepted path; forged-IR verifier tests cover the
    // rejection side of the boundary.
    run_with_execution_limits(program, mock_env, ExecutionLimits::NORMAL)
}
pub fn run_with_limits(
    program: &Program,
    mock_env: MockEnvironment,
    profile: ReducedExecutionProfile,
) -> Execution {
    let limits = match profile {
        ReducedExecutionProfile::Normal => ExecutionLimits::NORMAL,
        ReducedExecutionProfile::JNormal => ExecutionLimits {
            profile_id: "f0-0003-b-v1",
            ..ExecutionLimits::NORMAL
        },
        ReducedExecutionProfile::Types32 => ExecutionLimits {
            profile_id: "f0-0002-b-test-types-32-v2",
            ..ExecutionLimits::NORMAL
        },
        ReducedExecutionProfile::JTypes32 => ExecutionLimits {
            profile_id: "f0-0003-b-test-types-32-v1",
            ..ExecutionLimits::NORMAL
        },
        ReducedExecutionProfile::FlowFunction32 => ExecutionLimits {
            profile_id: "f0-0002-b-test-flow-32-v2",
            ..ExecutionLimits::NORMAL
        },
        ReducedExecutionProfile::JFlowFunction32 => ExecutionLimits {
            profile_id: "f0-0003-b-test-flow-32-v1",
            ..ExecutionLimits::NORMAL
        },
        ReducedExecutionProfile::FlowCompilation32 => ExecutionLimits {
            profile_id: "f0-0002-b-test-flow-global-32-v2",
            ..ExecutionLimits::NORMAL
        },
        ReducedExecutionProfile::JFlowCompilation32 => ExecutionLimits {
            profile_id: "f0-0003-b-test-flow-global-32-v1",
            ..ExecutionLimits::NORMAL
        },
        ReducedExecutionProfile::Effects32 => ExecutionLimits {
            profile_id: "f0-0002-b-test-effects-32-v2",
            ..ExecutionLimits::NORMAL
        },
        ReducedExecutionProfile::JEffects32 => ExecutionLimits {
            profile_id: "f0-0003-b-test-effects-32-v1",
            ..ExecutionLimits::NORMAL
        },
        ReducedExecutionProfile::IrItems32 => ExecutionLimits {
            profile_id: "f0-0002-b-test-ir-32-v2",
            ..ExecutionLimits::NORMAL
        },
        ReducedExecutionProfile::JIrItems32 => ExecutionLimits {
            profile_id: "f0-0003-b-test-ir-32-v1",
            ..ExecutionLimits::NORMAL
        },
        ReducedExecutionProfile::Frames8 => ExecutionLimits {
            profile_id: "f0-0002-b-test-frames-8-v2",
            frames: 8,
            ..ExecutionLimits::NORMAL
        },
        ReducedExecutionProfile::JFrames8 => ExecutionLimits {
            profile_id: "f0-0003-b-test-frames-8-v1",
            frames: 8,
            ..ExecutionLimits::NORMAL
        },
        ReducedExecutionProfile::Cells64 => ExecutionLimits {
            profile_id: "f0-0002-b-test-cells-64-v2",
            cells: 64,
            ..ExecutionLimits::NORMAL
        },
        ReducedExecutionProfile::JCells64 => ExecutionLimits {
            profile_id: "f0-0003-b-test-cells-64-v1",
            cells: 64,
            ..ExecutionLimits::NORMAL
        },
        ReducedExecutionProfile::Observations64 => ExecutionLimits {
            profile_id: "f0-0002-b-test-observations-64-v2",
            observations: 64,
            ..ExecutionLimits::NORMAL
        },
        ReducedExecutionProfile::JObservations64 => ExecutionLimits {
            profile_id: "f0-0003-b-test-observations-64-v1",
            observations: 64,
            ..ExecutionLimits::NORMAL
        },
    };
    run_with_execution_limits(program, mock_env, limits)
}
fn run_with_execution_limits(
    program: &Program,
    mock_env: MockEnvironment,
    limits: ExecutionLimits,
) -> Execution {
    std::thread::scope(|scope| {
        match std::thread::Builder::new()
            .stack_size(16 * 1024 * 1024)
            .spawn_scoped(scope, || run_verified(program, mock_env, limits))
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
fn run_verified(
    program: &Program,
    mock_env: MockEnvironment,
    limits: ExecutionLimits,
) -> Execution {
    let mut state = Machine {
        log: Log::with_limit(limits.observations),
        limits,
        work: 0,
        cells: 0,
        fault: None,
        frames: 0,
        next_resource: 0,
        mock_env,
        #[cfg(test)]
        disposals: 0,
    };
    // Every internal Result below is guarded by this verifier: identities,
    // modes, index paths and types, call slots, temporary ownership, jumps,
    // output types and return shape are checked before interpretation.
    // Verified source executions are covered by generated_scalar_programs_
    // match_independent_decimal_model, generated_calls_resources_and_early_
    // returns_match_independent_reference, generated_array_copies_and_writes_
    // match_independent_matrix_model, and the directed prefix/Unicode tests.
    let result = crate::ir::verify::verify_program(program)
        .and_then(|()| program.functions.function("Main").ok_or("missing main"));
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
        Ok(FrameReturn {
            value:
                Some(Value::Union {
                    kind: UnionKind::Result,
                    tag: VariantTag(0),
                    payload: Some(payload),
                }),
            ..
        }) if *payload == Value::Unit => {
            state.cells = state.cells.saturating_sub(2);
            Outcome::Ok
        }
        Ok(FrameReturn {
            value:
                Some(Value::Union {
                    kind: UnionKind::Result,
                    tag: VariantTag(1),
                    payload: Some(payload),
                }),
            ..
        }) => {
            if let Value::Int(error) = *payload {
                state.cells = state.cells.saturating_sub(2);
                Outcome::Err(error)
            } else {
                Outcome::InternalError("main error type")
            }
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
    limits: ExecutionLimits,
    work: u64,
    cells: u64,
    fault: Option<Box<Diagnostic>>,
    frames: u64,
    next_resource: u64,
    mock_env: MockEnvironment,
    #[cfg(test)]
    disposals: u64,
}
impl Machine {
    fn mock_call(
        &mut self,
        intrinsic: Intrinsic,
        values: &[TempValue],
        locals: &[Option<Value>],
        span: &diagnostic::Span,
    ) -> Result<Option<Value>, &'static str> {
        let (reply, event, stdout, cells, work) = match intrinsic {
            Intrinsic::MockReadRecords => {
                let Value::Resource { id, .. } = values[0].as_value(locals)? else {
                    return Err("mock read resource type");
                };
                let reply = self
                    .mock_env
                    .read_records
                    .front()
                    .cloned()
                    .unwrap_or(Err(-9000));
                let cells = match &reply {
                    Ok(records) => 2u64.saturating_add((records.len() as u64).saturating_mul(2)),
                    Err(_) => 2,
                };
                let work = match &reply {
                    Ok(records) => 2u64.saturating_add((records.len() as u64).saturating_mul(2)),
                    Err(_) => 2,
                };
                let event = match &reply {
                    Ok(records) => format!("read_records:{id}:ok:{}", records.len()),
                    Err(error) => format!("read_records:{id}:err:{error}"),
                };
                (MockReplyValue::Read(reply), event, None, cells, work)
            }
            Intrinsic::MockWriteI64 => {
                let Value::Int(number) = values[0].as_value(locals)? else {
                    return Err("mock write value type");
                };
                let reply = self
                    .mock_env
                    .write_i64
                    .front()
                    .cloned()
                    .unwrap_or(Err(-9000));
                let event = match &reply {
                    Ok(()) => format!("write_i64:{number}:ok"),
                    Err(error) => format!("write_i64:{number}:err:{error}"),
                };
                let stdout = reply.as_ref().ok().map(|_| number.to_string());
                let work = 2 + u64::from(stdout.is_some());
                (MockReplyValue::Unit(reply), event, stdout, 2, work)
            }
            Intrinsic::MockRequest => {
                let reply = self.mock_env.request.front().cloned().unwrap_or(Err(-9000));
                let event = match &reply {
                    Ok(value) => format!("request:ok:{value}"),
                    Err(error) => format!("request:err:{error}"),
                };
                (MockReplyValue::Scalar(reply), event, None, 2, 2)
            }
            Intrinsic::MockReply => {
                let Value::Int(status) = values[0].as_value(locals)? else {
                    return Err("mock reply status type");
                };
                let reply = self.mock_env.reply.front().cloned().unwrap_or(Err(-9000));
                let event = match &reply {
                    Ok(()) => format!("reply:{status}:ok"),
                    Err(error) => format!("reply:{status}:err:{error}"),
                };
                (MockReplyValue::Unit(reply), event, None, 2, 2)
            }
            _ => return Err("not a mock io intrinsic"),
        };
        if !self.charge_work(work, span) || !self.reserve_cells(cells, span) {
            return Ok(None);
        }
        let mut batch = vec![(
            super::observation::Channel::Event,
            event,
            "mock-io:".to_owned(),
        )];
        if let Some(line) = stdout {
            batch.push((super::observation::Channel::Stdout, line, String::new()));
        }
        if let Err(observed) = self.log.push_batch(batch) {
            self.cells = self.cells.saturating_sub(cells);
            self.observation_fault(span, observed);
            return Ok(None);
        }
        match intrinsic {
            Intrinsic::MockReadRecords => {
                self.mock_env.read_records.pop_front();
            }
            Intrinsic::MockWriteI64 => {
                self.mock_env.write_i64.pop_front();
            }
            Intrinsic::MockRequest => {
                self.mock_env.request.pop_front();
            }
            Intrinsic::MockReply => {
                self.mock_env.reply.pop_front();
            }
            _ => unreachable!(),
        }
        Ok(Some(reply.into_value()))
    }

    fn final_unit_context(&mut self, context: &diagnostic::Span) {
        if let Some(fault) = self.fault.as_mut() {
            fault.related_spans = vec![RelatedSpan {
                role: RelatedRole::Context,
                span: context.clone(),
            }];
        }
    }
    fn dispose(&mut self, value: Value) -> Result<(), &'static str> {
        #[cfg(test)]
        {
            self.disposals += 1;
        }
        self.cells = self.cells.saturating_sub(value.cells());
        let mut active = value;
        while let Value::Union {
            payload: Some(payload),
            ..
        } = active
        {
            active = *payload;
        }
        if let Value::Resource { id, label, .. } = active {
            self.log
                .emit_reserved_event(format!("close:{id}:{label}"), "mock-resource:")?;
        }
        Ok(())
    }
    fn dispose_intrinsic_resources(
        &mut self,
        values: &mut Vec<TempValue>,
    ) -> Result<(), &'static str> {
        for value in values.drain(..).rev() {
            if matches!(&value, TempValue::Own(owned) if owned.owns_resource()) {
                self.dispose(value.into_owned()?)?;
            }
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
                ("limit", DataValue::UInt(self.limits.observations)),
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
            profile_id: self.limits.profile_id.into(),
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
    fn index_fault(&mut self, site: &crate::ir::model::IndexSite, failed: usize) {
        let selector = site.selectors.get(failed).unwrap_or(&site.context);
        self.fault("R0102", selector, vec![]);
        if let Some(diagnostic) = self.fault.as_mut() {
            diagnostic.primary_span = Some(selector.clone());
            diagnostic.related_spans = vec![diagnostic::RelatedSpan {
                role: diagnostic::RelatedRole::Context,
                span: site.context.clone(),
            }];
        }
    }
    fn charge_work(&mut self, n: u64, span: &diagnostic::Span) -> bool {
        let proposed = self.work.saturating_add(n);
        if proposed > self.limits.work {
            self.work = self.limits.work;
            self.fault(
                "R0001",
                span,
                vec![
                    ("limit", DataValue::UInt(self.limits.work)),
                    (
                        "observed",
                        DataValue::UInt(self.limits.work.saturating_add(1)),
                    ),
                    ("work_consumed", DataValue::UInt(self.limits.work)),
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
        if proposed > self.limits.cells {
            self.fault(
                "R0002",
                span,
                vec![
                    (
                        "runtime_budget_kind",
                        DataValue::String("live_value_cells".into()),
                    ),
                    ("limit", DataValue::UInt(self.limits.cells)),
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
        if !self.charge_work(WorkPlanner::units(WorkEvent::Entry), call_span) {
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
        if self.frames >= self.limits.frames {
            self.fault(
                "R0002",
                call_span,
                vec![
                    (
                        "runtime_budget_kind",
                        DataValue::String("call_frames".into()),
                    ),
                    ("limit", DataValue::UInt(self.limits.frames)),
                    (
                        "observed",
                        DataValue::UInt(self.limits.frames.saturating_add(1)),
                    ),
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
        let mut edit_aliases: Vec<Option<EditAlias>> = vec![None; function.locals.len()];
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
                let implicit_final_unit = matches!(ins.op, Op::ConstUnit)
                    && ins.charges.last().is_some_and(|charge| {
                        charge.origin.start > ins.span.start && charge.origin.end == ins.span.end
                    });
                for charge in &ins.charges {
                    if !self.charge_work(charge.ticks, &charge.origin) {
                        if implicit_final_unit {
                            self.final_unit_context(&ins.span);
                        }
                        break;
                    }
                }
                if self.fault.is_some() {
                    if implicit_final_unit {
                        self.final_unit_context(&ins.span);
                    }
                    break;
                }
                if let Op::BorrowValue(id) = &ins.op {
                    if let Some(value @ (Value::Int(_) | Value::Bool(_) | Value::Unit)) =
                        locals.get(id.0).ok_or("borrow local index")?.as_ref()
                    {
                        Self::install_temp(
                            &mut temps,
                            ins.output.ok_or("borrow output identity")?,
                            TempValue::ReadScalar(value.clone()),
                        )?;
                        continue;
                    }
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
                    site,
                    permission,
                    ..
                } = &ins.op
                {
                    let mut path = Vec::with_capacity(indexes.len());
                    for id in indexes {
                        let taken = Self::take(&mut temps, *id)?;
                        let Value::Int(index) = taken.as_value(&locals)? else {
                            return Err("loan index type");
                        };
                        if matches!(taken, TempValue::Own(_)) {
                            self.cells = self.cells.saturating_sub(1);
                        }
                        path.push(*index);
                    }
                    let mut selected = locals
                        .get(root.0)
                        .ok_or("loan root")?
                        .as_ref()
                        .ok_or("loan unavailable root")?;
                    let mut valid = true;
                    for (position, index) in path.iter().enumerate() {
                        if let Some(next) = selected.element(*index) {
                            selected = next;
                        } else {
                            valid = false;
                            self.index_fault(site, position);
                            break;
                        }
                    }
                    if !valid {
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
                if let Op::BorrowIndex {
                    local,
                    indexes,
                    site,
                } = &ins.op
                {
                    let mut path = Vec::with_capacity(indexes.len());
                    for id in indexes {
                        let taken = Self::take(&mut temps, *id)?;
                        let Value::Int(index) = taken.as_value(&locals)? else {
                            return Err("borrow index type");
                        };
                        if matches!(taken, TempValue::Own(_)) {
                            self.cells = self.cells.saturating_sub(1);
                        }
                        path.push(*index);
                    }
                    let mut value = locals
                        .get(local.0)
                        .ok_or("borrow index local")?
                        .as_ref()
                        .ok_or("borrowed empty local")?;
                    let mut valid = true;
                    for (position, index) in path.iter().enumerate() {
                        if let Some(next) = value.element(*index) {
                            value = next;
                        } else {
                            valid = false;
                            self.index_fault(site, position);
                            break;
                        }
                    }
                    if !valid {
                        break;
                    }
                    let borrowed_value =
                        if matches!(value, Value::Int(_) | Value::Bool(_) | Value::Unit) {
                            TempValue::ReadScalar(value.clone())
                        } else {
                            TempValue::BorrowIndex(*local, path)
                        };
                    Self::install_temp(
                        &mut temps,
                        ins.output.ok_or("borrow index output identity")?,
                        borrowed_value,
                    )?;
                    continue;
                }
                if let Op::ProjectPayload { local, path } = &ins.op {
                    let source = locals
                        .get(local.0)
                        .and_then(Option::as_ref)
                        .ok_or("projection local")?;
                    let value = source
                        .at_variant_path(&path[..path.len().saturating_sub(1)])
                        .ok_or("projection path")?;
                    let Value::Union {
                        tag,
                        payload: Some(payload),
                        ..
                    } = value
                    else {
                        return Err("projection variant");
                    };
                    if Some(tag) != path.last() {
                        return Err("projection tag");
                    }
                    Self::install_temp(
                        &mut temps,
                        ins.output.ok_or("projection output")?,
                        TempValue::ReadScalar((**payload).clone()),
                    )?;
                    continue;
                }
                if let Op::BindView { local, value, hold } = &ins.op {
                    let borrowed = if *hold {
                        temps
                            .get(value.0)
                            .and_then(Option::as_ref)
                            .ok_or("view loan")?
                    } else {
                        // The arm-local projection can be released immediately.
                        // The selection loan remains live until the match joins.
                        let taken = Self::take(&mut temps, *value)?;
                        let selected = taken.as_value(&locals)?.clone();
                        let slot = locals.get_mut(local.0).ok_or("view local")?;
                        if slot.is_some() {
                            return Err("view already bound");
                        }
                        *slot = Some(selected);
                        borrowed_locals[local.0] = true;
                        continue;
                    };
                    let selected = borrowed.as_value(&locals)?.clone();
                    let slot = locals.get_mut(local.0).ok_or("view local")?;
                    if slot.is_some() {
                        return Err("view already bound");
                    }
                    *slot = Some(selected);
                    borrowed_locals[local.0] = true;
                    continue;
                }
                if let Op::BindEdit {
                    local,
                    root,
                    loan,
                    path,
                    ..
                } = &ins.op
                {
                    let indexes = if let Some(loan) = loan {
                        let Some(TempValue::Exclusive(source, indexes)) =
                            temps.get(loan.0).and_then(Option::as_ref)
                        else {
                            return Err("edit loan");
                        };
                        if source != root {
                            return Err("edit loan source");
                        }
                        indexes.clone()
                    } else {
                        Vec::new()
                    };
                    let source = locals
                        .get(root.0)
                        .and_then(Option::as_ref)
                        .ok_or("edit root")?;
                    let selected = get_path(source, &indexes)
                        .ok_or("edit index")?
                        .at_variant_path(path)
                        .ok_or("edit path")?
                        .deep_copy();
                    let slot = locals.get_mut(local.0).ok_or("edit local")?;
                    if slot.is_some() {
                        return Err("edit already bound");
                    }
                    *slot = Some(selected);
                    borrowed_locals[local.0] = true;
                    edit_aliases[local.0] = Some((*root, indexes, path.clone()));
                    continue;
                }
                let value = match &ins.op {
                    Op::BorrowValue(_) => return Err("borrow dispatch"),
                    Op::BorrowIndex { .. } => return Err("borrow index dispatch"),
                    Op::BeginLoan { .. } => return Err("loan dispatch"),
                    Op::EndLoan(_) => None,
                    Op::TestVariant { local, path, tag } => {
                        let found = locals.get(local.0).and_then(Option::as_ref)
                            .and_then(|value| value.at_variant_path(path))
                            .is_some_and(|value| matches!(value, Value::Union { tag: actual, .. } if actual == tag));
                        if !self.charge_work(1, &ins.span) || !self.cell(&ins.span) {
                            break;
                        }
                        Some(Value::Bool(found))
                    }
                    Op::TestLiteral {
                        local,
                        path,
                        literal,
                    } => {
                        let found = locals
                            .get(local.0)
                            .and_then(Option::as_ref)
                            .and_then(|value| value.at_variant_path(path))
                            .is_some_and(|value| match (value, literal) {
                                (Value::Int(actual), MatchLiteral::I64(expected)) => {
                                    actual == expected
                                }
                                (Value::Bool(actual), MatchLiteral::Bool(expected)) => {
                                    actual == expected
                                }
                                _ => false,
                            });
                        if !self.charge_work(1, &ins.span) || !self.cell(&ins.span) {
                            break;
                        }
                        Some(Value::Bool(found))
                    }
                    Op::ExtractPayload { local, path } => {
                        let source = locals
                            .get_mut(local.0)
                            .and_then(Option::as_mut)
                            .ok_or("extract local")?;
                        let Some((&last, prefix)) = path.split_last() else {
                            return Err("extract path");
                        };
                        let value = source.at_variant_path_mut(prefix).ok_or("extract prefix")?;
                        let Value::Union { tag, payload, .. } = value else {
                            return Err("extract variant");
                        };
                        if *tag != last {
                            return Err("extract tag");
                        }
                        Some(*payload.take().ok_or("extract missing payload")?)
                    }
                    Op::ProjectPayload { .. } | Op::BindView { .. } | Op::BindEdit { .. } => {
                        return Err("projection dispatch");
                    }
                    Op::ConstI64(n) => {
                        if !self.charge_work(1, &ins.span) || !self.cell(&ins.span) {
                            break;
                        }
                        Some(Value::Int(*n))
                    }
                    Op::ConstBool(b) => {
                        if !self.charge_work(1, &ins.span) || !self.cell(&ins.span) {
                            break;
                        }
                        Some(Value::Bool(*b))
                    }
                    Op::ConstUnit => {
                        let implicit_final = ins.charges.last().is_some_and(|charge| {
                            charge.origin.start > ins.span.start
                                && charge.origin.end == ins.span.end
                        });
                        let cell_span = if implicit_final {
                            &ins.charges.last().unwrap().origin
                        } else {
                            &ins.span
                        };
                        if (!implicit_final && !self.charge_work(1, cell_span))
                            || !self.cell(cell_span)
                        {
                            if implicit_final {
                                self.final_unit_context(&ins.span);
                            }
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
                        if !self.charge_work(v.cells(), &ins.span)
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
                        if !self.charge_work(1, &ins.span) || !self.cell(&ins.span) {
                            self.cells = self.cells.saturating_sub(total - 1);
                            break;
                        }
                        Some(Value::Array(Arc::new(xs), total))
                    }
                    Op::BuildVariant { tag, payload } => {
                        if !self.charge_work(1, &ins.span)
                            || !self.charge_work(1, &ins.span)
                            || !self.cell(&ins.span)
                        {
                            break;
                        }
                        let kind = ins
                            .ty
                            .as_ref()
                            .and_then(Ty::union_descriptor)
                            .ok_or("union type")?
                            .kind;
                        let payload = payload
                            .map(|payload| {
                                Self::take(&mut temps, payload)?.into_owned().map(Box::new)
                            })
                            .transpose()?;
                        Some(Value::Union {
                            kind,
                            tag: *tag,
                            payload,
                        })
                    }
                    Op::BranchVariant { value, .. } => {
                        let source = Self::take(&mut temps, *value)?.into_owned()?;
                        let Value::Union {
                            kind,
                            tag,
                            payload: Some(payload),
                        } = source
                        else {
                            return Err("propagation value type");
                        };
                        if kind.variant_name(tag).is_none() {
                            return Err("propagation variant tag");
                        }
                        if tag
                            == if kind == UnionKind::Result {
                                VariantTag(0)
                            } else {
                                VariantTag(1)
                            }
                        {
                            self.cells = self.cells.saturating_sub(1);
                            Some(*payload)
                        } else {
                            returned = Some(Value::Union {
                                kind,
                                tag,
                                payload: Some(payload),
                            });
                            break 'cfg;
                        }
                    }
                    Op::ReadIndex {
                        local,
                        indexes,
                        site,
                    } => {
                        let mut path = Vec::with_capacity(indexes.len());
                        for id in indexes {
                            let taken = Self::take(&mut temps, *id)?;
                            let Value::Int(n) = taken.as_value(&locals)? else {
                                return Err("index value type");
                            };
                            if matches!(taken, TempValue::Own(_)) {
                                self.cells = self.cells.saturating_sub(1);
                            }
                            path.push(*n);
                        }
                        let mut selected = locals
                            .get(local.0)
                            .ok_or("read index local")?
                            .as_ref()
                            .ok_or("empty local")?;
                        let mut valid = true;
                        for (position, index) in path.into_iter().enumerate() {
                            if let Some(next) = selected.element(index) {
                                selected = next;
                            } else {
                                valid = false;
                                self.index_fault(site, position);
                                break;
                            }
                        }
                        if !valid {
                            break;
                        }
                        if !self.charge_work(selected.cells(), &ins.span)
                            || !self.reserve_cells(selected.cells(), &ins.span)
                        {
                            break;
                        }
                        Some(selected.deep_copy())
                    }
                    Op::CheckIndex {
                        local,
                        indexes,
                        site,
                    } => {
                        let mut selected = locals
                            .get(local.0)
                            .ok_or("check index local")?
                            .as_ref()
                            .ok_or("empty local")?;
                        let mut valid = true;
                        for (position, id) in indexes.iter().enumerate() {
                            let index = temps
                                .get(id.0)
                                .and_then(Option::as_ref)
                                .ok_or("check index temporary")?;
                            let Value::Int(n) = index.as_value(&locals)? else {
                                return Err("check index type");
                            };
                            if let Some(next) = selected.element(*n) {
                                selected = next;
                            } else {
                                valid = false;
                                self.index_fault(site, position);
                                break;
                            }
                        }
                        if !valid {
                            break;
                        }
                        None
                    }
                    Op::IndexValue { base, index, site } => {
                        let base = Self::take(&mut temps, *base)?.into_owned()?;
                        let taken = Self::take(&mut temps, *index)?;
                        let Value::Int(n) = taken.as_value(&locals)? else {
                            return Err("index value type");
                        };
                        let size = base.cells();
                        let own = u64::from(matches!(taken, TempValue::Own(_)));
                        let Some(selected) = base.element(*n) else {
                            self.cells = self.cells.saturating_sub(size + own);
                            self.index_fault(site, 0);
                            break;
                        };
                        let selected = selected.clone();
                        self.cells = self.cells.saturating_sub(size + own - selected.cells());
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
                                if !self.charge_work(2, &ins.span) {
                                    break;
                                }
                                Value::Bool(a < b)
                            }
                            (Binary::And, Value::Bool(a), Value::Bool(b)) => Value::Bool(*a && *b),
                            (Binary::Equal, a, b) => {
                                if !self.charge_work(equal_pairs(a, b), &ins.span) {
                                    break;
                                }
                                Value::Bool(a == b)
                            }
                            _ => return Err("verified binary domain"),
                        };
                        if !self.charge_work(1, &ins.span) || !self.cell(&ins.span) {
                            break;
                        }
                        Some(v)
                    }
                    Op::WritePlace { local, value } => {
                        let v = Self::take(&mut temps, *value)?.into_owned()?;
                        if let Some((root, indexes, path)) =
                            edit_aliases.get(local.0).and_then(Clone::clone)
                        {
                            let old = if path.is_empty() {
                                if indexes.is_empty() {
                                    locals
                                        .get_mut(root.0)
                                        .ok_or("edit root")?
                                        .replace(v.deep_copy())
                                        .ok_or("edit root empty")?
                                } else {
                                    let source = locals
                                        .get_mut(root.0)
                                        .and_then(Option::as_mut)
                                        .ok_or("edit root")?;
                                    std::mem::replace(
                                        get_mut_path(source, &indexes).ok_or("edit index")?,
                                        v.deep_copy(),
                                    )
                                }
                            } else {
                                let source = locals
                                    .get_mut(root.0)
                                    .and_then(Option::as_mut)
                                    .ok_or("edit root")?;
                                let Some((&tag, prefix)) = path.split_last() else {
                                    return Err("edit path");
                                };
                                let Value::Union {
                                    tag: actual,
                                    payload,
                                    ..
                                } = get_mut_path(source, &indexes)
                                    .ok_or("edit index")?
                                    .at_variant_path_mut(prefix)
                                    .ok_or("edit prefix")?
                                else {
                                    return Err("edit variant");
                                };
                                if *actual != tag {
                                    return Err("edit tag");
                                }
                                *payload
                                    .replace(Box::new(v.deep_copy()))
                                    .ok_or("edit payload")?
                            };
                            if !indexes.is_empty() {
                                let source = locals
                                    .get_mut(root.0)
                                    .and_then(Option::as_mut)
                                    .ok_or("edit root")?;
                                adjust_size(source, &indexes, old.cells(), v.cells());
                            }
                            *locals.get_mut(local.0).ok_or("edit local")? = Some(v);
                            self.dispose(old)?;
                        } else {
                            let dest = locals.get_mut(local.0).ok_or("write index")?;
                            if let Some(old) = dest.replace(v) {
                                self.dispose(old)?;
                            }
                        }
                        None
                    }
                    Op::WriteIndex {
                        local,
                        indexes,
                        value,
                        site,
                    } => {
                        let mut path = Vec::with_capacity(indexes.len());
                        for id in indexes {
                            let taken = Self::take(&mut temps, *id)?;
                            let Value::Int(n) = taken.as_value(&locals)? else {
                                return Err("index value type");
                            };
                            if matches!(taken, TempValue::Own(_)) {
                                self.cells = self.cells.saturating_sub(1);
                            }
                            path.push(*n);
                        }
                        let replacement = Self::take(&mut temps, *value)?.into_owned()?;
                        if let Some((source_id, source_indexes, variants)) =
                            edit_aliases.get(local.0).and_then(Clone::clone)
                        {
                            let source = locals
                                .get_mut(source_id.0)
                                .and_then(Option::as_mut)
                                .ok_or("edit index root")?;
                            let selected = get_mut_path(source, &source_indexes)
                                .ok_or("edit index source")?
                                .at_variant_path_mut(&variants)
                                .ok_or("edit index variant")?;
                            if let Some(position) = first_bad_index(selected, &path) {
                                self.cells = self.cells.saturating_sub(replacement.cells());
                                self.index_fault(site, position);
                                break;
                            }
                            let node = get_mut_path(selected, &path).ok_or("edit index target")?;
                            let previous = std::mem::replace(node, replacement);
                            let old_size = previous.cells();
                            let new_size = node.cells();
                            adjust_size(selected, &path, old_size, new_size);
                            adjust_size(source, &source_indexes, old_size, new_size);
                            let snapshot = get_path(source, &source_indexes)
                                .ok_or("edit index source")?
                                .at_variant_path(&variants)
                                .ok_or("edit index variant")?
                                .deep_copy();
                            *locals.get_mut(local.0).ok_or("edit index local")? = Some(snapshot);
                            self.dispose(previous)?;
                            continue;
                        }
                        let root = locals
                            .get_mut(local.0)
                            .ok_or("write index local")?
                            .as_mut()
                            .ok_or("empty local")?;
                        if let Some(position) = first_bad_index(root, &path) {
                            self.cells = self.cells.saturating_sub(replacement.cells());
                            self.index_fault(site, position);
                            break;
                        }
                        let Some(node) = get_mut_path(root, &path) else {
                            self.cells = self.cells.saturating_sub(replacement.cells());
                            self.index_fault(site, path.len().saturating_sub(1));
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
                        let argument_ids = args.clone();
                        let mut values = Vec::with_capacity(args.len());
                        for id in args {
                            values.push(Self::take(&mut temps, *id)?);
                            if !matches!(values.last(), Some(TempValue::Own(value)) if value.owns_resource())
                            {
                                self.cells = self.cells.saturating_sub(
                                    values.last().map_or(0, TempValue::owned_cells),
                                );
                            }
                        }
                        if !self.charge_work(1, &ins.span) {
                            self.dispose_intrinsic_resources(&mut values)?;
                            break;
                        }
                        let mut pre_reserved_output = false;
                        let mut precharged_output_work = false;
                        if *intrinsic == Intrinsic::DropData {
                            if !self.charge_work(1, &ins.span) {
                                self.dispose_intrinsic_resources(&mut values)?;
                                break;
                            }
                            precharged_output_work = true;
                        }
                        let output = match intrinsic {
                            Intrinsic::Print => {
                                let value = values[0].as_value(&locals)?;
                                if !self.charge_work(value.cells(), &ins.span) {
                                    break;
                                }
                                if !self.charge_work(1, &ins.span) || !self.cell(&ins.span) {
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
                                            ("limit", DataValue::UInt(self.limits.observations)),
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
                                if !self.charge_work(equal_pairs(left, right), &ins.span) {
                                    break;
                                }
                                if left != right {
                                    self.fault("R0201", &ins.span, vec![]);
                                    break;
                                }
                                Value::Unit
                            }
                            Intrinsic::DropData | Intrinsic::MockUnknown => Value::Unit,
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
                                if !self.charge_work(1, &ins.span) || !self.cell(&ins.span) {
                                    break;
                                }
                                pre_reserved_output = true;
                                let id = self.next_resource;
                                let close = format!("close:{id}:{label}");
                                let close_bytes = ("mock-resource:".len() + close.len() + 1) as u64;
                                let open = format!("open:{id}:{label}");
                                let open_bytes = ("mock-resource:".len() + open.len() + 1) as u64;
                                let acquisition_bytes = close_bytes + open_bytes;
                                if let Err(observed) = self.log.reserve(acquisition_bytes) {
                                    self.cells -= 1;
                                    self.observation_fault(&ins.span, observed);
                                    break;
                                }
                                self.log.release_reservation(open_bytes)?;
                                if let Err(observed) = self.log.push_event(open, "mock-resource:") {
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
                                if !self.charge_work(1, &ins.span) || !self.cell(&ins.span) {
                                    self.dispose_intrinsic_resources(&mut values)?;
                                    break;
                                }
                                pre_reserved_output = true;
                                if let Err(observed) = self.log.push_event(text, "mock-resource:") {
                                    self.cells -= 1;
                                    self.observation_fault(&ins.span, observed);
                                    self.dispose_intrinsic_resources(&mut values)?;
                                    break;
                                }
                                Value::Unit
                            }
                            Intrinsic::TryAdd => {
                                let (Value::Int(a), Value::Int(b)) =
                                    (values[0].as_value(&locals)?, values[1].as_value(&locals)?)
                                else {
                                    return Err("try_add argument type");
                                };
                                let result = match a.checked_add(*b) {
                                    Some(sum) => Value::Union {
                                        kind: UnionKind::Result,
                                        tag: VariantTag(0),
                                        payload: Some(Box::new(Value::Int(sum))),
                                    },
                                    None => Value::Union {
                                        kind: UnionKind::Result,
                                        tag: VariantTag(1),
                                        payload: Some(Box::new(Value::Int(30))),
                                    },
                                };
                                if !self.charge_work(2, &ins.span)
                                    || !self.reserve_cells(2, &ins.span)
                                {
                                    break;
                                }
                                pre_reserved_output = true;
                                result
                            }
                            Intrinsic::MockReadRecords
                            | Intrinsic::MockWriteI64
                            | Intrinsic::MockRequest
                            | Intrinsic::MockReply => {
                                let Some(result) =
                                    self.mock_call(*intrinsic, &values, &locals, &ins.span)?
                                else {
                                    if *intrinsic == Intrinsic::MockReadRecords {
                                        self.dispose_intrinsic_resources(&mut values)?;
                                    }
                                    break;
                                };
                                pre_reserved_output = true;
                                result
                            }
                        };
                        for (id, value) in argument_ids.into_iter().zip(values).rev() {
                            if matches!(intrinsic, Intrinsic::Inspect | Intrinsic::MockReadRecords)
                                && matches!(&value, TempValue::Own(owned) if owned.owns_resource())
                            {
                                temps.insert(id.0, Some(value));
                            } else if matches!(&value, TempValue::Own(owned) if owned.owns_resource())
                            {
                                self.dispose(value.into_owned()?)?;
                            }
                        }
                        if *intrinsic != Intrinsic::Print
                            && !pre_reserved_output
                            && ((!precharged_output_work && !self.charge_work(1, &ins.span))
                                || !self.cell(&ins.span))
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
                                        return Err("edit argument loan");
                                    };
                                    let value = if path.is_empty() {
                                        locals
                                            .get_mut(root.0)
                                            .ok_or("edit root")?
                                            .take()
                                            .ok_or("edit unavailable root")?
                                    } else {
                                        let root_value = locals
                                            .get_mut(root.0)
                                            .ok_or("edit root")?
                                            .as_mut()
                                            .ok_or("edit unavailable root")?;
                                        // Move the selected owner into the callee. The unit is an
                                        // inaccessible structural placeholder, not a live Nexo cell.
                                        let selected = get_mut_path(root_value, &path)
                                            .ok_or("edit index path")?;
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
                                    targets[slot].take().ok_or("edit return target")?;
                                if path.is_empty() {
                                    if locals[root.0].replace(value).is_some() {
                                        return Err("edit occupied root");
                                    }
                                } else {
                                    let root_value =
                                        locals[root.0].as_mut().ok_or("edit missing root")?;
                                    let node = get_mut_path(root_value, &path)
                                        .ok_or("edit return path")?;
                                    let previous = std::mem::replace(node, value);
                                    if previous != Value::Unit {
                                        return Err("edit placeholder replaced");
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
                        if !self.charge_work(1, &ins.span) || !self.cell(&ins.span) {
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
                        edit_aliases[id.0] = None;
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
            let charges = match &current.terminator {
                crate::ir::model::Terminator::Unreachable { charges, .. }
                | crate::ir::model::Terminator::Return { charges, .. }
                | crate::ir::model::Terminator::Jump { charges, .. }
                | crate::ir::model::Terminator::BranchBool { charges, .. } => charges,
            };
            for charge in charges {
                if !self.charge_work(charge.ticks, &charge.origin) {
                    break;
                }
            }
            if self.fault.is_some() {
                break 'cfg;
            }
            match &current.terminator {
                crate::ir::model::Terminator::Unreachable { .. } => {
                    return Err("unreachable match arm");
                }
                crate::ir::model::Terminator::Return {
                    value: Some(id), ..
                } => {
                    returned = Some(Self::take(&mut temps, *id)?.into_owned()?);
                    break 'cfg;
                }
                crate::ir::model::Terminator::Return { value: None, .. } => {
                    return Err("return terminator");
                }
                crate::ir::model::Terminator::Jump { target, .. } => {
                    current = if target.0 == 0 {
                        &function.block
                    } else {
                        function.blocks.get(target.0 - 1).ok_or("jump target")?
                    };
                }
                crate::ir::model::Terminator::BranchBool {
                    condition, yes, no, ..
                } => {
                    let value = Self::take(&mut temps, *condition)?;
                    let Value::Bool(test) = *value.as_value(&locals)? else {
                        return Err("branch condition type");
                    };
                    self.cells = self.cells.saturating_sub(value.owned_cells());
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
fn get_path<'a>(mut node: &'a Value, path: &[i64]) -> Option<&'a Value> {
    for index in path {
        node = node.element(*index)?;
    }
    Some(node)
}

fn get_mut_path<'a>(mut node: &'a mut Value, path: &[i64]) -> Option<&'a mut Value> {
    for &index in path {
        node = node.element_mut(index)?;
    }
    Some(node)
}
fn first_bad_index(mut node: &Value, path: &[i64]) -> Option<usize> {
    for (position, index) in path.iter().enumerate() {
        let Some(next) = node.element(*index) else {
            return Some(position);
        };
        node = next;
    }
    None
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
            crate::ir::model::Terminator::Unreachable { span, .. }
            | crate::ir::model::Terminator::Return { span, .. } => span.clone(),
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
            crate::ir::compile_scalar("slot.n", "fn Main() {\n    let a = [1];\n    Drop(a);\n}\n")
                .unwrap();
        let mut function = program.functions().function("Main").unwrap().clone();
        let drop_pos = function
            .block
            .instructions
            .iter()
            .position(|i| {
                matches!(
                    i.op,
                    Op::CallIntrinsic {
                        intrinsic: crate::ir::model::Intrinsic::DropData,
                        ..
                    }
                )
            })
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
                charges: vec![],
            },
        );
        let mut state = machine();
        assert_eq!(state.run(&function), Err("empty local"));
    }
    fn span() -> diagnostic::Span {
        diagnostic::Span {
            file: "limits.n".into(),
            start: 1,
            end: 2,
        }
    }
    fn machine() -> Machine {
        Machine {
            log: Log::default(),
            limits: ExecutionLimits::NORMAL,
            work: 0,
            cells: 0,
            fault: None,
            frames: 0,
            next_resource: 0,
            mock_env: MockEnvironment::empty(),
            disposals: 0,
        }
    }
    #[test]
    fn denied_frame_tick_releases_prepared_owners_in_reverse_and_preserves_borrow() {
        let program = crate::ir::compile_scalar(
            "limits.n",
            "fn Own(a: move MockResource, b: move MockResource) {}\nfn Read(a: view MockResource, b: move MockResource) {}\nfn Main() {}\n",
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
        let target = program.functions().function("Own").unwrap();
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
        let target = program.functions().function("Read").unwrap();
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
        assert!(m.charge_work(1, &span()));
        assert_eq!(m.work, 1_000_000);
        assert!(!m.charge_work(1, &span()));
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
        d.validate("limits.n", "abc").unwrap();
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
        d.validate("limits.n", "abc").unwrap();
    }
    #[test]
    fn indexed_inout_and_read_have_same_exact_cell_boundary() {
        for source in [
            "fn F(x: edit Array<I64>) {}\nfn Main() {\n    var a = [[1]];\n    F(edit a[0]);\n}\n",
            "fn F(x: view Array<I64>) {}\nfn Main() {\n    var a = [[1]];\n    F(a[0]);\n}\n",
        ] {
            let p = crate::ir::compile_scalar("limits.n", source).unwrap();
            for (preload, accepted) in [(65_531, true), (65_532, true), (65_533, false)] {
                let mut m = machine();
                m.cells = preload;
                let returned = m
                    .run_frame(
                        p.functions().function("Main").unwrap(),
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
                    fault.validate("limits.n", source).unwrap();
                }
                assert_eq!(m.cells, preload, "{source}: {preload}");
            }
        }
    }
    #[test]
    fn indexed_array_growth_uses_exact_cached_cell_count() {
        let source = "fn Main() {\n    var a = [[0]];\n    a[0] = [1, 2];\n    let b = a;\n}\n";
        let p = crate::ir::compile_scalar("growth.n", source).unwrap();
        for (preload, accepted) in [(65_526, true), (65_527, true), (65_528, false)] {
            let mut m = machine();
            m.cells = preload;
            m.run(p.functions().function("Main").unwrap()).unwrap();
            assert_eq!(m.fault.is_none(), accepted, "{preload}");
            if let Some(fault) = m.fault.as_ref() {
                assert_eq!(fault.code, "R0002");
                assert_eq!(fault.data["observed"], DataValue::UInt(65_537));
                fault.validate("growth.n", source).unwrap();
            }
            assert_eq!(m.cells, preload);
        }
    }
    #[test]
    fn rejected_print_releases_prepared_result_cell_before_cleanup() {
        let program =
            crate::ir::compile_scalar("limits.n", "fn Main() {\n    Print(0);\n}\n").unwrap();
        let mut m = machine();
        m.log.push_stdout("x".repeat(65_535)).unwrap();
        m.run(program.functions().function("Main").unwrap())
            .unwrap();
        assert_eq!(m.fault.as_ref().map(|d| d.code.as_str()), Some("R0002"));
        assert_eq!(m.cells, 0);
        assert_eq!(m.log.bytes(), 65_536);
    }
    #[test]
    fn failed_indexed_write_releases_rhs_and_root_cells() {
        let p = crate::ir::compile_scalar(
            "limits.n",
            "fn Main() {\n    var a = [[1]];\n    a[2] = [3];\n}\n",
        )
        .unwrap();
        let mut m = machine();
        m.run(p.functions().function("Main").unwrap()).unwrap();
        assert_eq!(m.fault.as_ref().map(|d| d.code.as_str()), Some("R0102"));
        assert_eq!(m.cells, 0);
    }
    #[test]
    fn indexed_edit_loan_checks_each_index_before_entering_callee() {
        let source = "fn F(x:edit I64){}fn Main(){var a=[[0]];F(edit a[0][2]);}";
        let p = crate::ir::compile_scalar("d03.n", source).unwrap();
        let mut m = machine();
        m.run(p.functions().function("Main").unwrap()).unwrap();
        let fault = m.fault.as_ref().expect("D03 runtime index fault");
        assert_eq!(fault.code, "R0102");
        // F0-D15 §8.12 assigns the precise D03 primary [51,54) to I10b.
        assert!(fault.validate("d03.n", source).is_ok());
        assert_eq!(m.cells, 0);
        assert!(m.log.observations().is_empty());
    }
    #[test]
    fn rejected_array_copy_releases_prepared_values_and_all_bindings() {
        let mut source = String::from("fn Main(){let a0=[0];");
        for k in 1..=14 {
            source.push_str(&format!("let a{k}=[a{},a{}];", k - 1, k - 1));
        }
        source.push('}');
        let p = crate::ir::compile_scalar("limits.n", &source).unwrap();
        let mut m = machine();
        m.run(p.functions().function("Main").unwrap()).unwrap();
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
        let p =
            crate::ir::compile_scalar("limits.n", "fn Main() {\n    let a = [1];\n}\n").unwrap();
        let mut m = machine();
        m.cells = 65_535;
        m.run(p.functions().function("Main").unwrap()).unwrap();
        assert_eq!(m.fault.as_ref().map(|d| d.code.as_str()), Some("R0002"));
        assert_eq!(m.cells, 65_535);
    }
    #[test]
    fn repeated_borrowed_index_releases_consumed_index_cells() {
        let source = format!("fn Main(){{let a=[[1]];{}}}", "Len(a[0]);".repeat(10_000));
        let p = crate::ir::compile_scalar("limits.n", &source).unwrap();
        let mut m = machine();
        m.run(p.functions().function("Main").unwrap()).unwrap();
        assert!(m.fault.is_none(), "{:?}", m.fault);
        assert_eq!(m.cells, 0);
        assert_eq!(m.work, 90_009);
    }
    #[test]
    fn repeated_index_value_releases_unselected_array_cells() {
        let source = format!("fn Main(){{{}}}", "Len([[1,2,3],[4]][0]);".repeat(5_000));
        let p = crate::ir::compile_scalar("limits.n", &source).unwrap();
        let mut m = machine();
        m.run(p.functions().function("Main").unwrap()).unwrap();
        assert!(m.fault.is_none(), "{:?}", m.fault);
        assert_eq!(m.cells, 0);
        assert_eq!(m.work, 110_002);
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
            crate::ir::compile_scalar("limits.n", "fn Main() {\n    Inspect(MockOpen(1));\n}\n")
                .unwrap();
        let mut m = machine();
        m.log.push_stdout("x".repeat(65_488)).unwrap();
        m.run(p.functions().function("Main").unwrap()).unwrap();
        assert_eq!(m.fault.as_ref().map(|d| d.code.as_str()), Some("R0002"));
        assert_eq!(m.log.events(), ["open:0:1", "close:0:1"]);
        assert_eq!(m.log.reserved_bytes(), 0);
        assert_eq!(m.cells, 0);
    }
    #[test]
    fn rejected_open_does_not_emit_acquisition_or_leave_cell() {
        let p = crate::ir::compile_scalar("limits.n", "fn Main() {\n    let r = MockOpen(1);\n}\n")
            .unwrap();
        let mut m = machine();
        m.log.push_stdout("x".repeat(65_490)).unwrap();
        m.run(p.functions().function("Main").unwrap()).unwrap();
        assert_eq!(m.fault.as_ref().map(|d| d.code.as_str()), Some("R0002"));
        assert!(m.log.events().is_empty());
        assert_eq!(m.log.reserved_bytes(), 0);
        assert_eq!(m.cells, 0);
    }
    #[test]
    fn rejected_open_event_releases_prepared_close_reservation() {
        let p =
            crate::ir::compile_scalar("limits.n", "fn Main() {\n    MockOpen(1);\n}\n").unwrap();
        let mut m = machine();
        m.log.push_stdout("x".repeat(65_499)).unwrap();
        m.run(p.functions().function("Main").unwrap()).unwrap();
        assert_eq!(m.fault.as_ref().map(|d| d.code.as_str()), Some("R0002"));
        assert!(m.log.events().is_empty());
        assert_eq!(m.log.reserved_bytes(), 0);
        assert_eq!(m.cells, 0);
    }
    #[test]
    fn rejected_close_reservation_restores_new_resource_cell() {
        let p =
            crate::ir::compile_scalar("limits.n", "fn Main() {\n    MockOpen(1);\n}\n").unwrap();
        let mut m = machine();
        m.log.push_stdout("x".repeat(65_529)).unwrap();
        m.run(p.functions().function("Main").unwrap()).unwrap();
        assert_eq!(m.fault.as_ref().map(|d| d.code.as_str()), Some("R0002"));
        assert!(m.log.events().is_empty());
        assert_eq!(m.log.reserved_bytes(), 0);
        assert_eq!(m.cells, 0);
    }

    #[test]
    fn rejected_mock_batch_releases_reserved_result_cells() {
        let program = crate::ir::compile_scalar(
            "batch.n",
            "fn Main()->Result<(),I64>{MockWriteI64(25)?;return Ok(());}",
        )
        .unwrap();
        let mut m = machine();
        m.mock_env = MockEnvironment::parse(
            br#"{"schema_version":1,"mock_read_records":[],"mock_write_i64":[{"ok":null}],"mock_request":[],"mock_reply":[]}"#,
        )
        .unwrap();
        m.log.push_stdout("x".repeat(65_520)).unwrap();
        m.run(program.functions().function("Main").unwrap())
            .unwrap();
        assert_eq!(
            m.fault.as_ref().map(|diagnostic| diagnostic.code.as_str()),
            Some("R0002")
        );
        assert_eq!(m.cells, 0);
    }

    #[test]
    fn write_request_and_reply_each_reserve_two_result_cells() {
        for (source, environment) in [
            (
                "fn Main(){MockWriteI64(25);}",
                br#"{"schema_version":1,"mock_read_records":[],"mock_write_i64":[{"ok":null}],"mock_request":[],"mock_reply":[]}"#.as_slice(),
            ),
            (
                "fn Main(){MockRequest();}",
                br#"{"schema_version":1,"mock_read_records":[],"mock_write_i64":[],"mock_request":[{"ok":"1"}],"mock_reply":[]}"#,
            ),
            (
                "fn Main(){MockReply(200);}",
                br#"{"schema_version":1,"mock_read_records":[],"mock_write_i64":[],"mock_request":[],"mock_reply":[{"ok":null}]}"#,
            ),
        ] {
            let program = crate::ir::compile_scalar("primitive-cells.n", source).unwrap();
            let mut m = machine();
            m.mock_env = MockEnvironment::parse(environment).unwrap();
            m.cells = 65_535;
            m.run(program.functions().function("Main").unwrap())
                .unwrap();
            let fault = m.fault.as_ref().expect("two-cell result reservation");
            assert_eq!(fault.code, "R0002", "{source}");
            assert_eq!(fault.data["runtime_budget_kind"], DataValue::String("live_value_cells".into()));
            assert_eq!(fault.data["observed"], DataValue::UInt(65_537));
            assert_eq!(m.cells, 65_535);
        }
    }

    #[test]
    fn mock_unknown_is_runtime_unit_without_events_or_faults() {
        let program = crate::ir::compile_scalar("limits.n", "fn Main(){MockUnknown();}").unwrap();
        let mut machine = machine();
        machine
            .run(program.functions().function("Main").unwrap())
            .unwrap();
        assert!(machine.fault.is_none());
        assert!(machine.log.events().is_empty());
        assert_eq!(machine.cells, 0);
    }
}
