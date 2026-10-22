//! Parser and normalized syntax tree for the accepted part of f0-0002-v1.
use std::cell::RefCell;
use std::collections::{BTreeMap, HashSet};
use std::sync::mpsc::{self, Sender};

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
    "consume",
    "false",
    "if",
    "inout",
    "true",
];
const TRY_START: &[&str] = &[
    "(",
    "-",
    "IntLiteral",
    "Name",
    "[",
    "consume",
    "false",
    "inout",
    "true",
];
const BLOCK_NEXT: &[&str] = &[
    "(",
    "-",
    "IntLiteral",
    "Name",
    "[",
    "consume",
    "false",
    "if",
    "inout",
    "let",
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
    "MockBlockingResource",
    "MockResource",
    "Result",
    "bool",
    "i64",
];
const POSTFIX_START: &[&str] = &["(", "-", "IntLiteral", "Name", "[", "false", "true"];
// FIRST sets transcribed from §3 and §13.14. Callers select a committed
// production state before looking at the erroneous token (including EOF).
const PARAMETER_LIST_FIRST: &[&str] = &[")", "Name", "consume", "inout"];
const PARAMETER_FIRST: &[&str] = &["Name", "consume", "inout"];
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
    "consume",
    "false",
    "if",
    "inout",
    "true",
];
const ARRAY_FIRST: &[&str] = &[
    "(",
    "-",
    "IntLiteral",
    "Name",
    "[",
    "]",
    "consume",
    "false",
    "if",
    "inout",
    "true",
];
const ELSE_FIRST: &[&str] = &["if", "{"];

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Ast {
    pub functions: Vec<Function>,
    pub span: Span,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Function {
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
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum WithClause {
    Simple(Vec<String>),
    Operations(Vec<OperationLimit>),
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OperationLimit {
    pub name: String,
    pub restrictions: Vec<String>,
    pub span: Span,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Block {
    pub statements: Vec<Statement>,
    pub span: Span,
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
    Value {
        value: Option<Expr>,
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
    Block(Block),
    Expr {
        value: Expr,
        span: Span,
    },
}
impl Statement {
    fn span(&self) -> &Span {
        match self {
            Self::Let { span, .. }
            | Self::Assign { span, .. }
            | Self::Return { span, .. }
            | Self::Value { span, .. }
            | Self::If { span, .. }
            | Self::While { span, .. }
            | Self::Expr { span, .. } => span,
            Self::Block(block) => &block.span,
        }
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Expr {
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
        span: Span,
    },
    If {
        condition: Box<Expr>,
        then_block: Block,
        otherwise: Option<Box<Else>>,
        span: Span,
    },
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Else {
    Block(Block),
    If(Box<Expr>),
}
impl Else {
    fn span(&self) -> &Span {
        match self {
            Self::Block(block) => &block.span,
            Self::If(expr) => expr.span(),
        }
    }
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
        }
    }
}

type ParseResult = Result<Ast, Box<Diagnostic>>;
type WorkerResult = Result<ParseResult, Box<dyn std::any::Any + Send>>;
struct ParseJob {
    file: String,
    source: String,
    response: Sender<WorkerResult>,
}
thread_local! {
    static PARSE_WORKER: RefCell<Option<Sender<ParseJob>>> = const { RefCell::new(None) };
}
#[cfg(test)]
thread_local! {
    static PARSE_WORKER_SPAWNS: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

fn worker_unavailable(file: &str, source: &str) -> Box<Diagnostic> {
    // A failed worker allocation cannot safely recurse on the caller's unknown stack.
    let parser = Parser {
        file,
        source,
        tokens: Vec::new(),
        at: 0,
        statement_start: 0,
        split_type_close: None,
        depth: 0,
        function_names: HashSet::new(),
        local_scopes: Vec::new(),
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
    if source.len() > 1_048_576 {
        let parser = Parser {
            file,
            source,
            tokens: Vec::new(),
            at: 0,
            statement_start: 0,
            split_type_close: None,
            depth: 0,
            function_names: HashSet::new(),
            local_scopes: Vec::new(),
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
    };
    let sent = PARSE_WORKER.with(|slot| {
        let mut worker = slot.borrow_mut();
        if worker.is_none() {
            #[cfg(test)]
            PARSE_WORKER_SPAWNS.with(|count| count.set(count.get() + 1));
            let (sender, jobs) = mpsc::channel::<ParseJob>();
            std::thread::Builder::new()
                .stack_size(16 * 1024 * 1024)
                .spawn(move || {
                    for job in jobs {
                        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                            parse_inner(&job.file, &job.source)
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

fn parse_inner(file: &str, source: &str) -> Result<Ast, Box<Diagnostic>> {
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
        depth: 0,
        function_names,
        local_scopes: Vec::new(),
    }
    .program()
}

fn deferred_feature(word: &str) -> Option<&'static str> {
    match word {
        "move" => Some("legacy_move"),
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
fn returns_unit(expr: &Expr) -> bool {
    match expr {
        Expr::Unit(_) => true,
        Expr::Group(inner, _) => returns_unit(inner),
        _ => false,
    }
}
fn non_unit_return(statement: &Statement) -> Option<Span> {
    match statement {
        Statement::Return {
            value: Some(v),
            span,
        } if !returns_unit(v) => Some(Span {
            file: span.file.clone(),
            start: span.start,
            end: v.span().end,
        }),
        Statement::Block(b) => b.statements.iter().find_map(non_unit_return),
        Statement::If {
            condition,
            then_block,
            otherwise,
            ..
        } => expr_non_unit_return(condition).or_else(|| {
            then_block
                .statements
                .iter()
                .find_map(non_unit_return)
                .or_else(|| otherwise.as_deref().and_then(non_unit_return))
        }),
        Statement::While {
            condition, body, ..
        } => expr_non_unit_return(condition)
            .or_else(|| body.statements.iter().find_map(non_unit_return)),
        Statement::Let { value, .. }
        | Statement::Assign { value, .. }
        | Statement::Expr { value, .. }
        | Statement::Value {
            value: Some(value), ..
        }
        | Statement::Return {
            value: Some(value), ..
        } => expr_non_unit_return(value),
        _ => None,
    }
}
fn expr_non_unit_return(expr: &Expr) -> Option<Span> {
    match expr {
        Expr::If {
            condition,
            then_block,
            otherwise,
            ..
        } => expr_non_unit_return(condition)
            .or_else(|| then_block.statements.iter().find_map(non_unit_return))
            .or_else(|| match otherwise.as_deref() {
                Some(Else::Block(b)) => b.statements.iter().find_map(non_unit_return),
                Some(Else::If(e)) => expr_non_unit_return(e),
                None => None,
            }),
        Expr::Group(e, _) | Expr::Unary { value: e, .. } => expr_non_unit_return(e),
        Expr::Binary { left, right, .. } => {
            expr_non_unit_return(left).or_else(|| expr_non_unit_return(right))
        }
        Expr::Call { target, args, .. } => expr_non_unit_return(target)
            .or_else(|| args.iter().find_map(|arg| expr_non_unit_return(&arg.value))),
        Expr::Index { target, index, .. } => {
            expr_non_unit_return(target).or_else(|| expr_non_unit_return(index))
        }
        Expr::Array(xs, _) => xs.iter().find_map(expr_non_unit_return),
        _ => None,
    }
}
struct Parser<'a> {
    file: &'a str,
    source: &'a str,
    tokens: Vec<Token>,
    at: usize,
    statement_start: usize,
    split_type_close: Option<Span>,
    depth: usize,
    function_names: HashSet<String>,
    local_scopes: Vec<HashSet<String>>,
}
impl Parser<'_> {
    fn program(mut self) -> Result<Ast, Box<Diagnostic>> {
        let start = 0;
        let mut fs = Vec::new();
        while !self.eof() {
            if self.word("fn") {
                if fs.len() == 1_024
                    && let Some(name) = self.tokens.get(self.at + 1)
                    && name.kind == Kind::Name
                {
                    return Err(self.budget_error(
                        "function_count",
                        1_024,
                        1_025,
                        name.span.clone(),
                        Stage::Parse,
                    ));
                }
                fs.push(self.function()?)
            } else {
                if let Some(id) = deferred_feature(&self.peek().text) {
                    return Err(self.feature(id, self.peek().span.clone()));
                }
                return Err(self.err("module_item", &["EOF", "fn"], 0, None));
            }
        }
        Ok(Ast {
            functions: fs,
            span: self.span(start, self.source.len()),
        })
    }
    fn function(&mut self) -> Result<Function, Box<Diagnostic>> {
        let start = self.take().span.start as usize;
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
            let mode = if self.eat("inout") {
                Mode::Inout
            } else if self.eat("consume") {
                Mode::Consume
            } else {
                Mode::Read
            };
            let n = self.expect_name(
                "grammar.Parameter",
                if mode == Mode::Read {
                    PARAMETER_FIRST
                } else {
                    &["Name"]
                },
                ps,
            )?;
            self.expect(":", "grammar.Parameter", &[":"], ps)?;
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
            if !self.eat(",") {
                break;
            }
            after_comma = true;
            if self.eof() {
                return Err(self.err(
                    "grammar.Parameter",
                    PARAMETER_FIRST,
                    self.peek().span.start as usize,
                    None,
                ));
            }
            if self.at_text(")") {
                return Err(self.err(
                    "grammar.Parameter",
                    PARAMETER_FIRST,
                    self.peek().span.start as usize,
                    None,
                ));
            }
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
            Some(self.with_clause(&name)?)
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
        if result.is_none()
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
            name: name.text,
            name_span: ns,
            params,
            result,
            with_clause,
            body,
            span: self.span(start, end),
        })
    }
    fn with_clause(&mut self, function_name: &Token) -> Result<WithClause, Box<Diagnostic>> {
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
                if name.text != "call" && invalid.is_none() {
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
            if !operations.iter().any(|op| op.name == "call") {
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
    fn restrictions(&mut self) -> Result<Vec<String>, Box<Diagnostic>> {
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
        Ok(items.into_iter().map(|x| x.text).collect())
    }
    fn with_error(&self, subcause: &str, span: Span) -> Box<Diagnostic> {
        let mut d = self.base("NX-E0417", Stage::Parse, span, "Invalid with clause.");
        d.subcause = Some(subcause.into());
        d.rule = format!("NX-E0417/{subcause}");
        d.explanation_id = Some(d.rule.clone());
        d
    }
    fn ty(&mut self) -> Result<Type, Box<Diagnostic>> {
        self.depth += 1;
        if self.depth > 128 {
            self.depth -= 1;
            return Err(self.nesting_error("syntax_nesting"));
        }
        let t = if self.eat("i64") {
            Type::I64
        } else if self.eat("bool") {
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
            let x = self.ty()?;
            self.expect_type_close()?;
            Type::Array(Box::new(x))
        } else if self.eat("Result") {
            self.expect("<", "grammar.Type", &["<"], self.prev_start())?;
            let a = self.ty()?;
            self.expect(",", "grammar.Type", &[","], self.prev_start())?;
            let b = self.ty()?;
            self.expect_type_close()?;
            Type::Result(Box::new(a), Box::new(b))
        } else {
            let tok = self.peek().clone();
            if tok.text == "Option" {
                return Err(self.feature("option", tok.span));
            }
            if tok.kind == Kind::Name {
                let id = if matches!(tok.text.as_str(), "Future" | "Task") {
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
                TYPE_START,
                self.peek().span.start as usize,
                None,
            ));
        };
        self.depth -= 1;
        Ok(t)
    }
    fn expect_type_close(&mut self) -> Result<(), Box<Diagnostic>> {
        if self.eat(">") {
            return Ok(());
        }
        if self.at_text(">=") {
            let t = self.peek().clone();
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
        self.depth += 1;
        if self.depth > 128 {
            self.depth -= 1;
            return Err(self.nesting_error("syntax_nesting"));
        }
        let st = self.peek().span.start as usize;
        self.expect("{", "grammar.Block", &["{"], st)?;
        self.local_scopes.push(HashSet::new());
        let mut ss = Vec::new();
        let mut pending_value: Option<Span> = None;
        while !self.at_text("}") && !self.eof() {
            if !self.starts_statement() {
                return Err(self.err(
                    if ss.is_empty() {
                        "block_item"
                    } else {
                        "grammar.Block"
                    },
                    BLOCK_NEXT,
                    st,
                    None,
                ));
            }
            let statement = self.statement(value_target, st)?;
            if matches!(
                &statement,
                Statement::If { .. } | Statement::While { .. } | Statement::Block(_)
            ) && self.at_text(";")
                && self.peek().span.start == self.peek().span.end
            {
                self.take();
            }
            if let Some(value_span) = pending_value.take() {
                let next_span = statement.span().clone();
                let mut d = self.branch_error("non_final_branch_value", value_span);
                d.related_spans.push(RelatedSpan {
                    role: RelatedRole::Context,
                    span: next_span,
                });
                return Err(d);
            }
            if let Statement::Value { span, .. } = &statement {
                pending_value = Some(self.span(span.start as usize, span.start as usize + 6));
            }
            ss.push(statement)
        }
        if self.eof() && ss.is_empty() {
            return Err(self.err("block_item", BLOCK_NEXT, st, None));
        }
        let end = self.peek().span.end as usize;
        self.expect("}", "grammar.Block", BLOCK_NEXT, st)?;
        self.depth -= 1;
        self.local_scopes.pop();
        Ok(Block {
            statements: ss,
            span: self.span(st, end),
        })
    }
    fn statement(
        &mut self,
        value_target: bool,
        block_start: usize,
    ) -> Result<Statement, Box<Diagnostic>> {
        let start = self.peek().span.start as usize;
        let previous_start = std::mem::replace(&mut self.statement_start, start);
        let result = self.statement_inner(value_target, block_start);
        self.statement_start = previous_start;
        result
    }
    fn statement_inner(
        &mut self,
        value_target: bool,
        block_start: usize,
    ) -> Result<Statement, Box<Diagnostic>> {
        let st = self.peek().span.start as usize;
        if let Some(id) = deferred_feature(&self.peek().text) {
            return Err(self.feature(id, self.peek().span.clone()));
        }
        if self.word("let") || self.word("var") {
            let mutable = self.eat("var");
            if !mutable {
                self.eat("let");
            }
            let n = self.expect_name("binding_name", &["Name"], st)?;
            let ann = if self.eat(":") {
                Some(self.ty()?)
            } else {
                None
            };
            self.expect(
                "=",
                "grammar.Binding",
                if ann.is_some() { &["="] } else { &[":", "="] },
                st,
            )?;
            if !self.starts_expr() {
                return Err(self.err("binding_initializer", EXPR_START, st, None));
            }
            let v = self.expr(0)?;
            self.reject_function_value(&v)?;
            let end = self.terminator(st, v.span().clone())?;
            if let Some(scope) = self.local_scopes.last_mut() {
                scope.insert(n.text.clone());
            }
            return Ok(Statement::Let {
                mutable,
                name: n.text,
                annotation: ann,
                value: v,
                span: self.span(st, end),
            });
        }
        if self.eat("return") {
            let val = if !self.starts_expr() || self.line_break_before_next() {
                None
            } else {
                Some(self.expr(0)?)
            };
            if let Some(value) = &val {
                self.reject_function_value(value)?;
            }
            let end = self.terminator(
                st,
                val.as_ref()
                    .map(|x| x.span().clone())
                    .unwrap_or_else(|| self.span(st, self.prev_end())),
            )?;
            return Ok(Statement::Return {
                value: val,
                span: self.span(st, end),
            });
        }
        if self.word("value")
            && self.tokens.get(self.at + 1).is_some_and(|x| x.text == ":")
            && self.source
                [self.peek().span.end as usize..self.tokens[self.at + 1].span.start as usize]
                .find(['\r', '\n'])
                .is_none()
        {
            self.take();
            let colon = self.take();
            let marker = self.span(st, colon.span.end as usize);
            if !value_target {
                let mut d = self.branch_error("invalid_value_target", marker);
                d.related_spans.push(RelatedSpan {
                    role: RelatedRole::Context,
                    span: self.span(block_start, block_start + 1),
                });
                return Err(d);
            }
            let val = if !self.starts_expr() || self.line_break_before_next() {
                None
            } else {
                Some(self.expr(0)?)
            };
            if val.is_none() {
                let mut d = self.branch_error("missing_value_operand", marker);
                d.related_spans.push(RelatedSpan {
                    role: RelatedRole::Context,
                    span: self.span(block_start, block_start + 1),
                });
                let after = colon.span.end as usize;
                let next = self.peek().span.start as usize;
                let trivia = if next == after && self.peek().span.start == self.peek().span.end {
                    &self.source[after..]
                } else {
                    &self.source[after..next]
                };
                let delimiter = if let Some(offset) = trivia.find(['\r', '\n']) {
                    let start = after + offset;
                    let len = if trivia.as_bytes().get(offset..offset + 2) == Some(b"\r\n") {
                        2
                    } else {
                        1
                    };
                    self.span(start, start + len)
                } else {
                    self.peek().span.clone()
                };
                d.related_spans.push(RelatedSpan {
                    role: RelatedRole::Delimiter,
                    span: delimiter,
                });
                return Err(d);
            }
            let end = self.terminator(
                st,
                val.as_ref()
                    .map(|x| x.span().clone())
                    .unwrap_or_else(|| self.span(st, self.prev_end())),
            )?;
            return Ok(Statement::Value {
                value: val,
                span: self.span(st, end),
            });
        }
        if self.word("if") {
            let e = self.if_form(false)?;
            if let Expr::If {
                condition,
                then_block,
                otherwise,
                span,
            } = e
            {
                return Ok(Statement::If {
                    condition: *condition,
                    then_block,
                    otherwise: otherwise.map(|x| {
                        Box::new(match *x {
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
        }
        if self.word("while") {
            self.take();
            let c = self.expr(0)?;
            let b = self.block()?;
            let end = b.span.end as usize;
            return Ok(Statement::While {
                condition: c,
                body: b,
                span: self.span(st, end),
            });
        }
        if self.at_text("{") {
            return Ok(Statement::Block(self.block()?));
        }
        // ExprStmt enters CompareExpr directly (§13.13); no enclosing Expr
        // activation is charged. Child arguments, indices and groups still are.
        let e = self.expr(1)?;
        if is_place_expr(&e) && self.eat("=") {
            let val = self.expr(0)?;
            self.reject_function_value(&val)?;
            let end = self.terminator(st, val.span().clone())?;
            return Ok(Statement::Assign {
                place: e,
                value: val,
                span: self.span(st, end),
            });
        }
        let end = self.terminator(st, e.span().clone())?;
        self.reject_function_value(&e)?;
        Ok(Statement::Expr {
            value: e,
            span: self.span(st, end),
        })
    }
    fn terminator(&mut self, st: usize, preceding: Span) -> Result<usize, Box<Diagnostic>> {
        if self.eat(";") {
            return Ok(self.prev_end());
        }
        if let Some(offset) = self.newline_after(preceding.end as usize) {
            return Ok(offset);
        }
        let p = self.peek().clone();
        Err(self.terminator_error(st, p, preceding))
    }
    fn terminator_error(&self, st: usize, p: Token, preceding: Span) -> Box<Diagnostic> {
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
            description: "Insert canonical statement terminator.".into(),
            edits: vec![diagnostic::Edit {
                span: self.span(preceding.end as usize, preceding.end as usize),
                expected_text: String::new(),
                replacement: ";".into(),
            }],
            validation: None,
            consequences: vec!["Termina la sentencia antes del siguiente token.".into()],
        });
        d
    }
    fn expr(&mut self, min: u8) -> Result<Expr, Box<Diagnostic>> {
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
                TRY_START,
                self.peek().span.start as usize,
                None,
            ));
        }
        let mut lhs = self.prefix()?;
        let mut try_seen = false;
        loop {
            if try_seen && matches!(self.peek().text.as_str(), "?" | "(" | "[") {
                return Err(self.err(
                    "grammar.TryExpr",
                    &["+", "-", ";", "<", "=="],
                    lhs.span().start as usize,
                    None,
                ));
            }
            if matches!(&lhs, Expr::If { .. }) && matches!(self.peek().text.as_str(), "(" | "[") {
                return Err(self.err("grammar.Expr", &[";"], lhs.span().start as usize, None));
            }
            if self.at_text("(") {
                let start = lhs.span().start as usize;
                self.take();
                let arguments_start = self.prev_start();
                let mut args = Vec::new();
                let mut after_comma = false;
                while !self.at_text(")") && !self.eof() {
                    if !self.starts_expr() {
                        return Err(self.err(
                            if after_comma {
                                "grammar.Argument"
                            } else {
                                "grammar.Arguments"
                            },
                            if after_comma { EXPR_START } else { GROUP_FIRST },
                            if after_comma {
                                self.peek().span.start as usize
                            } else {
                                arguments_start
                            },
                            None,
                        ));
                    }
                    let as_ = self.peek().span.start as usize;
                    let label = if self.peek().kind == Kind::Name
                        && self.tokens.get(self.at + 1).is_some_and(|t| t.text == ":")
                    {
                        let n = self.take().text;
                        self.take();
                        Some(n)
                    } else {
                        None
                    };
                    let v = self.expr(0)?;
                    self.reject_function_value(&v)?;
                    args.push(Argument {
                        label,
                        value: v,
                        span: self.span(as_, self.prev_end()),
                    });
                    if !self.eat(",") {
                        break;
                    }
                    after_comma = true;
                    if self.eof() {
                        return Err(self.err(
                            "grammar.Argument",
                            EXPR_START,
                            self.peek().span.start as usize,
                            None,
                        ));
                    }
                    if self.at_text(")") {
                        return Err(self.err(
                            "grammar.Argument",
                            EXPR_START,
                            self.peek().span.start as usize,
                            None,
                        ));
                    }
                }
                let end = self.peek().span.end as usize;
                self.expect(
                    ")",
                    "grammar.Arguments",
                    if args.is_empty() {
                        GROUP_FIRST
                    } else {
                        &[")", ","]
                    },
                    arguments_start,
                )?;
                lhs = Expr::Call {
                    target: Box::new(lhs),
                    args,
                    span: self.span(start, end),
                };
                self.check_expression_height(&lhs)?;
                continue;
            }
            if self.eat("[") {
                let start = lhs.span().start as usize;
                let index_start = self.prev_start();
                let ix = self.expr(0)?;
                let end = self.peek().span.end as usize;
                self.expect("]", "grammar.Index", &["]"], index_start)?;
                lhs = Expr::Index {
                    target: Box::new(lhs),
                    index: Box::new(ix),
                    span: self.span(start, end),
                };
                self.check_expression_height(&lhs)?;
                continue;
            }
            if self.at_text("?") {
                if min == 4 {
                    break;
                }
                if matches!(&lhs, Expr::If { .. }) {
                    return Err(self.err("grammar.Expr", &[";"], lhs.span().start as usize, None));
                }
                self.take();
                let s = lhs.span().start as usize;
                lhs = Expr::Unary {
                    mode: Mode::Read,
                    value: Box::new(lhs),
                    span: self.span(s, self.prev_end()),
                };
                self.check_expression_height(&lhs)?;
                try_seen = true;
                continue;
            }
            let op = self.peek().text.as_str();
            if op == "." {
                return Err(self.feature("methods", self.peek().span.clone()));
            }
            if op == ".." {
                return Err(self.feature("range", self.peek().span.clone()));
            }
            if op == "+=" {
                return Err(self.feature("compound_assignment", self.peek().span.clone()));
            }
            if matches!(op, "*" | "!=" | ">" | "<=" | ">=") {
                let t = self.peek().clone();
                return Err(self.syntax(
                    &t,
                    "unsupported_binary_operator",
                    &["+", "-", "<", "=="],
                    self.statement_start,
                ));
            }
            let prec = match op {
                "==" | "<" => 1,
                "+" | "-" => 2,
                _ => 0,
            };
            if prec < min || prec == 0 {
                break;
            }
            if matches!(&lhs, Expr::If { .. }) {
                return Err(self.err("grammar.Expr", &[";"], lhs.span().start as usize, None));
            }
            if prec == 1 && matches!(&lhs, Expr::Binary { op, .. } if op == "<" || op == "==") {
                return Err(self.err(
                    "grammar.CompareExpr",
                    &[";"],
                    lhs.span().start as usize,
                    None,
                ));
            }
            let o = self.take();
            let rhs = self.expr(prec + 1)?;
            let s = lhs.span().start as usize;
            lhs = Expr::Binary {
                op: o.text,
                left: Box::new(lhs),
                right: Box::new(rhs),
                span: self.span(s, self.prev_end()),
            };
            self.check_expression_height(&lhs)?;
        }
        if logical_entry {
            self.depth -= 1;
        }
        Ok(lhs)
    }
    fn prefix(&mut self) -> Result<Expr, Box<Diagnostic>> {
        let t = self.peek().clone();
        if t.kind == Kind::Decimal {
            return Err(self.feature("float_literal", t.span));
        }
        if t.kind == Kind::String {
            return Err(self.feature("text", t.span));
        }
        if t.text == ".." {
            return Err(self.feature("range", t.span));
        }
        if t.text == "move" {
            return Err(self.feature("legacy_move", t.span));
        }
        if t.text == "self" {
            return Err(self.feature("methods", t.span));
        }
        if t.text == "-" {
            let s = self.take().span.start as usize;
            if self.peek().kind == Kind::Integer {
                let n = self.take();
                return Ok(Expr::Integer(
                    format!("-{}", n.text),
                    self.span(s, n.span.end as usize),
                ));
            }
            let unexpected = if self.eof() { self.peek().clone() } else { t };
            return Err(self.syntax(&unexpected, "signed_integer", &["IntLiteral"], s));
        }
        let mode = if self.eat("consume") {
            Some(Mode::Consume)
        } else if self.eat("inout") {
            Some(Mode::Inout)
        } else {
            None
        };
        if let Some(m) = mode {
            let op = self.prev_start();
            if !self.starts_postfix() {
                return Err(self.err("grammar.Postfix", POSTFIX_START, op, None));
            }
            // AccessExpr owns its Postfix; TryExpr applies '?' to the completed
            // access, including the mode prefix (§3).
            let val = self.expr(4)?;
            if has_call_root(&val) {
                return Err(self.feature(
                    "receiver_mode",
                    self.span(op, op + if m == Mode::Consume { 7 } else { 5 }),
                ));
            }
            if let Some(bad) = place_root_failure(&val) {
                let tok = self
                    .tokens
                    .iter()
                    .find(|t| t.span.start == bad)
                    .cloned()
                    .unwrap_or_else(|| self.peek().clone());
                return Err(self.syntax(
                    &tok,
                    if m == Mode::Consume {
                        "consume_place"
                    } else {
                        "inout_place"
                    },
                    &["Place"],
                    op,
                ));
            }
            if m == Mode::Consume && is_projected_place(&val) {
                let mut d = self.feature("partial_transfer", val.span().clone());
                d.related_spans.push(RelatedSpan {
                    role: RelatedRole::Context,
                    span: self.span(op, op + 7),
                });
                return Err(d);
            }
            let sp = self.span(op, val.span().end as usize);
            let result = Expr::Unary {
                mode: m,
                value: Box::new(val),
                span: sp,
            };
            self.check_expression_height(&result)?;
            return Ok(result);
        }
        if t.text == "if" {
            return self.if_form(true);
        }
        if t.text == "fn"
            && self
                .tokens
                .get(self.at + 1)
                .is_some_and(|next| next.text == "[")
        {
            return Err(self.feature("closure", t.span));
        }
        if let Some(id) = deferred_feature(&t.text) {
            return Err(self.feature(id, t.span));
        }
        if t.kind == Kind::Integer {
            self.take();
            return Ok(Expr::Integer(t.text, t.span));
        }
        if t.text == "true" || t.text == "false" {
            self.take();
            return Ok(Expr::Bool(t.text == "true", t.span));
        }
        if t.kind == Kind::Name {
            self.take();
            return Ok(Expr::Name(t.text, t.span));
        }
        if self.eat("(") {
            let st = t.span.start as usize;
            if self.eat(")") {
                return Ok(Expr::Unit(self.span(st, self.prev_end())));
            }
            if !self.starts_expr() {
                return Err(self.err("grammar.Primary", GROUP_FIRST, st, None));
            }
            let x = self.expr(0)?;
            if self.eat(",") {
                return Err(self.feature("tuple", self.prev_span()));
            }
            let end = self.peek().span.end as usize;
            self.expect(")", "grammar.Primary", &[")"], st)?;
            return Ok(Expr::Group(Box::new(x), self.span(st, end)));
        }
        if self.eat("[") {
            let st = t.span.start as usize;
            let mut xs = Vec::new();
            let mut after_comma = false;
            while !self.at_text("]") && !self.eof() {
                if !self.starts_expr() {
                    return Err(self.err(
                        if after_comma {
                            "array_element"
                        } else {
                            "grammar.Primary"
                        },
                        if after_comma { EXPR_START } else { ARRAY_FIRST },
                        st,
                        None,
                    ));
                }
                xs.push(self.expr(0)?);
                if !self.eat(",") {
                    break;
                }
                after_comma = true;
                if self.eof() {
                    return Err(self.err("array_element", EXPR_START, st, None));
                }
                if self.at_text("]") {
                    return Err(self.err("array_element", EXPR_START, st, None));
                }
            }
            let end = self.peek().span.end as usize;
            self.expect(
                "]",
                "grammar.Primary",
                if xs.is_empty() {
                    ARRAY_FIRST
                } else {
                    &[",", "]"]
                },
                st,
            )?;
            let span = self.span(st, end);
            let height = 1 + xs.iter().map(expression_height).max().unwrap_or(0);
            self.check_candidate_height(height, span.clone())?;
            if xs.iter().any(is_resource_constructor) {
                let mut d = self.feature("resource_array", span);
                d.stage = Stage::Types;
                d.operation = Operation::TypesCheck;
                return Err(d);
            }
            return Ok(Expr::Array(xs, span));
        }
        Err(self.err("grammar.Primary", EXPR_START, t.span.start as usize, None))
    }
    fn if_form(&mut self, value_target: bool) -> Result<Expr, Box<Diagnostic>> {
        let mut branches = Vec::new();
        let otherwise = loop {
            let st = self.take().span.start as usize;
            let condition = self.expr(0)?;
            let then_block = self.block_with_target(value_target)?;
            branches.push((st, condition, then_block));
            if !self.eat("else") {
                if value_target {
                    let first = branches[0].0;
                    return Err(self.branch_error("missing_else", self.span(first, first + 2)));
                }
                break None;
            }
            if self.eof() {
                return Err(self.err("grammar.IfForm", ELSE_FIRST, st, None));
            }
            if !self.word("if") && !self.at_text("{") {
                return Err(self.err("grammar.IfForm", ELSE_FIRST, st, None));
            }
            if !self.word("if") {
                break Some(Else::Block(self.block_with_target(value_target)?));
            }
        };
        let mut otherwise = otherwise;
        while let Some((st, condition, then_block)) = branches.pop() {
            let end = otherwise
                .as_ref()
                .map_or(then_block.span.end as usize, |x| x.span().end as usize);
            let candidate = Expr::If {
                condition: Box::new(condition),
                then_block,
                otherwise: otherwise.map(Box::new),
                span: self.span(st, end),
            };
            self.check_expression_height(&candidate)?;
            if branches.is_empty() {
                return Ok(candidate);
            }
            otherwise = Some(Else::If(Box::new(candidate)));
        }
        unreachable!("if form has a branch")
    }
    fn check_expression_height(&self, expr: &Expr) -> Result<(), Box<Diagnostic>> {
        self.check_candidate_height(expression_height(expr), expr.span().clone())
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
        d.expected_tokens_kind = Some(ExpectedTokensKind::AdmissibleSet);
        d.parser_context = Some(ctx.into());
        d.recognized_prefix = Some(self.span(prefix_start, t.span.start as usize));
        d
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
        if matches!(id, "unsupported_type" | "option") {
            d.operation = Operation::TypesCheck;
        }
        d.data.insert(
            "pending_scope".into(),
            DataValue::String(
                if id == "legacy_move" {
                    "migration"
                } else if matches!(id, "closure" | "generator" | "option" | "match") {
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
        match expr {
            Expr::Name(name, span)
                if self.function_names.contains(name)
                    && !self
                        .local_scopes
                        .iter()
                        .rev()
                        .any(|scope| scope.contains(name)) =>
            {
                Some(span.clone())
            }
            Expr::Group(inner, _) | Expr::Unary { value: inner, .. } => self.function_value(inner),
            Expr::Binary { left, right, .. } => self
                .function_value(left)
                .or_else(|| self.function_value(right)),
            Expr::Call { args, .. } => args.iter().find_map(|arg| self.function_value(&arg.value)),
            Expr::Index { target, index, .. } => self
                .function_value(target)
                .or_else(|| self.function_value(index)),
            Expr::Array(xs, _) => xs.iter().find_map(|x| self.function_value(x)),
            _ => None,
        }
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
        self.peek().kind == Kind::Name || matches!(self.peek().text.as_str(), "inout" | "consume")
    }
    fn starts_restriction(&self) -> bool {
        is_restriction(&self.peek().text)
    }
    fn starts_expr(&self) -> bool {
        matches!(
            self.peek().kind,
            Kind::Name | Kind::Integer | Kind::Decimal | Kind::String
        ) || matches!(
            self.peek().text.as_str(),
            "(" | "[" | "-" | "true" | "false" | "if" | "consume" | "inout"
        ) || deferred_feature(&self.peek().text).is_some()
            || (self.word("fn")
                && self
                    .tokens
                    .get(self.at + 1)
                    .is_some_and(|next| next.text == "["))
    }
    fn starts_try(&self) -> bool {
        self.starts_expr() && !self.word("if")
    }
    fn starts_postfix(&self) -> bool {
        matches!(
            self.peek().kind,
            Kind::Name | Kind::Integer | Kind::Decimal | Kind::String
        ) || matches!(
            self.peek().text.as_str(),
            "(" | "[" | "-" | "true" | "false"
        ) || deferred_feature(&self.peek().text).is_some()
    }
    fn starts_statement(&self) -> bool {
        self.starts_expr()
            || matches!(
                self.peek().text.as_str(),
                "let" | "var" | "return" | "while" | "{"
            )
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
    match expr {
        Expr::Name(..) => None,
        Expr::Group(inner, _) => place_root_failure(inner),
        Expr::Index { target, .. } => place_root_failure(target),
        Expr::Binary { left, right, .. } => {
            place_root_failure(left).or(Some(left.span().end.min(right.span().start)))
        }
        Expr::Unary {
            mode: Mode::Read,
            value,
            span,
        } => place_root_failure(value).or(Some(span.end - 1)),
        other => Some(other.span().start),
    }
}
fn is_place_expr(expr: &Expr) -> bool {
    place_root_failure(expr).is_none()
}
fn has_call_root(expr: &Expr) -> bool {
    match expr {
        Expr::Call { .. } => true,
        Expr::Group(inner, _) | Expr::Index { target: inner, .. } => has_call_root(inner),
        _ => false,
    }
}
fn is_projected_place(expr: &Expr) -> bool {
    match expr {
        Expr::Index { .. } => true,
        Expr::Group(inner, _) => is_projected_place(inner),
        _ => false,
    }
}
fn is_resource_constructor(expr: &Expr) -> bool {
    matches!(expr, Expr::Call { target, .. } if matches!(&**target, Expr::Name(name, _) if name == "mock_open" || name == "mock_blocking_open"))
}

fn expression_height(expr: &Expr) -> usize {
    match expr {
        Expr::Name(..) | Expr::Integer(..) | Expr::Bool(..) | Expr::Unit(..) => 1,
        Expr::Group(inner, _) => expression_height(inner),
        Expr::Array(elements, _) => 1 + elements.iter().map(expression_height).max().unwrap_or(0),
        Expr::Unary { value, .. } => 1 + expression_height(value),
        Expr::Binary { left, right, .. } => {
            1 + expression_height(left).max(expression_height(right))
        }
        Expr::Call { target, args, .. } => {
            1 + expression_height(target).max(
                args.iter()
                    .map(|arg| expression_height(&arg.value))
                    .max()
                    .unwrap_or(0),
            )
        }
        Expr::Index { target, index, .. } => {
            1 + expression_height(target).max(expression_height(index))
        }
        Expr::If {
            condition,
            then_block,
            otherwise,
            ..
        } => {
            let branch = then_block
                .statements
                .iter()
                .map(statement_expression_height)
                .max()
                .unwrap_or(0);
            let alternative = otherwise.as_ref().map_or(0, |other| match &**other {
                Else::Block(block) => block
                    .statements
                    .iter()
                    .map(statement_expression_height)
                    .max()
                    .unwrap_or(0),
                Else::If(expr) => expression_height(expr),
            });
            1 + expression_height(condition).max(branch).max(alternative)
        }
    }
}

fn statement_expression_height(statement: &Statement) -> usize {
    match statement {
        Statement::Let { value, .. }
        | Statement::Assign { value, .. }
        | Statement::Expr { value, .. } => expression_height(value),
        Statement::Return { value, .. } | Statement::Value { value, .. } => {
            value.as_ref().map_or(0, expression_height)
        }
        Statement::If {
            condition,
            then_block,
            otherwise,
            ..
        } => expression_height(condition)
            .max(
                then_block
                    .statements
                    .iter()
                    .map(statement_expression_height)
                    .max()
                    .unwrap_or(0),
            )
            .max(
                otherwise
                    .as_ref()
                    .map_or(0, |other| statement_expression_height(other)),
            ),
        Statement::While {
            condition, body, ..
        } => expression_height(condition).max(
            body.statements
                .iter()
                .map(statement_expression_height)
                .max()
                .unwrap_or(0),
        ),
        Statement::Block(block) => block
            .statements
            .iter()
            .map(statement_expression_height)
            .max()
            .unwrap_or(0),
    }
}

#[cfg(test)]
mod tests;
