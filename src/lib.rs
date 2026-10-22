//! F0 slice 1: deliberately bounded, sequential value/ownership experiment.
//! No native backend, CoW, reference counting, tasks or performance claims.
pub mod check;
pub mod diagnostic;
pub mod frontend;
pub mod interp;
pub mod ir;
pub mod runtime;
pub mod syntax;

#[cfg(test)]
pub(crate) mod test_alloc;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Diagnostic {
    pub code: &'static str,
    pub offset: usize,
    pub message: String,
}
impl Diagnostic {
    pub fn new(code: &'static str, offset: usize, message: impl Into<String>) -> Self {
        Self {
            code,
            offset,
            message: message.into(),
        }
    }
    pub fn render(&self, source: &str) -> String {
        let mut offset = self.offset.min(source.len());
        while !source.is_char_boundary(offset) {
            offset -= 1;
        }
        let prefix = &source[..offset];
        let line = prefix.bytes().filter(|b| *b == b'\n').count() + 1;
        let column = prefix.rsplit('\n').next().unwrap_or("").chars().count() + 1;
        format!("{}:{}:{}: {}", self.code, line, column, self.message)
    }
}
/// The runtime accepts only a checked artifact; its internals cannot be forged
/// by an external caller. Every CLI execution checks the whole input first.
pub struct CheckedProgram {
    pub(crate) body: Vec<syntax::Stmt>,
}
pub fn compile(source: &str) -> Result<CheckedProgram, Diagnostic> {
    let body = syntax::parse(source)?;
    check::check(&body)?;
    Ok(CheckedProgram { body })
}
