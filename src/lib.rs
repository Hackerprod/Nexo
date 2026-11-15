//! Nexo F0: official B frontend, semantic checker, verified IR and interpreter.
pub mod check;
pub mod diagnostic;
pub mod frontend;
pub mod interp;
pub mod ir;
pub mod prelude;
pub mod union;

// Measured Windows minima for valid sources through syntax_nesting=128.
// Workers keep one power-of-two margin above the measured minimum.
pub(crate) fn frontend_stack_size(phase: &str) -> usize {
    let measured_kib = match phase {
        "PARSER" | "CHECKER" | "LOWERING" => {
            if cfg!(debug_assertions) {
                8192
            } else {
                1024
            }
        }
        "FORMATTER" => {
            if cfg!(debug_assertions) {
                1024
            } else {
                256
            }
        }
        "SOLVER_TEST" => {
            if cfg!(debug_assertions) {
                4096
            } else {
                512
            }
        }
        _ => unreachable!("unknown front-end phase"),
    };
    // The override is confined to isolated test subprocesses. Normal CLI
    // compilation always uses the measured size plus its one-step margin.
    let probe = (phase == "SOLVER_TEST" || std::env::var_os("NEXO_STACK_MINIMUM_CASE").is_some())
        .then(|| std::env::var(format!("NEXO_PROBE_{phase}_KIB")))
        .and_then(Result::ok)
        .and_then(|value| value.parse::<usize>().ok())
        .filter(|kib| *kib >= 256);
    probe.unwrap_or(measured_kib * 2) * 1024
}

#[cfg(test)]
pub(crate) mod test_alloc;
