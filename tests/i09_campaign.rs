//! Deterministic typed I09 campaign spanning effect and ownership families.
use nexo::ir::{self, CompileError};

const FILE: &str = "i09_generated.n";
const SEED: u64 = 0x91a7_0e5c_3312_b4d1;

fn next(state: &mut u64) -> u64 {
    *state ^= *state << 13;
    *state ^= *state >> 7;
    *state ^= *state << 17;
    *state
}

fn program(index: usize, value: i64, family: u64) -> (String, Vec<&'static str>) {
    let suffix = index % 97;
    match family {
        0 => (format!("fn Main(){{AssertEq({value},{value});}}"), vec![]),
        1 => (
            format!("fn Main() with no_io{{Print({value});}}"),
            vec!["NX-E0415"],
        ),
        2 => (
            format!("fn H{suffix}(){{Print({value});}} fn Main() with no_io{{H{suffix}();}}"),
            vec!["NX-E0415"],
        ),
        3 => (
            format!(
                "fn A{suffix}(){{B{suffix}();}} fn B{suffix}(){{A{suffix}();Print({value});}} fn Main() with no_io{{A{suffix}();}}"
            ),
            vec!["NX-E0415"],
        ),
        4 => (
            format!("fn Main() with no_io{{if true{{Print({value});}}}}"),
            vec!["NX-E0415"],
        ),
        5 => (
            format!("fn Main() with no_io{{while false{{Print({value});}}}}"),
            vec!["NX-E0415"],
        ),
        6 => (
            format!("fn Main() with no_block{{Drop(MockBlockingOpen({value}));}}"),
            vec!["NX-E0413"],
        ),
        7 => (
            format!("fn Main() with no_block{{Inspect(MockBlockingOpen({value}));}}"),
            vec!["NX-E0413"],
        ),
        8 => (
            "fn Main() with no_io,no_alloc{MockUnknown();}".to_owned(),
            vec!["NX-E0412", "NX-E0415"],
        ),
        9 => (
            format!("fn Main() with no_alloc{{let a=[{value}];}}"),
            vec!["NX-E0412"],
        ),
        10 => (
            format!(
                "fn K{suffix}(a:view Array<Array<I64>>)->Array<I64> with no_alloc{{return a[{value}];}} fn Main(){{}}"
            ),
            vec!["NX-E0412"],
        ),
        11 => (
            format!(
                "fn K{suffix}(r:view Result<Array<I64>,I64>) with no_alloc{{let b=r;}} fn Main(){{}}"
            ),
            vec!["NX-E0412"],
        ),
        12 => (
            format!(
                "fn F{suffix}()->Result<I64,I64>{{return Err({value});}} fn Main()->Result<I64,I64> with no_block{{let r=MockBlockingOpen({value});let n=F{suffix}()?;return Ok(n);}}"
            ),
            vec!["NX-E0413"],
        ),
        13 => (
            format!("fn Print(x:view I64){{}} fn Main() with no_io{{let n={value};Print(n);}}"),
            vec![],
        ),
        _ => (
            format!("fn Main() with no_io{{let r=MockOpen({value});Inspect(r);}}"),
            vec!["NX-E0415"],
        ),
    }
}

#[test]
#[ignore = "typed I09 campaign; run explicitly in debug and release"]
fn typed_effect_programs_keep_contracts_and_transitive_summaries() {
    let count = std::env::var("NEXO_I09_CAMPAIGN_COUNT")
        .ok()
        .and_then(|value| value.parse::<usize>().ok())
        .unwrap_or(if cfg!(debug_assertions) {
            100_000
        } else {
            300_000
        });
    let mut state = SEED;
    let mut negative = 0usize;
    let mut families = [0usize; 15];
    for index in 0..count {
        let value = (next(&mut state) % 100_000) as i64;
        let family = next(&mut state) % families.len() as u64;
        families[family as usize] += 1;
        let (source, expected) = program(index, value, family);
        if !expected.is_empty() {
            negative += 1;
        }
        match (expected.as_slice(), ir::check_scalar(FILE, &source)) {
            ([], Ok(_)) => {}
            (expected, Err(CompileError::Rejected(errors))) => {
                let actual: Vec<_> = errors
                    .iter()
                    .map(|diagnostic| diagnostic.code.as_str())
                    .collect();
                assert_eq!(
                    actual, expected,
                    "seed={SEED} index={index} family={family} source={source}"
                );
                for diagnostic in errors {
                    diagnostic.validate(FILE, &source).unwrap();
                }
            }
            (expected, actual) => panic!(
                "seed={SEED} index={index} family={family} expected={expected:?} actual={actual:?} source={source}"
            ),
        }
    }
    println!("seed={SEED} count={count} negative={negative} families={families:?}");
}
