//! Independent reference model for A2 §12 and the I11a surface in D15.
//! This test module deliberately does not import the production mock parser or
//! the interpreter's internal machine representation.

use std::{
    collections::{BTreeMap, VecDeque},
    path::PathBuf,
};

use nexo::interp::{
    MockEnvironment, MockEnvironmentError, Outcome, ReducedExecutionProfile, run_scalar,
    run_with_limits,
};

const EMPTY_ENV: &[u8] = br#"{"schema_version":1,"mock_read_records":[],"mock_write_i64":[],"mock_request":[],"mock_reply":[]}"#;
const BATCH_SOURCE: &str = "fn Main(){var i=0;while i<250{let rows=MockReadRecords(MockOpen(i));MockWriteI64(i);MockRequest();MockReply(i+1000);i=i+1;}}";
const DOCUMENT_MUTATION_SEEDS: [(&str, &[u8]); 7] = [
    ("empty", EMPTY_ENV),
    (
        "ex01_json",
        br#"{"schema_version":1,"mock_read_records":[{"ok":[{"ok":"12"},{"ok":"8"},{"ok":"5"}]}],"mock_write_i64":[{"ok":null}],"mock_request":[],"mock_reply":[]}"#,
    ),
    (
        "read_minimum_and_err",
        br#"{"schema_version":1,"mock_read_records":[{"ok":[{"err":"-1"},{"ok":"0"}]},{"err":"7"}],"mock_write_i64":[],"mock_request":[],"mock_reply":[]}"#,
    ),
    (
        "write_minimum",
        br#"{"schema_version":1,"mock_read_records":[],"mock_write_i64":[{"ok":null},{"err":"7"}],"mock_request":[],"mock_reply":[]}"#,
    ),
    (
        "request_minimum",
        br#"{"schema_version":1,"mock_read_records":[],"mock_write_i64":[],"mock_request":[{"ok":"-5"},{"err":"6"}],"mock_reply":[]}"#,
    ),
    (
        "reply_minimum",
        br#"{"schema_version":1,"mock_read_records":[],"mock_write_i64":[],"mock_request":[],"mock_reply":[{"ok":null},{"err":"7"}]}"#,
    ),
    (
        "four_queue_mixed",
        br#"{"schema_version":1,"mock_read_records":[{"ok":[{"ok":"1"},{"err":"2"}]},{"err":"3"}],"mock_write_i64":[{"ok":null},{"err":"4"}],"mock_request":[{"ok":"5"},{"err":"6"}],"mock_reply":[{"ok":null},{"err":"7"}]}"#,
    ),
];

const STRUCTURAL_SEQUENCES: &[(&str, &str)] = &[
    ("unicode_escape_valid", r#"\u0041"#),
    ("unicode_escape_valid", r#"\u00aF"#),
    ("unicode_escape_valid", r#"\uD800\uDC00"#),
    ("unicode_escape_invalid", r#"\u"#),
    ("unicode_escape_invalid", r#"\u0"#),
    ("unicode_escape_invalid", r#"\u00G0"#),
    ("unicode_escape_invalid", r#"\uD800"#),
    ("unicode_escape_invalid", r#"\uDC00"#),
    ("unicode_escape_invalid", r#"\uD800\u0041"#),
    ("unicode_escape_invalid", r#"\uDC00\uD800"#),
    ("exponent", "e"),
    ("exponent", "E"),
    ("exponent", "e+"),
    ("exponent", "E-"),
    ("exponent", "e0"),
    ("exponent", "E0"),
    ("exponent", "e+0"),
    ("exponent", "E+0"),
    ("exponent", "e-0"),
    ("exponent", "E-0"),
    ("fraction", "."),
    ("fraction", ".0"),
    ("fraction", ".5"),
    ("malformed_number", "1."),
    ("malformed_number", "1e"),
    ("malformed_number", "1e+"),
    ("malformed_number", "01"),
    ("malformed_number", "-"),
    ("valid_number_noncanonical", "-0"),
    ("container_sequence", "[]"),
    ("container_sequence", "{}"),
    ("container_sequence", "[null]"),
    ("container_sequence", "{\"x\":null}"),
    ("literal_sequence", "null"),
    ("literal_sequence", "true"),
    ("literal_sequence", "false"),
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Reject {
    TooLarge,
    InvalidJson,
    InvalidSchema,
}

impl Reject {
    fn controller_reason(self) -> &'static str {
        match self {
            Self::TooLarge => "mock_env_too_large",
            Self::InvalidJson => "invalid_mock_json",
            Self::InvalidSchema => "invalid_mock_schema",
        }
    }
}

fn actual_rejection(error: MockEnvironmentError) -> Reject {
    match error {
        MockEnvironmentError::TooLarge => Reject::TooLarge,
        MockEnvironmentError::InvalidJson => Reject::InvalidJson,
        MockEnvironmentError::InvalidSchema => Reject::InvalidSchema,
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum Json {
    Null,
    Bool,
    Number(String),
    String(String),
    Array(Vec<Json>),
    Object(Vec<(String, Json)>),
}

struct ReferenceLexer<'a> {
    characters: std::iter::Peekable<std::str::Chars<'a>>,
}

/// Oracle A finishes a Unicode-scalar lexing pass, then recursively reduces
/// the token stream. Production instead descends over bytes as it scans.
impl<'a> ReferenceLexer<'a> {
    fn tokenize(text: &'a str) -> Result<Vec<JsonToken>, Reject> {
        let mut lexer = Self {
            characters: text.chars().peekable(),
        };
        let mut tokens = Vec::new();
        loop {
            lexer.whitespace();
            let Some(character) = lexer.characters.peek().copied() else {
                return Ok(tokens);
            };
            match character {
                '{' => tokens.push(lexer.punctuation('{', JsonToken::ObjectStart)),
                '}' => tokens.push(lexer.punctuation('}', JsonToken::ObjectEnd)),
                '[' => tokens.push(lexer.punctuation('[', JsonToken::ArrayStart)),
                ']' => tokens.push(lexer.punctuation(']', JsonToken::ArrayEnd)),
                ':' => tokens.push(lexer.punctuation(':', JsonToken::Colon)),
                ',' => tokens.push(lexer.punctuation(',', JsonToken::Comma)),
                '"' => tokens.push(JsonToken::Value(Json::String(lexer.string()?))),
                't' => {
                    lexer.literal("true")?;
                    tokens.push(JsonToken::Value(Json::Bool));
                }
                'f' => {
                    lexer.literal("false")?;
                    tokens.push(JsonToken::Value(Json::Bool));
                }
                'n' => {
                    lexer.literal("null")?;
                    tokens.push(JsonToken::Value(Json::Null));
                }
                '-' | '0'..='9' => {
                    tokens.push(JsonToken::Value(Json::Number(lexer.number()?)));
                }
                _ => return Err(Reject::InvalidJson),
            }
        }
    }

    fn punctuation(&mut self, character: char, token: JsonToken) -> JsonToken {
        let consumed = self.characters.next();
        debug_assert_eq!(consumed, Some(character));
        token
    }

    fn string(&mut self) -> Result<String, Reject> {
        if self.characters.next() != Some('"') {
            return Err(Reject::InvalidJson);
        }
        let mut decoded = String::new();
        loop {
            match self.characters.next().ok_or(Reject::InvalidJson)? {
                '"' => return Ok(decoded),
                character if (character as u32) < 0x20 => return Err(Reject::InvalidJson),
                '\\' => match self.characters.next().ok_or(Reject::InvalidJson)? {
                    '"' => decoded.push('"'),
                    '\\' => decoded.push('\\'),
                    '/' => decoded.push('/'),
                    'b' => decoded.push('\u{8}'),
                    'f' => decoded.push('\u{c}'),
                    'n' => decoded.push('\n'),
                    'r' => decoded.push('\r'),
                    't' => decoded.push('\t'),
                    'u' => {
                        let first = self.hex_quad()?;
                        let scalar = if (0xd800..=0xdbff).contains(&first) {
                            if self.characters.next() != Some('\\')
                                || self.characters.next() != Some('u')
                            {
                                return Err(Reject::InvalidJson);
                            }
                            let second = self.hex_quad()?;
                            if !(0xdc00..=0xdfff).contains(&second) {
                                return Err(Reject::InvalidJson);
                            }
                            0x10000 + ((first - 0xd800) << 10) + (second - 0xdc00)
                        } else {
                            first
                        };
                        decoded.push(char::from_u32(scalar).ok_or(Reject::InvalidJson)?);
                    }
                    _ => return Err(Reject::InvalidJson),
                },
                other => decoded.push(other),
            }
        }
    }

    fn hex_quad(&mut self) -> Result<u32, Reject> {
        let mut value = 0u32;
        for _ in 0..4 {
            let character = self.characters.next().ok_or(Reject::InvalidJson)?;
            let digit = match character {
                '0'..='9' => character as u32 - '0' as u32,
                'a'..='f' => character as u32 - 'a' as u32 + 10,
                'A'..='F' => character as u32 - 'A' as u32 + 10,
                _ => return Err(Reject::InvalidJson),
            };
            value = value * 16 + digit;
        }
        Ok(value)
    }

    fn number(&mut self) -> Result<String, Reject> {
        let mut spelling = String::new();
        if self.take('-') {
            spelling.push('-');
        }
        match self.characters.peek().copied() {
            Some('0') => {
                spelling.push('0');
                self.characters.next();
                if self.characters.peek().is_some_and(char::is_ascii_digit) {
                    return Err(Reject::InvalidJson);
                }
            }
            Some('1'..='9') => {
                while self.characters.peek().is_some_and(char::is_ascii_digit) {
                    spelling.push(self.characters.next().unwrap());
                }
            }
            _ => return Err(Reject::InvalidJson),
        }
        if self.take('.') {
            spelling.push('.');
            if !self.characters.peek().is_some_and(char::is_ascii_digit) {
                return Err(Reject::InvalidJson);
            }
            while self.characters.peek().is_some_and(char::is_ascii_digit) {
                spelling.push(self.characters.next().unwrap());
            }
        }
        if self
            .characters
            .peek()
            .is_some_and(|character| matches!(character, 'e' | 'E'))
        {
            spelling.push(self.characters.next().unwrap());
            if self
                .characters
                .peek()
                .is_some_and(|character| matches!(character, '+' | '-'))
            {
                spelling.push(self.characters.next().unwrap());
            }
            if !self.characters.peek().is_some_and(char::is_ascii_digit) {
                return Err(Reject::InvalidJson);
            }
            while self.characters.peek().is_some_and(char::is_ascii_digit) {
                spelling.push(self.characters.next().unwrap());
            }
        }
        Ok(spelling)
    }

    fn literal(&mut self, text: &str) -> Result<(), Reject> {
        for expected in text.chars() {
            if self.characters.next() != Some(expected) {
                return Err(Reject::InvalidJson);
            }
        }
        Ok(())
    }

    fn whitespace(&mut self) {
        while self
            .characters
            .peek()
            .is_some_and(|character| matches!(character, ' ' | '\t' | '\r' | '\n'))
        {
            self.characters.next();
        }
    }

    fn take(&mut self, expected: char) -> bool {
        if self.characters.peek() == Some(&expected) {
            self.characters.next();
            true
        } else {
            false
        }
    }
}

struct ReferenceParser<'a> {
    tokens: &'a [JsonToken],
    cursor: usize,
    container_depth: usize,
}

impl<'a> ReferenceParser<'a> {
    fn parse(input: &[u8]) -> Result<Json, Reject> {
        let text = std::str::from_utf8(input).map_err(|_| Reject::InvalidJson)?;
        let tokens = ReferenceLexer::tokenize(text)?;
        let mut parser = ReferenceParser {
            tokens: &tokens,
            cursor: 0,
            container_depth: 0,
        };
        let root = parser.value()?;
        if parser.cursor != tokens.len() {
            return Err(Reject::InvalidJson);
        }
        Ok(root)
    }

    fn value(&mut self) -> Result<Json, Reject> {
        match self.tokens.get(self.cursor) {
            Some(JsonToken::ObjectStart) => self.object(),
            Some(JsonToken::ArrayStart) => self.array(),
            Some(JsonToken::Value(value)) => {
                self.cursor += 1;
                Ok(value.clone())
            }
            _ => Err(Reject::InvalidJson),
        }
    }

    fn object(&mut self) -> Result<Json, Reject> {
        self.enter()?;
        self.cursor += 1;
        let mut fields = Vec::new();
        if self.take(&JsonToken::ObjectEnd) {
            self.leave();
            return Ok(Json::Object(fields));
        }
        loop {
            let key = match self.tokens.get(self.cursor) {
                Some(JsonToken::Value(Json::String(key))) => key.clone(),
                _ => return Err(Reject::InvalidJson),
            };
            self.cursor += 1;
            self.expect(&JsonToken::Colon)?;
            let value = self.value()?;
            fields.push((key, value));
            if self.take(&JsonToken::ObjectEnd) {
                self.leave();
                return Ok(Json::Object(fields));
            }
            self.expect(&JsonToken::Comma)?;
        }
    }

    fn array(&mut self) -> Result<Json, Reject> {
        self.enter()?;
        self.cursor += 1;
        let mut values = Vec::new();
        if self.take(&JsonToken::ArrayEnd) {
            self.leave();
            return Ok(Json::Array(values));
        }
        loop {
            values.push(self.value()?);
            if self.take(&JsonToken::ArrayEnd) {
                self.leave();
                return Ok(Json::Array(values));
            }
            self.expect(&JsonToken::Comma)?;
        }
    }

    fn enter(&mut self) -> Result<(), Reject> {
        if self.container_depth == 16 {
            return Err(Reject::InvalidJson);
        }
        self.container_depth += 1;
        Ok(())
    }

    fn leave(&mut self) {
        self.container_depth -= 1;
    }

    fn take(&mut self, expected: &JsonToken) -> bool {
        if self.tokens.get(self.cursor) == Some(expected) {
            self.cursor += 1;
            true
        } else {
            false
        }
    }

    fn expect(&mut self, expected: &JsonToken) -> Result<(), Reject> {
        if self.take(expected) {
            Ok(())
        } else {
            Err(Reject::InvalidJson)
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum JsonToken {
    ObjectStart,
    ObjectEnd,
    ArrayStart,
    ArrayEnd,
    Colon,
    Comma,
    Value(Json),
}

/// Reference B lexes the complete byte sequence first, then uses an explicit
/// container-state stack. It does not share the recursive descent of A or the
/// production parser.
fn token_reference_parse(input: &[u8]) -> Result<Json, Reject> {
    let text = std::str::from_utf8(input).map_err(|_| Reject::InvalidJson)?;
    let tokens = tokenize_json(text)?;
    let mut stack: Vec<JsonFrame> = Vec::new();
    let mut root = None;
    let mut cursor = 0;

    while cursor < tokens.len() || !stack.is_empty() {
        if stack.is_empty() {
            if root.is_some() || cursor == tokens.len() {
                return Err(Reject::InvalidJson);
            }
            begin_json_value(&tokens, &mut cursor, &mut stack, &mut root)?;
            continue;
        }

        let frame = stack.last_mut().expect("nonempty stack");
        match (&frame.kind, frame.state) {
            (JsonFrameKind::Array(_), JsonFrameState::FirstValueOrEnd) => {
                if tokens.get(cursor) == Some(&JsonToken::ArrayEnd) {
                    cursor += 1;
                    finish_json_frame(&mut stack, &mut root)?;
                } else {
                    frame.state = JsonFrameState::Delimiter;
                    begin_json_value(&tokens, &mut cursor, &mut stack, &mut root)?;
                }
            }
            (JsonFrameKind::Array(_), JsonFrameState::Value) => {
                frame.state = JsonFrameState::Delimiter;
                begin_json_value(&tokens, &mut cursor, &mut stack, &mut root)?;
            }
            (JsonFrameKind::Array(_), JsonFrameState::Delimiter) => match tokens.get(cursor) {
                Some(JsonToken::Comma) => {
                    cursor += 1;
                    frame.state = JsonFrameState::Value;
                }
                Some(JsonToken::ArrayEnd) => {
                    cursor += 1;
                    finish_json_frame(&mut stack, &mut root)?;
                }
                _ => return Err(Reject::InvalidJson),
            },
            (JsonFrameKind::Object(_), JsonFrameState::FirstKeyOrEnd) => {
                if tokens.get(cursor) == Some(&JsonToken::ObjectEnd) {
                    cursor += 1;
                    finish_json_frame(&mut stack, &mut root)?;
                } else {
                    take_json_key(&tokens, &mut cursor, frame)?;
                }
            }
            (JsonFrameKind::Object(_), JsonFrameState::Key) => {
                take_json_key(&tokens, &mut cursor, frame)?;
            }
            (JsonFrameKind::Object(_), JsonFrameState::Colon) => {
                if tokens.get(cursor) != Some(&JsonToken::Colon) {
                    return Err(Reject::InvalidJson);
                }
                cursor += 1;
                frame.state = JsonFrameState::Value;
            }
            (JsonFrameKind::Object(_), JsonFrameState::Value) => {
                frame.state = JsonFrameState::Delimiter;
                begin_json_value(&tokens, &mut cursor, &mut stack, &mut root)?;
            }
            (JsonFrameKind::Object(_), JsonFrameState::Delimiter) => match tokens.get(cursor) {
                Some(JsonToken::Comma) => {
                    cursor += 1;
                    frame.state = JsonFrameState::Key;
                }
                Some(JsonToken::ObjectEnd) => {
                    cursor += 1;
                    finish_json_frame(&mut stack, &mut root)?;
                }
                _ => return Err(Reject::InvalidJson),
            },
            _ => return Err(Reject::InvalidJson),
        }
    }

    root.ok_or(Reject::InvalidJson)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum JsonFrameState {
    FirstValueOrEnd,
    FirstKeyOrEnd,
    Key,
    Colon,
    Value,
    Delimiter,
}

#[derive(Debug)]
enum JsonFrameKind {
    Array(Vec<Json>),
    Object(Vec<(String, Json)>),
}

#[derive(Debug)]
struct JsonFrame {
    kind: JsonFrameKind,
    state: JsonFrameState,
    key: Option<String>,
}

fn take_json_key(
    tokens: &[JsonToken],
    cursor: &mut usize,
    frame: &mut JsonFrame,
) -> Result<(), Reject> {
    let Some(JsonToken::Value(Json::String(key))) = tokens.get(*cursor) else {
        return Err(Reject::InvalidJson);
    };
    frame.key = Some(key.clone());
    frame.state = JsonFrameState::Colon;
    *cursor += 1;
    Ok(())
}

fn begin_json_value(
    tokens: &[JsonToken],
    cursor: &mut usize,
    stack: &mut Vec<JsonFrame>,
    root: &mut Option<Json>,
) -> Result<(), Reject> {
    let token = tokens.get(*cursor).ok_or(Reject::InvalidJson)?;
    *cursor += 1;
    match token {
        JsonToken::ObjectStart => stack.push(JsonFrame {
            kind: JsonFrameKind::Object(Vec::new()),
            state: JsonFrameState::FirstKeyOrEnd,
            key: None,
        }),
        JsonToken::ArrayStart => stack.push(JsonFrame {
            kind: JsonFrameKind::Array(Vec::new()),
            state: JsonFrameState::FirstValueOrEnd,
            key: None,
        }),
        JsonToken::Value(value) => attach_json_value(value.clone(), stack, root)?,
        _ => return Err(Reject::InvalidJson),
    }
    Ok(())
}

fn finish_json_frame(stack: &mut Vec<JsonFrame>, root: &mut Option<Json>) -> Result<(), Reject> {
    let frame = stack.pop().ok_or(Reject::InvalidJson)?;
    let value = match frame.kind {
        JsonFrameKind::Array(items) => Json::Array(items),
        JsonFrameKind::Object(fields) => Json::Object(fields),
    };
    attach_json_value(value, stack, root)
}

fn attach_json_value(
    value: Json,
    stack: &mut [JsonFrame],
    root: &mut Option<Json>,
) -> Result<(), Reject> {
    if let Some(frame) = stack.last_mut() {
        match &mut frame.kind {
            JsonFrameKind::Array(items) if frame.state == JsonFrameState::Delimiter => {
                items.push(value);
            }
            JsonFrameKind::Object(fields) if frame.state == JsonFrameState::Delimiter => {
                let key = frame.key.take().ok_or(Reject::InvalidJson)?;
                fields.push((key, value));
            }
            _ => return Err(Reject::InvalidJson),
        }
    } else if root.replace(value).is_some() {
        return Err(Reject::InvalidJson);
    }
    Ok(())
}

fn tokenize_json(text: &str) -> Result<Vec<JsonToken>, Reject> {
    let bytes = text.as_bytes();
    let mut tokens = Vec::new();
    let mut cursor = 0;
    let mut depth = 0usize;
    while cursor < bytes.len() {
        match bytes[cursor] {
            b' ' | b'\t' | b'\r' | b'\n' => cursor += 1,
            b'{' => {
                depth += 1;
                if depth > 16 {
                    return Err(Reject::InvalidJson);
                }
                tokens.push(JsonToken::ObjectStart);
                cursor += 1;
            }
            b'[' => {
                depth += 1;
                if depth > 16 {
                    return Err(Reject::InvalidJson);
                }
                tokens.push(JsonToken::ArrayStart);
                cursor += 1;
            }
            b'}' => {
                depth = depth.saturating_sub(1);
                tokens.push(JsonToken::ObjectEnd);
                cursor += 1;
            }
            b']' => {
                depth = depth.saturating_sub(1);
                tokens.push(JsonToken::ArrayEnd);
                cursor += 1;
            }
            b':' => {
                tokens.push(JsonToken::Colon);
                cursor += 1;
            }
            b',' => {
                tokens.push(JsonToken::Comma);
                cursor += 1;
            }
            b'"' => {
                let (string, end) = token_string(text, cursor)?;
                tokens.push(JsonToken::Value(Json::String(string)));
                cursor = end;
            }
            b't' => {
                cursor = token_literal(bytes, cursor, b"true")?;
                tokens.push(JsonToken::Value(Json::Bool));
            }
            b'f' => {
                cursor = token_literal(bytes, cursor, b"false")?;
                tokens.push(JsonToken::Value(Json::Bool));
            }
            b'n' => {
                cursor = token_literal(bytes, cursor, b"null")?;
                tokens.push(JsonToken::Value(Json::Null));
            }
            b'-' | b'0'..=b'9' => {
                let end = token_number(bytes, cursor)?;
                tokens.push(JsonToken::Value(Json::Number(text[cursor..end].to_owned())));
                cursor = end;
            }
            _ => return Err(Reject::InvalidJson),
        }
    }
    Ok(tokens)
}

fn token_literal(input: &[u8], start: usize, literal: &[u8]) -> Result<usize, Reject> {
    if input.get(start..start + literal.len()) == Some(literal) {
        Ok(start + literal.len())
    } else {
        Err(Reject::InvalidJson)
    }
}

fn token_number(input: &[u8], mut cursor: usize) -> Result<usize, Reject> {
    let start = cursor;
    if input.get(cursor) == Some(&b'-') {
        cursor += 1;
    }
    match input.get(cursor) {
        Some(b'0') => {
            cursor += 1;
            if input.get(cursor).is_some_and(u8::is_ascii_digit) {
                return Err(Reject::InvalidJson);
            }
        }
        Some(b'1'..=b'9') => {
            cursor += 1;
            while input.get(cursor).is_some_and(u8::is_ascii_digit) {
                cursor += 1;
            }
        }
        _ => return Err(Reject::InvalidJson),
    }
    if input.get(cursor) == Some(&b'.') {
        cursor += 1;
        if !input.get(cursor).is_some_and(u8::is_ascii_digit) {
            return Err(Reject::InvalidJson);
        }
        while input.get(cursor).is_some_and(u8::is_ascii_digit) {
            cursor += 1;
        }
    }
    if input
        .get(cursor)
        .is_some_and(|byte| matches!(byte, b'e' | b'E'))
    {
        cursor += 1;
        if input
            .get(cursor)
            .is_some_and(|byte| matches!(byte, b'+' | b'-'))
        {
            cursor += 1;
        }
        if !input.get(cursor).is_some_and(u8::is_ascii_digit) {
            return Err(Reject::InvalidJson);
        }
        while input.get(cursor).is_some_and(u8::is_ascii_digit) {
            cursor += 1;
        }
    }
    if cursor == start {
        Err(Reject::InvalidJson)
    } else {
        Ok(cursor)
    }
}

fn token_string(text: &str, start: usize) -> Result<(String, usize), Reject> {
    let bytes = text.as_bytes();
    let mut cursor = start + 1;
    let content_start = cursor;
    let mut decoded = String::new();
    let mut segment = cursor;
    loop {
        match bytes.get(cursor).copied() {
            Some(b'"') => {
                decoded.push_str(&text[segment..cursor]);
                return Ok((decoded, cursor + 1));
            }
            Some(byte @ 0..=0x1f) => {
                let _ = byte;
                return Err(Reject::InvalidJson);
            }
            Some(b'\\') => {
                decoded.push_str(&text[segment..cursor]);
                cursor += 1;
                match bytes.get(cursor).copied().ok_or(Reject::InvalidJson)? {
                    b'"' => decoded.push('"'),
                    b'\\' => decoded.push('\\'),
                    b'/' => decoded.push('/'),
                    b'b' => decoded.push('\u{8}'),
                    b'f' => decoded.push('\u{c}'),
                    b'n' => decoded.push('\n'),
                    b'r' => decoded.push('\r'),
                    b't' => decoded.push('\t'),
                    b'u' => {
                        let (first, after_first) = token_hex_quad(bytes, cursor + 1)?;
                        cursor = after_first;
                        let scalar = if (0xd800..=0xdbff).contains(&first) {
                            if bytes.get(cursor..cursor + 2) != Some(b"\\u") {
                                return Err(Reject::InvalidJson);
                            }
                            let (second, after_second) = token_hex_quad(bytes, cursor + 2)?;
                            if !(0xdc00..=0xdfff).contains(&second) {
                                return Err(Reject::InvalidJson);
                            }
                            cursor = after_second;
                            0x10000 + ((first - 0xd800) << 10) + second - 0xdc00
                        } else {
                            if (0xdc00..=0xdfff).contains(&first) {
                                return Err(Reject::InvalidJson);
                            }
                            first
                        };
                        decoded.push(char::from_u32(scalar).ok_or(Reject::InvalidJson)?);
                        segment = cursor;
                        continue;
                    }
                    _ => return Err(Reject::InvalidJson),
                }
                cursor += 1;
                segment = cursor;
            }
            Some(_) => cursor += 1,
            None => return Err(Reject::InvalidJson),
        }
        if cursor < content_start {
            return Err(Reject::InvalidJson);
        }
    }
}

fn token_hex_quad(input: &[u8], start: usize) -> Result<(u32, usize), Reject> {
    let mut value = 0u32;
    for offset in 0..4 {
        let digit = match input
            .get(start + offset)
            .copied()
            .ok_or(Reject::InvalidJson)?
        {
            b'0'..=b'9' => u32::from(input[start + offset] - b'0'),
            b'a'..=b'f' => u32::from(input[start + offset] - b'a') + 10,
            b'A'..=b'F' => u32::from(input[start + offset] - b'A') + 10,
            _ => return Err(Reject::InvalidJson),
        };
        value = value * 16 + digit;
    }
    Ok((value, start + 4))
}

type Reply<T> = Result<T, i64>;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
struct ReferenceEnvironment {
    reads: VecDeque<Reply<Vec<Reply<i64>>>>,
    writes: VecDeque<Reply<()>>,
    requests: VecDeque<Reply<i64>>,
    replies: VecDeque<Reply<()>>,
}

fn reference_environment(input: &[u8]) -> Result<ReferenceEnvironment, Reject> {
    if input.len() > 1_048_576 {
        return Err(Reject::TooLarge);
    }
    let root = ReferenceParser::parse(input)?;
    reference_environment_from_json(root)
}

fn token_reference_environment(input: &[u8]) -> Result<ReferenceEnvironment, Reject> {
    if input.len() > 1_048_576 {
        return Err(Reject::TooLarge);
    }
    let root = token_reference_parse(input)?;
    token_environment_from_json(root)
}

fn reference_environment_from_json(root: Json) -> Result<ReferenceEnvironment, Reject> {
    let Json::Object(fields) = root else {
        return Err(Reject::InvalidSchema);
    };
    const REQUIRED: [&str; 5] = [
        "schema_version",
        "mock_read_records",
        "mock_write_i64",
        "mock_request",
        "mock_reply",
    ];
    if fields.len() != REQUIRED.len()
        || fields
            .iter()
            .any(|(key, _)| !REQUIRED.contains(&key.as_str()))
    {
        return Err(Reject::InvalidSchema);
    }
    let field = |name: &str| {
        fields
            .iter()
            .find(|(key, _)| key == name)
            .map(|(_, value)| value)
            .ok_or(Reject::InvalidSchema)
    };
    if !matches!(field("schema_version")?, Json::Number(version) if version == "1") {
        return Err(Reject::InvalidSchema);
    }
    Ok(ReferenceEnvironment {
        reads: read_queue(field("mock_read_records")?)?,
        writes: unit_queue(field("mock_write_i64")?)?,
        requests: scalar_queue(field("mock_request")?)?,
        replies: unit_queue(field("mock_reply")?)?,
    })
}

/// Oracle B validates objects by first materializing sorted maps and rejecting
/// collisions, then walks each queue with its own typed conversion functions.
/// It deliberately does not share Oracle A's field-list/cardinality traversal.
fn token_environment_from_json(root: Json) -> Result<ReferenceEnvironment, Reject> {
    let fields = token_object_map(root)?;
    if fields.len() != 5
        || [
            "schema_version",
            "mock_read_records",
            "mock_write_i64",
            "mock_request",
            "mock_reply",
        ]
        .iter()
        .any(|key| !fields.contains_key(*key))
    {
        return Err(Reject::InvalidSchema);
    }
    if !matches!(fields.get("schema_version"), Some(Json::Number(version)) if version == "1") {
        return Err(Reject::InvalidSchema);
    }
    Ok(ReferenceEnvironment {
        reads: token_read_queue(fields.get("mock_read_records").unwrap())?,
        writes: token_unit_queue(fields.get("mock_write_i64").unwrap())?,
        requests: token_scalar_queue(fields.get("mock_request").unwrap())?,
        replies: token_unit_queue(fields.get("mock_reply").unwrap())?,
    })
}

fn token_object_map(value: Json) -> Result<BTreeMap<String, Json>, Reject> {
    let Json::Object(fields) = value else {
        return Err(Reject::InvalidSchema);
    };
    let mut map = BTreeMap::new();
    for (key, child) in fields {
        if map.insert(key, child).is_some() {
            return Err(Reject::InvalidSchema);
        }
    }
    Ok(map)
}

fn token_array(value: &Json) -> Result<&[Json], Reject> {
    match value {
        Json::Array(items) => Ok(items),
        _ => Err(Reject::InvalidSchema),
    }
}

fn token_i64(value: &Json) -> Result<i64, Reject> {
    let Json::String(text) = value else {
        return Err(Reject::InvalidSchema);
    };
    let digits = text.strip_prefix('-').unwrap_or(text);
    if digits.is_empty()
        || text == "-0"
        || (digits.len() > 1 && digits.starts_with('0'))
        || digits.chars().any(|character| !character.is_ascii_digit())
    {
        return Err(Reject::InvalidSchema);
    }
    text.parse::<i64>().map_err(|_| Reject::InvalidSchema)
}

fn token_result<T>(
    value: &Json,
    success: impl FnOnce(&Json) -> Result<T, Reject>,
) -> Result<Reply<T>, Reject> {
    let pairs = match value {
        Json::Object(fields) => fields,
        _ => return Err(Reject::InvalidSchema),
    };
    if pairs.len() != 1 {
        return Err(Reject::InvalidSchema);
    }
    let (key, payload) = &pairs[0];
    match key.as_str() {
        "ok" => success(payload).map(Ok),
        "err" => token_i64(payload).map(Err),
        _ => Err(Reject::InvalidSchema),
    }
}

fn token_scalar_queue(value: &Json) -> Result<VecDeque<Reply<i64>>, Reject> {
    token_array(value)?
        .iter()
        .map(|item| token_result(item, token_i64))
        .collect()
}

fn token_unit_queue(value: &Json) -> Result<VecDeque<Reply<()>>, Reject> {
    token_array(value)?
        .iter()
        .map(|item| {
            token_result(item, |payload| match payload {
                Json::Null => Ok(()),
                _ => Err(Reject::InvalidSchema),
            })
        })
        .collect()
}

fn token_read_queue(value: &Json) -> Result<VecDeque<Reply<Vec<Reply<i64>>>>, Reject> {
    token_array(value)?
        .iter()
        .map(|item| {
            token_result(item, |payload| {
                token_array(payload)?
                    .iter()
                    .map(|record| token_result(record, token_i64))
                    .collect()
            })
        })
        .collect()
}

fn json_array(value: &Json) -> Result<&[Json], Reject> {
    if let Json::Array(items) = value {
        Ok(items)
    } else {
        Err(Reject::InvalidSchema)
    }
}

fn json_object(value: &Json) -> Result<&[(String, Json)], Reject> {
    if let Json::Object(fields) = value {
        Ok(fields)
    } else {
        Err(Reject::InvalidSchema)
    }
}

fn canonical_integer(value: &Json) -> Result<i64, Reject> {
    let Json::String(text) = value else {
        return Err(Reject::InvalidSchema);
    };
    let digits = text.strip_prefix('-').unwrap_or(text);
    if digits.is_empty()
        || text == "-0"
        || (digits.len() > 1 && digits.starts_with('0'))
        || !digits.bytes().all(|byte| byte.is_ascii_digit())
    {
        return Err(Reject::InvalidSchema);
    }
    text.parse().map_err(|_| Reject::InvalidSchema)
}

fn result_value<T>(
    value: &Json,
    success: impl FnOnce(&Json) -> Result<T, Reject>,
) -> Result<Reply<T>, Reject> {
    let fields = json_object(value)?;
    if fields.len() != 1 {
        return Err(Reject::InvalidSchema);
    }
    match fields[0].0.as_str() {
        "ok" => success(&fields[0].1).map(Ok),
        "err" => canonical_integer(&fields[0].1).map(Err),
        _ => Err(Reject::InvalidSchema),
    }
}

fn scalar_queue(value: &Json) -> Result<VecDeque<Reply<i64>>, Reject> {
    json_array(value)?
        .iter()
        .map(|item| result_value(item, canonical_integer))
        .collect()
}

fn unit_queue(value: &Json) -> Result<VecDeque<Reply<()>>, Reject> {
    json_array(value)?
        .iter()
        .map(|item| {
            result_value(item, |payload| {
                matches!(payload, Json::Null)
                    .then_some(())
                    .ok_or(Reject::InvalidSchema)
            })
        })
        .collect()
}

fn read_queue(value: &Json) -> Result<VecDeque<Reply<Vec<Reply<i64>>>>, Reject> {
    json_array(value)?
        .iter()
        .map(|item| {
            result_value(item, |payload| {
                json_array(payload)?
                    .iter()
                    .map(|record| result_value(record, canonical_integer))
                    .collect()
            })
        })
        .collect()
}

#[derive(Debug)]
struct ActualExecution {
    outcome: Outcome,
    exit_code: u8,
    observations: Vec<(nexo::interp::observation::Channel, String, String)>,
    stdout: Vec<String>,
    events: Vec<String>,
    bytes: u64,
    reserved: u64,
    work: u64,
    diagnostics: Vec<String>,
    json_diagnostics: Vec<String>,
    diagnostic_details: Vec<nexo::diagnostic::Diagnostic>,
}

/// The only adapter from an oracle input to the public parser/compiler/runtime.
fn real_adapter(
    environment: &[u8],
    source: Option<&str>,
    profile: ReducedExecutionProfile,
) -> Result<Option<ActualExecution>, Reject> {
    let parsed = MockEnvironment::parse(environment).map_err(actual_rejection)?;
    let Some(source) = source else {
        return Ok(None);
    };
    let program = nexo::ir::compile_scalar_with_profile(
        "mock-oracle.n",
        source,
        match profile {
            ReducedExecutionProfile::Cells64 => nexo::ir::ReducedProfile::Cells64,
            _ => nexo::ir::ReducedProfile::Normal,
        },
        true,
    )
    .expect("reference program is complete B source");
    let execution = if matches!(profile, ReducedExecutionProfile::Cells64) {
        run_with_limits(&program, parsed, profile)
    } else {
        run_scalar(&program, parsed)
    };
    Ok(Some(execution_view(execution, source)))
}

fn real_adapter_precompiled(
    environment: &[u8],
    program: &nexo::ir::model::Program,
    source: &str,
) -> Result<ActualExecution, Reject> {
    let parsed = MockEnvironment::parse(environment).map_err(actual_rejection)?;
    Ok(execution_view(run_scalar(program, parsed), source))
}

fn execution_view(execution: nexo::interp::Execution, source: &str) -> ActualExecution {
    let observations = execution
        .observations
        .observations()
        .iter()
        .map(|item| (item.channel, item.text.clone(), item.prefix.clone()))
        .collect::<Vec<_>>();
    let json_diagnostics = execution
        .diagnostics
        .iter()
        .map(|item| {
            item.to_json_view(
                "mock-oracle.n",
                nexo::diagnostic::SourceView {
                    bytes_len: source.len(),
                    text: Some(source),
                },
            )
            .expect("runtime diagnostic serializes")
        })
        .collect();
    let outcome_exit_code = exit_code(&execution.outcome);
    ActualExecution {
        outcome: execution.outcome,
        exit_code: outcome_exit_code,
        stdout: observations
            .iter()
            .filter(|(channel, _, _)| *channel == nexo::interp::observation::Channel::Stdout)
            .map(|(_, text, _)| text.clone())
            .collect(),
        events: observations
            .iter()
            .filter(|(channel, _, _)| *channel == nexo::interp::observation::Channel::Event)
            .map(|(_, text, _)| text.clone())
            .collect(),
        observations,
        bytes: execution.observations.bytes(),
        reserved: execution.observations.reserved_bytes(),
        work: execution.work_consumed,
        diagnostics: execution
            .diagnostics
            .iter()
            .map(|item| item.code.to_owned())
            .collect(),
        json_diagnostics,
        diagnostic_details: execution
            .diagnostics
            .iter()
            .map(|item| (**item).clone())
            .collect(),
    }
}

#[derive(Clone, Copy, Debug)]
enum Operation {
    Read { label: i64 },
    Write { number: i64 },
    Request,
    Reply { status: i64 },
}

#[derive(Debug)]
struct ReferenceRun {
    outcome: Outcome,
    observations: Vec<(nexo::interp::observation::Channel, String, String)>,
    bytes: u64,
    exit_code: u8,
    work_delta_from_empty: u64,
}

fn event(
    expected: &mut Vec<(nexo::interp::observation::Channel, String, String)>,
    text: String,
    prefix: &str,
) {
    expected.push((
        nexo::interp::observation::Channel::Event,
        text,
        prefix.to_owned(),
    ));
}

fn reference_run(operations: &[Operation], env: &mut ReferenceEnvironment) -> ReferenceRun {
    use nexo::interp::observation::Channel;

    let mut observations = Vec::new();
    let mut next_resource = 0usize;
    let mut work_delta_from_empty = 0u64;
    for operation in operations {
        match operation {
            Operation::Read { label } => {
                let id = next_resource;
                next_resource += 1;
                event(
                    &mut observations,
                    format!("open:{id}:{label}"),
                    "mock-resource:",
                );
                match env.reads.pop_front().unwrap_or(Err(-9000)) {
                    Ok(records) => {
                        work_delta_from_empty += 2 * records.len() as u64;
                        event(
                            &mut observations,
                            format!("read_records:{id}:ok:{}", records.len()),
                            "mock-io:",
                        );
                    }
                    Err(error) => event(
                        &mut observations,
                        format!("read_records:{id}:err:{error}"),
                        "mock-io:",
                    ),
                }
                event(
                    &mut observations,
                    format!("close:{id}:{label}"),
                    "mock-resource:",
                );
            }
            Operation::Write { number } => match env.writes.pop_front().unwrap_or(Err(-9000)) {
                Ok(()) => {
                    work_delta_from_empty += 1;
                    event(
                        &mut observations,
                        format!("write_i64:{number}:ok"),
                        "mock-io:",
                    );
                    observations.push((Channel::Stdout, number.to_string(), String::new()));
                }
                Err(error) => event(
                    &mut observations,
                    format!("write_i64:{number}:err:{error}"),
                    "mock-io:",
                ),
            },
            Operation::Request => match env.requests.pop_front().unwrap_or(Err(-9000)) {
                Ok(value) => event(&mut observations, format!("request:ok:{value}"), "mock-io:"),
                Err(error) => event(
                    &mut observations,
                    format!("request:err:{error}"),
                    "mock-io:",
                ),
            },
            Operation::Reply { status } => match env.replies.pop_front().unwrap_or(Err(-9000)) {
                Ok(()) => event(&mut observations, format!("reply:{status}:ok"), "mock-io:"),
                Err(error) => event(
                    &mut observations,
                    format!("reply:{status}:err:{error}"),
                    "mock-io:",
                ),
            },
        }
    }
    let bytes = observations
        .iter()
        .map(|(_, text, prefix)| (prefix.len() + text.len() + 1) as u64)
        .sum();
    ReferenceRun {
        outcome: Outcome::Unit,
        observations,
        bytes,
        exit_code: 0,
        work_delta_from_empty,
    }
}

fn compare_runtime(expected: &ReferenceRun, actual: &ActualExecution, base_work: u64) {
    assert_eq!(actual.outcome, expected.outcome);
    assert_eq!(actual.exit_code, expected.exit_code);
    assert_eq!(actual.observations, expected.observations);
    let expected_stdout = expected
        .observations
        .iter()
        .filter(|(channel, _, _)| *channel == nexo::interp::observation::Channel::Stdout)
        .map(|(_, text, _)| text.clone())
        .collect::<Vec<_>>();
    let expected_events = expected
        .observations
        .iter()
        .filter(|(channel, _, _)| *channel == nexo::interp::observation::Channel::Event)
        .map(|(_, text, _)| text.clone())
        .collect::<Vec<_>>();
    assert_eq!(actual.stdout, expected_stdout);
    assert_eq!(actual.events, expected_events);
    assert_eq!(actual.bytes, expected.bytes);
    assert_eq!(actual.reserved, 0);
    assert!(actual.diagnostics.is_empty());
    assert!(actual.json_diagnostics.is_empty());
    assert_eq!(exit_code(&actual.outcome), expected.exit_code);
    assert_eq!(actual.work, base_work + expected.work_delta_from_empty);
}

fn exit_code(outcome: &Outcome) -> u8 {
    match outcome {
        Outcome::Unit | Outcome::Ok => 0,
        Outcome::Err(_) | Outcome::SimulatorFault => 1,
        Outcome::InternalError(_) => 2,
    }
}

fn campaign_open_failure_reason(source: &std::path::Path, mock: &std::path::Path) {
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_nexo"))
        .arg("run")
        .arg(source)
        .arg("--mock-env")
        .arg(mock)
        .arg("--format")
        .arg("json")
        .output()
        .expect("run CLI for mock environment open case");
    assert_eq!(output.status.code(), Some(2));
    assert!(output.stderr.is_empty());
    assert_eq!(
        String::from_utf8(output.stdout).unwrap(),
        concat!(
            "{\"schema_version\":1,\"profile_id\":\"f0-0002-b-v2\",",
            "\"outcome\":{\"kind\":\"driver_error\",\"reason\":\"mock_env_unreadable\"},",
            "\"stdout\":[],\"events\":[],\"observations\":[],",
            "\"observation_bytes\":0,\"reserved_observation_bytes\":0,\"diagnostics\":[]}\n"
        )
    );
}

fn campaign_file_open_categories() -> [&'static str; 3] {
    let directory = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("target")
        .join("test-fixtures")
        .join(format!("i11a-open-campaign-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&directory);
    std::fs::create_dir_all(&directory).unwrap();
    let source = directory.join("program.n");
    std::fs::write(&source, "fn Main(){}\n").unwrap();

    campaign_open_failure_reason(&source, &directory.join("missing.json"));
    let directory_path = directory.join("directory.json");
    std::fs::create_dir(&directory_path).unwrap();
    campaign_open_failure_reason(&source, &directory_path);

    #[cfg(windows)]
    {
        use std::os::windows::fs::OpenOptionsExt;

        let locked = directory.join("locked.json");
        std::fs::write(&locked, EMPTY_ENV).unwrap();
        let _exclusive = std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .share_mode(0)
            .open(&locked)
            .unwrap();
        campaign_open_failure_reason(&source, &locked);
    }

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;

        let unreadable = directory.join("unreadable.json");
        std::fs::write(&unreadable, EMPTY_ENV).unwrap();
        std::fs::set_permissions(&unreadable, std::fs::Permissions::from_mode(0)).unwrap();
        campaign_open_failure_reason(&source, &unreadable);
    }

    let _ = std::fs::remove_dir_all(&directory);
    ["missing", "directory", "unreadable"]
}

fn generated_environment(
    reads: &[Reply<Vec<Reply<i64>>>],
    writes: &[Reply<()>],
    requests: &[Reply<i64>],
    replies: &[Reply<()>],
) -> String {
    let read_json = reads
        .iter()
        .map(|reply| match reply {
            Ok(records) => format!(
                "{{\"ok\":[{}]}}",
                records
                    .iter()
                    .map(|record| match record {
                        Ok(value) => format!("{{\"ok\":\"{value}\"}}"),
                        Err(error) => format!("{{\"err\":\"{error}\"}}"),
                    })
                    .collect::<Vec<_>>()
                    .join(",")
            ),
            Err(error) => format!("{{\"err\":\"{error}\"}}"),
        })
        .collect::<Vec<_>>()
        .join(",");
    let write_json = unit_json(writes);
    let request_json = requests
        .iter()
        .map(|reply| match reply {
            Ok(value) => format!("{{\"ok\":\"{value}\"}}"),
            Err(error) => format!("{{\"err\":\"{error}\"}}"),
        })
        .collect::<Vec<_>>()
        .join(",");
    let reply_json = unit_json(replies);
    format!(
        "{{\"schema_version\":1,\"mock_read_records\":[{read_json}],\"mock_write_i64\":[{write_json}],\"mock_request\":[{request_json}],\"mock_reply\":[{reply_json}]}}"
    )
}

fn unit_json(values: &[Reply<()>]) -> String {
    values
        .iter()
        .map(|reply| match reply {
            Ok(()) => "{\"ok\":null}".to_owned(),
            Err(error) => format!("{{\"err\":\"{error}\"}}"),
        })
        .collect::<Vec<_>>()
        .join(",")
}

struct SplitMix64(u64);

impl SplitMix64 {
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9e3779b97f4a7c15);
        let mut value = self.0;
        value = (value ^ (value >> 30)).wrapping_mul(0xbf58476d1ce4e5b9);
        value = (value ^ (value >> 27)).wrapping_mul(0x94d049bb133111eb);
        value ^ (value >> 31)
    }

    fn integer(&mut self) -> i64 {
        const EDGES: [i64; 8] = [
            i64::MIN,
            i64::MIN + 1,
            -1,
            0,
            1,
            i64::MAX - 1,
            i64::MAX,
            -9000,
        ];
        let sample = self.next();
        if sample & 3 == 0 {
            EDGES[(sample as usize >> 8) % EDGES.len()]
        } else {
            (sample as i64).wrapping_rem(1_000_001)
        }
    }
}

fn random_environment(rng: &mut SplitMix64, batch_index: usize) -> ReferenceEnvironment {
    let count = if batch_index.is_multiple_of(16) {
        0
    } else if batch_index % 16 == 1 {
        1
    } else if batch_index % 16 == 2 {
        125
    } else {
        250
    };
    let reads = (0..count)
        .map(|_| {
            if rng.next().is_multiple_of(5) {
                Err(rng.integer())
            } else {
                let length = (rng.next() % 13) as usize;
                Ok((0..length)
                    .map(|_| {
                        if rng.next().is_multiple_of(4) {
                            Err(rng.integer())
                        } else {
                            Ok(rng.integer())
                        }
                    })
                    .collect())
            }
        })
        .collect();
    let writes = (0..count)
        .map(|_| {
            if rng.next().is_multiple_of(3) {
                Err(rng.integer())
            } else {
                Ok(())
            }
        })
        .collect();
    let requests = (0..count)
        .map(|_| {
            if rng.next().is_multiple_of(4) {
                Err(rng.integer())
            } else {
                Ok(rng.integer())
            }
        })
        .collect();
    let replies = (0..count)
        .map(|_| {
            if rng.next().is_multiple_of(5) {
                Err(rng.integer())
            } else {
                Ok(())
            }
        })
        .collect();
    ReferenceEnvironment {
        reads,
        writes,
        requests,
        replies,
    }
}

fn encode_environment(env: &ReferenceEnvironment) -> String {
    generated_environment(
        env.reads.iter().cloned().collect::<Vec<_>>().as_slice(),
        env.writes.iter().cloned().collect::<Vec<_>>().as_slice(),
        env.requests.iter().cloned().collect::<Vec<_>>().as_slice(),
        env.replies.iter().cloned().collect::<Vec<_>>().as_slice(),
    )
}

fn verify_indexed_read_batch(records: &[Vec<Reply<i64>>]) {
    let source = "fn Main()->Result<(),I64>{var i=0;while i<250{let rows=MockReadRecords(MockOpen(i))?;Print(Len(rows));var j=0;while j<Len(rows){Print(rows[j]);j=j+1;}i=i+1;}return Ok(());} ";
    let reads = records.iter().cloned().map(Ok).collect::<Vec<_>>();
    let json = generated_environment(&reads, &[], &[], &[]);
    let actual = real_adapter(
        json.as_bytes(),
        Some(source),
        ReducedExecutionProfile::Normal,
    )
    .unwrap()
    .unwrap();
    use nexo::interp::observation::Channel;
    let mut expected = Vec::new();
    for (index, rows) in records.iter().enumerate() {
        expected.push((
            Channel::Event,
            format!("open:{index}:{index}"),
            "mock-resource:".to_owned(),
        ));
        expected.push((
            Channel::Event,
            format!("read_records:{index}:ok:{}", rows.len()),
            "mock-io:".to_owned(),
        ));
        expected.push((
            Channel::Event,
            format!("close:{index}:{index}"),
            "mock-resource:".to_owned(),
        ));
        expected.push((Channel::Stdout, rows.len().to_string(), String::new()));
        for record in rows {
            let value = match record {
                Ok(value) => format!("Ok({value})"),
                Err(error) => format!("Err({error})"),
            };
            expected.push((Channel::Stdout, value, String::new()));
        }
    }
    let bytes = expected
        .iter()
        .map(|(_, text, prefix)| (text.len() + prefix.len() + 1) as u64)
        .sum::<u64>();
    let expected_stdout = expected
        .iter()
        .filter(|(channel, _, _)| *channel == Channel::Stdout)
        .map(|(_, text, _)| text.clone())
        .collect::<Vec<_>>();
    let expected_events = expected
        .iter()
        .filter(|(channel, _, _)| *channel == Channel::Event)
        .map(|(_, text, _)| text.clone())
        .collect::<Vec<_>>();
    assert_eq!(actual.outcome, Outcome::Ok);
    assert_eq!(actual.exit_code, 0);
    assert_eq!(actual.observations, expected);
    assert_eq!(actual.stdout, expected_stdout);
    assert_eq!(actual.events, expected_events);
    assert_eq!(actual.bytes, bytes);
    assert_eq!(actual.reserved, 0);
    assert!(actual.diagnostics.is_empty());
    assert!(actual.json_diagnostics.is_empty());
}

fn parser_frontier_base(case: usize, rng: &mut SplitMix64) -> Vec<u8> {
    const BASE: &str = "{\"schema_version\":1,\"mock_read_records\":[],\"mock_write_i64\":[],\"mock_request\":[],\"mock_reply\":[]}";
    const ESCAPES: [(&str, &str); 8] = [
        ("\\\"", "\\u0022"),
        ("\\\\", "\\u005c"),
        ("\\/", "/"),
        ("\\b", "\\u0008"),
        ("\\f", "\\u000c"),
        ("\\n", "\\u000a"),
        ("\\r", "\\u000d"),
        ("\\t", "\\u0009"),
    ];
    const INVALID_ESCAPES: [&str; 8] = [
        "\\x",
        "\\u12G4",
        "\\",
        "\\uD800",
        "\\uDC00",
        "\\uD800\\uDBFF",
        "\\uDFFF\\uD800",
        "\\uD800\\u0000",
    ];
    const VALID_PAIRS: [&str; 4] = [
        "\\uD800\\uDC00",
        "\\uD800\\uDFFF",
        "\\uDBFF\\uDC00",
        "\\uDBFF\\uDFFF",
    ];
    match case % 64 {
        0 => BASE.as_bytes().to_vec(),
        1 => b"{\"schema_version\":1,\t\"mock_read_records\":[],\"mock_write_i64\":[],\"mock_request\":[],\"mock_reply\":[]}".to_vec(),
        2 | 3 => {
            let control = ((case / 64) % 32) as u8;
            let mut input = if case % 64 == 2 {
                br#"{"schema_version":1,"mock_read_records":[],"mock_write_i64":[],"mock_request":[],"mock_reply":[],""#.to_vec()
            } else {
                br#"{"schema_version":1,"mock_read_records":[],"mock_write_i64":[],"mock_request":[{"ok":""#.to_vec()
            };
            input.push(control);
            if case % 64 == 2 {
                input.extend_from_slice(b"\":null}");
            } else {
                input.extend_from_slice(b"\"}],\"mock_reply\":[]}");
            }
            input
        }
        4 => {
            let numeric = ["1e0", "1E0", "1e-1", "1e+0", "1.0", "-0", "-1"];
            let exponent = numeric[(rng.next() as usize) % numeric.len()];
            format!("{{\"schema_version\":{exponent},\"mock_read_records\":[],\"mock_write_i64\":[],\"mock_request\":[],\"mock_reply\":[]}}").into_bytes()
        }
        5 => {
            let value = rng.integer();
            format!("{{\"schema_version\":1,\"mock_read_records\":[],\"mock_write_i64\":[],\"mock_request\":[{{\"ok\":\"{value}\"}}],\"mock_reply\":[]}}").into_bytes()
        }
        6 => {
            let invalid = ["01", "-01", "+1", "-0", "9223372036854775808", "-9223372036854775809"];
            let value = invalid[(rng.next() as usize) % invalid.len()];
            format!("{{\"schema_version\":1,\"mock_read_records\":[],\"mock_write_i64\":[],\"mock_request\":[{{\"ok\":\"{value}\"}}],\"mock_reply\":[]}}").into_bytes()
        }
        7 => b"{\"schema_version\":1,\"schema_version\":1,\"mock_read_records\":[],\"mock_write_i64\":[],\"mock_request\":[],\"mock_reply\":[]} trailing".to_vec(),
        8 => b"{\"schema_version\":1,\"mock_read_records\":[],\"mock_write_i64\":[],\"mock_request\":[],\"mock_reply\":[]} trailing".to_vec(),
        9..=11 => {
            let nesting = match case % 64 {
                9 => 15,
                10 => 16,
                _ => 17 + (rng.next() % 4) as usize,
            };
            format!("{{\"schema_version\":1,\"mock_read_records\":{}null{},\"mock_write_i64\":[],\"mock_request\":[],\"mock_reply\":[]}}", "[".repeat(nesting), "]".repeat(nesting)).into_bytes()
        }
        12 => b"{\"schema_version\":1,\"mock_read_records\":[],\"mock_write_i64\":[],\"mock_request\":[],\"mock_\\u0072eply\":[]}".to_vec(),
        13 => b"{\"schema_version\":1,\"mock_read_records\":[],\"mock_write_i64\":[],\"mock_request\":[],\"mock_reply\":[],\"bad\\x\":null}".to_vec(),
        14 => b"{\"schema_version\":1,\"mock_read_records\":[],\"mock_write_i64\":[],\"mock_request\":[],\"mock_reply\":[],\"\\ud800\\udfff\":null}".to_vec(),
        15 => {
            let variants = ["\\ud800", "\\udfff", "\\ud800\\ud800", "\\udfff\\ud800", "\\ud800\\udfff"];
            let value = variants[(rng.next() as usize) % variants.len()];
            format!("{{\"schema_version\":1,\"mock_read_records\":[],\"mock_write_i64\":[],\"mock_request\":[{{\"ok\":\"{value}\"}}],\"mock_reply\":[]}}").into_bytes()
        }
        16 => b"{\"schema_version\":1,\"mock_read_records\":[],\"mock_write_i64\":[],\"mock_request\":[{\"ok\":\"\\u0031\"}],\"mock_reply\":[]}".to_vec(),
        17 => b"{\"schema_version\":1.0,\"mock_read_records\":[],\"mock_write_i64\":[],\"mock_request\":[],\"mock_reply\":[]}".to_vec(),
        18 => vec![0xff, b'{', b'}'],
        19 => {
            let mut input = BASE.as_bytes().to_vec();
            let insertion = (rng.next() as usize % input.len()).max(1);
            input.insert(insertion, [0x00, 0x1f, b'\\', b'"'][rng.next() as usize % 4]);
            input
        }
        20..=27 => {
            let (escape, equivalent) = ESCAPES[case % 64 - 20];
            format!(
                "{{\"schema_version\":1,\"mock_read_records\":[],\"mock_write_i64\":[],\"mock_request\":[],\"mock_reply\":[],\"a{escape}b\":0,\"a{equivalent}b\":0}} trailing"
            )
            .into_bytes()
        }
        28..=35 => {
            let (escape, _) = ESCAPES[case % 64 - 28];
            format!(
                "{{\"schema_version\":1,\"mock_read_records\":[],\"mock_write_i64\":[],\"mock_request\":[{{\"ok\":\"{escape}\"}}],\"mock_reply\":[]}}"
            )
            .into_bytes()
        }
        36..=43 => {
            let escape = INVALID_ESCAPES[case % 64 - 36];
            format!(
                "{{\"schema_version\":1,\"mock_read_records\":[],\"mock_write_i64\":[],\"mock_request\":[],\"mock_reply\":[],\"bad{escape}\":null}}"
            )
            .into_bytes()
        }
        44..=51 => {
            let escape = INVALID_ESCAPES[case % 64 - 44];
            format!(
                "{{\"schema_version\":1,\"mock_read_records\":[],\"mock_write_i64\":[],\"mock_request\":[{{\"ok\":\"{escape}\"}}],\"mock_reply\":[]}}"
            )
            .into_bytes()
        }
        52..=55 => {
            let pair = VALID_PAIRS[case % 64 - 52];
            format!(
                "{{\"schema_version\":1,\"mock_read_records\":[],\"mock_write_i64\":[],\"mock_request\":[],\"mock_reply\":[],\"{pair}\":null}}"
            )
            .into_bytes()
        }
        56..=59 => {
            let pair = VALID_PAIRS[case % 64 - 56];
            format!(
                "{{\"schema_version\":1,\"mock_read_records\":[],\"mock_write_i64\":[],\"mock_request\":[{{\"ok\":\"{pair}\"}}],\"mock_reply\":[]}}"
            )
            .into_bytes()
        }
        60 => b"{\"schema_version\":1,\"mock_read_records\":[{}],\"mock_write_i64\":[],\"mock_request\":[],\"mock_reply\":[]}".to_vec(),
        61 => b"{\"schema_version\":1,\"mock_read_records\":[],\"mock_write_i64\":[{\"ok\":true}],\"mock_request\":[],\"mock_reply\":[]}".to_vec(),
        62 => b"{\"schema_version\":1,\"mock_read_records\":[],\"mock_write_i64\":[],\"mock_request\":[],\"mock_reply\":[],}".to_vec(),
        63 => b"{\"schema_version\":1,\"mock_read_records\":[],\"mock_write_i64\":[],\"mock_request\":[{\"ok\":\"-0\"}],\"mock_reply\":[]}".to_vec(),
        _ => unreachable!(),
    }
}

fn parser_frontier(case: usize, rng: &mut SplitMix64) -> Vec<u8> {
    let category = case % 128;
    let i64_slots = [2usize, 3, 4, 6, 7, 8, 10];
    if category < 64 {
        return parser_frontier_base(case, rng);
    }
    let literal_values = ["false", "true", "null", "0", "-1", "1.0"];
    let valid_i64 = [
        "0",
        "-1",
        "1",
        "9223372036854775807",
        "-9223372036854775808",
    ];
    let invalid_i64 = [
        "+5",
        "+0",
        "-0",
        "00",
        "01",
        "-01",
        "５",
        "١",
        "9223372036854775808",
        "-9223372036854775809",
    ];
    match category {
        64..=74 => {
            let slot = category - 64;
            let raw = literal_values[(rng.next() as usize) % literal_values.len()];
            environment_with_raw_slot(slot, raw).into_bytes()
        }
        75..=81 => {
            let slot = i64_slots[category - 75];
            let raw = format!("{:?}", valid_i64[(rng.next() as usize) % valid_i64.len()]);
            environment_with_raw_slot(slot, &raw).into_bytes()
        }
        82..=88 => {
            let slot = i64_slots[category - 82];
            let raw = format!(
                "{:?}",
                invalid_i64[(rng.next() as usize) % invalid_i64.len()]
            );
            environment_with_raw_slot(slot, &raw).into_bytes()
        }
        89..=96 => {
            let key = [
                "schema_version",
                "mock_read_records",
                "mock_write_i64",
                "mock_request",
                "mock_reply",
                "mock_reply",
                "mock_request",
                "mock_read_records",
            ][category - 89];
            let suffix = if category.is_multiple_of(2) {
                "} garbage"
            } else {
                "}"
            };
            format!(
                "{{\"schema_version\":1,\"mock_read_records\":[],\"mock_write_i64\":[],\"mock_request\":[],\"mock_reply\":[],\"{key}\":[]}}{suffix}"
            )
            .into_bytes()
        }
        97..=104 => {
            let value = ["true", "false", "null", "0", "-0", "+1", "01", "1.0"][category - 97];
            let position = (rng.next() as usize) % 4;
            match position {
                0 => format!("{{\"schema_version\":{value},\"mock_read_records\":[],\"mock_write_i64\":[],\"mock_request\":[],\"mock_reply\":[]}}"),
                1 => format!("{{\"schema_version\":1,\"mock_read_records\":[{{\"ok\":[{{\"ok\":{value}}}]}}],\"mock_write_i64\":[],\"mock_request\":[],\"mock_reply\":[]}}"),
                2 => format!("{{\"schema_version\":1,\"mock_read_records\":[],\"mock_write_i64\":[{{\"ok\":{value}}}],\"mock_request\":[],\"mock_reply\":[]}}"),
                _ => format!("{{\"schema_version\":1,\"mock_read_records\":[],\"mock_write_i64\":[],\"mock_request\":[{{\"ok\":{value}}}],\"mock_reply\":[]}}"),
            }
            .into_bytes()
        }
        105..=112 => {
            let tail = ["]", "}", "x", ",", "\"x", " false", " 01", " @"][category - 105];
            let base = std::str::from_utf8(EMPTY_ENV)
                .unwrap()
                .strip_suffix('}')
                .unwrap();
            format!("{base}{tail}").into_bytes()
        }
        113..=120 => {
            let invalid = [
                "{\"schema_version\":1,",
                "{\"schema_version\":1,\"mock_read_records\":[",
                "{\"schema_version\":1,\"mock_write_i64\":[{",
                "{\"schema_version\":1,\"mock_request\":[{\"ok\":\"5",
                "{\"schema_version\":1,\"mock_reply\":[{\"err\":\"-",
                "{\"schema_version\":1,\"mock_read_records\":[{\"ok\":[{\"ok\":\"1\"}",
                "{\"schema_version\":1,\"mock_write_i64\":[],\"mock_request\":[],\"mock_reply\":[]",
                "{\"schema_version\":1,\"mock_read_records\":[],\"mock_write_i64\":[],\"mock_request\":[],\"mock_reply\":[],",
            ][category - 113];
            invalid.as_bytes().to_vec()
        }
        121..=127 => {
            let length = (rng.next() % 128) as usize;
            (0..length).map(|_| rng.next() as u8).collect::<Vec<_>>()
        }
        _ => unreachable!(),
    }
}

fn check_parser_case(input: &[u8]) -> Option<Reject> {
    let reference_a = reference_environment(input).map(|_| ());
    let reference_b = token_reference_environment(input).map(|_| ());
    assert_eq!(
        reference_b, reference_a,
        "reference disagreement: {input:?}"
    );
    let actual = real_adapter(input, None, ReducedExecutionProfile::Normal).map(|_| ());
    assert_eq!(
        actual, reference_a,
        "implementation disagreement: {input:?}"
    );
    reference_a.err()
}

fn mutation_probe_sources() -> [String; 4] {
    let reads = (0..4)
        .map(|index| {
            format!("{{let rows_{index}=MockReadRecords(MockOpen({index})) ;Print(rows_{index});}}")
        })
        .collect::<String>();
    let writes = (0..4)
        .map(|index| format!("MockWriteI64({});", 100 + index))
        .collect::<String>();
    let requests = (0..4)
        .map(|_| "MockRequest();".to_owned())
        .collect::<String>();
    let replies = (0..4)
        .map(|index| format!("MockReply({});", 200 + index))
        .collect::<String>();
    [
        format!("fn Main(){{{reads}}}"),
        format!("fn Main(){{{writes}}}"),
        format!("fn Main(){{{requests}}}"),
        format!("fn Main(){{{replies}}}"),
    ]
}

fn probe_expected(
    mut environment: ReferenceEnvironment,
    kind: usize,
) -> (
    Outcome,
    Vec<(nexo::interp::observation::Channel, String, String)>,
) {
    use nexo::interp::observation::Channel;

    let mut observations = Vec::new();
    match kind {
        0 => {
            for index in 0..4 {
                event(
                    &mut observations,
                    format!("open:{index}:{index}"),
                    "mock-resource:",
                );
                let reply = environment.reads.pop_front().unwrap_or(Err(-9000));
                let display = match reply {
                    Ok(records) => {
                        event(
                            &mut observations,
                            format!("read_records:{index}:ok:{}", records.len()),
                            "mock-io:",
                        );
                        let records = records
                            .iter()
                            .map(|record| match record {
                                Ok(value) => format!("Ok({value})"),
                                Err(error) => format!("Err({error})"),
                            })
                            .collect::<Vec<_>>()
                            .join(", ");
                        format!("Ok([{records}])")
                    }
                    Err(error) => {
                        event(
                            &mut observations,
                            format!("read_records:{index}:err:{error}"),
                            "mock-io:",
                        );
                        format!("Err({error})")
                    }
                };
                event(
                    &mut observations,
                    format!("close:{index}:{index}"),
                    "mock-resource:",
                );
                observations.push((Channel::Stdout, display, String::new()));
            }
        }
        1 => {
            for index in 0..4 {
                let number = 100 + index;
                match environment.writes.pop_front().unwrap_or(Err(-9000)) {
                    Ok(()) => {
                        event(
                            &mut observations,
                            format!("write_i64:{number}:ok"),
                            "mock-io:",
                        );
                        observations.push((Channel::Stdout, number.to_string(), String::new()));
                    }
                    Err(error) => event(
                        &mut observations,
                        format!("write_i64:{number}:err:{error}"),
                        "mock-io:",
                    ),
                }
            }
        }
        2 => {
            for _ in 0..4 {
                match environment.requests.pop_front().unwrap_or(Err(-9000)) {
                    Ok(value) => {
                        event(&mut observations, format!("request:ok:{value}"), "mock-io:")
                    }
                    Err(error) => event(
                        &mut observations,
                        format!("request:err:{error}"),
                        "mock-io:",
                    ),
                }
            }
        }
        3 => {
            for index in 0..4 {
                let status = 200 + index;
                match environment.replies.pop_front().unwrap_or(Err(-9000)) {
                    Ok(()) => event(&mut observations, format!("reply:{status}:ok"), "mock-io:"),
                    Err(error) => event(
                        &mut observations,
                        format!("reply:{status}:err:{error}"),
                        "mock-io:",
                    ),
                }
            }
        }
        _ => unreachable!(),
    }
    (Outcome::Unit, observations)
}

fn compare_probe_output(
    expected: (
        Outcome,
        Vec<(nexo::interp::observation::Channel, String, String)>,
    ),
    actual: &ActualExecution,
    label: &str,
) {
    let (expected_outcome, expected_observations) = expected;
    let expected_stdout = expected_observations
        .iter()
        .filter(|(channel, _, _)| *channel == nexo::interp::observation::Channel::Stdout)
        .map(|(_, text, _)| text.clone())
        .collect::<Vec<_>>();
    let expected_events = expected_observations
        .iter()
        .filter(|(channel, _, _)| *channel == nexo::interp::observation::Channel::Event)
        .map(|(_, text, _)| text.clone())
        .collect::<Vec<_>>();
    let expected_bytes = expected_observations
        .iter()
        .map(|(_, text, prefix)| (text.len() + prefix.len() + 1) as u64)
        .sum::<u64>();
    assert_eq!(actual.outcome, expected_outcome, "{label}");
    assert_eq!(actual.exit_code, 0, "{label}");
    assert_eq!(actual.observations, expected_observations, "{label}");
    assert_eq!(actual.stdout, expected_stdout, "{label}");
    assert_eq!(actual.events, expected_events, "{label}");
    assert_eq!(actual.bytes, expected_bytes, "{label}");
    assert_eq!(actual.reserved, 0, "{label}");
    assert!(actual.diagnostics.is_empty(), "{label}");
    assert!(actual.json_diagnostics.is_empty(), "{label}");
}

fn schema_environment_with(slot: &str, body: &str) -> String {
    let fields = [
        ("mock_read_records", "[]"),
        ("mock_write_i64", "[]"),
        ("mock_request", "[]"),
        ("mock_reply", "[]"),
    ]
    .into_iter()
    .map(|(key, empty)| format!("\"{key}\":{}", if key == slot { body } else { empty }))
    .collect::<Vec<_>>()
    .join(",");
    format!("{{\"schema_version\":1,{fields}}}")
}

#[test]
fn mock_oracle_enumerates_schema_variants_for_each_grammar_node() {
    const JSON_TYPES: [&str; 8] = ["null", "false", "true", "0", "1.0", "\"1\"", "[]", "{}"];
    let cases = std::cell::Cell::new(0usize);
    let empty_text = std::str::from_utf8(EMPTY_ENV).unwrap();
    let reject = |input: String, context: &str| {
        assert_eq!(
            check_parser_case(input.as_bytes()),
            Some(Reject::InvalidSchema),
            "{context}: {input}"
        );
        cases.set(cases.get() + 1);
    };

    for missing in [
        "schema_version",
        "mock_read_records",
        "mock_write_i64",
        "mock_request",
        "mock_reply",
    ] {
        let mut fields = vec![
            "\"schema_version\":1".to_owned(),
            "\"mock_read_records\":[]".to_owned(),
            "\"mock_write_i64\":[]".to_owned(),
            "\"mock_request\":[]".to_owned(),
            "\"mock_reply\":[]".to_owned(),
        ];
        fields.retain(|field| !field.starts_with(&format!("\"{missing}\"")));
        reject(format!("{{{}}}", fields.join(",")), "missing root key");
    }
    reject(
        empty_text.strip_suffix('}').unwrap().to_owned() + ",\"unknown\":[]}",
        "unknown root key",
    );
    for duplicate in [
        "schema_version",
        "mock_read_records",
        "mock_write_i64",
        "mock_request",
        "mock_reply",
    ] {
        let value = match duplicate {
            "schema_version" => "1",
            _ => "[]",
        };
        let input = format!(
            "{},\"{duplicate}\":{value}}}",
            empty_text.strip_suffix('}').unwrap()
        );
        reject(input, "duplicate root key");
    }

    for raw in JSON_TYPES {
        let input = format!(
            "{{\"schema_version\":{raw},\"mock_read_records\":[],\"mock_write_i64\":[],\"mock_request\":[],\"mock_reply\":[]}}"
        );
        reject(input, "version JSON type");
    }
    reject(
        empty_text.replace("\"schema_version\":1", "\"schema_version\":2"),
        "wrong version value",
    );
    assert_eq!(check_parser_case(EMPTY_ENV), None);

    for queue in [
        "mock_read_records",
        "mock_write_i64",
        "mock_request",
        "mock_reply",
    ] {
        for raw in [
            "null", "false", "true", "0", "1.0", "\"[]\"", "{}", "[null]",
        ] {
            let input = schema_environment_with(queue, raw);
            reject(input, "queue type/shape");
        }
        assert_eq!(
            check_parser_case(schema_environment_with(queue, "[]").as_bytes()),
            None,
            "empty queue {queue}"
        );
    }

    let result_nodes = ["read", "record", "unit", "petition", "reply"];
    let payload_for = |_node: &str, _key: &str, raw: &str| raw.to_owned();
    let wrap_result = |node: &str, body: &str| match node {
        "read" => schema_environment_with("mock_read_records", &format!("[{body}]")),
        "record" => schema_environment_with("mock_read_records", &format!("[{{\"ok\":[{body}]}}]")),
        "unit" => schema_environment_with("mock_write_i64", &format!("[{body}]")),
        "petition" => schema_environment_with("mock_request", &format!("[{body}]")),
        "reply" => schema_environment_with("mock_reply", &format!("[{body}]")),
        _ => unreachable!(),
    };
    for node in result_nodes {
        for key in ["ok", "err"] {
            let valid_payload = match (node, key) {
                ("read", "ok") => "[]",
                ("unit", "ok") | ("reply", "ok") => "null",
                _ => "\"1\"",
            };
            let invalid_key = format!(r#"{{"other":{valid_payload}}}"#);
            reject(wrap_result(node, &invalid_key), "unknown result key");
            reject(wrap_result(node, "{}"), "missing result key");
            let both = format!(r#"{{"ok":{valid_payload},"err":"1"}}"#);
            reject(wrap_result(node, &both), "two result keys");

            let duplicate_ok = format!(r#"{{"ok":{valid_payload},"ok":{valid_payload}}}"#);
            reject(wrap_result(node, &duplicate_ok), "duplicate ok key");
            let duplicate_err = r#"{"err":"1","err":"2"}"#;
            reject(wrap_result(node, duplicate_err), "duplicate err key");

            for raw in JSON_TYPES {
                let payload = payload_for(node, key, raw);
                let body = format!(r#"{{"{key}":{payload}}}"#);
                let expected_valid = matches!(
                    (node, key, raw),
                    ("read", "ok", "[]")
                        | ("unit", "ok", "null")
                        | ("reply", "ok", "null")
                        | (_, "err", "\"1\"")
                        | ("record" | "petition", "ok", "\"1\"")
                );
                let outcome = check_parser_case(wrap_result(node, &body).as_bytes());
                assert_eq!(
                    outcome,
                    (!expected_valid).then_some(Reject::InvalidSchema),
                    "node={node} key={key} JSON={raw}"
                );
                cases.set(cases.get() + 1);
            }
        }
    }

    for (node, integer_slots) in [
        ("read", [2usize, 4, 0, 0, 0, 0, 0]),
        ("record", [3usize, 4, 0, 0, 0, 0, 0]),
        ("unit", [6usize, 0, 0, 0, 0, 0, 0]),
        ("petition", [7usize, 8, 0, 0, 0, 0, 0]),
        ("reply", [10usize, 0, 0, 0, 0, 0, 0]),
    ] {
        let slots = integer_slots.into_iter().filter(|slot| *slot != 0);
        for slot in slots {
            for value in [
                "+5",
                "+0",
                "-0",
                "00",
                "01",
                "-01",
                "",
                " 1",
                "1 ",
                "1.0",
                "1e0",
                "9223372036854775808",
                "-9223372036854775809",
                "٥",
                "١",
            ] {
                reject(
                    environment_with_raw_slot(slot, &format!("{value:?}")),
                    &format!("{node} noncanonical I64"),
                );
            }
        }
    }
    eprintln!(
        "schema_variants={} grammar_nodes=11 json_types=6 i64_nodes=8",
        cases.get()
    );
    if let Ok(path) = std::env::var("NEXO_SCHEMA_EVIDENCE_PATH") {
        let evidence = format!(
            "{{\"cases\":{},\"categories\":{{\"root_key_structure\":11,\"version_type_and_value\":9,\"queue_types_and_shapes\":32,\"result_key_structure\":50,\"result_payload_json_types\":80,\"canonical_i64_by_slot\":120}},\"empty_queue_acceptance_cases\":4,\"nodes\":11,\"i64_slots\":8}}\n",
            cases.get()
        );
        std::fs::write(path, evidence).expect("write schema coverage evidence");
    }
}

fn run_structural_sequence_campaign(documents: &[(&str, &[u8])]) -> BTreeMap<String, usize> {
    let mut categories = BTreeMap::new();
    for (name, document) in documents {
        for position in 0..=document.len() {
            for (category, sequence) in STRUCTURAL_SEQUENCES {
                let mut inserted = Vec::with_capacity(document.len() + sequence.len());
                inserted.extend_from_slice(&document[..position]);
                inserted.extend_from_slice(sequence.as_bytes());
                inserted.extend_from_slice(&document[position..]);
                let inserted_result = check_parser_case(&inserted);
                assert!(
                    matches!(
                        inserted_result,
                        None | Some(Reject::InvalidJson | Reject::InvalidSchema)
                    ),
                    "{name} insert {category} at {position}"
                );
                *categories.entry(format!("{category}.insert")).or_insert(0) += 1;

                if position + 2 <= document.len() {
                    let mut replaced = Vec::with_capacity(document.len() - 2 + sequence.len());
                    replaced.extend_from_slice(&document[..position]);
                    replaced.extend_from_slice(sequence.as_bytes());
                    replaced.extend_from_slice(&document[position + 2..]);
                    check_parser_case(&replaced);
                    *categories
                        .entry(format!("{category}.replace_two_bytes"))
                        .or_insert(0) += 1;
                }
            }
        }
    }
    categories
}

fn json_counts(counts: &BTreeMap<String, usize>) -> String {
    let fields = counts
        .iter()
        .map(|(name, count)| format!("{name:?}:{count}"))
        .collect::<Vec<_>>()
        .join(",");
    format!("{{{fields}}}")
}

fn assert_structural_boundary_corpus() -> BTreeMap<String, usize> {
    let mut categories = BTreeMap::new();
    for (version, expected) in [
        ("1.", Reject::InvalidJson),
        (".5", Reject::InvalidJson),
        ("1e", Reject::InvalidJson),
        ("1E", Reject::InvalidJson),
        ("1e+", Reject::InvalidJson),
        ("01", Reject::InvalidJson),
        ("-", Reject::InvalidJson),
        ("-0", Reject::InvalidSchema),
        ("1e0", Reject::InvalidSchema),
        ("1E+0", Reject::InvalidSchema),
        ("1e-0", Reject::InvalidSchema),
        ("1.0", Reject::InvalidSchema),
    ] {
        let input = String::from_utf8(EMPTY_ENV.to_vec()).unwrap().replacen(
            "\"schema_version\":1",
            &format!("\"schema_version\":{version}"),
            1,
        );
        assert_eq!(
            check_parser_case(input.as_bytes()),
            Some(expected),
            "{version}"
        );
        let category = if matches!(expected, Reject::InvalidJson) {
            "malformed_numbers"
        } else {
            "valid_noncanonical_numbers"
        };
        *categories.entry(category.to_owned()).or_insert(0) += 1;
    }

    for (escape, expected) in [
        (r#"\u0031"#, None),
        (r#"\uD800\uDC00"#, Some(Reject::InvalidSchema)),
        (r#"\uD800"#, Some(Reject::InvalidJson)),
        (r#"\uDC00"#, Some(Reject::InvalidJson)),
        (r#"\uD800\u0041"#, Some(Reject::InvalidJson)),
        (r#"\uDC00\uD800"#, Some(Reject::InvalidJson)),
        (r#"\u00G0"#, Some(Reject::InvalidJson)),
        (r#"\u"#, Some(Reject::InvalidJson)),
        (r#"\u0"#, Some(Reject::InvalidJson)),
    ] {
        let input = schema_environment_with("mock_request", &format!(r#"[{{"ok":"{escape}"}}]"#));
        assert_eq!(
            check_parser_case(input.as_bytes()),
            expected,
            "escape={escape:?}"
        );
        *categories.entry("unicode_escapes".to_owned()).or_insert(0) += 1;
    }

    for kind in ["arrays", "objects"] {
        for (nested_depth, expected) in [(15, Reject::InvalidSchema), (16, Reject::InvalidJson)] {
            let nested = match kind {
                "arrays" => format!(
                    "{}null{}",
                    "[".repeat(nested_depth),
                    "]".repeat(nested_depth)
                ),
                "objects" => format!(
                    "{}null{}",
                    "{\"x\":".repeat(nested_depth),
                    "}".repeat(nested_depth)
                ),
                _ => unreachable!(),
            };
            let mut input = String::from_utf8(EMPTY_ENV.to_vec()).unwrap();
            input.pop();
            input.push_str(&format!(",\"extra\":{nested}}}"));
            assert_eq!(
                check_parser_case(input.as_bytes()),
                Some(expected),
                "{kind} nested_depth={nested_depth}"
            );
            let category = if kind == "arrays" {
                "array_depth_boundaries"
            } else {
                "object_depth_boundaries"
            };
            *categories.entry(category.to_owned()).or_insert(0) += 1;
        }
    }

    for size in [1_048_575, 1_048_576, 1_048_577] {
        let mut input = EMPTY_ENV.to_vec();
        input.resize(size, b' ');
        assert_eq!(
            check_parser_case(&input),
            (size > 1_048_576).then_some(Reject::TooLarge),
            "size={size}"
        );
        *categories.entry("size_boundaries".to_owned()).or_insert(0) += 1;
    }
    categories
}

#[test]
fn mock_oracle_structural_sequence_campaign_runs_in_the_normal_suite() {
    let categories = run_structural_sequence_campaign(&DOCUMENT_MUTATION_SEEDS[..4]);
    let cases: usize = categories.values().sum();
    assert!(cases >= 20_000, "structural campaign cases={cases}");
    assert!(categories.len() >= 12, "categories={categories:?}");
    assert!(categories.values().all(|count| *count > 0));
    let boundary_categories = assert_structural_boundary_corpus();
    let boundary_cases: usize = boundary_categories.values().sum();
    eprintln!(
        "structural_sequence_campaign_cases={cases} boundary_cases={boundary_cases} categories={categories:?} boundary_categories={boundary_categories:?}"
    );
    if let Ok(path) = std::env::var("NEXO_STRUCTURAL_EVIDENCE_PATH") {
        let evidence = format!(
            "{{\"cases\":{cases},\"boundary_cases\":{boundary_cases},\"categories\":{},\"boundary_categories\":{}}}\n",
            json_counts(&categories),
            json_counts(&boundary_categories)
        );
        std::fs::write(path, evidence).expect("write structural campaign evidence");
    }
}

#[test]
#[ignore = "exhaustive byte and structural-sequence mutations over valid mock documents"]
fn mock_oracle_exhaustive_document_mutation_campaign() {
    let seed = std::env::var("NEXO_DOCUMENT_MUTATION_SEED")
        .expect("document mutation campaign seed")
        .parse::<u64>()
        .expect("integer document mutation seed");
    let documents = DOCUMENT_MUTATION_SEEDS;
    let sources = mutation_probe_sources();
    let programs = sources
        .iter()
        .enumerate()
        .map(|(index, source)| {
            nexo::ir::compile_scalar(&format!("mutation-probe-{index}.n"), source)
                .unwrap_or_else(|error| panic!("probe {index} failed to compile: {error:?}"))
        })
        .collect::<Vec<_>>();

    let totals = std::array::from_fn::<_, 7, _>(|_| std::cell::Cell::new(0usize));
    let rejected = std::array::from_fn::<_, 3, _>(|_| std::cell::Cell::new(0usize));
    let accepted = std::cell::Cell::new(0usize);
    let checked = std::cell::Cell::new(0usize);
    let mut sequence_categories = BTreeMap::<String, usize>::new();
    let start_document = seed as usize % documents.len();
    for document_offset in 0..documents.len() {
        let (name, document) = documents[(start_document + document_offset) % documents.len()];
        let byte_len = document.len();
        let examine = |mutated: &[u8], operation: usize, position: usize, byte: Option<u8>| {
            checked.set(checked.get() + 1);
            totals[operation].set(totals[operation].get() + 1);
            let rejection = check_parser_case(mutated);
            if let Some(reason) = rejection {
                let index = match reason {
                    Reject::TooLarge => 0,
                    Reject::InvalidJson => 1,
                    Reject::InvalidSchema => 2,
                };
                rejected[index].set(rejected[index].get() + 1);
                return;
            }

            accepted.set(accepted.get() + 1);
            let model_a = reference_environment(mutated)
                .unwrap_or_else(|error| panic!("accepted A parse failed: {error:?}"));
            let model_b = token_reference_environment(mutated)
                .unwrap_or_else(|error| panic!("accepted B parse failed: {error:?}"));
            assert_eq!(
                model_a, model_b,
                "{name} {operation} at {position} {byte:?}"
            );
            for kind in 0..4 {
                let expected = probe_expected(model_a.clone(), kind);
                let actual = real_adapter_precompiled(mutated, &programs[kind], &sources[kind])
                    .unwrap_or_else(|error| panic!("accepted production parse failed: {error:?}"));
                compare_probe_output(
                    expected,
                    &actual,
                    &format!(
                        "{name} operation={operation} position={position} byte={byte:?} probe={kind}"
                    ),
                );
            }
        };

        for position in 0..=byte_len {
            if position < byte_len {
                let mut deleted = document.to_vec();
                deleted.remove(position);
                examine(&deleted, 0, position, None);
            }

            for byte in u8::MIN..=u8::MAX {
                let mut inserted = Vec::with_capacity(byte_len + 1);
                inserted.extend_from_slice(&document[..position]);
                inserted.push(byte);
                inserted.extend_from_slice(&document[position..]);
                examine(&inserted, 1, position, Some(byte));
            }

            if position < byte_len {
                for byte in u8::MIN..=u8::MAX {
                    let mut replaced = document.to_vec();
                    replaced[position] = byte;
                    examine(&replaced, 2, position, Some(byte));
                }
                examine(&document[..position], 3, position, None);

                let mut duplicated = document.to_vec();
                duplicated.insert(position, document[position]);
                examine(&duplicated, 4, position, Some(document[position]));
            } else {
                examine(&document[..position], 3, position, None);
            }

            for (category, sequence) in STRUCTURAL_SEQUENCES {
                let mut inserted = Vec::with_capacity(byte_len + sequence.len());
                inserted.extend_from_slice(&document[..position]);
                inserted.extend_from_slice(sequence.as_bytes());
                inserted.extend_from_slice(&document[position..]);
                examine(&inserted, 5, position, None);
                *sequence_categories
                    .entry(format!("{category}.insert"))
                    .or_default() += 1;

                if position + 2 <= byte_len {
                    let mut replaced = Vec::with_capacity(byte_len - 2 + sequence.len());
                    replaced.extend_from_slice(&document[..position]);
                    replaced.extend_from_slice(sequence.as_bytes());
                    replaced.extend_from_slice(&document[position + 2..]);
                    examine(&replaced, 6, position, None);
                    *sequence_categories
                        .entry(format!("{category}.replace_two_bytes"))
                        .or_default() += 1;
                }
            }
            if position.is_multiple_of(32) {
                eprintln!(
                    "document_mutation_progress={name}:{position}/{byte_len} checked={}",
                    checked.get()
                );
            }
        }
    }

    assert!(
        checked.get() > 100_000,
        "systematic mutation count={checked:?}"
    );
    assert!(
        accepted.get() > 0,
        "mutations should include unchanged valid bytes"
    );
    let boundary_categories = assert_structural_boundary_corpus();
    let boundary_cases: usize = boundary_categories.values().sum();
    checked.set(checked.get() + boundary_cases);
    eprintln!(
        "document_mutation_seed={seed} documents={} checked={} boundary_cases={boundary_cases} boundary_categories={boundary_categories:?} operations={{delete:{},insert_each_byte:{},replace_each_byte:{},truncate:{},duplicate_byte:{},insert_sequences:{},replace_two_bytes_with_sequences:{}}} sequence_categories={sequence_categories:?} accepted={} rejected={{too_large:{},invalid_json:{},invalid_schema:{}}}",
        documents.len(),
        checked.get(),
        totals[0].get(),
        totals[1].get(),
        totals[2].get(),
        totals[3].get(),
        totals[4].get(),
        totals[5].get(),
        totals[6].get(),
        accepted.get(),
        rejected[0].get(),
        rejected[1].get(),
        rejected[2].get()
    );
    if let Ok(path) = std::env::var("NEXO_DOCUMENT_MUTATION_EVIDENCE_PATH") {
        let evidence = format!(
            "{{\"seed\":{seed},\"documents\":{},\"checked\":{},\"boundary_cases\":{boundary_cases},\"boundary_categories\":{},\"operations\":{{\"delete\":{},\"insert_each_byte\":{},\"replace_each_byte\":{},\"truncate\":{},\"duplicate_byte\":{},\"insert_sequences\":{},\"replace_two_bytes_with_sequences\":{}}},\"sequence_categories\":{},\"accepted\":{},\"rejected\":{{\"mock_env_too_large\":{},\"invalid_mock_json\":{},\"invalid_mock_schema\":{}}},\"accepted_runtime_probes_per_case\":4}}\n",
            documents.len(),
            checked.get(),
            json_counts(&boundary_categories),
            totals[0].get(),
            totals[1].get(),
            totals[2].get(),
            totals[3].get(),
            totals[4].get(),
            totals[5].get(),
            totals[6].get(),
            json_counts(&sequence_categories),
            accepted.get(),
            rejected[0].get(),
            rejected[1].get(),
            rejected[2].get()
        );
        std::fs::write(path, evidence).expect("write document mutation evidence");
    }
}

#[test]
fn mock_oracle_parser_contract() {
    let valid = ReferenceEnvironment::default();
    let encoded = encode_environment(&valid);
    assert_eq!(check_parser_case(encoded.as_bytes()), None);
    assert_eq!(check_parser_case(EMPTY_ENV), None);
    assert_eq!(
        check_parser_case(b"{\n\"mock_reply\":[],\r\n\"mock_request\":[],\n\"mock_write_i64\":[],\r\"mock_read_records\":[],\n\"schema_version\":1\r}"),
        None,
        "the four JSON whitespace bytes are accepted in every position"
    );

    let invalid_schema = br#"{"\ud800\udfff":1,"mock_read_records":[],"mock_write_i64":[],"mock_request":[],"mock_reply":[]}"#;
    assert_eq!(
        reference_environment(invalid_schema),
        Err(Reject::InvalidSchema)
    );
    let actual = real_adapter(invalid_schema, None, ReducedExecutionProfile::Normal).unwrap_err();
    assert_eq!(actual, Reject::InvalidSchema);
    assert_eq!(actual.controller_reason(), "invalid_mock_schema");

    assert_eq!(
        check_parser_case(br#"{"schema_version":1,"mock_read_records":[],"mock_write_i64":[],"mock_request":[{"ok":"01"}],"mock_reply":[]}"#),
        Some(Reject::InvalidSchema)
    );
    assert_eq!(
        check_parser_case(br#"{"schema_version":1,"mock_read_records":[],"mock_write_i64":[],"mock_request":[{"ok":"-0"}],"mock_reply":[]}"#),
        Some(Reject::InvalidSchema)
    );
    assert_eq!(
        check_parser_case(br#"{"schema_version":1,"mock_read_records":[],"mock_write_i64":[],"mock_request":[],"mock_reply":[],"extra":[]}"#),
        Some(Reject::InvalidSchema)
    );
    assert_eq!(
        check_parser_case(br#"{"schema_version":1,"mock_read_records":[],"mock_write_i64":[{"ok":null,"err":"1"}],"mock_request":[],"mock_reply":[]}"#),
        Some(Reject::InvalidSchema)
    );
    assert_eq!(
        check_parser_case(br#"{"schema_version":1,"mock_read_records":[],"mock_write_i64":[],"mock_request":[{"ok":"9223372036854775807"},{"ok":"-9223372036854775808"}],"mock_reply":[]}"#),
        None
    );
    assert_eq!(
        check_parser_case(br#"{"schema_version":01,"mock_read_records":[],"mock_write_i64":[],"mock_request":[],"mock_reply":[]}"#),
        Some(Reject::InvalidJson)
    );
    assert_eq!(
        check_parser_case(br#"{"schema_version":-1,"mock_read_records":[],"mock_write_i64":[],"mock_request":[],"mock_reply":[]}"#),
        Some(Reject::InvalidSchema)
    );
    assert_eq!(
        check_parser_case(br#"{"schema_version":1E0,"mock_read_records":[],"mock_write_i64":[],"mock_request":[],"mock_reply":[]}"#),
        Some(Reject::InvalidSchema)
    );
    assert_eq!(
        check_parser_case(br#"{"schema_version":1e-1,"mock_read_records":[],"mock_write_i64":[],"mock_request":[],"mock_reply":[]}"#),
        Some(Reject::InvalidSchema)
    );
    assert_eq!(
        check_parser_case(br#"{"schema_version":1.0,"mock_read_records":[],"mock_write_i64":[],"mock_request":[],"mock_reply":[]}"#),
        Some(Reject::InvalidSchema)
    );
    assert_eq!(
        check_parser_case(br#"{"schema_version":1,"mock_read_records":[],"mock_write_i64":[],"mock_request":[],"mock_reply":[],"extra key":null}"#),
        Some(Reject::InvalidSchema)
    );
    assert_eq!(
        check_parser_case(br#"{"schema_version":1,"schema_version":1,"mock_read_records":[],"mock_write_i64":[],"mock_request":[],"mock_reply":[]} trailing"#),
        Some(Reject::InvalidJson)
    );
    assert_eq!(
        check_parser_case(br#"{"schema_version":1,"schema_version":1,"mock_read_records":[],"mock_write_i64":[],"mock_request":[],"mock_reply":[]}"#),
        Some(Reject::InvalidSchema)
    );
    assert_eq!(
        check_parser_case(br#"{"schema_version":1,"mock_reply":[],"mock_\u0072eply":[],"mock_read_records":[],"mock_write_i64":[]}"#),
        Some(Reject::InvalidSchema)
    );

    // JSON strings reject every raw U+0000..U+001F byte in either key or value.
    for control in 0u8..=0x1f {
        let mut in_key = br#"{"schema_version":1,"mock_read_records":[],"mock_write_i64":[],"mock_request":[],"mock_reply":[],""#.to_vec();
        in_key.push(control);
        in_key.extend_from_slice(b"\":null}");
        assert_eq!(
            check_parser_case(&in_key),
            Some(Reject::InvalidJson),
            "key control {control:#x}"
        );

        let mut in_value = br#"{"schema_version":1,"mock_read_records":[],"mock_write_i64":[],"mock_request":[{"ok":""#.to_vec();
        in_value.push(control);
        in_value.extend_from_slice(b"\"}],\"mock_reply\":[]}");
        assert_eq!(
            check_parser_case(&in_value),
            Some(Reject::InvalidJson),
            "value control {control:#x}"
        );
    }

    for (escaped, expected) in [
        (br#"\""#.as_slice(), Reject::InvalidSchema),
        (br#"\\"#, Reject::InvalidSchema),
        (br#"\/"#, Reject::InvalidSchema),
        (br#"\b"#, Reject::InvalidSchema),
        (br#"\f"#, Reject::InvalidSchema),
        (br#"\n"#, Reject::InvalidSchema),
        (br#"\r"#, Reject::InvalidSchema),
        (br#"\t"#, Reject::InvalidSchema),
        (br#"\u006f\u006b"#, Reject::InvalidSchema),
        (br#"\uD800\uDFFF"#, Reject::InvalidSchema),
        (br#"\uD800"#, Reject::InvalidJson),
        (br#"\uDC00"#, Reject::InvalidJson),
        (br#"\uD800\uDBFF"#, Reject::InvalidJson),
        (br#"\uDFFF\uD800"#, Reject::InvalidJson),
        (br#"\x20"#, Reject::InvalidJson),
        (br#"\u12G4"#, Reject::InvalidJson),
    ] {
        let mut input = br#"{"schema_version":1,"mock_read_records":[],"mock_write_i64":[],"mock_request":[],"mock_reply":[],""#.to_vec();
        input.extend_from_slice(escaped);
        input.extend_from_slice(b"\":null}");
        assert_eq!(
            check_parser_case(&input),
            Some(expected),
            "escape={escaped:?}"
        );
    }

    for (escape, equivalent) in [
        ("\\\"", "\\u0022"),
        ("\\\\", "\\u005c"),
        ("\\/", "/"),
        ("\\b", "\\u0008"),
        ("\\f", "\\u000c"),
        ("\\n", "\\u000a"),
        ("\\r", "\\u000d"),
        ("\\t", "\\u0009"),
    ] {
        let duplicate = format!(
            "{{\"schema_version\":1,\"mock_read_records\":[],\"mock_write_i64\":[],\"mock_request\":[],\"mock_reply\":[],\"a{escape}b\":0,\"a{equivalent}b\":0}} trailing"
        );
        assert_eq!(
            check_parser_case(duplicate.as_bytes()),
            Some(Reject::InvalidJson),
            "duplicate decoded escape {escape:?}"
        );
    }

    for (escaped, expected) in [
        (br#"\""#.as_slice(), Reject::InvalidSchema),
        (br#"\\"#, Reject::InvalidSchema),
        (br#"\/"#, Reject::InvalidSchema),
        (br#"\b"#, Reject::InvalidSchema),
        (br#"\f"#, Reject::InvalidSchema),
        (br#"\n"#, Reject::InvalidSchema),
        (br#"\r"#, Reject::InvalidSchema),
        (br#"\t"#, Reject::InvalidSchema),
        (br#"\ud800\udfff"#, Reject::InvalidSchema),
        (br#"\ud800"#, Reject::InvalidJson),
        (br#"\udfff"#, Reject::InvalidJson),
        (br#"\x20"#, Reject::InvalidJson),
    ] {
        let mut input = br#"{"schema_version":1,"mock_read_records":[],"mock_write_i64":[],"mock_request":[{"ok":""#.to_vec();
        input.extend_from_slice(escaped);
        input.extend_from_slice(b"\"}],\"mock_reply\":[]}");
        assert_eq!(
            check_parser_case(&input),
            Some(expected),
            "value escape={escaped:?}"
        );
    }
    assert_eq!(
        check_parser_case(br#"{"schema_version":1,"mock_read_records":[],"mock_write_i64":[],"mock_request":[{"ok":"\u0031"}],"mock_reply":[]}"#),
        None,
        "decoded Unicode escape is a canonical decimal digit"
    );

    for nesting in [15usize, 16, 17] {
        let nested = format!(
            "{{\"schema_version\":1,\"mock_read_records\":{}null{},\"mock_write_i64\":[],\"mock_request\":[],\"mock_reply\":[]}}",
            "[".repeat(nesting),
            "]".repeat(nesting)
        );
        let expected = if nesting < 16 {
            Reject::InvalidSchema
        } else {
            Reject::InvalidJson
        };
        assert_eq!(check_parser_case(nested.as_bytes()), Some(expected));
    }
    let empty_objects = std::iter::repeat_n("{}", 15).collect::<Vec<_>>().join(",");
    let sibling_objects = format!(
        "{{\"schema_version\":1,\"mock_read_records\":[{empty_objects}],\"mock_write_i64\":[],\"mock_request\":[],\"mock_reply\":[]}}"
    );
    assert_eq!(
        check_parser_case(sibling_objects.as_bytes()),
        Some(Reject::InvalidSchema)
    );
    let nonempty_objects = std::iter::repeat_n("{\"extra\":null}", 15)
        .collect::<Vec<_>>()
        .join(",");
    let sibling_nonempty = format!(
        "{{\"schema_version\":1,\"mock_read_records\":[{nonempty_objects}],\"mock_write_i64\":[],\"mock_request\":[],\"mock_reply\":[]}}"
    );
    assert_eq!(
        check_parser_case(sibling_nonempty.as_bytes()),
        Some(Reject::InvalidSchema)
    );
    let nonempty_read_array = std::iter::repeat_n(r#"{"ok":[{"ok":"1"}]}"#, 18)
        .collect::<Vec<_>>()
        .join(",");
    let many_arrays = format!(
        "{{\"schema_version\":1,\"mock_read_records\":[{nonempty_read_array}],\"mock_write_i64\":[],\"mock_request\":[],\"mock_reply\":[]}}"
    );
    assert_eq!(check_parser_case(many_arrays.as_bytes()), None);

    let mut at_size_limit = EMPTY_ENV.to_vec();
    at_size_limit.resize(1_048_576, b' ');
    assert_eq!(check_parser_case(&at_size_limit), None);
    at_size_limit.push(b' ');
    assert_eq!(check_parser_case(&at_size_limit), Some(Reject::TooLarge));
    assert_eq!(
        check_parser_case(&[0xff]),
        Some(Reject::InvalidJson),
        "UTF-8 validation precedes JSON decoding"
    );
}

fn unicode_escape_document(escapes: &[String], in_key: bool) -> Vec<u8> {
    let mut document = String::from(r#"{"schema_version":1,"mock_read_records":["#);
    for (index, escape) in escapes.iter().enumerate() {
        if index != 0 {
            document.push(',');
        }
        if in_key {
            document.push_str("{\"");
            document.push_str(escape);
            document.push_str("\":null}");
        } else {
            document.push_str("{\"ok\":\"x");
            document.push_str(escape);
            document.push_str("\"}");
        }
    }
    document.push_str(r#"],"mock_write_i64":[],"mock_request":[],"mock_reply":[]}"#);
    document.into_bytes()
}

fn assert_unicode_escape_batch(escapes: &[String], in_key: bool, expected: Reject) {
    let document = unicode_escape_document(escapes, in_key);
    assert_eq!(
        check_parser_case(&document),
        Some(expected),
        "key={in_key} batch_len={} first={:?} last={:?}",
        escapes.len(),
        escapes.first(),
        escapes.last()
    );
}

#[test]
fn every_unicode_code_unit_escape_has_the_contract_rejection_phase_in_keys_and_values() {
    const BATCH: usize = 20_000;
    let mut scalar_batch = Vec::with_capacity(BATCH);
    let mut scalar_units = 0usize;
    let mut batch_count = 0usize;

    for code_unit in 0u32..=0xffff {
        if (0xd800..=0xdfff).contains(&code_unit) {
            continue;
        }
        scalar_batch.push(format!(r"\u{code_unit:04X}"));
        scalar_units += 1;
        if scalar_batch.len() == BATCH {
            assert_unicode_escape_batch(&scalar_batch, true, Reject::InvalidSchema);
            assert_unicode_escape_batch(&scalar_batch, false, Reject::InvalidSchema);
            batch_count += 2;
            scalar_batch.clear();
        }
    }
    if !scalar_batch.is_empty() {
        assert_unicode_escape_batch(&scalar_batch, true, Reject::InvalidSchema);
        assert_unicode_escape_batch(&scalar_batch, false, Reject::InvalidSchema);
        batch_count += 2;
    }

    let mut lone_surrogates = 0usize;
    for code_unit in 0xd800u32..=0xdfff {
        let escape = format!(r"\u{code_unit:04X}");
        for in_key in [true, false] {
            assert_unicode_escape_batch(std::slice::from_ref(&escape), in_key, Reject::InvalidJson);
            lone_surrogates += 1;
            batch_count += 1;
        }
    }

    let mut pair_batch = Vec::with_capacity(BATCH);
    let mut surrogate_pairs = 0usize;
    for high in 0xd800u32..=0xdbff {
        for low in 0xdc00u32..=0xdfff {
            pair_batch.push(format!(r"\u{high:04X}\u{low:04X}"));
            surrogate_pairs += 1;
            if pair_batch.len() == BATCH {
                assert_unicode_escape_batch(&pair_batch, true, Reject::InvalidSchema);
                assert_unicode_escape_batch(&pair_batch, false, Reject::InvalidSchema);
                batch_count += 2;
                pair_batch.clear();
            }
        }
    }
    if !pair_batch.is_empty() {
        assert_unicode_escape_batch(&pair_batch, true, Reject::InvalidSchema);
        assert_unicode_escape_batch(&pair_batch, false, Reject::InvalidSchema);
        batch_count += 2;
    }

    assert_eq!(scalar_units, 63_488);
    assert_eq!(lone_surrogates, 4_096);
    assert_eq!(surrogate_pairs, 1_048_576);
    eprintln!(
        "unicode_escape_rejection_matrix scalar_units={scalar_units} lone_surrogate_cases={lone_surrogates} surrogate_pairs={surrogate_pairs} pair_key_value_cases={} batches={batch_count}",
        surrogate_pairs * 2
    );
}

fn environment_with_raw_slot(slot: usize, raw: &str) -> String {
    let version = if slot == 0 { raw } else { "1" };
    let read = match slot {
        1 => format!(r#"[{{"ok":{raw}}}]"#),
        2 => format!(r#"[{{"err":{raw}}}]"#),
        3 => format!(r#"[{{"ok":[{{"ok":{raw}}}]}}]"#),
        4 => format!(r#"[{{"ok":[{{"err":{raw}}}]}}]"#),
        _ => "[]".to_owned(),
    };
    let write = match slot {
        5 => format!("[{{\"ok\":{raw}}}]"),
        6 => format!("[{{\"err\":{raw}}}]"),
        _ => "[]".to_owned(),
    };
    let request = match slot {
        7 => format!("[{{\"ok\":{raw}}}]"),
        8 => format!("[{{\"err\":{raw}}}]"),
        _ => "[]".to_owned(),
    };
    let reply = match slot {
        9 => format!("[{{\"ok\":{raw}}}]"),
        10 => format!("[{{\"err\":{raw}}}]"),
        _ => "[]".to_owned(),
    };
    format!(
        "{{\"schema_version\":{version},\"mock_read_records\":{read},\"mock_write_i64\":{write},\"mock_request\":{request},\"mock_reply\":{reply}}}"
    )
}

#[test]
fn mock_oracle_covers_non_string_literals_and_canonical_i64_at_every_slot() {
    for slot in 0..11 {
        for literal in ["false", "true", "null", "0", "-1", "1.0"] {
            let input = environment_with_raw_slot(slot, literal);
            let should_accept =
                (slot == 0 && literal == "1") || (matches!(slot, 5 | 9) && literal == "null");
            assert_eq!(
                check_parser_case(input.as_bytes()),
                (!should_accept).then_some(Reject::InvalidSchema),
                "non-string literal {literal} at schema slot {slot}: {input}"
            );
        }
    }

    let integer_slots = [2usize, 3, 4, 6, 7, 8, 10];
    let invalid = [
        "+5",
        "+0",
        "-0",
        "00",
        "01",
        "-01",
        "５",
        "١",
        "9223372036854775808",
        "-9223372036854775809",
    ];
    let valid = [
        "0",
        "1",
        "-1",
        "9223372036854775807",
        "-9223372036854775808",
    ];
    for slot in integer_slots {
        for candidate in invalid {
            let input = environment_with_raw_slot(slot, &format!("{candidate:?}"));
            assert_eq!(
                check_parser_case(input.as_bytes()),
                Some(Reject::InvalidSchema),
                "invalid I64 {candidate:?} at schema slot {slot}"
            );
        }
        for candidate in valid {
            let input = environment_with_raw_slot(slot, &format!("{candidate:?}"));
            assert_eq!(
                check_parser_case(input.as_bytes()),
                None,
                "canonical I64 {candidate} at schema slot {slot}"
            );
        }
    }
}

#[test]
fn mock_oracle_parser_arbitrary_byte_fuzz_never_panics_or_disagrees() {
    let seed = std::env::var("NEXO_MOCK_JSON_FUZZ_SEED")
        .ok()
        .and_then(|value| value.parse::<u64>().ok())
        .unwrap_or(0x6a09_e667_f3bc_c909);
    let cases = std::env::var("NEXO_MOCK_JSON_FUZZ_CASES")
        .ok()
        .and_then(|value| value.parse::<usize>().ok())
        .unwrap_or(2_048);
    let mut rng = SplitMix64(seed);
    for _ in 0..cases {
        let length = (rng.next() % 512) as usize;
        let input = (0..length).map(|_| rng.next() as u8).collect::<Vec<_>>();
        let _ = check_parser_case(&input);
    }
}

#[test]
fn mock_oracle_compares_indexed_read_record_values_in_order() {
    let input = br#"{"schema_version":1,"mock_read_records":[{"ok":[{"ok":"7"},{"ok":"8"},{"err":"9"}]}],"mock_write_i64":[],"mock_request":[],"mock_reply":[]}"#;
    let source = "fn Main()->Result<(),I64>{let a=MockReadRecords(MockOpen(1))?;Print(Len(a));Print(a[0]);Print(a[1]);Print(a[2]);return Ok(());}";
    let actual = real_adapter(input, Some(source), ReducedExecutionProfile::Normal)
        .unwrap()
        .unwrap();
    use nexo::interp::observation::Channel;
    let expected_observations: Vec<(Channel, String, String)> = vec![
        (Channel::Event, "open:0:1".into(), "mock-resource:".into()),
        (
            Channel::Event,
            "read_records:0:ok:3".into(),
            "mock-io:".into(),
        ),
        (Channel::Event, "close:0:1".into(), "mock-resource:".into()),
        (Channel::Stdout, "3".into(), String::new()),
        (Channel::Stdout, "Ok(7)".into(), String::new()),
        (Channel::Stdout, "Ok(8)".into(), String::new()),
        (Channel::Stdout, "Err(9)".into(), String::new()),
    ];
    let expected_bytes = expected_observations
        .iter()
        .map(|(_, text, prefix)| (text.len() + prefix.len() + 1) as u64)
        .sum::<u64>();
    assert_eq!(actual.outcome, Outcome::Ok);
    assert_eq!(actual.exit_code, 0);
    assert_eq!(actual.stdout, ["3", "Ok(7)", "Ok(8)", "Err(9)"]);
    assert_eq!(
        actual.events,
        ["open:0:1", "read_records:0:ok:3", "close:0:1"]
    );
    assert_eq!(actual.observations, expected_observations);
    assert_eq!(actual.bytes, expected_bytes);
    assert_eq!(actual.reserved, 0);
    assert!(actual.diagnostics.is_empty());
    assert!(actual.json_diagnostics.is_empty());
}

#[test]
fn mock_oracle_all_primitives_cover_i64_extremes_and_both_queue_results() {
    let extremes = [i64::MIN, 0, i64::MAX];
    let reads = vec![
        Ok(vec![Ok(i64::MIN), Err(0), Ok(i64::MAX)]),
        Err(i64::MIN),
        Ok(Vec::new()),
    ];
    let writes = vec![Ok(()), Err(i64::MIN), Ok(())];
    let requests = vec![Ok(i64::MIN), Err(0), Ok(i64::MAX)];
    let replies = vec![Ok(()), Err(i64::MAX), Ok(())];
    let mut source = String::from("fn Main(){");
    let mut operations = Vec::new();
    for (index, value) in extremes.iter().enumerate() {
        source.push_str(&format!(
            "let rows_{index}=MockReadRecords(MockOpen({value}));MockWriteI64({value});MockRequest();MockReply({value});"
        ));
        operations.extend([
            Operation::Read { label: *value },
            Operation::Write { number: *value },
            Operation::Request,
            Operation::Reply { status: *value },
        ]);
    }
    source.push('}');
    let json = generated_environment(&reads, &writes, &requests, &replies);
    let mut expected_env = reference_environment(json.as_bytes()).unwrap();
    let expected = reference_run(&operations, &mut expected_env);
    let actual = real_adapter(
        json.as_bytes(),
        Some(&source),
        ReducedExecutionProfile::Normal,
    )
    .unwrap()
    .unwrap();
    let empty = real_adapter(EMPTY_ENV, Some(&source), ReducedExecutionProfile::Normal)
        .unwrap()
        .unwrap();
    compare_runtime(&expected, &actual, empty.work);
    assert_eq!(actual.stdout, [i64::MIN.to_string(), i64::MAX.to_string()]);
}

#[test]
#[ignore = "probe no normativo: A2 no ordena el agotamiento simultáneo de trabajo y celdas"]
fn mock_oracle_work_cells_priority_probe() {
    let source = "fn Main(){var i=0;while i<71425{i=i+1;}let rows=MockReadRecords(MockOpen(1));}";
    let records = std::iter::repeat_n(r#"{"ok":"1"}"#, 40)
        .collect::<Vec<_>>()
        .join(",");
    let environment = format!(
        r#"{{"schema_version":1,"mock_read_records":[{{"ok":[{records}]}}],"mock_write_i64":[],"mock_request":[],"mock_reply":[]}}"#
    );
    let environment = MockEnvironment::parse(environment.as_bytes()).unwrap();
    let cell_only_program = nexo::ir::compile_scalar_with_profile(
        "work-cells-probe.n",
        "fn Main(){let rows=MockReadRecords(MockOpen(1));}",
        nexo::ir::ReducedProfile::Cells64,
        true,
    )
    .unwrap();
    let cell_only = nexo::interp::run_with_limits(
        &cell_only_program,
        environment.clone(),
        ReducedExecutionProfile::Cells64,
    );
    assert_eq!(cell_only.diagnostics[0].code, "R0002");

    let program = nexo::ir::compile_scalar_with_profile(
        "work-cells-probe.n",
        source,
        nexo::ir::ReducedProfile::Cells64,
        true,
    )
    .unwrap();
    let execution =
        nexo::interp::run_with_limits(&program, environment, ReducedExecutionProfile::Cells64);
    assert_eq!(execution.outcome, Outcome::SimulatorFault);
    let code = execution.diagnostics[0].code.as_str();
    assert!(matches!(code, "R0001" | "R0002"), "{code}");
    let json = execution.diagnostics[0]
        .to_json_view(
            "work-cells-probe.n",
            nexo::diagnostic::SourceView {
                bytes_len: source.len(),
                text: Some(source),
            },
        )
        .unwrap();
    // A2 §11.6, ll. 1062–1064: compare every diagnostic field for either
    // independently failing budget, without selecting an order for both.
    let expected_json = match code {
        "R0001" => {
            r#"{"schema_version":1,"diagnostic_namespace":"F0-0002-sim","profile_id":"f0-0002-b-test-cells-64-v2","code":"R0001","subcause":null,"rule":"R0001","explanation_id":null,"severity":"error","stage":"runtime","message":"Fallo del simulador.","operation":"runtime.limit","primary_span":{"file":"work-cells-probe.n","start":48,"end":76},"related_spans":[],"cause_chain":[],"feature_id":null,"unexpected_token":null,"expected_tokens":[],"expected_tokens_kind":null,"parser_context":null,"recognized_prefix":null,"repair":[],"data":{"limit":1000000,"observed":1000001,"work_consumed":1000000}}"#
        }
        "R0002" => {
            r#"{"schema_version":1,"diagnostic_namespace":"F0-0002-sim","profile_id":"f0-0002-b-test-cells-64-v2","code":"R0002","subcause":null,"rule":"R0002","explanation_id":null,"severity":"error","stage":"runtime","message":"Fallo del simulador.","operation":"runtime.limit","primary_span":{"file":"work-cells-probe.n","start":48,"end":76},"related_spans":[],"cause_chain":[],"feature_id":null,"unexpected_token":null,"expected_tokens":[],"expected_tokens_kind":null,"parser_context":null,"recognized_prefix":null,"repair":[],"data":{"limit":64,"observed":84,"runtime_budget_kind":"live_value_cells"}}"#
        }
        _ => unreachable!("validated runtime budget code"),
    };
    assert_eq!(json, expected_json);
    eprintln!(
        "work_cells_priority_probe={code} fields={:?} json={json}",
        execution.diagnostics[0].data
    );
}

#[test]
fn mock_oracle_runtime_contract() {
    // D15 §2.3, ll. 96–99: arrays carry a wrapper,
    // each record carries wrapper+scalar; the exact source runs 90 empty reads.
    let read_source = "fn Main()->Result<(),I64>{var i=0;while i<90{let rows=MockReadRecords(MockOpen(i))?;i=i+1;}return Ok(());} ";
    let read_json = format!(
        "{{\"schema_version\":1,\"mock_read_records\":[{}],\"mock_write_i64\":[],\"mock_request\":[],\"mock_reply\":[]}}",
        std::iter::repeat_n("{\"ok\":[]}", 90)
            .collect::<Vec<_>>()
            .join(",")
    );
    let mut read_model = reference_environment(read_json.as_bytes()).unwrap();
    let read_expected = reference_run(
        &(0..90)
            .map(|index| Operation::Read {
                label: index as i64,
            })
            .collect::<Vec<_>>(),
        &mut read_model,
    );
    let read_actual = real_adapter(
        read_json.as_bytes(),
        Some(read_source),
        ReducedExecutionProfile::Cells64,
    )
    .unwrap()
    .unwrap();
    assert_eq!(read_actual.outcome, Outcome::Ok);
    assert_eq!(read_actual.observations, read_expected.observations);
    assert_eq!(read_actual.bytes, read_expected.bytes);
    assert_eq!(read_actual.reserved, 0);
    assert!(read_actual.diagnostics.is_empty());
    assert_eq!(read_actual.observations.len(), 270);

    let copy_source = format!(
        "fn Main()->Result<(),I64>{{let big=[{}];let rows=MockReadRecords(MockOpen(1))?;let copy=rows;AssertEq(Len(rows),1);AssertEq(Len(copy),1);return Ok(());}}",
        std::iter::repeat_n("0", 55).collect::<Vec<_>>().join(",")
    );
    let copy_environment = br#"{"schema_version":1,"mock_read_records":[{"ok":[{"ok":"1"}]}],"mock_write_i64":[],"mock_request":[],"mock_reply":[]}"#;
    let copied_array = real_adapter(
        copy_environment,
        Some(&copy_source),
        ReducedExecutionProfile::Cells64,
    )
    .unwrap()
    .unwrap();
    assert_eq!(copied_array.outcome, Outcome::Ok);
    assert!(copied_array.diagnostics.is_empty());
    assert_eq!(
        copied_array.observations,
        [
            (
                nexo::interp::observation::Channel::Event,
                "open:0:1".to_owned(),
                "mock-resource:".to_owned()
            ),
            (
                nexo::interp::observation::Channel::Event,
                "read_records:0:ok:1".to_owned(),
                "mock-io:".to_owned()
            ),
            (
                nexo::interp::observation::Channel::Event,
                "close:0:1".to_owned(),
                "mock-resource:".to_owned()
            ),
        ]
    );
    let exact_copy_source = format!(
        "fn Main()->Result<(),I64>{{let big=[{}];let resource=MockOpen(1);let rows=MockReadRecords(resource)?;let copy=rows;Drop(copy);Drop(rows);return Ok(());}}",
        std::iter::repeat_n("0", 60).collect::<Vec<_>>().join(",")
    );
    let exact_copy = real_adapter(
        br#"{"schema_version":1,"mock_read_records":[{"ok":[]}],"mock_write_i64":[],"mock_request":[],"mock_reply":[]}"#,
        Some(&exact_copy_source),
        ReducedExecutionProfile::Cells64,
    )
    .unwrap()
    .unwrap();
    assert_eq!(exact_copy.outcome, Outcome::Ok);
    assert!(exact_copy.diagnostics.is_empty());
    let twice_copied_source = format!(
        "fn Main()->Result<(),I64>{{let big=[{}];let rows=MockReadRecords(MockOpen(1))?;let first=rows;let second=rows;AssertEq(Len(rows),1);AssertEq(Len(first),1);AssertEq(Len(second),1);return Ok(());}}",
        std::iter::repeat_n("0", 55).collect::<Vec<_>>().join(",")
    );
    let twice_copied = real_adapter(
        copy_environment,
        Some(&twice_copied_source),
        ReducedExecutionProfile::Cells64,
    )
    .unwrap()
    .unwrap();
    assert_eq!(twice_copied.outcome, Outcome::SimulatorFault);
    assert_eq!(twice_copied.diagnostics, ["R0002"]);
    assert_eq!(
        twice_copied.diagnostic_details[0].data["observed"],
        nexo::diagnostic::DataValue::UInt(65)
    );

    // A2 §11.6, ll. 1042, 1026 and 1062: after factoring the shared open,
    // the mock read (2 call-entry + 4 materialization units for one record)
    // costs one more work unit than constructing [Ok(1)] (4 for Ok + 1 header).
    // Each later copy charges the same three array cells in both programs.
    let equivalent_arrays = |big_len: usize, copies: usize| {
        let big = std::iter::repeat_n("0", big_len)
            .collect::<Vec<_>>()
            .join(",");
        let make_source = |from_mock: bool| {
            let mut source =
                format!("fn Main()->Result<(),I64>{{let big=[{big}];let resource=MockOpen(7);");
            if from_mock {
                source.push_str("let rows=MockReadRecords(resource)?;");
            } else {
                source.push_str("let rows:Array<Result<I64,I64>>=[Ok(1)];");
            }
            for index in 0..copies {
                source.push_str(&format!("var copy_{index}=rows;copy_{index}[0]=Ok(2);"));
            }
            source.push_str("AssertEq(rows[0]?,1);");
            for index in 0..copies {
                source.push_str(&format!("AssertEq(copy_{index}[0]?,2);"));
            }
            source.push_str("return Ok(());}");
            source
        };
        let mock = real_adapter(
            br#"{"schema_version":1,"mock_read_records":[{"ok":[{"ok":"1"}]}],"mock_write_i64":[],"mock_request":[],"mock_reply":[]}"#,
            Some(&make_source(true)),
            ReducedExecutionProfile::Cells64,
        )
        .unwrap()
        .unwrap();
        let literal = real_adapter(
            EMPTY_ENV,
            Some(&make_source(false)),
            ReducedExecutionProfile::Cells64,
        )
        .unwrap()
        .unwrap();
        assert_eq!(
            mock.outcome, literal.outcome,
            "big={big_len} copies={copies}"
        );
        assert_eq!(
            mock.diagnostics, literal.diagnostics,
            "big={big_len} copies={copies}"
        );
        if mock.outcome == Outcome::Ok {
            assert_eq!(mock.work, literal.work + 1, "big={big_len} copies={copies}");
        }
    };
    for big_len in 1..=61 {
        for copies in 1..=20 {
            equivalent_arrays(big_len, copies);
        }
    }

    let assert_two_cell_result = |array_len: usize, operation: &str, environment: &[u8]| {
        let source = format!(
            "fn Main(){{let big=[{}];{operation}}}",
            std::iter::repeat_n("0", array_len)
                .collect::<Vec<_>>()
                .join(",")
        );
        let actual = real_adapter(environment, Some(&source), ReducedExecutionProfile::Cells64)
            .unwrap()
            .unwrap();
        assert_eq!(actual.outcome, Outcome::SimulatorFault, "{operation}");
        let diagnostic = &actual.diagnostic_details[0];
        assert_eq!(diagnostic.code, "R0002", "{operation}");
        assert_eq!(
            diagnostic.data["runtime_budget_kind"],
            nexo::diagnostic::DataValue::String("live_value_cells".into()),
            "{operation}"
        );
        assert_eq!(
            diagnostic.data["limit"],
            nexo::diagnostic::DataValue::UInt(64)
        );
        assert_eq!(
            diagnostic.data["observed"],
            nexo::diagnostic::DataValue::UInt(65)
        );
        assert!(actual.observations.is_empty());
        assert_eq!(actual.reserved, 0);
    };
    let request_cells_env = br#"{"schema_version":1,"mock_read_records":[],"mock_write_i64":[],"mock_request":[{"ok":"1"}],"mock_reply":[]}"#;
    let write_cells_env = br#"{"schema_version":1,"mock_read_records":[],"mock_write_i64":[{"ok":null}],"mock_request":[],"mock_reply":[]}"#;
    let reply_cells_env = br#"{"schema_version":1,"mock_read_records":[],"mock_write_i64":[],"mock_request":[],"mock_reply":[{"ok":null}]}"#;
    assert_two_cell_result(62, "MockRequest();", request_cells_env);
    assert_two_cell_result(62, "MockWriteI64(25);", write_cells_env);
    assert_two_cell_result(62, "MockReply(200);", reply_cells_env);

    // A2 §11.6, l. 1062: read Err reserves two result cells, not one.
    let big = std::iter::repeat_n("0", 61).collect::<Vec<_>>().join(",");
    let read_err_source =
        format!("fn Main(){{let big=[{big}];let rows=MockReadRecords(MockOpen(0));}}");
    let read_err_environment = br#"{"schema_version":1,"mock_read_records":[{"err":"7"}],"mock_write_i64":[],"mock_request":[],"mock_reply":[]}"#;
    let read_err_boundary = real_adapter(
        read_err_environment,
        Some(&read_err_source),
        ReducedExecutionProfile::Cells64,
    )
    .unwrap()
    .unwrap();
    assert_eq!(read_err_boundary.outcome, Outcome::SimulatorFault);
    assert_eq!(read_err_boundary.diagnostics, ["R0002"]);
    assert_eq!(
        read_err_boundary.diagnostic_details[0].data["observed"],
        nexo::diagnostic::DataValue::UInt(65)
    );

    let records_source = "fn Main()->Result<(),I64>{let rows=MockReadRecords(MockOpen(3))?;let good=rows[0]?;AssertEq(good,17);return Ok(());} ";
    let records_env = br#"{"schema_version":1,"mock_read_records":[{"ok":[{"ok":"17"}]}],"mock_write_i64":[],"mock_request":[],"mock_reply":[]}"#;
    let records_execution = real_adapter(
        records_env,
        Some(records_source),
        ReducedExecutionProfile::Normal,
    )
    .unwrap()
    .unwrap();
    assert_eq!(records_execution.outcome, Outcome::Ok);
    assert_eq!(
        records_execution.observations,
        [
            (
                nexo::interp::observation::Channel::Event,
                "open:0:3".to_owned(),
                "mock-resource:".to_owned()
            ),
            (
                nexo::interp::observation::Channel::Event,
                "read_records:0:ok:1".to_owned(),
                "mock-io:".to_owned()
            ),
            (
                nexo::interp::observation::Channel::Event,
                "close:0:3".to_owned(),
                "mock-resource:".to_owned()
            ),
        ]
    );
    let record_error_source = "fn Main()->Result<(),I64>{let rows=MockReadRecords(MockOpen(3))?;let value=rows[0]?;return Ok(());} ";
    let record_error_env = br#"{"schema_version":1,"mock_read_records":[{"ok":[{"err":"-3"}]}],"mock_write_i64":[],"mock_request":[],"mock_reply":[]}"#;
    let record_error = real_adapter(
        record_error_env,
        Some(record_error_source),
        ReducedExecutionProfile::Normal,
    )
    .unwrap()
    .unwrap();
    assert_eq!(record_error.outcome, Outcome::Err(-3));
    assert_eq!(exit_code(&record_error.outcome), 1);
    assert_eq!(record_error.observations[1].1, "read_records:0:ok:1");

    // A2 §12.4, «Eventos exactos», l. 1548, y codificación virtual, ll. 1578–1584:
    // conservar el signo, el prefijo mock-io: y un LF en los bytes virtuales.
    let request_source = "fn Main()->Result<(),I64>{let value=MockRequest()?;return Ok(());} ";
    let request_json = br#"{"schema_version":1,"mock_read_records":[],"mock_write_i64":[],"mock_request":[{"ok":"-5"}],"mock_reply":[]}"#;
    let actual = real_adapter(
        request_json,
        Some(request_source),
        ReducedExecutionProfile::Normal,
    )
    .unwrap()
    .unwrap();
    assert_eq!(actual.outcome, Outcome::Ok);
    assert_eq!(
        actual.observations,
        [(
            nexo::interp::observation::Channel::Event,
            "request:ok:-5".to_owned(),
            "mock-io:".to_owned()
        )]
    );
    assert_eq!(actual.bytes, 22);
    assert_eq!(actual.reserved, 0);
    assert_eq!(exit_code(&actual.outcome), 0);

    let escaped_digit_source =
        "fn Main()->Result<(),I64>{let value=MockRequest()?;return Ok(());} ";
    let escaped_digit_env = br#"{"schema_version":1,"mock_read_records":[],"mock_write_i64":[],"mock_request":[{"ok":"\u0031"}],"mock_reply":[]}"#;
    let escaped_digit = real_adapter(
        escaped_digit_env,
        Some(escaped_digit_source),
        ReducedExecutionProfile::Normal,
    )
    .unwrap()
    .unwrap();
    assert_eq!(escaped_digit.outcome, Outcome::Ok);
    assert_eq!(escaped_digit.observations[0].1, "request:ok:1");
    assert_eq!(escaped_digit.bytes, 21);

    // Exhaustion uses Err(-9000) independently per queue and retains request
    // order even when successful and failed calls are interleaved.
    let source = "fn Main(){let rows=MockReadRecords(MockOpen(4));MockWriteI64(-5);MockRequest();MockReply(404);}";
    let operations = [
        Operation::Read { label: 4 },
        Operation::Write { number: -5 },
        Operation::Request,
        Operation::Reply { status: 404 },
    ];
    let empty_expected = reference_run(&operations, &mut ReferenceEnvironment::default());
    let execution = real_adapter(EMPTY_ENV, Some(source), ReducedExecutionProfile::Normal)
        .unwrap()
        .unwrap();
    assert_eq!(execution.outcome, empty_expected.outcome);
    assert_eq!(execution.observations, empty_expected.observations);
    assert_eq!(execution.bytes, empty_expected.bytes);
    assert!(
        execution
            .observations
            .iter()
            .all(|(channel, _, _)| { *channel == nexo::interp::observation::Channel::Event })
    );
    assert_eq!(execution.reserved, 0);

    let read_fifo_source = "fn Main(){let first=MockReadRecords(MockOpen(1));let second=MockReadRecords(MockOpen(2));}";
    let read_fifo_json = br#"{"schema_version":1,"mock_read_records":[{"ok":[]},{"err":"7"}],"mock_write_i64":[],"mock_request":[],"mock_reply":[]}"#;
    let mut read_fifo_model = reference_environment(read_fifo_json).unwrap();
    let read_fifo_expected = reference_run(
        &[Operation::Read { label: 1 }, Operation::Read { label: 2 }],
        &mut read_fifo_model,
    );
    let read_fifo_actual = real_adapter(
        read_fifo_json,
        Some(read_fifo_source),
        ReducedExecutionProfile::Normal,
    )
    .unwrap()
    .unwrap();
    assert_eq!(
        read_fifo_actual.observations,
        read_fifo_expected.observations
    );
    assert_eq!(read_fifo_actual.bytes, read_fifo_expected.bytes);

    let scalar_fifo_source = "fn Main(){MockWriteI64(25);MockWriteI64(3);MockRequest();MockRequest();MockReply(200);MockReply(201);}";
    let scalar_fifo_json = br#"{"schema_version":1,"mock_read_records":[],"mock_write_i64":[{"ok":null},{"err":"4"}],"mock_request":[{"ok":"0"},{"err":"9"}],"mock_reply":[{"ok":null},{"err":"5"}]}"#;
    let mut scalar_fifo_model = reference_environment(scalar_fifo_json).unwrap();
    let scalar_fifo_expected = reference_run(
        &[
            Operation::Write { number: 25 },
            Operation::Write { number: 3 },
            Operation::Request,
            Operation::Request,
            Operation::Reply { status: 200 },
            Operation::Reply { status: 201 },
        ],
        &mut scalar_fifo_model,
    );
    let scalar_fifo_actual = real_adapter(
        scalar_fifo_json,
        Some(scalar_fifo_source),
        ReducedExecutionProfile::Normal,
    )
    .unwrap()
    .unwrap();
    assert_eq!(scalar_fifo_actual.outcome, scalar_fifo_expected.outcome);
    assert_eq!(
        scalar_fifo_actual.observations,
        scalar_fifo_expected.observations
    );
    assert_eq!(scalar_fifo_actual.bytes, scalar_fifo_expected.bytes);
    assert_eq!(
        exit_code(&scalar_fifo_actual.outcome),
        scalar_fifo_expected.exit_code
    );

    // A2 §11.6, ll. 1062–1064: read Err and request/reply charge two result
    // units; write charges two plus one only for successful stdout formatting.
    let work = |source: &str, environment: &[u8]| {
        real_adapter(environment, Some(source), ReducedExecutionProfile::Normal)
            .unwrap()
            .unwrap()
            .work
    };
    let request_work = work("fn Main(){MockRequest();}", EMPTY_ENV);
    let write_err_work = work("fn Main(){MockWriteI64(0);}", EMPTY_ENV);
    let write_ok_env = br#"{"schema_version":1,"mock_read_records":[],"mock_write_i64":[{"ok":null}],"mock_request":[],"mock_reply":[]}"#;
    let write_ok_work = work("fn Main(){MockWriteI64(0);}", write_ok_env);
    let reply_err_work = work("fn Main(){MockReply(0);}", EMPTY_ENV);
    let reply_ok_env = br#"{"schema_version":1,"mock_read_records":[],"mock_write_i64":[],"mock_request":[],"mock_reply":[{"ok":null}]}"#;
    let reply_ok_work = work("fn Main(){MockReply(0);}", reply_ok_env);
    let read_err_work = work("fn Main(){MockReadRecords(MockOpen(0));}", EMPTY_ENV);
    let read_ok_empty_env = br#"{"schema_version":1,"mock_read_records":[{"ok":[]}],"mock_write_i64":[],"mock_request":[],"mock_reply":[]}"#;
    let read_ok_empty_work = work(
        "fn Main(){MockReadRecords(MockOpen(0));}",
        read_ok_empty_env,
    );
    let read_ok_two_env = br#"{"schema_version":1,"mock_read_records":[{"ok":[{"ok":"1"},{"err":"2"}]}],"mock_write_i64":[],"mock_request":[],"mock_reply":[]}"#;
    let read_ok_two_work = work("fn Main(){MockReadRecords(MockOpen(0));}", read_ok_two_env);
    assert_eq!(write_err_work, request_work + 2);
    assert_eq!(write_ok_work, request_work + 3);
    assert_eq!(reply_err_work, request_work + 2);
    assert_eq!(reply_ok_work, request_work + 2);
    assert_eq!(read_err_work, request_work + 6);
    assert_eq!(read_ok_empty_work, request_work + 6);
    assert_eq!(read_ok_two_work, read_ok_empty_work + 4);
}

#[test]
#[ignore = "seeded independent I11a parser/runtime differential campaign"]
fn mock_oracle_differential_campaign() {
    let cases = std::env::var("NEXO_ORACLE_CASES")
        .expect("campaign case count")
        .parse::<usize>()
        .expect("integer case count");
    let seed = std::env::var("NEXO_ORACLE_SEED")
        .expect("campaign seed")
        .parse::<u64>()
        .expect("integer campaign seed");
    assert!(cases >= 100_000);
    let mut rng = SplitMix64(seed);
    const PARSER_CATEGORIES: usize = 128;
    let mut parser_coverage = [0usize; PARSER_CATEGORIES];
    let mut rejected = [0usize; 3];

    let mut over_limit = EMPTY_ENV.to_vec();
    over_limit.resize(1_048_577, b' ');
    assert_eq!(check_parser_case(&over_limit), Some(Reject::TooLarge));
    rejected[0] += 1;

    // Parser-only generated cases exercise every validation class; runtime
    // inputs below independently drive one complete mixed queue schedule.
    for case in 0..cases {
        let input = parser_frontier(case, &mut rng);
        parser_coverage[case % PARSER_CATEGORIES] += 1;
        if let Some(reason) = check_parser_case(&input) {
            rejected[match reason {
                Reject::TooLarge => 0,
                Reject::InvalidJson => 1,
                Reject::InvalidSchema => 2,
            }] += 1;
        }
    }
    assert!(
        parser_coverage
            .iter()
            .all(|count| *count >= cases / PARSER_CATEGORIES)
    );
    assert!(
        rejected.iter().all(|count| *count > 0),
        "rejection coverage={rejected:?}"
    );
    let file_open_categories = campaign_file_open_categories();

    // A2 §§11.2/11.6: the loop reaches the final granted work unit and the
    // successful write must fail before its event batch commits.
    let mut work_source = String::from("fn Main()->Result<(),I64>{var i=0;while i<71425{i=i+1;}");
    for slot in 0..7 {
        work_source.push_str(&format!("let z{slot}=1;"));
    }
    work_source.push_str("MockWriteI64(25)?;return Ok(());}");
    let work_environment = br#"{"schema_version":1,"mock_read_records":[],"mock_write_i64":[{"ok":null}],"mock_request":[],"mock_reply":[]}"#;
    let work_boundary = real_adapter(
        work_environment,
        Some(&work_source),
        ReducedExecutionProfile::Normal,
    )
    .unwrap()
    .unwrap();
    assert_eq!(work_boundary.outcome, Outcome::SimulatorFault);
    assert_eq!(work_boundary.diagnostics, ["R0001"]);
    assert_eq!(
        work_boundary.diagnostic_details[0].data["observed"],
        nexo::diagnostic::DataValue::UInt(1_000_001)
    );
    assert_eq!(
        work_boundary.diagnostic_details[0].data["work_consumed"],
        nexo::diagnostic::DataValue::UInt(1_000_000)
    );
    assert_eq!(
        work_boundary.observations,
        [
            (
                nexo::interp::observation::Channel::Event,
                "write_i64:25:ok".to_owned(),
                "mock-io:".to_owned()
            ),
            (
                nexo::interp::observation::Channel::Stdout,
                "25".to_owned(),
                String::new()
            ),
        ]
    );
    assert_eq!(work_boundary.bytes, 27);

    let operations = (0..250)
        .flat_map(|index| {
            [
                Operation::Read {
                    label: index as i64,
                },
                Operation::Write {
                    number: index as i64,
                },
                Operation::Request,
                Operation::Reply {
                    status: (index + 1000) as i64,
                },
            ]
        })
        .collect::<Vec<_>>();
    let empty = real_adapter(
        EMPTY_ENV,
        Some(BATCH_SOURCE),
        ReducedExecutionProfile::Cells64,
    )
    .unwrap()
    .unwrap();
    let empty_model = reference_run(&operations, &mut ReferenceEnvironment::default());
    assert_eq!(empty.outcome, empty_model.outcome);
    assert_eq!(empty.observations, empty_model.observations);
    assert_eq!(empty.bytes, empty_model.bytes);
    assert_eq!(empty.reserved, 0);
    assert!(empty.diagnostics.is_empty());

    let batches = cases.div_ceil(1_000);
    for batch_index in 0..batches {
        let mut model_env = random_environment(&mut rng, batch_index);
        let json = encode_environment(&model_env);
        let expected = reference_run(&operations, &mut model_env);
        let actual = real_adapter(
            json.as_bytes(),
            Some(BATCH_SOURCE),
            ReducedExecutionProfile::Cells64,
        )
        .unwrap_or_else(|error| panic!("seed={seed} batch={batch_index} reject={error:?}"))
        .unwrap();
        compare_runtime(&expected, &actual, empty.work);

        let indexed_records = (0..250)
            .map(|_| {
                let length = (rng.next() % 13) as usize;
                (0..length)
                    .map(|_| {
                        if rng.next().is_multiple_of(4) {
                            Err(rng.integer())
                        } else {
                            Ok(rng.integer())
                        }
                    })
                    .collect::<Vec<_>>()
            })
            .collect::<Vec<_>>();
        verify_indexed_read_batch(&indexed_records);
    }
    eprintln!(
        "mock_oracle_seed={seed} cases={cases} parser_categories={parser_coverage:?} rejection_classes={rejected:?} file_open_categories={file_open_categories:?} runtime_batches={batches} indexed_read_batches={batches} work_boundary=R0001 cells_profile=cells-64"
    );
}
