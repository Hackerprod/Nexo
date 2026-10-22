//! Lexer for the lexical layer of `f0-0002-v1`.
//!
//! This module is intentionally not wired into the slice-1 compiler entry
//! point: I02a adds the new token stream without changing current CLI output.

#[cfg(test)]
thread_local! {
    static LEX_THREAD_WORK: std::cell::Cell<Option<u64>> = const { std::cell::Cell::new(None) };
}
#[cfg(test)]
pub(crate) fn measure_lex_work<T>(work: impl FnOnce() -> T) -> (T, u64) {
    LEX_THREAD_WORK.with(|cell| cell.set(Some(0)));
    let result = work();
    let steps = LEX_THREAD_WORK.with(|cell| cell.replace(None).unwrap());
    (result, steps)
}

use std::collections::BTreeMap;

use crate::diagnostic::{
    self, Diagnostic, ExpectedTokensKind, Namespace, Operation, Severity, Span, Stage,
    Token as DiagnosticToken, TokenKind,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Name,
    Keyword,
    Integer,
    Decimal,
    String,
    Symbol,
    Newline,
    Eof,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Token {
    pub kind: Kind,
    /// Source spelling, excluding trivia; EOF has an empty spelling.
    pub text: String,
    pub span: Span,
}

/// Lexes valid UTF-8 source and reports the first unrecognized token as NX-E0001.
pub fn lex(file: &str, source: &str) -> Result<Vec<Token>, Box<Diagnostic>> {
    let mut lexer = Lexer {
        file,
        source,
        at: 0,
        tokens: Vec::new(),
    };
    lexer.scan()?;
    Ok(lexer.tokens)
}

struct Lexer<'a> {
    file: &'a str,
    source: &'a str,
    at: usize,
    tokens: Vec<Token>,
}

impl Lexer<'_> {
    fn scan(&mut self) -> Result<(), Box<Diagnostic>> {
        while self.at < self.source.len() {
            let start = self.at;
            let ch = self.peek_char().expect("offset is inside source");
            if ch == '\r' || ch == '\n' {
                self.consume_newline();
                self.push(Kind::Newline, start, self.at);
            } else if ch.is_whitespace() {
                self.bump();
            } else if self.source[self.at..].starts_with("//") {
                self.at += 2;
                while let Some(next) = self.peek_char() {
                    if next == '\r' || next == '\n' {
                        break;
                    }
                    self.bump();
                }
            } else if is_ident_start(ch) {
                self.bump();
                while self.peek_char().is_some_and(is_ident_continue) {
                    self.bump();
                }
                let word = &self.source[start..self.at];
                let kind = if is_reserved(word) {
                    Kind::Keyword
                } else {
                    Kind::Name
                };
                self.push(kind, start, self.at);
            } else if ch.is_ascii_digit() {
                self.bump();
                while self.peek_char().is_some_and(|c| c.is_ascii_digit()) {
                    self.bump();
                }
                let integer_end = self.at;
                if self.peek_char() == Some('.')
                    && !self.source[self.at..].starts_with("..")
                    && self.source[self.at + 1..]
                        .chars()
                        .next()
                        .is_some_and(|c| c.is_ascii_digit())
                {
                    self.bump();
                    while self.peek_char().is_some_and(|c| c.is_ascii_digit()) {
                        self.bump();
                    }
                    self.push(Kind::Decimal, start, self.at);
                } else {
                    self.push(Kind::Integer, start, integer_end);
                }
            } else if ch == '"' {
                self.bump();
                let mut closed = false;
                while let Some(next) = self.peek_char() {
                    if next == '"' {
                        self.bump();
                        closed = true;
                        break;
                    }
                    if next == '\r' || next == '\n' {
                        break;
                    }
                    if next == '\\' {
                        self.bump();
                        if self.peek_char().is_some() {
                            self.bump();
                        }
                    } else {
                        self.bump();
                    }
                }
                if !closed {
                    return Err(self.error(start, self.at));
                }
                self.push(Kind::String, start, self.at);
            } else if let Some(symbol) = longest_symbol(&self.source[self.at..]) {
                self.at += symbol.len();
                self.push(Kind::Symbol, start, self.at);
            } else {
                self.bump();
                return Err(self.error(start, self.at));
            }
        }
        let end = self.source.len();
        self.tokens.push(Token {
            kind: Kind::Eof,
            text: String::new(),
            span: self.span(end, end),
        });
        Ok(())
    }

    fn error(&self, start: usize, end: usize) -> Box<Diagnostic> {
        let spelling = &self.source[start..end];
        Box::new(Diagnostic {
            schema_version: diagnostic::SCHEMA_VERSION,
            diagnostic_namespace: Namespace::NexoProposedV1,
            profile_id: diagnostic::PROFILE_ID.into(),
            code: "NX-E0001".into(),
            subcause: None,
            rule: "NX-E0001".into(),
            explanation_id: Some("NX-E0001".into()),
            severity: Severity::Error,
            stage: Stage::Lex,
            message: "Token no reconocido por la gramática del corte.".into(),
            operation: Operation::SyntaxLex,
            primary_span: Some(self.span(start, end)),
            related_spans: Vec::new(),
            cause_chain: Vec::new(),
            feature_id: None,
            unexpected_token: Some(DiagnosticToken {
                kind: TokenKind::Invalid,
                text: Some(spelling.into()),
                span: self.span(start, end),
            }),
            expected_tokens: vec!["valid_token".into()],
            expected_tokens_kind: Some(ExpectedTokensKind::LexicalCategory),
            parser_context: Some("lexical_token".into()),
            recognized_prefix: Some(self.span(0, start)),
            repair: Vec::new(),
            data: BTreeMap::new(),
        })
    }

    fn span(&self, start: usize, end: usize) -> Span {
        Span {
            file: self.file.into(),
            start: start as u64,
            end: end as u64,
        }
    }
    fn push(&mut self, kind: Kind, start: usize, end: usize) {
        self.tokens.push(Token {
            kind,
            text: self.source[start..end].into(),
            span: self.span(start, end),
        });
    }
    fn peek_char(&self) -> Option<char> {
        #[cfg(test)]
        LEX_THREAD_WORK.with(|cell| {
            if let Some(current) = cell.get() {
                cell.set(Some(current.saturating_add(1)));
            }
        });
        self.source[self.at..].chars().next()
    }
    fn bump(&mut self) {
        self.at += self.peek_char().expect("character exists").len_utf8();
    }
    fn consume_newline(&mut self) {
        if self.source[self.at..].starts_with("\r\n") {
            self.at += 2;
        } else {
            self.bump();
        }
    }
}

fn is_ident_start(ch: char) -> bool {
    ch.is_ascii_alphabetic() || ch == '_'
}
fn is_ident_continue(ch: char) -> bool {
    ch.is_ascii_alphanumeric() || ch == '_'
}

fn is_reserved(word: &str) -> bool {
    matches!(
        word,
        "fn" | "let"
            | "var"
            | "return"
            | "if"
            | "else"
            | "while"
            | "true"
            | "false"
            | "inout"
            | "consume"
            | "with"
            | "match"
            | "for"
            | "break"
            | "continue"
            | "gen"
            | "yield"
            | "async"
            | "await"
            | "region"
            | "struct"
            | "enum"
            | "type"
            | "trait"
            | "impl"
            | "where"
            | "dyn"
            | "mod"
            | "use"
            | "pub"
            | "extend"
            | "extension"
            | "comptime"
            | "const"
            | "self"
            | "super"
            | "package"
            | "move"
    )
}

fn longest_symbol(source: &str) -> Option<&'static str> {
    [
        "..", "+=", "!=", "->", "=>", "==", "<=", ">=", "(", ")", "{", "}", "[", "]", ",", ":",
        ";", "?", "+", "-", "<", ">", "=", "@", ".", "*",
    ]
    .into_iter()
    .find(|symbol| source.starts_with(symbol))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn token(kind: Kind, text: &str, start: u64, end: u64) -> Token {
        Token {
            kind,
            text: text.into(),
            span: Span {
                file: "x".into(),
                start,
                end,
            },
        }
    }

    fn assert_tokens(source: &str, expected: &[Token]) {
        assert_eq!(lex("x", source).unwrap(), expected);
    }

    fn eof(source: &str) -> Token {
        token(Kind::Eof, "", source.len() as u64, source.len() as u64)
    }

    fn assert_lex_error(source: &str, file: &str, start: u64, end: u64, text: &str) {
        let span = Span {
            file: file.into(),
            start,
            end,
        };
        let error = lex(file, source).unwrap_err();
        let expected = Diagnostic {
            schema_version: diagnostic::SCHEMA_VERSION,
            diagnostic_namespace: Namespace::NexoProposedV1,
            profile_id: diagnostic::PROFILE_ID.into(),
            code: "NX-E0001".into(),
            subcause: None,
            rule: "NX-E0001".into(),
            explanation_id: Some("NX-E0001".into()),
            severity: Severity::Error,
            stage: Stage::Lex,
            message: "Token no reconocido por la gramática del corte.".into(),
            operation: Operation::SyntaxLex,
            primary_span: Some(span.clone()),
            related_spans: Vec::new(),
            cause_chain: Vec::new(),
            feature_id: None,
            unexpected_token: Some(DiagnosticToken {
                kind: TokenKind::Invalid,
                text: Some(text.into()),
                span,
            }),
            expected_tokens: vec!["valid_token".into()],
            expected_tokens_kind: Some(ExpectedTokensKind::LexicalCategory),
            parser_context: Some("lexical_token".into()),
            recognized_prefix: Some(Span {
                file: file.into(),
                start: 0,
                end: start,
            }),
            repair: Vec::new(),
            data: BTreeMap::new(),
        };
        assert_eq!(error.as_ref(), &expected);
        error.validate(file, source).unwrap();
    }

    #[test]
    fn emitted_nx_e0001_has_literal_lexical_contract_and_byte_spans() {
        let source = "fn main(){let x=1£2;}";
        let error = lex("x.nx", source).unwrap_err();
        assert_eq!(error.code, "NX-E0001");
        assert_eq!(error.diagnostic_namespace, Namespace::NexoProposedV1);
        assert_eq!(error.profile_id, "f0-0002-v1");
        assert_eq!(error.rule, "NX-E0001");
        assert_eq!(error.explanation_id.as_deref(), Some("NX-E0001"));
        assert_eq!(error.severity, Severity::Error);
        assert_eq!(error.stage, Stage::Lex);
        assert_eq!(error.operation, Operation::SyntaxLex);
        assert_eq!(
            error.message,
            "Token no reconocido por la gramática del corte."
        );
        assert_eq!(
            error.primary_span,
            Some(Span {
                file: "x.nx".into(),
                start: 17,
                end: 19
            })
        );
        assert_eq!(
            error.unexpected_token,
            Some(DiagnosticToken {
                kind: TokenKind::Invalid,
                text: Some("£".into()),
                span: Span {
                    file: "x.nx".into(),
                    start: 17,
                    end: 19
                }
            })
        );
        assert_eq!(error.parser_context.as_deref(), Some("lexical_token"));
        assert_eq!(error.expected_tokens, ["valid_token"]);
        assert_eq!(
            error.expected_tokens_kind,
            Some(ExpectedTokensKind::LexicalCategory)
        );
        assert_eq!(
            error.recognized_prefix,
            Some(Span {
                file: "x.nx".into(),
                start: 0,
                end: 17
            })
        );
        assert_eq!(error.subcause, None);
        assert_eq!(error.feature_id, None);
        assert!(
            error.related_spans.is_empty()
                && error.cause_chain.is_empty()
                && error.repair.is_empty()
                && error.data.is_empty()
        );
        error.validate("x.nx", source).unwrap();
    }

    #[test]
    fn lx01_to_lx05_are_single_character_lexical_failures() {
        for (bad, good) in [('/', '+'), ('%', '+'), ('!', '+'), ('&', '+'), ('|', '+')] {
            let source = format!("fn main(){{let x=1{bad}2;}}");
            assert_lex_error(&source, "lx.nx", 17, 18, &bad.to_string());
            let positive = format!("fn main(){{let x=1{good}2;}}");
            assert_tokens(
                &positive,
                &[
                    token(Kind::Keyword, "fn", 0, 2),
                    token(Kind::Name, "main", 3, 7),
                    token(Kind::Symbol, "(", 7, 8),
                    token(Kind::Symbol, ")", 8, 9),
                    token(Kind::Symbol, "{", 9, 10),
                    token(Kind::Keyword, "let", 10, 13),
                    token(Kind::Name, "x", 14, 15),
                    token(Kind::Symbol, "=", 15, 16),
                    token(Kind::Integer, "1", 16, 17),
                    token(Kind::Symbol, "+", 17, 18),
                    token(Kind::Integer, "2", 18, 19),
                    token(Kind::Symbol, ";", 19, 20),
                    token(Kind::Symbol, "}", 20, 21),
                    eof(&positive),
                ],
            );
        }
    }

    #[test]
    fn complete_symbol_inventory_and_longest_match_are_tokenized() {
        let source = "( ) { } [ ] , : ; ? + - < > = @ . * .. += != -> => == <= >=";
        let spellings = [
            "(", ")", "{", "}", "[", "]", ",", ":", ";", "?", "+", "-", "<", ">", "=", "@", ".",
            "*", "..", "+=", "!=", "->", "=>", "==", "<=", ">=",
        ];
        let mut expected = Vec::new();
        let mut at = 0;
        for spelling in spellings {
            let start = source[at..].find(spelling).unwrap() + at;
            expected.push(token(
                Kind::Symbol,
                spelling,
                start as u64,
                (start + spelling.len()) as u64,
            ));
            at = start + spelling.len();
        }
        expected.push(eof(source));
        assert_tokens(source, &expected);
    }

    #[test]
    fn reserved_word_classes_match_the_closed_adr_lists() {
        let included = "fn let var return if else while true false inout consume with";
        let deferred = "match for break continue gen yield async await region struct enum type trait impl where dyn mod use pub extend extension comptime const self super package";
        let migration = "move";
        let ordinary = "value from result_from retains borrow copy as in create call resume cancel drop unrestricted no_alloc no_block no_suspend no_io no_heap_alloc i64 Result";
        for word in included
            .split_whitespace()
            .chain(deferred.split_whitespace())
            .chain(migration.split_whitespace())
        {
            assert_tokens(
                word,
                &[token(Kind::Keyword, word, 0, word.len() as u64), eof(word)],
            );
        }
        for word in ordinary.split_whitespace() {
            assert_tokens(
                word,
                &[token(Kind::Name, word, 0, word.len() as u64), eof(word)],
            );
        }
    }

    #[test]
    fn incomplete_string_reports_the_full_consumed_lexeme() {
        let source = "let s=\"ab";
        assert_lex_error(source, "x", 6, 9, "\"ab");
    }

    #[test]
    fn identifiers_keywords_literals_comments_and_line_endings_preserve_spans() {
        let source = "let α=0.5\r\n// comentario 🙂\nvalue=\"ok\"";
        assert_lex_error(source, "x", 4, 6, "α");
        let source = "let _x9=0.5\r\n// comentario 🙂\nvalue=\"ok\"";
        assert_tokens(
            source,
            &[
                token(Kind::Keyword, "let", 0, 3),
                token(Kind::Name, "_x9", 4, 7),
                token(Kind::Symbol, "=", 7, 8),
                token(Kind::Decimal, "0.5", 8, 11),
                token(Kind::Newline, "\r\n", 11, 13),
                token(Kind::Newline, "\n", 31, 32),
                token(Kind::Name, "value", 32, 37),
                token(Kind::Symbol, "=", 37, 38),
                token(Kind::String, "\"ok\"", 38, 42),
                eof(source),
            ],
        );
    }

    #[test]
    fn decimal_range_and_separated_dot_follow_profile_boundaries() {
        assert_tokens(
            "0.5 0..n 0 . 5",
            &[
                token(Kind::Decimal, "0.5", 0, 3),
                token(Kind::Integer, "0", 4, 5),
                token(Kind::Symbol, "..", 5, 7),
                token(Kind::Name, "n", 7, 8),
                token(Kind::Integer, "0", 9, 10),
                token(Kind::Symbol, ".", 11, 12),
                token(Kind::Integer, "5", 13, 14),
                eof("0.5 0..n 0 . 5"),
            ],
        );
    }

    #[test]
    fn decimal_and_single_digit_integer_boundaries_have_exact_sequences() {
        assert_tokens(
            "10.25",
            &[token(Kind::Decimal, "10.25", 0, 5), eof("10.25")],
        );
        assert_tokens("007", &[token(Kind::Integer, "007", 0, 3), eof("007")]);
        assert_tokens("9", &[token(Kind::Integer, "9", 0, 1), eof("9")]);
    }

    #[test]
    fn minus_is_always_a_separate_symbol_token() {
        assert_tokens(
            "-1",
            &[
                token(Kind::Symbol, "-", 0, 1),
                token(Kind::Integer, "1", 1, 2),
                eof("-1"),
            ],
        );
        assert_tokens(
            "1-2",
            &[
                token(Kind::Integer, "1", 0, 1),
                token(Kind::Symbol, "-", 1, 2),
                token(Kind::Integer, "2", 2, 3),
                eof("1-2"),
            ],
        );
        assert_tokens(
            "1 - 2",
            &[
                token(Kind::Integer, "1", 0, 1),
                token(Kind::Symbol, "-", 2, 3),
                token(Kind::Integer, "2", 4, 5),
                eof("1 - 2"),
            ],
        );
        assert_tokens(
            "--1",
            &[
                token(Kind::Symbol, "-", 0, 1),
                token(Kind::Symbol, "-", 1, 2),
                token(Kind::Integer, "1", 2, 3),
                eof("--1"),
            ],
        );
    }

    #[test]
    fn block_comment_syntax_is_rejected_at_its_first_slash() {
        assert_lex_error("/*x*/", "x", 0, 1, "/");
        assert_lex_error("a /*x*/ b", "x", 2, 3, "/");
    }

    #[test]
    fn adr_gap_whitespace_includes_tab_and_nonbreaking_space() {
        assert_tokens(
            "a\t\u{00a0}b",
            &[
                token(Kind::Name, "a", 0, 1),
                token(Kind::Name, "b", 4, 5),
                eof("a\t\u{00a0}b"),
            ],
        );
    }

    #[test]
    fn adr_gap_digit_letter_boundary_is_integer_then_name() {
        assert_tokens(
            "1a",
            &[
                token(Kind::Integer, "1", 0, 1),
                token(Kind::Name, "a", 1, 2),
                eof("1a"),
            ],
        );
    }

    #[test]
    fn adr_gap_unclosed_string_stops_before_lf() {
        assert_lex_error("\"a\nb\"", "x", 0, 2, "\"a");
    }

    #[test]
    fn dot_without_following_digit_stays_a_symbol() {
        assert_tokens(
            "1.",
            &[
                token(Kind::Integer, "1", 0, 1),
                token(Kind::Symbol, ".", 1, 2),
                eof("1."),
            ],
        );
        assert_tokens(
            "1.x",
            &[
                token(Kind::Integer, "1", 0, 1),
                token(Kind::Symbol, ".", 1, 2),
                token(Kind::Name, "x", 2, 3),
                eof("1.x"),
            ],
        );
    }

    #[test]
    fn comment_ends_at_cr_and_crlf_span_keeps_both_bytes() {
        assert_tokens(
            "a//c\r\nb",
            &[
                token(Kind::Name, "a", 0, 1),
                token(Kind::Newline, "\r\n", 4, 6),
                token(Kind::Name, "b", 6, 7),
                eof("a//c\r\nb"),
            ],
        );
    }

    #[test]
    fn underscores_and_contextual_names_are_single_name_tokens() {
        assert_tokens("a_b", &[token(Kind::Name, "a_b", 0, 3), eof("a_b")]);
        assert_tokens(
            "result_from",
            &[token(Kind::Name, "result_from", 0, 11), eof("result_from")],
        );
    }

    #[test]
    fn unicode_digits_and_letters_are_rejected_as_single_utf8_tokens() {
        for (source, end) in [("²", 2), ("٣", 2), ("日", 3)] {
            assert_lex_error(source, "x", 0, end, source);
        }
    }

    #[test]
    fn migrated_k01_lexical_error_has_literal_fields_and_byte_spans() {
        let source = "fn main() { £ }";
        let error = lex("x", source).unwrap_err();
        assert_eq!(error.code, "NX-E0001");
        assert_eq!(error.diagnostic_namespace, Namespace::NexoProposedV1);
        assert_eq!(error.profile_id, "f0-0002-v1");
        assert_eq!(error.rule, "NX-E0001");
        assert_eq!(error.explanation_id.as_deref(), Some("NX-E0001"));
        assert_eq!(error.severity, Severity::Error);
        assert_eq!(error.stage, Stage::Lex);
        assert_eq!(error.operation, Operation::SyntaxLex);
        assert_eq!(
            error.primary_span,
            Some(Span {
                file: "x".into(),
                start: 12,
                end: 14
            })
        );
        assert_eq!(
            error.unexpected_token,
            Some(DiagnosticToken {
                kind: TokenKind::Invalid,
                text: Some("£".into()),
                span: Span {
                    file: "x".into(),
                    start: 12,
                    end: 14
                },
            })
        );
        assert_eq!(error.parser_context.as_deref(), Some("lexical_token"));
        assert_eq!(error.expected_tokens, ["valid_token"]);
        assert_eq!(
            error.expected_tokens_kind,
            Some(ExpectedTokensKind::LexicalCategory)
        );
        assert_eq!(
            error.recognized_prefix,
            Some(Span {
                file: "x".into(),
                start: 0,
                end: 12
            })
        );
        assert_eq!(error.subcause, None);
        assert_eq!(error.feature_id, None);
        assert_eq!(
            error.message,
            "Token no reconocido por la gramática del corte."
        );
        assert!(
            error.related_spans.is_empty()
                && error.cause_chain.is_empty()
                && error.repair.is_empty()
                && error.data.is_empty()
        );
        error.validate("x", source).unwrap();
    }

    fn is_trivia_gap(gap: &str) -> bool {
        let mut at = 0;
        while at < gap.len() {
            let rest = &gap[at..];
            if let Some(comment) = rest.strip_prefix("//") {
                return !comment.chars().any(|ch| ch == '\r' || ch == '\n');
            }
            let ch = rest.chars().next().unwrap();
            if !ch.is_whitespace() {
                return false;
            }
            at += ch.len_utf8();
        }
        true
    }

    #[test]
    fn trivia_gap_requires_line_comment_to_end_at_line_or_eof() {
        assert!(is_trivia_gap("  // comentario"));
        assert!(!is_trivia_gap("  // comentario\nresto"));
        assert!(!is_trivia_gap("  // comentario\rresto"));
    }

    #[test]
    fn generated_inputs_preserve_token_span_and_eof_invariants() {
        let atoms = [
            "name",
            "a_b",
            "result_from",
            "fn",
            "007",
            "10.25",
            "1.",
            ".",
            "..",
            "(",
            ")",
            "=",
            "==",
            "\"s\"",
            "//c",
            "\r\n",
            "\n",
            "🙂",
        ];
        let mut state = 0x5eed_cafe_1234_u64;
        for n in 0..5_500usize {
            let mut source = String::new();
            let count = n % 9 + 1;
            for _ in 0..count {
                state ^= state << 13;
                state ^= state >> 7;
                state ^= state << 17;
                source.push_str(atoms[(state as usize) % atoms.len()]);
                if state & 3 == 0 {
                    source.push(' ');
                }
            }
            if n % 3 == 0 {
                let mut bytes = Vec::new();
                for _ in 0..(n % 17) {
                    state ^= state << 13;
                    state ^= state >> 7;
                    state ^= state << 17;
                    bytes.push(state as u8);
                }
                bytes.extend_from_slice(b" name");
                if n % 2 == 0 {
                    bytes.extend_from_slice(&[0xf0, 0x9f, 0x99]);
                }
                let valid_end = std::str::from_utf8(&bytes)
                    .map_or_else(|error| error.valid_up_to(), |text| text.len());
                source = std::str::from_utf8(&bytes[..valid_end]).unwrap().into();
            }
            if n % 19 == 0 {
                let mut truncated = "日".as_bytes().to_vec();
                truncated.truncate(2);
                assert!(std::str::from_utf8(&truncated).is_err());
            }
            if n % 5 == 0 {
                source.push_str("// trailing 🙂\r\n");
            }
            if n % 7 == 0 {
                source.push('\u{00a0}');
            }
            if n % 11 == 0 {
                source.push('🙂');
            }
            if n % 13 == 0 {
                source.push_str("\"open");
            }
            let result = lex("generated", &source);
            if let Ok(tokens) = result {
                assert_eq!(
                    tokens.iter().filter(|t| t.kind == Kind::Eof).count(),
                    1,
                    "{source:?}"
                );
                assert_eq!(
                    tokens.last(),
                    Some(&Token {
                        kind: Kind::Eof,
                        text: String::new(),
                        span: Span {
                            file: "generated".into(),
                            start: source.len() as u64,
                            end: source.len() as u64
                        }
                    })
                );
                let mut prior_end = 0usize;
                for token in &tokens[..tokens.len() - 1] {
                    let start = token.span.start as usize;
                    let end = token.span.end as usize;
                    assert!(
                        start >= prior_end && start <= end && end <= source.len(),
                        "{source:?} {token:?}"
                    );
                    assert!(
                        source.is_char_boundary(start) && source.is_char_boundary(end),
                        "{source:?} {token:?}"
                    );
                    assert!(
                        is_trivia_gap(&source[prior_end..start]),
                        "non-trivia gap in {source:?}: {:?}",
                        &source[prior_end..start]
                    );
                    prior_end = end;
                }
                assert!(
                    is_trivia_gap(&source[prior_end..]),
                    "non-trivia tail in {source:?}"
                );
            } else {
                let error = result.unwrap_err();
                assert_eq!(error.code, "NX-E0001", "{source:?}");
                let span = error.primary_span.as_ref().expect("NX-E0001 has a span");
                let start = usize::try_from(span.start).unwrap();
                let end = usize::try_from(span.end).unwrap();
                assert!(start < end && end <= source.len(), "{source:?} {span:?}");
                assert!(
                    source.is_char_boundary(start) && source.is_char_boundary(end),
                    "{source:?} {span:?}"
                );
                let token = error
                    .unexpected_token
                    .as_ref()
                    .expect("NX-E0001 has an unexpected token");
                assert_eq!(token.kind, TokenKind::Invalid, "{source:?}");
                assert_eq!(token.span, *span, "{source:?}");
                assert_eq!(
                    token.text.as_deref(),
                    Some(&source[start..end]),
                    "{source:?}"
                );
            }
        }
    }

    #[test]
    fn arbitrary_bytes_and_truncated_utf8_never_panic() {
        let mut state = 0x5eed_u64;
        for n in 0..20_000usize {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            let len = n % 64;
            let mut bytes = Vec::with_capacity(len + 3);
            for _ in 0..len {
                state ^= state << 13;
                state ^= state >> 7;
                state ^= state << 17;
                bytes.push(state as u8);
            }
            if n % 2 == 0 {
                bytes.extend_from_slice(&[0xf0, 0x9f, 0x99]);
            }
            if let Ok(text) = std::str::from_utf8(&bytes) {
                let _ = lex("fuzz", text);
            }
        }
    }
}
