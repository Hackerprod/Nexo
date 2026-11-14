mod common;
use nexo::interp::{Outcome, ReducedExecutionProfile, run_with_limits};
use nexo::ir::CompileError;

fn next(seed: &mut u64) -> u64 {
    *seed ^= *seed << 13;
    *seed ^= *seed >> 7;
    *seed ^= *seed << 17;
    *seed
}

fn probe(case: usize, source: &str) {
    let checked = std::panic::catch_unwind(|| common::compile_j("<generator-fuzz>", source, false))
        .unwrap_or_else(|_| panic!("panic in check, case {case}: {source}"));
    let runnable = std::panic::catch_unwind(|| common::compile_j("<generator-fuzz>", source, true))
        .unwrap_or_else(|_| panic!("panic in run compilation, case {case}: {source}"));
    match (checked, runnable) {
        (Ok(_), Ok(program)) => {
            let reduced = match case % 3 {
                0 => ReducedExecutionProfile::JFrames8,
                1 => ReducedExecutionProfile::JCells64,
                _ => ReducedExecutionProfile::JObservations64,
            };
            for profile in [ReducedExecutionProfile::JNormal, reduced] {
                let execution = std::panic::catch_unwind(|| {
                    run_with_limits(&program, Default::default(), profile)
                })
                .unwrap_or_else(|_| panic!("panic in execution, case {case}: {source}"));
                assert!(
                    !matches!(execution.outcome, Outcome::InternalError(_)),
                    "{case}: {source}"
                );
                for diagnostic in &execution.diagnostics {
                    diagnostic.validate("<generator-fuzz>", source).unwrap();
                    diagnostic.to_json("<generator-fuzz>", source).unwrap();
                }
            }
        }
        (Err(CompileError::Rejected(a)), Err(CompileError::Rejected(b))) => {
            assert_eq!(a, b, "{case}: {source}");
            for diagnostic in a {
                diagnostic
                    .validate("<generator-fuzz>", source)
                    .unwrap_or_else(|error| panic!("{case}: {source}\n{diagnostic:?}\n{error:?}"));
                diagnostic.to_json("<generator-fuzz>", source).unwrap();
            }
        }
        (a, b) => panic!("unexpected compiler result, case {case}: {source}\n{a:?}\n{b:?}"),
    }
}

fn source(seed: &mut u64) -> String {
    let (item, delivered) = [
        ("I64", "1"),
        ("Option<I64>", "Some(1)"),
        ("Result<I64,I64>", "Ok(1)"),
        ("MockResource", "MockOpen(1)"),
    ][(next(seed) % 4) as usize];
    let setup = [
        "",
        "let r=MockOpen(2);",
        "var x=0;var f=fn [edit x](){x=1;};",
        "let f=fn [](){};",
    ][(next(seed) % 4) as usize];
    let delivery = [
        format!("yield {delivered};"),
        format!("if true{{yield {delivered};}}"),
        format!("var n=0;while n<2{{yield {delivered};n=n+1;}}"),
        format!("yield {delivered};yield {delivered};"),
        format!("let b=true;match b{{true=>{{yield {delivered};}},false=>{{}}}}"),
        format!("let b=true;match b{{true if b=>{{yield {delivered};}},_=>{{}}}}"),
    ][(next(seed) % 6) as usize]
        .clone();
    let trailer = ["", "Drop(r);", "Drop(f);", "return;"][(next(seed) % 4) as usize];
    let consumer = [
        "var g=G();let a=Iterator.Next(edit g);let b=Iterator.Next(edit g);Drop(a);Drop(b);Drop(g);",
        "for x in G(){Drop(x);}",
        "let g=G();for x in move g{Drop(x);break;}",
        "var g=G();let f=fn [move g]() with no_suspend{var h=move g;let n=Iterator.Next(edit h);};Drop(f);",
        "let g=G();for x in g{Drop(x);}",
        "for x in G(){let b=true;match b{true=>{break;},false=>{}}}",
        "for x in G(){let b=true;match b{true=>{continue;},false=>{}}}",
        "let f=fn [](){for x in G(){let b=true;match b{true=>{break;},false=>{}}}};f();",
    ][(next(seed) % 8) as usize];
    let mut result =
        format!("gen fn G()->{item}{{{setup}{delivery}{trailer}}}fn Main(){{{consumer}}}");
    if next(seed).is_multiple_of(5) {
        let (before, after) = [
            ("yield", "return"),
            ("edit g", "move g"),
            ("for x", "for edit x"),
            ("G()", "G(1)"),
            ("Some(1)", "None"),
            ("fn G", "fn Iterator"),
        ][(next(seed) % 6) as usize];
        result = result.replacen(before, after, 1);
    }
    result
}

#[test]
fn generator_control_resource_for_closure_union_sweep() {
    let mut case = 0;
    for item in ["I64", "Option<I64>", "Result<I64,I64>"] {
        for delivery in [
            "yield 1;",
            "if true{yield 1;}",
            "while false{yield 1;}yield 1;",
        ] {
            for setup in ["", "let r=MockOpen(1);", "let f=fn [](){};"] {
                for consumer in [
                    "for x in G(){Drop(x);}",
                    "var g=G();let n=Iterator.Next(edit g);Drop(n);",
                ] {
                    let source =
                        format!("gen fn G()->{item}{{{setup}{delivery}}}fn Main(){{{consumer}}}");
                    probe(case, &source);
                    case += 1;
                }
            }
        }
    }
    assert_eq!(case, 54);
}

#[test]
fn match_place_nonlocal_edges_across_loops_generators_and_closures() {
    let places = [
        ("let b=true;", "match b", "true", "false"),
        ("var b=true;", "match edit b", "true", "false"),
        ("let b=Some(1);", "match b", "Some(n)", "None"),
        ("var b=Some(1);", "match edit b", "Some(edit n)", "None"),
    ];
    let mut case = 0;
    for (setup, selection, selected, other) in places {
        for guard in ["", " if true"] {
            for wrapper in ["{}", "if true{{{}}}"] {
                for control in ["break;", "continue;", "return;"] {
                    for enclosing in ["while", "for"] {
                        let arm = wrapper.replacen("{}", control, 1);
                        let matched =
                            format!("{selection}{{{selected}{guard}=>{{{arm}}},{other}=>{{}}}}");
                        let source = if enclosing == "while" {
                            format!("fn Main(){{{setup}var i=0;while i<1{{i=i+1;{matched}}}}}")
                        } else {
                            format!(
                                "gen fn G()->I64{{yield 1;}}fn Main(){{{setup}for x in G(){{{matched}}}}}"
                            )
                        };
                        probe(case, &source);
                        case += 1;
                    }
                }
            }
        }
    }
    assert_eq!(case, 96);
    for control in [
        "yield 1;",
        "yield 1;yield 2;",
        "match true{true=>{yield 1;},false=>{}}",
    ] {
        for guard in ["", " if true"] {
            let source = format!(
                "gen fn G()->I64{{let b=true;match b{{true{guard}=>{{{control}}},false=>{{}}}}}}fn Main(){{for x in G(){{Print(x);}}}}"
            );
            probe(case, &source);
            case += 1;
        }
    }
    for control in ["break;", "continue;", "return;"] {
        let source = format!(
            "fn Main(){{let f=fn [](){{let b=true;while true{{match b{{true=>{{{control}}},false=>{{}}}}}}}};f();}}"
        );
        probe(case, &source);
        case += 1;
    }
    assert_eq!(case, 105);
}

#[test]
#[ignore = "20,000 seeded generator, control, resource, closure and union programs; run in release"]
fn seeded_generator_release_campaign() {
    let mut seed = std::env::var("NEXO_GENERATOR_FUZZ_SEED")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(202611141704_u64);
    for case in 0..20_000 {
        let source = source(&mut seed);
        probe(case, &source);
    }
}
