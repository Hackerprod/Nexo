mod match_coverage;
pub mod names;
#[cfg(test)]
mod tests;
pub mod types;

/// Shared semantic checker for the official B frontend.
pub fn check_proposed(file: &str, source: &str, run_entry: bool) -> Result<(), CheckProposedError> {
    check_proposed_with_worker_failure(file, source, run_entry, false)
}

#[derive(Debug)]
pub enum CheckProposedError {
    Diagnostics(Vec<Box<crate::diagnostic::Diagnostic>>),
    Internal(&'static str),
}

impl Default for CheckProposedError {
    fn default() -> Self {
        Self::Diagnostics(Vec::new())
    }
}

impl CheckProposedError {
    pub fn remove(&mut self, index: usize) -> Box<crate::diagnostic::Diagnostic> {
        match self {
            Self::Diagnostics(errors) => errors.remove(index),
            Self::Internal(_) => panic!("internal checker failure has no user diagnostics"),
        }
    }
}

impl std::ops::Deref for CheckProposedError {
    type Target = [Box<crate::diagnostic::Diagnostic>];

    fn deref(&self) -> &Self::Target {
        match self {
            Self::Diagnostics(errors) => errors,
            Self::Internal(_) => &[],
        }
    }
}

impl IntoIterator for CheckProposedError {
    type Item = Box<crate::diagnostic::Diagnostic>;
    type IntoIter = std::vec::IntoIter<Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        match self {
            Self::Diagnostics(errors) => errors,
            Self::Internal(_) => Vec::new(),
        }
        .into_iter()
    }
}

impl<'a> IntoIterator for &'a CheckProposedError {
    type Item = &'a Box<crate::diagnostic::Diagnostic>;
    type IntoIter = std::slice::Iter<'a, Box<crate::diagnostic::Diagnostic>>;

    fn into_iter(self) -> Self::IntoIter {
        (**self).iter()
    }
}

fn check_proposed_with_worker_failure(
    file: &str,
    source: &str,
    run_entry: bool,
    force_unavailable: bool,
) -> Result<(), CheckProposedError> {
    let ast = crate::frontend::parse::parse(file, source)
        .map_err(|d| CheckProposedError::Diagnostics(vec![d]))?;
    let report = check_with_entry(
        file,
        source,
        &ast,
        run_entry,
        check_ast_with_worker_failure(file, source, &ast, force_unavailable, 1_000_000, false),
    );
    if let Some(reason) = report.internal_error {
        return Err(CheckProposedError::Internal(reason));
    }
    if report.errors.is_empty() {
        Ok(())
    } else {
        Err(CheckProposedError::Diagnostics(report.errors))
    }
}

#[allow(clippy::vec_box)] // Diagnostics keep the shared boxed representation.
pub(crate) struct CompilationCheck {
    pub inferred: types::InferredTypes,
    pub resolved: Option<names::Resolved>,
    pub errors: Vec<Box<crate::diagnostic::Diagnostic>>,
    pub internal_error: Option<&'static str>,
}

#[allow(clippy::vec_box)] // Diagnostics share the boxed representation across stages.
#[cfg(test)]
pub(crate) fn check_for_compilation(
    file: &str,
    source: &str,
    ast: &crate::frontend::parse::Ast,
    run_entry: bool,
) -> CompilationCheck {
    check_for_compilation_with_type_limit(
        file,
        source,
        ast,
        run_entry,
        1_000_000,
        crate::diagnostic::PROFILE_ID,
    )
}

pub(crate) fn check_for_compilation_with_type_limit(
    file: &str,
    source: &str,
    ast: &crate::frontend::parse::Ast,
    run_entry: bool,
    type_steps_limit: u64,
    profile_id: &str,
) -> CompilationCheck {
    let mut report = check_with_entry(
        file,
        source,
        ast,
        run_entry,
        check_ast(
            file,
            source,
            ast,
            type_steps_limit,
            profile_id.starts_with("f0-0003-"),
        ),
    );
    for error in &mut report.errors {
        if error.code == "NX-E0003"
            && matches!(error.data.get("budget_kind"), Some(crate::diagnostic::DataValue::String(kind)) if kind == "type_solver_steps")
        {
            error.profile_id = profile_id.to_owned();
        }
    }
    report
}

#[cfg(test)]
pub(crate) fn check_for_compilation_with_worker_failure(
    file: &str,
    source: &str,
    ast: &crate::frontend::parse::Ast,
    run_entry: bool,
    force_unavailable: bool,
) -> CompilationCheck {
    check_with_entry(
        file,
        source,
        ast,
        run_entry,
        check_ast_with_worker_failure(file, source, ast, force_unavailable, 1_000_000, false),
    )
}

fn check_with_entry(
    file: &str,
    source: &str,
    ast: &crate::frontend::parse::Ast,
    run_entry: bool,
    mut report: CompilationCheck,
) -> CompilationCheck {
    if report.errors.is_empty()
        && run_entry
        && let Some(resolved) = report.resolved.as_ref()
        && let Err(error) = names::entry(file, source, resolved, ast)
    {
        report.errors.push(error);
    }
    report
}

#[allow(clippy::vec_box)] // Diagnostics share the boxed representation across stages.
fn check_ast(
    file: &str,
    source: &str,
    ast: &crate::frontend::parse::Ast,
    type_steps_limit: u64,
    allow_option: bool,
) -> CompilationCheck {
    check_ast_with_worker_failure(file, source, ast, false, type_steps_limit, allow_option)
}

fn check_ast_with_worker_failure(
    file: &str,
    source: &str,
    ast: &crate::frontend::parse::Ast,
    force_unavailable: bool,
    type_steps_limit: u64,
    allow_option: bool,
) -> CompilationCheck {
    std::thread::scope(|scope| {
        let worker = if force_unavailable {
            Err(std::io::Error::other("injected checker worker failure"))
        } else {
            std::thread::Builder::new()
                .stack_size(16 * 1024 * 1024)
                .spawn_scoped(scope, move || {
                    #[cfg(test)]
                    let _allocation_capture = crate::test_alloc::capture_for(file);
                    let resolved = match names::resolve_with_option(file, source, ast, allow_option)
                    {
                        Ok(resolved) => resolved,
                        Err(errors) => {
                            return CompilationCheck {
                                inferred: types::InferredTypes::default(),
                                resolved: None,
                                errors,
                                internal_error: None,
                            };
                        }
                    };
                    let types::TypeCheckReport { inferred, errors } =
                        types::check_with_partial_limit(
                            file,
                            source,
                            ast,
                            &resolved,
                            type_steps_limit,
                        );
                    CompilationCheck {
                        inferred,
                        resolved: Some(resolved),
                        errors,
                        internal_error: None,
                    }
                })
        };
        match worker {
            Ok(worker) => match worker.join() {
                Ok(result) => result,
                Err(payload) => std::panic::resume_unwind(payload),
            },
            Err(_) => CompilationCheck {
                inferred: types::InferredTypes::default(),
                resolved: None,
                errors: Vec::new(),
                internal_error: Some("type checker worker unavailable"),
            },
        }
    })
}
