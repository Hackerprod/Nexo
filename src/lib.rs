//! Nexo F0: official B frontend, semantic checker, verified IR and interpreter.
pub mod check;
pub mod diagnostic;
pub mod frontend;
pub mod interp;
pub mod ir;

#[cfg(test)]
pub(crate) mod test_alloc;
