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
    let mode = next(seed) % 3;
    let defect = next(seed) % 9;
    let mut ty = if resource {
        "MockResource".to_owned()
    } else {
        "I64".to_owned()
    };
    for _ in 0..depth {
        ty = format!("Option<{ty}>");
    }
    let leaf = if resource { "MockOpen(1)" } else { "1" };
    let none_at = (next(seed) % (depth as u64 + 1)) as usize;
    let mut value = if none_at == 0 {
        "None".to_owned()
    } else {
        leaf.to_owned()
    };
    for _ in 0..none_at {
        value = format!("Some({value})");
    }
    let binding = match (mode, defect % 5) {
        (1, 0) | (2, 0) => "move r",
        (0, 1) | (2, 1) => "edit r",
        _ => "r",
    };
    let mut pattern = binding.to_owned();
    for _ in 0..depth {
        pattern = format!("Some({pattern})");
    }
    if defect == 6 {
        pattern = pattern.replacen("Some", "Missing", 1);
    }
    let action = if resource {
        if binding == "move r" {
            "Inspect(r);"
        } else if binding == "edit r" {
            "r=MockOpen(2);"
        } else {
            "Inspect(r);"
        }
    } else if binding == "edit r" {
        "r=2;"
    } else {
        "let q=r;"
    };
    let mode_text = ["", "move ", "edit "][mode as usize];
    let mut source = format!(
        "fn Main(){{var o:{ty}={value};match {mode_text}o{{{pattern}=>{{{action}}},_=>{{}}}}}}",
    );
    if defect == 7 {
        source = source.replacen(",_=>{}", "", 1);
    }
    if defect == 8 {
        source.push('@');
    }
    source
}

fn sweep(cases: usize) {
    let mut seed = std::env::var("NEXO_MATCH_FUZZ_SEED")
        .ok()
        .and_then(|value| value.parse::<u64>().ok())
        .unwrap_or(0x65d4_3fe1_720a_9c85_u64);
    for case in 0..cases {
        let source = generated(&mut seed);
        let checked =
            compile_scalar_with_profile("<fuzz>", &source, ReducedProfile::JNormal, false);
        let runnable =
            compile_scalar_with_profile("<fuzz>", &source, ReducedProfile::JNormal, true);
        match (checked, runnable) {
            (Ok(_), Ok(program)) => {
                let result = run_scalar(&program, MockEnvironment::default());
                assert!(
                    !matches!(result.outcome, Outcome::InternalError(_)),
                    "case {case}: {source}"
                );
            }
            (Err(CompileError::Rejected(left)), Err(CompileError::Rejected(right))) => {
                assert_eq!(left.len(), right.len(), "case {case}: {source}");
                for (a, b) in left.iter().zip(right.iter()) {
                    a.validate("<fuzz>", &source).unwrap();
                    b.validate("<fuzz>", &source).unwrap();
                    // The CLI substitutes `null` only when diagnostic JSON
                    // serialization rejects a malformed diagnostic.
                    a.to_json("<fuzz>", &source).unwrap();
                    b.to_json("<fuzz>", &source).unwrap();
                    assert_eq!(
                        (&a.code, &a.subcause, &a.primary_span),
                        (&b.code, &b.subcause, &b.primary_span),
                        "case {case}: {source}"
                    );
                }
            }
            (left, right) => {
                panic!("check/run disagreement at case {case}: {source}\n{left:?}\n{right:?}")
            }
        }
    }
}

#[test]
fn nested_option_match_resource_and_mode_sweep() {
    sweep(500);
}

#[test]
#[ignore = "release robustness campaign with 20,000 generated sources"]
fn generated_match_sources_never_panic_or_escape_as_internal_errors() {
    sweep(20_000);
}

#[test]
#[ignore = "release campaign for temporary and subdivided match patterns"]
fn generated_temporary_and_subdivided_matches_never_escape_as_internal_errors() {
    let mut seed = std::env::var("NEXO_MATCH_FUZZ_SEED")
        .ok()
        .and_then(|value| value.parse::<u64>().ok())
        .unwrap_or(0xc0a5_1347_209b_1123);
    for case in 0..20_000 {
        let value = next(&mut seed) % 16;
        let source = match next(&mut seed) % 8 {
            0 => format!("fn Main(){{match Some({value}){{Some({value})=>{{}},_=>{{}}}}}}"),
            1 => format!("fn Main(){{match Some(Some({value})){{Some(Some(n))=>{{}},_=>{{}}}}}}"),
            2 => "fn Main(){let x:Result<Bool,Bool>=Ok(true);match x{Ok(true)=>{},Ok(false)=>{},Err(e)=>{}}}".to_string(),
            3 => format!(
                "fn Main(){{let x:Option<I64>=Some({value});match x{{MockOpen(n)=>{{}},_=>{{}}}}}}"
            ),
            4 => format!(
                "fn K()->I64{{return 5;}}fn Main(){{let x:Option<I64>=Some({value});match x{{Some(K)=>{{Print(K);}},None=>{{}}}}}}"
            ),
            5 => format!("fn Main(){{match Ok({value}){{Ok({value})=>{{}},_=>{{}}}}}}"),
            6 => format!("fn Main(){{match None{{Some({value})=>{{}},_=>{{}}}}}}"),
            _ => "fn Main(){match Err(true){Err(true)=>{},_=>{}}}".to_owned(),
        };
        let checked =
            compile_scalar_with_profile("<fuzz>", &source, ReducedProfile::JNormal, false);
        let runnable =
            compile_scalar_with_profile("<fuzz>", &source, ReducedProfile::JNormal, true);
        match (checked, runnable) {
            (Ok(_), Ok(program)) => {
                let result = run_scalar(&program, MockEnvironment::default());
                assert!(
                    !matches!(result.outcome, Outcome::InternalError(_)),
                    "case {case}: {source}"
                );
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
