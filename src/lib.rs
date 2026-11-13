//! Nexo F0: official B frontend, semantic checker, verified IR and interpreter.
pub mod check;
pub mod diagnostic;
pub mod frontend;
pub mod interp;
pub mod ir;
pub mod prelude;
pub mod union;

#[cfg(test)]
pub(crate) mod test_alloc;
