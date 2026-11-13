//! I04a scalar interpreter over verified IR.
mod machine;
mod mock_env;
pub mod observation;
pub use machine::{
    Execution, ExecutionLimits, Outcome, ReducedExecutionProfile, run_scalar, run_with_limits,
};
pub use mock_env::{MockEnvironment, MockEnvironmentError};
