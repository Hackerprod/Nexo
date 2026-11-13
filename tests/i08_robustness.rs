//! Bounded Result and propagation robustness probes.
use nexo::{
    interp::{self, Outcome},
    ir::{self, CompileError},
};

const FILE: &str = "i08_robustness.n";

#[test]
fn result_entry_and_propagation_on_small_stacks() {
    for kib in [128, 256, 512] {
        let worker = std::thread::Builder::new()
            .stack_size(kib * 1024)
            .spawn(move || {
                let good = "fn F()->Result<I64,I64>{return Err(9);}fn Main()->Result<(),I64>{let x=F()?;return Ok(());}";
                let program = ir::compile_scalar(FILE, good).unwrap();
                assert_eq!(interp::run_scalar(&program, Default::default()).outcome, Outcome::Err(9));
                let bad = "fn Main()->Result<(),Bool>{let x=TryAdd(1,2)?;return Ok(());}";
                assert!(matches!(ir::check_scalar(FILE, bad), Err(CompileError::Rejected(_))));
                assert!(matches!(ir::compile_scalar(FILE, bad), Err(CompileError::Rejected(_))));
            })
            .unwrap();
        worker.join().unwrap();
    }
}

#[test]
#[ignore = "sonda determinista de fuentes Result mutadas"]
fn mutated_result_sources_do_not_panic_or_reach_invalid_ir() {
    let mut state = 0x7e91_294b_56c3_d08au64;
    let seeds = [
        "fn Main()->Result<(),I64>{let x=TryAdd(1,2)?;return Ok(());}",
        "fn F()->Result<I64,I64>{return Err(7);}fn Main()->Result<(),I64>{let x=F()?;return Ok(());}",
        "fn Main(){let r:Result<MockResource,I64>=Ok(MockOpen(1));Drop(move r);}",
        "fn Main(){let a:Result<I64,I64>=Ok(1);AssertEq(a,a);}",
    ];
    let alphabet = b"?<>():,{}[]+-0123456789IRltuyOkeMna ;";
    for index in 0..20_000 {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        let mut bytes = seeds[(state as usize) % seeds.len()].as_bytes().to_vec();
        let mutations = 1 + (state as usize % 3);
        for _ in 0..mutations {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            let at = (state as usize) % bytes.len();
            bytes[at] = alphabet[((state >> 19) as usize) % alphabet.len()];
        }
        let source = String::from_utf8(bytes).unwrap();
        let result = std::panic::catch_unwind(|| {
            let _ = ir::check_scalar(FILE, &source);
            match ir::compile_scalar(FILE, &source) {
                Ok(program) => {
                    let execution = interp::run_scalar(&program, Default::default());
                    assert!(!matches!(execution.outcome, Outcome::InternalError(_)));
                }
                Err(CompileError::Rejected(_)) => {}
                Err(other) => panic!("unexpected compilation state: {other:?}"),
            }
        });
        assert!(
            result.is_ok(),
            "seed=0x7e91294b56c3d08a index={index} source={source}"
        );
    }
}
