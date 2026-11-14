mod common;
use nexo::interp::{MockEnvironment, Outcome};
use nexo::ir::CompileError;

fn rejected(source: &str) -> Box<nexo::diagnostic::Diagnostic> {
    let error = common::compile_j("<memory>", source, true).unwrap_err();
    let CompileError::Rejected(mut diagnostics) = error else {
        panic!("unexpected compiler failure")
    };
    assert_eq!(diagnostics.len(), 1);
    diagnostics.remove(0)
}

#[test]
fn cold_generator_discards_owned_parameter_without_running_body() {
    let source = "gen fn G(r:move MockResource)->I64{Inspect(r);}fn Main(){let g=G(MockOpen(4));}";
    let execution = common::run_j("cold.n", source, MockEnvironment::default()).unwrap();
    assert_eq!(execution.outcome, Outcome::Unit);
    assert!(execution.diagnostics.is_empty());
    assert_eq!(execution.observations.events(), ["open:0:4", "close:0:4"]);
}

#[test]
fn generator_arguments_evaluate_in_written_order_and_close_in_slot_order() {
    let source = "gen fn G(a:move MockResource,b:move MockResource)->I64{}fn Main(){let g=G(b:MockOpen(2),a:MockOpen(1));}";
    let execution = common::run_j("<memory>", source, MockEnvironment::default()).unwrap();
    assert_eq!(execution.outcome, Outcome::Unit);
    assert_eq!(
        execution.observations.events(),
        ["open:0:2", "open:1:1", "close:0:2", "close:1:1"]
    );
}

#[test]
fn generator_scalar_view_is_materialized_and_does_not_retain_a_caller_loan() {
    let source = "gen fn G(x:view I64)->I64{}fn Main(){var x=1;let g=G(x);x=2;}";
    let execution = common::run_j("<memory>", source, MockEnvironment::default()).unwrap();
    assert_eq!(execution.outcome, Outcome::Unit);
    assert!(execution.diagnostics.is_empty());
}

#[test]
fn generator_parameter_retention_rejects_view_resource() {
    let source = "gen fn G(r:view MockResource)->I64{Inspect(r);yield 1;}fn Main(){}";
    let error = rejected(source);
    assert_eq!(error.code, "NX-E1110");
    assert_eq!(error.subcause, None);
    assert_eq!(error.stage.as_str(), "ownership");
    assert_eq!(error.operation.as_str(), "ownership.check");
    let span = error.primary_span.unwrap();
    assert_eq!((span.start, span.end), (11, 15));
    assert_eq!(error.related_spans.len(), 1);
    assert_eq!(
        (
            error.related_spans[0].span.start,
            error.related_spans[0].span.end
        ),
        (9, 10)
    );
    assert!(error.data.is_empty());
}

#[test]
fn generator_layout_rejects_direct_recursive_state() {
    let source = "gen fn G()->I64{let g=G();}fn Main(){}";
    let error = rejected(source);
    assert_eq!(error.code, "NX-E0006");
    assert_eq!(error.feature_id.as_deref(), Some("generator"));
    assert_eq!(error.stage.as_str(), "types");
    assert_eq!(error.operation.as_str(), "types.check");
    let span = error.primary_span.unwrap();
    assert_eq!((span.start, span.end), (7, 8));
    assert_eq!(
        error.data.get("pending_scope"),
        Some(&nexo::diagnostic::DataValue::String("later_adr".into()))
    );
    let positive = "gen fn G()->I64{let g=H();}gen fn H()->I64{}fn Main(){}";
    let execution = common::run_j("<memory>", positive, MockEnvironment::default()).unwrap();
    assert_eq!(execution.outcome, Outcome::Unit);
    assert!(execution.diagnostics.is_empty());
}

#[test]
fn shadowed_generator_name_called_as_a_closure_is_not_a_layout_cycle() {
    let source = "gen fn G()->I64{let G=fn [](){};G();yield 1;}fn Main(){var g=G();let n=Iterator.Next(edit g);AssertEq(n,Some(1));}";
    let execution = common::run_j("<memory>", source, MockEnvironment::default()).unwrap();
    assert_eq!(execution.outcome, Outcome::Unit);
}
