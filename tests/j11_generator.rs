mod common;
use nexo::interp::{MockEnvironment, Outcome};
use nexo::ir::CompileError;

fn diagnostic(source: &str) -> Box<nexo::diagnostic::Diagnostic> {
    match common::compile_j("<memory>", source, true).unwrap_err() {
        CompileError::Rejected(mut diagnostics) => {
            assert_eq!(diagnostics.len(), 1, "{source}");
            diagnostics.remove(0)
        }
        _ => panic!("unexpected compiler failure: {source}"),
    }
}

#[test]
fn generator_yield_next_and_return_rules_have_paired_sources() {
    let cases = [
        (
            "gen fn G()->I64{return 1;}fn Main(){}",
            "NX-E1324",
            None,
            (16, 22),
            "gen fn G()->I64{return;}fn Main(){}",
        ),
        (
            "fn Main(){yield 1;}",
            "NX-E1213",
            None,
            (10, 15),
            "gen fn G()->I64{yield 1;}fn Main(){}",
        ),
        (
            "gen fn G()->Result<I64,I64>{let x=TryAdd(1,2)?;yield Ok(x);}fn Main(){}",
            "NX-E1214",
            None,
            (45, 46),
            "gen fn G()->Result<I64,I64>{yield TryAdd(1,2);}fn Main(){}",
        ),
        (
            "gen fn G(){yield 1;}fn Main(){}",
            "NX-E0102",
            Some("type_mismatch"),
            (17, 18),
            "gen fn G(){yield ();}fn Main(){}",
        ),
        (
            "gen fn G(){return ();}fn Main(){}",
            "NX-E1324",
            None,
            (11, 17),
            "gen fn G(){return;}fn Main(){}",
        ),
        (
            "fn Main(){var x=1;let n=Iterator.Next(edit x);}",
            "NX-E0102",
            Some("type_mismatch"),
            (38, 44),
            "gen fn G()->I64{}fn Main(){var x=G();let n=Iterator.Next(edit x);}",
        ),
        (
            "gen fn G()->I64{}fn Main(){var g=G();let n=Iterator.Next(g);}",
            "NX-E0111",
            Some("argument_mode_mismatch"),
            (57, 58),
            "gen fn G()->I64{}fn Main(){var g=G();let n=Iterator.Next(edit g);}",
        ),
        (
            "gen fn G(){}fn Main(){let Iterator=0;var g=G();let n=Iterator.Next(edit g);}",
            "NX-E0006",
            None,
            (61, 62),
            "gen fn G(){}fn Main(){let other=0;var g=G();let n=Iterator.Next(edit g);}",
        ),
        (
            "gen fn G(){}fn Main(){let g=G();let n=Iterator.Next(edit g);}",
            "NX-E0105",
            None,
            (57, 58),
            "gen fn G(){}fn Main(){var g=G();let n=Iterator.Next(edit g);}",
        ),
    ];
    for (negative, code, subcause, span, positive) in cases {
        let error = diagnostic(negative);
        assert_eq!(error.code, code, "{negative}");
        assert_eq!(error.subcause.as_deref(), subcause, "{negative}");
        let got = error.primary_span.unwrap();
        assert_eq!((got.start, got.end), span, "{negative}");
        let execution = common::run_j("<memory>", positive, MockEnvironment::default()).unwrap();
        assert_eq!(execution.outcome, Outcome::Unit, "{positive}");
        assert!(execution.diagnostics.is_empty(), "{positive}");
    }
}

#[test]
fn next_returns_one_item_then_repeated_none() {
    let source = "gen fn G(){yield ();}fn Main(){var g=G();AssertEq(Iterator.Next(edit g),Some(()));AssertEq(Iterator.Next(edit g),None);AssertEq(Iterator.Next(edit g),None);}";
    let execution = common::run_j("<memory>", source, MockEnvironment::default()).unwrap();
    assert_eq!(execution.outcome, Outcome::Unit);
    assert!(execution.diagnostics.is_empty());
    assert!(execution.observations.observations().is_empty());
}

#[test]
fn suspended_generator_moves_with_its_continuation_and_owned_resource() {
    let source = "gen fn G(r:move MockResource)->I64{yield 1;Inspect(r);yield 2;}fn Main(){var g=G(MockOpen(4));AssertEq(Iterator.Next(edit g),Some(1));var h=move g;AssertEq(Iterator.Next(edit h),Some(2));AssertEq(Iterator.Next(edit h),None);}";
    let execution = common::run_j("<memory>", source, MockEnvironment::default()).unwrap();
    assert_eq!(execution.outcome, Outcome::Unit);
    assert!(execution.diagnostics.is_empty());
    assert_eq!(
        execution.observations.events(),
        ["open:0:4", "inspect:0:4", "close:0:4"]
    );
}

#[test]
fn yield_inside_while_resumes_after_operand_without_repeating_prior_steps() {
    let source = "gen fn G()->I64{var n=0;while n<3{yield n;n=n+1;}}fn Main(){var g=G();AssertEq(Iterator.Next(edit g),Some(0));AssertEq(Iterator.Next(edit g),Some(1));AssertEq(Iterator.Next(edit g),Some(2));AssertEq(Iterator.Next(edit g),None);}";
    let execution = common::run_j("<memory>", source, MockEnvironment::default()).unwrap();
    assert_eq!(execution.outcome, Outcome::Unit);
    assert!(execution.diagnostics.is_empty());
}

#[test]
fn yielded_resource_is_transferred_to_the_option_owner_once() {
    let source = "gen fn G()->MockResource{yield MockOpen(7);}fn Main(){var g=G();let item=Iterator.Next(edit g);match move item{Some(move r)=>{Drop(r);},None=>{}}}";
    let execution = common::run_j("<memory>", source, MockEnvironment::default()).unwrap();
    assert_eq!(execution.outcome, Outcome::Unit);
    assert_eq!(execution.observations.events(), ["open:0:7", "close:0:7"]);
}

#[test]
fn generator_question_mark_and_shadowed_iterator_limit_forms() {
    let question = "gen fn G()->I64{yield 1?;}fn Main(){}";
    let error = diagnostic(question);
    assert_eq!(error.code, "NX-E1214");
    assert_eq!(
        error
            .primary_span
            .as_ref()
            .map(|span| (span.start, span.end)),
        Some((23, 24))
    );
    let error = diagnostic("struct Iterator{}fn Main(){}");
    assert_eq!(error.code, "NX-E0006");
    assert_eq!(error.feature_id.as_deref(), Some("user_types"));
    let positive = "gen fn G(){}fn Main(){var g=G();AssertEq(Iterator.Next(edit g),None);}";
    let execution = common::run_j("<memory>", positive, MockEnvironment::default()).unwrap();
    assert_eq!(execution.outcome, Outcome::Unit);
}
