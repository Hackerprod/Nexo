use crate::Diagnostic;

#[derive(Clone, Debug, PartialEq)]
enum Kind {
    Ident(String),
    Number(String),
    Sym(char),
    Eof,
}
#[derive(Clone, Debug)]
struct Token {
    kind: Kind,
    pos: usize,
}
#[derive(Clone, Debug)]
pub struct Expr {
    pub kind: ExprKind,
    pub pos: usize,
    depth: usize,
}
impl Expr {
    fn new(kind: ExprKind, pos: usize) -> Result<Self, Diagnostic> {
        let child_depth = match &kind {
            ExprKind::Array(xs) => xs.iter().map(|e| e.depth).max().unwrap_or(0),
            ExprKind::Add(a, b) | ExprKind::Index(a, b) => a.depth.max(b.depth),
            ExprKind::Open(e) => e.depth,
            _ => 0,
        };
        let depth = child_depth + 1;
        if depth > 128 {
            return Err(Diagnostic::new(
                "E0003",
                pos,
                "composed expression depth exceeds F0 limit 128",
            ));
        }
        Ok(Self { kind, pos, depth })
    }
}
#[derive(Clone, Debug)]
pub enum ExprKind {
    Int(i64),
    Bool(bool),
    Var(String),
    Array(Vec<Expr>),
    Index(Box<Expr>, Box<Expr>),
    Add(Box<Expr>, Box<Expr>),
    Move(String),
    Open(Box<Expr>),
}
#[derive(Clone, Debug)]
pub struct Stmt {
    pub kind: StmtKind,
    pub pos: usize,
}
#[derive(Clone, Debug)]
pub enum StmtKind {
    Bind {
        name: String,
        mutable: bool,
        expr: Expr,
    },
    Assign {
        name: String,
        indices: Vec<Expr>,
        expr: Expr,
    },
    Assert(Expr, Expr),
    Print(Expr),
    Drop(String),
    Inspect(String),
    Block(Vec<Stmt>),
    Return,
    Fail,
}
fn lex(source: &str) -> Result<Vec<Token>, Diagnostic> {
    let mut out = Vec::new();
    let mut it = source.char_indices().peekable();
    while let Some((pos, ch)) = it.next() {
        if ch.is_whitespace() {
            continue;
        }
        if ch == '/' && it.peek().is_some_and(|(_, c)| *c == '/') {
            for (_, c) in it.by_ref() {
                if c == '\n' {
                    break;
                }
            }
            continue;
        }
        let kind = if ch.is_ascii_alphabetic() || ch == '_' {
            let mut s = String::from(ch);
            while let Some((_, c)) = it.peek() {
                if !c.is_ascii_alphanumeric() && *c != '_' {
                    break;
                }
                s.push(*c);
                it.next();
            }
            Kind::Ident(s)
        } else if ch.is_ascii_digit() {
            let mut s = String::from(ch);
            while let Some((_, c)) = it.peek() {
                if !c.is_ascii_digit() {
                    break;
                }
                s.push(*c);
                it.next();
            }
            Kind::Number(s)
        } else if "{}()[];,=+-".contains(ch) {
            Kind::Sym(ch)
        } else {
            return Err(Diagnostic::new(
                "E0001",
                pos,
                format!("unsupported character {ch:?} in F0 slice 1"),
            ));
        };
        out.push(Token { kind, pos });
    }
    out.push(Token {
        kind: Kind::Eof,
        pos: source.len(),
    });
    Ok(out)
}
struct Parser {
    tokens: Vec<Token>,
    at: usize,
    depth: usize,
}
impl Parser {
    fn peek(&self) -> &Token {
        &self.tokens[self.at]
    }
    fn take(&mut self) -> Token {
        let t = self.peek().clone();
        self.at += 1;
        t
    }
    fn error(&self, message: &str) -> Diagnostic {
        Diagnostic::new("E0002", self.peek().pos, message)
    }
    fn sym(&mut self, ch: char) -> Result<(), Diagnostic> {
        if self.peek().kind == Kind::Sym(ch) {
            self.take();
            Ok(())
        } else {
            Err(self.error(&format!("expected '{ch}'")))
        }
    }
    fn eat(&mut self, ch: char) -> bool {
        if self.peek().kind == Kind::Sym(ch) {
            self.take();
            true
        } else {
            false
        }
    }
    fn word(&mut self, word: &str) -> Result<(), Diagnostic> {
        if self.peek().kind == Kind::Ident(word.into()) {
            self.take();
            Ok(())
        } else {
            Err(self.error(&format!("expected '{word}'")))
        }
    }
    fn name(&mut self) -> Result<String, Diagnostic> {
        if let Kind::Ident(s) = &self.peek().kind {
            let s = s.clone();
            if [
                "fn",
                "let",
                "var",
                "return",
                "true",
                "false",
                "move",
                "mock_open",
                "assert_eq",
                "print",
                "drop",
                "inspect",
                "fail",
            ]
            .contains(&s.as_str())
            {
                return Err(self.error("reserved word cannot be a binding name"));
            }
            self.take();
            Ok(s)
        } else {
            Err(self.error("expected identifier"))
        }
    }
    fn enter(&mut self) -> Result<(), Diagnostic> {
        self.depth += 1;
        if self.depth > 128 {
            Err(Diagnostic::new(
                "E0003",
                self.peek().pos,
                "syntax nesting exceeds F0 limit 128",
            ))
        } else {
            Ok(())
        }
    }
    fn block(&mut self) -> Result<Vec<Stmt>, Diagnostic> {
        self.enter()?;
        self.sym('{')?;
        let mut body = Vec::new();
        while self.peek().kind != Kind::Sym('}') {
            if self.peek().kind == Kind::Eof {
                return Err(self.error("unterminated block"));
            }
            body.push(self.stmt()?);
        }
        self.sym('}')?;
        self.depth -= 1;
        Ok(body)
    }
    fn stmt(&mut self) -> Result<Stmt, Diagnostic> {
        let pos = self.peek().pos;
        if self.peek().kind == Kind::Sym('{') {
            return Ok(Stmt {
                kind: StmtKind::Block(self.block()?),
                pos,
            });
        }
        let token = self.take();
        let Kind::Ident(word) = token.kind else {
            return Err(Diagnostic::new("E0002", pos, "expected statement"));
        };
        let kind = match word.as_str() {
            "let" | "var" => {
                let name = self.name()?;
                self.sym('=')?;
                let expr = self.expr()?;
                StmtKind::Bind {
                    name,
                    mutable: word == "var",
                    expr,
                }
            }
            "assert_eq" => {
                self.sym('(')?;
                let a = self.expr()?;
                self.sym(',')?;
                let b = self.expr()?;
                self.sym(')')?;
                StmtKind::Assert(a, b)
            }
            "print" => {
                self.sym('(')?;
                let a = self.expr()?;
                self.sym(')')?;
                StmtKind::Print(a)
            }
            "drop" | "inspect" => {
                self.sym('(')?;
                let name = self.name()?;
                self.sym(')')?;
                if word == "drop" {
                    StmtKind::Drop(name)
                } else {
                    StmtKind::Inspect(name)
                }
            }
            "return" => StmtKind::Return,
            "fail" => {
                self.sym('(')?;
                self.sym(')')?;
                StmtKind::Fail
            }
            name => {
                let mut indices = Vec::new();
                while self.eat('[') {
                    indices.push(self.expr()?);
                    self.sym(']')?;
                }
                self.sym('=')?;
                let expr = self.expr()?;
                StmtKind::Assign {
                    name: name.into(),
                    indices,
                    expr,
                }
            }
        };
        self.sym(';')?;
        Ok(Stmt { kind, pos })
    }
    fn expr(&mut self) -> Result<Expr, Diagnostic> {
        self.enter()?;
        let mut left = self.postfix()?;
        let mut chain = 0;
        while self.eat('+') {
            chain += 1;
            if chain > 128 {
                return Err(Diagnostic::new(
                    "E0003",
                    left.pos,
                    "expression chain exceeds F0 limit 128",
                ));
            }
            let right = self.postfix()?;
            let pos = left.pos;
            left = Expr::new(ExprKind::Add(Box::new(left), Box::new(right)), pos)?;
        }
        self.depth -= 1;
        Ok(left)
    }
    fn postfix(&mut self) -> Result<Expr, Diagnostic> {
        let mut expr = self.atom()?;
        let mut chain = 0;
        while self.eat('[') {
            chain += 1;
            if chain > 128 {
                return Err(Diagnostic::new(
                    "E0003",
                    expr.pos,
                    "index chain exceeds F0 limit 128",
                ));
            }
            let index = self.expr()?;
            self.sym(']')?;
            let pos = expr.pos;
            expr = Expr::new(ExprKind::Index(Box::new(expr), Box::new(index)), pos)?;
        }
        Ok(expr)
    }
    fn atom(&mut self) -> Result<Expr, Diagnostic> {
        let token = self.take();
        let pos = token.pos;
        let kind =
            match token.kind {
                Kind::Number(s) => ExprKind::Int(s.parse().map_err(|_| {
                    Diagnostic::new("E0107", pos, "integer literal outside i64 range")
                })?),
                Kind::Sym('-') => {
                    let t = self.take();
                    let Kind::Number(s) = t.kind else {
                        return Err(Diagnostic::new(
                            "E0002",
                            pos,
                            "minus only supports integer literals in this slice",
                        ));
                    };
                    ExprKind::Int(format!("-{s}").parse().map_err(|_| {
                        Diagnostic::new("E0107", pos, "integer literal outside i64 range")
                    })?)
                }
                Kind::Ident(s) if s == "true" || s == "false" => ExprKind::Bool(s == "true"),
                Kind::Ident(s) if s == "move" => ExprKind::Move(self.name()?),
                Kind::Ident(s) if s == "mock_open" => {
                    self.sym('(')?;
                    let value = self.expr()?;
                    self.sym(')')?;
                    ExprKind::Open(Box::new(value))
                }
                Kind::Ident(s) => ExprKind::Var(s),
                Kind::Sym('[') => {
                    let mut values = Vec::new();
                    if !self.eat(']') {
                        loop {
                            values.push(self.expr()?);
                            if self.eat(']') {
                                break;
                            }
                            self.sym(',')?;
                        }
                    }
                    ExprKind::Array(values)
                }
                Kind::Sym('(') => {
                    let value = self.expr()?;
                    self.sym(')')?;
                    return Ok(value);
                }
                _ => return Err(Diagnostic::new("E0002", pos, "expected expression")),
            };
        Expr::new(kind, pos)
    }
}
pub fn parse(source: &str) -> Result<Vec<Stmt>, Diagnostic> {
    if source.len() > 1_048_576 {
        return Err(Diagnostic::new("E0003", 0, "F0 source limit is 1 MiB"));
    }
    let mut p = Parser {
        tokens: lex(source)?,
        at: 0,
        depth: 0,
    };
    p.word("fn")?;
    p.word("main")?;
    p.sym('(')?;
    p.sym(')')?;
    let body = p.block()?;
    if p.peek().kind != Kind::Eof {
        return Err(p.error("only one fn main is supported in F0 slice 1"));
    }
    Ok(body)
}
