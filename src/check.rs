mod limits;
pub mod names;
#[cfg(test)]
mod tests;
pub mod types;

/// Shared semantic checker for the official B frontend.
pub fn check_proposed(
    file: &str,
    source: &str,
    run_entry: bool,
) -> Result<(), Vec<Box<crate::diagnostic::Diagnostic>>> {
    let ast = crate::frontend::parse::parse(file, source).map_err(|d| vec![d])?;
    let report = check_ast(file, source, &ast, run_entry, false);
    if report.errors.is_empty() {
        Ok(())
    } else {
        Err(report.errors)
    }
}

#[allow(clippy::vec_box)] // Diagnostics keep the shared boxed representation.
pub(crate) struct CompilationCheck {
    pub inferred: types::InferredTypes,
    pub errors: Vec<Box<crate::diagnostic::Diagnostic>>,
}

#[allow(clippy::vec_box)] // Diagnostics share the boxed representation across stages.
pub(crate) fn check_for_compilation(
    file: &str,
    source: &str,
    ast: &crate::frontend::parse::Ast,
    run_entry: bool,
) -> CompilationCheck {
    check_ast(file, source, ast, run_entry, true)
}

#[allow(clippy::vec_box)] // Diagnostics share the boxed representation across stages.
fn check_ast(
    file: &str,
    source: &str,
    ast: &crate::frontend::parse::Ast,
    run_entry: bool,
    enforce_capabilities: bool,
) -> CompilationCheck {
    std::thread::scope(|scope| {
        let worker = std::thread::Builder::new()
            .stack_size(16 * 1024 * 1024)
            .spawn_scoped(scope, move || {
                #[cfg(test)]
                let _allocation_capture = crate::test_alloc::capture_for(file);
                let resolved = match names::resolve(file, source, ast) {
                    Ok(resolved) => resolved,
                    Err(errors) => {
                        return CompilationCheck {
                            inferred: types::InferredTypes::default(),
                            errors,
                        };
                    }
                };
                if enforce_capabilities
                    && let Some(limit) = limits::first_pending(file, source, ast, &resolved)
                {
                    return CompilationCheck {
                        inferred: types::InferredTypes::default(),
                        errors: vec![limit],
                    };
                }
                let types::TypeCheckReport {
                    inferred,
                    mut errors,
                } = types::check_with_partial(file, source, ast, &resolved);
                if errors.is_empty()
                    && run_entry
                    && let Err(error) = names::entry(file, source, &resolved, ast)
                {
                    errors.push(error);
                }
                CompilationCheck { inferred, errors }
            });
        match worker {
            Ok(worker) => match worker.join() {
                Ok(result) => result,
                Err(payload) => std::panic::resume_unwind(payload),
            },
            Err(_) => {
                use crate::diagnostic::{DataValue, Operation, Span, Stage};
                let mut d = names::diagnostic(
                    "NX-E0003",
                    None,
                    Stage::Types,
                    Operation::AnalysisLimit,
                    Span {
                        file: file.into(),
                        start: 0,
                        end: 1,
                    },
                );
                d.data.insert(
                    "budget_kind".into(),
                    DataValue::String("type_solver_steps".into()),
                );
                d.data.insert("limit".into(), DataValue::UInt(1_000_000));
                d.data.insert("observed".into(), DataValue::UInt(1_000_001));
                CompilationCheck {
                    inferred: types::InferredTypes::default(),
                    errors: vec![d],
                }
            }
        }
    })
}
