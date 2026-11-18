//! Parser for F0-D15 B, building the shared normalized AST.
use std::cell::RefCell;
use std::collections::{BTreeMap, HashSet};
use std::sync::mpsc::{self, Sender};
#[cfg(test)]
pub(crate) fn measure_parse_work(source: &str) -> (Result<Ast, Box<Diagnostic>>, u64) {
    let work = std::sync::Arc::new(std::sync::atomic::AtomicU64::new(0));
    let result =
        parse_with_split_tokens_counted("scale.n", source, Some(work.clone()), false, false)
            .map(|(ast, _)| ast);
    let units = work.load(std::sync::atomic::Ordering::Relaxed);
    (result, units)
}

#[cfg(test)]
mod j16_parser_stack_probe {
    use super::*;

    #[test]
    fn direct_204_kib_child() {
        let Ok(family) = std::env::var("J16_PARSER_DIRECT_204") else {
            return;
        };
        let source = match family.as_str() {
            "block" => format!(
                "fn Main(){{{}let x=1;{}}}",
                "{".repeat(126),
                "}".repeat(126)
            ),
            "closure" => format!(
                "fn Main(){{let f={}1{};}}",
                "fn(){let g=".repeat(63),
                ";}".repeat(63)
            ),
            "match_expr" => {
                let mut expr = String::from("1");
                for _ in 0..63 {
                    expr = format!("match 1{{_=>{{value:{expr};}}}}");
                }
                format!("fn Main(){{let x={expr};}}")
            }
            _ => panic!("unknown family: {family}"),
        };
        std::thread::Builder::new()
            .stack_size(204 * 1024)
            .spawn(move || parse_inner("probe.n", &source, None, true, false).unwrap())
            .unwrap()
            .join()
            .unwrap();
    }

    #[test]
    fn direct_204_kib_file_child() {
        let Ok(path) = std::env::var("J16_PARSER_CASE_FILE") else {
            return;
        };
        let source = std::fs::read_to_string(path).unwrap();
        let kib = std::env::var("J16_PARSER_STACK_KIB")
            .ok()
            .and_then(|value| value.parse::<usize>().ok())
            .unwrap_or(204);
        let result = std::thread::Builder::new()
            .stack_size(kib * 1024)
            .spawn(move || parse_inner("probe.n", &source, None, true, false))
            .unwrap()
            .join()
            .unwrap();
        match std::env::var("J16_PARSER_EXPECT").as_deref() {
            Ok("ok") => {
                result.unwrap();
            }
            Ok(expected) => {
                let diagnostic = result.unwrap_err();
                let span = diagnostic.primary_span.as_ref().unwrap();
                let kind = match diagnostic.data.get("budget_kind") {
                    Some(DataValue::String(kind)) => kind.as_str(),
                    _ => "",
                };
                let actual = format!("{}:{}:{}:{}", diagnostic.code, kind, span.start, span.end);
                assert_eq!(actual, expected, "{diagnostic:?}");
                assert_eq!(diagnostic.subcause, None);
                assert_eq!(diagnostic.stage, Stage::Parse);
                assert_eq!(diagnostic.operation, Operation::AnalysisLimit);
                assert_eq!(diagnostic.data.get("limit"), Some(&DataValue::UInt(128)));
                assert_eq!(diagnostic.data.get("observed"), Some(&DataValue::UInt(129)));
            }
            Err(_) => panic!("J16_PARSER_EXPECT is required"),
        }
    }
}

use super::lex::{self, Kind, Token};
use crate::diagnostic::{
    self, DataValue, Diagnostic, ExpectedTokensKind, Namespace, Operation, RelatedRole,
    RelatedSpan, Repair, RepairStatus, Severity, Span, Stage, Token as DToken, TokenKind,
};

const EXPR_START: &[&str] = &[
    "(",
    "-",
    "IntLiteral",
    "Name",
    "[",
    "edit",
    "false",
    "if",
    "move",
    "true",
];
const PATTERN_FIRST: &[&str] = &[
    "(",
    "-",
    "IntLiteral",
    "Name",
    "_",
    "edit",
    "false",
    "move",
    "true",
];
const TRY_START: &[&str] = &[
    "(",
    "-",
    "IntLiteral",
    "Name",
    "[",
    "edit",
    "false",
    "move",
    "true",
];
const BLOCK_NEXT: &[&str] = &[
    "(",
    "-",
    "IntLiteral",
    "Name",
    "[",
    "edit",
    "false",
    "if",
    "let",
    "move",
    "return",
    "true",
    "var",
    "while",
    "{",
    "}",
];
const TYPE_START: &[&str] = &[
    "(",
    "Array",
    "Bool",
    "I64",
    "MockBlockingResource",
    "MockResource",
    "Result",
];
const POSTFIX_START: &[&str] = &["(", "-", "IntLiteral", "Name", "[", "false", "true"];
// FIRST sets transcribed from §3 and §13.14. Callers select a committed
// production state before looking at the erroneous token (including EOF).
const PARAMETER_LIST_FIRST: &[&str] = &[")", "Name"];
const PARAMETER_FIRST: &[&str] = &["Name"];
const PARAM_MODE_FIRST: &[&str] = &["edit", "move", "view"];
const OPERATION_FIRST: &[&str] = &["call", "cancel", "copy", "create", "drop", "resume"];
const OPERATION_LIST_FIRST: &[&str] = &["call", "cancel", "copy", "create", "drop", "resume", "}"];
const WITH_FIRST: &[&str] = &[
    "no_alloc",
    "no_block",
    "no_heap_alloc",
    "no_io",
    "no_suspend",
    "unrestricted",
    "{",
];
const RESTRICTION_FIRST: &[&str] = &[
    "no_alloc",
    "no_block",
    "no_heap_alloc",
    "no_io",
    "no_suspend",
    "unrestricted",
];
fn is_restriction(word: &str) -> bool {
    RESTRICTION_FIRST.contains(&word)
}
const GROUP_FIRST: &[&str] = &[
    "(",
    ")",
    "-",
    "IntLiteral",
    "Name",
    "[",
    "edit",
    "false",
    "if",
    "move",
    "true",
];
const ARRAY_FIRST: &[&str] = &[
    "(",
    "-",
    "IntLiteral",
    "Name",
    "[",
    "]",
    "edit",
    "false",
    "if",
    "move",
    "true",
];
const ELSE_FIRST: &[&str] = &["if", "{"];
const BRANCH_FIRST: &[&str] = &[
    "(",
    "-",
    "IntLiteral",
    "Name",
    "[",
    "edit",
    "false",
    "move",
    "true",
    "{",
];
const EXPR_START_NEW: &[&str] = &[
    "(",
    "-",
    "IntLiteral",
    "Name",
    "[",
    "edit",
    "false",
    "fn",
    "if",
    "match",
    "move",
    "true",
];
const TRY_START_NEW: &[&str] = &[
    "(",
    "-",
    "IntLiteral",
    "Name",
    "[",
    "edit",
    "false",
    "fn",
    "move",
    "true",
];
const BLOCK_NEXT_NEW: &[&str] = &[
    "(",
    "-",
    "IntLiteral",
    "Name",
    "[",
    "break",
    "continue",
    "edit",
    "false",
    "fn",
    "for",
    "if",
    "let",
    "match",
    "move",
    "return",
    "true",
    "var",
    "while",
    "yield",
    "{",
    "}",
];
const TYPE_START_NEW: &[&str] = &[
    "(",
    "Array",
    "Bool",
    "I64",
    "MockBlockingResource",
    "MockResource",
    "Option",
    "Result",
];
const POSTFIX_START_NEW: &[&str] = &["(", "-", "IntLiteral", "Name", "[", "false", "fn", "true"];
const GROUP_FIRST_NEW: &[&str] = &[
    "(",
    ")",
    "-",
    "IntLiteral",
    "Name",
    "[",
    "edit",
    "false",
    "fn",
    "if",
    "match",
    "move",
    "true",
];
const ARRAY_FIRST_NEW: &[&str] = &[
    "(",
    "-",
    "IntLiteral",
    "Name",
    "[",
    "]",
    "edit",
    "false",
    "fn",
    "if",
    "match",
    "move",
    "true",
];
const BRANCH_FIRST_NEW: &[&str] = &[
    "(",
    "-",
    "IntLiteral",
    "Name",
    "[",
    "edit",
    "false",
    "fn",
    "move",
    "true",
    "{",
];

#[derive(Clone, Copy)]
enum FirstSet {
    Expr,
    Try,
    Block,
    Type,
    Postfix,
    Group,
    Array,
    Branch,
    Module,
}

fn first(new_profile: bool, set: FirstSet) -> &'static [&'static str] {
    match (new_profile, set) {
        (true, FirstSet::Expr) => EXPR_START_NEW,
        (true, FirstSet::Try) => TRY_START_NEW,
        (true, FirstSet::Block) => BLOCK_NEXT_NEW,
        (true, FirstSet::Type) => TYPE_START_NEW,
        (true, FirstSet::Postfix) => POSTFIX_START_NEW,
        (true, FirstSet::Group) => GROUP_FIRST_NEW,
        (true, FirstSet::Array) => ARRAY_FIRST_NEW,
        (true, FirstSet::Branch) => BRANCH_FIRST_NEW,
        (true, FirstSet::Module) => &["EOF", "fn", "gen"],
        (false, FirstSet::Expr) => EXPR_START,
        (false, FirstSet::Try) => TRY_START,
        (false, FirstSet::Block) => BLOCK_NEXT,
        (false, FirstSet::Type) => TYPE_START,
        (false, FirstSet::Postfix) => POSTFIX_START,
        (false, FirstSet::Group) => GROUP_FIRST,
        (false, FirstSet::Array) => ARRAY_FIRST,
        (false, FirstSet::Branch) => BRANCH_FIRST,
        (false, FirstSet::Module) => &["EOF", "fn"],
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Ast {
    pub functions: Vec<Function>,
    pub span: Span,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Function {
    pub is_generator: bool,
    pub name: String,
    pub name_span: Span,
    pub params: Vec<Parameter>,
    pub result: Option<Type>,
    pub with_clause: Option<WithClause>,
    pub body: Block,
    pub span: Span,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Parameter {
    pub mode: Mode,
    pub name: String,
    pub ty: Type,
    pub span: Span,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ClosureParameter {
    pub mode: Mode,
    pub name: String,
    pub ty: Option<Type>,
    pub span: Span,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CaptureMode {
    View,
    Copy,
    Edit,
    Move,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Capture {
    pub mode: CaptureMode,
    pub name: String,
    pub span: Span,
    pub name_span: Span,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Closure {
    pub fn_span: Span,
    pub captures: Option<Vec<Capture>>,
    pub params: Vec<ClosureParameter>,
    pub result: Option<Type>,
    pub with_clause: Option<WithClause>,
    pub body: Block,
    pub span: Span,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    Read,
    Inout,
    Consume,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Type {
    I64,
    Bool,
    Unit,
    MockResource,
    MockBlockingResource,
    Array(Box<Type>),
    Result(Box<Type>, Box<Type>),
    Option(Box<Type>),
    /// Concrete owned state returned by a generator declaration.
    Generator {
        site: u64,
        item: Box<Type>,
    },
    /// Anonymous nominal identity of a checked closure literal. Source type
    /// grammar never constructs this variant directly.
    Closure {
        site: u64,
        copyable: bool,
        owns_resource: bool,
    },
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum WithClause {
    Simple(Vec<Restriction>),
    Operations(Vec<OperationLimit>),
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Restriction {
    pub name: String,
    pub span: Span,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OperationLimit {
    pub name: String,
    pub restrictions: Vec<Restriction>,
    pub span: Span,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Block {
    pub statements: Vec<Statement>,
    pub span: Span,
    pub close_span: Span,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Match {
    pub scrutinee: Box<Expr>,
    pub mode: Option<Mode>,
    pub mode_span: Option<Span>,
    pub arms: Vec<Arm>,
    pub span: Span,
    pub keyword_span: Span,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Arm {
    pub pattern: Pattern,
    pub guard: Option<Expr>,
    pub guard_span: Option<Span>,
    pub body: Block,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Pattern {
    Wildcard(Span),
    Integer(String, Span),
    Bool(bool, Span),
    Name {
        name: String,
        mode: Option<Mode>,
        span: Span,
    },
    Constructor {
        name: String,
        args: Vec<Pattern>,
        span: Span,
    },
}

impl Pattern {
    pub fn span(&self) -> &Span {
        match self {
            Self::Wildcard(span)
            | Self::Integer(_, span)
            | Self::Bool(_, span)
            | Self::Name { span, .. }
            | Self::Constructor { span, .. } => span,
        }
    }
}

fn collect_pattern_bindings(pattern: &Pattern, bindings: &mut HashSet<String>) {
    let mut pending = vec![pattern];
    while let Some(pattern) = pending.pop() {
        match pattern {
            Pattern::Name { name, .. } => {
                bindings.insert(name.clone());
            }
            Pattern::Constructor { args, .. } => {
                pending.extend(args.iter().rev());
            }
            _ => {}
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Statement {
    Let {
        mutable: bool,
        name: String,
        annotation: Option<Type>,
        value: Expr,
        span: Span,
    },
    Assign {
        place: Expr,
        value: Expr,
        span: Span,
    },
    Return {
        value: Option<Expr>,
        span: Span,
    },
    GuardControl {
        token: String,
        value: Option<Expr>,
        guarded: bool,
        token_span: Span,
        span: Span,
    },
    Value {
        value: Option<Expr>,
        marker: Option<Span>,
        span: Span,
    },
    If {
        condition: Expr,
        then_block: Block,
        otherwise: Option<Box<Statement>>,
        span: Span,
    },
    While {
        condition: Expr,
        body: Block,
        span: Span,
    },
    For {
        name: String,
        name_span: Span,
        iterable: Expr,
        body: Block,
        span: Span,
    },
    Match(Match),
    Block(Block),
    Expr {
        value: Expr,
        span: Span,
    },
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Expr {
    Closure(Box<Closure>),
    Name(String, Span),
    Integer(String, Span),
    Bool(bool, Span),
    Unit(Span),
    Array(Vec<Expr>, Span),
    Group(Box<Expr>, Span),
    Unary {
        mode: Mode,
        value: Box<Expr>,
        span: Span,
    },
    Binary {
        op: String,
        left: Box<Expr>,
        right: Box<Expr>,
        span: Span,
    },
    Call {
        target: Box<Expr>,
        args: Vec<Argument>,
        span: Span,
    },
    Index {
        target: Box<Expr>,
        index: Box<Expr>,
        selector: Span,
        span: Span,
    },
    If {
        condition: Box<Expr>,
        then_block: Block,
        otherwise: Option<Box<Else>>,
        span: Span,
    },
    Match(Box<Match>),
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Else {
    Block(Block),
    If(Box<Expr>),
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Argument {
    pub label: Option<String>,
    pub value: Expr,
    pub span: Span,
}
impl Expr {
    pub fn span(&self) -> &Span {
        match self {
            Self::Closure(closure) => &closure.span,
            Self::Name(_, s)
            | Self::Integer(_, s)
            | Self::Bool(_, s)
            | Self::Unit(s)
            | Self::Array(_, s)
            | Self::Group(_, s)
            | Self::Unary { span: s, .. }
            | Self::Binary { span: s, .. }
            | Self::Call { span: s, .. }
            | Self::Index { span: s, .. }
            | Self::If { span: s, .. } => s,
            Self::Match(selection) => &selection.span,
        }
    }
}

pub(crate) fn statement_span(s: &Statement) -> &Span {
    match s {
        Statement::Block(b) => &b.span,
        Statement::Let { span, .. }
        | Statement::Assign { span, .. }
        | Statement::Return { span, .. }
        | Statement::GuardControl { span, .. }
        | Statement::Value { span, .. }
        | Statement::If { span, .. }
        | Statement::While { span, .. }
        | Statement::For { span, .. }
        | Statement::Expr { span, .. } => span,
        Statement::Match(selection) => &selection.span,
    }
}
fn else_span(e: &Else) -> &Span {
    match e {
        Else::Block(b) => &b.span,
        Else::If(e) => e.span(),
    }
}

type ParseResult = Result<(Ast, Vec<usize>), Box<Diagnostic>>;
#[derive(Clone, Copy)]
enum TerminatedStatement {
    Binding,
    Other,
}
type WorkerResult = Result<ParseResult, Box<dyn std::any::Any + Send>>;
struct ParseJob {
    file: String,
    source: String,
    response: Sender<WorkerResult>,
    new_profile: bool,
    f5_operators: bool,
    work: Option<std::sync::Arc<std::sync::atomic::AtomicU64>>,
}
thread_local! {
    static PARSE_WORKER: RefCell<Option<Sender<ParseJob>>> = const { RefCell::new(None) };
}
#[cfg(test)]
thread_local! {
    static PARSE_WORKER_SPAWNS: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

pub(crate) fn worker_unavailable(file: &str, source: &str) -> Box<Diagnostic> {
    // A failed worker allocation cannot safely recurse on the caller's unknown stack.
    let parser = Parser {
        file,
        source,
        tokens: Vec::new(),
        at: 0,
        statement_start: 0,
        split_type_close: None,
        split_type_closes: Vec::new(),
        depth: 0,
        function_names: HashSet::new(),
        local_scopes: Vec::new(),
        work: None,
        new_profile: false,
        f5_operators: false,
        pending_match: None,
        guard_context: 0,
        callable_count: 0,
    };
    parser.budget_error(
        "syntax_nesting",
        128,
        129,
        parser.span(0, source.chars().next().map_or(0, char::len_utf8)),
        Stage::Parse,
    )
}

pub fn parse(file: &str, source: &str) -> Result<Ast, Box<Diagnostic>> {
    parse_with_split_tokens(file, source).map(|(ast, _)| ast)
}

pub(crate) fn parse_new_profile(file: &str, source: &str) -> Result<Ast, Box<Diagnostic>> {
    parse_with_split_tokens_counted(file, source, None, true, false).map(|(ast, _)| ast)
}

#[cfg(test)]
pub(crate) fn parse_f5_hidden(file: &str, source: &str) -> Result<Ast, Box<Diagnostic>> {
    parse_with_split_tokens_counted(file, source, None, true, true).map(|(ast, _)| ast)
}

pub(crate) fn parse_with_split_tokens(
    file: &str,
    source: &str,
) -> Result<(Ast, Vec<usize>), Box<Diagnostic>> {
    parse_with_split_tokens_counted(file, source, None, false, false)
}

fn parse_with_split_tokens_counted(
    file: &str,
    source: &str,
    work: Option<std::sync::Arc<std::sync::atomic::AtomicU64>>,
    new_profile: bool,
    f5_operators: bool,
) -> Result<(Ast, Vec<usize>), Box<Diagnostic>> {
    if source.len() > 1_048_576 {
        let parser = Parser {
            file,
            source,
            tokens: Vec::new(),
            at: 0,
            statement_start: 0,
            split_type_close: None,
            split_type_closes: Vec::new(),
            depth: 0,
            function_names: HashSet::new(),
            local_scopes: Vec::new(),
            work: None,
            new_profile,
            f5_operators,
            pending_match: None,
            guard_context: 0,
            callable_count: 0,
        };
        return Err(parser.budget_error(
            "source_bytes",
            1_048_576,
            1_048_577,
            parser.span(1_048_576, 1_048_577),
            Stage::Input,
        ));
    }
    let (response, received) = mpsc::channel();
    let job = ParseJob {
        file: file.into(),
        source: source.into(),
        response,
        work,
        new_profile,
        f5_operators,
    };
    let sent = PARSE_WORKER.with(|slot| {
        let mut worker = slot.borrow_mut();
        if worker.is_none() {
            #[cfg(test)]
            PARSE_WORKER_SPAWNS.with(|count| count.set(count.get() + 1));
            let (sender, jobs) = mpsc::channel::<ParseJob>();
            std::thread::Builder::new()
                .stack_size(crate::frontend_stack_size("PARSER"))
                .spawn(move || {
                    for job in jobs {
                        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                            parse_inner(
                                &job.file,
                                &job.source,
                                job.work.clone(),
                                job.new_profile,
                                job.f5_operators,
                            )
                        }));
                        let _ = job.response.send(result);
                    }
                })
                .map_err(|_| ())?;
            *worker = Some(sender);
        }
        worker.as_ref().ok_or(())?.send(job).map_err(|_| ())
    });
    if sent.is_err() {
        return Err(worker_unavailable(file, source));
    }
    match received.recv() {
        Ok(Ok(result)) => result,
        Ok(Err(payload)) => std::panic::resume_unwind(payload),
        Err(_) => Err(worker_unavailable(file, source)),
    }
}

fn parse_inner(
    file: &str,
    source: &str,
    work: Option<std::sync::Arc<std::sync::atomic::AtomicU64>>,
    new_profile: bool,
    f5_operators: bool,
) -> ParseResult {
    #[cfg(test)]
    let _allocation_capture = crate::test_alloc::capture_for(file);
    let tokens = lex::lex(file, source)?;
    // Terminators belong to the active statement. Preserve source gaps and
    // decide at its descent boundary; a token-only pre-pass cannot distinguish
    // a function suffix from an expression or a block inside parentheses.
    let tokens: Vec<_> = tokens
        .into_iter()
        .filter(|t| t.kind != Kind::Newline)
        .collect();
    let function_names = tokens
        .windows(2)
        .filter(|pair| pair[0].text == "fn" && pair[1].kind == Kind::Name)
        .map(|pair| pair[1].text.clone())
        .collect();
    Parser {
        file,
        source,
        tokens,
        at: 0,
        statement_start: 0,
        split_type_close: None,
        split_type_closes: Vec::new(),
        depth: 0,
        function_names,
        local_scopes: Vec::new(),
        work,
        new_profile,
        f5_operators,
        pending_match: None,
        guard_context: 0,
        callable_count: 0,
    }
    .program()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diagnostic::DataValue;

    #[test]
    fn production_table_has_108_declared_entry_fixtures() {
        // The spellings and A/D/X decisions are transcribed independently
        // from J14-b1-r1 §R1. The table itself must not supply the oracle.
        let fixtures: [(&str, &str, Option<&str>); 36] = [
            ("fn", "AAA", None),
            ("gen", "AXX", None),
            ("for", "XAX", None),
            ("yield", "XAX", None),
            ("break", "XAX", None),
            ("continue", "XAX", None),
            ("match", "XAA", None),
            ("if", "XAA", None),
            ("while", "XAX", None),
            ("let", "XAX", None),
            ("var", "XAX", None),
            ("return", "XAX", None),
            ("@", "DDD", Some("attributes")),
            ("struct", "DDD", Some("user_types")),
            ("enum", "DDD", Some("user_types")),
            ("type", "DDD", Some("user_types")),
            ("trait", "DDD", Some("traits_generics")),
            ("impl", "DDD", Some("traits_generics")),
            ("where", "DDD", Some("traits_generics")),
            ("dyn", "DDD", Some("traits_generics")),
            ("use", "DDD", Some("modules_visibility")),
            ("mod", "DDD", Some("modules_visibility")),
            ("pub", "DDD", Some("modules_visibility")),
            ("extend", "DDD", Some("extensions")),
            ("extension", "DDD", Some("extensions")),
            ("const", "DDD", Some("comptime")),
            ("comptime", "DDD", Some("comptime")),
            ("async", "DDD", Some("async")),
            ("await", "DDD", Some("async")),
            ("region", "DDD", Some("region")),
            ("self", "DDD", Some("methods")),
            ("super", "XXX", None),
            ("package", "XXX", None),
            ("and", "XXX", None),
            ("or", "XXX", None),
            ("not", "XAA", None),
        ];
        assert_eq!(PRODUCTION_ROWS.len() * 3, 108);
        let mut seen = HashSet::new();
        for (word, decisions, feature) in fixtures {
            let token = lex::lex("matrix.n", word).unwrap().remove(0);
            let terminal = production(&token).expect(word);
            assert!(seen.insert(terminal), "duplicate terminal: {word}");
            for (index, state) in [
                GrammarState::ModuleItem,
                GrammarState::BlockItem,
                GrammarState::Primary,
            ]
            .into_iter()
            .enumerate()
            {
                let expected = match decisions.as_bytes()[index] {
                    b'A' => Entry::Active,
                    b'D' => Entry::Deferred(feature.unwrap()),
                    b'X' => Entry::Invalid,
                    _ => panic!("bad fixture: {word}"),
                };
                assert_eq!(
                    production_entry(&token, state).map(|(_, entry)| entry),
                    Some(expected),
                    "{word}/{index}"
                );
                let source = match index {
                    0 => word.to_string(),
                    1 => format!("fn Main(){{{word}}}"),
                    _ => format!("fn Main(){{let x={word};}}"),
                };
                let start = source.rfind(word).unwrap();
                let result = parse_new_profile("matrix.n", &source);
                match expected {
                    Entry::Deferred("methods") if word == "self" && index == 2 => {
                        assert!(
                            result.is_ok(),
                            "self expression must reach names: {result:?}"
                        );
                    }
                    Entry::Deferred(id) => {
                        let d = result.unwrap_err();
                        assert_eq!(d.code, "NX-E0006", "{word}/{index}");
                        assert_eq!(d.feature_id.as_deref(), Some(id), "{word}/{index}");
                        assert_eq!(d.stage, Stage::Parse);
                        assert_eq!(d.operation, Operation::SyntaxParse);
                        assert_eq!(d.primary_span.as_ref().unwrap().start as usize, start);
                        assert_eq!(
                            d.primary_span.as_ref().unwrap().end as usize,
                            start + word.len()
                        );
                        assert_eq!(
                            d.data.get("pending_scope"),
                            Some(&DataValue::String("later_adr".into()))
                        );
                        d.validate("matrix.n", &source).unwrap();
                    }
                    Entry::Invalid => {
                        let d = result.unwrap_err();
                        assert_eq!(d.code, "NX-E0002", "{word}/{index}: {d:?}");
                        assert_eq!(d.primary_span.as_ref().unwrap().start as usize, start);
                        assert_eq!(
                            d.primary_span.as_ref().unwrap().end as usize,
                            start + word.len()
                        );
                        assert_eq!(
                            d.parser_context.as_deref(),
                            Some(match index {
                                0 => "module_item",
                                1 => "block_item",
                                _ => "binding_initializer",
                            }),
                            "{word}/{index}"
                        );
                        d.validate("matrix.n", &source).unwrap();
                    }
                    Entry::Active => {
                        if let Err(d) = result {
                            assert_ne!(d.code, "NX-E0006", "{word}/{index}: {d:?}");
                            d.validate("matrix.n", &source).unwrap();
                        }
                    }
                }
            }
        }
        assert_eq!(seen.len(), 36);
        assert!(
            PRODUCTION_ROWS
                .iter()
                .all(|row| seen.contains(&row.production))
        );
    }

    #[test]
    fn parser_token_probe_units_scale_with_source_length() {
        fn measured(target: usize) -> u64 {
            let mut source = String::from("fn F(){");
            while source.len() + 8 < target {
                source.push_str("let x=0;");
            }
            source.push('}');
            let (parsed, units) = measure_parse_work(&source);
            assert!(parsed.is_ok());
            units
        }
        let small = measured(32 * 1024);
        let large = measured(64 * 1024);
        assert!(small > 0);
        assert!(large >= small);
        assert!(
            large <= small * 23 / 10,
            "parser probe units: {small} -> {large}"
        );
    }

    #[test]
    #[cfg(not(debug_assertions))]
    fn parser_token_patterns_scale_with_source_length() {
        fn source(pattern: &str, bytes: usize) -> String {
            let (prefix, unit, suffix) = match pattern {
                "statements" => ("fn F(){", "a;", "}"),
                "functions" => ("", "fn F(){}", ""),
                "expressions" => ("fn F(){let x=1", "+1", ";}"),
                "parameters" => ("fn F(", "x:view I64,", "x:view I64){}"),
                "arguments" => ("fn F(){F(", "1,", "1);}"),
                "blocks" => ("fn F(){", "{}", "}"),
                "bindings" => ("fn F(){", "let x=1;", "}"),
                "typed_bindings" => ("fn F(){", "let x:I64=1;", "}"),
                "assignments" => ("fn F(){var x=0;", "x=1;", "}"),
                "returns" => ("fn F(){", "return;", "}"),
                "if_statements" => ("fn F(){", "if true{}", "}"),
                "while_statements" => ("fn F(){", "while false{}", "}"),
                "array_literals" => ("fn F(){", "let x=[1];", "}"),
                "calls" => ("fn F(){", "F(1);", "}"),
                "lf_statements" => ("fn F(){", "a\n", "}"),
                "crlf_statements" => ("fn F(){", "a\r\n", "}"),
                "cr_statements" => ("fn F(){", "a\r", "}"),
                "commented_statements" => ("fn F(){", "a;//x\n", "}"),
                "spaced_bindings" => ("fn F(){", "let x = 1; ", "}"),
                "paren_exprs" => ("fn F(){", "(1);", "}"),
                "return_values" => ("fn F()->I64{", "return 1;", "}"),
                "indexed_expressions" => ("fn F(){", "a[1];", "}"),
                "adjacent_type_close" => ("fn F(){", "let a:Array<I64>=x;", "}"),
                "nested_type_close" => ("fn F(){", "let a:Result<Array<Array<I64>>,Bool>=x;", "}"),
                _ => unreachable!(),
            };
            format!(
                "{prefix}{}{suffix}",
                unit.repeat((bytes - prefix.len() - suffix.len()) / unit.len())
            )
        }
        fn measure(pattern: &str, bytes: usize) -> (u64, u64) {
            let source = source(pattern, bytes);
            crate::test_alloc::reset("parser_scale_counter.n");
            if let Err(error) = parse("parser_scale_counter.n", &source) {
                assert!(
                    error.code != "NX-E0001" && error.code != "NX-E0002",
                    "{pattern} is not a B source: {error:?}"
                );
            }
            crate::test_alloc::snapshot("parser_scale_counter.n")
        }
        for pattern in [
            "statements",
            "functions",
            "expressions",
            "parameters",
            "arguments",
            "blocks",
            "bindings",
            "typed_bindings",
            "assignments",
            "returns",
            "if_statements",
            "while_statements",
            "array_literals",
            "calls",
            "lf_statements",
            "crlf_statements",
            "cr_statements",
            "commented_statements",
            "spaced_bindings",
            "paren_exprs",
            "return_values",
            "indexed_expressions",
            "adjacent_type_close",
            "nested_type_close",
        ] {
            let samples: Vec<_> = [65_536, 131_072, 262_144, 524_288, 1_048_576]
                .into_iter()
                .map(|bytes| measure(pattern, bytes))
                .collect();
            for (metric, axis) in [("allocated bytes", 0), ("allocations", 1)] {
                let value = |sample: &(u64, u64)| if axis == 0 { sample.0 } else { sample.1 };
                eprintln!(
                    "linearity {pattern} {metric}: {:?}",
                    samples.iter().map(value).collect::<Vec<_>>()
                );
                assert!(
                    value(&samples[4]) <= value(&samples[2]).saturating_mul(8),
                    "{pattern} {metric}: {samples:?}"
                );
                for pair in samples.windows(2) {
                    assert!(
                        value(&pair[1]) <= value(&pair[0]).saturating_mul(7) / 2 + 1,
                        "{pattern} {metric}: {samples:?}"
                    );
                }
            }
        }
    }

    #[test]
    fn lexer_comment_scan_scales_with_source_bytes() {
        let measure = |mebibytes: usize| {
            let source = format!("fn Main(){{}}//{}", "x".repeat(mebibytes * 1_048_576));
            let (tokens, steps) = super::super::lex::measure_lex_work(|| {
                super::super::lex::lex("linear.n", &source).unwrap()
            });
            assert!(!tokens.is_empty());
            steps
        };
        let one = measure(1);
        let four = measure(4);
        assert!(four <= one * 8, "1 MiB {one}; 4 MiB {four}");
    }

    #[test]
    fn adversarial_inputs_preserve_parser_diagnostic_validity() {
        let soup = b"fn Main(){let x=([,)]?+-<>!=@/*\r\n0123value";
        for seed in 0..20_000usize {
            let mut state = (seed as u64 + 1).wrapping_mul(0x9e3779b97f4a7c15);
            let mut next = || {
                state ^= state << 13;
                state ^= state >> 7;
                state ^= state << 17;
                state as u8
            };
            let len = seed % 193;
            let bytes: Vec<u8> = match seed % 4 {
                0 => (0..len).map(|_| next()).collect(),
                1 => (0..len)
                    .map(|_| soup[(next() as usize) % soup.len()])
                    .collect(),
                2 => {
                    let mut b = b"fn Main(){let x=1;//".to_vec();
                    b.extend_from_slice(&[0xc3, 0xa9, 0xf0, 0x9f, 0x99, 0x82][..seed % 7]);
                    b
                }
                _ => format!("fn Main(){{let x={};}}", next() % 100).into_bytes(),
            };
            if let Ok(source) = crate::diagnostic::decode_utf8(bytes)
                && let Err(d) = parse("fuzz.n", &source)
            {
                d.validate("fuzz.n", &source)
                    .unwrap_or_else(|e| panic!("invalid parser diagnostic for seed {seed}: {e}"));
            }
        }
    }

    #[test]
    fn contextual_value_name_stays_an_ordinary_identifier() {
        let ast = parse(
            "x",
            "fn value(x:view I64)->I64{return x;} fn Main(){let value=2;let y=value -1;}",
        )
        .unwrap();
        assert_eq!(ast.functions[1].body.statements.len(), 2);
    }

    #[test]
    fn parser_small_source_worker_cost_is_measured() {
        use std::time::Instant;
        const COUNT: usize = 20_000;
        let source = "fn F(){let x=1;}";
        let start = Instant::now();
        for _ in 0..COUNT {
            std::hint::black_box(parse_inner("cost.n", source, None, false, false).unwrap());
        }
        let direct = start.elapsed();
        let before = PARSE_WORKER_SPAWNS.with(std::cell::Cell::get);
        let start = Instant::now();
        for _ in 0..COUNT {
            std::hint::black_box(parse("cost.n", source).unwrap());
        }
        let worker = start.elapsed();
        let spawns = PARSE_WORKER_SPAWNS.with(std::cell::Cell::get) - before;
        eprintln!("small parses: direct {direct:?}, worker {worker:?}");
        assert!(spawns <= 1, "{spawns} worker spawns for repeated parses");
    }

    #[test]
    fn unavailable_parser_worker_has_a_structural_budget_diagnostic() {
        let source = "fn F(){}";
        let d = worker_unavailable("worker.n", source);
        assert_eq!(d.code, "NX-E0003");
        assert_eq!(d.stage, Stage::Parse);
        assert_eq!(
            d.data.get("budget_kind"),
            Some(&DataValue::String("syntax_nesting".into()))
        );
        assert_eq!(d.data.get("limit"), Some(&DataValue::UInt(128)));
        assert_eq!(d.data.get("observed"), Some(&DataValue::UInt(129)));
        let span = d.primary_span.as_ref().unwrap();
        assert_eq!((&*span.file, span.start, span.end), ("worker.n", 0, 1));
        d.validate("worker.n", source).unwrap();
    }

    #[test]
    fn non_unit_returns_are_found_in_source_order_through_all_branches() {
        for source in [
            "fn F(){if a{return 1;}}",
            "fn F(){if a{return 1;}else{}}",
            "fn F(){if a{return 1;}else if b{}}",
            "fn F(){while true{return 1;}}",
            "fn F(){if a{}else if b{return 1;}}",
            "fn F(){if a{}else if b{}else{return 1;}}",
            "fn F(){let x=if a then {return 1;}else{value:2;};}",
            "fn F(){{return 1;}}",
            "fn F(){let x=if a then {value:0;}else (if b then {return 1;}else{value:2;});}",
        ] {
            let d = parse("return.n", source).unwrap_err();
            assert_eq!(d.code, "NX-E0006", "{source}: {d:?}");
            assert_eq!(
                d.feature_id.as_deref(),
                Some("inferred_non_unit_result"),
                "{source}"
            );
            assert_eq!(d.related_spans.len(), 1, "{source}");
            assert_eq!(
                d.related_spans[0].span.start as usize,
                source.find("return 1").unwrap(),
                "{source}"
            );
            d.validate("return.n", source).unwrap();
        }
    }
}

fn deferred_feature(word: &str) -> Option<&'static str> {
    match word {
        "@" => Some("attributes"),
        "struct" | "enum" | "type" => Some("user_types"),
        "trait" | "impl" | "where" | "dyn" => Some("traits_generics"),
        "use" | "mod" | "pub" => Some("modules_visibility"),
        "extend" | "extension" => Some("extensions"),
        "const" | "comptime" => Some("comptime"),
        "for" => Some("for_iteration"),
        "break" | "continue" => Some("loop_control"),
        "gen" | "yield" => Some("generator"),
        "async" | "await" => Some("async"),
        "region" => Some("region"),
        "match" => Some("match"),
        "self" => Some("methods"),
        _ => None,
    }
}

fn legacy_deferred(word: &str) -> Option<&'static str> {
    deferred_feature(word)
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum GrammarState {
    ModuleItem,
    BlockItem,
    Primary,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
enum Production {
    FnDecl,
    GenDecl,
    ForStmt,
    YieldStmt,
    BreakStmt,
    ContinueStmt,
    MatchStmt,
    IfStmt,
    WhileStmt,
    LetBinding,
    VarBinding,
    ReturnStmt,
    Attribute,
    StructDecl,
    EnumDecl,
    AliasDecl,
    TraitDecl,
    ImplDecl,
    WhereClause,
    DynType,
    UseDecl,
    ModDecl,
    PubModifier,
    ExtendDecl,
    Extension,
    ConstDecl,
    Comptime,
    Async,
    AwaitExpr,
    RegionStmt,
    SelfPath,
    SuperPath,
    PackagePath,
    AndOp,
    OrOp,
    NotOp,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Entry {
    Active,
    Deferred(&'static str),
    Invalid,
}

#[derive(Clone, Copy)]
struct ProductionRow {
    production: Production,
    entries: [Entry; 3],
}

const A: Entry = Entry::Active;
const X: Entry = Entry::Invalid;
const fn d(id: &'static str) -> Entry {
    Entry::Deferred(id)
}

// One row per reserved terminal in the exterior D15 entry grammar. The
// columns are ModuleItem, BlockItem, and Expr/Primary, respectively.
const PRODUCTION_ROWS: [ProductionRow; 36] = [
    ProductionRow {
        production: Production::FnDecl,
        entries: [A, A, A],
    },
    ProductionRow {
        production: Production::GenDecl,
        entries: [A, X, X],
    },
    ProductionRow {
        production: Production::ForStmt,
        entries: [X, A, X],
    },
    ProductionRow {
        production: Production::YieldStmt,
        entries: [X, A, X],
    },
    ProductionRow {
        production: Production::BreakStmt,
        entries: [X, A, X],
    },
    ProductionRow {
        production: Production::ContinueStmt,
        entries: [X, A, X],
    },
    ProductionRow {
        production: Production::MatchStmt,
        entries: [X, A, A],
    },
    ProductionRow {
        production: Production::IfStmt,
        entries: [X, A, A],
    },
    ProductionRow {
        production: Production::WhileStmt,
        entries: [X, A, X],
    },
    ProductionRow {
        production: Production::LetBinding,
        entries: [X, A, X],
    },
    ProductionRow {
        production: Production::VarBinding,
        entries: [X, A, X],
    },
    ProductionRow {
        production: Production::ReturnStmt,
        entries: [X, A, X],
    },
    ProductionRow {
        production: Production::Attribute,
        entries: [d("attributes"); 3],
    },
    ProductionRow {
        production: Production::StructDecl,
        entries: [d("user_types"); 3],
    },
    ProductionRow {
        production: Production::EnumDecl,
        entries: [d("user_types"); 3],
    },
    ProductionRow {
        production: Production::AliasDecl,
        entries: [d("user_types"); 3],
    },
    ProductionRow {
        production: Production::TraitDecl,
        entries: [d("traits_generics"); 3],
    },
    ProductionRow {
        production: Production::ImplDecl,
        entries: [d("traits_generics"); 3],
    },
    ProductionRow {
        production: Production::WhereClause,
        entries: [d("traits_generics"); 3],
    },
    ProductionRow {
        production: Production::DynType,
        entries: [d("traits_generics"); 3],
    },
    ProductionRow {
        production: Production::UseDecl,
        entries: [d("modules_visibility"); 3],
    },
    ProductionRow {
        production: Production::ModDecl,
        entries: [d("modules_visibility"); 3],
    },
    ProductionRow {
        production: Production::PubModifier,
        entries: [d("modules_visibility"); 3],
    },
    ProductionRow {
        production: Production::ExtendDecl,
        entries: [d("extensions"); 3],
    },
    ProductionRow {
        production: Production::Extension,
        entries: [d("extensions"); 3],
    },
    ProductionRow {
        production: Production::ConstDecl,
        entries: [d("comptime"); 3],
    },
    ProductionRow {
        production: Production::Comptime,
        entries: [d("comptime"); 3],
    },
    ProductionRow {
        production: Production::Async,
        entries: [d("async"); 3],
    },
    ProductionRow {
        production: Production::AwaitExpr,
        entries: [d("async"); 3],
    },
    ProductionRow {
        production: Production::RegionStmt,
        entries: [d("region"); 3],
    },
    ProductionRow {
        production: Production::SelfPath,
        entries: [d("methods"); 3],
    },
    ProductionRow {
        production: Production::SuperPath,
        entries: [X; 3],
    },
    ProductionRow {
        production: Production::PackagePath,
        entries: [X; 3],
    },
    ProductionRow {
        production: Production::AndOp,
        entries: [X; 3],
    },
    ProductionRow {
        production: Production::OrOp,
        entries: [X; 3],
    },
    ProductionRow {
        production: Production::NotOp,
        entries: [X, A, A],
    },
];

// This is only the lexical spelling-to-terminal conversion. All production
// decisions, including deferred features, come from PRODUCTION_ROWS.
fn production(token: &Token) -> Option<Production> {
    Some(match token.text.as_str() {
        "fn" => Production::FnDecl,
        "gen" => Production::GenDecl,
        "for" => Production::ForStmt,
        "yield" => Production::YieldStmt,
        "break" => Production::BreakStmt,
        "continue" => Production::ContinueStmt,
        "match" => Production::MatchStmt,
        "if" => Production::IfStmt,
        "while" => Production::WhileStmt,
        "let" => Production::LetBinding,
        "var" => Production::VarBinding,
        "return" => Production::ReturnStmt,
        "@" => Production::Attribute,
        "struct" => Production::StructDecl,
        "enum" => Production::EnumDecl,
        "type" => Production::AliasDecl,
        "trait" => Production::TraitDecl,
        "impl" => Production::ImplDecl,
        "where" => Production::WhereClause,
        "dyn" => Production::DynType,
        "use" => Production::UseDecl,
        "mod" => Production::ModDecl,
        "pub" => Production::PubModifier,
        "extend" => Production::ExtendDecl,
        "extension" => Production::Extension,
        "const" => Production::ConstDecl,
        "comptime" => Production::Comptime,
        "async" => Production::Async,
        "await" => Production::AwaitExpr,
        "region" => Production::RegionStmt,
        "self" => Production::SelfPath,
        "super" => Production::SuperPath,
        "package" => Production::PackagePath,
        "and" => Production::AndOp,
        "or" => Production::OrOp,
        "not" => Production::NotOp,
        _ => return None,
    })
}

fn production_row(token: &Token) -> Option<&'static ProductionRow> {
    let terminal = production(token)?;
    PRODUCTION_ROWS
        .iter()
        .find(|row| row.production == terminal)
}

fn production_try_excluded(token: &Token) -> bool {
    production_row(token).is_some_and(|row| row.production == Production::IfStmt)
}

fn module_choice(row: &ProductionRow) -> ModuleChoice {
    match row.production {
        Production::FnDecl => ModuleChoice::Fn,
        Production::GenDecl => ModuleChoice::Gen,
        _ => ModuleChoice::Invalid,
    }
}

fn block_choice(row: &ProductionRow) -> BlockChoice {
    match row.production {
        Production::MatchStmt => BlockChoice::Match,
        Production::YieldStmt | Production::BreakStmt | Production::ContinueStmt => {
            BlockChoice::Guard
        }
        Production::LetBinding | Production::VarBinding => BlockChoice::Binding,
        Production::ReturnStmt => BlockChoice::Return,
        Production::IfStmt => BlockChoice::If,
        Production::WhileStmt => BlockChoice::While,
        Production::ForStmt => BlockChoice::For,
        _ => BlockChoice::Expr,
    }
}

fn primary_choice(row: &ProductionRow) -> PrimaryChoice {
    match row.production {
        Production::IfStmt => PrimaryChoice::If,
        Production::MatchStmt => PrimaryChoice::Match,
        Production::FnDecl => PrimaryChoice::Fn,
        Production::NotOp => PrimaryChoice::Not,
        Production::SelfPath => PrimaryChoice::SelfPath,
        _ => PrimaryChoice::Atom,
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum ModuleChoice {
    Fn,
    Gen,
    Deferred,
    Invalid,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum BlockChoice {
    Match,
    Guard,
    Binding,
    Return,
    If,
    While,
    For,
    Expr,
    Deferred,
    Invalid,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum PrimaryChoice {
    If,
    Match,
    Fn,
    Not,
    SelfPath,
    Atom,
    Deferred,
    Invalid,
}

// nexo:full-profile-dispatch
fn production_entry(token: &Token, state: GrammarState) -> Option<(&'static ProductionRow, Entry)> {
    let column = match state {
        GrammarState::ModuleItem => 0,
        GrammarState::BlockItem => 1,
        GrammarState::Primary => 2,
    };
    production_row(token).map(|row| (row, row.entries[column]))
}

#[derive(Clone, Copy)]
enum ListKind {
    FunctionParameter,
    Capture,
    ClosureParameter,
    Argument,
    ArrayElement,
    ConstructorPattern,
    TypeArgument,
    MatchArm,
}

// An `edit` argument is consumed by call binding. Descendants of that
// argument, including index expressions, do not inherit that permission.
fn unadmitted_edit_in_block(block: &Block) -> Option<Span> {
    unadmitted_edit_walk(EditNode::Block(block))
}

#[derive(Clone, Copy)]
enum EditNode<'a> {
    Expr(&'a Expr, bool),
    Statement(&'a Statement),
    Block(&'a Block),
    Match(&'a Match),
    Else(&'a Else),
    Arm(&'a Arm),
}

fn unadmitted_edit_walk(root: EditNode<'_>) -> Option<Span> {
    let mut pending = vec![root];
    while let Some(node) = pending.pop() {
        match node {
            EditNode::Block(block) => {
                pending.extend(block.statements.iter().rev().map(EditNode::Statement));
            }
            EditNode::Statement(statement) => match statement {
                Statement::Let { value, .. }
                | Statement::Expr { value, .. }
                | Statement::Return {
                    value: Some(value), ..
                }
                | Statement::GuardControl {
                    value: Some(value), ..
                }
                | Statement::Value {
                    value: Some(value), ..
                } => {
                    pending.push(EditNode::Expr(value, false));
                }
                Statement::Assign { place, value, .. } => {
                    pending.push(EditNode::Expr(value, false));
                    pending.push(EditNode::Expr(place, false));
                }
                Statement::If {
                    condition,
                    then_block,
                    otherwise,
                    ..
                } => {
                    if let Some(otherwise) = otherwise {
                        pending.push(EditNode::Statement(otherwise));
                    }
                    pending.push(EditNode::Block(then_block));
                    pending.push(EditNode::Expr(condition, false));
                }
                Statement::While {
                    condition, body, ..
                } => {
                    pending.push(EditNode::Block(body));
                    pending.push(EditNode::Expr(condition, false));
                }
                Statement::For { iterable, body, .. } => {
                    pending.push(EditNode::Block(body));
                    pending.push(EditNode::Expr(iterable, false));
                }
                Statement::Match(selection) => pending.push(EditNode::Match(selection)),
                Statement::Block(block) => pending.push(EditNode::Block(block)),
                Statement::Return { value: None, .. }
                | Statement::GuardControl { value: None, .. }
                | Statement::Value { value: None, .. } => {}
            },
            EditNode::Expr(expr, immediate_argument) => match expr {
                Expr::Closure(closure) => pending.push(EditNode::Block(&closure.body)),
                Expr::Unary {
                    mode: Mode::Inout,
                    value,
                    span,
                } if !immediate_argument && !matches!(&**value, Expr::Call { .. }) => {
                    return Some(span.clone());
                }
                Expr::Unary { value, .. } | Expr::Group(value, _) => {
                    pending.push(EditNode::Expr(value, false));
                }
                Expr::Array(items, _) => {
                    pending.extend(items.iter().rev().map(|item| EditNode::Expr(item, false)));
                }
                Expr::Binary { left, right, .. }
                | Expr::Index {
                    target: left,
                    index: right,
                    ..
                } => {
                    pending.push(EditNode::Expr(right, false));
                    pending.push(EditNode::Expr(left, false));
                }
                Expr::Call { target, args, .. } => {
                    pending.extend(
                        args.iter()
                            .rev()
                            .map(|arg| EditNode::Expr(&arg.value, true)),
                    );
                    pending.push(EditNode::Expr(target, false));
                }
                Expr::If {
                    condition,
                    then_block,
                    otherwise,
                    ..
                } => {
                    if let Some(otherwise) = otherwise {
                        pending.push(EditNode::Else(otherwise));
                    }
                    pending.push(EditNode::Block(then_block));
                    pending.push(EditNode::Expr(condition, false));
                }
                Expr::Match(selection) => pending.push(EditNode::Match(selection)),
                Expr::Name(..) | Expr::Integer(..) | Expr::Bool(..) | Expr::Unit(..) => {}
            },
            EditNode::Match(selection) => {
                pending.extend(selection.arms.iter().rev().map(EditNode::Arm));
                pending.push(EditNode::Expr(&selection.scrutinee, false));
            }
            EditNode::Else(otherwise) => match otherwise {
                Else::Block(block) => pending.push(EditNode::Block(block)),
                Else::If(expr) => pending.push(EditNode::Expr(expr, false)),
            },
            EditNode::Arm(arm) => {
                pending.push(EditNode::Block(&arm.body));
                if let Some(guard) = &arm.guard {
                    pending.push(EditNode::Expr(guard, false));
                }
            }
        }
    }
    None
}
fn returns_unit(expr: &Expr) -> bool {
    let mut current = expr;
    while let Expr::Group(inner, _) = current {
        current = inner;
    }
    matches!(current, Expr::Unit(_))
}
fn non_unit_return(statement: &Statement) -> Option<Span> {
    #[derive(Clone, Copy)]
    enum Node<'a> {
        Statement(&'a Statement),
        Expr(&'a Expr),
        Block(&'a Block),
        Match(&'a Match),
        Else(&'a Else),
        Arm(&'a Arm),
    }
    let mut pending = vec![Node::Statement(statement)];
    while let Some(node) = pending.pop() {
        match node {
            Node::Statement(statement) => match statement {
                Statement::Return {
                    value: Some(value),
                    span,
                } if !returns_unit(value) => {
                    return Some(Span {
                        file: span.file.clone(),
                        start: span.start,
                        end: value.span().end,
                    });
                }
                Statement::Block(block) => pending.push(Node::Block(block)),
                Statement::If {
                    condition,
                    then_block,
                    otherwise,
                    ..
                } => {
                    if let Some(otherwise) = otherwise {
                        pending.push(Node::Statement(otherwise));
                    }
                    pending.push(Node::Block(then_block));
                    pending.push(Node::Expr(condition));
                }
                Statement::While {
                    condition, body, ..
                } => {
                    pending.push(Node::Block(body));
                    pending.push(Node::Expr(condition));
                }
                Statement::Match(selection) => pending.push(Node::Match(selection)),
                Statement::Let { value, .. }
                | Statement::Assign { value, .. }
                | Statement::Expr { value, .. }
                | Statement::Value {
                    value: Some(value), ..
                }
                | Statement::Return {
                    value: Some(value), ..
                }
                | Statement::GuardControl {
                    value: Some(value), ..
                } => {
                    pending.push(Node::Expr(value));
                }
                Statement::For { .. }
                | Statement::Return { value: None, .. }
                | Statement::GuardControl { value: None, .. }
                | Statement::Value { value: None, .. } => {}
            },
            Node::Expr(expr) => match expr {
                Expr::If {
                    condition,
                    then_block,
                    otherwise,
                    ..
                } => {
                    if let Some(otherwise) = otherwise {
                        pending.push(Node::Else(otherwise));
                    }
                    pending.push(Node::Block(then_block));
                    pending.push(Node::Expr(condition));
                }
                Expr::Group(value, _) | Expr::Unary { value, .. } => {
                    pending.push(Node::Expr(value));
                }
                Expr::Binary { left, right, .. }
                | Expr::Index {
                    target: left,
                    index: right,
                    ..
                } => {
                    pending.push(Node::Expr(right));
                    pending.push(Node::Expr(left));
                }
                Expr::Call { target, args, .. } => {
                    pending.extend(args.iter().rev().map(|arg| Node::Expr(&arg.value)));
                    pending.push(Node::Expr(target));
                }
                Expr::Array(items, _) => pending.extend(items.iter().rev().map(Node::Expr)),
                Expr::Match(selection) => pending.push(Node::Match(selection)),
                Expr::Closure(..)
                | Expr::Name(..)
                | Expr::Integer(..)
                | Expr::Bool(..)
                | Expr::Unit(..) => {}
            },
            Node::Block(block) => {
                pending.extend(block.statements.iter().rev().map(Node::Statement));
            }
            Node::Match(selection) => {
                pending.extend(selection.arms.iter().rev().map(Node::Arm));
                pending.push(Node::Expr(&selection.scrutinee));
            }
            Node::Else(otherwise) => match otherwise {
                Else::Block(block) => pending.push(Node::Block(block)),
                Else::If(expr) => pending.push(Node::Expr(expr)),
            },
            Node::Arm(arm) => {
                pending.push(Node::Block(&arm.body));
                if let Some(guard) = &arm.guard {
                    pending.push(Node::Expr(guard));
                }
            }
        }
    }
    None
}
#[derive(Clone, Copy)]
enum ConstructEntry {
    Block(bool),
    #[cfg(test)]
    Statement {
        value_target: bool,
        block_start: usize,
    },
}

enum ConstructResult {
    Block(Block),
    #[cfg(test)]
    Statement(Box<Statement>),
}

enum PrefixValue {
    Expr(Expr),
    Mode { mode: Mode, start: usize },
}

struct ClosureHead {
    fn_span: Span,
    start: usize,
    captures: Option<Vec<Capture>>,
    params: Vec<ClosureParameter>,
    result: Option<Type>,
    with_clause: Option<WithClause>,
}

struct Parser<'a> {
    file: &'a str,
    source: &'a str,
    tokens: Vec<Token>,
    at: usize,
    statement_start: usize,
    split_type_close: Option<Span>,
    split_type_closes: Vec<usize>,
    depth: usize,
    function_names: HashSet<String>,
    local_scopes: Vec<HashSet<String>>,
    work: Option<std::sync::Arc<std::sync::atomic::AtomicU64>>,
    new_profile: bool,
    pending_match: Option<(&'static str, Span)>,
    f5_operators: bool,
    guard_context: usize,
    callable_count: usize,
}
impl Parser<'_> {
    fn list_separator(&mut self, kind: ListKind, start: usize) -> Result<bool, Box<Diagnostic>> {
        if !self.eat(",") {
            return Ok(false);
        }
        let (close, context, element_first, trailing) = match kind {
            ListKind::FunctionParameter => (")", "grammar.Parameter", PARAMETER_FIRST, false),
            ListKind::Capture => (
                "]",
                "grammar.Capture",
                &["copy", "edit", "move", "view"][..],
                false,
            ),
            ListKind::ClosureParameter => (")", "grammar.ClosureParam", &["Name"][..], false),
            ListKind::Argument => (
                ")",
                "grammar.Argument",
                first(self.new_profile, FirstSet::Expr),
                false,
            ),
            ListKind::ArrayElement => (
                "]",
                "array_element",
                first(self.new_profile, FirstSet::Expr),
                false,
            ),
            ListKind::ConstructorPattern => (")", "grammar.Pattern", PATTERN_FIRST, false),
            ListKind::TypeArgument => (
                ">",
                "grammar.Type",
                first(self.new_profile, FirstSet::Type),
                false,
            ),
            ListKind::MatchArm => ("}", "grammar.Arm", PATTERN_FIRST, true),
        };
        if self.eof() || (self.at_text(close) && !trailing) {
            let prefix_start = if matches!(
                kind,
                ListKind::FunctionParameter | ListKind::Argument | ListKind::TypeArgument
            ) {
                self.peek().span.start as usize
            } else {
                start
            };
            return Err(self.err(context, element_first, prefix_start, None));
        }
        Ok(true)
    }
    fn charge_callable(&mut self, anchor: Span) -> Result<(), Box<Diagnostic>> {
        if self.callable_count == 1_024 {
            return Err(self.budget_error("function_count", 1_024, 1_025, anchor, Stage::Parse));
        }
        self.callable_count += 1;
        Ok(())
    }
    fn parse_construct(
        &mut self,
        entry: ConstructEntry,
    ) -> Result<ConstructResult, Box<Diagnostic>> {
        struct BlockFrame {
            start: usize,
            value_target: bool,
            statements: Vec<Statement>,
            pending_value: Option<Span>,
        }
        struct MatchFrame {
            keyword: Token,
            expression: bool,
            mode: Option<Mode>,
            mode_span: Option<Span>,
            scrutinee: Expr,
            arms: Vec<Arm>,
        }
        enum Task {
            Enter(u8),
            BlockStart {
                value_target: bool,
            },
            StatementStart {
                value_target: bool,
                block_start: usize,
            },
            StatementBinding {
                start: usize,
                mutable: bool,
                name: String,
                annotation: Option<Type>,
            },
            StatementReturn {
                start: usize,
            },
            StatementGuard {
                start: usize,
                token: Token,
            },
            StatementValue {
                start: usize,
                marker: Span,
            },
            StatementExpr {
                start: usize,
            },
            StatementAssign {
                start: usize,
                place: Expr,
            },
            StatementWhileCondition {
                start: usize,
            },
            StatementWhileBody {
                start: usize,
                condition: Expr,
            },
            StatementForIterable {
                start: usize,
                name: Token,
            },
            StatementForBody {
                start: usize,
                name: Token,
                iterable: Expr,
            },
            StatementDirectBlock,
            StatementMatch,
            StatementIfCondition {
                branches: Vec<(usize, Expr, Block)>,
                start: usize,
            },
            StatementIfThen {
                branches: Vec<(usize, Expr, Block)>,
                start: usize,
                condition: Expr,
            },
            StatementIfElse {
                branches: Vec<(usize, Expr, Block)>,
            },
            MatchStart {
                expression: bool,
            },
            MatchAfterScrutinee {
                keyword: Token,
                expression: bool,
                mode: Option<Mode>,
                mode_span: Option<Span>,
            },
            MatchNextArm(MatchFrame),
            MatchAfterGuard {
                frame: MatchFrame,
                pattern: Pattern,
                guard_span: Option<Span>,
            },
            MatchAfterBody {
                frame: MatchFrame,
                pattern: Pattern,
                guard_span: Option<Span>,
                guard: Option<Expr>,
            },
            MatchFinish(MatchFrame),
            ExprAfterMatch,
            ClosureAfterBody(ClosureHead),
            BlockNext(BlockFrame),
            BlockAfterStatement {
                frame: BlockFrame,
                statement_token: usize,
                previous_start: usize,
            },
            Prefix,
            ModeOperand {
                mode: Mode,
                start: usize,
            },
            Continue {
                min: u8,
                logical_entry: bool,
                try_seen: bool,
            },
            Group {
                start: usize,
            },
            Array {
                start: usize,
                elements: Vec<Expr>,
                after_comma: bool,
                pending_element: bool,
            },
            Call {
                target: Expr,
                start: usize,
                arguments_start: usize,
                args: Vec<Argument>,
                after_comma: bool,
                pending_arg: Option<(usize, Option<String>)>,
            },
            Index {
                target: Expr,
                start: usize,
                selector_start: usize,
                index_start: usize,
            },
            Binary {
                left: Expr,
                op: String,
            },
            IfCondition {
                start: usize,
            },
            IfThenExpr {
                start: usize,
                condition: Expr,
            },
            IfThenBlock {
                start: usize,
                condition: Expr,
            },
            IfAfterThen {
                start: usize,
                condition: Expr,
                then_block: Block,
            },
            IfElseExpr {
                start: usize,
                condition: Expr,
                then_block: Block,
            },
            IfElseBlock {
                start: usize,
                condition: Expr,
                then_block: Block,
            },
        }
        fn value_block(operand: Expr) -> Block {
            let span = operand.span().clone();
            Block {
                statements: vec![Statement::Value {
                    value: Some(operand),
                    marker: None,
                    span: span.clone(),
                }],
                span: span.clone(),
                close_span: Span {
                    file: span.file.clone(),
                    start: span.end,
                    end: span.end,
                },
            }
        }
        fn accept_statement(
            parser: &mut Parser<'_>,
            frame: &mut BlockFrame,
            statement_token: usize,
            statement: Statement,
        ) -> Result<(), Box<Diagnostic>> {
            if matches!(
                &statement,
                Statement::If { .. }
                    | Statement::While { .. }
                    | Statement::For { .. }
                    | Statement::Block(_)
            ) && parser.at_text(";")
                && parser.peek().span.start == parser.peek().span.end
            {
                parser.take();
            }
            if let Some(value_span) = frame.pending_value.take() {
                let next_span = statement_span(&statement).clone();
                let mut diagnostic = parser.branch_error("non_final_branch_value", value_span);
                diagnostic.related_spans.push(RelatedSpan {
                    role: RelatedRole::Context,
                    span: next_span,
                });
                return Err(diagnostic);
            }
            if let Statement::Value { .. } = &statement {
                let value = &parser.tokens[statement_token];
                let colon = &parser.tokens[statement_token + 1];
                frame.pending_value =
                    Some(parser.span(value.span.start as usize, colon.span.end as usize));
            }
            frame.statements.push(statement);
            Ok(())
        }
        fn finish_if_statement(
            parser: &Parser<'_>,
            mut branches: Vec<(usize, Expr, Block)>,
            mut otherwise: Option<Else>,
        ) -> Result<Statement, Box<Diagnostic>> {
            while let Some((start, condition, then_block)) = branches.pop() {
                let end = otherwise
                    .as_ref()
                    .map_or(then_block.span.end as usize, |branch| {
                        else_span(branch).end as usize
                    });
                let candidate = Expr::If {
                    condition: Box::new(condition),
                    then_block,
                    otherwise: otherwise.map(Box::new),
                    span: parser.span(start, end),
                };
                let candidate = parser.checked_expression(candidate)?;
                if branches.is_empty()
                    && let Expr::If {
                        condition,
                        then_block,
                        otherwise,
                        span,
                    } = candidate
                {
                    return Ok(Statement::If {
                        condition: *condition,
                        then_block,
                        otherwise: otherwise.map(|branch| {
                            Box::new(match *branch {
                                Else::Block(block) => Statement::Block(block),
                                Else::If(expr) => Statement::Expr {
                                    span: expr.span().clone(),
                                    value: *expr,
                                },
                            })
                        }),
                        span,
                    });
                }
                otherwise = Some(Else::If(Box::new(candidate)));
            }
            unreachable!("if form has a branch")
        }
        let mut tasks = vec![match entry {
            ConstructEntry::Block(value_target) => Task::BlockStart { value_target },
            #[cfg(test)]
            ConstructEntry::Statement {
                value_target,
                block_start,
            } => Task::StatementStart {
                value_target,
                block_start,
            },
        }];
        let mut result: Option<Expr> = None;
        let mut block_result: Option<Block> = None;
        let mut statement_result: Option<Statement> = None;
        let mut match_result: Option<Match> = None;
        'work: while let Some(task) = tasks.pop() {
            match task {
                Task::BlockStart { value_target } => {
                    self.depth += 1;
                    if self.depth > 128 {
                        self.depth -= 1;
                        return Err(self.nesting_error("syntax_nesting"));
                    }
                    let start = self.peek().span.start as usize;
                    self.expect("{", "grammar.Block", &["{"], start)?;
                    self.local_scopes.push(HashSet::new());
                    tasks.push(Task::BlockNext(BlockFrame {
                        start,
                        value_target,
                        statements: Vec::new(),
                        pending_value: None,
                    }));
                }
                Task::BlockNext(frame) => {
                    if self.at_text("}") || self.eof() {
                        if self.eof() && frame.statements.is_empty() {
                            return Err(self.err(
                                "block_item",
                                first(self.new_profile, FirstSet::Block),
                                frame.start,
                                None,
                            ));
                        }
                        let close = self.peek().span.clone();
                        let end = close.end as usize;
                        self.expect(
                            "}",
                            "grammar.Block",
                            first(self.new_profile, FirstSet::Block),
                            frame.start,
                        )?;
                        self.depth -= 1;
                        self.local_scopes.pop();
                        block_result = Some(Block {
                            statements: frame.statements,
                            span: self.span(frame.start, end),
                            close_span: close,
                        });
                    } else {
                        if !self.starts_statement() {
                            return Err(self.err(
                                if frame.statements.is_empty() {
                                    "block_item"
                                } else {
                                    "grammar.Block"
                                },
                                first(self.new_profile, FirstSet::Block),
                                frame.start,
                                None,
                            ));
                        }
                        let statement_token = self.at;
                        let start = self.peek().span.start as usize;
                        let previous_start = std::mem::replace(&mut self.statement_start, start);
                        let value_target = frame.value_target;
                        let block_start = frame.start;
                        tasks.push(Task::BlockAfterStatement {
                            frame,
                            statement_token,
                            previous_start,
                        });
                        tasks.push(Task::StatementStart {
                            value_target,
                            block_start,
                        });
                    }
                }
                Task::BlockAfterStatement {
                    mut frame,
                    statement_token,
                    previous_start,
                } => {
                    self.statement_start = previous_start;
                    accept_statement(
                        self,
                        &mut frame,
                        statement_token,
                        statement_result.take().expect("statement result"),
                    )?;
                    tasks.push(Task::BlockNext(frame));
                }
                Task::StatementStart {
                    value_target,
                    block_start,
                } => {
                    let choice = self.statement_inner()?;
                    let start = self.peek().span.start as usize;
                    if choice == BlockChoice::Match {
                        tasks.push(Task::StatementMatch);
                        tasks.push(Task::MatchStart { expression: false });
                    } else if choice == BlockChoice::Guard {
                        let token = self.take();
                        if token.text == "yield" && !self.starts_expr() {
                            return Err(self.err(
                                "grammar.YieldStmt",
                                first(self.new_profile, FirstSet::Expr),
                                start,
                                None,
                            ));
                        }
                        if token.text == "yield" {
                            tasks.push(Task::StatementGuard { start, token });
                            tasks.push(Task::Enter(0));
                        } else {
                            let end = self.terminator(
                                start,
                                token.span.clone(),
                                TerminatedStatement::Other,
                            )?;
                            statement_result = Some(Statement::GuardControl {
                                token: token.text,
                                value: None,
                                guarded: self.guard_context > 0,
                                token_span: token.span,
                                span: self.span(start, end),
                            });
                        }
                    } else if choice == BlockChoice::Binding {
                        let mutable = self.eat("var");
                        if !mutable {
                            self.eat("let");
                        }
                        let name = self.expect_name("binding_name", &["Name"], start)?;
                        let annotation = if self.eat(":") {
                            Some(self.ty()?)
                        } else {
                            None
                        };
                        self.expect(
                            "=",
                            "grammar.Binding",
                            if annotation.is_some() {
                                &["="]
                            } else {
                                &[":", "="]
                            },
                            start,
                        )?;
                        if !self.starts_expr() {
                            return Err(self.err(
                                "binding_initializer",
                                first(self.new_profile, FirstSet::Expr),
                                start,
                                None,
                            ));
                        }
                        tasks.push(Task::StatementBinding {
                            start,
                            mutable,
                            name: name.text,
                            annotation,
                        });
                        tasks.push(Task::Enter(0));
                    } else if choice == BlockChoice::Return {
                        self.eat("return");
                        if !self.starts_expr() || self.line_break_before_next() {
                            let span = self.span(start, self.prev_end());
                            let end = self.terminator(start, span, TerminatedStatement::Other)?;
                            statement_result = Some(Statement::Return {
                                value: None,
                                span: self.span(start, end),
                            });
                        } else {
                            tasks.push(Task::StatementReturn { start });
                            tasks.push(Task::Enter(0));
                        }
                    } else if self.word("value")
                        && self.tokens.get(self.at + 1).is_some_and(|x| x.text == ":")
                        && self.source[self.peek().span.end as usize
                            ..self.tokens[self.at + 1].span.start as usize]
                            .find(['\r', '\n'])
                            .is_none()
                    {
                        self.take();
                        let colon = self.take();
                        let marker = self.span(start, colon.span.end as usize);
                        if !value_target {
                            let mut diagnostic = self.branch_error("invalid_value_target", marker);
                            diagnostic.related_spans.push(RelatedSpan {
                                role: RelatedRole::Context,
                                span: self.span(block_start, block_start + 1),
                            });
                            return Err(diagnostic);
                        }
                        if !self.starts_expr() || self.line_break_before_next() {
                            let mut diagnostic = self.branch_error("missing_value_operand", marker);
                            diagnostic.related_spans.push(RelatedSpan {
                                role: RelatedRole::Context,
                                span: self.span(block_start, block_start + 1),
                            });
                            let after = colon.span.end as usize;
                            let next = self.peek().span.start as usize;
                            let trivia = if next == after
                                && self.peek().span.start == self.peek().span.end
                            {
                                &self.source[after..]
                            } else {
                                &self.source[after..next]
                            };
                            let delimiter = if let Some(offset) = trivia.find(['\r', '\n']) {
                                let start = after + offset;
                                let len =
                                    if trivia.as_bytes().get(offset..offset + 2) == Some(b"\r\n") {
                                        2
                                    } else {
                                        1
                                    };
                                self.span(start, start + len)
                            } else {
                                self.peek().span.clone()
                            };
                            diagnostic.related_spans.push(RelatedSpan {
                                role: RelatedRole::Delimiter,
                                span: delimiter,
                            });
                            return Err(diagnostic);
                        }
                        tasks.push(Task::StatementValue { start, marker });
                        tasks.push(Task::Enter(0));
                    } else if choice == BlockChoice::If {
                        self.take();
                        tasks.push(Task::StatementIfCondition {
                            branches: Vec::new(),
                            start,
                        });
                        tasks.push(Task::Enter(0));
                    } else if choice == BlockChoice::While {
                        self.take();
                        tasks.push(Task::StatementWhileCondition { start });
                        tasks.push(Task::Enter(0));
                    } else if choice == BlockChoice::For {
                        self.eat("for");
                        if self.at_text("edit") {
                            let span = self.take().span;
                            let mut error = self.feature("for_iteration", span);
                            error.feature_id = Some("for_iteration".into());
                            error.message = "Recognized construct is outside the profile.".into();
                            error.data.insert(
                                "pending_scope".into(),
                                DataValue::String("later_adr".into()),
                            );
                            return Err(error);
                        }
                        let name = self.expect_name("grammar.ForBinding", &["Name"], start)?;
                        self.expect("in", "grammar.ForBinding", &["in"], start)?;
                        tasks.push(Task::StatementForIterable { start, name });
                        tasks.push(Task::Enter(0));
                    } else if self.at_text("{") {
                        tasks.push(Task::StatementDirectBlock);
                        tasks.push(Task::BlockStart {
                            value_target: false,
                        });
                    } else {
                        tasks.push(Task::StatementExpr { start });
                        tasks.push(Task::Enter(1));
                    }
                }
                Task::StatementBinding {
                    start,
                    mutable,
                    name,
                    annotation,
                } => {
                    let value = result.take().expect("binding value");
                    self.reject_function_value(&value)?;
                    let end =
                        self.terminator(start, value.span().clone(), TerminatedStatement::Binding)?;
                    if let Some(scope) = self.local_scopes.last_mut() {
                        scope.insert(name.clone());
                    }
                    statement_result = Some(Statement::Let {
                        mutable,
                        name,
                        annotation,
                        value,
                        span: self.span(start, end),
                    });
                }
                Task::StatementReturn { start } => {
                    let value = result.take().expect("return value");
                    self.reject_function_value(&value)?;
                    let end =
                        self.terminator(start, value.span().clone(), TerminatedStatement::Other)?;
                    statement_result = Some(Statement::Return {
                        value: Some(value),
                        span: self.span(start, end),
                    });
                }
                Task::StatementGuard { start, token } => {
                    let value = result.take().expect("yield value");
                    let end =
                        self.terminator(start, value.span().clone(), TerminatedStatement::Other)?;
                    statement_result = Some(Statement::GuardControl {
                        token: token.text,
                        value: Some(value),
                        guarded: self.guard_context > 0,
                        token_span: token.span,
                        span: self.span(start, end),
                    });
                }
                Task::StatementValue { start, marker } => {
                    let value = result.take().expect("branch value");
                    let end =
                        self.terminator(start, value.span().clone(), TerminatedStatement::Other)?;
                    statement_result = Some(Statement::Value {
                        value: Some(value),
                        marker: Some(marker),
                        span: self.span(start, end),
                    });
                }
                Task::StatementExpr { start } => {
                    let expression = result.take().expect("statement expression");
                    if is_place_expr(&expression) && self.eat("=") {
                        tasks.push(Task::StatementAssign {
                            start,
                            place: expression,
                        });
                        tasks.push(Task::Enter(0));
                    } else {
                        let end = self.terminator(
                            start,
                            expression.span().clone(),
                            TerminatedStatement::Other,
                        )?;
                        self.reject_function_value(&expression)?;
                        statement_result = Some(Statement::Expr {
                            value: expression,
                            span: self.span(start, end),
                        });
                    }
                }
                Task::StatementAssign { start, place } => {
                    let value = result.take().expect("assignment value");
                    self.reject_function_value(&value)?;
                    let end =
                        self.terminator(start, value.span().clone(), TerminatedStatement::Other)?;
                    statement_result = Some(Statement::Assign {
                        place,
                        value,
                        span: self.span(start, end),
                    });
                }
                Task::StatementWhileCondition { start } => {
                    let condition = result.take().expect("while condition");
                    tasks.push(Task::StatementWhileBody { start, condition });
                    tasks.push(Task::BlockStart {
                        value_target: false,
                    });
                }
                Task::StatementWhileBody { start, condition } => {
                    let body = block_result.take().expect("while body");
                    statement_result = Some(Statement::While {
                        condition,
                        span: self.span(start, body.span.end as usize),
                        body,
                    });
                }
                Task::StatementForIterable { start, name } => {
                    let iterable = result.take().expect("for iterable");
                    self.local_scopes.push(HashSet::from([name.text.clone()]));
                    tasks.push(Task::StatementForBody {
                        start,
                        name,
                        iterable,
                    });
                    tasks.push(Task::BlockStart {
                        value_target: false,
                    });
                }
                Task::StatementForBody {
                    start,
                    name,
                    iterable,
                } => {
                    self.local_scopes.pop();
                    let body = block_result.take().expect("for body");
                    statement_result = Some(Statement::For {
                        name: name.text,
                        name_span: name.span,
                        iterable,
                        span: self.span(start, body.span.end as usize),
                        body,
                    });
                }
                Task::StatementDirectBlock => {
                    statement_result =
                        Some(Statement::Block(block_result.take().expect("direct block")));
                }
                Task::StatementMatch => {
                    statement_result = Some(Statement::Match(
                        match_result.take().expect("match statement"),
                    ));
                }
                Task::StatementIfCondition { branches, start } => {
                    let condition = result.take().expect("if condition");
                    tasks.push(Task::StatementIfThen {
                        branches,
                        start,
                        condition,
                    });
                    tasks.push(Task::BlockStart {
                        value_target: false,
                    });
                }
                Task::StatementIfThen {
                    mut branches,
                    start,
                    condition,
                } => {
                    let then_block = block_result.take().expect("if block");
                    branches.push((start, condition, then_block));
                    if self.eat("else") {
                        if self.eof() || (!self.word("if") && !self.at_text("{")) {
                            return Err(self.err("grammar.IfForm", ELSE_FIRST, start, None));
                        }
                        if self.word("if") {
                            let next_start = self.take().span.start as usize;
                            tasks.push(Task::StatementIfCondition {
                                branches,
                                start: next_start,
                            });
                            tasks.push(Task::Enter(0));
                        } else {
                            tasks.push(Task::StatementIfElse { branches });
                            tasks.push(Task::BlockStart {
                                value_target: false,
                            });
                        }
                    } else {
                        statement_result = Some(finish_if_statement(self, branches, None)?);
                    }
                }
                Task::StatementIfElse { branches } => {
                    let otherwise = Else::Block(block_result.take().expect("else block"));
                    statement_result = Some(finish_if_statement(self, branches, Some(otherwise))?);
                }
                Task::MatchStart { expression } => {
                    let keyword = self.take();
                    let (mode, mode_span) = if self.word("move") || self.word("edit") {
                        let token = self.take();
                        let mode = if token.text == "move" {
                            Mode::Consume
                        } else {
                            Mode::Inout
                        };
                        (Some(mode), Some(token.span))
                    } else {
                        (None, None)
                    };
                    tasks.push(Task::MatchAfterScrutinee {
                        keyword,
                        expression,
                        mode,
                        mode_span,
                    });
                    tasks.push(Task::Enter(0));
                }
                Task::MatchAfterScrutinee {
                    keyword,
                    expression,
                    mode,
                    mode_span,
                } => {
                    let scrutinee = result.take().expect("match scrutinee");
                    if mode.is_some() && !is_place_expr(&scrutinee) {
                        let prefix = mode_span.as_ref().expect("match mode has span");
                        return Err(self.syntax(
                            &self.tokens[self.at.saturating_sub(1)],
                            if mode == Some(Mode::Consume) {
                                "consume_place"
                            } else {
                                "inout_place"
                            },
                            &["Place"],
                            prefix.start as usize,
                        ));
                    }
                    if mode == Some(Mode::Consume) && is_projected_place(&scrutinee) {
                        let mut diagnostic =
                            self.feature("partial_transfer", scrutinee.span().clone());
                        if let Some(span) = mode_span.clone() {
                            diagnostic.related_spans.push(RelatedSpan {
                                role: RelatedRole::Context,
                                span,
                            });
                        }
                        return Err(diagnostic);
                    }
                    self.expect("{", "grammar.Match", &["{"], keyword.span.start as usize)?;
                    tasks.push(Task::MatchNextArm(MatchFrame {
                        keyword,
                        expression,
                        mode,
                        mode_span,
                        scrutinee,
                        arms: Vec::new(),
                    }));
                }
                Task::MatchNextArm(frame) => {
                    if self.at_text("}") || self.eof() {
                        tasks.push(Task::MatchFinish(frame));
                    } else {
                        let pattern = self.pattern()?;
                        let mut bindings = HashSet::new();
                        collect_pattern_bindings(&pattern, &mut bindings);
                        self.local_scopes.push(bindings);
                        let guard_span = self.word("if").then(|| self.take().span);
                        if !self.new_profile
                            && let Some(span) = &guard_span
                        {
                            self.note_pending("match_guard", span.clone());
                        }
                        if guard_span.is_some() {
                            self.guard_context += 1;
                            tasks.push(Task::MatchAfterGuard {
                                frame,
                                pattern,
                                guard_span,
                            });
                            tasks.push(Task::Enter(0));
                        } else {
                            let expression = frame.expression;
                            self.expect(
                                "=>",
                                "grammar.Arm",
                                &["=>"],
                                pattern.span().start as usize,
                            )?;
                            tasks.push(Task::MatchAfterBody {
                                frame,
                                pattern,
                                guard_span: None,
                                guard: None,
                            });
                            tasks.push(Task::BlockStart {
                                value_target: expression,
                            });
                        }
                    }
                }
                Task::MatchAfterGuard {
                    frame,
                    pattern,
                    guard_span,
                } => {
                    self.guard_context -= 1;
                    let guard = result.take().expect("match guard");
                    self.expect("=>", "grammar.Arm", &["=>"], pattern.span().start as usize)?;
                    let expression = frame.expression;
                    tasks.push(Task::MatchAfterBody {
                        frame,
                        pattern,
                        guard_span,
                        guard: Some(guard),
                    });
                    tasks.push(Task::BlockStart {
                        value_target: expression,
                    });
                }
                Task::MatchAfterBody {
                    mut frame,
                    pattern,
                    guard_span,
                    guard,
                } => {
                    let body = block_result.take().expect("match arm body");
                    self.local_scopes.pop();
                    let span = self.span(pattern.span().start as usize, body.span.end as usize);
                    frame.arms.push(Arm {
                        pattern,
                        guard,
                        guard_span,
                        body,
                        span,
                    });
                    if self.list_separator(ListKind::MatchArm, frame.keyword.span.start as usize)? {
                        tasks.push(Task::MatchNextArm(frame));
                    } else {
                        tasks.push(Task::MatchFinish(frame));
                    }
                }
                Task::MatchFinish(frame) => {
                    self.expect(
                        "}",
                        "grammar.Match",
                        &[",", "}"],
                        frame.keyword.span.start as usize,
                    )?;
                    match_result = Some(Match {
                        scrutinee: Box::new(frame.scrutinee),
                        mode: frame.mode,
                        mode_span: frame.mode_span,
                        arms: frame.arms,
                        span: self.span(frame.keyword.span.start as usize, self.prev_end()),
                        keyword_span: frame.keyword.span,
                    });
                }
                Task::ExprAfterMatch => {
                    result = Some(Expr::Match(Box::new(
                        match_result.take().expect("match expression"),
                    )));
                }
                Task::ClosureAfterBody(head) => {
                    let body = block_result.take().expect("closure body");
                    self.local_scopes.pop();
                    let span = self.span(head.start, body.span.end as usize);
                    result = Some(Expr::Closure(Box::new(Closure {
                        fn_span: head.fn_span,
                        captures: head.captures,
                        params: head.params,
                        result: head.result,
                        with_clause: head.with_clause,
                        body,
                        span,
                    })));
                }
                Task::Enter(min) => {
                    let logical_entry = min == 0;
                    if logical_entry && self.depth == 128 {
                        return Err(self.nesting_error("syntax_nesting"));
                    }
                    if logical_entry {
                        self.depth += 1;
                    }
                    if min > 0 && !self.starts_try() {
                        return Err(self.err(
                            "grammar.TryExpr",
                            first(self.new_profile, FirstSet::Try),
                            self.peek().span.start as usize,
                            None,
                        ));
                    }
                    tasks.push(Task::Continue {
                        min,
                        logical_entry,
                        try_seen: false,
                    });
                    tasks.push(Task::Prefix);
                }
                Task::Prefix => {
                    let token = self.peek().clone();
                    if self.select_primary_entry() == PrimaryChoice::If {
                        self.take();
                        tasks.push(Task::IfCondition {
                            start: token.span.start as usize,
                        });
                        tasks.push(Task::Enter(0));
                    } else if self.select_primary_entry() == PrimaryChoice::Match {
                        tasks.push(Task::ExprAfterMatch);
                        tasks.push(Task::MatchStart { expression: true });
                    } else if self.select_primary_entry() == PrimaryChoice::Fn {
                        if !self.new_profile {
                            return Err(self.feature("closure", token.span));
                        }
                        let head = self.closure_head()?;
                        self.local_scopes
                            .push(head.params.iter().map(|p| p.name.clone()).collect());
                        tasks.push(Task::ClosureAfterBody(head));
                        tasks.push(Task::BlockStart {
                            value_target: false,
                        });
                    } else if self.eat("(") {
                        let start = token.span.start as usize;
                        if self.eat(")") {
                            result = Some(Expr::Unit(self.span(start, self.prev_end())));
                        } else {
                            if !self.starts_expr() {
                                return Err(self.err(
                                    "grammar.Primary",
                                    first(self.new_profile, FirstSet::Group),
                                    start,
                                    None,
                                ));
                            }
                            tasks.push(Task::Group { start });
                            tasks.push(Task::Enter(0));
                        }
                    } else if self.eat("[") {
                        tasks.push(Task::Array {
                            start: token.span.start as usize,
                            elements: Vec::new(),
                            after_comma: false,
                            pending_element: false,
                        });
                    } else {
                        match self.prefix()? {
                            PrefixValue::Expr(expr) => result = Some(expr),
                            PrefixValue::Mode { mode, start } => {
                                tasks.push(Task::ModeOperand { mode, start });
                                tasks.push(Task::Enter(4));
                            }
                        }
                    }
                }
                Task::ModeOperand { mode, start } => {
                    let val = result.take().expect("mode operand has a result");
                    let receiver_call = self.new_profile && matches!(val, Expr::Call { .. });
                    if has_call_root(&val) && !receiver_call {
                        return Err(self.feature("receiver_mode", self.span(start, start + 4)));
                    }
                    if let Expr::Call { target, .. } = &val
                        && receiver_call
                        && let Some(bad) = place_root_failure(target)
                    {
                        let tok = self
                            .tokens
                            .iter()
                            .find(|t| t.span.start == bad)
                            .cloned()
                            .unwrap_or_else(|| self.peek().clone());
                        return Err(self.syntax(
                            &tok,
                            if mode == Mode::Consume {
                                "consume_place"
                            } else {
                                "inout_place"
                            },
                            &["Place"],
                            start,
                        ));
                    }
                    if let Some(bad) = (!receiver_call).then(|| place_root_failure(&val)).flatten()
                    {
                        let tok = self
                            .tokens
                            .iter()
                            .find(|t| t.span.start == bad)
                            .cloned()
                            .unwrap_or_else(|| self.peek().clone());
                        return Err(self.syntax(
                            &tok,
                            if mode == Mode::Consume {
                                "consume_place"
                            } else {
                                "inout_place"
                            },
                            &["Place"],
                            start,
                        ));
                    }
                    if mode == Mode::Consume && is_projected_place(&val) {
                        let mut d = self.feature("partial_transfer", val.span().clone());
                        d.related_spans.push(RelatedSpan {
                            role: RelatedRole::Context,
                            span: self.span(start, start + 4),
                        });
                        return Err(d);
                    }
                    let span = self.span(start, val.span().end as usize);
                    let expression = Expr::Unary {
                        mode,
                        value: Box::new(val),
                        span,
                    };
                    result = Some(self.checked_expression(expression)?);
                }
                Task::Group { start } => {
                    let inner = result.take().expect("group expression has a result");
                    if self.eat(",") {
                        return Err(self.feature("tuple", self.prev_span()));
                    }
                    let end = self.peek().span.end as usize;
                    self.expect(")", "grammar.Primary", &[")"], start)?;
                    result = Some(Expr::Group(Box::new(inner), self.span(start, end)));
                }
                Task::Array {
                    start,
                    mut elements,
                    mut after_comma,
                    pending_element,
                } => {
                    if pending_element {
                        elements.push(result.take().expect("array element has a result"));
                        if self.list_separator(ListKind::ArrayElement, start)? {
                            after_comma = true;
                        } else {
                            let end = self.peek().span.end as usize;
                            self.expect("]", "grammar.Primary", &[",", "]"], start)?;
                            let span = self.span(start, end);
                            let array =
                                self.checked_expression(Expr::Array(elements, span.clone()))?;
                            if matches!(&array, Expr::Array(elements, _) if elements.iter().any(is_resource_constructor))
                            {
                                let mut d = self.feature("resource_array", span);
                                d.stage = Stage::Types;
                                d.operation = Operation::TypesCheck;
                                drop_partial_expression(array);
                                return Err(d);
                            }
                            result = Some(array);
                            continue;
                        }
                    }
                    if self.at_text("]") || self.eof() {
                        let end = self.peek().span.end as usize;
                        self.expect(
                            "]",
                            "grammar.Primary",
                            if elements.is_empty() {
                                first(self.new_profile, FirstSet::Array)
                            } else {
                                &[",", "]"]
                            },
                            start,
                        )?;
                        let span = self.span(start, end);
                        let array = self.checked_expression(Expr::Array(elements, span.clone()))?;
                        if matches!(&array, Expr::Array(elements, _) if elements.iter().any(is_resource_constructor))
                        {
                            let mut d = self.feature("resource_array", span);
                            d.stage = Stage::Types;
                            d.operation = Operation::TypesCheck;
                            drop_partial_expression(array);
                            return Err(d);
                        }
                        result = Some(array);
                    } else {
                        if !self.starts_expr() {
                            return Err(self.err(
                                if after_comma {
                                    "array_element"
                                } else {
                                    "grammar.Primary"
                                },
                                if after_comma {
                                    first(self.new_profile, FirstSet::Expr)
                                } else {
                                    first(self.new_profile, FirstSet::Array)
                                },
                                start,
                                None,
                            ));
                        }
                        tasks.push(Task::Array {
                            start,
                            elements,
                            after_comma,
                            pending_element: true,
                        });
                        tasks.push(Task::Enter(0));
                    }
                }
                Task::Call {
                    target,
                    start,
                    arguments_start,
                    mut args,
                    mut after_comma,
                    pending_arg,
                } => {
                    if let Some((argument_start, label)) = pending_arg {
                        let value = result.take().expect("call argument has a result");
                        self.reject_function_value(&value)?;
                        args.push(Argument {
                            label,
                            value,
                            span: self.span(argument_start, self.prev_end()),
                        });
                        if self
                            .list_separator(ListKind::Argument, self.peek().span.start as usize)?
                        {
                            after_comma = true;
                        } else {
                            let end = self.peek().span.end as usize;
                            self.expect(
                                ")",
                                "grammar.Arguments",
                                if args.is_empty() {
                                    first(self.new_profile, FirstSet::Group)
                                } else {
                                    &[")", ","]
                                },
                                arguments_start,
                            )?;
                            let call = Expr::Call {
                                target: Box::new(target),
                                args,
                                span: self.span(start, end),
                            };
                            result = Some(self.checked_expression(call)?);
                            continue;
                        }
                    }
                    if self.at_text(")") || self.eof() {
                        let end = self.peek().span.end as usize;
                        self.expect(
                            ")",
                            "grammar.Arguments",
                            if args.is_empty() {
                                first(self.new_profile, FirstSet::Group)
                            } else {
                                &[")", ","]
                            },
                            arguments_start,
                        )?;
                        let call = Expr::Call {
                            target: Box::new(target),
                            args,
                            span: self.span(start, end),
                        };
                        result = Some(self.checked_expression(call)?);
                    } else {
                        if !self.starts_expr() {
                            return Err(self.err(
                                if after_comma {
                                    "grammar.Argument"
                                } else {
                                    "grammar.Arguments"
                                },
                                if after_comma {
                                    first(self.new_profile, FirstSet::Expr)
                                } else {
                                    first(self.new_profile, FirstSet::Group)
                                },
                                if after_comma {
                                    self.peek().span.start as usize
                                } else {
                                    arguments_start
                                },
                                None,
                            ));
                        }
                        let argument_start = self.peek().span.start as usize;
                        let label = if self.peek().kind == Kind::Name
                            && self.tokens.get(self.at + 1).is_some_and(|t| t.text == ":")
                        {
                            let name = self.take().text;
                            self.take();
                            Some(name)
                        } else {
                            None
                        };
                        tasks.push(Task::Call {
                            target,
                            start,
                            arguments_start,
                            args,
                            after_comma,
                            pending_arg: Some((argument_start, label)),
                        });
                        tasks.push(Task::Enter(0));
                    }
                }
                Task::Index {
                    target,
                    start,
                    selector_start,
                    index_start,
                } => {
                    let index = result.take().expect("index expression has a result");
                    let end = self.peek().span.end as usize;
                    self.expect("]", "grammar.Index", &["]"], index_start)?;
                    let selector = self.span(selector_start, self.prev_end());
                    let indexed = Expr::Index {
                        target: Box::new(target),
                        index: Box::new(index),
                        selector,
                        span: self.span(start, end),
                    };
                    result = Some(self.checked_expression(indexed)?);
                }
                Task::Binary { left, op } => {
                    let right = result.take().expect("binary right side has a result");
                    let start = left.span().start as usize;
                    let binary = Expr::Binary {
                        op,
                        left: Box::new(left),
                        right: Box::new(right),
                        span: self.span(start, self.prev_end()),
                    };
                    result = Some(self.checked_expression(binary)?);
                }
                Task::IfCondition { start } => {
                    let condition = result.take().expect("if condition has a result");
                    self.expect("then", "grammar.IfExpr", &["then"], start)?;
                    if self.at_text("{") {
                        tasks.push(Task::IfThenBlock { start, condition });
                        tasks.push(Task::BlockStart { value_target: true });
                    } else {
                        if !self.starts_try() {
                            return Err(self.err(
                                "grammar.BranchOperand",
                                first(self.new_profile, FirstSet::Branch),
                                self.peek().span.start as usize,
                                None,
                            ));
                        }
                        tasks.push(Task::IfThenExpr { start, condition });
                        tasks.push(Task::Enter(1));
                    }
                }
                Task::IfThenExpr { start, condition } => {
                    let then_block = value_block(result.take().expect("if branch has a result"));
                    tasks.push(Task::IfAfterThen {
                        start,
                        condition,
                        then_block,
                    });
                }
                Task::IfThenBlock { start, condition } => {
                    let then_block = block_result.take().expect("if expression block");
                    tasks.push(Task::IfAfterThen {
                        start,
                        condition,
                        then_block,
                    });
                }
                Task::IfAfterThen {
                    start,
                    condition,
                    then_block,
                } => {
                    if !self.eat("else") {
                        return Err(self.branch_error("missing_else", self.span(start, start + 2)));
                    }
                    if !self.at_text("{") && !self.starts_try() {
                        return Err(self.err(
                            "grammar.BranchOperand",
                            first(self.new_profile, FirstSet::Branch),
                            start,
                            None,
                        ));
                    }
                    if self.at_text("{") {
                        tasks.push(Task::IfElseBlock {
                            start,
                            condition,
                            then_block,
                        });
                        tasks.push(Task::BlockStart { value_target: true });
                    } else {
                        tasks.push(Task::IfElseExpr {
                            start,
                            condition,
                            then_block,
                        });
                        tasks.push(Task::Enter(1));
                    }
                }
                Task::IfElseExpr {
                    start,
                    condition,
                    then_block,
                } => {
                    let operand = result.take().expect("else branch has a result");
                    let mut ungrouped = &operand;
                    while let Expr::Group(inner, _) = ungrouped {
                        ungrouped = inner;
                    }
                    let otherwise = if matches!(ungrouped, Expr::If { .. }) {
                        let mut nested = operand;
                        while let Expr::Group(inner, _) = nested {
                            nested = *inner;
                        }
                        Else::If(Box::new(nested))
                    } else {
                        Else::Block(value_block(operand))
                    };
                    let expression = Expr::If {
                        condition: Box::new(condition),
                        then_block,
                        otherwise: Some(Box::new(otherwise)),
                        span: self.span(start, self.prev_end()),
                    };
                    result = Some(self.checked_expression(expression)?);
                }
                Task::IfElseBlock {
                    start,
                    condition,
                    then_block,
                } => {
                    let otherwise =
                        Else::Block(block_result.take().expect("else expression block"));
                    let expression = Expr::If {
                        condition: Box::new(condition),
                        then_block,
                        otherwise: Some(Box::new(otherwise)),
                        span: self.span(start, self.prev_end()),
                    };
                    result = Some(self.checked_expression(expression)?);
                }
                Task::Continue {
                    min,
                    logical_entry,
                    mut try_seen,
                } => {
                    let mut lhs = result.take().expect("expression prefix has a result");
                    loop {
                        if try_seen && matches!(self.peek().text.as_str(), "?" | "(" | "[") {
                            return Err(self.err(
                                "grammar.TryExpr",
                                &["+", "-", ";", "<", "=="],
                                lhs.span().start as usize,
                                None,
                            ));
                        }
                        if matches!(&lhs, Expr::If { .. })
                            && matches!(self.peek().text.as_str(), "(" | "[")
                        {
                            return Err(self.err(
                                "grammar.Expr",
                                &[";"],
                                lhs.span().start as usize,
                                None,
                            ));
                        }
                        if self.at_text("(") {
                            let start = lhs.span().start as usize;
                            self.take();
                            let arguments_start = self.prev_start();
                            tasks.push(Task::Continue {
                                min,
                                logical_entry,
                                try_seen,
                            });
                            tasks.push(Task::Call {
                                target: lhs,
                                start,
                                arguments_start,
                                args: Vec::new(),
                                after_comma: false,
                                pending_arg: None,
                            });
                            continue 'work;
                        }
                        if self.eat("[") {
                            let start = lhs.span().start as usize;
                            let selector_start = self.prev_start();
                            let index_start = self.prev_start();
                            tasks.push(Task::Continue {
                                min,
                                logical_entry,
                                try_seen,
                            });
                            tasks.push(Task::Index {
                                target: lhs,
                                start,
                                selector_start,
                                index_start,
                            });
                            tasks.push(Task::Enter(0));
                            continue 'work;
                        }
                        if self.at_text("?") {
                            if min == 4 {
                                break;
                            }
                            if matches!(&lhs, Expr::If { .. }) {
                                return Err(self.err(
                                    "grammar.Expr",
                                    &[";"],
                                    lhs.span().start as usize,
                                    None,
                                ));
                            }
                            self.take();
                            let start = lhs.span().start as usize;
                            lhs = Expr::Unary {
                                mode: Mode::Read,
                                value: Box::new(lhs),
                                span: self.span(start, self.prev_end()),
                            };
                            lhs = self.checked_expression(lhs)?;
                            try_seen = true;
                            continue;
                        }
                        let op = self.peek().text.as_str();
                        if op == "." {
                            if self.new_profile
                                && matches!(&lhs, Expr::Name(name, _) if name == "Iterator")
                                && self.tokens.get(self.at + 1).is_some_and(|next| {
                                    crate::prelude::pending_intrinsic(&format!(
                                        "Iterator.{}",
                                        next.text
                                    ))
                                    .is_some()
                                })
                            {
                                if self.function_names.contains("Iterator")
                                    || self
                                        .local_scopes
                                        .iter()
                                        .any(|scope| scope.contains("Iterator"))
                                {
                                    return Err(self.feature("methods", self.peek().span.clone()));
                                }
                                let start = lhs.span().start as usize;
                                self.take();
                                let member = self.take();
                                lhs = Expr::Name(
                                    format!("Iterator.{}", member.text),
                                    self.span(start, member.span.end as usize),
                                );
                                continue;
                            }
                            return Err(self.feature("methods", self.peek().span.clone()));
                        }
                        if op == ".." {
                            let token = self.peek().clone();
                            return Err(self.syntax(
                                &token,
                                "range_endpoint",
                                &["..<", "..="],
                                lhs.span().start as usize,
                            ));
                        }
                        if matches!(op, "..<" | "..=") {
                            return Err(self.feature("range", self.peek().span.clone()));
                        }
                        if op == "+=" {
                            return Err(
                                self.feature("compound_assignment", self.peek().span.clone())
                            );
                        }
                        if self.new_profile
                            && op == "<"
                            && matches!(&lhs, Expr::Name(..))
                            && self.generic_head_call()
                        {
                            return Err(self.feature("traits_generics", self.peek().span.clone()));
                        }
                        if matches!(op, "and" | "or")
                            || (!self.f5_operators
                                && matches!(op, "*" | "/" | "%" | "!=" | ">" | "<=" | ">="))
                        {
                            let token = self.peek().clone();
                            return Err(self.syntax(
                                &token,
                                "unsupported_binary_operator",
                                &["+", "-", "<", "=="],
                                self.statement_start,
                            ));
                        }
                        let prec = match op {
                            "==" | "<" | "!=" | ">" | "<=" | ">=" => 1,
                            "+" | "-" => 2,
                            "*" | "/" | "%" if self.f5_operators => 3,
                            _ => 0,
                        };
                        if prec < min || prec == 0 {
                            break;
                        }
                        if matches!(&lhs, Expr::If { .. }) {
                            return Err(self.err(
                                "grammar.Expr",
                                &[";"],
                                lhs.span().start as usize,
                                None,
                            ));
                        }
                        if prec == 1
                            && self.f5_operators
                            && matches!(&lhs, Expr::Binary { op, .. } if matches!(op.as_str(), "<" | "<=" | ">" | ">=" | "==" | "!="))
                        {
                            let token = self.peek().clone();
                            return Err(self.syntax(
                                &token,
                                "grammar.CompareExpr",
                                &[";", "and", "or"],
                                lhs.span().start as usize,
                            ));
                        }
                        if prec == 1
                            && !self.f5_operators
                            && matches!(&lhs, Expr::Binary { op, .. } if op == "<" || op == "==")
                        {
                            return Err(self.err(
                                "grammar.CompareExpr",
                                &[";"],
                                lhs.span().start as usize,
                                None,
                            ));
                        }
                        let operator = self.take();
                        tasks.push(Task::Continue {
                            min,
                            logical_entry,
                            try_seen,
                        });
                        tasks.push(Task::Binary {
                            left: lhs,
                            op: operator.text,
                        });
                        tasks.push(Task::Enter(prec + 1));
                        continue 'work;
                    }
                    if logical_entry {
                        self.depth -= 1;
                    }
                    result = Some(lhs);
                }
            }
        }
        match entry {
            ConstructEntry::Block(_) => Ok(ConstructResult::Block(
                block_result.expect("block has a result"),
            )),
            #[cfg(test)]
            ConstructEntry::Statement { .. } => Ok(ConstructResult::Statement(Box::new(
                statement_result.expect("statement has a result"),
            ))),
        }
    }

    // nexo:full-profile-dispatch
    fn select_module_entry(&self) -> ModuleChoice {
        if !self.new_profile {
            return self.select_module_entry_legacy();
        }
        match production_entry(self.peek(), GrammarState::ModuleItem) {
            Some((row, Entry::Active)) => module_choice(row),
            Some((_, Entry::Deferred(_))) => ModuleChoice::Deferred,
            _ => ModuleChoice::Invalid,
        }
    }
    fn select_module_entry_legacy(&self) -> ModuleChoice {
        if self.word("fn") {
            ModuleChoice::Fn
        } else {
            ModuleChoice::Invalid
        }
    }
    fn program(mut self) -> ParseResult {
        let start = 0;
        let mut fs = Vec::new();
        while !self.eof() {
            let choice = self.select_module_entry();
            if matches!(choice, ModuleChoice::Fn | ModuleChoice::Gen) {
                if let Some(name) = self
                    .tokens
                    .get(self.at + if self.word("gen") { 2 } else { 1 })
                    && name.kind == Kind::Name
                {
                    self.charge_callable(name.span.clone())?;
                }
                let generator_span = if self.word("gen") {
                    Some(self.take().span)
                } else {
                    None
                };
                fs.push(self.function(generator_span)?)
            } else {
                if let Some(error) = self.deferred_in_state(GrammarState::ModuleItem) {
                    return Err(error);
                }
                return Err(self.err(
                    "module_item",
                    first(self.new_profile, FirstSet::Module),
                    0,
                    None,
                ));
            }
        }
        if let Some((feature, span)) = self.pending_match.clone() {
            return Err(self.feature(feature, span));
        }
        Ok((
            Ast {
                functions: fs,
                span: self.span(start, self.source.len()),
            },
            self.split_type_closes,
        ))
    }
    fn function(&mut self, generator_span: Option<Span>) -> Result<Function, Box<Diagnostic>> {
        let start = generator_span
            .as_ref()
            .map_or(self.peek().span.start, |span| span.start) as usize;
        self.expect("fn", "grammar.Function", &["fn"], start)?;
        let name = self.expect_name("function_name", &["Name"], start)?;
        let ns = name.span.clone();
        self.expect("(", "grammar.Function", &["("], start)?;
        let mut params = Vec::new();
        let mut after_comma = false;
        while !self.at_text(")") && !self.eof() {
            if !self.starts_parameter() {
                return Err(self.err(
                    if after_comma {
                        "grammar.Parameter"
                    } else {
                        "parameter_list"
                    },
                    if after_comma {
                        PARAMETER_FIRST
                    } else {
                        PARAMETER_LIST_FIRST
                    },
                    if after_comma {
                        self.peek().span.start as usize
                    } else {
                        self.prev_start()
                    },
                    None,
                ));
            }
            let ps = self.peek().span.start as usize;
            let n = self.expect_name("grammar.Parameter", PARAMETER_FIRST, ps)?;
            self.expect(":", "grammar.Parameter", &[":"], ps)?;
            let mode = if self.eat("view") {
                Mode::Read
            } else if self.eat("edit") {
                Mode::Inout
            } else if self.eat("move") {
                Mode::Consume
            } else {
                return Err(self.err(
                    "grammar.ParamMode",
                    PARAM_MODE_FIRST,
                    self.peek().span.start as usize,
                    None,
                ));
            };
            let ty = self.ty()?;
            if self.at_text("=") {
                return Err(self.feature("default_argument", self.peek().span.clone()));
            }
            params.push(Parameter {
                mode,
                name: n.text,
                ty,
                span: self.span(ps, self.prev_end()),
            });
            if !self.list_separator(ListKind::FunctionParameter, self.peek().span.start as usize)? {
                break;
            }
            after_comma = true;
        }
        if self.eof() && !params.is_empty() {
            return Err(self.err(
                "grammar.Parameters",
                &[")", ","],
                params[0].span.start as usize,
                None,
            ));
        }
        self.expect(
            ")",
            if params.is_empty() {
                "parameter_list"
            } else {
                "grammar.Parameters"
            },
            if params.is_empty() {
                PARAMETER_LIST_FIRST
            } else {
                &[")", ","]
            },
            if params.is_empty() {
                start
            } else {
                params[0].span.start as usize
            },
        )?;
        let result = if self.eat("->") {
            Some(self.ty()?)
        } else {
            None
        };
        if self.at_text("from") {
            return Err(self.feature("provenance_clauses", self.peek().span.clone()));
        }
        let with_clause = if self.eat("with") {
            Some(self.with_clause(&name, false, generator_span.is_some())?)
        } else {
            None
        };
        if !self.at_text("{") {
            let t = self.peek().clone();
            let expected: &[&str] = if matches!(with_clause, Some(WithClause::Simple(_))) {
                &[",", "{"]
            } else if with_clause.is_some() {
                &["{"]
            } else if result.is_some() {
                &["with", "{"]
            } else {
                &["->", "with", "{"]
            };
            return Err(self.syntax(&t, "function_suffix", expected, start));
        }
        self.local_scopes
            .push(params.iter().map(|p| p.name.clone()).collect());
        let body = self.block()?;
        self.local_scopes.pop();
        // Classify a selected `edit` prefix before later semantic exclusions
        // and before lowering. A call consumes the permission only when the
        // prefix is the immediate argument expression.
        if let Some(span) = unadmitted_edit_in_block(&body) {
            let prefix = self
                .tokens
                .iter()
                .find(|token| token.span.start == span.start && token.text == "edit")
                .expect("parsed edit prefix has a source token");
            return Err(self.syntax(prefix, "inout_place", &["Place"], span.start as usize));
        }
        if result.is_none()
            && generator_span.is_none()
            && let Some(ret) = body.statements.iter().find_map(non_unit_return)
        {
            let mut d = self.feature("inferred_non_unit_result", ns.clone());
            d.stage = Stage::Types;
            d.operation = Operation::TypesCheck;
            d.related_spans.push(RelatedSpan {
                role: RelatedRole::Context,
                span: ret,
            });
            return Err(d);
        }
        let end = body.span.end as usize;
        Ok(Function {
            is_generator: generator_span.is_some(),
            name: name.text,
            name_span: ns,
            params,
            result,
            with_clause,
            body,
            span: self.span(start, end),
        })
    }
    fn closure_head(&mut self) -> Result<ClosureHead, Box<Diagnostic>> {
        let fn_token = self.take();
        let start = fn_token.span.start as usize;
        self.charge_callable(fn_token.span.clone())?;
        let captures = if self.eat("[") {
            let mut captures = Vec::new();
            while !self.at_text("]") && !self.eof() {
                let mode_token = self.take();
                let mode = match mode_token.text.as_str() {
                    "view" => CaptureMode::View,
                    "copy" => CaptureMode::Copy,
                    "edit" => CaptureMode::Edit,
                    "move" => CaptureMode::Move,
                    _ => {
                        return Err(self.syntax(
                            &mode_token,
                            "grammar.Capture",
                            &["copy", "edit", "move", "view"],
                            start,
                        ));
                    }
                };
                let name = self.expect_name("grammar.Capture", &["Name"], start)?;
                captures.push(Capture {
                    mode,
                    name: name.text,
                    span: self.span(mode_token.span.start as usize, name.span.end as usize),
                    name_span: name.span,
                });
                if !self.list_separator(ListKind::Capture, start)? {
                    break;
                }
            }
            self.expect("]", "grammar.CaptureList", &[",", "]"], start)?;
            Some(captures)
        } else {
            None
        };
        self.expect("(", "grammar.ClosureExpr", &["("], start)?;
        let mut params = Vec::new();
        while !self.at_text(")") && !self.eof() {
            let name = self.expect_name("grammar.ClosureParam", &["Name"], start)?;
            self.expect(":", "grammar.ClosureParam", &[":"], start)?;
            let mode_token = self.take();
            let mode = match mode_token.text.as_str() {
                "view" => Mode::Read,
                "edit" => Mode::Inout,
                "move" => Mode::Consume,
                _ => {
                    return Err(self.syntax(
                        &mode_token,
                        "grammar.ParamMode",
                        PARAM_MODE_FIRST,
                        start,
                    ));
                }
            };
            let ty = if !matches!(self.peek().text.as_str(), "," | ")") {
                Some(self.ty()?)
            } else {
                None
            };
            let span = self.span(name.span.start as usize, self.prev_end());
            params.push(ClosureParameter {
                mode,
                name: name.text,
                ty,
                span,
            });
            if !self.list_separator(ListKind::ClosureParameter, start)? {
                break;
            }
        }
        self.expect(")", "grammar.ClosureExpr", &[")", ","], start)?;
        let result = if self.eat("->") {
            Some(self.ty()?)
        } else {
            None
        };
        if self.at_text("from") {
            return Err(self.feature("provenance_clauses", self.peek().span.clone()));
        }
        let with_clause = if self.eat("with") {
            Some(self.with_clause(&fn_token, true, false)?)
        } else {
            None
        };
        if !self.at_text("{") {
            let token = self.peek().clone();
            return Err(self.syntax(&token, "grammar.ClosureBody", &["{"], start));
        }
        Ok(ClosureHead {
            fn_span: fn_token.span,
            start,
            captures,
            params,
            result,
            with_clause,
        })
    }
    fn with_clause(
        &mut self,
        function_name: &Token,
        closure: bool,
        generator: bool,
    ) -> Result<WithClause, Box<Diagnostic>> {
        if !self.at_text("{") && !self.starts_restriction() {
            return Err(self.err("grammar.WithClause", WITH_FIRST, self.prev_start(), None));
        }
        if self.at_text("{") {
            let open = self.take();
            let mut operations = Vec::new();
            let mut first_operation: BTreeMap<String, Span> = BTreeMap::new();
            let mut invalid: Option<Span> = None;
            while !self.at_text("}") && !self.eof() {
                let name = self.peek().clone();
                if !OPERATION_FIRST.contains(&name.text.as_str()) {
                    let empty = operations.is_empty();
                    return Err(self.err(
                        if empty {
                            "grammar.OperationList"
                        } else {
                            "grammar.Operation"
                        },
                        if empty {
                            OPERATION_LIST_FIRST
                        } else {
                            OPERATION_FIRST
                        },
                        if empty {
                            open.span.start as usize
                        } else {
                            name.span.start as usize
                        },
                        None,
                    ));
                }
                self.take();
                if let Some(first) = first_operation.get(&name.text) {
                    let mut d = self.with_error("duplicate_operation", name.span.clone());
                    d.related_spans.push(RelatedSpan {
                        role: RelatedRole::Context,
                        span: first.clone(),
                    });
                    return Err(d);
                }
                first_operation.insert(name.text.clone(), name.span.clone());
                let forbidden = if generator {
                    !matches!(name.text.as_str(), "create" | "resume" | "drop")
                } else {
                    !closure && name.text != "call"
                };
                if forbidden && invalid.is_none() {
                    invalid = Some(name.span.clone());
                }
                self.expect(
                    "(",
                    "grammar.OperationLimit",
                    &["("],
                    name.span.start as usize,
                )?;
                let restrictions = self.restrictions()?;
                self.expect(
                    ")",
                    "grammar.OperationLimit",
                    &[")", ","],
                    name.span.start as usize,
                )?;
                operations.push(OperationLimit {
                    name: name.text,
                    restrictions,
                    span: self.span(name.span.start as usize, self.prev_end()),
                });
                if !self.eat(",") {
                    break;
                }
                if self.at_text("}") || self.eof() {
                    return Err(self.err(
                        "grammar.Operation",
                        OPERATION_FIRST,
                        self.peek().span.start as usize,
                        None,
                    ));
                }
            }
            let close = self.peek().clone();
            self.expect(
                "}",
                "grammar.OperationList",
                if operations.is_empty() {
                    OPERATION_LIST_FIRST
                } else {
                    &[",", "}"]
                },
                open.span.start as usize,
            )?;
            if operations.is_empty() {
                return Err(self.with_error(
                    "empty_operation_list",
                    self.span(open.span.start as usize, close.span.end as usize),
                ));
            }
            if let Some(span) = invalid {
                let mut d = self.with_error("invalid_operation", span);
                d.related_spans.push(RelatedSpan {
                    role: RelatedRole::Context,
                    span: function_name.span.clone(),
                });
                return Err(d);
            }
            if !operations
                .iter()
                .any(|op| op.name == if generator { "resume" } else { "call" })
            {
                return Err(self.with_error(
                    "missing_body_operation",
                    self.span(open.span.start as usize, close.span.end as usize),
                ));
            }
            Ok(WithClause::Operations(operations))
        } else {
            let restrictions = self.restrictions()?;
            Ok(WithClause::Simple(restrictions))
        }
    }
    fn restrictions(&mut self) -> Result<Vec<Restriction>, Box<Diagnostic>> {
        let start = self.peek().span.start as usize;
        let mut items: Vec<Token> = Vec::new();
        loop {
            let token = self.peek().clone();
            if token.kind != Kind::Name || !is_restriction(&token.text) {
                return Err(self.syntax(
                    &token,
                    "grammar.RestrictionItem",
                    RESTRICTION_FIRST,
                    start,
                ));
            }
            self.take();
            if token.text == "no_heap_alloc" {
                return Err(self.feature("no_heap_alloc", token.span));
            }
            if let Some(first) = items.iter().find(|x| x.text == token.text) {
                let mut d = self.with_error("duplicate_restriction", token.span.clone());
                d.related_spans.push(RelatedSpan {
                    role: RelatedRole::Context,
                    span: first.span.clone(),
                });
                return Err(d);
            }
            items.push(token);
            if !self.eat(",") {
                break;
            }
        }
        if items.len() > 1 && items.iter().any(|x| x.text == "unrestricted") {
            return Err(self.with_error("unrestricted_mixed", self.span(start, self.prev_end())));
        }
        Ok(items
            .into_iter()
            .map(|token| Restriction {
                name: token.text,
                span: token.span,
            })
            .collect())
    }
    fn with_error(&self, subcause: &str, span: Span) -> Box<Diagnostic> {
        let mut d = self.base("NX-E0417", Stage::Parse, span, "Invalid with clause.");
        d.subcause = Some(subcause.into());
        d.rule = format!("NX-E0417/{subcause}");
        d.explanation_id = Some(d.rule.clone());
        d
    }
    fn ty(&mut self) -> Result<Type, Box<Diagnostic>> {
        enum Frame {
            Array,
            Option,
            ResultFirst,
            ResultSecond(Type),
        }
        let mut frames = Vec::new();
        'parse: loop {
            self.depth += 1;
            if self.depth > 128 {
                self.depth -= 1;
                return Err(self.nesting_error("syntax_nesting"));
            }
            let mut t = if self.eat("I64") {
                Type::I64
            } else if self.eat("Bool") {
                Type::Bool
            } else if self.eat("MockResource") {
                Type::MockResource
            } else if self.eat("MockBlockingResource") {
                Type::MockBlockingResource
            } else if self.eat("(") {
                self.expect(")", "grammar.Type", &[")"], self.prev_start())?;
                Type::Unit
            } else if self.eat("Array") {
                self.expect("<", "grammar.Type", &["<"], self.prev_start())?;
                frames.push(Frame::Array);
                continue 'parse;
            } else if self.eat("Result") {
                self.expect("<", "grammar.Type", &["<"], self.prev_start())?;
                frames.push(Frame::ResultFirst);
                continue 'parse;
            } else if self.new_profile && self.eat("Option") {
                self.expect("<", "grammar.Type", &["<"], self.prev_start())?;
                frames.push(Frame::Option);
                continue 'parse;
            } else {
                let tok = self.peek().clone();
                if tok.text == "Option" {
                    // The new profile consumed Option above; this is the legacy path.
                    return Err(self.feature("option", tok.span));
                }
                if tok.kind == Kind::Name {
                    let id = if matches!(tok.text.as_str(), "Future" | "TaskHandle" | "TaskOutcome")
                    {
                        "async"
                    } else {
                        "unsupported_type"
                    };
                    let mut end = tok.span.end;
                    let mut next = self.at + 1;
                    while self.tokens.get(next).is_some_and(|t| t.text == ".")
                        && self
                            .tokens
                            .get(next + 1)
                            .is_some_and(|t| t.kind == Kind::Name)
                    {
                        end = self.tokens[next + 1].span.end;
                        next += 2;
                    }
                    return Err(self.feature(id, self.span(tok.span.start as usize, end as usize)));
                }
                return Err(self.err(
                    "grammar.Type",
                    first(self.new_profile, FirstSet::Type),
                    self.peek().span.start as usize,
                    None,
                ));
            };
            loop {
                self.depth -= 1;
                match frames.pop() {
                    Some(Frame::Array) => {
                        self.expect_type_close()?;
                        t = Type::Array(Box::new(t));
                    }
                    Some(Frame::Option) => {
                        self.expect_type_close()?;
                        t = Type::Option(Box::new(t));
                    }
                    Some(Frame::ResultFirst) => {
                        if !self.list_separator(ListKind::TypeArgument, self.prev_start())? {
                            self.expect(",", "grammar.Type", &[","], self.prev_start())?;
                        }
                        frames.push(Frame::ResultSecond(t));
                        continue 'parse;
                    }
                    Some(Frame::ResultSecond(first)) => {
                        self.expect_type_close()?;
                        t = Type::Result(Box::new(first), Box::new(t));
                    }
                    None => return Ok(t),
                }
            }
        }
    }
    fn expect_type_close(&mut self) -> Result<(), Box<Diagnostic>> {
        if self.eat(">") {
            return Ok(());
        }
        if self.at_text(">=") {
            let t = self.peek().clone();
            self.split_type_closes.push(t.span.start as usize);
            // Keep the second half at the current index. The consumed '>' is
            // represented only as the previous span until '=' is taken.
            self.split_type_close =
                Some(self.span(t.span.start as usize, t.span.start as usize + 1));
            self.tokens[self.at].text = "=".into();
            self.tokens[self.at].span.start += 1;
            return Ok(());
        }
        self.expect(">", "grammar.Type", &[">"], self.prev_start())
    }
    fn block(&mut self) -> Result<Block, Box<Diagnostic>> {
        self.block_with_target(false)
    }
    fn block_with_target(&mut self, value_target: bool) -> Result<Block, Box<Diagnostic>> {
        match self.parse_construct(ConstructEntry::Block(value_target))? {
            ConstructResult::Block(block) => Ok(block),
            #[cfg(test)]
            _ => unreachable!("block entry returned another construct"),
        }
    }
    #[cfg(test)]
    fn statement(
        &mut self,
        value_target: bool,
        block_start: usize,
    ) -> Result<Statement, Box<Diagnostic>> {
        let start = self.peek().span.start as usize;
        let previous_start = std::mem::replace(&mut self.statement_start, start);
        let result = self
            .parse_construct(ConstructEntry::Statement {
                value_target,
                block_start,
            })
            .map(|result| match result {
                ConstructResult::Statement(statement) => *statement,
                _ => unreachable!("statement entry returned another construct"),
            });
        self.statement_start = previous_start;
        result
    }
    // nexo:full-profile-dispatch
    fn select_block_entry(&self) -> BlockChoice {
        if !self.new_profile {
            return self.select_block_entry_legacy();
        }
        match production_entry(self.peek(), GrammarState::BlockItem) {
            Some((row, Entry::Active)) => block_choice(row),
            Some((_, Entry::Deferred(_))) => BlockChoice::Deferred,
            _ => BlockChoice::Invalid,
        }
    }
    fn select_block_entry_legacy(&self) -> BlockChoice {
        if self.guard_context > 0
            && matches!(self.peek().text.as_str(), "break" | "continue" | "yield")
        {
            BlockChoice::Guard
        } else if self.word("let") || self.word("var") {
            BlockChoice::Binding
        } else if self.word("return") {
            BlockChoice::Return
        } else if self.word("if") {
            BlockChoice::If
        } else if self.word("while") {
            BlockChoice::While
        } else {
            BlockChoice::Expr
        }
    }
    fn statement_inner(&self) -> Result<BlockChoice, Box<Diagnostic>> {
        let choice = self.select_block_entry();
        if !matches!(choice, BlockChoice::Match | BlockChoice::Guard)
            && let Some(error) = self.deferred_in_state(GrammarState::BlockItem)
        {
            return Err(error);
        }
        Ok(choice)
    }
    fn terminator(
        &mut self,
        st: usize,
        preceding: Span,
        kind: TerminatedStatement,
    ) -> Result<usize, Box<Diagnostic>> {
        if self.eat(";") {
            return Ok(self.prev_end());
        }
        if self.newline_after(preceding.end as usize).is_some() {
            return Ok(preceding.end as usize);
        }
        let p = self.peek().clone();
        Err(self.terminator_error(st, p, preceding, kind))
    }
    fn terminator_error(
        &self,
        st: usize,
        p: Token,
        preceding: Span,
        kind: TerminatedStatement,
    ) -> Box<Diagnostic> {
        let mut d = self.base(
            "NX-E0005",
            Stage::Parse,
            p.span.clone(),
            "Falta un terminador de sentencia.",
        );
        d.unexpected_token = Some(self.diag_token(&p));
        d.expected_tokens = vec![";".into()];
        d.expected_tokens_kind = Some(ExpectedTokensKind::CanonicalInsertion);
        d.parser_context = Some("statement_terminator".into());
        d.recognized_prefix = Some(self.span(st, p.span.start as usize));
        d.related_spans.push(RelatedSpan {
            role: if matches!(
                &self.source[st..preceding.end as usize],
                "return" | "value:"
            ) {
                RelatedRole::Context
            } else {
                RelatedRole::PrecedingExpression
            },
            span: preceding.clone(),
        });
        d.repair.push(Repair {
            id: "insert_terminator".into(),
            status: RepairStatus::Proposed,
            description: "Insertar un terminador explícito.".into(),
            edits: vec![diagnostic::Edit {
                span: self.span(preceding.end as usize, preceding.end as usize),
                expected_text: String::new(),
                replacement: ";".into(),
            }],
            validation: None,
            consequences: vec![
                match (kind, p.text.as_str()) {
                    (TerminatedStatement::Binding, "}") => {
                        "Termina el binding antes del cierre del bloque."
                    }
                    (TerminatedStatement::Binding, _) => {
                        "Termina el binding antes del siguiente token."
                    }
                    (TerminatedStatement::Other, _) => {
                        "Termina la sentencia antes del siguiente token."
                    }
                }
                .into(),
            ],
        });
        d
    }
    // nexo:full-profile-dispatch
    fn select_primary_entry(&self) -> PrimaryChoice {
        if !self.new_profile {
            return self.select_primary_entry_legacy();
        }
        match production_entry(self.peek(), GrammarState::Primary) {
            Some((row, Entry::Active)) => primary_choice(row),
            Some((row, Entry::Deferred(_))) if primary_choice(row) == PrimaryChoice::SelfPath => {
                PrimaryChoice::SelfPath
            }
            Some((_, Entry::Deferred(_))) => PrimaryChoice::Deferred,
            Some((_, Entry::Invalid)) => PrimaryChoice::Invalid,
            _ => PrimaryChoice::Atom,
        }
    }
    fn select_primary_entry_legacy(&self) -> PrimaryChoice {
        if self.word("if") {
            PrimaryChoice::If
        } else if self.word("fn") {
            PrimaryChoice::Fn
        } else if self.word("not") {
            PrimaryChoice::Not
        } else if self.word("self") {
            PrimaryChoice::SelfPath
        } else {
            PrimaryChoice::Atom
        }
    }
    fn prefix(&mut self) -> Result<PrefixValue, Box<Diagnostic>> {
        let choice = self.select_primary_entry();
        let t = self.peek().clone();
        if t.kind == Kind::Decimal {
            return Err(self.feature("float_literal", t.span));
        }
        if t.kind == Kind::String {
            return Err(self.feature("text", t.span));
        }
        if t.text == ".." {
            return Err(self.syntax(&t, "range_endpoint", &["..<", "..="], self.statement_start));
        }
        if matches!(t.text.as_str(), "..<" | "..=") {
            return Err(self.feature("range", t.span));
        }
        if choice == PrimaryChoice::Not {
            return Err(self.syntax(
                &t,
                "unsupported_unary_operator",
                first(self.new_profile, FirstSet::Expr),
                self.statement_start,
            ));
        }
        if choice == PrimaryChoice::SelfPath {
            if self.new_profile {
                self.take();
                return Ok(PrefixValue::Expr(Expr::Name(t.text, t.span)));
            }
            return Err(self.feature("methods", t.span));
        }
        if t.text == "-" {
            let (number, span) = self.signed_integer()?;
            return Ok(PrefixValue::Expr(Expr::Integer(number, span)));
        }
        let prefix_follows = self.tokens.get(self.at + 1).is_some_and(|next| {
            matches!(next.kind, Kind::Name | Kind::Integer)
                || matches!(next.text.as_str(), "(" | "[" | "-" | "true" | "false")
        });
        let mode = if prefix_follows && self.eat("move") {
            Some(Mode::Consume)
        } else if prefix_follows && self.eat("edit") {
            Some(Mode::Inout)
        } else {
            None
        };
        if let Some(m) = mode {
            let op = self.prev_start();
            if !self.starts_postfix() {
                return Err(self.err(
                    "grammar.Postfix",
                    first(self.new_profile, FirstSet::Postfix),
                    op,
                    None,
                ));
            }
            // A selected access mode cannot itself be a Place root. Reject a
            // second selected mode before entering the operand, including hostile
            // chains, while leaving a bare contextual Name available as a root.
            if matches!(self.peek().text.as_str(), "move" | "edit")
                && self.tokens.get(self.at + 1).is_some_and(|next| {
                    matches!(next.kind, Kind::Name | Kind::Integer)
                        || matches!(next.text.as_str(), "(" | "[" | "-" | "true" | "false")
                })
            {
                let token = self.peek().clone();
                return Err(self.syntax(
                    &token,
                    if m == Mode::Consume {
                        "consume_place"
                    } else {
                        "inout_place"
                    },
                    &["Place"],
                    op,
                ));
            }
            // The operand resumes in the same construct machine at precedence 4.
            return Ok(PrefixValue::Mode { mode: m, start: op });
        }
        if let Some(error) = self.deferred_in_state(GrammarState::Primary) {
            return Err(error);
        }
        if t.kind == Kind::Integer {
            self.take();
            return Ok(PrefixValue::Expr(Expr::Integer(t.text, t.span)));
        }
        if t.text == "true" || t.text == "false" {
            self.take();
            return Ok(PrefixValue::Expr(Expr::Bool(t.text == "true", t.span)));
        }
        if t.kind == Kind::Name {
            self.take();
            return Ok(PrefixValue::Expr(Expr::Name(t.text, t.span)));
        }
        Err(self.err(
            "grammar.Primary",
            first(self.new_profile, FirstSet::Expr),
            t.span.start as usize,
            None,
        ))
    }
    fn signed_integer(&mut self) -> Result<(String, Span), Box<Diagnostic>> {
        let sign = self.take();
        debug_assert_eq!(sign.text, "-");
        if self.peek().kind == Kind::Integer {
            let magnitude = self.take();
            return Ok((
                format!("-{}", magnitude.text),
                self.span(sign.span.start as usize, magnitude.span.end as usize),
            ));
        }
        let unexpected = self.peek().clone();
        Err(self.syntax(
            &unexpected,
            "signed_integer",
            &["IntLiteral"],
            sign.span.start as usize,
        ))
    }
    fn starts_pattern(&self) -> bool {
        matches!(self.peek().kind, Kind::Name | Kind::Integer)
            || matches!(self.peek().text.as_str(), "(" | "-" | "true" | "false")
    }
    fn pattern(&mut self) -> Result<Pattern, Box<Diagnostic>> {
        let previous_depth = self.depth;
        let result = self.pattern_iter();
        self.depth = previous_depth;
        result
    }

    fn pattern_iter(&mut self) -> Result<Pattern, Box<Diagnostic>> {
        enum Frame {
            Group {
                start: usize,
            },
            Constructor {
                name: String,
                start: usize,
                args: Vec<Pattern>,
            },
        }
        let mut frames = Vec::new();
        'parse: loop {
            self.depth += 1;
            if self.depth > 128 {
                self.depth -= 1;
                return Err(self.nesting_error("syntax_nesting"));
            }
            let next = self.peek().clone();
            if !self.starts_pattern() {
                return Err(self.syntax(
                    &next,
                    "grammar.Pattern",
                    PATTERN_FIRST,
                    next.span.start as usize,
                ));
            }
            let mut pattern = if next.text == "-" {
                let (number, span) = self.signed_integer()?;
                Pattern::Integer(number, span)
            } else {
                let token = self.take();
                let start = token.span.start as usize;
                if token.text == "(" {
                    frames.push(Frame::Group { start });
                    continue 'parse;
                }
                if token.text == "_" {
                    Pattern::Wildcard(token.span)
                } else if token.kind == Kind::Integer {
                    Pattern::Integer(token.text, token.span)
                } else if token.text == "true" || token.text == "false" {
                    Pattern::Bool(token.text == "true", token.span)
                } else {
                    let (mode, name) = if token.text == "edit" || token.text == "move" {
                        let name = self.expect_name("grammar.Pattern", &["Name"], start)?;
                        (
                            Some(if token.text == "edit" {
                                Mode::Inout
                            } else {
                                Mode::Consume
                            }),
                            name,
                        )
                    } else {
                        (None, token)
                    };
                    debug_assert_eq!(name.kind, Kind::Name);
                    if mode.is_none() && self.eat("(") {
                        if !self.at_text(")") && !self.eof() {
                            frames.push(Frame::Constructor {
                                name: name.text,
                                start,
                                args: Vec::new(),
                            });
                            continue 'parse;
                        }
                        self.expect(")", "grammar.Pattern", &[")", ","], start)?;
                        Pattern::Constructor {
                            name: name.text,
                            args: Vec::new(),
                            span: self.span(start, self.prev_end()),
                        }
                    } else {
                        Pattern::Name {
                            name: name.text,
                            mode,
                            span: self.span(start, self.prev_end()),
                        }
                    }
                }
            };
            loop {
                self.depth -= 1;
                match frames.pop() {
                    Some(Frame::Group { start }) => {
                        if self.eat(",") {
                            return Err(
                                self.feature("tuple", self.tokens[self.at - 1].span.clone())
                            );
                        }
                        self.expect(")", "grammar.Pattern", &["(", ")"], start)?;
                    }
                    Some(Frame::Constructor {
                        name,
                        start,
                        mut args,
                    }) => {
                        args.push(pattern);
                        if self.list_separator(ListKind::ConstructorPattern, start)?
                            && !self.at_text(")")
                            && !self.eof()
                        {
                            frames.push(Frame::Constructor { name, start, args });
                            continue 'parse;
                        }
                        self.expect(")", "grammar.Pattern", &[")", ","], start)?;
                        pattern = Pattern::Constructor {
                            name,
                            args,
                            span: self.span(start, self.prev_end()),
                        };
                    }
                    None => return Ok(pattern),
                }
            }
        }
    }

    fn note_pending(&mut self, feature: &'static str, span: Span) {
        if self
            .pending_match
            .as_ref()
            .is_none_or(|(_, prior)| span.start < prior.start)
        {
            self.pending_match = Some((feature, span));
        }
    }

    fn check_expression_height(&self, expr: &Expr) -> Result<(), Box<Diagnostic>> {
        self.check_candidate_height(expression_height(expr), expr.span().clone())
    }
    fn checked_expression(&self, expr: Expr) -> Result<Expr, Box<Diagnostic>> {
        if let Err(error) = self.check_expression_height(&expr) {
            drop_partial_expression(expr);
            Err(error)
        } else {
            Ok(expr)
        }
    }
    fn check_candidate_height(&self, height: usize, span: Span) -> Result<(), Box<Diagnostic>> {
        if height > 128 {
            return Err(self.budget_error(
                "expression_tree_depth",
                128,
                height as u64,
                span,
                Stage::Parse,
            ));
        }
        Ok(())
    }
    fn expect_name(
        &mut self,
        ctx: &str,
        exp: &[&str],
        st: usize,
    ) -> Result<Token, Box<Diagnostic>> {
        let t = self.peek().clone();
        if t.kind == Kind::Name {
            self.take();
            Ok(t)
        } else {
            Err(self.err(ctx, exp, st, None))
        }
    }
    fn expect(
        &mut self,
        s: &str,
        ctx: &str,
        exp: &[&str],
        st: usize,
    ) -> Result<(), Box<Diagnostic>> {
        if self.eat(s) {
            Ok(())
        } else {
            Err(self.err(ctx, exp, st, None))
        }
    }
    fn err(
        &self,
        ctx: &str,
        expected: &[&str],
        start: usize,
        term: Option<(Token, Span)>,
    ) -> Box<Diagnostic> {
        let t = term
            .as_ref()
            .map(|x| x.0.clone())
            .unwrap_or_else(|| self.peek().clone());
        let prefix_start = if ctx == "module_item" {
            0
        } else if ctx == "parameter_list" && self.eof() {
            self.prev_start()
        } else {
            start
        };
        self.syntax(&t, ctx, expected, prefix_start)
    }
    fn syntax(
        &self,
        t: &Token,
        ctx: &str,
        expected: &[&str],
        prefix_start: usize,
    ) -> Box<Diagnostic> {
        let mut d = self.base(
            "NX-E0002",
            Stage::Parse,
            t.span.clone(),
            "Syntax is not admitted by the grammar.",
        );
        d.unexpected_token = Some(self.diag_token(t));
        d.expected_tokens = expected.iter().map(|x| x.to_string()).collect();
        d.expected_tokens.sort();
        d.expected_tokens.dedup();
        d.expected_tokens_kind = Some(ExpectedTokensKind::AdmissibleSet);
        d.parser_context = Some(ctx.into());
        d.recognized_prefix = Some(self.span(prefix_start, t.span.start as usize));
        d
    }
    fn generic_head_call(&self) -> bool {
        let mut depth = 0usize;
        for (index, token) in self.tokens.iter().enumerate().skip(self.at) {
            match token.text.as_str() {
                "<" => depth += 1,
                ">" => {
                    depth = depth.saturating_sub(1);
                    if depth == 0 {
                        return self
                            .tokens
                            .get(index + 1)
                            .is_some_and(|next| next.text == "(");
                    }
                }
                _ => {}
            }
            if depth > 128 || token.kind == Kind::Eof {
                break;
            }
        }
        false
    }

    // nexo:full-profile-dispatch
    fn deferred_in_state(&self, state: GrammarState) -> Option<Box<Diagnostic>> {
        let token = self.peek();
        if !self.new_profile {
            return legacy_deferred(&token.text)
                .map(|feature| self.feature(feature, token.span.clone()));
        }
        match production_entry(token, state) {
            Some((_, Entry::Deferred(id))) => Some(self.feature(id, token.span.clone())),
            _ => None,
        }
    }

    fn feature(&self, id: &str, s: Span) -> Box<Diagnostic> {
        let mut d = self.base(
            "NX-E0006",
            if matches!(id, "unsupported_type" | "option") {
                Stage::Types
            } else {
                Stage::Parse
            },
            s.clone(),
            "Recognized construct is outside the profile.",
        );
        d.feature_id = Some(id.into());
        if id == "receiver_mode" && self.new_profile {
            d.stage = Stage::Names;
            d.operation = Operation::NamesResolve;
        }
        if id == "methods" && self.new_profile && self.slice(&s) == "." {
            d.stage = Stage::Names;
            d.operation = Operation::NamesResolve;
        }
        if matches!(id, "unsupported_type" | "option") {
            d.operation = Operation::TypesCheck;
        }
        d.data.insert(
            "pending_scope".into(),
            DataValue::String(
                if matches!(id, "closure" | "generator" | "option" | "match") {
                    "F0-0003"
                } else {
                    "later_adr"
                }
                .into(),
            ),
        );
        if id == "unsupported_type" {
            d.data.insert(
                "type_spelling".into(),
                DataValue::String(self.slice(&s).into()),
            );
        }
        d
    }
    fn function_value(&self, expr: &Expr) -> Option<Span> {
        let mut pending = vec![expr];
        while let Some(expr) = pending.pop() {
            match expr {
                Expr::Name(name, span)
                    if self.function_names.contains(name)
                        && !self
                            .local_scopes
                            .iter()
                            .rev()
                            .any(|scope| scope.contains(name)) =>
                {
                    return Some(span.clone());
                }
                Expr::Group(inner, _) | Expr::Unary { value: inner, .. } => pending.push(inner),
                Expr::Binary { left, right, .. }
                | Expr::Index {
                    target: left,
                    index: right,
                    ..
                } => {
                    pending.push(right);
                    pending.push(left);
                }
                Expr::Call { args, .. } => {
                    pending.extend(args.iter().rev().map(|arg| &arg.value));
                }
                Expr::Array(xs, _) => pending.extend(xs.iter().rev()),
                _ => {}
            }
        }
        None
    }
    fn reject_function_value(&self, expr: &Expr) -> Result<(), Box<Diagnostic>> {
        if let Some(span) = self.function_value(expr) {
            let mut d = self.feature("function_value", span);
            d.stage = Stage::Names;
            d.operation = Operation::NamesResolve;
            return Err(d);
        }
        Ok(())
    }
    fn nesting_error(&self, kind: &str) -> Box<Diagnostic> {
        self.budget_error(kind, 128, 129, self.peek().span.clone(), Stage::Parse)
    }
    fn budget_error(
        &self,
        kind: &str,
        limit: u64,
        observed: u64,
        span: Span,
        stage: Stage,
    ) -> Box<Diagnostic> {
        let mut d = self.base("NX-E0003", stage, span, "Analysis budget exceeded.");
        d.operation = Operation::AnalysisLimit;
        d.data
            .insert("budget_kind".into(), DataValue::String(kind.into()));
        d.data.insert(
            "budget_scope".into(),
            DataValue::String("compilation".into()),
        );
        d.data.insert(
            "incomplete_analysis".into(),
            DataValue::String(stage.as_str().into()),
        );
        d.data.insert("limit".into(), DataValue::UInt(limit));
        d.data.insert("observed".into(), DataValue::UInt(observed));
        d
    }
    fn base(&self, code: &str, stage: Stage, s: Span, msg: &str) -> Box<Diagnostic> {
        Box::new(Diagnostic {
            schema_version: diagnostic::SCHEMA_VERSION,
            diagnostic_namespace: Namespace::NexoProposedV1,
            profile_id: diagnostic::PROFILE_ID.into(),
            code: code.into(),
            subcause: None,
            rule: code.into(),
            explanation_id: Some(code.into()),
            severity: Severity::Error,
            stage,
            message: msg.into(),
            operation: Operation::SyntaxParse,
            primary_span: Some(s),
            related_spans: Vec::new(),
            cause_chain: Vec::new(),
            feature_id: None,
            unexpected_token: None,
            expected_tokens: Vec::new(),
            expected_tokens_kind: None,
            parser_context: None,
            recognized_prefix: None,
            repair: Vec::new(),
            data: BTreeMap::new(),
        })
    }
    fn branch_error(&self, subcause: &str, span: Span) -> Box<Diagnostic> {
        let mut d = self.base(
            "NX-E1302",
            Stage::Parse,
            span,
            "Invalid branch value syntax.",
        );
        d.subcause = Some(subcause.into());
        d.rule = format!("NX-E1302/{subcause}");
        d.explanation_id = Some(d.rule.clone());
        d
    }
    fn eof(&self) -> bool {
        self.peek().kind == Kind::Eof
    }
    fn starts_parameter(&self) -> bool {
        self.peek().kind == Kind::Name
    }
    fn starts_restriction(&self) -> bool {
        is_restriction(&self.peek().text)
    }
    // nexo:full-profile-dispatch
    fn starts_expr(&self) -> bool {
        if !self.new_profile {
            return self.starts_expr_legacy();
        }
        self.starts_atom()
            || matches!(
                production_entry(self.peek(), GrammarState::Primary).map(|(_, entry)| entry),
                Some(Entry::Active | Entry::Deferred(_))
            )
    }
    fn starts_expr_legacy(&self) -> bool {
        matches!(
            self.peek().kind,
            Kind::Name | Kind::Integer | Kind::Decimal | Kind::String
        ) || matches!(
            self.peek().text.as_str(),
            "(" | "[" | "-" | "true" | "false" | "if" | "fn" | "not"
        ) || legacy_deferred(&self.peek().text).is_some()
    }
    // nexo:full-profile-dispatch
    fn starts_try(&self) -> bool {
        self.starts_expr() && !production_try_excluded(self.peek())
    }
    // nexo:full-profile-dispatch
    fn starts_postfix(&self) -> bool {
        if !self.new_profile {
            return self.starts_postfix_legacy();
        }
        self.starts_atom()
            || matches!(
                production_entry(self.peek(), GrammarState::Primary).map(|(_, entry)| entry),
                Some(Entry::Deferred(_))
            )
    }
    fn starts_postfix_legacy(&self) -> bool {
        matches!(
            self.peek().kind,
            Kind::Name | Kind::Integer | Kind::Decimal | Kind::String
        ) || matches!(
            self.peek().text.as_str(),
            "(" | "[" | "-" | "true" | "false"
        ) || legacy_deferred(&self.peek().text).is_some()
    }
    // nexo:full-profile-dispatch
    fn starts_statement(&self) -> bool {
        if !self.new_profile {
            return self.starts_statement_legacy();
        }
        self.starts_atom()
            || matches!(
                production_entry(self.peek(), GrammarState::BlockItem).map(|(_, entry)| entry),
                Some(Entry::Active | Entry::Deferred(_))
            )
            || self.starts_block_open()
    }
    fn starts_statement_legacy(&self) -> bool {
        self.starts_expr()
            || matches!(
                self.peek().text.as_str(),
                "let" | "var" | "return" | "while" | "{"
            )
    }
    // nexo:full-profile-dispatch
    fn starts_atom(&self) -> bool {
        matches!(
            self.peek().kind,
            Kind::Name | Kind::Integer | Kind::Decimal | Kind::String
        ) || matches!(
            self.peek().text.as_str(),
            "(" | "[" | "-" | "true" | "false"
        )
    }
    // nexo:full-profile-dispatch
    fn starts_block_open(&self) -> bool {
        self.peek().text == "{"
    }
    fn word(&self, s: &str) -> bool {
        self.peek().text == s
    }
    fn at_text(&self, s: &str) -> bool {
        self.peek().text == s
    }
    fn eat(&mut self, s: &str) -> bool {
        if self.at_text(s) {
            self.take();
            true
        } else {
            false
        }
    }
    fn take(&mut self) -> Token {
        let t = self.peek().clone();
        if !self.eof() {
            self.split_type_close = None;
            self.at += 1;
        }
        t
    }
    fn newline_after(&self, end: usize) -> Option<usize> {
        let next = self.peek().span.start as usize;
        (end <= next)
            .then(|| self.source[end..next].find(['\r', '\n']).map(|n| end + n))
            .flatten()
    }
    fn line_break_before_next(&self) -> bool {
        self.newline_after(self.prev_end()).is_some()
    }
    fn peek(&self) -> &Token {
        if let Some(work) = &self.work {
            work.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        }
        &self.tokens[self.at.min(self.tokens.len() - 1)]
    }
    fn prev_span(&self) -> Span {
        if let Some(span) = &self.split_type_close {
            return span.clone();
        }
        self.tokens[self.at.saturating_sub(1)].span.clone()
    }
    fn prev_start(&self) -> usize {
        self.prev_span().start as usize
    }
    fn prev_end(&self) -> usize {
        self.prev_span().end as usize
    }
    fn span(&self, a: usize, b: usize) -> Span {
        Span {
            file: self.file.into(),
            start: a as u64,
            end: b as u64,
        }
    }
    fn slice(&self, s: &Span) -> &str {
        &self.source[s.start as usize..s.end as usize]
    }
    fn diag_token(&self, t: &Token) -> DToken {
        DToken {
            kind: match t.kind {
                Kind::Name => TokenKind::Name,
                Kind::Keyword => TokenKind::Keyword,
                Kind::Integer => TokenKind::Integer,
                Kind::Decimal => TokenKind::Decimal,
                Kind::String => TokenKind::String,
                Kind::Symbol => TokenKind::Symbol,
                Kind::Newline => TokenKind::Symbol,
                Kind::Eof => TokenKind::Eof,
            },
            text: Some(t.text.clone()),
            span: t.span.clone(),
        }
    }
}
fn place_root_failure(expr: &Expr) -> Option<u64> {
    let mut current = expr;
    let mut fallback = None;
    loop {
        match current {
            Expr::Name(..) => return fallback,
            Expr::Group(inner, _) => current = inner,
            Expr::Index { target, .. } => current = target,
            Expr::Binary { left, right, .. } => {
                fallback = Some(left.span().end.min(right.span().start));
                current = left;
            }
            Expr::Unary {
                mode: Mode::Read,
                value,
                span,
            } => {
                fallback = Some(span.end - 1);
                current = value;
            }
            other => return Some(other.span().start),
        }
    }
}
pub(crate) fn place_parts(expr: &Expr) -> Option<(&Expr, Vec<&Expr>)> {
    let mut node = expr;
    let mut indexes = Vec::new();
    loop {
        match node {
            Expr::Group(inner, _) => node = inner,
            Expr::Index { target, index, .. } => {
                indexes.push(index.as_ref());
                node = target;
            }
            Expr::Name(..) => {
                indexes.reverse();
                return Some((node, indexes));
            }
            _ => return None,
        }
    }
}

fn is_place_expr(expr: &Expr) -> bool {
    place_parts(expr).is_some()
}
fn has_call_root(expr: &Expr) -> bool {
    let mut current = expr;
    loop {
        match current {
            Expr::Call { .. } => return true,
            Expr::Group(inner, _) | Expr::Index { target: inner, .. } => current = inner,
            _ => return false,
        }
    }
}
fn is_projected_place(expr: &Expr) -> bool {
    let mut current = expr;
    while let Expr::Group(inner, _) = current {
        current = inner;
    }
    matches!(current, Expr::Index { .. })
}
fn is_resource_constructor(expr: &Expr) -> bool {
    matches!(expr, Expr::Call { target, .. } if matches!(&**target, Expr::Name(name, _) if name == "MockOpen" || name == "MockBlockingOpen"))
}

fn expression_height(expr: &Expr) -> usize {
    height_walk(HeightNode::Expr(expr))
}

// A rejected expression may already own a deep chain of blocks, matches and
// else branches. Dropping that partial AST through Rust's derived destructors
// would consume the parser worker's stack after the diagnostic is ready.
fn drop_partial_expression(expr: Expr) {
    enum Node {
        Expr(Expr),
        Statement(Statement),
        Block(Block),
        Else(Else),
        Match(Match),
        Arm(Arm),
        Pattern(Pattern),
        Type(Type),
    }
    let mut pending = vec![Node::Expr(expr)];
    while let Some(node) = pending.pop() {
        match node {
            Node::Expr(expr) => match expr {
                Expr::Closure(closure) => {
                    let closure = *closure;
                    pending.push(Node::Block(closure.body));
                    pending.extend(closure.result.into_iter().map(Node::Type));
                    pending.extend(
                        closure
                            .params
                            .into_iter()
                            .filter_map(|p| p.ty)
                            .map(Node::Type),
                    );
                }
                Expr::Name(..) | Expr::Integer(..) | Expr::Bool(..) | Expr::Unit(..) => {}
                Expr::Array(elements, _) => pending.extend(elements.into_iter().map(Node::Expr)),
                Expr::Group(inner, _) | Expr::Unary { value: inner, .. } => {
                    pending.push(Node::Expr(*inner));
                }
                Expr::Binary { left, right, .. } => {
                    pending.push(Node::Expr(*left));
                    pending.push(Node::Expr(*right));
                }
                Expr::Call { target, args, .. } => {
                    pending.push(Node::Expr(*target));
                    pending.extend(args.into_iter().map(|arg| Node::Expr(arg.value)));
                }
                Expr::Index { target, index, .. } => {
                    pending.push(Node::Expr(*target));
                    pending.push(Node::Expr(*index));
                }
                Expr::If {
                    condition,
                    then_block,
                    otherwise,
                    ..
                } => {
                    pending.push(Node::Expr(*condition));
                    pending.push(Node::Block(then_block));
                    pending.extend(otherwise.map(|branch| Node::Else(*branch)));
                }
                Expr::Match(selection) => pending.push(Node::Match(*selection)),
            },
            Node::Statement(statement) => match statement {
                Statement::Let {
                    annotation, value, ..
                } => {
                    pending.push(Node::Expr(value));
                    pending.extend(annotation.map(Node::Type));
                }
                Statement::Assign { place, value, .. } => {
                    pending.push(Node::Expr(place));
                    pending.push(Node::Expr(value));
                }
                Statement::Return { value, .. }
                | Statement::GuardControl { value, .. }
                | Statement::Value { value, .. } => {
                    pending.extend(value.map(Node::Expr));
                }
                Statement::If {
                    condition,
                    then_block,
                    otherwise,
                    ..
                } => {
                    pending.push(Node::Expr(condition));
                    pending.push(Node::Block(then_block));
                    pending.extend(otherwise.map(|branch| Node::Statement(*branch)));
                }
                Statement::While {
                    condition, body, ..
                } => {
                    pending.push(Node::Expr(condition));
                    pending.push(Node::Block(body));
                }
                Statement::For { iterable, body, .. } => {
                    pending.push(Node::Expr(iterable));
                    pending.push(Node::Block(body));
                }
                Statement::Match(selection) => pending.push(Node::Match(selection)),
                Statement::Block(block) => pending.push(Node::Block(block)),
                Statement::Expr { value, .. } => pending.push(Node::Expr(value)),
            },
            Node::Block(block) => {
                pending.extend(block.statements.into_iter().map(Node::Statement));
            }
            Node::Else(branch) => match branch {
                Else::Block(block) => pending.push(Node::Block(block)),
                Else::If(expr) => pending.push(Node::Expr(*expr)),
            },
            Node::Match(selection) => {
                pending.push(Node::Expr(*selection.scrutinee));
                pending.extend(selection.arms.into_iter().map(Node::Arm));
            }
            Node::Arm(arm) => {
                pending.push(Node::Pattern(arm.pattern));
                pending.extend(arm.guard.map(Node::Expr));
                pending.push(Node::Block(arm.body));
            }
            Node::Pattern(pattern) => {
                if let Pattern::Constructor { args, .. } = pattern {
                    pending.extend(args.into_iter().map(Node::Pattern));
                }
            }
            Node::Type(ty) => match ty {
                Type::Array(inner) | Type::Option(inner) | Type::Generator { item: inner, .. } => {
                    pending.push(Node::Type(*inner));
                }
                Type::Result(ok, err) => {
                    pending.push(Node::Type(*ok));
                    pending.push(Node::Type(*err));
                }
                _ => {}
            },
        }
    }
}

#[derive(Clone, Copy)]
enum HeightNode<'a> {
    Expr(&'a Expr),
    Statement(&'a Statement),
    Block(&'a Block),
    Else(&'a Else),
    Match(&'a Match),
    Arm(&'a Arm),
}

fn height_walk(root: HeightNode<'_>) -> usize {
    enum Work<'a> {
        Visit(HeightNode<'a>),
        Finish { children: usize, increment: usize },
    }
    let mut work = vec![Work::Visit(root)];
    let mut heights = Vec::new();
    while let Some(step) = work.pop() {
        match step {
            Work::Finish {
                children,
                increment,
            } => {
                let first = heights.len() - children;
                let maximum = heights.drain(first..).max().unwrap_or(0);
                heights.push(increment + maximum);
            }
            Work::Visit(node) => {
                let mut children = Vec::new();
                let increment = match node {
                    HeightNode::Expr(expr) => match expr {
                        Expr::Closure(..)
                        | Expr::Name(..)
                        | Expr::Integer(..)
                        | Expr::Bool(..)
                        | Expr::Unit(..) => 1,
                        Expr::Group(inner, _) => {
                            children.push(HeightNode::Expr(inner));
                            0
                        }
                        Expr::Array(elements, _) => {
                            children.extend(elements.iter().map(HeightNode::Expr));
                            1
                        }
                        Expr::Unary { value, .. } => {
                            children.push(HeightNode::Expr(value));
                            1
                        }
                        Expr::Binary { left, right, .. } => {
                            children.push(HeightNode::Expr(left));
                            children.push(HeightNode::Expr(right));
                            1
                        }
                        Expr::Call { target, args, .. } => {
                            children.push(HeightNode::Expr(target));
                            children.extend(args.iter().map(|arg| HeightNode::Expr(&arg.value)));
                            1
                        }
                        Expr::Index { target, index, .. } => {
                            children.push(HeightNode::Expr(target));
                            children.push(HeightNode::Expr(index));
                            1
                        }
                        Expr::If {
                            condition,
                            then_block,
                            otherwise,
                            ..
                        } => {
                            children.push(HeightNode::Expr(condition));
                            children.push(HeightNode::Block(then_block));
                            if let Some(otherwise) = otherwise {
                                children.push(HeightNode::Else(otherwise));
                            }
                            1
                        }
                        Expr::Match(selection) => {
                            children.push(HeightNode::Match(selection));
                            0
                        }
                    },
                    HeightNode::Statement(statement) => {
                        match statement {
                            Statement::Let { value, .. }
                            | Statement::Assign { value, .. }
                            | Statement::Expr { value, .. } => {
                                children.push(HeightNode::Expr(value));
                            }
                            Statement::Return { value, .. }
                            | Statement::GuardControl { value, .. }
                            | Statement::Value { value, .. } => {
                                if let Some(value) = value {
                                    children.push(HeightNode::Expr(value));
                                }
                            }
                            Statement::If {
                                condition,
                                then_block,
                                otherwise,
                                ..
                            } => {
                                children.push(HeightNode::Expr(condition));
                                children.push(HeightNode::Block(then_block));
                                if let Some(otherwise) = otherwise {
                                    children.push(HeightNode::Statement(otherwise));
                                }
                            }
                            Statement::While {
                                condition, body, ..
                            } => {
                                children.push(HeightNode::Expr(condition));
                                children.push(HeightNode::Block(body));
                            }
                            Statement::For { iterable, body, .. } => {
                                children.push(HeightNode::Expr(iterable));
                                children.push(HeightNode::Block(body));
                            }
                            Statement::Match(selection) => {
                                children.push(HeightNode::Match(selection));
                            }
                            Statement::Block(block) => children.push(HeightNode::Block(block)),
                        }
                        0
                    }
                    HeightNode::Block(block) => {
                        children.extend(block.statements.iter().map(HeightNode::Statement));
                        0
                    }
                    HeightNode::Else(otherwise) => {
                        match otherwise {
                            Else::Block(block) => children.push(HeightNode::Block(block)),
                            Else::If(expr) => children.push(HeightNode::Expr(expr)),
                        }
                        0
                    }
                    HeightNode::Match(selection) => {
                        children.push(HeightNode::Expr(&selection.scrutinee));
                        children.extend(selection.arms.iter().map(HeightNode::Arm));
                        1
                    }
                    HeightNode::Arm(arm) => {
                        if let Some(guard) = &arm.guard {
                            children.push(HeightNode::Expr(guard));
                        }
                        children.push(HeightNode::Block(&arm.body));
                        0
                    }
                };
                work.push(Work::Finish {
                    children: children.len(),
                    increment,
                });
                work.extend(children.into_iter().rev().map(Work::Visit));
            }
        }
    }
    heights.pop().unwrap_or(0)
}

#[cfg(test)]
mod b_tests {
    use super::*;
    use crate::frontend::print::print_canonical;

    #[test]
    fn type_close_ge_splits_only_inside_a_type() {
        let ast = parse("split.n", "fn Main(){let r:Result<I64,I64>=Ok(1);}").unwrap();
        assert_eq!(ast.functions[0].body.statements.len(), 1);
        let source = "Result<I64,I64>=Ok(1)";
        let tokens = lex::lex("split.n", source).unwrap();
        let at = tokens.iter().position(|token| token.text == ">=").unwrap();
        let start = tokens[at].span.start;
        let mut parser = Parser {
            file: "split.n",
            source,
            tokens,
            at,
            statement_start: 0,
            split_type_close: None,
            split_type_closes: Vec::new(),
            depth: 0,
            function_names: HashSet::new(),
            local_scopes: Vec::new(),
            work: None,
            new_profile: false,
            f5_operators: false,
            pending_match: None,
            guard_context: 0,
            callable_count: 0,
        };
        parser.expect_type_close().unwrap();
        assert_eq!(
            (parser.prev_span().start, parser.prev_span().end),
            (start, start + 1)
        );
        assert_eq!(parser.peek().text, "=");
        assert_eq!(
            (parser.peek().span.start, parser.peek().span.end),
            (start + 1, start + 2)
        );
        parser.take();
        assert_eq!(
            (parser.prev_span().start, parser.prev_span().end),
            (start + 1, start + 2)
        );
    }

    #[test]
    fn parses_shared_ast_and_prints_official_spelling() {
        let source = "fn F(x:view I64,y:edit Array<I64>)->I64{return if true then 1 else 2;} fn Main(){let r=MockOpen(1);let s=move r;}";
        let ast = parse("b.n", source).unwrap();
        assert_eq!(ast.functions[0].params[0].mode, Mode::Read);
        assert_eq!(ast.functions[0].params[1].mode, Mode::Inout);
        assert!(matches!(
            ast.functions[0].body.statements[0],
            Statement::Return {
                value: Some(Expr::If { .. }),
                ..
            }
        ));
        assert!(matches!(
            ast.functions[1].body.statements[1],
            Statement::Let {
                value: Expr::Unary {
                    mode: Mode::Consume,
                    ..
                },
                ..
            }
        ));
        let printed = print_canonical(&ast).unwrap();
        assert!(printed.contains("x: view I64, y: edit Array<I64>"));
        assert!(printed.contains("if true then 1 else 2"));
        assert!(printed.ends_with("\n"));
        let reparsed = parse("b.n", &printed).unwrap();
        assert_eq!(print_canonical(&reparsed).unwrap(), printed);
    }

    #[test]
    fn b_scope_and_syntax_diagnostics_validate() {
        for (source, code, feature, span) in [
            (
                "fn Main(){let s=\"…{x}\";}",
                "NX-E0006",
                Some("text"),
                (16, 24),
            ),
            ("fn Main(){let s=\"\\q\";}", "NX-E0001", None, (17, 19)),
            (
                "fn Main(){let x=0.5;}",
                "NX-E0006",
                Some("float_literal"),
                (16, 19),
            ),
            (
                "fn Main(){let n=1;let x=0..<n;}",
                "NX-E0006",
                Some("range"),
                (25, 28),
            ),
        ] {
            let error = parse("b.n", source).unwrap_err();
            assert_eq!(error.code, code, "{source}");
            assert_eq!(error.feature_id.as_deref(), feature, "{source}");
            let primary = error.primary_span.as_ref().unwrap();
            assert_eq!((primary.start, primary.end), span, "{source}");
            assert_eq!(error.profile_id, diagnostic::PROFILE_ID);
            assert!(
                error.validate("b.n", source).is_ok(),
                "{source}: {:?}",
                error.validate("b.n", source)
            );
        }
    }

    #[test]
    fn b_statement_start_scope_restores_on_success_and_error() {
        for (source, expect_error) in [
            ("  let x=1;", false),
            ("  let x=1;else", false),
            ("  let x=1*2;", true),
            (
                "  let x=(if c then {let a=1;value:2;}else{value:3;})*4;",
                true,
            ),
        ] {
            let tokens = lex::lex("scope.n", source)
                .unwrap()
                .into_iter()
                .filter(|t| t.kind != Kind::Newline)
                .collect();
            let mut parser = Parser {
                file: "scope.n",
                source,
                tokens,
                at: 0,
                statement_start: 97,
                split_type_close: None,
                split_type_closes: Vec::new(),
                depth: 0,
                function_names: HashSet::new(),
                local_scopes: Vec::new(),
                work: None,
                new_profile: false,
                f5_operators: false,
                pending_match: None,
                guard_context: 0,
                callable_count: 0,
            };
            let result = parser.statement(false, 0);
            assert_eq!(result.is_err(), expect_error, "{source}");
            if expect_error {
                let d = result.unwrap_err();
                assert_eq!(d.code, "NX-E0002", "{source}");
                assert_eq!(
                    d.recognized_prefix.as_ref().map(|s| s.start),
                    Some(2),
                    "{source}"
                );
            }
            assert_eq!(parser.statement_start, 97, "{source}");
        }
    }

    #[test]
    fn hidden_f5_binary_parser_preserves_public_j_and_rejects_chains() {
        for source in [
            "fn Main(){let x=1*2+3/4%5;}",
            "fn Main(){let x=1!=2;}",
            "fn Main(){let x=1>=2;}",
            "fn Main(){let x=(1<2)==true;}",
        ] {
            assert!(parse_f5_hidden("case.n", source).is_ok(), "{source}");
        }
        for source in [
            "fn Main(){let x=1*2+3;}",
            "fn Main(){let x=1+2*3;}",
            "fn Main(){let x=(1<2)==true;}",
        ] {
            let ast = parse_f5_hidden("case.n", source).unwrap();
            let printed = crate::frontend::print::print_canonical(&ast).unwrap();
            assert!(parse_f5_hidden("case.n", &printed).is_ok(), "{printed}");
        }
        assert!(parse_new_profile("case.n", "fn Main(){let x=1*2;}").is_err());
        let source = "fn Main(){let x=1<2<3;}";
        let error = parse_f5_hidden("case.n", source).unwrap_err();
        assert_eq!(error.code, "NX-E0002");
        assert_eq!(error.parser_context.as_deref(), Some("grammar.CompareExpr"));
        assert_eq!(error.expected_tokens, [";", "and", "or"]);
        assert_eq!(
            error.primary_span.as_ref().map(|s| (s.start, s.end)),
            Some((19, 20))
        );
    }
}
