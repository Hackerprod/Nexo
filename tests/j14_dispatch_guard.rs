//! Structural boundary for the full-profile production dispatcher.
use std::collections::{HashMap, HashSet};

const SOURCE: &str = include_str!("../src/frontend/parse.rs");
const MARK: &str = "// nexo:full-profile-dispatch";

#[derive(Clone, Debug)]
struct Token {
    text: String,
    value: Option<Vec<u8>>,
    at: usize,
}

fn token(text: &str, at: usize) -> Token {
    Token {
        text: text.into(),
        value: None,
        at,
    }
}

fn escaped(src: &str, mut at: usize, bytes: bool, quote: u8) -> Result<(Vec<u8>, usize), String> {
    let data = src.as_bytes();
    let mut out = Vec::new();
    let mut segment = at;
    loop {
        if at >= data.len() {
            return Err("unclosed literal".into());
        }
        if data[at] == quote {
            out.extend_from_slice(&data[segment..at]);
            return Ok((out, at + 1));
        }
        if data[at] != b'\\' {
            at += 1;
            continue;
        }
        out.extend_from_slice(&data[segment..at]);
        at += 1;
        if at >= data.len() {
            return Err("unfinished escape".into());
        }
        match data[at] {
            b'n' => {
                out.push(b'\n');
                at += 1;
            }
            b'r' => {
                out.push(b'\r');
                at += 1;
            }
            b't' => {
                out.push(b'\t');
                at += 1;
            }
            b'0' => {
                out.push(0);
                at += 1;
            }
            b'\\' => {
                out.push(b'\\');
                at += 1;
            }
            b'\'' => {
                out.push(b'\'');
                at += 1;
            }
            b'"' => {
                out.push(b'"');
                at += 1;
            }
            b'x' => {
                let hex = data.get(at + 1..at + 3).ok_or("short hex escape")?;
                let digits = std::str::from_utf8(hex).map_err(|_| "bad hex escape")?;
                let val = u8::from_str_radix(digits, 16).map_err(|_| "bad hex escape")?;
                if !bytes && val > 127 {
                    return Err("non-ASCII string hex escape".into());
                }
                out.push(val);
                at += 3;
            }
            b'u' if !bytes && data.get(at + 1) == Some(&b'{') => {
                let start = at + 2;
                let end = data[start..]
                    .iter()
                    .position(|c| *c == b'}')
                    .ok_or("open Unicode escape")?
                    + start;
                let digits = src[start..end].replace('_', "");
                let number = u32::from_str_radix(&digits, 16).map_err(|_| "bad Unicode escape")?;
                let scalar = char::from_u32(number).ok_or("invalid Unicode scalar")?;
                let mut buf = [0; 4];
                out.extend_from_slice(scalar.encode_utf8(&mut buf).as_bytes());
                at = end + 1;
            }
            b'\n' => {
                at += 1;
                while data.get(at).is_some_and(|c| c.is_ascii_whitespace()) {
                    at += 1;
                }
            }
            b'\r' if data.get(at + 1) == Some(&b'\n') => {
                at += 2;
                while data.get(at).is_some_and(|c| c.is_ascii_whitespace()) {
                    at += 1;
                }
            }
            _ => return Err(format!("unsupported escape at {at}")),
        }
        segment = at;
    }
}

fn lex(src: &str) -> Result<Vec<Token>, String> {
    let b = src.as_bytes();
    let mut out = Vec::new();
    let mut i = 0;
    while i < b.len() {
        if b[i].is_ascii_whitespace() {
            i += 1;
            continue;
        }
        if b[i..].starts_with(b"//") {
            i = b[i..]
                .iter()
                .position(|c| *c == b'\n')
                .map_or(b.len(), |n| i + n);
            continue;
        }
        if b[i..].starts_with(b"/*") {
            i += 2;
            let mut depth = 1;
            while i + 1 < b.len() && depth != 0 {
                if b[i..].starts_with(b"/*") {
                    depth += 1;
                    i += 2;
                } else if b[i..].starts_with(b"*/") {
                    depth -= 1;
                    i += 2;
                } else {
                    i += 1;
                }
            }
            if depth != 0 {
                return Err("unclosed block comment".into());
            }
            continue;
        }
        let start = i;
        let prefix = if b[i..].starts_with(b"br") {
            2
        } else if matches!(b[i], b'b' | b'r') {
            1
        } else {
            0
        };
        let bytes = b[i] == b'b';
        let mut j = i + prefix;
        let raw = prefix > 0 && (b[i] == b'r' || b.get(i + 1) == Some(&b'r'));
        if raw {
            while b.get(j) == Some(&b'#') {
                j += 1;
            }
        }
        if b.get(j) == Some(&b'"') && (prefix == 0 || raw || bytes && prefix == 1) {
            let hashes = j - i - prefix;
            let (value, end) = if raw {
                let close = format!("\"{}", "#".repeat(hashes));
                let offset = src[j + 1..].find(&close).ok_or("unclosed raw literal")?;
                let end = j + 1 + offset;
                (b[j + 1..end].to_vec(), end + close.len())
            } else {
                escaped(src, j + 1, bytes, b'"')?
            };
            out.push(Token {
                text: "<literal>".into(),
                value: Some(value),
                at: start,
            });
            i = end;
            continue;
        }
        if b[i] == b'\'' || b[i..].starts_with(b"b'") {
            let quote = if b[i] == b'\'' { i } else { i + 1 };
            let next = quote + 1;
            let char_form = b.get(next) == Some(&b'\\') || b.get(next + 1) == Some(&b'\'');
            if char_form {
                let (value, end) = escaped(src, next, b[i] == b'b', b'\'')?;
                if b.get(end - 1) != Some(&b'\'') || value.is_empty() {
                    return Err(format!("bad character literal at {start}"));
                }
                out.push(Token {
                    text: "<char>".into(),
                    value: Some(value),
                    at: start,
                });
                i = end;
                continue;
            }
        }
        if b[i].is_ascii_alphanumeric() || b[i] == b'_' {
            i += 1;
            while i < b.len() && (b[i].is_ascii_alphanumeric() || b[i] == b'_') {
                i += 1;
            }
        } else if b[i].is_ascii() {
            i += 1;
        } else {
            let c = src[i..].chars().next().ok_or("bad UTF-8 boundary")?;
            i += c.len_utf8();
        }
        out.push(token(&src[start..i], start));
    }
    Ok(out)
}

fn close(t: &[Token], open: usize, left: &str, right: &str) -> Result<usize, String> {
    let mut depth = 0;
    for (i, item) in t.iter().enumerate().skip(open) {
        if item.text == left {
            depth += 1;
        }
        if item.text == right {
            depth -= 1;
            if depth == 0 {
                return Ok(i);
            }
        }
    }
    Err(format!("unclosed {left} at {}", t[open].at))
}

#[derive(Clone)]
struct Function {
    body: Vec<Token>,
    marked: bool,
    start: usize,
    end: usize,
}

fn functions(src: &str, t: &[Token]) -> Result<HashMap<String, Function>, String> {
    let mut result: HashMap<String, Function> = HashMap::new();
    for i in 0..t.len().saturating_sub(3) {
        if t[i].text != "fn" || t[i + 2].text != "(" {
            continue;
        }
        let name = t[i + 1].text.clone();
        let params_end = close(t, i + 2, "(", ")")?;
        let body_start = (params_end + 1..t.len()).find(|&j| t[j].text == "{" || t[j].text == ";");
        let Some(open) = body_start else {
            return Err(format!("missing body of {name}"));
        };
        if t[open].text == ";" {
            continue;
        }
        let end = close(t, open, "{", "}")?;
        let previous = src[..t[i].at]
            .trim_end()
            .lines()
            .last()
            .unwrap_or("")
            .trim();
        let marked = previous == MARK;
        if let Some(previous) = result.get(&name) {
            if previous.marked && marked {
                return Err(format!("duplicate marked function {name}"));
            }
            if previous.marked {
                continue;
            }
        }
        result.insert(
            name,
            Function {
                body: t[open + 1..end].to_vec(),
                marked,
                start: t[i].at,
                end: t[end].at,
            },
        );
    }
    Ok(result)
}

fn calls(body: &[Token]) -> Vec<(usize, String)> {
    let mut result = Vec::new();
    for i in 0..body.len().saturating_sub(1) {
        if body[i].text == "<literal>" || body[i].text == "<char>" {
            continue;
        }
        let open = if body[i + 1].text == "(" {
            Some(i + 1)
        } else if body.get(i + 1).is_some_and(|x| x.text == "!")
            && body.get(i + 2).is_some_and(|x| x.text == "(")
        {
            Some(i + 2)
        } else {
            None
        };
        if open.is_some()
            && body[i]
                .text
                .chars()
                .next()
                .is_some_and(|c| c.is_ascii_alphabetic() || c == '_')
        {
            result.push((i, body[i].text.clone()));
        }
    }
    result
}

fn glyphs(fs: &HashMap<String, Function>) -> Result<HashSet<Vec<u8>>, String> {
    let body = &fs.get("production").ok_or("missing production()")?.body;
    let mut result = HashSet::new();
    for i in 0..body.len().saturating_sub(1) {
        if body[i].value.is_some() && body[i + 1].text == "=" {
            result.insert(body[i].value.clone().unwrap());
        }
    }
    if result.len() != 36 {
        return Err(format!(
            "production() has {} spellings, expected 36",
            result.len()
        ));
    }
    Ok(result)
}

fn terminal_aliases(tokens: &[Token], spellings: &HashSet<Vec<u8>>) -> HashSet<String> {
    let mut aliases = HashSet::new();
    for (start, item) in tokens.iter().enumerate() {
        if !matches!(item.text.as_str(), "const" | "let") {
            continue;
        }
        let Some(name) = tokens.get(start + 1) else {
            continue;
        };
        let end = (start + 2..tokens.len()).find(|&at| tokens[at].text == ";");
        let Some(end) = end else { continue };
        let equal = (start + 2..end).find(|&at| tokens[at].text == "=");
        if equal
            .and_then(|at| tokens.get(at + 1))
            .and_then(|token| token.value.as_ref())
            .is_some_and(|value| spellings.contains(value))
        {
            aliases.insert(name.text.clone());
        }
    }
    aliases
}

fn legacy_branch(body: &[Token], call: usize) -> bool {
    for i in 0..call.saturating_sub(6) {
        if body[i].text == "if"
            && body[i + 1].text == "!"
            && body[i + 2].text == "self"
            && body[i + 3].text == "."
            && body[i + 4].text == "new_profile"
            && body[i + 5].text == "{"
            && close(body, i + 5, "{", "}").is_ok_and(|end| call < end)
        {
            return true;
        }
    }
    false
}

fn guard(src: &str) -> Vec<String> {
    let mut errors = Vec::new();
    let t = match lex(src) {
        Ok(t) => t,
        Err(e) => return vec![e],
    };
    let fs = match functions(src, &t) {
        Ok(fs) => fs,
        Err(e) => return vec![e],
    };
    let spellings = match glyphs(&fs) {
        Ok(s) => s,
        Err(e) => return vec![e],
    };
    let aliases = terminal_aliases(&t, &spellings);
    let roots = [
        "production_entry",
        "deferred_in_state",
        "starts_expr",
        "starts_try",
        "starts_postfix",
        "starts_statement",
        "starts_atom",
        "starts_block_open",
        "select_module_entry",
        "select_block_entry",
        "select_primary_entry",
    ];
    for name in roots {
        if !fs.get(name).is_some_and(|f| f.marked) {
            errors.push(format!("{name}: missing dispatch marker"));
        }
    }
    let structural: HashSet<&str> = [
        "peek",
        "feature",
        "production_row",
        "production_try_excluded",
        "module_choice",
        "block_choice",
        "primary_choice",
        "starts_expr_legacy",
        "starts_postfix_legacy",
        "starts_statement_legacy",
        "select_module_entry_legacy",
        "select_block_entry_legacy",
        "select_primary_entry_legacy",
        "legacy_deferred",
    ]
    .into();
    let projections = [
        "production_row",
        "production_try_excluded",
        "module_choice",
        "block_choice",
        "primary_choice",
    ];
    let legacy = [
        "starts_expr_legacy",
        "starts_postfix_legacy",
        "starts_statement_legacy",
        "select_module_entry_legacy",
        "select_block_entry_legacy",
        "select_primary_entry_legacy",
        "legacy_deferred",
    ];
    let library: HashSet<&str> = [
        "Some",
        "Deferred",
        "matches",
        "map",
        "clone",
        "as_str",
        "is_some",
        "is_some_and",
    ]
    .into();
    for (name, f) in &fs {
        if f.marked || matches!(name.as_str(), "program" | "statement_inner" | "prefix") {
            for x in &f.body {
                if aliases.contains(&x.text) {
                    errors.push(format!("{name}:{}: terminal alias", x.at));
                }
            }
            for window in f.body.windows(4) {
                if window[0].text == "text"
                    && window[1].text == "."
                    && window[2].text == "len"
                    && window[3].text == "("
                {
                    errors.push(format!("{name}:{}: terminal length", window[0].at));
                }
            }
        }
        if !f.marked && !projections.contains(&name.as_str()) {
            continue;
        }
        for (i, x) in f.body.iter().enumerate() {
            if x.value.as_ref().is_some_and(|v| spellings.contains(v)) {
                errors.push(format!("{name}:{}: terminal literal", x.at));
            }
            if name == "starts_expr" && x.text == "text" && i > 0 && f.body[i - 1].text == "." {
                errors.push(format!("{name}:{}: .text", x.at));
            }
            if x.text == "deferred_feature" {
                errors.push(format!("{name}:{}: deferred_feature", x.at));
            }
        }
        if !f.marked {
            let permitted: &[&str] = match name.as_str() {
                "production_row" => &["production", "iter", "find"],
                "production_try_excluded" => &["production_row", "is_some_and"],
                "module_choice" | "block_choice" | "primary_choice" => &[],
                _ => &[],
            };
            for (i, callee) in calls(&f.body) {
                if !permitted.contains(&callee.as_str()) {
                    errors.push(format!(
                        "{name}:{}: new call inside table projection: {callee}",
                        f.body[i].at
                    ));
                }
            }
            continue;
        }
        for (i, callee) in calls(&f.body) {
            if matches!(callee.as_str(), "if" | "while" | "for" | "match" | "return") {
                continue;
            }
            let allowed = fs.get(&callee).is_some_and(|target| target.marked)
                || structural.contains(callee.as_str())
                || library.contains(callee.as_str());
            if !allowed {
                errors.push(format!(
                    "{name}:{}: unmarked or unknown callee {callee}",
                    f.body[i].at
                ));
            }
            if f.marked && legacy.contains(&callee.as_str()) && !legacy_branch(&f.body, i) {
                errors.push(format!(
                    "{name}:{}: legacy call outside !new_profile",
                    f.body[i].at
                ));
            }
        }
    }
    // The table declaration and its fixed projections are the only typed-terminal enclosure.
    let table_start = t
        .windows(2)
        .position(|w| w[0].text == "const" && w[1].text == "PRODUCTION_ROWS");
    let table_end = table_start.and_then(|s| {
        let equal = (s..t.len()).find(|&i| t[i].text == "=")?;
        let open = (equal + 1..t.len()).find(|&i| t[i].text == "[")?;
        close(&t, open, "[", "]").ok()
    });
    if table_start.is_none() || table_end.is_none() {
        errors.push("missing PRODUCTION_ROWS".into());
    }
    let (table_start, table_end) = (table_start.unwrap_or(0), table_end.unwrap_or(0));
    let enclosure = [
        "production",
        "production_row",
        "production_entry",
        "production_try_excluded",
        "module_choice",
        "block_choice",
        "primary_choice",
    ];
    let parser_start = src.find("impl Parser<'_> {").unwrap_or(usize::MAX);
    let test_ranges: Vec<_> = t
        .windows(3)
        .enumerate()
        .filter_map(|(i, w)| {
            (w[0].text == "mod" && w[1].text == "tests" && w[2].text == "{")
                .then(|| close(&t, i + 2, "{", "}").ok().map(|end| (i, end)))
                .flatten()
        })
        .collect();
    let mut table_variants = HashSet::new();
    let mut lexical_variants = HashSet::new();
    let conversion = &fs["production"].body;
    for w in conversion.windows(5) {
        if w[0].value.is_some()
            && w[1].text == "="
            && w[2].text == ">"
            && w[3].text == "Production"
            && w[4].text == ":"
        {
            // The actual variant is the next token after the two colons.
            let at = w[3].at;
            if let Some(index) = conversion.iter().position(|x| x.at == at)
                && let Some(variant) = conversion.get(index + 3)
            {
                lexical_variants.insert(variant.text.clone());
            }
        }
    }
    for i in 0..t.len().saturating_sub(2) {
        let at = t[i].at;
        let test_code = test_ranges
            .iter()
            .any(|(start, end)| *start <= i && i <= *end);
        if test_code {
            continue;
        }
        let owner = fs
            .iter()
            .find(|(_, f)| f.start <= at && at <= f.end)
            .map(|(name, _)| name.as_str());
        if t[i].text == "GrammarState"
            && t[i + 1].text == ":"
            && t[i + 2].text == ":"
            && at > parser_start
            && !owner.is_some_and(|name| fs[name].marked)
        {
            let committed_diagnostic =
                matches!(owner, Some("program" | "statement_inner" | "prefix"))
                    && i >= 2
                    && t[i - 1].text == "("
                    && t[i - 2].text == "deferred_in_state";
            if !committed_diagnostic {
                errors.push(format!(
                    "{}:{at}: unmarked GrammarState route",
                    owner.unwrap_or("global")
                ));
            }
        }
        if t[i].text == "production_entry"
            && t[i + 1].text == "("
            && (i == 0 || t[i - 1].text != "fn")
            && !owner.is_some_and(|name| fs[name].marked)
        {
            errors.push(format!(
                "{}:{at}: unmarked production_entry route",
                owner.unwrap_or("global")
            ));
        }
        let in_table = i >= table_start && i <= table_end;
        let production_variant =
            t[i].text == "Production" && t[i + 1].text == ":" && t[i + 2].text == ":";
        if production_variant
            && in_table
            && let Some(variant) = t.get(i + 3)
        {
            table_variants.insert(variant.text.clone());
        }
        if production_variant && !in_table && !owner.is_some_and(|name| enclosure.contains(&name)) {
            errors.push(format!(
                "{}:{at}: Production variant outside table",
                owner.unwrap_or("global")
            ));
        }
        let conversion_call =
            t[i].text == "production" && t[i + 1].text == "(" && (i == 0 || t[i - 1].text != "fn");
        if conversion_call && owner != Some("production_row") {
            errors.push(format!(
                "{}:{at}: production() outside production_row",
                owner.unwrap_or("global")
            ));
        }
    }
    if table_variants.len() != 36 || table_variants != lexical_variants {
        errors.push(format!(
            "production conversion/table variant mismatch: {} lexical, {} rows",
            lexical_variants.len(),
            table_variants.len()
        ));
    }
    for (name, selector) in [
        ("program", "select_module_entry"),
        ("statement_inner", "select_block_entry"),
        ("prefix", "select_primary_entry"),
    ] {
        let Some(f) = fs.get(name) else {
            errors.push(format!("missing {name}"));
            continue;
        };
        let expected_projection = match name {
            "program" => ("select_module_entry", "module_choice", "ModuleItem"),
            "statement_inner" => ("select_block_entry", "block_choice", "BlockItem"),
            _ => ("select_primary_entry", "primary_choice", "Primary"),
        };
        if let Some(select) = fs.get(expected_projection.0) {
            let selected_calls = calls(&select.body);
            if !selected_calls.iter().any(|(_, c)| c == "production_entry")
                || !selected_calls
                    .iter()
                    .any(|(_, c)| c == expected_projection.1)
                || !select.body.windows(4).any(|w| {
                    w[0].text == "GrammarState"
                        && w[1].text == ":"
                        && w[2].text == ":"
                        && w[3].text == expected_projection.2
                })
            {
                errors.push(format!(
                    "{selector}: missing table row projection for {}",
                    expected_projection.2
                ));
            }
        }
        let found: Vec<_> = calls(&f.body)
            .into_iter()
            .filter(|(_, call)| call.starts_with("select_") && call.ends_with("_entry"))
            .collect();
        if found.len() != 1 || found[0].1 != selector {
            errors.push(format!("{name}: expected one direct {selector}()"));
            continue;
        }
        let pos = found[0].0;
        if pos < 2 || f.body[pos - 1].text != "." || f.body[pos - 2].text != "self" {
            errors.push(format!("{name}: selector is not a direct self call"));
        }
        let end = match close(&f.body, pos + 1, "(", ")") {
            Ok(end) => end,
            Err(e) => {
                errors.push(e);
                continue;
            }
        };
        if f.body[end + 1..].windows(3).any(|window| {
            window[0].text == "choice" && window[1].text == "=" && window[2].text != "="
        }) {
            errors.push(format!("{name}: reclassification after commitment"));
        }
        if matches!(name, "program" | "statement_inner") {
            for at in end + 1..f.body.len() {
                if f.body[at].text != "if" {
                    continue;
                }
                let Some(open) = (at + 1..f.body.len()).find(|&i| f.body[i].text == "{") else {
                    continue;
                };
                let condition = &f.body[at + 1..open];
                if condition
                    .windows(2)
                    .any(|window| window[0].text == "." && window[1].text == "text")
                    && condition
                        .iter()
                        .any(|token| token.value.as_ref().is_some_and(|v| spellings.contains(v)))
                {
                    errors.push(format!("{name}:{}: post-commit terminal", f.body[at].at));
                }
            }
        }
        if name == "program" {
            let loop_open = f
                .body
                .iter()
                .position(|x| x.text == "while")
                .and_then(|i| (i..f.body.len()).find(|&j| f.body[j].text == "{"));
            if !loop_open.is_some_and(|open| open < pos) {
                errors.push("program: selector outside while body".into());
            }
        }
        for x in &f.body[..=end] {
            if x.value.as_ref().is_some_and(|v| spellings.contains(v)) {
                errors.push(format!("{name}:{}: terminal before commitment", x.at));
            }
            if matches!(x.text.as_str(), "if" | "match" | "return") {
                errors.push(format!("{name}:{}: classification before commitment", x.at));
            }
        }
        if f.body[..=end]
            .windows(3)
            .any(|w| w[0].text == "self" && w[1].text == "." && w[2].text == "at")
        {
            errors.push(format!("{name}: direct cursor access before commitment"));
        }
        for (i, callee) in calls(&f.body[..=end]) {
            if !matches!(
                callee.as_str(),
                "new"
                    | "eof"
                    | "peek"
                    | "select_module_entry"
                    | "select_block_entry"
                    | "select_primary_entry"
            ) {
                errors.push(format!(
                    "{name}:{}: call before commitment: {callee}",
                    f.body[i].at
                ));
            }
        }
    }
    errors
}

#[test]
fn structural_dispatch_guard() {
    let errors = guard(SOURCE);
    assert!(errors.is_empty(), "{}", errors.join("\n"));
}

#[test]
fn decoded_literal_forms() {
    for literal in [
        r#""\x66or""#,
        r#""fo\u{72}""#,
        r##"r#"for"#"##,
        r#"b"for""#,
        r#"br"for""#,
    ] {
        let value = lex(literal).unwrap()[0].value.clone().unwrap();
        assert_eq!(value, b"for");
    }
}

fn inserted(before: &str, anchor: &str, extra: &str) -> String {
    assert!(before.contains(anchor), "missing mutation anchor: {anchor}");
    before.replacen(anchor, &format!("{anchor}{extra}"), 1)
}

#[test]
fn dispatch_mutations_are_rejected() {
    let deferred = "let token = self.peek();";
    let cases = [
        (
            "M1",
            inserted(
                SOURCE,
                deferred,
                " if token.text == \"for\" { return None; }",
            ),
        ),
        (
            "M2",
            inserted(
                SOURCE,
                deferred,
                " if self.slice(&token.span) == \"..\" { return None; }",
            ),
        ),
        (
            "M3",
            inserted(
                SOURCE,
                deferred,
                " if matches!(token.text.as_str(), \"for\") { return None; }",
            ),
        ),
        (
            "M4",
            inserted(
                SOURCE,
                deferred,
                " if [\"for\", \"yield\"].contains(&token.text.as_str()) { return None; }",
            ),
        ),
        (
            "M5",
            inserted(
                &format!(
                    "{SOURCE}\nfn loop_word(text: &str) -> bool {{ matches!(text, \"for\" | \"yield\") }}"
                ),
                deferred,
                " if loop_word(&token.text) { return None; }",
            ),
        ),
        (
            "M6",
            inserted(
                SOURCE,
                deferred,
                " if production(token) == Some(Production::ForStmt) { return None; }",
            ),
        ),
        (
            "M7",
            inserted(
                SOURCE,
                "fn starts_atom(&self) -> bool {",
                " let _ = self.peek().text == \"gen\";",
            ),
        ),
        (
            "M8",
            inserted(
                SOURCE,
                "fn starts_try(&self) -> bool {",
                " let _ = self.peek().text != \"gen\";",
            ),
        ),
        (
            "M9",
            inserted(
                SOURCE,
                "fn starts_block_open(&self) -> bool {",
                " let _ = self.peek().text == \"gen\";",
            ),
        ),
        (
            "M10",
            inserted(
                SOURCE,
                "fn starts_expr(&self) -> bool {",
                " let _ = self.peek().text;",
            ),
        ),
    ];
    for (name, source) in cases {
        assert!(!guard(&source).is_empty(), "{name} escaped the guard");
    }
    for literal in [
        r#""\x66or""#,
        r#""fo\u{72}""#,
        r##"r#"for"#"##,
        r#"b"for""#,
        r#"br"for""#,
    ] {
        let source = inserted(
            SOURCE,
            deferred,
            &format!(" if token.text == {literal} {{ return None; }}"),
        );
        assert!(
            guard(&source)
                .iter()
                .any(|e| e.contains("terminal literal")),
            "{literal}"
        );
    }
    let marked_helper = inserted(
        SOURCE,
        deferred,
        " if loop_word(&token.text) { return None; }",
    ) + "\n// nexo:full-profile-dispatch\nfn loop_word(text: &str) -> bool { matches!(text, \"for\") }\n";
    assert!(
        guard(&marked_helper)
            .iter()
            .any(|e| e.contains("terminal literal"))
    );
    let new_state = inserted(
        SOURCE,
        "fn word(&self, s: &str) -> bool {",
        " let _ = production_entry(self.peek(), GrammarState::Primary);",
    );
    assert!(guard(&new_state).iter().any(|e| e.contains("unmarked")));
}

#[test]
fn commitment_and_indirect_reclassification_mutations_are_rejected() {
    let before = inserted(
        SOURCE,
        "while !self.eof() {",
        " if self.peek().text == \"for\" { return Err(self.feature(\"methods\", self.peek().span.clone())); }",
    );
    assert!(
        guard(&before)
            .iter()
            .any(|e| e.contains("before commitment"))
    );
    let consume = inserted(SOURCE, "while !self.eof() {", " self.take();");
    assert!(
        guard(&consume)
            .iter()
            .any(|e| e.contains("call before commitment"))
    );
    let after = inserted(
        SOURCE,
        "let choice = self.select_module_entry();",
        " if self.peek().text == \"for\" { return Err(self.feature(\"methods\", self.peek().span.clone())); }",
    );
    assert!(
        guard(&after)
            .iter()
            .any(|error| error.contains("post-commit terminal")),
        "post-commit terminal mutation escaped: {:?}",
        guard(&after),
    );
    let alias = inserted(
        &format!("{SOURCE}\nconst FOR_ALIAS: &str = \"for\";"),
        "let token = self.peek();",
        " if token.text == FOR_ALIAS { return None; }",
    );
    assert!(
        guard(&alias)
            .iter()
            .any(|error| error.contains("terminal alias")),
        "constant alias mutation escaped: {:?}",
        guard(&alias),
    );
    let kind_length = inserted(
        SOURCE,
        "let token = self.peek();",
        " if token.kind == Kind::Name && token.text.len() == 3 { return None; }",
    );
    assert!(
        guard(&kind_length)
            .iter()
            .any(|error| error.contains("terminal length")),
        "kind plus length mutation escaped: {:?}",
        guard(&kind_length),
    );
    let reclassified = inserted(
        SOURCE,
        "let choice = self.select_module_entry();",
        " let choice = if self.peek().kind == Kind::Name { ModuleChoice::Fn } else { choice };",
    );
    assert!(
        guard(&reclassified)
            .iter()
            .any(|error| error.contains("reclassification after commitment")),
        "post-commitment reclassification escaped: {:?}",
        guard(&reclassified),
    );
    let consumer = inserted(
        SOURCE,
        "let choice = self.select_module_entry();",
        " let _token_span = self.peek().span.clone();",
    );
    assert!(guard(&consumer).is_empty(), "{:?}", guard(&consumer));
}
