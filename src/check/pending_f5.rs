//! Temporary F5 capability boundaries keyed by AST nodes or resolved prelude identities.

use crate::check::names;
use crate::diagnostic::{DataValue, Diagnostic, Operation, Span, Stage};
use crate::frontend::parse::{ConstDecl, Expr};

pub(crate) fn diagnostic(scope: &'static str, span: Span) -> Box<Diagnostic> {
    let mut error = names::diagnostic("NX-E0006", None, Stage::Types, Operation::TypesCheck, span);
    error.feature_id = Some("implementation_limit".into());
    error.message = format!("Capacidad pendiente de implementación en {scope}.");
    error
        .data
        .insert("pending_scope".into(), DataValue::String(scope.into()));
    error
}

pub(crate) fn range_for(iterable: &Expr) -> bool {
    let mut node = iterable;
    while let Expr::Group(inner, _) = node {
        node = inner;
    }
    matches!(node, Expr::Binary { op, .. } if op == "..<" || op == "..=")
}

pub(crate) fn range_outside_for(span: Span) -> Box<Diagnostic> {
    let mut error = names::diagnostic("NX-E0006", None, Stage::Types, Operation::TypesCheck, span);
    error.feature_id = Some("range".into());
    error.message = "Recognized construct is outside the profile.".into();
    error.data.insert(
        "pending_scope".into(),
        DataValue::String("later_adr".into()),
    );
    error
}

pub(crate) fn intrinsic_scope(identity: &str) -> Option<&'static str> {
    crate::prelude::f5_array_intrinsic(identity).map(|_| "L07")
}

pub(crate) fn const_pending(declaration: &ConstDecl) -> Box<Diagnostic> {
    diagnostic("L09", declaration.const_span.clone())
}
