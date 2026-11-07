//! Deterministic I10b runtime work and timing campaigns.
use nexo::{interp, ir};

const FILE: &str = "i10b_work_generated.n";
const DEFAULT_SEED: u64 = 0x0081_1106_1106_c0de;

fn next(state: &mut u64) -> u64 {
    *state ^= *state << 13;
    *state ^= *state >> 7;
    *state ^= *state << 17;
    *state
}

fn loop_program(iterations: u64) -> String {
    format!("fn Main(){{var i=0;while i<{iterations}{{i=i+1;}}}}")
}

#[test]
#[ignore = "I10b deterministic runtime work campaign; run explicitly in debug and release"]
fn generated_work_counters_match_the_independent_loop_ledger() {
    let count = std::env::var("NEXO_I10B_CAMPAIGN_COUNT")
        .ok()
        .and_then(|value| value.parse::<usize>().ok())
        .unwrap_or(if cfg!(debug_assertions) {
            100_000
        } else {
            300_000
        });
    let seed = std::env::var("NEXO_I10B_CAMPAIGN_SEED")
        .ok()
        .and_then(|value| u64::from_str_radix(value.trim_start_matches("0x"), 16).ok())
        .unwrap_or(DEFAULT_SEED);
    let mut state = seed;
    let mut families = [0usize; 4];
    for index in 0..count {
        let iterations = next(&mut state) % families.len() as u64;
        families[iterations as usize] += 1;
        let source = loop_program(iterations);
        let program = ir::compile_scalar(FILE, &source)
            .unwrap_or_else(|error| panic!("seed={seed} index={index} source={source}: {error:?}"));
        let execution = interp::run_scalar(&program);
        assert_eq!(execution.outcome, interp::Outcome::Unit, "index={index}");
        assert!(
            execution.diagnostics.is_empty(),
            "index={index}: {execution:?}"
        );
        assert_eq!(
            execution.work_consumed,
            14 * iterations + 14,
            "seed={seed} index={index} iterations={iterations} source={source}"
        );
    }
    println!("generator=i10b-source-ledger-v2 seed={seed} count={count} loop_counts={families:?}");
}

#[test]
#[ignore = "release timing campaign; calibrates runtime work ratio against a linear control"]
fn runtime_work_time_ratio_uses_a_calibrated_threshold() {
    if cfg!(debug_assertions) {
        return;
    }

    fn median_runtime(iterations: u64) -> (f64, u64) {
        let source = loop_program(iterations);
        let program = ir::compile_scalar(FILE, &source).unwrap();
        let warm = interp::run_scalar(&program);
        assert_eq!(warm.work_consumed, 14 * iterations + 14);
        let mut samples = Vec::with_capacity(7);
        for _ in 0..7 {
            let began = std::time::Instant::now();
            let execution = interp::run_scalar(&program);
            samples.push(began.elapsed().as_secs_f64());
            assert_eq!(execution.work_consumed, 14 * iterations + 14);
        }
        samples.sort_by(f64::total_cmp);
        (samples[3], 14 * iterations + 14)
    }

    let (base_small, base_work_small) = median_runtime(2_000);
    let (base_large, base_work_large) = median_runtime(4_000);
    let calibrated = base_large / base_small;
    let threshold = 3.0_f64.min(2.25_f64.max(calibrated + 0.25));
    let (small, work_small) = median_runtime(4_000);
    let (large, work_large) = median_runtime(8_000);
    let ratio = large / small;
    assert!(
        ratio <= threshold,
        "runtime linear work ratio {ratio:.3} exceeds calibrated threshold {threshold:.3}; base={calibrated:.3}, work={work_small}/{work_large}, base_work={base_work_small}/{base_work_large}"
    );
    println!(
        "seed={DEFAULT_SEED} base_ratio={calibrated:.3} threshold={threshold:.3} measured_ratio={ratio:.3} work={work_small}/{work_large}"
    );
}
