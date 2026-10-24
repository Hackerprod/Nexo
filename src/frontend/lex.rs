//! Lexer for the official B surface of F0-D15.

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
            } else if is_horizontal_space(ch) {
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
                        let escape_start = self.at;
                        self.bump();
                        match self.peek_char() {
                            Some('"' | '\\' | 'n' | 'r' | 't') => self.bump(),
                            Some('\r' | '\n') | None => break,
                            Some(_) => {
                                self.bump();
                                return Err(self.error(escape_start, self.at));
                            }
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
pub(crate) fn is_horizontal_space(ch: char) -> bool {
    matches!(
        ch,
        '\t' | '\u{b}' | '\u{c}' | ' ' | '\u{85}' | '\u{a0}' | '\u{1680}' | '\u{2000}'
            ..='\u{200a}' | '\u{2028}' | '\u{2029}' | '\u{202f}' | '\u{205f}' | '\u{3000}'
    )
}

fn is_reserved(word: &str) -> bool {
    matches!(
        word,
        "fn" | "let"
            | "var"
            | "return"
            | "if"
            | "then"
            | "else"
            | "while"
            | "true"
            | "false"
            | "with"
            | "and"
            | "or"
            | "not"
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
    )
}

fn longest_symbol(source: &str) -> Option<&'static str> {
    [
        "..<", "..=", "..", "+=", "!=", "->", "=>", "==", "<=", ">=", "(", ")", "{", "}", "[", "]",
        ",", ":", ";", "?", "+", "-", "<", ">", "=", "@", ".", "*", "/", "%",
    ]
    .into_iter()
    .find(|symbol| source.starts_with(symbol))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn published_reserved_words_are_keywords_and_modes_stay_contextual() {
        // F0-D15 §2.2: exact reserved inventory, including `then`.
        let reserved = "fn let var return if then else while true false with and or not match for break continue gen yield async await region struct enum type trait impl where dyn mod use pub extend extension comptime const self super package";
        let tokens = lex("b.nx", reserved).unwrap();
        let words: Vec<_> = tokens.into_iter().filter(|t| t.kind != Kind::Eof).collect();
        assert_eq!(words.len(), 40);
        assert!(words.iter().all(|t| t.kind == Kind::Keyword));
        let contextual = lex("b.nx", "view edit move value copy").unwrap();
        assert!(
            contextual
                .iter()
                .filter(|t| t.kind != Kind::Eof)
                .all(|t| t.kind == Kind::Name)
        );
    }

    #[test]
    fn published_symbol_inventory_uses_longest_match() {
        // F0-D15 §2.4: `..` is additionally recognized for its specific error.
        let symbols = [
            "..<", "..=", "..", "+=", "!=", "->", "=>", "==", "<=", ">=", "(", ")", "{", "}", "[",
            "]", ",", ":", ";", "?", "+", "-", "<", ">", "=", "@", ".", "*", "/", "%",
        ];
        for symbol in symbols {
            let tokens = lex("b.nx", symbol).unwrap();
            assert_eq!(tokens.len(), 2, "{symbol}");
            assert_eq!(tokens[0].kind, Kind::Symbol, "{symbol}");
            assert_eq!(tokens[0].text, symbol, "{symbol}");
        }
    }

    #[test]
    fn reserved_contextual_symbols_and_byte_spans() {
        let tokens = lex(
            "b.nx",
            "fn Main(x: edit I64) { value: move x; }\r\n// 🙂\r1..<n 2..=n 3..n",
        )
        .unwrap();
        let selected: Vec<_> = tokens
            .iter()
            .filter(|t| !matches!(t.kind, Kind::Newline | Kind::Eof))
            .map(|t| (t.kind, t.text.as_str()))
            .collect();
        assert!(selected.contains(&(Kind::Name, "edit")));
        assert!(selected.contains(&(Kind::Name, "move")));
        assert!(selected.contains(&(Kind::Name, "value")));
        assert!(selected.contains(&(Kind::Symbol, "..<")));
        assert!(selected.contains(&(Kind::Symbol, "..=")));
        assert!(selected.contains(&(Kind::Symbol, "..")));
        assert_eq!(tokens.last().unwrap().span.start, 65);
    }

    #[test]
    fn quoted_text_is_opaque_but_escapes_are_closed() {
        let source = "fn Main(){let s=\"…{x}\";}";
        let tokens = lex("b.nx", source).unwrap();
        let string = tokens.iter().find(|t| t.kind == Kind::String).unwrap();
        assert_eq!((string.span.start, string.span.end), (16, 24));
        let invalid = "fn Main(){let s=\"\\q\";}";
        let error = lex("b.nx", invalid).unwrap_err();
        assert_eq!(error.code, "NX-E0001");
        assert_eq!(
            (
                error.primary_span.as_ref().unwrap().start,
                error.primary_span.as_ref().unwrap().end
            ),
            (17, 19)
        );
        assert_eq!(
            error.unexpected_token.as_ref().unwrap().text.as_deref(),
            Some("\\q")
        );
        assert_eq!(
            (
                error.recognized_prefix.as_ref().unwrap().start,
                error.recognized_prefix.as_ref().unwrap().end
            ),
            (0, 17)
        );
        assert_eq!(error.profile_id, diagnostic::PROFILE_ID);
        assert!(error.validate("b.nx", invalid).is_ok());
        assert!(lex("b.nx", "\"\\n\\r\\t\\\"\\\\\"").is_ok());
    }

    #[test]
    fn only_published_horizontal_whitespace_is_trivia() {
        assert!(lex("b.nx", "a\u{200a}b").is_ok());
        let error = lex("b.nx", "a\u{200b}b").unwrap_err();
        assert_eq!(error.primary_span.as_ref().unwrap().start, 1);
        assert!(error.validate("b.nx", "a\u{200b}b").is_ok());
    }

    fn exact_tokens(source: &str, expected: &[(Kind, &str, u64, u64)]) {
        let tokens = lex("b.nx", source).unwrap();
        let actual = tokens
            .iter()
            .map(|token| {
                (
                    token.kind,
                    token.text.as_str(),
                    token.span.start,
                    token.span.end,
                )
            })
            .collect::<Vec<_>>();
        assert_eq!(actual, expected, "{source:?}");
    }

    fn exact_invalid(source: &str, expected: &str, start: u64, end: u64) {
        let error = lex("b.nx", source).unwrap_err();
        assert_eq!(error.code, "NX-E0001");
        assert_eq!(error.stage, crate::diagnostic::Stage::Lex);
        let primary = error.primary_span.as_ref().unwrap();
        assert_eq!((primary.start, primary.end), (start, end));
        let token = error.unexpected_token.as_ref().unwrap();
        assert_eq!(token.kind, crate::diagnostic::TokenKind::Invalid);
        assert_eq!(token.text.as_deref(), Some(expected));
        assert_eq!((token.span.start, token.span.end), (start, end));
        assert_eq!(error.profile_id, diagnostic::PROFILE_ID);
        error.validate("b.nx", source).unwrap();
    }

    #[test]
    fn adr_gap_digit_letter_boundary_is_integer_then_name() {
        exact_tokens(
            "1a",
            &[
                (Kind::Integer, "1", 0, 1),
                (Kind::Name, "a", 1, 2),
                (Kind::Eof, "", 2, 2),
            ],
        );
    }

    #[test]
    fn adr_gap_unclosed_string_stops_before_lf() {
        exact_invalid("\"a\nb\"", "\"a", 0, 2);
    }

    #[test]
    fn adr_gap_whitespace_includes_tab_and_nonbreaking_space() {
        exact_tokens(
            "a\t\u{00a0}b",
            &[
                (Kind::Name, "a", 0, 1),
                (Kind::Name, "b", 4, 5),
                (Kind::Eof, "", 5, 5),
            ],
        );
    }

    #[test]
    fn block_comment_spelling_is_tokenized_as_b_operators() {
        exact_tokens(
            "/*x*/",
            &[
                (Kind::Symbol, "/", 0, 1),
                (Kind::Symbol, "*", 1, 2),
                (Kind::Name, "x", 2, 3),
                (Kind::Symbol, "*", 3, 4),
                (Kind::Symbol, "/", 4, 5),
                (Kind::Eof, "", 5, 5),
            ],
        );
    }

    #[test]
    fn dot_without_following_digit_stays_a_symbol() {
        exact_tokens(
            "1.",
            &[
                (Kind::Integer, "1", 0, 1),
                (Kind::Symbol, ".", 1, 2),
                (Kind::Eof, "", 2, 2),
            ],
        );
        exact_tokens(
            "1.x",
            &[
                (Kind::Integer, "1", 0, 1),
                (Kind::Symbol, ".", 1, 2),
                (Kind::Name, "x", 2, 3),
                (Kind::Eof, "", 3, 3),
            ],
        );
    }

    #[test]
    fn comment_ends_at_cr_and_crlf_span_keeps_both_bytes() {
        exact_tokens(
            "a//c\r\nb",
            &[
                (Kind::Name, "a", 0, 1),
                (Kind::Newline, "\r\n", 4, 6),
                (Kind::Name, "b", 6, 7),
                (Kind::Eof, "", 7, 7),
            ],
        );
    }

    #[test]
    fn underscores_and_contextual_names_are_single_name_tokens() {
        exact_tokens("a_b", &[(Kind::Name, "a_b", 0, 3), (Kind::Eof, "", 3, 3)]);
        exact_tokens(
            "result_from",
            &[(Kind::Name, "result_from", 0, 11), (Kind::Eof, "", 11, 11)],
        );
    }

    #[test]
    fn unicode_digits_and_letters_are_rejected_as_single_utf8_tokens() {
        for (source, end) in [("²", 2), ("٣", 2), ("日", 3)] {
            exact_invalid(source, source, 0, end);
        }
    }

    #[test]
    fn complete_symbol_inventory_and_longest_match_are_tokenized() {
        let spellings = [
            "..<", "..=", "+=", "!=", "->", "=>", "==", "<=", ">=", "..", "(", ")", "{", "}", "[",
            "]", ",", ":", ";", "?", "+", "-", "<", ">", "=", "@", ".", "*", "/", "%",
        ];
        let source = spellings.join(" ");
        let mut expected = Vec::new();
        let mut at = 0usize;
        for spelling in spellings {
            let start = source[at..].find(spelling).unwrap() + at;
            expected.push((
                Kind::Symbol,
                spelling,
                start as u64,
                (start + spelling.len()) as u64,
            ));
            at = start + spelling.len();
        }
        expected.push((Kind::Eof, "", source.len() as u64, source.len() as u64));
        exact_tokens(&source, &expected);
    }

    #[test]
    fn reserved_word_classes_match_the_closed_adr_lists() {
        let reserved = "fn let var return if then else while true false with and or not match for break continue gen yield async await region struct enum type trait impl where dyn mod use pub extend extension comptime const self super package";
        let contextual = "view edit move copy value from result_from retains as in create call resume cancel drop unrestricted no_alloc no_heap_alloc no_block no_suspend no_io capacity executor local migrable layout derive";
        for word in reserved.split_whitespace() {
            exact_tokens(
                word,
                &[
                    (Kind::Keyword, word, 0, word.len() as u64),
                    (Kind::Eof, "", word.len() as u64, word.len() as u64),
                ],
            );
        }
        for word in contextual.split_whitespace() {
            exact_tokens(
                word,
                &[
                    (Kind::Name, word, 0, word.len() as u64),
                    (Kind::Eof, "", word.len() as u64, word.len() as u64),
                ],
            );
        }
    }

    #[test]
    fn incomplete_string_reports_the_full_consumed_lexeme() {
        exact_invalid("let s=\"ab", "\"ab", 6, 9);
    }

    #[test]
    fn identifiers_keywords_literals_comments_and_line_endings_preserve_spans() {
        exact_invalid("let α=0.5\r\n", "α", 4, 6);
        let source = "let _x9=0.5\r\n// comentario 🙂\nvalue=\"ok\"";
        exact_tokens(
            source,
            &[
                (Kind::Keyword, "let", 0, 3),
                (Kind::Name, "_x9", 4, 7),
                (Kind::Symbol, "=", 7, 8),
                (Kind::Decimal, "0.5", 8, 11),
                (Kind::Newline, "\r\n", 11, 13),
                (Kind::Newline, "\n", 31, 32),
                (Kind::Name, "value", 32, 37),
                (Kind::Symbol, "=", 37, 38),
                (Kind::String, "\"ok\"", 38, 42),
                (Kind::Eof, "", 42, 42),
            ],
        );
    }

    #[test]
    fn decimal_range_and_separated_dot_follow_profile_boundaries() {
        exact_tokens(
            "0.5 0..<n 0 . 5",
            &[
                (Kind::Decimal, "0.5", 0, 3),
                (Kind::Integer, "0", 4, 5),
                (Kind::Symbol, "..<", 5, 8),
                (Kind::Name, "n", 8, 9),
                (Kind::Integer, "0", 10, 11),
                (Kind::Symbol, ".", 12, 13),
                (Kind::Integer, "5", 14, 15),
                (Kind::Eof, "", 15, 15),
            ],
        );
    }

    #[test]
    fn decimal_and_single_digit_integer_boundaries_have_exact_sequences() {
        exact_tokens(
            "10.25",
            &[(Kind::Decimal, "10.25", 0, 5), (Kind::Eof, "", 5, 5)],
        );
        exact_tokens(
            "007",
            &[(Kind::Integer, "007", 0, 3), (Kind::Eof, "", 3, 3)],
        );
        exact_tokens("9", &[(Kind::Integer, "9", 0, 1), (Kind::Eof, "", 1, 1)]);
    }

    #[test]
    fn minus_is_always_a_separate_symbol_token() {
        exact_tokens(
            "-1",
            &[
                (Kind::Symbol, "-", 0, 1),
                (Kind::Integer, "1", 1, 2),
                (Kind::Eof, "", 2, 2),
            ],
        );
        exact_tokens(
            "1-2",
            &[
                (Kind::Integer, "1", 0, 1),
                (Kind::Symbol, "-", 1, 2),
                (Kind::Integer, "2", 2, 3),
                (Kind::Eof, "", 3, 3),
            ],
        );
        exact_tokens(
            "1 - 2",
            &[
                (Kind::Integer, "1", 0, 1),
                (Kind::Symbol, "-", 2, 3),
                (Kind::Integer, "2", 4, 5),
                (Kind::Eof, "", 5, 5),
            ],
        );
        exact_tokens(
            "--1",
            &[
                (Kind::Symbol, "-", 0, 1),
                (Kind::Symbol, "-", 1, 2),
                (Kind::Integer, "1", 2, 3),
                (Kind::Eof, "", 3, 3),
            ],
        );
    }

    fn exact_published_lexical_error(source: &str, start: u64, end: u64, spelling: &str) {
        let d = lex("b.nx", source).unwrap_err();
        let span = Span {
            file: "b.nx".into(),
            start,
            end,
        };
        assert_eq!(d.schema_version, diagnostic::SCHEMA_VERSION);
        assert_eq!(d.diagnostic_namespace, Namespace::NexoProposedV1);
        assert_eq!(d.profile_id, diagnostic::PROFILE_ID);
        assert_eq!(d.code, "NX-E0001");
        assert_eq!(d.subcause, None);
        assert_eq!(d.rule, "NX-E0001");
        assert_eq!(d.explanation_id.as_deref(), Some("NX-E0001"));
        assert_eq!(d.severity, Severity::Error);
        assert_eq!(d.stage, Stage::Lex);
        assert_eq!(d.operation, Operation::SyntaxLex);
        assert_eq!(d.primary_span, Some(span.clone()));
        assert_eq!(
            d.unexpected_token,
            Some(DiagnosticToken {
                kind: TokenKind::Invalid,
                text: Some(spelling.into()),
                span,
            })
        );
        assert_eq!(d.expected_tokens, ["valid_token"]);
        assert_eq!(
            d.expected_tokens_kind,
            Some(ExpectedTokensKind::LexicalCategory)
        );
        assert_eq!(d.parser_context.as_deref(), Some("lexical_token"));
        assert_eq!(
            d.recognized_prefix,
            Some(Span {
                file: "b.nx".into(),
                start: 0,
                end: start
            })
        );
        assert!(
            d.related_spans.is_empty()
                && d.cause_chain.is_empty()
                && d.repair.is_empty()
                && d.data.is_empty()
        );
        d.validate("b.nx", source).unwrap();
    }

    #[test]
    fn emitted_nx_e0001_has_literal_lexical_contract_and_byte_spans() {
        exact_published_lexical_error("fn Main(){let x=1£2;}", 17, 19, "£");
    }

    #[test]
    fn migrated_k01_lexical_error_has_literal_fields_and_byte_spans() {
        // F0-D15 §6.3 K01, ASCII prefix and two-byte invalid token.
        exact_published_lexical_error("fn Main() { £ }", 12, 14, "£");
    }

    #[test]
    fn lx01_to_lx05_follow_b_lexical_and_parse_boundaries() {
        use crate::frontend::parse::parse;
        for (bad, stage, context) in [
            ('/', Stage::Parse, "unsupported_binary_operator"),
            ('%', Stage::Parse, "unsupported_binary_operator"),
            ('!', Stage::Lex, "lexical_token"),
            ('&', Stage::Lex, "lexical_token"),
            ('|', Stage::Lex, "lexical_token"),
        ] {
            let source = format!("fn Main(){{let x=1{bad}2;}}");
            let d = parse("b.nx", &source).unwrap_err();
            assert_eq!(d.stage, stage);
            assert_eq!(d.parser_context.as_deref(), Some(context));
            assert_eq!(
                d.primary_span.as_ref().map(|s| (s.start, s.end)),
                Some((17, 18))
            );
            assert_eq!(
                d.unexpected_token.as_ref().and_then(|t| t.text.as_deref()),
                Some(&bad.to_string()[..])
            );
            assert_eq!(
                d.code,
                if stage == Stage::Lex {
                    "NX-E0001"
                } else {
                    "NX-E0002"
                }
            );
            d.validate("b.nx", &source).unwrap();
            let good = "fn Main(){let x=1+2;}".to_string();
            assert!(parse("b.nx", &good).is_ok());
        }
    }

    #[test]
    fn trivia_gap_requires_line_comment_to_end_at_line_or_eof() {
        exact_tokens(
            "a // comment",
            &[(Kind::Name, "a", 0, 1), (Kind::Eof, "", 12, 12)],
        );
        exact_tokens(
            "a // comment\nb",
            &[
                (Kind::Name, "a", 0, 1),
                (Kind::Newline, "\n", 12, 13),
                (Kind::Name, "b", 13, 14),
                (Kind::Eof, "", 14, 14),
            ],
        );
        exact_tokens(
            "a // comment\rb",
            &[
                (Kind::Name, "a", 0, 1),
                (Kind::Newline, "\r", 12, 13),
                (Kind::Name, "b", 13, 14),
                (Kind::Eof, "", 14, 14),
            ],
        );
    }

    #[test]
    fn arbitrary_bytes_and_truncated_utf8_never_panic() {
        let mut state = 0x5eed_u64;
        for n in 0..20_000usize {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            let mut bytes = Vec::with_capacity(n % 64 + 3);
            for _ in 0..(n % 64) {
                state ^= state << 13;
                state ^= state >> 7;
                state ^= state << 17;
                bytes.push(state as u8);
            }
            if n % 2 == 0 {
                bytes.extend_from_slice(&[0xf0, 0x9f, 0x99]);
            }
            if let Ok(source) = std::str::from_utf8(&bytes) {
                match lex("b.nx", source) {
                    Ok(tokens) => assert_eq!(tokens.last().unwrap().kind, Kind::Eof),
                    Err(d) => d.validate("b.nx", source).unwrap(),
                }
            }
        }
    }

    #[test]
    fn generated_inputs_preserve_token_span_and_eof_invariants() {
        fn trivia_gap(gap: &str) -> bool {
            let mut at = 0;
            while at < gap.len() {
                let rest = &gap[at..];
                if rest.starts_with("//") {
                    at += 2;
                    while at < gap.len() && !matches!(gap.as_bytes()[at], b'\r' | b'\n') {
                        at += gap[at..].chars().next().unwrap().len_utf8();
                    }
                    continue;
                }
                let ch = rest.chars().next().unwrap();
                // F0-D15 §2.1 closed trivia alphabet, independently transcribed.
                if !matches!(
                    ch,
                    '\r' | '\n'
                        | '\t'
                        | '\u{000b}'
                        | '\u{000c}'
                        | ' '
                        | '\u{0085}'
                        | '\u{00a0}'
                        | '\u{1680}'
                        | '\u{2000}'
                        ..='\u{200a}'
                            | '\u{2028}'
                            | '\u{2029}'
                            | '\u{202f}'
                            | '\u{205f}'
                            | '\u{3000}'
                ) {
                    return false;
                }
                at += ch.len_utf8();
            }
            true
        }
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
            "..<",
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
            for _ in 0..(n % 9 + 1) {
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
                let valid_end =
                    std::str::from_utf8(&bytes).map_or_else(|e| e.valid_up_to(), |s| s.len());
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
            match lex("generated.nx", &source) {
                Ok(tokens) => {
                    assert_eq!(
                        tokens.iter().filter(|t| t.kind == Kind::Eof).count(),
                        1,
                        "{source:?}"
                    );
                    let eof = tokens.last().unwrap();
                    assert_eq!(
                        (eof.kind, eof.span.start, eof.span.end),
                        (Kind::Eof, source.len() as u64, source.len() as u64)
                    );
                    let mut prior_end = 0;
                    for token in &tokens[..tokens.len() - 1] {
                        let start = token.span.start as usize;
                        let end = token.span.end as usize;
                        assert!(
                            start >= prior_end && start <= end && end <= source.len(),
                            "{source:?}: {token:?}"
                        );
                        assert!(source.is_char_boundary(start) && source.is_char_boundary(end));
                        assert!(
                            trivia_gap(&source[prior_end..start]),
                            "gap {:?}",
                            &source[prior_end..start]
                        );
                        assert_eq!(token.text, source[start..end]);
                        prior_end = end;
                    }
                    assert!(
                        trivia_gap(&source[prior_end..]),
                        "tail {:?}",
                        &source[prior_end..]
                    );
                }
                Err(d) => {
                    assert_eq!(d.code, "NX-E0001");
                    d.validate("generated.nx", &source).unwrap();
                    let span = d.primary_span.as_ref().unwrap();
                    let start = span.start as usize;
                    let end = span.end as usize;
                    assert!(start < end && end <= source.len());
                    assert!(source.is_char_boundary(start) && source.is_char_boundary(end));
                    assert_eq!(
                        d.unexpected_token.as_ref().unwrap().text.as_deref(),
                        Some(&source[start..end])
                    );
                }
            }
        }
    }
}
