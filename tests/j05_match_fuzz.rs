use nexo::interp::{MockEnvironment, Outcome, run_scalar};
use nexo::ir::{CompileError, ReducedProfile, compile_scalar_with_profile};

fn next(seed: &mut u64) -> u64 {
    *seed ^= *seed << 13;
    *seed ^= *seed >> 7;
    *seed ^= *seed << 17;
    *seed
}

fn generated(seed: &mut u64) -> String {
    let depth = (next(seed) % 3 + 1) as usize;
    let resource = next(seed).is_multiple_of(2);
    let variant = next(seed) % 11;
    let ty = (0..depth).fold(
        if resource {
            "MockResource".to_owned()
        } else {
            "I64".to_owned()
        },
        |inner, _| format!("Option<{inner}>"),
    );
    let value = (0..depth).fold(
        if resource {
            "MockOpen(1)".to_owned()
        } else {
            "1".to_owned()
        },
        |inner, _| format!("Some({inner})"),
    );
    let pattern = (0..depth).fold("move r".to_owned(), |inner, _| format!("Some({inner})"));
    let read = if resource { "ReadResource(r)" } else { "r==1" };
    let guard = match variant {
        0 => "true",
        1 => "false",
        2 => read,
        3 => "if true then {value:true;}else{value:false;}",
        4 => "MockUnknown()==()",
        5 => "TryAdd(1,2)==Ok(3)",
        6 => "1",
        7 if resource => "ConsumeResource(move r)",
        7 => "ConsumeNumber(move r)",
        8 => "if true then {return;}else{value:true;}",
        9 => "Len([1])==1",
        10 => "match true{true=>{value:true;},false=>{value:false;}}",
        _ => "true",
    };
    let body = if resource {
        if variant.is_multiple_of(3) {
            "match true{true=>{Inspect(r);},false=>{}}"
        } else {
            "Inspect(r);"
        }
    } else if variant.is_multiple_of(3) {
        "match r==1{true=>{AssertEq(r,1);},false=>{}}"
    } else {
        "AssertEq(r,1);"
    };
    let functions = if resource {
        "fn ReadResource(r:view MockResource)->Bool{return true;}fn ConsumeResource(r:move MockResource)->Bool{return true;}"
    } else {
        "fn ConsumeNumber(r:move I64)->Bool{return true;}"
    };
    format!(
        "{functions}fn Main(){{let o:{ty}={value};match move o{{{pattern} if {guard}=>{{{body}}},{pattern}=>{{{body}}},_=>{{}}}}}}"
    )
}

fn sweep(cases: usize) {
    let mut seed = std::env::var("NEXO_GUARD_FUZZ_SEED")
        .ok()
        .and_then(|value| value.parse::<u64>().ok())
        .unwrap_or(0x4e45_584f_4a30_3531);
    for case in 0..cases {
        let source = generated(&mut seed);
        let checked =
            compile_scalar_with_profile("<fuzz>", &source, ReducedProfile::JNormal, false);
        let runnable =
            compile_scalar_with_profile("<fuzz>", &source, ReducedProfile::JNormal, true);
        match (checked, runnable) {
            (Ok(_), Ok(program)) => {
                let execution = run_scalar(&program, MockEnvironment::default());
                assert!(
                    !matches!(execution.outcome, Outcome::InternalError(_)),
                    "case {case}: {source}"
                );
                for diagnostic in &execution.diagnostics {
                    diagnostic.validate("<fuzz>", &source).unwrap();
                    diagnostic.to_json("<fuzz>", &source).unwrap();
                }
            }
            (Err(CompileError::Rejected(left)), Err(CompileError::Rejected(right))) => {
                assert_eq!(left.len(), right.len(), "case {case}: {source}");
                for (a, b) in left.iter().zip(right.iter()) {
                    a.validate("<fuzz>", &source).unwrap();
                    b.validate("<fuzz>", &source).unwrap();
                    a.to_json("<fuzz>", &source).unwrap();
                    b.to_json("<fuzz>", &source).unwrap();
                    assert_eq!(
                        (&a.code, &a.subcause, &a.primary_span),
                        (&b.code, &b.subcause, &b.primary_span),
                        "case {case}: {source}"
                    );
                }
            }
            (left, right) => panic!("case {case}: {source}\n{left:?}\n{right:?}"),
        }
    }
}

#[test]
fn nested_guarded_match_resource_and_copy_sweep() {
    sweep(500);
}

#[test]
#[ignore = "release robustness campaign with 20,000 generated guarded matches"]
fn generated_guarded_matches_never_panic_or_emit_invalid_diagnostics() {
    sweep(20_000);
}
