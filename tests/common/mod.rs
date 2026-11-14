#![allow(dead_code)]

pub fn compile_j(
    file: &str,
    source: &str,
    run_entry: bool,
) -> Result<nexo::ir::Program, nexo::ir::CompileError> {
    nexo::ir::compile_scalar_with_profile(
        file,
        source,
        nexo::ir::ReducedProfile::JNormal,
        run_entry,
    )
}

pub fn run_j(
    file: &str,
    source: &str,
    environment: nexo::interp::MockEnvironment,
) -> Result<nexo::interp::Execution, nexo::ir::CompileError> {
    let program = compile_j(file, source, true)?;
    Ok(nexo::interp::run_with_limits(
        &program,
        environment,
        nexo::interp::ReducedExecutionProfile::JNormal,
    ))
}
