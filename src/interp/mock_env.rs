use std::collections::VecDeque;

pub(crate) type Reply<T> = Result<T, i64>;

#[derive(Clone, Debug, Default)]
pub struct MockEnvironment {
    pub(crate) read_records: VecDeque<Reply<Vec<Reply<i64>>>>,
    pub(crate) write_i64: VecDeque<Reply<()>>,
    pub(crate) request: VecDeque<Reply<i64>>,
    pub(crate) reply: VecDeque<Reply<()>>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MockEnvironmentError {
    TooLarge,
    InvalidJson,
    InvalidSchema,
}

impl MockEnvironment {
    pub fn empty() -> Self {
        Self::default()
    }

    pub fn parse(bytes: &[u8]) -> Result<Self, MockEnvironmentError> {
        if bytes.len() > 1_048_576 {
            return Err(MockEnvironmentError::TooLarge);
        }
        let text = std::str::from_utf8(bytes).map_err(|_| MockEnvironmentError::InvalidJson)?;
        let value = JsonParser::new(text).parse()?;
        if has_duplicate_keys(&value) {
            return Err(MockEnvironmentError::InvalidSchema);
        }
        let fields = object(&value)?;
        if fields.len() != 5 {
            return Err(MockEnvironmentError::InvalidSchema);
        }
        let version = fields
            .iter()
            .find(|(key, _)| key == "schema_version")
            .ok_or(MockEnvironmentError::InvalidSchema)?;
        if !matches!(version.1, Json::Number(ref n) if n == "1") {
            return Err(MockEnvironmentError::InvalidSchema);
        }
        Ok(Self {
            read_records: read_queue(required(fields, "mock_read_records")?)?,
            write_i64: unit_queue(required(fields, "mock_write_i64")?)?,
            request: scalar_queue(required(fields, "mock_request")?)?,
            reply: unit_queue(required(fields, "mock_reply")?)?,
        })
    }
}

fn has_duplicate_keys(value: &Json) -> bool {
    match value {
        Json::Array(items) => items.iter().any(has_duplicate_keys),
        Json::Object(fields) => fields.iter().enumerate().any(|(index, (key, value))| {
            fields[..index].iter().any(|(prior, _)| prior == key) || has_duplicate_keys(value)
        }),
        Json::Null | Json::Bool | Json::Number(_) | Json::String(_) => false,
    }
}

#[derive(Debug)]
enum Json {
    Null,
    Bool,
    Number(String),
    String(String),
    Array(Vec<Json>),
    Object(Vec<(String, Json)>),
}

fn object(value: &Json) -> Result<&[(String, Json)], MockEnvironmentError> {
    if let Json::Object(items) = value {
        Ok(items)
    } else {
        Err(MockEnvironmentError::InvalidSchema)
    }
}

fn required<'a>(
    fields: &'a [(String, Json)],
    name: &str,
) -> Result<&'a Json, MockEnvironmentError> {
    fields
        .iter()
        .find(|(key, _)| key == name)
        .map(|(_, value)| value)
        .ok_or(MockEnvironmentError::InvalidSchema)
}

fn array(value: &Json) -> Result<&[Json], MockEnvironmentError> {
    if let Json::Array(items) = value {
        Ok(items)
    } else {
        Err(MockEnvironmentError::InvalidSchema)
    }
}

fn canonical_i64(value: &Json) -> Result<i64, MockEnvironmentError> {
    let Json::String(s) = value else {
        return Err(MockEnvironmentError::InvalidSchema);
    };
    if s.is_empty()
        || s == "-0"
        || (s.starts_with('0') && s.len() > 1)
        || (s.starts_with("-0") && s.len() > 2)
        || !s
            .strip_prefix('-')
            .unwrap_or(s)
            .bytes()
            .all(|b| b.is_ascii_digit())
    {
        return Err(MockEnvironmentError::InvalidSchema);
    }
    s.parse().map_err(|_| MockEnvironmentError::InvalidSchema)
}

fn result<T>(
    value: &Json,
    success: impl FnOnce(&Json) -> Result<T, MockEnvironmentError>,
) -> Result<Reply<T>, MockEnvironmentError> {
    let fields = object(value)?;
    if fields.len() != 1 {
        return Err(MockEnvironmentError::InvalidSchema);
    }
    match fields[0].0.as_str() {
        "ok" => success(&fields[0].1).map(Ok),
        "err" => canonical_i64(&fields[0].1).map(Err),
        _ => Err(MockEnvironmentError::InvalidSchema),
    }
}

fn scalar_queue(value: &Json) -> Result<VecDeque<Reply<i64>>, MockEnvironmentError> {
    array(value)?
        .iter()
        .map(|item| result(item, canonical_i64))
        .collect()
}

fn unit_queue(value: &Json) -> Result<VecDeque<Reply<()>>, MockEnvironmentError> {
    array(value)?
        .iter()
        .map(|item| {
            result(item, |v| {
                matches!(v, Json::Null)
                    .then_some(())
                    .ok_or(MockEnvironmentError::InvalidSchema)
            })
        })
        .collect()
}

fn read_queue(value: &Json) -> Result<VecDeque<Reply<Vec<Reply<i64>>>>, MockEnvironmentError> {
    array(value)?
        .iter()
        .map(|item| {
            result(item, |records| {
                array(records)?
                    .iter()
                    .map(|record| result(record, canonical_i64))
                    .collect()
            })
        })
        .collect()
}

struct JsonParser<'a> {
    bytes: &'a [u8],
    at: usize,
    depth: usize,
}

impl<'a> JsonParser<'a> {
    fn new(text: &'a str) -> Self {
        Self {
            bytes: text.as_bytes(),
            at: 0,
            depth: 0,
        }
    }

    fn parse(mut self) -> Result<Json, MockEnvironmentError> {
        let value = self.value()?;
        self.ws();
        if self.at != self.bytes.len() {
            return Err(MockEnvironmentError::InvalidJson);
        }
        Ok(value)
    }

    fn value(&mut self) -> Result<Json, MockEnvironmentError> {
        self.ws();
        match self.peek() {
            Some(b'{') => self.object_value(),
            Some(b'[') => self.array_value(),
            Some(b'"') => self.string().map(Json::String),
            Some(b't') => {
                self.literal(b"true")?;
                Ok(Json::Bool)
            }
            Some(b'f') => {
                self.literal(b"false")?;
                Ok(Json::Bool)
            }
            Some(b'n') => {
                self.literal(b"null")?;
                Ok(Json::Null)
            }
            Some(b'-' | b'0'..=b'9') => self.number().map(Json::Number),
            _ => Err(MockEnvironmentError::InvalidJson),
        }
    }

    fn object_value(&mut self) -> Result<Json, MockEnvironmentError> {
        self.enter()?;
        self.at += 1;
        self.ws();
        let mut fields = Vec::new();
        if self.take(b'}') {
            self.leave();
            return Ok(Json::Object(fields));
        }
        loop {
            self.ws();
            let key = self.string()?;
            self.ws();
            self.expect(b':')?;
            let value = self.value()?;
            fields.push((key, value));
            self.ws();
            if self.take(b'}') {
                break;
            }
            self.expect(b',')?;
        }
        self.leave();
        Ok(Json::Object(fields))
    }

    fn array_value(&mut self) -> Result<Json, MockEnvironmentError> {
        self.enter()?;
        self.at += 1;
        self.ws();
        let mut items = Vec::new();
        if self.take(b']') {
            self.leave();
            return Ok(Json::Array(items));
        }
        loop {
            items.push(self.value()?);
            self.ws();
            if self.take(b']') {
                break;
            }
            self.expect(b',')?;
        }
        self.leave();
        Ok(Json::Array(items))
    }

    fn enter(&mut self) -> Result<(), MockEnvironmentError> {
        if self.depth >= 16 {
            return Err(MockEnvironmentError::InvalidJson);
        }
        self.depth += 1;
        Ok(())
    }
    fn leave(&mut self) {
        self.depth -= 1;
    }
    fn string(&mut self) -> Result<String, MockEnvironmentError> {
        self.expect(b'"')?;
        let start = self.at;
        let mut escaped = false;
        while let Some(b) = self.peek() {
            if !escaped && b == b'"' {
                break;
            }
            if b < 0x20 {
                return Err(MockEnvironmentError::InvalidJson);
            }
            escaped = !escaped && b == b'\\';
            self.at += 1;
        }
        if self.peek() != Some(b'"') {
            return Err(MockEnvironmentError::InvalidJson);
        }
        let raw = std::str::from_utf8(&self.bytes[start..self.at])
            .map_err(|_| MockEnvironmentError::InvalidJson)?;
        let decoded = decode_string(raw)?;
        self.at += 1;
        Ok(decoded)
    }
    fn number(&mut self) -> Result<String, MockEnvironmentError> {
        let start = self.at;
        self.take(b'-');
        match self.peek() {
            Some(b'0') => {
                self.at += 1;
                if self.peek().is_some_and(|b| b.is_ascii_digit()) {
                    return Err(MockEnvironmentError::InvalidJson);
                }
            }
            Some(b'1'..=b'9') => {
                self.at += 1;
                while self.peek().is_some_and(|b| b.is_ascii_digit()) {
                    self.at += 1;
                }
            }
            _ => return Err(MockEnvironmentError::InvalidJson),
        }
        if self.take(b'.') {
            if !self.peek().is_some_and(|b| b.is_ascii_digit()) {
                return Err(MockEnvironmentError::InvalidJson);
            }
            while self.peek().is_some_and(|b| b.is_ascii_digit()) {
                self.at += 1;
            }
        }
        if self.peek().is_some_and(|b| matches!(b, b'e' | b'E')) {
            self.at += 1;
            if self.peek().is_some_and(|b| matches!(b, b'+' | b'-')) {
                self.at += 1;
            }
            if !self.peek().is_some_and(|b| b.is_ascii_digit()) {
                return Err(MockEnvironmentError::InvalidJson);
            }
            while self.peek().is_some_and(|b| b.is_ascii_digit()) {
                self.at += 1;
            }
        }
        let n = std::str::from_utf8(&self.bytes[start..self.at])
            .map_err(|_| MockEnvironmentError::InvalidJson)?;
        Ok(n.to_owned())
    }
    fn literal(&mut self, literal: &[u8]) -> Result<(), MockEnvironmentError> {
        if self.bytes.get(self.at..self.at + literal.len()) == Some(literal) {
            self.at += literal.len();
            Ok(())
        } else {
            Err(MockEnvironmentError::InvalidJson)
        }
    }
    fn ws(&mut self) {
        while self
            .peek()
            .is_some_and(|b| matches!(b, b' ' | b'\n' | b'\r' | b'\t'))
        {
            self.at += 1;
        }
    }
    fn peek(&self) -> Option<u8> {
        self.bytes.get(self.at).copied()
    }
    fn take(&mut self, b: u8) -> bool {
        if self.peek() == Some(b) {
            self.at += 1;
            true
        } else {
            false
        }
    }
    fn expect(&mut self, b: u8) -> Result<(), MockEnvironmentError> {
        if self.take(b) {
            Ok(())
        } else {
            Err(MockEnvironmentError::InvalidJson)
        }
    }
}

fn decode_string(raw: &str) -> Result<String, MockEnvironmentError> {
    let mut out = String::new();
    let mut chars = raw.chars();
    while let Some(c) = chars.next() {
        if c != '\\' {
            out.push(c);
            continue;
        }
        match chars.next().ok_or(MockEnvironmentError::InvalidJson)? {
            '"' => out.push('"'),
            '\\' => out.push('\\'),
            '/' => out.push('/'),
            'b' => out.push('\u{8}'),
            'f' => out.push('\u{c}'),
            'n' => out.push('\n'),
            'r' => out.push('\r'),
            't' => out.push('\t'),
            'u' => {
                let first = hex4(&mut chars)?;
                let scalar = if (0xD800..=0xDBFF).contains(&first) {
                    if chars.next() != Some('\\') || chars.next() != Some('u') {
                        return Err(MockEnvironmentError::InvalidJson);
                    }
                    let second = hex4(&mut chars)?;
                    if !(0xDC00..=0xDFFF).contains(&second) {
                        return Err(MockEnvironmentError::InvalidJson);
                    }
                    0x10000 + ((first - 0xD800) << 10) + (second - 0xDC00)
                } else {
                    first
                };
                out.push(char::from_u32(scalar).ok_or(MockEnvironmentError::InvalidJson)?);
            }
            _ => return Err(MockEnvironmentError::InvalidJson),
        }
    }
    Ok(out)
}
fn hex4(chars: &mut impl Iterator<Item = char>) -> Result<u32, MockEnvironmentError> {
    let mut value = 0;
    for _ in 0..4 {
        value = value * 16
            + chars
                .next()
                .and_then(|c| c.to_digit(16))
                .ok_or(MockEnvironmentError::InvalidJson)?;
    }
    Ok(value)
}
