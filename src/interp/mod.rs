//! I04a scalar interpreter over verified IR.
mod machine;
pub mod observation;
pub use machine::{Execution, Outcome, run_scalar};
