//! Isolated F0-0002 IR and scalar lowering.
mod lower;
pub mod model;
pub(crate) mod verify;

pub use lower::{CompileError, compile_scalar};
pub use model::Program;

#[cfg(test)]
mod i05_tests;
#[cfg(test)]
mod tests;
