mod common;
use nexo::interp::{Outcome, run_scalar};
use nexo::ir::CompileError;

fn next(seed: &mut u64) -> u64 {
    *seed ^= *seed << 13;
    *seed ^= *seed >> 7;
    *seed ^= *seed << 17;
    *seed
}

fn source(seed: &mut u64) -> String {
    let mode = ["view", "copy", "edit", "move"][(next(seed) % 4) as usize];
    let call = ["f()", "edit f()", "move f()", "Drop(f)"][(next(seed) % 4) as usize];
    let body = ["", "AssertEq(x,1);", "x=2;", "Drop(x);", "Inspect(x);"][(next(seed) % 5) as usize];
    let root = ["1", "MockOpen(1)", "MockBlockingOpen(1)"][(next(seed) % 3) as usize];
    let wrapper = (next(seed) % 5) as usize;
    let value = match wrapper {
        0 => format!("fn [{mode} x](){{{body}}}"),
        1 => format!("Some(fn [{mode} x](){{{body}}})"),
        2 => format!("Ok(fn [{mode} x](){{{body}}})"),
        3 => format!("if true then {{value:fn [{mode} x](){{{body}}};}}else{{value:fn [](){{}};}}"),
        _ => format!("fn [{mode} x](){{let g=fn [view x](){{}};Drop(g);{body}}}"),
    };
    let mut result = format!("fn Main(){{var x={root};var f={value};{call};}}");
    if next(seed).is_multiple_of(7) {
        let alterations = ["copy", "move", "edit", "view", "no_io", "no_alloc", "Some"];
        let choice = alterations[(next(seed) % alterations.len() as u64) as usize];
        result = result.replacen(mode, choice, 1);
    }
    result
}

fn sweep(cases: usize) {
    let mut seed = std::env::var("NEXO_CLOSURE_FUZZ_SEED")
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(202611140914_u64);
    for case in 0..cases {
        let source = source(&mut seed);
        probe(case, &source);
    }
}

fn probe(case: usize, source: &str) {
    let checked = common::compile_j("<closure-fuzz>", source, false);
    let runnable = common::compile_j("<closure-fuzz>", source, true);
    match (checked, runnable) {
        (Ok(_), Ok(program)) => {
            let execution = run_scalar(&program, Default::default());
            assert!(
                !matches!(execution.outcome, Outcome::InternalError(_)),
                "{case}: {source}"
            );
            for diagnostic in &execution.diagnostics {
                diagnostic
                    .validate("<closure-fuzz>", source)
                    .unwrap_or_else(|error| panic!("{case}: {source}\n{diagnostic:?}\n{error:?}"));
                diagnostic.to_json("<closure-fuzz>", source).unwrap();
            }
        }
        (Err(CompileError::Rejected(a)), Err(CompileError::Rejected(b))) => {
            assert_eq!(a, b, "{case}: {source}");
            for diagnostic in a {
                diagnostic
                    .validate("<closure-fuzz>", source)
                    .unwrap_or_else(|error| panic!("{case}: {source}\n{diagnostic:?}\n{error:?}"));
                diagnostic.to_json("<closure-fuzz>", source).unwrap();
            }
        }
        (a, b) => panic!("{case}: {source}\n{a:?}\n{b:?}"),
    }
}

#[test]
fn capture_modes_wrappers_resources_and_calls_cross_product() {
    let mut case = 0;
    for mode in ["view", "copy", "edit", "move"] {
        for root in ["1", "MockOpen(1)", "MockBlockingOpen(1)"] {
            for wrapper in ["", "Some", "Ok", "nested"] {
                for body in ["", "Inspect(x);", "Drop(x);", "x=2;"] {
                    for call in ["f()", "edit f()", "move f()", "Drop(f)"] {
                        let literal = format!("fn [{mode} x](){{{body}}}");
                        let value = match wrapper {
                            "Some" => format!("Some({literal})"),
                            "Ok" => format!("Ok({literal})"),
                            "nested" => format!("fn [{mode} x](){{let g={literal};Drop(g);}}"),
                            _ => literal,
                        };
                        let source = format!("fn Main(){{var x={root};var f={value};{call};}}");
                        probe(case, &source);
                        case += 1;
                    }
                }
            }
        }
    }
    assert_eq!(case, 768);
}

#[test]
fn closure_capture_mode_union_and_nesting_sweep() {
    sweep(320);
}

#[test]
fn nested_capture_permissions_and_union_carriers_cross_product() {
    let mut case = 0;
    for outer in ["view", "copy", "edit", "move"] {
        for inner in ["view", "copy", "edit", "move"] {
            for root in ["1", "MockOpen(1)", "Some(1)"] {
                for body in ["", "Drop(x);", "x=2;"] {
                    for call in ["f()", "edit f()", "move f()"] {
                        for carrier in ["", "Some", "Ok"] {
                            let nested = format!("fn [{inner} x](){{{body}}}");
                            let value = match carrier {
                                "Some" => format!("Some({nested})"),
                                "Ok" => format!("Ok({nested})"),
                                _ => nested,
                            };
                            let source = format!(
                                "fn Main(){{var x={root};var f=fn [{outer} x](){{var g={value};Drop(g);}};{call};}}"
                            );
                            probe(case, &source);
                            case += 1;
                        }
                    }
                }
            }
        }
    }
    assert_eq!(case, 1296);
}

#[test]
fn captured_closure_fields_preserve_call_and_consumption_permissions() {
    let mut case = 0;
    for source_binding in ["let", "var"] {
        for carrier_binding in ["let", "var"] {
            for mode in ["view", "copy", "edit", "move"] {
                for body in ["f();", "edit f();", "move f();", "Drop(f);"] {
                    for call in ["g()", "edit g()", "move g()", "Drop(g)"] {
                        let source = format!(
                            "fn Main(){{{source_binding} f=fn [](){{}};{carrier_binding} g=fn [{mode} f](){{{body}}};{call};}}"
                        );
                        probe(case, &source);
                        case += 1;
                    }
                }
            }
        }
    }
    assert_eq!(case, 256);
}

#[test]
#[ignore = "release robustness campaign of 20,000 generated closure programs"]
fn generated_closures_never_panic_or_emit_invalid_diagnostics() {
    sweep(20_000);
}

#[test]
fn distinct_closure_sites_use_canonical_types_in_diagnostic() {
    let source = "fn Main(){var x=1;var f=if true then {value:fn [view x](){AssertEq(x,1);};}else{value:fn [](){};};edit f();}";
    let Err(CompileError::Rejected(errors)) = common::compile_j("distinct.n", source, true) else {
        panic!("closures from distinct sites have distinct nominal types");
    };
    assert_eq!(errors.len(), 1);
    assert_eq!(errors[0].code, "NX-E1303");
    assert_eq!(
        errors[0].data.get("expected_type"),
        Some(&nexo::diagnostic::DataValue::String("Closure@44".into()))
    );
    assert_eq!(
        errors[0].data.get("found_type"),
        Some(&nexo::diagnostic::DataValue::String("Closure@86".into()))
    );
    errors[0].validate("distinct.n", source).unwrap();
}
