//! Isolated F0-0002 IR and scalar lowering.
mod effects;
mod lower;
pub mod model;
mod ownership_flow;
pub(crate) mod verify;

pub use lower::{CompileError, check_scalar, compile_scalar};
pub use model::Program;

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
mod tests;
