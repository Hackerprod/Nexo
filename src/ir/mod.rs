//! Isolated F0-0002 IR and scalar lowering.
mod effects;
mod lower;
pub mod model;
mod ownership_flow;
pub(crate) mod verify;
pub(crate) mod work;

#[cfg(test)]
pub(crate) use lower::compile_scalar_with_flow_limit;
#[cfg(test)]
pub(crate) use lower::compile_scalar_with_sparse_ownership;
pub use lower::{CompileError, check_scalar, compile_scalar};
pub use lower::{ReducedProfile, compile_scalar_with_profile};
pub use model::Program;

#[cfg(test)]
#[test]
fn verifier_and_program_drop_fit_256_kib_for_scale_families() {
    let block = format!(
        "fn Main(){{{}let x=1;{}}}",
        "{".repeat(126),
        "}".repeat(126)
    );
    let nested_if = format!(
        "fn Main(){{let x={}1{};}}",
        "if true then (".repeat(126),
        ") else 0".repeat(126)
    );
    let chain = format!("fn Main(){{var x=0;{}}}", "x=x+1;".repeat(2000));
    let states = format!(
        "gen fn G()->I64{{}}fn Main(){{{}{}}}",
        (0..200)
            .map(|i| format!("let g{i}=G();"))
            .collect::<String>(),
        (0..200).map(|i| format!("Drop(g{i});")).collect::<String>()
    );
    let captures = format!(
        "fn Main(){{{}let f=fn [{}](){{{}}};Drop(f);}}",
        (0..500)
            .map(|i| format!("let a{i}={i};"))
            .collect::<String>(),
        (0..500)
            .map(|i| format!("view a{i}"))
            .collect::<Vec<_>>()
            .join(","),
        (0..500)
            .map(|i| format!("let b{i}=a{i};"))
            .collect::<String>()
    );
    for source in [block, nested_if, chain, states, captures] {
        let program = compile_scalar_with_profile(
            "verify-stack.n",
            &source,
            ReducedProfile::from_id("f0-0003-b-v1").unwrap(),
            true,
        )
        .unwrap();
        std::thread::Builder::new()
            .stack_size(256 * 1024)
            .spawn(move || {
                verify::verify_program(&program).unwrap();
                drop(program);
            })
            .unwrap()
            .join()
            .unwrap();
    }
    let rejected = format!(
        "fn Main(){{{}let x=missing;{}}}",
        "{".repeat(126),
        "}".repeat(126)
    );
    std::thread::Builder::new()
        .stack_size(256 * 1024)
        .spawn(move || {
            let result = compile_scalar_with_profile(
                "reject-stack.n",
                &rejected,
                ReducedProfile::from_id("f0-0003-b-v1").unwrap(),
                true,
            );
            assert!(matches!(result, Err(CompileError::Rejected(_))));
        })
        .unwrap()
        .join()
        .unwrap();
}

#[cfg(test)]
thread_local! {
    static CFG_THREAD_WORK: std::cell::Cell<Option<u64>> = const { std::cell::Cell::new(None) };
}
#[cfg(test)]
pub(crate) fn count_cfg_work(n: u64) {
    CFG_THREAD_WORK.with(|cell| {
        if let Some(current) = cell.get() {
            cell.set(Some(current.saturating_add(n)));
        }
    });
}
#[cfg(test)]
pub(crate) fn measure_cfg_thread_work<T>(work: impl FnOnce() -> T) -> (T, u64) {
    CFG_THREAD_WORK.with(|cell| cell.set(Some(0)));
    let result = work();
    let steps = CFG_THREAD_WORK.with(|cell| cell.replace(None).unwrap());
    (result, steps)
}
#[cfg(test)]
pub(crate) fn cfg_measure_active() -> bool {
    CFG_THREAD_WORK.with(|cell| cell.get().is_some())
}
#[cfg(test)]
pub(crate) fn cfg_forward_work(n: u64) {
    CFG_THREAD_WORK.with(|cell| {
        if let Some(current) = cell.get() {
            cell.set(Some(current.saturating_add(n)));
        }
    });
}
#[cfg(not(test))]
pub(crate) fn count_cfg_work(_: u64) {}

#[cfg(test)]
mod i05_tests;
#[cfg(test)]
mod i06_tests;
#[cfg(test)]
mod i07a_tests;
#[cfg(test)]
mod i07b_tests;
#[cfg(test)]
mod i07c_tests;
#[cfg(test)]
mod i10a_tests;
#[cfg(test)]
mod i10b_tests;
#[cfg(test)]
mod tests;
#[cfg(test)]
mod work_oracle_tests;
