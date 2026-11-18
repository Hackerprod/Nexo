//! Literal F0-0003 §9 pairs and §7 tick witnesses. Expectations are ADR data.
use nexo::{
    interp::{Outcome, run_scalar},
    ir::{CompileError, ReducedProfile, compile_scalar_with_profile},
};
use std::{
    fs,
    process::Command,
    sync::atomic::{AtomicUsize, Ordering},
};

static NEXT: AtomicUsize = AtomicUsize::new(0);
const FILE: &str = "j16-integrated.n";
const PROFILE: &str = "f0-0003-b-v1";

struct Pair {
    id: &'static str,
    bad: &'static str,
    good: &'static str,
    code: &'static str,
    subcause: Option<&'static str>,
    span: (u64, u64),
}

fn report(command: &str, source: &str) -> (i32, String) {
    report_profile(command, source, PROFILE)
}

fn report_profile(command: &str, source: &str, profile: &str) -> (i32, String) {
    let id = NEXT.fetch_add(1, Ordering::Relaxed);
    let path =
        std::env::temp_dir().join(format!("nexo-j16-integrated-{}-{id}.n", std::process::id()));
    fs::write(&path, source).unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_nexo"))
        .args([
            command,
            path.to_str().unwrap(),
            "--profile",
            profile,
            "--format",
            "json",
        ])
        .output()
        .unwrap();
    fs::remove_file(path).unwrap();
    (
        output.status.code().unwrap(),
        String::from_utf8(output.stdout).unwrap(),
    )
}

#[test]
fn adr_section_nine_literal_pairs() {
    let profile = ReducedProfile::from_id(PROFILE).unwrap();
    for pair in PAIRS {
        let accepted = compile_scalar_with_profile(FILE, pair.good, profile, true);
        assert!(accepted.is_ok(), "{} positive: {accepted:?}", pair.id);
        let Err(CompileError::Rejected(errors)) =
            compile_scalar_with_profile(FILE, pair.bad, profile, true)
        else {
            panic!("{} negative was not rejected", pair.id);
        };
        assert_eq!(errors.len(), 1, "{}: {errors:?}", pair.id);
        let error = &errors[0];
        error.validate(FILE, pair.bad).unwrap();
        assert_eq!(error.code, pair.code, "{}", pair.id);
        assert_eq!(error.subcause.as_deref(), pair.subcause, "{}", pair.id);
        if pair.id == "G08" {
            assert_eq!(error.feature_id.as_deref(), Some("generator"));
        }
        let span = error.primary_span.as_ref().unwrap();
        assert_eq!((span.start, span.end), pair.span, "{}", pair.id);
    }
}

#[test]
fn adr_section_seven_ticks_and_check_run() {
    for (id, source, ticks) in TICKS {
        let (status, check) = report("check", source);
        assert_eq!(status, 0, "{id}: {check}");
        assert!(check.contains("\"diagnostics\":[]"), "{id}: {check}");
        let (status, run) = report("run", source);
        assert_eq!(status, 0, "{id}: {run}");
        assert!(run.contains("\"diagnostics\":[]"), "{id}: {run}");
        let program = compile_scalar_with_profile(
            FILE,
            source,
            ReducedProfile::from_id(PROFILE).unwrap(),
            true,
        )
        .unwrap();
        let execution = run_scalar(&program, Default::default());
        assert_eq!(execution.outcome, Outcome::Unit, "{id}");
        assert_eq!(execution.work_consumed, *ticks, "{id}");
    }
}

#[test]
fn consumed_call_once_receiver_precedes_mode_error() {
    const PREFIX: &str = "fn Main(){let r=MockOpen(1);let f=fn [move r](){Drop(r);};";
    let sources = [
        (format!("{PREFIX}move f();f();}}"), vec![("NX-E0202", Some("definitely_consumed"), 67, 68)]),
        (format!("{PREFIX}move f();f();var i=0;while i<1{{i=i+1;}}}}"), vec![("NX-E0202", Some("definitely_consumed"), 67, 68)]),
        ("fn Main(){var i=0;while i<1{i=i+1;}let r=MockOpen(1);let f=fn [move r](){Drop(r);};move f();f();}".into(), vec![("NX-E0202", Some("definitely_consumed"), 92, 93)]),
        (format!("{PREFIX}if true{{move f();}}f();}}"), vec![("NX-E0202", Some("maybe_consumed"), 76, 77)]),
        (format!("{PREFIX}move f();return;f();}}"), vec![("NX-E0202", Some("definitely_consumed"), 74, 75)]),
        (format!("{PREFIX}var i=0;while i<2{{move f();f();i=i+1;}}}}"), vec![("NX-E0202", Some("maybe_consumed"), 76, 84), ("NX-E0202", Some("definitely_consumed"), 85, 86)]),
        (format!("{PREFIX}var i=0;while i<1{{move f();i=i+1;}}f();}}"), vec![("NX-E0202", Some("maybe_consumed"), 76, 84), ("NX-E0202", Some("maybe_consumed"), 92, 93)]),
        (format!("{PREFIX}move f();f();f();}}"), vec![("NX-E0202", Some("definitely_consumed"), 67, 68), ("NX-E0202", Some("definitely_consumed"), 71, 72)]),
        (format!("{PREFIX}let g=move f;f();}}"), vec![("NX-E0202", Some("definitely_consumed"), 71, 72)]),
        ("fn Main(){let r=MockOpen(1);var f=fn [move r](){Drop(r);};move f();edit f();}".into(), vec![("NX-E0202", Some("definitely_consumed"), 72, 73)]),
        ("fn Main(){let r=MockOpen(1);var f=fn [move r](){Drop(r);};edit f();}".into(), vec![("NX-E1107", None, 63, 64)]),
        ("fn Main(){let r=MockOpen(1);let f=fn [move r](){Drop(r);};edit f();}".into(), vec![("NX-E1107", None, 63, 64)]),
        ("fn Main(){let r=MockOpen(1);var f=fn [move r](){Drop(r);};let h=fn [edit f](){};f();Drop(h);}".into(), vec![("NX-E1105", Some("active_borrow_conflict"), 80, 81), ("NX-E1107", None, 80, 81)]),
        (format!("{PREFIX}let g=f;g();f();}}"), vec![("NX-E0201", None, 64, 65), ("NX-E1107", None, 66, 67), ("NX-E1107", None, 70, 71)]),
        (format!("{PREFIX}f();move f();f();}}"), vec![("NX-E1107", None, 58, 59), ("NX-E0202", Some("definitely_consumed"), 71, 72)]),
        (format!("{PREFIX}f();move f();f();var i=0;while i<1{{i=i+1;}}}}"), vec![("NX-E1107", None, 58, 59), ("NX-E0202", Some("definitely_consumed"), 71, 72)]),
        (format!("{PREFIX}f();f();}}"), vec![("NX-E1107", None, 58, 59), ("NX-E1107", None, 62, 63)]),
        (format!("{PREFIX}f();}}"), vec![("NX-E1107", None, 58, 59)]),
    ];
    let profile = ReducedProfile::from_id(PROFILE).unwrap();
    for (source, expected) in sources {
        let Err(CompileError::Rejected(errors)) =
            compile_scalar_with_profile(FILE, &source, profile, true)
        else {
            panic!("expected rejection: {source}");
        };
        let actual: Vec<_> = errors
            .iter()
            .map(|d| {
                d.validate(FILE, &source).unwrap();
                let span = d.primary_span.as_ref().unwrap();
                (d.code.as_str(), d.subcause.as_deref(), span.start, span.end)
            })
            .collect();
        assert_eq!(actual, expected, "{source}");
        if source == format!("{PREFIX}move f();f();}}") {
            let d = &errors[0];
            let full = d.to_json(FILE, &source).unwrap();
            assert!(full.contains("\"stage\":\"ownership\",\"message\":"));
            assert!(full.contains("\"operation\":\"ownership.use\""));
            assert!(full.contains(
                "\"primary_span\":{\"file\":\"j16-integrated.n\",\"start\":67,\"end\":68}"
            ));
            assert!(full.contains("\"role\":\"declaration\",\"span\":{\"file\":\"j16-integrated.n\",\"start\":32,\"end\":33}"));
            assert!(full.contains("\"role\":\"consumption\",\"span\":{\"file\":\"j16-integrated.n\",\"start\":58,\"end\":66}"));
            assert!(full.contains("\"operation\":\"value.consume\""));
            assert!(full.contains("\"entity\":{\"kind\":\"binding\",\"id\":\"binding:32\"}"));
            assert!(full.contains("\"evidence\":\"verified\""));
            assert!(full.ends_with("\"data\":{}}"));
        }
        for action in ["check", "run"] {
            let (status, output) = report(action, &source);
            assert_eq!(status, 1, "{source}: {output}");
            assert!(output.contains("\"events\":[]"), "{source}: {output}");
            assert!(output.contains("\"observations\":[]"), "{source}: {output}");
        }
    }
    let accepted = format!("{PREFIX}move f();}}");
    assert!(compile_scalar_with_profile(FILE, &accepted, profile, true).is_ok());
}

#[test]
fn hypothetical_call_releases_its_receiver_loan() {
    use nexo::diagnostic::{DataValue, Operation, RelatedRole, Stage};

    const PREFIX: &str = "fn Main(){let r=MockOpen(1);let f=fn [move r](){Drop(r);};";
    let rejected = [
        (format!("{PREFIX}return;f();move f();}}"), (65, 66)),
        (format!("{PREFIX}return;f();Drop(f);}}"), (65, 66)),
        (
            format!("{PREFIX}var i=0;while i<1{{break;f();move f();}}}}"),
            (82, 83),
        ),
        (
            format!("{PREFIX}var i=0;while i<1{{continue;f();move f();}}}}"),
            (85, 86),
        ),
        (
            format!("{PREFIX}if true{{return;f();move f();}}}}"),
            (73, 74),
        ),
        (
            format!("{PREFIX}var i=0;while i<1{{break;f();Drop(f);}}}}"),
            (82, 83),
        ),
    ];
    let profile = ReducedProfile::from_id(PROFILE).unwrap();
    for (source, expected_span) in rejected {
        let Err(CompileError::Rejected(errors)) =
            compile_scalar_with_profile(FILE, &source, profile, true)
        else {
            panic!("expected mode rejection: {source}");
        };
        assert_eq!(errors.len(), 1, "{source}: {errors:?}");
        let error = &errors[0];
        error.validate(FILE, &source).unwrap();
        assert_eq!(error.code, "NX-E1107", "{source}");
        assert_eq!(error.subcause, None, "{source}");
        assert_eq!(error.stage, Stage::Ownership, "{source}");
        assert_eq!(error.operation, Operation::OwnershipCheck, "{source}");
        let span = error.primary_span.as_ref().unwrap();
        assert_eq!((span.start, span.end), expected_span, "{source}");
        assert_eq!(error.related_spans.len(), 1, "{source}");
        assert_eq!(
            error.related_spans[0].role,
            RelatedRole::Declaration,
            "{source}"
        );
        let declaration = &error.related_spans[0].span;
        assert_eq!((declaration.start, declaration.end), (32, 33), "{source}");
        assert!(error.cause_chain.is_empty(), "{source}");
        assert_eq!(
            error.data,
            std::collections::BTreeMap::from([
                ("requested_mode".into(), DataValue::String("read".into())),
                ("required_mode".into(), DataValue::String("consume".into())),
            ]),
            "{source}"
        );
        for action in ["check", "run"] {
            let (status, output) = report(action, &source);
            assert_eq!(status, 1, "{source}: {output}");
            assert_eq!(
                output.matches("\"code\":\"NX-E").count(),
                1,
                "{source}: {output}"
            );
            assert!(output.contains("\"observations\":[]"), "{source}: {output}");
        }
    }

    let accepted =
        "fn Main(){let r=MockOpen(1);let f=fn [move r](){Inspect(r);};return;f();move f();}";
    assert!(compile_scalar_with_profile(FILE, accepted, profile, true).is_ok());
    for action in ["check", "run"] {
        let (status, output) = report(action, accepted);
        assert_eq!(status, 0, "{output}");
        assert!(output.contains("\"diagnostics\":[]"), "{output}");
    }
}

#[test]
fn callonce_mode_error_survives_ownership_flow_limit() {
    // The pre-B3 base reported only NX-E1107 for this source and profile.
    const FLOW_PROFILE: &str = "f0-0003-b-test-flow-32-v1";
    let source = format!(
        "fn Main(){{let r=MockOpen(1);let f=fn [move r](){{Drop(r);}};f();{}}}",
        (0..6)
            .map(|i| format!("var i{i}=0;while i{i}<1{{i{i}=i{i}+1;}}"))
            .collect::<String>()
    );
    let Err(CompileError::Rejected(errors)) = compile_scalar_with_profile(
        FILE,
        &source,
        ReducedProfile::from_id(FLOW_PROFILE).unwrap(),
        true,
    ) else {
        panic!("expected mode and flow limit diagnostics");
    };
    let actual: Vec<_> = errors
        .iter()
        .map(|d| {
            d.validate(FILE, &source).unwrap();
            let span = d.primary_span.as_ref().unwrap();
            (d.code.as_str(), d.subcause.as_deref(), span.start, span.end)
        })
        .collect();
    assert_eq!(
        actual,
        [("NX-E1107", None, 58, 59), ("NX-E0003", None, 117, 118)]
    );
    assert_eq!(
        errors[1].data.get("budget_kind"),
        Some(&nexo::diagnostic::DataValue::String(
            "ownership_flow_steps".into()
        ))
    );
    for action in ["check", "run"] {
        let (status, output) = report_profile(action, &source, FLOW_PROFILE);
        assert_eq!(status, 1, "{output}");
        let first = output.find("\"code\":\"NX-E1107\"").unwrap();
        let second = output.find("\"code\":\"NX-E0003\"").unwrap();
        assert!(first < second, "{output}");
    }
}

#[test]
fn callonce_immutable_edit_reports_mode_before_mutability() {
    let source = "fn Main(){let r=MockOpen(1);let f=fn [move r](){Drop(r);};edit f();}";
    let Err(CompileError::Rejected(errors)) = compile_scalar_with_profile(
        FILE,
        source,
        ReducedProfile::from_id(PROFILE).unwrap(),
        true,
    ) else {
        panic!("expected mode rejection");
    };
    let actual: Vec<_> = errors
        .iter()
        .map(|d| {
            d.validate(FILE, source).unwrap();
            let span = d.primary_span.as_ref().unwrap();
            (d.code.as_str(), d.subcause.as_deref(), span.start, span.end)
        })
        .collect();
    assert_eq!(actual, [("NX-E1107", None, 63, 64)]);
}

#[test]
fn callonce_mode_error_survives_other_receiver_error() {
    let source = "fn Main(){let r=MockOpen(1);var f=fn [move r](){Drop(r);};let h=fn [edit f](){};f();Drop(h);}";
    let Err(CompileError::Rejected(errors)) = compile_scalar_with_profile(
        FILE,
        source,
        ReducedProfile::from_id(PROFILE).unwrap(),
        true,
    ) else {
        panic!("expected receiver and mode rejection");
    };
    let actual: Vec<_> = errors
        .iter()
        .map(|d| {
            d.validate(FILE, source).unwrap();
            let span = d.primary_span.as_ref().unwrap();
            (d.code.as_str(), d.subcause.as_deref(), span.start, span.end)
        })
        .collect();
    assert_eq!(
        actual,
        [
            ("NX-E1105", Some("active_borrow_conflict"), 80, 81),
            ("NX-E1107", None, 80, 81)
        ]
    );
}

const PAIRS: &[Pair] = &[
    Pair {
        id: "C01",
        bad: "fn Main(){let x=1;let f=fn []()->I64{return x;};}",
        good: "fn Main(){let x=1;let f=fn [copy x]()->I64{return x;};}",
        code: "NX-E1101",
        subcause: None,
        span: (44, 45),
    },
    Pair {
        id: "C02",
        bad: "fn Main(){let x=1;let f=fn [copy x,copy x](){};}",
        good: "fn Main(){let x=1;let f=fn [copy x](){};}",
        code: "NX-E1102",
        subcause: None,
        span: (40, 41),
    },
    Pair {
        id: "C03",
        bad: "fn Main(){let r=MockOpen(1);let f=fn [copy r](){};}",
        good: "fn Main(){let r=MockOpen(1);let f=fn [move r](){};}",
        code: "NX-E0201",
        subcause: None,
        span: (43, 44),
    },
    Pair {
        id: "C04",
        bad: "fn Main(){var x=1;let f=fn [view x](){};x=2;Drop(f);}",
        good: "fn Main(){var x=1;let f=fn [view x](){};Drop(f);x=2;}",
        code: "NX-E1105",
        subcause: Some("active_borrow_conflict"),
        span: (40, 41),
    },
    Pair {
        id: "C05",
        bad: "fn Main(){var x=1;var f=fn [edit x](){x=2;};f();}",
        good: "fn Main(){var x=1;var f=fn [edit x](){x=2;};edit f();}",
        code: "NX-E1107",
        subcause: None,
        span: (44, 45),
    },
    Pair {
        id: "C06",
        bad: "fn Main(){let f=if true then {let x=1;value:fn [view x]()->I64{return x;};}else{return;};Drop(f);}",
        good: "fn Main(){let f=if true then {let x=1;value:fn [copy x]()->I64{return x;};}else{return;};Drop(f);}",
        code: "NX-E1104",
        subcause: None,
        span: (44, 73),
    },
    Pair {
        id: "C07",
        bad: "fn Main(){let f=fn [](x:view){return x;};}",
        good: "fn Main(){let f=fn [](x:view I64){return x;};}",
        code: "NX-E1111",
        subcause: None,
        span: (22, 23),
    },
    Pair {
        id: "C08",
        bad: "fn Main(){let f=fn [](){};move f();f();}",
        good: "fn Main(){let f=fn [](){};move f();}",
        code: "NX-E0202",
        subcause: Some("definitely_consumed"),
        span: (35, 36),
    },
    Pair {
        id: "C09",
        bad: "fn Main(){let r=MockOpen(1);let f=fn [move r](){Drop(r);};f();}",
        good: "fn Main(){let r=MockOpen(1);let f=fn [move r](){Drop(r);};move f();}",
        code: "NX-E1107",
        subcause: None,
        span: (58, 59),
    },
    Pair {
        id: "E01",
        bad: "fn Main(){let a=[1];let f=fn [copy a]() with {create(no_alloc),call(unrestricted)}{};}",
        good: "fn Main(){let a=[1];let f=fn [move a]() with {create(no_alloc),call(unrestricted)}{};}",
        code: "NX-E0412",
        subcause: None,
        span: (26, 28),
    },
    Pair {
        id: "E02",
        bad: "fn Main(){let a=[1];let f=fn [move a]() with {copy(no_alloc),call(unrestricted)}{};}",
        good: "fn Main(){let a=1;let f=fn [move a]() with {copy(no_alloc),call(unrestricted)}{};}",
        code: "NX-E0412",
        subcause: None,
        span: (26, 28),
    },
    Pair {
        id: "E03",
        bad: "fn Main(){let r=MockBlockingOpen(1);let f=fn [move r]() with {drop(no_block),call(unrestricted)}{};}",
        good: "fn Main(){let r=MockOpen(1);let f=fn [move r]() with {drop(no_block),call(unrestricted)}{};}",
        code: "NX-E0413",
        subcause: None,
        span: (42, 44),
    },
    Pair {
        id: "E04",
        bad: "fn Main(){let r=MockOpen(1);let f=fn [move r]() with {drop(no_io),call(unrestricted)}{};}",
        good: "fn Main(){let r=1;let f=fn [move r]() with {drop(no_io),call(unrestricted)}{};}",
        code: "NX-E0415",
        subcause: None,
        span: (34, 36),
    },
    Pair {
        id: "E05",
        bad: "gen fn G()->I64{yield 1;}fn Main(){let g=G();let f=fn [move g]() with no_suspend{var h=move g;let n=Iterator.Next(edit h);};}",
        good: "gen fn G()->I64{}fn Main(){let g=G();let f=fn [move g]() with no_suspend{var h=move g;let n=Iterator.Next(edit h);};}",
        code: "NX-E0414",
        subcause: None,
        span: (109, 113),
    },
    Pair {
        id: "F01",
        bad: "gen fn G()->I64{yield 1;}fn Main(){let g=G();for x in g{Print(x);}}",
        good: "gen fn G()->I64{yield 1;}fn Main(){let g=G();for x in move g{Print(x);}}",
        code: "NX-E0201",
        subcause: None,
        span: (54, 55),
    },
    Pair {
        id: "F02",
        bad: "fn Main(){break;}",
        good: "fn Main(){while true{break;}}",
        code: "NX-E1326",
        subcause: Some("break_outside_loop"),
        span: (10, 15),
    },
    Pair {
        id: "F03",
        bad: "fn Main(){continue;}",
        good: "fn Main(){while false{continue;}}",
        code: "NX-E1326",
        subcause: Some("continue_outside_loop"),
        span: (10, 18),
    },
    Pair {
        id: "F04",
        bad: "gen fn G()->I64{yield 1;}fn Main() with no_suspend{for x in G(){}}",
        good: "gen fn G()->I64{}fn Main() with no_suspend{for x in G(){}}",
        code: "NX-E0414",
        subcause: None,
        span: (51, 54),
    },
    Pair {
        id: "G01",
        bad: "gen fn G()->I64{return 1;}fn Main(){}",
        good: "gen fn G()->I64{return;}fn Main(){}",
        code: "NX-E1324",
        subcause: None,
        span: (16, 22),
    },
    Pair {
        id: "G02",
        bad: "fn Main(){yield 1;}",
        good: "gen fn G()->I64{yield 1;}fn Main(){}",
        code: "NX-E1213",
        subcause: None,
        span: (10, 15),
    },
    Pair {
        id: "G03",
        bad: "gen fn G()->Result<I64,I64>{let x=TryAdd(1,2)?;yield Ok(x);}fn Main(){}",
        good: "gen fn G()->Result<I64,I64>{yield TryAdd(1,2);}fn Main(){}",
        code: "NX-E1214",
        subcause: None,
        span: (45, 46),
    },
    Pair {
        id: "G04",
        bad: "gen fn G(r:view MockResource)->I64{Inspect(r);yield 1;}fn Main(){}",
        good: "gen fn G(r:move MockResource)->I64{Inspect(r);yield 1;}fn Main(){}",
        code: "NX-E1110",
        subcause: None,
        span: (11, 15),
    },
    Pair {
        id: "G05",
        bad: "gen fn G()->I64{var x=0;var f=fn [edit x](){x=1;};yield 1;edit f();}fn Main(){}",
        good: "gen fn G()->I64{var x=0;var f=fn [edit x](){x=1;};edit f();Drop(f);yield 1;}fn Main(){}",
        code: "NX-E1204",
        subcause: None,
        span: (50, 55),
    },
    Pair {
        id: "G06",
        bad: "gen fn G()->I64{yield 1;}fn Main() with no_suspend{var g=G();let x=Iterator.Next(edit g);}",
        good: "gen fn G()->I64{yield 1;}fn Main(){var g=G();let x=Iterator.Next(edit g);}",
        code: "NX-E0414",
        subcause: None,
        span: (76, 80),
    },
    Pair {
        id: "G07",
        bad: "gen fn G()->I64 with no_suspend{yield 1;}fn Main(){}",
        good: "gen fn G()->I64{yield 1;}fn Main(){}",
        code: "NX-E0414",
        subcause: None,
        span: (32, 37),
    },
    Pair {
        id: "G08",
        bad: "gen fn G()->I64{let g=G();}fn Main(){}",
        good: "gen fn G()->I64{let g=H();}gen fn H()->I64{}fn Main(){}",
        code: "NX-E0006",
        subcause: None,
        span: (7, 8),
    },
    Pair {
        id: "L01",
        bad: "gen fn G()->I64{yield 1;}fn Main(){for edit x in G(){}}",
        good: "gen fn G()->I64{yield 1;}fn Main(){for x in G(){}}",
        code: "NX-E0006",
        subcause: None,
        span: (39, 43),
    },
    Pair {
        id: "L02",
        bad: "fn Main(){let f=fn [](){};let a=[f];}",
        good: "fn Main(){let f=fn [](){};let a=Some(f);}",
        code: "NX-E0006",
        subcause: None,
        span: (32, 35),
    },
    Pair {
        id: "M01",
        bad: "fn Main(){let o:Option<I64>=Some(1);match o{Some(n)=>{}}}",
        good: "fn Main(){let o:Option<I64>=Some(1);match o{Some(n)=>{},None=>{}}}",
        code: "NX-E1304",
        subcause: Some("non_exhaustive"),
        span: (36, 41),
    },
    Pair {
        id: "M02",
        bad: "fn Main(){match true{true=>{},false=>{},_=>{}}}",
        good: "fn Main(){match true{true=>{},false=>{}}}",
        code: "NX-E1304",
        subcause: Some("unreachable_arm"),
        span: (40, 41),
    },
    Pair {
        id: "M03",
        bad: "fn Main(){let x=match true{true=>{value:1;},false=>{value:false;}};}",
        good: "fn Main(){let x=match true{true=>{value:1;},false=>{value:2;}};}",
        code: "NX-E1303",
        subcause: None,
        span: (58, 63),
    },
    Pair {
        id: "M04",
        bad: "fn Main(){let o:Option<I64>=Some(1);match o{Some(a,b)=>{},None=>{}}}",
        good: "fn Main(){let o:Option<I64>=Some(1);match o{Some(a)=>{},None=>{}}}",
        code: "NX-E1325",
        subcause: Some("constructor_arity"),
        span: (44, 53),
    },
    Pair {
        id: "M05",
        bad: "fn Main(){let o:Option<I64>=Some(1);match o{Ok(n)=>{},_=>{}}}",
        good: "fn Main(){let o:Option<I64>=Some(1);match o{Some(n)=>{},_=>{}}}",
        code: "NX-E1325",
        subcause: Some("constructor_type"),
        span: (44, 49),
    },
    Pair {
        id: "M06",
        bad: "fn Main(){var o:Option<I64>=Some(1);match o{Some(edit n)=>{},None=>{}}}",
        good: "fn Main(){var o:Option<I64>=Some(1);match edit o{Some(edit n)=>{},None=>{}}}",
        code: "NX-E1105",
        subcause: Some("permission_escalation"),
        span: (49, 55),
    },
    Pair {
        id: "M07",
        bad: "fn Main(){let o:Option<I64>=Some(1);match o{Some(x,x)=>{},None=>{}}}",
        good: "fn Main(){let o:Option<I64>=Some(1);match o{Some(x)=>{},None=>{}}}",
        code: "NX-E0104",
        subcause: None,
        span: (51, 52),
    },
    Pair {
        id: "M08",
        bad: "fn Main(){let o:Option<I64>=Some(1);match o{Missing(x)=>{},None=>{}}}",
        good: "fn Main(){let o:Option<I64>=Some(1);match o{Some(x)=>{},None=>{}}}",
        code: "NX-E0101",
        subcause: None,
        span: (44, 51),
    },
    Pair {
        id: "M09",
        bad: "fn F(b:view Bool){let r=MockOpen(1);match b{true=>{Drop(r);},false=>{}}Inspect(r);}fn Main(){}",
        good: "fn F(b:view Bool){let r=MockOpen(1);match b{true=>{Inspect(r);},false=>{}}Inspect(r);}fn Main(){}",
        code: "NX-E0202",
        subcause: Some("maybe_consumed"),
        span: (79, 80),
    },
    Pair {
        id: "N01",
        bad: "fn Main(){var x=1;let n=Iterator.Next(edit x);}",
        good: "gen fn G()->I64{}fn Main(){var x=G();let n=Iterator.Next(edit x);}",
        code: "NX-E0102",
        subcause: Some("type_mismatch"),
        span: (38, 44),
    },
    Pair {
        id: "O01",
        bad: "fn Main(){let o=None;}",
        good: "fn Main(){let o:Option<I64>=None;}",
        code: "NX-E0113",
        subcause: None,
        span: (16, 20),
    },
    Pair {
        id: "O02",
        bad: "fn Main()->Result<(),I64>{let o:Option<I64>=Some(1);let x=o?;return Ok(());}",
        good: "fn Main()->Result<(),I64>{let o:Option<I64>=Some(1);let x=TryAdd(1,2)?;return Ok(());}",
        code: "NX-E0102",
        subcause: Some("type_mismatch"),
        span: (59, 60),
    },
    Pair {
        id: "Q01",
        bad: "fn Main(){match true{true if MockUnknown()==()=>{},_=>{}}}",
        good: "fn Main(){match true{true if true=>{},_=>{}}}",
        code: "NX-E1305",
        subcause: Some("guard_effect"),
        span: (29, 40),
    },
    Pair {
        id: "Q02",
        bad: "fn P(x:edit I64)->Bool{return true;}fn Main(){var x=0;match true{true if P(edit x)=>{},_=>{}}}",
        good: "fn P(x:edit I64)->Bool{return true;}fn Main(){var x=0;match true{true if true=>{},_=>{}}}",
        code: "NX-E1305",
        subcause: Some("guard_mutation"),
        span: (75, 79),
    },
    Pair {
        id: "Q03",
        bad: "fn P(x:move I64)->Bool{return true;}fn Main(){let x=0;match true{true if P(move x)=>{},_=>{}}}",
        good: "fn P(x:move I64)->Bool{return true;}fn Main(){let x=0;match true{true if true=>{},_=>{}}}",
        code: "NX-E1305",
        subcause: Some("guard_transfer"),
        span: (75, 79),
    },
    Pair {
        id: "Q04",
        bad: "fn Take(r:move MockResource)->Bool{Drop(r);return true;}fn Main(){let r=MockOpen(1);match true{true if Take(move r)=>{},_=>{}}Inspect(r);}",
        good: "fn Take(r:view MockResource)->Bool{return true;}fn Main(){let r=MockOpen(1);match true{true if Take(r)=>{},_=>{}}Inspect(r);}",
        code: "NX-E1305",
        subcause: Some("guard_transfer"),
        span: (108, 112),
    },
    Pair {
        id: "Q06",
        bad: "fn Main()->Result<(),I64>{match true{true if TryAdd(1,2)?==3=>{},_=>{}}return Ok(());}",
        good: "fn Main()->Result<(),I64>{match true{true if TryAdd(1,2)==Ok(3)=>{},_=>{}}return Ok(());}",
        code: "NX-E1305",
        subcause: Some("guard_control"),
        span: (56, 57),
    },
    Pair {
        id: "Q07",
        bad: "fn Main(){match true{true if if true then {return;}else{value:true;}=>{},_=>{}}}",
        good: "fn Main(){match true{true if if true then {value:true;}else{value:true;}=>{},_=>{}}}",
        code: "NX-E1305",
        subcause: Some("guard_control"),
        span: (43, 49),
    },
];

const TICKS: &[(&str, &str, u64)] = &[
    ("T01", "fn Main(){let x:Option<I64>=Some(1);}", 9),
    ("T02", "fn Main(){let x:Option<I64>=None;}", 5),
    ("T03", "fn Main(){let f=fn [](){};f();Drop(f);}", 15),
    (
        "T04",
        "gen fn G()->I64{}fn Main(){var g=G();let n=Iterator.Next(edit g);Drop(n);Drop(g);}",
        24,
    ),
    (
        "T05",
        "gen fn G()->I64{yield 1;}fn Main(){var g=G();let n=Iterator.Next(edit g);Drop(n);Drop(g);}",
        27,
    ),
    ("T06", "gen fn G()->I64{}fn Main(){for x in G(){}}", 12),
    (
        "T07",
        "gen fn G()->I64{yield 1;}fn Main(){for x in G(){}}",
        20,
    ),
    (
        "T08",
        "gen fn G()->I64{yield 1;}fn Main(){for x in G(){break;}}",
        16,
    ),
    (
        "T09",
        "gen fn G()->I64{yield 1;}fn Main(){for x in G(){continue;}}",
        21,
    ),
    (
        "T10",
        "gen fn G(n:view I64)->I64{}fn Main(){let g=G(1);Drop(g);}",
        15,
    ),
    (
        "T11",
        "fn Main(){let x=1;let f=fn [copy x](){};Drop(f);}",
        14,
    ),
    (
        "T12",
        "fn Main(){let x=1;let f=fn [view x](){};Drop(f);}",
        14,
    ),
    ("T13", "fn Main(){let f=fn [](){};move f();}", 11),
    (
        "T14",
        "gen fn G()->I64{}fn Main(){var g=G();let a=Iterator.Next(edit g);Drop(a);let b=Iterator.Next(edit g);Drop(b);Drop(g);}",
        36,
    ),
];
