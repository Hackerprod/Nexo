use nexo::{
    diagnostic::{DataValue, Diagnostic},
    interp::{self, Outcome, observation::Channel},
    ir::{self, CompileError},
};

const FILE: &str = "i08.n";

#[allow(clippy::vec_box)] // Matches the diagnostic API returned by compilation.
fn rejected(source: &str, run: bool) -> Vec<Box<Diagnostic>> {
    let result = if run {
        ir::compile_scalar(FILE, source)
    } else {
        ir::check_scalar(FILE, source)
    };
    match result {
        Err(CompileError::Rejected(errors)) => errors,
        other => panic!("expected source rejection: {source}: {other:?}"),
    }
}

fn run(source: &str) -> interp::Execution {
    interp::run_scalar(&ir::compile_scalar(FILE, source).expect(source))
}

#[test]
fn question_mark_accepts_never_operand_when_all_if_branches_return() {
    let source = "fn Main()->Result<(),I64>{let x=(if true then {return Err(1);} else {return Err(2);})?;return Ok(());}";

    ir::check_scalar(FILE, source).expect("Never is the bottom type, not a failed operand");
    assert_eq!(run(source).outcome, Outcome::Err(1));
}

fn span(d: &Diagnostic) -> (u64, u64) {
    let s = d.primary_span.as_ref().unwrap();
    (s.start, s.end)
}

fn nested_result_array_prefix(result_main: bool) -> String {
    let mut source = String::from(if result_main {
        "fn Main()->Result<(),I64>{"
    } else {
        "fn Main(){"
    });
    source.push_str("let a0:Array<Result<I64,I64>>=[Ok(1)];");
    for depth in 1..14 {
        source.push_str(&format!("let a{depth}=[a{},a{}];", depth - 1, depth - 1));
    }
    source
}

#[test]
fn published_result_type_negatives_and_repairs() {
    let cases = [
        (
            "fn Main(){let r=Ok(1);}",
            "NX-E0113",
            None,
            (16, 21),
            "fn Main(){let r:Result<I64,I64>=Ok(1);}",
        ),
        (
            "fn Main(){let a:Result<I64,I64>=Ok(1);let b=a<a;}",
            "NX-E0102",
            Some("type_mismatch"),
            (45, 46),
            "fn Main(){let a:Result<I64,I64>=Ok(1);let b=a==a;}",
        ),
        (
            "fn Main(){let a:Result<MockResource,I64>=Ok(MockOpen(1));let b=a==a;}",
            "NX-E0102",
            Some("type_mismatch"),
            (64, 66),
            "fn Main(){let a:Result<MockResource,I64>=Ok(MockOpen(1));Drop(a);}",
        ),
        (
            "fn Main(){let r:Result<MockResource,I64>=Ok(MockOpen(1));let s=r;Drop(s);}",
            "NX-E0201",
            None,
            (63, 64),
            "fn Main(){let r:Result<MockResource,I64>=Ok(MockOpen(1));let s=move r;Drop(s);}",
        ),
        (
            "fn Main(){let r:Result<I64,MockResource>=Ok(1);let s=r;}",
            "NX-E0201",
            None,
            (53, 54),
            "fn Main(){let r:Result<I64,MockResource>=Ok(1);let s=move r;}",
        ),
        (
            "fn Main()->Result<(),I64>{let x=1?;return Ok(());}",
            "NX-E0102",
            Some("type_mismatch"),
            (33, 34),
            "fn Main()->Result<(),I64>{let x=TryAdd(1,0)?;return Ok(());}",
        ),
        (
            "fn Main(){let x=TryAdd(1,2)?;}",
            "NX-E0102",
            Some("type_mismatch"),
            (27, 28),
            "fn Main()->Result<(),I64>{let x=TryAdd(1,2)?;return Ok(());}",
        ),
        (
            "fn Main()->Result<(),Bool>{let x=TryAdd(1,2)?;return Ok(());}",
            "NX-E0102",
            Some("type_mismatch"),
            (44, 45),
            "fn Main()->Result<(),I64>{let x=TryAdd(1,2)?;return Ok(());}",
        ),
    ];
    for (source, code, subcause, primary, repair) in cases {
        let errors = rejected(source, true);
        assert_eq!(errors.len(), 1, "{source}: {errors:?}");
        let d = &errors[0];
        assert_eq!(d.code, code, "{source}");
        assert_eq!(d.subcause.as_deref(), subcause, "{source}");
        assert_eq!(span(d), primary, "{source}");
        d.validate(FILE, source).unwrap();
        let result = run(repair);
        assert!(
            matches!(result.outcome, Outcome::Unit | Outcome::Ok),
            "{repair}: {result:?}"
        );
    }
}

#[test]
fn option_stays_deferred_and_result_parameter_is_admitted() {
    let negative = "fn F(x:view Option<I64>){} fn Main(){}";
    let d = &rejected(negative, true)[0];
    assert_eq!(d.code, "NX-E0006");
    assert_eq!(d.feature_id.as_deref(), Some("option"));
    ir::compile_scalar(FILE, "fn F(x:view Result<I64,I64>){} fn Main(){}").unwrap();
}

#[test]
fn entry_is_checked_only_for_run_after_other_errors() {
    let absent = "fn F(){}";
    ir::check_scalar(FILE, absent).unwrap();
    let missing = rejected(absent, true);
    assert_eq!(missing.len(), 1);
    assert_eq!(missing[0].code, "NX-E0112");
    assert_eq!(missing[0].subcause.as_deref(), Some("missing_main"));
    assert_eq!(
        span(&missing[0]),
        (absent.len() as u64, absent.len() as u64)
    );
    assert_eq!(
        missing[0].data.get("entry"),
        Some(&DataValue::String("Main".into()))
    );
    assert_eq!(missing[0].data.len(), 1);
    assert!(missing[0].related_spans.is_empty());
    assert!(missing[0].unexpected_token.is_none() && missing[0].expected_tokens.is_empty());
    missing[0].validate(FILE, absent).unwrap();
    let invalid = "fn Main(x:view I64){}";
    let errors = rejected(invalid, true);
    assert_eq!(errors.len(), 1);
    assert_eq!(errors[0].code, "NX-E0112");
    assert_eq!(
        errors[0].subcause.as_deref(),
        Some("invalid_main_signature")
    );
    assert_eq!(span(&errors[0]), (3, 7));
    assert_eq!(errors[0].related_spans.len(), 1);
    assert_eq!(errors[0].related_spans[0].span.start, 0);
    assert_eq!(errors[0].related_spans[0].span.end, 19);
    assert_eq!(errors[0].data.len(), 1);
    assert_eq!(
        errors[0].data.get("entry"),
        Some(&DataValue::String("Main".into()))
    );
    assert!(errors[0].unexpected_token.is_none() && errors[0].expected_tokens.is_empty());
    errors[0].validate(FILE, invalid).unwrap();
    for source in ["fn Main(){}", "fn Main()->Result<(),I64>{return Ok(());}"] {
        ir::compile_scalar(FILE, source).unwrap();
    }
    for source in [
        "fn Main()->I64{return 1;}",
        "fn Main()->Result<(),Bool>{return Ok(());}",
    ] {
        assert_eq!(
            rejected(source, true)[0].subcause.as_deref(),
            Some("invalid_main_signature")
        );
        ir::check_scalar(FILE, source).unwrap();
    }
    assert_eq!(rejected("fn F(){let x=true+1;}", true)[0].code, "NX-E0102");
}

#[test]
fn b_order_run_matches_check_after_lowering_error() {
    let ownership = "fn F(){let r:Result<MockResource,I64>=Ok(MockOpen(1));let s=r;}";
    let checked = rejected(ownership, false);
    let executed = rejected(ownership, true);
    assert_eq!(
        (checked[0].code.as_str(), span(&checked[0])),
        ("NX-E0201", (60, 61))
    );
    assert_eq!(
        (executed[0].code.as_str(), span(&executed[0])),
        ("NX-E0201", (60, 61))
    );

    let lowering = "fn f()->I64 { } fn main(){}";
    let checked = rejected(lowering, false);
    let executed = rejected(lowering, true);
    assert_eq!(
        (checked[0].code.as_str(), span(&checked[0])),
        ("NX-E1317", (14, 15))
    );
    assert_eq!(
        (executed[0].code.as_str(), span(&executed[0])),
        ("NX-E1317", (14, 15))
    );

    ir::check_scalar(FILE, "").unwrap();
    assert_eq!(
        rejected("", true)[0].subcause.as_deref(),
        Some("missing_main")
    );
    let bad_entry = "fn Main(x:view I64){}";
    ir::check_scalar(FILE, bad_entry).unwrap();
    assert_eq!(
        rejected(bad_entry, true)[0].subcause.as_deref(),
        Some("invalid_main_signature")
    );
    ir::check_scalar(FILE, "fn Main(){}").unwrap();
    ir::compile_scalar(FILE, "fn Main(){}").unwrap();
}

#[test]
fn result_build_and_branch_transfer_temporaries_once() {
    let k5 = "fn H(a:move Result<MockResource,I64>,b:view I64)->I64{return b;}\
fn P()->Result<(),I64>{let z=H(Ok(MockOpen(1)),if true then {value:0;}else{return Err(5);});return Ok(());}\
fn Main(){let q=P();}";
    ir::check_scalar(FILE, k5).unwrap();
    let execution = run(k5);
    assert_eq!(execution.outcome, Outcome::Unit);
    assert!(execution.diagnostics.is_empty());
    assert!(execution.observations.stdout().is_empty());
    let events: Vec<_> = execution
        .observations
        .observations()
        .iter()
        .filter(|o| o.channel == Channel::Event)
        .map(|o| o.text.as_str())
        .collect();
    assert_eq!(events, ["open:0:1", "close:0:1"]);

    let k4 = "fn G(a:view I64,b:view I64)->I64{return a;}\
fn Mk()->Result<I64,I64>{return Ok(1);}\
fn P()->Result<(),I64>{let z=G(Mk()?,if true then {value:0;}else{return Err(5);});return Ok(());}\
fn Main(){let q=P();}";
    ir::check_scalar(FILE, k4).unwrap();
    let execution = run(k4);
    assert_eq!(execution.outcome, Outcome::Unit);
    assert!(execution.diagnostics.is_empty());
    assert!(execution.observations.stdout().is_empty());
    assert!(
        !execution
            .observations
            .observations()
            .iter()
            .any(|o| o.channel == Channel::Event)
    );
    ir::check_scalar(FILE, "fn Main(){}").unwrap();
}

#[test]
fn e0202_after_while_pins_current_behavior() {
    let source = "fn F(){let v1:I64=6;Drop(v1);let z2=v1;while(true){}let r=MockOpen(8);let z=r;}";
    let errors = rejected(source, false);
    assert_eq!(errors.len(), 1);
    assert_eq!(
        (errors[0].code.as_str(), span(&errors[0])),
        ("NX-E0201", (76, 77))
    );

    let without_loop = "fn F(){let v1:I64=6;Drop(v1);let z2=v1;let r=MockOpen(8);let z=r;}";
    let errors = rejected(without_loop, false);
    assert!(
        errors
            .iter()
            .any(|d| d.code == "NX-E0202" && span(d) == (36, 38))
    );
    assert!(errors.iter().any(|d| d.code == "NX-E0201"));
    let without_final_error = "fn F(){let v1:I64=6;Drop(v1);let z2=v1;while(true){}}";
    let errors = rejected(without_final_error, false);
    assert_eq!(
        (errors[0].code.as_str(), span(&errors[0])),
        ("NX-E0202", (36, 38))
    );
}

#[test]
fn result_array_quota_uses_expanded_source_and_releases_payload() {
    let mut source = String::from("fn Main(){let m=MockOpen(1);let a:Array<Result<I64,I64>>=[");
    for index in 0..32768 {
        if index > 0 {
            source.push(',');
        }
        source.push_str("Ok(1)");
    }
    source.push_str("];Print(7);}");
    let bytes = source.len();
    assert_eq!(bytes, 196_677);
    ir::check_scalar(FILE, &source).unwrap();
    let execution = run(&source);
    assert_eq!(execution.outcome, Outcome::SimulatorFault);
    assert!(execution.observations.stdout().is_empty());
    assert_eq!(execution.diagnostics[0].code, "R0002");
    let fault = &execution.diagnostics[0];
    assert_eq!(span(fault), (196_660, 196_665));
    assert_eq!(
        fault.data.get("runtime_budget_kind"),
        Some(&DataValue::String("live_value_cells".into()))
    );
    let events: Vec<_> = execution
        .observations
        .observations()
        .iter()
        .filter(|o| o.channel == Channel::Event)
        .map(|o| o.text.as_str())
        .collect();
    assert_eq!(events, ["open:0:1", "close:0:1"]);
}

#[test]
fn result_payloads_and_propagation_clean_once() {
    let err = "fn Fallo()->Result<I64,I64>{let r=MockOpen(2);return Err(9);}fn Main()->Result<(),I64>{let r=MockOpen(1);let n=Fallo()?;Print(n);return Ok(());}";
    let outcome = run(err);
    assert_eq!(outcome.outcome, Outcome::Err(9));
    assert!(outcome.diagnostics.is_empty());
    let events: Vec<_> = outcome
        .observations
        .observations()
        .iter()
        .filter(|o| o.channel == Channel::Event)
        .map(|o| o.text.as_str())
        .collect();
    assert_eq!(events, ["open:0:1", "open:1:2", "close:1:2", "close:0:1"]);
    let success = "fn Crear()->Result<MockResource,I64>{return Ok(MockOpen(3));}fn Main()->Result<(),I64>{let r=Crear()?;Inspect(r);return Ok(());}";
    let outcome = run(success);
    assert_eq!(outcome.outcome, Outcome::Ok);
    let events: Vec<_> = outcome
        .observations
        .observations()
        .iter()
        .filter(|o| o.channel == Channel::Event)
        .map(|o| o.text.as_str())
        .collect();
    assert_eq!(events, ["open:0:3", "inspect:0:3", "close:0:3"]);
}

#[test]
fn try_add_uses_err_channel_while_add_uses_simulator_fault() {
    let result =
        run("fn Main()->Result<(),I64>{let x=TryAdd(9223372036854775807,1)?;return Ok(());}");
    assert_eq!(result.outcome, Outcome::Err(30));
    assert!(result.diagnostics.is_empty());
    let fault = run("fn Main()->Result<(),I64>{let x=9223372036854775807+1;return Ok(());}");
    assert_eq!(fault.outcome, Outcome::SimulatorFault);
    assert_eq!(fault.diagnostics[0].code, "R0101");
}

#[test]
fn try_add_evaluates_labeled_arguments_in_source_order() {
    let source = "fn First()->I64{Print(1);return 10;}fn Second()->I64{Print(2);return 20;}fn Main()->Result<(),I64>{let n=TryAdd(b:First(),a:Second())?;AssertEq(n,30);return Ok(());}";
    let execution = run(source);
    assert_eq!(execution.outcome, Outcome::Ok);
    assert_eq!(execution.observations.stdout(), ["1", "2"]);
}

#[test]
fn result_wrappers_charge_cells_at_budget_boundary() {
    let program = |extra: usize| {
        let mut source = nested_result_array_prefix(false);
        for index in 0..extra {
            source.push_str(&format!("let r{index}:Result<I64,I64>=Ok({index});"));
        }
        if extra == 9 {
            source.push_str("Print(99);");
        }
        source.push('}');
        source
    };
    assert_eq!(run(&program(8)).outcome, Outcome::Unit);
    let execution = run(&program(9));
    assert_eq!(execution.outcome, Outcome::SimulatorFault);
    assert!(execution.observations.stdout().is_empty());
    assert_eq!(execution.diagnostics.len(), 1);
    let fault = &execution.diagnostics[0];
    assert_eq!(fault.code, "R0002");
    assert_eq!(fault.data.get("limit"), Some(&DataValue::UInt(65_536)));
    assert_eq!(fault.data.get("observed"), Some(&DataValue::UInt(65_537)));
    assert_eq!(
        fault.data.get("runtime_budget_kind"),
        Some(&DataValue::String("live_value_cells".into()))
    );
}

#[test]
fn result_wrapper_quota_fault_releases_unconsumed_resource_payload() {
    let mut source = nested_result_array_prefix(false);
    for index in 0..8 {
        source.push_str(&format!("let r{index}:Result<I64,I64>=Ok({index});"));
    }
    source.push_str("let r:Result<MockResource,I64>=Ok(MockOpen(7));Print(99);}");
    let execution = run(&source);
    assert_eq!(execution.outcome, Outcome::SimulatorFault);
    assert_eq!(execution.diagnostics[0].code, "R0002");
    assert!(execution.observations.stdout().is_empty());
    let events: Vec<_> = execution
        .observations
        .observations()
        .iter()
        .filter(|o| o.channel == Channel::Event)
        .map(|o| o.text.as_str())
        .collect();
    assert_eq!(events, ["open:0:7", "close:0:7"]);
}

#[test]
fn propagation_releases_wrapper_cells_between_iterations() {
    let mut source = nested_result_array_prefix(true);
    source
        .push_str("let r:Result<I64,I64>=Ok(1);var i=0;while i<40{let x=r?;i=i+1;}return Ok(());}");
    assert_eq!(run(&source).outcome, Outcome::Ok);
}

#[test]
fn try_add_reserves_result_cells_once_near_budget() {
    let program = |extra: usize| {
        let mut source = nested_result_array_prefix(false);
        for index in 0..extra {
            source.push_str(&format!("let r{index}:Result<I64,I64>=Ok({index});"));
        }
        source.push_str("let x=TryAdd(1,0);");
        if extra == 8 {
            source.push_str("Print(99);");
        }
        source.push('}');
        source
    };
    assert_eq!(run(&program(7)).outcome, Outcome::Unit);
    let mut exact_success = nested_result_array_prefix(false);
    for index in 0..7 {
        exact_success.push_str(&format!("let r{index}:Result<I64,I64>=Ok({index});"));
    }
    exact_success.push_str("let pad=1;let x=TryAdd(1,0);}");
    assert_eq!(run(&exact_success).outcome, Outcome::Unit);
    let execution = run(&program(8));
    assert_eq!(execution.outcome, Outcome::SimulatorFault);
    assert!(execution.observations.stdout().is_empty());
    assert_eq!(execution.diagnostics[0].code, "R0002");
    assert_eq!(
        execution.diagnostics[0].data.get("observed"),
        Some(&DataValue::UInt(65_537))
    );
}

#[test]
fn propagation_moves_noncopyable_wrapper_and_rejects_implicit_copy() {
    let rejected_source = "fn Main()->Result<(),I64>{let r:Result<MockResource,I64>=Ok(MockOpen(4));let x=r?;Inspect(x);return Ok(());}";
    let errors = rejected(rejected_source, true);
    assert_eq!(errors[0].code, "NX-E0201");
    let source = "fn Main()->Result<(),I64>{let r:Result<MockResource,I64>=Ok(MockOpen(4));let x=(move r)?;Inspect(x);return Ok(());}";
    let execution = run(source);
    assert_eq!(execution.outcome, Outcome::Ok);
    let events: Vec<_> = execution
        .observations
        .observations()
        .iter()
        .filter(|o| o.channel == Channel::Event)
        .map(|o| o.text.as_str())
        .collect();
    assert_eq!(events, ["open:0:4", "inspect:0:4", "close:0:4"]);
}

#[test]
fn nested_result_and_argument_propagation_use_one_evaluation() {
    let nested = "fn Main()->Result<(),I64>{let r:Result<Result<I64,I64>,I64>=Ok(Err(5));let x=(r?)?;return Ok(());}";
    assert_eq!(run(nested).outcome, Outcome::Err(5));
    let argument =
        "fn Main()->Result<(),I64>{let x=TryAdd(TryAdd(1,2)?,4)?;AssertEq(x,7);return Ok(());}";
    assert_eq!(run(argument).outcome, Outcome::Ok);
    let branch = "fn Main()->Result<(),I64>{let x=if false then TryAdd(1,2)? else TryAdd(9223372036854775807,1)?;return Ok(());}";
    assert_eq!(run(branch).outcome, Outcome::Err(30));
    let selector =
        "fn Main()->Result<(),I64>{let a=[11];let x=a[TryAdd(0,0)?];AssertEq(x,11);return Ok(());}";
    assert_eq!(run(selector).outcome, Outcome::Ok);
    let union = "fn Main()->Result<(),I64>{let r:Result<I64,I64>=if true then Ok(1) else Err(9);let x=r?;AssertEq(x,1);return Ok(());}";
    assert_eq!(run(union).outcome, Outcome::Ok);
}

#[test]
fn only_active_result_payload_is_destroyed() {
    let ok = run("fn Main(){let r:Result<MockResource,I64>=Ok(MockOpen(1));Drop(move r);}");
    assert_eq!(ok.outcome, Outcome::Unit);
    let events: Vec<_> = ok
        .observations
        .observations()
        .iter()
        .filter(|o| o.channel == Channel::Event)
        .map(|o| o.text.as_str())
        .collect();
    assert_eq!(events, ["open:0:1", "close:0:1"]);
    let err = run("fn Main(){let r:Result<I64,MockResource>=Err(MockOpen(2));Drop(move r);}");
    assert_eq!(err.outcome, Outcome::Unit);
    let events: Vec<_> = err
        .observations
        .observations()
        .iter()
        .filter(|o| o.channel == Channel::Event)
        .map(|o| o.text.as_str())
        .collect();
    assert_eq!(events, ["open:0:2", "close:0:2"]);

    let through_frame = run(
        "fn Make()->Result<I64,MockResource>{return Err(MockOpen(3));}fn Main(){let r=Make();Drop(move r);}",
    );
    assert_eq!(through_frame.outcome, Outcome::Unit);
    let events: Vec<_> = through_frame
        .observations
        .observations()
        .iter()
        .filter(|o| o.channel == Channel::Event)
        .map(|o| o.text.as_str())
        .collect();
    assert_eq!(events, ["open:0:3", "close:0:3"]);
}

#[test]
fn failed_result_expression_suppresses_dependents_and_keeps_siblings() {
    let source = "fn Main()->Result<(),I64>{let x=TryAdd(true,false)?;let y=true+1;return Ok(());}";
    let errors = rejected(source, true);
    let sites: Vec<_> = errors.iter().map(|d| (d.code.as_str(), span(d))).collect();
    assert_eq!(
        sites,
        [
            ("NX-E0102", (39, 43)),
            ("NX-E0102", (44, 49)),
            ("NX-E0102", (58, 62)),
        ]
    );
    assert!(
        errors
            .iter()
            .all(|d| d.subcause.as_deref() == Some("type_mismatch"))
    );
    let constructor = "fn Main()->Result<(),I64>{let r:Result<(),I64>=Ok(AssertEq(1,true));let y=true+1;return Ok(());}";
    let errors = rejected(constructor, true);
    assert_eq!(errors.len(), 2, "{errors:?}");
    assert!(errors.iter().all(|d| d.code == "NX-E0102"));
    assert_eq!(span(&errors[0]), (61, 65));
    assert_eq!(span(&errors[1]), (74, 78));
    let unannotated = "fn Main(){let r=Ok(AssertEq(1,true));}";
    let errors = rejected(unannotated, true);
    assert_eq!(errors.len(), 1, "{errors:?}");
    assert_eq!(errors[0].code, "NX-E0102");
    assert_eq!(span(&errors[0]), (30, 34));
    let dependent =
        "fn Main()->Result<(),I64>{let r=MockOpen(1);let x=Ok(AssertEq(r,r))?;return Ok(());}";
    let errors = rejected(dependent, true);
    assert_eq!(errors.len(), 2, "{errors:?}");
    assert_eq!(
        errors.iter().map(|d| d.code.as_str()).collect::<Vec<_>>(),
        ["NX-E0102", "NX-E0102"]
    );
    assert_eq!(span(&errors[0]), (62, 63));
    assert_eq!(span(&errors[1]), (64, 65));

    let independent = "fn Main()->Result<(),I64>{let x=TryAdd(true,Ok(1))?;return Ok(());}";
    let errors = rejected(independent, true);
    assert_eq!(errors.len(), 2, "{errors:?}");
    assert_eq!(
        (errors[0].code.as_str(), span(&errors[0])),
        ("NX-E0102", (39, 43))
    );
    assert_eq!(
        (errors[1].code.as_str(), span(&errors[1])),
        ("NX-E0113", (44, 49))
    );
    assert_eq!(
        errors[1].data.get("unresolved_components"),
        Some(&DataValue::Array(vec![DataValue::String("$.err".into())]))
    );

    let resource_sibling = "fn Main(){let r=MockOpen(1);let x=TryAdd(AssertEq(r,r),Ok(1));}";
    let errors = rejected(resource_sibling, true);
    assert_eq!(errors.len(), 3, "{errors:?}");
    assert_eq!(
        errors
            .iter()
            .map(|d| (d.code.as_str(), span(d)))
            .collect::<Vec<_>>(),
        [
            ("NX-E0102", (50, 51)),
            ("NX-E0102", (52, 53)),
            ("NX-E0113", (55, 60)),
        ]
    );

    let dependent = "fn Main(){let x=TryAdd(true,false)+1;}";
    let errors = rejected(dependent, true);
    assert_eq!(
        errors
            .iter()
            .map(|d| (d.code.as_str(), span(d)))
            .collect::<Vec<_>>(),
        [("NX-E0102", (23, 27)), ("NX-E0102", (28, 33))]
    );
}

#[test]
fn result_diagnostics_keep_published_data_and_related_sites() {
    type DiagnosticCase<'a> = (
        &'a str,
        &'a [(&'a str, DataValue)],
        &'a [(&'a str, (u64, u64))],
    );
    let cases: &[DiagnosticCase<'_>] = &[
        (
            "fn Main(){let r=Ok(1);}",
            &[(
                "unresolved_components",
                DataValue::Array(vec![DataValue::String("$.err".into())]),
            )],
            &[],
        ),
        (
            "fn Main(){let a:Result<I64,I64>=Ok(1);let b=a<a;}",
            &[
                ("expected_type", DataValue::String("I64".into())),
                ("found_type", DataValue::String("Result<I64,I64>".into())),
            ],
            &[("found_type", (44, 45))],
        ),
        (
            "fn Main(){let a:Result<MockResource,I64>=Ok(MockOpen(1));let b=a==a;}",
            &[
                (
                    "expected_type",
                    DataValue::String("domain:copyable_data".into()),
                ),
                (
                    "found_type",
                    DataValue::String("Result<MockResource,I64>".into()),
                ),
            ],
            &[("found_type", (63, 64))],
        ),
        (
            "fn Main(){let r:Result<MockResource,I64>=Ok(MockOpen(1));let s=r;Drop(s);}",
            &[],
            &[("declaration", (14, 15))],
        ),
        (
            "fn Main(){let r:Result<I64,MockResource>=Ok(1);let s=r;}",
            &[],
            &[("declaration", (14, 15))],
        ),
        (
            "fn Main()->Result<(),I64>{let x=1?;return Ok(());}",
            &[
                ("expected_type", DataValue::String("domain:result".into())),
                ("found_type", DataValue::String("I64".into())),
            ],
            &[("found_type", (32, 33))],
        ),
        (
            "fn Main(){let x=TryAdd(1,2)?;}",
            &[
                ("expected_type", DataValue::String("domain:result".into())),
                ("found_type", DataValue::String("()".into())),
            ],
            &[("found_type", (16, 28))],
        ),
        (
            "fn Main()->Result<(),Bool>{let x=TryAdd(1,2)?;return Ok(());}",
            &[
                ("expected_type", DataValue::String("Bool".into())),
                ("found_type", DataValue::String("I64".into())),
            ],
            &[("expected_type", (11, 26)), ("found_type", (33, 44))],
        ),
    ];
    for (source, expected_data, expected_related) in cases {
        let errors = rejected(source, true);
        assert_eq!(errors.len(), 1, "{source}");
        let d = &errors[0];
        assert_eq!(
            d.data,
            expected_data
                .iter()
                .map(|(k, v)| ((*k).into(), v.clone()))
                .collect(),
            "{source}"
        );
        let related: Vec<_> = d
            .related_spans
            .iter()
            .map(|r| (r.role.as_str(), (r.span.start, r.span.end)))
            .collect();
        assert_eq!(related, *expected_related, "{source}");
        assert!(
            d.unexpected_token.is_none()
                && d.expected_tokens.is_empty()
                && d.parser_context.is_none()
        );
        assert!(d.cause_chain.is_empty() && d.repair.is_empty());
    }
}

#[test]
fn deferred_result_alias_conflict_pins_current_t01_diagnostic() {
    let source = "fn Main(){let r=Err(1);let s:Result<Bool,Bool>=r;let t:Result<I64,I64>=r;}";
    let errors = rejected(source, false);
    assert_eq!(errors.len(), 1);
    assert_eq!(
        (errors[0].code.as_str(), span(&errors[0])),
        ("NX-E0113", (16, 22))
    );
    assert_eq!(
        errors[0].data.get("unresolved_components"),
        Some(&DataValue::Array(vec![DataValue::String("$.ok".into())]))
    );
}

#[test]
fn published_result_call_examples_match_outcomes() {
    let success = "fn Inc(n:view I64)->Result<I64,I64>{return Ok(n+1);}fn Main()->Result<(),I64>{let x=Inc(2)?;return Ok(());}";
    assert_eq!(run(success).outcome, Outcome::Ok);
    let error = "fn Inc(n:view I64)->Result<I64,I64>{return Err(9);}fn Main()->Result<(),I64>{let x=Inc(2)?;return Ok(());}";
    assert_eq!(run(error).outcome, Outcome::Err(9));
}
