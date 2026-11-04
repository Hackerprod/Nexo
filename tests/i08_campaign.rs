//! Deterministic typed grammar campaign for the I08 Result boundary.
use nexo::{
    interp::{self, Outcome},
    ir::{self, CompileError},
};

const FILE: &str = "i08_generated.n";
const SEED: u64 = 0x8127_9c4a_3d5e_61b2;

fn next(state: &mut u64) -> u64 {
    *state ^= *state << 13;
    *state ^= *state >> 7;
    *state ^= *state << 17;
    *state
}

#[test]
#[ignore = "campaña tipada I08; ejecutar explícitamente en debug y release"]
fn generated_result_programs_preserve_check_run_and_cleanup() {
    let count = std::env::var("NEXO_I08_CAMPAIGN_COUNT")
        .ok()
        .and_then(|v| v.parse::<usize>().ok())
        .unwrap_or(if cfg!(debug_assertions) {
            100_000
        } else {
            300_000
        });
    let mut state = SEED;
    let mut negative = 0usize;
    for index in 0..count {
        let draw = next(&mut state);
        let a = (draw % 10_000) as i64;
        let b = (next(&mut state) % 10_000) as i64;
        let label = (next(&mut state) % 10_000) as i64;
        // The error injection rate cycles through 3–15% across the grammar.
        let rate = 3 + (index % 13) as u64;
        let inject = next(&mut state) % 100 < rate;
        let (source, expected, code) = if inject {
            negative += 1;
            match next(&mut state) % 6 {
                0 => (
                    format!("fn Main()->Result<(),I64>{{let x={a}?;return Ok(());}}"),
                    None,
                    Some("NX-E0102"),
                ),
                1 => (
                    format!("fn Main(){{let x=TryAdd({a},{b})?;}}"),
                    None,
                    Some("NX-E0102"),
                ),
                2 => (
                    format!("fn Main()->Result<(),Bool>{{let x=TryAdd({a},{b})?;return Ok(());}}"),
                    None,
                    Some("NX-E0102"),
                ),
                3 => (
                    format!(
                        "fn Main(){{let r:Result<MockResource,I64>=Ok(MockOpen({label}));let s=r;Drop(s);}}"
                    ),
                    None,
                    Some("NX-E0201"),
                ),
                4 => (
                    format!("fn Main(){{let a:Result<I64,I64>=Ok({a});let b=a<a;}}"),
                    None,
                    Some("NX-E0102"),
                ),
                _ => (
                    format!("fn Main(){{let x=TryAdd(true,{b});}}"),
                    None,
                    Some("NX-E0102"),
                ),
            }
        } else {
            match next(&mut state) % 7 {
                0 => (
                    format!(
                        "fn Main()->Result<(),I64>{{let x=TryAdd({a},{b})?;AssertEq(x,{});return Ok(());}}",
                        a + b
                    ),
                    Some(Outcome::Ok),
                    None,
                ),
                1 => (
                    format!(
                        "fn Main()->Result<(),I64>{{let x=TryAdd(9223372036854775807,{})?;return Ok(());}}",
                        b + 1
                    ),
                    Some(Outcome::Err(30)),
                    None,
                ),
                2 => (
                    format!(
                        "fn F()->Result<I64,I64>{{return Err({a});}}fn Main()->Result<(),I64>{{let x=F()?;return Ok(());}}"
                    ),
                    Some(Outcome::Err(a)),
                    None,
                ),
                3 => (
                    format!(
                        "fn Main(){{let a:Result<I64,I64>=Ok({a});let b:Result<I64,I64>=Ok({a});AssertEq(a,b);}}"
                    ),
                    Some(Outcome::Unit),
                    None,
                ),
                4 => (
                    format!(
                        "fn Make()->Result<MockResource,I64>{{return Ok(MockOpen({label}));}}fn Main()->Result<(),I64>{{let r=Make()?;Inspect(r);return Ok(());}}"
                    ),
                    Some(Outcome::Ok),
                    None,
                ),
                5 => (
                    format!(
                        "fn Main()->Result<(),I64>{{let x=TryAdd(TryAdd({a},{b})?,1)?;AssertEq(x,{});return Ok(());}}",
                        a + b + 1
                    ),
                    Some(Outcome::Ok),
                    None,
                ),
                _ => (
                    format!(
                        "fn Main()->Result<(),I64>{{let x=if true then TryAdd({a},{b})? else TryAdd(0,1)?;AssertEq(x,{});return Ok(());}}",
                        a + b
                    ),
                    Some(Outcome::Ok),
                    None,
                ),
            }
        };
        let checked = ir::check_scalar(FILE, &source);
        let compiled = ir::compile_scalar(FILE, &source);
        match (checked, compiled, code) {
            (Ok(_), Ok(program), None) => {
                let execution = interp::run_scalar(&program);
                assert_eq!(
                    Some(execution.outcome.clone()),
                    expected,
                    "seed={SEED} index={index} {source}"
                );
                assert!(
                    execution.diagnostics.is_empty(),
                    "seed={SEED} index={index} {source}: {execution:?}"
                );
            }
            (Err(CompileError::Rejected(a)), Err(CompileError::Rejected(b)), Some(code)) => {
                assert_eq!(a[0].code, code, "seed={SEED} index={index} {source}");
                assert_eq!(a[0].code, b[0].code, "seed={SEED} index={index} {source}");
                assert_eq!(
                    a[0].primary_span, b[0].primary_span,
                    "seed={SEED} index={index} {source}"
                );
            }
            (check, run, _) => {
                panic!("seed={SEED} index={index} {source}: check={check:?} run={run:?}")
            }
        }
    }
    eprintln!("i08 campaign seed={SEED} count={count} negative={negative}");
}
