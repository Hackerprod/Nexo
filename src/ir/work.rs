//! Static work catalog. Lowering asks this table for every fixed language tick.
use crate::diagnostic::Span;

use super::model::WorkCharge;

#[derive(Clone, Copy)]
pub(crate) enum WorkEvent {
    Entry,
    FinalUnit,
    WhileCondition,
    InoutPrefix,
    ReadSelector,
    ReadName,
    PlaceSelector,
    ReturnStatement,
    ValueStatement,
    ExpressionStatement,
    EmptyValue,
    IfStatement,
    Block,
    Assignment,
    Binding,
    IfExpression,
    IndexExpression,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum WorkOrigin {
    FunctionToken,
    ClosingBrace,
    Condition,
    EditPrefix,
    Selector,
    PlaceRoot,
    ReturnToken,
    ValueMarkerOrOperand,
    Expression,
    ValueStatement,
    IfOrWhileToken,
    OpeningBrace,
    FullPlace,
    BindingToken,
    BranchOperand,
    IndexAccess,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum WorkPhase {
    Entry,
    BeforeChildren,
    PreOrder,
    AfterOperand,
    Exit,
}

#[derive(Clone, Copy)]
struct WorkRule {
    event: WorkEvent,
    units: u64,
    origin: WorkOrigin,
    phase: WorkPhase,
    order: u8,
}

const CATALOG: &[WorkRule] = &[
    WorkRule {
        event: WorkEvent::Entry,
        units: 1,
        origin: WorkOrigin::FunctionToken,
        phase: WorkPhase::Entry,
        order: 0,
    },
    WorkRule {
        event: WorkEvent::FinalUnit,
        units: 1,
        origin: WorkOrigin::ClosingBrace,
        phase: WorkPhase::Exit,
        order: 16,
    },
    WorkRule {
        event: WorkEvent::WhileCondition,
        units: 1,
        origin: WorkOrigin::Condition,
        phase: WorkPhase::BeforeChildren,
        order: 1,
    },
    WorkRule {
        event: WorkEvent::InoutPrefix,
        units: 1,
        origin: WorkOrigin::EditPrefix,
        phase: WorkPhase::BeforeChildren,
        order: 2,
    },
    WorkRule {
        event: WorkEvent::ReadSelector,
        units: 1,
        origin: WorkOrigin::Selector,
        phase: WorkPhase::PreOrder,
        order: 3,
    },
    WorkRule {
        event: WorkEvent::ReadName,
        units: 1,
        origin: WorkOrigin::PlaceRoot,
        phase: WorkPhase::PreOrder,
        order: 4,
    },
    WorkRule {
        event: WorkEvent::PlaceSelector,
        units: 1,
        origin: WorkOrigin::Selector,
        phase: WorkPhase::PreOrder,
        order: 5,
    },
    WorkRule {
        event: WorkEvent::ReturnStatement,
        units: 1,
        origin: WorkOrigin::ReturnToken,
        phase: WorkPhase::BeforeChildren,
        order: 6,
    },
    WorkRule {
        event: WorkEvent::ValueStatement,
        units: 1,
        origin: WorkOrigin::ValueMarkerOrOperand,
        phase: WorkPhase::BeforeChildren,
        order: 7,
    },
    WorkRule {
        event: WorkEvent::ExpressionStatement,
        units: 1,
        origin: WorkOrigin::Expression,
        phase: WorkPhase::AfterOperand,
        order: 8,
    },
    WorkRule {
        event: WorkEvent::EmptyValue,
        units: 1,
        origin: WorkOrigin::ValueStatement,
        phase: WorkPhase::BeforeChildren,
        order: 9,
    },
    WorkRule {
        event: WorkEvent::IfStatement,
        units: 1,
        origin: WorkOrigin::IfOrWhileToken,
        phase: WorkPhase::BeforeChildren,
        order: 10,
    },
    WorkRule {
        event: WorkEvent::Block,
        units: 1,
        origin: WorkOrigin::OpeningBrace,
        phase: WorkPhase::BeforeChildren,
        order: 11,
    },
    WorkRule {
        event: WorkEvent::Assignment,
        units: 1,
        origin: WorkOrigin::FullPlace,
        phase: WorkPhase::PreOrder,
        order: 12,
    },
    WorkRule {
        event: WorkEvent::Binding,
        units: 1,
        origin: WorkOrigin::BindingToken,
        phase: WorkPhase::BeforeChildren,
        order: 13,
    },
    WorkRule {
        event: WorkEvent::IfExpression,
        units: 1,
        origin: WorkOrigin::BranchOperand,
        phase: WorkPhase::PreOrder,
        order: 14,
    },
    WorkRule {
        event: WorkEvent::IndexExpression,
        units: 1,
        origin: WorkOrigin::IndexAccess,
        phase: WorkPhase::PreOrder,
        order: 15,
    },
];

pub(crate) struct WorkPlanner;

impl WorkPlanner {
    fn rule(event: WorkEvent) -> &'static WorkRule {
        CATALOG
            .iter()
            .find(|rule| std::mem::discriminant(&rule.event) == std::mem::discriminant(&event))
            .expect("every static event has one catalog row")
    }

    pub(crate) fn units(event: WorkEvent) -> u64 {
        Self::rule(event).units
    }

    #[cfg(test)]
    pub(crate) fn origin(event: WorkEvent) -> WorkOrigin {
        Self::rule(event).origin
    }

    #[cfg(test)]
    pub(crate) fn phase(event: WorkEvent) -> WorkPhase {
        Self::rule(event).phase
    }

    #[cfg(test)]
    pub(crate) fn order(event: WorkEvent) -> u8 {
        Self::rule(event).order
    }

    pub(crate) fn charge(event: WorkEvent, origin: Span) -> WorkCharge {
        let rule = Self::rule(event);
        let _routing = (rule.origin, rule.phase, rule.order);
        WorkCharge {
            ticks: rule.units,
            origin,
        }
    }
}
