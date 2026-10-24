//! Concrete source tokens and attached line comments for the B formatter.
use super::lex::{self, Kind, Token};
use super::parse::{self, Ast};
use crate::diagnostic::Diagnostic;
use std::ops::Deref;

/// Identity is assigned once, in concrete source order, after newline trivia is removed.
#[derive(Clone, Debug)]
pub struct CstToken {
    pub id: usize,
    pub token: Token,
}

impl Deref for CstToken {
    type Target = Token;

    fn deref(&self) -> &Self::Target {
        &self.token
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Attachment {
    Previous(usize),
    Next(usize),
    Eof,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Comment {
    pub start: usize,
    pub end: usize,
    pub documentary: bool,
    pub attachment: Attachment,
}

#[derive(Clone, Debug)]
pub struct Cst {
    pub source: String,
    pub ast: Ast,
    pub tokens: Vec<CstToken>,
    pub comments: Vec<Comment>,
}

pub fn parse_cst(file: &str, source: &str) -> Result<Cst, Box<Diagnostic>> {
    let (ast, split_type_closes) = parse::parse_with_split_tokens(file, source)?;
    let mut tokens = Vec::new();
    for mut token in lex::lex(file, source)?
        .into_iter()
        .filter(|token| token.kind != Kind::Newline)
    {
        if split_type_closes
            .binary_search(&(token.span.start as usize))
            .is_ok()
        {
            // This production consumes one lexical >= as two concrete syntax
            // tokens. The parser, rather than the formatter, records the split.
            let mut close = token.clone();
            close.text = ">".into();
            close.span.end = close.span.start + 1;
            tokens.push(CstToken {
                id: tokens.len(),
                token: close,
            });
            token.text = "=".into();
            token.span.start += 1;
        }
        tokens.push(CstToken {
            id: tokens.len(),
            token,
        });
    }
    let mut comments = Vec::new();
    let mut gap_start = 0;
    for (next, token) in tokens.iter().enumerate() {
        let gap_end = token.span.start as usize;
        let mut at = gap_start;
        while at < gap_end {
            if source[at..gap_end].starts_with("//") {
                let start = at;
                at += 2;
                while at < gap_end
                    && source.as_bytes()[at] != b'\r'
                    && source.as_bytes()[at] != b'\n'
                {
                    at += source[at..]
                        .chars()
                        .next()
                        .expect("character boundary")
                        .len_utf8();
                }
                let attachment = if next > 0
                    && !source[tokens[next - 1].span.end as usize..start].contains(['\r', '\n'])
                {
                    Attachment::Previous(tokens[next - 1].id)
                } else if token.kind == Kind::Eof {
                    Attachment::Eof
                } else {
                    Attachment::Next(token.id)
                };
                comments.push(Comment {
                    start,
                    end: at,
                    documentary: source[start..at].starts_with("///"),
                    attachment,
                });
            } else {
                at += source[at..]
                    .chars()
                    .next()
                    .expect("character boundary")
                    .len_utf8();
            }
        }
        gap_start = token.span.end as usize;
    }
    Ok(Cst {
        source: source.into(),
        ast,
        tokens,
        comments,
    })
}
